//! Explicit directory/tag consumers over the original Core navigation writer.
//! Directory reads do not grant Query scopes, child sources or admission.
use super::*;
use std::path::{Component, Path};
use weftext_core::{ActorFeatureStatus, HostedNavigationIntent, TagRegistrySnapshot};

const RESOURCE: &str = "navigation";
const TTL: Duration = Duration::from_secs(900);
const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_REGISTRY: usize = 4 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Admission {
    path: String,
    title: String,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Config {
    #[serde(default)]
    directory_admissions: BTreeMap<String, Admission>,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct TagRights {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    create: bool,
    #[serde(default)]
    rename: bool,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    directories: BTreeSet<String>,
    #[serde(default)]
    admit_directories: BTreeSet<String>,
    #[serde(default)]
    tags: TagRights,
}
#[derive(Default)]
pub(super) struct Store {
    bases: HashMap<String, Basis>,
}
#[derive(Clone)]
struct Basis {
    actor: String,
    epoch: u64,
    binding: String,
    registry: TagRegistrySnapshot,
    retained_bytes: usize,
    expires: Instant,
}
pub(super) fn valid_config(config: &super::Config) -> bool {
    let admissions = &config.navigation.directory_admissions;
    admissions.len() <= 64
        && admissions.iter().all(|(id, slot)| {
            valid_id(id)
                && slot.path.len() <= 2048
                && !slot.path.contains(':')
                && !slot.title.trim().is_empty()
                && slot.title.len() <= 500
                && Path::new(&slot.path).components().all(|c| {
                    matches!(c, Component::Normal(_))
                        && !c
                            .as_os_str()
                            .to_string_lossy()
                            .eq_ignore_ascii_case(".weftext-trash")
                })
        })
        && admissions
            .values()
            .map(|s| s.path.replace('\\', "/").to_lowercase())
            .collect::<BTreeSet<_>>()
            .len()
            == admissions.len()
}
pub(super) fn valid_rights(config: &super::Config, rights: &Rights) -> bool {
    rights.directories.len() <= 64
        && rights.admit_directories.len() <= 64
        && rights
            .directories
            .iter()
            .all(|id| config.directories.contains_key(id))
        && rights
            .admit_directories
            .iter()
            .all(|id| config.navigation.directory_admissions.contains_key(id))
        && (!rights.tags.create && !rights.tags.rename || rights.tags.read)
}
fn rights<'a>(inner: &'a HostedInner, actor: &str) -> Result<&'a Rights, ApiError> {
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .map(|u| &u.navigation)
        .ok_or_else(expired)
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:navigation", inner.actor_scope(actor))
}
fn directory_path(inner: &HostedInner, relative: &str) -> PathBuf {
    // Joining an empty path adds a trailing separator. Core deliberately binds
    // a directory reservation's spelling to its marker's actual parent, so an
    // explicitly selected library root must retain the original root spelling.
    if relative.is_empty() {
        inner.config.root.clone()
    } else {
        inner.config.root.join(relative)
    }
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "navigationChanged",
        "原导航依据、配置或权限已变化；请保留输入并核对原操作。",
    )
}
fn core_error(_: CoreError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "navigationUnavailable",
        "当前原对象、注册表或原操作不可用；请保留输入并核对原操作状态。",
    )
}
fn valid_operation(id: &str) -> Result<(), ApiError> {
    if uuid::Uuid::parse_str(id)
        .is_ok_and(|u| u.to_string() == id && u.get_version() == Some(uuid::Version::Random))
    {
        Ok(())
    } else {
        Err(ApiError::bad("操作编号须为规范 UUID"))
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Intent {
    AdmitDirectory {
        slot: String,
    },
    CreateTag {
        basis: String,
        display_name: String,
    },
    RenameTag {
        basis: String,
        reference: String,
        display_name: String,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    operation_id: String,
    intent: Intent,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    operation_id: String,
    plan_id: String,
}
#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Resolve {
    Directory {
        directory: String,
    },
    AdmittedDirectory {
        slot: String,
        operation_id: String,
        reference: String,
    },
    Tag {
        reference: String,
    },
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    epoch: u64,
    expires: u64,
    binding: String,
    request: Prepare,
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn authorize(inner: &HostedInner, actor: &str, intent: &Intent) -> Result<(), ApiError> {
    let r = rights(inner, actor)?;
    match intent {
        Intent::AdmitDirectory { slot } => {
            if !r.admit_directories.contains(slot)
                || !inner
                    .config
                    .navigation
                    .directory_admissions
                    .contains_key(slot)
            {
                return Err(unavailable());
            }
        }
        Intent::CreateTag { display_name, .. } | Intent::RenameTag { display_name, .. } => {
            if !r.tags.read
                || match intent {
                    Intent::CreateTag { .. } => !r.tags.create,
                    _ => !r.tags.rename,
                }
            {
                return Err(unavailable());
            }
            if display_name.trim().is_empty()
                || display_name.chars().count() > 256
                || display_name.contains('\0')
            {
                return Err(ApiError::bad("标签显示名须为 1 至 256 个字符"));
            }
        }
    }
    Ok(())
}
fn any_access(inner: &HostedInner, actor: &str, write: bool) -> Result<(), ApiError> {
    let r = rights(inner, actor)?;
    let allowed = if write {
        !r.admit_directories.is_empty() || r.tags.read && (r.tags.create || r.tags.rename)
    } else {
        !r.directories.is_empty() || !r.admit_directories.is_empty() || r.tags.read
    };
    if allowed { Ok(()) } else { Err(unavailable()) }
}
fn binding(inner: &HostedInner, intent: &Intent) -> Result<String, ApiError> {
    let slot = match intent {
        Intent::AdmitDirectory { slot } => Some(
            inner
                .config
                .navigation
                .directory_admissions
                .get(slot)
                .ok_or_else(unavailable)?,
        ),
        _ => None,
    };
    Ok(digest(
        &serde_json::to_vec(
            &json!({"root":inner.config.root,"slot":slot,"tagRole":".weftext-tags.json"}),
        )
        .map_err(|_| changed())?,
    ))
}
fn tag_binding(inner: &HostedInner) -> Result<String, ApiError> {
    binding(
        inner,
        &Intent::CreateTag {
            basis: String::new(),
            display_name: String::new(),
        },
    )
}
fn release(inner: &mut HostedInner, basis: Basis) {
    let _ = inner.core.take_snapshot_witness(&basis.registry.base_id);
}
fn prune(inner: &mut HostedInner) {
    let binding = tag_binding(inner).unwrap_or_default();
    let expired = inner
        .navigation_registry
        .bases
        .iter()
        .filter(|(_, b)| {
            b.expires <= Instant::now()
                || b.binding != binding
                || inner
                    .policy
                    .users
                    .get(&b.actor)
                    .is_none_or(|u| !u.enabled || u.epoch != b.epoch || !u.navigation.tags.read)
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in expired {
        if let Some(b) = inner.navigation_registry.bases.remove(&id) {
            release(inner, b);
        }
    }
}
fn retain(
    inner: &mut HostedInner,
    actor: &str,
    registry: TagRegistrySnapshot,
    bytes: usize,
) -> Result<String, ApiError> {
    prune(inner);
    let total = inner
        .navigation_registry
        .bases
        .values()
        .map(|b| b.retained_bytes)
        .sum::<usize>();
    if inner.navigation_registry.bases.len() >= 128
        || inner
            .navigation_registry
            .bases
            .values()
            .filter(|b| b.actor == actor)
            .count()
            >= 16
        || total.saturating_add(bytes) > MAX_BYTES
    {
        let _ = inner.core.take_snapshot_witness(&registry.base_id);
        return Err(ApiError::bad(
            "标签注册表依据达到有界容量；请稍后重读，输入保留",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    inner.navigation_registry.bases.insert(
        id.clone(),
        Basis {
            actor: actor.into(),
            epoch: inner.actor_generation(actor),
            binding: tag_binding(inner)?,
            registry,
            retained_bytes: bytes,
            expires: Instant::now() + TTL,
        },
    );
    Ok(id)
}
/// Read only the authorized shared role. Measure its exact retained Core bytes
/// with a temporary explicit snapshot; both identities are checked before use.
fn tag_cut(inner: &mut HostedInner) -> Result<(TagRegistrySnapshot, usize), ApiError> {
    let root = inner.config.root.clone();
    let registry = inner.core.read_tag_registry(&root).map_err(core_error)?;
    let checked = (|| {
        let path = root
            .join(".weftext-tags.json")
            .to_string_lossy()
            .into_owned();
        let raw = inner
            .core
            .read_snapshot(&root, std::slice::from_ref(&path))
            .map_err(core_error)?
            .files
            .remove(0);
        let witness = inner
            .core
            .take_snapshot_witness(&raw.base_id)
            .map_err(core_error)?;
        let bytes = witness.retained_bytes().saturating_add(
            serde_json::to_vec(&registry.tags)
                .map_err(|_| changed())?
                .len(),
        );
        if bytes > MAX_REGISTRY {
            return Err(ApiError::bad("标签注册表超过本次 4 MiB 准确依据预算"));
        }
        inner
            .core
            .validate_snapshot(&root, &path, &registry.base_id)
            .map_err(core_error)?;
        Ok(bytes)
    })();
    match checked {
        Ok(bytes) => Ok((registry, bytes)),
        Err(error) => {
            let _ = inner.core.take_snapshot_witness(&registry.base_id);
            Err(error)
        }
    }
}
fn configured_directory(
    inner: &HostedInner,
    actor: &str,
    selector: &str,
) -> Result<Value, ApiError> {
    let r = rights(inner, actor)?;
    let mut matches = inner.config.directories.iter().filter(|(id, d)| {
        r.directories.contains(*id) && (*id == selector || d.reference == selector)
    });
    let (id, d) = matches.next().ok_or_else(unavailable)?;
    if matches.next().is_some() {
        return Err(unavailable());
    }
    let q = inner
        .core
        .qualified_query_scope_at(
            &inner.config.root,
            &directory_path(inner, &d.path),
            &d.reference,
            &d.lifecycle,
        )
        .map_err(|_| unavailable())?;
    inner
        .core
        .validate_query_scope_witness(&inner.config.root, &q.witness)
        .map_err(|_| unavailable())?;
    Ok(
        json!({"kind":"directory","directory":id,"title":d.title,"reference":q.target_ref,"status":"available","readOnly":true}),
    )
}
fn tag_target(inner: &mut HostedInner, actor: &str, reference: &str) -> Result<Value, ApiError> {
    if !rights(inner, actor)?.tags.read {
        return Err(unavailable());
    }
    let (registry, _) = tag_cut(inner)?;
    let result = registry.tags.iter().find(|tag| tag.tag_ref == reference)
        .map(|tag| json!({"kind":"tag","title":tag.display_name,"reference":tag.tag_ref,"status":"available","readOnly":true}))
        .ok_or_else(unavailable);
    let _ = inner.core.take_snapshot_witness(&registry.base_id);
    result
}
fn decision(inner: &HostedInner, actor: &str, status: &ActorFeatureStatus) -> Option<Decision> {
    let d: Decision = serde_json::from_value(status.preview["_navigation"].clone()).ok()?;
    (d.epoch == inner.actor_generation(actor)
        && d.request.operation_id == status.operation_id
        && authorize(inner, actor, &d.request.intent).is_ok()
        && binding(inner, &d.request.intent).is_ok_and(|b| b == d.binding))
    .then_some(d)
}
fn receipt_target(
    inner: &mut HostedInner,
    actor: &str,
    status: &ActorFeatureStatus,
    d: &Decision,
) -> Result<Value, ApiError> {
    if status.status != "committed" {
        return Err(unavailable());
    }
    let reference = status.navigation_ref.as_ref().ok_or_else(unavailable)?;
    match &d.request.intent {
        Intent::AdmitDirectory { slot } => {
            let target = inner
                .config
                .navigation
                .directory_admissions
                .get(slot)
                .ok_or_else(unavailable)?;
            let q = inner
                .core
                .qualified_query_scope_at(
                    &inner.config.root,
                    &directory_path(inner, &target.path),
                    reference,
                    "live",
                )
                .map_err(|_| unavailable())?;
            inner
                .core
                .validate_query_scope_witness(&inner.config.root, &q.witness)
                .map_err(|_| unavailable())?;
            Ok(
                json!({"kind":"directory","slot":slot,"title":target.title,"reference":q.target_ref,"status":"available","canResolve":true,"readOnly":true}),
            )
        }
        _ => {
            let mut result = tag_target(inner, actor, reference)?;
            result["canResolve"] = true.into();
            Ok(result)
        }
    }
}
fn safe_status(
    inner: &mut HostedInner,
    actor: &str,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let d = decision(inner, actor, &status);
    let mut preview = json!({});
    if d.is_some() {
        for key in ["kind", "title", "displayName", "beforeName"] {
            if let Some(value) = status.preview.get(key) {
                preview[key] = value.clone();
            }
        }
    }
    let allowed = status.status == "prepared" && d.as_ref().is_some_and(|d| d.expires > now());
    let result = if let Some(d) = &d {
        receipt_target(inner, actor, &status, d).ok()
    } else {
        None
    };
    Ok(
        json!({"operationId":status.operation_id,"status":status.status,"planId":status.plan_id.filter(|_|allowed || status.status=="committed" && d.is_some()),"preview":preview,"commitAllowed":allowed,"navigationRef":status.navigation_ref.filter(|_|d.is_some()),"result":result}),
    )
}
fn missing(id: &str) -> Value {
    json!({"operationId":id,"status":"notRegistered","planId":null,"preview":{},"commitAllowed":false,"navigationRef":null,"result":null})
}
fn build(inner: &mut HostedInner, actor: &str, req: Prepare) -> Result<Value, ApiError> {
    valid_operation(&req.operation_id)?;
    authorize(inner, actor, &req.intent)?;
    let bytes = serde_json::to_vec(&req).map_err(|_| changed())?;
    if bytes.len() > 256 * 1024 {
        return Err(ApiError::bad(
            "导航提案超过 256 KiB 输入预算；请保留输入并缩小提案",
        ));
    }
    let fingerprint = digest(&bytes);
    let scope = scope(inner, actor);
    if let Some(old) = inner
        .core
        .with_actor(&scope, |c| {
            c.actor_feature_retry(RESOURCE, &req.operation_id, &fingerprint)
        })
        .map_err(core_error)?
    {
        return safe_status(inner, actor, old);
    }
    prune(inner);
    let (intent, mut preview) = match &req.intent {
        Intent::AdmitDirectory { slot } => {
            let selected = &inner.config.navigation.directory_admissions[slot];
            (
                HostedNavigationIntent::AdmitDirectory {
                    path: directory_path(inner, &selected.path)
                        .to_string_lossy()
                        .into_owned(),
                },
                json!({"kind":"admitDirectory","title":selected.title}),
            )
        }
        Intent::CreateTag {
            basis,
            display_name,
        }
        | Intent::RenameTag {
            basis,
            display_name,
            ..
        } => {
            let cut = inner
                .navigation_registry
                .bases
                .get(basis)
                .filter(|b| {
                    b.actor == actor
                        && b.epoch == inner.actor_generation(actor)
                        && b.expires > Instant::now()
                        && b.binding == tag_binding(inner).unwrap_or_default()
                        && b.registry.complete
                })
                .cloned()
                .ok_or_else(changed)?;
            inner
                .core
                .validate_snapshot(
                    &inner.config.root,
                    inner.config.root.join(".weftext-tags.json"),
                    &cut.registry.base_id,
                )
                .map_err(core_error)?;
            match &req.intent {
                Intent::RenameTag { reference, .. } => {
                    let tag = cut
                        .registry
                        .tags
                        .iter()
                        .find(|t| &t.tag_ref == reference)
                        .ok_or_else(unavailable)?;
                    (
                        HostedNavigationIntent::RenameTag {
                            reference: reference.clone(),
                            display_name: display_name.clone(),
                            base_id: cut.registry.base_id,
                        },
                        json!({"kind":"renameTag","displayName":display_name,"beforeName":tag.display_name}),
                    )
                }
                _ => (
                    HostedNavigationIntent::CreateTag {
                        display_name: display_name.clone(),
                        base_id: cut.registry.base_id,
                    },
                    json!({"kind":"createTag","displayName":display_name}),
                ),
            }
        }
    };
    preview["_navigation"] = serde_json::to_value(Decision {
        epoch: inner.actor_generation(actor),
        expires: now() + TTL.as_secs(),
        binding: binding(inner, &req.intent)?,
        request: req.clone(),
    })
    .map_err(|_| changed())?;
    let root = inner.config.root.clone();
    let status = inner
        .core
        .with_actor(&scope, |c| {
            c.actor_feature_prepare_navigation(
                RESOURCE,
                &req.operation_id,
                &fingerprint,
                &root,
                intent,
                preview,
            )
        })
        .map_err(core_error)?;
    safe_status(inner, actor, status)
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/navigation/registry", get(catalog))
        .route("/api/hosted/navigation/registry/prepare", post(prepare))
        .route("/api/hosted/navigation/registry/commit", post(commit))
        .route("/api/hosted/navigation/registry/operation", post(operation))
        .route("/api/hosted/navigation/registry/settle", post(settle))
        .route("/api/hosted/navigation/registry/resolve", post(resolve))
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        let r = rights(inner, &actor)?.clone();
        prune(inner);
        let mut partial = false;
        let directories = r.directories.iter().filter_map(|id| {
            let d = inner.config.directories.get(id)?;
            let ready = configured_directory(inner, &actor, id).is_ok();
            partial |= !ready;
            Some(json!({"directory":id,"title":d.title,"reference":d.reference,"status":if ready{"available"}else{"unavailable"},"canResolve":ready}))
        }).collect::<Vec<_>>();
        let admission_slots = r.admit_directories.iter().filter_map(|id| inner.config.navigation.directory_admissions.get(id).map(|s|json!({"slot":id,"title":s.title}))).collect::<Vec<_>>();
        let tags = if r.tags.read {
            match tag_cut(inner) {
                Ok((registry, bytes)) => {
                    let items = registry.tags.iter().map(|tag|json!({"reference":tag.tag_ref,"displayName":tag.display_name})).collect::<Vec<_>>();
                    let complete = registry.complete;
                    let basis = if complete && (r.tags.create || r.tags.rename) { Some(retain(inner, &actor, registry, bytes)?) }
                        else { let _ = inner.core.take_snapshot_witness(&registry.base_id); None };
                    json!({"read":true,"create":r.tags.create,"rename":r.tags.rename,"coverage":if complete{"complete"}else{"partial"},"items":items,"basis":basis})
                }
                Err(_) => json!({"read":true,"create":r.tags.create,"rename":r.tags.rename,"coverage":"unavailable","items":[],"basis":null}),
            }
        } else { json!({"read":false,"create":false,"rename":false,"coverage":"unavailable","items":[],"basis":null}) };
        Ok(json!({"directoryScope":"configuredAuthorizedDirectories","directoryCoverage":if partial{"partial"}else{"complete"},"directories":directories,"admissionSlots":admission_slots,"tags":tags}))
    }).await?))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            build(inner, &actor, req)
        })
        .await?,
    ))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            any_access(inner, &actor, false)?;
            valid_operation(&req.operation_id)?;
            let old = inner
                .core
                .with_actor(&scope(inner, &actor), |c| {
                    c.actor_feature_status(RESOURCE, &req.operation_id)
                })
                .map_err(core_error)?;
            match old {
                Some(status) => safe_status(inner, &actor, status),
                None => Ok(missing(&req.operation_id)),
            }
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
            any_access(inner, &actor, true)?;
            valid_operation(&req.operation_id)?;
            let old = inner
                .core
                .with_actor(&scope(inner, &actor), |c| {
                    c.actor_feature_settle(RESOURCE, &req.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, old)
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
            any_access(inner, &actor, true)?;
            valid_operation(&req.operation_id)?;
            valid_operation(&req.plan_id)?;
            let actor_scope = scope(inner, &actor);
            let old = inner
                .core
                .with_actor(&actor_scope, |c| {
                    c.actor_feature_status(RESOURCE, &req.operation_id)
                })
                .map_err(core_error)?
                .ok_or_else(changed)?;
            if old.status == "prepared" {
                let d = decision(inner, &actor, &old).ok_or_else(changed)?;
                if d.expires <= now() || old.plan_id.as_deref() != Some(req.plan_id.as_str()) {
                    return Err(changed());
                }
            }
            let old = inner
                .core
                .with_actor(&actor_scope, |c| {
                    c.actor_feature_commit(RESOURCE, &req.operation_id, &req.plan_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, old)
        })
        .await?,
    ))
}
async fn resolve(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Resolve>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?;
        match req {
            Resolve::Directory { directory } => configured_directory(inner, &actor, &directory),
            Resolve::Tag { reference } => tag_target(inner, &actor, &reference),
            Resolve::AdmittedDirectory { slot, operation_id, reference } => {
                authorize(inner, &actor, &Intent::AdmitDirectory { slot: slot.clone() })?;
                valid_operation(&operation_id)?;
                let status = inner.core.with_actor(&scope(inner, &actor), |c|c.actor_feature_status(RESOURCE, &operation_id)).map_err(|_| unavailable())?.ok_or_else(unavailable)?;
                let d = decision(inner, &actor, &status).filter(|d|matches!(&d.request.intent,Intent::AdmitDirectory{slot:original} if original==&slot)).ok_or_else(unavailable)?;
                if status.navigation_ref.as_ref() != Some(&reference) { return Err(unavailable()); }
                receipt_target(inner, &actor, &status, &d)
            }
        }
    }).await?))
}

#[cfg(all(test, windows))]
#[path = "navigation_registry_tests.rs"]
mod tests;
