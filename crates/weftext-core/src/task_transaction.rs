use std::fmt;
use std::path::Path;

use crate::task_authoring::plan_task_dependency_edit;
use crate::task_workspace::TaskWorkspaceIndex;
use crate::workspace_transaction::plan_task_document_transaction;
use crate::{
    CommittedWorkspaceTransaction, DocumentError, DocumentRevision, NodeId, StructuralAction,
    TaskAuthoringFailure, TaskAuthoringPlan, TaskEditIntent, TaskEditTarget, TaskId,
    TaskRecurrenceCompletionContext, TaskRecurrenceCompletionFailure, TaskRecurrenceCompletionPlan,
    TaskWorkspaceDiagnostic, TaskWorkspaceError, WorkspaceReadScope, WorkspaceRevision,
    WorkspaceTransactionError, WorkspaceTransactionPlan, commit_workspace_transaction,
    plan_task_edit, plan_task_recurrence_completion, read_node_document, read_workspace_revision,
    scan_workspace,
};

#[derive(Clone, Debug)]
pub struct TaskEditTransactionPlan {
    pub node_id: NodeId,
    pub authoring: TaskAuthoringPlan,
    transaction: WorkspaceTransactionPlan,
}

impl TaskEditTransactionPlan {
    #[must_use]
    pub const fn workspace_transaction(&self) -> &WorkspaceTransactionPlan {
        &self.transaction
    }
}

#[derive(Clone, Debug)]
pub struct TaskRecurrenceTransactionPlan {
    pub node_id: NodeId,
    pub completion: TaskRecurrenceCompletionPlan,
    transaction: WorkspaceTransactionPlan,
}

impl TaskRecurrenceTransactionPlan {
    #[must_use]
    pub const fn workspace_transaction(&self) -> &WorkspaceTransactionPlan {
        &self.transaction
    }
}

#[derive(Clone, Debug)]
pub struct TaskDependencyTransactionPlan {
    pub node_id: NodeId,
    pub dependencies: Vec<TaskId>,
    pub authoring: TaskAuthoringPlan,
    transaction: WorkspaceTransactionPlan,
}

impl TaskDependencyTransactionPlan {
    #[must_use]
    pub const fn workspace_transaction(&self) -> &WorkspaceTransactionPlan {
        &self.transaction
    }
}

#[derive(Debug)]
pub enum TaskTransactionError {
    Workspace(WorkspaceTransactionError),
    Index(TaskWorkspaceError),
    Authoring(TaskAuthoringFailure),
    Recurrence(TaskRecurrenceCompletionFailure),
    TargetUnavailable,
    TargetWorkspaceInvalid {
        task_id: TaskId,
        diagnostics: Vec<TaskWorkspaceDiagnostic>,
    },
    ProposedWorkspaceInvalid {
        task_id: TaskId,
        diagnostics: Vec<TaskWorkspaceDiagnostic>,
    },
}

impl fmt::Display for TaskTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Workspace(error) => error.fmt(formatter),
            Self::Index(error) => error.fmt(formatter),
            Self::Authoring(error) => error.fmt(formatter),
            Self::Recurrence(error) => error.fmt(formatter),
            Self::TargetUnavailable => {
                formatter.write_str("task target is not uniquely available in the selected node")
            }
            Self::TargetWorkspaceInvalid {
                task_id,
                diagnostics,
            } => write!(
                formatter,
                "task `{task_id}` has {} workspace diagnostic(s)",
                diagnostics.len()
            ),
            Self::ProposedWorkspaceInvalid {
                task_id,
                diagnostics,
            } => write!(
                formatter,
                "proposed task `{task_id}` has {} workspace diagnostic(s)",
                diagnostics.len()
            ),
        }
    }
}

impl std::error::Error for TaskTransactionError {}

/// Plans an exact task edit as one stale-checked, recoverable workspace transaction.
///
/// The workspace index must uniquely resolve any structured target before and after the edit. A
/// target with invalid dependencies cannot be mutated through this ordinary action; callers use
/// the dependency transaction to repair it. Planning performs no write.
///
/// # Errors
///
/// Returns an error for stale source/workspace state, unavailable or workspace-invalid targets,
/// invalid authoring intent, a proposed workspace-invalid task, or pending recovery.
pub fn plan_task_edit_transaction(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    intent: &TaskEditIntent,
) -> Result<TaskEditTransactionPlan, TaskTransactionError> {
    plan_task_edit_transaction_internal(root.as_ref(), node_id, base_revision, target, intent, None)
}

/// Plans an exact task edit using only an already-authorized workspace projection.
///
/// Scope membership is checked before any managed document is opened. Task identity and
/// dependency validation is limited to visible nodes, so a hidden task is indistinguishable from
/// an unavailable task.
///
/// # Errors
///
/// Returns the same fail-closed errors as [`plan_task_edit_transaction`], plus an invalid-scope
/// error when the projection does not match the current workspace inventory.
pub fn plan_task_edit_transaction_scoped(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    intent: &TaskEditIntent,
    scope: &WorkspaceReadScope,
) -> Result<TaskEditTransactionPlan, TaskTransactionError> {
    plan_task_edit_transaction_internal(
        root.as_ref(),
        node_id,
        base_revision,
        target,
        intent,
        Some(scope),
    )
}

fn plan_task_edit_transaction_internal(
    root: &Path,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    intent: &TaskEditIntent,
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskEditTransactionPlan, TaskTransactionError> {
    let planning = load_planning_context(root, node_id, base_revision, target, scope)?;
    if let Some(task_id) = planning.target_id {
        require_clean_target(&planning.index, task_id, false)?;
    }
    let authoring = plan_task_edit(&planning.source, target, intent)
        .map_err(TaskTransactionError::Authoring)?;
    let proposed = rebuild_with_replacement(root, node_id, &authoring.proposed_source, scope)?;
    if let Some(task_id) = authoring
        .target
        .metadata
        .as_ref()
        .map(|metadata| metadata.id)
    {
        require_unique_in_node(&proposed, node_id, task_id, false)?;
        require_clean_target(&proposed, task_id, true)?;
    }
    let transaction = plan_task_document_transaction(
        root,
        &planning.workspace_revision,
        node_id,
        base_revision,
        authoring.edit.clone(),
        StructuralAction::TaskEdit,
    )
    .map_err(TaskTransactionError::Workspace)?;
    Ok(TaskEditTransactionPlan {
        node_id,
        authoring,
        transaction,
    })
}

/// Commits a previously previewed task edit through the shared workspace journal.
///
/// # Errors
///
/// Returns the ordinary workspace transaction error for stale revisions, I/O/recovery failures,
/// or post-commit verification failure.
pub fn commit_task_edit_transaction(
    plan: &TaskEditTransactionPlan,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    commit_workspace_transaction(&plan.transaction)
}

/// Plans recurring completion as one stale-checked, recoverable workspace transaction.
///
/// Both the completed identity and any generated successor identity are validated against the
/// complete task/dependency index before a commit plan is returned. Planning performs no write.
///
/// # Errors
///
/// Returns an error for stale source/workspace state, unavailable or workspace-invalid targets,
/// invalid completion context/recurrence, an invalid successor graph, or pending recovery.
pub fn plan_task_recurrence_transaction(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    context: &TaskRecurrenceCompletionContext,
) -> Result<TaskRecurrenceTransactionPlan, TaskTransactionError> {
    plan_task_recurrence_transaction_internal(
        root.as_ref(),
        node_id,
        base_revision,
        target,
        context,
        None,
    )
}

/// Plans recurring completion against only an already-authorized workspace projection.
///
/// # Errors
///
/// Returns the same fail-closed errors as [`plan_task_recurrence_transaction`], plus an
/// invalid-scope error when the projection does not match the current workspace inventory.
pub fn plan_task_recurrence_transaction_scoped(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    context: &TaskRecurrenceCompletionContext,
    scope: &WorkspaceReadScope,
) -> Result<TaskRecurrenceTransactionPlan, TaskTransactionError> {
    plan_task_recurrence_transaction_internal(
        root.as_ref(),
        node_id,
        base_revision,
        target,
        context,
        Some(scope),
    )
}

fn plan_task_recurrence_transaction_internal(
    root: &Path,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    context: &TaskRecurrenceCompletionContext,
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskRecurrenceTransactionPlan, TaskTransactionError> {
    let planning = load_planning_context(root, node_id, base_revision, target, scope)?;
    let task_id = planning
        .target_id
        .ok_or(TaskTransactionError::TargetUnavailable)?;
    require_clean_target(&planning.index, task_id, false)?;
    let completion = plan_task_recurrence_completion(&planning.source, target, context)
        .map_err(TaskTransactionError::Recurrence)?;
    let proposed = rebuild_with_replacement(root, node_id, &completion.proposed_source, scope)?;
    require_unique_in_node(&proposed, node_id, task_id, true)?;
    require_clean_target(&proposed, task_id, true)?;
    if let Some(next_id) = completion.next_task_id {
        require_unique_in_node(&proposed, node_id, next_id, true)?;
        require_clean_target(&proposed, next_id, true)?;
    }
    let transaction = plan_task_document_transaction(
        root,
        &planning.workspace_revision,
        node_id,
        base_revision,
        completion.edit.clone(),
        StructuralAction::TaskRecurrenceCompletion,
    )
    .map_err(TaskTransactionError::Workspace)?;
    Ok(TaskRecurrenceTransactionPlan {
        node_id,
        completion,
        transaction,
    })
}

/// Commits a previously previewed recurring completion through the shared workspace journal.
///
/// # Errors
///
/// Returns the ordinary workspace transaction error for stale revisions, I/O/recovery failures,
/// or post-commit verification failure.
pub fn commit_task_recurrence_transaction(
    plan: &TaskRecurrenceTransactionPlan,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    commit_workspace_transaction(&plan.transaction)
}

/// Plans replacement of one task's complete dependency set against the full workspace graph.
///
/// An identity-free checklist is promoted only when the requested set is non-empty. This action
/// may repair an existing unresolved or cyclic dependency set, but the complete proposed target
/// graph must be valid before the recoverable transaction is returned. Planning performs no write.
///
/// # Errors
///
/// Returns an error for stale source/workspace state, duplicate/self/unavailable dependencies,
/// ambiguous targets, a proposed cycle, invalid syntax, no change, or pending recovery.
pub fn plan_task_dependency_transaction(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    dependencies: &[TaskId],
) -> Result<TaskDependencyTransactionPlan, TaskTransactionError> {
    plan_task_dependency_transaction_internal(
        root.as_ref(),
        node_id,
        base_revision,
        target,
        dependencies,
        None,
    )
}

/// Plans dependency replacement against only an already-authorized workspace projection.
///
/// Dependencies outside the projection resolve exactly like missing dependencies; their node,
/// task, path, and diagnostics never enter the returned plan.
///
/// # Errors
///
/// Returns the same fail-closed errors as [`plan_task_dependency_transaction`], plus an
/// invalid-scope error when the projection does not match the current workspace inventory.
pub fn plan_task_dependency_transaction_scoped(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    dependencies: &[TaskId],
    scope: &WorkspaceReadScope,
) -> Result<TaskDependencyTransactionPlan, TaskTransactionError> {
    plan_task_dependency_transaction_internal(
        root.as_ref(),
        node_id,
        base_revision,
        target,
        dependencies,
        Some(scope),
    )
}

fn plan_task_dependency_transaction_internal(
    root: &Path,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    dependencies: &[TaskId],
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskDependencyTransactionPlan, TaskTransactionError> {
    let planning = load_planning_context(root, node_id, base_revision, target, scope)?;
    let authoring = plan_task_dependency_edit(&planning.source, target, dependencies)
        .map_err(TaskTransactionError::Authoring)?;
    if authoring.base_revision == authoring.next_revision {
        return Err(TaskTransactionError::Workspace(
            WorkspaceTransactionError::NoChange,
        ));
    }
    let task_id = authoring
        .target
        .metadata
        .as_ref()
        .map(|metadata| metadata.id)
        .ok_or(TaskTransactionError::TargetUnavailable)?;
    let proposed = rebuild_with_replacement(root, node_id, &authoring.proposed_source, scope)?;
    require_unique_in_node(&proposed, node_id, task_id, true)?;
    require_clean_target(&proposed, task_id, true)?;
    let transaction = plan_task_document_transaction(
        root,
        &planning.workspace_revision,
        node_id,
        base_revision,
        authoring.edit.clone(),
        StructuralAction::TaskDependencies,
    )
    .map_err(TaskTransactionError::Workspace)?;
    Ok(TaskDependencyTransactionPlan {
        node_id,
        dependencies: dependencies.to_vec(),
        authoring,
        transaction,
    })
}

/// Commits a previously previewed dependency replacement through the shared workspace journal.
///
/// # Errors
///
/// Returns the ordinary workspace transaction error for stale revisions, I/O/recovery failures,
/// or post-commit verification failure.
pub fn commit_task_dependency_transaction(
    plan: &TaskDependencyTransactionPlan,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    commit_workspace_transaction(&plan.transaction)
}

struct TaskPlanningContext {
    workspace_revision: WorkspaceRevision,
    source: String,
    index: TaskWorkspaceIndex,
    target_id: Option<TaskId>,
}

fn load_planning_context(
    root: &Path,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    target: &TaskEditTarget,
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskPlanningContext, TaskTransactionError> {
    if scope.is_some_and(|scope| !scope.allows(node_id)) {
        return Err(TaskTransactionError::TargetUnavailable);
    }
    let index = match scope {
        Some(scope) => TaskWorkspaceIndex::rebuild_scoped(root, scope),
        None => TaskWorkspaceIndex::rebuild(root),
    }
    .map_err(TaskTransactionError::Index)?;
    let inventory = scan_workspace(root);
    let node = inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(node_id))
        .ok_or(TaskTransactionError::Workspace(
            WorkspaceTransactionError::UnknownNode(node_id),
        ))?;
    let snapshot = read_node_document(&node.path)
        .map_err(WorkspaceTransactionError::Document)
        .map_err(TaskTransactionError::Workspace)?;
    if &snapshot.revision != base_revision {
        return Err(TaskTransactionError::Workspace(
            WorkspaceTransactionError::Document(DocumentError::StaleRevision {
                expected: base_revision.clone(),
                actual: snapshot.revision,
            }),
        ));
    }
    let target_occurrence = match target {
        TaskEditTarget::Occurrence { range } => index
            .occurrences_for_node(node_id)
            .find(|occurrence| occurrence.task.range == *range),
        TaskEditTarget::Id { id } => index.unique_task(*id).filter(|occurrence| {
            occurrence.node_id == node_id && occurrence.revision == *base_revision
        }),
    }
    .ok_or(TaskTransactionError::TargetUnavailable)?;
    let target_id = target_occurrence
        .task
        .metadata
        .as_ref()
        .map(|metadata| metadata.id);
    if let Some(task_id) = target_id {
        require_unique_in_node(&index, node_id, task_id, false)?;
    }
    // Bind the ordinary recoverable transaction only after authorization and scoped task
    // resolution have succeeded. This complete digest remains private Core transaction authority;
    // callers must expose an actor-scoped revision instead.
    let workspace_revision = read_workspace_revision(root)
        .map_err(WorkspaceTransactionError::Revision)
        .map_err(TaskTransactionError::Workspace)?;
    Ok(TaskPlanningContext {
        workspace_revision,
        source: snapshot.source,
        index,
        target_id,
    })
}

fn rebuild_with_replacement(
    root: &Path,
    node_id: NodeId,
    proposed_source: &str,
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskWorkspaceIndex, TaskTransactionError> {
    match scope {
        Some(scope) => TaskWorkspaceIndex::rebuild_scoped_with_replacement(
            root,
            node_id,
            proposed_source,
            scope,
        ),
        None => TaskWorkspaceIndex::rebuild_with_replacement(root, node_id, proposed_source),
    }
    .map_err(TaskTransactionError::Index)
}

fn require_unique_in_node(
    index: &TaskWorkspaceIndex,
    node_id: NodeId,
    task_id: TaskId,
    proposed: bool,
) -> Result<(), TaskTransactionError> {
    if index
        .unique_task(task_id)
        .is_some_and(|occurrence| occurrence.node_id == node_id)
    {
        return Ok(());
    }
    let diagnostics = diagnostics_for_task(index, task_id);
    if proposed {
        Err(TaskTransactionError::ProposedWorkspaceInvalid {
            task_id,
            diagnostics,
        })
    } else if diagnostics.is_empty() {
        Err(TaskTransactionError::TargetUnavailable)
    } else {
        Err(TaskTransactionError::TargetWorkspaceInvalid {
            task_id,
            diagnostics,
        })
    }
}

fn require_clean_target(
    index: &TaskWorkspaceIndex,
    task_id: TaskId,
    proposed: bool,
) -> Result<(), TaskTransactionError> {
    let diagnostics = diagnostics_for_task(index, task_id);
    if diagnostics.is_empty() {
        return Ok(());
    }
    if proposed {
        Err(TaskTransactionError::ProposedWorkspaceInvalid {
            task_id,
            diagnostics,
        })
    } else {
        Err(TaskTransactionError::TargetWorkspaceInvalid {
            task_id,
            diagnostics,
        })
    }
}

fn diagnostics_for_task(
    index: &TaskWorkspaceIndex,
    task_id: TaskId,
) -> Vec<TaskWorkspaceDiagnostic> {
    index
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.task_id == Some(task_id))
        .cloned()
        .collect()
}
