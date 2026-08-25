use std::collections::BTreeSet;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::{
    DocumentEdit, DocumentRevision, TaskDateTime, TaskId, TaskOccurrence, TaskPhase, TaskPriority,
    TaskRepeatFrom, TaskResolution, TaskSourceAnalysis, TaskState, analyze_task_source,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAuthoringPlan {
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub edit: DocumentEdit,
    pub proposed_source: String,
    pub assigned_id: Option<TaskId>,
    pub target: TaskOccurrence,
    pub analysis: TaskSourceAnalysis,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskEditTarget {
    Occurrence { range: Range<u64> },
    Id { id: TaskId },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDateField {
    Created,
    Start,
    Scheduled,
    Due,
    Closed,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TaskEditIntent {
    Toggle,
    SetPriority {
        priority: Option<TaskPriority>,
    },
    SetPhase {
        phase: Option<TaskPhase>,
    },
    SetResolution {
        resolution: Option<TaskResolution>,
    },
    SetDate {
        field: TaskDateField,
        value: Option<TaskDateTime>,
    },
    SetRecurrence {
        rrule: Option<String>,
        repeat_from: Option<TaskRepeatFrom>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAuthoringDiagnosticCode {
    InvalidTarget,
    AmbiguousTarget,
    InvalidSource,
    InvalidIntent,
    RecurrenceRequiresCompletionPlan,
    InvalidResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAuthoringFailure {
    pub code: TaskAuthoringDiagnosticCode,
    pub message: String,
    pub range: Option<Range<u64>>,
}

impl std::fmt::Display for TaskAuthoringFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TaskAuthoringFailure {}

/// Plans one exact task toggle or metadata edit against an exact source revision.
///
/// A simple checklist is promoted only when the requested metadata needs identity. Merely
/// toggling never writes a UUID. Existing structured metadata keeps every unrelated authored
/// byte; Core changes only the checkbox marker and requested attribute ranges. Recurring tasks
/// require the separate completion transaction because toggling one must also create or stop the
/// next occurrence.
///
/// # Errors
///
/// Returns a fail-closed diagnostic for an unavailable/ambiguous/invalid target, invalid typed
/// intent, recurrence that needs a completion transaction, or a proposed result rejected by the
/// canonical task parser.
pub fn plan_task_edit(
    source: &str,
    target: &TaskEditTarget,
    intent: &TaskEditIntent,
) -> Result<TaskAuthoringPlan, TaskAuthoringFailure> {
    let current = analyze_task_source(source);
    let occurrence = select_target(&current, target)?;
    validate_intent(occurrence, intent)?;
    let assigned_id =
        (occurrence.metadata.is_none() && intent_requires_metadata(intent)).then(TaskId::new);
    let mut edits = task_edits(source, occurrence, intent, assigned_id)?;
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    if edits
        .windows(2)
        .any(|pair| pair[0].range.end > pair[1].range.start)
    {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "task edit intent produced overlapping source changes",
            Some(occurrence.range.clone()),
        ));
    }
    let proposed_source = apply_edits(source, &edits)?;
    let analysis = analyze_task_source(&proposed_source);
    let next_target = select_result_target(&analysis, occurrence, assigned_id)?;
    let edit = minimal_edit(source, &proposed_source);
    Ok(TaskAuthoringPlan {
        base_revision: DocumentRevision::from_source(source),
        next_revision: DocumentRevision::from_source(&proposed_source),
        edit,
        proposed_source,
        assigned_id,
        target: next_target.clone(),
        analysis,
    })
}

pub(crate) fn plan_task_dependency_edit(
    source: &str,
    target: &TaskEditTarget,
    dependencies: &[TaskId],
) -> Result<TaskAuthoringPlan, TaskAuthoringFailure> {
    if dependencies.iter().copied().collect::<BTreeSet<_>>().len() != dependencies.len() {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "task dependencies must not contain duplicates",
            target_range(target),
        ));
    }
    let current = analyze_task_source(source);
    let occurrence = select_target(&current, target)?;
    let assigned_id = (occurrence.metadata.is_none() && !dependencies.is_empty()).then(TaskId::new);
    let effective_id =
        assigned_id.or_else(|| occurrence.metadata.as_ref().map(|metadata| metadata.id));
    if effective_id.is_some_and(|id| dependencies.contains(&id)) {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "a task cannot depend on itself",
            Some(occurrence.range.clone()),
        ));
    }
    let edits = if occurrence.metadata.is_some() {
        let encoded = (!dependencies.is_empty()).then(|| {
            serde_json::to_string(
                &dependencies
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .expect("dependency serialization")
        });
        attribute_edit(source, occurrence, "depends-on", encoded.as_deref())?
            .into_iter()
            .collect::<Vec<_>>()
    } else if let Some(id) = assigned_id {
        if occurrence.description.is_empty() {
            return Err(failure(
                TaskAuthoringDiagnosticCode::InvalidIntent,
                "an empty simple checklist cannot be promoted to structured metadata",
                Some(occurrence.range.clone()),
            ));
        }
        let encoded = serde_json::to_string(
            &dependencies
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        )
        .expect("dependency serialization");
        let insert = to_usize_range(&occurrence.range)?.end;
        vec![SourceChange {
            range: insert..insert,
            replacement: format!(" task:[id={id},depends-on={encoded}]"),
        }]
    } else {
        Vec::new()
    };
    let proposed_source = apply_edits(source, &edits)?;
    let analysis = analyze_task_source(&proposed_source);
    let next_target = select_result_target(&analysis, occurrence, assigned_id)?;
    if next_target
        .metadata
        .as_ref()
        .map_or(!dependencies.is_empty(), |metadata| {
            metadata.dependencies != dependencies
        })
    {
        return Err(invalid_result(next_target.range.clone()));
    }
    Ok(TaskAuthoringPlan {
        base_revision: DocumentRevision::from_source(source),
        next_revision: DocumentRevision::from_source(&proposed_source),
        edit: minimal_edit(source, &proposed_source),
        proposed_source,
        assigned_id,
        target: next_target.clone(),
        analysis,
    })
}

#[derive(Clone, Debug)]
pub(crate) struct SourceChange {
    pub(crate) range: Range<usize>,
    pub(crate) replacement: String,
}

fn select_target<'a>(
    analysis: &'a TaskSourceAnalysis,
    target: &TaskEditTarget,
) -> Result<&'a TaskOccurrence, TaskAuthoringFailure> {
    let candidates = analysis
        .tasks
        .iter()
        .filter(|task| match target {
            TaskEditTarget::Occurrence { range } => task.range == *range,
            TaskEditTarget::Id { id } => task
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.id == *id),
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [] => Err(failure(
            TaskAuthoringDiagnosticCode::InvalidTarget,
            "task edit target is not an exact current task occurrence",
            target_range(target),
        )),
        [task] if task.valid => Ok(task),
        [task] => {
            let diagnostic = analysis
                .diagnostics
                .iter()
                .find(|diagnostic| ranges_overlap(&diagnostic.range, &task.range));
            Err(failure(
                TaskAuthoringDiagnosticCode::InvalidSource,
                diagnostic.map_or_else(
                    || "target task source is invalid".to_owned(),
                    |diagnostic| diagnostic.message.clone(),
                ),
                Some(task.range.clone()),
            ))
        }
        _ => Err(failure(
            TaskAuthoringDiagnosticCode::AmbiguousTarget,
            "task edit target resolves to more than one current occurrence",
            target_range(target),
        )),
    }
}

fn validate_intent(
    occurrence: &TaskOccurrence,
    intent: &TaskEditIntent,
) -> Result<(), TaskAuthoringFailure> {
    if matches!(intent, TaskEditIntent::Toggle)
        && occurrence
            .metadata
            .as_ref()
            .is_some_and(|metadata| metadata.recurrence.is_some())
    {
        return Err(failure(
            TaskAuthoringDiagnosticCode::RecurrenceRequiresCompletionPlan,
            "a recurring task must use the recurrence completion plan",
            Some(occurrence.range.clone()),
        ));
    }
    if matches!(intent, TaskEditIntent::SetPhase { phase: Some(_) })
        && occurrence.state == TaskState::Closed
        || matches!(
            intent,
            TaskEditIntent::SetResolution {
                resolution: Some(_)
            } | TaskEditIntent::SetDate {
                field: TaskDateField::Closed,
                value: Some(_)
            }
        ) && occurrence.state == TaskState::Open
    {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "task lifecycle metadata is incompatible with the current checkbox state",
            Some(occurrence.range.clone()),
        ));
    }
    if let TaskEditIntent::SetRecurrence { rrule, repeat_from } = intent
        && rrule.is_some() != repeat_from.is_some()
    {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "rrule and repeat-from must be set or removed together",
            Some(occurrence.range.clone()),
        ));
    }
    Ok(())
}

fn intent_requires_metadata(intent: &TaskEditIntent) -> bool {
    !matches!(
        intent,
        TaskEditIntent::Toggle
            | TaskEditIntent::SetPriority {
                priority: None | Some(TaskPriority::Normal),
            }
            | TaskEditIntent::SetPhase {
                phase: None | Some(TaskPhase::Todo),
            }
            | TaskEditIntent::SetResolution {
                resolution: None | Some(TaskResolution::Completed),
            }
            | TaskEditIntent::SetDate { value: None, .. }
            | TaskEditIntent::SetRecurrence {
                rrule: None,
                repeat_from: None,
            }
    )
}

fn task_edits(
    source: &str,
    occurrence: &TaskOccurrence,
    intent: &TaskEditIntent,
    assigned_id: Option<TaskId>,
) -> Result<Vec<SourceChange>, TaskAuthoringFailure> {
    if occurrence.metadata.is_none() {
        return simple_task_edits(occurrence, intent, assigned_id);
    }
    let mut edits = Vec::new();
    match intent {
        TaskEditIntent::Toggle => {
            edits.push(SourceChange {
                range: to_usize_range(&occurrence.marker_range)?,
                replacement: if occurrence.state == TaskState::Open {
                    "x".to_owned()
                } else {
                    " ".to_owned()
                },
            });
            let fields = if occurrence.state == TaskState::Open {
                [Some("phase"), None, None]
            } else {
                [Some("resolution"), Some("closed"), Some("phase")]
            };
            for field in fields.into_iter().flatten() {
                if let Some(edit) = attribute_edit(source, occurrence, field, None)? {
                    edits.push(edit);
                }
            }
        }
        TaskEditIntent::SetPriority { priority } => edits.push_if_some(attribute_edit(
            source,
            occurrence,
            "priority",
            priority.map(priority_text),
        )?),
        TaskEditIntent::SetPhase { phase } => edits.push_if_some(attribute_edit(
            source,
            occurrence,
            "phase",
            phase.map(phase_text),
        )?),
        TaskEditIntent::SetResolution { resolution } => edits.push_if_some(attribute_edit(
            source,
            occurrence,
            "resolution",
            resolution.map(resolution_text),
        )?),
        TaskEditIntent::SetDate { field, value } => edits.push_if_some(attribute_edit(
            source,
            occurrence,
            date_field_text(*field),
            value.as_ref().map(task_date_text),
        )?),
        TaskEditIntent::SetRecurrence { rrule, repeat_from } => {
            let encoded = rrule
                .as_ref()
                .map(|value| serde_json::to_string(value).expect("RRULE serialization"));
            edits.push_if_some(attribute_edit(
                source,
                occurrence,
                "rrule",
                encoded.as_deref(),
            )?);
            edits.push_if_some(attribute_edit(
                source,
                occurrence,
                "repeat-from",
                repeat_from.map(repeat_from_text),
            )?);
        }
    }
    Ok(edits)
}

trait PushIfSome<T> {
    fn push_if_some(&mut self, value: Option<T>);
}

impl<T> PushIfSome<T> for Vec<T> {
    fn push_if_some(&mut self, value: Option<T>) {
        if let Some(value) = value {
            self.push(value);
        }
    }
}

fn simple_task_edits(
    occurrence: &TaskOccurrence,
    intent: &TaskEditIntent,
    assigned_id: Option<TaskId>,
) -> Result<Vec<SourceChange>, TaskAuthoringFailure> {
    if matches!(intent, TaskEditIntent::Toggle) {
        return Ok(vec![SourceChange {
            range: to_usize_range(&occurrence.marker_range)?,
            replacement: if occurrence.state == TaskState::Open {
                "x".to_owned()
            } else {
                " ".to_owned()
            },
        }]);
    }
    let Some(id) = assigned_id else {
        return Ok(Vec::new());
    };
    if occurrence.description.is_empty() {
        return Err(failure(
            TaskAuthoringDiagnosticCode::InvalidIntent,
            "an empty simple checklist cannot be promoted to structured metadata",
            Some(occurrence.range.clone()),
        ));
    }
    let mut attributes = vec![format!("id={id}")];
    match intent {
        TaskEditIntent::SetPriority {
            priority: Some(priority),
        } => attributes.push(format!("priority={}", priority_text(*priority))),
        TaskEditIntent::SetPhase { phase: Some(phase) } => {
            attributes.push(format!("phase={}", phase_text(*phase)));
        }
        TaskEditIntent::SetResolution {
            resolution: Some(resolution),
        } => attributes.push(format!("resolution={}", resolution_text(*resolution))),
        TaskEditIntent::SetDate {
            field,
            value: Some(value),
        } => attributes.push(format!(
            "{}={}",
            date_field_text(*field),
            task_date_text(value)
        )),
        TaskEditIntent::SetRecurrence {
            rrule: Some(rrule),
            repeat_from: Some(repeat_from),
        } => {
            attributes.push(format!(
                "rrule={}",
                serde_json::to_string(rrule).expect("RRULE serialization")
            ));
            attributes.push(format!("repeat-from={}", repeat_from_text(*repeat_from)));
        }
        TaskEditIntent::Toggle
        | TaskEditIntent::SetPriority { priority: None }
        | TaskEditIntent::SetPhase { phase: None }
        | TaskEditIntent::SetResolution { resolution: None }
        | TaskEditIntent::SetDate { value: None, .. }
        | TaskEditIntent::SetRecurrence { .. } => {}
    }
    Ok(vec![SourceChange {
        range: to_usize_range(&occurrence.range)?.end..to_usize_range(&occurrence.range)?.end,
        replacement: format!(" task:[{}]", attributes.join(",")),
    }])
}

pub(crate) fn attribute_edit(
    source: &str,
    occurrence: &TaskOccurrence,
    field: &str,
    value: Option<&str>,
) -> Result<Option<SourceChange>, TaskAuthoringFailure> {
    let metadata = occurrence.metadata.as_ref().ok_or_else(|| {
        failure(
            TaskAuthoringDiagnosticCode::InvalidSource,
            "structured task metadata is unavailable",
            Some(occurrence.range.clone()),
        )
    })?;
    let mut attributes = metadata.attributes.iter().collect::<Vec<_>>();
    attributes.sort_by_key(|attribute| (attribute.range.start, attribute.range.end));
    if let Some((index, attribute)) = attributes
        .iter()
        .enumerate()
        .find(|(_, attribute)| attribute.name == field)
    {
        if let Some(value) = value {
            if attribute.value == value
                || source
                    .get(to_usize_range(&attribute.value_range)?)
                    .is_some_and(|raw| raw == value)
            {
                return Ok(None);
            }
            return Ok(Some(SourceChange {
                range: to_usize_range(&attribute.value_range)?,
                replacement: value.to_owned(),
            }));
        }
        if field == "id" {
            return Err(failure(
                TaskAuthoringDiagnosticCode::InvalidIntent,
                "task identity cannot be removed by a metadata edit",
                Some(attribute.range.clone()),
            ));
        }
        let range =
            if let Some(previous) = index.checked_sub(1).and_then(|index| attributes.get(index)) {
                previous.range.end..attribute.range.end
            } else if let Some(next) = attributes.get(index + 1) {
                attribute.range.start..next.range.start
            } else {
                return Err(failure(
                    TaskAuthoringDiagnosticCode::InvalidSource,
                    "structured task metadata must retain its ID attribute",
                    Some(attribute.range.clone()),
                ));
            };
        return Ok(Some(SourceChange {
            range: to_usize_range(&range)?,
            replacement: String::new(),
        }));
    }
    let Some(value) = value else {
        return Ok(None);
    };
    let insert = metadata.range.end.checked_sub(1).ok_or_else(|| {
        failure(
            TaskAuthoringDiagnosticCode::InvalidSource,
            "structured task metadata range is invalid",
            Some(metadata.range.clone()),
        )
    })?;
    let insert = usize::try_from(insert).map_err(|_| invalid_result(metadata.range.clone()))?;
    Ok(Some(SourceChange {
        range: insert..insert,
        replacement: format!(",{field}={value}"),
    }))
}

pub(crate) fn apply_edits(
    source: &str,
    edits: &[SourceChange],
) -> Result<String, TaskAuthoringFailure> {
    let mut proposed = source.to_owned();
    for edit in edits.iter().rev() {
        if edit.range.start > edit.range.end
            || edit.range.end > proposed.len()
            || !proposed.is_char_boundary(edit.range.start)
            || !proposed.is_char_boundary(edit.range.end)
        {
            return Err(invalid_result(to_u64_range(edit.range.clone())));
        }
        proposed.replace_range(edit.range.clone(), &edit.replacement);
    }
    Ok(proposed)
}

fn select_result_target<'a>(
    analysis: &'a TaskSourceAnalysis,
    previous: &TaskOccurrence,
    assigned_id: Option<TaskId>,
) -> Result<&'a TaskOccurrence, TaskAuthoringFailure> {
    let existing_id = previous.metadata.as_ref().map(|metadata| metadata.id);
    let id = assigned_id.or(existing_id);
    let candidates = analysis
        .tasks
        .iter()
        .filter(|task| {
            id.map_or(task.range.start == previous.range.start, |id| {
                task.metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata.id == id)
            })
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [task] if task.valid => Ok(task),
        _ => {
            let diagnostic = analysis.diagnostics.iter().find(|diagnostic| {
                ranges_overlap(&diagnostic.range, &(previous.range.start..u64::MAX))
            });
            Err(failure(
                TaskAuthoringDiagnosticCode::InvalidResult,
                diagnostic.map_or_else(
                    || "proposed task source does not contain one valid edited target".to_owned(),
                    |diagnostic| diagnostic.message.clone(),
                ),
                diagnostic.map(|diagnostic| diagnostic.range.clone()),
            ))
        }
    }
}

pub(crate) fn minimal_edit(source: &str, proposed: &str) -> DocumentEdit {
    let mut prefix = source
        .bytes()
        .zip(proposed.bytes())
        .take_while(|(left, right)| left == right)
        .count();
    while !source.is_char_boundary(prefix) || !proposed.is_char_boundary(prefix) {
        prefix -= 1;
    }
    let source_tail = &source[prefix..];
    let proposed_tail = &proposed[prefix..];
    let mut suffix = source_tail
        .bytes()
        .rev()
        .zip(proposed_tail.bytes().rev())
        .take_while(|(left, right)| left == right)
        .count()
        .min(source_tail.len().min(proposed_tail.len()));
    while suffix > 0
        && (!source.is_char_boundary(source.len() - suffix)
            || !proposed.is_char_boundary(proposed.len() - suffix))
    {
        suffix -= 1;
    }
    DocumentEdit {
        start: to_u64(prefix),
        end: to_u64(source.len() - suffix),
        replacement: proposed[prefix..proposed.len() - suffix].to_owned(),
    }
}

fn task_date_text(value: &TaskDateTime) -> &str {
    match value {
        TaskDateTime::Date(value) | TaskDateTime::Instant(value) => value,
    }
}

const fn date_field_text(field: TaskDateField) -> &'static str {
    match field {
        TaskDateField::Created => "created",
        TaskDateField::Start => "start",
        TaskDateField::Scheduled => "scheduled",
        TaskDateField::Due => "due",
        TaskDateField::Closed => "closed",
    }
}

const fn priority_text(value: TaskPriority) -> &'static str {
    match value {
        TaskPriority::Lowest => "lowest",
        TaskPriority::Low => "low",
        TaskPriority::Normal => "normal",
        TaskPriority::Medium => "medium",
        TaskPriority::High => "high",
        TaskPriority::Highest => "highest",
    }
}

const fn phase_text(value: TaskPhase) -> &'static str {
    match value {
        TaskPhase::Todo => "todo",
        TaskPhase::InProgress => "in-progress",
        TaskPhase::OnHold => "on-hold",
    }
}

const fn resolution_text(value: TaskResolution) -> &'static str {
    match value {
        TaskResolution::Completed => "completed",
        TaskResolution::Cancelled => "cancelled",
    }
}

const fn repeat_from_text(value: TaskRepeatFrom) -> &'static str {
    match value {
        TaskRepeatFrom::Due => "due",
        TaskRepeatFrom::Scheduled => "scheduled",
        TaskRepeatFrom::Completion => "completion",
    }
}

fn target_range(target: &TaskEditTarget) -> Option<Range<u64>> {
    match target {
        TaskEditTarget::Occurrence { range } => Some(range.clone()),
        TaskEditTarget::Id { .. } => None,
    }
}

fn to_usize_range(range: &Range<u64>) -> Result<Range<usize>, TaskAuthoringFailure> {
    let start = usize::try_from(range.start).map_err(|_| invalid_result(range.clone()))?;
    let end = usize::try_from(range.end).map_err(|_| invalid_result(range.clone()))?;
    Ok(start..end)
}

fn ranges_overlap(left: &Range<u64>, right: &Range<u64>) -> bool {
    left.start < right.end && right.start < left.end
}

fn invalid_result(range: Range<u64>) -> TaskAuthoringFailure {
    failure(
        TaskAuthoringDiagnosticCode::InvalidResult,
        "proposed task edit contains an invalid UTF-8 source range",
        Some(range),
    )
}

fn failure(
    code: TaskAuthoringDiagnosticCode,
    message: impl Into<String>,
    range: Option<Range<u64>>,
) -> TaskAuthoringFailure {
    TaskAuthoringFailure {
        code,
        message: message.into(),
        range,
    }
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn to_u64_range(range: Range<usize>) -> Range<u64> {
    to_u64(range.start)..to_u64(range.end)
}
