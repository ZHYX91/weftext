use std::collections::BTreeMap;
use weftext_data::*;
mod support;
use support::assert_data_error;

const WS: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const KEY: &str = "eb8ee74a-330c-45d2-bb8b-82a6d5210e43";
const REV: &str = "8a81e35f-ad48-4c12-af4f-3d2607102fa9";
const OWNER: &str =
    "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/32eb9a12-e752-4bfa-8cdf-65e00f23ac26";

fn parameter(name: &str) -> QueryParameter {
    QueryParameter {
        name: name.into(),
        shape: Shape::Scalar {
            semantic: "int".into(),
        },
        nullable: false,
        default: None,
    }
}
fn plan(expression: &str) -> QueryPlan {
    QueryPlan {
        select: vec![
            QueryColumn {
                name: "closed".into(),
                expr: expression.into(),
            },
            QueryColumn {
                name: "unknown".into(),
                expr: "spare".into(),
            },
        ],
        advanced: Some(QueryAdvanced {
            parameters: vec![parameter("spare")],
            ..Default::default()
        }),
        ..Default::default()
    }
}
fn run(plan: &QueryPlan, context: &FrozenAdvancedInput) -> Result<QueryDefinitionPreflight> {
    preflight_query_definition(
        plan,
        &Schema::bundled().unwrap(),
        context,
        &QueryBudget::default(),
    )
}

#[test]
fn required_used_parameter_does_not_hide_independent_closed_temporal_overflow() {
    assert!(
        run(
            &plan("nextCivilDay(date('9999-12-31'))"),
            &FrozenAdvancedInput::default()
        )
        .is_err()
    );
    let result = run(
        &plan("nextCivilDay(date('2026-01-01'))"),
        &FrozenAdvancedInput::default(),
    )
    .unwrap();
    assert_eq!(result.deferred_parameters, ["spare"]);
    assert!(!result.requires_actual_rows);
    assert!(
        result
            .provider_dependencies
            .baselines
            .iter()
            .any(|baseline| !baseline.providers.is_empty())
    );
}

fn callee() -> (QueryPlan, FrozenAdvancedInput) {
    let schema = Schema::bundled().unwrap();
    let binding = SavedQueryBinding {
        schema: format!("weftext-config://{WS}/schema"),
        revision: REV.into(),
        language: "weftext.query.cel.v1".into(),
    };
    let callee = QueryPlan {
        select: vec![
            QueryColumn {
                name: "wanted".into(),
                expr: "wanted".into(),
            },
            QueryColumn {
                name: "amount".into(),
                expr: "amount".into(),
            },
        ],
        advanced: Some(QueryAdvanced {
            parameters: vec![
                QueryParameter {
                    name: "wanted".into(),
                    shape: Shape::Enum {
                        values: vec!["open".into()],
                        open: false,
                    },
                    nullable: false,
                    default: None,
                },
                parameter("amount"),
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    let source = upsert_saved_query(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n",
        None,
        KEY,
        "weftext-query-preflight",
        &binding,
        &callee,
        &schema,
    )
    .unwrap()
    .source;
    let context = FrozenAdvancedInput {
        definitions: vec![FrozenDefinitionOwner {
            owner: OWNER.into(),
            history: vec![DefinitionRevision {
                revision: "original-definition".into(),
                current: true,
                source,
            }],
        }],
        schemas: vec![FrozenDefinitionSchema {
            schema: binding.schema,
            revision: binding.revision,
            projection: schema,
        }],
        ..Default::default()
    };
    let caller = QueryPlan {
        advanced: Some(QueryAdvanced {
            parameters: vec![parameter("spare")],
            call: Some(QueryCall {
                owner: OWNER.into(),
                selection: DefinitionSelection::Current { key: KEY.into() },
                arguments: BTreeMap::from([
                    ("wanted".into(), "'open'".into()),
                    ("amount".into(), "spare".into()),
                ]),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    (caller, context)
}

#[test]
fn closed_call_argument_obeys_real_enum_shape_even_when_another_argument_is_unknown() {
    let (mut caller, context) = callee();
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("wanted".into(), "'closed'".into());
    assert_data_error(
        &run(&caller, &context).unwrap_err(),
        DataErrorKind::Invalid,
        UiMessage::error_data_query_advanced_parameter_shape_mismatch(
            weftext_messages::params::ErrorDataQueryAdvancedParameterShapeMismatch {
                parameter_name: "wanted".into(),
            },
        ),
    );
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("wanted".into(), "'open'".into());
    let result = run(&caller, &context).unwrap();
    assert_eq!(result.deferred_parameters, ["spare"]);
    assert!(!result.requires_actual_rows);
}

#[test]
fn known_short_circuit_keeps_unreachable_provider_failure_unexecuted_with_unknown_inputs() {
    for expression in [
        "false && (spare > 0 && nextCivilDay(date('9999-12-31')) == date('2026-01-01'))",
        "true || (spare > 0 && nextCivilDay(date('9999-12-31')) == date('2026-01-01'))",
        "false ? nextCivilDay(date('9999-12-31')) : date('2026-01-01')",
    ] {
        let result = run(&plan(expression), &FrozenAdvancedInput::default()).unwrap();
        assert_eq!(result.deferred_parameters, ["spare"], "{expression}");
        assert!(!result.requires_actual_rows, "{expression}");
    }
}

#[test]
fn unknown_guard_defers_alternatives_but_supplied_guard_uses_original_cel_choice() {
    let plan = plan("spare > 0 ? nextCivilDay(date('9999-12-31')) : date('2026-01-01')");
    let result = run(&plan, &FrozenAdvancedInput::default()).unwrap();
    assert_eq!(result.deferred_parameters, ["spare"]);
    assert!(
        result
            .provider_dependencies
            .baselines
            .iter()
            .all(|baseline| baseline.providers.is_empty())
    );
    let mut context = FrozenAdvancedInput {
        parameters: BTreeMap::from([("spare".into(), NativeValue::Int("0".into()))]),
        ..Default::default()
    };
    assert!(run(&plan, &context).is_ok());
    context
        .parameters
        .insert("spare".into(), NativeValue::Int("1".into()));
    assert!(run(&plan, &context).is_err());
}

#[test]
fn unknown_argument_does_not_hide_eager_closed_non_provider_failure() {
    assert!(run(&plan("spare + (1 / 0)"), &FrozenAdvancedInput::default()).is_err());
    assert!(run(&plan("spare + (4 / 2)"), &FrozenAdvancedInput::default()).is_ok());
}

#[test]
fn field_products_require_actual_rows_instead_of_treating_empty_type_terminals_as_evidence() {
    let result = run(&plan("title"), &FrozenAdvancedInput::default()).unwrap();
    assert!(result.requires_actual_rows);
    assert_eq!(result.deferred_parameters, ["spare"]);
}

#[test]
fn pinned_definition_and_schema_baseline_never_fall_back_to_current() {
    let (mut caller, mut context) = callee();
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .selection = DefinitionSelection::Pinned {
        key: KEY.into(),
        revision: "missing-old-definition".into(),
    };
    assert!(run(&caller, &context).is_err());
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .selection = DefinitionSelection::Current { key: KEY.into() };
    context.schemas.clear();
    assert!(run(&caller, &context).is_err());
}

#[test]
fn preflight_uses_original_budget_and_supplied_parameter_shapes() {
    let schema = Schema::bundled().unwrap();
    assert!(
        preflight_query_definition(
            &plan("1"),
            &schema,
            &FrozenAdvancedInput::default(),
            &QueryBudget {
                max_steps: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    let context = FrozenAdvancedInput {
        parameters: BTreeMap::from([("spare".into(), NativeValue::Text("wrong".into()))]),
        ..Default::default()
    };
    assert!(run(&plan("1"), &context).is_err());
}

fn actual_input(title: &str, context: &FrozenAdvancedInput) -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "original-actual-cut".into(),
        schema_revision: REV.into(),
        coverage: "complete".into(),
        freshness: "stale".into(),
        notes: vec![FrozenNote {
            entity_key: OWNER.into(),
            path: "N.typ".into(),
            source:
                "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n"
                    .into(),
            revision: "original-row-revision".into(),
            lifecycle: "live".into(),
            title: Some(title.into()),
            body: None,
            products: None,
            authorized_fields: None,
        }],
        advanced: Some(context.clone()),
    }
}

fn replay(
    invocation: &QueryDefinitionInvocation,
    title: &str,
    context: &FrozenAdvancedInput,
) -> Result<(QueryResult, QueryProviderDependencies)> {
    let mut context = context.clone();
    context.parameters = invocation.parameters.clone();
    execute_authorized_query_with_provider_dependencies(
        &actual_input(title, &context),
        &invocation.plan,
        &invocation.baseline,
        &QueryBudget::default(),
        &BTreeMap::new(),
    )
}

const ROW_DAY: &str = "nextCivilDay(title == 'done' ? date('9999-12-31') : date('2026-01-01'))";

#[test]
fn independent_row_expression_is_replayed_despite_unknown_sibling_parameter() {
    let original = plan(ROW_DAY);
    let context = FrozenAdvancedInput::default();
    let summary = run(&original, &context).unwrap();
    assert_eq!(summary.deferred_parameters, ["spare"]);
    assert!(summary.requires_actual_rows);
    assert_eq!(summary.closed_invocations.len(), 1);
    let invocation = &summary.closed_invocations[0];
    assert_eq!(invocation.plan.select.len(), 1);
    assert_eq!(invocation.plan.select[0].expr, ROW_DAY);
    assert!(invocation.parameters.is_empty());
    let (valid, providers) = replay(invocation, "open", &context).unwrap();
    assert_eq!(valid.coverage, "complete");
    assert_eq!(valid.rows.len(), 1);
    assert!(providers.baselines.iter().any(|b| !b.providers.is_empty()));
    let failed = replay(invocation, "done", &context);
    assert!(failed.is_err() || failed.unwrap().0.coverage == "partial");
    assert_eq!(original.select.len(), 2);
}

#[test]
fn parameter_controlled_row_membership_is_deferred_without_fabricated_input() {
    let mut original = plan(ROW_DAY);
    original.r#where = Some("spare > 0".into());
    let summary = run(&original, &FrozenAdvancedInput::default()).unwrap();
    assert_eq!(summary.deferred_parameters, ["spare"]);
    assert!(summary.requires_actual_rows);
    assert!(summary.closed_invocations.is_empty());
}

#[test]
fn closed_callee_rows_keep_exact_definition_and_schema_under_unknown_caller() {
    let (mut caller, mut context) = callee();
    let schema = Schema::bundled().unwrap();
    let mut callee_plan = plan(ROW_DAY);
    callee_plan.select.pop();
    callee_plan.advanced = None;
    context.definitions[0].history[0].source = upsert_saved_query(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n",
        None,
        KEY,
        "weftext-query-preflight",
        &SavedQueryBinding {
            schema: format!("weftext-config://{WS}/schema"),
            revision: REV.into(),
            language: "weftext.query.cel.v1".into(),
        },
        &callee_plan,
        &schema,
    )
    .unwrap()
    .source;
    caller
        .advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .clear();
    caller.select = vec![
        QueryColumn {
            name: "day".into(),
            expr: "closed".into(),
        },
        QueryColumn {
            name: "unknown".into(),
            expr: "spare".into(),
        },
    ];
    let summary = run(&caller, &context).unwrap();
    assert_eq!(summary.deferred_parameters, ["spare"]);
    assert!(summary.requires_actual_rows);
    assert_eq!(summary.closed_invocations.len(), 2);
    let invocation = summary
        .closed_invocations
        .iter()
        .find(|invocation| invocation.definition.is_some())
        .unwrap();
    let outer = summary
        .closed_invocations
        .iter()
        .find(|invocation| invocation.definition.is_none())
        .unwrap();
    assert!(outer.plan.advanced.as_ref().unwrap().call.is_some());
    assert_eq!(outer.plan.select.len(), 1);
    assert_eq!(outer.plan.select[0].expr, "closed");
    assert!(outer.parameters.is_empty());
    assert_eq!(
        replay(outer, "open", &context).unwrap().0.coverage,
        "complete"
    );
    let outer_failed = replay(outer, "done", &context);
    assert!(outer_failed.is_err() || outer_failed.unwrap().0.coverage == "partial");
    let source = invocation.definition.as_ref().unwrap();
    assert_eq!(source.owner, OWNER);
    assert_eq!(source.source_revision, "original-definition");
    assert_eq!(source.key, KEY);
    assert_eq!(invocation.schema_baseline.as_ref().unwrap().revision, REV);
    assert_eq!(
        replay(invocation, "open", &context).unwrap().0.coverage,
        "complete"
    );
    let failed = replay(invocation, "done", &context);
    assert!(failed.is_err() || failed.unwrap().0.coverage == "partial");
}

#[test]
fn global_empty_group_row_is_not_runtime_evidence_during_definition_preflight() {
    let original = QueryPlan {
        select: vec![QueryColumn {
            name: "one".into(),
            expr: "1".into(),
        }],
        group: Some(QueryGroup {
            keys: vec![],
            aggregates: vec![QueryAggregate {
                name: "n".into(),
                op: "count".into(),
                expr: None,
            }],
        }),
        order: vec![QueryOrder {
            expr: "nextCivilDay(n == 0 ? date('9999-12-31') : date('2026-01-01'))".into(),
            descending: false,
            nulls: "last".into(),
        }],
        ..Default::default()
    };
    let summary = run(&original, &FrozenAdvancedInput::default()).unwrap();
    assert!(summary.requires_actual_rows);
    assert_eq!(summary.closed_invocations.len(), 1);
    let mut empty = actual_input("unused", &FrozenAdvancedInput::default());
    empty.notes.clear();
    let actual = execute_query(
        &empty,
        &summary.closed_invocations[0].plan,
        &Schema::bundled().unwrap(),
    );
    assert!(actual.is_err() || actual.unwrap().coverage == "partial");
}

fn terminal_provider(kind: &str) -> (QueryPlan, FrozenAdvancedInput) {
    let producer = QueryPlan {
        select: vec![QueryColumn {
            name: "value".into(),
            expr: "title".into(),
        }],
        ..Default::default()
    };
    let mut root = plan("nextCivilDay(value == 'done' ? date('9999-12-31') : date('2026-01-01'))");
    let mut context = FrozenAdvancedInput::default();
    if kind == "union" {
        root.advanced.as_mut().unwrap().union_all = Some(vec![producer]);
    } else {
        let (_, original) = callee();
        context = original;
        context.definitions[0].history[0].source = upsert_saved_query(
            "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n",
            None,
            KEY,
            "weftext-query-preflight",
            &SavedQueryBinding {
                schema: format!("weftext-config://{WS}/schema"),
                revision: REV.into(),
                language: "weftext.query.cel.v1".into(),
            },
            &producer,
            &Schema::bundled().unwrap(),
        )
        .unwrap()
        .source;
        root.advanced.as_mut().unwrap().call = Some(QueryCall {
            owner: OWNER.into(),
            selection: DefinitionSelection::Current { key: KEY.into() },
            arguments: BTreeMap::new(),
        });
    }
    (root, context)
}

#[test]
fn independent_outer_terminal_provider_runs_over_exact_closed_call_and_union_rows() {
    for kind in ["call", "union"] {
        let (original, context) = terminal_provider(kind);
        let summary = run(&original, &context).unwrap();
        assert_eq!(summary.deferred_parameters, ["spare"]);
        let invocation = summary
            .closed_invocations
            .iter()
            .find(|invocation| {
                invocation
                    .plan
                    .select
                    .iter()
                    .any(|column| column.expr == original.select[0].expr)
            })
            .unwrap();
        assert_eq!(invocation.plan.select.len(), 1);
        assert_eq!(
            replay(invocation, "open", &context).unwrap().0.coverage,
            "complete",
            "{kind}"
        );
        let failed = replay(invocation, "done", &context);
        assert!(
            failed.is_err() || failed.unwrap().0.coverage == "partial",
            "{kind}"
        );
        assert_eq!(original.select.len(), 2);
    }
}

#[test]
fn unknown_outer_filter_does_not_invent_terminal_provider_reachability() {
    for kind in ["call", "union"] {
        let (mut original, context) = terminal_provider(kind);
        original.r#where = Some("spare > 0".into());
        let summary = run(&original, &context).unwrap();
        assert_eq!(summary.deferred_parameters, ["spare"]);
        assert!(
            summary.closed_invocations.iter().all(|invocation| {
                invocation
                    .plan
                    .select
                    .iter()
                    .all(|column| column.expr != original.select[0].expr)
            }),
            "{kind}"
        );
    }
}

#[test]
fn independent_order_distinct_and_group_providers_keep_their_actual_row_stages() {
    for stage in [
        "order",
        "distinct",
        "mixedDistinct",
        "groupKey",
        "groupAggregate",
        "groupOrder",
    ] {
        let mut original = plan("title");
        match stage {
            "order" => original.order.push(QueryOrder {
                expr: ROW_DAY.into(),
                descending: false,
                nulls: "last".into(),
            }),
            "distinct" | "mixedDistinct" => {
                let mut keys = vec![QueryColumn {
                    name: "day".into(),
                    expr:
                        "nextCivilDay(closed == 'done' ? date('9999-12-31') : date('2026-01-01'))"
                            .into(),
                }];
                if stage == "mixedDistinct" {
                    keys.push(QueryColumn {
                        name: "later".into(),
                        expr: "unknown".into(),
                    });
                }
                original.advanced.as_mut().unwrap().distinct = Some(keys);
            }
            "groupKey" => {
                original.group = Some(QueryGroup {
                    keys: vec![QueryColumn {
                        name: "day".into(),
                        expr: ROW_DAY.into(),
                    }],
                    aggregates: vec![QueryAggregate {
                        name: "n".into(),
                        op: "count".into(),
                        expr: None,
                    }],
                })
            }
            "groupAggregate" => {
                original.group = Some(QueryGroup {
                    keys: vec![],
                    aggregates: vec![QueryAggregate {
                        name: "day".into(),
                        op: "min".into(),
                        expr: Some(ROW_DAY.into()),
                    }],
                })
            }
            "groupOrder" => {
                original.group = Some(QueryGroup {
                    keys: vec![QueryColumn {
                        name: "label".into(),
                        expr: "title".into(),
                    }],
                    aggregates: vec![QueryAggregate {
                        name: "n".into(),
                        op: "count".into(),
                        expr: None,
                    }],
                });
                original.order.push(QueryOrder {
                    expr: "nextCivilDay(label == 'done' ? date('9999-12-31') : date('2026-01-01'))"
                        .into(),
                    descending: false,
                    nulls: "last".into(),
                });
            }
            _ => unreachable!(),
        }
        let context = FrozenAdvancedInput::default();
        let summary = run(&original, &context).unwrap();
        assert_eq!(summary.deferred_parameters, ["spare"], "{stage}");
        assert!(!summary.closed_invocations.is_empty(), "{stage}");
        for invocation in &summary.closed_invocations {
            assert_eq!(
                replay(invocation, "open", &context).unwrap().0.coverage,
                "complete",
                "{stage}"
            );
        }
        assert!(
            summary.closed_invocations.iter().any(|invocation| {
                match replay(invocation, "done", &context) {
                    Err(_) => true,
                    Ok((result, _)) => {
                        result.coverage == "partial"
                            || result.rows.iter().any(|row| {
                                row.cells.values().any(|cell| {
                                    matches!(cell, Cell::Invalid { .. } | Cell::Unavailable { .. })
                                })
                            })
                    }
                }
            }),
            "{stage}"
        );
    }
}
