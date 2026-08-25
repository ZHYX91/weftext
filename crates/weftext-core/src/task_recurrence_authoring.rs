use std::cmp::Ordering;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::query_workspace::{
    compare_temporal, offset_temporal, parse_date_ordinal, temporal_day_ordinal,
};
use crate::task_authoring::{SourceChange, apply_edits, attribute_edit, minimal_edit};
use crate::{
    DocumentEdit, DocumentRevision, TaskDateTime, TaskEditTarget, TaskId, TaskOccurrence,
    TaskRecurrence, TaskRecurrenceFrequency, TaskRepeatFrom, TaskSourceAnalysis, TaskState,
    analyze_task_source,
};

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecurrenceCompletionContext {
    pub completed_at: TaskDateTime,
    pub utc_offset_minutes: i16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecurrenceCompletionPlan {
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub edit: DocumentEdit,
    pub proposed_source: String,
    pub completed_task: TaskOccurrence,
    pub next_task: Option<TaskOccurrence>,
    pub next_task_id: Option<TaskId>,
    pub stopped: bool,
    pub analysis: TaskSourceAnalysis,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRecurrenceCompletionDiagnosticCode {
    InvalidTarget,
    AmbiguousTarget,
    InvalidSource,
    InvalidContext,
    NotOpenRecurringTask,
    UnrepresentableNextOccurrence,
    InvalidResult,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecurrenceCompletionFailure {
    pub code: TaskRecurrenceCompletionDiagnosticCode,
    pub message: String,
    pub range: Option<Range<u64>>,
}

impl std::fmt::Display for TaskRecurrenceCompletionFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TaskRecurrenceCompletionFailure {}

/// Plans atomic completion of one recurring task occurrence and its optional successor.
///
/// The current occurrence is closed with an explicit completion timestamp. `COUNT=1` and a next
/// anchor beyond `UNTIL` stop the series; otherwise a fresh task UUID and canonical open successor
/// are inserted immediately after the current item. Dates move by one shared calendar-day delta,
/// and the selected recurrence anchor is set to the exact computed next value.
///
/// # Errors
///
/// Returns a fail-closed diagnostic for an unavailable target, forged time context, non-open or
/// non-recurring source, exhaustion of the supported calendar range, or a proposed result rejected
/// by the canonical parser.
#[allow(clippy::too_many_lines)]
pub fn plan_task_recurrence_completion(
    source: &str,
    target: &TaskEditTarget,
    context: &TaskRecurrenceCompletionContext,
) -> Result<TaskRecurrenceCompletionPlan, TaskRecurrenceCompletionFailure> {
    validate_context(context)?;
    let current_analysis = analyze_task_source(source);
    let current = select_target(&current_analysis, target)?;
    if current.state != TaskState::Open {
        return Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::NotOpenRecurringTask,
            "recurrence completion requires an open task",
            Some(current.range.clone()),
        ));
    }
    let metadata = current.metadata.as_ref().ok_or_else(|| {
        completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::NotOpenRecurringTask,
            "recurrence completion requires structured task metadata",
            Some(current.range.clone()),
        )
    })?;
    let recurrence = metadata.recurrence.as_ref().ok_or_else(|| {
        completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::NotOpenRecurringTask,
            "task has no recurrence rule",
            Some(current.range.clone()),
        )
    })?;
    let repeat_from = metadata.repeat_from.ok_or_else(|| {
        completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidSource,
            "recurring task has no repeat-from value",
            Some(current.range.clone()),
        )
    })?;
    let anchor = match repeat_from {
        TaskRepeatFrom::Due => metadata.due.as_ref(),
        TaskRepeatFrom::Scheduled => metadata.scheduled.as_ref(),
        TaskRepeatFrom::Completion => Some(&context.completed_at),
    }
    .ok_or_else(|| {
        completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidSource,
            "recurrence anchor is absent from the target task",
            Some(current.range.clone()),
        )
    })?;

    let next_anchor = if recurrence.count == Some(1) {
        None
    } else {
        let candidate = next_recurrence_value(anchor, recurrence)?;
        if recurrence.until.as_ref().is_some_and(|until| {
            compare_temporal(&candidate, until, context.utc_offset_minutes)
                == Some(Ordering::Greater)
        }) {
            None
        } else {
            Some(candidate)
        }
    };

    let mut edits = vec![SourceChange {
        range: to_usize_range(&current.marker_range)?,
        replacement: "x".to_owned(),
    }];
    if let Some(edit) = attribute_edit(source, current, "phase", None).map_err(map_authoring)? {
        edits.push(edit);
    }
    if let Some(edit) =
        attribute_edit(source, current, "resolution", Some("completed")).map_err(map_authoring)?
    {
        edits.push(edit);
    }
    let completed_at = temporal_text(&context.completed_at);
    if let Some(edit) =
        attribute_edit(source, current, "closed", Some(completed_at)).map_err(map_authoring)?
    {
        edits.push(edit);
    }

    let next_task_id = next_anchor.as_ref().map(|_| TaskId::new());
    if let (Some(next_anchor), Some(next_task_id)) = (&next_anchor, next_task_id) {
        let next_dates = shifted_dates(
            metadata,
            repeat_from,
            anchor,
            next_anchor,
            context.utc_offset_minutes,
        )?;
        let next_rrule = next_rrule(recurrence);
        let next_line = render_next_task(
            current,
            metadata,
            next_task_id,
            &next_dates,
            &context.completed_at,
            &next_rrule,
            repeat_from,
        );
        let insert = usize::try_from(current.range.end).map_err(|_| invalid_result())?;
        let ending = line_ending_after(source, insert);
        edits.push(SourceChange {
            range: insert..insert,
            replacement: format!("{ending}{next_line}"),
        });
    }
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
    if edits
        .windows(2)
        .any(|pair| pair[0].range.end > pair[1].range.start)
    {
        return Err(invalid_result());
    }
    let proposed_source = apply_edits(source, &edits).map_err(map_authoring)?;
    let analysis = analyze_task_source(&proposed_source);
    let completed_task = unique_task(&analysis, metadata.id, TaskState::Closed)?;
    let next_task = next_task_id
        .map(|id| unique_task(&analysis, id, TaskState::Open).cloned())
        .transpose()?;
    let stopped = next_task_id.is_none();
    Ok(TaskRecurrenceCompletionPlan {
        base_revision: DocumentRevision::from_source(source),
        next_revision: DocumentRevision::from_source(&proposed_source),
        edit: minimal_edit(source, &proposed_source),
        proposed_source,
        completed_task: completed_task.clone(),
        next_task,
        next_task_id,
        stopped,
        analysis,
    })
}

fn validate_context(
    context: &TaskRecurrenceCompletionContext,
) -> Result<(), TaskRecurrenceCompletionFailure> {
    if !(-1_439..=1_439).contains(&context.utc_offset_minutes)
        || crate::task::parse_task_date_time(temporal_text(&context.completed_at))
            != Some(context.completed_at.clone())
    {
        return Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidContext,
            "completion time must be a valid date or explicit-offset instant and offset must be within -23:59..+23:59",
            None,
        ));
    }
    Ok(())
}

fn select_target<'a>(
    analysis: &'a TaskSourceAnalysis,
    target: &TaskEditTarget,
) -> Result<&'a TaskOccurrence, TaskRecurrenceCompletionFailure> {
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
        [] => Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidTarget,
            "recurrence target is not an exact current task",
            target_range(target),
        )),
        [task] if task.valid => Ok(task),
        [task] => Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidSource,
            "recurrence target source is invalid",
            Some(task.range.clone()),
        )),
        _ => Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::AmbiguousTarget,
            "recurrence target resolves to multiple tasks",
            target_range(target),
        )),
    }
}

fn next_recurrence_value(
    anchor: &TaskDateTime,
    recurrence: &TaskRecurrence,
) -> Result<TaskDateTime, TaskRecurrenceCompletionFailure> {
    if recurrence.frequency == TaskRecurrenceFrequency::Daily {
        return offset_temporal(anchor, i32::from(recurrence.interval))
            .ok_or_else(invalid_expansion);
    }
    let anchor_ordinal = anchor_ordinal(anchor)?;
    let anchor_date = temporal_date(anchor)?;
    let mut candidate = anchor_date;
    let mut candidate_ordinal = anchor_ordinal;
    while let Some((year, month, day)) = next_calendar_date(candidate) {
        candidate = (year, month, day);
        candidate_ordinal += 1;
        if recurrence_date_matches(
            candidate_ordinal,
            (year, month, day),
            anchor_ordinal,
            anchor_date,
            recurrence,
        )? {
            return with_temporal_date(anchor, year, month, day).ok_or_else(invalid_expansion);
        }
    }
    Err(invalid_expansion())
}

fn recurrence_date_matches(
    candidate_ordinal: i64,
    candidate: (i32, u8, u8),
    anchor_ordinal: i64,
    anchor: (i32, u8, u8),
    recurrence: &TaskRecurrence,
) -> Result<bool, TaskRecurrenceCompletionFailure> {
    let interval = i64::from(recurrence.interval);
    let (year, month, day) = candidate;
    let (anchor_year, anchor_month, anchor_day) = anchor;
    let candidate_weekday = candidate_ordinal.rem_euclid(7);
    let anchor_weekday = anchor_ordinal.rem_euclid(7);
    let mut by_day_matches = recurrence.by_day.is_empty();
    for value in &recurrence.by_day {
        by_day_matches |= i64::from(weekday_number(value)?) == candidate_weekday;
    }
    let by_month_day_matches = recurrence.by_month_day.is_empty()
        || recurrence
            .by_month_day
            .iter()
            .any(|value| month_day_matches(year, month, day, *value));

    match recurrence.frequency {
        TaskRecurrenceFrequency::Daily => Ok(false),
        TaskRecurrenceFrequency::Weekly => {
            let week_start = i64::from(weekday_number(
                recurrence.week_start.as_deref().unwrap_or("MO"),
            )?);
            let anchor_week = anchor_ordinal - (anchor_weekday - week_start).rem_euclid(7);
            let candidate_week = candidate_ordinal - (candidate_weekday - week_start).rem_euclid(7);
            let week_distance = (candidate_week - anchor_week).div_euclid(7);
            Ok(week_distance.rem_euclid(interval) == 0
                && (recurrence.by_day.is_empty() && candidate_weekday == anchor_weekday
                    || !recurrence.by_day.is_empty() && by_day_matches))
        }
        TaskRecurrenceFrequency::Monthly => {
            let month_distance =
                i64::from(year - anchor_year) * 12 + i64::from(month) - i64::from(anchor_month);
            let default_day_matches = !recurrence.by_day.is_empty()
                || !recurrence.by_month_day.is_empty()
                || day == anchor_day;
            Ok(month_distance.rem_euclid(interval) == 0
                && default_day_matches
                && by_day_matches
                && by_month_day_matches)
        }
        TaskRecurrenceFrequency::Yearly => {
            let year_distance = i64::from(year - anchor_year);
            let month_matches = if recurrence.by_month.is_empty() {
                !recurrence.by_day.is_empty()
                    || !recurrence.by_month_day.is_empty()
                    || month == anchor_month
            } else {
                recurrence.by_month.contains(&month)
            };
            let default_day_matches = !recurrence.by_day.is_empty()
                || !recurrence.by_month_day.is_empty()
                || day == anchor_day;
            Ok(year_distance.rem_euclid(interval) == 0
                && month_matches
                && default_day_matches
                && by_day_matches
                && by_month_day_matches)
        }
    }
}

fn month_day_matches(year: i32, month: u8, day: u8, expected: i8) -> bool {
    if expected > 0 {
        i16::from(day) == i16::from(expected)
    } else {
        i16::from(day) == i16::from(month_days(year, month)) + i16::from(expected) + 1
    }
}

fn next_calendar_date((year, month, day): (i32, u8, u8)) -> Option<(i32, u8, u8)> {
    if day < month_days(year, month) {
        Some((year, month, day + 1))
    } else if month < 12 {
        Some((year, month + 1, 1))
    } else if year < 9_999 {
        Some((year + 1, 1, 1))
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct ShiftedDates {
    created: Option<TaskDateTime>,
    start: Option<TaskDateTime>,
    scheduled: Option<TaskDateTime>,
    due: Option<TaskDateTime>,
}

fn shifted_dates(
    metadata: &crate::TaskMetadata,
    repeat_from: TaskRepeatFrom,
    anchor: &TaskDateTime,
    next_anchor: &TaskDateTime,
    utc_offset_minutes: i16,
) -> Result<ShiftedDates, TaskRecurrenceCompletionFailure> {
    let old_reference = match repeat_from {
        TaskRepeatFrom::Due | TaskRepeatFrom::Scheduled => anchor,
        TaskRepeatFrom::Completion => metadata
            .due
            .as_ref()
            .or(metadata.scheduled.as_ref())
            .or(metadata.start.as_ref())
            .unwrap_or(anchor),
    };
    let old_day =
        temporal_day_ordinal(old_reference, utc_offset_minutes).ok_or_else(invalid_expansion)?;
    let next_day =
        temporal_day_ordinal(next_anchor, utc_offset_minutes).ok_or_else(invalid_expansion)?;
    let delta = i32::try_from(next_day - old_day).map_err(|_| invalid_expansion())?;
    let mut dates = ShiftedDates {
        created: metadata.created.clone(),
        start: shift_optional(metadata.start.as_ref(), delta)?,
        scheduled: shift_optional(metadata.scheduled.as_ref(), delta)?,
        due: shift_optional(metadata.due.as_ref(), delta)?,
    };
    match repeat_from {
        TaskRepeatFrom::Due => dates.due = Some(next_anchor.clone()),
        TaskRepeatFrom::Scheduled => dates.scheduled = Some(next_anchor.clone()),
        TaskRepeatFrom::Completion => {}
    }
    Ok(dates)
}

fn shift_optional(
    value: Option<&TaskDateTime>,
    days: i32,
) -> Result<Option<TaskDateTime>, TaskRecurrenceCompletionFailure> {
    value
        .map(|value| offset_temporal(value, days).ok_or_else(invalid_expansion))
        .transpose()
}

#[allow(clippy::too_many_arguments)]
fn render_next_task(
    current: &TaskOccurrence,
    metadata: &crate::TaskMetadata,
    next_task_id: TaskId,
    dates: &ShiftedDates,
    completed_at: &TaskDateTime,
    rrule: &str,
    repeat_from: TaskRepeatFrom,
) -> String {
    let mut attributes = vec![format!("id={next_task_id}")];
    if metadata
        .attributes
        .iter()
        .any(|attribute| attribute.name == "priority")
    {
        attributes.push(format!("priority={}", priority_text(metadata.priority)));
    }
    let created = metadata
        .attributes
        .iter()
        .any(|attribute| attribute.name == "created")
        .then_some(completed_at)
        .or(dates.created.as_ref());
    for (name, value) in [
        ("created", created),
        ("start", dates.start.as_ref()),
        ("scheduled", dates.scheduled.as_ref()),
        ("due", dates.due.as_ref()),
    ] {
        if let Some(value) = value {
            attributes.push(format!("{name}={}", temporal_text(value)));
        }
    }
    attributes.push(format!(
        "rrule={}",
        serde_json::to_string(rrule).expect("RRULE serialization")
    ));
    attributes.push(format!("repeat-from={}", repeat_from_text(repeat_from)));
    if !metadata.dependencies.is_empty() {
        attributes.push(format!(
            "depends-on={}",
            serde_json::to_string(
                &metadata
                    .dependencies
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .expect("dependency serialization")
        ));
    }
    format!(
        "{} [ ] {} task:[{}]",
        "*".repeat(usize::from(current.list_depth)),
        current.description,
        attributes.join(",")
    )
}

fn next_rrule(recurrence: &TaskRecurrence) -> String {
    let mut parts = vec![format!(
        "FREQ={}",
        match recurrence.frequency {
            TaskRecurrenceFrequency::Daily => "DAILY",
            TaskRecurrenceFrequency::Weekly => "WEEKLY",
            TaskRecurrenceFrequency::Monthly => "MONTHLY",
            TaskRecurrenceFrequency::Yearly => "YEARLY",
        }
    )];
    if recurrence.interval != 1 {
        parts.push(format!("INTERVAL={}", recurrence.interval));
    }
    if !recurrence.by_day.is_empty() {
        parts.push(format!("BYDAY={}", recurrence.by_day.join(",")));
    }
    if !recurrence.by_month_day.is_empty() {
        parts.push(format!(
            "BYMONTHDAY={}",
            recurrence
                .by_month_day
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if !recurrence.by_month.is_empty() {
        parts.push(format!(
            "BYMONTH={}",
            recurrence
                .by_month
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        ));
    }
    if let Some(count) = recurrence.count {
        parts.push(format!("COUNT={}", count - 1));
    }
    if let Some(until) = &recurrence.until {
        parts.push(format!("UNTIL={}", temporal_text(until)));
    }
    if let Some(week_start) = &recurrence.week_start {
        parts.push(format!("WKST={week_start}"));
    }
    parts.join(";")
}

fn unique_task(
    analysis: &TaskSourceAnalysis,
    id: TaskId,
    state: TaskState,
) -> Result<&TaskOccurrence, TaskRecurrenceCompletionFailure> {
    let candidates = analysis
        .tasks
        .iter()
        .filter(|task| {
            task.valid
                && task.state == state
                && task
                    .metadata
                    .as_ref()
                    .is_some_and(|metadata| metadata.id == id)
        })
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [task] => Ok(task),
        _ => Err(completion_failure(
            TaskRecurrenceCompletionDiagnosticCode::InvalidResult,
            "recurrence result does not contain one valid expected occurrence",
            None,
        )),
    }
}

fn anchor_ordinal(value: &TaskDateTime) -> Result<i64, TaskRecurrenceCompletionFailure> {
    let (year, month, day) = temporal_date(value)?;
    parse_date_ordinal(&format!("{year:04}-{month:02}-{day:02}")).ok_or_else(invalid_expansion)
}

fn temporal_date(value: &TaskDateTime) -> Result<(i32, u8, u8), TaskRecurrenceCompletionFailure> {
    let value = temporal_text(value);
    let year = value[0..4]
        .parse::<i32>()
        .map_err(|_| invalid_expansion())?;
    let month = value[5..7].parse::<u8>().map_err(|_| invalid_expansion())?;
    let day = value[8..10]
        .parse::<u8>()
        .map_err(|_| invalid_expansion())?;
    Ok((year, month, day))
}

fn with_temporal_date(
    template: &TaskDateTime,
    year: i32,
    month: u8,
    day: u8,
) -> Option<TaskDateTime> {
    crate::CalendarDate::new(year, month, day).ok()?;
    Some(match template {
        TaskDateTime::Date(_) => TaskDateTime::Date(format!("{year:04}-{month:02}-{day:02}")),
        TaskDateTime::Instant(source) => {
            TaskDateTime::Instant(format!("{year:04}-{month:02}-{day:02}{}", &source[10..]))
        }
    })
}

fn month_days(year: i32, month: u8) -> u8 {
    (28..=31)
        .rev()
        .find(|day| crate::CalendarDate::new(year, month, *day).is_ok())
        .unwrap_or(28)
}

fn weekday_number(value: &str) -> Result<i32, TaskRecurrenceCompletionFailure> {
    match value {
        "MO" => Ok(0),
        "TU" => Ok(1),
        "WE" => Ok(2),
        "TH" => Ok(3),
        "FR" => Ok(4),
        "SA" => Ok(5),
        "SU" => Ok(6),
        _ => Err(invalid_expansion()),
    }
}

const fn priority_text(value: crate::TaskPriority) -> &'static str {
    match value {
        crate::TaskPriority::Lowest => "lowest",
        crate::TaskPriority::Low => "low",
        crate::TaskPriority::Normal => "normal",
        crate::TaskPriority::Medium => "medium",
        crate::TaskPriority::High => "high",
        crate::TaskPriority::Highest => "highest",
    }
}

const fn repeat_from_text(value: TaskRepeatFrom) -> &'static str {
    match value {
        TaskRepeatFrom::Due => "due",
        TaskRepeatFrom::Scheduled => "scheduled",
        TaskRepeatFrom::Completion => "completion",
    }
}

fn temporal_text(value: &TaskDateTime) -> &str {
    match value {
        TaskDateTime::Date(value) | TaskDateTime::Instant(value) => value,
    }
}

fn line_ending_after(source: &str, offset: usize) -> &str {
    match source.as_bytes().get(offset..) {
        Some([b'\r', b'\n', ..]) => "\r\n",
        Some([b'\r', ..]) => "\r",
        _ => "\n",
    }
}

fn target_range(target: &TaskEditTarget) -> Option<Range<u64>> {
    match target {
        TaskEditTarget::Occurrence { range } => Some(range.clone()),
        TaskEditTarget::Id { .. } => None,
    }
}

fn to_usize_range(range: &Range<u64>) -> Result<Range<usize>, TaskRecurrenceCompletionFailure> {
    Ok(usize::try_from(range.start).map_err(|_| invalid_result())?
        ..usize::try_from(range.end).map_err(|_| invalid_result())?)
}

fn map_authoring(error: crate::TaskAuthoringFailure) -> TaskRecurrenceCompletionFailure {
    completion_failure(
        TaskRecurrenceCompletionDiagnosticCode::InvalidResult,
        error.message,
        error.range,
    )
}

fn invalid_expansion() -> TaskRecurrenceCompletionFailure {
    completion_failure(
        TaskRecurrenceCompletionDiagnosticCode::UnrepresentableNextOccurrence,
        "recurrence has no representable next value within the supported calendar range",
        None,
    )
}

fn invalid_result() -> TaskRecurrenceCompletionFailure {
    completion_failure(
        TaskRecurrenceCompletionDiagnosticCode::InvalidResult,
        "recurrence completion produced invalid source",
        None,
    )
}

fn completion_failure(
    code: TaskRecurrenceCompletionDiagnosticCode,
    message: impl Into<String>,
    range: Option<Range<u64>>,
) -> TaskRecurrenceCompletionFailure {
    TaskRecurrenceCompletionFailure {
        code,
        message: message.into(),
        range,
    }
}
