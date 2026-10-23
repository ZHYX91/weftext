use serde_json::json;
use weftext_data::{DataError, DataErrorKind};
use weftext_export::{ExportError, ExportLoss, exchange::NativeLoss};
use weftext_messages::{DiagnosticOrigin, ErrorCauses, ErrorDescription, UiMessage};

fn original_description() -> ErrorDescription {
    let mut description = ErrorDescription::new(UiMessage::common_save())
        .diagnostic(DiagnosticOrigin::Json, "original {wire} diagnostic 😀\r\n");
    for index in 0..ErrorCauses::MAX {
        description = description
            .try_caused_by(
                format!("originalCode{index}"),
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: format!("cause-{index}"),
                    },
                ))
                .diagnostic(DiagnosticOrigin::ExternalTool, format!("raw-{index}")),
            )
            .unwrap();
    }
    description
}

#[test]
fn full_cause_chain_and_parallel_data_diagnostics_survive_every_export_boundary() {
    let original = original_description();
    let parallel: Vec<_> = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: format!("parallel-{index}"),
                },
            ))
            .diagnostic(DiagnosticOrigin::Dependency, format!("exact-{index}\r\n"))
        })
        .collect();
    for (kind, code) in [
        (DataErrorKind::Invalid, "dataInvalid"),
        (DataErrorKind::Unavailable, "dataUnavailable"),
        (DataErrorKind::Coverage, "dataCoverage"),
        (DataErrorKind::Budget, "dataBudget"),
        (DataErrorKind::NativeBudget, "dataNativeBudget"),
        (
            DataErrorKind::ClosedExpressionUnavailable,
            "dataClosedExpressionUnavailable",
        ),
    ] {
        let error = ExportError::from_data(
            DataError::with_kind(kind, original.clone()).with_diagnostics(parallel.clone()),
        );
        let (description, diagnostics) = error.clone().into_parts();
        assert_eq!(description.causes.as_slice()[0].code, code);
        assert_eq!(diagnostics.len(), 41);
        assert_eq!(diagnostics[0], original);
        assert_eq!(diagnostics[1..], parallel);
        let loss = NativeLoss::from_error("exact/path.typ".into(), "conversion".into(), error);
        let wire = serde_json::to_value(&loss).unwrap();
        let restored: NativeLoss = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), wire);
        assert_eq!(
            wire["productDiagnostics"][0],
            serde_json::to_value(&original).unwrap()
        );
    }
}

#[test]
fn export_loss_and_error_wire_require_typed_explanation_and_complete_diagnostics() {
    let error = ExportError::new(UiMessage::error_export_json());
    let wire = serde_json::to_value(&error).unwrap();
    assert_eq!(wire["productDiagnostics"], json!([]));
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<ExportError>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    for invalid in [
        json!({"message":"legacy string"}),
        json!({"explanation":wire["explanation"],"diagnostics":[],"causes":[]}),
        json!({"explanation":wire["explanation"],"diagnostics":[],"causes":[],"productDiagnostics":[],"message":"parallel legacy"}),
    ] {
        assert!(serde_json::from_value::<ExportError>(invalid).is_err());
    }
    let loss = ExportLoss {
        code: "exact_code".into(),
        description: error.description,
        product_diagnostics: vec![],
        location: Some("author/{name}/😀".into()),
    };
    let wire = serde_json::to_value(loss).unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<ExportLoss>(wire.clone()).unwrap()).unwrap(),
        wire
    );
    let mut unknown = wire.clone();
    unknown["detail"] = json!("legacy explanation");
    assert!(serde_json::from_value::<ExportLoss>(unknown).is_err());
    let mut incomplete = wire;
    incomplete
        .as_object_mut()
        .unwrap()
        .remove("productDiagnostics");
    assert!(serde_json::from_value::<ExportLoss>(incomplete).is_err());
}

#[test]
fn engine_parallel_reports_and_original_os_diagnostics_survive_export_adapters() {
    let original = original_description();
    let parallel: Vec<_> = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: index.to_string(),
                },
            ))
        })
        .collect();
    let error = ExportError::from_engine(
        weftext_engine::EngineError::new(original.clone())
            .with_product_diagnostics(parallel.clone()),
    );
    assert_eq!(error.into_parts(), (original, parallel));
    let error = std::io::Error::from_raw_os_error(2);
    let raw = error.to_string();
    let error = ExportError::from(error);
    assert_eq!(error.description.diagnostics.len(), 1);
    assert_eq!(
        error.description.diagnostics[0].origin,
        DiagnosticOrigin::Os
    );
    assert_eq!(error.description.diagnostics[0].text, raw);
}
