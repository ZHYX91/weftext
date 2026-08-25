use weftext_core::{ChecklistMarker, ChecklistState, analyze_checklist_source};

const EVIDENCE: &str = include_str!("../../../tests/fixtures/checklist-v1/valid/evidence.adoc");
const PROTECTED: &str =
    include_str!("../../../tests/fixtures/checklist-v1/invalid/protected-contexts.adoc");

#[test]
fn core_consumes_parser_checklist_evidence_without_a_second_recognizer() {
    let parser = weftext_asciidoc::analyze(EVIDENCE);
    let core = analyze_checklist_source(EVIDENCE);

    assert_eq!(core.semantic_model_version, parser.semantic_model_version);
    assert_eq!(core.occurrences, parser.checklists);
    assert!(core.diagnostics.is_empty());
    assert_eq!(core.occurrences[0].authored_marker, ChecklistMarker::Open);
    assert_eq!(core.occurrences[0].state, ChecklistState::Todo);
    assert_eq!(
        core.occurrences[1].authored_marker,
        ChecklistMarker::CheckedX
    );
    assert_eq!(
        core.occurrences[2].authored_marker,
        ChecklistMarker::CheckedStar
    );
    assert!(
        core.occurrences
            .iter()
            .all(|occurrence| occurrence.parser_occurrence.branch_complete)
    );
}

#[test]
fn protected_contexts_are_excluded_by_the_parser_projection() {
    let analysis = analyze_checklist_source(PROTECTED);
    assert_eq!(analysis.occurrences.len(), 1);
    assert_eq!(analysis.occurrences[0].description, "visible");
    assert!(analysis.diagnostics.is_empty());
}

#[test]
fn list_like_non_checklist_source_is_not_promoted() {
    let source = "= T\n\n* [X] uppercase\n* [/] custom\n. [ ] ordered\n";
    let analysis = analyze_checklist_source(source);
    assert!(analysis.occurrences.is_empty());
}

#[test]
fn parser_owned_unordered_variants_are_not_reclassified_by_core() {
    let source = "= T\n\n- [ ] dash marker\n";
    let parser = weftext_asciidoc::analyze(source);
    let core = analyze_checklist_source(source);
    assert_eq!(core.occurrences, parser.checklists);
    assert_eq!(core.occurrences.len(), 1);
    assert_eq!(core.occurrences[0].description, "dash marker");
}
