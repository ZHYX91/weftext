use serde_json::json;
use weftext_data::{DataError, DataErrorKind};
use weftext_knowledge::{KnowledgeError, Loss, decode_annotations, recover_discussion};
use weftext_messages::{DiagnosticOrigin, ErrorCauses, ErrorDescription, UiMessage};

#[test]
fn knowledge_keeps_full_original_chain_parallel_diagnostics_and_machine_kind() {
    let mut original = ErrorDescription::new(UiMessage::common_save())
        .diagnostic(DiagnosticOrigin::Json, "exact {diagnostic} 😀\r\n");
    for index in 0..ErrorCauses::MAX {
        original = original
            .try_caused_by(
                format!("source{index}"),
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: index.to_string(),
                    },
                )),
            )
            .unwrap();
    }
    let parallel: Vec<_> = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: format!("parallel-{index}"),
                },
            ))
            .diagnostic(
                DiagnosticOrigin::ExternalTool,
                format!("unmodified-{index}"),
            )
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
        let error = KnowledgeError::from_data(
            "actualOperation",
            DataError::with_kind(kind, original.clone()).with_diagnostics(parallel.clone()),
        );
        assert_eq!(error.code, "actualOperation");
        let (description, diagnostics) = error.into_parts();
        assert_eq!(description.causes.as_slice()[0].code, code);
        assert_eq!(diagnostics[0], original);
        assert_eq!(diagnostics[1..], parallel);
    }
    let loss = Loss {
        path: "exact/{author}/😀".into(),
        code: "actualLoss".into(),
        description: original.clone(),
        product_diagnostics: parallel.clone(),
    };
    let refusal = KnowledgeError::from_losses("copyLosses", vec![loss]);
    assert_eq!(refusal.code, "copyLosses");
    assert_eq!(refusal.product_diagnostics.len(), 42);
    assert_eq!(
        refusal.product_diagnostics[0].causes.as_slice()[0].code,
        "actualLoss"
    );
    assert_eq!(refusal.product_diagnostics[1], original);
    assert_eq!(refusal.product_diagnostics[2..], parallel);
    let wire = serde_json::to_value(refusal).unwrap();
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<KnowledgeError>(wire.clone()).unwrap())
            .unwrap(),
        wire
    );
    let mut unknown = wire.clone();
    unknown["message"] = json!("legacy explanation");
    assert!(serde_json::from_value::<KnowledgeError>(unknown).is_err());
    let mut incomplete = wire;
    incomplete
        .as_object_mut()
        .unwrap()
        .remove("productDiagnostics");
    assert!(serde_json::from_value::<KnowledgeError>(incomplete).is_err());
}

#[test]
fn annotation_duplicate_validation_is_typed_while_json_syntax_remains_original() {
    let bytes = br#"{"version":1,"annotations":[],"version":1}"#;
    let original = bytes.to_vec();
    let error = decode_annotations(bytes).unwrap_err();
    assert_eq!(bytes.as_slice(), original);
    assert_eq!(error.code, "damagedAnnotations");
    assert!(error.description.diagnostics.is_empty());
    assert!(
        error
            .description
            .causes
            .as_slice()
            .iter()
            .all(|cause| cause.diagnostics.is_empty())
    );
    let wire = serde_json::to_value(error).unwrap();
    assert!(
        wire["causes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cause| cause["explanation"]["key"] == "error.data.schema.jsonKeyDuplicate")
    );
    assert!(recover_discussion(bytes).is_empty());

    let malformed = br#"{"version":1,"annotations":[}"#;
    let expected = serde_json::from_slice::<serde_json::Value>(malformed)
        .unwrap_err()
        .to_string();
    let error = decode_annotations(malformed).unwrap_err();
    let raw: Vec<_> = error
        .description
        .causes
        .as_slice()
        .iter()
        .flat_map(|cause| &cause.diagnostics)
        .collect();
    assert_eq!(raw.len(), 1);
    assert_eq!(raw[0].origin, DiagnosticOrigin::Json);
    assert_eq!(raw[0].text, expected);
    assert!(
        serde_json::from_value::<KnowledgeError>(json!({"code":"legacy","message":"text"}))
            .is_err()
    );
}

#[test]
fn engine_parallel_product_reports_do_not_collapse_in_knowledge_adapter() {
    let description = ErrorDescription::new(UiMessage::common_save()).diagnostic(
        DiagnosticOrigin::Typst,
        "exact engine diagnostic {value} 😀\r\n",
    );
    let diagnostics: Vec<_> = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: index.to_string(),
                },
            ))
        })
        .collect();
    let error = KnowledgeError::from_engine(
        "encoding",
        weftext_engine::EngineError::new(description.clone())
            .with_product_diagnostics(diagnostics.clone()),
    );
    assert_eq!(error.code, "encoding");
    assert_eq!(error.into_parts(), (description, diagnostics));
}
