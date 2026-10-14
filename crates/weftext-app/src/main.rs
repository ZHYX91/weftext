use axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::Parser;
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
mod citation_api;
mod data_api;
mod embedded_output;
mod embedded_query_api;
mod export_api;
mod export_conversions;
mod history_api;
mod hosted;
mod knowledge_api;
mod media_api;
mod navigation_api;
mod writing_api;

static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../app/dist");

#[derive(Parser)]
#[command(
    name = "weftext",
    version,
    about = "Weftext local Typst writing and knowledge tools"
)]
struct Args {
    /// Run the bounded hosted service with an operator-owned configuration.
    #[arg(long, conflicts_with_all = ["open", "root", "port"])]
    server_config: Option<PathBuf>,
    #[arg(long, default_value_t = 0)]
    port: u16,
    #[arg(long)]
    data_dir: Option<PathBuf>,
    #[arg(long)]
    open: Option<PathBuf>,
    #[arg(long)]
    root: Option<PathBuf>,
    #[arg(long)]
    no_browser: bool,
    #[arg(long, hide = true)]
    render_worker: bool,
}

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
    initial: Option<Value>,
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
    operation_id: Option<String>,
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
    let recovery = core_call(&state, |core| core.recoveries()).await?;
    Ok(Json(
        json!({"version":env!("CARGO_PKG_VERSION"),"platform":std::env::consts::OS,
        "initialDocument":state.initial,"recoveries":recovery,
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
async fn create_library(State(state): State<AppState>, Json(req): Json<PathRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| match req.operation_id {
            Some(id) => c.create_library_with_id(req.path, &id),
            None => c.create_library(req.path),
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

#[derive(Serialize, Deserialize)]
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
async fn render(State(state): State<AppState>, Json(req): Json<RenderRequest>) -> ApiResult {
    compile_output(state, req, false).await
}
async fn paged(State(state): State<AppState>, Json(req): Json<RenderRequest>) -> ApiResult {
    compile_output(state, req, true).await
}
async fn compile_output(state: AppState, req: RenderRequest, paged: bool) -> ApiResult {
    if req.source.len() > 2 * 1024 * 1024 {
        return Err(ApiError::bad("预览上限为 2 MiB；原文仍可编辑保存"));
    }
    let loc = location(&state, &req.session_id)?;
    writing_api::observe_snapshot(&state, &req.session_id, &req.source, req.serial, None)?;
    let environment = writing_api::style_environment(&state, &req.session_id).await?;
    let citation_inputs = if weftext_engine::inspect_citation_declaration(&req.source)
        .map_err(|e| ApiError::bad(e.to_string()))?
        .is_some()
    {
        let root = loc.root.clone();
        let path = loc.path.clone();
        let source = req.source.clone();
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
        source: req.source.clone(),
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
    output["source"] = json!(req.source);
    output["environmentKey"] = json!(environment.key);
    output["environment"] = json!(environment.key);
    output["sessionId"] = json!(req.session_id);
    if let Some((_, citations)) = citation_inputs {
        output["citations"] = value(citations)?;
    }
    if paged {
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
    let exe = std::env::current_exe().map_err(|e| ApiError::internal(e.to_string()))?;
    let mut command = Command::new(exe);
    command
        .arg("--render-worker")
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
    serde_json::from_slice(&output.1).map_err(|_| ApiError::internal("编译输出无效"))
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    if args.render_worker {
        return worker_main();
    }
    let data = args.data_dir.unwrap_or_else(|| {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Weftext-MVP")
    });
    let mut core = Core::new(std::path::absolute(data)?)?;
    if let Some(config) = args.server_config {
        return hosted::serve(core, config).await;
    }
    let initial = if let Some(path) = args.open {
        Some(serde_json::to_value(core.open(
            std::path::absolute(path)?,
            args.root.map(std::path::absolute).transpose()?,
        )?)?)
    } else {
        None
    };
    let listener =
        tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, args.port)).await?;
    let port = listener.local_addr()?.port();
    let token = format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    );
    let state = AppState {
        core: Arc::new(Mutex::new(core)),
        token: Arc::from(token.as_str()),
        port,
        locations: Arc::new(Mutex::new(HashMap::new())),
        renders: Arc::new(Mutex::new(HashMap::new())),
        editor_versions: Arc::new(Mutex::new(HashMap::new())),
        exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
        render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
        initial,
    };
    if let Some(doc) = state.initial.as_ref() {
        remember(&state, doc).map_err(|e| e.message)?;
    }
    let app = Router::new()
        .merge(writing_api::router())
        .merge(data_api::router())
        .merge(embedded_query_api::router())
        .merge(history_api::router())
        .merge(knowledge_api::router())
        .merge(media_api::router())
        .merge(navigation_api::router())
        .merge(export_api::router())
        .route("/api/bootstrap", get(bootstrap))
        .route("/api/open", post(open))
        .route("/api/library/open", post(inventory))
        .route("/api/library/create", post(create_library))
        .route("/api/note/create", post(create_note))
        .route("/api/save", post(save))
        .route("/api/save-as", post(save_as))
        .route("/api/draft", post(draft))
        .route("/api/draft/resolve", post(resolve_draft))
        .route("/api/render", post(render))
        .route("/api/paged", post(paged))
        .route("/api/edit-literal", post(edit_literal))
        .route("/api/link/resolve", post(link_resolve))
        .route("/api/recovery", get(recovery))
        .route("/api/recovery/inspect", post(inspect_recovery))
        .route("/api/recovery/resolve", post(resolve_recovery))
        .fallback(assets)
        .layer(DefaultBodyLimit::max(96 * 1024 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state);
    let address = format!("http://127.0.0.1:{port}/#token={token}");
    println!(
        "Weftext {}\n{}\n关闭此进程停止本地服务。",
        env!("CARGO_PKG_VERSION"),
        address
    );
    if !args.no_browser {
        open_browser(&address);
    }
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
fn open_browser(address: &str) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", address])
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(address).spawn();
    }
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
