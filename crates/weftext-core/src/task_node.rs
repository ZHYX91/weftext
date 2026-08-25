use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;
use std::str::FromStr;

use serde::{Deserialize, Serialize, Serializer};
use weftext_asciidoc::{
    BlockKind, DiagnosticCode, DocumentHeaderAttribute, DocumentHeaderAttributeForm,
    DocumentHeaderIssueCode,
};

use crate::{DocumentRevision, NodeId, TaskNodeTemporal};

pub const TASK_NODE_PROFILE_VERSION: &str = "v1";
pub const TASK_NODE_PROFILE_MARKER: &str = TASK_NODE_PROFILE_VERSION;

const PROFILE_ATTRIBUTE: &str = "weftext-task";
const RESERVED_PREFIX: &str = "weftext-task-";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum TaskNodeProfileVersion {
    #[serde(rename = "v1")]
    V1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskNodeState {
    Todo,
    InProgress,
    OnHold,
    Completed,
    Cancelled,
}

impl TaskNodeState {
    #[must_use]
    pub const fn is_closed(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}

/// Canonical query rank is the declaration order: lowest through highest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodePriority {
    Lowest,
    Low,
    Normal,
    Medium,
    High,
    Highest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeAttributeKind {
    Profile,
    State,
    Priority,
    Created,
    Start,
    Scheduled,
    Due,
    Closed,
    DependsOn,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeAttributeForm {
    Set,
    Unset,
}

/// Exact source evidence for every reserved task-profile attribute in the document header.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeAttributeEvidence {
    pub kind: TaskNodeAttributeKind,
    pub name: String,
    pub literal_value: Option<String>,
    pub form: TaskNodeAttributeForm,
    pub range: Range<u64>,
    pub name_range: Range<u64>,
    pub value_range: Range<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeTitleEvidence {
    pub title: String,
    pub range: Range<u64>,
    pub text_range: Range<u64>,
}

/// Validated task-node v1 decoded fields. Node identity and document title remain outside this
/// closed attribute profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeProfile {
    pub profile: TaskNodeProfileVersion,
    pub state: TaskNodeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<TaskNodePriority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<TaskNodeTemporal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<TaskNodeTemporal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<TaskNodeTemporal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<TaskNodeTemporal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<TaskNodeTemporal>,
    #[serde(
        rename = "depends-on",
        skip_serializing_if = "Vec::is_empty",
        serialize_with = "serialize_dependencies"
    )]
    pub depends_on: Vec<NodeId>,
}

impl TaskNodeProfile {
    #[must_use]
    pub fn effective_priority(&self) -> TaskNodePriority {
        self.priority.unwrap_or(TaskNodePriority::Normal)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeDiagnosticCode {
    HeaderUnavailable,
    MissingDocumentTitle,
    AmbiguousDocumentTitle,
    MissingProfileMarker,
    UnsupportedProfile,
    InvalidReservedAttributeSyntax,
    DuplicateAttribute,
    UnknownAttribute,
    MissingState,
    InvalidState,
    InvalidPriority,
    InvalidTemporal,
    InvalidDependencies,
    DuplicateDependency,
    SelfDependency,
    ClosedOnOpenState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeDiagnostic {
    pub code: TaskNodeDiagnosticCode,
    pub range: Range<u64>,
    pub attribute: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeProfileAnalysis {
    pub profile_revision: DocumentRevision,
    pub declared: bool,
    pub has_reserved_evidence: bool,
    pub title: Option<TaskNodeTitleEvidence>,
    pub attributes: Vec<TaskNodeAttributeEvidence>,
    pub profile: Option<TaskNodeProfile>,
    pub diagnostics: Vec<TaskNodeDiagnostic>,
}

/// Validates the closed task-node v1 profile using only literal parser-owned header evidence.
///
/// Passing a node ID enables the source-local self-dependency check. Workspace eligibility,
/// dependency target resolution, authorization, and cycle checks belong to the later workspace
/// layer.
#[must_use]
pub fn analyze_task_node_profile(source: &str, node_id: Option<NodeId>) -> TaskNodeProfileAnalysis {
    let analysis = weftext_asciidoc::analyze(source);
    analyze_task_node_profile_analysis(source, node_id, &analysis)
}

/// Validates a task profile from an `AsciiDoc` analysis already produced for the exact source.
/// Workspace projection uses this crate-private entry point so checklist and task-node evidence
/// share one parser invocation.
#[allow(clippy::too_many_lines)]
pub(crate) fn analyze_task_node_profile_analysis(
    source: &str,
    node_id: Option<NodeId>,
    analysis: &weftext_asciidoc::Analysis,
) -> TaskNodeProfileAnalysis {
    let revision = DocumentRevision::from_source(source);
    let attributes = analysis
        .document_header
        .attributes
        .iter()
        .filter(|attribute| is_reserved_name(&attribute.name))
        .map(task_attribute_evidence)
        .collect::<Vec<_>>();
    let declared = attributes
        .iter()
        .any(|attribute| attribute.name == PROFILE_ATTRIBUTE);
    let has_reserved_evidence = !attributes.is_empty();

    if !has_reserved_evidence {
        let header_unavailable = analysis.document_header.issues.iter().any(|issue| {
            matches!(
                issue.code,
                DocumentHeaderIssueCode::ParserFailure
                    | DocumentHeaderIssueCode::UnclosedEnvelope
                    | DocumentHeaderIssueCode::AttributeLimitExceeded
            )
        });
        return TaskNodeProfileAnalysis {
            profile_revision: revision,
            declared,
            has_reserved_evidence,
            title: None,
            attributes,
            profile: None,
            diagnostics: if header_unavailable {
                vec![diagnostic(
                    TaskNodeDiagnosticCode::HeaderUnavailable,
                    analysis.document_header.range.clone(),
                    None,
                    "the complete document header is unavailable, so absence of a reserved task profile cannot be proven",
                )]
            } else {
                Vec::new()
            },
        };
    }

    let mut diagnostics = Vec::new();
    if analysis.document_header.issues.iter().any(|issue| {
        matches!(
            issue.code,
            DocumentHeaderIssueCode::ParserFailure
                | DocumentHeaderIssueCode::UnclosedEnvelope
                | DocumentHeaderIssueCode::AttributeLimitExceeded
        )
    }) {
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::HeaderUnavailable,
            analysis.document_header.range.clone(),
            None,
            "the complete document header is unavailable; the reserved task profile is invalid",
        ));
    }

    let title = task_title_evidence(analysis, &mut diagnostics);
    let mut by_name = BTreeMap::<&str, Vec<&DocumentHeaderAttribute>>::new();
    for attribute in analysis
        .document_header
        .attributes
        .iter()
        .filter(|attribute| is_reserved_name(&attribute.name))
    {
        by_name.entry(&attribute.name).or_default().push(attribute);
        if attribute.form != DocumentHeaderAttributeForm::Set
            || !attribute.continuation_ranges.is_empty()
            || attribute.literal_value.is_none()
        {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::InvalidReservedAttributeSyntax,
                attribute.range.clone(),
                Some(attribute.name.clone()),
                "task-profile attributes must be bounded literal set entries in the document header",
            ));
        }
        if task_attribute_kind(&attribute.name) == TaskNodeAttributeKind::Unknown {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::UnknownAttribute,
                attribute.name_range.clone(),
                Some(attribute.name.clone()),
                "unknown weftext-task-* attribute is not permitted by the closed v1 profile",
            ));
        }
    }
    for (name, occurrences) in &by_name {
        for duplicate in occurrences.iter().skip(1) {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::DuplicateAttribute,
                duplicate.range.clone(),
                Some((*name).to_owned()),
                "task-profile attributes may occur at most once",
            ));
        }
    }

    let profile_marker = single_literal(&by_name, PROFILE_ATTRIBUTE);
    match profile_marker {
        None => diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::MissingProfileMarker,
            analysis.document_header.range.clone(),
            Some(PROFILE_ATTRIBUTE.to_owned()),
            "reserved task attributes require exactly one :weftext-task: v1 marker",
        )),
        Some((value, range)) if value != TASK_NODE_PROFILE_MARKER => diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::UnsupportedProfile,
            range,
            Some(PROFILE_ATTRIBUTE.to_owned()),
            "only the exact weftext-task profile marker v1 is supported",
        )),
        Some(_) => {}
    }

    let state = match single_literal(&by_name, "weftext-task-state") {
        None => {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::MissingState,
                analysis.document_header.range.clone(),
                Some("weftext-task-state".to_owned()),
                "task-node v1 requires exactly one state attribute",
            ));
            None
        }
        Some((value, range)) => parse_state(value).or_else(|| {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::InvalidState,
                range,
                Some("weftext-task-state".to_owned()),
                "task-node state must be todo, in-progress, on-hold, completed, or cancelled",
            ));
            None
        }),
    };
    let priority = optional_priority(&by_name, &mut diagnostics);
    let created = optional_temporal("weftext-task-created", &by_name, &mut diagnostics);
    let start = optional_temporal("weftext-task-start", &by_name, &mut diagnostics);
    let scheduled = optional_temporal("weftext-task-scheduled", &by_name, &mut diagnostics);
    let due = optional_temporal("weftext-task-due", &by_name, &mut diagnostics);
    let closed = optional_temporal("weftext-task-closed", &by_name, &mut diagnostics);
    let depends_on = dependencies(&by_name, node_id, &mut diagnostics);

    if state.is_some_and(|state| !state.is_closed()) && closed.is_some() {
        let range = single_literal(&by_name, "weftext-task-closed").map_or_else(
            || analysis.document_header.range.clone(),
            |(_, range)| range,
        );
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::ClosedOnOpenState,
            range,
            Some("weftext-task-closed".to_owned()),
            "open task-node states cannot carry a closed date or instant",
        ));
    }

    let profile = if diagnostics.is_empty() {
        state.map(|state| TaskNodeProfile {
            profile: TaskNodeProfileVersion::V1,
            state,
            priority,
            created,
            start,
            scheduled,
            due,
            closed,
            depends_on,
        })
    } else {
        None
    };
    TaskNodeProfileAnalysis {
        profile_revision: revision,
        declared,
        has_reserved_evidence,
        title,
        attributes,
        profile,
        diagnostics,
    }
}

fn task_title_evidence(
    analysis: &weftext_asciidoc::Analysis,
    diagnostics: &mut Vec<TaskNodeDiagnostic>,
) -> Option<TaskNodeTitleEvidence> {
    let titles = analysis
        .blocks
        .iter()
        .filter(|block| {
            block.kind == BlockKind::DocumentTitle
                && analysis.document_header.range.start <= block.range.start
                && block.range.end <= analysis.document_header.range.end
        })
        .collect::<Vec<_>>();
    if analysis
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::AdditionalDocumentTitle)
        || titles.len() > 1
    {
        let range = analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::AdditionalDocumentTitle)
            .map_or_else(
                || analysis.document_header.range.clone(),
                |diagnostic| diagnostic.range.clone(),
            );
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::AmbiguousDocumentTitle,
            range,
            None,
            "the task node must have exactly one parser-confirmed document title",
        ));
        return None;
    }
    let Some(title) = titles.first() else {
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::MissingDocumentTitle,
            analysis.document_header.range.clone(),
            None,
            "the task node requires a parser-confirmed document title",
        ));
        return None;
    };
    if title.text.trim().is_empty() {
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::MissingDocumentTitle,
            title.text_range.clone(),
            None,
            "the task-node document title cannot be empty",
        ));
        return None;
    }
    Some(TaskNodeTitleEvidence {
        title: title.text.clone(),
        range: title.range.clone(),
        text_range: title.text_range.clone(),
    })
}

fn task_attribute_evidence(attribute: &DocumentHeaderAttribute) -> TaskNodeAttributeEvidence {
    TaskNodeAttributeEvidence {
        kind: task_attribute_kind(&attribute.name),
        name: attribute.name.clone(),
        literal_value: attribute.literal_value.clone(),
        form: match attribute.form {
            DocumentHeaderAttributeForm::Set => TaskNodeAttributeForm::Set,
            DocumentHeaderAttributeForm::Unset => TaskNodeAttributeForm::Unset,
        },
        range: attribute.range.clone(),
        name_range: attribute.name_range.clone(),
        value_range: attribute.value_range.clone(),
    }
}

fn task_attribute_kind(name: &str) -> TaskNodeAttributeKind {
    match name {
        PROFILE_ATTRIBUTE => TaskNodeAttributeKind::Profile,
        "weftext-task-state" => TaskNodeAttributeKind::State,
        "weftext-task-priority" => TaskNodeAttributeKind::Priority,
        "weftext-task-created" => TaskNodeAttributeKind::Created,
        "weftext-task-start" => TaskNodeAttributeKind::Start,
        "weftext-task-scheduled" => TaskNodeAttributeKind::Scheduled,
        "weftext-task-due" => TaskNodeAttributeKind::Due,
        "weftext-task-closed" => TaskNodeAttributeKind::Closed,
        "weftext-task-depends-on" => TaskNodeAttributeKind::DependsOn,
        _ => TaskNodeAttributeKind::Unknown,
    }
}

fn single_literal<'a>(
    by_name: &'a BTreeMap<&str, Vec<&DocumentHeaderAttribute>>,
    name: &str,
) -> Option<(&'a str, Range<u64>)> {
    let attributes = by_name.get(name)?;
    if attributes.len() != 1 {
        return None;
    }
    let attribute = attributes[0];
    (attribute.form == DocumentHeaderAttributeForm::Set && attribute.continuation_ranges.is_empty())
        .then(|| {
            attribute
                .literal_value
                .as_deref()
                .map(|value| (value, attribute.value_range.clone()))
        })
        .flatten()
}

fn optional_priority(
    by_name: &BTreeMap<&str, Vec<&DocumentHeaderAttribute>>,
    diagnostics: &mut Vec<TaskNodeDiagnostic>,
) -> Option<TaskNodePriority> {
    let (value, range) = single_literal(by_name, "weftext-task-priority")?;
    parse_priority(value).or_else(|| {
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::InvalidPriority,
            range,
            Some("weftext-task-priority".to_owned()),
            "task-node priority must be lowest, low, normal, medium, high, or highest",
        ));
        None
    })
}

fn optional_temporal(
    name: &str,
    by_name: &BTreeMap<&str, Vec<&DocumentHeaderAttribute>>,
    diagnostics: &mut Vec<TaskNodeDiagnostic>,
) -> Option<TaskNodeTemporal> {
    let (value, range) = single_literal(by_name, name)?;
    TaskNodeTemporal::parse(value).map_or_else(
        |_| {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::InvalidTemporal,
                range,
                Some(name.to_owned()),
                "task-node time must be an ISO date or uppercase-T RFC 3339 instant with an explicit offset",
            ));
            None
        },
        Some,
    )
}

fn dependencies(
    by_name: &BTreeMap<&str, Vec<&DocumentHeaderAttribute>>,
    node_id: Option<NodeId>,
    diagnostics: &mut Vec<TaskNodeDiagnostic>,
) -> Vec<NodeId> {
    let Some((value, range)) = single_literal(by_name, "weftext-task-depends-on") else {
        return Vec::new();
    };
    if value.is_empty() || value.split(' ').any(str::is_empty) {
        diagnostics.push(diagnostic(
            TaskNodeDiagnosticCode::InvalidDependencies,
            range,
            Some("weftext-task-depends-on".to_owned()),
            "depends-on must contain lowercase UUIDv4 node IDs separated by one ASCII space",
        ));
        return Vec::new();
    }
    let mut dependencies = Vec::new();
    let mut seen = BTreeSet::new();
    for value in value.split(' ') {
        let Ok(dependency) = NodeId::from_str(value) else {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::InvalidDependencies,
                range.clone(),
                Some("weftext-task-depends-on".to_owned()),
                "depends-on contains a node ID that is not a canonical lowercase UUIDv4",
            ));
            continue;
        };
        if !seen.insert(dependency) {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::DuplicateDependency,
                range.clone(),
                Some("weftext-task-depends-on".to_owned()),
                "depends-on contains the same node ID more than once",
            ));
            continue;
        }
        if node_id == Some(dependency) {
            diagnostics.push(diagnostic(
                TaskNodeDiagnosticCode::SelfDependency,
                range.clone(),
                Some("weftext-task-depends-on".to_owned()),
                "a task node cannot depend on itself",
            ));
            continue;
        }
        dependencies.push(dependency);
    }
    dependencies
}

fn parse_state(value: &str) -> Option<TaskNodeState> {
    match value {
        "todo" => Some(TaskNodeState::Todo),
        "in-progress" => Some(TaskNodeState::InProgress),
        "on-hold" => Some(TaskNodeState::OnHold),
        "completed" => Some(TaskNodeState::Completed),
        "cancelled" => Some(TaskNodeState::Cancelled),
        _ => None,
    }
}

fn parse_priority(value: &str) -> Option<TaskNodePriority> {
    match value {
        "lowest" => Some(TaskNodePriority::Lowest),
        "low" => Some(TaskNodePriority::Low),
        "normal" => Some(TaskNodePriority::Normal),
        "medium" => Some(TaskNodePriority::Medium),
        "high" => Some(TaskNodePriority::High),
        "highest" => Some(TaskNodePriority::Highest),
        _ => None,
    }
}

fn is_reserved_name(name: &str) -> bool {
    name == PROFILE_ATTRIBUTE || name.starts_with(RESERVED_PREFIX)
}

fn diagnostic(
    code: TaskNodeDiagnosticCode,
    range: Range<u64>,
    attribute: Option<String>,
    message: &str,
) -> TaskNodeDiagnostic {
    TaskNodeDiagnostic {
        code,
        range,
        attribute,
        message: message.to_owned(),
    }
}

fn serialize_dependencies<S>(dependencies: &[NodeId], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    dependencies
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
        .serialize(serializer)
}

/// Builds the one canonical managed-document source shape used when Core creates a task node.
///
/// This is deliberately a pure crate-internal helper. Callers must still validate the reviewed
/// title, body/context geometry, destination occupancy, dependency graph, and the completed source
/// with [`analyze_task_node_profile`]. Keeping source construction here prevents promotion,
/// rebaseline, and later import transactions from growing independent task-profile writers.
pub(crate) fn build_task_node_document_source(
    node_id: NodeId,
    title: &str,
    profile: &TaskNodeProfile,
    body: &str,
) -> Option<(String, usize)> {
    if profile.profile != TaskNodeProfileVersion::V1 {
        return None;
    }
    let mut source = weftext_asciidoc::new_managed_document_envelope(node_id.as_uuid()).ok()?;
    source.push_str("= ");
    source.push_str(title);
    source.push('\n');
    source.push_str(":weftext-task: v1\n:weftext-task-state: ");
    source.push_str(task_node_state_source(profile.state));
    source.push('\n');
    if let Some(priority) = profile.priority {
        source.push_str(":weftext-task-priority: ");
        source.push_str(task_node_priority_source(priority));
        source.push('\n');
    }
    for (name, value) in [
        ("created", profile.created.as_ref()),
        ("start", profile.start.as_ref()),
        ("scheduled", profile.scheduled.as_ref()),
        ("due", profile.due.as_ref()),
        ("closed", profile.closed.as_ref()),
    ] {
        if let Some(value) = value {
            source.push_str(":weftext-task-");
            source.push_str(name);
            source.push_str(": ");
            source.push_str(value.as_str());
            source.push('\n');
        }
    }
    if !profile.depends_on.is_empty() {
        source.push_str(":weftext-task-depends-on: ");
        source.push_str(
            &profile
                .depends_on
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" "),
        );
        source.push('\n');
    }
    source.push('\n');
    let body_start = source.len();
    source.push_str(body);
    Some((source, body_start))
}

const fn task_node_state_source(state: TaskNodeState) -> &'static str {
    match state {
        TaskNodeState::Todo => "todo",
        TaskNodeState::InProgress => "in-progress",
        TaskNodeState::OnHold => "on-hold",
        TaskNodeState::Completed => "completed",
        TaskNodeState::Cancelled => "cancelled",
    }
}

const fn task_node_priority_source(priority: TaskNodePriority) -> &'static str {
    match priority {
        TaskNodePriority::Lowest => "lowest",
        TaskNodePriority::Low => "low",
        TaskNodePriority::Normal => "normal",
        TaskNodePriority::Medium => "medium",
        TaskNodePriority::High => "high",
        TaskNodePriority::Highest => "highest",
    }
}
