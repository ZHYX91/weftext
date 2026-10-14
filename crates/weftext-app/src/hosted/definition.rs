//! Core allocates definition identities; proposals only edit the original Draft.
//! Admission uses the ordinary actor source save with explicit original IDs.
use super::*;
use weftext_core::{
    ActorDefinitionProposal, ActorDefinitionStatus, ActorSaveStatus, HostedSaveRequest,
};
use weftext_data::{FieldPatch, QueryPlan, SavedQuery, SavedQueryBinding};

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    resource: String,
    session_id: String,
    base_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    operation_id: String,
    label: String,
    plan: QueryPlan,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    resource: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Reservations {
    session_id: String,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/query/definition/prepare", post(prepare))
        .route("/api/hosted/query/definition/operation", post(operation))
        .route("/api/hosted/query/definition/settle", post(settle))
        .route(
            "/api/hosted/query/definition/reservations",
            post(reservations),
        )
}
fn conflict() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "definitionUnavailable",
        "原定义提案、对象、准确基线或调用依据不可用；草稿已保留，请核对原操作。",
    )
}
fn core_error(_: CoreError) -> ApiError {
    conflict()
}
fn save_validation_error(error: ApiError) -> ApiError {
    if error.status == StatusCode::BAD_REQUEST {
        conflict()
    } else {
        error
    }
}
fn access(
    inner: &HostedInner,
    auth: &Auth,
    resource: &str,
    write: bool,
) -> Result<String, ApiError> {
    let actor = inner.actor(auth)?;
    let rights = inner.rights(&actor, resource)?;
    if !rights.query.definitions
        || write && !rights.write
        || !inner.config.files.get(resource).is_some_and(|f| f.editable)
    {
        return Err(unavailable());
    }
    Ok(actor)
}
fn valid_id(id: &str) -> Result<(), ApiError> {
    if uuid::Uuid::parse_str(id).is_ok_and(|id_value| id_value.to_string() == id) {
        Ok(())
    } else {
        Err(ApiError::bad("原操作编号须为规范 UUID"))
    }
}
fn fingerprint(req: &Prepare) -> Result<String, ApiError> {
    writing_api::validate_source(&req.source, req.source_hash.as_deref())?;
    if req.label.len() > 256
        || serde_json::to_vec(&req.plan).map_err(|_| conflict())?.len() > 256 * 1024
    {
        return Err(ApiError::bad("定义名称或提案超出预算"));
    }
    valid_id(&req.operation_id)?;
    // The derived initial binding is separately frozen by Core. Hash is only
    // a source check, so omitting/repeating a correct hash is the same intent.
    Ok(digest(
        &serde_json::to_vec(
            &json!({"resource":req.resource,"session":req.session_id,"base":req.base_id,
        "source":req.source,"serial":req.serial,"label":req.label,"plan":req.plan}),
        )
        .map_err(|_| conflict())?,
    ))
}
fn summary(status: &ActorDefinitionStatus) -> Value {
    // owner is a native identity capability and is never implicitly disclosed.
    json!({"operationId":status.operation_id,"resource":status.resource,"status":status.status,
        "key":status.key,"binding":status.binding,"saveOperationId":status.save_operation_id})
}
fn missing(resource: &str, id: &str) -> Value {
    json!({"resource":resource,"operationId":id,"status":"notRegistered"})
}
fn check_owner(
    inner: &mut HostedInner,
    resource: &str,
    status: &ActorDefinitionStatus,
) -> Result<(), ApiError> {
    let Some(owner) = &status.owner else {
        return Ok(());
    };
    let root = inner.config.root.clone();
    let path = root.join(&inner.config.files[resource].path);
    let file = inner
        .core
        .read_snapshot(&root, &[path.to_string_lossy().into_owned()])
        .map_err(core_error)?
        .files
        .remove(0);
    let witness = inner
        .core
        .take_snapshot_witness(&file.base_id)
        .map_err(core_error)?;
    if file.kind != "managed"
        || file.currentness != "accepted"
        || file.node_id.as_ref() != Some(&owner.node_id)
    {
        return Err(conflict());
    }
    let config = inner
        .core
        .read_config(&root, weftext_core::ConfigRole::Schema)
        .map_err(core_error)?;
    drop(
        inner
            .core
            .take_snapshot_witness(&config.base_id)
            .map_err(core_error)?,
    );
    if config.role_ref != format!("weftext-config://{}/schema", owner.workspace_id) {
        return Err(conflict());
    }
    inner
        .core
        .validate_snapshot_witness(&root, &file.path, &witness)
        .map_err(core_error)
}
fn pending_keys(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
) -> Result<BTreeSet<String>, ApiError> {
    let root = inner.config.root.clone();
    let path = root.join(&inner.config.files[resource].path);
    let scope = inner.actor_scope(actor);
    // A short-lived private source session allows recovery without trusting an
    // expired browser session. It does not mutate the personal Draft or save.
    inner
        .core
        .with_actor(&scope, |core| {
            let doc = core.open(&path, Some(root.clone()))?;
            let result = core.actor_definition_list(resource, &root, &doc.session_id);
            core.close_session(&doc.session_id)?;
            Ok(result?.into_iter().filter_map(|s| s.key).collect())
        })
        .map_err(core_error)
}
fn original_basis(patch: &FieldPatch) -> Result<String, ApiError> {
    // This endpoint creates a new carrier by insertion only. Never invent the
    // preimage of a replacement or use its old empty range on a changed Draft.
    if patch.from != patch.to {
        return Err(conflict());
    }
    let end = patch
        .from
        .checked_add(patch.text.len())
        .ok_or_else(conflict)?;
    if patch.source.get(patch.from..end) != Some(patch.text.as_str()) {
        return Err(conflict());
    }
    Ok(format!(
        "{}{}",
        patch.source.get(..patch.from).ok_or_else(conflict)?,
        patch.source.get(end..).ok_or_else(conflict)?
    ))
}
fn original_definition(proposal: &ActorDefinitionProposal) -> Result<SavedQuery, ApiError> {
    let key = proposal.status.key.as_ref().ok_or_else(conflict)?;
    let patch = proposal.patch.as_ref().ok_or_else(conflict)?;
    let definition = weftext_data::saved_queries(&patch.source)
        .map_err(|_| conflict())?
        .into_iter()
        .find(|d| &d.key == key)
        .ok_or_else(conflict)?;
    if !same_binding(&definition.binding, proposal.status.binding.as_ref()) {
        return Err(conflict());
    }
    Ok(definition)
}
fn same_binding(binding: &SavedQueryBinding, original: Option<&SavedQueryBinding>) -> bool {
    original.is_some_and(|b| {
        binding.schema == b.schema
            && binding.revision == b.revision
            && binding.language == b.language
    })
}
fn delivery(
    inner: &mut HostedInner,
    auth: &Auth,
    proposal: &ActorDefinitionProposal,
    request: Option<&Prepare>,
) -> Result<Value, ApiError> {
    let actor = access(inner, auth, &proposal.status.resource, true)?;
    check_owner(inner, &proposal.status.resource, &proposal.status)?;
    let patch = proposal.patch.as_ref().ok_or_else(conflict)?;
    let basis = original_basis(patch)?;
    if request.is_some_and(|r| r.source != basis) {
        return Err(conflict());
    }
    let definition = original_definition(proposal)?;
    let keys = pending_keys(inner, &actor, &proposal.status.resource)?;
    let validation = query::validate_definition(
        inner,
        auth,
        query::DefinitionValidationRequest {
            resource: &proposal.status.resource,
            source: &patch.source,
            plan: &definition.plan,
            binding: Some(&definition.binding),
            draft_keys: &keys,
        },
    )?;
    validation.guard.recheck(inner, auth)?;
    let serial = request.map_or(0, |r| r.serial);
    let Json(mut output) = data_api::patch_response(patch.clone(), serial, &basis)?;
    output["definition"] = json!(definition);
    output["basisSource"] = json!(basis);
    output["draftOnly"] = json!(true);
    let mut can_apply = false;
    if let Some(req) = request {
        if proposal.status.status == "reserved"
            && inner
                .session(auth, &req.session_id)
                .is_ok_and(|(_, s, r)| s.resource == req.resource && r.write)
            && writing_api::exact_version(
                &inner.editor_versions,
                &req.session_id,
                &req.source,
                req.serial,
            )
            .is_ok()
        {
            let scope = inner.actor_scope(&actor);
            can_apply = inner
                .core
                .with_actor(&scope, |core| {
                    core.check_session_base(&req.session_id, &req.base_id)
                })
                .is_ok();
        }
    } else {
        output.as_object_mut().unwrap().remove("serial");
    }
    output["canApply"] = json!(can_apply);
    output["recoveryOnly"] = json!(!can_apply);
    Ok(output)
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = access(inner, &auth, &req.resource, true)?;
        let intent = fingerprint(&req)?;
        let scope = inner.actor_scope(&actor);
        let known = inner.core.with_actor(&scope, |core| core.actor_definition_retry(&req.resource, &req.operation_id, &intent)).map_err(core_error)?;
        let proposal = match known {
            Some(proposal) => proposal,
            None => {
                let (_, session, _) = inner.session(&auth, &req.session_id)?;
                if session.resource != req.resource { return Err(unavailable()); }
                writing_api::exact_version(&inner.editor_versions, &req.session_id, &req.source, req.serial)?;
                let keys = pending_keys(inner, &actor, &req.resource)?;
                let validation = query::validate_definition(inner, &auth, query::DefinitionValidationRequest {
                    resource: &req.resource, source: &req.source, plan: &req.plan, binding: None, draft_keys: &keys,
                })?;
                validation.guard.recheck(inner, &auth)?;
                let root = inner.config.root.clone();
                let witnesses = validation.guard.witnesses.iter().map(|(_, w)| w).collect::<Vec<_>>();
                inner.core.with_guarded_snapshot_witnesses(&root, &witnesses, &validation.guard.owner_path, |core| {
                    core.with_actor(&scope, |core| core.actor_definition_reserve(&req.resource, &root, weftext_core::HostedDefinitionReservationRequest {
                        session_id: req.session_id.clone(), base_id: req.base_id.clone(), operation_id: req.operation_id.clone(),
                        intent_fingerprint: intent, binding: validation.binding.clone(),
                    }, |key, binding| weftext_data::upsert_saved_query_with_context(&req.source, None, key, &req.label, binding,
                        &validation.plan, &validation.schema, &validation.context).map_err(|_| CoreError::new("definitionQualification", "The exact native Draft proposal is unavailable"))))
                }).map_err(core_error)?
            }
        };
        check_owner(inner, &req.resource, &proposal.status)?;
        let mut output = summary(&proposal.status);
        if proposal.patch.is_some() {
            let delivered = delivery(inner, &auth, &proposal, Some(&req))?;
            output.as_object_mut().unwrap().extend(delivered.as_object().unwrap().clone());
        }
        Ok(output)
    }).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = access(inner, &auth, &req.resource, false)?;
            valid_id(&req.operation_id)?;
            let scope = inner.actor_scope(&actor);
            let Some(proposal) = inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_definition_details(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?
            else {
                return Ok(missing(&req.resource, &req.operation_id));
            };
            check_owner(inner, &req.resource, &proposal.status)?;
            let mut output = summary(&proposal.status);
            if proposal.patch.is_some() && inner.rights(&actor, &req.resource)?.write {
                match delivery(inner, &auth, &proposal, None) {
                    Ok(delivered) => output["proposal"] = delivered,
                    Err(_) => output["proposalUnavailable"] = json!(true),
                }
            }
            Ok(output)
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = access(inner, &auth, &req.resource, true)?;
            valid_id(&req.operation_id)?;
            let scope = inner.actor_scope(&actor);
            let original = inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_definition_status(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            if let Some(status) = &original {
                check_owner(inner, &req.resource, status)?;
            }
            let status = inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_definition_settle(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            Ok(summary(&status))
        })
        .await?,
    ))
}
async fn reservations(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Reservations>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&auth, &req.session_id)?;
        access(inner, &auth, &session.resource, false)?;
        let scope = inner.actor_scope(&actor);
        let root = inner.config.root.clone();
        let statuses = inner.core.with_actor(&scope, |core| core.actor_definition_list(&session.resource, &root, &req.session_id)).map_err(core_error)?;
        Ok(json!({"resource":session.resource,"reservations":statuses.iter().map(summary).collect::<Vec<_>>()}))
    }).await?))
}
/// Called under HostedInner's lock by the original source-save endpoint.
pub(super) fn save(
    inner: &mut HostedInner,
    auth: &Auth,
    req: HostedSaveRequest,
    ids: Vec<String>,
) -> Result<ActorSaveStatus, ApiError> {
    let (actor, session, rights) = inner.session(auth, &req.session_id)?;
    if !rights.write {
        return Err(unavailable());
    }
    if ids.len() > 32 {
        return Err(ApiError::bad("每次保存最多携带 32 个原定义提案"));
    }
    for id in &ids {
        valid_id(id)?;
    }
    let scope = inner.actor_scope(&actor);
    let root = inner.config.root.clone();
    if ids.is_empty() {
        return inner
            .core
            .with_actor(&scope, |core| core.actor_save(&session.resource, req))
            .map_err(HostedInner::core_error);
    }
    let original = inner
        .core
        .with_actor(&scope, |core| core.actor_save_status(&req.operation_id))
        .map_err(core_error)?;
    if original.is_some_and(|s| s.status != "notCommitted") {
        // Core verifies the exact source/ID-set fingerprint before returning the
        // original status. No new callee lookup can turn recovery into a writer.
        return inner
            .core
            .with_actor(&scope, |core| {
                core.actor_save_with_definition_reservations(&session.resource, &root, req, &ids)
            })
            .map_err(HostedInner::core_error);
    }
    writing_api::validate_source(&req.source, None)?;
    let statuses = ids
        .iter()
        .map(|id| {
            inner
                .core
                .with_actor(&scope, |core| {
                    core.actor_definition_status(&session.resource, id)
                })
                .map_err(core_error)?
                .ok_or_else(conflict)
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    // An unfinished native Draft can be stored by the ordinary writer. It has
    // no qualified executable carrier; Core independently prevents admission.
    let definitions = weftext_data::saved_queries(&req.source).unwrap_or_default();
    let keys = statuses
        .iter()
        .filter_map(|s| s.key.clone())
        .collect::<BTreeSet<_>>();
    let mut guards = vec![];
    let mut witness_pool = Vec::new();
    for status in &statuses {
        let Some(key) = &status.key else {
            continue;
        };
        let Some(definition) = definitions.iter().find(|d| &d.key == key) else {
            continue;
        };
        if status.status == "admitted" {
            continue;
        }
        if status.status != "reserved" {
            return Err(conflict());
        }
        access(inner, auth, &session.resource, true)?;
        check_owner(inner, &session.resource, status)?;
        if !same_binding(&definition.binding, status.binding.as_ref()) {
            return Err(conflict());
        }
        let mut validation = query::validate_definition(
            inner,
            auth,
            query::DefinitionValidationRequest {
                resource: &session.resource,
                source: &req.source,
                plan: &definition.plan,
                binding: Some(&definition.binding),
                draft_keys: &keys,
            },
        )
        .map_err(save_validation_error)?;
        validation
            .guard
            .recheck(inner, auth)
            .map_err(save_validation_error)?;
        // Core compares every duplicate by its actual path/object and frozen
        // parent/stamp/bytes before releasing it. The original witnesses stay
        // in the bounded pool until its final guarded write decision.
        Core::merge_snapshot_witnesses(
            &mut witness_pool,
            std::mem::take(&mut validation.guard.witnesses)
                .into_iter()
                .map(|(_, w)| w)
                .collect(),
        )
        .map_err(core_error)?;
        guards.push(validation.guard);
    }
    for guard in &guards {
        guard.recheck(inner, auth).map_err(save_validation_error)?;
    }
    let witnesses = witness_pool.iter().collect::<Vec<_>>();
    let owner_path = root.join(&inner.config.files[&session.resource].path);
    inner
        .core
        .with_guarded_snapshot_witnesses(&root, &witnesses, &owner_path, |core| {
            core.with_actor(&scope, |core| {
                core.actor_save_with_definition_reservations(&session.resource, &root, req, &ids)
            })
        })
        .map_err(HostedInner::core_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> Prepare {
        Prepare {
            resource: "Shared".into(),
            session_id: "original-session".into(),
            base_id: "base".into(),
            source: "原草稿😀".into(),
            serial: 7,
            source_hash: None,
            operation_id: uuid::Uuid::new_v4().to_string(),
            label: "weftext-query-new".into(),
            plan: QueryPlan::default(),
        }
    }
    #[test]
    fn original_intent_uses_exact_source_serial_and_plan_without_derived_binding() {
        let mut req = request();
        let original = fingerprint(&req).unwrap();
        req.source_hash = Some(weftext_engine::source_hash(&req.source));
        assert_eq!(fingerprint(&req).unwrap(), original);
        req.serial += 1;
        assert_ne!(fingerprint(&req).unwrap(), original);
        req.serial -= 1;
        req.plan.take = Some(1);
        assert_ne!(fingerprint(&req).unwrap(), original);
        req.source_hash = Some("0".repeat(64));
        assert_eq!(fingerprint(&req).unwrap_err().status, StatusCode::CONFLICT);
    }
    #[test]
    fn exact_original_insert_reconstructs_basis_and_utf16_offsets() {
        let before = "// 原注😀\n#metadata((id: \"owner\")) <weftext>\n后文🧑‍💻";
        let from = before.find("\n后文").unwrap();
        let text = "\n#metadata((kind: \"weftext.query\")) <weftext-query-new>\n";
        let patch = FieldPatch {
            from,
            to: from,
            text: text.into(),
            source: format!("{}{}{}", &before[..from], text, &before[from..]),
        };
        assert_eq!(original_basis(&patch).unwrap(), before);
        let Json(result) = data_api::patch_response(patch.clone(), 7, before).unwrap();
        assert_eq!(
            result["patches"][0]["from"],
            before[..from].encode_utf16().count()
        );
        assert_ne!(
            result["patches"][0]["from"],
            result["patches"][0]["byteFrom"]
        );
        let mut replacement = patch.clone();
        replacement.to += 1;
        assert!(original_basis(&replacement).is_err());
        let mut wrong = patch;
        wrong.source.replace_range(from..from + text.len(), "wrong");
        assert!(original_basis(&wrong).is_err());
    }
    #[test]
    fn durable_summary_does_not_implicitly_deliver_native_identity_or_source() {
        let status = ActorDefinitionStatus {
            operation_id: uuid::Uuid::new_v4().to_string(),
            resource: "Shared".into(),
            status: "reserved".into(),
            key: Some(uuid::Uuid::new_v4().to_string()),
            owner: Some(weftext_knowledge::NodeRef {
                workspace_id: uuid::Uuid::new_v4().to_string(),
                node_id: uuid::Uuid::new_v4().to_string(),
            }),
            binding: None,
            save_operation_id: None,
        };
        let value = summary(&status);
        assert!(value.get("owner").is_none());
        assert!(value.get("source").is_none());
        assert!(value.get("path").is_none());
        assert_eq!(value["status"], "reserved");
    }
}

#[cfg(all(test, windows))]
mod integration_tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, HostedState, Auth, Value) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("state")).unwrap();
        core.create_library(&root).unwrap();
        let local = core.create_note(&root, &root, "Shared").unwrap();
        core.close_session(&local.session_id).unwrap();
        let schema =
            include_bytes!("../../../../docs/design/examples/schema.authoring.json").to_vec();
        let operation = uuid::Uuid::new_v4().to_string();
        let plan = core
            .prepare_config_save(
                &root,
                weftext_core::ConfigRole::Schema,
                None,
                schema,
                &operation,
            )
            .unwrap();
        core.execute_operation(&operation, &plan.plan_id).unwrap();
        let rights = Rights {
            read: true,
            write: true,
            fields: BTreeMap::from([(
                "tasks/status".into(),
                FieldRights {
                    read: true,
                    write: false,
                },
            )]),
            query: query::QueryRights {
                definitions: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let users = BTreeMap::from([(
            "alice".into(),
            User {
                query_providers: Default::default(),
                epoch: 0,
                token_sha256: digest(b"token"),
                enabled: true,
                grants: BTreeMap::from([("Shared".into(), rights)]),
                sync_owner: false,
            },
        )]);
        let mut inner = HostedInner {
            core,
            config: Config {
                deployment: "definitions-test".into(),
                bind: "127.0.0.1:0".parse().unwrap(),
                origin: None,
                allow_insecure_http: false,
                root,
                admin_token_sha256: digest(b"admin"),
                files: BTreeMap::from([(
                    "Shared".into(),
                    FileEntry {
                        path: "Shared/Shared.typ".into(),
                        title: "Shared".into(),
                        editable: true,
                        importable: false,
                        inputs: vec![],
                    },
                )]),
                users: users.clone(),
                sync_identity: None,
            },
            policy: Policy {
                deployment: "definitions-test".into(),
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
        };
        let doc = inner.open("alice", "Shared").unwrap();
        writing_api::observe_version(
            &mut inner.editor_versions,
            doc["sessionId"].as_str().unwrap(),
            doc["source"].as_str().unwrap(),
            1,
        )
        .unwrap();
        let auth = Auth {
            digest: digest(b"token"),
        };
        // A zero native render gate proves the creation path never executes a
        // parameterless preview or compiles native products.
        let state = HostedState {
            inner: Arc::new(Mutex::new(inner)),
            origin: Arc::from("http://localhost"),
            host: Arc::from("localhost"),
            render_gate: Arc::new(tokio::sync::Semaphore::new(0)),
        };
        (temp, state, auth, doc)
    }
    fn req(doc: &Value, plan: QueryPlan) -> Prepare {
        Prepare {
            resource: "Shared".into(),
            session_id: doc["sessionId"].as_str().unwrap().into(),
            base_id: doc["baseId"].as_str().unwrap().into(),
            source: doc["source"].as_str().unwrap().into(),
            serial: 1,
            source_hash: None,
            operation_id: uuid::Uuid::new_v4().to_string(),
            label: "weftext-query-new".into(),
            plan,
        }
    }
    #[tokio::test]
    async fn required_parameter_preparation_is_static_and_original_source_save_admits() {
        let (_temp, state, auth, doc) = fixture();
        let plan = serde_json::from_value(json!({"where":"status == wanted","select":[{"name":"status","expr":"status"}],
            "advanced":{"parameters":[{"name":"wanted","shape":{"kind":"scalar","semantic":"text"}}]}})).unwrap();
        let req = req(&doc, plan);
        let Json(output) = tokio::time::timeout(
            Duration::from_secs(5),
            prepare(
                State(state.clone()),
                Extension(auth.clone()),
                Json(req.clone()),
            ),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(output["status"], "reserved");
        assert_eq!(output["canApply"], true);
        assert_eq!(output["basisSource"], doc["source"]);
        let mut inner = state.inner.lock().unwrap();
        let path = inner.config.root.join("Shared/Shared.typ");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), req.source);
        assert!(fields::query_catalog(&inner, "alice").is_empty());
        let status = save(
            &mut inner,
            &auth,
            HostedSaveRequest {
                session_id: req.session_id,
                base_id: req.base_id,
                source: output["source"].as_str().unwrap().into(),
                operation_id: uuid::Uuid::new_v4().to_string(),
            },
            vec![req.operation_id.clone()],
        )
        .unwrap();
        assert_eq!(status.status, "committed");
        let scope = inner.actor_scope("alice");
        let status = inner
            .core
            .with_actor(&scope, |core| {
                core.actor_definition_status("Shared", &req.operation_id)
            })
            .unwrap()
            .unwrap();
        assert_eq!(status.status, "admitted");
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            output["source"].as_str().unwrap()
        );
    }
    #[tokio::test]
    async fn source_read_write_does_not_allocate_a_body_definition_without_product_grant() {
        let (_temp, state, auth, doc) = fixture();
        let req = req(
            &doc,
            serde_json::from_value(json!({"select":[{"name":"body","expr":"body"}]})).unwrap(),
        );
        let error = prepare(State(state.clone()), Extension(auth), Json(req.clone()))
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::NOT_FOUND);
        let mut inner = state.inner.lock().unwrap();
        let scope = inner.actor_scope("alice");
        assert!(
            inner
                .core
                .with_actor(&scope, |core| core
                    .actor_definition_status("Shared", &req.operation_id))
                .unwrap()
                .is_none()
        );
    }
    #[tokio::test]
    async fn hidden_and_unknown_field_registrations_have_identical_refusals() {
        let (_temp, state, auth, doc) = fixture();
        let mut refusal = None;
        for expr in [
            "field(\"tasks/due\")",
            "field(\"private/nonexistent-field\")",
            "due",
            "nonexistent_field",
        ] {
            let req = req(
                &doc,
                serde_json::from_value(json!({"select":[{"name":"probe","expr":expr}]})).unwrap(),
            );
            let error = prepare(
                State(state.clone()),
                Extension(auth.clone()),
                Json(req.clone()),
            )
            .await
            .unwrap_err();
            assert_eq!(error.status, StatusCode::NOT_FOUND);
            let current = (error.status, error.code, error.message);
            assert_eq!(refusal.get_or_insert(current.clone()), &current);
            let mut inner = state.inner.lock().unwrap();
            let scope = inner.actor_scope("alice");
            assert!(
                inner
                    .core
                    .with_actor(&scope, |core| core
                        .actor_definition_status("Shared", &req.operation_id))
                    .unwrap()
                    .is_none()
            );
        }
    }
    #[tokio::test]
    async fn unfinished_source_keeps_original_file_draft_and_reserved_key_until_repaired_save() {
        let (_temp, state, auth, doc) = fixture();
        let req = req(
            &doc,
            serde_json::from_value(json!({"select":[{"name":"status","expr":"status"}]})).unwrap(),
        );
        let Json(output) = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(req.clone()),
        )
        .await
        .unwrap();
        let mut inner = state.inner.lock().unwrap();
        let unfinished = format!(
            "{}\n#let still_editing = (\n",
            output["source"].as_str().unwrap()
        );
        let base = req.base_id.clone();
        let failed_id = uuid::Uuid::new_v4().to_string();
        let error = save(
            &mut inner,
            &auth,
            HostedSaveRequest {
                session_id: req.session_id.clone(),
                base_id: req.base_id,
                source: unfinished.clone(),
                operation_id: failed_id.clone(),
            },
            vec![req.operation_id.clone()],
        )
        .unwrap_err();
        assert_eq!(error.status, StatusCode::CONFLICT);
        let scope = inner.actor_scope("alice");
        assert_eq!(
            inner
                .core
                .with_actor(&scope, |core| core
                    .actor_definition_status("Shared", &req.operation_id))
                .unwrap()
                .unwrap()
                .status,
            "reserved"
        );
        assert_eq!(
            std::fs::read_to_string(inner.config.root.join("Shared/Shared.typ")).unwrap(),
            req.source
        );
        let reopened = inner.open("alice", "Shared").unwrap();
        assert_eq!(reopened["recoveredDraft"], unfinished);
        inner
            .core
            .with_actor(&scope, |core| core.actor_settle("Shared", &failed_id))
            .unwrap();
        save(
            &mut inner,
            &auth,
            HostedSaveRequest {
                session_id: req.session_id,
                base_id: base,
                source: format!("{unfinished})\n"),
                operation_id: uuid::Uuid::new_v4().to_string(),
            },
            vec![req.operation_id.clone()],
        )
        .unwrap();
        assert_eq!(
            inner
                .core
                .with_actor(&scope, |core| core
                    .actor_definition_status("Shared", &req.operation_id))
                .unwrap()
                .unwrap()
                .status,
            "admitted"
        );
    }
}
