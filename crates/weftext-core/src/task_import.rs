use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write as _};
use std::ops::Range;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{TaskId, TaskPhase, TaskPriority, TaskResolution, TaskState};

pub const TASK_IMPORT_PROFILE_ID: &str = "weftext.task-import.v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskImportDialect {
    MarkdownChecklistV1,
    ObsidianTasksEmojiV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskImportStatusType {
    Todo,
    Done,
    InProgress,
    OnHold,
    Cancelled,
    NonTask,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskImportStatusMapping {
    pub symbol: char,
    pub name: String,
    pub status_type: TaskImportStatusType,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskImportSettings {
    pub dialect: TaskImportDialect,
    pub plugin_version: Option<String>,
    pub global_filter: Option<String>,
    pub indentation_width: u8,
    pub statuses: Vec<TaskImportStatusMapping>,
}

impl TaskImportSettings {
    #[must_use]
    pub fn markdown_checklist_v1(indentation_width: u8) -> Self {
        Self {
            dialect: TaskImportDialect::MarkdownChecklistV1,
            plugin_version: None,
            global_filter: None,
            indentation_width,
            statuses: vec![
                TaskImportStatusMapping {
                    symbol: ' ',
                    name: "Open".to_owned(),
                    status_type: TaskImportStatusType::Todo,
                },
                TaskImportStatusMapping {
                    symbol: 'x',
                    name: "Closed".to_owned(),
                    status_type: TaskImportStatusType::Done,
                },
                TaskImportStatusMapping {
                    symbol: 'X',
                    name: "Closed".to_owned(),
                    status_type: TaskImportStatusType::Done,
                },
            ],
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskImportSettingsError {
    InvalidIndentationWidth,
    MissingPluginVersion,
    UnexpectedPluginVersion,
    InvalidGlobalFilter,
    InvalidStatusName,
    DuplicateStatusSymbol(char),
    MarkdownStatusMapping,
    DuplicateLocator(String),
    InvalidLocator,
    InvalidReviewedPlan,
}

impl fmt::Display for TaskImportSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIndentationWidth => {
                formatter.write_str("task import indentation width must be between 1 and 8")
            }
            Self::MissingPluginVersion => formatter
                .write_str("Obsidian Tasks import requires an exact non-empty plugin version"),
            Self::UnexpectedPluginVersion => {
                formatter.write_str("plain Markdown import cannot declare a plugin version")
            }
            Self::InvalidGlobalFilter => formatter.write_str(
                "task import global filter must be non-empty and contain no line endings",
            ),
            Self::InvalidStatusName => {
                formatter.write_str("task import status names must be non-empty")
            }
            Self::DuplicateStatusSymbol(symbol) => {
                write!(
                    formatter,
                    "task import status symbol `{symbol}` is duplicated"
                )
            }
            Self::MarkdownStatusMapping => formatter
                .write_str("plain Markdown import uses only the fixed space/x/X status mapping"),
            Self::DuplicateLocator(locator) => {
                write!(
                    formatter,
                    "task import document locator `{locator}` is duplicated"
                )
            }
            Self::InvalidLocator => formatter
                .write_str("task import document locators must be non-empty portable paths"),
            Self::InvalidReviewedPlan => formatter.write_str(
                "reviewed task import plan differs from its exact source set, settings, or identity mappings",
            ),
        }
    }
}

impl std::error::Error for TaskImportSettingsError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskImportDocumentInput {
    pub locator: String,
    pub source: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskImportDiagnosticCode {
    MisalignedIndentation,
    UnknownStatus,
    AmbiguousGlobalFilter,
    DuplicateMetadata,
    InvalidDate,
    InvalidState,
    UnsupportedMetadata,
    UnsupportedRecurrence,
    MissingRecurrenceAnchor,
    DuplicateLegacyId,
    UnresolvedDependency,
    AmbiguousDependency,
    SelfDependency,
    DependencyCycle,
    EmptyStructuredDescription,
    UnterminatedTasksQuery,
    UnsafeQueryInstruction,
    UnsupportedQueryInstruction,
    TargetValidation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportDiagnostic {
    pub code: TaskImportDiagnosticCode,
    pub locator: String,
    pub message: String,
    pub range: Range<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskImportEditKind {
    Checklist,
    ExcludedChecklist,
    TasksQuery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportEdit {
    pub kind: TaskImportEditKind,
    pub source_range: Range<u64>,
    pub replacement: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportDocumentPlan {
    pub locator: String,
    pub source_digest: String,
    pub proposed_source: String,
    pub edits: Vec<TaskImportEdit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportIdentityMapping {
    pub locator: String,
    pub occurrence_range: Range<u64>,
    pub legacy_id: Option<String>,
    pub task_id: TaskId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportPlan {
    pub profile: String,
    pub settings: TaskImportSettings,
    pub documents: Vec<TaskImportDocumentPlan>,
    pub identities: Vec<TaskImportIdentityMapping>,
    pub diagnostics: Vec<TaskImportDiagnostic>,
}

impl TaskImportPlan {
    #[must_use]
    pub fn is_committable(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug)]
struct SourceLine {
    range: Range<usize>,
    content: Range<usize>,
    ending: String,
}

#[derive(Clone, Debug, Default)]
struct ImportedMetadata {
    phase: Option<TaskPhase>,
    resolution: Option<TaskResolution>,
    priority: Option<TaskPriority>,
    created: Option<String>,
    start: Option<String>,
    scheduled: Option<String>,
    due: Option<String>,
    closed: Option<String>,
    recurrence: Option<String>,
    repeat_from: Option<&'static str>,
    legacy_id: Option<String>,
    dependencies: Vec<String>,
}

impl ImportedMetadata {
    fn needs_identity(&self) -> bool {
        self.phase.is_some()
            || self.resolution.is_some()
            || self.priority.is_some()
            || self.created.is_some()
            || self.start.is_some()
            || self.scheduled.is_some()
            || self.due.is_some()
            || self.closed.is_some()
            || self.recurrence.is_some()
            || self.legacy_id.is_some()
            || !self.dependencies.is_empty()
    }
}

#[derive(Clone, Debug)]
struct TaskCandidate {
    document: usize,
    range: Range<usize>,
    depth: usize,
    state: TaskState,
    description: String,
    ending: String,
    metadata: ImportedMetadata,
    diagnostics: Vec<TaskImportDiagnostic>,
    identity: Option<TaskId>,
}

#[derive(Clone, Debug)]
struct PendingEdit {
    kind: TaskImportEditKind,
    range: Range<usize>,
    replacement: String,
}

#[derive(Clone, Debug)]
struct QueryCandidate {
    document: usize,
    range: Range<usize>,
    body: Vec<(Range<usize>, String)>,
    ending: String,
    closed: bool,
}

/// Builds a read-only, workspace-wide preview for importing Markdown checklist semantics.
///
/// The source dialect and every settings-dependent Obsidian Tasks status are explicit inputs.
/// The planner resolves legacy IDs across all supplied documents before emitting any replacement.
/// It never writes files and leaves every blocked occurrence byte-exact in `proposed_source`.
///
/// # Errors
///
/// Returns an error when settings or document locators do not define one deterministic import.
pub fn plan_task_import(
    documents: &[TaskImportDocumentInput],
    settings: TaskImportSettings,
) -> Result<TaskImportPlan, TaskImportSettingsError> {
    build_task_import(documents, settings, None)
}

/// Revalidates one frozen task-import preview against its exact complete source set.
///
/// The reviewed identity mappings are injected back into the deterministic planner, so validation
/// never mints replacement task IDs. The resulting plan must be byte-for-byte identical.
///
/// # Errors
///
/// Returns an error when settings, source digests, edits, diagnostics, or any reviewed identity
/// mapping differs from the supplied plan.
pub fn validate_task_import_plan(
    documents: &[TaskImportDocumentInput],
    plan: &TaskImportPlan,
) -> Result<(), TaskImportSettingsError> {
    let unique_ids = plan
        .identities
        .iter()
        .map(|mapping| mapping.task_id)
        .collect::<BTreeSet<_>>();
    if plan.profile != TASK_IMPORT_PROFILE_ID || unique_ids.len() != plan.identities.len() {
        return Err(TaskImportSettingsError::InvalidReviewedPlan);
    }
    let expected = build_task_import(documents, plan.settings.clone(), Some(&plan.identities))?;
    if expected != *plan {
        return Err(TaskImportSettingsError::InvalidReviewedPlan);
    }
    Ok(())
}

fn build_task_import(
    documents: &[TaskImportDocumentInput],
    settings: TaskImportSettings,
    reviewed_identities: Option<&[TaskImportIdentityMapping]>,
) -> Result<TaskImportPlan, TaskImportSettingsError> {
    validate_settings(&settings)?;
    validate_documents(documents)?;
    let status_map = settings
        .statuses
        .iter()
        .map(|status| (status.symbol, status.status_type))
        .collect::<BTreeMap<_, _>>();
    let mut tasks = Vec::new();
    let mut queries = Vec::new();
    let mut excluded = vec![Vec::<PendingEdit>::new(); documents.len()];
    for (document, input) in documents.iter().enumerate() {
        scan_document(
            document,
            input,
            &settings,
            &status_map,
            &mut tasks,
            &mut queries,
            &mut excluded[document],
        );
    }

    diagnose_legacy_id_graph(documents, &mut tasks);
    assign_identities(documents, &mut tasks, reviewed_identities);
    diagnose_dependency_cycles(documents, &mut tasks);
    let legacy_targets = unique_legacy_targets(&tasks);
    let mut diagnostics = Vec::new();
    let mut edits = excluded;
    for task in &mut tasks {
        resolve_dependencies(documents, task, &legacy_targets);
        diagnostics.append(&mut task.diagnostics);
        if diagnostics_for_occurrence(&diagnostics, &documents[task.document].locator, &task.range)
        {
            continue;
        }
        let replacement = render_task(task, &legacy_targets);
        edits[task.document].push(PendingEdit {
            kind: TaskImportEditKind::Checklist,
            range: task.range.clone(),
            replacement,
        });
    }
    for query in queries {
        match render_query(documents, &query) {
            Ok(replacement) => edits[query.document].push(PendingEdit {
                kind: TaskImportEditKind::TasksQuery,
                range: query.range,
                replacement,
            }),
            Err(mut query_diagnostics) => diagnostics.append(&mut query_diagnostics),
        }
    }

    let identities = tasks
        .iter()
        .filter_map(|task| {
            task.identity.map(|task_id| TaskImportIdentityMapping {
                locator: documents[task.document].locator.clone(),
                occurrence_range: to_u64_range(task.range.clone()),
                legacy_id: task.metadata.legacy_id.clone(),
                task_id,
            })
        })
        .collect::<Vec<_>>();
    let mut document_plans = Vec::new();
    for (index, input) in documents.iter().enumerate() {
        edits[index].sort_by_key(|edit| (edit.range.start, edit.range.end));
        let proposed_source = apply_edits(&input.source, &edits[index]);
        document_plans.push(TaskImportDocumentPlan {
            locator: input.locator.clone(),
            source_digest: sha256(input.source.as_bytes()),
            proposed_source,
            edits: edits[index]
                .iter()
                .map(|edit| TaskImportEdit {
                    kind: edit.kind,
                    source_range: to_u64_range(edit.range.clone()),
                    replacement: edit.replacement.clone(),
                })
                .collect(),
        });
    }
    validate_targets(&document_plans, &mut diagnostics);
    diagnostics.sort_by(|left, right| {
        left.locator
            .cmp(&right.locator)
            .then_with(|| left.range.start.cmp(&right.range.start))
            .then_with(|| left.code.cmp(&right.code))
    });
    Ok(TaskImportPlan {
        profile: TASK_IMPORT_PROFILE_ID.to_owned(),
        settings,
        documents: document_plans,
        identities,
        diagnostics,
    })
}

fn validate_settings(settings: &TaskImportSettings) -> Result<(), TaskImportSettingsError> {
    if !(1..=8).contains(&settings.indentation_width) {
        return Err(TaskImportSettingsError::InvalidIndentationWidth);
    }
    if settings
        .global_filter
        .as_ref()
        .is_some_and(|filter| filter.is_empty() || filter.contains('\n') || filter.contains('\r'))
    {
        return Err(TaskImportSettingsError::InvalidGlobalFilter);
    }
    match settings.dialect {
        TaskImportDialect::MarkdownChecklistV1 => {
            if settings.plugin_version.is_some() {
                return Err(TaskImportSettingsError::UnexpectedPluginVersion);
            }
            if settings.global_filter.is_some()
                || settings.statuses
                    != TaskImportSettings::markdown_checklist_v1(settings.indentation_width)
                        .statuses
            {
                return Err(TaskImportSettingsError::MarkdownStatusMapping);
            }
        }
        TaskImportDialect::ObsidianTasksEmojiV1 => {
            if settings
                .plugin_version
                .as_ref()
                .is_none_or(|version| version.trim().is_empty())
            {
                return Err(TaskImportSettingsError::MissingPluginVersion);
            }
        }
    }
    let mut symbols = BTreeSet::new();
    for status in &settings.statuses {
        if status.name.trim().is_empty() {
            return Err(TaskImportSettingsError::InvalidStatusName);
        }
        if !symbols.insert(status.symbol) {
            return Err(TaskImportSettingsError::DuplicateStatusSymbol(
                status.symbol,
            ));
        }
    }
    Ok(())
}

fn validate_documents(
    documents: &[TaskImportDocumentInput],
) -> Result<(), TaskImportSettingsError> {
    let mut locators = BTreeSet::new();
    for document in documents {
        if document.locator.is_empty()
            || document.locator.starts_with('/')
            || document.locator.contains('\\')
            || document
                .locator
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(TaskImportSettingsError::InvalidLocator);
        }
        if !locators.insert(&document.locator) {
            return Err(TaskImportSettingsError::DuplicateLocator(
                document.locator.clone(),
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn scan_document(
    document: usize,
    input: &TaskImportDocumentInput,
    settings: &TaskImportSettings,
    status_map: &BTreeMap<char, TaskImportStatusType>,
    tasks: &mut Vec<TaskCandidate>,
    queries: &mut Vec<QueryCandidate>,
    excluded: &mut Vec<PendingEdit>,
) {
    let lines = source_lines(&input.source);
    let mut index = 0;
    let mut obsidian_comment = false;
    let mut html_comment = false;
    let mut frontmatter = lines
        .first()
        .is_some_and(|line| input.source[line.content.clone()].trim() == "---");
    while index < lines.len() {
        let line = &lines[index];
        let text = &input.source[line.content.clone()];
        if frontmatter {
            if index > 0 && matches!(text.trim(), "---" | "...") {
                frontmatter = false;
            }
            index += 1;
            continue;
        }
        if html_comment {
            if text.contains("-->") {
                html_comment = false;
            }
            index += 1;
            continue;
        }
        if text.trim_start().starts_with("<!--") {
            html_comment = !text.contains("-->");
            index += 1;
            continue;
        }
        if settings.dialect == TaskImportDialect::ObsidianTasksEmojiV1 && text.trim() == "%%" {
            obsidian_comment = !obsidian_comment;
            index += 1;
            continue;
        }
        if obsidian_comment {
            index += 1;
            continue;
        }
        if let Some((marker, width, info)) = markdown_fence(text) {
            let start = line.range.start;
            let is_tasks = info.eq_ignore_ascii_case("tasks");
            let mut body = Vec::new();
            let mut cursor = index + 1;
            let mut end = line.range.end;
            let mut closed = false;
            while cursor < lines.len() {
                let candidate = &lines[cursor];
                let candidate_text = &input.source[candidate.content.clone()];
                if closes_fence(candidate_text, marker, width) {
                    end = candidate.range.end;
                    closed = true;
                    cursor += 1;
                    break;
                }
                if is_tasks {
                    body.push((candidate.content.clone(), candidate_text.to_owned()));
                }
                end = candidate.range.end;
                cursor += 1;
            }
            if is_tasks {
                queries.push(QueryCandidate {
                    document,
                    range: start..end,
                    body,
                    ending: line.ending.clone(),
                    closed,
                });
            }
            index = cursor;
            continue;
        }
        if let Some(parsed) = parse_checklist_line(document, input, line, settings, status_map) {
            match parsed {
                ParsedChecklist::Task(task) => tasks.push(*task),
                ParsedChecklist::Excluded(replacement) => excluded.push(PendingEdit {
                    kind: TaskImportEditKind::ExcludedChecklist,
                    range: line.range.clone(),
                    replacement,
                }),
            }
        }
        index += 1;
    }
}

enum ParsedChecklist {
    Task(Box<TaskCandidate>),
    Excluded(String),
}

#[allow(clippy::too_many_lines)]
fn parse_checklist_line(
    document: usize,
    input: &TaskImportDocumentInput,
    line: &SourceLine,
    settings: &TaskImportSettings,
    status_map: &BTreeMap<char, TaskImportStatusType>,
) -> Option<ParsedChecklist> {
    let text = &input.source[line.content.clone()];
    let (indent_end, depth) = markdown_depth(text, settings.indentation_width);
    let bytes = text.as_bytes();
    let marker = *bytes.get(indent_end)?;
    if !matches!(marker, b'-' | b'*' | b'+')
        || !matches!(bytes.get(indent_end + 1), Some(b' ' | b'\t'))
    {
        return None;
    }
    let mut cursor = indent_end + 1;
    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'[') {
        return None;
    }
    let close = text[cursor + 1..].find(']')? + cursor + 1;
    let status_text = &text[cursor + 1..close];
    if status_text.chars().count() != 1 {
        return None;
    }
    let status = status_text.chars().next()?;
    let mut body_start = close + 1;
    if body_start < text.len() && !matches!(bytes[body_start], b' ' | b'\t') {
        return None;
    }
    while matches!(bytes.get(body_start), Some(b' ' | b'\t')) {
        body_start += 1;
    }
    let body = &text[body_start..];
    let included = settings
        .global_filter
        .as_ref()
        .is_none_or(|filter| body.contains(filter));
    let status_type = status_map.get(&status).copied();
    if !included || status_type == Some(TaskImportStatusType::NonTask) {
        let mut replacement = text.to_owned();
        replacement.insert(cursor, '\\');
        replacement.push_str(&line.ending);
        return Some(ParsedChecklist::Excluded(replacement));
    }
    let mut diagnostics = Vec::new();
    if text[..indent_end].contains(' ') && text[..indent_end].contains('\t')
        || (!text[..indent_end].contains('\t')
            && !text[..indent_end]
                .len()
                .is_multiple_of(usize::from(settings.indentation_width)))
    {
        diagnostics.push(import_diagnostic(
            TaskImportDiagnosticCode::MisalignedIndentation,
            &input.locator,
            "task indentation does not match the pinned import width",
            line.content.start..line.content.start + indent_end,
        ));
    }
    let Some(status_type) = status_type else {
        diagnostics.push(import_diagnostic(
            TaskImportDiagnosticCode::UnknownStatus,
            &input.locator,
            "task checkbox status is absent from the pinned status mapping",
            line.content.start + cursor + 1..line.content.start + close,
        ));
        return Some(ParsedChecklist::Task(Box::new(TaskCandidate {
            document,
            range: line.range.clone(),
            depth,
            state: TaskState::Open,
            description: body.to_owned(),
            ending: line.ending.clone(),
            metadata: ImportedMetadata::default(),
            diagnostics,
            identity: None,
        })));
    };
    let (state, phase, resolution) = mapped_status(status, status_type);
    let (description, mut metadata, mut metadata_diagnostics) = parse_metadata_suffix(
        body,
        line.content.start + body_start,
        &input.locator,
        settings,
    );
    diagnostics.append(&mut metadata_diagnostics);
    metadata.phase = metadata.phase.or(phase);
    metadata.resolution = metadata.resolution.or(resolution);
    if let Some(filter) = settings.global_filter.as_deref() {
        let occurrences = description.match_indices(filter).collect::<Vec<_>>();
        if occurrences.len() != 1 {
            diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::AmbiguousGlobalFilter,
                &input.locator,
                "the pinned global filter must occur exactly once in an imported task description",
                line.content.start + body_start..line.content.end,
            ));
        }
    }
    let description = remove_global_filter(&description, settings.global_filter.as_deref());
    if state == TaskState::Open && (metadata.resolution.is_some() || metadata.closed.is_some()) {
        diagnostics.push(import_diagnostic(
            TaskImportDiagnosticCode::InvalidState,
            &input.locator,
            "an open imported task cannot contain done/cancelled metadata",
            line.content.start + cursor..line.content.end,
        ));
    }
    if state == TaskState::Closed && metadata.phase.is_some() {
        diagnostics.push(import_diagnostic(
            TaskImportDiagnosticCode::InvalidState,
            &input.locator,
            "a closed imported task cannot map to an open phase",
            line.content.start + cursor..line.content.end,
        ));
    }
    if metadata.needs_identity() && description.is_empty() {
        diagnostics.push(import_diagnostic(
            TaskImportDiagnosticCode::EmptyStructuredDescription,
            &input.locator,
            "structured imported tasks require a non-empty description",
            line.content.start + body_start..line.content.end,
        ));
    }
    Some(ParsedChecklist::Task(Box::new(TaskCandidate {
        document,
        range: line.range.clone(),
        depth,
        state,
        description,
        ending: line.ending.clone(),
        metadata,
        diagnostics,
        identity: None,
    })))
}

fn markdown_depth(text: &str, indentation_width: u8) -> (usize, usize) {
    let indent_end = text
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let indent = &text[..indent_end];
    let depth = if indent.contains('\t') {
        indent.chars().count() + 1
    } else {
        indent.len() / usize::from(indentation_width) + 1
    };
    (indent_end, depth)
}

fn mapped_status(
    source_symbol: char,
    status_type: TaskImportStatusType,
) -> (TaskState, Option<TaskPhase>, Option<TaskResolution>) {
    match status_type {
        TaskImportStatusType::Todo | TaskImportStatusType::NonTask => (TaskState::Open, None, None),
        TaskImportStatusType::InProgress => (TaskState::Open, Some(TaskPhase::InProgress), None),
        TaskImportStatusType::OnHold => (TaskState::Open, Some(TaskPhase::OnHold), None),
        TaskImportStatusType::Done if matches!(source_symbol, 'x' | 'X') => {
            (TaskState::Closed, None, None)
        }
        TaskImportStatusType::Done => (TaskState::Closed, None, Some(TaskResolution::Completed)),
        TaskImportStatusType::Cancelled => {
            (TaskState::Closed, None, Some(TaskResolution::Cancelled))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MetadataKind {
    Created,
    Start,
    Scheduled,
    Due,
    Done,
    Cancelled,
    Priority(TaskPriority),
    Recurrence,
    Id,
    DependsOn,
    OnCompletion,
    Reminder,
}

const METADATA_MARKERS: &[(&str, MetadataKind)] = &[
    ("➕", MetadataKind::Created),
    ("🛫", MetadataKind::Start),
    ("⏳", MetadataKind::Scheduled),
    ("📅", MetadataKind::Due),
    ("✅", MetadataKind::Done),
    ("❌", MetadataKind::Cancelled),
    ("🔺", MetadataKind::Priority(TaskPriority::Highest)),
    ("⏫", MetadataKind::Priority(TaskPriority::High)),
    ("🔼", MetadataKind::Priority(TaskPriority::Medium)),
    ("🔽", MetadataKind::Priority(TaskPriority::Low)),
    ("⏬", MetadataKind::Priority(TaskPriority::Lowest)),
    ("🔁", MetadataKind::Recurrence),
    ("🆔", MetadataKind::Id),
    ("⛔", MetadataKind::DependsOn),
    ("🏁", MetadataKind::OnCompletion),
    ("⏰", MetadataKind::Reminder),
];

#[allow(clippy::too_many_lines)]
fn parse_metadata_suffix(
    body: &str,
    absolute_start: usize,
    locator: &str,
    settings: &TaskImportSettings,
) -> (String, ImportedMetadata, Vec<TaskImportDiagnostic>) {
    if settings.dialect == TaskImportDialect::MarkdownChecklistV1 {
        return (
            body.trim_end_matches([' ', '\t']).to_owned(),
            ImportedMetadata::default(),
            Vec::new(),
        );
    }
    let mut markers = Vec::<(usize, usize, MetadataKind)>::new();
    for (marker, kind) in METADATA_MARKERS {
        for (start, _) in body.match_indices(marker) {
            if start > 0
                && body[..start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
            {
                markers.push((start, start + marker.len(), *kind));
            }
        }
    }
    markers.sort_by_key(|(start, end, _)| (*start, *end));
    if markers.is_empty() {
        return (
            body.trim_end_matches([' ', '\t']).to_owned(),
            ImportedMetadata::default(),
            Vec::new(),
        );
    }
    let description = body[..markers[0].0]
        .trim_end_matches([' ', '\t'])
        .to_owned();
    let mut metadata = ImportedMetadata::default();
    let mut diagnostics = Vec::new();
    let mut seen = BTreeSet::<&'static str>::new();
    for (index, (start, marker_end, kind)) in markers.iter().copied().enumerate() {
        let end = markers.get(index + 1).map_or(body.len(), |next| next.0);
        let value = body[marker_end..end].trim();
        let range = absolute_start + start..absolute_start + end;
        let field = metadata_field_name(kind);
        if !seen.insert(field) {
            diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::DuplicateMetadata,
                locator,
                format!("imported task metadata `{field}` is duplicated"),
                range.clone(),
            ));
            continue;
        }
        match kind {
            MetadataKind::Created
            | MetadataKind::Start
            | MetadataKind::Scheduled
            | MetadataKind::Due
            | MetadataKind::Done
            | MetadataKind::Cancelled => {
                if !valid_iso_date(value) {
                    diagnostics.push(import_diagnostic(
                        TaskImportDiagnosticCode::InvalidDate,
                        locator,
                        "Obsidian Tasks date metadata must be an exact valid YYYY-MM-DD date",
                        range,
                    ));
                    continue;
                }
                match kind {
                    MetadataKind::Created => metadata.created = Some(value.to_owned()),
                    MetadataKind::Start => metadata.start = Some(value.to_owned()),
                    MetadataKind::Scheduled => metadata.scheduled = Some(value.to_owned()),
                    MetadataKind::Due => metadata.due = Some(value.to_owned()),
                    MetadataKind::Done => {
                        metadata.closed = Some(value.to_owned());
                        metadata.resolution = Some(TaskResolution::Completed);
                    }
                    MetadataKind::Cancelled => {
                        metadata.closed = Some(value.to_owned());
                        metadata.resolution = Some(TaskResolution::Cancelled);
                    }
                    _ => unreachable!(),
                }
            }
            MetadataKind::Priority(priority) => {
                if value.is_empty() {
                    metadata.priority = Some(priority);
                } else {
                    diagnostics.push(import_diagnostic(
                        TaskImportDiagnosticCode::UnsupportedMetadata,
                        locator,
                        "priority emoji may not carry trailing text in the v1 importer",
                        range,
                    ));
                }
            }
            MetadataKind::Recurrence => match convert_recurrence(value) {
                Some((rrule, repeat_from)) => {
                    metadata.recurrence = Some(rrule);
                    metadata.repeat_from = repeat_from;
                }
                None => diagnostics.push(import_diagnostic(
                    TaskImportDiagnosticCode::UnsupportedRecurrence,
                    locator,
                    "recurrence phrase is outside the reviewed task-import v1 subset",
                    range,
                )),
            },
            MetadataKind::Id => {
                if valid_legacy_id(value) {
                    metadata.legacy_id = Some(value.to_owned());
                } else {
                    diagnostics.push(import_diagnostic(
                        TaskImportDiagnosticCode::UnsupportedMetadata,
                        locator,
                        "legacy task ID must contain only ASCII letters, digits, underscore, or hyphen",
                        range,
                    ));
                }
            }
            MetadataKind::DependsOn => {
                let dependencies = value
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>();
                if dependencies.is_empty() || dependencies.iter().any(|id| !valid_legacy_id(id)) {
                    diagnostics.push(import_diagnostic(
                        TaskImportDiagnosticCode::UnsupportedMetadata,
                        locator,
                        "dependency metadata must contain comma-separated valid legacy task IDs",
                        range,
                    ));
                } else {
                    metadata.dependencies = dependencies.into_iter().map(str::to_owned).collect();
                }
            }
            MetadataKind::OnCompletion if value.eq_ignore_ascii_case("keep") => {}
            MetadataKind::OnCompletion => diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::UnsupportedMetadata,
                locator,
                "only `🏁 keep` has a lossless Weftext v1 mapping; delete semantics require a decision",
                range,
            )),
            MetadataKind::Reminder => diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::UnsupportedMetadata,
                locator,
                "reminder metadata has no accepted Weftext task field",
                range,
            )),
        }
    }
    if metadata.recurrence.is_some() && metadata.repeat_from.is_none() {
        metadata.repeat_from = if metadata.due.is_some() {
            Some("due")
        } else if metadata.scheduled.is_some() {
            Some("scheduled")
        } else {
            diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::MissingRecurrenceAnchor,
                locator,
                "recurrence without `when done` requires a due or scheduled date",
                absolute_start + markers[0].0..absolute_start + body.len(),
            ));
            None
        };
    }
    (description, metadata, diagnostics)
}

const fn metadata_field_name(kind: MetadataKind) -> &'static str {
    match kind {
        MetadataKind::Created => "created",
        MetadataKind::Start => "start",
        MetadataKind::Scheduled => "scheduled",
        MetadataKind::Due => "due",
        MetadataKind::Done | MetadataKind::Cancelled => "closed",
        MetadataKind::Priority(_) => "priority",
        MetadataKind::Recurrence => "recurrence",
        MetadataKind::Id => "id",
        MetadataKind::DependsOn => "depends-on",
        MetadataKind::OnCompletion => "on-completion",
        MetadataKind::Reminder => "reminder",
    }
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
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
    crate::CalendarDate::new(year, month, day).is_ok()
}

fn valid_legacy_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn convert_recurrence(value: &str) -> Option<(String, Option<&'static str>)> {
    let lower = value.to_ascii_lowercase();
    let (phrase, repeat_from) = lower
        .strip_suffix(" when done")
        .map_or((lower.as_str(), None), |phrase| {
            (phrase.trim_end(), Some("completion"))
        });
    let tokens = phrase.split_ascii_whitespace().collect::<Vec<_>>();
    match tokens.as_slice() {
        ["every", "day"] => Some(("FREQ=DAILY".to_owned(), repeat_from)),
        ["every", count, "days"] => recurrence_interval("DAILY", count, repeat_from),
        ["every", "week"] => Some(("FREQ=WEEKLY".to_owned(), repeat_from)),
        ["every", count, "weeks"] => recurrence_interval("WEEKLY", count, repeat_from),
        ["every", "month"] => Some(("FREQ=MONTHLY".to_owned(), repeat_from)),
        ["every", count, "months"] => recurrence_interval("MONTHLY", count, repeat_from),
        ["every", "year"] => Some(("FREQ=YEARLY".to_owned(), repeat_from)),
        ["every", count, "years"] => recurrence_interval("YEARLY", count, repeat_from),
        ["every", "week", "on", days @ ..] if !days.is_empty() => {
            recurrence_weekdays(days, repeat_from)
        }
        ["every", "month", "on", "the", ordinal] => {
            let day = parse_ordinal(ordinal)?;
            Some((format!("FREQ=MONTHLY;BYMONTHDAY={day}"), repeat_from))
        }
        _ => None,
    }
}

fn recurrence_interval(
    frequency: &str,
    count: &str,
    repeat_from: Option<&'static str>,
) -> Option<(String, Option<&'static str>)> {
    let count = count.parse::<u16>().ok()?;
    (1..=999)
        .contains(&count)
        .then(|| (format!("FREQ={frequency};INTERVAL={count}"), repeat_from))
}

fn recurrence_weekdays(
    days: &[&str],
    repeat_from: Option<&'static str>,
) -> Option<(String, Option<&'static str>)> {
    let joined = days.join(" ");
    let mut mapped = Vec::new();
    for day in joined.split(',').map(str::trim) {
        mapped.push(match day {
            "monday" => "MO",
            "tuesday" => "TU",
            "wednesday" => "WE",
            "thursday" => "TH",
            "friday" => "FR",
            "saturday" => "SA",
            "sunday" => "SU",
            _ => return None,
        });
    }
    let mut unique = mapped.clone();
    unique.sort_unstable();
    unique.dedup();
    if unique.len() != mapped.len() {
        return None;
    }
    Some((
        format!("FREQ=WEEKLY;BYDAY={}", mapped.join(",")),
        repeat_from,
    ))
}

fn parse_ordinal(value: &str) -> Option<i8> {
    let (digits, suffix) = value.split_at(value.len().checked_sub(2)?);
    let day = digits.parse::<i8>().ok()?;
    let expected = match day {
        11..=13 => "th",
        value if value % 10 == 1 => "st",
        value if value % 10 == 2 => "nd",
        value if value % 10 == 3 => "rd",
        _ => "th",
    };
    ((1..=31).contains(&day) && suffix == expected).then_some(day)
}

fn remove_global_filter(description: &str, filter: Option<&str>) -> String {
    let Some(filter) = filter else {
        return description.trim().to_owned();
    };
    let Some(index) = description.find(filter) else {
        return description.trim().to_owned();
    };
    let before = description[..index].trim_end_matches([' ', '\t']);
    let after = description[index + filter.len()..].trim_start_matches([' ', '\t']);
    match (before.is_empty(), after.is_empty()) {
        (true, _) => after.trim_end().to_owned(),
        (_, true) => before.trim_start().to_owned(),
        (false, false) => format!("{before} {after}"),
    }
}

fn diagnose_legacy_id_graph(documents: &[TaskImportDocumentInput], tasks: &mut [TaskCandidate]) {
    let mut declarations = BTreeMap::<String, Vec<usize>>::new();
    for (index, task) in tasks.iter().enumerate() {
        if let Some(id) = &task.metadata.legacy_id {
            declarations.entry(id.clone()).or_default().push(index);
        }
    }
    for (id, indices) in declarations {
        if indices.len() <= 1 {
            continue;
        }
        for index in indices {
            let task = &mut tasks[index];
            task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::DuplicateLegacyId,
                &documents[task.document].locator,
                format!("legacy task ID `{id}` is declared more than once"),
                task.range.clone(),
            ));
        }
    }
}

fn assign_identities(
    documents: &[TaskImportDocumentInput],
    tasks: &mut [TaskCandidate],
    reviewed: Option<&[TaskImportIdentityMapping]>,
) {
    for task in tasks {
        if task.metadata.needs_identity() {
            task.identity = reviewed.and_then(|mappings| {
                let locator = &documents[task.document].locator;
                let range = to_u64_range(task.range.clone());
                let mut matches = mappings.iter().filter(|mapping| {
                    mapping.locator == *locator
                        && mapping.occurrence_range == range
                        && mapping.legacy_id == task.metadata.legacy_id
                });
                let identity = matches.next().map(|mapping| mapping.task_id);
                (matches.next().is_none()).then_some(identity).flatten()
            });
            if reviewed.is_none() {
                task.identity = Some(TaskId::new());
            }
        }
    }
}

fn diagnose_dependency_cycles(documents: &[TaskImportDocumentInput], tasks: &mut [TaskCandidate]) {
    let mut declarations = BTreeMap::<String, Vec<usize>>::new();
    for (index, task) in tasks.iter().enumerate() {
        if let Some(id) = &task.metadata.legacy_id {
            declarations.entry(id.clone()).or_default().push(index);
        }
    }
    let unique = declarations
        .into_iter()
        .filter_map(|(id, indices)| (indices.len() == 1).then_some((id, indices[0])))
        .collect::<BTreeMap<_, _>>();
    let mut graph = vec![Vec::<usize>::new(); tasks.len()];
    for (source, task) in tasks.iter().enumerate() {
        for dependency in &task.metadata.dependencies {
            if let Some(target) = unique.get(dependency).copied() {
                if source == target {
                    continue;
                }
                graph[source].push(target);
            }
        }
        graph[source].sort_unstable();
        graph[source].dedup();
    }
    let components = strongly_connected_components(&graph);
    for component in components
        .into_iter()
        .filter(|component| component.len() > 1)
    {
        let members = component
            .iter()
            .filter_map(|index| tasks[*index].metadata.legacy_id.as_deref())
            .collect::<Vec<_>>()
            .join(", ");
        for index in component {
            let task = &mut tasks[index];
            task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::DependencyCycle,
                &documents[task.document].locator,
                format!("legacy task dependency cycle contains: {members}"),
                task.range.clone(),
            ));
        }
    }
}

fn strongly_connected_components(graph: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; graph.len()];
    let mut order = Vec::with_capacity(graph.len());
    for start in 0..graph.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, edge)) = stack.last_mut() {
            if *edge < graph[*node].len() {
                let target = graph[*node][*edge];
                *edge += 1;
                if !visited[target] {
                    visited[target] = true;
                    stack.push((target, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }
    let mut reverse = vec![Vec::<usize>::new(); graph.len()];
    for (source, targets) in graph.iter().enumerate() {
        for target in targets {
            reverse[*target].push(source);
        }
    }
    let mut assigned = vec![false; graph.len()];
    let mut components = Vec::new();
    for start in order.into_iter().rev() {
        if assigned[start] {
            continue;
        }
        assigned[start] = true;
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for source in &reverse[node] {
                if !assigned[*source] {
                    assigned[*source] = true;
                    stack.push(*source);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn unique_legacy_targets(tasks: &[TaskCandidate]) -> BTreeMap<String, Vec<TaskId>> {
    let mut targets = BTreeMap::<String, Vec<TaskId>>::new();
    for task in tasks {
        if let (Some(legacy_id), Some(task_id)) = (&task.metadata.legacy_id, task.identity) {
            targets.entry(legacy_id.clone()).or_default().push(task_id);
        }
    }
    targets
}

fn resolve_dependencies(
    documents: &[TaskImportDocumentInput],
    task: &mut TaskCandidate,
    targets: &BTreeMap<String, Vec<TaskId>>,
) {
    let mut seen = BTreeSet::new();
    for dependency in &task.metadata.dependencies {
        if !seen.insert(dependency) {
            task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::DuplicateMetadata,
                &documents[task.document].locator,
                format!("legacy dependency `{dependency}` is duplicated"),
                task.range.clone(),
            ));
            continue;
        }
        if task.metadata.legacy_id.as_deref() == Some(dependency) {
            task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::SelfDependency,
                &documents[task.document].locator,
                "an imported task cannot depend on its own legacy ID",
                task.range.clone(),
            ));
            continue;
        }
        match targets.get(dependency).map(Vec::as_slice) {
            None | Some([]) => task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::UnresolvedDependency,
                &documents[task.document].locator,
                format!("legacy dependency `{dependency}` does not resolve"),
                task.range.clone(),
            )),
            Some([_]) => {}
            Some(_) => task.diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::AmbiguousDependency,
                &documents[task.document].locator,
                format!("legacy dependency `{dependency}` resolves to multiple tasks"),
                task.range.clone(),
            )),
        }
    }
}

fn diagnostics_for_occurrence(
    diagnostics: &[TaskImportDiagnostic],
    locator: &str,
    occurrence: &Range<usize>,
) -> bool {
    let occurrence = to_u64_range(occurrence.clone());
    diagnostics
        .iter()
        .any(|item| item.locator == locator && ranges_overlap(&item.range, &occurrence))
}

fn render_task(task: &TaskCandidate, targets: &BTreeMap<String, Vec<TaskId>>) -> String {
    let mut output = "*".repeat(task.depth);
    output.push_str(match task.state {
        TaskState::Open => " [ ] ",
        TaskState::Closed => " [x] ",
    });
    output.push_str(&task.description);
    if let Some(id) = task.identity {
        let mut attributes = vec![format!("id={id}")];
        if let Some(phase) = task.metadata.phase {
            attributes.push(format!("phase={}", phase_text(phase)));
        }
        if let Some(resolution) = task.metadata.resolution {
            attributes.push(format!("resolution={}", resolution_text(resolution)));
        }
        if let Some(priority) = task.metadata.priority {
            attributes.push(format!("priority={}", priority_text(priority)));
        }
        for (name, value) in [
            ("created", task.metadata.created.as_deref()),
            ("start", task.metadata.start.as_deref()),
            ("scheduled", task.metadata.scheduled.as_deref()),
            ("due", task.metadata.due.as_deref()),
            ("closed", task.metadata.closed.as_deref()),
        ] {
            if let Some(value) = value {
                attributes.push(format!("{name}={value}"));
            }
        }
        if let Some(rrule) = task.metadata.recurrence.as_deref() {
            attributes.push(format!(
                "rrule={}",
                serde_json::to_string(rrule).expect("RRULE string serialization")
            ));
        }
        if let Some(repeat_from) = task.metadata.repeat_from {
            attributes.push(format!("repeat-from={repeat_from}"));
        }
        let dependencies = task
            .metadata
            .dependencies
            .iter()
            .filter_map(|legacy| targets.get(legacy).and_then(|values| values.first()))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if !dependencies.is_empty() {
            attributes.push(format!(
                "depends-on={}",
                serde_json::to_string(&dependencies.join(" "))
                    .expect("dependency string serialization")
            ));
        }
        output.push_str(" task:[");
        output.push_str(&attributes.join(","));
        output.push(']');
    }
    output.push_str(&task.ending);
    output
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

#[allow(clippy::too_many_lines)]
fn render_query(
    documents: &[TaskImportDocumentInput],
    query: &QueryCandidate,
) -> Result<String, Vec<TaskImportDiagnostic>> {
    let locator = &documents[query.document].locator;
    if !query.closed {
        return Err(vec![import_diagnostic(
            TaskImportDiagnosticCode::UnterminatedTasksQuery,
            locator,
            "Obsidian Tasks query fence is not closed",
            query.range.clone(),
        )]);
    }
    let mut predicates = Vec::new();
    let mut group = None;
    let mut sorts = Vec::new();
    let mut limit = None;
    let mut diagnostics = Vec::new();
    for (range, raw) in &query.body {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.contains(" function")
            || lower.starts_with("function")
            || line.contains("{{")
            || lower.starts_with("preset ")
        {
            diagnostics.push(import_diagnostic(
                TaskImportDiagnosticCode::UnsafeQueryInstruction,
                locator,
                "executable, preset, or placeholder query instructions are never imported",
                range.clone(),
            ));
            continue;
        }
        if lower == "not done" {
            predicates.push("task.closed = false".to_owned());
        } else if lower == "done" {
            predicates.push("task.closed = true".to_owned());
        } else if lower == "no due date" {
            predicates.push("task.due is null".to_owned());
        } else if lower == "has due date" {
            predicates.push("task.due is not null".to_owned());
        } else if lower == "due today" {
            predicates.push("task.due = context.today".to_owned());
        } else if let Some(value) = lower.strip_prefix("due on ") {
            if valid_iso_date(value) {
                predicates.push(format!("task.due = date(\"{value}\")"));
            } else {
                diagnostics.push(unsupported_query(locator, range, line));
            }
        } else if let Some(value) = lower.strip_prefix("due before ") {
            if let Some(value) = query_date_value(value) {
                predicates.push(format!("task.due < {value}"));
            } else {
                diagnostics.push(unsupported_query(locator, range, line));
            }
        } else if let Some(value) = lower.strip_prefix("due after ") {
            if let Some(value) = query_date_value(value) {
                predicates.push(format!("task.due > {value}"));
            } else {
                diagnostics.push(unsupported_query(locator, range, line));
            }
        } else if lower.starts_with("description includes ") {
            let value = &line["description includes ".len()..];
            predicates.push(format!(
                "contains(task.title, {})",
                serde_json::to_string(value).expect("query string serialization")
            ));
        } else if lower.starts_with("path includes ") {
            let value = &line["path includes ".len()..];
            predicates.push(format!(
                "contains(task.owner_node.path, {})",
                serde_json::to_string(value).expect("query string serialization")
            ));
        } else if lower == "sort by due" {
            sorts.push("task.due asc nulls last".to_owned());
        } else if lower == "sort by priority" {
            sorts.push("task.priority desc".to_owned());
        } else if lower == "group by status" || lower == "group by status.type" {
            group = Some("task.state as group_state".to_owned());
        } else if lower == "group by due" {
            group = Some("task.due as group_due".to_owned());
        } else if let Some(value) = lower
            .strip_prefix("limit to ")
            .and_then(|value| value.strip_suffix(" tasks"))
        {
            match value.parse::<u16>() {
                Ok(value) if (1..=crate::QUERY_MAX_LIMIT).contains(&value) => {
                    limit = Some(value);
                }
                _ => diagnostics.push(unsupported_query(locator, range, line)),
            }
        } else {
            diagnostics.push(unsupported_query(locator, range, line));
        }
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let ending = if query.ending.is_empty() {
        "\n"
    } else {
        &query.ending
    };
    let mut output = format!(
        "[.weftext-query,version=1,view=task-list]{ending}....{ending}from tasks as task{ending}scope workspace{ending}where "
    );
    if predicates.is_empty() {
        output.push_str("true");
    } else {
        output.push_str(&predicates.join(" and "));
    }
    output.push_str(ending);
    if let Some(group) = group {
        output.push_str("group by ");
        output.push_str(&group);
        output.push_str(ending);
    }
    output.push_str("select task.kind, task.id as task_id, task.owner_node.id as owner_node_id, task.title, task.closed, task.state, task.priority, task.due");
    output.push_str(ending);
    output.push_str("order by ");
    if sorts.is_empty() {
        output.push_str("task.owner_node.path asc");
    } else {
        output.push_str(&sorts.join(", "));
    }
    output.push_str(ending);
    write!(
        output,
        "limit {}{ending}",
        limit.unwrap_or(crate::QUERY_DEFAULT_LIMIT)
    )
    .expect("string write");
    output.push_str("....");
    if query.ending.is_empty() {
        output.push('\n');
    } else {
        output.push_str(ending);
    }
    Ok(output)
}

fn query_date_value(value: &str) -> Option<String> {
    match value {
        "today" => Some("context.today".to_owned()),
        "tomorrow" => Some("context.today + P1D".to_owned()),
        "yesterday" => Some("context.today - P1D".to_owned()),
        value if valid_iso_date(value) => Some(format!("date(\"{value}\")")),
        _ => None,
    }
}

fn unsupported_query(locator: &str, range: &Range<usize>, line: &str) -> TaskImportDiagnostic {
    import_diagnostic(
        TaskImportDiagnosticCode::UnsupportedQueryInstruction,
        locator,
        format!("Obsidian Tasks query instruction has no deterministic v1 mapping: `{line}`"),
        range.clone(),
    )
}

fn validate_targets(
    documents: &[TaskImportDocumentPlan],
    diagnostics: &mut Vec<TaskImportDiagnostic>,
) {
    for document in documents {
        let task_analysis = crate::analyze_task_source(&document.proposed_source);
        for item in task_analysis.diagnostics {
            diagnostics.push(TaskImportDiagnostic {
                code: TaskImportDiagnosticCode::TargetValidation,
                locator: document.locator.clone(),
                message: format!("generated task source is invalid: {}", item.message),
                range: item.range,
            });
        }
        let query_analysis = crate::analyze_query_source(&document.proposed_source);
        for item in query_analysis.diagnostics {
            diagnostics.push(TaskImportDiagnostic {
                code: TaskImportDiagnosticCode::TargetValidation,
                locator: document.locator.clone(),
                message: format!("generated query source is invalid: {}", item.message),
                range: item.range,
            });
        }
    }
}

fn apply_edits(source: &str, edits: &[PendingEdit]) -> String {
    let mut output = source.to_owned();
    for edit in edits.iter().rev() {
        output.replace_range(edit.range.clone(), &edit.replacement);
    }
    output
}

fn source_lines(source: &str) -> Vec<SourceLine> {
    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\r' || bytes[cursor] == b'\n' {
            let content_end = cursor;
            let ending_end = if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                cursor + 2
            } else {
                cursor + 1
            };
            lines.push(SourceLine {
                range: start..ending_end,
                content: start..content_end,
                ending: source[content_end..ending_end].to_owned(),
            });
            start = ending_end;
            cursor = ending_end;
        } else {
            cursor += 1;
        }
    }
    if start < source.len() {
        lines.push(SourceLine {
            range: start..source.len(),
            content: start..source.len(),
            ending: String::new(),
        });
    }
    lines
}

fn markdown_fence(line: &str) -> Option<(char, usize, &str)> {
    let trimmed = line.trim_start_matches(' ');
    if line.len() - trimmed.len() > 3 {
        return None;
    }
    let marker = trimmed.chars().next()?;
    if !matches!(marker, '`' | '~') {
        return None;
    }
    let width = trimmed
        .chars()
        .take_while(|character| *character == marker)
        .count();
    (width >= 3).then(|| (marker, width, trimmed[width..].trim()))
}

fn closes_fence(line: &str, marker: char, width: usize) -> bool {
    let trimmed = line.trim_start_matches(' ');
    line.len() - trimmed.len() <= 3
        && trimmed
            .chars()
            .take_while(|character| *character == marker)
            .count()
            >= width
        && trimmed.trim_matches(marker).trim().is_empty()
}

fn import_diagnostic(
    code: TaskImportDiagnosticCode,
    locator: &str,
    message: impl Into<String>,
    range: Range<usize>,
) -> TaskImportDiagnostic {
    TaskImportDiagnostic {
        code,
        locator: locator.to_owned(),
        message: message.into(),
        range: to_u64_range(range),
    }
}

fn ranges_overlap(left: &Range<u64>, right: &Range<u64>) -> bool {
    left.start < right.end && right.start < left.end
}

fn to_u64_range(range: Range<usize>) -> Range<u64> {
    range.start as u64..range.end as u64
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().fold(String::new(), |mut output, byte| {
        write!(output, "{byte:02x}").expect("string write");
        output
    })
}
