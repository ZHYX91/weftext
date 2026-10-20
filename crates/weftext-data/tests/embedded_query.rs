use weftext_data::*;

#[test]
fn arrangement_preserves_exact_carriers_unicode_and_refuses_content_barriers() {
    let one = query(KEY, "one", "");
    let two = query("6b2b9e19-5599-48c3-b27b-6b006ec9c397", "two", "");
    let source = format!("前😀\r\n\r\n{one}\r\n\r\n{two}\n\n后");
    let found = embedded_queries(&source);
    let moved = arrange_embedded_query(&source, &found[1].source.id, "up").unwrap();
    assert_eq!(
        moved.source,
        format!("前😀\r\n\r\n{two}\r\n\r\n{one}\n\n后")
    );
    let removed = arrange_embedded_query(&source, &found[0].source.id, "remove").unwrap();
    assert_eq!(removed.source, source.replacen(&one, "", 1));
    assert!(arrange_embedded_query(&source, &found[0].source.id, "up").is_err());
    for barrier in ["/* author comment */", "正文", "#set text(size: 12pt)"] {
        let source = format!("{one}\n{barrier}\n{two}");
        let found = embedded_queries(&source);
        assert!(arrange_embedded_query(&source, &found[1].source.id, "up").is_err());
    }
}

#[test]
fn group_builder_roundtrips_and_preserves_unchanged_native_comments() {
    let source = query(KEY, "one", "");
    let found = embedded_queries(&source);
    let mut plan = found[0].plan.clone().unwrap();
    plan.group = Some(
        serde_json::from_value(
            serde_json::json!({"keys":[],"aggregates":[{"name":"count","op":"count"}]}),
        )
        .unwrap(),
    );
    let edited = edit_embedded_query_plan(&source, &found[0].source.id, &plan).unwrap();
    assert!(edited.source.contains("/* preserve comment */"));
    let next = embedded_queries(&edited.source);
    assert!(next[0].builder_supported);
    assert_eq!(
        serde_json::to_value(&next[0].plan).unwrap(),
        serde_json::to_value(Some(plan)).unwrap()
    );
}

#[test]
fn simple_shared_call_builder_changes_only_the_caller_and_retains_exact_owner() {
    let source = query(KEY, "callee", "");
    let call = query(
        "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
        "call",
        "advanced: (call: (owner: \"weftext-node://58c47256-bfce-4d29-a037-b83a3a644fb0/8a81e35f-ad48-4c12-af4f-3d2607102fa9\", selection: (mode: \"current\", key: \"f8597b15-6b7f-4a6a-9b52-c2732730fe41\"), arguments: (:))),",
    );
    let original = format!("{source}\n\n{call}");
    let found = embedded_queries(&original);
    assert!(found[1].builder_supported);
    let mut plan = found[1].plan.clone().unwrap();
    plan.advanced
        .as_mut()
        .unwrap()
        .call
        .as_mut()
        .unwrap()
        .arguments
        .insert("wanted".into(), "\"中文😀\"".into());
    let changed = edit_embedded_query_plan(&original, &found[1].source.id, &plan).unwrap();
    assert!(changed.source.starts_with(&source));
    assert!(changed.source.contains("/* preserve comment */"));
    assert_eq!(saved_queries(&changed.source).unwrap()[0].key, KEY);
}

const KEY: &str = "f8597b15-6b7f-4a6a-9b52-c2732730fe41";
fn query(key: &str, label: &str, extra: &str) -> String {
    format!(
        "#metadata((kind: \"weftext.query\", key: \"{key}\", binding: (schema: \"weftext-config://58c47256-bfce-4d29-a037-b83a3a644fb0/schema\", revision: \"8a81e35f-ad48-4c12-af4f-3d2607102fa9\", language: \"weftext.query.cel.v1\"), scope: (library: \"current\", lifecycle: \"live\"), types: (\"task\",), /* preserve comment */ where: \"status == 'open'\", select: ((name: \"task\", expr: \"self\"),), {extra})) <weftext-query-{label}>"
    )
}

#[test]
fn builder_changes_only_selected_definition_preserving_comment_and_baseline() {
    let one = query(KEY, "one", "");
    let two = query("6b2b9e19-5599-48c3-b27b-6b006ec9c397", "two", "");
    let source = format!("前😀\r\n{one}\n中\n{two}\n后");
    let found = embedded_queries(&source);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|q| q.writable && q.builder_supported));
    let mut plan = found[1].plan.clone().unwrap();
    plan.r#where = Some("status == 'done'".into());
    plan.take = Some(12);
    let patch = edit_embedded_query_plan(&source, &found[1].source.id, &plan).unwrap();
    assert!(patch.source.contains(&one));
    assert!(patch.text.contains("/* preserve comment */"));
    assert!(patch.text.contains("8a81e35f-ad48-4c12-af4f-3d2607102fa9"));
    assert_eq!(&source[..patch.from], &patch.source[..patch.from]);
    assert!(patch.source.ends_with(&source[patch.to..]));
    let readback = embedded_queries(&patch.source);
    assert_eq!(
        serde_json::to_value(&readback[1].plan).unwrap(),
        serde_json::to_value(Some(plan)).unwrap()
    );
    assert!(
        edit_embedded_query_plan(
            &patch.source,
            &found[1].source.id,
            readback[1].plan.as_ref().unwrap()
        )
        .is_err()
    );
}

#[test]
fn advanced_source_is_runnable_but_builder_never_simplifies_it() {
    let source = query(
        KEY,
        "advanced",
        "advanced: (distinct: ((name: \"task\", expr: \"self\"),)),",
    );
    let found = embedded_queries(&source);
    assert!(found[0].runnable);
    assert!(!found[0].builder_supported);
    let proposed = QueryPlan::default();
    assert!(edit_embedded_query_plan(&source, &found[0].source.id, &proposed).is_err());
    let replacement = "#metadata((temporarily: ";
    let patch = replace_embedded_query_source(&source, &found[0].source.id, replacement).unwrap();
    assert_eq!(patch.source, replacement);
}

#[test]
fn duplicate_keys_computed_and_invalid_source_never_get_guessed_writers() {
    let one = query(KEY, "one", "");
    let two = query(KEY, "two", "");
    let duplicates = embedded_queries(&format!("{one}\n{two}"));
    assert_eq!(duplicates.len(), 2);
    assert!(duplicates.iter().all(|q| !q.writable && !q.runnable));
    let computed = one.replace("where: \"status == 'open'\"", "where: str(1 + 1)");
    let found = embedded_queries(&computed);
    assert!(!found[0].builder_supported && !found[0].runnable);
    assert!(found[0].writable); // explicit source proposal still has one exact target
    let invalid = format!("{one}\n#let incomplete = (");
    let found = embedded_queries(&invalid);
    assert!(found.iter().all(|q| !q.writable && !q.runnable));
}

#[test]
fn commented_values_are_preserved_by_rejecting_lossy_builder_edits() {
    let source = query(KEY, "one", "").replace("(\"task\",)", "(\"task\", /* author detail */)");
    let found = embedded_queries(&source);
    let mut plan = found[0].plan.clone().unwrap();
    plan.types = vec![];
    assert!(edit_embedded_query_plan(&source, &found[0].source.id, &plan).is_err());
}
