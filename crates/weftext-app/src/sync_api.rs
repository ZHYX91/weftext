//! Same-owner local adapters only. Never mounted in the hosted actor router.
use super::{ApiResult, AppState, core_call, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeSet;
use weftext_core::sync::{ObjectKey, SyncIdentity};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sync/binding", post(binding))
        .route("/api/sync/enable", post(enable))
        .route("/api/sync/status", post(status))
        .route("/api/sync/publish", post(publish))
        .route("/api/sync/receive", post(receive))
        .route("/api/sync/export", post(export))
        .route("/api/sync/file", post(file))
        .route("/api/sync/cancel-prepared", post(cancel))
        .route("/api/sync/operation", post(operation))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OperationLookup {
    operation_id: String,
}
async fn operation(State(state): State<AppState>, Json(req): Json<OperationLookup>) -> ApiResult {
    Ok(Json(
        core_call(&state, move |core| {
            let request = core.sync_operation_request(&req.operation_id)?;
            let plan = core.operation_details(&req.operation_id)?;
            Ok(json!({"request":request,"plan":plan}))
        })
        .await?,
    ))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelRequest {
    operation_id: String,
    plan_id: String,
}
async fn cancel(State(state): State<AppState>, Json(req): Json<CancelRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.cancel_prepared_sync(&req.operation_id, &req.plan_id)
        })
        .await?,
    )?))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RootRequest {
    root: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EnableRequest {
    root: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct IdentityRequest {
    identity: SyncIdentity,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReceiveRequest {
    identity: SyncIdentity,
    directory: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExportRequest {
    root: String,
    directory: String,
    objects: BTreeSet<ObjectKey>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileRequest {
    identity: SyncIdentity,
    object: ObjectKey,
    revision: String,
    path: String,
}
async fn binding(State(state): State<AppState>, Json(req): Json<RootRequest>) -> ApiResult {
    let result = core_call(&state, move |core| match core.sync_binding_info(req.root) {
        Ok(binding) => Ok(Some(binding)),
        Err(error) if error.code == "syncNotEnabled" => Ok(None),
        Err(error) => Err(error),
    })
    .await?;
    Ok(Json(value(result)?))
}
async fn enable(State(state): State<AppState>, Json(req): Json<EnableRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.enable_sync(req.root, &req.operation_id)
        })
        .await?,
    )?))
}
async fn status(State(state): State<AppState>, Json(req): Json<IdentityRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.sync_status(&req.identity)).await?,
    )?))
}
async fn publish(State(state): State<AppState>, Json(req): Json<RootRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| core.publish_sync(req.root)).await?,
    )?))
}
async fn receive(State(state): State<AppState>, Json(req): Json<ReceiveRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.read_sync_directory(&req.identity, req.directory)
        })
        .await?,
    )?))
}
async fn export(State(state): State<AppState>, Json(req): Json<ExportRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.write_sync_directory(req.root, req.directory, &req.objects)
        })
        .await?,
    )?))
}
async fn file(State(state): State<AppState>, Json(req): Json<FileRequest>) -> ApiResult {
    let result = core_call(&state, move |core| {
        let graph = core.read_sync_history(&req.identity)?;
        let objects = graph.objects()?;
        let version = objects.get(&req.object)
            .and_then(|view| view.versions.iter().chain(&view.common_bases).find(|v| v.revision == req.revision))
            .ok_or_else(|| weftext_core::CoreError::new("syncVersionMissing", "Refresh and select an exact received version"))?;
        let digest = version.state.files.get(&req.path)
            .ok_or_else(|| weftext_core::CoreError::new("syncFileMissing", "File is not part of the selected version"))?;
        let bytes = graph.blob(digest)
            .ok_or_else(|| weftext_core::CoreError::new("syncBlobMissing", "Selected file bytes have not arrived"))?;
        // Inspection never interprets HTML, Typst, paths, or resource bytes.
        let source = (bytes.len() <= 2 * 1024 * 1024).then(|| std::str::from_utf8(bytes).ok()).flatten();
        Ok(json!({"object":req.object,"revision":req.revision,"path":req.path,"digest":digest,"bytes":bytes.len(),"text":source}))
    }).await?;
    Ok(Json(result))
}
