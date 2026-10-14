use weftext_data::*;

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
