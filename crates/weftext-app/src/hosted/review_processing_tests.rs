//! Hosted processing authorization tests. The exact scan is seeded after a real
//! review read; these cases exercise accepting that frozen scan, not the worker.
use super::*;

fn settings(version: &str) -> Value {
    json!({
        "version": 1,
        "processing": {
            "schemaVersion": 1,
            "defaults": knowledge::ProcessingSettings::default(),
            "rules": [{
                "id": "shared.saved",
                "version": version,
                "category": "typo",
                "text": "Saved",
                "message": format!("Shared dictionary {version}"),
                "replacement": "Reviewed",
                "scope": {"universal": true}
            }]
        }
    })
}

fn write_settings(state: &HostedState, version: &str) {
    let inner = state.inner.lock().unwrap();
    // A fixture edit of the real author-owned settings file, never a new store.
    std::fs::write(
        inner.config.root.join(".weftext-settings.json"),
        serde_json::to_vec_pretty(&settings(version)).unwrap(),
    )
    .unwrap();
}

fn settings_access(state: &HostedState, actor: &str, allowed: bool) {
    state
        .inner
        .lock()
        .unwrap()
        .policy
        .users
        .get_mut(actor)
        .unwrap()
        .library = serde_json::from_value(json!({
        "settings": {"read": allowed, "write": allowed}
    }))
    .unwrap();
}

fn fixture() -> (tempfile::TempDir, HostedState, Auth) {
    let (temp, state, auth, _, _, _) = history_api::test_fixture();
    {
        let mut inner = state.inner.lock().unwrap();
        for actor in ["alice", "bob"] {
            inner
                .policy
                .users
                .get_mut(actor)
                .unwrap()
                .grants
                .get_mut("Shared")
                .unwrap()
                .annotations = ReviewRights {
                read: true,
                write: true,
                target: true,
                accept: true,
                proofread: true,
            };
        }
    }
    settings_access(&state, "alice", true);
    settings_access(&state, "bob", true);
    write_settings(&state, "1");
    (temp, state, auth)
}

async fn options(state: &HostedState, auth: &Auth) -> knowledge::ProofreadOptions {
    let profile = library::test_processing_profile(state, auth)
        .await
        .unwrap()
        .0;
    let source: Value = serde_json::from_str(profile["source"].as_str().unwrap()).unwrap();
    let document: knowledge::ProcessingDocument =
        serde_json::from_value(source["processing"].clone()).unwrap();
    knowledge::ProofreadOptions {
        characters: false,
        pairs: false,
        sensitive: false,
        rules: document.rules,
        processing: document.defaults,
        processing_origin: Some(knowledge::SharedProcessingOrigin {
            source: "shared-library".into(),
            base_id: profile["basis"].as_str().unwrap().into(),
            revision: None,
        }),
        ..Default::default()
    }
}

struct Seed {
    basis: String,
    intent: Value,
}

async fn seed_scan(state: &HostedState, auth: &Auth, options: knowledge::ProofreadOptions) -> Seed {
    let response = read(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Read {
            resource: "Shared".into(),
            asset: None,
        }),
    )
    .await
    .unwrap()
    .0;
    let basis = response["basis"].as_str().unwrap().to_owned();
    let mut inner = state.inner.lock().unwrap();
    let b = inner.review.bases[&basis].clone();
    let node = b.node.as_ref().unwrap();
    let from = node.source.find("Saved B").unwrap();
    let to = from + "Saved".len();
    let tree = weftext_engine::RenderNode {
        key: "exact-saved-prose".into(),
        kind: "text".into(),
        tag: None,
        text: Some("Saved".into()),
        attrs: BTreeMap::new(),
        children: vec![],
        origin: Some(weftext_engine::Origin {
            path: node.path.clone(),
            from,
            to,
            utf16_from: node.source[..from].encode_utf16().count(),
            utf16_to: node.source[..to].encode_utf16().count(),
            raw: "Saved".into(),
            syntax_kind: "Text".into(),
            is_main: true,
        }),
        writable: true,
    };
    let result = knowledge::proofread(&tree, &node.source, &options).unwrap();
    assert_eq!(result.findings.len(), 1);
    assert_eq!(
        result.findings[0].source_range.as_ref().unwrap().raw,
        "Saved"
    );
    assert!(result.findings[0].executable);
    let finding = result.findings[0].id.clone();
    let processing = options.processing_origin.as_ref().map(|origin| {
        library::capture_processing(
            &mut inner,
            "alice",
            origin,
            &b.source.as_ref().unwrap().path,
        )
        .unwrap()
    });
    let environment = library::environment(&mut inner, "alice").unwrap();
    let scan_id = uuid::Uuid::new_v4().to_string();
    inner.review.scans.insert(
        scan_id.clone(),
        Scan {
            actor: "alice".into(),
            epoch: b.epoch,
            resource: "Shared".into(),
            basis: basis.clone(),
            options_hash: digest(&serde_json::to_vec(&options).unwrap()),
            result,
            inputs: BTreeMap::new(),
            dependencies: vec![],
            environment,
            processing,
            expires: Instant::now() + TTL,
        },
    );
    Seed {
        basis,
        intent: json!({
            "kind": "annotationProofread",
            "annotationLocale": "zh-CN",
            "scanId": scan_id,
            "options": options,
            "findings": [finding],
            "ignore": false
        }),
    }
}

async fn propose(state: &HostedState, auth: &Auth, seed: &Seed, id: &str) -> ApiResult {
    prepare(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Prepare {
            resource: "Shared".into(),
            basis: seed.basis.clone(),
            operation_id: id.into(),
            intent: seed.intent.clone(),
        }),
    )
    .await
}

async fn status(state: &HostedState, auth: &Auth, id: &str) -> Value {
    operation(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Operation {
            resource: "Shared".into(),
            operation_id: id.into(),
        }),
    )
    .await
    .unwrap()
    .0
}

async fn apply(state: &HostedState, auth: &Auth, plan: &Value) -> ApiResult {
    commit(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Commit {
            resource: "Shared".into(),
            operation_id: plan["operationId"].as_str().unwrap().into(),
            plan_id: plan["planId"].as_str().unwrap().into(),
        }),
    )
    .await
}

fn author_files(state: &HostedState) -> (Vec<u8>, Option<Vec<u8>>) {
    let inner = state.inner.lock().unwrap();
    (
        std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap(),
        std::fs::read(inner.config.root.join("Shared/weftext.annotations.json")).ok(),
    )
}

#[tokio::test]
async fn shared_origin_requires_opaque_actor_bound_settings_read_token() {
    let (_temp, state, auth) = fixture();
    let options = options(&state, &auth).await;
    let origin = options.processing_origin.as_ref().unwrap().clone();
    let owner = {
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let owner = root
            .join("Shared/Shared.typ")
            .to_string_lossy()
            .into_owned();
        let raw = inner
            .core
            .read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)
            .unwrap();
        assert_ne!(origin.base_id, raw.base_id);
        let mut forged = origin.clone();
        forged.base_id = raw.base_id.clone();
        assert!(library::capture_processing(&mut inner, "alice", &forged, &owner).is_err());
        assert!(library::capture_processing(&mut inner, "bob", &origin, &owner).is_err());
        let _ = inner.core.take_snapshot_witness(&raw.base_id).unwrap();
        assert!(library::capture_processing(&mut inner, "alice", &origin, &owner).is_ok());
        owner
    };
    settings_access(&state, "alice", false);
    {
        let mut inner = state.inner.lock().unwrap();
        assert!(inner.grant("alice", "Shared").unwrap().read);
        assert!(library::capture_processing(&mut inner, "alice", &origin, &owner).is_err());
    }
    assert!(
        library::test_processing_profile(&state, &auth)
            .await
            .is_err()
    );
    let before = read(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Read {
            resource: "Shared".into(),
            asset: None,
        }),
    )
    .await
    .unwrap()
    .0;
    let denied = proofread(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(Proofread {
            resource: "Shared".into(),
            basis: before["basis"].as_str().unwrap().into(),
            options,
        }),
    )
    .await
    .unwrap_err();
    // The fixture has no worker permit: authorization must fail before "busy".
    assert_eq!(denied.code, unavailable().code);
    assert!(state.inner.lock().unwrap().review.scans.is_empty());
    assert!(author_files(&state).1.is_none());
}

#[tokio::test]
async fn captured_scan_survives_library_profile_expiry_and_commits_once() {
    let (_temp, state, auth) = fixture();
    let before = author_files(&state);
    let options = options(&state, &auth).await;
    let seed = seed_scan(&state, &auth, options).await;
    {
        let mut inner = state.inner.lock().unwrap();
        library::test_expire_processing_profiles(&mut inner);
        let scan = inner.review.scans.values().next().unwrap();
        scan.processing
            .as_ref()
            .unwrap()
            .validate(&inner, "alice")
            .unwrap();
    }
    let id = uuid::Uuid::new_v4().to_string();
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    assert_eq!(plan["status"], "prepared");
    assert_eq!(plan["preview"]["count"], 1);
    assert_eq!(plan["commitAllowed"], true);
    assert_eq!(
        apply(&state, &auth, &plan).await.unwrap().0["status"],
        "committed"
    );
    let after = author_files(&state);
    assert_eq!(after.0, before.0);
    let annotations = knowledge::decode_annotations(after.1.as_deref().unwrap()).unwrap();
    assert_eq!(annotations.records.len(), 1);
    assert_eq!(annotations.records[0].body, "Shared dictionary 1");
    assert_eq!(
        apply(&state, &auth, &plan).await.unwrap().0["status"],
        "committed"
    );
    assert_eq!(author_files(&state), after);
}

#[tokio::test]
async fn prepared_shared_dictionary_status_rejects_equal_bytes_settings_replacement() {
    let (temp, state, auth) = fixture();
    let before = author_files(&state);
    {
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let observed = inner
            .core
            .read_config(&root, weftext_core::ConfigRole::WorkspaceSettings)
            .unwrap();
        assert!(observed.revision.is_none());
    }
    let options = options(&state, &auth).await;
    let seed = seed_scan(&state, &auth, options).await;
    let id = uuid::Uuid::new_v4().to_string();
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    {
        let mut inner = state.inner.lock().unwrap();
        let actor_scope = scope(&inner, "alice");
        let root = inner.config.root.clone();
        let result = inner.core.with_actor_for_root(&actor_scope, &root, |core| {
            core.validate_actor_feature_settings_dependency("Shared", &id)
        });
        assert!(result.is_ok(), "{result:?}");
    }
    assert_eq!(plan["commitAllowed"], true);
    let path = state
        .inner
        .lock()
        .unwrap()
        .config
        .root
        .join(".weftext-settings.json");
    let raw = std::fs::read(&path).unwrap();
    std::fs::rename(&path, temp.path().join("retained-original-settings.json")).unwrap();
    std::fs::write(&path, &raw).unwrap();
    let hidden = status(&state, &auth, &id).await;
    assert_eq!(hidden["status"], "prepared");
    assert_eq!(hidden["preview"], json!({}));
    assert_eq!(hidden["planId"], Value::Null);
    assert_eq!(hidden["commitAllowed"], false);
    assert_eq!(propose(&state, &auth, &seed, &id).await.unwrap().0, hidden);
    assert!(apply(&state, &auth, &plan).await.is_err());
    assert_eq!(std::fs::read(path).unwrap(), raw);
    assert_eq!(author_files(&state), before);
}

#[tokio::test]
async fn settled_shared_dictionary_history_reauthorizes_without_revalidating_old_defaults() {
    let (_temp, state, auth) = fixture();
    let options = options(&state, &auth).await;
    let seed = seed_scan(&state, &auth, options).await;
    let id = uuid::Uuid::new_v4().to_string();
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    assert_eq!(
        apply(&state, &auth, &plan).await.unwrap().0["status"],
        "committed"
    );
    let after = author_files(&state);
    write_settings(&state, "2");
    let history = status(&state, &auth, &id).await;
    assert_eq!(history["status"], "committed");
    assert_eq!(history["preview"]["count"], 1);
    assert_eq!(history["commitAllowed"], false);
    assert_eq!(apply(&state, &auth, &plan).await.unwrap().0, history);
    settings_access(&state, "alice", false);
    let hidden = status(&state, &auth, &id).await;
    assert_eq!(hidden["status"], "committed");
    assert_eq!(hidden["preview"], json!({}));
    assert_eq!(hidden["planId"], Value::Null);
    assert_eq!(author_files(&state), after);
}

#[tokio::test]
async fn changed_shared_settings_or_revoked_read_prevents_preparing_a_scan() {
    for revoke in [false, true] {
        let (_temp, state, auth) = fixture();
        let before = author_files(&state);
        let options = options(&state, &auth).await;
        let seed = seed_scan(&state, &auth, options).await;
        if revoke {
            settings_access(&state, "alice", false);
        } else {
            write_settings(&state, "2");
        }
        let id = uuid::Uuid::new_v4().to_string();
        assert!(propose(&state, &auth, &seed, &id).await.is_err());
        let absent = status(&state, &auth, &id).await;
        assert_eq!(absent["status"], "notRegistered");
        assert_eq!(absent["preview"], json!({}));
        assert_eq!(absent["commitAllowed"], false);
        assert_eq!(author_files(&state), before);
    }
}

#[tokio::test]
async fn prepared_shared_plan_loses_preview_and_commit_when_settings_change_or_read_is_revoked() {
    for revoke in [false, true] {
        let (_temp, state, auth) = fixture();
        let before = author_files(&state);
        let options = options(&state, &auth).await;
        let seed = seed_scan(&state, &auth, options).await;
        let id = uuid::Uuid::new_v4().to_string();
        let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
        assert_eq!(plan["commitAllowed"], true);
        if revoke {
            settings_access(&state, "alice", false);
        } else {
            write_settings(&state, "2");
        }
        let unavailable = status(&state, &auth, &id).await;
        assert_eq!(unavailable["status"], "prepared");
        assert_eq!(unavailable["preview"], json!({}));
        assert_eq!(unavailable["commitAllowed"], false);
        assert!(unavailable["planId"].is_null());
        assert!(apply(&state, &auth, &plan).await.is_err());
        assert_eq!(author_files(&state), before);
    }
}

#[tokio::test]
async fn committed_shared_operation_survives_restart_and_new_defaults_but_reauthorizes_preview() {
    let (temp, state, auth) = fixture();
    let options = options(&state, &auth).await;
    let seed = seed_scan(&state, &auth, options).await;
    let id = uuid::Uuid::new_v4().to_string();
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    let committed = apply(&state, &auth, &plan).await.unwrap().0;
    let saved = author_files(&state);
    {
        let mut inner = state.inner.lock().unwrap();
        inner.sessions.clear();
        inner.editor_versions.clear();
        inner.review = Default::default();
        inner.library = Default::default();
        let placeholder = Core::new(temp.path().join("placeholder")).unwrap();
        drop(std::mem::replace(&mut inner.core, placeholder));
        inner.core = Core::new(temp.path().join("state")).unwrap();
    }
    write_settings(&state, "2");
    let replay = propose(&state, &auth, &seed, &id).await.unwrap().0;
    assert_eq!(replay["status"], "committed");
    assert_eq!(replay["preview"], committed["preview"]);
    assert_eq!(
        apply(&state, &auth, &plan).await.unwrap().0["status"],
        "committed"
    );
    assert_eq!(author_files(&state), saved);
    let annotations = knowledge::decode_annotations(saved.1.as_deref().unwrap()).unwrap();
    assert_eq!(annotations.records.len(), 1);
    assert_eq!(annotations.records[0].body, "Shared dictionary 1");
    settings_access(&state, "alice", false);
    let hidden = status(&state, &auth, &id).await;
    assert_eq!(hidden["status"], "committed");
    assert_eq!(hidden["preview"], json!({}));
    assert_eq!(
        apply(&state, &auth, &plan).await.unwrap().0["preview"],
        json!({})
    );
    assert_eq!(author_files(&state), saved);
    settings_access(&state, "alice", true);
    assert_eq!(
        status(&state, &auth, &id).await["preview"],
        committed["preview"]
    );
}

#[tokio::test]
async fn personal_scan_without_shared_origin_does_not_require_settings_read() {
    let (_temp, state, auth) = fixture();
    settings_access(&state, "alice", false);
    let options = knowledge::ProofreadOptions {
        characters: false,
        pairs: false,
        sensitive: false,
        rules: vec![knowledge::ProofreadRule {
            id: "personal.saved".into(),
            version: "1".into(),
            category: "typo".into(),
            text: "Saved".into(),
            message: "Personal dictionary".into(),
            replacement: Some("Reviewed".into()),
            scope: Some(knowledge::ProcessingRuleScope {
                universal: true,
                ..Default::default()
            }),
        }],
        ..Default::default()
    };
    let seed = seed_scan(&state, &auth, options).await;
    assert!(
        state
            .inner
            .lock()
            .unwrap()
            .review
            .scans
            .values()
            .all(|scan| scan.processing.is_none())
    );
    let id = uuid::Uuid::new_v4().to_string();
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    assert_eq!(plan["commitAllowed"], true);
    let committed = apply(&state, &auth, &plan).await.unwrap().0;
    assert_eq!(committed["status"], "committed");
    assert_eq!(committed["preview"]["count"], 1);
    let annotations = author_files(&state).1.unwrap();
    assert_eq!(
        knowledge::decode_annotations(&annotations)
            .unwrap()
            .records
            .len(),
        1
    );
}

#[tokio::test]
async fn annotation_content_language_is_required_frozen_and_cannot_reuse_an_operation() {
    let (_temp, state, auth) = fixture();
    let before = author_files(&state);
    let options = options(&state, &auth).await;
    let seed = seed_scan(&state, &auth, options).await;
    let id = uuid::Uuid::new_v4().to_string();
    for supplied in [None, Some(json!("fr-FR")), Some(json!(7))] {
        let mut intent = seed.intent.clone();
        intent.as_object_mut().unwrap().remove("annotationLocale");
        if let Some(value) = supplied {
            intent["annotationLocale"] = value;
        }
        let invalid = Seed {
            basis: seed.basis.clone(),
            intent,
        };
        let error = propose(&state, &auth, &invalid, &id).await.unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(
            error.explanation.key(),
            "error.api.hostedReview.annotationLocaleRequired"
        );
        assert_eq!(status(&state, &auth, &id).await["status"], "notRegistered");
        assert_eq!(author_files(&state), before);
    }
    let plan = propose(&state, &auth, &seed, &id).await.unwrap().0;
    assert_eq!(plan["status"], "prepared");
    assert_eq!(plan["preview"]["annotationLocale"], "zh-CN");
    assert!(plan["preview"]["summary"]["key"].is_string());
    assert_eq!(author_files(&state), before);
    let mut changed = Seed {
        basis: seed.basis.clone(),
        intent: seed.intent.clone(),
    };
    changed.intent["annotationLocale"] = json!("en-US");
    assert_eq!(
        propose(&state, &auth, &changed, &id)
            .await
            .unwrap_err()
            .status,
        StatusCode::CONFLICT
    );
    assert_eq!(propose(&state, &auth, &seed, &id).await.unwrap().0, plan);
    let committed = apply(&state, &auth, &plan).await.unwrap().0;
    assert_eq!(committed["status"], "committed");
    let after = author_files(&state);
    assert_eq!(after.0, before.0);
    let annotations = knowledge::decode_annotations(after.1.as_deref().unwrap()).unwrap();
    assert_eq!(annotations.records.len(), 1);
    // The explicit content choice never translates an author's own rule text.
    assert_eq!(annotations.records[0].body, "Shared dictionary 1");
    assert_eq!(
        annotations.records[0].unknown["proofreading"]["annotationLocale"],
        "zh-CN"
    );
    assert_eq!(
        propose(&state, &auth, &changed, &id)
            .await
            .unwrap_err()
            .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        propose(&state, &auth, &seed, &id).await.unwrap().0["status"],
        "committed"
    );
    assert_eq!(author_files(&state), after);
}
