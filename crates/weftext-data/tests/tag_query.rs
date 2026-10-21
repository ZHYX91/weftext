use std::collections::BTreeMap;
use weftext_data::*;

const WS: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const OTHER: &str = "708365a4-0574-491e-9676-eb179f3a8098";
fn id(n: usize) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}
fn reference(n: usize) -> String {
    format!("weftext-tag://{WS}/{}", id(n))
}
fn literal(n: usize) -> String {
    format!("tagRef('{}')", reference(n))
}
fn tree() -> FrozenTagStructure {
    FrozenTagStructure::new(
        WS.into(),
        TagDocument {
            version: 2,
            tags: (0..3)
                .map(|n| TagDefinition {
                    tag_id: id(n),
                    display_name: "same/path".into(),
                    parent_tag_id: (n > 0).then(|| id(0)),
                })
                .collect(),
        },
    )
    .unwrap()
}
fn input(rows: bool) -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "frozen-cut".into(),
        schema_revision: "schema-cut".into(),
        coverage: "complete".into(),
        freshness: "stale".into(),
        notes: if rows {
            vec![FrozenNote {
                entity_key: format!("weftext-node://{WS}/{}", id(9)),
                path: "one/one.typ".into(),
                source: format!(
                    "#metadata((id: \"{}\", types: (\"person\",))) <weftext>",
                    id(9)
                ),
                revision: "note-cut".into(),
                lifecycle: "live".into(),
                title: None,
                body: None,
                products: None,
                authorized_fields: None,
            }]
        } else {
            vec![]
        },
        advanced: Some(FrozenAdvancedInput {
            tag_structure: Some(tree()),
            ..Default::default()
        }),
    }
}
fn plan(expression: String) -> QueryPlan {
    QueryPlan {
        r#where: Some(expression),
        select: vec![QueryColumn {
            name: "value".into(),
            expr: "true".into(),
        }],
        ..Default::default()
    }
}

#[test]
fn exact_comparison_does_not_read_structure_and_move_changes_subtree_only() {
    let schema = Schema::bundled().unwrap();
    let exact = plan(format!("{} == {}", literal(1), literal(0)));
    let subtree = plan(format!("tagInSubtree({}, {})", literal(1), literal(0)));
    let mut frozen = input(true);
    assert!(
        !query_read_requirements(&exact, &schema, frozen.advanced.as_ref().unwrap())
            .unwrap()
            .tag_structure
    );
    assert!(
        query_read_requirements(&subtree, &schema, frozen.advanced.as_ref().unwrap())
            .unwrap()
            .tag_structure
    );
    assert!(
        execute_query(&frozen, &exact, &schema)
            .unwrap()
            .rows
            .is_empty()
    );
    assert_eq!(
        execute_query(&frozen, &subtree, &schema)
            .unwrap()
            .rows
            .len(),
        1
    );
    let original_source = frozen.notes[0].source.clone();
    let mut document = (*tree().document).clone();
    document.tags[1].parent_tag_id = None;
    frozen.advanced.as_mut().unwrap().tag_structure =
        Some(FrozenTagStructure::new(WS.into(), document).unwrap());
    assert!(
        execute_query(&frozen, &subtree, &schema)
            .unwrap()
            .rows
            .is_empty()
    );
    assert_eq!(frozen.notes[0].source, original_source);
    frozen.advanced = None;
    assert!(
        execute_query(&frozen, &exact, &schema)
            .unwrap()
            .rows
            .is_empty()
    );
    assert!(execute_query(&frozen, &subtree, &schema).is_err());
}

#[test]
fn zero_rows_literals_negation_and_unvisited_union_validate_all_roots() {
    let schema = Schema::bundled().unwrap();
    let frozen = input(false);
    for expression in [
        format!("tagInSubtree({}, {})", literal(1), literal(99)),
        format!("true || !tagInSubtree({}, {})", literal(1), literal(99)),
    ] {
        assert!(execute_query(&frozen, &plan(expression), &schema).is_err());
    }
    assert!(
        execute_query(
            &frozen,
            &plan(format!("!tagInSubtree({}, {})", literal(1), literal(0))),
            &schema
        )
        .is_ok()
    );
    let union = QueryPlan {
        take: Some(0),
        advanced: Some(QueryAdvanced {
            union_all: vec![
                plan("true".into()),
                plan(format!("tagInSubtree({}, {})", literal(1), literal(99))),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(execute_query(&frozen, &union, &schema).is_err());
}

#[test]
fn typed_frozen_parameter_is_valid_but_unknown_candidate_stays_partial() {
    let schema = Schema::bundled().unwrap();
    let mut frozen = input(true);
    frozen.advanced.as_mut().unwrap().parameters =
        BTreeMap::from([("root".into(), NativeValue::Text(reference(0)))]);
    let mut query = plan(format!("tagInSubtree({}, root)", literal(1)));
    query.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "root".into(),
            shape: Shape::Scalar {
                semantic: "tag_ref".into(),
            },
            nullable: false,
            default: None,
        }],
        ..Default::default()
    });
    assert_eq!(
        execute_query(&frozen, &query, &schema).unwrap().rows.len(),
        1
    );
    let mut shadowed = query.clone();
    shadowed.r#where = Some(format!(
        "[{}].exists(root, tagInSubtree(root, root))",
        literal(1)
    ));
    assert!(execute_query(&frozen, &shadowed, &schema).is_err());
    assert!(
        validate_query_plan_with_context(&shadowed, &schema, frozen.advanced.as_ref().unwrap())
            .is_err()
    );
    frozen.advanced.as_mut().unwrap().parameters.clear();
    let unknown = plan(format!(
        "tagInSubtree({}, {}) || true",
        literal(99),
        literal(0)
    ));
    let result = execute_query(&frozen, &unknown, &schema).unwrap();
    assert_eq!(result.coverage, "partial");
    assert!(!result.aggregate_exact);
    let foreign = plan(format!(
        "tagInSubtree(tagRef('weftext-tag://{OTHER}/{}'), {})",
        id(1),
        literal(0)
    ));
    let result = execute_query(&frozen, &foreign, &schema).unwrap();
    assert!(result.rows.is_empty());
    assert_eq!(result.coverage, "complete");
    let foreign_root = plan(format!(
        "tagInSubtree({}, tagRef('weftext-tag://{OTHER}/{}'))",
        literal(1),
        id(0)
    ));
    assert!(execute_query(&frozen, &foreign_root, &schema).is_err());
    assert!(validate_query_plan(&plan("tagInSubtree('text', 'text')".into()), &schema).is_err());
}

#[test]
fn json_cannot_supply_the_trusted_tag_capability() {
    let context = FrozenAdvancedInput {
        tag_structure: Some(tree()),
        ..Default::default()
    };
    let wire = serde_json::to_value(&context).unwrap();
    assert!(wire.get("tagStructure").is_none());
    let roundtrip: FrozenAdvancedInput = serde_json::from_value(wire).unwrap();
    assert!(roundtrip.tag_structure.is_none());
}

#[test]
fn multiple_direct_leaves_count_one_note_and_empty_arrays_keep_static_tree_effect() {
    let schema = Schema::bundled().unwrap();
    let mut frozen = input(true);
    frozen.notes[0].source = format!(
        "#metadata((id: \"{}\", types: (\"person\",), tags: ((ref: \"{}\",), (ref: \"{}\",)))) <weftext>",
        id(9),
        reference(1),
        reference(2)
    );
    let mut query = plan(format!(
        "tags.exists(t, has(t.ref) ? tagInSubtree(t.ref, {}) : false)",
        literal(0)
    ));
    query.group = Some(QueryGroup {
        keys: vec![],
        aggregates: vec![QueryAggregate {
            name: "notes".into(),
            op: "count".into(),
            expr: None,
        }],
    });
    query.select.clear();
    let result = execute_query(&frozen, &query, &schema).unwrap();
    assert_eq!(
        result.rows[0].cells["notes"].native(),
        Some(&NativeValue::Int("1".into()))
    );
    frozen.notes[0].source = format!(
        "#metadata((id: \"{}\", types: (\"person\",), tags: ())) <weftext>",
        id(9)
    );
    assert!(execute_query(&frozen, &query, &schema).is_ok());
    frozen.advanced.as_mut().unwrap().tag_structure = None;
    assert!(execute_query(&frozen, &query, &schema).is_err());
}

#[test]
fn pinned_definition_reuses_its_definition_with_the_current_frozen_tree() {
    let schema = Schema::bundled().unwrap();
    let key = id(8);
    let revision = id(7);
    let owner = format!("weftext-node://{WS}/{}", id(9));
    let expression = format!("tagInSubtree({}, root)", literal(1));
    let definition = format!(
        "#metadata((kind: \"weftext.query\", key: \"{key}\", binding: (schema: \"weftext-config://{WS}/schema\", revision: \"{revision}\", language: \"weftext.query.cel.v1\"), where: {}, select: ((name: \"value\", expr: \"true\"),), advanced: (parameters: ((name: \"root\", shape: (kind: \"scalar\", semantic: \"tag_ref\"), nullable: false, default: \"{}\"),),))) <weftext-query-tree>",
        serde_json::to_string(&expression).unwrap(),
        reference(0)
    );
    let mut frozen = input(true);
    let context = frozen.advanced.as_mut().unwrap();
    context.definitions = vec![FrozenDefinitionOwner {
        owner: owner.clone(),
        history: vec![DefinitionRevision {
            revision: "old-definition".into(),
            source: definition,
            current: false,
        }],
    }];
    context.schemas = vec![FrozenDefinitionSchema {
        schema: format!("weftext-config://{WS}/schema"),
        revision,
        projection: schema.clone(),
    }];
    let query = QueryPlan {
        advanced: Some(QueryAdvanced {
            call: Some(QueryCall {
                owner,
                selection: DefinitionSelection::Pinned {
                    key,
                    revision: "old-definition".into(),
                },
                arguments: BTreeMap::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        query_read_requirements(&query, &schema, frozen.advanced.as_ref().unwrap())
            .unwrap()
            .tag_structure
    );
    assert_eq!(
        execute_query(&frozen, &query, &schema).unwrap().rows.len(),
        1
    );
    let mut document = (*tree().document).clone();
    document.tags[1].parent_tag_id = None;
    frozen.advanced.as_mut().unwrap().tag_structure =
        Some(FrozenTagStructure::new(WS.into(), document).unwrap());
    assert!(
        execute_query(&frozen, &query, &schema)
            .unwrap()
            .rows
            .is_empty()
    );
    frozen.advanced.as_mut().unwrap().tag_structure = None;
    assert!(execute_query(&frozen, &query, &schema).is_err());
}

#[test]
fn definition_preflight_validates_closed_roots_under_short_circuit_and_defers_only_real_parameters()
{
    let schema = Schema::bundled().unwrap();
    let context = input(false).advanced.unwrap();
    let bad = plan(format!(
        "false && tagInSubtree({}, {})",
        literal(1),
        literal(99)
    ));
    assert!(validate_query_plan_with_context(&bad, &schema, &context).is_err());
    assert!(preflight_query_definition(&bad, &schema, &context, &QueryBudget::default()).is_err());
    let mut deferred = plan(format!("false && tagInSubtree({}, root)", literal(1)));
    deferred.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "root".into(),
            shape: Shape::Scalar {
                semantic: "tag_ref".into(),
            },
            nullable: false,
            default: None,
        }],
        ..Default::default()
    });
    validate_query_plan_with_context(&deferred, &schema, &context).unwrap();
    let checked =
        preflight_query_definition(&deferred, &schema, &context, &QueryBudget::default()).unwrap();
    assert_eq!(checked.deferred_parameters, vec!["root"]);
    assert!(
        query_read_requirements(&deferred, &schema, &context)
            .unwrap()
            .tag_structure
    );
    assert!(prepare_tag_structure(&deferred, &schema, &context).is_err());
    let mut no_tree = context.clone();
    no_tree.tag_structure = None;
    assert!(validate_query_plan_with_context(&deferred, &schema, &no_tree).is_err());
    let mut default_bad = deferred.clone();
    default_bad.advanced.as_mut().unwrap().parameters[0].default =
        Some(NativeValue::Text(reference(99)));
    assert!(
        preflight_query_definition(&default_bad, &schema, &context, &QueryBudget::default())
            .is_err()
    );
    let mut unbound_sibling = bad.clone();
    unbound_sibling.advanced = deferred.advanced.clone();
    assert!(
        preflight_query_definition(&unbound_sibling, &schema, &context, &QueryBudget::default())
            .is_err()
    );
    let key = id(8);
    let revision = id(7);
    let owner = format!("weftext-node://{WS}/{}", id(9));
    let expression = format!("false && tagInSubtree({}, root)", literal(1));
    let definition = format!(
        "#metadata((kind: \"weftext.query\", key: \"{key}\", binding: (schema: \"weftext-config://{WS}/schema\", revision: \"{revision}\", language: \"weftext.query.cel.v1\"), where: {}, select: ((name: \"value\", expr: \"true\"),), advanced: (parameters: ((name: \"root\", shape: (kind: \"scalar\", semantic: \"tag_ref\"), nullable: false, default: \"{}\"),),))) <weftext-query-tree>",
        serde_json::to_string(&expression).unwrap(),
        reference(99)
    );
    let mut nested_context = context.clone();
    nested_context.definitions = vec![FrozenDefinitionOwner {
        owner: owner.clone(),
        history: vec![DefinitionRevision {
            revision: "callee".into(),
            source: definition,
            current: true,
        }],
    }];
    nested_context.schemas = vec![FrozenDefinitionSchema {
        schema: format!("weftext-config://{WS}/schema"),
        revision,
        projection: schema.clone(),
    }];
    let mut caller = QueryPlan {
        advanced: Some(QueryAdvanced {
            parameters: deferred.advanced.as_ref().unwrap().parameters.clone(),
            call: Some(QueryCall {
                owner,
                selection: DefinitionSelection::Current { key },
                arguments: BTreeMap::new(),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        preflight_query_definition(&caller, &schema, &nested_context, &QueryBudget::default())
            .is_err()
    );
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("root".into(), "root".into());
    // An unknown actual argument overrides the callee default without borrowing it.
    preflight_query_definition(&caller, &schema, &nested_context, &QueryBudget::default()).unwrap();
}
