use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::path::Path;

use serde::Serialize;

use crate::{
    DocumentRevision, InventoryIssueCode, NodeId, TaskDiagnosticCode, TaskId, TaskOccurrence,
    WorkspaceDocumentGeneration, analyze_task_source, scan_workspace,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWorkspaceOccurrence {
    pub node_id: NodeId,
    pub revision: DocumentRevision,
    pub task: TaskOccurrence,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskWorkspaceDiagnosticCode {
    InvalidTaskSyntax,
    DuplicateTaskId,
    UnresolvedDependency,
    AmbiguousDependency,
    InvalidDependencyTarget,
    DependencyCycle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskWorkspaceDiagnostic {
    pub code: TaskWorkspaceDiagnosticCode,
    pub message: String,
    pub node_id: NodeId,
    pub task_id: Option<TaskId>,
    pub dependency_id: Option<TaskId>,
    pub range: Range<u64>,
    pub related_node_ids: Vec<NodeId>,
    pub related_task_ids: Vec<TaskId>,
    pub task_code: Option<TaskDiagnosticCode>,
}

#[derive(Clone, Debug)]
pub struct TaskWorkspaceIndex {
    generation: WorkspaceDocumentGeneration,
    occurrences: Vec<TaskWorkspaceOccurrence>,
    declarations: BTreeMap<TaskId, Vec<usize>>,
    diagnostics: Vec<TaskWorkspaceDiagnostic>,
}

impl TaskWorkspaceIndex {
    /// Rebuilds all task occurrences and their dependency graph from exact managed `AsciiDoc`
    /// source. The index is derived and does not become task authority.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace, a non-AsciiDoc generation, or a document that
    /// cannot be reopened through the selected Core generation boundary.
    pub fn rebuild(root: impl AsRef<Path>) -> Result<Self, TaskWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), None, None)
    }

    /// Rebuilds task data only from nodes in an already-authorized projection.
    /// The scope check occurs before any document body is opened.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid workspace/scope or an unreadable
    /// authorized document.
    pub fn rebuild_scoped(
        root: impl AsRef<Path>,
        scope: &crate::WorkspaceReadScope,
    ) -> Result<Self, TaskWorkspaceError> {
        Self::rebuild_internal(root.as_ref(), None, Some(scope))
    }

    pub(crate) fn rebuild_with_replacement(
        root: &Path,
        node_id: NodeId,
        proposed_source: &str,
    ) -> Result<Self, TaskWorkspaceError> {
        Self::rebuild_internal(root, Some((node_id, proposed_source)), None)
    }

    pub(crate) fn rebuild_scoped_with_replacement(
        root: &Path,
        node_id: NodeId,
        proposed_source: &str,
        scope: &crate::WorkspaceReadScope,
    ) -> Result<Self, TaskWorkspaceError> {
        Self::rebuild_internal(root, Some((node_id, proposed_source)), Some(scope))
    }

    #[allow(clippy::too_many_lines)]
    fn rebuild_internal(
        root: &Path,
        replacement: Option<(NodeId, &str)>,
        scope: Option<&crate::WorkspaceReadScope>,
    ) -> Result<Self, TaskWorkspaceError> {
        let inventory = scan_workspace(root);
        if let Some(scope) = scope {
            scope
                .validate_inventory(&inventory)
                .map_err(|_| TaskWorkspaceError::InvalidScope)?;
        } else if !inventory.is_valid() {
            return Err(TaskWorkspaceError::InvalidWorkspace(
                inventory
                    .issues
                    .first()
                    .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
            ));
        }
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
            return Err(TaskWorkspaceError::UnsupportedGeneration(
                inventory.generation,
            ));
        }
        if replacement.is_some_and(|(node_id, _)| scope.is_some_and(|scope| !scope.allows(node_id)))
        {
            return Err(TaskWorkspaceError::InvalidScope);
        }

        let mut index = Self {
            generation: inventory.generation,
            occurrences: Vec::new(),
            declarations: BTreeMap::new(),
            diagnostics: Vec::new(),
        };
        for node in &inventory.nodes {
            if crate::workspace_trash::is_trash_storage_path(root, &node.path) {
                continue;
            }
            let Some(node_id) = node.id else {
                if scope.is_some() {
                    continue;
                }
                return Err(TaskWorkspaceError::InvalidWorkspace(
                    InventoryIssueCode::MissingIdentity,
                ));
            };
            if scope.is_some_and(|scope| !scope.allows(node_id)) {
                continue;
            }
            let snapshot = crate::read_node_document(&node.path).map_err(|error| {
                TaskWorkspaceError::DocumentRead {
                    node_id,
                    message: error.to_string(),
                }
            })?;
            let source = replacement
                .filter(|(replacement_id, _)| *replacement_id == node_id)
                .map_or(snapshot.source.as_str(), |(_, source)| source);
            let revision = replacement
                .filter(|(replacement_id, _)| *replacement_id == node_id)
                .map_or_else(
                    || snapshot.revision.clone(),
                    |(_, source)| DocumentRevision::from_source(source),
                );
            let analysis = analyze_task_source(source);
            for diagnostic in &analysis.diagnostics {
                if diagnostic.code == TaskDiagnosticCode::DuplicateId {
                    continue;
                }
                let task_id = analysis
                    .tasks
                    .iter()
                    .find(|task| ranges_overlap(&task.range, &diagnostic.range))
                    .and_then(|task| task.metadata.as_ref())
                    .map(|metadata| metadata.id);
                index.diagnostics.push(TaskWorkspaceDiagnostic {
                    code: TaskWorkspaceDiagnosticCode::InvalidTaskSyntax,
                    message: diagnostic.message.clone(),
                    node_id,
                    task_id,
                    dependency_id: None,
                    range: diagnostic.range.clone(),
                    related_node_ids: Vec::new(),
                    related_task_ids: Vec::new(),
                    task_code: Some(diagnostic.code),
                });
            }
            for task in analysis.tasks {
                let occurrence_index = index.occurrences.len();
                if let Some(metadata) = &task.metadata {
                    index
                        .declarations
                        .entry(metadata.id)
                        .or_default()
                        .push(occurrence_index);
                }
                index.occurrences.push(TaskWorkspaceOccurrence {
                    node_id,
                    revision: revision.clone(),
                    task,
                });
            }
        }

        index.diagnose_duplicate_ids();
        let graph = index.resolve_dependency_graph();
        index.diagnose_dependency_cycles(&graph);
        index.sort_diagnostics();
        Ok(index)
    }

    #[must_use]
    pub const fn generation(&self) -> WorkspaceDocumentGeneration {
        self.generation
    }

    #[must_use]
    pub fn occurrences(&self) -> &[TaskWorkspaceOccurrence] {
        &self.occurrences
    }

    pub fn occurrences_for_node(
        &self,
        node_id: NodeId,
    ) -> impl Iterator<Item = &TaskWorkspaceOccurrence> {
        self.occurrences
            .iter()
            .filter(move |occurrence| occurrence.node_id == node_id)
    }

    #[must_use]
    pub fn declarations_for_id(&self, task_id: TaskId) -> Vec<&TaskWorkspaceOccurrence> {
        self.declarations
            .get(&task_id)
            .map_or_else(Vec::new, |indexes| {
                indexes
                    .iter()
                    .map(|index| &self.occurrences[*index])
                    .collect()
            })
    }

    /// Resolves an identity only when exactly one syntax-valid occurrence declares it.
    #[must_use]
    pub fn unique_task(&self, task_id: TaskId) -> Option<&TaskWorkspaceOccurrence> {
        let indexes = self.declarations.get(&task_id)?;
        (indexes.len() == 1)
            .then(|| &self.occurrences[indexes[0]])
            .filter(|occurrence| occurrence.task.valid)
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[TaskWorkspaceDiagnostic] {
        &self.diagnostics
    }

    fn diagnose_duplicate_ids(&mut self) {
        for (task_id, indexes) in self
            .declarations
            .iter()
            .filter(|(_, indexes)| indexes.len() > 1)
        {
            let related_node_ids = indexes
                .iter()
                .map(|index| self.occurrences[*index].node_id)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            for index in indexes {
                let occurrence = &self.occurrences[*index];
                self.diagnostics.push(TaskWorkspaceDiagnostic {
                    code: TaskWorkspaceDiagnosticCode::DuplicateTaskId,
                    message: format!(
                        "task ID `{task_id}` is declared by multiple workspace occurrences"
                    ),
                    node_id: occurrence.node_id,
                    task_id: Some(*task_id),
                    dependency_id: None,
                    range: occurrence.task.metadata.as_ref().map_or_else(
                        || occurrence.task.range.clone(),
                        |metadata| metadata.range.clone(),
                    ),
                    related_node_ids: related_node_ids.clone(),
                    related_task_ids: vec![*task_id],
                    task_code: None,
                });
            }
        }
    }

    fn resolve_dependency_graph(&mut self) -> BTreeMap<TaskId, Vec<TaskId>> {
        let mut graph = BTreeMap::new();
        for occurrence in &self.occurrences {
            let Some(metadata) = &occurrence.task.metadata else {
                continue;
            };
            if !occurrence.task.valid
                || self
                    .declarations
                    .get(&metadata.id)
                    .is_none_or(|indexes| indexes.len() != 1)
            {
                continue;
            }
            graph.entry(metadata.id).or_insert_with(Vec::new);
            let dependency_range = metadata
                .attributes
                .iter()
                .find(|attribute| attribute.name == "depends-on")
                .map_or_else(
                    || metadata.range.clone(),
                    |attribute| attribute.value_range.clone(),
                );
            for dependency_id in &metadata.dependencies {
                let Some(targets) = self.declarations.get(dependency_id) else {
                    self.diagnostics.push(dependency_diagnostic(
                        TaskWorkspaceDiagnosticCode::UnresolvedDependency,
                        format!("dependency task `{dependency_id}` does not exist"),
                        occurrence,
                        metadata.id,
                        *dependency_id,
                        dependency_range.clone(),
                        Vec::new(),
                    ));
                    continue;
                };
                if targets.len() > 1 {
                    let related_node_ids = targets
                        .iter()
                        .map(|index| self.occurrences[*index].node_id)
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect();
                    self.diagnostics.push(dependency_diagnostic(
                        TaskWorkspaceDiagnosticCode::AmbiguousDependency,
                        format!("dependency task `{dependency_id}` has multiple declarations"),
                        occurrence,
                        metadata.id,
                        *dependency_id,
                        dependency_range.clone(),
                        related_node_ids,
                    ));
                    continue;
                }
                let target = &self.occurrences[targets[0]];
                if !target.task.valid {
                    self.diagnostics.push(dependency_diagnostic(
                        TaskWorkspaceDiagnosticCode::InvalidDependencyTarget,
                        format!("dependency task `{dependency_id}` has invalid source"),
                        occurrence,
                        metadata.id,
                        *dependency_id,
                        dependency_range.clone(),
                        vec![target.node_id],
                    ));
                    continue;
                }
                graph.entry(metadata.id).or_default().push(*dependency_id);
            }
        }
        graph
    }

    fn diagnose_dependency_cycles(&mut self, graph: &BTreeMap<TaskId, Vec<TaskId>>) {
        for mut component in strongly_connected_components(graph) {
            if component.len() < 2 {
                continue;
            }
            component.sort_unstable();
            let related_node_ids = component
                .iter()
                .filter_map(|task_id| self.unique_task(*task_id))
                .map(|occurrence| occurrence.node_id)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            for task_id in &component {
                let Some(occurrence) = self.unique_task(*task_id) else {
                    continue;
                };
                let range = occurrence.task.metadata.as_ref().map_or_else(
                    || occurrence.task.range.clone(),
                    |metadata| metadata.range.clone(),
                );
                self.diagnostics.push(TaskWorkspaceDiagnostic {
                    code: TaskWorkspaceDiagnosticCode::DependencyCycle,
                    message: "task dependency graph contains a cycle".to_owned(),
                    node_id: occurrence.node_id,
                    task_id: Some(*task_id),
                    dependency_id: None,
                    range,
                    related_node_ids: related_node_ids.clone(),
                    related_task_ids: component.clone(),
                    task_code: None,
                });
            }
        }
    }

    fn sort_diagnostics(&mut self) {
        self.diagnostics.sort_by(|left, right| {
            left.node_id
                .cmp(&right.node_id)
                .then_with(|| left.range.start.cmp(&right.range.start))
                .then_with(|| left.range.end.cmp(&right.range.end))
                .then_with(|| left.code.cmp(&right.code))
                .then_with(|| left.task_id.cmp(&right.task_id))
                .then_with(|| left.dependency_id.cmp(&right.dependency_id))
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskWorkspaceError {
    InvalidWorkspace(InventoryIssueCode),
    UnsupportedGeneration(WorkspaceDocumentGeneration),
    DocumentRead { node_id: NodeId, message: String },
    InvalidScope,
}

impl fmt::Display for TaskWorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::UnsupportedGeneration(generation) => {
                write!(
                    formatter,
                    "task runtime requires AsciiDoc v1, got {generation:?}"
                )
            }
            Self::DocumentRead { node_id, message } => {
                write!(formatter, "could not read node {node_id}: {message}")
            }
            Self::InvalidScope => formatter.write_str("task read scope is invalid"),
        }
    }
}

impl std::error::Error for TaskWorkspaceError {}

fn dependency_diagnostic(
    code: TaskWorkspaceDiagnosticCode,
    message: String,
    occurrence: &TaskWorkspaceOccurrence,
    task_id: TaskId,
    dependency_id: TaskId,
    range: Range<u64>,
    related_node_ids: Vec<NodeId>,
) -> TaskWorkspaceDiagnostic {
    TaskWorkspaceDiagnostic {
        code,
        message,
        node_id: occurrence.node_id,
        task_id: Some(task_id),
        dependency_id: Some(dependency_id),
        range,
        related_node_ids,
        related_task_ids: vec![dependency_id],
        task_code: None,
    }
}

fn strongly_connected_components(graph: &BTreeMap<TaskId, Vec<TaskId>>) -> Vec<Vec<TaskId>> {
    let mut visited = BTreeSet::new();
    let mut finish_order = Vec::new();
    for start in graph.keys().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next_index)) = stack.last_mut() {
            let neighbors = graph.get(node).map_or(&[][..], Vec::as_slice);
            if *next_index < neighbors.len() {
                let next = neighbors[*next_index];
                *next_index += 1;
                if visited.insert(next) {
                    stack.push((next, 0));
                }
            } else {
                finish_order.push(*node);
                stack.pop();
            }
        }
    }

    let mut reverse = graph
        .keys()
        .copied()
        .map(|task_id| (task_id, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (task_id, dependencies) in graph {
        for dependency in dependencies {
            reverse.entry(*dependency).or_default().push(*task_id);
        }
    }
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for start in finish_order.into_iter().rev() {
        if !assigned.insert(start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for next in reverse.get(&node).map_or(&[][..], Vec::as_slice) {
                if assigned.insert(*next) {
                    stack.push(*next);
                }
            }
        }
        components.push(component);
    }
    components
}

fn ranges_overlap(left: &Range<u64>, right: &Range<u64>) -> bool {
    left.start < right.end && right.start < left.end
}
