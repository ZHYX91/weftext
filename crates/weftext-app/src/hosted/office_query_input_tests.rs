fn field_output_fixture() -> (tempfile::TempDir, HostedState, Auth, String) {
    let (temp, state, auth, _, _, _) = history_api::test_fixture();
    let source = {
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let doc = inner
            .core
            .open(root.join("Shared").join("Shared.typ"), Some(root.clone()))
            .unwrap();
        let source = format!(
            "#metadata((id: \"{}\", types: (\"task\",), status: \"open\")) <weftext>\nPRIVATE BODY NOT EXPORTED\n",
            doc.node_id.unwrap()
        );
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: source.clone(),
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        inner.core.close_session(&doc.session_id).unwrap();
        for actor in ["alice", "bob"] {
            inner.policy.users.get_mut(actor).unwrap().grants.insert(
                "Shared".into(),
                Rights {
                    office: OfficeRights {
                        export: true,
                        download: true,
                        r#import: false,
                        ..Default::default()
                    },
                    fields: BTreeMap::from([(
                        "tasks/status".into(),
                        FieldRights {
                            read: true,
                            write: false,
                        },
                    )]),
                    ..Default::default()
                },
            );
        }
        source
    };
    (temp, state, auth, source)
}

fn output_is_unclaimed(state: &HostedState, actor: &str, request_id: &str) {
    let mut inner = state.inner.lock().unwrap();
    assert!(
        !inner
            .office
            .jobs
            .contains_key(&(actor.into(), request_id.into()))
    );
    let root = inner.config.root.clone();
    let output_scope = scope(&inner, actor);
    assert!(
        inner
            .core
            .with_actor_for_root(&output_scope, &root, |core| {
                core.actor_output_fingerprint(request_id)
            })
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn frozen_query_output_rejects_unbound_source_and_draft_inputs_without_claiming() {
    let (_temp, state, auth, _) = field_output_fixture();
    let query = query::execute(
        &state,
        &auth,
        serde_json::from_value(json!({
            "fields":["tasks/status"],
            "plan":{"select":[{"name":"状态","expr":"status"},{"name":"序号","expr":"1"}]}
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(query["columns"], json!(["状态", "序号"]));
    let mut original = request("xlsx");
    original.query_id = query["queryId"].as_str().map(str::to_owned);
    for extra in [
        json!({"source":"hidden source"}),
        json!({"source":""}),
        json!({"sessionId":"unbound-session"}),
        json!({"baseId":"unbound-base"}),
        json!({"serial":0}),
        json!({"sourceHash":weftext_engine::source_hash("")}),
        json!({"source":"hidden source","sessionId":"unbound-session","baseId":"unbound-base","serial":1,"sourceHash":weftext_engine::source_hash("hidden source")}),
    ] {
        let mut value = serde_json::to_value(&original).unwrap();
        value["requestId"] = json!(uuid::Uuid::new_v4());
        value
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        let invalid: Prepare = serde_json::from_value(value).unwrap();
        let request_id = invalid.request_id.clone();
        let error = prepare(State(state.clone()), Extension(auth.clone()), Json(invalid))
            .await
            .unwrap_err();
        assert_eq!(error.status, StatusCode::BAD_REQUEST);
        assert_eq!(error.code, "invalid_request");
        output_is_unclaimed(&state, "alice", &request_id);
        let Json(status) = status(
            State(state.clone()),
            Extension(auth.clone()),
            Json(RequestId { request_id }),
        )
        .await
        .unwrap();
        assert_eq!(status["status"], "notRegistered");
    }

    // A real retained field query remains usable without granting its actor
    // source read; another actor cannot borrow the same query ID.
    let mut other = original.clone();
    other.request_id = uuid::Uuid::new_v4().to_string();
    let other_id = other.request_id.clone();
    let error = prepare(
        State(state.clone()),
        Extension(Auth {
            digest: digest(b"bob"),
        }),
        Json(other),
    )
    .await
    .unwrap_err();
    assert_eq!(error.status, StatusCode::GONE);
    assert_eq!(error.code, "queryExpired");
    output_is_unclaimed(&state, "bob", &other_id);

    let Json(prepared) = prepare(
        State(state.clone()),
        Extension(auth),
        Json(original.clone()),
    )
    .await
    .unwrap();
    assert_eq!(prepared["status"], "prepared");
    assert_eq!(prepared["preview"]["query"]["columns"], query["columns"]);
    assert_eq!(prepared["preview"]["query"]["rowCount"], 1);
    let inner = state.inner.lock().unwrap();
    assert!(inner.rights("alice", "Shared").is_err());
    let job = &inner.office.jobs[&("alice".into(), original.request_id)];
    assert!(job.needs.sources.is_empty());
    assert_eq!(
        job.needs.fields["Shared"],
        BTreeSet::from(["tasks/status".into()])
    );
}

#[tokio::test]
async fn explicit_field_source_still_requires_independent_source_and_field_read() {
    let (_temp, state, auth, source) = field_output_fixture();
    let mut req = request("xlsx");
    req.resource = Some("Shared".into());
    req.source = Some(source);
    req.bindings.insert(
        "状态".into(),
        Binding {
            kind: "field".into(),
            field: Some("tasks/status".into()),
            text: None,
        },
    );
    validate(&req).unwrap();
    let error = prepare(
        State(state.clone()),
        Extension(auth.clone()),
        Json(req.clone()),
    )
    .await
    .unwrap_err();
    assert_eq!(error.status, StatusCode::NOT_FOUND);
    output_is_unclaimed(&state, "alice", &req.request_id);
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
            .read = true;
    }
    let Json(prepared) = prepare(
        State(state.clone()),
        Extension(auth.clone()),
        Json(req.clone()),
    )
    .await
    .unwrap();
    assert_eq!(prepared["status"], "prepared");
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
            .fields
            .clear();
    }
    req.request_id = uuid::Uuid::new_v4().to_string();
    let rejected_id = req.request_id.clone();
    let error = prepare(State(state.clone()), Extension(auth), Json(req))
        .await
        .unwrap_err();
    assert_eq!(error.status, StatusCode::NOT_FOUND);
    output_is_unclaimed(&state, "alice", &rejected_id);
}

#[test]
fn frozen_query_output_source_shape_is_strict_and_null_retains_absence() {
    let mut original = request("xlsx");
    original.query_id = Some("retained-query".into());
    let base = serde_json::to_value(original).unwrap();
    for source in [
        json!(true),
        json!(0),
        json!([]),
        json!({"text":"hidden source"}),
    ] {
        let mut invalid = base.clone();
        invalid["source"] = source;
        assert!(serde_json::from_value::<Prepare>(invalid).is_err());
    }
    for key in ["sources", "Source", "sourceText"] {
        let mut invalid = base.clone();
        invalid[key] = json!("hidden source");
        assert!(serde_json::from_value::<Prepare>(invalid).is_err());
    }
    let parsed: Prepare = serde_json::from_value(base.clone()).unwrap();
    validate(&parsed).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), base);
}
