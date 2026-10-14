//! Exact history/source deliveries and actor-owned original restores. A restored
//! source is an ordinary Core save; it never rewinds history or replaces Drafts.
use super::*;
use weftext_core::{
    ActorHistoryRestoreStatus, HistoryComparison, HistoryRead, HistoryRestoreRequest,
    HistoryRestoreResult, HistoryTimeline, QualifiedNode,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Session {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Version {
    session_id: String,
    version_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DraftComparison {
    session_id: String,
    version_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Restore {
    resource: String,
    session_id: String,
    base_id: String,
    version_id: String,
    operation_id: String,
    draft_action: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    resource: String,
    operation_id: String,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/history", post(list))
        .route("/api/hosted/history/list", post(list))
        .route("/api/hosted/history/read", post(read))
        .route("/api/hosted/history/compare", post(compare))
        .route("/api/hosted/history/compare-draft", post(compare_draft))
        .route("/api/hosted/history/restore", post(restore))
        .route("/api/hosted/history/status", post(status))
        .route("/api/hosted/history/settle", post(settle))
}
fn history_error(error: CoreError) -> ApiError {
    let message = match error.code.as_str() {
        "draftProtectionRequired" => "存在未保存草稿，请明确选择保留草稿后再恢复。",
        "recoveryPending" => "原恢复操作尚待核对；请查询原操作，不要重新执行或换编号。",
        "operationConflict" => "原操作编号已绑定其他恢复输入或操作类别；请保留输入并核对原编号。",
        _ => "当前对象、历史版本或原恢复依据不可用；原文和草稿已保留。",
    };
    ApiError::new(StatusCode::CONFLICT, &error.code, message)
}
fn access(
    inner: &HostedInner,
    auth: &Auth,
    resource: &str,
    write: bool,
) -> Result<String, ApiError> {
    let actor = inner.actor(auth)?;
    let rights = inner.rights(&actor, resource)?;
    let file = author_file(inner, resource)?;
    if !rights.history || write && (!rights.write || !file.editable) {
        return Err(unavailable());
    }
    Ok(actor)
}
fn qualify_session(
    inner: &mut HostedInner,
    auth: &Auth,
    session_id: &str,
    write: bool,
) -> Result<(String, HostedSession, QualifiedNode), ApiError> {
    let (actor, session, _) = inner.session(auth, session_id)?;
    access(inner, auth, &session.resource, write)?;
    let root = inner.config.root.clone();
    let path = root.join(&inner.file(&session.resource)?.path);
    let scope = inner.actor_scope(&actor);
    let qualified = inner
        .core
        .with_actor_for_root(&scope, &inner.config.root, |core| {
            core.qualified_node_for_session_at(&root, &path, session_id)
        })
        .map_err(history_error)?;
    Ok((actor, session, qualified))
}
fn final_session(
    inner: &mut HostedInner,
    auth: &Auth,
    session_id: &str,
    resource: &str,
    original: &QualifiedNode,
) -> Result<(), ApiError> {
    let (_, session, current) = qualify_session(inner, auth, session_id, false)?;
    if session.resource != resource || current.owner != original.owner {
        return Err(unavailable());
    }
    let path = inner.config.root.join(&inner.file(resource)?.path);
    inner
        .core
        .validate_snapshot_witness(&inner.config.root, &path, &original.witness)
        .map_err(history_error)
}
fn resource_path(resource: &str) -> String {
    format!("resource:{resource}")
}
fn clean_version(version: &mut weftext_core::HistoryVersion) {
    version.message = if version.status == "available" {
        "准确的已保存源码可读。"
    } else {
        "此历史版本当前不可读，未替换为其他版本。"
    }
    .into();
}
fn clean_read(mut read: HistoryRead) -> HistoryRead {
    clean_version(&mut read.version);
    read.preview_message = "仅查看准确历史源码；不使用当前配置或依赖代替原语义预览。".into();
    read
}
fn clean_timeline(mut timeline: HistoryTimeline, resource: &str) -> HistoryTimeline {
    timeline.path = resource_path(resource);
    for version in &mut timeline.versions {
        clean_version(version);
    }
    for draft in &mut timeline.protected_drafts {
        draft.path = resource_path(resource);
    }
    timeline.retention.message = "仅列出原 Core 仍保留的已提交源码记录；空列表不能证明从未保存，早期或其他设备历史不保证可用。".into();
    timeline.warnings = if timeline.warnings.is_empty() {
        vec![]
    } else {
        vec!["当前稿为外部观察或准确保存依据不可用，未将其冒作已保存版本。".into()]
    };
    timeline
}
fn clean_comparison(mut comparison: HistoryComparison, resource: &str) -> HistoryComparison {
    comparison.selected = clean_read(comparison.selected);
    comparison.current.path = resource_path(resource);
    comparison
}
fn clean_result(
    mut result: HistoryRestoreResult,
    resource: &str,
    id: &str,
) -> HistoryRestoreResult {
    result.receipt.operation_id = id.into();
    result.receipt.path = resource_path(resource);
    result.receipt.message = "原恢复操作已提交为一次普通保存；受保护草稿仍单独保留。".into();
    for draft in &mut result.protected_drafts {
        draft.path = resource_path(resource);
    }
    result
}
async fn list(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Session>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, session, qualified) =
                qualify_session(inner, &auth, &req.session_id, false)?;
            let scope = inner.actor_scope(&actor);
            let timeline = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
                    core.list_history(&req.session_id)
                })
                .map_err(history_error)?;
            final_session(inner, &auth, &req.session_id, &session.resource, &qualified)?;
            value(clean_timeline(timeline, &session.resource))
        })
        .await?,
    ))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Version>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, session, qualified) =
                qualify_session(inner, &auth, &req.session_id, false)?;
            let scope = inner.actor_scope(&actor);
            let read = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
                    core.read_history(&req.session_id, &req.version_id)
                })
                .map_err(history_error)?;
            final_session(inner, &auth, &req.session_id, &session.resource, &qualified)?;
            value(clean_read(read))
        })
        .await?,
    ))
}
async fn compare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Version>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, session, qualified) =
                qualify_session(inner, &auth, &req.session_id, false)?;
            let scope = inner.actor_scope(&actor);
            let comparison = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
                    core.compare_history(&req.session_id, &req.version_id)
                })
                .map_err(history_error)?;
            final_session(inner, &auth, &req.session_id, &session.resource, &qualified)?;
            value(clean_comparison(comparison, &session.resource))
        })
        .await?,
    ))
}
async fn compare_draft(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<DraftComparison>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        writing_api::validate_source(&req.source, req.source_hash.as_deref())?;
        let (actor, session, qualified) = qualify_session(inner, &auth, &req.session_id, false)?;
        writing_api::exact_version(&inner.editor_versions, &req.session_id, &req.source, req.serial)?;
        let scope = inner.actor_scope(&actor);
        let selected = inner.core.with_actor_for_root(&scope, &inner.config.root, |core| core.read_history(&req.session_id, &req.version_id)).map_err(history_error)?;
        let timeline = inner.core.with_actor_for_root(&scope, &inner.config.root, |core| core.list_history(&req.session_id)).map_err(history_error)?;
        final_session(inner, &auth, &req.session_id, &session.resource, &qualified)?;
        writing_api::exact_version(&inner.editor_versions, &req.session_id, &req.source, req.serial)?;
        Ok(json!({"selected":clean_read(selected.clone()),"current":{"source":req.source,"serial":req.serial,
            "sourceHash":weftext_engine::source_hash(&req.source),"baseId":timeline.current_base_id,"path":resource_path(&session.resource)},
            "equal":selected.source == req.source,"comparison":"draftSnapshot"}))
    }).await?))
}
fn restore_fingerprint(req: &Restore) -> Result<String, ApiError> {
    writing_api::validate_source(&req.source, req.source_hash.as_deref())?;
    if !matches!(req.draft_action.as_str(), "retain" | "none") {
        return Err(ApiError::bad("请明确选择保留草稿或确认没有草稿"));
    }
    Ok(digest(&serde_json::to_vec(&json!({"resource":req.resource,"sessionId":req.session_id,"baseId":req.base_id,
        "versionId":req.version_id,"operationId":req.operation_id,"draftAction":req.draft_action,"source":req.source,"serial":req.serial})).map_err(|_| ApiError::internal("无法固定原恢复输入"))?))
}
fn public_status(
    inner: &mut HostedInner,
    auth: &Auth,
    status: ActorHistoryRestoreStatus,
) -> Result<Value, ApiError> {
    let actor = access(inner, auth, &status.resource, false)?;
    let scope = inner.actor_scope(&actor);
    let root = inner.config.root.clone();
    let path = root.join(&inner.file(&status.resource)?.path);
    let mut output = json!({"operationId":status.operation_id,"resource":status.resource,"status":status.status,"resultStatus":"unavailable"});
    if status.status == "committed" {
        let result = inner
            .core
            .with_actor_for_root(&scope, &inner.config.root, |core| {
                let result = core.actor_history_restore_result_at(
                    &status.resource,
                    &root,
                    &path,
                    &status.operation_id,
                )?;
                let qualified = core.qualified_node_at(&root, &path)?;
                let current = core.actor_history_restore_current_at(
                    &status.resource,
                    &root,
                    &path,
                    &status.operation_id,
                )?;
                // A configured path could have been replaced between reading the
                // retained result and qualifying its current basis. Recheck the
                // original private owner, then the captured current source bytes.
                core.actor_history_restore_owner_at(
                    &status.resource,
                    &root,
                    &path,
                    &status.operation_id,
                )?;
                core.validate_snapshot_witness(&root, &path, &qualified.witness)?;
                Ok((result, qualified, current))
            });
        match result {
            Ok((result, qualified, original_current)) => {
                let current = original_current
                    && qualified.source_hash == digest(result.source.as_bytes())
                    && qualified.revision.is_some()
                    && qualified.revision == result.receipt.revision_id;
                output["resultBasis"] = json!(if current {
                    "current"
                } else if qualified.revision.is_none() {
                    "external"
                } else {
                    "superseded"
                });
                output["currentRevision"] = json!(qualified.revision);
                access(inner, auth, &status.resource, false)?;
                output["resultStatus"] = json!("available");
                output["result"] =
                    json!(clean_result(result, &status.resource, &status.operation_id));
            }
            Err(_) => output["resultStatus"] = json!("unavailable"),
        }
    }
    Ok(output)
}
async fn restore(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Restore>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = access(inner, &auth, &req.resource, true)?;
            let intent = restore_fingerprint(&req)?;
            let scope = inner.actor_scope(&actor);
            let known = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
                    core.actor_history_restore_retry(&req.resource, &req.operation_id, &intent)
                })
                .map_err(history_error)?;
            let status = match known {
                Some(status) => status,
                None => {
                    let (_, session, _) = qualify_session(inner, &auth, &req.session_id, true)?;
                    if session.resource != req.resource {
                        return Err(unavailable());
                    }
                    writing_api::exact_version(
                        &inner.editor_versions,
                        &req.session_id,
                        &req.source,
                        req.serial,
                    )?;
                    let root = inner.config.root.clone();
                    let path = root.join(&inner.file(&req.resource)?.path);
                    inner
                        .core
                        .with_actor_for_root(&scope, &inner.config.root, |core| {
                            core.actor_history_restore(
                                &req.resource,
                                &root,
                                &path,
                                HistoryRestoreRequest {
                                    session_id: req.session_id,
                                    base_id: req.base_id,
                                    version_id: req.version_id,
                                    operation_id: req.operation_id,
                                    draft_action: req.draft_action,
                                    draft_source: Some(req.source),
                                },
                                &intent,
                            )
                        })
                        .map_err(history_error)?
                }
            };
            public_status(inner, &auth, status)
        })
        .await?,
    ))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = access(inner, &auth, &req.resource, false)?;
        let scope = inner.actor_scope(&actor);
        let status = inner.core.with_actor_for_root(&scope, &inner.config.root, |core| core.actor_history_restore_status(&req.resource, &req.operation_id)).map_err(history_error)?;
        match status {
            Some(status) => public_status(inner, &auth, status),
            None => Ok(json!({"operationId":req.operation_id,"resource":req.resource,"status":"notRegistered","resultStatus":"unavailable"})),
        }
    }).await?))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = access(inner, &auth, &req.resource, true)?;
            let scope = inner.actor_scope(&actor);
            let status = inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |core| {
                    core.actor_history_restore_settle(&req.resource, &req.operation_id)
                })
                .map_err(history_error)?;
            public_status(inner, &auth, status)
        })
        .await?,
    ))
}

#[cfg(all(test, windows))]
pub(super) fn test_fixture() -> (tempfile::TempDir, HostedState, Auth, Value, String, String) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut core = Core::new(temp.path().join("state")).unwrap();
    core.create_library(&root).unwrap();
    let first = core.create_note(&root, &root, "Shared").unwrap();
    let original = first.source.clone();
    let version = core
        .list_history(&first.session_id)
        .unwrap()
        .current_saved_version_id
        .unwrap();
    core.save(weftext_core::SaveRequest {
        session_id: first.session_id.clone(),
        base_id: first.base_id.clone(),
        source: original.clone() + "\nSaved B 😀\n",
        operation_id: uuid::Uuid::new_v4().to_string(),
    })
    .unwrap();
    core.close_session(&first.session_id).unwrap();
    let target = core.create_note(&root, &root, "Target").unwrap();
    core.close_session(&target.session_id).unwrap();
    let full = Rights {
        read: true,
        write: true,
        history: true,
        ..Default::default()
    };
    let users = [
        ("alice", full.clone()),
        ("bob", full),
        (
            "reader",
            Rights {
                read: true,
                ..Default::default()
            },
        ),
        (
            "historian",
            Rights {
                read: true,
                history: true,
                ..Default::default()
            },
        ),
        (
            "writer",
            Rights {
                read: true,
                write: true,
                ..Default::default()
            },
        ),
    ]
    .into_iter()
    .map(|(actor, rights)| {
        (
            actor.into(),
            User {
                query_providers: Default::default(),
                query_scopes: Default::default(),
                library: Default::default(),
                resources: Default::default(),
                resource_collections: Default::default(),
                structure: Default::default(),
                epoch: 0,
                token_sha256: digest(actor.as_bytes()),
                enabled: true,
                grants: BTreeMap::from([("Shared".into(), rights)]),
                sync_owner: false,
                native_whole_library: Default::default(),
                content_imports: Default::default(),
                owner_backup: Default::default(),
                navigation: Default::default(),
            },
        )
    })
    .collect::<BTreeMap<_, _>>();
    let mut inner = HostedInner {
        core,
        config: Config {
            directories: Default::default(),
            style_inputs: Default::default(),
            resources: Default::default(),
            resource_collections: Default::default(),
            structure_slots: Default::default(),
            structure_templates: Default::default(),
            native_whole_library_export: false,
            content_import_slots: Default::default(),
            owner_backup_directory: None,
            navigation: Default::default(),
            deployment: "history-test".into(),
            bind: "127.0.0.1:0".parse().unwrap(),
            origin: None,
            allow_insecure_http: false,
            root,
            admin_token_sha256: digest(b"admin"),
            files: [
                ("Shared", "Shared/Shared.typ"),
                ("Target", "Target/Target.typ"),
            ]
            .into_iter()
            .map(|(resource, path)| {
                (
                    resource.into(),
                    FileEntry {
                        lifecycle: "live".into(),
                        expected_node: None,
                        path: path.into(),
                        title: resource.into(),
                        editable: true,
                        importable: false,
                        inputs: vec![],
                    },
                )
            })
            .collect(),
            users: users.clone(),
            sync_identity: None,
        },
        policy: Policy {
            deployment: "history-test".into(),
            generation: 0,
            users,
            sync_identity: None,
        },
        sessions: HashMap::new(),
        downloads: HashMap::new(),
        sync: Default::default(),
        editor_versions: HashMap::new(),
        fields: Default::default(),
        review: Default::default(),
        office: Default::default(),
        library: Default::default(),
        media: Default::default(),
        navigation_registry: Default::default(),
        owner_backup: Default::default(),
    };
    let doc = inner.open("alice", "Shared").unwrap();
    writing_api::observe_version(
        &mut inner.editor_versions,
        doc["sessionId"].as_str().unwrap(),
        doc["source"].as_str().unwrap(),
        1,
    )
    .unwrap();
    let state = HostedState {
        inner: Arc::new(Mutex::new(inner)),
        origin: Arc::from("http://localhost"),
        host: Arc::from("localhost"),
        render_gate: Arc::new(tokio::sync::Semaphore::new(0)),
    };
    (
        temp,
        state,
        Auth {
            digest: digest(b"alice"),
        },
        doc,
        version,
        original,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(windows)]
    use weftext_core::HostedSaveRequest;

    fn request() -> Restore {
        Restore {
            resource: "Shared".into(),
            session_id: "original-session".into(),
            base_id: "original-base".into(),
            version_id: "exact-version".into(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            draft_action: "retain".into(),
            source: "Original Draft 😀".into(),
            serial: 7,
            source_hash: None,
        }
    }
    #[test]
    fn original_restore_intent_freezes_exact_buffer_and_protection_choice() {
        let request = request();
        let fingerprint = restore_fingerprint(&request).unwrap();
        let mut hashed = request.clone();
        hashed.source_hash = Some(weftext_engine::source_hash(&request.source));
        assert_eq!(restore_fingerprint(&hashed).unwrap(), fingerprint);
        for change in [0, 1, 2, 3, 4, 5, 6] {
            let mut changed = request.clone();
            match change {
                0 => changed.source.push('!'),
                1 => changed.serial += 1,
                2 => changed.draft_action = "none".into(),
                3 => changed.base_id.push('!'),
                4 => changed.session_id.push('!'),
                5 => changed.version_id.push('!'),
                _ => changed.resource = "Target".into(),
            }
            assert_ne!(restore_fingerprint(&changed).unwrap(), fingerprint);
        }
        hashed.source_hash = Some(digest(b"other source"));
        assert!(restore_fingerprint(&hashed).is_err());
        hashed.source_hash = None;
        hashed.draft_action = "discard".into();
        assert!(restore_fingerprint(&hashed).is_err());
    }

    #[cfg(windows)]
    fn restore_request(doc: &Value, version: &str) -> Restore {
        Restore {
            resource: "Shared".into(),
            session_id: doc["sessionId"].as_str().unwrap().into(),
            base_id: doc["baseId"].as_str().unwrap().into(),
            version_id: version.into(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            draft_action: "retain".into(),
            source: doc["source"].as_str().unwrap().into(),
            serial: 1,
            source_hash: None,
        }
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn comparisons_use_exact_saved_and_registered_draft_without_saving() {
        let (_temp, state, auth, doc, version, original) = test_fixture();
        let session = doc["sessionId"].as_str().unwrap().to_string();
        let source = doc["source"].as_str().unwrap().to_string();
        let Json(saved) = compare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Version {
                session_id: session.clone(),
                version_id: version.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(saved["selected"]["source"], original);
        assert_eq!(saved["current"]["source"], source);
        assert_eq!(saved["current"]["path"], "resource:Shared");
        let draft = source.clone() + "\nAn unsaved Draft 😀\n";
        {
            let mut inner = state.inner.lock().unwrap();
            writing_api::observe_version(&mut inner.editor_versions, &session, &draft, 2).unwrap();
        }
        let Json(compared) = compare_draft(
            State(state.clone()),
            Extension(auth.clone()),
            Json(DraftComparison {
                session_id: session.clone(),
                version_id: version.clone(),
                source: draft.clone(),
                serial: 2,
                source_hash: Some(weftext_engine::source_hash(&draft)),
            }),
        )
        .await
        .unwrap();
        assert_eq!(compared["current"]["source"], draft);
        assert_eq!(
            compared["current"]["sourceHash"],
            weftext_engine::source_hash(&draft)
        );
        assert_eq!(compared["selected"]["source"], original);
        {
            let mut inner = state.inner.lock().unwrap();
            assert_eq!(
                std::fs::read_to_string(inner.config.root.join("Shared/Shared.typ")).unwrap(),
                source
            );
            writing_api::observe_version(&mut inner.editor_versions, &session, &source, 3).unwrap();
        }
        let error = compare_draft(
            State(state),
            Extension(auth),
            Json(DraftComparison {
                session_id: session,
                version_id: version,
                source: source.clone(),
                serial: 1,
                source_hash: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(error.status, StatusCode::CONFLICT);
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn restore_retains_all_own_drafts_and_old_reply_is_superseded_by_later_save() {
        let (_temp, state, auth, doc, version, original) = test_fixture();
        let mut req = restore_request(&doc, &version);
        let persisted = req.source.clone() + "\nAlice persisted Draft\n";
        req.source.push_str("\nAlice buffer Draft 😀\n");
        req.serial = 2;
        let buffer = req.source.clone();
        {
            let mut inner = state.inner.lock().unwrap();
            let bob = inner.open("bob", "Shared").unwrap();
            let scope = inner.actor_scope("bob");
            inner
                .core
                .with_actor(&scope, |core| {
                    core.persist_draft(bob["sessionId"].as_str().unwrap(), "Bob private Draft")
                })
                .unwrap();
            let scope = inner.actor_scope("alice");
            inner
                .core
                .with_actor(&scope, |core| {
                    core.persist_draft(&req.session_id, &persisted)
                })
                .unwrap();
            writing_api::observe_version(&mut inner.editor_versions, &req.session_id, &buffer, 2)
                .unwrap();
        }
        let Json(restored) = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        assert_eq!(restored["status"], "committed");
        assert_eq!(restored["resultStatus"], "available");
        assert_eq!(restored["resultBasis"], "current");
        assert_eq!(restored["result"]["source"], original);
        assert_eq!(restored["result"]["receipt"]["path"], "resource:Shared");
        assert_eq!(
            restored["result"]["receipt"]["operationId"],
            req.operation_id
        );
        let drafts = restored["result"]["protectedDrafts"].as_array().unwrap();
        assert!(drafts.iter().any(|d| d["source"] == persisted));
        assert!(drafts.iter().any(|d| d["source"] == buffer));
        assert!(
            drafts
                .iter()
                .all(|d| d["path"] == "resource:Shared" && d["source"] != "Bob private Draft")
        );
        {
            let mut inner = state.inner.lock().unwrap();
            let opened = inner.open("alice", "Shared").unwrap();
            let scope = inner.actor_scope("alice");
            inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_save(
                        "Shared",
                        HostedSaveRequest {
                            session_id: opened["sessionId"].as_str().unwrap().into(),
                            base_id: opened["baseId"].as_str().unwrap().into(),
                            source: original.clone() + "\nSaved F\n",
                            operation_id: uuid::Uuid::new_v4().to_string(),
                        },
                    )
                })
                .unwrap();
        }
        let Json(later) = status(
            State(state),
            Extension(auth),
            Json(Operation {
                resource: "Shared".into(),
                operation_id: req.operation_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(later["status"], "committed");
        assert_eq!(later["resultBasis"], "superseded");
        assert_eq!(later["result"]["source"], original);
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn history_permission_and_write_are_independent_and_none_cannot_drop_dirty_buffer() {
        let (_temp, state, auth, doc, version, _original) = test_fixture();
        for actor in ["reader", "writer"] {
            let doc = state.inner.lock().unwrap().open(actor, "Shared").unwrap();
            let error = list(
                State(state.clone()),
                Extension(Auth {
                    digest: digest(actor.as_bytes()),
                }),
                Json(Session {
                    session_id: doc["sessionId"].as_str().unwrap().into(),
                }),
            )
            .await
            .unwrap_err();
            assert_eq!(error.status, StatusCode::NOT_FOUND);
        }
        let history_only = state
            .inner
            .lock()
            .unwrap()
            .open("historian", "Shared")
            .unwrap();
        let historian = Auth {
            digest: digest(b"historian"),
        };
        let _ = list(
            State(state.clone()),
            Extension(historian.clone()),
            Json(Session {
                session_id: history_only["sessionId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            restore(
                State(state.clone()),
                Extension(historian),
                Json(restore_request(&history_only, &version))
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        let mut req = restore_request(&doc, &version);
        req.source.push_str("\nUnsaved buffer\n");
        req.draft_action = "none".into();
        req.serial = 2;
        writing_api::observe_version(
            &mut state.inner.lock().unwrap().editor_versions,
            &req.session_id,
            &req.source,
            2,
        )
        .unwrap();
        let error = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap_err();
        assert_eq!(error.code, "draftProtectionRequired");
        let Json(result) = status(
            State(state),
            Extension(auth),
            Json(Operation {
                resource: "Shared".into(),
                operation_id: req.operation_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(result["status"], "notCommitted");
        assert_eq!(result["resultStatus"], "unavailable");
        assert!(result.get("result").is_none());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn restart_replays_original_and_remapping_only_keeps_committed_fact() {
        let (temp, state, auth, doc, version, original) = test_fixture();
        let req = restore_request(&doc, &version);
        let _ = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            inner.sessions.clear();
            inner.editor_versions.clear();
            let placeholder = Core::new(temp.path().join("placeholder")).unwrap();
            drop(std::mem::replace(&mut inner.core, placeholder));
            inner.core = Core::new(temp.path().join("state")).unwrap();
        }
        let Json(replay) = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        assert_eq!(replay["result"]["source"], original);
        let mut changed = req.clone();
        changed.source.push('!');
        assert_eq!(
            restore(State(state.clone()), Extension(auth.clone()), Json(changed))
                .await
                .unwrap_err()
                .code,
            "operationConflict"
        );
        state
            .inner
            .lock()
            .unwrap()
            .config
            .files
            .get_mut("Shared")
            .unwrap()
            .path = "Target/Target.typ".into();
        let Json(remapped) = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        assert_eq!(remapped["status"], "committed");
        assert_eq!(remapped["resultStatus"], "unavailable");
        assert!(remapped.get("result").is_none());
        state
            .inner
            .lock()
            .unwrap()
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .remove("Shared");
        assert_eq!(
            status(
                State(state),
                Extension(auth),
                Json(Operation {
                    resource: "Shared".into(),
                    operation_id: req.operation_id
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn settling_unknown_operation_fences_late_restore_before_session_checks() {
        let (_temp, state, auth, doc, version, _original) = test_fixture();
        let req = restore_request(&doc, &version);
        let Json(settled) = settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                resource: "Shared".into(),
                operation_id: req.operation_id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(settled["status"], "cancelled");
        state.inner.lock().unwrap().sessions.clear();
        let Json(late) = restore(State(state), Extension(auth), Json(req))
            .await
            .unwrap();
        assert_eq!(late["status"], "cancelled");
        assert!(late.get("result").is_none());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn same_bytes_later_save_and_external_body_do_not_rebind_original_receipt() {
        let (_temp, state, auth, doc, version, original) = test_fixture();
        let req = restore_request(&doc, &version);
        let Json(restored) = restore(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            let doc = inner.open("alice", "Shared").unwrap();
            let scope = inner.actor_scope("alice");
            inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_save(
                        "Shared",
                        HostedSaveRequest {
                            session_id: doc["sessionId"].as_str().unwrap().into(),
                            base_id: doc["baseId"].as_str().unwrap().into(),
                            source: original.clone(),
                            operation_id: uuid::Uuid::new_v4().to_string(),
                        },
                    )
                })
                .unwrap();
        }
        let Json(noop) = status(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                resource: "Shared".into(),
                operation_id: req.operation_id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(noop["currentRevision"], restored["currentRevision"]);
        assert_eq!(noop["resultBasis"], "superseded");
        assert_eq!(noop["result"]["source"], original);
        {
            let inner = state.inner.lock().unwrap();
            std::fs::write(
                inner.config.root.join("Shared/Shared.typ"),
                original.clone() + "\nExternal body\n",
            )
            .unwrap();
        }
        let Json(external) = status(
            State(state),
            Extension(auth),
            Json(Operation {
                resource: "Shared".into(),
                operation_id: req.operation_id,
            }),
        )
        .await
        .unwrap();
        assert_eq!(external["status"], "committed");
        assert_eq!(external["resultStatus"], "available");
        assert_eq!(external["resultBasis"], "external");
        assert!(external["currentRevision"].is_null());
        assert_eq!(external["result"]["source"], original);
    }
}
