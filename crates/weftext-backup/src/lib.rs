#![forbid(unsafe_code)]

//! Versioned, fail-closed physical workspace backup and restore contracts.

mod server_control_plane;

pub use server_control_plane::*;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};
use weftext_core::{
    ANNOTATIONS_FILE_NAME, AnnotationReplicaCompleteness, CommittedWorkspaceTransaction,
    MANAGED_DOCUMENT_PROFILE_ID, NodeId, PhysicalEntryKind, PhysicalInventoryError,
    PhysicalTreeInventory, WorkspaceContentKind, WorkspaceImportAuthority, WorkspaceImportResource,
    WorkspaceRestoreAnnotationSidecar, WorkspaceRestoreTreeNode, WorkspaceRevision,
    WorkspaceRevisionError, WorkspaceTransactionError, WorkspaceTransactionLease,
    acquire_workspace_transaction_lease as acquire_core_workspace_transaction_lease,
    capture_stable_physical_tree, capture_stable_workspace_physical_inventory,
    commit_workspace_transaction, plan_create_child_node, plan_restore_snapshot_tree,
    read_node_annotations, read_workspace_revision, scan_workspace,
};

pub const SNAPSHOT_MANIFEST_SCHEMA: &str = "weftext.full-workspace-backup.v1";
pub const SNAPSHOT_COMPLETION_SCHEMA: &str = "weftext.full-workspace-backup-completion.v1";
pub const SNAPSHOT_PROTECTION_SCHEMA: &str = "weftext.full-workspace-backup-protection.v1";
pub const BACKUP_PLAN_SCHEMA: &str = "weftext.full-workspace-backup-plan.v1";
pub const RESTORE_PLAN_SCHEMA: &str = "weftext.full-workspace-restore-plan.v1";
pub const RESTORE_DRILL_PLAN_SCHEMA: &str = "weftext.full-workspace-restore-drill-plan.v1";
pub const RESTORE_DRILL_RESULT_SCHEMA: &str = "weftext.full-workspace-restore-drill-result.v1";
pub const SNAPSHOT_RETENTION_PLAN_SCHEMA: &str = "weftext.snapshot-retention-plan.v1";
pub const SNAPSHOT_RETENTION_JOURNAL_SCHEMA: &str = "weftext.snapshot-retention-journal.v1";
pub const SNAPSHOT_RETENTION_RECEIPT_SCHEMA: &str = "weftext.snapshot-retention-receipt.v1";
pub const SCOPED_RESTORE_PLAN_SCHEMA: &str = "weftext.scoped-workspace-restore-plan.v1";
pub const SCOPED_RESTORE_RECEIPT_SCHEMA: &str = "weftext.scoped-workspace-restore-receipt.v1";
pub const SNAPSHOT_MANIFEST_FILE: &str = "manifest.json";
pub const SNAPSHOT_COMPLETION_FILE: &str = "complete.json";
pub const SNAPSHOT_CONTENT_DIRECTORY: &str = "content";
pub const SNAPSHOT_PROTECTION_FILE: &str = "protected.json";

const SNAPSHOT_DIRECTORY_PREFIX: &str = "weftext-backup-";
const RESTORE_DRILL_DIRECTORY_PREFIX: &str = "weftext-restore-drill-";
const RETENTION_TRANSACTION_PREFIX: &str = ".__weftext-backup-retention-";
const RETENTION_LOCK_DIRECTORY: &str = ".__weftext-backup-retention.lock";
const RETENTION_RECEIPT_PREFIX: &str = "weftext-retention-receipt-";
const RETENTION_JOURNAL_FILE: &str = "journal.json";
const RETENTION_HOLDING_DIRECTORY: &str = "holding";
const RETENTION_COMMIT_FILE: &str = "committed.json";
const WORKSPACE_ROOT_TRANSACTION_PREFIX: &str = ".__weftext-transaction-workspace-";
const DOCUMENT_PROFILE: &str = MANAGED_DOCUMENT_PROFILE_ID;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MANIFEST_ENTRIES: usize = 1_000_000;
const COPY_BUFFER_BYTES: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupScope {
    FullWorkspace,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupEntryType {
    Directory,
    File,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BackupEntry {
    pub locator: String,
    #[serde(rename = "type")]
    pub entry_type: BackupEntryType,
    pub length: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotManifest {
    pub schema: String,
    pub scope: BackupScope,
    pub snapshot_id: String,
    pub document_profile: String,
    pub workspace_root_id: String,
    pub workspace_revision: String,
    pub root_name: String,
    pub exclusions: Vec<String>,
    pub entries: Vec<BackupEntry>,
    pub entry_count: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotCompletion {
    schema: String,
    snapshot_id: String,
    manifest_sha256: String,
    manifest_length: u64,
    entry_count: u64,
    total_bytes: u64,
    created_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotProtection {
    pub schema: String,
    pub snapshot_id: Uuid,
    pub protected_at_unix_ms: u64,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotRetentionPolicy {
    /// Number of newest unprotected snapshots to retain in addition to every protected snapshot.
    pub keep_latest_unprotected: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRetentionItem {
    pub snapshot_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub created_at_unix_ms: u64,
    pub manifest_sha256: String,
    pub protection: Option<SnapshotProtection>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRetentionPlan {
    pub schema: String,
    pub operation_id: Uuid,
    pub plan_digest: String,
    pub backup_parent: PathBuf,
    pub receipt_file: PathBuf,
    pub policy: SnapshotRetentionPolicy,
    pub retained: Vec<SnapshotRetentionItem>,
    pub pruned: Vec<SnapshotRetentionItem>,
    #[serde(skip)]
    parent_binding: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotRetentionReceipt {
    pub schema: String,
    pub operation_id: Uuid,
    pub plan_digest: String,
    pub policy: SnapshotRetentionPolicy,
    pub retained_snapshot_ids: Vec<Uuid>,
    pub pruned_snapshot_ids: Vec<Uuid>,
    pub completed_at_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotRetentionRecoveryReport {
    pub rolled_back_operation_ids: Vec<Uuid>,
    pub finalized_operation_ids: Vec<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotRetentionJournalEntry {
    snapshot_id: Uuid,
    directory_name: String,
    manifest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotRetentionJournal {
    schema: String,
    operation_id: Uuid,
    plan_digest: String,
    parent_binding: String,
    policy: SnapshotRetentionPolicy,
    retained_snapshot_ids: Vec<Uuid>,
    entries: Vec<SnapshotRetentionJournalEntry>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullWorkspaceBackupPlan {
    pub schema: String,
    pub snapshot_id: Uuid,
    pub plan_digest: String,
    pub workspace_root: PathBuf,
    pub backup_parent: PathBuf,
    pub snapshot_directory: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entries: Vec<BackupEntry>,
    pub entry_count: u64,
    pub total_bytes: u64,
    #[serde(skip)]
    root_name: String,
    #[serde(skip)]
    source_binding: String,
    #[serde(skip)]
    destination_binding: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullWorkspaceBackupReceipt {
    pub schema: String,
    pub snapshot_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FullWorkspaceBackupVerification {
    pub schema: String,
    pub snapshot_id: Uuid,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub complete: bool,
    pub created_at_unix_ms: u64,
    pub protection: Option<SnapshotProtection>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternateRestorePlan {
    pub schema: String,
    pub restore_id: Uuid,
    pub plan_digest: String,
    pub snapshot_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub destination_root: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entries: Vec<BackupEntry>,
    pub entry_count: u64,
    pub total_bytes: u64,
    #[serde(skip)]
    root_name: String,
    #[serde(skip)]
    snapshot_workspace_root: PathBuf,
    #[serde(skip)]
    snapshot_binding: String,
    #[serde(skip)]
    destination_binding: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternateRestoreReceipt {
    pub schema: String,
    pub restore_id: Uuid,
    pub snapshot_id: Uuid,
    pub destination_root: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub bytewise_verified: bool,
}

/// The amount of one snapshot node selected for an in-workspace restore.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedRestoreScope {
    /// Restores only the selected node, its canonical document, optional annotation sidecar, and
    /// directly owned resources. Existing child nodes in the snapshot are not selected.
    SingleNode,
    /// Restores the selected node and every managed descendant as one reviewed tree.
    Subtree,
}

/// The semantic role of one exact snapshot entry in a scoped restore preview.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedRestoreEntryKind {
    NodeDirectory,
    CanonicalDocument,
    AnnotationSidecar,
    Resource,
}

/// One source-to-destination mapping bound into a scoped restore plan digest.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedRestoreEntry {
    pub owner_node_id: NodeId,
    pub kind: ScopedRestoreEntryKind,
    pub source_locator: String,
    pub destination_locator: String,
    #[serde(rename = "type")]
    pub entry_type: BackupEntryType,
    pub length: u64,
    pub sha256: String,
}

/// One identity-preserving node mapping in a scoped restore preview.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedRestoreNode {
    pub node_id: NodeId,
    pub source_locator: String,
    pub destination_locator: String,
}

/// Stable machine-readable reason that the reviewed exact tree cannot be committed through Core.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedRestoreBlockerCode {
    CoreExactTreeCreateUnavailable,
    CoreImportSafetyEnvelopeExceeded,
}

/// An explicit exact-tree contract or bounded-safety blocker exposed by the read-only preview.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScopedRestoreBlocker {
    pub code: ScopedRestoreBlockerCode,
    pub message: String,
}

/// Whether Core can execute the exact reviewed scoped restore as one recoverable transaction.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScopedRestoreCommitState {
    Ready,
    Blocked,
}

/// Read-only, identity-preserving scoped restore preview.
///
/// `entries` is an exact allow-list. Any unmanaged, ignored, reserved, or otherwise unowned
/// physical entry inside the selected boundary rejects planning rather than being omitted.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedRestorePlan {
    pub schema: String,
    pub restore_id: Uuid,
    pub plan_digest: String,
    pub scope: ScopedRestoreScope,
    pub snapshot_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub snapshot_manifest_sha256: String,
    pub source_workspace_root_id: NodeId,
    pub source_workspace_revision: WorkspaceRevision,
    pub source_node_id: NodeId,
    pub target_workspace_root: PathBuf,
    pub target_workspace_root_id: NodeId,
    pub target_workspace_revision: WorkspaceRevision,
    pub destination_parent_id: NodeId,
    pub destination_name: String,
    pub destination_locator: String,
    pub nodes: Vec<ScopedRestoreNode>,
    pub entries: Vec<ScopedRestoreEntry>,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub commit_state: ScopedRestoreCommitState,
    pub blockers: Vec<ScopedRestoreBlocker>,
    #[serde(skip)]
    snapshot_workspace_root: PathBuf,
    #[serde(skip)]
    snapshot_binding: String,
    #[serde(skip)]
    target_binding: String,
}

/// Receipt returned only after Core's recoverable transaction and exact reopen verification pass.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopedRestoreReceipt {
    pub schema: String,
    pub restore_id: Uuid,
    pub snapshot_id: Uuid,
    pub scope: ScopedRestoreScope,
    pub source_node_id: NodeId,
    pub destination_parent_id: NodeId,
    pub destination_locator: String,
    pub restored_node_ids: Vec<NodeId>,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub transaction: CommittedWorkspaceTransaction,
    pub exact_bytes_verified: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDrillPlan {
    pub schema: String,
    pub drill_id: Uuid,
    pub plan_digest: String,
    pub snapshot_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub drill_parent: PathBuf,
    pub results_parent: PathBuf,
    pub drill_directory: PathBuf,
    pub destination_root: PathBuf,
    pub result_file: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    #[serde(skip)]
    root_name: String,
    #[serde(skip)]
    snapshot_binding: String,
    #[serde(skip)]
    drill_parent_binding: String,
    #[serde(skip)]
    results_parent_binding: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreDrillResult {
    pub schema: String,
    pub drill_id: Uuid,
    pub snapshot_id: Uuid,
    pub completed_at_unix_ms: u64,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub opened_clean: bool,
    pub bytewise_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreDrillReceipt {
    pub schema: String,
    pub drill_id: Uuid,
    pub snapshot_id: Uuid,
    pub destination_root: PathBuf,
    pub result_file: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub opened_clean: bool,
    pub bytewise_verified: bool,
}

struct VerifiedSnapshot {
    directory: PathBuf,
    workspace_content_root: PathBuf,
    manifest: SnapshotManifest,
    manifest_sha256: String,
    snapshot_id: Uuid,
    workspace_root_id: NodeId,
    workspace_revision: WorkspaceRevision,
    created_at_unix_ms: u64,
    protection: Option<SnapshotProtection>,
}

/// Builds a full physical-workspace backup preview without writing to either location.
///
/// # Errors
///
/// Fails closed for an invalid workspace, linked/reparse paths, unreadable or special entries,
/// unfinished transactions, unsafe destinations, or an unstable physical inventory.
pub fn plan_full_workspace_backup(
    workspace_root: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
) -> Result<FullWorkspaceBackupPlan, BackupError> {
    replan_full_workspace_backup(workspace_root, backup_parent, Uuid::new_v4())
}

/// Rebuilds the same read-only backup preview for a caller-provided reviewed snapshot ID.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_full_workspace_backup`] and rejects a non-v4 ID.
pub fn replan_full_workspace_backup(
    workspace_root: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
    snapshot_id: Uuid,
) -> Result<FullWorkspaceBackupPlan, BackupError> {
    validate_v4(snapshot_id, "snapshot")?;
    let workspace_root = canonical_existing_directory(workspace_root.as_ref(), "workspace root")?;
    let workspace_lease = acquire_core_workspace_transaction_lease(&workspace_root)
        .map_err(BackupError::CoreTransaction)?;
    replan_full_workspace_backup_with_lease(
        &workspace_root,
        backup_parent.as_ref(),
        snapshot_id,
        &workspace_lease,
    )
}

fn replan_full_workspace_backup_with_lease(
    workspace_root: &Path,
    backup_parent: &Path,
    snapshot_id: Uuid,
    workspace_lease: &WorkspaceTransactionLease,
) -> Result<FullWorkspaceBackupPlan, BackupError> {
    let backup_parent = canonical_existing_directory(backup_parent, "backup parent")?;
    reject_workspace_marker_ancestor(&backup_parent, "backup parent")?;
    if backup_parent.starts_with(workspace_root) {
        return Err(BackupError::Path(
            "backup parent must be disjoint from the workspace".to_owned(),
        ));
    }
    let snapshot_directory =
        backup_parent.join(format!("{SNAPSHOT_DIRECTORY_PREFIX}{snapshot_id}"));
    reject_linked_existing_ancestors(&snapshot_directory)?;
    if snapshot_directory.exists() {
        return Err(BackupError::SnapshotExists(snapshot_directory));
    }

    let (workspace_root_id, workspace_revision, root_name, entries) =
        stable_workspace_inventory(workspace_root, workspace_lease)?;
    let entry_count = u64::try_from(entries.len())
        .map_err(|_| BackupError::InvalidManifest("too many physical entries".to_owned()))?;
    let total_bytes = total_file_bytes(&entries)?;
    let manifest = SnapshotManifest {
        schema: SNAPSHOT_MANIFEST_SCHEMA.to_owned(),
        scope: BackupScope::FullWorkspace,
        snapshot_id: snapshot_id.hyphenated().to_string(),
        document_profile: DOCUMENT_PROFILE.to_owned(),
        workspace_root_id: workspace_root_id.to_string(),
        workspace_revision: workspace_revision.to_string(),
        root_name: root_name.clone(),
        exclusions: Vec::new(),
        entries: entries.clone(),
        entry_count,
        total_bytes,
    };
    validate_manifest(&manifest)?;
    let manifest_bytes = manifest_bytes(&manifest)?;
    let manifest_sha256 = sha256(&manifest_bytes);
    let source_binding = path_binding(workspace_root)?;
    let destination_binding = path_binding(&backup_parent)?;
    let plan_digest = backup_plan_digest(&manifest_sha256, &source_binding, &destination_binding);
    Ok(FullWorkspaceBackupPlan {
        schema: BACKUP_PLAN_SCHEMA.to_owned(),
        snapshot_id,
        plan_digest,
        workspace_root: workspace_root.to_path_buf(),
        backup_parent,
        snapshot_directory,
        workspace_root_id,
        workspace_revision,
        manifest_sha256,
        entries,
        entry_count,
        total_bytes,
        root_name,
        source_binding,
        destination_binding,
    })
}

/// Commits an exact preview using create-new writes and a completion marker written last.
///
/// # Errors
///
/// Rejects tampered plans, stale or racing workspaces, destination collisions, failed durability
/// operations, and any digest or bytewise verification mismatch.
pub fn commit_full_workspace_backup(
    plan: &FullWorkspaceBackupPlan,
) -> Result<FullWorkspaceBackupReceipt, BackupError> {
    validate_backup_plan(plan)?;
    let workspace_root = canonical_existing_directory(&plan.workspace_root, "workspace root")?;
    let workspace_lease = acquire_core_workspace_transaction_lease(&workspace_root)
        .map_err(BackupError::CoreTransaction)?;
    let current = replan_full_workspace_backup_with_lease(
        &workspace_root,
        &plan.backup_parent,
        plan.snapshot_id,
        &workspace_lease,
    )?;
    if current.plan_digest != plan.plan_digest || current.manifest_sha256 != plan.manifest_sha256 {
        return Err(BackupError::StalePreview);
    }
    if plan.snapshot_directory.exists() {
        return Err(BackupError::SnapshotExists(plan.snapshot_directory.clone()));
    }

    fs::create_dir(&plan.snapshot_directory).map_err(BackupError::Io)?;
    let created_snapshot =
        canonical_existing_directory(&plan.snapshot_directory, "new snapshot directory")?;
    if created_snapshot != plan.snapshot_directory {
        return Err(BackupError::PathEscape(created_snapshot));
    }
    commit_snapshot_contents(plan, &workspace_lease)
}

/// Verifies marker binding, manifest syntax, exact physical inventory, lengths and SHA-256.
///
/// # Errors
///
/// Rejects incomplete snapshots, unknown fields, unsafe paths, linked content, tampering, and
/// physical inventory mismatches.
pub fn verify_full_workspace_snapshot(
    snapshot_directory: impl AsRef<Path>,
) -> Result<FullWorkspaceBackupVerification, BackupError> {
    let verified = verify_snapshot_internal(snapshot_directory.as_ref())?;
    Ok(FullWorkspaceBackupVerification {
        schema: "weftext.full-workspace-backup-verification.v1".to_owned(),
        snapshot_id: verified.snapshot_id,
        workspace_root_id: verified.workspace_root_id,
        workspace_revision: verified.workspace_revision,
        manifest_sha256: verified.manifest_sha256,
        entry_count: verified.manifest.entry_count,
        total_bytes: verified.manifest.total_bytes,
        complete: true,
        created_at_unix_ms: verified.created_at_unix_ms,
        protection: verified.protection,
    })
}

/// Permanently marks a verified snapshot as a protected restore point.
///
/// Protection is create-new and idempotent only for the exact same label. This v1 safety boundary
/// intentionally has no unprotect operation, so retention cannot silently remove a point that was
/// explicitly protected.
///
/// # Errors
///
/// Rejects an invalid/tampered snapshot, an empty or unbounded label, linked metadata, and an
/// existing protection record with different contents.
pub fn protect_full_workspace_snapshot(
    snapshot_directory: impl AsRef<Path>,
    label: impl Into<String>,
) -> Result<SnapshotProtection, BackupError> {
    let verified = verify_snapshot_internal(snapshot_directory.as_ref())?;
    let backup_parent = verified
        .directory
        .parent()
        .ok_or_else(|| BackupError::Path("snapshot directory has no backup parent".to_owned()))?;
    let retention_lock = retention_lock_path(backup_parent);
    if retention_lock.try_exists().map_err(BackupError::Io)? {
        return Err(BackupError::UnfinishedTransaction(retention_lock));
    }
    let label = label.into();
    validate_protection_label(&label)?;
    let expected = SnapshotProtection {
        schema: SNAPSHOT_PROTECTION_SCHEMA.to_owned(),
        snapshot_id: verified.snapshot_id,
        protected_at_unix_ms: unix_time_ms()?,
        label,
    };
    if let Some(existing) = verified.protection.as_ref() {
        if existing.label == expected.label {
            return Ok(existing.clone());
        }
        return Err(BackupError::Verification(
            "snapshot is already protected with a different label".to_owned(),
        ));
    }
    let mut bytes = serde_json::to_vec_pretty(&expected).map_err(BackupError::Json)?;
    bytes.push(b'\n');
    let protection_path = verified.directory.join(SNAPSHOT_PROTECTION_FILE);
    if let Err(error) = write_new_file(&protection_path, &bytes)
        && !matches!(&error, BackupError::Io(io_error) if io_error.kind() == io::ErrorKind::AlreadyExists)
    {
        return Err(error);
    }
    let reopened = verify_snapshot_internal(&verified.directory)?;
    match reopened.protection {
        Some(protection) if protection.label == expected.label => Ok(protection),
        Some(_) => Err(BackupError::Verification(
            "snapshot is already protected with a different label".to_owned(),
        )),
        None => Err(BackupError::Verification(
            "snapshot protection failed reopen verification".to_owned(),
        )),
    }
}

/// Builds a read-only retention preview for one backup destination.
///
/// Every protected snapshot is retained permanently. `keep_latest_unprotected` applies only to
/// unprotected snapshots, so adding a protected restore point never causes another snapshot to be
/// selected for deletion.
///
/// # Errors
///
/// Rejects invalid policy, incomplete/tampered snapshots, linked entries, and unfinished retention
/// transactions. Planning never creates, moves, or removes an entry.
pub fn plan_snapshot_retention(
    backup_parent: impl AsRef<Path>,
    policy: SnapshotRetentionPolicy,
) -> Result<SnapshotRetentionPlan, BackupError> {
    replan_snapshot_retention(backup_parent, policy, Uuid::new_v4())
}

/// Rebuilds an exact retention preview for a caller-provided reviewed operation ID.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_snapshot_retention`] and rejects a non-v4 ID.
pub fn replan_snapshot_retention(
    backup_parent: impl AsRef<Path>,
    policy: SnapshotRetentionPolicy,
    operation_id: Uuid,
) -> Result<SnapshotRetentionPlan, BackupError> {
    validate_v4(operation_id, "retention operation")?;
    validate_retention_policy(policy)?;
    let backup_parent = canonical_existing_directory(backup_parent.as_ref(), "backup parent")?;
    let retention_lock = retention_lock_path(&backup_parent);
    if retention_lock.try_exists().map_err(BackupError::Io)? {
        return Err(BackupError::UnfinishedTransaction(retention_lock));
    }
    if let Some(transaction) = list_retention_transactions(&backup_parent)?
        .into_iter()
        .next()
    {
        return Err(BackupError::UnfinishedTransaction(transaction));
    }
    let receipt_file = retention_receipt_path(&backup_parent, operation_id);
    reject_linked_existing_ancestors(&receipt_file)?;
    if receipt_file.exists() {
        return Err(BackupError::SnapshotExists(receipt_file));
    }

    let snapshots = inventory_retention_snapshots(&backup_parent)?;
    let keep_latest =
        usize::try_from(policy.keep_latest_unprotected).map_err(|_| BackupError::InvalidPlan)?;
    let mut retained = Vec::new();
    let mut pruned = Vec::new();
    let mut unprotected_seen = 0_usize;
    for snapshot in snapshots {
        if snapshot.protection.is_some() || unprotected_seen < keep_latest {
            if snapshot.protection.is_none() {
                unprotected_seen = unprotected_seen.saturating_add(1);
            }
            retained.push(snapshot);
        } else {
            unprotected_seen = unprotected_seen.saturating_add(1);
            pruned.push(snapshot);
        }
    }
    let parent_binding = path_binding(&backup_parent)?;
    let plan_digest =
        retention_plan_digest(operation_id, &parent_binding, policy, &retained, &pruned)?;
    Ok(SnapshotRetentionPlan {
        schema: SNAPSHOT_RETENTION_PLAN_SCHEMA.to_owned(),
        operation_id,
        plan_digest,
        backup_parent,
        receipt_file,
        policy,
        retained,
        pruned,
        parent_binding,
    })
}

/// Applies one reviewed retention plan through a marker-last recoverable move transaction.
///
/// Selected snapshots are first atomically moved into transaction holding. Only after every held
/// snapshot re-verifies is a commit marker written. A pre-marker interruption is rolled back by
/// [`recover_snapshot_retention`]; a post-marker interruption is finalized without resurrecting
/// snapshots that the reviewed plan committed to prune.
///
/// # Errors
///
/// Rejects stale/tampered plans, any changed snapshot, collisions, unsafe filesystem entries, or a
/// failure to durably record and verify the transaction. Interrupted evidence is retained.
pub fn commit_snapshot_retention(
    plan: &SnapshotRetentionPlan,
) -> Result<SnapshotRetentionReceipt, BackupError> {
    validate_retention_plan(plan)?;
    let current = replan_snapshot_retention(&plan.backup_parent, plan.policy, plan.operation_id)?;
    if current.plan_digest != plan.plan_digest
        || current.retained != plan.retained
        || current.pruned != plan.pruned
    {
        return Err(BackupError::StalePreview);
    }

    let retention_lock = retention_lock_path(&plan.backup_parent);
    fs::create_dir(&retention_lock).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            BackupError::UnfinishedTransaction(retention_lock.clone())
        } else {
            BackupError::Io(error)
        }
    })?;
    if canonical_existing_directory(&retention_lock, "retention lock")? != retention_lock {
        return Err(BackupError::PathEscape(retention_lock));
    }
    sync_directory(&plan.backup_parent)?;

    let transaction = retention_transaction_path(&plan.backup_parent, plan.operation_id);
    if transaction.exists() {
        return Err(BackupError::UnfinishedTransaction(transaction));
    }
    fs::create_dir(&transaction).map_err(BackupError::Io)?;
    let transaction = canonical_existing_directory(&transaction, "retention transaction")?;
    let journal = retention_journal_from_plan(plan);
    write_json_new(&transaction.join(RETENTION_JOURNAL_FILE), &journal)?;
    fs::create_dir(transaction.join(RETENTION_HOLDING_DIRECTORY)).map_err(BackupError::Io)?;
    sync_directory(&transaction)?;

    for entry in &journal.entries {
        let source = plan.backup_parent.join(&entry.directory_name);
        let holding = transaction
            .join(RETENTION_HOLDING_DIRECTORY)
            .join(&entry.directory_name);
        verify_retention_snapshot_binding(&source, entry)?;
        fs::rename(&source, &holding).map_err(BackupError::Io)?;
        sync_directory(&plan.backup_parent)?;
        sync_directory(&transaction.join(RETENTION_HOLDING_DIRECTORY))?;
        verify_retention_snapshot_binding(&holding, entry)?;
    }

    let receipt = retention_receipt_from_plan(plan)?;
    validate_retention_receipt(&receipt, &journal)?;
    write_json_new(&transaction.join(RETENTION_COMMIT_FILE), &receipt)?;
    sync_directory(&transaction)?;
    finalize_retention_transaction(&plan.backup_parent, &transaction, &journal, &receipt)?;
    release_retention_lock(&plan.backup_parent)?;
    Ok(receipt)
}

/// Recovers every retention transaction in a backup destination.
///
/// Transactions without a valid commit marker are rolled back. Transactions with an exact marker
/// are finalized. Unknown or contradictory evidence is never guessed or removed.
///
/// # Errors
///
/// Rejects linked, malformed, missing, or contradictory transaction evidence and any snapshot
/// that no longer matches its journal binding.
pub fn recover_snapshot_retention(
    backup_parent: impl AsRef<Path>,
) -> Result<SnapshotRetentionRecoveryReport, BackupError> {
    let backup_parent = canonical_existing_directory(backup_parent.as_ref(), "backup parent")?;
    let retention_lock = retention_lock_path(&backup_parent);
    let lock_exists = retention_lock.try_exists().map_err(BackupError::Io)?;
    if lock_exists {
        validate_empty_retention_lock(&retention_lock)?;
    }
    let transactions = list_retention_transactions(&backup_parent)?;
    if transactions.len() > 1 || (!transactions.is_empty() && !lock_exists) {
        return Err(BackupError::Verification(
            "retention recovery found contradictory transaction-lock state".to_owned(),
        ));
    }
    let mut report = SnapshotRetentionRecoveryReport {
        rolled_back_operation_ids: Vec::new(),
        finalized_operation_ids: Vec::new(),
    };
    for transaction in transactions {
        let Some(journal) = read_retention_journal_or_remove_empty(&backup_parent, &transaction)?
        else {
            continue;
        };
        let commit_path = transaction.join(RETENTION_COMMIT_FILE);
        let durable_receipt = retention_receipt_path(&backup_parent, journal.operation_id);
        if commit_path.try_exists().map_err(BackupError::Io)?
            || durable_receipt.try_exists().map_err(BackupError::Io)?
        {
            let receipt: SnapshotRetentionReceipt =
                if commit_path.try_exists().map_err(BackupError::Io)? {
                    read_json_bounded(&commit_path, &transaction, 1024 * 1024)?
                } else {
                    read_json_bounded(&durable_receipt, &backup_parent, 1024 * 1024)?
                };
            validate_retention_receipt(&receipt, &journal)?;
            finalize_retention_transaction(&backup_parent, &transaction, &journal, &receipt)?;
            report.finalized_operation_ids.push(journal.operation_id);
        } else {
            rollback_retention_transaction(&backup_parent, &transaction, &journal)?;
            report.rolled_back_operation_ids.push(journal.operation_id);
        }
    }
    if lock_exists {
        release_retention_lock(&backup_parent)?;
    }
    Ok(report)
}

/// Reads and validates one durable retention receipt.
///
/// # Errors
///
/// Rejects unknown fields, non-canonical IDs, invalid digests, timestamps, or duplicate IDs.
pub fn read_snapshot_retention_receipt(
    receipt_file: impl AsRef<Path>,
) -> Result<SnapshotRetentionReceipt, BackupError> {
    let requested = receipt_file.as_ref();
    let parent = requested
        .parent()
        .ok_or_else(|| BackupError::Path("retention receipt has no parent".to_owned()))?;
    let parent = canonical_existing_directory(parent, "retention receipt parent")?;
    let file_name = requested
        .file_name()
        .ok_or_else(|| BackupError::Path("retention receipt has no filename".to_owned()))?;
    let receipt_file = parent.join(file_name);
    let receipt: SnapshotRetentionReceipt = read_json_bounded(&receipt_file, &parent, 1024 * 1024)?;
    validate_retention_receipt_shape(&receipt)?;
    let expected = retention_receipt_path(&parent, receipt.operation_id);
    if expected != receipt_file {
        return Err(BackupError::Verification(
            "retention receipt filename does not bind its operation ID".to_owned(),
        ));
    }
    Ok(receipt)
}

/// Builds a read-only alternate-location restore dry run.
///
/// # Errors
///
/// Rejects incomplete/tampered snapshots, linked paths, unsafe or existing destinations, and a
/// destination whose name would break the canonical `X/X.adoc` layout.
pub fn plan_alternate_restore(
    snapshot_directory: impl AsRef<Path>,
    destination_root: impl AsRef<Path>,
) -> Result<AlternateRestorePlan, BackupError> {
    replan_alternate_restore(snapshot_directory, destination_root, Uuid::new_v4())
}

/// Rebuilds the same restore dry run for a caller-provided reviewed restore ID.
///
/// # Errors
///
/// Has the same behavior as [`plan_alternate_restore`] and rejects a non-v4 restore ID.
pub fn replan_alternate_restore(
    snapshot_directory: impl AsRef<Path>,
    destination_root: impl AsRef<Path>,
    restore_id: Uuid,
) -> Result<AlternateRestorePlan, BackupError> {
    validate_v4(restore_id, "restore")?;
    let verified = verify_snapshot_internal(snapshot_directory.as_ref())?;
    let destination_root = normalize_new_destination(destination_root.as_ref())?;
    let destination_name = destination_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BackupError::Path("restore destination name must be UTF-8".to_owned()))?;
    if destination_name != verified.manifest.root_name {
        return Err(BackupError::Path(format!(
            "restore destination must retain root name {} for the canonical X/X.adoc layout",
            verified.manifest.root_name
        )));
    }
    let destination_parent = destination_root
        .parent()
        .ok_or_else(|| BackupError::Path("restore destination has no parent".to_owned()))?;
    if destination_parent.starts_with(&verified.directory) {
        return Err(BackupError::Path(
            "restore destination must be disjoint from the snapshot".to_owned(),
        ));
    }
    let snapshot_binding = path_binding(&verified.directory)?;
    let destination_binding = new_path_binding(&destination_root)?;
    let plan_digest = restore_plan_digest(
        restore_id,
        &verified.manifest_sha256,
        &snapshot_binding,
        &destination_binding,
    );
    Ok(AlternateRestorePlan {
        schema: RESTORE_PLAN_SCHEMA.to_owned(),
        restore_id,
        plan_digest,
        snapshot_id: verified.snapshot_id,
        snapshot_directory: verified.directory,
        destination_root,
        workspace_root_id: verified.workspace_root_id,
        workspace_revision: verified.workspace_revision,
        manifest_sha256: verified.manifest_sha256,
        entries: verified.manifest.entries,
        entry_count: verified.manifest.entry_count,
        total_bytes: verified.manifest.total_bytes,
        root_name: verified.manifest.root_name,
        snapshot_workspace_root: verified.workspace_content_root,
        snapshot_binding,
        destination_binding,
    })
}

/// Restores a reviewed snapshot into a new alternate location and reopens it through Core.
///
/// # Errors
///
/// Rejects stale/tampered plans or snapshots, destination collisions, failed durability, invalid
/// restored Core identity/revision, and any byte mismatch.
pub fn commit_alternate_restore(
    plan: &AlternateRestorePlan,
) -> Result<AlternateRestoreReceipt, BackupError> {
    validate_restore_plan(plan)?;
    let current = replan_alternate_restore(
        &plan.snapshot_directory,
        &plan.destination_root,
        plan.restore_id,
    )?;
    if current.plan_digest != plan.plan_digest
        || current.snapshot_id != plan.snapshot_id
        || current.workspace_root_id != plan.workspace_root_id
        || current.workspace_revision != plan.workspace_revision
        || current.manifest_sha256 != plan.manifest_sha256
        || current.entries != plan.entries
        || current.entry_count != plan.entry_count
        || current.total_bytes != plan.total_bytes
    {
        return Err(BackupError::StalePreview);
    }
    commit_restore_contents(plan)
}

/// Builds a strictly read-only preview for restoring only one managed node from a verified full
/// snapshot into an existing workspace.
///
/// Child nodes are deliberately excluded. The selected node's canonical document, annotation
/// sidecar when present, and directly owned ordinary resources are included exactly. A physical
/// unmanaged/ignored/unowned entry at that node boundary rejects the preview.
///
/// # Errors
///
/// Rejects an invalid snapshot or target workspace, root selection, unknown identities, target
/// collisions, content boundaries, linked paths, concurrent changes, and unfinished Core work.
pub fn plan_single_node_restore(
    snapshot_directory: impl AsRef<Path>,
    target_workspace_root: impl AsRef<Path>,
    source_node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
) -> Result<ScopedRestorePlan, BackupError> {
    replan_scoped_restore(
        snapshot_directory,
        target_workspace_root,
        source_node_id,
        destination_parent_id,
        destination_name,
        ScopedRestoreScope::SingleNode,
        Uuid::new_v4(),
    )
}

/// Rebuilds one single-node restore preview for a caller-provided reviewed restore ID.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_single_node_restore`].
pub fn replan_single_node_restore(
    snapshot_directory: impl AsRef<Path>,
    target_workspace_root: impl AsRef<Path>,
    source_node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
    restore_id: Uuid,
) -> Result<ScopedRestorePlan, BackupError> {
    replan_scoped_restore(
        snapshot_directory,
        target_workspace_root,
        source_node_id,
        destination_parent_id,
        destination_name,
        ScopedRestoreScope::SingleNode,
        restore_id,
    )
}

/// Builds a strictly read-only preview for restoring a managed snapshot subtree into an existing
/// workspace while preserving every selected node identity.
///
/// # Errors
///
/// Rejects the same unsafe conditions as [`plan_single_node_restore`]. A preview can be valid yet
/// explicitly blocked when the selected tree exceeds Core's bounded exact-tree transaction safety
/// envelope; callers must inspect `commit_state` and `blockers`.
pub fn plan_subtree_restore(
    snapshot_directory: impl AsRef<Path>,
    target_workspace_root: impl AsRef<Path>,
    source_node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
) -> Result<ScopedRestorePlan, BackupError> {
    replan_scoped_restore(
        snapshot_directory,
        target_workspace_root,
        source_node_id,
        destination_parent_id,
        destination_name,
        ScopedRestoreScope::Subtree,
        Uuid::new_v4(),
    )
}

/// Rebuilds one subtree restore preview for a caller-provided reviewed restore ID.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_subtree_restore`].
pub fn replan_subtree_restore(
    snapshot_directory: impl AsRef<Path>,
    target_workspace_root: impl AsRef<Path>,
    source_node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
    restore_id: Uuid,
) -> Result<ScopedRestorePlan, BackupError> {
    replan_scoped_restore(
        snapshot_directory,
        target_workspace_root,
        source_node_id,
        destination_parent_id,
        destination_name,
        ScopedRestoreScope::Subtree,
        restore_id,
    )
}

/// Commits one exact, executable scoped restore through Core's recoverable workspace transaction.
///
/// A plan carrying typed blockers is never partially applied. Commit first re-verifies the full
/// snapshot, target revision, identities, destination, entry inventory, and digest, then lets Core
/// stage, journal, reopen, and verify the complete exact selected tree.
///
/// # Errors
///
/// Rejects tampered or stale plans, blocked tree shapes, snapshot/target changes, Core transaction
/// failures, or any post-commit identity/byte mismatch.
pub fn commit_scoped_restore(
    plan: &ScopedRestorePlan,
) -> Result<ScopedRestoreReceipt, BackupError> {
    validate_scoped_restore_plan(plan)?;
    let current_revision = read_workspace_revision(&plan.target_workspace_root)?;
    if current_revision != plan.target_workspace_revision {
        return Err(BackupError::StalePreview);
    }
    let current = replan_scoped_restore(
        &plan.snapshot_directory,
        &plan.target_workspace_root,
        plan.source_node_id,
        plan.destination_parent_id,
        &plan.destination_name,
        plan.scope,
        plan.restore_id,
    )?;
    if !same_scoped_restore_plan(plan, &current) {
        return Err(BackupError::StalePreview);
    }
    if !current.blockers.is_empty() {
        return Err(BackupError::ScopedRestoreBlocked(current.blockers));
    }

    let restore_nodes = scoped_restore_tree_nodes(&current)?;
    let authority = WorkspaceImportAuthority {
        proposal_id: format!("backup-scoped-restore-{}", current.restore_id.hyphenated()),
        proposal_digest: current.plan_digest.clone(),
    };
    let transaction = plan_restore_snapshot_tree(
        &current.target_workspace_root,
        &current.target_workspace_revision,
        authority,
        restore_nodes,
    )
    .map_err(BackupError::CoreTransaction)?;
    verify_snapshot_binding_for_scoped_restore(&current)?;
    let transaction =
        commit_workspace_transaction(&transaction).map_err(BackupError::CoreTransaction)?;
    verify_scoped_restore_outcome(&current, &transaction)?;
    Ok(ScopedRestoreReceipt {
        schema: SCOPED_RESTORE_RECEIPT_SCHEMA.to_owned(),
        restore_id: current.restore_id,
        snapshot_id: current.snapshot_id,
        scope: current.scope,
        source_node_id: current.source_node_id,
        destination_parent_id: current.destination_parent_id,
        destination_locator: current.destination_locator,
        restored_node_ids: current.nodes.iter().map(|node| node.node_id).collect(),
        entry_count: current.entry_count,
        total_bytes: current.total_bytes,
        transaction,
        exact_bytes_verified: true,
    })
}

/// Builds a read-only plan for a clean alternate restore plus a durable drill result record.
///
/// # Errors
///
/// Rejects incomplete snapshots, linked or overlapping locations, existing drill targets, and
/// invalid path bindings. Planning does not create the drill directory or result file.
pub fn plan_restore_drill(
    snapshot_directory: impl AsRef<Path>,
    drill_parent: impl AsRef<Path>,
    results_parent: impl AsRef<Path>,
) -> Result<RestoreDrillPlan, BackupError> {
    replan_restore_drill(
        snapshot_directory,
        drill_parent,
        results_parent,
        Uuid::new_v4(),
    )
}

/// Rebuilds a restore-drill preview for one exact reviewed drill identity.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_restore_drill`] and rejects a non-v4 ID.
pub fn replan_restore_drill(
    snapshot_directory: impl AsRef<Path>,
    drill_parent: impl AsRef<Path>,
    results_parent: impl AsRef<Path>,
    drill_id: Uuid,
) -> Result<RestoreDrillPlan, BackupError> {
    validate_v4(drill_id, "restore drill")?;
    let verified = verify_snapshot_internal(snapshot_directory.as_ref())?;
    let drill_parent = canonical_existing_directory(drill_parent.as_ref(), "drill parent")?;
    let results_parent =
        canonical_existing_directory(results_parent.as_ref(), "drill results parent")?;
    reject_workspace_marker_ancestor(&drill_parent, "restore drill parent")?;
    reject_workspace_marker_ancestor(&results_parent, "restore drill results parent")?;
    if drill_parent.starts_with(&verified.directory)
        || results_parent.starts_with(&verified.directory)
        || verified.directory.starts_with(&drill_parent)
        || verified.directory.starts_with(&results_parent)
    {
        return Err(BackupError::Path(
            "restore drill locations must be disjoint from the snapshot".to_owned(),
        ));
    }
    let drill_directory = drill_parent.join(format!(
        "{RESTORE_DRILL_DIRECTORY_PREFIX}{}",
        drill_id.hyphenated()
    ));
    reject_linked_existing_ancestors(&drill_directory)?;
    if drill_directory.exists() {
        return Err(BackupError::RestoreTargetExists(drill_directory));
    }
    let destination_root = drill_directory.join(&verified.manifest.root_name);
    let result_file = results_parent.join(format!(
        "{RESTORE_DRILL_DIRECTORY_PREFIX}{}.json",
        drill_id.hyphenated()
    ));
    reject_linked_existing_ancestors(&result_file)?;
    if result_file.exists() {
        return Err(BackupError::RestoreTargetExists(result_file));
    }
    let snapshot_binding = path_binding(&verified.directory)?;
    let drill_parent_binding = path_binding(&drill_parent)?;
    let results_parent_binding = path_binding(&results_parent)?;
    let plan_digest = restore_drill_plan_digest(
        drill_id,
        &verified.manifest_sha256,
        &snapshot_binding,
        &drill_parent_binding,
        &results_parent_binding,
    );
    Ok(RestoreDrillPlan {
        schema: RESTORE_DRILL_PLAN_SCHEMA.to_owned(),
        drill_id,
        plan_digest,
        snapshot_id: verified.snapshot_id,
        snapshot_directory: verified.directory,
        drill_parent,
        results_parent,
        drill_directory,
        destination_root,
        result_file,
        workspace_root_id: verified.workspace_root_id,
        workspace_revision: verified.workspace_revision,
        manifest_sha256: verified.manifest_sha256,
        entry_count: verified.manifest.entry_count,
        total_bytes: verified.manifest.total_bytes,
        root_name: verified.manifest.root_name,
        snapshot_binding,
        drill_parent_binding,
        results_parent_binding,
    })
}

/// Performs one reviewed clean restore drill and writes its success record only after Core and
/// bytewise verification both pass.
///
/// The verified alternate workspace is intentionally retained for operator inspection. A failed
/// drill may retain a partial create-new directory as evidence and never overwrites existing data.
///
/// # Errors
///
/// Rejects stale/tampered plans, changed snapshots, destination collisions, failed restore or
/// reopen verification, and result-record collisions.
pub fn commit_restore_drill(plan: &RestoreDrillPlan) -> Result<RestoreDrillReceipt, BackupError> {
    validate_restore_drill_plan(plan)?;
    let current = replan_restore_drill(
        &plan.snapshot_directory,
        &plan.drill_parent,
        &plan.results_parent,
        plan.drill_id,
    )?;
    if !same_restore_drill_plan(&current, plan) {
        return Err(BackupError::StalePreview);
    }
    fs::create_dir(&plan.drill_directory).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            BackupError::RestoreTargetExists(plan.drill_directory.clone())
        } else {
            BackupError::Io(error)
        }
    })?;
    let restore_plan = replan_alternate_restore(
        &plan.snapshot_directory,
        &plan.destination_root,
        plan.drill_id,
    )?;
    let restored = commit_alternate_restore(&restore_plan)?;
    if restored.snapshot_id != plan.snapshot_id
        || restored.workspace_root_id != plan.workspace_root_id
        || restored.workspace_revision != plan.workspace_revision
        || restored.manifest_sha256 != plan.manifest_sha256
        || restored.entry_count != plan.entry_count
        || restored.total_bytes != plan.total_bytes
        || !restored.bytewise_verified
    {
        return Err(BackupError::Verification(
            "restore drill result differs from its reviewed snapshot".to_owned(),
        ));
    }
    let result = RestoreDrillResult {
        schema: RESTORE_DRILL_RESULT_SCHEMA.to_owned(),
        drill_id: plan.drill_id,
        snapshot_id: plan.snapshot_id,
        completed_at_unix_ms: unix_time_ms()?,
        workspace_root_id: plan.workspace_root_id,
        workspace_revision: plan.workspace_revision.clone(),
        manifest_sha256: plan.manifest_sha256.clone(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
        opened_clean: true,
        bytewise_verified: true,
    };
    let mut bytes = serde_json::to_vec_pretty(&result).map_err(BackupError::Json)?;
    bytes.push(b'\n');
    write_new_file(&plan.result_file, &bytes)?;
    let recorded = read_restore_drill_result(&plan.result_file)?;
    if recorded != result {
        return Err(BackupError::Verification(
            "restore drill result record failed reopen verification".to_owned(),
        ));
    }
    sync_directory(&plan.drill_parent)?;
    sync_directory(&plan.results_parent)?;
    Ok(RestoreDrillReceipt {
        schema: "weftext.full-workspace-restore-drill-receipt.v1".to_owned(),
        drill_id: plan.drill_id,
        snapshot_id: plan.snapshot_id,
        destination_root: restored.destination_root,
        result_file: plan.result_file.clone(),
        workspace_root_id: plan.workspace_root_id,
        workspace_revision: plan.workspace_revision.clone(),
        manifest_sha256: plan.manifest_sha256.clone(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
        opened_clean: true,
        bytewise_verified: true,
    })
}

/// Reads and validates a bounded successful restore-drill record.
///
/// # Errors
///
/// Rejects linked, oversized, unknown-field, non-v4, malformed revision, and false-result records.
pub fn read_restore_drill_result(
    result_file: impl AsRef<Path>,
) -> Result<RestoreDrillResult, BackupError> {
    let result_file = result_file.as_ref();
    let parent = result_file
        .parent()
        .ok_or_else(|| BackupError::Path("restore drill result has no parent".to_owned()))?;
    let parent = canonical_existing_directory(parent, "restore drill results parent")?;
    let bytes = read_bounded_regular_file(result_file, &parent, 1024 * 1024)?;
    let result: RestoreDrillResult = serde_json::from_slice(&bytes).map_err(BackupError::Json)?;
    if result.schema != RESTORE_DRILL_RESULT_SCHEMA
        || !result.opened_clean
        || !result.bytewise_verified
    {
        return Err(BackupError::Verification(
            "restore drill record is not a successful clean restore".to_owned(),
        ));
    }
    validate_v4(result.drill_id, "restore drill")?;
    validate_v4(result.snapshot_id, "snapshot")?;
    validate_sha256(&result.manifest_sha256)?;
    WorkspaceRevision::parse(&result.workspace_revision.to_string())?;
    Ok(result)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn replan_scoped_restore(
    snapshot_directory: impl AsRef<Path>,
    target_workspace_root: impl AsRef<Path>,
    source_node_id: NodeId,
    destination_parent_id: NodeId,
    destination_name: &str,
    scope: ScopedRestoreScope,
    restore_id: Uuid,
) -> Result<ScopedRestorePlan, BackupError> {
    validate_v4(restore_id, "scoped restore")?;
    let verified = verify_snapshot_internal(snapshot_directory.as_ref())?;
    let target_workspace_root =
        canonical_existing_directory(target_workspace_root.as_ref(), "target workspace root")?;
    if target_workspace_root.starts_with(&verified.directory)
        || verified.directory.starts_with(&target_workspace_root)
    {
        return Err(BackupError::Path(
            "scoped restore target and snapshot must be disjoint".to_owned(),
        ));
    }

    let source_inventory = scan_workspace(&verified.workspace_content_root);
    if !source_inventory.is_valid() {
        return Err(BackupError::InvalidWorkspace(
            "verified snapshot source inventory became invalid".to_owned(),
        ));
    }
    let selected = source_inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(source_node_id))
        .cloned()
        .ok_or(BackupError::UnknownSnapshotNode(source_node_id))?;
    if selected.parent_id.is_none() {
        return Err(BackupError::ScopedRestoreRootUnsupported(source_node_id));
    }
    let mut selected_nodes = match scope {
        ScopedRestoreScope::SingleNode => vec![selected.clone()],
        ScopedRestoreScope::Subtree => source_inventory
            .nodes
            .iter()
            .filter(|node| node.path.starts_with(&selected.path))
            .cloned()
            .collect::<Vec<_>>(),
    };
    selected_nodes.sort_by(|left, right| left.path.cmp(&right.path));

    let create_probe = plan_create_child_node(
        &target_workspace_root,
        destination_parent_id,
        destination_name,
    )
    .map_err(BackupError::CoreTransaction)?;
    let destination_locator = create_probe
        .path_changes
        .first()
        .map(|change| change.new_path.clone())
        .ok_or_else(|| {
            BackupError::Verification(
                "Core create preview did not expose one destination".to_owned(),
            )
        })?;
    let target_inventory = scan_workspace(&target_workspace_root);
    if !target_inventory.is_valid() {
        return Err(BackupError::InvalidWorkspace(
            "target workspace inventory is invalid".to_owned(),
        ));
    }
    let selected_ids = selected_nodes
        .iter()
        .filter_map(|node| node.id)
        .collect::<BTreeSet<_>>();
    if let Some(conflict) = target_inventory
        .nodes
        .iter()
        .filter_map(|node| node.id)
        .find(|node_id| selected_ids.contains(node_id))
    {
        return Err(BackupError::RestoreIdentityConflict(conflict));
    }
    let target_workspace_root_id = workspace_root_id(&target_workspace_root)?;
    let target_workspace_revision = read_workspace_revision(&target_workspace_root)?;
    if target_workspace_revision != create_probe.base_revision {
        return Err(BackupError::ConcurrentChange(target_workspace_root));
    }

    let (nodes, entries) = scoped_restore_inventory(
        &verified,
        &source_inventory,
        &selected,
        &selected_nodes,
        &destination_locator,
        destination_name,
        scope,
    )?;
    let entry_count = u64::try_from(entries.len()).unwrap_or(u64::MAX);
    let total_bytes = entries
        .iter()
        .filter(|entry| entry.entry_type == BackupEntryType::File)
        .try_fold(0_u64, |total, entry| {
            total.checked_add(entry.length).ok_or_else(|| {
                BackupError::Verification("scoped restore byte count overflowed".to_owned())
            })
        })?;
    let mut blockers = scoped_restore_capability_blockers(&nodes, &entries, total_bytes);
    if blockers.is_empty() {
        let probe_digest = scoped_restore_entries_digest(&nodes, &entries);
        let restore_nodes = scoped_restore_tree_nodes_from_entries(
            &verified.workspace_content_root,
            &nodes,
            &entries,
        )?;
        let authority = WorkspaceImportAuthority {
            proposal_id: format!("backup-scoped-probe-{}", restore_id.hyphenated()),
            proposal_digest: probe_digest,
        };
        match plan_restore_snapshot_tree(
            &target_workspace_root,
            &target_workspace_revision,
            authority,
            restore_nodes,
        ) {
            Ok(_) => {}
            Err(
                error @ (WorkspaceTransactionError::Metadata(_)
                | WorkspaceTransactionError::VerificationFailed(_)),
            ) => blockers.push(ScopedRestoreBlocker {
                code: ScopedRestoreBlockerCode::CoreExactTreeCreateUnavailable,
                message: format!(
                    "Core cannot create this exact snapshot-selected node tree: {error}"
                ),
            }),
            Err(error) => return Err(BackupError::CoreTransaction(error)),
        }
    }
    blockers.sort();
    blockers.dedup();
    let commit_state = if blockers.is_empty() {
        ScopedRestoreCommitState::Ready
    } else {
        ScopedRestoreCommitState::Blocked
    };
    let snapshot_binding = path_binding(&verified.directory)?;
    let target_binding = path_binding(&target_workspace_root)?;
    let mut plan = ScopedRestorePlan {
        schema: SCOPED_RESTORE_PLAN_SCHEMA.to_owned(),
        restore_id,
        plan_digest: String::new(),
        scope,
        snapshot_id: verified.snapshot_id,
        snapshot_directory: verified.directory.clone(),
        snapshot_manifest_sha256: verified.manifest_sha256.clone(),
        source_workspace_root_id: verified.workspace_root_id,
        source_workspace_revision: verified.workspace_revision.clone(),
        source_node_id,
        target_workspace_root,
        target_workspace_root_id,
        target_workspace_revision,
        destination_parent_id,
        destination_name: destination_name.to_owned(),
        destination_locator,
        nodes,
        entries,
        entry_count,
        total_bytes,
        commit_state,
        blockers,
        snapshot_workspace_root: verified.workspace_content_root.clone(),
        snapshot_binding,
        target_binding,
    };
    plan.plan_digest = scoped_restore_plan_digest(&plan)?;

    let reopened = verify_snapshot_internal(&plan.snapshot_directory)?;
    if reopened.snapshot_id != plan.snapshot_id
        || reopened.manifest_sha256 != plan.snapshot_manifest_sha256
        || reopened.workspace_revision != plan.source_workspace_revision
        || reopened.workspace_content_root != plan.snapshot_workspace_root
    {
        return Err(BackupError::ConcurrentChange(plan.snapshot_directory));
    }
    if read_workspace_revision(&plan.target_workspace_root)? != plan.target_workspace_revision {
        return Err(BackupError::ConcurrentChange(plan.target_workspace_root));
    }
    Ok(plan)
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn scoped_restore_inventory(
    verified: &VerifiedSnapshot,
    inventory: &weftext_core::WorkspaceInventory,
    selected: &weftext_core::NodeRecord,
    selected_nodes: &[weftext_core::NodeRecord],
    destination_locator: &str,
    destination_name: &str,
    scope: ScopedRestoreScope,
) -> Result<(Vec<ScopedRestoreNode>, Vec<ScopedRestoreEntry>), BackupError> {
    let source_root = &verified.workspace_content_root;
    let selected_locator = portable_locator(source_root, &selected.path)?;
    let manifest_entries = verified
        .manifest
        .entries
        .iter()
        .map(|entry| (entry.locator.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let selected_ids = selected_nodes
        .iter()
        .filter_map(|node| node.id)
        .collect::<BTreeSet<_>>();

    for content in &inventory.content {
        let within_scope = match scope {
            ScopedRestoreScope::SingleNode => {
                content.parent_relative_path.as_deref() == Some(selected_locator.as_str())
            }
            ScopedRestoreScope::Subtree => {
                content.relative_path == selected_locator
                    || content
                        .relative_path
                        .starts_with(&format!("{selected_locator}/"))
            }
        };
        if within_scope
            && matches!(
                content.kind,
                WorkspaceContentKind::UnmanagedDirectory | WorkspaceContentKind::UnmanagedMarkdown
            )
        {
            return Err(BackupError::ScopedRestoreBoundary(
                content.relative_path.clone(),
            ));
        }
    }

    let mut nodes = Vec::new();
    let mut entries = Vec::new();
    let mut allowed_source_locators = BTreeSet::new();
    for node in selected_nodes {
        let node_id = node.id.ok_or_else(|| {
            BackupError::InvalidWorkspace("selected snapshot node has no identity".to_owned())
        })?;
        let source_node_locator = portable_locator(source_root, &node.path)?;
        let destination_node_locator = if node.path == selected.path {
            destination_locator.to_owned()
        } else {
            let suffix = portable_locator(&selected.path, &node.path)?;
            join_portable_locator(destination_locator, &suffix)
        };
        nodes.push(ScopedRestoreNode {
            node_id,
            source_locator: source_node_locator.clone(),
            destination_locator: destination_node_locator.clone(),
        });
        push_scoped_entry(
            &mut entries,
            &mut allowed_source_locators,
            node_id,
            ScopedRestoreEntryKind::NodeDirectory,
            &source_node_locator,
            &destination_node_locator,
            manifest_entries.get(source_node_locator.as_str()).copied(),
        )?;

        let source_document_locator = portable_locator(source_root, &node.document_path)?;
        let destination_document_name = if node.path == selected.path {
            format!("{destination_name}.adoc")
        } else {
            format!("{}.adoc", node.name)
        };
        let destination_document_locator =
            join_portable_locator(&destination_node_locator, &destination_document_name);
        push_scoped_entry(
            &mut entries,
            &mut allowed_source_locators,
            node_id,
            ScopedRestoreEntryKind::CanonicalDocument,
            &source_document_locator,
            &destination_document_locator,
            manifest_entries
                .get(source_document_locator.as_str())
                .copied(),
        )?;

        let sidecar_locator = join_portable_locator(&source_node_locator, ANNOTATIONS_FILE_NAME);
        if let Some(sidecar) = manifest_entries.get(sidecar_locator.as_str()).copied() {
            read_node_annotations(
                source_root,
                node_id,
                AnnotationReplicaCompleteness::CompleteLocalWorkspace,
            )
            .map_err(BackupError::CoreTransaction)?;
            let destination_sidecar =
                join_portable_locator(&destination_node_locator, ANNOTATIONS_FILE_NAME);
            push_scoped_entry(
                &mut entries,
                &mut allowed_source_locators,
                node_id,
                ScopedRestoreEntryKind::AnnotationSidecar,
                &sidecar_locator,
                &destination_sidecar,
                Some(sidecar),
            )?;
        }

        for resource in inventory.content.iter().filter(|content| {
            content.kind == WorkspaceContentKind::Resource
                && content.owner_node_id == Some(node_id)
                && content.relative_path != sidecar_locator
        }) {
            let resource_name = resource
                .relative_path
                .strip_prefix(&format!("{source_node_locator}/"))
                .filter(|relative| !relative.is_empty() && !relative.contains('/'))
                .ok_or_else(|| {
                    BackupError::ScopedRestoreBoundary(resource.relative_path.clone())
                })?;
            let destination_resource =
                join_portable_locator(&destination_node_locator, resource_name);
            push_scoped_entry(
                &mut entries,
                &mut allowed_source_locators,
                node_id,
                ScopedRestoreEntryKind::Resource,
                &resource.relative_path,
                &destination_resource,
                manifest_entries
                    .get(resource.relative_path.as_str())
                    .copied(),
            )?;
        }
    }

    let direct_child_nodes = inventory
        .nodes
        .iter()
        .filter(|node| node.parent_id == Some(selected.id.expect("selected node identity")))
        .map(|node| portable_locator(source_root, &node.path))
        .collect::<Result<BTreeSet<_>, _>>()?;
    for entry in &verified.manifest.entries {
        let selected_physical_entry = match scope {
            ScopedRestoreScope::SingleNode => {
                portable_parent(&entry.locator) == Some(selected_locator.as_str())
            }
            ScopedRestoreScope::Subtree => {
                entry.locator == selected_locator
                    || entry.locator.starts_with(&format!("{selected_locator}/"))
            }
        };
        if selected_physical_entry
            && !allowed_source_locators.contains(&entry.locator)
            && !(scope == ScopedRestoreScope::SingleNode
                && entry.entry_type == BackupEntryType::Directory
                && direct_child_nodes.contains(&entry.locator))
        {
            return Err(BackupError::ScopedRestoreBoundary(entry.locator.clone()));
        }
    }
    if entries
        .iter()
        .any(|entry| !selected_ids.contains(&entry.owner_node_id))
    {
        return Err(BackupError::Verification(
            "scoped restore entry owner is outside the selected identities".to_owned(),
        ));
    }
    nodes.sort();
    entries.sort();
    Ok((nodes, entries))
}

#[allow(clippy::too_many_arguments)]
fn push_scoped_entry(
    entries: &mut Vec<ScopedRestoreEntry>,
    allowed_source_locators: &mut BTreeSet<String>,
    owner_node_id: NodeId,
    kind: ScopedRestoreEntryKind,
    source_locator: &str,
    destination_locator: &str,
    source: Option<&BackupEntry>,
) -> Result<(), BackupError> {
    let source = source.ok_or_else(|| {
        BackupError::Verification(format!(
            "snapshot manifest is missing selected entry {source_locator}"
        ))
    })?;
    let expected_type = if kind == ScopedRestoreEntryKind::NodeDirectory {
        BackupEntryType::Directory
    } else {
        BackupEntryType::File
    };
    if source.entry_type != expected_type || !allowed_source_locators.insert(source_locator.into())
    {
        return Err(BackupError::Verification(format!(
            "snapshot selected entry has an invalid type or duplicate locator: {source_locator}"
        )));
    }
    entries.push(ScopedRestoreEntry {
        owner_node_id,
        kind,
        source_locator: source_locator.to_owned(),
        destination_locator: destination_locator.to_owned(),
        entry_type: source.entry_type,
        length: source.length,
        sha256: source.sha256.clone(),
    });
    Ok(())
}

fn scoped_restore_capability_blockers(
    nodes: &[ScopedRestoreNode],
    entries: &[ScopedRestoreEntry],
    total_bytes: u64,
) -> Vec<ScopedRestoreBlocker> {
    let mut blockers = Vec::new();
    let document_too_large = entries.iter().any(|entry| {
        entry.kind == ScopedRestoreEntryKind::CanonicalDocument && entry.length > 32 * 1024 * 1024
    });
    let resources = entries
        .iter()
        .filter(|entry| entry.kind == ScopedRestoreEntryKind::Resource)
        .collect::<Vec<_>>();
    let resource_too_large = resources
        .iter()
        .any(|entry| entry.length > 64 * 1024 * 1024);
    if document_too_large
        || resource_too_large
        || nodes.len() > 10_000
        || entries.len() > 100_000
        || total_bytes > 2 * 1024 * 1024 * 1024
    {
        blockers.push(ScopedRestoreBlocker {
            code: ScopedRestoreBlockerCode::CoreImportSafetyEnvelopeExceeded,
            message:
                "the exact selected bytes exceed Core's bounded import transaction safety envelope"
                    .to_owned(),
        });
    }
    blockers
}

fn scoped_restore_tree_nodes(
    plan: &ScopedRestorePlan,
) -> Result<Vec<WorkspaceRestoreTreeNode>, BackupError> {
    scoped_restore_tree_nodes_from_entries(
        &plan.snapshot_workspace_root,
        &plan.nodes,
        &plan.entries,
    )
}

fn scoped_restore_tree_nodes_from_entries(
    snapshot_workspace_root: &Path,
    nodes: &[ScopedRestoreNode],
    entries: &[ScopedRestoreEntry],
) -> Result<Vec<WorkspaceRestoreTreeNode>, BackupError> {
    let mut restored = Vec::with_capacity(nodes.len());
    for node in nodes {
        let document = entries
            .iter()
            .find(|entry| {
                entry.owner_node_id == node.node_id
                    && entry.kind == ScopedRestoreEntryKind::CanonicalDocument
            })
            .ok_or_else(|| {
                BackupError::Verification(format!(
                    "scoped restore node has no canonical document: {}",
                    node.node_id
                ))
            })?;
        let document_bytes = read_scoped_snapshot_entry(snapshot_workspace_root, document)?;
        let exact_source = String::from_utf8(document_bytes).map_err(|_| {
            BackupError::Verification(format!(
                "scoped restore document is not UTF-8: {}",
                node.node_id
            ))
        })?;
        let document_file =
            portable_child_name(&node.destination_locator, &document.destination_locator)?;
        let annotation_sidecar = entries
            .iter()
            .find(|entry| {
                entry.owner_node_id == node.node_id
                    && entry.kind == ScopedRestoreEntryKind::AnnotationSidecar
            })
            .map(
                |entry| -> Result<WorkspaceRestoreAnnotationSidecar, BackupError> {
                    Ok(WorkspaceRestoreAnnotationSidecar {
                        bytes: read_scoped_snapshot_entry(snapshot_workspace_root, entry)?,
                        sha256: entry.sha256.clone(),
                    })
                },
            )
            .transpose()?;
        let mut resources = Vec::new();
        for entry in entries.iter().filter(|entry| {
            entry.owner_node_id == node.node_id && entry.kind == ScopedRestoreEntryKind::Resource
        }) {
            let locator =
                portable_child_name(&node.destination_locator, &entry.destination_locator)?;
            resources.push(WorkspaceImportResource {
                locator,
                bytes: read_scoped_snapshot_entry(snapshot_workspace_root, entry)?,
                sha256: entry.sha256.clone(),
            });
        }
        restored.push(WorkspaceRestoreTreeNode {
            locator: node.destination_locator.clone(),
            node_id: node.node_id,
            document_file,
            exact_source,
            document_sha256: document.sha256.clone(),
            annotation_sidecar,
            resources,
        });
    }
    Ok(restored)
}

fn read_scoped_snapshot_entry(
    snapshot_workspace_root: &Path,
    entry: &ScopedRestoreEntry,
) -> Result<Vec<u8>, BackupError> {
    if entry.entry_type != BackupEntryType::File {
        return Err(BackupError::Verification(
            "attempted to read a scoped restore directory as a file".to_owned(),
        ));
    }
    let path = safe_join(snapshot_workspace_root, &entry.source_locator)?;
    let bytes = read_bounded_regular_file(
        &path,
        snapshot_workspace_root,
        entry.length.saturating_add(1),
    )?;
    if bytes.len() as u64 != entry.length || sha256(&bytes) != entry.sha256 {
        return Err(BackupError::ConcurrentChange(path));
    }
    Ok(bytes)
}

fn verify_snapshot_binding_for_scoped_restore(plan: &ScopedRestorePlan) -> Result<(), BackupError> {
    let verified = verify_snapshot_internal(&plan.snapshot_directory)?;
    if verified.snapshot_id != plan.snapshot_id
        || verified.manifest_sha256 != plan.snapshot_manifest_sha256
        || verified.workspace_root_id != plan.source_workspace_root_id
        || verified.workspace_revision != plan.source_workspace_revision
        || verified.workspace_content_root != plan.snapshot_workspace_root
        || path_binding(&verified.directory)? != plan.snapshot_binding
    {
        return Err(BackupError::StalePreview);
    }
    Ok(())
}

fn verify_scoped_restore_outcome(
    plan: &ScopedRestorePlan,
    transaction: &CommittedWorkspaceTransaction,
) -> Result<(), BackupError> {
    if transaction.base_revision != plan.target_workspace_revision
        || read_workspace_revision(&plan.target_workspace_root)? != transaction.revision
    {
        return Err(BackupError::Verification(
            "Core scoped restore receipt does not bind the target revisions".to_owned(),
        ));
    }
    let inventory = scan_workspace(&plan.target_workspace_root);
    if !inventory.is_valid() {
        return Err(BackupError::InvalidWorkspace(
            "target became invalid after scoped restore".to_owned(),
        ));
    }
    for node in &plan.nodes {
        let expected_path = safe_join(&plan.target_workspace_root, &node.destination_locator)?;
        if !inventory
            .nodes
            .iter()
            .any(|candidate| candidate.id == Some(node.node_id) && candidate.path == expected_path)
        {
            return Err(BackupError::Verification(format!(
                "restored node identity or locator differs: {}",
                node.node_id
            )));
        }
    }
    for entry in &plan.entries {
        let destination = safe_join(&plan.target_workspace_root, &entry.destination_locator)?;
        let metadata = fs::symlink_metadata(&destination).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(destination));
        }
        match entry.entry_type {
            BackupEntryType::Directory => {
                if !metadata.is_dir() {
                    return Err(BackupError::Verification(format!(
                        "restored directory is missing: {}",
                        entry.destination_locator
                    )));
                }
            }
            BackupEntryType::File if metadata.is_file() => {
                let (length, digest) =
                    digest_regular_file(&destination, &plan.target_workspace_root)?;
                if length != entry.length || digest != entry.sha256 {
                    return Err(BackupError::Verification(format!(
                        "restored bytes differ: {}",
                        entry.destination_locator
                    )));
                }
            }
            BackupEntryType::File => {
                return Err(BackupError::Verification(format!(
                    "restored entry type differs: {}",
                    entry.destination_locator
                )));
            }
        }
    }
    verify_snapshot_binding_for_scoped_restore(plan)
}

fn validate_scoped_restore_plan(plan: &ScopedRestorePlan) -> Result<(), BackupError> {
    if plan.schema != SCOPED_RESTORE_PLAN_SCHEMA {
        return Err(BackupError::InvalidPlan);
    }
    validate_v4(plan.restore_id, "scoped restore")?;
    validate_v4(plan.snapshot_id, "snapshot")?;
    validate_sha256(&plan.snapshot_manifest_sha256)?;
    validate_sha256(&plan.plan_digest)?;
    let snapshot = canonical_existing_directory(&plan.snapshot_directory, "snapshot directory")?;
    let target =
        canonical_existing_directory(&plan.target_workspace_root, "target workspace root")?;
    if snapshot != plan.snapshot_directory
        || target != plan.target_workspace_root
        || path_binding(&snapshot)? != plan.snapshot_binding
        || path_binding(&target)? != plan.target_binding
        || plan.entry_count != plan.entries.len() as u64
        || plan.total_bytes
            != plan
                .entries
                .iter()
                .filter(|entry| entry.entry_type == BackupEntryType::File)
                .try_fold(0_u64, |total, entry| total.checked_add(entry.length))
                .ok_or(BackupError::InvalidPlan)?
        || (plan.blockers.is_empty() != (plan.commit_state == ScopedRestoreCommitState::Ready))
        || scoped_restore_plan_digest(plan)? != plan.plan_digest
    {
        return Err(BackupError::InvalidPlan);
    }
    if !plan.nodes.windows(2).all(|pair| pair[0] < pair[1])
        || !plan.entries.windows(2).all(|pair| pair[0] < pair[1])
        || !plan.blockers.windows(2).all(|pair| pair[0] < pair[1])
    {
        return Err(BackupError::InvalidPlan);
    }
    for entry in &plan.entries {
        validate_locator(&entry.source_locator)?;
        validate_locator(&entry.destination_locator)?;
        validate_sha256(&entry.sha256)?;
        if entry.entry_type == BackupEntryType::Directory
            && (entry.length != 0
                || entry.kind != ScopedRestoreEntryKind::NodeDirectory
                || entry.sha256 != directory_sha256(&entry.source_locator))
        {
            return Err(BackupError::InvalidPlan);
        }
    }
    Ok(())
}

fn same_scoped_restore_plan(left: &ScopedRestorePlan, right: &ScopedRestorePlan) -> bool {
    left.schema == right.schema
        && left.restore_id == right.restore_id
        && left.plan_digest == right.plan_digest
        && left.scope == right.scope
        && left.snapshot_id == right.snapshot_id
        && left.snapshot_directory == right.snapshot_directory
        && left.snapshot_manifest_sha256 == right.snapshot_manifest_sha256
        && left.source_workspace_root_id == right.source_workspace_root_id
        && left.source_workspace_revision == right.source_workspace_revision
        && left.source_node_id == right.source_node_id
        && left.target_workspace_root == right.target_workspace_root
        && left.target_workspace_root_id == right.target_workspace_root_id
        && left.target_workspace_revision == right.target_workspace_revision
        && left.destination_parent_id == right.destination_parent_id
        && left.destination_name == right.destination_name
        && left.destination_locator == right.destination_locator
        && left.nodes == right.nodes
        && left.entries == right.entries
        && left.entry_count == right.entry_count
        && left.total_bytes == right.total_bytes
        && left.commit_state == right.commit_state
        && left.blockers == right.blockers
        && left.snapshot_workspace_root == right.snapshot_workspace_root
        && left.snapshot_binding == right.snapshot_binding
        && left.target_binding == right.target_binding
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScopedRestoreDigestRecord<'a> {
    schema: &'a str,
    restore_id: Uuid,
    scope: ScopedRestoreScope,
    snapshot_id: Uuid,
    snapshot_manifest_sha256: &'a str,
    snapshot_binding: &'a str,
    source_workspace_root_id: NodeId,
    source_workspace_revision: &'a WorkspaceRevision,
    source_node_id: NodeId,
    target_binding: &'a str,
    target_workspace_root_id: NodeId,
    target_workspace_revision: &'a WorkspaceRevision,
    destination_parent_id: NodeId,
    destination_name: &'a str,
    destination_locator: &'a str,
    nodes: &'a [ScopedRestoreNode],
    entries: &'a [ScopedRestoreEntry],
    commit_state: ScopedRestoreCommitState,
    blockers: &'a [ScopedRestoreBlocker],
}

fn scoped_restore_plan_digest(plan: &ScopedRestorePlan) -> Result<String, BackupError> {
    let record = ScopedRestoreDigestRecord {
        schema: &plan.schema,
        restore_id: plan.restore_id,
        scope: plan.scope,
        snapshot_id: plan.snapshot_id,
        snapshot_manifest_sha256: &plan.snapshot_manifest_sha256,
        snapshot_binding: &plan.snapshot_binding,
        source_workspace_root_id: plan.source_workspace_root_id,
        source_workspace_revision: &plan.source_workspace_revision,
        source_node_id: plan.source_node_id,
        target_binding: &plan.target_binding,
        target_workspace_root_id: plan.target_workspace_root_id,
        target_workspace_revision: &plan.target_workspace_revision,
        destination_parent_id: plan.destination_parent_id,
        destination_name: &plan.destination_name,
        destination_locator: &plan.destination_locator,
        nodes: &plan.nodes,
        entries: &plan.entries,
        commit_state: plan.commit_state,
        blockers: &plan.blockers,
    };
    serde_json::to_vec(&record)
        .map(|bytes| sha256(&bytes))
        .map_err(BackupError::Json)
}

fn scoped_restore_entries_digest(
    nodes: &[ScopedRestoreNode],
    entries: &[ScopedRestoreEntry],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.scoped-workspace-restore-probe.v1\0");
    for node in nodes {
        hasher.update(node.node_id.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(node.source_locator.as_bytes());
        hasher.update([0]);
        hasher.update(node.destination_locator.as_bytes());
        hasher.update([0]);
    }
    for entry in entries {
        hasher.update(entry.source_locator.as_bytes());
        hasher.update([0]);
        hasher.update(entry.destination_locator.as_bytes());
        hasher.update([0]);
        hasher.update(entry.sha256.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

fn join_portable_locator(parent: &str, child: &str) -> String {
    format!("{parent}/{child}")
}

fn portable_parent(locator: &str) -> Option<&str> {
    locator.rsplit_once('/').map(|(parent, _)| parent)
}

fn portable_child_name(parent: &str, child: &str) -> Result<String, BackupError> {
    child
        .strip_prefix(&format!("{parent}/"))
        .filter(|relative| !relative.is_empty() && !relative.contains('/'))
        .map(str::to_owned)
        .ok_or(BackupError::InvalidPlan)
}

fn stable_workspace_inventory(
    workspace_root: &Path,
    workspace_lease: &WorkspaceTransactionLease,
) -> Result<(NodeId, WorkspaceRevision, String, Vec<BackupEntry>), BackupError> {
    stable_workspace_inventory_with_post_capture_probe(workspace_root, workspace_lease, || Ok(()))
}

fn stable_workspace_inventory_with_post_capture_probe(
    workspace_root: &Path,
    workspace_lease: &WorkspaceTransactionLease,
    post_capture_probe: impl FnOnce() -> Result<(), BackupError>,
) -> Result<(NodeId, WorkspaceRevision, String, Vec<BackupEntry>), BackupError> {
    let first_id = workspace_root_id(workspace_root)?;
    let first_revision = read_workspace_revision(workspace_root)?;
    let entries = backup_entries(
        &capture_stable_workspace_physical_inventory(workspace_lease)
            .map_err(|error| map_physical_inventory_error(workspace_root, error))?,
    );
    post_capture_probe()?;
    let second_id = workspace_root_id(workspace_root)?;
    let second_revision = read_workspace_revision(workspace_root)?;
    if first_id != second_id || first_revision != second_revision {
        return Err(BackupError::ConcurrentChange(workspace_root.to_path_buf()));
    }
    let root_name = workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BackupError::Path("workspace root name must be UTF-8".to_owned()))?
        .to_owned();
    workspace_lease
        .validate_anchor_identity()
        .map_err(BackupError::CoreTransaction)?;
    Ok((first_id, first_revision, root_name, entries))
}

fn workspace_root_id(workspace_root: &Path) -> Result<NodeId, BackupError> {
    let inventory = scan_workspace(workspace_root);
    if !inventory.is_valid() {
        let detail = inventory.issues.first().map_or_else(
            || "workspace has no root node".to_owned(),
            |issue| format!("{:?}: {}", issue.code, issue.message),
        );
        return Err(BackupError::InvalidWorkspace(detail));
    }
    let mut roots = inventory
        .nodes
        .iter()
        .filter(|node| node.parent_id.is_none());
    let root = roots
        .next()
        .ok_or_else(|| BackupError::InvalidWorkspace("workspace has no root node".to_owned()))?;
    if roots.next().is_some() || root.path != workspace_root {
        return Err(BackupError::InvalidWorkspace(
            "workspace root identity is ambiguous".to_owned(),
        ));
    }
    root.id
        .ok_or_else(|| BackupError::InvalidWorkspace("workspace root has no identity".to_owned()))
}

fn collect_stable_physical_entries(root: &Path) -> Result<Vec<BackupEntry>, BackupError> {
    capture_stable_physical_tree(root)
        .map(|inventory| backup_entries(&inventory))
        .map_err(|error| map_physical_inventory_error(root, error))
}

fn collect_workspace_physical_entries(
    workspace_root: &Path,
    workspace_lease: &WorkspaceTransactionLease,
) -> Result<Vec<BackupEntry>, BackupError> {
    capture_stable_workspace_physical_inventory(workspace_lease)
        .map(|inventory| backup_entries(&inventory))
        .map_err(|error| map_physical_inventory_error(workspace_root, error))
}

fn backup_entries(inventory: &PhysicalTreeInventory) -> Vec<BackupEntry> {
    inventory
        .entries()
        .iter()
        .map(|entry| {
            let locator = entry.locator().as_str().to_owned();
            match entry.kind() {
                PhysicalEntryKind::Directory => BackupEntry {
                    sha256: directory_sha256(&locator),
                    locator,
                    entry_type: BackupEntryType::Directory,
                    length: 0,
                },
                PhysicalEntryKind::RegularFile => BackupEntry {
                    sha256: entry
                        .sha256()
                        .expect("Core regular-file inventory entry has a SHA-256")
                        .to_hex(),
                    locator,
                    entry_type: BackupEntryType::File,
                    length: entry.byte_length(),
                },
            }
        })
        .collect()
}

fn map_physical_inventory_error(root: &Path, error: PhysicalInventoryError) -> BackupError {
    let entry_path = |locator: Option<&weftext_core::PhysicalLocator>| {
        locator.map_or_else(
            || root.to_path_buf(),
            |locator| {
                locator
                    .as_str()
                    .split('/')
                    .fold(root.to_path_buf(), |path, component| path.join(component))
            },
        )
    };
    match error {
        PhysicalInventoryError::LinkedOrReparse(locator)
        | PhysicalInventoryError::DirectoryIdentityAlias(locator) => {
            BackupError::LinkedPath(entry_path(locator.as_ref()))
        }
        PhysicalInventoryError::PathEscape(locator) => {
            BackupError::PathEscape(entry_path(locator.as_ref()))
        }
        PhysicalInventoryError::NonUtf8Path => BackupError::NonUtf8Path(root.to_path_buf()),
        PhysicalInventoryError::UnsupportedEntry(locator) => {
            BackupError::UnsupportedEntry(entry_path(locator.as_ref()))
        }
        PhysicalInventoryError::UnfinishedTransaction(locator) => {
            BackupError::UnfinishedTransaction(entry_path(Some(&locator)))
        }
        PhysicalInventoryError::LeaseAnchorMismatch => BackupError::UnfinishedTransaction(
            root.join(weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
        ),
        PhysicalInventoryError::ConcurrentChange => {
            BackupError::ConcurrentChange(root.to_path_buf())
        }
        PhysicalInventoryError::IdentityUnavailable { source, .. }
        | PhysicalInventoryError::Io { source, .. } => BackupError::Io(source),
        PhysicalInventoryError::RootNotDirectory
        | PhysicalInventoryError::IdentityUnsupported
        | PhysicalInventoryError::EntryLimitExceeded
        | PhysicalInventoryError::LocatorLimitExceeded
        | PhysicalInventoryError::FileByteCountOverflow
        | PhysicalInventoryError::InvalidLocator
        | PhysicalInventoryError::InvalidBinding
        | PhysicalInventoryError::ExternalTreeNotDisjoint
        | PhysicalInventoryError::BindingMismatch => {
            BackupError::InvalidManifest(error.to_string())
        }
    }
}

fn digest_regular_file(path: &Path, canonical_root: &Path) -> Result<(u64, String), BackupError> {
    let path_before = fs::symlink_metadata(path).map_err(BackupError::Io)?;
    if linked_or_reparse(&path_before) || !path_before.is_file() {
        return Err(BackupError::LinkedPath(path.to_path_buf()));
    }
    ensure_resolved_inside(path, canonical_root)?;
    let mut file = File::open(path).map_err(BackupError::Io)?;
    let before = file.metadata().map_err(BackupError::Io)?;
    if !before.is_file() {
        return Err(BackupError::UnsupportedEntry(path.to_path_buf()));
    }
    let before_modified = before.modified().ok();
    let mut length = 0_u64;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = file.read(&mut buffer).map_err(BackupError::Io)?;
        if read == 0 {
            break;
        }
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| BackupError::Verification("file length overflow".to_owned()))?;
        hasher.update(&buffer[..read]);
    }
    let after = file.metadata().map_err(BackupError::Io)?;
    let path_after = fs::symlink_metadata(path).map_err(BackupError::Io)?;
    if linked_or_reparse(&path_after)
        || !path_after.is_file()
        || before.len() != length
        || after.len() != length
        || (before_modified.is_some() && before_modified != after.modified().ok())
    {
        return Err(BackupError::ConcurrentChange(path.to_path_buf()));
    }
    ensure_resolved_inside(path, canonical_root)?;
    Ok((length, format!("{:x}", hasher.finalize())))
}

fn ensure_resolved_inside(path: &Path, canonical_root: &Path) -> Result<(), BackupError> {
    let canonical = fs::canonicalize(path).map_err(BackupError::Io)?;
    if canonical.starts_with(canonical_root) {
        Ok(())
    } else {
        Err(BackupError::PathEscape(path.to_path_buf()))
    }
}

fn portable_locator(root: &Path, path: &Path) -> Result<String, BackupError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| BackupError::PathEscape(path.to_path_buf()))?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return Err(BackupError::PathEscape(path.to_path_buf()));
        };
        let value = value
            .to_str()
            .ok_or_else(|| BackupError::NonUtf8Path(path.to_path_buf()))?;
        if value.is_empty() || value.contains(['\\', '\0']) {
            return Err(BackupError::Path(format!(
                "non-portable backup path: {}",
                path.display()
            )));
        }
        pieces.push(value);
    }
    if pieces.is_empty() {
        return Err(BackupError::PathEscape(path.to_path_buf()));
    }
    Ok(pieces.join("/"))
}

fn total_file_bytes(entries: &[BackupEntry]) -> Result<u64, BackupError> {
    entries
        .iter()
        .filter(|entry| entry.entry_type == BackupEntryType::File)
        .try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.length)
                .ok_or_else(|| BackupError::InvalidManifest("total byte count overflow".to_owned()))
        })
}

fn directory_sha256(locator: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.backup.directory.v1\0");
    hasher.update(locator.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn validate_manifest(manifest: &SnapshotManifest) -> Result<(), BackupError> {
    if manifest.schema != SNAPSHOT_MANIFEST_SCHEMA
        || manifest.scope != BackupScope::FullWorkspace
        || manifest.document_profile != DOCUMENT_PROFILE
    {
        return Err(BackupError::InvalidManifest(
            "unsupported snapshot schema, scope, or document profile".to_owned(),
        ));
    }
    parse_canonical_v4(&manifest.snapshot_id, "snapshot")?;
    NodeId::from_str(&manifest.workspace_root_id)
        .map_err(|error| BackupError::InvalidManifest(error.to_string()))?;
    WorkspaceRevision::parse(&manifest.workspace_revision)?;
    if manifest.root_name.is_empty()
        || matches!(manifest.root_name.as_str(), "." | "..")
        || manifest.root_name.contains(['/', '\\', '\0'])
    {
        return Err(BackupError::InvalidManifest(
            "snapshot root name is not portable".to_owned(),
        ));
    }
    if !manifest.exclusions.is_empty() {
        return Err(BackupError::UnsupportedScope(
            "v1 full-workspace snapshots do not support exclusions".to_owned(),
        ));
    }
    if manifest.entries.len() > MAX_MANIFEST_ENTRIES
        || manifest.entry_count != manifest.entries.len() as u64
        || manifest.total_bytes != total_file_bytes(&manifest.entries)?
    {
        return Err(BackupError::InvalidManifest(
            "snapshot entry or byte totals are inconsistent".to_owned(),
        ));
    }

    let mut previous: Option<&str> = None;
    let mut directories = BTreeSet::new();
    for entry in &manifest.entries {
        validate_locator(&entry.locator)?;
        if previous.is_some_and(|value| value >= entry.locator.as_str()) {
            return Err(BackupError::InvalidManifest(
                "snapshot entries must be strictly sorted and unique".to_owned(),
            ));
        }
        previous = Some(&entry.locator);
        validate_sha256(&entry.sha256)?;
        if let Some(parent) = entry.locator.rsplit_once('/').map(|(parent, _)| parent)
            && !directories.contains(parent)
        {
            return Err(BackupError::InvalidManifest(format!(
                "snapshot entry has an unrecorded parent directory: {}",
                entry.locator
            )));
        }
        match entry.entry_type {
            BackupEntryType::Directory => {
                if entry.length != 0 || entry.sha256 != directory_sha256(&entry.locator) {
                    return Err(BackupError::InvalidManifest(format!(
                        "directory metadata is inconsistent: {}",
                        entry.locator
                    )));
                }
                directories.insert(entry.locator.as_str());
            }
            BackupEntryType::File => {}
        }
    }
    Ok(())
}

fn validate_locator(locator: &str) -> Result<(), BackupError> {
    if locator.is_empty()
        || locator.starts_with('/')
        || locator.ends_with('/')
        || locator.contains(['\\', '\0'])
    {
        return Err(BackupError::InvalidManifest(format!(
            "non-portable snapshot locator: {locator}"
        )));
    }
    for (index, component) in locator.split('/').enumerate() {
        if component.is_empty() || matches!(component, "." | "..") {
            return Err(BackupError::InvalidManifest(format!(
                "unsafe snapshot locator: {locator}"
            )));
        }
        if index == 0
            && component
                .to_ascii_lowercase()
                .starts_with(WORKSPACE_ROOT_TRANSACTION_PREFIX)
        {
            return Err(BackupError::UnfinishedTransaction(PathBuf::from(locator)));
        }
    }
    let path = Path::new(locator);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(BackupError::InvalidManifest(format!(
            "unsafe snapshot locator: {locator}"
        )));
    }
    Ok(())
}

fn validate_sha256(value: &str) -> Result<(), BackupError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(BackupError::InvalidManifest(
            "snapshot digest is not canonical lowercase SHA-256".to_owned(),
        ))
    }
}

fn manifest_bytes(manifest: &SnapshotManifest) -> Result<Vec<u8>, BackupError> {
    let mut bytes = serde_json::to_vec_pretty(manifest).map_err(BackupError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn manifest_for_backup_plan(plan: &FullWorkspaceBackupPlan) -> SnapshotManifest {
    SnapshotManifest {
        schema: SNAPSHOT_MANIFEST_SCHEMA.to_owned(),
        scope: BackupScope::FullWorkspace,
        snapshot_id: plan.snapshot_id.hyphenated().to_string(),
        document_profile: DOCUMENT_PROFILE.to_owned(),
        workspace_root_id: plan.workspace_root_id.to_string(),
        workspace_revision: plan.workspace_revision.to_string(),
        root_name: plan.root_name.clone(),
        exclusions: Vec::new(),
        entries: plan.entries.clone(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
    }
}

fn backup_plan_digest(
    manifest_sha256: &str,
    source_binding: &str,
    destination_binding: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.full-workspace-backup-plan.v1\0");
    hasher.update(manifest_sha256.as_bytes());
    hasher.update([0]);
    hasher.update(source_binding.as_bytes());
    hasher.update([0]);
    hasher.update(destination_binding.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn restore_plan_digest(
    restore_id: Uuid,
    manifest_sha256: &str,
    snapshot_binding: &str,
    destination_binding: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.full-workspace-restore-plan.v1\0");
    hasher.update(restore_id.hyphenated().to_string().as_bytes());
    hasher.update([0]);
    hasher.update(manifest_sha256.as_bytes());
    hasher.update([0]);
    hasher.update(snapshot_binding.as_bytes());
    hasher.update([0]);
    hasher.update(destination_binding.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn restore_drill_plan_digest(
    drill_id: Uuid,
    manifest_sha256: &str,
    snapshot_binding: &str,
    drill_parent_binding: &str,
    results_parent_binding: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.full-workspace-restore-drill-plan.v1\0");
    hasher.update(drill_id.hyphenated().to_string().as_bytes());
    for value in [
        manifest_sha256,
        snapshot_binding,
        drill_parent_binding,
        results_parent_binding,
    ] {
        hasher.update([0]);
        hasher.update(value.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn validate_retention_policy(policy: SnapshotRetentionPolicy) -> Result<(), BackupError> {
    let keep_latest =
        usize::try_from(policy.keep_latest_unprotected).map_err(|_| BackupError::InvalidPlan)?;
    if keep_latest > MAX_MANIFEST_ENTRIES {
        return Err(BackupError::InvalidManifest(format!(
            "retention keep count exceeds {MAX_MANIFEST_ENTRIES}"
        )));
    }
    Ok(())
}

fn retention_transaction_path(backup_parent: &Path, operation_id: Uuid) -> PathBuf {
    backup_parent.join(format!("{RETENTION_TRANSACTION_PREFIX}{operation_id}"))
}

fn retention_lock_path(backup_parent: &Path) -> PathBuf {
    backup_parent.join(RETENTION_LOCK_DIRECTORY)
}

fn validate_empty_retention_lock(lock: &Path) -> Result<(), BackupError> {
    let metadata = fs::symlink_metadata(lock).map_err(BackupError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(BackupError::LinkedPath(lock.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(BackupError::UnsupportedEntry(lock.to_path_buf()));
    }
    if canonical_existing_directory(lock, "retention lock")? != lock {
        return Err(BackupError::PathEscape(lock.to_path_buf()));
    }
    if fs::read_dir(lock)
        .map_err(BackupError::Io)?
        .next()
        .is_some()
    {
        return Err(BackupError::Verification(
            "retention lock contains unknown evidence".to_owned(),
        ));
    }
    Ok(())
}

fn release_retention_lock(backup_parent: &Path) -> Result<(), BackupError> {
    let lock = retention_lock_path(backup_parent);
    validate_empty_retention_lock(&lock)?;
    fs::remove_dir(lock).map_err(BackupError::Io)?;
    sync_directory(backup_parent)
}

fn retention_receipt_path(backup_parent: &Path, operation_id: Uuid) -> PathBuf {
    backup_parent.join(format!("{RETENTION_RECEIPT_PREFIX}{operation_id}.json"))
}

fn list_retention_transactions(backup_parent: &Path) -> Result<Vec<PathBuf>, BackupError> {
    let mut transactions = Vec::new();
    for entry in fs::read_dir(backup_parent).map_err(BackupError::Io)? {
        let entry = entry.map_err(BackupError::Io)?;
        let path = entry.path();
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| BackupError::NonUtf8Path(path.clone()))?
            .to_owned();
        let Some(identifier) = name.strip_prefix(RETENTION_TRANSACTION_PREFIX) else {
            continue;
        };
        parse_canonical_v4(identifier, "retention operation")?;
        let metadata = fs::symlink_metadata(&path).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(path));
        }
        if !metadata.is_dir() {
            return Err(BackupError::UnsupportedEntry(path));
        }
        if canonical_existing_directory(&path, "retention transaction")? != path {
            return Err(BackupError::PathEscape(path));
        }
        transactions.push(path);
    }
    transactions.sort();
    Ok(transactions)
}

fn inventory_retention_snapshots(
    backup_parent: &Path,
) -> Result<Vec<SnapshotRetentionItem>, BackupError> {
    let mut snapshots = Vec::new();
    let mut identifiers = BTreeSet::new();
    for entry in fs::read_dir(backup_parent).map_err(BackupError::Io)? {
        let entry = entry.map_err(BackupError::Io)?;
        let path = entry.path();
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| BackupError::NonUtf8Path(path.clone()))?
            .to_owned();
        let Some(identifier) = name.strip_prefix(SNAPSHOT_DIRECTORY_PREFIX) else {
            continue;
        };
        let expected_id = parse_canonical_v4(identifier, "snapshot")?;
        let metadata = fs::symlink_metadata(&path).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(path));
        }
        if !metadata.is_dir() {
            return Err(BackupError::UnsupportedEntry(path));
        }
        if canonical_existing_directory(&path, "snapshot directory")? != path {
            return Err(BackupError::PathEscape(path));
        }
        let verified = verify_snapshot_internal(&path)?;
        if verified.snapshot_id != expected_id || !identifiers.insert(expected_id) {
            return Err(BackupError::Verification(
                "snapshot directory name does not uniquely bind its manifest ID".to_owned(),
            ));
        }
        snapshots.push(SnapshotRetentionItem {
            snapshot_id: verified.snapshot_id,
            snapshot_directory: verified.directory,
            created_at_unix_ms: verified.created_at_unix_ms,
            manifest_sha256: verified.manifest_sha256,
            protection: verified.protection,
        });
    }
    snapshots.sort_by(|left, right| {
        right
            .created_at_unix_ms
            .cmp(&left.created_at_unix_ms)
            .then_with(|| right.snapshot_id.cmp(&left.snapshot_id))
    });
    Ok(snapshots)
}

fn retention_plan_digest(
    operation_id: Uuid,
    parent_binding: &str,
    policy: SnapshotRetentionPolicy,
    retained: &[SnapshotRetentionItem],
    pruned: &[SnapshotRetentionItem],
) -> Result<String, BackupError> {
    let payload = serde_json::to_vec(&(
        SNAPSHOT_RETENTION_PLAN_SCHEMA,
        operation_id.hyphenated().to_string(),
        parent_binding,
        policy,
        retained,
        pruned,
    ))
    .map_err(BackupError::Json)?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.snapshot-retention-plan.v1\0");
    hasher.update(payload);
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_retention_plan(plan: &SnapshotRetentionPlan) -> Result<(), BackupError> {
    if plan.schema != SNAPSHOT_RETENTION_PLAN_SCHEMA {
        return Err(BackupError::InvalidPlan);
    }
    validate_v4(plan.operation_id, "retention operation")?;
    validate_retention_policy(plan.policy)?;
    validate_sha256(&plan.plan_digest)?;
    let parent = canonical_existing_directory(&plan.backup_parent, "backup parent")?;
    if parent != plan.backup_parent
        || path_binding(&parent)? != plan.parent_binding
        || retention_receipt_path(&parent, plan.operation_id) != plan.receipt_file
    {
        return Err(BackupError::InvalidPlan);
    }
    let mut identifiers = BTreeSet::new();
    for snapshot in plan.retained.iter().chain(&plan.pruned) {
        validate_v4(snapshot.snapshot_id, "snapshot")?;
        validate_sha256(&snapshot.manifest_sha256)?;
        if snapshot.created_at_unix_ms == 0
            || snapshot.snapshot_directory
                != parent.join(format!(
                    "{SNAPSHOT_DIRECTORY_PREFIX}{}",
                    snapshot.snapshot_id
                ))
            || !identifiers.insert(snapshot.snapshot_id)
        {
            return Err(BackupError::InvalidPlan);
        }
        if let Some(protection) = &snapshot.protection {
            if protection.snapshot_id != snapshot.snapshot_id
                || protection.schema != SNAPSHOT_PROTECTION_SCHEMA
                || protection.protected_at_unix_ms == 0
            {
                return Err(BackupError::InvalidPlan);
            }
            validate_protection_label(&protection.label)?;
        }
    }
    if plan
        .pruned
        .iter()
        .any(|snapshot| snapshot.protection.is_some())
        || retention_plan_digest(
            plan.operation_id,
            &plan.parent_binding,
            plan.policy,
            &plan.retained,
            &plan.pruned,
        )? != plan.plan_digest
    {
        return Err(BackupError::InvalidPlan);
    }
    Ok(())
}

fn retention_journal_from_plan(plan: &SnapshotRetentionPlan) -> SnapshotRetentionJournal {
    SnapshotRetentionJournal {
        schema: SNAPSHOT_RETENTION_JOURNAL_SCHEMA.to_owned(),
        operation_id: plan.operation_id,
        plan_digest: plan.plan_digest.clone(),
        parent_binding: plan.parent_binding.clone(),
        policy: plan.policy,
        retained_snapshot_ids: plan
            .retained
            .iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect(),
        entries: plan
            .pruned
            .iter()
            .map(|snapshot| SnapshotRetentionJournalEntry {
                snapshot_id: snapshot.snapshot_id,
                directory_name: snapshot
                    .snapshot_directory
                    .file_name()
                    .and_then(|name| name.to_str())
                    .expect("validated UTF-8 snapshot name")
                    .to_owned(),
                manifest_sha256: snapshot.manifest_sha256.clone(),
            })
            .collect(),
    }
}

fn retention_receipt_from_plan(
    plan: &SnapshotRetentionPlan,
) -> Result<SnapshotRetentionReceipt, BackupError> {
    Ok(SnapshotRetentionReceipt {
        schema: SNAPSHOT_RETENTION_RECEIPT_SCHEMA.to_owned(),
        operation_id: plan.operation_id,
        plan_digest: plan.plan_digest.clone(),
        policy: plan.policy,
        retained_snapshot_ids: plan
            .retained
            .iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect(),
        pruned_snapshot_ids: plan
            .pruned
            .iter()
            .map(|snapshot| snapshot.snapshot_id)
            .collect(),
        completed_at_unix_ms: unix_time_ms()?,
    })
}

fn validate_retention_journal(
    journal: &SnapshotRetentionJournal,
    backup_parent: &Path,
    transaction: &Path,
) -> Result<(), BackupError> {
    if journal.schema != SNAPSHOT_RETENTION_JOURNAL_SCHEMA {
        return Err(BackupError::InvalidManifest(
            "unsupported retention journal schema".to_owned(),
        ));
    }
    validate_v4(journal.operation_id, "retention operation")?;
    validate_sha256(&journal.plan_digest)?;
    validate_retention_policy(journal.policy)?;
    if journal.parent_binding != path_binding(backup_parent)?
        || retention_transaction_path(backup_parent, journal.operation_id) != transaction
        || journal.entries.len() > MAX_MANIFEST_ENTRIES
    {
        return Err(BackupError::Verification(
            "retention journal does not bind its destination or operation".to_owned(),
        ));
    }
    let mut identifiers = BTreeSet::new();
    for identifier in &journal.retained_snapshot_ids {
        validate_v4(*identifier, "snapshot")?;
        if !identifiers.insert(*identifier) {
            return Err(BackupError::InvalidManifest(
                "retention journal contains duplicate snapshot IDs".to_owned(),
            ));
        }
    }
    for entry in &journal.entries {
        validate_v4(entry.snapshot_id, "snapshot")?;
        validate_sha256(&entry.manifest_sha256)?;
        if entry.directory_name != format!("{SNAPSHOT_DIRECTORY_PREFIX}{}", entry.snapshot_id)
            || !identifiers.insert(entry.snapshot_id)
        {
            return Err(BackupError::InvalidManifest(
                "retention journal snapshot binding is invalid or duplicated".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_retention_receipt_shape(receipt: &SnapshotRetentionReceipt) -> Result<(), BackupError> {
    if receipt.schema != SNAPSHOT_RETENTION_RECEIPT_SCHEMA || receipt.completed_at_unix_ms == 0 {
        return Err(BackupError::InvalidManifest(
            "retention receipt schema or timestamp is invalid".to_owned(),
        ));
    }
    validate_v4(receipt.operation_id, "retention operation")?;
    validate_sha256(&receipt.plan_digest)?;
    validate_retention_policy(receipt.policy)?;
    let mut identifiers = BTreeSet::new();
    for identifier in receipt
        .retained_snapshot_ids
        .iter()
        .chain(&receipt.pruned_snapshot_ids)
    {
        validate_v4(*identifier, "snapshot")?;
        if !identifiers.insert(*identifier) {
            return Err(BackupError::InvalidManifest(
                "retention receipt contains duplicate snapshot IDs".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_retention_receipt(
    receipt: &SnapshotRetentionReceipt,
    journal: &SnapshotRetentionJournal,
) -> Result<(), BackupError> {
    validate_retention_receipt_shape(receipt)?;
    let pruned = journal
        .entries
        .iter()
        .map(|entry| entry.snapshot_id)
        .collect::<Vec<_>>();
    if receipt.operation_id != journal.operation_id
        || receipt.plan_digest != journal.plan_digest
        || receipt.policy != journal.policy
        || receipt.retained_snapshot_ids != journal.retained_snapshot_ids
        || receipt.pruned_snapshot_ids != pruned
    {
        return Err(BackupError::Verification(
            "retention receipt does not bind the reviewed journal".to_owned(),
        ));
    }
    Ok(())
}

fn verify_retention_snapshot_binding(
    snapshot: &Path,
    entry: &SnapshotRetentionJournalEntry,
) -> Result<(), BackupError> {
    let verified = verify_snapshot_internal(snapshot)?;
    if verified.snapshot_id != entry.snapshot_id
        || verified.manifest_sha256 != entry.manifest_sha256
        || verified.protection.is_some()
    {
        return Err(BackupError::Verification(
            "retention snapshot no longer matches its journal binding or became protected"
                .to_owned(),
        ));
    }
    Ok(())
}

fn write_json_new(path: &Path, value: &impl Serialize) -> Result<(), BackupError> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(BackupError::Json)?;
    bytes.push(b'\n');
    write_new_file(path, &bytes)
}

fn read_json_bounded<T: DeserializeOwned>(
    path: &Path,
    canonical_root: &Path,
    limit: u64,
) -> Result<T, BackupError> {
    let bytes = read_bounded_regular_file(path, canonical_root, limit)?;
    serde_json::from_slice(&bytes).map_err(BackupError::Json)
}

fn read_retention_journal_or_remove_empty(
    backup_parent: &Path,
    transaction: &Path,
) -> Result<Option<SnapshotRetentionJournal>, BackupError> {
    validate_retention_transaction_container(transaction)?;
    let journal_path = transaction.join(RETENTION_JOURNAL_FILE);
    if !journal_path.try_exists().map_err(BackupError::Io)? {
        if fs::read_dir(transaction)
            .map_err(BackupError::Io)?
            .next()
            .is_none()
        {
            fs::remove_dir(transaction).map_err(BackupError::Io)?;
            sync_directory(backup_parent)?;
            return Ok(None);
        }
        return Err(BackupError::IncompleteSnapshot(transaction.to_path_buf()));
    }
    let journal: SnapshotRetentionJournal =
        read_json_bounded(&journal_path, transaction, 1024 * 1024)?;
    validate_retention_journal(&journal, backup_parent, transaction)?;
    Ok(Some(journal))
}

fn validate_retention_transaction_container(transaction: &Path) -> Result<(), BackupError> {
    let allowed = BTreeSet::from([
        RETENTION_JOURNAL_FILE.to_owned(),
        RETENTION_HOLDING_DIRECTORY.to_owned(),
        RETENTION_COMMIT_FILE.to_owned(),
    ]);
    for entry in fs::read_dir(transaction).map_err(BackupError::Io)? {
        let entry = entry.map_err(BackupError::Io)?;
        let path = entry.path();
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| BackupError::NonUtf8Path(path.clone()))?
            .to_owned();
        if !allowed.contains(&name) {
            return Err(BackupError::InvalidManifest(
                "retention transaction contains an unknown entry".to_owned(),
            ));
        }
        let metadata = fs::symlink_metadata(&path).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(path));
        }
        if name == RETENTION_HOLDING_DIRECTORY {
            if !metadata.is_dir() {
                return Err(BackupError::UnsupportedEntry(path));
            }
        } else if !metadata.is_file() {
            return Err(BackupError::UnsupportedEntry(path));
        }
    }
    Ok(())
}

fn retention_receipt_exists(backup_parent: &Path, operation_id: Uuid) -> Result<bool, BackupError> {
    retention_receipt_path(backup_parent, operation_id)
        .try_exists()
        .map_err(BackupError::Io)
}

fn finalize_retention_transaction(
    backup_parent: &Path,
    transaction: &Path,
    journal: &SnapshotRetentionJournal,
    receipt: &SnapshotRetentionReceipt,
) -> Result<(), BackupError> {
    validate_retention_journal(journal, backup_parent, transaction)?;
    validate_retention_receipt(receipt, journal)?;
    let receipt_path = retention_receipt_path(backup_parent, journal.operation_id);
    if receipt_path.try_exists().map_err(BackupError::Io)? {
        let existing: SnapshotRetentionReceipt =
            read_json_bounded(&receipt_path, backup_parent, 1024 * 1024)?;
        if existing != *receipt {
            return Err(BackupError::Verification(
                "durable retention receipt conflicts with the commit marker".to_owned(),
            ));
        }
    } else {
        write_json_new(&receipt_path, receipt)?;
        sync_directory(backup_parent)?;
    }

    let holding_root = transaction.join(RETENTION_HOLDING_DIRECTORY);
    for entry in &journal.entries {
        let source = backup_parent.join(&entry.directory_name);
        if source.try_exists().map_err(BackupError::Io)? {
            return Err(BackupError::Verification(
                "committed retention transaction cannot resurrect a selected snapshot".to_owned(),
            ));
        }
        let holding = holding_root.join(&entry.directory_name);
        if holding.try_exists().map_err(BackupError::Io)? {
            verify_retention_snapshot_binding(&holding, entry)?;
            remove_verified_tree(&holding, transaction)?;
        }
    }
    if holding_root.try_exists().map_err(BackupError::Io)? {
        if fs::read_dir(&holding_root)
            .map_err(BackupError::Io)?
            .next()
            .is_some()
        {
            return Err(BackupError::Verification(
                "retention holding contains unjournaled entries".to_owned(),
            ));
        }
        fs::remove_dir(&holding_root).map_err(BackupError::Io)?;
    }
    let commit_path = transaction.join(RETENTION_COMMIT_FILE);
    if commit_path.try_exists().map_err(BackupError::Io)? {
        fs::remove_file(commit_path).map_err(BackupError::Io)?;
        sync_directory(transaction)?;
    }
    fs::remove_file(transaction.join(RETENTION_JOURNAL_FILE)).map_err(BackupError::Io)?;
    sync_directory(transaction)?;
    fs::remove_dir(transaction).map_err(BackupError::Io)?;
    sync_directory(backup_parent)
}

fn rollback_retention_transaction(
    backup_parent: &Path,
    transaction: &Path,
    journal: &SnapshotRetentionJournal,
) -> Result<(), BackupError> {
    validate_retention_journal(journal, backup_parent, transaction)?;
    if retention_receipt_exists(backup_parent, journal.operation_id)? {
        return Err(BackupError::Verification(
            "retention transaction has a durable commit receipt and cannot roll back".to_owned(),
        ));
    }
    let holding_root = transaction.join(RETENTION_HOLDING_DIRECTORY);
    for entry in journal.entries.iter().rev() {
        let source = backup_parent.join(&entry.directory_name);
        let holding = holding_root.join(&entry.directory_name);
        let source_exists = source.try_exists().map_err(BackupError::Io)?;
        let holding_exists = holding.try_exists().map_err(BackupError::Io)?;
        match (source_exists, holding_exists) {
            (true, false) => verify_retention_snapshot_binding(&source, entry)?,
            (false, true) => {
                verify_retention_snapshot_binding(&holding, entry)?;
                fs::rename(&holding, &source).map_err(BackupError::Io)?;
                sync_directory(&holding_root)?;
                sync_directory(backup_parent)?;
                verify_retention_snapshot_binding(&source, entry)?;
            }
            _ => {
                return Err(BackupError::Verification(
                    "retention rollback found contradictory source and holding state".to_owned(),
                ));
            }
        }
    }
    if holding_root.try_exists().map_err(BackupError::Io)? {
        if fs::read_dir(&holding_root)
            .map_err(BackupError::Io)?
            .next()
            .is_some()
        {
            return Err(BackupError::Verification(
                "retention holding contains unjournaled entries".to_owned(),
            ));
        }
        fs::remove_dir(&holding_root).map_err(BackupError::Io)?;
    }
    fs::remove_file(transaction.join(RETENTION_JOURNAL_FILE)).map_err(BackupError::Io)?;
    sync_directory(transaction)?;
    fs::remove_dir(transaction).map_err(BackupError::Io)?;
    sync_directory(backup_parent)
}

fn remove_verified_tree(path: &Path, allowed_root: &Path) -> Result<(), BackupError> {
    let allowed_root = canonical_existing_directory(allowed_root, "retention transaction")?;
    let path = canonical_existing_directory(path, "held snapshot")?;
    if path == allowed_root || !path.starts_with(&allowed_root) {
        return Err(BackupError::PathEscape(path));
    }
    remove_verified_directory_recursive(&path, &allowed_root)
}

fn remove_verified_directory_recursive(
    directory: &Path,
    allowed_root: &Path,
) -> Result<(), BackupError> {
    let metadata = fs::symlink_metadata(directory).map_err(BackupError::Io)?;
    if linked_or_reparse(&metadata) || !metadata.is_dir() {
        return Err(BackupError::LinkedPath(directory.to_path_buf()));
    }
    ensure_resolved_inside(directory, allowed_root)?;
    let mut entries = fs::read_dir(directory)
        .map_err(BackupError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(BackupError::Io)?;
    entries.sort_by_key(fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(path));
        }
        ensure_resolved_inside(&path, allowed_root)?;
        if metadata.is_dir() {
            remove_verified_directory_recursive(&path, allowed_root)?;
        } else if metadata.is_file() {
            fs::remove_file(&path).map_err(BackupError::Io)?;
        } else {
            return Err(BackupError::UnsupportedEntry(path));
        }
    }
    fs::remove_dir(directory).map_err(BackupError::Io)
}

fn validate_backup_plan(plan: &FullWorkspaceBackupPlan) -> Result<(), BackupError> {
    if plan.schema != BACKUP_PLAN_SCHEMA {
        return Err(BackupError::InvalidPlan);
    }
    validate_v4(plan.snapshot_id, "snapshot")?;
    let canonical_root = canonical_existing_directory(&plan.workspace_root, "workspace root")?;
    let canonical_parent = canonical_existing_directory(&plan.backup_parent, "backup parent")?;
    if canonical_root != plan.workspace_root || canonical_parent != plan.backup_parent {
        return Err(BackupError::InvalidPlan);
    }
    let expected_snapshot =
        canonical_parent.join(format!("{SNAPSHOT_DIRECTORY_PREFIX}{}", plan.snapshot_id));
    if expected_snapshot != plan.snapshot_directory
        || path_binding(&canonical_root)? != plan.source_binding
        || path_binding(&canonical_parent)? != plan.destination_binding
    {
        return Err(BackupError::InvalidPlan);
    }
    let manifest = manifest_for_backup_plan(plan);
    validate_manifest(&manifest)?;
    let manifest_sha256 = sha256(&manifest_bytes(&manifest)?);
    if manifest_sha256 != plan.manifest_sha256
        || backup_plan_digest(
            &manifest_sha256,
            &plan.source_binding,
            &plan.destination_binding,
        ) != plan.plan_digest
    {
        return Err(BackupError::InvalidPlan);
    }
    Ok(())
}

fn commit_snapshot_contents(
    plan: &FullWorkspaceBackupPlan,
    workspace_lease: &WorkspaceTransactionLease,
) -> Result<FullWorkspaceBackupReceipt, BackupError> {
    let content_directory = plan.snapshot_directory.join(SNAPSHOT_CONTENT_DIRECTORY);
    fs::create_dir(&content_directory).map_err(BackupError::Io)?;
    let content_root = content_directory.join(&plan.root_name);
    fs::create_dir(&content_root).map_err(BackupError::Io)?;
    for entry in &plan.entries {
        let destination = safe_join(&content_root, &entry.locator)?;
        match entry.entry_type {
            BackupEntryType::Directory => {
                fs::create_dir(&destination).map_err(BackupError::Io)?;
            }
            BackupEntryType::File => {
                let source = safe_join(&plan.workspace_root, &entry.locator)?;
                copy_regular_file_exact(
                    &source,
                    &plan.workspace_root,
                    &destination,
                    &content_root,
                    entry,
                )?;
            }
        }
    }
    sync_recorded_directories(&content_root, &plan.entries)?;
    sync_directory(&content_directory)?;

    require_backup_source_current(plan, workspace_lease)?;
    verify_bytewise_trees(&plan.workspace_root, &content_root, &plan.entries)?;

    let manifest = manifest_for_backup_plan(plan);
    let manifest_bytes = manifest_bytes(&manifest)?;
    if sha256(&manifest_bytes) != plan.manifest_sha256 {
        return Err(BackupError::InvalidPlan);
    }
    write_new_file(
        &plan.snapshot_directory.join(SNAPSHOT_MANIFEST_FILE),
        &manifest_bytes,
    )?;
    if collect_stable_physical_entries(&content_root)? != plan.entries {
        return Err(BackupError::Verification(
            "snapshot content inventory differs from the reviewed plan".to_owned(),
        ));
    }
    require_backup_source_current(plan, workspace_lease)?;

    let completion = SnapshotCompletion {
        schema: SNAPSHOT_COMPLETION_SCHEMA.to_owned(),
        snapshot_id: plan.snapshot_id.hyphenated().to_string(),
        manifest_sha256: plan.manifest_sha256.clone(),
        manifest_length: manifest_bytes.len() as u64,
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
        created_at_unix_ms: unix_time_ms()?,
    };
    let mut completion_bytes = serde_json::to_vec_pretty(&completion).map_err(BackupError::Json)?;
    completion_bytes.push(b'\n');
    write_new_file(
        &plan.snapshot_directory.join(SNAPSHOT_COMPLETION_FILE),
        &completion_bytes,
    )?;
    sync_directory(&plan.snapshot_directory)?;
    sync_directory(&plan.backup_parent)?;

    let verified = verify_snapshot_internal(&plan.snapshot_directory)?;
    if verified.manifest_sha256 != plan.manifest_sha256 || verified.snapshot_id != plan.snapshot_id
    {
        return Err(BackupError::Verification(
            "committed snapshot did not reopen as the reviewed plan".to_owned(),
        ));
    }
    Ok(FullWorkspaceBackupReceipt {
        schema: "weftext.full-workspace-backup-receipt.v1".to_owned(),
        snapshot_id: plan.snapshot_id,
        snapshot_directory: plan.snapshot_directory.clone(),
        workspace_root_id: plan.workspace_root_id,
        workspace_revision: plan.workspace_revision.clone(),
        manifest_sha256: plan.manifest_sha256.clone(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
        verified: true,
    })
}

fn require_backup_source_current(
    plan: &FullWorkspaceBackupPlan,
    workspace_lease: &WorkspaceTransactionLease,
) -> Result<(), BackupError> {
    let (root_id, revision, root_name, current_entries) =
        stable_workspace_inventory(&plan.workspace_root, workspace_lease)?;
    if root_id == plan.workspace_root_id
        && revision == plan.workspace_revision
        && root_name == plan.root_name
        && current_entries == plan.entries
    {
        Ok(())
    } else {
        Err(BackupError::StalePreview)
    }
}

fn copy_regular_file_exact(
    source: &Path,
    source_root: &Path,
    destination: &Path,
    destination_root: &Path,
    entry: &BackupEntry,
) -> Result<(), BackupError> {
    let source_metadata = fs::symlink_metadata(source).map_err(BackupError::Io)?;
    if linked_or_reparse(&source_metadata) || !source_metadata.is_file() {
        return Err(BackupError::LinkedPath(source.to_path_buf()));
    }
    ensure_resolved_inside(source, source_root)?;
    let parent = destination
        .parent()
        .ok_or_else(|| BackupError::Path("copied file has no parent".to_owned()))?;
    let parent_metadata = fs::symlink_metadata(parent).map_err(BackupError::Io)?;
    if linked_or_reparse(&parent_metadata) || !parent_metadata.is_dir() {
        return Err(BackupError::LinkedPath(parent.to_path_buf()));
    }
    ensure_resolved_inside(parent, destination_root)?;

    let mut input = File::open(source).map_err(BackupError::Io)?;
    let input_before = input.metadata().map_err(BackupError::Io)?;
    let input_modified = input_before.modified().ok();
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(BackupError::Io)?;
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    let mut buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let read = input.read(&mut buffer).map_err(BackupError::Io)?;
        if read == 0 {
            break;
        }
        output.write_all(&buffer[..read]).map_err(BackupError::Io)?;
        hasher.update(&buffer[..read]);
        length = length
            .checked_add(read as u64)
            .ok_or_else(|| BackupError::Verification("file length overflow".to_owned()))?;
    }
    output.flush().map_err(BackupError::Io)?;
    output.sync_all().map_err(BackupError::Io)?;
    drop(output);

    let input_after = input.metadata().map_err(BackupError::Io)?;
    let digest = format!("{:x}", hasher.finalize());
    if length != entry.length
        || digest != entry.sha256
        || input_before.len() != length
        || input_after.len() != length
        || (input_modified.is_some() && input_modified != input_after.modified().ok())
    {
        return Err(BackupError::ConcurrentChange(source.to_path_buf()));
    }
    let current_source = digest_regular_file(source, source_root)?;
    let copied = digest_regular_file(destination, destination_root)?;
    if current_source != (entry.length, entry.sha256.clone())
        || copied != (entry.length, entry.sha256.clone())
        || !files_equal(source, destination)?
    {
        return Err(BackupError::Verification(format!(
            "copied file differs from reviewed bytes: {}",
            entry.locator
        )));
    }
    sync_directory(parent)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, BackupError> {
    let mut left = File::open(left).map_err(BackupError::Io)?;
    let mut right = File::open(right).map_err(BackupError::Io)?;
    let mut left_buffer = vec![0_u8; COPY_BUFFER_BYTES];
    let mut right_buffer = vec![0_u8; COPY_BUFFER_BYTES];
    loop {
        let left_read = left.read(&mut left_buffer).map_err(BackupError::Io)?;
        let right_read = right.read(&mut right_buffer).map_err(BackupError::Io)?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn verify_bytewise_trees(
    left_root: &Path,
    right_root: &Path,
    entries: &[BackupEntry],
) -> Result<(), BackupError> {
    for entry in entries {
        let left = safe_join(left_root, &entry.locator)?;
        let right = safe_join(right_root, &entry.locator)?;
        match entry.entry_type {
            BackupEntryType::Directory => {
                let left_metadata = fs::symlink_metadata(&left).map_err(BackupError::Io)?;
                let right_metadata = fs::symlink_metadata(&right).map_err(BackupError::Io)?;
                if linked_or_reparse(&left_metadata)
                    || linked_or_reparse(&right_metadata)
                    || !left_metadata.is_dir()
                    || !right_metadata.is_dir()
                {
                    return Err(BackupError::Verification(format!(
                        "directory type mismatch: {}",
                        entry.locator
                    )));
                }
            }
            BackupEntryType::File if !files_equal(&left, &right)? => {
                return Err(BackupError::Verification(format!(
                    "byte mismatch: {}",
                    entry.locator
                )));
            }
            BackupEntryType::File => {}
        }
    }
    Ok(())
}

fn sync_recorded_directories(root: &Path, entries: &[BackupEntry]) -> Result<(), BackupError> {
    for entry in entries.iter().rev() {
        if entry.entry_type == BackupEntryType::Directory {
            sync_directory(&safe_join(root, &entry.locator)?)?;
        }
    }
    sync_directory(root)
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), BackupError> {
    let parent = path
        .parent()
        .ok_or_else(|| BackupError::Path("new file has no parent".to_owned()))?;
    let parent_metadata = fs::symlink_metadata(parent).map_err(BackupError::Io)?;
    if linked_or_reparse(&parent_metadata) || !parent_metadata.is_dir() {
        return Err(BackupError::LinkedPath(parent.to_path_buf()));
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(BackupError::Io)?;
    file.write_all(bytes).map_err(BackupError::Io)?;
    file.flush().map_err(BackupError::Io)?;
    file.sync_all().map_err(BackupError::Io)?;
    drop(file);
    let reopened = fs::read(path).map_err(BackupError::Io)?;
    if reopened != bytes {
        return Err(BackupError::Verification(format!(
            "new file failed reopen verification: {}",
            path.display()
        )));
    }
    sync_directory(parent)
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), BackupError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(BackupError::Io)
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<(), BackupError> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(BackupError::Io)
}

fn verify_snapshot_internal(snapshot_directory: &Path) -> Result<VerifiedSnapshot, BackupError> {
    let snapshot_directory =
        canonical_existing_directory(snapshot_directory, "snapshot directory")?;
    let snapshot_parent = snapshot_directory
        .parent()
        .ok_or_else(|| BackupError::Path("snapshot directory has no parent".to_owned()))?;
    reject_workspace_marker_ancestor(snapshot_parent, "snapshot directory")?;
    validate_snapshot_container(&snapshot_directory)?;
    let manifest_path = snapshot_directory.join(SNAPSHOT_MANIFEST_FILE);
    let completion_path = snapshot_directory.join(SNAPSHOT_COMPLETION_FILE);
    let manifest_bytes =
        read_bounded_regular_file(&manifest_path, &snapshot_directory, MAX_MANIFEST_BYTES)?;
    let completion_bytes =
        read_bounded_regular_file(&completion_path, &snapshot_directory, 1024 * 1024)?;
    let manifest: SnapshotManifest =
        serde_json::from_slice(&manifest_bytes).map_err(BackupError::Json)?;
    let completion: SnapshotCompletion =
        serde_json::from_slice(&completion_bytes).map_err(BackupError::Json)?;
    validate_manifest(&manifest)?;
    validate_completion(&completion, &manifest, &manifest_bytes)?;
    let protection = read_snapshot_protection_internal(
        &snapshot_directory,
        parse_canonical_v4(&manifest.snapshot_id, "snapshot")?,
    )?;

    let content_root = validate_snapshot_content_root(&snapshot_directory, &manifest.root_name)?;
    let actual_entries = collect_stable_physical_entries(&content_root)?;
    if actual_entries != manifest.entries {
        return Err(BackupError::Verification(
            "snapshot physical inventory differs from its manifest".to_owned(),
        ));
    }
    let content_root_id = workspace_root_id(&content_root)?;
    let content_revision = read_workspace_revision(&content_root)?;
    let expected_root_id = NodeId::from_str(&manifest.workspace_root_id)
        .map_err(|error| BackupError::InvalidManifest(error.to_string()))?;
    let expected_revision = WorkspaceRevision::parse(&manifest.workspace_revision)?;
    if content_root_id != expected_root_id || content_revision != expected_revision {
        return Err(BackupError::Verification(
            "snapshot cannot be reopened with its recorded workspace identity and revision"
                .to_owned(),
        ));
    }
    let manifest_after =
        read_bounded_regular_file(&manifest_path, &snapshot_directory, MAX_MANIFEST_BYTES)?;
    let completion_after =
        read_bounded_regular_file(&completion_path, &snapshot_directory, 1024 * 1024)?;
    let protection_after = read_snapshot_protection_internal(
        &snapshot_directory,
        parse_canonical_v4(&manifest.snapshot_id, "snapshot")?,
    )?;
    if manifest_after != manifest_bytes
        || completion_after != completion_bytes
        || protection_after != protection
    {
        return Err(BackupError::ConcurrentChange(snapshot_directory));
    }

    let snapshot_id = parse_canonical_v4(&manifest.snapshot_id, "snapshot")?;
    let workspace_root_id = NodeId::from_str(&manifest.workspace_root_id)
        .map_err(|error| BackupError::InvalidManifest(error.to_string()))?;
    let workspace_revision = WorkspaceRevision::parse(&manifest.workspace_revision)?;
    Ok(VerifiedSnapshot {
        directory: snapshot_directory,
        workspace_content_root: content_root,
        manifest,
        manifest_sha256: completion.manifest_sha256,
        snapshot_id,
        workspace_root_id,
        workspace_revision,
        created_at_unix_ms: completion.created_at_unix_ms,
        protection,
    })
}

fn validate_snapshot_content_root(
    snapshot_directory: &Path,
    root_name: &str,
) -> Result<PathBuf, BackupError> {
    let content_directory = snapshot_directory.join(SNAPSHOT_CONTENT_DIRECTORY);
    let mut entries = fs::read_dir(&content_directory)
        .map_err(BackupError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(BackupError::Io)?;
    if entries.len() != 1 {
        return Err(BackupError::Verification(
            "snapshot content directory must contain exactly one workspace root".to_owned(),
        ));
    }
    let entry = entries.pop().expect("exactly one snapshot root");
    if entry.file_name().to_str() != Some(root_name) {
        return Err(BackupError::Verification(
            "snapshot workspace root name differs from the manifest".to_owned(),
        ));
    }
    canonical_existing_directory(&entry.path(), "snapshot workspace root")
}

fn validate_snapshot_container(snapshot_directory: &Path) -> Result<(), BackupError> {
    let completion = snapshot_directory.join(SNAPSHOT_COMPLETION_FILE);
    if !completion.exists() {
        return Err(BackupError::IncompleteSnapshot(
            snapshot_directory.to_path_buf(),
        ));
    }
    let expected = BTreeSet::from([
        SNAPSHOT_COMPLETION_FILE.to_owned(),
        SNAPSHOT_CONTENT_DIRECTORY.to_owned(),
        SNAPSHOT_MANIFEST_FILE.to_owned(),
    ]);
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(snapshot_directory).map_err(BackupError::Io)? {
        let entry = entry.map_err(BackupError::Io)?;
        let path = entry.path();
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| BackupError::NonUtf8Path(path.clone()))?
            .to_owned();
        let metadata = fs::symlink_metadata(&path).map_err(BackupError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(BackupError::LinkedPath(path));
        }
        actual.insert(name);
    }
    let mut expected_with_protection = expected.clone();
    expected_with_protection.insert(SNAPSHOT_PROTECTION_FILE.to_owned());
    if actual != expected && actual != expected_with_protection {
        return Err(BackupError::InvalidManifest(
            "snapshot container has missing or unknown top-level entries".to_owned(),
        ));
    }
    let content = fs::symlink_metadata(snapshot_directory.join(SNAPSHOT_CONTENT_DIRECTORY))
        .map_err(BackupError::Io)?;
    let manifest = fs::symlink_metadata(snapshot_directory.join(SNAPSHOT_MANIFEST_FILE))
        .map_err(BackupError::Io)?;
    let completion = fs::symlink_metadata(completion).map_err(BackupError::Io)?;
    if !content.is_dir() || !manifest.is_file() || !completion.is_file() {
        return Err(BackupError::InvalidManifest(
            "snapshot container entry types are invalid".to_owned(),
        ));
    }
    if actual.contains(SNAPSHOT_PROTECTION_FILE) {
        let protection = fs::symlink_metadata(snapshot_directory.join(SNAPSHOT_PROTECTION_FILE))
            .map_err(BackupError::Io)?;
        if !protection.is_file() {
            return Err(BackupError::InvalidManifest(
                "snapshot protection entry type is invalid".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_completion(
    completion: &SnapshotCompletion,
    manifest: &SnapshotManifest,
    manifest_bytes: &[u8],
) -> Result<(), BackupError> {
    if completion.schema != SNAPSHOT_COMPLETION_SCHEMA
        || completion.snapshot_id != manifest.snapshot_id
        || completion.manifest_sha256 != sha256(manifest_bytes)
        || completion.manifest_length != manifest_bytes.len() as u64
        || completion.entry_count != manifest.entry_count
        || completion.total_bytes != manifest.total_bytes
        || completion.created_at_unix_ms == 0
    {
        return Err(BackupError::Verification(
            "snapshot completion marker does not bind the manifest".to_owned(),
        ));
    }
    validate_sha256(&completion.manifest_sha256)
}

fn validate_protection_label(label: &str) -> Result<(), BackupError> {
    let scalar_count = label.chars().count();
    if scalar_count == 0
        || scalar_count > 256
        || label.trim() != label
        || label.chars().any(char::is_control)
    {
        return Err(BackupError::InvalidManifest(
            "snapshot protection label must contain 1..=256 non-control Unicode scalars without surrounding whitespace"
                .to_owned(),
        ));
    }
    Ok(())
}

fn read_snapshot_protection_internal(
    snapshot_directory: &Path,
    expected_snapshot_id: Uuid,
) -> Result<Option<SnapshotProtection>, BackupError> {
    let path = snapshot_directory.join(SNAPSHOT_PROTECTION_FILE);
    if !path.try_exists().map_err(BackupError::Io)? {
        return Ok(None);
    }
    let bytes = read_bounded_regular_file(&path, snapshot_directory, 1024 * 1024)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(BackupError::Json)?;
    let raw_snapshot_id = value
        .get("snapshotId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            BackupError::InvalidManifest(
                "snapshot protection snapshotId must be a string".to_owned(),
            )
        })?;
    let expected_text = expected_snapshot_id.hyphenated().to_string();
    if raw_snapshot_id != expected_text {
        return Err(BackupError::Verification(
            "snapshot protection does not bind the containing snapshot".to_owned(),
        ));
    }
    let protection: SnapshotProtection =
        serde_json::from_value(value).map_err(BackupError::Json)?;
    if protection.schema != SNAPSHOT_PROTECTION_SCHEMA
        || protection.snapshot_id != expected_snapshot_id
        || protection.protected_at_unix_ms == 0
    {
        return Err(BackupError::Verification(
            "snapshot protection record is invalid".to_owned(),
        ));
    }
    validate_protection_label(&protection.label)?;
    Ok(Some(protection))
}

fn read_bounded_regular_file(
    path: &Path,
    canonical_root: &Path,
    limit: u64,
) -> Result<Vec<u8>, BackupError> {
    let metadata = fs::symlink_metadata(path).map_err(BackupError::Io)?;
    if linked_or_reparse(&metadata) || !metadata.is_file() {
        return Err(BackupError::LinkedPath(path.to_path_buf()));
    }
    ensure_resolved_inside(path, canonical_root)?;
    if metadata.len() > limit {
        return Err(BackupError::InvalidManifest(format!(
            "snapshot metadata exceeds {limit} bytes"
        )));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(BackupError::Io)?
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(BackupError::Io)?;
    if bytes.len() as u64 > limit || bytes.len() as u64 != metadata.len() {
        return Err(BackupError::ConcurrentChange(path.to_path_buf()));
    }
    Ok(bytes)
}

fn validate_restore_plan(plan: &AlternateRestorePlan) -> Result<(), BackupError> {
    if plan.schema != RESTORE_PLAN_SCHEMA {
        return Err(BackupError::InvalidPlan);
    }
    validate_v4(plan.restore_id, "restore")?;
    validate_v4(plan.snapshot_id, "snapshot")?;
    validate_sha256(&plan.manifest_sha256)?;
    let canonical_snapshot =
        canonical_existing_directory(&plan.snapshot_directory, "snapshot directory")?;
    let expected_workspace_root = canonical_snapshot
        .join(SNAPSHOT_CONTENT_DIRECTORY)
        .join(&plan.root_name);
    if canonical_snapshot != plan.snapshot_directory
        || expected_workspace_root != plan.snapshot_workspace_root
        || path_binding(&canonical_snapshot)? != plan.snapshot_binding
    {
        return Err(BackupError::InvalidPlan);
    }
    let normalized_destination = normalize_new_destination(&plan.destination_root)?;
    if normalized_destination != plan.destination_root
        || new_path_binding(&normalized_destination)? != plan.destination_binding
    {
        return Err(BackupError::InvalidPlan);
    }
    if plan.entries.len() as u64 != plan.entry_count
        || total_file_bytes(&plan.entries)? != plan.total_bytes
        || normalized_destination
            .file_name()
            .and_then(|name| name.to_str())
            != Some(plan.root_name.as_str())
        || restore_plan_digest(
            plan.restore_id,
            &plan.manifest_sha256,
            &plan.snapshot_binding,
            &plan.destination_binding,
        ) != plan.plan_digest
    {
        return Err(BackupError::InvalidPlan);
    }
    Ok(())
}

fn validate_restore_drill_plan(plan: &RestoreDrillPlan) -> Result<(), BackupError> {
    if plan.schema != RESTORE_DRILL_PLAN_SCHEMA {
        return Err(BackupError::InvalidPlan);
    }
    validate_v4(plan.drill_id, "restore drill")?;
    validate_v4(plan.snapshot_id, "snapshot")?;
    validate_sha256(&plan.manifest_sha256)?;
    let snapshot = canonical_existing_directory(&plan.snapshot_directory, "snapshot directory")?;
    let drill_parent = canonical_existing_directory(&plan.drill_parent, "drill parent")?;
    let results_parent =
        canonical_existing_directory(&plan.results_parent, "drill results parent")?;
    let expected_drill_directory = drill_parent.join(format!(
        "{RESTORE_DRILL_DIRECTORY_PREFIX}{}",
        plan.drill_id.hyphenated()
    ));
    let expected_result_file = results_parent.join(format!(
        "{RESTORE_DRILL_DIRECTORY_PREFIX}{}.json",
        plan.drill_id.hyphenated()
    ));
    if snapshot != plan.snapshot_directory
        || drill_parent != plan.drill_parent
        || results_parent != plan.results_parent
        || expected_drill_directory != plan.drill_directory
        || plan.destination_root != expected_drill_directory.join(&plan.root_name)
        || expected_result_file != plan.result_file
        || path_binding(&snapshot)? != plan.snapshot_binding
        || path_binding(&drill_parent)? != plan.drill_parent_binding
        || path_binding(&results_parent)? != plan.results_parent_binding
        || restore_drill_plan_digest(
            plan.drill_id,
            &plan.manifest_sha256,
            &plan.snapshot_binding,
            &plan.drill_parent_binding,
            &plan.results_parent_binding,
        ) != plan.plan_digest
    {
        return Err(BackupError::InvalidPlan);
    }
    if plan.drill_directory.exists() || plan.result_file.exists() {
        return Err(BackupError::StalePreview);
    }
    Ok(())
}

fn same_restore_drill_plan(left: &RestoreDrillPlan, right: &RestoreDrillPlan) -> bool {
    left.schema == right.schema
        && left.drill_id == right.drill_id
        && left.plan_digest == right.plan_digest
        && left.snapshot_id == right.snapshot_id
        && left.snapshot_directory == right.snapshot_directory
        && left.drill_parent == right.drill_parent
        && left.results_parent == right.results_parent
        && left.drill_directory == right.drill_directory
        && left.destination_root == right.destination_root
        && left.result_file == right.result_file
        && left.workspace_root_id == right.workspace_root_id
        && left.workspace_revision == right.workspace_revision
        && left.manifest_sha256 == right.manifest_sha256
        && left.entry_count == right.entry_count
        && left.total_bytes == right.total_bytes
        && left.root_name == right.root_name
        && left.snapshot_binding == right.snapshot_binding
        && left.drill_parent_binding == right.drill_parent_binding
        && left.results_parent_binding == right.results_parent_binding
}

fn commit_restore_contents(
    plan: &AlternateRestorePlan,
) -> Result<AlternateRestoreReceipt, BackupError> {
    if plan.destination_root.exists() {
        return Err(BackupError::RestoreTargetExists(
            plan.destination_root.clone(),
        ));
    }
    let source_root = plan.snapshot_workspace_root.clone();
    fs::create_dir(&plan.destination_root).map_err(|error| {
        if error.kind() == io::ErrorKind::AlreadyExists {
            BackupError::RestoreTargetExists(plan.destination_root.clone())
        } else {
            BackupError::Io(error)
        }
    })?;

    let destination_root = fs::canonicalize(&plan.destination_root).map_err(BackupError::Io)?;
    if destination_root != plan.destination_root {
        return Err(BackupError::PathEscape(destination_root));
    }
    for entry in &plan.entries {
        let destination = safe_join(&destination_root, &entry.locator)?;
        match entry.entry_type {
            BackupEntryType::Directory => {
                fs::create_dir(&destination).map_err(BackupError::Io)?;
            }
            BackupEntryType::File => {
                let source = safe_join(&source_root, &entry.locator)?;
                copy_regular_file_exact(
                    &source,
                    &source_root,
                    &destination,
                    &destination_root,
                    entry,
                )?;
            }
        }
    }
    sync_recorded_directories(&destination_root, &plan.entries)?;
    let verified_snapshot = verify_snapshot_internal(&plan.snapshot_directory)?;
    if verified_snapshot.snapshot_id != plan.snapshot_id
        || verified_snapshot.manifest_sha256 != plan.manifest_sha256
    {
        return Err(BackupError::StalePreview);
    }
    verify_restored_workspace(plan, &source_root, &destination_root)?;
    let parent = destination_root
        .parent()
        .ok_or_else(|| BackupError::Path("restored workspace has no parent".to_owned()))?;
    sync_directory(parent)?;
    verify_restored_workspace(plan, &source_root, &destination_root)?;

    Ok(AlternateRestoreReceipt {
        schema: "weftext.full-workspace-restore-receipt.v1".to_owned(),
        restore_id: plan.restore_id,
        snapshot_id: plan.snapshot_id,
        destination_root,
        workspace_root_id: plan.workspace_root_id,
        workspace_revision: plan.workspace_revision.clone(),
        manifest_sha256: plan.manifest_sha256.clone(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
        bytewise_verified: true,
    })
}

fn verify_restored_workspace(
    plan: &AlternateRestorePlan,
    source_root: &Path,
    destination_root: &Path,
) -> Result<(), BackupError> {
    let restored_id = workspace_root_id(destination_root)?;
    let restored_revision = read_workspace_revision(destination_root)?;
    let restored_entries = collect_stable_physical_entries(destination_root)?;
    if restored_id != plan.workspace_root_id
        || restored_revision != plan.workspace_revision
        || restored_entries != plan.entries
    {
        return Err(BackupError::Verification(
            "restored workspace identity, revision, or physical inventory differs".to_owned(),
        ));
    }
    verify_bytewise_trees(source_root, destination_root, &plan.entries)
}

fn normalize_new_destination(path: &Path) -> Result<PathBuf, BackupError> {
    reject_linked_existing_ancestors(path)?;
    match fs::symlink_metadata(path) {
        Ok(_) => return Err(BackupError::RestoreTargetExists(path.to_path_buf())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(BackupError::Io(error)),
    }
    let parent = path
        .parent()
        .ok_or_else(|| BackupError::Path("restore destination has no parent".to_owned()))?;
    let canonical_parent = canonical_existing_directory(parent, "restore destination parent")?;
    reject_workspace_marker_ancestor(&canonical_parent, "restore destination parent")?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| BackupError::Path("restore destination name must be UTF-8".to_owned()))?;
    if name.is_empty() || matches!(name, "." | "..") || name.contains(['/', '\\', '\0']) {
        return Err(BackupError::Path(
            "restore destination name is not portable".to_owned(),
        ));
    }
    Ok(canonical_parent.join(name))
}

fn reject_workspace_marker_ancestor(directory: &Path, label: &str) -> Result<(), BackupError> {
    for ancestor in directory.ancestors() {
        let marker = ancestor.join(weftext_core::WORKSPACE_FORMAT_MARKER_FILE);
        match fs::symlink_metadata(&marker) {
            Ok(metadata) => {
                if linked_or_reparse(&metadata) {
                    return Err(BackupError::LinkedPath(marker));
                }
                return Err(BackupError::Path(format!(
                    "{label} must be outside every Weftext workspace root; found {}",
                    marker.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(BackupError::Io(error)),
        }
    }
    Ok(())
}

fn canonical_existing_directory(path: &Path, label: &str) -> Result<PathBuf, BackupError> {
    let metadata = fs::symlink_metadata(path).map_err(BackupError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(BackupError::LinkedPath(path.to_path_buf()));
    }
    if !metadata.is_dir() {
        return Err(BackupError::Path(format!(
            "{label} must be an existing directory"
        )));
    }
    let canonical = fs::canonicalize(path).map_err(BackupError::Io)?;
    reject_linked_existing_ancestors(&canonical)?;
    Ok(canonical)
}

fn reject_linked_existing_ancestors(path: &Path) -> Result<(), BackupError> {
    let absolute;
    let path = if path.is_absolute() {
        path
    } else {
        absolute = std::env::current_dir().map_err(BackupError::Io)?.join(path);
        &absolute
    };
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if linked_or_reparse(&metadata) => {
                return Err(BackupError::LinkedPath(ancestor.to_path_buf()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(BackupError::Io(error)),
        }
    }
    Ok(())
}

fn linked_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn safe_join(root: &Path, locator: &str) -> Result<PathBuf, BackupError> {
    validate_locator(locator)?;
    let mut path = root.to_path_buf();
    for component in locator.split('/') {
        path.push(component);
    }
    Ok(path)
}

fn path_binding(path: &Path) -> Result<String, BackupError> {
    let text = path
        .to_str()
        .ok_or_else(|| BackupError::NonUtf8Path(path.to_path_buf()))?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.backup.existing-path.v1\0");
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn new_path_binding(path: &Path) -> Result<String, BackupError> {
    let text = path
        .to_str()
        .ok_or_else(|| BackupError::NonUtf8Path(path.to_path_buf()))?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.backup.new-path.v1\0");
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn parse_canonical_v4(value: &str, label: &str) -> Result<Uuid, BackupError> {
    let parsed = Uuid::parse_str(value)
        .map_err(|_| BackupError::InvalidManifest(format!("{label} ID is not a UUID")))?;
    if parsed.get_version() != Some(Version::Random) || parsed.hyphenated().to_string() != value {
        return Err(BackupError::InvalidManifest(format!(
            "{label} ID must be a canonical lowercase UUIDv4"
        )));
    }
    Ok(parsed)
}

fn validate_v4(value: Uuid, label: &str) -> Result<(), BackupError> {
    if value.get_version() == Some(Version::Random) {
        Ok(())
    } else {
        Err(BackupError::InvalidManifest(format!(
            "{label} ID must be UUIDv4"
        )))
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn unix_time_ms() -> Result<u64, BackupError> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| BackupError::Verification("system time precedes the Unix epoch".to_owned()))?
        .as_millis();
    u64::try_from(millis).map_err(|_| {
        BackupError::Verification("system time is outside u64 milliseconds".to_owned())
    })
}

#[derive(Debug)]
pub enum BackupError {
    InvalidWorkspace(String),
    InvalidManifest(String),
    InvalidPlan,
    UnsupportedScope(String),
    UnknownSnapshotNode(NodeId),
    ScopedRestoreRootUnsupported(NodeId),
    RestoreIdentityConflict(NodeId),
    ScopedRestoreBoundary(String),
    ScopedRestoreBlocked(Vec<ScopedRestoreBlocker>),
    SnapshotExists(PathBuf),
    IncompleteSnapshot(PathBuf),
    RestoreTargetExists(PathBuf),
    LinkedPath(PathBuf),
    PathEscape(PathBuf),
    NonUtf8Path(PathBuf),
    UnsupportedEntry(PathBuf),
    UnfinishedTransaction(PathBuf),
    ConcurrentChange(PathBuf),
    StalePreview,
    Path(String),
    Verification(String),
    CoreTransaction(WorkspaceTransactionError),
    Revision(WorkspaceRevisionError),
    Json(serde_json::Error),
    Io(io::Error),
}

impl fmt::Display for BackupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(message) => write!(formatter, "invalid workspace: {message}"),
            Self::InvalidManifest(message) => {
                write!(formatter, "invalid backup manifest: {message}")
            }
            Self::InvalidPlan => {
                formatter.write_str("backup or restore plan is invalid or tampered")
            }
            Self::UnsupportedScope(message) => {
                write!(formatter, "unsupported backup scope: {message}")
            }
            Self::UnknownSnapshotNode(node_id) => {
                write!(formatter, "snapshot node is unavailable: {node_id}")
            }
            Self::ScopedRestoreRootUnsupported(node_id) => write!(
                formatter,
                "workspace root {node_id} requires full-workspace restore, not scoped restore"
            ),
            Self::RestoreIdentityConflict(node_id) => write!(
                formatter,
                "scoped restore identity already exists in the target workspace: {node_id}"
            ),
            Self::ScopedRestoreBoundary(locator) => write!(
                formatter,
                "scoped restore refuses unmanaged, ignored, reserved, or unowned content at {locator}"
            ),
            Self::ScopedRestoreBlocked(blockers) => {
                let codes = blockers
                    .iter()
                    .map(|blocker| format!("{:?}", blocker.code))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(
                    formatter,
                    "scoped restore is blocked by missing Core transaction capability: {codes}"
                )
            }
            Self::SnapshotExists(path) => {
                write!(
                    formatter,
                    "create-new snapshot already exists: {}",
                    path.display()
                )
            }
            Self::IncompleteSnapshot(path) => write!(
                formatter,
                "snapshot has no marker-last completion record: {}",
                path.display()
            ),
            Self::RestoreTargetExists(path) => write!(
                formatter,
                "restore target must not already exist: {}",
                path.display()
            ),
            Self::LinkedPath(path) => write!(
                formatter,
                "backup refuses a symlink, junction, or reparse path: {}",
                path.display()
            ),
            Self::PathEscape(path) => {
                write!(
                    formatter,
                    "backup path escapes its root: {}",
                    path.display()
                )
            }
            Self::NonUtf8Path(path) => {
                write!(formatter, "backup path is not UTF-8: {}", path.display())
            }
            Self::UnsupportedEntry(path) => write!(
                formatter,
                "backup entry is not a regular file or directory: {}",
                path.display()
            ),
            Self::UnfinishedTransaction(path) => write!(
                formatter,
                "unfinished workspace transaction blocks backup: {}",
                path.display()
            ),
            Self::ConcurrentChange(path) => write!(
                formatter,
                "filesystem changed during backup verification: {}",
                path.display()
            ),
            Self::StalePreview => formatter.write_str("backup or restore preview is stale"),
            Self::Path(message) => formatter.write_str(message),
            Self::Verification(message) => {
                write!(formatter, "backup verification failed: {message}")
            }
            Self::CoreTransaction(error) => {
                write!(
                    formatter,
                    "Core workspace safety boundary rejected backup or restore: {error}"
                )
            }
            Self::Revision(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "backup JSON is invalid: {error}"),
            Self::Io(error) => write!(formatter, "backup I/O failed: {error}"),
        }
    }
}

impl std::error::Error for BackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CoreTransaction(error) => Some(error),
            Self::Revision(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<WorkspaceRevisionError> for BackupError {
    fn from(error: WorkspaceRevisionError) -> Self {
        Self::Revision(error)
    }
}

#[cfg(test)]
mod retention_recovery_tests {
    use super::*;

    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temporary = tempfile::tempdir().expect("temporary retention fixture");
        let workspace = temporary.path().join("workspace");
        let backup_parent = temporary.path().join("backups");
        fs::create_dir(&backup_parent).expect("backup parent");
        weftext_core::create_workspace(&workspace).expect("workspace");
        (temporary, workspace, backup_parent)
    }

    fn create_snapshot(workspace: &Path, backup_parent: &Path) -> FullWorkspaceBackupPlan {
        let plan = plan_full_workspace_backup(workspace, backup_parent).expect("backup preview");
        commit_full_workspace_backup(&plan).expect("backup commit");
        plan
    }

    #[test]
    fn shared_workspace_source_inventory_revalidates_anchor_after_semantic_checks() {
        let temporary = tempfile::tempdir().expect("temporary source inventory fixture");
        let workspace = temporary.path().join("Workspace");
        weftext_core::create_workspace(&workspace).expect("workspace");
        let workspace = fs::canonicalize(workspace).expect("canonical workspace");
        let workspace_lease =
            acquire_core_workspace_transaction_lease(&workspace).expect("workspace lease");
        let displaced = temporary.path().join("held-workspace-anchor");

        let result = stable_workspace_inventory_with_post_capture_probe(
            &workspace,
            &workspace_lease,
            || {
                fs::rename(
                    workspace.join(weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
                    &displaced,
                )
                .map_err(BackupError::Io)?;
                fs::write(
                    workspace.join(weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
                    [],
                )
                .map_err(BackupError::Io)?;
                assert!(acquire_core_workspace_transaction_lease(&workspace).is_ok());
                Ok(())
            },
        );
        assert!(matches!(
            result,
            Err(BackupError::CoreTransaction(
                WorkspaceTransactionError::RecoveryRequired(_)
            ))
        ));
    }

    #[test]
    fn full_workspace_v1_manifest_digest_is_a_compatibility_golden() {
        let temporary = tempfile::tempdir().expect("temporary golden fixture");
        let workspace = temporary.path().join("Golden");
        let backup_parent = temporary.path().join("backups");
        fs::create_dir(&workspace).expect("workspace root");
        fs::create_dir(&backup_parent).expect("backup parent");
        fs::write(
            workspace.join(weftext_core::WORKSPACE_FORMAT_MARKER_FILE),
            weftext_core::ASCIIDOC_V1_MARKER,
        )
        .expect("format marker");
        fs::write(
            workspace.join("Golden.adoc"),
            concat!(
                "---\n",
                "weftext:\n",
                "  id: \"11111111-1111-4111-8111-111111111111\"\n",
                "---\n",
                "= Golden\n"
            ),
        )
        .expect("root document");
        fs::write(
            workspace.join(".weftext-rules"),
            "weftext-content-rules-v1\nignore ignored/\n",
        )
        .expect("rules");
        fs::create_dir_all(workspace.join("ignored/empty")).expect("ignored empty directory");
        fs::write(workspace.join("ignored/zero.bin"), []).expect("zero-byte ignored file");
        fs::create_dir(workspace.join(".git")).expect("git directory");
        fs::write(workspace.join(".git/config"), b"[core]\n\tbare = false\n").expect("git config");

        let snapshot_id = Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
        let plan = replan_full_workspace_backup(&workspace, &backup_parent, snapshot_id)
            .expect("deterministic backup preview");
        let manifest = manifest_for_backup_plan(&plan);
        let bytes = manifest_bytes(&manifest).expect("canonical v1 manifest bytes");
        assert_eq!(sha256(&bytes), plan.manifest_sha256);
        assert_eq!(
            plan.manifest_sha256,
            "0e391f408475e483c08c72e2bccdb84aa7c5c9c95bb1db0da41322f60b448193"
        );
        assert_eq!(manifest.schema, SNAPSHOT_MANIFEST_SCHEMA);
        for directory in manifest
            .entries
            .iter()
            .filter(|entry| entry.entry_type == BackupEntryType::Directory)
        {
            assert_eq!(directory.sha256, directory_sha256(&directory.locator));
        }
    }

    #[test]
    fn full_workspace_v1_receipt_serialization_is_a_compatibility_golden() {
        let receipt = FullWorkspaceBackupReceipt {
            schema: "weftext.full-workspace-backup-receipt.v1".to_owned(),
            snapshot_id: Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap(),
            snapshot_directory: PathBuf::from(
                "/snapshots/weftext-backup-22222222-2222-4222-8222-222222222222",
            ),
            workspace_root_id: NodeId::from_str("11111111-1111-4111-8111-111111111111").unwrap(),
            workspace_revision: WorkspaceRevision::parse(&"a".repeat(64)).unwrap(),
            manifest_sha256: "b".repeat(64),
            entry_count: 7,
            total_bytes: 19,
            verified: true,
        };
        let bytes = serde_json::to_vec(&receipt).unwrap();
        assert_eq!(
            bytes,
            r#"{"schema":"weftext.full-workspace-backup-receipt.v1","snapshotId":"22222222-2222-4222-8222-222222222222","snapshotDirectory":"/snapshots/weftext-backup-22222222-2222-4222-8222-222222222222","workspaceRootId":"11111111-1111-4111-8111-111111111111","workspaceRevision":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","manifestSha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","entryCount":7,"totalBytes":19,"verified":true}"#.as_bytes()
        );
    }

    fn prepare_interrupted_transaction(
        plan: &SnapshotRetentionPlan,
        moved_entries: usize,
        committed: bool,
    ) -> (
        PathBuf,
        SnapshotRetentionJournal,
        Option<SnapshotRetentionReceipt>,
    ) {
        fs::create_dir(retention_lock_path(&plan.backup_parent)).expect("retention lock");
        let transaction = retention_transaction_path(&plan.backup_parent, plan.operation_id);
        fs::create_dir(&transaction).expect("transaction");
        let journal = retention_journal_from_plan(plan);
        write_json_new(&transaction.join(RETENTION_JOURNAL_FILE), &journal).expect("journal");
        let holding = transaction.join(RETENTION_HOLDING_DIRECTORY);
        fs::create_dir(&holding).expect("holding");
        for entry in journal.entries.iter().take(moved_entries) {
            fs::rename(
                plan.backup_parent.join(&entry.directory_name),
                holding.join(&entry.directory_name),
            )
            .expect("interrupted move");
        }
        let receipt = if committed {
            assert_eq!(moved_entries, journal.entries.len());
            let receipt = retention_receipt_from_plan(plan).expect("receipt");
            write_json_new(&transaction.join(RETENTION_COMMIT_FILE), &receipt)
                .expect("commit marker");
            Some(receipt)
        } else {
            None
        };
        (transaction, journal, receipt)
    }

    #[test]
    fn pre_marker_interruption_rolls_every_selected_snapshot_back() {
        let (_temporary, workspace, backup_parent) = fixture();
        let first = create_snapshot(&workspace, &backup_parent);
        let second = create_snapshot(&workspace, &backup_parent);
        let plan = plan_snapshot_retention(
            &backup_parent,
            SnapshotRetentionPolicy {
                keep_latest_unprotected: 0,
            },
        )
        .expect("retention plan");
        let (transaction, journal, _) = prepare_interrupted_transaction(&plan, 1, false);
        let not_yet_moved = backup_parent.join(&journal.entries[1].directory_name);
        assert!(matches!(
            protect_full_workspace_snapshot(&not_yet_moved, "must wait"),
            Err(BackupError::UnfinishedTransaction(_))
        ));

        let recovered = recover_snapshot_retention(&backup_parent).expect("rollback recovery");
        assert_eq!(recovered.rolled_back_operation_ids, vec![plan.operation_id]);
        assert!(recovered.finalized_operation_ids.is_empty());
        assert!(!transaction.exists());
        assert!(!plan.receipt_file.exists());
        assert!(verify_full_workspace_snapshot(&first.snapshot_directory).is_ok());
        assert!(verify_full_workspace_snapshot(&second.snapshot_directory).is_ok());
        assert!(
            protect_full_workspace_snapshot(&second.snapshot_directory, "after recovery").is_ok()
        );
        assert_eq!(journal.entries.len(), 2);
    }

    #[test]
    fn post_marker_partial_cleanup_never_resurrects_committed_prunes() {
        let (_temporary, workspace, backup_parent) = fixture();
        let first = create_snapshot(&workspace, &backup_parent);
        let second = create_snapshot(&workspace, &backup_parent);
        let plan = plan_snapshot_retention(
            &backup_parent,
            SnapshotRetentionPolicy {
                keep_latest_unprotected: 0,
            },
        )
        .expect("retention plan");
        let (transaction, journal, receipt) =
            prepare_interrupted_transaction(&plan, plan.pruned.len(), true);
        let receipt = receipt.expect("committed receipt");
        write_json_new(&plan.receipt_file, &receipt).expect("durable receipt before crash");
        let first_holding = transaction
            .join(RETENTION_HOLDING_DIRECTORY)
            .join(&journal.entries[0].directory_name);
        remove_verified_tree(&first_holding, &transaction).expect("partial cleanup");

        let recovered = recover_snapshot_retention(&backup_parent).expect("finalize recovery");
        assert!(recovered.rolled_back_operation_ids.is_empty());
        assert_eq!(recovered.finalized_operation_ids, vec![plan.operation_id]);
        assert!(!transaction.exists());
        assert!(!first.snapshot_directory.exists());
        assert!(!second.snapshot_directory.exists());
        assert_eq!(
            read_snapshot_retention_receipt(&plan.receipt_file).unwrap(),
            receipt
        );
    }
}
