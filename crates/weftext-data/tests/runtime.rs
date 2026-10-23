use std::collections::BTreeMap;
use weftext_data::*;
mod support;
use support::assert_data_error;
fn schema() -> Schema {
    Schema::bundled().unwrap()
}
const ID: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";
fn source(fields: &str) -> String {
    format!(
        "#metadata((id: \"{ID}\", types: (\"task\",), status: \"open\"{fields})) <weftext>\n中文 😀 body\r\n"
    )
}
fn input(source: String) -> FrozenQueryInput {
    FrozenQueryInput {
        cut_id: "test-cut".into(),
        schema_revision: "test-schema".into(),
        coverage: "complete".into(),
        freshness: "current".into(),
        advanced: None,
        notes: vec![FrozenNote {
            entity_key: format!("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{ID}"),
            path: "Tasks/One/One.typ".into(),
            source,
            revision: "v1".into(),
            lifecycle: "live".into(),
            title: Some("一项任务".into()),
            body: Some("正文".into()),
            authorized_fields: None,
            products: None,
        }],
    }
}
fn predicate(expr: &str) -> QueryPlan {
    QueryPlan {
        r#where: Some(expr.into()),
        ..Default::default()
    }
}

#[test]
fn official_native_decoder_preserves_order_duplicates_and_exact_values() {
    let v=decode_native_expression("(\"same\", \"same\", (x: none, n: int(\"-9223372036854775808\"), d: decimal(\"0.30\"), f: -0.0, day: datetime(year: 2024, month: 2, day: 29), t: datetime(hour: 12, minute: 30, second: 0)))").unwrap();
    let a = v.array().unwrap();
    assert_eq!(a[0], a[1]);
    let record = &a[2];
    assert_eq!(
        record.member("n"),
        Some(&NativeValue::Int(i64::MIN.to_string()))
    );
    assert_eq!(
        record.member("d"),
        Some(&NativeValue::Decimal("0.30".into()))
    );
    assert_eq!(record.member("f"), Some(&NativeValue::Float("-0".into())));
    assert_eq!(
        record.member("day"),
        Some(&NativeValue::Date("2024-02-29".into()))
    );
    assert_eq!(
        record.member("t"),
        Some(&NativeValue::Time("12:30:00".into()))
    );
    assert!(decode_native_expression("(x: 1, \"x\": 2)").is_err());
    assert!(decode_native_expression("(1, 1 + 1)").is_err());
    assert!(decode_native_expression("datetime(year: 2023, month: 2, day: 29)").is_err());
}
#[test]
fn native_budget_keeps_deep_static_source_unavailable_and_writer_bounded() {
    let expression = format!("{}1{}", "(".repeat(70), ")".repeat(70));
    assert_data_error(
        &decode_native_expression(&expression).unwrap_err(),
        DataErrorKind::NativeBudget,
        UiMessage::error_data_native_decoder_traversal_budget(),
    );
    let projection = project(&source(&format!(", priority: {expression}")), &schema());
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
    let mut value = NativeValue::Int("1".into());
    for _ in 0..70 {
        value = NativeValue::Array(vec![value]);
    }
    assert_data_error(
        &value.to_typst().unwrap_err(),
        DataErrorKind::Budget,
        UiMessage::error_data_native_native_value_traversal_budget_exceeded(),
    );
    assert!(
        !schema()
            .validate(
                &Shape::Scalar {
                    semantic: "int".into()
                },
                &value
            )
            .unavailable
            .is_empty()
    );
    let large = NativeValue::Array(vec![NativeValue::None; 100_001]);
    assert_data_error(
        &large.to_typst().unwrap_err(),
        DataErrorKind::Budget,
        UiMessage::error_data_native_native_value_byte_or_node_budget_exceeded(),
    );
}
#[test]
fn exact_writer_compiles_in_real_typst() {
    let dir = tempfile::TempDir::new().unwrap();
    let engine = weftext_engine::Engine::new().unwrap();
    let values = [
        NativeValue::Int(i64::MIN.to_string()),
        NativeValue::Decimal("0.30".into()),
        NativeValue::Float("-0.0".into()),
        NativeValue::Date("2024-02-29".into()),
        NativeValue::Time("12:30:00".into()),
        NativeValue::LocalDatetime("2024-02-29T12:30:00".into()),
        NativeValue::Duration("90".into()),
        NativeValue::Text("中文😀\n\"#".into()),
    ];
    let expressions = values
        .iter()
        .map(NativeValue::to_typst)
        .collect::<Result<Vec<_>>>()
        .unwrap();
    let author = format!(
        "#let values = ({},)\n#assert.eq(values.at(0), int(\"-9223372036854775808\"))\n#assert.eq(values.at(1), decimal(\"0.3\"))\n#assert.eq(values.at(3), datetime(year: 2024, month: 2, day: 29))\n#assert.eq(values.at(6), duration(seconds: 90))\n正文",
        expressions.join(",")
    );
    let output = engine
        .render(&author, &dir.path().join("values.typ"), dir.path())
        .unwrap();
    assert!(output.ok, "{:?}", output.diagnostics);
}
#[test]
fn full_central_catalog_installs_and_duplicate_json_or_signature_override_reject() {
    let s = schema();
    assert_eq!(s.fields.len(), 56);
    assert_eq!(s.types.len(), 8);
    assert_eq!(s.relations.len(), 28);
    assert_eq!(
        s.fields
            .iter()
            .find(|f| f.id == "tasks/reminder")
            .unwrap()
            .key,
        "reminder"
    );
    let constraint = s
        .types
        .iter()
        .find(|t| t.id == "tasks/task")
        .unwrap()
        .constraints
        .iter()
        .find(|c| c.validator == "temporal/task-reminder")
        .unwrap();
    assert_eq!(constraint.inputs.len(), 2);
    for (slot, field) in [("due", "tasks/due"), ("reminder", "tasks/reminder")] {
        assert!(
            matches!(constraint.inputs.get(slot), Some(Selector::Field { id, path }) if id == field && path.is_empty())
        );
    }
    let json = serde_json::to_string(&s).unwrap();
    assert!(Schema::from_author_json(&json).is_ok());
    assert!(
        Schema::from_author_json(&json.replacen("\"version\":1", "\"version\":1,\"version\":1", 1))
            .is_err()
    );
    let mut value = serde_json::to_value(&s).unwrap();
    value["providers"][0]["exports"] = serde_json::json!([]);
    assert!(Schema::from_author_json(&value.to_string()).is_err());
    assert!(!scalar_valid("node_ref", &NativeValue::Text("weftext-node://58c47256-bfce-1d29-a037-b83a3a644fb0/32eb9a12-e752-4bfa-8cdf-65e00f23ac26".into())));
    assert!(!scalar_valid("tag_ref", &NativeValue::Text("weftext-tag://58c47256-bfce-4d29-a037-b83a3a644fb0/32eb9a12-e752-1bfa-8cdf-65e00f23ac26".into())));
    let mut value = serde_json::to_value(&s).unwrap();
    value["fields"][0]["shape"] = serde_json::json!({"kind":"scalar","semantic":"arbitrary_code"});
    assert!(Schema::from_author_json(&value.to_string()).is_err());
}
#[test]
fn independent_static_field_computed_and_unknown_preserve() {
    let source = source(", priority: 1 + 1, extra: (k: \"raw\")");
    let p = project(&source, &schema());
    assert_eq!(p.qualification, "qualified");
    assert!(matches!(
        p.fields.iter().find(|f| f.key == "priority").unwrap().cell,
        Cell::Unavailable { .. }
    ));
    assert!(matches!(
        p.fields.iter().find(|f| f.key == "status").unwrap().cell,
        Cell::Value { .. }
    ));
    assert_eq!(p.unknown_fields[0].raw, "(k: \"raw\")");
    assert!(!typed_ref_closure(&source, &schema()).unwrap().complete);
}
#[test]
fn root_binding_and_control_are_never_guessed() {
    let source = source("");
    assert_eq!(
        project(&format!("#let metadata = x => x\n{source}"), &schema()).qualification,
        "unavailable"
    );
    assert_eq!(
        project(&format!("{source}\n{source}"), &schema()).qualification,
        "unavailable"
    );
    assert_eq!(
        project(&source.replace("id: ", "\"id\": \"bad\", id: "), &schema()).qualification,
        "unavailable"
    );
    assert_eq!(
        project(
            &source.replace("types: (\"task\",)", "types: (\"task\", \"tasks/task\")"),
            &schema()
        )
        .diagnostics
        .len(),
        1
    );
}
#[test]
fn exact_narrow_patch_retains_every_untouched_byte() {
    let original = source(", due: datetime(year: 2026, month: 10, day: 13), mystery: \"保持\"");
    let patch = patch_field(
        &original,
        &schema(),
        "status",
        &[],
        &NativeValue::Text("done".into()),
    )
    .unwrap();
    assert_eq!(&patch.source[..patch.from], &original[..patch.from]);
    assert_eq!(
        &patch.source[patch.from + patch.text.len()..],
        &original[patch.to..]
    );
    assert!(patch.source.contains("mystery: \"保持\""));
    assert!(patch.source.ends_with("\r\n"));
    assert!(
        patch_field(
            &original,
            &schema(),
            "status",
            &[],
            &NativeValue::Text("todo".into())
        )
        .is_err()
    );
    let patch = patch_field(
        &original,
        &schema(),
        "priority",
        &[],
        &NativeValue::Text("high".into()),
    );
    assert!(patch.is_ok(), "{patch:?}");
}
#[test]
fn rich_person_same_item_and_order_use_actual_cel() {
    let source = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), phones: ((number: \"138\", label: \"personal\"), (number: \"010\", label: \"work\"), (number: \"138\", label: \"work\"), (number: \"138\", label: \"work\")))) <weftext>"
    );
    let query = QueryPlan {
        r#where: Some(
            "phones.exists(p, p.number == '138' && has(p.label) && p.label == 'work')".into(),
        ),
        select: vec![QueryColumn {
            name: "contacts".into(),
            expr: "phones.filter(p, p.number == '138')".into(),
        }],
        ..Default::default()
    };
    let result = execute_query(&input(source.clone()), &query, &schema()).unwrap();
    assert_eq!(result.coverage, "complete");
    assert_eq!(result.rows.len(), 1);
    let a = result.rows[0].cells["contacts"]
        .native()
        .unwrap()
        .array()
        .unwrap();
    assert_eq!(a.len(), 3);
    assert_eq!(a[0].member("label").unwrap().text(), Some("personal"));
    assert_eq!(a[1], a[2]);
    let cross = source.replace(
        ", (number: \"138\", label: \"work\"), (number: \"138\", label: \"work\")",
        "",
    );
    assert_eq!(
        execute_query(&input(cross), &query, &schema())
            .unwrap()
            .rows
            .len(),
        0
    );
}
#[test]
fn cel_error_absorption_operand_swaps_macros_and_conditional() {
    let s = schema();
    let i = input(source(""));
    let cases = [
        ("false && (1 / 0 == 0)", 0, "complete"),
        ("(1 / 0 == 0) && false", 0, "complete"),
        ("true || (1 / 0 == 0)", 1, "complete"),
        ("(1 / 0 == 0) || true", 1, "complete"),
        ("true && (1 / 0 == 0)", 0, "partial"),
        ("false || (1 / 0 == 0)", 0, "partial"),
        ("[1, 0].exists(x, 1 / x > 0)", 1, "complete"),
        ("[0, 1].exists(x, 1 / x > 0)", 1, "complete"),
        ("[0, 1].all(x, 1 / x < 0)", 0, "complete"),
        ("[1, 0].all(x, 1 / x < 0)", 0, "complete"),
        ("[1, 0].filter(x, 1 / x > 0).size() == 1", 0, "partial"),
        ("[1, 0].map(x, 1 / x).size() == 2", 0, "partial"),
        ("false ? (1 / 0 == 0) : true", 1, "complete"),
    ];
    for (expr, rows, coverage) in cases {
        let result =
            execute_query(&i, &predicate(expr), &s).unwrap_or_else(|e| panic!("{expr}: {e}"));
        assert_eq!(
            (result.rows.len(), result.coverage.as_str()),
            (rows, coverage),
            "{expr}"
        );
    }
}
#[test]
fn cells_no_false_empty_and_host_budget_cannot_be_absorbed() {
    let s = schema();
    let i = input(source(""));
    let result = execute_query(&i, &predicate("due == date('2026-10-13')"), &s).unwrap();
    assert_eq!(result.coverage, "partial");
    assert_eq!(
        execute_query(
            &i,
            &predicate("isAbsent(due) ? false : due == date('2026-10-13')"),
            &s
        )
        .unwrap()
        .coverage,
        "complete"
    );
    assert!(execute_query(&i, &predicate("status == 1"), &s).is_err());
    assert!(execute_query(&i, &predicate("1 == 1.0"), &s).is_err());
    let limits = QueryBudget {
        max_steps: 1,
        ..Default::default()
    };
    let result = execute_query_with_budget(
        &i,
        &predicate("[1,2,3,4].map(x, x*x).size() == 4 || true"),
        &s,
        &limits,
    )
    .unwrap();
    assert_eq!(result.coverage, "partial");
    assert!(result.rows.is_empty());
}
#[test]
fn typed_decimal_precision_and_civil_date_no_host_zone() {
    let mut s = schema();
    s.fields.push(Field {
        id: "test/amount".into(),
        key: "amount".into(),
        shape: Shape::Scalar {
            semantic: "decimal".into(),
        },
        nullable: false,
        default: None,
    });
    s.types
        .iter_mut()
        .find(|t| t.id == "tasks/task")
        .unwrap()
        .fields
        .push(FieldContribution {
            id: "test/amount".into(),
            required: false,
        });
    let i = input(source(
        ", amount: decimal(\"9007199254740993.01\"), due: datetime(year: 2026, month: 10, day: 13)",
    ));
    assert_eq!(
        execute_query(
            &i,
            &predicate("amount > decimal('9007199254740993.00') && due == date('2026-10-13')"),
            &s
        )
        .unwrap()
        .rows
        .len(),
        1
    );
    assert!(execute_query(&i, &predicate("amount > 9007199254740993.0"), &s).is_err());
}
#[test]
fn source_scope_frozen_order_take_group_and_readonly_trash() {
    let mut i = input(source(""));
    let mut second = i.notes[0].clone();
    second.entity_key.push('b');
    second.title = Some("乙".into());
    i.notes.push(second);
    let p = QueryPlan {
        order: vec![QueryOrder {
            expr: "title".into(),
            descending: true,
            nulls: "last".into(),
        }],
        take: Some(1),
        ..Default::default()
    };
    let r = execute_query(&i, &p, &schema()).unwrap();
    assert_eq!(r.total_known, 2);
    assert!(r.limited);
    assert_eq!(r.coverage, "complete");
    let p = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![QueryColumn {
                name: "status".into(),
                expr: "status".into(),
            }],
            aggregates: vec![QueryAggregate {
                name: "count".into(),
                op: "count".into(),
                expr: None,
            }],
        }),
        ..Default::default()
    };
    let r = execute_query(&i, &p, &schema()).unwrap();
    assert_eq!(r.rows.len(), 1);
    assert!(r.rows[0].read_only);
    assert!(r.rows[0].entity_key.is_none());
    assert_eq!(
        r.rows[0].cells["count"].native(),
        Some(&NativeValue::Int("2".into()))
    );
    i.coverage = "partial".into();
    i.freshness = "stale".into();
    assert!(!execute_query(&i, &p, &schema()).unwrap().aggregate_exact);
    i.notes[0].lifecycle = "trash".into();
    let p = QueryPlan {
        scope: QueryScope {
            lifecycle: "trash".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let r = execute_query(&i, &p, &schema()).unwrap();
    assert!(r.rows[0].read_only);
    assert_eq!(r.freshness, "stale");
}
#[test]
fn typed_ref_slots_only_schema_paths_and_unknown_blocks_complete_copy() {
    let target =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let source = source(&format!(", project: \"{target}\""));
    let slots = typed_ref_slots(&source, &schema()).unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(&source[slots[0].from..slots[0].to], format!("\"{target}\""));
    let bad = source.replace("status: \"open\"", "status: \"bogus\"");
    assert!(typed_ref_slots(&bad, &schema()).is_err());
}
#[test]
fn parenthesized_business_frames_preserve_bytes_but_controls_are_direct_literals() {
    let target =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let source = source(&format!(
        ", project: (/* reference */ \"{target}\"), dependencies: (/* list */ (((target: \"{target}\", status: \"blocks\", extra: decimal(\"0.30\"))),))"
    ));
    let closure = typed_ref_closure(&source, &schema()).unwrap();
    assert!(!closure.complete); // Unknown members may hide an undeclared ref.
    assert_eq!(closure.slots.len(), 2);
    for slot in closure.slots {
        assert_eq!(&source[slot.from..slot.to], format!("\"{target}\""));
    }
    let projection = project(&source, &schema());
    let before = projection
        .fields
        .iter()
        .find(|f| f.key == "dependencies")
        .unwrap()
        .cell
        .native()
        .unwrap();
    let mut after = before.clone();
    let NativeValue::Array(items) = &mut after else {
        panic!()
    };
    let NativeValue::Record(members) = &mut items[0] else {
        panic!()
    };
    members
        .iter_mut()
        .find(|m| m.key == "status")
        .unwrap()
        .value = NativeValue::Text("relates".into());
    let patch = patch_field(&source, &schema(), "dependencies", &[], &after).unwrap();
    assert_eq!(patch.source, source.replace("\"blocks\"", "\"relates\""));
    assert_eq!(
        project(
            &source.replace(&format!("id: \"{ID}\""), &format!("id: (\"{ID}\")")),
            &schema()
        )
        .qualification,
        "invalid-control"
    );
    assert_eq!(
        project(
            &source.replace("types: (\"task\",)", "types: ((\"task\",))"),
            &schema()
        )
        .qualification,
        "invalid-control"
    );
}
#[test]
fn search_dictionary_uses_installed_permanent_name_ids_and_reports_incomplete_values() {
    let mut schema = schema();
    schema
        .fields
        .iter_mut()
        .find(|f| f.id == "core/aliases")
        .unwrap()
        .key = "other_names".into();
    let person = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), other_names: (\"白云山\", \"山\", \"白云山\"), names: ((text: \"白云山2026\", role: \"ordinary\"), (text: \"山\", role: \"former\"), (text: \"白雲山\", role: \"alias\")), arbitrary: \"不可扫描\")) <weftext>"
    );
    let terms = search_terms(&person, &schema);
    assert!(terms.complete, "{:?}", terms.reasons);
    assert_eq!(terms.terms, ["白云山", "山", "白云山2026", "白雲山"]);
    let computed = person.replace(
        "other_names: (\"白云山\", \"山\", \"白云山\")",
        "other_names: (\"白云山\",) + (\"山\",)",
    );
    let terms = search_terms(&computed, &schema);
    assert!(!terms.complete);
    assert_eq!(terms.terms, ["白云山2026", "山", "白雲山"]);
    assert!(!terms.reasons.is_empty());
    let wrong = person.replace("role: \"ordinary\"", "role: 1");
    let terms = search_terms(&wrong, &schema);
    assert!(!terms.complete);
    assert_eq!(terms.terms, ["白云山", "山"]);
}
#[test]
fn real_set_title_product_retains_metadata_names_and_exact_field_writes() {
    let schema = schema();
    let original = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), names: ((text: \"白云山2026\", role: \"ordinary\"),), aliases: (\"白云山\", \"山\",))) <weftext>\n正文 😀\r\n"
    );
    let before = project(&original, &schema);
    assert_eq!(before.qualification, "qualified");
    let titled = weftext_engine::edit(
        &original,
        &weftext_engine::EditRequest {
            source_hash: weftext_engine::source_hash(&original),
            intent: weftext_engine::EditIntent::SetTitle {
                text: Some("白云山".into()),
            },
        },
    )
    .unwrap();
    assert!(titled.source.contains("#title()"));
    assert_eq!(titled.map.title.text.as_deref(), Some("白云山"));
    let after = project(&titled.source, &schema);
    assert_eq!(after.qualification, "qualified", "{:?}", after.diagnostics);
    assert_eq!(after.id, before.id);
    assert_eq!(after.types, before.types);
    for field in before.fields {
        let actual = after
            .fields
            .iter()
            .find(|f| f.field_id == field.field_id)
            .unwrap();
        assert_eq!(actual.cell, field.cell);
        assert_eq!(actual.writable, field.writable);
    }
    let terms = search_terms(&titled.source, &schema);
    assert!(terms.complete, "{:?}", terms.reasons);
    assert_eq!(terms.terms, ["白云山", "山", "白云山2026"]);
    let replacement = NativeValue::Array(vec![
        NativeValue::Text("白云山".into()),
        NativeValue::Text("新别名".into()),
    ]);
    let patch = patch_field(&titled.source, &schema, "core/aliases", &[], &replacement).unwrap();
    assert_eq!(&patch.source[..patch.from], &titled.source[..patch.from]);
    assert_eq!(
        &patch.source[patch.from + patch.text.len()..],
        &titled.source[patch.to..]
    );
    assert_eq!(
        project(&patch.source, &schema)
            .fields
            .iter()
            .find(|f| f.field_id == "core/aliases")
            .unwrap()
            .cell
            .native(),
        Some(&replacement)
    );
    assert_eq!(
        weftext_engine::inspect_editor(&patch.source)
            .title
            .text
            .as_deref(),
        Some("白云山")
    );
    let dir = tempfile::TempDir::new().unwrap();
    let rendered = weftext_engine::Engine::new()
        .unwrap()
        .render(&patch.source, &dir.path().join("title.typ"), dir.path())
        .unwrap();
    assert!(rendered.ok, "{:?}", rendered.diagnostics);
    assert!(rendered.html.contains("白云山"));
    assert_eq!(
        rendered.products.as_ref().unwrap().title.as_deref(),
        Some("白云山")
    );
    assert_eq!(
        weftext_engine::inspect_metadata(&patch.source)
            .id
            .as_deref(),
        Some(ID)
    );
}
#[test]
fn capture_is_identity_free_validated_and_defaults_never_added_on_read() {
    let fields = BTreeMap::from([("status".into(), NativeValue::Text("open".into()))]);
    let template = capture_template(&["task".into()], &fields, "正文", &schema()).unwrap();
    assert!(!template.contains("id:"));
    assert!(template.contains("types:"));
    assert!(capture_template(&["task".into()], &BTreeMap::new(), "", &schema()).is_err());
    assert!(matches!(
        project(&source(""), &schema())
            .fields
            .iter()
            .find(|f| f.key == "due")
            .unwrap()
            .cell,
        Cell::Absent
    ));
}
#[test]
fn saved_query_direct_role_exact_current_and_pinned_no_latest_fallback() {
    let source = include_str!("../../../docs/design/examples/native-cases/saved-query.typ");
    let defs = saved_queries(source).unwrap();
    assert_eq!(defs.len(), 1);
    let key = defs[0].key.clone();
    let history = vec![
        DefinitionRevision {
            revision: "old".into(),
            source: source.into(),
            current: false,
        },
        DefinitionRevision {
            revision: "new".into(),
            source: source.replace("p.label == 'work'", "p.label == 'home'"),
            current: true,
        },
    ];
    assert!(
        select_definition(
            &DefinitionSelection::Pinned {
                key: key.clone(),
                revision: "gone".into()
            },
            &history
        )
        .is_err()
    );
    let pinned = select_definition(
        &DefinitionSelection::Pinned {
            key: key.clone(),
            revision: "old".into(),
        },
        &history,
    )
    .unwrap();
    assert!(pinned.plan.r#where.unwrap().contains("'work'"));
    let current = select_definition(&DefinitionSelection::Current { key }, &history).unwrap();
    assert!(current.plan.r#where.unwrap().contains("'home'"));
}
#[test]
fn saved_query_writer_preserves_root_identity_and_exact_other_source() {
    let original = source("");
    let key = "6b2b9e19-5599-48c3-b27b-6b006ec9c397";
    let binding = SavedQueryBinding {
        schema: "weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema".into(),
        revision: "8a81e35f-ad48-4c12-af4f-3d2607102fa9".into(),
        language: "weftext.query.cel.v1".into(),
    };
    let plan = predicate("status == 'open'");
    let patch = upsert_saved_query(
        &original,
        None,
        key,
        "weftext-query-tasks",
        &binding,
        &plan,
        &schema(),
    )
    .unwrap();
    assert_eq!(
        weftext_engine::inspect_metadata(&patch.source)
            .id
            .as_deref(),
        Some(ID)
    );
    assert_eq!(project(&patch.source, &schema()).id.as_deref(), Some(ID));
    assert_eq!(&patch.source[..patch.from], &original[..patch.from]);
    assert_eq!(
        &patch.source[patch.from + patch.text.len()..],
        &original[patch.to..]
    );
    let update = upsert_saved_query(
        &patch.source,
        Some(key),
        key,
        "weftext-query-done",
        &binding,
        &predicate("status == 'done'"),
        &schema(),
    )
    .unwrap();
    let defs = saved_queries(&update.source).unwrap();
    assert_eq!(defs.len(), 1);
    assert_eq!(defs[0].key, key);
    assert_eq!(defs[0].label, "weftext-query-done");
    assert_eq!(
        weftext_engine::inspect_metadata(&update.source)
            .id
            .as_deref(),
        Some(ID)
    );
    let computed = source(", priority: 1 + 1");
    assert!(
        upsert_saved_query(
            &computed,
            None,
            key,
            "weftext-query-tasks",
            &binding,
            &plan,
            &schema()
        )
        .is_err()
    );
}
#[test]
fn cel_locals_do_not_rebind_to_namesake_schema_fields_and_member_order_survives() {
    let mut i = input(source(""));
    let source = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), phones: ((note: \"first\", number: \"138\", label: \"work\"),))) <weftext>"
    );
    i.notes[0].source = source;
    let p = QueryPlan {
        select: vec![QueryColumn {
            name: "v".into(),
            expr: "phones.filter(status, status.label == 'work')".into(),
        }],
        ..Default::default()
    };
    let result = execute_query(&i, &p, &schema()).unwrap();
    let record = &result.rows[0].cells["v"].native().unwrap().array().unwrap()[0];
    assert_eq!(
        record
            .record()
            .unwrap()
            .iter()
            .map(|m| m.key.as_str())
            .collect::<Vec<_>>(),
        vec!["note", "number", "label"]
    );
    assert!(
        execute_query(
            &i,
            &predicate("phones.exists(status, isAbsent(status))"),
            &schema()
        )
        .is_err()
    );
    assert_eq!(
        execute_query(
            &i,
            &predicate("has({}.label) ? {}.label == 'work' : false"),
            &schema()
        )
        .unwrap()
        .coverage,
        "complete"
    );
}
fn period(kind: &str, key: &str) -> NativeValue {
    decode_native_expression(&format!("(series_key: \"main\", kind: \"{kind}\", calendar: \"gregorian\", calendar_version: \"proleptic-gregorian.v1\", rule: \"iso8601\", key: \"{key}\", zone: \"UTC\", zone_version: \"fixed-utc.v1\")")).unwrap()
}
#[test]
fn gregorian_period_executor_checks_all_five_kinds_versions_and_week53() {
    let s = schema();
    for (kind, key, start, end) in [
        ("day", "2024-02-29", "2024-02-29", "2024-03-01"),
        ("week", "2020-W53", "2020-12-28", "2021-01-04"),
        ("month", "2026-12", "2026-12-01", "2027-01-01"),
        ("quarter", "2026-Q4", "2026-10-01", "2027-01-01"),
        ("year", "2026", "2026-01-01", "2027-01-01"),
    ] {
        let value = period(kind, key);
        let range = period_bounds(&value, &s).unwrap();
        assert_eq!(
            range.member("start"),
            Some(&NativeValue::Date(start.into()))
        );
        assert_eq!(
            range.member("end_exclusive"),
            Some(&NativeValue::Date(end.into()))
        );
        assert!(
            s.validate(
                &Shape::Use {
                    name: "Period".into()
                },
                &value
            )
            .is_valid()
        );
    }
    assert!(period_bounds(&period("week", "2021-W53"), &s).is_err());
    assert!(period_bounds(&period("day", "2023-02-29"), &s).is_err());
    assert_eq!(
        period_step(&period("month", "2026-12"), 1, &s)
            .unwrap()
            .member("key")
            .unwrap()
            .text(),
        Some("2027-01")
    );
    assert_eq!(
        period_step(&period("week", "2020-W53"), 1, &s)
            .unwrap()
            .member("key")
            .unwrap()
            .text(),
        Some("2021-W01")
    );
}
#[test]
fn offset_instants_compare_exactly_beyond_nanoseconds() {
    let a = ExactInstant::parse("2026-10-13T12:00:00.123456789123+08:00").unwrap();
    let b = ExactInstant::parse("2026-10-13T04:00:00.123456789124Z").unwrap();
    assert!(a.compare(&b).is_lt());
    let c = ExactInstant::parse("2026-10-13T04:00:00.123456789123Z").unwrap();
    assert!(a.compare(&c).is_eq());
    for s in [
        "2026-10-13T04:00:00-00:00",
        "2026-10-13T04:00:60Z",
        "2026-10-13T04:00:00",
        "2026-10-13T04:00:00+24:00",
    ] {
        assert!(ExactInstant::parse(s).is_err(), "{s}");
    }
}
#[test]
fn canonical_decimal_grouping_and_empty_global_count() {
    let mut s = schema();
    s.fields.push(Field {
        id: "test/amount".into(),
        key: "amount".into(),
        shape: Shape::Scalar {
            semantic: "decimal".into(),
        },
        nullable: false,
        default: Some(serde_json::json!({"kind":"absent"})),
    });
    s.types
        .iter_mut()
        .find(|t| t.id == "tasks/task")
        .unwrap()
        .fields
        .push(FieldContribution {
            id: "test/amount".into(),
            required: false,
        });
    let mut i = input(source(", amount: decimal(\"0.30\")"));
    let mut second = i.notes[0].clone();
    second.entity_key.push('b');
    second.source = source(", amount: decimal(\"0.3\")");
    i.notes.push(second);
    let p = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![QueryColumn {
                name: "amount".into(),
                expr: "amount".into(),
            }],
            aggregates: vec![QueryAggregate {
                name: "count".into(),
                op: "count".into(),
                expr: None,
            }],
        }),
        ..Default::default()
    };
    assert_eq!(execute_query(&i, &p, &s).unwrap().rows.len(), 1);
    let p = QueryPlan {
        group: Some(QueryGroup {
            keys: vec![],
            aggregates: vec![QueryAggregate {
                name: "count".into(),
                op: "count".into(),
                expr: None,
            }],
        }),
        ..Default::default()
    };
    i.notes.clear();
    let result = execute_query(&i, &p, &s).unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].cells["count"].native(),
        Some(&NativeValue::Int("0".into()))
    );
    assert!(result.aggregate_exact);
}
#[test]
fn smart_form_writer_preserves_unknown_raw_literals_comments_and_repeated_array_items() {
    let original = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), phones: ((number: \"138\", /* 原注释 */ label: \"work\", mystery: decimal(\"0.300\")), (number: \"138\", label: \"work\"),))) <weftext>\n正文"
    );
    let s = schema();
    let projection = project(&original, &s);
    let field = projection
        .fields
        .iter()
        .find(|f| f.key == "phones")
        .unwrap();
    let mut value = field.cell.native().unwrap().clone();
    let NativeValue::Array(ref mut a) = value else {
        unreachable!()
    };
    let NativeValue::Record(m) = &mut a[0] else {
        unreachable!()
    };
    m.iter_mut().find(|m| m.key == "label").unwrap().value = NativeValue::Text("personal".into());
    let patch = patch_field(&original, &s, "phones", &[], &value).unwrap();
    assert!(patch.source.contains("/* 原注释 */"));
    assert!(patch.source.contains("mystery: decimal(\"0.300\")"));
    assert_eq!(
        patch.source,
        original.replacen("label: \"work\"", "label: \"personal\"", 1)
    );
    assert!(
        patch_field(
            &original,
            &s,
            "phones",
            &[
                ValuePath::Index { index: 0 },
                ValuePath::Member {
                    key: "mystery".into()
                }
            ],
            &NativeValue::Decimal("0.4".into())
        )
        .is_err()
    );
    let mut added = field.cell.native().unwrap().clone();
    let NativeValue::Array(ref mut a) = added else {
        unreachable!()
    };
    a.push(decode_native_expression("(number: \"010\", label: \"personal\")").unwrap());
    let patch = patch_field(&original, &s, "phones", &[], &added).unwrap();
    assert!(patch.source.contains("/* 原注释 */"));
    assert!(patch.source.contains("mystery: decimal(\"0.300\")"));
    let mut removed = field.cell.native().unwrap().clone();
    let NativeValue::Array(ref mut a) = removed else {
        unreachable!()
    };
    a.remove(1);
    let patch = patch_field(&original, &s, "phones", &[], &removed).unwrap();
    assert!(patch.source.contains("mystery: decimal(\"0.300\")"));
    assert!(patch.source.contains("/* 原注释 */"));
}
#[test]
fn relation_admission_requires_real_typed_live_target_and_preserves_existing_dangling() {
    let s = schema();
    let target =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let before = source("");
    let after = source(&format!(", dependencies: ((target: \"{target}\"),)"));
    let target = RelationTarget {
        entity_key: target.into(),
        types: vec!["task".into()],
        lifecycle: "live".into(),
    };
    assert!(
        validate_relation_change(&before, &after, &s, std::slice::from_ref(&target), true)
            .is_valid()
    );
    let wrong = RelationTarget {
        types: vec!["person".into()],
        ..target
    };
    assert!(!validate_relation_change(&before, &after, &s, &[wrong], true).is_valid());
    assert!(
        !validate_relation_change(&before, &after, &s, &[], false)
            .unavailable
            .is_empty()
    );
    assert!(validate_relation_change(&after, &after, &s, &[], true).is_valid());
}
#[test]
fn temporal_provider_precision_and_common_basis_run_in_actual_cel() {
    let s = schema();
    let month=decode_native_expression("(calendar: \"gregorian\", version: \"proleptic-gregorian.v1\", precision: \"month\", year: 2026, month: 3)").unwrap();
    let end = NativeValue::Date("2026-04-15".into());
    let during = NativeValue::Record(vec![
        NativeMember {
            key: "start".into(),
            value: month.clone(),
        },
        NativeMember {
            key: "end".into(),
            value: end,
        },
    ]);
    assert!(matches!(
        contains_date(&during, &NativeValue::Date("2026-03-15".into()), &s),
        Cell::Unavailable { .. }
    ));
    assert_eq!(
        contains_date(&during, &NativeValue::Date("2026-04-01".into()), &s),
        Cell::value(NativeValue::Bool(true))
    );
    assert_eq!(
        contains_date(&during, &NativeValue::Date("2026-02-01".into()), &s),
        Cell::value(NativeValue::Bool(false))
    );
    let open = decode_native_expression("(start: none, end: none)").unwrap();
    assert_eq!(
        contains_date(&open, &month, &s),
        Cell::value(NativeValue::Bool(true))
    );
    assert!(matches!(
        contains_date(&open, &NativeValue::Bool(false), &s),
        Cell::Invalid { .. }
    ));
    assert!(matches!(
        contains_date(&NativeValue::Record(vec![]), &month, &s),
        Cell::Unavailable { .. }
    ));
    let civil =
        decode_native_expression("(start: datetime(year: 2026, month: 1, day: 1))").unwrap();
    let instant = decode_native_expression("(start: \"2026-01-01T00:00:00Z\")").unwrap();
    assert!(matches!(
        range_overlap(&civil, &instant, &s),
        Cell::Invalid { .. }
    ));
    let input = input(source(""));
    let q = predicate(
        "containsDate({'start': date('2026-10-01'), 'end': date('2026-10-31')}, date('2026-10-13')) && nextCivilDay(date('2026-10-13')) == date('2026-10-14')",
    );
    let r = execute_query(&input, &q, &s).unwrap();
    assert_eq!(r.coverage, "complete");
    assert_eq!(r.rows.len(), 1);
    let q = predicate(
        "rangeOverlap({'start': date('2026-10-01')}, {'start': instant('2026-10-01T00:00:00Z')}) || true",
    );
    assert_eq!(execute_query(&input, &q, &s).unwrap().rows.len(), 1);
}
#[test]
fn field_closure_validation_uses_frozen_resource_owner_and_keeps_unrelated_computed() {
    let s = schema();
    let before = source(", priority: 1 + 1");
    let after = before.replace("status: \"open\"", "status: \"done\"");
    assert!(
        validate_field_change(
            &before,
            &after,
            &s,
            "tasks/status",
            &ValidationContext::default()
        )
        .is_valid()
    );
    assert!(
        !validate_note(&after, &s, &ValidationContext::default())
            .unavailable
            .is_empty()
    );
    let reference = "weftext-resource://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let person = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), avatar: (resource: \"{reference}\"))) <weftext>"
    );
    let owner = format!("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{ID}");
    let context = ValidationContext {
        owner: Some(owner.clone()),
        resources: vec![ResourceMetadata {
            resource_ref: reference.into(),
            owner,
            lifecycle: "live".into(),
        }],
        resources_complete: true,
    };
    assert!(validate_note(&person, &s, &context).is_valid());
    let mut wrong = context.clone();
    wrong.resources[0].owner.push('x');
    assert!(!validate_note(&person, &s, &wrong).diagnostics.is_empty());
    assert!(
        !validate_note(&person, &s, &ValidationContext::default())
            .unavailable
            .is_empty()
    );
}
#[test]
fn required_absence_and_creation_defaults_do_not_share_read_semantics() {
    let mut s = schema();
    s.fields
        .iter_mut()
        .find(|f| f.id == "tasks/status")
        .unwrap()
        .default = Some(serde_json::json!({"kind":"native","source":"\"open\""}));
    let source = format!("#metadata((id: \"{ID}\", types: (\"task\",))) <weftext>");
    let p = project(&source, &s);
    assert!(matches!(
        p.fields.iter().find(|f| f.key == "status").unwrap().cell,
        Cell::Absent
    ));
    assert!(!validate_note(&source, &s, &ValidationContext::default()).is_valid());
    let created = capture_source(ID, &["task".into()], &BTreeMap::new(), "正文", &s).unwrap();
    assert_eq!(
        project(&created, &s)
            .fields
            .iter()
            .find(|f| f.key == "status")
            .unwrap()
            .cell
            .native()
            .unwrap()
            .text(),
        Some("open")
    );
    let fields = BTreeMap::from([
        ("status".into(), NativeValue::Text("open".into())),
        ("tasks/status".into(), NativeValue::Text("done".into())),
    ]);
    assert!(capture_template(&["task".into()], &fields, "", &s).is_err());
}
#[test]
fn configured_relation_policies_and_directed_cycles_use_exact_frozen_closure() {
    let mut s = schema();
    s.relations
        .iter_mut()
        .find(|r| r.id == "tasks/dependency")
        .unwrap()
        .cycle_policy = "reject".into();
    let s = Schema::from_author_json(&serde_json::to_string(&s).unwrap()).unwrap();
    let owner = format!("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{ID}");
    let other_id = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let other = format!("weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/{other_id}");
    let before = source("");
    let after = source(&format!(", dependencies: ((target: \"{other}\"),)"));
    let target = RelationTarget {
        entity_key: other.clone(),
        types: vec!["task".into()],
        lifecycle: "live".into(),
    };
    let reverse = RelationSource {
        entity_key: other.clone(),
        source: source(&format!(", dependencies: ((target: \"{owner}\"),)"))
            .replacen(ID, other_id, 1),
        lifecycle: "live".into(),
    };
    let cycle = validate_relation_change_with_graph(
        &owner,
        &before,
        &after,
        &s,
        std::slice::from_ref(&target),
        std::slice::from_ref(&reverse),
        true,
    );
    assert!(!cycle.diagnostics.is_empty());
    assert!(cycle.unavailable.is_empty());
    let unavailable = validate_relation_change_with_graph(
        &owner,
        &before,
        &after,
        &s,
        std::slice::from_ref(&target),
        &[reverse],
        false,
    );
    assert!(unavailable.diagnostics.is_empty());
    assert!(!unavailable.unavailable.is_empty());
    let independent = source(", dependencies: query(1 + 1)");
    let done = independent.replace("status: \"open\"", "status: \"done\"");
    assert!(
        validate_relation_change_with_graph(&owner, &independent, &done, &s, &[], &[], false)
            .is_valid()
    );
    let leaf = RelationSource {
        entity_key: other,
        source: source("").replace(ID, other_id),
        lifecycle: "live".into(),
    };
    assert!(
        validate_relation_change_with_graph(&owner, &before, &after, &s, &[target], &[leaf], true)
            .is_valid()
    );
    let mut bad = serde_json::to_value(&s).unwrap();
    bad["relations"][0]["target_domain"] = serde_json::json!("text");
    assert!(Schema::from_author_json(&bad.to_string()).is_err());
    let mut bad = serde_json::to_value(&s).unwrap();
    bad["relations"][0]["max_assertions"] = serde_json::json!(1);
    assert!(Schema::from_author_json(&bad.to_string()).is_err());
    let mut bad = serde_json::to_value(&s).unwrap();
    bad["relations"][0]["cycle_policy"] = serde_json::json!("acyclic");
    assert!(Schema::from_author_json(&bad.to_string()).is_err());
}
#[test]
fn array_permutations_move_exact_unknown_author_slots_and_singleton_removal_stays_an_array() {
    let person = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), phones: ((number: \"1\", extra: decimal(\"0.30\")), (number: \"2\", extra: decimal(\"0.40\"))))) <weftext>"
    );
    let s = schema();
    let original = project(&person, &s)
        .fields
        .into_iter()
        .find(|f| f.key == "phones")
        .unwrap()
        .cell
        .native()
        .unwrap()
        .clone();
    let NativeValue::Array(mut values) = original else {
        panic!()
    };
    values.reverse();
    let moved = patch_field(
        &person,
        &s,
        "phones",
        &[],
        &NativeValue::Array(values.clone()),
    )
    .unwrap();
    assert_eq!(
        moved.source,
        person.replace(
            "(number: \"1\", extra: decimal(\"0.30\")), (number: \"2\", extra: decimal(\"0.40\"))",
            "(number: \"2\", extra: decimal(\"0.40\")), (number: \"1\", extra: decimal(\"0.30\"))"
        )
    );
    let singleton = patch_field(
        &moved.source,
        &s,
        "phones",
        &[],
        &NativeValue::Array(vec![values[1].clone()]),
    )
    .unwrap();
    assert_eq!(
        project(&singleton.source, &s)
            .fields
            .iter()
            .find(|f| f.key == "phones")
            .unwrap()
            .cell
            .native()
            .unwrap()
            .array()
            .unwrap()
            .len(),
        1
    );
    assert!(singleton.source.contains("decimal(\"0.30\")"));
    assert!(!singleton.source.contains("decimal(\"0.40\")"));
    let mut added = values;
    added.push(decode_native_expression("(number: \"3\", extra: 1)").unwrap());
    assert!(patch_field(&moved.source, &s, "phones", &[], &NativeValue::Array(added)).is_err());
    let multi = NativeValue::Array(vec![
        decode_native_expression("(number: \"2\", note: \"changed\", extra: decimal(\"0.40\"))")
            .unwrap(),
        decode_native_expression("(number: \"1\", extra: decimal(\"0.30\"))").unwrap(),
        decode_native_expression("(number: \"3\")").unwrap(),
    ]);
    let combined = patch_field(&person, &s, "phones", &[], &multi).unwrap();
    assert!(combined.source.contains("extra: decimal(\"0.30\")"));
    assert!(combined.source.contains("extra: decimal(\"0.40\")"));
    assert_eq!(
        project(&combined.source, &s)
            .fields
            .iter()
            .find(|f| f.key == "phones")
            .unwrap()
            .cell
            .native(),
        Some(&multi)
    );
    let cleared = patch_field(
        &combined.source,
        &s,
        "phones",
        &[],
        &NativeValue::Array(vec![]),
    )
    .unwrap();
    assert_eq!(
        project(&cleared.source, &s)
            .fields
            .iter()
            .find(|f| f.key == "phones")
            .unwrap()
            .cell
            .native(),
        Some(&NativeValue::Array(vec![]))
    );
}
#[test]
fn record_add_remove_and_union_transition_keep_unknown_source_and_native_empty_record() {
    let mut s = schema();
    s.fields.push(Field {
        id: "testing/box".into(),
        key: "box".into(),
        shape: Shape::Record {
            members: BTreeMap::from([
                (
                    "a".into(),
                    Member {
                        shape: Shape::Scalar {
                            semantic: "text".into(),
                        },
                        required: false,
                        nullable: false,
                    },
                ),
                (
                    "b".into(),
                    Member {
                        shape: Shape::Scalar {
                            semantic: "text".into(),
                        },
                        required: false,
                        nullable: false,
                    },
                ),
            ]),
            unknown: "preserve".into(),
            constraints: vec![],
        },
        nullable: false,
        default: Some(serde_json::json!({"kind":"absent"})),
    });
    s.types
        .iter_mut()
        .find(|t| t.id == "people/person")
        .unwrap()
        .fields
        .push(FieldContribution {
            id: "testing/box".into(),
            required: false,
        });
    let s = Schema::from_author_json(&serde_json::to_string(&s).unwrap()).unwrap();
    let person = format!(
        "#metadata((id: \"{ID}\", types: (\"person\",), phones: ((label: \"work\", number: \"1\", note: \"remove\", extra: decimal(\"0.30\")),), parents: ((target: (text: \"待确认\", extra: decimal(\"0.40\"))),), box: (a: \"remove\", /* retain */ b: \"remove\"))) <weftext>"
    );
    let p = project(&person, &s);
    let mut phones = p
        .fields
        .iter()
        .find(|f| f.key == "phones")
        .unwrap()
        .cell
        .native()
        .unwrap()
        .clone();
    let NativeValue::Array(items) = &mut phones else {
        panic!()
    };
    let NativeValue::Record(members) = &mut items[0] else {
        panic!()
    };
    members.retain(|m| m.key != "label" && m.key != "note");
    let removed = patch_field(&person, &s, "phones", &[], &phones).unwrap();
    assert!(removed.source.contains("extra: decimal(\"0.30\")"));
    assert!(!removed.source.contains("label: \"work\""));
    assert!(!removed.source.contains("note: \"remove\""));
    let target =
        "weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
    let replacement =
        decode_native_expression(&format!("(ref: \"{target}\", extra: decimal(\"0.40\"))"))
            .unwrap();
    let switched = patch_field(
        &removed.source,
        &s,
        "parents",
        &[
            ValuePath::Index { index: 0 },
            ValuePath::Member {
                key: "target".into(),
            },
        ],
        &replacement,
    )
    .unwrap();
    assert!(switched.source.contains("extra: decimal(\"0.40\")"));
    assert!(!switched.source.contains("text: \"待确认\""));
    assert_eq!(
        project(&switched.source, &s)
            .fields
            .iter()
            .find(|f| f.key == "parents")
            .unwrap()
            .cell
            .native()
            .unwrap()
            .array()
            .unwrap()[0]
            .member("target"),
        Some(&replacement)
    );
    let box_value = p
        .fields
        .iter()
        .find(|f| f.key == "box")
        .unwrap()
        .cell
        .native()
        .unwrap();
    assert!(box_value.record().is_some());
    let empty = patch_field(
        &switched.source,
        &s,
        "box",
        &[],
        &NativeValue::Record(vec![]),
    )
    .unwrap();
    assert!(empty.source.contains("/* retain */"));
    assert_eq!(
        project(&empty.source, &s)
            .fields
            .iter()
            .find(|f| f.key == "box")
            .unwrap()
            .cell
            .native(),
        Some(&NativeValue::Record(vec![]))
    );
    let recreated = patch_field(
        &empty.source,
        &s,
        "box",
        &[],
        &decode_native_expression("(a: \"new\")").unwrap(),
    )
    .unwrap();
    assert!(recreated.source.contains("/* retain */"));
    let dir = tempfile::TempDir::new().unwrap();
    let compiled = weftext_engine::Engine::new()
        .unwrap()
        .render(
            &recreated.source,
            &dir.path().join("records.typ"),
            dir.path(),
        )
        .unwrap();
    assert!(compiled.ok, "{:?}", compiled.diagnostics);
}
