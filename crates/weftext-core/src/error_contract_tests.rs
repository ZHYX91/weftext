use super::*;

fn original_failure() -> CoreError {
    let parallel = (0..40)
        .map(|index| {
            ErrorDescription::new(UiMessage::common_operation(
                weftext_messages::params::CommonOperation {
                    operation_id: format!("issue-{index}"),
                },
            ))
            .diagnostic(
                DiagnosticOrigin::Typst,
                format!("author diagnostic {index}\n{{literal}}"),
            )
        })
        .collect::<Vec<_>>();
    let mut description = ErrorDescription::new(UiMessage::error_core_io_failure());
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
    let mut error = CoreError::from_data(
        "originalCode",
        weftext_data::DataError::native_budget(description).with_diagnostics(parallel),
    );
    error.current_source = Some("protected original source 😀\n{literal}".into());
    error
}

#[test]
fn causal_overflow_retains_source_classification_and_all_parallel_problems() {
    let original = original_failure();
    let expected_details = original.details.clone().unwrap();
    let expected_description = ErrorDescription {
        explanation: original.explanation.clone(),
        diagnostics: original.diagnostics.clone(),
        causes: original.causes.clone(),
    };
    let error = CoreError::new("recoveryPending", UiMessage::common_save()).caused_by(original);
    assert_eq!(error.code, "recoveryPending");
    assert_eq!(
        error.current_source.as_deref(),
        Some("protected original source 😀\n{literal}")
    );
    assert_eq!(error.causes.as_slice().len(), 1);
    assert_eq!(error.causes.as_slice()[0].code, "originalCode");
    let Some(CoreErrorDetails::Aggregate { groups }) = &error.details else {
        panic!("complete facts missing")
    };
    let CoreErrorDetails::Aggregate { groups: expected } = expected_details;
    assert_eq!(&groups[..expected.len()], expected.as_slice());
    let ErrorDetailGroup::Product {
        code: Some(code),
        diagnostics,
    } = groups.last().unwrap()
    else {
        panic!("full original cause missing")
    };
    assert_eq!(code, "originalCode");
    assert_eq!(diagnostics, &[expected_description]);
    let wire = serde_json::to_string(&error).unwrap();
    assert_eq!(wire.matches("currentSource").count(), 1);
    assert!(
        !serde_json::to_value(error)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("message")
    );
}

#[test]
fn typed_io_payloads_retain_actual_core_facts_and_os_codes_do_not_depend_on_display_text() {
    let original = original_failure();
    let expected = serde_json::to_value(&original).unwrap();
    let recovered = CoreError::from(std::io::Error::other(original));
    assert_eq!(serde_json::to_value(recovered).unwrap(), expected);
    let typed = ErrorDescription::new(UiMessage::common_save());
    let recovered = CoreError::from(std::io::Error::other(typed.clone()));
    assert_eq!(recovered.explanation, typed.explanation);
    assert!(recovered.diagnostics.is_empty());
    let original_os = std::io::Error::from_raw_os_error(5);
    let expected_text = original_os.to_string();
    let recovered = CoreError::from(original_os);
    assert_eq!(recovered.os_error(), Some(5));
    assert_eq!(
        recovered.diagnostics,
        vec![RawDiagnostic {
            origin: DiagnosticOrigin::Os,
            text: expected_text
        }]
    );
}

#[test]
fn computed_description_preserves_metadata_without_republishing_protected_source() {
    let description = CoreDescription::from(original_failure());
    let value = serde_json::to_value(&description).unwrap();
    assert!(!value.to_string().contains("protected original source"));
    assert_eq!(description.causes.as_slice().len(), ErrorCauses::MAX);
    let Some(CoreErrorDetails::Aggregate { groups }) = &description.details else {
        panic!("metadata lost")
    };
    assert!(
        matches!(&groups[0], ErrorDetailGroup::Data {classification: weftext_data::DataErrorKind::NativeBudget, diagnostics} if diagnostics.len() == 40)
    );
    assert_eq!(
        serde_json::from_value::<CoreDescription>(value).unwrap(),
        description
    );
}
