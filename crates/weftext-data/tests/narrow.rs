use std::collections::BTreeSet;
use weftext_data::*;

fn source(extra: &str, body: &str) -> String {
    format!(
        "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"task\",), status: \"open\"{extra})) <weftext>\r\n{body}\r\n"
    )
}
fn selected() -> BTreeSet<String> {
    BTreeSet::from(["tasks/status".into()])
}

fn assert_narrow_unavailable(error: DataError) {
    assert_eq!(error.kind(), DataErrorKind::Unavailable);
    assert_eq!(
        error.description().explanation,
        weftext_messages::UiMessage::error_data_narrow_narrow_field_profile_unavailable()
    );
}

#[test]
fn narrow_static_field_allows_arbitrary_legal_body_and_hidden_computed_invalid_values() {
    let schema = Schema::bundled().unwrap();
    let one = source(
        ", due: unknown(), aliases: 42, secret: read(\"private.txt\")",
        "#let f(x) = x * 2\n#context query(heading)\n#show heading: it => it\n#f(3)",
    );
    let a = narrow_profile(&one, &schema, &selected(), Some("tasks/status")).unwrap();
    assert_eq!(a.fields.len(), 1);
    assert_eq!(a.fields[0].cell.native().unwrap().text(), Some("open"));
    let delivery = serde_json::to_string(&a.fields).unwrap();
    for secret in [
        "private.txt",
        "unknown",
        "sourceRange",
        "raw",
        "aliases",
        "types",
        "nodeId",
    ] {
        assert!(!delivery.contains(secret), "{secret}: {delivery}");
    }
    let two = source(
        ", due: (\"invalid\",), aliases: \"hidden differs\"",
        "Hidden body of a different length 😀",
    );
    let b = narrow_profile(&two, &schema, &selected(), Some("tasks/status")).unwrap();
    assert_eq!(a.witness, b.witness);
    let patch = patch_narrow_field(
        &two,
        &schema,
        &selected(),
        "tasks/status",
        &NativeValue::Text("done".into()),
    )
    .unwrap();
    assert_eq!(
        patch.source,
        two.replacen("status: \"open\"", "status: \"done\"", 1)
    );
    assert!(
        patch_narrow_field(
            &two,
            &schema,
            &selected(),
            "tasks/status",
            &NativeValue::Text("not-installed".into())
        )
        .is_err()
    );
}

#[test]
fn field_shape_preimage_and_binding_changes_withdraw_old_witness() {
    let schema = Schema::bundled().unwrap();
    let original = source("", "正文");
    let before = narrow_profile(&original, &schema, &selected(), None).unwrap();
    let after = narrow_profile(
        &original.replace("\"open\"", "\"done\""),
        &schema,
        &selected(),
        None,
    )
    .unwrap();
    assert_ne!(before.witness, after.witness);
    for bad in [
        format!("#let metadata(x) = x\n{original}"),
        original.replace("status: \"open\"", "status: \"open\", \"status\": \"done\""),
        original.replace(" <weftext>", " <weftext>\n#metadata(0) <weftext>"),
    ] {
        assert!(narrow_profile(&bad, &schema, &selected(), None).is_err());
    }
    let mut unrelated = schema.clone();
    unrelated
        .fields
        .iter_mut()
        .find(|f| f.id == "core/aliases")
        .unwrap()
        .nullable = true;
    assert_eq!(
        before.witness,
        narrow_profile(&original, &unrelated, &selected(), None)
            .unwrap()
            .witness
    );
    let mut changed = schema;
    changed
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/status")
        .unwrap()
        .nullable = true;
    assert_ne!(
        before.witness,
        narrow_profile(&original, &changed, &selected(), None)
            .unwrap()
            .witness
    );
}

#[test]
fn affected_constraint_requires_authorized_input_closure_before_values() {
    let schema = Schema::bundled().unwrap();
    let source = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"event\",), range: read(\"SECRET_RANGE\"), recurrence: none)) <weftext>\n#context query(heading)";
    let id = schema
        .fields
        .iter()
        .find(|f| f.key == "recurrence")
        .unwrap()
        .id
        .clone();
    let fields = BTreeSet::from([id.clone()]);
    let needed = narrow_write_fields(source, &schema, &id).unwrap();
    assert!(needed.len() > 1);
    assert_narrow_unavailable(
        narrow_profile(source, &schema, &fields, Some(&id))
            .err()
            .unwrap(),
    );
    assert!(narrow_profile(source, &schema, &fields, None).is_ok());
    assert!(patch_narrow_field(source, &schema, &needed, &id, &NativeValue::None).is_err());
}

#[test]
fn reminder_write_closure_is_static_and_required_before_hidden_values() {
    let schema = Schema::bundled().unwrap();
    let fields = BTreeSet::from(["tasks/due".into(), "tasks/reminder".into()]);
    for reminder in [
        "",
        ", reminder: none",
        ", reminder: read(\"SECRET_REMINDER\")",
        ", reminder: (id: \"a0358e3e-5784-4a8b-8e5f-146029b70dbd\", kind: \"absolute\", at: \"2026-10-23T09:00:00Z\")",
    ] {
        let original = source(
            &format!(", due: datetime(year: 2026, month: 10, day: 23){reminder}"),
            "#context query(heading)",
        );
        for selected in ["tasks/due", "tasks/reminder"] {
            assert_eq!(
                narrow_write_fields(&original, &schema, selected).unwrap(),
                fields
            );
            assert_narrow_unavailable(
                narrow_profile(
                    &original,
                    &schema,
                    &BTreeSet::from([selected.into()]),
                    Some(selected),
                )
                .err()
                .unwrap(),
            );
        }
        assert!(
            narrow_profile(
                &original,
                &schema,
                &BTreeSet::from(["tasks/due".into()]),
                None
            )
            .is_ok()
        );
    }
}

fn due_day_source() -> String {
    source(
        ", due: datetime(year: 2026, month: 10, day: 23), reminder: (id: \"a0358e3e-5784-4a8b-8e5f-146029b70dbd\", kind: \"due_day\", clock: (time: datetime(hour: 9, minute: 0, second: 0)), zone: \"UTC\", zone_version: \"fixed-utc.v1\")",
        "#context query(heading)",
    )
}

#[test]
fn reminder_authorized_patch_revalidates_due_and_exact_dependencies() {
    let schema = Schema::bundled().unwrap();
    let original = due_day_source();
    let fields = narrow_write_fields(&original, &schema, "tasks/due").unwrap();
    let before = narrow_profile(&original, &schema, &fields, Some("tasks/due"))
        .unwrap()
        .witness;
    let replacement = decode_native_expression("datetime(year: 2026, month: 10, day: 24)").unwrap();
    let patch = patch_narrow_field(&original, &schema, &fields, "tasks/due", &replacement).unwrap();
    assert_eq!(patch.source, original.replacen("day: 23", "day: 24", 1));
    // A valid due endpoint which cannot supply due_day's complete civil day.
    // This must reach the Type validator, rather than fail the Field shape.
    let incompatible = NativeValue::Text("2026-10-23T09:00:00Z".into());
    let due = schema.fields.iter().find(|f| f.id == "tasks/due").unwrap();
    assert!(schema.validate(&due.shape, &incompatible).is_valid());
    let error = patch_narrow_field(&original, &schema, &fields, "tasks/due", &incompatible)
        .err()
        .unwrap();
    assert_eq!(error.kind(), DataErrorKind::Invalid);
    assert_eq!(
        error.description().explanation,
        UiMessage::error_data_narrow_selected_field_constraint_rejected_proposal()
    );
    assert_ne!(
        before,
        narrow_profile(
            &original.replace("hour: 9", "hour: 10"),
            &schema,
            &fields,
            Some("tasks/due")
        )
        .unwrap()
        .witness
    );
    let mut unrelated = schema.clone();
    unrelated
        .providers
        .iter_mut()
        .find(|p| p.id == "units/rules")
        .unwrap()
        .limits
        .max_expansion = 9999;
    assert_eq!(
        before,
        narrow_profile(&original, &unrelated, &fields, Some("tasks/due"))
            .unwrap()
            .witness
    );
    let mut changed = schema;
    changed
        .providers
        .iter_mut()
        .find(|p| p.data["kind"] == "zone-table" && p.data["key"] == "UTC")
        .unwrap()
        .limits
        .max_steps -= 1;
    assert_ne!(
        before,
        narrow_profile(&original, &changed, &fields, Some("tasks/due"))
            .unwrap()
            .witness
    );
}

#[test]
fn narrow_write_discovery_and_preflight_reject_non_field_or_unsupported_constraints() {
    let schema = Schema::bundled().unwrap();
    let original = due_day_source();
    let fields = narrow_write_fields(&original, &schema, "tasks/due").unwrap();
    let mut unsupported = schema.clone();
    unsupported
        .validators
        .iter_mut()
        .find(|v| v.id == "temporal/task-reminder")
        .unwrap()
        .operation = "owned-resources".into();
    let mut missing = schema.clone();
    missing
        .validators
        .retain(|v| v.id != "temporal/task-reminder");
    let mut schemas = vec![unsupported, missing];
    for selector in [
        Selector::Context {
            name: "owner".into(),
        },
        Selector::Local { path: vec![] },
    ] {
        let mut changed = schema.clone();
        changed
            .types
            .iter_mut()
            .flat_map(|t| &mut t.constraints)
            .find(|c| c.validator == "temporal/task-reminder")
            .unwrap()
            .inputs
            .insert("reminder".into(), selector);
        schemas.push(changed);
    }
    for changed in schemas {
        assert_narrow_unavailable(
            narrow_write_fields(&original, &changed, "tasks/due")
                .err()
                .unwrap(),
        );
        assert_narrow_unavailable(
            narrow_profile(&original, &changed, &fields, Some("tasks/due"))
                .err()
                .unwrap(),
        );
    }
}

#[test]
fn reminder_narrow_patch_keeps_selector_cardinality_validation() {
    let mut schema = Schema::bundled().unwrap();
    let field = schema
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/reminder")
        .unwrap();
    field.shape = Shape::List {
        items: Box::new(field.shape.clone()),
        min: 0,
        max: None,
    };
    schema
        .types
        .iter_mut()
        .flat_map(|t| &mut t.constraints)
        .find(|c| c.validator == "temporal/task-reminder")
        .unwrap()
        .inputs
        .insert(
            "reminder".into(),
            Selector::Field {
                id: "tasks/reminder".into(),
                path: vec![SchemaPath::Each],
            },
        );
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    let reminder = "(id: \"a0358e3e-5784-4a8b-8e5f-146029b70dbd\", kind: \"absolute\", at: \"2026-10-23T09:00:00Z\")";
    let original = source(
        &format!(
            ", due: datetime(year: 2026, month: 10, day: 23), reminder: ({reminder}, {reminder})"
        ),
        "body",
    );
    let fields = narrow_write_fields(&original, &schema, "tasks/due").unwrap();
    let replacement = decode_native_expression("datetime(year: 2026, month: 10, day: 24)").unwrap();
    let error = patch_narrow_field(&original, &schema, &fields, "tasks/due", &replacement)
        .err()
        .unwrap();
    assert_eq!(error.kind(), DataErrorKind::Invalid);
    assert_eq!(
        error.description().explanation,
        UiMessage::error_data_narrow_selected_field_constraint_rejected_proposal()
    );
}

#[test]
fn native_string_injection_is_only_a_selected_literal() {
    let schema = Schema::bundled().unwrap();
    let original = source(", aliases: ()", "中文😀");
    let fields = BTreeSet::from(["core/aliases".into()]);
    let value = decode_native_expression("(\"\\\"); #let metadata(x) = x; <weftext>\",)").unwrap();
    let patch = patch_narrow_field(&original, &schema, &fields, "core/aliases", &value).unwrap();
    let after = narrow_profile(&patch.source, &schema, &fields, None).unwrap();
    assert_eq!(after.fields[0].cell.native(), Some(&value));
    assert!(patch.source.ends_with("中文😀\r\n"));
}

#[test]
fn query_uses_authorized_cells_and_cannot_deserialize_a_projection_capability() {
    let schema = narrow_query_schema(&Schema::bundled().unwrap(), &selected()).unwrap();
    let note = FrozenNote {
        entity_key: "shared".into(),
        path: "resource:shared".into(),
        source: "invalid source #[(".into(),
        revision: "opaque".into(),
        lifecycle: "live".into(),
        title: None,
        body: None,
        products: None,
        authorized_fields: Some(std::collections::BTreeMap::from([(
            "tasks/status".into(),
            Cell::value(NativeValue::Text("done".into())),
        )])),
    };
    let decoded: FrozenNote = serde_json::from_value(serde_json::to_value(&note).unwrap()).unwrap();
    assert!(decoded.authorized_fields.is_none());
    let input = FrozenQueryInput {
        cut_id: "cut".into(),
        schema_revision: "opaque".into(),
        coverage: "complete".into(),
        freshness: "current".into(),
        notes: vec![note],
        advanced: None,
    };
    let plan = QueryPlan {
        select: vec![QueryColumn {
            name: "status".into(),
            expr: "field(\"tasks/status\")".into(),
        }],
        ..Default::default()
    };
    let result = execute_query(&input, &plan, &schema).unwrap();
    assert_eq!(
        result.rows[0].cells["status"].native().unwrap().text(),
        Some("done")
    );
    let limited = execute_query_with_budget(
        &input,
        &plan,
        &schema,
        &QueryBudget {
            max_source_bytes: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(limited.rows.is_empty());
    assert_eq!(limited.coverage, "partial");
    let hidden = QueryPlan {
        select: vec![QueryColumn {
            name: "x".into(),
            expr: "field(\"core/aliases\")".into(),
        }],
        ..Default::default()
    };
    assert!(execute_query(&input, &hidden, &schema).is_err());
}

#[test]
fn referenced_shape_expansion_is_bounded_before_author_values() {
    let mut schema = Schema::bundled().unwrap();
    schema.definitions.insert(
        "wide0".into(),
        Shape::Scalar {
            semantic: "text".into(),
        },
    );
    for i in 1..20 {
        schema.definitions.insert(
            format!("wide{i}"),
            Shape::Union {
                variants: vec![
                    Shape::Use {
                        name: format!("wide{}", i - 1)
                    };
                    2
                ],
                discriminator: None,
            },
        );
    }
    schema
        .fields
        .iter_mut()
        .find(|f| f.id == "tasks/status")
        .unwrap()
        .shape = Shape::Use {
        name: "wide19".into(),
    };
    assert!(
        narrow_profile(
            &source("", "#context query(heading)"),
            &schema,
            &selected(),
            None
        )
        .is_err()
    );
    assert!(narrow_query_schema(&schema, &selected()).is_err());
}

#[test]
fn unbound_namesake_is_absent_without_borrowing_an_unknown_author_value() {
    let schema = Schema::bundled().unwrap();
    let unbound = source("", "body").replace("types: (\"task\",)", "types: ()");
    let profile = narrow_profile(&unbound, &schema, &selected(), None).unwrap();
    assert_eq!(profile.fields[0].cell, Cell::Absent);
    assert!(!profile.fields[0].writable);
    assert!(
        patch_narrow_field(
            &unbound,
            &schema,
            &selected(),
            "tasks/status",
            &NativeValue::Text("done".into())
        )
        .is_err()
    );
}

#[test]
fn canonical_field_id_does_not_borrow_a_hidden_fields_matching_key() {
    let mut schema = Schema::bundled().unwrap();
    schema.fields.insert(
        0,
        Field {
            id: "private/config".into(),
            key: "tasks/status".into(),
            shape: Shape::Scalar {
                semantic: "int".into(),
            },
            nullable: true,
            default: Some(serde_json::json!({"kind":"absent"})),
        },
    );
    // Cross id/key collisions are legal installation syntax; canonical slots
    // must therefore retain the exact selected FieldId at every later lookup.
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    let original = source("", "body");
    let field = &narrow_profile(&original, &schema, &selected(), None)
        .unwrap()
        .fields[0];
    assert_eq!(field.key, "status");
    assert!(!field.nullable);
    let patch = patch_narrow_field(
        &original,
        &schema,
        &selected(),
        "tasks/status",
        &NativeValue::Text("done".into()),
    )
    .unwrap();
    assert_eq!(
        patch.source,
        original.replace("status: \"open\"", "status: \"done\"")
    );
    let unbound = original.replace("types: (\"task\",)", "types: ()");
    let field = &narrow_profile(&unbound, &schema, &selected(), None)
        .unwrap()
        .fields[0];
    assert_eq!(field.key, "status");
    assert!(!field.nullable);
    assert_eq!(field.cell, Cell::Absent);
}

fn private_calendar(schema: &mut Schema) {
    let mut provider = schema
        .providers
        .iter()
        .find(|p| p.implementation.id == "weftext.calendar.gregorian.v2")
        .unwrap()
        .clone();
    provider.id = "calendars/private".into();
    provider.implementation.id = "weftext.calendar.table.v2".into();
    provider.data = serde_json::json!({"kind":"calendar-table","key":"secretcal","version":"v1","epoch_date":"2026-01-01","years":[{"year":1,"months":[{"number":1,"leap":false,"days":30}]},{"year":2,"months":[{"number":1,"leap":false,"days":30}]}]});
    schema.providers.push(provider);
}

fn query_input() -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "cut".into(),
        schema_revision: "opaque".into(),
        coverage: "complete".into(),
        freshness: "current".into(),
        advanced: None,
        notes: vec![FrozenNote {
            entity_key: "shared".into(),
            path: "resource:shared".into(),
            source: String::new(),
            revision: "opaque".into(),
            lifecycle: "live".into(),
            title: None,
            body: None,
            products: None,
            authorized_fields: Some(std::collections::BTreeMap::from([(
                "tasks/status".into(),
                Cell::value(NativeValue::Text("open".into())),
            )])),
        }],
    }
}

#[test]
fn narrow_query_cannot_probe_an_ambient_provider_with_literal_inputs() {
    let mut installed = Schema::bundled().unwrap();
    private_calendar(&mut installed);
    let installed = Schema::from_author_json(&serde_json::to_string(&installed).unwrap()).unwrap();
    let expression = "compareTemporal({'calendar':'secretcal','version':'v1','precision':'day','year':1,'month':1,'day':1},date('2026-01-01'))";
    let mut plan = QueryPlan {
        select: vec![QueryColumn {
            name: "probe".into(),
            expr: expression.into(),
        }],
        ..Default::default()
    };
    // This would expose the private table's epoch as an ordinary numeric result
    // if the hosted projection retained the ambient provider registry.
    assert_eq!(
        execute_query(&query_input(), &plan, &installed)
            .unwrap()
            .rows[0]
            .cells["probe"]
            .native()
            .unwrap()
            .integer(),
        Some(0)
    );
    let restricted = narrow_query_schema(&installed, &selected()).unwrap();
    assert!(restricted.providers.is_empty());
    assert!(restricted.validators.is_empty());
    assert!(execute_query(&query_input(), &plan, &restricted).is_err());
    let mut empty = query_input();
    empty.notes.clear();
    assert!(execute_query(&empty, &plan, &restricted).is_err());
    plan.select[0].expr = "date('2026-01-01') == date('2026-01-01') && status == 'open'".into();
    assert_eq!(
        execute_query(&query_input(), &plan, &restricted)
            .unwrap()
            .rows[0]
            .cells["probe"]
            .native(),
        Some(&NativeValue::Bool(true))
    );
}

#[test]
fn pure_field_witness_ignores_unconsumed_registries_and_registry_order() {
    let mut schema = Schema::bundled().unwrap();
    let original = source("", "body");
    let before = narrow_profile(&original, &schema, &selected(), Some("tasks/status"))
        .unwrap()
        .witness;
    private_calendar(&mut schema);
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "units/rules")
        .unwrap()
        .limits
        .max_expansion = 9999;
    schema.validators.reverse();
    schema.providers.reverse();
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    assert_eq!(
        before,
        narrow_profile(&original, &schema, &selected(), Some("tasks/status"))
            .unwrap()
            .witness
    );
    assert!(
        patch_narrow_field(
            &original,
            &schema,
            &selected(),
            "tasks/status",
            &NativeValue::Text("done".into())
        )
        .is_ok()
    );
}

#[test]
fn authorized_calendar_value_binds_only_its_exact_provider() {
    let mut schema = Schema::bundled().unwrap();
    private_calendar(&mut schema);
    let source = source(
        ", due: (calendar: \"secretcal\", version: \"v1\", precision: \"day\", year: 1, month: 1, day: 1)",
        "body",
    );
    let fields = narrow_write_fields(&source, &schema, "tasks/due").unwrap();
    assert_eq!(
        fields,
        BTreeSet::from(["tasks/due".into(), "tasks/reminder".into()])
    );
    let before = narrow_profile(&source, &schema, &fields, Some("tasks/due"))
        .unwrap()
        .witness;
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "units/rules")
        .unwrap()
        .limits
        .max_expansion = 9999;
    assert_eq!(
        before,
        narrow_profile(&source, &schema, &fields, Some("tasks/due"))
            .unwrap()
            .witness
    );
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "calendars/private")
        .unwrap()
        .data["epoch_date"] = serde_json::json!("2026-02-01");
    assert_ne!(
        before,
        narrow_profile(&source, &schema, &fields, Some("tasks/due"))
            .unwrap()
            .witness
    );
}

#[test]
fn event_authorized_closure_remains_writable_and_binds_its_zone_provider() {
    let mut schema = Schema::bundled().unwrap();
    let source = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"event\",), range: (start: datetime(year: 2026, month: 1, day: 1), end_exclusive: datetime(year: 2026, month: 1, day: 2)), recurrence: (frequency: \"daily\", interval: 1, calendar: \"gregorian\", calendar_version: \"proleptic-gregorian.v1\", zone: \"UTC\", zone_version: \"fixed-utc.v1\", fold: \"earlier\", gap: \"skip\", count: 2))) <weftext>\nbody";
    let fields = narrow_write_fields(source, &schema, "calendar/range").unwrap();
    let before = narrow_profile(source, &schema, &fields, Some("calendar/range"))
        .unwrap()
        .witness;
    let replacement = decode_native_expression("(start: datetime(year: 2026, month: 1, day: 3), end_exclusive: datetime(year: 2026, month: 1, day: 4))").unwrap();
    assert!(patch_narrow_field(source, &schema, &fields, "calendar/range", &replacement).is_ok());
    let zone = schema
        .providers
        .iter_mut()
        .find(|p| p.data["kind"] == "zone-table" && p.data["key"] == "UTC")
        .unwrap();
    zone.limits.max_steps -= 1;
    assert_ne!(
        before,
        narrow_profile(source, &schema, &fields, Some("calendar/range"))
            .unwrap()
            .witness
    );
    // The read-only recurrence dependency can have a key spelling equal to the
    // writable range FieldId. Selecting the range must still patch that range.
    let recurrence = schema
        .fields
        .iter()
        .position(|f| f.id == "calendar/recurrence")
        .unwrap();
    schema.fields[recurrence].key = "calendar/range".into();
    let range = schema
        .fields
        .iter()
        .position(|f| f.id == "calendar/range")
        .unwrap();
    schema.fields.swap(recurrence, range);
    let schema = Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).unwrap();
    let source = source.replace("recurrence:", "\"calendar/range\":");
    let patch =
        patch_narrow_field(&source, &schema, &fields, "calendar/range", &replacement).unwrap();
    assert!(
        patch
            .source
            .contains("\"calendar/range\": (frequency: \"daily\"")
    );
}

#[test]
fn unavailable_calendar_inputs_retain_exact_negative_and_coverage_dependencies() {
    let mut schema = Schema::bundled().unwrap();
    let fields = BTreeSet::from(["tasks/due".into()]);
    let source = source(
        ", due: (calendar: \"secretcal\", version: \"v1\", precision: \"day\", year: 9, month: 1, day: 1)",
        "body",
    );
    let absent = narrow_profile(&source, &schema, &fields, None).unwrap();
    assert!(matches!(absent.fields[0].cell, Cell::Unavailable { .. }));
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "units/rules")
        .unwrap()
        .limits
        .max_expansion -= 1;
    assert_eq!(
        absent.witness,
        narrow_profile(&source, &schema, &fields, None)
            .unwrap()
            .witness
    );
    private_calendar(&mut schema);
    let outside = narrow_profile(&source, &schema, &fields, None).unwrap();
    assert!(matches!(outside.fields[0].cell, Cell::Unavailable { .. }));
    assert_ne!(absent.witness, outside.witness);
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "calendars/private")
        .unwrap()
        .data["epoch_date"] = serde_json::json!("2026-02-01");
    let changed = narrow_profile(&source, &schema, &fields, None).unwrap();
    assert_eq!(outside.fields[0].cell, changed.fields[0].cell);
    assert_ne!(outside.witness, changed.witness);
    let mut duplicate = schema
        .providers
        .iter()
        .find(|p| p.id == "calendars/private")
        .unwrap()
        .clone();
    duplicate.id = "calendars/duplicate".into();
    schema.providers.push(duplicate);
    assert!(Schema::from_author_json(&serde_json::to_string(&schema).unwrap()).is_err());
}

#[test]
fn period_witness_binds_its_series_calendar_and_zone_without_other_series() {
    let mut schema = Schema::bundled().unwrap();
    let source = "#metadata((id: \"32eb9a12-e752-4bfa-8cdf-65e00f23ac26\", types: (\"period-note\",), period: (series_key: \"main\", kind: \"day\", calendar: \"gregorian\", calendar_version: \"proleptic-gregorian.v1\", rule: \"iso8601\", key: \"2026-01-01\", zone: \"UTC\", zone_version: \"fixed-utc.v1\"))) <weftext>\nbody";
    let fields = BTreeSet::from(["calendar/period".into()]);
    let before = narrow_profile(source, &schema, &fields, Some("calendar/period")).unwrap();
    assert!(before.fields[0].cell.native().is_some());
    schema
        .series_policies
        .iter_mut()
        .find(|p| p.key == "work-log")
        .unwrap()
        .multiplicity = "many".into();
    assert_eq!(
        before.witness,
        narrow_profile(source, &schema, &fields, Some("calendar/period"))
            .unwrap()
            .witness
    );
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "zones/utc")
        .unwrap()
        .limits
        .max_steps -= 1;
    assert_ne!(
        before.witness,
        narrow_profile(source, &schema, &fields, Some("calendar/period"))
            .unwrap()
            .witness
    );
    let with_zone = narrow_profile(source, &schema, &fields, Some("calendar/period"))
        .unwrap()
        .witness;
    schema
        .providers
        .iter_mut()
        .find(|p| p.id == "calendars/gregorian")
        .unwrap()
        .limits
        .max_steps -= 1;
    assert_ne!(
        with_zone,
        narrow_profile(source, &schema, &fields, Some("calendar/period"))
            .unwrap()
            .witness
    );
    let with_calendar = narrow_profile(source, &schema, &fields, Some("calendar/period"))
        .unwrap()
        .witness;
    schema
        .series_policies
        .iter_mut()
        .find(|p| p.key == "main")
        .unwrap()
        .multiplicity = "unique".into();
    assert_ne!(
        with_calendar,
        narrow_profile(source, &schema, &fields, Some("calendar/period"))
            .unwrap()
            .witness
    );
}
