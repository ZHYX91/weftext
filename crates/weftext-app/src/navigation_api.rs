//! Source-independent navigation adapters. Preview and commit share Core's
//! original operation ledger with every other author action.
use super::{ApiResult, AppState, core_call, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/navigation/capture", post(capture))
        .route("/api/navigation/resolve", post(resolve))
        .route("/api/navigation/admit-directory", post(admit_directory))
        .route("/api/navigation/tags", post(tags))
        .route("/api/navigation/create-tag", post(create_tag))
        .route("/api/navigation/rename-tag", post(rename_tag))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CaptureRequest {
    root: String,
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ResolveRequest {
    root: String,
    target_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdmitRequest {
    root: String,
    path: String,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TagsRequest {
    root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateTagRequest {
    root: String,
    display_name: String,
    base_id: String,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RenameTagRequest {
    root: String,
    tag_ref: String,
    display_name: String,
    base_id: String,
    operation_id: String,
}

async fn capture(State(state): State<AppState>, Json(req): Json<CaptureRequest>) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.capture_navigation_target(req.root, req.path)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn resolve(State(state): State<AppState>, Json(req): Json<ResolveRequest>) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.resolve_navigation_target(req.root, &req.target_ref)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn admit_directory(
    State(state): State<AppState>,
    Json(req): Json<AdmitRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_directory_admission(req.root, req.path, &req.operation_id)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn tags(State(state): State<AppState>, Json(req): Json<TagsRequest>) -> ApiResult {
    let result = core_call(&state, move |core| core.read_tag_registry(req.root)).await?;
    Ok(Json(value(result)?))
}

async fn create_tag(State(state): State<AppState>, Json(req): Json<CreateTagRequest>) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_create_tag(req.root, &req.display_name, &req.base_id, &req.operation_id)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn rename_tag(State(state): State<AppState>, Json(req): Json<RenameTagRequest>) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_rename_tag(
            req.root,
            &req.tag_ref,
            &req.display_name,
            &req.base_id,
            &req.operation_id,
        )
    })
    .await?;
    Ok(Json(value(result)?))
}
