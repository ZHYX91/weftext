//! Same-owner local adapters only. Never mounted in the hosted actor router.
use super::{ApiResult, AppState, core_call, value};
use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use weftext_core::sync::{ObjectKey, SyncIdentity, SyncRevalidation};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/sync/binding", post(binding))
        .route("/api/sync/enable", post(enable))
        .route("/api/sync/status", post(status))
        .route("/api/sync/publish", post(publish))
        .route("/api/sync/receive", post(receive))
        .route("/api/sync/export", post(export))
        .route("/api/sync/file", post(file))
        .route("/api/sync/history", post(history))
        .route("/api/sync/adoption-cut", post(adoption_cut))
        .route("/api/sync/revalidation-cut", post(revalidation_cut))
        .route("/api/sync/revalidation-review", post(revalidation_review))
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HistoryRequest {
    identity: SyncIdentity,
    object: ObjectKey,
    after_commit: Option<String>,
    limit: Option<usize>,
}
async fn history(State(state): State<AppState>, Json(req): Json<HistoryRequest>) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.received_sync_versions(
                &req.identity,
                &req.object,
                req.after_commit.as_deref(),
                req.limit.unwrap_or(25),
            )
        })
        .await?,
    )?))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AdoptionCutRequest {
    root: String,
    identity: SyncIdentity,
    objects: BTreeSet<ObjectKey>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevalidationReviewRequest {
    root: String,
    identity: SyncIdentity,
    resolutions: Vec<SyncRevalidation>,
    blobs: BTreeMap<String, Vec<u8>>,
}
async fn revalidation_cut(
    State(state): State<AppState>,
    Json(req): Json<AdoptionCutRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            if core.sync_binding_info(&req.root)?.identity != req.identity {
                return Err(weftext_core::CoreError::new(
                    "syncIdentity",
                    "The selected replica does not match the explicit pairing identity",
                ));
            }
            core.sync_revalidation_cut(&req.root, &req.objects)
        })
        .await?,
    )?))
}
async fn revalidation_review(
    State(state): State<AppState>,
    Json(req): Json<RevalidationReviewRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            core.review_sync_revalidation(&req.root, &req.identity, &req.resolutions, &req.blobs)
        })
        .await?,
    )?))
}
struct AdoptionJsonBudget(usize);
impl std::io::Write for AdoptionJsonBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.0.saturating_add(bytes.len()) > 16 * 1024 * 1024 {
            return Err(std::io::Error::other("adoption JSON exceeds 16 MiB"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
async fn adoption_cut(
    State(state): State<AppState>,
    Json(req): Json<AdoptionCutRequest>,
) -> ApiResult {
    Ok(Json(value(
        core_call(&state, move |core| {
            let binding = core.sync_binding_info(&req.root)?;
            if binding.identity != req.identity {
                return Err(weftext_core::CoreError::new(
                    "syncIdentity",
                    "The selected replica does not match the explicit pairing identity",
                ));
            }
            let cut = core.read_sync_adoption_cut(std::path::Path::new(&req.root), &req.objects)?;
            // Count the actual wire representation without allocating a second
            // large byte array. Browser persistence can have a smaller budget.
            if serde_json::to_writer(AdoptionJsonBudget(0), &cut).is_err() {
                for change in &cut.changes {
                    if let Some(base) = &change.base_id {
                        let _ = core.take_snapshot_witness(base);
                    }
                }
                return Err(weftext_core::CoreError::new(
                    "budget",
                    "采纳预览的完整请求超过 16 MiB；请减少本次所选对象，原文件与草稿保留",
                ));
            }
            Ok(cut)
        })
        .await?,
    )?))
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
        let exact = core.read_received_sync_file(&req.identity, &req.object, &req.revision, &req.path)?;
        let bytes = &exact.bytes;
        let digest = format!("{:x}", Sha256::digest(bytes));
        // Inspection never interprets HTML, Typst, paths, or resource bytes.
        let source = (bytes.len() <= 2 * 1024 * 1024).then(|| std::str::from_utf8(bytes).ok()).flatten();
        Ok(json!({"object":exact.object,"revision":exact.object_revision,"fileRevision":exact.file_revision,"path":exact.relative_path,"digest":digest,"bytes":bytes.len(),"text":source}))
    }).await?;
    Ok(Json(result))
}
