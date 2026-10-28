//! HTTP adapters for exact-snapshot Typst authoring and native library styles.
//! Draft proposals remain in the shared source buffer; Core owns persistence.

use super::{ApiError, ApiErrorDetails, ApiResult, AppState, core_call, location, value};
#[cfg(test)]
use crate::CoreTestFixture;
use crate::RequestJson;
use crate::readonly_evidence::{ReadonlyEvidence, capture_readonly};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    sync::Arc,
};
use weftext_core::{ConfigDocument, ConfigRole, LibraryStyleDocument};
use weftext_messages::UiMessage;

/// The most recent draft supplied by this local session. This is an exact
/// request basis, not a persisted revision or authorization obtained from a hash.
#[derive(Clone)]
pub(crate) struct EditorVersion {
    pub(crate) source: String,
    pub(crate) serial: u64,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/editor-map", post(editor_map))
        .route("/api/edit", post(edit))
        .route("/api/fonts", get(fonts))
        .route("/api/editor/endnotes-module", get(endnotes_module))
        .route("/api/styles/read", post(styles_read))
        .route("/api/styles/prepare", post(styles_prepare))
        .route("/api/styles/apply", post(styles_apply))
}

pub(crate) const ENDNOTES_MODULE: &str =
    include_str!("../../weftext-export/assets/weftext-endnotes.typ");

/// Public, fixed distribution bytes. Installing them remains a separately
/// authorized owned-resource operation and never saves the current note.
pub(crate) async fn endnotes_module() -> ApiResult {
    Ok(Json(
        json!({"name":"weftext-endnotes.typ","source":ENDNOTES_MODULE,
        "sha256":weftext_engine::source_hash(ENDNOTES_MODULE),"version":"1"}),
    ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StyleReadRequest {
    session_id: Option<String>,
    root: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StylePrepareRequest {
    session_id: Option<String>,
    root: Option<PathBuf>,
    operation_id: String,
    base_id: String,
    source: Option<String>,
    changes: Option<BTreeMap<String, String>>,
    enabled: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StyleApplyRequest {
    session_id: Option<String>,
    root: Option<PathBuf>,
    operation_id: String,
    plan_id: String,
}

fn same_path(a: &Path, b: &Path) -> bool {
    #[cfg(windows)]
    {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        a == b
    }
}

fn style_root(
    state: &AppState,
    session_id: Option<&str>,
    requested: Option<PathBuf>,
) -> Result<PathBuf, ApiError> {
    if let Some(session_id) = session_id {
        let root = location(state, session_id)?.root;
        if requested
            .as_ref()
            .is_some_and(|requested| !same_path(requested, &root))
        {
            return Err(ApiError::bad(
                UiMessage::error_api_writing_style_library_mismatch(),
            ));
        }
        return Ok(root);
    }
    requested.ok_or_else(|| ApiError::bad(UiMessage::error_api_writing_library_or_note_required()))
}

fn style_state(document: LibraryStyleDocument) -> Result<Value, ApiError> {
    let inspection = weftext_engine::inspect_library_style(&document.source);
    let mut warnings = Vec::new();
    if !document.installed {
        warnings.push(UiMessage::error_api_writing_default_style_not_installed());
    }
    if !inspection.syntax_valid {
        warnings.push(UiMessage::error_api_writing_style_source_syntax_warning());
    } else if inspection.controls.iter().any(|control| !control.editable) {
        warnings.push(UiMessage::error_api_writing_custom_style_control_warning());
    }
    let controls: Vec<_> = inspection
        .controls
        .into_iter()
        .map(|control| {
            // These values stay exact native Typst expressions, including units,
            // quoted language strings and font tuples. There is no CSS authority.
            json!({"key":control.key,"label":control.label,"kind":"text",
            "value":control.value,"editable":control.editable,"reason":control.reason})
        })
        .collect();
    Ok(
        json!({"path":document.path,"source":document.source,"baseId":document.base_id,
        "enabled":document.enabled,"entryFunction":document.entry_function,
        "resourceRef":document.resource_ref,"installed":document.installed,
        "sourceHash":inspection.source_hash,"syntaxValid":inspection.syntax_valid,
        "controls":controls,"warnings":warnings}),
    )
}

async fn styles_read(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<StyleReadRequest>,
) -> ApiResult {
    let root = style_root(&state, req.session_id.as_deref(), req.root)?;
    let document = core_call(&state, move |core| core.read_library_style(root)).await?;
    Ok(Json(style_state(document)?))
}

async fn styles_prepare(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<StylePrepareRequest>,
) -> ApiResult {
    let root = style_root(&state, req.session_id.as_deref(), req.root)?;
    if req.source.is_some()
        && req
            .changes
            .as_ref()
            .is_some_and(|changes| !changes.is_empty())
    {
        return Err(ApiError::bad(
            UiMessage::error_api_writing_separate_style_proposals_required(),
        ));
    }
    let read_root = root.clone();
    let base_id = req.base_id.clone();
    let before = core_call(&state, move |core| {
        core.library_style_basis(read_root, &base_id)
    })
    .await?;
    let before_source = before.source.clone();
    let source = before.source.clone();
    let after_source = match req.source {
        Some(source) => source,
        None => {
            let changes = req.changes.unwrap_or_default();
            crate::work::spawn_blocking(move || {
                let hash = weftext_engine::source_hash(&source);
                weftext_engine::apply_library_style_changes(&source, &hash, &changes)
                    .map(|edited| edited.source)
            })
            .await
            .map_err(|_| ApiError::internal(UiMessage::error_api_writing_style_edit_incomplete()))?
            .map_err(|error| {
                ApiError::from_engine(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "unsupported_style_edit",
                    error,
                )
            })?
        }
    };
    validate_source(&after_source, None)?;
    if !weftext_engine::inspect_library_style(&after_source).syntax_valid {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "style_syntax_invalid",
            UiMessage::error_api_writing_style_syntax_invalid(),
        ));
    }
    let enabled = req.enabled.unwrap_or(before.enabled);
    if after_source == before_source && enabled == before.enabled {
        return Err(ApiError::bad(
            UiMessage::error_api_writing_style_change_missing(),
        ));
    }
    let source = after_source.clone();
    let plan = core_call(&state, move |core| {
        core.prepare_library_style(root, &req.base_id, &source, enabled, &req.operation_id)
    })
    .await?;
    let summary: Vec<UiMessage> = plan
        .changes
        .iter()
        .map(|change| {
            UiMessage::error_api_writing_style_plan_change(
                weftext_messages::params::ErrorApiWritingStylePlanChange {
                    action: change.action.clone(),
                    path: change.path.clone(),
                    bytes: weftext_messages::UiInteger::new(
                        i64::try_from(change.bytes).expect("Plan byte count fits i64"),
                    )
                    .expect("Plan byte budget fits a safe integer"),
                },
            )
        })
        .collect();
    Ok(Json(
        json!({"operationId":plan.operation_id,"planId":plan.plan_id,
        "beforeSource":before_source,"afterSource":after_source,"enabled":enabled,
        "summary":summary,"status":plan.status}),
    ))
}

fn invalidate_style_projections(state: &AppState, root: &Path) -> Result<(), ApiError> {
    let sessions: Vec<String> = state
        .locations
        .lock()
        .map_err(|_| ApiError::internal(UiMessage::error_file_session_unavailable()))?
        .iter()
        .filter(|(_, location)| same_path(&location.root, root))
        .map(|(id, _)| id.clone())
        .collect();
    let mut renders = state.renders.lock().map_err(|_| {
        ApiError::internal(UiMessage::error_api_writing_compilation_session_unavailable())
    })?;
    for session in sessions {
        renders.remove(&session);
    }
    Ok(())
}

async fn styles_apply(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<StyleApplyRequest>,
) -> ApiResult {
    let root = style_root(&state, req.session_id.as_deref(), req.root)?;
    let operation_root = root.clone();
    let receipt = core_call(&state, move |core| {
        let plan = core.operation_details(&req.operation_id)?;
        let settings = operation_root.join(".weftext-settings.json");
        if plan.kind != "configureStyle"
            || !plan
                .changes
                .iter()
                .any(|change| same_path(Path::new(&change.path), &settings))
        {
            return Err(weftext_core::CoreError::new(
                "operationConflict",
                UiMessage::error_api_writing_style_plan_library_mismatch(),
            ));
        }
        core.execute_operation(&req.operation_id, &req.plan_id)
    })
    .await?;
    let fresh = async {
        invalidate_style_projections(&state, &root)?;
        let document = core_call(&state, move |core| core.read_library_style(root)).await?;
        style_state(document)
    }
    .await;
    match fresh {
        Ok(mut document) => {
            document["operationReceipt"] = value(receipt)?;
            Ok(Json(document))
        }
        Err(error) => {
            let result = ApiError::new(
                StatusCode::CONFLICT,
                "style_state_unavailable",
                UiMessage::error_api_writing_committed_style_state_unavailable(),
            )
            .committed_receipt(receipt, error);
            Err(result)
        }
    }
}

/// A frozen compilation profile. The key describes its contents for consumers;
/// the opaque Core basis separately proves current object and permission state.
pub(crate) struct StyleEnvironment {
    pub(crate) style: Option<weftext_engine::StyleModule>,
    pub(crate) key: String,
    root: PathBuf,
    basis: StyleEnvironmentBasis,
    evidence: Arc<ReadonlyEvidence>,
}

#[derive(Clone)]
enum StyleEnvironmentBasis {
    Ordinary,
    Settings { path: String, base_id: String },
    Module { base_id: String },
}

enum StyleProfile {
    Ordinary,
    Disabled(ConfigDocument),
    Library(LibraryStyleDocument),
}

fn selected_style_enabled(settings: &ConfigDocument) -> weftext_core::Result<bool> {
    let value: Value = settings
        .bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice(bytes))
        .transpose()?
        .unwrap_or_else(|| json!({"version":1}));
    if !value.is_object()
        || value
            .get("defaultStyle")
            .is_some_and(|style| !style.is_object())
    {
        return Err(weftext_core::CoreError::new(
            "styleSelection",
            UiMessage::error_api_writing_style_configuration_object_required(),
        ));
    }
    match value
        .get("defaultStyle")
        .and_then(|style| style.get("enabled"))
    {
        Some(Value::Bool(enabled)) => Ok(*enabled),
        None => Ok(false),
        Some(_) => Err(weftext_core::CoreError::new(
            "styleSelection",
            UiMessage::error_api_writing_style_boolean_required(),
        )),
    }
}

pub(crate) async fn style_environment(
    state: &AppState,
    session_id: &str,
) -> Result<StyleEnvironment, ApiError> {
    let root = location(state, session_id)?.root;
    let read_root = root.clone();
    let (captured, evidence) = capture_readonly(state, move |capture| {
        if capture.core.has_library(&read_root)? {
            let settings = capture.read_config(&read_root, ConfigRole::WorkspaceSettings)?;
            if selected_style_enabled(&settings)? {
                let document = capture.read_library_style(&read_root)?;
                if !document.installed {
                    return Err(weftext_core::CoreError::new(
                        "styleSelection",
                        UiMessage::error_api_writing_enabled_style_module_required(),
                    ));
                }
                Ok(StyleProfile::Library(document))
            } else {
                // An explicitly disabled module is not a compilation dependency.
                // The frozen selector still rejects a late opt-out result once
                // styles are enabled.
                Ok(StyleProfile::Disabled(settings))
            }
        } else {
            Ok(StyleProfile::Ordinary)
        }
    })
    .await?;
    let (style, basis, selected) = match captured {
        StyleProfile::Library(document) => {
            let module_path = Path::new(&document.path)
                .strip_prefix(&root)
                .map_err(|_| {
                    ApiError::new(
                        StatusCode::CONFLICT,
                        "styleSelection",
                        UiMessage::error_api_writing_style_outside_library(),
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let selected = json!({"resource":document.resource_ref,"path":document.path,
                "source":document.source,"enabled":document.enabled,"entryFunction":document.entry_function});
            let style =
                (document.installed && document.enabled).then_some(weftext_engine::StyleModule {
                    path: module_path,
                    source: document.source,
                    enabled: document.enabled,
                    entry_function: document.entry_function,
                });
            (
                style,
                StyleEnvironmentBasis::Module {
                    base_id: document.base_id,
                },
                selected,
            )
        }
        StyleProfile::Disabled(settings) => {
            let selected = json!({"enabled":false,"settings":settings.bytes});
            (
                None,
                StyleEnvironmentBasis::Settings {
                    path: settings.path,
                    base_id: settings.base_id,
                },
                selected,
            )
        }
        StyleProfile::Ordinary => (None, StyleEnvironmentBasis::Ordinary, Value::Null),
    };
    let profile = json!({"style":selected,"fonts":weftext_engine::bundled_font_manifest(),"compiler":"0.15.1"});
    let key = weftext_engine::source_hash(&profile.to_string());
    Ok(StyleEnvironment {
        style,
        key,
        root,
        basis,
        evidence,
    })
}

pub(crate) async fn check_style_environment(
    state: &AppState,
    session_id: &str,
    environment: &StyleEnvironment,
) -> Result<(), ApiError> {
    let root = location(state, session_id)?.root;
    if !same_path(&root, &environment.root) {
        return Err(stale_snapshot().into());
    }
    let basis = environment.basis.clone();
    let evidence = environment.evidence.clone();
    core_call(state, move |core| {
        let _evidence = evidence;
        recheck_style_environment(core, &root, &basis)
    })
    .await
}

fn recheck_style_environment(
    core: &weftext_core::Core,
    root: &Path,
    basis: &StyleEnvironmentBasis,
) -> weftext_core::Result<()> {
    match basis {
        StyleEnvironmentBasis::Module { base_id } => core.validate_library_style(root, base_id),
        StyleEnvironmentBasis::Settings { path, base_id } => {
            core.validate_snapshot(root, path, base_id)
        }
        StyleEnvironmentBasis::Ordinary => {
            if core.has_library(root)? {
                Err(weftext_core::CoreError::new(
                    "staleEnvironment",
                    UiMessage::error_api_writing_ordinary_profile_changed(),
                ))
            } else {
                Ok(())
            }
        }
    }
}

pub(crate) async fn fonts() -> ApiResult {
    let mut manifest = weftext_engine::bundled_font_manifest();
    if let Some(fonts) = manifest["fonts"].as_array_mut() {
        for font in fonts {
            if let Some(file) = font["file"].as_str() {
                font["url"] = json!(format!("/fonts/{file}"));
            }
            if let Some(file) = font["licenseFile"].as_str() {
                font["licenseUrl"] = json!(format!("/fonts/{file}"));
            }
        }
    }
    Ok(Json(manifest))
}

/// This route delivers only the fixed admitted distribution fonts. Ordinary
/// library resources use their separate Core delivery boundary.
pub(crate) fn font_asset(name: &str) -> Option<Response> {
    if name.is_empty() || name.contains(['/', '\\']) || name.starts_with('.') {
        return None;
    }
    let bytes = weftext_engine::bundled_font_asset(name)?;
    let content_type = if name.ends_with(".otf") {
        "font/otf"
    } else if name.ends_with(".ttf") {
        "font/ttf"
    } else if name.ends_with(".woff2") {
        "font/woff2"
    } else {
        "text/plain; charset=utf-8"
    };
    Some(([(header::CONTENT_TYPE, content_type)], Body::from(bytes)).into_response())
}

/// Closed validation failures shared by the HTTP and Core-call boundaries.
/// Keeping this domain separate prevents reverse conversion of arbitrary HTTP
/// details, while preserving the exact snapshot checks on both sides of work.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SnapshotError {
    Location(super::LocationError),
    SourceBudget,
    SourceHashMismatch,
    Stale,
    EditorUnavailable,
}
impl From<super::LocationError> for SnapshotError {
    fn from(error: super::LocationError) -> Self {
        Self::Location(error)
    }
}
impl SnapshotError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Location(super::LocationError::Unavailable) | Self::EditorUnavailable => {
                "internal"
            }
            Self::Location(super::LocationError::Expired) | Self::SourceBudget => "invalid_request",
            Self::SourceHashMismatch => "source_hash_mismatch",
            Self::Stale => "stale_draft",
        }
    }
    fn explanation(self) -> UiMessage {
        match self {
            Self::Location(super::LocationError::Unavailable) => {
                UiMessage::error_file_session_unavailable()
            }
            Self::Location(super::LocationError::Expired) => {
                UiMessage::error_file_session_expired()
            }
            Self::SourceBudget => UiMessage::error_api_writing_edit_source_budget(),
            Self::SourceHashMismatch => UiMessage::error_api_writing_source_hash_mismatch(),
            Self::Stale => UiMessage::error_api_writing_editor_basis_expired(),
            Self::EditorUnavailable => {
                UiMessage::error_api_writing_editor_session_temporarily_unavailable()
            }
        }
    }
    pub(crate) fn into_core(self, code: &str) -> weftext_core::CoreError {
        match self {
            Self::Location(error) => error.into_core(code),
            _ => weftext_core::CoreError::new(code, self.explanation()),
        }
    }
}
impl From<SnapshotError> for ApiError {
    fn from(error: SnapshotError) -> Self {
        match error {
            SnapshotError::Location(error) => error.into(),
            SnapshotError::SourceBudget => Self::bad(error.explanation()),
            SnapshotError::SourceHashMismatch => Self::new(
                StatusCode::CONFLICT,
                "source_hash_mismatch",
                error.explanation(),
            ),
            SnapshotError::Stale => {
                Self::new(StatusCode::CONFLICT, "stale_draft", error.explanation())
            }
            SnapshotError::EditorUnavailable => Self::internal(error.explanation()),
        }
    }
}
fn stale_snapshot() -> SnapshotError {
    SnapshotError::Stale
}

pub(crate) fn validate_source(
    source: &str,
    source_hash: Option<&str>,
) -> Result<(), SnapshotError> {
    if source.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(SnapshotError::SourceBudget);
    }
    if source_hash.is_some_and(|expected| expected != weftext_engine::source_hash(source)) {
        return Err(SnapshotError::SourceHashMismatch);
    }
    Ok(())
}

pub(crate) fn observe_version(
    versions: &mut HashMap<String, EditorVersion>,
    session_id: &str,
    source: &str,
    serial: u64,
) -> Result<(), SnapshotError> {
    if let Some(old) = versions.get(session_id) {
        if serial < old.serial || serial == old.serial && source != old.source {
            return Err(stale_snapshot());
        }
        if serial == old.serial {
            return Ok(());
        }
    }
    versions.insert(
        session_id.to_owned(),
        EditorVersion {
            source: source.to_owned(),
            serial,
        },
    );
    Ok(())
}

pub(crate) fn exact_version(
    versions: &HashMap<String, EditorVersion>,
    session_id: &str,
    source: &str,
    serial: u64,
) -> Result<(), SnapshotError> {
    versions
        .get(session_id)
        .filter(|old| old.serial == serial && old.source == source)
        .map(|_| ())
        .ok_or_else(stale_snapshot)
}

/// Register a draft before asynchronous work starts. The global HTTP boundary
/// and Core session supply access; a serial alone never grants a source writer.
pub(crate) fn observe_snapshot(
    state: &AppState,
    session_id: &str,
    source: &str,
    serial: u64,
    source_hash: Option<&str>,
) -> Result<(), SnapshotError> {
    location(state, session_id)?;
    validate_source(source, source_hash)?;
    let mut versions = state
        .editor_versions
        .lock()
        .map_err(|_| SnapshotError::EditorUnavailable)?;
    observe_version(&mut versions, session_id, source, serial)
}

/// Check again after worker latency. An earlier request cannot publish a patch
/// or writable projection after this session has supplied a newer draft.
pub(crate) fn check_snapshot(
    state: &AppState,
    session_id: &str,
    source: &str,
    serial: u64,
    source_hash: Option<&str>,
) -> Result<(), SnapshotError> {
    location(state, session_id)?;
    validate_source(source, source_hash)?;
    let versions = state
        .editor_versions
        .lock()
        .map_err(|_| SnapshotError::EditorUnavailable)?;
    exact_version(&versions, session_id, source, serial)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SnapshotRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EditRequest {
    session_id: String,
    source: String,
    serial: u64,
    source_hash: Option<String>,
    intent: weftext_engine::EditIntent,
}

async fn editor_map(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<SnapshotRequest>,
) -> ApiResult {
    observe_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let source = req.source.clone();
    let map = crate::work::spawn_blocking(move || weftext_engine::inspect_editor(&source))
        .await
        .map_err(
            |_| ApiError::internal(UiMessage::error_api_writing_editor_mapping_incomplete()),
        )?;
    check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let mut output = value(map)?;
    output["serial"] = json!(req.serial);
    Ok(Json(output))
}

async fn edit(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<EditRequest>,
) -> ApiResult {
    // Commands consume a map already observed by this session. The engine
    // reparses this same source and independently certifies the named operation.
    check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let engine_req = weftext_engine::EditRequest {
        source_hash: req
            .source_hash
            .clone()
            .unwrap_or_else(|| weftext_engine::source_hash(&req.source)),
        intent: req.intent,
    };
    let needs_module = match &engine_req.intent {
        weftext_engine::EditIntent::NativeNote { action } => {
            action.requires_frozen_module(&req.source)
        }
        weftext_engine::EditIntent::HeadingNumbering { action } => {
            action.requires_frozen_module(&req.source)
        }
        _ => false,
    };
    if needs_module {
        let loc = location(&state, &req.session_id)?;
        let environment = style_environment(&state, &req.session_id).await?;
        let original_loc = loc.clone();
        let source = req.source.clone();
        let (frozen, guards, cut_id) = core_call(&state, move |core| {
            let expected = loc
                .path
                .parent()
                .ok_or_else(|| {
                    weftext_core::CoreError::new(
                        "invalidPath",
                        UiMessage::error_api_writing_note_directory_missing(),
                    )
                })?
                .join("weftext-endnotes.typ");
            let resources = core.managed_resource_snapshot(&loc.root, &loc.path)?;
            let module = resources
                .into_iter()
                .find(|resource| same_path(Path::new(&resource.path), &expected))
                .filter(|resource| {
                    resource.identity_status == "admitted"
                        && resource.bytes == ENDNOTES_MODULE.as_bytes()
                })
                .ok_or_else(|| {
                    weftext_core::CoreError::new(
                        "endnotesModuleUnavailable",
                        UiMessage::error_api_writing_owned_endnotes_module_required(),
                    )
                })?;
            let captured = core.admitted_compile_snapshot(&loc.root, "live")?;
            let relative = expected
                .strip_prefix(&loc.root)
                .map_err(|_| {
                    weftext_core::CoreError::new(
                        "outsideRoot",
                        UiMessage::error_api_builtin_module_outside_library(),
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            if captured.files.get(&relative) != Some(&module.bytes) {
                core.release_compile_dependency_cut(&captured.snapshot.cut_id);
                return Err(weftext_core::CoreError::new(
                    "staleBase",
                    UiMessage::error_api_writing_module_changed_during_capture(),
                ));
            }
            let main_path = loc
                .path
                .strip_prefix(&loc.root)
                .map_err(|_| {
                    weftext_core::CoreError::new(
                        "outsideRoot",
                        UiMessage::error_api_writing_draft_outside_library(),
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            let mut files = captured.files;
            // The current author Draft is a separate, serial-checked input.
            // It replaces only the main file in the immutable compiler view;
            // dependency qualification keeps the original physical Core cut.
            files.insert(main_path, source.as_bytes().to_vec());
            let guards = vec![
                (module.path, module.base_id),
                (module.manifest_path, module.manifest_base_id),
            ];
            Ok((
                weftext_engine::FrozenRenderRequest {
                    source,
                    path: loc.path,
                    root: loc.root,
                    files,
                    style: captured.style,
                },
                guards,
                captured.snapshot.cut_id,
            ))
        })
        .await?;
        let cleanup_cut = cut_id.clone();
        let result = async {
            let _permit =
                state.render_gate.acquire().await.map_err(|_| {
                    ApiError::internal(UiMessage::error_compile_service_unavailable())
                })?;
            let receipt = super::notes_edit_worker(frozen, engine_req).await?;
            let guard_root = original_loc.root.clone();
            // Only the worker's actual current/proposed World reads participate.
            // Core qualifies them against the original cut, including absences.
            // Dependency paths never become part of the browser response.
            let style_evidence = environment.evidence.clone();
            let mut output = core_call(&state, move |core| {
                let _style_evidence = style_evidence;
                let dependencies = receipt
                    .dependencies
                    .iter()
                    .map(|dependency| {
                        weftext_core::CompileDependency::new(
                            &dependency.path,
                            &dependency.kind,
                            &dependency.status,
                        )
                    })
                    .collect::<weftext_core::Result<Vec<_>>>()?;
                for (path, base_id) in guards {
                    core.validate_snapshot(&guard_root, path, &base_id)?;
                }
                recheck_style_environment(core, &environment.root, &environment.basis)?;
                let witness =
                    core.take_compile_dependency_witness(&guard_root, &cut_id, &dependencies)?;
                core.with_validated_compile_dependencies(&witness, |_| Ok(receipt.edit))
            })
            .await?;
            validate_source(
                output["source"].as_str().ok_or_else(|| {
                    ApiError::internal(UiMessage::error_api_writing_endnotes_source_missing())
                })?,
                None,
            )?;
            output["serial"] = json!(req.serial);
            Ok::<_, ApiError>(output)
        }
        .await;
        let output = match result {
            Ok(output) => output,
            Err(error) => {
                // A successful take consumes the cut. All earlier error paths
                // release only retained memory; no author files are removed.
                core_call(&state, move |core| {
                    core.release_compile_dependency_cut(&cleanup_cut);
                    Ok(())
                })
                .await?;
                return Err(error);
            }
        };
        check_snapshot(
            &state,
            &req.session_id,
            &req.source,
            req.serial,
            req.source_hash.as_deref(),
        )?;
        let current = location(&state, &req.session_id)?;
        if !same_path(&current.path, &original_loc.path)
            || !same_path(&current.root, &original_loc.root)
        {
            return Err(stale_snapshot().into());
        }
        return Ok(Json(output));
    }
    let source = req.source.clone();
    let edited = crate::work::spawn_blocking(move || weftext_engine::edit(&source, &engine_req))
        .await
        .map_err(|_| ApiError::internal(UiMessage::error_api_writing_edit_incomplete()))?;
    check_snapshot(
        &state,
        &req.session_id,
        &req.source,
        req.serial,
        req.source_hash.as_deref(),
    )?;
    let edited = edited.map_err(|error| {
        let mut result =
            ApiError::from_engine(StatusCode::UNPROCESSABLE_ENTITY, "unsupported_edit", error);
        let cause = Box::new(result.clone());
        result.details = Some(Box::new(ApiErrorDetails::SourcePreserved {
            serial: req.serial,
            source_preserved: true,
            cause,
        }));
        result
    })?;
    validate_source(&edited.source, None)?;
    let mut output: Value = value(edited)?;
    output["serial"] = json!(req.serial);
    Ok(Json(output))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_snapshot_failures_keep_http_and_core_meaning_without_string_reclassification() {
        for error in [
            SnapshotError::Location(super::super::LocationError::Unavailable),
            SnapshotError::Location(super::super::LocationError::Expired),
            SnapshotError::SourceBudget,
            SnapshotError::SourceHashMismatch,
            SnapshotError::Stale,
            SnapshotError::EditorUnavailable,
        ] {
            let core = error.into_core(error.code());
            let api = ApiError::from(error);
            assert_eq!(core.code, api.code);
            assert_eq!(core.explanation, api.explanation);
            assert!(core.diagnostics.is_empty());
            assert!(api.diagnostics.is_empty());
            assert!(api.details.is_none());
            assert_eq!(
                error.into_core("callerSnapshotConflict").code,
                "callerSnapshotConflict"
            );
        }
        let exact = "same author source 😀\r\n";
        let invalid = validate_source(exact, Some("other exact hash")).unwrap_err();
        assert!(matches!(invalid, SnapshotError::SourceHashMismatch));
        assert_eq!(ApiError::from(invalid).status, StatusCode::CONFLICT);
        assert_eq!(exact, "same author source 😀\r\n");
    }

    #[test]
    fn old_results_cannot_install_after_newer_input() {
        let mut versions = HashMap::new();
        assert!(observe_version(&mut versions, "session", "旧草稿", 4).is_ok());
        assert!(observe_version(&mut versions, "session", "新草稿", 5).is_ok());
        assert!(exact_version(&versions, "session", "旧草稿", 4).is_err());
        assert!(observe_version(&mut versions, "session", "旧草稿", 4).is_err());
        assert!(exact_version(&versions, "session", "新草稿", 5).is_ok());
    }

    #[test]
    fn equal_serial_never_changes_exact_bytes() {
        let mut versions = HashMap::new();
        assert!(observe_version(&mut versions, "session", "相同串行号", 8).is_ok());
        assert!(observe_version(&mut versions, "session", "不同原文", 8).is_err());
        assert!(exact_version(&versions, "session", "相同串行号", 8).is_ok());
        // Undo may restore earlier bytes, but must have a fresh serial.
        assert!(observe_version(&mut versions, "session", "不同原文", 9).is_ok());
        assert!(observe_version(&mut versions, "session", "相同串行号", 10).is_ok());
        assert!(exact_version(&versions, "session", "相同串行号", 8).is_err());
    }

    #[test]
    fn digest_is_an_additional_exact_source_check() {
        let hash = weftext_engine::source_hash("正文");
        assert!(validate_source("正文", Some(&hash)).is_ok());
        assert!(validate_source("正文已变", Some(&hash)).is_err());
    }

    #[test]
    fn style_panel_keeps_native_expressions_and_source() {
        let source = weftext_engine::DEFAULT_LIBRARY_STYLE.to_owned();
        let document = LibraryStyleDocument {
            source: source.clone(),
            base_id: "exact-core-basis".into(),
            path: "Styles/library-style.typ".into(),
            enabled: true,
            resource_ref: Some("weftext-resource://workspace/resource".into()),
            entry_function: "library-style".into(),
            installed: true,
        };
        let result = style_state(document);
        assert!(result.is_ok());
        let state = result.ok().unwrap();
        assert_eq!(state["source"], source);
        let controls = state["controls"].as_array().unwrap();
        assert!(
            controls
                .iter()
                .all(|control| control["kind"] == "text" && control["value"].is_string())
        );
        assert!(controls.iter().any(|control| control["key"] == "text.size"
            && control["value"] == "11pt"
            && control["editable"] == true));
        assert!(
            controls
                .iter()
                .any(|control| control["key"] == "text.lang" && control["value"] == "\"zh\"")
        );
    }

    #[test]
    fn font_delivery_is_a_distribution_allowlist() {
        assert!(font_asset("../NotoSerifCJKsc-Regular.otf").is_none());
        assert!(font_asset("nested/NotoSerifCJKsc-Regular.otf").is_none());
        assert!(font_asset("manifest.json").is_none());
        assert!(font_asset("not-a-distribution-font.otf").is_none());
        assert!(font_asset("NotoSerifCJKsc-Regular.otf").is_some());
    }

    #[test]
    fn malformed_style_selector_never_becomes_an_implicit_opt_out() {
        let settings = |source: &str| ConfigDocument {
            role_ref: "weftext-config://workspace/workspace-settings".into(),
            path: ".weftext-settings.json".into(),
            bytes: Some(source.as_bytes().to_vec()),
            revision: None,
            base_id: "core-settings-basis".into(),
        };
        assert_eq!(
            selected_style_enabled(&settings(r#"{"defaultStyle":{"enabled":false}}"#)).ok(),
            Some(false)
        );
        assert_eq!(
            selected_style_enabled(&settings(r#"{"defaultStyle":{"enabled":true}}"#)).ok(),
            Some(true)
        );
        assert!(
            selected_style_enabled(&settings(r#"{"defaultStyle":{"enabled":"false"}}"#)).is_err()
        );
        assert!(
            selected_style_enabled(&settings(r#"{"defaultStyle":"unrecognized selection"}"#))
                .is_err()
        );
        assert!(selected_style_enabled(&settings("[]")).is_err());
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn native_style_commit_rejects_late_environment_without_changing_note_basis() {
        use super::super::{Location, RenderCache};
        use std::sync::{Arc, Mutex};
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("library");
        let mut core = weftext_core::Core::new(directory.path().join("private-state")).unwrap();
        core.create_library(&root).unwrap();
        let note = core.create_note_fixture(&root, &root, "Note").unwrap();
        let source = note.source.clone();
        let session_id = note.session_id.clone();
        let note_path = PathBuf::from(note.path);
        let state = AppState {
            core: Arc::new(Mutex::new(core)),
            token: Arc::from("fixture-token"),
            port: 12345,
            locations: Arc::new(Mutex::new(HashMap::from([(
                session_id.clone(),
                Location {
                    path: note_path.clone(),
                    root: root.clone(),
                },
            )]))),
            renders: Arc::new(Mutex::new(HashMap::new())),
            editor_versions: Arc::new(Mutex::new(HashMap::new())),
            exports: Arc::new(Mutex::new(crate::export_api::ExportStore::default())),
            native_status: Default::default(),
            render_gate: Arc::new(tokio::sync::Semaphore::new(1)),
            sync_remote: Arc::new(Mutex::new(crate::remote_sync::Store::default())),
            initial: None,
        };
        observe_snapshot(&state, &session_id, &source, 7, None).unwrap();
        let Json(before) = styles_read(
            State(state.clone()),
            crate::RequestJson(StyleReadRequest {
                session_id: Some(session_id.clone()),
                root: None,
            }),
        )
        .await
        .unwrap();
        let Json(v1_plan) = styles_prepare(
            State(state.clone()),
            crate::RequestJson(StylePrepareRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
                base_id: before["baseId"].as_str().unwrap().into(),
                source: None,
                changes: Some(BTreeMap::from([("text.size".into(), "12pt".into())])),
                enabled: Some(true),
            }),
        )
        .await
        .unwrap();
        let Json(v1_state) = styles_apply(
            State(state.clone()),
            crate::RequestJson(StyleApplyRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id: v1_plan["operationId"].as_str().unwrap().into(),
                plan_id: v1_plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        let v1_environment = style_environment(&state, &session_id).await.unwrap();
        let engine = weftext_engine::Engine::new().unwrap();
        let v1 = engine
            .render_with_style(
                &source,
                &note_path,
                &root,
                v1_environment.style.as_ref().unwrap(),
            )
            .unwrap();
        assert!(v1.ok, "{:?}", v1.diagnostics);
        let v1_html = v1.html.clone();
        state.renders.lock().unwrap().insert(
            session_id.clone(),
            RenderCache {
                source: source.clone(),
                serial: 7,
                output: value(v1).unwrap(),
            },
        );
        let Json(v2_plan) = styles_prepare(
            State(state.clone()),
            crate::RequestJson(StylePrepareRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
                base_id: v1_state["baseId"].as_str().unwrap().into(),
                source: None,
                changes: Some(BTreeMap::from([("text.size".into(), "18pt".into())])),
                enabled: None,
            }),
        )
        .await
        .unwrap();
        let operation_id = v2_plan["operationId"].as_str().unwrap().to_owned();
        let plan_id = v2_plan["planId"].as_str().unwrap().to_owned();
        let wrong_root = styles_apply(
            State(state.clone()),
            crate::RequestJson(StyleApplyRequest {
                session_id: None,
                root: Some(directory.path().join("other-library")),
                operation_id: operation_id.clone(),
                plan_id: plan_id.clone(),
            }),
        )
        .await;
        assert!(wrong_root.is_err());
        let _ = styles_apply(
            State(state.clone()),
            crate::RequestJson(StyleApplyRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id,
                plan_id,
            }),
        )
        .await
        .unwrap();
        // A real v1 native projection is complete but may no longer install.
        assert!(
            check_style_environment(&state, &session_id, &v1_environment)
                .await
                .is_err()
        );
        assert!(!state.renders.lock().unwrap().contains_key(&session_id));
        // Static source/field commands did not consume library styles.
        check_snapshot(&state, &session_id, &source, 7, None).unwrap();
        assert_eq!(std::fs::read_to_string(&note_path).unwrap(), source);
        let v2_environment = style_environment(&state, &session_id).await.unwrap();
        assert_ne!(v1_environment.key, v2_environment.key);
        let v2 = engine
            .render_with_style(
                &source,
                &note_path,
                &root,
                v2_environment.style.as_ref().unwrap(),
            )
            .unwrap();
        assert!(v2.ok, "{:?}", v2.diagnostics);
        assert_ne!(v1_html, v2.html);
        check_style_environment(&state, &session_id, &v2_environment)
            .await
            .unwrap();

        let Json(current) = styles_read(
            State(state.clone()),
            crate::RequestJson(StyleReadRequest {
                session_id: Some(session_id.clone()),
                root: None,
            }),
        )
        .await
        .unwrap();
        let Json(disable_plan) = styles_prepare(
            State(state.clone()),
            crate::RequestJson(StylePrepareRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id: uuid::Uuid::new_v4().to_string(),
                base_id: current["baseId"].as_str().unwrap().into(),
                source: None,
                changes: None,
                enabled: Some(false),
            }),
        )
        .await
        .unwrap();
        let _ = styles_apply(
            State(state.clone()),
            crate::RequestJson(StyleApplyRequest {
                session_id: Some(session_id.clone()),
                root: None,
                operation_id: disable_plan["operationId"].as_str().unwrap().into(),
                plan_id: disable_plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        // Simulate an external move without deleting the original. Opting out
        // excludes this unconsumed module from the compilation dependency set.
        std::fs::rename(
            root.join("Styles/library-style.typ"),
            root.join("Styles/disabled-original.typ"),
        )
        .unwrap();
        let disabled = style_environment(&state, &session_id).await.unwrap();
        assert!(disabled.style.is_none());
        check_style_environment(&state, &session_id, &disabled)
            .await
            .unwrap();
        let disabled_base = match &disabled.basis {
            StyleEnvironmentBasis::Settings { path, base_id } => (path.clone(), base_id.clone()),
            _ => panic!("the disabled selector must keep its original settings snapshot"),
        };
        let module_base = match &v1_environment.basis {
            StyleEnvironmentBasis::Module { base_id } => base_id.clone(),
            _ => panic!("the enabled selector must keep its original module basis"),
        };
        assert!(
            state
                .core
                .lock()
                .unwrap()
                .library_style_basis_retained_bytes(&module_base)
                .is_ok()
        );
        drop(v1_environment);
        drop(disabled);
        let core = state.core.lock().unwrap();
        assert_eq!(
            core.library_style_basis_retained_bytes(&module_base)
                .unwrap_err()
                .code,
            "basisRequired"
        );
        assert_eq!(
            core.validate_snapshot(&root, &disabled_base.0, &disabled_base.1)
                .unwrap_err()
                .code,
            "basisRequired"
        );
        assert_eq!(std::fs::read_to_string(&note_path).unwrap(), source);
    }
}
