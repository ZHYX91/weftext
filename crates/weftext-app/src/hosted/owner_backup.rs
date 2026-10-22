//! Independently authorized delivery of the original Core recovery backup.
//! Neither native content export nor sync-owner status grants this capability.
use super::*;
use base64::Engine;
use weftext_core::ActorOwnerBackupStatus;
use weftext_export::recovery_backup::{MAX_BACKUP_ZIP_BYTES, encode_recovery_backup};

const RESOURCE: &str = "owner-library";
const TTL: Duration = Duration::from_secs(900);
const MAX_STORED: usize = 160 * 1024 * 1024;

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    export: bool,
    #[serde(default)]
    download: bool,
}
pub(super) fn valid_rights(rights: &Rights) -> bool {
    (!rights.export || rights.read) && (!rights.download || rights.read && rights.export)
}
pub(super) fn valid_config(config: &Config) -> bool {
    config.owner_backup_directory.as_ref().is_none_or(|p| {
        p.is_absolute()
            && !p.starts_with(&config.root)
            && !config.root.starts_with(p)
            && std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.is_symlink())
    })
}

#[derive(Default)]
pub(super) struct Store {
    jobs: HashMap<(String, String), Artifact>,
}
struct Artifact {
    actor_epoch: u64,
    config_hash: String,
    bytes: Vec<u8>,
    digest: String,
    expires: Instant,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RequestId {
    request_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    request_id: String,
    plan_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Download {
    request_id: String,
    digest: String,
    offset: usize,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    actor_epoch: u64,
    config_hash: String,
    expires_at_unix_ms: u64,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/owner-backup", get(catalog))
        .route("/api/hosted/owner-backup/prepare", post(prepare))
        .route("/api/hosted/owner-backup/commit", post(commit))
        .route("/api/hosted/owner-backup/status", post(status))
        .route("/api/hosted/owner-backup/settle", post(settle))
        .route("/api/hosted/owner-backup/download", post(download))
        .route("/api/hosted/owner-backup/close", post(close))
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:owner-backup", inner.actor_scope(actor))
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "ownerBackupChanged",
        UiMessage::error_api_hosted_owner_backup_backup_basis_changed(),
    )
}
fn core_error(_: CoreError) -> ApiError {
    // No private record path, actor name/count or foreign-library diagnostics.
    ApiError::new(
        StatusCode::CONFLICT,
        "ownerBackupUnavailable",
        UiMessage::error_api_hosted_owner_backup_backup_result_unqualified(),
    )
}
fn check_id(id: &str) -> Result<(), ApiError> {
    if weftext_data::uuid_v4(id) {
        Ok(())
    } else {
        Err(ApiError::bad(
            UiMessage::error_api_hosted_owner_backup_backup_request_id_invalid(),
        ))
    }
}
fn config_hash(inner: &HostedInner) -> Result<String, ApiError> {
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"directory":inner.config.owner_backup_directory,"deployment":inner.config.deployment})).map_err(|_| changed())?))
}
fn access(
    inner: &HostedInner,
    actor: &str,
    export: bool,
    download: bool,
) -> Result<Rights, ApiError> {
    let rights = &inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(unavailable)?
        .owner_backup;
    if inner.config.owner_backup_directory.is_none()
        || !rights.read
        || export && !rights.export
        || download && !rights.download
    {
        return Err(unavailable());
    }
    Ok(rights.clone())
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    let hash = config_hash(inner).ok();
    inner.owner_backup.jobs.retain(|(actor, _), job| {
        job.expires > Instant::now()
            && hash.as_ref() == Some(&job.config_hash)
            && inner.config.owner_backup_directory.is_some()
            && inner.policy.users.get(actor).is_some_and(|u| {
                u.enabled
                    && u.epoch == job.actor_epoch
                    && u.owner_backup.read
                    && u.owner_backup.export
            })
    });
}
fn original(
    inner: &mut HostedInner,
    actor: &str,
    request: &str,
) -> Result<Option<ActorOwnerBackupStatus>, ApiError> {
    let root = inner.config.root.clone();
    let scope = scope(inner, actor);
    inner
        .core
        .with_actor_for_root(&scope, &root, |core| {
            core.actor_owner_backup_status(RESOURCE, request)
        })
        .map_err(core_error)
}
fn safe_status(
    inner: &HostedInner,
    actor: &str,
    status: ActorOwnerBackupStatus,
) -> Result<Value, ApiError> {
    let decision = serde_json::from_value::<Decision>(
        status
            .preview
            .get("_ownerBackup")
            .cloned()
            .unwrap_or(Value::Null),
    )
    .ok();
    let hash = config_hash(inner)?;
    let same = decision
        .as_ref()
        .is_some_and(|d| d.actor_epoch == inner.actor_generation(actor) && d.config_hash == hash);
    let commit_allowed = same
        && status.status == "prepared"
        && decision
            .as_ref()
            .is_some_and(|d| d.expires_at_unix_ms > now_ms());
    let artifact = inner
        .owner_backup
        .jobs
        .get(&(actor.to_owned(), status.operation_id.clone()))
        .filter(|a| {
            same && a.expires > Instant::now()
                && a.actor_epoch == inner.actor_generation(actor)
                && a.config_hash == hash
        });
    Ok(
        json!({"requestId":status.operation_id,"status":status.status,"planId":status.plan_id.filter(|_|commit_allowed),"commitAllowed":commit_allowed,
        "summary":if same { Some(UiMessage::error_api_hosted_owner_backup_recovery_materials_summary()) } else { None },
        "artifact":artifact.map(|a|json!({"digest":a.digest,"length":a.bytes.len(),"filename":"weftext-recovery-backup.zip","contentType":"application/zip"})),
        "deliveryUnavailable":status.status=="committed" && artifact.is_none(),"automaticRestore":false}),
    )
}
fn no_record(request: &str) -> Value {
    json!({"requestId":request,"status":"notRegistered","commitAllowed":false,"automaticRestore":false})
}

async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let rights = access(inner, &actor, false, false).ok();
        Ok(json!({"enabled":rights.is_some(),"rights":rights,"scope":"completeSelectedLibraryRecoveryMaterials","automaticRestore":false}))
    }).await?))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        check_id(&req.request_id)?;
        let actor = inner.actor(&auth)?;
        access(inner, &actor, true, false)?;
        let root = inner.config.root.clone();
        let actor_scope = scope(inner, &actor);
        let hash = config_hash(inner)?;
        let fingerprint = digest(&serde_json::to_vec(&json!({"kind":"ownerBackup","config":hash})).map_err(|_|changed())?);
        if let Some(status) = inner.core.with_actor_for_root(&actor_scope, &root, |core| core.actor_owner_backup_retry(RESOURCE, &req.request_id, &fingerprint)).map_err(core_error)? {
            return safe_status(inner, &actor, status);
        }
        let directory = inner.config.owner_backup_directory.as_ref().ok_or_else(unavailable)?;
        let destination = directory.join(format!("{}-{}",digest(actor_scope.as_bytes()),req.request_id));
        let preview = json!({"_ownerBackup":Decision{actor_epoch:inner.actor_generation(&actor),config_hash:hash,expires_at_unix_ms:now_ms().saturating_add(TTL.as_millis() as u64)}});
        let status = inner.core.with_actor_for_root(&actor_scope, &root, |core| core.actor_owner_backup_prepare(RESOURCE, &req.request_id, &fingerprint, &destination, preview)).map_err(core_error)?;
        safe_status(inner, &actor, status)
    }).await?))
}
async fn status(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            check_id(&req.request_id)?;
            let actor = inner.actor(&auth)?;
            access(inner, &actor, false, false)?;
            expire_revoked(inner);
            match original(inner, &actor, &req.request_id)? {
                Some(status) => safe_status(inner, &actor, status),
                None => Ok(no_record(&req.request_id)),
            }
        })
        .await?,
    ))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            check_id(&req.request_id)?;
            let actor = inner.actor(&auth)?;
            access(inner, &actor, true, false)?;
            expire_revoked(inner);
            let status = original(inner, &actor, &req.request_id)?.ok_or_else(changed)?;
            if status.status == "committed" {
                return safe_status(inner, &actor, status);
            }
            let view = safe_status(inner, &actor, status)?;
            if view["commitAllowed"] != true
                || view["planId"].as_str() != Some(req.plan_id.as_str())
            {
                return Err(changed());
            }
            let used = inner
                .owner_backup
                .jobs
                .values()
                .map(|a| a.bytes.len())
                .sum::<usize>();
            if inner.owner_backup.jobs.len() >= 8
                || used.saturating_add(MAX_BACKUP_ZIP_BYTES) > MAX_STORED
            {
                return Err(ApiError::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "ownerBackupBudget",
                    UiMessage::error_api_hosted_owner_backup_backup_delivery_cache_capacity(),
                ));
            }
            let root = inner.config.root.clone();
            let actor_scope = scope(inner, &actor);
            // Verify and encode the very same original Plan before its first
            // backup effect. No live-tree read and no post-commit conversion
            // failure can turn an accepted Plan into an undeliverable backup.
            let bytes = inner
                .core
                .with_actor_for_root(&actor_scope, &root, |core| {
                    let original = core.actor_owner_backup_preflight_artifact(
                        RESOURCE,
                        &req.request_id,
                        &req.plan_id,
                    )?;
                    encode_recovery_backup(
                        &original.directories,
                        original
                            .files
                            .iter()
                            .map(|(path, bytes)| (path.as_str(), *bytes)),
                    )
                    .map_err(|_| {
                        CoreError::new(
                            "backupArtifact",
                            UiMessage::error_api_hosted_owner_backup_backup_delivery_encoding_failed(),
                        )
                    })
                })
                .map_err(core_error)?;
            let status = inner
                .core
                .with_actor_for_root(&actor_scope, &root, |core| {
                    core.actor_owner_backup_commit(RESOURCE, &req.request_id, &req.plan_id)
                })
                .map_err(core_error)?;
            if status.status == "committed" {
                inner.owner_backup.jobs.insert(
                    (actor.clone(), req.request_id),
                    Artifact {
                        actor_epoch: inner.actor_generation(&actor),
                        config_hash: config_hash(inner)?,
                        digest: digest(&bytes),
                        bytes,
                        expires: Instant::now() + TTL,
                    },
                );
            }
            safe_status(inner, &actor, status)
        })
        .await?,
    ))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            check_id(&req.request_id)?;
            let actor = inner.actor(&auth)?;
            access(inner, &actor, true, false)?;
            let root = inner.config.root.clone();
            let actor_scope = scope(inner, &actor);
            let status = inner
                .core
                .with_actor_for_root(&actor_scope, &root, |core| {
                    core.actor_owner_backup_settle(RESOURCE, &req.request_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, status)
        })
        .await?,
    ))
}
async fn download(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Download>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        check_id(&req.request_id)?;let actor=inner.actor(&auth)?;access(inner,&actor,true,true)?;expire_revoked(inner);
        let status=original(inner,&actor,&req.request_id)?.ok_or_else(unavailable)?;
        if status.status!="committed" {return Err(unavailable());}
        let view=safe_status(inner,&actor,status)?;
        if view["artifact"]["digest"].as_str()!=Some(req.digest.as_str()) {return Err(unavailable());}
        let artifact=inner.owner_backup.jobs.get(&(actor,req.request_id.clone())).ok_or_else(unavailable)?;
        if req.offset>artifact.bytes.len() {return Err(ApiError::bad(UiMessage::error_api_hosted_owner_backup_backup_download_offset_invalid()));}
        let end=req.offset.saturating_add(64*1024).min(artifact.bytes.len());
        Ok(json!({"requestId":req.request_id,"digest":artifact.digest,"offset":req.offset,"nextOffset":end,"length":artifact.bytes.len(),"done":end==artifact.bytes.len(),"base64":base64::engine::general_purpose::STANDARD.encode(&artifact.bytes[req.offset..end])}))
    }).await?))
}
async fn close(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<RequestId>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        check_id(&req.request_id)?;let actor=inner.actor(&auth)?;access(inner,&actor,false,false)?;
        inner.owner_backup.jobs.remove(&(actor,req.request_id.clone()));
        Ok(json!({"requestId":req.request_id,"delivery":"closed","originalOperationRetained":true}))
    }).await?))
}
