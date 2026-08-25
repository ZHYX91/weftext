use std::fmt;
use std::path::{Path, PathBuf};

use crate::document::plan_document_edit_from_snapshot;
use crate::workspace_transaction::acquire_clean_workspace_mutation_guard;
use crate::{
    ChecklistToggleError, ChecklistToggleEvidence, ChecklistToggleSourcePlan,
    ChecklistToggleSummary, CommittedDocument, DocumentEdit, DocumentEditPlan, DocumentError,
    DocumentSnapshot, NodeId, TaskNodeEditError, TaskNodeEditRequest, TaskNodeEditSummary,
    TaskNodeSourceEditPlan, WorkspaceInventory, WorkspaceReadScope, WorkspaceTransactionError,
    commit_document_edit, plan_checklist_toggle_source, plan_task_node_source_edit,
    read_node_document, scan_workspace,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum AuthorizationBinding {
    Owner,
    Scoped(WorkspaceReadScope),
}

#[derive(Clone)]
pub struct ChecklistToggleTransactionPlan {
    source_plan: ChecklistToggleSourcePlan,
    document_plan: DocumentEditPlan,
    workspace_root: PathBuf,
    node_directory: PathBuf,
    authorization: AuthorizationBinding,
}

impl fmt::Debug for ChecklistToggleTransactionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ChecklistToggleTransactionPlan")
            .field("source_plan", &self.source_plan)
            .field("authorization", &authorization_kind(&self.authorization))
            .finish_non_exhaustive()
    }
}

impl ChecklistToggleTransactionPlan {
    #[must_use]
    pub const fn source_plan(&self) -> &ChecklistToggleSourcePlan {
        &self.source_plan
    }

    #[must_use]
    pub const fn summary(&self) -> &ChecklistToggleSummary {
        &self.source_plan.summary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedChecklistToggle {
    pub summary: ChecklistToggleSummary,
    pub document: CommittedDocument,
}

#[derive(Clone)]
pub struct TaskNodeEditTransactionPlan {
    request: TaskNodeEditRequest,
    source_plan: TaskNodeSourceEditPlan,
    document_plan: DocumentEditPlan,
    workspace_root: PathBuf,
    node_directory: PathBuf,
    authorization: AuthorizationBinding,
}

impl fmt::Debug for TaskNodeEditTransactionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskNodeEditTransactionPlan")
            .field("source_plan", &self.source_plan)
            .field("authorization", &authorization_kind(&self.authorization))
            .finish_non_exhaustive()
    }
}

impl TaskNodeEditTransactionPlan {
    #[must_use]
    pub const fn source_plan(&self) -> &TaskNodeSourceEditPlan {
        &self.source_plan
    }

    #[must_use]
    pub const fn summary(&self) -> &TaskNodeEditSummary {
        &self.source_plan.summary
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedTaskNodeEdit {
    pub summary: TaskNodeEditSummary,
    pub document: CommittedDocument,
}

#[derive(Debug)]
pub enum TaskActionTransactionError {
    InvalidWorkspace,
    InvalidScope,
    TargetUnavailable,
    AuthorizationChanged,
    RootTaskIneligible,
    DocumentRead(DocumentError),
    DocumentIdentity,
    ChecklistToggle(ChecklistToggleError),
    TaskNodeEdit(TaskNodeEditError),
    DocumentPlan(DocumentError),
    DocumentCommit(DocumentError),
    WorkspaceGuard(WorkspaceTransactionError),
    RecoveryRequired,
    ReviewedPlanMismatch,
    PostValidation,
}

impl fmt::Display for TaskActionTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => formatter.write_str("workspace inventory is invalid"),
            Self::InvalidScope => formatter.write_str("workspace read scope is invalid"),
            Self::TargetUnavailable => formatter.write_str("task action target is unavailable"),
            Self::AuthorizationChanged => {
                formatter.write_str("task action authorization changed after review")
            }
            Self::RootTaskIneligible => {
                formatter.write_str("workspace root is ineligible for a task-node profile")
            }
            Self::DocumentRead(error) => write!(formatter, "task document read failed: {error}"),
            Self::DocumentIdentity => {
                formatter.write_str("task document identity differs from inventory authority")
            }
            Self::ChecklistToggle(error) => error.fmt(formatter),
            Self::TaskNodeEdit(error) => error.fmt(formatter),
            Self::DocumentPlan(error) => write!(formatter, "task document plan failed: {error}"),
            Self::DocumentCommit(error) => {
                write!(formatter, "task document commit failed: {error}")
            }
            Self::WorkspaceGuard(error) => {
                write!(formatter, "workspace mutation guard failed: {error}")
            }
            Self::RecoveryRequired => {
                formatter.write_str("workspace transaction recovery is required")
            }
            Self::ReviewedPlanMismatch => {
                formatter.write_str("fresh task action differs from the reviewed plan")
            }
            Self::PostValidation => {
                formatter.write_str("committed task action failed post-validation")
            }
        }
    }
}

impl std::error::Error for TaskActionTransactionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DocumentRead(error) | Self::DocumentPlan(error) | Self::DocumentCommit(error) => {
                Some(error)
            }
            Self::ChecklistToggle(error) => Some(error),
            Self::TaskNodeEdit(error) => Some(error),
            Self::WorkspaceGuard(error) => Some(error),
            Self::InvalidWorkspace
            | Self::InvalidScope
            | Self::TargetUnavailable
            | Self::AuthorizationChanged
            | Self::RootTaskIneligible
            | Self::DocumentIdentity
            | Self::RecoveryRequired
            | Self::ReviewedPlanMismatch
            | Self::PostValidation => None,
        }
    }
}

/// Plans an owner-authorized checklist toggle without changing workspace bytes.
///
/// # Errors
///
/// Returns an error for invalid inventory, an unavailable or inconsistent target document,
/// forged parser evidence, or an invalid exact document edit.
pub fn plan_checklist_toggle_transaction(
    root: impl AsRef<Path>,
    evidence: &ChecklistToggleEvidence,
) -> Result<ChecklistToggleTransactionPlan, TaskActionTransactionError> {
    plan_checklist_toggle_transaction_internal(root.as_ref(), evidence, None)
}

/// Plans a checklist toggle against an already-authorized workspace projection.
///
/// # Errors
///
/// Returns the same errors as [`plan_checklist_toggle_transaction`], plus invalid-scope errors.
pub fn plan_checklist_toggle_transaction_scoped(
    root: impl AsRef<Path>,
    evidence: &ChecklistToggleEvidence,
    scope: &WorkspaceReadScope,
) -> Result<ChecklistToggleTransactionPlan, TaskActionTransactionError> {
    plan_checklist_toggle_transaction_internal(root.as_ref(), evidence, Some(scope))
}

fn plan_checklist_toggle_transaction_internal(
    root: &Path,
    evidence: &ChecklistToggleEvidence,
    scope: Option<&WorkspaceReadScope>,
) -> Result<ChecklistToggleTransactionPlan, TaskActionTransactionError> {
    let inventory = authorized_inventory(root, evidence.owner_node_id, scope)?;
    let node_directory = locate_target(&inventory, evidence.owner_node_id)?.to_path_buf();
    let snapshot = read_authorized_snapshot(&node_directory, evidence.owner_node_id)?;
    let source_plan = plan_checklist_toggle_source(&snapshot.source, evidence)
        .map_err(TaskActionTransactionError::ChecklistToggle)?;
    let document_plan =
        plan_document_edit_from_snapshot(&snapshot, [document_edit(&source_plan.edit)?])
            .map_err(TaskActionTransactionError::DocumentPlan)?;
    require_document_plan_matches_checklist(&document_plan, &source_plan)?;
    Ok(ChecklistToggleTransactionPlan {
        source_plan,
        document_plan,
        workspace_root: inventory.root,
        node_directory,
        authorization: scope.map_or(AuthorizationBinding::Owner, |scope| {
            AuthorizationBinding::Scoped(scope.clone())
        }),
    })
}

/// Commits an owner-authorized checklist plan under the workspace-wide mutation lease.
///
/// # Errors
///
/// Returns an error when authorization form changed, recovery is pending, fresh authority differs
/// from the reviewed plan, or the atomic document commit fails.
pub fn commit_checklist_toggle_transaction(
    plan: &ChecklistToggleTransactionPlan,
) -> Result<CommittedChecklistToggle, TaskActionTransactionError> {
    require_owner_binding(&plan.authorization)?;
    commit_checklist_toggle_transaction_internal(plan, None)
}

/// Commits a scoped checklist plan only when the complete fresh scope equals the reviewed scope.
///
/// # Errors
///
/// Returns the same errors as [`commit_checklist_toggle_transaction`], including
/// [`TaskActionTransactionError::AuthorizationChanged`] for any scope change.
pub fn commit_checklist_toggle_transaction_scoped(
    plan: &ChecklistToggleTransactionPlan,
    fresh_scope: &WorkspaceReadScope,
) -> Result<CommittedChecklistToggle, TaskActionTransactionError> {
    require_scoped_binding(&plan.authorization, fresh_scope)?;
    commit_checklist_toggle_transaction_internal(plan, Some(fresh_scope))
}

fn commit_checklist_toggle_transaction_internal(
    plan: &ChecklistToggleTransactionPlan,
    scope: Option<&WorkspaceReadScope>,
) -> Result<CommittedChecklistToggle, TaskActionTransactionError> {
    let _guard = acquire_clean_workspace_mutation_guard(&plan.workspace_root)
        .map_err(map_workspace_guard_error)?;
    let owner_node_id = plan.source_plan.evidence.owner_node_id;
    let inventory = authorized_inventory(&plan.workspace_root, owner_node_id, scope)?;
    let current_directory = locate_target(&inventory, owner_node_id)?;
    if current_directory != plan.node_directory {
        return Err(TaskActionTransactionError::ReviewedPlanMismatch);
    }
    let snapshot = read_authorized_snapshot(current_directory, owner_node_id)?;
    let fresh_source_plan =
        plan_checklist_toggle_source(&snapshot.source, &plan.source_plan.evidence)
            .map_err(TaskActionTransactionError::ChecklistToggle)?;
    let fresh_document_plan =
        plan_document_edit_from_snapshot(&snapshot, [document_edit(&fresh_source_plan.edit)?])
            .map_err(TaskActionTransactionError::DocumentPlan)?;
    if fresh_source_plan != plan.source_plan || fresh_document_plan != plan.document_plan {
        return Err(TaskActionTransactionError::ReviewedPlanMismatch);
    }
    let document = commit_document_edit(&plan.document_plan)
        .map_err(TaskActionTransactionError::DocumentCommit)?;
    verify_committed_document(
        &document,
        owner_node_id,
        &plan.source_plan.summary.next_revision,
        &plan.document_plan,
    )?;
    Ok(CommittedChecklistToggle {
        summary: plan.source_plan.summary.clone(),
        document,
    })
}

/// Plans an owner-authorized task-node profile edit without changing workspace bytes.
///
/// # Errors
///
/// Returns an error for invalid inventory, a root target, invalid source profile/action evidence,
/// or an invalid exact document edit.
pub fn plan_task_node_edit_transaction(
    root: impl AsRef<Path>,
    request: &TaskNodeEditRequest,
) -> Result<TaskNodeEditTransactionPlan, TaskActionTransactionError> {
    plan_task_node_edit_transaction_internal(root.as_ref(), request, None)
}

/// Plans a task-node profile edit against an already-authorized workspace projection.
///
/// # Errors
///
/// Returns the same errors as [`plan_task_node_edit_transaction`], plus invalid-scope errors.
pub fn plan_task_node_edit_transaction_scoped(
    root: impl AsRef<Path>,
    request: &TaskNodeEditRequest,
    scope: &WorkspaceReadScope,
) -> Result<TaskNodeEditTransactionPlan, TaskActionTransactionError> {
    plan_task_node_edit_transaction_internal(root.as_ref(), request, Some(scope))
}

fn plan_task_node_edit_transaction_internal(
    root: &Path,
    request: &TaskNodeEditRequest,
    scope: Option<&WorkspaceReadScope>,
) -> Result<TaskNodeEditTransactionPlan, TaskActionTransactionError> {
    let node_id = request.evidence.node_id;
    let inventory = authorized_inventory(root, node_id, scope)?;
    let node_directory = locate_target(&inventory, node_id)?.to_path_buf();
    if node_directory == inventory.root {
        return Err(TaskActionTransactionError::RootTaskIneligible);
    }
    let snapshot = read_authorized_snapshot(&node_directory, node_id)?;
    let source_plan = plan_task_node_source_edit(&snapshot.source, request)
        .map_err(TaskActionTransactionError::TaskNodeEdit)?;
    let document_plan = plan_document_edit_from_snapshot(
        &snapshot,
        source_plan
            .edits
            .iter()
            .map(document_edit)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(TaskActionTransactionError::DocumentPlan)?;
    require_document_plan_matches_task_node(&document_plan, &source_plan)?;
    Ok(TaskNodeEditTransactionPlan {
        request: request.clone(),
        source_plan,
        document_plan,
        workspace_root: inventory.root,
        node_directory,
        authorization: scope.map_or(AuthorizationBinding::Owner, |scope| {
            AuthorizationBinding::Scoped(scope.clone())
        }),
    })
}

/// Commits an owner-authorized task-node plan under the workspace-wide mutation lease.
///
/// # Errors
///
/// Returns an error when authorization form changed, recovery is pending, fresh authority differs
/// from the reviewed plan, or the atomic document commit fails.
pub fn commit_task_node_edit_transaction(
    plan: &TaskNodeEditTransactionPlan,
) -> Result<CommittedTaskNodeEdit, TaskActionTransactionError> {
    require_owner_binding(&plan.authorization)?;
    commit_task_node_edit_transaction_internal(plan, None)
}

/// Commits a scoped task-node plan only when the complete fresh scope equals the reviewed scope.
///
/// # Errors
///
/// Returns the same errors as [`commit_task_node_edit_transaction`], including
/// [`TaskActionTransactionError::AuthorizationChanged`] for any scope change.
pub fn commit_task_node_edit_transaction_scoped(
    plan: &TaskNodeEditTransactionPlan,
    fresh_scope: &WorkspaceReadScope,
) -> Result<CommittedTaskNodeEdit, TaskActionTransactionError> {
    require_scoped_binding(&plan.authorization, fresh_scope)?;
    commit_task_node_edit_transaction_internal(plan, Some(fresh_scope))
}

fn commit_task_node_edit_transaction_internal(
    plan: &TaskNodeEditTransactionPlan,
    scope: Option<&WorkspaceReadScope>,
) -> Result<CommittedTaskNodeEdit, TaskActionTransactionError> {
    let _guard = acquire_clean_workspace_mutation_guard(&plan.workspace_root)
        .map_err(map_workspace_guard_error)?;
    let node_id = plan.request.evidence.node_id;
    let inventory = authorized_inventory(&plan.workspace_root, node_id, scope)?;
    let current_directory = locate_target(&inventory, node_id)?;
    if current_directory == inventory.root {
        return Err(TaskActionTransactionError::RootTaskIneligible);
    }
    if current_directory != plan.node_directory {
        return Err(TaskActionTransactionError::ReviewedPlanMismatch);
    }
    let snapshot = read_authorized_snapshot(current_directory, node_id)?;
    let fresh_source_plan = plan_task_node_source_edit(&snapshot.source, &plan.request)
        .map_err(TaskActionTransactionError::TaskNodeEdit)?;
    let fresh_document_plan = plan_document_edit_from_snapshot(
        &snapshot,
        fresh_source_plan
            .edits
            .iter()
            .map(document_edit)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(TaskActionTransactionError::DocumentPlan)?;
    if fresh_source_plan != plan.source_plan || fresh_document_plan != plan.document_plan {
        return Err(TaskActionTransactionError::ReviewedPlanMismatch);
    }
    let document = commit_document_edit(&plan.document_plan)
        .map_err(TaskActionTransactionError::DocumentCommit)?;
    verify_committed_document(
        &document,
        node_id,
        &plan.source_plan.summary.next_revision,
        &plan.document_plan,
    )?;
    Ok(CommittedTaskNodeEdit {
        summary: plan.source_plan.summary.clone(),
        document,
    })
}

fn authorized_inventory(
    root: &Path,
    target: NodeId,
    scope: Option<&WorkspaceReadScope>,
) -> Result<WorkspaceInventory, TaskActionTransactionError> {
    let inventory = scan_workspace(root);
    if let Some(scope) = scope {
        scope
            .validate_inventory(&inventory)
            .map_err(|_| TaskActionTransactionError::InvalidScope)?;
        if !scope.allows(target) {
            return Err(TaskActionTransactionError::TargetUnavailable);
        }
    } else if !inventory.is_valid() {
        return Err(TaskActionTransactionError::InvalidWorkspace);
    }
    Ok(inventory)
}

fn locate_target(
    inventory: &WorkspaceInventory,
    target: NodeId,
) -> Result<&Path, TaskActionTransactionError> {
    let matches = inventory
        .nodes
        .iter()
        .filter(|node| {
            node.id == Some(target)
                && node.metadata.is_some()
                && !crate::workspace_trash::is_trash_storage_path(&inventory.root, &node.path)
        })
        .collect::<Vec<_>>();
    let [node] = matches.as_slice() else {
        return Err(TaskActionTransactionError::TargetUnavailable);
    };
    Ok(node.path.as_path())
}

fn read_authorized_snapshot(
    node_directory: &Path,
    expected_node_id: NodeId,
) -> Result<DocumentSnapshot, TaskActionTransactionError> {
    let snapshot =
        read_node_document(node_directory).map_err(TaskActionTransactionError::DocumentRead)?;
    if snapshot.node_id != expected_node_id || snapshot.node_directory != node_directory {
        return Err(TaskActionTransactionError::DocumentIdentity);
    }
    Ok(snapshot)
}

fn document_edit(
    edit: &weftext_asciidoc::SourceEdit,
) -> Result<DocumentEdit, TaskActionTransactionError> {
    Ok(DocumentEdit {
        start: u64::try_from(edit.range.start)
            .map_err(|_| TaskActionTransactionError::ReviewedPlanMismatch)?,
        end: u64::try_from(edit.range.end)
            .map_err(|_| TaskActionTransactionError::ReviewedPlanMismatch)?,
        replacement: edit.replacement.clone(),
    })
}

fn require_document_plan_matches_checklist(
    document_plan: &DocumentEditPlan,
    source_plan: &ChecklistToggleSourcePlan,
) -> Result<(), TaskActionTransactionError> {
    if document_plan.base_revision == source_plan.summary.base_revision
        && document_plan.next_revision == source_plan.summary.next_revision
        && document_plan.next_source() == source_plan.proposed_source
    {
        Ok(())
    } else {
        Err(TaskActionTransactionError::ReviewedPlanMismatch)
    }
}

fn require_document_plan_matches_task_node(
    document_plan: &DocumentEditPlan,
    source_plan: &TaskNodeSourceEditPlan,
) -> Result<(), TaskActionTransactionError> {
    if document_plan.base_revision == source_plan.summary.base_revision
        && document_plan.next_revision == source_plan.summary.next_revision
        && document_plan.next_source() == source_plan.proposed_source
    {
        Ok(())
    } else {
        Err(TaskActionTransactionError::ReviewedPlanMismatch)
    }
}

fn require_owner_binding(binding: &AuthorizationBinding) -> Result<(), TaskActionTransactionError> {
    if matches!(binding, AuthorizationBinding::Owner) {
        Ok(())
    } else {
        Err(TaskActionTransactionError::AuthorizationChanged)
    }
}

const fn authorization_kind(binding: &AuthorizationBinding) -> &'static str {
    match binding {
        AuthorizationBinding::Owner => "owner",
        AuthorizationBinding::Scoped(_) => "scoped",
    }
}

fn require_scoped_binding(
    binding: &AuthorizationBinding,
    fresh_scope: &WorkspaceReadScope,
) -> Result<(), TaskActionTransactionError> {
    if matches!(binding, AuthorizationBinding::Scoped(reviewed) if reviewed == fresh_scope) {
        Ok(())
    } else {
        Err(TaskActionTransactionError::AuthorizationChanged)
    }
}

fn map_workspace_guard_error(error: WorkspaceTransactionError) -> TaskActionTransactionError {
    match error {
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskActionTransactionError::RecoveryRequired
        }
        error => TaskActionTransactionError::WorkspaceGuard(error),
    }
}

fn verify_committed_document(
    committed: &CommittedDocument,
    expected_node_id: NodeId,
    expected_revision: &crate::DocumentRevision,
    plan: &DocumentEditPlan,
) -> Result<(), TaskActionTransactionError> {
    let expected_length = u64::try_from(plan.next_source().len()).unwrap_or(u64::MAX);
    if committed.node_id == expected_node_id
        && committed.document_path == plan.document_path
        && &committed.revision == expected_revision
        && committed.length == expected_length
    {
        Ok(())
    } else {
        Err(TaskActionTransactionError::PostValidation)
    }
}
