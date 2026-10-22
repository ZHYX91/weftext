use super::*;
use weftext_messages::ErrorCauses;

#[test]
fn api_data_bridge_keeps_status_kind_and_independent_problems() {
    let parallel = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: format!("original-{index}"),
                },
            ))
            .diagnostic(DiagnosticOrigin::Dependency, format!("original {index}"))
        })
        .collect::<Vec<_>>();
    let error = ApiError::from_data(
        StatusCode::BAD_REQUEST,
        "invalid_data",
        weftext_data::DataError::native_budget(UiMessage::common_save())
            .with_diagnostics(parallel.clone()),
    );
    assert_eq!(error.status, StatusCode::BAD_REQUEST);
    let wire = serde_json::to_value(&error).unwrap();
    assert_eq!(wire["code"], "invalid_data");
    assert_eq!(wire["details"]["kind"], "core");
    assert_eq!(
        wire["details"]["details"]["groups"][0]["classification"],
        "nativeBudget"
    );
    assert_eq!(
        wire["details"]["details"]["groups"][0]["diagnostics"],
        serde_json::to_value(parallel).unwrap()
    );
    for required in ["code", "explanation", "diagnostics", "causes"] {
        assert!(wire.get(required).is_some());
    }
    assert!(wire.get("message").is_none());
    assert!(wire.get("status").is_none());
}

#[test]
fn committed_receipt_and_full_failure_remain_independent_facts() {
    let mut description = ErrorDescription::new(UiMessage::common_save());
    for index in 0..ErrorCauses::MAX {
        description = description
            .try_caused_by(
                format!("cause{index}"),
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: index.to_string(),
                    },
                )),
            )
            .unwrap();
    }
    let mut original = CoreError::new("sourceChanged", description);
    original.current_source = Some("protected current source 😀".into());
    let expected = serde_json::to_value(ApiError::from(original.clone())).unwrap();
    let receipt = weftext_core::OperationReceipt {
        operation_id: "original-operation".into(),
        plan_id: "original-plan".into(),
        status: "committed".into(),
        path: "original-path".into(),
        node_ids: vec!["original-node".into()],
    };
    let error = ApiError::new(
        StatusCode::CONFLICT,
        "deliveryPending",
        UiMessage::common_save(),
    )
    .committed_receipt(receipt, ApiError::from(original));
    let wire = serde_json::to_value(&error).unwrap();
    assert_eq!(error.status, StatusCode::CONFLICT);
    assert_eq!(wire["details"]["kind"], "committedReceipt");
    assert_eq!(wire["details"]["receipt"]["status"], "committed");
    assert_eq!(
        wire["details"]["receipt"]["operationId"],
        "original-operation"
    );
    assert!(wire["details"]["receipt"].get("message").is_none());
    assert_eq!(wire["details"]["cause"], expected);
    assert_eq!(wire.to_string().matches("currentSource").count(), 1);
    assert!(error.causes.as_slice().is_empty());
}

#[cfg(windows)]
#[tokio::test]
async fn original_creation_receipt_survives_typed_document_delivery_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("original-library");
    let retained = temp.path().join("retained-original-library");
    let mut core = Core::new(temp.path().join("private")).unwrap();
    let receipt = core.create_library(&root).unwrap();
    let expected = serde_json::to_value(&receipt).unwrap();
    let original_workspace = std::fs::read(root.join(".weftext-workspace.json")).unwrap();
    std::fs::rename(&root, &retained).unwrap();
    std::fs::create_dir(&root).unwrap();
    let state = AppState {
        core: Arc::new(Mutex::new(core)),
        token: Arc::from("test-token"),
        port: 0,
        locations: Default::default(),
        renders: Default::default(),
        editor_versions: Default::default(),
        exports: Default::default(),
        native_status: Default::default(),
        render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
        sync_remote: Default::default(),
        initial: None,
    };
    let delivered = library_creation_output(&state, receipt).await.unwrap();
    for (key, fact) in expected.as_object().unwrap() {
        assert_eq!(delivered.get(key), Some(fact));
    }
    let failure = &delivered["deliveryError"];
    assert!(failure.get("message").is_none());
    for field in ["code", "explanation", "diagnostics", "causes", "details"] {
        assert!(failure.get(field).is_some(), "missing {field}");
    }
    assert!(delivered.get("rootDocument").is_none());
    assert_eq!(
        std::fs::read(retained.join(".weftext-workspace.json")).unwrap(),
        original_workspace
    );
    assert!(std::fs::read_dir(&root).unwrap().next().is_none());
}
