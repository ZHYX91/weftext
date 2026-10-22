use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use weftext_data::*;
mod support;
use support::assert_data_error;

const WS: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const KEY: &str = "eb8ee74a-330c-45d2-bb8b-82a6d5210e43";
const REV: &str = "8a81e35f-ad48-4c12-af4f-3d2607102fa9";
const PRIVATE: &str =
    r#"{"calendar":"secretcal","version":"v1","precision":"day","year":1,"month":1,"day":1}"#;

fn installed() -> Schema {
    let mut schema = Schema::bundled().unwrap();
    let mut p = schema
        .providers
        .iter()
        .find(|p| p.id == "calendars/gregorian")
        .unwrap()
        .clone();
    p.id = "calendars/private".into();
    p.implementation.id = "weftext.calendar.table.v2".into();
    p.data = json!({"kind":"calendar-table","key":"secretcal","version":"v1","epoch_date":"2026-01-01","years":[{"year":1,"months":[{"number":1,"leap":false,"days":30}]},{"year":2,"months":[{"number":1,"leap":false,"days":30}]}]});
    schema.providers.push(p);
    Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap()
}
fn rule(schema: &Schema) -> String {
    schema
        .providers
        .iter()
        .find(|p| p.implementation.id == "weftext.temporal.rules.v2")
        .unwrap()
        .id
        .clone()
}
fn plan(expr: impl Into<String>) -> QueryPlan {
    QueryPlan {
        select: vec![QueryColumn {
            name: "value".into(),
            expr: expr.into(),
        }],
        ..Default::default()
    }
}
fn empty(context: FrozenAdvancedInput) -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "cut".into(),
        schema_revision: "exact".into(),
        coverage: "complete".into(),
        freshness: "current".into(),
        notes: vec![],
        advanced: Some(context),
    }
}
fn projection(
    schema: &Schema,
    access: &QueryProviderAccess,
    req: &QueryProviderRequirements,
) -> Schema {
    let mut result = narrow_query_schema(schema, &BTreeSet::from(["tasks/status".into()])).unwrap();
    let registries = narrow_query_provider_schema(schema, access, req).unwrap();
    result.providers = registries.providers;
    result.series_policies = registries.series_policies;
    result.validators = registries.validators;
    result.definitions = registries.definitions;
    result
}
fn access(req: &QueryProviderRequirements) -> QueryProviderAccess {
    QueryProviderAccess {
        providers: req.providers.keys().cloned().collect(),
        series_policies: req.series_policies.keys().cloned().collect(),
    }
}
fn run(
    input: &FrozenQueryInput,
    plan: &QueryPlan,
    schema: &Schema,
) -> Result<(QueryResult, QueryProviderDependencies)> {
    execute_authorized_query_with_provider_dependencies(
        input,
        plan,
        schema,
        &QueryBudget::default(),
        &BTreeMap::new(),
    )
}
fn record(pairs: Vec<(&str, NativeValue)>) -> NativeValue {
    NativeValue::Record(
        pairs
            .into_iter()
            .map(|(k, value)| NativeMember {
                key: k.into(),
                value,
            })
            .collect(),
    )
}
fn calendar(version: &str) -> NativeValue {
    record(vec![
        ("calendar", NativeValue::Text("secretcal".into())),
        ("version", NativeValue::Text(version.into())),
        ("precision", NativeValue::Text("day".into())),
        ("year", NativeValue::Int("1".into())),
        ("month", NativeValue::Int("1".into())),
        ("day", NativeValue::Int("1".into())),
    ])
}

#[test]
fn provider_fingerprints_use_canonical_complete_records_and_distinct_contracts() {
    let schema = installed();
    let p = schema
        .providers
        .iter()
        .find(|p| p.id == "calendars/private")
        .unwrap();
    let fp = provider_fingerprint(p).unwrap();
    let mut reordered = p.clone();
    let mut pairs = p.data.as_object().unwrap().iter().collect::<Vec<_>>();
    pairs.reverse();
    reordered.data = serde_json::Value::Object(
        pairs
            .into_iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    assert_eq!(fp, provider_fingerprint(&reordered).unwrap());
    reordered.implementation.abi += 1;
    assert_ne!(fp, provider_fingerprint(&reordered).unwrap());
    reordered = p.clone();
    reordered.data["version"] = json!("v2");
    assert_ne!(fp, provider_fingerprint(&reordered).unwrap());
    reordered = p.clone();
    reordered.limits.max_steps -= 1;
    assert_ne!(fp, provider_fingerprint(&reordered).unwrap());
    let mut policy = schema.series_policies[0].clone();
    let old = series_policy_fingerprint(&policy).unwrap();
    policy.key.push_str("-other");
    assert_ne!(old, series_policy_fingerprint(&policy).unwrap());
}

#[test]
fn provider_static_partial_known_arguments_and_closed_end_cannot_hide_private_calendar() {
    let schema = installed();
    for expr in [
        format!("compareTemporal({PRIVATE}, date('2026-01-01'))"),
        format!("containsDate({{'start':date('2025-12-31'),'end':{PRIVATE}}}, date('2026-01-01'))"),
    ] {
        let req =
            query_provider_requirements(&plan(&expr), &schema, &FrozenAdvancedInput::default())
                .unwrap()
                .remove(0);
        assert!(req.providers.contains_key("calendars/private"));
        assert!(req.providers.contains_key(&rule(&schema)));
        assert!(
            narrow_query_provider_schema(
                &schema,
                &QueryProviderAccess {
                    providers: BTreeSet::from([rule(&schema)]),
                    ..Default::default()
                },
                &req
            )
            .is_err()
        );
        let (_, dependencies) = run(
            &empty(FrozenAdvancedInput::default()),
            &plan(expr),
            &projection(&schema, &access(&req), &req),
        )
        .unwrap();
        assert!(
            dependencies.baselines[0]
                .providers
                .contains_key("calendars/private")
        );
    }
}

#[test]
fn provider_static_use_constraints_defaults_and_required_dynamic_parameter_are_exact() {
    let schema = installed();
    let mut p = plan("compareTemporal(wanted, date('2026-01-01'))");
    p.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "wanted".into(),
            shape: Shape::Use {
                name: "CalendarDate".into(),
            },
            nullable: false,
            default: Some(calendar("v1")),
        }],
        ..Default::default()
    });
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    assert!(req.definitions.contains("CalendarDate"));
    assert!(req.validators.contains("temporal/calendar-date"));
    assert!(req.providers.contains_key("calendars/private"));
    let restricted = projection(&schema, &access(&req), &req);
    validate_query_plan_with_context(&p, &restricted, &FrozenAdvancedInput::default()).unwrap();
    let (result, deps) = run(&empty(FrozenAdvancedInput::default()), &p, &restricted).unwrap();
    assert_eq!(result.coverage, "complete");
    assert!(
        deps.baselines[0]
            .providers
            .contains_key("calendars/private")
    );
    p.advanced.as_mut().unwrap().parameters[0].default = None;
    let dynamic = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    assert!(!dynamic.providers.contains_key("calendars/private"));
    assert!(dynamic.dynamic_selectors.contains("calendar"));
    validate_query_plan_with_context(
        &p,
        &projection(&schema, &access(&dynamic), &dynamic),
        &FrozenAdvancedInput::default(),
    )
    .unwrap();
    let actual = FrozenAdvancedInput {
        parameters: BTreeMap::from([("wanted".into(), calendar("v1"))]),
        ..Default::default()
    };
    assert!(
        query_provider_requirements(&p, &schema, &actual).unwrap()[0]
            .providers
            .contains_key("calendars/private")
    );
}

#[test]
fn provider_static_known_missing_versions_and_series_do_not_become_dynamic_admission() {
    let schema = installed();
    let missing = plan(format!(
        "compareTemporal({}, date('2026-01-01'))",
        PRIVATE.replace("\"v1\"", "\"missing\"")
    ));
    assert_data_error(
        &query_provider_requirements(&missing, &schema, &FrozenAdvancedInput::default())
            .unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_providers_query_provider_unavailable(),
    );
    let period = plan(
        "periodBounds({'series_key':'missing','kind':'day','calendar':'gregorian','calendar_version':'proleptic-gregorian.v1','zone':'UTC','zone_version':'fixed-utc.v1','rule':'iso8601','key':'2026-01-01'})",
    );
    assert_data_error(
        &query_provider_requirements(&period, &schema, &FrozenAdvancedInput::default())
            .unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_providers_query_provider_unavailable(),
    );
}

#[test]
fn provider_series_requires_its_own_selection_even_when_calendar_and_zone_are_allowed() {
    let schema = installed();
    let p = plan(
        "periodBounds({'series_key':'main','kind':'day','calendar':'gregorian','calendar_version':'proleptic-gregorian.v1','zone':'UTC','zone_version':'fixed-utc.v1','rule':'iso8601','key':'2026-01-01'})",
    );
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    assert_eq!(
        req.series_policies.keys().collect::<Vec<_>>(),
        vec!["calendar/main"]
    );
    let mut allowed = access(&req);
    allowed.series_policies.clear();
    assert!(narrow_query_provider_schema(&schema, &allowed, &req).is_err());
    allowed = access(&req);
    let restricted = projection(&schema, &allowed, &req);
    assert_eq!(restricted.series_policies.len(), 1);
    assert!(
        !restricted
            .series_policies
            .iter()
            .any(|p| p.id == "calendar/work-log")
    );
    let (result, deps) = run(&empty(FrozenAdvancedInput::default()), &p, &restricted).unwrap();
    assert_eq!(result.coverage, "complete");
    assert!(
        deps.baselines[0]
            .series_policies
            .contains_key("calendar/main")
    );
}

#[test]
fn provider_empty_scope_hidden_missing_and_masked_failure_are_indistinguishable() {
    let schema = installed();
    let mut selected = schema.clone();
    selected.providers.retain(|p| p.id == rule(&schema));
    selected.validators.clear();
    selected.series_policies.clear();
    let mut unavailable_results = Vec::new();
    for name in ["secretcal", "unknown"] {
        let expression = format!(
            "compareTemporal({}, date('2026-01-01')) == 0 || true",
            PRIVATE.replace("secretcal", name)
        );
        let p = plan(expression);
        let req =
            query_provider_runtime_requirements(&p, &selected, &FrozenAdvancedInput::default())
                .unwrap()
                .remove(0);
        let restricted = projection(&selected, &access(&req), &req);
        let error = run(&empty(FrozenAdvancedInput::default()), &p, &restricted).unwrap_err();
        assert_data_error(
            &error,
            DataErrorKind::Unavailable,
            UiMessage::error_data_query_provider_unavailable(),
        );
        unavailable_results.push(
            serde_json::to_value((error.kind(), error.description(), error.details())).unwrap(),
        );
        // The original local evaluator keeps its pre-existing empty-domain behavior.
        assert_eq!(
            execute_query(&empty(FrozenAdvancedInput::default()), &p, &restricted)
                .unwrap()
                .coverage,
            "complete"
        );
    }
    assert_eq!(unavailable_results[0], unavailable_results[1]);
}

#[test]
fn provider_empty_terminal_sort_and_distinct_stages_preflight_their_real_expression() {
    let schema = installed();
    let expression = format!(
        "compareTemporal({}, date('2026-01-01'))",
        PRIVATE.replace("v1", "absent")
    );
    let mut selected = schema.clone();
    selected.providers.retain(|p| p.id == rule(&schema));
    selected.validators.clear();
    selected.series_policies.clear();
    for stage in ["transform", "sort", "distinct"] {
        let mut p = plan("status");
        p.advanced = Some(QueryAdvanced {
            union_all: Some(vec![plan("status"), plan("status")]),
            ..Default::default()
        });
        if stage == "transform" {
            p.select = plan(expression.clone()).select;
        }
        if stage == "sort" {
            p.order = vec![QueryOrder {
                expr: expression.clone(),
                descending: false,
                nulls: "last".into(),
            }];
        }
        if stage == "distinct" {
            p.advanced.as_mut().unwrap().distinct = Some(vec![QueryColumn {
                name: "key".into(),
                expr: expression.clone(),
            }]);
        }
        let req =
            query_provider_runtime_requirements(&p, &selected, &FrozenAdvancedInput::default())
                .unwrap()
                .remove(0);
        assert!(
            run(
                &empty(FrozenAdvancedInput::default()),
                &p,
                &projection(&selected, &access(&req), &req)
            )
            .is_err(),
            "{stage}"
        );
    }
}

#[test]
fn provider_runtime_records_dynamic_positive_and_negative_exact_selection_without_latest() {
    let mut schema = installed();
    schema.fields.push(Field {
        id: "tasks/when".into(),
        key: "when".into(),
        shape: Shape::Use {
            name: "CalendarDate".into(),
        },
        nullable: false,
        default: None,
    });
    let p = plan("compareTemporal(when, date('2026-01-01')) == 0 || true");
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    assert!(req.dynamic_selectors.contains("calendar"));
    assert!(!req.providers.contains_key("calendars/private"));
    let mut allowed = access(&req);
    allowed.providers.insert("calendars/private".into());
    let mut restricted = projection(&schema, &allowed, &req);
    restricted.fields.push(Field {
        id: "tasks/when".into(),
        key: "when".into(),
        shape: schema.definitions["CalendarDate"].clone(),
        nullable: false,
        default: None,
    });
    for (version, coverage, selection) in [
        ("v1", "complete", Some("calendars/private")),
        ("v2", "partial", None),
    ] {
        let mut input = empty(FrozenAdvancedInput::default());
        input.notes.push(FrozenNote {
            entity_key: "owner".into(),
            path: "resource:owner".into(),
            source: String::new(),
            revision: "exact".into(),
            lifecycle: "live".into(),
            title: None,
            body: None,
            products: None,
            authorized_fields: Some(BTreeMap::from([(
                "tasks/when".into(),
                Cell::value(calendar(version)),
            )])),
        });
        let (result, deps) = run(&input, &p, &restricted).unwrap();
        assert_eq!(result.coverage, coverage);
        assert_eq!(
            deps.baselines[0].exact_provider_selections
                [&serde_json::to_string(&["calendar", "secretcal", version]).unwrap()]
                .as_deref(),
            selection
        );
    }
}

#[test]
fn provider_current_and_pinned_callees_keep_exact_original_baseline_and_compatibility() {
    let schema = installed();
    let binding = SavedQueryBinding {
        schema: format!("weftext-config://{WS}/schema"),
        revision: REV.into(),
        language: "weftext.query.cel.v1".into(),
    };
    let callee = plan(format!("compareTemporal({PRIVATE}, date('2026-01-01'))"));
    let source = upsert_saved_query(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n",
        None,
        KEY,
        "weftext-query-calendar",
        &binding,
        &callee,
        &schema,
    )
    .unwrap()
    .source;
    let context = FrozenAdvancedInput {
        definitions: vec![FrozenDefinitionOwner {
            owner: "owner".into(),
            history: vec![DefinitionRevision {
                revision: "old-version".into(),
                source,
                current: true,
            }],
        }],
        schemas: vec![FrozenDefinitionSchema {
            schema: binding.schema.clone(),
            revision: binding.revision.clone(),
            projection: schema.clone(),
        }],
        ..Default::default()
    };
    for selection in [
        DefinitionSelection::Current { key: KEY.into() },
        DefinitionSelection::Pinned {
            key: KEY.into(),
            revision: "old-version".into(),
        },
    ] {
        let p = QueryPlan {
            advanced: Some(QueryAdvanced {
                call: Some(QueryCall {
                    owner: "owner".into(),
                    selection,
                    arguments: BTreeMap::new(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        };
        let requirements = query_provider_requirements(&p, &schema, &context).unwrap();
        assert_eq!(requirements.len(), 2);
        assert!(requirements[0].baseline.is_none());
        assert_eq!(requirements[1].baseline.as_ref().unwrap().revision, REV);
        let effect = &requirements[1];
        let restricted = projection(&schema, &access(effect), effect);
        let mut frozen = context.clone();
        frozen.schemas[0].projection = restricted.clone();
        let (_, deps) = run(&empty(frozen.clone()), &p, &restricted).unwrap();
        assert!(
            deps.baselines
                .iter()
                .any(|d| d.baseline.as_ref().is_some_and(|b| b.revision == REV)
                    && d.providers.contains_key("calendars/private"))
        );
        let mut changed = restricted.clone();
        changed
            .providers
            .iter_mut()
            .find(|p| p.id == "calendars/private")
            .unwrap()
            .data["version"] = json!("v2");
        assert_data_error(
            &run(&empty(frozen), &p, &changed).unwrap_err(),
            DataErrorKind::Unavailable,
            UiMessage::error_data_query_advanced_definition_schema_requires_explicit_migration(),
        );
    }
}

#[test]
fn provider_projection_never_adds_all_granted_registries_or_unselected_series() {
    let schema = installed();
    let p = plan("nextCivilDay(date('2026-01-01'))");
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    let all = QueryProviderAccess {
        providers: schema.providers.iter().map(|p| p.id.clone()).collect(),
        series_policies: schema
            .series_policies
            .iter()
            .map(|p| p.id.clone())
            .collect(),
    };
    let projected = narrow_query_provider_schema(&schema, &all, &req).unwrap();
    assert_eq!(projected.providers.len(), 1);
    assert!(projected.series_policies.is_empty());
    assert!(projected.validators.is_empty());
    let no = QueryProviderAccess::default();
    assert!(narrow_query_provider_schema(&schema, &no, &req).is_err());
    let (result, _) = run(
        &empty(FrozenAdvancedInput::default()),
        &p,
        &projection(&schema, &access(&req), &req),
    )
    .unwrap();
    assert_eq!(result.coverage, "complete");
}

#[test]
fn provider_preflight_lexical_shadowing_does_not_execute_an_outer_parameter_as_a_row_item() {
    let mut schema = installed();
    let item = schema.definitions["CalendarDate"].clone();
    schema.fields.push(Field {
        id: "tasks/items".into(),
        key: "items".into(),
        shape: Shape::List {
            items: Box::new(item.clone()),
            min: 0,
            max: None,
        },
        nullable: false,
        default: None,
    });
    let mut parameter_shape = item;
    let Shape::Record { constraints, .. } = &mut parameter_shape else {
        panic!("CalendarDate record");
    };
    constraints.clear();
    let mut p = plan("items.map(p, compareTemporal(p, date('2026-01-01'))).size()");
    p.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "p".into(),
            shape: parameter_shape,
            nullable: false,
            default: Some(calendar("v1")),
        }],
        ..Default::default()
    });
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    assert!(!req.providers.contains_key("calendars/private"));
    assert!(req.dynamic_selectors.contains("calendar"));
    let mut restricted = projection(&schema, &access(&req), &req);
    restricted.fields.push(
        schema
            .fields
            .iter()
            .find(|f| f.id == "tasks/items")
            .unwrap()
            .clone(),
    );
    let (result, _) = run(&empty(FrozenAdvancedInput::default()), &p, &restricted).unwrap();
    assert_eq!(result.coverage, "complete");
    p.select[0].expr = "[p].map(item, compareTemporal(item, date('2026-01-01'))).size()".into();
    assert_data_error(
        &run(&empty(FrozenAdvancedInput::default()), &p, &restricted).unwrap_err(),
        DataErrorKind::Unavailable,
        UiMessage::error_data_query_provider_unavailable(),
    );
}

#[test]
fn provider_row_free_preflight_uses_the_original_execution_step_budget() {
    let schema = installed();
    let p = plan(format!("compareTemporal({PRIVATE}, date('2026-01-01'))"));
    let req = query_provider_requirements(&p, &schema, &FrozenAdvancedInput::default())
        .unwrap()
        .remove(0);
    let error = execute_authorized_query_with_provider_dependencies(
        &empty(FrozenAdvancedInput::default()),
        &p,
        &projection(&schema, &access(&req), &req),
        &QueryBudget {
            max_steps: 1,
            ..Default::default()
        },
        &BTreeMap::new(),
    )
    .unwrap_err();
    assert_data_error(
        &error,
        DataErrorKind::Budget,
        UiMessage::error_data_query_host_budget_exhausted(),
    );
}

#[test]
fn provider_saved_call_partial_selector_evidence_is_separate_from_real_execution_arguments() {
    let schema = installed();
    let binding = SavedQueryBinding {
        schema: format!("weftext-config://{WS}/schema"),
        revision: REV.into(),
        language: "weftext.query.cel.v1".into(),
    };
    let mut callee = plan("containsDate(wanted, date('2026-01-01'))");
    callee.advanced = Some(QueryAdvanced {
        parameters: vec![QueryParameter {
            name: "wanted".into(),
            shape: Shape::Use {
                name: "During".into(),
            },
            nullable: false,
            default: None,
        }],
        ..Default::default()
    });
    let source = upsert_saved_query(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: ())) <weftext>\n",
        None,
        KEY,
        "weftext-query-partial",
        &binding,
        &callee,
        &schema,
    )
    .unwrap()
    .source;
    let context = FrozenAdvancedInput {
        definitions: vec![FrozenDefinitionOwner {
            owner: "owner".into(),
            history: vec![DefinitionRevision {
                revision: "original".into(),
                source,
                current: true,
            }],
        }],
        schemas: vec![FrozenDefinitionSchema {
            schema: binding.schema.clone(),
            revision: binding.revision.clone(),
            projection: schema.clone(),
        }],
        ..Default::default()
    };
    let p = QueryPlan {
        advanced: Some(QueryAdvanced {
            call: Some(QueryCall {
                owner: "owner".into(),
                selection: DefinitionSelection::Current { key: KEY.into() },
                arguments: BTreeMap::from([(
                    "wanted".into(),
                    format!("{{'start':date('2025-12-31'),'end':{PRIVATE}}}"),
                )]),
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let requirements = query_provider_requirements(&p, &schema, &context).unwrap();
    let effect = requirements.iter().find(|r| r.baseline.is_some()).unwrap();
    assert!(effect.providers.contains_key("calendars/private"));
    assert!(effect.dynamic_selectors.contains("calendar"));
    assert!(
        narrow_query_provider_schema(
            &schema,
            &QueryProviderAccess {
                providers: BTreeSet::from([rule(&schema)]),
                ..Default::default()
            },
            effect
        )
        .is_err()
    );
    let restricted = projection(&schema, &access(effect), effect);
    let mut frozen = context;
    frozen.schemas[0].projection = restricted.clone();
    let (result, deps) = run(&empty(frozen), &p, &restricted).unwrap();
    assert_eq!(result.coverage, "complete");
    assert!(
        deps.baselines
            .iter()
            .any(|d| d.baseline.is_some() && d.providers.contains_key("calendars/private"))
    );
}
