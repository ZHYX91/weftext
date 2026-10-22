use crate::owned_process::{Command, Stdio};
use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use weftext_core::{Core, CoreError, SaveAsRequest, SaveRequest};

#[cfg(test)]
trait CoreTestFixture {
    fn create_note_fixture(
        &mut self,
        root: impl AsRef<std::path::Path>,
        parent: impl AsRef<std::path::Path>,
        name: &str,
    ) -> weftext_core::Result<weftext_core::OpenedDocument>;
}
#[cfg(test)]
impl CoreTestFixture for Core {
    fn create_note_fixture(
        &mut self,
        root: impl AsRef<std::path::Path>,
        parent: impl AsRef<std::path::Path>,
        name: &str,
    ) -> weftext_core::Result<weftext_core::OpenedDocument> {
        let receipt = self.create_note(root.as_ref(), parent, name)?;
        self.open(
            std::path::Path::new(&receipt.path).join(format!("{name}.typ")),
            Some(root.as_ref().to_path_buf()),
        )
    }
}

#[cfg(windows)]
mod artifact_catalog;
mod bibliography_import_api;
mod broker;
mod builtin_module_api;
mod capture_api;
mod citation_api;
pub mod cli;
pub mod client_protocol;
mod data_api;
mod desktop_api;
mod embedded_output;
mod embedded_query_api;
mod export_api;
mod export_conversions;
mod history_api;
#[cfg(windows)]
mod host_contract;
mod host_environment;
#[cfg(windows)]
mod host_paths;
#[cfg(windows)]
mod host_state;
mod hosted;
#[cfg(windows)]
mod hosted_owner;
mod knowledge_api;
pub mod launch;
mod legacy_office;
pub mod local_client;
pub mod local_service;
mod media_api;
#[cfg(windows)]
mod native;
mod navigation_api;
mod owned_process;
mod processing_api;
mod recognition_api;
mod remote_sync;
mod sync_api;
mod sync_wire;
mod work;
mod worker_failure;
mod writing_api;

static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../app/dist");

#[derive(Clone)]
struct AppState {
    core: Arc<Mutex<Core>>,
    token: Arc<str>,
    port: u16,
    locations: Arc<Mutex<HashMap<String, Location>>>,
    renders: Arc<Mutex<HashMap<String, RenderCache>>>,
    editor_versions: Arc<Mutex<HashMap<String, writing_api::EditorVersion>>>,
    exports: Arc<Mutex<export_api::ExportStore>>,
    render_gate: Arc<tokio::sync::Semaphore>,
    native_status: Arc<Mutex<desktop_api::HostStatus>>,
    sync_remote: Arc<Mutex<remote_sync::Store>>,
    // Route and middleware state clones share the immutable launch locator;
    // bootstrap still reopens its original identity and current Draft.
    initial: Option<Arc<Value>>,
}

#[derive(Clone)]
struct Location {
    path: PathBuf,
    root: PathBuf,
}
#[derive(Clone)]
struct RenderCache {
    source: String,
    serial: u64,
    output: Value,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
    details: Option<Box<Value>>,
    private_worker_failure: Option<Arc<worker_failure::WorkerFailure>>,
}
impl ApiError {
    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
            private_worker_failure: None,
        }
    }
    fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_request", message)
    }
    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
    }
    fn worker_failure(failure: worker_failure::WorkerFailure) -> Self {
        let mut error = Self::worker_failure_summary(&failure);
        error.private_worker_failure = Some(Arc::new(failure));
        error
    }
    fn worker_failure_summary(failure: &worker_failure::WorkerFailure) -> Self {
        use worker_failure::Kind;
        let (status, code, message) = if !failure.cleanup_confirmed {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "compiler_exit_unknown",
                "The compiler's actual process exit is unconfirmed; the original input is retained",
            )
        } else {
            match failure.kind {
                Kind::InputBudget => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "compiler_input_budget",
                    "The complete frozen compiler input exceeds its byte budget",
                ),
                Kind::Deadline => (
                    StatusCode::REQUEST_TIMEOUT,
                    "compile_timeout",
                    "The compiler exceeded its deadline; the original input is retained",
                ),
                Kind::NonzeroExit => (
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "compiler_failed",
                    "The compiler stopped with a nonzero result; the original input is retained",
                ),
                Kind::StdoutBudget | Kind::StderrBudget => (
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "compiler_output_budget",
                    "The compiler stream exceeded its byte budget; the original input is retained",
                ),
                Kind::OutputRejected => (
                    StatusCode::BAD_GATEWAY,
                    "compiler_output_invalid",
                    "The original compiler output does not satisfy its protocol",
                ),
                Kind::SupervisionLost => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "compiler_supervisor_unknown",
                    "The compiler supervisor did not return its actual completion",
                ),
                Kind::Setup => (
                    StatusCode::SERVICE_UNAVAILABLE,
                    "compiler_unavailable",
                    "The contained compiler process could not be started",
                ),
                Kind::InputEncoding => (
                    StatusCode::BAD_REQUEST,
                    "compiler_input_invalid",
                    "The frozen compiler input could not be encoded",
                ),
                Kind::InputIo | Kind::StdoutIo | Kind::StderrIo | Kind::Wait => (
                    StatusCode::BAD_GATEWAY,
                    "compiler_transport_failed",
                    "The original compiler transport did not complete",
                ),
            }
        };
        Self::new(status, code, message)
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.private_worker_failure
            .as_deref()
            .map(|failure| failure as &dyn std::error::Error)
    }
}
impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        let value = serde_json::to_value(&error).unwrap_or(json!({"message":"文件操作失败"}));
        let code = value["code"].as_str().unwrap_or("file_error").to_owned();
        let message = value["message"]
            .as_str()
            .unwrap_or("文件操作失败")
            .to_owned();
        Self {
            status: StatusCode::CONFLICT,
            code,
            message,
            details: Some(Box::new(value)),
            private_worker_failure: None,
        }
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"error":{"code":self.code,"message":self.message,"details":self.details}})),
        )
            .into_response()
    }
}
type ApiResult = Result<Json<Value>, ApiError>;

async fn core_call<T, F>(state: &AppState, f: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&mut Core) -> Result<T, CoreError> + Send + 'static,
{
    let core = state.core.clone();
    crate::work::spawn_blocking(move || {
        let mut core = core
            .lock()
            .map_err(|_| ApiError::internal("文件服务暂不可用"))?;
        f(&mut core).map_err(ApiError::from)
    })
    .await
    .map_err(|_| ApiError::internal("文件服务未完成"))?
}
fn value<T: Serialize>(v: T) -> Result<Value, ApiError> {
    serde_json::to_value(v).map_err(|_| ApiError::internal("响应编码失败"))
}
fn remember(state: &AppState, document: &Value) -> Result<(), ApiError> {
    let id = document["sessionId"]
        .as_str()
        .ok_or_else(|| ApiError::internal("缺少文件会话"))?;
    let path = document["path"]
        .as_str()
        .ok_or_else(|| ApiError::internal("缺少文件路径"))?;
    let root = document["root"]
        .as_str()
        .ok_or_else(|| ApiError::internal("缺少依赖根"))?;
    state
        .locations
        .lock()
        .map_err(|_| ApiError::internal("会话暂不可用"))?
        .insert(
            id.to_owned(),
            Location {
                path: path.into(),
                root: root.into(),
            },
        );
    Ok(())
}
fn location(state: &AppState, id: &str) -> Result<Location, ApiError> {
    state
        .locations
        .lock()
        .map_err(|_| ApiError::internal("会话暂不可用"))?
        .get(id)
        .cloned()
        .ok_or_else(|| ApiError::bad("文件会话已失效，请重新打开并恢复草稿"))
}
/// Disposable projections have exactly the same Session coordinate. This does
/// not delete its persisted author Draft or original-operation recovery facts.
fn forget_session_projection(state: &AppState, session_id: &str) -> Result<(), ApiError> {
    state
        .locations
        .lock()
        .map_err(|_| ApiError::internal("Session locations are unavailable"))?
        .remove(session_id);
    state
        .renders
        .lock()
        .map_err(|_| ApiError::internal("Session render projections are unavailable"))?
        .remove(session_id);
    state
        .editor_versions
        .lock()
        .map_err(|_| ApiError::internal("Session editor projections are unavailable"))?
        .remove(session_id);
    Ok(())
}

async fn boundary(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if host != format!("127.0.0.1:{}", state.port) && host != format!("localhost:{}", state.port) {
        return ApiError::new(StatusCode::FORBIDDEN, "host", "不允许的访问地址").into_response();
    }
    if request.uri().path().starts_with("/api/") {
        let token = request
            .headers()
            .get("x-weftext-token")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let origin = request
            .headers()
            .get(header::ORIGIN)
            .and_then(|h| h.to_str().ok());
        if token != state.token.as_ref() || origin.is_some_and(|o| o != format!("http://{host}")) {
            return ApiError::new(
                StatusCode::FORBIDDEN,
                "session_token",
                "此本机会话无效，请使用启动窗口中的地址重新打开",
            )
            .into_response();
        }
    }
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "cross-origin-opener-policy",
        HeaderValue::from_static("same-origin"),
    );
    headers.insert("content-security-policy", HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' blob:; font-src 'self' data:; connect-src 'self'; worker-src 'self' blob:; frame-src blob:; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"));
    response
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathRequest {
    path: String,
    root: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateLibraryRequest {
    path: String,
    operation_id: Option<String>,
    #[serde(default)]
    options: weftext_core::LibraryCreateOptions,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateNote {
    root: String,
    parent: String,
    name: String,
    operation_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftRequest {
    session_id: String,
    source: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenderRequest {
    session_id: String,
    source: String,
    serial: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProposalPreviewRequest {
    session_id: String,
    source: String,
    serial: u64,
    proposal_source: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditRequest {
    session_id: String,
    source: String,
    serial: u64,
    node_key: String,
    text: String,
    kind: Option<String>,
    level: Option<u8>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecoveryRequest {
    operation_id: String,
    action: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OperationRequest {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftResolution {
    session_id: String,
    action: String,
    source: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LinkRequest {
    session_id: String,
    target: String,
}

async fn bootstrap(State(state): State<AppState>) -> ApiResult {
    // The launch document is a locator, never a reusable source/Draft snapshot.
    // A browser reload must observe Core's current original identity and Draft.
    let launch = state.initial.clone();
    let (recovery, reopened) = core_call(&state, move |core| {
        let recovery = core.recoveries()?;
        let reopened = launch.as_ref().map(|original| {
            let root = original["root"].as_str().ok_or_else(|| {
                CoreError::new(
                    "initialUnavailable",
                    "The original launch root is unavailable",
                )
            })?;
            if let (Some(workspace), Some(node)) = (
                original["workspaceId"].as_str(),
                original["nodeId"].as_str(),
            ) {
                data_api::open_live_entity(
                    core,
                    root,
                    &format!("weftext-node://{workspace}/{node}"),
                )
            } else {
                let path = original["path"].as_str().ok_or_else(|| {
                    CoreError::new(
                        "initialUnavailable",
                        "The original launch path is unavailable",
                    )
                })?;
                core.open(path, Some(PathBuf::from(root)))
            }
        });
        Ok((recovery, reopened))
    })
    .await?;
    let (initial, initial_error) = match reopened {
        Some(Ok(document)) => {
            let document = value(document)?;
            remember(&state, &document)?;
            (Some(document), None)
        }
        Some(Err(error)) => (
            None,
            Some(json!({"code":error.code,
            "message":"启动文档当前不可用，原草稿保留；请从资料库重新打开。"})),
        ),
        None => (None, None),
    };
    Ok(Json(
        json!({"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,
        "initialDocument":initial,"initialDocumentError":initial_error,"recoveries":recovery,
        "capabilities":{"ordinaryFiles":true,"folderNodes":true,"sourceEditing":true,
        "boundedRichEditing":true,"nativeTypst":true,"serverSync":false,"officeExport":true,
        "localNoteTimeline":true,"embeddedQueryViews":true}}),
    ))
}
async fn open(State(state): State<AppState>, Json(req): Json<PathRequest>) -> ApiResult {
    let doc = value(core_call(&state, move |c| c.open(req.path, req.root)).await?)?;
    remember(&state, &doc)?;
    Ok(Json(doc))
}
async fn inventory(State(state): State<AppState>, Json(req): Json<PathRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| c.inventory(req.path)).await?,
    )?))
}
async fn library_creation_defaults() -> ApiResult {
    let source = weftext_engine::DEFAULT_LIBRARY_STYLE;
    let inspection = weftext_engine::inspect_library_style(source);
    Ok(Json(
        json!({"source":source,"sourceHash":inspection.source_hash,"controls":inspection.controls,"rootPresets":weftext_core::root_query_presets()}),
    ))
}
async fn create_library(
    State(state): State<AppState>,
    Json(req): Json<CreateLibraryRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| {
            let operation_id = req
                .operation_id
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            c.create_library_with_options_and_id(req.path, req.options, &operation_id)
        })
        .await?,
    )?))
}
async fn cancel_library_creation(
    State(state): State<AppState>,
    Json(req): Json<CreateLibraryRequest>,
) -> ApiResult {
    let operation_id = req
        .operation_id
        .ok_or_else(|| ApiError::bad("请提供原创建操作编号；未生成替代操作"))?;
    Ok(Json(value(
        core_call(&state, move |c| {
            c.cancel_library_creation_with_options_and_id(req.path, req.options, &operation_id)
        })
        .await?,
    )?))
}
async fn create_note(State(state): State<AppState>, Json(req): Json<CreateNote>) -> ApiResult {
    let (preview, receipt) = core_call(&state, move |c| {
        let id = req
            .operation_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let receipt = c.create_note_with_id(req.root, req.parent, &req.name, &id)?;
        Ok((c.operation_details(&id)?, receipt))
    })
    .await?;
    knowledge_api::operation_output(&state, preview, receipt, None)
        .await
        .map(Json)
}
async fn save(State(state): State<AppState>, Json(req): Json<SaveRequest>) -> ApiResult {
    Ok(Json(value(core_call(&state, move |c| c.save(req)).await?)?))
}
async fn save_as(State(state): State<AppState>, Json(req): Json<SaveAsRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| c.save_as(req)).await?,
    )?))
}
async fn draft(State(state): State<AppState>, Json(req): Json<DraftRequest>) -> ApiResult {
    core_call(&state, move |c| {
        c.persist_draft(&req.session_id, &req.source)
    })
    .await?;
    Ok(Json(json!({"ok":true})))
}
async fn resume_draft(
    State(state): State<AppState>,
    Json(req): Json<weftext_core::ResumeDraftRequest>,
) -> ApiResult {
    core_call(&state, move |core| core.resume_recovered_draft(req)).await?;
    Ok(Json(json!({"ok":true})))
}
async fn recovery(State(state): State<AppState>) -> ApiResult {
    Ok(Json(value(core_call(&state, |c| c.recoveries()).await?)?))
}
async fn inspect_recovery(
    State(state): State<AppState>,
    Json(req): Json<OperationRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| c.inspect_recovery(&req.operation_id)).await?,
    )?))
}
async fn resolve_recovery(
    State(state): State<AppState>,
    Json(req): Json<RecoveryRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| {
            c.resolve_recovery(&req.operation_id, &req.action)
        })
        .await?,
    )?))
}

async fn resolve_draft(
    State(state): State<AppState>,
    Json(req): Json<DraftResolution>,
) -> ApiResult {
    core_call(&state, move |c| {
        c.resolve_draft(&req.session_id, &req.action, req.source.as_deref())
    })
    .await?;
    Ok(Json(json!({"ok":true})))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkerRequest {
    source: String,
    path: PathBuf,
    root: PathBuf,
    #[serde(default)]
    paged: bool,
    #[serde(default)]
    style: Option<weftext_engine::StyleModule>,
    frozen_files: std::collections::BTreeMap<String, String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NotesWorkerRequest {
    input: WorkerRequest,
    request: weftext_engine::EditRequest,
}
async fn render(State(state): State<AppState>, Json(req): Json<RenderRequest>) -> ApiResult {
    compile_output(state, req, false, None).await
}
async fn paged(State(state): State<AppState>, Json(req): Json<RenderRequest>) -> ApiResult {
    compile_output(state, req, true, None).await
}
async fn proposal_preview(
    State(state): State<AppState>,
    Json(req): Json<ProposalPreviewRequest>,
) -> ApiResult {
    compile_output(
        state,
        RenderRequest {
            session_id: req.session_id,
            source: req.source,
            serial: req.serial,
        },
        false,
        Some(req.proposal_source),
    )
    .await
}
async fn compile_output(
    state: AppState,
    req: RenderRequest,
    paged: bool,
    proposal_source: Option<String>,
) -> ApiResult {
    let preview_only = proposal_source.is_some();
    let compiled_source = proposal_source.unwrap_or_else(|| req.source.clone());
    if req.source.len() > 2 * 1024 * 1024 || compiled_source.len() > 2 * 1024 * 1024 {
        return Err(ApiError::bad(
            "预览上限为 2 MiB；完整输入仍保留在当前窗口，保存也受源码上限约束",
        ));
    }
    let loc = location(&state, &req.session_id)?;
    writing_api::observe_snapshot(&state, &req.session_id, &req.source, req.serial, None)?;
    let environment = writing_api::style_environment(&state, &req.session_id).await?;
    let citation_inputs = if weftext_engine::inspect_citation_declaration(&compiled_source)
        .map_err(|e| ApiError::bad(e.to_string()))?
        .is_some()
    {
        let root = loc.root.clone();
        let path = loc.path.clone();
        let source = compiled_source.clone();
        let captured = core_call(&state, move |core| {
            let mut captured = core.admitted_compile_snapshot(&root, "live")?;
            let citations = citation_api::freeze_citations(&mut captured, &source, &path)?;
            Ok((captured, citations))
        })
        .await?;
        Some(captured)
    } else {
        None
    };
    let preview = if citation_inputs.is_none() {
        let session = req.session_id.clone();
        let source = compiled_source.clone();
        Some(
            core_call(&state, move |core| {
                core.preview_compile_snapshot(&session, &source)
            })
            .await?,
        )
    } else {
        None
    };
    let _permit = state
        .render_gate
        .acquire()
        .await
        .map_err(|_| ApiError::internal("编译服务不可用"))?;
    let input = WorkerRequest {
        source: compiled_source.clone(),
        path: loc.path,
        root: loc.root,
        paged,
        style: citation_inputs
            .as_ref()
            .and_then(|c| c.0.style.clone())
            .or_else(|| preview.as_ref().and_then(|preview| preview.style.clone())),
        frozen_files: {
            use base64::Engine as _;
            let files = if let Some(captured) = &citation_inputs {
                &captured.0.files
            } else {
                &preview
                    .as_ref()
                    .expect("the non-citation preview was captured")
                    .files
            };
            files
                .iter()
                .map(|(p, b)| {
                    (
                        p.clone(),
                        base64::engine::general_purpose::STANDARD.encode(b),
                    )
                })
                .collect()
        },
    };
    let mut output = render_worker(input).await?;
    writing_api::check_snapshot(&state, &req.session_id, &req.source, req.serial, None)?;
    writing_api::check_style_environment(&state, &req.session_id, &environment).await?;
    output["serial"] = json!(req.serial);
    output["source"] = json!(compiled_source);
    output["environmentKey"] = json!(environment.key);
    output["environment"] = json!(environment.key);
    output["sessionId"] = json!(req.session_id);
    if let Some((_, citations)) = citation_inputs {
        output["citations"] = value(citations)?;
    }
    // A proposed source never becomes the current editor version or its cached
    // projection. Confirmation separately rechecks and patches the original Draft.
    if paged || preview_only {
        return Ok(Json(output));
    }
    let mut cache = state
        .renders
        .lock()
        .map_err(|_| ApiError::internal("编译会话不可用"))?;
    let replace = cache.get(&req.session_id).is_none_or(|old| {
        req.serial > old.serial || req.serial == old.serial && req.source == old.source
    });
    if replace {
        cache.insert(
            req.session_id,
            RenderCache {
                source: req.source,
                serial: req.serial,
                output: output.clone(),
            },
        );
    }
    Ok(Json(output))
}

async fn render_frozen_worker(
    state: &AppState,
    request: weftext_engine::FrozenRenderRequest,
) -> Result<Value, ApiError> {
    render_frozen_target_worker(state, request, false).await
}
async fn render_frozen_target_worker(
    state: &AppState,
    request: weftext_engine::FrozenRenderRequest,
    paged: bool,
) -> Result<Value, ApiError> {
    use base64::Engine as _;
    let total: usize = request.files.values().map(Vec::len).sum();
    if total > 64 * 1024 * 1024 || request.source.len() > 2 * 1024 * 1024 {
        return Err(ApiError::bad(
            "此查询的冻结编译输入超出本机预算，结果不可当作完整。",
        ));
    }
    let _permit = state
        .render_gate
        .acquire()
        .await
        .map_err(|_| ApiError::internal("编译服务不可用"))?;
    render_worker(WorkerRequest {
        source: request.source,
        path: request.path,
        root: request.root,
        paged,
        style: request.style,
        frozen_files: request
            .files
            .into_iter()
            .map(|(path, bytes)| {
                (
                    path,
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                )
            })
            .collect(),
    })
    .await
}

async fn render_worker(input: WorkerRequest) -> Result<Value, ApiError> {
    bounded_compiler_worker(value(input)?, false).await
}

#[derive(Deserialize)]
struct NotesWorkerOutput {
    edit: Value,
    dependencies: Vec<weftext_engine::Dependency>,
}

async fn notes_edit_worker(
    input: weftext_engine::FrozenRenderRequest,
    request: weftext_engine::EditRequest,
) -> Result<NotesWorkerOutput, ApiError> {
    use base64::Engine as _;
    if input.source.len() > weftext_core::MAX_SOURCE_BYTES
        || input.files.values().map(Vec::len).sum::<usize>() > 64 * 1024 * 1024
    {
        return Err(ApiError::bad("尾注编辑的冻结输入超过预算，原稿保留"));
    }
    notes_edit_from_worker(
        WorkerRequest {
            source: input.source,
            path: input.path,
            root: input.root,
            paged: false,
            style: input.style,
            frozen_files: input
                .files
                .into_iter()
                .map(|(path, bytes)| {
                    (
                        path,
                        base64::engine::general_purpose::STANDARD.encode(bytes),
                    )
                })
                .collect(),
        },
        request,
    )
    .await
}

async fn notes_edit_from_worker(
    input: WorkerRequest,
    request: weftext_engine::EditRequest,
) -> Result<NotesWorkerOutput, ApiError> {
    let output =
        bounded_compiler_worker(value(NotesWorkerRequest { input, request })?, true).await?;
    serde_json::from_value(output).map_err(|_| ApiError::internal("尾注编译依赖回执无效，原稿保留"))
}

async fn bounded_compiler_worker(input: Value, notes_edit: bool) -> Result<Value, ApiError> {
    // Own the whole process future independently of an HTTP caller. Dropping
    // that caller cannot discard a live child or its final kill/wait receipt.
    crate::work::spawn(owned_compiler_worker(input, notes_edit))
        .await
        .map_err(|_| {
            let mut failure =
                worker_failure::WorkerFailure::new(worker_failure::Kind::SupervisionLost);
            failure.cleanup_confirmed = false;
            ApiError::worker_failure(failure)
        })?
}

async fn owned_compiler_worker(input: Value, notes_edit: bool) -> Result<Value, ApiError> {
    use worker_failure::{Kind, WorkerFailure};
    let bytes = serde_json::to_vec(&input)
        .map_err(|_| ApiError::worker_failure(WorkerFailure::new(Kind::InputEncoding)))?;
    if bytes.len() > 96 * 1024 * 1024 {
        return Err(ApiError::worker_failure(WorkerFailure::new(
            Kind::InputBudget,
        )));
    }
    drop(input);
    let exe = std::env::current_exe().map_err(|error| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        ApiError::worker_failure(failure)
    })?;
    #[cfg(test)]
    let exe = std::env::var_os("WEFTEXT_TEST_EXECUTABLE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            exe.parent()
                .and_then(std::path::Path::parent)
                .unwrap_or_else(|| std::path::Path::new("."))
                .join(if cfg!(windows) {
                    "weftext.exe"
                } else {
                    "weftext"
                })
        });
    let mut command = Command::new(exe);
    command
        .arg(if notes_edit {
            "--notes-edit-worker"
        } else {
            "--render-worker"
        })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().await.map_err(|error| {
        let mut failure = WorkerFailure::new(Kind::Setup);
        failure.io_error = Some(error);
        ApiError::worker_failure(failure)
    })?;
    let mut facts = WorkerFailure::new(Kind::Setup);
    facts.cleanup_confirmed = false;
    // Stream buffers live outside cancellation/try_join futures. A budget,
    // timeout or another stream's failure preserves the exact bytes observed,
    // their EOF/truncation flags and the original non-lossy exit facts.
    let completed: Result<Value, (Kind, Option<std::io::Error>, Option<ApiError>)> = async {
        let mut stdin = child.stdin.take().ok_or((Kind::Setup, None, None))?;
        let stdout = child.stdout.take().ok_or((Kind::Setup, None, None))?;
        let stderr = child.stderr.take().ok_or((Kind::Setup, None, None))?;
        let output = tokio::time::timeout(Duration::from_secs(20), async {
            let input = async {
                stdin
                    .write_all(&bytes)
                    .await
                    .map_err(|error| (Kind::InputIo, Some(error)))?;
                drop(stdin);
                facts.stdin_complete = true;
                Ok::<_, (Kind, Option<std::io::Error>)>(())
            };
            let result = capture_worker_stream(
                stdout,
                &mut facts.stdout,
                24 * 1024 * 1024,
                Kind::StdoutIo,
                Kind::StdoutBudget,
            );
            let diagnostics = capture_worker_stream(
                stderr,
                &mut facts.stderr,
                256 * 1024,
                Kind::StderrIo,
                Kind::StderrBudget,
            );
            let status = async {
                let status = child
                    .wait()
                    .await
                    .map_err(|error| (Kind::Wait, Some(error)))?;
                facts.exit = Some(status.exit());
                facts.cleanup_confirmed = true;
                Ok::<_, (Kind, Option<std::io::Error>)>(status)
            };
            let (_, (), (), status) = tokio::try_join!(input, result, diagnostics, status)?;
            Ok::<_, (Kind, Option<std::io::Error>)>(status)
        })
        .await
        .map_err(|_| (Kind::Deadline, None, None))?
        .map_err(|(kind, error)| (kind, error, None))?;
        if !output.success() {
            return Err((Kind::NonzeroExit, None, None));
        }
        decode_compiler_worker_output(&facts.stdout.bytes, notes_edit)
            .map_err(|error| (Kind::OutputRejected, None, Some(error)))
    }
    .await;
    let (kind, error, protocol_error) = match completed {
        Ok(output) => return Ok(output),
        Err(error) => error,
    };
    facts.kind = kind;
    facts.io_error = error;
    if !facts.cleanup_confirmed {
        match child.kill().await {
            Ok(()) => match child.wait().await {
                Ok(status) => {
                    facts.exit = Some(status.exit());
                    facts.cleanup_confirmed = true;
                }
                Err(error) => facts.cleanup_error = Some(error),
            },
            Err(error) => {
                facts.cleanup_error = Some(error);
                // A failed termination call is not a completion receipt. Only
                // the same held process/job's independent actual exit can be.
                if let Ok(Some(status)) = child.try_wait() {
                    facts.exit = Some(status.exit());
                    facts.cleanup_confirmed = true;
                }
            }
        }
    }
    let facts = Arc::new(facts);
    let mut error = match protocol_error {
        Some(error) if facts.cleanup_confirmed => error,
        _ => ApiError::worker_failure_summary(&facts),
    };
    error.private_worker_failure = Some(facts);
    Err(error)
}

async fn capture_worker_stream(
    stream: impl tokio::io::AsyncRead + Unpin,
    capture: &mut worker_failure::StreamCapture,
    limit: usize,
    io_kind: worker_failure::Kind,
    budget_kind: worker_failure::Kind,
) -> Result<(), (worker_failure::Kind, Option<std::io::Error>)> {
    stream
        .take(limit as u64 + 1)
        .read_to_end(&mut capture.bytes)
        .await
        .map_err(|error| (io_kind, Some(error)))?;
    if capture.bytes.len() > limit {
        capture.truncated = true;
        return Err((budget_kind, None));
    }
    capture.complete = true;
    Ok(())
}

#[cfg(test)]
mod worker_capture_tests {
    use super::*;
    use worker_failure::{Kind, StreamCapture, WorkerFailure};

    #[tokio::test]
    async fn original_stream_bytes_survive_invalid_utf8_and_real_eof() {
        let raw = vec![0xff, b'a', 0, b'\n'];
        let mut captured = StreamCapture::default();
        capture_worker_stream(
            std::io::Cursor::new(raw.clone()),
            &mut captured,
            16,
            Kind::StdoutIo,
            Kind::StdoutBudget,
        )
        .await
        .unwrap();
        assert_eq!(captured.bytes, raw);
        assert!(captured.complete);
        assert!(!captured.truncated);
        assert!(std::str::from_utf8(&captured.bytes).is_err());
    }
    #[tokio::test]
    async fn cancelling_a_pending_read_preserves_its_observed_prefix_without_eof_claim() {
        let (mut writer, reader) = tokio::io::duplex(32);
        writer.write_all(b"partial-original").await.unwrap();
        let mut captured = StreamCapture::default();
        let result = tokio::time::timeout(
            Duration::from_millis(20),
            capture_worker_stream(
                reader,
                &mut captured,
                32,
                Kind::StderrIo,
                Kind::StderrBudget,
            ),
        )
        .await;
        assert!(result.is_err());
        assert_eq!(captured.bytes, b"partial-original");
        assert!(!captured.complete);
        assert!(!captured.truncated);
        drop(writer);
    }
    #[tokio::test]
    async fn budget_capture_keeps_the_exact_overflow_witness_and_closed_failure_kind() {
        let mut captured = StreamCapture::default();
        let (kind, error) = capture_worker_stream(
            std::io::Cursor::new(b"12345678"),
            &mut captured,
            4,
            Kind::StdoutIo,
            Kind::StdoutBudget,
        )
        .await
        .unwrap_err();
        assert_eq!(kind, Kind::StdoutBudget);
        assert!(error.is_none());
        assert_eq!(captured.bytes, b"12345");
        assert!(captured.truncated);
        assert!(!captured.complete);
    }
    #[tokio::test]
    async fn private_product_streams_are_retained_once_and_never_public_or_debug_diagnostics() {
        fn traits<T: std::error::Error + Send + Sync>() {}
        traits::<WorkerFailure>();
        let raw = b"server-private-original\0\xff".to_vec();
        let mut failure = WorkerFailure::new(Kind::NonzeroExit);
        failure.stderr = StreamCapture {
            bytes: raw.clone(),
            complete: true,
            truncated: false,
        };
        failure.io_error = Some(std::io::Error::other("private server path C:\\secret"));
        let error = ApiError::worker_failure(failure);
        let retained = error.private_worker_failure.as_ref().unwrap().clone();
        assert_eq!(retained.stderr.bytes, raw);
        let private = std::error::Error::source(&error)
            .unwrap()
            .downcast_ref::<WorkerFailure>()
            .unwrap();
        assert!(std::ptr::eq(private, retained.as_ref()));
        let debug = format!("{error:?}");
        assert!(!debug.contains("server-private-original"));
        assert!(!debug.contains("C:\\secret"));
        let response = error.into_response();
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let public = std::str::from_utf8(&bytes).unwrap();
        assert!(!public.contains("server-private-original"));
        assert!(!public.contains("C:\\secret"));
        assert!(public.contains("compiler_failed"));
        assert_eq!(retained.stderr.bytes, raw);
    }
}

const DISPLAY_TREE_BUDGET_ENGINE_ERROR: &str = "显示树超过安全预算；请使用源码入口";
const DISPLAY_TREE_BUDGET_MESSAGE: &str =
    "预览显示树超过 100000 个节点或 256 层深度的安全预算；请使用源码编辑，输入已保留。";
const RENDER_WORKER_FAILURE_KEY: &str = "weftextRenderWorkerFailure";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderWorkerFailureEnvelope {
    #[serde(rename = "weftextRenderWorkerFailure")]
    failure: RenderWorkerFailure,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderWorkerFailure {
    version: u8,
    code: RenderWorkerFailureCode,
}

#[derive(Serialize, Deserialize)]
enum RenderWorkerFailureCode {
    #[serde(rename = "display_tree_budget")]
    DisplayTreeBudget,
}

fn write_render_worker_failure(
    error: Box<dyn std::error::Error>,
    writer: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    // Only this engine-owned static failure gets a public code. Neither an
    // arbitrary error's text nor stderr is a trusted failure protocol.
    if !error
        .downcast_ref::<weftext_engine::EngineError>()
        .is_some_and(|error| error.0 == DISPLAY_TREE_BUDGET_ENGINE_ERROR)
    {
        return Err(error);
    }
    serde_json::to_writer(
        &mut *writer,
        &RenderWorkerFailureEnvelope {
            failure: RenderWorkerFailure {
                version: 1,
                code: RenderWorkerFailureCode::DisplayTreeBudget,
            },
        },
    )?;
    writer.flush()?;
    Ok(())
}

fn decode_compiler_worker_output(bytes: &[u8], notes_edit: bool) -> Result<Value, ApiError> {
    let output: Value =
        serde_json::from_slice(bytes).map_err(|_| ApiError::internal("编译输出无效"))?;
    if notes_edit {
        return Ok(output);
    }
    let object = output
        .as_object()
        .ok_or_else(|| ApiError::internal("编译输出无效"))?;
    if !object.contains_key(RENDER_WORKER_FAILURE_KEY) {
        return Ok(output);
    }
    // Struct decoding rejects duplicate/unknown fields; the object checks also
    // exclude JSON sequence representations of a named struct. A nested author
    // field in a normal RenderOutput never enters this private protocol.
    let exact_objects = object.len() == 1
        && output[RENDER_WORKER_FAILURE_KEY]
            .as_object()
            .is_some_and(|object| object.len() == 2);
    if exact_objects
        && matches!(
            serde_json::from_slice::<RenderWorkerFailureEnvelope>(bytes),
            Ok(RenderWorkerFailureEnvelope {
                failure: RenderWorkerFailure {
                    version: 1,
                    code: RenderWorkerFailureCode::DisplayTreeBudget,
                },
            })
        )
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "display_tree_budget",
            DISPLAY_TREE_BUDGET_MESSAGE,
        ));
    }
    Err(ApiError::internal("编译输出无效"))
}

#[cfg(test)]
mod render_worker_codec_tests {
    use super::*;

    const FAILURE: &[u8] =
        br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}}"#;

    #[test]
    fn exact_typed_engine_budget_emits_only_the_private_code() {
        let mut output = Vec::new();
        write_render_worker_failure(
            Box::new(weftext_engine::EngineError(
                DISPLAY_TREE_BUDGET_ENGINE_ERROR.into(),
            )),
            &mut output,
        )
        .unwrap();
        assert_eq!(output, FAILURE);
        let error = decode_compiler_worker_output(&output, false).unwrap_err();
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, "display_tree_budget");
        assert_eq!(error.message, DISPLAY_TREE_BUDGET_MESSAGE);
        assert!(error.details.is_none());
    }

    #[test]
    fn other_types_and_non_exact_engine_errors_write_nothing() {
        let errors: Vec<Box<dyn std::error::Error>> = vec![
            Box::new(std::io::Error::other(DISPLAY_TREE_BUDGET_ENGINE_ERROR)),
            Box::new(weftext_engine::EngineError(format!(
                "{DISPLAY_TREE_BUDGET_ENGINE_ERROR}\nprivate-source-or-path"
            ))),
            Box::new(weftext_engine::EngineError(
                "unrelated private-source-or-path".into(),
            )),
        ];
        for original in errors {
            let original_message = original.to_string();
            let mut output = Vec::new();
            let returned = write_render_worker_failure(original, &mut output).unwrap_err();
            assert_eq!(returned.to_string(), original_message);
            assert!(output.is_empty());
        }
    }

    #[test]
    fn invalid_or_unknown_failure_envelopes_use_only_fixed_invalid_output() {
        for bytes in [
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget"}}"#.as_slice(),
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"private-unknown-code"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":"1","code":"display_tree_budget"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget","message":"private-source-or-path"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"},"extra":"private-source-or-path"}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"version":1,"code":"display_tree_budget"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget","code":"display_tree_budget"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"},"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}}"#,
            br#"{"weftextRenderWorkerFailure":[1,"display_tree_budget"]}"#,
            br#"[{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}}]"#,
            br#"{"weftextRenderWorkerFailure":null}"#,
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}} trailing"#,
            br#"{"weftextRenderWorkerFailure":"#,
        ] {
            let error = decode_compiler_worker_output(bytes, false).unwrap_err();
            assert_eq!(error.code, "internal");
            assert_eq!(error.message, "编译输出无效");
            assert!(error.details.is_none());
        }
    }

    #[test]
    fn ordinary_outputs_and_nested_author_fields_are_unchanged() {
        for ordinary in [
            json!({"ok":true,"tree":null,"sourceInfo":{},"products":null}),
            json!({"ok":false,"diagnostics":[{"message":DISPLAY_TREE_BUDGET_ENGINE_ERROR}],"tree":{"attrs":{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}}}}),
        ] {
            assert_eq!(
                decode_compiler_worker_output(&serde_json::to_vec(&ordinary).unwrap(), false)
                    .unwrap(),
                ordinary
            );
        }
    }

    #[test]
    fn notes_output_keeps_its_existing_decoder() {
        let output = serde_json::from_slice::<Value>(FAILURE).unwrap();
        assert_eq!(
            decode_compiler_worker_output(FAILURE, true).unwrap(),
            output
        );
        let ordinary = json!({"edit":{"source":"retained","weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}},"dependencies":[]});
        assert_eq!(
            decode_compiler_worker_output(&serde_json::to_vec(&ordinary).unwrap(), true).unwrap(),
            ordinary
        );
    }
}

fn find_node<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
    if node["key"].as_str() == Some(key) {
        return Some(node);
    }
    node["children"]
        .as_array()?
        .iter()
        .find_map(|n| find_node(n, key))
}
async fn edit_literal(State(state): State<AppState>, Json(req): Json<EditRequest>) -> ApiResult {
    location(&state, &req.session_id)?;
    let cache = state
        .renders
        .lock()
        .map_err(|_| ApiError::internal("编译会话不可用"))?;
    let proof = cache
        .get(&req.session_id)
        .filter(|p| p.serial == req.serial && p.source == req.source)
        .ok_or_else(|| ApiError::bad("编辑来源已过期，请等待当前预览"))?;
    let (from, to) = if req.node_key == "@body" {
        let insertion = &proof.output["sourceInfo"]["bodyInsertion"];
        if insertion["writable"] != true {
            return Err(ApiError::bad("当前正文没有安全插入点"));
        }
        let pos = insertion["from"]
            .as_u64()
            .filter(|p| Some(*p) == insertion["to"].as_u64())
            .ok_or_else(|| ApiError::bad("当前正文没有安全插入点"))? as usize;
        (pos, pos)
    } else {
        let node = find_node(&proof.output["tree"], &req.node_key)
            .filter(|n| n["writable"] == true)
            .ok_or_else(|| ApiError::bad("此区域请使用源码编辑"))?;
        let origin = &node["origin"];
        if origin["isMain"] != true {
            return Err(ApiError::bad("不能修改引用或生成内容"));
        }
        let from = origin["from"]
            .as_u64()
            .ok_or_else(|| ApiError::bad("缺少来源"))? as usize;
        let to = origin["to"]
            .as_u64()
            .ok_or_else(|| ApiError::bad("缺少来源"))? as usize;
        if req.source.get(from..to) != origin["raw"].as_str() {
            return Err(ApiError::bad("原文字节已变化"));
        }
        (from, to)
    };
    let literal =
        weftext_engine::encode_literal(&req.text).map_err(|e| ApiError::bad(e.to_string()))?;
    if req.node_key != "@body" && req.text.contains(['\r', '\n']) {
        return Err(ApiError::bad(
            "此文字区域只支持行内编辑；换段请使用新增正文或源码",
        ));
    }
    let encoded = if req.node_key == "@body" {
        let body = match req.kind.as_deref().unwrap_or("paragraph") {
            "paragraph" => literal,
            "heading" => {
                let level = req.level.unwrap_or(1);
                if !(1..=9).contains(&level) {
                    return Err(ApiError::bad("标题级别须为 1–9"));
                }
                format!("{} {}", "=".repeat(level as usize), literal)
            }
            "bullet" => format!("- {literal}"),
            "strong" => format!("*{literal}*"),
            "emph" => format!("_{literal}_"),
            _ => return Err(ApiError::bad("尚未支持的正文插入形式")),
        };
        format!("{}{body}\n", if from > 0 { "\n\n" } else { "" })
    } else {
        literal
    };
    if !req.source.is_char_boundary(from) || !req.source.is_char_boundary(to) || from > to {
        return Err(ApiError::bad("无效字符边界"));
    }
    let start = req.source[..from].encode_utf16().count();
    let end = req.source[..to].encode_utf16().count();
    let source = format!("{}{}{}", &req.source[..from], encoded, &req.source[to..]);
    Ok(Json(
        json!({"source":source,"edit":{"from":start,"to":end,"text":encoded}}),
    ))
}

async fn link_resolve(State(state): State<AppState>, Json(req): Json<LinkRequest>) -> ApiResult {
    let loc = location(&state, &req.session_id)?;
    let target = req.target.trim();
    let parsed = url::Url::parse(target).ok();
    if let Some(url) = &parsed {
        match url.scheme() {
            "https" | "http" | "mailto" => return Ok(Json(json!({"externalUrl":url.as_str()}))),
            "weftext-node" | "weftext-workspace" | "weftext-resource" => {
                return knowledge_api::resolve_address(&state, &req.session_id, url.as_str()).await;
            }
            "file" => {}
            _ => return Err(ApiError::bad("此链接形式尚未支持，原文保留")),
        }
    }
    // Managed targets are opened again by stable identity. Only an explicitly
    // ordinary file may use the ordinary path route; partial/missing identity
    // evidence must not silently downgrade an application address.
    match knowledge_api::resolve_address(&state, &req.session_id, target).await {
        Ok(resolved) => return Ok(resolved),
        Err(error) if error.code == "ordinaryFile" => {}
        Err(error) => return Err(error),
    }
    if let Some(url) = parsed {
        let path = url
            .to_file_path()
            .map_err(|_| ApiError::bad("无效文件链接"))?;
        return link_path(path, loc.root, url.fragment().map(str::to_owned));
    }
    let (path, fragment) = target
        .split_once('#')
        .map(|(a, b)| (a, Some(b.to_owned())))
        .unwrap_or((target, None));
    if path.is_empty() {
        return Ok(Json(
            json!({"path":loc.path,"root":loc.root,"fragment":fragment}),
        ));
    }
    // Standard file-URL resolution decodes percent escapes exactly once.
    let base = url::Url::from_file_path(&loc.path).map_err(|_| ApiError::bad("来源路径无效"))?;
    let resolved = base.join(path).map_err(|_| ApiError::bad("无效文件路径"))?;
    let path = resolved
        .to_file_path()
        .map_err(|_| ApiError::bad("无效文件路径"))?;
    link_path(path, loc.root, fragment)
}

fn link_path(path: PathBuf, root: PathBuf, fragment: Option<String>) -> ApiResult {
    let path = path
        .canonicalize()
        .map_err(|_| ApiError::bad("目标文件不存在"))?;
    let root = root
        .canonicalize()
        .map_err(|_| ApiError::bad("依赖根不可用"))?;
    if !path.starts_with(&root) {
        return Err(ApiError::bad("链接超出当前项目根，请明确打开该文件"));
    }
    if path
        .extension()
        .is_none_or(|s| !s.eq_ignore_ascii_case("typ"))
    {
        return Err(ApiError::bad(
            "普通文件路径只在编辑器中打开 .typ；其他文件请先登记为笔记资源",
        ));
    }
    let path = ordinary_drive_path(path)?;
    let root = ordinary_drive_path(root)?;
    Ok(Json(json!({"path":path,"root":root,"fragment":fragment})))
}

// Windows canonicalize adds a verbatim-drive prefix. Containment above uses
// canonical paths; returning an ordinary drive path keeps Core's narrow local
// file policy intact. UNC/device namespaces remain unsupported.
fn ordinary_drive_path(path: PathBuf) -> Result<PathBuf, ApiError> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        let mut components = path.components();
        if let Some(Component::Prefix(prefix)) = components.next() {
            match prefix.kind() {
                Prefix::VerbatimDisk(drive) => {
                    let mut normalized = PathBuf::from(format!("{}:\\", char::from(drive)));
                    for component in components {
                        match component {
                            Component::RootDir => {}
                            Component::Normal(name) => normalized.push(name),
                            _ => return Err(ApiError::bad("不支持的文件路径")),
                        }
                    }
                    return Ok(normalized);
                }
                Prefix::Disk(_) => {}
                _ => return Err(ApiError::bad("本地档位只支持本机盘符路径")),
            }
        }
    }
    Ok(path)
}

async fn assets(request: Request) -> Response {
    let path = request.uri().path().trim_start_matches('/');
    if let Some(font) = path.strip_prefix("fonts/") {
        return writing_api::font_asset(font)
            .unwrap_or_else(|| StatusCode::NOT_FOUND.into_response());
    }
    let path = if path.is_empty() { "index.html" } else { path };
    let Some(file) = ASSETS.get_file(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        [(header::CONTENT_TYPE, mime.as_ref())],
        Body::from(file.contents().to_vec()),
    )
        .into_response()
}

fn notes_worker_main() -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine as _;
    use std::io::{Read, Write};
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(96 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 96 * 1024 * 1024 {
        return Err("Frozen note edit exceeds input budget".into());
    }
    let NotesWorkerRequest { input, request } = serde_json::from_slice(&bytes)?;
    let mut files = std::collections::BTreeMap::new();
    let mut total = 0usize;
    for (path, bytes) in input.frozen_files {
        let decoded = base64::engine::general_purpose::STANDARD.decode(bytes)?;
        total = total
            .checked_add(decoded.len())
            .ok_or("Note edit size overflow")?;
        if total > 64 * 1024 * 1024 {
            return Err("Frozen note files exceed budget".into());
        }
        files.insert(path, decoded);
    }
    if input.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err("Note edit source exceeds budget".into());
    }
    let engine = weftext_engine::Engine::new_with_system_fonts(false)?;
    let output = engine.edit_notes_frozen_with_dependencies(
        &weftext_engine::FrozenRenderRequest {
            source: input.source,
            path: input.path,
            root: input.root,
            files,
            style: input.style,
        },
        &request,
    )?;
    let output =
        serde_json::to_vec(&json!({"edit": output.edit, "dependencies": output.dependencies}))?;
    if output.len() > 24 * 1024 * 1024 {
        return Err("Note edit result exceeds budget".into());
    }
    std::io::stdout().write_all(&output)?;
    Ok(())
}

fn worker_main() -> Result<(), Box<dyn std::error::Error>> {
    use base64::Engine as _;
    use std::io::{Read, Write};
    let mut input = String::new();
    std::io::stdin()
        .take(96 * 1024 * 1024 + 1)
        .read_to_string(&mut input)?;
    if input.len() > 96 * 1024 * 1024 {
        return Err("Compile input exceeds the local budget".into());
    }
    let input: WorkerRequest = serde_json::from_str(&input)?;
    if input.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err("Compile source exceeds the local budget".into());
    }
    let engine = weftext_engine::Engine::new_with_system_fonts(false)?;
    let output = {
        let mut decoded = std::collections::BTreeMap::new();
        let mut total = 0usize;
        for (path, bytes) in input.frozen_files {
            let bytes = base64::engine::general_purpose::STANDARD.decode(bytes)?;
            total = total
                .checked_add(bytes.len())
                .ok_or("Compile input size overflow")?;
            if total > 64 * 1024 * 1024 {
                return Err("Frozen files exceed the local budget".into());
            }
            decoded.insert(path, bytes);
        }
        let request = weftext_engine::FrozenRenderRequest {
            source: input.source,
            path: input.path,
            root: input.root,
            files: decoded,
            style: input.style,
        };
        if input.paged {
            serde_json::to_value(engine.render_frozen_paged(&request)?)?
        } else {
            serde_json::to_value(engine.render_frozen(&request)?)?
        }
    };
    serde_json::to_writer(std::io::stdout().lock(), &output)?;
    std::io::stdout().flush()?;
    Ok(())
}
