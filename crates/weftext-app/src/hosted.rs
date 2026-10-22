//! A separate hosted boundary. Local path, library, recovery and feature APIs
//! are deliberately not mounted here. Every resource comes from operator config.
use super::*;
mod builtin_module;
mod citations;
mod content_import;
mod definition;
mod editor;
mod embedded_query;
mod fields;
mod history_api;
mod inputs;
mod knowledge_links;
mod library;
mod media;
mod navigation;
mod navigation_registry;
mod office;
mod office_import;
mod owner_backup;
mod query;
mod query_providers;
mod query_scopes;
mod recognition;
mod resource_collections;
mod review;
#[cfg(all(test, windows))]
mod session_tests;
mod structure;
mod sync_transport;
use axum::Extension;
use base64::Engine as _;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    time::{Duration, Instant},
};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileEntry {
    path: String,
    title: String,
    #[serde(default = "query_scopes::live")]
    lifecycle: String,
    #[serde(default)]
    expected_node: Option<weftext_knowledge::NodeRef>,
    #[serde(default)]
    editable: bool,
    #[serde(default)]
    importable: bool,
    #[serde(default)]
    inputs: Vec<String>,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Rights {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
    #[serde(default)]
    history: bool,
    #[serde(default)]
    download: bool,
    #[serde(default)]
    fields: BTreeMap<String, FieldRights>,
    #[serde(default)]
    annotations: review::ReviewRights,
    #[serde(default)]
    office: office::OfficeRights,
    #[serde(default)]
    citations: citations::Rights,
    #[serde(default)]
    outputs: office::OutputRights,
    #[serde(default)]
    query: query::QueryRights,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FieldRights {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct User {
    #[serde(default)]
    epoch: u64,
    token_sha256: String,
    enabled: bool,
    grants: BTreeMap<String, Rights>,
    #[serde(default)]
    query_providers: query_providers::Rights,
    #[serde(default)]
    query_scopes: query_scopes::Rights,
    #[serde(default)]
    library: library::Rights,
    #[serde(default)]
    resources: BTreeMap<String, media::Rights>,
    #[serde(default)]
    resource_collections: BTreeMap<String, resource_collections::Rights>,
    #[serde(default)]
    structure: structure::Rights,
    #[serde(default)]
    sync_owner: bool,
    #[serde(default)]
    native_whole_library: office::NativeRights,
    #[serde(default)]
    navigation: navigation_registry::Rights,
    #[serde(default)]
    content_imports: BTreeMap<String, content_import::Rights>,
    #[serde(default)]
    owner_backup: owner_backup::Rights,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Policy {
    deployment: String,
    generation: u64,
    users: BTreeMap<String, User>,
    #[serde(default)]
    sync_identity: Option<weftext_core::sync::SyncIdentity>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    deployment: String,
    bind: SocketAddr,
    origin: Option<String>,
    #[serde(default)]
    allow_insecure_http: bool,
    root: PathBuf,
    admin_token_sha256: String,
    files: BTreeMap<String, FileEntry>,
    #[serde(default)]
    directories: BTreeMap<String, query_scopes::Directory>,
    #[serde(default)]
    style_inputs: Vec<String>,
    #[serde(default)]
    resources: BTreeMap<String, media::Resource>,
    #[serde(default)]
    resource_collections: BTreeMap<String, resource_collections::Collection>,
    #[serde(default)]
    structure_slots: BTreeMap<String, structure::Slot>,
    #[serde(default)]
    structure_templates: BTreeMap<String, structure::Template>,
    #[serde(default)]
    native_whole_library_export: bool,
    #[serde(default)]
    navigation: navigation_registry::Config,
    #[serde(default)]
    content_import_slots: BTreeMap<String, content_import::Slot>,
    #[serde(default)]
    owner_backup_directory: Option<PathBuf>,
    users: BTreeMap<String, User>,
    #[serde(default)]
    sync_identity: Option<weftext_core::sync::SyncIdentity>,
}
#[derive(Clone)]
struct HostedState {
    inner: Arc<Mutex<HostedInner>>,
    origin: Arc<str>,
    host: Arc<str>,
    render_gate: Arc<tokio::sync::Semaphore>,
}
struct HostedInner {
    core: Core,
    config: Config,
    policy: Policy,
    sessions: HashMap<String, HostedSession>,
    downloads: HashMap<String, Download>,
    sync: sync_transport::Store,
    editor_versions: HashMap<String, writing_api::EditorVersion>,
    fields: fields::Store,
    review: review::Store,
    office: office::Store,
    library: library::Store,
    media: media::Store,
    navigation_registry: navigation_registry::Store,
    owner_backup: owner_backup::Store,
}
#[derive(Clone)]
struct HostedSession {
    actor: String,
    generation: u64,
    resource: String,
    expires: Arc<Mutex<Instant>>,
}
const SESSION_IDLE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
struct Download {
    actor: String,
    generation: u64,
    resource: String,
    bytes: Vec<u8>,
    expires: Instant,
}
#[derive(Clone)]
struct Auth {
    digest: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Open {
    resource: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceRequest {
    session_id: String,
    source: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceSaveRequest {
    session_id: String,
    base_id: String,
    source: String,
    operation_id: String,
    #[serde(default)]
    definition_reservations: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OperationStatus {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settle {
    resource: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Search {
    text: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Chunk {
    ticket: String,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PolicyUpdate {
    expected_generation: u64,
    users: BTreeMap<String, User>,
}

fn unavailable() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "unavailable",
        UiMessage::error_api_hosted_content_unavailable(),
    )
}
fn expired() -> ApiError {
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "authentication",
        UiMessage::error_api_hosted_authentication_expired(),
    )
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}
fn validate_users(config: &Config, users: &BTreeMap<String, User>) -> Result<(), ApiError> {
    let mut tokens = BTreeSet::from([config.admin_token_sha256.clone()]);
    if users.len() > 128 {
        return Err(ApiError::bad(UiMessage::error_api_hosted_user_budget()));
    }
    for (id, user) in users {
        if !content_import::valid_rights(config, &user.content_imports)
            || !owner_backup::valid_rights(&user.owner_backup)
        {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_import_backup_needs_separate_permission(),
            ));
        }
        if !navigation_registry::valid_rights(config, &user.navigation) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_navigation_target_permission(),
            ));
        }
        if !office::valid_native_rights(&user.native_whole_library) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_whole_library_output_permission(),
            ));
        }
        if !structure::valid_rights(config, &user.structure) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_structure_target_permission(),
            ));
        }
        if !resource_collections::valid_rights(config, &user.resource_collections) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_resource_collection_permission(),
            ));
        }
        if !media::valid_rights(config, &user.resources) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_resource_permission(),
            ));
        }
        if !library::valid_rights(&user.library) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_configuration_write_needs_read(),
            ));
        }
        if !query_scopes::validate_rights(config, &user.query_scopes) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_directory_scope_permission(),
            ));
        }
        if !query_providers::valid_rights(&user.query_providers) {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_provider_permission_ids(),
            ));
        }
        if user.sync_owner && config.sync_identity.is_none() {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_sync_pairing_required(),
            ));
        }
        if !valid_id(id)
            || !valid_digest(&user.token_sha256)
            || !tokens.insert(user.token_sha256.clone())
        {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_invalid_user_or_token_digest(),
            ));
        }
        for (resource, rights) in &user.grants {
            if !office::valid_legacy_rights(&rights.office) {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_legacy_download_needs_export(),
                ));
            }
            if config
                .files
                .get(resource)
                .is_some_and(|f| f.lifecycle == "trash")
                && (rights.write
                    || rights.fields.values().any(|r| r.write)
                    || rights.annotations.write
                    || rights.annotations.accept
                    || rights.annotations.proofread
                    || rights.office.r#import
                    || rights.office.doc.r#import
                    || rights.office.wps.r#import)
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_trash_read_only_permissions(),
                ));
            }
            if !query::valid_rights(&rights.query) {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_definition_history_needs_read(),
                ));
            }
            if (rights.office.r#import || rights.office.doc.r#import || rights.office.wps.r#import)
                && (!rights.read
                    || !rights.write
                    || !config
                        .files
                        .get(resource)
                        .is_some_and(|f| f.importable && f.editable))
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_office_import_permission(),
                ));
            }
            if (rights.annotations.write && !rights.annotations.read)
                || (rights.annotations.accept
                    && (!rights.annotations.write || !rights.annotations.target || !rights.write))
                || (rights.annotations.proofread && (!rights.read || !rights.annotations.target))
                || (rights.annotations.write
                    && !config.files.get(resource).is_some_and(|f| f.editable))
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_review_write_permission(),
                ));
            }
            if rights.fields.len() > 64
                || rights.fields.iter().any(|(id, field)| {
                    id.is_empty()
                        || id.len() > 256
                        || (field.write && !field.read)
                        || (field.write && !config.files.get(resource).is_some_and(|f| f.editable))
                })
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_field_permission_ids(),
                ));
            }
            if !config.files.contains_key(resource)
                || (!rights.read && (rights.write || rights.history || rights.download))
                || (rights.write && !config.files[resource].editable)
            {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_resource_grant_permission(),
                ));
            }
        }
    }
    Ok(())
}
/// Reading a managed author document is separate from authoring. Compiler
/// inputs and arbitrary JSON/binary files do not become document sessions.
fn author_file(inner: &HostedInner, resource: &str) -> Result<FileEntry, ApiError> {
    let file = inner.file(resource)?;
    if file.lifecycle != "live" {
        return Err(unavailable());
    }
    structure::guard_slot(inner, resource)?;
    content_import::qualify_member(inner, resource)?;
    if !file.editable {
        let path = inner.config.root.join(&file.path);
        if path.extension().is_none_or(|e| e != "typ") {
            return Err(unavailable());
        }
        let node = inner
            .core
            .qualified_node_at(&inner.config.root, &path)
            .map_err(|_| unavailable())?;
        if file
            .expected_node
            .as_ref()
            .is_some_and(|n| n != &node.owner)
        {
            return Err(unavailable());
        }
        inner
            .core
            .validate_snapshot_witness(&inner.config.root, &path, &node.witness)
            .map_err(|_| unavailable())?;
    }
    Ok(file)
}
impl HostedInner {
    /// Effective author locators are private projections of original producer
    /// facts. They are never written back into operator configuration.
    fn file(&self, resource: &str) -> Result<FileEntry, ApiError> {
        self.config
            .files
            .get(resource)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| content_import::member_file(self, resource)?.ok_or_else(unavailable))
    }
    fn grant(&self, actor: &str, resource: &str) -> Result<Rights, ApiError> {
        let user = self
            .policy
            .users
            .get(actor)
            .filter(|u| u.enabled)
            .ok_or_else(expired)?;
        user.grants
            .get(resource)
            .cloned()
            .or_else(|| content_import::member_rights(self, actor, resource))
            .ok_or_else(unavailable)
    }
    fn files_view(&self, actor: &str) -> Result<BTreeMap<String, FileEntry>, ApiError> {
        let mut files = self.config.files.clone();
        for id in content_import::member_ids(self, actor)? {
            files.insert(id.clone(), self.file(&id)?);
        }
        Ok(files)
    }
    fn grants_view(&self, actor: &str) -> Result<BTreeMap<String, Rights>, ApiError> {
        let mut grants = self
            .policy
            .users
            .get(actor)
            .filter(|u| u.enabled)
            .ok_or_else(expired)?
            .grants
            .clone();
        for id in content_import::member_ids(self, actor)? {
            grants.insert(id.clone(), self.grant(actor, &id)?);
        }
        Ok(grants)
    }
    fn actor(&self, auth: &Auth) -> Result<String, ApiError> {
        self.policy
            .users
            .iter()
            .find(|(_, user)| user.enabled && user.token_sha256 == auth.digest)
            .map(|(id, _)| id.clone())
            .ok_or_else(expired)
    }
    fn rights(&self, actor: &str, resource: &str) -> Result<Rights, ApiError> {
        if self.file(resource)?.lifecycle != "live" {
            return Err(unavailable());
        }
        let rights = self.grant(actor, resource)?;
        if !rights.read {
            return Err(unavailable());
        }
        structure::guard_slot(self, resource)?;
        content_import::qualify_member(self, resource)?;
        Ok(rights)
    }
    fn actor_generation(&self, actor: &str) -> u64 {
        self.policy.users.get(actor).map_or(0, |u| u.epoch)
    }
    fn session(&self, auth: &Auth, id: &str) -> Result<(String, HostedSession, Rights), ApiError> {
        let actor = self.actor(auth)?;
        let session = self
            .sessions
            .get(id)
            .filter(|s| s.actor == actor && s.generation == self.actor_generation(&actor))
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::GONE,
                    "sessionExpired",
                    UiMessage::error_api_hosted_session_expired_with_draft(),
                )
            })?;
        let rights = self.rights(&actor, &session.resource)?;
        let now = Instant::now();
        let mut expires = session
            .expires
            .lock()
            .map_err(|_| ApiError::internal(UiMessage::error_api_hosted_session_unavailable()))?;
        if *expires <= now {
            return Err(ApiError::new(
                StatusCode::GONE,
                "sessionExpired",
                UiMessage::error_api_hosted_session_expired_with_draft(),
            ));
        }
        *expires = now + SESSION_IDLE_TIMEOUT;
        Ok((actor, session.clone(), rights))
    }
    /// Release only the in-memory coordinate. Core's persisted personal draft
    /// and original-operation recovery records keep their existing ownership.
    fn release_session(&mut self, id: &str) -> Result<(), ApiError> {
        let Some(session) = self.sessions.get(id) else {
            return Ok(());
        };
        let scope = self.actor_scope(&session.actor);
        match self
            .core
            .with_actor_for_root(&scope, &self.config.root, |c| c.close_session(id))
        {
            Ok(()) => {}
            Err(error) if error.code == "sessionMissing" => {}
            Err(error) => return Err(Self::core_error(error)),
        }
        self.sessions.remove(id);
        self.editor_versions.remove(id);
        Ok(())
    }
    fn reap_idle_sessions(&mut self, now: Instant) -> Result<(), ApiError> {
        let expired = self
            .sessions
            .iter()
            .filter_map(|(id, session)| match session.expires.lock() {
                Ok(expires) if *expires <= now => Some(Ok(id.clone())),
                Ok(_) => None,
                Err(_) => Some(Err(ApiError::internal(
                    UiMessage::error_api_hosted_session_unavailable(),
                ))),
            })
            .collect::<Result<Vec<_>, _>>()?;
        for id in expired {
            self.release_session(&id)?;
        }
        Ok(())
    }
    fn core_error(error: CoreError) -> ApiError {
        // OS errors and recovery paths can contain absolute server locations.
        let explanation = match error.code.as_str() {
            "conflict" | "staleBase" | "draftConflict" => {
                UiMessage::error_api_hosted_current_basis_changed()
            }
            "operationConflict" => UiMessage::error_api_hosted_operation_content_conflict(),
            "identityConflict" => UiMessage::error_api_hosted_stable_identity_required(),
            "historyUnavailable" => UiMessage::error_api_hosted_original_history_unavailable(),
            "basisRequired" => UiMessage::error_api_hosted_installed_library_rule_required(),
            _ => UiMessage::error_api_hosted_file_operation_incomplete(),
        };
        let mut result = ApiError::new(StatusCode::CONFLICT, &error.code, explanation);
        result.details = error
            .current_source
            .map(|current_source| Box::new(ApiErrorDetails::CurrentSource { current_source }));
        result
    }
    fn actor_scope(&self, actor: &str) -> String {
        format!("{}:{actor}", self.config.deployment)
    }
    fn open(&mut self, actor: &str, resource: &str) -> Result<Value, ApiError> {
        let rights = self.rights(actor, resource)?;
        let file = author_file(self, resource)?;
        self.reap_idle_sessions(Instant::now())?;
        if self.sessions.len() >= 128
            || self.sessions.values().filter(|s| s.actor == actor).count() >= 32
        {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_open_session_budget(),
            ));
        }
        let path = self.config.root.join(&file.path);
        let root = self.config.root.clone();
        let scope = self.actor_scope(actor);
        let opened = self
            .core
            .with_actor_for_root(&scope, &self.config.root, |c| c.open(path, Some(root)))
            .map_err(Self::core_error)?;
        self.sessions.insert(
            opened.session_id.clone(),
            HostedSession {
                actor: actor.into(),
                generation: self.actor_generation(actor),
                resource: resource.into(),
                expires: Arc::new(Mutex::new(Instant::now() + SESSION_IDLE_TIMEOUT)),
            },
        );
        Ok(
            json!({"resource":resource,"title":file.title,"sessionId":opened.session_id,"baseId":opened.base_id,
            "source":opened.source,"recoveredDraft":opened.recovered_draft,"recoveredDraftBaseSource":opened.recovered_draft_base_source,
            "recoveredDraftConflict":opened.recovered_draft_conflict,"rights":rights}),
        )
    }
    fn files(
        &self,
        actor: &str,
        resources: &[String],
    ) -> Result<BTreeMap<String, Vec<u8>>, ApiError> {
        inputs::read_files(self, actor, resources)
    }
}

async fn locked<T: Send + 'static>(
    state: &HostedState,
    f: impl FnOnce(&mut HostedInner) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    let inner = state.inner.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = inner
            .lock()
            .map_err(|_| ApiError::internal(UiMessage::error_api_hosted_service_unavailable()))?;
        f(&mut guard)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_api_hosted_service_incomplete()))?
}
async fn boundary(State(state): State<HostedState>, mut request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&state.host)
        || headers
            .get(header::ORIGIN)
            .is_some_and(|v| v.to_str().ok() != Some(&state.origin))
    {
        return protected_response(
            ApiError::new(
                StatusCode::FORBIDDEN,
                "origin",
                UiMessage::error_api_hosted_origin_denied(),
            )
            .into_response(),
        );
    }
    if request.uri().path().starts_with("/api/") {
        let token = headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if token.len() < 32 || token.len() > 512 {
            return protected_response(expired().into_response());
        }
        let auth = Auth {
            digest: digest(token.as_bytes()),
        };
        let check = auth.clone();
        let admin = request.uri().path().starts_with("/api/hosted/admin/");
        if let Err(error) = locked(&state, move |inner| {
            if admin {
                if check.digest != inner.config.admin_token_sha256 {
                    return Err(expired());
                }
            } else {
                inner.actor(&check)?;
            }
            Ok(())
        })
        .await
        {
            return protected_response(error.into_response());
        }
        request.extensions_mut().insert(auth);
    }
    protected_response(next.run(request).await)
}
fn protected_response(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; font-src 'self'; connect-src 'self'; frame-src 'self' blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"));
    response
}

async fn me(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        Ok(json!({"actor":actor,"deployment":inner.config.deployment,"generation":inner.actor_generation(&actor),
            "version":env!("CARGO_PKG_VERSION"),"mode":"hosted","managedSync":inner.policy.users[&actor].sync_owner && inner.config.sync_identity.is_some()}))
    }).await?))
}
async fn list(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let files = inner
                .files_view(&actor)?
                .iter()
                .filter_map(|(id, f)| {
                    inner
                        .rights(&actor, id)
                        .ok()
                        .filter(|_| author_file(inner, id).is_ok())
                        .map(|rights| json!({"resource":id,"title":f.title,"rights":rights}))
                })
                .collect::<Vec<_>>();
            Ok(json!({"documents":files,"coverage":"configuredAuthorizedSources"}))
        })
        .await?,
    ))
}
async fn open(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Open>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            inner.open(&actor, &req.resource)
        })
        .await?,
    ))
}
async fn close(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SessionRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            inner
                .sessions
                .get(&req.session_id)
                .filter(|session| {
                    session.actor == actor && session.generation == inner.actor_generation(&actor)
                })
                .ok_or_else(|| {
                    ApiError::new(
                        StatusCode::GONE,
                        "sessionExpired",
                        UiMessage::error_api_hosted_session_expired(),
                    )
                })?;
            inner.release_session(&req.session_id)?;
            Ok(json!({"closed":true}))
        })
        .await?,
    ))
}
async fn keepalive(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SessionRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            inner.session(&auth, &req.session_id)?;
            Ok(json!({"active":true}))
        })
        .await?,
    ))
}
async fn draft(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SourceRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _, rights) = inner.session(&auth, &req.session_id)?;
            if !rights.write {
                return Err(unavailable());
            }
            inner
                .core
                .with_actor_for_root(&inner.actor_scope(&actor), &inner.config.root, |c| {
                    c.persist_draft(&req.session_id, &req.source)
                })
                .map_err(HostedInner::core_error)?;
            Ok(json!({"status":"draftRetained"}))
        })
        .await?,
    ))
}
async fn save(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SourceSaveRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let status = definition::save(
                inner,
                &auth,
                weftext_core::HostedSaveRequest {
                    session_id: req.session_id,
                    base_id: req.base_id,
                    source: req.source,
                    operation_id: req.operation_id,
                },
                req.definition_reservations,
            )?;
            value(status)
        })
        .await?,
    ))
}
async fn resolve_draft(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SourceRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _, rights) = inner.session(&auth, &req.session_id)?;
            if !rights.write {
                return Err(unavailable());
            }
            inner
                .core
                .with_actor_for_root(&inner.actor_scope(&actor), &inner.config.root, |c| {
                    c.resolve_draft(&req.session_id, "mergeRecovered", Some(&req.source))
                })
                .map_err(HostedInner::core_error)?;
            Ok(json!({"status":"draftRebased","explanation":UiMessage::error_api_hosted_draft_rebased(),"diagnostics":[],"causes":[]}))
        })
        .await?,
    ))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<OperationStatus>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let status = inner
                .core
                .with_actor_for_root(&inner.actor_scope(&actor), &inner.config.root, |c| {
                    c.actor_save_status(&req.operation_id)
                })
                .map_err(HostedInner::core_error)?;
            if let Some(status) = status {
                inner.rights(&actor, &status.resource)?;
                value(status)
            } else {
                Ok(json!({"operationId":req.operation_id,"status":"notRegistered"}))
            }
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Settle>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            if !inner.rights(&actor, &req.resource)?.write {
                return Err(unavailable());
            }
            value(
                inner
                    .core
                    .with_actor_for_root(&inner.actor_scope(&actor), &inner.config.root, |c| {
                        c.actor_settle(&req.resource, &req.operation_id)
                    })
                    .map_err(HostedInner::core_error)?,
            )
        })
        .await?,
    ))
}
async fn search(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Search>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        if req.text.is_empty() || req.text.len() > 256 { return Err(ApiError::bad(UiMessage::error_api_hosted_search_term_budget())); }
        let mut rows = Vec::new(); let mut complete = true;
        let docs = inner.files_view(&actor)?.iter().filter(|(id, _)| inner.rights(&actor, id).is_ok() && author_file(inner,id).is_ok()).map(|(id,f)| (id.clone(),f.clone())).collect::<Vec<_>>();
        let mut total = 0;
        for (id, file) in docs {
            match inner.files(&actor, std::slice::from_ref(&id)) {
                Ok(files) => {
                    let bytes = &files[&file.path]; total += bytes.len();
                    if total > 16 * 1024 * 1024 { complete = false; break; }
                    let text = std::str::from_utf8(bytes).map_err(|_| unavailable())?;
                    if text.contains(&req.text) || file.title.contains(&req.text) { rows.push(json!({"resource":id,"title":file.title})); }
                }
                Err(_) => complete = false,
            }
        }
        Ok(json!({"rows":rows,"complete":complete,"coverage":"configuredAuthorizedSources","queryKind":"literalSourceText"}))
    }).await?))
}
async fn preview(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<SourceRequest>,
) -> ApiResult {
    if req.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_preview_source_budget(),
        ));
    }
    let sid = req.session_id.clone();
    let check = auth.clone();
    let source = req.source;
    let (input, dependencies, environment, generation) = locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&check, &sid)?;
        let (input, dependencies, environment) =
            editor::freeze(inner, &actor, &session.resource, &source)?;
        Ok((
            input,
            dependencies,
            environment,
            inner.actor_generation(&actor),
        ))
    })
    .await?;
    // Queue capacity is bounded; await does not hold the authorization/writer lock.
    let _permit = state.render_gate.try_acquire().map_err(|_| {
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            UiMessage::error_api_hosted_preview_busy(),
        )
    })?;
    let output = render_worker(input).await;
    locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&auth, &req.session_id)?;
        if inner.actor_generation(&actor) != generation { return Err(unavailable()); }
        editor::validate_inputs(inner, &actor, &session.resource, &dependencies)?;
        environment.recheck(inner, &actor)?;
        let output = output.map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "previewUnavailable", UiMessage::error_api_hosted_preview_incomplete()))?;
        // Return rendered content only. Compiler paths, origins and ambient
        // metadata are not part of this bounded hosted presentation API.
        Ok(Json(json!({"ok":output["ok"],"html":output["html"],
            "explanation":if output["ok"] == true { None } else { Some(UiMessage::error_api_hosted_authorized_preview_unavailable()) },"diagnostics":[],"causes":[]})))
    }).await
}
async fn prepare_download(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Open>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        if !inner.rights(&actor, &req.resource)?.download { return Err(unavailable()); }
        inner.downloads.retain(|_, d| d.expires > Instant::now());
        if inner.downloads.len() >= 32 { return Err(ApiError::bad(UiMessage::error_api_hosted_download_count_budget())); }
        let files = inner.files(&actor, std::slice::from_ref(&req.resource))?;
        let bytes = files[&inner.file(&req.resource)?.path].clone();
        let ticket = uuid::Uuid::new_v4().to_string();
        let result = json!({"ticket":ticket,"length":bytes.len(),"sha256":digest(&bytes),"chunkSize":65536,"status":"frozen"});
        if inner.downloads.values().map(|d| d.bytes.len()).sum::<usize>() + bytes.len() > 64*1024*1024 { return Err(ApiError::bad(UiMessage::error_api_hosted_download_byte_budget())); }
        inner.downloads.insert(ticket, Download {generation:inner.actor_generation(&actor),actor,resource:req.resource,bytes,expires:Instant::now()+Duration::from_secs(900)});
        Ok(result)
    }).await?))
}
async fn chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Chunk>,
) -> Result<Response, ApiError> {
    locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let download = inner
            .downloads
            .get(&req.ticket)
            .filter(|d| {
                d.actor == actor
                    && d.generation == inner.actor_generation(&actor)
                    && d.expires > Instant::now()
            })
            .ok_or_else(unavailable)?;
        if !inner.rights(&actor, &download.resource)?.download || req.offset > download.bytes.len()
        {
            return Err(unavailable());
        }
        let end = (req.offset + 65536).min(download.bytes.len());
        Ok((
            [(header::CONTENT_TYPE, "application/octet-stream")],
            download.bytes[req.offset..end].to_vec(),
        )
            .into_response())
    })
    .await
}
async fn update_policy(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<PolicyUpdate>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            if auth.digest != inner.config.admin_token_sha256 {
                return Err(expired());
            }
            if req.expected_generation != inner.policy.generation {
                return Err(ApiError::new(
                    StatusCode::CONFLICT,
                    "policyConflict",
                    UiMessage::error_api_hosted_policy_generation_conflict(),
                ));
            }
            validate_users(&inner.config, &req.users)?;
            let mut next = Policy {
                deployment: inner.policy.deployment.clone(),
                sync_identity: inner.policy.sync_identity.clone(),
                generation: inner.policy.generation.checked_add(1).ok_or_else(|| {
                    ApiError::internal(UiMessage::error_api_hosted_policy_generation_exhausted())
                })?,
                users: req.users,
            };
            for (actor, user) in &mut next.users {
                user.epoch = inner
                    .policy
                    .users
                    .get(actor)
                    .filter(|old| {
                        old.enabled == user.enabled
                            && old.token_sha256 == user.token_sha256
                            && old.grants == user.grants
                            && old.query_providers == user.query_providers
                            && old.query_scopes == user.query_scopes
                            && old.library == user.library
                            && old.native_whole_library == user.native_whole_library
                            && old.navigation == user.navigation
                            && old.content_imports == user.content_imports
                            && old.owner_backup == user.owner_backup
                            && old.resources == user.resources
                            && old.resource_collections == user.resource_collections
                            && old.structure == user.structure
                            && old.sync_owner == user.sync_owner
                    })
                    .map_or(next.generation, |old| old.epoch);
            }
            inner
                .core
                .store_hosted_policy(&serde_json::to_vec(&next).map_err(|_| {
                    ApiError::internal(UiMessage::error_api_hosted_policy_encoding_failed())
                })?)
                .map_err(HostedInner::core_error)?;
            inner.policy = next;
            let expired_sessions = inner
                .sessions
                .iter()
                .filter(|(_, s)| s.generation != inner.actor_generation(&s.actor))
                .map(|(id, s)| (id.clone(), s.clone()))
                .collect::<Vec<_>>();
            for (id, _) in expired_sessions {
                inner.release_session(&id)?;
            }
            inner.downloads.retain(|_, d| {
                inner
                    .policy
                    .users
                    .get(&d.actor)
                    .is_some_and(|u| u.epoch == d.generation)
            });
            sync_transport::expire_revoked(inner);
            fields::expire_revoked(inner);
            review::expire_revoked(inner);
            office::expire_revoked(inner);
            owner_backup::expire_revoked(inner);
            media::expire_revoked(inner);
            Ok(json!({"generation":inner.policy.generation,"status":"applied"}))
        })
        .await?,
    ))
}
async fn policy_read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            if auth.digest != inner.config.admin_token_sha256 {
                return Err(expired());
            }
            value(&inner.policy)
        })
        .await?,
    ))
}
async fn hosted_assets(request: Request) -> Response {
    let path = request.uri().path();
    if path == "/" || path == "/hosted.html" {
        return (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            ASSETS
                .get_file("hosted.html")
                .map_or(&[][..], |f| f.contents()),
        )
            .into_response();
    }
    if path.starts_with("/assets/") || path.starts_with("/fonts/") {
        return super::assets(request).await;
    }
    StatusCode::NOT_FOUND.into_response()
}
pub(super) async fn serve(
    core: Core,
    config_path: PathBuf,
    legacy_root: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config: Config = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    config.root = std::path::absolute(&config.root)?;
    if !valid_id(&config.deployment)
        || !valid_digest(&config.admin_token_sha256)
        || config.files.len() > 256
        || config.directories.len() > 64
    {
        return Err(ErrorDescription::new(
            UiMessage::error_api_hosted_configuration_invalid_deployment(),
        )
        .into());
    }
    let mut paths = BTreeSet::new();
    if config.style_inputs.len() > 64
        || config
            .style_inputs
            .iter()
            .any(|id| !config.files.contains_key(id) && !media::valid_input(&config, id))
        || config.style_inputs.iter().collect::<BTreeSet<_>>().len() != config.style_inputs.len()
    {
        return Err(ErrorDescription::new(
            UiMessage::error_api_hosted_configuration_invalid_style_inputs(),
        )
        .into());
    }
    for (id, file) in &config.files {
        let p = std::path::Path::new(&file.path);
        if !valid_id(id)
            || file.path.is_empty()
            || p.components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
            || file.path.contains(':')
            || !paths.insert(file.path.replace('\\', "/").to_lowercase())
            || (file.editable && p.extension().is_none_or(|e| e != "typ"))
            || (file.importable
                && (!file.editable
                    || p.file_name().is_none_or(|n| n != "document.typ")
                    || p.parent()
                        .and_then(|p| p.file_name())
                        .is_none_or(|n| n != "document")
                    || p.parent()
                        .and_then(|p| p.parent())
                        .and_then(|p| p.file_name())
                        .is_none()))
            || file.title.len() > 500
            || file.inputs.len() > 64
            || !query_scopes::valid_file(file)
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_hosted_configuration_invalid_configured_resource(),
            )
            .into());
        }
        if file
            .inputs
            .iter()
            .any(|i| (!config.files.contains_key(i) && !media::valid_input(&config, i)) || i == id)
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_hosted_configuration_invalid_configured_dependency(),
            )
            .into());
        }
    }
    let mut references = BTreeSet::new();
    if !media::valid_config(&config)
        || !content_import::valid_config(&config)
        || !owner_backup::valid_config(&config)
        || !navigation_registry::valid_config(&config)
        || !resource_collections::valid_config(&config)
        || !structure::valid_config(&config)
    {
        return Err(ErrorDescription::new(
            UiMessage::error_api_hosted_configuration_invalid_owned_resource(),
        )
        .into());
    }
    for (id, directory) in &config.directories {
        if !valid_id(id)
            || !query_scopes::valid_directory(directory)
            || !references.insert(directory.reference.clone())
        {
            return Err(ErrorDescription::new(
                UiMessage::error_api_hosted_configuration_invalid_directory_scope(),
            )
            .into());
        }
    }
    validate_users(&config, &config.users).map_err(ApiError::into_description)?;
    if let Some(identity) = &config.sync_identity {
        match core.sync_binding_info(&config.root) {
            Ok(binding) if binding.identity != *identity => {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_hosted_configuration_sync_pairing_mismatch(),
                )
                .into());
            }
            Err(error) if error.code != "syncNotEnabled" => return Err(error.into()),
            _ => {}
        }
        core.receive_sync(identity, weftext_core::sync::SyncBundle::default())?;
    }
    for user in config.users.values_mut() {
        user.epoch = 1;
    }
    let policy = match core.hosted_policy()? {
        Some(bytes) => {
            let mut policy: Policy = serde_json::from_slice(&bytes)?;
            if policy.deployment != config.deployment {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_hosted_configuration_deployment_mismatch(),
                )
                .into());
            }
            if policy.sync_identity != config.sync_identity {
                if policy.sync_identity.is_none()
                    && config.sync_identity.is_some()
                    && policy.users.values().all(|u| !u.sync_owner)
                {
                    // Adding a transport does not reapply bootstrap grants.
                    policy.sync_identity = config.sync_identity.clone();
                    policy.generation =
                        policy
                            .generation
                            .checked_add(1)
                            .ok_or(ErrorDescription::new(
                            UiMessage::error_api_hosted_configuration_policy_generation_exhausted(),
                        ))?;
                    core.store_hosted_policy(&serde_json::to_vec(&policy)?)?;
                } else {
                    return Err(ErrorDescription::new(
                        UiMessage::error_api_hosted_configuration_persisted_pairing_mismatch(),
                    )
                    .into());
                }
            }
            validate_users(&config, &policy.users).map_err(ApiError::into_description)?;
            policy
        }
        None => {
            let policy = Policy {
                deployment: config.deployment.clone(),
                generation: 1,
                users: config.users.clone(),
                sync_identity: config.sync_identity.clone(),
            };
            core.store_hosted_policy(&serde_json::to_vec(&policy)?)?;
            policy
        }
    };
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let address = listener.local_addr()?;
    let origin = match config.origin.clone() {
        Some(origin) => origin,
        None if address.ip().is_loopback() => format!("http://{address}"),
        None => {
            return Err(ErrorDescription::new(
                UiMessage::error_api_hosted_configuration_explicit_origin_required(),
            )
            .into());
        }
    };
    let parsed = url::Url::parse(&origin)?;
    if parsed.origin().ascii_serialization() != origin
        || !matches!(parsed.scheme(), "http" | "https")
        || (!address.ip().is_loopback() && parsed.scheme() == "http" && !config.allow_insecure_http)
    {
        return Err(ErrorDescription::new(
            UiMessage::error_api_hosted_configuration_secure_origin_required(),
        )
        .into());
    }
    let host = origin
        .split_once("://")
        .ok_or(ErrorDescription::new(
            UiMessage::error_api_hosted_configuration_invalid_origin(),
        ))?
        .1
        .to_owned();
    let state = HostedState {
        inner: Arc::new(Mutex::new(HostedInner {
            core,
            config,
            policy,
            sessions: HashMap::new(),
            downloads: HashMap::new(),
            sync: sync_transport::Store::default(),
            editor_versions: HashMap::new(),
            fields: fields::Store::default(),
            review: review::Store::default(),
            office: office::Store::with_legacy_root(legacy_root),
            library: library::Store::default(),
            media: media::Store::default(),
            navigation_registry: Default::default(),
            owner_backup: Default::default(),
        })),
        origin: origin.clone().into(),
        host: host.into(),
        render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
    };
    let reaper_state = state.clone();
    let app = Router::new()
        .merge(builtin_module::router())
        .merge(sync_transport::router())
        .merge(editor::router())
        .merge(fields::router())
        .merge(history_api::router())
        .merge(navigation::router())
        .merge(navigation_registry::router())
        .merge(content_import::router())
        .merge(recognition::router())
        .merge(owner_backup::router())
        .merge(library::router())
        .merge(knowledge_links::router())
        .merge(citations::router())
        .merge(media::router())
        .merge(resource_collections::router())
        .merge(structure::router())
        .merge(query::router())
        .merge(definition::router())
        .merge(embedded_query::router())
        .merge(review::router())
        .merge(office::router())
        .merge(office_import::router())
        .route("/api/hosted/me", get(me))
        .route("/api/hosted/documents", get(list))
        .route("/api/hosted/open", post(open))
        .route("/api/hosted/close", post(close))
        .route("/api/hosted/session/keepalive", post(keepalive))
        .route("/api/hosted/save", post(save))
        .route("/api/hosted/draft", post(draft))
        .route("/api/hosted/draft/resolve", post(resolve_draft))
        .route("/api/hosted/operation", post(status))
        .route("/api/hosted/operation/settle", post(settle))
        .route("/api/hosted/search", post(search))
        .route("/api/hosted/preview", post(preview))
        .route("/api/hosted/download/prepare", post(prepare_download))
        .route("/api/hosted/download/chunk", post(chunk))
        .route(
            "/api/hosted/admin/policy",
            get(policy_read).post(update_policy),
        )
        .fallback(hosted_assets)
        .layer(DefaultBodyLimit::max(3 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state);
    println!(
        "Weftext {} hosted\n{origin}/\nBound to {address}; operator-managed resources only.",
        env!("CARGO_PKG_VERSION")
    );
    let reaper = tokio::spawn(async move {
        let mut ticks = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticks.tick().await;
            if locked(&reaper_state, |inner| {
                inner.reap_idle_sessions(Instant::now())
            })
            .await
            .is_err()
            {
                eprintln!("Hosted session cleanup did not complete; it will be retried.");
            }
        }
    });
    let result = axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    reaper.abort();
    let _ = reaper.await;
    result?;
    Ok(())
}
