use std::str::FromStr;

use serde_json::json;
use weftext_core::{
    ChecklistMarker, ChecklistState, ChecklistToggleError, ChecklistToggleEvidence,
    DocumentRevision, NodeId, plan_checklist_toggle_source,
};

const OWNER_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

fn evidence(source: &str, index: usize) -> ChecklistToggleEvidence {
    let occurrence = &weftext_asciidoc::analyze(source).checklists[index];
    ChecklistToggleEvidence {
        owner_node_id: NodeId::from_str(OWNER_ID).expect("owner ID"),
        revision: DocumentRevision::from_source(source),
        occurrence: occurrence.parser_occurrence.clone(),
        authored_marker: occurrence.authored_marker,
        marker_range: occurrence.marker_range.clone(),
    }
}

#[test]
fn all_authored_markers_toggle_canonically_and_keep_exact_utf8_source() {
    for (source_marker, after_marker, before_state, after_state) in [
        (
            "[ ]",
            "[x]",
            ChecklistState::Todo,
            ChecklistState::Completed,
        ),
        (
            "[x]",
            "[ ]",
            ChecklistState::Completed,
            ChecklistState::Todo,
        ),
        (
            "[*]",
            "[ ]",
            ChecklistState::Completed,
            ChecklistState::Todo,
        ),
    ] {
        let source = format!("= 文缕 😀\r\n\r\n* {source_marker} שלום 中文");
        let evidence = evidence(&source, 0);
        let plan = plan_checklist_toggle_source(&source, &evidence).expect("toggle plan");
        assert_eq!(&source[plan.edit.range.clone()], source_marker);
        assert_eq!(plan.edit.replacement, after_marker);
        assert_eq!(plan.summary.before_state, before_state);
        assert_eq!(plan.summary.after_state, after_state);
        assert_eq!(plan.summary.base_revision, evidence.revision);
        assert_eq!(
            plan.summary.next_revision,
            DocumentRevision::from_source(&plan.proposed_source)
        );
        assert_eq!(
            &source[..plan.edit.range.start],
            &plan.proposed_source[..plan.edit.range.start]
        );
        assert_eq!(
            &source[plan.edit.range.end..],
            &plan.proposed_source[plan.edit.range.start + plan.edit.replacement.len()..]
        );
        assert!(plan.proposed_source.ends_with("שלום 中文"));
    }
}

#[test]
fn exact_occurrence_selects_one_nested_duplicate_description() {
    let source = concat!("= T\n\n", "* [ ] same\n", "** [ ] same\n", "* [ ] same\n",);
    let evidence = evidence(source, 1);
    let plan = plan_checklist_toggle_source(source, &evidence).expect("nested toggle");
    assert_eq!(
        plan.summary.occurrence.parser_ordinal_path,
        vec![0, 0, 0, 0]
    );
    assert_eq!(plan.proposed_source.matches("[x] same").count(), 1);
    assert_eq!(plan.proposed_source.matches("[ ] same").count(), 2);
    assert_eq!(
        weftext_asciidoc::analyze(&plan.proposed_source).checklists[1].list_depth,
        2
    );
}

#[test]
fn stale_tampered_and_protected_evidence_fails_closed() {
    let source = "= T\n\n* [ ] visible 😀\n\n----\n* [ ] protected\n----\n";
    let valid = evidence(source, 0);

    let mut stale = valid.clone();
    stale.revision = DocumentRevision::from_source("different");
    assert_eq!(
        plan_checklist_toggle_source(source, &stale),
        Err(ChecklistToggleError::StaleDocumentRevision)
    );

    let mut path = valid.clone();
    path.occurrence.parser_ordinal_path.push(99);
    assert_eq!(
        plan_checklist_toggle_source(source, &path),
        Err(ChecklistToggleError::EvidenceMismatch)
    );

    let mut marker = valid.clone();
    marker.authored_marker = ChecklistMarker::CheckedStar;
    assert_eq!(
        plan_checklist_toggle_source(source, &marker),
        Err(ChecklistToggleError::EvidenceMismatch)
    );

    let protected_start = source.rfind("[ ] protected").expect("protected marker") as u64;
    let mut protected = valid.clone();
    protected.marker_range = protected_start..protected_start + 3;
    assert_eq!(
        plan_checklist_toggle_source(source, &protected),
        Err(ChecklistToggleError::EvidenceMismatch)
    );

    let emoji = source.find('😀').expect("emoji");
    let mut non_boundary = valid;
    non_boundary.marker_range = (emoji + 1) as u64..(emoji + 2) as u64;
    assert_eq!(
        plan_checklist_toggle_source(source, &non_boundary),
        Err(ChecklistToggleError::EvidenceMismatch)
    );
}

#[test]
fn action_evidence_serde_is_closed_and_enum_spelling_is_stable() {
    let source = "= T\n\n* [*] item\n";
    let evidence = evidence(source, 0);
    let value = serde_json::to_value(&evidence).expect("serialize evidence");
    assert_eq!(value["authoredMarker"], json!("checked_star"));
    assert_eq!(value["occurrence"]["branchComplete"], json!(true));
    assert!(value["occurrence"]["promotionBranch"].is_object());
    let decoded: ChecklistToggleEvidence =
        serde_json::from_value(value.clone()).expect("deserialize evidence");
    assert_eq!(decoded, evidence);

    let mut unknown_field = value.clone();
    unknown_field["extra"] = json!(true);
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(unknown_field).is_err());
    let mut unknown_marker = value.clone();
    unknown_marker["authoredMarker"] = json!("checked_uppercase");
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(unknown_marker).is_err());
    let mut inconsistent_branch = value.clone();
    inconsistent_branch["occurrence"]["branchComplete"] = json!(false);
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(inconsistent_branch).is_err());
    let mut missing_promotion = value.clone();
    missing_promotion["occurrence"]["promotionBranch"] = json!(null);
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(missing_promotion).is_err());
    let mut unknown_lift_edit = value.clone();
    unknown_lift_edit["occurrence"]["promotionBranch"]["liftEdits"][0]["extra"] = json!(0);
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(unknown_lift_edit).is_err());
    let mut unknown_occurrence = value;
    unknown_occurrence["occurrence"]["extra"] = json!(0);
    assert!(serde_json::from_value::<ChecklistToggleEvidence>(unknown_occurrence).is_err());
    assert!(serde_json::from_value::<ChecklistState>(json!("done")).is_err());
}
