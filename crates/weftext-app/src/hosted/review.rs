//! Resource-scoped hosted review. Only Core commits the frozen author files.
//! Stored discussions and current target context have independent permissions.
use super::*;
use std::path::Path;
use weftext_core::{
    ActorFeatureStatus, FileChange, OperationIntent, OperationRequest, SnapshotFile,
};
use weftext_knowledge as knowledge;

const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(900);

#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ReviewRights {
    #[serde(default)]
    pub(super) read: bool,
    #[serde(default)]
    pub(super) write: bool,
    #[serde(default)]
    pub(super) target: bool,
    #[serde(default)]
    pub(super) accept: bool,
    #[serde(default)]
    pub(super) proofread: bool,
}

#[derive(Clone)]
struct Basis {
    actor: String,
    epoch: u64,
    resource: String,
    annotations: SnapshotFile,
    source: Option<SnapshotFile>,
    node: Option<knowledge::NodeSnapshot>,
    media: Option<Arc<media::Context>>,
    expires: Instant,
}
impl Basis {
    fn bytes(&self) -> usize {
        self.annotations.bytes.as_ref().map_or(0, Vec::len)
            + self
                .source
                .as_ref()
                .and_then(|f| f.bytes.as_ref())
                .map_or(0, Vec::len)
            + self.node.as_ref().map_or(0, |n| n.source.len())
            + self.media.as_ref().map_or(0, |m| m.retained_bytes())
    }
}
#[derive(Clone)]
struct Scan {
    actor: String,
    epoch: u64,
    resource: String,
    basis: String,
    options_hash: String,
    result: knowledge::ProofreadResult,
    inputs: BTreeMap<String, String>,
    dependencies: Vec<SnapshotFile>,
    environment: library::Environment,
    expires: Instant,
}
impl Scan {
    fn bytes(&self) -> usize {
        self.dependencies
            .iter()
            .map(|f| f.bytes.as_ref().map_or(0, Vec::len))
            .sum::<usize>()
            + serde_json::to_vec(&self.result).map_or(MAX_BYTES, |b| b.len())
            + self.environment.retained_bytes()
    }
}
#[derive(Default)]
pub(super) struct Store {
    bases: HashMap<String, Basis>,
    scans: HashMap<String, Scan>,
}
impl Store {
    fn bytes(&self) -> usize {
        self.bases.values().map(Basis::bytes).sum::<usize>()
            + self.scans.values().map(Scan::bytes).sum::<usize>()
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Needs {
    write: bool,
    source_read: bool,
    source_write: bool,
    target: bool,
    accept: bool,
    proofread: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Decision {
    actor_epoch: u64,
    expires_at_unix_ms: u64,
    config_hash: String,
    needs: Needs,
    #[serde(default)]
    execution: library::ExecutionAuthority,
    #[serde(default)]
    media: Option<MediaDecision>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MediaDecision {
    asset: String,
    reference: String,
    revision: String,
    binding: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    resource: String,
    #[serde(default)]
    asset: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proofread {
    resource: String,
    basis: String,
    options: knowledge::ProofreadOptions,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Prepare {
    resource: String,
    basis: String,
    operation_id: String,
    intent: Value,
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
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/review", get(list))
        .route("/api/hosted/review/read", post(read))
        .route("/api/hosted/review/proofread", post(proofread))
        .route("/api/hosted/review/prepare", post(prepare))
        .route("/api/hosted/review/commit", post(commit))
        .route("/api/hosted/review/operation", post(operation))
        .route("/api/hosted/review/settle", post(settle))
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    let current = |actor: &str, epoch| {
        inner
            .policy
            .users
            .get(actor)
            .is_some_and(|u| u.enabled && u.epoch == epoch)
    };
    let expired = inner
        .review
        .bases
        .values()
        .filter(|b| b.expires <= Instant::now() || !current(&b.actor, b.epoch))
        .flat_map(snapshot_ids)
        .chain(
            inner
                .review
                .scans
                .values()
                .filter(|s| s.expires <= Instant::now() || !current(&s.actor, s.epoch))
                .flat_map(|s| s.dependencies.iter().map(|f| f.base_id.clone())),
        )
        .collect::<BTreeSet<_>>();
    inner
        .review
        .bases
        .retain(|_, b| b.expires > Instant::now() && current(&b.actor, b.epoch));
    inner
        .review
        .scans
        .retain(|_, s| s.expires > Instant::now() && current(&s.actor, s.epoch));
    release_unretained(inner, expired);
}
fn release_unretained(inner: &mut HostedInner, ids: BTreeSet<String>) {
    let retained = inner
        .review
        .bases
        .values()
        .flat_map(snapshot_ids)
        .chain(
            inner
                .review
                .scans
                .values()
                .flat_map(|s| s.dependencies.iter().map(|f| f.base_id.clone())),
        )
        .collect::<BTreeSet<_>>();
    for id in ids.difference(&retained) {
        let _ = inner.core.take_snapshot_witness(id);
    }
}
fn snapshot_ids(b: &Basis) -> impl Iterator<Item = String> + '_ {
    std::iter::once(b.annotations.base_id.clone()).chain(b.source.iter().map(|f| f.base_id.clone()))
}
fn scope(inner: &HostedInner, actor: &str) -> String {
    format!("{}:review", inner.actor_scope(actor))
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
        "reviewBasisChanged",
        "批注、原文或检查依据已变化，请保留输入并重新读取核对。",
    )
}
fn core_error(_: CoreError) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "reviewOperation",
        "原批注操作未完成或依据已变化；请核对原操作结果，输入保持保留。",
    )
}
fn knowledge_error(error: knowledge::KnowledgeError) -> ApiError {
    // Never forward parser diagnostics containing sidecar preimages or private paths.
    ApiError::new(
        StatusCode::CONFLICT,
        &error.code,
        "当前批注或目标不能安全执行此操作；原记录与输入仍保留。",
    )
}
fn text<'a>(input: &'a Value, key: &str) -> Result<&'a str, ApiError> {
    input[key]
        .as_str()
        .ok_or_else(|| ApiError::bad(format!("缺少 {key}")))
}
fn access(inner: &HostedInner, actor: &str, resource: &str) -> Result<Rights, ApiError> {
    let rights = inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.grants.get(resource))
        .filter(|r| r.annotations.read)
        .cloned()
        .ok_or_else(unavailable)?;
    if !inner
        .config
        .files
        .get(resource)
        .is_some_and(|f| f.editable || f.lifecycle == "trash")
    {
        return Err(unavailable());
    }
    Ok(rights)
}
fn authorize(rights: &Rights, n: &Needs) -> Result<(), ApiError> {
    let r = &rights.annotations;
    if !r.read
        || n.write && !r.write
        || n.source_read && !rights.read
        || n.source_write && !rights.write
        || n.target && !r.target
        || n.accept && !r.accept
        || n.proofread && !r.proofread
    {
        return Err(unavailable());
    }
    Ok(())
}
fn needs(kind: &str) -> Result<Needs, ApiError> {
    let mut n = Needs {
        write: true,
        ..Needs::default()
    };
    match kind {
        "annotationReply" | "annotationResolve" | "annotationReject" => {}
        "annotationCreate" | "annotationReattach" | "annotationReconfirm" => {
            n.source_read = true;
            n.target = true;
        }
        "annotationAccept" => {
            n.source_read = true;
            n.source_write = true;
            n.target = true;
            n.accept = true;
        }
        "annotationProofread" => {
            n.source_read = true;
            n.target = true;
            n.proofread = true;
        }
        _ => return Err(ApiError::bad("未知托管批注操作")),
    }
    Ok(n)
}
fn is_media_target(value: &Value) -> bool {
    matches!(
        value["kind"].as_str(),
        Some(
            "whole_resource"
                | "wholeResource"
                | "image_region"
                | "imageRegion"
                | "pdf_region"
                | "pdfRegion"
                | "media_interval"
                | "mediaInterval"
        )
    )
}
fn authorize_media(
    inner: &HostedInner,
    actor: &str,
    owner: &str,
    d: &MediaDecision,
) -> Result<(), ApiError> {
    media::input_access(inner, actor, &d.asset, false)?;
    let configured = inner
        .config
        .resources
        .get(&d.asset)
        .filter(|r| r.owner == owner && r.reference == d.reference && r.lifecycle == "live")
        .ok_or_else(unavailable)?;
    let _ = configured;
    if media::binding_hash(inner, &BTreeSet::from([d.asset.clone()]))? != d.binding {
        return Err(changed());
    }
    media::exact(inner, actor, &d.asset, &d.revision)?;
    Ok(())
}
fn media_target(
    inner: &HostedInner,
    actor: &str,
    b: &Basis,
    input: &Value,
) -> Result<
    (
        knowledge::AnnotationTarget,
        MediaDecision,
        Vec<weftext_core::SnapshotWitness>,
    ),
    ApiError,
> {
    let asset = text(input, "asset")?;
    let revision = text(input, "revision")?;
    let m = b
        .media
        .as_ref()
        .filter(|m| m.asset == asset && m.resource.revision.as_deref() == Some(revision))
        .ok_or_else(changed)?;
    let configured = inner
        .config
        .resources
        .get(asset)
        .filter(|r| r.owner == b.resource && r.lifecycle == "live")
        .ok_or_else(unavailable)?;
    if input
        .get("resource")
        .is_some_and(|r| r.as_str() != Some(configured.reference.as_str()))
    {
        return Err(ApiError::bad(
            "所选 asset 与原稳定资源引用不一致；未改写目标",
        ));
    }
    media::validate_original(inner, actor, asset, &m.resource)?;
    let target = crate::knowledge_api::target_from_resource_metadata(
        input,
        &configured.reference,
        revision,
        &m.metadata,
    )?;
    let decision = MediaDecision {
        asset: asset.into(),
        reference: configured.reference.clone(),
        revision: revision.into(),
        binding: media::binding_hash(inner, &BTreeSet::from([asset.into()]))?,
    };
    let mut witnesses = vec![];
    Core::merge_resource_witness(&mut witnesses, &m.resource.witness).map_err(core_error)?;
    Ok((target, decision, witnesses))
}
fn rights_json(rights: &Rights) -> Value {
    let mut value = serde_json::to_value(&rights.annotations).unwrap();
    value["sourceRead"] = json!(rights.read);
    value["sourceWrite"] = json!(rights.write);
    value
}
fn config_hash(inner: &HostedInner, resource: &str) -> Result<String, ApiError> {
    let file = inner.config.files.get(resource).ok_or_else(unavailable)?;
    let mut ids = file.inputs.iter().cloned().collect::<BTreeSet<_>>();
    ids.insert(resource.into());
    inputs::binding_hash(inner, &ids)
}
fn document(b: &Basis) -> Result<knowledge::AnnotationDocument, ApiError> {
    knowledge::decode_annotations(
        b.annotations
            .bytes
            .as_deref()
            .unwrap_or(b"{\"version\":1,\"annotations\":[]}"),
    )
    .map_err(knowledge_error)
}
fn capture(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    source_needed: bool,
) -> Result<Basis, ApiError> {
    let source_path = inner.config.root.join(&inner.config.files[resource].path);
    let annotation_path = source_path
        .parent()
        .ok_or_else(unavailable)?
        .join("weftext.annotations.json");
    let root = inner.config.root.clone();
    let actor_scope = scope(inner, actor);
    let (annotations, source, node) = inner
        .core
        .with_actor(&actor_scope, |core| {
            let annotations = core
                .read_snapshot(&root, &[annotation_path.to_string_lossy().into_owned()])?
                .files
                .remove(0);
            if !source_needed {
                return Ok((annotations, None, None));
            }
            // Target failure cannot hide otherwise authorized stored discussion.
            let source = core
                .read_snapshot(&root, &[source_path.to_string_lossy().into_owned()])
                .ok()
                .and_then(|mut s| s.files.pop());
            let node = match (&source, core.open(&source_path, Some(root.clone()))) {
                (Some(f), Ok(doc)) => {
                    let node = doc
                        .node_id
                        .zip(doc.workspace_id)
                        .zip(f.bytes.as_ref())
                        .and_then(|((node_id, workspace_id), bytes)| {
                            if f.node_id.as_ref() != Some(&node_id)
                                || doc.source.as_bytes() != bytes
                            {
                                return None;
                            }
                            Some(knowledge::NodeSnapshot {
                                owner: knowledge::NodeRef {
                                    workspace_id,
                                    node_id,
                                },
                                path: f.path.clone(),
                                library_path: String::new(),
                                revision_id: f.revision.clone().unwrap_or_default(),
                                source: doc.source.clone(),
                                title: None,
                                aliases: vec![],
                                labels: vec![],
                                labels_complete: false,
                            })
                        });
                    core.close_session(&doc.session_id)?;
                    node
                }
                (_, Ok(doc)) => {
                    core.close_session(&doc.session_id)?;
                    None
                }
                (_, Err(_)) => None,
            };
            Ok((annotations, source, node))
        })
        .map_err(core_error)?;
    if annotations
        .bytes
        .as_ref()
        .is_some_and(|b| b.len() > MAX_DOCUMENT_BYTES)
        || source
            .as_ref()
            .and_then(|f| f.bytes.as_ref())
            .is_some_and(|b| b.len() > MAX_DOCUMENT_BYTES)
    {
        let _ = inner.core.take_snapshot_witness(&annotations.base_id);
        if let Some(file) = &source {
            let _ = inner.core.take_snapshot_witness(&file.base_id);
        }
        return Err(ApiError::bad("批注或原文超过本次读取预算"));
    }
    Ok(Basis {
        actor: actor.into(),
        epoch: inner.actor_generation(actor),
        resource: resource.into(),
        annotations,
        source,
        node,
        media: None,
        expires: Instant::now() + TTL,
    })
}
fn basis(
    inner: &mut HostedInner,
    actor: &str,
    resource: &str,
    id: &str,
    source: bool,
) -> Result<Basis, ApiError> {
    expire_revoked(inner);
    let b = inner
        .review
        .bases
        .get(id)
        .filter(|b| {
            b.actor == actor && b.resource == resource && b.epoch == inner.actor_generation(actor)
        })
        .cloned()
        .ok_or_else(changed)?;
    validate_basis(inner, &b, source)?;
    Ok(b)
}
fn validate_basis(inner: &HostedInner, b: &Basis, source: bool) -> Result<(), ApiError> {
    if b.expires <= Instant::now() || b.epoch != inner.actor_generation(&b.actor) {
        return Err(changed());
    }
    inner
        .core
        .validate_snapshot(
            &inner.config.root,
            &b.annotations.path,
            &b.annotations.base_id,
        )
        .map_err(|_| changed())?;
    if source {
        let file = b
            .source
            .as_ref()
            .filter(|f| f.revision.is_some())
            .ok_or_else(changed)?;
        if b.node.is_none() {
            return Err(changed());
        }
        inner
            .core
            .validate_snapshot(&inner.config.root, &file.path, &file.base_id)
            .map_err(|_| changed())?;
    }
    Ok(())
}

fn projection(
    doc: &knowledge::AnnotationDocument,
    node: Option<&knowledge::NodeSnapshot>,
    target: bool,
    media: Option<&media::Context>,
) -> Value {
    let views = knowledge::annotation_views(doc, node, &[]);
    let items: Vec<_> = views.iter().filter(|v| v.reply_to.is_none()).map(|v| {
        let record = doc.records.iter().find(|r| r.annotation_id == v.annotation_id).unwrap();
        let own = target && node.is_some_and(|n| match &record.target { knowledge::AnnotationTarget::WholeDocument { node } => node == &n.owner, _ => record.target.anchor().is_some_and(|a| a.node == n.owner) });
        let range = (own && matches!(v.target_state.as_str(), "exact" | "mapped")).then(|| record.target.anchor().and_then(|a| knowledge::resolve_anchor(a, node, &[]).current_range)).flatten();
        let whole = own && matches!(record.target, knowledge::AnnotationTarget::WholeDocument { .. });
        let media_own=target && media.is_some_and(|m|resource_basis(&record.target).is_some_and(|(reference,revision)|reference==m.resource.resource_ref && Some(revision)==m.resource.revision.as_deref())
            && (matches!(&record.target,knowledge::AnnotationTarget::WholeResource {..}) || crate::media_api::metadata_facts(&m.metadata,&m.resource.resource_ref,revision_from(&m.resource)).is_some_and(|facts|knowledge::validate_resource_target(&record.target,&facts).is_ok())));
        let state = if media_own {"exact"} else if !target || !own { "unavailable" } else if whole { "exact" } else { &v.target_state };
        let verified = own && matches!(state, "exact" | "mapped") && (whole || range.is_some());
        let replies: Vec<_> = views.iter().filter(|reply| reply.reply_to.is_some() && thread_root(doc, &reply.annotation_id) == Some(v.annotation_id.as_str())).map(|reply| json!({"id":reply.annotation_id,"replyTo":reply.reply_to,"body":reply.body,"author":reply.attribution,"createdAt":reply.created_at})).collect();
        let mut item = json!({"id":v.annotation_id,"annotationId":v.annotation_id,"body":v.body,"purpose":v.purpose,"reviewState":v.review_state,"suggestionState":v.suggestion_state,"author":v.attribution,"createdAt":v.created_at,"verified":verified,"targetResolution":state,"replies":replies});
        if let Some(range) = &range {
            item["visibleExcerpt"] = json!(range.raw);
            if verified { item["sourceLocation"] = json!({"from":range.utf16_from,"to":range.utf16_to}); }
        }
        if own && let Some(suggestion) = &record.suggestion { item["replacement"] = json!(suggestion.replacement); }
        if media_own && let Some(m)=media {item["verified"]=true.into();item["resourceLocation"]=json!({"asset":m.asset,"resource":m.resource.resource_ref,"revision":m.resource.revision,"target":record.target});}
        item
    }).collect();
    json!(items)
}
fn resource_basis(target: &knowledge::AnnotationTarget) -> Option<(&str, &str)> {
    match target {
        knowledge::AnnotationTarget::WholeResource {
            resource,
            revision_id,
        }
        | knowledge::AnnotationTarget::ImageRegion {
            resource,
            revision_id,
            ..
        }
        | knowledge::AnnotationTarget::PdfRegion {
            resource,
            revision_id,
            ..
        }
        | knowledge::AnnotationTarget::PdfTextRange {
            resource,
            revision_id,
            ..
        }
        | knowledge::AnnotationTarget::MediaInterval {
            resource,
            revision_id,
            ..
        } => Some((resource, revision_id)),
        _ => None,
    }
}
fn revision_from(resource: &weftext_core::QualifiedResource) -> &str {
    resource.revision.as_deref().unwrap_or_default()
}
fn thread_root<'a>(doc: &'a knowledge::AnnotationDocument, id: &'a str) -> Option<&'a str> {
    let mut id = id;
    for _ in 0..=doc.records.len() {
        let record = doc.records.iter().find(|r| r.annotation_id == id)?;
        match record.reply_to.as_deref() {
            Some(parent) => id = parent,
            None => return Some(id),
        }
    }
    None
}
async fn list(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let resources: Vec<_> = inner.policy.users[&actor]
                .grants
                .iter()
                .filter(|(id, r)| {
                    r.annotations.read
                        && inner
                            .config
                            .files
                            .get(*id)
                            .is_some_and(|f| f.editable || f.lifecycle == "trash")
                })
                .map(|(id, r)| json!({"resource":id,"rights":rights_json(r)}))
                .collect();
            Ok(json!({"resources":resources}))
        })
        .await?,
    ))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Read>,
) -> ApiResult {
    Ok(Json(locked(&state, move |inner| {
        let actor = inner.actor(&auth)?; let rights = access(inner, &actor, &req.resource)?; expire_revoked(inner);
        if let Some(asset)=&req.asset {media::input_access(inner,&actor,asset,false)?;if !inner.config.resources.get(asset).is_some_and(|r|r.owner==req.resource){return Err(unavailable());}}
        if inner.review.bases.len() >= 128 || inner.review.bases.values().filter(|b|b.actor==actor).count() >= 32 { return Err(ApiError::bad("批注依据已达上限，请稍后重新读取")); }
        let live=inner.config.files[&req.resource].lifecycle=="live";
        let mut b = capture(inner, &actor, &req.resource, live && (rights.read || (req.asset.is_none() && rights.annotations.target)))?;
        let cleanup=snapshot_ids(&b).collect::<Vec<_>>();
        let result=(|| {
        let mut media_unavailable=None;
        if let Some(asset)=&req.asset {
            media::input_access(inner,&actor,asset,false)?;
            let configured=inner.config.resources.get(asset).filter(|r|r.owner==req.resource).ok_or_else(unavailable)?;
            match media::context(inner,&actor,asset,None) {
                Ok(context)=>b.media=Some(Arc::new(context)),
                Err(error) if error.status==StatusCode::CONFLICT=>media_unavailable=Some(json!({"asset":asset,"owner":configured.owner,"title":configured.title,"name":configured.title,"status":"unavailable"})),
                Err(error)=>return Err(error),
            }
        }
        let total = inner.review.bytes();
        if total.saturating_add(b.bytes()) > MAX_BYTES { return Err(ApiError::bad("批注依据缓存已达预算，请稍后重新读取")); }
        inner.core.validate_snapshot(&inner.config.root, &b.annotations.path, &b.annotations.base_id).map_err(|_|changed())?;
        if let Some(source) = &b.source { inner.core.validate_snapshot(&inner.config.root, &source.path, &source.base_id).map_err(|_|changed())?; }
        let doc = document(&b);
        let mut response = json!({"resource":req.resource,"basis":null,"annotations":[],"rights":rights_json(&rights),"editable":doc.is_ok()});
        if !live {response["rights"]["sourceRead"]=false.into();response["rights"]["sourceWrite"]=false.into();}
        if rights.read && let Some(node) = &b.node { response["sourceHash"] = json!(digest(node.source.as_bytes())); }
        if let Some(m)=&b.media {media::validate_original(inner,&actor,&m.asset,&m.resource)?;response["media"]=m.descriptor.clone();}
        if let Some(unavailable)=media_unavailable {response["media"]=unavailable;}
        match doc {
            Ok(doc) => {
                response["annotations"] = projection(&doc, b.node.as_ref(), rights.annotations.target, b.media.as_deref());
                let id = uuid::Uuid::new_v4().to_string(); response["basis"] = json!(id); inner.review.bases.insert(id, b);
            },
            Err(_) => {
                // Salvage only discussion; damaged/unknown targets cannot be written.
                let recovered = knowledge::recover_discussion(b.annotations.bytes.as_deref().unwrap_or_default());
                response["annotations"] = json!(recovered.into_iter().map(|v|json!({"id":v.annotation_id,"annotationId":v.annotation_id,"body":v.body,"purpose":v.purpose,"reviewState":v.review_state,"suggestionState":v.suggestion_state,"author":v.attribution,"createdAt":v.created_at,"verified":false,"targetResolution":"unavailable","replies":[]})).collect::<Vec<_>>());
                response["warning"] = json!("批注记录受损或格式尚不支持；原文件保留，讨论仅供只读核对。");
            },
        }
        if response["basis"].is_null() {for id in &cleanup {let _=inner.core.take_snapshot_witness(id);}}
        Ok(response)
        })();
        if result.is_err() {for id in cleanup {let _=inner.core.take_snapshot_witness(&id);}}
        result
    }).await?))
}

fn existing(
    doc: &knowledge::AnnotationDocument,
    finding: &knowledge::ProofreadFinding,
    node: &knowledge::NodeSnapshot,
) -> bool {
    doc.records.iter().any(|r| {
        r.unknown.get("proofreading").is_some_and(|p| {
            p["ruleId"] == finding.rule_id && p["ruleVersion"] == finding.rule_version
        }) && r
            .target
            .anchor()
            .and_then(|a| knowledge::resolve_anchor(a, Some(node), &[]).current_range)
            .zip(finding.source_range.as_ref())
            .is_some_and(|(a, b)| a.from == b.from && a.to == b.to && a.raw == b.raw)
    })
}
async fn proofread(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Proofread>,
) -> ApiResult {
    let check = auth.clone();
    let resource = req.resource.clone();
    let basis_id = req.basis.clone();
    let (b, input, inputs, dependencies, environment) = locked(&state, move |inner| {
        let actor = inner.actor(&check)?;
        let rights = access(inner, &actor, &resource)?;
        authorize(
            &rights,
            &Needs {
                source_read: true,
                target: true,
                proofread: true,
                ..Needs::default()
            },
        )?;
        let b = basis(inner, &actor, &resource, &basis_id, true)?;
        let (input, inputs, environment) =
            editor::freeze(inner, &actor, &resource, &b.node.as_ref().unwrap().source)?;
        let paths: Vec<_> = inputs
            .keys()
            .map(|p| inner.config.root.join(p).to_string_lossy().into_owned())
            .collect();
        let dependencies = if paths.is_empty() {
            vec![]
        } else {
            inner
                .core
                .read_snapshot(&inner.config.root, &paths)
                .map_err(core_error)?
                .files
        };
        for (path, hash) in &inputs {
            if !dependencies.iter().any(|f| {
                Path::new(&f.path) == inner.config.root.join(path)
                    && f.bytes.as_ref().is_some_and(|b| digest(b) == *hash)
            }) {
                return Err(changed());
            }
        }
        Ok((b, input, inputs, dependencies, environment))
    })
    .await?;
    let output = editor::compile_frozen(&state, input).await?;
    if output["ok"] != true {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "reviewPreviewUnavailable",
            "当前获权正文未能编译，校对未完成。",
        ));
    }
    let tree: weftext_engine::RenderNode = serde_json::from_value(output["tree"].clone())
        .map_err(|_| ApiError::bad("当前编译缺少准确正文树"))?;
    let source = b.node.as_ref().unwrap().source.clone();
    let options = req.options.clone();
    let result =
        tokio::task::spawn_blocking(move || knowledge::proofread(&tree, &source, &options))
            .await
            .map_err(|_| ApiError::internal("校对未完成"))?
            .map_err(knowledge_error)?;
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            let rights = access(inner, &actor, &req.resource)?;
            authorize(
                &rights,
                &Needs {
                    source_read: true,
                    target: true,
                    proofread: true,
                    ..Needs::default()
                },
            )?;
            if actor != b.actor {
                return Err(changed());
            }
            validate_basis(inner, &b, true)?;
            editor::validate_inputs(inner, &actor, &req.resource, &inputs)?;
            environment.recheck(inner, &actor)?;
            for f in &dependencies {
                inner
                    .core
                    .validate_snapshot(&inner.config.root, &f.path, &f.base_id)
                    .map_err(|_| changed())?;
            }
            expire_revoked(inner);
            if inner.review.scans.len() >= 32
                || inner
                    .review
                    .scans
                    .values()
                    .filter(|s| s.actor == actor)
                    .count()
                    >= 8
            {
                return Err(ApiError::bad("校对结果缓存已达上限，请稍后重新检查"));
            }
            let doc = document(&b)?;
            let node = b.node.as_ref().unwrap();
            let existing: Vec<_> = result
                .findings
                .iter()
                .filter(|f| existing(&doc, f, node))
                .map(|f| f.id.clone())
                .collect();
            let mut response = value(&result)?;
            if let Some(findings) = response["findings"].as_array_mut() {
                for f in findings {
                    if let Some(r) = f["sourceRange"].as_object_mut() {
                        r.remove("raw");
                    }
                }
            }
            let id = uuid::Uuid::new_v4().to_string();
            response["scanId"] = json!(id);
            response["existing"] = json!(existing);
            response["sourceHash"] = json!(digest(node.source.as_bytes()));
            let scan = Scan {
                actor,
                epoch: b.epoch,
                resource: req.resource,
                basis: req.basis,
                options_hash: digest(&serde_json::to_vec(&req.options).map_err(|_| changed())?),
                result,
                inputs,
                dependencies,
                environment,
                expires: Instant::now() + TTL,
            };
            if inner.review.bytes().saturating_add(scan.bytes()) > MAX_BYTES {
                return Err(ApiError::bad("校对依据缓存已达预算，请稍后重新检查"));
            }
            inner.review.scans.insert(id, scan);
            Ok(response)
        })
        .await?,
    ))
}

fn utf16_byte(source: &str, offset: usize) -> Result<usize, ApiError> {
    let mut units = 0;
    for (byte, ch) in source.char_indices() {
        if units == offset {
            return Ok(byte);
        }
        units += ch.len_utf16();
        if units > offset {
            return Err(ApiError::bad("选区分割了 UTF-16 字符"));
        }
    }
    if units == offset {
        Ok(source.len())
    } else {
        Err(ApiError::bad("选区超出当前已保存正文"))
    }
}
fn target(
    input: &Value,
    node: &knowledge::NodeSnapshot,
) -> Result<knowledge::AnnotationTarget, ApiError> {
    match text(input, "kind")? {
        "whole_node" | "whole_document" | "wholeDocument" => {
            Ok(knowledge::AnnotationTarget::WholeDocument {
                node: node.owner.clone(),
            })
        }
        "source_range" | "sourceRange" | "source_point" | "sourcePoint" => {
            let offset = |key| {
                input[key]
                    .as_u64()
                    .and_then(|v| usize::try_from(v).ok())
                    .ok_or_else(|| ApiError::bad("需要准确 UTF-16 选区"))
            };
            let from = utf16_byte(&node.source, offset("from")?)?;
            let to = utf16_byte(&node.source, offset("to")?)?;
            let point = matches!(input["kind"].as_str(), Some("source_point" | "sourcePoint"));
            if (!point && from >= to) || (point && from != to) {
                return Err(ApiError::bad("范围需要非空选区，插入点需要相同起止位置"));
            }
            let range =
                knowledge::ExactRange::new(&node.source, from, to).map_err(knowledge_error)?;
            if point && !knowledge::writable_suggestion_range(&node.source, &range) {
                return Err(ApiError::bad("插入点需要准确可写的普通正文位置"));
            }
            let anchor =
                knowledge::source_anchor(node, from, to, range.raw).map_err(knowledge_error)?;
            Ok(knowledge::AnnotationTarget::SourceRange { anchor })
        }
        _ => Err(ApiError::bad(
            "当前托管批注支持准确源范围、源插入点及整篇笔记",
        )),
    }
}
fn record(
    actor: &str,
    target: knowledge::AnnotationTarget,
    body: String,
    purpose: String,
    replacement: Option<String>,
) -> knowledge::AnnotationRecord {
    let confirmed_revision = target
        .anchor()
        .map(|a| a.revision_id.clone())
        .unwrap_or_default();
    knowledge::AnnotationRecord {
        annotation_id: uuid::Uuid::new_v4().to_string(),
        reply_to: None,
        attribution: actor.into(),
        created_at: format!("unix:{}", now_ms() / 1000),
        purpose,
        body,
        labels: vec![],
        appearance: json!({}),
        review_state: "open".into(),
        target,
        suggestion: replacement.map(|replacement| knowledge::Suggestion {
            replacement,
            state: "pending".into(),
            confirmed_revision,
            requires_reconfirmation: false,
        }),
        target_history: vec![],
        unknown: BTreeMap::new(),
    }
}
fn change(file: &SnapshotFile, after: Vec<u8>) -> FileChange {
    FileChange {
        path: file.path.clone(),
        base_id: Some(file.base_id.clone()),
        before: file.bytes.clone(),
        after,
    }
}
fn safe_status(
    inner: &HostedInner,
    actor: &str,
    rights: &Rights,
    status: ActorFeatureStatus,
) -> Result<Value, ApiError> {
    let original = &status.preview;
    let decision = serde_json::from_value::<Decision>(original["_review"].clone()).ok();
    let same_config = decision.as_ref().is_some_and(|d| {
        config_hash(inner, &status.resource).is_ok_and(|hash| hash == d.config_hash)
    });
    let mut preview = json!({});
    for key in ["kind", "summary", "annotationId", "count", "body"]
        .into_iter()
        .filter(|_| same_config)
    {
        if let Some(v) = original.get(key) {
            preview[key] = v.clone();
        }
    }
    if same_config
        && rights.annotations.target
        && let Some(v) = original.get("replacement")
    {
        preview["replacement"] = v.clone();
    }
    if same_config
        && rights.read
        && rights.annotations.target
        && let Some(v) = original.get("source")
    {
        preview["source"] = v.clone();
    }
    let commit_allowed = status.status == "prepared"
        && same_config
        && decision.is_some_and(|d| {
            d.actor_epoch == inner.actor_generation(actor)
                && d.expires_at_unix_ms > now_ms()
                && config_hash(inner, &status.resource).is_ok_and(|hash| hash == d.config_hash)
                && authorize(rights, &d.needs).is_ok()
                && library::authorize_execution(inner, actor, &d.execution).is_ok()
                && d.media
                    .as_ref()
                    .is_none_or(|m| authorize_media(inner, actor, &status.resource, m).is_ok())
        });
    Ok(
        json!({"operationId":status.operation_id,"resource":status.resource,"status":status.status,"planId":status.plan_id.filter(|_|commit_allowed),"preview":preview,"commitAllowed":commit_allowed}),
    )
}
async fn prepare(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(mut req): Json<Prepare>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?; let rights=access(inner,&actor,&req.resource)?;
        if let Some(object)=req.intent.as_object_mut() { for key in ["attribution","author","createdAt"] {object.remove(key);} }
        let kind=text(&req.intent,"kind")?.to_owned(); let mut needs=needs(&kind)?;
        let media_selected=is_media_target(&req.intent["target"]);
        if media_selected && matches!(kind.as_str(),"annotationCreate"|"annotationReattach") {needs.source_read=false;}
        authorize(&rights,&needs)?;
        if media_selected && matches!(kind.as_str(),"annotationCreate"|"annotationReattach") {
            let asset=text(&req.intent["target"],"asset")?;media::input_access(inner,&actor,asset,false)?;
            if !inner.config.resources.get(asset).is_some_and(|r|r.owner==req.resource && r.lifecycle=="live") {return Err(unavailable());}
        }
        if inner.config.files[&req.resource].lifecycle!="live" {return Err(unavailable());}
        let canonical=json!({"resource":req.resource,"basis":req.basis,"intent":req.intent});
        let fingerprint=digest(&serde_json::to_vec(&canonical).map_err(|_|changed())?); let actor_scope=scope(inner,&actor);
        if let Some(old)=inner.core.with_actor(&actor_scope,|core|core.actor_feature_retry(&req.resource,&req.operation_id,&fingerprint)).map_err(core_error)? {return safe_status(inner,&actor,&rights,old);}
        let b=basis(inner,&actor,&req.resource,&req.basis,needs.source_read)?; let doc=document(&b)?;
        let mut changes=vec![]; let mut dependencies=vec![]; let mut witnesses=vec![]; let mut media_decision=None; let mut execution=library::ExecutionAuthority::default(); let mut preview=json!({"kind":kind,"summary":"保存所选批注变更"});
        let annotation_id=req.intent["annotationId"].as_str(); if let Some(id)=annotation_id {preview["annotationId"]=json!(id);}
        let after=match kind.as_str() {
            "annotationCreate" => {
                let purpose=req.intent["purpose"].as_str().unwrap_or("comment"); if !matches!(purpose,"comment"|"mark"|"suggestion") {return Err(ApiError::bad("未知批注目的"));}
                let target=if media_selected {
                    if purpose=="suggestion" {return Err(ApiError::bad("媒体目标可评论或标记；文字替换建议需要准确作者源范围"));}
                    let (target,decision,pool)=media_target(inner,&actor,&b,&req.intent["target"])?;
                    witnesses=pool;media_decision=Some(decision);target
                }else{target(&req.intent["target"],b.node.as_ref().ok_or_else(changed)?)?};
                let replacement=if purpose=="suggestion" { if target.anchor().is_none(){return Err(ApiError::bad("文字建议需要准确原文范围或插入点"));} Some(text(&req.intent,"replacement")?.to_owned()) } else {None};
                let body=text(&req.intent,"body")?.to_owned(); preview["body"]=json!(body); if let Some(r)=&replacement {preview["replacement"]=json!(r);}
                let new=record(&actor,target,body,purpose.into(),replacement); preview["annotationId"]=json!(new.annotation_id); doc.create(new).map_err(knowledge_error)?
            },
            "annotationReply" => {
                let id=text(&req.intent,"annotationId")?; let parent=doc.records.iter().find(|r|r.annotation_id==id).ok_or_else(||ApiError::bad("所选原讨论不存在"))?;
                let body=text(&req.intent,"body")?.to_owned(); preview["body"]=json!(body);
                let mut new=record(&actor,parent.target.clone(),body,"comment".into(),None); new.reply_to=Some(id.into()); doc.reply(new).map_err(knowledge_error)?
            },
            "annotationResolve" => doc.resolve(text(&req.intent,"annotationId")?,req.intent["resolved"].as_bool().ok_or_else(||ApiError::bad("需要明确讨论状态"))?).map_err(knowledge_error)?,
            "annotationReject" => doc.reject(text(&req.intent,"annotationId")?).map_err(knowledge_error)?,
            "annotationReattach" => {
                let id=text(&req.intent,"annotationId")?;
                let target=if media_selected {
                    if doc.records.iter().find(|r|r.annotation_id==id).is_some_and(|r|r.suggestion.is_some()) {return Err(ApiError::bad("已有文字建议不能重新附着到媒体目标"));}
                    let (target,decision,pool)=media_target(inner,&actor,&b,&req.intent["target"])?;
                    witnesses=pool;media_decision=Some(decision);target
                }else{target(&req.intent["target"],b.node.as_ref().ok_or_else(changed)?)?};
                doc.reattach(id,target).map_err(knowledge_error)?
            },
            "annotationReconfirm" => {
                let replacement=text(&req.intent,"replacement")?.to_owned(); preview["replacement"]=json!(replacement);
                doc.reconfirm(text(&req.intent,"annotationId")?,target(&req.intent["target"],b.node.as_ref().unwrap())?,replacement).map_err(knowledge_error)?
            },
            "annotationAccept" => {
                let plan=knowledge::suggestion_plan(&doc,&b.annotations.path,text(&req.intent,"annotationId")?,b.node.as_ref().unwrap()).map_err(knowledge_error)?;
                preview["summary"]=json!("将所选建议的准确正文变更与 accepted 状态一并保存");
                preview["source"]=json!({"before":b.node.as_ref().unwrap().source,"after":plan.source_patch.after});
                changes.push(change(b.source.as_ref().unwrap(),plan.source_patch.after.as_bytes().to_vec()));
                plan.files.into_iter().find(|f|f.path==b.annotations.path).ok_or_else(changed)?.after
            },
            "annotationProofread" => {
                let options:knowledge::ProofreadOptions=serde_json::from_value(req.intent["options"].clone()).map_err(|_|ApiError::bad("校对选项无效"))?;
                let hash=digest(&serde_json::to_vec(&options).map_err(|_|changed())?);
                let scan_id=text(&req.intent,"scanId")?;
                let scan=inner.review.scans.get(scan_id).filter(|s|s.actor==actor&&s.epoch==b.epoch&&s.resource==req.resource&&s.basis==req.basis&&s.options_hash==hash&&s.expires>Instant::now()).cloned().ok_or_else(changed)?;
                editor::validate_inputs(inner,&actor,&req.resource,&scan.inputs)?;
                scan.environment.recheck(inner,&actor)?;
                execution=scan.environment.authority.clone();
                witnesses=scan.environment.plan_witnesses()?;
                for f in &scan.dependencies {inner.core.validate_snapshot(&inner.config.root,&f.path,&f.base_id).map_err(|_|changed())?;if Some(&f.path)!=b.source.as_ref().map(|s|&s.path) { dependencies.push(f.base_id.clone()); }}
                let ids:BTreeSet<String>=serde_json::from_value(req.intent["findings"].clone()).map_err(|_|ApiError::bad("请选择准确原校对结果"))?;
                if ids.is_empty()||ids.len()>200||ids.iter().any(|id|!scan.result.findings.iter().any(|f|&f.id==id)) {return Err(ApiError::bad("每批请选择原检查中的 1 至 200 条结果"));}
                let node=b.node.as_ref().unwrap(); let mut records=vec![];
                for finding in scan.result.findings.iter().filter(|f|ids.contains(&f.id)) {
                    if existing(&doc,finding,node) {continue;}
                    let range=finding.source_range.as_ref().ok_or_else(||ApiError::bad("所选校对结果没有准确作者源位置，保持只读"))?;
                    let target=knowledge::AnnotationTarget::SourceRange {anchor:knowledge::source_anchor(node,range.from,range.to,finding.excerpt.clone()).map_err(knowledge_error)?};
                    let executable=finding.executable&&knowledge::writable_suggestion_range(&node.source,range);
                    // Discussion-only readers receive the rule message, never an
                    // automatically copied target excerpt or replacement in body.
                    let mut r=record(&actor,target,finding.message.clone(),if executable{"suggestion"}else{"comment"}.into(),if executable{finding.replacement.clone()}else{None});
                    r.labels=vec!["proofreading".into(),finding.category.clone()];
                    if req.intent["ignore"]==true {r.review_state="resolved".into();if let Some(s)=&mut r.suggestion {s.state="rejected".into();}}
                    r.unknown.insert("proofreading".into(),json!({"ruleId":finding.rule_id,"ruleVersion":finding.rule_version,"category":finding.category,"ignored":req.intent["ignore"]==true}));records.push(r);
                }
                preview["count"]=json!(records.len());preview["summary"]=json!("批量保存本次已核验的校对批注；已有出现项不重复创建");doc.create_many(records).map_err(knowledge_error)?
            },
            _=>unreachable!(),
        };
        if after.len()>MAX_DOCUMENT_BYTES {return Err(ApiError::bad("批注变更超过本次保存预算"));}
        if needs.source_read&&kind!="annotationAccept" {dependencies.push(b.source.as_ref().unwrap().base_id.clone());}
        changes.push(change(&b.annotations,after));
        preview["_review"]=value(Decision {actor_epoch:b.epoch,expires_at_unix_ms:now_ms().saturating_add(TTL.as_millis() as u64),config_hash:config_hash(inner,&req.resource)?,needs,execution,media:media_decision})?;
        let request=OperationRequest {operation_id:req.operation_id.clone(),root:inner.config.root.to_string_lossy().into_owned(),intent:OperationIntent::UpdateFiles {changes}};
        let status=inner.core.with_actor(&actor_scope,|core|core.actor_feature_prepare_with_witnesses(&req.resource,&req.operation_id,&fingerprint,request,preview,&dependencies,&witnesses.iter().collect::<Vec<_>>())).map_err(core_error)?;
        safe_status(inner,&actor,&rights,status)
    }).await?))
}
async fn operation(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Operation>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;let rights=access(inner,&actor,&req.resource)?;
        let status=inner.core.with_actor(&scope(inner,&actor),|core|core.actor_feature_status(&req.resource,&req.operation_id)).map_err(core_error)?;
        match status {Some(status)=>safe_status(inner,&actor,&rights,status),None=>Ok(json!({"operationId":req.operation_id,"resource":req.resource,"status":"notRegistered","planId":null,"preview":{},"commitAllowed":false}))}
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
            let rights = access(inner, &actor, &req.resource)?;
            let actor_scope = scope(inner, &actor);
            let old = inner
                .core
                .with_actor(&actor_scope, |core| {
                    core.actor_feature_status(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?
                .ok_or_else(changed)?;
            if old.status != "prepared" {
                let status = inner
                    .core
                    .with_actor(&actor_scope, |core| {
                        core.actor_feature_commit(&req.resource, &req.operation_id, &req.plan_id)
                    })
                    .map_err(core_error)?;
                return safe_status(inner, &actor, &rights, status);
            }
            let decision: Decision =
                serde_json::from_value(old.preview["_review"].clone()).map_err(|_| changed())?;
            authorize(&rights, &decision.needs)?;
            library::validate_execution_versions(inner, &actor, &decision.execution)?;
            if let Some(m) = &decision.media {
                authorize_media(inner, &actor, &req.resource, m)?;
            }
            if decision.actor_epoch != inner.actor_generation(&actor)
                || decision.expires_at_unix_ms <= now_ms()
                || decision.config_hash != config_hash(inner, &req.resource)?
                || old.plan_id.as_deref() != Some(&req.plan_id)
            {
                return Err(changed());
            }
            let status = inner
                .core
                .with_actor(&actor_scope, |core| {
                    core.actor_feature_commit(&req.resource, &req.operation_id, &req.plan_id)
                })
                .map_err(core_error)?;
            if status.status == "committed" {
                let released = inner
                    .review
                    .bases
                    .values()
                    .filter(|b| b.resource == req.resource)
                    .flat_map(snapshot_ids)
                    .chain(
                        inner
                            .review
                            .scans
                            .values()
                            .filter(|s| s.resource == req.resource)
                            .flat_map(|s| s.dependencies.iter().map(|f| f.base_id.clone())),
                    )
                    .collect();
                inner.review.bases.retain(|_, b| b.resource != req.resource);
                inner.review.scans.retain(|_, s| s.resource != req.resource);
                release_unretained(inner, released);
            }
            safe_status(inner, &actor, &rights, status)
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
            let rights = access(inner, &actor, &req.resource)?;
            // Fencing one's original request grants no new author effect or target read.
            let status = inner
                .core
                .with_actor(&scope(inner, &actor), |core| {
                    core.actor_feature_settle(&req.resource, &req.operation_id)
                })
                .map_err(core_error)?;
            safe_status(inner, &actor, &rights, status)
        })
        .await?,
    ))
}

#[cfg(all(test, windows))]
mod media_tests {
    use super::*;
    async fn context(state: &HostedState, auth: &Auth) -> Value {
        read(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Read {
                resource: "Shared".into(),
                asset: Some("Diagram".into()),
            }),
        )
        .await
        .unwrap()
        .0
    }
    fn intent(revision: &str) -> Value {
        json!({"kind":"annotationCreate","purpose":"comment","body":"媒体讨论😀","target":{"kind":"image_region","asset":"Diagram","revision":revision,"x":0,"y":0,"rectWidth":1,"rectHeight":1}})
    }
    async fn propose(state: &HostedState, auth: &Auth, basis: &str, intent: Value) -> Value {
        prepare(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Prepare {
                resource: "Shared".into(),
                basis: basis.into(),
                operation_id: uuid::Uuid::new_v4().to_string(),
                intent,
            }),
        )
        .await
        .unwrap()
        .0
    }
    #[tokio::test]
    async fn exact_media_annotation_needs_its_own_read_but_no_body_access() {
        let (_temp, state, auth, revision) = media::tests::fixture();
        let before = context(&state, &auth).await;
        assert!(before.get("sourceHash").is_none());
        assert_eq!(before["media"]["revision"], revision);
        let original = {
            let inner = state.inner.lock().unwrap();
            std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap()
        };
        let plan = propose(
            &state,
            &auth,
            before["basis"].as_str().unwrap(),
            intent(&revision),
        )
        .await;
        assert_eq!(plan["status"], "prepared");
        assert!(plan["preview"].get("source").is_none());
        let done = commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                resource: "Shared".into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(done["status"], "committed");
        let after = context(&state, &auth).await;
        let record = &after["annotations"][0];
        assert_eq!(record["body"], "媒体讨论😀");
        assert_eq!(record["targetResolution"], "exact");
        assert!(record.get("visibleExcerpt").is_none());
        assert!(record.get("sourceLocation").is_none());
        assert_eq!(record["resourceLocation"]["target"]["kind"], "imageRegion");
        assert_eq!(record["resourceLocation"]["target"]["revisionId"], revision);
        let inner = state.inner.lock().unwrap();
        assert_eq!(
            std::fs::read(inner.config.root.join("Shared/Shared.typ")).unwrap(),
            original
        );
    }
    #[tokio::test]
    async fn media_target_wrong_reference_and_fake_geometry_do_not_register_operations() {
        let (_temp, state, auth, revision) = media::tests::fixture();
        let before = context(&state, &auth).await;
        for variant in [0, 1] {
            let mut request = intent(&revision);
            if variant == 0 {
                request["target"]["resource"] = json!(format!(
                    "weftext-resource://{}/{}",
                    uuid::Uuid::new_v4(),
                    uuid::Uuid::new_v4()
                ));
            } else {
                request["target"]["width"] = 999.into();
            }
            let id = uuid::Uuid::new_v4().to_string();
            assert!(
                prepare(
                    State(state.clone()),
                    Extension(auth.clone()),
                    Json(Prepare {
                        resource: "Shared".into(),
                        basis: before["basis"].as_str().unwrap().into(),
                        operation_id: id.clone(),
                        intent: request
                    })
                )
                .await
                .is_err()
            );
            assert_eq!(
                operation(
                    State(state.clone()),
                    Extension(auth.clone()),
                    Json(Operation {
                        resource: "Shared".into(),
                        operation_id: id
                    })
                )
                .await
                .unwrap()
                .0["status"],
                "notRegistered"
            );
        }
        let sidecar = state
            .inner
            .lock()
            .unwrap()
            .config
            .root
            .join("Shared/weftext.annotations.json");
        assert!(!sidecar.exists());
    }
    #[tokio::test]
    async fn ordinary_discussion_reply_does_not_borrow_the_medias_revoked_read() {
        let (_temp, state, auth, revision) = media::tests::fixture();
        let before = context(&state, &auth).await;
        let plan = propose(
            &state,
            &auth,
            before["basis"].as_str().unwrap(),
            intent(&revision),
        )
        .await;
        let _ = commit(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Commit {
                resource: "Shared".into(),
                operation_id: plan["operationId"].as_str().unwrap().into(),
                plan_id: plan["planId"].as_str().unwrap().into(),
            }),
        )
        .await
        .unwrap();
        {
            let mut inner = state.inner.lock().unwrap();
            let user = inner.policy.users.get_mut("alice").unwrap();
            user.resources.clear();
            user.epoch += 1;
        }
        let discussion = read(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Read {
                resource: "Shared".into(),
                asset: None,
            }),
        )
        .await
        .unwrap()
        .0;
        let record = &discussion["annotations"][0];
        assert_eq!(record["body"], "媒体讨论😀");
        assert_eq!(record["targetResolution"], "unavailable");
        assert!(record.get("resourceLocation").is_none());
        let reply=propose(&state,&auth,discussion["basis"].as_str().unwrap(),json!({"kind":"annotationReply","annotationId":record["annotationId"],"body":"独立讨论回复"})).await;
        assert_eq!(reply["status"], "prepared");
        assert_eq!(reply["commitAllowed"], true);
    }
}
