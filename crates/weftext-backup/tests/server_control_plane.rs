use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, params};
use tempfile::TempDir;
use weftext_backup::{
    SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE, SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE,
    SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE, SERVER_CONTROL_PLANE_DATABASE_FILE,
    SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE, SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE,
    SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE, ServerControlPlaneBackupError,
    acquire_server_control_plane_lease, commit_alternate_restore,
    commit_alternate_server_control_plane_restore, commit_full_workspace_backup,
    commit_server_backup_pair_with_lease, commit_server_control_plane_backup,
    harden_server_control_plane_permissions, plan_alternate_restore,
    plan_alternate_server_control_plane_restore, plan_full_workspace_backup,
    plan_server_backup_pair_with_lease, plan_server_control_plane_backup,
    verify_alternate_server_control_plane_restore, verify_server_control_plane_snapshot,
};
use weftext_core::{create_workspace, scan_workspace};

struct Fixture {
    _temporary: TempDir,
    workspace: PathBuf,
    control: PathBuf,
    backups: PathBuf,
    restores: PathBuf,
    workspace_snapshot: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = fs::canonicalize(temporary.path()).expect("canonical temporary root");
        let workspace = root.join("source").join("Workspace");
        fs::create_dir(root.join("source")).unwrap();
        create_workspace(&workspace).unwrap();
        fs::create_dir(workspace.join("ignored")).unwrap();
        fs::write(
            workspace.join("ignored/private.bin"),
            b"ignored backup bytes\0\xff",
        )
        .unwrap();
        fs::write(
            workspace.join(".weftext-rules"),
            b"weftext-content-rules-v1\nignore ignored/\n",
        )
        .unwrap();
        assert!(scan_workspace(&workspace).is_valid());

        let control = root.join("server-control");
        let backups = root.join("backups");
        let restores = root.join("restores");
        fs::create_dir(&control).unwrap();
        fs::create_dir(&backups).unwrap();
        fs::create_dir(&restores).unwrap();
        create_initialized_control_plane(&control);
        harden_server_control_plane_permissions(&control).unwrap();

        let workspace_plan = plan_full_workspace_backup(&workspace, &backups).unwrap();
        commit_full_workspace_backup(&workspace_plan).unwrap();
        Self {
            _temporary: temporary,
            workspace,
            control,
            backups,
            restores,
            workspace_snapshot: workspace_plan.snapshot_directory,
        }
    }

    fn backup_control_plane(&self) -> PathBuf {
        let plan = plan_server_control_plane_backup(
            &self.control,
            &self.workspace,
            &self.workspace_snapshot,
            &self.backups,
        )
        .unwrap();
        let receipt = commit_server_control_plane_backup(&plan).unwrap();
        assert!(receipt.verified);
        plan.snapshot_directory
    }

    fn restore_workspace(&self) -> PathBuf {
        let destination = self.restores.join("Workspace");
        let plan = plan_alternate_restore(&self.workspace_snapshot, &destination).unwrap();
        commit_alternate_restore(&plan).unwrap();
        destination
    }
}

#[test]
fn server_control_plane_backup_cannot_target_a_managed_workspace() {
    let fixture = Fixture::new();
    let nested_backup_parent = fixture.workspace.join("ignored/Backups");
    fs::create_dir(&nested_backup_parent).unwrap();
    let workspace_plan = plan_full_workspace_backup(&fixture.workspace, &fixture.backups).unwrap();
    commit_full_workspace_backup(&workspace_plan).unwrap();

    let error = plan_server_control_plane_backup(
        &fixture.control,
        &fixture.workspace,
        &workspace_plan.snapshot_directory,
        &nested_backup_parent,
    )
    .expect_err("control-plane backup bytes must stay outside managed workspace authority");

    assert!(
        error
            .to_string()
            .contains("outside every Weftext workspace root"),
        "{error:?}"
    );
    assert_eq!(fs::read_dir(&nested_backup_parent).unwrap().count(), 0);
    assert!(scan_workspace(&fixture.workspace).is_valid());
}

#[test]
#[allow(clippy::too_many_lines)]
fn paired_snapshot_and_clean_alternate_restore_preserve_authority_and_clear_sessions() {
    let fixture = Fixture::new();
    let control_snapshot = fixture.backup_control_plane();
    let verification =
        verify_server_control_plane_snapshot(&control_snapshot, &fixture.workspace_snapshot)
            .unwrap();
    assert!(verification.complete);
    assert!(
        control_snapshot
            .join(SERVER_CONTROL_PLANE_DATABASE_FILE)
            .is_file()
    );
    assert!(
        control_snapshot
            .join(SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE)
            .is_file()
    );
    assert!(
        control_snapshot
            .join(SERVER_CONTROL_PLANE_BACKUP_COMPLETION_FILE)
            .is_file()
    );
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

    let restored_workspace = fixture.restore_workspace();
    let restored_control = fixture.restores.join("alternate-control");
    let plan = plan_alternate_server_control_plane_restore(
        &control_snapshot,
        &fixture.workspace_snapshot,
        &restored_workspace,
        &restored_control,
    )
    .unwrap();
    let receipt = commit_alternate_server_control_plane_restore(&plan).unwrap();
    assert!(receipt.sessions_invalidated);
    assert!(receipt.permissions_verified);
    assert!(!receipt.reverse_proxy_secret_present);
    assert!(!receipt.bootstrap_secret_present);
    assert_eq!(receipt.receipt_sha256.len(), 64);
    assert!(receipt.receipt_length > 0);
    assert!(
        restored_control
            .join(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE)
            .is_file()
    );
    assert!(
        restored_control
            .join(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE)
            .is_file()
    );
    assert!(
        !restored_control
            .join(SERVER_CONTROL_PLANE_REVERSE_PROXY_SECRET_FILE)
            .exists()
    );
    assert!(
        !restored_control
            .join(SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE)
            .exists()
    );

    let database = Connection::open(restored_control.join(SERVER_CONTROL_PLANE_DATABASE_FILE))
        .expect("restored database");
    for (table, expected) in [
        ("owner", 1_i64),
        ("accounts", 2),
        ("node_acl", 1),
        ("security_events", 1),
        ("audit_receipts", 1),
        ("audit_outbox", 1),
        ("collaboration_documents", 1),
        ("collaboration_receipts", 1),
        ("collaboration_pending", 1),
        ("sessions", 0),
    ] {
        let count: i64 = database
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, expected, "unexpected restored count for {table}");
    }
    drop(database);

    assert_eq!(
        verify_alternate_server_control_plane_restore(
            &restored_control,
            &control_snapshot,
            &fixture.workspace_snapshot,
            &restored_workspace,
        )
        .unwrap(),
        receipt,
        "restore verification is idempotent"
    );

    let next_plan = plan_server_control_plane_backup(
        &restored_control,
        &restored_workspace,
        &fixture.workspace_snapshot,
        &fixture.backups,
    )
    .unwrap();
    assert!(
        next_plan
            .excluded_operational_files
            .iter()
            .any(|name| name == SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE)
    );
    assert!(
        next_plan
            .excluded_operational_files
            .iter()
            .any(|name| name == SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE)
    );
}

#[test]
fn live_lease_wal_unknown_bootstrap_and_hardlinks_fail_closed() {
    let fixture = Fixture::new();
    let signal = fixture.restores.join("lease-held");
    let release = fixture.restores.join("lease-release");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "cross_process_lease_helper", "--nocapture"])
        .env("WEFTEXT_TEST_LEASE_ROOT", &fixture.control)
        .env("WEFTEXT_TEST_LEASE_SIGNAL", &signal)
        .env("WEFTEXT_TEST_LEASE_RELEASE", &release)
        .spawn()
        .unwrap();
    wait_for_path(&signal);
    let live_result = plan_server_control_plane_backup(
        &fixture.control,
        &fixture.workspace,
        &fixture.workspace_snapshot,
        &fixture.backups,
    );
    assert!(
        matches!(
            &live_result,
            Err(ServerControlPlaneBackupError::ControlPlaneInUse(_))
        ),
        "unexpected live-lease result: {live_result:?}"
    );
    fs::write(&release, b"release").unwrap();
    assert!(child.wait().unwrap().success());

    let wal = fixture
        .control
        .join(format!("{SERVER_CONTROL_PLANE_DATABASE_FILE}-wal"));
    fs::write(&wal, b"uncheckpointed").unwrap();
    assert!(plan_for_fixture(&fixture).is_err());
    fs::remove_file(&wal).unwrap();

    let unknown = fixture.control.join("future-secret");
    fs::write(&unknown, b"must not be silently omitted").unwrap();
    assert!(plan_for_fixture(&fixture).is_err());
    fs::remove_file(&unknown).unwrap();

    let bootstrap = fixture
        .control
        .join(SERVER_CONTROL_PLANE_BOOTSTRAP_SECRET_FILE);
    fs::write(&bootstrap, b"raw-bootstrap-secret\n").unwrap();
    assert!(matches!(
        plan_for_fixture(&fixture),
        Err(ServerControlPlaneBackupError::UninitializedControlPlane)
    ));
    fs::remove_file(&bootstrap).unwrap();

    let database = fixture.control.join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let alias = fixture.control.join("database-hardlink");
    fs::hard_link(&database, &alias).unwrap();
    assert!(plan_for_fixture(&fixture).is_err());
}

#[test]
fn cross_process_lease_helper() {
    let Some(root) = std::env::var_os("WEFTEXT_TEST_LEASE_ROOT") else {
        return;
    };
    let signal = PathBuf::from(std::env::var_os("WEFTEXT_TEST_LEASE_SIGNAL").unwrap());
    let release = PathBuf::from(std::env::var_os("WEFTEXT_TEST_LEASE_RELEASE").unwrap());
    let _lease = acquire_server_control_plane_lease(PathBuf::from(root)).unwrap();
    fs::write(&signal, b"held").unwrap();
    wait_for_path(&release);
}

#[test]
fn concurrent_database_or_workspace_changes_stale_the_reviewed_pair() {
    let fixture = Fixture::new();
    let plan = plan_for_fixture(&fixture).unwrap();
    let database =
        Connection::open(fixture.control.join(SERVER_CONTROL_PLANE_DATABASE_FILE)).unwrap();
    database
        .execute(
            "INSERT INTO security_events(occurred_at, event_type, actor_scope, detail)
             VALUES(2, 'concurrent', NULL, 'changed after preview')",
            [],
        )
        .unwrap();
    drop(database);
    assert!(matches!(
        commit_server_control_plane_backup(&plan),
        Err(ServerControlPlaneBackupError::StalePreview)
    ));
    assert!(!plan.snapshot_directory.exists());

    let fixture = Fixture::new();
    fs::write(
        fixture.workspace.join("ignored/private.bin"),
        b"changed ignored bytes",
    )
    .unwrap();
    assert!(matches!(
        plan_for_fixture(&fixture),
        Err(ServerControlPlaneBackupError::WorkspaceSnapshotMismatch)
    ));
}

#[test]
fn running_server_pair_borrows_lease_and_copies_live_wal_consistently() {
    let fixture = Fixture::new();
    let database_path = fixture.control.join(SERVER_CONTROL_PLANE_DATABASE_FILE);
    let connection = Connection::open(&database_path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL;
             INSERT INTO security_events(occurred_at, event_type, actor_scope, detail)
             VALUES(3, 'live_before_backup', 'actor-owner', 'must enter consistent copy');",
        )
        .unwrap();
    let _: i64 = connection
        .query_row("SELECT COUNT(*) FROM security_events", [], |row| row.get(0))
        .unwrap();
    assert!(fixture.control.join("control-plane.sqlite3-wal").is_file());
    harden_server_control_plane_permissions(&fixture.control).unwrap();
    let lease = acquire_server_control_plane_lease(&fixture.control).unwrap();

    let plan = plan_server_backup_pair_with_lease(&fixture.workspace, &lease, &fixture.backups)
        .expect("live Server pair preview");
    assert!(
        plan.excluded_operational_files
            .iter()
            .any(|name| name == "control-plane.sqlite3-wal")
    );
    let receipt = commit_server_backup_pair_with_lease(&lease, &plan)
        .expect("consistent live Server pair commit");
    assert!(receipt.complete);
    assert!(receipt.verification.exact_pair);

    let snapshot = Connection::open(
        plan.control_plane_snapshot_directory
            .join(SERVER_CONTROL_PLANE_DATABASE_FILE),
    )
    .unwrap();
    let copied: i64 = snapshot
        .query_row(
            "SELECT COUNT(*) FROM security_events WHERE event_type = 'live_before_backup'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(copied, 1);
}

#[test]
fn tampered_snapshot_receipt_and_non_disjoint_restore_are_rejected() {
    let fixture = Fixture::new();
    let control_snapshot = fixture.backup_control_plane();
    let manifest = control_snapshot.join(SERVER_CONTROL_PLANE_BACKUP_MANIFEST_FILE);
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("futureAuthority".to_owned(), serde_json::json!(true));
    fs::write(&manifest, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    assert!(
        verify_server_control_plane_snapshot(&control_snapshot, &fixture.workspace_snapshot)
            .is_err()
    );

    let fixture = Fixture::new();
    let control_snapshot = fixture.backup_control_plane();
    assert!(
        plan_alternate_server_control_plane_restore(
            &control_snapshot,
            &fixture.workspace_snapshot,
            &fixture.workspace,
            fixture.workspace.join("nested-control"),
        )
        .is_err()
    );

    let restored_workspace = fixture.restore_workspace();
    let restored_control = fixture.restores.join("control");
    let plan = plan_alternate_server_control_plane_restore(
        &control_snapshot,
        &fixture.workspace_snapshot,
        &restored_workspace,
        &restored_control,
    )
    .unwrap();
    commit_alternate_server_control_plane_restore(&plan).unwrap();
    let receipt = restored_control.join(SERVER_CONTROL_PLANE_RESTORE_RECEIPT_FILE);
    let receipt_bytes = fs::read(&receipt).unwrap();
    fs::write(&receipt, b"{}\n").unwrap();
    assert!(
        verify_alternate_server_control_plane_restore(
            &restored_control,
            &control_snapshot,
            &fixture.workspace_snapshot,
            &restored_workspace,
        )
        .is_err()
    );
    fs::write(receipt, receipt_bytes).unwrap();
    fs::write(
        restored_control.join(SERVER_CONTROL_PLANE_RESTORE_COMPLETION_FILE),
        b"{}\n",
    )
    .unwrap();
    assert!(
        verify_alternate_server_control_plane_restore(
            &restored_control,
            &control_snapshot,
            &fixture.workspace_snapshot,
            &restored_workspace,
        )
        .is_err()
    );
}

#[test]
fn missing_or_unknown_collaboration_schema_is_rejected() {
    let fixture = Fixture::new();
    let connection =
        Connection::open(fixture.control.join(SERVER_CONTROL_PLANE_DATABASE_FILE)).unwrap();
    connection
        .execute("DROP TABLE collaboration_documents", [])
        .unwrap();
    drop(connection);
    assert!(plan_for_fixture(&fixture).is_err());

    let fixture = Fixture::new();
    let connection =
        Connection::open(fixture.control.join(SERVER_CONTROL_PLANE_DATABASE_FILE)).unwrap();
    connection
        .execute("CREATE TABLE future_collaboration_state(id TEXT)", [])
        .unwrap();
    drop(connection);
    assert!(plan_for_fixture(&fixture).is_err());
}

fn plan_for_fixture(
    fixture: &Fixture,
) -> Result<weftext_backup::ServerControlPlaneBackupPlan, ServerControlPlaneBackupError> {
    plan_server_control_plane_backup(
        &fixture.control,
        &fixture.workspace,
        &fixture.workspace_snapshot,
        &fixture.backups,
    )
}

fn wait_for_path(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {}",
            path.display()
        );
        thread::sleep(Duration::from_millis(10));
    }
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
             CREATE TABLE metadata (
                 key TEXT PRIMARY KEY,
                 value BLOB NOT NULL
             );
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
             VALUES
             ('actor-owner', 'owner', '$argon2id$fixture', 'owner', 1),
             ('actor-editor', 'editor', '$argon2id$fixture', 'editor', 1)",
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
            "INSERT INTO node_acl(actor_scope, node_id, access, updated_at, updated_by)
             VALUES('actor-editor', '00000000-0000-4000-8000-000000000001', 'read', 1, 'actor-owner')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO authorization_epochs(actor_scope, epoch)
             VALUES('actor-owner', 3), ('actor-editor', 2)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO security_events(occurred_at, event_type, actor_scope, detail)
             VALUES(1, 'login_succeeded', 'actor-owner', 'bounded')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO audit_receipts(occurred_at, event_type, actor_scope, detail)
             VALUES(1, 'document_committed', 'actor-owner', 'receipt')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO audit_outbox(
                 intent_id, created_at, event_type, actor_scope, detail,
                 authority_kind, target, expected_revision
             ) VALUES('intent-1', 1, 'workspace_move', 'actor-owner', 'pending',
                      'workspace', 'node', 'revision')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO collaboration_documents(
                 node_id, epoch, version, checkpoint_revision,
                 frozen_reason, expected_revision, updated_at
             ) VALUES('00000000-0000-4000-8000-000000000001', 1, 4,
                      'checkpoint', NULL, NULL, 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO collaboration_receipts(
                 operation_id, actor_scope, actor_id, client_id, node_id, epoch,
                 base_version, base_revision, applied_base_version, applied_base_revision,
                 result_version, result_revision,
                 request_digest, transaction_id, occurred_at
             ) VALUES('operation-1', 'actor-owner', 'human-1', 'client-1',
                      '00000000-0000-4000-8000-000000000001', 1, 3, 'base', 3,
                      'base', 4,
                      'result', 'request-digest', 'transaction-1', 1)",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO collaboration_pending(
                 intent_id, operation_id, actor_scope, actor_id, client_id, node_id,
                 epoch, base_version, base_revision, applied_base_version,
                 applied_base_revision, result_version, result_revision,
                 request_digest, transaction_id
             ) VALUES('intent-1', 'pending-operation', 'actor-owner', 'human-1',
                      'client-1', '00000000-0000-4000-8000-000000000001', 1, 4,
                      'base', 4, 'base', 5, 'result', 'request-digest-2',
                      'transaction-2')",
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
