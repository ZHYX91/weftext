use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use serde::de::{self, DeserializeSeed as _, Visitor};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::Builder;

use crate::content_boundary::{
    linked_or_reparse, reject_linked_existing_ancestors, validate_managed_file_path,
    validate_managed_node_path,
};
use crate::frontmatter::{
    new_node_document, replace_node_id, set_adjacent_heading_body, set_node_aliases,
    set_node_child_sort, set_node_icon, set_node_sibling_rank,
};
use crate::workspace::{validate_node_name, validate_portable_path_component};
use crate::workspace_revision::WORKSPACE_TRANSACTION_PREFIX;
use crate::{
    ANNOTATIONS_FILE_NAME, AdjacentHeadingBody, Annotation, AnnotationAction, AnnotationBody,
    AnnotationKind, AnnotationReanchorOutcome, AnnotationReplicaCompleteness, AnnotationResolution,
    AnnotationState, AnnotationStore, CalendarDate, ChildSort, ChronoPeriod, ChronoPlan,
    DocumentEdit, DocumentEditPlan, DocumentRevision, MAX_ANNOTATION_STORE_BYTES, NodeId,
    ThreadMessage, WorkspaceDocumentGeneration, WorkspaceIndex, WorkspaceRevision,
    annotation_suggestion_edit, build_annotation_target, build_workspace_link_index,
    canonical_document_file_name_for, canonical_document_locator_for, canonical_document_path_for,
    plan_document_edit, read_node_document, read_workspace_revision, reanchor_annotation,
    scan_workspace,
};

pub const TRASH_NODE_NAME: &str = ".weftext-trash";
const JOURNAL_SCHEMA_V1: &str = "weftext.workspace-transaction.v1";
const JOURNAL_SCHEMA_V2: &str = "weftext.workspace-transaction.v2";
const JOURNAL_SCHEMA_V3: &str = "weftext.workspace-transaction.v3";
const JOURNAL_SCHEMA_V4: &str = "weftext.workspace-transaction.v4";
const WORKSPACE_TRANSACTION_DIRECTORY: &str = ".__weftext-transaction-workspace-current";
const WORKSPACE_TRANSACTION_CLEANUP_PREFIX: &str = ".__weftext-transaction-workspace-cleanup-";
const WORKSPACE_TRANSACTION_ROLLBACK_PREFIX: &str = ".__weftext-transaction-workspace-rolled-back-";
pub const WORKSPACE_TRANSACTION_LEASE_FILE_NAME: &str =
    ".__weftext-transaction-workspace-owner.lock";
const ROLLBACK_MARKER_SCHEMA: &str = "weftext.workspace-transaction-rollback.v1";
const MAX_SNAPSHOT_RESTORE_NODES: usize = 10_000;
const MAX_SNAPSHOT_RESTORE_ENTRIES: usize = 100_000;
const MAX_SNAPSHOT_RESTORE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
// Every non-journal TaskRebaseline recovery artifact is an exact source file already bounded by
// the sealed authority. The aggregate ceiling leaves room for the complete <=48 MiB rollback
// authority while the independent journal.json keeps its existing 64 MiB bound.
pub(crate) const MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES: u64 =
    crate::task_rebaseline::TASK_REBASELINE_MAX_DOCUMENT_BYTES as u64;
const MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_BYTES: u64 = MAX_JOURNAL_BYTES;
const MAX_JOURNAL_STEPS: usize = 100_000;
const MAX_JOURNAL_PATH_CHANGES: usize = 100_000;
const MAX_TRANSACTION_ENTRIES: usize = 200_000;
const EXTERNAL_RECEIPT_CLAIM_SCHEMA: &str = "weftext.workspace-external-receipt.v1";
const EXTERNAL_RECEIPT_PAYLOAD_FILE: &str = "external-receipt.payload";
const EXTERNAL_RECEIPT_CLAIM_FILE: &str = "external-receipt.json";
const MAX_EXTERNAL_RECEIPT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_EXTERNAL_RECEIPT_CLAIM_BYTES: u64 = 128 * 1024;
const MAX_ROLLBACK_MARKER_BYTES: u64 = 128 * 1024;
const LEGACY_TRASH_BACKUP_MANIFEST_FILE: &str = "_weftext.legacy-trash-backup.json";
const LEGACY_TRASH_BACKUP_CONTENT_DIRECTORY: &str = "content";

const fn journal_schema_for_action(action: StructuralAction) -> &'static str {
    match action {
        StructuralAction::TaskPromotion => JOURNAL_SCHEMA_V2,
        StructuralAction::TaskRebaseline => JOURNAL_SCHEMA_V3,
        _ => JOURNAL_SCHEMA_V1,
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuralAction {
    Create,
    Import,
    SnapshotRestore,
    Rename,
    Move,
    Copy,
    Trash,
    Restore,
    Presentation,
    Annotation,
    Chrono,
    TaskEdit,
    TaskRecurrenceCompletion,
    TaskDependencies,
    TaskPromotion,
    TaskRebaseline,
    NodeMetadata,
    PermanentDelete,
    TrashMigration,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum TaskRebaselineJournalDirection {
    ApplyRebaseline,
    RollbackRebaseline,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspacePathChange {
    pub source_node_id: Option<NodeId>,
    pub node_id: NodeId,
    pub old_path: Option<String>,
    pub new_path: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceDocumentChange {
    pub node_id: NodeId,
    pub path: String,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub edit_count: u64,
}

/// Identity treatment for one complete managed-node branch action.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceIdentityPolicy {
    Preserve,
    Rekey,
}

/// Stable identity and display evidence for the root of one branch action.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceScopeRootNode {
    pub node_id: NodeId,
    pub display_name: String,
}

/// One exact permanent-identity rewrite in a copied node branch.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceIdentityMapEntry {
    pub source_node_id: NodeId,
    pub destination_node_id: NodeId,
}

/// Invocation origin recorded with one immutable typed action target.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceTargetResolution {
    FocusedPane,
    ExplicitRow,
    CallerExplicit,
}

/// Immutable action target captured before planning. Destination authority is
/// recorded separately by path changes and destination node UUIDs.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceCapturedTarget {
    Node {
        #[serde(rename = "nodeId")]
        node_id: NodeId,
        #[serde(rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
    TrashItem {
        #[serde(rename = "trashItemId")]
        trash_item_id: crate::TrashItemId,
        #[serde(rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
    OwnedResource {
        #[serde(rename = "ownerNodeId")]
        owner_node_id: NodeId,
        name: String,
        #[serde(rename = "resolvedBy")]
        resolved_by: WorkspaceTargetResolution,
    },
}

/// Closed Core-authored impact summary for a single managed-node branch action.
///
/// This summary is intentionally absent for operations that have no single
/// operated branch, such as resource batches and heterogeneous permanent
/// deletion. Exact paths and old-to-new copy identities remain in the plan's
/// path and document changes; the UUID sets here are the canonical draft and
/// authorization projection.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceTransactionScopeSummary {
    pub root_node: WorkspaceScopeRootNode,
    pub descendant_node_count: u64,
    pub resource_count: u64,
    pub annotation_sidecar_count: u64,
    pub byte_total: u64,
    pub affected_document_node_ids: Vec<NodeId>,
    pub rewritten_document_node_ids: Vec<NodeId>,
    pub identity_policy: WorkspaceIdentityPolicy,
    pub trash_item_count: u64,
    pub operation_id: Option<crate::TrashOperationId>,
}

const DRAFT_GATE_SCHEMA: &str = "weftext.workspace-draft-gate/v1";

/// Authoritative draft-registry observation supplied by the invoking native
/// or hosted session boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceDraftRegistryView {
    pub observation: String,
    pub dirty_node_ids: Vec<NodeId>,
}

impl WorkspaceDraftRegistryView {
    /// Builds one canonical bounded registry view.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty/oversized observation or an excessive
    /// number of dirty identities.
    pub fn new(
        observation: impl Into<String>,
        dirty_node_ids: impl IntoIterator<Item = NodeId>,
    ) -> Result<Self, WorkspaceTransactionError> {
        let observation = observation.into();
        if observation.is_empty() || observation.len() > 4_096 {
            return Err(WorkspaceTransactionError::DraftGateAuthorityMismatch);
        }
        let mut dirty_node_ids = dirty_node_ids.into_iter().collect::<Vec<_>>();
        if dirty_node_ids.len() > 100_000 {
            return Err(WorkspaceTransactionError::DraftGateAuthorityMismatch);
        }
        canonicalize_node_ids(&mut dirty_node_ids);
        Ok(Self {
            observation,
            dirty_node_ids,
        })
    }

    pub(crate) fn empty_authority() -> Self {
        Self {
            observation: "core:no-draft-registry".to_owned(),
            dirty_node_ids: Vec::new(),
        }
    }
}

/// Scoped preview evidence returned by Core's exact draft gate.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceDraftGatePreview {
    pub required_clean_node_ids: Vec<NodeId>,
    pub blocking_dirty_node_ids: Vec<NodeId>,
    pub observation_digest: String,
    pub executable_token: Option<WorkspaceDraftGateToken>,
}

/// Digest-bound executable evidence issued only for a clean scoped preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceDraftGateToken {
    schema: String,
    plan_id: String,
    base_revision: WorkspaceRevision,
    required_clean_node_ids: Vec<NodeId>,
    preview_observation_digest: String,
    authority_digest: String,
}

#[derive(Clone, Debug)]
pub struct WorkspaceTransactionPlan {
    pub plan_id: String,
    pub action: StructuralAction,
    pub workspace_root: PathBuf,
    pub base_revision: WorkspaceRevision,
    pub path_changes: Vec<WorkspacePathChange>,
    pub document_changes: Vec<WorkspaceDocumentChange>,
    pub generated_node_ids: Vec<NodeId>,
    /// Closed branch summary for actions that operate on exactly one managed
    /// node branch. It is durable transaction authority, not a UI estimate.
    pub scope_summary: Option<WorkspaceTransactionScopeSummary>,
    /// Closed checklist-promotion impact evidence. Present exactly for `TaskPromotion`.
    pub promotion_summary: Option<crate::TaskPromotionSummary>,
    /// Complete, canonical old-to-new UUID map. It is non-empty only for a
    /// rekeying copy and covers every node in that copied source branch.
    pub identity_map: Vec<WorkspaceIdentityMapEntry>,
    pub captured_target: Option<WorkspaceCapturedTarget>,
    /// Canonically ordered immutable node UUIDs explicitly captured by the
    /// action request. Non-node Trash item identities remain in typed Trash
    /// evidence.
    pub target_node_ids: Vec<NodeId>,
    /// Canonically ordered UUIDs whose authoritative device/session drafts
    /// must be checked before preview authorization and again before commit.
    pub draft_sensitive_node_ids: Vec<NodeId>,
    pub import_authority: Option<WorkspaceImportAuthority>,
    /// Exact complete-replica sidecar state bound to annotation plans. This is
    /// carried into the durable journal and is absent for every other action.
    pub annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    reviewed_trash_request: Option<crate::TrashReviewedRequest>,
    legacy_trash_migration_backup: Option<crate::LegacyTrashMigrationBackup>,
    task_dependencies_authority: Option<TaskDependenciesPlanAuthority>,
    task_promotion_authority: Option<TaskPromotionPlanAuthority>,
    task_rebaseline_authority:
        Option<crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary>,
    task_rebaseline_external_snapshot: Option<crate::VerifiedExternalPhysicalTree>,
    task_rebaseline_commit_confirmation: Option<TaskRebaselineCommitConfirmation>,
    task_rebaseline_rollback_authority:
        Option<crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary>,
    task_rebaseline_rollback_commit_confirmation: Option<TaskRebaselineRollbackCommitConfirmation>,
    steps: Vec<PlannedStep>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TaskPromotionSidecarState {
    Present { sha256: String },
    ConfirmedAbsent,
}

#[derive(Clone, Debug)]
pub(crate) struct TaskPromotionWorkspaceMaterial {
    pub(crate) root: PathBuf,
    pub(crate) workspace_revision: WorkspaceRevision,
    pub(crate) summary: crate::TaskPromotionSummary,
    pub(crate) source_node_directory: PathBuf,
    pub(crate) source_document_plan: DocumentEditPlan,
    pub(crate) source_base: String,
    pub(crate) destination_node_directory: PathBuf,
    pub(crate) task_document_source: String,
    pub(crate) annotation_replica_completeness: AnnotationReplicaCompleteness,
    pub(crate) expected_source_sidecar: TaskPromotionSidecarState,
    pub(crate) source_sidecar_bytes: Option<Vec<u8>>,
    pub(crate) task_sidecar_bytes: Option<Vec<u8>>,
    pub(crate) disclosure: TaskPromotionDisclosure,
}

#[allow(
    dead_code,
    reason = "pre-release Core primitive is not exported before native Owner authority exists"
)]
pub(crate) struct TaskRebaselineWorkspaceMaterial {
    pub(crate) root: PathBuf,
    pub(crate) summary: crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary,
    pub(crate) external_snapshot: crate::VerifiedExternalPhysicalTree,
}

#[allow(
    dead_code,
    clippy::too_many_lines,
    reason = "pre-release exact rollback remains crate-private until native Owner authority exists"
)]
pub(crate) struct TaskRebaselineRollbackWorkspaceMaterial {
    pub(crate) root: PathBuf,
    pub(crate) summary: crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary,
    pub(crate) external_snapshot: crate::VerifiedExternalPhysicalTree,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct TaskRebaselineCommitConfirmation {
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct TaskRebaselineRollbackCommitConfirmation {
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
}

impl fmt::Debug for TaskRebaselineRollbackCommitConfirmation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineRollbackCommitConfirmation")
            .field("confirmation_id", &self.confirmation_id)
            .field("actor_binding", &"<redacted>")
            .field("authorization_epoch", &"<redacted>")
            .finish()
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskRebaselineJournalSnapshotAuthority {
    canonical_root: String,
    physical_inventory: crate::PhysicalInventoryBinding,
    root_identity: crate::physical_inventory::PhysicalRootIdentityBinding,
}

impl fmt::Debug for TaskRebaselineJournalSnapshotAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskRebaselineJournalSnapshotAuthority")
            .field("canonical_root", &"<redacted>")
            .field("physical_inventory", &self.physical_inventory)
            .field("root_identity", &self.root_identity)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TaskPromotionDisclosure {
    Owner,
    Scoped,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct TaskPromotionPlanAuthority {
    source_node_id: NodeId,
    generated_node_id: NodeId,
    parent_node_id: NodeId,
    source_document_path: String,
    destination_node_path: String,
    task_document_path: String,
    source_base_digest: String,
    source_next_digest: String,
    task_document_digest: String,
    task_payload_digest: String,
    annotation_replica_completeness: AnnotationReplicaCompleteness,
    source_sidecar_path: String,
    expected_source_sidecar: TaskPromotionSidecarState,
    source_sidecar_next_digest: Option<String>,
    task_sidecar_path: String,
    task_sidecar_digest: Option<String>,
    disclosure: TaskPromotionDisclosure,
}

#[derive(Clone, Debug)]
enum TaskDependenciesPlanAuthority {
    LegacyInline {
        node_id: NodeId,
        base_revision: DocumentRevision,
        next_revision: DocumentRevision,
        next_source_digest: String,
    },
    TaskNodeReplacement {
        node_id: NodeId,
        base_revision: DocumentRevision,
        next_revision: DocumentRevision,
        before_depends_on: Vec<NodeId>,
        after_depends_on: Vec<NodeId>,
        edits: Vec<DocumentEdit>,
        next_source_digest: String,
    },
}

impl WorkspaceTransactionPlan {
    #[must_use]
    pub fn trash_item_changes(&self) -> &[crate::WorkspaceTrashPlanItemChange] {
        &self.trash_item_changes
    }

    /// Returns the closed, path-free artifact required to reproduce this exact Trash preview in
    /// another process. Non-Trash plans return `None`.
    #[must_use]
    pub const fn reviewed_trash_request(&self) -> Option<&crate::TrashReviewedRequest> {
        self.reviewed_trash_request.as_ref()
    }
}

/// Binds the invocation resolver origin without changing the already-frozen
/// typed target identity. Trash reviewed-request authority is regenerated by
/// Core so preview/replan/journal evidence stays identical.
///
/// # Errors
///
/// Returns an error when the plan has no captured target, a focused-pane
/// origin is used for a non-node target, or the resulting authority is invalid.
pub fn bind_workspace_transaction_target_resolution(
    plan: &mut WorkspaceTransactionPlan,
    resolution: WorkspaceTargetResolution,
) -> Result<(), WorkspaceTransactionError> {
    let target = plan.captured_target.as_mut().ok_or_else(|| {
        WorkspaceTransactionError::Metadata("plan has no captured target".to_owned())
    })?;
    match target {
        WorkspaceCapturedTarget::Node { resolved_by, .. } => *resolved_by = resolution,
        WorkspaceCapturedTarget::TrashItem { resolved_by, .. }
        | WorkspaceCapturedTarget::OwnedResource { resolved_by, .. } => {
            if resolution == WorkspaceTargetResolution::FocusedPane {
                return Err(WorkspaceTransactionError::Metadata(
                    "focused-pane resolution is valid only for a node target".to_owned(),
                ));
            }
            *resolved_by = resolution;
        }
    }
    if let Some(action) = plan
        .reviewed_trash_request
        .as_ref()
        .map(|request| request.action.clone())
    {
        plan.reviewed_trash_request = Some(
            crate::workspace_trash::build_trash_reviewed_request(
                &plan.workspace_root,
                plan,
                action,
            )
            .map_err(WorkspaceTransactionError::InvalidTrashReviewedRequest)?,
        );
    }
    validate_plan_scope_authority(plan)
}

/// Exact node-local sidecar state frozen into one annotation plan and journal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AnnotationSidecarPlanAuthority {
    pub node_id: NodeId,
    pub destination: String,
    pub workspace_revision: WorkspaceRevision,
    pub completeness: AnnotationReplicaCompleteness,
    pub expected_state: AnnotationSidecarExpectedState,
}

/// Whether a complete replica contained an exact sidecar or proved its
/// absence when Core captured the annotation snapshot.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnnotationSidecarExpectedState {
    Present { sha256: String },
    ConfirmedAbsent,
}

/// Opaque Core snapshot used for both request binding and mutation planning.
/// The root binding and parsed store cannot be authored by a UI request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnnotationSidecarSnapshot {
    canonical_workspace_root: PathBuf,
    node_id: NodeId,
    workspace_revision: WorkspaceRevision,
    completeness: AnnotationReplicaCompleteness,
    expected_state: AnnotationSidecarExpectedState,
    store: AnnotationStore,
}

impl AnnotationSidecarSnapshot {
    #[must_use]
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    #[must_use]
    pub fn workspace_revision(&self) -> &WorkspaceRevision {
        &self.workspace_revision
    }

    #[must_use]
    pub fn expected_state(&self) -> &AnnotationSidecarExpectedState {
        &self.expected_state
    }

    #[must_use]
    pub fn store(&self) -> &AnnotationStore {
        &self.store
    }

    #[must_use]
    pub fn into_store(self) -> AnnotationStore {
        self.store
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommittedWorkspaceTransaction {
    pub plan_id: String,
    pub action: StructuralAction,
    pub base_revision: WorkspaceRevision,
    pub revision: WorkspaceRevision,
    pub path_changes: Vec<WorkspacePathChange>,
    pub scope_summary: Option<WorkspaceTransactionScopeSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promotion_summary: Option<crate::TaskPromotionSummary>,
    pub identity_map: Vec<WorkspaceIdentityMapEntry>,
    pub captured_target: Option<WorkspaceCapturedTarget>,
    pub target_node_ids: Vec<NodeId>,
    pub draft_sensitive_node_ids: Vec<NodeId>,
    pub import_authority: Option<WorkspaceImportAuthority>,
}

/// Exact reviewed import proposal bound to a Core workspace transaction.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkspaceImportAuthority {
    pub proposal_id: String,
    pub proposal_digest: String,
}

/// Opaque durable receipt bytes staged under a retained Core journal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceTransactionReceiptHandoff {
    pub destination: Option<PathBuf>,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

/// One resource carried by an already validated import proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceImportResource {
    pub locator: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// One canonical node carried by an already validated import proposal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceImportNode {
    pub locator: String,
    pub node_id: NodeId,
    pub document_file: String,
    pub exact_source: String,
    pub document_sha256: String,
    pub resources: Vec<WorkspaceImportResource>,
}

/// One exact node-local annotation sidecar carried by a reviewed snapshot restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRestoreAnnotationSidecar {
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// One identity-preserving managed node carried by a reviewed snapshot-tree restore.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceRestoreTreeNode {
    /// Canonical destination locator relative to the target workspace root.
    pub locator: String,
    pub node_id: NodeId,
    pub document_file: String,
    pub exact_source: String,
    pub document_sha256: String,
    pub annotation_sidecar: Option<WorkspaceRestoreAnnotationSidecar>,
    pub resources: Vec<WorkspaceImportResource>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub prepared_removed: u64,
    pub applying_rolled_back: u64,
    pub committed_cleaned: u64,
    pub committed_retained: u64,
    pub committed_transactions: Vec<CommittedWorkspaceTransaction>,
}

/// Read-only state of the sole unfinished transaction for one exact import authority.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
#[allow(clippy::large_enum_variant)]
pub enum WorkspaceImportTransactionState {
    Absent,
    Prepared {
        plan_id: String,
    },
    Applying {
        plan_id: String,
    },
    Committed {
        transaction: CommittedWorkspaceTransaction,
    },
}

#[derive(Clone, Debug)]
enum PlannedStep {
    CreateTree {
        destination: String,
        payload: TreePayload,
    },
    CreateFile {
        destination: String,
        next_digest: String,
        next_bytes: Vec<u8>,
    },
    MovePath {
        source: String,
        destination: String,
        digest: String,
    },
    RemovePath {
        source: String,
        digest: String,
    },
    ReplaceFile {
        destination: String,
        base_digest: String,
        next_digest: String,
        next_bytes: Vec<u8>,
    },
}

#[derive(Clone, Debug)]
struct TreePayload {
    directories: Vec<String>,
    files: Vec<TreeFile>,
    digest: String,
}

#[derive(Clone, Debug)]
struct TreeFile {
    path: String,
    bytes: Vec<u8>,
}

struct PlanningContext {
    root: PathBuf,
    generation: WorkspaceDocumentGeneration,
    inventory: crate::WorkspaceInventory,
    index: WorkspaceIndex,
    revision: WorkspaceRevision,
}

impl PlanningContext {
    fn load(root: &Path) -> Result<Self, WorkspaceTransactionError> {
        let selected_root = root;
        let root = canonical_non_linked_workspace_root(selected_root)?;
        ensure_no_unfinished_transaction(&root)
            .map_err(|error| rebase_transaction_scan_error(error, &root, selected_root))?;
        let inventory = scan_workspace(&root);
        if !inventory.is_valid() {
            if inventory.legacy_trash_format {
                return Err(WorkspaceTransactionError::LegacyTrashMigrationRequired);
            }
            if let Some(issue) = inventory.issues.iter().find(|issue| {
                matches!(
                    issue.code,
                    crate::InventoryIssueCode::TrashReconciliationRequired
                        | crate::InventoryIssueCode::DuplicateIdentity
                ) && issue.path.starts_with(root.join(TRASH_NODE_NAME))
            }) {
                return Err(WorkspaceTransactionError::TrashReconciliation(
                    issue.message.clone(),
                ));
            }
            return Err(WorkspaceTransactionError::InvalidWorkspace);
        }
        let index = WorkspaceIndex::rebuild(&inventory)
            .map_err(|_| WorkspaceTransactionError::InvalidWorkspace)?;
        let revision =
            read_workspace_revision(&root).map_err(WorkspaceTransactionError::Revision)?;
        Ok(Self {
            root,
            generation: inventory.generation,
            inventory,
            index,
            revision,
        })
    }

    fn load_legacy_trash(root: &Path) -> Result<Self, WorkspaceTransactionError> {
        let selected_root = root;
        let root = canonical_non_linked_workspace_root(selected_root)?;
        ensure_no_unfinished_transaction(&root)
            .map_err(|error| rebase_transaction_scan_error(error, &root, selected_root))?;
        let inventory = scan_workspace(&root);
        if !inventory.legacy_trash_format
            || inventory.issues.is_empty()
            || !inventory
                .issues
                .iter()
                .all(|issue| issue.code == crate::InventoryIssueCode::LegacyTrashMigrationRequired)
        {
            return Err(if inventory.legacy_trash_format {
                WorkspaceTransactionError::InvalidWorkspace
            } else {
                WorkspaceTransactionError::NoChange
            });
        }
        let mut indexable = inventory.clone();
        indexable.issues.clear();
        indexable.legacy_trash_format = false;
        let index = WorkspaceIndex::rebuild(&indexable)
            .map_err(|_| WorkspaceTransactionError::InvalidWorkspace)?;
        let revision =
            read_workspace_revision(&root).map_err(WorkspaceTransactionError::Revision)?;
        Ok(Self {
            root,
            generation: inventory.generation,
            inventory,
            index,
            revision,
        })
    }

    fn node(&self, id: NodeId) -> Result<&crate::NodeRecord, WorkspaceTransactionError> {
        self.inventory
            .nodes
            .iter()
            .find(|node| node.id == Some(id))
            .ok_or(WorkspaceTransactionError::UnknownNode(id))
    }

    fn root_node(&self) -> Result<&crate::NodeRecord, WorkspaceTransactionError> {
        self.inventory
            .nodes
            .iter()
            .find(|node| node.parent_id.is_none())
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)
    }

    fn require_managed_subtree(
        &self,
        node: &crate::NodeRecord,
    ) -> Result<(), WorkspaceTransactionError> {
        let relative = relative_string(&self.root, &node.path)?;
        if let Some(boundary) = self.inventory.boundaries.iter().find(|boundary| {
            boundary.relative_path == relative
                || boundary.relative_path.starts_with(&format!("{relative}/"))
        }) {
            let classification = if boundary.ignored {
                "ignored"
            } else {
                "unmanaged"
            };
            return Err(WorkspaceTransactionError::ContentBoundary(format!(
                "node subtree contains {classification} content at {}",
                boundary.relative_path
            )));
        }
        Ok(())
    }
}

fn require_managed_destination(destination: &Path) -> Result<(), WorkspaceTransactionError> {
    validate_managed_node_path(destination)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))
}

fn generation_document_file_name(
    generation: WorkspaceDocumentGeneration,
    name: &str,
) -> Result<String, WorkspaceTransactionError> {
    canonical_document_file_name_for(generation, name)
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)
}

fn generation_document_locator(
    generation: WorkspaceDocumentGeneration,
    relative_node: &str,
    name: &str,
) -> Result<String, WorkspaceTransactionError> {
    canonical_document_locator_for(generation, relative_node, name)
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)
}

fn generation_document_path(
    generation: WorkspaceDocumentGeneration,
    directory: &Path,
    name: &str,
) -> Result<PathBuf, WorkspaceTransactionError> {
    canonical_document_path_for(generation, directory, name)
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)
}

fn planned_destination_parent_node_id(
    context: &PlanningContext,
    destination: &str,
) -> Result<NodeId, WorkspaceTransactionError> {
    let destination_path = context.root.join(Path::new(destination));
    let parent_path = destination_path
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(destination_path.clone()))?;
    context
        .inventory
        .nodes
        .iter()
        .find(|node| node.path == parent_path)
        .and_then(|node| node.id)
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)
}

/// Plans creation of one child node without changing the workspace.
///
/// # Errors
///
/// Returns an error for invalid names, parents, workspace authority, or pending recovery.
pub fn plan_create_child_node(
    root: impl AsRef<Path>,
    parent_id: NodeId,
    name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_node_name(name, false).map_err(WorkspaceTransactionError::Workspace)?;
    let context = PlanningContext::load(root.as_ref())?;
    let parent = context.node(parent_id)?;
    reject_trash_parent(&context, parent)?;
    let destination = parent.path.join(name);
    require_managed_destination(&destination)?;
    require_destination_available(&destination, None)?;
    let id = NodeId::new_v4();
    let relative = relative_string(&context.root, &destination)?;
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Create,
        vec![WorkspacePathChange {
            source_node_id: None,
            node_id: id,
            old_path: None,
            new_path: relative.clone(),
        }],
        Vec::new(),
        vec![id],
        vec![PlannedStep::CreateTree {
            destination: relative,
            payload: single_node_payload(context.generation, name, id)?,
        }],
    );
    attach_node_target(&mut plan, parent_id);
    Ok(plan)
}

/// Plans one already validated import proposal as an exact recoverable tree creation.
///
/// The importer remains responsible for conversion and proposal validation. Core independently
/// rechecks the canonical node shape, identity, digests, destination, resources, content rules,
/// and workspace revision before freezing the bytes in its transaction journal.
///
/// # Errors
///
/// Returns an error for stale authority, a non-canonical proposal, an unavailable destination,
/// unsafe resources, content-boundary conflicts, or pending recovery.
#[allow(clippy::too_many_lines)]
pub fn plan_import_node(
    root: impl AsRef<Path>,
    expected_workspace_revision: &WorkspaceRevision,
    authority: WorkspaceImportAuthority,
    node: WorkspaceImportNode,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_import_authority(&authority)?;
    let context = PlanningContext::load(root.as_ref())?;
    require_workspace_revision(expected_workspace_revision, &context.revision)?;
    if context.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        return Err(WorkspaceTransactionError::InvalidWorkspace);
    }

    let locator_path = Path::new(&node.locator);
    let normalized_locator = components_string(locator_path, locator_path)?;
    if normalized_locator != node.locator || node.locator.len() > 512 {
        return Err(WorkspaceTransactionError::Metadata(
            "import node locator is not a canonical portable relative path".to_owned(),
        ));
    }
    for component in node.locator.split('/') {
        if component.len() > 120 {
            return Err(WorkspaceTransactionError::Metadata(
                "import node locator component exceeds 120 UTF-8 bytes".to_owned(),
            ));
        }
        validate_node_name(component, false).map_err(WorkspaceTransactionError::Workspace)?;
    }
    let destination = context.root.join(locator_path);
    require_managed_destination(&destination)?;
    require_destination_available(&destination, None)?;
    let parent_path = destination
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(destination.clone()))?;
    let parent = context
        .inventory
        .nodes
        .iter()
        .find(|candidate| candidate.path == parent_path && candidate.id.is_some())
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
    reject_trash_parent(&context, parent)?;
    let parent_id = parent
        .id
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;

    let name = destination
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(destination.clone()))?;
    let expected_document_file = generation_document_file_name(context.generation, name)?;
    if node.document_file != expected_document_file {
        return Err(WorkspaceTransactionError::Metadata(
            "import document does not use the exact X/X.adoc shape".to_owned(),
        ));
    }
    validate_import_digest(&node.document_sha256, "import document digest")?;
    if node.document_sha256 != digest_bytes(node.exact_source.as_bytes()) {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "import document digest differs from its exact source".to_owned(),
        ));
    }
    if node.exact_source.len() > 32 * 1024 * 1024 {
        return Err(WorkspaceTransactionError::Metadata(
            "import document exceeds the Core 32 MiB safety ceiling".to_owned(),
        ));
    }
    let metadata = crate::parse_node_metadata(&node.exact_source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if metadata.id != Some(node.node_id) {
        return Err(WorkspaceTransactionError::Metadata(
            "import document identity differs from its proposal".to_owned(),
        ));
    }
    if metadata.presentation.adjacent_heading_body_explicit {
        return Err(WorkspaceTransactionError::Metadata(
            "adjacent_heading_body is valid only on the existing workspace root".to_owned(),
        ));
    }
    let profile = weftext_asciidoc::analyze(&node.exact_source);
    if profile.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            weftext_asciidoc::DiagnosticCode::UnclosedFrontmatter
                | weftext_asciidoc::DiagnosticCode::ParserError
                | weftext_asciidoc::DiagnosticCode::UnsupportedProfileSyntax
        )
    }) {
        return Err(WorkspaceTransactionError::Metadata(
            "import document does not satisfy the canonical AsciiDoc Profile".to_owned(),
        ));
    }

    if node.resources.len() > 5_000 {
        return Err(WorkspaceTransactionError::Metadata(
            "import resource count exceeds the Core safety ceiling".to_owned(),
        ));
    }
    let mut files = vec![TreeFile {
        path: node.document_file.clone(),
        bytes: node.exact_source.into_bytes(),
    }];
    let mut file_locators = BTreeSet::from([node.document_file.to_ascii_lowercase()]);
    let mut directories = BTreeSet::new();
    let mut total_bytes = u64::try_from(files[0].bytes.len()).unwrap_or(u64::MAX);
    for resource in node.resources {
        validate_import_resource_locator(&resource.locator, &node.document_file)?;
        validate_import_digest(&resource.sha256, "import resource digest")?;
        if resource.sha256 != digest_bytes(&resource.bytes) {
            return Err(WorkspaceTransactionError::VerificationFailed(format!(
                "import resource digest differs from bytes: {}",
                resource.locator
            )));
        }
        if resource.bytes.len() > 64 * 1024 * 1024 {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "import resource exceeds the Core 64 MiB safety ceiling: {}",
                resource.locator
            )));
        }
        total_bytes = total_bytes
            .checked_add(u64::try_from(resource.bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "import output byte count overflowed".to_owned(),
                )
            })?;
        if total_bytes > 512 * 1024 * 1024 {
            return Err(WorkspaceTransactionError::Metadata(
                "import output exceeds the Core 512 MiB safety ceiling".to_owned(),
            ));
        }
        if !file_locators.insert(resource.locator.to_ascii_lowercase()) {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "duplicate import resource locator: {}",
                resource.locator
            )));
        }
        let resource_path = Path::new(&resource.locator);
        let mut parent = resource_path.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            directories.insert(components_string(directory, resource_path)?);
            parent = directory.parent();
        }
        validate_managed_file_path(&context.root, &destination.join(resource_path))
            .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
        files.push(TreeFile {
            path: resource.locator,
            bytes: resource.bytes,
        });
    }
    if file_locators
        .iter()
        .any(|locator| directories.contains(locator))
    {
        return Err(WorkspaceTransactionError::Metadata(
            "an import resource cannot be both a file and a directory".to_owned(),
        ));
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let payload = TreePayload {
        digest: payload_digest(&directories, &files),
        directories,
        files,
    };
    let relative = relative_string(&context.root, &destination)?;
    let node_id = node.node_id;
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Import,
        vec![WorkspacePathChange {
            source_node_id: None,
            node_id,
            old_path: None,
            new_path: relative.clone(),
        }],
        Vec::new(),
        vec![node_id],
        vec![PlannedStep::CreateTree {
            destination: relative,
            payload,
        }],
    );
    attach_node_target(&mut plan, parent_id);
    plan.import_authority = Some(authority);
    let latest =
        read_workspace_revision(&context.root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(expected_workspace_revision, &latest)?;
    Ok(plan)
}

/// Plans one connected multi-node import proposal as a single recoverable tree creation.
///
/// Every node identity and exact canonical document byte is supplied by the reviewed proposal.
/// Core independently validates the complete hierarchy, proposal authority, identities, digests,
/// resources, destination boundary, portable collisions, and workspace revision. All imported
/// identities are reported as generated by this import transaction.
///
/// # Errors
///
/// Returns an error for stale authority, a disconnected or non-canonical tree, conflicting
/// identities or paths, unsafe resources, content-boundary violations, or pending recovery.
pub fn plan_import_tree(
    root: impl AsRef<Path>,
    expected_workspace_revision: &WorkspaceRevision,
    authority: WorkspaceImportAuthority,
    nodes: Vec<WorkspaceImportNode>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_import_authority(&authority)?;
    let context = PlanningContext::load(root.as_ref())?;
    require_workspace_revision(expected_workspace_revision, &context.revision)?;
    if context.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        return Err(WorkspaceTransactionError::InvalidWorkspace);
    }
    let generated_node_ids = nodes.iter().map(|node| node.node_id).collect::<Vec<_>>();
    let exact_tree = nodes
        .into_iter()
        .map(|node| WorkspaceRestoreTreeNode {
            locator: node.locator,
            node_id: node.node_id,
            document_file: node.document_file,
            exact_source: node.exact_source,
            document_sha256: node.document_sha256,
            annotation_sidecar: None,
            resources: node.resources,
        })
        .collect();
    let (destination, mut path_changes, payload) =
        validate_snapshot_restore_tree(&context, exact_tree)?;
    let destination_parent_id = planned_destination_parent_node_id(&context, &destination)?;
    for change in &mut path_changes {
        change.source_node_id = None;
    }
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Import,
        path_changes,
        Vec::new(),
        generated_node_ids,
        vec![PlannedStep::CreateTree {
            destination,
            payload,
        }],
    );
    attach_node_target(&mut plan, destination_parent_id);
    plan.import_authority = Some(authority);
    let latest =
        read_workspace_revision(&context.root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(expected_workspace_revision, &latest)?;
    Ok(plan)
}

/// Plans one exact identity-preserving snapshot tree as a single recoverable `CreateTree` step.
///
/// The caller supplies reviewed source bytes and a digest-bound authority. Core independently
/// validates the complete destination hierarchy, canonical `X/X.adoc` shape, every preserved
/// UUID, document and resource digest, node-local annotation sidecar contract, content rules,
/// collisions, and the target workspace revision. No target byte is written while planning.
///
/// # Errors
///
/// Returns an error for stale authority, malformed or disconnected trees, duplicate identities,
/// unsafe paths, invalid documents/annotations/resources, target collisions, boundary crossings,
/// safety-limit violations, or pending recovery.
pub fn plan_restore_snapshot_tree(
    root: impl AsRef<Path>,
    expected_workspace_revision: &WorkspaceRevision,
    authority: WorkspaceImportAuthority,
    nodes: Vec<WorkspaceRestoreTreeNode>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_import_authority(&authority)?;
    let context = PlanningContext::load(root.as_ref())?;
    require_workspace_revision(expected_workspace_revision, &context.revision)?;
    if context.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        return Err(WorkspaceTransactionError::InvalidWorkspace);
    }
    let (destination, path_changes, payload) = validate_snapshot_restore_tree(&context, nodes)?;
    let destination_parent_id = planned_destination_parent_node_id(&context, &destination)?;
    let mut plan = finalize_plan(
        &context,
        StructuralAction::SnapshotRestore,
        path_changes,
        Vec::new(),
        Vec::new(),
        vec![PlannedStep::CreateTree {
            destination,
            payload,
        }],
    );
    attach_node_target(&mut plan, destination_parent_id);
    plan.import_authority = Some(authority);
    let latest =
        read_workspace_revision(&context.root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(expected_workspace_revision, &latest)?;
    Ok(plan)
}

/// Plans all missing fixed-path Chrono nodes for one date under the selected
/// Chrono root.
///
/// # Errors
///
/// Returns an error for an invalid workspace/root, Trash placement, path
/// conflicts, or when every requested period already exists.
#[allow(clippy::too_many_lines)]
pub fn plan_chrono_nodes(
    root: impl AsRef<Path>,
    chrono_root_id: NodeId,
    date: CalendarDate,
    enabled: &[ChronoPeriod],
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let chrono_root = context.node(chrono_root_id)?;
    reject_trash_parent(&context, chrono_root)?;
    let mut periods = vec![ChronoPeriod::Year];
    periods.extend(
        enabled
            .iter()
            .copied()
            .filter(|period| *period != ChronoPeriod::Year),
    );
    periods.sort_by_key(|period| match period {
        ChronoPeriod::Year => 0,
        ChronoPeriod::Quarter => 1,
        ChronoPeriod::Month => 2,
        ChronoPeriod::Week => 3,
        ChronoPeriod::Day => 4,
    });
    periods.dedup();
    let chrono = ChronoPlan::build(date, &periods);
    let year = chrono
        .nodes
        .iter()
        .find(|node| node.period == ChronoPeriod::Year)
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
    let year_path = chrono_root.path.join(&year.name);
    let mut changes = Vec::new();
    let mut generated = Vec::new();
    let mut steps = Vec::new();

    if year_path.exists() {
        let existing_year = context
            .inventory
            .nodes
            .iter()
            .find(|node| node.path == year_path)
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        if existing_year.parent_id != Some(chrono_root_id) {
            return Err(WorkspaceTransactionError::InvalidWorkspace);
        }
        for period in chrono
            .nodes
            .iter()
            .filter(|node| node.period != ChronoPeriod::Year)
        {
            let destination = chrono_root.path.join(&period.relative_path);
            require_managed_destination(&destination)?;
            if destination.exists() {
                if !context
                    .inventory
                    .nodes
                    .iter()
                    .any(|node| node.path == destination)
                {
                    return Err(WorkspaceTransactionError::DestinationExists(destination));
                }
                continue;
            }
            let id = NodeId::new_v4();
            let relative = relative_string(&context.root, &destination)?;
            changes.push(WorkspacePathChange {
                source_node_id: None,
                node_id: id,
                old_path: None,
                new_path: relative.clone(),
            });
            generated.push(id);
            steps.push(PlannedStep::CreateTree {
                destination: relative,
                payload: single_node_payload(context.generation, &period.name, id)?,
            });
        }
    } else {
        let mut directories = Vec::new();
        let mut files = Vec::new();
        for period in &chrono.nodes {
            let id = NodeId::new_v4();
            generated.push(id);
            let destination = chrono_root.path.join(&period.relative_path);
            require_managed_destination(&destination)?;
            changes.push(WorkspacePathChange {
                source_node_id: None,
                node_id: id,
                old_path: None,
                new_path: relative_string(&context.root, &destination)?,
            });
            if period.period == ChronoPeriod::Year {
                files.push(TreeFile {
                    path: generation_document_file_name(context.generation, &period.name)?,
                    bytes: new_node_document(id).into_bytes(),
                });
            } else {
                directories.push(period.name.clone());
                files.push(TreeFile {
                    path: generation_document_locator(
                        context.generation,
                        &period.name,
                        &period.name,
                    )?,
                    bytes: new_node_document(id).into_bytes(),
                });
            }
        }
        directories.sort();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        let digest = payload_digest(&directories, &files);
        steps.push(PlannedStep::CreateTree {
            destination: relative_string(&context.root, &year_path)?,
            payload: TreePayload {
                directories,
                files,
                digest,
            },
        });
    }
    if steps.is_empty() {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Chrono,
        changes,
        Vec::new(),
        generated,
        steps,
    );
    attach_node_target(&mut plan, chrono_root_id);
    Ok(plan)
}

/// Plans an identity-preserving move and the required unambiguous link rewrites.
///
/// # Errors
///
/// Returns an error for invalid destinations, ambiguous affected links, or pending recovery.
pub fn plan_move_node(
    root: impl AsRef<Path>,
    node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_node_name(destination_name, false).map_err(WorkspaceTransactionError::Workspace)?;
    let context = PlanningContext::load(root.as_ref())?;
    let node = context.node(node_id)?;
    let parent = context.node(destination_parent_id)?;
    reject_root_or_trash(&context, node)?;
    reject_trash_parent(&context, parent)?;
    if destination_name != node.name {
        return Err(WorkspaceTransactionError::Metadata(
            "move preserves the branch name; use the dedicated rename action first".to_owned(),
        ));
    }
    build_move_plan(
        &context,
        StructuralAction::Move,
        node,
        parent,
        destination_name,
        true,
        Vec::new(),
        Vec::new(),
    )
}

/// Plans an identity-preserving rename within the node's existing parent.
///
/// Rename is a distinct semantic action and cannot change the parent. A move
/// likewise preserves the current branch name.
///
/// # Errors
///
/// Returns an error for root/Trash mutation, invalid names, destination
/// collisions, ambiguous affected links, or pending recovery.
pub fn plan_rename_node(
    root: impl AsRef<Path>,
    node_id: NodeId,
    destination_name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_node_name(destination_name, false).map_err(WorkspaceTransactionError::Workspace)?;
    let context = PlanningContext::load(root.as_ref())?;
    let node = context.node(node_id)?;
    reject_root_or_trash(&context, node)?;
    let parent_id = node
        .parent_id
        .ok_or(WorkspaceTransactionError::RootMutationUnsupported)?;
    let parent = context.node(parent_id)?;
    reject_trash_parent(&context, parent)?;
    build_move_plan(
        &context,
        StructuralAction::Rename,
        node,
        parent,
        destination_name,
        true,
        Vec::new(),
        Vec::new(),
    )
}

/// Plans a recursive copy with fresh identities for every copied node.
///
/// # Errors
///
/// Returns an error for invalid destinations, unreadable content, or pending recovery.
pub fn plan_copy_node(
    root: impl AsRef<Path>,
    node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    validate_node_name(destination_name, false).map_err(WorkspaceTransactionError::Workspace)?;
    let context = PlanningContext::load(root.as_ref())?;
    let node = context.node(node_id)?;
    let parent = context.node(destination_parent_id)?;
    reject_root_or_trash(&context, node)?;
    reject_trash_parent(&context, parent)?;
    if parent.path.starts_with(&node.path) {
        return Err(WorkspaceTransactionError::MoveIntoDescendant);
    }
    let destination = parent.path.join(destination_name);
    require_managed_destination(&destination)?;
    context.require_managed_subtree(node)?;
    require_destination_available(&destination, None)?;
    let subtree = subtree_nodes(&context, &node.path);
    let mut replacements = BTreeMap::new();
    let mut generated = Vec::new();
    for source in &subtree {
        let source_id = source
            .id
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        let copied_id = NodeId::new_v4();
        replacements.insert(source_id, copied_id);
        generated.push(copied_id);
    }
    let copied_documents =
        copy_document_sources(&context, node, &destination, &subtree, &replacements)?;
    let payload = copy_subtree_payload(
        context.generation,
        node,
        destination_name,
        &subtree,
        &copied_documents,
        &replacements,
    )?;
    let destination_relative = relative_string(&context.root, &destination)?;
    let mut changes = Vec::new();
    for source in subtree {
        let source_id = source
            .id
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        let inside = source
            .path
            .strip_prefix(&node.path)
            .map_err(|_| WorkspaceTransactionError::PathEscape(source.path.clone()))?;
        changes.push(WorkspacePathChange {
            source_node_id: Some(source_id),
            node_id: replacements[&source_id],
            old_path: Some(relative_string(&context.root, &source.path)?),
            new_path: relative_string(&context.root, &destination.join(inside))?,
        });
    }
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Copy,
        changes,
        Vec::new(),
        generated,
        vec![PlannedStep::CreateTree {
            destination: destination_relative,
            payload,
        }],
    );
    attach_active_branch_scope(
        &mut plan,
        &context,
        node_id,
        WorkspaceIdentityPolicy::Rekey,
        [destination_parent_id],
        0,
        None,
    )?;
    Ok(plan)
}

/// Plans one complete managed subtree as a no-clobber Workspace Trash item.
///
/// # Errors
///
/// Returns an error for root/Trash mutations, boundary conflicts, invalid authority, or pending
/// recovery. The current UTC timestamp is captured once and bound into the returned plan.
pub fn plan_trash_node(
    root: impl AsRef<Path>,
    node_id: NodeId,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_trash_node_at(root, node_id, &current_utc_timestamp()?)
}

/// Plans one complete managed subtree with an exact reviewed explicit-offset timestamp.
///
/// # Errors
///
/// Returns the same errors as [`plan_trash_node`], plus invalid timestamp evidence.
pub fn plan_trash_node_at(
    root: impl AsRef<Path>,
    node_id: NodeId,
    trashed_at: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_trash_node_at_with_ids(root.as_ref(), node_id, trashed_at, None)
}

fn plan_trash_node_at_with_ids(
    root: &Path,
    node_id: NodeId,
    trashed_at: &str,
    reviewed_ids: Option<(crate::TrashItemId, crate::TrashOperationId, Option<NodeId>)>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root)?;
    let node = context.node(node_id)?;
    reject_root_or_trash(&context, node)?;
    context.require_managed_subtree(node)?;
    let (trash_item_id, operation_id, reviewed_trash_node_id) = reviewed_ids.unwrap_or_else(|| {
        (
            crate::TrashItemId::new_v4(),
            crate::TrashOperationId::new_v4(),
            None,
        )
    });
    let summary = crate::workspace_trash::trash_payload_summary(&node.path)
        .map_err(|issue| WorkspaceTransactionError::TrashReconciliation(issue.message))?;
    let parent_id = node
        .parent_id
        .ok_or(WorkspaceTransactionError::RootMutationUnsupported)?;
    let manifest = crate::TrashItemManifest::new_node(
        trash_item_id,
        operation_id,
        trashed_at.to_owned(),
        node_id,
        Some(parent_id),
        node.name.clone(),
        ancestor_node_ids(&context, parent_id)?,
        &summary,
    )
    .map_err(WorkspaceTransactionError::Metadata)?;
    let item_payload = node_trash_item_payload(node, &manifest)?;
    let (create_step, generated) = plan_trash_item_creation(
        &context,
        trash_item_id,
        item_payload,
        reviewed_trash_node_id,
    )?;
    let source_digest = tree_digest(&node.path)?;
    let item_root = trash_item_relative_path(trash_item_id);
    let payload_node = format!(
        "{item_root}/{}/{name}",
        crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME,
        name = node.name
    );
    let path_changes = subtree_nodes(&context, &node.path)
        .into_iter()
        .map(|record| {
            let id = record
                .id
                .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
            let inside = record
                .path
                .strip_prefix(&node.path)
                .map_err(|_| WorkspaceTransactionError::PathEscape(record.path.clone()))?;
            let suffix = components_string_allow_empty(inside, &record.path)?;
            Ok(WorkspacePathChange {
                source_node_id: Some(id),
                node_id: id,
                old_path: Some(relative_string(&context.root, &record.path)?),
                new_path: if suffix.is_empty() {
                    payload_node.clone()
                } else {
                    format!("{payload_node}/{suffix}")
                },
            })
        })
        .collect::<Result<Vec<_>, WorkspaceTransactionError>>()?;
    let trash_node_id = generated.first().copied();
    finalize_reviewed_trash_plan(
        &context,
        StructuralAction::Trash,
        path_changes,
        generated,
        vec![
            create_step,
            PlannedStep::RemovePath {
                source: relative_string(&context.root, &node.path)?,
                digest: source_digest,
            },
        ],
        vec![crate::WorkspaceTrashPlanItemChange {
            disposition: crate::TrashPlanDisposition::Stored,
            manifest,
            destination_node_id: None,
            destination_name: None,
        }],
        crate::TrashReviewedAction::StoreNode {
            node_id,
            trashed_at: trashed_at.to_owned(),
            trash_item_id,
            operation_id,
            trash_node_id,
        },
    )
}

/// Plans one node-owned regular resource as an independently recoverable Trash item.
///
/// # Errors
///
/// Returns an error for unmanaged/ignored/reserved/non-regular files, ownership mismatch, invalid
/// authority, or pending recovery.
pub fn plan_trash_resource(
    root: impl AsRef<Path>,
    owner_node_id: NodeId,
    name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_trash_resources_at(
        root,
        vec![crate::TrashResourceSelection {
            owner_node_id,
            name: name.to_owned(),
        }],
        &current_utc_timestamp()?,
    )
}

/// Plans a batch of resources as separate items sharing one generated operation ID.
///
/// # Errors
///
/// Returns an error before producing a plan unless every selected resource is valid.
pub fn plan_trash_resources(
    root: impl AsRef<Path>,
    resources: Vec<crate::TrashResourceSelection>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_trash_resources_at(root, resources, &current_utc_timestamp()?)
}

/// Plans a resource batch with one exact explicit-offset timestamp.
///
/// # Errors
///
/// Returns the same errors as [`plan_trash_resources`], plus invalid timestamp evidence.
pub fn plan_trash_resources_at(
    root: impl AsRef<Path>,
    resources: Vec<crate::TrashResourceSelection>,
    trashed_at: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_trash_resources_at_with_ids(root.as_ref(), resources, trashed_at, None)
}

#[allow(clippy::too_many_lines)]
fn plan_trash_resources_at_with_ids(
    root: &Path,
    resources: Vec<crate::TrashResourceSelection>,
    trashed_at: &str,
    reviewed_ids: Option<(
        Vec<crate::TrashItemId>,
        crate::TrashOperationId,
        Option<NodeId>,
    )>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    if resources.is_empty() || resources.len() > 10_000 {
        return Err(WorkspaceTransactionError::Metadata(
            "resource Trash batch must contain 1..=10000 files".to_owned(),
        ));
    }
    let context = PlanningContext::load(root)?;
    let action_resources = resources.clone();
    let (trash_item_ids, operation_id, reviewed_trash_node_id) =
        if let Some(reviewed) = reviewed_ids {
            if reviewed.0.len() != resources.len() {
                return Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
                    "reviewed resource item ID count differs from the selection".to_owned(),
                ));
            }
            reviewed
        } else {
            (
                (0..resources.len())
                    .map(|_| crate::TrashItemId::new_v4())
                    .collect(),
                crate::TrashOperationId::new_v4(),
                None,
            )
        };
    let mut selected = BTreeSet::new();
    let mut payloads = Vec::new();
    let mut sources = Vec::new();
    let mut item_changes = Vec::new();
    for (position, selection) in resources.into_iter().enumerate() {
        validate_restore_name(crate::TrashItemKind::Resource, &selection.name)?;
        let owner = context.node(selection.owner_node_id)?;
        reject_trash_parent(&context, owner)?;
        if selection
            .name
            .eq_ignore_ascii_case(&generation_document_file_name(
                context.generation,
                &owner.name,
            )?)
        {
            return Err(WorkspaceTransactionError::ContentBoundary(
                "managed canonical documents cannot enter resource Trash".to_owned(),
            ));
        }
        let key = (selection.owner_node_id, selection.name.to_lowercase());
        if !selected.insert(key) {
            return Err(WorkspaceTransactionError::Metadata(
                "resource Trash batch contains duplicate or case-fold-colliding selections"
                    .to_owned(),
            ));
        }
        let path = owner.path.join(&selection.name);
        validate_managed_file_path(&context.root, &path)
            .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
        let relative = relative_string(&context.root, &path)?;
        let inventory_entry = context.inventory.content.iter().find(|entry| {
            entry.kind == crate::WorkspaceContentKind::Resource
                && entry.relative_path == relative
                && entry.owner_node_id == Some(selection.owner_node_id)
        });
        if inventory_entry.is_none() {
            return Err(WorkspaceTransactionError::ContentBoundary(format!(
                "selected file is not a regular resource owned by node {}",
                selection.owner_node_id
            )));
        }
        let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
        if linked_or_reparse(&metadata) || !metadata.is_file() {
            return Err(WorkspaceTransactionError::SymlinkUnsupported(path));
        }
        if metadata.len() > MAX_SNAPSHOT_RESTORE_BYTES {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "resource Trash payload exceeds {MAX_SNAPSHOT_RESTORE_BYTES} bytes"
            )));
        }
        let bytes = fs::read(&path).map_err(WorkspaceTransactionError::Io)?;
        let trash_item_id = trash_item_ids[position];
        let manifest = crate::TrashItemManifest::new_resource(
            trash_item_id,
            operation_id,
            trashed_at.to_owned(),
            Some(selection.owner_node_id),
            selection.name.clone(),
            &bytes,
        )
        .map_err(WorkspaceTransactionError::Metadata)?;
        payloads.push((
            trash_item_id,
            resource_trash_item_payload(&selection.name, bytes, &manifest)?,
        ));
        sources.push((relative, file_digest(&path)?));
        item_changes.push(crate::WorkspaceTrashPlanItemChange {
            disposition: crate::TrashPlanDisposition::Stored,
            manifest,
            destination_node_id: None,
            destination_name: None,
        });
    }
    let (mut steps, generated) =
        plan_trash_item_creations(&context, payloads, reviewed_trash_node_id)?;
    steps.extend(
        sources
            .into_iter()
            .map(|(source, digest)| PlannedStep::RemovePath { source, digest }),
    );
    let trash_node_id = generated.first().copied();
    finalize_reviewed_trash_plan(
        &context,
        StructuralAction::Trash,
        Vec::new(),
        generated,
        steps,
        item_changes,
        crate::TrashReviewedAction::StoreResources {
            resources: action_resources,
            trashed_at: trashed_at.to_owned(),
            trash_item_ids,
            operation_id,
            trash_node_id,
        },
    )
}

/// Creates and verifies an exact disjoint snapshot required before legacy Trash migration.
///
/// # Errors
///
/// Returns an error for unsafe legacy authority, an invalid/disjoint destination, collisions,
/// links/reparse points, unreadable bytes, or an unfinished transaction.
pub fn prepare_legacy_trash_migration_backup(
    root: impl AsRef<Path>,
    snapshot_parent: impl AsRef<Path>,
) -> Result<crate::LegacyTrashMigrationBackup, WorkspaceTransactionError> {
    let root = root.as_ref();
    let snapshot_parent = snapshot_parent.as_ref();
    let lease = acquire_workspace_transaction_lease(root)?;
    let context = PlanningContext::load_legacy_trash(root)?;
    reject_linked_existing_ancestors(snapshot_parent)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let parent_metadata =
        fs::symlink_metadata(snapshot_parent).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&parent_metadata) || !parent_metadata.is_dir() {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "snapshot parent must be an existing regular non-link directory".to_owned(),
            ),
        );
    }
    let canonical_root = fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?;
    let canonical_parent =
        fs::canonicalize(snapshot_parent).map_err(WorkspaceTransactionError::Io)?;
    if canonical_parent.starts_with(&canonical_root) {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot must be outside the workspace".to_owned(),
            ),
        );
    }
    let backup_id = crate::TrashReviewId::new_v4();
    let snapshot_directory =
        canonical_parent.join(format!("weftext-legacy-trash-migration-{backup_id}"));
    require_portable_destination_available(&snapshot_directory)?;
    let trash_path = context.root.join(TRASH_NODE_NAME);
    let payload = collect_existing_tree_payload(&trash_path)?;
    let physical_entries = u64::try_from(
        payload
            .directories
            .len()
            .saturating_add(payload.files.len())
            .saturating_add(1),
    )
    .unwrap_or(u64::MAX);
    let physical_bytes = payload.files.iter().try_fold(0_u64, |total, file| {
        total
            .checked_add(u64::try_from(file.bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                    "legacy Trash snapshot byte count overflowed".to_owned(),
                )
            })
    })?;
    let mut authority = crate::LegacyTrashMigrationBackupAuthority {
        schema: crate::LEGACY_TRASH_MIGRATION_BACKUP_SCHEMA.to_owned(),
        backup_id,
        workspace_root_sha256: crate::workspace_trash::trash_reviewed_workspace_root_digest(root)
            .map_err(WorkspaceTransactionError::Io)?,
        base_revision: context.revision.clone(),
        trash_tree_sha256: payload.digest.clone(),
        physical_entries,
        physical_bytes,
        authority_digest: "0".repeat(64),
    };
    authority.authority_digest =
        crate::workspace_trash::legacy_trash_backup_authority_digest(&authority)
            .map_err(WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup)?;
    authority
        .validate()
        .map_err(WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup)?;

    fs::create_dir(&snapshot_directory).map_err(WorkspaceTransactionError::Io)?;
    let creation = (|| {
        let snapshot_trash = snapshot_directory
            .join(LEGACY_TRASH_BACKUP_CONTENT_DIRECTORY)
            .join(TRASH_NODE_NAME);
        materialize_payload(&snapshot_trash, &payload)?;
        let mut manifest_bytes =
            serde_json::to_vec(&authority).map_err(WorkspaceTransactionError::Json)?;
        manifest_bytes.push(b'\n');
        write_verified_file(
            &snapshot_directory.join(LEGACY_TRASH_BACKUP_MANIFEST_FILE),
            &manifest_bytes,
            &digest_bytes(&manifest_bytes),
        )?;
        sync_directory(&snapshot_directory)?;
        sync_directory(&canonical_parent)?;
        Ok::<(), WorkspaceTransactionError>(())
    })();
    if let Err(error) = creation {
        let _ = fs::remove_dir_all(&snapshot_directory);
        return Err(error);
    }
    let snapshot_directory =
        fs::canonicalize(&snapshot_directory).map_err(WorkspaceTransactionError::Io)?;
    let backup = crate::LegacyTrashMigrationBackup {
        canonical_workspace_root: canonical_root,
        snapshot_directory,
        authority,
    };
    verify_legacy_trash_migration_backup(root, &backup, &context.revision)?;
    drop(lease);
    Ok(backup)
}

/// Reopens a Core-created external legacy-Trash snapshot as opaque verified evidence.
/// This is the cross-process counterpart to [`prepare_legacy_trash_migration_backup`].
///
/// # Errors
///
/// Returns an error when the directory/manifest belongs to another workspace root, is malformed,
/// or any snapshotted byte, entry count, or digest differs from its authority. Reopening is
/// intentionally state-independent so the exact snapshot remains verifiable after migration.
pub fn load_legacy_trash_migration_backup(
    root: impl AsRef<Path>,
    snapshot_directory: impl AsRef<Path>,
) -> Result<crate::LegacyTrashMigrationBackup, WorkspaceTransactionError> {
    let root = root.as_ref();
    let snapshot_directory = snapshot_directory.as_ref();
    reject_linked_existing_ancestors(snapshot_directory).map_err(|error| {
        WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(error.to_string())
    })?;
    let snapshot_directory =
        fs::canonicalize(snapshot_directory).map_err(WorkspaceTransactionError::Io)?;
    let manifest_path = snapshot_directory.join(LEGACY_TRASH_BACKUP_MANIFEST_FILE);
    let bytes = read_bounded_regular_file(&manifest_path, MAX_ROLLBACK_MARKER_BYTES)?;
    reject_duplicate_json_keys(&bytes).map_err(|error| {
        WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(error.to_string())
    })?;
    let authority: crate::LegacyTrashMigrationBackupAuthority =
        serde_json::from_slice(&bytes).map_err(WorkspaceTransactionError::Json)?;
    let backup = crate::LegacyTrashMigrationBackup {
        canonical_workspace_root: fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?,
        snapshot_directory,
        authority,
    };
    verify_legacy_trash_migration_backup_snapshot(root, &backup)?;
    Ok(backup)
}

fn verify_legacy_trash_migration_backup(
    root: &Path,
    backup: &crate::LegacyTrashMigrationBackup,
    expected_revision: &WorkspaceRevision,
) -> Result<(), WorkspaceTransactionError> {
    verify_legacy_trash_migration_backup_snapshot(root, backup)?;
    if backup.authority.base_revision != *expected_revision
        || tree_digest(&root.join(TRASH_NODE_NAME))? != backup.authority.trash_tree_sha256
    {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash backup differs from the current migration source".to_owned(),
            ),
        );
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn verify_legacy_trash_migration_backup_snapshot(
    root: &Path,
    backup: &crate::LegacyTrashMigrationBackup,
) -> Result<(), WorkspaceTransactionError> {
    backup
        .authority
        .validate()
        .map_err(WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup)?;
    let canonical_root = fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?;
    if canonical_root != backup.canonical_workspace_root
        || backup.snapshot_directory.starts_with(&canonical_root)
        || canonical_root.starts_with(&backup.snapshot_directory)
        || backup.authority.workspace_root_sha256
            != crate::workspace_trash::trash_reviewed_workspace_root_digest(root)
                .map_err(WorkspaceTransactionError::Io)?
    {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash backup belongs to another workspace or revision".to_owned(),
            ),
        );
    }
    reject_linked_existing_ancestors(&backup.snapshot_directory).map_err(|error| {
        WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(error.to_string())
    })?;
    let snapshot_metadata =
        fs::symlink_metadata(&backup.snapshot_directory).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&snapshot_metadata) || !snapshot_metadata.is_dir() {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot directory is not regular".to_owned(),
            ),
        );
    }
    let mut root_entries = fs::read_dir(&backup.snapshot_directory)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?;
    root_entries.sort_by_key(std::fs::DirEntry::file_name);
    let names = root_entries
        .iter()
        .map(|entry| entry.file_name().into_string().map_err(|_| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::NonUtf8Path)?;
    if names
        != [
            LEGACY_TRASH_BACKUP_MANIFEST_FILE.to_owned(),
            LEGACY_TRASH_BACKUP_CONTENT_DIRECTORY.to_owned(),
        ]
    {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot has extra or missing root entries".to_owned(),
            ),
        );
    }
    let manifest_path = backup
        .snapshot_directory
        .join(LEGACY_TRASH_BACKUP_MANIFEST_FILE);
    let manifest_bytes = read_bounded_regular_file(&manifest_path, MAX_ROLLBACK_MARKER_BYTES)?;
    reject_duplicate_json_keys(&manifest_bytes).map_err(|error| {
        WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(error.to_string())
    })?;
    let manifest: crate::LegacyTrashMigrationBackupAuthority =
        serde_json::from_slice(&manifest_bytes).map_err(WorkspaceTransactionError::Json)?;
    if manifest != backup.authority {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot manifest differs from its opaque evidence".to_owned(),
            ),
        );
    }
    let content = backup
        .snapshot_directory
        .join(LEGACY_TRASH_BACKUP_CONTENT_DIRECTORY);
    let mut content_entries = fs::read_dir(&content)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?;
    if content_entries.len() != 1
        || content_entries
            .pop()
            .and_then(|entry| entry.file_name().into_string().ok())
            .as_deref()
            != Some(TRASH_NODE_NAME)
    {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot content is not one exact Trash tree".to_owned(),
            ),
        );
    }
    let snapshot_payload = collect_existing_tree_payload(&content.join(TRASH_NODE_NAME))?;
    let entries = u64::try_from(
        snapshot_payload
            .directories
            .len()
            .saturating_add(snapshot_payload.files.len())
            .saturating_add(1),
    )
    .unwrap_or(u64::MAX);
    let bytes = snapshot_payload
        .files
        .iter()
        .try_fold(0_u64, |total, file| {
            total.checked_add(u64::try_from(file.bytes.len()).unwrap_or(u64::MAX))
        });
    if snapshot_payload.digest != backup.authority.trash_tree_sha256
        || entries != backup.authority.physical_entries
        || bytes != Some(backup.authority.physical_bytes)
    {
        return Err(
            WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                "legacy Trash snapshot bytes differ from their verified authority".to_owned(),
            ),
        );
    }
    Ok(())
}

/// Plans explicit conversion of the historical direct-child Trash layout into v1 items.
///
/// Every legacy direct entry becomes one origin-unknown item. No basename or historical path is
/// promoted into parent/owner authority.
///
/// # Errors
///
/// Returns an error for mixed old/new authority, malformed or reserved entries, links, content
/// rules, duplicate permanent UUIDs, invalid timestamps, or pending recovery.
pub fn plan_migrate_legacy_workspace_trash(
    _root: impl AsRef<Path>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired)
}

/// Plans legacy Trash migration with one exact migration-time timestamp.
///
/// # Errors
///
/// Returns the same errors as [`plan_migrate_legacy_workspace_trash`].
pub fn plan_migrate_legacy_workspace_trash_at(
    _root: impl AsRef<Path>,
    _trashed_at: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired)
}

/// Plans legacy Trash migration only after Core has created a verified disjoint snapshot.
///
/// # Errors
///
/// Returns an error for stale, changed, unsafe, or mismatched workspace/snapshot authority.
pub fn plan_migrate_legacy_workspace_trash_with_backup(
    root: impl AsRef<Path>,
    backup: &crate::LegacyTrashMigrationBackup,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_migrate_legacy_workspace_trash_at_with_backup(root, &current_utc_timestamp()?, backup)
}

/// Plans externally backed legacy Trash migration with an exact reviewed timestamp.
///
/// # Errors
///
/// Returns an error for an invalid timestamp or stale, unsafe, or mismatched authority.
pub fn plan_migrate_legacy_workspace_trash_at_with_backup(
    root: impl AsRef<Path>,
    trashed_at: &str,
    backup: &crate::LegacyTrashMigrationBackup,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_migrate_legacy_workspace_trash_at_with_ids(root.as_ref(), trashed_at, backup, None)
}

#[allow(clippy::too_many_lines)]
fn plan_migrate_legacy_workspace_trash_at_with_ids(
    root: &Path,
    trashed_at: &str,
    backup: &crate::LegacyTrashMigrationBackup,
    reviewed_ids: Option<(Vec<crate::TrashItemId>, crate::TrashOperationId)>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load_legacy_trash(root)?;
    verify_legacy_trash_migration_backup(root, backup, &context.revision)?;
    let trash = context.root.join(TRASH_NODE_NAME);
    let canonical_document = generation_document_path(context.generation, &trash, TRASH_NODE_NAME)?;
    let rules = crate::content_boundary::ContentRules::load(&context.root)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let (reviewed_item_ids, operation_id) = reviewed_ids.map_or_else(
        || (None, crate::TrashOperationId::new_v4()),
        |(ids, operation)| (Some(ids.into_iter()), operation),
    );
    let mut reviewed_item_ids = reviewed_item_ids;
    let mut seen_node_ids = context
        .inventory
        .nodes
        .iter()
        .filter_map(|node| node.id.map(|id| (id, node.path.clone())))
        .collect::<BTreeMap<_, _>>();
    let mut entries = fs::read_dir(&trash)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    let mut payloads = Vec::new();
    let mut sources = Vec::new();
    let mut changes = Vec::new();
    for entry in entries {
        let path = entry.path();
        if path == canonical_document {
            continue;
        }
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(path.clone()))?;
        reject_legacy_trash_rule_classification(&context.root, &rules, &path)?;
        let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(WorkspaceTransactionError::SymlinkUnsupported(path));
        }
        let trash_item_id = if let Some(ids) = &mut reviewed_item_ids {
            ids.next().ok_or_else(|| {
                WorkspaceTransactionError::InvalidTrashReviewedRequest(
                    "reviewed legacy migration has too few item IDs".to_owned(),
                )
            })?
        } else {
            crate::TrashItemId::new_v4()
        };
        let (manifest, payload) = if metadata.is_dir() {
            let (node_id, summary) =
                crate::workspace_trash::inspect_legacy_node_payload(&path, &mut seen_node_ids)
                    .map_err(|issue| {
                        WorkspaceTransactionError::TrashReconciliation(issue.message)
                    })?;
            let manifest = crate::TrashItemManifest::new_node(
                trash_item_id,
                operation_id,
                trashed_at.to_owned(),
                node_id,
                None,
                name.clone(),
                Vec::new(),
                &summary,
            )
            .map_err(WorkspaceTransactionError::Metadata)?;
            let node = crate::NodeRecord {
                id: Some(node_id),
                name: name.clone(),
                path: path.clone(),
                document_path: generation_document_path(context.generation, &path, &name)?,
                parent_id: None,
                metadata: None,
                metadata_diagnostics: Vec::new(),
            };
            let payload = node_trash_item_payload(&node, &manifest)?;
            (manifest, payload)
        } else if metadata.is_file() {
            validate_restore_name(crate::TrashItemKind::Resource, &name)?;
            let bytes = fs::read(&path).map_err(WorkspaceTransactionError::Io)?;
            let manifest = crate::TrashItemManifest::new_resource(
                trash_item_id,
                operation_id,
                trashed_at.to_owned(),
                None,
                name.clone(),
                &bytes,
            )
            .map_err(WorkspaceTransactionError::Metadata)?;
            let payload = resource_trash_item_payload(&name, bytes, &manifest)?;
            (manifest, payload)
        } else {
            return Err(WorkspaceTransactionError::ContentBoundary(
                "legacy Trash contains an unsupported filesystem entry".to_owned(),
            ));
        };
        payloads.push((trash_item_id, payload));
        sources.push((relative_string(&context.root, &path)?, tree_digest(&path)?));
        changes.push(crate::WorkspaceTrashPlanItemChange {
            disposition: crate::TrashPlanDisposition::Migrated,
            manifest,
            destination_node_id: None,
            destination_name: None,
        });
    }
    if payloads.is_empty() {
        return Err(WorkspaceTransactionError::NoChange);
    }
    if reviewed_item_ids
        .as_mut()
        .is_some_and(|ids| ids.next().is_some())
    {
        return Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
            "reviewed legacy migration has too many item IDs".to_owned(),
        ));
    }
    let (mut steps, generated) = plan_trash_item_creations(&context, payloads, None)?;
    steps.extend(
        sources
            .into_iter()
            .map(|(source, digest)| PlannedStep::RemovePath { source, digest }),
    );
    let trash_item_ids = changes
        .iter()
        .map(|change| change.manifest.trash_item_id())
        .collect();
    let mut plan = finalize_reviewed_trash_plan(
        &context,
        StructuralAction::TrashMigration,
        Vec::new(),
        generated,
        steps,
        changes,
        crate::TrashReviewedAction::MigrateLegacy {
            trashed_at: trashed_at.to_owned(),
            trash_item_ids,
            operation_id,
            backup: backup.authority.clone(),
        },
    )?;
    plan.legacy_trash_migration_backup = Some(backup.clone());
    Ok(plan)
}

fn reject_legacy_trash_rule_classification(
    root: &Path,
    rules: &crate::content_boundary::ContentRules,
    path: &Path,
) -> Result<(), WorkspaceTransactionError> {
    let metadata = fs::symlink_metadata(path).map_err(WorkspaceTransactionError::Io)?;
    let relative = crate::content_boundary::portable_path(root, path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    if rules.classify(&relative, metadata.is_dir()).is_some() {
        return Err(WorkspaceTransactionError::ContentBoundary(format!(
            "content rules classify legacy Trash authority at {relative}"
        )));
    }
    if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(WorkspaceTransactionError::Io)? {
            reject_legacy_trash_rule_classification(
                root,
                rules,
                &entry.map_err(WorkspaceTransactionError::Io)?.path(),
            )?;
        }
    }
    Ok(())
}

/// Plans an identity-preserving restore to an explicit parent and name.
///
/// # Errors
///
/// Returns an error when the node is not trashed or the destination is invalid.
pub fn plan_restore_node(
    root: impl AsRef<Path>,
    node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let mut matches = context
        .inventory
        .trash_items
        .iter()
        .filter(|item| item.manifest.node_id() == Some(node_id));
    let item = matches
        .next()
        .ok_or(WorkspaceTransactionError::NotInTrash(node_id))?;
    if matches.next().is_some() {
        return Err(WorkspaceTransactionError::TrashReconciliation(
            "multiple Trash items claim the selected root node identity".to_owned(),
        ));
    }
    let item_id = item.manifest.trash_item_id();
    drop(context);
    plan_restore_trash_item(
        root,
        item_id,
        crate::TrashRestoreMode::ExistingTarget {
            target_node_id: destination_parent_id,
            name: destination_name.to_owned(),
        },
    )
}

/// Plans an exact Trash-item restore using a Core-projected restore mode.
///
/// # Errors
///
/// Returns an error for unavailable origins/ancestor chains, unknown items, stale or malformed
/// authority, boundary conflicts, or exact/case-fold destination occupancy.
pub fn plan_restore_trash_item(
    root: impl AsRef<Path>,
    trash_item_id: crate::TrashItemId,
    mode: crate::TrashRestoreMode,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let item = context
        .inventory
        .trash_items
        .iter()
        .find(|item| item.manifest.trash_item_id() == trash_item_id)
        .ok_or(WorkspaceTransactionError::UnknownTrashItem(trash_item_id))?;
    let reviewed_mode = mode.clone();
    let resolved = match mode {
        crate::TrashRestoreMode::Original => {
            let resolved = resolve_restore_chain(&context, item)?;
            if resolved.len() != 1 {
                return Err(WorkspaceTransactionError::TrashRestoreUnavailable(
                    crate::TrashRestoreBlockedReason::OriginMissing,
                ));
            }
            resolved
        }
        crate::TrashRestoreMode::WithAncestors => {
            let resolved = resolve_restore_chain(&context, item)?;
            if resolved.len() <= 1 {
                return Err(WorkspaceTransactionError::TrashRestoreUnavailable(
                    crate::TrashRestoreBlockedReason::OriginMissing,
                ));
            }
            resolved
        }
        crate::TrashRestoreMode::ExistingTarget {
            target_node_id,
            name,
        } => vec![resolve_existing_target(
            &context,
            item,
            target_node_id,
            &name,
        )?],
    };
    build_restore_plan(
        &context,
        &resolved,
        crate::TrashReviewedAction::Restore {
            trash_item_id,
            mode: reviewed_mode,
        },
    )
}

/// Projects path-free Trash item evidence and Core-resolved restore availability.
///
/// # Errors
///
/// Returns an error when the workspace or any Trash item requires reconciliation or migration.
pub fn project_workspace_trash_items(
    root: impl AsRef<Path>,
) -> Result<Vec<crate::WorkspaceTrashItemProjection>, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let mut projections = Vec::new();
    for item in &context.inventory.trash_items {
        let origin_id = trash_manifest_origin(&item.manifest);
        let origin_resolution =
            if item.manifest.origin_status() == crate::TrashOriginStatus::Unknown {
                crate::TrashOriginResolution::Unknown
            } else if origin_id.is_some_and(|id| {
                context
                    .inventory
                    .nodes
                    .iter()
                    .any(|node| node.id == Some(id))
            }) {
                crate::TrashOriginResolution::Active
            } else if origin_id.is_some_and(|id| {
                context
                    .inventory
                    .trash_items
                    .iter()
                    .any(|candidate| candidate.node_locators.contains_key(&id))
            }) {
                crate::TrashOriginResolution::InTrash
            } else {
                crate::TrashOriginResolution::Missing
            };
        let (original_available, with_ancestors_available, required, blocked_reason) =
            match resolve_restore_chain(&context, item) {
                Ok(chain) if chain.len() == 1 => (true, false, Vec::new(), None),
                Ok(chain) => (
                    false,
                    true,
                    chain[..chain.len().saturating_sub(1)]
                        .iter()
                        .map(|entry| entry.item.manifest.trash_item_id())
                        .collect(),
                    None,
                ),
                Err(WorkspaceTransactionError::TrashRestoreUnavailable(reason)) => {
                    (false, false, Vec::new(), Some(reason))
                }
                Err(WorkspaceTransactionError::TrashReconciliation(_)) => (
                    false,
                    false,
                    Vec::new(),
                    Some(crate::TrashRestoreBlockedReason::ReconciliationRequired),
                ),
                Err(error) => return Err(error),
            };
        let mut contained_node_ids = item.node_locators.keys().copied().collect::<Vec<_>>();
        contained_node_ids.sort();
        projections.push(crate::WorkspaceTrashItemProjection {
            manifest: item.manifest.clone(),
            contained_node_ids,
            restore: crate::TrashItemRestoreAvailability {
                origin_resolution,
                original_available,
                with_ancestors_available,
                required_ancestor_item_ids: required,
                blocked_reason,
            },
        });
    }
    Ok(projections)
}

/// Projects a path-free, read-only Trash state even when Trash-only diagnostics freeze writes.
/// Invalid items are never partially exposed as trusted authority.
///
/// # Errors
///
/// Returns an error when the active workspace (outside the special Trash store) is invalid.
pub fn project_workspace_trash_state(
    root: impl AsRef<Path>,
) -> Result<crate::WorkspaceTrashStateProjection, WorkspaceTransactionError> {
    let root = root.as_ref();
    let inventory = scan_workspace(root);
    let trash = root.join(TRASH_NODE_NAME);
    if inventory.nodes.is_empty()
        || inventory
            .issues
            .iter()
            .any(|issue| issue.path != trash && !issue.path.starts_with(&trash))
    {
        return Err(WorkspaceTransactionError::InvalidWorkspace);
    }
    let diagnostic_count = u64::try_from(inventory.issues.len()).unwrap_or(u64::MAX);
    if !inventory.issues.is_empty() {
        let legacy_migration_required = inventory.legacy_trash_format
            && inventory
                .issues
                .iter()
                .all(|issue| issue.code == crate::InventoryIssueCode::LegacyTrashMigrationRequired);
        return Ok(crate::WorkspaceTrashStateProjection {
            state: if legacy_migration_required {
                crate::WorkspaceTrashState::LegacyMigrationRequired
            } else {
                crate::WorkspaceTrashState::ReconciliationRequired
            },
            items: Vec::new(),
            legacy_migration_required,
            reconciliation_required: !legacy_migration_required,
            diagnostic_count,
        });
    }
    Ok(crate::WorkspaceTrashStateProjection {
        state: crate::WorkspaceTrashState::Ready,
        items: project_workspace_trash_items(root)?,
        legacy_migration_required: false,
        reconciliation_required: false,
        diagnostic_count: 0,
    })
}

/// Previews a high-permission permanent deletion without mutating Trash.
///
/// # Errors
///
/// Returns an error for an empty/duplicate selection, unknown item, invalid workspace, or byte
/// count overflow.
pub fn preview_permanent_delete_trash_items(
    root: impl AsRef<Path>,
    trash_item_ids: Vec<crate::TrashItemId>,
) -> Result<crate::TrashPermanentDeletePreview, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    permanent_delete_preview(&context, trash_item_ids)
}

/// Converts an exact preview into an opaque Core confirmation.
///
/// # Errors
///
/// Returns an error unless the higher-permission boundary was granted and the destructive phrase
/// is byte-exact.
pub fn confirm_permanent_delete_trash_items(
    preview: crate::TrashPermanentDeletePreview,
    higher_permission_granted: bool,
    exact_phrase: &str,
) -> Result<crate::TrashPermanentDeleteConfirmation, WorkspaceTransactionError> {
    if !higher_permission_granted {
        return Err(WorkspaceTransactionError::PermanentDeleteAuthorizationRequired);
    }
    if exact_phrase != crate::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE {
        return Err(WorkspaceTransactionError::PermanentDeleteConfirmationMismatch);
    }
    let authority_digest = permanent_delete_confirmation_digest(&preview, true)?;
    Ok(crate::TrashPermanentDeleteConfirmation {
        preview,
        higher_permission_granted: true,
        authority_digest,
    })
}

/// Plans journaled permanent deletion of the exact confirmed item set.
///
/// # Errors
///
/// Returns an error for stale revisions, changed manifests/payloads, invalid permission or
/// confirmation evidence, unknown items, or pending recovery.
pub fn plan_permanently_delete_trash_items(
    root: impl AsRef<Path>,
    confirmation: &crate::TrashPermanentDeleteConfirmation,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    if !confirmation.higher_permission_granted
        || confirmation.authority_digest
            != permanent_delete_confirmation_digest(&confirmation.preview, true)?
    {
        return Err(WorkspaceTransactionError::PermanentDeleteConfirmationMismatch);
    }
    let context = PlanningContext::load(root.as_ref())?;
    require_workspace_revision(&confirmation.preview.base_revision, &context.revision)?;
    let item_ids = confirmation
        .preview
        .items
        .iter()
        .map(|item| item.trash_item_id)
        .collect::<Vec<_>>();
    let current = permanent_delete_preview(&context, item_ids)?;
    if current != confirmation.preview {
        return Err(WorkspaceTransactionError::PermanentDeleteConfirmationMismatch);
    }
    let mut steps = Vec::new();
    let mut item_changes = Vec::new();
    for preview_item in &current.items {
        let item = context
            .inventory
            .trash_items
            .iter()
            .find(|item| item.manifest.trash_item_id() == preview_item.trash_item_id)
            .ok_or(WorkspaceTransactionError::UnknownTrashItem(
                preview_item.trash_item_id,
            ))?;
        steps.push(PlannedStep::RemovePath {
            source: relative_string(&context.root, &item.item_path)?,
            digest: tree_digest(&item.item_path)?,
        });
        item_changes.push(crate::WorkspaceTrashPlanItemChange {
            disposition: crate::TrashPlanDisposition::PermanentlyDeleted,
            manifest: item.manifest.clone(),
            destination_node_id: None,
            destination_name: None,
        });
    }
    finalize_reviewed_trash_plan(
        &context,
        StructuralAction::PermanentDelete,
        Vec::new(),
        Vec::new(),
        steps,
        item_changes,
        crate::TrashReviewedAction::PermanentDelete { preview: current },
    )
}

/// Replans one exact, closed Trash preview in another process without accepting executable steps.
/// Generated item/operation identities, destinations, manifests, revision, and workspace-root
/// binding must all reproduce byte-for-byte.
///
/// # Errors
///
/// Returns an error for a changed/tampered request, another workspace root, stale state, changed
/// payloads or destinations, unavailable restore modes, or missing permanent-delete authority.
#[allow(clippy::too_many_lines)]
pub fn replan_reviewed_trash_request(
    root: impl AsRef<Path>,
    request: &crate::TrashReviewedRequest,
    authorization: crate::TrashReviewedReplanAuthorization,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    request
        .validate()
        .map_err(WorkspaceTransactionError::InvalidTrashReviewedRequest)?;
    let root = root.as_ref();
    let root_digest = crate::workspace_trash::trash_reviewed_workspace_root_digest(root)
        .map_err(WorkspaceTransactionError::Io)?;
    if root_digest != request.workspace_root_sha256 {
        return Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
            "reviewed request belongs to another workspace root".to_owned(),
        ));
    }
    let current_revision =
        read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&request.base_revision, &current_revision)?;

    let mut plan = match request.action.clone() {
        crate::TrashReviewedAction::StoreNode {
            node_id,
            trashed_at,
            trash_item_id,
            operation_id,
            trash_node_id,
        } => {
            require_ordinary_trash_replan_authorization(&authorization)?;
            plan_trash_node_at_with_ids(
                root,
                node_id,
                &trashed_at,
                Some((trash_item_id, operation_id, trash_node_id)),
            )?
        }
        crate::TrashReviewedAction::StoreResources {
            resources,
            trashed_at,
            trash_item_ids,
            operation_id,
            trash_node_id,
        } => {
            require_ordinary_trash_replan_authorization(&authorization)?;
            plan_trash_resources_at_with_ids(
                root,
                resources,
                &trashed_at,
                Some((trash_item_ids, operation_id, trash_node_id)),
            )?
        }
        crate::TrashReviewedAction::Restore {
            trash_item_id,
            mode,
        } => {
            require_ordinary_trash_replan_authorization(&authorization)?;
            plan_restore_trash_item(root, trash_item_id, mode)?
        }
        crate::TrashReviewedAction::MigrateLegacy {
            trashed_at,
            trash_item_ids,
            operation_id,
            backup: reviewed_backup,
        } => {
            let crate::TrashReviewedReplanAuthorization::LegacyMigration { backup } = authorization
            else {
                return Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired);
            };
            if backup.authority != reviewed_backup {
                return Err(
                    WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                        "legacy migration replan backup differs from the reviewed snapshot"
                            .to_owned(),
                    ),
                );
            }
            plan_migrate_legacy_workspace_trash_at_with_ids(
                root,
                &trashed_at,
                &backup,
                Some((trash_item_ids, operation_id)),
            )?
        }
        crate::TrashReviewedAction::PermanentDelete { preview } => {
            let crate::TrashReviewedReplanAuthorization::PermanentDelete {
                higher_permission_granted,
                exact_phrase,
            } = authorization
            else {
                return Err(WorkspaceTransactionError::PermanentDeleteAuthorizationRequired);
            };
            let confirmation = confirm_permanent_delete_trash_items(
                preview,
                higher_permission_granted,
                &exact_phrase,
            )?;
            plan_permanently_delete_trash_items(root, &confirmation)?
        }
    };

    if plan.base_revision != request.base_revision
        || plan.path_changes != request.path_changes
        || plan.generated_node_ids != request.generated_node_ids
        || plan.scope_summary != request.scope_summary
        || plan.identity_map != request.identity_map
        || plan.captured_target != request.captured_target
        || plan.target_node_ids != request.target_node_ids
        || plan.draft_sensitive_node_ids != request.draft_sensitive_node_ids
        || plan.trash_item_changes != request.trash_item_changes
        || plan
            .reviewed_trash_request
            .as_ref()
            .is_none_or(|current| current.action != request.action)
    {
        return Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
            "current Core plan differs from the exact reviewed preview".to_owned(),
        ));
    }
    plan.plan_id = request.review_id.to_string();
    plan.reviewed_trash_request = Some(request.clone());
    Ok(plan)
}

fn require_ordinary_trash_replan_authorization(
    authorization: &crate::TrashReviewedReplanAuthorization,
) -> Result<(), WorkspaceTransactionError> {
    if *authorization == crate::TrashReviewedReplanAuthorization::Ordinary {
        Ok(())
    } else {
        Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
            "permanent-delete authority was supplied for an ordinary Trash request".to_owned(),
        ))
    }
}

#[allow(clippy::needless_pass_by_value)]
fn permanent_delete_preview(
    context: &PlanningContext,
    trash_item_ids: Vec<crate::TrashItemId>,
) -> Result<crate::TrashPermanentDeletePreview, WorkspaceTransactionError> {
    if trash_item_ids.is_empty() || trash_item_ids.len() > 10_000 {
        return Err(WorkspaceTransactionError::Metadata(
            "permanent deletion must select 1..=10000 Trash items".to_owned(),
        ));
    }
    let unique = trash_item_ids.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != trash_item_ids.len() {
        return Err(WorkspaceTransactionError::Metadata(
            "permanent deletion contains duplicate Trash item IDs".to_owned(),
        ));
    }
    let mut items = Vec::new();
    let mut total_payload_bytes = 0_u64;
    for trash_item_id in unique {
        let item = context
            .inventory
            .trash_items
            .iter()
            .find(|item| item.manifest.trash_item_id() == trash_item_id)
            .ok_or(WorkspaceTransactionError::UnknownTrashItem(trash_item_id))?;
        total_payload_bytes = total_payload_bytes
            .checked_add(item.manifest.payload_byte_length())
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "permanent deletion payload byte count overflowed".to_owned(),
                )
            })?;
        items.push(crate::TrashPermanentDeleteItemPreview {
            trash_item_id,
            kind: item.manifest.kind(),
            original_name: item.manifest.original_name().to_owned(),
            payload_sha256: item.manifest.payload_sha256().to_owned(),
            payload_byte_length: item.manifest.payload_byte_length(),
        });
    }
    Ok(crate::TrashPermanentDeletePreview {
        base_revision: context.revision.clone(),
        items,
        total_payload_bytes,
    })
}

fn permanent_delete_confirmation_digest(
    preview: &crate::TrashPermanentDeletePreview,
    higher_permission_granted: bool,
) -> Result<String, WorkspaceTransactionError> {
    let bytes = serde_json::to_vec(&(
        "weftext.trash-permanent-delete-confirmation/v1",
        crate::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
        higher_permission_granted,
        preview,
    ))
    .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&bytes))
}

struct ResolvedRestoreItem<'a> {
    item: &'a crate::WorkspaceTrashItem,
    destination_node_id: NodeId,
    destination_name: String,
    destination_path: PathBuf,
}

fn resolve_restore_chain<'a>(
    context: &'a PlanningContext,
    item: &'a crate::WorkspaceTrashItem,
) -> Result<Vec<ResolvedRestoreItem<'a>>, WorkspaceTransactionError> {
    let mut visiting = BTreeSet::new();
    resolve_restore_chain_inner(context, item, &mut visiting)
}

fn resolve_restore_chain_inner<'a>(
    context: &'a PlanningContext,
    item: &'a crate::WorkspaceTrashItem,
    visiting: &mut BTreeSet<crate::TrashItemId>,
) -> Result<Vec<ResolvedRestoreItem<'a>>, WorkspaceTransactionError> {
    let item_id = item.manifest.trash_item_id();
    if !visiting.insert(item_id) {
        return Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            crate::TrashRestoreBlockedReason::AncestorCycle,
        ));
    }
    let origin_id = trash_manifest_origin(&item.manifest).ok_or(
        WorkspaceTransactionError::TrashRestoreUnavailable(
            crate::TrashRestoreBlockedReason::OriginUnknown,
        ),
    )?;
    let original_name = item.manifest.original_name().to_owned();
    if let Some(active_origin) = context
        .inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(origin_id))
    {
        reject_trash_parent(context, active_origin)?;
        let destination = active_origin.path.join(&original_name);
        validate_restored_destination(context, item, &destination, &destination)?;
        visiting.remove(&item_id);
        return Ok(vec![ResolvedRestoreItem {
            item,
            destination_node_id: origin_id,
            destination_name: original_name,
            destination_path: destination,
        }]);
    }

    let mut containing = context
        .inventory
        .trash_items
        .iter()
        .filter(|candidate| candidate.node_locators.contains_key(&origin_id));
    let parent_item =
        containing
            .next()
            .ok_or(WorkspaceTransactionError::TrashRestoreUnavailable(
                crate::TrashRestoreBlockedReason::OriginMissing,
            ))?;
    if containing.next().is_some() {
        return Err(WorkspaceTransactionError::TrashRestoreUnavailable(
            crate::TrashRestoreBlockedReason::AncestorAmbiguous,
        ));
    }
    let mut chain = resolve_restore_chain_inner(context, parent_item, visiting)?;
    let parent_destination = chain
        .iter()
        .find(|resolved| {
            resolved.item.manifest.trash_item_id() == parent_item.manifest.trash_item_id()
        })
        .ok_or_else(|| {
            WorkspaceTransactionError::TrashReconciliation(
                "resolved ancestor chain omitted its parent item".to_owned(),
            )
        })?;
    let locator = parent_item.node_locators.get(&origin_id).ok_or_else(|| {
        WorkspaceTransactionError::TrashReconciliation(
            "ancestor item no longer carries the recorded origin node".to_owned(),
        )
    })?;
    let relative_inside_root = locator
        .split_once('/')
        .map_or("", |(_, remainder)| remainder);
    let future_origin = if relative_inside_root.is_empty() {
        parent_destination.destination_path.clone()
    } else {
        parent_destination
            .destination_path
            .join(Path::new(relative_inside_root))
    };
    let source_origin = parent_item
        .item_path
        .join(crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME)
        .join(Path::new(locator));
    let destination = future_origin.join(&original_name);
    let collision_probe = source_origin.join(&original_name);
    validate_restored_destination(context, item, &destination, &collision_probe)?;
    chain.push(ResolvedRestoreItem {
        item,
        destination_node_id: origin_id,
        destination_name: original_name,
        destination_path: destination,
    });
    visiting.remove(&item_id);
    Ok(chain)
}

fn resolve_existing_target<'a>(
    context: &'a PlanningContext,
    item: &'a crate::WorkspaceTrashItem,
    target_node_id: NodeId,
    name: &str,
) -> Result<ResolvedRestoreItem<'a>, WorkspaceTransactionError> {
    let target = context.node(target_node_id)?;
    reject_trash_parent(context, target)?;
    validate_restore_name(item.manifest.kind(), name)?;
    let destination = target.path.join(name);
    validate_restored_destination(context, item, &destination, &destination)?;
    Ok(ResolvedRestoreItem {
        item,
        destination_node_id: target_node_id,
        destination_name: name.to_owned(),
        destination_path: destination,
    })
}

fn trash_manifest_origin(manifest: &crate::TrashItemManifest) -> Option<NodeId> {
    match manifest.kind() {
        crate::TrashItemKind::Node => manifest.original_parent_node_id(),
        crate::TrashItemKind::Resource => manifest.original_owner_node_id(),
    }
}

fn validate_restore_name(
    kind: crate::TrashItemKind,
    name: &str,
) -> Result<(), WorkspaceTransactionError> {
    if kind == crate::TrashItemKind::Node {
        validate_node_name(name, false).map_err(WorkspaceTransactionError::Workspace)?;
    } else {
        validate_portable_path_component(name, false)
            .map_err(WorkspaceTransactionError::Workspace)?;
        let folded = name.to_ascii_lowercase();
        if folded == crate::ANNOTATIONS_FILE_NAME
            || folded == crate::WORKSPACE_FORMAT_MARKER_FILE
            || folded == crate::content_boundary::CONTENT_RULES_FILE_NAME
            || folded == crate::TRASH_ITEM_MANIFEST_FILE_NAME
            || folded == crate::TRASH_ITEMS_DIRECTORY_NAME
            || folded.starts_with(".__weftext-transaction-")
        {
            return Err(WorkspaceTransactionError::ContentBoundary(
                "resource restore name is reserved by Core".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_restored_destination(
    context: &PlanningContext,
    item: &crate::WorkspaceTrashItem,
    destination: &Path,
    collision_probe: &Path,
) -> Result<(), WorkspaceTransactionError> {
    validate_restore_name(item.manifest.kind(), item.manifest.original_name())?;
    match item.manifest.kind() {
        crate::TrashItemKind::Node => require_managed_destination(destination)?,
        crate::TrashItemKind::Resource => validate_managed_file_path(&context.root, destination)
            .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?,
    }
    require_destination_unclassified(
        context,
        destination,
        item.manifest.kind() == crate::TrashItemKind::Node,
    )?;
    if let Some(reason) = portable_destination_conflict(collision_probe)? {
        return Err(WorkspaceTransactionError::TrashRestoreUnavailable(reason));
    }
    Ok(())
}

fn portable_destination_conflict(
    destination: &Path,
) -> Result<Option<crate::TrashRestoreBlockedReason>, WorkspaceTransactionError> {
    match fs::symlink_metadata(destination) {
        Ok(_) => return Ok(Some(crate::TrashRestoreBlockedReason::NameConflict)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(WorkspaceTransactionError::Io(error)),
    }
    let Some(parent) = destination.parent() else {
        return Err(WorkspaceTransactionError::PathEscape(
            destination.to_path_buf(),
        ));
    };
    if !parent.is_dir() {
        return Ok(None);
    }
    let expected = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(destination.to_path_buf()))?;
    for entry in fs::read_dir(parent).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if name.to_lowercase() == expected.to_lowercase() {
            return Ok(Some(crate::TrashRestoreBlockedReason::CaseFoldConflict));
        }
    }
    Ok(None)
}

fn require_destination_unclassified(
    context: &PlanningContext,
    destination: &Path,
    is_directory: bool,
) -> Result<(), WorkspaceTransactionError> {
    let rules = crate::content_boundary::ContentRules::load(&context.root)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let relative = crate::content_boundary::portable_path(&context.root, destination)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    if rules.classify(&relative, is_directory).is_some() {
        Err(WorkspaceTransactionError::ContentBoundary(format!(
            "destination is classified unmanaged or ignored: {relative}"
        )))
    } else {
        Ok(())
    }
}

fn build_restore_plan(
    context: &PlanningContext,
    resolved: &[ResolvedRestoreItem<'_>],
    reviewed_action: crate::TrashReviewedAction,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let mut steps = Vec::new();
    let mut path_changes = Vec::new();
    let mut item_changes = Vec::new();
    for entry in resolved {
        let item = entry.item;
        let destination = relative_string(&context.root, &entry.destination_path)?;
        match item.manifest.kind() {
            crate::TrashItemKind::Node => {
                let payload = restored_node_payload(item, &entry.destination_name)?;
                steps.push(PlannedStep::CreateTree {
                    destination: destination.clone(),
                    payload,
                });
                let old_root_name = item.manifest.original_name();
                for (node_id, locator) in &item.node_locators {
                    let relative_inside_root = locator
                        .strip_prefix(old_root_name)
                        .and_then(|value| value.strip_prefix('/').or(Some(value)))
                        .ok_or_else(|| {
                            WorkspaceTransactionError::TrashReconciliation(
                                "Trash node locator is outside its item root".to_owned(),
                            )
                        })?;
                    let new_path = if relative_inside_root.is_empty() {
                        destination.clone()
                    } else {
                        format!("{destination}/{relative_inside_root}")
                    };
                    path_changes.push(WorkspacePathChange {
                        source_node_id: Some(*node_id),
                        node_id: *node_id,
                        old_path: Some(format!(
                            "{}/{}/{locator}",
                            relative_string(context.root.as_path(), &item.item_path)?,
                            crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME
                        )),
                        new_path,
                    });
                }
            }
            crate::TrashItemKind::Resource => {
                let bytes = fs::read(&item.payload_path).map_err(WorkspaceTransactionError::Io)?;
                steps.push(PlannedStep::CreateFile {
                    destination,
                    next_digest: format!("{:x}", Sha256::digest(&bytes)),
                    next_bytes: bytes,
                });
            }
        }
        steps.push(PlannedStep::RemovePath {
            source: relative_string(&context.root, &item.item_path)?,
            digest: tree_digest(&item.item_path)?,
        });
        item_changes.push(crate::WorkspaceTrashPlanItemChange {
            disposition: crate::TrashPlanDisposition::Restored,
            manifest: item.manifest.clone(),
            destination_node_id: Some(entry.destination_node_id),
            destination_name: Some(entry.destination_name.clone()),
        });
    }
    finalize_reviewed_trash_plan(
        context,
        StructuralAction::Restore,
        path_changes,
        Vec::new(),
        steps,
        item_changes,
        reviewed_action,
    )
}

fn restored_node_payload(
    item: &crate::WorkspaceTrashItem,
    destination_name: &str,
) -> Result<TreePayload, WorkspaceTransactionError> {
    let mut payload = collect_existing_tree_payload(&item.payload_path)?;
    let original_document = format!("{}.adoc", item.manifest.original_name());
    let destination_document = format!("{destination_name}.adoc");
    let document = payload
        .files
        .iter_mut()
        .find(|file| file.path == original_document)
        .ok_or_else(|| {
            WorkspaceTransactionError::TrashReconciliation(
                "Trash node payload lost its canonical root document".to_owned(),
            )
        })?;
    document.path = destination_document;
    payload
        .files
        .sort_by(|left, right| left.path.cmp(&right.path));
    payload.digest = payload_digest(&payload.directories, &payload.files);
    Ok(payload)
}

/// Plans replacement of one node's ordered alias list as a narrow envelope patch.
///
/// An empty list removes the optional field. Unknown forward-compatible `weftext` fields and all
/// unrelated source bytes are preserved exactly.
///
/// # Errors
///
/// Returns an error for an invalid workspace/node, stale document revision, invalid aliases,
/// ambiguous YAML, a no-op, or pending recovery.
pub fn plan_node_aliases_setting(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    aliases: &[String],
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_node_metadata_update(root.as_ref(), node_id, base_revision, false, |source| {
        set_node_aliases(source, aliases)
    })
}

/// Plans replacement or removal of one node's portable scalar icon as a
/// narrow envelope patch.
///
/// # Errors
///
/// Returns an error for an unsupported scalar, invalid workspace/node, stale
/// document revision, ambiguous YAML, a no-op, or pending recovery.
pub fn plan_node_icon_setting(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    icon: Option<&str>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_node_metadata_update(root.as_ref(), node_id, base_revision, false, |source| {
        set_node_icon(source, icon)
    })
}

/// Plans replacement of one node's direct-child ordering policy as a narrow envelope patch.
///
/// Default name/ascending ordering removes the optional fields. Manual ordering retains child
/// ranks and removes the inapplicable direction field.
///
/// # Errors
///
/// Returns an error for an invalid workspace/node, stale document revision, ambiguous YAML, a
/// no-op, or pending recovery.
pub fn plan_node_child_sort_setting(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    child_sort: ChildSort,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_node_metadata_update(root.as_ref(), node_id, base_revision, false, |source| {
        set_node_child_sort(source, child_sort)
    })
}

/// Plans replacement or removal of one node's sparse sibling rank.
///
/// # Errors
///
/// Returns an error for zero rank, an invalid workspace/node, stale document revision, ambiguous
/// YAML, a no-op, or pending recovery.
pub fn plan_node_sibling_rank_setting(
    root: impl AsRef<Path>,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    sibling_rank: Option<u64>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    plan_node_metadata_update(root.as_ref(), node_id, base_revision, true, |source| {
        set_node_sibling_rank(source, sibling_rank)
    })
}

fn plan_node_metadata_update(
    root: &Path,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    require_parent: bool,
    update: impl FnOnce(&str) -> Result<String, crate::FrontmatterError>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root)?;
    let node = context.node(node_id)?;
    if require_parent && node.parent_id.is_none() {
        return Err(WorkspaceTransactionError::RootMutationUnsupported);
    }
    let snapshot = read_node_document(&node.path).map_err(WorkspaceTransactionError::Document)?;
    if &snapshot.revision != base_revision {
        return Err(WorkspaceTransactionError::Document(
            crate::DocumentError::StaleRevision {
                expected: base_revision.clone(),
                actual: snapshot.revision,
            },
        ));
    }
    let next_source = update(&snapshot.source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if next_source == snapshot.source {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let metadata = crate::parse_node_metadata(&next_source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if metadata.id != Some(node_id) {
        return Err(WorkspaceTransactionError::Metadata(
            "node metadata patch changed or removed identity".to_owned(),
        ));
    }
    let next_revision = DocumentRevision::from_source(&next_source);
    let next_bytes = next_source.into_bytes();
    let destination = relative_string(&context.root, &node.document_path)?;
    Ok(finalize_plan(
        &context,
        StructuralAction::NodeMetadata,
        Vec::new(),
        vec![WorkspaceDocumentChange {
            node_id,
            path: destination.clone(),
            base_revision: snapshot.revision,
            next_revision,
            edit_count: 1,
        }],
        Vec::new(),
        vec![PlannedStep::ReplaceFile {
            destination,
            base_digest: format!("{:x}", Sha256::digest(snapshot.source.as_bytes())),
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        }],
    ))
}

/// Plans a narrow root-document patch for the portable run-in presentation setting.
///
/// # Errors
///
/// Returns an error when the workspace is invalid or has an ambiguous reserved
/// YAML shape that cannot be patched without reserializing user frontmatter.
pub fn plan_adjacent_heading_body_setting(
    root: impl AsRef<Path>,
    value: AdjacentHeadingBody,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let root_node = context.root_node()?;
    let snapshot =
        read_node_document(&root_node.path).map_err(WorkspaceTransactionError::Document)?;
    let next_source = set_adjacent_heading_body(&snapshot.source, value)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if next_source == snapshot.source {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let next_revision = DocumentRevision::from_source(&next_source);
    let next_bytes = next_source.into_bytes();
    let destination = relative_string(&context.root, &root_node.document_path)?;
    Ok(finalize_plan(
        &context,
        StructuralAction::Presentation,
        Vec::new(),
        vec![WorkspaceDocumentChange {
            node_id: root_node
                .id
                .ok_or(WorkspaceTransactionError::InvalidWorkspace)?,
            path: destination.clone(),
            base_revision: snapshot.revision,
            next_revision,
            edit_count: 1,
        }],
        Vec::new(),
        vec![PlannedStep::ReplaceFile {
            destination,
            base_digest: format!("{:x}", Sha256::digest(snapshot.source.as_bytes())),
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        }],
    ))
}

/// Captures the exact fixed-sidecar state from a backend that can prove its
/// workspace replica is complete.
///
/// # Errors
///
/// Returns an error for partial/unknown replicas, an invalid workspace,
/// unknown node, malformed sidecar, foreign document identity, or a state
/// change while the snapshot is being captured.
pub fn capture_annotation_sidecar_snapshot(
    root: impl AsRef<Path>,
    node_id: NodeId,
    completeness: AnnotationReplicaCompleteness,
) -> Result<AnnotationSidecarSnapshot, WorkspaceTransactionError> {
    require_complete_annotation_replica(completeness)?;
    let context = PlanningContext::load(root.as_ref())?;
    let node = context.node(node_id)?;
    let sidecar = node.path.join(ANNOTATIONS_FILE_NAME);
    validate_managed_file_path(&context.root, &sidecar)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    require_no_annotation_conflict_copies(&node.path)?;
    let (expected_state, store) = observe_annotation_sidecar(&sidecar, node_id)?;
    let latest =
        read_workspace_revision(&context.root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&context.revision, &latest)?;
    let canonical_workspace_root =
        fs::canonicalize(&context.root).map_err(WorkspaceTransactionError::Io)?;
    Ok(AnnotationSidecarSnapshot {
        canonical_workspace_root,
        node_id,
        workspace_revision: context.revision,
        completeness,
        expected_state,
        store,
    })
}

/// Reads one sidecar through the same complete-replica snapshot contract used
/// by mutation planning.
///
/// # Errors
///
/// Returns the same errors as [`capture_annotation_sidecar_snapshot`].
pub fn read_node_annotations(
    root: impl AsRef<Path>,
    node_id: NodeId,
    completeness: AnnotationReplicaCompleteness,
) -> Result<AnnotationStore, WorkspaceTransactionError> {
    capture_annotation_sidecar_snapshot(root, node_id, completeness)
        .map(AnnotationSidecarSnapshot::into_store)
}

/// Reads one node-local sidecar after the caller has already resolved and
/// authorized the exact node path.
///
/// This read does not inspect unrelated workspace nodes. It is intended for
/// scoped read surfaces whose authorization layer has already bound `node_path`
/// to `node_id`; mutation planning must still use
/// [`capture_annotation_sidecar_snapshot`].
///
/// # Errors
///
/// Returns an error when the sidecar is linked, oversized, malformed, or bound
/// to a different document identity.
pub fn read_node_annotations_at_node_path(
    node_path: impl AsRef<Path>,
    node_id: NodeId,
) -> Result<AnnotationStore, WorkspaceTransactionError> {
    let node_path = node_path.as_ref();
    require_no_annotation_conflict_copies(node_path)?;
    let sidecar = node_path.join(ANNOTATIONS_FILE_NAME);
    observe_annotation_sidecar(&sidecar, node_id).map(|(_, store)| store)
}

pub(crate) fn observe_annotation_sidecar_at_authorized_node(
    node_path: &Path,
    node_id: NodeId,
) -> Result<(TaskPromotionSidecarState, AnnotationStore), WorkspaceTransactionError> {
    require_no_annotation_conflict_copies(node_path)?;
    let path = node_path.join(ANNOTATIONS_FILE_NAME);
    let (state, store) = observe_annotation_sidecar(&path, node_id)?;
    let state = match state {
        AnnotationSidecarExpectedState::Present { sha256 } => {
            TaskPromotionSidecarState::Present { sha256 }
        }
        AnnotationSidecarExpectedState::ConfirmedAbsent => {
            TaskPromotionSidecarState::ConfirmedAbsent
        }
    };
    Ok((state, store))
}

pub(crate) fn validate_task_promotion_annotation_snapshot(
    root: &Path,
    node_path: &Path,
    node_id: NodeId,
    workspace_revision: &WorkspaceRevision,
    snapshot: &AnnotationSidecarSnapshot,
) -> Result<
    (
        TaskPromotionSidecarState,
        AnnotationStore,
        AnnotationReplicaCompleteness,
    ),
    WorkspaceTransactionError,
> {
    let canonical_workspace_root = fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?;
    if canonical_workspace_root != snapshot.canonical_workspace_root
        || snapshot.node_id != node_id
        || &snapshot.workspace_revision != workspace_revision
    {
        return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
    }
    require_complete_annotation_replica(snapshot.completeness)?;
    validate_managed_file_path(root, &node_path.join(ANNOTATIONS_FILE_NAME))
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    require_no_annotation_conflict_copies(node_path)?;
    let sidecar = node_path.join(ANNOTATIONS_FILE_NAME);
    let (observed_state, observed_store) = observe_annotation_sidecar(&sidecar, node_id)?;
    if observed_state != snapshot.expected_state || observed_store != snapshot.store {
        return Err(WorkspaceTransactionError::AnnotationSidecarChanged);
    }
    let state = match observed_state {
        AnnotationSidecarExpectedState::Present { sha256 } => {
            TaskPromotionSidecarState::Present { sha256 }
        }
        AnnotationSidecarExpectedState::ConfirmedAbsent => {
            TaskPromotionSidecarState::ConfirmedAbsent
        }
    };
    Ok((state, observed_store, snapshot.completeness))
}

/// Plans one recoverable annotation mutation against the fixed node-local
/// sidecar. Suggestion acceptance updates the document and sidecar in the same
/// workspace transaction.
///
/// # Errors
///
/// Returns an error for stale/invalid workspace state, malformed existing
/// annotations, unknown targets, invalid messages, ambiguous anchors, or an
/// invalid suggestion edit.
#[allow(clippy::too_many_lines)]
pub fn plan_annotation_action(
    root: impl AsRef<Path>,
    sidecar_snapshot: &AnnotationSidecarSnapshot,
    action: AnnotationAction,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let context = PlanningContext::load(root.as_ref())?;
    let canonical_workspace_root =
        fs::canonicalize(&context.root).map_err(WorkspaceTransactionError::Io)?;
    if canonical_workspace_root != sidecar_snapshot.canonical_workspace_root {
        return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
    }
    require_complete_annotation_replica(sidecar_snapshot.completeness)?;
    require_workspace_revision(&sidecar_snapshot.workspace_revision, &context.revision)?;
    let node_id = sidecar_snapshot.node_id;
    let node = context.node(node_id)?;
    let snapshot = read_node_document(&node.path).map_err(WorkspaceTransactionError::Document)?;
    let sidecar = node.path.join(ANNOTATIONS_FILE_NAME);
    validate_managed_file_path(&context.root, &sidecar)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    require_no_annotation_conflict_copies(&node.path)?;
    let mut store =
        load_expected_annotation_store(&sidecar, node_id, &sidecar_snapshot.expected_state)?;
    if store != sidecar_snapshot.store {
        return Err(WorkspaceTransactionError::AnnotationSidecarChanged);
    }
    let mut document_plan = None;
    match action {
        AnnotationAction::Create {
            kind,
            target,
            appearance,
            labels,
            body_source,
            suggested_source,
            author_id,
            author_name,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let target = build_annotation_target(
                snapshot.profile,
                &snapshot.source,
                snapshot.revision.as_str(),
                &target,
            )
            .map_err(|error| annotation_metadata_error(&error))?;
            let thread = body_source
                .map(|body_source| {
                    require_annotation_text(&body_source, &timestamp)?;
                    Ok(ThreadMessage {
                        id: uuid::Uuid::new_v4(),
                        author_id,
                        author_name,
                        body: AnnotationBody::asciidoc(body_source),
                        created_at: timestamp.clone(),
                        updated_at: timestamp.clone(),
                    })
                })
                .transpose()?
                .into_iter()
                .collect();
            let now = timestamp.clone();
            store.annotations.push(Annotation {
                id: uuid::Uuid::new_v4(),
                kind,
                target,
                appearance,
                suggested_source,
                labels,
                thread,
                state: AnnotationState::Open,
                resolution: None,
                created_at: now.clone(),
                updated_at: now,
            });
        }
        AnnotationAction::Reply {
            annotation_id,
            body_source,
            author_id,
            author_name,
            timestamp,
        } => {
            require_annotation_text(&body_source, &timestamp)?;
            let annotation = store
                .annotations
                .iter_mut()
                .find(|annotation| annotation.id == annotation_id)
                .ok_or_else(|| {
                    WorkspaceTransactionError::Metadata("annotation is unavailable".to_owned())
                })?;
            annotation.thread.push(ThreadMessage {
                id: uuid::Uuid::new_v4(),
                author_id,
                author_name,
                body: AnnotationBody::asciidoc(body_source),
                created_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            });
            annotation.updated_at = timestamp;
        }
        AnnotationAction::EditMessage {
            annotation_id,
            message_id,
            body_source,
            author_id,
            timestamp,
        } => {
            require_annotation_text(&body_source, &timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            let message = annotation
                .thread
                .iter_mut()
                .find(|message| message.id == message_id)
                .ok_or_else(annotation_unavailable)?;
            if message.author_id != author_id {
                return Err(WorkspaceTransactionError::Metadata(
                    "only the message author can edit an annotation reply".to_owned(),
                ));
            }
            if message.body.source == body_source {
                return Err(WorkspaceTransactionError::NoChange);
            }
            message.body = AnnotationBody::asciidoc(body_source);
            message.updated_at.clone_from(&timestamp);
            annotation.updated_at = timestamp;
        }
        AnnotationAction::SetAppearance {
            annotation_id,
            appearance,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            if annotation.appearance == appearance {
                return Err(WorkspaceTransactionError::NoChange);
            }
            annotation.appearance = appearance;
            annotation.updated_at = timestamp;
        }
        AnnotationAction::SetLabels {
            annotation_id,
            labels,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            if annotation.labels == labels {
                return Err(WorkspaceTransactionError::NoChange);
            }
            annotation.labels = labels;
            annotation.updated_at = timestamp;
        }
        AnnotationAction::SetResolved {
            annotation_id,
            resolved,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            let (state, resolution) = if resolved {
                (
                    AnnotationState::Resolved,
                    Some(AnnotationResolution::Resolved),
                )
            } else {
                if annotation.resolution == Some(AnnotationResolution::Accepted) {
                    return Err(WorkspaceTransactionError::Metadata(
                        "an accepted suggestion cannot be reopened".to_owned(),
                    ));
                }
                (AnnotationState::Open, None)
            };
            if annotation.state == state && annotation.resolution == resolution {
                return Err(WorkspaceTransactionError::NoChange);
            }
            annotation.state = state;
            annotation.resolution = resolution;
            annotation.updated_at = timestamp;
        }
        AnnotationAction::Reanchor {
            annotation_id,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            let outcome = reanchor_annotation(
                annotation,
                snapshot.profile,
                &snapshot.source,
                snapshot.revision.as_str(),
            );
            if outcome == AnnotationReanchorOutcome::Unchanged {
                return Err(WorkspaceTransactionError::NoChange);
            }
            annotation.updated_at = timestamp;
        }
        AnnotationAction::AcceptSuggestion {
            annotation_id,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            let suggestion = annotation_suggestion_edit(
                annotation,
                snapshot.profile,
                &snapshot.source,
                snapshot.revision.as_str(),
            )
            .map_err(|error| annotation_metadata_error(&error))?;
            let plan = plan_document_edit(
                &node.path,
                &snapshot.revision,
                [DocumentEdit {
                    start: suggestion.range.start,
                    end: suggestion.range.end,
                    replacement: suggestion.replacement,
                }],
            )
            .map_err(WorkspaceTransactionError::Document)?;
            if !plan.changed {
                return Err(WorkspaceTransactionError::NoChange);
            }
            annotation.state = AnnotationState::Resolved;
            annotation.resolution = Some(AnnotationResolution::Accepted);
            annotation.updated_at = timestamp;
            document_plan = Some(plan);
        }
        AnnotationAction::RejectSuggestion {
            annotation_id,
            timestamp,
        } => {
            require_annotation_timestamp(&timestamp)?;
            let annotation = require_annotation_mut(&mut store, annotation_id)?;
            if !matches!(
                annotation.kind,
                AnnotationKind::SuggestionInsert | AnnotationKind::SuggestionDelete
            ) || annotation.state != AnnotationState::Open
            {
                return Err(WorkspaceTransactionError::Metadata(
                    "annotation is not an open suggestion".to_owned(),
                ));
            }
            annotation.state = AnnotationState::Resolved;
            annotation.resolution = Some(AnnotationResolution::Rejected);
            annotation.updated_at = timestamp;
        }
    }
    let next_source = store
        .to_pretty_json()
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    let next_bytes = next_source.into_bytes();
    let next_digest = format!("{:x}", Sha256::digest(&next_bytes));
    let destination = relative_string(&context.root, &sidecar)?;
    let sidecar_step = match &sidecar_snapshot.expected_state {
        AnnotationSidecarExpectedState::Present { sha256 } => PlannedStep::ReplaceFile {
            destination: destination.clone(),
            base_digest: sha256.clone(),
            next_digest,
            next_bytes,
        },
        AnnotationSidecarExpectedState::ConfirmedAbsent => PlannedStep::CreateFile {
            destination: destination.clone(),
            next_digest,
            next_bytes,
        },
    };
    let mut steps = Vec::new();
    let mut document_changes = Vec::new();
    if let Some(document) = document_plan {
        let destination = relative_string(&context.root, &node.document_path)?;
        let next_bytes = document.next_source().as_bytes().to_vec();
        steps.push(PlannedStep::ReplaceFile {
            destination: destination.clone(),
            base_digest: document.base_revision.as_str().to_owned(),
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        });
        document_changes.push(WorkspaceDocumentChange {
            node_id,
            path: destination,
            base_revision: document.base_revision,
            next_revision: document.next_revision,
            edit_count: to_u64(document.edits.len()),
        });
    }
    steps.push(sidecar_step);
    let mut plan = finalize_plan(
        &context,
        StructuralAction::Annotation,
        Vec::new(),
        document_changes,
        Vec::new(),
        steps,
    );
    plan.annotation_sidecar_authority = Some(AnnotationSidecarPlanAuthority {
        node_id,
        destination,
        workspace_revision: sidecar_snapshot.workspace_revision.clone(),
        completeness: sidecar_snapshot.completeness,
        expected_state: sidecar_snapshot.expected_state.clone(),
    });
    Ok(plan)
}

fn require_annotation_mut(
    store: &mut AnnotationStore,
    annotation_id: uuid::Uuid,
) -> Result<&mut Annotation, WorkspaceTransactionError> {
    store
        .annotations
        .iter_mut()
        .find(|annotation| annotation.id == annotation_id)
        .ok_or_else(annotation_unavailable)
}

fn annotation_unavailable() -> WorkspaceTransactionError {
    WorkspaceTransactionError::Metadata("annotation is unavailable".to_owned())
}

fn annotation_metadata_error(error: &impl ToString) -> WorkspaceTransactionError {
    WorkspaceTransactionError::Metadata(error.to_string())
}

fn annotation_store_validation_error(
    error: crate::AnnotationValidationError,
) -> WorkspaceTransactionError {
    match error {
        crate::AnnotationValidationError::DuplicateId(_)
        | crate::AnnotationValidationError::DocumentMismatch => {
            WorkspaceTransactionError::AnnotationSidecarReconciliationRequired
        }
        other => annotation_metadata_error(&other),
    }
}

fn require_no_annotation_conflict_copies(
    node_directory: &Path,
) -> Result<(), WorkspaceTransactionError> {
    let entries = fs::read_dir(node_directory).map_err(WorkspaceTransactionError::Io)?;
    for entry in entries {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if is_annotation_conflict_copy_name(&name) {
            return Err(WorkspaceTransactionError::AnnotationSidecarReconciliationRequired);
        }
    }
    Ok(())
}

fn is_annotation_conflict_copy_name(name: &str) -> bool {
    if name == ANNOTATIONS_FILE_NAME {
        return false;
    }
    let folded = name.to_lowercase();
    if folded == ANNOTATIONS_FILE_NAME
        || folded.starts_with("weftext.annotations")
        || folded.contains(ANNOTATIONS_FILE_NAME)
    {
        return true;
    }
    let annotation_json = Path::new(name)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        && folded.contains("weftext")
        && folded.contains("annotations");
    annotation_json
        && [
            "conflict",
            "conflicted",
            "copy",
            "duplicate",
            "merge",
            "sync",
        ]
        .iter()
        .any(|marker| folded.contains(marker))
}

fn require_complete_annotation_replica(
    completeness: AnnotationReplicaCompleteness,
) -> Result<(), WorkspaceTransactionError> {
    if completeness.is_complete() {
        Ok(())
    } else {
        Err(WorkspaceTransactionError::IncompleteAnnotationReplica)
    }
}

fn observe_annotation_sidecar(
    path: &Path,
    node_id: NodeId,
) -> Result<(AnnotationSidecarExpectedState, AnnotationStore), WorkspaceTransactionError> {
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((
                AnnotationSidecarExpectedState::ConfirmedAbsent,
                AnnotationStore::empty(node_id),
            ));
        }
        Err(error) => return Err(WorkspaceTransactionError::Io(error)),
    };
    if linked_or_reparse(&metadata) || !metadata.is_file() {
        return Err(WorkspaceTransactionError::ContentBoundary(
            "annotation sidecar is not a regular non-link file".to_owned(),
        ));
    }
    let maximum = u64::try_from(MAX_ANNOTATION_STORE_BYTES).unwrap_or(u64::MAX);
    if metadata.len() > maximum {
        return Err(WorkspaceTransactionError::Metadata(
            "annotation sidecar exceeds the byte limit".to_owned(),
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(WorkspaceTransactionError::Io)?
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum {
        return Err(WorkspaceTransactionError::Metadata(
            "annotation sidecar exceeds the byte limit".to_owned(),
        ));
    }
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let source = std::str::from_utf8(&bytes)
        .map_err(|_| WorkspaceTransactionError::InvalidUtf8(path.to_path_buf()))?;
    let store = AnnotationStore::from_json(source).map_err(annotation_store_validation_error)?;
    store
        .validate(node_id)
        .map_err(annotation_store_validation_error)?;
    Ok((
        AnnotationSidecarExpectedState::Present {
            sha256: digest_bytes(&bytes),
        },
        store,
    ))
}

fn load_expected_annotation_store(
    path: &Path,
    node_id: NodeId,
    expected: &AnnotationSidecarExpectedState,
) -> Result<AnnotationStore, WorkspaceTransactionError> {
    let (actual, store) = observe_annotation_sidecar(path, node_id)?;
    if &actual == expected {
        Ok(store)
    } else {
        Err(WorkspaceTransactionError::AnnotationSidecarChanged)
    }
}

fn require_annotation_text(body: &str, timestamp: &str) -> Result<(), WorkspaceTransactionError> {
    if body.trim().is_empty() || timestamp.trim().is_empty() {
        return Err(WorkspaceTransactionError::Metadata(
            "annotation message or timestamp is empty".to_owned(),
        ));
    }
    Ok(())
}

fn require_annotation_timestamp(timestamp: &str) -> Result<(), WorkspaceTransactionError> {
    if timestamp.trim().is_empty() {
        Err(WorkspaceTransactionError::Metadata(
            "annotation timestamp is empty".to_owned(),
        ))
    } else {
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn build_move_plan(
    context: &PlanningContext,
    action: StructuralAction,
    node: &crate::NodeRecord,
    parent: &crate::NodeRecord,
    destination_name: &str,
    rewrite_links: bool,
    mut steps: Vec<PlannedStep>,
    generated: Vec<NodeId>,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    if parent.path.starts_with(&node.path) {
        return Err(WorkspaceTransactionError::MoveIntoDescendant);
    }
    let destination = parent.path.join(destination_name);
    context.require_managed_subtree(node)?;
    require_managed_destination(&destination)?;
    if destination == node.path {
        return Err(WorkspaceTransactionError::NoChange);
    }
    require_destination_available(&destination, Some(&node.path))?;
    steps.push(PlannedStep::MovePath {
        source: relative_string(&context.root, &node.path)?,
        destination: relative_string(&context.root, &destination)?,
        digest: tree_digest(&node.path)?,
    });
    if node.name != destination_name {
        steps.push(PlannedStep::MovePath {
            source: relative_string(
                &context.root,
                &generation_document_path(context.generation, &destination, &node.name)?,
            )?,
            destination: relative_string(
                &context.root,
                &generation_document_path(context.generation, &destination, destination_name)?,
            )?,
            digest: file_digest(&node.document_path)?,
        });
    }
    let subtree = subtree_nodes(context, &node.path);
    let mut changes = Vec::new();
    let mut locators = BTreeMap::new();
    let mut paths = BTreeMap::new();
    for record in subtree {
        let id = record
            .id
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        let inside = record
            .path
            .strip_prefix(&node.path)
            .map_err(|_| WorkspaceTransactionError::PathEscape(record.path.clone()))?;
        let new_path = destination.join(inside);
        let new_relative = relative_string(&context.root, &new_path)?;
        locators.insert(id, new_relative.clone());
        paths.insert(id, new_path);
        changes.push(WorkspacePathChange {
            source_node_id: Some(id),
            node_id: id,
            old_path: Some(relative_string(&context.root, &record.path)?),
            new_path: new_relative,
        });
    }
    let mut previews = Vec::new();
    if rewrite_links {
        let (writes, document_previews) = plan_link_rewrites(context, &locators, &paths)?;
        steps.extend(writes);
        previews = document_previews;
    }
    let mut plan = finalize_plan(context, action, changes, previews, generated, steps);
    attach_active_branch_scope(
        &mut plan,
        context,
        node.id.ok_or(WorkspaceTransactionError::InvalidWorkspace)?,
        WorkspaceIdentityPolicy::Preserve,
        [parent
            .id
            .ok_or(WorkspaceTransactionError::InvalidWorkspace)?],
        0,
        None,
    )?;
    Ok(plan)
}

fn plan_link_rewrites(
    context: &PlanningContext,
    new_locators: &BTreeMap<NodeId, String>,
    new_paths: &BTreeMap<NodeId, PathBuf>,
) -> Result<(Vec<PlannedStep>, Vec<WorkspaceDocumentChange>), WorkspaceTransactionError> {
    let index =
        build_workspace_link_index(&context.root).map_err(WorkspaceTransactionError::LinkIndex)?;
    let mut edits = BTreeMap::<NodeId, Vec<DocumentEdit>>::new();
    for link in &index.outgoing {
        if NodeId::from_str(&link.authored_locator).is_ok() {
            continue;
        }
        let affected = link
            .target_node_ids
            .iter()
            .filter(|id| new_locators.contains_key(id))
            .copied()
            .collect::<Vec<_>>();
        if affected.is_empty() {
            continue;
        }
        if link.target_node_ids.len() != 1 {
            return Err(WorkspaceTransactionError::AmbiguousAffectedLink {
                source: link.source_node_id,
                start: link.start,
            });
        }
        let replacement = &new_locators[&affected[0]];
        if &link.authored_locator == replacement {
            continue;
        }
        edits
            .entry(link.source_node_id)
            .or_default()
            .push(DocumentEdit {
                start: link.locator_start,
                end: link.locator_end,
                replacement: replacement.clone(),
            });
    }
    let mut steps = Vec::new();
    let mut previews = Vec::new();
    for (source_id, source_edits) in edits {
        let original_path = context
            .index
            .path_for(source_id)
            .ok_or(WorkspaceTransactionError::UnknownNode(source_id))?;
        let snapshot =
            read_node_document(original_path).map_err(WorkspaceTransactionError::Document)?;
        let edit_count = to_u64(source_edits.len());
        let plan = plan_document_edit(original_path, &snapshot.revision, source_edits)
            .map_err(WorkspaceTransactionError::Document)?;
        let final_node_path = new_paths
            .get(&source_id)
            .map_or_else(|| original_path.to_path_buf(), Clone::clone);
        let final_name = final_node_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| WorkspaceTransactionError::PathEscape(final_node_path.clone()))?;
        let destination = relative_string(
            &context.root,
            &generation_document_path(context.generation, &final_node_path, final_name)?,
        )?;
        let next_bytes = plan.next_source().as_bytes().to_vec();
        steps.push(PlannedStep::ReplaceFile {
            destination: destination.clone(),
            base_digest: format!("{:x}", Sha256::digest(snapshot.source.as_bytes())),
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        });
        previews.push(WorkspaceDocumentChange {
            node_id: source_id,
            path: destination,
            base_revision: snapshot.revision,
            next_revision: plan.next_revision,
            edit_count,
        });
    }
    Ok((steps, previews))
}

fn finalize_plan(
    context: &PlanningContext,
    action: StructuralAction,
    mut path_changes: Vec<WorkspacePathChange>,
    mut document_changes: Vec<WorkspaceDocumentChange>,
    mut generated_node_ids: Vec<NodeId>,
    steps: Vec<PlannedStep>,
) -> WorkspaceTransactionPlan {
    path_changes.sort_by(|a, b| {
        a.new_path
            .cmp(&b.new_path)
            .then_with(|| a.node_id.cmp(&b.node_id))
    });
    document_changes.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.node_id.cmp(&b.node_id)));
    generated_node_ids.sort();
    generated_node_ids.dedup();
    let identity_map = if action == StructuralAction::Copy {
        let mut entries = path_changes
            .iter()
            .filter_map(|change| {
                change
                    .source_node_id
                    .map(|source_node_id| WorkspaceIdentityMapEntry {
                        source_node_id,
                        destination_node_id: change.node_id,
                    })
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.source_node_id);
        entries
    } else {
        Vec::new()
    };
    let mut target_node_ids = path_changes
        .iter()
        .filter_map(|change| change.source_node_id)
        .chain(document_changes.iter().map(|change| change.node_id))
        .collect::<Vec<_>>();
    canonicalize_node_ids(&mut target_node_ids);
    let draft_sensitive_node_ids = target_node_ids.clone();
    let captured_target = target_node_ids
        .first()
        .copied()
        .filter(|_| target_node_ids.len() == 1)
        .map(|node_id| WorkspaceCapturedTarget::Node {
            node_id,
            resolved_by: WorkspaceTargetResolution::CallerExplicit,
        });
    WorkspaceTransactionPlan {
        plan_id: NodeId::new_v4().to_string(),
        action,
        workspace_root: context.root.clone(),
        base_revision: context.revision.clone(),
        path_changes,
        document_changes,
        generated_node_ids,
        scope_summary: None,
        promotion_summary: None,
        identity_map,
        captured_target,
        target_node_ids,
        draft_sensitive_node_ids,
        import_authority: None,
        annotation_sidecar_authority: None,
        trash_item_changes: Vec::new(),
        reviewed_trash_request: None,
        legacy_trash_migration_backup: None,
        task_dependencies_authority: None,
        task_promotion_authority: None,
        task_rebaseline_authority: None,
        task_rebaseline_external_snapshot: None,
        task_rebaseline_commit_confirmation: None,
        task_rebaseline_rollback_authority: None,
        task_rebaseline_rollback_commit_confirmation: None,
        steps,
    }
}

fn canonicalize_node_ids(node_ids: &mut Vec<NodeId>) {
    node_ids.sort();
    node_ids.dedup();
}

fn extend_canonical_node_ids(
    node_ids: &mut Vec<NodeId>,
    additional: impl IntoIterator<Item = NodeId>,
) {
    node_ids.extend(additional);
    canonicalize_node_ids(node_ids);
}

fn attach_node_target(plan: &mut WorkspaceTransactionPlan, node_id: NodeId) {
    plan.captured_target = Some(WorkspaceCapturedTarget::Node {
        node_id,
        resolved_by: WorkspaceTargetResolution::CallerExplicit,
    });
    extend_canonical_node_ids(&mut plan.target_node_ids, [node_id]);
}

pub(crate) fn validate_scope_summary(
    summary: &WorkspaceTransactionScopeSummary,
) -> Result<(), WorkspaceTransactionError> {
    let mut affected = summary.affected_document_node_ids.clone();
    canonicalize_node_ids(&mut affected);
    let mut rewritten = summary.rewritten_document_node_ids.clone();
    canonicalize_node_ids(&mut rewritten);
    if summary.root_node.display_name.is_empty()
        || summary.affected_document_node_ids != affected
        || summary.rewritten_document_node_ids != rewritten
        || rewritten
            .iter()
            .any(|node_id| affected.binary_search(node_id).is_err())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "workspace transaction scope summary is not closed and canonical".to_owned(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn validate_plan_scope_authority(
    plan: &WorkspaceTransactionPlan,
) -> Result<(), WorkspaceTransactionError> {
    let mut expected_identity_map = if plan.action == StructuralAction::Copy {
        plan.path_changes
            .iter()
            .filter_map(|change| {
                change
                    .source_node_id
                    .map(|source_node_id| WorkspaceIdentityMapEntry {
                        source_node_id,
                        destination_node_id: change.node_id,
                    })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    expected_identity_map.sort_by_key(|entry| entry.source_node_id);
    if plan.identity_map != expected_identity_map {
        return Err(WorkspaceTransactionError::Metadata(
            "workspace transaction identity map is incomplete or non-canonical".to_owned(),
        ));
    }
    let mut targets = plan.target_node_ids.clone();
    canonicalize_node_ids(&mut targets);
    let mut draft_sensitive = plan.draft_sensitive_node_ids.clone();
    canonicalize_node_ids(&mut draft_sensitive);
    if targets != plan.target_node_ids || draft_sensitive != plan.draft_sensitive_node_ids {
        return Err(WorkspaceTransactionError::Metadata(
            "workspace transaction identity sets are not canonical".to_owned(),
        ));
    }
    if (plan.action == StructuralAction::TaskDependencies)
        != plan.task_dependencies_authority.is_some()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task dependency action lacks its private plan authority".to_owned(),
        ));
    }
    if (plan.action == StructuralAction::TaskPromotion)
        != (plan.task_promotion_authority.is_some() && plan.promotion_summary.is_some())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion action lacks its private plan authority and closed summary".to_owned(),
        ));
    }
    if plan.action != StructuralAction::TaskPromotion
        && (plan.task_promotion_authority.is_some() || plan.promotion_summary.is_some())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "non-promotion action carries task promotion authority".to_owned(),
        ));
    }
    let forward_rebaseline = plan.task_rebaseline_authority.is_some()
        && plan.task_rebaseline_rollback_authority.is_none();
    let rollback_rebaseline = plan.task_rebaseline_authority.is_none()
        && plan.task_rebaseline_rollback_authority.is_some();
    if (plan.action == StructuralAction::TaskRebaseline)
        != (plan.task_rebaseline_external_snapshot.is_some()
            && (forward_rebaseline ^ rollback_rebaseline))
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline action lacks one direction-exclusive physical/snapshot authority"
                .to_owned(),
        ));
    }
    if plan.action != StructuralAction::TaskRebaseline
        && (plan.task_rebaseline_authority.is_some()
            || plan.task_rebaseline_external_snapshot.is_some()
            || plan.task_rebaseline_commit_confirmation.is_some()
            || plan.task_rebaseline_rollback_authority.is_some()
            || plan.task_rebaseline_rollback_commit_confirmation.is_some())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "non-rebaseline action carries task rebaseline authority".to_owned(),
        ));
    }
    match &plan.captured_target {
        Some(WorkspaceCapturedTarget::Node { node_id, .. }) => {
            if targets.binary_search(node_id).is_err() {
                return Err(WorkspaceTransactionError::Metadata(
                    "captured node target is absent from transaction target authority".to_owned(),
                ));
            }
        }
        Some(WorkspaceCapturedTarget::OwnedResource { owner_node_id, .. }) => {
            if targets.binary_search(owner_node_id).is_err() {
                return Err(WorkspaceTransactionError::Metadata(
                    "captured resource owner is absent from transaction target authority"
                        .to_owned(),
                ));
            }
        }
        Some(WorkspaceCapturedTarget::TrashItem { .. }) | None => {}
    }
    let mut required_drafts = plan
        .path_changes
        .iter()
        .filter_map(|change| change.source_node_id)
        .chain(plan.document_changes.iter().map(|change| change.node_id))
        .collect::<Vec<_>>();
    canonicalize_node_ids(&mut required_drafts);
    if required_drafts
        .iter()
        .any(|node_id| draft_sensitive.binary_search(node_id).is_err())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "workspace transaction omits a draft-sensitive identity".to_owned(),
        ));
    }
    if plan.action == StructuralAction::TaskDependencies {
        let exact_task_document_authority = match (
            plan.document_changes.as_slice(),
            plan.steps.as_slice(),
            &plan.captured_target,
        ) {
            (
                [document],
                [PlannedStep::ReplaceFile { destination, .. }],
                Some(WorkspaceCapturedTarget::Node { node_id, .. }),
            ) => {
                document.node_id == *node_id
                    && document.path == *destination
                    && plan.target_node_ids == [*node_id]
                    && plan.draft_sensitive_node_ids == [*node_id]
            }
            _ => false,
        };
        if !exact_task_document_authority
            || !task_dependencies_authority_matches(plan)
            || !plan.path_changes.is_empty()
            || !plan.generated_node_ids.is_empty()
            || plan.scope_summary.is_some()
            || plan.import_authority.is_some()
            || plan.annotation_sidecar_authority.is_some()
            || !plan.trash_item_changes.is_empty()
            || plan.reviewed_trash_request.is_some()
            || plan.legacy_trash_migration_backup.is_some()
        {
            return Err(WorkspaceTransactionError::Metadata(
                "task dependency transaction must contain exactly one captured document replacement"
                    .to_owned(),
            ));
        }
    }
    if plan.action == StructuralAction::TaskPromotion
        && (!promotion_authority_matches(plan)
            || plan.scope_summary.is_some()
            || !plan.identity_map.is_empty()
            || plan.import_authority.is_some()
            || plan.annotation_sidecar_authority.is_some()
            || !plan.trash_item_changes.is_empty()
            || plan.reviewed_trash_request.is_some()
            || plan.legacy_trash_migration_backup.is_some()
            || plan.task_dependencies_authority.is_some())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion transaction differs from its closed private authority".to_owned(),
        ));
    }
    if plan.action == StructuralAction::TaskRebaseline
        && (!(if forward_rebaseline {
            task_rebaseline_authority_matches(plan)
        } else {
            task_rebaseline_rollback_authority_matches(plan)
        }) || plan.scope_summary.is_some()
            || plan.promotion_summary.is_some()
            || !plan.identity_map.is_empty()
            || plan.captured_target.is_some()
            || plan.import_authority.is_some()
            || plan.annotation_sidecar_authority.is_some()
            || !plan.trash_item_changes.is_empty()
            || plan.reviewed_trash_request.is_some()
            || plan.legacy_trash_migration_backup.is_some()
            || plan.task_dependencies_authority.is_some()
            || plan.task_promotion_authority.is_some())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline transaction differs from its closed direction authority".to_owned(),
        ));
    }
    if let Some(summary) = &plan.scope_summary {
        validate_scope_summary(summary)?;
        let expected_policy = match plan.action {
            StructuralAction::Copy => WorkspaceIdentityPolicy::Rekey,
            StructuralAction::Rename
            | StructuralAction::Move
            | StructuralAction::Trash
            | StructuralAction::Restore => WorkspaceIdentityPolicy::Preserve,
            _ => {
                return Err(WorkspaceTransactionError::Metadata(
                    "workspace transaction action cannot carry a node-branch scope summary"
                        .to_owned(),
                ));
            }
        };
        if summary.identity_policy != expected_policy {
            return Err(WorkspaceTransactionError::Metadata(
                "workspace transaction identity policy disagrees with its action".to_owned(),
            ));
        }
        match (&plan.captured_target, plan.action) {
            (Some(WorkspaceCapturedTarget::TrashItem { .. }), StructuralAction::Restore) => {}
            (
                Some(WorkspaceCapturedTarget::Node { node_id, .. }),
                StructuralAction::Rename
                | StructuralAction::Move
                | StructuralAction::Copy
                | StructuralAction::Trash,
            ) if *node_id == summary.root_node.node_id => {}
            _ => {
                return Err(WorkspaceTransactionError::Metadata(
                    "captured target disagrees with the node-branch scope root".to_owned(),
                ));
            }
        }
        let mut rewritten = if plan.action == StructuralAction::Copy {
            plan.path_changes
                .iter()
                .map(|change| change.node_id)
                .collect::<Vec<_>>()
        } else {
            plan.document_changes
                .iter()
                .map(|change| change.node_id)
                .collect::<Vec<_>>()
        };
        canonicalize_node_ids(&mut rewritten);
        if summary.rewritten_document_node_ids != rewritten {
            return Err(WorkspaceTransactionError::Metadata(
                "workspace transaction rewritten-document scope disagrees with its plan".to_owned(),
            ));
        }
        let mut affected = plan
            .path_changes
            .iter()
            .map(|change| change.node_id)
            .chain(rewritten.iter().copied())
            .collect::<Vec<_>>();
        canonicalize_node_ids(&mut affected);
        if summary.affected_document_node_ids != affected
            || summary.descendant_node_count
                != to_u64(summary.affected_document_node_ids.len().saturating_sub(1))
                    .saturating_sub(to_u64(
                        rewritten
                            .iter()
                            .filter(|node_id| {
                                !plan
                                    .path_changes
                                    .iter()
                                    .any(|change| change.node_id == **node_id)
                            })
                            .count(),
                    ))
        {
            return Err(WorkspaceTransactionError::Metadata(
                "workspace transaction affected-document scope disagrees with its path changes"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn promotion_authority_matches(plan: &WorkspaceTransactionPlan) -> bool {
    let (Some(summary), Some(authority)) = (
        plan.promotion_summary.as_ref(),
        plan.task_promotion_authority.as_ref(),
    ) else {
        return false;
    };
    let ([path_change], [document_change]) = (
        plan.path_changes.as_slice(),
        plan.document_changes.as_slice(),
    ) else {
        return false;
    };
    let mut expected_targets = vec![summary.source_node_id, summary.generated_parent_node_id];
    canonicalize_node_ids(&mut expected_targets);
    if authority.source_node_id != summary.source_node_id
        || authority.generated_node_id != summary.generated_node_id
        || authority.parent_node_id != summary.generated_parent_node_id
        || path_change.source_node_id.is_some()
        || path_change.node_id != summary.generated_node_id
        || path_change.old_path.is_some()
        || path_change.new_path != summary.generated_path
        || Path::new(&summary.generated_path)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(summary.generated_portable_name.as_str())
        || (authority.disclosure == TaskPromotionDisclosure::Owner
            && authority.destination_node_path != summary.generated_path)
        || document_change.node_id != summary.source_node_id
        || document_change.path != authority.source_document_path
        || document_change.base_revision != summary.source_revision
        || document_change.next_revision != summary.next_source_revision
        || document_change.edit_count != 1
        || plan.generated_node_ids != [summary.generated_node_id]
        || plan.target_node_ids != expected_targets
        || plan.draft_sensitive_node_ids != [summary.source_node_id]
        || !matches!(
            plan.captured_target,
            Some(WorkspaceCapturedTarget::Node { node_id, .. }) if node_id == summary.source_node_id
        )
        || summary.affected_document_node_ids.len() != 2
        || !summary
            .affected_document_node_ids
            .contains(&summary.source_node_id)
        || !summary
            .affected_document_node_ids
            .contains(&summary.generated_node_id)
        || authority.source_base_digest != summary.source_revision.as_str()
        || authority.source_next_digest != summary.next_source_revision.as_str()
        || !authority.annotation_replica_completeness.is_complete()
        || authority.annotation_replica_completeness != summary.annotations.replica_completeness
        || promotion_expected_sidecar_state(&authority.expected_source_sidecar)
            != summary.annotations.expected_source_sidecar
        || authority.task_document_path
            != format!(
                "{}/{}.adoc",
                authority.destination_node_path, summary.generated_portable_name
            )
        || authority.task_sidecar_path
            != format!(
                "{}/{}",
                authority.destination_node_path, ANNOTATIONS_FILE_NAME
            )
    {
        return false;
    }
    let [
        PlannedStep::CreateTree {
            destination,
            payload,
        },
        PlannedStep::ReplaceFile {
            destination: source_destination,
            base_digest,
            next_digest,
            next_bytes,
        },
        tail @ ..,
    ] = plan.steps.as_slice()
    else {
        return false;
    };
    if destination != &authority.destination_node_path
        || source_destination != &authority.source_document_path
        || base_digest != &authority.source_base_digest
        || next_digest != &authority.source_next_digest
        || digest_bytes(next_bytes) != authority.source_next_digest
        || DocumentRevision::from_source(std::str::from_utf8(next_bytes).unwrap_or(""))
            != summary.next_source_revision
        || !payload.directories.is_empty()
        || payload.digest != payload_digest(&payload.directories, &payload.files)
        || payload.digest != authority.task_payload_digest
    {
        return false;
    }
    let task_document = payload
        .files
        .iter()
        .find(|file| file.path == format!("{}.adoc", summary.generated_portable_name));
    let Some(task_document) = task_document else {
        return false;
    };
    let Some(task_source) = std::str::from_utf8(&task_document.bytes).ok() else {
        return false;
    };
    let metadata = crate::parse_node_metadata(task_source).ok();
    let profile = crate::analyze_task_node_profile(task_source, Some(summary.generated_node_id));
    if digest_bytes(&task_document.bytes) != authority.task_document_digest
        || metadata.and_then(|metadata| metadata.id) != Some(summary.generated_node_id)
        || !profile.diagnostics.is_empty()
        || profile.title.as_ref().map(|title| title.title.as_str())
            != Some(summary.generated_title.as_str())
        || profile.profile.as_ref().map(|profile| profile.state) != Some(summary.initial_state)
        || profile
            .profile
            .is_none_or(|profile| profile.closed.is_some() || !profile.depends_on.is_empty())
    {
        return false;
    }
    let task_sidecar = payload
        .files
        .iter()
        .find(|file| file.path == ANNOTATIONS_FILE_NAME);
    if task_sidecar.map(|file| digest_bytes(&file.bytes)) != authority.task_sidecar_digest
        || payload.files.len() != usize::from(authority.task_sidecar_digest.is_some()) + 1
    {
        return false;
    }
    match (&authority.source_sidecar_next_digest, tail) {
        (None, []) => true,
        (
            Some(expected_next),
            [
                PlannedStep::ReplaceFile {
                    destination,
                    base_digest,
                    next_digest,
                    next_bytes,
                },
            ],
        ) => {
            destination == &authority.source_sidecar_path
                && matches!(
                    &authority.expected_source_sidecar,
                    TaskPromotionSidecarState::Present { sha256 } if sha256 == base_digest
                )
                && expected_next == next_digest
                && digest_bytes(next_bytes) == *expected_next
        }
        _ => false,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the closed v3 authority-to-step correspondence is kept together for auditability"
)]
fn task_rebaseline_authority_matches(plan: &WorkspaceTransactionPlan) -> bool {
    let (Some(authority), Some(snapshot)) = (
        plan.task_rebaseline_authority.as_ref(),
        plan.task_rebaseline_external_snapshot.as_ref(),
    ) else {
        return false;
    };
    if crate::task_rebaseline_transaction::validate_summary(authority).is_err()
        || authority.base_workspace_revision != plan.base_revision
        || authority.physical_pre_state != *snapshot.binding()
        || authority.external_snapshot.root_identity != *snapshot.root_identity()
        || plan.generated_node_ids
            != authority
                .new_nodes
                .iter()
                .map(|node| node.generated_node_id)
                .collect::<Vec<_>>()
        || plan.target_node_ids != authority.draft_sensitive_node_ids
        || plan.draft_sensitive_node_ids != authority.draft_sensitive_node_ids
        || plan.path_changes.len() != authority.new_nodes.len()
        || plan.document_changes.len() != authority.source_replacements.len()
        || plan.steps.len()
            != authority
                .new_nodes
                .len()
                .saturating_add(authority.source_replacements.len())
    {
        return false;
    }
    if let Some(confirmation) = &plan.task_rebaseline_commit_confirmation
        && (confirmation.confirmation_id == authority.owner_confirmation_id
            || confirmation.actor_binding != authority.owner_actor_binding
            || confirmation.authorization_epoch != authority.owner_authorization_epoch)
    {
        return false;
    }
    let path_changes = plan
        .path_changes
        .iter()
        .map(|change| (change.node_id, change))
        .collect::<BTreeMap<_, _>>();
    if authority.new_nodes.iter().any(|node| {
        path_changes
            .get(&node.generated_node_id)
            .is_none_or(|change| {
                change.source_node_id.is_some()
                    || change.old_path.is_some()
                    || change.new_path != node.destination_node_locator
            })
    }) {
        return false;
    }
    let document_changes = plan
        .document_changes
        .iter()
        .map(|change| (change.path.as_str(), change))
        .collect::<BTreeMap<_, _>>();
    if authority.source_replacements.iter().any(|replacement| {
        document_changes
            .get(replacement.document_locator.as_str())
            .is_none_or(|change| {
                change.node_id != replacement.source_node_id
                    || change.base_revision != replacement.base_revision
                    || change.next_revision != replacement.next_revision
            })
    }) {
        return false;
    }
    authority
        .new_nodes
        .iter()
        .zip(&plan.steps)
        .all(|(node, step)| match step {
            PlannedStep::CreateTree {
                destination,
                payload,
            } => {
                destination == &node.destination_node_locator
                    && payload.directories.is_empty()
                    && payload.files.len() == 1
                    && payload.files[0].bytes == node.exact_source.as_bytes()
            }
            _ => false,
        })
        && authority
            .source_replacements
            .iter()
            .zip(&plan.steps[authority.new_nodes.len()..])
            .all(|(replacement, step)| match step {
                PlannedStep::ReplaceFile {
                    destination,
                    base_digest,
                    next_digest,
                    next_bytes,
                } => {
                    destination == &replacement.document_locator
                        && base_digest == replacement.base_revision.as_str()
                        && next_digest == replacement.next_revision.as_str()
                        && next_bytes == replacement.proposed_source.as_bytes()
                }
                _ => false,
            })
}

#[allow(
    clippy::too_many_lines,
    reason = "the closed v4 rollback authority-to-step correspondence is kept together for auditability"
)]
fn task_rebaseline_rollback_authority_matches(plan: &WorkspaceTransactionPlan) -> bool {
    let (Some(authority), Some(snapshot)) = (
        plan.task_rebaseline_rollback_authority.as_ref(),
        plan.task_rebaseline_external_snapshot.as_ref(),
    ) else {
        return false;
    };
    let forward = &authority.forward_authority;
    if crate::task_rebaseline_transaction::validate_rollback_summary(authority).is_err()
        || plan.task_rebaseline_authority.is_some()
        || plan.task_rebaseline_commit_confirmation.is_some()
        || authority.base_workspace_revision != plan.base_revision
        || authority.external_snapshot.physical_inventory != *snapshot.binding()
        || authority.external_snapshot.root_identity != *snapshot.root_identity()
        || !plan.generated_node_ids.is_empty()
        || !plan.path_changes.is_empty()
        || plan.target_node_ids != authority.draft_sensitive_node_ids
        || plan.draft_sensitive_node_ids != authority.draft_sensitive_node_ids
        || plan.document_changes.len() != forward.source_replacements.len()
        || plan.steps.len()
            != forward
                .source_replacements
                .len()
                .saturating_add(forward.new_nodes.len())
    {
        return false;
    }
    if let Some(confirmation) = &plan.task_rebaseline_rollback_commit_confirmation
        && (confirmation.confirmation_id == authority.rollback_confirmation_id
            || confirmation.confirmation_id == forward.owner_confirmation_id
            || confirmation.confirmation_id
                == authority
                    .forward_committed_evidence
                    .forward_commit_confirmation_id
            || confirmation.actor_binding != authority.owner_actor_binding
            || confirmation.authorization_epoch != authority.owner_authorization_epoch)
    {
        return false;
    }
    let documents = plan
        .document_changes
        .iter()
        .map(|change| (change.path.as_str(), change))
        .collect::<BTreeMap<_, _>>();
    if forward.source_replacements.iter().any(|replacement| {
        documents
            .get(replacement.document_locator.as_str())
            .is_none_or(|change| {
                change.node_id != replacement.source_node_id
                    || change.base_revision != replacement.next_revision
                    || change.next_revision != replacement.base_revision
                    || change.edit_count != 1
            })
    }) {
        return false;
    }
    forward
        .source_replacements
        .iter()
        .zip(&plan.steps)
        .all(|(replacement, step)| match step {
            PlannedStep::ReplaceFile {
                destination,
                base_digest,
                next_digest,
                next_bytes,
            } => {
                destination == &replacement.document_locator
                    && base_digest == replacement.next_revision.as_str()
                    && next_digest == replacement.base_revision.as_str()
                    && next_bytes == replacement.original_source.as_bytes()
            }
            _ => false,
        })
        && forward
            .new_nodes
            .iter()
            .zip(&plan.steps[forward.source_replacements.len()..])
            .all(|(node, step)| match step {
                PlannedStep::RemovePath { source, digest } => {
                    let document_file = Path::new(&node.document_locator)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("");
                    let files = vec![TreeFile {
                        path: document_file.to_owned(),
                        bytes: node.exact_source.as_bytes().to_vec(),
                    }];
                    source == &node.destination_node_locator
                        && digest == &payload_digest(&[], &files)
                }
                _ => false,
            })
}

fn task_dependencies_authority_matches(plan: &WorkspaceTransactionPlan) -> bool {
    let ([document], [step]) = (plan.document_changes.as_slice(), plan.steps.as_slice()) else {
        return false;
    };
    let PlannedStep::ReplaceFile {
        base_digest,
        next_digest,
        next_bytes,
        ..
    } = step
    else {
        return false;
    };
    match &plan.task_dependencies_authority {
        Some(TaskDependenciesPlanAuthority::LegacyInline {
            node_id,
            base_revision,
            next_revision,
            next_source_digest,
        }) => {
            document.node_id == *node_id
                && document.base_revision == *base_revision
                && document.next_revision == *next_revision
                && document.edit_count == 1
                && base_digest == base_revision.as_str()
                && next_digest == next_source_digest
                && format!("{:x}", Sha256::digest(next_bytes)) == *next_source_digest
        }
        Some(TaskDependenciesPlanAuthority::TaskNodeReplacement {
            node_id,
            base_revision,
            next_revision,
            before_depends_on,
            after_depends_on,
            edits,
            next_source_digest,
        }) => {
            let Some(next_source) = std::str::from_utf8(next_bytes).ok() else {
                return false;
            };
            let analysis = crate::analyze_task_node_profile(next_source, Some(*node_id));
            let profile_matches = analysis.diagnostics.is_empty()
                && analysis.profile_revision == *next_revision
                && analysis
                    .profile
                    .is_some_and(|profile| profile.depends_on == *after_depends_on);
            document.node_id == *node_id
                && document.base_revision == *base_revision
                && document.next_revision == *next_revision
                && document.edit_count == 1
                && edits.len() == 1
                && before_depends_on != after_depends_on
                && unique_node_ids(before_depends_on)
                && strictly_sorted_node_ids(after_depends_on)
                && !after_depends_on.contains(node_id)
                && base_digest == base_revision.as_str()
                && next_digest == next_source_digest
                && format!("{:x}", Sha256::digest(next_bytes)) == *next_source_digest
                && DocumentRevision::from_source(next_source) == *next_revision
                && profile_matches
        }
        None => false,
    }
}

fn strictly_sorted_node_ids(node_ids: &[NodeId]) -> bool {
    node_ids.windows(2).all(|pair| pair[0] < pair[1])
}

fn unique_node_ids(node_ids: &[NodeId]) -> bool {
    node_ids.iter().copied().collect::<BTreeSet<_>>().len() == node_ids.len()
}

#[derive(Default)]
struct BranchPhysicalStats {
    resource_count: u64,
    annotation_sidecar_count: u64,
    byte_total: u64,
}

fn active_branch_physical_stats(
    context: &PlanningContext,
    root: &crate::NodeRecord,
    subtree: &[&crate::NodeRecord],
    document_changes: &[WorkspaceDocumentChange],
) -> Result<BranchPhysicalStats, WorkspaceTransactionError> {
    let payload = collect_existing_tree_payload(&root.path)?;
    let canonical_documents = subtree
        .iter()
        .map(|node| {
            node.document_path
                .strip_prefix(&root.path)
                .map_err(|_| WorkspaceTransactionError::PathEscape(node.document_path.clone()))
                .and_then(|path| components_string(path, &node.document_path))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let annotation_sidecars = subtree
        .iter()
        .map(|node| node.path.join(ANNOTATIONS_FILE_NAME))
        .filter(|path| path.is_file())
        .map(|path| {
            path.strip_prefix(&root.path)
                .map_err(|_| WorkspaceTransactionError::PathEscape(path.clone()))
                .and_then(|relative| components_string(relative, &path))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut stats = BranchPhysicalStats {
        resource_count: to_u64(
            payload
                .files
                .iter()
                .filter(|file| {
                    !canonical_documents.contains(&file.path)
                        && !annotation_sidecars.contains(&file.path)
                })
                .count(),
        ),
        annotation_sidecar_count: to_u64(annotation_sidecars.len()),
        byte_total: payload.files.iter().try_fold(0_u64, |total, file| {
            total.checked_add(to_u64(file.bytes.len())).ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "workspace transaction byte scope overflowed".to_owned(),
                )
            })
        })?,
    };
    let subtree_ids = subtree
        .iter()
        .filter_map(|node| node.id)
        .collect::<BTreeSet<_>>();
    for change in document_changes {
        if subtree_ids.contains(&change.node_id) {
            continue;
        }
        let document = context.node(change.node_id)?;
        let metadata =
            fs::symlink_metadata(&document.document_path).map_err(WorkspaceTransactionError::Io)?;
        if linked_or_reparse(&metadata) || !metadata.is_file() {
            return Err(WorkspaceTransactionError::SymlinkUnsupported(
                document.document_path.clone(),
            ));
        }
        stats.byte_total = stats
            .byte_total
            .checked_add(metadata.len())
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "workspace transaction byte scope overflowed".to_owned(),
                )
            })?;
    }
    Ok(stats)
}

fn attach_active_branch_scope(
    plan: &mut WorkspaceTransactionPlan,
    context: &PlanningContext,
    root_node_id: NodeId,
    identity_policy: WorkspaceIdentityPolicy,
    destination_node_ids: impl IntoIterator<Item = NodeId>,
    trash_item_count: u64,
    operation_id: Option<crate::TrashOperationId>,
) -> Result<(), WorkspaceTransactionError> {
    let root = context.node(root_node_id)?;
    let subtree = subtree_nodes(context, &root.path);
    let source_node_ids = subtree
        .iter()
        .map(|node| node.id.ok_or(WorkspaceTransactionError::InvalidWorkspace))
        .collect::<Result<Vec<_>, _>>()?;
    let stats = active_branch_physical_stats(context, root, &subtree, &plan.document_changes)?;
    let mut rewritten_document_node_ids = plan
        .document_changes
        .iter()
        .map(|change| change.node_id)
        .collect::<Vec<_>>();
    if identity_policy == WorkspaceIdentityPolicy::Rekey {
        rewritten_document_node_ids.extend(plan.path_changes.iter().map(|change| change.node_id));
    }
    canonicalize_node_ids(&mut rewritten_document_node_ids);
    let mut affected_document_node_ids = if identity_policy == WorkspaceIdentityPolicy::Rekey {
        plan.path_changes
            .iter()
            .map(|change| change.node_id)
            .collect::<Vec<_>>()
    } else {
        source_node_ids.clone()
    };
    affected_document_node_ids.extend(rewritten_document_node_ids.iter().copied());
    canonicalize_node_ids(&mut affected_document_node_ids);
    let summary = WorkspaceTransactionScopeSummary {
        root_node: WorkspaceScopeRootNode {
            node_id: root_node_id,
            display_name: root.name.clone(),
        },
        descendant_node_count: to_u64(source_node_ids.len().saturating_sub(1)),
        resource_count: stats.resource_count,
        annotation_sidecar_count: stats.annotation_sidecar_count,
        byte_total: stats.byte_total,
        affected_document_node_ids,
        rewritten_document_node_ids,
        identity_policy,
        trash_item_count,
        operation_id,
    };
    validate_scope_summary(&summary)?;
    plan.scope_summary = Some(summary);
    plan.captured_target = Some(WorkspaceCapturedTarget::Node {
        node_id: root_node_id,
        resolved_by: WorkspaceTargetResolution::CallerExplicit,
    });
    extend_canonical_node_ids(
        &mut plan.target_node_ids,
        std::iter::once(root_node_id).chain(destination_node_ids),
    );
    extend_canonical_node_ids(
        &mut plan.draft_sensitive_node_ids,
        source_node_ids
            .into_iter()
            .chain(plan.document_changes.iter().map(|change| change.node_id)),
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn attach_restore_branch_scope(
    plan: &mut WorkspaceTransactionPlan,
    context: &PlanningContext,
) -> Result<(), WorkspaceTransactionError> {
    let mut restored_node_ids = Vec::new();
    let mut root_node = None;
    let mut resource_count = 0_u64;
    let mut annotation_sidecar_count = 0_u64;
    let mut byte_total = 0_u64;
    for change in &plan.trash_item_changes {
        let item = context
            .inventory
            .trash_items
            .iter()
            .find(|item| item.manifest.trash_item_id() == change.manifest.trash_item_id())
            .ok_or(WorkspaceTransactionError::UnknownTrashItem(
                change.manifest.trash_item_id(),
            ))?;
        match item.manifest.kind() {
            crate::TrashItemKind::Node => {
                if root_node.is_none() {
                    root_node = Some(WorkspaceScopeRootNode {
                        node_id: item.manifest.node_id().ok_or_else(|| {
                            WorkspaceTransactionError::TrashReconciliation(
                                "node Trash item has no permanent root identity".to_owned(),
                            )
                        })?,
                        display_name: change
                            .destination_name
                            .clone()
                            .unwrap_or_else(|| item.manifest.original_name().to_owned()),
                    });
                }
                restored_node_ids.extend(item.node_locators.keys().copied());
                let payload = collect_existing_tree_payload(&item.payload_path)?;
                let root_name = item.manifest.original_name();
                let mut canonical_documents = BTreeSet::new();
                let mut annotation_sidecars = BTreeSet::new();
                for locator in item.node_locators.values() {
                    let relative_node = locator
                        .strip_prefix(root_name)
                        .and_then(|suffix| suffix.strip_prefix('/').or(Some(suffix)))
                        .ok_or_else(|| {
                            WorkspaceTransactionError::TrashReconciliation(
                                "Trash node locator is outside its item root".to_owned(),
                            )
                        })?;
                    let node_name = locator.rsplit('/').next().unwrap_or(root_name);
                    let document =
                        generation_document_locator(context.generation, relative_node, node_name)?;
                    canonical_documents.insert(document);
                    let sidecar = if relative_node.is_empty() {
                        ANNOTATIONS_FILE_NAME.to_owned()
                    } else {
                        format!("{relative_node}/{ANNOTATIONS_FILE_NAME}")
                    };
                    if payload.files.iter().any(|file| file.path == sidecar) {
                        annotation_sidecars.insert(sidecar);
                    }
                }
                resource_count = resource_count
                    .checked_add(to_u64(
                        payload
                            .files
                            .iter()
                            .filter(|file| {
                                !canonical_documents.contains(&file.path)
                                    && !annotation_sidecars.contains(&file.path)
                            })
                            .count(),
                    ))
                    .ok_or_else(|| {
                        WorkspaceTransactionError::Metadata(
                            "workspace transaction resource scope overflowed".to_owned(),
                        )
                    })?;
                annotation_sidecar_count = annotation_sidecar_count
                    .checked_add(to_u64(annotation_sidecars.len()))
                    .ok_or_else(|| {
                        WorkspaceTransactionError::Metadata(
                            "workspace transaction annotation scope overflowed".to_owned(),
                        )
                    })?;
                byte_total = payload.files.iter().try_fold(byte_total, |total, file| {
                    total.checked_add(to_u64(file.bytes.len())).ok_or_else(|| {
                        WorkspaceTransactionError::Metadata(
                            "workspace transaction byte scope overflowed".to_owned(),
                        )
                    })
                })?;
            }
            crate::TrashItemKind::Resource => {
                let metadata = fs::symlink_metadata(&item.payload_path)
                    .map_err(WorkspaceTransactionError::Io)?;
                if linked_or_reparse(&metadata) || !metadata.is_file() {
                    return Err(WorkspaceTransactionError::SymlinkUnsupported(
                        item.payload_path.clone(),
                    ));
                }
                resource_count = resource_count.checked_add(1).ok_or_else(|| {
                    WorkspaceTransactionError::Metadata(
                        "workspace transaction resource scope overflowed".to_owned(),
                    )
                })?;
                byte_total = byte_total.checked_add(metadata.len()).ok_or_else(|| {
                    WorkspaceTransactionError::Metadata(
                        "workspace transaction byte scope overflowed".to_owned(),
                    )
                })?;
            }
        }
    }
    canonicalize_node_ids(&mut restored_node_ids);
    let Some(root_node) = root_node else {
        return Ok(());
    };
    let mut rewritten_document_node_ids = plan
        .document_changes
        .iter()
        .map(|change| change.node_id)
        .collect::<Vec<_>>();
    canonicalize_node_ids(&mut rewritten_document_node_ids);
    let mut affected_document_node_ids = restored_node_ids.clone();
    affected_document_node_ids.extend(rewritten_document_node_ids.iter().copied());
    canonicalize_node_ids(&mut affected_document_node_ids);
    let summary = WorkspaceTransactionScopeSummary {
        root_node,
        descendant_node_count: to_u64(restored_node_ids.len().saturating_sub(1)),
        resource_count,
        annotation_sidecar_count,
        byte_total,
        affected_document_node_ids,
        rewritten_document_node_ids,
        identity_policy: WorkspaceIdentityPolicy::Preserve,
        trash_item_count: to_u64(plan.trash_item_changes.len()),
        operation_id: None,
    };
    validate_scope_summary(&summary)?;
    plan.scope_summary = Some(summary);
    extend_canonical_node_ids(&mut plan.draft_sensitive_node_ids, restored_node_ids);
    Ok(())
}

fn attach_trash_identity_sets(plan: &mut WorkspaceTransactionPlan, context: &PlanningContext) {
    let mut identities = Vec::new();
    for change in &plan.trash_item_changes {
        identities.extend(change.manifest.node_id());
        identities.extend(change.manifest.original_owner_node_id());
        if let Some(item) = context
            .inventory
            .trash_items
            .iter()
            .find(|item| item.manifest.trash_item_id() == change.manifest.trash_item_id())
        {
            identities.extend(item.node_locators.keys().copied());
        }
        identities.extend(change.destination_node_id);
    }
    extend_canonical_node_ids(&mut plan.target_node_ids, identities.iter().copied());
    extend_canonical_node_ids(&mut plan.draft_sensitive_node_ids, identities);
}

fn finalize_trash_plan(
    context: &PlanningContext,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    generated_node_ids: Vec<NodeId>,
    steps: Vec<PlannedStep>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
) -> WorkspaceTransactionPlan {
    let mut plan = finalize_plan(
        context,
        action,
        path_changes,
        Vec::new(),
        generated_node_ids,
        steps,
    );
    plan.trash_item_changes = trash_item_changes;
    plan
}

fn finalize_reviewed_trash_plan(
    context: &PlanningContext,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    generated_node_ids: Vec<NodeId>,
    steps: Vec<PlannedStep>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    reviewed_action: crate::TrashReviewedAction,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    let mut plan = finalize_trash_plan(
        context,
        action,
        path_changes,
        generated_node_ids,
        steps,
        trash_item_changes,
    );
    attach_trash_identity_sets(&mut plan, context);
    match &reviewed_action {
        crate::TrashReviewedAction::StoreNode {
            node_id,
            operation_id,
            ..
        } => attach_active_branch_scope(
            &mut plan,
            context,
            *node_id,
            WorkspaceIdentityPolicy::Preserve,
            std::iter::empty(),
            1,
            Some(*operation_id),
        )?,
        crate::TrashReviewedAction::Restore { .. } => {
            attach_restore_branch_scope(&mut plan, context)?;
        }
        crate::TrashReviewedAction::StoreResources { resources, .. } if resources.len() == 1 => {
            plan.captured_target = Some(WorkspaceCapturedTarget::OwnedResource {
                owner_node_id: resources[0].owner_node_id,
                name: resources[0].name.clone(),
                resolved_by: WorkspaceTargetResolution::CallerExplicit,
            });
        }
        crate::TrashReviewedAction::PermanentDelete { preview } if preview.items.len() == 1 => {
            plan.captured_target = Some(WorkspaceCapturedTarget::TrashItem {
                trash_item_id: preview.items[0].trash_item_id,
                resolved_by: WorkspaceTargetResolution::CallerExplicit,
            });
        }
        crate::TrashReviewedAction::StoreResources { .. }
        | crate::TrashReviewedAction::MigrateLegacy { .. }
        | crate::TrashReviewedAction::PermanentDelete { .. } => {}
    }
    if let crate::TrashReviewedAction::Restore { trash_item_id, .. } = &reviewed_action {
        plan.captured_target = Some(WorkspaceCapturedTarget::TrashItem {
            trash_item_id: *trash_item_id,
            resolved_by: WorkspaceTargetResolution::CallerExplicit,
        });
    }
    plan.reviewed_trash_request = Some(
        crate::workspace_trash::build_trash_reviewed_request(&context.root, &plan, reviewed_action)
            .map_err(WorkspaceTransactionError::InvalidTrashReviewedRequest)?,
    );
    Ok(plan)
}

pub(crate) fn plan_task_document_transaction(
    root: &Path,
    expected_workspace_revision: &WorkspaceRevision,
    node_id: NodeId,
    base_revision: &DocumentRevision,
    edit: DocumentEdit,
    action: StructuralAction,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    if !matches!(
        action,
        StructuralAction::TaskEdit
            | StructuralAction::TaskRecurrenceCompletion
            | StructuralAction::TaskDependencies
    ) {
        return Err(WorkspaceTransactionError::Metadata(
            "task document transaction requires a task action".to_owned(),
        ));
    }
    let context = PlanningContext::load(root)?;
    require_workspace_revision(expected_workspace_revision, &context.revision)?;
    let node = context.node(node_id)?;
    let document = plan_document_edit(&node.path, base_revision, [edit])
        .map_err(WorkspaceTransactionError::Document)?;
    if !document.changed {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let destination = relative_string(&context.root, &node.document_path)?;
    let base_digest = document.base_revision.as_str().to_owned();
    let next_bytes = document.next_source().as_bytes().to_vec();
    let dependency_authority = (action == StructuralAction::TaskDependencies).then(|| {
        TaskDependenciesPlanAuthority::LegacyInline {
            node_id,
            base_revision: document.base_revision.clone(),
            next_revision: document.next_revision.clone(),
            next_source_digest: format!("{:x}", Sha256::digest(&next_bytes)),
        }
    });
    let mut transaction = finalize_plan(
        &context,
        action,
        Vec::new(),
        vec![WorkspaceDocumentChange {
            node_id,
            path: destination.clone(),
            base_revision: document.base_revision,
            next_revision: document.next_revision,
            edit_count: to_u64(document.edits.len()),
        }],
        Vec::new(),
        vec![PlannedStep::ReplaceFile {
            destination,
            base_digest,
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        }],
    );
    transaction.task_dependencies_authority = dependency_authority;
    let latest = read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(expected_workspace_revision, &latest)?;
    Ok(transaction)
}

pub(crate) fn plan_task_document_transaction_from_document_plan(
    root: &Path,
    expected_workspace_revision: &WorkspaceRevision,
    node_id: NodeId,
    document: &DocumentEditPlan,
    base_source: &str,
    before_depends_on: &[NodeId],
    after_depends_on: &[NodeId],
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    ensure_no_unfinished_transaction(root)?;
    if document.node_id != node_id
        || document.node_directory == root
        || !document.node_directory.starts_with(root)
        || !document.document_path.starts_with(&document.node_directory)
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task document plan does not match one eligible managed node".to_owned(),
        ));
    }
    if !document.changed {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let canonical_after = validate_task_dependency_document_plan(
        document,
        base_source,
        before_depends_on,
        after_depends_on,
    )?;
    let destination = relative_string(root, &document.document_path)?;
    let next_bytes = document.next_source().as_bytes().to_vec();
    let plan_id = NodeId::new_v4().to_string();
    let latest = read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(expected_workspace_revision, &latest)?;
    Ok(WorkspaceTransactionPlan {
        plan_id,
        action: StructuralAction::TaskDependencies,
        workspace_root: root.to_path_buf(),
        base_revision: expected_workspace_revision.clone(),
        path_changes: Vec::new(),
        document_changes: vec![WorkspaceDocumentChange {
            node_id,
            path: destination.clone(),
            base_revision: document.base_revision.clone(),
            next_revision: document.next_revision.clone(),
            edit_count: to_u64(document.edits.len()),
        }],
        generated_node_ids: Vec::new(),
        scope_summary: None,
        promotion_summary: None,
        identity_map: Vec::new(),
        captured_target: Some(WorkspaceCapturedTarget::Node {
            node_id,
            resolved_by: WorkspaceTargetResolution::CallerExplicit,
        }),
        target_node_ids: vec![node_id],
        draft_sensitive_node_ids: vec![node_id],
        import_authority: None,
        annotation_sidecar_authority: None,
        trash_item_changes: Vec::new(),
        reviewed_trash_request: None,
        legacy_trash_migration_backup: None,
        task_dependencies_authority: Some(TaskDependenciesPlanAuthority::TaskNodeReplacement {
            node_id,
            base_revision: document.base_revision.clone(),
            next_revision: document.next_revision.clone(),
            before_depends_on: before_depends_on.to_vec(),
            after_depends_on: canonical_after,
            edits: document.edits.clone(),
            next_source_digest: format!("{:x}", Sha256::digest(&next_bytes)),
        }),
        task_promotion_authority: None,
        task_rebaseline_authority: None,
        task_rebaseline_external_snapshot: None,
        task_rebaseline_commit_confirmation: None,
        task_rebaseline_rollback_authority: None,
        task_rebaseline_rollback_commit_confirmation: None,
        steps: vec![PlannedStep::ReplaceFile {
            destination,
            base_digest: document.base_revision.as_str().to_owned(),
            next_digest: format!("{:x}", Sha256::digest(&next_bytes)),
            next_bytes,
        }],
    })
}

fn validate_task_dependency_document_plan(
    document: &DocumentEditPlan,
    base_source: &str,
    before_depends_on: &[NodeId],
    after_depends_on: &[NodeId],
) -> Result<Vec<NodeId>, WorkspaceTransactionError> {
    let canonical_value = (!after_depends_on.is_empty()).then(|| {
        after_depends_on
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    });
    let expected_source_edit = weftext_asciidoc::plan_document_header_attribute_patch(
        base_source,
        "weftext-task-depends-on",
        canonical_value.as_deref(),
    )
    .map_err(|_| {
        WorkspaceTransactionError::Metadata(
            "task dependency document plan is not one canonical header patch".to_owned(),
        )
    })?
    .ok_or_else(|| {
        WorkspaceTransactionError::Metadata(
            "changing task dependency transaction requires one header patch".to_owned(),
        )
    })?;
    let expected_document_edit = DocumentEdit {
        start: u64::try_from(expected_source_edit.range.start).map_err(|_| {
            WorkspaceTransactionError::Metadata(
                "task dependency header patch range exceeds document authority".to_owned(),
            )
        })?,
        end: u64::try_from(expected_source_edit.range.end).map_err(|_| {
            WorkspaceTransactionError::Metadata(
                "task dependency header patch range exceeds document authority".to_owned(),
            )
        })?,
        replacement: expected_source_edit.replacement.clone(),
    };
    let expected_next_source =
        weftext_asciidoc::SourceEditPlan::new(base_source, vec![expected_source_edit])
            .ok()
            .and_then(|plan| plan.apply(base_source))
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "task dependency header patch cannot be applied exactly".to_owned(),
                )
            })?;
    let before_profile = crate::analyze_task_node_profile(base_source, None)
        .profile
        .ok_or_else(|| {
            WorkspaceTransactionError::Metadata(
                "task dependency base source lacks a local task profile".to_owned(),
            )
        })?;
    if DocumentRevision::from_source(base_source) != document.base_revision
        || document.edits.as_slice() != [expected_document_edit]
        || document.next_source() != expected_next_source
        || before_profile.depends_on != before_depends_on
        || before_depends_on == after_depends_on
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task dependency document plan differs from its canonical replacement authority"
                .to_owned(),
        ));
    }
    let mut canonical_after = after_depends_on.to_vec();
    canonicalize_node_ids(&mut canonical_after);
    if canonical_after != after_depends_on {
        return Err(WorkspaceTransactionError::Metadata(
            "task dependency replacement is not canonical".to_owned(),
        ));
    }
    Ok(canonical_after)
}

#[allow(clippy::too_many_lines)]
pub(crate) fn plan_task_promotion_workspace_transaction(
    material: TaskPromotionWorkspaceMaterial,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    require_complete_annotation_replica(material.annotation_replica_completeness)?;
    ensure_no_unfinished_transaction(&material.root)?;
    let current =
        read_workspace_revision(&material.root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&material.workspace_revision, &current)?;
    if material.source_document_plan.node_id != material.summary.source_node_id
        || material.source_document_plan.base_revision != material.summary.source_revision
        || material.source_document_plan.next_revision != material.summary.next_source_revision
        || material.source_document_plan.edits.len() != 1
        || material.source_document_plan.next_source().is_empty()
        || DocumentRevision::from_source(&material.source_base)
            != material.source_document_plan.base_revision
        || material.source_document_plan.node_directory != material.source_node_directory
        || material.destination_node_directory == material.root
        || material.destination_node_directory == material.source_node_directory
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion source/destination material is inconsistent".to_owned(),
        ));
    }
    let context = PlanningContext::load(&material.root)?;
    require_workspace_revision(&material.workspace_revision, &context.revision)?;
    let source = context.node(material.summary.source_node_id)?;
    let parent = context.node(material.summary.generated_parent_node_id)?;
    reject_trash_parent(&context, parent)?;
    if source.path != material.source_node_directory
        || material.destination_node_directory.parent() != Some(parent.path.as_path())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion parent or source locator is inconsistent".to_owned(),
        ));
    }
    let destination_name = material
        .destination_node_directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            WorkspaceTransactionError::NonUtf8Path(material.destination_node_directory.clone())
        })?;
    validate_node_name(destination_name, false).map_err(WorkspaceTransactionError::Workspace)?;
    if destination_name != material.summary.generated_portable_name {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion portable name differs from its destination".to_owned(),
        ));
    }
    require_managed_destination(&material.destination_node_directory)?;
    require_portable_unicode_destination_available(&material.destination_node_directory)?;
    let destination = relative_string(&material.root, &material.destination_node_directory)?;
    validate_workspace_journal_path(&material.summary.generated_path)?;
    let public_name = Path::new(&material.summary.generated_path)
        .file_name()
        .and_then(|name| name.to_str());
    if public_name != Some(destination_name)
        || (material.disclosure == TaskPromotionDisclosure::Owner
            && destination != material.summary.generated_path)
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion public disclosure differs from its destination authority".to_owned(),
        ));
    }
    let document_file = generation_document_file_name(context.generation, destination_name)?;
    let task_document_path = format!("{destination}/{document_file}");
    let task_metadata = crate::parse_node_metadata(&material.task_document_source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    let task_profile = crate::analyze_task_node_profile(
        &material.task_document_source,
        Some(material.summary.generated_node_id),
    );
    if task_metadata.id != Some(material.summary.generated_node_id)
        || !task_profile.diagnostics.is_empty()
        || task_profile
            .title
            .as_ref()
            .map(|title| title.title.as_str())
            != Some(material.summary.generated_title.as_str())
        || task_profile.profile.as_ref().map(|profile| profile.state)
            != Some(material.summary.initial_state)
        || task_profile
            .profile
            .as_ref()
            .is_none_or(|profile| profile.closed.is_some() || !profile.depends_on.is_empty())
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion destination is not the reviewed closed task profile".to_owned(),
        ));
    }
    let source_document_path = relative_string(&material.root, &source.document_path)?;
    let source_sidecar_path = format!(
        "{}/{}",
        relative_string(&material.root, &source.path)?,
        ANNOTATIONS_FILE_NAME
    );
    let task_sidecar_path = format!("{destination}/{ANNOTATIONS_FILE_NAME}");
    let observed = observe_annotation_sidecar_at_authorized_node(
        &source.path,
        material.summary.source_node_id,
    )?;
    if observed.0 != material.expected_source_sidecar {
        return Err(WorkspaceTransactionError::AnnotationSidecarChanged);
    }
    if material.summary.annotations.replica_completeness != material.annotation_replica_completeness
        || material.summary.annotations.expected_source_sidecar
            != promotion_expected_sidecar_state(&material.expected_source_sidecar)
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion annotation authority differs from its closed summary".to_owned(),
        ));
    }
    if material.source_sidecar_bytes.is_some()
        && !matches!(
            material.expected_source_sidecar,
            TaskPromotionSidecarState::Present { .. }
        )
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion cannot rewrite an absent source sidecar".to_owned(),
        ));
    }
    validate_promotion_sidecar_bytes(
        material.source_sidecar_bytes.as_deref(),
        material.summary.source_node_id,
    )?;
    validate_promotion_sidecar_bytes(
        material.task_sidecar_bytes.as_deref(),
        material.summary.generated_node_id,
    )?;
    let mut task_files = vec![TreeFile {
        path: document_file,
        bytes: material.task_document_source.as_bytes().to_vec(),
    }];
    if let Some(bytes) = &material.task_sidecar_bytes {
        task_files.push(TreeFile {
            path: ANNOTATIONS_FILE_NAME.to_owned(),
            bytes: bytes.clone(),
        });
    }
    task_files.sort_by(|left, right| left.path.cmp(&right.path));
    let task_payload = TreePayload {
        directories: Vec::new(),
        digest: payload_digest(&[], &task_files),
        files: task_files,
    };
    let task_payload_digest = task_payload.digest.clone();
    let source_next_bytes = material
        .source_document_plan
        .next_source()
        .as_bytes()
        .to_vec();
    let mut steps = vec![
        PlannedStep::CreateTree {
            destination: destination.clone(),
            payload: task_payload,
        },
        PlannedStep::ReplaceFile {
            destination: source_document_path.clone(),
            base_digest: material
                .source_document_plan
                .base_revision
                .as_str()
                .to_owned(),
            next_digest: digest_bytes(&source_next_bytes),
            next_bytes: source_next_bytes,
        },
    ];
    let source_sidecar_next_digest = material.source_sidecar_bytes.as_ref().map(|bytes| {
        let next_digest = digest_bytes(bytes);
        if let TaskPromotionSidecarState::Present { sha256 } = &material.expected_source_sidecar {
            steps.push(PlannedStep::ReplaceFile {
                destination: source_sidecar_path.clone(),
                base_digest: sha256.clone(),
                next_digest: next_digest.clone(),
                next_bytes: bytes.clone(),
            });
        }
        next_digest
    });
    let task_document_digest = digest_bytes(material.task_document_source.as_bytes());
    let task_sidecar_digest = material.task_sidecar_bytes.as_deref().map(digest_bytes);
    let authority = TaskPromotionPlanAuthority {
        source_node_id: material.summary.source_node_id,
        generated_node_id: material.summary.generated_node_id,
        parent_node_id: material.summary.generated_parent_node_id,
        source_document_path: source_document_path.clone(),
        destination_node_path: destination.clone(),
        task_document_path,
        source_base_digest: material
            .source_document_plan
            .base_revision
            .as_str()
            .to_owned(),
        source_next_digest: material
            .source_document_plan
            .next_revision
            .as_str()
            .to_owned(),
        task_document_digest,
        task_payload_digest,
        annotation_replica_completeness: material.annotation_replica_completeness,
        source_sidecar_path,
        expected_source_sidecar: material.expected_source_sidecar,
        source_sidecar_next_digest,
        task_sidecar_path,
        task_sidecar_digest,
        disclosure: material.disclosure,
    };
    let mut plan = WorkspaceTransactionPlan {
        plan_id: NodeId::new_v4().to_string(),
        action: StructuralAction::TaskPromotion,
        workspace_root: material.root,
        base_revision: material.workspace_revision,
        path_changes: vec![WorkspacePathChange {
            source_node_id: None,
            node_id: material.summary.generated_node_id,
            old_path: None,
            new_path: material.summary.generated_path.clone(),
        }],
        document_changes: vec![WorkspaceDocumentChange {
            node_id: material.summary.source_node_id,
            path: source_document_path,
            base_revision: material.source_document_plan.base_revision,
            next_revision: material.source_document_plan.next_revision,
            edit_count: 1,
        }],
        generated_node_ids: vec![material.summary.generated_node_id],
        scope_summary: None,
        promotion_summary: Some(material.summary.clone()),
        identity_map: Vec::new(),
        captured_target: Some(WorkspaceCapturedTarget::Node {
            node_id: material.summary.source_node_id,
            resolved_by: WorkspaceTargetResolution::CallerExplicit,
        }),
        target_node_ids: vec![
            material.summary.source_node_id,
            material.summary.generated_parent_node_id,
        ],
        draft_sensitive_node_ids: vec![material.summary.source_node_id],
        import_authority: None,
        annotation_sidecar_authority: None,
        trash_item_changes: Vec::new(),
        reviewed_trash_request: None,
        legacy_trash_migration_backup: None,
        task_dependencies_authority: None,
        task_promotion_authority: Some(authority),
        task_rebaseline_authority: None,
        task_rebaseline_external_snapshot: None,
        task_rebaseline_commit_confirmation: None,
        task_rebaseline_rollback_authority: None,
        task_rebaseline_rollback_commit_confirmation: None,
        steps,
    };
    canonicalize_node_ids(&mut plan.target_node_ids);
    canonicalize_node_ids(&mut plan.draft_sensitive_node_ids);
    validate_plan_scope_authority(&plan)?;
    let latest = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&plan.base_revision, &latest)?;
    Ok(plan)
}

fn validate_promotion_sidecar_bytes(
    bytes: Option<&[u8]>,
    node_id: NodeId,
) -> Result<(), WorkspaceTransactionError> {
    let Some(bytes) = bytes else {
        return Ok(());
    };
    let source = std::str::from_utf8(bytes)
        .map_err(|_| WorkspaceTransactionError::Metadata("sidecar is not UTF-8".to_owned()))?;
    let store = AnnotationStore::from_json(source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    store
        .validate(node_id)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if store.to_pretty_json().ok().as_deref() != Some(source) {
        return Err(WorkspaceTransactionError::Metadata(
            "task promotion sidecar is not canonical v3 JSON".to_owned(),
        ));
    }
    Ok(())
}

fn promotion_expected_sidecar_state(
    state: &TaskPromotionSidecarState,
) -> AnnotationSidecarExpectedState {
    match state {
        TaskPromotionSidecarState::Present { sha256 } => AnnotationSidecarExpectedState::Present {
            sha256: sha256.clone(),
        },
        TaskPromotionSidecarState::ConfirmedAbsent => {
            AnnotationSidecarExpectedState::ConfirmedAbsent
        }
    }
}

pub(crate) fn task_promotion_workspace_plans_match(
    reviewed: &WorkspaceTransactionPlan,
    fresh: &WorkspaceTransactionPlan,
) -> bool {
    reviewed.action == StructuralAction::TaskPromotion
        && fresh.action == StructuralAction::TaskPromotion
        && reviewed.workspace_root == fresh.workspace_root
        && reviewed.base_revision == fresh.base_revision
        && reviewed.path_changes == fresh.path_changes
        && reviewed.document_changes == fresh.document_changes
        && reviewed.generated_node_ids == fresh.generated_node_ids
        && reviewed.scope_summary == fresh.scope_summary
        && reviewed.promotion_summary == fresh.promotion_summary
        && reviewed.identity_map == fresh.identity_map
        && reviewed.captured_target == fresh.captured_target
        && reviewed.target_node_ids == fresh.target_node_ids
        && reviewed.draft_sensitive_node_ids == fresh.draft_sensitive_node_ids
        && promotion_authority_matches(reviewed)
        && promotion_authority_matches(fresh)
        && planned_steps_equal(&reviewed.steps, &fresh.steps)
}

#[allow(clippy::too_many_lines)]
#[allow(
    dead_code,
    reason = "pre-release Core primitive is not exported before native Owner authority exists"
)]
pub(crate) fn plan_task_rebaseline_workspace_transaction(
    material: TaskRebaselineWorkspaceMaterial,
    lease: &WorkspaceTransactionLease,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    crate::task_rebaseline_transaction::validate_summary(&material.summary)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    let canonical_root = fs::canonicalize(&material.root).map_err(WorkspaceTransactionError::Io)?;
    if canonical_root != lease.physical_inventory_root()
        || material.summary.physical_pre_state != *material.external_snapshot.binding()
        || material.summary.external_snapshot.root_identity
            != *material.external_snapshot.root_identity()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline root or external snapshot authority is inconsistent".to_owned(),
        ));
    }
    lease.validate_anchor_identity()?;

    let mut steps = Vec::new();
    let mut path_changes = Vec::new();
    let mut document_changes = Vec::new();
    let mut generated_node_ids = Vec::new();
    for node in &material.summary.new_nodes {
        validate_workspace_journal_path(&node.destination_node_locator)?;
        validate_workspace_journal_path(&node.document_locator)?;
        let destination = safe_join(&canonical_root, &node.destination_node_locator)?;
        require_managed_destination(&destination)?;
        require_portable_unicode_destination_available(&destination)?;
        let expected_document = format!(
            "{}/{}.adoc",
            node.destination_node_locator,
            Path::new(&node.destination_node_locator)
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| WorkspaceTransactionError::Metadata(
                    "task rebaseline destination has no portable name".to_owned()
                ))?
        );
        let metadata = crate::parse_node_metadata(&node.exact_source)
            .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
        let profile =
            crate::analyze_task_node_profile(&node.exact_source, Some(node.generated_node_id));
        if node.document_locator != expected_document
            || metadata.id != Some(node.generated_node_id)
            || !profile.diagnostics.is_empty()
            || profile.profile.is_none()
            || digest_bytes(node.exact_source.as_bytes()) != node.source_sha256
        {
            return Err(WorkspaceTransactionError::Metadata(
                "task rebaseline generated node differs from its closed task authority".to_owned(),
            ));
        }
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "task rebaseline document locator has no portable file".to_owned(),
                )
            })?
            .to_owned();
        let files = vec![TreeFile {
            path: document_file,
            bytes: node.exact_source.as_bytes().to_vec(),
        }];
        let payload = TreePayload {
            directories: Vec::new(),
            digest: payload_digest(&[], &files),
            files,
        };
        steps.push(PlannedStep::CreateTree {
            destination: node.destination_node_locator.clone(),
            payload,
        });
        path_changes.push(WorkspacePathChange {
            source_node_id: None,
            node_id: node.generated_node_id,
            old_path: None,
            new_path: node.destination_node_locator.clone(),
        });
        generated_node_ids.push(node.generated_node_id);
    }
    for replacement in &material.summary.source_replacements {
        validate_workspace_journal_path(&replacement.document_locator)?;
        let destination = safe_join(&canonical_root, &replacement.document_locator)?;
        require_path_digest(&destination, replacement.base_revision.as_str())?;
        steps.push(PlannedStep::ReplaceFile {
            destination: replacement.document_locator.clone(),
            base_digest: replacement.base_revision.as_str().to_owned(),
            next_digest: replacement.next_revision.as_str().to_owned(),
            next_bytes: replacement.proposed_source.as_bytes().to_vec(),
        });
        let proposal_count = material
            .summary
            .reviewed_preview
            .source_previews
            .iter()
            .find(|preview| preview.document_locator == replacement.document_locator)
            .map_or(0, |preview| preview.proposals.len());
        document_changes.push(WorkspaceDocumentChange {
            node_id: replacement.source_node_id,
            path: replacement.document_locator.clone(),
            base_revision: replacement.base_revision.clone(),
            next_revision: replacement.next_revision.clone(),
            edit_count: to_u64(proposal_count),
        });
    }
    path_changes.sort_by(|left, right| left.new_path.cmp(&right.new_path));
    document_changes.sort_by(|left, right| left.path.cmp(&right.path));
    generated_node_ids.sort_unstable();
    if steps.len() > 20_000
        || generated_node_ids.windows(2).any(|pair| pair[0] >= pair[1])
        || material.summary.new_nodes.is_empty()
        || material.summary.source_replacements.is_empty()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline transaction exceeds its closed step/identity bounds".to_owned(),
        ));
    }
    let mut plan = WorkspaceTransactionPlan {
        plan_id: NodeId::new_v4().to_string(),
        action: StructuralAction::TaskRebaseline,
        workspace_root: material.root,
        base_revision: material.summary.base_workspace_revision.clone(),
        path_changes,
        document_changes,
        generated_node_ids,
        scope_summary: None,
        promotion_summary: None,
        identity_map: Vec::new(),
        captured_target: None,
        target_node_ids: material.summary.draft_sensitive_node_ids.clone(),
        draft_sensitive_node_ids: material.summary.draft_sensitive_node_ids.clone(),
        import_authority: None,
        annotation_sidecar_authority: None,
        trash_item_changes: Vec::new(),
        reviewed_trash_request: None,
        legacy_trash_migration_backup: None,
        task_dependencies_authority: None,
        task_promotion_authority: None,
        task_rebaseline_authority: Some(material.summary),
        task_rebaseline_external_snapshot: Some(material.external_snapshot),
        task_rebaseline_commit_confirmation: None,
        task_rebaseline_rollback_authority: None,
        task_rebaseline_rollback_commit_confirmation: None,
        steps,
    };
    canonicalize_node_ids(&mut plan.target_node_ids);
    canonicalize_node_ids(&mut plan.draft_sensitive_node_ids);
    validate_plan_scope_authority(&plan)?;
    lease.validate_anchor_identity()?;
    Ok(plan)
}

#[allow(
    dead_code,
    reason = "pre-release Core primitive is not exported before native Owner authority exists"
)]
pub(crate) fn bind_task_rebaseline_commit_confirmation(
    plan: &mut WorkspaceTransactionPlan,
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
) -> Result<(), WorkspaceTransactionError> {
    let authority = plan.task_rebaseline_authority.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::Metadata(
            "task rebaseline plan lacks reviewed Owner authority".to_owned(),
        )
    })?;
    if plan.action != StructuralAction::TaskRebaseline
        || plan.task_rebaseline_commit_confirmation.is_some()
        || confirmation_id == authority.owner_confirmation_id
        || actor_binding != authority.owner_actor_binding
        || authorization_epoch != authority.owner_authorization_epoch
    {
        return Err(WorkspaceTransactionError::Metadata(
            "fresh task rebaseline Owner confirmation differs from reviewed actor/epoch".to_owned(),
        ));
    }
    plan.task_rebaseline_commit_confirmation = Some(TaskRebaselineCommitConfirmation {
        confirmation_id,
        actor_binding,
        authorization_epoch,
    });
    Ok(())
}

#[allow(
    dead_code,
    clippy::too_many_lines,
    reason = "pre-release exact rollback remains crate-private until native Owner authority exists"
)]
pub(crate) fn plan_task_rebaseline_rollback_workspace_transaction(
    material: TaskRebaselineRollbackWorkspaceMaterial,
    lease: &WorkspaceTransactionLease,
) -> Result<WorkspaceTransactionPlan, WorkspaceTransactionError> {
    crate::task_rebaseline_transaction::validate_rollback_summary(&material.summary)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    let canonical_root = fs::canonicalize(&material.root).map_err(WorkspaceTransactionError::Io)?;
    if canonical_root != lease.physical_inventory_root()
        || material.summary.external_snapshot.physical_inventory
            != *material.external_snapshot.binding()
        || material.summary.external_snapshot.root_identity
            != *material.external_snapshot.root_identity()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline rollback root or exact-A snapshot authority is inconsistent"
                .to_owned(),
        ));
    }
    lease.validate_anchor_identity()?;

    let forward = &material.summary.forward_authority;
    let mut steps = Vec::new();
    let mut document_changes = Vec::new();
    for replacement in &forward.source_replacements {
        validate_workspace_journal_path(&replacement.document_locator)?;
        let destination = safe_join(&canonical_root, &replacement.document_locator)?;
        require_path_digest(&destination, replacement.next_revision.as_str())?;
        steps.push(PlannedStep::ReplaceFile {
            destination: replacement.document_locator.clone(),
            base_digest: replacement.next_revision.as_str().to_owned(),
            next_digest: replacement.base_revision.as_str().to_owned(),
            next_bytes: replacement.original_source.as_bytes().to_vec(),
        });
        document_changes.push(WorkspaceDocumentChange {
            node_id: replacement.source_node_id,
            path: replacement.document_locator.clone(),
            base_revision: replacement.next_revision.clone(),
            next_revision: replacement.base_revision.clone(),
            edit_count: 1,
        });
    }
    for node in &forward.new_nodes {
        validate_workspace_journal_path(&node.destination_node_locator)?;
        validate_workspace_journal_path(&node.document_locator)?;
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::Metadata(
                    "task rebaseline rollback generated document locator is invalid".to_owned(),
                )
            })?;
        let files = vec![TreeFile {
            path: document_file.to_owned(),
            bytes: node.exact_source.as_bytes().to_vec(),
        }];
        let digest = payload_digest(&[], &files);
        require_path_digest(
            &safe_join(&canonical_root, &node.destination_node_locator)?,
            &digest,
        )?;
        steps.push(PlannedStep::RemovePath {
            source: node.destination_node_locator.clone(),
            digest,
        });
    }
    document_changes.sort_by(|left, right| left.path.cmp(&right.path));
    if steps.len() > 20_000
        || forward.new_nodes.is_empty()
        || forward.source_replacements.is_empty()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline rollback exceeds its closed step bounds".to_owned(),
        ));
    }
    let mut plan = WorkspaceTransactionPlan {
        plan_id: NodeId::new_v4().to_string(),
        action: StructuralAction::TaskRebaseline,
        workspace_root: material.root,
        base_revision: material.summary.base_workspace_revision.clone(),
        path_changes: Vec::new(),
        document_changes,
        generated_node_ids: Vec::new(),
        scope_summary: None,
        promotion_summary: None,
        identity_map: Vec::new(),
        captured_target: None,
        target_node_ids: material.summary.draft_sensitive_node_ids.clone(),
        draft_sensitive_node_ids: material.summary.draft_sensitive_node_ids.clone(),
        import_authority: None,
        annotation_sidecar_authority: None,
        trash_item_changes: Vec::new(),
        reviewed_trash_request: None,
        legacy_trash_migration_backup: None,
        task_dependencies_authority: None,
        task_promotion_authority: None,
        task_rebaseline_authority: None,
        task_rebaseline_external_snapshot: Some(material.external_snapshot),
        task_rebaseline_commit_confirmation: None,
        task_rebaseline_rollback_authority: Some(material.summary),
        task_rebaseline_rollback_commit_confirmation: None,
        steps,
    };
    canonicalize_node_ids(&mut plan.target_node_ids);
    canonicalize_node_ids(&mut plan.draft_sensitive_node_ids);
    validate_plan_scope_authority(&plan)?;
    lease.validate_anchor_identity()?;
    Ok(plan)
}

pub(crate) fn bind_task_rebaseline_rollback_commit_confirmation(
    plan: &mut WorkspaceTransactionPlan,
    confirmation_id: NodeId,
    actor_binding: String,
    authorization_epoch: String,
) -> Result<(), WorkspaceTransactionError> {
    let authority = plan
        .task_rebaseline_rollback_authority
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::Metadata(
                "task rebaseline rollback plan lacks reviewed Owner authority".to_owned(),
            )
        })?;
    let forward = &authority.forward_committed_evidence;
    if plan.action != StructuralAction::TaskRebaseline
        || plan.task_rebaseline_rollback_commit_confirmation.is_some()
        || plan.task_rebaseline_commit_confirmation.is_some()
        || confirmation_id == authority.rollback_confirmation_id
        || confirmation_id == authority.forward_authority.owner_confirmation_id
        || confirmation_id == forward.forward_commit_confirmation_id
        || actor_binding != authority.owner_actor_binding
        || authorization_epoch != authority.owner_authorization_epoch
    {
        return Err(WorkspaceTransactionError::Metadata(
            "fresh task rebaseline rollback Owner confirmation differs from reviewed actor/epoch"
                .to_owned(),
        ));
    }
    plan.task_rebaseline_rollback_commit_confirmation =
        Some(TaskRebaselineRollbackCommitConfirmation {
            confirmation_id,
            actor_binding,
            authorization_epoch,
        });
    Ok(())
}

pub(crate) fn task_rebaseline_committed_matches_plan(
    plan: &WorkspaceTransactionPlan,
    committed: &CommittedWorkspaceTransaction,
) -> bool {
    plan.action == StructuralAction::TaskRebaseline
        && plan.task_rebaseline_authority.is_some()
        && plan.task_rebaseline_rollback_authority.is_none()
        && task_rebaseline_authority_matches(plan)
        && committed.plan_id == plan.plan_id
        && committed.action == plan.action
        && committed.base_revision == plan.base_revision
        && committed.path_changes == plan.path_changes
        && committed.scope_summary == plan.scope_summary
        && committed.promotion_summary == plan.promotion_summary
        && committed.identity_map == plan.identity_map
        && committed.captured_target == plan.captured_target
        && committed.target_node_ids == plan.target_node_ids
        && committed.draft_sensitive_node_ids == plan.draft_sensitive_node_ids
        && committed.import_authority == plan.import_authority
}

fn planned_steps_equal(left: &[PlannedStep], right: &[PlannedStep]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| match (left, right) {
                (
                    PlannedStep::CreateTree {
                        destination: left_destination,
                        payload: left_payload,
                    },
                    PlannedStep::CreateTree {
                        destination: right_destination,
                        payload: right_payload,
                    },
                ) => {
                    left_destination == right_destination
                        && left_payload.directories == right_payload.directories
                        && left_payload.digest == right_payload.digest
                        && left_payload.files.len() == right_payload.files.len()
                        && left_payload.files.iter().zip(&right_payload.files).all(
                            |(left, right)| left.path == right.path && left.bytes == right.bytes,
                        )
                }
                (
                    PlannedStep::ReplaceFile {
                        destination: left_destination,
                        base_digest: left_base,
                        next_digest: left_next,
                        next_bytes: left_bytes,
                    },
                    PlannedStep::ReplaceFile {
                        destination: right_destination,
                        base_digest: right_base,
                        next_digest: right_next,
                        next_bytes: right_bytes,
                    },
                ) => {
                    left_destination == right_destination
                        && left_base == right_base
                        && left_next == right_next
                        && left_bytes == right_bytes
                }
                _ => false,
            })
}

fn single_node_payload(
    generation: WorkspaceDocumentGeneration,
    name: &str,
    id: NodeId,
) -> Result<TreePayload, WorkspaceTransactionError> {
    let files = vec![TreeFile {
        path: generation_document_file_name(generation, name)?,
        bytes: new_node_document(id).into_bytes(),
    }];
    Ok(TreePayload {
        directories: Vec::new(),
        digest: payload_digest(&[], &files),
        files,
    })
}

fn node_trash_item_payload(
    node: &crate::NodeRecord,
    manifest: &crate::TrashItemManifest,
) -> Result<TreePayload, WorkspaceTransactionError> {
    let source = collect_existing_tree_payload(&node.path)?;
    let payload_root = format!("{}/{}", crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME, node.name);
    let mut directories = vec![
        crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME.to_owned(),
        payload_root.clone(),
    ];
    directories.extend(
        source
            .directories
            .into_iter()
            .map(|path| format!("{payload_root}/{path}")),
    );
    let mut files = vec![TreeFile {
        path: crate::TRASH_ITEM_MANIFEST_FILE_NAME.to_owned(),
        bytes: crate::workspace_trash::manifest_bytes(manifest)
            .map_err(WorkspaceTransactionError::Json)?,
    }];
    files.extend(source.files.into_iter().map(|file| TreeFile {
        path: format!("{payload_root}/{}", file.path),
        bytes: file.bytes,
    }));
    directories.sort();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(TreePayload {
        digest: payload_digest(&directories, &files),
        directories,
        files,
    })
}

fn resource_trash_item_payload(
    name: &str,
    bytes: Vec<u8>,
    manifest: &crate::TrashItemManifest,
) -> Result<TreePayload, WorkspaceTransactionError> {
    let directories = vec![crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME.to_owned()];
    let mut files = vec![
        TreeFile {
            path: crate::TRASH_ITEM_MANIFEST_FILE_NAME.to_owned(),
            bytes: crate::workspace_trash::manifest_bytes(manifest)
                .map_err(WorkspaceTransactionError::Json)?,
        },
        TreeFile {
            path: format!("{}/{name}", crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME),
            bytes,
        },
    ];
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(TreePayload {
        digest: payload_digest(&directories, &files),
        directories,
        files,
    })
}

fn plan_trash_item_creation(
    context: &PlanningContext,
    trash_item_id: crate::TrashItemId,
    item_payload: TreePayload,
    reviewed_trash_node_id: Option<NodeId>,
) -> Result<(PlannedStep, Vec<NodeId>), WorkspaceTransactionError> {
    let (mut steps, generated) = plan_trash_item_creations(
        context,
        vec![(trash_item_id, item_payload)],
        reviewed_trash_node_id,
    )?;
    let step = steps.pop().ok_or_else(|| {
        WorkspaceTransactionError::VerificationFailed(
            "Trash item creation produced no executable step".to_owned(),
        )
    })?;
    Ok((step, generated))
}

fn plan_trash_item_creations(
    context: &PlanningContext,
    items: Vec<(crate::TrashItemId, TreePayload)>,
    reviewed_trash_node_id: Option<NodeId>,
) -> Result<(Vec<PlannedStep>, Vec<NodeId>), WorkspaceTransactionError> {
    if items.is_empty() {
        return Err(WorkspaceTransactionError::NoChange);
    }
    let unique_ids = items
        .iter()
        .map(|(item_id, _)| *item_id)
        .collect::<BTreeSet<_>>();
    if unique_ids.len() != items.len() {
        return Err(WorkspaceTransactionError::TrashReconciliation(
            "planned Trash item IDs are not unique".to_owned(),
        ));
    }
    let trash_path = context.root.join(TRASH_NODE_NAME);
    let items_path = trash_path.join(crate::TRASH_ITEMS_DIRECTORY_NAME);
    if !trash_path.exists() {
        require_portable_destination_available(&trash_path)?;
        let trash_node_id = reviewed_trash_node_id.unwrap_or_else(NodeId::new_v4);
        let mut directories = vec![crate::TRASH_ITEMS_DIRECTORY_NAME.to_owned()];
        let mut files = vec![TreeFile {
            path: generation_document_file_name(context.generation, TRASH_NODE_NAME)?,
            bytes: new_node_document(trash_node_id).into_bytes(),
        }];
        for (trash_item_id, item_payload) in items {
            let prefixed = prefix_tree_payload(
                item_payload,
                &format!("{}/{}", crate::TRASH_ITEMS_DIRECTORY_NAME, trash_item_id),
            );
            directories.extend(prefixed.directories);
            files.extend(prefixed.files);
        }
        directories.sort();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        return Ok((
            vec![PlannedStep::CreateTree {
                destination: TRASH_NODE_NAME.to_owned(),
                payload: TreePayload {
                    digest: payload_digest(&directories, &files),
                    directories,
                    files,
                },
            }],
            vec![trash_node_id],
        ));
    }
    if reviewed_trash_node_id.is_some() {
        return Err(WorkspaceTransactionError::InvalidTrashReviewedRequest(
            "reviewed request generates a Trash node although the store already exists".to_owned(),
        ));
    }
    if !items_path.exists() {
        let destination = format!("{TRASH_NODE_NAME}/{}", crate::TRASH_ITEMS_DIRECTORY_NAME);
        let path = context.root.join(&destination);
        require_portable_destination_available(&path)?;
        let mut directories = Vec::new();
        let mut files = Vec::new();
        for (trash_item_id, item_payload) in items {
            let prefixed = prefix_tree_payload(item_payload, &trash_item_id.to_string());
            directories.extend(prefixed.directories);
            files.extend(prefixed.files);
        }
        directories.sort();
        files.sort_by(|left, right| left.path.cmp(&right.path));
        return Ok((
            vec![PlannedStep::CreateTree {
                destination,
                payload: TreePayload {
                    digest: payload_digest(&directories, &files),
                    directories,
                    files,
                },
            }],
            Vec::new(),
        ));
    }
    let mut steps = Vec::new();
    for (trash_item_id, item_payload) in items {
        let destination = trash_item_relative_path(trash_item_id);
        require_portable_destination_available(&context.root.join(&destination))?;
        steps.push(PlannedStep::CreateTree {
            destination,
            payload: item_payload,
        });
    }
    Ok((steps, Vec::new()))
}

fn prefix_tree_payload(payload: TreePayload, prefix: &str) -> TreePayload {
    let mut directories = vec![prefix.to_owned()];
    directories.extend(
        payload
            .directories
            .into_iter()
            .map(|path| format!("{prefix}/{path}")),
    );
    let mut files = payload
        .files
        .into_iter()
        .map(|file| TreeFile {
            path: format!("{prefix}/{}", file.path),
            bytes: file.bytes,
        })
        .collect::<Vec<_>>();
    directories.sort();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    TreePayload {
        digest: payload_digest(&directories, &files),
        directories,
        files,
    }
}

fn trash_item_relative_path(trash_item_id: crate::TrashItemId) -> String {
    format!(
        "{TRASH_NODE_NAME}/{}/{trash_item_id}",
        crate::TRASH_ITEMS_DIRECTORY_NAME
    )
}

fn ancestor_node_ids(
    context: &PlanningContext,
    mut node_id: NodeId,
) -> Result<Vec<NodeId>, WorkspaceTransactionError> {
    let mut reversed = Vec::new();
    loop {
        if reversed.contains(&node_id) {
            return Err(WorkspaceTransactionError::InvalidWorkspace);
        }
        reversed.push(node_id);
        let node = context.node(node_id)?;
        let Some(parent) = node.parent_id else {
            break;
        };
        node_id = parent;
    }
    reversed.reverse();
    Ok(reversed)
}

fn copy_subtree_payload(
    generation: WorkspaceDocumentGeneration,
    root_node: &crate::NodeRecord,
    destination_name: &str,
    nodes: &[&crate::NodeRecord],
    copied_documents: &BTreeMap<PathBuf, Vec<u8>>,
    replacements: &BTreeMap<NodeId, NodeId>,
) -> Result<TreePayload, WorkspaceTransactionError> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    collect_copy_entries(
        &root_node.path,
        &root_node.path,
        root_node,
        destination_name,
        nodes,
        copied_documents,
        replacements,
        generation,
        &mut directories,
        &mut files,
    )?;
    directories.sort();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let digest = payload_digest(&directories, &files);
    Ok(TreePayload {
        directories,
        files,
        digest,
    })
}

#[allow(clippy::too_many_arguments)]
fn collect_copy_entries(
    root: &Path,
    directory: &Path,
    root_node: &crate::NodeRecord,
    destination_name: &str,
    nodes: &[&crate::NodeRecord],
    copied_documents: &BTreeMap<PathBuf, Vec<u8>>,
    replacements: &BTreeMap<NodeId, NodeId>,
    generation: WorkspaceDocumentGeneration,
    directories: &mut Vec<String>,
    files: &mut Vec<TreeFile>,
) -> Result<(), WorkspaceTransactionError> {
    let mut entries = fs::read_dir(directory)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(WorkspaceTransactionError::SymlinkUnsupported(path));
        }
        let relative = path
            .strip_prefix(root)
            .map_err(|_| WorkspaceTransactionError::PathEscape(path.clone()))?;
        let mut relative = components_string(relative, &path)?;
        if metadata.is_dir() {
            directories.push(relative);
            collect_copy_entries(
                root,
                &path,
                root_node,
                destination_name,
                nodes,
                copied_documents,
                replacements,
                generation,
                directories,
                files,
            )?;
        } else if metadata.is_file() {
            let mut bytes = fs::read(&path).map_err(WorkspaceTransactionError::Io)?;
            if let Some(node) = nodes.iter().find(|node| node.document_path == path) {
                bytes = copied_documents
                    .get(&path)
                    .cloned()
                    .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
                if node.path == root_node.path && root_node.name != destination_name {
                    relative = generation_document_file_name(generation, destination_name)?;
                }
            } else if path.file_name().and_then(|name| name.to_str()) == Some(ANNOTATIONS_FILE_NAME)
            {
                let owner = nodes
                    .iter()
                    .find(|node| node.path == directory)
                    .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
                let source_id = owner
                    .id
                    .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
                let copied_id = replacements
                    .get(&source_id)
                    .copied()
                    .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
                let copied_source = copied_documents
                    .get(&owner.document_path)
                    .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
                let copied_source = std::str::from_utf8(copied_source).map_err(|_| {
                    WorkspaceTransactionError::InvalidUtf8(owner.document_path.clone())
                })?;
                let mut copied_store = AnnotationStore::from_json(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| WorkspaceTransactionError::InvalidUtf8(path.clone()))?,
                )
                .map_err(annotation_store_validation_error)?;
                copied_store
                    .validate(source_id)
                    .map_err(annotation_store_validation_error)?;
                copied_store.rekey_for_copy(
                    copied_id,
                    DocumentRevision::from_source(copied_source).as_str(),
                );
                bytes = copied_store
                    .to_pretty_json()
                    .map_err(|error| annotation_metadata_error(&error))?
                    .into_bytes();
            }
            files.push(TreeFile {
                path: relative,
                bytes,
            });
        }
    }
    Ok(())
}

fn copy_document_sources(
    context: &PlanningContext,
    copied_root: &crate::NodeRecord,
    destination: &Path,
    nodes: &[&crate::NodeRecord],
    replacements: &BTreeMap<NodeId, NodeId>,
) -> Result<BTreeMap<PathBuf, Vec<u8>>, WorkspaceTransactionError> {
    let link_index =
        build_workspace_link_index(&context.root).map_err(WorkspaceTransactionError::LinkIndex)?;
    let mut new_locators = BTreeMap::new();
    for node in nodes {
        let id = node.id.ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        let inside = node
            .path
            .strip_prefix(&copied_root.path)
            .map_err(|_| WorkspaceTransactionError::PathEscape(node.path.clone()))?;
        new_locators.insert(
            id,
            relative_string(&context.root, &destination.join(inside))?,
        );
    }
    let mut result = BTreeMap::new();
    for node in nodes {
        let id = node.id.ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
        let source =
            fs::read_to_string(&node.document_path).map_err(WorkspaceTransactionError::Io)?;
        let mut copied = replace_node_id(&source, replacements[&id])
            .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
        let mut edits = link_index
            .outgoing
            .iter()
            .filter(|link| link.source_node_id == id && link.target_node_ids.len() == 1)
            .filter_map(|link| {
                let target = link.target_node_ids[0];
                if context.generation == WorkspaceDocumentGeneration::AsciiDocV1 {
                    replacements
                        .get(&target)
                        .map(ToString::to_string)
                        .map(|replacement| (link.locator_start, link.locator_end, replacement))
                } else {
                    new_locators
                        .get(&target)
                        .cloned()
                        .map(|replacement| (link.locator_start, link.locator_end, replacement))
                }
            })
            .collect::<Vec<_>>();
        edits.sort_by_key(|edit| std::cmp::Reverse(edit.0));
        for (start, end, replacement) in edits {
            let start = usize::try_from(start).map_err(|_| {
                WorkspaceTransactionError::VerificationFailed(
                    "copy link offset overflowed".to_owned(),
                )
            })?;
            let end = usize::try_from(end).map_err(|_| {
                WorkspaceTransactionError::VerificationFailed(
                    "copy link offset overflowed".to_owned(),
                )
            })?;
            copied.replace_range(start..end, &replacement);
        }
        result.insert(node.document_path.clone(), copied.into_bytes());
    }
    Ok(result)
}

fn payload_digest(directories: &[String], files: &[TreeFile]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.tree.v1\0");
    for directory in directories {
        hasher.update(b"D\0");
        hasher.update(directory.as_bytes());
        hasher.update([0]);
    }
    for file in files {
        hasher.update(b"F\0");
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(Sha256::digest(&file.bytes));
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[allow(clippy::too_many_lines)]
fn validate_snapshot_restore_tree(
    context: &PlanningContext,
    mut nodes: Vec<WorkspaceRestoreTreeNode>,
) -> Result<(String, Vec<WorkspacePathChange>, TreePayload), WorkspaceTransactionError> {
    if nodes.is_empty() || nodes.len() > MAX_SNAPSHOT_RESTORE_NODES {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore tree must contain 1..={MAX_SNAPSHOT_RESTORE_NODES} nodes"
        )));
    }
    nodes.sort_by(|left, right| left.locator.cmp(&right.locator));
    let mut locators = BTreeMap::new();
    let mut folded_locators = BTreeSet::new();
    let mut node_ids = BTreeSet::new();
    for node in &nodes {
        validate_snapshot_restore_node_locator(&node.locator)?;
        if !folded_locators.insert(node.locator.to_lowercase()) {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "snapshot restore node locators collide by portable case-folding: {}",
                node.locator
            )));
        }
        if !node_ids.insert(node.node_id) {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "duplicate snapshot restore node identity: {}",
                node.node_id
            )));
        }
        if context
            .inventory
            .nodes
            .iter()
            .any(|existing| existing.id == Some(node.node_id))
        {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "snapshot restore identity already exists in the target workspace: {}",
                node.node_id
            )));
        }
        let destination = context.root.join(Path::new(&node.locator));
        require_managed_destination(&destination)?;
        locators.insert(node.locator.as_str(), node.node_id);
    }

    let roots = nodes
        .iter()
        .filter(|node| {
            portable_locator_parent(&node.locator)
                .is_none_or(|parent| !locators.contains_key(parent))
        })
        .collect::<Vec<_>>();
    if roots.len() != 1 {
        return Err(WorkspaceTransactionError::Metadata(
            "snapshot restore nodes must form exactly one connected direct-parent tree".to_owned(),
        ));
    }
    let root_locator = roots[0].locator.clone();
    let root_destination = context.root.join(Path::new(&root_locator));
    let target_parent_path = root_destination
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(root_destination.clone()))?;
    let target_parent = context
        .inventory
        .nodes
        .iter()
        .find(|node| node.path == target_parent_path && node.id.is_some())
        .ok_or(WorkspaceTransactionError::InvalidWorkspace)?;
    reject_trash_parent(context, target_parent)?;
    require_portable_destination_available(&root_destination)?;
    for node in &nodes {
        if node.locator == root_locator {
            continue;
        }
        let parent = portable_locator_parent(&node.locator).ok_or_else(|| {
            WorkspaceTransactionError::Metadata(
                "snapshot restore descendant has no direct parent locator".to_owned(),
            )
        })?;
        if !locators.contains_key(parent) {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "snapshot restore descendant has no selected direct parent: {}",
                node.locator
            )));
        }
    }

    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut occupied = BTreeMap::new();
    let mut path_changes = Vec::new();
    let mut total_bytes = 0_u64;
    for node in nodes {
        let relative_node = snapshot_restore_relative_locator(&root_locator, &node.locator)?;
        if !relative_node.is_empty() {
            insert_snapshot_restore_path(&mut occupied, &relative_node, true)?;
            directories.push(relative_node.clone());
        }
        validate_snapshot_restore_document(&node, context.generation)?;
        total_bytes = add_snapshot_restore_bytes(total_bytes, node.exact_source.len())?;
        let document_locator = join_optional_locator(&relative_node, &node.document_file);
        insert_snapshot_restore_path(&mut occupied, &document_locator, false)?;
        files.push(TreeFile {
            path: document_locator,
            bytes: node.exact_source.into_bytes(),
        });

        if let Some(sidecar) = node.annotation_sidecar {
            validate_import_digest(&sidecar.sha256, "snapshot annotation digest")?;
            if digest_bytes(&sidecar.bytes) != sidecar.sha256 {
                return Err(WorkspaceTransactionError::VerificationFailed(format!(
                    "snapshot annotation digest differs from bytes for node {}",
                    node.node_id
                )));
            }
            let source = std::str::from_utf8(&sidecar.bytes).map_err(|_| {
                WorkspaceTransactionError::Metadata(format!(
                    "snapshot annotation sidecar is not UTF-8 for node {}",
                    node.node_id
                ))
            })?;
            let store = AnnotationStore::from_json(source)
                .map_err(|error| annotation_metadata_error(&error))?;
            store
                .validate(node.node_id)
                .map_err(|error| annotation_metadata_error(&error))?;
            total_bytes = add_snapshot_restore_bytes(total_bytes, sidecar.bytes.len())?;
            let locator = join_optional_locator(&relative_node, ANNOTATIONS_FILE_NAME);
            insert_snapshot_restore_path(&mut occupied, &locator, false)?;
            validate_managed_file_path(
                &context.root,
                &context
                    .root
                    .join(Path::new(&node.locator))
                    .join(ANNOTATIONS_FILE_NAME),
            )
            .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
            files.push(TreeFile {
                path: locator,
                bytes: sidecar.bytes,
            });
        }

        if node.resources.len() > 5_000 {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "snapshot restore resource count exceeds 5000 for node {}",
                node.node_id
            )));
        }
        for resource in node.resources {
            validate_import_resource_locator(&resource.locator, &node.document_file)?;
            validate_import_digest(&resource.sha256, "snapshot resource digest")?;
            if resource.sha256 != digest_bytes(&resource.bytes) {
                return Err(WorkspaceTransactionError::VerificationFailed(format!(
                    "snapshot resource digest differs from bytes: {}/{}",
                    node.locator, resource.locator
                )));
            }
            if resource.bytes.len() > 64 * 1024 * 1024 {
                return Err(WorkspaceTransactionError::Metadata(format!(
                    "snapshot restore resource exceeds 64 MiB: {}/{}",
                    node.locator, resource.locator
                )));
            }
            total_bytes = add_snapshot_restore_bytes(total_bytes, resource.bytes.len())?;
            let locator = join_optional_locator(&relative_node, &resource.locator);
            insert_snapshot_restore_path(&mut occupied, &locator, false)?;
            validate_managed_file_path(
                &context.root,
                &context
                    .root
                    .join(Path::new(&node.locator))
                    .join(&resource.locator),
            )
            .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
            files.push(TreeFile {
                path: locator,
                bytes: resource.bytes,
            });
        }
        path_changes.push(WorkspacePathChange {
            source_node_id: Some(node.node_id),
            node_id: node.node_id,
            old_path: None,
            new_path: node.locator,
        });
    }
    if directories
        .len()
        .saturating_add(files.len())
        .saturating_add(1)
        > MAX_SNAPSHOT_RESTORE_ENTRIES
    {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore tree exceeds {MAX_SNAPSHOT_RESTORE_ENTRIES} entries"
        )));
    }
    directories.sort();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    let digest = payload_digest(&directories, &files);
    Ok((
        root_locator,
        path_changes,
        TreePayload {
            directories,
            files,
            digest,
        },
    ))
}

fn validate_snapshot_restore_node_locator(locator: &str) -> Result<(), WorkspaceTransactionError> {
    let path = Path::new(locator);
    if locator.is_empty() || locator.len() > 512 || components_string(path, path)? != locator {
        return Err(WorkspaceTransactionError::Metadata(
            "snapshot restore node locator is not a canonical portable relative path".to_owned(),
        ));
    }
    for component in locator.split('/') {
        if component.len() > 120 {
            return Err(WorkspaceTransactionError::Metadata(
                "snapshot restore node locator component exceeds 120 UTF-8 bytes".to_owned(),
            ));
        }
        validate_node_name(component, false).map_err(WorkspaceTransactionError::Workspace)?;
    }
    Ok(())
}

fn validate_snapshot_restore_document(
    node: &WorkspaceRestoreTreeNode,
    generation: WorkspaceDocumentGeneration,
) -> Result<(), WorkspaceTransactionError> {
    let name = node.locator.rsplit('/').next().ok_or_else(|| {
        WorkspaceTransactionError::Metadata(
            "snapshot restore node locator has no basename".to_owned(),
        )
    })?;
    let expected_document = generation_document_file_name(generation, name)?;
    if node.document_file != expected_document {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore document does not use canonical {name}/{name}.adoc shape"
        )));
    }
    validate_import_digest(&node.document_sha256, "snapshot document digest")?;
    if digest_bytes(node.exact_source.as_bytes()) != node.document_sha256 {
        return Err(WorkspaceTransactionError::VerificationFailed(format!(
            "snapshot document digest differs from exact source for node {}",
            node.node_id
        )));
    }
    if node.exact_source.len() > 32 * 1024 * 1024 {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore document exceeds 32 MiB for node {}",
            node.node_id
        )));
    }
    let metadata = crate::parse_node_metadata(&node.exact_source)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if metadata.id != Some(node.node_id) {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore document identity differs for node {}",
            node.node_id
        )));
    }
    if metadata.presentation.adjacent_heading_body_explicit {
        return Err(WorkspaceTransactionError::Metadata(
            "adjacent_heading_body is valid only on the existing workspace root".to_owned(),
        ));
    }
    let profile = weftext_asciidoc::analyze(&node.exact_source);
    if profile.diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            weftext_asciidoc::DiagnosticCode::UnclosedFrontmatter
                | weftext_asciidoc::DiagnosticCode::ParserError
                | weftext_asciidoc::DiagnosticCode::UnsupportedProfileSyntax
        )
    }) {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore document does not satisfy the AsciiDoc Profile for node {}",
            node.node_id
        )));
    }
    Ok(())
}

fn add_snapshot_restore_bytes(
    current: u64,
    additional: usize,
) -> Result<u64, WorkspaceTransactionError> {
    let total = current
        .checked_add(u64::try_from(additional).unwrap_or(u64::MAX))
        .ok_or_else(|| {
            WorkspaceTransactionError::Metadata("snapshot restore byte count overflowed".to_owned())
        })?;
    if total > MAX_SNAPSHOT_RESTORE_BYTES {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore tree exceeds {MAX_SNAPSHOT_RESTORE_BYTES} bytes"
        )));
    }
    Ok(total)
}

fn insert_snapshot_restore_path(
    occupied: &mut BTreeMap<String, bool>,
    locator: &str,
    directory: bool,
) -> Result<(), WorkspaceTransactionError> {
    let folded = locator.to_lowercase();
    if let Some(existing_directory) = occupied.insert(folded, directory) {
        let conflict = if existing_directory == directory {
            "duplicate"
        } else {
            "file/directory"
        };
        return Err(WorkspaceTransactionError::Metadata(format!(
            "snapshot restore tree has a portable {conflict} collision at {locator}"
        )));
    }
    Ok(())
}

fn snapshot_restore_relative_locator(
    root: &str,
    locator: &str,
) -> Result<String, WorkspaceTransactionError> {
    if locator == root {
        return Ok(String::new());
    }
    locator
        .strip_prefix(&format!("{root}/"))
        .map(str::to_owned)
        .ok_or_else(|| {
            WorkspaceTransactionError::Metadata(
                "snapshot restore node is outside the selected tree root".to_owned(),
            )
        })
}

fn portable_locator_parent(locator: &str) -> Option<&str> {
    locator.rsplit_once('/').map(|(parent, _)| parent)
}

fn join_optional_locator(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}

fn validate_import_authority(
    authority: &WorkspaceImportAuthority,
) -> Result<(), WorkspaceTransactionError> {
    if authority.proposal_id.is_empty()
        || authority.proposal_id.len() > 128
        || !authority
            .proposal_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(WorkspaceTransactionError::Metadata(
            "import proposal ID is not a bounded portable identifier".to_owned(),
        ));
    }
    validate_import_digest(&authority.proposal_digest, "import proposal digest")
}

fn validate_import_digest(digest: &str, label: &str) -> Result<(), WorkspaceTransactionError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WorkspaceTransactionError::Metadata(format!(
            "{label} must be one lowercase SHA-256 digest"
        )));
    }
    Ok(())
}

fn validate_import_resource_locator(
    locator: &str,
    document_file: &str,
) -> Result<(), WorkspaceTransactionError> {
    let path = Path::new(locator);
    if locator.is_empty() || locator.len() > 512 || components_string(path, path)? != locator {
        return Err(WorkspaceTransactionError::Metadata(
            "import resource locator is not a canonical portable relative path".to_owned(),
        ));
    }
    let components = locator.split('/').collect::<Vec<_>>();
    if components.len() != 1 {
        return Err(WorkspaceTransactionError::Metadata(
            "canonical node resources must be direct files, not implicit content directories"
                .to_owned(),
        ));
    }
    for component in components {
        if component.len() > 120 {
            return Err(WorkspaceTransactionError::Metadata(
                "import resource locator component exceeds 120 UTF-8 bytes".to_owned(),
            ));
        }
        validate_portable_path_component(component, false)
            .map_err(WorkspaceTransactionError::Workspace)?;
        let lower = component.to_ascii_lowercase();
        if lower == ".git"
            || lower == ".weftext-format"
            || lower == ".weftext-rules"
            || lower == ANNOTATIONS_FILE_NAME
            || lower == document_file.to_ascii_lowercase()
            || Path::new(&lower)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            || lower.starts_with(".__weftext-transaction-")
            || lower.starts_with(".__weftext-resource-")
        {
            return Err(WorkspaceTransactionError::Metadata(format!(
                "import resource collides with a reserved path: {locator}"
            )));
        }
    }
    Ok(())
}

fn subtree_nodes<'a>(context: &'a PlanningContext, root: &Path) -> Vec<&'a crate::NodeRecord> {
    let mut result = context
        .inventory
        .nodes
        .iter()
        .filter(|node| node.path.starts_with(root))
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.path.cmp(&b.path));
    result
}

fn reject_root_or_trash(
    context: &PlanningContext,
    node: &crate::NodeRecord,
) -> Result<(), WorkspaceTransactionError> {
    if node.parent_id.is_none() {
        return Err(WorkspaceTransactionError::RootMutationUnsupported);
    }
    if node.path == context.root.join(TRASH_NODE_NAME) {
        return Err(WorkspaceTransactionError::TrashMutationUnsupported);
    }
    Ok(())
}

fn reject_trash_parent(
    context: &PlanningContext,
    parent: &crate::NodeRecord,
) -> Result<(), WorkspaceTransactionError> {
    if parent.path.starts_with(context.root.join(TRASH_NODE_NAME)) {
        return Err(WorkspaceTransactionError::TrashMutationUnsupported);
    }
    Ok(())
}

fn require_destination_available(
    destination: &Path,
    allowed_source: Option<&Path>,
) -> Result<(), WorkspaceTransactionError> {
    if !destination.exists() {
        return Ok(());
    }
    if let Some(source) = allowed_source
        && fs::canonicalize(destination).ok() == fs::canonicalize(source).ok()
    {
        return Ok(());
    }
    Err(WorkspaceTransactionError::DestinationExists(
        destination.to_path_buf(),
    ))
}

fn require_portable_destination_available(
    destination: &Path,
) -> Result<(), WorkspaceTransactionError> {
    require_destination_available(destination, None)?;
    let parent = destination
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(destination.to_path_buf()))?;
    let expected_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(destination.to_path_buf()))?;
    let expected_folded = expected_name.to_lowercase();
    for entry in fs::read_dir(parent).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if name.to_lowercase() == expected_folded {
            return Err(WorkspaceTransactionError::DestinationExists(entry.path()));
        }
    }
    Ok(())
}

fn require_portable_unicode_destination_available(
    destination: &Path,
) -> Result<(), WorkspaceTransactionError> {
    require_destination_available(destination, None)?;
    let parent = destination
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(destination.to_path_buf()))?;
    let expected_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(destination.to_path_buf()))?;
    let expected_key = crate::portable_name::portable_name_collision_key(expected_name);
    for entry in fs::read_dir(parent).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if crate::portable_name::portable_name_collision_key(&name) == expected_key {
            return Err(WorkspaceTransactionError::DestinationExists(entry.path()));
        }
    }
    Ok(())
}

fn relative_string(root: &Path, path: &Path) -> Result<String, WorkspaceTransactionError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| WorkspaceTransactionError::PathEscape(path.to_path_buf()))?;
    components_string(relative, path)
}

fn components_string(
    relative: &Path,
    original: &Path,
) -> Result<String, WorkspaceTransactionError> {
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return Err(WorkspaceTransactionError::PathEscape(
                original.to_path_buf(),
            ));
        };
        parts.push(
            value
                .to_str()
                .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(original.to_path_buf()))?,
        );
    }
    if parts.is_empty() {
        return Err(WorkspaceTransactionError::PathEscape(
            original.to_path_buf(),
        ));
    }
    Ok(parts.join("/"))
}

fn components_string_allow_empty(
    relative: &Path,
    original: &Path,
) -> Result<String, WorkspaceTransactionError> {
    if relative.as_os_str().is_empty() {
        Ok(String::new())
    } else {
        components_string(relative, original)
    }
}

fn current_utc_timestamp() -> Result<String, WorkspaceTransactionError> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| {
            WorkspaceTransactionError::Metadata(
                "system clock is before the Unix epoch; cannot timestamp Trash item".to_owned(),
            )
        })?
        .as_secs();
    let days = i64::try_from(seconds / 86_400).map_err(|_| {
        WorkspaceTransactionError::Metadata("system clock exceeds supported range".to_owned())
    })?;
    let seconds_in_day = seconds % 86_400;
    let (year, month, day) = civil_date_from_unix_days(days);
    let hour = seconds_in_day / 3_600;
    let minute = (seconds_in_day % 3_600) / 60;
    let second = seconds_in_day % 60;
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn civil_date_from_unix_days(days: i64) -> (i64, u64, u64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (
        year,
        u64::try_from(month).unwrap_or_default(),
        u64::try_from(day).unwrap_or_default(),
    )
}

fn file_digest(path: &Path) -> Result<String, WorkspaceTransactionError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(fs::read(path).map_err(WorkspaceTransactionError::Io)?)
    ))
}

fn tree_digest(path: &Path) -> Result<String, WorkspaceTransactionError> {
    if path.is_file() {
        return file_digest(path);
    }
    Ok(collect_existing_tree_payload(path)?.digest)
}

fn collect_existing_tree_payload(path: &Path) -> Result<TreePayload, WorkspaceTransactionError> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    collect_raw_tree_entries(path, path, &mut directories, &mut files)?;
    directories.sort();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let digest = payload_digest(&directories, &files);
    Ok(TreePayload {
        directories,
        files,
        digest,
    })
}

fn collect_raw_tree_entries(
    root: &Path,
    directory: &Path,
    directories: &mut Vec<String>,
    files: &mut Vec<TreeFile>,
) -> Result<(), WorkspaceTransactionError> {
    let mut entries = fs::read_dir(directory)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(WorkspaceTransactionError::SymlinkUnsupported(path));
        }
        let relative = components_string(
            path.strip_prefix(root)
                .map_err(|_| WorkspaceTransactionError::PathEscape(path.clone()))?,
            &path,
        )?;
        if metadata.is_dir() {
            directories.push(relative);
            collect_raw_tree_entries(root, &path, directories, files)?;
        } else if metadata.is_file() {
            files.push(TreeFile {
                path: relative,
                bytes: fs::read(path).map_err(WorkspaceTransactionError::Io)?,
            });
        } else {
            return Err(WorkspaceTransactionError::ContentBoundary(
                "tree contains an unsupported filesystem entry".to_owned(),
            ));
        }
    }
    Ok(())
}

fn ensure_no_unfinished_transaction(root: &Path) -> Result<(), WorkspaceTransactionError> {
    for entry in fs::read_dir(root).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if is_canonical_workspace_transaction_cleanup_name(&name)
            || is_canonical_workspace_transaction_rollback_name(&name)
            || name == WORKSPACE_TRANSACTION_LEASE_FILE_NAME
        {
            continue;
        }
        if is_workspace_transaction_name(&name) {
            return Err(WorkspaceTransactionError::RecoveryRequired(entry.path()));
        }
    }
    Ok(())
}

fn rebase_transaction_scan_error(
    error: WorkspaceTransactionError,
    canonical_root: &Path,
    selected_root: &Path,
) -> WorkspaceTransactionError {
    let rebase = |path: PathBuf| {
        path.strip_prefix(canonical_root)
            .map_or(path.clone(), |relative| selected_root.join(relative))
    };
    match error {
        WorkspaceTransactionError::RecoveryRequired(path) => {
            WorkspaceTransactionError::RecoveryRequired(rebase(path))
        }
        WorkspaceTransactionError::NonUtf8Path(path) => {
            WorkspaceTransactionError::NonUtf8Path(rebase(path))
        }
        other => other,
    }
}

fn is_workspace_transaction_name(name: &str) -> bool {
    name.to_ascii_lowercase()
        .starts_with(WORKSPACE_TRANSACTION_PREFIX)
}

fn is_workspace_transaction_cleanup_name(name: &str) -> bool {
    name.to_ascii_lowercase()
        .starts_with(WORKSPACE_TRANSACTION_CLEANUP_PREFIX)
}

fn is_canonical_workspace_transaction_cleanup_name(name: &str) -> bool {
    let Some(suffix) = name.strip_prefix(WORKSPACE_TRANSACTION_CLEANUP_PREFIX) else {
        return false;
    };
    suffix
        .parse::<NodeId>()
        .is_ok_and(|node_id| node_id.to_string() == suffix)
}

fn is_workspace_transaction_rollback_name(name: &str) -> bool {
    name.to_ascii_lowercase()
        .starts_with(WORKSPACE_TRANSACTION_ROLLBACK_PREFIX)
}

fn is_canonical_workspace_transaction_rollback_name(name: &str) -> bool {
    let Some(suffix) = name
        .strip_prefix(WORKSPACE_TRANSACTION_ROLLBACK_PREFIX)
        .and_then(|suffix| suffix.strip_suffix(".json"))
    else {
        return false;
    };
    suffix
        .parse::<NodeId>()
        .is_ok_and(|node_id| node_id.to_string() == suffix)
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalState {
    Prepared,
    Applying,
    Committed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum JournalFinalization {
    Core,
    ExternalReceipt,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum RolledBackPriorState {
    Prepared,
    Applying,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RolledBackMarker {
    schema: String,
    plan_id: String,
    prior_state: RolledBackPriorState,
    base_revision: WorkspaceRevision,
    import_authority: Option<WorkspaceImportAuthority>,
    journal_authority_digest: String,
    marker_digest: String,
}

#[derive(Clone, Serialize)]
struct Journal {
    schema: String,
    plan_id: String,
    state: JournalState,
    base_revision: WorkspaceRevision,
    committed_revision: Option<WorkspaceRevision>,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    document_changes: Vec<WorkspaceDocumentChange>,
    scope_summary: Option<WorkspaceTransactionScopeSummary>,
    promotion_summary: Option<crate::TaskPromotionSummary>,
    task_promotion_authority: Option<TaskPromotionPlanAuthority>,
    task_rebaseline_authority:
        Option<crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary>,
    task_rebaseline_snapshot_authority: Option<TaskRebaselineJournalSnapshotAuthority>,
    task_rebaseline_commit_confirmation: Option<TaskRebaselineCommitConfirmation>,
    task_rebaseline_direction: Option<TaskRebaselineJournalDirection>,
    task_rebaseline_rollback_authority:
        Option<crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary>,
    task_rebaseline_rollback_commit_confirmation: Option<TaskRebaselineRollbackCommitConfirmation>,
    identity_map: Vec<WorkspaceIdentityMapEntry>,
    captured_target: Option<WorkspaceCapturedTarget>,
    target_node_ids: Vec<NodeId>,
    draft_sensitive_node_ids: Vec<NodeId>,
    import_authority: Option<WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    legacy_trash_migration_backup_authority: Option<crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<String>,
    authority_digest: String,
    lifecycle_digest: String,
    commit_digest: Option<String>,
    steps: Vec<JournalStep>,
}

impl fmt::Debug for Journal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Journal")
            .field("schema", &self.schema)
            .field("plan_id", &self.plan_id)
            .field("state", &self.state)
            .field("base_revision", &self.base_revision)
            .field("committed_revision", &self.committed_revision)
            .field("action", &self.action)
            .field("task_rebaseline_direction", &self.task_rebaseline_direction)
            .field("path_change_count", &self.path_changes.len())
            .field("document_change_count", &self.document_changes.len())
            .field("step_count", &self.steps.len())
            .field("finalization", &self.finalization)
            .field("authority_digest", &self.authority_digest)
            .field("lifecycle_digest", &self.lifecycle_digest)
            .field("commit_digest", &self.commit_digest)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalV1Wire {
    schema: String,
    plan_id: String,
    state: JournalState,
    base_revision: WorkspaceRevision,
    committed_revision: Option<WorkspaceRevision>,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    scope_summary: Option<WorkspaceTransactionScopeSummary>,
    identity_map: Vec<WorkspaceIdentityMapEntry>,
    captured_target: Option<WorkspaceCapturedTarget>,
    target_node_ids: Vec<NodeId>,
    draft_sensitive_node_ids: Vec<NodeId>,
    import_authority: Option<WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    legacy_trash_migration_backup_authority: Option<crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<String>,
    authority_digest: String,
    lifecycle_digest: String,
    commit_digest: Option<String>,
    steps: Vec<JournalStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalV2Wire {
    schema: String,
    plan_id: String,
    state: JournalState,
    base_revision: WorkspaceRevision,
    committed_revision: Option<WorkspaceRevision>,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    document_changes: Vec<WorkspaceDocumentChange>,
    scope_summary: Option<WorkspaceTransactionScopeSummary>,
    promotion_summary: crate::TaskPromotionSummary,
    task_promotion_authority: TaskPromotionPlanAuthority,
    identity_map: Vec<WorkspaceIdentityMapEntry>,
    captured_target: Option<WorkspaceCapturedTarget>,
    target_node_ids: Vec<NodeId>,
    draft_sensitive_node_ids: Vec<NodeId>,
    import_authority: Option<WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    legacy_trash_migration_backup_authority: Option<crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<String>,
    authority_digest: String,
    lifecycle_digest: String,
    commit_digest: Option<String>,
    steps: Vec<JournalStep>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalV3Wire {
    schema: String,
    plan_id: String,
    state: JournalState,
    base_revision: WorkspaceRevision,
    committed_revision: Option<WorkspaceRevision>,
    action: StructuralAction,
    path_changes: Vec<WorkspacePathChange>,
    document_changes: Vec<WorkspaceDocumentChange>,
    scope_summary: Option<WorkspaceTransactionScopeSummary>,
    task_rebaseline_authority: crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary,
    task_rebaseline_snapshot_authority: TaskRebaselineJournalSnapshotAuthority,
    task_rebaseline_commit_confirmation: TaskRebaselineCommitConfirmation,
    identity_map: Vec<WorkspaceIdentityMapEntry>,
    captured_target: Option<WorkspaceCapturedTarget>,
    target_node_ids: Vec<NodeId>,
    draft_sensitive_node_ids: Vec<NodeId>,
    import_authority: Option<WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    legacy_trash_migration_backup_authority: Option<crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<String>,
    authority_digest: String,
    lifecycle_digest: String,
    commit_digest: Option<String>,
    steps: Vec<JournalStep>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct JournalV4Wire {
    schema: String,
    plan_id: String,
    state: JournalState,
    base_revision: WorkspaceRevision,
    committed_revision: Option<WorkspaceRevision>,
    action: StructuralAction,
    direction: TaskRebaselineJournalDirection,
    path_changes: Vec<WorkspacePathChange>,
    document_changes: Vec<WorkspaceDocumentChange>,
    scope_summary: Option<WorkspaceTransactionScopeSummary>,
    task_rebaseline_rollback_authority:
        crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary,
    task_rebaseline_snapshot_authority: TaskRebaselineJournalSnapshotAuthority,
    task_rebaseline_rollback_commit_confirmation: TaskRebaselineRollbackCommitConfirmation,
    identity_map: Vec<WorkspaceIdentityMapEntry>,
    captured_target: Option<WorkspaceCapturedTarget>,
    target_node_ids: Vec<NodeId>,
    draft_sensitive_node_ids: Vec<NodeId>,
    import_authority: Option<WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<AnnotationSidecarPlanAuthority>,
    trash_item_changes: Vec<crate::WorkspaceTrashPlanItemChange>,
    legacy_trash_migration_backup_authority: Option<crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<String>,
    authority_digest: String,
    lifecycle_digest: String,
    commit_digest: Option<String>,
    steps: Vec<JournalStep>,
}

impl Journal {
    fn v1_wire(&self) -> JournalV1Wire {
        JournalV1Wire {
            schema: self.schema.clone(),
            plan_id: self.plan_id.clone(),
            state: self.state,
            base_revision: self.base_revision.clone(),
            committed_revision: self.committed_revision.clone(),
            action: self.action,
            path_changes: self.path_changes.clone(),
            scope_summary: self.scope_summary.clone(),
            identity_map: self.identity_map.clone(),
            captured_target: self.captured_target.clone(),
            target_node_ids: self.target_node_ids.clone(),
            draft_sensitive_node_ids: self.draft_sensitive_node_ids.clone(),
            import_authority: self.import_authority.clone(),
            annotation_sidecar_authority: self.annotation_sidecar_authority.clone(),
            trash_item_changes: self.trash_item_changes.clone(),
            legacy_trash_migration_backup_authority: self
                .legacy_trash_migration_backup_authority
                .clone(),
            finalization: self.finalization,
            external_receipt_destination: self.external_receipt_destination.clone(),
            authority_digest: self.authority_digest.clone(),
            lifecycle_digest: self.lifecycle_digest.clone(),
            commit_digest: self.commit_digest.clone(),
            steps: self.steps.clone(),
        }
    }

    fn v2_wire(&self) -> Result<JournalV2Wire, WorkspaceTransactionError> {
        Ok(JournalV2Wire {
            schema: self.schema.clone(),
            plan_id: self.plan_id.clone(),
            state: self.state,
            base_revision: self.base_revision.clone(),
            committed_revision: self.committed_revision.clone(),
            action: self.action,
            path_changes: self.path_changes.clone(),
            document_changes: self.document_changes.clone(),
            scope_summary: self.scope_summary.clone(),
            promotion_summary: self.promotion_summary.clone().ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v2 promotion journal lacks promotionSummary".to_owned(),
                )
            })?,
            task_promotion_authority: self.task_promotion_authority.clone().ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v2 promotion journal lacks private promotion authority".to_owned(),
                )
            })?,
            identity_map: self.identity_map.clone(),
            captured_target: self.captured_target.clone(),
            target_node_ids: self.target_node_ids.clone(),
            draft_sensitive_node_ids: self.draft_sensitive_node_ids.clone(),
            import_authority: self.import_authority.clone(),
            annotation_sidecar_authority: self.annotation_sidecar_authority.clone(),
            trash_item_changes: self.trash_item_changes.clone(),
            legacy_trash_migration_backup_authority: self
                .legacy_trash_migration_backup_authority
                .clone(),
            finalization: self.finalization,
            external_receipt_destination: self.external_receipt_destination.clone(),
            authority_digest: self.authority_digest.clone(),
            lifecycle_digest: self.lifecycle_digest.clone(),
            commit_digest: self.commit_digest.clone(),
            steps: self.steps.clone(),
        })
    }

    fn v3_wire(&self) -> Result<JournalV3Wire, WorkspaceTransactionError> {
        Ok(JournalV3Wire {
            schema: self.schema.clone(),
            plan_id: self.plan_id.clone(),
            state: self.state,
            base_revision: self.base_revision.clone(),
            committed_revision: self.committed_revision.clone(),
            action: self.action,
            path_changes: self.path_changes.clone(),
            document_changes: self.document_changes.clone(),
            scope_summary: self.scope_summary.clone(),
            task_rebaseline_authority: self.task_rebaseline_authority.clone().ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v3 rebaseline journal lacks private authority".to_owned(),
                )
            })?,
            task_rebaseline_snapshot_authority: self
                .task_rebaseline_snapshot_authority
                .clone()
                .ok_or_else(|| {
                    WorkspaceTransactionError::InvalidJournal(
                        "v3 rebaseline journal lacks external snapshot authority".to_owned(),
                    )
                })?,
            task_rebaseline_commit_confirmation: self
                .task_rebaseline_commit_confirmation
                .clone()
                .ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v3 rebaseline journal lacks fresh Owner confirmation".to_owned(),
                )
            })?,
            identity_map: self.identity_map.clone(),
            captured_target: self.captured_target.clone(),
            target_node_ids: self.target_node_ids.clone(),
            draft_sensitive_node_ids: self.draft_sensitive_node_ids.clone(),
            import_authority: self.import_authority.clone(),
            annotation_sidecar_authority: self.annotation_sidecar_authority.clone(),
            trash_item_changes: self.trash_item_changes.clone(),
            legacy_trash_migration_backup_authority: self
                .legacy_trash_migration_backup_authority
                .clone(),
            finalization: self.finalization,
            external_receipt_destination: self.external_receipt_destination.clone(),
            authority_digest: self.authority_digest.clone(),
            lifecycle_digest: self.lifecycle_digest.clone(),
            commit_digest: self.commit_digest.clone(),
            steps: self.steps.clone(),
        })
    }

    fn v4_wire(&self) -> Result<JournalV4Wire, WorkspaceTransactionError> {
        Ok(JournalV4Wire {
            schema: self.schema.clone(),
            plan_id: self.plan_id.clone(),
            state: self.state,
            base_revision: self.base_revision.clone(),
            committed_revision: self.committed_revision.clone(),
            action: self.action,
            direction: self.task_rebaseline_direction.ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v4 rollback journal lacks mandatory direction".to_owned(),
                )
            })?,
            path_changes: self.path_changes.clone(),
            document_changes: self.document_changes.clone(),
            scope_summary: self.scope_summary.clone(),
            task_rebaseline_rollback_authority: self
                .task_rebaseline_rollback_authority
                .clone()
                .ok_or_else(|| {
                    WorkspaceTransactionError::InvalidJournal(
                        "v4 rollback journal lacks private rollback authority".to_owned(),
                    )
                })?,
            task_rebaseline_snapshot_authority: self
                .task_rebaseline_snapshot_authority
                .clone()
                .ok_or_else(|| {
                    WorkspaceTransactionError::InvalidJournal(
                        "v4 rollback journal lacks external snapshot authority".to_owned(),
                    )
                })?,
            task_rebaseline_rollback_commit_confirmation: self
                .task_rebaseline_rollback_commit_confirmation
                .clone()
                .ok_or_else(|| {
                    WorkspaceTransactionError::InvalidJournal(
                        "v4 rollback journal lacks fresh Owner confirmation".to_owned(),
                    )
                })?,
            identity_map: self.identity_map.clone(),
            captured_target: self.captured_target.clone(),
            target_node_ids: self.target_node_ids.clone(),
            draft_sensitive_node_ids: self.draft_sensitive_node_ids.clone(),
            import_authority: self.import_authority.clone(),
            annotation_sidecar_authority: self.annotation_sidecar_authority.clone(),
            trash_item_changes: self.trash_item_changes.clone(),
            legacy_trash_migration_backup_authority: self
                .legacy_trash_migration_backup_authority
                .clone(),
            finalization: self.finalization,
            external_receipt_destination: self.external_receipt_destination.clone(),
            authority_digest: self.authority_digest.clone(),
            lifecycle_digest: self.lifecycle_digest.clone(),
            commit_digest: self.commit_digest.clone(),
            steps: self.steps.clone(),
        })
    }
}

impl From<JournalV1Wire> for Journal {
    fn from(wire: JournalV1Wire) -> Self {
        Self {
            schema: wire.schema,
            plan_id: wire.plan_id,
            state: wire.state,
            base_revision: wire.base_revision,
            committed_revision: wire.committed_revision,
            action: wire.action,
            path_changes: wire.path_changes,
            document_changes: Vec::new(),
            scope_summary: wire.scope_summary,
            promotion_summary: None,
            task_promotion_authority: None,
            task_rebaseline_authority: None,
            task_rebaseline_snapshot_authority: None,
            task_rebaseline_commit_confirmation: None,
            task_rebaseline_direction: None,
            task_rebaseline_rollback_authority: None,
            task_rebaseline_rollback_commit_confirmation: None,
            identity_map: wire.identity_map,
            captured_target: wire.captured_target,
            target_node_ids: wire.target_node_ids,
            draft_sensitive_node_ids: wire.draft_sensitive_node_ids,
            import_authority: wire.import_authority,
            annotation_sidecar_authority: wire.annotation_sidecar_authority,
            trash_item_changes: wire.trash_item_changes,
            legacy_trash_migration_backup_authority: wire.legacy_trash_migration_backup_authority,
            finalization: wire.finalization,
            external_receipt_destination: wire.external_receipt_destination,
            authority_digest: wire.authority_digest,
            lifecycle_digest: wire.lifecycle_digest,
            commit_digest: wire.commit_digest,
            steps: wire.steps,
        }
    }
}

impl From<JournalV2Wire> for Journal {
    fn from(wire: JournalV2Wire) -> Self {
        Self {
            schema: wire.schema,
            plan_id: wire.plan_id,
            state: wire.state,
            base_revision: wire.base_revision,
            committed_revision: wire.committed_revision,
            action: wire.action,
            path_changes: wire.path_changes,
            document_changes: wire.document_changes,
            scope_summary: wire.scope_summary,
            promotion_summary: Some(wire.promotion_summary),
            task_promotion_authority: Some(wire.task_promotion_authority),
            task_rebaseline_authority: None,
            task_rebaseline_snapshot_authority: None,
            task_rebaseline_commit_confirmation: None,
            task_rebaseline_direction: None,
            task_rebaseline_rollback_authority: None,
            task_rebaseline_rollback_commit_confirmation: None,
            identity_map: wire.identity_map,
            captured_target: wire.captured_target,
            target_node_ids: wire.target_node_ids,
            draft_sensitive_node_ids: wire.draft_sensitive_node_ids,
            import_authority: wire.import_authority,
            annotation_sidecar_authority: wire.annotation_sidecar_authority,
            trash_item_changes: wire.trash_item_changes,
            legacy_trash_migration_backup_authority: wire.legacy_trash_migration_backup_authority,
            finalization: wire.finalization,
            external_receipt_destination: wire.external_receipt_destination,
            authority_digest: wire.authority_digest,
            lifecycle_digest: wire.lifecycle_digest,
            commit_digest: wire.commit_digest,
            steps: wire.steps,
        }
    }
}

impl From<JournalV3Wire> for Journal {
    fn from(wire: JournalV3Wire) -> Self {
        Self {
            schema: wire.schema,
            plan_id: wire.plan_id,
            state: wire.state,
            base_revision: wire.base_revision,
            committed_revision: wire.committed_revision,
            action: wire.action,
            path_changes: wire.path_changes,
            document_changes: wire.document_changes,
            scope_summary: wire.scope_summary,
            promotion_summary: None,
            task_promotion_authority: None,
            task_rebaseline_authority: Some(wire.task_rebaseline_authority),
            task_rebaseline_snapshot_authority: Some(wire.task_rebaseline_snapshot_authority),
            task_rebaseline_commit_confirmation: Some(wire.task_rebaseline_commit_confirmation),
            task_rebaseline_direction: Some(TaskRebaselineJournalDirection::ApplyRebaseline),
            task_rebaseline_rollback_authority: None,
            task_rebaseline_rollback_commit_confirmation: None,
            identity_map: wire.identity_map,
            captured_target: wire.captured_target,
            target_node_ids: wire.target_node_ids,
            draft_sensitive_node_ids: wire.draft_sensitive_node_ids,
            import_authority: wire.import_authority,
            annotation_sidecar_authority: wire.annotation_sidecar_authority,
            trash_item_changes: wire.trash_item_changes,
            legacy_trash_migration_backup_authority: wire.legacy_trash_migration_backup_authority,
            finalization: wire.finalization,
            external_receipt_destination: wire.external_receipt_destination,
            authority_digest: wire.authority_digest,
            lifecycle_digest: wire.lifecycle_digest,
            commit_digest: wire.commit_digest,
            steps: wire.steps,
        }
    }
}

impl From<JournalV4Wire> for Journal {
    fn from(wire: JournalV4Wire) -> Self {
        Self {
            schema: wire.schema,
            plan_id: wire.plan_id,
            state: wire.state,
            base_revision: wire.base_revision,
            committed_revision: wire.committed_revision,
            action: wire.action,
            path_changes: wire.path_changes,
            document_changes: wire.document_changes,
            scope_summary: wire.scope_summary,
            promotion_summary: None,
            task_promotion_authority: None,
            task_rebaseline_authority: None,
            task_rebaseline_snapshot_authority: Some(wire.task_rebaseline_snapshot_authority),
            task_rebaseline_commit_confirmation: None,
            task_rebaseline_direction: Some(wire.direction),
            task_rebaseline_rollback_authority: Some(wire.task_rebaseline_rollback_authority),
            task_rebaseline_rollback_commit_confirmation: Some(
                wire.task_rebaseline_rollback_commit_confirmation,
            ),
            identity_map: wire.identity_map,
            captured_target: wire.captured_target,
            target_node_ids: wire.target_node_ids,
            draft_sensitive_node_ids: wire.draft_sensitive_node_ids,
            import_authority: wire.import_authority,
            annotation_sidecar_authority: wire.annotation_sidecar_authority,
            trash_item_changes: wire.trash_item_changes,
            legacy_trash_migration_backup_authority: wire.legacy_trash_migration_backup_authority,
            finalization: wire.finalization,
            external_receipt_destination: wire.external_receipt_destination,
            authority_digest: wire.authority_digest,
            lifecycle_digest: wire.lifecycle_digest,
            commit_digest: wire.commit_digest,
            steps: wire.steps,
        }
    }
}

struct JournalAuthorityDigestMaterial<'a> {
    schema: &'a str,
    plan_id: &'a str,
    base_revision: &'a WorkspaceRevision,
    action: StructuralAction,
    path_changes: &'a [WorkspacePathChange],
    document_changes: &'a [WorkspaceDocumentChange],
    scope_summary: Option<&'a WorkspaceTransactionScopeSummary>,
    promotion_summary: Option<&'a crate::TaskPromotionSummary>,
    task_promotion_authority: Option<&'a TaskPromotionPlanAuthority>,
    task_rebaseline_authority:
        Option<&'a crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary>,
    task_rebaseline_snapshot_authority: Option<&'a TaskRebaselineJournalSnapshotAuthority>,
    task_rebaseline_commit_confirmation: Option<&'a TaskRebaselineCommitConfirmation>,
    task_rebaseline_direction: Option<TaskRebaselineJournalDirection>,
    task_rebaseline_rollback_authority:
        Option<&'a crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary>,
    task_rebaseline_rollback_commit_confirmation:
        Option<&'a TaskRebaselineRollbackCommitConfirmation>,
    identity_map: &'a [WorkspaceIdentityMapEntry],
    captured_target: Option<&'a WorkspaceCapturedTarget>,
    target_node_ids: &'a [NodeId],
    draft_sensitive_node_ids: &'a [NodeId],
    import_authority: Option<&'a WorkspaceImportAuthority>,
    annotation_sidecar_authority: Option<&'a AnnotationSidecarPlanAuthority>,
    trash_item_changes: &'a [crate::WorkspaceTrashPlanItemChange],
    legacy_trash_migration_backup_authority: Option<&'a crate::LegacyTrashMigrationBackupAuthority>,
    finalization: JournalFinalization,
    external_receipt_destination: Option<&'a str>,
    steps: &'a [JournalStep],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum JournalStep {
    CreateTree {
        destination: String,
        staged: String,
        digest: String,
    },
    CreateFile {
        destination: String,
        staged: String,
        next_digest: String,
    },
    MovePath {
        source: String,
        destination: String,
        holding: String,
        digest: String,
    },
    RemovePath {
        source: String,
        holding: String,
        digest: String,
    },
    ReplaceFile {
        destination: String,
        staged: String,
        displaced: String,
        base_digest: String,
        next_digest: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ExternalReceiptClaim {
    schema: String,
    plan_id: String,
    destination: String,
    sha256: String,
    byte_length: u64,
    claim_digest: String,
}

/// An exclusive workspace-wide coordination lease.
///
/// The lease is released when this value is dropped. Its durable, zero-byte
/// anchor is coordination state rather than portable workspace content.
#[derive(Debug)]
#[must_use = "dropping the lease releases workspace transaction exclusion"]
pub struct WorkspaceTransactionLease {
    file: fs::File,
    lease_anchor_identity: same_file::Handle,
    canonical_root: PathBuf,
}

impl WorkspaceTransactionLease {
    pub(crate) fn physical_inventory_root(&self) -> &Path {
        &self.canonical_root
    }

    /// Proves that the root lease path still names this lease's held file.
    ///
    /// Long-running coordinated operations must call this immediately before
    /// reporting success, after their final physical and semantic checks.
    ///
    /// # Errors
    ///
    /// Returns recovery-required when the anchor was removed, renamed, or
    /// replaced, and an I/O error when its identity cannot be reopened.
    pub fn validate_anchor_identity(&self) -> Result<(), WorkspaceTransactionError> {
        let held_identity = same_file::Handle::from_file(
            self.file
                .try_clone()
                .map_err(WorkspaceTransactionError::Io)?,
        )
        .map_err(WorkspaceTransactionError::Io)?;
        if held_identity != self.lease_anchor_identity {
            return Err(WorkspaceTransactionError::RecoveryRequired(
                self.canonical_root
                    .join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
            ));
        }
        let anchor_path = self
            .canonical_root
            .join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
        let current = current_workspace_transaction_lease_anchor_identity(&anchor_path)
            .map_err(WorkspaceTransactionError::Io)?;
        if current.is_some_and(|current| current == self.lease_anchor_identity) {
            Ok(())
        } else {
            Err(WorkspaceTransactionError::RecoveryRequired(anchor_path))
        }
    }
}

/// Acquires the same exclusive lease used by every Core workspace transaction.
///
/// This call creates the durable zero-byte lease anchor when it is absent. It
/// does not validate a transaction journal or workspace revision; callers that
/// coordinate another physical operation must retain the returned value while
/// performing their own checks and commit.
///
/// # Errors
///
/// Returns an error when the root or anchor crosses a link/reparse boundary,
/// the anchor cannot be opened and durably synchronized, or another operation
/// already owns the lease.
pub fn acquire_workspace_transaction_lease(
    root: impl AsRef<Path>,
) -> Result<WorkspaceTransactionLease, WorkspaceTransactionError> {
    acquire_workspace_transaction_lease_platform(root.as_ref())
}

pub(crate) fn acquire_clean_workspace_mutation_guard(
    root: &Path,
) -> Result<WorkspaceTransactionLease, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(root)?;
    ensure_no_unfinished_transaction(root)?;
    Ok(lease)
}

#[cfg(unix)]
fn acquire_workspace_transaction_lease_platform(
    root: &Path,
) -> Result<WorkspaceTransactionLease, WorkspaceTransactionError> {
    use rustix::fs::{FlockOperation, flock};

    let canonical_root = canonical_non_linked_workspace_root(root)?;
    let lock_path = canonical_root.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    reject_linked_existing_ancestors(&lock_path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(WorkspaceTransactionError::Io)?;
    flock(&file, FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| WorkspaceTransactionError::RecoveryRequired(lock_path.clone()))?;
    validate_workspace_transaction_lease_anchor(&file, &lock_path)?;
    let lease_anchor_identity = bind_workspace_transaction_lease_anchor(&file, &lock_path)?;
    file.sync_all().map_err(WorkspaceTransactionError::Io)?;
    sync_directory(&canonical_root)?;
    Ok(WorkspaceTransactionLease {
        file,
        lease_anchor_identity,
        canonical_root,
    })
}

#[cfg(windows)]
fn acquire_workspace_transaction_lease_platform(
    root: &Path,
) -> Result<WorkspaceTransactionLease, WorkspaceTransactionError> {
    use std::os::windows::fs::OpenOptionsExt as _;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    let canonical_root = canonical_non_linked_workspace_root(root)?;
    let lock_path = canonical_root.join(WORKSPACE_TRANSACTION_LEASE_FILE_NAME);
    reject_linked_existing_ancestors(&lock_path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE)
        .open(&lock_path)
        .map_err(|_| WorkspaceTransactionError::RecoveryRequired(lock_path.clone()))?;
    validate_workspace_transaction_lease_anchor(&file, &lock_path)?;
    let lease_anchor_identity = bind_workspace_transaction_lease_anchor(&file, &lock_path)?;
    file.sync_all().map_err(WorkspaceTransactionError::Io)?;
    sync_directory(&canonical_root)?;
    Ok(WorkspaceTransactionLease {
        file,
        lease_anchor_identity,
        canonical_root,
    })
}

fn canonical_non_linked_workspace_root(root: &Path) -> Result<PathBuf, WorkspaceTransactionError> {
    let metadata = fs::symlink_metadata(root).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(WorkspaceTransactionError::SymlinkUnsupported(
            root.to_path_buf(),
        ));
    }
    if !metadata.is_dir() {
        return Err(WorkspaceTransactionError::InvalidWorkspace);
    }
    let canonical_root = fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?;
    reject_linked_existing_ancestors(&canonical_root)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    Ok(canonical_root)
}

fn validate_workspace_transaction_lease_anchor(
    file: &fs::File,
    path: &Path,
) -> Result<(), WorkspaceTransactionError> {
    let metadata = file.metadata().map_err(WorkspaceTransactionError::Io)?;
    if !metadata.is_file() || metadata.len() != 0 {
        return Err(WorkspaceTransactionError::RecoveryRequired(
            path.to_path_buf(),
        ));
    }
    Ok(())
}

fn bind_workspace_transaction_lease_anchor(
    file: &fs::File,
    path: &Path,
) -> Result<same_file::Handle, WorkspaceTransactionError> {
    let held_identity =
        same_file::Handle::from_file(file.try_clone().map_err(WorkspaceTransactionError::Io)?)
            .map_err(WorkspaceTransactionError::Io)?;
    let current_identity = current_workspace_transaction_lease_anchor_identity(path)
        .map_err(WorkspaceTransactionError::Io)?;
    if current_identity.as_ref() != Some(&held_identity) {
        return Err(WorkspaceTransactionError::RecoveryRequired(
            path.to_path_buf(),
        ));
    }
    Ok(held_identity)
}

fn current_workspace_transaction_lease_anchor_identity(
    path: &Path,
) -> io::Result<Option<same_file::Handle>> {
    let valid_metadata = |metadata: &fs::Metadata| {
        metadata.is_file() && metadata.len() == 0 && !linked_or_reparse(metadata)
    };
    let before = match fs::symlink_metadata(path) {
        Ok(metadata) if valid_metadata(&metadata) => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let first = same_file::Handle::from_path(path)?;
    let after = match fs::symlink_metadata(path) {
        Ok(metadata) if valid_metadata(&metadata) => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    if before.len() != after.len() {
        return Ok(None);
    }
    let second = same_file::Handle::from_path(path)?;
    Ok((first == second).then_some(second))
}

/// Commits one previewed structural plan through its recoverable journal.
///
/// # Errors
///
/// Returns an error for a stale workspace revision, staging or verification
/// failure, or recovery ambiguity. A failed applying transaction is rolled back
/// Previews the exact device/session draft intersection for one plan.
///
/// A commit token is issued only when every draft-sensitive UUID is clean.
/// The opaque observation digest binds the plan, required UUID set, and the
/// supplied authoritative registry observation without exposing draft bytes.
///
/// # Errors
///
/// Returns an error for a malformed/non-canonical registry view or plan
/// authority.
pub fn preview_workspace_transaction_draft_gate(
    plan: &WorkspaceTransactionPlan,
    registry: &WorkspaceDraftRegistryView,
) -> Result<WorkspaceDraftGatePreview, WorkspaceTransactionError> {
    validate_plan_scope_authority(plan)?;
    validate_draft_registry_view(registry)?;
    let blocking_dirty_node_ids = plan
        .draft_sensitive_node_ids
        .iter()
        .copied()
        .filter(|node_id| registry.dirty_node_ids.binary_search(node_id).is_ok())
        .collect::<Vec<_>>();
    let observation_digest = draft_registry_observation_digest(plan, registry)?;
    let executable_token = if blocking_dirty_node_ids.is_empty() {
        let mut token = WorkspaceDraftGateToken {
            schema: DRAFT_GATE_SCHEMA.to_owned(),
            plan_id: plan.plan_id.clone(),
            base_revision: plan.base_revision.clone(),
            required_clean_node_ids: plan.draft_sensitive_node_ids.clone(),
            preview_observation_digest: observation_digest.clone(),
            authority_digest: "0".repeat(64),
        };
        token.authority_digest = draft_gate_token_digest(&token)?;
        Some(token)
    } else {
        None
    };
    Ok(WorkspaceDraftGatePreview {
        required_clean_node_ids: plan.draft_sensitive_node_ids.clone(),
        blocking_dirty_node_ids,
        observation_digest,
        executable_token,
    })
}

/// Commits only after rechecking the same exact draft-sensitive set against a
/// fresh authoritative registry observation.
///
/// # Errors
///
/// Returns [`WorkspaceTransactionError::DraftGateBlocked`] without writing if
/// any required node became dirty, or an authority error for a foreign or
/// tampered token.
pub fn commit_workspace_transaction_with_draft_gate(
    plan: &WorkspaceTransactionPlan,
    token: &WorkspaceDraftGateToken,
    current_registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    validate_workspace_transaction_draft_gate_for_commit(plan, token, current_registry)?;
    commit_workspace_transaction_internal(plan, None, None)
}

pub(crate) fn validate_workspace_transaction_draft_gate_for_commit(
    plan: &WorkspaceTransactionPlan,
    token: &WorkspaceDraftGateToken,
    current_registry: &WorkspaceDraftRegistryView,
) -> Result<(), WorkspaceTransactionError> {
    validate_draft_gate_token(plan, token)?;
    validate_draft_registry_view(current_registry)?;
    let blockers = plan
        .draft_sensitive_node_ids
        .iter()
        .copied()
        .filter(|node_id| {
            current_registry
                .dirty_node_ids
                .binary_search(node_id)
                .is_ok()
        })
        .collect::<Vec<_>>();
    if !blockers.is_empty() {
        return Err(WorkspaceTransactionError::DraftGateBlocked(blockers));
    }
    Ok(())
}

fn validate_draft_registry_view(
    registry: &WorkspaceDraftRegistryView,
) -> Result<(), WorkspaceTransactionError> {
    let canonical = WorkspaceDraftRegistryView::new(
        registry.observation.clone(),
        registry.dirty_node_ids.iter().copied(),
    )?;
    if &canonical == registry {
        Ok(())
    } else {
        Err(WorkspaceTransactionError::DraftGateAuthorityMismatch)
    }
}

fn draft_registry_observation_digest(
    plan: &WorkspaceTransactionPlan,
    registry: &WorkspaceDraftRegistryView,
) -> Result<String, WorkspaceTransactionError> {
    serde_json::to_vec(&(
        DRAFT_GATE_SCHEMA,
        &plan.plan_id,
        &plan.base_revision,
        &plan.draft_sensitive_node_ids,
        registry,
    ))
    .map(|bytes| digest_bytes(&bytes))
    .map_err(WorkspaceTransactionError::Json)
}

fn draft_gate_token_digest(
    token: &WorkspaceDraftGateToken,
) -> Result<String, WorkspaceTransactionError> {
    serde_json::to_vec(&(
        &token.schema,
        &token.plan_id,
        &token.base_revision,
        &token.required_clean_node_ids,
        &token.preview_observation_digest,
    ))
    .map(|bytes| digest_bytes(&bytes))
    .map_err(WorkspaceTransactionError::Json)
}

fn validate_draft_gate_token(
    plan: &WorkspaceTransactionPlan,
    token: &WorkspaceDraftGateToken,
) -> Result<(), WorkspaceTransactionError> {
    if token.schema != DRAFT_GATE_SCHEMA
        || token.plan_id != plan.plan_id
        || token.base_revision != plan.base_revision
        || token.required_clean_node_ids != plan.draft_sensitive_node_ids
        || token.authority_digest != draft_gate_token_digest(token)?
    {
        return Err(WorkspaceTransactionError::DraftGateAuthorityMismatch);
    }
    Ok(())
}

/// Commits under an explicit empty draft-registry authority.
///
/// Native and hosted surfaces that maintain drafts must instead call
/// [`preview_workspace_transaction_draft_gate`] and
/// [`commit_workspace_transaction_with_draft_gate`]. This compatibility path
/// is for headless callers that have no draft store.
///
/// # Errors
///
/// Returns the same authority, revision, recovery, filesystem, and verification
/// errors as the gated commit path.
pub fn commit_workspace_transaction(
    plan: &WorkspaceTransactionPlan,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let preview = preview_workspace_transaction_draft_gate(plan, &registry)?;
    let token = preview
        .executable_token
        .as_ref()
        .ok_or_else(|| WorkspaceTransactionError::DraftGateBlocked(Vec::new()))?;
    commit_workspace_transaction_with_draft_gate(plan, token, &registry)
}

/// Commits a transaction but retains its verified committed journal until the caller durably
/// persists its higher-level receipt and explicitly finalizes the journal.
///
/// Ordinary workspace writes remain blocked while the committed journal is retained. This is the
/// receipt handoff used by import orchestration; it does not create another write authority.
///
/// # Errors
///
/// Returns the same typed errors as [`commit_workspace_transaction`].
pub fn commit_workspace_transaction_retaining_journal(
    plan: &WorkspaceTransactionPlan,
    receipt_path: impl AsRef<Path>,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let preview = preview_workspace_transaction_draft_gate(plan, &registry)?;
    let token = preview
        .executable_token
        .as_ref()
        .ok_or_else(|| WorkspaceTransactionError::DraftGateBlocked(Vec::new()))?;
    commit_workspace_transaction_retaining_journal_with_draft_gate(
        plan,
        receipt_path,
        token,
        &registry,
    )
}

/// Retains the committed journal after the same fresh authoritative draft
/// recheck used by the ordinary commit path.
///
/// # Errors
///
/// Returns the same draft, receipt, and transaction errors as the corresponding
/// non-retained gated commit.
pub fn commit_workspace_transaction_retaining_journal_with_draft_gate(
    plan: &WorkspaceTransactionPlan,
    receipt_path: impl AsRef<Path>,
    token: &WorkspaceDraftGateToken,
    current_registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    validate_draft_gate_token(plan, token)?;
    validate_draft_registry_view(current_registry)?;
    let blockers = plan
        .draft_sensitive_node_ids
        .iter()
        .copied()
        .filter(|node_id| {
            current_registry
                .dirty_node_ids
                .binary_search(node_id)
                .is_ok()
        })
        .collect::<Vec<_>>();
    if !blockers.is_empty() {
        return Err(WorkspaceTransactionError::DraftGateBlocked(blockers));
    }
    commit_workspace_transaction_internal(plan, None, Some(receipt_path.as_ref()))
}

/// Persists one authentic prepared journal for cross-crate recovery acceptance tests.
///
/// This helper is absent from release builds and deliberately stops before applying any step.
/// It uses the same lease, revision check, staged payloads, digests, and durable journal writer as
/// the production commit path.
///
/// # Errors
///
/// Returns the same planning, filesystem, recovery, and revision errors as transaction commit.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn prepare_workspace_transaction_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    let journal =
        prepared_journal_metadata_with_finalization(plan, JournalFinalization::Core, None)?;
    validate_journal_lifecycle_wire_limit(&journal, MAX_JOURNAL_BYTES)?;
    cleanup_workspace_transaction_tombstones(&plan.workspace_root)?;
    cleanup_rolled_back_markers(&plan.workspace_root)?;
    ensure_no_unfinished_transaction(&plan.workspace_root)?;
    let current = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&plan.base_revision, &current)?;
    let transaction = transaction_path(&plan.workspace_root, &plan.plan_id)?;
    match fs::create_dir(&transaction) {
        Ok(()) => {
            if let Err(error) = sync_directory(&plan.workspace_root) {
                let _ = fs::remove_dir(&transaction);
                return Err(error);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
        }
        Err(error) => return Err(WorkspaceTransactionError::Io(error)),
    }
    let materialized_steps = match prepare_journal_steps(plan, &transaction) {
        Ok(steps) => steps,
        Err(error) => {
            let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
            return Err(error);
        }
    };
    if materialized_steps != journal.steps {
        let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
        return Err(WorkspaceTransactionError::InvalidJournal(
            "materialized journal steps differ from preflight authority".to_owned(),
        ));
    }
    if let Err(error) = write_journal(&transaction, &journal) {
        let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
        return Err(error);
    }
    drop(lease);
    Ok(transaction)
}

#[cfg(test)]
pub(crate) fn debug_workspace_transaction_journal_for_test(
    transaction: &Path,
) -> Result<String, WorkspaceTransactionError> {
    read_journal(transaction).map(|journal| {
        format!(
            "{journal:#?}\nrollback_confirmation={:#?}",
            journal.task_rebaseline_rollback_commit_confirmation
        )
    })
}

#[cfg(test)]
pub(crate) fn validate_task_rebaseline_transaction_artifacts_for_test(
    transaction: &Path,
) -> Result<(), WorkspaceTransactionError> {
    let journal = read_journal(transaction)?;
    validate_task_rebaseline_transaction_artifacts(transaction, &journal)
}

#[cfg(test)]
pub(crate) fn rewrite_workspace_transaction_journal_applying_with_limit_for_test(
    transaction: &Path,
    maximum_bytes: u64,
) -> Result<(), WorkspaceTransactionError> {
    let mut journal = read_journal(transaction)?;
    mark_journal_applying(&mut journal)?;
    write_journal_with_limit(transaction, &journal, maximum_bytes)
}

#[cfg(test)]
pub(crate) fn workspace_transaction_journal_lifecycle_bytes_for_test(
    plan: &WorkspaceTransactionPlan,
) -> Result<u64, WorkspaceTransactionError> {
    let journal =
        prepared_journal_metadata_with_finalization(plan, JournalFinalization::Core, None)?;
    maximum_journal_lifecycle_wire_bytes(&journal)
}

#[cfg(test)]
pub(crate) fn commit_workspace_transaction_with_journal_limit_for_test(
    plan: &WorkspaceTransactionPlan,
    maximum_bytes: u64,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    commit_workspace_transaction_with_lease_and_verification_failure(
        plan,
        None,
        None,
        &lease,
        false,
        maximum_bytes,
    )
}

#[cfg(debug_assertions)]
pub(crate) fn commit_workspace_transaction_with_injected_failure_for_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
    fail_after_steps: usize,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    commit_workspace_transaction_internal(plan, Some(fail_after_steps), None)
}

#[cfg(debug_assertions)]
pub(crate) fn commit_workspace_transaction_with_injected_verification_failure_for_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    commit_workspace_transaction_with_lease_and_verification_failure(
        plan,
        None,
        None,
        &lease,
        true,
        MAX_JOURNAL_BYTES,
    )
}

#[cfg(debug_assertions)]
pub(crate) fn prepare_workspace_transaction_applying_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
    applied_steps: usize,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let transaction = prepare_workspace_transaction_recovery_fixture(plan)?;
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    let mut journal = read_journal(&transaction)?;
    mark_journal_applying(&mut journal)?;
    write_journal(&transaction, &journal)?;
    match apply_journal_steps(
        &plan.workspace_root,
        &transaction,
        &journal.steps,
        Some(applied_steps),
    ) {
        Err(WorkspaceTransactionError::InjectedFailure(actual)) if actual == applied_steps => {}
        Err(error) => return Err(error),
        Ok(()) => {
            return Err(WorkspaceTransactionError::Metadata(
                "applying recovery fixture boundary exceeds the step program".to_owned(),
            ));
        }
    }
    drop(lease);
    Ok(transaction)
}

#[cfg(debug_assertions)]
#[allow(
    dead_code,
    reason = "used by the private TaskRebaseline mid-ReplaceFile recovery probes"
)]
pub(crate) fn prepare_workspace_transaction_displaced_replace_file_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
    step_index: usize,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let transaction = prepare_workspace_transaction_recovery_fixture(plan)?;
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    let mut journal = read_journal(&transaction)?;
    mark_journal_applying(&mut journal)?;
    write_journal(&transaction, &journal)?;
    let JournalStep::ReplaceFile {
        destination,
        staged,
        displaced,
        base_digest,
        next_digest,
    } = journal.steps.get(step_index).ok_or_else(|| {
        WorkspaceTransactionError::Metadata(
            "mid-ReplaceFile recovery fixture step is unavailable".to_owned(),
        )
    })?
    else {
        return Err(WorkspaceTransactionError::Metadata(
            "mid-ReplaceFile recovery fixture requires a ReplaceFile step".to_owned(),
        ));
    };
    let destination = safe_join(&plan.workspace_root, destination)?;
    let staged = safe_join(&transaction, staged)?;
    let displaced = safe_join(&transaction, displaced)?;
    require_path_digest(&destination, base_digest)?;
    require_path_digest(&staged, next_digest)?;
    require_path_absent(&displaced)?;
    create_parent(&displaced)?;
    durable_rename(&destination, &displaced)?;
    require_missing_path(&destination)?;
    require_path_digest(&staged, next_digest)?;
    require_path_digest(&displaced, base_digest)?;
    lease.validate_anchor_identity()?;
    drop(lease);
    Ok(transaction)
}

#[cfg(debug_assertions)]
pub(crate) fn prepare_workspace_transaction_committed_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let transaction = prepare_workspace_transaction_recovery_fixture(plan)?;
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    let mut journal = read_journal(&transaction)?;
    mark_journal_applying(&mut journal)?;
    write_journal(&transaction, &journal)?;
    apply_journal_steps(&plan.workspace_root, &transaction, &journal.steps, None)?;
    verify_plan_outcome(plan)?;
    let revision = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    mark_journal_committed(&mut journal, revision)?;
    write_journal(&transaction, &journal)?;
    drop(lease);
    Ok(transaction)
}

#[cfg(debug_assertions)]
#[allow(
    dead_code,
    reason = "used by the private TaskRebaseline unit recovery probes"
)]
pub(crate) fn prepare_workspace_transaction_fully_applied_recovery_fixture(
    plan: &WorkspaceTransactionPlan,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let transaction = prepare_workspace_transaction_recovery_fixture(plan)?;
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    let mut journal = read_journal(&transaction)?;
    mark_journal_applying(&mut journal)?;
    write_journal(&transaction, &journal)?;
    apply_journal_steps(&plan.workspace_root, &transaction, &journal.steps, None)?;
    verify_plan_outcome(plan)?;
    lease.validate_anchor_identity()?;
    drop(lease);
    Ok(transaction)
}

/// Reads opaque receipt bytes already staged under one retained committed transaction.
///
/// A returned handoff with no destination represents a crash after the receipt bytes were fixed
/// but before the external path claim was atomically installed.
///
/// # Errors
///
/// Returns an error for foreign transaction evidence, an unsafe external path, altered bytes, or
/// a non-committed/non-retained journal.
pub fn read_committed_workspace_transaction_receipt_handoff(
    root: impl AsRef<Path>,
    expected: &CommittedWorkspaceTransaction,
) -> Result<Option<WorkspaceTransactionReceiptHandoff>, WorkspaceTransactionError> {
    let root = root.as_ref();
    let (transaction, journal, _lease) = load_retained_committed_journal(root, expected)?;
    read_external_receipt_handoff(root, &transaction, &journal)
}

/// Atomically fixes opaque receipt bytes and one external destination for a retained transaction,
/// then publishes or verifies that exact external file without overwrite.
///
/// Concurrent callers may reuse the same bytes/path, but a different timestamp, receipt body, or
/// destination loses the atomic claim and fails closed.
///
/// # Errors
///
/// Returns an error for an unsafe/in-workspace destination, altered transaction authority,
/// conflicting receipt claim, size limit, or durable publication failure.
pub fn publish_committed_workspace_transaction_receipt(
    root: impl AsRef<Path>,
    expected: &CommittedWorkspaceTransaction,
    receipt_path: impl AsRef<Path>,
    receipt_bytes: &[u8],
) -> Result<WorkspaceTransactionReceiptHandoff, WorkspaceTransactionError> {
    if u64::try_from(receipt_bytes.len()).unwrap_or(u64::MAX) > MAX_EXTERNAL_RECEIPT_BYTES {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt exceeds {MAX_EXTERNAL_RECEIPT_BYTES} bytes"
        )));
    }
    let root = root.as_ref();
    let (transaction, journal, _lease) = load_retained_committed_journal(root, expected)?;
    let destination = canonical_external_receipt_destination(root, receipt_path.as_ref())?;
    let intended_destination =
        validated_external_receipt_destination(root, &journal)?.ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "retained transaction has no fixed external receipt destination".to_owned(),
            )
        })?;
    if destination != intended_destination {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt destination differs from the pre-commit intent: {}",
            intended_destination.display()
        )));
    }
    let payload_path = transaction.join(EXTERNAL_RECEIPT_PAYLOAD_FILE);
    publish_exact_file(&payload_path, receipt_bytes, MAX_EXTERNAL_RECEIPT_BYTES)?;
    let receipt_sha256 = digest_bytes(receipt_bytes);

    match read_external_receipt_handoff(root, &transaction, &journal)? {
        Some(existing) => {
            if existing.bytes != receipt_bytes
                || existing.sha256 != receipt_sha256
                || existing
                    .destination
                    .as_ref()
                    .is_some_and(|claimed| claimed != &destination)
            {
                return Err(WorkspaceTransactionError::ExternalReceipt(
                    "another exact receipt body or destination already owns this transaction"
                        .to_owned(),
                ));
            }
        }
        None => {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "receipt payload disappeared after atomic staging".to_owned(),
            ));
        }
    }

    let claim = external_receipt_claim(&journal, &destination, receipt_bytes)?;
    let claim_bytes = serde_json::to_vec_pretty(&claim).map_err(WorkspaceTransactionError::Json)?;
    publish_exact_file(
        &transaction.join(EXTERNAL_RECEIPT_CLAIM_FILE),
        &claim_bytes,
        MAX_EXTERNAL_RECEIPT_CLAIM_BYTES,
    )?;
    let claimed =
        read_external_receipt_handoff(root, &transaction, &journal)?.ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "receipt claim disappeared after atomic publication".to_owned(),
            )
        })?;
    if claimed.destination.as_ref() != Some(&destination)
        || claimed.sha256 != receipt_sha256
        || claimed.bytes != receipt_bytes
    {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "atomic receipt claim differs from the requested handoff".to_owned(),
        ));
    }
    publish_exact_file(&destination, receipt_bytes, MAX_EXTERNAL_RECEIPT_BYTES)?;
    verify_published_external_receipt(&claimed)?;
    Ok(claimed)
}

#[allow(clippy::too_many_lines)]
fn commit_workspace_transaction_internal(
    plan: &WorkspaceTransactionPlan,
    fail_after_steps: Option<usize>,
    external_receipt_path: Option<&Path>,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(&plan.workspace_root)?;
    commit_workspace_transaction_with_lease(plan, fail_after_steps, external_receipt_path, &lease)
}

pub(crate) fn commit_workspace_transaction_with_clean_guard(
    plan: &WorkspaceTransactionPlan,
    guard: &WorkspaceTransactionLease,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let plan_root =
        fs::canonicalize(&plan.workspace_root).map_err(WorkspaceTransactionError::Io)?;
    if plan_root != guard.canonical_root {
        return Err(WorkspaceTransactionError::Metadata(
            "workspace mutation guard belongs to another workspace root".to_owned(),
        ));
    }
    commit_workspace_transaction_with_lease(plan, None, None, guard)
}

#[allow(clippy::too_many_lines)]
fn commit_workspace_transaction_with_lease(
    plan: &WorkspaceTransactionPlan,
    fail_after_steps: Option<usize>,
    external_receipt_path: Option<&Path>,
    lease: &WorkspaceTransactionLease,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    commit_workspace_transaction_with_lease_and_verification_failure(
        plan,
        fail_after_steps,
        external_receipt_path,
        lease,
        false,
        MAX_JOURNAL_BYTES,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "directional physical, semantic, snapshot, and Owner checks form one pre-journal boundary"
)]
fn recheck_task_rebaseline_before_journal(
    plan: &WorkspaceTransactionPlan,
    lease: &WorkspaceTransactionLease,
) -> Result<(), WorkspaceTransactionError> {
    if plan.action != StructuralAction::TaskRebaseline {
        return Ok(());
    }
    validate_plan_scope_authority(plan)?;
    let (
        physical_pre_state,
        physical_pre_entries,
        workspace_root_identity,
        root_node_id,
        root_revision,
        source_replacements,
        generated_nodes,
        rollback,
    ) = if let Some(authority) = &plan.task_rebaseline_authority {
        if plan.task_rebaseline_commit_confirmation.is_none() {
            return Err(WorkspaceTransactionError::Metadata(
                "task rebaseline requires a fresh Owner confirmation immediately before apply"
                    .to_owned(),
            ));
        }
        (
            &authority.physical_pre_state,
            authority.physical_pre_entries.as_slice(),
            &authority.workspace_root_identity,
            authority.workspace_root_node_id,
            &authority.workspace_root_document_revision,
            authority.source_replacements.as_slice(),
            authority.new_nodes.as_slice(),
            false,
        )
    } else if let Some(authority) = &plan.task_rebaseline_rollback_authority {
        if plan.task_rebaseline_rollback_commit_confirmation.is_none() {
            return Err(WorkspaceTransactionError::Metadata(
                    "task rebaseline rollback requires a fresh Owner confirmation immediately before apply"
                        .to_owned(),
                ));
        }
        (
            &authority.physical_pre_state,
            authority.physical_pre_entries.as_slice(),
            &authority.workspace_root_identity,
            authority.workspace_root_node_id,
            &authority.workspace_root_pre_document_revision,
            authority.forward_authority.source_replacements.as_slice(),
            authority.forward_authority.new_nodes.as_slice(),
            true,
        )
    } else {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline pre-state authority is unavailable".to_owned(),
        ));
    };
    if plan.task_rebaseline_authority.is_some() == plan.task_rebaseline_rollback_authority.is_some()
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline direction authority is ambiguous".to_owned(),
        ));
    }
    let inventory = crate::capture_stable_workspace_physical_inventory(lease)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    if inventory.binding() != physical_pre_state
        || inventory.root_identity() != workspace_root_identity
        || inventory.records() != physical_pre_entries
    {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline physical pre-state differs from reviewed direction authority"
                .to_owned(),
        ));
    }
    let root_document = read_node_document(lease.physical_inventory_root())
        .map_err(WorkspaceTransactionError::Document)?;
    if root_document.node_id != root_node_id || root_document.revision != *root_revision {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline workspace root authority changed".to_owned(),
        ));
    }
    if rollback {
        let semantic = scan_workspace(lease.physical_inventory_root());
        if !semantic.is_valid() {
            return Err(WorkspaceTransactionError::VerificationFailed(
                "task rebaseline rollback semantic pre-state differs from exact C".to_owned(),
            ));
        }
        for replacement in source_replacements {
            if fs::read(safe_join(
                lease.physical_inventory_root(),
                &replacement.document_locator,
            )?)
            .map_err(WorkspaceTransactionError::Io)?
                != replacement.proposed_source.as_bytes()
            {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline rollback source bytes differ from exact C".to_owned(),
                ));
            }
        }
        for node in generated_nodes {
            if fs::read(safe_join(
                lease.physical_inventory_root(),
                &node.document_locator,
            )?)
            .map_err(WorkspaceTransactionError::Io)?
                != node.exact_source.as_bytes()
            {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline rollback generated node differs from exact C".to_owned(),
                ));
            }
        }
    }
    let external = plan
        .task_rebaseline_external_snapshot
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::Metadata(
                "task rebaseline external snapshot proof is unavailable".to_owned(),
            )
        })?;
    external
        .revalidate(lease)
        .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    lease.validate_anchor_identity()
}

#[allow(
    clippy::too_many_lines,
    reason = "directional physical and semantic post-state checks form one pre-commit boundary"
)]
fn verify_task_rebaseline_physical_post_state(
    plan: &WorkspaceTransactionPlan,
    lease: &WorkspaceTransactionLease,
    transaction: &Path,
    transaction_identity: &crate::physical_inventory::PhysicalRootIdentityBinding,
) -> Result<(), WorkspaceTransactionError> {
    if plan.action != StructuralAction::TaskRebaseline {
        return Ok(());
    }
    let (
        physical_post_state,
        physical_post_entries,
        workspace_root_identity,
        root_node_id,
        root_revision,
        source_replacements,
        generated_nodes,
        rollback,
    ) = if let Some(authority) = &plan.task_rebaseline_authority {
        (
            &authority.physical_post_state,
            authority.physical_post_entries.as_slice(),
            &authority.workspace_root_identity,
            authority.workspace_root_node_id,
            None,
            authority.source_replacements.as_slice(),
            authority.new_nodes.as_slice(),
            false,
        )
    } else if let Some(authority) = &plan.task_rebaseline_rollback_authority {
        (
            &authority.physical_post_state,
            authority.physical_post_entries.as_slice(),
            &authority.workspace_root_identity,
            authority.workspace_root_node_id,
            Some(&authority.workspace_root_post_document_revision),
            authority.forward_authority.source_replacements.as_slice(),
            authority.forward_authority.new_nodes.as_slice(),
            true,
        )
    } else {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline post-state authority is unavailable".to_owned(),
        ));
    };
    let inventory = crate::physical_inventory::capture_stable_workspace_physical_inventory_excluding_transaction(
        lease,
        transaction,
        transaction_identity,
    )
    .map_err(|error| WorkspaceTransactionError::VerificationFailed(error.to_string()))?;
    if inventory.binding() != physical_post_state
        || inventory.root_identity() != workspace_root_identity
        || inventory.records() != physical_post_entries
    {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline physical post-state differs from expected direction result".to_owned(),
        ));
    }
    for replacement in source_replacements {
        let path = safe_join(&plan.workspace_root, &replacement.document_locator)?;
        let expected = if rollback {
            replacement.original_source.as_bytes()
        } else {
            replacement.proposed_source.as_bytes()
        };
        if fs::read(&path).map_err(WorkspaceTransactionError::Io)? != expected {
            return Err(WorkspaceTransactionError::VerificationFailed(
                "task rebaseline source replacement differs byte-for-byte".to_owned(),
            ));
        }
    }
    for node in generated_nodes {
        let node_path = safe_join(&plan.workspace_root, &node.destination_node_locator)?;
        if rollback {
            if non_link_path_exists(&node_path)? {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline rollback left one generated task node present".to_owned(),
                ));
            }
        } else if fs::read(safe_join(&plan.workspace_root, &node.document_locator)?)
            .map_err(WorkspaceTransactionError::Io)?
            != node.exact_source.as_bytes()
        {
            return Err(WorkspaceTransactionError::VerificationFailed(
                "task rebaseline generated task node differs byte-for-byte".to_owned(),
            ));
        }
    }
    let semantic = scan_workspace(&plan.workspace_root);
    if !semantic.is_valid() {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline semantic post-state inventory is invalid".to_owned(),
        ));
    }
    if let Some(root_revision) = root_revision {
        let root_document = read_node_document(&plan.workspace_root)
            .map_err(WorkspaceTransactionError::Document)?;
        if root_document.node_id != root_node_id || root_document.revision != *root_revision {
            return Err(WorkspaceTransactionError::VerificationFailed(
                "task rebaseline rollback root document differs from exact A".to_owned(),
            ));
        }
    }
    let external = plan
        .task_rebaseline_external_snapshot
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::VerificationFailed(
                "task rebaseline external snapshot proof is unavailable".to_owned(),
            )
        })?;
    external
        .revalidate_excluding_transaction(lease, transaction, transaction_identity)
        .map_err(|error| WorkspaceTransactionError::VerificationFailed(error.to_string()))?;
    lease.validate_anchor_identity()
}

#[allow(clippy::too_many_lines)]
fn commit_workspace_transaction_with_lease_and_verification_failure(
    plan: &WorkspaceTransactionPlan,
    fail_after_steps: Option<usize>,
    external_receipt_path: Option<&Path>,
    lease: &WorkspaceTransactionLease,
    inject_verification_failure: bool,
    journal_maximum_bytes: u64,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let external_receipt_destination = external_receipt_path
        .map(|path| canonical_external_receipt_destination(&plan.workspace_root, path))
        .transpose()?;
    if let Some(destination) = &external_receipt_destination
        && non_link_path_exists(destination)?
    {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "external receipt destination already exists: {}",
            destination.display()
        )));
    }
    ensure_no_unfinished_transaction(&plan.workspace_root)?;
    let current = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&plan.base_revision, &current)?;
    recheck_task_rebaseline_before_journal(plan, lease)?;
    let finalization = if external_receipt_destination.is_some() {
        JournalFinalization::ExternalReceipt
    } else {
        JournalFinalization::Core
    };
    let mut journal = prepared_journal_metadata_with_finalization(
        plan,
        finalization,
        external_receipt_destination.as_deref(),
    )?;
    validate_journal_lifecycle_wire_limit(&journal, journal_maximum_bytes)?;
    cleanup_workspace_transaction_tombstones(&plan.workspace_root)?;
    cleanup_rolled_back_markers(&plan.workspace_root)?;
    ensure_no_unfinished_transaction(&plan.workspace_root)?;
    let current = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&plan.base_revision, &current)?;
    recheck_task_rebaseline_before_journal(plan, lease)?;
    let transaction = transaction_path(&plan.workspace_root, &plan.plan_id)?;
    match fs::create_dir(&transaction) {
        Ok(()) => {
            if let Err(error) = sync_directory(&plan.workspace_root) {
                let _ = fs::remove_dir(&transaction);
                return Err(error);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
        }
        Err(error) => return Err(WorkspaceTransactionError::Io(error)),
    }
    let transaction_identity =
        crate::physical_inventory::physical_root_identity_at(&transaction)
            .map_err(|error| WorkspaceTransactionError::Metadata(error.to_string()))?;
    let materialized_steps = match prepare_journal_steps(plan, &transaction) {
        Ok(steps) => steps,
        Err(error) => {
            let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
            return Err(error);
        }
    };
    if materialized_steps != journal.steps {
        let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
        return Err(WorkspaceTransactionError::InvalidJournal(
            "materialized journal steps differ from preflight authority".to_owned(),
        ));
    }
    write_journal_with_limit(&transaction, &journal, journal_maximum_bytes)?;
    let latest = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    if let Err(error) = require_workspace_revision(&plan.base_revision, &latest) {
        let _ = remove_transaction_directory(&plan.workspace_root, &transaction);
        return Err(error);
    }

    mark_journal_applying(&mut journal)?;
    write_journal_with_limit(&transaction, &journal, journal_maximum_bytes)?;
    let apply_and_verify_result = apply_journal_steps(
        &plan.workspace_root,
        &transaction,
        &journal.steps,
        fail_after_steps,
    )
    .and_then(|()| {
        if inject_verification_failure {
            Err(WorkspaceTransactionError::VerificationFailed(
                "injected post-apply verification failure".to_owned(),
            ))
        } else {
            verify_plan_outcome(plan).and_then(|()| {
                verify_task_rebaseline_physical_post_state(
                    plan,
                    lease,
                    &transaction,
                    &transaction_identity,
                )
            })
        }
    });
    if let Err(error) = apply_and_verify_result {
        if plan.action == StructuralAction::TaskRebaseline {
            match classify_task_rebaseline_physical_state(
                &transaction,
                &transaction_identity,
                &journal,
                lease,
            ) {
                Ok(TaskRebaselinePhysicalState::Old) => {}
                Ok(TaskRebaselinePhysicalState::RecoverableMixed) => {
                    if let Err(recovery) =
                        rollback_journal(&plan.workspace_root, &transaction, &journal)
                    {
                        return Err(WorkspaceTransactionError::RecoveryRequiredWithCause {
                            path: transaction,
                            cause: format!("{error}; rollback failed: {recovery}"),
                        });
                    }
                }
                Ok(TaskRebaselinePhysicalState::New) | Err(_) => {
                    return Err(WorkspaceTransactionError::RecoveryRequiredWithCause {
                        path: transaction,
                        cause: format!(
                            "{error}; task rebaseline state is not an exact safe rollback input"
                        ),
                    });
                }
            }
            if classify_task_rebaseline_physical_state(
                &transaction,
                &transaction_identity,
                &journal,
                lease,
            )? != TaskRebaselinePhysicalState::Old
            {
                return Err(WorkspaceTransactionError::RecoveryRequiredWithCause {
                    path: transaction,
                    cause: format!("{error}; exact A was not restored"),
                });
            }
            write_rolled_back_marker(
                &plan.workspace_root,
                &journal,
                RolledBackPriorState::Applying,
            )?;
            remove_transaction_directory(&plan.workspace_root, &transaction)?;
            return Err(error);
        }
        if let Err(recovery) = rollback_journal(&plan.workspace_root, &transaction, &journal) {
            return Err(WorkspaceTransactionError::RecoveryRequiredWithCause {
                path: transaction,
                cause: format!("{error}; rollback failed: {recovery}"),
            });
        }
        write_rolled_back_marker(
            &plan.workspace_root,
            &journal,
            RolledBackPriorState::Applying,
        )?;
        remove_transaction_directory(&plan.workspace_root, &transaction)?;
        return Err(error);
    }

    let revision = read_workspace_revision(&plan.workspace_root)
        .map_err(WorkspaceTransactionError::Revision)?;
    lease.validate_anchor_identity()?;
    mark_journal_committed(&mut journal, revision.clone())?;
    write_journal_with_limit(&transaction, &journal, journal_maximum_bytes)?;
    let committed = CommittedWorkspaceTransaction {
        plan_id: plan.plan_id.clone(),
        action: plan.action,
        base_revision: plan.base_revision.clone(),
        revision,
        path_changes: plan.path_changes.clone(),
        scope_summary: plan.scope_summary.clone(),
        promotion_summary: plan.promotion_summary.clone(),
        identity_map: plan.identity_map.clone(),
        captured_target: plan.captured_target.clone(),
        target_node_ids: plan.target_node_ids.clone(),
        draft_sensitive_node_ids: plan.draft_sensitive_node_ids.clone(),
        import_authority: plan.import_authority.clone(),
    };
    if external_receipt_destination.is_none() {
        remove_transaction_directory(&plan.workspace_root, &transaction)?;
    }
    Ok(committed)
}

/// Deletes one retained committed journal only after its exact transaction evidence and current
/// workspace revision still match.
///
/// # Errors
///
/// Returns an error and keeps the journal for recovery when the identifier, state, authority,
/// paths, or workspace revision differs.
pub fn finalize_committed_workspace_transaction(
    root: impl AsRef<Path>,
    expected: &CommittedWorkspaceTransaction,
) -> Result<(), WorkspaceTransactionError> {
    let root = root.as_ref();
    let (transaction, journal, lease) = load_retained_committed_journal(root, expected)?;
    let handoff =
        read_external_receipt_handoff(root, &transaction, &journal)?.ok_or_else(|| {
            WorkspaceTransactionError::ExternalReceipt(
                "receipt bytes were not staged before finalization".to_owned(),
            )
        })?;
    if handoff.destination.is_none() {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "receipt destination was not claimed before finalization".to_owned(),
        ));
    }
    verify_published_external_receipt(&handoff)?;
    let result = remove_transaction_directory(root, &transaction);
    drop(lease);
    result
}

fn load_retained_committed_journal(
    root: &Path,
    expected: &CommittedWorkspaceTransaction,
) -> Result<(PathBuf, Journal, WorkspaceTransactionLease), WorkspaceTransactionError> {
    let mut journals = validated_workspace_journals(root)?;
    let Some((transaction, journal, lease)) = journals.pop() else {
        return Err(WorkspaceTransactionError::RecoveryRequired(
            transaction_path(root, &expected.plan_id)?,
        ));
    };
    if journal.state != JournalState::Committed
        || journal.finalization != JournalFinalization::ExternalReceipt
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "only a retained committed workspace transaction can hand off a receipt".to_owned(),
        ));
    }
    validated_external_receipt_destination(root, &journal)?;
    let committed = committed_transaction_from_journal(&journal)?;
    if &committed != expected {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "retained committed transaction differs from the receipt authority".to_owned(),
        ));
    }
    let actual = read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
    require_workspace_revision(&committed.revision, &actual)?;
    Ok((transaction, journal, lease))
}

/// Recovers every root-level workspace journal before ordinary writes resume.
///
/// # Errors
///
/// Returns an error and retains evidence if a journal is invalid or filesystem
/// state differs from both its recorded old and new forms.
pub fn recover_workspace_transactions(
    root: impl AsRef<Path>,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    recover_workspace_transactions_internal(root.as_ref(), true)
}

/// Recovers only the journal belonging to one exact Core workspace plan.
///
/// This is the commit-outcome resolution boundary for callers that still hold the opaque plan
/// after a commit returned an indeterminate I/O result. A foreign journal is retained unchanged;
/// callers never infer transaction ownership from a directory name.
///
/// # Errors
///
/// Returns an error without changing the journal when its plan identity, action, path authority,
/// import authority, finalization policy, or staged-step authority differs from `plan`.
pub fn recover_workspace_transaction_for_plan(
    plan: &WorkspaceTransactionPlan,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    let mut journals = validated_workspace_journals(&plan.workspace_root)?;
    let Some((transaction, journal, lease)) = journals.pop() else {
        return Ok(RecoveryReport::default());
    };
    if !journal_matches_workspace_plan(&journal, plan) {
        return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
    }
    recover_validated_journal(&plan.workspace_root, &transaction, &journal, lease, true)
}

/// Rolls back unfinished journals while retaining verified committed journals for receipt
/// reconstruction. No new workspace write may begin until each returned committed transaction is
/// explicitly finalized.
///
/// # Errors
///
/// Returns an error and keeps ambiguous evidence when any journal or filesystem state differs.
pub fn recover_workspace_transactions_retaining_committed(
    root: impl AsRef<Path>,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    recover_workspace_transactions_internal(root.as_ref(), false)
}

/// Reports whether a validated unfinished workspace transaction needs recovery.
///
/// This read-only check shares Core's journal, rollback-marker, tombstone, link, path, and digest
/// validation. An unknown or malformed transaction-shaped entry is an error rather than an absent
/// transaction.
///
/// # Errors
///
/// Returns a typed recovery error when transaction authority cannot be validated without mutation.
pub fn has_unfinished_workspace_transaction(
    root: impl AsRef<Path>,
) -> Result<bool, WorkspaceTransactionError> {
    Ok(!validated_workspace_journals_read_only(root.as_ref())?.is_empty())
}

/// Recovers only the unfinished import transaction carrying one exact reviewed authority.
///
/// A foreign or concurrent journal is never changed. All journal structure and linked-path
/// checks complete before prepared/applying state is removed or rolled back. A matching committed
/// transaction remains retained for the caller to persist its receipt and explicitly finalize.
///
/// # Errors
///
/// Returns an error without changing any journal when the sole unfinished transaction is foreign,
/// ambiguous, malformed, linked, or not an external-receipt handoff.
pub fn recover_workspace_import_transaction(
    root: impl AsRef<Path>,
    expected_authority: &WorkspaceImportAuthority,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    validate_import_authority(expected_authority)?;
    let root = root.as_ref();
    let mut journals = validated_workspace_journals(root)?;
    let Some((transaction, journal, lease)) = journals.pop() else {
        return rolled_back_recovery_report(root, Some(expected_authority));
    };
    if journal.finalization != JournalFinalization::ExternalReceipt
        || journal.import_authority.as_ref() != Some(expected_authority)
    {
        return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
    }
    recover_validated_journal(root, &transaction, &journal, lease, false)
}

/// Inspects, but never changes, the unfinished transaction carrying one exact import authority.
///
/// # Errors
///
/// Returns an error for a foreign, malformed, ambiguous, linked, or non-receipt journal.
pub fn inspect_workspace_import_transaction(
    root: impl AsRef<Path>,
    expected_authority: &WorkspaceImportAuthority,
) -> Result<WorkspaceImportTransactionState, WorkspaceTransactionError> {
    validate_import_authority(expected_authority)?;
    let mut journals = validated_workspace_journals_read_only(root.as_ref())?;
    let Some((transaction, journal, _lease)) = journals.pop() else {
        return Ok(WorkspaceImportTransactionState::Absent);
    };
    if journal.finalization != JournalFinalization::ExternalReceipt
        || journal.import_authority.as_ref() != Some(expected_authority)
    {
        return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
    }
    Ok(match journal.state {
        JournalState::Prepared => WorkspaceImportTransactionState::Prepared {
            plan_id: journal.plan_id,
        },
        JournalState::Applying => WorkspaceImportTransactionState::Applying {
            plan_id: journal.plan_id,
        },
        JournalState::Committed => WorkspaceImportTransactionState::Committed {
            transaction: committed_transaction_from_journal(&journal)?,
        },
    })
}

fn recover_workspace_transactions_internal(
    root: &Path,
    clean_committed: bool,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    let mut journals = validated_workspace_journals(root)?;
    let Some((transaction, journal, lease)) = journals.pop() else {
        return rolled_back_recovery_report(root, None);
    };
    recover_validated_journal(root, &transaction, &journal, lease, clean_committed)
}

fn rolled_back_recovery_report(
    root: &Path,
    expected_authority: Option<&WorkspaceImportAuthority>,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    let _lease = acquire_workspace_transaction_lease(root)?;
    let markers = read_rolled_back_markers(root)?;
    let mut report = RecoveryReport::default();
    for (path, marker) in markers {
        if let Some(expected) = expected_authority
            && marker.import_authority.as_ref() != Some(expected)
        {
            return Err(WorkspaceTransactionError::RecoveryRequired(path));
        }
        match marker.prior_state {
            RolledBackPriorState::Prepared => report.prepared_removed += 1,
            RolledBackPriorState::Applying => report.applying_rolled_back += 1,
        }
    }
    Ok(report)
}

fn validated_workspace_journals(
    root: &Path,
) -> Result<Vec<(PathBuf, Journal, WorkspaceTransactionLease)>, WorkspaceTransactionError> {
    validated_workspace_journals_internal(root, true)
}

fn validated_workspace_journals_read_only(
    root: &Path,
) -> Result<Vec<(PathBuf, Journal, WorkspaceTransactionLease)>, WorkspaceTransactionError> {
    validated_workspace_journals_internal(root, false)
}

fn validated_workspace_journals_internal(
    root: &Path,
    cleanup_tombstones: bool,
) -> Result<Vec<(PathBuf, Journal, WorkspaceTransactionLease)>, WorkspaceTransactionError> {
    let lease = acquire_workspace_transaction_lease(root)?;
    reject_linked_existing_ancestors(root)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    validate_or_cleanup_workspace_transaction_tombstones(root, cleanup_tombstones)?;
    read_rolled_back_markers(root)?;
    let mut transactions = fs::read_dir(root)
        .map_err(WorkspaceTransactionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceTransactionError::Io)?
        .into_iter()
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                is_workspace_transaction_name(name)
                    && !is_canonical_workspace_transaction_cleanup_name(name)
                    && !is_canonical_workspace_transaction_rollback_name(name)
                    && name != WORKSPACE_TRANSACTION_LEASE_FILE_NAME
            })
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    transactions.sort();
    if transactions.len() > 1 {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "workspace contains multiple unfinished transaction authorities".to_owned(),
        ));
    }
    let Some(transaction) = transactions.pop() else {
        return Ok(Vec::new());
    };
    let journal = read_journal(&transaction)?;
    validate_transaction_identity(&transaction, &journal)?;
    validate_journal_paths(root, &transaction, &journal)?;
    validated_external_receipt_destination(root, &journal)?;
    if journal.action == StructuralAction::TaskRebaseline {
        let transaction_identity =
            crate::physical_inventory::physical_root_identity_at(&transaction)
                .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
        let physical_state = classify_task_rebaseline_physical_state(
            &transaction,
            &transaction_identity,
            &journal,
            &lease,
        )?;
        if matches!(journal.state, JournalState::Prepared)
            && physical_state != TaskRebaselinePhysicalState::Old
            || matches!(journal.state, JournalState::Committed)
                && physical_state != TaskRebaselinePhysicalState::New
        {
            return Err(WorkspaceTransactionError::RecoveryRequired(transaction));
        }
    }
    if journal.state == JournalState::Committed {
        verify_committed_journal_outcome(root, &transaction, &journal)?;
    }
    Ok(vec![(transaction, journal, lease)])
}

fn verify_committed_journal_outcome(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    for step in &journal.steps {
        match step {
            JournalStep::CreateTree {
                destination,
                staged,
                digest,
            } => {
                require_path_digest(&safe_join(root, destination)?, digest)?;
                require_missing_path(&safe_join(transaction, staged)?)?;
            }
            JournalStep::CreateFile {
                destination,
                staged,
                next_digest,
            } => {
                require_path_digest(&safe_join(root, destination)?, next_digest)?;
                require_missing_path(&safe_join(transaction, staged)?)?;
            }
            JournalStep::MovePath {
                source,
                destination,
                holding,
                digest,
            } => {
                let source = safe_join(root, source)?;
                let destination = safe_join(root, destination)?;
                require_path_digest(&destination, digest)?;
                require_missing_or_same_path(&source, &destination)?;
                require_missing_path(&safe_join(transaction, holding)?)?;
            }
            JournalStep::RemovePath {
                source,
                holding,
                digest,
            } => {
                require_missing_path(&safe_join(root, source)?)?;
                require_path_digest(&safe_join(transaction, holding)?, digest)?;
            }
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            } => {
                require_path_digest(&safe_join(root, destination)?, next_digest)?;
                require_missing_path(&safe_join(transaction, staged)?)?;
                let displaced = safe_join(transaction, displaced)?;
                match digest_if_exists(&displaced)? {
                    Some(actual) if actual != *base_digest => {
                        return Err(WorkspaceTransactionError::VerificationFailed(format!(
                            "committed displaced file {} has digest {actual} instead of {base_digest}",
                            displaced.display()
                        )));
                    }
                    Some(_) | None => {}
                }
            }
        }
    }
    Ok(())
}

fn require_missing_or_same_path(
    source: &Path,
    destination: &Path,
) -> Result<(), WorkspaceTransactionError> {
    match fs::symlink_metadata(source) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) if fs::canonicalize(source).ok() == fs::canonicalize(destination).ok() => Ok(()),
        Ok(_) => Err(WorkspaceTransactionError::VerificationFailed(format!(
            "committed move retained a distinct source path at {}",
            source.display()
        ))),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn require_missing_path(path: &Path) -> Result<(), WorkspaceTransactionError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(WorkspaceTransactionError::VerificationFailed(format!(
            "committed transaction expected an absent path at {}",
            path.display()
        ))),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn recover_validated_journal(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
    lease: WorkspaceTransactionLease,
    clean_committed: bool,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    if journal.action == StructuralAction::TaskRebaseline {
        return recover_task_rebaseline_journal(root, transaction, journal, lease, clean_committed);
    }
    let mut report = RecoveryReport::default();
    match journal.state {
        JournalState::Prepared => {
            write_rolled_back_marker(root, journal, RolledBackPriorState::Prepared)?;
            remove_transaction_directory(root, transaction)?;
            report.prepared_removed += 1;
        }
        JournalState::Applying => {
            rollback_journal(root, transaction, journal)?;
            let revision =
                read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
            require_workspace_revision(&journal.base_revision, &revision)?;
            write_rolled_back_marker(root, journal, RolledBackPriorState::Applying)?;
            remove_transaction_directory(root, transaction)?;
            report.applying_rolled_back += 1;
        }
        JournalState::Committed => {
            let committed = committed_transaction_from_journal(journal)?;
            let actual =
                read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
            require_workspace_revision(&committed.revision, &actual)?;
            report.committed_transactions.push(committed);
            if clean_committed && journal.finalization == JournalFinalization::Core {
                remove_transaction_directory(root, transaction)?;
                report.committed_cleaned += 1;
            } else {
                report.committed_retained += 1;
            }
        }
    }
    drop(lease);
    Ok(report)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TaskRebaselinePhysicalState {
    Old,
    New,
    RecoverableMixed,
}

fn recover_task_rebaseline_journal(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
    lease: WorkspaceTransactionLease,
    clean_committed: bool,
) -> Result<RecoveryReport, WorkspaceTransactionError> {
    let transaction_identity = crate::physical_inventory::physical_root_identity_at(transaction)
        .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    let state = classify_task_rebaseline_physical_state(
        transaction,
        &transaction_identity,
        journal,
        &lease,
    )?;
    let mut report = RecoveryReport::default();
    match (journal.state, state) {
        (JournalState::Prepared, TaskRebaselinePhysicalState::Old) => {
            write_rolled_back_marker(root, journal, RolledBackPriorState::Prepared)?;
            remove_transaction_directory(root, transaction)?;
            report.prepared_removed += 1;
        }
        (JournalState::Applying, TaskRebaselinePhysicalState::Old) => {
            write_rolled_back_marker(root, journal, RolledBackPriorState::Applying)?;
            remove_transaction_directory(root, transaction)?;
            report.applying_rolled_back += 1;
        }
        (JournalState::Applying, TaskRebaselinePhysicalState::RecoverableMixed) => {
            rollback_journal(root, transaction, journal)?;
            if classify_task_rebaseline_physical_state(
                transaction,
                &transaction_identity,
                journal,
                &lease,
            )? != TaskRebaselinePhysicalState::Old
            {
                return Err(WorkspaceTransactionError::RecoveryRequired(
                    transaction.to_path_buf(),
                ));
            }
            write_rolled_back_marker(root, journal, RolledBackPriorState::Applying)?;
            remove_transaction_directory(root, transaction)?;
            report.applying_rolled_back += 1;
        }
        (JournalState::Applying, TaskRebaselinePhysicalState::New) => {
            verify_committed_journal_outcome(root, transaction, journal)?;
            verify_task_rebaseline_journal_semantic_post(root, journal)?;
            let revision =
                read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
            let mut committed_journal = journal.clone();
            mark_journal_committed(&mut committed_journal, revision)?;
            write_journal(transaction, &committed_journal)?;
            report
                .committed_transactions
                .push(committed_transaction_from_journal(&committed_journal)?);
            if clean_committed {
                remove_transaction_directory(root, transaction)?;
                report.committed_cleaned += 1;
            } else {
                report.committed_retained += 1;
            }
        }
        (JournalState::Committed, TaskRebaselinePhysicalState::New) => {
            verify_committed_journal_outcome(root, transaction, journal)?;
            verify_task_rebaseline_journal_semantic_post(root, journal)?;
            let committed = committed_transaction_from_journal(journal)?;
            let revision =
                read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
            require_workspace_revision(&committed.revision, &revision)?;
            report.committed_transactions.push(committed);
            if clean_committed {
                remove_transaction_directory(root, transaction)?;
                report.committed_cleaned += 1;
            } else {
                report.committed_retained += 1;
            }
        }
        _ => {
            return Err(WorkspaceTransactionError::RecoveryRequired(
                transaction.to_path_buf(),
            ));
        }
    }
    drop(lease);
    Ok(report)
}

fn classify_task_rebaseline_physical_state(
    transaction: &Path,
    transaction_identity: &crate::physical_inventory::PhysicalRootIdentityBinding,
    journal: &Journal,
    lease: &WorkspaceTransactionLease,
) -> Result<TaskRebaselinePhysicalState, WorkspaceTransactionError> {
    let (
        root_identity,
        physical_pre_state,
        physical_post_state,
        physical_pre_entries,
        physical_post_entries,
    ) = if let Some(authority) = &journal.task_rebaseline_authority {
        (
            &authority.workspace_root_identity,
            &authority.physical_pre_state,
            &authority.physical_post_state,
            authority.physical_pre_entries.as_slice(),
            authority.physical_post_entries.as_slice(),
        )
    } else if let Some(authority) = &journal.task_rebaseline_rollback_authority {
        (
            &authority.workspace_root_identity,
            &authority.physical_pre_state,
            &authority.physical_post_state,
            authority.physical_pre_entries.as_slice(),
            authority.physical_post_entries.as_slice(),
        )
    } else {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "task rebaseline recovery lacks complete directional pre/post authority".to_owned(),
        ));
    };
    let snapshot = journal
        .task_rebaseline_snapshot_authority
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "task rebaseline recovery lacks external snapshot authority".to_owned(),
            )
        })?;
    crate::physical_inventory::verify_disjoint_external_physical_tree_excluding_transaction(
        lease,
        Path::new(&snapshot.canonical_root),
        &snapshot.physical_inventory,
        &snapshot.root_identity,
        transaction,
        transaction_identity,
    )
    .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    let inventory = crate::physical_inventory::capture_stable_workspace_physical_inventory_excluding_transaction(
        lease,
        transaction,
        transaction_identity,
    )
    .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    if inventory.root_identity() != root_identity {
        return Err(WorkspaceTransactionError::RecoveryRequired(
            transaction.to_path_buf(),
        ));
    }
    let records = inventory.records();
    validate_task_rebaseline_transaction_artifacts(transaction, journal)
        .map_err(|_| WorkspaceTransactionError::RecoveryRequired(transaction.to_path_buf()))?;
    let closed = if let Some(authority) = &journal.task_rebaseline_authority {
        task_rebaseline_records_are_recoverable_mixed(transaction, journal, authority, &records)
    } else if let Some(authority) = &journal.task_rebaseline_rollback_authority {
        task_rebaseline_rollback_records_are_recoverable_mixed(
            transaction,
            journal,
            authority,
            &records,
        )
    } else {
        unreachable!("directional authority checked above")
    };
    if !closed.unwrap_or(false) {
        return Err(WorkspaceTransactionError::RecoveryRequired(
            transaction.to_path_buf(),
        ));
    }
    if inventory.binding() == physical_pre_state && records == physical_pre_entries {
        return Ok(TaskRebaselinePhysicalState::Old);
    }
    if inventory.binding() == physical_post_state && records == physical_post_entries {
        return Ok(TaskRebaselinePhysicalState::New);
    }
    Ok(TaskRebaselinePhysicalState::RecoverableMixed)
}

fn validate_task_rebaseline_transaction_artifacts(
    transaction: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    let mut tree_roots = BTreeSet::new();
    let mut exact_files = BTreeSet::from(["journal.json".to_owned()]);
    for step in &journal.steps {
        match step {
            JournalStep::CreateTree { staged, .. } => {
                tree_roots.insert(staged.clone());
            }
            JournalStep::CreateFile { staged, .. } => {
                exact_files.insert(staged.clone());
            }
            JournalStep::MovePath { .. } => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "task rebaseline transaction cannot contain a MovePath artifact".to_owned(),
                ));
            }
            JournalStep::RemovePath { holding, .. } => {
                tree_roots.insert(holding.clone());
            }
            JournalStep::ReplaceFile {
                staged, displaced, ..
            } => {
                exact_files.insert(staged.clone());
                exact_files.insert(displaced.clone());
            }
        }
    }
    let mut pending = vec![transaction.to_path_buf()];
    let mut recovery_artifact_bytes = 0_u64;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(WorkspaceTransactionError::Io)? {
            let entry = entry.map_err(WorkspaceTransactionError::Io)?;
            let path = entry.path();
            let relative = path
                .strip_prefix(transaction)
                .map_err(|_| WorkspaceTransactionError::PathEscape(path.clone()))?;
            let relative = components_string(relative, &path)?;
            let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
            let is_tree_root = tree_roots.contains(&relative);
            let in_tree = tree_roots
                .iter()
                .any(|prefix| relative.starts_with(&format!("{prefix}/")));
            let parent_of_expected = exact_files
                .iter()
                .chain(&tree_roots)
                .any(|expected| expected.starts_with(&format!("{relative}/")));
            let is_exact_file = exact_files.contains(&relative);
            if (!is_exact_file && !is_tree_root && !in_tree && !parent_of_expected)
                || (parent_of_expected && !metadata.is_dir())
                || (is_tree_root && (!metadata.is_dir() || linked_or_reparse(&metadata)))
                || (is_exact_file && (!metadata.is_file() || linked_or_reparse(&metadata)))
            {
                return Err(WorkspaceTransactionError::InvalidJournal(format!(
                    "task rebaseline transaction contains unknown artifact {relative}"
                )));
            }
            if metadata.is_file() {
                let maximum_file_bytes = if relative == "journal.json" {
                    MAX_JOURNAL_BYTES
                } else {
                    MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES
                };
                if metadata.len() > maximum_file_bytes {
                    return Err(WorkspaceTransactionError::InvalidJournal(format!(
                        "task rebaseline artifact {relative} exceeds {maximum_file_bytes} bytes"
                    )));
                }
                if relative != "journal.json" {
                    recovery_artifact_bytes = recovery_artifact_bytes
                        .checked_add(metadata.len())
                        .ok_or_else(|| {
                        WorkspaceTransactionError::InvalidJournal(
                            "task rebaseline artifact byte count overflowed".to_owned(),
                        )
                    })?;
                    if recovery_artifact_bytes > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_BYTES {
                        return Err(WorkspaceTransactionError::InvalidJournal(format!(
                            "task rebaseline artifacts exceed {MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_BYTES} bytes"
                        )));
                    }
                }
            }
            if metadata.is_dir() {
                pending.push(path);
            }
        }
    }
    Ok(())
}

fn task_rebaseline_records_are_recoverable_mixed(
    transaction: &Path,
    journal: &Journal,
    authority: &crate::task_rebaseline_transaction::TaskRebaselineTransactionSummary,
    current: &[crate::physical_inventory::PhysicalInventoryRecord],
) -> Result<bool, WorkspaceTransactionError> {
    let before = authority
        .physical_pre_entries
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let after = authority
        .physical_post_entries
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let actual = current
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let mut touched = BTreeSet::new();
    for replacement in &authority.source_replacements {
        touched.insert(replacement.document_locator.as_str());
    }
    for node in &authority.new_nodes {
        touched.insert(node.destination_node_locator.as_str());
        touched.insert(node.document_locator.as_str());
    }
    let all = before
        .keys()
        .chain(after.keys())
        .chain(actual.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    if all.iter().any(|locator| {
        !touched.contains(locator)
            && (before.get(locator) != after.get(locator)
                || actual.get(locator) != before.get(locator))
    }) {
        return Ok(false);
    }
    let mut artifact_bytes_read = 0_u64;

    for (index, node) in authority.new_nodes.iter().enumerate() {
        let Some(JournalStep::CreateTree { staged, digest, .. }) = journal.steps.get(index) else {
            return Ok(false);
        };
        let directory = node.destination_node_locator.as_str();
        let document = node.document_locator.as_str();
        let both_old = actual.get(directory) == before.get(directory)
            && actual.get(document) == before.get(document);
        let both_new = actual.get(directory) == after.get(directory)
            && actual.get(document) == after.get(document);
        let staged_digest = task_rebaseline_tree_digest_if_exists(
            &safe_join(transaction, staged)?,
            &mut artifact_bytes_read,
        )?;
        if !((both_old && staged_digest.as_deref() == Some(digest))
            || (both_new && staged_digest.is_none()))
        {
            return Ok(false);
        }
    }

    for (offset, replacement) in authority.source_replacements.iter().enumerate() {
        let step_index = authority.new_nodes.len() + offset;
        let Some(JournalStep::ReplaceFile {
            destination,
            staged,
            displaced,
            base_digest,
            next_digest,
        }) = journal.steps.get(step_index)
        else {
            return Ok(false);
        };
        let locator = replacement.document_locator.as_str();
        if destination != locator {
            return Ok(false);
        }
        let staged_digest = task_rebaseline_regular_file_digest_if_exists(
            &safe_join(transaction, staged)?,
            &mut artifact_bytes_read,
        )?;
        let displaced_digest = task_rebaseline_regular_file_digest_if_exists(
            &safe_join(transaction, displaced)?,
            &mut artifact_bytes_read,
        )?;
        let evidence_is_closed = if actual.get(locator) == before.get(locator) {
            staged_digest.as_deref() == Some(next_digest) && displaced_digest.is_none()
        } else if actual.get(locator) == after.get(locator) {
            staged_digest.is_none() && displaced_digest.as_deref() == Some(base_digest)
        } else if !actual.contains_key(locator) {
            staged_digest.as_deref() == Some(next_digest)
                && displaced_digest.as_deref() == Some(base_digest)
        } else {
            false
        };
        if !evidence_is_closed {
            return Ok(false);
        }
    }
    Ok(true)
}

#[allow(
    clippy::too_many_lines,
    reason = "the directional C/A and staged/holding artifact matrix is one recovery audit boundary"
)]
fn task_rebaseline_rollback_records_are_recoverable_mixed(
    transaction: &Path,
    journal: &Journal,
    authority: &crate::task_rebaseline_transaction::TaskRebaselineRollbackSummary,
    current: &[crate::physical_inventory::PhysicalInventoryRecord],
) -> Result<bool, WorkspaceTransactionError> {
    let before = authority
        .physical_pre_entries
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let after = authority
        .physical_post_entries
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let actual = current
        .iter()
        .map(|record| (record.locator.as_str(), record))
        .collect::<BTreeMap<_, _>>();
    let forward = &authority.forward_authority;
    let mut touched = BTreeSet::new();
    for replacement in &forward.source_replacements {
        touched.insert(replacement.document_locator.as_str());
    }
    for node in &forward.new_nodes {
        touched.insert(node.destination_node_locator.as_str());
        touched.insert(node.document_locator.as_str());
    }
    let all = before
        .keys()
        .chain(after.keys())
        .chain(actual.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    if all.iter().any(|locator| {
        !touched.contains(locator)
            && (before.get(locator) != after.get(locator)
                || actual.get(locator) != before.get(locator))
    }) {
        return Ok(false);
    }
    let mut artifact_bytes_read = 0_u64;
    for (index, replacement) in forward.source_replacements.iter().enumerate() {
        let Some(JournalStep::ReplaceFile {
            destination,
            staged,
            displaced,
            base_digest,
            next_digest,
        }) = journal.steps.get(index)
        else {
            return Ok(false);
        };
        let locator = replacement.document_locator.as_str();
        if destination != locator {
            return Ok(false);
        }
        let staged_digest = task_rebaseline_regular_file_digest_if_exists(
            &safe_join(transaction, staged)?,
            &mut artifact_bytes_read,
        )?;
        let displaced_digest = task_rebaseline_regular_file_digest_if_exists(
            &safe_join(transaction, displaced)?,
            &mut artifact_bytes_read,
        )?;
        let closed = if actual.get(locator) == before.get(locator) {
            staged_digest.as_deref() == Some(next_digest) && displaced_digest.is_none()
        } else if actual.get(locator) == after.get(locator) {
            staged_digest.is_none() && displaced_digest.as_deref() == Some(base_digest)
        } else if !actual.contains_key(locator) {
            staged_digest.as_deref() == Some(next_digest)
                && displaced_digest.as_deref() == Some(base_digest)
        } else {
            false
        };
        if !closed {
            return Ok(false);
        }
    }
    for (offset, node) in forward.new_nodes.iter().enumerate() {
        let index = forward.source_replacements.len() + offset;
        let Some(JournalStep::RemovePath {
            source,
            holding,
            digest,
        }) = journal.steps.get(index)
        else {
            return Ok(false);
        };
        if source != &node.destination_node_locator {
            return Ok(false);
        }
        let directory = node.destination_node_locator.as_str();
        let document = node.document_locator.as_str();
        let source_pre = actual.get(directory) == before.get(directory)
            && actual.get(document) == before.get(document);
        let source_post = actual.get(directory) == after.get(directory)
            && actual.get(document) == after.get(document);
        let holding_digest = task_rebaseline_tree_digest_if_exists(
            &safe_join(transaction, holding)?,
            &mut artifact_bytes_read,
        )?;
        if !((source_pre && holding_digest.is_none())
            || (source_post && holding_digest.as_deref() == Some(digest)))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn task_rebaseline_tree_digest_if_exists(
    path: &Path,
    artifact_bytes_read: &mut u64,
) -> Result<Option<String>, WorkspaceTransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if linked_or_reparse(&metadata) || !metadata.is_dir() => {
            Err(WorkspaceTransactionError::VerificationFailed(format!(
                "task rebaseline recovery evidence is not a regular non-link tree: {}",
                path.display()
            )))
        }
        Ok(_) => task_rebaseline_streaming_tree_digest(path, artifact_bytes_read).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn task_rebaseline_streaming_tree_digest(
    root: &Path,
    artifact_bytes_read: &mut u64,
) -> Result<String, WorkspaceTransactionError> {
    let mut directories = Vec::new();
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    let mut entry_count = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(WorkspaceTransactionError::Io)? {
            let entry = entry.map_err(WorkspaceTransactionError::Io)?;
            entry_count = entry_count.checked_add(1).ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "task rebaseline tree entry count overflowed".to_owned(),
                )
            })?;
            if entry_count > MAX_TRANSACTION_ENTRIES {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "task rebaseline tree entry count exceeds the recovery bound".to_owned(),
                ));
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).map_err(WorkspaceTransactionError::Io)?;
            if linked_or_reparse(&metadata) {
                return Err(WorkspaceTransactionError::InvalidJournal(format!(
                    "task rebaseline tree contains linked evidence: {}",
                    path.display()
                )));
            }
            let relative = components_string(
                path.strip_prefix(root)
                    .map_err(|_| WorkspaceTransactionError::PathEscape(path.clone()))?,
                &path,
            )?;
            if metadata.is_dir() {
                directories.push(relative);
                pending.push(path);
            } else if metadata.is_file() {
                if metadata.len() > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES {
                    return Err(WorkspaceTransactionError::InvalidJournal(format!(
                        "task rebaseline tree file exceeds {MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES} bytes"
                    )));
                }
                files.push((relative, path));
            } else {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "task rebaseline tree contains unsupported evidence".to_owned(),
                ));
            }
        }
    }
    directories.sort();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.tree.v1\0");
    for directory in directories {
        hasher.update(b"D\0");
        hasher.update(directory.as_bytes());
        hasher.update([0]);
    }
    for (relative, path) in files {
        hasher.update(b"F\0");
        hasher.update(relative.as_bytes());
        hasher.update([0]);
        hasher.update(task_rebaseline_streaming_file_sha256(
            &path,
            artifact_bytes_read,
        )?);
        hasher.update([0]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn task_rebaseline_streaming_file_digest(
    path: &Path,
    artifact_bytes_read: &mut u64,
) -> Result<String, WorkspaceTransactionError> {
    Ok(format!(
        "{:x}",
        task_rebaseline_streaming_file_sha256(path, artifact_bytes_read)?
    ))
}

fn task_rebaseline_streaming_file_sha256(
    path: &Path,
    artifact_bytes_read: &mut u64,
) -> Result<sha2::digest::Output<Sha256>, WorkspaceTransactionError> {
    let metadata = fs::symlink_metadata(path).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&metadata)
        || !metadata.is_file()
        || metadata.len() > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES
    {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "task rebaseline recovery file exceeds its closed kind or byte bound: {}",
            path.display()
        )));
    }
    let mut file = fs::File::open(path).map_err(WorkspaceTransactionError::Io)?;
    let opened_metadata = file.metadata().map_err(WorkspaceTransactionError::Io)?;
    if !opened_metadata.is_file()
        || opened_metadata.len() > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES
    {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "task rebaseline recovery file changed beyond its byte bound: {}",
            path.display()
        )));
    }
    let mut hasher = Sha256::new();
    let mut file_bytes_read = 0_u64;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let file_remaining =
            MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES.saturating_sub(file_bytes_read);
        let aggregate_remaining =
            MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_BYTES.saturating_sub(*artifact_bytes_read);
        let read_length =
            usize::try_from(file_remaining.min(aggregate_remaining).saturating_add(1))
                .unwrap_or(usize::MAX)
                .min(buffer.len());
        let read = file
            .read(&mut buffer[..read_length])
            .map_err(WorkspaceTransactionError::Io)?;
        if read == 0 {
            break;
        }
        let read = u64::try_from(read).unwrap_or(u64::MAX);
        file_bytes_read = file_bytes_read.checked_add(read).ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "task rebaseline recovery file byte count overflowed".to_owned(),
            )
        })?;
        *artifact_bytes_read = artifact_bytes_read.checked_add(read).ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "task rebaseline recovery artifact byte count overflowed".to_owned(),
            )
        })?;
        if file_bytes_read > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_FILE_BYTES
            || *artifact_bytes_read > MAX_TASK_REBASELINE_RECOVERY_ARTIFACT_BYTES
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "task rebaseline recovery artifacts exceed their byte bound".to_owned(),
            ));
        }
        hasher.update(&buffer[..usize::try_from(read).unwrap_or(buffer.len())]);
    }
    if file_bytes_read != opened_metadata.len() {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "task rebaseline recovery file changed while hashing: {}",
            path.display()
        )));
    }
    Ok(hasher.finalize())
}

fn task_rebaseline_regular_file_digest_if_exists(
    path: &Path,
    artifact_bytes_read: &mut u64,
) -> Result<Option<String>, WorkspaceTransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if linked_or_reparse(&metadata) || !metadata.is_file() => {
            Err(WorkspaceTransactionError::VerificationFailed(format!(
                "task rebaseline recovery evidence is not a regular non-link file: {}",
                path.display()
            )))
        }
        Ok(_) => task_rebaseline_streaming_file_digest(path, artifact_bytes_read).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn verify_task_rebaseline_journal_semantic_post(
    root: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task rebaseline semantic post-state inventory is invalid".to_owned(),
        ));
    }
    if let Some(authority) = &journal.task_rebaseline_authority {
        for replacement in &authority.source_replacements {
            if fs::read(safe_join(root, &replacement.document_locator)?)
                .map_err(WorkspaceTransactionError::Io)?
                != replacement.proposed_source.as_bytes()
            {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline recovery source bytes differ from C".to_owned(),
                ));
            }
        }
        for node in &authority.new_nodes {
            if fs::read(safe_join(root, &node.document_locator)?)
                .map_err(WorkspaceTransactionError::Io)?
                != node.exact_source.as_bytes()
            {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline recovery task-node bytes differ from C".to_owned(),
                ));
            }
        }
    } else if let Some(authority) = &journal.task_rebaseline_rollback_authority {
        for replacement in &authority.forward_authority.source_replacements {
            if fs::read(safe_join(root, &replacement.document_locator)?)
                .map_err(WorkspaceTransactionError::Io)?
                != replacement.original_source.as_bytes()
            {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline rollback recovery source bytes differ from exact A".to_owned(),
                ));
            }
        }
        for node in &authority.forward_authority.new_nodes {
            if non_link_path_exists(&safe_join(root, &node.destination_node_locator)?)? {
                return Err(WorkspaceTransactionError::VerificationFailed(
                    "task rebaseline rollback recovery left a generated node in A".to_owned(),
                ));
            }
        }
        let root_document =
            read_node_document(root).map_err(WorkspaceTransactionError::Document)?;
        if root_document.node_id != authority.workspace_root_node_id
            || root_document.revision != authority.workspace_root_post_document_revision
        {
            return Err(WorkspaceTransactionError::VerificationFailed(
                "task rebaseline rollback recovery root identity/revision differs from A"
                    .to_owned(),
            ));
        }
        let actual = read_workspace_revision(root).map_err(WorkspaceTransactionError::Revision)?;
        require_workspace_revision(&authority.post_workspace_revision, &actual)?;
    } else {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "task rebaseline semantic post-state authority is unavailable".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
fn prepare_journal(
    plan: &WorkspaceTransactionPlan,
    transaction: &Path,
) -> Result<Journal, WorkspaceTransactionError> {
    prepare_journal_with_finalization(plan, transaction, JournalFinalization::Core, None)
}

#[cfg(test)]
fn prepare_journal_with_finalization(
    plan: &WorkspaceTransactionPlan,
    transaction: &Path,
    finalization: JournalFinalization,
    external_receipt_destination: Option<&Path>,
) -> Result<Journal, WorkspaceTransactionError> {
    let journal = prepared_journal_metadata_with_finalization(
        plan,
        finalization,
        external_receipt_destination,
    )?;
    validate_journal_lifecycle_wire_limit(&journal, MAX_JOURNAL_BYTES)?;
    let materialized_steps = prepare_journal_steps(plan, transaction)?;
    if materialized_steps != journal.steps {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "materialized journal steps differ from preflight authority".to_owned(),
        ));
    }
    Ok(journal)
}

#[allow(
    clippy::too_many_lines,
    reason = "all schema-specific authority is frozen before the single staging boundary"
)]
fn prepared_journal_metadata_with_finalization(
    plan: &WorkspaceTransactionPlan,
    finalization: JournalFinalization,
    external_receipt_destination: Option<&Path>,
) -> Result<Journal, WorkspaceTransactionError> {
    validate_plan_scope_authority(plan)?;
    validate_trash_plan_authority(plan)?;
    validate_annotation_sidecar_plan_authority(plan)?;
    recheck_annotation_sidecar_plan_authority(plan)?;
    let task_rebaseline_snapshot_authority = plan
        .task_rebaseline_external_snapshot
        .as_ref()
        .map(|snapshot| {
            let canonical_root = snapshot
                .journal_authority_root()
                .to_str()
                .ok_or_else(|| {
                    WorkspaceTransactionError::Metadata(
                        "task rebaseline external snapshot path is not UTF-8".to_owned(),
                    )
                })?
                .to_owned();
            Ok(TaskRebaselineJournalSnapshotAuthority {
                canonical_root,
                physical_inventory: snapshot.binding().clone(),
                root_identity: snapshot.root_identity().clone(),
            })
        })
        .transpose()?;
    if task_rebaseline_snapshot_authority
        .as_ref()
        .is_some_and(|authority| {
            authority.canonical_root.is_empty()
                || authority.canonical_root.len() > 32_768
                || !Path::new(&authority.canonical_root).is_absolute()
        })
    {
        return Err(WorkspaceTransactionError::Metadata(
            "task rebaseline snapshot locator exceeds its private journal bound".to_owned(),
        ));
    }
    // Resolve and bound all absolute external authority before materializing
    // staged transaction payloads.
    let schema = if plan.task_rebaseline_rollback_authority.is_some() {
        JOURNAL_SCHEMA_V4
    } else {
        journal_schema_for_action(plan.action)
    };
    let task_rebaseline_direction = match schema {
        JOURNAL_SCHEMA_V3 => Some(TaskRebaselineJournalDirection::ApplyRebaseline),
        JOURNAL_SCHEMA_V4 => Some(TaskRebaselineJournalDirection::RollbackRebaseline),
        _ => None,
    };
    let steps = journal_steps_for_plan(plan);
    let authority_digest = journal_authority_digest(&JournalAuthorityDigestMaterial {
        schema,
        plan_id: &plan.plan_id,
        base_revision: &plan.base_revision,
        action: plan.action,
        path_changes: &plan.path_changes,
        document_changes: &plan.document_changes,
        scope_summary: plan.scope_summary.as_ref(),
        promotion_summary: plan.promotion_summary.as_ref(),
        task_promotion_authority: plan.task_promotion_authority.as_ref(),
        task_rebaseline_authority: plan.task_rebaseline_authority.as_ref(),
        task_rebaseline_snapshot_authority: task_rebaseline_snapshot_authority.as_ref(),
        task_rebaseline_commit_confirmation: plan.task_rebaseline_commit_confirmation.as_ref(),
        task_rebaseline_direction,
        task_rebaseline_rollback_authority: plan.task_rebaseline_rollback_authority.as_ref(),
        task_rebaseline_rollback_commit_confirmation: plan
            .task_rebaseline_rollback_commit_confirmation
            .as_ref(),
        identity_map: &plan.identity_map,
        captured_target: plan.captured_target.as_ref(),
        target_node_ids: &plan.target_node_ids,
        draft_sensitive_node_ids: &plan.draft_sensitive_node_ids,
        import_authority: plan.import_authority.as_ref(),
        annotation_sidecar_authority: plan.annotation_sidecar_authority.as_ref(),
        trash_item_changes: &plan.trash_item_changes,
        legacy_trash_migration_backup_authority: plan
            .legacy_trash_migration_backup
            .as_ref()
            .map(|backup| &backup.authority),
        finalization,
        external_receipt_destination: external_receipt_destination.and_then(Path::to_str),
        steps: &steps,
    })?;
    let lifecycle_digest =
        journal_lifecycle_digest(schema, &authority_digest, JournalState::Prepared, None)?;
    Ok(Journal {
        schema: schema.to_owned(),
        plan_id: plan.plan_id.clone(),
        state: JournalState::Prepared,
        base_revision: plan.base_revision.clone(),
        committed_revision: None,
        action: plan.action,
        path_changes: plan.path_changes.clone(),
        document_changes: plan.document_changes.clone(),
        scope_summary: plan.scope_summary.clone(),
        promotion_summary: plan.promotion_summary.clone(),
        task_promotion_authority: plan.task_promotion_authority.clone(),
        task_rebaseline_authority: plan.task_rebaseline_authority.clone(),
        task_rebaseline_snapshot_authority,
        task_rebaseline_commit_confirmation: plan.task_rebaseline_commit_confirmation.clone(),
        task_rebaseline_direction,
        task_rebaseline_rollback_authority: plan.task_rebaseline_rollback_authority.clone(),
        task_rebaseline_rollback_commit_confirmation: plan
            .task_rebaseline_rollback_commit_confirmation
            .clone(),
        identity_map: plan.identity_map.clone(),
        captured_target: plan.captured_target.clone(),
        target_node_ids: plan.target_node_ids.clone(),
        draft_sensitive_node_ids: plan.draft_sensitive_node_ids.clone(),
        import_authority: plan.import_authority.clone(),
        annotation_sidecar_authority: plan.annotation_sidecar_authority.clone(),
        trash_item_changes: plan.trash_item_changes.clone(),
        legacy_trash_migration_backup_authority: plan
            .legacy_trash_migration_backup
            .as_ref()
            .map(|backup| backup.authority.clone()),
        finalization,
        external_receipt_destination: external_receipt_destination
            .and_then(Path::to_str)
            .map(ToOwned::to_owned),
        authority_digest,
        lifecycle_digest,
        commit_digest: None,
        steps,
    })
}

fn validate_trash_plan_authority(
    plan: &WorkspaceTransactionPlan,
) -> Result<(), WorkspaceTransactionError> {
    let expected_disposition = match plan.action {
        StructuralAction::Trash => Some(crate::TrashPlanDisposition::Stored),
        StructuralAction::Restore => Some(crate::TrashPlanDisposition::Restored),
        StructuralAction::PermanentDelete => Some(crate::TrashPlanDisposition::PermanentlyDeleted),
        StructuralAction::TrashMigration => Some(crate::TrashPlanDisposition::Migrated),
        _ => None,
    };
    if expected_disposition.is_some() == plan.trash_item_changes.is_empty() {
        return Err(WorkspaceTransactionError::TrashReconciliation(
            "transaction action and Trash plan authority disagree".to_owned(),
        ));
    }
    match (plan.action, &plan.legacy_trash_migration_backup) {
        (StructuralAction::TrashMigration, Some(backup)) => {
            verify_legacy_trash_migration_backup(
                &plan.workspace_root,
                backup,
                &plan.base_revision,
            )?;
        }
        (StructuralAction::TrashMigration, None) => {
            return Err(WorkspaceTransactionError::LegacyTrashMigrationBackupRequired);
        }
        (_, Some(_)) => {
            return Err(
                WorkspaceTransactionError::InvalidLegacyTrashMigrationBackup(
                    "backup evidence is attached to a non-migration plan".to_owned(),
                ),
            );
        }
        (_, None) => {}
    }
    let mut item_ids = BTreeSet::new();
    for change in &plan.trash_item_changes {
        if Some(change.disposition) != expected_disposition
            || !item_ids.insert(change.manifest.trash_item_id())
        {
            return Err(WorkspaceTransactionError::TrashReconciliation(
                "Trash plan has an invalid disposition or duplicate item ID".to_owned(),
            ));
        }
        let has_destination = change.destination_node_id.is_some()
            && change
                .destination_name
                .as_deref()
                .is_some_and(|name| !name.is_empty());
        if (plan.action == StructuralAction::Restore) != has_destination {
            return Err(WorkspaceTransactionError::TrashReconciliation(
                "Trash plan destination evidence disagrees with its action".to_owned(),
            ));
        }
    }
    Ok(())
}

fn journal_steps_for_plan(plan: &WorkspaceTransactionPlan) -> Vec<JournalStep> {
    plan.steps
        .iter()
        .enumerate()
        .map(|(index, step)| match step {
            PlannedStep::CreateTree {
                destination,
                payload,
            } => JournalStep::CreateTree {
                destination: destination.clone(),
                staged: format!("staged/{index}"),
                digest: payload.digest.clone(),
            },
            PlannedStep::CreateFile {
                destination,
                next_digest,
                ..
            } => JournalStep::CreateFile {
                destination: destination.clone(),
                staged: format!("staged/{index}.file"),
                next_digest: next_digest.clone(),
            },
            PlannedStep::MovePath {
                source,
                destination,
                digest,
            } => JournalStep::MovePath {
                source: source.clone(),
                destination: destination.clone(),
                holding: format!("holding/{index}"),
                digest: digest.clone(),
            },
            PlannedStep::RemovePath { source, digest } => JournalStep::RemovePath {
                source: source.clone(),
                holding: format!("removed/{index}"),
                digest: digest.clone(),
            },
            PlannedStep::ReplaceFile {
                destination,
                base_digest,
                next_digest,
                ..
            } => JournalStep::ReplaceFile {
                destination: destination.clone(),
                staged: format!("staged/{index}.file"),
                displaced: format!("displaced/{index}.file"),
                base_digest: base_digest.clone(),
                next_digest: next_digest.clone(),
            },
        })
        .collect()
}

fn prepare_journal_steps(
    plan: &WorkspaceTransactionPlan,
    transaction: &Path,
) -> Result<Vec<JournalStep>, WorkspaceTransactionError> {
    let mut steps = Vec::new();
    for (index, step) in plan.steps.iter().enumerate() {
        match step {
            PlannedStep::CreateTree {
                destination,
                payload,
            } => {
                let staged = format!("staged/{index}");
                let staged_path = safe_join(transaction, &staged)?;
                materialize_payload(&staged_path, payload)?;
                require_path_digest(&staged_path, &payload.digest)?;
                steps.push(JournalStep::CreateTree {
                    destination: destination.clone(),
                    staged,
                    digest: payload.digest.clone(),
                });
            }
            PlannedStep::CreateFile {
                destination,
                next_digest,
                next_bytes,
            } => {
                let destination_path = pre_apply_path(&plan.workspace_root, destination, &steps)?;
                require_path_absent(&destination_path)?;
                let staged = format!("staged/{index}.file");
                let staged_path = safe_join(transaction, &staged)?;
                write_verified_file(&staged_path, next_bytes, next_digest)?;
                steps.push(JournalStep::CreateFile {
                    destination: destination.clone(),
                    staged,
                    next_digest: next_digest.clone(),
                });
            }
            PlannedStep::MovePath {
                source,
                destination,
                digest,
            } => {
                let source_path = pre_apply_path(&plan.workspace_root, source, &steps)?;
                require_path_digest(&source_path, digest)?;
                steps.push(JournalStep::MovePath {
                    source: source.clone(),
                    destination: destination.clone(),
                    holding: format!("holding/{index}"),
                    digest: digest.clone(),
                });
            }
            PlannedStep::RemovePath { source, digest } => {
                let source_path = pre_apply_path(&plan.workspace_root, source, &steps)?;
                require_path_digest(&source_path, digest)?;
                steps.push(JournalStep::RemovePath {
                    source: source.clone(),
                    holding: format!("removed/{index}"),
                    digest: digest.clone(),
                });
            }
            PlannedStep::ReplaceFile {
                destination,
                base_digest,
                next_digest,
                next_bytes,
            } => {
                let destination_path = pre_apply_path(&plan.workspace_root, destination, &steps)?;
                require_path_digest(&destination_path, base_digest)?;
                let staged = format!("staged/{index}.file");
                let staged_path = safe_join(transaction, &staged)?;
                write_verified_file(&staged_path, next_bytes, next_digest)?;
                steps.push(JournalStep::ReplaceFile {
                    destination: destination.clone(),
                    staged,
                    displaced: format!("displaced/{index}.file"),
                    base_digest: base_digest.clone(),
                    next_digest: next_digest.clone(),
                });
            }
        }
    }
    Ok(steps)
}

fn journal_matches_workspace_plan(journal: &Journal, plan: &WorkspaceTransactionPlan) -> bool {
    let expected_schema = if plan.task_rebaseline_rollback_authority.is_some() {
        JOURNAL_SCHEMA_V4
    } else {
        journal_schema_for_action(plan.action)
    };
    journal.schema == expected_schema
        && journal.plan_id == plan.plan_id
        && journal.base_revision == plan.base_revision
        && journal.action == plan.action
        && journal.path_changes == plan.path_changes
        && (journal.schema == JOURNAL_SCHEMA_V1
            || journal.document_changes == plan.document_changes)
        && journal.scope_summary == plan.scope_summary
        && journal.promotion_summary == plan.promotion_summary
        && journal.task_promotion_authority == plan.task_promotion_authority
        && journal.task_rebaseline_authority == plan.task_rebaseline_authority
        && journal.task_rebaseline_rollback_authority == plan.task_rebaseline_rollback_authority
        && journal.task_rebaseline_direction
            == match journal.schema.as_str() {
                JOURNAL_SCHEMA_V3 => Some(TaskRebaselineJournalDirection::ApplyRebaseline),
                JOURNAL_SCHEMA_V4 => Some(TaskRebaselineJournalDirection::RollbackRebaseline),
                _ => None,
            }
        && task_rebaseline_journal_snapshot_matches_plan(journal, plan)
        && (journal.task_rebaseline_commit_confirmation == plan.task_rebaseline_commit_confirmation
            || (plan.action == StructuralAction::TaskRebaseline
                && plan.task_rebaseline_commit_confirmation.is_none()
                && journal.task_rebaseline_commit_confirmation.is_some()))
        && (journal.task_rebaseline_rollback_commit_confirmation
            == plan.task_rebaseline_rollback_commit_confirmation
            || (journal.schema == JOURNAL_SCHEMA_V4
                && plan.task_rebaseline_rollback_commit_confirmation.is_none()
                && journal
                    .task_rebaseline_rollback_commit_confirmation
                    .is_some()))
        && journal.identity_map == plan.identity_map
        && journal.captured_target == plan.captured_target
        && journal.target_node_ids == plan.target_node_ids
        && journal.draft_sensitive_node_ids == plan.draft_sensitive_node_ids
        && journal.import_authority == plan.import_authority
        && journal.annotation_sidecar_authority == plan.annotation_sidecar_authority
        && journal.trash_item_changes == plan.trash_item_changes
        && journal.legacy_trash_migration_backup_authority
            == plan
                .legacy_trash_migration_backup
                .as_ref()
                .map(|backup| backup.authority.clone())
        && journal.finalization == JournalFinalization::Core
        && journal.external_receipt_destination.is_none()
        && journal.steps.len() == plan.steps.len()
        && journal.steps.iter().zip(&plan.steps).enumerate().all(
            |(index, (journal_step, planned_step))| {
                journal_step_matches_planned_step(index, journal_step, planned_step)
            },
        )
}

fn task_rebaseline_journal_snapshot_matches_plan(
    journal: &Journal,
    plan: &WorkspaceTransactionPlan,
) -> bool {
    match (
        &journal.task_rebaseline_snapshot_authority,
        &plan.task_rebaseline_external_snapshot,
    ) {
        (None, None) => true,
        (Some(authority), Some(snapshot)) => {
            snapshot.journal_authority_root().to_str() == Some(authority.canonical_root.as_str())
                && snapshot.binding() == &authority.physical_inventory
                && snapshot.root_identity() == &authority.root_identity
        }
        _ => false,
    }
}

fn validate_annotation_sidecar_plan_authority(
    plan: &WorkspaceTransactionPlan,
) -> Result<(), WorkspaceTransactionError> {
    let Some(authority) = &plan.annotation_sidecar_authority else {
        if plan.action == StructuralAction::Annotation {
            return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
        }
        return Ok(());
    };
    if plan.action != StructuralAction::Annotation
        || authority.workspace_revision != plan.base_revision
        || !authority.completeness.is_complete()
    {
        return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
    }
    validate_workspace_journal_path(&authority.destination)?;
    if Path::new(&authority.destination)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(ANNOTATIONS_FILE_NAME)
    {
        return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
    }
    let mut matches = plan
        .steps
        .iter()
        .filter(|step| match (step, &authority.expected_state) {
            (
                PlannedStep::ReplaceFile {
                    destination,
                    base_digest,
                    ..
                },
                AnnotationSidecarExpectedState::Present { sha256 },
            ) => destination == &authority.destination && base_digest == sha256,
            (
                PlannedStep::CreateFile { destination, .. },
                AnnotationSidecarExpectedState::ConfirmedAbsent,
            ) => destination == &authority.destination,
            _ => false,
        });
    if matches.next().is_none() || matches.next().is_some() {
        return Err(WorkspaceTransactionError::AnnotationSidecarAuthorityMismatch);
    }
    if let AnnotationSidecarExpectedState::Present { sha256 } = &authority.expected_state {
        validate_import_digest(sha256, "annotation sidecar digest")?;
    }
    Ok(())
}

fn recheck_annotation_sidecar_plan_authority(
    plan: &WorkspaceTransactionPlan,
) -> Result<(), WorkspaceTransactionError> {
    let Some(authority) = &plan.annotation_sidecar_authority else {
        return Ok(());
    };
    let sidecar = safe_join(&plan.workspace_root, &authority.destination)?;
    let node_directory = sidecar
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(sidecar.clone()))?;
    require_no_annotation_conflict_copies(node_directory)?;
    let (actual, _) = observe_annotation_sidecar(&sidecar, authority.node_id)?;
    if actual != authority.expected_state {
        return Err(WorkspaceTransactionError::AnnotationSidecarChanged);
    }
    Ok(())
}

fn journal_step_matches_planned_step(
    index: usize,
    journal: &JournalStep,
    planned: &PlannedStep,
) -> bool {
    match (journal, planned) {
        (
            JournalStep::CreateTree {
                destination,
                staged,
                digest,
            },
            PlannedStep::CreateTree {
                destination: planned_destination,
                payload,
            },
        ) => {
            destination == planned_destination
                && staged == &format!("staged/{index}")
                && digest == &payload.digest
        }
        (
            JournalStep::CreateFile {
                destination,
                staged,
                next_digest,
            },
            PlannedStep::CreateFile {
                destination: planned_destination,
                next_digest: planned_digest,
                ..
            },
        ) => {
            destination == planned_destination
                && staged == &format!("staged/{index}.file")
                && next_digest == planned_digest
        }
        (
            JournalStep::MovePath {
                source,
                destination,
                holding,
                digest,
            },
            PlannedStep::MovePath {
                source: planned_source,
                destination: planned_destination,
                digest: planned_digest,
            },
        ) => {
            source == planned_source
                && destination == planned_destination
                && holding == &format!("holding/{index}")
                && digest == planned_digest
        }
        (
            JournalStep::RemovePath {
                source,
                holding,
                digest,
            },
            PlannedStep::RemovePath {
                source: planned_source,
                digest: planned_digest,
            },
        ) => {
            source == planned_source
                && holding == &format!("removed/{index}")
                && digest == planned_digest
        }
        (
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            },
            PlannedStep::ReplaceFile {
                destination: planned_destination,
                base_digest: planned_base,
                next_digest: planned_next,
                ..
            },
        ) => {
            destination == planned_destination
                && staged == &format!("staged/{index}.file")
                && displaced == &format!("displaced/{index}.file")
                && base_digest == planned_base
                && next_digest == planned_next
        }
        _ => false,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the three version-exclusive journal digest shapes are intentionally adjacent"
)]
fn journal_authority_digest(
    material: &JournalAuthorityDigestMaterial<'_>,
) -> Result<String, WorkspaceTransactionError> {
    let bytes = if material.schema == JOURNAL_SCHEMA_V1 {
        if material.promotion_summary.is_some()
            || material.task_promotion_authority.is_some()
            || material.task_rebaseline_authority.is_some()
            || material.task_rebaseline_snapshot_authority.is_some()
            || material.task_rebaseline_commit_confirmation.is_some()
            || material.task_rebaseline_direction.is_some()
            || material.task_rebaseline_rollback_authority.is_some()
            || material
                .task_rebaseline_rollback_commit_confirmation
                .is_some()
            || matches!(
                material.action,
                StructuralAction::TaskPromotion | StructuralAction::TaskRebaseline
            )
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "v1 journal cannot carry task promotion or rebaseline authority".to_owned(),
            ));
        }
        serde_json::to_vec(&(
            material.schema,
            material.plan_id,
            material.base_revision,
            material.action,
            material.path_changes,
            (
                material.scope_summary,
                material.identity_map,
                material.captured_target,
                material.target_node_ids,
                material.draft_sensitive_node_ids,
            ),
            material.import_authority,
            material.annotation_sidecar_authority,
            material.trash_item_changes,
            material.legacy_trash_migration_backup_authority,
            material.finalization,
            material.external_receipt_destination,
            material.steps,
        ))
    } else if material.schema == JOURNAL_SCHEMA_V2 {
        if material.action != StructuralAction::TaskPromotion
            || material.promotion_summary.is_none()
            || material.task_promotion_authority.is_none()
            || material.task_rebaseline_authority.is_some()
            || material.task_rebaseline_snapshot_authority.is_some()
            || material.task_rebaseline_commit_confirmation.is_some()
            || material.task_rebaseline_direction.is_some()
            || material.task_rebaseline_rollback_authority.is_some()
            || material
                .task_rebaseline_rollback_commit_confirmation
                .is_some()
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "v2 journal is reserved for a complete task promotion".to_owned(),
            ));
        }
        serde_json::to_vec(&(
            material.schema,
            material.plan_id,
            material.base_revision,
            material.action,
            material.path_changes,
            material.document_changes,
            (
                material.scope_summary,
                material.promotion_summary,
                material.task_promotion_authority,
                material.identity_map,
                material.captured_target,
                material.target_node_ids,
                material.draft_sensitive_node_ids,
            ),
            material.import_authority,
            material.annotation_sidecar_authority,
            material.trash_item_changes,
            material.legacy_trash_migration_backup_authority,
            material.finalization,
            material.external_receipt_destination,
            material.steps,
        ))
    } else if material.schema == JOURNAL_SCHEMA_V3 {
        if material.action != StructuralAction::TaskRebaseline
            || material.task_rebaseline_authority.is_none()
            || material.task_rebaseline_snapshot_authority.is_none()
            || material.task_rebaseline_commit_confirmation.is_none()
            || material.task_rebaseline_direction
                != Some(TaskRebaselineJournalDirection::ApplyRebaseline)
            || material.task_rebaseline_rollback_authority.is_some()
            || material
                .task_rebaseline_rollback_commit_confirmation
                .is_some()
            || material.promotion_summary.is_some()
            || material.task_promotion_authority.is_some()
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "v3 journal is reserved for one complete task rebaseline".to_owned(),
            ));
        }
        serde_json::to_vec(&(
            material.schema,
            material.plan_id,
            material.base_revision,
            material.action,
            material.path_changes,
            material.document_changes,
            (
                material.scope_summary,
                material.task_rebaseline_authority,
                material.task_rebaseline_snapshot_authority,
                material.task_rebaseline_commit_confirmation,
                material.identity_map,
                material.captured_target,
                material.target_node_ids,
                material.draft_sensitive_node_ids,
            ),
            material.import_authority,
            material.annotation_sidecar_authority,
            material.trash_item_changes,
            material.legacy_trash_migration_backup_authority,
            material.finalization,
            material.external_receipt_destination,
            material.steps,
        ))
    } else if material.schema == JOURNAL_SCHEMA_V4 {
        if material.action != StructuralAction::TaskRebaseline
            || material.task_rebaseline_direction
                != Some(TaskRebaselineJournalDirection::RollbackRebaseline)
            || material.task_rebaseline_rollback_authority.is_none()
            || material.task_rebaseline_snapshot_authority.is_none()
            || material
                .task_rebaseline_rollback_commit_confirmation
                .is_none()
            || material.task_rebaseline_authority.is_some()
            || material.task_rebaseline_commit_confirmation.is_some()
            || material.promotion_summary.is_some()
            || material.task_promotion_authority.is_some()
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "v4 journal is reserved for one complete task rebaseline exact rollback".to_owned(),
            ));
        }
        serde_json::to_vec(&(
            material.schema,
            material.plan_id,
            material.base_revision,
            material.action,
            material.task_rebaseline_direction,
            material.path_changes,
            material.document_changes,
            (
                material.scope_summary,
                material.task_rebaseline_rollback_authority,
                material.task_rebaseline_snapshot_authority,
                material.task_rebaseline_rollback_commit_confirmation,
                material.identity_map,
                material.captured_target,
                material.target_node_ids,
                material.draft_sensitive_node_ids,
            ),
            material.import_authority,
            material.annotation_sidecar_authority,
            material.trash_item_changes,
            material.legacy_trash_migration_backup_authority,
            material.finalization,
            material.external_receipt_destination,
            material.steps,
        ))
    } else {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "unsupported schema {}",
            material.schema
        )));
    }
    .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&bytes))
}

fn committed_transaction_from_journal(
    journal: &Journal,
) -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError> {
    let revision = journal.committed_revision.clone().ok_or_else(|| {
        WorkspaceTransactionError::InvalidJournal(
            "committed journal has no committed revision".to_owned(),
        )
    })?;
    Ok(CommittedWorkspaceTransaction {
        plan_id: journal.plan_id.clone(),
        action: journal.action,
        base_revision: journal.base_revision.clone(),
        revision,
        path_changes: journal.path_changes.clone(),
        scope_summary: journal.scope_summary.clone(),
        promotion_summary: journal.promotion_summary.clone(),
        identity_map: journal.identity_map.clone(),
        captured_target: journal.captured_target.clone(),
        target_node_ids: journal.target_node_ids.clone(),
        draft_sensitive_node_ids: journal.draft_sensitive_node_ids.clone(),
        import_authority: journal.import_authority.clone(),
    })
}

fn mark_journal_committed(
    journal: &mut Journal,
    revision: WorkspaceRevision,
) -> Result<(), WorkspaceTransactionError> {
    let commit_digest =
        journal_commit_digest(&journal.schema, &journal.authority_digest, &revision)?;
    let lifecycle_digest = journal_lifecycle_digest(
        &journal.schema,
        &journal.authority_digest,
        JournalState::Committed,
        Some(&revision),
    )?;
    journal.state = JournalState::Committed;
    journal.committed_revision = Some(revision);
    journal.lifecycle_digest = lifecycle_digest;
    journal.commit_digest = Some(commit_digest);
    Ok(())
}

fn mark_journal_applying(journal: &mut Journal) -> Result<(), WorkspaceTransactionError> {
    journal.state = JournalState::Applying;
    journal.lifecycle_digest = journal_lifecycle_digest(
        &journal.schema,
        &journal.authority_digest,
        JournalState::Applying,
        None,
    )?;
    Ok(())
}

fn journal_lifecycle_digest(
    schema: &str,
    authority_digest: &str,
    state: JournalState,
    committed_revision: Option<&WorkspaceRevision>,
) -> Result<String, WorkspaceTransactionError> {
    let material = serde_json::to_vec(&(
        schema,
        "lifecycle",
        authority_digest,
        state,
        committed_revision,
    ))
    .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&material))
}

fn journal_commit_digest(
    schema: &str,
    authority_digest: &str,
    revision: &WorkspaceRevision,
) -> Result<String, WorkspaceTransactionError> {
    let material = serde_json::to_vec(&(schema, "committed", authority_digest, revision))
        .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&material))
}

fn pre_apply_path(
    root: &Path,
    relative: &str,
    previous_steps: &[JournalStep],
) -> Result<PathBuf, WorkspaceTransactionError> {
    let mut path = PathBuf::from(relative);
    for step in previous_steps.iter().rev() {
        let JournalStep::MovePath {
            source,
            destination,
            ..
        } = step
        else {
            continue;
        };
        let destination = Path::new(destination);
        if let Ok(suffix) = path.strip_prefix(destination) {
            path = Path::new(source).join(suffix);
        }
    }
    let relative = path
        .to_str()
        .ok_or_else(|| WorkspaceTransactionError::NonUtf8Path(path.clone()))?
        .replace('\\', "/");
    safe_join(root, &relative)
}

fn materialize_payload(
    root: &Path,
    payload: &TreePayload,
) -> Result<(), WorkspaceTransactionError> {
    durable_create_dir_all(root)?;
    for directory in &payload.directories {
        durable_create_dir_all(&safe_join(root, directory)?)?;
    }
    for file in &payload.files {
        let path = safe_join(root, &file.path)?;
        let digest = format!("{:x}", Sha256::digest(&file.bytes));
        write_verified_file(&path, &file.bytes, &digest)?;
    }
    Ok(())
}

fn write_verified_file(
    path: &Path,
    bytes: &[u8],
    expected_digest: &str,
) -> Result<(), WorkspaceTransactionError> {
    let parent = path
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(path.to_path_buf()))?;
    durable_create_dir_all(parent)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(WorkspaceTransactionError::Io)?;
    file.write_all(bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    file.flush().map_err(WorkspaceTransactionError::Io)?;
    file.sync_all().map_err(WorkspaceTransactionError::Io)?;
    drop(file);
    sync_directory(parent)?;
    require_path_digest(path, expected_digest)
}

fn apply_journal_steps(
    root: &Path,
    transaction: &Path,
    steps: &[JournalStep],
    fail_after_steps: Option<usize>,
) -> Result<(), WorkspaceTransactionError> {
    for (index, step) in steps.iter().enumerate() {
        if fail_after_steps == Some(index) {
            return Err(WorkspaceTransactionError::InjectedFailure(index));
        }
        match step {
            JournalStep::CreateTree {
                destination,
                staged,
                digest,
            } => {
                let destination = safe_join(root, destination)?;
                require_portable_destination_available(&destination)?;
                let staged = safe_join(transaction, staged)?;
                require_path_digest(&staged, digest)?;
                create_parent(&destination)?;
                durable_rename(&staged, &destination)?;
                require_path_digest(&destination, digest)?;
            }
            JournalStep::RemovePath {
                source,
                holding,
                digest,
            } => {
                let source = safe_join(root, source)?;
                let holding = safe_join(transaction, holding)?;
                require_path_digest(&source, digest)?;
                require_path_absent(&holding)?;
                create_parent(&holding)?;
                durable_rename(&source, &holding)?;
                require_path_digest(&holding, digest)?;
                require_missing_path(&source)?;
            }
            JournalStep::CreateFile {
                destination,
                staged,
                next_digest,
            } => {
                let destination = safe_join(root, destination)?;
                require_path_absent(&destination)?;
                let staged = safe_join(transaction, staged)?;
                require_path_digest(&staged, next_digest)?;
                create_parent(&destination)?;
                durable_rename(&staged, &destination)?;
                require_path_digest(&destination, next_digest)?;
            }
            JournalStep::MovePath {
                source,
                destination,
                holding,
                digest,
            } => {
                let source = safe_join(root, source)?;
                let destination = safe_join(root, destination)?;
                let holding = safe_join(transaction, holding)?;
                require_path_digest(&source, digest)?;
                if destination.exists()
                    && fs::canonicalize(&destination).ok() != fs::canonicalize(&source).ok()
                {
                    return Err(WorkspaceTransactionError::DestinationExists(destination));
                }
                create_parent(&holding)?;
                durable_rename(&source, &holding)?;
                create_parent(&destination)?;
                durable_rename(&holding, &destination)?;
                require_path_digest(&destination, digest)?;
            }
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            } => {
                let destination = safe_join(root, destination)?;
                let staged = safe_join(transaction, staged)?;
                let displaced = safe_join(transaction, displaced)?;
                require_path_digest(&destination, base_digest)?;
                require_path_digest(&staged, next_digest)?;
                create_parent(&displaced)?;
                durable_rename(&destination, &displaced)?;
                durable_rename(&staged, &destination)?;
                require_path_digest(&destination, next_digest)?;
            }
        }
    }
    if fail_after_steps == Some(steps.len()) {
        return Err(WorkspaceTransactionError::InjectedFailure(steps.len()));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn rollback_journal(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    for (index, step) in journal.steps.iter().enumerate().rev() {
        match step {
            JournalStep::CreateTree {
                destination,
                staged,
                digest,
            } => {
                let destination = safe_join(root, destination)?;
                let staged = safe_join(transaction, staged)?;
                match (digest_if_exists(&destination)?, digest_if_exists(&staged)?) {
                    (Some(actual), None) if actual == *digest => {
                        create_parent(&staged)?;
                        durable_rename(&destination, &staged)?;
                    }
                    (None, Some(actual)) if actual == *digest => {}
                    _ => return Err(ambiguous_recovery(&destination)),
                }
            }
            JournalStep::CreateFile {
                destination,
                staged,
                next_digest,
            } => {
                let destination = safe_join(root, destination)?;
                let staged = safe_join(transaction, staged)?;
                match (digest_if_exists(&destination)?, digest_if_exists(&staged)?) {
                    (Some(actual), None) if actual == *next_digest => {
                        create_parent(&staged)?;
                        durable_rename(&destination, &staged)?;
                    }
                    (None, Some(actual)) if actual == *next_digest => {}
                    _ => return Err(ambiguous_recovery(&destination)),
                }
            }
            JournalStep::MovePath {
                source,
                destination,
                holding,
                digest,
            } => {
                let source = safe_join(root, source)?;
                let destination = safe_join(root, destination)?;
                let holding = safe_join(transaction, holding)?;
                let states = (
                    digest_if_exists(&source)?,
                    digest_if_exists(&destination)?,
                    digest_if_exists(&holding)?,
                );
                let pre_apply_source = pre_apply_path(
                    root,
                    match step {
                        JournalStep::MovePath { source, .. } => source,
                        _ => unreachable!(),
                    },
                    &journal.steps[..index],
                )?;
                match states {
                    (Some(actual), None, None) if actual == *digest => {}
                    (None, None, None)
                        if digest_if_exists(&pre_apply_source)?.as_deref() == Some(digest) => {}
                    (None, Some(actual), None) if actual == *digest => {
                        create_parent(&source)?;
                        durable_rename(&destination, &source)?;
                    }
                    (None, None, Some(actual)) if actual == *digest => {
                        create_parent(&source)?;
                        durable_rename(&holding, &source)?;
                    }
                    _ => return Err(ambiguous_recovery(&source)),
                }
            }
            JournalStep::RemovePath {
                source,
                holding,
                digest,
            } => {
                let source = safe_join(root, source)?;
                let holding = safe_join(transaction, holding)?;
                match (digest_if_exists(&source)?, digest_if_exists(&holding)?) {
                    (Some(actual), None) if actual == *digest => {}
                    (None, Some(actual)) if actual == *digest => {
                        create_parent(&source)?;
                        durable_rename(&holding, &source)?;
                    }
                    _ => return Err(ambiguous_recovery(&source)),
                }
            }
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            } => {
                let destination = safe_join(root, destination)?;
                let staged = safe_join(transaction, staged)?;
                let displaced = safe_join(transaction, displaced)?;
                let states = (
                    digest_if_exists(&destination)?,
                    digest_if_exists(&staged)?,
                    digest_if_exists(&displaced)?,
                );
                let pre_apply_destination = pre_apply_path(
                    root,
                    match step {
                        JournalStep::ReplaceFile { destination, .. } => destination,
                        _ => unreachable!(),
                    },
                    &journal.steps[..index],
                )?;
                match states {
                    (Some(actual), Some(next), None)
                        if actual == *base_digest && next == *next_digest => {}
                    (None, Some(next), Some(base))
                        if next == *next_digest && base == *base_digest =>
                    {
                        create_parent(&destination)?;
                        durable_rename(&displaced, &destination)?;
                    }
                    (Some(next), None, Some(base))
                        if next == *next_digest && base == *base_digest =>
                    {
                        create_parent(&staged)?;
                        durable_rename(&destination, &staged)?;
                        durable_rename(&displaced, &destination)?;
                    }
                    (None, Some(next), None)
                        if next == *next_digest
                            && digest_if_exists(&pre_apply_destination)?.as_deref()
                                == Some(base_digest) => {}
                    _ => return Err(ambiguous_recovery(&destination)),
                }
            }
        }
    }
    Ok(())
}

fn verify_plan_outcome(plan: &WorkspaceTransactionPlan) -> Result<(), WorkspaceTransactionError> {
    let inventory = scan_workspace(&plan.workspace_root);
    if !inventory.is_valid() {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "final workspace inventory is invalid".to_owned(),
        ));
    }
    let by_id = inventory
        .nodes
        .iter()
        .filter_map(|node| node.id.map(|id| (id, &node.path)))
        .collect::<BTreeMap<_, _>>();
    let trashed_by_id = inventory
        .trash_items
        .iter()
        .flat_map(|item| {
            let item_root = relative_string(&plan.workspace_root, &item.item_path)
                .unwrap_or_else(|_| String::new());
            item.node_locators.iter().map(move |(id, locator)| {
                (
                    *id,
                    format!(
                        "{item_root}/{}/{locator}",
                        crate::TRASH_ITEM_PAYLOAD_DIRECTORY_NAME
                    ),
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    for change in &plan.path_changes {
        let reached = if plan.action == StructuralAction::Trash {
            trashed_by_id.get(&change.node_id) == Some(&change.new_path)
        } else if plan.action == StructuralAction::TaskPromotion {
            let authority = plan.task_promotion_authority.as_ref().ok_or_else(|| {
                WorkspaceTransactionError::VerificationFailed(
                    "task promotion physical destination authority is unavailable".to_owned(),
                )
            })?;
            let expected = safe_join(&plan.workspace_root, &authority.destination_node_path)?;
            by_id.get(&change.node_id).copied() == Some(&expected)
        } else {
            let expected = safe_join(&plan.workspace_root, &change.new_path)?;
            by_id.get(&change.node_id).copied() == Some(&expected)
        };
        if !reached {
            return Err(WorkspaceTransactionError::VerificationFailed(format!(
                "node {} did not reach {}",
                change.node_id, change.new_path
            )));
        }
    }
    for change in &plan.document_changes {
        let document = safe_join(&plan.workspace_root, &change.path)?;
        let source = fs::read_to_string(&document).map_err(WorkspaceTransactionError::Io)?;
        let revision = DocumentRevision::from_source(&source);
        if revision != change.next_revision {
            return Err(WorkspaceTransactionError::VerificationFailed(format!(
                "document {} has revision {revision} instead of {}",
                change.path, change.next_revision
            )));
        }
    }
    for (index, step) in plan.steps.iter().enumerate() {
        if let PlannedStep::CreateFile {
            destination,
            next_digest,
            ..
        }
        | PlannedStep::ReplaceFile {
            destination,
            next_digest,
            ..
        } = step
        {
            require_path_digest(&safe_join(&plan.workspace_root, destination)?, next_digest)?;
        }
        if let PlannedStep::RemovePath { source, .. } = step {
            let recreated = plan.steps[index.saturating_add(1)..].iter().any(|later| {
                matches!(
                    later,
                    PlannedStep::CreateTree { destination, .. }
                        | PlannedStep::CreateFile { destination, .. }
                        if destination == source
                )
            });
            if !recreated {
                require_missing_path(&safe_join(&plan.workspace_root, source)?)?;
            }
        }
    }
    if plan.action == StructuralAction::TaskPromotion {
        verify_task_promotion_plan_outcome(plan)?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn verify_task_promotion_plan_outcome(
    plan: &WorkspaceTransactionPlan,
) -> Result<(), WorkspaceTransactionError> {
    if !promotion_authority_matches(plan) {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion plan authority changed before final verification".to_owned(),
        ));
    }
    let summary = plan.promotion_summary.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::VerificationFailed(
            "task promotion summary is unavailable".to_owned(),
        )
    })?;
    let authority = plan.task_promotion_authority.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::VerificationFailed(
            "task promotion private authority is unavailable".to_owned(),
        )
    })?;
    let source_path = safe_join(&plan.workspace_root, &authority.source_document_path)?;
    let source = fs::read_to_string(&source_path).map_err(WorkspaceTransactionError::Io)?;
    let replacement_start =
        usize::try_from(summary.source_replacement_range.start).map_err(|_| {
            WorkspaceTransactionError::VerificationFailed(
                "task promotion replacement start exceeds source geometry".to_owned(),
            )
        })?;
    let replacement_end = replacement_start
        .checked_add(summary.replacement_source.len())
        .filter(|end| *end <= source.len())
        .ok_or_else(|| {
            WorkspaceTransactionError::VerificationFailed(
                "task promotion replacement exceeds committed source".to_owned(),
            )
        })?;
    if source.get(replacement_start..replacement_end) != Some(summary.replacement_source.as_str()) {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion committed source lacks its exact reviewed link replacement".to_owned(),
        ));
    }
    let source_parser = weftext_asciidoc::analyze(&source);
    let links = source_parser
        .links
        .iter()
        .filter(|link| {
            link.kind == weftext_asciidoc::LinkKind::Node
                && link.target == summary.generated_node_id.to_string()
                && link.display.as_deref() == Some(summary.replacement_link_label.as_str())
        })
        .count();
    if source_parser.status == weftext_asciidoc::AnalysisStatus::Failed
        || links != 1
        || source_parser.checklists.iter().any(|checklist| {
            summary.source_replacement_range.start <= checklist.item_range.start
                && checklist.item_range.start
                    < summary
                        .source_replacement_range
                        .start
                        .saturating_add(to_u64(summary.replacement_source.len()))
        })
    {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion source parser outcome is inconsistent".to_owned(),
        ));
    }
    let task_directory = safe_join(&plan.workspace_root, &authority.destination_node_path)?;
    let task = read_node_document(&task_directory).map_err(WorkspaceTransactionError::Document)?;
    let profile = crate::analyze_task_node_profile(&task.source, Some(summary.generated_node_id));
    if task.node_id != summary.generated_node_id
        || digest_bytes(task.source.as_bytes()) != authority.task_document_digest
        || !profile.diagnostics.is_empty()
        || profile.title.as_ref().map(|title| title.title.as_str())
            != Some(summary.generated_title.as_str())
        || profile.profile.as_ref().map(|profile| profile.state) != Some(summary.initial_state)
        || profile
            .profile
            .is_none_or(|profile| profile.closed.is_some() || !profile.depends_on.is_empty())
    {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion destination identity/profile differs from reviewed authority"
                .to_owned(),
        ));
    }
    let source_node_directory = source_path.parent().ok_or_else(|| {
        WorkspaceTransactionError::VerificationFailed(
            "task promotion source document has no node directory".to_owned(),
        )
    })?;
    let (source_sidecar_state, _) = observe_annotation_sidecar_at_authorized_node(
        source_node_directory,
        summary.source_node_id,
    )?;
    let expected_source_state = authority.source_sidecar_next_digest.as_ref().map_or_else(
        || authority.expected_source_sidecar.clone(),
        |sha256| TaskPromotionSidecarState::Present {
            sha256: sha256.clone(),
        },
    );
    if source_sidecar_state != expected_source_state {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion source annotation sidecar differs from reviewed authority".to_owned(),
        ));
    }
    let (task_sidecar_state, _) =
        observe_annotation_sidecar_at_authorized_node(&task_directory, summary.generated_node_id)?;
    let expected_task_state = authority.task_sidecar_digest.as_ref().map_or(
        TaskPromotionSidecarState::ConfirmedAbsent,
        |sha256| TaskPromotionSidecarState::Present {
            sha256: sha256.clone(),
        },
    );
    if task_sidecar_state != expected_task_state {
        return Err(WorkspaceTransactionError::VerificationFailed(
            "task promotion task annotation sidecar differs from reviewed authority".to_owned(),
        ));
    }
    Ok(())
}

fn write_journal(transaction: &Path, journal: &Journal) -> Result<(), WorkspaceTransactionError> {
    write_journal_with_limit(transaction, journal, MAX_JOURNAL_BYTES)
}

fn write_journal_with_limit(
    transaction: &Path,
    journal: &Journal,
    maximum_bytes: u64,
) -> Result<(), WorkspaceTransactionError> {
    let bytes = serialize_journal(journal)?;
    ensure_journal_wire_within_limit(&bytes, maximum_bytes)?;
    let mut staged = Builder::new()
        .prefix("journal-")
        .tempfile_in(transaction)
        .map_err(WorkspaceTransactionError::Io)?;
    staged
        .write_all(&bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    staged.flush().map_err(WorkspaceTransactionError::Io)?;
    staged
        .as_file()
        .sync_all()
        .map_err(WorkspaceTransactionError::Io)?;
    staged
        .persist(transaction.join("journal.json"))
        .map_err(|error| WorkspaceTransactionError::Io(error.error))?;
    sync_directory(transaction)
}

fn serialize_journal(journal: &Journal) -> Result<Vec<u8>, WorkspaceTransactionError> {
    match journal.schema.as_str() {
        JOURNAL_SCHEMA_V1 => serde_json::to_vec_pretty(&journal.v1_wire()),
        JOURNAL_SCHEMA_V2 => serde_json::to_vec_pretty(&journal.v2_wire()?),
        JOURNAL_SCHEMA_V3 => serde_json::to_vec_pretty(&journal.v3_wire()?),
        JOURNAL_SCHEMA_V4 => serde_json::to_vec_pretty(&journal.v4_wire()?),
        unsupported => {
            return Err(WorkspaceTransactionError::InvalidJournal(format!(
                "unsupported schema {unsupported}"
            )));
        }
    }
    .map_err(WorkspaceTransactionError::Json)
}

fn ensure_journal_wire_within_limit(
    bytes: &[u8],
    maximum_bytes: u64,
) -> Result<(), WorkspaceTransactionError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "journal exceeds {maximum_bytes} bytes"
        )));
    }
    Ok(())
}

fn validate_journal_lifecycle_wire_limit(
    prepared: &Journal,
    maximum_bytes: u64,
) -> Result<(), WorkspaceTransactionError> {
    let maximum_lifecycle_bytes = maximum_journal_lifecycle_wire_bytes(prepared)?;
    if maximum_lifecycle_bytes > maximum_bytes {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "journal lifecycle exceeds {maximum_bytes} bytes"
        )));
    }
    Ok(())
}

fn maximum_journal_lifecycle_wire_bytes(
    prepared: &Journal,
) -> Result<u64, WorkspaceTransactionError> {
    let prepared_bytes = u64::try_from(serialize_journal(prepared)?.len()).unwrap_or(u64::MAX);
    let mut applying = prepared.clone();
    mark_journal_applying(&mut applying)?;
    let applying_bytes = u64::try_from(serialize_journal(&applying)?.len()).unwrap_or(u64::MAX);
    let mut committed = applying;
    mark_journal_committed(&mut committed, prepared.base_revision.clone())?;
    let committed_bytes = u64::try_from(serialize_journal(&committed)?.len()).unwrap_or(u64::MAX);
    Ok(prepared_bytes.max(applying_bytes).max(committed_bytes))
}

fn external_receipt_claim(
    journal: &Journal,
    destination: &Path,
    bytes: &[u8],
) -> Result<ExternalReceiptClaim, WorkspaceTransactionError> {
    let destination = destination.to_str().ok_or_else(|| {
        WorkspaceTransactionError::ExternalReceipt("external receipt path is not UTF-8".to_owned())
    })?;
    let sha256 = digest_bytes(bytes);
    let byte_length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    let claim_digest =
        external_receipt_claim_digest(&journal.plan_id, destination, &sha256, byte_length)?;
    Ok(ExternalReceiptClaim {
        schema: EXTERNAL_RECEIPT_CLAIM_SCHEMA.to_owned(),
        plan_id: journal.plan_id.clone(),
        destination: destination.to_owned(),
        sha256,
        byte_length,
        claim_digest,
    })
}

fn external_receipt_claim_digest(
    plan_id: &str,
    destination: &str,
    sha256: &str,
    byte_length: u64,
) -> Result<String, WorkspaceTransactionError> {
    let material = serde_json::to_vec(&(
        EXTERNAL_RECEIPT_CLAIM_SCHEMA,
        plan_id,
        destination,
        sha256,
        byte_length,
    ))
    .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&material))
}

fn read_external_receipt_handoff(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
) -> Result<Option<WorkspaceTransactionReceiptHandoff>, WorkspaceTransactionError> {
    let payload_path = transaction.join(EXTERNAL_RECEIPT_PAYLOAD_FILE);
    let claim_path = transaction.join(EXTERNAL_RECEIPT_CLAIM_FILE);
    let payload_exists = non_link_path_exists(&payload_path)?;
    let claim_exists = non_link_path_exists(&claim_path)?;
    if !payload_exists && !claim_exists {
        return Ok(None);
    }
    if !payload_exists {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "external receipt claim exists without its staged payload".to_owned(),
        ));
    }
    let bytes = read_bounded_regular_file(&payload_path, MAX_EXTERNAL_RECEIPT_BYTES)?;
    let sha256 = digest_bytes(&bytes);
    if !claim_exists {
        return Ok(Some(WorkspaceTransactionReceiptHandoff {
            destination: None,
            sha256,
            bytes,
        }));
    }
    let claim_bytes = read_bounded_regular_file(&claim_path, MAX_EXTERNAL_RECEIPT_CLAIM_BYTES)?;
    reject_duplicate_json_keys(&claim_bytes).map_err(WorkspaceTransactionError::Json)?;
    let claim: ExternalReceiptClaim =
        serde_json::from_slice(&claim_bytes).map_err(WorkspaceTransactionError::Json)?;
    if claim.schema != EXTERNAL_RECEIPT_CLAIM_SCHEMA
        || claim.plan_id != journal.plan_id
        || claim.sha256 != sha256
        || claim.byte_length != u64::try_from(bytes.len()).unwrap_or(u64::MAX)
        || claim.claim_digest
            != external_receipt_claim_digest(
                &claim.plan_id,
                &claim.destination,
                &claim.sha256,
                claim.byte_length,
            )?
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "external receipt claim differs from its exact staged payload or transaction"
                .to_owned(),
        ));
    }
    let destination = canonical_external_receipt_destination(root, Path::new(&claim.destination))?;
    if destination.to_str() != Some(claim.destination.as_str()) {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "external receipt destination no longer resolves to its fixed canonical path"
                .to_owned(),
        ));
    }
    let intended_destination =
        validated_external_receipt_destination(root, journal)?.ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "external receipt claim belongs to a transaction without a receipt intent"
                    .to_owned(),
            )
        })?;
    if destination != intended_destination {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "external receipt claim differs from the pre-commit destination intent".to_owned(),
        ));
    }
    Ok(Some(WorkspaceTransactionReceiptHandoff {
        destination: Some(destination),
        sha256,
        bytes,
    }))
}

fn validated_external_receipt_destination(
    root: &Path,
    journal: &Journal,
) -> Result<Option<PathBuf>, WorkspaceTransactionError> {
    match (journal.finalization, &journal.external_receipt_destination) {
        (JournalFinalization::Core, None) => Ok(None),
        (JournalFinalization::Core, Some(_)) => Err(WorkspaceTransactionError::InvalidJournal(
            "Core-finalized journal carries an external receipt destination".to_owned(),
        )),
        (JournalFinalization::ExternalReceipt, None) => {
            Err(WorkspaceTransactionError::InvalidJournal(
                "external-receipt journal has no pre-commit destination intent".to_owned(),
            ))
        }
        (JournalFinalization::ExternalReceipt, Some(destination)) => {
            let canonical = canonical_external_receipt_destination(root, Path::new(destination))?;
            if canonical.to_str() != Some(destination.as_str()) {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "external receipt intent is not its fixed canonical path".to_owned(),
                ));
            }
            Ok(Some(canonical))
        }
    }
}

fn canonical_external_receipt_destination(
    root: &Path,
    path: &Path,
) -> Result<PathBuf, WorkspaceTransactionError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(WorkspaceTransactionError::Io)?
            .join(path)
    };
    reject_linked_existing_ancestors(&absolute)
        .map_err(|error| WorkspaceTransactionError::ExternalReceipt(error.to_string()))?;
    let parent = absolute.parent().ok_or_else(|| {
        WorkspaceTransactionError::ExternalReceipt(
            "external receipt destination has no parent".to_owned(),
        )
    })?;
    let name = absolute.file_name().ok_or_else(|| {
        WorkspaceTransactionError::ExternalReceipt(
            "external receipt destination has no filename".to_owned(),
        )
    })?;
    let canonical_root = fs::canonicalize(root).map_err(WorkspaceTransactionError::Io)?;
    let canonical_parent = fs::canonicalize(parent).map_err(WorkspaceTransactionError::Io)?;
    reject_linked_existing_ancestors(&canonical_parent)
        .map_err(|error| WorkspaceTransactionError::ExternalReceipt(error.to_string()))?;
    if canonical_parent.starts_with(&canonical_root) {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "external receipt destination must be outside the workspace".to_owned(),
        ));
    }
    let destination = canonical_parent.join(name);
    if destination.to_str().is_none() {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "external receipt destination is not UTF-8".to_owned(),
        ));
    }
    Ok(destination)
}

fn non_link_path_exists(path: &Path) -> Result<bool, WorkspaceTransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if linked_or_reparse(&metadata) => {
            Err(WorkspaceTransactionError::ExternalReceipt(format!(
                "receipt handoff path is a link or reparse point: {}",
                path.display()
            )))
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn read_bounded_regular_file(
    path: &Path,
    maximum_bytes: u64,
) -> Result<Vec<u8>, WorkspaceTransactionError> {
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ExternalReceipt(error.to_string()))?;
    let metadata = fs::symlink_metadata(path).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&metadata) || !metadata.is_file() {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff path is not a regular non-link file: {}",
            path.display()
        )));
    }
    if metadata.len() > maximum_bytes {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff exceeds {maximum_bytes} bytes"
        )));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(WorkspaceTransactionError::Io)?
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ExternalReceipt(error.to_string()))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff grew beyond {maximum_bytes} bytes"
        )));
    }
    Ok(bytes)
}

fn publish_exact_file(
    path: &Path,
    bytes: &[u8],
    maximum_bytes: u64,
) -> Result<(), WorkspaceTransactionError> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff exceeds {maximum_bytes} bytes"
        )));
    }
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ExternalReceipt(error.to_string()))?;
    if non_link_path_exists(path)? {
        let existing = read_bounded_regular_file(path, maximum_bytes)?;
        if existing == bytes {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(path)
                .and_then(|file| file.sync_all())
                .map_err(WorkspaceTransactionError::Io)?;
            let parent = path.parent().ok_or_else(|| {
                WorkspaceTransactionError::ExternalReceipt(
                    "receipt handoff destination has no parent".to_owned(),
                )
            })?;
            sync_directory(parent)?;
            return Ok(());
        }
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff destination already contains different bytes: {}",
            path.display()
        )));
    }
    let parent = path.parent().ok_or_else(|| {
        WorkspaceTransactionError::ExternalReceipt(
            "receipt handoff destination has no parent".to_owned(),
        )
    })?;
    let mut staged = Builder::new()
        .prefix("receipt-")
        .tempfile_in(parent)
        .map_err(WorkspaceTransactionError::Io)?;
    staged
        .write_all(bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    staged.flush().map_err(WorkspaceTransactionError::Io)?;
    staged
        .as_file()
        .sync_all()
        .map_err(WorkspaceTransactionError::Io)?;
    match fs::hard_link(staged.path(), path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(WorkspaceTransactionError::Io(error)),
    }
    drop(staged);
    sync_directory(parent)?;
    let existing = read_bounded_regular_file(path, maximum_bytes)?;
    if existing != bytes {
        return Err(WorkspaceTransactionError::ExternalReceipt(format!(
            "receipt handoff publication did not preserve exact bytes: {}",
            path.display()
        )));
    }
    Ok(())
}

fn verify_published_external_receipt(
    handoff: &WorkspaceTransactionReceiptHandoff,
) -> Result<(), WorkspaceTransactionError> {
    let destination = handoff.destination.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::ExternalReceipt(
            "external receipt has no fixed destination".to_owned(),
        )
    })?;
    let bytes = read_bounded_regular_file(destination, MAX_EXTERNAL_RECEIPT_BYTES)?;
    if bytes != handoff.bytes || digest_bytes(&bytes) != handoff.sha256 {
        return Err(WorkspaceTransactionError::ExternalReceipt(
            "published receipt differs from the staged exact bytes".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), WorkspaceTransactionError> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(WorkspaceTransactionError::Io)
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<(), WorkspaceTransactionError> {
    use std::os::windows::fs::OpenOptionsExt as _;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(WorkspaceTransactionError::Io)
}

#[derive(Clone, Copy)]
struct DuplicateKeyDetector;

impl<'de> de::DeserializeSeed<'de> for DuplicateKeyDetector {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for DuplicateKeyDetector {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key: {key}"
                )));
            }
            map.next_value_seed(Self)?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        while sequence.next_element_seed(Self)?.is_some() {}
        Ok(())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }
}

pub(crate) fn reject_duplicate_json_keys(bytes: &[u8]) -> Result<(), serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    DuplicateKeyDetector.deserialize(&mut deserializer)?;
    deserializer.end()
}

fn read_journal(transaction: &Path) -> Result<Journal, WorkspaceTransactionError> {
    reject_linked_transaction_tree(transaction)?;
    let journal_path = transaction.join("journal.json");
    reject_linked_existing_ancestors(&journal_path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let metadata = fs::symlink_metadata(&journal_path).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&metadata) || !metadata.is_file() {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal.json is not a regular non-link file".to_owned(),
        ));
    }
    if metadata.len() > MAX_JOURNAL_BYTES {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "journal exceeds {MAX_JOURNAL_BYTES} bytes"
        )));
    }
    let mut bytes = Vec::new();
    fs::File::open(&journal_path)
        .map_err(WorkspaceTransactionError::Io)?
        .take(MAX_JOURNAL_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_JOURNAL_BYTES {
        return Err(WorkspaceTransactionError::InvalidJournal(format!(
            "journal grew beyond {MAX_JOURNAL_BYTES} bytes"
        )));
    }
    reject_duplicate_json_keys(&bytes).map_err(WorkspaceTransactionError::Json)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(WorkspaceTransactionError::Json)?;
    let schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "journal schema is missing or invalid".to_owned(),
            )
        })?;
    let journal = match schema {
        JOURNAL_SCHEMA_V1 => {
            let wire: JournalV1Wire =
                serde_json::from_value(value).map_err(WorkspaceTransactionError::Json)?;
            if matches!(
                wire.action,
                StructuralAction::TaskPromotion | StructuralAction::TaskRebaseline
            ) {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v1 journal cannot carry TaskPromotion or TaskRebaseline".to_owned(),
                ));
            }
            Journal::from(wire)
        }
        JOURNAL_SCHEMA_V2 => {
            let wire: JournalV2Wire =
                serde_json::from_value(value).map_err(WorkspaceTransactionError::Json)?;
            if wire.action != StructuralAction::TaskPromotion {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v2 journal is reserved for TaskPromotion".to_owned(),
                ));
            }
            Journal::from(wire)
        }
        JOURNAL_SCHEMA_V3 => {
            let wire: JournalV3Wire =
                serde_json::from_value(value).map_err(WorkspaceTransactionError::Json)?;
            if wire.action != StructuralAction::TaskRebaseline {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v3 journal is reserved for TaskRebaseline".to_owned(),
                ));
            }
            Journal::from(wire)
        }
        JOURNAL_SCHEMA_V4 => {
            let wire: JournalV4Wire =
                serde_json::from_value(value).map_err(WorkspaceTransactionError::Json)?;
            if wire.action != StructuralAction::TaskRebaseline
                || wire.direction != TaskRebaselineJournalDirection::RollbackRebaseline
            {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v4 journal is reserved for TaskRebaseline exact rollback".to_owned(),
                ));
            }
            Journal::from(wire)
        }
        unsupported => {
            return Err(WorkspaceTransactionError::InvalidJournal(format!(
                "unsupported schema {unsupported}"
            )));
        }
    };
    if journal.steps.len() > MAX_JOURNAL_STEPS
        || journal.path_changes.len() > MAX_JOURNAL_PATH_CHANGES
        || journal.target_node_ids.len() > MAX_JOURNAL_PATH_CHANGES
        || journal.draft_sensitive_node_ids.len() > MAX_JOURNAL_PATH_CHANGES
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal step or path-change count exceeds the recovery bound".to_owned(),
        ));
    }
    Ok(journal)
}

fn reject_linked_transaction_tree(transaction: &Path) -> Result<(), WorkspaceTransactionError> {
    let metadata = fs::symlink_metadata(transaction).map_err(WorkspaceTransactionError::Io)?;
    if linked_or_reparse(&metadata) || !metadata.is_dir() {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "transaction authority is not a regular non-link directory".to_owned(),
        ));
    }
    let mut pending = vec![transaction.to_path_buf()];
    let mut entries = 0_usize;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory).map_err(WorkspaceTransactionError::Io)? {
            let entry = entry.map_err(WorkspaceTransactionError::Io)?;
            entries = entries.checked_add(1).ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "transaction entry count overflowed".to_owned(),
                )
            })?;
            if entries > MAX_TRANSACTION_ENTRIES {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "transaction entry count exceeds the recovery bound".to_owned(),
                ));
            }
            let metadata =
                fs::symlink_metadata(entry.path()).map_err(WorkspaceTransactionError::Io)?;
            if linked_or_reparse(&metadata) {
                return Err(WorkspaceTransactionError::InvalidJournal(format!(
                    "transaction authority contains a link or reparse point: {}",
                    entry.path().display()
                )));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            }
        }
    }
    Ok(())
}

fn validate_transaction_identity(
    transaction: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    let parsed = journal.plan_id.parse::<NodeId>().map_err(|_| {
        WorkspaceTransactionError::InvalidJournal("plan ID is not UUIDv4".to_owned())
    })?;
    if parsed.to_string() != journal.plan_id {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "plan ID is not canonical".to_owned(),
        ));
    }
    if transaction.file_name().and_then(|name| name.to_str())
        != Some(WORKSPACE_TRANSACTION_DIRECTORY)
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "transaction journal is outside the fixed workspace commit claim".to_owned(),
        ));
    }
    match journal.state {
        JournalState::Committed => {
            let revision = journal.committed_revision.as_ref().ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "committed journal has no committed revision".to_owned(),
                )
            })?;
            let expected =
                journal_commit_digest(&journal.schema, &journal.authority_digest, revision)?;
            if journal.commit_digest.as_deref() != Some(expected.as_str()) {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "committed journal marker digest is absent or invalid".to_owned(),
                ));
            }
        }
        JournalState::Prepared | JournalState::Applying => {
            if journal.committed_revision.is_some() || journal.commit_digest.is_some() {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "uncommitted journal carries committed marker evidence".to_owned(),
                ));
            }
        }
    }
    let expected_lifecycle_digest = journal_lifecycle_digest(
        &journal.schema,
        &journal.authority_digest,
        journal.state,
        journal.committed_revision.as_ref(),
    )?;
    if journal.lifecycle_digest != expected_lifecycle_digest {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal lifecycle marker differs from its exact state".to_owned(),
        ));
    }
    WorkspaceRevision::parse(journal.base_revision.as_str()).map_err(|error| {
        WorkspaceTransactionError::InvalidJournal(format!(
            "journal base revision is invalid: {error}"
        ))
    })?;
    if let Some(revision) = &journal.committed_revision {
        WorkspaceRevision::parse(revision.as_str()).map_err(|error| {
            WorkspaceTransactionError::InvalidJournal(format!(
                "journal committed revision is invalid: {error}"
            ))
        })?;
    }
    let expected_digest = journal_authority_digest(&JournalAuthorityDigestMaterial {
        schema: &journal.schema,
        plan_id: &journal.plan_id,
        base_revision: &journal.base_revision,
        action: journal.action,
        path_changes: &journal.path_changes,
        document_changes: &journal.document_changes,
        scope_summary: journal.scope_summary.as_ref(),
        promotion_summary: journal.promotion_summary.as_ref(),
        task_promotion_authority: journal.task_promotion_authority.as_ref(),
        task_rebaseline_authority: journal.task_rebaseline_authority.as_ref(),
        task_rebaseline_snapshot_authority: journal.task_rebaseline_snapshot_authority.as_ref(),
        task_rebaseline_commit_confirmation: journal.task_rebaseline_commit_confirmation.as_ref(),
        task_rebaseline_direction: journal.task_rebaseline_direction,
        task_rebaseline_rollback_authority: journal.task_rebaseline_rollback_authority.as_ref(),
        task_rebaseline_rollback_commit_confirmation: journal
            .task_rebaseline_rollback_commit_confirmation
            .as_ref(),
        identity_map: &journal.identity_map,
        captured_target: journal.captured_target.as_ref(),
        target_node_ids: &journal.target_node_ids,
        draft_sensitive_node_ids: &journal.draft_sensitive_node_ids,
        import_authority: journal.import_authority.as_ref(),
        annotation_sidecar_authority: journal.annotation_sidecar_authority.as_ref(),
        trash_item_changes: &journal.trash_item_changes,
        legacy_trash_migration_backup_authority: journal
            .legacy_trash_migration_backup_authority
            .as_ref(),
        finalization: journal.finalization,
        external_receipt_destination: journal.external_receipt_destination.as_deref(),
        steps: &journal.steps,
    })?;
    if journal.authority_digest != expected_digest {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal authority digest differs from its exact plan evidence".to_owned(),
        ));
    }
    validate_journal_action_and_path_changes(journal)
}

#[allow(clippy::too_many_lines)]
fn validate_journal_action_and_path_changes(
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    match journal.schema.as_str() {
        JOURNAL_SCHEMA_V1 => {
            if matches!(
                journal.action,
                StructuralAction::TaskPromotion | StructuralAction::TaskRebaseline
            ) || journal.promotion_summary.is_some()
                || journal.task_promotion_authority.is_some()
                || journal.task_rebaseline_authority.is_some()
                || journal.task_rebaseline_snapshot_authority.is_some()
                || journal.task_rebaseline_commit_confirmation.is_some()
                || journal.task_rebaseline_direction.is_some()
                || journal.task_rebaseline_rollback_authority.is_some()
                || journal
                    .task_rebaseline_rollback_commit_confirmation
                    .is_some()
                || !journal.document_changes.is_empty()
            {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v1 journal carries task promotion or rebaseline fields".to_owned(),
                ));
            }
        }
        JOURNAL_SCHEMA_V2 => validate_journal_promotion(journal)?,
        JOURNAL_SCHEMA_V3 => validate_journal_task_rebaseline(journal)?,
        JOURNAL_SCHEMA_V4 => validate_journal_task_rebaseline_rollback(journal)?,
        _ => {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal schema is unsupported".to_owned(),
            ));
        }
    }
    let mut expected_identity_map = if journal.action == StructuralAction::Copy {
        journal
            .path_changes
            .iter()
            .filter_map(|change| {
                change
                    .source_node_id
                    .map(|source_node_id| WorkspaceIdentityMapEntry {
                        source_node_id,
                        destination_node_id: change.node_id,
                    })
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    expected_identity_map.sort_by_key(|entry| entry.source_node_id);
    if journal.identity_map != expected_identity_map {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal identity map is incomplete or non-canonical".to_owned(),
        ));
    }
    let mut target_node_ids = journal.target_node_ids.clone();
    canonicalize_node_ids(&mut target_node_ids);
    let mut draft_sensitive_node_ids = journal.draft_sensitive_node_ids.clone();
    canonicalize_node_ids(&mut draft_sensitive_node_ids);
    if target_node_ids != journal.target_node_ids
        || draft_sensitive_node_ids != journal.draft_sensitive_node_ids
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal identity sets are not canonical".to_owned(),
        ));
    }
    match &journal.captured_target {
        Some(WorkspaceCapturedTarget::Node { node_id, .. }) => {
            if target_node_ids.binary_search(node_id).is_err() {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "journal captured node target is absent from target authority".to_owned(),
                ));
            }
        }
        Some(WorkspaceCapturedTarget::OwnedResource { owner_node_id, .. }) => {
            if target_node_ids.binary_search(owner_node_id).is_err() {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "journal captured resource owner is absent from target authority".to_owned(),
                ));
            }
        }
        Some(WorkspaceCapturedTarget::TrashItem { .. }) | None => {}
    }
    if let Some(summary) = &journal.scope_summary {
        validate_scope_summary(summary).map_err(|error| {
            WorkspaceTransactionError::InvalidJournal(format!(
                "journal scope summary is invalid: {error}"
            ))
        })?;
        let expected_policy = match journal.action {
            StructuralAction::Copy => WorkspaceIdentityPolicy::Rekey,
            StructuralAction::Rename
            | StructuralAction::Move
            | StructuralAction::Trash
            | StructuralAction::Restore => WorkspaceIdentityPolicy::Preserve,
            _ => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "journal action cannot carry a node-branch scope summary".to_owned(),
                ));
            }
        };
        if summary.identity_policy != expected_policy {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal action and identity policy disagree".to_owned(),
            ));
        }
    }
    if let Some(authority) = &journal.import_authority {
        validate_import_authority(authority)
            .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    }
    if matches!(
        journal.action,
        StructuralAction::Import | StructuralAction::SnapshotRestore
    ) != journal.import_authority.is_some()
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal action and import authority disagree".to_owned(),
        ));
    }
    if (journal.action == StructuralAction::Annotation)
        != journal.annotation_sidecar_authority.is_some()
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal action and annotation sidecar authority disagree".to_owned(),
        ));
    }
    if let Some(authority) = &journal.annotation_sidecar_authority {
        validate_journal_annotation_sidecar_authority(journal, authority)?;
    }
    let trash_action = matches!(
        journal.action,
        StructuralAction::Trash
            | StructuralAction::Restore
            | StructuralAction::PermanentDelete
            | StructuralAction::TrashMigration
    );
    if trash_action == journal.trash_item_changes.is_empty() {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal action and Trash item authority disagree".to_owned(),
        ));
    }
    match (
        journal.action,
        &journal.legacy_trash_migration_backup_authority,
    ) {
        (StructuralAction::TrashMigration, Some(authority)) => {
            authority.validate().map_err(|error| {
                WorkspaceTransactionError::InvalidJournal(format!(
                    "legacy Trash backup authority is invalid: {error}"
                ))
            })?;
            if authority.base_revision != journal.base_revision {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "legacy Trash backup revision differs from the journal".to_owned(),
                ));
            }
        }
        (StructuralAction::TrashMigration, None) | (_, Some(_)) => {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal action and legacy Trash backup authority disagree".to_owned(),
            ));
        }
        (_, None) => {}
    }
    let mut trash_item_ids = BTreeSet::new();
    if journal
        .trash_item_changes
        .iter()
        .any(|change| !trash_item_ids.insert(change.manifest.trash_item_id()))
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal Trash item authority contains duplicate item IDs".to_owned(),
        ));
    }
    let mut node_ids = BTreeSet::new();
    let mut destinations = BTreeSet::new();
    for change in &journal.path_changes {
        if !node_ids.insert(change.node_id) || !destinations.insert(change.new_path.to_lowercase())
        {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal path changes contain duplicate identities or destinations".to_owned(),
            ));
        }
        validate_workspace_journal_path(&change.new_path)?;
        if let Some(old_path) = &change.old_path {
            validate_workspace_journal_path(old_path)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn validate_journal_promotion(journal: &Journal) -> Result<(), WorkspaceTransactionError> {
    let summary = journal.promotion_summary.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion lacks promotionSummary".to_owned(),
        )
    })?;
    let authority = journal.task_promotion_authority.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion lacks private promotion authority".to_owned(),
        )
    })?;
    for (digest, label) in [
        (
            authority.task_document_digest.as_str(),
            "task document digest",
        ),
        (
            authority.task_payload_digest.as_str(),
            "task payload digest",
        ),
    ] {
        validate_import_digest(digest, label)
            .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    }
    if let TaskPromotionSidecarState::Present { sha256 } = &authority.expected_source_sidecar {
        validate_import_digest(sha256, "source sidecar base digest")
            .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    }
    for (digest, label) in [
        (
            authority.source_sidecar_next_digest.as_deref(),
            "source sidecar next digest",
        ),
        (
            authority.task_sidecar_digest.as_deref(),
            "task sidecar digest",
        ),
    ] {
        if let Some(digest) = digest {
            validate_import_digest(digest, label)
                .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
        }
    }
    let ([path_change], [document_change]) = (
        journal.path_changes.as_slice(),
        journal.document_changes.as_slice(),
    ) else {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion must contain one generated path and one source document change"
                .to_owned(),
        ));
    };
    let mut expected_targets = vec![summary.source_node_id, summary.generated_parent_node_id];
    canonicalize_node_ids(&mut expected_targets);
    let retained_dispositions = summary
        .annotations
        .dispositions
        .iter()
        .filter(|record| {
            record.disposition == crate::TaskPromotionAnnotationDisposition::RetainedInSource
        })
        .count();
    let migrated_dispositions = summary
        .annotations
        .dispositions
        .iter()
        .filter(|record| {
            record.disposition == crate::TaskPromotionAnnotationDisposition::MigratedToTaskNode
        })
        .count();
    let disposition_ids = summary
        .annotations
        .dispositions
        .iter()
        .map(|record| record.annotation_id)
        .collect::<Vec<_>>();
    let dispositions_canonical = disposition_ids.windows(2).all(|pair| pair[0] < pair[1]);
    if journal.action != StructuralAction::TaskPromotion
        || summary.workspace_revision != journal.base_revision
        || journal.scope_summary.is_some()
        || !journal.identity_map.is_empty()
        || journal.import_authority.is_some()
        || journal.annotation_sidecar_authority.is_some()
        || !journal.trash_item_changes.is_empty()
        || journal.legacy_trash_migration_backup_authority.is_some()
        || path_change.source_node_id.is_some()
        || path_change.node_id != summary.generated_node_id
        || path_change.old_path.is_some()
        || path_change.new_path != summary.generated_path
        || document_change.node_id != summary.source_node_id
        || document_change.base_revision != summary.source_revision
        || document_change.next_revision != summary.next_source_revision
        || document_change.path != authority.source_document_path
        || document_change.edit_count != 1
        || journal.target_node_ids != expected_targets
        || journal.draft_sensitive_node_ids != [summary.source_node_id]
        || !matches!(
            journal.captured_target,
            Some(WorkspaceCapturedTarget::Node { node_id, .. }) if node_id == summary.source_node_id
        )
        || summary.affected_document_node_ids.len() != 2
        || !summary
            .affected_document_node_ids
            .contains(&summary.source_node_id)
        || !summary
            .affected_document_node_ids
            .contains(&summary.generated_node_id)
        || !summary.annotations.replica_completeness.is_complete()
        || authority.source_node_id != summary.source_node_id
        || authority.generated_node_id != summary.generated_node_id
        || authority.parent_node_id != summary.generated_parent_node_id
        || Path::new(&summary.generated_path)
            .file_name()
            .and_then(|name| name.to_str())
            != Some(summary.generated_portable_name.as_str())
        || (authority.disclosure == TaskPromotionDisclosure::Owner
            && authority.destination_node_path != summary.generated_path)
        || authority.source_base_digest != summary.source_revision.as_str()
        || authority.source_next_digest != summary.next_source_revision.as_str()
        || !authority.annotation_replica_completeness.is_complete()
        || authority.annotation_replica_completeness != summary.annotations.replica_completeness
        || promotion_expected_sidecar_state(&authority.expected_source_sidecar)
            != summary.annotations.expected_source_sidecar
        || authority.task_document_path
            != format!(
                "{}/{}.adoc",
                authority.destination_node_path, summary.generated_portable_name
            )
        || authority.task_sidecar_path
            != format!(
                "{}/{}",
                authority.destination_node_path, ANNOTATIONS_FILE_NAME
            )
        || !dispositions_canonical
        || to_u64(retained_dispositions) != summary.annotations.retained_in_source_count
        || to_u64(migrated_dispositions) != summary.annotations.migrated_to_task_node_count
        || summary.annotations.task_sidecar_created != authority.task_sidecar_digest.is_some()
        || summary.annotations.task_sidecar_created
            != (summary.annotations.migrated_to_task_node_count != 0)
        || matches!(
            summary.annotations.expected_source_sidecar,
            AnnotationSidecarExpectedState::ConfirmedAbsent
        ) && (summary.annotations.retained_in_source_count != 0
            || summary.annotations.migrated_to_task_node_count != 0)
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion summary/action authority is inconsistent".to_owned(),
        ));
    }
    let [
        JournalStep::CreateTree {
            destination,
            staged: _,
            digest: task_payload_digest,
        },
        JournalStep::ReplaceFile {
            destination: source_document,
            staged: _,
            displaced: _,
            base_digest,
            next_digest,
        },
        tail @ ..,
    ] = journal.steps.as_slice()
    else {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion journal has an invalid step program".to_owned(),
        ));
    };
    let tail_matches = match tail {
        [] => authority.source_sidecar_next_digest.is_none(),
        [
            JournalStep::ReplaceFile {
                destination,
                base_digest,
                next_digest,
                ..
            },
        ] => {
            destination == &authority.source_sidecar_path
                && matches!(
                    &authority.expected_source_sidecar,
                    TaskPromotionSidecarState::Present { sha256 } if sha256 == base_digest
                )
                && authority.source_sidecar_next_digest.as_ref() == Some(next_digest)
        }
        _ => false,
    };
    if destination != &authority.destination_node_path
        || task_payload_digest != &authority.task_payload_digest
        || source_document == destination
        || source_document.starts_with(&format!("{destination}/"))
        || source_document != &authority.source_document_path
        || base_digest != summary.source_revision.as_str()
        || base_digest != &authority.source_base_digest
        || next_digest != summary.next_source_revision.as_str()
        || next_digest != &authority.source_next_digest
        || summary.annotations.source_sidecar_rewritten
            != authority.source_sidecar_next_digest.is_some()
        || summary.annotations.source_sidecar_rewritten == tail.is_empty()
        || !tail_matches
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v2 task promotion steps differ from the closed summary".to_owned(),
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn validate_journal_task_rebaseline(journal: &Journal) -> Result<(), WorkspaceTransactionError> {
    let authority = journal.task_rebaseline_authority.as_ref().ok_or_else(|| {
        WorkspaceTransactionError::InvalidJournal(
            "v3 task rebaseline lacks its closed authority".to_owned(),
        )
    })?;
    let snapshot = journal
        .task_rebaseline_snapshot_authority
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v3 task rebaseline lacks recoverable external snapshot authority".to_owned(),
            )
        })?;
    let confirmation = journal
        .task_rebaseline_commit_confirmation
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v3 task rebaseline lacks fresh Owner confirmation".to_owned(),
            )
        })?;
    crate::task_rebaseline_transaction::validate_summary(authority).map_err(|error| {
        WorkspaceTransactionError::InvalidJournal(format!(
            "v3 task rebaseline summary is invalid: {error}"
        ))
    })?;
    if journal.action != StructuralAction::TaskRebaseline
        || journal.base_revision != authority.base_workspace_revision
        || journal.scope_summary.is_some()
        || journal.promotion_summary.is_some()
        || journal.task_promotion_authority.is_some()
        || journal.task_rebaseline_direction
            != Some(TaskRebaselineJournalDirection::ApplyRebaseline)
        || journal.task_rebaseline_rollback_authority.is_some()
        || journal
            .task_rebaseline_rollback_commit_confirmation
            .is_some()
        || !journal.identity_map.is_empty()
        || journal.captured_target.is_some()
        || journal.import_authority.is_some()
        || journal.annotation_sidecar_authority.is_some()
        || !journal.trash_item_changes.is_empty()
        || journal.legacy_trash_migration_backup_authority.is_some()
        || journal.finalization != JournalFinalization::Core
        || journal.external_receipt_destination.is_some()
        || journal.target_node_ids != authority.draft_sensitive_node_ids
        || journal.draft_sensitive_node_ids != authority.draft_sensitive_node_ids
        || confirmation.confirmation_id == authority.owner_confirmation_id
        || confirmation.actor_binding != authority.owner_actor_binding
        || confirmation.authorization_epoch != authority.owner_authorization_epoch
        || snapshot.physical_inventory != authority.physical_pre_state
        || snapshot.root_identity != authority.external_snapshot.root_identity
        || snapshot.canonical_root.is_empty()
        || snapshot.canonical_root.len() > 32_768
        || !Path::new(&snapshot.canonical_root).is_absolute()
        || journal.path_changes.len() != authority.new_nodes.len()
        || journal.document_changes.len() != authority.source_replacements.len()
        || journal.steps.len()
            != authority
                .new_nodes
                .len()
                .saturating_add(authority.source_replacements.len())
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v3 task rebaseline action/Owner/snapshot authority is inconsistent".to_owned(),
        ));
    }
    let path_changes = journal
        .path_changes
        .iter()
        .map(|change| (change.node_id, change))
        .collect::<BTreeMap<_, _>>();
    for (index, node) in authority.new_nodes.iter().enumerate() {
        let change = path_changes.get(&node.generated_node_id).ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v3 task rebaseline omits one generated path".to_owned(),
            )
        })?;
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v3 task rebaseline document locator is invalid".to_owned(),
                )
            })?;
        let files = vec![TreeFile {
            path: document_file.to_owned(),
            bytes: node.exact_source.as_bytes().to_vec(),
        }];
        let expected_payload = payload_digest(&[], &files);
        match &journal.steps[index] {
            JournalStep::CreateTree {
                destination,
                staged,
                digest,
            } if destination == &node.destination_node_locator
                && staged == &format!("staged/{index}")
                && digest == &expected_payload
                && change.source_node_id.is_none()
                && change.old_path.is_none()
                && change.new_path == node.destination_node_locator => {}
            _ => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v3 task rebaseline generated-tree step is inconsistent".to_owned(),
                ));
            }
        }
    }
    let documents = journal
        .document_changes
        .iter()
        .map(|change| (change.path.as_str(), change))
        .collect::<BTreeMap<_, _>>();
    for (offset, replacement) in authority.source_replacements.iter().enumerate() {
        let index = authority.new_nodes.len() + offset;
        let document = documents
            .get(replacement.document_locator.as_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v3 task rebaseline omits one source replacement".to_owned(),
                )
            })?;
        match &journal.steps[index] {
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            } if destination == &replacement.document_locator
                && staged == &format!("staged/{index}.file")
                && displaced == &format!("displaced/{index}.file")
                && base_digest == replacement.base_revision.as_str()
                && next_digest == replacement.next_revision.as_str()
                && document.node_id == replacement.source_node_id
                && document.base_revision == replacement.base_revision
                && document.next_revision == replacement.next_revision => {}
            _ => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v3 task rebaseline source-replacement step is inconsistent".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the closed v4 rollback authority and reverse program are audited together"
)]
fn validate_journal_task_rebaseline_rollback(
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    let authority = journal
        .task_rebaseline_rollback_authority
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v4 task rebaseline rollback lacks its closed authority".to_owned(),
            )
        })?;
    let snapshot = journal
        .task_rebaseline_snapshot_authority
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v4 task rebaseline rollback lacks exact-A snapshot authority".to_owned(),
            )
        })?;
    let confirmation = journal
        .task_rebaseline_rollback_commit_confirmation
        .as_ref()
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal(
                "v4 task rebaseline rollback lacks fresh Owner confirmation".to_owned(),
            )
        })?;
    crate::task_rebaseline_transaction::validate_rollback_summary(authority).map_err(|error| {
        WorkspaceTransactionError::InvalidJournal(format!(
            "v4 task rebaseline rollback summary is invalid: {error}"
        ))
    })?;
    let forward = &authority.forward_authority;
    if journal.action != StructuralAction::TaskRebaseline
        || journal.task_rebaseline_direction
            != Some(TaskRebaselineJournalDirection::RollbackRebaseline)
        || journal.task_rebaseline_authority.is_some()
        || journal.task_rebaseline_commit_confirmation.is_some()
        || journal.base_revision != authority.base_workspace_revision
        || journal.scope_summary.is_some()
        || journal.promotion_summary.is_some()
        || journal.task_promotion_authority.is_some()
        || !journal.identity_map.is_empty()
        || journal.captured_target.is_some()
        || journal.import_authority.is_some()
        || journal.annotation_sidecar_authority.is_some()
        || !journal.trash_item_changes.is_empty()
        || journal.legacy_trash_migration_backup_authority.is_some()
        || journal.finalization != JournalFinalization::Core
        || journal.external_receipt_destination.is_some()
        || !journal.path_changes.is_empty()
        || journal.target_node_ids != authority.draft_sensitive_node_ids
        || journal.draft_sensitive_node_ids != authority.draft_sensitive_node_ids
        || confirmation.confirmation_id == authority.rollback_confirmation_id
        || confirmation.confirmation_id == forward.owner_confirmation_id
        || confirmation.confirmation_id
            == authority
                .forward_committed_evidence
                .forward_commit_confirmation_id
        || confirmation.actor_binding != authority.owner_actor_binding
        || confirmation.authorization_epoch != authority.owner_authorization_epoch
        || snapshot.physical_inventory != authority.external_snapshot.physical_inventory
        || snapshot.root_identity != authority.external_snapshot.root_identity
        || snapshot.canonical_root.is_empty()
        || snapshot.canonical_root.len() > 32_768
        || !Path::new(&snapshot.canonical_root).is_absolute()
        || journal.document_changes.len() != forward.source_replacements.len()
        || journal.steps.len()
            != forward
                .source_replacements
                .len()
                .saturating_add(forward.new_nodes.len())
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "v4 task rebaseline rollback action/Owner/snapshot authority is inconsistent"
                .to_owned(),
        ));
    }
    let documents = journal
        .document_changes
        .iter()
        .map(|change| (change.path.as_str(), change))
        .collect::<BTreeMap<_, _>>();
    for (index, replacement) in forward.source_replacements.iter().enumerate() {
        let document = documents
            .get(replacement.document_locator.as_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v4 task rebaseline rollback omits one source replacement".to_owned(),
                )
            })?;
        match &journal.steps[index] {
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                base_digest,
                next_digest,
            } if destination == &replacement.document_locator
                && staged == &format!("staged/{index}.file")
                && displaced == &format!("displaced/{index}.file")
                && base_digest == replacement.next_revision.as_str()
                && next_digest == replacement.base_revision.as_str()
                && document.node_id == replacement.source_node_id
                && document.base_revision == replacement.next_revision
                && document.next_revision == replacement.base_revision
                && document.edit_count == 1 => {}
            _ => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v4 task rebaseline rollback source step is inconsistent".to_owned(),
                ));
            }
        }
    }
    for (offset, node) in forward.new_nodes.iter().enumerate() {
        let index = forward.source_replacements.len() + offset;
        let document_file = Path::new(&node.document_locator)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                WorkspaceTransactionError::InvalidJournal(
                    "v4 task rebaseline rollback generated locator is invalid".to_owned(),
                )
            })?;
        let expected_digest = payload_digest(
            &[],
            &[TreeFile {
                path: document_file.to_owned(),
                bytes: node.exact_source.as_bytes().to_vec(),
            }],
        );
        match &journal.steps[index] {
            JournalStep::RemovePath {
                source,
                holding,
                digest,
            } if source == &node.destination_node_locator
                && holding == &format!("removed/{index}")
                && digest == &expected_digest => {}
            _ => {
                return Err(WorkspaceTransactionError::InvalidJournal(
                    "v4 task rebaseline rollback removal step is inconsistent".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_journal_annotation_sidecar_authority(
    journal: &Journal,
    authority: &AnnotationSidecarPlanAuthority,
) -> Result<(), WorkspaceTransactionError> {
    if authority.workspace_revision != journal.base_revision
        || !authority.completeness.is_complete()
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "annotation sidecar authority is not bound to a complete base revision".to_owned(),
        ));
    }
    validate_workspace_journal_path(&authority.destination)?;
    if Path::new(&authority.destination)
        .file_name()
        .and_then(|name| name.to_str())
        != Some(ANNOTATIONS_FILE_NAME)
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "annotation sidecar authority targets a non-sidecar path".to_owned(),
        ));
    }
    if let AnnotationSidecarExpectedState::Present { sha256 } = &authority.expected_state {
        validate_import_digest(sha256, "annotation sidecar digest")
            .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    }
    let mut matching =
        journal
            .steps
            .iter()
            .filter(|step| match (step, &authority.expected_state) {
                (
                    JournalStep::ReplaceFile {
                        destination,
                        base_digest,
                        ..
                    },
                    AnnotationSidecarExpectedState::Present { sha256 },
                ) => destination == &authority.destination && base_digest == sha256,
                (
                    JournalStep::CreateFile { destination, .. },
                    AnnotationSidecarExpectedState::ConfirmedAbsent,
                ) => destination == &authority.destination,
                _ => false,
            });
    if matching.next().is_none() || matching.next().is_some() {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "annotation sidecar authority does not match exactly one journal step".to_owned(),
        ));
    }
    Ok(())
}

fn validate_journal_paths(
    root: &Path,
    transaction: &Path,
    journal: &Journal,
) -> Result<(), WorkspaceTransactionError> {
    let mut keys = BTreeSet::new();
    for (index, step) in journal.steps.iter().enumerate() {
        let (key, workspace_paths, transaction_paths) = match step {
            JournalStep::CreateTree {
                destination,
                staged,
                ..
            } => (
                format!("create:{destination}"),
                vec![destination.as_str()],
                vec![(staged.as_str(), format!("staged/{index}"))],
            ),
            JournalStep::CreateFile {
                destination,
                staged,
                ..
            } => (
                format!("create-file:{destination}"),
                vec![destination.as_str()],
                vec![(staged.as_str(), format!("staged/{index}.file"))],
            ),
            JournalStep::MovePath {
                source,
                destination,
                holding,
                ..
            } => (
                format!("move:{source}:{destination}"),
                vec![source.as_str(), destination.as_str()],
                vec![(holding.as_str(), format!("holding/{index}"))],
            ),
            JournalStep::RemovePath {
                source, holding, ..
            } => (
                format!("remove:{source}"),
                vec![source.as_str()],
                vec![(holding.as_str(), format!("removed/{index}"))],
            ),
            JournalStep::ReplaceFile {
                destination,
                staged,
                displaced,
                ..
            } => (
                format!("replace:{destination}"),
                vec![destination.as_str()],
                vec![
                    (staged.as_str(), format!("staged/{index}.file")),
                    (displaced.as_str(), format!("displaced/{index}.file")),
                ],
            ),
        };
        if !keys.insert(key) {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal contains a duplicate step".to_owned(),
            ));
        }
        for relative in workspace_paths {
            validate_workspace_journal_path(relative)?;
            let _ = safe_join(root, relative)?;
        }
        for (relative, expected) in transaction_paths {
            if relative != expected {
                return Err(WorkspaceTransactionError::InvalidJournal(format!(
                    "journal staging path {relative} does not match expected {expected}"
                )));
            }
            let _ = safe_join(transaction, relative)?;
        }
    }
    Ok(())
}

fn validate_workspace_journal_path(relative: &str) -> Result<(), WorkspaceTransactionError> {
    let first = Path::new(relative)
        .components()
        .next()
        .and_then(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .ok_or_else(|| {
            WorkspaceTransactionError::InvalidJournal("invalid workspace path".to_owned())
        })?;
    let folded = first.to_ascii_lowercase();
    if folded == ".git" || folded.starts_with(&WORKSPACE_TRANSACTION_PREFIX.to_ascii_lowercase()) {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal path targets reserved workspace state".to_owned(),
        ));
    }
    Ok(())
}

fn transaction_path(root: &Path, plan_id: &str) -> Result<PathBuf, WorkspaceTransactionError> {
    let parsed = plan_id.parse::<NodeId>().map_err(|_| {
        WorkspaceTransactionError::InvalidJournal("plan ID is not UUIDv4".to_owned())
    })?;
    if parsed.to_string() != plan_id {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "plan ID is not canonical".to_owned(),
        ));
    }
    Ok(root.join(WORKSPACE_TRANSACTION_DIRECTORY))
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, WorkspaceTransactionError> {
    let path = Path::new(relative);
    if path.is_absolute() || relative.contains('\\') {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal path is not portable and relative".to_owned(),
        ));
    }
    let mut result = root.to_path_buf();
    let mut count = 0;
    for component in path.components() {
        let Component::Normal(value) = component else {
            return Err(WorkspaceTransactionError::InvalidJournal(
                "journal path contains a non-normal component".to_owned(),
            ));
        };
        result.push(value);
        count += 1;
    }
    if count == 0 {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "journal path is empty".to_owned(),
        ));
    }
    reject_linked_existing_ancestors(&result)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    Ok(result)
}

fn create_parent(path: &Path) -> Result<(), WorkspaceTransactionError> {
    let parent = path
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(path.to_path_buf()))?;
    durable_create_dir_all(parent)
}

fn durable_create_dir_all(path: &Path) -> Result<(), WorkspaceTransactionError> {
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    let mut missing = Vec::new();
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if linked_or_reparse(&metadata) || !metadata.is_dir() => {
                return Err(WorkspaceTransactionError::ContentBoundary(format!(
                    "directory path is linked or not a directory: {}",
                    ancestor.display()
                )));
            }
            Ok(_) => break,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(ancestor.to_path_buf());
            }
            Err(error) => return Err(WorkspaceTransactionError::Io(error)),
        }
    }
    fs::create_dir_all(path).map_err(WorkspaceTransactionError::Io)?;
    for directory in missing.iter().rev() {
        sync_directory(directory)?;
        if let Some(parent) = directory.parent() {
            sync_directory(parent)?;
        }
    }
    sync_directory(path)
}

fn durable_rename(source: &Path, destination: &Path) -> Result<(), WorkspaceTransactionError> {
    let source_parent = source
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(source.to_path_buf()))?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| WorkspaceTransactionError::PathEscape(destination.to_path_buf()))?;
    fs::rename(source, destination).map_err(WorkspaceTransactionError::Io)?;
    if destination_parent != source_parent {
        sync_directory(destination_parent)?;
    }
    sync_directory(source_parent)?;
    Ok(())
}

fn remove_transaction_directory(
    root: &Path,
    transaction: &Path,
) -> Result<(), WorkspaceTransactionError> {
    reject_linked_transaction_tree(transaction)?;
    let cleanup = root.join(format!(
        "{WORKSPACE_TRANSACTION_CLEANUP_PREFIX}{}",
        NodeId::new_v4()
    ));
    durable_rename(transaction, &cleanup)?;
    if fs::remove_dir_all(&cleanup).is_ok() {
        sync_directory(root)?;
    }
    Ok(())
}

fn rollback_marker_digest(
    plan_id: &str,
    prior_state: RolledBackPriorState,
    base_revision: &WorkspaceRevision,
    import_authority: Option<&WorkspaceImportAuthority>,
    journal_authority_digest: &str,
) -> Result<String, WorkspaceTransactionError> {
    let bytes = serde_json::to_vec(&(
        ROLLBACK_MARKER_SCHEMA,
        plan_id,
        prior_state,
        base_revision,
        import_authority,
        journal_authority_digest,
    ))
    .map_err(WorkspaceTransactionError::Json)?;
    Ok(digest_bytes(&bytes))
}

fn write_rolled_back_marker(
    root: &Path,
    journal: &Journal,
    prior_state: RolledBackPriorState,
) -> Result<(), WorkspaceTransactionError> {
    let marker = RolledBackMarker {
        schema: ROLLBACK_MARKER_SCHEMA.to_owned(),
        plan_id: journal.plan_id.clone(),
        prior_state,
        base_revision: journal.base_revision.clone(),
        import_authority: journal.import_authority.clone(),
        journal_authority_digest: journal.authority_digest.clone(),
        marker_digest: rollback_marker_digest(
            &journal.plan_id,
            prior_state,
            &journal.base_revision,
            journal.import_authority.as_ref(),
            &journal.authority_digest,
        )?,
    };
    let path = root.join(format!(
        "{WORKSPACE_TRANSACTION_ROLLBACK_PREFIX}{}.json",
        journal.plan_id
    ));
    let bytes = serde_json::to_vec_pretty(&marker).map_err(WorkspaceTransactionError::Json)?;
    if non_link_path_exists(&path)? {
        let existing = read_bounded_regular_file(&path, MAX_ROLLBACK_MARKER_BYTES)?;
        if existing == bytes {
            return Ok(());
        }
        return Err(WorkspaceTransactionError::InvalidJournal(
            "rollback terminal conflicts with the recovered transaction".to_owned(),
        ));
    }
    let mut staged = Builder::new()
        .prefix("rollback-marker-")
        .tempfile_in(root)
        .map_err(WorkspaceTransactionError::Io)?;
    staged
        .write_all(&bytes)
        .map_err(WorkspaceTransactionError::Io)?;
    staged.flush().map_err(WorkspaceTransactionError::Io)?;
    staged
        .as_file()
        .sync_all()
        .map_err(WorkspaceTransactionError::Io)?;
    staged
        .persist(&path)
        .map_err(|error| WorkspaceTransactionError::Io(error.error))?;
    sync_directory(root)
}

fn read_rolled_back_markers(
    root: &Path,
) -> Result<Vec<(PathBuf, RolledBackMarker)>, WorkspaceTransactionError> {
    let mut markers = Vec::new();
    for entry in fs::read_dir(root).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if !is_workspace_transaction_rollback_name(&name) {
            continue;
        }
        if !is_canonical_workspace_transaction_rollback_name(&name) {
            return Err(WorkspaceTransactionError::InvalidJournal(format!(
                "rollback terminal has a non-canonical name: {name}"
            )));
        }
        let bytes = read_bounded_regular_file(&entry.path(), MAX_ROLLBACK_MARKER_BYTES)?;
        reject_duplicate_json_keys(&bytes).map_err(WorkspaceTransactionError::Json)?;
        let marker: RolledBackMarker =
            serde_json::from_slice(&bytes).map_err(WorkspaceTransactionError::Json)?;
        validate_rolled_back_marker(&name, &marker)?;
        markers.push((entry.path(), marker));
    }
    markers.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(markers)
}

fn validate_rolled_back_marker(
    name: &str,
    marker: &RolledBackMarker,
) -> Result<(), WorkspaceTransactionError> {
    let expected_name = format!(
        "{WORKSPACE_TRANSACTION_ROLLBACK_PREFIX}{}.json",
        marker.plan_id
    );
    let plan_id = marker.plan_id.parse::<NodeId>().map_err(|_| {
        WorkspaceTransactionError::InvalidJournal("rollback plan ID is not UUIDv4".to_owned())
    })?;
    let expected_digest = rollback_marker_digest(
        &marker.plan_id,
        marker.prior_state,
        &marker.base_revision,
        marker.import_authority.as_ref(),
        &marker.journal_authority_digest,
    )?;
    if marker.schema != ROLLBACK_MARKER_SCHEMA
        || plan_id.to_string() != marker.plan_id
        || name != expected_name
        || marker.marker_digest != expected_digest
        || WorkspaceRevision::parse(marker.base_revision.as_str()).is_err()
    {
        return Err(WorkspaceTransactionError::InvalidJournal(
            "rollback terminal differs from its closed authority".to_owned(),
        ));
    }
    if let Some(authority) = &marker.import_authority {
        validate_import_authority(authority)
            .map_err(|error| WorkspaceTransactionError::InvalidJournal(error.to_string()))?;
    }
    Ok(())
}

fn cleanup_rolled_back_markers(root: &Path) -> Result<(), WorkspaceTransactionError> {
    let markers = read_rolled_back_markers(root)?;
    for (path, _) in &markers {
        fs::remove_file(path).map_err(WorkspaceTransactionError::Io)?;
    }
    if !markers.is_empty() {
        sync_directory(root)?;
    }
    Ok(())
}

fn cleanup_workspace_transaction_tombstones(root: &Path) -> Result<(), WorkspaceTransactionError> {
    validate_or_cleanup_workspace_transaction_tombstones(root, true)
}

fn validate_or_cleanup_workspace_transaction_tombstones(
    root: &Path,
    cleanup: bool,
) -> Result<(), WorkspaceTransactionError> {
    let mut removed = false;
    for entry in fs::read_dir(root).map_err(WorkspaceTransactionError::Io)? {
        let entry = entry.map_err(WorkspaceTransactionError::Io)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| WorkspaceTransactionError::NonUtf8Path(entry.path()))?;
        if !is_workspace_transaction_cleanup_name(&name) {
            continue;
        }
        if !is_canonical_workspace_transaction_cleanup_name(&name) {
            return Err(WorkspaceTransactionError::InvalidJournal(format!(
                "transaction cleanup tombstone has a non-canonical name: {name}"
            )));
        }
        reject_linked_transaction_tree(&entry.path())?;
        if cleanup {
            fs::remove_dir_all(entry.path()).map_err(WorkspaceTransactionError::Io)?;
            removed = true;
        }
    }
    if removed {
        sync_directory(root)?;
    }
    Ok(())
}

fn require_path_digest(path: &Path, expected: &str) -> Result<(), WorkspaceTransactionError> {
    let actual = path_digest(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(WorkspaceTransactionError::VerificationFailed(format!(
            "{} has digest {actual} instead of {expected}",
            path.display()
        )))
    }
}

fn path_digest(path: &Path) -> Result<String, WorkspaceTransactionError> {
    if path.is_file() {
        file_digest(path)
    } else if path.is_dir() {
        tree_digest(path)
    } else {
        Err(WorkspaceTransactionError::VerificationFailed(format!(
            "{} is unavailable",
            path.display()
        )))
    }
}

fn digest_if_exists(path: &Path) -> Result<Option<String>, WorkspaceTransactionError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if linked_or_reparse(&metadata) => Err(
            WorkspaceTransactionError::SymlinkUnsupported(path.to_path_buf()),
        ),
        Ok(_) => path_digest(path).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
    }
}

fn require_path_absent(path: &Path) -> Result<(), WorkspaceTransactionError> {
    reject_linked_existing_ancestors(path)
        .map_err(|error| WorkspaceTransactionError::ContentBoundary(error.to_string()))?;
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(WorkspaceTransactionError::Io(error)),
        Ok(_) => Err(WorkspaceTransactionError::DestinationExists(
            path.to_path_buf(),
        )),
    }
}

fn ambiguous_recovery(path: &Path) -> WorkspaceTransactionError {
    WorkspaceTransactionError::VerificationFailed(format!(
        "recovery evidence is ambiguous at {}",
        path.display()
    ))
}

fn require_workspace_revision(
    expected: &WorkspaceRevision,
    actual: &WorkspaceRevision,
) -> Result<(), WorkspaceTransactionError> {
    if expected == actual {
        Ok(())
    } else {
        Err(WorkspaceTransactionError::StaleRevision {
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}

#[derive(Debug)]
pub enum WorkspaceTransactionError {
    InvalidWorkspace,
    ContentBoundary(String),
    UnknownNode(NodeId),
    RootMutationUnsupported,
    TrashMutationUnsupported,
    NotInTrash(NodeId),
    UnknownTrashItem(crate::TrashItemId),
    LegacyTrashMigrationRequired,
    LegacyTrashMigrationBackupRequired,
    InvalidLegacyTrashMigrationBackup(String),
    TrashReconciliation(String),
    TrashRestoreUnavailable(crate::TrashRestoreBlockedReason),
    PermanentDeleteAuthorizationRequired,
    PermanentDeleteConfirmationMismatch,
    InvalidTrashReviewedRequest(String),
    DraftGateBlocked(Vec<NodeId>),
    DraftGateAuthorityMismatch,
    MoveIntoDescendant,
    DestinationExists(PathBuf),
    NoChange,
    AmbiguousAffectedLink {
        source: NodeId,
        start: u64,
    },
    IncompleteAnnotationReplica,
    AnnotationSidecarAuthorityMismatch,
    AnnotationSidecarChanged,
    AnnotationSidecarReconciliationRequired,
    StaleRevision {
        expected: WorkspaceRevision,
        actual: WorkspaceRevision,
    },
    RecoveryRequired(PathBuf),
    ExternalReceipt(String),
    InvalidJournal(String),
    PathEscape(PathBuf),
    NonUtf8Path(PathBuf),
    SymlinkUnsupported(PathBuf),
    InvalidUtf8(PathBuf),
    Metadata(String),
    VerificationFailed(String),
    Workspace(crate::WorkspaceError),
    Document(crate::DocumentError),
    LinkIndex(crate::LinkIndexError),
    Revision(crate::WorkspaceRevisionError),
    Io(std::io::Error),
    Json(serde_json::Error),
    InjectedFailure(usize),
    RecoveryRequiredWithCause {
        path: PathBuf,
        cause: String,
    },
}

impl fmt::Display for WorkspaceTransactionError {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => f.write_str("workspace inventory is invalid"),
            Self::ContentBoundary(message) => {
                write!(
                    f,
                    "workspace content boundary rejected the transaction: {message}"
                )
            }
            Self::UnknownNode(id) => write!(f, "workspace node is unavailable: {id}"),
            Self::RootMutationUnsupported => {
                f.write_str("workspace root cannot be structurally moved")
            }
            Self::TrashMutationUnsupported => {
                f.write_str("Workspace Trash cannot be used as an ordinary node")
            }
            Self::NotInTrash(id) => write!(f, "node is not in Workspace Trash: {id}"),
            Self::UnknownTrashItem(id) => write!(f, "Workspace Trash item is unavailable: {id}"),
            Self::LegacyTrashMigrationRequired => f.write_str(
                "legacy direct-entry Workspace Trash requires explicit migration before ordinary mutations",
            ),
            Self::LegacyTrashMigrationBackupRequired => f.write_str(
                "legacy Trash migration requires a Core-created verified external snapshot",
            ),
            Self::InvalidLegacyTrashMigrationBackup(message) => {
                write!(f, "invalid legacy Trash migration backup: {message}")
            }
            Self::TrashReconciliation(message) => {
                write!(f, "Workspace Trash requires explicit reconciliation: {message}")
            }
            Self::TrashRestoreUnavailable(reason) => {
                write!(f, "Workspace Trash restore mode is unavailable: {reason:?}")
            }
            Self::PermanentDeleteAuthorizationRequired => f.write_str(
                "permanent Trash deletion requires the higher-permission boundary",
            ),
            Self::PermanentDeleteConfirmationMismatch => f.write_str(
                "permanent Trash deletion confirmation differs from the exact reviewed items",
            ),
            Self::InvalidTrashReviewedRequest(message) => {
                write!(f, "invalid Trash reviewed request: {message}")
            }
            Self::DraftGateBlocked(node_ids) => write!(
                f,
                "workspace transaction is blocked by dirty drafts for node IDs: {}",
                node_ids
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::DraftGateAuthorityMismatch => {
                f.write_str("workspace draft-gate authority is invalid or belongs to another plan")
            }
            Self::MoveIntoDescendant => {
                f.write_str("a node cannot move or copy into its own subtree")
            }
            Self::DestinationExists(path) => write!(
                f,
                "workspace destination already exists: {}",
                path.display()
            ),
            Self::NoChange => f.write_str("structural action would not change the workspace"),
            Self::AmbiguousAffectedLink { source, start } => write!(
                f,
                "affected link is ambiguous in node {source} at byte {start}"
            ),
            Self::IncompleteAnnotationReplica => f.write_str(
                "annotation sidecar state requires a complete local or hosted replica authority",
            ),
            Self::AnnotationSidecarAuthorityMismatch => f.write_str(
                "annotation sidecar snapshot belongs to another workspace authority",
            ),
            Self::AnnotationSidecarChanged => f.write_str(
                "annotation sidecar changed after its complete-replica snapshot",
            ),
            Self::AnnotationSidecarReconciliationRequired => f.write_str(
                "annotation sidecar has duplicate identity, foreign ownership, or a sync conflict copy requiring explicit reconciliation",
            ),
            Self::StaleRevision { expected, actual } => write!(
                f,
                "stale workspace revision: expected {expected}, found {actual}"
            ),
            Self::RecoveryRequired(path) => write!(
                f,
                "workspace transaction recovery is required: {}",
                path.display()
            ),
            Self::RecoveryRequiredWithCause { path, cause } => write!(
                f,
                "workspace transaction recovery is required at {}: {cause}",
                path.display()
            ),
            Self::ExternalReceipt(message) => {
                write!(f, "invalid external workspace receipt handoff: {message}")
            }
            Self::InvalidJournal(message) => {
                write!(f, "invalid workspace transaction journal: {message}")
            }
            Self::PathEscape(path) => write!(
                f,
                "workspace transaction path escapes the root: {}",
                path.display()
            ),
            Self::NonUtf8Path(path) => write!(
                f,
                "workspace transaction path is not UTF-8: {}",
                path.display()
            ),
            Self::SymlinkUnsupported(path) => write!(
                f,
                "linked workspace transaction path is unsupported: {}",
                path.display()
            ),
            Self::InvalidUtf8(path) => write!(f, "node document is not UTF-8: {}", path.display()),
            Self::Metadata(message) => write!(f, "invalid node metadata: {message}"),
            Self::VerificationFailed(message) => {
                write!(f, "workspace transaction verification failed: {message}")
            }
            Self::Workspace(error) => error.fmt(f),
            Self::Document(error) => error.fmt(f),
            Self::LinkIndex(error) => error.fmt(f),
            Self::Revision(error) => error.fmt(f),
            Self::Io(error) => write!(f, "workspace transaction I/O failed: {error}"),
            Self::Json(error) => write!(f, "workspace transaction JSON failed: {error}"),
            Self::InjectedFailure(step) => write!(
                f,
                "injected workspace transaction failure after {step} steps"
            ),
        }
    }
}

impl std::error::Error for WorkspaceTransactionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_child_node, create_workspace};

    fn setup() -> (tempfile::TempDir, PathBuf) {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Notes");
        create_workspace(&workspace).unwrap();
        (temporary, workspace)
    }

    #[cfg(unix)]
    #[test]
    fn workspace_lease_resolves_linked_ancestor_but_rejects_linked_root() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        let real_parent = temporary.path().join("real");
        fs::create_dir(&real_parent).unwrap();
        let workspace = real_parent.join("Notes");
        create_workspace(&workspace).unwrap();

        let parent_alias = temporary.path().join("alias");
        symlink(&real_parent, &parent_alias).unwrap();
        let alias_workspace = parent_alias.join("Notes");
        let lease = acquire_workspace_transaction_lease(&alias_workspace).unwrap();
        assert_eq!(
            lease.physical_inventory_root(),
            fs::canonicalize(&workspace).unwrap()
        );
        drop(lease);

        let root_alias = temporary.path().join("linked-root");
        symlink(&workspace, &root_alias).unwrap();
        assert!(matches!(
            acquire_workspace_transaction_lease(&root_alias),
            Err(WorkspaceTransactionError::SymlinkUnsupported(path)) if path == root_alias
        ));
    }

    fn append_document(node: &Path, text: &str) {
        let name = node.file_name().unwrap().to_str().unwrap();
        let path = node.join(format!("{name}.adoc"));
        let mut source = fs::read_to_string(&path).unwrap();
        source.push_str(text);
        fs::write(path, source).unwrap();
    }

    fn local_annotation_snapshot(root: &Path, node_id: NodeId) -> AnnotationSidecarSnapshot {
        capture_annotation_sidecar_snapshot(
            root,
            node_id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace,
        )
        .unwrap()
    }

    fn read_local_annotations(root: &Path, node_id: NodeId) -> AnnotationStore {
        read_node_annotations(
            root,
            node_id,
            AnnotationReplicaCompleteness::CompleteLocalWorkspace,
        )
        .unwrap()
    }

    fn import_authority() -> WorkspaceImportAuthority {
        WorkspaceImportAuthority {
            proposal_id: "proposal-core-import-test".to_owned(),
            proposal_digest: digest_bytes(b"exact reviewed proposal"),
        }
    }

    fn imported_node(locator: &str) -> WorkspaceImportNode {
        let node_id = NodeId::from_str("11111111-1111-4111-8111-111111111111").unwrap();
        let name = locator.rsplit('/').next().unwrap();
        let exact_source =
            format!("---\nweftext:\n  id: \"{node_id}\"\n---\n= {name}\n\n精确导入内容 😀 مرحبا\n");
        let resource_bytes = b"exact-resource-bytes".to_vec();
        WorkspaceImportNode {
            locator: locator.to_owned(),
            node_id,
            document_file: format!("{name}.adoc"),
            document_sha256: digest_bytes(exact_source.as_bytes()),
            exact_source,
            resources: vec![WorkspaceImportResource {
                locator: "示意图.bin".to_owned(),
                sha256: digest_bytes(&resource_bytes),
                bytes: resource_bytes,
            }],
        }
    }

    fn snapshot_restore_tree(locator: &str) -> (Vec<WorkspaceRestoreTreeNode>, NodeId, NodeId) {
        let root_id = NodeId::new_v4();
        let child_id = NodeId::new_v4();
        let root_name = locator.rsplit('/').next().unwrap();
        let root_source = format!(
            "---\nweftext:\n  id: \"{root_id}\"\n---\n= Snapshot root\n\nExact restored source.\n"
        );
        let child_source = format!(
            "---\nweftext:\n  id: \"{child_id}\"\n---\n= Child\n\nNested restored source.\n"
        );
        let annotation_bytes = AnnotationStore::empty(root_id)
            .to_pretty_json()
            .unwrap()
            .into_bytes();
        let resource_bytes = b"snapshot-resource\0\xff".to_vec();
        (
            vec![
                WorkspaceRestoreTreeNode {
                    locator: locator.to_owned(),
                    node_id: root_id,
                    document_file: format!("{root_name}.adoc"),
                    document_sha256: digest_bytes(root_source.as_bytes()),
                    exact_source: root_source,
                    annotation_sidecar: Some(WorkspaceRestoreAnnotationSidecar {
                        sha256: digest_bytes(&annotation_bytes),
                        bytes: annotation_bytes,
                    }),
                    resources: vec![WorkspaceImportResource {
                        locator: "asset.bin".to_owned(),
                        sha256: digest_bytes(&resource_bytes),
                        bytes: resource_bytes,
                    }],
                },
                WorkspaceRestoreTreeNode {
                    locator: format!("{locator}/Child"),
                    node_id: child_id,
                    document_file: "Child.adoc".to_owned(),
                    document_sha256: digest_bytes(child_source.as_bytes()),
                    exact_source: child_source,
                    annotation_sidecar: None,
                    resources: Vec::new(),
                },
            ],
            root_id,
            child_id,
        )
    }

    fn imported_tree(locator: &str) -> (Vec<WorkspaceImportNode>, NodeId, NodeId) {
        let (nodes, root_id, child_id) = snapshot_restore_tree(locator);
        (
            nodes
                .into_iter()
                .map(|node| WorkspaceImportNode {
                    locator: node.locator,
                    node_id: node.node_id,
                    document_file: node.document_file,
                    exact_source: node.exact_source,
                    document_sha256: node.document_sha256,
                    resources: node.resources,
                })
                .collect(),
            root_id,
            child_id,
        )
    }

    #[test]
    fn clean_guard_cannot_commit_a_plan_for_another_workspace_root() {
        let temporary = tempfile::tempdir().unwrap();
        let first = temporary.path().join("First");
        let second = temporary.path().join("Second");
        let first_root = create_workspace(&first).unwrap();
        let second_root = create_workspace(&second).unwrap();
        let plan = plan_create_child_node(&second, second_root.id, "Child").unwrap();
        let guard = acquire_clean_workspace_mutation_guard(&first).unwrap();

        let error = commit_workspace_transaction_with_clean_guard(&plan, &guard).unwrap_err();
        assert!(matches!(error, WorkspaceTransactionError::Metadata(_)));
        assert_eq!(scan_workspace(&second).nodes.len(), 1);
        assert_eq!(scan_workspace(&first).nodes[0].id, Some(first_root.id));
    }

    #[test]
    fn public_action_spoof_cannot_mint_private_task_dependency_authority() {
        let (_temporary, workspace) = setup();
        let root = read_node_document(&workspace).unwrap();
        let before = root.source.clone();
        let mut metadata =
            plan_node_aliases_setting(&workspace, root.node_id, &root.revision, &["Alias".into()])
                .unwrap();
        metadata.action = StructuralAction::TaskDependencies;
        let registry = WorkspaceDraftRegistryView::empty_authority();

        assert!(matches!(
            preview_workspace_transaction_draft_gate(&metadata, &registry),
            Err(WorkspaceTransactionError::Metadata(_))
        ));
        assert!(matches!(
            commit_workspace_transaction(&metadata),
            Err(WorkspaceTransactionError::Metadata(_))
        ));
        assert_eq!(read_node_document(&workspace).unwrap().source, before);
    }

    #[test]
    fn node_alias_sort_and_rank_actions_commit_narrow_metadata_patches() {
        let (_temporary, workspace) = setup();
        let inventory = scan_workspace(&workspace);
        let root_id = inventory.nodes[0].id.unwrap();
        let root_document = workspace.join("Notes.adoc");
        let original = fs::read_to_string(&root_document).unwrap();
        let with_future = original.replace(
            &format!("  id: \"{root_id}\"\n"),
            &format!("  id: \"{root_id}\"\n  future:\n    opaque: [preserve, bytes]\n"),
        );
        fs::write(&root_document, with_future).unwrap();

        let root_snapshot = read_node_document(&workspace).unwrap();
        let aliases = vec!["文缕".to_owned(), "Weftext".to_owned()];
        let alias_plan =
            plan_node_aliases_setting(&workspace, root_id, &root_snapshot.revision, &aliases)
                .unwrap();
        assert_eq!(alias_plan.action, StructuralAction::NodeMetadata);
        assert_eq!(alias_plan.document_changes.len(), 1);
        assert_eq!(
            fs::read_to_string(&root_document).unwrap(),
            root_snapshot.source,
            "preview must not write"
        );
        commit_workspace_transaction(&alias_plan).unwrap();

        let root_snapshot = read_node_document(&workspace).unwrap();
        assert!(root_snapshot.source.contains("    - \"文缕\"\n"));
        assert!(
            root_snapshot
                .source
                .contains("  future:\n    opaque: [preserve, bytes]\n")
        );
        let sort_plan = plan_node_child_sort_setting(
            &workspace,
            root_id,
            &root_snapshot.revision,
            ChildSort {
                mode: crate::SortMode::Name,
                direction: crate::SortDirection::Descending,
            },
        )
        .unwrap();
        commit_workspace_transaction(&sort_plan).unwrap();
        let root_snapshot = read_node_document(&workspace).unwrap();
        assert!(root_snapshot.source.contains("  child_sort: name\n"));
        assert!(
            root_snapshot
                .source
                .contains("  child_sort_direction: descending\n")
        );
        assert!(matches!(
            plan_node_sibling_rank_setting(
                &workspace,
                root_id,
                &root_snapshot.revision,
                Some(1024)
            ),
            Err(WorkspaceTransactionError::RootMutationUnsupported)
        ));

        let child = create_child_node(&workspace, "Child").unwrap();
        let child_snapshot = read_node_document(workspace.join("Child")).unwrap();
        let rank_plan = plan_node_sibling_rank_setting(
            &workspace,
            child.id,
            &child_snapshot.revision,
            Some(2048),
        )
        .unwrap();
        commit_workspace_transaction(&rank_plan).unwrap();
        assert!(
            read_node_document(workspace.join("Child"))
                .unwrap()
                .source
                .contains("  sibling_rank: 2048\n")
        );

        let stale = plan_node_aliases_setting(
            &workspace,
            root_id,
            &DocumentRevision::from_source("stale base"),
            &["stale".to_owned()],
        )
        .expect_err("stale source revision");
        assert!(matches!(
            stale,
            WorkspaceTransactionError::Document(crate::DocumentError::StaleRevision { .. })
        ));
    }

    #[test]
    fn import_commits_exact_document_and_resources_through_one_core_transaction() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let authority = import_authority();
        let node = imported_node("导入");
        let expected_source = node.exact_source.clone();
        let expected_resource = node.resources[0].bytes.clone();

        let plan = plan_import_node(&workspace, &base_revision, authority.clone(), node).unwrap();
        assert_eq!(plan.action, StructuralAction::Import);
        assert_eq!(plan.import_authority.as_ref(), Some(&authority));
        assert!(!workspace.join("导入").exists());

        let committed = commit_workspace_transaction(&plan).unwrap();
        assert_eq!(committed.action, StructuralAction::Import);
        assert_eq!(committed.import_authority.as_ref(), Some(&authority));
        assert_eq!(
            fs::read_to_string(workspace.join("导入/导入.adoc")).unwrap(),
            expected_source
        );
        assert_eq!(
            fs::read(workspace.join("导入/示意图.bin")).unwrap(),
            expected_resource
        );
        assert_ne!(committed.revision, base_revision);
        assert!(scan_workspace(&workspace).is_valid());
    }

    #[test]
    fn import_refuses_stale_or_noncanonical_proposal_authority() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let plan = plan_import_node(
            &workspace,
            &base_revision,
            import_authority(),
            imported_node("Imported"),
        )
        .unwrap();
        create_child_node(&workspace, "Concurrent").unwrap();
        assert!(matches!(
            commit_workspace_transaction(&plan),
            Err(WorkspaceTransactionError::StaleRevision { .. })
        ));
        assert!(!workspace.join("Imported").exists());

        let latest = read_workspace_revision(&workspace).unwrap();
        let mut legacy = imported_node("Legacy");
        legacy.exact_source = format!(
            "---\nweftext:\n  id: \"{}\"\nreference:\n  key: retired\n---\n= Legacy\n",
            legacy.node_id
        );
        legacy.document_sha256 = digest_bytes(legacy.exact_source.as_bytes());
        assert!(matches!(
            plan_import_node(&workspace, &latest, import_authority(), legacy),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let mut reserved = imported_node("Reserved");
        reserved.resources[0].locator = ANNOTATIONS_FILE_NAME.to_owned();
        assert!(matches!(
            plan_import_node(&workspace, &latest, import_authority(), reserved),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let mut markdown_peer = imported_node("MarkdownPeer");
        markdown_peer.resources[0].locator = "legacy.MD".to_owned();
        assert!(matches!(
            plan_import_node(&workspace, &latest, import_authority(), markdown_peer),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let mut case_collision = imported_node("CaseCollision");
        let duplicate_bytes = case_collision.resources[0].bytes.clone();
        case_collision.resources.push(WorkspaceImportResource {
            locator: "示意图.BIN".to_owned(),
            sha256: digest_bytes(&duplicate_bytes),
            bytes: duplicate_bytes,
        });
        assert!(matches!(
            plan_import_node(&workspace, &latest, import_authority(), case_collision),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let mut escaping = imported_node("Escape");
        escaping.locator = "../Escape".to_owned();
        assert!(plan_import_node(&workspace, &latest, import_authority(), escaping).is_err());
    }

    #[test]
    fn node_planners_reject_reserved_portable_names() {
        let (_temporary, workspace) = setup();
        let inventory = scan_workspace(&workspace);
        let root_id = inventory
            .nodes
            .iter()
            .find(|node| node.parent_id.is_none())
            .and_then(|node| node.id)
            .unwrap();

        for reserved in [".weftext-format", "_weftext.items"] {
            assert!(matches!(
                plan_create_child_node(&workspace, root_id, reserved),
                Err(WorkspaceTransactionError::Workspace(
                    crate::WorkspaceError::InvalidName(_)
                ))
            ));
        }

        let child = create_child_node(&workspace, "Child").unwrap();
        assert!(matches!(
            plan_rename_node(&workspace, child.id, "weftext.annotations.json"),
            Err(WorkspaceTransactionError::Workspace(
                crate::WorkspaceError::InvalidName(_)
            ))
        ));

        let revision = read_workspace_revision(&workspace).unwrap();
        assert!(matches!(
            plan_import_node(
                &workspace,
                &revision,
                import_authority(),
                imported_node(".__weftext-transaction-import")
            ),
            Err(WorkspaceTransactionError::Workspace(
                crate::WorkspaceError::InvalidName(_)
            ))
        ));

        let (snapshot, _, _) = snapshot_restore_tree("_weftext.items");
        assert!(matches!(
            plan_restore_snapshot_tree(&workspace, &revision, import_authority(), snapshot),
            Err(WorkspaceTransactionError::Workspace(
                crate::WorkspaceError::InvalidName(_)
            ))
        ));
    }

    #[test]
    fn every_import_step_boundary_recovers_exactly() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=1 {
            let plan = plan_import_node(
                &workspace,
                &base_revision,
                import_authority(),
                imported_node("Recoverable"),
            )
            .unwrap();
            assert_eq!(plan.steps.len(), 1);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();

            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary)
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            let report = recover_workspace_transactions(&workspace).unwrap();
            assert_eq!(report.applying_rolled_back, 1);
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
            assert!(!workspace.join("Recoverable").exists());
        }
    }

    #[test]
    fn plan_bound_recovery_retains_a_foreign_prepared_journal() {
        let (_temporary, workspace) = setup();
        let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
        let expected = plan_create_child_node(&workspace, root_id, "Expected").unwrap();
        let foreign = plan_create_child_node(&workspace, root_id, "Foreign").unwrap();
        let transaction = prepare_workspace_transaction_recovery_fixture(&expected).unwrap();

        assert!(matches!(
            recover_workspace_transaction_for_plan(&foreign),
            Err(WorkspaceTransactionError::RecoveryRequired(path)) if path == transaction
        ));
        assert!(transaction.is_dir());
        assert!(!workspace.join("Expected").exists());
        assert!(!workspace.join("Foreign").exists());

        let report = recover_workspace_transaction_for_plan(&expected).unwrap();
        assert_eq!(report.prepared_removed, 1);
        assert_eq!(report.committed_cleaned, 0);
        assert!(report.committed_transactions.is_empty());
        assert!(!transaction.exists());
    }

    #[test]
    fn plan_bound_recovery_returns_and_cleans_the_exact_committed_outcome() {
        let (_temporary, workspace) = setup();
        let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
        let plan = plan_create_child_node(&workspace, root_id, "Committed").unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        write_journal(&transaction, &journal).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        apply_journal_steps(&workspace, &transaction, &journal.steps, None).unwrap();
        verify_plan_outcome(&plan).unwrap();
        mark_journal_committed(&mut journal, read_workspace_revision(&workspace).unwrap()).unwrap();
        write_journal(&transaction, &journal).unwrap();
        let expected = committed_transaction_from_journal(&journal).unwrap();

        let report = recover_workspace_transaction_for_plan(&plan).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        assert_eq!(report.prepared_removed, 0);
        assert_eq!(report.applying_rolled_back, 0);
        assert_eq!(report.committed_transactions, vec![expected]);
        assert!(!transaction.exists());
        assert!(workspace.join("Committed/Committed.adoc").is_file());
    }

    #[test]
    fn multi_node_import_tree_preserves_reviewed_ids_and_recovers_or_replays_atomically() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=1 {
            let (nodes, _, _) = imported_tree("RecoverableImportTree");
            let plan =
                plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
            assert_eq!(plan.action, StructuralAction::Import);
            assert_eq!(plan.steps.len(), 1);
            assert_eq!(plan.generated_node_ids.len(), 2);
            assert!(
                plan.path_changes
                    .iter()
                    .all(|change| change.source_node_id.is_none())
            );
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();
            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary)
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert_eq!(
                recover_workspace_transactions(&workspace)
                    .unwrap()
                    .applying_rolled_back,
                1
            );
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
            assert!(!workspace.join("RecoverableImportTree").exists());
        }

        let (nodes, root_id, child_id) = imported_tree("CommittedImportTree");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        write_journal(&transaction, &journal).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        apply_journal_steps(&workspace, &transaction, &journal.steps, None).unwrap();
        verify_plan_outcome(&plan).unwrap();
        mark_journal_committed(&mut journal, read_workspace_revision(&workspace).unwrap()).unwrap();
        write_journal(&transaction, &journal).unwrap();

        let report = recover_workspace_transactions(&workspace).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        let inventory = scan_workspace(&workspace);
        assert!(inventory.is_valid());
        assert!(inventory.nodes.iter().any(|node| node.id == Some(root_id)));
        assert!(inventory.nodes.iter().any(|node| node.id == Some(child_id)));
        assert_eq!(
            fs::read(workspace.join("CommittedImportTree/asset.bin")).unwrap(),
            b"snapshot-resource\0\xff"
        );
    }

    #[test]
    fn external_receipt_journal_survives_generic_recovery_until_exact_finalization() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("ReceiptHandoff");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let receipt_path = temporary.path().join("receipt.json");
        let committed =
            commit_workspace_transaction_retaining_journal(&plan, &receipt_path).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        assert!(transaction.is_dir());

        let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
        assert!(matches!(
            plan_create_child_node(&workspace, root_id, "BlockedWhileUnreceipted"),
            Err(WorkspaceTransactionError::RecoveryRequired(path)) if path == transaction
        ));

        let report = recover_workspace_transactions(&workspace).unwrap();
        assert_eq!(report.committed_cleaned, 0);
        assert_eq!(report.committed_retained, 1);
        assert_eq!(report.committed_transactions, vec![committed.clone()]);
        assert!(transaction.is_dir());

        let mut forged = committed.clone();
        forged.path_changes[0].new_path.push_str("-forged");
        assert!(matches!(
            finalize_committed_workspace_transaction(&workspace, &forged),
            Err(WorkspaceTransactionError::InvalidJournal(_))
        ));
        assert!(transaction.is_dir());

        let receipt_bytes = br#"{"receipt":"exact"}"#;
        publish_committed_workspace_transaction_receipt(
            &workspace,
            &committed,
            &receipt_path,
            receipt_bytes,
        )
        .unwrap();
        fs::write(&receipt_path, b"altered").unwrap();
        assert!(matches!(
            finalize_committed_workspace_transaction(&workspace, &committed),
            Err(WorkspaceTransactionError::ExternalReceipt(_))
        ));
        assert!(transaction.is_dir());
        fs::write(&receipt_path, receipt_bytes).unwrap();
        finalize_committed_workspace_transaction(&workspace, &committed).unwrap();
        assert!(!transaction.exists());
        plan_create_child_node(&workspace, root_id, "AllowedAfterReceipt").unwrap();
    }

    #[test]
    fn retained_commit_binds_receipt_destination_before_workspace_mutation() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("FixedReceiptTarget");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let intended = temporary.path().join("intended-receipt.json");
        let wrong = temporary.path().join("wrong-receipt.json");
        let committed = commit_workspace_transaction_retaining_journal(&plan, &intended).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();

        assert!(matches!(
            publish_committed_workspace_transaction_receipt(
                &workspace,
                &committed,
                &wrong,
                b"exact receipt"
            ),
            Err(WorkspaceTransactionError::ExternalReceipt(message))
                if message.contains("pre-commit intent")
        ));
        assert!(!transaction.join(EXTERNAL_RECEIPT_PAYLOAD_FILE).exists());
        assert!(!transaction.join(EXTERNAL_RECEIPT_CLAIM_FILE).exists());
        assert!(!wrong.exists());

        publish_committed_workspace_transaction_receipt(
            &workspace,
            &committed,
            &intended,
            b"exact receipt",
        )
        .unwrap();
        finalize_committed_workspace_transaction(&workspace, &committed).unwrap();
        assert_eq!(fs::read(intended).unwrap(), b"exact receipt");
    }

    #[test]
    fn existing_receipt_target_rejects_retained_commit_without_workspace_write() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("ExistingReceiptTarget");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let receipt = temporary.path().join("existing.json");
        fs::write(&receipt, b"foreign").unwrap();

        assert!(matches!(
            commit_workspace_transaction_retaining_journal(&plan, &receipt),
            Err(WorkspaceTransactionError::ExternalReceipt(message))
                if message.contains("already exists")
        ));
        assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
        assert!(!workspace.join("ExistingReceiptTarget").exists());
        assert!(
            !transaction_path(&workspace, &plan.plan_id)
                .unwrap()
                .exists()
        );
    }

    #[test]
    fn duplicate_journal_key_fails_closed_without_cleanup() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("DuplicateJournalKey");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        commit_workspace_transaction_retaining_journal(
            &plan,
            temporary.path().join("duplicate-key-receipt.json"),
        )
        .unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        let journal_path = transaction.join("journal.json");
        let source = fs::read_to_string(&journal_path).unwrap();
        let altered = source.replacen(
            "\"state\": \"committed\",",
            "\"state\": \"committed\",\n  \"state\": \"prepared\",",
            1,
        );
        fs::write(&journal_path, altered).unwrap();

        assert!(matches!(
            recover_workspace_transactions_retaining_committed(&workspace),
            Err(WorkspaceTransactionError::Json(error))
                if error.to_string().contains("duplicate JSON object key")
        ));
        assert!(transaction.is_dir());
    }

    #[test]
    fn lifecycle_and_committed_outcome_tampering_never_mutate_evidence() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        let (nodes, _, _) = imported_tree("LifecycleTamper");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        journal.state = JournalState::Prepared;
        write_journal(&transaction, &journal).unwrap();
        assert!(matches!(
            recover_workspace_transactions(&workspace),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("lifecycle")
        ));
        assert!(transaction.is_dir());
        fs::remove_dir_all(&transaction).unwrap();

        let (nodes, _, _) = imported_tree("ForgedCommittedOutcome");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        mark_journal_committed(&mut journal, base_revision).unwrap();
        write_journal(&transaction, &journal).unwrap();
        assert!(matches!(
            recover_workspace_transactions(&workspace),
            Err(WorkspaceTransactionError::VerificationFailed(_))
        ));
        assert!(transaction.is_dir());
        assert!(!workspace.join("ForgedCommittedOutcome").exists());
    }

    #[test]
    fn rolled_back_terminal_makes_recovery_durably_idempotent() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("DurableRollback");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        assert!(matches!(
            apply_journal_steps(&workspace, &transaction, &journal.steps, Some(0)),
            Err(WorkspaceTransactionError::InjectedFailure(0))
        ));

        let first = recover_workspace_transactions(&workspace).unwrap();
        assert_eq!(first.applying_rolled_back, 1);
        assert!(!transaction.exists());
        assert!(!has_unfinished_workspace_transaction(&workspace).unwrap());
        let second = recover_workspace_transactions(&workspace).unwrap();
        assert_eq!(second.applying_rolled_back, 1);
        assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
        assert!(!workspace.join("DurableRollback").exists());
    }

    #[test]
    fn transaction_name_scanning_is_casefolded_and_cleanup_names_are_closed() {
        let (_temporary, workspace) = setup();
        let uppercase = workspace.join(WORKSPACE_TRANSACTION_DIRECTORY.to_ascii_uppercase());
        fs::create_dir(&uppercase).unwrap();
        assert!(matches!(
            ensure_no_unfinished_transaction(&workspace),
            Err(WorkspaceTransactionError::RecoveryRequired(path)) if path == uppercase
        ));
        fs::remove_dir(&uppercase).unwrap();

        let malformed_cleanup = workspace.join(format!(
            "{}NOT-A-UUID",
            WORKSPACE_TRANSACTION_CLEANUP_PREFIX.to_ascii_uppercase()
        ));
        fs::create_dir(&malformed_cleanup).unwrap();
        assert!(matches!(
            recover_workspace_transactions(&workspace),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("non-canonical name")
        ));
        assert!(malformed_cleanup.is_dir());
        fs::remove_dir(&malformed_cleanup).unwrap();

        let cleanup = workspace.join(format!(
            "{WORKSPACE_TRANSACTION_CLEANUP_PREFIX}{}",
            NodeId::new_v4()
        ));
        fs::create_dir(&cleanup).unwrap();
        fs::write(cleanup.join("sensitive-staged-bytes"), b"staged").unwrap();
        assert_eq!(
            inspect_workspace_import_transaction(&workspace, &import_authority()).unwrap(),
            WorkspaceImportTransactionState::Absent
        );
        assert!(
            cleanup.is_dir(),
            "read-only inspection must not clean tombstones"
        );
        assert_eq!(
            recover_workspace_transactions(&workspace).unwrap(),
            RecoveryReport::default()
        );
        assert!(!cleanup.exists());
    }

    #[test]
    fn journal_authority_digest_rejects_tampered_committed_handoff_evidence() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("TamperEvidence");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        let committed = commit_workspace_transaction_retaining_journal(
            &plan,
            temporary.path().join("tampered-receipt.json"),
        )
        .unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        let mut journal = read_journal(&transaction).unwrap();
        journal.path_changes[0].new_path.push_str("-forged");
        write_journal(&transaction, &journal).unwrap();

        assert!(matches!(
            recover_workspace_transactions_retaining_committed(&workspace),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("authority digest")
        ));
        assert!(transaction.is_dir());
        assert!(matches!(
            finalize_committed_workspace_transaction(&workspace, &committed),
            Err(WorkspaceTransactionError::InvalidJournal(message))
                if message.contains("authority digest")
        ));
        assert!(transaction.is_dir());
    }

    #[test]
    fn committed_journal_rejects_unknown_nested_authority_fields_without_cleanup() {
        let (temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, _, _) = imported_tree("ClosedJournal");
        let plan = plan_import_tree(&workspace, &base_revision, import_authority(), nodes).unwrap();
        commit_workspace_transaction_retaining_journal(
            &plan,
            temporary.path().join("closed-receipt.json"),
        )
        .unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        let journal_path = transaction.join("journal.json");
        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&journal_path).unwrap()).unwrap();
        value["path_changes"][0]["unreviewed"] = serde_json::json!(true);
        fs::write(&journal_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();

        assert!(matches!(
            recover_workspace_transactions_retaining_committed(&workspace),
            Err(WorkspaceTransactionError::Json(_))
        ));
        assert!(transaction.is_dir());
    }

    #[test]
    fn snapshot_restore_tree_preserves_all_ids_sidecars_and_resource_bytes_atomically() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, root_id, child_id) = snapshot_restore_tree("Recovered");
        let expected_sidecar = nodes[0].annotation_sidecar.as_ref().unwrap().bytes.clone();
        let expected_resource = nodes[0].resources[0].bytes.clone();
        let plan =
            plan_restore_snapshot_tree(&workspace, &base_revision, import_authority(), nodes)
                .unwrap();
        assert_eq!(plan.action, StructuralAction::SnapshotRestore);
        assert_eq!(plan.steps.len(), 1);
        assert_eq!(plan.path_changes.len(), 2);
        assert!(plan.generated_node_ids.is_empty());
        assert!(
            !workspace.join("Recovered").exists(),
            "preview is read-only"
        );

        let committed = commit_workspace_transaction(&plan).unwrap();
        assert_eq!(committed.action, StructuralAction::SnapshotRestore);
        assert_eq!(committed.import_authority, Some(import_authority()));
        assert_eq!(
            fs::read(workspace.join("Recovered/weftext.annotations.json")).unwrap(),
            expected_sidecar
        );
        assert_eq!(
            fs::read(workspace.join("Recovered/asset.bin")).unwrap(),
            expected_resource
        );
        let inventory = scan_workspace(&workspace);
        assert!(inventory.is_valid());
        assert!(
            inventory.nodes.iter().any(|node| {
                node.id == Some(root_id) && node.path == workspace.join("Recovered")
            })
        );
        assert!(inventory.nodes.iter().any(|node| {
            node.id == Some(child_id) && node.path == workspace.join("Recovered/Child")
        }));
        assert_eq!(
            read_local_annotations(&workspace, root_id),
            AnnotationStore::empty(root_id)
        );
    }

    #[test]
    fn snapshot_restore_tree_rejects_invalid_topology_sidecar_identity_conflicts_and_stale_state() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        let (mut disconnected, _, _) = snapshot_restore_tree("Recovered");
        disconnected[1].locator = "Other/Child".to_owned();
        assert!(matches!(
            plan_restore_snapshot_tree(
                &workspace,
                &base_revision,
                import_authority(),
                disconnected
            ),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let (mut wrong_sidecar, _, _) = snapshot_restore_tree("Recovered");
        let wrong = AnnotationStore::empty(NodeId::new_v4())
            .to_pretty_json()
            .unwrap()
            .into_bytes();
        wrong_sidecar[0].annotation_sidecar = Some(WorkspaceRestoreAnnotationSidecar {
            sha256: digest_bytes(&wrong),
            bytes: wrong,
        });
        assert!(matches!(
            plan_restore_snapshot_tree(
                &workspace,
                &base_revision,
                import_authority(),
                wrong_sidecar
            ),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let (mut wrong_digest, _, _) = snapshot_restore_tree("WrongDigest");
        wrong_digest[0].resources[0].sha256 = "0".repeat(64);
        assert!(matches!(
            plan_restore_snapshot_tree(
                &workspace,
                &base_revision,
                import_authority(),
                wrong_digest
            ),
            Err(WorkspaceTransactionError::VerificationFailed(_))
        ));

        let existing = create_child_node(&workspace, "Existing").unwrap();
        let latest = read_workspace_revision(&workspace).unwrap();
        let (portable_collision, _, _) = snapshot_restore_tree("existing");
        assert!(matches!(
            plan_restore_snapshot_tree(&workspace, &latest, import_authority(), portable_collision),
            Err(WorkspaceTransactionError::DestinationExists(_))
        ));
        let (mut conflict, _, _) = snapshot_restore_tree("Conflict");
        conflict[0].node_id = existing.id;
        conflict[0].exact_source = replace_node_id(&conflict[0].exact_source, existing.id).unwrap();
        conflict[0].document_sha256 = digest_bytes(conflict[0].exact_source.as_bytes());
        assert!(matches!(
            plan_restore_snapshot_tree(&workspace, &latest, import_authority(), conflict),
            Err(WorkspaceTransactionError::Metadata(_))
        ));

        let (nodes, _, _) = snapshot_restore_tree("Stale");
        let stale =
            plan_restore_snapshot_tree(&workspace, &latest, import_authority(), nodes).unwrap();
        create_child_node(&workspace, "Concurrent").unwrap();
        assert!(matches!(
            commit_workspace_transaction(&stale),
            Err(WorkspaceTransactionError::StaleRevision { .. })
        ));
        assert!(!workspace.join("Stale").exists());
    }

    #[test]
    fn every_snapshot_restore_step_boundary_recovers_the_complete_tree() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        for failure_boundary in 0..=1 {
            let (nodes, _, _) = snapshot_restore_tree("RecoverableTree");
            let plan =
                plan_restore_snapshot_tree(&workspace, &base_revision, import_authority(), nodes)
                    .unwrap();
            assert_eq!(plan.steps.len(), 1);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();

            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary)
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            let report = recover_workspace_transactions(&workspace).unwrap();
            assert_eq!(report.applying_rolled_back, 1);
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
            assert!(!workspace.join("RecoverableTree").exists());
        }
    }

    #[test]
    fn committed_snapshot_restore_journal_replays_without_rolling_back_the_tree() {
        let (_temporary, workspace) = setup();
        let base_revision = read_workspace_revision(&workspace).unwrap();
        let (nodes, root_id, child_id) = snapshot_restore_tree("CommittedTree");
        let plan =
            plan_restore_snapshot_tree(&workspace, &base_revision, import_authority(), nodes)
                .unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        write_journal(&transaction, &journal).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        apply_journal_steps(&workspace, &transaction, &journal.steps, None).unwrap();
        verify_plan_outcome(&plan).unwrap();
        mark_journal_committed(&mut journal, read_workspace_revision(&workspace).unwrap()).unwrap();
        write_journal(&transaction, &journal).unwrap();

        let report = recover_workspace_transactions(&workspace).unwrap();
        assert_eq!(report.committed_cleaned, 1);
        assert!(!transaction.exists());
        let inventory = scan_workspace(&workspace);
        assert!(inventory.is_valid());
        assert!(inventory.nodes.iter().any(|node| node.id == Some(root_id)));
        assert!(inventory.nodes.iter().any(|node| node.id == Some(child_id)));
        assert_eq!(
            recover_workspace_transactions(&workspace).unwrap(),
            RecoveryReport::default()
        );
    }

    #[test]
    fn every_step_boundary_can_recover_an_applying_move() {
        let (_temporary, workspace) = setup();
        let alpha = create_child_node(&workspace, "Alpha").unwrap();
        let group = create_child_node(&workspace, "Group").unwrap();
        append_document(&workspace, &format!("\nSee node:{}[Alpha].\n", alpha.id));
        let base_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=1 {
            let plan = plan_move_node(&workspace, alpha.id, group.id, "Alpha").unwrap();
            assert_eq!(plan.steps.len(), 1);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();

            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary)
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            let report = recover_workspace_transactions(&workspace).unwrap();
            assert_eq!(report.applying_rolled_back, 1);
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
            assert!(workspace.join("Alpha/Alpha.adoc").is_file());
            assert!(!workspace.join("Group/Alpha").exists());
            assert!(
                fs::read_to_string(workspace.join("Notes.adoc"))
                    .unwrap()
                    .contains(&alpha.id.to_string())
            );
        }
    }

    #[test]
    fn every_trash_and_restore_step_boundary_rolls_back_exactly() {
        let (_temporary, workspace) = setup();
        let node = create_child_node(&workspace, "CrashSafeTrash").unwrap();
        fs::write(node.path.join("asset.bin"), b"exact resource bytes").unwrap();
        let active_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=2 {
            let plan = plan_trash_node_at(&workspace, node.id, "2026-08-24T12:00:00Z").unwrap();
            assert_eq!(plan.steps.len(), 2);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();
            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary),
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert_eq!(
                recover_workspace_transactions(&workspace)
                    .unwrap()
                    .applying_rolled_back,
                1
            );
            assert_eq!(
                read_workspace_revision(&workspace).unwrap(),
                active_revision
            );
            assert_eq!(
                fs::read(node.path.join("asset.bin")).unwrap(),
                b"exact resource bytes"
            );
            assert!(scan_workspace(&workspace).trash_items.is_empty());
        }

        commit_workspace_transaction(
            &plan_trash_node_at(&workspace, node.id, "2026-08-24T12:00:00Z").unwrap(),
        )
        .unwrap();
        let trashed_revision = read_workspace_revision(&workspace).unwrap();
        let item_id = scan_workspace(&workspace).trash_items[0]
            .manifest
            .trash_item_id();
        for failure_boundary in 0..=2 {
            let plan =
                plan_restore_trash_item(&workspace, item_id, crate::TrashRestoreMode::Original)
                    .unwrap();
            assert_eq!(plan.steps.len(), 2);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(&plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();
            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary),
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert_eq!(
                recover_workspace_transactions(&workspace)
                    .unwrap()
                    .applying_rolled_back,
                1
            );
            assert_eq!(
                read_workspace_revision(&workspace).unwrap(),
                trashed_revision
            );
            assert!(!node.path.exists());
            assert_eq!(scan_workspace(&workspace).trash_items.len(), 1);
        }
    }

    #[test]
    fn every_resource_batch_trash_and_restore_boundary_rolls_back_exactly() {
        let (_temporary, workspace) = setup();
        let owner = create_child_node(&workspace, "ResourceCrash").unwrap();
        fs::write(owner.path.join("a.bin"), b"alpha").unwrap();
        fs::write(owner.path.join("b.bin"), b"beta").unwrap();
        let selections = vec![
            crate::TrashResourceSelection {
                owner_node_id: owner.id,
                name: "a.bin".to_owned(),
            },
            crate::TrashResourceSelection {
                owner_node_id: owner.id,
                name: "b.bin".to_owned(),
            },
        ];
        for failure_boundary in 0..=3 {
            let plan =
                plan_trash_resources_at(&workspace, selections.clone(), "2026-08-24T12:00:00Z")
                    .unwrap();
            assert_eq!(plan.steps.len(), 3);
            assert!(matches!(
                commit_workspace_transaction_internal(&plan, Some(failure_boundary), None),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert_eq!(fs::read(owner.path.join("a.bin")).unwrap(), b"alpha");
            assert_eq!(fs::read(owner.path.join("b.bin")).unwrap(), b"beta");
            assert!(scan_workspace(&workspace).trash_items.is_empty());
        }

        commit_workspace_transaction(
            &plan_trash_resources_at(&workspace, selections, "2026-08-24T12:00:00Z").unwrap(),
        )
        .unwrap();
        let item_id = scan_workspace(&workspace).trash_items[0]
            .manifest
            .trash_item_id();
        for failure_boundary in 0..=2 {
            let plan =
                plan_restore_trash_item(&workspace, item_id, crate::TrashRestoreMode::Original)
                    .unwrap();
            assert!(matches!(
                commit_workspace_transaction_internal(&plan, Some(failure_boundary), None),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert!(!owner.path.join("a.bin").exists());
            assert_eq!(scan_workspace(&workspace).trash_items.len(), 2);
        }
    }

    #[test]
    fn every_legacy_trash_migration_boundary_restores_direct_layout() {
        let (temporary, workspace) = setup();
        let node = create_child_node(&workspace, "LegacyCrash").unwrap();
        commit_workspace_transaction(
            &plan_trash_node_at(&workspace, node.id, "2026-08-24T12:00:00Z").unwrap(),
        )
        .unwrap();
        let inventory = scan_workspace(&workspace);
        let item = &inventory.trash_items[0];
        let direct = workspace.join(TRASH_NODE_NAME).join("LegacyCrash");
        fs::rename(&item.payload_path, &direct).unwrap();
        fs::remove_dir_all(
            workspace
                .join(TRASH_NODE_NAME)
                .join(crate::TRASH_ITEMS_DIRECTORY_NAME),
        )
        .unwrap();
        let snapshots = temporary.path().join("legacy-snapshots");
        fs::create_dir(&snapshots).unwrap();
        let backup = prepare_legacy_trash_migration_backup(&workspace, &snapshots).unwrap();
        for failure_boundary in 0..=2 {
            let plan = plan_migrate_legacy_workspace_trash_at_with_backup(
                &workspace,
                "2026-08-24T12:00:00Z",
                &backup,
            )
            .unwrap();
            assert_eq!(plan.steps.len(), 2);
            assert!(matches!(
                commit_workspace_transaction_internal(&plan, Some(failure_boundary), None),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert!(direct.join("LegacyCrash.adoc").is_file());
            assert!(scan_workspace(&workspace).legacy_trash_format);
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn committed_node_and_resource_trash_journals_replay_as_committed() {
        let (_temporary, workspace) = setup();
        let node = create_child_node(&workspace, "CommittedTrash").unwrap();
        let plan = plan_trash_node_at(&workspace, node.id, "2026-08-24T12:00:00Z").unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        write_journal(&transaction, &journal).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        apply_journal_steps(&workspace, &transaction, &journal.steps, None).unwrap();
        verify_plan_outcome(&plan).unwrap();
        mark_journal_committed(&mut journal, read_workspace_revision(&workspace).unwrap()).unwrap();
        write_journal(&transaction, &journal).unwrap();
        assert_eq!(
            recover_workspace_transactions(&workspace)
                .unwrap()
                .committed_cleaned,
            1
        );
        assert!(!node.path.exists());

        let owner = create_child_node(&workspace, "CommittedResources").unwrap();
        fs::write(owner.path.join("a.bin"), b"alpha").unwrap();
        fs::write(owner.path.join("b.bin"), b"beta").unwrap();
        let plan = plan_trash_resources_at(
            &workspace,
            vec![
                crate::TrashResourceSelection {
                    owner_node_id: owner.id,
                    name: "a.bin".to_owned(),
                },
                crate::TrashResourceSelection {
                    owner_node_id: owner.id,
                    name: "b.bin".to_owned(),
                },
            ],
            "2026-08-24T12:00:00Z",
        )
        .unwrap();
        let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
        fs::create_dir(&transaction).unwrap();
        let mut journal = prepare_journal(&plan, &transaction).unwrap();
        write_journal(&transaction, &journal).unwrap();
        mark_journal_applying(&mut journal).unwrap();
        write_journal(&transaction, &journal).unwrap();
        apply_journal_steps(&workspace, &transaction, &journal.steps, None).unwrap();
        verify_plan_outcome(&plan).unwrap();
        mark_journal_committed(&mut journal, read_workspace_revision(&workspace).unwrap()).unwrap();
        write_journal(&transaction, &journal).unwrap();
        assert_eq!(
            recover_workspace_transactions(&workspace)
                .unwrap()
                .committed_cleaned,
            1
        );
        assert!(!owner.path.join("a.bin").exists());
        assert!(!owner.path.join("b.bin").exists());
        assert_eq!(scan_workspace(&workspace).trash_items.len(), 3);
    }

    #[test]
    fn every_step_boundary_exactly_recovers_a_recurring_task_completion() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Tasks");
        fs::create_dir(&workspace).unwrap();
        fs::write(workspace.join(".weftext-format"), "weftext.asciidoc.v1\n").unwrap();
        let root_id = NodeId::from_str("550e8400-e29b-41d4-a716-446655440010").unwrap();
        let task_id = crate::TaskId::from_str("11111111-1111-4111-8111-111111111111").unwrap();
        let source = format!(
            "---\nweftext:\n  id: \"{root_id}\"\n---\n= Tasks\n\n* [ ] Repeat task:[id={task_id},due=2026-08-24,rrule=\"FREQ=DAILY;COUNT=2\",repeat-from=due]\n"
        );
        fs::write(workspace.join("Tasks.adoc"), &source).unwrap();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=1 {
            let snapshot = read_node_document(&workspace).unwrap();
            let completion = crate::plan_task_recurrence_transaction(
                &workspace,
                root_id,
                &snapshot.revision,
                &crate::TaskEditTarget::Id { id: task_id },
                &crate::TaskRecurrenceCompletionContext {
                    completed_at: crate::TaskDateTime::Date("2026-08-24".to_owned()),
                    utc_offset_minutes: 0,
                },
            )
            .unwrap();
            let plan = completion.workspace_transaction();
            assert_eq!(plan.steps.len(), 1);
            let transaction = transaction_path(&workspace, &plan.plan_id).unwrap();
            fs::create_dir(&transaction).unwrap();
            let mut journal = prepare_journal(plan, &transaction).unwrap();
            write_journal(&transaction, &journal).unwrap();
            mark_journal_applying(&mut journal).unwrap();
            write_journal(&transaction, &journal).unwrap();

            assert!(matches!(
                apply_journal_steps(
                    &workspace,
                    &transaction,
                    &journal.steps,
                    Some(failure_boundary)
                ),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            let report = recover_workspace_transactions(&workspace).unwrap();
            assert_eq!(report.applying_rolled_back, 1);
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
            assert_eq!(
                fs::read_to_string(workspace.join("Tasks.adoc")).unwrap(),
                source
            );
        }
    }

    #[test]
    fn every_step_boundary_rolls_back_suggestion_document_and_sidecar_together() {
        let (_temporary, workspace) = setup();
        append_document(&workspace, "\n= Review\n\nBefore after.\n");
        let root_id = scan_workspace(&workspace).nodes[0].id.unwrap();
        let snapshot = read_node_document(&workspace).unwrap();
        let position = u64::try_from(snapshot.source.find("after").unwrap()).unwrap();
        let sidecar_snapshot = local_annotation_snapshot(&workspace, root_id);
        let create = plan_annotation_action(
            &workspace,
            &sidecar_snapshot,
            AnnotationAction::Create {
                kind: AnnotationKind::SuggestionInsert,
                target: crate::AnnotationTargetIntent::InsertionPoint { position },
                appearance: None,
                labels: Vec::new(),
                body_source: None,
                suggested_source: Some("inserted ".to_owned()),
                author_id: uuid::Uuid::new_v4(),
                author_name: "Reviewer".to_owned(),
                timestamp: "2026-08-24T12:00:00+08:00".to_owned(),
            },
        )
        .unwrap();
        commit_workspace_transaction(&create).unwrap();
        let annotation_id = read_local_annotations(&workspace, root_id).annotations[0].id;
        let document_path = workspace.join("Notes.adoc");
        let sidecar_path = workspace.join(ANNOTATIONS_FILE_NAME);
        let base_document = fs::read(&document_path).unwrap();
        let base_sidecar = fs::read(&sidecar_path).unwrap();
        let base_revision = read_workspace_revision(&workspace).unwrap();

        for failure_boundary in 0..=2 {
            let sidecar_snapshot = local_annotation_snapshot(&workspace, root_id);
            let plan = plan_annotation_action(
                &workspace,
                &sidecar_snapshot,
                AnnotationAction::AcceptSuggestion {
                    annotation_id,
                    timestamp: "2026-08-24T12:01:00+08:00".to_owned(),
                },
            )
            .unwrap();
            assert_eq!(plan.steps.len(), 2);
            assert!(matches!(
                commit_workspace_transaction_internal(&plan, Some(failure_boundary), None),
                Err(WorkspaceTransactionError::InjectedFailure(boundary))
                    if boundary == failure_boundary
            ));
            assert_eq!(fs::read(&document_path).unwrap(), base_document);
            assert_eq!(fs::read(&sidecar_path).unwrap(), base_sidecar);
            assert_eq!(read_workspace_revision(&workspace).unwrap(), base_revision);
        }
    }
}
