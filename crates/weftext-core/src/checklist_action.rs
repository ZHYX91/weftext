use std::fmt;
use std::ops::Range;

use serde::{Deserialize, Serialize};
use weftext_asciidoc::{SourceEdit, SourceEditPlan};

use crate::{
    ChecklistEvidence, ChecklistMarker, ChecklistParserOccurrence, ChecklistState,
    DocumentRevision, NodeId,
};

/// Revision-bound parser evidence authorizing one identity-free checklist marker toggle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChecklistToggleEvidence {
    pub owner_node_id: NodeId,
    pub revision: DocumentRevision,
    pub occurrence: ChecklistParserOccurrence,
    pub authored_marker: ChecklistMarker,
    pub marker_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistToggleSummary {
    pub owner_node_id: NodeId,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub occurrence: ChecklistParserOccurrence,
    pub marker_range: Range<u64>,
    pub before_marker: ChecklistMarker,
    pub after_marker: ChecklistMarker,
    pub before_state: ChecklistState,
    pub after_state: ChecklistState,
}

/// Verified source-only plan. Filesystem freshness and committing remain workspace-layer work.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistToggleSourcePlan {
    pub evidence: ChecklistToggleEvidence,
    pub summary: ChecklistToggleSummary,
    pub edit: SourceEdit,
    pub proposed_source: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChecklistToggleError {
    StaleDocumentRevision,
    EvidenceMismatch,
    InvalidMarkerRange,
    EditPlan,
    PostValidation,
}

impl fmt::Display for ChecklistToggleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StaleDocumentRevision => "checklist evidence has a stale document revision",
            Self::EvidenceMismatch => {
                "checklist evidence does not identify exactly one current parser occurrence"
            }
            Self::InvalidMarkerRange => {
                "checklist marker evidence is not an exact UTF-8 source marker"
            }
            Self::EditPlan => "checklist marker edit is not a valid exact-source edit",
            Self::PostValidation => {
                "checklist marker edit did not preserve parser identity and surrounding source"
            }
        })
    }
}

impl std::error::Error for ChecklistToggleError {}

/// Plans and fully reparses one canonical checklist marker toggle.
///
/// `branch_complete` is promotion evidence and does not restrict marker toggling.
///
/// # Errors
///
/// Returns an error for stale or forged evidence, invalid UTF-8/range authority, or any parser
/// identity change outside the expected marker spelling and state.
pub fn plan_checklist_toggle_source(
    source: &str,
    evidence: &ChecklistToggleEvidence,
) -> Result<ChecklistToggleSourcePlan, ChecklistToggleError> {
    let base_revision = DocumentRevision::from_source(source);
    if base_revision != evidence.revision {
        return Err(ChecklistToggleError::StaleDocumentRevision);
    }

    let analysis = weftext_asciidoc::analyze(source);
    let matches = analysis
        .checklists
        .iter()
        .enumerate()
        .filter(|(_, occurrence)| checklist_matches_evidence(occurrence, evidence))
        .collect::<Vec<_>>();
    let [(target_index, current)] = matches.as_slice() else {
        return Err(ChecklistToggleError::EvidenceMismatch);
    };

    let marker_range = usize_range(&evidence.marker_range, source)?;
    let expected_marker = marker_source(evidence.authored_marker);
    if source.get(marker_range.clone()) != Some(expected_marker) {
        return Err(ChecklistToggleError::InvalidMarkerRange);
    }
    let (after_marker, after_state, replacement) = match evidence.authored_marker {
        ChecklistMarker::Open => (ChecklistMarker::CheckedX, ChecklistState::Completed, "[x]"),
        ChecklistMarker::CheckedX | ChecklistMarker::CheckedStar => {
            (ChecklistMarker::Open, ChecklistState::Todo, "[ ]")
        }
    };
    let edit = SourceEdit {
        range: marker_range,
        replacement: replacement.to_owned(),
    };
    let proposed_source = SourceEditPlan::new(source, vec![edit.clone()])
        .map_err(|_| ChecklistToggleError::EditPlan)?
        .apply(source)
        .ok_or(ChecklistToggleError::EditPlan)?;
    if !outside_edit_is_identical(source, &proposed_source, &edit) {
        return Err(ChecklistToggleError::PostValidation);
    }

    let next_analysis = weftext_asciidoc::analyze(&proposed_source);
    if analysis.checklists.len() != next_analysis.checklists.len() {
        return Err(ChecklistToggleError::PostValidation);
    }
    for (index, (before, after)) in analysis
        .checklists
        .iter()
        .zip(&next_analysis.checklists)
        .enumerate()
    {
        if index == *target_index {
            if !same_checklist_authority(before, after)
                || after.authored_marker != after_marker
                || after.state != after_state
            {
                return Err(ChecklistToggleError::PostValidation);
            }
        } else if before != after {
            return Err(ChecklistToggleError::PostValidation);
        }
    }

    let next_revision = DocumentRevision::from_source(&proposed_source);
    Ok(ChecklistToggleSourcePlan {
        evidence: evidence.clone(),
        summary: ChecklistToggleSummary {
            owner_node_id: evidence.owner_node_id,
            base_revision,
            next_revision,
            occurrence: evidence.occurrence.clone(),
            marker_range: evidence.marker_range.clone(),
            before_marker: current.authored_marker,
            after_marker,
            before_state: current.state,
            after_state,
        },
        edit,
        proposed_source,
    })
}

fn usize_range(range: &Range<u64>, source: &str) -> Result<Range<usize>, ChecklistToggleError> {
    let start =
        usize::try_from(range.start).map_err(|_| ChecklistToggleError::InvalidMarkerRange)?;
    let end = usize::try_from(range.end).map_err(|_| ChecklistToggleError::InvalidMarkerRange)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(ChecklistToggleError::InvalidMarkerRange);
    }
    Ok(start..end)
}

const fn marker_source(marker: ChecklistMarker) -> &'static str {
    match marker {
        ChecklistMarker::Open => "[ ]",
        ChecklistMarker::CheckedX => "[x]",
        ChecklistMarker::CheckedStar => "[*]",
    }
}

fn same_checklist_authority(before: &ChecklistEvidence, after: &ChecklistEvidence) -> bool {
    before.item_range == after.item_range
        && before.marker_range == after.marker_range
        && before.description_range == after.description_range
        && before.description == after.description
        && before.list_depth == after.list_depth
        && before.parser_occurrence == after.parser_occurrence
}

fn outside_edit_is_identical(source: &str, proposed: &str, edit: &SourceEdit) -> bool {
    source.get(..edit.range.start) == proposed.get(..edit.range.start)
        && source.get(edit.range.end..)
            == proposed.get(edit.range.start.saturating_add(edit.replacement.len())..)
}

fn checklist_matches_evidence(
    occurrence: &ChecklistEvidence,
    evidence: &ChecklistToggleEvidence,
) -> bool {
    occurrence.parser_occurrence == evidence.occurrence
        && occurrence.marker_range == evidence.marker_range
        && occurrence.authored_marker == evidence.authored_marker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incomplete_branch_evidence_is_not_rejected_by_toggle_policy() {
        let source = "= T\n\n* [ ] item\n";
        let mut occurrence = weftext_asciidoc::analyze(source).checklists[0].clone();
        occurrence.parser_occurrence.branch_complete = false;
        occurrence.parser_occurrence.branch_range = None;
        let evidence = ChecklistToggleEvidence {
            owner_node_id: "550e8400-e29b-41d4-a716-446655440000"
                .parse()
                .expect("node ID"),
            revision: DocumentRevision::from_source(source),
            occurrence: occurrence.parser_occurrence.clone(),
            authored_marker: occurrence.authored_marker,
            marker_range: occurrence.marker_range.clone(),
        };
        assert!(checklist_matches_evidence(&occurrence, &evidence));
    }
}
