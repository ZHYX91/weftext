//! One-operation orchestration for exact portable/control-plane backup and restore pairs.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;
use weftext_core::{NodeId, WorkspaceRevision};

use super::{
    AlternateServerControlPlaneRestoreReceipt, BootstrapSecretBackupPolicy,
    CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX, ControlPlaneDatabaseEvidence, PairedWorkspaceSnapshot,
    ReverseProxySecretBackupPolicy, ServerControlPlaneBackupError,
    ServerControlPlaneBackupVerification, ServerControlPlaneLease, SessionRestorePolicy,
    acquire_server_control_plane_lease, canonical_existing_directory,
    commit_alternate_server_control_plane_restore, commit_server_control_plane_backup,
    commit_server_control_plane_backup_with_lease, inspect_control_plane,
    inspect_control_plane_mode, normalize_new_destination, paired_workspace_snapshot, path_binding,
    replan_alternate_server_control_plane_restore, replan_server_control_plane_backup,
    replan_server_control_plane_backup_with_lease, require_disjoint, require_v4,
    require_workspace_matches_snapshot, verify_alternate_server_control_plane_restore,
    verify_server_control_plane_snapshot, verify_server_control_plane_snapshot_internal,
    verify_snapshot_internal,
};
use crate::{
    AlternateRestorePlan, FullWorkspaceBackupPlan, FullWorkspaceBackupVerification,
    commit_alternate_restore, commit_full_workspace_backup, replan_alternate_restore,
    replan_full_workspace_backup,
};

pub const SERVER_BACKUP_PAIR_PLAN_SCHEMA: &str = "weftext.server-backup-pair-plan.v1";
pub const SERVER_BACKUP_PAIR_RECEIPT_SCHEMA: &str = "weftext.server-backup-pair-receipt.v1";
pub const SERVER_BACKUP_PAIR_VERIFICATION_SCHEMA: &str =
    "weftext.server-backup-pair-verification.v1";
pub const SERVER_RESTORE_PAIR_PLAN_SCHEMA: &str = "weftext.server-restore-pair-plan.v1";
pub const SERVER_RESTORE_PAIR_RECEIPT_SCHEMA: &str = "weftext.server-restore-pair-receipt.v1";
pub const SERVER_RESTORE_PAIR_VERIFICATION_SCHEMA: &str =
    "weftext.server-restore-pair-verification.v1";
pub const RESTORED_WORKSPACE_VERIFICATION_SCHEMA: &str =
    "weftext.restored-workspace-verification.v1";

const WORKSPACE_BACKUP_PREFIX: &str = "weftext-backup-";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerPairCommitState {
    Pending,
    WorkspaceCompleteControlPlanePending,
    Complete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverseProxySecretRestoreAction {
    RegenerateAndRotateAtFirstServerStart,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupPairPlan {
    pub schema: String,
    pub workspace_snapshot_id: Uuid,
    pub control_plane_backup_id: Uuid,
    pub plan_digest: String,
    pub workspace_plan_digest: String,
    pub workspace_root: PathBuf,
    pub control_plane_root: PathBuf,
    pub backup_parent: PathBuf,
    pub workspace_snapshot_directory: PathBuf,
    pub control_plane_snapshot_directory: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub control_plane_database: ControlPlaneDatabaseEvidence,
    pub excluded_operational_files: Vec<String>,
    pub bootstrap_secret_policy: BootstrapSecretBackupPolicy,
    pub reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    pub session_restore_policy: SessionRestorePolicy,
    pub commit_state: ServerPairCommitState,
    #[serde(skip)]
    workspace_plan: Option<FullWorkspaceBackupPlan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupPairVerification {
    pub schema: String,
    pub workspace_snapshot: FullWorkspaceBackupVerification,
    pub control_plane_snapshot: ServerControlPlaneBackupVerification,
    pub exact_pair: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerBackupPairReceipt {
    pub schema: String,
    pub workspace_snapshot_id: Uuid,
    pub control_plane_backup_id: Uuid,
    pub plan_digest: String,
    pub workspace_snapshot_directory: PathBuf,
    pub control_plane_snapshot_directory: PathBuf,
    pub resumed_from: ServerPairCommitState,
    pub verification: ServerBackupPairVerification,
    pub complete: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRestorePairPlan {
    pub schema: String,
    pub workspace_restore_id: Uuid,
    pub control_plane_restore_id: Uuid,
    pub plan_digest: String,
    pub workspace_snapshot_directory: PathBuf,
    pub control_plane_snapshot_directory: PathBuf,
    pub restored_workspace_root: PathBuf,
    pub restored_control_plane_root: PathBuf,
    pub workspace_snapshot: PairedWorkspaceSnapshot,
    pub control_plane_backup_id: Uuid,
    pub control_plane_manifest_sha256: String,
    pub session_restore_policy: SessionRestorePolicy,
    pub reverse_proxy_secret_policy: ReverseProxySecretBackupPolicy,
    pub reverse_proxy_secret_action: ReverseProxySecretRestoreAction,
    pub commit_state: ServerPairCommitState,
    #[serde(skip)]
    workspace_plan: Option<AlternateRestorePlan>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRestorePairVerification {
    pub schema: String,
    pub workspace_restore: RestoredWorkspaceVerification,
    pub control_plane_restore: AlternateServerControlPlaneRestoreReceipt,
    pub reverse_proxy_secret_action: ReverseProxySecretRestoreAction,
    pub exact_pair: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoredWorkspaceVerification {
    pub schema: String,
    pub snapshot_id: Uuid,
    pub destination_root: PathBuf,
    pub workspace_root_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub manifest_sha256: String,
    pub entry_count: u64,
    pub total_bytes: u64,
    pub bytewise_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRestorePairReceipt {
    pub schema: String,
    pub workspace_restore_id: Uuid,
    pub control_plane_restore_id: Uuid,
    pub plan_digest: String,
    pub restored_workspace_root: PathBuf,
    pub restored_control_plane_root: PathBuf,
    pub resumed_from: ServerPairCommitState,
    pub verification: ServerRestorePairVerification,
    pub complete: bool,
}

/// Creates a read-only preview for one portable/control-plane backup pair.
///
/// # Errors
///
/// Fails unless both sources are safe, the Server lease proves the control plane is stopped,
/// both create-new destinations are disjoint, and the control plane is initialized.
pub fn plan_server_backup_pair(
    workspace_root: impl AsRef<Path>,
    control_plane_root: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
) -> Result<ServerBackupPairPlan, ServerControlPlaneBackupError> {
    replan_server_backup_pair(
        workspace_root,
        control_plane_root,
        backup_parent,
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
}

/// Creates a read-only pair preview under the owning Server's already-held exclusive lease.
/// The caller must drain and block all other API requests for the duration of this call.
///
/// # Errors
///
/// Has the same validation as [`plan_server_backup_pair`] and rejects a lease for another root.
pub fn plan_server_backup_pair_with_lease(
    workspace_root: impl AsRef<Path>,
    lease: &ServerControlPlaneLease,
    backup_parent: impl AsRef<Path>,
) -> Result<ServerBackupPairPlan, ServerControlPlaneBackupError> {
    replan_server_backup_pair_internal(
        workspace_root,
        lease.root(),
        backup_parent,
        Uuid::new_v4(),
        Uuid::new_v4(),
        Some(lease),
    )
}

/// Rebuilds a pair preview or an exact resumable state for reviewed `UUIDv4` identities.
///
/// # Errors
///
/// Fails closed for stale, tampered, incomplete, colliding, live, or mismatched state.
#[allow(clippy::too_many_lines)]
pub fn replan_server_backup_pair(
    workspace_root: impl AsRef<Path>,
    control_plane_root: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
    workspace_snapshot_id: Uuid,
    control_plane_backup_id: Uuid,
) -> Result<ServerBackupPairPlan, ServerControlPlaneBackupError> {
    replan_server_backup_pair_internal(
        workspace_root,
        control_plane_root,
        backup_parent,
        workspace_snapshot_id,
        control_plane_backup_id,
        None,
    )
}

/// Rebuilds a reviewed pair while borrowing the owning Server's exclusive lease and quiescence.
///
/// # Errors
///
/// Has the same validation as [`replan_server_backup_pair`].
pub fn replan_server_backup_pair_with_lease(
    workspace_root: impl AsRef<Path>,
    lease: &ServerControlPlaneLease,
    backup_parent: impl AsRef<Path>,
    workspace_snapshot_id: Uuid,
    control_plane_backup_id: Uuid,
) -> Result<ServerBackupPairPlan, ServerControlPlaneBackupError> {
    replan_server_backup_pair_internal(
        workspace_root,
        lease.root(),
        backup_parent,
        workspace_snapshot_id,
        control_plane_backup_id,
        Some(lease),
    )
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn replan_server_backup_pair_internal(
    workspace_root: impl AsRef<Path>,
    control_plane_root: impl AsRef<Path>,
    backup_parent: impl AsRef<Path>,
    workspace_snapshot_id: Uuid,
    control_plane_backup_id: Uuid,
    borrowed_lease: Option<&ServerControlPlaneLease>,
) -> Result<ServerBackupPairPlan, ServerControlPlaneBackupError> {
    require_v4(workspace_snapshot_id, "workspace snapshot")?;
    require_v4(control_plane_backup_id, "control-plane backup")?;
    let workspace_root = canonical_existing_directory(workspace_root.as_ref(), "workspace root")?;
    let control_plane_root =
        canonical_existing_directory(control_plane_root.as_ref(), "control-plane root")?;
    let backup_parent = canonical_existing_directory(backup_parent.as_ref(), "backup parent")?;
    require_disjoint(&workspace_root, &control_plane_root)?;
    require_disjoint(&workspace_root, &backup_parent)?;
    require_disjoint(&control_plane_root, &backup_parent)?;

    let workspace_snapshot_directory = backup_parent.join(format!(
        "{WORKSPACE_BACKUP_PREFIX}{}",
        workspace_snapshot_id.hyphenated()
    ));
    let control_plane_snapshot_directory = backup_parent.join(format!(
        "{CONTROL_PLANE_BACKUP_DIRECTORY_PREFIX}{}",
        control_plane_backup_id.hyphenated()
    ));
    reject_case_collision(&backup_parent, &workspace_snapshot_directory)?;
    reject_case_collision(&backup_parent, &control_plane_snapshot_directory)?;

    let workspace_exists = workspace_snapshot_directory.try_exists()?;
    let control_exists = control_plane_snapshot_directory.try_exists()?;
    if control_exists && !workspace_exists {
        return Err(ServerControlPlaneBackupError::Verification(
            "control-plane snapshot exists without its portable workspace snapshot".to_owned(),
        ));
    }

    let (workspace_snapshot, workspace_plan) = if workspace_exists {
        let verified = verify_snapshot_internal(&workspace_snapshot_directory)?;
        if verified.snapshot_id != workspace_snapshot_id {
            return Err(ServerControlPlaneBackupError::Verification(
                "portable snapshot directory does not bind the reviewed snapshot ID".to_owned(),
            ));
        }
        if !control_exists {
            require_workspace_matches_snapshot(&workspace_root, &verified)?;
        }
        (paired_workspace_snapshot(&verified), None)
    } else {
        let plan =
            replan_full_workspace_backup(&workspace_root, &backup_parent, workspace_snapshot_id)?;
        (paired_workspace_backup_plan(&plan), Some(plan))
    };

    let (control_plane_database, excluded_operational_files, commit_state) = if control_exists {
        let verified = verify_server_control_plane_snapshot_internal(
            &control_plane_snapshot_directory,
            &workspace_snapshot_directory,
        )?;
        if verified.manifest.backup_id != control_plane_backup_id {
            return Err(ServerControlPlaneBackupError::Verification(
                "control-plane snapshot directory does not bind the reviewed backup ID".to_owned(),
            ));
        }
        (
            verified.manifest.database,
            verified.manifest.excluded_operational_files,
            ServerPairCommitState::Complete,
        )
    } else {
        let inspected = if let Some(lease) = borrowed_lease {
            if lease.root() != control_plane_root {
                return Err(ServerControlPlaneBackupError::InvalidControlPlane(
                    "borrowed lease does not bind the reviewed control-plane root".to_owned(),
                ));
            }
            inspect_control_plane_mode(lease, true)?
        } else {
            let lease = acquire_server_control_plane_lease(&control_plane_root)?;
            inspect_control_plane(&lease)?
        };
        (
            inspected.database,
            inspected.excluded_operational_files,
            if workspace_exists {
                ServerPairCommitState::WorkspaceCompleteControlPlanePending
            } else {
                ServerPairCommitState::Pending
            },
        )
    };

    let workspace_plan_digest =
        portable_backup_plan_digest(&workspace_snapshot, &workspace_root, &backup_parent)?;
    let plan_digest = backup_pair_plan_digest(
        workspace_snapshot_id,
        control_plane_backup_id,
        &workspace_plan_digest,
        &workspace_snapshot,
        &control_plane_database,
        &excluded_operational_files,
        &workspace_root,
        &control_plane_root,
        &backup_parent,
    )?;
    Ok(ServerBackupPairPlan {
        schema: SERVER_BACKUP_PAIR_PLAN_SCHEMA.to_owned(),
        workspace_snapshot_id,
        control_plane_backup_id,
        plan_digest,
        workspace_plan_digest,
        workspace_root,
        control_plane_root,
        backup_parent,
        workspace_snapshot_directory,
        control_plane_snapshot_directory,
        workspace_snapshot,
        control_plane_database,
        excluded_operational_files,
        bootstrap_secret_policy: BootstrapSecretBackupPolicy::ConsumedRequiredSecretExcluded,
        reverse_proxy_secret_policy:
            ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime,
        session_restore_policy: SessionRestorePolicy::InvalidateAll,
        commit_state,
        workspace_plan,
    })
}

/// Commits or resumes a reviewed pair and verifies both marker-last snapshots.
///
/// A completed portable snapshot is never removed when the control-plane step fails.
///
/// # Errors
///
/// Fails closed for a stale/tampered plan or either underlying backup failure. A failure after the
/// portable step reports [`ServerControlPlaneBackupError::PairIncomplete`].
pub fn commit_server_backup_pair(
    plan: &ServerBackupPairPlan,
) -> Result<ServerBackupPairReceipt, ServerControlPlaneBackupError> {
    commit_server_backup_pair_internal(plan, None)
}

/// Commits a reviewed pair while borrowing the owning Server's process-lifetime lease. The
/// caller must retain an exclusive quiescence barrier for the whole operation.
///
/// # Errors
///
/// Has the same stale/tamper and pair-completion behavior as [`commit_server_backup_pair`].
pub fn commit_server_backup_pair_with_lease(
    lease: &ServerControlPlaneLease,
    plan: &ServerBackupPairPlan,
) -> Result<ServerBackupPairReceipt, ServerControlPlaneBackupError> {
    commit_server_backup_pair_internal(plan, Some(lease))
}

fn commit_server_backup_pair_internal(
    plan: &ServerBackupPairPlan,
    borrowed_lease: Option<&ServerControlPlaneLease>,
) -> Result<ServerBackupPairReceipt, ServerControlPlaneBackupError> {
    validate_backup_pair_plan(plan, borrowed_lease)?;
    if borrowed_lease.is_some_and(|lease| lease.root() != plan.control_plane_root) {
        return Err(ServerControlPlaneBackupError::InvalidControlPlane(
            "borrowed lease does not bind the reviewed control-plane root".to_owned(),
        ));
    }
    let resumed_from = plan.commit_state;
    if plan.commit_state == ServerPairCommitState::Pending {
        let workspace_plan = plan
            .workspace_plan
            .as_ref()
            .ok_or(ServerControlPlaneBackupError::InvalidPlan)?;
        commit_full_workspace_backup(workspace_plan)?;
    }
    if plan.commit_state != ServerPairCommitState::Complete {
        let control_plan = match borrowed_lease {
            Some(lease) => replan_server_control_plane_backup_with_lease(
                lease,
                &plan.workspace_root,
                &plan.workspace_snapshot_directory,
                &plan.backup_parent,
                plan.control_plane_backup_id,
            ),
            None => replan_server_control_plane_backup(
                &plan.control_plane_root,
                &plan.workspace_root,
                &plan.workspace_snapshot_directory,
                &plan.backup_parent,
                plan.control_plane_backup_id,
            ),
        }
        .map_err(|error| incomplete_backup_pair(plan, &error))?;
        if control_plan.workspace_snapshot != plan.workspace_snapshot
            || control_plan.source_database != plan.control_plane_database
            || control_plan.excluded_operational_files != plan.excluded_operational_files
            || control_plan.snapshot_directory != plan.control_plane_snapshot_directory
        {
            return Err(incomplete_backup_pair(
                plan,
                &ServerControlPlaneBackupError::StalePreview,
            ));
        }
        match borrowed_lease {
            Some(lease) => commit_server_control_plane_backup_with_lease(lease, &control_plan),
            None => commit_server_control_plane_backup(&control_plan),
        }
        .map_err(|error| incomplete_backup_pair(plan, &error))?;
    }
    let verification = verify_server_backup_pair(
        &plan.workspace_snapshot_directory,
        &plan.control_plane_snapshot_directory,
    )?;
    Ok(ServerBackupPairReceipt {
        schema: SERVER_BACKUP_PAIR_RECEIPT_SCHEMA.to_owned(),
        workspace_snapshot_id: plan.workspace_snapshot_id,
        control_plane_backup_id: plan.control_plane_backup_id,
        plan_digest: plan.plan_digest.clone(),
        workspace_snapshot_directory: plan.workspace_snapshot_directory.clone(),
        control_plane_snapshot_directory: plan.control_plane_snapshot_directory.clone(),
        resumed_from,
        verification,
        complete: true,
    })
}

/// Verifies the exact workspace/control-plane snapshot binding.
///
/// # Errors
///
/// Rejects either invalid snapshot or a control snapshot paired to another workspace snapshot.
pub fn verify_server_backup_pair(
    workspace_snapshot_directory: impl AsRef<Path>,
    control_plane_snapshot_directory: impl AsRef<Path>,
) -> Result<ServerBackupPairVerification, ServerControlPlaneBackupError> {
    let workspace_snapshot =
        crate::verify_full_workspace_snapshot(workspace_snapshot_directory.as_ref())?;
    let control_plane_snapshot = verify_server_control_plane_snapshot(
        control_plane_snapshot_directory.as_ref(),
        workspace_snapshot_directory.as_ref(),
    )?;
    Ok(ServerBackupPairVerification {
        schema: SERVER_BACKUP_PAIR_VERIFICATION_SCHEMA.to_owned(),
        workspace_snapshot,
        control_plane_snapshot,
        exact_pair: true,
    })
}

/// Creates a read-only alternate restore preview for an exact Server backup pair.
///
/// # Errors
///
/// Rejects a tampered/mismatched pair and any existing, linked, colliding, or overlapping target.
pub fn plan_server_restore_pair(
    workspace_snapshot_directory: impl AsRef<Path>,
    control_plane_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
    restored_control_plane_root: impl AsRef<Path>,
) -> Result<ServerRestorePairPlan, ServerControlPlaneBackupError> {
    if restored_workspace_root.as_ref().try_exists()? {
        return Err(ServerControlPlaneBackupError::RestoreTargetExists(
            restored_workspace_root.as_ref().to_path_buf(),
        ));
    }
    if restored_control_plane_root.as_ref().try_exists()? {
        return Err(ServerControlPlaneBackupError::RestoreTargetExists(
            restored_control_plane_root.as_ref().to_path_buf(),
        ));
    }
    replan_server_restore_pair(
        workspace_snapshot_directory,
        control_plane_snapshot_directory,
        restored_workspace_root,
        restored_control_plane_root,
        Uuid::new_v4(),
        Uuid::new_v4(),
    )
}

/// Rebuilds or resumes an exact create-new alternate restore pair.
///
/// # Errors
///
/// Fails closed for stale/tampered sources, target collisions, partial corruption, or overlap.
#[allow(clippy::too_many_arguments)]
pub fn replan_server_restore_pair(
    workspace_snapshot_directory: impl AsRef<Path>,
    control_plane_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
    restored_control_plane_root: impl AsRef<Path>,
    workspace_restore_id: Uuid,
    control_plane_restore_id: Uuid,
) -> Result<ServerRestorePairPlan, ServerControlPlaneBackupError> {
    require_v4(workspace_restore_id, "workspace restore")?;
    require_v4(control_plane_restore_id, "control-plane restore")?;
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory.as_ref())?;
    let control_snapshot = verify_server_control_plane_snapshot_internal(
        control_plane_snapshot_directory.as_ref(),
        &workspace_snapshot.directory,
    )?;
    let restored_workspace_root = resolve_destination(restored_workspace_root.as_ref())?;
    let restored_control_plane_root = resolve_destination(restored_control_plane_root.as_ref())?;
    require_disjoint(&restored_workspace_root, &restored_control_plane_root)?;
    require_disjoint(&restored_workspace_root, &workspace_snapshot.directory)?;
    require_disjoint(&restored_workspace_root, &control_snapshot.directory)?;
    require_disjoint(&restored_control_plane_root, &workspace_snapshot.directory)?;
    require_disjoint(&restored_control_plane_root, &control_snapshot.directory)?;
    reject_case_collision_for_target(&restored_workspace_root)?;
    reject_case_collision_for_target(&restored_control_plane_root)?;

    let workspace_exists = restored_workspace_root.try_exists()?;
    let control_exists = restored_control_plane_root.try_exists()?;
    if control_exists && !workspace_exists {
        return Err(ServerControlPlaneBackupError::Verification(
            "restored control plane exists without its restored portable workspace".to_owned(),
        ));
    }
    let workspace_plan = if workspace_exists {
        require_workspace_matches_snapshot(&restored_workspace_root, &workspace_snapshot)?;
        None
    } else {
        Some(replan_alternate_restore(
            &workspace_snapshot.directory,
            &restored_workspace_root,
            workspace_restore_id,
        )?)
    };
    let commit_state = if control_exists {
        verify_alternate_server_control_plane_restore(
            &restored_control_plane_root,
            &control_snapshot.directory,
            &workspace_snapshot.directory,
            &restored_workspace_root,
        )?;
        ServerPairCommitState::Complete
    } else if workspace_exists {
        ServerPairCommitState::WorkspaceCompleteControlPlanePending
    } else {
        ServerPairCommitState::Pending
    };
    let paired = paired_workspace_snapshot(&workspace_snapshot);
    let plan_digest = restore_pair_plan_digest(
        workspace_restore_id,
        control_plane_restore_id,
        &paired,
        control_snapshot.manifest.backup_id,
        &control_snapshot.manifest_sha256,
        &workspace_snapshot.directory,
        &control_snapshot.directory,
        &restored_workspace_root,
        &restored_control_plane_root,
    )?;
    Ok(ServerRestorePairPlan {
        schema: SERVER_RESTORE_PAIR_PLAN_SCHEMA.to_owned(),
        workspace_restore_id,
        control_plane_restore_id,
        plan_digest,
        workspace_snapshot_directory: workspace_snapshot.directory,
        control_plane_snapshot_directory: control_snapshot.directory,
        restored_workspace_root,
        restored_control_plane_root,
        workspace_snapshot: paired,
        control_plane_backup_id: control_snapshot.manifest.backup_id,
        control_plane_manifest_sha256: control_snapshot.manifest_sha256,
        session_restore_policy: SessionRestorePolicy::InvalidateAll,
        reverse_proxy_secret_policy:
            ReverseProxySecretBackupPolicy::ExcludedRegenerateAndRotateAtRuntime,
        reverse_proxy_secret_action:
            ReverseProxySecretRestoreAction::RegenerateAndRotateAtFirstServerStart,
        commit_state,
        workspace_plan,
    })
}

/// Commits or resumes both create-new alternate restore targets and verifies their exact pair.
///
/// # Errors
///
/// A failure after restoring the portable workspace leaves it intact and reports the resumable
/// partial pair. Existing unrelated targets are never overwritten.
pub fn commit_server_restore_pair(
    plan: &ServerRestorePairPlan,
) -> Result<ServerRestorePairReceipt, ServerControlPlaneBackupError> {
    validate_restore_pair_plan(plan)?;
    let resumed_from = plan.commit_state;
    if plan.commit_state == ServerPairCommitState::Pending {
        let workspace_plan = plan
            .workspace_plan
            .as_ref()
            .ok_or(ServerControlPlaneBackupError::InvalidPlan)?;
        commit_alternate_restore(workspace_plan)?;
    }
    if plan.commit_state != ServerPairCommitState::Complete {
        let control_plan = replan_alternate_server_control_plane_restore(
            &plan.control_plane_snapshot_directory,
            &plan.workspace_snapshot_directory,
            &plan.restored_workspace_root,
            &plan.restored_control_plane_root,
            plan.control_plane_restore_id,
        )
        .map_err(|error| incomplete_restore_pair(plan, &error))?;
        if control_plan.backup_id != plan.control_plane_backup_id
            || control_plan.control_plane_manifest_sha256 != plan.control_plane_manifest_sha256
            || control_plan.workspace_snapshot != plan.workspace_snapshot
            || control_plan.destination_control_plane_root != plan.restored_control_plane_root
        {
            return Err(incomplete_restore_pair(
                plan,
                &ServerControlPlaneBackupError::StalePreview,
            ));
        }
        commit_alternate_server_control_plane_restore(&control_plan)
            .map_err(|error| incomplete_restore_pair(plan, &error))?;
    }
    let verification = verify_server_restore_pair(
        &plan.workspace_snapshot_directory,
        &plan.control_plane_snapshot_directory,
        &plan.restored_workspace_root,
        &plan.restored_control_plane_root,
    )?;
    if verification.control_plane_restore.restore_id != plan.control_plane_restore_id {
        return Err(ServerControlPlaneBackupError::Verification(
            "restored control plane does not bind the reviewed restore ID".to_owned(),
        ));
    }
    Ok(ServerRestorePairReceipt {
        schema: SERVER_RESTORE_PAIR_RECEIPT_SCHEMA.to_owned(),
        workspace_restore_id: plan.workspace_restore_id,
        control_plane_restore_id: plan.control_plane_restore_id,
        plan_digest: plan.plan_digest.clone(),
        restored_workspace_root: plan.restored_workspace_root.clone(),
        restored_control_plane_root: plan.restored_control_plane_root.clone(),
        resumed_from,
        verification,
        complete: true,
    })
}

/// Verifies both alternate targets against their exact source snapshot pair.
///
/// # Errors
///
/// Rejects byte, identity, receipt, session-policy, secret-policy, permission, or pair mismatch.
#[allow(clippy::too_many_arguments)]
pub fn verify_server_restore_pair(
    workspace_snapshot_directory: impl AsRef<Path>,
    control_plane_snapshot_directory: impl AsRef<Path>,
    restored_workspace_root: impl AsRef<Path>,
    restored_control_plane_root: impl AsRef<Path>,
) -> Result<ServerRestorePairVerification, ServerControlPlaneBackupError> {
    let workspace_snapshot = verify_snapshot_internal(workspace_snapshot_directory.as_ref())?;
    let restored_workspace_root =
        canonical_existing_directory(restored_workspace_root.as_ref(), "restored workspace root")?;
    require_workspace_matches_snapshot(&restored_workspace_root, &workspace_snapshot)?;
    let workspace_restore = RestoredWorkspaceVerification {
        schema: RESTORED_WORKSPACE_VERIFICATION_SCHEMA.to_owned(),
        snapshot_id: workspace_snapshot.snapshot_id,
        destination_root: restored_workspace_root.clone(),
        workspace_root_id: workspace_snapshot.workspace_root_id,
        workspace_revision: workspace_snapshot.workspace_revision.clone(),
        manifest_sha256: workspace_snapshot.manifest_sha256.clone(),
        entry_count: workspace_snapshot.manifest.entry_count,
        total_bytes: workspace_snapshot.manifest.total_bytes,
        bytewise_verified: true,
    };
    let control_plane_restore = verify_alternate_server_control_plane_restore(
        restored_control_plane_root,
        control_plane_snapshot_directory,
        &workspace_snapshot.directory,
        &restored_workspace_root,
    )?;
    Ok(ServerRestorePairVerification {
        schema: SERVER_RESTORE_PAIR_VERIFICATION_SCHEMA.to_owned(),
        workspace_restore,
        control_plane_restore,
        reverse_proxy_secret_action:
            ReverseProxySecretRestoreAction::RegenerateAndRotateAtFirstServerStart,
        exact_pair: true,
    })
}

fn paired_workspace_backup_plan(plan: &FullWorkspaceBackupPlan) -> PairedWorkspaceSnapshot {
    PairedWorkspaceSnapshot {
        snapshot_id: plan.snapshot_id,
        manifest_sha256: plan.manifest_sha256.clone(),
        workspace_root_id: plan.workspace_root_id.to_string(),
        workspace_revision: plan.workspace_revision.to_string(),
        entry_count: plan.entry_count,
        total_bytes: plan.total_bytes,
    }
}

fn portable_backup_plan_digest(
    snapshot: &PairedWorkspaceSnapshot,
    workspace_root: &Path,
    backup_parent: &Path,
) -> Result<String, ServerControlPlaneBackupError> {
    Ok(crate::backup_plan_digest(
        &snapshot.manifest_sha256,
        &path_binding(workspace_root)?,
        &path_binding(backup_parent)?,
    ))
}

#[allow(clippy::too_many_arguments)]
fn backup_pair_plan_digest(
    workspace_snapshot_id: Uuid,
    control_plane_backup_id: Uuid,
    workspace_plan_digest: &str,
    workspace_snapshot: &PairedWorkspaceSnapshot,
    database: &ControlPlaneDatabaseEvidence,
    exclusions: &[String],
    workspace_root: &Path,
    control_plane_root: &Path,
    backup_parent: &Path,
) -> Result<String, ServerControlPlaneBackupError> {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-backup-pair-plan.v1\0");
    for value in [
        workspace_snapshot_id.hyphenated().to_string(),
        control_plane_backup_id.hyphenated().to_string(),
        workspace_plan_digest.to_owned(),
        path_binding(workspace_root)?,
        path_binding(control_plane_root)?,
        path_binding(backup_parent)?,
    ] {
        pair_digest_text(&mut hasher, &value);
    }
    pair_digest_workspace(&mut hasher, workspace_snapshot);
    pair_digest_logical_database(&mut hasher, database);
    for exclusion in exclusions {
        pair_digest_text(&mut hasher, exclusion);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(clippy::too_many_arguments)]
fn restore_pair_plan_digest(
    workspace_restore_id: Uuid,
    control_plane_restore_id: Uuid,
    workspace_snapshot: &PairedWorkspaceSnapshot,
    control_plane_backup_id: Uuid,
    control_plane_manifest_sha256: &str,
    workspace_snapshot_directory: &Path,
    control_plane_snapshot_directory: &Path,
    restored_workspace_root: &Path,
    restored_control_plane_root: &Path,
) -> Result<String, ServerControlPlaneBackupError> {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-restore-pair-plan.v1\0");
    for value in [
        workspace_restore_id.hyphenated().to_string(),
        control_plane_restore_id.hyphenated().to_string(),
        control_plane_backup_id.hyphenated().to_string(),
        control_plane_manifest_sha256.to_owned(),
        path_binding(workspace_snapshot_directory)?,
        path_binding(control_plane_snapshot_directory)?,
        pair_destination_binding(restored_workspace_root)?,
        pair_destination_binding(restored_control_plane_root)?,
    ] {
        pair_digest_text(&mut hasher, &value);
    }
    pair_digest_workspace(&mut hasher, workspace_snapshot);
    Ok(format!("{:x}", hasher.finalize()))
}

fn pair_digest_workspace(hasher: &mut Sha256, snapshot: &PairedWorkspaceSnapshot) {
    for value in [
        snapshot.snapshot_id.hyphenated().to_string(),
        snapshot.manifest_sha256.clone(),
        snapshot.workspace_root_id.clone(),
        snapshot.workspace_revision.clone(),
        snapshot.entry_count.to_string(),
        snapshot.total_bytes.to_string(),
    ] {
        pair_digest_text(hasher, &value);
    }
}

fn pair_digest_logical_database(hasher: &mut Sha256, database: &ControlPlaneDatabaseEvidence) {
    for value in [
        database.schema_sha256.clone(),
        database.workspace_scope_sha256.clone(),
        database.application_id.to_string(),
        database.user_version.to_string(),
    ] {
        pair_digest_text(hasher, &value);
    }
    for table in &database.tables {
        pair_digest_text(hasher, &table.name);
        pair_digest_text(hasher, &table.row_count.to_string());
        pair_digest_text(hasher, &table.sha256);
    }
}

fn pair_digest_text(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn validate_backup_pair_plan(
    plan: &ServerBackupPairPlan,
    borrowed_lease: Option<&ServerControlPlaneLease>,
) -> Result<(), ServerControlPlaneBackupError> {
    let current = replan_server_backup_pair_internal(
        &plan.workspace_root,
        &plan.control_plane_root,
        &plan.backup_parent,
        plan.workspace_snapshot_id,
        plan.control_plane_backup_id,
        borrowed_lease,
    )?;
    if current.schema != plan.schema
        || current.workspace_snapshot_id != plan.workspace_snapshot_id
        || current.control_plane_backup_id != plan.control_plane_backup_id
        || current.plan_digest != plan.plan_digest
        || current.workspace_plan_digest != plan.workspace_plan_digest
        || current.workspace_root != plan.workspace_root
        || current.control_plane_root != plan.control_plane_root
        || current.backup_parent != plan.backup_parent
        || current.workspace_snapshot_directory != plan.workspace_snapshot_directory
        || current.control_plane_snapshot_directory != plan.control_plane_snapshot_directory
        || current.workspace_snapshot != plan.workspace_snapshot
        || current.control_plane_database != plan.control_plane_database
        || current.excluded_operational_files != plan.excluded_operational_files
        || current.bootstrap_secret_policy != plan.bootstrap_secret_policy
        || current.reverse_proxy_secret_policy != plan.reverse_proxy_secret_policy
        || current.session_restore_policy != plan.session_restore_policy
        || current.commit_state != plan.commit_state
    {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    Ok(())
}

fn validate_restore_pair_plan(
    plan: &ServerRestorePairPlan,
) -> Result<(), ServerControlPlaneBackupError> {
    let current = replan_server_restore_pair(
        &plan.workspace_snapshot_directory,
        &plan.control_plane_snapshot_directory,
        &plan.restored_workspace_root,
        &plan.restored_control_plane_root,
        plan.workspace_restore_id,
        plan.control_plane_restore_id,
    )?;
    if current.schema != plan.schema
        || current.workspace_restore_id != plan.workspace_restore_id
        || current.control_plane_restore_id != plan.control_plane_restore_id
        || current.plan_digest != plan.plan_digest
        || current.workspace_snapshot_directory != plan.workspace_snapshot_directory
        || current.control_plane_snapshot_directory != plan.control_plane_snapshot_directory
        || current.restored_workspace_root != plan.restored_workspace_root
        || current.restored_control_plane_root != plan.restored_control_plane_root
        || current.workspace_snapshot != plan.workspace_snapshot
        || current.control_plane_backup_id != plan.control_plane_backup_id
        || current.control_plane_manifest_sha256 != plan.control_plane_manifest_sha256
        || current.session_restore_policy != plan.session_restore_policy
        || current.reverse_proxy_secret_policy != plan.reverse_proxy_secret_policy
        || current.reverse_proxy_secret_action != plan.reverse_proxy_secret_action
        || current.commit_state != plan.commit_state
    {
        return Err(ServerControlPlaneBackupError::StalePreview);
    }
    Ok(())
}

fn incomplete_backup_pair(
    plan: &ServerBackupPairPlan,
    error: &ServerControlPlaneBackupError,
) -> ServerControlPlaneBackupError {
    ServerControlPlaneBackupError::PairIncomplete {
        completed: plan.workspace_snapshot_directory.clone(),
        pending: plan.control_plane_snapshot_directory.clone(),
        cause: error.to_string(),
    }
}

fn incomplete_restore_pair(
    plan: &ServerRestorePairPlan,
    error: &ServerControlPlaneBackupError,
) -> ServerControlPlaneBackupError {
    ServerControlPlaneBackupError::PairIncomplete {
        completed: plan.restored_workspace_root.clone(),
        pending: plan.restored_control_plane_root.clone(),
        cause: error.to_string(),
    }
}

fn resolve_destination(path: &Path) -> Result<PathBuf, ServerControlPlaneBackupError> {
    if path.try_exists()? {
        canonical_existing_directory(path, "existing pair destination").map_err(Into::into)
    } else {
        normalize_new_destination(path).map_err(Into::into)
    }
}

fn pair_destination_binding(path: &Path) -> Result<String, ServerControlPlaneBackupError> {
    let text = path.to_str().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "pair destination path is not UTF-8".to_owned(),
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext.server-pair-destination.v1\0");
    hasher.update(text.as_bytes());
    Ok(format!("{:x}", hasher.finalize()))
}

fn reject_case_collision(
    parent: &Path,
    expected: &Path,
) -> Result<(), ServerControlPlaneBackupError> {
    let expected_name = expected
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            ServerControlPlaneBackupError::InvalidControlPlane(
                "pair destination name is not UTF-8".to_owned(),
            )
        })?;
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            ServerControlPlaneBackupError::InvalidControlPlane(
                "backup parent contains a non-UTF-8 entry".to_owned(),
            )
        })?;
        if name.eq_ignore_ascii_case(expected_name) && name != expected_name {
            return Err(ServerControlPlaneBackupError::Verification(format!(
                "case-colliding pair destination already exists: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn reject_case_collision_for_target(path: &Path) -> Result<(), ServerControlPlaneBackupError> {
    let parent = path.parent().ok_or_else(|| {
        ServerControlPlaneBackupError::InvalidControlPlane(
            "pair destination has no parent".to_owned(),
        )
    })?;
    reject_case_collision(parent, path)
}
