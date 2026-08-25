use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;
use weftext_backup::{
    SnapshotRetentionPolicy, commit_alternate_restore, commit_full_workspace_backup,
    commit_restore_drill, commit_scoped_restore, commit_server_backup_pair,
    commit_server_restore_pair, commit_snapshot_retention, plan_alternate_restore,
    plan_full_workspace_backup, plan_restore_drill, plan_server_backup_pair,
    plan_server_restore_pair, plan_single_node_restore, plan_snapshot_retention,
    plan_subtree_restore, protect_full_workspace_snapshot, recover_snapshot_retention,
    replan_alternate_restore, replan_full_workspace_backup, replan_restore_drill,
    replan_server_backup_pair, replan_server_restore_pair, replan_single_node_restore,
    replan_snapshot_retention, replan_subtree_restore, verify_full_workspace_snapshot,
    verify_server_backup_pair, verify_server_restore_pair,
};
use weftext_core::NodeId;

const SERVER_BACKUP_PREVIEW_REQUEST_SCHEMA: &str = "weftext.cli.server-backup-preview-request.v1";
const SERVER_BACKUP_COMMIT_REQUEST_SCHEMA: &str = "weftext.cli.server-backup-commit-request.v1";
const SERVER_BACKUP_VERIFY_REQUEST_SCHEMA: &str = "weftext.cli.server-backup-verify-request.v1";
const SERVER_RESTORE_PREVIEW_REQUEST_SCHEMA: &str = "weftext.cli.server-restore-preview-request.v1";
const SERVER_RESTORE_COMMIT_REQUEST_SCHEMA: &str = "weftext.cli.server-restore-commit-request.v1";
const SERVER_RESTORE_VERIFY_REQUEST_SCHEMA: &str = "weftext.cli.server-restore-verify-request.v1";
const MAX_SERVER_REQUEST_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupPreviewRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_root: PathBuf,
    control_plane_root: PathBuf,
    backup_parent: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupCommitRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_root: PathBuf,
    control_plane_root: PathBuf,
    backup_parent: PathBuf,
    workspace_snapshot_id: Uuid,
    control_plane_backup_id: Uuid,
    plan_digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupVerifyRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestorePreviewRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    restored_workspace_root: PathBuf,
    restored_control_plane_root: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestoreCommitRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    restored_workspace_root: PathBuf,
    restored_control_plane_root: PathBuf,
    workspace_restore_id: Uuid,
    control_plane_restore_id: Uuid,
    plan_digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestoreVerifyRequest {
    #[serde(rename = "schema")]
    _schema: String,
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    restored_workspace_root: PathBuf,
    restored_control_plane_root: PathBuf,
}

pub(crate) fn run(arguments: &[String], schema: &str) -> Result<serde_json::Value, String> {
    if let Some(server) = run_server_pair(arguments, schema) {
        return server;
    }
    if let Some(scoped_restore) = run_scoped_restore(arguments, schema) {
        return scoped_restore;
    }
    match arguments {
        [scope, command, root, snapshot_parent] if scope == "backup" && command == "preview" => {
            preview_backup(schema, root, snapshot_parent)
        }
        [
            scope,
            command,
            root,
            snapshot_parent,
            snapshot_id,
            plan_digest,
        ] if scope == "backup" && command == "commit" => {
            commit_backup(schema, root, snapshot_parent, snapshot_id, plan_digest)
        }
        [scope, command, snapshot] if scope == "backup" && command == "verify" => {
            verify_backup(schema, snapshot)
        }
        [scope, command, snapshot, label] if scope == "backup" && command == "protect" => {
            protect_backup(schema, snapshot, label)
        }
        [scope, command, snapshot_parent, keep_latest]
            if scope == "backup" && command == "retention-preview" =>
        {
            preview_retention(schema, snapshot_parent, keep_latest)
        }
        [
            scope,
            command,
            snapshot_parent,
            keep_latest,
            operation_id,
            plan_digest,
        ] if scope == "backup" && command == "retention-commit" => commit_retention(
            schema,
            snapshot_parent,
            keep_latest,
            operation_id,
            plan_digest,
        ),
        [scope, command, snapshot_parent]
            if scope == "backup" && command == "retention-recover" =>
        {
            recover_retention(schema, snapshot_parent)
        }
        [scope, command, snapshot, target]
            if scope == "backup" && matches!(command.as_str(), "dry-run" | "restore-preview") =>
        {
            let stage = if command == "dry-run" {
                "dry_run"
            } else {
                "restore_preview"
            };
            preview_restore(schema, snapshot, target, stage)
        }
        [scope, command, snapshot, target, restore_id, plan_digest]
            if scope == "backup" && command == "restore-commit" =>
        {
            commit_restore(schema, snapshot, target, restore_id, plan_digest)
        }
        [scope, command, snapshot, drill_parent, results_parent]
            if scope == "backup" && command == "drill-preview" =>
        {
            preview_drill(schema, snapshot, drill_parent, results_parent)
        }
        [
            scope,
            command,
            snapshot,
            drill_parent,
            results_parent,
            drill_id,
            plan_digest,
        ] if scope == "backup" && command == "drill-commit" => commit_drill(
            schema,
            snapshot,
            drill_parent,
            results_parent,
            drill_id,
            plan_digest,
        ),
        _ => Err(backup_usage()),
    }
}

fn backup_usage() -> String {
    concat!(
        "usage: weftext backup <preview ROOT SNAPSHOT_PARENT|",
        "commit ROOT SNAPSHOT_PARENT SNAPSHOT_ID PLAN_DIGEST|",
        "verify SNAPSHOT|protect SNAPSHOT LABEL|dry-run SNAPSHOT TARGET|",
        "retention-preview SNAPSHOT_PARENT KEEP_LATEST_UNPROTECTED|",
        "retention-commit SNAPSHOT_PARENT KEEP_LATEST_UNPROTECTED OPERATION_ID PLAN_DIGEST|",
        "retention-recover SNAPSHOT_PARENT|restore-preview SNAPSHOT TARGET|",
        "restore-commit SNAPSHOT TARGET RESTORE_ID PLAN_DIGEST|",
        "drill-preview SNAPSHOT DRILL_PARENT RESULTS_PARENT|",
        "drill-commit SNAPSHOT DRILL_PARENT RESULTS_PARENT DRILL_ID PLAN_DIGEST|",
        "server-preview REQUEST_JSON|server-commit REQUEST_JSON|",
        "server-verify REQUEST_JSON|server-restore-preview REQUEST_JSON|",
        "server-restore-commit REQUEST_JSON|server-restore-verify REQUEST_JSON|",
        "node-restore-preview SNAPSHOT TARGET_WORKSPACE SOURCE_NODE_ID TARGET_PARENT_ID NAME|",
        "node-restore-commit SNAPSHOT TARGET_WORKSPACE SOURCE_NODE_ID TARGET_PARENT_ID NAME RESTORE_ID PLAN_DIGEST|",
        "subtree-restore-preview SNAPSHOT TARGET_WORKSPACE SOURCE_NODE_ID TARGET_PARENT_ID NAME|",
        "subtree-restore-commit SNAPSHOT TARGET_WORKSPACE SOURCE_NODE_ID TARGET_PARENT_ID NAME RESTORE_ID PLAN_DIGEST>"
    )
    .to_owned()
}

fn run_server_pair(
    arguments: &[String],
    schema: &str,
) -> Option<Result<serde_json::Value, String>> {
    match arguments {
        [scope, command, request] if scope == "backup" && command == "server-preview" => {
            Some(preview_server_backup(schema, request))
        }
        [scope, command, request] if scope == "backup" && command == "server-commit" => {
            Some(commit_server_backup(schema, request))
        }
        [scope, command, request] if scope == "backup" && command == "server-verify" => {
            Some(verify_server_backup(schema, request))
        }
        [scope, command, request] if scope == "backup" && command == "server-restore-preview" => {
            Some(preview_server_restore(schema, request))
        }
        [scope, command, request] if scope == "backup" && command == "server-restore-commit" => {
            Some(commit_server_restore(schema, request))
        }
        [scope, command, request] if scope == "backup" && command == "server-restore-verify" => {
            Some(verify_server_restore(schema, request))
        }
        _ => None,
    }
}

fn preview_server_backup(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerBackupPreviewRequest =
        read_server_request(Path::new(request), SERVER_BACKUP_PREVIEW_REQUEST_SCHEMA)?;
    let plan = plan_server_backup_pair(
        &request.workspace_root,
        &request.control_plane_root,
        &request.backup_parent,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_preview",
            "workspaceSnapshotId": plan.workspace_snapshot_id,
            "controlPlaneBackupId": plan.control_plane_backup_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
        },
    }))
}

fn commit_server_backup(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerBackupCommitRequest =
        read_server_request(Path::new(request), SERVER_BACKUP_COMMIT_REQUEST_SCHEMA)?;
    let plan = replan_server_backup_pair(
        &request.workspace_root,
        &request.control_plane_root,
        &request.backup_parent,
        request.workspace_snapshot_id,
        request.control_plane_backup_id,
    )
    .map_err(|error| error.to_string())?;
    require_digest(
        &plan.plan_digest,
        &request.plan_digest,
        "Server backup pair",
    )?;
    let receipt = commit_server_backup_pair(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_committed",
            "workspaceSnapshotId": plan.workspace_snapshot_id,
            "controlPlaneBackupId": plan.control_plane_backup_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn verify_server_backup(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerBackupVerifyRequest =
        read_server_request(Path::new(request), SERVER_BACKUP_VERIFY_REQUEST_SCHEMA)?;
    let verification = verify_server_backup_pair(
        &request.workspace_snapshot_directory,
        &request.control_plane_snapshot_directory,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_verified",
            "verification": verification,
        },
    }))
}

fn preview_server_restore(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerRestorePreviewRequest =
        read_server_request(Path::new(request), SERVER_RESTORE_PREVIEW_REQUEST_SCHEMA)?;
    let plan = plan_server_restore_pair(
        &request.workspace_snapshot_directory,
        &request.control_plane_snapshot_directory,
        &request.restored_workspace_root,
        &request.restored_control_plane_root,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_restore_preview",
            "workspaceRestoreId": plan.workspace_restore_id,
            "controlPlaneRestoreId": plan.control_plane_restore_id,
            "planDigest": plan.plan_digest,
            "reverseProxySecretAction": plan.reverse_proxy_secret_action,
            "plan": plan,
        },
    }))
}

fn commit_server_restore(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerRestoreCommitRequest =
        read_server_request(Path::new(request), SERVER_RESTORE_COMMIT_REQUEST_SCHEMA)?;
    let plan = replan_server_restore_pair(
        &request.workspace_snapshot_directory,
        &request.control_plane_snapshot_directory,
        &request.restored_workspace_root,
        &request.restored_control_plane_root,
        request.workspace_restore_id,
        request.control_plane_restore_id,
    )
    .map_err(|error| error.to_string())?;
    require_digest(
        &plan.plan_digest,
        &request.plan_digest,
        "Server restore pair",
    )?;
    let receipt = commit_server_restore_pair(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_restored",
            "workspaceRestoreId": plan.workspace_restore_id,
            "controlPlaneRestoreId": plan.control_plane_restore_id,
            "planDigest": plan.plan_digest,
            "reverseProxySecretAction": plan.reverse_proxy_secret_action,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn verify_server_restore(schema: &str, request: &str) -> Result<serde_json::Value, String> {
    let request: ServerRestoreVerifyRequest =
        read_server_request(Path::new(request), SERVER_RESTORE_VERIFY_REQUEST_SCHEMA)?;
    let verification = verify_server_restore_pair(
        &request.workspace_snapshot_directory,
        &request.control_plane_snapshot_directory,
        &request.restored_workspace_root,
        &request.restored_control_plane_root,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "server_pair_restore_verified",
            "reverseProxySecretAction": verification.reverse_proxy_secret_action,
            "verification": verification,
        },
    }))
}

fn read_server_request<T: for<'de> Deserialize<'de>>(
    path: &Path,
    expected_schema: &str,
) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect Server request JSON: {error}"))?;
    if !metadata.is_file() || linked_or_reparse(&metadata) {
        return Err("Server request JSON must be an unlinked regular file".to_owned());
    }
    if metadata.len() > MAX_SERVER_REQUEST_BYTES {
        return Err(format!(
            "Server request JSON exceeds {MAX_SERVER_REQUEST_BYTES} bytes"
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .and_then(|file| {
            file.take(MAX_SERVER_REQUEST_BYTES.saturating_add(1))
                .read_to_end(&mut bytes)
        })
        .map_err(|error| format!("cannot read Server request JSON: {error}"))?;
    if bytes.len() as u64 != metadata.len() || bytes.len() as u64 > MAX_SERVER_REQUEST_BYTES {
        return Err("Server request JSON changed while it was read".to_owned());
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid Server request JSON: {error}"))?;
    let actual_schema = value
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Server request JSON schema must be a string".to_owned())?;
    if actual_schema != expected_schema {
        return Err(format!(
            "unsupported Server request schema: expected {expected_schema}"
        ));
    }
    serde_json::from_value(value).map_err(|error| format!("invalid Server request JSON: {error}"))
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

fn run_scoped_restore(
    arguments: &[String],
    schema: &str,
) -> Option<Result<serde_json::Value, String>> {
    match arguments {
        [
            scope,
            command,
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
        ] if scope == "backup"
            && matches!(
                command.as_str(),
                "node-restore-preview" | "subtree-restore-preview"
            ) =>
        {
            Some(preview_scoped_restore(
                schema,
                command,
                snapshot,
                target_workspace,
                source_node_id,
                destination_parent_id,
                destination_name,
            ))
        }
        [
            scope,
            command,
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
            restore_id,
            plan_digest,
        ] if scope == "backup"
            && matches!(
                command.as_str(),
                "node-restore-commit" | "subtree-restore-commit"
            ) =>
        {
            Some(commit_scoped(
                schema,
                command,
                snapshot,
                target_workspace,
                source_node_id,
                destination_parent_id,
                destination_name,
                restore_id,
                plan_digest,
            ))
        }
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn preview_scoped_restore(
    schema: &str,
    command: &str,
    snapshot: &str,
    target_workspace: &str,
    source_node_id: &str,
    destination_parent_id: &str,
    destination_name: &str,
) -> Result<serde_json::Value, String> {
    let source_node_id = parse_node_id(source_node_id, "source node ID")?;
    let destination_parent_id = parse_node_id(destination_parent_id, "destination parent ID")?;
    let plan = if command == "node-restore-preview" {
        plan_single_node_restore(
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
        )
    } else {
        plan_subtree_restore(
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
        )
    }
    .map_err(|error| error.to_string())?;
    let stage = if command == "node-restore-preview" {
        "single_node_restore_preview"
    } else {
        "subtree_restore_preview"
    };
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": stage,
            "restoreId": plan.restore_id,
            "planDigest": plan.plan_digest,
            "commitState": plan.commit_state,
            "blockers": plan.blockers,
            "plan": plan,
        },
    }))
}

#[allow(clippy::too_many_arguments)]
fn commit_scoped(
    schema: &str,
    command: &str,
    snapshot: &str,
    target_workspace: &str,
    source_node_id: &str,
    destination_parent_id: &str,
    destination_name: &str,
    restore_id: &str,
    plan_digest: &str,
) -> Result<serde_json::Value, String> {
    let source_node_id = parse_node_id(source_node_id, "source node ID")?;
    let destination_parent_id = parse_node_id(destination_parent_id, "destination parent ID")?;
    let restore_id = parse_id(restore_id, "scoped restore ID")?;
    let plan = if command == "node-restore-commit" {
        replan_single_node_restore(
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
            restore_id,
        )
    } else {
        replan_subtree_restore(
            snapshot,
            target_workspace,
            source_node_id,
            destination_parent_id,
            destination_name,
            restore_id,
        )
    }
    .map_err(|error| error.to_string())?;
    require_digest(&plan.plan_digest, plan_digest, "scoped restore")?;
    let receipt = commit_scoped_restore(&plan).map_err(|error| error.to_string())?;
    let stage = if command == "node-restore-commit" {
        "single_node_restored"
    } else {
        "subtree_restored"
    };
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": stage,
            "restoreId": plan.restore_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn parse_node_id(value: &str, label: &str) -> Result<NodeId, String> {
    NodeId::from_str(value).map_err(|_| format!("{label} must be a lowercase UUIDv4"))
}

fn protect_backup(schema: &str, snapshot: &str, label: &str) -> Result<serde_json::Value, String> {
    let protection =
        protect_full_workspace_snapshot(snapshot, label).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "protected",
            "snapshot": snapshot,
            "protection": protection,
        },
    }))
}

fn preview_retention(
    schema: &str,
    snapshot_parent: &str,
    keep_latest: &str,
) -> Result<serde_json::Value, String> {
    let policy = parse_retention_policy(keep_latest)?;
    let plan =
        plan_snapshot_retention(snapshot_parent, policy).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "retention_preview",
            "operationId": plan.operation_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
        },
    }))
}

fn commit_retention(
    schema: &str,
    snapshot_parent: &str,
    keep_latest: &str,
    operation_id: &str,
    plan_digest: &str,
) -> Result<serde_json::Value, String> {
    let policy = parse_retention_policy(keep_latest)?;
    let plan = replan_snapshot_retention(
        snapshot_parent,
        policy,
        parse_id(operation_id, "retention operation ID")?,
    )
    .map_err(|error| error.to_string())?;
    require_digest(&plan.plan_digest, plan_digest, "retention")?;
    let receipt = commit_snapshot_retention(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "retention_committed",
            "operationId": plan.operation_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn recover_retention(schema: &str, snapshot_parent: &str) -> Result<serde_json::Value, String> {
    let recovery =
        recover_snapshot_retention(snapshot_parent).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "retention_recovered",
            "recovery": recovery,
        },
    }))
}

fn parse_retention_policy(value: &str) -> Result<SnapshotRetentionPolicy, String> {
    let keep_latest_unprotected = value
        .parse::<u32>()
        .map_err(|_| "retention keep count must be an unsigned 32-bit integer".to_owned())?;
    Ok(SnapshotRetentionPolicy {
        keep_latest_unprotected,
    })
}

fn preview_backup(
    schema: &str,
    root: &str,
    snapshot_parent: &str,
) -> Result<serde_json::Value, String> {
    let plan =
        plan_full_workspace_backup(root, snapshot_parent).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "preview",
            "snapshotId": plan.snapshot_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
        },
    }))
}

fn commit_backup(
    schema: &str,
    root: &str,
    snapshot_parent: &str,
    snapshot_id: &str,
    plan_digest: &str,
) -> Result<serde_json::Value, String> {
    let plan =
        replan_full_workspace_backup(root, snapshot_parent, parse_id(snapshot_id, "snapshot ID")?)
            .map_err(|error| error.to_string())?;
    require_digest(&plan.plan_digest, plan_digest, "backup")?;
    let receipt = commit_full_workspace_backup(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "committed",
            "snapshotId": plan.snapshot_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn verify_backup(schema: &str, snapshot: &str) -> Result<serde_json::Value, String> {
    let verification =
        verify_full_workspace_snapshot(snapshot).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "verified",
            "snapshot": snapshot,
            "verification": verification,
        },
    }))
}

fn preview_restore(
    schema: &str,
    snapshot: &str,
    target: &str,
    stage: &str,
) -> Result<serde_json::Value, String> {
    let plan = plan_alternate_restore(snapshot, target).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": stage,
            "restoreId": plan.restore_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
        },
    }))
}

fn commit_restore(
    schema: &str,
    snapshot: &str,
    target: &str,
    restore_id: &str,
    plan_digest: &str,
) -> Result<serde_json::Value, String> {
    let plan = replan_alternate_restore(snapshot, target, parse_id(restore_id, "restore ID")?)
        .map_err(|error| error.to_string())?;
    require_digest(&plan.plan_digest, plan_digest, "restore")?;
    let receipt = commit_alternate_restore(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "restored",
            "restoreId": plan.restore_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn preview_drill(
    schema: &str,
    snapshot: &str,
    drill_parent: &str,
    results_parent: &str,
) -> Result<serde_json::Value, String> {
    let plan = plan_restore_drill(snapshot, drill_parent, results_parent)
        .map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "drill_preview",
            "drillId": plan.drill_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
        },
    }))
}

fn commit_drill(
    schema: &str,
    snapshot: &str,
    drill_parent: &str,
    results_parent: &str,
    drill_id: &str,
    plan_digest: &str,
) -> Result<serde_json::Value, String> {
    let plan = replan_restore_drill(
        snapshot,
        drill_parent,
        results_parent,
        parse_id(drill_id, "restore drill ID")?,
    )
    .map_err(|error| error.to_string())?;
    require_digest(&plan.plan_digest, plan_digest, "restore drill")?;
    let receipt = commit_restore_drill(&plan).map_err(|error| error.to_string())?;
    Ok(json!({
        "schema": schema,
        "ok": true,
        "backup": {
            "stage": "drill_completed",
            "drillId": plan.drill_id,
            "planDigest": plan.plan_digest,
            "plan": plan,
            "receipt": receipt,
        },
    }))
}

fn parse_id(value: &str, label: &str) -> Result<Uuid, String> {
    Uuid::parse_str(value).map_err(|_| format!("{label} must be a UUID"))
}

fn require_digest(actual: &str, provided: &str, operation: &str) -> Result<(), String> {
    if actual == provided {
        Ok(())
    } else {
        Err(format!(
            "{operation} plan digest does not match the current preview"
        ))
    }
}
