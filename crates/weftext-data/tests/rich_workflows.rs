//! Selected rich workflows use the installed schema and actual native writer/CEL.
//! These pure tests do not establish Core persistence, permissions or recovery.
use weftext_data::*;

const WORKSPACE: &str = "58c47256-bfce-4d29-a037-b83a3a644fb0";
const OWNER: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";
const A: &str = "12d7c4f3-9ee6-4c36-920d-ec6af27bc654";
const B: &str = "8aa10c08-3e7d-4d10-9488-92bc7f393050";
fn reference(id: &str) -> String {
    format!("weftext-node://{WORKSPACE}/{id}")
}
fn source(kind: &str, fields: &str) -> String {
    format!(
        "#metadata((id: \"{OWNER}\", types: (\"{kind}\",), {fields})) <weftext>\n\n正文😀 /* untouched body */\r\n"
    )
}
fn field(source: &str, schema: &Schema, key: &str) -> NativeValue {
    let projected = project(source, schema);
    assert_eq!(projected.qualification, "qualified");
    assert!(
        projected.diagnostics.is_empty(),
        "{:?}",
        projected.diagnostics
    );
    projected
        .fields
        .into_iter()
        .find(|field| field.key == key)
        .unwrap()
        .cell
        .native()
        .expect("fixture must provide a qualified business value")
        .clone()
}
fn query(source: &str, schema: &Schema, condition: &str, expression: &str) -> QueryResult {
    execute_query(
        &FrozenQueryInput {
            cut_id: "rich-workflow".into(),
            schema_revision: "installed".into(),
            coverage: "complete".into(),
            freshness: "current".into(),
            advanced: None,
            notes: vec![FrozenNote {
                entity_key: reference(OWNER),
                path: "Owned/Owned.typ".into(),
                source: source.into(),
                revision: "source-cut".into(),
                lifecycle: "live".into(),
                title: Some("Owned fixture".into()),
                body: None,
                authorized_fields: None,
                products: None,
            }],
        },
        &QueryPlan {
            r#where: Some(condition.into()),
            select: vec![QueryColumn {
                name: "records".into(),
                expr: expression.into(),
            }],
            ..Default::default()
        },
        schema,
    )
    .unwrap()
}
#[test]
fn rich_workflow_contact_reorder_keeps_duplicate_usage_notes_and_unknown_source() {
    let schema = Schema::bundled().unwrap();
    let original = source(
        "person",
        "phones: ((number: \"00138\", label: \"work\", note: \"first\", imported: decimal(\"0.30\")), (number: \"00138\", label: \"personal\", note: \"second\"), (number: \"00138\", label: \"work\", note: \"third\"))",
    );
    let NativeValue::Array(mut records) = field(&original, &schema, "phones") else {
        panic!("contact list")
    };
    records.rotate_right(1);
    let changed = patch_field(
        &original,
        &schema,
        "phones",
        &[],
        &NativeValue::Array(records.clone()),
    )
    .unwrap();
    assert!(changed.source.ends_with("正文😀 /* untouched body */\r\n"));
    assert!(changed.source.contains("decimal(\"0.30\")"));
    let result = query(
        &changed.source,
        &schema,
        "phones.exists(p, p.number == '00138' && p.label == 'work' && p.note == 'first')",
        "phones",
    );
    assert_eq!(result.coverage, "complete");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(
        result.rows[0].cells["records"].native(),
        Some(&NativeValue::Array(records))
    );
    let negative = query(
        &changed.source,
        &schema,
        "phones.exists(p, p.label == 'personal' && p.note == 'first')",
        "phones",
    );
    assert_eq!(negative.coverage, "complete");
    assert!(
        negative.rows.is_empty(),
        "never borrow a neighboring record's note"
    );
    // Ordinary direct contact input needs no evidence or optional usage label.
    let direct = source("person", "phones: ((number: \"00138\"),)");
    assert_eq!(field(&direct, &schema, "phones").array().unwrap().len(), 1);
}
#[test]
fn rich_workflow_employment_matches_organization_position_and_precision_in_one_record() {
    let schema = Schema::bundled().unwrap();
    let original = source(
        "person",
        &format!(
            "engagements: ((employer: (ref: \"{}\"), position: \"Engineer\", during: (start: (calendar: \"gregorian\", version: \"proleptic-gregorian.v1\", precision: \"year\", year: 2020), end: none)), (employer: (ref: \"{}\"), position: \"Editor\", during: (start: (calendar: \"gregorian\", version: \"proleptic-gregorian.v1\", precision: \"month\", year: 2024, month: 3), end: none)))",
            reference(A),
            reference(B)
        ),
    );
    let records = field(&original, &schema, "engagements");
    let condition = format!(
        "engagements.exists(e, e.employer.ref == nodeRef('{}') && e.position == 'Engineer' && e.during.start.year == 2020)",
        reference(A)
    );
    let result = query(&original, &schema, &condition, "engagements");
    assert_eq!(result.coverage, "complete");
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].cells["records"].native(), Some(&records));
    for wrong in [
        condition.replace("'Engineer'", "'Editor'"),
        condition.replace("== 2020", "== 2024"),
    ] {
        let negative = query(&original, &schema, &wrong, "engagements");
        assert_eq!(negative.coverage, "complete");
        assert!(negative.rows.is_empty(), "{wrong}");
    }
    let mut reordered = records.array().unwrap().to_vec();
    reordered.reverse();
    let changed = patch_field(
        &original,
        &schema,
        "engagements",
        &[],
        &NativeValue::Array(reordered.clone()),
    )
    .unwrap();
    let after = query(&changed.source, &schema, &condition, "engagements");
    assert_eq!(
        after.rows[0].cells["records"].native(),
        Some(&NativeValue::Array(reordered.clone()))
    );
    let year = reordered[1]
        .member("during")
        .unwrap()
        .member("start")
        .unwrap();
    assert_eq!(year.member("precision").unwrap().text(), Some("year"));
    assert!(year.member("month").is_none());
    assert!(year.member("day").is_none());
    assert!(reordered.iter().all(|record| matches!(
        record.member("during").unwrap().member("end"),
        Some(NativeValue::None)
    )));
    // Unlike Range, an absent During endpoint is unknown; never invent an open end.
    let unknown_end = original.replace(", end: none", "");
    let projection = project(&unknown_end, &schema);
    assert!(matches!(
        projection
            .fields
            .iter()
            .find(|field| field.key == "engagements")
            .unwrap()
            .cell,
        Cell::Unavailable { .. }
    ));
    let before = source("person", "");
    // Engagements admit ordinary nodes. A Person target is legal; the negative
    // below proves only that the newly referenced B is missing from admission.
    let admitted_a = RelationTarget {
        entity_key: reference(A),
        types: vec!["person".into()],
        lifecycle: "live".into(),
    };
    let missing_b = validate_relation_change(
        &before,
        &original,
        &schema,
        std::slice::from_ref(&admitted_a),
        true,
    );
    assert!(missing_b.unavailable.is_empty());
    assert_eq!(missing_b.diagnostics.len(), 1);
    assert_eq!(
        missing_b.diagnostics[0].explanation,
        UiMessage::error_data_relations_new_target_requires_exact_live_admitted_node(
            weftext_messages::params::ErrorDataRelationsNewTargetRequiresExactLiveAdmittedNode {
                relation_id: "people/engagement".into(),
            },
        )
    );
    let admitted_b = RelationTarget {
        entity_key: reference(B),
        types: vec!["organization".into()],
        lifecycle: "live".into(),
    };
    let qualified =
        validate_relation_change(&before, &original, &schema, &[admitted_a, admitted_b], true);
    assert!(qualified.is_valid(), "{qualified:?}");
}
#[test]
fn rich_workflow_same_name_contributors_reorder_without_merging_identity_or_role() {
    let schema = Schema::bundled().unwrap();
    let original = source(
        "paper",
        &format!(
            "publication: (kind: \"article\", title: \"Owned paper\", contributors: ((agent: (ref: \"{}\"), credited_as: \"同名\", roles: (\"author\",)), (agent: (ref: \"{}\"), credited_as: \"同名\", roles: (\"editor\",))))",
            reference(A),
            reference(B)
        ),
    );
    let mut publication = field(&original, &schema, "publication");
    let contributors = publication.member("contributors").unwrap().array().unwrap();
    let mut reversed = contributors.to_vec();
    reversed.reverse();
    if let NativeValue::Record(members) = &mut publication {
        members
            .iter_mut()
            .find(|member| member.key == "contributors")
            .unwrap()
            .value = NativeValue::Array(reversed.clone());
    }
    let changed = patch_field(&original, &schema, "publication", &[], &publication).unwrap();
    let result = query(&changed.source, &schema, "true", "publication.contributors");
    assert_eq!(result.coverage, "complete");
    assert_eq!(
        result.rows[0].cells["records"].native(),
        Some(&NativeValue::Array(reversed))
    );
    let negative = query(
        &changed.source,
        &schema,
        &format!(
            "publication.contributors.exists(c, c.agent.ref == nodeRef('{}') && c.roles.exists(r, r == 'editor'))",
            reference(A)
        ),
        "publication.contributors",
    );
    assert_eq!(negative.coverage, "complete");
    assert!(negative.rows.is_empty());
    assert!(changed.source.ends_with("正文😀 /* untouched body */\r\n"));
}
