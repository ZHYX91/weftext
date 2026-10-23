use std::collections::BTreeMap;
use weftext_data::*;

const ID: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";

fn collision_schema(applicable: bool, required: bool) -> Schema {
    let mut schema = Schema::bundled().unwrap();
    schema.fields.insert(
        0,
        Field {
            id: "private/config".into(),
            key: "tasks/status".into(),
            shape: Shape::Scalar {
                semantic: "text".into(),
            },
            nullable: false,
            default: Some(serde_json::json!({"kind":"absent"})),
        },
    );
    if applicable {
        schema
            .types
            .iter_mut()
            .find(|t| t.id == "tasks/task")
            .unwrap()
            .fields
            .push(FieldContribution {
                id: "private/config".into(),
                required,
            });
    }
    Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap()
}

fn fields() -> BTreeMap<String, NativeValue> {
    BTreeMap::from([("tasks/status".into(), NativeValue::Text("open".into()))])
}

#[test]
fn capture_canonical_id_never_borrows_another_fields_native_key() {
    for applicable in [false, true] {
        let schema = collision_schema(applicable, false);
        let source =
            capture_source(ID, &["task".into()], &fields(), "作者原文😀", &schema).unwrap();
        let projection = project(&source, &schema);
        assert_eq!(
            projection
                .fields
                .iter()
                .find(|f| f.field_id == "tasks/status")
                .unwrap()
                .cell
                .native(),
            Some(&NativeValue::Text("open".into()))
        );
        assert!(!source.contains("\"tasks/status\":"), "{source}");
        if applicable {
            assert!(matches!(
                projection
                    .fields
                    .iter()
                    .find(|f| f.field_id == "private/config")
                    .unwrap()
                    .cell,
                Cell::Absent
            ));
        }
    }
    let schema = collision_schema(true, false);
    let mut supplied = fields();
    supplied.insert(
        "private/config".into(),
        NativeValue::Text("independent private value".into()),
    );
    let source = capture_source(ID, &["task".into()], &supplied, "正文", &schema).unwrap();
    let projection = project(&source, &schema);
    for (id, value) in &supplied {
        assert_eq!(
            projection
                .fields
                .iter()
                .find(|f| &f.field_id == id)
                .unwrap()
                .cell
                .native(),
            Some(value)
        );
    }
}

#[test]
fn capture_ids_reject_native_keys_and_preserve_independent_required_and_default_values() {
    let schema = collision_schema(true, true);
    let error = capture_source(ID, &["task".into()], &fields(), "", &schema).unwrap_err();
    assert_eq!(error.kind(), DataErrorKind::Invalid);
    assert_eq!(
        error.description().explanation,
        UiMessage::error_data_projection_required_capture_field_missing(
            weftext_messages::params::ErrorDataProjectionRequiredCaptureFieldMissing {
                field_key: "tasks/status".into()
            }
        )
    );
    let mut schema = schema;
    schema
        .fields
        .iter_mut()
        .find(|f| f.id == "private/config")
        .unwrap()
        .default = Some(serde_json::json!({"kind":"native","source":"\"private default\""}));
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    let source = capture_source(ID, &["task".into()], &fields(), "", &schema).unwrap();
    assert_eq!(
        project(&source, &schema)
            .fields
            .iter()
            .find(|f| f.field_id == "private/config")
            .unwrap()
            .cell
            .native(),
        Some(&NativeValue::Text("private default".into()))
    );
    for key in ["status", "unknown/field"] {
        let supplied = BTreeMap::from([(key.into(), NativeValue::Text("open".into()))]);
        let error = capture_source(ID, &["task".into()], &supplied, "", &schema).unwrap_err();
        assert_eq!(error.kind(), DataErrorKind::Invalid);
        assert_eq!(
            error.description().explanation,
            UiMessage::error_data_projection_capture_field_unbound(
                weftext_messages::params::ErrorDataProjectionCaptureFieldUnbound {
                    field_key: key.into()
                }
            )
        );
    }
}
