use std::collections::BTreeMap;
use weftext_data::*;

const ID: &str = "32eb9a12-e752-4bfa-8cdf-65e00f23ac26";
const FROM: &str = "transfer/from";
const INTO: &str = "transfer/into";

fn text() -> Shape {
    Shape::Scalar {
        semantic: "text".into(),
    }
}
fn list(items: Shape, min: usize, max: Option<usize>) -> Shape {
    Shape::List {
        items: Box::new(items),
        min,
        max,
    }
}
fn schema() -> Schema {
    let mut schema = Schema::bundled().unwrap();
    schema.definitions.insert("transfer_item".into(), text());
    for (id, key, items) in [
        (
            FROM,
            "transfer_from",
            Shape::Use {
                name: "transfer_item".into(),
            },
        ),
        (INTO, "transfer_into", text()),
    ] {
        schema.fields.push(Field {
            id: id.into(),
            key: key.into(),
            shape: list(items, 0, None),
            nullable: true,
            default: None,
        });
        schema
            .types
            .iter_mut()
            .find(|kind| kind.id == "tasks/task")
            .unwrap()
            .fields
            .push(FieldContribution {
                id: id.into(),
                required: false,
            });
    }
    schema
}
fn source(fields: &str) -> String {
    format!(
        "// outside-before 😀\r\n#metadata((id: \"{ID}\", types: (\"task\",), status: \"open\", {fields})) <weftext>\r\n// outside-after\r\nBody 原件😀\r\n"
    )
}
fn values(source: &str, schema: &Schema, key: &str) -> Vec<NativeValue> {
    project(source, schema)
        .fields
        .iter()
        .find(|field| field.field_id == key)
        .unwrap()
        .cell
        .native()
        .unwrap()
        .array()
        .unwrap()
        .to_vec()
}
fn strings(values: &[&str]) -> Vec<NativeValue> {
    values
        .iter()
        .map(|value| NativeValue::Text((*value).into()))
        .collect()
}
fn assert_single_inverse(before: &str, patch: &FieldPatch) {
    assert_eq!(
        patch.source,
        format!(
            "{}{}{}",
            &before[..patch.from],
            patch.text,
            &before[patch.to..]
        )
    );
    assert_eq!(&before[..patch.from], &patch.source[..patch.from]);
    assert_eq!(
        &before[patch.to..],
        &patch.source[patch.from + patch.text.len()..]
    );
    let exact_undo = format!(
        "{}{}{}",
        &patch.source[..patch.from],
        &before[patch.from..patch.to],
        &patch.source[patch.from + patch.text.len()..]
    );
    assert_eq!(exact_undo.as_bytes(), before.as_bytes());
    assert!(patch.source.starts_with("// outside-before 😀\r\n"));
    assert!(patch.source.ends_with("Body 原件😀\r\n"));
}
fn shape_mut<'a>(schema: &'a mut Schema, key: &str) -> &'a mut Shape {
    &mut schema
        .fields
        .iter_mut()
        .find(|field| field.id == key)
        .unwrap()
        .shape
}

#[test]
fn duplicate_occurrences_move_by_ordinal_and_keep_their_exact_native_spelling() {
    let schema = schema();
    let original =
        source(r#"transfer_from: ("dup", "\u{64}up", "尾😀",), transfer_into: ("dest",)"#);
    let patch = patch_list_transfer(&original, &schema, FROM, INTO, 1, 1, 1).unwrap();
    assert_single_inverse(&original, &patch);
    assert_eq!(
        values(&patch.source, &schema, FROM),
        strings(&["dup", "尾😀"])
    );
    assert_eq!(
        values(&patch.source, &schema, INTO),
        strings(&["dest", "dup"])
    );
    assert!(patch.source.contains(r#"transfer_from: ("dup", "尾😀",)"#));
    assert!(
        patch
            .source
            .contains(r#"transfer_into: ("dest", "\u{64}up",)"#)
    );
}

#[test]
fn contiguous_ranges_insert_at_the_beginning_middle_and_end_of_the_other_list() {
    let schema = schema();
    let original = source(r#"transfer_into: ("x", "y",), transfer_from: ("a", "b", "c", "d",)"#);
    for (at, expected) in [
        (0, vec!["b", "c", "x", "y"]),
        (1, vec!["x", "b", "c", "y"]),
        (2, vec!["x", "y", "b", "c"]),
    ] {
        let patch = patch_list_transfer(&original, &schema, FROM, INTO, 1, 2, at).unwrap();
        assert_single_inverse(&original, &patch);
        assert_eq!(values(&patch.source, &schema, FROM), strings(&["a", "d"]));
        assert_eq!(values(&patch.source, &schema, INTO), strings(&expected));
    }
}

#[test]
fn moving_the_whole_list_to_an_existing_empty_list_keeps_both_fields_present() {
    let schema = schema();
    let original = source(r#"transfer_from: ("a", "b",), transfer_into: ()"#);
    let patch = patch_list_transfer(&original, &schema, FROM, INTO, 0, 1, 0).unwrap();
    assert_single_inverse(&original, &patch);
    assert!(values(&patch.source, &schema, FROM).is_empty());
    assert_eq!(values(&patch.source, &schema, INTO), strings(&["a", "b"]));
    assert!(patch.source.contains("transfer_from: ()"));
}

#[test]
fn neighboring_comments_unknown_root_members_and_parenthesized_frames_stay_byte_exact() {
    let schema = schema();
    let middle = ",\r\n // remains on the dictionary slot\r\n unknown_fact: (opaque: \"unknown 原件\",),\r\n // before destination\r\n transfer_into: ";
    let original = source(&format!(
        "transfer_from: ( /* field frame */ (\"a\", \"b\",) ){middle}(\"x\",)"
    ));
    let patch = patch_list_transfer(&original, &schema, FROM, INTO, 0, 0, 1).unwrap();
    assert_single_inverse(&original, &patch);
    assert!(patch.text.contains(middle));
    assert!(patch.source.contains("( /* field frame */ (\"b\",) )"));
    assert_eq!(
        project(&patch.source, &schema).unknown_fields[0].raw,
        "(opaque: \"unknown 原件\",)"
    );
}

#[test]
fn comments_between_list_occurrences_are_refused_instead_of_lost_or_reanchored() {
    let schema = schema();
    for fields in [
        "transfer_from: (\"a\", /* item ownership? */ \"b\",), transfer_into: (\"x\",)",
        "transfer_from: (\"a\", \"b\",), transfer_into: (/* destination note */ \"x\",)",
        "transfer_from: (\"a\", \"b\",), transfer_into: (\"x\", // trailing note\n)",
    ] {
        let original = source(fields);
        let failure = patch_list_transfer(&original, &schema, FROM, INTO, 0, 0, 0).unwrap_err();
        assert_eq!(
            failure.description().explanation.key(),
            "error.data.fieldTransfer.triviaUncertified"
        );
    }
}

#[test]
fn list_bounds_are_checked_jointly_on_both_final_values_before_a_patch_exists() {
    let original = source(r#"transfer_from: ("a", "b",), transfer_into: ("x",)"#);
    for (from_min, into_max) in [(2, None), (0, Some(1))] {
        let mut schema = schema();
        *shape_mut(&mut schema, FROM) = list(text(), from_min, None);
        *shape_mut(&mut schema, INTO) = list(text(), 0, into_max);
        let failure = patch_list_transfer(&original, &schema, FROM, INTO, 0, 0, 1).unwrap_err();
        assert_eq!(
            failure.description().explanation.key(),
            "error.data.projection.fieldEditValidationFailed"
        );
        assert!(!failure.clone().into_diagnostics().is_empty());
    }
}

#[test]
fn compatible_native_strings_do_not_erase_different_declared_item_semantics() {
    let mut schema = schema();
    *shape_mut(&mut schema, FROM) = list(
        Shape::Scalar {
            semantic: "phone".into(),
        },
        0,
        None,
    );
    let original = source(r#"transfer_from: ("12345",), transfer_into: ("text",)"#);
    let failure = patch_list_transfer(&original, &schema, FROM, INTO, 0, 0, 0).unwrap_err();
    assert_eq!(
        failure.description().explanation.key(),
        "error.data.fieldTransfer.itemShapesDiffer"
    );
}

#[test]
fn top_level_union_and_conflicting_control_types_do_not_gain_transfer_write_qualification() {
    let original = source(r#"transfer_from: ("a", "b",), transfer_into: ("x",)"#);
    let mut union_schema = schema();
    *shape_mut(&mut union_schema, FROM) = Shape::Union {
        variants: vec![list(text(), 0, None), text()],
        discriminator: None,
    };
    let failure = patch_list_transfer(&original, &union_schema, FROM, INTO, 0, 0, 0).unwrap_err();
    assert_eq!(
        failure.description().explanation.key(),
        "error.data.fieldTransfer.existingListsRequired"
    );

    let mut conflict_schema = schema();
    conflict_schema
        .types
        .iter_mut()
        .find(|kind| kind.id == "tasks/task")
        .unwrap()
        .conflicts
        .push("transfer/block".into());
    conflict_schema.types.push(Type {
        id: "transfer/block".into(),
        fields: vec![],
        requires: vec![],
        conflicts: vec!["tasks/task".into()],
        constraints: vec![],
    });
    let conflicting = original.replace(
        r#"types: ("task",)"#,
        r#"types: ("task", "transfer/block",)"#,
    );
    let failure =
        patch_list_transfer(&conflicting, &conflict_schema, FROM, INTO, 0, 0, 0).unwrap_err();
    assert_eq!(
        failure.description().explanation.key(),
        "error.data.projection.controlTypesUnqualifiedOrConflicting"
    );
}

#[test]
fn invalid_ordinals_same_field_and_unavailable_or_nonlist_values_never_produce_half_a_move() {
    let schema = schema();
    let original = source(r#"transfer_from: ("a", "b",), transfer_into: ("x",)"#);
    for (start, end, to) in [
        (1, 0, 0),
        (0, 2, 0),
        (0, usize::MAX, 0),
        (0, 0, 2),
        (usize::MAX, usize::MAX, 0),
    ] {
        assert!(patch_list_transfer(&original, &schema, FROM, INTO, start, end, to).is_err());
    }
    assert!(patch_list_transfer(&original, &schema, FROM, "transfer_from", 0, 0, 0).is_err());
    for fields in [
        r#"transfer_into: ("x",)"#,
        r#"transfer_from: ("a",)"#,
        r#"transfer_from: none, transfer_into: ("x",)"#,
        r#"transfer_from: ("a",), transfer_into: none"#,
        r#"transfer_from: (), transfer_into: ("x",)"#,
        r#"transfer_from: (1,), transfer_into: ("x",)"#,
        r#"transfer_from: ("a",), transfer_into: (1,)"#,
        r#"transfer_from: make-list(), transfer_into: ("x",)"#,
        r#"transfer_from: ("a",), transfer_into: make-list()"#,
        r#"transfer_from: "scalar", transfer_into: ("x",)"#,
    ] {
        assert!(
            patch_list_transfer(&source(fields), &schema, FROM, INTO, 0, 0, 0).is_err(),
            "{fields}"
        );
    }
    let mut scalar_schema = schema.clone();
    *shape_mut(&mut scalar_schema, INTO) = text();
    assert!(
        patch_list_transfer(
            &source(r#"transfer_from: ("a",), transfer_into: "x""#),
            &scalar_schema,
            FROM,
            INTO,
            0,
            0,
            0
        )
        .is_err()
    );
    let mut uncontributed = schema.clone();
    uncontributed
        .types
        .iter_mut()
        .find(|kind| kind.id == "tasks/task")
        .unwrap()
        .fields
        .retain(|field| field.id != INTO);
    assert!(patch_list_transfer(&original, &uncontributed, FROM, INTO, 0, 0, 0).is_err());
}

fn record_item() -> Shape {
    Shape::Record {
        members: BTreeMap::from([(
            "value".into(),
            Member {
                shape: text(),
                required: true,
                nullable: false,
            },
        )]),
        unknown: "preserve".into(),
        constraints: vec![],
    }
}
#[test]
fn known_record_items_move_whole_raw_expressions_but_unbound_members_are_refused() {
    let mut schema = schema();
    *shape_mut(&mut schema, FROM) = list(record_item(), 0, None);
    *shape_mut(&mut schema, INTO) = list(record_item(), 0, None);
    let item = "(value: /* inside the exact record */ \"a\",)";
    let original = source(&format!(
        "transfer_from: ({item},), transfer_into: ((value: \"x\",),)"
    ));
    let patch = patch_list_transfer(&original, &schema, FROM, INTO, 0, 0, 1).unwrap();
    assert_single_inverse(&original, &patch);
    assert!(
        patch
            .source
            .contains(&format!("transfer_into: ((value: \"x\",), {item},)"))
    );
    assert_eq!(
        patch
            .source
            .matches("/* inside the exact record */")
            .count(),
        1
    );
    for fields in [
        r#"transfer_from: ((value: "a", alien: "keep",),), transfer_into: ()"#,
        r#"transfer_from: ((value: "a",),), transfer_into: ((value: "x", alien: "keep",),)"#,
    ] {
        let failure =
            patch_list_transfer(&source(fields), &schema, FROM, INTO, 0, 0, 0).unwrap_err();
        assert_eq!(
            failure.description().explanation.key(),
            "error.data.projection.unknownAuthorMembersReadOnly"
        );
    }
}
