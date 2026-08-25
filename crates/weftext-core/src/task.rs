use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::CalendarDate;
use crate::source_lexing::{
    decode_attribute_value, find_closing_bracket, find_unquoted_equals, line_end,
    split_comma_parts, trim_range,
};

pub const TASK_PROFILE_ID: &str = "weftext.tasks.v1";

pub const TASK_METADATA_FIELDS: &[&str] = &[
    "id",
    "phase",
    "resolution",
    "priority",
    "created",
    "start",
    "scheduled",
    "due",
    "closed",
    "rrule",
    "repeat-from",
    "depends-on",
];
const DATE_FIELDS: &[&str] = &["created", "start", "scheduled", "due", "closed"];
const RRULE_PARTS: &[&str] = &[
    "FREQ",
    "INTERVAL",
    "BYDAY",
    "BYMONTHDAY",
    "BYMONTH",
    "COUNT",
    "UNTIL",
    "WKST",
];
const WEEKDAYS: &[&str] = &["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
const MAX_LEGACY_TASK_MACRO_EVIDENCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_LEGACY_DEPENDENCY_DECODED_BYTES: usize =
    weftext_asciidoc::MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES;
const MAX_LEGACY_DEPENDENCY_TOKENS: usize = 110;
const MAX_LEGACY_UNIQUE_DEPENDENCIES: usize = 110;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TaskId(Uuid);

impl TaskId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl FromStr for TaskId {
    type Err = TaskIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = Uuid::parse_str(value).map_err(|_| TaskIdError)?;
        if uuid.get_version_num() != 4 || uuid.to_string() != value {
            return Err(TaskIdError);
        }
        Ok(Self(uuid))
    }
}

impl Serialize for TaskId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for TaskId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::from_str(&value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TaskIdError;

impl fmt::Display for TaskIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("task ID must be a lowercase UUIDv4")
    }
}

impl std::error::Error for TaskIdError {}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Open,
    Closed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskPhase {
    Todo,
    InProgress,
    OnHold,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskResolution {
    Completed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Lowest,
    Low,
    Normal,
    Medium,
    High,
    Highest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum TaskDateTime {
    Date(String),
    Instant(String),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRepeatFrom {
    Due,
    Scheduled,
    Completion,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum TaskRecurrenceFrequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecurrence {
    pub source: String,
    pub frequency: TaskRecurrenceFrequency,
    pub interval: u16,
    pub by_day: Vec<String>,
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<u8>,
    pub count: Option<u16>,
    pub until: Option<TaskDateTime>,
    pub week_start: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskAttribute {
    pub name: String,
    pub value: String,
    pub range: Range<u64>,
    pub name_range: Range<u64>,
    pub value_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskMetadata {
    pub id: TaskId,
    pub phase: Option<TaskPhase>,
    pub resolution: Option<TaskResolution>,
    pub priority: TaskPriority,
    pub created: Option<TaskDateTime>,
    pub start: Option<TaskDateTime>,
    pub scheduled: Option<TaskDateTime>,
    pub due: Option<TaskDateTime>,
    pub closed: Option<TaskDateTime>,
    pub recurrence: Option<TaskRecurrence>,
    pub repeat_from: Option<TaskRepeatFrom>,
    pub dependencies: Vec<TaskId>,
    pub range: Range<u64>,
    pub attributes: Vec<TaskAttribute>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOccurrence {
    pub state: TaskState,
    pub authored_marker: String,
    pub description: String,
    pub list_depth: u16,
    pub range: Range<u64>,
    pub marker_range: Range<u64>,
    pub description_range: Range<u64>,
    pub metadata: Option<TaskMetadata>,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskDiagnosticCode {
    MalformedMacro,
    DuplicateMacro,
    UnknownAttribute,
    DuplicateAttribute,
    InvalidAttributeName,
    InvalidAttributeValue,
    MissingId,
    InvalidId,
    DuplicateId,
    InvalidPhase,
    InvalidResolution,
    InvalidPriority,
    InvalidDateTime,
    InvalidStateCombination,
    InvalidRecurrence,
    MissingRepeatFrom,
    UnexpectedRepeatFrom,
    MissingRecurrenceAnchor,
    InvalidDependency,
    DuplicateDependency,
    SelfDependency,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDiagnostic {
    pub code: TaskDiagnosticCode,
    pub message: String,
    pub range: Range<u64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSourceAnalysis {
    pub tasks: Vec<TaskOccurrence>,
    pub diagnostics: Vec<TaskDiagnostic>,
}

/// Parses native `AsciiDoc` checklist items and their optional trailing Weftext task metadata.
///
/// The parser reads exact source and returns UTF-8 byte ranges. It shares the literal attribute
/// decoder used by canonical citation macros, ignores profile-protected ranges, and never assigns
/// an identity to a simple checklist as a side effect of reading it.
#[must_use]
pub fn analyze_task_source(source: &str) -> TaskSourceAnalysis {
    let protected = weftext_asciidoc::analyze(source).protected_ranges;
    let mut analysis = TaskSourceAnalysis::default();
    let mut start = 0_usize;
    while start < source.len() {
        let end = line_end(source, start, source.len());
        if !contains_offset(&protected, start)
            && let Some(mut task) = parse_task_line(source, start, end, &protected, &mut analysis)
        {
            task.valid = task.valid
                && !analysis
                    .diagnostics
                    .iter()
                    .any(|diagnostic| ranges_overlap(&task.range, &diagnostic.range));
            analysis.tasks.push(task);
        }
        start = next_line_start(source, end);
    }
    diagnose_duplicate_ids(&mut analysis);
    analysis
}

#[allow(clippy::too_many_lines)]
fn parse_task_line(
    source: &str,
    line_start: usize,
    line_end: usize,
    protected: &[Range<u64>],
    analysis: &mut TaskSourceAnalysis,
) -> Option<TaskOccurrence> {
    let bytes = source.as_bytes();
    let mut cursor = line_start;
    while cursor < line_end && matches!(bytes[cursor], b' ' | b'\t') {
        cursor += 1;
    }
    let marker_start = cursor;
    let depth = if bytes.get(cursor) == Some(&b'*') {
        while cursor < line_end && bytes[cursor] == b'*' {
            cursor += 1;
        }
        u16::try_from(cursor - marker_start).unwrap_or(u16::MAX)
    } else if bytes.get(cursor) == Some(&b'-') {
        cursor += 1;
        1
    } else {
        return None;
    };
    if cursor >= line_end || !matches!(bytes[cursor], b' ' | b'\t') {
        return None;
    }
    while cursor < line_end && matches!(bytes[cursor], b' ' | b'\t') {
        cursor += 1;
    }
    if cursor + 3 > line_end || bytes[cursor] != b'[' || bytes[cursor + 2] != b']' {
        return None;
    }
    let (state, authored_marker) = match bytes[cursor + 1] {
        b' ' => (TaskState::Open, " "),
        b'x' => (TaskState::Closed, "x"),
        b'*' => (TaskState::Closed, "*"),
        _ => return None,
    };
    let marker_range = cursor + 1..cursor + 2;
    cursor += 3;
    if cursor < line_end && !matches!(bytes[cursor], b' ' | b'\t') {
        return None;
    }
    while cursor < line_end && matches!(bytes[cursor], b' ' | b'\t') {
        cursor += 1;
    }
    let content_start = cursor;
    let macro_starts = unprotected_macro_starts(source, content_start..line_end, protected);
    let mut valid = true;
    let mut metadata = None;
    let description_end;
    if macro_starts.is_empty() {
        description_end = trim_horizontal_end(source, content_start..line_end).end;
    } else {
        if macro_starts.len() > 1 {
            valid = false;
            for duplicate in macro_starts.iter().skip(1) {
                analysis.diagnostics.push(task_diagnostic(
                    TaskDiagnosticCode::DuplicateMacro,
                    "a checklist item may contain at most one task metadata macro",
                    *duplicate..(*duplicate + "task:[".len()).min(line_end),
                ));
            }
        }
        let macro_start = macro_starts[0];
        let open = macro_start + "task:".len();
        let Some(close) = find_closing_bracket(source, open, line_end) else {
            analysis.diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::MalformedMacro,
                "task metadata attribute list is not closed",
                macro_start..line_end,
            ));
            return Some(task_occurrence(
                state,
                authored_marker,
                depth,
                source,
                TaskOccurrenceRanges {
                    occurrence: line_start..line_end,
                    marker: marker_range,
                    description: content_start..line_end,
                },
                None,
                false,
            ));
        };
        let macro_end = close + 1;
        if !source[macro_end..line_end]
            .chars()
            .all(|character| matches!(character, ' ' | '\t'))
        {
            valid = false;
            analysis.diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::MalformedMacro,
                "task metadata macro must be the final non-whitespace element of its item line",
                macro_start..line_end,
            ));
        }
        let separator = trim_horizontal_end(source, content_start..macro_start);
        if separator.end == macro_start
            || !source.as_bytes()[separator.end..macro_start]
                .iter()
                .all(u8::is_ascii_whitespace)
        {
            valid = false;
            analysis.diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::MalformedMacro,
                "task metadata macro must be separated from the description by whitespace",
                macro_start..macro_end,
            ));
        }
        description_end = separator.end;
        let parsed = parse_task_metadata(source, state, macro_start, open, close);
        valid = valid && parsed.diagnostics.is_empty();
        analysis.diagnostics.extend(parsed.diagnostics);
        metadata = parsed.metadata;
    }
    Some(task_occurrence(
        state,
        authored_marker,
        depth,
        source,
        TaskOccurrenceRanges {
            occurrence: line_start..line_end,
            marker: marker_range,
            description: content_start..description_end,
        },
        metadata,
        valid,
    ))
}

struct TaskOccurrenceRanges {
    occurrence: Range<usize>,
    marker: Range<usize>,
    description: Range<usize>,
}

fn task_occurrence(
    state: TaskState,
    authored_marker: &str,
    list_depth: u16,
    source: &str,
    ranges: TaskOccurrenceRanges,
    metadata: Option<TaskMetadata>,
    valid: bool,
) -> TaskOccurrence {
    TaskOccurrence {
        state,
        authored_marker: authored_marker.to_owned(),
        description: source[ranges.description.clone()].to_owned(),
        list_depth,
        range: to_u64_range(ranges.occurrence),
        marker_range: to_u64_range(ranges.marker),
        description_range: to_u64_range(ranges.description),
        metadata,
        valid,
    }
}

struct ParsedTaskMetadata {
    id: Option<TaskId>,
    metadata: Option<TaskMetadata>,
    diagnostics: Vec<TaskDiagnostic>,
}

/// Extracts only a valid legacy task identity from one exact, closed `task:[...]` range.
///
/// This is conversion occupancy evidence, not a second task parser or runtime authority. Other
/// metadata fields may be invalid; callers must still block conversion unless the ordinary task
/// analysis and canonical `AsciiDoc` evidence agree.
pub(crate) fn legacy_task_id_from_closed_macro(
    source: &str,
    macro_range: &Range<u64>,
) -> Option<TaskId> {
    let start = usize::try_from(macro_range.start).ok()?;
    let end = usize::try_from(macro_range.end).ok()?;
    if start >= end
        || end.checked_sub(start)? > MAX_LEGACY_TASK_MACRO_EVIDENCE_BYTES
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
        || source.get(start..start.checked_add("task:[".len())?)? != "task:["
        || source.as_bytes().get(end - 1) != Some(&b']')
    {
        return None;
    }
    let open = start.checked_add("task:".len())?;
    let close = end - 1;
    if find_closing_bracket(source, open, end)? != close {
        return None;
    }
    parse_task_metadata(source, TaskState::Open, start, open, close).id
}

#[allow(clippy::too_many_lines)]
fn parse_task_metadata(
    source: &str,
    state: TaskState,
    macro_start: usize,
    open: usize,
    close: usize,
) -> ParsedTaskMetadata {
    let content = open + 1..close;
    let parts = match split_comma_parts(source, content.clone()) {
        Ok(parts) => parts,
        Err(range) => {
            return ParsedTaskMetadata {
                id: None,
                metadata: None,
                diagnostics: vec![task_diagnostic(
                    TaskDiagnosticCode::MalformedMacro,
                    "task metadata contains an invalid quoted value",
                    range,
                )],
            };
        }
    };
    let mut diagnostics = Vec::new();
    let mut names = BTreeSet::new();
    let mut values = BTreeMap::new();
    let mut attributes = Vec::new();
    for part in parts {
        let part = trim_range(source, part);
        let Some(equals) = find_unquoted_equals(source, part.clone()) else {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::InvalidAttributeName,
                "task metadata accepts named attributes only",
                part,
            ));
            continue;
        };
        let name_range = trim_range(source, part.start..equals);
        let value_range = trim_range(source, equals + 1..part.end);
        let name = &source[name_range.clone()];
        if !valid_attribute_name(name) {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::InvalidAttributeName,
                "task attribute names must be lowercase ASCII kebab-case",
                name_range,
            ));
            continue;
        }
        if !TASK_METADATA_FIELDS.contains(&name) {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::UnknownAttribute,
                format!("unknown task attribute `{name}`"),
                name_range,
            ));
            continue;
        }
        if !names.insert(name.to_owned()) {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::DuplicateAttribute,
                format!("duplicate task attribute `{name}`"),
                name_range,
            ));
            continue;
        }
        let raw = &source[value_range.clone()];
        let Some(value) = decode_attribute_value(source, value_range.clone()) else {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::InvalidAttributeValue,
                format!("task attribute `{name}` has an invalid literal value"),
                value_range,
            ));
            continue;
        };
        if matches!(name, "rrule" | "depends-on") && !raw.starts_with('"') {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::InvalidAttributeValue,
                format!("task attribute `{name}` must use a quoted string"),
                value_range,
            ));
            continue;
        }
        values.insert(name.to_owned(), (value.clone(), value_range.clone()));
        attributes.push(TaskAttribute {
            name: name.to_owned(),
            value,
            range: to_u64_range(part),
            name_range: to_u64_range(name_range),
            value_range: to_u64_range(value_range),
        });
    }
    let Some((id_source, id_range)) = values.get("id") else {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::MissingId,
            "structured task metadata requires `id`",
            content,
        ));
        return ParsedTaskMetadata {
            id: None,
            metadata: None,
            diagnostics,
        };
    };
    let Ok(id) = TaskId::from_str(id_source) else {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::InvalidId,
            "task ID must be a lowercase UUIDv4",
            id_range.clone(),
        ));
        return ParsedTaskMetadata {
            id: None,
            metadata: None,
            diagnostics,
        };
    };
    let phase = parse_enum_field(
        &values,
        "phase",
        &[
            ("todo", TaskPhase::Todo),
            ("in-progress", TaskPhase::InProgress),
            ("on-hold", TaskPhase::OnHold),
        ],
        TaskDiagnosticCode::InvalidPhase,
        &mut diagnostics,
    );
    let resolution = parse_enum_field(
        &values,
        "resolution",
        &[
            ("completed", TaskResolution::Completed),
            ("cancelled", TaskResolution::Cancelled),
        ],
        TaskDiagnosticCode::InvalidResolution,
        &mut diagnostics,
    );
    let priority = parse_enum_field(
        &values,
        "priority",
        &[
            ("highest", TaskPriority::Highest),
            ("high", TaskPriority::High),
            ("medium", TaskPriority::Medium),
            ("normal", TaskPriority::Normal),
            ("low", TaskPriority::Low),
            ("lowest", TaskPriority::Lowest),
        ],
        TaskDiagnosticCode::InvalidPriority,
        &mut diagnostics,
    )
    .unwrap_or(TaskPriority::Normal);
    let mut dates = BTreeMap::new();
    for field in DATE_FIELDS {
        if let Some((value, range)) = values.get(*field) {
            match parse_task_date_time(value) {
                Some(value) => {
                    dates.insert(*field, value);
                }
                None => diagnostics.push(task_diagnostic(
                    TaskDiagnosticCode::InvalidDateTime,
                    format!("task attribute `{field}` must be an ISO date or RFC 3339 instant with an explicit offset"),
                    range.clone(),
                )),
            }
        }
    }
    if state == TaskState::Open && (resolution.is_some() || dates.contains_key("closed")) {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::InvalidStateCombination,
            "an open task cannot have `resolution` or `closed`",
            macro_start..close + 1,
        ));
    }
    if state == TaskState::Closed && phase.is_some() {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::InvalidStateCombination,
            "a closed task cannot have `phase`",
            macro_start..close + 1,
        ));
    }
    let recurrence = values.get("rrule").and_then(|(value, range)| {
        parse_rrule(value).or_else(|| {
            diagnostics.push(task_diagnostic(
                TaskDiagnosticCode::InvalidRecurrence,
                "rrule is outside the deterministic Weftext v1 subset",
                range.clone(),
            ));
            None
        })
    });
    let repeat_from = parse_enum_field(
        &values,
        "repeat-from",
        &[
            ("due", TaskRepeatFrom::Due),
            ("scheduled", TaskRepeatFrom::Scheduled),
            ("completion", TaskRepeatFrom::Completion),
        ],
        TaskDiagnosticCode::InvalidRecurrence,
        &mut diagnostics,
    );
    if recurrence.is_some() && repeat_from.is_none() {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::MissingRepeatFrom,
            "rrule requires `repeat-from`",
            macro_start..close + 1,
        ));
    } else if recurrence.is_none() && repeat_from.is_some() {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::UnexpectedRepeatFrom,
            "repeat-from requires `rrule`",
            macro_start..close + 1,
        ));
    }
    if (repeat_from == Some(TaskRepeatFrom::Due) && !dates.contains_key("due"))
        || (repeat_from == Some(TaskRepeatFrom::Scheduled) && !dates.contains_key("scheduled"))
    {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::MissingRecurrenceAnchor,
            "the selected recurrence anchor is not present on the task",
            macro_start..close + 1,
        ));
    }
    let dependencies = values
        .get("depends-on")
        .map_or_else(Vec::new, |(value, range)| {
            parse_dependencies(value, id, range, &mut diagnostics)
        });
    let metadata = diagnostics.is_empty().then(|| TaskMetadata {
        id,
        phase,
        resolution,
        priority,
        created: dates.remove("created"),
        start: dates.remove("start"),
        scheduled: dates.remove("scheduled"),
        due: dates.remove("due"),
        closed: dates.remove("closed"),
        recurrence,
        repeat_from,
        dependencies,
        range: to_u64_range(macro_start..close + 1),
        attributes,
    });
    ParsedTaskMetadata {
        id: Some(id),
        metadata,
        diagnostics,
    }
}

fn parse_enum_field<T: Copy>(
    values: &BTreeMap<String, (String, Range<usize>)>,
    field: &str,
    variants: &[(&str, T)],
    code: TaskDiagnosticCode,
    diagnostics: &mut Vec<TaskDiagnostic>,
) -> Option<T> {
    let (value, range) = values.get(field)?;
    let parsed = variants
        .iter()
        .find_map(|(candidate, parsed)| (*candidate == value).then_some(*parsed));
    if parsed.is_none() {
        diagnostics.push(task_diagnostic(
            code,
            format!("unsupported task {field} `{value}`"),
            range.clone(),
        ));
    }
    parsed
}

fn parse_dependencies(
    value: &str,
    own_id: TaskId,
    range: &Range<usize>,
    diagnostics: &mut Vec<TaskDiagnostic>,
) -> Vec<TaskId> {
    if value.len() > MAX_LEGACY_DEPENDENCY_DECODED_BYTES {
        diagnostics.push(task_diagnostic(
            TaskDiagnosticCode::InvalidDependency,
            "depends-on exceeds the canonical task-node header value limit",
            range.clone(),
        ));
        return Vec::new();
    }
    let mut dependencies = Vec::new();
    let mut seen = BTreeSet::new();
    let mut invalid_reported = false;
    let mut self_reported = false;
    let mut duplicate_reported = false;
    for (index, raw) in value.split(' ').enumerate() {
        if index == MAX_LEGACY_DEPENDENCY_TOKENS {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut invalid_reported,
                TaskDiagnosticCode::InvalidDependency,
                "depends-on exceeds the decoded dependency token limit",
                range,
            );
            break;
        }
        if raw.is_empty() {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut invalid_reported,
                TaskDiagnosticCode::InvalidDependency,
                "depends-on uses single ASCII spaces between task UUIDs",
                range,
            );
            continue;
        }
        let Ok(id) = TaskId::from_str(raw) else {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut invalid_reported,
                TaskDiagnosticCode::InvalidDependency,
                "depends-on contains an invalid task UUID",
                range,
            );
            continue;
        };
        if id == own_id {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut self_reported,
                TaskDiagnosticCode::SelfDependency,
                "a task cannot depend on itself",
                range,
            );
        } else if seen.contains(&id) {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut duplicate_reported,
                TaskDiagnosticCode::DuplicateDependency,
                "depends-on contains the same task UUID more than once",
                range,
            );
        } else if seen.len() == MAX_LEGACY_UNIQUE_DEPENDENCIES {
            push_dependency_diagnostic_once(
                diagnostics,
                &mut invalid_reported,
                TaskDiagnosticCode::InvalidDependency,
                "depends-on exceeds the unique dependency limit",
                range,
            );
            break;
        } else {
            seen.insert(id);
            dependencies.push(id);
        }
    }
    dependencies
}

fn push_dependency_diagnostic_once(
    diagnostics: &mut Vec<TaskDiagnostic>,
    reported: &mut bool,
    code: TaskDiagnosticCode,
    message: &str,
    range: &Range<usize>,
) {
    if !*reported {
        diagnostics.push(task_diagnostic(code, message, range.clone()));
        *reported = true;
    }
}

pub(crate) fn parse_task_date_time(value: &str) -> Option<TaskDateTime> {
    if valid_calendar_date(value) {
        return Some(TaskDateTime::Date(value.to_owned()));
    }
    valid_rfc3339(value).then(|| TaskDateTime::Instant(value.to_owned()))
}

fn valid_calendar_date(value: &str) -> bool {
    if !value.is_ascii()
        || value.len() != 10
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
    {
        return false;
    }
    let Ok(year) = value[0..4].parse::<i32>() else {
        return false;
    };
    let Ok(month) = value[5..7].parse::<u8>() else {
        return false;
    };
    let Ok(day) = value[8..10].parse::<u8>() else {
        return false;
    };
    CalendarDate::new(year, month, day).is_ok()
}

fn valid_rfc3339(value: &str) -> bool {
    if !value.is_ascii()
        || value.len() < 20
        || !valid_calendar_date(&value[..10])
        || value.as_bytes()[10] != b'T'
    {
        return false;
    }
    let time = &value[11..];
    if time.len() < 9 || time.as_bytes()[2] != b':' || time.as_bytes()[5] != b':' {
        return false;
    }
    let Ok(hour) = time[0..2].parse::<u8>() else {
        return false;
    };
    let Ok(minute) = time[3..5].parse::<u8>() else {
        return false;
    };
    let Ok(second) = time[6..8].parse::<u8>() else {
        return false;
    };
    if hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let mut offset_start = 8;
    if time.as_bytes().get(offset_start) == Some(&b'.') {
        offset_start += 1;
        let fraction_start = offset_start;
        while time
            .as_bytes()
            .get(offset_start)
            .is_some_and(u8::is_ascii_digit)
        {
            offset_start += 1;
        }
        if offset_start == fraction_start {
            return false;
        }
    }
    if time.get(offset_start..) == Some("Z") {
        return true;
    }
    let Some(offset) = time.get(offset_start..) else {
        return false;
    };
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return false;
    }
    offset[1..3].parse::<u8>().is_ok_and(|hour| hour <= 23)
        && offset[4..6].parse::<u8>().is_ok_and(|minute| minute <= 59)
}

fn parse_rrule(value: &str) -> Option<TaskRecurrence> {
    let mut parts = BTreeMap::new();
    for part in value.split(';') {
        let (name, value) = part.split_once('=')?;
        if !RRULE_PARTS.contains(&name) || value.is_empty() || parts.insert(name, value).is_some() {
            return None;
        }
    }
    let frequency = match parts.remove("FREQ")? {
        "DAILY" => TaskRecurrenceFrequency::Daily,
        "WEEKLY" => TaskRecurrenceFrequency::Weekly,
        "MONTHLY" => TaskRecurrenceFrequency::Monthly,
        "YEARLY" => TaskRecurrenceFrequency::Yearly,
        _ => return None,
    };
    let interval = parts
        .remove("INTERVAL")
        .map_or(Some(1_u16), |value| bounded_u16(value, 1, 999))?;
    let by_day = parts.remove("BYDAY").map_or(Some(Vec::new()), |value| {
        parse_unique_tokens(value, WEEKDAYS)
    })?;
    let by_month_day = parts
        .remove("BYMONTHDAY")
        .map_or(Some(Vec::new()), |value| {
            parse_i8_list(value, -31, 31, false)
        })?;
    let by_month = parts
        .remove("BYMONTH")
        .map_or(Some(Vec::new()), |value| parse_u8_list(value, 1, 12))?;
    let count = match parts.remove("COUNT") {
        Some(value) => Some(bounded_u16(value, 1, 9_999)?),
        None => None,
    };
    let until = match parts.remove("UNTIL") {
        Some(value) => Some(parse_task_date_time(value)?),
        None => None,
    };
    if count.is_some() && until.is_some() {
        return None;
    }
    let week_start = parts.remove("WKST").map(str::to_owned);
    if week_start
        .as_deref()
        .is_some_and(|value| !WEEKDAYS.contains(&value))
        || !parts.is_empty()
    {
        return None;
    }
    match frequency {
        TaskRecurrenceFrequency::Daily
            if !by_day.is_empty() || !by_month_day.is_empty() || !by_month.is_empty() =>
        {
            return None;
        }
        TaskRecurrenceFrequency::Weekly if !by_month_day.is_empty() || !by_month.is_empty() => {
            return None;
        }
        TaskRecurrenceFrequency::Monthly if !by_month.is_empty() => return None,
        _ => {}
    }
    Some(TaskRecurrence {
        source: value.to_owned(),
        frequency,
        interval,
        by_day,
        by_month_day,
        by_month,
        count,
        until,
        week_start,
    })
}

fn bounded_u16(value: &str, minimum: u16, maximum: u16) -> Option<u16> {
    let value = value.parse::<u16>().ok()?;
    (minimum..=maximum).contains(&value).then_some(value)
}

fn parse_unique_tokens(value: &str, allowed: &[&str]) -> Option<Vec<String>> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for value in value.split(',') {
        if !allowed.contains(&value) || !seen.insert(value) {
            return None;
        }
        values.push(value.to_owned());
    }
    Some(values)
}

fn parse_i8_list(value: &str, minimum: i8, maximum: i8, allow_zero: bool) -> Option<Vec<i8>> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for value in value.split(',') {
        let parsed = value.parse::<i8>().ok()?;
        if !(minimum..=maximum).contains(&parsed)
            || (!allow_zero && parsed == 0)
            || !seen.insert(parsed)
        {
            return None;
        }
        values.push(parsed);
    }
    Some(values)
}

fn parse_u8_list(value: &str, minimum: u8, maximum: u8) -> Option<Vec<u8>> {
    let mut seen = BTreeSet::new();
    let mut values = Vec::new();
    for value in value.split(',') {
        let parsed = value.parse::<u8>().ok()?;
        if !(minimum..=maximum).contains(&parsed) || !seen.insert(parsed) {
            return None;
        }
        values.push(parsed);
    }
    Some(values)
}

fn diagnose_duplicate_ids(analysis: &mut TaskSourceAnalysis) {
    let mut by_id: BTreeMap<TaskId, Vec<usize>> = BTreeMap::new();
    for (index, task) in analysis.tasks.iter().enumerate() {
        if let Some(metadata) = &task.metadata {
            by_id.entry(metadata.id).or_default().push(index);
        }
    }
    for (id, indexes) in by_id.into_iter().filter(|(_, indexes)| indexes.len() > 1) {
        for index in indexes {
            analysis.tasks[index].valid = false;
            let range = analysis.tasks[index].metadata.as_ref().map_or_else(
                || analysis.tasks[index].range.clone(),
                |metadata| metadata.range.clone(),
            );
            analysis.diagnostics.push(TaskDiagnostic {
                code: TaskDiagnosticCode::DuplicateId,
                message: format!("task ID `{id}` occurs more than once in this source"),
                range,
            });
        }
    }
}

fn unprotected_macro_starts(
    source: &str,
    range: Range<usize>,
    protected: &[Range<u64>],
) -> Vec<usize> {
    let mut starts = Vec::new();
    let mut cursor = range.start;
    while cursor < range.end {
        let Some(relative) = source[cursor..range.end].find("task:[") else {
            break;
        };
        let start = cursor + relative;
        if !contains_offset(protected, start) {
            starts.push(start);
        }
        cursor = start + "task:[".len();
    }
    starts
}

fn valid_attribute_name(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.ends_with('-')
        && !value.contains("--")
}

fn contains_offset(ranges: &[Range<u64>], offset: usize) -> bool {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    ranges
        .iter()
        .any(|range| range.start <= offset && offset < range.end)
}

fn ranges_overlap(left: &Range<u64>, right: &Range<u64>) -> bool {
    left.start < right.end && right.start < left.end
}

fn trim_horizontal_end(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && matches!(source.as_bytes()[range.end - 1], b' ' | b'\t') {
        range.end -= 1;
    }
    range
}

fn next_line_start(source: &str, end: usize) -> usize {
    match source.as_bytes().get(end..) {
        Some([b'\r', b'\n', ..]) => end + 2,
        Some([b'\r' | b'\n', ..]) => end + 1,
        _ => source.len(),
    }
}

fn task_diagnostic(
    code: TaskDiagnosticCode,
    message: impl Into<String>,
    range: Range<usize>,
) -> TaskDiagnostic {
    TaskDiagnostic {
        code,
        message: message.into(),
        range: to_u64_range(range),
    }
}

fn to_u64_range(range: Range<usize>) -> Range<u64> {
    u64::try_from(range.start).unwrap_or(u64::MAX)..u64::try_from(range.end).unwrap_or(u64::MAX)
}
