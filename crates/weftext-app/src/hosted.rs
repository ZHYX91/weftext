//! A separate hosted boundary. Local path, library, recovery and feature APIs
//! are deliberately not mounted here. Every resource comes from operator config.
use super::*;
mod definition;
mod editor;
mod embedded_query;
mod fields;
mod history_api;
mod library;
mod navigation;
mod office;
mod office_import;
mod query;
mod query_providers;
mod query_scopes;
mod review;
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
    sync_owner: bool,
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
}
#[derive(Clone)]
struct HostedSession {
    actor: String,
    generation: u64,
    resource: String,
}
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
        "内容不可用或当前无权访问，请重新连接",
    )
}
fn expired() -> ApiError {
    ApiError::new(
        StatusCode::UNAUTHORIZED,
        "authentication",
        "访问凭据已失效，请重新连接",
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
        return Err(ApiError::bad("用户上限为 128"));
    }
    for (id, user) in users {
        if !library::valid_rights(&user.library) {
            return Err(ApiError::bad("库配置写入须具备同角色读取权"));
        }
        if !query_scopes::validate_rights(config, &user.query_scopes) {
            return Err(ApiError::bad("目录范围授权须引用有界的明确配置目录"));
        }
        if !query_providers::valid_rights(&user.query_providers) {
            return Err(ApiError::bad(
                "Provider 与 SeriesPolicy 授权须使用有界的明确 ID",
            ));
        }
        if user.sync_owner && config.sync_identity.is_none() {
            return Err(ApiError::bad("完整同步授权须先在部署配置中明确配对身份"));
        }
        if !valid_id(id)
            || !valid_digest(&user.token_sha256)
            || !tokens.insert(user.token_sha256.clone())
        {
            return Err(ApiError::bad("用户标识或唯一凭据摘要无效"));
        }
        for (resource, rights) in &user.grants {
            if config
                .files
                .get(resource)
                .is_some_and(|f| f.lifecycle == "trash")
                && (rights.write
                    || rights.fields.values().any(|r| r.write)
                    || rights.annotations.write
                    || rights.annotations.accept
                    || rights.annotations.proofread
                    || rights.office.r#import)
            {
                return Err(ApiError::bad(
                    "Trash 对象只读，不能配置正文、字段、批注、校对或导入写入权限",
                ));
            }
            if !query::valid_rights(&rights.query) {
                return Err(ApiError::bad(
                    "查询定义历史须具备定义读取权；语义产品及关系须明确授权",
                ));
            }
            if rights.office.r#import
                && (!rights.read
                    || !rights.write
                    || !config
                        .files
                        .get(resource)
                        .is_some_and(|f| f.importable && f.editable))
            {
                return Err(ApiError::bad(
                    "Office 导入须具备配置空目标的正文读写和明确导入权限",
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
                    "批注写入须具备批注读取权；校对和应用建议须具备各自正文与目标权限",
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
                    "字段授权须使用明确 FieldId；写入须同时授予该字段读取权",
                ));
            }
            if !config.files.contains_key(resource)
                || (!rights.read && (rights.write || rights.history || rights.download))
                || (rights.write && !config.files[resource].editable)
            {
                return Err(ApiError::bad(
                    "授权须引用已配置对象，写入、历史、下载还须具备读取权",
                ));
            }
        }
    }
    Ok(())
}
impl HostedInner {
    fn actor(&self, auth: &Auth) -> Result<String, ApiError> {
        self.policy
            .users
            .iter()
            .find(|(_, user)| user.enabled && user.token_sha256 == auth.digest)
            .map(|(id, _)| id.clone())
            .ok_or_else(expired)
    }
    fn rights(&self, actor: &str, resource: &str) -> Result<Rights, ApiError> {
        if !self
            .config
            .files
            .get(resource)
            .is_some_and(|f| f.lifecycle == "live")
        {
            return Err(unavailable());
        }
        self.policy
            .users
            .get(actor)
            .filter(|u| u.enabled)
            .and_then(|u| u.grants.get(resource))
            .filter(|r| r.read)
            .cloned()
            .ok_or_else(unavailable)
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
            .cloned()
            .ok_or_else(|| {
                ApiError::new(
                    StatusCode::GONE,
                    "sessionExpired",
                    "会话已失效，请重新读取并核对草稿",
                )
            })?;
        let rights = self.rights(&actor, &session.resource)?;
        Ok((actor, session, rights))
    }
    fn core_error(error: CoreError) -> ApiError {
        // OS errors and recovery paths can contain absolute server locations.
        let message = match error.code.as_str() {
            "conflict" | "staleBase" | "draftConflict" => {
                "当前稿或原保存依据已变化。双方稿已保留，请重新打开并明确合并。"
            }
            "operationConflict" => "同一操作编号不能用于不同内容。",
            "identityConflict" => "须保留笔记的稳定身份声明。",
            "historyUnavailable" => "所选历史版本不可用，未替换为当前版本。",
            "basisRequired" => {
                "目录或 Trash 定义的保存保护需要现存的库规则文件；请由库所有者显式安装后重试。"
            }
            _ => "文件操作未完成，草稿仍可保留；请查看服务端恢复状态。",
        };
        ApiError {
            status: StatusCode::CONFLICT,
            code: error.code,
            message: message.into(),
            details: error
                .current_source
                .map(|s| Box::new(json!({"currentSource":s}))),
        }
    }
    fn actor_scope(&self, actor: &str) -> String {
        format!("{}:{actor}", self.config.deployment)
    }
    fn open(&mut self, actor: &str, resource: &str) -> Result<Value, ApiError> {
        let rights = self.rights(actor, resource)?;
        let file = self
            .config
            .files
            .get(resource)
            .filter(|f| f.editable)
            .cloned()
            .ok_or_else(unavailable)?;
        if self.sessions.len() >= 128
            || self.sessions.values().filter(|s| s.actor == actor).count() >= 32
        {
            return Err(ApiError::bad("打开会话已达 32 个，请关闭不使用的文档"));
        }
        let path = self.config.root.join(&file.path);
        let root = self.config.root.clone();
        let scope = self.actor_scope(actor);
        let opened = self
            .core
            .with_actor(&scope, |c| c.open(path, Some(root)))
            .map_err(Self::core_error)?;
        self.sessions.insert(
            opened.session_id.clone(),
            HostedSession {
                actor: actor.into(),
                generation: self.actor_generation(actor),
                resource: resource.into(),
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
        let mut paths = BTreeSet::new();
        for resource in resources {
            self.rights(actor, resource)?;
            paths.insert(
                self.config
                    .files
                    .get(resource)
                    .ok_or_else(unavailable)?
                    .path
                    .clone(),
            );
        }
        self.core
            .hosted_files(&self.config.root, &paths.into_iter().collect::<Vec<_>>())
            .map_err(|_| unavailable())
    }
}

async fn locked<T: Send + 'static>(
    state: &HostedState,
    f: impl FnOnce(&mut HostedInner) -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    let inner = state.inner.clone();
    tokio::task::spawn_blocking(move || {
        let mut guard = inner.lock().map_err(|_| ApiError::internal("服务不可用"))?;
        f(&mut guard)
    })
    .await
    .map_err(|_| ApiError::internal("服务未完成"))?
}
async fn boundary(State(state): State<HostedState>, mut request: Request, next: Next) -> Response {
    let headers = request.headers();
    if headers.get(header::HOST).and_then(|v| v.to_str().ok()) != Some(&state.host)
        || headers
            .get(header::ORIGIN)
            .is_some_and(|v| v.to_str().ok() != Some(&state.origin))
    {
        return protected_response(
            ApiError::new(StatusCode::FORBIDDEN, "origin", "不允许的访问来源").into_response(),
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
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; connect-src 'self'; frame-src 'self' blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"));
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
                .config
                .files
                .iter()
                .filter_map(|(id, f)| {
                    inner
                        .rights(&actor, id)
                        .ok()
                        .filter(|_| f.editable)
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
    Json(req): Json<Open>,
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
    Json(req): Json<SessionRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _, _) = inner.session(&auth, &req.session_id)?;
            inner
                .core
                .with_actor(&inner.actor_scope(&actor), |c| {
                    c.close_session(&req.session_id)
                })
                .map_err(HostedInner::core_error)?;
            inner.sessions.remove(&req.session_id);
            inner.editor_versions.remove(&req.session_id);
            Ok(json!({"closed":true}))
        })
        .await?,
    ))
}
async fn draft(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<SourceRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _, rights) = inner.session(&auth, &req.session_id)?;
            if !rights.write {
                return Err(unavailable());
            }
            inner
                .core
                .with_actor(&inner.actor_scope(&actor), |c| {
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
    Json(req): Json<SourceSaveRequest>,
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
    Json(req): Json<SourceRequest>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let (actor, _, rights) = inner.session(&auth, &req.session_id)?;
            if !rights.write {
                return Err(unavailable());
            }
            inner
                .core
                .with_actor(&inner.actor_scope(&actor), |c| {
                    c.resolve_draft(&req.session_id, "mergeRecovered", Some(&req.source))
                })
                .map_err(HostedInner::core_error)?;
            Ok(json!({"status":"draftRebased","message":"原草稿已保留；尚未保存正文"}))
        })
        .await?,
    ))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<OperationStatus>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let status = inner
                .core
                .with_actor(&inner.actor_scope(&actor), |c| {
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
    Json(req): Json<Settle>,
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
                    .with_actor(&inner.actor_scope(&actor), |c| {
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
    Json(req): Json<Search>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        if req.text.is_empty() || req.text.len() > 256 { return Err(ApiError::bad("搜索词须为 1–256 字节")); }
        let mut rows = Vec::new(); let mut complete = true;
        let docs = inner.config.files.iter().filter(|(id, f)| f.editable && inner.rights(&actor, id).is_ok()).map(|(id,f)| (id.clone(),f.clone())).collect::<Vec<_>>();
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
    Json(req): Json<SourceRequest>,
) -> ApiResult {
    if req.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(ApiError::bad("预览上限为 2 MiB"));
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
            "预览正在运行，请稍后再试",
        )
    })?;
    let output = render_worker(input).await;
    locked(&state, move |inner| {
        let (actor, session, _) = inner.session(&auth, &req.session_id)?;
        if inner.actor_generation(&actor) != generation { return Err(unavailable()); }
        editor::validate_inputs(inner, &actor, &session.resource, &dependencies)?;
        environment.recheck(inner, &actor)?;
        let output = output.map_err(|_| ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "previewUnavailable", "预览未完成，源码与草稿仍保留。"))?;
        // Return rendered content only. Compiler paths, origins and ambient
        // metadata are not part of this bounded hosted presentation API.
        Ok(Json(json!({"ok":output["ok"],"html":output["html"],
            "message":if output["ok"] == true { "" } else { "无法在当前获权依赖内预览；源码和草稿仍保留。" }})))
    }).await
}
async fn prepare_download(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Open>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        if !inner.rights(&actor, &req.resource)?.download { return Err(unavailable()); }
        inner.downloads.retain(|_, d| d.expires > Instant::now());
        if inner.downloads.len() >= 32 { return Err(ApiError::bad("暂存下载已达上限，请稍后重试")); }
        let files = inner.files(&actor, std::slice::from_ref(&req.resource))?;
        let bytes = files[&inner.config.files[&req.resource].path].clone();
        let ticket = uuid::Uuid::new_v4().to_string();
        let result = json!({"ticket":ticket,"length":bytes.len(),"sha256":digest(&bytes),"chunkSize":65536,"status":"frozen"});
        if inner.downloads.values().map(|d| d.bytes.len()).sum::<usize>() + bytes.len() > 64*1024*1024 { return Err(ApiError::bad("下载暂存总量已达 64 MiB")); }
        inner.downloads.insert(ticket, Download {generation:inner.actor_generation(&actor),actor,resource:req.resource,bytes,expires:Instant::now()+Duration::from_secs(900)});
        Ok(result)
    }).await?))
}
async fn chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Chunk>,
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
    Json(req): Json<PolicyUpdate>,
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
                    "授权策略已变化，请读取当前代次",
                ));
            }
            validate_users(&inner.config, &req.users)?;
            let mut next = Policy {
                deployment: inner.policy.deployment.clone(),
                sync_identity: inner.policy.sync_identity.clone(),
                generation: inner
                    .policy
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| ApiError::internal("授权代次耗尽"))?,
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
                            && old.sync_owner == user.sync_owner
                    })
                    .map_or(next.generation, |old| old.epoch);
            }
            inner
                .core
                .store_hosted_policy(
                    &serde_json::to_vec(&next).map_err(|_| ApiError::internal("策略编码失败"))?,
                )
                .map_err(HostedInner::core_error)?;
            inner.policy = next;
            let expired_sessions = inner
                .sessions
                .iter()
                .filter(|(_, s)| s.generation != inner.actor_generation(&s.actor))
                .map(|(id, s)| (id.clone(), s.clone()))
                .collect::<Vec<_>>();
            for (id, session) in expired_sessions {
                inner.sessions.remove(&id);
                inner.editor_versions.remove(&id);
                let scope = format!("{}:{}", inner.config.deployment, session.actor);
                inner
                    .core
                    .with_actor(&scope, |c| c.close_session(&id))
                    .map_err(HostedInner::core_error)?;
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
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config: Config = serde_json::from_slice(&std::fs::read(&config_path)?)?;
    config.root = std::path::absolute(&config.root)?;
    if !valid_id(&config.deployment)
        || !valid_digest(&config.admin_token_sha256)
        || config.files.len() > 256
        || config.directories.len() > 64
    {
        return Err("Invalid deployment, administrator digest or resource budget".into());
    }
    let mut paths = BTreeSet::new();
    if config.style_inputs.len() > 64
        || config
            .style_inputs
            .iter()
            .any(|id| !config.files.contains_key(id))
        || config.style_inputs.iter().collect::<BTreeSet<_>>().len() != config.style_inputs.len()
    {
        return Err("Invalid explicit style inputs".into());
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
            return Err("Invalid configured resource".into());
        }
        if file
            .inputs
            .iter()
            .any(|i| !config.files.contains_key(i) || i == id)
        {
            return Err("Invalid configured dependency".into());
        }
    }
    let mut references = BTreeSet::new();
    for (id, directory) in &config.directories {
        if !valid_id(id)
            || !query_scopes::valid_directory(directory)
            || !references.insert(directory.reference.clone())
        {
            return Err("Invalid configured directory scope".into());
        }
    }
    validate_users(&config, &config.users).map_err(|e| e.message)?;
    if let Some(identity) = &config.sync_identity {
        match core.sync_binding_info(&config.root) {
            Ok(binding) if binding.identity != *identity => {
                return Err("Configured sync pairing differs from the served replica".into());
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
                return Err("Private server state belongs to another deployment".into());
            }
            if policy.sync_identity != config.sync_identity {
                if policy.sync_identity.is_none()
                    && config.sync_identity.is_some()
                    && policy.users.values().all(|u| !u.sync_owner)
                {
                    // Adding a transport does not reapply bootstrap grants.
                    policy.sync_identity = config.sync_identity.clone();
                    policy.generation = policy
                        .generation
                        .checked_add(1)
                        .ok_or("Policy generation exhausted")?;
                    core.store_hosted_policy(&serde_json::to_vec(&policy)?)?;
                } else {
                    return Err("Persisted sync pairing differs from configuration; existing grants cannot silently move to another family".into());
                }
            }
            validate_users(&config, &policy.users).map_err(|e| e.message)?;
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
        None => return Err("Non-loopback binding requires an explicit origin".into()),
    };
    let parsed = url::Url::parse(&origin)?;
    if parsed.origin().ascii_serialization() != origin
        || !matches!(parsed.scheme(), "http" | "https")
        || (!address.ip().is_loopback() && parsed.scheme() == "http" && !config.allow_insecure_http)
    {
        return Err("Use an exact HTTPS origin, or explicitly allow insecure HTTP for a controlled test network".into());
    }
    let host = origin
        .split_once("://")
        .ok_or("Invalid origin")?
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
            office: office::Store::default(),
            library: library::Store::default(),
        })),
        origin: origin.clone().into(),
        host: host.into(),
        render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
    };
    let app = Router::new()
        .merge(sync_transport::router())
        .merge(editor::router())
        .merge(fields::router())
        .merge(history_api::router())
        .merge(navigation::router())
        .merge(library::router())
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
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
