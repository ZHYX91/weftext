//! Local timeline adapters; all authorization, exact-source reads and restore
//! decisions stay inside the original Core.
use super::{ApiResult, AppState, core_call, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use weftext_core::HistoryRestoreRequest;

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/history/list", post(list))
        .route("/api/history/read", post(read))
        .route("/api/history/compare", post(compare))
        .route("/api/history/restore", post(restore))
        .route("/api/history/status", post(status))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SessionRequest {
    session_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VersionRequest {
    session_id: String,
    version_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StatusRequest {
    operation_id: String,
}
async fn list(State(state): State<AppState>, Json(req): Json<SessionRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.list_history(&req.session_id)).await?,
    )?))
}
async fn read(State(state): State<AppState>, Json(req): Json<VersionRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.read_history(&req.session_id, &req.version_id)
        })
        .await?,
    )?))
}
async fn compare(State(state): State<AppState>, Json(req): Json<VersionRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.compare_history(&req.session_id, &req.version_id)
        })
        .await?,
    )?))
}
async fn restore(
    State(state): State<AppState>,
    Json(req): Json<HistoryRestoreRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.restore_history(req)).await?,
    )?))
}
async fn status(State(state): State<AppState>, Json(req): Json<StatusRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.history_restore_status(&req.operation_id)
        })
        .await?,
    )?))
}
