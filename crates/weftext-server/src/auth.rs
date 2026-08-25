use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior, params};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const DATABASE_FILE: &str = "control-plane.sqlite3";
const DATABASE_SIDECAR_SUFFIXES: [&str; 3] = ["-wal", "-shm", "-journal"];
const BOOTSTRAP_SECRET_FILE: &str = "bootstrap-secret";
const REVERSE_PROXY_SECRET_FILE: &str = "reverse-proxy-secret";
const WORKSPACE_SCOPE_KEY: &str = "workspace_scope_v1";
#[cfg(test)]
const OWNER_LOGIN: &str = "owner";
const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_ITERATIONS: u32 = 2;
const ARGON2_PARALLELISM: u32 = 1;
const SECURITY_EVENT_LIMIT: i64 = 1_000;

#[derive(Clone, Copy, Debug)]
pub struct SessionPolicy {
    pub absolute_seconds: i64,
    pub idle_seconds: i64,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            absolute_seconds: 12 * 60 * 60,
            idle_seconds: 30 * 60,
        }
    }
}

#[derive(Clone)]
pub struct ControlPlane {
    connection: Arc<Mutex<Connection>>,
    root: Arc<PathBuf>,
    database_path: Arc<PathBuf>,
    bootstrap_secret_path: Arc<PathBuf>,
    reverse_proxy_secret_path: Arc<PathBuf>,
    dummy_password_hash: Arc<str>,
    session_policy: SessionPolicy,
}

#[derive(Debug)]
pub(crate) struct PreparedControlPlane {
    root: PathBuf,
    database_path: PathBuf,
    bootstrap_secret_path: PathBuf,
    reverse_proxy_secret_path: PathBuf,
}

impl PreparedControlPlane {
    #[must_use]
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionPrincipal {
    pub actor_scope: String,
    pub role: SessionRole,
    pub session_id: String,
    pub absolute_expires_at: i64,
    pub idle_expires_at: i64,
}

/// Hosted-workspace authorization role bound to a validated Server session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionRole {
    Owner,
    Admin,
    Editor,
    Commenter,
    Viewer,
}

impl SessionRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Admin => "admin",
            Self::Editor => "editor",
            Self::Commenter => "commenter",
            Self::Viewer => "viewer",
        }
    }

    #[must_use]
    pub const fn can_write_annotations(self) -> bool {
        matches!(
            self,
            Self::Owner | Self::Admin | Self::Editor | Self::Commenter
        )
    }

    #[must_use]
    pub const fn can_write_content(self) -> bool {
        matches!(self, Self::Owner | Self::Admin | Self::Editor)
    }

    #[must_use]
    pub const fn can_mutate_structure(self) -> bool {
        matches!(self, Self::Owner | Self::Admin | Self::Editor)
    }

    #[must_use]
    pub const fn can_permanently_delete(self) -> bool {
        matches!(self, Self::Owner)
    }

    #[must_use]
    pub const fn can_manage_members(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    #[must_use]
    pub const fn can_manage_workspace(self) -> bool {
        matches!(self, Self::Owner)
    }

    /// Parses the closed persisted/API role spelling.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidRole`] for every unsupported spelling.
    pub fn parse(value: &str) -> Result<Self, AuthError> {
        match value {
            "owner" => Ok(Self::Owner),
            "admin" => Ok(Self::Admin),
            "editor" => Ok(Self::Editor),
            "commenter" => Ok(Self::Commenter),
            "viewer" => Ok(Self::Viewer),
            _ => Err(AuthError::InvalidRole),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberRecord {
    pub actor_scope: String,
    pub login: String,
    pub role: SessionRole,
    pub enabled: bool,
    pub created_at: i64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeAccess {
    Hidden,
    Read,
    Write,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeAclRecord {
    pub actor_scope: String,
    pub node_id: String,
    pub access: NodeAccess,
    pub updated_at: i64,
    pub updated_by: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditReceipt {
    pub receipt_id: i64,
    pub occurred_at: i64,
    pub event_type: String,
    pub actor_scope: String,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingAuditIntent {
    pub intent_id: String,
    pub created_at: i64,
    pub event_type: String,
    pub actor_scope: String,
    pub detail: String,
    pub authority_kind: String,
    pub target: String,
    pub expected_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollaborationDocumentRecord {
    pub node_id: String,
    pub epoch: u64,
    pub version: u64,
    pub checkpoint_revision: String,
    pub frozen_reason: Option<String>,
    pub expected_revision: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CollaborationReceipt {
    pub operation_id: String,
    pub actor_scope: String,
    pub actor_id: String,
    pub client_id: String,
    pub node_id: String,
    pub epoch: u64,
    pub base_version: u64,
    pub base_revision: String,
    pub applied_base_version: u64,
    pub applied_base_revision: String,
    pub result_version: u64,
    pub result_revision: String,
    pub request_digest: String,
    pub transaction_id: String,
}

pub(crate) struct NewCollaborationIntent<'a> {
    pub actor_id: &'a str,
    pub client_id: &'a str,
    pub operation_id: &'a str,
    pub node_id: &'a str,
    pub epoch: u64,
    pub base_version: u64,
    pub base_revision: &'a str,
    pub applied_base_version: u64,
    pub applied_base_revision: &'a str,
    pub result_version: u64,
    pub result_revision: &'a str,
    pub request_digest: &'a str,
    pub transaction_id: &'a str,
    pub detail: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingCollaborationIntent {
    intent_id: String,
    operation_id: String,
    actor_scope: String,
    actor_id: String,
    client_id: String,
    node_id: String,
    epoch: u64,
    base_version: u64,
    base_revision: String,
    applied_base_version: u64,
    applied_base_revision: String,
    result_version: u64,
    result_revision: String,
    request_digest: String,
    transaction_id: String,
}

impl NodeAccess {
    /// Parses the closed persisted/API access spelling.
    ///
    /// # Errors
    ///
    /// Returns [`AuthError::InvalidAccess`] for every unsupported spelling.
    pub fn parse(value: &str) -> Result<Self, AuthError> {
        match value {
            "hidden" => Ok(Self::Hidden),
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            _ => Err(AuthError::InvalidAccess),
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hidden => "hidden",
            Self::Read => "read",
            Self::Write => "write",
        }
    }
}

#[derive(Debug)]
pub struct IssuedSession {
    pub token: String,
    pub principal: SessionPrincipal,
}

#[derive(Debug)]
pub enum AuthError {
    Io(std::io::Error),
    Database(rusqlite::Error),
    Random(getrandom::Error),
    Password,
    InvalidControlPlane,
    BootstrapUnavailable,
    BootstrapFailed,
    InvalidCredentials,
    InvalidSession,
    ExpiredSession,
    MemberExists,
    MemberUnavailable,
    InvalidLogin,
    InvalidRole,
    InvalidAccess,
    AuthorizationDenied,
    LastOwner,
    Poisoned,
}

impl fmt::Display for AuthError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) | Self::Database(_) | Self::Random(_) | Self::Password | Self::Poisoned => {
                formatter.write_str("Server identity control plane is unavailable")
            }
            Self::InvalidControlPlane => formatter
                .write_str("control-plane directory must be disjoint from the hosted workspace"),
            Self::BootstrapUnavailable => {
                formatter.write_str("Owner bootstrap is no longer available")
            }
            Self::BootstrapFailed => formatter.write_str("Owner bootstrap failed"),
            Self::InvalidCredentials => formatter.write_str("authentication failed"),
            Self::InvalidSession | Self::ExpiredSession => {
                formatter.write_str("authentication required")
            }
            Self::MemberExists => formatter.write_str("member login already exists"),
            Self::MemberUnavailable => formatter.write_str("member is unavailable"),
            Self::InvalidLogin => formatter.write_str("member login is invalid"),
            Self::InvalidRole => formatter.write_str("member role is invalid"),
            Self::InvalidAccess => formatter.write_str("node access is invalid"),
            Self::AuthorizationDenied => formatter.write_str("authorization denied"),
            Self::LastOwner => formatter.write_str("the last enabled Owner cannot be changed"),
        }
    }
}

impl std::error::Error for AuthError {}

impl From<std::io::Error> for AuthError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for AuthError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Database(error)
    }
}

impl From<getrandom::Error> for AuthError {
    fn from(error: getrandom::Error) -> Self {
        Self::Random(error)
    }
}

impl ControlPlane {
    #[cfg(test)]
    pub fn open(
        workspace_root: &Path,
        requested_root: &Path,
        session_policy: SessionPolicy,
    ) -> Result<Self, AuthError> {
        Self::open_prepared(
            Self::prepare(workspace_root, requested_root)?,
            session_policy,
        )
    }

    pub(crate) fn prepare(
        workspace_root: &Path,
        requested_root: &Path,
    ) -> Result<PreparedControlPlane, AuthError> {
        for ancestor in requested_root.ancestors().filter(|path| path.exists()) {
            if path_is_link_or_reparse(ancestor)? {
                return Err(AuthError::InvalidControlPlane);
            }
        }
        fs::create_dir_all(requested_root)?;
        let root = fs::canonicalize(requested_root)?;
        if root == workspace_root
            || root.starts_with(workspace_root)
            || workspace_root.starts_with(&root)
        {
            return Err(AuthError::InvalidControlPlane);
        }
        let database_path = root.join(DATABASE_FILE);
        let bootstrap_secret_path = root.join(BOOTSTRAP_SECRET_FILE);
        let reverse_proxy_secret_path = root.join(REVERSE_PROXY_SECRET_FILE);
        prepare_control_plane_paths(
            &root,
            &database_path,
            &bootstrap_secret_path,
            &reverse_proxy_secret_path,
        )?;
        Ok(PreparedControlPlane {
            root,
            database_path,
            bootstrap_secret_path,
            reverse_proxy_secret_path,
        })
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn open_prepared(
        prepared: PreparedControlPlane,
        session_policy: SessionPolicy,
    ) -> Result<Self, AuthError> {
        let PreparedControlPlane {
            root,
            database_path,
            bootstrap_secret_path,
            reverse_proxy_secret_path,
        } = prepared;
        prepare_control_plane_paths(
            &root,
            &database_path,
            &bootstrap_secret_path,
            &reverse_proxy_secret_path,
        )?;
        let mut connection = Connection::open(&database_path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=FULL;
             PRAGMA secure_delete=ON;
             CREATE TABLE IF NOT EXISTS owner (
                 singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                 actor_scope TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 created_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS accounts (
                 actor_scope TEXT PRIMARY KEY,
                 login TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'editor', 'commenter', 'viewer')),
                 created_at INTEGER NOT NULL,
                 disabled_at INTEGER
             );
             INSERT OR IGNORE INTO accounts(actor_scope, login, password_hash, role, created_at)
             SELECT actor_scope, 'owner', password_hash, 'owner', created_at FROM owner;
             CREATE TABLE IF NOT EXISTS metadata (
                 key TEXT PRIMARY KEY,
                 value BLOB NOT NULL
             );
             CREATE TABLE IF NOT EXISTS sessions (
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
             CREATE INDEX IF NOT EXISTS sessions_expiry
             ON sessions(revoked_at, absolute_expires_at, idle_expires_at);
             CREATE TABLE IF NOT EXISTS node_acl (
                 actor_scope TEXT NOT NULL,
                 node_id TEXT NOT NULL,
                 access TEXT NOT NULL CHECK(access IN ('hidden', 'read', 'write')),
                 updated_at INTEGER NOT NULL,
                 updated_by TEXT NOT NULL,
                 PRIMARY KEY(actor_scope, node_id),
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             CREATE TABLE IF NOT EXISTS authorization_epochs (
                 actor_scope TEXT PRIMARY KEY,
                 epoch INTEGER NOT NULL DEFAULT 0,
                 FOREIGN KEY(actor_scope) REFERENCES accounts(actor_scope)
             );
             INSERT OR IGNORE INTO authorization_epochs(actor_scope, epoch)
             SELECT actor_scope, 0 FROM accounts;
             CREATE TABLE IF NOT EXISTS security_events (
                 event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                 occurred_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT,
                 detail TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS audit_receipts (
                 receipt_id INTEGER PRIMARY KEY AUTOINCREMENT,
                 occurred_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT NOT NULL,
                 detail TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS audit_outbox (
                 intent_id TEXT PRIMARY KEY,
                 created_at INTEGER NOT NULL,
                 event_type TEXT NOT NULL,
                 actor_scope TEXT NOT NULL,
                 detail TEXT NOT NULL,
                 authority_kind TEXT NOT NULL CHECK(authority_kind IN ('document', 'workspace')),
                 target TEXT NOT NULL,
                 expected_revision TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS collaboration_documents (
                 node_id TEXT PRIMARY KEY,
                 epoch INTEGER NOT NULL CHECK(epoch >= 1),
                 version INTEGER NOT NULL CHECK(version >= 0),
                 checkpoint_revision TEXT NOT NULL,
                 frozen_reason TEXT,
                 expected_revision TEXT,
                 updated_at INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS collaboration_receipts (
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
             CREATE TABLE IF NOT EXISTS collaboration_pending (
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
        )?;
        migrate_account_roles(&mut connection)?;
        migrate_session_foreign_key(&mut connection)?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS sessions_expiry
             ON sessions(revoked_at, absolute_expires_at, idle_expires_at)",
            [],
        )?;
        ensure_workspace_scope(&mut connection)?;
        ensure_bootstrap_secret(&mut connection, &bootstrap_secret_path)?;
        secure_control_plane_paths(
            &root,
            &database_path,
            &bootstrap_secret_path,
            &reverse_proxy_secret_path,
        )?;
        let dummy_password_hash = hash_password("dummy-password-that-is-never-valid")?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            root: Arc::new(root),
            database_path: Arc::new(database_path),
            bootstrap_secret_path: Arc::new(bootstrap_secret_path),
            reverse_proxy_secret_path: Arc::new(reverse_proxy_secret_path),
            dummy_password_hash: Arc::from(dummy_password_hash),
            session_policy,
        })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    #[must_use]
    pub fn bootstrap_secret_path(&self) -> &Path {
        &self.bootstrap_secret_path
    }

    pub(crate) fn reverse_proxy_secret_path(&self) -> &Path {
        &self.reverse_proxy_secret_path
    }

    pub(crate) fn provision_reverse_proxy_secret(&self) -> Result<[u8; 32], AuthError> {
        let path = self.reverse_proxy_secret_path();
        if path.exists() {
            if path_is_link_or_reparse(path)? || file_has_multiple_links(path)? {
                return Err(AuthError::InvalidControlPlane);
            }
        } else {
            write_new_secret(path, &random_hex(32)?)?;
        }
        secure_secret_file(path)?;
        let metadata = fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(AuthError::InvalidControlPlane);
        }
        let mut secret = String::new();
        OpenOptions::new()
            .read(true)
            .open(path)?
            .read_to_string(&mut secret)?;
        let secret = secret
            .strip_suffix("\r\n")
            .or_else(|| secret.strip_suffix('\n'))
            .unwrap_or(&secret);
        reverse_proxy_token_digest(secret).ok_or(AuthError::InvalidControlPlane)
    }

    pub(crate) fn readiness_check(&self) -> Result<bool, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let database_status =
            connection.query_row("PRAGMA quick_check(1)", [], |row| row.get::<_, String>(0))?;
        let workspace_scope_count = connection.query_row(
            "SELECT COUNT(*) FROM metadata WHERE key = ?1",
            params![WORKSPACE_SCOPE_KEY],
            |row| row.get::<_, i64>(0),
        )?;
        let pending_audit_count =
            connection.query_row("SELECT COUNT(*) FROM audit_outbox", [], |row| {
                row.get::<_, i64>(0)
            })?;
        Ok(database_status == "ok" && workspace_scope_count == 1 && pending_audit_count == 0)
    }

    #[must_use]
    pub fn session_policy(&self) -> SessionPolicy {
        self.session_policy
    }

    pub(crate) fn workspace_scope(&self) -> Result<String, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection
            .query_row(
                "SELECT value FROM metadata WHERE key = ?1",
                params![WORKSPACE_SCOPE_KEY],
                |row| row.get(0),
            )
            .map_err(AuthError::Database)
    }

    pub(crate) fn authorization_epoch(&self, actor_scope: &str) -> Result<i64, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection
            .query_row(
                "SELECT epoch FROM authorization_epochs WHERE actor_scope = ?1",
                params![actor_scope],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(AuthError::MemberUnavailable)
    }

    pub fn bootstrap(
        &self,
        supplied_secret: &str,
        password: &str,
        prior_token: Option<&str>,
        now: i64,
    ) -> Result<IssuedSession, AuthError> {
        validate_password(password)?;
        let supplied_digest = bootstrap_digest(supplied_secret);
        {
            let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
            if owner_exists(&connection)? {
                return Err(AuthError::BootstrapUnavailable);
            }
            let stored: Option<Vec<u8>> = connection
                .query_row(
                    "SELECT value FROM metadata WHERE key = 'bootstrap_secret_digest'",
                    [],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(stored) = stored else {
                return Err(AuthError::BootstrapFailed);
            };
            if !constant_time_equal(&stored, &supplied_digest) {
                return Err(AuthError::BootstrapFailed);
            }
        }

        let password_hash = hash_password(password)?;
        let actor_scope = random_hex(32)?;
        let material = SessionMaterial::new(now, self.session_policy)?;
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if owner_exists(&transaction)? {
            return Err(AuthError::BootstrapUnavailable);
        }
        let stored: Vec<u8> = transaction.query_row(
            "SELECT value FROM metadata WHERE key = 'bootstrap_secret_digest'",
            [],
            |row| row.get(0),
        )?;
        if !constant_time_equal(&stored, &supplied_digest) {
            return Err(AuthError::BootstrapFailed);
        }
        transaction.execute(
            "INSERT INTO owner(singleton, actor_scope, password_hash, created_at)
             VALUES(1, ?1, ?2, ?3)",
            params![actor_scope, password_hash, now],
        )?;
        transaction.execute(
            "INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
             VALUES(?1, 'owner', ?2, 'owner', ?3)",
            params![actor_scope, password_hash, now],
        )?;
        transaction.execute(
            "INSERT INTO authorization_epochs(actor_scope, epoch) VALUES(?1, 0)",
            params![actor_scope],
        )?;
        revoke_prior_token(&transaction, prior_token, now, "login_rotated")?;
        insert_session(&transaction, &actor_scope, &material)?;
        transaction.execute(
            "DELETE FROM metadata WHERE key = 'bootstrap_secret_digest'",
            [],
        )?;
        insert_event(
            &transaction,
            now,
            "bootstrap_succeeded",
            Some(&actor_scope),
            "success",
        )?;
        transaction.commit()?;
        let _ = fs::remove_file(self.bootstrap_secret_path());
        Ok(material.issued(actor_scope, SessionRole::Owner))
    }

    pub fn login(
        &self,
        login: &str,
        password: &str,
        prior_token: Option<&str>,
        now: i64,
    ) -> Result<IssuedSession, AuthError> {
        let account = {
            let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
            connection
                .query_row(
                    "SELECT actor_scope, password_hash, role FROM accounts
                     WHERE login = ?1 AND disabled_at IS NULL",
                    params![login],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                        ))
                    },
                )
                .optional()?
        };
        let candidate_hash = account
            .as_ref()
            .map_or(self.dummy_password_hash.as_ref(), |value| value.1.as_str());
        let verified = verify_password(candidate_hash, password);
        let Some((actor_scope, _, role)) = account.filter(|_| verified) else {
            let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
            insert_event(
                &connection,
                now,
                "login_failed",
                None,
                "authentication_failed",
            )?;
            return Err(AuthError::InvalidCredentials);
        };

        let material = SessionMaterial::new(now, self.session_policy)?;
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        revoke_prior_token(&transaction, prior_token, now, "login_rotated")?;
        insert_session(&transaction, &actor_scope, &material)?;
        insert_event(
            &transaction,
            now,
            "login_succeeded",
            Some(&actor_scope),
            "success",
        )?;
        transaction.commit()?;
        Ok(material.issued(actor_scope, SessionRole::parse(&role)?))
    }

    pub fn validate_session(&self, token: &str, now: i64) -> Result<SessionPrincipal, AuthError> {
        let token_digest = session_digest(token)?;
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let session = transaction
            .query_row(
                "SELECT sessions.session_id, sessions.actor_scope,
                        sessions.absolute_expires_at, sessions.idle_expires_at,
                        accounts.role, sessions.revoked_at, accounts.disabled_at
                 FROM sessions JOIN accounts USING(actor_scope)
                 WHERE sessions.token_digest = ?1",
                params![token_digest],
                session_row,
            )
            .optional()?;
        let Some(session) = session else {
            expire_inactive_sessions(&transaction, now)?;
            transaction.commit()?;
            return Err(AuthError::InvalidSession);
        };
        if session.revoked_at.is_some() || session.disabled_at.is_some() {
            expire_inactive_sessions(&transaction, now)?;
            transaction.commit()?;
            return Err(AuthError::InvalidSession);
        }
        if now >= session.absolute_expires_at || now >= session.idle_expires_at {
            transaction.execute(
                "UPDATE sessions SET revoked_at = ?1, end_reason = 'expired'
                 WHERE session_id = ?2 AND revoked_at IS NULL",
                params![now, session.session_id],
            )?;
            insert_event(
                &transaction,
                now,
                "session_expired",
                Some(&session.actor_scope),
                "expired",
            )?;
            expire_inactive_sessions(&transaction, now)?;
            transaction.commit()?;
            return Err(AuthError::ExpiredSession);
        }
        expire_inactive_sessions(&transaction, now)?;
        let idle_expires_at =
            (now + self.session_policy.idle_seconds).min(session.absolute_expires_at);
        transaction.execute(
            "UPDATE sessions SET last_seen_at = ?1, idle_expires_at = ?2 WHERE session_id = ?3",
            params![now, idle_expires_at, session.session_id],
        )?;
        transaction.commit()?;
        Ok(SessionPrincipal {
            actor_scope: session.actor_scope,
            role: session.role,
            session_id: session.session_id,
            absolute_expires_at: session.absolute_expires_at,
            idle_expires_at,
        })
    }

    /// Validates one session without extending it or changing any control-plane byte.
    ///
    /// This narrow path exists for backup/restore administration: an exact preview must not be
    /// invalidated by authenticating its subsequent commit request. It never performs expiry
    /// cleanup and therefore cannot replace ordinary sliding-session validation.
    pub(crate) fn validate_session_without_refresh(
        &self,
        token: &str,
        now: i64,
    ) -> Result<SessionPrincipal, AuthError> {
        let token_digest = session_digest(token)?;
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let session = connection
            .query_row(
                "SELECT sessions.session_id, sessions.actor_scope,
                        sessions.absolute_expires_at, sessions.idle_expires_at,
                        accounts.role, sessions.revoked_at, accounts.disabled_at
                 FROM sessions JOIN accounts USING(actor_scope)
                 WHERE sessions.token_digest = ?1",
                params![token_digest],
                session_row,
            )
            .optional()?;
        let Some(session) = session else {
            return Err(AuthError::InvalidSession);
        };
        if session.revoked_at.is_some()
            || session.disabled_at.is_some()
            || now >= session.absolute_expires_at
            || now >= session.idle_expires_at
        {
            return Err(AuthError::ExpiredSession);
        }
        Ok(SessionPrincipal {
            actor_scope: session.actor_scope,
            role: session.role,
            session_id: session.session_id,
            absolute_expires_at: session.absolute_expires_at,
            idle_expires_at: session.idle_expires_at,
        })
    }

    pub(crate) fn session_is_current(
        &self,
        principal: &SessionPrincipal,
        now: i64,
    ) -> Result<bool, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let role = connection
            .query_row(
                "SELECT accounts.role FROM sessions JOIN accounts USING(actor_scope)
                 WHERE sessions.session_id = ?1 AND sessions.actor_scope = ?2
                   AND sessions.revoked_at IS NULL AND accounts.disabled_at IS NULL
                   AND sessions.absolute_expires_at > ?3 AND sessions.idle_expires_at > ?3",
                params![principal.session_id, principal.actor_scope, now],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(role
            .as_deref()
            .and_then(|value| SessionRole::parse(value).ok())
            == Some(principal.role))
    }

    pub fn logout(&self, principal: &SessionPrincipal, now: i64) -> Result<(), AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection.execute(
            "UPDATE sessions SET revoked_at = ?1, end_reason = 'logout'
             WHERE session_id = ?2 AND actor_scope = ?3 AND revoked_at IS NULL",
            params![now, principal.session_id, principal.actor_scope],
        )?;
        insert_event(
            &connection,
            now,
            "logout",
            Some(&principal.actor_scope),
            "success",
        )
    }

    pub fn revoke_all(&self, principal: &SessionPrincipal, now: i64) -> Result<(), AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "UPDATE sessions SET revoked_at = ?1, end_reason = 'owner_revoked'
             WHERE actor_scope = ?2 AND revoked_at IS NULL",
            params![now, principal.actor_scope],
        )?;
        insert_event(
            &transaction,
            now,
            "sessions_revoked",
            Some(&principal.actor_scope),
            "all",
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn list_members(&self) -> Result<Vec<MemberRecord>, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let mut statement = connection.prepare(
            "SELECT actor_scope, login, role, disabled_at IS NULL, created_at
             FROM accounts ORDER BY login",
        )?;
        let rows = statement.query_map([], |row| {
            let role = SessionRole::parse(&row.get::<_, String>(2)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok(MemberRecord {
                actor_scope: row.get(0)?,
                login: row.get(1)?,
                role,
                enabled: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuthError::Database)
    }

    pub(crate) fn create_member(
        &self,
        principal: &SessionPrincipal,
        login: &str,
        password: &str,
        role: SessionRole,
        now: i64,
    ) -> Result<MemberRecord, AuthError> {
        if !principal.role.can_manage_members()
            || (principal.role != SessionRole::Owner && role == SessionRole::Owner)
        {
            return Err(AuthError::AuthorizationDenied);
        }
        validate_login(login)?;
        validate_password(password)?;
        let password_hash = hash_password(password)?;
        let actor_scope = random_hex(32)?;
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let inserted = transaction.execute(
            "INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![actor_scope, login, password_hash, role.as_str(), now],
        );
        match inserted {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(AuthError::MemberExists);
            }
            Err(error) => return Err(AuthError::Database(error)),
        }
        transaction.execute(
            "INSERT INTO authorization_epochs(actor_scope, epoch) VALUES(?1, 0)",
            params![actor_scope],
        )?;
        let detail = format!("actor={actor_scope};role={}", role.as_str());
        insert_event(
            &transaction,
            now,
            "member_created",
            Some(&principal.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            "member_created",
            &principal.actor_scope,
            &detail,
        )?;
        transaction.commit()?;
        Ok(MemberRecord {
            actor_scope,
            login: login.to_owned(),
            role,
            enabled: true,
            created_at: now,
        })
    }

    pub(crate) fn update_member(
        &self,
        principal: &SessionPrincipal,
        actor_scope: &str,
        role: SessionRole,
        enabled: bool,
        now: i64,
    ) -> Result<MemberRecord, AuthError> {
        if !principal.role.can_manage_members() {
            return Err(AuthError::AuthorizationDenied);
        }
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = transaction
            .query_row(
                "SELECT login, role, disabled_at IS NULL, created_at FROM accounts
                 WHERE actor_scope = ?1",
                params![actor_scope],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, bool>(2)?,
                        row.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or(AuthError::MemberUnavailable)?;
        let current_role = SessionRole::parse(&current.1)?;
        if principal.role != SessionRole::Owner
            && (current_role == SessionRole::Owner || role == SessionRole::Owner)
        {
            return Err(AuthError::AuthorizationDenied);
        }
        if current_role == SessionRole::Owner
            && (role != SessionRole::Owner || !enabled)
            && enabled_owner_count(&transaction)? <= 1
        {
            return Err(AuthError::LastOwner);
        }
        transaction.execute(
            "UPDATE accounts SET role = ?1, disabled_at = ?2 WHERE actor_scope = ?3",
            params![role.as_str(), (!enabled).then_some(now), actor_scope],
        )?;
        transaction.execute(
            "UPDATE authorization_epochs SET epoch = epoch + 1 WHERE actor_scope = ?1",
            params![actor_scope],
        )?;
        if !enabled || role != current_role {
            transaction.execute(
                "UPDATE sessions SET revoked_at = ?1, end_reason = 'member_changed'
                 WHERE actor_scope = ?2 AND revoked_at IS NULL",
                params![now, actor_scope],
            )?;
        }
        let detail = format!(
            "actor={actor_scope};role={};enabled={enabled}",
            role.as_str()
        );
        insert_event(
            &transaction,
            now,
            "member_updated",
            Some(&principal.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            "member_updated",
            &principal.actor_scope,
            &detail,
        )?;
        transaction.commit()?;
        Ok(MemberRecord {
            actor_scope: actor_scope.to_owned(),
            login: current.0,
            role,
            enabled,
            created_at: current.3,
        })
    }

    pub(crate) fn set_node_acl(
        &self,
        principal: &SessionPrincipal,
        actor_scope: &str,
        node_id: &str,
        access: Option<NodeAccess>,
        now: i64,
    ) -> Result<(), AuthError> {
        if !principal.role.can_manage_members() {
            return Err(AuthError::AuthorizationDenied);
        }
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM accounts WHERE actor_scope = ?1)",
            params![actor_scope],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(AuthError::MemberUnavailable);
        }
        if let Some(access) = access {
            transaction.execute(
                "INSERT INTO node_acl(actor_scope, node_id, access, updated_at, updated_by)
                 VALUES(?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(actor_scope, node_id) DO UPDATE SET
                   access=excluded.access, updated_at=excluded.updated_at,
                   updated_by=excluded.updated_by",
                params![
                    actor_scope,
                    node_id,
                    access.as_str(),
                    now,
                    principal.actor_scope
                ],
            )?;
        } else {
            transaction.execute(
                "DELETE FROM node_acl WHERE actor_scope = ?1 AND node_id = ?2",
                params![actor_scope, node_id],
            )?;
        }
        transaction.execute(
            "UPDATE authorization_epochs SET epoch = epoch + 1 WHERE actor_scope = ?1",
            params![actor_scope],
        )?;
        let detail = format!(
            "actor={actor_scope};node={node_id};access={}",
            access.map_or("inherit", NodeAccess::as_str)
        );
        insert_event(
            &transaction,
            now,
            "node_acl_updated",
            Some(&principal.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            "node_acl_updated",
            &principal.actor_scope,
            &detail,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn list_node_acl(&self) -> Result<Vec<NodeAclRecord>, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let mut statement = connection.prepare(
            "SELECT actor_scope, node_id, access, updated_at, updated_by
             FROM node_acl ORDER BY actor_scope, node_id",
        )?;
        let rows = statement.query_map([], |row| {
            let access = NodeAccess::parse(&row.get::<_, String>(2)?)
                .map_err(|_| rusqlite::Error::InvalidQuery)?;
            Ok(NodeAclRecord {
                actor_scope: row.get(0)?,
                node_id: row.get(1)?,
                access,
                updated_at: row.get(3)?,
                updated_by: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuthError::Database)
    }

    pub(crate) fn effective_node_access(
        &self,
        principal: &SessionPrincipal,
        node_ids_nearest_first: &[String],
        allow_role_default: bool,
    ) -> Result<NodeAccess, AuthError> {
        if principal.role == SessionRole::Owner {
            return Ok(NodeAccess::Write);
        }
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        for node_id in node_ids_nearest_first {
            let explicit = connection
                .query_row(
                    "SELECT access FROM node_acl WHERE actor_scope = ?1 AND node_id = ?2",
                    params![principal.actor_scope, node_id],
                    |row| row.get::<_, String>(0),
                )
                .optional()?;
            if let Some(value) = explicit {
                return Ok(intersect_role_access(
                    principal.role,
                    NodeAccess::parse(&value)?,
                ));
            }
        }
        if !allow_role_default {
            return Ok(NodeAccess::Hidden);
        }
        Ok(match principal.role {
            SessionRole::Owner
            | SessionRole::Admin
            | SessionRole::Editor
            | SessionRole::Commenter => NodeAccess::Write,
            SessionRole::Viewer => NodeAccess::Read,
        })
    }

    pub(crate) fn collaboration_document(
        &self,
        node_id: &str,
        canonical_revision: &str,
        now: i64,
    ) -> Result<CollaborationDocumentRecord, AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let record = transaction
            .query_row(
                "SELECT node_id, epoch, version, checkpoint_revision,
                        frozen_reason, expected_revision
                 FROM collaboration_documents WHERE node_id = ?1",
                params![node_id],
                collaboration_document_row,
            )
            .optional()?;
        let record = if let Some(record) = record {
            record
        } else {
            transaction.execute(
                "INSERT INTO collaboration_documents(
                         node_id, epoch, version, checkpoint_revision,
                         frozen_reason, expected_revision, updated_at
                     ) VALUES(?1, 1, 0, ?2, NULL, NULL, ?3)",
                params![node_id, canonical_revision, now],
            )?;
            CollaborationDocumentRecord {
                node_id: node_id.to_owned(),
                epoch: 1,
                version: 0,
                checkpoint_revision: canonical_revision.to_owned(),
                frozen_reason: None,
                expected_revision: None,
            }
        };
        transaction.commit()?;
        Ok(record)
    }

    pub(crate) fn store_collaboration_document(
        &self,
        record: &CollaborationDocumentRecord,
        now: i64,
    ) -> Result<(), AuthError> {
        let epoch = collaboration_integer(record.epoch)?;
        let version = collaboration_integer(record.version)?;
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection.execute(
            "INSERT INTO collaboration_documents(
                 node_id, epoch, version, checkpoint_revision,
                 frozen_reason, expected_revision, updated_at
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(node_id) DO UPDATE SET
                 epoch = excluded.epoch,
                 version = excluded.version,
                 checkpoint_revision = excluded.checkpoint_revision,
                 frozen_reason = excluded.frozen_reason,
                 expected_revision = excluded.expected_revision,
                 updated_at = excluded.updated_at",
            params![
                record.node_id,
                epoch,
                version,
                record.checkpoint_revision,
                record.frozen_reason,
                record.expected_revision,
                now,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn collaboration_receipt(
        &self,
        operation_id: &str,
    ) -> Result<Option<CollaborationReceipt>, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection
            .query_row(
                "SELECT operation_id, actor_scope, actor_id, client_id, node_id,
                        epoch, base_version, base_revision, applied_base_version,
                        applied_base_revision, result_version, result_revision,
                        request_digest, transaction_id
                 FROM collaboration_receipts WHERE operation_id = ?1",
                params![operation_id],
                collaboration_receipt_row,
            )
            .optional()
            .map_err(AuthError::Database)
    }

    pub(crate) fn begin_collaboration_intent(
        &self,
        principal: &SessionPrincipal,
        intent: &NewCollaborationIntent<'_>,
        now: i64,
    ) -> Result<String, AuthError> {
        let epoch = collaboration_integer(intent.epoch)?;
        let base_version = collaboration_integer(intent.base_version)?;
        let applied_base_version = collaboration_integer(intent.applied_base_version)?;
        let result_version = collaboration_integer(intent.result_version)?;
        let intent_id = random_hex(16)?;
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let already_committed: bool = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM collaboration_receipts WHERE operation_id = ?1
             )",
            params![intent.operation_id],
            |row| row.get(0),
        )?;
        if already_committed {
            return Err(AuthError::InvalidControlPlane);
        }
        let node_recovery_pending: bool = transaction.query_row(
            "SELECT EXISTS(
                 SELECT 1 FROM collaboration_pending WHERE node_id = ?1
             )",
            params![intent.node_id],
            |row| row.get(0),
        )?;
        if node_recovery_pending {
            return Err(AuthError::InvalidControlPlane);
        }
        transaction.execute(
            "INSERT INTO audit_outbox(
                 intent_id, created_at, event_type, actor_scope, detail,
                 authority_kind, target, expected_revision
             ) VALUES(?1, ?2, 'collaboration_operation_committed', ?3, ?4,
                      'document', ?5, ?6)",
            params![
                intent_id,
                now,
                principal.actor_scope,
                intent.detail,
                intent.node_id,
                intent.result_revision,
            ],
        )?;
        transaction.execute(
            "INSERT INTO collaboration_pending(
                 intent_id, operation_id, actor_scope, actor_id, client_id,
                 node_id, epoch, base_version, base_revision, applied_base_version,
                 applied_base_revision, result_version, result_revision,
                 request_digest, transaction_id
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12,
                      ?13, ?14, ?15)",
            params![
                intent_id,
                intent.operation_id,
                principal.actor_scope,
                intent.actor_id,
                intent.client_id,
                intent.node_id,
                epoch,
                base_version,
                intent.base_revision,
                applied_base_version,
                intent.applied_base_revision,
                result_version,
                intent.result_revision,
                intent.request_digest,
                intent.transaction_id,
            ],
        )?;
        transaction.commit()?;
        Ok(intent_id)
    }

    pub(crate) fn cancel_collaboration_intent(&self, intent_id: &str) -> Result<(), AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            "DELETE FROM collaboration_pending WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.execute(
            "DELETE FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn finalize_collaboration_intent(
        &self,
        intent_id: &str,
        now: i64,
        recovery_detail: Option<&str>,
    ) -> Result<CollaborationReceipt, AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let audit =
            pending_audit_intent(&transaction, intent_id)?.ok_or(AuthError::InvalidControlPlane)?;
        let pending = pending_collaboration_intent(&transaction, intent_id)?
            .ok_or(AuthError::InvalidControlPlane)?;
        let detail = match recovery_detail {
            Some(recovery) => format!("{};{recovery}", audit.detail),
            None => audit.detail,
        };
        finalize_collaboration_transaction(&transaction, &pending, now)?;
        insert_event(
            &transaction,
            now,
            &audit.event_type,
            Some(&audit.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            &audit.event_type,
            &audit.actor_scope,
            &detail,
        )?;
        transaction.execute(
            "DELETE FROM collaboration_pending WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.execute(
            "DELETE FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.commit()?;
        Ok(collaboration_receipt_from_pending(pending))
    }

    pub(crate) fn audit_receipts(&self) -> Result<Vec<AuditReceipt>, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let mut statement = connection.prepare(
            "SELECT receipt_id, occurred_at, event_type, actor_scope, detail
             FROM audit_receipts ORDER BY receipt_id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(AuditReceipt {
                receipt_id: row.get(0)?,
                occurred_at: row.get(1)?,
                event_type: row.get(2)?,
                actor_scope: row.get(3)?,
                detail: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuthError::Database)
    }

    pub(crate) fn record_completed_operation(
        &self,
        principal: &SessionPrincipal,
        event_type: &str,
        detail: &str,
        now: i64,
    ) -> Result<(), AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        insert_event(
            &transaction,
            now,
            event_type,
            Some(&principal.actor_scope),
            detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            event_type,
            &principal.actor_scope,
            detail,
        )?;
        transaction.commit().map_err(AuthError::Database)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn begin_audit_intent(
        &self,
        principal: &SessionPrincipal,
        event_type: &str,
        detail: &str,
        authority_kind: &str,
        target: &str,
        expected_revision: &str,
        now: i64,
    ) -> Result<String, AuthError> {
        if !matches!(authority_kind, "document" | "workspace") {
            return Err(AuthError::InvalidControlPlane);
        }
        let intent_id = random_hex(16)?;
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection.execute(
            "INSERT INTO audit_outbox(
                 intent_id, created_at, event_type, actor_scope, detail,
                 authority_kind, target, expected_revision
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                intent_id,
                now,
                event_type,
                principal.actor_scope,
                detail,
                authority_kind,
                target,
                expected_revision,
            ],
        )?;
        Ok(intent_id)
    }

    pub(crate) fn cancel_audit_intent(&self, intent_id: &str) -> Result<(), AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        connection.execute(
            "DELETE FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
        )?;
        Ok(())
    }

    pub(crate) fn finalize_audit_intent(
        &self,
        intent_id: &str,
        now: i64,
        recovery_detail: Option<&str>,
    ) -> Result<(), AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let intent =
            pending_audit_intent(&transaction, intent_id)?.ok_or(AuthError::InvalidControlPlane)?;
        let detail = match recovery_detail {
            Some(recovery) => format!("{};{recovery}", intent.detail),
            None => intent.detail,
        };
        insert_event(
            &transaction,
            now,
            &intent.event_type,
            Some(&intent.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(
            &transaction,
            now,
            &intent.event_type,
            &intent.actor_scope,
            &detail,
        )?;
        transaction.execute(
            "DELETE FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn update_audit_intent_authority(
        &self,
        intent_id: &str,
        authority_kind: &str,
        target: &str,
        expected_revision: &str,
    ) -> Result<(), AuthError> {
        if !matches!(authority_kind, "document" | "workspace") {
            return Err(AuthError::InvalidControlPlane);
        }
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let changed = connection.execute(
            "UPDATE audit_outbox
             SET authority_kind = ?1, target = ?2, expected_revision = ?3
             WHERE intent_id = ?4",
            params![authority_kind, target, expected_revision, intent_id],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(AuthError::InvalidControlPlane)
        }
    }

    pub(crate) fn pending_audit_intents(&self) -> Result<Vec<PendingAuditIntent>, AuthError> {
        let connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let mut statement = connection.prepare(
            "SELECT intent_id, created_at, event_type, actor_scope, detail,
                    authority_kind, target, expected_revision
             FROM audit_outbox ORDER BY created_at, intent_id",
        )?;
        let rows = statement.query_map([], pending_audit_intent_row)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(AuthError::Database)
    }

    pub(crate) fn recover_audit_intent(
        &self,
        intent_id: &str,
        authority_confirmed: bool,
        now: i64,
    ) -> Result<(), AuthError> {
        let mut connection = self.connection.lock().map_err(|_| AuthError::Poisoned)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let intent =
            pending_audit_intent(&transaction, intent_id)?.ok_or(AuthError::InvalidControlPlane)?;
        let collaboration = pending_collaboration_intent(&transaction, intent_id)?;
        let (event_type, detail) = if authority_confirmed {
            if let Some(collaboration) = &collaboration {
                finalize_collaboration_transaction(&transaction, collaboration, now)?;
            }
            (
                intent.event_type,
                format!("{};auditRecovery=authority_confirmed", intent.detail),
            )
        } else {
            (
                "audit_intent_recovered".to_owned(),
                format!(
                    "intent={};intendedEvent={};outcome=indeterminate_authority",
                    intent.intent_id, intent.event_type
                ),
            )
        };
        insert_event(
            &transaction,
            now,
            &event_type,
            Some(&intent.actor_scope),
            &detail,
        )?;
        insert_audit_receipt(&transaction, now, &event_type, &intent.actor_scope, &detail)?;
        transaction.execute(
            "DELETE FROM collaboration_pending WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.execute(
            "DELETE FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

struct SessionMaterial {
    session_id: String,
    token: String,
    token_digest: Vec<u8>,
    created_at: i64,
    absolute_expires_at: i64,
    idle_expires_at: i64,
}

impl SessionMaterial {
    fn new(now: i64, policy: SessionPolicy) -> Result<Self, AuthError> {
        let token = random_hex(32)?;
        Ok(Self {
            session_id: random_hex(16)?,
            token_digest: session_digest(&token)?,
            token,
            created_at: now,
            absolute_expires_at: now + policy.absolute_seconds,
            idle_expires_at: now + policy.idle_seconds.min(policy.absolute_seconds),
        })
    }

    fn issued(self, actor_scope: String, role: SessionRole) -> IssuedSession {
        IssuedSession {
            token: self.token,
            principal: SessionPrincipal {
                actor_scope,
                role,
                session_id: self.session_id,
                absolute_expires_at: self.absolute_expires_at,
                idle_expires_at: self.idle_expires_at,
            },
        }
    }
}

struct SessionRow {
    session_id: String,
    actor_scope: String,
    absolute_expires_at: i64,
    idle_expires_at: i64,
    role: SessionRole,
    revoked_at: Option<i64>,
    disabled_at: Option<i64>,
}

fn session_row(row: &rusqlite::Row<'_>) -> Result<SessionRow, rusqlite::Error> {
    let role =
        SessionRole::parse(&row.get::<_, String>(4)?).map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(SessionRow {
        session_id: row.get(0)?,
        actor_scope: row.get(1)?,
        absolute_expires_at: row.get(2)?,
        idle_expires_at: row.get(3)?,
        role,
        revoked_at: row.get(5)?,
        disabled_at: row.get(6)?,
    })
}

fn expire_inactive_sessions(connection: &Connection, now: i64) -> Result<(), AuthError> {
    connection.execute(
        "UPDATE sessions SET revoked_at = ?1, end_reason = 'expired'
         WHERE revoked_at IS NULL AND (absolute_expires_at <= ?1 OR idle_expires_at <= ?1)",
        params![now],
    )?;
    Ok(())
}

fn pending_audit_intent_row(
    row: &rusqlite::Row<'_>,
) -> Result<PendingAuditIntent, rusqlite::Error> {
    Ok(PendingAuditIntent {
        intent_id: row.get(0)?,
        created_at: row.get(1)?,
        event_type: row.get(2)?,
        actor_scope: row.get(3)?,
        detail: row.get(4)?,
        authority_kind: row.get(5)?,
        target: row.get(6)?,
        expected_revision: row.get(7)?,
    })
}

fn pending_audit_intent(
    connection: &Connection,
    intent_id: &str,
) -> Result<Option<PendingAuditIntent>, AuthError> {
    connection
        .query_row(
            "SELECT intent_id, created_at, event_type, actor_scope, detail,
                    authority_kind, target, expected_revision
             FROM audit_outbox WHERE intent_id = ?1",
            params![intent_id],
            pending_audit_intent_row,
        )
        .optional()
        .map_err(AuthError::Database)
}

fn collaboration_integer(value: u64) -> Result<i64, AuthError> {
    i64::try_from(value).map_err(|_| AuthError::InvalidControlPlane)
}

fn collaboration_u64(value: i64) -> Result<u64, rusqlite::Error> {
    u64::try_from(value).map_err(|_| rusqlite::Error::InvalidQuery)
}

fn collaboration_document_row(
    row: &rusqlite::Row<'_>,
) -> Result<CollaborationDocumentRecord, rusqlite::Error> {
    Ok(CollaborationDocumentRecord {
        node_id: row.get(0)?,
        epoch: collaboration_u64(row.get(1)?)?,
        version: collaboration_u64(row.get(2)?)?,
        checkpoint_revision: row.get(3)?,
        frozen_reason: row.get(4)?,
        expected_revision: row.get(5)?,
    })
}

fn collaboration_receipt_row(
    row: &rusqlite::Row<'_>,
) -> Result<CollaborationReceipt, rusqlite::Error> {
    Ok(CollaborationReceipt {
        operation_id: row.get(0)?,
        actor_scope: row.get(1)?,
        actor_id: row.get(2)?,
        client_id: row.get(3)?,
        node_id: row.get(4)?,
        epoch: collaboration_u64(row.get(5)?)?,
        base_version: collaboration_u64(row.get(6)?)?,
        base_revision: row.get(7)?,
        applied_base_version: collaboration_u64(row.get(8)?)?,
        applied_base_revision: row.get(9)?,
        result_version: collaboration_u64(row.get(10)?)?,
        result_revision: row.get(11)?,
        request_digest: row.get(12)?,
        transaction_id: row.get(13)?,
    })
}

fn pending_collaboration_intent_row(
    row: &rusqlite::Row<'_>,
) -> Result<PendingCollaborationIntent, rusqlite::Error> {
    Ok(PendingCollaborationIntent {
        intent_id: row.get(0)?,
        operation_id: row.get(1)?,
        actor_scope: row.get(2)?,
        actor_id: row.get(3)?,
        client_id: row.get(4)?,
        node_id: row.get(5)?,
        epoch: collaboration_u64(row.get(6)?)?,
        base_version: collaboration_u64(row.get(7)?)?,
        base_revision: row.get(8)?,
        applied_base_version: collaboration_u64(row.get(9)?)?,
        applied_base_revision: row.get(10)?,
        result_version: collaboration_u64(row.get(11)?)?,
        result_revision: row.get(12)?,
        request_digest: row.get(13)?,
        transaction_id: row.get(14)?,
    })
}

fn pending_collaboration_intent(
    connection: &Connection,
    intent_id: &str,
) -> Result<Option<PendingCollaborationIntent>, AuthError> {
    connection
        .query_row(
            "SELECT intent_id, operation_id, actor_scope, actor_id, client_id,
                    node_id, epoch, base_version, base_revision,
                    applied_base_version, applied_base_revision, result_version,
                    result_revision, request_digest, transaction_id
             FROM collaboration_pending WHERE intent_id = ?1",
            params![intent_id],
            pending_collaboration_intent_row,
        )
        .optional()
        .map_err(AuthError::Database)
}

fn finalize_collaboration_transaction(
    transaction: &Transaction<'_>,
    pending: &PendingCollaborationIntent,
    now: i64,
) -> Result<(), AuthError> {
    transaction.execute(
        "INSERT INTO collaboration_receipts(
             operation_id, actor_scope, actor_id, client_id, node_id,
             epoch, base_version, base_revision, applied_base_version,
             applied_base_revision, result_version, result_revision,
             request_digest, transaction_id, occurred_at
         ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                  ?14, ?15)
         ",
        params![
            pending.operation_id,
            pending.actor_scope,
            pending.actor_id,
            pending.client_id,
            pending.node_id,
            collaboration_integer(pending.epoch)?,
            collaboration_integer(pending.base_version)?,
            pending.base_revision,
            collaboration_integer(pending.applied_base_version)?,
            pending.applied_base_revision,
            collaboration_integer(pending.result_version)?,
            pending.result_revision,
            pending.request_digest,
            pending.transaction_id,
            now,
        ],
    )?;
    transaction.execute(
        "INSERT INTO collaboration_documents(
             node_id, epoch, version, checkpoint_revision,
             frozen_reason, expected_revision, updated_at
         ) VALUES(?1, ?2, ?3, ?4, NULL, NULL, ?5)
         ON CONFLICT(node_id) DO UPDATE SET
             epoch = excluded.epoch,
             version = excluded.version,
             checkpoint_revision = excluded.checkpoint_revision,
             frozen_reason = NULL,
             expected_revision = NULL,
             updated_at = excluded.updated_at",
        params![
            pending.node_id,
            collaboration_integer(pending.epoch)?,
            collaboration_integer(pending.result_version)?,
            pending.result_revision,
            now,
        ],
    )?;
    Ok(())
}

fn collaboration_receipt_from_pending(pending: PendingCollaborationIntent) -> CollaborationReceipt {
    CollaborationReceipt {
        operation_id: pending.operation_id,
        actor_scope: pending.actor_scope,
        actor_id: pending.actor_id,
        client_id: pending.client_id,
        node_id: pending.node_id,
        epoch: pending.epoch,
        base_version: pending.base_version,
        base_revision: pending.base_revision,
        applied_base_version: pending.applied_base_version,
        applied_base_revision: pending.applied_base_revision,
        result_version: pending.result_version,
        result_revision: pending.result_revision,
        request_digest: pending.request_digest,
        transaction_id: pending.transaction_id,
    }
}

fn ensure_workspace_scope(connection: &mut Connection) -> Result<(), AuthError> {
    let existing: Option<String> = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            params![WORKSPACE_SCOPE_KEY],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(existing) = existing {
        return if decode_hex_32(&existing).is_some() {
            Ok(())
        } else {
            Err(AuthError::InvalidControlPlane)
        };
    }
    let scope = random_hex(32)?;
    connection.execute(
        "INSERT INTO metadata(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![WORKSPACE_SCOPE_KEY, scope],
    )?;
    Ok(())
}

fn ensure_bootstrap_secret(
    connection: &mut Connection,
    secret_path: &Path,
) -> Result<(), AuthError> {
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    if owner_exists(&transaction)? {
        transaction.execute(
            "DELETE FROM metadata WHERE key = 'bootstrap_secret_digest'",
            [],
        )?;
        transaction.commit()?;
        let _ = fs::remove_file(secret_path);
        return Ok(());
    }
    let secret = if secret_path.exists() {
        let mut value = String::new();
        OpenOptions::new()
            .read(true)
            .open(secret_path)?
            .read_to_string(&mut value)?;
        value.trim().to_owned()
    } else {
        let value = random_hex(32)?;
        write_new_secret(secret_path, &value)?;
        value
    };
    if decode_hex_32(&secret).is_none() {
        return Err(AuthError::InvalidControlPlane);
    }
    transaction.execute(
        "INSERT INTO metadata(key, value) VALUES('bootstrap_secret_digest', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![bootstrap_digest(&secret)],
    )?;
    transaction.commit()?;
    Ok(())
}

fn secure_control_plane_paths(
    root: &Path,
    database_path: &Path,
    bootstrap_secret_path: &Path,
    reverse_proxy_secret_path: &Path,
) -> Result<(), AuthError> {
    let database_sidecars = database_sidecar_paths(database_path);
    if path_is_link_or_reparse(root)? {
        return Err(AuthError::InvalidControlPlane);
    }
    for path in [database_path]
        .into_iter()
        .chain(database_sidecars.iter().map(PathBuf::as_path))
        .filter(|path| path.exists())
    {
        if path_is_link_or_reparse(path)? || file_has_multiple_links(path)? {
            return Err(AuthError::InvalidControlPlane);
        }
    }
    if (bootstrap_secret_path.exists() && file_has_multiple_links(bootstrap_secret_path)?)
        || (reverse_proxy_secret_path.exists()
            && file_has_multiple_links(reverse_proxy_secret_path)?)
    {
        return Err(AuthError::InvalidControlPlane);
    }
    if bootstrap_secret_path.exists() && path_is_link_or_reparse(bootstrap_secret_path)? {
        return Err(AuthError::InvalidControlPlane);
    }
    if reverse_proxy_secret_path.exists() && path_is_link_or_reparse(reverse_proxy_secret_path)? {
        return Err(AuthError::InvalidControlPlane);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        for path in [database_path]
            .into_iter()
            .chain(database_sidecars.iter().map(PathBuf::as_path))
            .filter(|path| path.exists())
        {
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        if bootstrap_secret_path.exists() {
            fs::set_permissions(bootstrap_secret_path, fs::Permissions::from_mode(0o600))?;
        }
        if reverse_proxy_secret_path.exists() {
            fs::set_permissions(reverse_proxy_secret_path, fs::Permissions::from_mode(0o600))?;
        }
    }
    #[cfg(windows)]
    {
        secure_windows_dacl(root, true)?;
        for path in [database_path]
            .into_iter()
            .chain(database_sidecars.iter().map(PathBuf::as_path))
            .filter(|path| path.exists())
        {
            secure_windows_dacl(path, false)?;
        }
        if bootstrap_secret_path.exists() {
            secure_windows_dacl(bootstrap_secret_path, false)?;
        }
        if reverse_proxy_secret_path.exists() {
            secure_windows_dacl(reverse_proxy_secret_path, false)?;
        }
    }
    Ok(())
}

fn prepare_control_plane_paths(
    root: &Path,
    database_path: &Path,
    bootstrap_secret_path: &Path,
    reverse_proxy_secret_path: &Path,
) -> Result<(), AuthError> {
    let database_sidecars = database_sidecar_paths(database_path);
    if path_is_link_or_reparse(root)?
        || (bootstrap_secret_path.exists() && path_is_link_or_reparse(bootstrap_secret_path)?)
        || (reverse_proxy_secret_path.exists()
            && path_is_link_or_reparse(reverse_proxy_secret_path)?)
        || (bootstrap_secret_path.exists() && file_has_multiple_links(bootstrap_secret_path)?)
        || (reverse_proxy_secret_path.exists()
            && file_has_multiple_links(reverse_proxy_secret_path)?)
    {
        return Err(AuthError::InvalidControlPlane);
    }
    for path in [database_path]
        .into_iter()
        .chain(database_sidecars.iter().map(PathBuf::as_path))
        .filter(|path| path.exists())
    {
        if path_is_link_or_reparse(path)? || file_has_multiple_links(path)? {
            return Err(AuthError::InvalidControlPlane);
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))?;
        if fs::metadata(root)?.permissions().mode() & 0o777 != 0o700 {
            return Err(AuthError::InvalidControlPlane);
        }
    }
    #[cfg(windows)]
    secure_windows_dacl(root, true)?;
    Ok(())
}

fn database_sidecar_paths(database_path: &Path) -> [PathBuf; 3] {
    DATABASE_SIDECAR_SUFFIXES
        .map(|suffix| database_path.with_file_name(format!("{DATABASE_FILE}{suffix}")))
}

fn secure_secret_file(path: &Path) -> Result<(), AuthError> {
    if path_is_link_or_reparse(path)? || file_has_multiple_links(path)? {
        return Err(AuthError::InvalidControlPlane);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    #[cfg(windows)]
    secure_windows_dacl(path, false)?;
    Ok(())
}

fn path_is_link_or_reparse(path: &Path) -> Result<bool, std::io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Ok(true);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
        Ok(metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
    }
    #[cfg(not(windows))]
    {
        Ok(false)
    }
}

fn file_has_multiple_links(path: &Path) -> Result<bool, std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(path)?;
        Ok(metadata.nlink() > 1)
    }
    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("fsutil")
            .arg("hardlink")
            .arg("list")
            .arg(path)
            .output()?;
        if !output.status.success() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "cannot verify Windows control-plane hard links",
            ));
        }
        let links = String::from_utf8(output.stdout).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Windows hard-link inventory is not UTF-8",
            )
        })?;
        Ok(links.lines().filter(|line| !line.trim().is_empty()).count() > 1)
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(false)
    }
}

#[cfg(windows)]
fn secure_windows_dacl(path: &Path, directory: bool) -> Result<(), std::io::Error> {
    use std::process::Command;

    const SCRIPT: &str = r"
$ErrorActionPreference = 'Stop'
$path = [Environment]::GetEnvironmentVariable('WEFTEXT_CONTROL_ACL_PATH', 'Process')
$isDirectory = [Environment]::GetEnvironmentVariable('WEFTEXT_CONTROL_ACL_DIRECTORY', 'Process') -eq '1'
if ([String]::IsNullOrWhiteSpace($path)) { throw 'missing control-plane ACL path' }
$current = [Security.Principal.WindowsIdentity]::GetCurrent().User
$system = [Security.Principal.SecurityIdentifier]::new('S-1-5-18')
$administrators = [Security.Principal.SecurityIdentifier]::new('S-1-5-32-544')
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
if ($isDirectory) {
    $item = [IO.DirectoryInfo]::new($path)
    $item.SetAccessControl($acl)
} else {
    $item = [IO.FileInfo]::new($path)
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
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Windows control-plane ACL path is not Unicode",
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
        .env("WEFTEXT_CONTROL_ACL_PATH", powershell_path)
        .env(
            "WEFTEXT_CONTROL_ACL_DIRECTORY",
            if directory { "1" } else { "0" },
        )
        .output()?;
    if result.status.success() {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Windows control-plane ACL could not be restricted for {}: {}",
                path.display(),
                String::from_utf8_lossy(&result.stderr).trim()
            ),
        ))
    }
}

fn write_new_secret(path: &Path, secret: &str) -> Result<(), AuthError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(secret.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn owner_exists(connection: &Connection) -> Result<bool, rusqlite::Error> {
    connection.query_row("SELECT EXISTS(SELECT 1 FROM owner)", [], |row| row.get(0))
}

fn enabled_owner_count(connection: &Connection) -> Result<i64, rusqlite::Error> {
    connection.query_row(
        "SELECT COUNT(*) FROM accounts WHERE role = 'owner' AND disabled_at IS NULL",
        [],
        |row| row.get(0),
    )
}

fn intersect_role_access(role: SessionRole, explicit: NodeAccess) -> NodeAccess {
    match (role, explicit) {
        (_, NodeAccess::Hidden) => NodeAccess::Hidden,
        (SessionRole::Viewer, _) => NodeAccess::Read,
        (
            SessionRole::Owner | SessionRole::Admin | SessionRole::Editor | SessionRole::Commenter,
            access,
        ) => access,
    }
}

fn migrate_account_roles(connection: &mut Connection) -> Result<(), AuthError> {
    let schema = connection
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'accounts'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(AuthError::InvalidControlPlane)?;
    let normalized = schema
        .to_ascii_lowercase()
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let legacy_constraint = "check(rolein('owner','editor','viewer'))";
    let current_constraint = "check(rolein('owner','admin','editor','commenter','viewer'))";
    let is_legacy = normalized.contains(legacy_constraint);
    let is_current = normalized.contains(current_constraint);
    if !is_legacy && !is_current {
        return Err(AuthError::InvalidControlPlane);
    }

    let unsupported_roles = connection.query_row(
        "SELECT COUNT(*) FROM accounts
         WHERE role NOT IN ('owner', 'admin', 'editor', 'commenter', 'viewer')",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    if unsupported_roles != 0 {
        return Err(AuthError::InvalidControlPlane);
    }
    if is_current {
        return Ok(());
    }

    connection.pragma_update(None, "foreign_keys", "OFF")?;
    let migration = (|| -> Result<(), AuthError> {
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute_batch(
            "CREATE TABLE accounts_roles_v2 (
                 actor_scope TEXT PRIMARY KEY,
                 login TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 role TEXT NOT NULL CHECK(role IN ('owner', 'admin', 'editor', 'commenter', 'viewer')),
                 created_at INTEGER NOT NULL,
                 disabled_at INTEGER
             );
             INSERT INTO accounts_roles_v2(
                 actor_scope, login, password_hash, role, created_at, disabled_at
             )
             SELECT actor_scope, login, password_hash, role, created_at, disabled_at
             FROM accounts;
             DROP TABLE accounts;
             ALTER TABLE accounts_roles_v2 RENAME TO accounts;",
        )?;
        transaction.commit()?;
        Ok(())
    })();
    let foreign_keys_enabled = connection.pragma_update(None, "foreign_keys", "ON");
    migration?;
    foreign_keys_enabled?;

    let has_violation = {
        let mut statement = connection.prepare("PRAGMA foreign_key_check")?;
        let mut rows = statement.query([])?;
        rows.next()?.is_some()
    };
    if has_violation {
        return Err(AuthError::InvalidControlPlane);
    }
    Ok(())
}

fn migrate_session_foreign_key(connection: &mut Connection) -> Result<(), AuthError> {
    let foreign_table = connection
        .query_row("PRAGMA foreign_key_list(sessions)", [], |row| {
            row.get::<_, String>(2)
        })
        .optional()?;
    if foreign_table.as_deref() != Some("owner") {
        return Ok(());
    }
    connection.execute_batch(
        "PRAGMA foreign_keys=OFF;
         BEGIN IMMEDIATE;
         ALTER TABLE sessions RENAME TO sessions_owner_legacy;
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
         INSERT INTO sessions SELECT * FROM sessions_owner_legacy;
         DROP TABLE sessions_owner_legacy;
         COMMIT;
         PRAGMA foreign_keys=ON;",
    )?;
    Ok(())
}

fn insert_session(
    transaction: &Transaction<'_>,
    actor_scope: &str,
    material: &SessionMaterial,
) -> Result<(), AuthError> {
    transaction.execute(
        "INSERT INTO sessions(
             session_id, actor_scope, token_digest, created_at, last_seen_at,
             absolute_expires_at, idle_expires_at
         ) VALUES(?1, ?2, ?3, ?4, ?4, ?5, ?6)",
        params![
            material.session_id,
            actor_scope,
            material.token_digest,
            material.created_at,
            material.absolute_expires_at,
            material.idle_expires_at,
        ],
    )?;
    Ok(())
}

fn revoke_prior_token(
    transaction: &Transaction<'_>,
    token: Option<&str>,
    now: i64,
    reason: &str,
) -> Result<(), AuthError> {
    let Some(token) = token else {
        return Ok(());
    };
    let Ok(digest) = session_digest(token) else {
        return Ok(());
    };
    transaction.execute(
        "UPDATE sessions SET revoked_at = ?1, end_reason = ?2
         WHERE token_digest = ?3 AND revoked_at IS NULL",
        params![now, reason, digest],
    )?;
    Ok(())
}

fn insert_event(
    connection: &Connection,
    now: i64,
    event_type: &str,
    actor_scope: Option<&str>,
    detail: &str,
) -> Result<(), AuthError> {
    connection.execute(
        "INSERT INTO security_events(occurred_at, event_type, actor_scope, detail)
         VALUES(?1, ?2, ?3, ?4)",
        params![now, event_type, actor_scope, detail],
    )?;
    connection.execute(
        "DELETE FROM security_events WHERE event_id IN (
             SELECT event_id FROM security_events ORDER BY event_id DESC
             LIMIT -1 OFFSET ?1
         )",
        params![SECURITY_EVENT_LIMIT],
    )?;
    Ok(())
}

fn insert_audit_receipt(
    connection: &Connection,
    now: i64,
    event_type: &str,
    actor_scope: &str,
    detail: &str,
) -> Result<(), AuthError> {
    connection.execute(
        "INSERT INTO audit_receipts(occurred_at, event_type, actor_scope, detail)
         VALUES(?1, ?2, ?3, ?4)",
        params![now, event_type, actor_scope, detail],
    )?;
    Ok(())
}

fn validate_password(password: &str) -> Result<(), AuthError> {
    if (12..=1_024).contains(&password.len()) {
        Ok(())
    } else {
        Err(AuthError::Password)
    }
}

fn validate_login(login: &str) -> Result<(), AuthError> {
    if (3..=64).contains(&login.len())
        && login.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        Ok(())
    } else {
        Err(AuthError::InvalidLogin)
    }
}

fn argon2() -> Result<Argon2<'static>, AuthError> {
    let parameters = Params::new(
        ARGON2_MEMORY_KIB,
        ARGON2_ITERATIONS,
        ARGON2_PARALLELISM,
        None,
    )
    .map_err(|_| AuthError::Password)?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters))
}

fn hash_password(password: &str) -> Result<String, AuthError> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::fill(&mut salt_bytes)?;
    let salt = SaltString::encode_b64(&salt_bytes).map_err(|_| AuthError::Password)?;
    argon2()?
        .hash_password(password.as_bytes(), &salt)
        .map(|value| value.to_string())
        .map_err(|_| AuthError::Password)
}

fn verify_password(encoded: &str, password: &str) -> bool {
    PasswordHash::new(encoded).is_ok_and(|hash| {
        argon2().is_ok_and(|algorithm| {
            algorithm
                .verify_password(password.as_bytes(), &hash)
                .is_ok()
        })
    })
}

fn random_hex(length: usize) -> Result<String, AuthError> {
    let mut bytes = vec![0_u8; length];
    getrandom::fill(&mut bytes)?;
    Ok(hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(char::from(HEX[usize::from(byte >> 4)]));
        value.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    value
}

fn decode_hex_32(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let high = hex_value(chunk[0])?;
        let low = hex_value(chunk[1])?;
        bytes[index] = (high << 4) | low;
    }
    Some(bytes)
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn bootstrap_digest(secret: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"weftext-bootstrap-secret-v1\0");
    hasher.update(secret.as_bytes());
    hasher.finalize().to_vec()
}

pub(crate) fn reverse_proxy_token_digest(token: &str) -> Option<[u8; 32]> {
    let bytes = decode_hex_32(token)?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext-reverse-proxy-token-v1\0");
    hasher.update(bytes);
    Some(hasher.finalize().into())
}

fn session_digest(token: &str) -> Result<Vec<u8>, AuthError> {
    let bytes = decode_hex_32(token).ok_or(AuthError::InvalidSession)?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext-session-token-v1\0");
    hasher.update(bytes);
    Ok(hasher.finalize().to_vec())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    left.ct_eq(right).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_core::create_workspace;

    fn fixture(policy: SessionPolicy) -> (tempfile::TempDir, PathBuf, ControlPlane, String) {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        let control = temporary.path().join("ControlPlane");
        create_workspace(&workspace).expect("workspace");
        let canonical_workspace = fs::canonicalize(&workspace).expect("canonical workspace");
        let plane = ControlPlane::open(&canonical_workspace, &control, policy).expect("control");
        let secret = fs::read_to_string(plane.bootstrap_secret_path())
            .expect("bootstrap secret")
            .trim()
            .to_owned();
        (temporary, workspace, plane, secret)
    }

    #[test]
    fn password_hashes_use_argon2id_parameters_and_random_salts() {
        let first = hash_password("correct horse battery staple").expect("first hash");
        let second = hash_password("correct horse battery staple").expect("second hash");
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
        assert!(verify_password(&first, "correct horse battery staple"));
        assert!(!verify_password(&first, "wrong password"));
    }

    #[test]
    fn bootstrap_is_one_time_and_session_tokens_are_digest_only() {
        let (_temporary, _workspace, plane, secret) = fixture(SessionPolicy::default());
        let issued = plane
            .bootstrap(&secret, "correct horse battery staple", None, 100)
            .expect("bootstrap");
        assert!(!plane.bootstrap_secret_path().exists());
        assert!(matches!(
            plane.bootstrap(&secret, "another acceptable password", None, 101),
            Err(AuthError::BootstrapUnavailable)
        ));
        let connection = plane.connection.lock().expect("connection");
        let stored: Vec<u8> = connection
            .query_row("SELECT token_digest FROM sessions", [], |row| row.get(0))
            .expect("stored digest");
        assert_eq!(stored.len(), 32);
        assert!(!String::from_utf8_lossy(&stored).contains(&issued.token));
        let plan: String = connection
            .query_row(
                "EXPLAIN QUERY PLAN
                 SELECT session_id FROM sessions WHERE token_digest = ?1",
                params![stored],
                |row| row.get(3),
            )
            .expect("session lookup plan");
        assert!(
            plan.contains("token_digest") || plan.contains("sqlite_autoindex_sessions"),
            "session validation must use the unique digest index: {plan}"
        );
    }

    #[test]
    fn collaboration_outbox_recovery_links_receipt_and_checkpoint_without_content_bytes() {
        let (_temporary, _workspace, plane, secret) = fixture(SessionPolicy::default());
        let issued = plane
            .bootstrap(&secret, "correct horse battery staple", None, 100)
            .expect("bootstrap");
        let node_id = "11111111-1111-4111-8111-111111111111";
        let operation_id = "22222222-2222-4222-8222-222222222222";
        let intent = NewCollaborationIntent {
            actor_id: "33333333-3333-4333-8333-333333333333",
            client_id: "44444444-4444-4444-8444-444444444444",
            operation_id,
            node_id,
            epoch: 3,
            base_version: 8,
            base_revision: &"a".repeat(64),
            applied_base_version: 8,
            applied_base_revision: &"a".repeat(64),
            result_version: 9,
            result_revision: &"b".repeat(64),
            request_digest: &"c".repeat(64),
            transaction_id: operation_id,
            detail: "actor=wire;client=client;operation=operation;transaction=transaction",
        };
        let intent_id = plane
            .begin_collaboration_intent(&issued.principal, &intent, 101)
            .expect("begin collaboration intent");
        assert!(plane.collaboration_receipt(operation_id).unwrap().is_none());

        plane
            .recover_audit_intent(&intent_id, true, 102)
            .expect("recover confirmed canonical commit");
        let receipt = plane
            .collaboration_receipt(operation_id)
            .expect("receipt query")
            .expect("durable receipt");
        assert_eq!(receipt.actor_scope, issued.principal.actor_scope);
        assert_eq!(receipt.client_id, intent.client_id);
        assert_eq!(receipt.applied_base_version, 8);
        assert_eq!(receipt.applied_base_revision, intent.applied_base_revision);
        assert_eq!(receipt.result_version, 9);
        assert_eq!(receipt.result_revision, intent.result_revision);
        let document = plane
            .collaboration_document(node_id, intent.result_revision, 103)
            .expect("collaboration checkpoint");
        assert_eq!(document.epoch, 3);
        assert_eq!(document.version, 9);
        assert_eq!(document.checkpoint_revision, intent.result_revision);

        let rejected_operation = "55555555-5555-4555-8555-555555555555";
        let rejected = NewCollaborationIntent {
            operation_id: rejected_operation,
            transaction_id: rejected_operation,
            ..intent
        };
        let rejected_intent = plane
            .begin_collaboration_intent(&issued.principal, &rejected, 104)
            .expect("begin rejected collaboration intent");
        plane
            .recover_audit_intent(&rejected_intent, false, 105)
            .expect("recover unconfirmed intent");
        assert!(
            plane
                .collaboration_receipt(rejected_operation)
                .unwrap()
                .is_none()
        );

        let connection = plane.connection.lock().expect("connection");
        for table in [
            "collaboration_documents",
            "collaboration_receipts",
            "collaboration_pending",
        ] {
            let columns = connection
                .prepare(&format!("PRAGMA table_info({table})"))
                .expect("table info")
                .query_map([], |row| row.get::<_, String>(1))
                .expect("column rows")
                .collect::<Result<Vec<_>, _>>()
                .expect("columns");
            assert!(!columns.iter().any(|column| {
                column.contains("source")
                    || column.contains("replacement")
                    || column.contains("content")
            }));
        }
    }

    #[test]
    fn legacy_three_role_constraint_migrates_atomically_and_survives_restart() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        let control = temporary.path().join("ControlPlane");
        create_workspace(&workspace).expect("workspace");
        fs::create_dir(&control).expect("control directory");
        let database_path = control.join(DATABASE_FILE);
        let legacy = Connection::open(&database_path).expect("legacy database");
        legacy
            .execute_batch(
                "CREATE TABLE accounts (
                     actor_scope TEXT PRIMARY KEY,
                     login TEXT NOT NULL UNIQUE,
                     password_hash TEXT NOT NULL,
                     role TEXT NOT NULL CHECK(role IN ('owner', 'editor', 'viewer')),
                     created_at INTEGER NOT NULL,
                     disabled_at INTEGER
                 );
                 INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
                 VALUES('legacy-editor', 'legacy.editor', 'unused', 'editor', 1);",
            )
            .expect("legacy schema");
        drop(legacy);

        let canonical_workspace = fs::canonicalize(&workspace).expect("canonical workspace");
        let plane = ControlPlane::open(&canonical_workspace, &control, SessionPolicy::default())
            .expect("migrate legacy roles");
        {
            let connection = plane.connection.lock().expect("connection");
            let schema: String = connection
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'accounts'",
                    [],
                    |row| row.get(0),
                )
                .expect("accounts schema");
            assert!(schema.contains("'admin'"));
            assert!(schema.contains("'commenter'"));
            connection
                .execute(
                    "INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
                     VALUES('new-admin', 'new.admin', 'unused', 'admin', 2)",
                    [],
                )
                .expect("new Admin role accepted");
            connection
                .execute(
                    "INSERT INTO accounts(actor_scope, login, password_hash, role, created_at)
                     VALUES('new-commenter', 'new.commenter', 'unused', 'commenter', 3)",
                    [],
                )
                .expect("new Commenter role accepted");
        }
        drop(plane);

        let restarted =
            ControlPlane::open(&canonical_workspace, &control, SessionPolicy::default())
                .expect("reopen migrated control plane");
        let connection = restarted.connection.lock().expect("restarted connection");
        let roles = connection
            .prepare("SELECT role FROM accounts ORDER BY role")
            .expect("role query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("role rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("roles");
        assert_eq!(roles, ["admin", "commenter", "editor"]);
    }

    #[test]
    fn unknown_accounts_role_schema_fails_closed() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        let control = temporary.path().join("ControlPlane");
        create_workspace(&workspace).expect("workspace");
        fs::create_dir(&control).expect("control directory");
        let database = Connection::open(control.join(DATABASE_FILE)).expect("database");
        database
            .execute_batch(
                "CREATE TABLE accounts (
                     actor_scope TEXT PRIMARY KEY,
                     login TEXT NOT NULL UNIQUE,
                     password_hash TEXT NOT NULL,
                     role TEXT NOT NULL,
                     created_at INTEGER NOT NULL,
                     disabled_at INTEGER
                 );",
            )
            .expect("unconstrained schema");
        drop(database);
        let canonical_workspace = fs::canonicalize(&workspace).expect("canonical workspace");
        assert!(matches!(
            ControlPlane::open(&canonical_workspace, &control, SessionPolicy::default()),
            Err(AuthError::InvalidControlPlane)
        ));
    }

    #[test]
    fn session_rotation_logout_revocation_and_expiry_fail_closed() {
        let policy = SessionPolicy {
            absolute_seconds: 100,
            idle_seconds: 10,
        };
        let (_temporary, _workspace, plane, secret) = fixture(policy);
        let bootstrap = plane
            .bootstrap(&secret, "correct horse battery staple", None, 100)
            .expect("bootstrap");
        let login = plane
            .login(
                OWNER_LOGIN,
                "correct horse battery staple",
                Some(&bootstrap.token),
                101,
            )
            .expect("login");
        assert_ne!(bootstrap.token, login.token);
        assert!(matches!(
            plane.validate_session(&bootstrap.token, 102),
            Err(AuthError::InvalidSession)
        ));
        let principal = plane.validate_session(&login.token, 105).expect("valid");
        assert_eq!(principal.idle_expires_at, 115);
        plane.logout(&principal, 106).expect("logout");
        assert!(plane.validate_session(&login.token, 107).is_err());

        let second = plane
            .login(OWNER_LOGIN, "correct horse battery staple", None, 120)
            .expect("second login");
        assert!(matches!(
            plane.validate_session(&second.token, 130),
            Err(AuthError::ExpiredSession)
        ));
        let third = plane
            .login(OWNER_LOGIN, "correct horse battery staple", None, 140)
            .expect("third login");
        let third_principal = plane
            .validate_session(&third.token, 141)
            .expect("third valid");
        plane.revoke_all(&third_principal, 142).expect("revoke all");
        assert!(plane.validate_session(&third.token, 143).is_err());

        let connection = plane.connection.lock().expect("connection");
        let event_types = connection
            .prepare("SELECT event_type FROM security_events ORDER BY event_id")
            .expect("event query")
            .query_map([], |row| row.get::<_, String>(0))
            .expect("event rows")
            .collect::<Result<Vec<_>, _>>()
            .expect("event types");
        for expected in [
            "bootstrap_succeeded",
            "login_succeeded",
            "logout",
            "session_expired",
            "sessions_revoked",
        ] {
            assert!(event_types.iter().any(|value| value == expected));
        }
        drop(connection);

        let absolute_policy = SessionPolicy {
            absolute_seconds: 5,
            idle_seconds: 100,
        };
        let (_absolute_temporary, _absolute_workspace, absolute_plane, absolute_secret) =
            fixture(absolute_policy);
        let absolute = absolute_plane
            .bootstrap(&absolute_secret, "correct horse battery staple", None, 200)
            .expect("absolute bootstrap");
        let absolute_principal = absolute_plane
            .validate_session(&absolute.token, 202)
            .expect("valid before absolute deadline");
        assert_eq!(absolute_principal.absolute_expires_at, 205);
        assert_eq!(absolute_principal.idle_expires_at, 205);
        assert!(matches!(
            absolute_plane.validate_session(&absolute.token, 205),
            Err(AuthError::ExpiredSession)
        ));
    }

    #[test]
    fn control_plane_must_be_disjoint_from_workspace() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let canonical = fs::canonicalize(&workspace).expect("canonical workspace");
        assert!(matches!(
            ControlPlane::open(
                &canonical,
                &workspace.join("control"),
                SessionPolicy::default()
            ),
            Err(AuthError::InvalidControlPlane)
        ));
    }

    #[test]
    fn reverse_proxy_secret_is_persistent_and_digest_only_in_memory() {
        let (_temporary, _workspace, plane, _secret) = fixture(SessionPolicy::default());
        let first = plane
            .provision_reverse_proxy_secret()
            .expect("provision reverse proxy secret");
        let raw = fs::read_to_string(plane.reverse_proxy_secret_path())
            .expect("read reverse proxy secret")
            .trim()
            .to_owned();
        assert_eq!(raw.len(), 64);
        assert_eq!(reverse_proxy_token_digest(&raw), Some(first));
        assert_eq!(
            plane
                .provision_reverse_proxy_secret()
                .expect("reuse reverse proxy secret"),
            first
        );
    }

    #[cfg(unix)]
    #[test]
    fn unix_control_plane_permissions_are_private() {
        use std::os::unix::fs::PermissionsExt;

        let (_temporary, _workspace, plane, _secret) = fixture(SessionPolicy::default());
        let root_mode = fs::metadata(plane.root())
            .expect("control root metadata")
            .permissions()
            .mode()
            & 0o777;
        let database_mode = fs::metadata(plane.database_path())
            .expect("database metadata")
            .permissions()
            .mode()
            & 0o777;
        let secret_mode = fs::metadata(plane.bootstrap_secret_path())
            .expect("secret metadata")
            .permissions()
            .mode()
            & 0o777;
        plane
            .provision_reverse_proxy_secret()
            .expect("provision reverse proxy secret");
        let reverse_proxy_secret_mode = fs::metadata(plane.reverse_proxy_secret_path())
            .expect("reverse proxy secret metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(root_mode, 0o700);
        assert_eq!(database_mode, 0o600);
        assert_eq!(secret_mode, 0o600);
        assert_eq!(reverse_proxy_secret_mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn linked_control_plane_path_is_rejected_before_database_open() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let canonical = fs::canonicalize(&workspace).expect("canonical workspace");
        let target = temporary.path().join("target-control");
        fs::create_dir(&target).expect("control target");
        let linked = temporary.path().join("linked-control");
        symlink(&target, &linked).expect("control link");
        assert!(matches!(
            ControlPlane::open(&canonical, &linked, SessionPolicy::default()),
            Err(AuthError::InvalidControlPlane)
        ));
        assert!(!target.join(DATABASE_FILE).exists());
    }

    #[cfg(unix)]
    #[test]
    fn linked_reverse_proxy_secret_is_rejected() {
        use std::os::unix::fs::symlink;

        let (temporary, _workspace, plane, _secret) = fixture(SessionPolicy::default());
        let target = temporary.path().join("proxy-secret-target");
        fs::write(&target, random_hex(32).expect("secret material")).expect("proxy secret target");
        symlink(&target, plane.reverse_proxy_secret_path()).expect("proxy secret link");
        assert!(matches!(
            plane.provision_reverse_proxy_secret(),
            Err(AuthError::InvalidControlPlane)
        ));
    }
}
