//! Source-independent navigation adapters. Preview and commit share Core's
//! original operation ledger with every other author action.
use super::{ApiResult, AppState, core_call, value};
use crate::RequestJson;
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/navigation/capture", post(capture))
        .route("/api/navigation/resolve", post(resolve))
        .route("/api/navigation/order/read", post(order_read))
        .route("/api/navigation/order/prepare", post(order_prepare))
        .route("/api/navigation/order/cancel", post(order_cancel))
        .route("/api/navigation/admit-directory", post(admit_directory))
        .route("/api/navigation/tags", post(tags))
        .route("/api/navigation/create-tag", post(create_tag))
        .route("/api/navigation/rename-tag", post(rename_tag))
        .route("/api/navigation/move-tag", post(move_tag))
        .route("/api/navigation/cancel", post(cancel))
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
struct OrderReadRequest {
    root: String,
    parent_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OrderPrepareRequest {
    root: String,
    parent_ref: String,
    children: Vec<String>,
    base_id: String,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OrderCancelRequest {
    root: String,
    operation_id: String,
    plan_id: String,
}

async fn order_cancel(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<OrderCancelRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.cancel_navigation_reorder(req.root, &req.operation_id, &req.plan_id)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn order_read(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<OrderReadRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.read_navigation_order(req.root, &req.parent_ref)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn order_prepare(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<OrderPrepareRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_navigation_reorder(
            req.root,
            &req.parent_ref,
            &req.children,
            &req.base_id,
            &req.operation_id,
        )
    })
    .await?;
    Ok(Json(value(result)?))
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
struct CancelRequest {
    root: String,
    operation_id: String,
    plan_id: Option<String>,
}

async fn cancel(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CancelRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.cancel_navigation_attempt(req.root, &req.operation_id, req.plan_id.as_deref())
        })
        .await?,
    )?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CreateTagRequest {
    root: String,
    display_name: String,
    #[serde(deserialize_with = "weftext_data::deserialize_required_tag_parent")]
    parent_ref: Option<String>,
    assignment: Option<weftext_core::TagAssignment>,
    base_id: String,
    operation_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MoveTagRequest {
    root: String,
    tag_ref: String,
    #[serde(deserialize_with = "weftext_data::deserialize_required_tag_parent")]
    parent_ref: Option<String>,
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

async fn capture(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CaptureRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.capture_navigation_target(req.root, req.path)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn resolve(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<ResolveRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.resolve_navigation_target(req.root, &req.target_ref)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn admit_directory(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<AdmitRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_directory_admission(req.root, req.path, &req.operation_id)
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn tags(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<TagsRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| core.read_tag_registry(req.root)).await?;
    Ok(Json(value(result)?))
}

async fn create_tag(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<CreateTagRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_create_tag_with_assignment(
            req.root,
            &req.display_name,
            req.parent_ref.as_deref(),
            &req.base_id,
            req.assignment,
            &req.operation_id,
        )
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn move_tag(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<MoveTagRequest>,
) -> ApiResult {
    let result = core_call(&state, move |core| {
        core.prepare_move_tag(
            req.root,
            &req.tag_ref,
            req.parent_ref.as_deref(),
            &req.base_id,
            &req.operation_id,
        )
    })
    .await?;
    Ok(Json(value(result)?))
}

async fn rename_tag(
    State(state): State<AppState>,
    RequestJson(req): RequestJson<RenameTagRequest>,
) -> ApiResult {
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
