use std::collections::BTreeMap;
use std::fmt;
use std::ops::Range;
use std::path::Path;

use serde::Serialize;

use crate::checklist::{ChecklistDiagnosticCode, analyze_checklist_analysis};
use crate::task_dependency_graph::{
    TaskGraphDiagnostic, TaskGraphDiagnosticCode, TaskGraphNode, TaskGraphPolicy,
    TaskGraphResolution, TaskGraphTargetClassification, resolve_task_dependency_graph,
};
use crate::task_node::analyze_task_node_profile_analysis;
use crate::{
    ChecklistMarker, ChecklistParserOccurrence, ChecklistState, DocumentRevision,
    InventoryIssueCode, NodeId, TaskNodeAttributeKind, TaskNodeDiagnosticCode, TaskNodePriority,
    TaskNodeProfile, TaskNodeState, TaskNodeTemporal, WorkspaceDocumentGeneration,
    WorkspaceInventory, WorkspaceReadScope, scan_workspace,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRowKind {
    Checklist,
    Node,
}

/// Revision-bound action evidence for one projected task row.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum TaskRowEvidence {
    Checklist {
        owner_node_id: NodeId,
        revision: DocumentRevision,
        occurrence: ChecklistParserOccurrence,
        authored_marker: ChecklistMarker,
        item_range: Range<u64>,
        marker_range: Range<u64>,
        description_range: Range<u64>,
    },
    Node {
        node_id: NodeId,
        revision: DocumentRevision,
        profile_revision: DocumentRevision,
    },
}

/// Canonical tagged checklist/task-node projection consumed by the future tasks query source.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRow {
    pub kind: TaskRowKind,
    pub id: Option<NodeId>,
    pub owner_node_id: NodeId,
    pub owner_node_name: String,
    pub owner_node_path: String,
    pub description: String,
    pub closed: bool,
    pub state: TaskNodeState,
    pub checklist_depth: Option<u16>,
    pub priority: Option<TaskNodePriority>,
    pub created: Option<TaskNodeTemporal>,
    pub start: Option<TaskNodeTemporal>,
    pub scheduled: Option<TaskNodeTemporal>,
    pub due: Option<TaskNodeTemporal>,
    pub closed_at: Option<TaskNodeTemporal>,
    pub blocked: Option<bool>,
    pub evidence: TaskRowEvidence,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskWorkspaceProjectionDiagnosticCode {
    ChecklistParserFailure,
    IncompleteChecklistBranch,
    InvalidTaskProfile,
    MissingTaskTitle,
    UnresolvedDependency,
    NonTaskDependency,
    InvalidDependencyTarget,
    DependencyCycle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWorkspaceProjectionDiagnostic {
    pub code: TaskWorkspaceProjectionDiagnosticCode,
    pub message: String,
    pub node_id: NodeId,
    pub dependency_id: Option<NodeId>,
    pub range: Range<u64>,
    pub related_node_ids: Vec<NodeId>,
    pub task_node_code: Option<TaskNodeDiagnosticCode>,
    pub checklist_code: Option<ChecklistDiagnosticCode>,
}

#[derive(Clone, Debug)]
pub struct TaskWorkspaceProjection {
    generation: WorkspaceDocumentGeneration,
    rows: Vec<TaskRow>,
    diagnostics: Vec<TaskWorkspaceProjectionDiagnostic>,
    revisions: BTreeMap<NodeId, DocumentRevision>,
    candidates: BTreeMap<NodeId, Candidate>,
    targets: BTreeMap<NodeId, TaskGraphTargetClassification>,
    local_diagnostics: Vec<TaskWorkspaceProjectionDiagnostic>,
    graph_policy: TaskGraphPolicy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TaskProjectionAccessMode {
    Complete,
    Filtered,
}

#[derive(Clone, Debug)]
pub(crate) struct TaskWorkspaceProjectionView {
    pub(crate) rows: Vec<TaskRow>,
    pub(crate) diagnostics: Vec<TaskWorkspaceProjectionDiagnostic>,
}

impl TaskWorkspaceProjection {
    /// Rebuilds the complete canonical task projection from active managed `AsciiDoc` nodes.
    /// Inventory validation finishes before any complete document body is opened.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid workspace authority or an unreadable active managed document.
    pub fn rebuild(root: impl AsRef<Path>) -> Result<Self, TaskWorkspaceProjectionError> {
        Self::rebuild_internal(root.as_ref(), None)
    }

    /// Rebuilds only from an already-authorized logical node projection. Hidden document bodies
    /// are never opened, and hidden versus missing dependency targets remain indistinguishable.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid global/scope authority or an unreadable authorized document.
    pub fn rebuild_scoped(
        root: impl AsRef<Path>,
        scope: &WorkspaceReadScope,
    ) -> Result<Self, TaskWorkspaceProjectionError> {
        Self::rebuild_internal(root.as_ref(), Some(scope))
    }

    #[allow(clippy::too_many_lines)]
    fn rebuild_internal(
        root: &Path,
        scope: Option<&WorkspaceReadScope>,
    ) -> Result<Self, TaskWorkspaceProjectionError> {
        let inventory = scan_workspace(root);
        if let Some(scope) = scope {
            scope
                .validate_inventory(&inventory)
                .map_err(|_| TaskWorkspaceProjectionError::InvalidScope)?;
        } else if !inventory.is_valid() {
            return Err(TaskWorkspaceProjectionError::InvalidWorkspace(
                inventory
                    .issues
                    .first()
                    .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
            ));
        }
        Self::rebuild_from_validated_inventory(root, &inventory, scope)
    }

    /// Builds from inventory whose complete/scoped authority was already validated by the caller.
    /// This is crate-private so a composite derived index can share one scan without weakening the
    /// public validation boundary.
    #[allow(clippy::too_many_lines)]
    pub(crate) fn rebuild_from_validated_inventory(
        root: &Path,
        inventory: &WorkspaceInventory,
        scope: Option<&WorkspaceReadScope>,
    ) -> Result<Self, TaskWorkspaceProjectionError> {
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
            return Err(TaskWorkspaceProjectionError::UnsupportedGeneration(
                inventory.generation,
            ));
        }

        let mut rows = Vec::new();
        let mut diagnostics = Vec::new();
        let mut targets = BTreeMap::new();
        let mut candidates = BTreeMap::new();
        let mut revisions = BTreeMap::new();

        for node in &inventory.nodes {
            if crate::workspace_trash::is_trash_storage_path(root, &node.path) {
                continue;
            }
            let Some(node_id) = node.id else {
                if scope.is_some() {
                    continue;
                }
                return Err(TaskWorkspaceProjectionError::InvalidWorkspace(
                    InventoryIssueCode::MissingIdentity,
                ));
            };
            if scope.is_some_and(|scope| !scope.allows(node_id)) {
                continue;
            }

            let snapshot = crate::read_node_document(&node.path)
                .map_err(|_| TaskWorkspaceProjectionError::DocumentRead { node_id })?;
            if snapshot.node_id != node_id {
                return Err(TaskWorkspaceProjectionError::DocumentRead { node_id });
            }
            revisions.insert(node_id, snapshot.revision.clone());
            let parser_analysis = weftext_asciidoc::analyze(&snapshot.source);
            let checklist_analysis = analyze_checklist_analysis(&parser_analysis);
            let task_analysis = analyze_task_node_profile_analysis(
                &snapshot.source,
                Some(node_id),
                &parser_analysis,
            );
            let (owner_name, owner_path) = owner_placement(root, node, node_id, scope)?;

            for checklist_diagnostic in checklist_analysis.diagnostics {
                diagnostics.push(TaskWorkspaceProjectionDiagnostic {
                    code: match checklist_diagnostic.code {
                        ChecklistDiagnosticCode::ParserFailure => {
                            TaskWorkspaceProjectionDiagnosticCode::ChecklistParserFailure
                        }
                        ChecklistDiagnosticCode::IncompleteParserBranch => {
                            TaskWorkspaceProjectionDiagnosticCode::IncompleteChecklistBranch
                        }
                    },
                    message: checklist_diagnostic.message,
                    node_id,
                    dependency_id: None,
                    range: checklist_diagnostic.range,
                    related_node_ids: Vec::new(),
                    task_node_code: None,
                    checklist_code: Some(checklist_diagnostic.code),
                });
            }
            for checklist in checklist_analysis.occurrences {
                let (state, closed) = match checklist.state {
                    ChecklistState::Todo => (TaskNodeState::Todo, false),
                    ChecklistState::Completed => (TaskNodeState::Completed, true),
                };
                rows.push(TaskRow {
                    kind: TaskRowKind::Checklist,
                    id: None,
                    owner_node_id: node_id,
                    owner_node_name: owner_name.clone(),
                    owner_node_path: owner_path.clone(),
                    description: checklist.description,
                    closed,
                    state,
                    checklist_depth: Some(u16::from(checklist.list_depth)),
                    priority: None,
                    created: None,
                    start: None,
                    scheduled: None,
                    due: None,
                    closed_at: None,
                    blocked: None,
                    evidence: TaskRowEvidence::Checklist {
                        owner_node_id: node_id,
                        revision: snapshot.revision.clone(),
                        occurrence: checklist.parser_occurrence,
                        authored_marker: checklist.authored_marker,
                        item_range: checklist.item_range,
                        marker_range: checklist.marker_range,
                        description_range: checklist.description_range,
                    },
                });
            }

            let task_eligible = node.path != inventory.root;
            if task_analysis.has_reserved_evidence {
                for task_diagnostic in &task_analysis.diagnostics {
                    diagnostics.push(profile_diagnostic(node_id, task_diagnostic));
                }
                if !task_eligible && task_analysis.profile.is_some() {
                    diagnostics.push(TaskWorkspaceProjectionDiagnostic {
                        code: TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile,
                        message: "the workspace root cannot carry the task-node profile".to_owned(),
                        node_id,
                        dependency_id: None,
                        range: task_analysis
                            .attributes
                            .first()
                            .map_or(0..0, |attribute| attribute.range.clone()),
                        related_node_ids: Vec::new(),
                        task_node_code: None,
                        checklist_code: None,
                    });
                }
            }

            let classification = if !task_eligible {
                TaskGraphTargetClassification::Invalid
            } else if let (Some(profile), Some(title)) =
                (task_analysis.profile.clone(), task_analysis.title.clone())
            {
                let dependency_range = task_analysis
                    .attributes
                    .iter()
                    .find(|attribute| attribute.kind == TaskNodeAttributeKind::DependsOn)
                    .map_or(0..0, |attribute| attribute.value_range.clone());
                candidates.insert(
                    node_id,
                    Candidate {
                        node_id,
                        revision: snapshot.revision.clone(),
                        profile_revision: task_analysis.profile_revision.clone(),
                        owner_name,
                        owner_path,
                        title: title.title,
                        profile,
                        dependency_range,
                    },
                );
                TaskGraphTargetClassification::Valid
            } else if task_analysis.has_reserved_evidence {
                TaskGraphTargetClassification::Invalid
            } else {
                TaskGraphTargetClassification::NonTask
            };
            targets.insert(node_id, classification);
        }

        let policy = if scope.is_some() {
            TaskGraphPolicy::ProjectionScoped
        } else {
            TaskGraphPolicy::ProjectionComplete
        };
        let local_diagnostics = diagnostics.clone();
        let graph = resolve_projection_graph(&candidates, &targets, policy);
        diagnostics.extend(graph.diagnostics.iter().map(graph_diagnostic));
        for (node_id, candidate) in &candidates {
            if !graph.valid_node_ids.contains(node_id) {
                continue;
            }
            rows.push(candidate.row(graph.blocked_node_ids.contains(node_id)));
        }

        rows.sort_by(compare_rows);
        sort_diagnostics(&mut diagnostics);
        Ok(Self {
            generation: inventory.generation,
            rows,
            diagnostics,
            revisions,
            candidates,
            targets,
            local_diagnostics,
            graph_policy: policy,
        })
    }

    #[must_use]
    pub const fn generation(&self) -> WorkspaceDocumentGeneration {
        self.generation
    }

    #[must_use]
    pub fn rows(&self) -> &[TaskRow] {
        &self.rows
    }

    pub fn rows_for_owner_node(&self, node_id: NodeId) -> impl Iterator<Item = &TaskRow> {
        self.rows
            .iter()
            .filter(move |row| row.owner_node_id == node_id)
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[TaskWorkspaceProjectionDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn task_node_row(&self, node_id: NodeId) -> Option<&TaskRow> {
        self.rows
            .iter()
            .find(|row| row.kind == TaskRowKind::Node && row.id == Some(node_id))
    }

    pub(crate) fn document_revision(&self, node_id: NodeId) -> Option<&DocumentRevision> {
        self.revisions.get(&node_id)
    }

    /// Re-derives graph-sensitive rows and diagnostics for a narrower execution authority without
    /// reading or parsing document bodies again. A filtered access scope can only preserve or
    /// narrow the projection's build-time disclosure mode; it can never upgrade a scoped build to
    /// complete graph knowledge.
    pub(crate) fn derive_for_access(
        &self,
        access_mode: TaskProjectionAccessMode,
        allows: impl Fn(NodeId) -> bool,
    ) -> TaskWorkspaceProjectionView {
        let candidates = self
            .candidates
            .iter()
            .filter(|(node_id, _)| allows(**node_id))
            .map(|(node_id, candidate)| (*node_id, candidate.clone()))
            .collect::<BTreeMap<_, _>>();
        let targets = self
            .targets
            .iter()
            .filter(|(node_id, _)| allows(**node_id))
            .map(|(node_id, classification)| (*node_id, *classification))
            .collect::<BTreeMap<_, _>>();
        let mut diagnostics = self
            .local_diagnostics
            .iter()
            .filter(|diagnostic| allows(diagnostic.node_id))
            .cloned()
            .collect::<Vec<_>>();
        let policy = if self.graph_policy == TaskGraphPolicy::ProjectionComplete
            && access_mode == TaskProjectionAccessMode::Complete
        {
            TaskGraphPolicy::ProjectionComplete
        } else {
            TaskGraphPolicy::ProjectionScoped
        };
        let graph = resolve_projection_graph(&candidates, &targets, policy);
        diagnostics.extend(graph.diagnostics.iter().map(graph_diagnostic));
        let mut rows = self
            .rows
            .iter()
            .filter(|row| row.kind == TaskRowKind::Checklist && allows(row.owner_node_id))
            .cloned()
            .collect::<Vec<_>>();
        for (node_id, candidate) in &candidates {
            if !graph.valid_node_ids.contains(node_id) {
                continue;
            }
            rows.push(candidate.row(graph.blocked_node_ids.contains(node_id)));
        }
        rows.sort_by(compare_rows);
        sort_diagnostics(&mut diagnostics);
        TaskWorkspaceProjectionView { rows, diagnostics }
    }
}

#[derive(Clone, Debug)]
struct Candidate {
    node_id: NodeId,
    revision: DocumentRevision,
    profile_revision: DocumentRevision,
    owner_name: String,
    owner_path: String,
    title: String,
    profile: TaskNodeProfile,
    dependency_range: Range<u64>,
}

impl Candidate {
    fn row(&self, blocked: bool) -> TaskRow {
        debug_assert_eq!(self.revision, self.profile_revision);
        TaskRow {
            kind: TaskRowKind::Node,
            id: Some(self.node_id),
            owner_node_id: self.node_id,
            owner_node_name: self.owner_name.clone(),
            owner_node_path: self.owner_path.clone(),
            description: self.title.clone(),
            closed: self.profile.state.is_closed(),
            state: self.profile.state,
            checklist_depth: None,
            priority: Some(self.profile.effective_priority()),
            created: self.profile.created.clone(),
            start: self.profile.start.clone(),
            scheduled: self.profile.scheduled.clone(),
            due: self.profile.due.clone(),
            closed_at: self.profile.closed.clone(),
            blocked: Some(blocked),
            evidence: TaskRowEvidence::Node {
                node_id: self.node_id,
                revision: self.revision.clone(),
                profile_revision: self.profile_revision.clone(),
            },
        }
    }
}

fn owner_placement(
    root: &Path,
    node: &crate::NodeRecord,
    node_id: NodeId,
    scope: Option<&WorkspaceReadScope>,
) -> Result<(String, String), TaskWorkspaceProjectionError> {
    if let Some(scope) = scope {
        let locator = scope
            .locator(node_id)
            .ok_or(TaskWorkspaceProjectionError::InvalidScope)?;
        let name = locator
            .rsplit_once('/')
            .map_or(locator, |(_, name)| name)
            .to_owned();
        return Ok((
            if name.is_empty() {
                node.name.clone()
            } else {
                name
            },
            if locator.is_empty() {
                "/".to_owned()
            } else {
                format!("/{locator}")
            },
        ));
    }
    let relative = node
        .path
        .strip_prefix(root)
        .map_err(|_| TaskWorkspaceProjectionError::InvalidNodeLocator)?;
    let components = relative
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .ok_or(TaskWorkspaceProjectionError::InvalidNodeLocator)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        node.name.clone(),
        if components.is_empty() {
            "/".to_owned()
        } else {
            format!("/{}", components.join("/"))
        },
    ))
}

fn profile_diagnostic(
    node_id: NodeId,
    diagnostic: &crate::TaskNodeDiagnostic,
) -> TaskWorkspaceProjectionDiagnostic {
    TaskWorkspaceProjectionDiagnostic {
        code: if diagnostic.code == TaskNodeDiagnosticCode::MissingDocumentTitle {
            TaskWorkspaceProjectionDiagnosticCode::MissingTaskTitle
        } else {
            TaskWorkspaceProjectionDiagnosticCode::InvalidTaskProfile
        },
        message: diagnostic.message.clone(),
        node_id,
        dependency_id: None,
        range: diagnostic.range.clone(),
        related_node_ids: Vec::new(),
        task_node_code: Some(diagnostic.code),
        checklist_code: None,
    }
}

fn resolve_projection_graph(
    candidates: &BTreeMap<NodeId, Candidate>,
    targets: &BTreeMap<NodeId, TaskGraphTargetClassification>,
    policy: TaskGraphPolicy,
) -> TaskGraphResolution {
    let nodes = candidates
        .iter()
        .map(|(node_id, candidate)| {
            (
                *node_id,
                TaskGraphNode {
                    node_id: *node_id,
                    state: candidate.profile.state,
                    depends_on: candidate.profile.depends_on.clone(),
                    dependency_range: candidate.dependency_range.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    resolve_task_dependency_graph(&nodes, targets, policy)
}

fn graph_diagnostic(diagnostic: &TaskGraphDiagnostic) -> TaskWorkspaceProjectionDiagnostic {
    TaskWorkspaceProjectionDiagnostic {
        code: match diagnostic.code {
            TaskGraphDiagnosticCode::UnresolvedDependency => {
                TaskWorkspaceProjectionDiagnosticCode::UnresolvedDependency
            }
            TaskGraphDiagnosticCode::NonTaskDependency => {
                TaskWorkspaceProjectionDiagnosticCode::NonTaskDependency
            }
            TaskGraphDiagnosticCode::InvalidDependencyTarget => {
                TaskWorkspaceProjectionDiagnosticCode::InvalidDependencyTarget
            }
            TaskGraphDiagnosticCode::DependencyCycle => {
                TaskWorkspaceProjectionDiagnosticCode::DependencyCycle
            }
        },
        message: diagnostic.message.clone(),
        node_id: diagnostic.source_node_id,
        dependency_id: diagnostic.target_node_id,
        range: diagnostic.range.clone(),
        related_node_ids: diagnostic.related_node_ids.clone(),
        task_node_code: None,
        checklist_code: None,
    }
}

fn compare_rows(left: &TaskRow, right: &TaskRow) -> std::cmp::Ordering {
    left.owner_node_path
        .cmp(&right.owner_node_path)
        .then_with(|| left.kind.cmp(&right.kind))
        .then_with(|| row_source_start(left).cmp(&row_source_start(right)))
        .then_with(|| left.owner_node_id.cmp(&right.owner_node_id))
}

fn row_source_start(row: &TaskRow) -> u64 {
    match &row.evidence {
        TaskRowEvidence::Checklist { item_range, .. } => item_range.start,
        TaskRowEvidence::Node { .. } => u64::MAX,
    }
}

fn sort_diagnostics(diagnostics: &mut [TaskWorkspaceProjectionDiagnostic]) {
    diagnostics.sort_by(|left, right| {
        left.node_id
            .cmp(&right.node_id)
            .then_with(|| left.range.start.cmp(&right.range.start))
            .then_with(|| left.range.end.cmp(&right.range.end))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.dependency_id.cmp(&right.dependency_id))
            .then_with(|| left.related_node_ids.cmp(&right.related_node_ids))
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskWorkspaceProjectionError {
    InvalidWorkspace(InventoryIssueCode),
    UnsupportedGeneration(WorkspaceDocumentGeneration),
    DocumentRead { node_id: NodeId },
    InvalidScope,
    InvalidNodeLocator,
}

impl fmt::Display for TaskWorkspaceProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::UnsupportedGeneration(generation) => {
                write!(
                    formatter,
                    "task projection requires AsciiDoc v1, got {generation:?}"
                )
            }
            Self::DocumentRead { node_id } => {
                write!(
                    formatter,
                    "authorized node {node_id} document is unavailable"
                )
            }
            Self::InvalidScope => formatter.write_str("task read scope is invalid"),
            Self::InvalidNodeLocator => formatter.write_str("managed node locator is invalid"),
        }
    }
}

impl std::error::Error for TaskWorkspaceProjectionError {}
