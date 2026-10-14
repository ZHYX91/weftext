//! Create-only hosted DOCX import into one explicitly configured empty bundle.
//! Core retains the original upload and alone installs its complete Node tree.
use super::*;
use axum::extract::DefaultBodyLimit;
use std::{fs, path::Path};
use weftext_core::{ActorFeatureStatus, OperationIntent, OperationRequest};

const MAX_UPLOAD: usize = 16 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(900);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    resource: String,
    operation_id: String,
    bytes_base64: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    resource: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    resource: String,
    operation_id: String,
    plan_id: String,
    #[serde(default)]
    accept_losses: bool,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    actor_epoch: u64,
    expires_at_unix_ms: u64,
    config_hash: String,
    upload_digest: String,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/office/import-targets", get(targets))
        .route("/api/hosted/office/import/prepare", post(prepare))
        .route("/api/hosted/office/import/commit", post(commit))
        .route("/api/hosted/office/import/operation", post(operation))
        .route("/api/hosted/office/import/settle", post(settle))
        // A 16 MiB upload expands to ~22 MiB of base64. Other routes keep the
        // hosted service's smaller default body budget.
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}

fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "officeImportChanged",
        "导入目标、授权或原计划已变化；请保留原件并核对原操作。",
    )
}
fn core_error(error: CoreError) -> ApiError {
    // Parser diagnostics and Core previews can contain server/schema paths.
    let (code, message) = if error.code == "copyLoss" {
        ("officeImportLosses", "请明确接受本次原导入计划的转换损失。")
    } else {
        (
            "officeImportOperation",
            "原导入操作未完成或目标已变化；请保留原件并核对原操作。",
        )
    };
    ApiError::new(StatusCode::CONFLICT, code, message)
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:office-import", inner.actor_scope(actor))
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn access(inner: &HostedInner, actor: &str, resource: &str) -> Result<FileEntry, ApiError> {
    let rights = inner.rights(actor, resource)?;
    if !rights.write || !rights.office.r#import {
        return Err(unavailable());
    }
    inner
        .file(resource)
        .ok()
        .filter(|f| f.editable && f.importable)
        .ok_or_else(unavailable)
}

/// The native producer's canonical tree is NAME/document/document.typ.
/// Only operator configuration selects PARENT/NAME; no client supplies a path.
fn slot(inner: &HostedInner, file: &FileEntry) -> Result<(PathBuf, String, PathBuf), ApiError> {
    let path = Path::new(&file.path);
    if path.file_name().and_then(|p| p.to_str()) != Some("document.typ") {
        return Err(unavailable());
    }
    let document = path.parent().ok_or_else(unavailable)?;
    if document.file_name().and_then(|p| p.to_str()) != Some("document") {
        return Err(unavailable());
    }
    let bundle = document.parent().ok_or_else(unavailable)?;
    let name = bundle
        .file_name()
        .and_then(|p| p.to_str())
        .ok_or_else(unavailable)?
        .to_owned();
    let parent = inner
        .config
        .root
        .join(bundle.parent().ok_or_else(unavailable)?);
    let target = inner.config.root.join(bundle);
    Ok((parent, name, target))
}
fn available(target: &Path) -> bool {
    fs::symlink_metadata(target).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
}
fn config_hash(inner: &HostedInner, file: &FileEntry) -> Result<String, ApiError> {
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"path":file.path,"editable":file.editable,"importable":file.importable})).map_err(|_|changed())?))
}
fn safe_status(
    inner: &HostedInner,
    actor: &str,
    file: &FileEntry,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let decision = serde_json::from_value::<Decision>(status.preview["_officeImport"].clone()).ok();
    let same_basis = decision.as_ref().is_some_and(|d| {
        d.actor_epoch == inner.actor_generation(actor)
            && config_hash(inner, file).is_ok_and(|hash| hash == d.config_hash)
    });
    let mut preview = json!({});
    if same_basis {
        for key in ["summary", "losses"] {
            if let Some(value) = status.preview.get(key) {
                preview[key] = value.clone();
            }
        }
    }
    let commit_allowed = status.status == "prepared"
        && same_basis
        && decision.is_some_and(|d| d.expires_at_unix_ms > now_ms());
    Ok(
        json!({"operationId":status.operation_id,"resource":status.resource,"status":status.status,"planId":status.plan_id.filter(|_|commit_allowed),"preview":preview,"commitAllowed":commit_allowed}),
    )
}

async fn targets(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let mut targets = vec![];
            for (resource, file) in &inner.config.files {
                if access(inner, &actor, resource).is_err() {
                    continue;
                }
                let (_, _, target) = slot(inner, file)?;
                targets.push(
                    json!({"resource":resource,"title":file.title,"available":available(&target)}),
                );
            }
            Ok(json!({"targets":targets}))
        })
        .await?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;let file=access(inner,&actor,&req.resource)?;
        if req.bytes_base64.len()>MAX_UPLOAD.div_ceil(3)*4 { return Err(ApiError::bad("DOCX 原件上限为 16 MiB")); }
        let bytes=base64::engine::general_purpose::STANDARD.decode(&req.bytes_base64).map_err(|_|ApiError::bad("DOCX 原件编码无效"))?;
        if bytes.is_empty()||bytes.len()>MAX_UPLOAD {return Err(ApiError::bad("DOCX 原件须为 1 字节至 16 MiB"));}
        let upload_digest=digest(&bytes);let fingerprint=digest(&serde_json::to_vec(&json!({"resource":req.resource,"uploadDigest":upload_digest})).map_err(|_|changed())?);
        let actor_scope=scope(inner,&actor);
        if let Some(old)=inner.core.with_actor_for_root(&actor_scope, &inner.config.root,|core|core.actor_feature_retry(&req.resource,&req.operation_id,&fingerprint)).map_err(core_error)? {return safe_status(inner,&actor,&file,old);}
        let (parent,name,target)=slot(inner,&file)?;
        if !available(&target) {return Err(changed());}
        let decision=Decision {actor_epoch:inner.actor_generation(&actor),expires_at_unix_ms:now_ms().saturating_add(TTL.as_millis() as u64),config_hash:config_hash(inner,&file)?,upload_digest};
        let preview=json!({"summary":"在所选空槽位导入可编辑正文，并保留 DOCX 原件、批注和图片；转换损失须明确接受。","_officeImport":decision});
        let request=OperationRequest {operation_id:req.operation_id.clone(),root:inner.config.root.to_string_lossy().into_owned(),intent:OperationIntent::ImportDocx {parent:parent.to_string_lossy().into_owned(),name,bytes,allow_losses:true}};
        let status=inner.core.with_actor_for_root(&actor_scope, &inner.config.root,|core|core.actor_feature_prepare(&req.resource,&req.operation_id,&fingerprint,request,preview,&[])).map_err(core_error)?;
        safe_status(inner,&actor,&file,status)
    }).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;let file=access(inner,&actor,&req.resource)?;
        let status=inner.core.with_actor_for_root(&scope(inner,&actor), &inner.config.root,|core|core.actor_feature_status(&req.resource,&req.operation_id)).map_err(core_error)?;
        match status {Some(status)=>safe_status(inner,&actor,&file,status),None=>Ok(json!({"operationId":req.operation_id,"resource":req.resource,"status":"notRegistered","planId":null,"preview":{},"commitAllowed":false}))}
    }).await?))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let file = access(inner, &actor, &req.resource)?;
            let actor_scope = scope(inner, &actor);
            let old = inner
                .core
                .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
                    core.actor_feature_status(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?
                .ok_or_else(changed)?;
            if old.status == "prepared" {
                let decision: Decision =
                    serde_json::from_value(old.preview["_officeImport"].clone())
                        .map_err(|_| changed())?;
                if decision.actor_epoch != inner.actor_generation(&actor)
                    || decision.expires_at_unix_ms <= now_ms()
                    || decision.config_hash != config_hash(inner, &file)?
                    || old.plan_id.as_deref() != Some(&req.plan_id)
                {
                    return Err(changed());
                }
                let (_, _, target) = slot(inner, &file)?;
                if !available(&target) {
                    return Err(changed());
                }
            }
            let status = inner
                .core
                .with_actor_for_root(&actor_scope, &inner.config.root, |core| {
                    core.actor_feature_commit_with_losses(
                        &req.resource,
                        &req.operation_id,
                        &req.plan_id,
                        req.accept_losses,
                    )
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, &file, status)
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let file = access(inner, &actor, &req.resource)?;
            let status = inner
                .core
                .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                    core.actor_feature_settle(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, &file, status)
        })
        .await?,
    ))
}
