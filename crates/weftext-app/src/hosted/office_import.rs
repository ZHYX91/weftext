//! Create-only hosted DOCX import into one explicitly configured empty bundle.
//! Core retains the original upload and alone installs its complete Node tree.
use super::*;
use axum::extract::DefaultBodyLimit;
use std::{
    fs,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};
use weftext_core::{ActorFeatureStatus, OperationIntent, OperationRequest};

const MAX_UPLOAD: usize = 16 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(900);

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    resource: String,
    operation_id: String,
    bytes_base64: String,
    #[serde(default = "docx_format")]
    format: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    legacy_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    docx_options: Option<weftext_export::docx::DocxImportOptions>,
}
fn docx_format() -> String {
    "docx".into()
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    resource: String,
    operation_id: String,
    /// Only an unbound, never-started operation may use this format to end a
    /// partial private claim. Existing Core/claim formats remain authoritative.
    #[serde(default)]
    format: Option<String>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    actor_epoch: u64,
    expires_at_unix_ms: u64,
    config_hash: String,
    upload_digest: String,
    #[serde(default = "docx_format")]
    format: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inspect {
    resource: String,
    format: String,
    bytes_base64: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ImportClaim {
    schema_version: u32,
    fingerprint: String,
    decision: Decision,
    resource: String,
    operation_id: String,
    format: String,
    legacy_backend: Option<String>,
    docx_options: Option<weftext_export::docx::DocxImportOptions>,
    /// Missing means the original nested layout. Never move an old once-only
    /// request to a new execution directory and accidentally run it again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    conversion_layout: Option<String>,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/office/import-targets", get(targets))
        .route("/api/hosted/office/import/prepare", post(prepare))
        .route("/api/hosted/office/import/commit", post(commit))
        .route("/api/hosted/office/import/operation", post(operation))
        .route("/api/hosted/office/import/settle", post(settle))
        .route("/api/hosted/office/import/inspect", post(inspect))
        .route("/api/hosted/office/import/cancel", post(cancel))
        // A 16 MiB upload expands to ~22 MiB of base64. Other routes keep the
        // hosted service's smaller default body budget.
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}

fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "officeImportChanged",
        UiMessage::error_api_hosted_office_import_word_import_basis_changed(),
    )
}
fn core_error(error: CoreError) -> ApiError {
    // Parser diagnostics and Core previews can contain server/schema paths.
    let code = if error.code == "copyLoss" {
        "officeImportLosses"
    } else {
        "officeImportOperation"
    };
    ApiError::new(
        StatusCode::CONFLICT,
        code,
        UiMessage::error_api_hosted_office_import_original_operation_unavailable(),
    )
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
    if !rights.write
        || !(rights.office.r#import || rights.office.doc.r#import || rights.office.wps.r#import)
    {
        return Err(unavailable());
    }
    inner
        .file(resource)
        .ok()
        .filter(|f| f.editable && f.importable)
        .ok_or_else(unavailable)
}
fn access_format(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
    format: &str,
) -> Result<FileEntry, ApiError> {
    let file = access(inner, actor, resource)?;
    let rights = inner.rights(actor, resource)?;
    let granted = match format {
        "docx" => rights.office.r#import,
        "doc" => rights.office.doc.r#import,
        "wps" => rights.office.wps.r#import,
        _ => false,
    };
    if !granted {
        return Err(unavailable());
    }
    Ok(file)
}
fn upload(encoded: &str) -> Result<Vec<u8>, ApiError> {
    if encoded.len() > MAX_UPLOAD.div_ceil(3) * 4 {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_word_original_size_limit(),
        ));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            ApiError::bad(
                UiMessage::error_api_hosted_office_import_word_original_encoding_invalid(),
            )
        })?;
    if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_word_original_size_range(),
        ));
    }
    Ok(bytes)
}
fn fingerprint(req: &Prepare, upload_digest: &str) -> Result<String, ApiError> {
    let mut value = json!({"resource":req.resource,"uploadDigest":upload_digest});
    if req.format != "docx" || req.legacy_backend.is_some() || req.docx_options.is_some() {
        value["format"] = json!(req.format);
        value["legacyBackend"] = json!(req.legacy_backend);
        value["docxOptions"] = json!(req.docx_options);
    }
    Ok(digest(&serde_json::to_vec(&value).map_err(|_| changed())?))
}
fn unknown() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "resultUnknown",
        UiMessage::error_api_hosted_office_import_external_import_unresolved(),
    )
}
fn legacy_root(inner: &HostedInner, actor: &str) -> Result<PathBuf, ApiError> {
    let root = inner.office.legacy_root.as_ref().ok_or_else(|| {
        ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "conversionUnavailable",
            UiMessage::error_api_hosted_office_import_external_import_directory_required(),
        )
    })?;
    Ok(root
        .join(digest(scope(inner, actor).as_bytes()))
        .join("imports"))
}
fn conversion_location(
    inner: &HostedInner,
    actor: &str,
    operation_id: &str,
    actor_epoch: u64,
    layout: Option<&str>,
) -> Result<super::super::legacy_office::ConversionLocation, ApiError> {
    match layout {
        None => Ok(super::super::legacy_office::ConversionLocation {
            root: legacy_root(inner, actor)?,
            request_id: operation_id.into(),
        }),
        Some("flatV1") => super::super::legacy_office::scoped_location(
            inner.office.legacy_root.as_ref().ok_or_else(unknown)?,
            &inner.config.deployment,
            &inner.config.root,
            actor,
            actor_epoch,
            operation_id,
            "import",
        )
        .map_err(|_| unknown()),
        _ => Err(unknown()),
    }
}
fn claim_path(root: &Path, id: &str) -> PathBuf {
    root.join("claims").join(format!("{id}.json"))
}
fn plain_file(metadata: &fs::Metadata) -> bool {
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return false;
        }
    }
    true
}
fn owned_directory_exists(base: &Path, directory: &Path) -> Result<bool, ApiError> {
    directory.strip_prefix(base).map_err(|_| unknown())?;
    let mut existing = directory;
    loop {
        match fs::symlink_metadata(existing) {
            Ok(_) => {
                office::ensure_owned(base, existing, false).map_err(|_| unknown())?;
                return Ok(existing == directory);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if existing == base {
                    return Ok(false);
                }
                existing = existing.parent().ok_or_else(unknown)?;
            }
            Err(_) => return Err(unknown()),
        }
    }
}
fn read_claim(base: &Path, root: &Path, id: &str) -> Result<Option<ImportClaim>, ApiError> {
    // Validate the existing ancestor chain even when the final file or claims
    // directory is absent; an empty redirected path is not a trusted absence.
    if !owned_directory_exists(base, &root.join("claims"))? {
        return Ok(None);
    }
    let path = claim_path(root, id);
    let meta = match fs::symlink_metadata(&path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(unknown()),
    };
    if !plain_file(&meta) || meta.len() > 2 * 1024 * 1024 {
        return Err(unknown());
    }
    let data = fs::read(&path).map_err(|_| unknown())?;
    let seal = path.with_extension("sha256");
    let seal_meta = fs::symlink_metadata(&seal).map_err(|_| unknown())?;
    if !plain_file(&seal_meta)
        || seal_meta.len() != 64
        || fs::read(seal).map_err(|_| unknown())? != digest(&data).as_bytes()
    {
        return Err(unknown());
    }
    let claim: ImportClaim = serde_json::from_slice(&data).map_err(|_| unknown())?;
    if claim.schema_version != 1 || claim.operation_id != id {
        return Err(unknown());
    }
    Ok(Some(claim))
}
fn retain_claim(base: &Path, root: &Path, claim: ImportClaim) -> Result<ImportClaim, ApiError> {
    if let Some(old) = read_claim(base, root, &claim.operation_id)? {
        if old.fingerprint != claim.fingerprint {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "operationConflict",
                UiMessage::error_api_hosted_office_import_import_request_input_conflict(),
            ));
        }
        return Ok(old);
    }
    office::ensure_owned(base, &root.join("claims"), true).map_err(|_| unknown())?;
    let bytes = serde_json::to_vec(&claim).map_err(|_| unknown())?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(claim_path(root, &claim.operation_id))
        .map_err(|_| unknown())?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| unknown())?;
    let mut seal = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(claim_path(root, &claim.operation_id).with_extension("sha256"))
        .map_err(|_| unknown())?;
    seal.write_all(digest(&bytes).as_bytes())
        .and_then(|_| seal.sync_all())
        .map_err(|_| unknown())?;
    Ok(claim)
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
    if let Some(decision) = &decision {
        access_format(inner, actor, &status.resource, &decision.format)?;
    }
    let same_basis = decision.as_ref().is_some_and(|d| {
        d.actor_epoch == inner.actor_generation(actor)
            && config_hash(inner, file).is_ok_and(|hash| hash == d.config_hash)
            && access_format(inner, actor, &status.resource, &d.format).is_ok()
    });
    let mut preview = json!({});
    if same_basis {
        for key in ["summary", "losses", "legacyConversion"] {
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
                let formats=["docx","doc","wps"].into_iter().filter(|format|access_format(inner,&actor,resource,format).is_ok()).collect::<Vec<_>>();
                targets.push(json!({"resource":resource,"title":file.title,"available":available(&target),"formats":formats}));
            }
            Ok(json!({"targets":targets}))
        })
        .await?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Prepare>,
) -> ApiResult {
    let parsed_id = uuid::Uuid::parse_str(&req.operation_id);
    if parsed_id.is_err()
        || !["docx", "doc", "wps"].contains(&req.format.as_str())
        || (req.format != "docx"
            && parsed_id.as_ref().is_ok_and(|id| {
                id.to_string() != req.operation_id
                    || id.get_version() != Some(uuid::Version::Random)
            }))
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_word_import_request_invalid(),
        ));
    }
    if req.format == "docx" && req.legacy_backend.is_some() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_docx_external_converter_unused(),
        ));
    }
    if req.docx_options.as_ref().is_some_and(|options| {
        serde_json::to_vec(options).map_or(true, |bytes| bytes.len() > 1024 * 1024)
    }) {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_caption_options_budget(),
        ));
    }
    let bytes = upload(&req.bytes_base64)?;
    let upload_digest = digest(&bytes);
    let input_fingerprint = fingerprint(&req, &upload_digest)?;
    let captured = req.clone();
    let check = auth.clone();
    let claim_fingerprint = input_fingerprint.clone();
    let guard_state = state.clone();
    let frozen=locked(&state,move|inner|{
        let actor=inner.actor(&check)?;let file=access_format(inner,&actor,&captured.resource,&captured.format)?;let actor_scope=scope(inner,&actor);
        if let Some(old)=inner.core.with_actor_for_root(&actor_scope,&inner.config.root,|core|core.actor_feature_retry(&captured.resource,&captured.operation_id,&claim_fingerprint)).map_err(core_error)?{return Ok(ImportStart::Existing(safe_status(inner,&actor,&file,old)?));}
        let (_,_,target)=slot(inner,&file)?;if !available(&target){return Err(changed());}
        let mut decision=Decision{actor_epoch:inner.actor_generation(&actor),expires_at_unix_ms:now_ms().saturating_add(TTL.as_millis()as u64),config_hash:config_hash(inner,&file)?,upload_digest,format:captured.format.clone()};
        let mut root=None;let mut conversion_id=None;let mut cancel=None;let mut recover_only=false;let mut guard=None;
        if captured.format!="docx"{
            let owned_root=legacy_root(inner,&actor)?;
            let base=inner.office.legacy_root.as_ref().ok_or_else(unknown)?;
            // Check capacity before retaining any new upload/options claim.
            // Existing IDs still validate their original binding before a
            // pending response or capacity rejection; no rejected UUID grows
            // the durable claims directory.
            let previous=read_claim(base,&owned_root,&captured.operation_id)?;
            recover_only=previous.is_some();
            if previous.as_ref().is_some_and(|claim|claim.fingerprint!=claim_fingerprint){return Err(ApiError::new(StatusCode::CONFLICT,"operationConflict",UiMessage::error_api_hosted_office_import_import_request_input_conflict()));}
            let key=(actor.clone(),captured.operation_id.clone());
            if let Some(claim)=&previous && (claim.decision.actor_epoch!=inner.actor_generation(&actor)||claim.decision.config_hash!=config_hash(inner,&file)?||claim.decision.expires_at_unix_ms<=now_ms()){return Err(changed());}
            if inner.office.import_active.contains_key(&key){return Ok(ImportStart::Existing(json!({"resource":captured.resource,"operationId":captured.operation_id,"status":"pending","planId":null,"preview":{},"commitAllowed":false})));}
            if inner.office.import_active.len()>=32||inner.office.import_active.keys().filter(|(owner,_)|owner==&actor).count()>=8{return Err(ApiError::bad(UiMessage::error_api_hosted_office_import_external_import_capacity()));}
            let claim=retain_claim(base,&owned_root,ImportClaim{schema_version:1,fingerprint:claim_fingerprint.clone(),decision:decision.clone(),resource:captured.resource.clone(),operation_id:captured.operation_id.clone(),format:captured.format.clone(),legacy_backend:captured.legacy_backend.clone(),docx_options:captured.docx_options.clone(),conversion_layout:Some("flatV1".into())})?;
            let location=conversion_location(inner,&actor,&captured.operation_id,claim.decision.actor_epoch,claim.conversion_layout.as_deref())?;
            office::ensure_owned(base,&location.root,!recover_only).map_err(|_|unknown())?;
            decision=claim.decision;
            if decision.actor_epoch!=inner.actor_generation(&actor)||decision.config_hash!=config_hash(inner,&file)?||decision.expires_at_unix_ms<=now_ms(){return Err(changed());}
            let flag=Arc::new(AtomicBool::new(inner.office.closing));inner.office.import_active.insert(key.clone(),office::LegacyImportRun{actor_epoch:decision.actor_epoch,cancel:flag.clone()});guard=Some(ImportGuard{state:guard_state,key,cancel:flag.clone()});cancel=Some(flag);root=Some(location.root);conversion_id=Some(location.request_id);
        }
        Ok(ImportStart::Frozen(Box::new(FrozenImport{actor,decision,root,conversion_id,cancel,recover_only,_guard:guard})))
    }).await?;
    let frozen = match frozen {
        ImportStart::Existing(value) => return Ok(Json(value)),
        ImportStart::Frozen(frozen) => *frozen,
    };
    let mut native_bytes = bytes;
    let mut originals = vec![];
    let mut conversion_losses = vec![];
    let mut legacy_preview = None;
    if let (Some(root), Some(cancel)) = (&frozen.root, &frozen.cancel) {
        let conversion_id = frozen.conversion_id.as_deref().ok_or_else(unknown)?;
        let conversion_request = super::super::legacy_office::ConversionRequest {
            backend: req
                .legacy_backend
                .clone()
                .unwrap_or_else(|| "libreoffice".into()),
            direction: if req.format == "doc" {
                super::super::legacy_office::Direction::DocToDocx
            } else {
                super::super::legacy_office::Direction::WpsToDocx
            },
            bytes: native_bytes,
        };
        let result = if frozen.recover_only {
            super::super::legacy_office::recover_once(conversion_id, conversion_request, root).await
        } else {
            super::super::legacy_office::convert_once(
                conversion_id,
                conversion_request,
                root,
                cancel.clone(),
            )
            .await
        };
        // Failed/unknown/cancelled external results cross the same authority
        // boundary as success. Do not disclose a late result after revocation
        // or after the originally frozen target was replaced.
        let original_actor = frozen.actor.clone();
        let original_decision = frozen.decision.clone();
        let original_resource = req.resource.clone();
        let original_format = req.format.clone();
        let original_auth = auth.clone();
        locked(&state, move |inner| {
            if inner.actor(&original_auth)? != original_actor
                || inner.actor_generation(&original_actor) != original_decision.actor_epoch
                || original_decision.expires_at_unix_ms <= now_ms()
            {
                return Err(changed());
            }
            let file = access_format(inner, &original_actor, &original_resource, &original_format)?;
            if config_hash(inner, &file)? != original_decision.config_hash
                || !available(&slot(inner, &file)?.2)
            {
                return Err(changed());
            }
            Ok(())
        })
        .await?;
        let converted = result.map_err(|error| {
            ApiError::new(
                StatusCode::CONFLICT,
                if matches!(
                    error.code.as_str(),
                    "conversionUnknown" | "receiptFailed" | "conversionNotRegistered"
                ) {
                    "resultUnknown"
                } else {
                    "officeImportConversion"
                },
                UiMessage::error_api_hosted_office_import_external_import_result_unconfirmed(),
            )
        })?;
        if converted.status != "succeeded" {
            return Err(ApiError::from_core_parts(
                StatusCode::CONFLICT,
                if converted.status == "unknown" {
                    "resultUnknown"
                } else if converted.status == "cancelled" {
                    "officeImportCancelled"
                } else {
                    "conversionUnavailable"
                },
                converted.public_description(),
                converted.public_details(),
            ));
        }
        native_bytes = converted
            .intermediate_docx
            .as_ref()
            .ok_or_else(unknown)?
            .bytes
            .clone();
        originals.push(weftext_export::docx::OriginalImportAsset {
            filename: format!("original.{}", req.format),
            mime: if req.format == "doc" {
                "application/msword"
            } else {
                "application/octet-stream"
            }
            .into(),
            bytes: converted.original.bytes.clone(),
        });
        conversion_losses = converted
            .public_losses()
            .into_iter()
            .map(|loss| weftext_export::exchange::NativeLoss {
                path: format!("original.{}", req.format),
                code: loss.code,
                description: loss.description,
                product_diagnostics: loss.product_diagnostics,
            })
            .collect();
        let mut metadata = converted.metadata();
        metadata["requestId"] = json!(req.operation_id);
        legacy_preview = Some(metadata);
    }
    Ok(Json(locked(&state,move|inner|{
        let actor=inner.actor(&auth)?;if actor!=frozen.actor||inner.actor_generation(&actor)!=frozen.decision.actor_epoch||frozen.decision.expires_at_unix_ms<=now_ms(){return Err(changed());}
        let file=access_format(inner,&actor,&req.resource,&req.format)?;if config_hash(inner,&file)?!=frozen.decision.config_hash||frozen.cancel.as_ref().is_some_and(|cancel|cancel.load(Ordering::Acquire)){return Err(changed());}
        let actor_scope=scope(inner,&actor);if let Some(old)=inner.core.with_actor_for_root(&actor_scope,&inner.config.root,|core|core.actor_feature_retry(&req.resource,&req.operation_id,&input_fingerprint)).map_err(core_error)?{return safe_status(inner,&actor,&file,old);}
        let(parent,name,target)=slot(inner,&file)?;if !available(&target){return Err(changed());}
        let mut preview=json!({"summary":UiMessage::error_api_hosted_office_import_plan_summary(),"_officeImport":frozen.decision});
        if let Some(legacy)=legacy_preview{preview["legacyConversion"]=legacy;}
        let intent=if req.format=="docx"&&req.docx_options.is_none(){OperationIntent::ImportDocx{parent:parent.to_string_lossy().into_owned(),name,bytes:native_bytes,allow_losses:true}}else{OperationIntent::ImportDocxWithOptions{parent:parent.to_string_lossy().into_owned(),name,bytes:native_bytes,allow_losses:true,options:req.docx_options.unwrap_or_default(),originals,conversion_losses}};
        let request=OperationRequest{operation_id:req.operation_id.clone(),root:inner.config.root.to_string_lossy().into_owned(),intent};
        let status=inner.core.with_actor_for_root(&actor_scope,&inner.config.root,|core|core.actor_feature_prepare(&req.resource,&req.operation_id,&input_fingerprint,request,preview,&[])).map_err(core_error)?;safe_status(inner,&actor,&file,status)
    }).await?))
}
enum ImportStart {
    Existing(Value),
    Frozen(Box<FrozenImport>),
}
struct FrozenImport {
    actor: String,
    decision: Decision,
    root: Option<PathBuf>,
    conversion_id: Option<String>,
    cancel: Option<Arc<AtomicBool>>,
    recover_only: bool,
    // Created inside the blocking registration transaction, so dropping its
    // undelivered result also cancels/removes the exact active entry.
    _guard: Option<ImportGuard>,
}
struct ImportGuard {
    state: HostedState,
    key: (String, String),
    cancel: Arc<AtomicBool>,
}
impl Drop for ImportGuard {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Release);
        if let Ok(mut inner) = self.state.inner.lock() {
            inner.office.import_active.remove(&self.key);
        }
    }
}
async fn inspect(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Inspect>,
) -> ApiResult {
    if req.format != "docx" {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_caption_analysis_docx_required(),
        ));
    }
    let bytes = upload(&req.bytes_base64)?;
    let resource = req.resource;
    let captured = resource.clone();
    let check = auth.clone();
    let (actor, epoch, binding) = locked(&state, move |inner| {
        let actor = inner.actor(&check)?;
        let file = access_format(inner, &actor, &captured, "docx")?;
        let (_, _, target) = slot(inner, &file)?;
        if !available(&target) {
            return Err(changed());
        }
        Ok((
            actor.clone(),
            inner.actor_generation(&actor),
            config_hash(inner, &file)?,
        ))
    })
    .await?;
    let analysis =
        crate::work::spawn_blocking(move || weftext_export::docx::analyze_docx_import(&bytes))
            .await
            .map_err(|_| unknown())?
            .map_err(|_| {
                ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "officeImportInspect",
                    UiMessage::error_api_hosted_office_import_word_original_analysis_unavailable(),
                )
            })?;
    Ok(Json(
        locked(&state, move |inner| {
            if inner.actor(&auth)? != actor || inner.actor_generation(&actor) != epoch {
                return Err(changed());
            }
            let file = access_format(inner, &actor, &resource, "docx")?;
            if config_hash(inner, &file)? != binding {
                return Err(changed());
            }
            let (_, _, target) = slot(inner, &file)?;
            if !available(&target) {
                return Err(changed());
            }
            value(analysis)
        })
        .await?,
    ))
}
async fn cancel(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    if uuid::Uuid::parse_str(&req.operation_id).is_err() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_original_import_id_invalid(),
        ));
    }
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;access(inner,&actor,&req.resource)?;
        let root=legacy_root(inner,&actor)?;let claim=read_claim(inner.office.legacy_root.as_deref().ok_or_else(unknown)?,&root,&req.operation_id)?.ok_or_else(unavailable)?;
        access_format(inner,&actor,&req.resource,&claim.format)?;if claim.resource!=req.resource||claim.decision.actor_epoch!=inner.actor_generation(&actor){return Err(changed());}
        let active=inner.office.import_active.get(&(actor,req.operation_id.clone()));if let Some(active)=active{active.cancel.store(true,Ordering::Release);}
        let description = ErrorDescription::new(UiMessage::operation_hosted_office_import_cancel_requested());
        Ok(json!({"resource":req.resource,"operationId":req.operation_id,"status":if active.is_some(){"cancelling"}else{"notRunning"},
            "explanation":description.explanation,"diagnostics":description.diagnostics,"causes":description.causes}))
    }).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    if uuid::Uuid::parse_str(&req.operation_id).is_err() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_original_import_id_invalid(),
        ));
    }
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;let file=access(inner,&actor,&req.resource)?;
        let status=inner.core.with_actor_for_root(&scope(inner,&actor), &inner.config.root,|core|core.actor_feature_status(&req.resource,&req.operation_id)).map_err(core_error)?;
        match status {Some(status)=>safe_status(inner,&actor,&file,status),None=>{
            let pending=inner.office.import_active.contains_key(&(actor.clone(),req.operation_id.clone()));
            let retained=if let Some(base)=inner.office.legacy_root.as_deref(){if let Some(claim)=read_claim(base,&legacy_root(inner,&actor)?,&req.operation_id)?{if claim.resource!=req.resource{return Err(unavailable());}access_format(inner,&actor,&req.resource,&claim.format)?;true}else{false}}else{false};
            Ok(json!({"operationId":req.operation_id,"resource":req.resource,"status":if pending{"pending"}else if retained{"recoveryPending"}else{"notRegistered"},"planId":null,"preview":{},"commitAllowed":false}))
        }}
    }).await?))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Commit>,
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
            if let Ok(decision) =
                serde_json::from_value::<Decision>(old.preview["_officeImport"].clone())
            {
                access_format(inner, &actor, &req.resource, &decision.format)?;
            }
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
    RequestJson(req): RequestJson<Operation>,
) -> ApiResult {
    if uuid::Uuid::parse_str(&req.operation_id).is_err() {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_office_import_original_import_id_invalid(),
        ));
    }
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let file = access(inner, &actor, &req.resource)?;
            // Authorize the original operation's exact format *before* Core
            // writes a cancellation tombstone or cancels a prepared plan.
            let original = inner
                .core
                .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |core| {
                    core.actor_feature_status(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            let original_format = original
                .as_ref()
                .and_then(|status| {
                    serde_json::from_value::<Decision>(status.preview["_officeImport"].clone()).ok()
                })
                .map(|decision| decision.format);
            let mut authorized_format = original_format;
            if inner.office.legacy_root.is_some() {
                let root = legacy_root(inner, &actor)?;
                match read_claim(
                    inner.office.legacy_root.as_deref().ok_or_else(unknown)?,
                    &root,
                    &req.operation_id,
                ) {
                    Ok(Some(claim)) => {
                        if claim.resource != req.resource {
                            return Err(unavailable());
                        }
                        access_format(inner, &actor, &req.resource, &claim.format)?;
                        if authorized_format
                            .as_ref()
                            .is_some_and(|format| format != &claim.format)
                        {
                            return Err(changed());
                        }
                        authorized_format = Some(claim.format);
                    }
                    Ok(None) => {}
                    // A trusted Core record supplies the original format even
                    // if its earlier private conversion claim was damaged.
                    // Without Core, only a claim that provably never entered
                    // the external wrapper may be fenced. Keep every damaged
                    // file in place, and never reinterpret an existing wrapper
                    // directory (or a metadata I/O failure) as no effects.
                    Err(_) if authorized_format.is_some() => {}
                    Err(_) => {
                        let base = inner.office.legacy_root.as_deref().ok_or_else(unknown)?;
                        let flat = conversion_location(
                            inner,
                            &actor,
                            &req.operation_id,
                            inner.actor_generation(&actor),
                            Some("flatV1"),
                        )?;
                        if inner
                            .office
                            .import_active
                            .contains_key(&(actor.clone(), req.operation_id.clone()))
                            || owned_directory_exists(base, &root.join(&req.operation_id))?
                            || owned_directory_exists(base, &flat.root.join(flat.request_id))?
                        {
                            return Err(unknown());
                        }
                    }
                }
            }
            if let Some(requested) = &req.format
                && authorized_format
                    .as_ref()
                    .is_some_and(|format| format != requested)
            {
                return Err(changed());
            }
            access_format(
                inner,
                &actor,
                &req.resource,
                authorized_format
                    .as_deref()
                    .or(req.format.as_deref())
                    .unwrap_or("docx"),
            )?;
            if let Some(active) = inner
                .office
                .import_active
                .get(&(actor.clone(), req.operation_id.clone()))
            {
                active.cancel.store(true, Ordering::Release);
            }
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

#[cfg(all(test, windows))]
#[path = "office_import_tests.rs"]
mod tests;
