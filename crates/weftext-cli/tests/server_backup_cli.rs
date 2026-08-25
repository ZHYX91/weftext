use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rusqlite::{Connection, params};
use serde_json::{Value, json};
use tempfile::TempDir;
use weftext_backup::{
    SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE, SERVER_CONTROL_PLANE_DATABASE_FILE,
    SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE, harden_server_control_plane_permissions,
};

struct Fixture {
    _temporary: TempDir,
    root: PathBuf,
    workspace: PathBuf,
    control: PathBuf,
    backups: PathBuf,
    restores: PathBuf,
    requests: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = fs::canonicalize(temporary.path()).expect("canonical temporary root");
        let workspace = root.join("source").join("Workspace");
        let control = root.join("server-control");
        let backups = root.join("backups");
        let restores = root.join("restores");
        let requests = root.join("requests");
        fs::create_dir(root.join("source")).unwrap();
        fs::create_dir(&control).unwrap();
        fs::create_dir(&backups).unwrap();
        fs::create_dir(&restores).unwrap();
        fs::create_dir(&requests).unwrap();
        weftext_core::create_workspace(&workspace).unwrap();
        fs::write(workspace.join("portable.bin"), b"portable\0payload\xff").unwrap();
        create_initialized_control_plane(&control);
        harden_server_control_plane_permissions(&control).unwrap();
        Self {
            _temporary: temporary,
            root,
            workspace,
            control,
            backups,
            restores,
            requests,
        }
    }

    #[allow(clippy::needless_pass_by_value)]
    fn request(&self, name: &str, value: Value) -> PathBuf {
        let path = self.requests.join(format!("{name}.json"));
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        path
    }

    fn backup_preview(&self, name: &str) -> (PathBuf, Output, Value) {
        let request = self.request(
            name,
            json!({
                "schema": "weftext.cli.server-backup-preview-request.v1",
                "workspaceRoot": self.workspace,
                "controlPlaneRoot": self.control,
                "backupParent": self.backups,
            }),
        );
        let output = command("server-preview", &request);
        assert_success(&output);
        let value = output_json(&output);
        (request, output, value)
    }

    fn backup_commit_request(&self, name: &str, preview: &Value) -> PathBuf {
        self.request(
            name,
            json!({
                "schema": "weftext.cli.server-backup-commit-request.v1",
                "workspaceRoot": self.workspace,
                "controlPlaneRoot": self.control,
                "backupParent": self.backups,
                "workspaceSnapshotId": preview["backup"]["workspaceSnapshotId"],
                "controlPlaneBackupId": preview["backup"]["controlPlaneBackupId"],
                "planDigest": preview["backup"]["planDigest"],
            }),
        )
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn server_pair_cli_previews_commits_verifies_restores_and_replays() {
    let fixture = Fixture::new();
    let (_, _, preview) = fixture.backup_preview("backup-preview");
    assert_eq!(preview["schema"], "weftext.cli.v1");
    assert_eq!(preview["backup"]["stage"], "server_pair_preview");
    assert_eq!(preview["backup"]["plan"]["commitState"], "pending");
    assert!(preview["backup"]["plan"]["workspacePlanDigest"].is_string());
    assert!(directory_is_empty(&fixture.backups));

    let commit_request = fixture.backup_commit_request("backup-commit", &preview);
    let committed = command("server-commit", &commit_request);
    assert_success(&committed);
    let committed = output_json(&committed);
    assert_eq!(committed["backup"]["stage"], "server_pair_committed");
    assert_eq!(committed["backup"]["receipt"]["complete"], true);
    assert_eq!(
        committed["backup"]["receipt"]["verification"]["exactPair"],
        true
    );
    let workspace_snapshot = value_path(&committed["backup"]["plan"]["workspaceSnapshotDirectory"]);
    let control_snapshot =
        value_path(&committed["backup"]["plan"]["controlPlaneSnapshotDirectory"]);
    assert!(
        !control_snapshot
            .join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE)
            .exists()
    );
    assert!(
        !control_snapshot
            .join(SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE)
            .exists()
    );

    let verify_request = fixture.request(
        "backup-verify",
        json!({
            "schema": "weftext.cli.server-backup-verify-request.v1",
            "workspaceSnapshotDirectory": workspace_snapshot,
            "controlPlaneSnapshotDirectory": control_snapshot,
        }),
    );
    let verified = command("server-verify", &verify_request);
    assert_success(&verified);
    assert_eq!(
        output_json(&verified)["backup"]["stage"],
        "server_pair_verified"
    );

    let replay = command("server-commit", &commit_request);
    assert_success(&replay);
    let replay = output_json(&replay);
    assert_eq!(replay["backup"]["receipt"]["resumedFrom"], "complete");

    let restored_workspace = fixture.restores.join("Workspace");
    let restored_control = fixture.restores.join("alternate-control");
    let restore_preview_request = fixture.request(
        "restore-preview",
        json!({
            "schema": "weftext.cli.server-restore-preview-request.v1",
            "workspaceSnapshotDirectory": workspace_snapshot,
            "controlPlaneSnapshotDirectory": control_snapshot,
            "restoredWorkspaceRoot": restored_workspace,
            "restoredControlPlaneRoot": restored_control,
        }),
    );
    let restore_preview = command("server-restore-preview", &restore_preview_request);
    assert_success(&restore_preview);
    let restore_preview = output_json(&restore_preview);
    assert_eq!(
        restore_preview["backup"]["stage"],
        "server_pair_restore_preview"
    );
    assert_eq!(
        restore_preview["backup"]["reverseProxySecretAction"],
        "regenerate_and_rotate_at_first_server_start"
    );
    assert!(!restored_workspace.exists());
    assert!(!restored_control.exists());

    let restore_commit_request = fixture.request(
        "restore-commit",
        json!({
            "schema": "weftext.cli.server-restore-commit-request.v1",
            "workspaceSnapshotDirectory": workspace_snapshot,
            "controlPlaneSnapshotDirectory": control_snapshot,
            "restoredWorkspaceRoot": restored_workspace,
            "restoredControlPlaneRoot": restored_control,
            "workspaceRestoreId": restore_preview["backup"]["workspaceRestoreId"],
            "controlPlaneRestoreId": restore_preview["backup"]["controlPlaneRestoreId"],
            "planDigest": restore_preview["backup"]["planDigest"],
        }),
    );
    let restored = command("server-restore-commit", &restore_commit_request);
    assert_success(&restored);
    let restored = output_json(&restored);
    assert_eq!(restored["backup"]["stage"], "server_pair_restored");
    assert_eq!(
        restored["backup"]["reverseProxySecretAction"],
        "regenerate_and_rotate_at_first_server_start"
    );
    assert!(
        !restored_control
            .join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE)
            .exists()
    );
    let session_count: i64 =
        Connection::open(restored_control.join(SERVER_CONTROL_PLANE_DATABASE_FILE))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .unwrap();
    assert_eq!(session_count, 0);

    let restore_verify_request = fixture.request(
        "restore-verify",
        json!({
            "schema": "weftext.cli.server-restore-verify-request.v1",
            "workspaceSnapshotDirectory": workspace_snapshot,
            "controlPlaneSnapshotDirectory": control_snapshot,
            "restoredWorkspaceRoot": restored_workspace,
            "restoredControlPlaneRoot": restored_control,
        }),
    );
    let restore_verified = command("server-restore-verify", &restore_verify_request);
    assert_success(&restore_verified);
    assert_eq!(
        output_json(&restore_verified)["backup"]["stage"],
        "server_pair_restore_verified"
    );
    let replay = command("server-restore-commit", &restore_commit_request);
    assert_success(&replay);
    assert_eq!(
        output_json(&replay)["backup"]["receipt"]["resumedFrom"],
        "complete"
    );
}

#[test]
fn server_pair_cli_is_closed_stale_safe_and_resumes_a_partial_pair() {
    let fixture = Fixture::new();
    let missing_request = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["backup", "server-preview"])
        .output()
        .unwrap();
    assert_failure_contains(&missing_request, "server-preview REQUEST_JSON");

    let unknown_request = fixture.request(
        "unknown-field",
        json!({
            "schema": "weftext.cli.server-backup-preview-request.v1",
            "workspaceRoot": fixture.workspace,
            "controlPlaneRoot": fixture.control,
            "backupParent": fixture.backups,
            "futureField": true,
        }),
    );
    assert_failure_contains(
        &command("server-preview", &unknown_request),
        "unknown field",
    );

    let (_, _, preview) = fixture.backup_preview("partial-preview");
    let tampered_request = fixture.request(
        "tampered-digest",
        json!({
            "schema": "weftext.cli.server-backup-commit-request.v1",
            "workspaceRoot": fixture.workspace,
            "controlPlaneRoot": fixture.control,
            "backupParent": fixture.backups,
            "workspaceSnapshotId": preview["backup"]["workspaceSnapshotId"],
            "controlPlaneBackupId": preview["backup"]["controlPlaneBackupId"],
            "planDigest": "0000000000000000000000000000000000000000000000000000000000000000",
        }),
    );
    assert_failure_contains(
        &command("server-commit", &tampered_request),
        "does not match",
    );
    assert!(directory_is_empty(&fixture.backups));

    let snapshot_id = preview["backup"]["workspaceSnapshotId"].as_str().unwrap();
    let workspace_digest = preview["backup"]["plan"]["workspacePlanDigest"]
        .as_str()
        .unwrap();
    let portable = Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["backup", "commit"])
        .arg(&fixture.workspace)
        .arg(&fixture.backups)
        .arg(snapshot_id)
        .arg(workspace_digest)
        .output()
        .unwrap();
    assert_success(&portable);

    let commit_request = fixture.backup_commit_request("partial-commit", &preview);
    let unknown_control_file = fixture.control.join("future-control-state");
    fs::write(&unknown_control_file, b"must fail closed").unwrap();
    assert_failure_contains(
        &command("server-commit", &commit_request),
        "unknown control-plane entries",
    );
    assert!(value_path(&preview["backup"]["plan"]["workspaceSnapshotDirectory"]).exists());
    fs::remove_file(&unknown_control_file).unwrap();
    let resumed = command("server-commit", &commit_request);
    assert_success(&resumed);
    assert_eq!(
        output_json(&resumed)["backup"]["receipt"]["resumedFrom"],
        "workspace_complete_control_plane_pending"
    );
}

#[test]
fn server_pair_cli_rejects_stale_sources_and_existing_restore_collisions() {
    let stale = Fixture::new();
    let (_, _, preview) = stale.backup_preview("stale-preview");
    let commit_request = stale.backup_commit_request("stale-commit", &preview);
    fs::write(
        stale.workspace.join("changed-after-preview.bin"),
        b"changed",
    )
    .unwrap();
    assert_failure_contains(&command("server-commit", &commit_request), "does not match");
    assert!(directory_is_empty(&stale.backups));

    let fixture = Fixture::new();
    let (_, _, preview) = fixture.backup_preview("collision-backup-preview");
    let commit_request = fixture.backup_commit_request("collision-backup-commit", &preview);
    assert_success(&command("server-commit", &commit_request));
    let workspace_snapshot = value_path(&preview["backup"]["plan"]["workspaceSnapshotDirectory"]);
    let control_snapshot = value_path(&preview["backup"]["plan"]["controlPlaneSnapshotDirectory"]);
    let collision_parent = fixture.root.join("collision");
    fs::create_dir(&collision_parent).unwrap();
    let restored_workspace = collision_parent.join("Workspace");
    fs::create_dir(&restored_workspace).unwrap();
    fs::write(
        restored_workspace.join("unrelated.txt"),
        b"do not overwrite",
    )
    .unwrap();
    let restored_control = collision_parent.join("control");
    let request = fixture.request(
        "collision-restore-preview",
        json!({
            "schema": "weftext.cli.server-restore-preview-request.v1",
            "workspaceSnapshotDirectory": workspace_snapshot,
            "controlPlaneSnapshotDirectory": control_snapshot,
            "restoredWorkspaceRoot": restored_workspace,
            "restoredControlPlaneRoot": restored_control,
        }),
    );
    assert_failure_contains(
        &command("server-restore-preview", &request),
        "already exists",
    );
    assert_eq!(
        fs::read(restored_workspace.join("unrelated.txt")).unwrap(),
        b"do not overwrite"
    );
    assert!(!restored_control.exists());
}

fn command(command: &str, request: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_weftext"))
        .args(["backup", command])
        .arg(request)
        .output()
        .expect("run Server backup CLI")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure_contains(output: &Output, expected: &str) {
    assert!(!output.status.success());
    let error: Value = serde_json::from_slice(&output.stderr).expect("JSON failure");
    assert_eq!(error["schema"], "weftext.cli.v1");
    assert_eq!(error["ok"], false);
    assert!(
        error["error"].as_str().unwrap().contains(expected),
        "error was: {}",
        error["error"]
    );
}

fn output_json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("CLI JSON stdout")
}

fn value_path(value: &Value) -> PathBuf {
    PathBuf::from(value.as_str().expect("path string"))
}

fn directory_is_empty(path: &Path) -> bool {
    fs::read_dir(path).unwrap().next().is_none()
}

#[allow(clippy::too_many_lines)]
fn create_initialized_control_plane(root: &Path) {
    let database_path = root.join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let connection = Connection::open(database_path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode=DELETE;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=FULL;
             CREATE TABLE owner (
                 singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                 actor_scope TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE accounts (
                 actor_scope TEXT PRIMARY KEY,
                 login TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'editor', 'commenter', 'viewer')),
                 created_at INTEGER NOT NULL,
                 disabled_at INTEGER
             );
             CREATE TABLE metadata (key TEXT PRIMARY KEY, value BLOB NOT NULL);
             CREATE TABLE sessions (
                 session_id TEXT PRIMARY KEY,
                 actor_scope TEXT NOT NULL,
                 token_digest BLOB NOT NULL UNIQUE,
                 created_at INTEGER NOT NULL,
                 last_seen_at INTEGER NOT NULL,
                 absolute_expires_at INTEGER NOT NULL,
                 idle_expires_at INTEGER NOT NULL,
                 revoked_at INTEGER,
                 end_reason TEXT,
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             CREATE INDEX sessions_expiry
             ON sessions(revoked_at, absolute_expires_at, idle_expires_at);
             CREATE TABLE node_acl (
                 actor_scope TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 access TEXT NOT NULL CHECK(access IN ('hidden', 'read', 'write')),
                 updated_at INTEGER NOT NULL,
                 updated_by TEXT NOT NULL,
                 PRIMARY KEY(actor_scope, node_id),
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             CREATE TABLE authorization_epochs (
                 actor_scope TEXT PRIMARY KEY,
                 epoch INTEGER NOT NULL DEFAULT 0,
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             CREATE TABLE security_events (
                 event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                 occurred_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT,
                 detail TEXT NOT NULL
             );
             CREATE TABLE audit_receipts (
                 receipt_id INTEGER PRIMARY KEY AUTOINCREMENT,
                 occurred_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT NOT NULL,
                 detail TEXT NOT NULL
             );
             CREATE TABLE audit_outbox (
                 intent_id TEXT PRIMARY KEY,
                 created_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 authority_kind TEXT NOT NULL CHECK(authority_kind IN ('document', 'workspace')),
                 target TEXT NOT NULL,
                 expected_revision TEXT NOT NULL
             );
             CREATE TABLE collaboration_documents (
                 node_id TEXT PRIMARY KEY,
                 epoch INTEGER NOT NULL CHECK(epoch >= 1),
                 version INTEGER NOT NULL CHECK(version >= 0),
                 checkpoint_revision TEXT NOT NULL,
                 frozen_reason TEXT,
                 expected_revision TEXT,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE collaboration_receipts (
                 operation_id TEXT PRIMARY KEY,
                 actor_scope TEXT NOT NULL,
                 actor_id TEXT NOT NULL,
                 client_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 epoch INTEGER NOT NULL CHECK(epoch >= 1),
                 base_version INTEGER NOT NULL CHECK(base_version >= 0),
                 base_revision TEXT NOT NULL,
                 applied_base_version INTEGER NOT NULL CHECK(applied_base_version >= 0),
                 applied_base_revision TEXT NOT NULL,
                 result_version INTEGER NOT NULL CHECK(result_version >= 0),
                 result_revision TEXT NOT NULL,
                 request_digest TEXT NOT NULL,
                 transaction_id TEXT NOT NULL UNIQUE,
                 occurred_at INTEGER NOT NULL,
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             CREATE TABLE collaboration_pending (
                 intent_id TEXT PRIMARY KEY,
                 operation_id TEXT NOT NULL UNIQUE,
                 actor_scope TEXT NOT NULL,
                 actor_id TEXT NOT NULL,
                 client_id TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 epoch INTEGER NOT NULL CHECK(epoch >= 1),
                 base_version INTEGER NOT NULL CHECK(base_version >= 0),
                 base_revision TEXT NOT NULL,
                 applied_base_version INTEGER NOT NULL CHECK(applied_base_version >= 0),
                 applied_base_revision TEXT NOT NULL,
                 result_version INTEGER NOT NULL CHECK(result_version >= 0),
                 result_revision TEXT NOT NULL,
                 request_digest TEXT NOT NULL,
                 transaction_id TEXT NOT NULL UNIQUE,
                 FOREIGN KEY(intent_id) REFERENCES audit_outbox(intent_id) ON DELETE CASCADE,
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );",
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO owner(singleton, actor_scope, password_hash, created_at)
             VALUES(1, 'actor-owner', '$argon2id$fixture', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
             VALUES('actor-owner', 'owner', '$argon2id$fixture', 'owner', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO metadata(key, value) VALUES('workspace_scope_v1', ?1)",
            ["0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO sessions(
                 session_id, actor_scope, token_digest, created_at, last_seen_at,
                 absolute_expires_at, idle_expires_at, revoked_at, end_reason
             ) VALUES('session-live', 'actor-owner', ?1, 1, 1, 9999, 9999, NULL, NULL)",
            params![vec![7_u8; 32]],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO authorization_epochs(actor_scope, epoch) VALUES('actor-owner', 1)",
            [],
        )
        .unwrap();
    drop(connection);
    fs::write(
        root.join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE),
        b"runtime-only-secret\n",
    )
    .unwrap();
}
