//! Paired, fail-closed backup and alternate restore for the Server control plane.

mod pair;

pub use pair::*;

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::backup::{Backup, StepResult};
use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};

use super::{
    BackupError, VerifiedSnapshot, acquire_core_workspace_transaction_lease,
    canonical_existing_directory, collect_workspace_physical_entries, linked_or_reparse,
    new_path_binding, normalize_new_destination, path_binding, read_bounded_regular_file,
    reject_linked_existing_ancestors, reject_workspace_marker_ancestor, sha256, sync_directory,
    unix_time_ms, verify_bytewise_trees, verify_snapshot_internal, write_new_file,
};

pub const SERVER_CONTROL_PLANE_BACKUP_MANIFEST_SCHEMA: &str =
    "weftext.server-control-plane-backup.v1";
pub const SERVER_CONTROL_PLANE_BACKUP_COMPLETION_SCHEMA: &str =
    "weftext.server-control-plane-backup-completion.v1";
pub const SERVER_CONTROL_PLANE_BACKUP_PLAN_SCHEMA: &str =
    "weftext.server-control-plane-backup-plan.v1";
pub const SERVER_CONTROL_PLANE_BACKUP_RECEIPT_SCHEMA: &str =
    "weftext.server-control-plane-backup-receipt.v1";
pub const SERVER_CONTROL_PLANE_RESTORE_PLAN_SCHEMA: &str =
    "weftext.server-control-plane-restore-plan.v1";
pub const SERVER_CONTROL_PLANE_RESTORE_RECEIPT_SCHEMA: &str =
    "weftext.server-control-plane-restore-receipt.v1";
pub const SERVER_CONTROL_PLANE_RESTORE_COMPLETION_SCHEMA: &str =
    "weftext.server-control-plane-restore-completion.v1";
pub const SERVER_CONTROL_PLANE_DATABASE_FILE: &str = "control-plane.sqlite3";
pub const SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE: &str = "bootstrap-secret";
pub const SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE: &str = "reverse-proxy-secret";
pub const SERVER_CONTROL_PLANE_LEASE_FILE: &str = ".weftext-server-control-plane.lease";
pub const SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE: &str = "restore-receipt.json";
pub const SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE: &str = "restore-complete.json";
pub const SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE: &str = "manifest.json";
pub const SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE: &str = "complete.json";

const CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX: &str = "weftext-control-plane-backup-";
const SERVER_CONTROL_PLANE_DATABASE_WAL_FILE: &str = "control-plane.sqlite3-wal";
const SERVER_CONTROL_PLANE_DATABASE_SHM_FILE: &str = "control-plane.sqlite3-shm";
const SERVER_CONTROL_PLANE_DATABASE_JOURNAL_FILE: &str = "control-plane.sqlite3-journal";
const CONTROL_PLANE_RESTORE_STAGING_PREFIX: &str = ".__weftext-control-plane-restore-";
const MAX_CONTROL_METADATA_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CONTROL_DATABASE_BYTES: u64 = 8 * 1024 * 1024 * 1024;
const WORKSPACE_SCOPE_KEY: &str = "workspace_scope_v1";
const BOOTSTRAP_DIGEST_KEY: &str = "bootstrap_secret_digest";

const CONTROL_TABLES: &[&str] = &[
    "accounts",
    "audit_outbox",
    "audit_receipts",
    "authorization_epochs",
    "collaboration_documents",
    "collaboration_pending",
    "collaboration_receipts",
    "metadata",
    "node_acl",
    "owner",
    "security_events",
    "sessions",
];

const PRESERVED_ON_RESTORE_TABLES: &[&str] = &[
    "accounts",
    "audit_outbox",
    "audit_receipts",
    "authorization_epochs",
    "collaboration_documents",
    "collaboration_pending",
    "collaboration_receipts",
    "metadata",
    "node_acl",
    "owner",
    "security_events",
];

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapSecretBackupPolicy {
    ConsumedRequiredSecretExcluded,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverseProxySecretBackupPolicy {
    ExcludedRegenerateAndRotateAtRuntime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRestorePolicy {
    InvalidateAll,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairedWorkspaceSnapshot {
    pub snapshot_id: Uuid,
    pub manifest_sha256: String,
    pub workspace_root_id: String,
    pub workspace_revision: String,
    pub entry_count: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlPlaneTableEvidence {
    pub name: String,
    pub row_count: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlPlaneDatabaseEvidence {
    pub byte_length: u64,
    pub sha256: String,
    pub schema_sha256: String,
    pub workspace_scope_sha256: String,
    pub application_id: i64,
    pub user_version: i64,
    pub tables: Vec<ControlPlaneTableEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerControlPlaneBackupManifest {
    pub schema: String,
    pub backup_id: Uuid,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub database: ControlPlaneDatabaseEvidence,
    pub bootstrap_secret_policy: BootstrapSecretBackupPolicy,
    pub reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    pub session_restore_policy: SessionRestorePolicy,
    pub preserved_on_restore_tables: Vec<String>,
    pub excluded_operational_files: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerControlPlaneBackupCompletion {
    schema: String,
    backup_id: Uuid,
    workspace_snapshot_id: Uuid,
    workspace_manifest_sha256: String,
    manifest_sha256: String,
    manifest_length: u64,
    database_sha256: String,
    database_length: u64,
    created_at_unix_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerControlPlaneBackupPlan {
    pub schema: String,
    pub backup_id: Uuid,
    pub plan_digest: String,
    pub control_plane_root: PathBuf,
    pub workspace_root: PathBuf,
    pub workspace_snapshot_directory: PathBuf,
    pub backup_parent: PathBuf,
    pub snapshot_directory: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub source_database: ControlPlaneDatabaseEvidence,
    pub bootstrap_secret_policy: BootstrapSecretBackupPolicy,
    pub reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    pub session_restore_policy: SessionRestorePolicy,
    pub preserved_on_restore_tables: Vec<String>,
    pub excluded_operational_files: Vec<String>,
    #[serde(skip)]
    control_plane_binding: String,
    #[serde(skip)]
    workspace_binding: String,
    #[serde(skip)]
    workspace_snapshot_binding: String,
    #[serde(skip)]
    destination_binding: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerControlPlaneBackupReceipt {
    pub schema: String,
    pub backup_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub manifest_sha256: String,
    pub manifest_length: u64,
    pub database_sha256: String,
    pub database_length: u64,
    pub verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerControlPlaneBackupVerification {
    pub schema: String,
    pub backup_id: Uuid,
    pub snapshot_directory: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub manifest_sha256: String,
    pub manifest_length: u64,
    pub database: ControlPlaneDatabaseEvidence,
    pub complete: bool,
    pub created_at_unix_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternateServerControlPlaneRestorePlan {
    pub schema: String,
    pub restore_id: Uuid,
    pub plan_digest: String,
    pub control_plane_snapshot_directory: PathBuf,
    pub workspace_snapshot_directory: PathBuf,
    pub restored_workspace_root: PathBuf,
    pub destination_control_plane_root: PathBuf,
    pub staging_control_plane_root: PathBuf,
    pub backup_id: Uuid,
    pub control_plane_manifest_sha256: String,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub source_database: ControlPlaneDatabaseEvidence,
    pub session_restore_policy: SessionRestorePolicy,
    pub reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    #[serde(skip)]
    control_snapshot_binding: String,
    #[serde(skip)]
    workspace_snapshot_binding: String,
    #[serde(skip)]
    restored_workspace_binding: String,
    #[serde(skip)]
    destination_binding: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DurableServerControlPlaneRestoreReceipt {
    schema: String,
    restore_id: Uuid,
    backup_id: Uuid,
    control_plane_manifest_sha256: String,
    workspace_snapshot: PairedWorkspaceSnapshot,
    database_sha256: String,
    database_length: u64,
    session_restore_policy: SessionRestorePolicy,
    sessions_invalidated: bool,
    reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    reverse_proxy_secret_present: bool,
    bootstrap_secret_present: bool,
    preserved_tables_verified: Vec<String>,
    completed_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DurableServerControlPlaneRestoreCompletion {
    schema: String,
    restore_id: Uuid,
    backup_id: Uuid,
    control_plane_manifest_sha256: String,
    workspace_snapshot_id: Uuid,
    workspace_manifest_sha256: String,
    receipt_sha256: String,
    receipt_length: u64,
    database_sha256: String,
    database_length: u64,
    completed_at_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct AlternateServerControlPlaneRestoreReceipt {
    pub schema: String,
    pub restore_id: Uuid,
    pub backup_id: Uuid,
    pub control_plane_manifest_sha256: String,
    pub destination_control_plane_root: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub database_sha256: String,
    pub database_length: u64,
    pub receipt_sha256: String,
    pub receipt_length: u64,
    pub sessions_invalidated: bool,
    pub reverse_proxy_secret_present: bool,
    pub bootstrap_secret_present: bool,
    pub preserved_tables_verified: Vec<String>,
    pub permissions_verified: bool,
    pub completed_at_unix_ms: u64,
}

#[derive(Debug)]
pub struct ServerControlPlaneLease {
    root: PathBuf,
    _file: File,
}

impl ServerControlPlaneLease {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug)]
pub enum ServerControlPlaneBackupError {
    Backup(BackupError),
    Database(rusqlite::Error),
    Json(serde_json::Error),
    Io(io::Error),
    InvalidControlPlane(String),
    ControlPlaneInUse(PathBuf),
    UninitializedControlPlane,
    SnapshotExists(PathBuf),
    IncompleteSnapshot(PathBuf),
    RestoreTargetExists(PathBuf),
    InvalidPlan,
    StalePreview,
    WorkspaceSnapshotMismatch,
    PairIncomplete {
        completed: PathBuf,
        pending: PathBuf,
        cause: String,
    },
    Verification(String),
}

impl fmt::Display for ServerControlPlaneBackupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backup(error) => error.fmt(formatter),
            Self::Database(error) => write!(formatter, "control-plane database error: {error}"),
            Self::Json(error) => write!(formatter, "control-plane backup JSON error: {error}"),
            Self::Io(error) => write!(formatter, "control-plane backup I/O error: {error}"),
            Self::InvalidControlPlane(message) => {
                write!(formatter, "invalid Server control plane: {message}")
            }
            Self::ControlPlaneInUse(path) => write!(
                formatter,
                "Server control plane is in use; exclusive lease unavailable: {}",
                path.display()
            ),
            Self::UninitializedControlPlane => formatter.write_str(
                "uninitialized Server control plane cannot be backed up; consume bootstrap first",
            ),
            Self::SnapshotExists(path) => write!(
                formatter,
                "create-new control-plane snapshot already exists: {}",
                path.display()
            ),
            Self::IncompleteSnapshot(path) => write!(
                formatter,
                "control-plane snapshot has no valid marker-last completion record: {}",
                path.display()
            ),
            Self::RestoreTargetExists(path) => write!(
                formatter,
                "alternate control-plane restore target already exists: {}",
                path.display()
            ),
            Self::InvalidPlan => {
                formatter.write_str("control-plane backup or restore plan is invalid or tampered")
            }
            Self::StalePreview => formatter.write_str("control-plane backup preview is stale"),
            Self::WorkspaceSnapshotMismatch => formatter.write_str(
                "portable workspace does not exactly match the paired verified snapshot",
            ),
            Self::PairIncomplete {
                completed,
                pending,
                cause,
            } => write!(
                formatter,
                "Server backup pair is incomplete: {} is complete, {} is pending; replay the same reviewed pair after fixing the cause: {cause}",
                completed.display(),
                pending.display()
            ),
            Self::Verification(message) => {
                write!(formatter, "control-plane verification failed: {message}")
            }
        }
    }
}

impl std::error::Error for ServerControlPlaneBackupError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Backup(error) => Some(error),
            Self::Database(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BackupError> for ServerControlPlaneBackupError {
    fn from(error: BackupError) -> Self {
        Self::Backup(error)
    }
}

impl From<rusqlite::Error> for ServerControlPlaneBackupError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<serde_json::Error> for ServerControlPlaneBackupError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<io::Error> for ServerControlPlaneBackupError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

struct InspectedControlPlane {
    database: ControlPlaneDatabaseEvidence,
    excluded_operational_files: Vec<String>,
}

struct VerifiedControlPlaneSnapshot {
    directory: PathBuf,
    manifest: ServerControlPlaneBackupManifest,
    manifest_sha256: String,
    manifest_length: u64,
    created_at_unix_ms: u64,
}

/// Acquires the cross-process lease shared by a running Server and backup tooling.
///
/// A Server process must retain this guard for its full lifetime. Backup planning and commit use
/// the same non-blocking exclusive lease, so a successful acquisition proves that a cooperating
/// Server is stopped rather than guessing from `SQLite` file state.
///
/// # Errors
///
/// Fails closed for an unsafe control-plane path, an unverifiable hard link or permission
/// boundary, or when another process already holds the lease.
pub fn acquire_server_control_plane_lease(
    control_plane_root: impl AsRef<Path>,
) -> Result<ServerControlPlaneLease, ServerControlPlaneBackupError> {
    let root = canonical_existing_directory(control_plane_root.as_ref(), "control-plane root")?;
    let lock_path = root.join(SERVER_CONTROL_PLANE_LEASE_FILE);
    reject_linked_existing_ancestors(&lock_path)?;
    if lock_path.exists() {
        reject_link_or_reparse(&lock_path)?;
    }

    let file = open_exclusive_lease(&lock_path)?;
    set_private_permissions(&lock_path, false)?;
    reject_link_or_hardlink(&lock_path)?;
    file.sync_all()?;
    sync_directory(&root)?;
    Ok(ServerControlPlaneLease { root, _file: file })
}

/// Builds a paired control-plane backup preview without writing the backup destination.
///
/// # Errors
///
/// Fails closed unless the Server is stopped, the control plane is initialized and private, and
/// the current physical workspace exactly matches the verified full-workspace snapshot.
pub fn plan_server_control_plane_backup(
    control_plane_root: impl AsRef<Path>,
    workspace_root: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
) -> Result<ServerControlPlaneBackupPlan, ServerControlPlaneBackupError> {
    replan_server_control_plane_backup(
        control_plane_root,
        workspace_root,
        workspace_snapshot_directory,
        backup_parent,
        Uuid::new_v4(),
    )
}

/// Rebuilds a preview for a caller-provided reviewed `UUIDv4` backup identity.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_server_control_plane_backup`] and rejects a
/// non-v4 backup identity.
pub fn replan_server_control_plane_backup(
    control_plane_root: impl AsRef<Path>,
    workspace_root: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
    backup_id: Uuid,
) -> Result<ServerControlPlaneBackupPlan, ServerControlPlaneBackupError> {
    require_v4(backup_id, "control-plane backup")?;
    let lease = acquire_server_control_plane_lease(control_plane_root)?;
    build_control_plane_backup_plan(
        &lease,
        workspace_root.as_ref(),
        workspace_snapshot_directory.as_ref(),
        backup_parent.as_ref(),
        backup_id,
        false,
    )
}

/// Rebuilds a control-plane preview while the owning Server has quiesced every API request and
/// lends its process-lifetime exclusive lease.
///
/// This is deliberately separate from the stopped-Server entry point: possession of a lease is
/// not itself quiescence, so only the Server runtime should call this after draining requests.
///
/// # Errors
///
/// Has the same fail-closed validation as [`replan_server_control_plane_backup`].
pub fn replan_server_control_plane_backup_with_lease(
    lease: &ServerControlPlaneLease,
    workspace_root: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
    backup_id: Uuid,
) -> Result<ServerControlPlaneBackupPlan, ServerControlPlaneBackupError> {
    require_v4(backup_id, "control-plane backup")?;
    build_control_plane_backup_plan(
        lease,
        workspace_root.as_ref(),
        workspace_snapshot_directory.as_ref(),
        backup_parent.as_ref(),
        backup_id,
        true,
    )
}

/// Commits the reviewed control-plane database through `SQLite`'s consistency backup API.
///
/// # Errors
///
/// Rejects stale or tampered plans, a live Server lease, destination collisions, concurrent
/// changes, unsafe permissions, and any consistency or reopen-verification failure.
pub fn commit_server_control_plane_backup(
    plan: &ServerControlPlaneBackupPlan,
) -> Result<ServerControlPlaneBackupReceipt, ServerControlPlaneBackupError> {
    let lease = acquire_server_control_plane_lease(&plan.control_plane_root)?;
    commit_server_control_plane_backup_internal(&lease, plan, false)
}

/// Commits a reviewed control-plane backup under the owning Server's already-held exclusive
/// lease. The caller must hold the Server-wide quiescence barrier for the whole call.
///
/// # Errors
///
/// Rejects a lease for another root and every stale/tampered condition rejected by
/// [`commit_server_control_plane_backup`].
pub fn commit_server_control_plane_backup_with_lease(
    lease: &ServerControlPlaneLease,
    plan: &ServerControlPlaneBackupPlan,
) -> Result<ServerControlPlaneBackupReceipt, ServerControlPlaneBackupError> {
    commit_server_control_plane_backup_internal(lease, plan, true)
}

#[expect(
    clippy::too_many_lines,
    reason = "the marker-last control-plane commit keeps its single reviewed verification sequence visible"
)]
fn commit_server_control_plane_backup_internal(
    lease: &ServerControlPlaneLease,
    plan: &ServerControlPlaneBackupPlan,
    allow_live_sqlite_sidecars: bool,
) -> Result<ServerControlPlaneBackupReceipt, ServerControlPlaneBackupError> {
    validate_control_plane_backup_plan_shape(plan)?;
    if lease.root() != plan.control_plane_root {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "borrowed lease does not bind the reviewed control-plane root".to_owned(),
        ));
    }
    let current = build_control_plane_backup_plan(
        lease,
        &plan.workspace_root,
        &plan.workspace_snapshot_directory,
        &plan.backup_parent,
        plan.backup_id,
        allow_live_sqlite_sidecars,
    )?;
    if !same_control_plane_backup_plan(plan, &current) {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    if plan.snapshot_directory.exists() {
        return Err(ServerControlPlaneBackupError::SnapshotExists(
            plan.snapshot_directory.clone(),
        ));
    }

    create_private_directory(&plan.snapshot_directory)?;
    let database_path = plan
        .snapshot_directory
        .join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    sqlite_consistent_copy(
        &plan
            .control_plane_root
            .join(SERVER_CONTROL_PLANE_DATABASE_FILE),
        &database_path,
    )?;
    set_private_permissions(&database_path, false)?;
    let database = inspect_database(&database_path)?;
    require_logical_database_match(&plan.source_database, &database, false)?;

    let manifest = ServerControlPlaneBackupManifest {
        schema: SERVER_CONTROL_PLANE_BACKUP_MANIFEST_SCHEMA.to_owned(),
        backup_id: plan.backup_id,
        workspace_snapshot: plan.workspace_snapshot.clone(),
        database,
        bootstrap_secret_policy: plan.bootstrap_secret_policy,
        reverse_proxy_secret_policy: plan.reverse_proxy_secret_policy,
        session_restore_policy: plan.session_restore_policy,
        preserved_on_restore_tables: plan.preserved_on_restore_tables.clone(),
        excluded_operational_files: plan.excluded_operational_files.clone(),
    };
    validate_control_plane_manifest(&manifest)?;
    let manifest_bytes = pretty_json_bytes(&manifest)?;
    let manifest_sha256 = sha256(&manifest_bytes);
    let manifest_path = plan
        .snapshot_directory
        .join(SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE);
    write_new_file(&manifest_path, &manifest_bytes)?;
    set_private_permissions(&manifest_path, false)?;

    require_workspace_matches_snapshot(
        &plan.workspace_root,
        &verify_snapshot_internal(&plan.workspace_snapshot_directory)?,
    )?;
    let source_after = inspect_control_plane_mode(lease, allow_live_sqlite_sidecars)?;
    if source_after.database != plan.source_database
        || source_after.excluded_operational_files != plan.excluded_operational_files
    {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }

    let completion = ServerControlPlaneBackupCompletion {
        schema: SERVER_CONTROL_PLANE_BACKUP_COMPLETION_SCHEMA.to_owned(),
        backup_id: plan.backup_id,
        workspace_snapshot_id: plan.workspace_snapshot.snapshot_id,
        workspace_manifest_sha256: plan.workspace_snapshot.manifest_sha256.clone(),
        manifest_sha256: manifest_sha256.clone(),
        manifest_length: manifest_bytes.len() as u64,
        database_sha256: manifest.database.sha256.clone(),
        database_length: manifest.database.byte_length,
        created_at_unix_ms: unix_time_ms()?,
    };
    let completion_bytes = pretty_json_bytes(&completion)?;
    let completion_path = plan
        .snapshot_directory
        .join(SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE);
    write_new_file(&completion_path, &completion_bytes)?;
    set_private_permissions(&completion_path, false)?;
    sync_directory(&plan.snapshot_directory)?;
    sync_directory(&plan.backup_parent)?;

    let verified = verify_server_control_plane_snapshot_internal(
        &plan.snapshot_directory,
        &plan.workspace_snapshot_directory,
    )?;
    if verified.manifest.backup_id != plan.backup_id || verified.manifest_sha256 != manifest_sha256
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "committed snapshot did not reopen with the reviewed identity".to_owned(),
        ));
    }
    Ok(ServerControlPlaneBackupReceipt {
        schema: SERVER_CONTROL_PLANE_BACKUP_RECEIPT_SCHEMA.to_owned(),
        backup_id: plan.backup_id,
        snapshot_directory: plan.snapshot_directory.clone(),
        workspace_snapshot: plan.workspace_snapshot.clone(),
        manifest_sha256,
        manifest_length: manifest_bytes.len() as u64,
        database_sha256: verified.manifest.database.sha256,
        database_length: verified.manifest.database.byte_length,
        verified: true,
    })
}

/// Reopens a control-plane snapshot and its paired full-workspace snapshot read-only.
///
/// # Errors
///
/// Rejects incomplete, linked, hard-linked, permission-unsafe, schema-incompatible, tampered, or
/// incorrectly paired snapshots.
pub fn verify_server_control_plane_snapshot(
    control_plane_snapshot_directory: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
) -> Result<ServerControlPlaneBackupVerification, ServerControlPlaneBackupError> {
    let verified = verify_server_control_plane_snapshot_internal(
        control_plane_snapshot_directory.as_ref(),
        workspace_snapshot_directory.as_ref(),
    )?;
    Ok(ServerControlPlaneBackupVerification {
        schema: "weftext.server-control-plane-backup-verification.v1".to_owned(),
        backup_id: verified.manifest.backup_id,
        snapshot_directory: verified.directory,
        workspace_snapshot: verified.manifest.workspace_snapshot,
        manifest_sha256: verified.manifest_sha256,
        manifest_length: verified.manifest_length,
        database: verified.manifest.database,
        complete: true,
        created_at_unix_ms: verified.created_at_unix_ms,
    })
}

/// Builds a read-only alternate control-plane restore paired to an exact restored workspace.
///
/// # Errors
///
/// Rejects an invalid snapshot pair, a workspace that differs from the recorded physical
/// snapshot, a linked path, an existing destination, or a destination overlapping either set.
pub fn plan_alternate_server_control_plane_restore(
    control_plane_snapshot_directory: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
    destination_control_plane_root: impl AsRef<Path>,
) -> Result<AlternateServerControlPlaneRestorePlan, ServerControlPlaneBackupError> {
    replan_alternate_server_control_plane_restore(
        control_plane_snapshot_directory,
        workspace_snapshot_directory,
        restored_workspace_root,
        destination_control_plane_root,
        Uuid::new_v4(),
    )
}

/// Rebuilds the same alternate restore preview for a reviewed `UUIDv4` identity.
///
/// # Errors
///
/// Has the same fail-closed behavior as [`plan_alternate_server_control_plane_restore`] and
/// rejects a non-v4 restore identity.
pub fn replan_alternate_server_control_plane_restore(
    control_plane_snapshot_directory: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
    destination_control_plane_root: impl AsRef<Path>,
    restore_id: Uuid,
) -> Result<AlternateServerControlPlaneRestorePlan, ServerControlPlaneBackupError> {
    require_v4(restore_id, "control-plane restore")?;
    let verified = verify_server_control_plane_snapshot_internal(
        control_plane_snapshot_directory.as_ref(),
        workspace_snapshot_directory.as_ref(),
    )?;
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory.as_ref())?;
    let restored_workspace_root =
        canonical_existing_directory(restored_workspace_root.as_ref(), "restored workspace root")?;
    require_workspace_matches_snapshot(&restored_workspace_root, &workspace_snapshot)?;
    let destination_control_plane_root =
        normalize_new_destination(destination_control_plane_root.as_ref())?;
    require_disjoint(&destination_control_plane_root, &restored_workspace_root)?;
    require_disjoint(&destination_control_plane_root, &verified.directory)?;
    require_disjoint(
        &destination_control_plane_root,
        &workspace_snapshot.directory,
    )?;
    let parent = destination_control_plane_root.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "restore destination has no parent".to_owned(),
        )
    })?;
    let staging_control_plane_root = parent.join(format!(
        "{CONTROL_PLANE_RESTORE_STAGING_PREFIX}{}.staging",
        restore_id.hyphenated()
    ));
    reject_linked_existing_ancestors(&staging_control_plane_root)?;
    if staging_control_plane_root.exists() {
        return Err(ServerControlPlaneBackupError::RestoreTargetExists(
            staging_control_plane_root,
        ));
    }

    let control_snapshot_binding = path_binding(&verified.directory)?;
    let workspace_snapshot_binding = path_binding(&workspace_snapshot.directory)?;
    let restored_workspace_binding = path_binding(&restored_workspace_root)?;
    let destination_binding = new_path_binding(&destination_control_plane_root)?;
    let plan_digest = restore_plan_digest(
        restore_id,
        verified.manifest.backup_id,
        &verified.manifest_sha256,
        &verified.manifest.workspace_snapshot,
        &control_snapshot_binding,
        &workspace_snapshot_binding,
        &restored_workspace_binding,
        &destination_binding,
    );
    Ok(AlternateServerControlPlaneRestorePlan {
        schema: SERVER_CONTROL_PLANE_RESTORE_PLAN_SCHEMA.to_owned(),
        restore_id,
        plan_digest,
        control_plane_snapshot_directory: verified.directory,
        workspace_snapshot_directory: workspace_snapshot.directory,
        restored_workspace_root,
        destination_control_plane_root,
        staging_control_plane_root,
        backup_id: verified.manifest.backup_id,
        control_plane_manifest_sha256: verified.manifest_sha256,
        workspace_snapshot: verified.manifest.workspace_snapshot,
        source_database: verified.manifest.database,
        session_restore_policy: SessionRestorePolicy::InvalidateAll,
        reverse_proxy_secret_policy:
            ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime,
        control_snapshot_binding,
        workspace_snapshot_binding,
        restored_workspace_binding,
        destination_binding,
    })
}

/// Restores into a create-new, workspace-disjoint control-plane directory.
///
/// # Errors
///
/// Rejects stale or tampered plans, destination/staging collisions, snapshot changes, database
/// inconsistency, incomplete session invalidation, permission failures, and receipt mismatch.
pub fn commit_alternate_server_control_plane_restore(
    plan: &AlternateServerControlPlaneRestorePlan,
) -> Result<AlternateServerControlPlaneRestoreReceipt, ServerControlPlaneBackupError> {
    validate_control_plane_restore_plan_shape(plan)?;
    let current = replan_alternate_server_control_plane_restore(
        &plan.control_plane_snapshot_directory,
        &plan.workspace_snapshot_directory,
        &plan.restored_workspace_root,
        &plan.destination_control_plane_root,
        plan.restore_id,
    )?;
    if !same_control_plane_restore_plan(plan, &current) {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    if plan.destination_control_plane_root.exists() || plan.staging_control_plane_root.exists() {
        return Err(ServerControlPlaneBackupError::RestoreTargetExists(
            plan.destination_control_plane_root.clone(),
        ));
    }

    create_private_directory(&plan.staging_control_plane_root)?;
    let source_database_path = plan
        .control_plane_snapshot_directory
        .join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let destination_database_path = plan
        .staging_control_plane_root
        .join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    sqlite_consistent_copy(&source_database_path, &destination_database_path)?;
    set_private_permissions(&destination_database_path, false)?;
    let copied = inspect_database(&destination_database_path)?;
    require_logical_database_match(&plan.source_database, &copied, false)?;
    invalidate_all_sessions(&destination_database_path)?;
    let restored_database = inspect_database(&destination_database_path)?;
    require_logical_database_match(&plan.source_database, &restored_database, true)?;
    if table_evidence(&restored_database, "sessions")?.row_count != 0 {
        return Err(ServerControlPlaneBackupError::Verification(
            "restored control plane retained session or revocation rows".to_owned(),
        ));
    }

    let durable = DurableServerControlPlaneRestoreReceipt {
        schema: SERVER_CONTROL_PLANE_RESTORE_RECEIPT_SCHEMA.to_owned(),
        restore_id: plan.restore_id,
        backup_id: plan.backup_id,
        control_plane_manifest_sha256: plan.control_plane_manifest_sha256.clone(),
        workspace_snapshot: plan.workspace_snapshot.clone(),
        database_sha256: restored_database.sha256.clone(),
        database_length: restored_database.byte_length,
        session_restore_policy: SessionRestorePolicy::InvalidateAll,
        sessions_invalidated: true,
        reverse_proxy_secret_policy:
            ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime,
        reverse_proxy_secret_present: false,
        bootstrap_secret_present: false,
        preserved_tables_verified: preserved_table_names(),
        completed_at_unix_ms: unix_time_ms()?,
    };
    let receipt_bytes = pretty_json_bytes(&durable)?;
    let receipt_path = plan
        .staging_control_plane_root
        .join(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE);
    write_new_file(&receipt_path, &receipt_bytes)?;
    set_private_permissions(&receipt_path, false)?;
    let completion = DurableServerControlPlaneRestoreCompletion {
        schema: SERVER_CONTROL_PLANE_RESTORE_COMPLETION_SCHEMA.to_owned(),
        restore_id: plan.restore_id,
        backup_id: plan.backup_id,
        control_plane_manifest_sha256: plan.control_plane_manifest_sha256.clone(),
        workspace_snapshot_id: plan.workspace_snapshot.snapshot_id,
        workspace_manifest_sha256: plan.workspace_snapshot.manifest_sha256.clone(),
        receipt_sha256: sha256(&receipt_bytes),
        receipt_length: receipt_bytes.len() as u64,
        database_sha256: restored_database.sha256,
        database_length: restored_database.byte_length,
        completed_at_unix_ms: durable.completed_at_unix_ms,
    };
    let completion_bytes = pretty_json_bytes(&completion)?;
    let completion_path = plan
        .staging_control_plane_root
        .join(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE);
    write_new_file(&completion_path, &completion_bytes)?;
    set_private_permissions(&completion_path, false)?;
    verify_private_control_plane_tree(&plan.staging_control_plane_root)?;
    sync_directory(&plan.staging_control_plane_root)?;
    fs::rename(
        &plan.staging_control_plane_root,
        &plan.destination_control_plane_root,
    )?;
    let destination_parent = plan
        .destination_control_plane_root
        .parent()
        .ok_or_else(|| {
            ServerControlPlaneBackupError::InvalidControlPlane(
                "restore destination has no parent".to_owned(),
            )
        })?;
    sync_directory(destination_parent)?;

    verify_alternate_server_control_plane_restore(
        &plan.destination_control_plane_root,
        &plan.control_plane_snapshot_directory,
        &plan.workspace_snapshot_directory,
        &plan.restored_workspace_root,
    )
}

/// Verifies a marker-last alternate restore, session invalidation, permissions, and workspace pair.
///
/// # Errors
///
/// Rejects an incomplete or tampered receipt, mismatched workspace snapshot, retained sessions or
/// secrets, changed database, unsafe permissions, and non-disjoint destinations.
pub fn verify_alternate_server_control_plane_restore(
    destination_control_plane_root: impl AsRef<Path>,
    control_plane_snapshot_directory: impl AsRef<Path>,
    workspace_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
) -> Result<AlternateServerControlPlaneRestoreReceipt, ServerControlPlaneBackupError> {
    let root = canonical_existing_directory(
        destination_control_plane_root.as_ref(),
        "restored control-plane root",
    )?;
    let control_snapshot = verify_server_control_plane_snapshot_internal(
        control_plane_snapshot_directory.as_ref(),
        workspace_snapshot_directory.as_ref(),
    )?;
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory.as_ref())?;
    let restored_workspace_root =
        canonical_existing_directory(restored_workspace_root.as_ref(), "restored workspace root")?;
    require_disjoint(&root, &restored_workspace_root)?;
    require_workspace_matches_snapshot(&restored_workspace_root, &workspace_snapshot)?;
    validate_control_root_inventory(&root, true, false)?;
    verify_private_control_plane_tree(&root)?;
    let receipt_path = root.join(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE);
    let receipt_bytes =
        read_private_bounded_file(&receipt_path, &root, MAX_CONTROL_METADATA_BYTES)?;
    let durable: DurableServerControlPlaneRestoreReceipt = serde_json::from_slice(&receipt_bytes)?;
    let completion_path = root.join(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE);
    let completion_bytes =
        read_private_bounded_file(&completion_path, &root, MAX_CONTROL_METADATA_BYTES)?;
    let completion: DurableServerControlPlaneRestoreCompletion =
        serde_json::from_slice(&completion_bytes)?;
    validate_durable_restore_receipt(&durable, &control_snapshot, &workspace_snapshot)?;
    validate_durable_restore_completion(&completion, &durable, &receipt_bytes)?;
    let database = inspect_database(&root.join(SERVER_CONTROL_PLANE_DATABASE_FILE))?;
    if database.sha256 != durable.database_sha256
        || database.byte_length != durable.database_length
        || table_evidence(&database, "sessions")?.row_count != 0
        || root
            .join(SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE)
            .exists()
        || root
            .join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE)
            .exists()
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "restored database or secret policy differs from its receipt".to_owned(),
        ));
    }
    Ok(AlternateServerControlPlaneRestoreReceipt {
        schema: durable.schema,
        restore_id: durable.restore_id,
        backup_id: durable.backup_id,
        control_plane_manifest_sha256: durable.control_plane_manifest_sha256,
        destination_control_plane_root: root,
        workspace_snapshot: durable.workspace_snapshot,
        database_sha256: durable.database_sha256,
        database_length: durable.database_length,
        receipt_sha256: completion.receipt_sha256,
        receipt_length: completion.receipt_length,
        sessions_invalidated: durable.sessions_invalidated,
        reverse_proxy_secret_present: durable.reverse_proxy_secret_present,
        bootstrap_secret_present: durable.bootstrap_secret_present,
        preserved_tables_verified: durable.preserved_tables_verified,
        permissions_verified: true,
        completed_at_unix_ms: durable.completed_at_unix_ms,
    })
}

/// Applies and re-verifies the supported private permission boundary to an existing control plane.
///
/// This helper is intended for installation/migration tooling. It acquires the same exclusive
/// lease first and never changes database contents or secret bytes.
///
/// # Errors
///
/// Fails if the lease is held, any path is linked or special, an entry is not a supported
/// control-plane file, or the platform cannot apply and re-verify a private permission boundary.
pub fn harden_server_control_plane_permissions(
    control_plane_root: impl AsRef<Path>,
) -> Result<(), ServerControlPlaneBackupError> {
    let lease = acquire_server_control_plane_lease(control_plane_root)?;
    set_private_permissions(lease.root(), true)?;
    for name in control_root_entry_names(lease.root())? {
        let path = lease.root().join(name);
        set_private_permissions(&path, false)?;
    }
    verify_private_control_plane_tree(lease.root())
}

fn build_control_plane_backup_plan(
    lease: &ServerControlPlaneLease,
    workspace_root: &Path,
    workspace_snapshot_directory: &Path,
    backup_parent: &Path,
    backup_id: Uuid,
    allow_live_sqlite_sidecars: bool,
) -> Result<ServerControlPlaneBackupPlan, ServerControlPlaneBackupError> {
    let workspace_root = canonical_existing_directory(workspace_root, "workspace root")?;
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory)?;
    require_workspace_matches_snapshot(&workspace_root, &workspace_snapshot)?;
    require_disjoint(lease.root(), &workspace_root)?;
    require_disjoint(lease.root(), &workspace_snapshot.directory)?;
    let backup_parent = canonical_existing_directory(backup_parent, "backup parent")?;
    reject_workspace_marker_ancestor(&backup_parent, "control-plane backup parent")?;
    let snapshot_directory = backup_parent.join(format!(
        "{CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX}{}",
        backup_id.hyphenated()
    ));
    reject_linked_existing_ancestors(&snapshot_directory)?;
    require_disjoint(&snapshot_directory, lease.root())?;
    require_disjoint(&snapshot_directory, &workspace_root)?;
    require_disjoint(&snapshot_directory, &workspace_snapshot.directory)?;
    if snapshot_directory.exists() {
        return Err(ServerControlPlaneBackupError::SnapshotExists(
            snapshot_directory,
        ));
    }

    let inspected = inspect_control_plane_mode(lease, allow_live_sqlite_sidecars)?;
    let paired = paired_workspace_snapshot(&workspace_snapshot);
    let control_plane_binding = path_binding(lease.root())?;
    let workspace_binding = path_binding(&workspace_root)?;
    let workspace_snapshot_binding = path_binding(&workspace_snapshot.directory)?;
    let destination_binding = new_path_binding(&snapshot_directory)?;
    let plan_digest = backup_plan_digest(
        backup_id,
        &paired,
        &inspected.database,
        &inspected.excluded_operational_files,
        &control_plane_binding,
        &workspace_binding,
        &workspace_snapshot_binding,
        &destination_binding,
    );
    Ok(ServerControlPlaneBackupPlan {
        schema: SERVER_CONTROL_PLANE_BACKUP_PLAN_SCHEMA.to_owned(),
        backup_id,
        plan_digest,
        control_plane_root: lease.root().to_path_buf(),
        workspace_root,
        workspace_snapshot_directory: workspace_snapshot.directory,
        backup_parent,
        snapshot_directory,
        workspace_snapshot: paired,
        source_database: inspected.database,
        bootstrap_secret_policy: BootstrapSecretBackupPolicy::ConsumedRequiredSecretExcluded,
        reverse_proxy_secret_policy:
            ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime,
        session_restore_policy: SessionRestorePolicy::InvalidateAll,
        preserved_on_restore_tables: preserved_table_names(),
        excluded_operational_files: inspected.excluded_operational_files,
        control_plane_binding,
        workspace_binding,
        workspace_snapshot_binding,
        destination_binding,
    })
}

fn paired_workspace_snapshot(snapshot: &VerifiedSnapshot) -> PairedWorkspaceSnapshot {
    PairedWorkspaceSnapshot {
        snapshot_id: snapshot.snapshot_id,
        manifest_sha256: snapshot.manifest_sha256.clone(),
        workspace_root_id: snapshot.workspace_root_id.to_string(),
        workspace_revision: snapshot.workspace_revision.to_string(),
        entry_count: snapshot.manifest.entry_count,
        total_bytes: snapshot.manifest.total_bytes,
    }
}

fn require_workspace_matches_snapshot(
    workspace_root: &Path,
    snapshot: &VerifiedSnapshot,
) -> Result<(), ServerControlPlaneBackupError> {
    require_workspace_matches_snapshot_with_lease_probe(workspace_root, snapshot, || Ok(()))
}

fn require_workspace_matches_snapshot_with_lease_probe(
    workspace_root: &Path,
    snapshot: &VerifiedSnapshot,
    lease_probe: impl FnOnce() -> Result<(), ServerControlPlaneBackupError>,
) -> Result<(), ServerControlPlaneBackupError> {
    // This binding intentionally remains live through the physical copy
    // comparison and every semantic identity/revision check below.
    let workspace_lease = acquire_core_workspace_transaction_lease(workspace_root)
        .map_err(BackupError::CoreTransaction)?;
    let entries = collect_workspace_physical_entries(workspace_root, &workspace_lease)?;
    lease_probe()?;
    if entries != snapshot.manifest.entries {
        return Err(ServerControlPlaneBackupError::WorkspaceSnapshotMismatch);
    }
    verify_bytewise_trees(
        &snapshot.workspace_content_root,
        workspace_root,
        &snapshot.manifest.entries,
    )?;
    let reopened = super::scan_workspace(workspace_root);
    let root_id = reopened
        .nodes
        .iter()
        .find(|node| node.path == workspace_root)
        .and_then(|node| node.id)
        .ok_or(ServerControlPlaneBackupError::WorkspaceSnapshotMismatch)?;
    let revision = super::read_workspace_revision(workspace_root).map_err(BackupError::Revision)?;
    if root_id != snapshot.workspace_root_id || revision != snapshot.workspace_revision {
        return Err(ServerControlPlaneBackupError::WorkspaceSnapshotMismatch);
    }
    workspace_lease
        .validate_anchor_identity()
        .map_err(BackupError::CoreTransaction)?;
    drop(workspace_lease);
    Ok(())
}

fn inspect_control_plane(
    lease: &ServerControlPlaneLease,
) -> Result<InspectedControlPlane, ServerControlPlaneBackupError> {
    inspect_control_plane_mode(lease, false)
}

fn inspect_control_plane_mode(
    lease: &ServerControlPlaneLease,
    allow_live_sqlite_sidecars: bool,
) -> Result<InspectedControlPlane, ServerControlPlaneBackupError> {
    validate_control_root_inventory(lease.root(), false, allow_live_sqlite_sidecars)?;
    verify_private_control_plane_tree(lease.root())?;
    let database_path = lease.root().join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let database = inspect_database(&database_path)?;
    let mut excluded_operational_files = vec![SERVER_CONTROL_PLANE_LEASE_FILE.to_owned()];
    if lease
        .root()
        .join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE)
        .exists()
    {
        excluded_operational_files.push(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE.to_owned());
    }
    if lease
        .root()
        .join(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE)
        .exists()
    {
        excluded_operational_files.push(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE.to_owned());
    }
    if lease
        .root()
        .join(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE)
        .exists()
    {
        excluded_operational_files.push(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE.to_owned());
    }
    if allow_live_sqlite_sidecars {
        for name in [
            SERVER_CONTROL_PLANE_DATABASE_WAL_FILE,
            SERVER_CONTROL_PLANE_DATABASE_SHM_FILE,
            SERVER_CONTROL_PLANE_DATABASE_JOURNAL_FILE,
        ] {
            if lease.root().join(name).exists() {
                excluded_operational_files.push(name.to_owned());
            }
        }
    }
    excluded_operational_files.sort();
    Ok(InspectedControlPlane {
        database,
        excluded_operational_files,
    })
}

fn validate_control_root_inventory(
    root: &Path,
    require_restore_receipt: bool,
    allow_live_sqlite_sidecars: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    let bootstrap = root.join(SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE);
    if bootstrap.try_exists()? {
        reject_link_or_hardlink(&bootstrap)?;
        return Err(ServerControlPlaneBackupError::UninitializedControlPlane);
    }
    for suffix in ["-wal", "-shm", "-journal"] {
        let transient = root.join(format!("{SERVER_CONTROL_PLANE_DATABASE_FILE}{suffix}"));
        if !allow_live_sqlite_sidecars && transient.try_exists()? {
            return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
                "live or uncheckpointed SQLite artifact is present: {}",
                transient.display()
            )));
        }
    }

    let mut actual = BTreeSet::new();
    let mut folded = BTreeSet::new();
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry
            .file_name()
            .to_str()
            .ok_or_else(|| {
                ServerControlPlaneBackupError::InvalidControlPlane(
                    "control-plane entry name is not UTF-8".to_owned(),
                )
            })?
            .to_owned();
        let metadata = fs::symlink_metadata(&path)?;
        if linked_or_reparse(&metadata) || !metadata.is_file() {
            return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
                "control-plane entry is linked, reparsed, or not a regular file: {}",
                path.display()
            )));
        }
        reject_hardlink(&path)?;
        if !folded.insert(name.to_lowercase()) {
            return Err(ServerControlPlaneBackupError::InvalidControlPlane(
                "case-colliding control-plane entries are not supported".to_owned(),
            ));
        }
        actual.insert(name);
    }

    let database = SERVER_CONTROL_PLANE_DATABASE_FILE.to_owned();
    if !actual.contains(&database) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "control-plane database is missing".to_owned(),
        ));
    }
    let allowed = BTreeSet::from([
        database,
        SERVER_CONTROL_PLANE_LEASE_FILE.to_owned(),
        SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE.to_owned(),
        SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE.to_owned(),
        SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE.to_owned(),
        SERVER_CONTROL_PLANE_DATABASE_WAL_FILE.to_owned(),
        SERVER_CONTROL_PLANE_DATABASE_SHM_FILE.to_owned(),
        SERVER_CONTROL_PLANE_DATABASE_JOURNAL_FILE.to_owned(),
    ]);
    if !actual.is_subset(&allowed) {
        let unknown = actual
            .difference(&allowed)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "unknown control-plane entries: {unknown}"
        )));
    }
    if require_restore_receipt
        && (!actual.contains(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE)
            || !actual.contains(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE))
    {
        return Err(ServerControlPlaneBackupError::IncompleteSnapshot(
            root.to_path_buf(),
        ));
    }
    if let Some(path) = [
        SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE,
        SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE,
        SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE,
    ]
    .into_iter()
    .map(|name| root.join(name))
    .find(|path| {
        path.exists()
            && fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_CONTROL_METADATA_BYTES)
    }) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "operational control-plane file is unbounded: {}",
            path.display()
        )));
    }
    Ok(())
}

fn control_root_entry_names(root: &Path) -> Result<Vec<String>, ServerControlPlaneBackupError> {
    fs::read_dir(root)?
        .map(|entry| {
            let entry = entry?;
            entry.file_name().into_string().map_err(|_| {
                ServerControlPlaneBackupError::InvalidControlPlane(
                    "control-plane entry name is not UTF-8".to_owned(),
                )
            })
        })
        .collect()
}

fn verify_server_control_plane_snapshot_internal(
    control_plane_snapshot_directory: &Path,
    workspace_snapshot_directory: &Path,
) -> Result<VerifiedControlPlaneSnapshot, ServerControlPlaneBackupError> {
    let directory = canonical_existing_directory(
        control_plane_snapshot_directory,
        "control-plane snapshot directory",
    )?;
    let completion_path = directory.join(SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE);
    if !completion_path.try_exists()? {
        return Err(ServerControlPlaneBackupError::IncompleteSnapshot(directory));
    }
    validate_control_snapshot_container(&directory)?;
    verify_private_snapshot_tree(&directory)?;
    let manifest_path = directory.join(SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE);
    let database_path = directory.join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let manifest_bytes =
        read_private_bounded_file(&manifest_path, &directory, MAX_CONTROL_METADATA_BYTES)?;
    let completion_bytes =
        read_private_bounded_file(&completion_path, &directory, MAX_CONTROL_METADATA_BYTES)?;
    let manifest: ServerControlPlaneBackupManifest = serde_json::from_slice(&manifest_bytes)?;
    let completion: ServerControlPlaneBackupCompletion = serde_json::from_slice(&completion_bytes)?;
    validate_control_plane_manifest(&manifest)?;
    validate_control_plane_completion(&completion, &manifest, &manifest_bytes)?;
    let expected_name = format!(
        "{CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX}{}",
        manifest.backup_id.hyphenated()
    );
    if directory.file_name().and_then(|name| name.to_str()) != Some(expected_name.as_str()) {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane snapshot directory name does not bind its backup ID".to_owned(),
        ));
    }
    let database = inspect_database(&database_path)?;
    if database != manifest.database {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane database differs from the manifest".to_owned(),
        ));
    }
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory)?;
    if paired_workspace_snapshot(&workspace_snapshot) != manifest.workspace_snapshot {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane snapshot is paired to a different full-workspace snapshot".to_owned(),
        ));
    }
    let manifest_after =
        read_private_bounded_file(&manifest_path, &directory, MAX_CONTROL_METADATA_BYTES)?;
    let completion_after =
        read_private_bounded_file(&completion_path, &directory, MAX_CONTROL_METADATA_BYTES)?;
    let database_after = inspect_database(&database_path)?;
    if manifest_after != manifest_bytes
        || completion_after != completion_bytes
        || database_after != database
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane snapshot changed during verification".to_owned(),
        ));
    }
    Ok(VerifiedControlPlaneSnapshot {
        directory,
        manifest,
        manifest_sha256: sha256(&manifest_bytes),
        manifest_length: manifest_bytes.len() as u64,
        created_at_unix_ms: completion.created_at_unix_ms,
    })
}

fn validate_control_snapshot_container(
    directory: &Path,
) -> Result<(), ServerControlPlaneBackupError> {
    let expected = BTreeSet::from([
        SERVER_CONTROL_PLANE_DATABASE_FILE.to_owned(),
        SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE.to_owned(),
        SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE.to_owned(),
    ]);
    let mut actual = BTreeSet::new();
    let mut folded = BTreeSet::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().into_string().map_err(|_| {
            ServerControlPlaneBackupError::Verification(
                "control-plane snapshot entry is not UTF-8".to_owned(),
            )
        })?;
        let metadata = fs::symlink_metadata(&path)?;
        if linked_or_reparse(&metadata) || !metadata.is_file() {
            return Err(ServerControlPlaneBackupError::Verification(format!(
                "control-plane snapshot entry is linked or not a regular file: {}",
                path.display()
            )));
        }
        reject_hardlink(&path)?;
        if !folded.insert(name.to_lowercase()) {
            return Err(ServerControlPlaneBackupError::Verification(
                "case-colliding control-plane snapshot entries".to_owned(),
            ));
        }
        actual.insert(name);
    }
    if actual != expected {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane snapshot has missing or unknown entries".to_owned(),
        ));
    }
    Ok(())
}

fn validate_control_plane_manifest(
    manifest: &ServerControlPlaneBackupManifest,
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(manifest.backup_id, "control-plane backup")?;
    validate_paired_workspace_snapshot(&manifest.workspace_snapshot)?;
    validate_database_evidence(&manifest.database)?;
    if manifest.schema != SERVER_CONTROL_PLANE_BACKUP_MANIFEST_SCHEMA
        || manifest.bootstrap_secret_policy
            != BootstrapSecretBackupPolicy::ConsumedRequiredSecretExcluded
        || manifest.reverse_proxy_secret_policy
            != ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime
        || manifest.session_restore_policy != SessionRestorePolicy::InvalidateAll
        || manifest.preserved_on_restore_tables != preserved_table_names()
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane manifest has an unsupported policy or schema".to_owned(),
        ));
    }
    validate_operational_exclusions(&manifest.excluded_operational_files)
}

fn validate_control_plane_completion(
    completion: &ServerControlPlaneBackupCompletion,
    manifest: &ServerControlPlaneBackupManifest,
    manifest_bytes: &[u8],
) -> Result<(), ServerControlPlaneBackupError> {
    if completion.schema != SERVER_CONTROL_PLANE_BACKUP_COMPLETION_SCHEMA
        || completion.backup_id != manifest.backup_id
        || completion.workspace_snapshot_id != manifest.workspace_snapshot.snapshot_id
        || completion.workspace_manifest_sha256 != manifest.workspace_snapshot.manifest_sha256
        || completion.manifest_sha256 != sha256(manifest_bytes)
        || completion.manifest_length != manifest_bytes.len() as u64
        || completion.database_sha256 != manifest.database.sha256
        || completion.database_length != manifest.database.byte_length
        || completion.created_at_unix_ms == 0
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane completion marker does not exactly bind manifest and database"
                .to_owned(),
        ));
    }
    require_sha256(&completion.manifest_sha256, "manifest")?;
    require_sha256(&completion.database_sha256, "database")
}

fn validate_paired_workspace_snapshot(
    snapshot: &PairedWorkspaceSnapshot,
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(snapshot.snapshot_id, "workspace snapshot")?;
    require_sha256(&snapshot.manifest_sha256, "workspace manifest")?;
    if snapshot.workspace_root_id.len() != 36
        || snapshot.workspace_revision.len() != 64
        || snapshot.entry_count == 0
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "paired workspace snapshot identity is invalid".to_owned(),
        ));
    }
    require_sha256(&snapshot.workspace_revision, "workspace revision")
}

fn validate_operational_exclusions(
    exclusions: &[String],
) -> Result<(), ServerControlPlaneBackupError> {
    let mut sorted = exclusions.to_vec();
    sorted.sort();
    sorted.dedup();
    let allowed = BTreeSet::from([
        SERVER_CONTROL_PLANE_LEASE_FILE,
        SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE,
        SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE,
        SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE,
        SERVER_CONTROL_PLANE_DATABASE_WAL_FILE,
        SERVER_CONTROL_PLANE_DATABASE_SHM_FILE,
        SERVER_CONTROL_PLANE_DATABASE_JOURNAL_FILE,
    ]);
    if sorted != exclusions
        || !exclusions
            .iter()
            .all(|entry| allowed.contains(entry.as_str()))
        || !exclusions
            .iter()
            .any(|entry| entry == SERVER_CONTROL_PLANE_LEASE_FILE)
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane operational exclusion set is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_control_plane_backup_plan_shape(
    plan: &ServerControlPlaneBackupPlan,
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(plan.backup_id, "control-plane backup")?;
    validate_paired_workspace_snapshot(&plan.workspace_snapshot)?;
    validate_database_evidence(&plan.source_database)?;
    validate_operational_exclusions(&plan.excluded_operational_files)?;
    let control_plane_root =
        canonical_existing_directory(&plan.control_plane_root, "control-plane root")?;
    let workspace_root = canonical_existing_directory(&plan.workspace_root, "workspace root")?;
    let workspace_snapshot = canonical_existing_directory(
        &plan.workspace_snapshot_directory,
        "workspace snapshot directory",
    )?;
    let backup_parent = canonical_existing_directory(&plan.backup_parent, "backup parent")?;
    let expected_snapshot = backup_parent.join(format!(
        "{CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX}{}",
        plan.backup_id.hyphenated()
    ));
    if plan.schema != SERVER_CONTROL_PLANE_BACKUP_PLAN_SCHEMA
        || control_plane_root != plan.control_plane_root
        || workspace_root != plan.workspace_root
        || workspace_snapshot != plan.workspace_snapshot_directory
        || backup_parent != plan.backup_parent
        || expected_snapshot != plan.snapshot_directory
        || path_binding(&control_plane_root)? != plan.control_plane_binding
        || path_binding(&workspace_root)? != plan.workspace_binding
        || path_binding(&workspace_snapshot)? != plan.workspace_snapshot_binding
        || new_path_binding(&expected_snapshot)? != plan.destination_binding
        || plan.bootstrap_secret_policy
            != BootstrapSecretBackupPolicy::ConsumedRequiredSecretExcluded
        || plan.reverse_proxy_secret_policy
            != ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime
        || plan.session_restore_policy != SessionRestorePolicy::InvalidateAll
        || plan.preserved_on_restore_tables != preserved_table_names()
        || backup_plan_digest(
            plan.backup_id,
            &plan.workspace_snapshot,
            &plan.source_database,
            &plan.excluded_operational_files,
            &plan.control_plane_binding,
            &plan.workspace_binding,
            &plan.workspace_snapshot_binding,
            &plan.destination_binding,
        ) != plan.plan_digest
    {
        return Err(ServerControlPlaneBackupError::InvalidPlan);
    }
    Ok(())
}

fn same_control_plane_backup_plan(
    left: &ServerControlPlaneBackupPlan,
    right: &ServerControlPlaneBackupPlan,
) -> bool {
    left.schema == right.schema
        && left.backup_id == right.backup_id
        && left.plan_digest == right.plan_digest
        && left.control_plane_root == right.control_plane_root
        && left.workspace_root == right.workspace_root
        && left.workspace_snapshot_directory == right.workspace_snapshot_directory
        && left.backup_parent == right.backup_parent
        && left.snapshot_directory == right.snapshot_directory
        && left.workspace_snapshot == right.workspace_snapshot
        && left.source_database == right.source_database
        && left.bootstrap_secret_policy == right.bootstrap_secret_policy
        && left.reverse_proxy_secret_policy == right.reverse_proxy_secret_policy
        && left.session_restore_policy == right.session_restore_policy
        && left.preserved_on_restore_tables == right.preserved_on_restore_tables
        && left.excluded_operational_files == right.excluded_operational_files
        && left.control_plane_binding == right.control_plane_binding
        && left.workspace_binding == right.workspace_binding
        && left.workspace_snapshot_binding == right.workspace_snapshot_binding
        && left.destination_binding == right.destination_binding
}

fn validate_control_plane_restore_plan_shape(
    plan: &AlternateServerControlPlaneRestorePlan,
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(plan.restore_id, "control-plane restore")?;
    require_v4(plan.backup_id, "control-plane backup")?;
    validate_paired_workspace_snapshot(&plan.workspace_snapshot)?;
    validate_database_evidence(&plan.source_database)?;
    require_sha256(
        &plan.control_plane_manifest_sha256,
        "control-plane manifest",
    )?;
    let control_snapshot = canonical_existing_directory(
        &plan.control_plane_snapshot_directory,
        "control-plane snapshot directory",
    )?;
    let workspace_snapshot = canonical_existing_directory(
        &plan.workspace_snapshot_directory,
        "workspace snapshot directory",
    )?;
    let restored_workspace =
        canonical_existing_directory(&plan.restored_workspace_root, "restored workspace root")?;
    let destination = normalize_new_destination(&plan.destination_control_plane_root)?;
    let parent = destination.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "restore destination has no parent".to_owned(),
        )
    })?;
    let expected_staging = parent.join(format!(
        "{CONTROL_PLANE_RESTORE_STAGING_PREFIX}{}.staging",
        plan.restore_id.hyphenated()
    ));
    let actual_control_manifest_sha256 = control_snapshot_manifest_digest(&control_snapshot)?;
    if plan.schema != SERVER_CONTROL_PLANE_RESTORE_PLAN_SCHEMA
        || control_snapshot != plan.control_plane_snapshot_directory
        || workspace_snapshot != plan.workspace_snapshot_directory
        || restored_workspace != plan.restored_workspace_root
        || destination != plan.destination_control_plane_root
        || expected_staging != plan.staging_control_plane_root
        || actual_control_manifest_sha256 != plan.control_plane_manifest_sha256
        || path_binding(&control_snapshot)? != plan.control_snapshot_binding
        || path_binding(&workspace_snapshot)? != plan.workspace_snapshot_binding
        || path_binding(&restored_workspace)? != plan.restored_workspace_binding
        || new_path_binding(&destination)? != plan.destination_binding
        || plan.session_restore_policy != SessionRestorePolicy::InvalidateAll
        || plan.reverse_proxy_secret_policy
            != ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime
        || restore_plan_digest(
            plan.restore_id,
            plan.backup_id,
            &plan.control_plane_manifest_sha256,
            &plan.workspace_snapshot,
            &plan.control_snapshot_binding,
            &plan.workspace_snapshot_binding,
            &plan.restored_workspace_binding,
            &plan.destination_binding,
        ) != plan.plan_digest
    {
        return Err(ServerControlPlaneBackupError::InvalidPlan);
    }
    Ok(())
}

fn same_control_plane_restore_plan(
    left: &AlternateServerControlPlaneRestorePlan,
    right: &AlternateServerControlPlaneRestorePlan,
) -> bool {
    left.schema == right.schema
        && left.restore_id == right.restore_id
        && left.plan_digest == right.plan_digest
        && left.control_plane_snapshot_directory == right.control_plane_snapshot_directory
        && left.workspace_snapshot_directory == right.workspace_snapshot_directory
        && left.restored_workspace_root == right.restored_workspace_root
        && left.destination_control_plane_root == right.destination_control_plane_root
        && left.staging_control_plane_root == right.staging_control_plane_root
        && left.backup_id == right.backup_id
        && left.control_plane_manifest_sha256 == right.control_plane_manifest_sha256
        && left.workspace_snapshot == right.workspace_snapshot
        && left.source_database == right.source_database
        && left.session_restore_policy == right.session_restore_policy
        && left.reverse_proxy_secret_policy == right.reverse_proxy_secret_policy
        && left.control_snapshot_binding == right.control_snapshot_binding
        && left.workspace_snapshot_binding == right.workspace_snapshot_binding
        && left.restored_workspace_binding == right.restored_workspace_binding
        && left.destination_binding == right.destination_binding
}

fn control_snapshot_manifest_digest(
    snapshot_directory: &Path,
) -> Result<String, ServerControlPlaneBackupError> {
    let bytes = read_private_bounded_file(
        &snapshot_directory.join(SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE),
        snapshot_directory,
        MAX_CONTROL_METADATA_BYTES,
    )?;
    Ok(sha256(&bytes))
}

#[allow(clippy::too_many_arguments)]
fn backup_plan_digest(
    backup_id: Uuid,
    workspace: &PairedWorkspaceSnapshot,
    database: &ControlPlaneDatabaseEvidence,
    exclusions: &[String],
    control_plane_binding: &str,
    workspace_binding: &str,
    workspace_snapshot_binding: &str,
    destination_binding: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-control-plane-backup-plan.v1\0");
    digest_text(&mut hasher, &backup_id.hyphenated().to_string());
    digest_workspace_snapshot(&mut hasher, workspace);
    digest_database_evidence(&mut hasher, database);
    for exclusion in exclusions {
        digest_text(&mut hasher, exclusion);
    }
    digest_text(&mut hasher, control_plane_binding);
    digest_text(&mut hasher, workspace_binding);
    digest_text(&mut hasher, workspace_snapshot_binding);
    digest_text(&mut hasher, destination_binding);
    format!("{:x}", hasher.finalize())
}

#[allow(clippy::too_many_arguments)]
fn restore_plan_digest(
    restore_id: Uuid,
    backup_id: Uuid,
    control_manifest_sha256: &str,
    workspace: &PairedWorkspaceSnapshot,
    control_snapshot_binding: &str,
    workspace_snapshot_binding: &str,
    restored_workspace_binding: &str,
    destination_binding: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-control-plane-restore-plan.v1\0");
    digest_text(&mut hasher, &restore_id.hyphenated().to_string());
    digest_text(&mut hasher, &backup_id.hyphenated().to_string());
    digest_text(&mut hasher, control_manifest_sha256);
    digest_workspace_snapshot(&mut hasher, workspace);
    digest_text(&mut hasher, control_snapshot_binding);
    digest_text(&mut hasher, workspace_snapshot_binding);
    digest_text(&mut hasher, restored_workspace_binding);
    digest_text(&mut hasher, destination_binding);
    format!("{:x}", hasher.finalize())
}

fn digest_workspace_snapshot(hasher: &mut Sha256, workspace: &PairedWorkspaceSnapshot) {
    digest_text(hasher, &workspace.snapshot_id.hyphenated().to_string());
    digest_text(hasher, &workspace.manifest_sha256);
    digest_text(hasher, &workspace.workspace_root_id);
    digest_text(hasher, &workspace.workspace_revision);
    hasher.update(workspace.entry_count.to_le_bytes());
    hasher.update(workspace.total_bytes.to_le_bytes());
}

fn digest_database_evidence(hasher: &mut Sha256, database: &ControlPlaneDatabaseEvidence) {
    hasher.update(database.byte_length.to_le_bytes());
    digest_text(hasher, &database.sha256);
    digest_text(hasher, &database.schema_sha256);
    digest_text(hasher, &database.workspace_scope_sha256);
    hasher.update(database.application_id.to_le_bytes());
    hasher.update(database.user_version.to_le_bytes());
    for table in &database.tables {
        digest_text(hasher, &table.name);
        hasher.update(table.row_count.to_le_bytes());
        digest_text(hasher, &table.sha256);
    }
}

fn digest_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn validate_database_evidence(
    database: &ControlPlaneDatabaseEvidence,
) -> Result<(), ServerControlPlaneBackupError> {
    require_sha256(&database.sha256, "control-plane database")?;
    require_sha256(&database.schema_sha256, "control-plane schema")?;
    require_sha256(&database.workspace_scope_sha256, "workspace scope")?;
    if database.byte_length == 0 || database.byte_length > MAX_CONTROL_DATABASE_BYTES {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane database length is invalid".to_owned(),
        ));
    }
    let names = database
        .tables
        .iter()
        .map(|table| table.name.as_str())
        .collect::<Vec<_>>();
    if names != CONTROL_TABLES {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane table evidence set is invalid".to_owned(),
        ));
    }
    for table in &database.tables {
        require_sha256(&table.sha256, "control-plane table")?;
    }
    Ok(())
}

fn preserved_table_names() -> Vec<String> {
    PRESERVED_ON_RESTORE_TABLES
        .iter()
        .map(|name| (*name).to_owned())
        .collect()
}

fn table_evidence<'a>(
    database: &'a ControlPlaneDatabaseEvidence,
    table: &str,
) -> Result<&'a ControlPlaneTableEvidence, ServerControlPlaneBackupError> {
    database
        .tables
        .iter()
        .find(|evidence| evidence.name == table)
        .ok_or_else(|| {
            ServerControlPlaneBackupError::Verification(format!(
                "missing evidence for control-plane table {table}"
            ))
        })
}

fn require_logical_database_match(
    source: &ControlPlaneDatabaseEvidence,
    destination: &ControlPlaneDatabaseEvidence,
    sessions_invalidated: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    if source.schema_sha256 != destination.schema_sha256
        || source.workspace_scope_sha256 != destination.workspace_scope_sha256
        || source.application_id != destination.application_id
        || source.user_version != destination.user_version
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "SQLite backup changed schema or required metadata".to_owned(),
        ));
    }
    for table in CONTROL_TABLES {
        let source_table = table_evidence(source, table)?;
        let destination_table = table_evidence(destination, table)?;
        if *table == "sessions" && sessions_invalidated {
            if destination_table.row_count != 0 {
                return Err(ServerControlPlaneBackupError::Verification(
                    "session invalidation did not clear every session row".to_owned(),
                ));
            }
        } else if source_table != destination_table {
            return Err(ServerControlPlaneBackupError::Verification(format!(
                "SQLite backup changed preserved table {table}"
            )));
        }
    }
    Ok(())
}

fn validate_durable_restore_receipt(
    receipt: &DurableServerControlPlaneRestoreReceipt,
    control_snapshot: &VerifiedControlPlaneSnapshot,
    workspace_snapshot: &VerifiedSnapshot,
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(receipt.restore_id, "control-plane restore")?;
    require_v4(receipt.backup_id, "control-plane backup")?;
    require_sha256(
        &receipt.control_plane_manifest_sha256,
        "control-plane manifest",
    )?;
    require_sha256(&receipt.database_sha256, "restored database")?;
    if receipt.schema != SERVER_CONTROL_PLANE_RESTORE_RECEIPT_SCHEMA
        || receipt.backup_id != control_snapshot.manifest.backup_id
        || receipt.control_plane_manifest_sha256 != control_snapshot.manifest_sha256
        || receipt.workspace_snapshot != control_snapshot.manifest.workspace_snapshot
        || receipt.workspace_snapshot != paired_workspace_snapshot(workspace_snapshot)
        || receipt.database_length == 0
        || receipt.database_length > MAX_CONTROL_DATABASE_BYTES
        || receipt.session_restore_policy != SessionRestorePolicy::InvalidateAll
        || !receipt.sessions_invalidated
        || receipt.reverse_proxy_secret_policy
            != ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime
        || receipt.reverse_proxy_secret_present
        || receipt.bootstrap_secret_present
        || receipt.preserved_tables_verified != preserved_table_names()
        || receipt.completed_at_unix_ms == 0
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "alternate control-plane restore receipt is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_durable_restore_completion(
    completion: &DurableServerControlPlaneRestoreCompletion,
    receipt: &DurableServerControlPlaneRestoreReceipt,
    receipt_bytes: &[u8],
) -> Result<(), ServerControlPlaneBackupError> {
    require_v4(completion.restore_id, "control-plane restore")?;
    require_v4(completion.backup_id, "control-plane backup")?;
    require_sha256(
        &completion.control_plane_manifest_sha256,
        "control-plane manifest",
    )?;
    require_sha256(&completion.workspace_manifest_sha256, "workspace manifest")?;
    require_sha256(&completion.receipt_sha256, "restore receipt")?;
    require_sha256(&completion.database_sha256, "restored database")?;
    if completion.schema != SERVER_CONTROL_PLANE_RESTORE_COMPLETION_SCHEMA
        || completion.restore_id != receipt.restore_id
        || completion.backup_id != receipt.backup_id
        || completion.control_plane_manifest_sha256 != receipt.control_plane_manifest_sha256
        || completion.workspace_snapshot_id != receipt.workspace_snapshot.snapshot_id
        || completion.workspace_manifest_sha256 != receipt.workspace_snapshot.manifest_sha256
        || completion.receipt_sha256 != sha256(receipt_bytes)
        || completion.receipt_length != receipt_bytes.len() as u64
        || completion.database_sha256 != receipt.database_sha256
        || completion.database_length != receipt.database_length
        || completion.completed_at_unix_ms != receipt.completed_at_unix_ms
        || completion.completed_at_unix_ms == 0
    {
        return Err(ServerControlPlaneBackupError::Verification(
            "alternate control-plane restore completion does not bind its receipt".to_owned(),
        ));
    }
    Ok(())
}

fn inspect_database(
    database_path: &Path,
) -> Result<ControlPlaneDatabaseEvidence, ServerControlPlaneBackupError> {
    reject_link_or_hardlink(database_path)?;
    verify_private_permissions(database_path, false)?;
    let (byte_length, digest_before, modified_before) = digest_regular_file(database_path)?;
    if byte_length == 0 || byte_length > MAX_CONTROL_DATABASE_BYTES {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "control-plane database size is outside the supported boundary".to_owned(),
        ));
    }
    let connection = open_database_read_only(database_path)?;
    validate_database_schema_and_state(&connection)?;
    let schema_sha256 = database_schema_digest(&connection)?;
    let workspace_scope: String = connection.query_row(
        "SELECT value FROM metadata WHERE key = ?1",
        [WORKSPACE_SCOPE_KEY],
        |row| row.get(0),
    )?;
    if !is_lower_hex(&workspace_scope, 64) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "workspace_scope_v1 metadata is not a 32-byte lowercase hex value".to_owned(),
        ));
    }
    let application_id = connection.query_row("PRAGMA application_id", [], |row| row.get(0))?;
    let user_version = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    let mut tables = Vec::with_capacity(CONTROL_TABLES.len());
    for table in CONTROL_TABLES {
        tables.push(database_table_evidence(&connection, table)?);
    }
    drop(connection);
    let (length_after, digest_after, modified_after) = digest_regular_file(database_path)?;
    if byte_length != length_after
        || digest_before != digest_after
        || modified_before != modified_after
    {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    Ok(ControlPlaneDatabaseEvidence {
        byte_length,
        sha256: digest_before,
        schema_sha256,
        workspace_scope_sha256: sha256(workspace_scope.as_bytes()),
        application_id,
        user_version,
        tables,
    })
}

fn open_database_read_only(
    database_path: &Path,
) -> Result<Connection, ServerControlPlaneBackupError> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch("PRAGMA query_only=ON; PRAGMA foreign_keys=ON;")?;
    Ok(connection)
}

fn validate_database_schema_and_state(
    connection: &Connection,
) -> Result<(), ServerControlPlaneBackupError> {
    let quick_check: String =
        connection.query_row("PRAGMA quick_check(1)", [], |row| row.get(0))?;
    if quick_check != "ok" {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "SQLite quick_check failed: {quick_check}"
        )));
    }
    let foreign_key_violation = {
        let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        rows.next()?.is_some()
    };
    if foreign_key_violation {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "SQLite foreign_key_check found a violation".to_owned(),
        ));
    }

    let mut table_names = Vec::new();
    let mut explicit_indexes = Vec::new();
    let mut unsupported_objects = Vec::new();
    {
        let mut statement = connection.prepare(
            "SELECT type, name FROM sqlite_schema
             WHERE name NOT LIKE 'sqlite_%' ORDER BY type, name",
        )?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let object_type: String = row.get(0)?;
            let name: String = row.get(1)?;
            match object_type.as_str() {
                "table" => table_names.push(name),
                "index" => explicit_indexes.push(name),
                _ => unsupported_objects.push(format!("{object_type}:{name}")),
            }
        }
    }
    table_names.sort();
    if table_names != CONTROL_TABLES || explicit_indexes != ["sessions_expiry"] {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "unsupported control-plane schema objects (tables={table_names:?}, indexes={explicit_indexes:?})"
        )));
    }
    if !unsupported_objects.is_empty() {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "unsupported control-plane schema objects: {}",
            unsupported_objects.join(", ")
        )));
    }
    for table in CONTROL_TABLES {
        validate_table_columns(connection, table)?;
    }

    let workspace_scope_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM metadata WHERE key = ?1",
        [WORKSPACE_SCOPE_KEY],
        |row| row.get(0),
    )?;
    let bootstrap_digest_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM metadata WHERE key = ?1",
        [BOOTSTRAP_DIGEST_KEY],
        |row| row.get(0),
    )?;
    let owner_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM owner", [], |row| row.get(0))?;
    let matching_owner_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM owner
         JOIN accounts USING(actor_scope)
         WHERE accounts.role = 'owner' AND accounts.disabled_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    let enabled_owner_count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM accounts
         WHERE role = 'owner' AND disabled_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    if workspace_scope_count != 1
        || bootstrap_digest_count != 0
        || owner_count != 1
        || matching_owner_count != 1
        || enabled_owner_count < 1
    {
        return Err(ServerControlPlaneBackupError::UninitializedControlPlane);
    }
    Ok(())
}

fn validate_table_columns(
    connection: &Connection,
    table: &str,
) -> Result<(), ServerControlPlaneBackupError> {
    let sql = format!("PRAGMA table_info(\"{table}\")");
    let mut statement = connection.prepare(&sql)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if columns != expected_table_columns(table) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "unsupported column contract for table {table}"
        )));
    }
    Ok(())
}

fn expected_table_columns(table: &str) -> &'static [&'static str] {
    match table {
        "accounts" => &[
            "actor_scope",
            "login",
            "password_hash",
            "role",
            "created_at",
            "disabled_at",
        ],
        "audit_outbox" => &[
            "intent_id",
            "created_at",
            "event_type",
            "actor_scope",
            "detail",
            "authority_kind",
            "target",
            "expected_revision",
        ],
        "audit_receipts" => &[
            "receipt_id",
            "occurred_at",
            "event_type",
            "actor_scope",
            "detail",
        ],
        "authorization_epochs" => &["actor_scope", "epoch"],
        "collaboration_documents" => &[
            "node_id",
            "epoch",
            "version",
            "checkpoint_revision",
            "frozen_reason",
            "expected_revision",
            "updated_at",
        ],
        "collaboration_pending" => &[
            "intent_id",
            "operation_id",
            "actor_scope",
            "actor_id",
            "client_id",
            "node_id",
            "epoch",
            "base_version",
            "base_revision",
            "applied_base_version",
            "applied_base_revision",
            "result_version",
            "result_revision",
            "request_digest",
            "transaction_id",
        ],
        "collaboration_receipts" => &[
            "operation_id",
            "actor_scope",
            "actor_id",
            "client_id",
            "node_id",
            "epoch",
            "base_version",
            "base_revision",
            "applied_base_version",
            "applied_base_revision",
            "result_version",
            "result_revision",
            "request_digest",
            "transaction_id",
            "occurred_at",
        ],
        "metadata" => &["key", "value"],
        "node_acl" => &[
            "actor_scope",
            "node_id",
            "access",
            "updated_at",
            "updated_by",
        ],
        "owner" => &["singleton", "actor_scope", "password_hash", "created_at"],
        "security_events" => &[
            "event_id",
            "occurred_at",
            "event_type",
            "actor_scope",
            "detail",
        ],
        "sessions" => &[
            "session_id",
            "actor_scope",
            "token_digest",
            "created_at",
            "last_seen_at",
            "absolute_expires_at",
            "idle_expires_at",
            "revoked_at",
            "end_reason",
        ],
        _ => &[],
    }
}

fn database_schema_digest(
    connection: &Connection,
) -> Result<String, ServerControlPlaneBackupError> {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-control-plane-schema.v1\0");
    let mut statement = connection.prepare(
        "SELECT type, name, tbl_name, COALESCE(sql, '') FROM sqlite_schema
         ORDER BY type, name, tbl_name, COALESCE(sql, '')",
    )?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        for column in 0..4 {
            let value: String = row.get(column)?;
            digest_text(&mut hasher, &value);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn database_table_evidence(
    connection: &Connection,
    table: &str,
) -> Result<ControlPlaneTableEvidence, ServerControlPlaneBackupError> {
    let mut statement = connection.prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))?;
    let column_count = statement.column_count();
    let column_names = statement
        .column_names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-control-plane-table.v1\0");
    digest_text(&mut hasher, table);
    for name in &column_names {
        digest_text(&mut hasher, name);
    }
    let mut row_count = 0_u64;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        hasher.update(b"row\0");
        for column in 0..column_count {
            digest_sql_value(&mut hasher, row.get_ref(column)?);
        }
        row_count = row_count.checked_add(1).ok_or_else(|| {
            ServerControlPlaneBackupError::Verification(
                "control-plane table row count overflow".to_owned(),
            )
        })?;
    }
    Ok(ControlPlaneTableEvidence {
        name: table.to_owned(),
        row_count,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

fn digest_sql_value(hasher: &mut Sha256, value: ValueRef<'_>) {
    match value {
        ValueRef::Null => hasher.update(b"null\0"),
        ValueRef::Integer(value) => {
            hasher.update(b"integer\0");
            hasher.update(value.to_le_bytes());
        }
        ValueRef::Real(value) => {
            hasher.update(b"real\0");
            hasher.update(value.to_bits().to_le_bytes());
        }
        ValueRef::Text(value) => {
            hasher.update(b"text\0");
            hasher.update((value.len() as u64).to_le_bytes());
            hasher.update(value);
        }
        ValueRef::Blob(value) => {
            hasher.update(b"blob\0");
            hasher.update((value.len() as u64).to_le_bytes());
            hasher.update(value);
        }
    }
}

fn sqlite_consistent_copy(
    source_path: &Path,
    destination_path: &Path,
) -> Result<(), ServerControlPlaneBackupError> {
    reject_link_or_hardlink(source_path)?;
    let parent = destination_path.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "SQLite backup destination has no parent".to_owned(),
        )
    })?;
    verify_private_permissions(parent, true)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination_path)?
        .sync_all()?;
    set_private_permissions(destination_path, false)?;

    let source = open_database_read_only(source_path)?;
    validate_database_schema_and_state(&source)?;
    let mut destination = Connection::open_with_flags(
        destination_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    {
        let backup = Backup::new(&source, &mut destination)?;
        if backup.step(-1)? != StepResult::Done {
            return Err(ServerControlPlaneBackupError::Verification(
                "SQLite consistency backup did not finish atomically".to_owned(),
            ));
        }
    }
    destination.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")?;
    drop(destination);
    drop(source);
    reject_sqlite_sidecars(destination_path)?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(destination_path)?
        .sync_all()?;
    Ok(sync_directory(parent)?)
}

fn invalidate_all_sessions(database_path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    reject_link_or_hardlink(database_path)?;
    let mut connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.execute_batch(
        "PRAGMA foreign_keys=ON;
         PRAGMA synchronous=FULL;
         PRAGMA secure_delete=ON;
         PRAGMA journal_mode=DELETE;",
    )?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    transaction.execute("DELETE FROM sessions", [])?;
    transaction.commit()?;
    drop(connection);
    reject_sqlite_sidecars(database_path)?;
    OpenOptions::new()
        .read(true)
        .write(true)
        .open(database_path)?
        .sync_all()?;
    let parent = database_path.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "restored database has no parent".to_owned(),
        )
    })?;
    Ok(sync_directory(parent)?)
}

fn reject_sqlite_sidecars(database_path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    let file_name = database_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ServerControlPlaneBackupError::InvalidControlPlane(
                "SQLite database file name is not UTF-8".to_owned(),
            )
        })?;
    let parent = database_path.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "SQLite database has no parent".to_owned(),
        )
    })?;
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = parent.join(format!("{file_name}{suffix}"));
        if sidecar.try_exists()? {
            return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
                "SQLite consistency operation left a sidecar: {}",
                sidecar.display()
            )));
        }
    }
    Ok(())
}

fn digest_regular_file(
    path: &Path,
) -> Result<(u64, String, Option<SystemTime>), ServerControlPlaneBackupError> {
    reject_link_or_hardlink(path)?;
    let metadata_before = fs::metadata(path)?;
    if !metadata_before.is_file() {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "not a regular file: {}",
            path.display()
        )));
    }
    let mut file = File::open(path)?;
    let mut buffer = vec![0_u8; 128 * 1024];
    let mut hasher = Sha256::new();
    let mut length = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        length = length.checked_add(read as u64).ok_or_else(|| {
            ServerControlPlaneBackupError::Verification("file length overflow".to_owned())
        })?;
        if length > MAX_CONTROL_DATABASE_BYTES {
            return Err(ServerControlPlaneBackupError::InvalidControlPlane(
                "control-plane database exceeds the supported size".to_owned(),
            ));
        }
    }
    let metadata_after = file.metadata()?;
    if length != metadata_before.len()
        || length != metadata_after.len()
        || metadata_before.modified().ok() != metadata_after.modified().ok()
    {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    Ok((
        length,
        format!("{:x}", hasher.finalize()),
        metadata_after.modified().ok(),
    ))
}

fn verify_private_control_plane_tree(root: &Path) -> Result<(), ServerControlPlaneBackupError> {
    verify_private_permissions(root, true)?;
    for name in control_root_entry_names(root)? {
        let path = root.join(name);
        reject_link_or_hardlink(&path)?;
        verify_private_permissions(&path, false)?;
    }
    Ok(())
}

fn verify_private_snapshot_tree(root: &Path) -> Result<(), ServerControlPlaneBackupError> {
    verify_private_permissions(root, true)?;
    for name in [
        SERVER_CONTROL_PLANE_DATABASE_FILE,
        SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE,
        SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE,
    ] {
        let path = root.join(name);
        reject_link_or_hardlink(&path)?;
        verify_private_permissions(&path, false)?;
    }
    Ok(())
}

fn read_private_bounded_file(
    path: &Path,
    root: &Path,
    limit: u64,
) -> Result<Vec<u8>, ServerControlPlaneBackupError> {
    reject_link_or_hardlink(path)?;
    verify_private_permissions(path, false)?;
    Ok(read_bounded_regular_file(path, root, limit)?)
}

fn create_private_directory(path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    reject_linked_existing_ancestors(path)?;
    if path.exists() {
        return Err(ServerControlPlaneBackupError::RestoreTargetExists(
            path.to_path_buf(),
        ));
    }
    fs::create_dir(path)?;
    set_private_permissions(path, true)?;
    let canonical = fs::canonicalize(path)?;
    if canonical != path {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "new private directory escaped its reviewed path: {}",
            canonical.display()
        )));
    }
    sync_directory(path.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "new private directory has no parent".to_owned(),
        )
    })?)
    .map_err(Into::into)
}

fn require_disjoint(left: &Path, right: &Path) -> Result<(), ServerControlPlaneBackupError> {
    if left == right || left.starts_with(right) || right.starts_with(left) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "control plane and portable workspace/backup targets must be disjoint: {} versus {}",
            left.display(),
            right.display()
        )));
    }
    Ok(())
}

fn require_v4(value: Uuid, label: &str) -> Result<(), ServerControlPlaneBackupError> {
    if value.get_version() != Some(Version::Random) {
        return Err(ServerControlPlaneBackupError::Verification(format!(
            "{label} ID must be UUIDv4"
        )));
    }
    Ok(())
}

fn require_sha256(value: &str, label: &str) -> Result<(), ServerControlPlaneBackupError> {
    if !is_lower_hex(value, 64) {
        return Err(ServerControlPlaneBackupError::Verification(format!(
            "{label} digest must be lowercase SHA-256"
        )));
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn pretty_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, ServerControlPlaneBackupError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(unix)]
fn open_exclusive_lease(path: &Path) -> Result<File, ServerControlPlaneBackupError> {
    use std::os::unix::fs::OpenOptionsExt;

    use rustix::fs::{FlockOperation, flock};

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    flock(&file, FlockOperation::NonBlockingLockExclusive)
        .map_err(|_| ServerControlPlaneBackupError::ControlPlaneInUse(path.to_path_buf()))?;
    Ok(file)
}

#[cfg(windows)]
fn open_exclusive_lease(path: &Path) -> Result<File, ServerControlPlaneBackupError> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(FILE_SHARE_DELETE)
        .open(path)
        .map_err(|error| {
            if matches!(error.raw_os_error(), Some(32 | 33))
                || matches!(
                    error.kind(),
                    io::ErrorKind::PermissionDenied | io::ErrorKind::WouldBlock
                )
            {
                ServerControlPlaneBackupError::ControlPlaneInUse(path.to_path_buf())
            } else {
                ServerControlPlaneBackupError::Io(error)
            }
        })
}

#[cfg(not(any(unix, windows)))]
fn open_exclusive_lease(_path: &Path) -> Result<File, ServerControlPlaneBackupError> {
    Err(ServerControlPlaneBackupError::InvalidControlPlane(
        "cross-process control-plane lease is unsupported on this platform".to_owned(),
    ))
}

fn reject_link_or_hardlink(path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    reject_link_or_reparse(path)?;
    reject_hardlink(path)
}

fn reject_link_or_reparse(path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    let metadata = fs::symlink_metadata(path)?;
    if linked_or_reparse(&metadata) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "linked or reparsed control-plane path: {}",
            path.display()
        )));
    }
    Ok(())
}

fn reject_hardlink(path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    if file_has_multiple_links(path)? {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "hard-linked control-plane file: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn file_has_multiple_links(path: &Path) -> Result<bool, ServerControlPlaneBackupError> {
    use std::os::unix::fs::MetadataExt;

    Ok(fs::metadata(path)?.nlink() > 1)
}

#[cfg(windows)]
fn file_has_multiple_links(path: &Path) -> Result<bool, ServerControlPlaneBackupError> {
    use std::process::Command;

    let output = Command::new("fsutil")
        .arg("hardlink")
        .arg("list")
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "cannot verify Windows hard-link boundary for {}",
            path.display()
        )));
    }
    let links = String::from_utf8(output.stdout).map_err(|_| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "Windows hard-link inventory is not UTF-8".to_owned(),
        )
    })?;
    Ok(links.lines().filter(|line| !line.trim().is_empty()).count() > 1)
}

#[cfg(not(any(unix, windows)))]
fn file_has_multiple_links(_path: &Path) -> Result<bool, ServerControlPlaneBackupError> {
    Err(ServerControlPlaneBackupError::InvalidControlPlane(
        "hard-link verification is unsupported on this platform".to_owned(),
    ))
}

#[cfg(unix)]
fn set_private_permissions(
    path: &Path,
    directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    use std::os::unix::fs::PermissionsExt;

    let mode = if directory { 0o700 } else { 0o600 };
    fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
    verify_private_permissions(path, directory)
}

#[cfg(unix)]
fn verify_private_permissions(
    path: &Path,
    directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;
    let expected = if directory { 0o700 } else { 0o600 };
    if metadata.permissions().mode() & 0o777 != expected {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "private Unix mode must be {expected:o}: {}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn set_private_permissions(
    path: &Path,
    directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    windows_private_acl(path, directory, true)
}

#[cfg(windows)]
fn verify_private_permissions(
    path: &Path,
    directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    windows_private_acl(path, directory, false)
}

#[cfg(not(any(unix, windows)))]
fn set_private_permissions(
    _path: &Path,
    _directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    Err(ServerControlPlaneBackupError::InvalidControlPlane(
        "private permission enforcement is unsupported on this platform".to_owned(),
    ))
}

#[cfg(not(any(unix, windows)))]
fn verify_private_permissions(
    _path: &Path,
    _directory: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    Err(ServerControlPlaneBackupError::InvalidControlPlane(
        "private permission verification is unsupported on this platform".to_owned(),
    ))
}

#[cfg(windows)]
fn windows_private_acl(
    path: &Path,
    directory: bool,
    apply: bool,
) -> Result<(), ServerControlPlaneBackupError> {
    use std::process::Command;

    const SCRIPT: &str = r"
$ErrorActionPreference = 'Stop'
$path = [Environment]::GetEnvironmentVariable('WEFTEXT_BACKUP_ACL_PATH', 'Process')
$isDirectory = [Environment]::GetEnvironmentVariable('WEFTEXT_BACKUP_ACL_DIRECTORY', 'Process') -eq '1'
$apply = [Environment]::GetEnvironmentVariable('WEFTEXT_BACKUP_ACL_APPLY', 'Process') -eq '1'
if ([String]::IsNullOrWhiteSpace($path)) { throw 'missing control-plane ACL path' }
$current = [Security.Principal.WindowsIdentity]::GetCurrent().User
$system = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$administrators = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
if ($isDirectory) {
    $item = [IO.DirectoryInfo]::new($path)
} else {
    $item = [IO.FileInfo]::new($path)
}
if ($apply) {
    if ($isDirectory) {
        $acl = [Security.AccessControl.DirectorySecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::ContainerInherit -bor [Security.AccessControl.InheritanceFlags]::ObjectInherit
    } else {
        $acl = [Security.AccessControl.FileSecurity]::new()
        $inheritance = [Security.AccessControl.InheritanceFlags]::None
    }
    $acl.SetAccessRuleProtection($true, $false)
    $acl.SetOwner($current)
    foreach ($sid in @($current, $system, $administrators)) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new(
            $sid,
            [Security.AccessControl.FileSystemRights]::FullControl,
            $inheritance,
            [Security.AccessControl.PropagationFlags]::None,
            [Security.AccessControl.AccessControlType]::Allow
        )
        [void]$acl.AddAccessRule($rule)
    }
    $item.SetAccessControl($acl)
}
$verified = $item.GetAccessControl()
if (-not $verified.AreAccessRulesProtected) { throw 'control-plane ACL still inherits' }
$owner = $verified.GetOwner([Security.Principal.SecurityIdentifier]).Value
if ($owner -ne $current.Value) { throw 'control-plane ACL owner mismatch' }
$allowed = @($current.Value, $system.Value, $administrators.Value)
foreach ($rule in $verified.Access) {
    $sid = $rule.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value
    if ($allowed -notcontains $sid) { throw 'unexpected control-plane ACL principal' }
    if ($rule.AccessControlType -ne [Security.AccessControl.AccessControlType]::Allow) { throw 'unexpected deny rule' }
    if (($rule.FileSystemRights -band [Security.AccessControl.FileSystemRights]::FullControl) -ne [Security.AccessControl.FileSystemRights]::FullControl) { throw 'insufficient control-plane ACL rights' }
}
";
    let path_text = path.to_str().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "Windows control-plane ACL path is not Unicode".to_owned(),
        )
    })?;
    let powershell_path = path_text.strip_prefix(r"\\?\UNC\").map_or_else(
        || {
            path_text
                .strip_prefix(r"\\?\")
                .unwrap_or(path_text)
                .to_owned()
        },
        |unc| format!(r"\\{unc}"),
    );
    let result = Command::new("powershell.exe")
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(SCRIPT)
        .env("WEFTEXT_BACKUP_ACL_PATH", powershell_path)
        .env(
            "WEFTEXT_BACKUP_ACL_DIRECTORY",
            if directory { "1" } else { "0" },
        )
        .env("WEFTEXT_BACKUP_ACL_APPLY", if apply { "1" } else { "0" })
        .output()?;
    if !result.status.success() {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(format!(
            "Windows control-plane ACL boundary failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&result.stderr).trim()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod workspace_match_lease_tests {
    use super::*;
    use weftext_core::WorkspaceTransactionError;

    #[test]
    fn snapshot_match_holds_workspace_lease_until_every_check_finishes() {
        let temporary = tempfile::tempdir().unwrap();
        let workspace = temporary.path().join("Workspace");
        let backups = temporary.path().join("backups");
        fs::create_dir(&backups).unwrap();
        weftext_core::create_workspace(&workspace).unwrap();
        let workspace = fs::canonicalize(workspace).unwrap();
        let plan = crate::plan_full_workspace_backup(&workspace, &backups).unwrap();
        crate::commit_full_workspace_backup(&plan).unwrap();
        let snapshot = crate::verify_snapshot_internal(&plan.snapshot_directory).unwrap();

        let displaced = temporary.path().join("held-workspace-anchor");
        let result =
            require_workspace_matches_snapshot_with_lease_probe(&workspace, &snapshot, || {
                fs::rename(
                    workspace.join(weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
                    &displaced,
                )?;
                fs::write(
                    workspace.join(weftext_core::WORKSPACE_TRANSACTION_LEASE_FILE_NAME),
                    [],
                )?;
                assert!(acquire_core_workspace_transaction_lease(&workspace).is_ok());
                // The temporary second lease is dropped here; the original
                // guard must still reject success because its identity moved.
                Ok(())
            });
        assert!(matches!(
            result,
            Err(ServerControlPlaneBackupError::Backup(
                BackupError::CoreTransaction(WorkspaceTransactionError::RecoveryRequired(_))
            ))
        ));

        drop(acquire_core_workspace_transaction_lease(&workspace).unwrap());
    }
}
