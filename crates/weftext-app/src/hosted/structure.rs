//! Operator-bound namespace transactions over the original Core writer.
//! Source, subtree transfer and destination capabilities are independent.
use super::*;
#[cfg(test)]
use crate::CoreTestFixture;
use std::path::Path;
use weftext_core::{
    ActorFeatureStatus, ConfigRole, OperationIntent, OperationRequest, SnapshotWitness,
};
use weftext_data::{NativeValue, Schema};
use weftext_knowledge::NodeRef;
#[path = "structure/capture.rs"]
mod capture;
#[path = "structure/members.rs"]
mod members;
pub(super) use members::{member_file, member_ids, member_rights, split_member};

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_SOURCE: usize = 2 * 1024 * 1024;
const TTL_MS: u64 = 900_000;
const KINDS: [&str; 7] = [
    "create", "rename", "move", "trash", "restore", "copy", "template",
];

#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Slot {
    parent: String,
    name_policy: NamePolicy,
    #[serde(default)]
    types: BTreeSet<String>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum NamePolicy {
    Fixed { name: String },
    UniqueChildren { prefix: String, max_members: u32 },
}
impl Slot {
    fn name(&self, operation: &str) -> Result<String, ApiError> {
        match &self.name_policy {
            NamePolicy::Fixed { name } => Ok(name.clone()),
            NamePolicy::UniqueChildren { prefix, .. } => {
                if !uuid::Uuid::parse_str(operation).is_ok_and(|id| id.to_string() == operation) {
                    return Err(ApiError::bad("创建原操作编号须为规范 UUID"));
                }
                Ok(format!("{prefix}{operation}"))
            }
        }
    }
    fn unique(&self) -> bool {
        matches!(self.name_policy, NamePolicy::UniqueChildren { .. })
    }
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Template {
    source: String,
    template_id: String,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SourceRights {
    #[serde(default)]
    rename: bool,
    #[serde(default)]
    r#move: bool,
    #[serde(default)]
    trash: bool,
    #[serde(default)]
    restore: bool,
    #[serde(default)]
    copy: bool,
    #[serde(default)]
    template: bool,
    #[serde(default)]
    tree_read: bool,
    #[serde(default)]
    tree_export: bool,
}
impl SourceRights {
    fn allows(&self, kind: &str) -> bool {
        match kind {
            "rename" => self.rename,
            "move" => self.r#move,
            "trash" => self.trash,
            "restore" => self.restore,
            "copy" => self.copy,
            "template" => self.template,
            _ => false,
        }
    }
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DestinationRights {
    #[serde(default)]
    create: bool,
    #[serde(default)]
    receive: bool,
    #[serde(default)]
    members: super::Rights,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    sources: BTreeMap<String, SourceRights>,
    #[serde(default)]
    destinations: BTreeMap<String, DestinationRights>,
}
fn valid_member_rights(r: &super::Rights) -> bool {
    !r.office.r#import
        && !r.office.doc.r#import
        && !r.office.wps.r#import
        && (!r.write || r.read)
        && (!r.history || r.read)
        && (!r.download || r.read)
        && query::valid_rights(&r.query)
        && r.fields.len() <= 64
        && r.fields
            .iter()
            .all(|(id, f)| !id.is_empty() && id.len() <= 256 && (!f.write || f.read))
        && (!r.annotations.write || r.annotations.read)
        && (!r.annotations.accept || r.annotations.write && r.annotations.target && r.write)
        && (!r.annotations.proofread || r.read && r.annotations.target)
        && office::valid_legacy_rights(&r.office)
}
fn creation_grants<'a>(
    inner: &'a HostedInner,
    actor: &str,
    slot: &str,
) -> Result<&'a super::Rights, ApiError> {
    let user = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .ok_or_else(expired)?;
    if inner
        .config
        .structure_slots
        .get(slot)
        .is_some_and(Slot::unique)
    {
        user.structure
            .destinations
            .get(slot)
            .map(|r| &r.members)
            .ok_or_else(unavailable)
    } else {
        user.grants.get(slot).ok_or_else(unavailable)
    }
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 240
        && !name.contains(['/', '\\', ':'])
        && !name.ends_with(['.', ' '])
        && !name.chars().any(char::is_control)
        && !matches!(name, "." | "..")
}
fn same_path(a: &Path, b: &Path) -> bool {
    a.components().eq(b.components())
}
pub(super) fn valid_config(config: &Config) -> bool {
    config.structure_slots.len() <= 64
        && config.structure_templates.len() <= 64
        && config.structure_slots.iter().all(|(id, slot)| {
            valid_id(id)
                && slot.types.len() <= 32
                && slot.types.iter().all(|t| !t.is_empty() && t.len() <= 256)
                && config.directories.get(&slot.parent).is_some_and(|p| {
                    p.lifecycle == "live"
                        && match &slot.name_policy {
                            NamePolicy::Fixed { name } => {
                                valid_name(name)
                                    && config.files.get(id).is_some_and(|f| {
                                        f.lifecycle == "live"
                                            && f.editable
                                            && !f.importable
                                            && same_path(
                                                Path::new(&f.path),
                                                &Path::new(&p.path)
                                                    .join(name)
                                                    .join(format!("{name}.typ")),
                                            )
                                    })
                            }
                            NamePolicy::UniqueChildren {
                                prefix,
                                max_members,
                            } => {
                                valid_name(prefix)
                                    && prefix.len() <= 180
                                    && (1..=5000).contains(max_members)
                                    && !config.files.contains_key(id)
                                    && !config.resources.contains_key(id)
                                    && !config.resource_collections.contains_key(id)
                                    && !config.content_import_slots.contains_key(id)
                            }
                        }
                })
        })
        && config.structure_templates.iter().all(|(id, t)| {
            valid_id(id)
                && id != "native-note"
                && !t.template_id.is_empty()
                && t.template_id.len() <= 256
                && config
                    .files
                    .get(&t.source)
                    .is_some_and(|f| f.lifecycle == "live" && f.editable)
        })
}
pub(super) fn valid_rights(config: &Config, rights: &Rights) -> bool {
    rights.sources.len() <= 256
        && rights.destinations.len() <= 64
        && rights.sources.iter().all(|(id, _)| {
            id.strip_suffix("/*").is_some_and(|s| {
                config.content_import_slots.contains_key(s)
                    || config.structure_slots.get(s).is_some_and(Slot::unique)
            }) || content_import::split_member(config, id).is_some()
                || split_member(config, id).is_some()
                || config
                    .files
                    .get(id)
                    .is_some_and(|f| f.lifecycle == "live" && f.editable)
        })
        && rights.destinations.iter().all(|(id, r)| {
            config.structure_slots.get(id).is_some_and(|slot| {
                if slot.unique() {
                    !r.receive && valid_member_rights(&r.members)
                } else {
                    r.members == super::Rights::default()
                }
            })
        })
}
/// Future slots are bound to original committed structural producer facts.
/// Neither a raw occupant nor another admitted object at that name qualifies.
pub(super) fn guard_slot(inner: &HostedInner, resource: &str) -> Result<(), ApiError> {
    if split_member(&inner.config, resource).is_some() {
        return members::qualify_member(inner, resource);
    }
    if inner
        .config
        .structure_slots
        .get(resource)
        .is_some_and(|s| !s.unique())
    {
        let file = &inner.file(resource)?;
        let q = inner
            .core
            .qualified_structure_slot_at(
                &inner.config.root,
                &inner.config.root.join(&file.path),
                resource,
            )
            .map_err(|_| unavailable())?;
        if file.expected_node.as_ref().is_some_and(|n| n != &q.owner) {
            return Err(unavailable());
        }
    }
    Ok(())
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "structureChanged",
        "原结构依据、权限或目标已变化；请保留草稿并核对原操作。",
    )
}
fn core_error(error: CoreError) -> ApiError {
    if error.code == "basisRequired" {
        return ApiError::new(
            StatusCode::CONFLICT,
            "basisRequired",
            "结构事务的保护需要现存的库规则文件；请由库所有者显式安装后重试。",
        );
    }
    ApiError::new(
        StatusCode::CONFLICT,
        &error.code,
        "结构操作未完成，原件与草稿保留；请核对原操作状态。",
    )
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:structure", inner.actor_scope(actor))
}
fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
fn rights(inner: &HostedInner, actor: &str) -> Result<Rights, ApiError> {
    let mut rights = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .map(|u| u.structure.clone())
        .ok_or_else(expired)?;
    let selectors = rights
        .sources
        .iter()
        .filter_map(|(id, r)| {
            id.strip_suffix("/*")
                .filter(|s| {
                    inner.config.content_import_slots.contains_key(*s)
                        || inner
                            .config
                            .structure_slots
                            .get(*s)
                            .is_some_and(Slot::unique)
                })
                .map(|s| (id.clone(), s.to_owned(), r.clone()))
        })
        .collect::<Vec<_>>();
    for (selector, slot, r) in selectors {
        rights.sources.remove(&selector);
        let selected = if inner.config.content_import_slots.contains_key(&slot) {
            content_import::slot_member_ids(inner, &slot)?
        } else {
            members::slot_member_ids(inner, &slot)?
        };
        for id in selected {
            if let Some(existing) = rights.sources.get_mut(&id) {
                existing.rename |= r.rename;
                existing.r#move |= r.r#move;
                existing.trash |= r.trash;
                existing.restore |= r.restore;
                existing.copy |= r.copy;
                existing.template |= r.template;
                existing.tree_read |= r.tree_read;
                existing.tree_export |= r.tree_export;
            } else {
                rights.sources.insert(id, r.clone());
            }
            if rights.sources.len() > 5000 {
                return Err(ApiError::bad("获权结构来源超过 5000 项预算"));
            }
        }
    }
    Ok(rights)
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Capture {
    title: Option<String>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    body_mode: Option<String>,
    #[serde(default)]
    types: Vec<String>,
    #[serde(default, deserialize_with = "crate::data_api::unique_fields")]
    fields: BTreeMap<String, NativeValue>,
    form_basis: Option<weftext_core::CaptureFormBasis>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    resource: String,
    operation_id: String,
    kind: String,
    source: Option<String>,
    destination: Option<String>,
    trash_operation_id: Option<String>,
    template: Option<String>,
    #[serde(default)]
    parameters: BTreeMap<String, Value>,
    #[serde(default)]
    inputs: Vec<String>,
    capture: Option<Capture>,
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
    epoch: u64,
    expires: u64,
    binding: String,
    request: Prepare,
    owner: Option<NodeRef>,
}
pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/structure", get(catalog))
        .route("/api/hosted/structure/prepare", post(prepare))
        .route("/api/hosted/structure/commit", post(commit))
        .route("/api/hosted/structure/operation", post(operation))
        .route("/api/hosted/structure/settle", post(settle))
        .merge(capture::router())
}
fn anchor_access(inner: &HostedInner, actor: &str, resource: &str) -> Result<(), ApiError> {
    let r = rights(inner, actor)?;
    if r.sources
        .get(resource)
        .is_some_and(|s| KINDS.iter().any(|k| s.allows(k)))
        || r.destinations
            .get(resource)
            .is_some_and(|d| d.create || d.receive)
    {
        Ok(())
    } else {
        Err(unavailable())
    }
}
fn authorize(inner: &HostedInner, actor: &str, req: &Prepare) -> Result<(), ApiError> {
    let rights = rights(inner, actor)?;
    if !KINDS.contains(&req.kind.as_str()) {
        return Err(ApiError::bad("未知结构操作"));
    }
    let builtin = req.kind == "template" && req.template.as_deref() == Some("native-note");
    let unique_destination = req
        .destination
        .as_ref()
        .and_then(|id| inner.config.structure_slots.get(id))
        .is_some_and(Slot::unique);
    if unique_destination && !matches!(req.kind.as_str(), "create" | "template") {
        return Err(unavailable());
    }
    if req.kind == "create" || builtin {
        let d = req
            .destination
            .as_deref()
            .ok_or_else(|| ApiError::bad("缺少固定目标槽"))?;
        if req.resource != d
            || req.source.is_some()
            || !rights.destinations.get(d).is_some_and(|d| d.create)
        {
            return Err(unavailable());
        }
    } else {
        let source = req
            .source
            .as_deref()
            .ok_or_else(|| ApiError::bad("缺少配置源对象"))?;
        let r = rights
            .sources
            .get(source)
            .filter(|r| r.allows(&req.kind))
            .ok_or_else(unavailable)?;
        let anchor = if matches!(req.kind.as_str(), "copy" | "template") {
            req.destination.as_deref()
        } else {
            Some(source)
        };
        if anchor != Some(req.resource.as_str()) {
            return Err(unavailable());
        }
        if matches!(req.kind.as_str(), "copy" | "template") {
            if !r.tree_read || !r.tree_export {
                return Err(unavailable());
            }
            library::access(inner, actor, "schema", false)?;
        }
        if req.kind != "trash" {
            let d = req
                .destination
                .as_deref()
                .ok_or_else(|| ApiError::bad("缺少固定目标槽"))?;
            if !rights.destinations.get(d).is_some_and(|d| {
                if unique_destination {
                    d.create
                } else {
                    d.receive
                }
            }) {
                return Err(unavailable());
            }
        }
    }
    if let Some(d) = &req.destination
        && !inner.config.structure_slots.contains_key(d)
    {
        return Err(unavailable());
    }
    if req.inputs.len() > 64 || req.inputs.iter().collect::<BTreeSet<_>>().len() != req.inputs.len()
    {
        return Err(ApiError::bad("显式引用输入超过预算或重复"));
    }
    if !req.inputs.is_empty() && !matches!(req.kind.as_str(), "create" | "copy" | "template") {
        return Err(ApiError::bad("此结构操作不读取显式引用输入"));
    }
    for id in &req.inputs {
        if !inner
            .file(id)
            .ok()
            .is_some_and(|f| f.editable && f.lifecycle == "live")
        {
            return Err(unavailable());
        }
        inner.rights(actor, id)?;
    }
    if req.kind == "template" && !builtin {
        let t = req
            .template
            .as_ref()
            .and_then(|id| inner.config.structure_templates.get(id))
            .ok_or_else(unavailable)?;
        if req.source.as_ref() != Some(&t.source) {
            return Err(unavailable());
        }
    }
    if req.capture.is_some() && req.kind != "create"
        || req.kind != "template" && (!req.parameters.is_empty() || req.template.is_some())
        || req.kind != "restore" && req.trash_operation_id.is_some()
        || req.kind == "trash" && req.destination.is_some()
    {
        return Err(ApiError::bad("输入不属于所选原结构操作"));
    }
    Ok(())
}
fn binding(inner: &HostedInner, req: &Prepare) -> Result<String, ApiError> {
    let source = req.source.as_ref().and_then(|id| inner.file(id).ok());
    let destination = req.destination.as_ref().and_then(|id| inner.file(id).ok());
    let slot = req
        .destination
        .as_ref()
        .and_then(|id| inner.config.structure_slots.get(id));
    let parent = slot.and_then(|s| inner.config.directories.get(&s.parent));
    let template = req
        .template
        .as_ref()
        .and_then(|id| inner.config.structure_templates.get(id));
    let inputs = req
        .inputs
        .iter()
        .map(|id| inner.file(id).ok().map(|f| (id, f)).ok_or_else(unavailable))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"source":source,"destination":destination,"slot":slot,"parent":parent,"template":template,"inputs":inputs})).map_err(|_|changed())?))
}
fn original_allowed(inner: &HostedInner, actor: &str, status: &ActorFeatureStatus) -> bool {
    serde_json::from_value::<Decision>(status.preview["_structure"].clone()).is_ok_and(|d| {
        d.epoch == inner.actor_generation(actor)
            && authorize(inner, actor, &d.request).is_ok()
            && binding(inner, &d.request).is_ok_and(|b| b == d.binding)
    })
}
fn safe_status(
    inner: &mut HostedInner,
    actor: &str,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let same = original_allowed(inner, actor, &status);
    let d = serde_json::from_value::<Decision>(status.preview["_structure"].clone()).ok();
    let mut preview = json!({});
    if same {
        for key in ["kind", "summary", "source", "destination", "losses"] {
            if let Some(v) = status.preview.get(key) {
                preview[key] = v.clone();
            }
        }
    }
    let mut result = Value::Null;
    if status.status == "committed"
        && same
        && let Some(d) = &d
    {
        let destination = d.request.destination.as_ref();
        let actor_scope = scope(inner, actor);
        if let Some(destination) = destination {
            let slot = &inner.config.structure_slots[destination];
            let unique = slot.unique();
            let path = if unique {
                let name = slot.name(&status.operation_id)?;
                inner
                    .config
                    .root
                    .join(&inner.config.directories[&slot.parent].path)
                    .join(&name)
                    .join(format!("{name}.typ"))
            } else {
                inner.config.root.join(&inner.file(destination)?.path)
            };
            let root = inner.config.root.clone();
            let receipt = inner
                .core
                .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
                    c.actor_feature_structure_receipt(&status.resource, &status.operation_id)
                })
                .map_err(core_error)?;
            let selected = receipt
                .as_ref()
                .and_then(|r| r.nodes.iter().find(|n| same_path(&n.source, &path)));
            if let Some(node) = selected {
                let q = inner
                    .core
                    .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
                        c.actor_feature_structure_result_at(
                            &status.resource,
                            &root,
                            &path,
                            &status.operation_id,
                            &node.node_ref,
                        )
                    });
                let result_resource = if unique {
                    format!("{destination}/{}", node.node_ref.node_id)
                } else {
                    destination.clone()
                };
                let can_open =
                    q.is_ok() && inner.rights(actor, &result_resource).is_ok_and(|r| r.read);
                result = json!({"resource":result_resource,"nodeRef":node.node_ref,"canOpen":can_open,"status":if q.is_ok(){"available"}else{"unavailable"}});
            } else {
                result = json!({"resource":destination,"canOpen":false,"status":"unavailable"});
            }
        } else {
            result = json!({"resource":status.resource,"canOpen":false,"status":"trash"});
        }
    }
    let allowed = status.status == "prepared" && same && d.is_some_and(|d| d.expires > now_ms());
    Ok(
        json!({"resource":status.resource,"operationId":status.operation_id,"status":status.status,"planId":status.plan_id,
        "preview":preview,"nodeRefs":if same{status.node_refs}else{vec![]},"resourceRefs":if same{status.resource_refs}else{vec![]},"commitAllowed":allowed,"result":result}),
    )
}
fn owner(inner: &HostedInner, id: &str) -> Result<weftext_core::QualifiedQueryNode, ApiError> {
    content_import::qualify_member(inner, id)?;
    guard_slot(inner, id)?;
    let file = inner
        .file(id)
        .ok()
        .filter(|f| f.lifecycle == "live")
        .ok_or_else(unavailable)?;
    let path = inner.config.root.join(&file.path);
    let q = inner
        .core
        .qualified_node_at(&inner.config.root, &path)
        .map_err(core_error)?;
    if file.expected_node.as_ref().is_some_and(|n| n != &q.owner) {
        return Err(changed());
    }
    inner
        .core
        .qualified_query_node_at(&inner.config.root, &path, &q.owner, "live")
        .map_err(core_error)
}
fn capture_body(c: &Capture) -> Result<String, ApiError> {
    if c.body.len() > MAX_SOURCE || c.title.as_ref().is_some_and(|t| t.len() > 16 * 1024) {
        return Err(ApiError::bad("创建正文或标题超过预算"));
    }
    let body = match c.body_mode.as_deref().unwrap_or("text") {
        "text" => weftext_engine::encode_literal(&c.body)
            .map_err(|_| ApiError::bad("文字输入不可编码"))?,
        "markup" => {
            fn isolated(
                node: &typst_syntax::SyntaxNode,
                depth: usize,
                remaining: &mut usize,
            ) -> bool {
                if depth > 64 || *remaining == 0 {
                    return false;
                }
                *remaining -= 1;
                if matches!(
                    node.kind(),
                    typst_syntax::SyntaxKind::Hash
                        | typst_syntax::SyntaxKind::FuncCall
                        | typst_syntax::SyntaxKind::MathCall
                ) || node
                    .cast::<typst_syntax::ast::Ident>()
                    .is_some_and(|i| i.get() == "metadata")
                    || node
                        .cast::<typst_syntax::ast::Label>()
                        .is_some_and(|l| l.get() == "weftext")
                {
                    return false;
                }
                node.children()
                    .all(|child| isolated(child, depth + 1, remaining))
            }
            let source = typst_syntax::Source::detached(c.body.clone());
            if !isolated(source.root(), 0, &mut 20_000) {
                return Err(ApiError::bad(
                    "创建 markup 仅支持可证明的排版内容；代码与身份、类型、字段声明须经独立正文编辑保存",
                ));
            }
            c.body.clone()
        }
        _ => return Err(ApiError::bad("正文模式须为 text 或 markup")),
    };
    Ok(if let Some(title) = &c.title {
        format!(
            "\n#set document(title: [{}])\n#title()\n\n{}",
            weftext_engine::encode_literal(title).map_err(|_| ApiError::bad("标题不可编码"))?,
            body
        )
    } else {
        body
    })
}
fn capture_source(
    inner: &mut HostedInner,
    actor: &str,
    destination: &str,
    c: &Capture,
    pool: &mut Vec<SnapshotWitness>,
    captured: &inputs::Captured,
) -> Result<String, ApiError> {
    if c.types.len() > 32 || c.fields.len() > 64 {
        return Err(ApiError::bad("创建类型或字段超过预算"));
    }
    let body = capture_body(c)?;
    if c.types.is_empty() && c.fields.is_empty() {
        return Ok(format!("#metadata((types: ())) <weftext>\n{body}"));
    }
    library::access(inner, actor, "schema", false)?;
    let slot = &inner.config.structure_slots[destination];
    if c.types.iter().any(|t| !slot.types.contains(t)) {
        return Err(unavailable());
    }
    let grants = creation_grants(inner, actor, destination)?.clone();
    for id in c.fields.keys() {
        if !grants.fields.get(id).is_some_and(|f| f.read && f.write) {
            return Err(unavailable());
        }
    }
    let doc = inner
        .core
        .read_config(&inner.config.root, ConfigRole::Schema)
        .map_err(core_error)?;
    let w = inner
        .core
        .take_snapshot_witness(&doc.base_id)
        .map_err(core_error)?;
    if doc.bytes.is_some() && doc.revision.is_none() {
        return Err(changed());
    }
    let schema = match &doc.bytes {
        Some(b) => Schema::from_author_json(std::str::from_utf8(b).map_err(|_| changed())?),
        None => Schema::bundled(),
    }
    .map_err(|_| changed())?;
    let schema_digest = digest(&serde_json::to_vec(&schema).map_err(|_| changed())?);
    if c.form_basis.as_ref().is_none_or(|basis| {
        basis.schema_ref != doc.role_ref || basis.semantic_digest != schema_digest
    }) {
        return Err(changed());
    }
    // Creation defaults are writes too. Authorize the exact applicable field
    // closure before invoking any native default, constraint or validator.
    let (resolved, check) = schema.expand_types(&c.types);
    if !check.is_valid() {
        return Err(ApiError::bad("类型组合不可用"));
    }
    for (f, required) in schema.applicable_fields(&resolved) {
        if (required
            || c.fields.contains_key(&f.id)
            || f.default.as_ref().is_some_and(|d| d["kind"] == "native"))
            && !grants.fields.get(&f.id).is_some_and(|f| f.read && f.write)
        {
            return Err(unavailable());
        }
        if c.fields.keys().any(|id| id == &f.key && id != &f.id) {
            return Err(ApiError::bad("创建字段须使用准确 FieldId"));
        }
    }
    // Provider capability remains separate even when complete schema read is
    // authorized: only granted registries can execute field validation.
    let providers = &inner.policy.users[actor].query_providers;
    let mut restricted = schema.clone();
    restricted
        .providers
        .retain(|p| providers.providers.contains(&p.id));
    restricted
        .series_policies
        .retain(|p| providers.series_policies.contains(&p.id));
    let source = weftext_data::capture_template(&c.types, &c.fields, &body, &restricted)
        .map_err(|_| ApiError::bad("创建字段或原生依赖不可用"))?;
    let check_source = weftext_data::capture_source(
        "00000000-0000-4000-8000-000000000000",
        &c.types,
        &c.fields,
        &body,
        &restricted,
    )
    .map_err(|_| ApiError::bad("创建字段或原生依赖不可用"))?;
    let validation = weftext_data::validate_note(
        &check_source,
        &restricted,
        &weftext_data::ValidationContext {
            owner: None,
            resources: vec![],
            resources_complete: true,
        },
    );
    if !validation.is_valid() {
        return Err(ApiError::bad("创建类型、字段或上下文约束不可用"));
    }
    let mut targets = vec![];
    let mut sources = vec![];
    for id in captured.ids() {
        let (_, bytes, accepted) = captured.entry(id)?.ok_or_else(changed)?;
        if !accepted {
            return Err(changed());
        }
        let native = std::str::from_utf8(bytes).map_err(|_| changed())?;
        let q = owner(inner, id)?;
        if q.source_hash != digest(bytes) {
            return Err(changed());
        }
        let entity_key = format!(
            "weftext-node://{}/{}",
            q.owner.workspace_id, q.owner.node_id
        );
        targets.push(weftext_data::RelationTarget {
            entity_key: entity_key.clone(),
            types: weftext_data::project(native, &restricted).types,
            lifecycle: "live".into(),
        });
        sources.push(weftext_data::RelationSource {
            entity_key,
            source: native.into(),
            lifecycle: "live".into(),
        });
    }
    let workspace = doc
        .role_ref
        .strip_prefix("weftext-config://")
        .and_then(|s| s.strip_suffix("/schema"))
        .ok_or_else(changed)?;
    let new_owner = format!("weftext-node://{workspace}/00000000-0000-4000-8000-000000000000");
    let relations = weftext_data::validate_relation_change_with_graph(
        &new_owner,
        "",
        &check_source,
        &restricted,
        &targets,
        &sources,
        false,
    );
    if !relations.is_valid() {
        return Err(ApiError::bad(
            "创建关系需要准确获权目标；完整图依赖未获权时不可假定成功",
        ));
    }
    pool.push(w);
    Ok(source)
}
fn typed_catalog(inner: &mut HostedInner, actor: &str, id: &str) -> Result<Value, ApiError> {
    library::access(inner, actor, "schema", false)?;
    let doc = inner
        .core
        .read_config(&inner.config.root, ConfigRole::Schema)
        .map_err(core_error)?;
    let w = inner
        .core
        .take_snapshot_witness(&doc.base_id)
        .map_err(core_error)?;
    if w.retained_bytes() > MAX_BYTES || doc.bytes.is_some() && doc.revision.is_none() {
        return Err(changed());
    }
    let schema = match &doc.bytes {
        Some(b) => Schema::from_author_json(std::str::from_utf8(b).map_err(|_| changed())?),
        None => Schema::bundled(),
    }
    .map_err(|_| changed())?;
    let slot = &inner.config.structure_slots[id];
    let mut expanded = BTreeSet::new();
    for allowed in &slot.types {
        if !schema.types.iter().any(|t| &t.id == allowed) {
            return Err(changed());
        }
        let (one, valid) = schema.expand_types(std::slice::from_ref(allowed));
        if !valid.is_valid() {
            return Err(changed());
        }
        expanded.extend(one);
    }
    let fields = schema
        .applicable_fields(&expanded)
        .into_iter()
        .filter(|(f, _)| {
            creation_grants(inner, actor, id)
                .ok()
                .and_then(|r| r.fields.get(&f.id))
                .is_some_and(|f| f.read && f.write)
        })
        .map(|(f, _)| f.clone())
        .collect::<Vec<_>>();
    fn names(
        shape: &weftext_data::Shape,
        schema: &Schema,
        out: &mut BTreeMap<String, weftext_data::Shape>,
    ) -> Result<(), ApiError> {
        use weftext_data::Shape;
        match shape {
            Shape::Use { name } => {
                if !out.contains_key(name) {
                    if out.len() >= 64 {
                        return Err(changed());
                    }
                    let s = schema.definitions.get(name).ok_or_else(changed)?;
                    out.insert(name.clone(), s.clone());
                    names(s, schema, out)?;
                }
            }
            Shape::List { items, .. } => names(items, schema, out)?,
            Shape::Record { members, .. } => {
                for m in members.values() {
                    names(&m.shape, schema, out)?;
                }
            }
            Shape::Union { variants, .. } => {
                for v in variants {
                    names(v, schema, out)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let mut definitions = BTreeMap::new();
    for f in &fields {
        names(&f.shape, &schema, &mut definitions)?;
    }
    inner
        .core
        .validate_snapshot_witness(&inner.config.root, Path::new(&doc.path), &w)
        .map_err(core_error)?;
    let dto = json!({"types":schema.types.iter().filter(|t|expanded.contains(&t.id)).collect::<Vec<_>>(),"fields":fields,"definitions":definitions,"aliases":{},"roleRef":doc.role_ref,"semanticDigest":digest(&serde_json::to_vec(&schema).map_err(|_| changed())?)});
    if serde_json::to_vec(&dto).map_err(|_| changed())?.len() > MAX_SOURCE {
        return Err(ApiError::bad("类型创建表单超过预算"));
    }
    Ok(dto)
}
fn template_catalog(inner: &mut HostedInner, actor: &str, id: &str) -> Result<Value, ApiError> {
    let configured = inner.config.structure_templates[id].clone();
    let q = owner(inner, &configured.source)?;
    let source_path = inner
        .config
        .root
        .join(&inner.file(&configured.source)?.path);
    let root = inner.config.root.clone();
    let cut = inner
        .core
        .read_snapshot(
            &root,
            &[
                source_path.to_string_lossy().into_owned(),
                root.join(".weftext-settings.json")
                    .to_string_lossy()
                    .into_owned(),
            ],
        )
        .map_err(core_error)?;
    let mut pool = vec![];
    for f in &cut.files {
        pool.push(
            inner
                .core
                .take_snapshot_witness(&f.base_id)
                .map_err(core_error)?,
        );
    }
    if q.witness.retained_bytes().saturating_add(
        pool.iter()
            .map(SnapshotWitness::retained_bytes)
            .sum::<usize>(),
    ) > MAX_BYTES
    {
        return Err(ApiError::bad("模板依据超过预算"));
    }
    let f = &cut.files[0];
    if cut.files[1].revision.is_none() {
        return Err(changed());
    }
    let source = String::from_utf8(f.bytes.clone().ok_or_else(changed)?).map_err(|_| changed())?;
    let definition = crate::knowledge_api::template_registry(&cut.files[1])?
        .1
        .into_iter()
        .find(|d| d.id == configured.template_id)
        .ok_or_else(unavailable)?;
    let snapshot = weftext_knowledge::NodeSnapshot {
        owner: q.owner,
        path: String::new(),
        library_path: String::new(),
        revision_id: f.revision.clone().ok_or_else(changed)?,
        source,
        title: None,
        aliases: vec![],
        labels: vec![],
        labels_complete: false,
    };
    weftext_knowledge::validate_template_definition(&definition, &snapshot)
        .map_err(|_| changed())?;
    for (f, w) in cut.files.iter().zip(&pool) {
        inner
            .core
            .validate_snapshot_witness(&root, Path::new(&f.path), w)
            .map_err(core_error)?;
    }
    inner
        .core
        .validate_query_node_witness(&root, &q.witness)
        .map_err(core_error)?;
    let parameters=definition.slots.iter().map(|s|{let mut v=json!({"name":s.name,"kind":s.kind,"nullable":s.nullable,"required":s.default.is_none()});if let Some(d)=&s.default{v["default"]=crate::knowledge_api::template_value(d);}v}).collect::<Vec<_>>();
    let _ = actor;
    Ok(
        json!({"id":id,"title":inner.file(&configured.source)?.title,"source":configured.source,"status":"ready","builtin":false,"parameters":parameters}),
    )
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| prepare_inner(inner, &auth, req)).await?,
    ))
}
fn prepare_inner(inner: &mut HostedInner, auth: &Auth, req: Prepare) -> Result<Value, ApiError> {
    let actor = inner.actor(auth)?;
    authorize(inner, &actor, &req)?;
    let encoded = serde_json::to_vec(&req).map_err(|_| changed())?;
    if encoded.len() > MAX_SOURCE + 128 * 1024 {
        return Err(ApiError::bad("结构请求超过预算"));
    }
    let fp = digest(&encoded);
    let actor_scope = scope(inner, &actor);
    if let Some(status) = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
            c.actor_feature_retry(&req.resource, &req.operation_id, &fp)
        })
        .map_err(core_error)?
    {
        return safe_status(inner, &actor, status);
    }
    let root = inner.config.root.clone();
    let mut pool = vec![];
    let mut tree = None;
    let mut source_owner = None;
    let mut tokens = vec![];
    let built = (|| {
        let captured = inputs::capture(inner, &actor, &req.inputs)?;
        if captured.retained_bytes() > MAX_BYTES {
            return Err(ApiError::bad("显式引用输入超过 32 MiB 预算"));
        }
        let transfer = captured.plan_witnesses()?;
        let destination = if let Some(d) = &req.destination {
            let slot = inner.config.structure_slots[d].clone();
            let parent = inner.config.directories[&slot.parent].clone();
            let q = inner
                .core
                .qualified_query_scope_at(
                    &root,
                    &root.join(&parent.path),
                    &parent.reference,
                    "live",
                )
                .map_err(core_error)?;
            Core::merge_query_scope_witness(&mut pool, &q.witness).map_err(core_error)?;
            Some((slot, root.join(&parent.path), q.directory))
        } else {
            None
        };
        let mut selected_source = None;
        if let Some(source) = &req.source {
            let q = if req.kind == "restore" {
                let original = req
                    .trash_operation_id
                    .as_deref()
                    .ok_or_else(|| ApiError::bad("缺少原 Trash 操作编号"))?;
                let receipt = inner
                    .core
                    .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
                        c.actor_feature_structure_receipt(source, original)
                    })
                    .map_err(core_error)?
                    .filter(|r| r.trash_operation_id.is_some())
                    .ok_or_else(unavailable)?;
                let node = receipt
                    .nodes
                    .first()
                    .filter(|_| receipt.nodes.len() == 1)
                    .ok_or_else(unavailable)?;
                let q = inner
                    .core
                    .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
                        c.actor_feature_structure_result_at(
                            source,
                            &root,
                            &node.source,
                            original,
                            &node.node_ref,
                        )
                    })
                    .map_err(|_| unavailable())?;
                selected_source = Some((node.source.clone(), receipt.trash_operation_id.unwrap()));
                q
            } else {
                let q = owner(inner, source)?;
                selected_source = Some((root.join(&inner.file(source)?.path), String::new()));
                q
            };
            source_owner = Some(q.owner.clone());
            Core::merge_query_node_witness(&mut pool, &q.witness).map_err(core_error)?;
            if matches!(req.kind.as_str(), "copy" | "template") {
                let path = &selected_source.as_ref().unwrap().0;
                let captured = inner
                    .core
                    .capture_feature_tree(&root, path.parent().ok_or_else(changed)?, &q.owner)
                    .map_err(core_error)?;
                if captured.retained_bytes() > MAX_BYTES {
                    return Err(ApiError::bad("整树传递依据超过 32 MiB 预算"));
                }
                tree = Some(captured);
            }
        }
        let intent = match req.kind.as_str() {
            "create" => {
                let (s, p, _) = destination.as_ref().unwrap();
                let source = req
                    .capture
                    .as_ref()
                    .map(|c| {
                        capture_source(
                            inner,
                            &actor,
                            req.destination.as_ref().unwrap(),
                            c,
                            &mut pool,
                            &captured,
                        )
                    })
                    .transpose()?;
                OperationIntent::NewNode {
                    parent: p.to_string_lossy().into_owned(),
                    name: s.name(&req.operation_id)?,
                    source,
                }
            }
            "template" if req.template.as_deref() == Some("native-note") => {
                let (s, p, _) = destination.as_ref().unwrap();
                let source = crate::knowledge_api::template_source(
                    &json!({"templateId":"native-note","parameters":req.parameters}),
                )?;
                OperationIntent::NewNode {
                    parent: p.to_string_lossy().into_owned(),
                    name: s.name(&req.operation_id)?,
                    source: Some(source),
                }
            }
            "rename" => {
                let (s, _, coordinate) = destination.as_ref().unwrap();
                let path = &selected_source.as_ref().unwrap().0;
                let q = inner
                    .core
                    .qualified_query_node_at(&root, path, source_owner.as_ref().unwrap(), "live")
                    .map_err(core_error)?;
                if q.coordinate.parent().and_then(Path::parent) != Some(coordinate.as_path()) {
                    return Err(ApiError::bad("重命名须使用同一父目录中的固定目标槽"));
                }
                OperationIntent::RenameNode {
                    source: path
                        .parent()
                        .ok_or_else(changed)?
                        .to_string_lossy()
                        .into_owned(),
                    name: s.name(&req.operation_id)?,
                }
            }
            "move" => {
                let (s, p, _) = destination.as_ref().unwrap();
                let path = &selected_source.as_ref().unwrap().0;
                let name = s.name(&req.operation_id)?;
                if path
                    .parent()
                    .and_then(Path::file_name)
                    .is_none_or(|n| n != name.as_str())
                {
                    return Err(ApiError::bad("移动目标须保留原笔记名称"));
                }
                OperationIntent::MoveNode {
                    source: path
                        .parent()
                        .ok_or_else(changed)?
                        .to_string_lossy()
                        .into_owned(),
                    parent: p.to_string_lossy().into_owned(),
                }
            }
            "copy" => {
                let (s, p, _) = destination.as_ref().unwrap();
                OperationIntent::CopyNode {
                    source: selected_source
                        .as_ref()
                        .unwrap()
                        .0
                        .parent()
                        .ok_or_else(changed)?
                        .to_string_lossy()
                        .into_owned(),
                    parent: p.to_string_lossy().into_owned(),
                    name: s.name(&req.operation_id)?,
                    allow_losses: true,
                }
            }
            "trash" => OperationIntent::TrashNode {
                source: selected_source
                    .as_ref()
                    .unwrap()
                    .0
                    .parent()
                    .ok_or_else(changed)?
                    .to_string_lossy()
                    .into_owned(),
            },
            "restore" => {
                let (s, p, _) = destination.as_ref().unwrap();
                OperationIntent::RestoreNode {
                    trash_operation_id: selected_source.as_ref().unwrap().1.clone(),
                    node_id: Some(source_owner.as_ref().unwrap().node_id.clone()),
                    parent: p.to_string_lossy().into_owned(),
                    name: s.name(&req.operation_id)?,
                }
            }
            "template" => {
                let (s, p, _) = destination.as_ref().unwrap();
                let config = &inner.config.structure_templates[req.template.as_ref().unwrap()];
                let path = &selected_source.as_ref().unwrap().0;
                let cut = inner
                    .core
                    .read_snapshot(
                        &root,
                        &[
                            path.to_string_lossy().into_owned(),
                            root.join(".weftext-settings.json")
                                .to_string_lossy()
                                .into_owned(),
                        ],
                    )
                    .map_err(core_error)?;
                tokens.extend(cut.files.iter().map(|f| f.base_id.clone()));
                let f = &cut.files[0];
                let source = String::from_utf8(f.bytes.clone().ok_or_else(changed)?)
                    .map_err(|_| changed())?;
                if tree.as_ref().unwrap().source_hash() != digest(source.as_bytes()) {
                    return Err(changed());
                }
                if cut.files[1].revision.is_none() {
                    return Err(changed());
                }
                let definitions = crate::knowledge_api::template_registry(&cut.files[1])?.1;
                let definition = definitions
                    .iter()
                    .find(|d| d.id == config.template_id)
                    .ok_or_else(unavailable)?;
                let snapshot = weftext_knowledge::NodeSnapshot {
                    owner: source_owner.clone().unwrap(),
                    path: String::new(),
                    library_path: String::new(),
                    revision_id: f.revision.clone().ok_or_else(changed)?,
                    source: source.clone(),
                    title: None,
                    aliases: vec![],
                    labels: vec![],
                    labels_complete: false,
                };
                weftext_knowledge::validate_template_definition(definition, &snapshot)
                    .map_err(|_| changed())?;
                let mut parameters = BTreeMap::new();
                for (name, v) in &req.parameters {
                    let slot = definition
                        .slots
                        .iter()
                        .find(|s| s.name == *name)
                        .ok_or_else(|| ApiError::bad("参数不属于原准确模板槽"))?;
                    parameters.insert(
                        name.clone(),
                        crate::knowledge_api::parse_template_value(v, slot.kind)?,
                    );
                }
                let instantiated = weftext_knowledge::instantiate_template(
                    &source,
                    &definition.slots,
                    &parameters,
                )
                .map_err(|_| ApiError::bad("模板参数或原准确槽不可用"))?;
                OperationIntent::TemplateNode {
                    source: path.to_string_lossy().into_owned(),
                    parent: p.to_string_lossy().into_owned(),
                    name: s.name(&req.operation_id)?,
                    instantiated_source: instantiated,
                    source_base_id: f.base_id.clone(),
                }
            }
            _ => return Err(ApiError::bad("未知结构操作")),
        };
        let bytes = pool
            .iter()
            .map(SnapshotWitness::retained_bytes)
            .sum::<usize>()
            .saturating_add(tree.as_ref().map_or(0, |t| t.retained_bytes()))
            .saturating_add(captured.retained_bytes())
            .saturating_add(
                transfer
                    .iter()
                    .map(SnapshotWitness::retained_bytes)
                    .sum::<usize>(),
            )
            .saturating_add(encoded.len());
        if bytes > MAX_BYTES {
            return Err(ApiError::bad("结构请求与原保护依据超过 32 MiB 预算"));
        }
        let decision = Decision {
            epoch: inner.actor_generation(&actor),
            expires: now_ms().saturating_add(TTL_MS),
            binding: binding(inner, &req)?,
            request: req.clone(),
            owner: source_owner.clone(),
        };
        let preview = json!({"kind":req.kind,"summary":"在明确授权的源与固定目标槽之间执行原结构事务；正文草稿不自动保存或切换。","source":req.source,"destination":req.destination,"_structure":decision});
        let request = OperationRequest {
            operation_id: req.operation_id.clone(),
            root: root.to_string_lossy().into_owned(),
            intent,
        };
        let mut refs = pool.iter().collect::<Vec<_>>();
        refs.extend(transfer.iter());
        let transfer_refs = if tree.is_some() {
            transfer.iter().collect::<Vec<_>>()
        } else {
            vec![]
        };
        let destination_binding = req
            .destination
            .as_ref()
            .map(|id| {
                let slot = &inner.config.structure_slots[id];
                match &slot.name_policy {
                    NamePolicy::Fixed { .. } => Ok(weftext_core::HostedDestination::fixed(id)),
                    NamePolicy::UniqueChildren {
                        prefix,
                        max_members,
                    } => inner
                        .core
                        .hosted_unique_child_destination(
                            &root,
                            &root.join(&inner.config.directories[&slot.parent].path),
                            id,
                            prefix,
                            *max_members,
                            &req.operation_id,
                        )
                        .map_err(core_error),
                }
            })
            .transpose()?;
        let status = inner
            .core
            .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
                c.actor_feature_prepare_structure(
                    &req.resource,
                    &req.operation_id,
                    &fp,
                    request,
                    preview,
                    &tokens,
                    &refs,
                    &transfer_refs,
                    destination_binding.as_ref(),
                    tree.as_ref(),
                )
            })
            .map_err(core_error)?;
        safe_status(inner, &actor, status)
    })();
    for token in tokens {
        let _ = inner.core.take_snapshot_witness(&token);
    }
    built
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{let actor=inner.actor(&auth)?;anchor_access(inner,&actor,&req.resource)?;let status=inner.core.with_actor_for_root(&scope(inner,&actor), &inner.config.root,|c|c.actor_feature_status(&req.resource,&req.operation_id)).map_err(core_error)?;match status{Some(s)=>safe_status(inner,&actor,s),None=>Ok(json!({"resource":req.resource,"operationId":req.operation_id,"status":"notRegistered","planId":null,"preview":{},"nodeRefs":[],"resourceRefs":[],"commitAllowed":false,"result":null}))}}).await?))
}
async fn settle(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            anchor_access(inner, &actor, &req.resource)?;
            let s = inner
                .core
                .with_actor_for_root(&scope(inner, &actor), &inner.config.root, |c| {
                    c.actor_feature_settle(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, s)
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
        locked(&state, move |inner| commit_inner(inner, &auth, req)).await?,
    ))
}
fn commit_inner(inner: &mut HostedInner, auth: &Auth, req: Commit) -> Result<Value, ApiError> {
    let actor = inner.actor(auth)?;
    anchor_access(inner, &actor, &req.resource)?;
    let actor_scope = scope(inner, &actor);
    let old = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
            c.actor_feature_status(&req.resource, &req.operation_id)
        })
        .map_err(core_error)?
        .ok_or_else(changed)?;
    if old.status == "prepared" {
        let d: Decision =
            serde_json::from_value(old.preview["_structure"].clone()).map_err(|_| changed())?;
        authorize(inner, &actor, &d.request)?;
        if !original_allowed(inner, &actor, &old)
            || d.expires <= now_ms()
            || old.plan_id.as_deref() != Some(req.plan_id.as_str())
        {
            return Err(changed());
        }
    }
    let s = inner
        .core
        .with_actor_for_root(&actor_scope, &inner.config.root, |c| {
            c.actor_feature_commit_with_losses(
                &req.resource,
                &req.operation_id,
                &req.plan_id,
                req.accept_losses,
            )
        })
        .map_err(core_error)?;
    safe_status(inner, &actor, s)
}
async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(locked(&state,move|inner|{
        let actor=inner.actor(&auth)?;let rights=rights(inner,&actor)?.clone();let mut sources=vec![];let mut slots=vec![];let mut templates=vec![];let mut partial=false;
        for(id,r)in &rights.sources {let file=&inner.file(id)?;let ready=owner(inner,id).is_ok();partial|=!ready;
            let actions=KINDS.iter().filter(|k|r.allows(k)&&(!matches!(**k,"copy"|"template")||(r.tree_read&&r.tree_export&&library::access(inner,&actor,"schema",false).is_ok()))).collect::<Vec<_>>();
            if !actions.is_empty(){sources.push(json!({"resource":id,"title":file.title,"actions":actions,"status":if ready{"ready"}else{"unavailable"}}));}}
        for(id,r)in &rights.destinations {if !r.create&&!r.receive{continue;}let slot=&inner.config.structure_slots[id];let parent=&inner.config.directories[&slot.parent];let parent_path = inner.config.root.join(&parent.path);let ready=inner.core.qualified_query_scope_at(&inner.config.root,&parent_path,&parent.reference,"live").is_ok() && match &slot.name_policy { NamePolicy::Fixed {..} => true, NamePolicy::UniqueChildren {prefix,max_members} => inner.core.hosted_unique_child_available_at(&inner.config.root,&parent_path,id,prefix,*max_members).unwrap_or(false) };partial|=!ready;
            let slot_title = if slot.unique() { id.clone() } else { inner.file(id)?.title };
            let mut row=json!({"resource":id,"title":slot_title,"namePolicy":slot.name_policy,"parent":slot.parent,"parentTitle":parent.title,"create":r.create,"receive":r.receive,"status":if ready{"ready"}else{"unavailable"},"types":slot.types});
            if r.create&&!slot.types.is_empty(){match typed_catalog(inner,&actor,id){Ok(c)=>row["capture"]=c,Err(_)=>{row["captureStatus"]="unavailable".into();partial=true;}}}slots.push(row);}
        if rights.destinations.values().any(|d|d.create){templates.push(json!({"id":"native-note","title":"原生笔记","status":"ready","builtin":true,"parameters":[{"name":"title","kind":"markup","nullable":false,"required":true},{"name":"body","kind":"markup","nullable":false,"required":false,"default":""}]}));}
        let configured=inner.config.structure_templates.clone();for(id,t)in &configured {if rights.sources.get(&t.source).is_some_and(|r|r.template&&r.tree_read&&r.tree_export)&&library::access(inner,&actor,"schema",false).is_ok(){match template_catalog(inner,&actor,id){Ok(t)=>templates.push(t),Err(_)=>{templates.push(json!({"id":id,"title":inner.file(&t.source)?.title,"source":t.source,"status":"unavailable","builtin":false}));partial=true;}}}}
        Ok(json!({"scope":"configuredAuthorizedStructure","coverage":if partial{"partial"}else{"complete"},"sources":sources,"slots":slots,"templates":templates}))
    }).await?))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn id() -> String {
        uuid::Uuid::new_v4().to_string()
    }
    pub(super) fn fixture() -> (tempfile::TempDir, HostedState, Auth) {
        let (temp, state, auth, _, _, _) = history_api::test_fixture();
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
                title: "获准父目录".into(),
                reference: admitted.target_ref,
                lifecycle: "live".into(),
            },
        );
        for name in ["Created", "Copied", "Renamed", "Restored"] {
            inner.config.structure_slots.insert(
                name.into(),
                Slot {
                    parent: "root".into(),
                    name_policy: NamePolicy::Fixed { name: name.into() },
                    types: BTreeSet::new(),
                },
            );
            inner.config.files.insert(
                name.into(),
                FileEntry {
                    path: format!("{name}/{name}.typ"),
                    title: name.into(),
                    lifecycle: "live".into(),
                    expected_node: None,
                    editable: true,
                    importable: false,
                    inputs: vec![],
                },
            );
            inner.policy.users.get_mut("alice").unwrap().grants.insert(
                name.into(),
                super::super::Rights {
                    read: true,
                    write: true,
                    ..Default::default()
                },
            );
        }
        assert!(valid_config(&inner.config));
        drop(inner);
        (temp, state, auth)
    }
    fn request(kind: &str, source: Option<&str>, destination: Option<&str>) -> Prepare {
        let resource = if matches!(kind, "create" | "copy" | "template") {
            destination.unwrap()
        } else {
            source.unwrap()
        };
        Prepare {
            resource: resource.into(),
            operation_id: id(),
            kind: kind.into(),
            source: source.map(str::to_owned),
            destination: destination.map(str::to_owned),
            trash_operation_id: None,
            template: None,
            parameters: BTreeMap::new(),
            inputs: vec![],
            capture: None,
        }
    }
    fn grant(inner: &mut HostedInner, source: &str, kind: &str, destination: &str) {
        let r = &mut inner.policy.users.get_mut("alice").unwrap().structure;
        let s = r.sources.entry(source.into()).or_default();
        match kind {
            "rename" => s.rename = true,
            "copy" => s.copy = true,
            "trash" => s.trash = true,
            "restore" => s.restore = true,
            _ => {}
        }
        r.destinations.insert(
            destination.into(),
            DestinationRights {
                create: kind == "create",
                receive: kind != "create",
                members: super::super::Rights::default(),
            },
        );
    }
    fn execute(inner: &mut HostedInner, auth: &Auth, req: &Prepare) -> Value {
        let p = prepare_inner(inner, auth, req.clone()).unwrap();
        let actor = inner.actor(auth).unwrap();
        let actor_scope = scope(inner, &actor);
        let s = inner
            .core
            .with_actor(&actor_scope, |c| {
                c.actor_feature_commit_with_losses(
                    &req.resource,
                    &req.operation_id,
                    p["planId"].as_str().unwrap(),
                    true,
                )
            })
            .unwrap();
        safe_status(inner, &actor, s).unwrap()
    }
    #[test]
    fn independent_creation_read_and_original_request_fence() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let mut req = request("create", None, Some("Created"));
        req.capture = Some(Capture {
            title: Some("创建😀".into()),
            body: "#panic(\"literal\")".into(),
            ..Default::default()
        });
        assert_eq!(
            prepare_inner(&mut inner, &auth, req.clone())
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        grant(&mut inner, "Shared", "create", "Created");
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Created")
            .unwrap()
            .read = false;
        let p = prepare_inner(&mut inner, &auth, req.clone()).unwrap();
        assert_eq!(p["status"], "prepared");
        assert!(!inner.config.root.join("Created").exists());
        let repeat = prepare_inner(&mut inner, &auth, req.clone()).unwrap();
        assert_eq!(p["planId"], repeat["planId"]);
        let mut changed = req.clone();
        changed.capture.as_mut().unwrap().body.push_str("changed");
        assert_eq!(
            prepare_inner(&mut inner, &auth, changed)
                .unwrap_err()
                .status,
            StatusCode::CONFLICT
        );
        let result = execute(&mut inner, &auth, &req);
        assert_eq!(result["status"], "committed");
        assert_eq!(result["result"]["canOpen"], false);
        assert!(inner.open("alice", "Created").is_err());
        inner.policy.users.get_mut("reader").unwrap().grants.insert(
            "Created".into(),
            super::super::Rights {
                read: true,
                ..Default::default()
            },
        );
        let opened = inner.open("reader", "Created").unwrap();
        assert!(opened["source"].as_str().unwrap().contains("创建😀"));
        let source = opened["source"].as_str().unwrap();
        assert!(weftext_engine::inspect_metadata(source).syntax_valid);
    }
    #[test]
    fn raw_occupant_and_unrelated_local_producer_never_acquire_slot_read() {
        let (_temp, state, _auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let local = inner
            .core
            .create_note_fixture(&root, &root, "Created")
            .unwrap();
        inner.core.close_session(&local.session_id).unwrap();
        assert!(inner.open("alice", "Created").is_err());
        assert!(guard_slot(&inner, "Created").is_err());
        inner
            .config
            .structure_slots
            .get_mut("Created")
            .unwrap()
            .name_policy = NamePolicy::Fixed {
            name: "OtherName".into(),
        };
        assert!(!valid_config(&inner.config));
    }
    #[test]
    fn copy_requires_whole_tree_permissions_before_reading_source() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        grant(&mut inner, "Shared", "copy", "Copied");
        let req = request("copy", Some("Shared"), Some("Copied"));
        let path = inner.config.root.join("Shared/Shared.typ");
        let _lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(path)
            .unwrap();
        assert_eq!(
            prepare_inner(&mut inner, &auth, req.clone())
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .structure
            .sources
            .get_mut("Shared")
            .unwrap()
            .tree_read = true;
        assert_eq!(
            prepare_inner(&mut inner, &auth, req).unwrap_err().status,
            StatusCode::NOT_FOUND
        );
    }
    #[test]
    fn rename_then_read_uses_original_receipt_and_preserves_identity() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let original = owner(&inner, "Shared").unwrap().owner;
        grant(&mut inner, "Shared", "rename", "Renamed");
        let req = request("rename", Some("Shared"), Some("Renamed"));
        let result = execute(&mut inner, &auth, &req);
        assert_eq!(result["result"]["canOpen"], true);
        assert!(guard_slot(&inner, "Renamed").is_ok());
        assert_eq!(owner(&inner, "Renamed").unwrap().owner, original);
        assert!(!inner.config.root.join("Shared").exists());
    }
    #[test]
    fn trash_restore_uses_original_actor_receipt_not_global_trash() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let original = owner(&inner, "Shared").unwrap().owner;
        grant(&mut inner, "Shared", "trash", "Restored");
        grant(&mut inner, "Shared", "restore", "Restored");
        let trash = request("trash", Some("Shared"), None);
        execute(&mut inner, &auth, &trash);
        let mut restore = request("restore", Some("Shared"), Some("Restored"));
        restore.trash_operation_id = Some(id());
        assert_eq!(
            prepare_inner(&mut inner, &auth, restore.clone())
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        restore.trash_operation_id = Some(trash.operation_id);
        let result = execute(&mut inner, &auth, &restore);
        assert_eq!(result["result"]["canOpen"], true);
        assert_eq!(owner(&inner, "Restored").unwrap().owner, original);
    }
    #[test]
    fn markup_capture_rejects_executable_identity_paths_and_keeps_literal_raw() {
        for body in [
            "#metadata((secret: true)) <weftext>",
            "#let x = metadata\n#x((id: \"fake\"))",
            "#eval(\"metadata((secret: true))\")",
            "#import \"private.typ\"",
            "$eval(\"metadata\")$",
        ] {
            let c = Capture {
                body: body.into(),
                body_mode: Some("markup".into()),
                ..Default::default()
            };
            assert!(capture_body(&c).is_err(), "{body}");
        }
        for body in [
            "= Heading\n*strong* _emphasis_\n- item\n$x^2 + 1$",
            "```typst\n#metadata((secret: true)) <weftext>\n```",
        ] {
            let c = Capture {
                body: body.into(),
                body_mode: Some("markup".into()),
                ..Default::default()
            };
            assert_eq!(capture_body(&c).unwrap(), body);
        }
        let text = Capture {
            body: "#eval(\"metadata\")".into(),
            ..Default::default()
        };
        assert!(capture_body(&text).unwrap().contains("\\#"));
    }
    #[test]
    fn typed_task_capture_writes_only_required_or_native_default_fields() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        grant(&mut inner, "Shared", "create", "Created");
        inner
            .config
            .structure_slots
            .get_mut("Created")
            .unwrap()
            .types
            .insert("tasks/task".into());
        inner.policy.users.get_mut("alice").unwrap().library =
            serde_json::from_value(json!({"schema":{"read":true}})).unwrap();
        let catalog = typed_catalog(&mut inner, "alice", "Created").unwrap();
        let mut req = request("create", None, Some("Created"));
        req.capture = Some(Capture {
            types: vec!["tasks/task".into()],
            fields: BTreeMap::from([("tasks/status".into(), NativeValue::Text("open".into()))]),
            form_basis: Some(weftext_core::CaptureFormBasis {
                schema_ref: catalog["roleRef"].as_str().unwrap().into(),
                semantic_digest: catalog["semanticDigest"].as_str().unwrap().into(),
            }),
            ..Default::default()
        });
        assert_eq!(
            prepare_inner(&mut inner, &auth, req.clone())
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Created")
            .unwrap()
            .fields
            .insert(
                "tasks/status".into(),
                FieldRights {
                    read: true,
                    write: true,
                },
            );
        let mut stale = req.clone();
        stale
            .capture
            .as_mut()
            .unwrap()
            .form_basis
            .as_mut()
            .unwrap()
            .semantic_digest = "0".repeat(64);
        assert_eq!(
            prepare_inner(&mut inner, &auth, stale).unwrap_err().status,
            StatusCode::CONFLICT
        );
        assert!(!inner.config.root.join("Created").exists());
        let result = execute(&mut inner, &auth, &req);
        assert_eq!(result["result"]["canOpen"], true);
        let source = inner.open("alice", "Created").unwrap()["source"]
            .as_str()
            .unwrap()
            .to_owned();
        let p = weftext_data::project(&source, &Schema::bundled().unwrap());
        assert_eq!(p.types, vec!["tasks/task".to_owned()]);
        assert_eq!(
            p.fields
                .iter()
                .find(|f| f.field_id == "tasks/status")
                .unwrap()
                .cell
                .native(),
            Some(&NativeValue::Text("open".into()))
        );
    }
    #[test]
    fn future_resource_collection_uses_original_structure_slot_and_independent_grants() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        inner.config.resource_collections.insert(
            "CreatedAssets".into(),
            resource_collections::Collection {
                owner: "Created".into(),
                expected_owner: None,
                from_import_slot: false,
                from_structure_slot: true,
                lifecycle: "live".into(),
                title: "独立预授权资源集合".into(),
            },
        );
        assert!(resource_collections::valid_config(&inner.config));
        assert!(resource_collections::effective_owner(&inner, "CreatedAssets").is_err());
        grant(&mut inner, "Shared", "create", "Created");
        execute(&mut inner, &auth, &request("create", None, Some("Created")));
        let expected = owner(&inner, "Created").unwrap().owner;
        assert_eq!(
            resource_collections::effective_owner(&inner, "CreatedAssets").unwrap(),
            expected
        );
        assert!(resource_collections::capture(&inner, "alice", "CreatedAssets").is_err());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resource_collections
            .insert(
                "CreatedAssets".into(),
                resource_collections::Rights {
                    read: true,
                    ..Default::default()
                },
            );
        assert!(
            resource_collections::capture(&inner, "alice", "CreatedAssets")
                .unwrap()
                .entries
                .is_empty()
        );
        assert!(
            resource_collections::input_access(&inner, "alice", "CreatedAssets", true).is_err()
        );
        inner
            .config
            .resource_collections
            .get_mut("CreatedAssets")
            .unwrap()
            .from_import_slot = true;
        assert!(!resource_collections::valid_config(&inner.config));
    }
    #[test]
    fn registered_native_template_uses_exact_source_revision_and_parameters() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let opened = inner
            .core
            .open(root.join("Shared/Shared.typ"), Some(root.clone()))
            .unwrap();
        let source = opened.source.clone() + "\nTemplateLiteral\n";
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: opened.session_id.clone(),
                base_id: opened.base_id,
                source: source.clone(),
                operation_id: id(),
            })
            .unwrap();
        inner.core.close_session(&opened.session_id).unwrap();
        let q = owner(&inner, "Shared").unwrap();
        let from = source.find("TemplateLiteral").unwrap();
        let definition = weftext_knowledge::TemplateDefinition {
            id: "approved-template".into(),
            title: "私有模板题名不直接交付".into(),
            description: String::new(),
            source_node: q.owner,
            source_revision: q.revision.unwrap(),
            slots: vec![weftext_knowledge::TemplateSlot {
                name: "body".into(),
                range: weftext_knowledge::ExactRange::new(
                    &source,
                    from,
                    from + "TemplateLiteral".len(),
                )
                .unwrap(),
                kind: weftext_knowledge::TemplateSlotKind::Markup,
                default: None,
                nullable: false,
            }],
        };
        let config = inner
            .core
            .read_config(&root, ConfigRole::WorkspaceSettings)
            .unwrap();
        let _ = inner.core.take_snapshot_witness(&config.base_id);
        let settings=serde_json::to_vec(&json!({"version":1,"unrecognized":{"preserved":true},"templates":{"version":1,"entries":[definition]}})).unwrap();
        let plan = inner
            .core
            .prepare_config_save(
                &root,
                ConfigRole::WorkspaceSettings,
                config.bytes,
                settings,
                &id(),
            )
            .unwrap();
        inner
            .core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        inner.config.structure_templates.insert(
            "approved".into(),
            Template {
                source: "Shared".into(),
                template_id: "approved-template".into(),
            },
        );
        let r = &mut inner.policy.users.get_mut("alice").unwrap().structure;
        r.sources.insert(
            "Shared".into(),
            SourceRights {
                template: true,
                tree_read: true,
                tree_export: true,
                ..Default::default()
            },
        );
        r.destinations.insert(
            "Copied".into(),
            DestinationRights {
                receive: true,
                ..Default::default()
            },
        );
        inner.policy.users.get_mut("alice").unwrap().library =
            serde_json::from_value(json!({"schema":{"read":true}})).unwrap();
        let descriptor = template_catalog(&mut inner, "alice", "approved").unwrap();
        assert_eq!(descriptor["parameters"][0]["name"], "body");
        assert_eq!(descriptor["title"], "Shared");
        let mut req = request("template", Some("Shared"), Some("Copied"));
        req.template = Some("approved".into());
        req.parameters
            .insert("body".into(), "#panic(\"literal\") 😀".into());
        let result = execute(&mut inner, &auth, &req);
        assert_eq!(result["result"]["canOpen"], true);
        let copied = inner.open("alice", "Copied").unwrap();
        let compiled = weftext_engine::inspect_metadata(copied["source"].as_str().unwrap());
        assert!(compiled.syntax_valid);
        assert!(copied["source"].as_str().unwrap().contains("literal"));
        assert!(
            std::fs::read_to_string(root.join("Shared/Shared.typ"))
                .unwrap()
                .contains("TemplateLiteral")
        );
    }
    #[test]
    fn imported_members_need_explicit_structure_rights_and_keep_original_trash_status() {
        let (_temp, state, auth) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        inner.config.content_import_slots.insert(
            "Native".into(),
            content_import::Slot {
                parent: "root".into(),
                name: "Native".into(),
                title: "Approved import".into(),
                inputs: vec![],
            },
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .content_imports
            .insert(
                "Native".into(),
                content_import::Rights {
                    members: super::super::Rights {
                        read: true,
                        write: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        let bytes = inner.core.capture_native_export(&root).unwrap().bytes;
        let import_id = id();
        let import_scope = format!("{}:content-import", inner.actor_scope("alice"));
        let p = inner
            .core
            .with_actor_for_root(&import_scope, &root, |c| {
                c.actor_feature_prepare_content_import(
                    "Native",
                    &import_id,
                    &digest(b"original-import"),
                    OperationRequest {
                        operation_id: import_id.clone(),
                        root: root.to_string_lossy().into_owned(),
                        intent: OperationIntent::ImportNative {
                            parent: root.to_string_lossy().into_owned(),
                            name: "Native".into(),
                            bytes,
                            allow_losses: true,
                        },
                    },
                    json!({}),
                    &[],
                    "Native",
                )
            })
            .unwrap();
        inner
            .core
            .with_actor_for_root(&import_scope, &root, |c| {
                c.actor_feature_commit_with_losses(
                    "Native",
                    &import_id,
                    p.plan_id.as_deref().unwrap(),
                    true,
                )
            })
            .unwrap();
        let member = content_import::slot_member_ids(&inner, "Native")
            .unwrap()
            .remove(0);
        let trash = request("trash", Some(&member), None);
        assert_eq!(
            prepare_inner(&mut inner, &auth, trash.clone())
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .structure
            .sources
            .insert(
                "Native/*".into(),
                SourceRights {
                    trash: true,
                    ..Default::default()
                },
            );
        assert!(valid_rights(
            &inner.config,
            &inner.policy.users["alice"].structure
        ));
        let expanded = rights(&inner, "alice").unwrap();
        assert!(expanded.sources[&member].trash);
        assert!(!expanded.sources[&member].copy);
        assert!(!expanded.sources[&member].tree_read);
        assert!(!expanded.sources[&member].tree_export);
        let unknown = format!("Native/{}", id());
        assert_eq!(
            prepare_inner(&mut inner, &auth, request("trash", Some(&unknown), None))
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        let prepared = prepare_inner(&mut inner, &auth, trash.clone()).unwrap();
        let actor_scope = scope(&inner, "alice");
        let committed = inner
            .core
            .with_actor_for_root(&actor_scope, &root, |c| {
                c.actor_feature_commit_with_losses(
                    &member,
                    &trash.operation_id,
                    prepared["planId"].as_str().unwrap(),
                    true,
                )
            })
            .unwrap();
        assert_eq!(
            safe_status(&mut inner, "alice", committed).unwrap()["status"],
            "committed"
        );
        assert!(owner(&inner, &member).is_err());
        assert!(prepare_inner(&mut inner, &auth, request("trash", Some(&member), None)).is_err());
        let original = inner
            .core
            .with_actor_for_root(&actor_scope, &root, |c| {
                c.actor_feature_status(&member, &trash.operation_id)
            })
            .unwrap()
            .unwrap();
        let replay = safe_status(&mut inner, "alice", original).unwrap();
        assert_eq!(replay["status"], "committed");
        assert_eq!(replay["preview"]["kind"], "trash");
        assert_eq!(replay["result"]["status"], "trash");
    }
}
