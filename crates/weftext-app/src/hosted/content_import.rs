//! Operator-bound content bundles over the original Core CreateTree writer.
//! Producer permission and future author/resource permissions are independent.
use super::*;
use axum::extract::DefaultBodyLimit;
use std::{fs, path::Path};
use weftext_core::{
    ActorFeatureStatus, ConfigRole, OperationIntent, OperationRequest, SnapshotWitness,
};
use weftext_knowledge::NodeRef;

const MAX_UPLOAD: usize = 16 * 1024 * 1024;
const MAX_MEMBERS: usize = 5000;
const TTL_MS: u64 = 900_000;
const FORMATS: [&str; 4] = ["native", "bibtex", "csl-json", "ris"];

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Slot {
    pub(super) parent: String,
    pub(super) name: String,
    pub(super) title: String,
    #[serde(default)]
    pub(super) inputs: Vec<String>,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Actions {
    #[serde(default)]
    pub(super) native: bool,
    #[serde(default)]
    pub(super) bibliography: bool,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) imports: Actions,
    #[serde(default)]
    pub(super) members: super::Rights,
    #[serde(default)]
    pub(super) resources: resource_collections::Rights,
}
fn uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
pub(super) fn split_member<'a>(config: &Config, value: &'a str) -> Option<(&'a str, &'a str)> {
    let (slot, node) = value.split_once('/')?;
    (config.content_import_slots.contains_key(slot) && uuid(node)).then_some((slot, node))
}
pub(super) fn split_asset<'a>(config: &Config, value: &'a str) -> Option<(&'a str, &'a str)> {
    let (owner, resource) = value.rsplit_once('/')?;
    (split_member(config, owner).is_some() && uuid(resource)).then_some((owner, resource))
}
pub(super) fn selection<'a>(config: &Config, value: &'a str) -> Option<&'a str> {
    let selected = value.strip_suffix("/*")?;
    (config.content_import_slots.contains_key(selected) || split_member(config, selected).is_some())
        .then_some(selected)
}
pub(super) fn member_rights(
    inner: &HostedInner,
    actor: &str,
    resource: &str,
) -> Option<super::Rights> {
    let (slot, _) = split_member(&inner.config, resource)?;
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)?
        .content_imports
        .get(slot)
        .map(|r| r.members.clone())
}
pub(super) fn resource_rights(
    inner: &HostedInner,
    actor: &str,
    owner: &str,
) -> Option<resource_collections::Rights> {
    let (slot, _) = split_member(&inner.config, owner)?;
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)?
        .content_imports
        .get(slot)
        .map(|r| r.resources.clone())
}
fn target(inner: &HostedInner, id: &str) -> Result<PathBuf, ApiError> {
    let slot = inner
        .config
        .content_import_slots
        .get(id)
        .ok_or_else(unavailable)?;
    let parent = inner
        .config
        .directories
        .get(&slot.parent)
        .ok_or_else(unavailable)?;
    Ok(inner.config.root.join(&parent.path).join(&slot.name))
}
/// Immutable producer symbols only. This does not qualify/read each body.
fn members(inner: &HostedInner, slot: &str) -> Result<Vec<(String, NodeRef)>, ApiError> {
    let target = target(inner, slot)?;
    let entries = inner
        .core
        .content_import_slot_members_at(&inner.config.root, &target, slot)
        .map_err(core_error)?;
    if entries.len() > MAX_MEMBERS {
        return Err(ApiError::bad("导入成员超过 5000 项预算"));
    }
    let mut unique = BTreeSet::new();
    entries
        .into_iter()
        .map(|n| {
            let path = Path::new(&n.relative_path);
            if !unique.insert(n.owner.node_id.clone())
                || !path
                    .components()
                    .all(|p| matches!(p, std::path::Component::Normal(_)))
            {
                return Err(changed());
            }
            let path = target
                .join(path)
                .strip_prefix(&inner.config.root)
                .map_err(|_| changed())?
                .to_string_lossy()
                .into_owned();
            Ok((path, n.owner))
        })
        .collect()
}
/// This private FileEntry is derived, never inserted into operator Config.
pub(super) fn member_file(
    inner: &HostedInner,
    resource: &str,
) -> Result<Option<FileEntry>, ApiError> {
    let Some((slot, node)) = split_member(&inner.config, resource) else {
        return Ok(None);
    };
    let config = &inner.config.content_import_slots[slot];
    let (path, owner) = members(inner, slot)?
        .into_iter()
        .find(|(_, o)| o.node_id == node)
        .ok_or_else(unavailable)?;
    Ok(Some(FileEntry {
        path,
        title: config.title.clone(),
        lifecycle: "live".into(),
        expected_node: Some(owner),
        editable: true,
        importable: false,
        inputs: config.inputs.clone(),
    }))
}
pub(super) fn member_ids(inner: &HostedInner, actor: &str) -> Result<Vec<String>, ApiError> {
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    let mut ids = vec![];
    for (id, r) in &user.content_imports {
        if r.members == super::Rights::default() {
            continue;
        }
        for (_, owner) in members(inner, id)? {
            ids.push(format!("{id}/{}", owner.node_id));
        }
        if ids.len() > MAX_MEMBERS {
            return Err(ApiError::bad("获权导入成员超过 5000 项预算"));
        }
    }
    Ok(ids)
}
pub(super) fn slot_member_ids(inner: &HostedInner, slot: &str) -> Result<Vec<String>, ApiError> {
    members(inner, slot)?
        .into_iter()
        .map(|(_, o)| Ok(format!("{slot}/{}", o.node_id)))
        .collect()
}
pub(super) fn collection_ids(inner: &HostedInner, actor: &str) -> Result<Vec<String>, ApiError> {
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    let mut ids = vec![];
    for (slot, r) in &user.content_imports {
        if r.resources.read || r.resources.r#import {
            ids.extend(slot_member_ids(inner, slot)?);
            if ids.len() > MAX_MEMBERS {
                return Err(ApiError::bad("集合目录超过 5000 项预算"));
            }
        }
    }
    Ok(ids)
}
pub(super) fn qualify_member(inner: &HostedInner, resource: &str) -> Result<(), ApiError> {
    let Some((slot, node)) = split_member(&inner.config, resource) else {
        return Ok(());
    };
    let file = member_file(inner, resource)?.ok_or_else(unavailable)?;
    let q = inner
        .core
        .qualified_content_import_node_at(
            &inner.config.root,
            &inner.config.root.join(&file.path),
            slot,
            node,
        )
        .map_err(core_error)?;
    inner
        .core
        .validate_query_node_witness(&inner.config.root, &q.witness)
        .map_err(core_error)
}
pub(super) fn binding(inner: &HostedInner, id: &str) -> Result<String, ApiError> {
    let slot = split_member(&inner.config, id)
        .map(|(s, _)| s)
        .or_else(|| {
            split_asset(&inner.config, id)
                .and_then(|(o, _)| split_member(&inner.config, o).map(|(s, _)| s))
        })
        .or_else(|| selection(&inner.config, id).map(|s| s.split('/').next().unwrap()))
        .ok_or_else(unavailable)?;
    let config = &inner.config.content_import_slots[slot];
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"slot":config,"parent":inner.config.directories.get(&config.parent),"id":id})).map_err(|_|changed())?))
}
pub(super) fn valid_config(config: &Config) -> bool {
    config.content_import_slots.len() <= 64
        && config.content_import_slots.iter().all(|(id, s)| {
            valid_id(id)
                && !config.files.contains_key(id)
                && !config.resources.contains_key(id)
                && !config.resource_collections.contains_key(id)
                && !s.name.is_empty()
                && s.name.len() <= 240
                && !s.name.contains(['/', '\\', ':'])
                && !s.name.ends_with(['.', ' '])
                && !s.name.chars().any(char::is_control)
                && !matches!(s.name.as_str(), "." | "..")
                && !s.title.is_empty()
                && s.title.len() <= 500
                && s.inputs.len() <= 64
                && config
                    .directories
                    .get(&s.parent)
                    .is_some_and(|p| p.lifecycle == "live")
        })
}
pub(super) fn valid_rights(config: &Config, rights: &BTreeMap<String, Rights>) -> bool {
    rights.len() <= 64
        && rights.iter().all(|(id, r)| {
            config.content_import_slots.contains_key(id)
                && !r.members.office.r#import
                && (!r.members.write || r.members.read)
                && (!r.members.history || r.members.read)
                && (!r.members.download || r.members.read)
                && (!r.resources.download || r.resources.read)
                && query::valid_rights(&r.members.query)
                && r.members.fields.len() <= 64
                && r.members
                    .fields
                    .iter()
                    .all(|(id, f)| !id.is_empty() && id.len() <= 256 && (!f.write || f.read))
                && (!r.members.annotations.write || r.members.annotations.read)
                && (!r.members.annotations.accept
                    || (r.members.annotations.write
                        && r.members.annotations.target
                        && r.members.write))
                && (!r.members.annotations.proofread
                    || (r.members.read && r.members.annotations.target))
        })
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Inspect {
    slot: String,
    format: String,
    bytes_base64: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    slot: String,
    operation_id: String,
    format: String,
    bytes_base64: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    slot: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    slot: String,
    operation_id: String,
    plan_id: String,
    #[serde(default)]
    accept_losses: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Members {
    slot: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    epoch: u64,
    config_hash: String,
    expires: u64,
    format: String,
    upload_digest: String,
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:content-import", inner.actor_scope(actor))
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "contentImportChanged",
        "导入槽、原件、授权或原计划已变化，请保留原输入并核对原操作。",
    )
}
fn core_error(_: CoreError) -> ApiError {
    changed()
}
fn import_access(
    inner: &HostedInner,
    actor: &str,
    slot: &str,
    format: &str,
) -> Result<(), ApiError> {
    let r = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.content_imports.get(slot))
        .ok_or_else(unavailable)?;
    if !inner.config.content_import_slots.contains_key(slot)
        || !FORMATS.contains(&format)
        || !(if format == "native" {
            r.imports.native
        } else {
            r.imports.bibliography
        })
    {
        return Err(unavailable());
    }
    Ok(())
}
fn operation_access(inner: &HostedInner, actor: &str, slot: &str) -> Result<(), ApiError> {
    let r = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.content_imports.get(slot))
        .ok_or_else(unavailable)?;
    if !inner.config.content_import_slots.contains_key(slot)
        || !(r.imports.native || r.imports.bibliography)
    {
        return Err(unavailable());
    }
    Ok(())
}
fn upload(value: &str) -> Result<Vec<u8>, ApiError> {
    if value.len() > MAX_UPLOAD.div_ceil(3) * 4 {
        return Err(ApiError::bad("导入原件上限为 16 MiB"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad("导入原件编码无效"))?;
    if bytes.is_empty() || bytes.len() > MAX_UPLOAD {
        return Err(ApiError::bad("导入原件须为 1 字节至 16 MiB"));
    }
    Ok(bytes)
}
fn config_hash(inner: &HostedInner, id: &str) -> Result<String, ApiError> {
    let s = inner
        .config
        .content_import_slots
        .get(id)
        .ok_or_else(unavailable)?;
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"slot":s,"parent":inner.config.directories.get(&s.parent)})).map_err(|_|changed())?))
}
fn safe_status(
    inner: &HostedInner,
    actor: &str,
    slot: &str,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let d = serde_json::from_value::<Decision>(status.preview["_contentImport"].clone()).ok();
    let same = d.as_ref().is_some_and(|d| {
        d.epoch == inner.actor_generation(actor)
            && config_hash(inner, slot).is_ok_and(|h| h == d.config_hash)
            && import_access(inner, actor, slot, &d.format).is_ok()
    });
    let allowed =
        same && status.status == "prepared" && d.as_ref().is_some_and(|d| d.expires > now_ms());
    let mut preview = json!({});
    if same {
        for k in ["summary", "losses", "recordCount"] {
            if let Some(v) = status.preview.get(k) {
                preview[k] = v.clone();
            }
        }
    }
    Ok(
        json!({"slot":slot,"operationId":status.operation_id,"status":status.status,"planId":status.plan_id.filter(|_|allowed),"preview":preview,"nodeRefs":if same{status.node_refs}else{vec![]},"commitAllowed":allowed}),
    )
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/import", get(catalog))
        .route("/api/hosted/import/inspect", post(inspect))
        .route("/api/hosted/import/prepare", post(prepare))
        .route("/api/hosted/import/commit", post(commit))
        .route("/api/hosted/import/operation", post(operation))
        .route("/api/hosted/import/settle", post(settle))
        .route("/api/hosted/import/members", post(list_members))
        .layer(DefaultBodyLimit::max(24 * 1024 * 1024))
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let mut targets = vec![];
            for (id, s) in &inner.config.content_import_slots {
                let formats = FORMATS
                    .iter()
                    .filter(|f| import_access(inner, &actor, id, f).is_ok())
                    .collect::<Vec<_>>();
                if formats.is_empty() {
                    continue;
                }
                let t = target(inner, id)?;
                let available = fs::symlink_metadata(&t)
                    .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
                targets
                    .push(json!({"id":id,"title":s.title,"formats":formats,"available":available}));
            }
            Ok(json!({"scope":"configuredImportSlots","targets":targets}))
        })
        .await?,
    ))
}
fn inspect_upload(inner: &mut HostedInner, format: &str, bytes: &[u8]) -> Result<Value, ApiError> {
    if format == "native" {
        let p = weftext_export::exchange::read_native_package(bytes)
            .map_err(|_| ApiError::bad("原生包无效或超过预算；原件未改动"))?;
        return Ok(
            json!({"uploadDigest":digest(bytes),"recordCount":p.manifest.entries.values().filter(|e|e.role==weftext_export::exchange::NativeFileRole::Source).count(),"losses":[],"canPrepare":true}),
        );
    }
    let proposal = bibliography_import_api::inspect_bibliography_import(format, bytes)
        .map_err(|_| ApiError::bad("文献原件无法完整解析或核对其保留字段，输入未改动"))?;
    let conflict = proposal.records.iter().any(|r| !r.conflicts.is_empty());
    let config = inner
        .core
        .read_config(&inner.config.root, ConfigRole::Schema)
        .map_err(core_error)?;
    let schema = match config.bytes {
        Some(bytes) => String::from_utf8(bytes).map_err(|_| changed())?,
        None => include_str!("../../../../docs/design/examples/schema.authoring.json").into(),
    };
    let _ = inner.core.take_snapshot_witness(&config.base_id);
    let can_prepare = !conflict
        && weftext_export::bibliography_import::build_bibliography_native_package(
            format,
            bytes,
            &schema,
            &uuid::Uuid::new_v4().to_string(),
        )
        .is_ok();
    Ok(
        json!({"uploadDigest":digest(bytes),"records":proposal.records,"losses":proposal.losses,"canPrepare":can_prepare,"reason":if can_prepare{Value::Null}else{json!("本次上传存在出版字段冲突、无映射的 typed 引用或不符合目的 Schema；请保留并核对原件，未创建或更新笔记。")}}),
    )
}
async fn inspect(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Inspect>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            import_access(inner, &actor, &req.slot, &req.format)?;
            library::access(inner, &actor, "schema", false)?;
            inspect_upload(inner, &req.format, &upload(&req.bytes_base64)?)
        })
        .await?,
    ))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{
    let actor=inner.actor(&auth)?;import_access(inner,&actor,&req.slot,&req.format)?;let bytes=upload(&req.bytes_base64)?;let upload_digest=digest(&bytes);let fp=digest(&serde_json::to_vec(&json!({"slot":req.slot,"format":req.format,"uploadDigest":upload_digest})).map_err(|_|changed())?);let scope=scope(inner,&actor);let root=inner.config.root.clone();
    if let Some(old)=inner.core.with_actor_for_root(&scope,&root,|c|c.actor_feature_retry(&req.slot,&req.operation_id,&fp)).map_err(core_error)?{return safe_status(inner,&actor,&req.slot,old);}
    library::access(inner,&actor,"schema",false)?;
    let inspected=inspect_upload(inner,&req.format,&bytes)?;if inspected["canPrepare"]!=true{return Err(ApiError::new(StatusCode::CONFLICT,"contentImportUnavailable","本次上传有冲突、typed 引用缺少映射或不符合目的 Schema；原件保留，未创建或更新笔记。"));}
    let s=inner.config.content_import_slots[&req.slot].clone();let p=inner.config.directories[&s.parent].clone();let parent=root.join(&p.path);let q=inner.core.qualified_query_scope_at(&root,&parent,&p.reference,"live").map_err(core_error)?;let mut witnesses:Vec<SnapshotWitness>=vec![];weftext_core::Core::merge_query_scope_witness(&mut witnesses,&q.witness).map_err(core_error)?;
    let d=Decision{epoch:inner.actor_generation(&actor),config_hash:config_hash(inner,&req.slot)?,expires:now_ms().saturating_add(TTL_MS),format:req.format.clone(),upload_digest};let preview=json!({"summary":"在固定空槽中创建独立管理笔记，保留上传原件与已支持的资源；原计划损失须明确接受。","recordCount":inspected.get("recordCount").cloned().unwrap_or_else(||json!(inspected["records"].as_array().map_or(0,Vec::len))),"_contentImport":d});
    let intent=if req.format=="native"{OperationIntent::ImportNative{parent:parent.to_string_lossy().into_owned(),name:s.name,bytes,allow_losses:true}}else{OperationIntent::ImportBibliography{parent:parent.to_string_lossy().into_owned(),name:s.name,format:req.format,bytes,allow_losses:true}};
    let request=OperationRequest{operation_id:req.operation_id.clone(),root:root.to_string_lossy().into_owned(),intent};let refs=witnesses.iter().collect::<Vec<_>>();let status=inner.core.with_actor_for_root(&scope,&root,|c|c.actor_feature_prepare_content_import(&req.slot,&req.operation_id,&fp,request,preview,&refs,&req.slot)).map_err(core_error)?;safe_status(inner,&actor,&req.slot,status)
}).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;operation_access(inner,&actor,&req.slot)?;let root=inner.config.root.clone();let scope=scope(inner,&actor);match inner.core.with_actor_for_root(&scope,&root,|c|c.actor_feature_status(&req.slot,&req.operation_id)).map_err(core_error)?{Some(s)=>safe_status(inner,&actor,&req.slot,s),None=>Ok(json!({"slot":req.slot,"operationId":req.operation_id,"status":"notRegistered","planId":null,"preview":{},"nodeRefs":[],"commitAllowed":false}))}}).await?))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            operation_access(inner, &actor, &req.slot)?;
            let root = inner.config.root.clone();
            let scope = scope(inner, &actor);
            let s = inner
                .core
                .with_actor_for_root(&scope, &root, |c| {
                    c.actor_feature_settle(&req.slot, &req.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, &req.slot, s)
        })
        .await?,
    ))
}
async fn commit(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Commit>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            operation_access(inner, &actor, &req.slot)?;
            let root = inner.config.root.clone();
            let scope = scope(inner, &actor);
            let old = inner
                .core
                .with_actor_for_root(&scope, &root, |c| {
                    c.actor_feature_status(&req.slot, &req.operation_id)
                })
                .map_err(core_error)?
                .ok_or_else(changed)?;
            let safe = safe_status(inner, &actor, &req.slot, old.clone())?;
            if old.status == "prepared"
                && (safe["commitAllowed"] != true || old.plan_id.as_deref() != Some(&req.plan_id))
            {
                return Err(changed());
            }
            let s = inner
                .core
                .with_actor_for_root(&scope, &root, |c| {
                    c.actor_feature_commit_with_losses(
                        &req.slot,
                        &req.operation_id,
                        &req.plan_id,
                        req.accept_losses,
                    )
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, &req.slot, s)
        })
        .await?,
    ))
}
async fn list_members(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Members>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;let r=inner.policy.users[&actor].content_imports.get(&req.slot).ok_or_else(unavailable)?;if r.members==super::Rights::default()&&!r.resources.read{return Err(unavailable());}let mut result=vec![];let mut partial=false;for(_,owner)in members(inner,&req.slot)?{let resource=format!("{}/{}",req.slot,owner.node_id);let ready=qualify_member(inner,&resource).is_ok();partial|=!ready;result.push(json!({"resource":resource,"nodeRef":owner,"title":inner.config.content_import_slots[&req.slot].title,"status":if ready{"ready"}else{"unavailable"},"canOpen":ready&&r.members.read}));}Ok(json!({"scope":"configuredImportSlotMembers","coverage":if partial{"partial"}else{"complete"},"members":result}))}).await?))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    fn fixture() -> (tempfile::TempDir, HostedState, Auth, Vec<u8>) {
        let (temp, state, auth, _, _, _) = history_api::test_fixture();
        let bytes = {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let admitted = inner
                .core
                .prepare_directory_admission(&root, &root, &id())
                .unwrap();
            inner
                .core
                .execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
                .unwrap();
            inner.config.directories.insert(
                "root".into(),
                query_scopes::Directory {
                    path: String::new(),
                    title: "approved parent".into(),
                    reference: admitted.target_ref,
                    lifecycle: "live".into(),
                },
            );
            for name in ["Native", "Bib", "Cancelled"] {
                inner.config.content_import_slots.insert(
                    name.into(),
                    Slot {
                        parent: "root".into(),
                        name: name.into(),
                        title: "approved upload".into(),
                        inputs: vec![],
                    },
                );
            }
            inner.core.capture_native_export(&root).unwrap().bytes
        };
        (temp, state, auth, bytes)
    }
    fn grant(state: &HostedState, source_read: bool) {
        let mut inner = state.inner.lock().unwrap();
        let user = inner.policy.users.get_mut("alice").unwrap();
        user.library = serde_json::from_value(json!({"schema":{"read":true}})).unwrap();
        for slot in ["Native", "Bib", "Cancelled"] {
            user.content_imports.insert(
                slot.into(),
                Rights {
                    imports: Actions {
                        native: true,
                        bibliography: true,
                    },
                    members: super::super::Rights {
                        read: source_read,
                        write: source_read,
                        fields: BTreeMap::from([(
                            "library/publication".into(),
                            FieldRights {
                                read: true,
                                write: false,
                            },
                        )]),
                        ..Default::default()
                    },
                    resources: resource_collections::Rights {
                        read: true,
                        download: true,
                        r#import: false,
                    },
                },
            );
        }
    }
    fn request(slot: &str, format: &str, bytes: &[u8]) -> Prepare {
        Prepare {
            slot: slot.into(),
            operation_id: id(),
            format: format.into(),
            bytes_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        }
    }
    async fn apply(state: &HostedState, auth: &Auth, req: Prepare) -> Value {
        let plan = prepare(State(state.clone()), Extension(auth.clone()), Json(req))
            .await
            .unwrap()
            .0;
        assert_eq!(plan["status"], "prepared");
        assert!(plan["preview"]["losses"].is_array());
        commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                slot: plan["slot"].as_str().unwrap().into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
                accept_losses: true,
            }),
        )
        .await
        .unwrap()
        .0
    }
    #[tokio::test]
    async fn native_producer_permission_does_not_open_members_and_independent_reader_uses_original_birth()
     {
        let (_temp, state, auth, bytes) = fixture();
        let req = request("Native", "native", &bytes);
        assert_eq!(
            prepare(State(state.clone()), Extension(auth.clone()), Json(req))
                .await
                .err()
                .unwrap()
                .status,
            StatusCode::NOT_FOUND
        );
        grant(&state, false);
        let result = apply(&state, &auth, request("Native", "native", &bytes)).await;
        assert_eq!(result["status"], "committed");
        let node = result["nodeRefs"][0]["nodeId"].as_str().unwrap();
        let resource = format!("Native/{node}");
        {
            let mut inner = state.inner.lock().unwrap();
            assert!(inner.open("alice", &resource).is_err());
            assert!(!inner.config.files.contains_key(&resource));
            assert!(inner.grant("alice", &resource).unwrap().fields["library/publication"].read);
            let r = inner.policy.users["alice"].content_imports["Native"].clone();
            inner
                .policy
                .users
                .get_mut("bob")
                .unwrap()
                .content_imports
                .insert(
                    "Native".into(),
                    Rights {
                        members: super::super::Rights {
                            read: true,
                            ..Default::default()
                        },
                        ..r
                    },
                );
            let opened = inner.open("bob", &resource).unwrap();
            assert_eq!(opened["resource"], resource);
            assert!(!inner.config.files.contains_key(&resource));
        }
        let listing = list_members(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Members {
                slot: "Native".into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(listing["coverage"], "complete");
        assert!(
            listing["members"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["canOpen"] == false)
        );
    }
    #[tokio::test]
    async fn bibliography_field_only_query_and_registered_original_keep_exact_uploaded_bytes() {
        let (_temp, state, auth, _) = fixture();
        grant(&state, false);
        let bytes = b"@article{k,title={Own Publication},author={Alpha, Author},year={2024}}\n";
        let result = apply(&state, &auth, request("Bib", "bibtex", bytes)).await;
        let resource = format!("Bib/{}", result["nodeRefs"][0]["nodeId"].as_str().unwrap());
        {
            let mut inner = state.inner.lock().unwrap();
            assert!(inner.open("alice", &resource).is_err());
            let c = resource_collections::capture(&inner, "alice", &resource).unwrap();
            let original = c
                .entries
                .iter()
                .find(|e| e.relative_path == "original.bib")
                .unwrap();
            let asset = format!(
                "{resource}/{}",
                original.resource_ref.rsplit('/').next().unwrap()
            );
            assert_eq!(
                media::capture_input(&inner, "alice", &asset)
                    .unwrap()
                    .bytes(),
                bytes
            );
            let q = inner
                .core
                .qualified_content_import_node_at(
                    &inner.config.root,
                    &inner.config.root.join(&inner.file(&resource).unwrap().path),
                    "Bib",
                    result["nodeRefs"][0]["nodeId"].as_str().unwrap(),
                )
                .unwrap();
            assert!(q.revision.is_some());
        }
        let req=serde_json::from_value(json!({"resources":[resource],"fields":["library/publication"],"plan":{"select":[{"name":"publication","expr":"field(\"library/publication\")"}]}})).unwrap();
        let output = query::execute(&state, &auth, req).await.unwrap();
        assert_eq!(output["result"]["rows"][0]["entityKey"], resource);
        assert!(!output.to_string().contains("sourceHash"));
    }
    #[tokio::test]
    async fn original_request_fingerprint_and_unknown_settle_fence_never_reallocate() {
        let (_temp, state, auth, bytes) = fixture();
        grant(&state, true);
        let original = request("Native", "native", &bytes);
        let op = original.operation_id.clone();
        let encoded = original.bytes_base64.clone();
        let first = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(original),
        )
        .await
        .unwrap()
        .0;
        let retry = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                slot: "Native".into(),
                operation_id: op.clone(),
                format: "native".into(),
                bytes_base64: encoded.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(first, retry);
        let changed = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                slot: "Native".into(),
                operation_id: op,
                format: "native".into(),
                bytes_base64: base64::engine::general_purpose::STANDARD.encode(b"different"),
            }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(changed.status, StatusCode::CONFLICT);
        let cancelled = id();
        let result = settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                slot: "Cancelled".into(),
                operation_id: cancelled.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(result["status"], "cancelled");
        let late = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                slot: "Cancelled".into(),
                operation_id: cancelled,
                format: "native".into(),
                bytes_base64: encoded,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(late["status"], "cancelled");
        assert!(
            !state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join("Cancelled")
                .exists()
        );
    }
}
