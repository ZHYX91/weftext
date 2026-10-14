//! Explicit library roles and one frozen native style environment.
use super::*;
use weftext_core::{
    ActorFeatureStatus, ConfigDocument, ConfigRole, FileChange, LibraryStyleDocument,
    LibraryStyleWitness, OperationIntent, OperationRequest, SnapshotWitness,
};

const TTL: Duration = Duration::from_secs(900);
const MAX_SOURCE: usize = 2 * 1024 * 1024;
const MAX_CACHE: usize = 32 * 1024 * 1024;
const ROLES: [&str; 4] = ["schema", "settings", "rules", "style"];

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RoleRights {
    #[serde(default)]
    read: bool,
    #[serde(default)]
    write: bool,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    schema: RoleRights,
    #[serde(default)]
    settings: RoleRights,
    #[serde(default)]
    rules: RoleRights,
    #[serde(default)]
    style: RoleRights,
    #[serde(default)]
    pub(super) style_use: bool,
}
impl Rights {
    fn role(&self, role: &str) -> Option<&RoleRights> {
        match role {
            "schema" => Some(&self.schema),
            "settings" => Some(&self.settings),
            "rules" => Some(&self.rules),
            "style" => Some(&self.style),
            _ => None,
        }
    }
}
pub(super) fn valid_rights(rights: &Rights) -> bool {
    ROLES
        .iter()
        .all(|r| rights.role(r).is_some_and(|p| !p.write || p.read))
}
#[derive(Clone)]
enum Document {
    Config(ConfigDocument),
    Style(LibraryStyleDocument),
}
#[derive(Clone)]
struct Basis {
    actor: String,
    epoch: u64,
    role: String,
    config_hash: String,
    document: Document,
    source: String,
    retained_bytes: usize,
    expires: Instant,
}
#[derive(Default)]
pub(super) struct Store {
    bases: HashMap<String, Basis>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    role: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    role: String,
    basis: String,
    operation_id: String,
    source: Option<String>,
    changes: Option<BTreeMap<String, String>>,
    enabled: Option<bool>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Operation {
    role: String,
    operation_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Commit {
    role: String,
    operation_id: String,
    plan_id: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Decision {
    actor_epoch: u64,
    config_hash: String,
    workspace: String,
    expires_at: u64,
    style_write: bool,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/library", get(catalog))
        .route("/api/hosted/library/read", post(read))
        .route("/api/hosted/library/prepare", post(prepare))
        .route("/api/hosted/library/commit", post(commit))
        .route("/api/hosted/library/operation", post(operation))
        .route("/api/hosted/library/settle", post(settle))
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "libraryBasisChanged",
        "库配置、样式或原授权依据已变化；请保留输入并核对原操作。",
    )
}
fn core_error(error: CoreError) -> ApiError {
    let message = match error.code.as_str() {
        "operationConflict" => "同一原操作编号不能用于另一角色或不同输入。",
        "rulesUnavailable" => {
            "已提交的库规则文件缺失，不能用缺省值重新安装；请由库所有者核对原操作恢复。"
        }
        "basisRequired" => "原库配置依据不可用，请重新读取并保留输入。",
        _ => "原库配置操作未完成或依据已变化；输入保持保留，请核对原操作。",
    };
    ApiError::new(StatusCode::CONFLICT, &error.code, message)
}
fn rights<'a>(inner: &'a HostedInner, actor: &str) -> Result<&'a Rights, ApiError> {
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .map(|u| &u.library)
        .ok_or_else(unavailable)
}
fn access(inner: &HostedInner, actor: &str, role: &str, write: bool) -> Result<(), ApiError> {
    rights(inner, actor)?
        .role(role)
        .filter(|r| r.read && (!write || r.write))
        .map(|_| ())
        .ok_or_else(unavailable)
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:library", inner.actor_scope(actor))
}
fn resource(role: &str) -> String {
    format!("library_{role}")
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn config_hash(inner: &HostedInner) -> Result<String, ApiError> {
    let inputs = inputs::binding_hash(inner, &inner.config.style_inputs.iter().cloned().collect())?;
    Ok(digest(
        &serde_json::to_vec(
            &json!({"root":inner.config.root,"deployment":inner.config.deployment,"inputs":inputs}),
        )
        .map_err(|_| changed())?,
    ))
}
fn config_role(role: &str) -> Result<ConfigRole, ApiError> {
    match role {
        "schema" => Ok(ConfigRole::Schema),
        "settings" => Ok(ConfigRole::WorkspaceSettings),
        "rules" => Ok(ConfigRole::Rules),
        _ => Err(unavailable()),
    }
}
fn source(document: &ConfigDocument, role: &str) -> Result<String, ApiError> {
    match &document.bytes {
        Some(bytes) => String::from_utf8(bytes.clone()).map_err(|_| changed()),
        None => Ok(match role {
            "schema" => {
                include_str!("../../../../docs/design/examples/schema.authoring.json").to_owned()
            }
            "rules" => "{\"version\":1,\"ignored\":[],\"unmanaged\":[]}".to_owned(),
            _ => "{\"version\":1}".to_owned(),
        }),
    }
}
fn release(inner: &mut HostedInner, document: Document) {
    match document {
        Document::Style(d) => {
            let _ = inner.core.discard_library_style_basis(&d.base_id);
        }
        Document::Config(d) => {
            let _ = inner.core.take_snapshot_witness(&d.base_id);
        }
    }
}
fn prune(inner: &mut HostedInner) {
    let expired = inner
        .library
        .bases
        .iter()
        .filter(|(_, b)| {
            b.expires <= Instant::now()
                || b.epoch != inner.actor_generation(&b.actor)
                || access(inner, &b.actor, &b.role, false).is_err()
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in expired {
        if let Some(b) = inner.library.bases.remove(&id) {
            release(inner, b.document);
        }
    }
}
fn retain(inner: &mut HostedInner, basis: Basis) -> Result<String, ApiError> {
    prune(inner);
    let bytes = basis.retained_bytes;
    let total = inner
        .library
        .bases
        .values()
        .map(|b| b.retained_bytes)
        .sum::<usize>();
    if inner.library.bases.len() >= 128
        || inner
            .library
            .bases
            .values()
            .filter(|b| b.actor == basis.actor)
            .count()
            >= 32
        || bytes > MAX_CACHE
        || total.saturating_add(bytes) > MAX_CACHE
    {
        release(inner, basis.document);
        return Err(ApiError::bad(
            "库配置读取依据已达有界容量，请稍后重新读取；输入仍保留",
        ));
    }
    let id = uuid::Uuid::new_v4().to_string();
    inner.library.bases.insert(id.clone(), basis);
    Ok(id)
}
fn workspace(inner: &mut HostedInner) -> Result<String, ApiError> {
    let doc = inner
        .core
        .read_config(&inner.config.root, ConfigRole::WorkspaceSettings)
        .map_err(core_error)?;
    let _ = inner
        .core
        .take_snapshot_witness(&doc.base_id)
        .map_err(core_error)?;
    Ok(doc.role_ref)
}
fn status(
    inner: &mut HostedInner,
    actor: &str,
    role: &str,
    old: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let decision = serde_json::from_value::<Decision>(old.preview["_library"].clone()).ok();
    let same = decision.as_ref().is_some_and(|d| {
        d.actor_epoch == inner.actor_generation(actor)
            && config_hash(inner).is_ok_and(|h| h == d.config_hash)
    });
    let same = same
        && decision
            .as_ref()
            .is_some_and(|d| workspace(inner).is_ok_and(|w| w == d.workspace));
    let mut preview = json!({});
    if same {
        for key in ["beforeSource", "afterSource", "summary", "enabled"] {
            if let Some(v) = old.preview.get(key) {
                preview[key] = v.clone();
            }
        }
    }
    let writable = access(inner, actor, role, true).is_ok()
        && decision
            .as_ref()
            .is_none_or(|d| !d.style_write || access(inner, actor, "style", true).is_ok());
    let allowed = old.status == "prepared"
        && same
        && writable
        && decision.as_ref().is_some_and(|d| d.expires_at > now());
    // Committed facts survive expiry, but a stale authorization never revives a plan.
    Ok(
        json!({"role":role,"operationId":old.operation_id,"status":old.status,"planId":old.plan_id.filter(|_|allowed),"preview":preview,"commitAllowed":allowed}),
    )
}

async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {let actor=inner.actor(&auth)?;let r=rights(inner,&actor)?;
        let roles=ROLES.iter().filter_map(|role|r.role(role).filter(|g|g.read).map(|g|json!({"role":role,"title":match *role {"schema"=>"Schema","settings"=>"库设置","rules"=>"库规则",_=>"库样式"},"read":g.read,"write":g.write}))).collect::<Vec<_>>();
        Ok(json!({"roles":roles,"styleUse":r.style_use}))}).await?))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Read>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {let actor=inner.actor(&auth)?;access(inner,&actor,&req.role,false)?;prune(inner);let config_hash=config_hash(inner)?;
        let (document,source,mut result)=if req.role=="style" {
            let doc=inner.core.read_library_style(&inner.config.root).map_err(core_error)?;
            let inspect=weftext_engine::inspect_library_style(&doc.source);
            let controls=inspect.controls.iter().map(|c|json!({"key":c.key,"label":c.label,"kind":"text","value":c.value,"editable":c.editable,"reason":c.reason})).collect::<Vec<_>>();
            let result=json!({"role":req.role,"source":doc.source,"installed":doc.installed,"enabled":doc.enabled,"controls":controls,"warnings":if inspect.syntax_valid {vec![]} else {vec!["样式源码存在语法错误；请保留原文修复。"]}});
            let source=doc.source.clone();(Document::Style(doc),source,result)
        } else {
            let doc=inner.core.read_config(&inner.config.root,config_role(&req.role)?).map_err(core_error)?;
            let source=match source(&doc,&req.role) {Ok(source)=>source,Err(error)=>{release(inner,Document::Config(doc));return Err(error);}};
            let result=json!({"role":req.role,"source":source,"installed":doc.bytes.is_some()});(Document::Config(doc),source,result)
        };
        // ConfigDocument also retains one private Core snapshot; StyleBasis
        // has four exact controls, not merely the displayed module text.
        let retained_bytes=match &document {Document::Config(d)=>2*d.bytes.as_ref().map_or(0,Vec::len)+source.len(),Document::Style(d)=>match inner.core.library_style_basis_retained_bytes(&d.base_id) {Ok(bytes)=>bytes+2*source.len(),Err(error)=>{release(inner,document);return Err(core_error(error));}}};
        let basis=Basis {actor:actor.clone(),epoch:inner.actor_generation(&actor),role:req.role.clone(),config_hash,document,source,retained_bytes,expires:Instant::now()+TTL};
        result["basis"]=retain(inner,basis)?.into();Ok(result)
    }).await?))
}
fn validate_config(role: &str, before: &str, after: &str) -> Result<bool, ApiError> {
    if after.len() > MAX_SOURCE {
        return Err(ApiError::bad("配置或样式源码上限为 2 MiB；输入仍保留"));
    }
    if role == "schema" {
        weftext_data::Schema::from_author_json(after)
            .map_err(|_| ApiError::bad("Schema 格式或约束无效；输入仍保留"))?;
        return Ok(false);
    }
    if role == "rules" {
        weftext_core::validate_library_rules(after.as_bytes()).map_err(|_| {
            ApiError::bad("库规则格式无效；请使用 version:1、ignored 和 unmanaged 原生路径列表")
        })?;
        return Ok(false);
    }
    let parse = |s: &str| {
        serde_json::from_str::<Value>(s)
            .ok()
            .filter(|v| v.is_object() && v["version"] == 1)
            .ok_or_else(|| ApiError::bad("库设置须为原生 version:1 JSON 对象"))
    };
    let after = parse(after)?;
    let before = parse(before)?;
    Ok(before.get("defaultStyle") != after.get("defaultStyle"))
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {let actor=inner.actor(&auth)?;access(inner,&actor,&req.role,true)?;
        let fingerprint=digest(&serde_json::to_vec(&req).map_err(|_|changed())?);let actor_scope=scope(inner,&actor);let resource=resource(&req.role);
        if let Some(old)=inner.core.with_actor(&actor_scope,|c|c.actor_feature_retry(&resource,&req.operation_id,&fingerprint)).map_err(core_error)? {return status(inner,&actor,&req.role,old);}
        prune(inner);let basis=inner.library.bases.get(&req.basis).filter(|b|b.actor==actor && b.role==req.role && b.epoch==inner.actor_generation(&actor) && b.config_hash==config_hash(inner).unwrap_or_default()).cloned().ok_or_else(changed)?;
        if req.source.is_some() && req.changes.is_some() {return Err(ApiError::bad("源码和控件修改须分别准备"));}
        let root=inner.config.root.clone();
        let (after,enabled,intent,style_write)=match &basis.document {
            Document::Style(doc)=>{
                inner.core.validate_library_style(&root,&doc.base_id).map_err(core_error)?;
                let after=match &req.source {Some(s)=>s.clone(),None=>weftext_engine::apply_library_style_changes(&basis.source,&weftext_engine::source_hash(&basis.source),&req.changes.clone().unwrap_or_default()).map_err(|_|ApiError::bad("样式控件不能无损修改此原生表达式；请保留源码"))?.source};
                if after.len()>MAX_SOURCE || !weftext_engine::inspect_library_style(&after).syntax_valid {return Err(ApiError::bad("样式源码超出预算或语法无效；输入保持保留"));}
                let enabled=req.enabled.unwrap_or(doc.enabled);let intent=OperationIntent::ConfigureStyle {base_id:doc.base_id.clone(),source:after.clone(),enabled};(after,Some(enabled),intent,false)
            },
            Document::Config(doc)=>{
                if req.changes.is_some() || req.enabled.is_some() {return Err(ApiError::bad("此配置角色只接受原始 JSON 源码"));}
                inner.core.validate_snapshot(&root,&doc.path,&doc.base_id).map_err(core_error)?;
                let after=req.source.clone().ok_or_else(||ApiError::bad("缺少配置源码"))?;let extra=validate_config(&req.role,&basis.source,&after)?;
                if extra {access(inner,&actor,"style",true)?;}
                let intent=OperationIntent::UpdateFiles {changes:vec![FileChange {path:doc.path.clone(),base_id:Some(doc.base_id.clone()),before:doc.bytes.clone(),after:after.as_bytes().to_vec()}]};(after,None,intent,extra)
            }
        };
        let decision=Decision {actor_epoch:inner.actor_generation(&actor),config_hash:config_hash(inner)?,workspace:workspace(inner)?,expires_at:now()+TTL.as_secs(),style_write};
        let mut preview=json!({"beforeSource":basis.source,"afterSource":after,"summary":"按原准确依据应用此库配置；不会改写正文或个人草稿。","_library":decision});
        if let Some(enabled)=enabled {preview["enabled"]=enabled.into();}
        // Core's durable preview budget applies before creating an original ID.
        if serde_json::to_vec(&preview).map_err(|_|changed())?.len()>MAX_SOURCE {return Err(ApiError::bad("本次前后源码预览超出 2 MiB 原操作预算；输入仍保留"));}
        let request=OperationRequest {operation_id:req.operation_id.clone(),root:root.to_string_lossy().into_owned(),intent};
        let old=inner.core.with_actor(&actor_scope,|c|c.actor_feature_prepare(&resource,&req.operation_id,&fingerprint,request,preview,&[])).map_err(core_error)?;
        status(inner,&actor,&req.role,old)
    }).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {let actor=inner.actor(&auth)?;access(inner,&actor,&req.role,false)?;
        let old=inner.core.with_actor(&scope(inner,&actor),|c|c.actor_feature_status(&resource(&req.role),&req.operation_id)).map_err(core_error)?;
        match old {Some(old)=>status(inner,&actor,&req.role,old),None=>Ok(json!({"role":req.role,"operationId":req.operation_id,"status":"notRegistered","planId":null,"preview":{},"commitAllowed":false}))}
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
            access(inner, &actor, &req.role, true)?;
            let actor_scope = scope(inner, &actor);
            let resource = resource(&req.role);
            let old = inner
                .core
                .with_actor(&actor_scope, |c| {
                    c.actor_feature_status(&resource, &req.operation_id)
                })
                .map_err(core_error)?
                .ok_or_else(changed)?;
            if old.status != "committed" {
                let safe = status(inner, &actor, &req.role, old)?;
                if safe["commitAllowed"] != true || safe["planId"] != req.plan_id {
                    return Err(changed());
                }
            }
            let old = inner
                .core
                .with_actor(&actor_scope, |c| {
                    c.actor_feature_commit(&resource, &req.operation_id, &req.plan_id)
                })
                .map_err(core_error)?;
            status(inner, &actor, &req.role, old)
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
            access(inner, &actor, &req.role, true)?;
            let old = inner
                .core
                .with_actor(&scope(inner, &actor), |c| {
                    c.actor_feature_settle(&resource(&req.role), &req.operation_id)
                })
                .map_err(core_error)?;
            status(inner, &actor, &req.role, old)
        })
        .await?,
    ))
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExecutionAuthority {
    pub(super) inputs: BTreeSet<String>,
    pub(super) style_used: bool,
    #[serde(default)]
    config_hash: String,
    #[serde(default)]
    input_binding: String,
    #[serde(default)]
    resource_revisions: BTreeMap<String, Option<String>>,
    #[serde(default)]
    collection_revisions: BTreeMap<String, Option<String>>,
}
enum StyleEvidence {
    Disabled {
        path: String,
        witness: SnapshotWitness,
    },
    Enabled(Box<LibraryStyleWitness>),
}
struct Evidence {
    style: StyleEvidence,
}
#[derive(Clone)]
pub(super) struct Environment {
    pub(super) authority: ExecutionAuthority,
    module: Option<weftext_engine::StyleModule>,
    files: BTreeMap<String, String>,
    evidence: Arc<Evidence>,
    input_sets: Vec<inputs::Captured>,
    bytes: usize,
}
pub(super) fn authorize_execution(
    inner: &HostedInner,
    actor: &str,
    authority: &ExecutionAuthority,
) -> Result<(), ApiError> {
    if !authority.config_hash.is_empty() && config_hash(inner)? != authority.config_hash {
        return Err(changed());
    }
    if authority.style_used && !rights(inner, actor)?.style_use {
        return Err(unavailable());
    }
    for id in &authority.inputs {
        inputs::access(inner, actor, id)?;
    }
    if !authority.input_binding.is_empty()
        && inputs::binding_hash(inner, &authority.inputs)? != authority.input_binding
    {
        return Err(changed());
    }
    Ok(())
}
/// A prepared author effect must still refer to its original portable resource
/// versions. Already-frozen output delivery only rechecks authority/bindings.
pub(super) fn validate_execution_versions(
    inner: &HostedInner,
    actor: &str,
    authority: &ExecutionAuthority,
) -> Result<(), ApiError> {
    authorize_execution(inner, actor, authority)?;
    for (id, revision) in &authority.collection_revisions {
        if resource_collections::capture(inner, actor, id)?.revision != *revision {
            return Err(changed());
        }
    }
    for (id, revision) in &authority.resource_revisions {
        if media::capture_input(inner, actor, id)?.revision != *revision {
            return Err(changed());
        }
    }
    Ok(())
}
impl Environment {
    pub(super) fn attach_inputs(
        &mut self,
        inner: &HostedInner,
        actor: &str,
        captured: inputs::Captured,
    ) -> Result<(), ApiError> {
        captured.validate(inner, actor)?;
        for (id, revision) in captured.revisions() {
            if self
                .authority
                .resource_revisions
                .get(&id)
                .is_some_and(|old| old != &revision)
            {
                return Err(changed());
            }
            self.authority.resource_revisions.insert(id, revision);
        }
        for (id, revision) in captured.collection_revisions() {
            if self
                .authority
                .collection_revisions
                .get(&id)
                .is_some_and(|old| old != &revision)
            {
                return Err(changed());
            }
            self.authority.collection_revisions.insert(id, revision);
        }
        self.authority.inputs.extend(captured.authority_ids());
        self.authority.input_binding = inputs::binding_hash(inner, &self.authority.inputs)?;
        self.bytes = self.bytes.saturating_add(captured.retained_bytes());
        if self.bytes > MAX_CACHE {
            return Err(ApiError::bad("编译环境与原输入依据超出 32 MiB 预算"));
        }
        self.input_sets.push(captured);
        Ok(())
    }
    pub(super) fn apply(&self, worker: &mut WorkerRequest) -> Result<(), ApiError> {
        let files = worker.frozen_files.get_or_insert_with(BTreeMap::new);
        for (path, bytes) in &self.files {
            if files.get(path).is_some_and(|old| old != bytes) {
                return Err(changed());
            }
            files.insert(path.clone(), bytes.clone());
        }
        worker.style = self.module.clone();
        let bytes = worker
            .source
            .len()
            .saturating_add(worker.style.as_ref().map_or(0, |s| s.source.len()))
            .saturating_add(files.values().map(String::len).sum::<usize>())
            .saturating_add(self.retained_bytes());
        if files.len() > 64 || bytes > MAX_CACHE {
            return Err(ApiError::bad(
                "本次编译环境与正文输入合计超出 32 MiB 或 64 个输入的预算",
            ));
        }
        Ok(())
    }
    pub(super) fn recheck(&self, inner: &HostedInner, actor: &str) -> Result<(), ApiError> {
        authorize_execution(inner, actor, &self.authority)?;
        match &self.evidence.style {
            StyleEvidence::Disabled { path, witness } => {
                inner
                    .core
                    .validate_snapshot_witness(&inner.config.root, path, witness)
            }
            StyleEvidence::Enabled(w) => inner
                .core
                .validate_library_style_witness(&inner.config.root, w),
        }
        .map_err(core_error)?;
        for captured in &self.input_sets {
            captured.validate(inner, actor)?;
        }
        Ok(())
    }
    pub(super) fn retained_bytes(&self) -> usize {
        self.bytes
    }
    pub(super) fn plan_witnesses(&self) -> Result<Vec<SnapshotWitness>, ApiError> {
        let mut pool = vec![];
        for captured in &self.input_sets {
            Core::merge_snapshot_witnesses(&mut pool, captured.plan_witnesses()?)
                .map_err(core_error)?;
        }
        match &self.evidence.style {
            StyleEvidence::Disabled { witness, .. } => {
                Core::merge_snapshot_witnesses(&mut pool, vec![witness.clone()])
            }
            StyleEvidence::Enabled(w) => Core::merge_library_style_witness(&mut pool, w),
        }
        .map_err(core_error)?;
        Ok(pool)
    }
}
pub(super) fn environment(inner: &mut HostedInner, actor: &str) -> Result<Environment, ApiError> {
    let root = inner.config.root.clone();
    let config_hash = config_hash(inner)?;
    let settings = inner
        .core
        .read_config(&root, ConfigRole::WorkspaceSettings)
        .map_err(core_error)?;
    let witness = inner
        .core
        .take_snapshot_witness(&settings.base_id)
        .map_err(core_error)?;
    let value = settings
        .bytes
        .as_deref()
        .map(serde_json::from_slice::<Value>)
        .transpose()
        .map_err(|_| changed())?
        .unwrap_or_else(|| json!({"version":1}));
    let enabled = value
        .get("defaultStyle")
        .and_then(|s| s.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !enabled {
        let style = if settings.bytes.is_none() {
            // Genuine absence is a read-only decision and a negative original
            // feature-plan dependency, never a positive saved-definition guard.
            let doc = inner.core.read_library_style(&root).map_err(core_error)?;
            let style_witness = inner
                .core
                .take_library_style_witness(&doc.base_id)
                .map_err(core_error)?;
            inner
                .core
                .validate_snapshot_witness(&root, &settings.path, &witness)
                .map_err(core_error)?;
            StyleEvidence::Enabled(Box::new(style_witness))
        } else {
            StyleEvidence::Disabled {
                path: settings.path,
                witness,
            }
        };
        let bytes = match &style {
            StyleEvidence::Disabled { witness, .. } => witness.retained_bytes(),
            StyleEvidence::Enabled(w) => w.retained_bytes(),
        };
        if bytes > MAX_CACHE {
            return Err(ApiError::bad("编译环境超出 32 MiB 输入预算"));
        }
        return Ok(Environment {
            authority: ExecutionAuthority {
                config_hash,
                ..Default::default()
            },
            module: None,
            files: BTreeMap::new(),
            evidence: Arc::new(Evidence { style }),
            input_sets: vec![],
            bytes,
        });
    }
    if !rights(inner, actor)?.style_use {
        return Err(unavailable());
    }
    let doc = inner.core.read_library_style(&root).map_err(core_error)?;
    let style_witness = inner
        .core
        .take_library_style_witness(&doc.base_id)
        .map_err(core_error)?;
    if !doc.installed || !doc.enabled {
        return Err(changed());
    }
    inner
        .core
        .validate_snapshot_witness(&root, &settings.path, &witness)
        .map_err(core_error)?;
    let actual = PathBuf::from(&doc.path);
    let relative = actual
        .strip_prefix(&root)
        .map_err(|_| changed())?
        .to_string_lossy()
        .replace('\\', "/");
    let private = !rights(inner, actor)?.style.read;
    let module_path = if private {
        actual
            .parent()
            .ok_or_else(changed)?
            .join(format!(".hosted-style-{}.typ", uuid::Uuid::new_v4()))
            .strip_prefix(&root)
            .map_err(|_| changed())?
            .to_string_lossy()
            .replace('\\', "/")
    } else {
        relative.clone()
    };
    let mut files = BTreeMap::new();
    let input_ids = inner
        .config
        .style_inputs
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let captured = inputs::capture(inner, actor, &input_ids.iter().cloned().collect::<Vec<_>>())?;
    let mut bytes = style_witness
        .retained_bytes()
        .saturating_add(doc.source.len())
        .saturating_add(captured.retained_bytes());
    for id in captured.ids() {
        let (path, content, _) = captured.entry(id)?.ok_or_else(changed)?;
        bytes = bytes.saturating_add(content.len().div_ceil(3) * 4);
        if bytes > MAX_CACHE {
            return Err(ApiError::bad("编译环境超出 32 MiB 输入预算；输入仍保留"));
        }
        files.insert(
            path.to_owned(),
            base64::engine::general_purpose::STANDARD.encode(content),
        );
    }
    if bytes > MAX_CACHE {
        return Err(ApiError::bad("编译环境超出 32 MiB 输入预算"));
    }
    Ok(Environment {
        authority: ExecutionAuthority {
            input_binding: inputs::binding_hash(inner, &captured.authority_ids())?,
            resource_revisions: captured.revisions(),
            collection_revisions: captured.collection_revisions(),
            inputs: captured.authority_ids(),
            style_used: true,
            config_hash,
        },
        module: Some(weftext_engine::StyleModule {
            path: module_path,
            source: doc.source,
            enabled: true,
            entry_function: doc.entry_function,
        }),
        files,
        evidence: Arc::new(Evidence {
            style: StyleEvidence::Enabled(Box::new(style_witness)),
        }),
        input_sets: vec![captured],
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_requires_its_own_role_read_and_use_grants_no_read() {
        let rights: Rights = serde_json::from_value(json!({"styleUse":true})).unwrap();
        assert!(valid_rights(&rights));
        assert!(!rights.style.read);
        let invalid: Rights =
            serde_json::from_value(json!({"schema":{"write":true},"style":{"read":true}})).unwrap();
        assert!(!valid_rights(&invalid));
        assert!(serde_json::from_value::<Rights>(json!({"unknown":{"read":true}})).is_err());
    }
    #[test]
    fn settings_unknown_members_stay_native_but_selector_needs_extra_permission() {
        assert!(
            !validate_config(
                "settings",
                "{\"version\":1}",
                "{\"version\":1,\"custom\":{\"keep\":[\"未知\",7]}}"
            )
            .unwrap()
        );
        assert!(
            validate_config(
                "settings",
                "{\"version\":1}",
                "{\"version\":1,\"defaultStyle\":{\"enabled\":true}}"
            )
            .unwrap()
        );
        assert!(
            validate_config(
                "rules",
                "{}",
                "{\"version\":1,\"ignored\":[\"../hidden\"],\"unmanaged\":[]}"
            )
            .is_err()
        );
    }
    #[cfg(windows)]
    fn fixture() -> (tempfile::TempDir, HostedState, Auth) {
        let (temp, state, auth, _, _, _) = history_api::test_fixture();
        {
            let mut inner = state.inner.lock().unwrap();
            inner.policy.users.get_mut("alice").unwrap().library = Rights {
                schema: RoleRights {
                    read: true,
                    write: true,
                },
                settings: RoleRights {
                    read: true,
                    write: true,
                },
                rules: RoleRights {
                    read: true,
                    write: true,
                },
                style: RoleRights {
                    read: true,
                    write: true,
                },
                style_use: true,
            };
        }
        (temp, state, auth)
    }
    #[cfg(windows)]
    async fn read_role(state: &HostedState, auth: &Auth, role: &str) -> Value {
        read(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Read { role: role.into() }),
        )
        .await
        .unwrap()
        .0
    }
    #[cfg(windows)]
    async fn prepare_source(
        state: &HostedState,
        auth: &Auth,
        role: &str,
        source: &str,
        enabled: Option<bool>,
    ) -> (Value, Prepare) {
        let before = read_role(state, auth, role).await;
        let req = Prepare {
            role: role.into(),
            basis: before["basis"].as_str().unwrap().into(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            source: Some(source.into()),
            changes: None,
            enabled,
        };
        let again = serde_json::from_value(serde_json::to_value(&req).unwrap()).unwrap();
        (
            prepare(State(state.clone()), Extension(auth.clone()), Json(again))
                .await
                .unwrap()
                .0,
            req,
        )
    }
    #[cfg(windows)]
    async fn install_style(state: &HostedState, auth: &Auth) {
        let (plan,_)=prepare_source(state,auth,"style","// private comment\n#let private-value = \"private constant\"\n#let library-style(body) = body\n",Some(true)).await;
        let receipt = commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                role: "style".into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(receipt["status"], "committed");
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn original_config_replay_restart_and_settle_keep_exact_plan_and_source() {
        let (temp, state, auth) = fixture();
        let raw = "{\"version\":1,\"extension\":{\"text\":\"原文😀\",\"unknown\":[7]}}\n";
        let (plan, request) = prepare_source(&state, &auth, "settings", raw, None).await;
        assert_eq!(plan["status"], "prepared");
        assert_eq!(plan["preview"]["afterSource"], raw);
        assert!(
            !state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join(".weftext-settings.json")
                .exists()
        );
        let repeated = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap()),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(repeated["planId"], plan["planId"]);
        let HostedState {
            inner,
            origin,
            host,
            render_gate,
        } = state;
        let HostedInner {
            core,
            config,
            policy,
            ..
        } = Arc::try_unwrap(inner).ok().unwrap().into_inner().unwrap();
        drop(core);
        let state = HostedState {
            inner: Arc::new(Mutex::new(HostedInner {
                core: Core::new(temp.path().join("state")).unwrap(),
                config,
                policy,
                sessions: HashMap::new(),
                downloads: HashMap::new(),
                sync: Default::default(),
                editor_versions: HashMap::new(),
                fields: Default::default(),
                review: Default::default(),
                office: Default::default(),
                library: Default::default(),
                media: Default::default(),
            })),
            origin,
            host,
            render_gate,
        };
        let repeated = prepare(State(state.clone()), Extension(auth.clone()), Json(request))
            .await
            .unwrap()
            .0;
        assert_eq!(repeated["planId"], plan["planId"]);
        let done = commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                role: "settings".into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(done["status"], "committed");
        {
            let inner = state.inner.lock().unwrap();
            assert_eq!(
                std::fs::read_to_string(inner.config.root.join(".weftext-settings.json")).unwrap(),
                raw
            );
        }
        let current = read_role(&state, &auth, "settings").await;
        let late = Prepare {
            role: "settings".into(),
            basis: current["basis"].as_str().unwrap().into(),
            operation_id: uuid::Uuid::new_v4().to_string(),
            source: Some(raw.into()),
            changes: None,
            enabled: None,
        };
        let cancelled = settle(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                role: late.role.clone(),
                operation_id: late.operation_id.clone(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(cancelled["status"], "cancelled");
        assert_eq!(
            prepare(State(state), Extension(auth), Json(late))
                .await
                .unwrap()
                .0["status"],
            "cancelled"
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn config_cross_role_and_selector_without_style_write_are_rejected() {
        let (_temp, state, auth) = fixture();
        let (plan, _) = prepare_source(
            &state,
            &auth,
            "settings",
            "{\"version\":1,\"choice\":1}",
            None,
        )
        .await;
        let wrong = operation(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Operation {
                role: "rules".into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(wrong.status, StatusCode::CONFLICT);
        let basis = read_role(&state, &auth, "settings").await;
        state
            .inner
            .lock()
            .unwrap()
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .library
            .style
            .write = false;
        let denied = prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                role: "settings".into(),
                basis: basis["basis"].as_str().unwrap().into(),
                operation_id: uuid::Uuid::new_v4().to_string(),
                source: Some("{\"version\":1,\"defaultStyle\":{\"enabled\":true}}".into()),
                changes: None,
                enabled: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(denied.status, StatusCode::NOT_FOUND);
        assert!(
            !state
                .inner
                .lock()
                .unwrap()
                .config
                .root
                .join(".weftext-settings.json")
                .exists()
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn enabled_style_uses_private_entry_and_original_witness_but_no_read_grant() {
        let (_temp, state, auth) = fixture();
        install_style(&state, &auth).await;
        let mut inner = state.inner.lock().unwrap();
        inner.policy.users.get_mut("bob").unwrap().library.style_use = true;
        let first = environment(&mut inner, "bob").unwrap();
        let second = environment(&mut inner, "bob").unwrap();
        let path = &first.module.as_ref().unwrap().path;
        assert_ne!(path, "Styles/library-style.typ");
        assert_ne!(path, &second.module.as_ref().unwrap().path);
        assert_eq!(first.authority.config_hash, second.authority.config_hash);
        assert!(first.files.is_empty());
        assert!(access(&inner, "bob", "style", false).is_err());
        first.recheck(&inner, "bob").unwrap();
        let authority = first.authority.clone();
        let style = inner.config.root.join("Styles/library-style.typ");
        std::fs::write(style, "#let library-style(body) = [later style]").unwrap();
        assert!(first.recheck(&inner, "bob").is_err());
        // Completed bytes keep their frozen style. ACL recheck never executes it.
        authorize_execution(&inner, "bob", &authority).unwrap();
        inner.policy.users.get_mut("bob").unwrap().library.style_use = false;
        assert!(authorize_execution(&inner, "bob", &authority).is_err());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn explicit_style_input_has_independent_acl_and_shared_budget() {
        let (_temp, state, auth) = fixture();
        install_style(&state, &auth).await;
        let mut inner = state.inner.lock().unwrap();
        inner.config.style_inputs = vec!["Target".into()];
        assert_eq!(
            environment(&mut inner, "alice").err().unwrap().status,
            StatusCode::NOT_FOUND
        );
        let right = super::super::Rights {
            read: true,
            ..Default::default()
        };
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .insert("Target".into(), right);
        let env = environment(&mut inner, "alice").unwrap();
        assert!(env.files.contains_key("Target/Target.typ"));
        let mut worker = WorkerRequest {
            source: "body".into(),
            path: "Shared/Shared.typ".into(),
            root: inner.config.root.clone(),
            paged: false,
            style: None,
            frozen_files: Some(BTreeMap::from([(
                "owned.bin".into(),
                "x".repeat(MAX_CACHE),
            )])),
        };
        assert!(env.apply(&mut worker).is_err());
        let authority = env.authority.clone();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .remove("Target");
        assert!(authorize_execution(&inner, "alice", &authority).is_err());
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn unknown_and_ungranted_role_have_identical_unavailable_result() {
        let (_temp, state, _auth) = fixture();
        let auth = Auth {
            digest: digest(b"bob"),
        };
        let hidden = read(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Read {
                role: "schema".into(),
            }),
        )
        .await
        .unwrap_err();
        let missing = read(
            State(state),
            Extension(auth),
            Json(Read {
                role: "not-configured".into(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(hidden.status, missing.status);
        assert_eq!(hidden.code, missing.code);
        assert_eq!(hidden.message, missing.message);
    }
}
