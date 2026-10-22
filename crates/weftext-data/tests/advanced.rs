use std::collections::BTreeMap;
use weftext_data::*;
mod support;
use support::assert_data_error;
const WS: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const A: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";
const B: &str = "708365a4-0574-491e-9676-eb179f3a8098";
const C: &str = "7f28ed24-c09b-4e54-a1ba-2c1a966781c6";
const KEY: &str = "eb8ee74a-330c-45d2-bb8b-82a6d5210e43";
const REV: &str = "8a81e35f-ad48-4c12-af4f-3d2607102fa9";
fn reference(id: &str) -> String {
    format!("weftext-node://{WS}/{id}")
}

#[test]
fn projected_schema_parameters_validate_the_declared_shape_without_reinstallation() {
    let schema = narrow_query_schema(
        &Schema::bundled().unwrap(),
        &std::collections::BTreeSet::from(["tasks/status".into()]),
    )
    .unwrap();
    assert!(schema.fields[0].default.is_none());
    let plan = QueryPlan {
        r#where: Some("status == wanted".into()),
        select: vec![column("status", "status")],
        advanced: Some(QueryAdvanced {
            parameters: vec![QueryParameter {
                name: "wanted".into(),
                shape: Shape::Scalar {
                    semantic: "text".into(),
                },
                nullable: false,
                default: None,
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut frozen = input(vec![FrozenNote {
        entity_key: reference(A),
        path: "resource:one".into(),
        source: String::new(),
        revision: REV.into(),
        lifecycle: "live".into(),
        title: None,
        body: None,
        products: None,
        authorized_fields: Some(BTreeMap::from([(
            "tasks/status".into(),
            Cell::value(NativeValue::Text("open".into())),
        )])),
    }]);
    frozen.advanced = Some(FrozenAdvancedInput {
        parameters: BTreeMap::from([("wanted".into(), NativeValue::Text("open".into()))]),
        ..Default::default()
    });
    validate_query_plan_with_context(&plan, &schema, frozen.advanced.as_ref().unwrap()).unwrap();
    let result = execute_query(&frozen, &plan, &schema).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].cells["status"].native().unwrap().text(),
        Some("open")
    );
    frozen
        .advanced
        .as_mut()
        .unwrap()
        .parameters
        .insert("wanted".into(), NativeValue::Int("1".into()));
    assert!(execute_query(&frozen, &plan, &schema).is_err());
    frozen.advanced.as_mut().unwrap().parameters.clear();
    assert!(execute_query(&frozen, &plan, &schema).is_err());
    let mut unknown = plan;
    unknown.advanced.as_mut().unwrap().parameters[0].shape = Shape::Use {
        name: "unregistered_shape".into(),
    };
    assert!(
        validate_query_plan_with_context(&unknown, &schema, &FrozenAdvancedInput::default())
            .is_err()
    );
}

#[test]
fn read_effects_bind_terminal_columns_and_lexical_locals_before_products_or_fields() {
    let schema = Schema::bundled().unwrap();
    let context = FrozenAdvancedInput::default();
    let branch = QueryPlan {
        select: vec![column("body", "1")],
        ..Default::default()
    };
    let terminal = QueryPlan {
        r#where: Some("body > 0".into()),
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![branch.clone(), branch]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let demands = query_read_requirements(&terminal, &schema, &context).unwrap();
    assert!(demands.products.is_empty());
    assert!(demands.fields.is_empty());
    assert_eq!(query_output_columns(&terminal, &[]).unwrap(), vec!["body"]);
    let branch = QueryPlan {
        select: vec![column("status", "1")],
        ..Default::default()
    };
    let terminal = QueryPlan {
        select: vec![column("renamed", "status")],
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![branch]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let demands = query_read_requirements(&terminal, &schema, &context).unwrap();
    assert!(demands.fields.is_empty());
    assert!(demands.products.is_empty());
    for expr in [
        "[1, 2].map(status, status + 1)",
        "[\"x\"].map(body, body)",
        "[\"x\"].exists(title, title == \"x\")",
    ] {
        let demands = query_read_requirements(&project(expr), &schema, &context).unwrap();
        assert!(demands.products.is_empty(), "{expr}");
        assert!(demands.fields.is_empty(), "{expr}");
    }
    let grouped = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![column("body", "status")],
            aggregates: vec![],
        }),
        order: vec![QueryOrder {
            expr: "body".into(),
            descending: false,
            nulls: "last".into(),
        }],
        ..Default::default()
    };
    let demands = query_read_requirements(&grouped, &schema, &context).unwrap();
    assert_eq!(
        demands.fields,
        std::collections::BTreeSet::from(["tasks/status".into()])
    );
    assert!(demands.products.is_empty());
    let distinct = QueryPlan {
        select: vec![column("body", "1")],
        order: vec![QueryOrder {
            expr: "body".into(),
            descending: false,
            nulls: "last".into(),
        }],
        advanced: Some(QueryAdvanced {
            distinct: Some(vec![column("k", "body")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let demands = query_read_requirements(&distinct, &schema, &context).unwrap();
    assert!(demands.products.is_empty());
    assert!(demands.fields.is_empty());
    let result = execute_query(
        &input(vec![note(A, "task", ", status: \"open\"")]),
        &distinct,
        &schema,
    )
    .unwrap();
    assert_eq!(
        result.rows[0].cells["body"].native(),
        Some(&NativeValue::Int("1".into()))
    );
}

#[test]
fn trusted_query_metadata_grants_identity_and_types_without_any_source_payload() {
    let schema = narrow_query_schema(
        &Schema::bundled().unwrap(),
        &std::collections::BTreeSet::from(["tasks/status".into()]),
    )
    .unwrap();
    let frozen = input(vec![FrozenNote {
        entity_key: reference(A),
        path: "resource:one".into(),
        source: String::new(),
        revision: REV.into(),
        lifecycle: "live".into(),
        title: None,
        body: None,
        products: None,
        authorized_fields: Some(BTreeMap::from([(
            "tasks/status".into(),
            Cell::value(NativeValue::Text("open".into())),
        )])),
    }]);
    let plan = QueryPlan {
        select: vec![
            column("self", "self"),
            column("types", "types"),
            column("body", "body"),
        ],
        ..Default::default()
    };
    let denied = execute_query(&frozen, &plan, &schema).unwrap();
    assert_eq!(denied.rows[0].cells["self"].state(), "unavailable");
    assert_eq!(denied.rows[0].cells["types"].state(), "unavailable");
    let metadata = BTreeMap::from([(
        reference(A),
        AuthorizedQueryMetadata {
            identity: Some(reference(A)),
            types: Some(vec!["task".into()]),
            resolved_types: vec!["tasks/task".into()],
        },
    )]);
    let result =
        execute_authorized_query(&frozen, &plan, &schema, &QueryBudget::default(), &metadata)
            .unwrap();
    assert_eq!(
        result.rows[0].cells["self"].native().unwrap().text(),
        Some(reference(A).as_str())
    );
    assert_eq!(result.rows[0].cells["types"].state(), "value");
    assert_eq!(result.rows[0].cells["body"].state(), "unavailable");
    assert!(frozen.notes[0].source.is_empty());
}

#[test]
fn related_consumes_authorized_field_cells_and_control_types_without_parsing_a_body() {
    let full = Schema::bundled().unwrap();
    let fields = std::collections::BTreeSet::from(["people/parents".into()]);
    let relation = full
        .relations
        .iter()
        .find(|r| r.field == "people/parents")
        .unwrap()
        .clone();
    let mut schema = narrow_query_schema(&full, &fields).unwrap();
    schema.types = full.types.clone();
    schema.aliases = full.aliases.clone();
    schema.relations = vec![relation.clone()];
    let mut notes = vec![];
    let mut metadata = BTreeMap::new();
    for (id, parents) in [
        (
            A,
            format!(", parents: ((target: (ref: \"{}\"),),)", reference(B)),
        ),
        (B, String::new()),
    ] {
        let raw = format!(
            "#metadata((id: \"{id}\", types: (\"person\",){parents})) <weftext>\nPRIVATE_BODY_DO_NOT_EXECUTE"
        );
        let profile = narrow_profile(&raw, &full, &fields, None).unwrap();
        let (_, types, resolved) = narrow_query_metadata(&raw, &full).unwrap();
        metadata.insert(
            reference(id),
            AuthorizedQueryMetadata {
                identity: Some(reference(id)),
                types: Some(types),
                resolved_types: resolved,
            },
        );
        notes.push(FrozenNote {
            entity_key: reference(id),
            path: format!("resource:{id}"),
            source: String::new(),
            revision: REV.into(),
            lifecycle: "live".into(),
            title: None,
            body: None,
            products: None,
            authorized_fields: Some(
                profile
                    .fields
                    .into_iter()
                    .map(|f| (f.field_id, f.cell))
                    .collect(),
            ),
        });
    }
    let seed = QueryPlan {
        r#where: Some("self == origin".into()),
        select: vec![column("self", "self")],
        advanced: Some(QueryAdvanced {
            parameters: vec![QueryParameter {
                name: "origin".into(),
                shape: Shape::Scalar {
                    semantic: "node_ref".into(),
                },
                nullable: false,
                default: Some(NativeValue::Text(reference(A))),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut plan = QueryPlan {
        select: vec![column("self", "self")],
        advanced: Some(QueryAdvanced {
            related: Some(QueryRelated {
                relation: relation.id,
                from: Box::new(seed),
                direction: "forward".into(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let frozen = input(notes);
    let forward =
        execute_authorized_query(&frozen, &plan, &schema, &QueryBudget::default(), &metadata)
            .unwrap();
    assert_eq!(forward.rows.len(), 1);
    assert_eq!(
        forward.rows[0].entity_key.as_deref(),
        Some(reference(B).as_str())
    );
    assert!(
        !serde_json::to_string(&forward)
            .unwrap()
            .contains("PRIVATE_BODY")
    );
    let related = plan.advanced.as_mut().unwrap().related.as_mut().unwrap();
    related.direction = "inverse".into();
    related.from.advanced.as_mut().unwrap().parameters[0].default =
        Some(NativeValue::Text(reference(B)));
    let inverse =
        execute_authorized_query(&frozen, &plan, &schema, &QueryBudget::default(), &metadata)
            .unwrap();
    assert_eq!(inverse.rows.len(), 1);
    assert_eq!(
        inverse.rows[0].entity_key.as_deref(),
        Some(reference(A).as_str())
    );
}
fn note(id: &str, kind: &str, fields: &str) -> FrozenNote {
    FrozenNote {
        entity_key: reference(id),
        path: format!("{id}/{id}.typ"),
        source: format!("#metadata((id: \"{id}\", types: (\"{kind}\",){fields})) <weftext>"),
        revision: "cut-v1".into(),
        lifecycle: "live".into(),
        title: Some(id.into()),
        body: Some("body".into()),
        authorized_fields: None,
        products: None,
    }
}
fn input(notes: Vec<FrozenNote>) -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "one-cut".into(),
        schema_revision: REV.into(),
        coverage: "complete".into(),
        freshness: "stale".into(),
        notes,
        advanced: None,
    }
}
fn column(name: &str, expr: &str) -> QueryColumn {
    QueryColumn {
        name: name.into(),
        expr: expr.into(),
    }
}
fn project(expr: &str) -> QueryPlan {
    QueryPlan {
        select: vec![column("value", expr)],
        ..Default::default()
    }
}
fn custom_schema() -> Schema {
    let mut schema = Schema::bundled().unwrap();
    for (key, shape, nullable) in [
        (
            "exact",
            Shape::Scalar {
                semantic: "exact_decimal".into(),
            },
            true,
        ),
        (
            "values",
            Shape::List {
                items: Box::new(Shape::Scalar {
                    semantic: "exact_decimal".into(),
                }),
                min: 0,
                max: None,
            },
            false,
        ),
    ] {
        let id = format!("tests/{key}");
        schema.fields.push(Field {
            id: id.clone(),
            key: key.into(),
            shape,
            nullable,
            default: Some(serde_json::json!({"kind":"absent"})),
        });
        schema
            .types
            .iter_mut()
            .find(|t| t.id == "tasks/task")
            .unwrap()
            .fields
            .push(FieldContribution {
                id,
                required: false,
            });
    }
    Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap()
}
#[test]
fn union_declared_order_duplicate_rows_and_actual_terminal_types() {
    let input = input(vec![note(
        A,
        "task",
        ", status: \"open\", priority: \"high\"",
    )]);
    let plan = QueryPlan {
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![project("priority"), project("priority")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.rows.len(), 2);
    assert!(result.rows[0].key.starts_with("union/0/"));
    assert!(result.rows[1].key.starts_with("union/1/"));
    assert_eq!(
        result.terminal_schema["value"],
        QueryValueType::Scalar("text".into())
    );
    assert_eq!(result.coverage, "complete");
    assert_eq!(result.freshness, "stale");
    let mut incompatible = plan;
    incompatible
        .advanced
        .as_mut()
        .unwrap()
        .union_all
        .as_mut()
        .unwrap()[1] = project("1");
    assert_data_error(
        &execute_query(&input, &incompatible, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_union_all_terminal_types_or_columns_mismatch(),
    );
}
#[test]
fn distinct_exact_decimal_missing_null_and_ordered_duplicate_arrays() {
    let input = input(vec![
        note(
            A,
            "task",
            ", status: \"open\", exact: \"0.30\", values: (\"1.0\", \"1\")",
        ),
        note(
            B,
            "task",
            ", status: \"open\", exact: \"0.3\", values: (\"1\", \"1.00\")",
        ),
        note(
            C,
            "task",
            ", status: \"open\", exact: none, values: (\"1\",)",
        ),
        note(
            "eeac09ab-b77e-4e23-a4f4-c5562e87ee61",
            "task",
            ", status: \"open\"",
        ),
    ]);
    let mut plan = QueryPlan {
        select: vec![column("exact", "exact"), column("values", "values")],
        advanced: Some(QueryAdvanced {
            distinct: Some(vec![column("e", "exact"), column("a", "values")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&input, &plan, &custom_schema()).unwrap();
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.coverage, "complete");
    assert!(result.rows.iter().all(|r| r.read_only
        && r.entity_key.is_none()
        && r.revision.is_none()
        && r.path.is_none()));
    assert!(
        result
            .rows
            .iter()
            .any(|r| matches!(r.cells["exact"], Cell::Null))
    );
    assert!(
        result
            .rows
            .iter()
            .any(|r| matches!(r.cells["exact"], Cell::Absent))
    );
    // Array duplicates and length survive equality rather than becoming sets.
    plan.advanced.as_mut().unwrap().distinct = Some(vec![column("a", "values")]);
    assert_eq!(
        execute_query(&input, &plan, &custom_schema())
            .unwrap()
            .rows
            .len(),
        3
    );
}
#[test]
fn distinct_undecided_keys_keep_confirmed_rows_partial() {
    let input = input(vec![
        note(A, "task", ", status: \"open\", priority: \"high\""),
        note(B, "task", ", status: \"open\", priority: \"h\" + \"igh\""),
    ]);
    let mut plan = project("priority");
    plan.advanced = Some(QueryAdvanced {
        distinct: Some(vec![column("p", "value")]),
        ..Default::default()
    });
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
}
#[test]
fn related_forward_inverse_and_partial_do_not_guess_literal_targets() {
    let relation = Schema::bundled()
        .unwrap()
        .relations
        .into_iter()
        .find(|r| r.field == "people/parents")
        .unwrap()
        .id;
    let input = input(vec![
        note(
            A,
            "person",
            &format!(", parents: ((target: (ref: \"{}\"),),)", reference(B)),
        ),
        note(B, "person", ""),
        note(
            C,
            "person",
            ", parents: ((target: (text: \"same name\"),),)",
        ),
    ]);
    // Select the exact seed using a typed parameter, never a text-to-ref cast.
    let mut seed = QueryPlan {
        r#where: Some("self == origin".into()),
        ..Default::default()
    };
    seed.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "origin".into(),
            shape: Shape::Scalar {
                semantic: "node_ref".into(),
            },
            nullable: false,
            default: Some(NativeValue::Text(reference(A))),
        }],
        ..Default::default()
    });
    let mut plan = QueryPlan {
        select: vec![column("self", "self")],
        advanced: Some(QueryAdvanced {
            related: Some(QueryRelated {
                relation,
                from: Box::new(seed),
                direction: "forward".into(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity_key, Some(reference(B)));
    let related = plan.advanced.as_mut().unwrap().related.as_mut().unwrap();
    related.direction = "inverse".into();
    related.from.advanced.as_mut().unwrap().parameters[0].default =
        Some(NativeValue::Text(reference(B)));
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].entity_key, Some(reference(A)));
    let mut partial = input;
    partial.coverage = "partial".into();
    let result = execute_query(&partial, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
}
fn saved_source(default: &str, extra: &str) -> String {
    format!(
        "#metadata((kind: \"weftext.query\", key: \"{KEY}\", binding: (schema: \"weftext-config://{WS}/schema\", revision: \"{REV}\", language: \"weftext.query.cel.v1\"), select: ((name: \"n\", expr: \"threshold\"),), advanced: (parameters: ((name: \"threshold\", shape: (kind: \"scalar\", semantic: \"int\"), nullable: false, default: {default}),), {extra}))) <weftext-query-count>"
    )
}
fn saved_input(source: String) -> FrozenQueryInput {
    let mut input = input(vec![note(A, "task", ", status: \"open\"")]);
    input.advanced = Some(FrozenAdvancedInput {
        definitions: vec![FrozenDefinitionOwner {
            owner: reference(A),
            history: vec![DefinitionRevision {
                revision: "definition-v1".into(),
                source,
                current: true,
            }],
        }],
        schemas: vec![FrozenDefinitionSchema {
            schema: format!("weftext-config://{WS}/schema"),
            revision: REV.into(),
            projection: Schema::bundled().unwrap(),
        }],
        ..Default::default()
    });
    input
}
fn call_plan(selection: DefinitionSelection) -> QueryPlan {
    QueryPlan {
        advanced: Some(QueryAdvanced {
            call: Some(QueryCall {
                owner: reference(A),
                selection,
                arguments: BTreeMap::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    }
}
#[test]
fn saved_calls_actual_native_defaults_explicit_arguments_and_wrong_types() {
    let input = saved_input(saved_source("2", ""));
    let mut plan = call_plan(DefinitionSelection::Current { key: KEY.into() });
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(
        result.rows[0].cells["n"].native(),
        Some(&NativeValue::Int("2".into()))
    );
    plan.advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("threshold".into(), "3".into());
    assert_eq!(
        execute_query(&input, &plan, &Schema::bundled().unwrap())
            .unwrap()
            .rows[0]
            .cells["n"]
            .native(),
        Some(&NativeValue::Int("3".into()))
    );
    plan.advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("threshold".into(), "3.0".into());
    assert_data_error(
        &execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_parameter_shape_mismatch(
            weftext_messages::params::ErrorDataQueryAdvancedParameterShapeMismatch {
                parameter_name: "threshold".into(),
            },
        ),
    );
    let invalid = saved_input(saved_source("\"wrong\"", ""));
    assert!(
        execute_query(
            &invalid,
            &call_plan(DefinitionSelection::Current { key: KEY.into() }),
            &Schema::bundled().unwrap()
        )
        .is_err()
    );
}
#[test]
fn pinned_calls_use_exact_history_and_current_cut_data_without_fallback() {
    let mut input = saved_input(saved_source("2", ""));
    input.advanced.as_mut().unwrap().definitions[0]
        .history
        .push(DefinitionRevision {
            revision: "old-v0".into(),
            source: saved_source("7", ""),
            current: false,
        });
    let plan = call_plan(DefinitionSelection::Pinned {
        key: KEY.into(),
        revision: "old-v0".into(),
    });
    let result = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.cut_id, "one-cut");
    assert_eq!(result.rows[0].revision.as_deref(), Some("cut-v1"));
    assert_eq!(
        result.rows[0].cells["n"].native(),
        Some(&NativeValue::Int("7".into()))
    );
    let absent = call_plan(DefinitionSelection::Pinned {
        key: KEY.into(),
        revision: "discarded".into(),
    });
    assert_data_error(
        &execute_query(&input, &absent, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_advanced_definition_revision_missing_or_ambiguous(),
    );
    input.advanced.as_mut().unwrap().schemas.clear();
    assert_data_error(
        &execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_advanced_definition_schema_baseline_missing_or_ambiguous(),
    );
}
#[test]
fn recursive_saved_calls_reject_before_any_row_execution() {
    let source = saved_source(
        "2",
        &format!(
            "call: (owner: \"{}\", selection: (mode: \"current\", key: \"{KEY}\")),",
            reference(A)
        ),
    );
    let input = saved_input(source);
    let plan = call_plan(DefinitionSelection::Current { key: KEY.into() });
    let error = execute_query(&input, &plan, &Schema::bundled().unwrap()).unwrap_err();
    assert_data_error(
        &error,
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_definition_call_cycle(),
    );
}
#[test]
fn native_writer_keeps_domain_defaults_without_transport_literal_records() {
    let schema = Schema::bundled().unwrap();
    let mut plan = project("d");
    plan.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "d".into(),
            shape: Shape::Scalar {
                semantic: "decimal".into(),
            },
            nullable: false,
            default: Some(NativeValue::Decimal("0.30".into())),
        }],
        ..Default::default()
    });
    let binding = SavedQueryBinding {
        schema: format!("weftext-config://{WS}/schema"),
        revision: REV.into(),
        language: "weftext.query.cel.v1".into(),
    };
    let patch = upsert_saved_query(
        &note(A, "task", ", status: \"open\"").source,
        None,
        KEY,
        "weftext-query-d",
        &binding,
        &plan,
        &schema,
    )
    .unwrap();
    assert!(
        patch.source.contains("\"default\": decimal(\"0.30\")"),
        "{}",
        patch.source
    );
    assert!(!patch.source.contains("kind: \"decimal\""));
    let parsed = saved_queries(&patch.source).unwrap();
    assert_eq!(
        parsed[0].plan.advanced.as_ref().unwrap().parameters[0].default,
        Some(NativeValue::Decimal("0.30".into()))
    );
}
#[test]
fn products_are_actual_free_symbols_and_html_identity_preserves_absence() {
    let plan = QueryPlan {
        select: vec![column("p", "phones.exists(body, body.number == \"x\")")],
        ..Default::default()
    };
    assert!(query_products(&plan, &[]).unwrap().is_empty());
    let plan = QueryPlan {
        select: vec![column("p", "headings.exists(h, h.level > 9)")],
        ..Default::default()
    };
    assert_eq!(query_products(&plan, &[]).unwrap(), vec!["headings"]);
    let mut n = note(A, "task", ", status: \"open\"");
    n.products = Some(NoteProducts {
        compile_identity: "compiled-at-one-cut".into(),
        target: "html".into(),
        title: Cell::Absent,
        body: Cell::value(NativeValue::Text("visible final text".into())),
        headings: Cell::value(NativeValue::Array(vec![])),
    });
    let result = execute_query(
        &input(vec![n.clone()]),
        &project("title"),
        &Schema::bundled().unwrap(),
    )
    .unwrap();
    assert!(matches!(result.rows[0].cells["value"], Cell::Absent));
    n.products.as_mut().unwrap().target = "paged".into();
    let result = execute_query(
        &input(vec![n]),
        &project("body"),
        &Schema::bundled().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        result.rows[0].cells["value"],
        Cell::Unavailable { .. }
    ));
}

#[test]
fn union_terminal_group_uses_real_exact_aggregates_and_partial_state() {
    let mut plan = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![],
            aggregates: vec![
                QueryAggregate {
                    name: "sum".into(),
                    op: "sum".into(),
                    expr: Some("value".into()),
                },
                QueryAggregate {
                    name: "count".into(),
                    op: "count".into(),
                    expr: None,
                },
            ],
        }),
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![
                project("exactDecimal(\"0.1\")"),
                project("exactDecimal(\"0.2\")"),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(
        result.rows[0].cells["sum"].native(),
        Some(&NativeValue::Text("0.3".into()))
    );
    assert_eq!(
        result.rows[0].cells["count"].native(),
        Some(&NativeValue::Int("2".into()))
    );
    assert!(result.rows[0].read_only && result.rows[0].entity_key.is_none());
    frozen.coverage = "partial".into();
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert!(!result.aggregate_exact);
    assert_eq!(result.coverage, "partial");
    plan.advanced.as_mut().unwrap().union_all = Some(vec![project("1"), project("2")]);
    plan.group.as_mut().unwrap().aggregates[0].expr = Some("value + 0.5".into());
    assert!(execute_query(&frozen, &plan, &Schema::bundled().unwrap()).is_err());
}

#[test]
fn terminal_union_distinct_order_take_and_host_budget_are_bounded() {
    let frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    let plan = QueryPlan {
        order: vec![QueryOrder {
            expr: "value".into(),
            descending: true,
            nulls: "last".into(),
        }],
        take: Some(1),
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![project("1"), project("2"), project("1")]),
            distinct: Some(vec![column("n", "value")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.total_known, 2);
    assert!(result.limited);
    assert_eq!(
        result.rows[0].cells["value"].native(),
        Some(&NativeValue::Int("2".into()))
    );
    let budget = QueryBudget {
        max_rows: 1,
        ..Default::default()
    };
    let result =
        execute_query_with_budget(&frozen, &plan, &Schema::bundled().unwrap(), &budget).unwrap();
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
    let mut nested = project("1");
    for _ in 0..66 {
        nested = QueryPlan {
            advanced: Some(QueryAdvanced {
                union_all: Some(vec![nested]),
                ..Default::default()
            }),
            ..Default::default()
        };
    }
    assert_data_error(
        &execute_query(&frozen, &nested, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Budget,
        UiMessage::error_data_query_advanced_definition_call_expansion_budget_exhausted(),
    );
}

#[test]
fn parameters_missing_default_and_explicit_null_are_distinct_and_products_follow_callees() {
    let mut plan = project("p");
    plan.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "p".into(),
            shape: Shape::Scalar {
                semantic: "int".into(),
            },
            nullable: true,
            default: Some(NativeValue::Int("2".into())),
        }],
        ..Default::default()
    });
    let mut frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    assert_eq!(
        execute_query(&frozen, &plan, &Schema::bundled().unwrap())
            .unwrap()
            .rows[0]
            .cells["value"]
            .native(),
        Some(&NativeValue::Int("2".into()))
    );
    frozen.advanced = Some(FrozenAdvancedInput {
        parameters: BTreeMap::from([("p".into(), NativeValue::None)]),
        ..Default::default()
    });
    assert!(matches!(
        execute_query(&frozen, &plan, &Schema::bundled().unwrap())
            .unwrap()
            .rows[0]
            .cells["value"],
        Cell::Null
    ));
    plan.advanced.as_mut().unwrap().parameters[0].default = None;
    frozen.advanced = None;
    assert_data_error(
        &execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_required_parameter_missing(
            weftext_messages::params::ErrorDataQueryAdvancedRequiredParameterMissing {
                parameter_name: "p".into(),
            },
        ),
    );
    assert!(validate_query_plan(&plan, &Schema::bundled().unwrap()).is_ok());
    let frozen =
        saved_input(saved_source("2", "").replace("expr: \"threshold\"", "expr: \"body\""));
    assert_eq!(
        query_products(
            &call_plan(DefinitionSelection::Current { key: KEY.into() }),
            &frozen.advanced.unwrap().definitions
        )
        .unwrap(),
        vec!["body"]
    );
}

#[test]
fn real_cel_recurrence_results_and_provider_partial_survive_boolean_absorption() {
    let event = r#"{"range":{"start":date("2025-01-01"),"end_exclusive":date("2025-01-02")},"recurrence":{"frequency":"daily","interval":1,"calendar":"gregorian","calendar_version":"proleptic-gregorian.v1","zone":"UTC","zone_version":"fixed-utc.v1","fold":"earlier","gap":"skip","count":2}}"#;
    let window = r#"{"start":date("2025-01-01"),"end_exclusive":date("2025-01-10")}"#;
    let budget = r#"{"max_steps":1000,"max_occurrences":100,"max_bytes":100000}"#;
    let call = format!("expandRecurrence({event},{window},{budget})");
    let frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    let result = execute_query(
        &frozen,
        &project(&format!("{call}.occurrences.size()")),
        &Schema::bundled().unwrap(),
    )
    .unwrap();
    assert_eq!(
        result.rows[0].cells["value"].native(),
        Some(&NativeValue::Int("2".into()))
    );
    assert_eq!(result.coverage, "complete");
    let call = call.replace("max_occurrences\":100", "max_occurrences\":1");
    let plan = QueryPlan {
        r#where: Some(format!("({call}.status == \"partial\") && false")),
        ..Default::default()
    };
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert!(result.rows.is_empty());
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
    let result = execute_query(
        &frozen,
        &project("1e308 * 1e308"),
        &Schema::bundled().unwrap(),
    )
    .unwrap();
    assert!(matches!(
        result.rows[0].cells["value"],
        Cell::Unavailable { .. }
    ));
}

#[test]
fn dependency_discovery_loads_new_pinned_revision_then_rejects_real_missing_history() {
    let frozen = saved_input(saved_source("2", ""));
    let owners = &frozen.advanced.as_ref().unwrap().definitions;
    let plan = call_plan(DefinitionSelection::Pinned {
        key: KEY.into(),
        revision: "not-yet-loaded".into(),
    });
    let deps = definition_dependencies(&plan, owners).unwrap();
    assert_eq!(deps.len(), 1);
    assert_data_error(
        &execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_advanced_definition_revision_missing_or_ambiguous(),
    );
}

#[test]
fn exact_context_writer_validates_saved_call_terminal_types_and_arguments() {
    let frozen = saved_input(saved_source("2", ""));
    let context = frozen.advanced.unwrap();
    let mut plan = call_plan(DefinitionSelection::Current { key: KEY.into() });
    plan.select = vec![column("doubled", "n + n")];
    assert!(validate_query_plan_with_context(&plan, &Schema::bundled().unwrap(), &context).is_ok());
    let binding = SavedQueryBinding {
        schema: format!("weftext-config://{WS}/schema"),
        revision: REV.into(),
        language: "weftext.query.cel.v1".into(),
    };
    let saved = upsert_saved_query_with_context(
        &note(B, "task", ", status: \"open\"").source,
        None,
        C,
        "weftext-query-call",
        &binding,
        &plan,
        &Schema::bundled().unwrap(),
        &context,
    )
    .unwrap();
    assert!(saved_queries(&saved.source).is_ok());
    let replay = upsert_saved_query_replay(
        &note(B, "task", ", status: \"open\"").source,
        None,
        C,
        "weftext-query-call",
        &binding,
        &plan,
        &Schema::bundled().unwrap(),
    )
    .unwrap();
    assert_eq!(replay.source, saved.source);
    plan.advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("threshold".into(), "2.0".into());
    assert_data_error(
        &validate_query_plan_with_context(&plan, &Schema::bundled().unwrap(), &context)
            .unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_saved_call_argument_shape_mismatch(),
    );
    plan.advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("threshold".into(), "field(\"tasks/status\")".into());
    assert!(
        validate_query_plan_with_context(&plan, &Schema::bundled().unwrap(), &context).is_err()
    );
}

#[test]
fn typed_scope_refs_use_frozen_binding_in_nested_callees_and_lifecycles_are_explicit() {
    let directory = format!("weftext-directory://{WS}/{B}");
    let mut frozen = saved_input(saved_source("2", "").replace(
        "select:",
        &format!("scope: (directory: \"{directory}\"), select:"),
    ));
    let plan = call_plan(DefinitionSelection::Current { key: KEY.into() });
    let owners = &frozen.advanced.as_ref().unwrap().definitions;
    assert_eq!(
        required_query_scopes(&plan, owners).unwrap(),
        vec![directory.clone()]
    );
    assert_eq!(
        required_query_lifecycles(&plan, owners).unwrap(),
        vec!["live"]
    );
    assert_data_error(
        &execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_advanced_frozen_directory_binding_missing(),
    );
    frozen
        .advanced
        .as_mut()
        .unwrap()
        .scope_bindings
        .insert(directory, A.into());
    assert_eq!(
        execute_query(&frozen, &plan, &Schema::bundled().unwrap())
            .unwrap()
            .rows
            .len(),
        1
    );
    let live = project("1");
    let mut trash = project("1");
    trash.scope.lifecycle = "trash".into();
    let mixed = QueryPlan {
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![live, trash]),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        required_query_lifecycles(&mixed, &[]).unwrap(),
        vec!["live", "trash"]
    );
}

#[test]
fn literal_controls_and_typed_constants_reject_without_reading_rows() {
    let source = saved_source("2", "");
    assert!(
        saved_queries(&source.replace("kind: \"weftext.query\"", "kind: (\"weftext.query\")"))
            .is_err()
    );
    assert!(
        saved_queries(&source.replace(&format!("key: \"{KEY}\""), &format!("key: (\"{KEY}\")")))
            .is_err()
    );
    let frozen = input(vec![]);
    assert!(
        execute_query(
            &frozen,
            &project("nodeRef(\"wrong\")"),
            &Schema::bundled().unwrap()
        )
        .is_err()
    );
    assert!(
        execute_query(
            &frozen,
            &project("date(\"2023-02-29\")"),
            &Schema::bundled().unwrap()
        )
        .is_err()
    );
    let omitted_union =
        serde_json::from_value::<QueryPlan>(serde_json::json!({"advanced":{}})).unwrap();
    assert!(omitted_union.advanced.as_ref().unwrap().union_all.is_none());
    assert!(
        serde_json::to_value(&omitted_union).unwrap()["advanced"]
            .get("unionAll")
            .is_none()
    );
    assert!(validate_query_plan(&omitted_union, &Schema::bundled().unwrap()).is_ok());
    let empty_union =
        serde_json::from_value::<QueryPlan>(serde_json::json!({"advanced":{"unionAll":[]}}))
            .unwrap();
    assert_eq!(
        empty_union
            .advanced
            .as_ref()
            .unwrap()
            .union_all
            .as_ref()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        serde_json::to_value(&empty_union).unwrap()["advanced"]["unionAll"],
        serde_json::json!([])
    );
    assert_data_error(
        &validate_query_plan(&empty_union, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_union_requires_producer(),
    );
    assert_data_error(
        &execute_query(&input(vec![]), &empty_union, &Schema::bundled().unwrap()).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_union_requires_producer(),
    );
    let frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    let plan = QueryPlan {
        r#where: Some(format!("self == nodeRef(\"{}\")", reference(A))),
        ..Default::default()
    };
    assert_eq!(
        execute_query(&frozen, &plan, &Schema::bundled().unwrap())
            .unwrap()
            .rows
            .len(),
        1
    );
    assert_eq!(
        execute_query(
            &frozen,
            &project("exactSeconds(\"0.123\")"),
            &Schema::bundled().unwrap()
        )
        .unwrap()
        .terminal_schema["value"],
        QueryValueType::Scalar("exact_seconds".into())
    );
    let schema = Schema::bundled().unwrap();
    let mut aliases = schema.clone();
    aliases
        .aliases
        .insert("new-alias".into(), "tasks/task".into());
    assert!(baseline_contract_compatible(&schema, &aliases).unwrap());
    aliases.fields[0].nullable = !aliases.fields[0].nullable;
    assert!(!baseline_contract_compatible(&schema, &aliases).unwrap());
}

#[test]
fn cel_literal_records_keep_member_types_authored_iteration_and_exact_nested_equality() {
    let frozen = input(vec![note(A, "task", ", status: \"open\"")]);
    let schema = Schema::bundled().unwrap();
    let plan = QueryPlan {
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![
                project(r#"{"d":exactDecimal("0.30"),"a":[1,1]}"#),
                project(r#"{"a":[1,1],"d":exactDecimal("0.3")}"#),
            ]),
            distinct: Some(vec![column("r", "value")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(
        execute_query(&frozen, &plan, &schema).unwrap().rows.len(),
        1
    );
    let result = execute_query(&frozen, &project(r#"{"z":1,"a":2}.map(k,k)"#), &schema).unwrap();
    assert_eq!(
        result.rows[0].cells["value"].native(),
        Some(&NativeValue::Array(vec![
            NativeValue::Text("z".into()),
            NativeValue::Text("a".into())
        ]))
    );
    assert!(execute_query(&frozen, &project("[1,1.0]"), &schema).is_err());
    assert!(execute_query(&frozen, &project("1 in [1.0]"), &schema).is_err());
}

#[test]
fn unproved_exact_union_branches_remain_partial_instead_of_raw_text_keys() {
    let mut schema = custom_schema();
    let branch = |tag: &str, semantic: &str| Shape::Record {
        members: BTreeMap::from([
            (
                "tag".into(),
                Member {
                    shape: Shape::Enum {
                        values: vec![tag.into()],
                        open: false,
                    },
                    required: true,
                    nullable: false,
                },
            ),
            (
                "value".into(),
                Member {
                    shape: Shape::Scalar {
                        semantic: semantic.into(),
                    },
                    required: true,
                    nullable: false,
                },
            ),
        ]),
        unknown: "reject".into(),
        constraints: vec![],
    };
    schema.fields.push(Field {
        id: "tests/union".into(),
        key: "u".into(),
        shape: Shape::Union {
            variants: vec![branch("decimal", "exact_decimal"), branch("text", "text")],
            discriminator: Some(Discriminator::Tag { key: "tag".into() }),
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
            id: "tests/union".into(),
            required: false,
        });
    let frozen = input(vec![note(
        A,
        "task",
        ", status: \"open\", u: (tag: \"decimal\", value: \"0.30\")",
    )]);
    let plan = QueryPlan {
        select: vec![column("u", "u")],
        advanced: Some(QueryAdvanced {
            distinct: Some(vec![column("u", "u")]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&frozen, &plan, &schema).unwrap();
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
    assert!(result.rows.is_empty());
    assert!(result.diagnostics.iter().any(|d| d.explanation
        == UiMessage::error_data_query_union_equality_requires_selected_schema_branch()));
}

#[test]
fn group_sort_compiles_terminal_aliases_and_precedes_author_take() {
    let frozen = input(vec![
        note(A, "task", ", status: \"open\", priority: \"high\""),
        note(B, "task", ", status: \"open\", priority: \"low\""),
    ]);
    let plan = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![column("p", "priority")],
            aggregates: vec![QueryAggregate {
                name: "n".into(),
                op: "count".into(),
                expr: None,
            }],
        }),
        order: vec![QueryOrder {
            expr: "p".into(),
            descending: true,
            nulls: "last".into(),
        }],
        take: Some(1),
        ..Default::default()
    };
    assert!(validate_query_plan(&plan, &Schema::bundled().unwrap()).is_ok());
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.total_known, 2);
    assert!(result.limited);
    assert_eq!(
        result.rows[0].cells["p"].native(),
        Some(&NativeValue::Text("low".into()))
    );
    let mut union = plan;
    union.group.as_mut().unwrap().keys = vec![column("p", "value")];
    union.advanced = Some(QueryAdvanced {
        union_all: Some(vec![project("priority"), project("priority")]),
        ..Default::default()
    });
    let result = execute_query(&frozen, &union, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(
        result.rows[0].cells["p"].native(),
        Some(&NativeValue::Text("low".into()))
    );
}

#[test]
fn consumed_native_decode_budget_cannot_be_absorbed_and_unrelated_fields_stay_complete() {
    let mut schema = Schema::bundled().unwrap();
    schema
        .fields
        .iter_mut()
        .find(|f| f.key == "priority")
        .unwrap()
        .shape = Shape::Scalar {
        semantic: "int".into(),
    };
    let expression = format!("{}1{}", "(".repeat(70), ")".repeat(70));
    let frozen = input(vec![note(
        A,
        "task",
        &format!(", status: \"open\", priority: {expression}"),
    )]);
    let projection = weftext_data::project(&frozen.notes[0].source, &schema);
    assert!(projection.budget_exhausted);
    assert!(
        projection
            .fields
            .iter()
            .find(|f| f.key == "priority")
            .unwrap()
            .cell
            .is_native_budget()
    );
    for predicate in [
        "priority == 1 || true",
        "true || priority == 1",
        "isUnavailable(priority)",
    ] {
        let plan = QueryPlan {
            r#where: Some(predicate.into()),
            select: vec![column("status", "status")],
            ..Default::default()
        };
        let result = execute_query(&frozen, &plan, &schema).unwrap();
        assert_eq!(result.rows.len(), 1);
        assert_eq!(result.coverage, "partial");
        assert!(!result.aggregate_exact);
        assert!(result.diagnostics.iter().any(|d| d.explanation
            == UiMessage::error_data_query_advanced_consumed_field_decoder_budget_exhausted()));
    }
    let plan = QueryPlan {
        r#where: Some("status == \"open\"".into()),
        select: vec![column("status", "status")],
        ..Default::default()
    };
    let result = execute_query(&frozen, &plan, &schema).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.coverage, "complete");
    assert!(result.aggregate_exact);
}

#[test]
fn row_lifecycle_follows_actual_leaf_lineage_and_disappears_for_distinct_groups() {
    let mut trash = note(B, "task", ", status: \"open\"");
    trash.lifecycle = "trash".into();
    let frozen = input(vec![note(A, "task", ", status: \"open\""), trash]);
    let live = project("status");
    let mut trash = project("status");
    trash.scope.lifecycle = "trash".into();
    let mut plan = QueryPlan {
        advanced: Some(QueryAdvanced {
            union_all: Some(vec![live, trash]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert_eq!(result.rows[0].lifecycle.as_deref(), Some("live"));
    assert_eq!(result.rows[1].lifecycle.as_deref(), Some("trash"));
    assert!(result.rows[1].read_only);
    plan.advanced.as_mut().unwrap().distinct = Some(vec![column("s", "value")]);
    let result = execute_query(&frozen, &plan, &Schema::bundled().unwrap()).unwrap();
    assert!(
        result
            .rows
            .iter()
            .all(|r| r.entity_key.is_none() && r.lifecycle.is_none() && r.read_only)
    );
}
