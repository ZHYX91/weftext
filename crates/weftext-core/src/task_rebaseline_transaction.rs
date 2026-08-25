#![allow(
    dead_code,
    reason = "pre-release Core transaction primitive remains crate-private until native Owner authority is wired"
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::physical_inventory::{
    PhysicalInventoryBinding, PhysicalInventoryError, PhysicalInventoryProjectionChange,
    PhysicalInventoryRecord, PhysicalRootIdentityBinding, PhysicalTreeInventory,
    VerifiedExternalPhysicalTree, capture_stable_workspace_physical_inventory,
    physical_inventory_binding_from_records, project_physical_inventory_binding,
    project_physical_inventory_records, project_physical_inventory_records_from_records,
    verify_disjoint_external_physical_tree,
};
use crate::task_rebaseline::{
    LocalTaskRebaselineAuthority, plan_internal, validate_reviewed_shape,
};
use crate::workspace_transaction::{
    TaskRebaselineRollbackWorkspaceMaterial, TaskRebaselineWorkspaceMaterial,
    WorkspaceTransactionLease, acquire_clean_workspace_mutation_guard,
    acquire_workspace_transaction_lease, bind_task_rebaseline_commit_confirmation,
    bind_task_rebaseline_rollback_commit_confirmation,
    commit_workspace_transaction_with_clean_guard,
    plan_task_rebaseline_rollback_workspace_transaction,
    plan_task_rebaseline_workspace_transaction, task_rebaseline_committed_matches_plan,
    validate_workspace_transaction_draft_gate_for_commit,
};
use crate::{
    AnnotationReplicaCompleteness, CommittedWorkspaceTransaction, DocumentRevision, NodeId,
    RecoveryReport, TaskId, TaskRebaselineAnnotationInventory, TaskRebaselineError,
    TaskRebaselineIdentityMapping, TaskRebaselinePlan, WorkspaceDraftGatePreview,
    WorkspaceDraftGateToken, WorkspaceDraftRegistryView, WorkspaceRevision,
    WorkspaceTransactionError, WorkspaceTransactionPlan, read_node_document,
    read_workspace_revision, recover_workspace_transaction_for_plan, scan_workspace,
};

pub const TASK_REBASELINE_TRANSACTION_SCHEMA: &str = "weftext.task-rebaseline-transaction/v1";
pub const TASK_REBASELINE_EXTERNAL_SNAPSHOT_SCHEMA: &str =
    "weftext.task-rebaseline-external-snapshot/v1";
pub(crate) const TASK_REBASELINE_ROLLBACK_SCHEMA: &str = "weftext.task-rebaseline-rollback/v1";
const TASK_REBASELINE_COMMITTED_EXECUTION_EVIDENCE_SCHEMA: &str =
    "weftext.task-rebaseline-committed-execution-evidence/v1";

const AUTHORITY_DIGEST_DOMAIN: &[u8] = b"weftext.task-rebaseline-transaction.authority/v1\0";
const SNAPSHOT_RECEIPT_DIGEST_DOMAIN: &[u8] =
    b"weftext.task-rebaseline-external-snapshot.receipt/v1\0";
const ROLLBACK_AUTHORITY_DIGEST_DOMAIN: &[u8] = b"weftext.task-rebaseline-rollback.authority/v1\0";
const COMMITTED_EXECUTION_EVIDENCE_DIGEST_DOMAIN: &[u8] =
    b"weftext.task-rebaseline-committed-execution-evidence/v1\0";
const MAX_EXECUTABLE_NODES: usize = 10_000;
const MAX_EXECUTABLE_SOURCES: usize = 10_000;
const MAX_EXECUTABLE_PHYSICAL_ENTRIES: usize = 50_000;
const MAX_EXECUTABLE_AUTHORITY_BYTES: usize = 32 * 1024 * 1024;
const MAX_ROLLBACK_AUTHORITY_BYTES: usize = 48 * 1024 * 1024;

/// The only authorization mode accepted by the pre-release executable Core entry point.
///
/// This value records an assertion made by the native calling boundary; it does not discover an
/// operating-system ACL. Hosted, scoped, and non-Owner callers have no constructor or commit API.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineExecutionAuthorization {
    PreReleaseOwnerLocal,
}

/// Crate-private native-boundary assertion. Core does not fabricate an ACL actor or expose this
/// constructor as a product API while hosted Owner authority is still undefined.
pub(crate) struct TaskRebaselineOwnerConfirmation {
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
}

impl TaskRebaselineOwnerConfirmation {
    pub(crate) fn new_native_assertion(
        actor_binding: impl Into<String>,
        authorization_epoch: impl Into<String>,
    ) -> Result<Self, TaskRebaselineTransactionError> {
        let actor_binding = actor_binding.into();
        let authorization_epoch = authorization_epoch.into();
        validate_owner_binding(&actor_binding, &authorization_epoch)?;
        Ok(Self {
            confirmation_id: NodeId::new_v4(),
            actor_binding,
            authorization_epoch,
        })
    }
}

fn validate_owner_binding(
    actor_binding: &str,
    authorization_epoch: &str,
) -> Result<(), TaskRebaselineTransactionError> {
    if actor_binding.is_empty()
        || actor_binding.len() > 4_096
        || authorization_epoch.is_empty()
        || authorization_epoch.len() > 4_096
    {
        Err(TaskRebaselineTransactionError::InvalidReviewedAuthority)
    } else {
        Ok(())
    }
}

/// A distinct current-Owner assertion used only to review or commit exact rollback.
///
/// The reviewed rollback Owner may legitimately differ from the Owner that applied the original
/// rebaseline. Commit must nevertheless present a second assertion for the same reviewed actor
/// and authorization epoch, with a confirmation ID not used by any earlier phase.
pub(crate) struct TaskRebaselineRollbackOwnerConfirmation {
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
}

impl TaskRebaselineRollbackOwnerConfirmation {
    pub(crate) fn new_native_assertion(
        actor_binding: impl Into<String>,
        authorization_epoch: impl Into<String>,
    ) -> Result<Self, TaskRebaselineTransactionError> {
        let actor_binding = actor_binding.into();
        let authorization_epoch = authorization_epoch.into();
        validate_owner_binding(&actor_binding, &authorization_epoch)?;
        Ok(Self {
            confirmation_id: NodeId::new_v4(),
            actor_binding,
            authorization_epoch,
        })
    }
}

/// Complete native draft-registry assertion. It has no empty-authority compatibility path.
pub(crate) struct TaskRebaselineDraftRegistryAuthority {
    registry: WorkspaceDraftRegistryView,
}

impl TaskRebaselineDraftRegistryAuthority {
    pub(crate) fn new_complete_native(
        registry: WorkspaceDraftRegistryView,
    ) -> Result<Self, TaskRebaselineTransactionError> {
        if registry.observation.starts_with("core:") {
            return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
        }
        Ok(Self { registry })
    }
}

/// Path-free proof that a disjoint exact external tree matched the physical pre-state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineExternalSnapshotReceipt {
    pub schema: String,
    pub physical_inventory: PhysicalInventoryBinding,
    pub root_identity: PhysicalRootIdentityBinding,
    pub receipt_digest: String,
}

/// One exact managed source replacement authorized by the workspace-wide plan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineSourceReplacement {
    pub source_node_id: NodeId,
    pub document_locator: String,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub original_source: String,
    pub proposed_source: String,
    pub annotations: TaskRebaselineAnnotationInventory,
}

/// One complete fresh task-node tree created by the transaction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineNewNode {
    pub old_task_id: TaskId,
    pub generated_node_id: NodeId,
    pub destination_parent_node_id: NodeId,
    pub destination_node_locator: String,
    pub document_locator: String,
    pub exact_source: String,
    pub source_sha256: String,
}

/// Closed path-free executable authority. The v1 preview remains embedded without changing its
/// preview-only meaning; this separate schema adds the physical/snapshot/transaction evidence.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineTransactionSummary {
    pub schema: String,
    pub authorization: TaskRebaselineExecutionAuthorization,
    pub owner_confirmation_id: NodeId,
    pub owner_actor_binding: String,
    pub owner_authorization_epoch: String,
    pub base_workspace_revision: WorkspaceRevision,
    pub workspace_root_node_id: NodeId,
    pub workspace_root_document_revision: DocumentRevision,
    pub workspace_root_identity: PhysicalRootIdentityBinding,
    pub physical_pre_state: PhysicalInventoryBinding,
    pub physical_post_state: PhysicalInventoryBinding,
    pub physical_pre_entries: Vec<PhysicalInventoryRecord>,
    pub physical_post_entries: Vec<PhysicalInventoryRecord>,
    pub external_snapshot: TaskRebaselineExternalSnapshotReceipt,
    pub reviewed_preview: TaskRebaselinePlan,
    pub identity_map: Vec<TaskRebaselineIdentityMapping>,
    pub source_replacements: Vec<TaskRebaselineSourceReplacement>,
    pub new_nodes: Vec<TaskRebaselineNewNode>,
    pub draft_sensitive_node_ids: Vec<NodeId>,
    pub draft_observation: WorkspaceDraftRegistryView,
    pub authority_digest: String,
}

impl fmt::Debug for TaskRebaselineTransactionSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineTransactionSummary")
            .field("schema", &self.schema)
            .field("authorization", &self.authorization)
            .field("base_workspace_revision", &self.base_workspace_revision)
            .field("source_replacement_count", &self.source_replacements.len())
            .field("generated_node_count", &self.new_nodes.len())
            .field(
                "draft_sensitive_node_count",
                &self.draft_sensitive_node_ids.len(),
            )
            .field("authority_digest", &self.authority_digest)
            .finish_non_exhaustive()
    }
}

/// Process-local evidence that the exact sealed v3 plan reached committed C.
///
/// This is deliberately embedded in the private rollback authority. It is not a durable product
/// receipt, audit event, or caller-authored DTO.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct TaskRebaselineCommittedExecutionEvidence {
    pub(crate) schema: String,
    pub(crate) forward_plan_id: String,
    pub(crate) forward_authority_digest: String,
    pub(crate) forward_commit_confirmation_id: NodeId,
    pub(crate) forward_owner_actor_binding: String,
    pub(crate) forward_owner_authorization_epoch: String,
    pub(crate) committed_transaction: CommittedWorkspaceTransaction,
    pub(crate) evidence_digest: String,
}

impl fmt::Debug for TaskRebaselineCommittedExecutionEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineCommittedExecutionEvidence")
            .field("schema", &self.schema)
            .field("forward_plan_id", &self.forward_plan_id)
            .field("forward_authority_digest", &self.forward_authority_digest)
            .field("action", &self.committed_transaction.action)
            .field("base_revision", &self.committed_transaction.base_revision)
            .field("revision", &self.committed_transaction.revision)
            .field("evidence_digest", &self.evidence_digest)
            .finish_non_exhaustive()
    }
}

/// Closed C-to-A authority derived only from one opaque v3 plan and its exact committed result.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct TaskRebaselineRollbackSummary {
    pub(crate) schema: String,
    pub(crate) authorization: TaskRebaselineExecutionAuthorization,
    pub(crate) rollback_confirmation_id: NodeId,
    pub(crate) owner_actor_binding: String,
    pub(crate) owner_authorization_epoch: String,
    pub(crate) base_workspace_revision: WorkspaceRevision,
    pub(crate) post_workspace_revision: WorkspaceRevision,
    pub(crate) workspace_root_node_id: NodeId,
    pub(crate) workspace_root_pre_document_revision: DocumentRevision,
    pub(crate) workspace_root_post_document_revision: DocumentRevision,
    pub(crate) workspace_root_identity: PhysicalRootIdentityBinding,
    pub(crate) physical_pre_state: PhysicalInventoryBinding,
    pub(crate) physical_post_state: PhysicalInventoryBinding,
    pub(crate) physical_pre_entries: Vec<PhysicalInventoryRecord>,
    pub(crate) physical_post_entries: Vec<PhysicalInventoryRecord>,
    pub(crate) external_snapshot: TaskRebaselineExternalSnapshotReceipt,
    pub(crate) forward_authority: TaskRebaselineTransactionSummary,
    pub(crate) forward_committed_evidence: TaskRebaselineCommittedExecutionEvidence,
    pub(crate) draft_sensitive_node_ids: Vec<NodeId>,
    pub(crate) draft_observation: WorkspaceDraftRegistryView,
    pub(crate) authority_digest: String,
}

impl fmt::Debug for TaskRebaselineRollbackSummary {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineRollbackSummary")
            .field("schema", &self.schema)
            .field("authorization", &self.authorization)
            .field("base_workspace_revision", &self.base_workspace_revision)
            .field("post_workspace_revision", &self.post_workspace_revision)
            .field("workspace_root_node_id", &self.workspace_root_node_id)
            .field(
                "source_replacement_count",
                &self.forward_authority.source_replacements.len(),
            )
            .field(
                "generated_node_count",
                &self.forward_authority.new_nodes.len(),
            )
            .field(
                "draft_sensitive_node_count",
                &self.draft_sensitive_node_ids.len(),
            )
            .field("authority_digest", &self.authority_digest)
            .finish_non_exhaustive()
    }
}

/// Opaque executable Owner-local plan. Filesystem roots remain private and Debug-redacted.
pub struct TaskRebaselineTransactionPlan {
    summary: TaskRebaselineTransactionSummary,
    workspace_root: PathBuf,
    external_snapshot: VerifiedExternalPhysicalTree,
    transaction: WorkspaceTransactionPlan,
    draft_gate: WorkspaceDraftGatePreview,
}

/// Opaque internal exact-rollback plan. Filesystem roots and the external snapshot locator stay
/// private; `Debug` exposes only path-free reviewed authority.
pub(crate) struct TaskRebaselineRollbackPlan {
    summary: TaskRebaselineRollbackSummary,
    workspace_root: PathBuf,
    external_snapshot: VerifiedExternalPhysicalTree,
    transaction: WorkspaceTransactionPlan,
    draft_gate: WorkspaceDraftGatePreview,
}

impl fmt::Debug for TaskRebaselineRollbackPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineRollbackPlan")
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}

impl TaskRebaselineRollbackPlan {
    pub(crate) const fn summary(&self) -> &TaskRebaselineRollbackSummary {
        &self.summary
    }

    pub(crate) const fn draft_gate(&self) -> &WorkspaceDraftGatePreview {
        &self.draft_gate
    }
}

impl fmt::Debug for TaskRebaselineTransactionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineTransactionPlan")
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}

impl TaskRebaselineTransactionPlan {
    #[must_use]
    pub const fn summary(&self) -> &TaskRebaselineTransactionSummary {
        &self.summary
    }

    /// Runs the shared exact draft gate for every source document rewritten by rebaseline.
    pub const fn draft_gate(&self) -> &WorkspaceDraftGatePreview {
        &self.draft_gate
    }
}

/// Process-local successful commit result from the shared structural transaction engine.
///
/// This is not the deferred durable product receipt or audit record.
#[derive(Clone, Eq, PartialEq)]
pub struct CommittedTaskRebaseline {
    pub summary: TaskRebaselineTransactionSummary,
    pub transaction: CommittedWorkspaceTransaction,
    committed_evidence: TaskRebaselineCommittedExecutionEvidence,
}

impl fmt::Debug for CommittedTaskRebaseline {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommittedTaskRebaseline")
            .field("summary", &self.summary)
            .field("plan_id", &self.transaction.plan_id)
            .field("action", &self.transaction.action)
            .field("base_revision", &self.transaction.base_revision)
            .field("revision", &self.transaction.revision)
            .finish_non_exhaustive()
    }
}

/// Process-local successful C-to-A result. It is not a durable product rollback receipt or audit
/// record and remains unreachable from CLI, Server, Desktop, and `WebUI`.
#[derive(Clone, Eq, PartialEq)]
pub(crate) struct CommittedTaskRebaselineRollback {
    pub(crate) summary: TaskRebaselineRollbackSummary,
    pub(crate) transaction: CommittedWorkspaceTransaction,
}

impl fmt::Debug for CommittedTaskRebaselineRollback {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CommittedTaskRebaselineRollback")
            .field("summary", &self.summary)
            .field("plan_id", &self.transaction.plan_id)
            .field("action", &self.transaction.action)
            .field("base_revision", &self.transaction.base_revision)
            .field("revision", &self.transaction.revision)
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub enum TaskRebaselineTransactionError {
    Preview(TaskRebaselineError),
    PhysicalInventory(PhysicalInventoryError),
    WorkspaceTransaction(WorkspaceTransactionError),
    WorkspaceRevisionUnavailable,
    InvalidReviewedAuthority,
    ConversionBlocked,
    PhysicalPreStateChanged,
    ExternalSnapshotChanged,
    InvalidCommittedEvidence,
    RollbackPreStateChanged,
}

impl fmt::Display for TaskRebaselineTransactionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Preview(error) => error.fmt(formatter),
            Self::PhysicalInventory(error) => error.fmt(formatter),
            Self::WorkspaceTransaction(error) => error.fmt(formatter),
            Self::WorkspaceRevisionUnavailable => {
                formatter.write_str("task rebaseline workspace revision is unavailable")
            }
            Self::InvalidReviewedAuthority => formatter
                .write_str("task rebaseline executable authority is invalid or non-canonical"),
            Self::ConversionBlocked => {
                formatter.write_str("task rebaseline preview contains blocking conversion evidence")
            }
            Self::PhysicalPreStateChanged => {
                formatter.write_str("task rebaseline physical workspace pre-state changed")
            }
            Self::ExternalSnapshotChanged => {
                formatter.write_str("task rebaseline external exact snapshot changed")
            }
            Self::InvalidCommittedEvidence => formatter.write_str(
                "task rebaseline rollback lacks the exact committed v3 execution evidence",
            ),
            Self::RollbackPreStateChanged => formatter
                .write_str("task rebaseline rollback physical or semantic C pre-state changed"),
        }
    }
}

impl std::error::Error for TaskRebaselineTransactionError {}

impl From<TaskRebaselineError> for TaskRebaselineTransactionError {
    fn from(value: TaskRebaselineError) -> Self {
        Self::Preview(value)
    }
}

impl From<PhysicalInventoryError> for TaskRebaselineTransactionError {
    fn from(value: PhysicalInventoryError) -> Self {
        Self::PhysicalInventory(value)
    }
}

impl From<WorkspaceTransactionError> for TaskRebaselineTransactionError {
    fn from(value: WorkspaceTransactionError) -> Self {
        Self::WorkspaceTransaction(value)
    }
}

/// Builds the separate executable v1 authority for one reviewed preview and one exact external
/// snapshot. The lease spans physical A, semantic revalidation, physical B, external verification,
/// and a final physical observation. No workspace payload is changed.
pub(crate) fn plan_owner_local_task_rebaseline_transaction(
    workspace_root: impl AsRef<Path>,
    external_snapshot_root: impl AsRef<Path>,
    reviewed_preview: &TaskRebaselinePlan,
    owner_confirmation: &TaskRebaselineOwnerConfirmation,
    draft_authority: &TaskRebaselineDraftRegistryAuthority,
) -> Result<TaskRebaselineTransactionPlan, TaskRebaselineTransactionError> {
    let workspace_root = workspace_root.as_ref().to_path_buf();
    let lease = acquire_workspace_transaction_lease(&workspace_root)?;
    let physical_a = capture_stable_workspace_physical_inventory(&lease)?;
    let revision = read_workspace_revision(lease.physical_inventory_root())
        .map_err(|_| TaskRebaselineTransactionError::WorkspaceRevisionUnavailable)?;
    let authority = LocalTaskRebaselineAuthority {
        root: lease.physical_inventory_root().to_path_buf(),
        workspace_revision: revision,
        annotation_replica_completeness: AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    };
    let preview = plan_internal(&authority, Some(reviewed_preview))?;
    if preview != *reviewed_preview || !preview.conversion_ready() {
        return Err(if preview.conversion_ready() {
            TaskRebaselineTransactionError::InvalidReviewedAuthority
        } else {
            TaskRebaselineTransactionError::ConversionBlocked
        });
    }
    let external_snapshot = verify_disjoint_external_physical_tree(
        &lease,
        external_snapshot_root,
        physical_a.binding(),
    )?;
    let physical_b = capture_stable_workspace_physical_inventory(&lease)?;
    if physical_a != physical_b {
        return Err(TaskRebaselineTransactionError::PhysicalPreStateChanged);
    }
    lease.validate_anchor_identity()?;

    let root_document = read_node_document(lease.physical_inventory_root())
        .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    let inventory = scan_workspace(lease.physical_inventory_root());
    if !inventory.is_valid() {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let mut active_document_ids = inventory
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<Option<Vec<_>>>()
        .ok_or(TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    active_document_ids.sort_unstable();
    active_document_ids.dedup();
    let summary = build_summary(
        reviewed_preview,
        &physical_a,
        root_document.node_id,
        root_document.revision,
        external_snapshot.root_identity().clone(),
        active_document_ids,
        owner_confirmation,
        &draft_authority.registry,
    )?;
    let transaction = plan_task_rebaseline_workspace_transaction(
        TaskRebaselineWorkspaceMaterial {
            root: workspace_root.clone(),
            summary: summary.clone(),
            external_snapshot: external_snapshot.clone(),
        },
        &lease,
    )?;
    let draft_gate =
        crate::preview_workspace_transaction_draft_gate(&transaction, &draft_authority.registry)?;
    if draft_gate.executable_token.is_none() {
        return Err(TaskRebaselineTransactionError::WorkspaceTransaction(
            WorkspaceTransactionError::DraftGateBlocked(draft_gate.blocking_dirty_node_ids.clone()),
        ));
    }
    lease.validate_anchor_identity()?;
    Ok(TaskRebaselineTransactionPlan {
        summary,
        workspace_root,
        external_snapshot,
        transaction,
        draft_gate,
    })
}

/// Commits only after a fresh A/semantic/B capture, external snapshot revalidation, and the
/// shared draft-token recheck under one lease.
pub(crate) fn commit_owner_local_task_rebaseline_transaction(
    plan: &TaskRebaselineTransactionPlan,
    token: &WorkspaceDraftGateToken,
    fresh_owner_confirmation: &TaskRebaselineOwnerConfirmation,
    current_draft_authority: &TaskRebaselineDraftRegistryAuthority,
) -> Result<CommittedTaskRebaseline, TaskRebaselineTransactionError> {
    let lease = acquire_clean_workspace_mutation_guard(&plan.workspace_root)?;
    revalidate_under_lease(plan, &lease)?;
    let mut transaction_plan = plan.transaction.clone();
    bind_task_rebaseline_commit_confirmation(
        &mut transaction_plan,
        fresh_owner_confirmation.confirmation_id,
        fresh_owner_confirmation.actor_binding.clone(),
        fresh_owner_confirmation.authorization_epoch.clone(),
    )?;
    validate_workspace_transaction_draft_gate_for_commit(
        &transaction_plan,
        token,
        &current_draft_authority.registry,
    )?;
    let transaction = commit_workspace_transaction_with_clean_guard(&transaction_plan, &lease)?;
    let committed_evidence = committed_execution_evidence(
        &plan.summary,
        &transaction,
        fresh_owner_confirmation.confirmation_id,
        &fresh_owner_confirmation.actor_binding,
        &fresh_owner_confirmation.authorization_epoch,
    )?;
    Ok(CommittedTaskRebaseline {
        summary: plan.summary.clone(),
        transaction,
        committed_evidence,
    })
}

/// Resolves a crash only for the exact opaque rebaseline plan. Foreign or ambiguous evidence is
/// retained by the shared recovery engine.
pub(crate) fn recover_task_rebaseline_transaction_for_plan(
    plan: &TaskRebaselineTransactionPlan,
) -> Result<RecoveryReport, TaskRebaselineTransactionError> {
    recover_workspace_transaction_for_plan(&plan.transaction).map_err(Into::into)
}

/// Reviews an explicit exact C-to-A rollback for one opaque v3 plan and its exact process-local
/// committed result. This constructor remains crate-private while real ACL authority and durable
/// product receipts are deferred.
pub(crate) fn plan_owner_local_task_rebaseline_exact_rollback(
    forward_plan: &TaskRebaselineTransactionPlan,
    committed: &CommittedTaskRebaseline,
    rollback_confirmation: &TaskRebaselineRollbackOwnerConfirmation,
    draft_authority: &TaskRebaselineDraftRegistryAuthority,
) -> Result<TaskRebaselineRollbackPlan, TaskRebaselineTransactionError> {
    validate_committed_rollback_source(forward_plan, committed)?;
    let workspace_root = forward_plan.workspace_root.clone();
    let lease = acquire_clean_workspace_mutation_guard(&workspace_root)?;
    let active_document_ids = observe_exact_rollback_pre_state(
        &forward_plan.summary,
        &committed.committed_evidence,
        &forward_plan.external_snapshot,
        &lease,
    )?;
    if rollback_confirmation.confirmation_id == forward_plan.summary.owner_confirmation_id
        || rollback_confirmation.confirmation_id
            == committed.committed_evidence.forward_commit_confirmation_id
    {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let summary = build_rollback_summary(
        &forward_plan.summary,
        &committed.committed_evidence,
        active_document_ids,
        rollback_confirmation,
        &draft_authority.registry,
    )?;
    let transaction = plan_task_rebaseline_rollback_workspace_transaction(
        TaskRebaselineRollbackWorkspaceMaterial {
            root: workspace_root.clone(),
            summary: summary.clone(),
            external_snapshot: forward_plan.external_snapshot.clone(),
        },
        &lease,
    )?;
    let draft_gate =
        crate::preview_workspace_transaction_draft_gate(&transaction, &draft_authority.registry)?;
    if draft_gate.executable_token.is_none() {
        return Err(TaskRebaselineTransactionError::WorkspaceTransaction(
            WorkspaceTransactionError::DraftGateBlocked(draft_gate.blocking_dirty_node_ids.clone()),
        ));
    }
    lease.validate_anchor_identity()?;
    Ok(TaskRebaselineRollbackPlan {
        summary,
        workspace_root,
        external_snapshot: forward_plan.external_snapshot.clone(),
        transaction,
        draft_gate,
    })
}

/// Commits the reviewed C-to-A plan only after a second current-Owner assertion and a fresh
/// complete all-active-document draft observation.
pub(crate) fn commit_owner_local_task_rebaseline_exact_rollback(
    plan: &TaskRebaselineRollbackPlan,
    token: &WorkspaceDraftGateToken,
    fresh_owner_confirmation: &TaskRebaselineRollbackOwnerConfirmation,
    current_draft_authority: &TaskRebaselineDraftRegistryAuthority,
) -> Result<CommittedTaskRebaselineRollback, TaskRebaselineTransactionError> {
    let lease = acquire_clean_workspace_mutation_guard(&plan.workspace_root)?;
    validate_rollback_summary(&plan.summary)?;
    if current_draft_authority.registry.observation == plan.summary.draft_observation.observation {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let active_document_ids = observe_exact_rollback_pre_state(
        &plan.summary.forward_authority,
        &plan.summary.forward_committed_evidence,
        &plan.external_snapshot,
        &lease,
    )?;
    if active_document_ids != plan.summary.draft_sensitive_node_ids {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    let mut transaction_plan = plan.transaction.clone();
    bind_task_rebaseline_rollback_commit_confirmation(
        &mut transaction_plan,
        fresh_owner_confirmation.confirmation_id,
        fresh_owner_confirmation.actor_binding.clone(),
        fresh_owner_confirmation.authorization_epoch.clone(),
    )?;
    validate_workspace_transaction_draft_gate_for_commit(
        &transaction_plan,
        token,
        &current_draft_authority.registry,
    )?;
    let transaction = commit_workspace_transaction_with_clean_guard(&transaction_plan, &lease)?;
    Ok(CommittedTaskRebaselineRollback {
        summary: plan.summary.clone(),
        transaction,
    })
}

/// Resolves a crash only for this exact opaque v4 rollback plan. Recovery uses durable journal
/// evidence and intentionally does not re-run current ACL checks while safely closing a crash.
pub(crate) fn recover_task_rebaseline_exact_rollback_for_plan(
    plan: &TaskRebaselineRollbackPlan,
) -> Result<RecoveryReport, TaskRebaselineTransactionError> {
    recover_workspace_transaction_for_plan(&plan.transaction).map_err(Into::into)
}

fn revalidate_under_lease(
    plan: &TaskRebaselineTransactionPlan,
    lease: &WorkspaceTransactionLease,
) -> Result<(), TaskRebaselineTransactionError> {
    let physical_a = capture_stable_workspace_physical_inventory(lease)?;
    if physical_a.binding() != &plan.summary.physical_pre_state {
        return Err(TaskRebaselineTransactionError::PhysicalPreStateChanged);
    }
    let authority = LocalTaskRebaselineAuthority {
        root: lease.physical_inventory_root().to_path_buf(),
        workspace_revision: plan.summary.base_workspace_revision.clone(),
        annotation_replica_completeness: AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    };
    let preview = plan_internal(&authority, Some(&plan.summary.reviewed_preview))?;
    if preview != plan.summary.reviewed_preview {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let physical_b = capture_stable_workspace_physical_inventory(lease)?;
    if physical_a != physical_b {
        return Err(TaskRebaselineTransactionError::PhysicalPreStateChanged);
    }
    plan.external_snapshot
        .revalidate(lease)
        .map_err(|_| TaskRebaselineTransactionError::ExternalSnapshotChanged)?;
    let physical_c = capture_stable_workspace_physical_inventory(lease)?;
    if physical_a != physical_c {
        return Err(TaskRebaselineTransactionError::PhysicalPreStateChanged);
    }
    let root_document = read_node_document(lease.physical_inventory_root())
        .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    if root_document.node_id != plan.summary.workspace_root_node_id
        || root_document.revision != plan.summary.workspace_root_document_revision
        || physical_a.root_identity() != &plan.summary.workspace_root_identity
    {
        return Err(TaskRebaselineTransactionError::PhysicalPreStateChanged);
    }
    let inventory = scan_workspace(lease.physical_inventory_root());
    let mut active_document_ids = if inventory.is_valid() {
        inventory
            .nodes
            .iter()
            .map(|node| node.id)
            .collect::<Option<Vec<_>>>()
            .ok_or(TaskRebaselineTransactionError::InvalidReviewedAuthority)?
    } else {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    };
    active_document_ids.sort_unstable();
    active_document_ids.dedup();
    if active_document_ids != plan.summary.draft_sensitive_node_ids {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    validate_summary(&plan.summary)?;
    lease.validate_anchor_identity()?;
    Ok(())
}

fn validate_committed_rollback_source(
    forward_plan: &TaskRebaselineTransactionPlan,
    committed: &CommittedTaskRebaseline,
) -> Result<(), TaskRebaselineTransactionError> {
    if committed.summary != forward_plan.summary
        || committed.committed_evidence.committed_transaction != committed.transaction
        || !task_rebaseline_committed_matches_plan(
            &forward_plan.transaction,
            &committed.transaction,
        )
    {
        return Err(TaskRebaselineTransactionError::InvalidCommittedEvidence);
    }
    validate_committed_execution_evidence(&committed.committed_evidence, &forward_plan.summary)
}

fn observe_exact_rollback_pre_state(
    forward: &TaskRebaselineTransactionSummary,
    committed: &TaskRebaselineCommittedExecutionEvidence,
    external_snapshot: &VerifiedExternalPhysicalTree,
    lease: &WorkspaceTransactionLease,
) -> Result<Vec<NodeId>, TaskRebaselineTransactionError> {
    validate_summary(forward)?;
    validate_committed_execution_evidence(committed, forward)?;
    let physical_c = capture_stable_workspace_physical_inventory(lease)?;
    if physical_c.binding() != &forward.physical_post_state
        || physical_c.records() != forward.physical_post_entries
        || physical_c.root_identity() != &forward.workspace_root_identity
    {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    let revision = read_workspace_revision(lease.physical_inventory_root())
        .map_err(|_| TaskRebaselineTransactionError::WorkspaceRevisionUnavailable)?;
    if revision != committed.committed_transaction.revision {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    let root_document = read_node_document(lease.physical_inventory_root())
        .map_err(|_| TaskRebaselineTransactionError::RollbackPreStateChanged)?;
    if root_document.node_id != forward.workspace_root_node_id
        || root_document.revision != forward_post_root_document_revision(forward)
    {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    let inventory = scan_workspace(lease.physical_inventory_root());
    if !inventory.is_valid() {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    for replacement in &forward.source_replacements {
        let path = lease
            .physical_inventory_root()
            .join(Path::new(&replacement.document_locator));
        let source = fs_read(&path)?;
        if source != replacement.proposed_source.as_bytes() {
            return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
        }
        let node = inventory
            .nodes
            .iter()
            .find(|node| node.id == Some(replacement.source_node_id))
            .ok_or(TaskRebaselineTransactionError::RollbackPreStateChanged)?;
        if node.document_path != path {
            return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
        }
    }
    for node in &forward.new_nodes {
        let node_path = lease
            .physical_inventory_root()
            .join(Path::new(&node.destination_node_locator));
        let document_path = lease
            .physical_inventory_root()
            .join(Path::new(&node.document_locator));
        let record = inventory
            .nodes
            .iter()
            .find(|record| record.id == Some(node.generated_node_id))
            .ok_or(TaskRebaselineTransactionError::RollbackPreStateChanged)?;
        let source = fs_read(&document_path)?;
        let profile = crate::analyze_task_node_profile(
            std::str::from_utf8(&source)
                .map_err(|_| TaskRebaselineTransactionError::RollbackPreStateChanged)?,
            Some(node.generated_node_id),
        );
        if record.path != node_path
            || record.document_path != document_path
            || source != node.exact_source.as_bytes()
            || !profile.diagnostics.is_empty()
            || profile.profile.is_none()
        {
            return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
        }
    }
    let mut active_document_ids = inventory
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<Option<Vec<_>>>()
        .ok_or(TaskRebaselineTransactionError::RollbackPreStateChanged)?;
    active_document_ids.sort_unstable();
    active_document_ids.dedup();
    let expected = rollback_draft_sensitive_node_ids(forward);
    if active_document_ids != expected {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    external_snapshot
        .revalidate(lease)
        .map_err(|_| TaskRebaselineTransactionError::ExternalSnapshotChanged)?;
    let physical_after = capture_stable_workspace_physical_inventory(lease)?;
    if physical_after != physical_c {
        return Err(TaskRebaselineTransactionError::RollbackPreStateChanged);
    }
    lease.validate_anchor_identity()?;
    Ok(active_document_ids)
}

fn fs_read(path: &Path) -> Result<Vec<u8>, TaskRebaselineTransactionError> {
    std::fs::read(path).map_err(|_| TaskRebaselineTransactionError::RollbackPreStateChanged)
}

fn rollback_draft_sensitive_node_ids(forward: &TaskRebaselineTransactionSummary) -> Vec<NodeId> {
    let mut ids = forward.draft_sensitive_node_ids.clone();
    ids.extend(forward.new_nodes.iter().map(|node| node.generated_node_id));
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn forward_post_root_document_revision(
    forward: &TaskRebaselineTransactionSummary,
) -> DocumentRevision {
    forward
        .source_replacements
        .iter()
        .find(|replacement| replacement.source_node_id == forward.workspace_root_node_id)
        .map_or_else(
            || forward.workspace_root_document_revision.clone(),
            |replacement| replacement.next_revision.clone(),
        )
}

fn committed_execution_evidence(
    forward: &TaskRebaselineTransactionSummary,
    transaction: &CommittedWorkspaceTransaction,
    confirmation_id: NodeId,
    actor_binding: &str,
    authorization_epoch: &str,
) -> Result<TaskRebaselineCommittedExecutionEvidence, TaskRebaselineTransactionError> {
    let mut evidence = TaskRebaselineCommittedExecutionEvidence {
        schema: TASK_REBASELINE_COMMITTED_EXECUTION_EVIDENCE_SCHEMA.to_owned(),
        forward_plan_id: transaction.plan_id.clone(),
        forward_authority_digest: forward.authority_digest.clone(),
        forward_commit_confirmation_id: confirmation_id,
        forward_owner_actor_binding: actor_binding.to_owned(),
        forward_owner_authorization_epoch: authorization_epoch.to_owned(),
        committed_transaction: transaction.clone(),
        evidence_digest: String::new(),
    };
    evidence.evidence_digest = committed_execution_evidence_digest(&evidence)?;
    validate_committed_execution_evidence(&evidence, forward)?;
    Ok(evidence)
}

fn validate_committed_execution_evidence(
    evidence: &TaskRebaselineCommittedExecutionEvidence,
    forward: &TaskRebaselineTransactionSummary,
) -> Result<(), TaskRebaselineTransactionError> {
    validate_owner_binding(
        &evidence.forward_owner_actor_binding,
        &evidence.forward_owner_authorization_epoch,
    )?;
    let canonical_plan_id = evidence
        .forward_plan_id
        .parse::<NodeId>()
        .is_ok_and(|id| id.to_string() == evidence.forward_plan_id);
    let committed = &evidence.committed_transaction;
    let path_changes = committed
        .path_changes
        .iter()
        .map(|change| (change.node_id, change))
        .collect::<BTreeMap<_, _>>();
    let generated_paths_match = path_changes.len() == forward.new_nodes.len()
        && forward.new_nodes.iter().all(|node| {
            path_changes
                .get(&node.generated_node_id)
                .is_some_and(|change| {
                    change.source_node_id.is_none()
                        && change.old_path.is_none()
                        && change.new_path == node.destination_node_locator
                })
        });
    if evidence.schema != TASK_REBASELINE_COMMITTED_EXECUTION_EVIDENCE_SCHEMA
        || !canonical_plan_id
        || evidence.forward_authority_digest != forward.authority_digest
        || evidence.forward_commit_confirmation_id == forward.owner_confirmation_id
        || evidence.forward_owner_actor_binding != forward.owner_actor_binding
        || evidence.forward_owner_authorization_epoch != forward.owner_authorization_epoch
        || committed.plan_id != evidence.forward_plan_id
        || committed.action != crate::StructuralAction::TaskRebaseline
        || committed.base_revision != forward.base_workspace_revision
        || committed.scope_summary.is_some()
        || committed.promotion_summary.is_some()
        || !committed.identity_map.is_empty()
        || committed.captured_target.is_some()
        || committed.target_node_ids != forward.draft_sensitive_node_ids
        || committed.draft_sensitive_node_ids != forward.draft_sensitive_node_ids
        || committed.import_authority.is_some()
        || !generated_paths_match
        || WorkspaceRevision::parse(committed.revision.as_str()).is_err()
        || evidence.evidence_digest != committed_execution_evidence_digest(evidence)?
    {
        return Err(TaskRebaselineTransactionError::InvalidCommittedEvidence);
    }
    Ok(())
}

fn committed_execution_evidence_digest(
    evidence: &TaskRebaselineCommittedExecutionEvidence,
) -> Result<String, TaskRebaselineTransactionError> {
    let bytes = serde_json::to_vec(&(
        &evidence.schema,
        &evidence.forward_plan_id,
        &evidence.forward_authority_digest,
        evidence.forward_commit_confirmation_id,
        &evidence.forward_owner_actor_binding,
        &evidence.forward_owner_authorization_epoch,
        &evidence.committed_transaction,
    ))
    .map_err(|_| TaskRebaselineTransactionError::InvalidCommittedEvidence)?;
    let mut hasher = Sha256::new();
    hasher.update(COMMITTED_EXECUTION_EVIDENCE_DIGEST_DOMAIN);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn build_rollback_summary(
    forward: &TaskRebaselineTransactionSummary,
    committed: &TaskRebaselineCommittedExecutionEvidence,
    draft_sensitive_node_ids: Vec<NodeId>,
    owner_confirmation: &TaskRebaselineRollbackOwnerConfirmation,
    draft_observation: &WorkspaceDraftRegistryView,
) -> Result<TaskRebaselineRollbackSummary, TaskRebaselineTransactionError> {
    let mut summary = TaskRebaselineRollbackSummary {
        schema: TASK_REBASELINE_ROLLBACK_SCHEMA.to_owned(),
        authorization: TaskRebaselineExecutionAuthorization::PreReleaseOwnerLocal,
        rollback_confirmation_id: owner_confirmation.confirmation_id,
        owner_actor_binding: owner_confirmation.actor_binding.clone(),
        owner_authorization_epoch: owner_confirmation.authorization_epoch.clone(),
        base_workspace_revision: committed.committed_transaction.revision.clone(),
        post_workspace_revision: forward.base_workspace_revision.clone(),
        workspace_root_node_id: forward.workspace_root_node_id,
        workspace_root_pre_document_revision: forward_post_root_document_revision(forward),
        workspace_root_post_document_revision: forward.workspace_root_document_revision.clone(),
        workspace_root_identity: forward.workspace_root_identity.clone(),
        physical_pre_state: forward.physical_post_state.clone(),
        physical_post_state: forward.physical_pre_state.clone(),
        physical_pre_entries: forward.physical_post_entries.clone(),
        physical_post_entries: forward.physical_pre_entries.clone(),
        external_snapshot: forward.external_snapshot.clone(),
        forward_authority: forward.clone(),
        forward_committed_evidence: committed.clone(),
        draft_sensitive_node_ids,
        draft_observation: draft_observation.clone(),
        authority_digest: String::new(),
    };
    summary.authority_digest = rollback_authority_digest(&summary)?;
    validate_rollback_summary(&summary)?;
    Ok(summary)
}

pub(crate) fn validate_rollback_summary(
    summary: &TaskRebaselineRollbackSummary,
) -> Result<(), TaskRebaselineTransactionError> {
    validate_summary(&summary.forward_authority)?;
    validate_committed_execution_evidence(
        &summary.forward_committed_evidence,
        &summary.forward_authority,
    )?;
    validate_owner_binding(
        &summary.owner_actor_binding,
        &summary.owner_authorization_epoch,
    )?;
    let forward = &summary.forward_authority;
    let expected_drafts = rollback_draft_sensitive_node_ids(forward);
    let valid = summary.schema == TASK_REBASELINE_ROLLBACK_SCHEMA
        && summary.authorization == TaskRebaselineExecutionAuthorization::PreReleaseOwnerLocal
        && summary.rollback_confirmation_id != forward.owner_confirmation_id
        && summary.rollback_confirmation_id
            != summary
                .forward_committed_evidence
                .forward_commit_confirmation_id
        && summary.base_workspace_revision
            == summary
                .forward_committed_evidence
                .committed_transaction
                .revision
        && summary.post_workspace_revision == forward.base_workspace_revision
        && summary.workspace_root_node_id == forward.workspace_root_node_id
        && summary.workspace_root_pre_document_revision
            == forward_post_root_document_revision(forward)
        && summary.workspace_root_post_document_revision
            == forward.workspace_root_document_revision
        && summary.workspace_root_identity == forward.workspace_root_identity
        && summary.physical_pre_state == forward.physical_post_state
        && summary.physical_post_state == forward.physical_pre_state
        && summary.physical_pre_entries == forward.physical_post_entries
        && summary.physical_post_entries == forward.physical_pre_entries
        && summary.external_snapshot == forward.external_snapshot
        && summary.draft_sensitive_node_ids == expected_drafts
        && !summary.draft_observation.observation.starts_with("core:")
        && summary.authority_digest == rollback_authority_digest(summary)?
        && serde_json::to_vec(summary)
            .is_ok_and(|bytes| bytes.len() <= MAX_ROLLBACK_AUTHORITY_BYTES);
    if valid {
        Ok(())
    } else {
        Err(TaskRebaselineTransactionError::InvalidReviewedAuthority)
    }
}

fn rollback_authority_digest(
    summary: &TaskRebaselineRollbackSummary,
) -> Result<String, TaskRebaselineTransactionError> {
    let bytes = serde_json::to_vec(&(
        (
            &summary.schema,
            summary.authorization,
            summary.rollback_confirmation_id,
            &summary.owner_actor_binding,
            &summary.owner_authorization_epoch,
        ),
        (
            &summary.base_workspace_revision,
            &summary.post_workspace_revision,
            summary.workspace_root_node_id,
            &summary.workspace_root_pre_document_revision,
            &summary.workspace_root_post_document_revision,
            &summary.workspace_root_identity,
        ),
        (
            &summary.physical_pre_state,
            &summary.physical_post_state,
            &summary.physical_pre_entries,
            &summary.physical_post_entries,
            &summary.external_snapshot,
        ),
        (
            &summary.forward_authority,
            &summary.forward_committed_evidence,
            &summary.draft_sensitive_node_ids,
            &summary.draft_observation,
        ),
    ))
    .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    let mut hasher = Sha256::new();
    hasher.update(ROLLBACK_AUTHORITY_DIGEST_DOMAIN);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "all independent rebaseline authority roots are explicit at the sole summary-construction boundary"
)]
fn build_summary(
    preview: &TaskRebaselinePlan,
    physical_pre_state: &PhysicalTreeInventory,
    workspace_root_node_id: NodeId,
    workspace_root_document_revision: DocumentRevision,
    external_root_identity: PhysicalRootIdentityBinding,
    draft_sensitive_node_ids: Vec<NodeId>,
    owner_confirmation: &TaskRebaselineOwnerConfirmation,
    draft_observation: &WorkspaceDraftRegistryView,
) -> Result<TaskRebaselineTransactionSummary, TaskRebaselineTransactionError> {
    if !preview.conversion_ready() {
        return Err(TaskRebaselineTransactionError::ConversionBlocked);
    }
    let source_count = preview
        .source_previews
        .iter()
        .filter(|source| !source.proposals.is_empty())
        .count();
    if preview.identity_map.len() > MAX_EXECUTABLE_NODES || source_count > MAX_EXECUTABLE_SOURCES {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    validate_execution_entry_budget(
        physical_pre_state.entries().len(),
        preview.identity_map.len(),
    )?;
    let (source_replacements, new_nodes) = derive_execution_changes(preview)?;
    let projection = physical_projection(&source_replacements, &new_nodes);
    let physical_pre_entries = physical_pre_state.records();
    let physical_post_state = project_physical_inventory_binding(physical_pre_state, &projection)?;
    let physical_post_entries =
        project_physical_inventory_records(physical_pre_state, &projection)?;
    let external_snapshot =
        snapshot_receipt(physical_pre_state.binding().clone(), external_root_identity)?;
    let mut summary = TaskRebaselineTransactionSummary {
        schema: TASK_REBASELINE_TRANSACTION_SCHEMA.to_owned(),
        authorization: TaskRebaselineExecutionAuthorization::PreReleaseOwnerLocal,
        owner_confirmation_id: owner_confirmation.confirmation_id,
        owner_actor_binding: owner_confirmation.actor_binding.clone(),
        owner_authorization_epoch: owner_confirmation.authorization_epoch.clone(),
        base_workspace_revision: preview.base_workspace_revision.clone(),
        workspace_root_node_id,
        workspace_root_document_revision,
        workspace_root_identity: physical_pre_state.root_identity().clone(),
        physical_pre_state: physical_pre_state.binding().clone(),
        physical_post_state,
        physical_pre_entries,
        physical_post_entries,
        external_snapshot,
        reviewed_preview: preview.clone(),
        identity_map: preview.identity_map.clone(),
        source_replacements,
        new_nodes,
        draft_sensitive_node_ids,
        draft_observation: draft_observation.clone(),
        authority_digest: String::new(),
    };
    summary.authority_digest = authority_digest(&summary)?;
    validate_summary(&summary)?;
    Ok(summary)
}

fn validate_execution_entry_budget(
    base_entry_count: usize,
    generated_node_count: usize,
) -> Result<usize, TaskRebaselineTransactionError> {
    let projected_entry_count = generated_node_count
        .checked_mul(2)
        .and_then(|created| base_entry_count.checked_add(created))
        .ok_or(TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    if base_entry_count > MAX_EXECUTABLE_PHYSICAL_ENTRIES
        || projected_entry_count > MAX_EXECUTABLE_PHYSICAL_ENTRIES
    {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    Ok(projected_entry_count)
}

fn derive_execution_changes(
    preview: &TaskRebaselinePlan,
) -> Result<
    (
        Vec<TaskRebaselineSourceReplacement>,
        Vec<TaskRebaselineNewNode>,
    ),
    TaskRebaselineTransactionError,
> {
    let mappings = preview
        .identity_map
        .iter()
        .map(|mapping| (mapping.generated_node_id, mapping))
        .collect::<BTreeMap<_, _>>();
    if mappings.len() != preview.identity_map.len() {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let mut source_replacements = Vec::new();
    let mut new_nodes = Vec::new();
    for source in &preview.source_previews {
        if source.proposals.is_empty() {
            continue;
        }
        source_replacements.push(TaskRebaselineSourceReplacement {
            source_node_id: source.source_node_id,
            document_locator: source.document_locator.clone(),
            base_revision: source.document_revision.clone(),
            next_revision: DocumentRevision::from_source(&source.proposed_source),
            original_source: source.original_source.clone(),
            proposed_source: source.proposed_source.clone(),
            annotations: source.annotations.clone(),
        });
        for proposal in &source.proposals {
            let mapping = mappings
                .get(&proposal.generated_node_id)
                .ok_or(TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
            let document_locator = format!(
                "{}/{}.adoc",
                mapping.destination_node_locator, mapping.destination_portable_name
            );
            new_nodes.push(TaskRebaselineNewNode {
                old_task_id: mapping.old_task_id,
                generated_node_id: mapping.generated_node_id,
                destination_parent_node_id: mapping.destination_parent_node_id,
                destination_node_locator: mapping.destination_node_locator.clone(),
                document_locator,
                exact_source: proposal.proposed_task_source.clone(),
                source_sha256: sha256(proposal.proposed_task_source.as_bytes()),
            });
        }
    }
    source_replacements.sort_by(|left, right| left.document_locator.cmp(&right.document_locator));
    new_nodes.sort_by_key(|node| node.generated_node_id);
    if new_nodes.len() != preview.identity_map.len() {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    Ok((source_replacements, new_nodes))
}

fn physical_projection(
    source_replacements: &[TaskRebaselineSourceReplacement],
    new_nodes: &[TaskRebaselineNewNode],
) -> Vec<PhysicalInventoryProjectionChange> {
    let mut projection = Vec::new();
    for replacement in source_replacements {
        projection.push(PhysicalInventoryProjectionChange::ReplaceRegularFile {
            locator: replacement.document_locator.clone(),
            expected_bytes: replacement.original_source.as_bytes().to_vec(),
            next_bytes: replacement.proposed_source.as_bytes().to_vec(),
        });
    }
    for node in new_nodes {
        projection.push(PhysicalInventoryProjectionChange::CreateDirectory {
            locator: node.destination_node_locator.clone(),
        });
        projection.push(PhysicalInventoryProjectionChange::CreateRegularFile {
            locator: node.document_locator.clone(),
            bytes: node.exact_source.as_bytes().to_vec(),
        });
    }
    projection
}

#[allow(
    clippy::too_many_lines,
    reason = "the restart-time v1/A/C/proposal authority closure is kept in one audit boundary"
)]
pub(crate) fn validate_summary(
    summary: &TaskRebaselineTransactionSummary,
) -> Result<(), TaskRebaselineTransactionError> {
    if summary.source_replacements.len() > MAX_EXECUTABLE_SOURCES
        || summary.new_nodes.len() > MAX_EXECUTABLE_NODES
        || summary.physical_pre_entries.len() > MAX_EXECUTABLE_PHYSICAL_ENTRIES
        || summary.physical_post_entries.len() > MAX_EXECUTABLE_PHYSICAL_ENTRIES
    {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    let projected_entry_count = validate_execution_entry_budget(
        summary.physical_pre_entries.len(),
        summary.new_nodes.len(),
    )?;
    if summary.physical_post_entries.len() != projected_entry_count {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    validate_reviewed_shape(
        &summary.reviewed_preview,
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
    .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    summary
        .physical_pre_state
        .validate()
        .map_err(TaskRebaselineTransactionError::PhysicalInventory)?;
    summary
        .external_snapshot
        .physical_inventory
        .validate()
        .map_err(TaskRebaselineTransactionError::PhysicalInventory)?;
    summary
        .workspace_root_identity
        .validate()
        .map_err(TaskRebaselineTransactionError::PhysicalInventory)?;
    summary
        .external_snapshot
        .root_identity
        .validate()
        .map_err(TaskRebaselineTransactionError::PhysicalInventory)?;
    summary
        .physical_post_state
        .validate()
        .map_err(TaskRebaselineTransactionError::PhysicalInventory)?;
    let (expected_sources, expected_nodes) = derive_execution_changes(&summary.reviewed_preview)?;
    let expected_projection = physical_projection(&expected_sources, &expected_nodes);
    let expected_post_entries = project_physical_inventory_records_from_records(
        &summary.physical_pre_entries,
        &expected_projection,
    )?;
    let canonical_drafts = summary
        .draft_sensitive_node_ids
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let canonical_sources = summary
        .source_replacements
        .windows(2)
        .all(|pair| pair[0].document_locator < pair[1].document_locator);
    let canonical_nodes = summary
        .new_nodes
        .windows(2)
        .all(|pair| pair[0].generated_node_id < pair[1].generated_node_id);
    let pre_binding = physical_inventory_binding_from_records(&summary.physical_pre_entries)?;
    let expected_post_binding = physical_inventory_binding_from_records(&expected_post_entries)?;
    if summary.schema != TASK_REBASELINE_TRANSACTION_SCHEMA
        || summary.authorization != TaskRebaselineExecutionAuthorization::PreReleaseOwnerLocal
        || summary.base_workspace_revision != summary.reviewed_preview.base_workspace_revision
        || !summary.reviewed_preview.conversion_ready()
        || summary.identity_map != summary.reviewed_preview.identity_map
        || summary.physical_pre_state != summary.external_snapshot.physical_inventory
        || summary.physical_pre_state != pre_binding
        || summary.physical_post_state != expected_post_binding
        || summary.physical_post_entries != expected_post_entries
        || summary.source_replacements != expected_sources
        || summary.new_nodes != expected_nodes
        || summary.owner_actor_binding.is_empty()
        || summary.owner_actor_binding.len() > 4_096
        || summary.owner_authorization_epoch.is_empty()
        || summary.owner_authorization_epoch.len() > 4_096
        || summary.draft_observation.observation.starts_with("core:")
        || summary.external_snapshot.schema != TASK_REBASELINE_EXTERNAL_SNAPSHOT_SCHEMA
        || summary.external_snapshot.receipt_digest
            != snapshot_receipt_digest(
                &summary.external_snapshot.physical_inventory,
                &summary.external_snapshot.root_identity,
            )?
        || canonical_drafts != summary.draft_sensitive_node_ids
        || !canonical_sources
        || !canonical_nodes
        || summary.new_nodes.len() != summary.identity_map.len()
        || summary.authority_digest != authority_digest(summary)?
        || serde_json::to_vec(summary)
            .map_or(true, |bytes| bytes.len() > MAX_EXECUTABLE_AUTHORITY_BYTES)
        || summary.source_replacements.iter().any(|replacement| {
            replacement.base_revision != DocumentRevision::from_source(&replacement.original_source)
                || replacement.next_revision
                    != DocumentRevision::from_source(&replacement.proposed_source)
                || !matches!(
                    replacement.annotations,
                    TaskRebaselineAnnotationInventory::ConfirmedAbsent
                )
        })
        || summary.new_nodes.iter().any(|node| {
            node.source_sha256 != sha256(node.exact_source.as_bytes())
                || !summary.identity_map.iter().any(|mapping| {
                    mapping.old_task_id == node.old_task_id
                        && mapping.generated_node_id == node.generated_node_id
                        && mapping.destination_parent_node_id == node.destination_parent_node_id
                        && mapping.destination_node_locator == node.destination_node_locator
                })
        })
    {
        return Err(TaskRebaselineTransactionError::InvalidReviewedAuthority);
    }
    Ok(())
}

fn snapshot_receipt(
    physical_inventory: PhysicalInventoryBinding,
    root_identity: PhysicalRootIdentityBinding,
) -> Result<TaskRebaselineExternalSnapshotReceipt, TaskRebaselineTransactionError> {
    Ok(TaskRebaselineExternalSnapshotReceipt {
        schema: TASK_REBASELINE_EXTERNAL_SNAPSHOT_SCHEMA.to_owned(),
        receipt_digest: snapshot_receipt_digest(&physical_inventory, &root_identity)?,
        physical_inventory,
        root_identity,
    })
}

fn snapshot_receipt_digest(
    binding: &PhysicalInventoryBinding,
    root_identity: &PhysicalRootIdentityBinding,
) -> Result<String, TaskRebaselineTransactionError> {
    let bytes = serde_json::to_vec(&(
        TASK_REBASELINE_EXTERNAL_SNAPSHOT_SCHEMA,
        binding,
        root_identity,
    ))
    .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    let mut hasher = Sha256::new();
    hasher.update(SNAPSHOT_RECEIPT_DIGEST_DOMAIN);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn authority_digest(
    summary: &TaskRebaselineTransactionSummary,
) -> Result<String, TaskRebaselineTransactionError> {
    let bytes = serde_json::to_vec(&(
        (
            &summary.schema,
            summary.authorization,
            summary.owner_confirmation_id,
            &summary.owner_actor_binding,
            &summary.owner_authorization_epoch,
        ),
        (
            &summary.base_workspace_revision,
            summary.workspace_root_node_id,
            &summary.workspace_root_document_revision,
            &summary.workspace_root_identity,
        ),
        (
            &summary.physical_pre_state,
            &summary.physical_post_state,
            &summary.physical_pre_entries,
            &summary.physical_post_entries,
            &summary.external_snapshot,
        ),
        (
            &summary.reviewed_preview,
            &summary.identity_map,
            &summary.source_replacements,
            &summary.new_nodes,
            &summary.draft_sensitive_node_ids,
            &summary.draft_observation,
        ),
    ))
    .map_err(|_| TaskRebaselineTransactionError::InvalidReviewedAuthority)?;
    let mut hasher = Sha256::new();
    hasher.update(AUTHORITY_DIGEST_DOMAIN);
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use sha2::Sha256;
    use tempfile::TempDir;

    use super::*;
    use crate::workspace_transaction::{
        MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES, bind_task_rebaseline_commit_confirmation,
        commit_workspace_transaction_with_journal_limit_for_test,
        debug_workspace_transaction_journal_for_test,
        prepare_workspace_transaction_applying_recovery_fixture,
        prepare_workspace_transaction_committed_recovery_fixture,
        prepare_workspace_transaction_displaced_replace_file_recovery_fixture,
        prepare_workspace_transaction_fully_applied_recovery_fixture,
        prepare_workspace_transaction_recovery_fixture,
        rewrite_workspace_transaction_journal_applying_with_limit_for_test,
        validate_task_rebaseline_transaction_artifacts_for_test,
        workspace_transaction_journal_lifecycle_bytes_for_test,
    };
    use crate::{
        capture_local_task_rebaseline_authority, create_child_node, create_workspace,
        has_unfinished_workspace_transaction, plan_task_rebaseline, recover_workspace_transactions,
    };

    const LEGACY_TASK_ID: &str = "11111111-1111-4111-8111-111111111111";

    struct Fixture {
        temporary: TempDir,
        workspace: PathBuf,
        snapshot: PathBuf,
        source_document: PathBuf,
        preview: TaskRebaselinePlan,
    }

    fn setup_fixture() -> Fixture {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        let root = create_workspace(&workspace).unwrap();
        let source = create_child_node(&root.path, "Source").unwrap();
        let exact = format!(
            "---\nweftext:\n  id: \"{}\"\n---\n= Source\n\n* [ ] Ship safely task:[id={LEGACY_TASK_ID}]\n",
            source.id
        );
        fs::write(&source.document_path, exact).unwrap();
        fs::create_dir(workspace.join(".git")).unwrap();
        fs::write(
            workspace.join(".git/unmanaged-sentinel.bin"),
            b"unmanaged sentinel exact bytes",
        )
        .unwrap();
        let authority = capture_local_task_rebaseline_authority(&workspace).unwrap();
        let preview = plan_task_rebaseline(&authority).unwrap();
        assert!(preview.conversion_ready(), "{:#?}", preview.blockers);
        let snapshot = temporary.path().join("Snapshot");
        copy_tree(&workspace, &snapshot);
        Fixture {
            temporary,
            workspace,
            snapshot,
            source_document: source.document_path,
            preview,
        }
    }

    fn copy_tree(source: &Path, destination: &Path) {
        fs::create_dir(destination).unwrap();
        let mut entries = fs::read_dir(source)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn single_file_tree_digest_preimage(file_name: &str, bytes: &[u8]) -> Vec<u8> {
        let mut preimage = b"weftext.tree.v1\0F\0".to_vec();
        preimage.extend_from_slice(file_name.as_bytes());
        preimage.push(0);
        preimage.extend_from_slice(&Sha256::digest(bytes));
        preimage.push(0);
        preimage
    }

    fn draft_authority(
        dirty: impl IntoIterator<Item = NodeId>,
    ) -> TaskRebaselineDraftRegistryAuthority {
        TaskRebaselineDraftRegistryAuthority::new_complete_native(
            WorkspaceDraftRegistryView::new(
                format!("native:test-complete:{}", NodeId::new_v4()),
                dirty,
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn named_draft_authority(
        observation: &str,
        dirty: impl IntoIterator<Item = NodeId>,
    ) -> TaskRebaselineDraftRegistryAuthority {
        TaskRebaselineDraftRegistryAuthority::new_complete_native(
            WorkspaceDraftRegistryView::new(observation, dirty).unwrap(),
        )
        .unwrap()
    }

    fn owner_confirmation() -> TaskRebaselineOwnerConfirmation {
        TaskRebaselineOwnerConfirmation::new_native_assertion("local-owner:test", "acl-epoch:1")
            .unwrap()
    }

    fn rollback_owner_confirmation(
        actor: &str,
        epoch: &str,
    ) -> TaskRebaselineRollbackOwnerConfirmation {
        TaskRebaselineRollbackOwnerConfirmation::new_native_assertion(actor, epoch).unwrap()
    }

    fn executable(fixture: &Fixture) -> TaskRebaselineTransactionPlan {
        plan_owner_local_task_rebaseline_transaction(
            &fixture.workspace,
            &fixture.snapshot,
            &fixture.preview,
            &owner_confirmation(),
            &draft_authority([]),
        )
        .unwrap()
    }

    fn transaction_with_fresh_owner(
        plan: &TaskRebaselineTransactionPlan,
    ) -> WorkspaceTransactionPlan {
        let mut transaction = plan.transaction.clone();
        let fresh = owner_confirmation();
        bind_task_rebaseline_commit_confirmation(
            &mut transaction,
            fresh.confirmation_id,
            fresh.actor_binding,
            fresh.authorization_epoch,
        )
        .unwrap();
        transaction
    }

    fn committed_forward(
        fixture: &Fixture,
    ) -> (TaskRebaselineTransactionPlan, CommittedTaskRebaseline) {
        let plan = executable(fixture);
        let token = plan
            .draft_gate()
            .executable_token
            .clone()
            .expect("clean forward draft authority");
        let committed = commit_owner_local_task_rebaseline_transaction(
            &plan,
            &token,
            &owner_confirmation(),
            &draft_authority([]),
        )
        .unwrap();
        (plan, committed)
    }

    fn executable_rollback(
        forward: &TaskRebaselineTransactionPlan,
        committed: &CommittedTaskRebaseline,
        actor: &str,
        epoch: &str,
    ) -> TaskRebaselineRollbackPlan {
        plan_owner_local_task_rebaseline_exact_rollback(
            forward,
            committed,
            &rollback_owner_confirmation(actor, epoch),
            &draft_authority([]),
        )
        .unwrap()
    }

    fn rollback_transaction_with_fresh_owner(
        plan: &TaskRebaselineRollbackPlan,
    ) -> WorkspaceTransactionPlan {
        let mut transaction = plan.transaction.clone();
        let fresh = rollback_owner_confirmation(
            &plan.summary.owner_actor_binding,
            &plan.summary.owner_authorization_epoch,
        );
        bind_task_rebaseline_rollback_commit_confirmation(
            &mut transaction,
            fresh.confirmation_id,
            fresh.actor_binding,
            fresh.authorization_epoch,
        )
        .unwrap();
        transaction
    }

    fn assert_prepared_journal_tamper_rejected(mutate: impl FnOnce(&mut serde_json::Value)) {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let journal_path = journal.join("journal.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        mutate(&mut value);
        fs::write(&journal_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(recover_workspace_transactions(&fixture.workspace).is_err());
        assert!(journal.exists(), "invalid journal evidence must remain");
    }

    fn assert_prepared_rollback_journal_tamper_rejected(
        mutate: impl FnOnce(&mut serde_json::Value),
    ) {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let journal_path = journal.join("journal.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        mutate(&mut value);
        fs::write(&journal_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(recover_workspace_transactions(&fixture.workspace).is_err());
        assert!(journal.exists(), "invalid v4 evidence must remain");
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
    }

    #[test]
    fn owner_local_commit_rechecks_full_authority_and_reaches_exact_c() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        assert_eq!(
            plan.summary.draft_sensitive_node_ids.len(),
            2,
            "root and task-free source are both draft-sensitive"
        );
        let token = plan
            .draft_gate()
            .executable_token
            .clone()
            .expect("clean complete draft authority");
        let committed = commit_owner_local_task_rebaseline_transaction(
            &plan,
            &token,
            &owner_confirmation(),
            &draft_authority([]),
        )
        .unwrap();
        assert_eq!(
            committed.transaction.action,
            crate::StructuralAction::TaskRebaseline
        );
        assert!(
            !fs::read_to_string(&fixture.source_document)
                .unwrap()
                .contains("task:[")
        );
        for node in &committed.summary.new_nodes {
            assert_eq!(
                fs::read(fixture.workspace.join(&node.document_locator)).unwrap(),
                node.exact_source.as_bytes()
            );
        }
        let lease = acquire_workspace_transaction_lease(&fixture.workspace).unwrap();
        let physical = capture_stable_workspace_physical_inventory(&lease).unwrap();
        assert_eq!(physical.binding(), &committed.summary.physical_post_state);
        assert_eq!(physical.records(), committed.summary.physical_post_entries);
    }

    #[test]
    fn owner_changed_exact_rollback_reaches_original_a() {
        let fixture = setup_fixture();
        let original_source = fs::read(&fixture.source_document).unwrap();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        assert!(
            rollback
                .summary
                .draft_sensitive_node_ids
                .iter()
                .all(|node_id| rollback
                    .transaction
                    .draft_sensitive_node_ids
                    .contains(node_id))
        );
        assert!(
            rollback
                .summary
                .forward_authority
                .new_nodes
                .iter()
                .all(|node| rollback
                    .summary
                    .draft_sensitive_node_ids
                    .contains(&node.generated_node_id))
        );
        let token = rollback
            .draft_gate()
            .executable_token
            .clone()
            .expect("clean rollback draft authority");
        let result = commit_owner_local_task_rebaseline_exact_rollback(
            &rollback,
            &token,
            &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
            &draft_authority([]),
        )
        .unwrap();
        assert_eq!(
            result.transaction.revision,
            forward.summary.base_workspace_revision
        );
        assert_eq!(fs::read(&fixture.source_document).unwrap(), original_source);
        for node in &forward.summary.new_nodes {
            assert!(
                !fixture
                    .workspace
                    .join(&node.destination_node_locator)
                    .exists()
            );
        }
        let lease = acquire_workspace_transaction_lease(&fixture.workspace).unwrap();
        let physical = capture_stable_workspace_physical_inventory(&lease).unwrap();
        assert_eq!(physical.binding(), &forward.summary.physical_pre_state);
        assert_eq!(physical.records(), forward.summary.physical_pre_entries);
        let snapshot = verify_disjoint_external_physical_tree(
            &lease,
            &fixture.snapshot,
            &forward.summary.physical_pre_state,
        )
        .unwrap();
        assert_eq!(snapshot.binding(), &forward.summary.physical_pre_state);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "one end-to-end Debug surface test keeps every opaque forward/rollback secret together"
    )]
    fn rollback_debug_is_source_path_owner_and_draft_redacted() {
        const SOURCE_SECRET: &str = "SOURCE_DEBUG_SECRET_7c41e690";
        const FORWARD_ACTOR_SECRET: &str = "FORWARD_ACTOR_DEBUG_SECRET_08e134c2";
        const FORWARD_EPOCH_SECRET: &str = "FORWARD_EPOCH_DEBUG_SECRET_43afe881";
        const ROLLBACK_ACTOR_SECRET: &str = "ROLLBACK_ACTOR_DEBUG_SECRET_1da9f603";
        const ROLLBACK_EPOCH_SECRET: &str = "ROLLBACK_EPOCH_DEBUG_SECRET_66bc529a";
        const FORWARD_DRAFT_SECRET: &str = "FORWARD_DRAFT_DEBUG_SECRET_e33d7d3f";
        const FORWARD_COMMIT_DRAFT_SECRET: &str = "FORWARD_COMMIT_DRAFT_SECRET_63d75615";
        const ROLLBACK_DRAFT_SECRET: &str = "ROLLBACK_DRAFT_DEBUG_SECRET_b222d13f";
        const COMMIT_DRAFT_SECRET: &str = "COMMIT_DRAFT_DEBUG_SECRET_f61f81e5";
        const SNAPSHOT_NAME_SECRET: &str = "SNAPSHOT_PATH_DEBUG_SECRET_84cd8de7";

        let mut fixture = setup_fixture();
        let source = fs::read_to_string(&fixture.source_document)
            .unwrap()
            .replace("Ship safely", SOURCE_SECRET);
        fs::write(&fixture.source_document, source).unwrap();
        let authority = capture_local_task_rebaseline_authority(&fixture.workspace).unwrap();
        fixture.preview = plan_task_rebaseline(&authority).unwrap();
        fs::remove_dir_all(&fixture.snapshot).unwrap();
        fixture.snapshot = fixture.temporary.path().join(SNAPSHOT_NAME_SECRET);
        copy_tree(&fixture.workspace, &fixture.snapshot);
        let workspace_absolute_secret = fixture.workspace.to_string_lossy().into_owned();
        let source_document_absolute_secret =
            fixture.source_document.to_string_lossy().into_owned();
        let snapshot_absolute_secret = fixture.snapshot.to_string_lossy().into_owned();
        let hidden_dirty_id = NodeId::new_v4();
        let hidden_dirty_id_secret = hidden_dirty_id.to_string();

        let forward_review = TaskRebaselineOwnerConfirmation::new_native_assertion(
            FORWARD_ACTOR_SECRET,
            FORWARD_EPOCH_SECRET,
        )
        .unwrap();
        let forward = plan_owner_local_task_rebaseline_transaction(
            &fixture.workspace,
            &fixture.snapshot,
            &fixture.preview,
            &forward_review,
            &named_draft_authority(FORWARD_DRAFT_SECRET, [hidden_dirty_id]),
        )
        .unwrap();
        let source_locator_secret = forward.summary.source_replacements[0]
            .document_locator
            .clone();
        let generated_node_locator_secret = forward.summary.new_nodes[0]
            .destination_node_locator
            .clone();
        let generated_document_locator_secret =
            forward.summary.new_nodes[0].document_locator.clone();
        let forward_summary_debug = format!("{:#?}", forward.summary);
        let forward_plan_debug = format!("{forward:#?}");
        let forward_token = forward.draft_gate().executable_token.as_ref().unwrap();
        let forward_committed = commit_owner_local_task_rebaseline_transaction(
            &forward,
            forward_token,
            &TaskRebaselineOwnerConfirmation::new_native_assertion(
                FORWARD_ACTOR_SECRET,
                FORWARD_EPOCH_SECRET,
            )
            .unwrap(),
            &named_draft_authority(FORWARD_COMMIT_DRAFT_SECRET, []),
        )
        .unwrap();
        let forward_result_debug = format!("{forward_committed:#?}");
        let rollback = plan_owner_local_task_rebaseline_exact_rollback(
            &forward,
            &forward_committed,
            &rollback_owner_confirmation(ROLLBACK_ACTOR_SECRET, ROLLBACK_EPOCH_SECRET),
            &named_draft_authority(ROLLBACK_DRAFT_SECRET, [hidden_dirty_id]),
        )
        .unwrap();

        let evidence_debug = format!("{:#?}", forward_committed.committed_evidence);
        let summary_debug = format!("{:#?}", rollback.summary);
        let plan_debug = format!("{rollback:#?}");
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal_path = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let journal_debug = debug_workspace_transaction_journal_for_test(&journal_path).unwrap();
        fs::remove_dir_all(&journal_path).unwrap();

        let rollback_token = rollback.draft_gate().executable_token.as_ref().unwrap();
        let committed_rollback = commit_owner_local_task_rebaseline_exact_rollback(
            &rollback,
            rollback_token,
            &rollback_owner_confirmation(ROLLBACK_ACTOR_SECRET, ROLLBACK_EPOCH_SECRET),
            &named_draft_authority(COMMIT_DRAFT_SECRET, [hidden_dirty_id]),
        )
        .unwrap();
        let result_debug = format!("{committed_rollback:#?}");
        let all_debug = [
            forward_summary_debug,
            forward_plan_debug,
            forward_result_debug,
            evidence_debug,
            summary_debug,
            plan_debug,
            journal_debug,
            result_debug,
        ]
        .join("\n");
        assert!(all_debug.contains("TaskRebaselineRollback"));
        for secret in [
            SOURCE_SECRET,
            FORWARD_ACTOR_SECRET,
            FORWARD_EPOCH_SECRET,
            ROLLBACK_ACTOR_SECRET,
            ROLLBACK_EPOCH_SECRET,
            FORWARD_DRAFT_SECRET,
            FORWARD_COMMIT_DRAFT_SECRET,
            ROLLBACK_DRAFT_SECRET,
            COMMIT_DRAFT_SECRET,
            SNAPSHOT_NAME_SECRET,
            workspace_absolute_secret.as_str(),
            source_document_absolute_secret.as_str(),
            snapshot_absolute_secret.as_str(),
            source_locator_secret.as_str(),
            generated_node_locator_secret.as_str(),
            generated_document_locator_secret.as_str(),
            hidden_dirty_id_secret.as_str(),
        ] {
            assert!(
                !all_debug.contains(secret),
                "rollback Debug disclosed secret {secret}: {all_debug}"
            );
        }
    }

    #[test]
    fn rollback_commit_requires_reviewed_owner_epoch_and_complete_c_drafts() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let token = rollback.draft_gate().executable_token.as_ref().unwrap();
        assert!(
            commit_owner_local_task_rebaseline_exact_rollback(
                &rollback,
                token,
                &rollback_owner_confirmation("wrong-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            )
            .is_err()
        );
        assert!(
            commit_owner_local_task_rebaseline_exact_rollback(
                &rollback,
                token,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:3"),
                &draft_authority([]),
            )
            .is_err()
        );
        let generated = rollback.summary.forward_authority.new_nodes[0].generated_node_id;
        assert!(matches!(
            commit_owner_local_task_rebaseline_exact_rollback(
                &rollback,
                token,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([generated]),
            ),
            Err(TaskRebaselineTransactionError::WorkspaceTransaction(
                WorkspaceTransactionError::DraftGateBlocked(_)
            ))
        ));
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            rollback.summary.forward_authority.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
    }

    #[test]
    fn rollback_planning_rejects_managed_ignored_and_unmanaged_c_drift_without_writes() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        fs::write(&fixture.source_document, b"managed drift must remain").unwrap();
        assert!(
            plan_owner_local_task_rebaseline_exact_rollback(
                &forward,
                &committed,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            )
            .is_err()
        );
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            b"managed drift must remain"
        );

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let ignored = fixture.workspace.join(".git/unmanaged-sentinel.bin");
        fs::write(&ignored, b"ignored drift must remain").unwrap();
        assert!(
            plan_owner_local_task_rebaseline_exact_rollback(
                &forward,
                &committed,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            )
            .is_err()
        );
        assert_eq!(fs::read(&ignored).unwrap(), b"ignored drift must remain");

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let unmanaged = fixture.workspace.join("unmanaged-after-c.bin");
        fs::write(&unmanaged, b"unmanaged drift must remain").unwrap();
        assert!(
            plan_owner_local_task_rebaseline_exact_rollback(
                &forward,
                &committed,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            )
            .is_err()
        );
        assert_eq!(
            fs::read(&unmanaged).unwrap(),
            b"unmanaged drift must remain"
        );
    }

    #[test]
    fn rollback_requires_unchanged_exact_a_snapshot_and_rejects_repeated_attempt() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        fs::write(fixture.snapshot.join("tampered-after-c.bin"), b"tampered").unwrap();
        assert!(matches!(
            plan_owner_local_task_rebaseline_exact_rollback(
                &forward,
                &committed,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            ),
            Err(TaskRebaselineTransactionError::ExternalSnapshotChanged)
        ));

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let token = rollback.draft_gate().executable_token.as_ref().unwrap();
        commit_owner_local_task_rebaseline_exact_rollback(
            &rollback,
            token,
            &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
            &draft_authority([]),
        )
        .unwrap();
        assert!(matches!(
            plan_owner_local_task_rebaseline_exact_rollback(
                &forward,
                &committed,
                &rollback_owner_confirmation("replacement-owner:test", "acl-epoch:2"),
                &draft_authority([]),
            ),
            Err(TaskRebaselineTransactionError::RollbackPreStateChanged)
        ));
    }

    #[test]
    fn prestate_snapshot_authority_and_fresh_drafts_fail_closed() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        fs::write(fixture.snapshot.join("tampered.bin"), b"tampered").unwrap();
        let token = plan.draft_gate().executable_token.as_ref().unwrap();
        assert!(
            commit_owner_local_task_rebaseline_transaction(
                &plan,
                token,
                &owner_confirmation(),
                &draft_authority([]),
            )
            .is_err()
        );

        let fixture = setup_fixture();
        let plan = executable(&fixture);
        fs::remove_dir_all(&fixture.snapshot).unwrap();
        let token = plan.draft_gate().executable_token.as_ref().unwrap();
        assert!(
            commit_owner_local_task_rebaseline_transaction(
                &plan,
                token,
                &owner_confirmation(),
                &draft_authority([]),
            )
            .is_err()
        );

        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let dirty = plan.summary.draft_sensitive_node_ids[0];
        let token = plan.draft_gate().executable_token.as_ref().unwrap();
        assert!(matches!(
            commit_owner_local_task_rebaseline_transaction(
                &plan,
                token,
                &owner_confirmation(),
                &draft_authority([dirty]),
            ),
            Err(TaskRebaselineTransactionError::WorkspaceTransaction(
                WorkspaceTransactionError::DraftGateBlocked(_)
            ))
        ));
    }

    #[test]
    fn prepared_and_applying_recovery_choose_only_exact_old_or_new() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let report = recover_workspace_transactions(&fixture.workspace).unwrap();
        assert_eq!(report.prepared_removed, 1);
        assert!(
            fs::read_to_string(&fixture.source_document)
                .unwrap()
                .contains("task:[")
        );

        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        prepare_workspace_transaction_applying_recovery_fixture(&transaction, 1).unwrap();
        let report = recover_workspace_transactions(&fixture.workspace).unwrap();
        assert_eq!(report.applying_rolled_back, 1);
        assert!(
            fs::read_to_string(&fixture.source_document)
                .unwrap()
                .contains("task:[")
        );
        assert!(
            !fixture
                .workspace
                .join(&plan.summary.new_nodes[0].destination_node_locator)
                .exists()
        );
    }

    #[test]
    fn prepared_v3_with_tampered_staged_tree_is_retained_without_workspace_write() {
        let fixture = setup_fixture();
        let original = fs::read(&fixture.source_document).unwrap();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        fs::write(journal.join("staged/0/tampered.bin"), b"tampered").unwrap();
        assert!(matches!(
            recover_workspace_transactions(&fixture.workspace),
            Err(WorkspaceTransactionError::RecoveryRequired(_))
        ));
        assert!(journal.exists());
        assert_eq!(fs::read(&fixture.source_document).unwrap(), original);
    }

    #[test]
    fn v3_and_v4_unreferenced_artifact_containers_are_never_cleanup_authority() {
        for as_directory in [false, true] {
            let fixture = setup_fixture();
            let original = fs::read(&fixture.source_document).unwrap();
            let plan = executable(&fixture);
            let transaction = transaction_with_fresh_owner(&plan);
            let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
            let unknown = journal.join("holding");
            if as_directory {
                fs::create_dir(&unknown).unwrap();
            } else {
                fs::write(&unknown, b"unreferenced v3 artifact secret").unwrap();
            }
            assert!(matches!(
                recover_workspace_transactions(&fixture.workspace),
                Err(WorkspaceTransactionError::RecoveryRequired(_))
            ));
            assert!(journal.exists());
            assert!(unknown.exists());
            assert_eq!(fs::read(&fixture.source_document).unwrap(), original);
        }

        for as_directory in [false, true] {
            let fixture = setup_fixture();
            let (forward, committed) = committed_forward(&fixture);
            let rollback = executable_rollback(
                &forward,
                &committed,
                "replacement-owner:test",
                "acl-epoch:2",
            );
            let transaction = rollback_transaction_with_fresh_owner(&rollback);
            let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
            let unknown = journal.join("holding");
            if as_directory {
                fs::create_dir(&unknown).unwrap();
            } else {
                fs::write(&unknown, b"unreferenced v4 artifact secret").unwrap();
            }
            assert!(matches!(
                recover_task_rebaseline_exact_rollback_for_plan(&rollback),
                Err(TaskRebaselineTransactionError::WorkspaceTransaction(
                    WorkspaceTransactionError::RecoveryRequired(_)
                ))
            ));
            assert!(journal.exists());
            assert!(unknown.exists());
            assert_eq!(
                fs::read(&fixture.source_document).unwrap(),
                forward.summary.source_replacements[0]
                    .proposed_source
                    .as_bytes()
            );
        }
    }

    #[test]
    fn v3_create_and_v4_remove_tree_roots_reject_digest_equivalent_regular_files() {
        let fixture = setup_fixture();
        let original = fs::read(&fixture.source_document).unwrap();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let node = &plan.summary.new_nodes[0];
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap();
        let digest_preimage =
            single_file_tree_digest_preimage(document_file, node.exact_source.as_bytes());
        let staged_tree = journal.join("staged/0");
        fs::remove_dir_all(&staged_tree).unwrap();
        fs::write(&staged_tree, &digest_preimage).unwrap();
        let journal_value: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.join("journal.json")).unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&digest_preimage)),
            journal_value["steps"][0]["digest"].as_str().unwrap()
        );
        assert!(matches!(
            recover_workspace_transactions(&fixture.workspace),
            Err(WorkspaceTransactionError::RecoveryRequired(_))
        ));
        assert!(journal.exists());
        assert!(staged_tree.is_file());
        assert_eq!(fs::read(&fixture.source_document).unwrap(), original);

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal =
            prepare_workspace_transaction_fully_applied_recovery_fixture(&transaction).unwrap();
        let node = &forward.summary.new_nodes[0];
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap();
        let digest_preimage =
            single_file_tree_digest_preimage(document_file, node.exact_source.as_bytes());
        let remove_index = forward.summary.source_replacements.len();
        let removed_tree = journal.join(format!("removed/{remove_index}"));
        fs::remove_dir_all(&removed_tree).unwrap();
        fs::write(&removed_tree, &digest_preimage).unwrap();
        let journal_value: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.join("journal.json")).unwrap()).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&digest_preimage)),
            journal_value["steps"][remove_index]["digest"]
                .as_str()
                .unwrap()
        );
        assert!(recover_task_rebaseline_exact_rollback_for_plan(&rollback).is_err());
        assert!(journal.exists());
        assert!(removed_tree.is_file());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );
    }

    #[test]
    fn v4_prepared_applying_and_post_recovery_follow_c_to_a_direction() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let value: serde_json::Value =
            serde_json::from_slice(&fs::read(journal.join("journal.json")).unwrap()).unwrap();
        assert_eq!(value["schema"], "weftext.workspace-transaction.v4");
        assert_eq!(value["direction"], "rollback_rebaseline");
        assert!(value["task_rebaseline_rollback_authority"].is_object());
        let report = recover_task_rebaseline_exact_rollback_for_plan(&rollback).unwrap();
        assert_eq!(report.prepared_removed, 1);
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        prepare_workspace_transaction_applying_recovery_fixture(&transaction, 1).unwrap();
        let report = recover_task_rebaseline_exact_rollback_for_plan(&rollback).unwrap();
        assert_eq!(report.applying_rolled_back, 1);
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
        assert!(
            fixture
                .workspace
                .join(&forward.summary.new_nodes[0].destination_node_locator)
                .exists()
        );

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        prepare_workspace_transaction_fully_applied_recovery_fixture(&transaction).unwrap();
        let report = recover_task_rebaseline_exact_rollback_for_plan(&rollback).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );
        assert!(
            !fixture
                .workspace
                .join(&forward.summary.new_nodes[0].destination_node_locator)
                .exists()
        );
    }

    #[test]
    fn v4_remove_holding_tamper_and_committed_non_a_are_retained() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal =
            prepare_workspace_transaction_fully_applied_recovery_fixture(&transaction).unwrap();
        let remove_index = forward.summary.source_replacements.len();
        fs::write(
            journal.join(format!("removed/{remove_index}/tampered.bin")),
            b"tampered holding",
        )
        .unwrap();
        assert!(matches!(
            recover_task_rebaseline_exact_rollback_for_plan(&rollback),
            Err(TaskRebaselineTransactionError::WorkspaceTransaction(
                WorkspaceTransactionError::RecoveryRequired(_)
            ))
        ));
        assert!(journal.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal =
            prepare_workspace_transaction_committed_recovery_fixture(&transaction).unwrap();
        fs::write(
            &fixture.source_document,
            &forward.summary.source_replacements[0].proposed_source,
        )
        .unwrap();
        assert!(recover_task_rebaseline_exact_rollback_for_plan(&rollback).is_err());
        assert!(journal.exists());
    }

    #[test]
    fn v4_replace_rename_gap_recovers_c_and_tampered_evidence_is_retained() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal =
            prepare_workspace_transaction_displaced_replace_file_recovery_fixture(&transaction, 0)
                .unwrap();
        assert!(!fixture.source_document.exists());
        let report = recover_task_rebaseline_exact_rollback_for_plan(&rollback).unwrap();
        assert_eq!(report.applying_rolled_back, 1);
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
        assert!(!journal.exists());

        for evidence in ["staged/0.file", "displaced/0.file"] {
            let fixture = setup_fixture();
            let (forward, committed) = committed_forward(&fixture);
            let rollback = executable_rollback(
                &forward,
                &committed,
                "replacement-owner:test",
                "acl-epoch:2",
            );
            let transaction = rollback_transaction_with_fresh_owner(&rollback);
            let journal = prepare_workspace_transaction_displaced_replace_file_recovery_fixture(
                &transaction,
                0,
            )
            .unwrap();
            fs::write(journal.join(evidence), b"tampered gap evidence").unwrap();
            assert!(recover_task_rebaseline_exact_rollback_for_plan(&rollback).is_err());
            assert!(journal.exists());
            assert!(
                !fixture.source_document.exists(),
                "ambiguous gap must remain byte-for-byte untouched"
            );
        }
    }

    #[test]
    fn v4_prepared_tampered_staging_is_retained_at_exact_c() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        fs::write(journal.join("staged/0.file"), b"tampered prepared A bytes").unwrap();
        assert!(recover_task_rebaseline_exact_rollback_for_plan(&rollback).is_err());
        assert!(journal.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
    }

    #[test]
    fn v3_recovery_artifact_file_boundary_and_oversize_fail_before_hashing() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let transaction_directory =
            prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let replacement_index = plan.summary.new_nodes.len();
        let staged_file = transaction_directory.join(format!("staged/{replacement_index}.file"));
        fs::OpenOptions::new()
            .write(true)
            .open(&staged_file)
            .unwrap()
            .set_len(MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES)
            .unwrap();
        validate_task_rebaseline_transaction_artifacts_for_test(&transaction_directory).unwrap();

        fs::OpenOptions::new()
            .write(true)
            .open(&staged_file)
            .unwrap()
            .set_len(MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            recover_task_rebaseline_transaction_for_plan(&plan),
            Err(TaskRebaselineTransactionError::WorkspaceTransaction(
                WorkspaceTransactionError::RecoveryRequired(_)
            ))
        ));
        assert!(transaction_directory.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            plan.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );
    }

    #[test]
    fn v3_staged_and_v4_holding_tree_children_reject_oversize_without_workspace_writes() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let transaction_directory =
            prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let generated_document = Path::new(&plan.summary.new_nodes[0].document_locator)
            .file_name()
            .unwrap();
        let staged_child = transaction_directory
            .join("staged/0")
            .join(generated_document);
        fs::OpenOptions::new()
            .write(true)
            .open(&staged_child)
            .unwrap()
            .set_len(MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES + 1)
            .unwrap();
        assert!(recover_task_rebaseline_transaction_for_plan(&plan).is_err());
        assert!(transaction_directory.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            plan.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );

        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let transaction_directory =
            prepare_workspace_transaction_fully_applied_recovery_fixture(&transaction).unwrap();
        let remove_index = forward.summary.source_replacements.len();
        let generated_document = Path::new(&forward.summary.new_nodes[0].document_locator)
            .file_name()
            .unwrap();
        let holding_child = transaction_directory
            .join(format!("removed/{remove_index}"))
            .join(generated_document);
        fs::OpenOptions::new()
            .write(true)
            .open(&holding_child)
            .unwrap()
            .set_len(MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES + 1)
            .unwrap();
        assert!(recover_task_rebaseline_exact_rollback_for_plan(&rollback).is_err());
        assert!(transaction_directory.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );
        assert!(
            !fixture
                .workspace
                .join(&forward.summary.new_nodes[0].destination_node_locator)
                .exists()
        );
    }

    #[test]
    fn v4_journal_wire_limit_rejects_before_replacing_readable_evidence() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let transaction_directory =
            prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let journal_path = transaction_directory.join("journal.json");
        let prepared_bytes = fs::read(&journal_path).unwrap();
        let exact_applying_bytes = u64::try_from(prepared_bytes.len()).unwrap();

        assert!(matches!(
            rewrite_workspace_transaction_journal_applying_with_limit_for_test(
                &transaction_directory,
                exact_applying_bytes - 1,
            ),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("journal exceeds")
        ));
        assert_eq!(fs::read(&journal_path).unwrap(), prepared_bytes);
        assert!(
            debug_workspace_transaction_journal_for_test(&transaction_directory)
                .unwrap()
                .contains("Prepared")
        );

        rewrite_workspace_transaction_journal_applying_with_limit_for_test(
            &transaction_directory,
            exact_applying_bytes,
        )
        .unwrap();
        let applying_bytes = fs::read(&journal_path).unwrap();
        assert_eq!(applying_bytes.len(), prepared_bytes.len());
        assert_ne!(applying_bytes, prepared_bytes);
        assert!(
            debug_workspace_transaction_journal_for_test(&transaction_directory)
                .unwrap()
                .contains("Applying")
        );
    }

    #[test]
    fn v4_lifecycle_wire_limit_rejects_before_initial_transaction_materialization() {
        let fixture = setup_fixture();
        let (forward, committed) = committed_forward(&fixture);
        let rollback = executable_rollback(
            &forward,
            &committed,
            "replacement-owner:test",
            "acl-epoch:2",
        );
        let transaction = rollback_transaction_with_fresh_owner(&rollback);
        let lifecycle_bytes =
            workspace_transaction_journal_lifecycle_bytes_for_test(&transaction).unwrap();
        let transaction_directory = fixture
            .workspace
            .join(".__weftext-transaction-workspace-current");

        assert!(matches!(
            commit_workspace_transaction_with_journal_limit_for_test(
                &transaction,
                lifecycle_bytes - 1,
            ),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("journal lifecycle exceeds")
        ));
        assert!(!transaction_directory.exists());
        assert!(!has_unfinished_workspace_transaction(&fixture.workspace).unwrap());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .proposed_source
                .as_bytes()
        );
        assert!(
            fixture
                .workspace
                .join(&forward.summary.new_nodes[0].destination_node_locator)
                .is_dir()
        );

        commit_workspace_transaction_with_journal_limit_for_test(&transaction, lifecycle_bytes)
            .unwrap();
        assert!(!transaction_directory.exists());
        assert_eq!(
            fs::read(&fixture.source_document).unwrap(),
            forward.summary.source_replacements[0]
                .original_source
                .as_bytes()
        );
        assert!(
            !fixture
                .workspace
                .join(&forward.summary.new_nodes[0].destination_node_locator)
                .exists()
        );
    }

    #[test]
    fn displaced_replace_file_intermediate_recovers_only_with_exact_staging_evidence() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let step_index = plan.summary.new_nodes.len();
        let journal = prepare_workspace_transaction_displaced_replace_file_recovery_fixture(
            &transaction,
            step_index,
        )
        .unwrap();
        let original_source = plan.summary.source_replacements[0]
            .original_source
            .as_bytes()
            .to_vec();
        let sentinel = fixture.workspace.join(".git/unmanaged-sentinel.bin");
        assert!(!fixture.source_document.exists());
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            b"unmanaged sentinel exact bytes"
        );

        let report = recover_workspace_transactions(&fixture.workspace).unwrap();
        assert_eq!(report.applying_rolled_back, 1);
        assert_eq!(fs::read(&fixture.source_document).unwrap(), original_source);
        assert_eq!(
            fs::read(&sentinel).unwrap(),
            b"unmanaged sentinel exact bytes"
        );
        assert!(!journal.exists());
        assert_eq!(
            fs::read(fixture.snapshot.join(".git/unmanaged-sentinel.bin")).unwrap(),
            b"unmanaged sentinel exact bytes"
        );

        for tampered_relative in [
            format!("staged/{step_index}.file"),
            format!("displaced/{step_index}.file"),
        ] {
            let fixture = setup_fixture();
            let plan = executable(&fixture);
            let transaction = transaction_with_fresh_owner(&plan);
            let journal = prepare_workspace_transaction_displaced_replace_file_recovery_fixture(
                &transaction,
                step_index,
            )
            .unwrap();
            fs::write(
                journal.join(tampered_relative),
                b"tampered recovery evidence",
            )
            .unwrap();
            assert!(matches!(
                recover_workspace_transactions(&fixture.workspace),
                Err(WorkspaceTransactionError::RecoveryRequired(_))
            ));
            assert!(journal.exists(), "ambiguous evidence must remain");
            assert!(
                !fixture.source_document.exists(),
                "classifier must not write the workspace before rejecting evidence"
            );
            assert_eq!(
                fs::read(fixture.workspace.join(".git/unmanaged-sentinel.bin")).unwrap(),
                b"unmanaged sentinel exact bytes"
            );
        }
    }

    #[test]
    fn ambiguous_unmanaged_change_preserves_journal_and_concurrent_bytes() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let journal =
            prepare_workspace_transaction_applying_recovery_fixture(&transaction, 1).unwrap();
        let concurrent = fixture.workspace.join("concurrent-unmanaged.bin");
        fs::write(&concurrent, b"do not overwrite").unwrap();
        assert!(matches!(
            recover_workspace_transactions(&fixture.workspace),
            Err(WorkspaceTransactionError::RecoveryRequired(_)
                | WorkspaceTransactionError::InvalidJournal(_))
        ));
        assert_eq!(fs::read(&concurrent).unwrap(), b"do not overwrite");
        assert!(journal.exists(), "ambiguous recovery evidence must remain");
    }

    #[test]
    fn committed_restart_recovery_accepts_only_exact_c_and_snapshot_a() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        prepare_workspace_transaction_committed_recovery_fixture(&transaction).unwrap();
        let report = recover_workspace_transactions(&fixture.workspace).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        assert_eq!(report.committed_transactions.len(), 1);
        assert!(
            !fs::read_to_string(&fixture.source_document)
                .unwrap()
                .contains("task:[")
        );
    }

    #[test]
    fn applying_journal_with_exact_c_is_idempotently_finalized() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        prepare_workspace_transaction_fully_applied_recovery_fixture(&transaction).unwrap();
        let report = recover_workspace_transactions(&fixture.workspace).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        assert_eq!(report.committed_transactions.len(), 1);
        assert!(
            !fs::read_to_string(&fixture.source_document)
                .unwrap()
                .contains("task:[")
        );
    }

    #[test]
    fn journal_authority_tamper_and_workspace_root_replacement_fail_closed() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let transaction = transaction_with_fresh_owner(&plan);
        let journal = prepare_workspace_transaction_recovery_fixture(&transaction).unwrap();
        let journal_path = journal.join("journal.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        value["task_rebaseline_authority"]["identityMap"][0]["destinationPortableName"] =
            serde_json::Value::String("tampered".to_owned());
        fs::write(&journal_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        assert!(recover_workspace_transactions(&fixture.workspace).is_err());
        assert!(journal.exists());

        let fixture = setup_fixture();
        let plan = executable(&fixture);
        let displaced = fixture.temporary.path().join("displaced-workspace");
        fs::rename(&fixture.workspace, &displaced).unwrap();
        copy_tree(&fixture.snapshot, &fixture.workspace);
        let token = plan.draft_gate().executable_token.as_ref().unwrap();
        assert!(
            commit_owner_local_task_rebaseline_transaction(
                &plan,
                token,
                &owner_confirmation(),
                &draft_authority([]),
            )
            .is_err()
        );
    }

    #[test]
    fn journal_v3_shape_is_required_and_cannot_be_downgraded() {
        assert_prepared_journal_tamper_rejected(|value| {
            assert_eq!(value["schema"], "weftext.workspace-transaction.v3");
            assert_eq!(value["action"], "task_rebaseline");
            assert!(value["task_rebaseline_authority"].is_object());
            assert!(value["task_rebaseline_snapshot_authority"].is_object());
            assert!(value["task_rebaseline_commit_confirmation"].is_object());
            value["schema"] =
                serde_json::Value::String("weftext.workspace-transaction.v1".to_owned());
        });
        assert_prepared_journal_tamper_rejected(|value| {
            value["schema"] =
                serde_json::Value::String("weftext.workspace-transaction.v2".to_owned());
        });
        assert_prepared_journal_tamper_rejected(|value| {
            value
                .as_object_mut()
                .unwrap()
                .remove("task_rebaseline_snapshot_authority");
        });
    }

    #[test]
    fn journal_v4_direction_schema_authority_and_digest_confusion_fail_closed() {
        assert_prepared_rollback_journal_tamper_rejected(|value| {
            assert_eq!(value["schema"], "weftext.workspace-transaction.v4");
            value["direction"] = serde_json::Value::String("apply_rebaseline".to_owned());
        });
        assert_prepared_rollback_journal_tamper_rejected(|value| {
            value["schema"] =
                serde_json::Value::String("weftext.workspace-transaction.v3".to_owned());
        });
        assert_prepared_rollback_journal_tamper_rejected(|value| {
            value["task_rebaseline_rollback_authority"]["ownerActorBinding"] =
                serde_json::Value::String("tampered-owner".to_owned());
        });
        assert_prepared_rollback_journal_tamper_rejected(|value| {
            value["authority_digest"] = serde_json::Value::String("0".repeat(64));
        });
    }

    #[test]
    fn restart_summary_validation_rejects_open_v1_shape_unprojected_c_and_detached_changes() {
        let fixture = setup_fixture();
        let plan = executable(&fixture);
        validate_summary(&plan.summary).unwrap();

        let mut open_v1 = plan.summary.clone();
        open_v1.reviewed_preview.plan_digest = "0".repeat(64);
        open_v1.authority_digest = authority_digest(&open_v1).unwrap();
        assert!(matches!(
            validate_summary(&open_v1),
            Err(TaskRebaselineTransactionError::InvalidReviewedAuthority)
        ));

        let mut invented_c = plan.summary.clone();
        let sentinel = invented_c
            .physical_post_entries
            .iter_mut()
            .find(|record| record.locator == ".git/unmanaged-sentinel.bin")
            .unwrap();
        sentinel.sha256 = Some("1".repeat(64));
        invented_c.physical_post_state =
            physical_inventory_binding_from_records(&invented_c.physical_post_entries).unwrap();
        invented_c.authority_digest = authority_digest(&invented_c).unwrap();
        assert!(matches!(
            validate_summary(&invented_c),
            Err(TaskRebaselineTransactionError::InvalidReviewedAuthority
                | TaskRebaselineTransactionError::PhysicalInventory(_))
        ));

        let mut detached = plan.summary.clone();
        detached.source_replacements[0].source_node_id = NodeId::new_v4();
        detached.authority_digest = authority_digest(&detached).unwrap();
        assert!(matches!(
            validate_summary(&detached),
            Err(TaskRebaselineTransactionError::InvalidReviewedAuthority)
        ));
    }

    #[test]
    fn executable_physical_entry_limit_is_checked_before_projection_materialization() {
        assert_eq!(
            validate_execution_entry_budget(MAX_EXECUTABLE_PHYSICAL_ENTRIES - 2, 1).unwrap(),
            MAX_EXECUTABLE_PHYSICAL_ENTRIES
        );
        assert!(validate_execution_entry_budget(MAX_EXECUTABLE_PHYSICAL_ENTRIES - 1, 1).is_err());
        assert!(validate_execution_entry_budget(MAX_EXECUTABLE_PHYSICAL_ENTRIES, 1).is_err());
        assert!(validate_execution_entry_budget(0, usize::MAX).is_err());
    }

    #[test]
    fn overlapping_external_root_and_tampered_summary_are_rejected() {
        let fixture = setup_fixture();
        assert!(
            plan_owner_local_task_rebaseline_transaction(
                &fixture.workspace,
                fixture.workspace.join("Source"),
                &fixture.preview,
                &owner_confirmation(),
                &draft_authority([]),
            )
            .is_err()
        );

        let mut plan = executable(&fixture);
        let replacement = if plan.summary.authority_digest.starts_with('0') {
            "1"
        } else {
            "0"
        };
        plan.summary
            .authority_digest
            .replace_range(0..1, replacement);
        let token = plan.draft_gate().executable_token.as_ref().unwrap();
        assert!(
            commit_owner_local_task_rebaseline_transaction(
                &plan,
                token,
                &owner_confirmation(),
                &draft_authority([]),
            )
            .is_err()
        );
    }
}
