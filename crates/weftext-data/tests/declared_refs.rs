use std::collections::BTreeMap;
use weftext_data::*;

const ID: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";
const TARGET: &str =
    "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
const OPAQUE: &str =
    "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/b70db814-f42b-43f4-9d73-8711549f4e9e";

fn source(fields: &str) -> String {
    format!(
        "#metadata((id: \"{ID}\", types: (\"task\",), status: \"open\"{fields})) <weftext>\n原文😀"
    )
}

fn member(nullable: bool) -> Member {
    Member {
        shape: Shape::Scalar {
            semantic: "node_ref".into(),
        },
        required: true,
        nullable,
    }
}

fn nested_schema(unknown: &str) -> Schema {
    let mut schema = Schema::bundled().unwrap();
    schema.definitions.insert(
        "ReviewedRefItem".into(),
        Shape::Record {
            members: BTreeMap::from([("target".into(), member(true))]),
            unknown: unknown.into(),
            constraints: vec![],
        },
    );
    schema.definitions.insert(
        "ReviewedRefs".into(),
        Shape::Record {
            members: BTreeMap::from([
                ("primary".into(), member(false)),
                (
                    "nullable".into(),
                    Member {
                        required: false,
                        ..member(true)
                    },
                ),
                (
                    "items".into(),
                    Member {
                        shape: Shape::List {
                            items: Box::new(Shape::Use {
                                name: "ReviewedRefItem".into(),
                            }),
                            min: 0,
                            max: None,
                        },
                        required: false,
                        nullable: false,
                    },
                ),
            ]),
            unknown: unknown.into(),
            constraints: vec![],
        },
    );
    schema.fields.push(Field {
        id: "test/reviewed_refs".into(),
        key: "reviewed_refs".into(),
        shape: Shape::Use {
            name: "ReviewedRefs".into(),
        },
        nullable: false,
        default: Some(serde_json::json!({"kind":"absent"})),
    });
    schema
        .types
        .iter_mut()
        .find(|t| t.id == "tasks/task")
        .unwrap()
        .fields
        .push(FieldContribution {
            id: "test/reviewed_refs".into(),
            required: false,
        });
    Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap()
}

#[test]
fn declared_refs_preserve_unknown_root_without_claiming_its_possible_targets() {
    let schema = Schema::bundled().unwrap();
    let original = source(&format!(
        ", project: (/* exact */ \"{TARGET}\"), opaque: (target: \"{OPAQUE}\",), computed_opaque: private_function()"
    ));
    let result = declared_typed_ref_closure(&original, &schema).unwrap();
    assert!(result.complete && result.reasons.is_empty());
    assert_eq!(result.slots.len(), 1);
    assert_eq!(result.slots[0].field_id, "tasks/project");
    assert_eq!(result.slots[0].target, TARGET);
    assert_eq!(
        &original[result.slots[0].from..result.slots[0].to],
        format!("\"{TARGET}\"")
    );
    assert_eq!(result.unbound.len(), 2);
    assert!(result.unbound.iter().any(|p| p.contains("computed_opaque")));
    assert!(!result.slots.iter().any(|slot| slot.target == OPAQUE));
    assert!(validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    let strict = typed_ref_closure(&original, &schema).unwrap();
    assert!(!strict.complete);
    assert!(strict.complete_without_unbound_root_fields);
    assert!(typed_ref_slots(&original, &schema).is_err());
}

#[test]
fn declared_refs_never_approve_invalid_or_computed_known_values() {
    let schema = Schema::bundled().unwrap();
    for fields in [
        ", project: 42",
        ", project: private_function()",
        ", project: none",
    ] {
        let original = source(fields);
        let result = declared_typed_ref_closure(&original, &schema).unwrap();
        assert!(!result.complete, "{fields}");
        assert!(!result.reasons.is_empty());
        assert!(result.slots.is_empty());
        assert!(
            !typed_ref_closure(&original, &schema)
                .unwrap()
                .complete_without_unbound_root_fields
        );
    }
    let original = source(&format!(", project: \"{TARGET}\", status: \"bogus\""));
    assert!(declared_typed_ref_closure(&original, &schema).is_err()); // Duplicate direct control members remain unavailable.
}

#[test]
fn declared_refs_handle_nullable_none_without_changing_strict_copy_closure() {
    let mut schema = Schema::bundled().unwrap();
    schema
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/project")
        .unwrap()
        .nullable = true;
    let original = source(", project: (/* explicit nullable */ none)");
    let result = declared_typed_ref_closure(&original, &schema).unwrap();
    assert!(result.complete);
    assert!(result.slots.is_empty() && result.unbound.is_empty());
    assert!(validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    assert!(!typed_ref_closure(&original, &schema).unwrap().complete);
    schema
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/project")
        .unwrap()
        .nullable = false;
    assert!(
        !declared_typed_ref_closure(&original, &schema)
            .unwrap()
            .complete
    );
}

#[test]
fn declared_refs_nested_shapes_only_locate_proved_known_members() {
    let schema = nested_schema("preserve");
    let original = source(&format!(
        ", reviewed_refs: (primary: (\"{TARGET}\"), nullable: none, items: (((target: \"{TARGET}\", extra: \"{OPAQUE}\")), (target: none, extra: \"{OPAQUE}\")), extra: \"{OPAQUE}\")"
    ));
    let result = declared_typed_ref_closure(&original, &schema).unwrap();
    assert!(result.complete);
    assert_eq!(result.slots.len(), 2);
    assert_eq!(result.unbound.len(), 3);
    for slot in &result.slots {
        assert_eq!(slot.target, TARGET);
        assert_eq!(&original[slot.from..slot.to], format!("\"{TARGET}\""));
    }
    assert_eq!(
        result.slots[1].path,
        vec![
            ValuePath::Member {
                key: "items".into()
            },
            ValuePath::Index { index: 0 },
            ValuePath::Member {
                key: "target".into()
            },
        ]
    );
    assert!(!typed_ref_closure(&original, &schema).unwrap().complete);
    assert!(validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    let rejecting = nested_schema("reject");
    assert!(
        !declared_typed_ref_closure(&original, &rejecting)
            .unwrap()
            .complete
    );
    assert!(!validate_note(&original, &rejecting, &ValidationContext::default()).is_valid());
}

#[test]
fn declared_refs_nested_invalid_or_computed_fields_remain_incomplete() {
    let schema = nested_schema("preserve");
    for nested in [
        format!("(primary: \"{TARGET}\", items: ((target: 42,),))"),
        format!("(primary: \"{TARGET}\", items: ((target: private_function(),),))"),
        "(primary: none)".into(),
        format!("(primary: \"{TARGET}\", extra: private_function())"),
    ] {
        let original = source(&format!(", project: \"{TARGET}\", reviewed_refs: {nested}"));
        let result = declared_typed_ref_closure(&original, &schema).unwrap();
        assert!(!result.complete, "{nested}");
        // A partially decodable known record cannot yield an exact claim for
        // its child refs. The independent qualified field is still retained.
        assert_eq!(result.slots.len(), 1);
        assert_eq!(result.slots[0].field_id, "tasks/project");
        assert!(!validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    }
}

#[test]
fn declared_refs_do_not_replace_required_fields_or_owner_resource_constraints() {
    let schema = Schema::bundled().unwrap();
    let missing_required =
        source(&format!(", project: \"{TARGET}\"")).replace("status: \"open\"", "opaque: 1");
    assert!(
        declared_typed_ref_closure(&missing_required, &schema)
            .unwrap()
            .complete
    );
    assert!(!validate_note(&missing_required, &schema, &ValidationContext::default()).is_valid());

    let mut schema = schema;
    schema.fields.push(Field {
        id: "test/resource".into(),
        key: "reviewed_resource".into(),
        shape: Shape::Scalar {
            semantic: "resource_ref".into(),
        },
        nullable: false,
        default: Some(serde_json::json!({"kind":"absent"})),
    });
    let task = schema
        .types
        .iter_mut()
        .find(|t| t.id == "tasks/task")
        .unwrap();
    task.fields.push(FieldContribution {
        id: "test/resource".into(),
        required: false,
    });
    task.constraints.push(Constraint {
        validator: "core/owned-resources".into(),
        inputs: BTreeMap::from([
            (
                "owner".into(),
                Selector::Context {
                    name: "owner".into(),
                },
            ),
            (
                "values".into(),
                Selector::Field {
                    id: "test/resource".into(),
                    path: vec![],
                },
            ),
            (
                "metadata".into(),
                Selector::Context {
                    name: "resource_metadata".into(),
                },
            ),
        ]),
        parameters: serde_json::json!({"owner_required":true}),
    });
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    let owner = format!("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{ID}");
    let resource = "weftext-resource://58c47256-bfce-4d29-a037-b83a3a644fb0/b70db814-f42b-43f4-9d73-8711549f4e9e";
    let original = source(&format!(", reviewed_resource: \"{resource}\""));
    let refs = declared_typed_ref_closure(&original, &schema).unwrap();
    assert!(refs.complete && refs.slots.len() == 1);
    assert!(!validate_note(&original, &schema, &ValidationContext::default()).is_valid());
    let mut context = ValidationContext {
        owner: Some(owner.clone()),
        resources_complete: true,
        resources: vec![ResourceMetadata {
            resource_ref: resource.into(),
            owner,
            lifecycle: "live".into(),
        }],
    };
    assert!(validate_note(&original, &schema, &context).is_valid());
    context.resources[0].owner = TARGET.into();
    assert!(!validate_note(&original, &schema, &context).is_valid());
}
