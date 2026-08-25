//! Canonical `AsciiDoc` loopback hosted workspace server with local accounts and node ACL.

mod auth;
mod collaboration;

use std::collections::{BTreeMap, BTreeSet};
use std::convert::Infallible;
use std::fmt;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::extract::{Path as RoutePath, Query, State};
use axum::http::request::Parts;
use axum::http::uri::Authority;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, RwLock, Semaphore, broadcast, watch};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use weftext_backup::{
    ServerBackupPairPlan, ServerControlPlaneBackupError, ServerControlPlaneLease,
    ServerRestorePairPlan, acquire_server_control_plane_lease,
    commit_server_backup_pair_with_lease, commit_server_restore_pair,
    plan_server_backup_pair_with_lease, plan_server_restore_pair, verify_server_backup_pair,
    verify_server_restore_pair,
};
use weftext_core::{
    AdjacentHeadingBody, AnnotationAction, AnnotationAppearance, AnnotationColor, AnnotationKind,
    AnnotationMark, AnnotationReplicaCompleteness, AnnotationResourceMediaKind,
    AnnotationResourceRegion, AnnotationSidecarSnapshot, AnnotationStore, AnnotationTargetIntent,
    CitationAccessScope, CitationAuthoringFailure, CitationEditTarget, CitationMacroIntent,
    CitationPresentationProfile, CitationPresentationRequest, CitationWorkspaceIndex,
    CommittedDocument, CommittedWorkspaceTransaction, DocumentEdit, DocumentEditPlan,
    DocumentError, DocumentModel, DocumentProfileDescriptor, DocumentRevision, DocumentViewModel,
    NavigationContentItem, NavigationNode, NodeId, NodeMetadataProjection, NodeMetadataScope,
    QueryAccessScope, QueryEvaluationContext, QueryExecutionError, QueryWorkspaceIndex,
    ResolvedNodeIcon, TaskAuthoringFailure, TaskDependencyTransactionPlan, TaskEditIntent,
    TaskEditTarget, TaskEditTransactionPlan, TaskId, TaskRecurrenceCompletionContext,
    TaskRecurrenceCompletionFailure, TaskRecurrenceTransactionPlan, TaskTransactionError,
    TaskWorkspaceIndex, TrashItemId, TrashItemManifest, TrashResourceSelection, TrashRestoreMode,
    TrashReviewedAction, TrashReviewedReplanAuthorization, TrashReviewedRequest,
    WorkspaceContentKind, WorkspaceDocumentGeneration, WorkspaceDraftGateToken,
    WorkspaceDraftRegistryView, WorkspaceItemIcon, WorkspaceItemIconFallback,
    WorkspaceNavigationProjection, WorkspaceNodeProjection, WorkspaceReadScope, WorkspaceRevision,
    WorkspaceTargetResolution, WorkspaceTransactionError, WorkspaceTransactionPlan,
    WorkspaceTrashItemProjection, analyze_citation_authoring_source, analyze_document_for_profile,
    bind_workspace_transaction_target_resolution, canonical_document_locator,
    capture_annotation_sidecar_snapshot, citation_presentation_capabilities, commit_document_edit,
    commit_task_dependency_transaction, commit_task_edit_transaction,
    commit_task_recurrence_transaction, commit_workspace_transaction,
    commit_workspace_transaction_with_draft_gate, confirm_permanent_delete_trash_items,
    derive_workspace_item_icon, has_unfinished_workspace_transaction, plan_annotation_action,
    plan_citation_macro_edit, plan_document_edit,
    plan_migrate_legacy_workspace_trash_at_with_backup, plan_permanently_delete_trash_items,
    plan_restore_trash_item, plan_task_dependency_transaction,
    plan_task_dependency_transaction_scoped, plan_task_edit_transaction,
    plan_task_edit_transaction_scoped, plan_task_recurrence_transaction,
    plan_task_recurrence_transaction_scoped, plan_trash_node_at, plan_trash_resources_at,
    prepare_legacy_trash_migration_backup, present_citations, preview_permanent_delete_trash_items,
    preview_workspace_transaction_draft_gate, project_node_metadata, project_workspace_trash_state,
    read_node_annotations_at_node_path, read_node_document, read_workspace_revision,
    recover_workspace_transaction_for_plan, recover_workspace_transactions,
    replan_reviewed_trash_request, resolve_node_icon_from_source, scan_workspace, search_workspace,
    search_workspace_scoped,
};

use auth::{
    AuditReceipt, CollaborationDocumentRecord, CollaborationReceipt, ControlPlane, IssuedSession,
    MemberRecord, NewCollaborationIntent, NodeAccess, NodeAclRecord, reverse_proxy_token_digest,
};
pub use auth::{AuthError, SessionPolicy, SessionPrincipal, SessionRole};
use collaboration::{
    CollaborationDocument, CollaborationError, DirtyDraftRequest, DocumentStateView,
    OperationRequest, Participant, PresenceRegistry, PresenceRequest, ResyncRequest, TextOperation,
};

const API_PREFIX: &str = "/api/v1";
const CHANGE_CHANNEL_CAPACITY: usize = 128;
const SESSION_COOKIE: &str = "weftext_session";
const CSRF_HEADER: &str = "x-weftext-csrf";
const CSRF_VALUE: &str = "same-origin";
const PROXY_TOKEN_HEADER: &str = "x-weftext-proxy-token";
const TASK_PLAN_TTL_SECONDS: i64 = 10 * 60;
const TRASH_PLAN_TTL_SECONDS: i64 = 10 * 60;
const MAX_PENDING_TASK_PLANS: usize = 64;
const MAX_PENDING_TRASH_PLANS: usize = 64;
const MAX_SERVER_TRASH_ITEMS: usize = 256;
const MAX_PENDING_BACKUP_PLANS: usize = 16;
const MAX_CHANGE_STREAMS: usize = 16;
const COLLABORATION_EVENT_CHANNEL_CAPACITY: usize = 256;
/// A committed node change notification. It is not a collaborative-edit operation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeEvent {
    pub node_id: NodeId,
    pub revision: DocumentRevision,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CollaborationEvent {
    wire_version: &'static str,
    event_type: &'static str,
    node_id: NodeId,
    epoch: u64,
    version: u64,
    revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    actor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    participants: Option<Vec<Participant>>,
}

#[derive(Clone, Debug)]
struct AuthorizationChange {
    actor_scope: String,
}

#[derive(Clone, Debug)]
pub struct HttpSecurityConfig {
    bind_address: SocketAddr,
    allowed_host: String,
    allowed_origin: String,
    secure_cookies: bool,
    boundary: HttpBoundary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HttpBoundary {
    DirectLoopback,
    SameHostReverseProxy,
}

impl HttpSecurityConfig {
    #[must_use]
    pub fn loopback(address: SocketAddr) -> Self {
        Self {
            bind_address: address,
            allowed_host: address.to_string(),
            allowed_origin: format!("http://{address}"),
            secure_cookies: false,
            boundary: HttpBoundary::DirectLoopback,
        }
    }

    /// Configures a fixed HTTPS origin served by a trusted reverse proxy in the
    /// same network namespace as this loopback-only listener.
    ///
    /// # Errors
    ///
    /// Fails unless `address` is loopback and `public_origin` is exactly one
    /// lowercase `https://authority` without credentials, a path, or a query.
    pub fn same_host_reverse_proxy(
        address: SocketAddr,
        public_origin: &str,
    ) -> Result<Self, StartupError> {
        validate_bind_address(address)?;
        let (allowed_host, allowed_origin) = canonical_https_origin(public_origin)?;
        Ok(Self {
            bind_address: address,
            allowed_host,
            allowed_origin,
            secure_cookies: true,
            boundary: HttpBoundary::SameHostReverseProxy,
        })
    }

    #[must_use]
    pub fn with_secure_cookies(mut self, secure: bool) -> Self {
        self.secure_cookies = secure || self.boundary == HttpBoundary::SameHostReverseProxy;
        self
    }

    fn validate(&self) -> Result<(), StartupError> {
        validate_bind_address(self.bind_address)?;
        match self.boundary {
            HttpBoundary::DirectLoopback => Ok(()),
            HttpBoundary::SameHostReverseProxy => {
                let (host, origin) = canonical_https_origin(&self.allowed_origin)?;
                if !self.secure_cookies
                    || host != self.allowed_host
                    || origin != self.allowed_origin
                {
                    return Err(StartupError::InvalidReverseProxyOrigin);
                }
                Ok(())
            }
        }
    }

    fn uses_same_host_reverse_proxy(&self) -> bool {
        self.boundary == HttpBoundary::SameHostReverseProxy
    }
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    control_plane_root: PathBuf,
    http: HttpSecurityConfig,
    session_policy: SessionPolicy,
    bootstrap_attempt_limit: usize,
    login_attempt_limit: usize,
    rate_window: Duration,
    allow_admin_permanent_delete: bool,
    trash_migration_snapshot_parent: Option<PathBuf>,
}

impl ServerConfig {
    #[must_use]
    pub fn new(control_plane_root: impl Into<PathBuf>, http: HttpSecurityConfig) -> Self {
        Self {
            control_plane_root: control_plane_root.into(),
            http,
            session_policy: SessionPolicy::default(),
            bootstrap_attempt_limit: 5,
            login_attempt_limit: 5,
            rate_window: Duration::from_secs(60),
            allow_admin_permanent_delete: false,
            trash_migration_snapshot_parent: None,
        }
    }

    #[must_use]
    pub fn with_session_policy(mut self, policy: SessionPolicy) -> Self {
        self.session_policy = policy;
        self
    }

    #[must_use]
    pub fn with_rate_limits(mut self, bootstrap: usize, login: usize, window: Duration) -> Self {
        self.bootstrap_attempt_limit = bootstrap;
        self.login_attempt_limit = login;
        self.rate_window = window;
        self
    }

    /// Allows configured Admin sessions to cross the same exact permanent-delete boundary as an
    /// Owner. The default remains Owner-only.
    #[must_use]
    pub const fn with_admin_permanent_delete(mut self, enabled: bool) -> Self {
        self.allow_admin_permanent_delete = enabled;
        self
    }

    /// Configures the existing external directory where Core may create exact legacy-Trash
    /// migration snapshots. Core still verifies that it is regular and outside the workspace.
    #[must_use]
    pub fn with_trash_migration_snapshot_parent(mut self, parent: impl Into<PathBuf>) -> Self {
        self.trash_migration_snapshot_parent = Some(parent.into());
        self
    }
}

#[derive(Clone)]
struct RateLimiter {
    attempts: Arc<std::sync::Mutex<Vec<Instant>>>,
    limit: usize,
    window: Duration,
}

impl RateLimiter {
    fn new(limit: usize, window: Duration) -> Self {
        Self {
            attempts: Arc::new(std::sync::Mutex::new(Vec::new())),
            limit,
            window,
        }
    }

    fn allow(&self) -> bool {
        let Ok(mut attempts) = self.attempts.lock() else {
            return false;
        };
        let now = Instant::now();
        attempts.retain(|attempt| now.duration_since(*attempt) < self.window);
        if attempts.len() >= self.limit {
            return false;
        }
        attempts.push(now);
        true
    }
}

/// Shared state for one hosted workspace.
#[derive(Clone)]
pub struct ServerState {
    workspace_root: Arc<PathBuf>,
    workspace_scope: Arc<str>,
    control_plane: ControlPlane,
    control_plane_lease: Arc<ServerControlPlaneLease>,
    http: HttpSecurityConfig,
    proxy_token_digest: Option<Arc<[u8; 32]>>,
    bootstrap_limiter: RateLimiter,
    login_limiter: RateLimiter,
    commits: Arc<Mutex<()>>,
    api_quiescence: Arc<RwLock<()>>,
    task_plans: Arc<Mutex<BTreeMap<String, PendingTaskPlan>>>,
    trash_plans: Arc<Mutex<BTreeMap<String, PendingTrashPlan>>>,
    backup_plans: Arc<Mutex<BTreeMap<String, ServerBackupPairPlan>>>,
    restore_plans: Arc<Mutex<BTreeMap<String, PendingServerRestorePlan>>>,
    change_stream_slots: Arc<Semaphore>,
    changes: broadcast::Sender<ChangeEvent>,
    collaboration_documents: Arc<Mutex<BTreeMap<NodeId, CollaborationDocument>>>,
    presence: Arc<Mutex<PresenceRegistry>>,
    collaboration_events: broadcast::Sender<CollaborationEvent>,
    authorization_changes: broadcast::Sender<AuthorizationChange>,
    shutting_down: Arc<AtomicBool>,
    shutdown: watch::Sender<bool>,
    allow_admin_permanent_delete: bool,
    trash_migration_snapshot_parent: Option<Arc<PathBuf>>,
}

#[derive(Debug)]
struct PendingTaskPlan {
    session_id: String,
    expires_at: i64,
    base_actor_revision: String,
    transaction: PendingTaskTransaction,
}

#[derive(Debug)]
struct PendingTrashPlan {
    session_id: String,
    expires_at: i64,
    base_actor_revision: String,
    reviewed_request: TrashReviewedRequest,
    draft_gate_token: WorkspaceDraftGateToken,
    authorization: PendingTrashAuthorization,
}

#[derive(Debug)]
enum PendingTrashAuthorization {
    Ordinary,
    LegacyMigration(Box<weftext_core::LegacyTrashMigrationBackup>),
    PermanentDelete,
}

#[derive(Debug)]
enum PendingTaskTransaction {
    Edit(Box<TaskEditTransactionPlan>),
    Recurrence(Box<TaskRecurrenceTransactionPlan>),
    Dependencies(Box<TaskDependencyTransactionPlan>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ServerRestorePurpose {
    AlternateRestore,
    RestoreDrill,
}

#[derive(Clone, Debug)]
struct PendingServerRestorePlan {
    purpose: ServerRestorePurpose,
    plan: ServerRestorePairPlan,
}

impl ServerState {
    /// Opens one valid hosted workspace and fixes its resolved root for the process lifetime.
    ///
    /// # Errors
    ///
    /// Fails closed for a missing, linked, or invalid workspace.
    pub fn open(
        workspace_root: impl AsRef<Path>,
        config: ServerConfig,
    ) -> Result<Self, StartupError> {
        let requested = workspace_root.as_ref();
        config.http.validate()?;
        let metadata = fs::symlink_metadata(requested).map_err(StartupError::WorkspaceIo)?;
        if metadata_is_link_or_reparse(&metadata) {
            return Err(StartupError::LinkedWorkspace);
        }
        let resolved = fs::canonicalize(requested).map_err(StartupError::WorkspaceIo)?;
        recover_workspace_transactions(&resolved).map_err(StartupError::WorkspaceRecovery)?;
        let inventory = scan_workspace(&resolved);
        let trash_state = project_workspace_trash_state(&resolved);
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 || trash_state.is_err() {
            return Err(StartupError::InvalidWorkspace(trash_state.map_or_else(
                |error| error.to_string(),
                |_| "workspace is not canonical AsciiDoc v1".to_owned(),
            )));
        }
        let prepared_control_plane = ControlPlane::prepare(&resolved, &config.control_plane_root)
            .map_err(StartupError::ControlPlane)?;
        let control_plane_lease = acquire_server_control_plane_lease(prepared_control_plane.root())
            .map_err(startup_control_plane_lease_error)?;
        if control_plane_lease.root() != prepared_control_plane.root() {
            return Err(StartupError::ControlPlaneLease(
                ServerControlPlaneBackupError::InvalidControlPlane(
                    "exclusive lease resolved a different control-plane root".to_owned(),
                ),
            ));
        }
        let control_plane =
            ControlPlane::open_prepared(prepared_control_plane, config.session_policy)
                .map_err(StartupError::ControlPlane)?;
        let workspace_scope = control_plane
            .workspace_scope()
            .map_err(StartupError::ControlPlane)?;
        let proxy_token_digest = config
            .http
            .uses_same_host_reverse_proxy()
            .then(|| control_plane.provision_reverse_proxy_secret())
            .transpose()
            .map_err(StartupError::ControlPlane)?
            .map(Arc::new);
        let (changes, _) = broadcast::channel(CHANGE_CHANNEL_CAPACITY);
        let (collaboration_events, _) = broadcast::channel(COLLABORATION_EVENT_CHANNEL_CAPACITY);
        let (authorization_changes, _) = broadcast::channel(CHANGE_CHANNEL_CAPACITY);
        let (shutdown, _) = watch::channel(false);
        let state = Self {
            workspace_scope: Arc::from(workspace_scope),
            workspace_root: Arc::new(resolved),
            control_plane,
            control_plane_lease: Arc::new(control_plane_lease),
            http: config.http,
            proxy_token_digest,
            bootstrap_limiter: RateLimiter::new(config.bootstrap_attempt_limit, config.rate_window),
            login_limiter: RateLimiter::new(config.login_attempt_limit, config.rate_window),
            commits: Arc::new(Mutex::new(())),
            api_quiescence: Arc::new(RwLock::new(())),
            task_plans: Arc::new(Mutex::new(BTreeMap::new())),
            trash_plans: Arc::new(Mutex::new(BTreeMap::new())),
            backup_plans: Arc::new(Mutex::new(BTreeMap::new())),
            restore_plans: Arc::new(Mutex::new(BTreeMap::new())),
            change_stream_slots: Arc::new(Semaphore::new(MAX_CHANGE_STREAMS)),
            changes,
            collaboration_documents: Arc::new(Mutex::new(BTreeMap::new())),
            presence: Arc::new(Mutex::new(PresenceRegistry::default())),
            collaboration_events,
            authorization_changes,
            shutting_down: Arc::new(AtomicBool::new(false)),
            shutdown,
            allow_admin_permanent_delete: config.allow_admin_permanent_delete,
            trash_migration_snapshot_parent: config.trash_migration_snapshot_parent.map(Arc::new),
        };
        state
            .recover_pending_audit_intents()
            .map_err(StartupError::ControlPlane)?;
        Ok(state)
    }

    /// Returns the fixed hosted workspace root for process-level orchestration only.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    #[must_use]
    pub fn control_plane_root(&self) -> &Path {
        self.control_plane.root()
    }

    #[must_use]
    pub fn control_plane_database_path(&self) -> &Path {
        self.control_plane.database_path()
    }

    #[must_use]
    pub fn bootstrap_secret_path(&self) -> &Path {
        self.control_plane.bootstrap_secret_path()
    }

    /// Returns the protected proxy-token file only for strict same-host proxy mode.
    #[must_use]
    pub fn reverse_proxy_secret_path(&self) -> Option<&Path> {
        self.http
            .uses_same_host_reverse_proxy()
            .then(|| self.control_plane.reverse_proxy_secret_path())
    }

    /// Marks the runtime unready and closes long-lived change streams.
    pub fn begin_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Release);
        self.shutdown.send_replace(true);
    }

    fn is_shutting_down(&self) -> bool {
        self.shutting_down.load(Ordering::Acquire)
    }

    fn workspace_scope(&self) -> &str {
        &self.workspace_scope
    }

    fn notify_authorization_change(&self, actor_scope: impl Into<String>) {
        let _ = self.authorization_changes.send(AuthorizationChange {
            actor_scope: actor_scope.into(),
        });
    }

    fn node_path(&self, id: NodeId) -> Result<PathBuf, ApiError> {
        let inventory = scan_workspace(self.workspace_root());
        let mut matches = inventory
            .nodes
            .iter()
            .filter(|node| node.id == Some(id))
            .map(|node| node.path.clone());
        let path = matches.next().ok_or(ApiError::NodeNotFound)?;
        if matches.next().is_some() {
            return Err(ApiError::NodeNotFound);
        }
        Ok(path)
    }

    fn node_access(
        &self,
        principal: &SessionPrincipal,
        id: NodeId,
    ) -> Result<NodeAccess, ApiError> {
        let inventory = scan_workspace(self.workspace_root());
        let mut matches = inventory.nodes.iter().filter(|node| node.id == Some(id));
        let node = matches.next().ok_or(ApiError::NodeNotFound)?;
        if matches.next().is_some() {
            return Err(ApiError::NodeNotFound);
        }
        let (ancestry, allow_role_default) = physical_ancestry_ids(&inventory, &node.path);
        self.control_plane
            .effective_node_access(principal, &ancestry, allow_role_default)
            .map_err(ApiError::ControlPlane)
    }

    fn authorized_node_path(
        &self,
        principal: &SessionPrincipal,
        id: NodeId,
        write: bool,
    ) -> Result<PathBuf, ApiError> {
        self.require_current_principal(principal)?;
        match self.node_access(principal, id)? {
            NodeAccess::Hidden => Err(ApiError::NodeNotFound),
            NodeAccess::Read if write => Err(ApiError::AuthorizationDenied),
            NodeAccess::Read | NodeAccess::Write => self.node_path(id),
        }
    }

    fn require_current_principal(&self, principal: &SessionPrincipal) -> Result<(), ApiError> {
        if self
            .control_plane
            .session_is_current(principal, unix_now()?)
            .map_err(ApiError::ControlPlane)?
        {
            Ok(())
        } else {
            Err(ApiError::AuthenticationRequired)
        }
    }

    fn recover_pending_audit_intents(&self) -> Result<(), AuthError> {
        for intent in self.control_plane.pending_audit_intents()? {
            let authority_confirmed = match intent.authority_kind.as_str() {
                "document" => NodeId::from_str(&intent.target)
                    .ok()
                    .and_then(|node_id| self.node_path(node_id).ok())
                    .and_then(|path| read_node_document(path).ok())
                    .is_some_and(|snapshot| {
                        snapshot.revision.to_string() == intent.expected_revision
                    }),
                "workspace" => {
                    !intent.expected_revision.starts_with("not:")
                        && read_workspace_revision(self.workspace_root())
                            .is_ok_and(|revision| revision.to_string() == intent.expected_revision)
                }
                _ => false,
            };
            self.control_plane.recover_audit_intent(
                &intent.intent_id,
                authority_confirmed,
                unix_now().unwrap_or(intent.created_at),
            )?;
        }
        Ok(())
    }
}

fn metadata_is_link_or_reparse(metadata: &fs::Metadata) -> bool {
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

/// Builds the same-origin `WebUI` and versioned API router.
#[allow(clippy::too_many_lines)]
pub fn app(state: ServerState) -> Router {
    Router::new()
        .route("/", get(webui_index))
        .route("/app.js", get(webui_app))
        .route("/api.js", get(webui_api))
        .route("/navigation.js", get(webui_navigation))
        .route("/style.css", get(webui_style))
        .route(&format!("{API_PREFIX}/health"), get(health))
        .route(&format!("{API_PREFIX}/health/live"), get(health))
        .route(&format!("{API_PREFIX}/health/ready"), get(readiness))
        .route(&format!("{API_PREFIX}/capabilities"), get(capabilities))
        .route(
            &format!("{API_PREFIX}/auth/bootstrap"),
            post(bootstrap_owner),
        )
        .route(&format!("{API_PREFIX}/auth/login"), post(login_owner))
        .route(&format!("{API_PREFIX}/auth/session"), get(current_session))
        .route(&format!("{API_PREFIX}/auth/logout"), post(logout_owner))
        .route(
            &format!("{API_PREFIX}/auth/revoke-all"),
            post(revoke_all_sessions),
        )
        .route(
            &format!("{API_PREFIX}/admin/members"),
            get(list_members).post(create_member),
        )
        .route(
            &format!("{API_PREFIX}/admin/members/{{actor_scope}}"),
            axum::routing::put(update_member),
        )
        .route(
            &format!("{API_PREFIX}/admin/node-acl"),
            get(list_node_acl).put(set_node_acl),
        )
        .route(
            &format!("{API_PREFIX}/admin/audit"),
            get(export_audit_receipts),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/capabilities"),
            get(server_backup_capabilities),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/preview"),
            post(preview_server_backup),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/commit"),
            post(commit_server_backup),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/verify"),
            post(verify_server_backup),
        )
        .route(
            &format!("{API_PREFIX}/admin/restore/preview"),
            post(preview_server_restore),
        )
        .route(
            &format!("{API_PREFIX}/admin/restore/commit"),
            post(commit_server_restore),
        )
        .route(
            &format!("{API_PREFIX}/admin/restore/verify"),
            post(verify_server_restore),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/drill/preview"),
            post(preview_server_restore_drill),
        )
        .route(
            &format!("{API_PREFIX}/admin/backup/drill/commit"),
            post(commit_server_restore_drill),
        )
        .route(&format!("{API_PREFIX}/workspace"), get(inventory))
        .route(&format!("{API_PREFIX}/trash"), get(trash_inventory))
        .route(
            &format!("{API_PREFIX}/trash/nodes/{{node_id}}/preview"),
            post(preview_trash_node),
        )
        .route(
            &format!("{API_PREFIX}/trash/resources/preview"),
            post(preview_trash_resources),
        )
        .route(
            &format!("{API_PREFIX}/trash/items/{{trash_item_id}}/restore/preview"),
            post(preview_trash_restore),
        )
        .route(
            &format!("{API_PREFIX}/trash/permanent-delete/preview"),
            post(preview_trash_permanent_delete),
        )
        .route(
            &format!("{API_PREFIX}/trash/migrate-legacy/preview"),
            post(preview_legacy_trash_migration),
        )
        .route(
            &format!("{API_PREFIX}/trash/transactions/{{plan_id}}/commit"),
            post(commit_trash_transaction),
        )
        .route(
            &format!("{API_PREFIX}/documents/{{node_id}}"),
            get(open_document).put(commit_document),
        )
        .route(
            &format!("{API_PREFIX}/documents/{{node_id}}/preview"),
            post(preview_document),
        )
        .route(
            &format!("{API_PREFIX}/annotations/{{node_id}}"),
            get(read_annotations).post(commit_annotation_action),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}"),
            get(collaboration_snapshot),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}/operations"),
            post(commit_collaboration_operation),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}/drafts"),
            post(commit_collaboration_draft),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}/presence"),
            post(update_collaboration_presence),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}/presence/{{client_id}}"),
            axum::routing::delete(leave_collaboration_presence),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/documents/{{node_id}}/resync"),
            post(acknowledge_collaboration_resync),
        )
        .route(
            &format!("{API_PREFIX}/collaboration/events"),
            get(collaboration_event_stream),
        )
        .route(&format!("{API_PREFIX}/search"), get(search))
        .route(
            &format!("{API_PREFIX}/citations/capabilities"),
            get(citation_capabilities),
        )
        .route(
            &format!("{API_PREFIX}/citations/validate"),
            get(validate_citations),
        )
        .route(
            &format!("{API_PREFIX}/citations/references"),
            get(search_citation_references),
        )
        .route(
            &format!("{API_PREFIX}/citations/{{node_id}}/analyze"),
            post(analyze_citation_draft),
        )
        .route(
            &format!("{API_PREFIX}/citations/{{node_id}}/macros/preview"),
            post(preview_citation_macro_edit),
        )
        .route(
            &format!("{API_PREFIX}/citations/{{node_id}}/macros/commit"),
            post(commit_citation_macro_edit),
        )
        .route(
            &format!("{API_PREFIX}/queries/execute"),
            post(execute_query),
        )
        .route(&format!("{API_PREFIX}/tasks/validate"), get(validate_tasks))
        .route(
            &format!("{API_PREFIX}/tasks/nodes/{{node_id}}"),
            get(inspect_node_tasks),
        )
        .route(
            &format!("{API_PREFIX}/tasks/nodes/{{node_id}}/edit/preview"),
            post(preview_task_edit),
        )
        .route(
            &format!("{API_PREFIX}/tasks/nodes/{{node_id}}/recurrence/preview"),
            post(preview_task_recurrence),
        )
        .route(
            &format!("{API_PREFIX}/tasks/nodes/{{node_id}}/dependencies/preview"),
            post(preview_task_dependencies),
        )
        .route(
            &format!("{API_PREFIX}/tasks/transactions/{{plan_id}}/commit"),
            post(commit_task_transaction),
        )
        .route(
            &format!("{API_PREFIX}/tasks/recover"),
            post(recover_task_transactions),
        )
        .route(&format!("{API_PREFIX}/changes"), get(changes))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(not_found)
        .layer(middleware::from_fn_with_state(state.clone(), security_gate))
        .with_state(state)
}

/// Rejects every non-loopback bind, including strict same-host proxy mode.
///
/// # Errors
///
/// Returns a safety error for a non-loopback address.
pub fn validate_bind_address(address: SocketAddr) -> Result<SocketAddr, StartupError> {
    if address.ip().is_loopback() {
        Ok(address)
    } else {
        Err(StartupError::AuthenticationRequiredForNonLoopback(
            address.ip(),
        ))
    }
}

fn canonical_https_origin(value: &str) -> Result<(String, String), StartupError> {
    let authority_text = value
        .strip_prefix("https://")
        .ok_or(StartupError::InvalidReverseProxyOrigin)?;
    if authority_text.is_empty()
        || authority_text.len() > 512
        || authority_text.contains(['/', '?', '#', '@'])
        || authority_text
            .bytes()
            .any(|byte| byte.is_ascii_whitespace())
        || authority_text != authority_text.to_ascii_lowercase()
    {
        return Err(StartupError::InvalidReverseProxyOrigin);
    }
    let authority =
        Authority::from_str(authority_text).map_err(|_| StartupError::InvalidReverseProxyOrigin)?;
    if authority.host().is_empty() || authority.port_u16() == Some(0) {
        return Err(StartupError::InvalidReverseProxyOrigin);
    }
    let host = authority.to_string();
    let origin = format!("https://{host}");
    if origin != value {
        return Err(StartupError::InvalidReverseProxyOrigin);
    }
    Ok((host, origin))
}

#[derive(Debug)]
pub enum StartupError {
    AuthenticationRequiredForNonLoopback(IpAddr),
    InvalidReverseProxyOrigin,
    WorkspaceIo(std::io::Error),
    LinkedWorkspace,
    WorkspaceRecovery(WorkspaceTransactionError),
    InvalidWorkspace(String),
    ControlPlane(AuthError),
    ControlPlaneInUse(PathBuf),
    ControlPlaneLease(ServerControlPlaneBackupError),
}

fn startup_control_plane_lease_error(error: ServerControlPlaneBackupError) -> StartupError {
    match error {
        ServerControlPlaneBackupError::ControlPlaneInUse(path) => {
            StartupError::ControlPlaneInUse(path)
        }
        error => StartupError::ControlPlaneLease(error),
    }
}

impl fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AuthenticationRequiredForNonLoopback(address) => write!(
                formatter,
                "AUTHENTICATION_REQUIRED_FOR_NON_LOOPBACK: the local-account Server is not deployment-ready and refuses {address}"
            ),
            Self::InvalidReverseProxyOrigin => formatter.write_str(
                "invalid same-host proxy origin: expected exact lowercase https://authority",
            ),
            Self::WorkspaceIo(error) => write!(formatter, "cannot open hosted workspace: {error}"),
            Self::LinkedWorkspace => formatter.write_str("hosted workspace root cannot be a link"),
            Self::WorkspaceRecovery(error) => {
                write!(
                    formatter,
                    "cannot recover hosted workspace transaction: {error}"
                )
            }
            Self::InvalidWorkspace(message) => {
                write!(formatter, "invalid hosted workspace: {message}")
            }
            Self::ControlPlane(error) => {
                write!(formatter, "cannot open Server control plane: {error}")
            }
            Self::ControlPlaneInUse(path) => write!(
                formatter,
                "Server control plane is in use; exclusive lease unavailable: {}",
                path.display()
            ),
            Self::ControlPlaneLease(error) => {
                write!(
                    formatter,
                    "cannot acquire Server control-plane lease: {error}"
                )
            }
        }
    }
}

impl std::error::Error for StartupError {}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
    api_version: &'static str,
    stage: &'static str,
    runtime_boundary: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        api_version: "v1",
        stage: "canonical-asciidoc-multirole-acl",
        runtime_boundary: "process_live",
    })
}

async fn readiness(State(state): State<ServerState>) -> Response {
    let ready = if state.is_shutting_down() {
        false
    } else if let Ok(_commit_guard) = state.commits.try_lock() {
        state.control_plane.readiness_check().unwrap_or(false)
            && read_workspace_revision(state.workspace_root()).is_ok()
            && !has_unfinished_workspace_transaction(state.workspace_root()).unwrap_or(true)
            && state.reverse_proxy_secret_path().is_none_or(|path| {
                fs::symlink_metadata(path).is_ok_and(|metadata| {
                    metadata.is_file() && !metadata_is_link_or_reparse(&metadata)
                })
            })
    } else {
        false
    };
    let status = if ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(HealthResponse {
            status: if ready { "ready" } else { "not_ready" },
            api_version: "v1",
            stage: "canonical-asciidoc-multirole-acl",
            runtime_boundary: if state.http.uses_same_host_reverse_proxy() {
                "same_host_tls_reverse_proxy"
            } else {
                "direct_loopback"
            },
        }),
    )
        .into_response()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitiesResponse {
    api_version: &'static str,
    hosted_workspace_count: u8,
    features: [&'static str; 17],
    managed_document_profile: &'static str,
    reference_record_writes: &'static str,
    change_subscription: &'static str,
    collaboration_protocol: &'static str,
    collaboration_history_limit: usize,
    authentication: &'static str,
    authorization: &'static str,
    runtime_boundary: &'static str,
    forwarded_header_policy: &'static str,
    loopback_only: bool,
    deployment_ready: bool,
    public_internet: &'static str,
    realtime_collaboration: bool,
    role_capabilities: RoleCapabilityMap,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the wire contract exposes independent effective capability decisions"
)]
struct RoleCapabilities {
    read_visible_content: bool,
    edit_documents: bool,
    mutate_structure: bool,
    write_annotations: bool,
    permanently_delete: bool,
    manage_members: bool,
    manage_workspace: bool,
}

impl RoleCapabilities {
    const fn for_role(role: SessionRole, allow_admin_permanent_delete: bool) -> Self {
        Self {
            read_visible_content: true,
            edit_documents: role.can_write_content(),
            mutate_structure: role.can_mutate_structure(),
            write_annotations: role.can_write_annotations(),
            permanently_delete: role.can_permanently_delete()
                || (allow_admin_permanent_delete && matches!(role, SessionRole::Admin)),
            manage_members: role.can_manage_members(),
            manage_workspace: role.can_manage_workspace(),
        }
    }
}

#[derive(Serialize)]
struct RoleCapabilityMap {
    owner: RoleCapabilities,
    admin: RoleCapabilities,
    editor: RoleCapabilities,
    commenter: RoleCapabilities,
    viewer: RoleCapabilities,
}

impl RoleCapabilityMap {
    const fn all(allow_admin_permanent_delete: bool) -> Self {
        Self {
            owner: RoleCapabilities::for_role(SessionRole::Owner, allow_admin_permanent_delete),
            admin: RoleCapabilities::for_role(SessionRole::Admin, allow_admin_permanent_delete),
            editor: RoleCapabilities::for_role(SessionRole::Editor, allow_admin_permanent_delete),
            commenter: RoleCapabilities::for_role(
                SessionRole::Commenter,
                allow_admin_permanent_delete,
            ),
            viewer: RoleCapabilities::for_role(SessionRole::Viewer, allow_admin_permanent_delete),
        }
    }
}

async fn capabilities(State(state): State<ServerState>) -> Json<CapabilitiesResponse> {
    Json(CapabilitiesResponse {
        api_version: "v1",
        hosted_workspace_count: 1,
        features: [
            "workspace_inventory",
            "workspace_content_boundary",
            "document_read",
            "document_preview",
            "revision_checked_commit",
            "search",
            "multi_role_authentication",
            "revocable_sessions",
            "citation_occurrence_analysis",
            "query_execution",
            "annotation_read_v3",
            "annotation_write_v3",
            "local_members",
            "inherited_node_acl",
            "durable_audit_receipts",
            "collaboration_linearized_v1_preview",
            "workspace_trash_items",
        ],
        managed_document_profile: weftext_core::MANAGED_DOCUMENT_PROFILE_ID,
        reference_record_writes: "retired_until_typed_citation_data",
        change_subscription: "server_sent_events",
        collaboration_protocol: collaboration::WIRE_VERSION,
        collaboration_history_limit: collaboration::HISTORY_LIMIT,
        authentication: "local_account_session",
        authorization: "workspace_role_and_inherited_node_acl",
        runtime_boundary: if state.http.uses_same_host_reverse_proxy() {
            "same_host_tls_reverse_proxy"
        } else {
            "direct_loopback"
        },
        forwarded_header_policy: "rejected_not_trusted",
        loopback_only: true,
        deployment_ready: false,
        public_internet: "unsupported",
        realtime_collaboration: false,
        role_capabilities: RoleCapabilityMap::all(state.allow_admin_permanent_delete),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BootstrapRequest {
    bootstrap_secret: String,
    password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoginRequest {
    login: String,
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    authenticated: bool,
    role: &'static str,
    actor_scope: String,
    session_id: String,
    absolute_expires_at: i64,
    idle_expires_at: i64,
    capabilities: RoleCapabilities,
}

async fn bootstrap_owner(
    State(state): State<ServerState>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<BootstrapRequest>,
) -> Result<Response, ApiError> {
    if !state.bootstrap_limiter.allow() {
        return Err(ApiError::RateLimited);
    }
    let prior = session_token(&headers);
    let now = unix_now()?;
    let prior_actor = prior.as_deref().and_then(|token| {
        state
            .control_plane
            .validate_session(token, now)
            .ok()
            .map(|principal| principal.actor_scope)
    });
    let issued = state
        .control_plane
        .bootstrap(
            &request.bootstrap_secret,
            &request.password,
            prior.as_deref(),
            now,
        )
        .map_err(ApiError::from_bootstrap)?;
    if let Some(actor_scope) = prior_actor {
        state.notify_authorization_change(actor_scope);
    }
    Ok(session_response(&state, &issued))
}

async fn login_owner(
    State(state): State<ServerState>,
    headers: HeaderMap,
    ApiJson(request): ApiJson<LoginRequest>,
) -> Result<Response, ApiError> {
    if !state.login_limiter.allow() {
        return Err(ApiError::RateLimited);
    }
    let prior = session_token(&headers);
    let now = unix_now()?;
    let prior_actor = prior.as_deref().and_then(|token| {
        state
            .control_plane
            .validate_session(token, now)
            .ok()
            .map(|principal| principal.actor_scope)
    });
    let issued = state
        .control_plane
        .login(&request.login, &request.password, prior.as_deref(), now)
        .map_err(ApiError::from_login)?;
    if let Some(actor_scope) = prior_actor {
        state.notify_authorization_change(actor_scope);
    }
    Ok(session_response(&state, &issued))
}

async fn current_session(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Json<SessionResponse> {
    Json(principal_response(&state, &principal))
}

async fn logout_owner(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Response, ApiError> {
    state
        .control_plane
        .logout(&principal, unix_now()?)
        .map_err(ApiError::ControlPlane)?;
    state.notify_authorization_change(principal.actor_scope);
    Ok(cleared_session_response(&state))
}

async fn revoke_all_sessions(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Response, ApiError> {
    state
        .control_plane
        .revoke_all(&principal, unix_now()?)
        .map_err(ApiError::ControlPlane)?;
    state.notify_authorization_change(principal.actor_scope);
    Ok(cleared_session_response(&state))
}

fn principal_response(state: &ServerState, principal: &SessionPrincipal) -> SessionResponse {
    SessionResponse {
        authenticated: true,
        role: principal.role.as_str(),
        actor_scope: principal.actor_scope.clone(),
        session_id: principal.session_id.clone(),
        absolute_expires_at: principal.absolute_expires_at,
        idle_expires_at: principal.idle_expires_at,
        capabilities: RoleCapabilities::for_role(
            principal.role,
            state.allow_admin_permanent_delete,
        ),
    }
}

fn session_response(state: &ServerState, issued: &IssuedSession) -> Response {
    let mut response = Json(principal_response(state, &issued.principal)).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(
            &issued.token,
            state.http.secure_cookies,
            state.control_plane.session_policy().absolute_seconds,
        ),
    );
    response
}

fn cleared_session_response(state: &ServerState) -> Response {
    let mut response = Json(serde_json::json!({ "authenticated": false })).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        clear_session_cookie(state.http.secure_cookies),
    );
    response
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MemberResponse {
    actor_scope: String,
    login: String,
    role: &'static str,
    enabled: bool,
    created_at: i64,
}

impl From<MemberRecord> for MemberResponse {
    fn from(value: MemberRecord) -> Self {
        Self {
            actor_scope: value.actor_scope,
            login: value.login,
            role: value.role.as_str(),
            enabled: value.enabled,
            created_at: value.created_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateMemberRequest {
    login: String,
    password: String,
    role: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UpdateMemberRequest {
    role: String,
    enabled: bool,
}

async fn list_members(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    require_member_admin(&state, &principal)?;
    let members = state
        .control_plane
        .list_members()
        .map_err(ApiError::ControlPlane)?
        .into_iter()
        .map(MemberResponse::from)
        .collect();
    Ok(Json(members))
}

async fn create_member(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<CreateMemberRequest>,
) -> Result<Json<MemberResponse>, ApiError> {
    require_member_admin(&state, &principal)?;
    let role = SessionRole::parse(&request.role).map_err(map_member_error)?;
    let member = state
        .control_plane
        .create_member(
            &principal,
            &request.login,
            &request.password,
            role,
            unix_now()?,
        )
        .map_err(map_member_error)?;
    Ok(Json(member.into()))
}

async fn update_member(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(actor_scope): ApiPath<String>,
    ApiJson(request): ApiJson<UpdateMemberRequest>,
) -> Result<Json<MemberResponse>, ApiError> {
    require_member_admin(&state, &principal)?;
    let role = SessionRole::parse(&request.role).map_err(map_member_error)?;
    let member = state
        .control_plane
        .update_member(&principal, &actor_scope, role, request.enabled, unix_now()?)
        .map_err(map_member_error)?;
    state.notify_authorization_change(actor_scope);
    Ok(Json(member.into()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NodeAclResponse {
    actor_scope: String,
    node_id: String,
    access: &'static str,
    updated_at: i64,
    updated_by: String,
}

impl From<NodeAclRecord> for NodeAclResponse {
    fn from(value: NodeAclRecord) -> Self {
        Self {
            actor_scope: value.actor_scope,
            node_id: value.node_id,
            access: value.access.as_str(),
            updated_at: value.updated_at,
            updated_by: value.updated_by,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SetNodeAclRequest {
    actor_scope: String,
    node_id: String,
    access: Option<String>,
}

async fn list_node_acl(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<Vec<NodeAclResponse>>, ApiError> {
    require_member_admin(&state, &principal)?;
    let entries = state
        .control_plane
        .list_node_acl()
        .map_err(ApiError::ControlPlane)?
        .into_iter()
        .map(NodeAclResponse::from)
        .collect();
    Ok(Json(entries))
}

async fn set_node_acl(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<SetNodeAclRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_member_admin(&state, &principal)?;
    let node_id = parse_node_id(&request.node_id)?;
    state.node_path(node_id)?;
    let access = request
        .access
        .as_deref()
        .map(NodeAccess::parse)
        .transpose()
        .map_err(map_member_error)?;
    state
        .control_plane
        .set_node_acl(
            &principal,
            &request.actor_scope,
            &node_id.to_string(),
            access,
            unix_now()?,
        )
        .map_err(map_member_error)?;
    state.notify_authorization_change(request.actor_scope.clone());
    Ok(Json(serde_json::json!({
        "actorScope": request.actor_scope,
        "nodeId": node_id,
        "access": access.map(NodeAccess::as_str),
    })))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditReceiptResponse {
    receipt_id: i64,
    occurred_at: i64,
    event_type: String,
    actor_scope: String,
    detail: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuditReceiptQuery {
    after_receipt_id: Option<i64>,
    event_type: Option<String>,
    actor_scope: Option<String>,
    limit: Option<usize>,
}

impl From<AuditReceipt> for AuditReceiptResponse {
    fn from(value: AuditReceipt) -> Self {
        Self {
            receipt_id: value.receipt_id,
            occurred_at: value.occurred_at,
            event_type: value.event_type,
            actor_scope: value.actor_scope,
            detail: value.detail,
        }
    }
}

async fn export_audit_receipts(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiQuery(query): ApiQuery<AuditReceiptQuery>,
) -> Result<Json<Vec<AuditReceiptResponse>>, ApiError> {
    require_member_admin(&state, &principal)?;
    if query.after_receipt_id.is_some_and(|value| value < 0) {
        return Err(ApiError::InvalidRequest(
            "afterReceiptId must be zero or greater",
        ));
    }
    if query.limit.is_some_and(|value| value == 0 || value > 1_000) {
        return Err(ApiError::InvalidRequest("limit must be between 1 and 1000"));
    }
    if query
        .event_type
        .as_ref()
        .is_some_and(|value| value.len() > 128)
        || query
            .actor_scope
            .as_ref()
            .is_some_and(|value| value.len() > 128)
    {
        return Err(ApiError::InvalidRequest("audit filter is too long"));
    }
    let after_receipt_id = query.after_receipt_id.unwrap_or_default();
    let limit = query.limit.unwrap_or(usize::MAX);
    let receipts = state
        .control_plane
        .audit_receipts()
        .map_err(ApiError::ControlPlane)?
        .into_iter()
        .filter(|receipt| receipt.receipt_id > after_receipt_id)
        .filter(|receipt| {
            query
                .event_type
                .as_ref()
                .is_none_or(|event_type| receipt.event_type == *event_type)
        })
        .filter(|receipt| {
            query
                .actor_scope
                .as_ref()
                .is_none_or(|actor_scope| receipt.actor_scope == *actor_scope)
        })
        .take(limit)
        .map(AuditReceiptResponse::from)
        .collect();
    Ok(Json(receipts))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupPreviewRequest {
    backup_parent: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupPlanCommitRequest {
    plan_digest: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerBackupVerifyRequest {
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestorePreviewRequest {
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    restored_workspace_root: PathBuf,
    restored_control_plane_root: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestoreVerifyRequest {
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    restored_workspace_root: PathBuf,
    restored_control_plane_root: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ServerRestoreDrillPreviewRequest {
    workspace_snapshot_directory: PathBuf,
    control_plane_snapshot_directory: PathBuf,
    drill_workspace_root: PathBuf,
    drill_control_plane_root: PathBuf,
}

async fn server_backup_capabilities(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    Ok(Json(serde_json::json!({
        "schema": "weftext.server-backup-capabilities.v1",
        "ownerOnly": true,
        "fullWorkspaceAndControlPlanePair": true,
        "exclusiveLease": true,
        "apiQuiescence": true,
        "alternateCleanRestore": true,
        "restoreDrill": true,
        "sessionRestorePolicy": "invalidate_all",
        "reverseProxySecretRestoreAction": "regenerate_and_rotate_at_first_server_start",
        "managedShape": "X/X.adoc",
        "annotations": "node_local_weftext.annotations.json",
    })))
}

async fn preview_server_backup(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerBackupPreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    let plan = plan_server_backup_pair_with_lease(
        state.workspace_root(),
        &state.control_plane_lease,
        request.backup_parent,
    )
    .map_err(ApiError::Backup)?;
    let payload = serde_json::to_value(&plan).map_err(|_| ApiError::ControlPlaneUnavailable)?;
    let mut pending = state.backup_plans.lock().await;
    if pending.len() >= MAX_PENDING_BACKUP_PLANS {
        return Err(ApiError::BackupPlanLimit);
    }
    pending.insert(plan.plan_digest.clone(), plan);
    Ok(Json(serde_json::json!({
        "stage": "preview",
        "plan": payload,
        "quiesced": true,
    })))
}

async fn commit_server_backup(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerBackupPlanCommitRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    let plan = state
        .backup_plans
        .lock()
        .await
        .get(&request.plan_digest)
        .cloned()
        .ok_or(ApiError::BackupPlanUnavailable)?;
    let receipt = commit_server_backup_pair_with_lease(&state.control_plane_lease, &plan)
        .map_err(ApiError::Backup)?;
    record_server_operation_audit(
        &state,
        &principal,
        "server_backup_completed",
        &format!(
            "planDigest={};workspaceSnapshotId={};controlPlaneBackupId={};exactPair={}",
            plan.plan_digest,
            plan.workspace_snapshot_id,
            plan.control_plane_backup_id,
            receipt.verification.exact_pair,
        ),
    )?;
    state.backup_plans.lock().await.remove(&request.plan_digest);
    Ok(Json(serde_json::json!({
        "stage": "committed",
        "receipt": receipt,
        "quiesced": true,
        "auditRecorded": true,
    })))
}

async fn verify_server_backup(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerBackupVerifyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    let verification = verify_server_backup_pair(
        request.workspace_snapshot_directory,
        request.control_plane_snapshot_directory,
    )
    .map_err(ApiError::Backup)?;
    Ok(Json(serde_json::json!({
        "stage": "verified",
        "verification": verification,
    })))
}

async fn preview_server_restore(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerRestorePreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    preview_server_restore_for_purpose(
        &state,
        &principal,
        request,
        ServerRestorePurpose::AlternateRestore,
    )
    .await
}

async fn preview_server_restore_drill(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerRestoreDrillPreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    preview_server_restore_for_purpose(
        &state,
        &principal,
        ServerRestorePreviewRequest {
            workspace_snapshot_directory: request.workspace_snapshot_directory,
            control_plane_snapshot_directory: request.control_plane_snapshot_directory,
            restored_workspace_root: request.drill_workspace_root,
            restored_control_plane_root: request.drill_control_plane_root,
        },
        ServerRestorePurpose::RestoreDrill,
    )
    .await
}

async fn preview_server_restore_for_purpose(
    state: &ServerState,
    principal: &SessionPrincipal,
    request: ServerRestorePreviewRequest,
    purpose: ServerRestorePurpose,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(state, principal)?;
    let plan = plan_server_restore_pair(
        request.workspace_snapshot_directory,
        request.control_plane_snapshot_directory,
        request.restored_workspace_root,
        request.restored_control_plane_root,
    )
    .map_err(ApiError::Backup)?;
    let payload = serde_json::to_value(&plan).map_err(|_| ApiError::ControlPlaneUnavailable)?;
    let mut pending = state.restore_plans.lock().await;
    if pending.len() >= MAX_PENDING_BACKUP_PLANS {
        return Err(ApiError::BackupPlanLimit);
    }
    pending.insert(
        plan.plan_digest.clone(),
        PendingServerRestorePlan { purpose, plan },
    );
    Ok(Json(serde_json::json!({
        "stage": match purpose {
            ServerRestorePurpose::AlternateRestore => "restore_preview",
            ServerRestorePurpose::RestoreDrill => "drill_preview",
        },
        "plan": payload,
        "cleanTargetsRequired": true,
    })))
}

async fn commit_server_restore(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerBackupPlanCommitRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    commit_server_restore_for_purpose(
        &state,
        &principal,
        request,
        ServerRestorePurpose::AlternateRestore,
    )
    .await
}

async fn commit_server_restore_drill(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerBackupPlanCommitRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    commit_server_restore_for_purpose(
        &state,
        &principal,
        request,
        ServerRestorePurpose::RestoreDrill,
    )
    .await
}

async fn commit_server_restore_for_purpose(
    state: &ServerState,
    principal: &SessionPrincipal,
    request: ServerBackupPlanCommitRequest,
    purpose: ServerRestorePurpose,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(state, principal)?;
    let pending = state
        .restore_plans
        .lock()
        .await
        .get(&request.plan_digest)
        .cloned()
        .ok_or(ApiError::BackupPlanUnavailable)?;
    if pending.purpose != purpose {
        return Err(ApiError::BackupPlanUnavailable);
    }
    let receipt = commit_server_restore_pair(&pending.plan).map_err(ApiError::Backup)?;
    let (event_type, response_stage) = match purpose {
        ServerRestorePurpose::AlternateRestore => ("server_restore_completed", "restored"),
        ServerRestorePurpose::RestoreDrill => ("server_restore_drill_completed", "drill_completed"),
    };
    record_server_operation_audit(
        state,
        principal,
        event_type,
        &format!(
            "planDigest={};workspaceRestoreId={};controlPlaneRestoreId={};exactPair={}",
            pending.plan.plan_digest,
            pending.plan.workspace_restore_id,
            pending.plan.control_plane_restore_id,
            receipt.verification.exact_pair,
        ),
    )?;
    state
        .restore_plans
        .lock()
        .await
        .remove(&request.plan_digest);
    Ok(Json(serde_json::json!({
        "stage": response_stage,
        "receipt": receipt,
        "cleanTargets": true,
        "auditRecorded": true,
    })))
}

async fn verify_server_restore(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<ServerRestoreVerifyRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    let verification = verify_server_restore_pair(
        request.workspace_snapshot_directory,
        request.control_plane_snapshot_directory,
        request.restored_workspace_root,
        request.restored_control_plane_root,
    )
    .map_err(ApiError::Backup)?;
    Ok(Json(serde_json::json!({
        "stage": "restore_verified",
        "verification": verification,
    })))
}

fn record_server_operation_audit(
    state: &ServerState,
    principal: &SessionPrincipal,
    event_type: &str,
    detail: &str,
) -> Result<(), ApiError> {
    state
        .control_plane
        .record_completed_operation(principal, event_type, detail, unix_now()?)
        .map_err(ApiError::ControlPlane)
}

fn require_owner(state: &ServerState, principal: &SessionPrincipal) -> Result<(), ApiError> {
    state.require_current_principal(principal)?;
    if principal.role.can_manage_workspace() {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn require_member_admin(state: &ServerState, principal: &SessionPrincipal) -> Result<(), ApiError> {
    state.require_current_principal(principal)?;
    if principal.role.can_manage_members() {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn require_content_write(principal: &SessionPrincipal) -> Result<(), ApiError> {
    if principal.role.can_write_content() {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn require_structure_write(principal: &SessionPrincipal) -> Result<(), ApiError> {
    if principal.role.can_mutate_structure() {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn map_member_error(error: AuthError) -> ApiError {
    match error {
        AuthError::MemberExists => ApiError::MemberExists,
        AuthError::MemberUnavailable => ApiError::MemberUnavailable,
        AuthError::LastOwner => ApiError::LastOwner,
        AuthError::InvalidLogin => ApiError::InvalidRequest(
            "login must be 3-64 lowercase ASCII letters, digits, dots, underscores, or hyphens",
        ),
        AuthError::InvalidRole => {
            ApiError::InvalidRequest("role must be owner, admin, editor, commenter, or viewer")
        }
        AuthError::InvalidAccess => {
            ApiError::InvalidRequest("access must be hidden, read, write, or null for inherit")
        }
        AuthError::Password => {
            ApiError::InvalidRequest("member password must contain between 12 and 1024 UTF-8 bytes")
        }
        AuthError::AuthorizationDenied => ApiError::AuthorizationDenied,
        other => ApiError::ControlPlane(other),
    }
}

fn visible_node_ids(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<BTreeSet<NodeId>, ApiError> {
    state.require_current_principal(principal)?;
    let inventory = scan_workspace(state.workspace_root());
    if principal.role == SessionRole::Owner {
        project_workspace_trash_state(state.workspace_root())
            .map_err(|_| ApiError::WorkspaceInvalid)?;
        return inventory
            .nodes
            .iter()
            .filter(|node| node.path != inventory.root.join(weftext_core::TRASH_NODE_NAME))
            .map(|node| node.id.ok_or(ApiError::WorkspaceInvalid))
            .collect();
    }
    let (_, scope) = authorized_scope_from_inventory(state, principal, &inventory)?;
    Ok(scope.node_ids().collect())
}

fn physical_ancestry_ids(
    inventory: &weftext_core::WorkspaceInventory,
    node_path: &Path,
) -> (Vec<String>, bool) {
    let mut ids = Vec::new();
    let mut allow_role_default = true;
    let mut cursor = Some(node_path);
    while let Some(path) = cursor {
        if !path.starts_with(&inventory.root) {
            break;
        }
        let mut matching = inventory.nodes.iter().filter(|node| node.path == path);
        if let Some(node) = matching.next() {
            if matching.next().is_some() {
                allow_role_default = false;
                break;
            }
            if let Some(node_id) = node.id {
                ids.push(node_id.to_string());
            } else {
                allow_role_default = false;
                break;
            }
        }
        if path == inventory.root {
            break;
        }
        cursor = path.parent();
    }
    (ids, allow_role_default)
}

fn inventory_node_access(
    state: &ServerState,
    principal: &SessionPrincipal,
    inventory: &weftext_core::WorkspaceInventory,
    node: &weftext_core::NodeRecord,
) -> Result<NodeAccess, ApiError> {
    let (ancestry, allow_role_default) = physical_ancestry_ids(inventory, &node.path);
    state
        .control_plane
        .effective_node_access(principal, &ancestry, allow_role_default)
        .map_err(ApiError::ControlPlane)
}

fn authorized_scope_from_inventory(
    state: &ServerState,
    principal: &SessionPrincipal,
    inventory: &weftext_core::WorkspaceInventory,
) -> Result<(NodeId, WorkspaceReadScope), ApiError> {
    state.require_current_principal(principal)?;

    let mut visible_nodes = inventory
        .nodes
        .iter()
        .filter(|node| node.path != inventory.root.join(weftext_core::TRASH_NODE_NAME))
        .filter_map(|node| {
            let node_id = node.id?;
            match inventory_node_access(state, principal, inventory, node) {
                Ok(NodeAccess::Read | NodeAccess::Write) => Some(Ok((node_id, node))),
                Ok(NodeAccess::Hidden) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    visible_nodes.sort_by(|(_, left), (_, right)| {
        left.path
            .components()
            .count()
            .cmp(&right.path.components().count())
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut visible_by_path = BTreeMap::<PathBuf, NodeId>::new();
    for (node_id, node) in &visible_nodes {
        if visible_by_path
            .insert(node.path.clone(), *node_id)
            .is_some()
        {
            return Err(ApiError::WorkspaceInvalid);
        }
    }

    let mut locators = BTreeMap::<PathBuf, String>::new();
    let mut projections = Vec::with_capacity(visible_nodes.len());
    for (node_id, node) in &visible_nodes {
        let parent = node
            .path
            .parent()
            .and_then(|path| visible_by_path.get(path).copied());
        let locator = if node.path == inventory.root {
            String::new()
        } else if let Some(parent_path) = node.path.parent().filter(|_| parent.is_some()) {
            join_portable_locator(
                locators
                    .get(parent_path)
                    .ok_or(ApiError::WorkspaceInvalid)?,
                &node.name,
            )
        } else {
            node.name.clone()
        };
        locators.insert(node.path.clone(), locator.clone());
        projections.push(WorkspaceNodeProjection::new(*node_id, parent, locator));
    }

    let scope = WorkspaceReadScope::new(projections).map_err(|_| ApiError::WorkspaceInvalid)?;
    if inventory.is_valid() {
        scope
            .validate_inventory(inventory)
            .map_err(|_| ApiError::WorkspaceInvalid)?;
    }

    let mut root_matches = inventory
        .nodes
        .iter()
        .filter(|node| node.path == inventory.root)
        .filter_map(|node| node.id);
    let root_node_id = root_matches.next().ok_or(ApiError::WorkspaceInvalid)?;
    if root_matches.next().is_some() {
        return Err(ApiError::WorkspaceInvalid);
    }
    if !scope.allows(root_node_id) {
        return Err(ApiError::NodeNotFound);
    }
    Ok((root_node_id, scope))
}

#[allow(clippy::too_many_lines)]
fn authorized_navigation(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<(NodeId, WorkspaceNavigationProjection), ApiError> {
    let inventory = scan_workspace(state.workspace_root());
    if principal.role == SessionRole::Owner {
        project_workspace_trash_state(state.workspace_root())
            .map_err(|_| ApiError::WorkspaceInvalid)?;
        let root_node_id = inventory
            .nodes
            .iter()
            .find(|node| node.path == state.workspace_root())
            .and_then(|node| node.id)
            .ok_or(ApiError::WorkspaceInvalid)?;
        let navigation = weftext_core::build_workspace_navigation(&inventory)
            .map_err(|_| ApiError::WorkspaceInvalid)?;
        return Ok((root_node_id, navigation));
    }
    let (root_node_id, scope) = authorized_scope_from_inventory(state, principal, &inventory)?;
    let visible = scope.node_ids().collect::<BTreeSet<_>>();
    let mut scoped_nodes = scope
        .node_ids()
        .map(|node_id| {
            let mut matching = inventory
                .nodes
                .iter()
                .filter(|node| node.id == Some(node_id));
            let node = matching.next().ok_or(ApiError::WorkspaceInvalid)?;
            if matching.next().is_some() {
                return Err(ApiError::WorkspaceInvalid);
            }
            Ok((node_id, node))
        })
        .collect::<Result<Vec<_>, ApiError>>()?;
    scoped_nodes.sort_by(|(left_id, _), (right_id, _)| {
        scope
            .depth(*left_id)
            .cmp(&scope.depth(*right_id))
            .then_with(|| scope.locator(*left_id).cmp(&scope.locator(*right_id)))
            .then_with(|| left_id.cmp(right_id))
    });
    let mut hierarchy = Vec::with_capacity(scoped_nodes.len());
    for (node_id, node) in scoped_nodes {
        let source =
            fs::read_to_string(&node.document_path).map_err(|_| ApiError::WorkspaceInvalid)?;
        let parent_node_id = scope.parent_node_id(node_id);
        let depth = usize::from(scope.depth(node_id).ok_or(ApiError::WorkspaceInvalid)?);
        let fallback = if node_id == root_node_id {
            WorkspaceItemIconFallback::WorkspaceRoot
        } else if node
            .name
            .eq_ignore_ascii_case(weftext_core::TRASH_NODE_NAME)
            && depth == 1
        {
            WorkspaceItemIconFallback::Trash
        } else {
            WorkspaceItemIconFallback::ManagedNode
        };
        hierarchy.push(NavigationNode {
            node_id,
            name: node.name.clone(),
            parent_node_id,
            locator: scope
                .locator(node_id)
                .ok_or(ApiError::WorkspaceInvalid)?
                .to_owned(),
            depth,
            child_count: 0,
            display_icon: derive_workspace_item_icon(
                resolve_node_icon_from_source(&source),
                fallback,
            ),
        });
    }
    let mut child_counts = BTreeMap::<NodeId, usize>::new();
    for node in &hierarchy {
        if let Some(parent) = node.parent_node_id {
            *child_counts.entry(parent).or_default() += 1;
        }
    }
    for node in &mut hierarchy {
        node.child_count = child_counts.get(&node.node_id).copied().unwrap_or(0);
    }
    let locations = hierarchy
        .iter()
        .map(|node| (node.node_id, node.locator.clone()))
        .collect::<BTreeMap<_, _>>();
    let icons = hierarchy
        .iter()
        .map(|node| (node.node_id, node.display_icon.clone()))
        .collect::<BTreeMap<_, _>>();
    let parents = hierarchy
        .iter()
        .map(|node| (node.node_id, node.parent_node_id))
        .collect::<BTreeMap<_, _>>();
    let hidden_node_paths = inventory
        .nodes
        .iter()
        .filter(|node| node.id.is_none_or(|id| !visible.contains(&id)))
        .map(|node| node.path.as_path())
        .collect::<Vec<_>>();
    let mut contents = Vec::new();
    for entry in &inventory.content {
        if entry.node_id.is_some_and(|id| !visible.contains(&id))
            || entry.owner_node_id.is_some_and(|id| !visible.contains(&id))
        {
            continue;
        }
        let entry_path = state.workspace_root().join(&entry.relative_path);
        let owner_path = entry.owner_node_id.and_then(|owner_id| {
            inventory
                .nodes
                .iter()
                .find(|node| node.id == Some(owner_id))
                .map(|node| node.path.as_path())
        });
        if entry.node_id.is_none()
            && hidden_node_paths.iter().any(|hidden_path| {
                entry_path.starts_with(hidden_path)
                    && owner_path.is_none_or(|owner_path| !owner_path.starts_with(hidden_path))
            })
        {
            continue;
        }
        let (locator, parent_locator) = if let Some(node_id) = entry.node_id {
            let locator = locations.get(&node_id).cloned().unwrap_or_default();
            let parent = parents
                .get(&node_id)
                .copied()
                .flatten()
                .and_then(|id| locations.get(&id).cloned());
            (locator, parent)
        } else if let Some(owner_id) = entry.owner_node_id {
            let owner_locator = locations.get(&owner_id).ok_or(ApiError::WorkspaceInvalid)?;
            let owner = inventory
                .nodes
                .iter()
                .find(|node| node.id == Some(owner_id))
                .ok_or(ApiError::WorkspaceInvalid)?;
            let owner_physical = owner
                .path
                .strip_prefix(state.workspace_root())
                .map_err(|_| ApiError::WorkspaceInvalid)?
                .to_string_lossy()
                .replace('\\', "/");
            let suffix = if owner_physical.is_empty() {
                entry.relative_path.as_str()
            } else {
                entry
                    .relative_path
                    .strip_prefix(&owner_physical)
                    .and_then(|value| value.strip_prefix('/'))
                    .unwrap_or(&entry.name)
            };
            let locator = join_portable_locator(owner_locator, suffix);
            let parent = locator
                .rsplit_once('/')
                .map_or_else(String::new, |(parent, _)| parent.to_owned());
            (locator, Some(parent))
        } else {
            (
                entry.relative_path.clone(),
                entry.parent_relative_path.clone(),
            )
        };
        let fallback = match entry.kind {
            WorkspaceContentKind::ManagedNode => WorkspaceItemIconFallback::ManagedNode,
            WorkspaceContentKind::UnmanagedDirectory => WorkspaceItemIconFallback::UnmanagedFolder,
            WorkspaceContentKind::UnmanagedMarkdown => WorkspaceItemIconFallback::UnmanagedMarkdown,
            WorkspaceContentKind::Resource => WorkspaceItemIconFallback::OrdinaryFile,
        };
        contents.push(NavigationContentItem {
            kind: entry.kind,
            name: entry.name.clone(),
            locator,
            parent_locator,
            node_id: entry.node_id,
            owner_node_id: entry.owner_node_id,
            display_icon: entry
                .node_id
                .and_then(|id| icons.get(&id).cloned())
                .unwrap_or_else(|| derive_workspace_item_icon(None, fallback)),
        });
    }
    Ok((
        root_node_id,
        WorkspaceNavigationProjection {
            version: weftext_core::NAVIGATION_PROJECTION_VERSION,
            root_node_id,
            hierarchy,
            contents,
        },
    ))
}

fn join_portable_locator(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_owned()
    } else if child.is_empty() {
        parent.to_owned()
    } else {
        format!("{parent}/{child}")
    }
}

fn actor_workspace_revision(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<String, ApiError> {
    if principal.role == SessionRole::Owner {
        return read_workspace_revision(state.workspace_root())
            .map(|revision| revision.to_string())
            .map_err(|_| ApiError::WorkspaceInvalid);
    }
    let (_, navigation) = authorized_navigation(state, principal)?;
    let mut hasher = Sha256::new();
    hasher.update(b"weftext-server-actor-workspace-revision-v1\0");
    hasher.update(state.workspace_scope().as_bytes());
    hasher.update([0]);
    hasher.update(principal.actor_scope.as_bytes());
    hasher.update([0]);
    hasher.update(principal.role.as_str().as_bytes());
    hasher.update([0]);
    hasher.update(
        state
            .control_plane
            .authorization_epoch(&principal.actor_scope)
            .map_err(ApiError::ControlPlane)?
            .to_le_bytes(),
    );
    hasher
        .update(serde_json::to_vec(&navigation.contents).map_err(|_| ApiError::WorkspaceInvalid)?);
    for node in &navigation.hierarchy {
        let path = state.authorized_node_path(principal, node.node_id, false)?;
        let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
        hasher.update(node.node_id.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(node.locator.as_bytes());
        hasher.update([0]);
        hasher.update(snapshot.revision.to_string().as_bytes());
        let annotations = path.join("weftext.annotations.json");
        if annotations.is_file() {
            let bytes = fs::read(annotations).map_err(|_| ApiError::WorkspaceInvalid)?;
            hasher.update(Sha256::digest(bytes));
        }
    }
    Ok(format!("actor-v1:{:x}", hasher.finalize()))
}

fn authorized_read_scope(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<WorkspaceReadScope, ApiError> {
    let inventory = scan_workspace(state.workspace_root());
    authorized_scope_from_inventory(state, principal, &inventory).map(|(_, scope)| scope)
}

fn resolve_client_workspace_revision(
    state: &ServerState,
    principal: &SessionPrincipal,
    supplied: &str,
) -> Result<WorkspaceRevision, ApiError> {
    if principal.role == SessionRole::Owner {
        let actual = read_workspace_revision(state.workspace_root())
            .map_err(|_| ApiError::WorkspaceInvalid)?;
        let expected = parse_workspace_revision(supplied)?;
        require_workspace_revision(&expected, &actual)?;
        return Ok(actual);
    }
    let projected = actor_workspace_revision(state, principal)?;
    if supplied != projected {
        return Err(ApiError::StaleWorkspaceRevision {
            expected: supplied.to_owned(),
            actual: projected,
        });
    }
    read_workspace_revision(state.workspace_root()).map_err(|_| ApiError::WorkspaceInvalid)
}

fn require_planned_workspace_revision(
    state: &ServerState,
    principal: &SessionPrincipal,
    supplied: &str,
    expected_actual: &WorkspaceRevision,
    planned_actual: &WorkspaceRevision,
) -> Result<(), ApiError> {
    if principal.role == SessionRole::Owner {
        require_workspace_revision(expected_actual, planned_actual)
    } else {
        let projected = actor_workspace_revision(state, principal)?;
        if supplied == projected {
            Ok(())
        } else {
            Err(ApiError::StaleWorkspaceRevision {
                expected: supplied.to_owned(),
                actual: projected,
            })
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryResponse {
    workspace_scope: String,
    workspace_revision: String,
    document_format: weftext_core::WorkspaceDocumentFormat,
    root_node_id: NodeId,
    nodes: Vec<InventoryNode>,
    content: Vec<InventoryContent>,
    navigation: weftext_core::WorkspaceNavigationProjection,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryNode {
    id: NodeId,
    name: String,
    parent_id: Option<NodeId>,
    locator: String,
    icon: Option<weftext_core::ResolvedNodeIcon>,
    display_icon: WorkspaceItemIcon,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryContent {
    kind: WorkspaceContentKind,
    name: String,
    locator: String,
    parent_locator: Option<String>,
    node_id: Option<NodeId>,
    owner_node_id: Option<NodeId>,
    display_icon: WorkspaceItemIcon,
}

async fn inventory(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<InventoryResponse>, ApiError> {
    let authorization_epoch = (principal.role != SessionRole::Owner)
        .then(|| {
            state
                .control_plane
                .authorization_epoch(&principal.actor_scope)
                .map_err(ApiError::ControlPlane)
        })
        .transpose()?;
    let (root_node_id, navigation) = authorized_navigation(&state, &principal)?;
    let workspace_revision = actor_workspace_revision(&state, &principal)?;
    if let Some(expected) = authorization_epoch {
        let actual = state
            .control_plane
            .authorization_epoch(&principal.actor_scope)
            .map_err(ApiError::ControlPlane)?;
        if actual != expected {
            return Err(ApiError::ControlPlaneUnavailable);
        }
    }
    let nodes = navigation
        .hierarchy
        .iter()
        .map(|node| InventoryNode {
            id: node.node_id,
            name: node.name.clone(),
            parent_id: node.parent_node_id,
            locator: node.locator.clone(),
            icon: match &node.display_icon {
                WorkspaceItemIcon::ExplicitNode(icon) => Some(icon.clone()),
                _ => None,
            },
            display_icon: node.display_icon.clone(),
        })
        .collect();
    let content = navigation
        .contents
        .iter()
        .map(|entry| InventoryContent {
            kind: entry.kind,
            name: entry.name.clone(),
            locator: entry.locator.clone(),
            parent_locator: entry.parent_locator.clone(),
            node_id: entry.node_id,
            owner_node_id: entry.owner_node_id,
            display_icon: entry.display_icon.clone(),
        })
        .collect();
    Ok(Json(InventoryResponse {
        workspace_scope: state.workspace_scope().to_owned(),
        workspace_revision,
        document_format: weftext_core::workspace_document_format(state.workspace_root()),
        root_node_id,
        nodes,
        content,
        navigation,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrashInventoryResponse {
    workspace_revision: String,
    state: weftext_core::WorkspaceTrashState,
    legacy_migration_required: bool,
    items: Vec<WorkspaceTrashItemProjection>,
    reconciliation: TrashReconciliationResponse,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrashReconciliationResponse {
    required: bool,
    issue_count: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashNodePreviewRequest {
    base_workspace_revision: String,
    trashed_at: String,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashResourcesPreviewRequest {
    base_workspace_revision: String,
    trashed_at: String,
    resources: Vec<TrashResourceSelection>,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashRestorePreviewRequest {
    base_workspace_revision: String,
    mode: String,
    target_node_id: Option<String>,
    name: Option<String>,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashPermanentDeleteEvidence {
    trash_item_id: TrashItemId,
    payload_sha256: String,
    payload_byte_length: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashPermanentDeletePreviewRequest {
    base_workspace_revision: String,
    items: Vec<TrashPermanentDeleteEvidence>,
    #[serde(default = "caller_explicit_target_resolution")]
    resolved_by: WorkspaceTargetResolution,
}

const fn caller_explicit_target_resolution() -> WorkspaceTargetResolution {
    WorkspaceTargetResolution::CallerExplicit
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TrashLegacyMigrationPreviewRequest {
    base_workspace_revision: String,
    trashed_at: String,
}

async fn trash_inventory(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<TrashInventoryResponse>, ApiError> {
    state.require_current_principal(&principal)?;
    let projection = project_workspace_trash_state(state.workspace_root())
        .map_err(|_| ApiError::WorkspaceInvalid)?;
    let items = if projection.reconciliation_required || projection.legacy_migration_required {
        Vec::new()
    } else {
        projection
            .items
            .into_iter()
            .filter_map(
                |item| match trash_item_access(&state, &principal, &item, false) {
                    Ok(true) => Some(Ok(item)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                },
            )
            .collect::<Result<Vec<_>, ApiError>>()?
    };
    Ok(Json(TrashInventoryResponse {
        workspace_revision: actor_workspace_revision(&state, &principal)?,
        state: projection.state,
        legacy_migration_required: projection.legacy_migration_required,
        items,
        reconciliation: TrashReconciliationResponse {
            required: projection.reconciliation_required,
            issue_count: projection.diagnostic_count,
        },
    }))
}

async fn preview_trash_node(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<TrashNodePreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    require_trash_workspace_writable(&state, false)?;
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, true)?;
    let actual =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let mut plan = plan_trash_node_at(state.workspace_root(), node_id, &request.trashed_at)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &actual,
        &plan.base_revision,
    )?;
    stage_trash_plan(
        &state,
        &principal,
        request.base_workspace_revision,
        plan,
        PendingTrashAuthorization::Ordinary,
    )
    .await
}

async fn preview_trash_resources(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<TrashResourcesPreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    require_trash_workspace_writable(&state, false)?;
    if request.resources.is_empty() || request.resources.len() > MAX_SERVER_TRASH_ITEMS {
        return Err(ApiError::InvalidRequest(
            "resource Trash selection must contain 1-256 items",
        ));
    }
    for resource in &request.resources {
        state.authorized_node_path(&principal, resource.owner_node_id, true)?;
    }
    let actual =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let mut plan = plan_trash_resources_at(
        state.workspace_root(),
        request.resources,
        &request.trashed_at,
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    if plan.captured_target.is_some() {
        bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
            .map_err(|error| map_trash_transaction_error(&principal, error))?;
    }
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &actual,
        &plan.base_revision,
    )?;
    stage_trash_plan(
        &state,
        &principal,
        request.base_workspace_revision,
        plan,
        PendingTrashAuthorization::Ordinary,
    )
    .await
}

async fn preview_trash_restore(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(trash_item_id): ApiPath<String>,
    ApiJson(request): ApiJson<TrashRestorePreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    require_trash_workspace_writable(&state, false)?;
    let item_id = parse_server_trash_item_id(&trash_item_id)?;
    let projection = authorized_trash_item(&state, &principal, item_id, true)?;
    let mode = match request.mode.as_str() {
        "original" if request.target_node_id.is_none() && request.name.is_none() => {
            TrashRestoreMode::Original
        }
        "with_ancestors" if request.target_node_id.is_none() && request.name.is_none() => {
            for ancestor in &projection.restore.required_ancestor_item_ids {
                authorized_trash_item(&state, &principal, *ancestor, true)?;
            }
            TrashRestoreMode::WithAncestors
        }
        "existing_target" => {
            let target_node_id =
                request
                    .target_node_id
                    .as_deref()
                    .ok_or(ApiError::InvalidRequest(
                        "existing-target restore requires targetNodeId and name",
                    ))?;
            let name = request.name.ok_or(ApiError::InvalidRequest(
                "existing-target restore requires targetNodeId and name",
            ))?;
            let target_node_id = parse_node_id(target_node_id)?;
            state.authorized_node_path(&principal, target_node_id, true)?;
            TrashRestoreMode::ExistingTarget {
                target_node_id,
                name,
            }
        }
        _ => {
            return Err(ApiError::InvalidRequest(
                "restore mode must be original, with_ancestors, or existing_target with an explicit target and name",
            ));
        }
    };
    let actual =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let mut plan = plan_restore_trash_item(state.workspace_root(), item_id, mode)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &actual,
        &plan.base_revision,
    )?;
    stage_trash_plan(
        &state,
        &principal,
        request.base_workspace_revision,
        plan,
        PendingTrashAuthorization::Ordinary,
    )
    .await
}

async fn preview_trash_permanent_delete(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<TrashPermanentDeletePreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_permanent_trash_delete(&state, &principal)?;
    require_trash_workspace_writable(&state, false)?;
    if request.items.is_empty() || request.items.len() > MAX_SERVER_TRASH_ITEMS {
        return Err(ApiError::InvalidRequest(
            "permanent-delete selection must contain 1-256 exact items",
        ));
    }
    for evidence in &request.items {
        authorized_trash_item(&state, &principal, evidence.trash_item_id, true)?;
    }
    let actual =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let preview = preview_permanent_delete_trash_items(
        state.workspace_root(),
        request
            .items
            .iter()
            .map(|item| item.trash_item_id)
            .collect(),
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    require_exact_server_permanent_delete_evidence(&preview, request.items)?;
    let confirmation = confirm_permanent_delete_trash_items(
        preview,
        true,
        weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE,
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    let mut plan = plan_permanently_delete_trash_items(state.workspace_root(), &confirmation)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    if plan.captured_target.is_some() {
        bind_workspace_transaction_target_resolution(&mut plan, request.resolved_by)
            .map_err(|error| map_trash_transaction_error(&principal, error))?;
    }
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &actual,
        &plan.base_revision,
    )?;
    stage_trash_plan(
        &state,
        &principal,
        request.base_workspace_revision,
        plan,
        PendingTrashAuthorization::PermanentDelete,
    )
    .await
}

async fn preview_legacy_trash_migration(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<TrashLegacyMigrationPreviewRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    state.require_current_principal(&principal)?;
    if principal.role != SessionRole::Owner {
        return Err(ApiError::AuthorizationDenied);
    }
    require_trash_workspace_writable(&state, true)?;
    let snapshot_parent = state
        .trash_migration_snapshot_parent
        .as_deref()
        .ok_or(ApiError::TrashMigrationBackupUnavailable)?;
    let actual =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let backup = prepare_legacy_trash_migration_backup(state.workspace_root(), snapshot_parent)
        .map_err(|error| map_trash_transaction_error(&principal, error))?;
    let plan = plan_migrate_legacy_workspace_trash_at_with_backup(
        state.workspace_root(),
        &request.trashed_at,
        &backup,
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &actual,
        &plan.base_revision,
    )?;
    stage_trash_plan(
        &state,
        &principal,
        request.base_workspace_revision,
        plan,
        PendingTrashAuthorization::LegacyMigration(Box::new(backup)),
    )
    .await
}

async fn stage_trash_plan(
    state: &ServerState,
    principal: &SessionPrincipal,
    base_actor_revision: String,
    plan: WorkspaceTransactionPlan,
    authorization: PendingTrashAuthorization,
) -> Result<Json<serde_json::Value>, ApiError> {
    let draft_registry = WorkspaceDraftRegistryView::new(
        format!("server-session:{}:trash-preview", principal.session_id),
        std::iter::empty(),
    )
    .map_err(|error| map_trash_transaction_error(principal, error))?;
    let draft_gate = preview_workspace_transaction_draft_gate(&plan, &draft_registry)
        .map_err(|error| map_trash_transaction_error(principal, error))?;
    let draft_gate_token = draft_gate
        .executable_token
        .clone()
        .ok_or(ApiError::WorkspaceWriteRejected)?;
    let reviewed_request = plan
        .reviewed_trash_request()
        .cloned()
        .ok_or(ApiError::WorkspaceWriteRejected)?;
    let plan_id = plan.plan_id.clone();
    let response = serde_json::json!({
        "planId": plan_id,
        "baseWorkspaceRevision": base_actor_revision,
        "action": plan.action,
        "scopeSummary": &plan.scope_summary,
        "identityMap": &plan.identity_map,
        "capturedTarget": &plan.captured_target,
        "targetNodeIds": &plan.target_node_ids,
        "draftSensitiveNodeIds": &plan.draft_sensitive_node_ids,
        "draftGate": {
            "requiredCleanNodeIds": draft_gate.required_clean_node_ids,
            "blockingDirtyNodeIds": draft_gate.blocking_dirty_node_ids,
            "observationDigest": draft_gate.observation_digest,
        },
        "trashItemChanges": plan.trash_item_changes(),
    });
    let now = unix_now()?;
    let mut plans = state.trash_plans.lock().await;
    plans.retain(|_, pending| pending.expires_at > now);
    if plans.len() >= MAX_PENDING_TRASH_PLANS {
        return Err(ApiError::TrashPlanLimit);
    }
    plans.insert(
        plan_id,
        PendingTrashPlan {
            session_id: principal.session_id.clone(),
            expires_at: now + TRASH_PLAN_TTL_SECONDS,
            base_actor_revision,
            reviewed_request,
            draft_gate_token,
            authorization,
        },
    );
    Ok(Json(response))
}

#[expect(
    clippy::too_many_lines,
    reason = "the single commit boundary keeps session consumption, exact Core replan, durable audit, recovery, and response authority together"
)]
async fn commit_trash_transaction(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(plan_id): ApiPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    if weftext_core::TrashReviewId::from_str(&plan_id).is_err() {
        return Err(ApiError::TrashPlanUnavailable);
    }
    let _commit_guard = state.commits.lock().await;
    let now = unix_now()?;
    let pending = {
        let mut plans = state.trash_plans.lock().await;
        plans.retain(|_, pending| pending.expires_at > now);
        if plans
            .get(&plan_id)
            .is_none_or(|pending| pending.session_id != principal.session_id)
        {
            return Err(ApiError::TrashPlanUnavailable);
        }
        plans
            .remove(&plan_id)
            .ok_or(ApiError::TrashPlanUnavailable)?
    };
    let legacy_migration = matches!(
        &pending.authorization,
        PendingTrashAuthorization::LegacyMigration(_)
    );
    require_trash_workspace_writable(&state, legacy_migration)?;
    let current_actor_revision = actor_workspace_revision(&state, &principal)?;
    if current_actor_revision != pending.base_actor_revision {
        return Err(ApiError::StaleWorkspaceRevision {
            expected: pending.base_actor_revision,
            actual: current_actor_revision,
        });
    }
    authorize_reviewed_trash_action(&state, &principal, &pending.reviewed_request.action)?;
    let authorization = match pending.authorization {
        PendingTrashAuthorization::Ordinary => TrashReviewedReplanAuthorization::Ordinary,
        PendingTrashAuthorization::LegacyMigration(backup) => {
            TrashReviewedReplanAuthorization::LegacyMigration { backup: *backup }
        }
        PendingTrashAuthorization::PermanentDelete => {
            require_permanent_trash_delete(&state, &principal)?;
            TrashReviewedReplanAuthorization::PermanentDelete {
                higher_permission_granted: true,
                exact_phrase: weftext_core::TRASH_PERMANENT_DELETE_CONFIRMATION_PHRASE.to_owned(),
            }
        }
    };
    let plan = replan_reviewed_trash_request(
        state.workspace_root(),
        &pending.reviewed_request,
        authorization,
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    let event_type = trash_audit_event(&pending.reviewed_request.action);
    let audit_time = unix_now()?;
    let audit_intent = state
        .control_plane
        .begin_audit_intent(
            &principal,
            event_type,
            &format!(
                "plan={};itemCount={};authority={}",
                plan.plan_id,
                plan.trash_item_changes().len(),
                pending.reviewed_request.authority_digest
            ),
            "workspace",
            "workspace",
            &format!("not:{}", plan.base_revision),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let current_draft_registry = WorkspaceDraftRegistryView::new(
        format!("server-session:{}:trash-commit", principal.session_id),
        std::iter::empty(),
    )
    .map_err(|error| map_trash_transaction_error(&principal, error))?;
    let committed = match commit_workspace_transaction_with_draft_gate(
        &plan,
        &pending.draft_gate_token,
        &current_draft_registry,
    ) {
        Ok(committed) => committed,
        Err(error) => {
            if let Some(committed) = recover_workspace_commit(&plan)? {
                committed
            } else {
                let _ = state.control_plane.cancel_audit_intent(&audit_intent);
                return Err(map_trash_transaction_error(&principal, error));
            }
        }
    };
    let _ = state.control_plane.update_audit_intent_authority(
        &audit_intent,
        "workspace",
        "workspace",
        &committed.revision.to_string(),
    );
    let audit_recorded = state
        .control_plane
        .finalize_audit_intent(&audit_intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    if let Ok(snapshot) = read_node_document(state.workspace_root()) {
        let _ = state.changes.send(ChangeEvent {
            node_id: snapshot.node_id,
            revision: snapshot.revision,
        });
    }
    Ok(Json(serde_json::json!({
        "committed": true,
        "planId": plan.plan_id,
        "action": plan.action,
        "workspaceRevision": actor_workspace_revision(&state, &principal)
            .map_err(|_| ApiError::CommitOutcomeIndeterminate)?,
        "trashItemChanges": plan.trash_item_changes(),
        "auditRecorded": audit_recorded,
    })))
}

fn require_trash_workspace_writable(
    state: &ServerState,
    allow_legacy_migration: bool,
) -> Result<(), ApiError> {
    let projection = project_workspace_trash_state(state.workspace_root())
        .map_err(|_| ApiError::WorkspaceInvalid)?;
    if projection.reconciliation_required {
        return Err(ApiError::TrashReadOnly);
    }
    if allow_legacy_migration {
        if projection.legacy_migration_required {
            Ok(())
        } else {
            Err(ApiError::TrashPlanUnavailable)
        }
    } else if projection.legacy_migration_required {
        Err(ApiError::TrashReadOnly)
    } else {
        Ok(())
    }
}

fn require_permanent_trash_delete(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<(), ApiError> {
    state.require_current_principal(principal)?;
    if principal.role == SessionRole::Owner
        || (state.allow_admin_permanent_delete && principal.role == SessionRole::Admin)
    {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn parse_server_trash_item_id(value: &str) -> Result<TrashItemId, ApiError> {
    TrashItemId::from_str(value).map_err(|_| ApiError::TrashItemUnavailable)
}

fn authorized_trash_item(
    state: &ServerState,
    principal: &SessionPrincipal,
    item_id: TrashItemId,
    write: bool,
) -> Result<WorkspaceTrashItemProjection, ApiError> {
    let projection = project_workspace_trash_state(state.workspace_root())
        .map_err(|_| ApiError::WorkspaceInvalid)?;
    if projection.reconciliation_required || projection.legacy_migration_required {
        return Err(ApiError::TrashItemUnavailable);
    }
    let item = projection
        .items
        .into_iter()
        .find(|item| item.manifest.trash_item_id() == item_id)
        .ok_or(ApiError::TrashItemUnavailable)?;
    if trash_item_access(state, principal, &item, write)? {
        Ok(item)
    } else {
        Err(ApiError::TrashItemUnavailable)
    }
}

fn trash_item_access(
    state: &ServerState,
    principal: &SessionPrincipal,
    item: &WorkspaceTrashItemProjection,
    write: bool,
) -> Result<bool, ApiError> {
    state.require_current_principal(principal)?;
    if principal.role == SessionRole::Owner {
        return Ok(true);
    }
    if matches!(
        item.restore.origin_resolution,
        weftext_core::TrashOriginResolution::Missing
            | weftext_core::TrashOriginResolution::Unknown
            | weftext_core::TrashOriginResolution::ReconciliationRequired
    ) {
        return Ok(false);
    }
    match &item.manifest {
        TrashItemManifest::Resource {
            original_owner_node_id: Some(owner_node_id),
            ..
        } if item.restore.origin_resolution == weftext_core::TrashOriginResolution::Active => {
            match state.authorized_node_path(principal, *owner_node_id, write) {
                Ok(_) => Ok(true),
                Err(ApiError::NodeNotFound) => Ok(false),
                Err(error) => Err(error),
            }
        }
        TrashItemManifest::Resource { .. } => Ok(false),
        TrashItemManifest::Node {
            node_id,
            original_parent_node_id,
            ancestor_node_ids,
            ..
        } => {
            if item.restore.origin_resolution == weftext_core::TrashOriginResolution::Active {
                let Some(parent_node_id) = original_parent_node_id else {
                    return Ok(false);
                };
                match state.authorized_node_path(principal, *parent_node_id, write) {
                    Ok(_) => {}
                    Err(ApiError::NodeNotFound) => return Ok(false),
                    Err(error) => return Err(error),
                }
            }
            for contained_node_id in &item.contained_node_ids {
                let mut nearest = vec![contained_node_id.to_string()];
                if contained_node_id != node_id {
                    nearest.push(node_id.to_string());
                }
                if let Some(parent_node_id) = original_parent_node_id {
                    nearest.push(parent_node_id.to_string());
                }
                nearest.extend(ancestor_node_ids.iter().rev().map(ToString::to_string));
                let mut seen = BTreeSet::new();
                nearest.retain(|id| seen.insert(id.clone()));
                let access = state
                    .control_plane
                    .effective_node_access(principal, &nearest, true)
                    .map_err(ApiError::ControlPlane)?;
                if access == NodeAccess::Hidden || (write && access != NodeAccess::Write) {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

fn authorize_reviewed_trash_action(
    state: &ServerState,
    principal: &SessionPrincipal,
    action: &TrashReviewedAction,
) -> Result<(), ApiError> {
    require_structure_write(principal)?;
    match action {
        TrashReviewedAction::StoreNode { node_id, .. } => {
            state.authorized_node_path(principal, *node_id, true)?;
        }
        TrashReviewedAction::StoreResources { resources, .. } => {
            for resource in resources {
                state.authorized_node_path(principal, resource.owner_node_id, true)?;
            }
        }
        TrashReviewedAction::Restore {
            trash_item_id,
            mode,
        } => {
            let item = authorized_trash_item(state, principal, *trash_item_id, true)?;
            if matches!(mode, TrashRestoreMode::WithAncestors) {
                for ancestor in item.restore.required_ancestor_item_ids {
                    authorized_trash_item(state, principal, ancestor, true)?;
                }
            }
            if let TrashRestoreMode::ExistingTarget { target_node_id, .. } = mode {
                state.authorized_node_path(principal, *target_node_id, true)?;
            }
        }
        TrashReviewedAction::MigrateLegacy { .. } => {
            if principal.role != SessionRole::Owner {
                return Err(ApiError::AuthorizationDenied);
            }
        }
        TrashReviewedAction::PermanentDelete { preview } => {
            require_permanent_trash_delete(state, principal)?;
            for item in &preview.items {
                authorized_trash_item(state, principal, item.trash_item_id, true)?;
            }
        }
    }
    Ok(())
}

fn require_exact_server_permanent_delete_evidence(
    preview: &weftext_core::TrashPermanentDeletePreview,
    mut supplied: Vec<TrashPermanentDeleteEvidence>,
) -> Result<(), ApiError> {
    let mut expected = preview
        .items
        .iter()
        .map(|item| TrashPermanentDeleteEvidence {
            trash_item_id: item.trash_item_id,
            payload_sha256: item.payload_sha256.clone(),
            payload_byte_length: item.payload_byte_length,
        })
        .collect::<Vec<_>>();
    supplied.sort_by_key(|item| item.trash_item_id);
    expected.sort_by_key(|item| item.trash_item_id);
    if supplied == expected {
        Ok(())
    } else {
        Err(ApiError::InvalidRequest(
            "permanent-delete evidence must exactly match item IDs, payload digests, and byte lengths",
        ))
    }
}

fn trash_audit_event(action: &TrashReviewedAction) -> &'static str {
    match action {
        TrashReviewedAction::StoreNode { .. } => "trash_node_stored",
        TrashReviewedAction::StoreResources { .. } => "trash_resources_stored",
        TrashReviewedAction::Restore { .. } => "trash_item_restored",
        TrashReviewedAction::MigrateLegacy { .. } => "trash_legacy_migrated",
        TrashReviewedAction::PermanentDelete { .. } => "trash_items_permanently_deleted",
    }
}

fn map_trash_transaction_error(
    principal: &SessionPrincipal,
    error: WorkspaceTransactionError,
) -> ApiError {
    if principal.role == SessionRole::Owner {
        ApiError::WorkspaceTransaction(error)
    } else {
        ApiError::WorkspaceWriteRejected
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentResponse {
    node_id: NodeId,
    name: String,
    revision: DocumentRevision,
    length: u64,
    source: String,
    profile: DocumentProfileDescriptor,
    model: DocumentModel,
    view: DocumentViewModel,
    metadata: NodeMetadataProjection,
    properties: weftext_core::DocumentPropertyAnalysis,
    icon: Option<weftext_core::ResolvedNodeIcon>,
}

async fn open_document(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
) -> Result<Json<DocumentResponse>, ApiError> {
    let id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, id, false)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let setting = root_presentation_setting(&state, &principal)?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(ApiError::WorkspaceInvalid)?
        .to_owned();
    let analysis = analyze_document_for_profile(snapshot.profile, &snapshot.source, setting);
    let metadata = project_node_metadata(
        &snapshot.source,
        if path == state.workspace_root() {
            NodeMetadataScope::WorkspaceRoot
        } else {
            NodeMetadataScope::Node
        },
    )
    .map_err(|_| ApiError::WorkspaceInvalid)?;
    let properties = weftext_core::analyze_document_header_properties(&snapshot.source);
    Ok(Json(DocumentResponse {
        node_id: snapshot.node_id,
        name,
        revision: snapshot.revision,
        length: u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX),
        profile: analysis.descriptor,
        model: analysis.model,
        view: analysis.view,
        metadata,
        properties,
        icon: resolve_node_icon_from_source(&snapshot.source),
        source: snapshot.source,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DocumentMutationRequest {
    base_revision: String,
    source: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    node_id: NodeId,
    base_revision: DocumentRevision,
    next_revision: DocumentRevision,
    old_length: u64,
    new_length: u64,
    changed: bool,
    profile: DocumentProfileDescriptor,
    model: DocumentModel,
    view: DocumentViewModel,
    icon: Option<weftext_core::ResolvedNodeIcon>,
}

async fn preview_document(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<DocumentMutationRequest>,
) -> Result<Json<PreviewResponse>, ApiError> {
    require_content_write(&principal)?;
    let id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, id, true)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let edit = whole_source_edit(&snapshot.source, request.source);
    let plan = plan_document_edit(&path, &base_revision, [edit]).map_err(ApiError::Document)?;
    let setting = root_presentation_setting(&state, &principal)?;
    let analysis = analyze_document_for_profile(snapshot.profile, plan.next_source(), setting);
    let icon = resolve_node_icon_from_source(plan.next_source());
    Ok(Json(PreviewResponse {
        node_id: plan.node_id,
        base_revision: plan.base_revision,
        next_revision: plan.next_revision,
        old_length: plan.old_length,
        new_length: plan.new_length,
        changed: plan.changed,
        profile: analysis.descriptor,
        model: analysis.model,
        view: analysis.view,
        icon,
    }))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CommitResponse {
    node_id: NodeId,
    revision: DocumentRevision,
    length: u64,
    changed: bool,
    icon: Option<weftext_core::ResolvedNodeIcon>,
}

async fn commit_document(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<DocumentMutationRequest>,
) -> Result<Json<CommitResponse>, ApiError> {
    require_content_write(&principal)?;
    let id = parse_node_id(&node_id)?;
    let _commit_guard = state.commits.lock().await;
    let path = state.authorized_node_path(&principal, id, true)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let edit = whole_source_edit(&snapshot.source, request.source);
    let plan = plan_document_edit(&path, &base_revision, [edit]).map_err(ApiError::Document)?;
    let icon = resolve_node_icon_from_source(plan.next_source());
    let audit_time = unix_now()?;
    let audit_intent = state
        .control_plane
        .begin_audit_intent(
            &principal,
            "document_edited",
            &format!("node={id};changed={}", plan.changed),
            "document",
            &id.to_string(),
            &plan.next_revision.to_string(),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let committed = match commit_document_edit(&plan) {
        Ok(committed) => committed,
        Err(error) => match document_authority(&state, id, &plan.next_revision) {
            DocumentAuthority::Committed(committed) => committed,
            DocumentAuthority::Rejected => {
                let _ = state.control_plane.cancel_audit_intent(&audit_intent);
                return Err(ApiError::Document(error));
            }
            DocumentAuthority::Indeterminate => {
                return Err(ApiError::CommitOutcomeIndeterminate);
            }
        },
    };
    let response = finish_committed_document(&plan, committed, icon, &state.changes);
    let _ = state.control_plane.finalize_audit_intent(
        &audit_intent,
        unix_now().unwrap_or(audit_time),
        None,
    );
    Ok(Json(response))
}

fn finish_committed_document(
    plan: &DocumentEditPlan,
    committed: CommittedDocument,
    icon: Option<ResolvedNodeIcon>,
    changes: &broadcast::Sender<ChangeEvent>,
) -> CommitResponse {
    let response = CommitResponse {
        node_id: committed.node_id,
        revision: committed.revision.clone(),
        length: committed.length,
        changed: plan.changed,
        icon,
    };
    if plan.changed {
        let _ = changes.send(ChangeEvent {
            node_id: committed.node_id,
            revision: committed.revision,
        });
    }
    response
}

enum DocumentAuthority {
    Committed(CommittedDocument),
    Rejected,
    Indeterminate,
}

fn document_authority(
    state: &ServerState,
    node_id: NodeId,
    expected_revision: &DocumentRevision,
) -> DocumentAuthority {
    let Ok(path) = state.node_path(node_id) else {
        return DocumentAuthority::Indeterminate;
    };
    match read_node_document(&path) {
        Ok(snapshot) if snapshot.revision == *expected_revision => {
            DocumentAuthority::Committed(CommittedDocument {
                node_id: snapshot.node_id,
                document_path: snapshot.document_path,
                revision: snapshot.revision,
                length: u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX),
            })
        }
        Ok(_) => DocumentAuthority::Rejected,
        Err(_) => DocumentAuthority::Indeterminate,
    }
}

fn recover_workspace_commit(
    plan: &WorkspaceTransactionPlan,
) -> Result<Option<CommittedWorkspaceTransaction>, ApiError> {
    let mut report = recover_workspace_transaction_for_plan(plan)
        .map_err(|_| ApiError::CommitOutcomeIndeterminate)?;
    if report.committed_transactions.is_empty() {
        return Ok(None);
    }
    if report.committed_cleaned != 1
        || report.committed_transactions.len() != 1
        || report.applying_rolled_back != 0
        || report.prepared_removed != 0
    {
        return Err(ApiError::CommitOutcomeIndeterminate);
    }
    Ok(report.committed_transactions.pop())
}

fn whole_source_edit(current_source: &str, replacement: String) -> DocumentEdit {
    DocumentEdit {
        start: 0,
        end: u64::try_from(current_source.len()).unwrap_or(u64::MAX),
        replacement,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollaborationSnapshotResponse {
    wire_version: &'static str,
    node_id: NodeId,
    actor_id: String,
    state: DocumentStateView,
    source: String,
    participants: Vec<Participant>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollaborationOperationResponse {
    wire_version: &'static str,
    status: &'static str,
    node_id: NodeId,
    actor_id: String,
    client_id: String,
    operation_id: String,
    transaction_id: String,
    request_base_revision: String,
    request_base_version: u64,
    applied_base_revision: String,
    applied_base_version: u64,
    result_revision: String,
    state: DocumentStateView,
    transformed: bool,
    operations: Vec<TextOperation>,
    audit_recorded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error_code: Option<&'static str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CollaborationPresenceResponse {
    wire_version: &'static str,
    node_id: NodeId,
    state: DocumentStateView,
    participants: Vec<Participant>,
}

enum CollaborationSubmission {
    Operations(OperationRequest),
    DirtyDraft(DirtyDraftRequest),
}

impl CollaborationSubmission {
    fn client_id(&self) -> &str {
        match self {
            Self::Operations(request) => &request.client_id,
            Self::DirtyDraft(request) => &request.client_id,
        }
    }

    fn operation_id(&self) -> &str {
        match self {
            Self::Operations(request) => &request.operation_id,
            Self::DirtyDraft(request) => &request.operation_id,
        }
    }

    fn epoch(&self) -> u64 {
        match self {
            Self::Operations(request) => request.epoch,
            Self::DirtyDraft(request) => request.epoch,
        }
    }

    fn base_version(&self) -> u64 {
        match self {
            Self::Operations(request) => request.base_version,
            Self::DirtyDraft(request) => request.base_version,
        }
    }

    fn base_revision(&self) -> &str {
        match self {
            Self::Operations(request) => &request.base_revision,
            Self::DirtyDraft(request) => &request.base_revision,
        }
    }

    fn digest(&self) -> String {
        match self {
            Self::Operations(request) => collaboration::operation_request_digest(request),
            Self::DirtyDraft(request) => collaboration::dirty_draft_request_digest(request),
        }
    }

    fn validate(&self) -> Result<(), CollaborationError> {
        match self {
            Self::Operations(request) => collaboration::validate_operation_request(request),
            Self::DirtyDraft(request) => collaboration::validate_dirty_draft_request(request),
        }
    }

    fn prepare(
        &self,
        document: &mut CollaborationDocument,
        actor_id: &str,
    ) -> Result<collaboration::PreparedOperation, CollaborationError> {
        match self {
            Self::Operations(request) => document.prepare_operation(actor_id, request),
            Self::DirtyDraft(request) => document.prepare_dirty_draft(actor_id, request),
        }
    }
}

async fn collaboration_snapshot(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
) -> Result<Json<CollaborationSnapshotResponse>, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, node_id, false)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let actor_id = collaboration_actor_uuid(&principal);
    let (document_state, source) = {
        let mut documents = state.collaboration_documents.lock().await;
        let document = ensure_collaboration_document(&state, node_id, &snapshot, &mut documents)?;
        (document.state(), document.source().to_owned())
    };
    let participants = state.presence.lock().await.for_node(node_id, unix_now()?);
    Ok(Json(CollaborationSnapshotResponse {
        wire_version: collaboration::WIRE_VERSION,
        node_id,
        actor_id,
        state: document_state,
        source,
        participants,
    }))
}

async fn commit_collaboration_operation(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<OperationRequest>,
) -> Result<Response, ApiError> {
    commit_collaboration_submission(
        &state,
        &principal,
        &node_id,
        CollaborationSubmission::Operations(request),
    )
    .await
}

async fn commit_collaboration_draft(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<DirtyDraftRequest>,
) -> Result<Response, ApiError> {
    commit_collaboration_submission(
        &state,
        &principal,
        &node_id,
        CollaborationSubmission::DirtyDraft(request),
    )
    .await
}

#[expect(
    clippy::too_many_lines,
    reason = "the commit path keeps authorization, transform, Core authority, durable dedupe, audit, and event ordering visible in one reviewable sequence"
)]
async fn commit_collaboration_submission(
    state: &ServerState,
    principal: &SessionPrincipal,
    node_id_text: &str,
    submission: CollaborationSubmission,
) -> Result<Response, ApiError> {
    require_content_write(principal)?;
    let node_id = parse_node_id(node_id_text)?;
    let path = state.authorized_node_path(principal, node_id, true)?;
    let actor_id = collaboration_actor_uuid(principal);
    if let Err(error) = submission.validate() {
        let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
        let state_view = collaboration_state_for_snapshot(state, node_id, &snapshot).await?;
        return Ok(collaboration_rejection_response(
            node_id,
            &actor_id,
            &submission,
            state_view,
            error,
        ));
    }
    let request_digest = submission.digest();
    if let Some(receipt) = state
        .control_plane
        .collaboration_receipt(submission.operation_id())
        .map_err(ApiError::ControlPlane)?
    {
        let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
        let state_view = collaboration_state_for_snapshot(state, node_id, &snapshot).await?;
        return Ok(collaboration_replay_response(
            node_id,
            principal,
            &actor_id,
            &submission,
            &request_digest,
            receipt,
            state_view,
        ));
    }

    let _commit_guard = state.commits.lock().await;
    if let Some(receipt) = state
        .control_plane
        .collaboration_receipt(submission.operation_id())
        .map_err(ApiError::ControlPlane)?
    {
        let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
        let state_view = collaboration_state_for_snapshot(state, node_id, &snapshot).await?;
        return Ok(collaboration_replay_response(
            node_id,
            principal,
            &actor_id,
            &submission,
            &request_digest,
            receipt,
            state_view,
        ));
    }
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let mut documents = state.collaboration_documents.lock().await;
    let document = ensure_collaboration_document(state, node_id, &snapshot, &mut documents)?;
    let prepared = match submission.prepare(document, &actor_id) {
        Ok(prepared) => prepared,
        Err(error) => {
            if error.freezes_document() {
                persist_collaboration_state(state, node_id, &document.state())?;
                publish_collaboration_state_event(
                    state,
                    node_id,
                    &document.state(),
                    "conflict",
                    Some(&actor_id),
                    Some(submission.client_id()),
                    Some(submission.operation_id()),
                );
            }
            return Ok(collaboration_rejection_response(
                node_id,
                &actor_id,
                &submission,
                document.state(),
                error,
            ));
        }
    };
    let base_revision =
        DocumentRevision::parse(&prepared.applied_base_revision).map_err(ApiError::Document)?;
    let plan = match plan_document_edit(&path, &base_revision, prepared.document_edits()) {
        Ok(plan) => plan,
        Err(DocumentError::StaleRevision { .. }) => {
            let current = read_node_document(&path).map_err(ApiError::Document)?;
            let _ = document.reconcile_canonical(&current.revision.to_string(), &current.source);
            persist_collaboration_state(state, node_id, &document.state())?;
            publish_collaboration_state_event(
                state,
                node_id,
                &document.state(),
                "external-edit",
                Some(&actor_id),
                Some(submission.client_id()),
                Some(submission.operation_id()),
            );
            return Ok(collaboration_rejection_response(
                node_id,
                &actor_id,
                &submission,
                document.state(),
                CollaborationError::ExternalEdit,
            ));
        }
        Err(error) => return Err(ApiError::Document(error)),
    };
    if plan.next_source() != prepared.next_source {
        return Err(ApiError::CommitOutcomeIndeterminate);
    }

    let next_revision = plan.next_revision.to_string();
    let result_version = prepared.applied_base_version.saturating_add(1);
    let transaction_id = submission.operation_id().to_owned();
    let audit_time = unix_now()?;
    let detail = format!(
        "actor={actor_id};client={};operation={};transaction={transaction_id};node={node_id};base={};result={next_revision}",
        submission.client_id(),
        submission.operation_id(),
        prepared.request_base_revision,
    );
    let audit_intent = state
        .control_plane
        .begin_collaboration_intent(
            principal,
            &NewCollaborationIntent {
                actor_id: &actor_id,
                client_id: submission.client_id(),
                operation_id: submission.operation_id(),
                node_id: &node_id.to_string(),
                epoch: submission.epoch(),
                base_version: submission.base_version(),
                base_revision: submission.base_revision(),
                applied_base_version: prepared.applied_base_version,
                applied_base_revision: &prepared.applied_base_revision,
                result_version,
                result_revision: &next_revision,
                request_digest: &prepared.request_digest,
                transaction_id: &transaction_id,
                detail: &detail,
            },
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let committed = match commit_document_edit(&plan) {
        Ok(committed) => committed,
        Err(error) => match document_authority(state, node_id, &plan.next_revision) {
            DocumentAuthority::Committed(committed) => committed,
            DocumentAuthority::Rejected => {
                let _ = state
                    .control_plane
                    .cancel_collaboration_intent(&audit_intent);
                return Err(ApiError::Document(error));
            }
            DocumentAuthority::Indeterminate => {
                return Err(ApiError::CommitOutcomeIndeterminate);
            }
        },
    };
    if plan.changed {
        let _ = state.changes.send(ChangeEvent {
            node_id,
            revision: committed.revision.clone(),
        });
    }
    let transformed = prepared.request_base_revision != prepared.applied_base_revision
        || prepared.request_base_version != prepared.applied_base_version;
    let response_operations = prepared.operations.clone();
    let request_base_revision = prepared.request_base_revision.clone();
    let request_base_version = prepared.request_base_version;
    let applied_base_revision = prepared.applied_base_revision.clone();
    let applied_base_version = prepared.applied_base_version;
    let document_state = document.accept(prepared, committed.revision.to_string());
    let audit_recorded = state
        .control_plane
        .finalize_collaboration_intent(&audit_intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    publish_collaboration_state_event(
        state,
        node_id,
        &document_state,
        "operation-committed",
        Some(&actor_id),
        Some(submission.client_id()),
        Some(submission.operation_id()),
    );
    Ok(Json(CollaborationOperationResponse {
        wire_version: collaboration::WIRE_VERSION,
        status: "accepted",
        node_id,
        actor_id,
        client_id: submission.client_id().to_owned(),
        operation_id: submission.operation_id().to_owned(),
        transaction_id,
        request_base_revision,
        request_base_version,
        applied_base_revision,
        applied_base_version,
        result_revision: committed.revision.to_string(),
        state: document_state,
        transformed,
        operations: response_operations,
        audit_recorded,
        error_code: None,
    })
    .into_response())
}

fn collaboration_replay_response(
    node_id: NodeId,
    principal: &SessionPrincipal,
    actor_id: &str,
    submission: &CollaborationSubmission,
    request_digest: &str,
    receipt: CollaborationReceipt,
    state: DocumentStateView,
) -> Response {
    let matches = receipt.actor_scope == principal.actor_scope
        && receipt.actor_id == actor_id
        && receipt.client_id == submission.client_id()
        && receipt.node_id == node_id.to_string()
        && receipt.epoch == submission.epoch()
        && receipt.base_version == submission.base_version()
        && receipt.base_revision == submission.base_revision()
        && receipt.request_digest == request_digest;
    let status = if matches {
        StatusCode::OK
    } else {
        StatusCode::CONFLICT
    };
    let transformed = matches
        && (receipt.base_revision != receipt.applied_base_revision
            || receipt.base_version != receipt.applied_base_version);
    let response = CollaborationOperationResponse {
        wire_version: collaboration::WIRE_VERSION,
        status: if matches { "replayed" } else { "conflict" },
        node_id,
        actor_id: actor_id.to_owned(),
        client_id: submission.client_id().to_owned(),
        operation_id: submission.operation_id().to_owned(),
        transaction_id: if matches {
            receipt.transaction_id
        } else {
            String::new()
        },
        request_base_revision: submission.base_revision().to_owned(),
        request_base_version: submission.base_version(),
        applied_base_revision: if matches {
            receipt.applied_base_revision
        } else {
            String::new()
        },
        applied_base_version: if matches {
            receipt.applied_base_version
        } else {
            0
        },
        result_revision: if matches {
            receipt.result_revision
        } else {
            state.revision.clone()
        },
        state,
        transformed,
        operations: Vec::new(),
        audit_recorded: matches,
        error_code: (!matches).then_some(CollaborationError::ReplayMismatch.code()),
    };
    (status, Json(response)).into_response()
}

fn collaboration_rejection_response(
    node_id: NodeId,
    actor_id: &str,
    submission: &CollaborationSubmission,
    state: DocumentStateView,
    error: CollaborationError,
) -> Response {
    let status = match error {
        CollaborationError::UnsupportedWireVersion
        | CollaborationError::InvalidClientId
        | CollaborationError::InvalidOperationId
        | CollaborationError::InvalidOperations
        | CollaborationError::InvalidCursor => StatusCode::UNPROCESSABLE_ENTITY,
        CollaborationError::ReplayMismatch
        | CollaborationError::EpochMismatch
        | CollaborationError::VersionMismatch
        | CollaborationError::HistoryUnavailable
        | CollaborationError::OverlappingConcurrentEdit
        | CollaborationError::Frozen
        | CollaborationError::ExternalEdit => StatusCode::CONFLICT,
    };
    (
        status,
        Json(CollaborationOperationResponse {
            wire_version: collaboration::WIRE_VERSION,
            status: if error == CollaborationError::HistoryUnavailable {
                "resync_required"
            } else if matches!(
                error,
                CollaborationError::Frozen | CollaborationError::ExternalEdit
            ) {
                "frozen"
            } else {
                "conflict"
            },
            node_id,
            actor_id: actor_id.to_owned(),
            client_id: submission.client_id().to_owned(),
            operation_id: submission.operation_id().to_owned(),
            transaction_id: String::new(),
            request_base_revision: submission.base_revision().to_owned(),
            request_base_version: submission.base_version(),
            applied_base_revision: state.revision.clone(),
            applied_base_version: state.version,
            result_revision: state.revision.clone(),
            state,
            transformed: false,
            operations: Vec::new(),
            audit_recorded: false,
            error_code: Some(error.code()),
        }),
    )
        .into_response()
}

async fn update_collaboration_presence(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<PresenceRequest>,
) -> Result<Response, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, node_id, false)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let actor_id = collaboration_actor_uuid(&principal);
    let (document_state, source) = {
        let mut documents = state.collaboration_documents.lock().await;
        let document = ensure_collaboration_document(&state, node_id, &snapshot, &mut documents)?;
        (document.state(), document.source().to_owned())
    };
    let participants = match state.presence.lock().await.upsert(
        node_id,
        &actor_id,
        principal.role.as_str(),
        &source,
        &document_state,
        &request,
        unix_now()?,
    ) {
        Ok(participants) => participants,
        Err(error) => {
            return Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "wireVersion": collaboration::WIRE_VERSION,
                    "status": "rejected",
                    "errorCode": error.code(),
                    "nodeId": node_id,
                    "state": document_state,
                })),
            )
                .into_response());
        }
    };
    publish_presence_event(
        &state,
        node_id,
        &document_state,
        &actor_id,
        &request.client_id,
        participants.clone(),
    );
    Ok(Json(CollaborationPresenceResponse {
        wire_version: collaboration::WIRE_VERSION,
        node_id,
        state: document_state,
        participants,
    })
    .into_response())
}

async fn leave_collaboration_presence(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath((node_id, client_id)): ApiPath<(String, String)>,
) -> Result<Response, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, node_id, false)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let actor_id = collaboration_actor_uuid(&principal);
    let document_state = collaboration_state_for_snapshot(&state, node_id, &snapshot).await?;
    let participants =
        match state
            .presence
            .lock()
            .await
            .leave(node_id, &actor_id, &client_id, unix_now()?)
        {
            Ok(participants) => participants,
            Err(error) => {
                return Ok((
                    StatusCode::UNPROCESSABLE_ENTITY,
                    Json(serde_json::json!({
                        "wireVersion": collaboration::WIRE_VERSION,
                        "status": "rejected",
                        "errorCode": error.code(),
                    })),
                )
                    .into_response());
            }
        };
    publish_presence_event(
        &state,
        node_id,
        &document_state,
        &actor_id,
        &client_id,
        participants.clone(),
    );
    Ok(Json(CollaborationPresenceResponse {
        wire_version: collaboration::WIRE_VERSION,
        node_id,
        state: document_state,
        participants,
    })
    .into_response())
}

async fn acknowledge_collaboration_resync(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<ResyncRequest>,
) -> Result<Response, ApiError> {
    require_content_write(&principal)?;
    let node_id = parse_node_id(&node_id)?;
    let _commit_guard = state.commits.lock().await;
    let path = state.authorized_node_path(&principal, node_id, true)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let actor_id = collaboration_actor_uuid(&principal);
    let mut documents = state.collaboration_documents.lock().await;
    let document = ensure_collaboration_document(&state, node_id, &snapshot, &mut documents)?;
    let document_state = match document.acknowledge_resync(&request) {
        Ok(state_view) => state_view,
        Err(error) => {
            return Ok((
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "wireVersion": collaboration::WIRE_VERSION,
                    "status": "resync_required",
                    "errorCode": error.code(),
                    "nodeId": node_id,
                    "state": document.state(),
                })),
            )
                .into_response());
        }
    };
    persist_collaboration_state(&state, node_id, &document_state)?;
    publish_collaboration_state_event(
        &state,
        node_id,
        &document_state,
        "resynced",
        Some(&actor_id),
        Some(&request.client_id),
        None,
    );
    Ok(Json(serde_json::json!({
        "wireVersion": collaboration::WIRE_VERSION,
        "status": "ready",
        "nodeId": node_id,
        "actorId": actor_id,
        "state": document_state,
        "source": document.source(),
    }))
    .into_response())
}

fn ensure_collaboration_document<'a>(
    state: &ServerState,
    node_id: NodeId,
    snapshot: &weftext_core::DocumentSnapshot,
    documents: &'a mut BTreeMap<NodeId, CollaborationDocument>,
) -> Result<&'a mut CollaborationDocument, ApiError> {
    if let std::collections::btree_map::Entry::Vacant(entry) = documents.entry(node_id) {
        let record = state
            .control_plane
            .collaboration_document(
                &node_id.to_string(),
                &snapshot.revision.to_string(),
                unix_now()?,
            )
            .map_err(ApiError::ControlPlane)?;
        let frozen_reason = match record.frozen_reason.as_deref() {
            None => None,
            Some("external_edit") => Some("external_edit"),
            Some("overlapping_concurrent_edit") => Some("overlapping_concurrent_edit"),
            Some(_) => return Err(ApiError::ControlPlane(AuthError::InvalidControlPlane)),
        };
        entry.insert(CollaborationDocument::new(
            record.epoch,
            record.version,
            record.checkpoint_revision,
            snapshot.source.clone(),
            frozen_reason,
            record.expected_revision,
        ));
    }
    let document = documents
        .get_mut(&node_id)
        .ok_or(ApiError::CommitOutcomeIndeterminate)?;
    if document
        .reconcile_canonical(&snapshot.revision.to_string(), &snapshot.source)
        .is_err()
    {
        persist_collaboration_state(state, node_id, &document.state())?;
        publish_collaboration_state_event(
            state,
            node_id,
            &document.state(),
            "external-edit",
            None,
            None,
            None,
        );
    }
    Ok(document)
}

async fn collaboration_state_for_snapshot(
    state: &ServerState,
    node_id: NodeId,
    snapshot: &weftext_core::DocumentSnapshot,
) -> Result<DocumentStateView, ApiError> {
    let mut documents = state.collaboration_documents.lock().await;
    Ok(ensure_collaboration_document(state, node_id, snapshot, &mut documents)?.state())
}

fn persist_collaboration_state(
    state: &ServerState,
    node_id: NodeId,
    document: &DocumentStateView,
) -> Result<(), ApiError> {
    state
        .control_plane
        .store_collaboration_document(
            &CollaborationDocumentRecord {
                node_id: node_id.to_string(),
                epoch: document.epoch,
                version: document.version,
                checkpoint_revision: document.revision.clone(),
                frozen_reason: document.reason.map(str::to_owned),
                expected_revision: document
                    .comparison
                    .as_ref()
                    .map(|comparison| comparison.expected_revision.clone()),
            },
            unix_now()?,
        )
        .map_err(ApiError::ControlPlane)
}

fn publish_collaboration_state_event(
    state: &ServerState,
    node_id: NodeId,
    document: &DocumentStateView,
    event_type: &'static str,
    actor_id: Option<&str>,
    client_id: Option<&str>,
    operation_id: Option<&str>,
) {
    let _ = state.collaboration_events.send(CollaborationEvent {
        wire_version: collaboration::WIRE_VERSION,
        event_type,
        node_id,
        epoch: document.epoch,
        version: document.version,
        revision: document.revision.clone(),
        actor_id: actor_id.map(str::to_owned),
        client_id: client_id.map(str::to_owned),
        operation_id: operation_id.map(str::to_owned),
        reason: document.reason,
        participants: None,
    });
}

fn publish_presence_event(
    state: &ServerState,
    node_id: NodeId,
    document: &DocumentStateView,
    actor_id: &str,
    client_id: &str,
    participants: Vec<Participant>,
) {
    let _ = state.collaboration_events.send(CollaborationEvent {
        wire_version: collaboration::WIRE_VERSION,
        event_type: "presence",
        node_id,
        epoch: document.epoch,
        version: document.version,
        revision: document.revision.clone(),
        actor_id: Some(actor_id.to_owned()),
        client_id: Some(client_id.to_owned()),
        operation_id: None,
        reason: None,
        participants: Some(participants),
    });
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnotationReadResponse {
    node_id: NodeId,
    workspace_revision: String,
    revision: DocumentRevision,
    store: AnnotationStore,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AnnotationMutationRequest {
    base_workspace_revision: String,
    base_revision: String,
    action: String,
    node_id: String,
    annotation_id: Option<String>,
    message_id: Option<String>,
    kind: Option<AnnotationKind>,
    target: Option<AnnotationTargetRequest>,
    appearance: Option<AnnotationAppearanceRequest>,
    body_source: Option<String>,
    suggested_source: Option<String>,
    labels: Option<Vec<String>>,
    timestamp: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AnnotationTargetRequest {
    Document(EmptyAnnotationTargetRequest),
    TextRange(TextRangeTargetRequest),
    InsertionPoint(InsertionPointTargetRequest),
    BlockAt(BlockTargetRequest),
    ResourceRegion(ResourceRegionTargetRequest),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EmptyAnnotationTargetRequest {}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextRangeTargetRequest {
    start: u64,
    end: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InsertionPointTargetRequest {
    position: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BlockTargetRequest {
    source_offset: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResourceRegionTargetRequest {
    resource_locator: String,
    resource_digest: String,
    media_kind: AnnotationResourceMediaKind,
    region: AnnotationResourceRegionRequest,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AnnotationResourceRegionRequest {
    Rect(RectRegionRequest),
    TimeRange(TimeRangeRegionRequest),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RectRegionRequest {
    page: Option<u32>,
    x_millionths: u32,
    y_millionths: u32,
    width_millionths: u32,
    height_millionths: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TimeRangeRegionRequest {
    start_milliseconds: u64,
    end_milliseconds: u64,
}

impl From<AnnotationResourceRegionRequest> for AnnotationResourceRegion {
    fn from(value: AnnotationResourceRegionRequest) -> Self {
        match value {
            AnnotationResourceRegionRequest::Rect(request) => Self::Rect {
                page: request.page,
                x_millionths: request.x_millionths,
                y_millionths: request.y_millionths,
                width_millionths: request.width_millionths,
                height_millionths: request.height_millionths,
            },
            AnnotationResourceRegionRequest::TimeRange(request) => Self::TimeRange {
                start_milliseconds: request.start_milliseconds,
                end_milliseconds: request.end_milliseconds,
            },
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AnnotationAppearanceRequest {
    mark: AnnotationMark,
    theme: Option<AnnotationColor>,
}

impl AnnotationAppearanceRequest {
    fn for_create(self) -> Result<AnnotationAppearance, ApiError> {
        if self.mark == AnnotationMark::None {
            return Err(ApiError::InvalidRequest(
                "create cannot use appearance.mark=none",
            ));
        }
        Ok(AnnotationAppearance {
            mark: self.mark,
            color: self
                .theme
                .ok_or(ApiError::InvalidRequest("appearance theme is required"))?,
        })
    }

    fn for_update(self) -> Result<Option<AnnotationAppearance>, ApiError> {
        if self.mark == AnnotationMark::None {
            if self.theme.is_some() {
                return Err(ApiError::InvalidRequest(
                    "appearance.mark=none cannot include theme",
                ));
            }
            Ok(None)
        } else {
            self.for_create().map(Some)
        }
    }
}

impl From<AnnotationTargetRequest> for AnnotationTargetIntent {
    fn from(value: AnnotationTargetRequest) -> Self {
        match value {
            AnnotationTargetRequest::Document(_) => Self::Document,
            AnnotationTargetRequest::TextRange(request) => Self::TextRange {
                start: request.start,
                end: request.end,
            },
            AnnotationTargetRequest::InsertionPoint(request) => Self::InsertionPoint {
                position: request.position,
            },
            AnnotationTargetRequest::BlockAt(request) => Self::BlockAt {
                source_offset: request.source_offset,
            },
            AnnotationTargetRequest::ResourceRegion(request) => Self::ResourceRegion {
                resource_locator: request.resource_locator,
                resource_digest: request.resource_digest,
                media_kind: request.media_kind,
                region: request.region.into(),
            },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnnotationCommitResponse {
    node_id: NodeId,
    base_workspace_revision: String,
    workspace_revision: String,
    revision: DocumentRevision,
    store: AnnotationStore,
    audit_recorded: bool,
}

async fn read_annotations(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
) -> Result<Json<AnnotationReadResponse>, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    let path = state.authorized_node_path(&principal, node_id, false)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let store = read_node_annotations_at_node_path(&path, node_id)
        .map_err(|_| ApiError::WorkspaceInvalid)?;
    let workspace_revision = actor_workspace_revision(&state, &principal)?;
    Ok(Json(AnnotationReadResponse {
        node_id,
        workspace_revision,
        revision: snapshot.revision,
        store,
    }))
}

#[expect(
    clippy::too_many_lines,
    reason = "annotation commit retains the established transaction ordering plus one collaboration notification"
)]
async fn commit_annotation_action(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<AnnotationMutationRequest>,
) -> Result<Json<AnnotationCommitResponse>, ApiError> {
    require_annotation_write(&principal)?;
    if request.action == "accept_suggestion" {
        require_content_write(&principal)?;
    }
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, true)?;
    let body_node_id = parse_node_id(&request.node_id)?;
    if body_node_id != node_id {
        return Err(ApiError::InvalidRequest(
            "annotation nodeId does not match the route node",
        ));
    }
    let supplied_workspace_revision = request.base_workspace_revision.clone();
    let expected_document_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let _commit_guard = state.commits.lock().await;
    let expected_workspace_revision =
        resolve_client_workspace_revision(&state, &principal, &supplied_workspace_revision)?;
    let path = state.authorized_node_path(&principal, node_id, true)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    if expected_document_revision != snapshot.revision {
        return Err(ApiError::Document(DocumentError::StaleRevision {
            expected: expected_document_revision,
            actual: snapshot.revision,
        }));
    }
    let sidecar_snapshot = capture_hosted_node_annotations(state.workspace_root(), node_id)?;
    let (action, event_type) =
        bind_annotation_action(request, sidecar_snapshot.store(), &principal)?;
    let plan = plan_annotation_action(state.workspace_root(), &sidecar_snapshot, action).map_err(
        |error| {
            if principal.role == SessionRole::Owner {
                ApiError::WorkspaceTransaction(error)
            } else {
                ApiError::WorkspaceWriteRejected
            }
        },
    )?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &supplied_workspace_revision,
        &expected_workspace_revision,
        &plan.base_revision,
    )?;
    let audit_time = unix_now()?;
    let audit_intent = state
        .control_plane
        .begin_audit_intent(
            &principal,
            event_type,
            &format!("node={node_id};plan={}", plan.plan_id),
            "workspace",
            "workspace",
            &format!("not:{}", plan.base_revision),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let committed = match commit_workspace_transaction(&plan) {
        Ok(committed) => committed,
        Err(error) => {
            if let Some(committed) = recover_workspace_commit(&plan)? {
                committed
            } else {
                let _ = state.control_plane.cancel_audit_intent(&audit_intent);
                return Err(if principal.role == SessionRole::Owner {
                    ApiError::WorkspaceTransaction(error)
                } else {
                    ApiError::WorkspaceWriteRejected
                });
            }
        }
    };
    let _ = state.control_plane.update_audit_intent_authority(
        &audit_intent,
        "workspace",
        "workspace",
        &committed.revision.to_string(),
    );
    let audit_recorded = state
        .control_plane
        .finalize_audit_intent(&audit_intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    let snapshot = read_node_document(&path).map_err(|_| ApiError::CommitOutcomeIndeterminate)?;
    let store = capture_hosted_node_annotations(state.workspace_root(), node_id)
        .map(AnnotationSidecarSnapshot::into_store)
        .map_err(|_| ApiError::CommitOutcomeIndeterminate)?;
    let _ = state.changes.send(ChangeEvent {
        node_id,
        revision: snapshot.revision.clone(),
    });
    let collaboration_state = collaboration_state_for_snapshot(&state, node_id, &snapshot).await?;
    publish_collaboration_state_event(
        &state,
        node_id,
        &collaboration_state,
        "annotation-mutated",
        Some(&collaboration_actor_uuid(&principal)),
        None,
        None,
    );
    let workspace_revision = actor_workspace_revision(&state, &principal)
        .map_err(|_| ApiError::CommitOutcomeIndeterminate)?;
    Ok(Json(AnnotationCommitResponse {
        node_id,
        base_workspace_revision: supplied_workspace_revision,
        workspace_revision,
        revision: snapshot.revision,
        store,
        audit_recorded,
    }))
}

fn require_annotation_write(principal: &SessionPrincipal) -> Result<(), ApiError> {
    if principal.role.can_write_annotations() {
        Ok(())
    } else {
        Err(ApiError::AuthorizationDenied)
    }
}

fn capture_hosted_node_annotations(
    workspace_root: &Path,
    node_id: NodeId,
) -> Result<AnnotationSidecarSnapshot, ApiError> {
    capture_annotation_sidecar_snapshot(
        workspace_root,
        node_id,
        AnnotationReplicaCompleteness::CompleteHostedWorkspace,
    )
    .map_err(|_| ApiError::WorkspaceInvalid)
}

#[allow(clippy::too_many_lines)]
fn bind_annotation_action(
    request: AnnotationMutationRequest,
    store: &AnnotationStore,
    principal: &SessionPrincipal,
) -> Result<(AnnotationAction, &'static str), ApiError> {
    let author_id_text = session_actor_uuid(principal);
    let author_name = principal.role.as_str().to_owned();
    Ok(match request.action.as_str() {
        "create" => {
            reject_annotation_fields(
                request.annotation_id.is_some() || request.message_id.is_some(),
            )?;
            let kind = request
                .kind
                .ok_or(ApiError::InvalidRequest("annotation kind is required"))?;
            let target = request
                .target
                .ok_or(ApiError::InvalidRequest("annotation target is required"))?;
            (
                AnnotationAction::Create {
                    kind,
                    target: target.into(),
                    appearance: request
                        .appearance
                        .map(AnnotationAppearanceRequest::for_create)
                        .transpose()?,
                    labels: request.labels.unwrap_or_default(),
                    body_source: request.body_source,
                    suggested_source: request.suggested_source,
                    author_id: author_id_text
                        .parse()
                        .map_err(|_| ApiError::ControlPlaneUnavailable)?,
                    author_name,
                    timestamp: request.timestamp,
                },
                "annotation_created",
            )
        }
        "reply" => {
            reject_annotation_fields(
                request.message_id.is_some()
                    || request.kind.is_some()
                    || request.target.is_some()
                    || request.appearance.is_some()
                    || request.suggested_source.is_some()
                    || request.labels.is_some(),
            )?;
            let body_source = request
                .body_source
                .ok_or(ApiError::InvalidRequest("bodySource is required"))?;
            (
                AnnotationAction::Reply {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    body_source,
                    author_id: author_id_text
                        .parse()
                        .map_err(|_| ApiError::ControlPlaneUnavailable)?,
                    author_name,
                    timestamp: request.timestamp,
                },
                "annotation_replied",
            )
        }
        "edit_message" => {
            reject_annotation_fields(
                request.kind.is_some()
                    || request.target.is_some()
                    || request.appearance.is_some()
                    || request.suggested_source.is_some()
                    || request.labels.is_some(),
            )?;
            let annotation = required_annotation(store, request.annotation_id.as_deref())?;
            let message_id = request
                .message_id
                .as_deref()
                .ok_or(ApiError::InvalidRequest("messageId is required"))?;
            let message = annotation
                .thread
                .iter()
                .find(|message| canonical_id_matches(message_id, &message.id.to_string()))
                .ok_or(ApiError::AnnotationUnavailable)?;
            let body_source = request
                .body_source
                .ok_or(ApiError::InvalidRequest("bodySource is required"))?;
            (
                AnnotationAction::EditMessage {
                    annotation_id: annotation.id,
                    message_id: message.id,
                    body_source,
                    author_id: author_id_text
                        .parse()
                        .map_err(|_| ApiError::ControlPlaneUnavailable)?,
                    timestamp: request.timestamp,
                },
                "annotation_message_edited",
            )
        }
        "set_appearance" => {
            reject_annotation_fields(
                request.message_id.is_some()
                    || request.kind.is_some()
                    || request.target.is_some()
                    || request.body_source.is_some()
                    || request.suggested_source.is_some()
                    || request.labels.is_some(),
            )?;
            (
                AnnotationAction::SetAppearance {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    appearance: request
                        .appearance
                        .ok_or(ApiError::InvalidRequest("appearance is required"))?
                        .for_update()?,
                    timestamp: request.timestamp,
                },
                "annotation_appearance_set",
            )
        }
        "set_labels" => {
            reject_annotation_fields(
                request.message_id.is_some()
                    || request.kind.is_some()
                    || request.target.is_some()
                    || request.appearance.is_some()
                    || request.body_source.is_some()
                    || request.suggested_source.is_some(),
            )?;
            let labels = request
                .labels
                .ok_or(ApiError::InvalidRequest("labels are required"))?;
            (
                AnnotationAction::SetLabels {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    labels,
                    timestamp: request.timestamp,
                },
                "annotation_labels_set",
            )
        }
        "resolve" => {
            reject_status_action_fields(&request)?;
            (
                AnnotationAction::SetResolved {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    resolved: true,
                    timestamp: request.timestamp,
                },
                "annotation_resolved",
            )
        }
        "reopen" => {
            reject_status_action_fields(&request)?;
            (
                AnnotationAction::SetResolved {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    resolved: false,
                    timestamp: request.timestamp,
                },
                "annotation_reopened",
            )
        }
        "reanchor" => {
            reject_status_action_fields(&request)?;
            (
                AnnotationAction::Reanchor {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    timestamp: request.timestamp,
                },
                "annotation_reanchored",
            )
        }
        "accept_suggestion" => {
            reject_status_action_fields(&request)?;
            (
                AnnotationAction::AcceptSuggestion {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    timestamp: request.timestamp,
                },
                "annotation_suggestion_accepted",
            )
        }
        "reject_suggestion" => {
            reject_status_action_fields(&request)?;
            (
                AnnotationAction::RejectSuggestion {
                    annotation_id: required_annotation(store, request.annotation_id.as_deref())?.id,
                    timestamp: request.timestamp,
                },
                "annotation_suggestion_rejected",
            )
        }
        _ => {
            return Err(ApiError::InvalidRequest(
                "annotation action is not a canonical snake_case action",
            ));
        }
    })
}

fn reject_status_action_fields(request: &AnnotationMutationRequest) -> Result<(), ApiError> {
    reject_annotation_fields(
        request.message_id.is_some()
            || request.kind.is_some()
            || request.target.is_some()
            || request.appearance.is_some()
            || request.body_source.is_some()
            || request.suggested_source.is_some()
            || request.labels.is_some(),
    )
}

fn reject_annotation_fields(reject: bool) -> Result<(), ApiError> {
    if reject {
        Err(ApiError::InvalidRequest(
            "annotation action contains fields that do not apply",
        ))
    } else {
        Ok(())
    }
}

fn required_annotation<'a>(
    store: &'a AnnotationStore,
    requested_id: Option<&str>,
) -> Result<&'a weftext_core::Annotation, ApiError> {
    requested_id
        .ok_or(ApiError::InvalidRequest("annotationId is required"))
        .and_then(|id| find_annotation(store, id))
}

fn find_annotation<'a>(
    store: &'a AnnotationStore,
    requested_id: &str,
) -> Result<&'a weftext_core::Annotation, ApiError> {
    store
        .annotations
        .iter()
        .find(|annotation| canonical_id_matches(requested_id, &annotation.id.to_string()))
        .ok_or(ApiError::AnnotationUnavailable)
}

fn canonical_id_matches(requested: &str, canonical: &str) -> bool {
    requested == canonical
}

fn session_actor_uuid(principal: &SessionPrincipal) -> String {
    stable_actor_uuid(
        b"weftext-server-annotation-actor-v1\0",
        &principal.actor_scope,
    )
}

fn collaboration_actor_uuid(principal: &SessionPrincipal) -> String {
    stable_actor_uuid(
        b"weftext-server-collaboration-actor-v1\0",
        &principal.actor_scope,
    )
}

fn stable_actor_uuid(domain: &[u8], actor_scope: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(actor_scope.as_bytes());
    let digest = hasher.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponse {
    query: String,
    results: Vec<weftext_core::WorkspaceSearchResult>,
}

async fn search(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiQuery(query): ApiQuery<SearchQuery>,
) -> Result<Json<SearchResponse>, ApiError> {
    if query.q.len() > 512 {
        return Err(ApiError::InvalidRequest("search query is too long"));
    }
    let results = if principal.role == SessionRole::Owner {
        search_workspace(state.workspace_root(), &query.q)
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        search_workspace_scoped(state.workspace_root(), &query.q, &scope)
    }
    .map_err(|_| ApiError::Search)?;
    Ok(Json(SearchResponse {
        query: query.q,
        results,
    }))
}

async fn citation_capabilities(
    Authenticated(_principal): Authenticated,
) -> Json<weftext_core::CitationPresentationCapabilities> {
    Json(citation_presentation_capabilities())
}

async fn validate_citations(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = citation_index(&state, &principal)?;
    let access = server_citation_scope(&index, &principal);
    let visible = visible_node_ids(&state, &principal)?;
    let inventory = scan_workspace(state.workspace_root());
    let mut components = Vec::with_capacity(inventory.nodes.len());
    let mut component_diagnostic_count = 0_usize;
    for node in &inventory.nodes {
        let Some(node_id) = node.id else {
            if principal.role == SessionRole::Owner {
                return Err(ApiError::WorkspaceInvalid);
            }
            continue;
        };
        if !visible.contains(&node_id) {
            continue;
        }
        let analysis = index
            .analyze_component(node_id, &access)
            .map_err(|_| ApiError::CitationUnavailable)?;
        component_diagnostic_count += analysis.diagnostics.len();
        components.push(analysis);
    }
    let reference_diagnostics = if principal.role == SessionRole::Owner {
        index.diagnostics()
    } else {
        &[]
    };
    let valid = reference_diagnostics.is_empty() && component_diagnostic_count == 0;
    Ok(Json(serde_json::json!({
        "valid": valid,
        "generation": index.generation(),
        "workspaceRevision": actor_workspace_revision(&state, &principal)?,
        "referenceDiagnostics": reference_diagnostics,
        "components": components,
    })))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CitationReferenceQuery {
    q: String,
    #[serde(default = "default_citation_search_limit")]
    limit: usize,
}

const fn default_citation_search_limit() -> usize {
    25
}

async fn search_citation_references(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiQuery(query): ApiQuery<CitationReferenceQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = citation_index(&state, &principal)?;
    let access = server_citation_scope(&index, &principal);
    let references = index
        .search_references(&query.q, &access, query.limit)
        .map_err(|error| ApiError::CitationRequest(error.to_string()))?;
    Ok(Json(serde_json::json!({
        "query": query.q,
        "references": references,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CitationDraftRequest {
    source: String,
    style_id: String,
    locale: String,
}

async fn analyze_citation_draft(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<CitationDraftRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, false)?;
    let index = citation_index(&state, &principal)?;
    let access = server_citation_scope(&index, &principal);
    let analysis = index
        .analyze_component_source(node_id, &request.source, &access)
        .map_err(|_| ApiError::CitationUnavailable)?;
    let compilation = index
        .collect_bibliography_input_for_source(node_id, &request.source, &access)
        .map_err(|_| ApiError::CitationUnavailable)?;
    let presentation = present_citations(&CitationPresentationRequest::new(
        CitationPresentationProfile::new(request.style_id, request.locale),
        compilation,
    ));
    let (presentation, presentation_failure) = match presentation {
        Ok(presentation) => (Some(presentation), None),
        Err(failure) => (None, Some(failure)),
    };
    Ok(Json(serde_json::json!({
        "authoring": analyze_citation_authoring_source(&request.source),
        "analysis": analysis,
        "presentation": presentation,
        "presentationFailure": presentation_failure,
    })))
}

struct PreparedCitationDocument {
    authoring: weftext_core::CitationAuthoringPlan,
    document: DocumentEditPlan,
    icon: Option<ResolvedNodeIcon>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CitationMacroEditRequest {
    base_revision: String,
    source: String,
    target: CitationEditTarget,
    intent: CitationMacroIntent,
}

fn prepare_citation_macro_edit(
    state: &ServerState,
    principal: &SessionPrincipal,
    node_id: NodeId,
    request: &CitationMacroEditRequest,
) -> Result<PreparedCitationDocument, ApiError> {
    require_content_write(principal)?;
    let path = state.authorized_node_path(principal, node_id, true)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let snapshot = read_node_document(&path).map_err(ApiError::Document)?;
    let index = citation_index(state, principal)?;
    let access = server_citation_scope(&index, principal);
    let authoring = plan_citation_macro_edit(
        &index,
        node_id,
        &request.source,
        &access,
        &request.target,
        &request.intent,
    )
    .map_err(ApiError::CitationAuthoring)?;
    let document = plan_document_edit(
        &path,
        &base_revision,
        [whole_source_edit(
            &snapshot.source,
            authoring.proposed_source.clone(),
        )],
    )
    .map_err(ApiError::Document)?;
    let icon = resolve_node_icon_from_source(document.next_source());
    Ok(PreparedCitationDocument {
        authoring,
        document,
        icon,
    })
}

async fn preview_citation_macro_edit(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<CitationMacroEditRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let prepared =
        prepare_citation_macro_edit(&state, &principal, parse_node_id(&node_id)?, &request)?;
    Ok(Json(citation_document_preview(&prepared)))
}

async fn commit_citation_macro_edit(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<CitationMacroEditRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    let _commit_guard = state.commits.lock().await;
    let prepared = prepare_citation_macro_edit(&state, &principal, node_id, &request)?;
    let audit_time = unix_now()?;
    let audit_intent = state
        .control_plane
        .begin_audit_intent(
            &principal,
            "citation_macro_edited",
            &format!("node={node_id}"),
            "document",
            &node_id.to_string(),
            &prepared.document.next_revision.to_string(),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let committed = match commit_document_edit(&prepared.document) {
        Ok(committed) => committed,
        Err(error) => match document_authority(&state, node_id, &prepared.document.next_revision) {
            DocumentAuthority::Committed(committed) => committed,
            DocumentAuthority::Rejected => {
                let _ = state.control_plane.cancel_audit_intent(&audit_intent);
                return Err(ApiError::Document(error));
            }
            DocumentAuthority::Indeterminate => {
                return Err(ApiError::CommitOutcomeIndeterminate);
            }
        },
    };
    let response =
        finish_committed_document(&prepared.document, committed, prepared.icon, &state.changes);
    let audit_recorded = state
        .control_plane
        .finalize_audit_intent(&audit_intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    Ok(Json(serde_json::json!({
        "commit": response,
        "auditRecorded": audit_recorded,
    })))
}

fn citation_document_preview(prepared: &PreparedCitationDocument) -> serde_json::Value {
    serde_json::json!({
        "baseRevision": prepared.document.base_revision,
        "draftRevision": prepared.authoring.base_revision,
        "nextRevision": prepared.document.next_revision,
        "changed": prepared.document.changed,
        "edit": prepared.authoring.edit,
        "proposedSource": prepared.authoring.proposed_source,
        "analysis": prepared.authoring.analysis,
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QueryExecuteRequest {
    source: String,
    block_index: usize,
    context: QueryEvaluationContext,
}

async fn execute_query(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiJson(request): ApiJson<QueryExecuteRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = if principal.role == SessionRole::Owner {
        QueryWorkspaceIndex::rebuild(state.workspace_root())
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        QueryWorkspaceIndex::rebuild_scoped(state.workspace_root(), &scope)
    }
    .map_err(|_| ApiError::QueryUnavailable)?;
    let access = if principal.role == SessionRole::Owner {
        QueryAccessScope::complete(index.node_ids())
    } else {
        QueryAccessScope::filtered(index.node_ids())
    };
    let execution = index
        .execute_source(
            &request.source,
            request.block_index,
            &access,
            &request.context,
        )
        .map_err(|error| query_api_error(&error))?;
    Ok(Json(serde_json::json!({
        "valid": execution.result.is_some(),
        "workspaceRevision": actor_workspace_revision(&state, &principal)?,
        "execution": execution,
    })))
}

async fn validate_tasks(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<serde_json::Value>, ApiError> {
    let index = if principal.role == SessionRole::Owner {
        TaskWorkspaceIndex::rebuild(state.workspace_root())
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        TaskWorkspaceIndex::rebuild_scoped(state.workspace_root(), &scope)
    }
    .map_err(|_| ApiError::TaskUnavailable)?;
    let occurrences = index.occurrences();
    let diagnostics = index.diagnostics();
    Ok(Json(serde_json::json!({
        "valid": diagnostics.is_empty(),
        "generation": index.generation(),
        "occurrences": occurrences,
        "diagnostics": diagnostics,
    })))
}

async fn inspect_node_tasks(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, false)?;
    let index = if principal.role == SessionRole::Owner {
        TaskWorkspaceIndex::rebuild(state.workspace_root())
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        TaskWorkspaceIndex::rebuild_scoped(state.workspace_root(), &scope)
    }
    .map_err(|_| ApiError::TaskUnavailable)?;
    let occurrences = index.occurrences_for_node(node_id).collect::<Vec<_>>();
    let diagnostics = index
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            diagnostic.node_id == node_id
                && diagnostic.related_node_ids.iter().all(|related| {
                    index
                        .occurrences()
                        .iter()
                        .any(|occurrence| occurrence.node_id == *related)
                })
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::json!({
        "nodeId": node_id,
        "occurrences": occurrences,
        "diagnostics": diagnostics,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskEditRequest {
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    intent: TaskEditIntent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskRecurrenceRequest {
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    context: TaskRecurrenceCompletionContext,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TaskDependenciesRequest {
    base_workspace_revision: String,
    base_revision: String,
    target: TaskEditTarget,
    dependencies: Vec<TaskId>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectedWorkspaceDocumentChange {
    node_id: NodeId,
    path: String,
    base_revision: DocumentRevision,
    next_revision: DocumentRevision,
    edit_count: u64,
}

fn projected_task_document_changes(
    state: &ServerState,
    principal: &SessionPrincipal,
    plan: &weftext_core::WorkspaceTransactionPlan,
) -> Result<Vec<ProjectedWorkspaceDocumentChange>, ApiError> {
    let scope = (principal.role != SessionRole::Owner)
        .then(|| authorized_read_scope(state, principal))
        .transpose()?;
    plan.document_changes
        .iter()
        .map(|change| {
            let path = if let Some(scope) = &scope {
                if !scope.allows(change.node_id) {
                    return Err(ApiError::TaskTransactionRejected);
                }
                let node_path = state.node_path(change.node_id)?;
                let name = node_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or(ApiError::TaskTransactionRejected)?;
                canonical_document_locator(
                    scope
                        .locator(change.node_id)
                        .ok_or(ApiError::TaskTransactionRejected)?,
                    name,
                )
            } else {
                change.path.clone()
            };
            Ok(ProjectedWorkspaceDocumentChange {
                node_id: change.node_id,
                path,
                base_revision: change.base_revision.clone(),
                next_revision: change.next_revision.clone(),
                edit_count: change.edit_count,
            })
        })
        .collect()
}

async fn preview_task_edit(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<TaskEditRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, true)?;
    let workspace_revision =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let plan = if principal.role == SessionRole::Owner {
        plan_task_edit_transaction(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.intent,
        )
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        plan_task_edit_transaction_scoped(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.intent,
            &scope,
        )
    }
    .map_err(|error| map_task_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &workspace_revision,
        &plan.workspace_transaction().base_revision,
    )?;
    let document_changes =
        projected_task_document_changes(&state, &principal, plan.workspace_transaction())?;
    let base_actor_revision = actor_workspace_revision(&state, &principal)?;
    let response = serde_json::json!({
        "planId": plan.workspace_transaction().plan_id,
        "baseWorkspaceRevision": base_actor_revision.clone(),
        "nodeId": node_id,
        "authoring": &plan.authoring,
        "documentChanges": document_changes,
    });
    stage_task_plan(
        &state,
        &principal,
        base_actor_revision,
        PendingTaskTransaction::Edit(Box::new(plan)),
    )
    .await?;
    Ok(Json(response))
}

async fn preview_task_recurrence(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<TaskRecurrenceRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, true)?;
    let workspace_revision =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let plan = if principal.role == SessionRole::Owner {
        plan_task_recurrence_transaction(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.context,
        )
    } else {
        let scope = authorized_read_scope(&state, &principal)?;
        plan_task_recurrence_transaction_scoped(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.context,
            &scope,
        )
    }
    .map_err(|error| map_task_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &workspace_revision,
        &plan.workspace_transaction().base_revision,
    )?;
    let document_changes =
        projected_task_document_changes(&state, &principal, plan.workspace_transaction())?;
    let base_actor_revision = actor_workspace_revision(&state, &principal)?;
    let response = serde_json::json!({
        "planId": plan.workspace_transaction().plan_id,
        "baseWorkspaceRevision": base_actor_revision.clone(),
        "nodeId": node_id,
        "completion": &plan.completion,
        "documentChanges": document_changes,
    });
    stage_task_plan(
        &state,
        &principal,
        base_actor_revision,
        PendingTaskTransaction::Recurrence(Box::new(plan)),
    )
    .await?;
    Ok(Json(response))
}

async fn preview_task_dependencies(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(node_id): ApiPath<String>,
    ApiJson(request): ApiJson<TaskDependenciesRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    let node_id = parse_node_id(&node_id)?;
    state.authorized_node_path(&principal, node_id, true)?;
    let workspace_revision =
        resolve_client_workspace_revision(&state, &principal, &request.base_workspace_revision)?;
    let base_revision =
        DocumentRevision::parse(&request.base_revision).map_err(ApiError::Document)?;
    let scope = (principal.role != SessionRole::Owner)
        .then(|| authorized_read_scope(&state, &principal))
        .transpose()?;
    let index = if let Some(scope) = &scope {
        TaskWorkspaceIndex::rebuild_scoped(state.workspace_root(), scope)
    } else {
        TaskWorkspaceIndex::rebuild(state.workspace_root())
    }
    .map_err(|_| ApiError::TaskUnavailable)?;
    if request.dependencies.iter().any(|dependency| {
        !index.occurrences().iter().any(|occurrence| {
            occurrence
                .task
                .metadata
                .as_ref()
                .is_some_and(|metadata| metadata.id == *dependency)
        })
    }) {
        return Err(ApiError::TaskTransactionRejected);
    }
    let plan = if let Some(scope) = &scope {
        plan_task_dependency_transaction_scoped(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.dependencies,
            scope,
        )
    } else {
        plan_task_dependency_transaction(
            state.workspace_root(),
            node_id,
            &base_revision,
            &request.target,
            &request.dependencies,
        )
    }
    .map_err(|error| map_task_transaction_error(&principal, error))?;
    require_planned_workspace_revision(
        &state,
        &principal,
        &request.base_workspace_revision,
        &workspace_revision,
        &plan.workspace_transaction().base_revision,
    )?;
    let document_changes =
        projected_task_document_changes(&state, &principal, plan.workspace_transaction())?;
    let base_actor_revision = actor_workspace_revision(&state, &principal)?;
    let response = serde_json::json!({
        "planId": plan.workspace_transaction().plan_id,
        "baseWorkspaceRevision": base_actor_revision.clone(),
        "nodeId": node_id,
        "dependencies": &plan.dependencies,
        "authoring": &plan.authoring,
        "documentChanges": document_changes,
    });
    stage_task_plan(
        &state,
        &principal,
        base_actor_revision,
        PendingTaskTransaction::Dependencies(Box::new(plan)),
    )
    .await?;
    Ok(Json(response))
}

async fn stage_task_plan(
    state: &ServerState,
    principal: &SessionPrincipal,
    base_actor_revision: String,
    transaction: PendingTaskTransaction,
) -> Result<(), ApiError> {
    let now = unix_now()?;
    let plan_id = match &transaction {
        PendingTaskTransaction::Edit(plan) => plan.workspace_transaction().plan_id.clone(),
        PendingTaskTransaction::Recurrence(plan) => plan.workspace_transaction().plan_id.clone(),
        PendingTaskTransaction::Dependencies(plan) => plan.workspace_transaction().plan_id.clone(),
    };
    let mut plans = state.task_plans.lock().await;
    plans.retain(|_, pending| pending.expires_at > now);
    if plans.len() >= MAX_PENDING_TASK_PLANS {
        return Err(ApiError::TaskPlanLimit);
    }
    plans.insert(
        plan_id,
        PendingTaskPlan {
            session_id: principal.session_id.clone(),
            expires_at: now + TASK_PLAN_TTL_SECONDS,
            base_actor_revision,
            transaction,
        },
    );
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn commit_task_transaction(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
    ApiPath(plan_id): ApiPath<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_structure_write(&principal)?;
    if NodeId::from_str(&plan_id).is_err() {
        return Err(ApiError::TaskPlanUnavailable);
    }
    let _commit_guard = state.commits.lock().await;
    let now = unix_now()?;
    let pending = {
        let mut plans = state.task_plans.lock().await;
        plans.retain(|_, pending| pending.expires_at > now);
        if plans
            .get(&plan_id)
            .is_none_or(|pending| pending.session_id != principal.session_id)
        {
            return Err(ApiError::TaskPlanUnavailable);
        }
        plans
            .remove(&plan_id)
            .ok_or(ApiError::TaskPlanUnavailable)?
    };
    let pending_node_id = match &pending.transaction {
        PendingTaskTransaction::Edit(plan) => plan.node_id,
        PendingTaskTransaction::Recurrence(plan) => plan.node_id,
        PendingTaskTransaction::Dependencies(plan) => plan.node_id,
    };
    state.authorized_node_path(&principal, pending_node_id, true)?;
    let base_actor_revision = pending.base_actor_revision;
    let (committed, node_id, revision, audit_recorded, result) = match pending.transaction {
        PendingTaskTransaction::Edit(plan) => {
            let node_id = plan.node_id;
            let revision = task_plan_revision(plan.workspace_transaction())?;
            let result = serde_json::json!({
                "task": &plan.authoring.target,
                "assignedId": plan.authoring.assigned_id,
            });
            let (committed, audit_recorded) = commit_task_action_with_audit(
                &state,
                &principal,
                "task_edited",
                node_id,
                &revision,
                &plan_id,
                plan.workspace_transaction(),
                || commit_task_edit_transaction(&plan),
            )?;
            (committed, node_id, revision, audit_recorded, result)
        }
        PendingTaskTransaction::Recurrence(plan) => {
            let node_id = plan.node_id;
            let revision = task_plan_revision(plan.workspace_transaction())?;
            let result = serde_json::json!({
                "completedTask": &plan.completion.completed_task,
                "nextTask": &plan.completion.next_task,
                "nextTaskId": plan.completion.next_task_id,
                "stopped": plan.completion.stopped,
            });
            let (committed, audit_recorded) = commit_task_action_with_audit(
                &state,
                &principal,
                "task_recurrence_completed",
                node_id,
                &revision,
                &plan_id,
                plan.workspace_transaction(),
                || commit_task_recurrence_transaction(&plan),
            )?;
            (committed, node_id, revision, audit_recorded, result)
        }
        PendingTaskTransaction::Dependencies(plan) => {
            let node_id = plan.node_id;
            let revision = task_plan_revision(plan.workspace_transaction())?;
            let result = serde_json::json!({
                "task": &plan.authoring.target,
                "assignedId": plan.authoring.assigned_id,
                "dependencies": &plan.dependencies,
            });
            let (committed, audit_recorded) = commit_task_action_with_audit(
                &state,
                &principal,
                "task_dependencies_replaced",
                node_id,
                &revision,
                &plan_id,
                plan.workspace_transaction(),
                || commit_task_dependency_transaction(&plan),
            )?;
            (committed, node_id, revision, audit_recorded, result)
        }
    };
    let _ = state.changes.send(ChangeEvent { node_id, revision });
    let commit_response = if principal.role == SessionRole::Owner {
        serde_json::to_value(&committed).map_err(|_| ApiError::CommitOutcomeIndeterminate)?
    } else {
        let revision = actor_workspace_revision(&state, &principal)
            .map_err(|_| ApiError::CommitOutcomeIndeterminate)?;
        serde_json::json!({
            "planId": committed.plan_id,
            "action": committed.action,
            "baseRevision": base_actor_revision,
            "revision": revision,
            "pathChanges": [],
            "importAuthority": null,
        })
    };
    Ok(Json(serde_json::json!({
        "commit": commit_response,
        "nodeId": node_id,
        "result": result,
        "auditRecorded": audit_recorded,
    })))
}

#[allow(clippy::too_many_arguments)]
fn commit_task_action_with_audit(
    state: &ServerState,
    principal: &SessionPrincipal,
    event_type: &str,
    node_id: NodeId,
    expected_revision: &DocumentRevision,
    plan_id: &str,
    workspace_plan: &WorkspaceTransactionPlan,
    commit: impl FnOnce() -> Result<CommittedWorkspaceTransaction, WorkspaceTransactionError>,
) -> Result<(CommittedWorkspaceTransaction, bool), ApiError> {
    let audit_time = unix_now()?;
    let intent = state
        .control_plane
        .begin_audit_intent(
            principal,
            event_type,
            &format!("plan={plan_id};node={node_id}"),
            "document",
            &node_id.to_string(),
            &expected_revision.to_string(),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let committed = match commit() {
        Ok(committed) => committed,
        Err(error) => {
            if let Some(committed) = recover_workspace_commit(workspace_plan)? {
                committed
            } else {
                let _ = state.control_plane.cancel_audit_intent(&intent);
                return Err(if principal.role == SessionRole::Owner {
                    ApiError::WorkspaceTransaction(error)
                } else {
                    ApiError::TaskTransactionRejected
                });
            }
        }
    };
    let recorded = state
        .control_plane
        .finalize_audit_intent(&intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    Ok((committed, recorded))
}

fn task_plan_revision(
    plan: &weftext_core::WorkspaceTransactionPlan,
) -> Result<DocumentRevision, ApiError> {
    plan.document_changes
        .first()
        .filter(|_| plan.document_changes.len() == 1)
        .map(|change| change.next_revision.clone())
        .ok_or(ApiError::TaskTransactionRejected)
}

async fn recover_task_transactions(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_owner(&state, &principal)?;
    let _commit_guard = state.commits.lock().await;
    require_owner(&state, &principal)?;
    let base =
        read_workspace_revision(state.workspace_root()).map_err(|_| ApiError::WorkspaceInvalid)?;
    let audit_time = unix_now()?;
    let intent = state
        .control_plane
        .begin_audit_intent(
            &principal,
            "task_transaction_recovery",
            "recover",
            "workspace",
            "workspace",
            &format!("not:{base}"),
            audit_time,
        )
        .map_err(ApiError::ControlPlane)?;
    let Ok(recovery) = recover_workspace_transactions(state.workspace_root()) else {
        return Err(ApiError::CommitOutcomeIndeterminate);
    };
    if let Ok(revision) = read_workspace_revision(state.workspace_root()) {
        let _ = state.control_plane.update_audit_intent_authority(
            &intent,
            "workspace",
            "workspace",
            &revision.to_string(),
        );
    }
    let audit_recorded = state
        .control_plane
        .finalize_audit_intent(&intent, unix_now().unwrap_or(audit_time), None)
        .is_ok();
    Ok(Json(serde_json::json!({
        "recovery": recovery,
        "auditRecorded": audit_recorded,
    })))
}

fn map_task_transaction_error(
    principal: &SessionPrincipal,
    error: TaskTransactionError,
) -> ApiError {
    if principal.role != SessionRole::Owner
        && matches!(
            &error,
            TaskTransactionError::Workspace(WorkspaceTransactionError::StaleRevision { .. })
        )
    {
        return ApiError::TaskTransactionRejected;
    }
    match error {
        TaskTransactionError::Workspace(WorkspaceTransactionError::Document(error)) => {
            ApiError::Document(error)
        }
        TaskTransactionError::Workspace(error) => ApiError::WorkspaceTransaction(error),
        TaskTransactionError::Authoring(error) => ApiError::TaskAuthoring(error),
        TaskTransactionError::Recurrence(error) => ApiError::TaskRecurrence(error),
        TaskTransactionError::Index(_) => ApiError::TaskUnavailable,
        TaskTransactionError::TargetUnavailable
        | TaskTransactionError::TargetWorkspaceInvalid { .. }
        | TaskTransactionError::ProposedWorkspaceInvalid { .. } => {
            ApiError::TaskTransactionRejected
        }
    }
}

fn citation_index(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<CitationWorkspaceIndex, ApiError> {
    if principal.role == SessionRole::Owner {
        CitationWorkspaceIndex::rebuild(state.workspace_root())
    } else {
        let scope = authorized_read_scope(state, principal)?;
        CitationWorkspaceIndex::rebuild_scoped(state.workspace_root(), &scope)
    }
    .map_err(|_| ApiError::CitationUnavailable)
}

fn server_citation_scope(
    index: &CitationWorkspaceIndex,
    principal: &SessionPrincipal,
) -> CitationAccessScope {
    let ids = index.reference_node_ids().collect::<Vec<_>>();
    if principal.role == SessionRole::Owner {
        CitationAccessScope::complete(ids)
    } else {
        CitationAccessScope::filtered(ids)
    }
}

fn parse_workspace_revision(value: &str) -> Result<WorkspaceRevision, ApiError> {
    WorkspaceRevision::parse(value)
        .map_err(|_| ApiError::InvalidRequest("workspace revision is invalid"))
}

fn require_workspace_revision(
    expected: &WorkspaceRevision,
    actual: &WorkspaceRevision,
) -> Result<(), ApiError> {
    if expected == actual {
        Ok(())
    } else {
        Err(ApiError::StaleWorkspaceRevision {
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn session_is_current_now(state: &ServerState, principal: &SessionPrincipal) -> bool {
    unix_now().is_ok_and(|now| {
        state
            .control_plane
            .session_is_current(principal, now)
            .unwrap_or(false)
    })
}

async fn collaboration_event_stream(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let stream_permit = state
        .change_stream_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::ChangeStreamLimit)?;
    let expires_at = principal.absolute_expires_at.min(principal.idle_expires_at);
    let seconds_until_expiry =
        u64::try_from(expires_at.saturating_sub(unix_now()?).max(0)).unwrap_or(0);
    let filter_state = state.clone();
    let filter_principal = principal.clone();
    let current_state = state.clone();
    let current_principal = principal.clone();
    let authorization_epoch = state
        .control_plane
        .authorization_epoch(&principal.actor_scope)
        .map_err(ApiError::ControlPlane)?;
    let mut shutdown = state.shutdown.subscribe();
    let runtime_shutdown = async move {
        if *shutdown.borrow() {
            return;
        }
        while shutdown.changed().await.is_ok() {
            if *shutdown.borrow() {
                break;
            }
        }
    };
    let mut authorization_changes = state.authorization_changes.subscribe();
    let revocation = async move {
        loop {
            tokio::select! {
                biased;
                change = authorization_changes.recv() => match change {
                    Ok(change) if change.actor_scope == current_principal.actor_scope => break,
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let unchanged = current_state
                            .control_plane
                            .authorization_epoch(&current_principal.actor_scope)
                            .is_ok_and(|epoch| epoch == authorization_epoch);
                        if !unchanged {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                () = tokio::time::sleep(Duration::from_millis(25)) => {
                    let current = unix_now().is_ok_and(|now| {
                        current_state
                            .control_plane
                            .session_is_current(&current_principal, now)
                            .unwrap_or(false)
                    });
                    if !current {
                        break;
                    }
                }
            }
        }
    };
    let stream = BroadcastStream::new(state.collaboration_events.subscribe())
        .filter_map(move |event| {
            let allowed = match &event {
                Ok(event) => unix_now().is_ok_and(|now| {
                    filter_state
                        .control_plane
                        .session_is_current(&filter_principal, now)
                        .unwrap_or(false)
                        && matches!(
                            filter_state.node_access(&filter_principal, event.node_id),
                            Ok(NodeAccess::Read | NodeAccess::Write)
                        )
                }),
                Err(_) => session_is_current_now(&filter_state, &filter_principal),
            };
            std::future::ready(allowed.then_some(event))
        })
        .map(move |event| {
            let _keep_slot_for_stream_lifetime = &stream_permit;
            let (event_name, data) =
                collaboration_stream_payload(event, principal.role == SessionRole::Owner);
            Event::default().event(event_name).data(data)
        })
        .map(Ok::<_, Infallible>)
        .take_until(revocation)
        .take_until(runtime_shutdown)
        .take_until(tokio::time::sleep(Duration::from_secs(
            seconds_until_expiry,
        )));
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

fn collaboration_stream_payload(
    event: Result<CollaborationEvent, BroadcastStreamRecvError>,
    disclose_lag_count: bool,
) -> (&'static str, String) {
    match event {
        Ok(event) => (
            event.event_type,
            serde_json::to_string(&event).expect("CollaborationEvent serialization cannot fail"),
        ),
        Err(BroadcastStreamRecvError::Lagged(missed)) if disclose_lag_count => (
            "resync-required",
            format!(
                r#"{{"wireVersion":"{}","reason":"lagged","missedEvents":{missed}}}"#,
                collaboration::WIRE_VERSION
            ),
        ),
        Err(BroadcastStreamRecvError::Lagged(_)) => (
            "resync-required",
            format!(
                r#"{{"wireVersion":"{}","reason":"lagged"}}"#,
                collaboration::WIRE_VERSION
            ),
        ),
    }
}

async fn changes(
    State(state): State<ServerState>,
    Authenticated(principal): Authenticated,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let stream_permit = state
        .change_stream_slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::ChangeStreamLimit)?;
    let expires_at = principal.absolute_expires_at.min(principal.idle_expires_at);
    let seconds_until_expiry =
        u64::try_from(expires_at.saturating_sub(unix_now()?).max(0)).unwrap_or(0);
    let filter_state = state.clone();
    let filter_principal = principal.clone();
    let current_state = state.clone();
    let current_principal = principal.clone();
    let authorization_epoch = state
        .control_plane
        .authorization_epoch(&principal.actor_scope)
        .map_err(ApiError::ControlPlane)?;
    let mut shutdown = state.shutdown.subscribe();
    let runtime_shutdown = async move {
        if *shutdown.borrow() {
            return;
        }
        while shutdown.changed().await.is_ok() {
            if *shutdown.borrow() {
                break;
            }
        }
    };
    let mut authorization_changes = state.authorization_changes.subscribe();
    let revocation = async move {
        loop {
            tokio::select! {
                biased;
                change = authorization_changes.recv() => match change {
                    Ok(change) if change.actor_scope == current_principal.actor_scope => break,
                    Ok(_) => {}
                    Err(broadcast::error::RecvError::Lagged(_)) => {
                        let unchanged = current_state
                            .control_plane
                            .authorization_epoch(&current_principal.actor_scope)
                            .is_ok_and(|epoch| epoch == authorization_epoch);
                        if !unchanged {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                },
                () = tokio::time::sleep(Duration::from_millis(25)) => {
                    let current = unix_now().is_ok_and(|now| {
                        current_state
                            .control_plane
                            .session_is_current(&current_principal, now)
                            .unwrap_or(false)
                    });
                    if !current {
                        break;
                    }
                }
            }
        }
    };
    let stream = BroadcastStream::new(state.changes.subscribe())
        .filter_map(move |event| {
            let allowed = match &event {
                Ok(change) => unix_now().is_ok_and(|now| {
                    filter_state
                        .control_plane
                        .session_is_current(&filter_principal, now)
                        .unwrap_or(false)
                        && matches!(
                            filter_state.node_access(&filter_principal, change.node_id),
                            Ok(NodeAccess::Read | NodeAccess::Write)
                        )
                }),
                Err(_) => session_is_current_now(&filter_state, &filter_principal),
            };
            std::future::ready(allowed.then_some(event))
        })
        .map(move |event| {
            let _keep_slot_for_stream_lifetime = &stream_permit;
            if principal.role == SessionRole::Owner {
                change_stream_event(event)
            } else {
                change_stream_event_filtered(event)
            }
        })
        .map(Ok::<_, Infallible>)
        .take_until(revocation)
        .take_until(runtime_shutdown)
        .take_until(tokio::time::sleep(Duration::from_secs(
            seconds_until_expiry,
        )));
    Ok(Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

fn change_stream_event_filtered(event: Result<ChangeEvent, BroadcastStreamRecvError>) -> Event {
    match event {
        Ok(change) => Event::default()
            .event("node-committed")
            .data(serde_json::to_string(&change).expect("ChangeEvent serialization cannot fail")),
        Err(BroadcastStreamRecvError::Lagged(_)) => Event::default()
            .event("resync-required")
            .data(r#"{"reason":"lagged"}"#),
    }
}

fn change_stream_event(event: Result<ChangeEvent, BroadcastStreamRecvError>) -> Event {
    let (event_name, data) = change_stream_payload(event);
    Event::default().event(event_name).data(data)
}

fn change_stream_payload(
    event: Result<ChangeEvent, BroadcastStreamRecvError>,
) -> (&'static str, String) {
    match event {
        Ok(change) => (
            "node-committed",
            serde_json::to_string(&change).expect("ChangeEvent serialization cannot fail"),
        ),
        Err(BroadcastStreamRecvError::Lagged(missed_events)) => (
            "resync-required",
            format!(r#"{{"reason":"lagged","missedEvents":{missed_events}}}"#),
        ),
    }
}

fn root_presentation_setting(
    state: &ServerState,
    principal: &SessionPrincipal,
) -> Result<AdjacentHeadingBody, ApiError> {
    let inventory = scan_workspace(state.workspace_root());
    if principal.role == SessionRole::Owner {
        if !inventory.is_valid() {
            return Err(ApiError::WorkspaceInvalid);
        }
    } else {
        authorized_scope_from_inventory(state, principal, &inventory)?;
    }
    let mut roots = inventory
        .nodes
        .iter()
        .filter(|node| node.path == inventory.root);
    let root = roots.next().ok_or(ApiError::WorkspaceInvalid)?;
    if roots.next().is_some() {
        return Err(ApiError::WorkspaceInvalid);
    }
    root.metadata
        .map(|metadata| metadata.presentation.adjacent_heading_body)
        .ok_or(ApiError::WorkspaceInvalid)
}

fn parse_node_id(value: &str) -> Result<NodeId, ApiError> {
    NodeId::from_str(value).map_err(|_| ApiError::InvalidNodeId)
}

#[expect(
    clippy::too_many_lines,
    reason = "the exhaustive middleware keeps proxy, CSRF, shutdown, authentication, quiescence, and degraded-workspace gates in one ordered boundary"
)]
async fn security_gate(
    State(state): State<ServerState>,
    mut request: Request,
    next: Next,
) -> Response {
    let no_store = request.uri().path().starts_with(API_PREFIX);
    let strict_transport = state.http.uses_same_host_reverse_proxy();
    if let Err(error) = enforce_proxy_boundary(&state, &mut request) {
        return secure_response(error.into_response(), no_store, strict_transport);
    }
    if request
        .headers()
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        != Some(state.http.allowed_host.as_str())
    {
        return secure_response(
            ApiError::InvalidHost.into_response(),
            no_store,
            strict_transport,
        );
    }
    if state.is_shutting_down()
        && !matches!(
            request.uri().path(),
            "/api/v1/health/live" | "/api/v1/health/ready"
        )
    {
        return secure_response(
            ApiError::ServerShuttingDown.into_response(),
            no_store,
            strict_transport,
        );
    }
    if is_state_changing(request.method()) {
        let origin_matches = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|value| value.to_str().ok())
            == Some(state.http.allowed_origin.as_str());
        let csrf_matches = request
            .headers()
            .get(CSRF_HEADER)
            .and_then(|value| value.to_str().ok())
            == Some(CSRF_VALUE);
        if !origin_matches || !csrf_matches {
            return secure_response(
                ApiError::CsrfRejected.into_response(),
                no_store,
                strict_transport,
            );
        }
    }
    let backup_administration = is_backup_administration_path(request.uri().path());
    let principal = if is_protected_api_path(request.uri().path()) {
        let Some(token) = session_token(request.headers()) else {
            return secure_response(
                ApiError::AuthenticationRequired.into_response(),
                no_store,
                strict_transport,
            );
        };
        let now = match unix_now() {
            Ok(now) => now,
            Err(error) => {
                return secure_response(error.into_response(), no_store, strict_transport);
            }
        };
        let validation = if backup_administration {
            state
                .control_plane
                .validate_session_without_refresh(&token, now)
        } else {
            state.control_plane.validate_session(&token, now)
        };
        let principal = match validation {
            Ok(principal) => principal,
            Err(AuthError::InvalidSession | AuthError::ExpiredSession) => {
                return secure_response(
                    ApiError::AuthenticationRequired.into_response(),
                    no_store,
                    strict_transport,
                );
            }
            Err(error) => {
                return secure_response(
                    ApiError::ControlPlane(error).into_response(),
                    no_store,
                    strict_transport,
                );
            }
        };
        if backup_administration && !principal.role.can_manage_workspace() {
            return secure_response(
                ApiError::AuthorizationDenied.into_response(),
                no_store,
                strict_transport,
            );
        }
        Some(principal)
    } else {
        None
    };
    let (_exclusive_quiescence, _shared_quiescence) = if backup_administration {
        (Some(state.api_quiescence.write().await), None)
    } else {
        (None, Some(state.api_quiescence.read().await))
    };
    if let Some(principal) = principal {
        if backup_administration {
            if let Err(error) = state.require_current_principal(&principal) {
                return secure_response(error.into_response(), no_store, strict_transport);
            }
            if !principal.role.can_manage_workspace() {
                return secure_response(
                    ApiError::AuthorizationDenied.into_response(),
                    no_store,
                    strict_transport,
                );
            }
        }
        if is_workspace_authority_mutation(request.method(), request.uri().path())
            && let Err(error) = require_trash_workspace_writable(&state, false)
        {
            return secure_response(error.into_response(), no_store, strict_transport);
        }
        request.extensions_mut().insert(principal);
    }
    let response = next.run(request).await;
    secure_response(response, no_store, strict_transport)
}

fn enforce_proxy_boundary(state: &ServerState, request: &mut Request) -> Result<(), ApiError> {
    if has_untrusted_forwarding_headers(request.headers()) {
        return Err(ApiError::ForwardedHeaderRejected);
    }
    if state.http.uses_same_host_reverse_proxy() && request.uri().path() != "/api/v1/health/live" {
        let mut supplied = request.headers().get_all(PROXY_TOKEN_HEADER).iter();
        let candidate = supplied
            .next()
            .and_then(|value| value.to_str().ok())
            .and_then(reverse_proxy_token_digest);
        let valid = supplied.next().is_none()
            && candidate
                .zip(state.proxy_token_digest.as_deref())
                .is_some_and(|(candidate, expected)| candidate.ct_eq(expected).into());
        if !valid {
            return Err(ApiError::ProxyBoundaryRejected);
        }
        request.headers_mut().remove(PROXY_TOKEN_HEADER);
    } else if !state.http.uses_same_host_reverse_proxy()
        && request.headers().contains_key(PROXY_TOKEN_HEADER)
    {
        return Err(ApiError::ProxyBoundaryRejected);
    }
    Ok(())
}

fn has_untrusted_forwarding_headers(headers: &HeaderMap) -> bool {
    headers.keys().any(|name| {
        matches!(name.as_str(), "forwarded" | "x-real-ip")
            || name.as_str().starts_with("x-forwarded-")
    })
}

fn secure_response(mut response: Response, no_store: bool, strict_transport: bool) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'",
        ),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("cross-origin-resource-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    if strict_transport {
        headers.insert(
            header::STRICT_TRANSPORT_SECURITY,
            HeaderValue::from_static("max-age=31536000"),
        );
    }
    if no_store {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    }
    response
}

fn is_state_changing(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

fn is_backup_administration_path(path: &str) -> bool {
    path.starts_with("/api/v1/admin/backup/") || path.starts_with("/api/v1/admin/restore/")
}

fn is_workspace_authority_mutation(method: &Method, path: &str) -> bool {
    (*method == Method::PUT && path.starts_with("/api/v1/documents/"))
        || (*method == Method::POST
            && ((path.starts_with("/api/v1/annotations/"))
                || (path.starts_with("/api/v1/collaboration/documents/")
                    && (path.ends_with("/operations") || path.ends_with("/drafts")))
                || (path.starts_with("/api/v1/citations/") && path.ends_with("/macros/commit"))
                || (path.starts_with("/api/v1/tasks/transactions/") && path.ends_with("/commit"))
                || path == "/api/v1/tasks/recover"))
}

fn is_protected_api_path(path: &str) -> bool {
    path.starts_with(API_PREFIX)
        && !matches!(
            path,
            "/api/v1/health"
                | "/api/v1/health/live"
                | "/api/v1/health/ready"
                | "/api/v1/capabilities"
                | "/api/v1/auth/bootstrap"
                | "/api/v1/auth/login"
        )
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .map(str::trim)
        .find_map(|cookie| {
            cookie
                .strip_prefix(SESSION_COOKIE)
                .and_then(|value| value.strip_prefix('='))
                .map(str::to_owned)
        })
}

fn session_cookie(token: &str, secure: bool, max_age: i64) -> HeaderValue {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}={token}; Path=/api/v1; HttpOnly; SameSite=Strict; Max-Age={max_age}{secure_attribute}"
    ))
    .expect("hex token and fixed attributes form a valid cookie")
}

fn clear_session_cookie(secure: bool) -> HeaderValue {
    let secure_attribute = if secure { "; Secure" } else { "" };
    HeaderValue::from_str(&format!(
        "{SESSION_COOKIE}=; Path=/api/v1; HttpOnly; SameSite=Strict; Max-Age=0{secure_attribute}"
    ))
    .expect("fixed attributes form a valid cookie")
}

fn unix_now() -> Result<i64, ApiError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::ControlPlaneUnavailable)?
        .as_secs();
    i64::try_from(seconds).map_err(|_| ApiError::ControlPlaneUnavailable)
}

struct Authenticated(SessionPrincipal);

impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    fn from_request_parts(
        parts: &mut Parts,
        _: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        std::future::ready(
            parts
                .extensions
                .get::<SessionPrincipal>()
                .cloned()
                .map(Self)
                .ok_or(ApiError::AuthenticationRequired),
        )
    }
}

struct ApiPath<T>(T);

impl<S, T> FromRequestParts<S> for ApiPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        RoutePath::<T>::from_request_parts(parts, state)
            .await
            .map(|RoutePath(value)| Self(value))
            .map_err(ApiError::from_path_rejection)
    }
}

struct ApiJson<T>(T);

impl<S, T> FromRequest<S> for ApiJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        Json::<T>::from_request(request, state)
            .await
            .map(|Json(value)| Self(value))
            .map_err(|rejection| ApiError::from_json_rejection(&rejection))
    }
}

struct ApiQuery<T>(T);

impl<S, T> FromRequestParts<S> for ApiQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Query::<T>::from_request_parts(parts, state)
            .await
            .map(|Query(value)| Self(value))
            .map_err(ApiError::from_query_rejection)
    }
}

#[derive(Debug)]
enum QueryClientRejection {
    Generic,
    DomainUnavailable,
    MissingContext,
    MissingHeadingContext,
    NullComparison,
    ResourceLimit,
}

impl QueryClientRejection {
    fn response(
        self,
    ) -> (
        StatusCode,
        &'static str,
        String,
        Option<ConflictBody>,
        Option<serde_json::Value>,
    ) {
        let (code, message) = match self {
            Self::Generic => (
                "query_rejected",
                "query execution was rejected without disclosing unavailable content",
            ),
            Self::DomainUnavailable => (
                "domain_unavailable",
                "the requested query domain is not available in this runtime",
            ),
            Self::MissingContext => (
                "missing_context",
                "the query requires an explicit owning-node context binding",
            ),
            Self::MissingHeadingContext => (
                "missing_heading_context",
                "the query requires an explicitly bound heading context",
            ),
            Self::NullComparison => (
                "null_comparison",
                "ordinary comparison encountered null; use an explicit null guard",
            ),
            Self::ResourceLimit => (
                "resource_limit",
                "query execution exceeded a bounded resource limit",
            ),
        };
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            code,
            message.to_owned(),
            None,
            None,
        )
    }
}

fn query_api_error(error: &QueryExecutionError) -> ApiError {
    let rejection = match error {
        QueryExecutionError::DomainUnavailable(_) => QueryClientRejection::DomainUnavailable,
        QueryExecutionError::MissingContext(_) => QueryClientRejection::MissingContext,
        QueryExecutionError::MissingHeadingContext => QueryClientRejection::MissingHeadingContext,
        QueryExecutionError::NullComparison => QueryClientRejection::NullComparison,
        QueryExecutionError::ResourceLimit => QueryClientRejection::ResourceLimit,
        QueryExecutionError::InvalidPlan
        | QueryExecutionError::InvalidContext
        | QueryExecutionError::MissingScopeNode(_)
        | QueryExecutionError::UnavailableScope => QueryClientRejection::Generic,
    };
    ApiError::QueryRejected(rejection)
}

#[derive(Debug)]
enum ApiError {
    Extraction {
        status: StatusCode,
        code: &'static str,
        message: &'static str,
    },
    InvalidNodeId,
    InvalidHost,
    ForwardedHeaderRejected,
    ProxyBoundaryRejected,
    CsrfRejected,
    ServerShuttingDown,
    AuthenticationRequired,
    AuthorizationDenied,
    MemberExists,
    MemberUnavailable,
    LastOwner,
    AuthenticationFailed,
    BootstrapFailed,
    RateLimited,
    ChangeStreamLimit,
    ControlPlane(AuthError),
    ControlPlaneUnavailable,
    Backup(ServerControlPlaneBackupError),
    BackupPlanLimit,
    BackupPlanUnavailable,
    CommitOutcomeIndeterminate,
    MethodNotAllowed,
    InvalidRequest(&'static str),
    NodeNotFound,
    AnnotationUnavailable,
    WorkspaceInvalid,
    Document(DocumentError),
    CitationAuthoring(CitationAuthoringFailure),
    CitationRequest(String),
    CitationUnavailable,
    TaskAuthoring(TaskAuthoringFailure),
    TaskRecurrence(TaskRecurrenceCompletionFailure),
    TaskUnavailable,
    TaskTransactionRejected,
    TaskPlanLimit,
    TaskPlanUnavailable,
    TrashItemUnavailable,
    TrashPlanLimit,
    TrashPlanUnavailable,
    TrashReadOnly,
    TrashMigrationBackupUnavailable,
    QueryUnavailable,
    QueryRejected(QueryClientRejection),
    StaleWorkspaceRevision {
        expected: String,
        actual: String,
    },
    WorkspaceTransaction(WorkspaceTransactionError),
    WorkspaceWriteRejected,
    Search,
}

impl ApiError {
    fn from_bootstrap(error: AuthError) -> Self {
        match error {
            AuthError::BootstrapUnavailable | AuthError::BootstrapFailed => Self::BootstrapFailed,
            AuthError::Password => {
                Self::InvalidRequest("Owner password must contain between 12 and 1024 UTF-8 bytes")
            }
            other => Self::ControlPlane(other),
        }
    }

    fn from_login(error: AuthError) -> Self {
        match error {
            AuthError::InvalidCredentials => Self::AuthenticationFailed,
            other => Self::ControlPlane(other),
        }
    }

    fn from_json_rejection(rejection: &JsonRejection) -> Self {
        let status = rejection.status();
        let (code, message) = match status {
            StatusCode::UNSUPPORTED_MEDIA_TYPE => (
                "unsupported_media_type",
                "request body must use application/json",
            ),
            StatusCode::PAYLOAD_TOO_LARGE => {
                ("payload_too_large", "request body exceeds the Server limit")
            }
            _ => (
                "invalid_json",
                "request body is malformed or does not match the API schema",
            ),
        };
        Self::Extraction {
            status,
            code,
            message,
        }
    }

    fn from_query_rejection(_: QueryRejection) -> Self {
        Self::Extraction {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_query",
            message: "query parameters are missing or do not match the API schema",
        }
    }

    fn from_path_rejection(_: PathRejection) -> Self {
        Self::Extraction {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_path",
            message: "path parameters are malformed or do not match the API schema",
        }
    }
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: &'static str,
    message: String,
    conflict: Option<ConflictBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConflictBody {
    expected_revision: String,
    actual_revision: String,
}

impl IntoResponse for ApiError {
    #[expect(
        clippy::too_many_lines,
        reason = "one exhaustive mapping keeps the public JSON error contract auditable"
    )]
    fn into_response(self) -> Response {
        let (status, code, message, conflict, details) = match self {
            Self::Extraction {
                status,
                code,
                message,
            } => (status, code, message.to_owned(), None, None),
            Self::InvalidNodeId => (
                StatusCode::BAD_REQUEST,
                "invalid_node_id",
                "node ID must be a canonical lowercase UUIDv4".to_owned(),
                None,
                None,
            ),
            Self::InvalidHost => (
                StatusCode::BAD_REQUEST,
                "invalid_host",
                "request Host does not match this Server listener".to_owned(),
                None,
                None,
            ),
            Self::ForwardedHeaderRejected => (
                StatusCode::BAD_REQUEST,
                "forwarded_header_rejected",
                "forwarding metadata is not accepted by this Server trust boundary".to_owned(),
                None,
                None,
            ),
            Self::ProxyBoundaryRejected => (
                StatusCode::FORBIDDEN,
                "proxy_boundary_rejected",
                "request did not arrive through the configured same-host proxy boundary".to_owned(),
                None,
                None,
            ),
            Self::CsrfRejected => (
                StatusCode::FORBIDDEN,
                "csrf_rejected",
                "state-changing requests require the configured same origin and CSRF header"
                    .to_owned(),
                None,
                None,
            ),
            Self::ServerShuttingDown => (
                StatusCode::SERVICE_UNAVAILABLE,
                "server_shutting_down",
                "Server is draining active requests and is not accepting new work".to_owned(),
                None,
                None,
            ),
            Self::AuthenticationRequired => (
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "a current local-account session is required".to_owned(),
                None,
                None,
            ),
            Self::AuthorizationDenied => (
                StatusCode::FORBIDDEN,
                "authorization_denied",
                "the current workspace role cannot mutate annotations".to_owned(),
                None,
                None,
            ),
            Self::MemberExists => (
                StatusCode::CONFLICT,
                "member_exists",
                "member login already exists".to_owned(),
                None,
                None,
            ),
            Self::MemberUnavailable => (
                StatusCode::NOT_FOUND,
                "member_unavailable",
                "member is not present".to_owned(),
                None,
                None,
            ),
            Self::LastOwner => (
                StatusCode::CONFLICT,
                "last_owner_required",
                "the last enabled Owner cannot be changed".to_owned(),
                None,
                None,
            ),
            Self::AuthenticationFailed => (
                StatusCode::UNAUTHORIZED,
                "authentication_failed",
                "login name or password is invalid".to_owned(),
                None,
                None,
            ),
            Self::BootstrapFailed => (
                StatusCode::FORBIDDEN,
                "bootstrap_failed",
                "Owner bootstrap could not be completed".to_owned(),
                None,
                None,
            ),
            Self::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited",
                "too many authentication attempts; retry later".to_owned(),
                None,
                None,
            ),
            Self::ChangeStreamLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                "change_stream_limit",
                "too many change subscriptions are active".to_owned(),
                None,
                None,
            ),
            Self::ControlPlane(error) => {
                let _ = error;
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "control_plane_unavailable",
                    "Server identity control plane is unavailable".to_owned(),
                    None,
                    None,
                )
            }
            Self::ControlPlaneUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "control_plane_unavailable",
                "Server identity control plane is unavailable".to_owned(),
                None,
                None,
            ),
            Self::Backup(ServerControlPlaneBackupError::RestoreTargetExists(_)) => (
                StatusCode::CONFLICT,
                "restore_target_not_clean",
                "restore targets must be new, disjoint, clean locations".to_owned(),
                None,
                None,
            ),
            Self::Backup(ServerControlPlaneBackupError::StalePreview) => (
                StatusCode::CONFLICT,
                "backup_preview_stale",
                "backup or restore authority changed after preview; preview again".to_owned(),
                None,
                None,
            ),
            Self::Backup(ServerControlPlaneBackupError::ControlPlaneInUse(_)) => (
                StatusCode::CONFLICT,
                "backup_quiescence_unavailable",
                "the exclusive Server backup lease is unavailable".to_owned(),
                None,
                None,
            ),
            Self::Backup(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "backup_rejected",
                "backup, restore, or drill safety verification rejected the operation".to_owned(),
                None,
                None,
            ),
            Self::BackupPlanLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                "backup_plan_limit",
                "too many backup or restore previews are pending".to_owned(),
                None,
                None,
            ),
            Self::BackupPlanUnavailable => (
                StatusCode::CONFLICT,
                "backup_plan_unavailable",
                "the exact backup or restore preview is unavailable; preview again".to_owned(),
                None,
                None,
            ),
            Self::CommitOutcomeIndeterminate => (
                StatusCode::SERVICE_UNAVAILABLE,
                "commit_outcome_indeterminate",
                "the durable commit outcome requires recovery; refresh authority before retrying"
                    .to_owned(),
                None,
                None,
            ),
            Self::MethodNotAllowed => (
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "HTTP method is not supported for this route".to_owned(),
                None,
                None,
            ),
            Self::InvalidRequest(message) => (
                StatusCode::BAD_REQUEST,
                "invalid_request",
                message.to_owned(),
                None,
                None,
            ),
            Self::NodeNotFound => (
                StatusCode::NOT_FOUND,
                "node_not_found",
                "node is not present in the hosted workspace".to_owned(),
                None,
                None,
            ),
            Self::AnnotationUnavailable => (
                StatusCode::NOT_FOUND,
                "annotation_unavailable",
                "annotation or message is not present in the authorized node".to_owned(),
                None,
                None,
            ),
            Self::WorkspaceInvalid => (
                StatusCode::SERVICE_UNAVAILABLE,
                "workspace_invalid",
                "hosted workspace is incomplete or requires recovery".to_owned(),
                None,
                None,
            ),
            Self::Document(DocumentError::StaleRevision { expected, actual }) => (
                StatusCode::CONFLICT,
                "stale_revision",
                "document changed after the supplied base revision".to_owned(),
                Some(ConflictBody {
                    expected_revision: expected.to_string(),
                    actual_revision: actual.to_string(),
                }),
                None,
            ),
            Self::Document(error) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "document_rejected",
                safe_document_error(&error),
                None,
                None,
            ),
            Self::CitationAuthoring(failure) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "citation_authoring_rejected",
                "citation authoring request was rejected without changing hosted content"
                    .to_owned(),
                None,
                serde_json::to_value(failure).ok(),
            ),
            Self::CitationRequest(message) => (
                StatusCode::BAD_REQUEST,
                "invalid_citation_request",
                message,
                None,
                None,
            ),
            Self::CitationUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "citation_unavailable",
                "citation data is unavailable for this authorized workspace scope".to_owned(),
                None,
                None,
            ),
            Self::TaskAuthoring(failure) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "task_authoring_rejected",
                "task authoring request was rejected without changing hosted content".to_owned(),
                None,
                serde_json::to_value(failure).ok(),
            ),
            Self::TaskRecurrence(failure) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "task_recurrence_rejected",
                "recurring completion was rejected without changing hosted content".to_owned(),
                None,
                serde_json::to_value(failure).ok(),
            ),
            Self::TaskUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "task_unavailable",
                "task data is unavailable for this authorized workspace scope".to_owned(),
                None,
                None,
            ),
            Self::TaskTransactionRejected => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "task_transaction_rejected",
                "task transaction was rejected without changing or disclosing unavailable content"
                    .to_owned(),
                None,
                None,
            ),
            Self::TaskPlanLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                "task_plan_limit",
                "too many task transaction previews are pending".to_owned(),
                None,
                None,
            ),
            Self::TaskPlanUnavailable => (
                StatusCode::NOT_FOUND,
                "task_plan_unavailable",
                "task transaction preview is missing, expired, or belongs to another session"
                    .to_owned(),
                None,
                None,
            ),
            Self::TrashItemUnavailable => (
                StatusCode::NOT_FOUND,
                "trash_item_unavailable",
                "Trash item is missing or unavailable to the current session".to_owned(),
                None,
                None,
            ),
            Self::TrashPlanLimit => (
                StatusCode::TOO_MANY_REQUESTS,
                "trash_plan_limit",
                "too many Trash transaction previews are pending".to_owned(),
                None,
                None,
            ),
            Self::TrashPlanUnavailable => (
                StatusCode::NOT_FOUND,
                "trash_plan_unavailable",
                "Trash transaction preview is missing, expired, or belongs to another session"
                    .to_owned(),
                None,
                None,
            ),
            Self::TrashReadOnly => (
                StatusCode::CONFLICT,
                "trash_reconciliation_required",
                "Workspace Trash requires migration or reconciliation; workspace writes are paused"
                    .to_owned(),
                None,
                None,
            ),
            Self::TrashMigrationBackupUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "trash_migration_backup_unavailable",
                "legacy Trash migration requires a configured external snapshot parent".to_owned(),
                None,
                None,
            ),
            Self::QueryUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "query_unavailable",
                "query data is unavailable for this authorized workspace scope".to_owned(),
                None,
                None,
            ),
            Self::QueryRejected(rejection) => rejection.response(),
            Self::StaleWorkspaceRevision { expected, actual } => (
                StatusCode::CONFLICT,
                "stale_workspace_revision",
                "workspace changed after the supplied transaction preview".to_owned(),
                Some(ConflictBody {
                    expected_revision: expected,
                    actual_revision: actual,
                }),
                None,
            ),
            Self::WorkspaceTransaction(WorkspaceTransactionError::StaleRevision {
                expected,
                actual,
            }) => (
                StatusCode::CONFLICT,
                "stale_workspace_revision",
                "workspace changed after the supplied transaction preview".to_owned(),
                Some(ConflictBody {
                    expected_revision: expected.to_string(),
                    actual_revision: actual.to_string(),
                }),
                None,
            ),
            Self::WorkspaceTransaction(_) | Self::WorkspaceWriteRejected => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "workspace_transaction_rejected",
                "workspace transaction failed without exposing or changing unauthorized content"
                    .to_owned(),
                None,
                None,
            ),
            Self::Search => (
                StatusCode::SERVICE_UNAVAILABLE,
                "search_unavailable",
                "workspace search is temporarily unavailable".to_owned(),
                None,
                None,
            ),
        };
        (
            status,
            Json(ErrorEnvelope {
                error: ErrorBody {
                    code,
                    message,
                    conflict,
                    details,
                },
            }),
        )
            .into_response()
    }
}

fn safe_document_error(error: &DocumentError) -> String {
    match error {
        DocumentError::InvalidRevision(_) => {
            "document revision is not canonical lowercase SHA-256".to_owned()
        }
        DocumentError::IdentityChanged { .. } => {
            "document edit would change node identity".to_owned()
        }
        DocumentError::MissingIdentity => "document edit would remove node identity".to_owned(),
        DocumentError::InvalidMetadata(_) => "document metadata is invalid or ambiguous".to_owned(),
        DocumentError::InvalidEditRange { .. }
        | DocumentError::NonCharacterBoundary { .. }
        | DocumentError::OverlappingEdits => "document edit range is invalid".to_owned(),
        _ => "document operation failed without changing hosted content".to_owned(),
    }
}

async fn webui_index() -> Html<&'static str> {
    Html(include_str!("../webui/index.html"))
}

async fn webui_app() -> impl IntoResponse {
    javascript(include_str!("../webui/app.js"))
}

async fn webui_api() -> impl IntoResponse {
    javascript(include_str!("../webui/api.js"))
}

async fn webui_navigation() -> impl IntoResponse {
    javascript(include_str!("../webui/navigation.js"))
}

fn javascript(source: &'static str) -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/javascript; charset=utf-8"),
        )],
        source,
    )
}

async fn webui_style() -> impl IntoResponse {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/css; charset=utf-8"),
        )],
        include_str!("../webui/style.css"),
    )
}

async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope {
            error: ErrorBody {
                code: "route_not_found",
                message: "route is not part of the Weftext Server API".to_owned(),
                conflict: None,
                details: None,
            },
        }),
    )
}

async fn method_not_allowed() -> ApiError {
    ApiError::MethodNotAllowed
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_core::{DocumentEdit, create_workspace, plan_document_edit, read_node_document};

    #[test]
    fn rejects_non_loopback_bind_addresses() {
        let address = "0.0.0.0:8080".parse().expect("address");
        assert!(matches!(
            validate_bind_address(address),
            Err(StartupError::AuthenticationRequiredForNonLoopback(_))
        ));
    }

    #[test]
    fn accepts_ipv4_and_ipv6_loopback() {
        for value in ["127.0.0.1:8080", "[::1]:8080"] {
            let address = value.parse().expect("address");
            assert_eq!(validate_bind_address(address).expect("loopback"), address);
        }
    }

    #[test]
    fn committed_response_and_event_need_no_post_commit_document_read() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("create workspace");
        let snapshot = read_node_document(&workspace).expect("read document");
        let source = format!("{}\ncommitted", snapshot.source);
        let plan = plan_document_edit(
            &workspace,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("source length"),
                replacement: source,
            }],
        )
        .expect("plan document");
        let expected_revision = plan.next_revision.clone();
        let icon = resolve_node_icon_from_source(plan.next_source());
        let committed = commit_document_edit(&plan).expect("commit document");
        std::fs::remove_file(&committed.document_path).expect("simulate post-commit read failure");
        let (sender, _) = broadcast::channel(2);
        let mut receiver = sender.subscribe();

        let response = finish_committed_document(&plan, committed, icon, &sender);

        assert_eq!(response.revision, expected_revision);
        assert!(response.changed);
        let event = receiver.try_recv().expect("commit event");
        assert_eq!(event.revision, expected_revision);
    }

    #[test]
    fn lagged_change_stream_requires_explicit_resynchronization() {
        let (event, data) = change_stream_payload(Err(BroadcastStreamRecvError::Lagged(9)));
        assert_eq!(event, "resync-required");
        assert_eq!(data, r#"{"reason":"lagged","missedEvents":9}"#);
    }

    #[test]
    fn collaboration_lag_requires_resync_without_disclosing_counts_to_non_owner_roles() {
        let (owner_event, owner_data) =
            collaboration_stream_payload(Err(BroadcastStreamRecvError::Lagged(7)), true);
        assert_eq!(owner_event, "resync-required");
        assert_eq!(
            owner_data,
            r#"{"wireVersion":"weftext.collaboration.v1","reason":"lagged","missedEvents":7}"#
        );
        let (filtered_event, filtered_data) =
            collaboration_stream_payload(Err(BroadcastStreamRecvError::Lagged(7)), false);
        assert_eq!(filtered_event, "resync-required");
        assert_eq!(
            filtered_data,
            r#"{"wireVersion":"weftext.collaboration.v1","reason":"lagged"}"#
        );
    }

    #[test]
    fn session_cookie_has_host_only_http_only_same_site_and_secure_modes() {
        let token = "a".repeat(64);
        let development = session_cookie(&token, false, 3_600)
            .to_str()
            .expect("development cookie")
            .to_owned();
        assert!(development.contains("Path=/api/v1"));
        assert!(development.contains("HttpOnly"));
        assert!(development.contains("SameSite=Strict"));
        assert!(!development.contains("Domain="));
        assert!(!development.contains("Secure"));
        let production = session_cookie(&token, true, 3_600)
            .to_str()
            .expect("production cookie")
            .to_owned();
        assert!(production.ends_with("; Secure"));
    }

    #[test]
    fn annotation_role_matrix_allows_commenters_but_keeps_viewers_read_only() {
        let principal = |role| SessionPrincipal {
            actor_scope: "actor".to_owned(),
            role,
            session_id: "session".to_owned(),
            absolute_expires_at: i64::MAX,
            idle_expires_at: i64::MAX,
        };
        assert!(require_annotation_write(&principal(SessionRole::Owner)).is_ok());
        assert!(require_annotation_write(&principal(SessionRole::Admin)).is_ok());
        assert!(require_annotation_write(&principal(SessionRole::Editor)).is_ok());
        assert!(require_annotation_write(&principal(SessionRole::Commenter)).is_ok());
        assert!(matches!(
            require_annotation_write(&principal(SessionRole::Viewer)),
            Err(ApiError::AuthorizationDenied)
        ));
    }

    #[test]
    fn annotation_actor_ids_are_session_bound_stable_v4_values() {
        let principal = SessionPrincipal {
            actor_scope: "stable actor scope".to_owned(),
            role: SessionRole::Editor,
            session_id: "session".to_owned(),
            absolute_expires_at: i64::MAX,
            idle_expires_at: i64::MAX,
        };
        let id = session_actor_uuid(&principal);
        assert_eq!(id, session_actor_uuid(&principal));
        assert_eq!(id.as_bytes()[14], b'4');
        assert!(matches!(id.as_bytes()[19], b'8' | b'9' | b'a' | b'b'));
    }
}
