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
use weftext_messages::{DiagnosticOrigin, ErrorCauses, ErrorDescription, RawDiagnostic, UiMessage};
#[cfg(test)]
mod error_contract_tests;

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

mod bibliography_import_api;
mod builtin_module_api;
mod capture_api;
mod citation_api;
pub mod client_protocol;
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
mod request_json;
pub(crate) use request_json::RequestJson;
mod client_preferences_api;
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
#[derive(Debug, Clone, Copy)]
pub(crate) enum LocationError {
    Unavailable,
    Expired,
}
impl LocationError {
    pub(crate) fn into_core(self, code: &str) -> CoreError {
        CoreError::new(
            code,
            match self {
                Self::Unavailable => UiMessage::error_file_session_unavailable(),
                Self::Expired => UiMessage::error_file_session_expired(),
            },
        )
    }
}
impl From<LocationError> for ApiError {
    fn from(error: LocationError) -> Self {
        match error {
            LocationError::Unavailable => {
                Self::internal(UiMessage::error_file_session_unavailable())
            }
            LocationError::Expired => Self::bad(UiMessage::error_file_session_expired()),
        }
    }
}
#[derive(Clone)]
struct RenderCache {
    source: String,
    serial: u64,
    output: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApiError {
    #[serde(skip)]
    status: StatusCode,
    code: String,
    explanation: UiMessage,
    diagnostics: Vec<RawDiagnostic>,
    causes: ErrorCauses,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<Box<ApiErrorDetails>>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ApiErrorDetails {
    Core {
        #[serde(flatten)]
        error: CoreError,
    },
    CurrentSource {
        #[serde(rename = "currentSource")]
        current_source: String,
    },
    SchemaSource {
        source: String,
        #[serde(rename = "roleRef")]
        role_ref: Option<String>,
        revision: Option<String>,
        cause: Box<ApiError>,
    },
    OriginalOperation {
        fact: Value,
        cause: Box<ApiError>,
    },
    EngineDiagnostics {
        diagnostics: Vec<weftext_engine::Diagnostic>,
    },
    ProductDiagnostics {
        code: Option<String>,
        diagnostics: Vec<ErrorDescription>,
    },
    CommittedReceipt {
        receipt: weftext_core::OperationReceipt,
        cause: Box<ApiError>,
    },
    SourcePreserved {
        serial: u64,
        #[serde(rename = "sourcePreserved")]
        source_preserved: bool,
        cause: Box<ApiError>,
    },
}
impl ApiError {
    fn new(status: StatusCode, code: &str, description: impl Into<ErrorDescription>) -> Self {
        let description = description.into();
        Self {
            status,
            code: code.into(),
            explanation: description.explanation,
            diagnostics: description.diagnostics,
            causes: description.causes,
            details: None,
        }
    }
    fn bad(description: impl Into<ErrorDescription>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_request", description)
    }
    fn internal(description: impl Into<ErrorDescription>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal", description)
    }
    fn diagnostic(mut self, origin: DiagnosticOrigin, text: impl Into<String>) -> Self {
        self.diagnostics.push(RawDiagnostic {
            origin,
            text: text.into(),
        });
        self
    }
    fn from_data(status: StatusCode, code: &str, error: weftext_data::DataError) -> Self {
        let mut error = Self::from(CoreError::from_data(code, error));
        error.status = status;
        error
    }
    fn from_engine(status: StatusCode, code: &str, error: weftext_engine::EngineError) -> Self {
        let mut error = Self::from(CoreError::from_engine(code, error));
        error.status = status;
        error
    }
    fn from_product(
        status: StatusCode,
        code: &str,
        source_code: Option<String>,
        description: ErrorDescription,
        diagnostics: Vec<ErrorDescription>,
    ) -> Self {
        let mut error = Self::from(CoreError::from_product(
            code,
            source_code,
            description,
            diagnostics,
        ));
        error.status = status;
        error
    }
    fn from_core_parts(
        status: StatusCode,
        code: &str,
        description: ErrorDescription,
        details: Option<weftext_core::CoreErrorDetails>,
    ) -> Self {
        let mut core = CoreError::new(code, description);
        core.details = details;
        let mut error = Self::from(core);
        error.status = status;
        error
    }
    fn committed_receipt(
        mut self,
        receipt: weftext_core::OperationReceipt,
        cause: ApiError,
    ) -> Self {
        let original = ErrorDescription {
            explanation: cause.explanation.clone(),
            diagnostics: cause.diagnostics.clone(),
            causes: cause.causes.clone(),
        };
        let description = ErrorDescription::new(self.explanation.clone());
        if let Ok(description) = description.try_caused_by(cause.code.clone(), original) {
            self.causes = description.causes;
        }
        // The complete typed failure remains here even when the causal-chain budget is full.
        self.details = Some(Box::new(ApiErrorDetails::CommittedReceipt {
            receipt,
            cause: Box::new(cause),
        }));
        self
    }
}
impl From<CoreError> for ApiError {
    fn from(error: CoreError) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: error.code.clone(),
            explanation: error.explanation.clone(),
            diagnostics: error.diagnostics.clone(),
            causes: error.causes.clone(),
            details: Some(Box::new(ApiErrorDetails::Core { error })),
        }
    }
}
impl From<ErrorDescription> for ApiError {
    fn from(description: ErrorDescription) -> Self {
        Self::internal(description)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(json!({"error":self}))).into_response()
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
            .map_err(|_| ApiError::internal(UiMessage::error_file_service_unavailable()))?;
        f(&mut core).map_err(ApiError::from)
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_file_service_incomplete()))?
}
fn value<T: Serialize>(v: T) -> Result<Value, ApiError> {
    serde_json::to_value(v)
        .map_err(|_| ApiError::internal(UiMessage::error_response_encoding_failed()))
}
fn remember(state: &AppState, document: &Value) -> Result<(), ErrorDescription> {
    let id = document["sessionId"]
        .as_str()
        .ok_or_else(|| ErrorDescription::new(UiMessage::error_file_session_missing()))?;
    let path = document["path"]
        .as_str()
        .ok_or_else(|| ErrorDescription::new(UiMessage::error_file_path_missing()))?;
    let root = document["root"]
        .as_str()
        .ok_or_else(|| ErrorDescription::new(UiMessage::error_dependency_root_missing()))?;
    state
        .locations
        .lock()
        .map_err(|_| ErrorDescription::new(UiMessage::error_file_session_unavailable()))?
        .insert(
            id.to_owned(),
            Location {
                path: path.into(),
                root: root.into(),
            },
        );
    Ok(())
}
fn location(state: &AppState, id: &str) -> Result<Location, LocationError> {
    state
        .locations
        .lock()
        .map_err(|_| LocationError::Unavailable)?
        .get(id)
        .cloned()
        .ok_or(LocationError::Expired)
}

async fn boundary(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if host != format!("127.0.0.1:{}", state.port) && host != format!("localhost:{}", state.port) {
        return ApiError::new(
            StatusCode::FORBIDDEN,
            "host",
            UiMessage::error_host_address_disallowed(),
        )
        .into_response();
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
                UiMessage::error_local_session_invalid(),
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
}
#[derive(Deserialize)]
#[serde(untagged)]
enum OpenRequest {
    Session(ReopenSessionRequest),
    Path(OpenPathRequest),
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReopenSessionRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OpenPathRequest {
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
                    UiMessage::error_api_launch_root_unavailable(),
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
                let session_id = original["sessionId"].as_str().ok_or_else(|| {
                    CoreError::new(
                        "initialUnavailable",
                        UiMessage::error_api_launch_session_unavailable(),
                    )
                })?;
                core.reopen_local_session(session_id)
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
            Some(value(ApiError::from(
                CoreError::new(
                    "initialUnavailable",
                    UiMessage::error_api_launch_document_unavailable(),
                )
                .caused_by(error),
            ))?),
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
async fn open(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<OpenRequest>,
) -> ApiResult {
    let doc = value(
        core_call(&state, move |c| match req {
            OpenRequest::Session(req) => c.reopen_local_session(&req.session_id),
            OpenRequest::Path(req) => c.open(req.path, req.root),
        })
        .await?,
    )?;
    remember(&state, &doc)?;
    Ok(Json(doc))
}
async fn inventory(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<PathRequest>,
) -> ApiResult {
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
    RequestJson(req): RequestJson<CreateLibraryRequest>,
) -> ApiResult {
    let receipt = core_call(&state, move |c| {
        let operation_id = req
            .operation_id
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        c.create_library_with_options_and_id(req.path, req.options, &operation_id)
    })
    .await?;
    library_creation_output(&state, receipt).await.map(Json)
}
pub(crate) async fn library_creation_output(
    state: &AppState,
    receipt: weftext_core::LibraryInfo,
) -> Result<Value, ApiError> {
    let id = receipt.operation_id.clone();
    let mut output = value(&receipt)?;
    let delivery = core_call(state, move |core| core.reopen_local_library_creation(&id)).await;
    let delivered = (|| -> Result<(), ApiError> {
        let (library, document) = delivery?;
        output["library"] = value(library)?;
        if let Some(document) = document {
            let document = value(document)?;
            remember(state, &document)?;
            output["rootDocument"] = document;
        }
        Ok(())
    })();
    if let Err(error) = delivered {
        output["deliveryError"] = json!(error);
    }
    Ok(output)
}
async fn cancel_library_creation(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CreateLibraryRequest>,
) -> ApiResult {
    let operation_id = req
        .operation_id
        .ok_or_else(|| ApiError::bad(UiMessage::error_creation_operation_id_required()))?;
    Ok(Json(value(
        core_call(&state, move |c| {
            c.cancel_library_creation_with_options_and_id(req.path, req.options, &operation_id)
        })
        .await?,
    )?))
}
async fn create_note(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CreateNote>,
) -> ApiResult {
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
async fn save(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<SaveRequest>,
) -> ApiResult {
    let id = req.operation_id.clone();
    let output = value(core_call(&state, move |c| c.save(req)).await?)?;
    Ok(Json(
        knowledge_api::file_save_delivery(&state, id, output).await,
    ))
}
async fn save_as(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<SaveAsRequest>,
) -> ApiResult {
    let id = req.operation_id.clone();
    let output = value(core_call(&state, move |c| c.save_as(req)).await?)?;
    Ok(Json(
        knowledge_api::file_save_delivery(&state, id, output).await,
    ))
}
async fn draft(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<DraftRequest>,
) -> ApiResult {
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
    RequestJson(req): RequestJson<OperationRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |c| c.inspect_recovery(&req.operation_id)).await?,
    )?))
}
async fn resolve_recovery(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RecoveryRequest>,
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
    RequestJson(req): RequestJson<DraftResolution>,
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
async fn render(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RenderRequest>,
) -> ApiResult {
    compile_output(state, req, false, None).await
}
async fn paged(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RenderRequest>,
) -> ApiResult {
    compile_output(state, req, true, None).await
}
async fn proposal_preview(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<ProposalPreviewRequest>,
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
        return Err(ApiError::bad(UiMessage::error_preview_source_byte_limit()));
    }
    let loc = location(&state, &req.session_id)?;
    writing_api::observe_snapshot(&state, &req.session_id, &req.source, req.serial, None)?;
    let environment = writing_api::style_environment(&state, &req.session_id).await?;
    let citation_inputs = if weftext_engine::inspect_citation_declaration(&compiled_source)
        .map_err(|e| ApiError::from_engine(StatusCode::BAD_REQUEST, "invalid_request", e))?
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
        .map_err(|_| ApiError::internal(UiMessage::error_compile_service_unavailable()))?;
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
        .map_err(|_| ApiError::internal(UiMessage::error_compile_session_unavailable()))?;
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
        return Err(ApiError::bad(UiMessage::error_frozen_compile_input_limit()));
    }
    let _permit = state
        .render_gate
        .acquire()
        .await
        .map_err(|_| ApiError::internal(UiMessage::error_compile_service_unavailable()))?;
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
        return Err(ApiError::bad(UiMessage::error_notes_frozen_input_limit()));
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
    serde_json::from_value(output)
        .map_err(|_| ApiError::internal(UiMessage::error_notes_dependency_receipt_invalid()))
}

async fn bounded_compiler_worker(input: Value, notes_edit: bool) -> Result<Value, ApiError> {
    let exe = std::env::current_exe().map_err(|e| {
        ApiError::internal(UiMessage::error_api_worker_executable_unavailable())
            .diagnostic(DiagnosticOrigin::Os, e.to_string())
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
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let mut child = command.spawn().map_err(|e| {
        ApiError::internal(UiMessage::error_api_worker_start_failed())
            .diagnostic(DiagnosticOrigin::Os, e.to_string())
    })?;
    #[cfg(windows)]
    let _job = worker_job::guard(&child)?;
    let bytes = serde_json::to_vec(&input).map_err(|e| {
        ApiError::internal(UiMessage::error_api_worker_input_encoding_failed())
            .diagnostic(DiagnosticOrigin::Json, e.to_string())
    })?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| ApiError::internal(UiMessage::error_compiler_input_unavailable()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ApiError::internal(UiMessage::error_compiler_output_unavailable()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| ApiError::internal(UiMessage::error_compiler_diagnostic_unavailable()))?;
    // Bound the pipe while reading, rather than after buffering arbitrary output.
    // Dropping this future also drops the child and its kill-on-close Windows job.
    let output = tokio::time::timeout(Duration::from_secs(20), async {
        let input = async {
            stdin.write_all(&bytes).await?;
            drop(stdin);
            Ok::<(), WorkerExecutionError>(())
        };
        let result = async {
            let mut data = Vec::new();
            stdout
                .take(24 * 1024 * 1024 + 1)
                .read_to_end(&mut data)
                .await?;
            if data.len() > 24 * 1024 * 1024 {
                return Err(WorkerExecutionError::OutputBudget);
            }
            Ok(data)
        };
        let diagnostics = async {
            let mut data = Vec::new();
            stderr.take(256 * 1024 + 1).read_to_end(&mut data).await?;
            if data.len() > 256 * 1024 {
                return Err(WorkerExecutionError::DiagnosticBudget);
            }
            Ok(data)
        };
        let status = async { child.wait().await.map_err(WorkerExecutionError::Io) };
        let (_, data, _, status) = tokio::try_join!(input, result, diagnostics, status)?;
        Ok::<_, WorkerExecutionError>((status, data))
    })
    .await
    .map_err(|_| {
        ApiError::new(
            StatusCode::REQUEST_TIMEOUT,
            "compile_timeout",
            UiMessage::error_preview_timeout(),
        )
    })?
    .map_err(WorkerExecutionError::into_api_error)?;
    if !output.0.success() {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "compiler_failed",
            UiMessage::error_compiler_process_failed(),
        ));
    }
    decode_compiler_worker_output(&output.1, notes_edit)
}

#[derive(Debug)]
enum WorkerExecutionError {
    Io(std::io::Error),
    OutputBudget,
    DiagnosticBudget,
}
impl From<std::io::Error> for WorkerExecutionError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
impl WorkerExecutionError {
    fn into_api_error(self) -> ApiError {
        match self {
            Self::Io(error) => ApiError::internal(UiMessage::error_api_worker_io_failure())
                .diagnostic(DiagnosticOrigin::Os, error.to_string()),
            Self::OutputBudget => ApiError::internal(UiMessage::error_api_worker_output_budget()),
            Self::DiagnosticBudget => {
                ApiError::internal(UiMessage::error_api_worker_diagnostic_budget())
            }
        }
    }
}
const RENDER_WORKER_FAILURE_KEY: &str = "weftextRenderWorkerFailure";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RenderWorkerFailureEnvelope {
    #[serde(rename = "weftextRenderWorkerFailure")]
    failure: RenderWorkerFailure,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenderWorkerFailure {
    version: u8,
    code: RenderWorkerFailureCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<ErrorDescription>,
    product_diagnostics: Vec<ErrorDescription>,
}

#[derive(Serialize, Deserialize)]
enum RenderWorkerFailureCode {
    #[serde(rename = "display_tree_budget")]
    DisplayTreeBudget,
    #[serde(rename = "product_error")]
    ProductError,
}

fn write_render_worker_failure(
    error: Box<dyn std::error::Error>,
    writer: &mut impl std::io::Write,
) -> Result<(), Box<dyn std::error::Error>> {
    // Typed product failures cross the private pipe without rendering or parsing
    // their display text. Unrecognized errors remain original stderr failures.
    let (code, description, product_diagnostics) =
        match error.downcast_ref::<weftext_engine::EngineError>() {
            Some(weftext_engine::EngineError::DisplayTreeBudget {
                product_diagnostics,
            }) => (
                RenderWorkerFailureCode::DisplayTreeBudget,
                None,
                product_diagnostics.clone(),
            ),
            Some(error) => {
                let (description, diagnostics) = error.clone().into_parts();
                (
                    RenderWorkerFailureCode::ProductError,
                    Some(description),
                    diagnostics,
                )
            }
            None => match error.downcast_ref::<ErrorDescription>() {
                Some(description) => (
                    RenderWorkerFailureCode::ProductError,
                    Some(description.clone()),
                    Vec::new(),
                ),
                None => {
                    if let Some(source) = error.downcast_ref::<std::io::Error>() {
                        (
                            RenderWorkerFailureCode::ProductError,
                            Some(
                                ErrorDescription::new(UiMessage::error_api_worker_pipe_failure())
                                    .diagnostic(DiagnosticOrigin::Os, source.to_string()),
                            ),
                            Vec::new(),
                        )
                    } else if let Some(source) = error.downcast_ref::<serde_json::Error>() {
                        (
                            RenderWorkerFailureCode::ProductError,
                            Some(
                                ErrorDescription::new(
                                    UiMessage::error_api_worker_output_encoding_failed(),
                                )
                                .diagnostic(DiagnosticOrigin::Json, source.to_string()),
                            ),
                            Vec::new(),
                        )
                    } else {
                        return Err(error);
                    }
                }
            },
        };
    serde_json::to_writer(
        &mut *writer,
        &RenderWorkerFailureEnvelope {
            failure: RenderWorkerFailure {
                version: 2,
                code,
                description,
                product_diagnostics,
            },
        },
    )?;
    writer.flush()?;
    Ok(())
}

fn decode_compiler_worker_output(bytes: &[u8], _notes_edit: bool) -> Result<Value, ApiError> {
    let output: Value = serde_json::from_slice(bytes).map_err(|error| {
        ApiError::internal(UiMessage::error_compiler_output_invalid())
            .diagnostic(DiagnosticOrigin::Json, error.to_string())
    })?;
    let object = output
        .as_object()
        .ok_or_else(|| ApiError::internal(UiMessage::error_compiler_output_invalid()))?;
    if !object.contains_key(RENDER_WORKER_FAILURE_KEY) {
        return Ok(output);
    }
    // Struct decoding rejects duplicate/unknown fields; the object checks also
    // exclude JSON sequence representations of a named struct. A nested author
    // field in a normal RenderOutput never enters this private protocol.
    let exact_objects = object.len() == 1
        && output[RENDER_WORKER_FAILURE_KEY]
            .as_object()
            .is_some_and(|object| object.len() == 3 || object.len() == 4);
    if exact_objects {
        match serde_json::from_slice::<RenderWorkerFailureEnvelope>(bytes) {
            Ok(RenderWorkerFailureEnvelope {
                failure:
                    RenderWorkerFailure {
                        version: 2,
                        code: RenderWorkerFailureCode::DisplayTreeBudget,
                        description: None,
                        product_diagnostics,
                    },
            }) if output[RENDER_WORKER_FAILURE_KEY].as_object().unwrap().len() == 3 => {
                return Err(ApiError::from_product(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "display_tree_budget",
                    None,
                    ErrorDescription::new(UiMessage::error_preview_display_tree_budget()),
                    product_diagnostics,
                ));
            }
            Ok(RenderWorkerFailureEnvelope {
                failure:
                    RenderWorkerFailure {
                        version: 2,
                        code: RenderWorkerFailureCode::ProductError,
                        description: Some(description),
                        product_diagnostics: diagnostics,
                    },
            }) => {
                return Err(ApiError::from_product(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "worker_product_error",
                    None,
                    description,
                    diagnostics,
                ));
            }
            _ => {}
        }
    }
    Err(ApiError::internal(
        UiMessage::error_compiler_output_invalid(),
    ))
}

#[cfg(test)]
mod render_worker_codec_tests {
    use super::*;
    const LOOKALIKE: &str = "显示树超过安全预算；请使用源码入口";

    const FAILURE: &[u8] =
        br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]}}"#;

    #[test]
    fn exact_typed_engine_budget_emits_only_the_private_code() {
        let mut output = Vec::new();
        write_render_worker_failure(
            Box::new(weftext_engine::EngineError::DisplayTreeBudget {
                product_diagnostics: Vec::new(),
            }),
            &mut output,
        )
        .unwrap();
        assert_eq!(output, FAILURE);
        let error = decode_compiler_worker_output(&output, false).unwrap_err();
        assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(error.code, "display_tree_budget");
        assert_eq!(
            error.explanation,
            UiMessage::error_preview_display_tree_budget()
        );
        assert!(error.diagnostics.is_empty());
        assert!(error.causes.as_slice().is_empty());
        let Some(details) = error.details else {
            panic!("Core error boundary missing")
        };
        let ApiErrorDetails::Core { error } = *details else {
            panic!("incorrect details boundary")
        };
        assert!(error.details.is_none());
    }

    #[test]
    fn untyped_errors_cannot_impersonate_the_private_protocol() {
        #[derive(Debug)]
        struct Unattributed(&'static str);
        impl std::fmt::Display for Unattributed {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.0)
            }
        }
        impl std::error::Error for Unattributed {}
        let errors: Vec<Box<dyn std::error::Error>> = vec![
            Box::new(Unattributed(LOOKALIKE)),
            Box::new(Unattributed("unrelated private-source-or-path")),
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
    fn typed_product_errors_round_trip_causes_and_original_diagnostics_in_both_workers() {
        let original = ErrorDescription::new(UiMessage::error_api_worker_output_budget())
            .try_caused_by(
                "original_machine_code",
                ErrorDescription::new(UiMessage::error_compiler_output_invalid()).diagnostic(
                    DiagnosticOrigin::Typst,
                    "原始 diagnostic <source>\n{literal}",
                ),
            )
            .unwrap();
        let mut output = Vec::new();
        write_render_worker_failure(Box::new(original.clone()), &mut output).unwrap();
        assert_ne!(output, FAILURE);
        for notes in [false, true] {
            let error = decode_compiler_worker_output(&output, notes).unwrap_err();
            assert_eq!(error.code, "worker_product_error");
            assert_eq!(error.explanation, original.explanation);
            assert_eq!(error.diagnostics, original.diagnostics);
            assert_eq!(error.causes, original.causes);
        }
        let mut output = Vec::new();
        write_render_worker_failure(
            Box::new(weftext_engine::EngineError::new(
                UiMessage::error_preview_display_tree_budget(),
            )),
            &mut output,
        )
        .unwrap();
        assert_ne!(output, FAILURE);
        assert_eq!(
            decode_compiler_worker_output(&output, false)
                .unwrap_err()
                .code,
            "worker_product_error"
        );
    }

    #[test]
    fn genuine_io_keeps_its_origin_and_cannot_impersonate_display_tree_budget() {
        let mut output = Vec::new();
        write_render_worker_failure(Box::new(std::io::Error::other(LOOKALIKE)), &mut output)
            .unwrap();
        assert_ne!(output, FAILURE);
        let error = decode_compiler_worker_output(&output, false).unwrap_err();
        assert_eq!(error.code, "worker_product_error");
        assert_eq!(
            error.explanation,
            UiMessage::error_api_worker_pipe_failure()
        );
        assert_eq!(
            error.diagnostics,
            vec![RawDiagnostic {
                origin: DiagnosticOrigin::Os,
                text: LOOKALIKE.into()
            }]
        );
    }

    #[test]
    fn budget_and_product_frames_preserve_all_parallel_facts_for_both_workers() {
        let parallel = (0..40)
            .map(|index| {
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: format!("original-{index}"),
                    },
                ))
                .diagnostic(
                    DiagnosticOrigin::Typst,
                    format!("original diagnostic {index}\n{{literal}}"),
                )
            })
            .collect::<Vec<_>>();
        for (engine, code) in [
            (
                weftext_engine::EngineError::DisplayTreeBudget {
                    product_diagnostics: parallel.clone(),
                },
                "display_tree_budget",
            ),
            (
                weftext_engine::EngineError::new(UiMessage::error_compiler_output_invalid())
                    .with_product_diagnostics(parallel.clone()),
                "worker_product_error",
            ),
        ] {
            let mut output = Vec::new();
            write_render_worker_failure(Box::new(engine), &mut output).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(&output).unwrap()[RENDER_WORKER_FAILURE_KEY]["version"],
                2
            );
            for notes in [false, true] {
                let error = decode_compiler_worker_output(&output, notes).unwrap_err();
                assert_eq!(error.code, code);
                assert_eq!(error.status, StatusCode::UNPROCESSABLE_ENTITY);
                let Some(details) = error.details else {
                    panic!("Core facts missing")
                };
                let ApiErrorDetails::Core { error } = *details else {
                    panic!("wrong facts boundary")
                };
                let Some(weftext_core::CoreErrorDetails::Aggregate { groups }) = error.details
                else {
                    panic!("parallel group missing")
                };
                let [
                    weftext_core::ErrorDetailGroup::Product {
                        code: None,
                        diagnostics,
                    },
                ] = groups.as_slice()
                else {
                    panic!("wrong parallel shape")
                };
                assert_eq!(diagnostics, &parallel);
            }
        }
    }

    #[test]
    fn invalid_or_unknown_failure_envelopes_use_only_fixed_invalid_output() {
        for bytes in [
            br#"{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget","productDiagnostics":[]}}"#.as_slice(),
            br#"{"weftextRenderWorkerFailure":{"version":3,"code":"display_tree_budget","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"private-unknown-code","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":"2","code":"display_tree_budget","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[],"message":"private-source-or-path"}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[],"description":null}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":null}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"product_error","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"product_error","description":null,"productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]},"extra":"private-source-or-path"}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"version":2,"code":"display_tree_budget","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","code":"display_tree_budget","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[],"productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]},"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]}}"#,
            br#"{"weftextRenderWorkerFailure":[2,"display_tree_budget",[]]}"#,
            br#"[{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]}}]"#,
            br#"{"weftextRenderWorkerFailure":null}"#,
            br#"{"weftextRenderWorkerFailure":{"version":2,"code":"display_tree_budget","productDiagnostics":[]}} trailing"#,
            br#"{"weftextRenderWorkerFailure":"#,
        ] {
            let error = decode_compiler_worker_output(bytes, false).unwrap_err();
            assert_eq!(error.code, "internal");
            assert_eq!(error.explanation, UiMessage::error_compiler_output_invalid());
            assert!(error.diagnostics.iter().all(|diagnostic| diagnostic.origin == DiagnosticOrigin::Json));
            assert!(error.details.is_none());
        }
    }

    #[test]
    fn ordinary_outputs_and_nested_author_fields_are_unchanged() {
        for ordinary in [
            json!({"ok":true,"tree":null,"sourceInfo":{},"products":null}),
            json!({"ok":false,"diagnostics":[{"message":LOOKALIKE}],"tree":{"attrs":{"weftextRenderWorkerFailure":{"version":1,"code":"display_tree_budget"}}}}),
        ] {
            assert_eq!(
                decode_compiler_worker_output(&serde_json::to_vec(&ordinary).unwrap(), false)
                    .unwrap(),
                ordinary
            );
        }
    }

    #[test]
    fn notes_output_uses_the_same_closed_error_protocol() {
        assert_eq!(
            decode_compiler_worker_output(FAILURE, true)
                .unwrap_err()
                .code,
            "display_tree_budget"
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
async fn edit_literal(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<EditRequest>,
) -> ApiResult {
    location(&state, &req.session_id)?;
    let cache = state
        .renders
        .lock()
        .map_err(|_| ApiError::internal(UiMessage::error_compile_session_unavailable()))?;
    let proof = cache
        .get(&req.session_id)
        .filter(|p| p.serial == req.serial && p.source == req.source)
        .ok_or_else(|| ApiError::bad(UiMessage::error_edit_preview_stale()))?;
    let (from, to) = if req.node_key == "@body" {
        let insertion = &proof.output["sourceInfo"]["bodyInsertion"];
        if insertion["writable"] != true {
            return Err(ApiError::bad(UiMessage::error_body_insertion_unavailable()));
        }
        let pos = insertion["from"]
            .as_u64()
            .filter(|p| Some(*p) == insertion["to"].as_u64())
            .ok_or_else(|| ApiError::bad(UiMessage::error_body_insertion_unavailable()))?
            as usize;
        (pos, pos)
    } else {
        let node = find_node(&proof.output["tree"], &req.node_key)
            .filter(|n| n["writable"] == true)
            .ok_or_else(|| ApiError::bad(UiMessage::error_source_edit_required()))?;
        let origin = &node["origin"];
        if origin["isMain"] != true {
            return Err(ApiError::bad(UiMessage::error_generated_content_read_only()));
        }
        let from = origin["from"]
            .as_u64()
            .ok_or_else(|| ApiError::bad(UiMessage::error_edit_origin_missing()))?
            as usize;
        let to = origin["to"]
            .as_u64()
            .ok_or_else(|| ApiError::bad(UiMessage::error_edit_origin_missing()))?
            as usize;
        if req.source.get(from..to) != origin["raw"].as_str() {
            return Err(ApiError::bad(UiMessage::error_edit_origin_bytes_changed()));
        }
        (from, to)
    };
    let literal = weftext_engine::encode_literal(&req.text)
        .map_err(|e| ApiError::from_engine(StatusCode::BAD_REQUEST, "invalid_request", e))?;
    if req.node_key != "@body" && req.text.contains(['\r', '\n']) {
        return Err(ApiError::bad(UiMessage::error_inline_edit_only()));
    }
    let encoded = if req.node_key == "@body" {
        let body = match req.kind.as_deref().unwrap_or("paragraph") {
            "paragraph" => literal,
            "heading" => {
                let level = req.level.unwrap_or(1);
                if !(1..=9).contains(&level) {
                    return Err(ApiError::bad(UiMessage::error_heading_level_range()));
                }
                format!("{} {}", "=".repeat(level as usize), literal)
            }
            "bullet" => format!("- {literal}"),
            "strong" => format!("*{literal}*"),
            "emph" => format!("_{literal}_"),
            _ => {
                return Err(ApiError::bad(
                    UiMessage::error_body_insertion_kind_unsupported(),
                ));
            }
        };
        format!("{}{body}\n", if from > 0 { "\n\n" } else { "" })
    } else {
        literal
    };
    if !req.source.is_char_boundary(from) || !req.source.is_char_boundary(to) || from > to {
        return Err(ApiError::bad(
            UiMessage::error_edit_character_boundary_invalid(),
        ));
    }
    let start = req.source[..from].encode_utf16().count();
    let end = req.source[..to].encode_utf16().count();
    let source = format!("{}{}{}", &req.source[..from], encoded, &req.source[to..]);
    Ok(Json(
        json!({"source":source,"edit":{"from":start,"to":end,"text":encoded}}),
    ))
}

async fn link_resolve(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<LinkRequest>,
) -> ApiResult {
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
            _ => return Err(ApiError::bad(UiMessage::error_link_kind_unsupported())),
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
            .map_err(|_| ApiError::bad(UiMessage::error_file_link_invalid()))?;
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
    let base = url::Url::from_file_path(&loc.path)
        .map_err(|_| ApiError::bad(UiMessage::error_link_source_path_invalid()))?;
    let resolved = base
        .join(path)
        .map_err(|_| ApiError::bad(UiMessage::error_file_path_invalid()))?;
    let path = resolved
        .to_file_path()
        .map_err(|_| ApiError::bad(UiMessage::error_file_path_invalid()))?;
    link_path(path, loc.root, fragment)
}

fn link_path(path: PathBuf, root: PathBuf, fragment: Option<String>) -> ApiResult {
    let path = path
        .canonicalize()
        .map_err(|_| ApiError::bad(UiMessage::error_link_target_missing()))?;
    let root = root
        .canonicalize()
        .map_err(|_| ApiError::bad(UiMessage::error_link_dependency_root_unavailable()))?;
    if !path.starts_with(&root) {
        return Err(ApiError::bad(UiMessage::error_link_outside_project_root()));
    }
    if path
        .extension()
        .is_none_or(|s| !s.eq_ignore_ascii_case("typ"))
    {
        return Err(ApiError::bad(
            UiMessage::error_ordinary_source_file_required(),
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
                            _ => {
                                return Err(
                                    ApiError::bad(UiMessage::error_file_path_unsupported()),
                                );
                            }
                        }
                    }
                    return Ok(normalized);
                }
                Prefix::Disk(_) => {}
                _ => return Err(ApiError::bad(UiMessage::error_local_drive_path_required())),
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
        return Err(ErrorDescription::new(UiMessage::error_api_worker_notes_input_budget()).into());
    }
    let NotesWorkerRequest { input, request } =
        serde_json::from_slice(&bytes).map_err(|error| {
            ErrorDescription::new(UiMessage::error_api_worker_request_invalid())
                .diagnostic(DiagnosticOrigin::Json, error.to_string())
        })?;
    let mut files = std::collections::BTreeMap::new();
    let mut total = 0usize;
    for (path, bytes) in input.frozen_files.ok_or_else(|| {
        ErrorDescription::new(UiMessage::error_api_worker_notes_frozen_files_required())
    })? {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .map_err(|error| {
                ErrorDescription::new(UiMessage::error_api_worker_frozen_encoding_invalid())
                    .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
            })?;
        total = total.checked_add(decoded.len()).ok_or_else(|| {
            ErrorDescription::new(UiMessage::error_api_worker_notes_size_overflow())
        })?;
        if total > 64 * 1024 * 1024 {
            return Err(ErrorDescription::new(
                UiMessage::error_api_worker_notes_frozen_files_budget(),
            )
            .into());
        }
        files.insert(path, decoded);
    }
    if input.source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(
            ErrorDescription::new(UiMessage::error_api_worker_notes_source_budget()).into(),
        );
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
        return Err(
            ErrorDescription::new(UiMessage::error_api_worker_notes_result_budget()).into(),
        );
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
        return Err(
            ErrorDescription::new(UiMessage::error_api_worker_render_input_budget()).into(),
        );
    }
    let input: WorkerRequest = serde_json::from_str(&input).map_err(|error| {
        ErrorDescription::new(UiMessage::error_api_worker_request_invalid())
            .diagnostic(DiagnosticOrigin::Json, error.to_string())
    })?;
    let engine = if input.frozen_files.is_some() {
        weftext_engine::Engine::new_with_system_fonts(false)?
    } else {
        weftext_engine::Engine::new()?
    };
    let output = if let Some(files) = input.frozen_files {
        let mut decoded = std::collections::BTreeMap::new();
        let mut total = 0usize;
        for (path, bytes) in files {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(bytes)
                .map_err(|error| {
                    ErrorDescription::new(UiMessage::error_api_worker_frozen_encoding_invalid())
                        .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
                })?;
            total = total.checked_add(bytes.len()).ok_or_else(|| {
                ErrorDescription::new(UiMessage::error_api_worker_render_size_overflow())
            })?;
            if total > 64 * 1024 * 1024 {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_worker_render_frozen_files_budget(),
                )
                .into());
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
    use super::{ApiError, DiagnosticOrigin, UiMessage};
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
                let diagnostic = std::io::Error::last_os_error();
                return Err(
                    ApiError::internal(UiMessage::error_compile_isolation_create_failed())
                        .diagnostic(DiagnosticOrigin::Os, diagnostic.to_string()),
                );
            }
            let guard = Guard(job);
            let mut limit: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            limit.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_PROCESS_MEMORY;
            limit.ProcessMemoryLimit = 1024 * 1024 * 1024;
            let Some(handle) = child.raw_handle() else {
                return Err(ApiError::internal(
                    UiMessage::error_compile_process_handle_unavailable(),
                ));
            };
            if SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limit as *const _ as *const _,
                std::mem::size_of_val(&limit) as u32,
            ) == 0
                || AssignProcessToJobObject(job, handle as HANDLE) == 0
            {
                let diagnostic = std::io::Error::last_os_error();
                return Err(
                    ApiError::internal(UiMessage::error_compile_isolation_unsupported())
                        .diagnostic(DiagnosticOrigin::Os, diagnostic.to_string()),
                );
            }
            Ok(guard)
        }
    }
}
