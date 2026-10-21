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
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use weftext_core::{Core, CoreError, SaveAsRequest, SaveRequest};

mod bibliography_import_api;
mod builtin_module_api;
mod citation_api;
mod data_api;
mod embedded_output;
mod embedded_query_api;
mod export_api;
mod export_conversions;
mod history_api;
mod host_environment;
mod hosted;
mod knowledge_api;
pub mod launch;
mod legacy_office;
pub mod local_service;
mod media_api;
mod navigation_api;
mod processing_api;
mod recognition_api;
mod remote_sync;
mod sync_api;
mod sync_wire;
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
}
impl ApiError {
    fn new(status: StatusCode, code: &str, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }
    fn bad(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_request", message)
    }
    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", message)
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
    tokio::task::spawn_blocking(move || {
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
    let doc = value(
        core_call(&state, move |c| match req.operation_id {
            Some(id) => c.create_note_with_id(req.root, req.parent, &req.name, &id),
            None => c.create_note(req.root, req.parent, &req.name),
        })
        .await?,
    )?;
    remember(&state, &doc)?;
    Ok(Json(doc))
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
struct WorkerRequest {
    source: String,
    path: PathBuf,
    root: PathBuf,
    #[serde(default)]
    paged: bool,
    #[serde(default)]
    style: Option<weftext_engine::StyleModule>,
    #[serde(default)]
    frozen_files: Option<std::collections::BTreeMap<String, String>>,
}
#[derive(Serialize, Deserialize)]
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
            .or_else(|| environment.style.clone()),
        frozen_files: citation_inputs.as_ref().map(|c| {
            use base64::Engine as _;
            c.0.files
                .iter()
                .map(|(p, b)| {
                    (
                        p.clone(),
                        base64::engine::general_purpose::STANDARD.encode(b),
                    )
                })
                .collect()
        }),
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
        frozen_files: Some(
            request
                .files
                .into_iter()
                .map(|(path, bytes)| {
                    (
                        path,
                        base64::engine::general_purpose::STANDARD.encode(bytes),
                    )
                })
                .collect(),
        ),
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
            frozen_files: Some(
                input
                    .files
                    .into_iter()
                    .map(|(path, bytes)| {
                        (
                            path,
                            base64::engine::general_purpose::STANDARD.encode(bytes),
                        )
                    })
                    .collect(),
            ),
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
    let exe = std::env::current_exe().map_err(|e| ApiError::internal(e.to_string()))?;
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command
        .spawn()
        .map_err(|e| ApiError::internal(e.to_string()))?;
    #[cfg(windows)]
    let _job = worker_job::guard(&child)?;
    let bytes = serde_json::to_vec(&input).map_err(|e| ApiError::internal(e.to_string()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ApiError::internal("无法连接编译器"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ApiError::internal("无法读取编译结果"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ApiError::internal("无法读取编译诊断"))?;
    // Bound the pipe while reading, rather than after buffering arbitrary output.
    // Dropping this future also drops the child and its kill-on-close Windows job.
    let output = tokio::time::timeout(Duration::from_secs(20), async {
        let input = async {
            stdin.write_all(&bytes).await?;
            drop(stdin);
            Ok::<(), std::io::Error>(())
        };
        let result = async {
            let mut data = Vec::new();
            stdout
                .take(24 * 1024 * 1024 + 1)
                .read_to_end(&mut data)
                .await?;
            if data.len() > 24 * 1024 * 1024 {
                return Err(std::io::Error::other("预览结果超出 24 MiB"));
            }
            Ok(data)
        };
        let diagnostics = async {
            let mut data = Vec::new();
            stderr.take(256 * 1024 + 1).read_to_end(&mut data).await?;
            if data.len() > 256 * 1024 {
                return Err(std::io::Error::other("编译诊断超出 256 KiB"));
            }
            Ok(data)
        };
        let (_, data, _, status) = tokio::try_join!(input, result, diagnostics, child.wait())?;
        Ok::<_, std::io::Error>((status, data))
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::REQUEST_TIMEOUT,
            "compile_timeout",
            "预览超时，草稿已保留；请简化计算后重试",
        )
    })?
    .map_err(|e| ApiError::internal(e.to_string()))?;
    if !output.0.success() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "compiler_failed",
            "编译进程停止或超出资源限额；原文已保留",
        ));
    }
    decode_compiler_worker_output(&output.1, notes_edit)
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
    for (path, bytes) in input
        .frozen_files
        .ok_or("A frozen note edit requires exact admitted files")?
    {
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
    let engine = if input.frozen_files.is_some() {
        weftext_engine::Engine::new_with_system_fonts(false)?
    } else {
        weftext_engine::Engine::new()?
    };
    let output = if let Some(files) = input.frozen_files {
        let mut decoded = std::collections::BTreeMap::new();
        let mut total = 0usize;
        for (path, bytes) in files {
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
    } else if input.paged {
        serde_json::to_value(if let Some(style) = &input.style {
            engine.render_paged_with_style(&input.source, &input.path, &input.root, style)?
        } else {
            engine.render_paged(&input.source, &input.path, &input.root)?
        })?
    } else {
        serde_json::to_value(if let Some(style) = &input.style {
            engine.render_with_style(&input.source, &input.path, &input.root, style)?
        } else {
            engine.render(&input.source, &input.path, &input.root)?
        })?
    };
    serde_json::to_writer(std::io::stdout().lock(), &output)?;
    std::io::stdout().flush()?;
    Ok(())
}

#[cfg(windows)]
mod worker_job {
    use super::ApiError;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, HANDLE},
        System::JobObjects::*,
    };
    pub struct Guard(HANDLE);
    // Owned kernel handle may be closed on the executor thread that resumes this task.
    unsafe impl Send for Guard {}
    impl Drop for Guard {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
    pub fn guard(child: &tokio::process::Child) -> Result<Guard, ApiError> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return Err(ApiError::internal("无法建立编译资源隔离"));
            }
            let guard = Guard(job);
            let mut limit: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limit.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            limit.ProcessMemoryLimit = 1024 * 1024 * 1024;
            let Some(handle) = child.raw_handle() else {
                return Err(ApiError::internal("无法连接编译进程"));
            };
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limit as *const _ as *const _,
                std::mem::size_of_val(&limit) as u32,
            ) == 0
                || AssignProcessToJobObject(job, handle as HANDLE) == 0
            {
                return Err(ApiError::internal("当前环境不支持所需编译资源隔离"));
            }
            Ok(guard)
        }
    }
}
