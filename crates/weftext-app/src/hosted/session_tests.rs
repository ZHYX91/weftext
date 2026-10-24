use super::*;

fn session_id(value: &Value) -> String {
    value["sessionId"].as_str().unwrap().to_owned()
}

async fn original_status(
    state: &HostedState,
    auth: &Auth,
    resource: &str,
    operation: &str,
) -> ApiResult {
    status(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(OperationStatus {
            resource: resource.into(),
            operation_id: operation.into(),
        }),
    )
    .await
}

#[tokio::test]
async fn original_save_status_authorizes_frozen_resource_before_reading_private_actor_body() {
    let (temp, state, auth, doc, _, _) = history_api::test_fixture();
    assert!(
        serde_json::from_value::<OperationStatus>(json!({"operationId":uuid::Uuid::new_v4()}))
            .is_err()
    );
    let operation = uuid::Uuid::new_v4().to_string();
    let source = doc["source"].as_str().unwrap().to_owned() + "\nActual private saved S";
    let Json(saved) = save(
        State(state.clone()),
        Extension(auth.clone()),
        crate::RequestJson(SourceSaveRequest {
            input_revision: None,
            session_id: session_id(&doc),
            base_id: doc["baseId"].as_str().unwrap().into(),
            source: source.clone(),
            operation_id: operation.clone(),
            definition_reservations: vec![],
        }),
    )
    .await
    .unwrap();
    assert_eq!(saved["status"], "committed");
    let scope = state.inner.lock().unwrap().actor_scope("alice");
    let path = temp
        .path()
        .join("state/actors")
        .join(digest(scope.as_bytes()))
        .join("operations")
        .join(format!("{operation}.json"));
    let original = std::fs::read(&path).unwrap();
    let record: Value = serde_json::from_slice(&original).unwrap();
    let core_id = record["coreOperationId"].as_str().unwrap();
    let plan = temp
        .path()
        .join("state/journal")
        .join(&core_id[..2])
        .join(format!("{core_id}.plan.json"));
    let commit = temp
        .path()
        .join("state/journal")
        .join(&core_id[..2])
        .join(format!("{core_id}.commit.json"));
    let plan_bytes = std::fs::read(&plan).unwrap();
    let commit_bytes = std::fs::read(&commit).unwrap();
    let marker = b",\"cancelled\":";
    let at = original
        .windows(marker.len())
        .position(|part| part == marker)
        .unwrap();
    let mut damaged = original[..at].to_vec();
    damaged.extend_from_slice(b",\"cancelled\":BROKEN_PRIVATE_BODY");
    std::fs::write(&path, &damaged).unwrap();
    {
        let mut inner = state.inner.lock().unwrap();
        let grants = &mut inner.policy.users.get_mut("alice").unwrap().grants;
        let mut target = grants["Shared"].clone();
        target.read = true;
        grants.insert("Target".into(), target);
        grants.get_mut("Shared").unwrap().read = false;
    }
    assert_eq!(
        original_status(&state, &auth, "Shared", &operation)
            .await
            .unwrap_err()
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        original_status(&state, &auth, "Target", &operation)
            .await
            .unwrap_err()
            .code,
        "operationConflict"
    );
    assert_eq!(std::fs::read(&path).unwrap(), damaged);
    state
        .inner
        .lock()
        .unwrap()
        .policy
        .users
        .get_mut("alice")
        .unwrap()
        .grants
        .get_mut("Shared")
        .unwrap()
        .read = true;
    assert_eq!(
        original_status(&state, &auth, "Shared", &operation)
            .await
            .unwrap_err()
            .code,
        "invalidState"
    );
    std::fs::write(&path, &original).unwrap();
    let Json(recovered) = original_status(&state, &auth, "Shared", &operation)
        .await
        .unwrap();
    assert_eq!(recovered, saved);
    assert_eq!(std::fs::read(plan).unwrap(), plan_bytes);
    assert_eq!(std::fs::read(commit).unwrap(), commit_bytes);
    assert_eq!(std::fs::read(path).unwrap(), original);
    let root = state.inner.lock().unwrap().config.root.clone();
    assert_eq!(
        std::fs::read_to_string(root.join("Shared/Shared.typ")).unwrap(),
        source
    );
}

#[tokio::test]
async fn repeated_open_and_close_releases_only_the_calling_actors_coordinate() {
    let (_temp, state, auth, original, _, _) = history_api::test_fixture();
    for _ in 0..40 {
        let Json(opened) = open(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Open {
                resource: "Shared".into(),
            }),
        )
        .await
        .unwrap();
        let id = session_id(&opened);
        let foreign = close(
            State(state.clone()),
            Extension(Auth {
                digest: digest(b"bob"),
            }),
            crate::RequestJson(SessionRequest {
                session_id: id.clone(),
            }),
        )
        .await;
        assert_eq!(foreign.unwrap_err().status, StatusCode::GONE);
        let Json(renewed) = keepalive(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(SessionRequest {
                session_id: id.clone(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(renewed["active"], true);
        let Json(closed) = close(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(SessionRequest { session_id: id }),
        )
        .await
        .unwrap();
        assert_eq!(closed["closed"], true);
    }
    let inner = state.inner.lock().unwrap();
    assert_eq!(inner.sessions.len(), 1);
    assert!(inner.sessions.contains_key(&session_id(&original)));
}

#[test]
fn quota_admission_reaps_an_abandoned_session_and_preserves_its_persisted_draft() {
    let (_temp, state, auth, original, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    let original_id = session_id(&original);
    let bob = inner.open("bob", "Shared").unwrap();
    for _ in 1..32 {
        inner.open("alice", "Shared").unwrap();
    }
    assert!(inner.open("alice", "Shared").is_err());
    let personal = format!("{}\n个人恢复稿 😀\n", original["source"].as_str().unwrap());
    let scope = inner.actor_scope("alice");
    let root = inner.config.root.clone();
    inner
        .core
        .with_actor_for_root(&scope, &root, |core| {
            core.persist_draft(&original_id, &personal)
        })
        .unwrap();
    *inner.sessions[&original_id].expires.lock().unwrap() = Instant::now();
    assert_eq!(
        inner.session(&auth, &original_id).err().unwrap().code,
        "sessionExpired"
    );

    let fresh = inner.open("alice", "Shared").unwrap();
    assert_eq!(fresh["recoveredDraft"], personal);
    assert!(!inner.sessions.contains_key(&original_id));
    assert!(!inner.editor_versions.contains_key(&original_id));
    assert!(inner.sessions.contains_key(&session_id(&bob)));
    assert_eq!(inner.sessions.len(), 33);
    assert_eq!(
        inner
            .core
            .with_actor_for_root(&scope, &root, |core| {
                core.check_session_base(&original_id, original["baseId"].as_str().unwrap())
            })
            .unwrap_err()
            .code,
        "sessionMissing"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("Shared/Shared.typ")).unwrap(),
        original["source"].as_str().unwrap()
    );
}

#[test]
fn renewal_keeps_active_sessions_but_never_extends_another_actors_lease() {
    let (_temp, state, auth, original, _, _) = history_api::test_fixture();
    let mut inner = state.inner.lock().unwrap();
    let id = session_id(&original);
    let now = Instant::now();
    let soon = now + Duration::from_secs(60);
    *inner.sessions[&id].expires.lock().unwrap() = soon;
    let bob = Auth {
        digest: digest(b"bob"),
    };
    assert!(inner.session(&bob, &id).is_err());
    assert_eq!(*inner.sessions[&id].expires.lock().unwrap(), soon);
    inner.session(&auth, &id).unwrap();
    assert!(*inner.sessions[&id].expires.lock().unwrap() > soon);
    let abandoned = inner.open("alice", "Shared").unwrap();
    let abandoned_id = session_id(&abandoned);
    *inner.sessions[&abandoned_id].expires.lock().unwrap() = now;
    inner
        .reap_idle_sessions(now + Duration::from_secs(61))
        .unwrap();
    assert!(inner.sessions.contains_key(&id));
    assert!(!inner.sessions.contains_key(&abandoned_id));
}

#[tokio::test]
async fn close_does_not_require_a_document_read_grant() {
    let (_temp, state, auth, original, _, _) = history_api::test_fixture();
    let id = session_id(&original);
    {
        let mut inner = state.inner.lock().unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .read = false;
        assert!(inner.session(&auth, &id).is_err());
    }
    let Json(closed) = close(
        State(state.clone()),
        Extension(auth),
        crate::RequestJson(SessionRequest {
            session_id: id.clone(),
        }),
    )
    .await
    .unwrap();
    assert_eq!(closed["closed"], true);
    assert!(!state.inner.lock().unwrap().sessions.contains_key(&id));
}
