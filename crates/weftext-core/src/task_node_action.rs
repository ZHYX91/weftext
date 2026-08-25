use std::fmt;

use serde::{Deserialize, Serialize};
use weftext_asciidoc::{
    DocumentHeaderPatchError, SourceEdit, SourceEditPlan, plan_document_header_attribute_patch,
};

use crate::{
    DocumentRevision, NodeId, TaskNodeDiagnostic, TaskNodePriority, TaskNodeProfile, TaskNodeState,
    TaskNodeTemporal, analyze_task_node_profile,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskNodeActionEvidence {
    pub node_id: NodeId,
    pub revision: DocumentRevision,
    pub profile_revision: DocumentRevision,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeTemporalField {
    Created,
    Start,
    Scheduled,
    Due,
    Closed,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TaskNodeClosedEdit {
    #[default]
    Preserve,
    Clear,
    Set {
        value: TaskNodeTemporal,
    },
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TaskNodeClosedEditWire {
    Preserve {},
    Clear {},
    Set { value: TaskNodeTemporal },
}

impl<'de> Deserialize<'de> for TaskNodeClosedEdit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(match TaskNodeClosedEditWire::deserialize(deserializer)? {
            TaskNodeClosedEditWire::Preserve {} => Self::Preserve,
            TaskNodeClosedEditWire::Clear {} => Self::Clear,
            TaskNodeClosedEditWire::Set { value } => Self::Set { value },
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TaskNodeEditIntent {
    SetState {
        state: TaskNodeState,
        #[serde(default)]
        closed: TaskNodeClosedEdit,
    },
    SetPriority {
        priority: Option<TaskNodePriority>,
    },
    SetTemporal {
        field: TaskNodeTemporalField,
        value: Option<TaskNodeTemporal>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskNodeEditRequest {
    pub evidence: TaskNodeActionEvidence,
    pub intent: TaskNodeEditIntent,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeEditSummary {
    pub node_id: NodeId,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub base_profile_revision: DocumentRevision,
    pub next_profile_revision: DocumentRevision,
    pub intent: TaskNodeEditIntent,
    pub before: TaskNodeProfile,
    pub after: TaskNodeProfile,
}

/// Verified source-only task-node edit. It is not a filesystem or commit plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeSourceEditPlan {
    pub summary: TaskNodeEditSummary,
    pub edits: Vec<SourceEdit>,
    pub proposed_source: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskNodeEditError {
    StaleDocumentRevision,
    StaleProfileRevision,
    EvidenceMismatch,
    InvalidCurrentProfile {
        diagnostics: Vec<TaskNodeDiagnostic>,
    },
    InvalidIntent,
    HeaderPatch(DocumentHeaderPatchError),
    EditPlan,
    PostValidation {
        diagnostics: Vec<TaskNodeDiagnostic>,
    },
}

impl fmt::Display for TaskNodeEditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StaleDocumentRevision => {
                formatter.write_str("task-node action has a stale document revision")
            }
            Self::StaleProfileRevision => {
                formatter.write_str("task-node action has a stale profile revision")
            }
            Self::EvidenceMismatch => {
                formatter.write_str("task-node action evidence does not match the current source")
            }
            Self::InvalidCurrentProfile { .. } => {
                formatter.write_str("current source does not contain one valid task-node profile")
            }
            Self::InvalidIntent => {
                formatter.write_str("task-node edit intent conflicts with the state invariants")
            }
            Self::HeaderPatch(error) => write!(formatter, "task-node header patch failed: {error}"),
            Self::EditPlan => {
                formatter.write_str("task-node edits are not one valid non-overlapping source plan")
            }
            Self::PostValidation { .. } => {
                formatter.write_str("task-node edit failed complete profile post-validation")
            }
        }
    }
}

impl std::error::Error for TaskNodeEditError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::HeaderPatch(error) => Some(error),
            _ => None,
        }
    }
}

/// Plans a narrow typed task-node profile edit and fully revalidates the proposed source.
///
/// # Errors
///
/// Returns an error for either stale revision dimension, an invalid current profile, contradictory
/// state/closed intent, unsafe header syntax, overlapping edits, or failed post-validation.
#[allow(clippy::too_many_lines)]
pub fn plan_task_node_source_edit(
    source: &str,
    request: &TaskNodeEditRequest,
) -> Result<TaskNodeSourceEditPlan, TaskNodeEditError> {
    let base_revision = DocumentRevision::from_source(source);
    if request.evidence.revision != base_revision {
        return Err(TaskNodeEditError::StaleDocumentRevision);
    }
    if request.evidence.profile_revision != base_revision {
        return Err(TaskNodeEditError::StaleProfileRevision);
    }

    let current = analyze_task_node_profile(source, Some(request.evidence.node_id));
    if current.profile_revision != request.evidence.profile_revision {
        return Err(TaskNodeEditError::EvidenceMismatch);
    }
    let Some(before) = current.profile.clone() else {
        return Err(TaskNodeEditError::InvalidCurrentProfile {
            diagnostics: current.diagnostics,
        });
    };
    if current.title.is_none() || !current.diagnostics.is_empty() {
        return Err(TaskNodeEditError::InvalidCurrentProfile {
            diagnostics: current.diagnostics,
        });
    }

    let mut expected_after = before.clone();
    let mut edits = Vec::new();
    match &request.intent {
        TaskNodeEditIntent::SetState { state, closed } => {
            if !state.is_closed() && matches!(closed, TaskNodeClosedEdit::Set { .. }) {
                return Err(TaskNodeEditError::InvalidIntent);
            }
            expected_after.state = *state;
            expected_after.closed = if state.is_closed() {
                match closed {
                    TaskNodeClosedEdit::Preserve => before.closed.clone(),
                    TaskNodeClosedEdit::Clear => None,
                    TaskNodeClosedEdit::Set { value } => Some(value.clone()),
                }
            } else {
                None
            };
            push_header_edit(
                source,
                "weftext-task-state",
                Some(state_source(*state)),
                &mut edits,
            )?;
            push_header_edit(
                source,
                "weftext-task-closed",
                expected_after.closed.as_ref().map(TaskNodeTemporal::as_str),
                &mut edits,
            )?;
        }
        TaskNodeEditIntent::SetPriority { priority } => {
            expected_after.priority = *priority;
            push_header_edit(
                source,
                "weftext-task-priority",
                priority.map(priority_source),
                &mut edits,
            )?;
        }
        TaskNodeEditIntent::SetTemporal { field, value } => {
            if *field == TaskNodeTemporalField::Closed
                && value.is_some()
                && !before.state.is_closed()
            {
                return Err(TaskNodeEditError::InvalidIntent);
            }
            temporal_field_mut(&mut expected_after, *field).clone_from(value);
            push_header_edit(
                source,
                temporal_attribute(*field),
                value.as_ref().map(TaskNodeTemporal::as_str),
                &mut edits,
            )?;
        }
    }

    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    let proposed_source = SourceEditPlan::new(source, edits.clone())
        .map_err(|_| TaskNodeEditError::EditPlan)?
        .apply(source)
        .ok_or(TaskNodeEditError::EditPlan)?;
    if !outside_edits_are_identical(source, &proposed_source, &edits) {
        return Err(TaskNodeEditError::EditPlan);
    }

    let next_revision = DocumentRevision::from_source(&proposed_source);
    let next = analyze_task_node_profile(&proposed_source, Some(request.evidence.node_id));
    let Some(after) = next.profile.clone() else {
        return Err(TaskNodeEditError::PostValidation {
            diagnostics: next.diagnostics,
        });
    };
    if next.title.is_none()
        || !next.diagnostics.is_empty()
        || next.profile_revision != next_revision
        || after != expected_after
    {
        return Err(TaskNodeEditError::PostValidation {
            diagnostics: next.diagnostics,
        });
    }

    Ok(TaskNodeSourceEditPlan {
        summary: TaskNodeEditSummary {
            node_id: request.evidence.node_id,
            base_revision: base_revision.clone(),
            next_revision: next_revision.clone(),
            base_profile_revision: base_revision,
            next_profile_revision: next_revision,
            intent: request.intent.clone(),
            before,
            after,
        },
        edits,
        proposed_source,
    })
}

fn push_header_edit(
    source: &str,
    name: &str,
    value: Option<&str>,
    edits: &mut Vec<SourceEdit>,
) -> Result<(), TaskNodeEditError> {
    if let Some(edit) = plan_document_header_attribute_patch(source, name, value)
        .map_err(TaskNodeEditError::HeaderPatch)?
    {
        edits.push(edit);
    }
    Ok(())
}

const fn state_source(state: TaskNodeState) -> &'static str {
    match state {
        TaskNodeState::Todo => "todo",
        TaskNodeState::InProgress => "in-progress",
        TaskNodeState::OnHold => "on-hold",
        TaskNodeState::Completed => "completed",
        TaskNodeState::Cancelled => "cancelled",
    }
}

const fn priority_source(priority: TaskNodePriority) -> &'static str {
    match priority {
        TaskNodePriority::Lowest => "lowest",
        TaskNodePriority::Low => "low",
        TaskNodePriority::Normal => "normal",
        TaskNodePriority::Medium => "medium",
        TaskNodePriority::High => "high",
        TaskNodePriority::Highest => "highest",
    }
}

const fn temporal_attribute(field: TaskNodeTemporalField) -> &'static str {
    match field {
        TaskNodeTemporalField::Created => "weftext-task-created",
        TaskNodeTemporalField::Start => "weftext-task-start",
        TaskNodeTemporalField::Scheduled => "weftext-task-scheduled",
        TaskNodeTemporalField::Due => "weftext-task-due",
        TaskNodeTemporalField::Closed => "weftext-task-closed",
    }
}

fn temporal_field_mut(
    profile: &mut TaskNodeProfile,
    field: TaskNodeTemporalField,
) -> &mut Option<TaskNodeTemporal> {
    match field {
        TaskNodeTemporalField::Created => &mut profile.created,
        TaskNodeTemporalField::Start => &mut profile.start,
        TaskNodeTemporalField::Scheduled => &mut profile.scheduled,
        TaskNodeTemporalField::Due => &mut profile.due,
        TaskNodeTemporalField::Closed => &mut profile.closed,
    }
}

fn outside_edits_are_identical(source: &str, proposed: &str, edits: &[SourceEdit]) -> bool {
    let mut source_cursor = 0_usize;
    let mut proposed_cursor = 0_usize;
    for edit in edits {
        let unchanged_len = edit.range.start.saturating_sub(source_cursor);
        if source.get(source_cursor..edit.range.start)
            != proposed.get(proposed_cursor..proposed_cursor.saturating_add(unchanged_len))
        {
            return false;
        }
        source_cursor = edit.range.end;
        proposed_cursor = proposed_cursor
            .saturating_add(unchanged_len)
            .saturating_add(edit.replacement.len());
    }
    source.get(source_cursor..) == proposed.get(proposed_cursor..)
}
