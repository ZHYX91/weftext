use std::collections::BTreeMap;
use weftext_data::*;
use weftext_messages::UiLocale;
mod support;
use support::assert_data_error;

fn record(entries: Vec<(&str, NativeValue)>) -> NativeValue {
    NativeValue::Record(
        entries
            .into_iter()
            .map(|(key, value)| NativeMember {
                key: key.into(),
                value,
            })
            .collect(),
    )
}

#[test]
fn duplicate_json_is_a_typed_product_fact_and_real_parser_diagnostics_remain_original() {
    let duplicate = "{\"version\":1,\"version\":2}";
    let error = Schema::from_author_json(duplicate).unwrap_err();
    assert_eq!(error.kind(), DataErrorKind::Invalid);
    assert_eq!(
        serde_json::to_value(error.description()).unwrap()["explanation"],
        serde_json::json!({
            "key":"error.data.schema.jsonKeyDuplicate", "params":{"key":"version"}
        })
    );
    assert!(error.description().diagnostics.is_empty());
    let broken = "{ broken";
    let original = serde_json::from_str::<serde_json::Value>(broken)
        .unwrap_err()
        .to_string();
    let error = Schema::from_author_json(broken).unwrap_err();
    assert_eq!(
        error.description().explanation.key(),
        "error.data.json.invalid"
    );
    assert_eq!(error.description().diagnostics.len(), 1);
    assert_eq!(
        error.description().diagnostics[0].origin,
        DiagnosticOrigin::Json
    );
    assert_eq!(error.description().diagnostics[0].text, original);
    assert_eq!(duplicate, "{\"version\":1,\"version\":2}");
    assert_eq!(broken, "{ broken");
}

#[test]
fn native_decoder_budget_is_a_fact_and_preserves_the_exact_author_input() {
    let source = format!("{}1{}", "(".repeat(80), ")".repeat(80));
    let original = source.clone();
    let error = decode_native_expression(&source).unwrap_err();
    assert_eq!(error.kind(), DataErrorKind::NativeBudget);
    let cell = Cell::unavailable_error(error);
    assert!(cell.is_native_budget());
    assert_eq!(source, original);
    let wire = serde_json::to_value(&cell).unwrap();
    assert_eq!(wire["kind"], "nativeBudget");
    assert_eq!(
        wire["reason"]["explanation"]["key"],
        "error.data.native.decoderTraversalBudget"
    );
    assert_eq!(serde_json::from_value::<Cell>(wire.clone()).unwrap(), cell);
    let mut missing_kind = wire.clone();
    missing_kind.as_object_mut().unwrap().remove("kind");
    assert!(serde_json::from_value::<Cell>(missing_kind).is_err());
    let mut legacy_reason = wire;
    legacy_reason["reason"] = serde_json::json!("native-decoder traversal budget exceeded");
    assert!(serde_json::from_value::<Cell>(legacy_reason).is_err());
}

#[test]
fn actual_capture_validation_preserves_more_than_one_cause_chain_of_parallel_diagnostics() {
    let schema = Schema::bundled().unwrap();
    let fields = BTreeMap::from([(
        "aliases".into(),
        NativeValue::Array(vec![NativeValue::Int("7".into()); 96]),
    )]);
    let before = serde_json::to_vec(&fields).unwrap();
    let error =
        capture_template(&["task".into()], &fields, "author input 原文😀", &schema).unwrap_err();
    let (kind, description, details) = error.into_parts();
    assert_eq!(kind, DataErrorKind::Invalid);
    assert_eq!(
        description.explanation,
        UiMessage::error_data_projection_capture_field_validation_failed(
            weftext_messages::params::ErrorDataProjectionCaptureFieldValidationFailed {
                field: "aliases".into()
            }
        )
    );
    let Some(DataErrorDetails::Diagnostics { diagnostics }) = details else {
        panic!("capture validation details must remain structured")
    };
    assert_eq!(diagnostics.len(), 96);
    for (index, diagnostic) in diagnostics.iter().enumerate() {
        assert_eq!(
            diagnostic.explanation,
            UiMessage::error_data_schema_value_semantic_mismatch(
                weftext_messages::params::ErrorDataSchemaValueSemanticMismatch {
                    value_path: format!("$[{index}]"),
                    semantic: "nonempty_text".into(),
                }
            )
        );
        assert!(diagnostic.diagnostics.is_empty());
        assert!(diagnostic.causes.as_slice().is_empty());
    }
    let wire = serde_json::to_value(&diagnostics).unwrap();
    assert_eq!(
        serde_json::from_value::<Vec<ErrorDescription>>(wire).unwrap(),
        diagnostics
    );
    assert_eq!(serde_json::to_vec(&fields).unwrap(), before);
}

#[test]
fn invalid_field_keeps_parallel_typed_diagnostics_and_the_original_author_value() {
    let schema = Schema::bundled().unwrap();
    let value = NativeValue::Array(vec![NativeValue::Int("7".into()); 96]);
    let raw = value.to_typst().unwrap();
    let source = format!(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"task\",), status: \"open\", aliases: {raw})) <weftext>\n作者正文 原文😀\r\n"
    );
    let projection = project(&source, &schema);
    assert_eq!(projection.qualification, "qualified");
    let field = projection
        .fields
        .iter()
        .find(|field| field.field_id == "core/aliases")
        .unwrap();
    assert_eq!(field.raw.as_deref(), Some(raw.as_str()));
    let Cell::Invalid {
        diagnostics,
        value: Some(preserved),
    } = &field.cell
    else {
        panic!("invalid author values must remain available beside typed diagnostics")
    };
    assert_eq!(preserved, &value);
    assert_eq!(diagnostics.len(), 96);
    let before = serde_json::to_vec(&field.cell).unwrap();
    for diagnostic in diagnostics {
        let _ = diagnostic.explanation.render(UiLocale::ZhCn);
        let _ = diagnostic.explanation.render(UiLocale::EnUs);
    }
    assert_eq!(serde_json::to_vec(&field.cell).unwrap(), before);
    assert_eq!(serde_json::from_slice::<Cell>(&before).unwrap(), field.cell);
    assert!(source.ends_with("作者正文 原文😀\r\n"));
}

#[test]
fn original_cel_parser_diagnostic_is_separate_from_the_typed_product_explanation() {
    let expression = "1 +";
    let original = match cel::Program::compile(expression) {
        Ok(_) => panic!("fixture must be invalid CEL"),
        Err(error) => error.to_string(),
    };
    let plan = QueryPlan {
        select: vec![QueryColumn {
            name: "value".into(),
            expr: expression.into(),
        }],
        ..Default::default()
    };
    let error = validate_query_plan(&plan, &Schema::bundled().unwrap()).unwrap_err();
    assert_data_error(
        &error,
        DataErrorKind::Invalid,
        UiMessage::error_data_query_cel_syntax_invalid(),
    );
    assert_eq!(
        error.description().diagnostics,
        vec![RawDiagnostic {
            origin: DiagnosticOrigin::Dependency,
            text: original,
        }]
    );
    let before = serde_json::to_vec(error.description()).unwrap();
    let _ = error.description().explanation.render(UiLocale::ZhCn);
    let _ = error.description().explanation.render(UiLocale::EnUs);
    assert_eq!(serde_json::to_vec(error.description()).unwrap(), before);
}

#[test]
fn consumed_field_budget_follows_the_fact_even_when_its_message_says_otherwise() {
    let schema = Schema::bundled().unwrap();
    let plan = QueryPlan {
        r#where: Some("true || isUnavailable(priority)".into()),
        select: vec![QueryColumn {
            name: "value".into(),
            expr: "true".into(),
        }],
        ..Default::default()
    };
    for (kind, explanation, expected_coverage) in [
        (
            DataErrorKind::Unavailable,
            UiMessage::error_data_native_decoder_traversal_budget(),
            "complete",
        ),
        (
            DataErrorKind::NativeBudget,
            UiMessage::error_data_projection_validation_unavailable(),
            "partial",
        ),
    ] {
        let input = FrozenQueryInput {
            cut_id: "budget-fact-cut".into(),
            schema_revision: "budget-fact-schema".into(),
            coverage: "complete".into(),
            freshness: "current".into(),
            advanced: None,
            notes: vec![FrozenNote {
                entity_key: "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/32eb9a12-e752-4bfa-8cdf-65e00f23ac26".into(),
                path: "One/One.typ".into(),
                source: "private author body 原文😀".into(),
                revision: "budget-fact-revision".into(),
                lifecycle: "live".into(),
                title: None,
                body: None,
                products: None,
                authorized_fields: Some(BTreeMap::from([(
                    "tasks/priority".into(),
                    Cell::unavailable_group(kind, explanation, Vec::new()),
                )])),
            }],
        };
        let original = serde_json::to_vec(&input).unwrap();
        let result = execute_query(&input, &plan, &schema).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.coverage, expected_coverage);
        assert_eq!(result.aggregate_exact, kind != DataErrorKind::NativeBudget);
        assert_eq!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.explanation
                    == UiMessage::error_data_query_advanced_consumed_field_decoder_budget_exhausted()),
            kind == DataErrorKind::NativeBudget,
        );
        assert_eq!(serde_json::to_vec(&input).unwrap(), original);
    }
}

#[test]
fn native_recurrence_diagnostics_remain_canonical_records_across_both_display_locales() {
    let schema = Schema::bundled().unwrap();
    let event = record(vec![(
        "title",
        NativeValue::Text("author title 原文😀".into()),
    )]);
    let window = record(vec![
        ("start", NativeValue::Date("2026-10-22".into())),
        ("end_exclusive", NativeValue::Date("2026-10-23".into())),
    ]);
    let event_before = serde_json::to_vec(&event).unwrap();
    let result = expand_event(&event, &window, &schema, ExpansionBudget::default());
    assert_eq!(result.status, ExpansionStatus::Invalid);
    let before = serde_json::to_vec(&result.native()).unwrap();
    assert!(!result.diagnostics.is_empty());
    for diagnostic in &result.diagnostics {
        assert_ne!(
            diagnostic.explanation.render(UiLocale::ZhCn),
            diagnostic.explanation.render(UiLocale::EnUs)
        );
    }
    assert_eq!(serde_json::to_vec(&result.native()).unwrap(), before);
    assert_eq!(serde_json::to_vec(&event).unwrap(), event_before);
    let native = result.native();
    let diagnostic = &native.member("diagnostics").unwrap().array().unwrap()[0];
    assert!(
        diagnostic
            .member("explanation")
            .unwrap()
            .member("key")
            .unwrap()
            .text()
            .unwrap()
            .starts_with("error.data.recurrence.")
    );
    assert!(
        diagnostic
            .member("diagnostics")
            .unwrap()
            .array()
            .unwrap()
            .is_empty()
    );
    assert!(
        diagnostic
            .member("causes")
            .unwrap()
            .array()
            .unwrap()
            .is_empty()
    );
}
