//! Processing defaults share the existing workspace-settings object and Core
//! operation, history and synchronization boundaries. They do not set text locale.
use super::{ApiError, ApiResult, AppState, core_call, location, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use weftext_core::{
    ActorFeatureStatus, ConfigRole, Core, CoreError, FileChange, OperationIntent, OperationRequest,
};
use weftext_knowledge::ProcessingDocument;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/processing/read", post(read))
        .route("/api/processing/prepare", post(prepare))
        .route("/api/processing/status", post(status))
        .route("/api/processing/settle", post(settle))
        .route("/api/processing/apply", post(apply))
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LibraryRequest {
    session_id: Option<String>,
    root: Option<PathBuf>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PrepareRequest {
    session_id: Option<String>,
    root: Option<PathBuf>,
    operation_id: String,
    base_id: String,
    before_source: Option<String>,
    processing: Option<ProcessingDocument>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    request: PrepareRequest,
    plan_id: String,
}

fn selected_root(state: &AppState, request: LibraryRequest) -> Result<PathBuf, ApiError> {
    if let Some(session) = request.session_id {
        let actual = location(state, &session)?.root;
        if request.root.as_ref().is_some_and(|root| {
            !root
                .to_string_lossy()
                .eq_ignore_ascii_case(&actual.to_string_lossy())
        }) {
            return Err(ApiError::bad("处理设置与当前资料库会话不一致"));
        }
        Ok(actual)
    } else {
        request.root.ok_or_else(|| ApiError::bad("请先选择资料库"))
    }
}

/// Also used by hosted settings validation. Unknown settings members stay intact.
pub(crate) fn validate_settings(settings: &Value) -> Result<(), ApiError> {
    if !settings.is_object() || settings["version"] != 1 {
        return Err(ApiError::bad("库设置须为原生 version:1 JSON 对象"));
    }
    if let Some(processing) = settings.get("processing") {
        let document: ProcessingDocument = serde_json::from_value(processing.clone())
            .map_err(|_| ApiError::bad("处理默认或词表格式无效；原输入保留"))?;
        document
            .validate()
            .map_err(|error| ApiError::bad(error.to_string()))?;
    }
    Ok(())
}

fn changed_settings(
    before: Option<&str>,
    processing: Option<&ProcessingDocument>,
) -> Result<Vec<u8>, ApiError> {
    if before.is_some_and(|source| source.len() > weftext_core::MAX_SOURCE_BYTES) {
        return Err(ApiError::bad("原设置超过 2 MiB 预算"));
    }
    let mut settings: Value = match before {
        Some(source) => serde_json::from_str(source)
            .map_err(|_| ApiError::bad("原库设置不是有效 JSON；未覆盖原件"))?,
        None => json!({"version":1}),
    };
    // Preserve an unrecognized existing processing object when reading it, but
    // replacing/resetting it is an explicit proposal against these exact bytes.
    if !settings.is_object() || settings["version"] != 1 {
        return Err(ApiError::bad("原库设置版本或类型不受支持；未覆盖原件"));
    }
    if let Some(document) = processing {
        document
            .validate()
            .map_err(|error| ApiError::bad(error.to_string()))?;
        settings["processing"] = value(document)?;
    } else {
        settings
            .as_object_mut()
            .expect("checked object")
            .remove("processing");
    }
    validate_settings(&settings)?;
    let bytes =
        serde_json::to_vec_pretty(&settings).map_err(|_| ApiError::internal("处理设置无法编码"))?;
    if bytes.len() > weftext_core::MAX_SOURCE_BYTES {
        return Err(ApiError::bad("处理设置超过 2 MiB 预算；原输入保留"));
    }
    Ok(bytes)
}

async fn read(State(state): State<AppState>, Json(request): Json<LibraryRequest>) -> ApiResult {
    let root = selected_root(&state, request)?;
    let document = core_call(&state, move |core| {
        core.read_config(root, ConfigRole::WorkspaceSettings)
    })
    .await?;
    let source = document
        .bytes
        .as_ref()
        .map(|bytes| String::from_utf8(bytes.clone()))
        .transpose()
        .map_err(|_| ApiError::bad("库设置不是可读 UTF-8；原件保留"))?;
    let settings = source
        .as_ref()
        .map(|source| serde_json::from_str::<Value>(source))
        .transpose()
        .map_err(|_| ApiError::bad("库设置不是有效 JSON；原件保留"))?
        .unwrap_or_else(|| json!({"version":1}));
    let warning = validate_settings(&settings)
        .err()
        .map(|error| error.message);
    Ok(Json(
        json!({"baseId":document.base_id,"roleRef":document.role_ref,
        "beforeSource":source,"revision":document.revision,"processing":settings.get("processing"),
        "warning":warning,"source":"shared-library-suggestion"}),
    ))
}

const PROCESSING_ACTOR: &str = "local:processing-settings";
const PROCESSING_RESOURCE: &str = "library.processing";

fn fingerprint(root: &Path, request: &PrepareRequest) -> Result<String, ApiError> {
    let frozen = json!({"root":root.to_string_lossy(),"baseId":request.base_id,"beforeSource":request.before_source,"processing":request.processing});
    let bytes = serde_json::to_vec(&frozen).map_err(|_| ApiError::bad("原处理设置无法编码"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}
fn feature_plan(
    core: &mut Core,
    root: &Path,
    request: &PrepareRequest,
    after: Vec<u8>,
    fingerprint: &str,
    existing_only: bool,
) -> Result<Option<ActorFeatureStatus>, CoreError> {
    core.with_actor_for_root(PROCESSING_ACTOR, root, |core| {
        if let Some(original) = core.actor_feature_retry(PROCESSING_RESOURCE, &request.operation_id, fingerprint)? { return Ok(Some(original)); }
        if existing_only { return Ok(None); }
        let path = root.join(".weftext-settings.json");
        core.validate_snapshot(root, &path, &request.base_id)?;
        let preview = json!({"beforeSource":request.before_source,"afterSource":String::from_utf8_lossy(&after),"summary":"更新同一库设置中的共享处理建议与词表；文档和个人偏好不变。"});
        let operation = OperationRequest { operation_id: request.operation_id.clone(), root: root.to_string_lossy().into_owned(), intent: OperationIntent::UpdateFiles { changes: vec![FileChange { path: path.to_string_lossy().into_owned(), base_id: Some(request.base_id.clone()), before: request.before_source.as_ref().map(|s| s.as_bytes().to_vec()), after }] } };
        core.actor_feature_prepare(PROCESSING_RESOURCE, &request.operation_id, fingerprint, operation, preview, &[]).map(Some)
    })
}
async fn original_plan(
    state: &AppState,
    request: PrepareRequest,
    existing_only: bool,
) -> Result<Value, ApiError> {
    let root = selected_root(
        state,
        LibraryRequest {
            session_id: request.session_id.clone(),
            root: request.root.clone(),
        },
    )?;
    let after = changed_settings(
        request.before_source.as_deref(),
        request.processing.as_ref(),
    )?;
    let fingerprint = fingerprint(&root, &request)?;
    let operation_id = request.operation_id.clone();
    let preview = core_call(state, move |core| {
        feature_plan(core, &root, &request, after, &fingerprint, existing_only)
    })
    .await?;
    match preview {
        Some(preview) => value(preview),
        None => Ok(json!({"operationId":operation_id,"planId":null,"status":"notRegistered"})),
    }
}

async fn prepare(State(state): State<AppState>, Json(request): Json<PrepareRequest>) -> ApiResult {
    Ok(Json(original_plan(&state, request, false).await?))
}

async fn status(State(state): State<AppState>, Json(request): Json<PrepareRequest>) -> ApiResult {
    Ok(Json(original_plan(&state, request, true).await?))
}

async fn apply(State(state): State<AppState>, Json(request): Json<ApplyRequest>) -> ApiResult {
    let root = selected_root(
        &state,
        LibraryRequest {
            session_id: request.request.session_id.clone(),
            root: request.request.root.clone(),
        },
    )?;
    let fingerprint = fingerprint(&root, &request.request)?;
    let receipt = core_call(&state, move |core| {
        core.with_actor_for_root(PROCESSING_ACTOR, &root, |core| {
            let original = core
                .actor_feature_retry(
                    PROCESSING_RESOURCE,
                    &request.request.operation_id,
                    &fingerprint,
                )?
                .ok_or_else(|| CoreError::new("operationMissing", "请先查询原处理设置操作"))?;
            if original.plan_id.as_deref() != Some(request.plan_id.as_str()) {
                return Err(CoreError::new(
                    "operationConflict",
                    "处理设置计划与原操作不一致",
                ));
            }
            core.actor_feature_commit(
                PROCESSING_RESOURCE,
                &request.request.operation_id,
                &request.plan_id,
            )
        })
    })
    .await?;
    Ok(Json(value(receipt)?))
}

async fn settle(State(state): State<AppState>, Json(request): Json<PrepareRequest>) -> ApiResult {
    let root = selected_root(
        &state,
        LibraryRequest {
            session_id: request.session_id.clone(),
            root: request.root.clone(),
        },
    )?;
    let fingerprint = fingerprint(&root, &request)?;
    let receipt = core_call(&state, move |core| {
        core.with_actor_for_root(PROCESSING_ACTOR, &root, |core| {
            // A known original must match its exact frozen effect. For an unknown
            // ID Core persists a cancellation fence before reporting cancellation.
            core.actor_feature_retry(PROCESSING_RESOURCE, &request.operation_id, &fingerprint)?;
            core.actor_feature_settle(PROCESSING_RESOURCE, &request.operation_id)
        })
    })
    .await?;
    Ok(Json(value(receipt)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_preserves_style_and_unknown_members() {
        let before = r#"{"version":1,"defaultStyle":{"enabled":false},"custom":[1,"原文"],"processing":{"unknown":true}}"#;
        let after: Value =
            serde_json::from_slice(&changed_settings(Some(before), None).unwrap()).unwrap();
        assert_eq!(after["defaultStyle"], json!({"enabled":false}));
        assert_eq!(after["custom"], json!([1, "原文"]));
        assert!(after.get("processing").is_none());
    }

    #[test]
    fn malformed_settings_are_never_replaced() {
        for source in ["[]", "not json", r#"{"version":2}"#] {
            assert!(changed_settings(Some(source), None).is_err());
        }
        assert!(
            validate_settings(&json!({"version":1,"processing":{"schemaVersion":999}})).is_err()
        );
    }

    fn fixture() -> (tempfile::TempDir, Core, PathBuf, PrepareRequest) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("private")).unwrap();
        core.create_library(&root).unwrap();
        let document = core
            .read_config(&root, ConfigRole::WorkspaceSettings)
            .unwrap();
        let request = PrepareRequest {
            session_id: None,
            root: Some(root.clone()),
            operation_id: uuid::Uuid::new_v4().to_string(),
            base_id: document.base_id,
            before_source: document
                .bytes
                .map(|bytes| String::from_utf8(bytes).unwrap()),
            processing: Some(ProcessingDocument {
                schema_version: 1,
                defaults: Default::default(),
                rules: vec![],
                scoped_allowlist: vec![],
            }),
        };
        (temp, core, root, request)
    }
    #[test]
    fn settling_unknown_id_persists_fence_against_delayed_prepare_after_restart() {
        let (temp, mut core, root, request) = fixture();
        let ended = core
            .with_actor_for_root(PROCESSING_ACTOR, &root, |core| {
                core.actor_feature_settle(PROCESSING_RESOURCE, &request.operation_id)
            })
            .unwrap();
        assert_eq!(ended.status, "cancelled");
        drop(core);
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let after = changed_settings(
            request.before_source.as_deref(),
            request.processing.as_ref(),
        )
        .unwrap();
        let late = feature_plan(
            &mut core,
            &root,
            &request,
            after,
            &fingerprint(&root, &request).unwrap(),
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(late.status, "cancelled");
        assert!(!root.join(".weftext-settings.json").exists());
    }
    #[test]
    fn cancelling_prepared_proposal_prevents_commit_and_keeps_author_settings_absent() {
        let (_temp, mut core, root, request) = fixture();
        let after = changed_settings(
            request.before_source.as_deref(),
            request.processing.as_ref(),
        )
        .unwrap();
        let prepared = feature_plan(
            &mut core,
            &root,
            &request,
            after,
            &fingerprint(&root, &request).unwrap(),
            false,
        )
        .unwrap()
        .unwrap();
        assert_eq!(prepared.status, "prepared");
        let ended = core
            .with_actor_for_root(PROCESSING_ACTOR, &root, |core| {
                core.actor_feature_settle(PROCESSING_RESOURCE, &request.operation_id)?;
                core.actor_feature_commit(
                    PROCESSING_RESOURCE,
                    &request.operation_id,
                    prepared.plan_id.as_deref().unwrap(),
                )
            })
            .unwrap();
        assert_eq!(ended.status, "cancelled");
        assert!(!root.join(".weftext-settings.json").exists());
    }
    #[test]
    fn original_effect_retries_survive_restart_and_reject_changed_payload() {
        let (temp, mut core, root, request) = fixture();
        let after = changed_settings(
            request.before_source.as_deref(),
            request.processing.as_ref(),
        )
        .unwrap();
        let prepared = feature_plan(
            &mut core,
            &root,
            &request,
            after.clone(),
            &fingerprint(&root, &request).unwrap(),
            false,
        )
        .unwrap()
        .unwrap();
        let committed = core
            .with_actor_for_root(PROCESSING_ACTOR, &root, |core| {
                core.actor_feature_commit(
                    PROCESSING_RESOURCE,
                    &request.operation_id,
                    prepared.plan_id.as_deref().unwrap(),
                )
            })
            .unwrap();
        assert_eq!(committed.status, "committed");
        drop(core);
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let replay = feature_plan(
            &mut core,
            &root,
            &request,
            after,
            &fingerprint(&root, &request).unwrap(),
            true,
        )
        .unwrap()
        .unwrap();
        assert_eq!(replay.status, "committed");
        assert_eq!(replay.plan_id, prepared.plan_id);
        let mut changed = request.clone();
        changed.processing.as_mut().unwrap().defaults.version = "2".into();
        let after = changed_settings(
            changed.before_source.as_deref(),
            changed.processing.as_ref(),
        )
        .unwrap();
        assert!(
            feature_plan(
                &mut core,
                &root,
                &changed,
                after,
                &fingerprint(&root, &changed).unwrap(),
                false
            )
            .is_err()
        );
    }
}
