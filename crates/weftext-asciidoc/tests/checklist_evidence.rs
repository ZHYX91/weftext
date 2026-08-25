use std::ops::Range;

use weftext_asciidoc::{
    ChecklistBranchLiftEditKind, ChecklistEvidence, ChecklistMarker,
    ChecklistPromotionContextDependencyKind, ChecklistState, SEMANTIC_MODEL_VERSION, analyze,
};

const EVIDENCE: &str = include_str!("../../../tests/fixtures/checklist-v1/valid/evidence.adoc");
const PROTECTED: &str =
    include_str!("../../../tests/fixtures/checklist-v1/invalid/protected-contexts.adoc");

fn exact<'a>(source: &'a str, range: &Range<u64>) -> &'a str {
    let start = usize::try_from(range.start).expect("range start");
    let end = usize::try_from(range.end).expect("range end");
    assert!(source.is_char_boundary(start));
    assert!(source.is_char_boundary(end));
    &source[start..end]
}

#[test]
fn promotion_evidence_closes_document_context_dependencies() {
    let source = "* [ ] context\n+\nUses {custom-name}.\n";
    let analysis = analyze(source);
    let promotion = analysis.checklists[0]
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("promotion evidence");
    let expected = ChecklistPromotionContextDependencyKind::DocumentAttributeReference;
    assert!(
        promotion
            .context_dependencies
            .iter()
            .any(|dependency| dependency.kind == expected),
        "missing {expected:?}: {promotion:#?}"
    );
}

fn promotion_body(source: &str, evidence: &ChecklistEvidence) -> String {
    evidence
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("complete promotion evidence")
        .destination_body(source)
        .expect("valid parser-owned recipe")
}

#[test]
fn unordered_list_parser_projects_complete_exact_checklist_evidence() {
    let analysis = analyze(EVIDENCE);

    assert_eq!(SEMANTIC_MODEL_VERSION, 3);
    assert_eq!(analysis.semantic_model_version, 3);
    assert_eq!(analysis.checklists.len(), 3);
    assert_eq!(
        analysis
            .checklists
            .iter()
            .map(|item| (item.authored_marker, item.state))
            .collect::<Vec<_>>(),
        [
            (ChecklistMarker::Open, ChecklistState::Todo),
            (ChecklistMarker::CheckedX, ChecklistState::Completed),
            (ChecklistMarker::CheckedStar, ChecklistState::Completed),
        ]
    );
    assert_eq!(
        analysis
            .checklists
            .iter()
            .map(|item| item.parser_occurrence.parser_ordinal_path.as_slice())
            .collect::<Vec<_>>(),
        [&[0, 0][..], &[0, 0, 0, 0][..], &[0, 1][..]]
    );

    let open = &analysis.checklists[0];
    assert_eq!(exact(EVIDENCE, &open.marker_range), "[ ]");
    assert_eq!(exact(EVIDENCE, &open.description_range), "发布 文缕 😀");
    assert_eq!(open.description, "发布 文缕 😀");
    assert_eq!(open.list_depth, 1);
    assert!(open.parser_occurrence.branch_complete);
    let branch = open
        .parser_occurrence
        .branch_range
        .as_ref()
        .expect("complete parent branch");
    assert!(exact(EVIDENCE, branch).contains("** [x] مرحبا"));
    assert_eq!(promotion_body(EVIDENCE, open), "* [x] مرحبا\n");

    let nested = &analysis.checklists[1];
    assert_eq!(nested.list_depth, 2);
    assert_eq!(exact(EVIDENCE, &nested.marker_range), "[x]");
    assert_eq!(exact(EVIDENCE, &nested.description_range), "مرحبا");

    let completed = &analysis.checklists[2];
    assert!(completed.parser_occurrence.branch_complete);
    let branch = exact(
        EVIDENCE,
        completed
            .parser_occurrence
            .branch_range
            .as_ref()
            .expect("continuation branch"),
    );
    assert!(branch.contains("Continuation paragraph."));
    assert!(branch.contains("* [ ] protected listing text"));
    assert_eq!(
        promotion_body(EVIDENCE, completed),
        "Continuation paragraph.\n----\n* [ ] protected listing text\n----\n"
    );
    assert_eq!(
        completed
            .parser_occurrence
            .promotion_branch
            .as_ref()
            .unwrap()
            .lift_edits
            .iter()
            .filter(|edit| {
                edit.kind == ChecklistBranchLiftEditKind::RemoveContinuationConnector
            })
            .count(),
        2
    );
}

#[test]
fn protected_context_markers_never_become_checklist_evidence() {
    let analysis = analyze(PROTECTED);
    assert_eq!(analysis.checklists.len(), 1);
    assert_eq!(analysis.checklists[0].description, "visible");
    assert_eq!(
        exact(PROTECTED, &analysis.checklists[0].marker_range),
        "[ ]"
    );
}

#[test]
fn utf8_ranges_survive_crlf_and_a_missing_final_newline() {
    let source = "= T\r\n\r\n* [ ] 中文 😀\r\n* [x] שלום";
    let analysis = analyze(source);
    assert_eq!(analysis.checklists.len(), 2);
    assert_eq!(
        exact(source, &analysis.checklists[0].description_range),
        "中文 😀"
    );
    assert_eq!(
        exact(source, &analysis.checklists[1].description_range),
        "שלום"
    );
    for item in &analysis.checklists {
        assert!(item.parser_occurrence.branch_complete);
        let _ = exact(source, &item.item_range);
        let _ = exact(source, &item.marker_range);
        let _ = exact(source, &item.description_range);
        let _ = exact(
            source,
            item.parser_occurrence
                .branch_range
                .as_ref()
                .expect("complete branch"),
        );
    }
}

#[test]
fn leaf_markers_own_one_physical_line_and_lift_to_an_empty_body() {
    for marker in ["[ ]", "[x]", "[*]"] {
        let source = format!("* {marker} leaf\n* sibling\n");
        let analysis = analyze(&source);
        assert_eq!(analysis.checklists.len(), 1, "{marker}");
        let checklist = &analysis.checklists[0];
        let promotion = checklist
            .parser_occurrence
            .promotion_branch
            .as_ref()
            .expect("leaf promotion evidence");
        assert_eq!(
            exact(&source, &promotion.source_replacement_range),
            format!("* {marker} leaf\n")
        );
        assert_eq!(promotion_body(&source, checklist), "");
        assert_eq!(promotion.lift_edits.len(), 1);
        assert_eq!(
            promotion.lift_edits[0].kind,
            ChecklistBranchLiftEditKind::OmitPrincipal
        );
    }
}

#[test]
fn nested_star_descendants_are_dedented_without_consuming_the_sibling() {
    let source = "* [ ] parent\n** child\n*** grandchild\n* sibling\n";
    let analysis = analyze(source);
    let parent = &analysis.checklists[0];
    let promotion = parent
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("nested branch");

    assert_eq!(
        exact(source, &promotion.source_replacement_range),
        "* [ ] parent\n** child\n*** grandchild\n"
    );
    assert_eq!(promotion_body(source, parent), "* child\n** grandchild\n");
    assert_eq!(
        promotion
            .lift_edits
            .iter()
            .filter_map(|edit| match edit.kind {
                ChecklistBranchLiftEditKind::DedentDescendant {
                    from_depth,
                    to_depth,
                } => Some((
                    exact(source, &edit.range),
                    edit.replacement.as_str(),
                    from_depth,
                    to_depth
                )),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [("**", "*", 2, 1), ("***", "**", 3, 2)]
    );
}

#[test]
fn mixed_unordered_and_ordered_descendants_preserve_relative_ordered_markers() {
    let source = "* [ ] parent\n+\n. ordered\n.. nested ordered\n* sibling\n";
    let analysis = analyze(source);
    let parent = &analysis.checklists[0];
    let promotion = parent
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("mixed-list branch");

    assert_eq!(
        exact(source, &promotion.source_replacement_range),
        "* [ ] parent\n+\n. ordered\n.. nested ordered\n"
    );
    assert_eq!(
        promotion_body(source, parent),
        ". ordered\n.. nested ordered\n"
    );
    let ordered_edits = promotion
        .lift_edits
        .iter()
        .filter(|edit| {
            matches!(
                edit.kind,
                ChecklistBranchLiftEditKind::DedentDescendant { .. }
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(ordered_edits.len(), 2);
    assert_eq!(promotion.lifted_descendant_count, 2);
    assert_eq!(promotion.lifted_continuation_count, 1);
    assert_eq!(ordered_edits[0].replacement, ".");
    assert_eq!(ordered_edits[1].replacement, "..");
}

#[test]
fn continuation_attachments_have_exact_deterministic_recipes() {
    let cases = [
        ("paragraph", "Attached paragraph.\n"),
        ("admonition", "NOTE: attached\n"),
        ("listing", "----\ncode * [ ] protected\n----\n"),
        ("literal", "....\nliteral * [ ] protected\n....\n"),
        ("image", "image::picture.png[Alt]\n"),
    ];

    for (name, attachment) in cases {
        let source = format!("* [ ] task\n+\n{attachment}\n* sibling\n");
        let analysis = analyze(&source);
        assert_eq!(analysis.checklists.len(), 1, "{name}: {analysis:#?}");
        let checklist = &analysis.checklists[0];
        let promotion = checklist
            .parser_occurrence
            .promotion_branch
            .as_ref()
            .unwrap_or_else(|| panic!("missing {name} evidence: {analysis:#?}"));
        assert_eq!(
            exact(&source, &promotion.source_replacement_range),
            format!("* [ ] task\n+\n{attachment}"),
            "{name}"
        );
        assert_eq!(promotion.lifted_continuation_count, 1, "{name}");
        if name == "image" {
            assert!(promotion.context_dependencies.iter().any(|dependency| {
                dependency.kind == ChecklistPromotionContextDependencyKind::RelativeLocator
            }));
        }
        assert_eq!(promotion_body(&source, checklist), attachment, "{name}");
        assert_eq!(
            promotion
                .lift_edits
                .iter()
                .filter(|edit| {
                    edit.kind == ChecklistBranchLiftEditKind::RemoveContinuationConnector
                })
                .count(),
            1,
            "{name}"
        );
    }
}

#[test]
fn deepest_delimited_descendant_closing_line_is_owned() {
    let source = concat!(
        "* [ ] parent\n",
        "** child\n",
        "+\n",
        "----\n",
        "deep listing\n",
        "----\n",
        "* sibling\n",
    );
    let analysis = analyze(source);
    let parent = &analysis.checklists[0];
    let promotion = parent
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("deep delimited branch");

    assert_eq!(
        exact(source, &promotion.source_replacement_range),
        concat!(
            "* [ ] parent\n",
            "** child\n",
            "+\n",
            "----\n",
            "deep listing\n",
            "----\n",
        )
    );
    assert_eq!(
        promotion_body(source, parent),
        "* child\n+\n----\ndeep listing\n----\n"
    );
}

#[test]
fn multiple_connectors_are_removed_by_exact_ranges() {
    let source = concat!(
        "* [ ] principal line\n",
        "+\n",
        "First attachment.\n",
        "+\n",
        "----\n",
        "second attachment\n",
        "----\n",
        "\n",
        "* sibling\n",
    );
    let analysis = analyze(source);
    let checklist = &analysis.checklists[0];
    let promotion = checklist
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("multi-connector branch");
    assert_eq!(
        promotion_body(source, checklist),
        "First attachment.\n----\nsecond attachment\n----\n"
    );
    assert_eq!(
        promotion
            .lift_edits
            .iter()
            .filter(|edit| {
                edit.kind == ChecklistBranchLiftEditKind::RemoveContinuationConnector
            })
            .count(),
        2
    );
}

#[test]
fn multiline_principal_is_omitted_as_one_parser_owned_range() {
    let source = concat!(
        "* [ ] principal line\n",
        "  continued principal\n",
        "+\n",
        "Attached body.\n",
    );
    let analysis = analyze(source);
    assert_eq!(analysis.checklists.len(), 1);
    let checklist = &analysis.checklists[0];
    let promotion = checklist
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("multiline principal branch");
    assert_eq!(promotion_body(source, checklist), "Attached body.\n");
    assert_eq!(
        exact(source, &promotion.lift_edits[0].range),
        "* [ ] principal line\n  continued principal\n"
    );
}

#[test]
fn attached_table_parser_abort_fails_closed() {
    // asciidork-ast 0.18.2 cannot compute ListItem::last_loc for a table attachment. The adapter
    // catches that upstream panic, protects every byte, and deliberately emits no invented branch.
    let source = "* [ ] task\n+\n|===\n|A |B\n|===\n";
    let analysis = analyze(source);
    assert!(analysis.checklists.is_empty());
    assert!(
        analysis
            .protected_ranges
            .contains(&(0..source.len() as u64))
    );
}

#[test]
fn promotion_ranges_preserve_utf8_and_each_authored_line_ending() {
    let source = "* [ ] 中文 😀 مرحبا\r\n** שלום\n*** 子项 🧪\r\n* sibling";
    let analysis = analyze(source);
    let parent = &analysis.checklists[0];
    let promotion = parent
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("mixed-EOL UTF-8 branch");
    assert_eq!(
        exact(source, &promotion.source_replacement_range),
        "* [ ] 中文 😀 مرحبا\r\n** שלום\n*** 子项 🧪\r\n"
    );
    assert_eq!(promotion_body(source, parent), "* שלום\n** 子项 🧪\r\n");
    for edit in &promotion.lift_edits {
        let _ = exact(source, &edit.range);
    }
}

#[test]
fn missing_final_newline_is_owned_exactly() {
    let source = "* [ ] task\n** child";
    let analysis = analyze(source);
    let checklist = &analysis.checklists[0];
    let promotion = checklist
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .expect("EOF branch");
    assert_eq!(promotion.source_replacement_range.end, source.len() as u64);
    assert_eq!(promotion_body(source, checklist), "* child");
}

#[test]
fn malformed_and_dangling_branches_fail_closed_without_losing_checklist_projection() {
    let cases = [
        "* [ ] dangling\n+\n* sibling\n",
        "* [ ] unclosed\n+\n----\nunterminated\n",
    ];
    for source in cases {
        let analysis = analyze(source);
        assert_eq!(analysis.checklists.len(), 1, "{source:?}: {analysis:#?}");
        let occurrence = &analysis.checklists[0].parser_occurrence;
        assert!(!occurrence.branch_complete, "{source:?}");
        assert!(occurrence.branch_range.is_none(), "{source:?}");
        assert!(occurrence.promotion_branch.is_none(), "{source:?}");
    }
}
