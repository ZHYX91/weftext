//! Authorized snapshot-only links and mention discovery. Proposals are Draft
//! patches; the existing actor save remains the sole persistence entry point.
use super::*;
use std::path::Path;
use weftext_core::{ConfigRole, QualifiedNode, QualifiedResource, SnapshotWitness};
use weftext_data::Schema;
use weftext_knowledge as knowledge;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
const MAX_COMPILES: usize = 8;
const MAX_EDGES: usize = 4096;
const MAX_OUTPUT: usize = 8 * 1024 * 1024;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Snapshot {
    session_id: String,
    source: String,
    source_hash: String,
    serial: u64,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Occurrence {
    from: usize,
    to: usize,
    target: String,
}
#[derive(Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Intent {
    Link {
        occurrence: Occurrence,
        form: knowledge::LinkForm,
    },
    Mention {
        fragment_id: String,
        start_scalar: usize,
        end_scalar: usize,
        target: knowledge::NodeRef,
        #[serde(default)]
        fragment: Option<String>,
    },
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Convert {
    basis: Snapshot,
    snapshot: String,
    intent: Intent,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Copy {
    basis: Snapshot,
    snapshot: String,
    occurrence: Occurrence,
    /// original copies the exact authored call/address without resolving it.
    form: String,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/knowledge-links/read", post(read))
        .route("/api/hosted/knowledge-links/convert", post(convert))
        .route("/api/hosted/knowledge-links/copy", post(copy))
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "knowledgeLinksChanged",
        "链接、词典或当前草稿依据已变化；请保留输入并重新读取。",
    )
}
fn knowledge_error(error: knowledge::KnowledgeError) -> ApiError {
    let message = match error.code.as_str() {
        "sourceEntry" | "unmapped" => "此出现项没有唯一可写的原生参数；请保留源码并使用源码入口。",
        "staleDictionary" | "conflict" => "当前词典或准确文字已变化；请重新读取，草稿保留。",
        _ => "此目标或片段不能按当前获权依据准确转换；原链接和草稿保留。",
    };
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "knowledgeLinksUnavailable",
        message,
    )
}
fn file_form() -> ApiError {
    ApiError::new(
        StatusCode::UNPROCESSABLE_ENTITY,
        "knowledgeLinksFileAddress",
        "托管地址转换不交付服务器绝对路径；可选择稳定 ID、源相对或库内路径，已有原文仍可复制。",
    )
}
fn uri(node: &knowledge::NodeRef) -> String {
    format!("weftext-node://{}/{}", node.workspace_id, node.node_id)
}

/// Filter before touching any source. A configured read-only managed note is
/// a candidate; editable controls writing and never confers target authority.
fn authorized_files(
    inner: &HostedInner,
    actor: &str,
) -> Result<BTreeMap<String, FileEntry>, ApiError> {
    Ok(inner
        .files_view(actor)?
        .iter()
        .filter(|(id, f)| f.lifecycle == "live" && inner.grant(actor, id).is_ok_and(|r| r.read))
        .map(|(id, f)| (id.clone(), f.clone()))
        .collect())
}
fn binding(inner: &HostedInner, actor: &str) -> Result<String, ApiError> {
    let user = &inner.policy.users[actor];
    let resources = inner
        .config
        .resources
        .iter()
        .filter(|(id, r)| r.lifecycle == "live" && user.resources.get(*id).is_some_and(|r| r.read))
        .collect::<BTreeMap<_, _>>();
    let collections = inner
        .config
        .resource_collections
        .iter()
        .filter(|(id, c)| {
            c.lifecycle == "live" && user.resource_collections.get(*id).is_some_and(|r| r.read)
        })
        .collect::<BTreeMap<_, _>>();
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"files":authorized_files(inner,actor)?,"resources":resources,"collections":collections})).map_err(|_| changed())?))
}
fn qualify(
    inner: &mut HostedInner,
    auth: &Auth,
    req: &Snapshot,
    observe: bool,
    write: bool,
) -> Result<(String, HostedSession, Rights), ApiError> {
    let (actor, session, rights) = inner.session(auth, &req.session_id)?;
    if write && (!rights.write || !inner.file(&session.resource)?.editable) {
        return Err(unavailable());
    }
    writing_api::validate_source(&req.source, Some(&req.source_hash))?;
    if observe {
        writing_api::observe_version(
            &mut inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    } else {
        writing_api::exact_version(
            &inner.editor_versions,
            &req.session_id,
            &req.source,
            req.serial,
        )?;
    }
    Ok((actor, session, rights))
}

struct Candidate {
    resource: String,
    file: FileEntry,
    qualified: QualifiedNode,
}
struct Asset {
    id: String,
    config: media::Resource,
    qualified: QualifiedResource,
}
struct Compilation {
    index: usize,
    worker: Option<WorkerRequest>,
    dependencies: BTreeMap<String, String>,
    environment: library::Environment,
    output: Option<Value>,
}
struct FieldSemantics {
    aliases: Vec<String>,
    references: Vec<weftext_data::TypedRefSlot>,
    complete: bool,
    fingerprint: Value,
}
/// Qualify and narrow definitions before deriving any word or typed reference.
/// This follows the same selected-field producer as hosted fields/read. Neither
/// complete source permission nor Schema role permission grants other fields.
fn field_semantics(
    source: &str,
    schema: Option<&Schema>,
    granted: Option<&BTreeSet<String>>,
) -> Result<FieldSemantics, ApiError> {
    let mut result = FieldSemantics {
        aliases: vec![],
        references: vec![],
        complete: true,
        fingerprint: json!([]),
    };
    let Some(granted) = granted.filter(|fields| !fields.is_empty()) else {
        return Ok(result);
    };
    let Some(schema) = schema else {
        result.complete = false;
        result.fingerprint = json!({"fields":granted,"status":"unavailable"});
        return Ok(result);
    };
    let mut fields = vec![];
    for id in granted {
        match weftext_data::narrow_profile(source, schema, &BTreeSet::from([id.clone()]), None) {
            Ok(profile) => {
                if let Some(field) = profile.fields.into_iter().next() {
                    if matches!(
                        field.cell,
                        weftext_data::Cell::Invalid { .. } | weftext_data::Cell::Unavailable { .. }
                    ) {
                        result.complete = false;
                    } else {
                        fields.push(field);
                    }
                } else {
                    result.complete = false;
                }
            }
            Err(_) => result.complete = false,
        }
    }
    result.fingerprint = json!({"fields":fields,"complete":result.complete});
    let accepted = fields
        .iter()
        .map(|field| field.field_id.clone())
        .collect::<BTreeSet<_>>();
    if accepted.is_empty() {
        return Ok(result);
    }
    let mut scoped = schema.clone();
    scoped.fields.retain(|field| accepted.contains(&field.id));
    for field in &mut scoped.fields {
        field.shape = fields
            .iter()
            .find(|p| p.field_id == field.id)
            .unwrap()
            .shape
            .clone();
        field.default = None;
    }
    scoped.definitions.clear();
    scoped.relations.clear();
    scoped.providers.clear();
    scoped.validators.clear();
    scoped.series_policies.clear();
    // Type spelling/control is needed privately to qualify the granted field;
    // no type IDs or alias registry are part of this consumer's response.
    for ty in &mut scoped.types {
        ty.fields.retain(|field| accepted.contains(&field.id));
        ty.constraints.clear();
    }
    let mut names = scoped.clone();
    names
        .fields
        .retain(|field| matches!(field.id.as_str(), "core/aliases" | "contacts/names"));
    if !names.fields.is_empty() {
        let names_ids = names
            .fields
            .iter()
            .map(|field| field.id.clone())
            .collect::<BTreeSet<_>>();
        for ty in &mut names.types {
            ty.fields.retain(|field| names_ids.contains(&field.id));
        }
        let terms = weftext_data::search_terms(source, &names);
        result.complete &= terms.complete;
        result.aliases = terms.terms;
    }
    match weftext_data::typed_ref_closure(source, &scoped) {
        Ok(closure) => {
            // Ungranted authored members are outside the declared field scope,
            // not evidence that its qualified selected fields were incomplete.
            result.complete &= closure.reasons.iter().all(|reason| {
                reason == "unbound root fields are outside the installed typed closure"
            });
            result.references = closure
                .slots
                .into_iter()
                .filter(|slot| {
                    accepted.contains(&slot.field_id)
                        && matches!(slot.kind.as_str(), "node_ref" | "resource_ref" | "tag_ref")
                })
                .take(MAX_EDGES + 1)
                .collect();
            if result.references.len() > MAX_EDGES {
                result.references.truncate(MAX_EDGES);
                result.complete = false;
            }
        }
        Err(_) => result.complete = false,
    }
    result.fingerprint["complete"] = result.complete.into();
    Ok(result)
}
struct Captured {
    actor: String,
    epoch: u64,
    deployment: String,
    resource: String,
    request: Snapshot,
    config_hash: String,
    schema_witness: Option<(String, SnapshotWitness)>,
    semantic_fields: BTreeMap<String, BTreeSet<String>>,
    candidates: Vec<Candidate>,
    assets: Vec<Asset>,
    collections: Vec<weftext_core::QualifiedResourceCollection>,
    inventory: knowledge::InventorySnapshot,
    resource_fragments: Vec<knowledge::ResourceFragmentQualification>,
    properties: Vec<knowledge::PropertyReferenceInput>,
    property_complete: bool,
    dictionary_complete: bool,
    selected: usize,
    compilations: Vec<Compilation>,
    bytes: usize,
}
impl Captured {
    fn source(&self) -> &knowledge::NodeSnapshot {
        &self.inventory.nodes[self.selected]
    }
    fn basis(&self) -> Value {
        json!({"actor":self.actor,"actorEpoch":self.epoch,"deployment":self.deployment,"resource":self.resource,"sessionId":self.request.session_id,"sourceHash":self.request.source_hash,"serial":self.request.serial,"snapshot":self.inventory.generation})
    }
    fn add_compile(&mut self, inner: &mut HostedInner, index: usize) {
        if self.compilations.iter().any(|c| c.index == index)
            || self.compilations.len() >= MAX_COMPILES
        {
            return;
        }
        let resource = &self.candidates[index].resource;
        let source = &self.inventory.nodes[index].source;
        if let Ok((worker, dependencies, environment)) =
            editor::freeze(inner, &self.actor, resource, source)
        {
            let cost = serde_json::to_vec(&worker)
                .map_or(MAX_BYTES, |b| b.len())
                .saturating_add(environment.retained_bytes());
            if self.bytes.saturating_add(cost) <= MAX_BYTES {
                self.bytes += cost;
                self.compilations.push(Compilation {
                    index,
                    worker: Some(worker),
                    dependencies,
                    environment,
                    output: None,
                });
            }
        }
    }
}

fn capture(
    inner: &mut HostedInner,
    auth: &Auth,
    req: Snapshot,
    observe: bool,
    write: bool,
    _discovery: bool,
) -> Result<Captured, ApiError> {
    let (actor, session, _) = qualify(inner, auth, &req, observe, write)?;
    let root = inner.config.root.clone();
    let selected_file = inner.file(&session.resource)?.clone();
    let scope = inner.actor_scope(&actor);
    let selected = inner
        .core
        .with_actor_for_root(&scope, &inner.config.root, |c| {
            c.qualified_node_for_session_at(&root, &root.join(&selected_file.path), &req.session_id)
        })
        .map_err(|_| unavailable())?;
    if weftext_engine::inspect_metadata(&req.source).id.as_deref()
        != Some(selected.owner.node_id.as_str())
    {
        return Err(changed());
    }
    let mut bytes = req
        .source
        .len()
        .saturating_add(selected.witness.retained_bytes());
    let configured = authorized_files(inner, &actor)?;
    let mut candidates = vec![Candidate {
        resource: session.resource.clone(),
        file: selected_file,
        qualified: selected,
    }];
    let mut complete = true;
    let mut source_bytes = req.source.len();
    for (id, file) in configured.iter().filter(|(id, _)| *id != &session.resource) {
        // Source permission precedes both structural qualification and reading.
        if inner.rights(&actor, id).is_err() {
            complete = false;
            continue;
        }
        match inner.core.qualified_node_at(&root, &root.join(&file.path)) {
            Ok(q) if file.expected_node.as_ref().is_none_or(|n| n == &q.owner) => {
                let size = q.witness.retained_bytes();
                if source_bytes.saturating_add(size) > MAX_SOURCE_BYTES
                    || bytes.saturating_add(size.saturating_mul(2)) > MAX_BYTES
                {
                    complete = false;
                    break;
                }
                source_bytes += size;
                bytes += size;
                candidates.push(Candidate {
                    resource: id.clone(),
                    file: file.clone(),
                    qualified: q,
                });
            }
            _ => complete = false,
        }
    }
    if candidates[0]
        .file
        .expected_node
        .as_ref()
        .is_some_and(|n| n != &candidates[0].qualified.owner)
    {
        return Err(unavailable());
    }
    // Multiple configured aliases of one admitted object are ambiguous, rather
    // than two separate notes or a namesake from which to pick a locator.
    let mut owners = BTreeMap::<knowledge::NodeRef, usize>::new();
    for candidate in &candidates {
        *owners.entry(candidate.qualified.owner.clone()).or_default() += 1;
    }
    if owners[&candidates[0].qualified.owner] != 1 {
        return Err(unavailable());
    }
    if owners.values().any(|n| *n > 1) {
        complete = false;
        candidates.retain(|c| owners[&c.qualified.owner] == 1);
    }
    // Source bytes expose authored values, not the private Schema meanings of
    // those values. Only separately granted fields supply a semantic producer.
    // A Schema role read grant alone does not enlarge this field authority.
    let semantic_fields = candidates
        .iter()
        .filter_map(|c| {
            let fields = inner
                .grant(&actor, &c.resource)
                .ok()?
                .fields
                .iter()
                .filter(|(_, r)| r.read)
                .map(|(id, _)| id.clone())
                .collect::<BTreeSet<_>>();
            (!fields.is_empty()).then(|| (c.resource.clone(), fields))
        })
        .collect::<BTreeMap<_, _>>();
    let mut schema_witness = None;
    let mut schema = None;
    if !semantic_fields.is_empty()
        && let Ok(document) = inner.core.read_config(&root, ConfigRole::Schema)
    {
        let witness = inner
            .core
            .take_snapshot_witness(&document.base_id)
            .map_err(|_| changed())?;
        bytes = bytes.saturating_add(witness.retained_bytes());
        schema = document
            .bytes
            .as_deref()
            .map(|bytes| {
                std::str::from_utf8(bytes)
                    .ok()
                    .and_then(|text| Schema::from_author_json(text).ok())
            })
            .unwrap_or_else(|| Schema::bundled().ok());
        schema_witness = Some((document.path, witness));
    }
    let paths = candidates
        .iter()
        .map(|c| c.file.path.clone())
        .collect::<Vec<_>>();
    let mut frozen = BTreeMap::new();
    for chunk in paths.chunks(64) {
        frozen.extend(
            inner
                .core
                .hosted_files(&root, chunk)
                .map_err(|_| changed())?,
        );
    }
    let mut nodes = vec![];
    let mut dictionary_complete = complete;
    let mut properties = vec![];
    let mut property_complete = complete;
    let mut semantic_keys = BTreeMap::new();
    for candidate in &candidates {
        let actual = frozen.get(&candidate.file.path).ok_or_else(changed)?;
        if digest(actual) != candidate.qualified.source_hash {
            return Err(changed());
        }
        let source = if candidate.resource == session.resource {
            req.source.clone()
        } else {
            String::from_utf8(actual.clone()).map_err(|_| changed())?
        };
        let title = weftext_engine::inspect_editor(&source).title;
        let info = weftext_engine::inspect_metadata(&source);
        if title.status != "writable" && info.writer_status != "unique-literal-profile" {
            dictionary_complete = false;
        }
        let semantic = field_semantics(
            &source,
            schema.as_ref(),
            semantic_fields.get(&candidate.resource),
        )?;
        dictionary_complete &= semantic.complete;
        property_complete &= semantic.complete;
        if semantic_fields.contains_key(&candidate.resource) {
            semantic_keys.insert(candidate.resource.clone(), semantic.fingerprint);
        }
        for slot in semantic.references {
            properties.push(knowledge::PropertyReferenceInput {
                source: candidate.qualified.owner.clone(),
                field_id: slot.field_id,
                item_path: slot
                    .path
                    .into_iter()
                    .map(|p| serde_json::to_string(&p).unwrap_or_default())
                    .collect(),
                target: slot.target,
            });
        }
        bytes = bytes.saturating_add(source.len());
        nodes.push(knowledge::NodeSnapshot {
            owner: candidate.qualified.owner.clone(),
            path: url::Url::from_file_path(root.join(&candidate.file.path))
                .map_err(|_| changed())?
                .to_string(),
            library_path: candidate.file.path.replace('\\', "/"),
            revision_id: candidate.qualified.revision.clone().unwrap_or_default(),
            source,
            title: title.text,
            aliases: semantic.aliases,
            labels: vec![],
            labels_complete: false,
        });
    }
    let mut assets = vec![];
    let mut collections = vec![];
    let mut selections = inner
        .config
        .resources
        .iter()
        .filter(|(id, r)| {
            r.lifecycle == "live" && media::input_access(inner, &actor, id, false).is_ok()
        })
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();
    let collection_ids = inner
        .config
        .resource_collections
        .keys()
        .filter(|id| resource_collections::input_access(inner, &actor, id, false).is_ok())
        .cloned()
        .collect::<Vec<_>>();
    for id in collection_ids {
        match resource_collections::capture(inner, &actor, &id) {
            Ok(c) if bytes.saturating_add(c.retained_bytes()) <= MAX_BYTES => {
                bytes += c.retained_bytes();
                if c.unrecognized_entries > 0 {
                    complete = false;
                }
                for entry in &c.entries {
                    if let Some(member) = entry.resource_ref.rsplit('/').next() {
                        selections.insert(format!("{id}/{member}"));
                    }
                }
                collections.push(c);
            }
            _ => complete = false,
        }
    }
    let mut resources = vec![];
    for (i, id) in selections.into_iter().enumerate() {
        if i >= 64 {
            complete = false;
            break;
        }
        match media::capture_input(inner, &actor, &id) {
            Ok(q) if bytes.saturating_add(q.witness.retained_bytes()) <= MAX_BYTES => {
                let config = media::resolve_asset(inner, &actor, &id)?;
                bytes += q.witness.retained_bytes();
                let snapshot = knowledge::ResourceSnapshot {
                    key: q.resource_ref.clone(),
                    path: url::Url::from_file_path(root.join(&config.path))
                        .map_err(|_| changed())?
                        .to_string(),
                    revision_id: q.revision.clone().unwrap_or_default(),
                };
                if let Some(previous) = assets
                    .iter()
                    .find(|a: &&Asset| a.qualified.resource_ref == q.resource_ref)
                {
                    if previous.qualified.owner != q.owner
                        || previous.qualified.content_hash != q.content_hash
                        || previous.qualified.revision != q.revision
                    {
                        return Err(changed());
                    }
                } else {
                    resources.push(snapshot);
                }
                assets.push(Asset {
                    id,
                    config,
                    qualified: q,
                });
            }
            _ => complete = false,
        }
    }
    if bytes > MAX_BYTES {
        return Err(ApiError::bad("本次链接与词典准确依据超过 32 MiB 预算"));
    }
    let generation = digest(&serde_json::to_vec(&json!({"binding":binding(inner,&actor)?,"actor":actor,"epoch":inner.actor_generation(&actor),"source":req.source_hash,"serial":req.serial,"semanticFields":semantic_keys,"nodes":candidates.iter().map(|c|json!({"resource":c.resource,"owner":c.qualified.owner,"hash":c.qualified.source_hash,"revision":c.qualified.revision})).collect::<Vec<_>>(),"resources":assets.iter().map(|a|json!({"asset":a.id,"hash":a.qualified.content_hash,"revision":a.qualified.revision})).collect::<Vec<_>>(),"collections":collections.iter().map(|c|json!({"owner":c.owner,"revision":c.revision})).collect::<Vec<_>>(),"complete":complete})).map_err(|_| changed())?);
    let mut captured = Captured {
        actor: actor.clone(),
        epoch: inner.actor_generation(&actor),
        deployment: inner.config.deployment.clone(),
        resource: session.resource,
        request: req,
        config_hash: binding(inner, &actor)?,
        schema_witness,
        semantic_fields,
        candidates,
        assets,
        collections,
        inventory: knowledge::InventorySnapshot {
            generation,
            coverage: if complete {
                knowledge::Coverage::Complete
            } else {
                knowledge::Coverage::Partial
            },
            nodes,
            resources,
        },
        resource_fragments: vec![],
        properties,
        property_complete: property_complete && complete,
        dictionary_complete,
        selected: 0,
        compilations: vec![],
        bytes,
    };
    // Preparing the same bounded compile set in read and proposal captures
    // keeps the displayed generation tied to actual style/input bytes. A new
    // dependency cannot silently reinterpret an old displayed conversion.
    captured.add_compile(inner, 0);
    let relevant = relevant_labels(&captured);
    for index in 1..captured.inventory.nodes.len() {
        if index < MAX_COMPILES || relevant.contains(&captured.inventory.nodes[index].owner) {
            captured.add_compile(inner, index);
        }
    }
    let environment_keys = captured.compilations.iter().map(|c| {
        Ok(json!({"resource":captured.candidates[c.index].resource,"worker":digest(&serde_json::to_vec(&c.worker).map_err(|_|changed())?)}))
    }).collect::<Result<Vec<_>, ApiError>>()?;
    captured.inventory.generation = digest(
        &serde_json::to_vec(
            &json!({"sourceCut":captured.inventory.generation,"compileInputs":environment_keys}),
        )
        .map_err(|_| changed())?,
    );
    captured.resource_fragments = resource_fragments(&captured);
    Ok(captured)
}

/// Resource fragments use metadata derived from these exact qualified bytes;
/// a page-looking URI alone cannot certify a PDF page or acquire a writer.
fn resource_fragments(captured: &Captured) -> Vec<knowledge::ResourceFragmentQualification> {
    let mut seen = BTreeSet::new();
    let mut metadata = BTreeMap::new();
    let mut out = vec![];
    let mut remaining = MAX_EDGES;
    for node in &captured.inventory.nodes {
        let scan = knowledge::native_links_bounded(&node.source, remaining, 1024 * 1024);
        remaining = remaining.saturating_sub(scan.links.len());
        for link in scan.links {
            let resolution = knowledge::resolve_link(&link.target, node, &captured.inventory);
            if let (Some(resource), Some(fragment)) = (resolution.resource, resolution.fragment) {
                if !seen.insert((resource.clone(), fragment.clone())) {
                    continue;
                }
                let Some(asset) = captured
                    .assets
                    .iter()
                    .find(|a| a.qualified.resource_ref == resource)
                else {
                    continue;
                };
                let Some(revision) = asset.qualified.revision.as_ref() else {
                    continue;
                };
                let actual = metadata.entry(resource.clone()).or_insert_with(|| {
                    crate::media_api::metadata_from_bytes(asset.qualified.bytes())
                });
                let qualification =
                    crate::knowledge_api::resource_fragment_metadata(actual, Some(&fragment));
                let status = match qualification["status"].as_str() {
                    Some("found") => knowledge::ResourceFragmentStatus::Found,
                    Some("missing") => knowledge::ResourceFragmentStatus::Missing,
                    _ => knowledge::ResourceFragmentStatus::Unavailable,
                };
                out.push(knowledge::ResourceFragmentQualification {
                    resource,
                    revision_id: revision.clone(),
                    fragment,
                    status,
                });
            }
        }
        if !scan.complete {
            break;
        }
    }
    out
}

fn recheck(
    inner: &mut HostedInner,
    auth: &Auth,
    captured: &Captured,
    write: bool,
) -> Result<(), ApiError> {
    let (actor, session, _) = qualify(inner, auth, &captured.request, false, write)?;
    if actor != captured.actor
        || inner.actor_generation(&actor) != captured.epoch
        || session.resource != captured.resource
    {
        return Err(expired());
    }
    if binding(inner, &actor)? != captured.config_hash {
        return Err(changed());
    }
    let root = inner.config.root.clone();
    for (resource, fields) in &captured.semantic_fields {
        let rights = inner.grant(&actor, resource)?;
        if fields
            .iter()
            .any(|field| !rights.fields.get(field).is_some_and(|r| r.read))
        {
            return Err(unavailable());
        }
    }
    if let Some((path, witness)) = &captured.schema_witness {
        inner
            .core
            .validate_snapshot_witness(&root, Path::new(path), witness)
            .map_err(|_| changed())?;
    }
    for candidate in &captured.candidates {
        inner.rights(&actor, &candidate.resource)?;
        let path = root.join(&candidate.file.path);
        let current = if candidate.resource == captured.resource {
            let scope = inner.actor_scope(&actor);
            inner
                .core
                .with_actor_for_root(&scope, &inner.config.root, |c| {
                    c.qualified_node_for_session_at(&root, &path, &captured.request.session_id)
                })
        } else {
            inner.core.qualified_node_at(&root, &path)
        }
        .map_err(|_| changed())?;
        if current.owner != candidate.qualified.owner {
            return Err(changed());
        }
        inner
            .core
            .validate_snapshot_witness(&root, &path, &candidate.qualified.witness)
            .map_err(|_| changed())?;
    }
    for asset in &captured.assets {
        media::validate_original(inner, &actor, &asset.id, &asset.qualified)?;
        let current = media::resolve_asset(inner, &actor, &asset.id)?;
        if serde_json::to_value(&current).map_err(|_| changed())?
            != serde_json::to_value(&asset.config).map_err(|_| changed())?
        {
            return Err(changed());
        }
    }
    for collection in &captured.collections {
        inner
            .core
            .validate_resource_collection_witness(&root, &collection.witness)
            .map_err(|_| changed())?;
    }
    for compilation in &captured.compilations {
        editor::validate_inputs(
            inner,
            &actor,
            &captured.candidates[compilation.index].resource,
            &compilation.dependencies,
        )?;
        compilation.environment.recheck(inner, &actor)?;
    }
    Ok(())
}
async fn compile(state: &HostedState, captured: &mut Captured) {
    for compilation in &mut captured.compilations {
        if let Some(worker) = compilation.worker.take() {
            compilation.output = editor::compile_frozen(state, worker).await.ok();
        }
        let node = &mut captured.inventory.nodes[compilation.index];
        if let Some(output) = compilation.output.as_ref().filter(|o| o["ok"] == true)
            && let Ok(labels) = serde_json::from_value::<Vec<weftext_engine::CompiledLabel>>(
                output["labels"].clone(),
            )
            && labels.len() <= MAX_EDGES
            && labels.iter().all(|l| l.label.len() <= 1024)
        {
            node.labels = labels
                .into_iter()
                .flat_map(|l| std::iter::repeat_n(l.label, l.count.min(2)))
                .collect();
            node.labels_complete = true;
        }
    }
}
fn relevant_labels(captured: &Captured) -> BTreeSet<knowledge::NodeRef> {
    let mut relevant = BTreeSet::new();
    let mut remaining = MAX_EDGES;
    for source in &captured.inventory.nodes {
        let scan = knowledge::native_links_bounded(&source.source, remaining, 1024 * 1024);
        remaining = remaining.saturating_sub(scan.links.len());
        for link in scan.links {
            let resolution = knowledge::resolve_link(&link.target, source, &captured.inventory);
            if resolution.fragment.is_some()
                && let Some(target) = resolution.target
            {
                relevant.insert(target);
            }
        }
        if !scan.complete {
            break;
        }
    }
    relevant
}
fn mention_scan(captured: &Captured, writable: bool) -> Result<knowledge::MentionScan, ApiError> {
    let tree = captured
        .compilations
        .iter()
        .find(|c| c.index == captured.selected)
        .and_then(|c| c.output.as_ref())
        .filter(|o| o["ok"] == true)
        .and_then(|o| serde_json::from_value::<weftext_engine::RenderNode>(o["tree"].clone()).ok());
    let fragments = tree
        .as_ref()
        .map(knowledge::visible_fragments)
        .unwrap_or_default();
    let mut inventory = captured.inventory.clone();
    if !captured.dictionary_complete || tree.is_none() {
        inventory.coverage = knowledge::Coverage::Partial;
    }
    let candidates = captured
        .compilations
        .iter()
        .flat_map(|c| {
            let output = c.output.as_ref();
            let headings: Vec<weftext_engine::HeadingInfo> = output
                .and_then(|o| serde_json::from_value(o["sourceInfo"]["headings"].clone()).ok())
                .unwrap_or_default();
            let labels: Vec<weftext_engine::CompiledLabel> = output
                .and_then(|o| serde_json::from_value(o["labels"].clone()).ok())
                .unwrap_or_default();
            knowledge::compiled_mention_candidates(&inventory.nodes[c.index], &headings, &labels)
        })
        .collect::<Vec<_>>();
    if inventory.nodes.iter().any(|n| !n.labels_complete) {
        inventory.coverage = knowledge::Coverage::Partial;
    }
    let mut scan = knowledge::scan_mentions_with_candidates(
        &inventory,
        &captured.source().owner,
        &fragments,
        &candidates,
        500_000,
        MAX_EDGES,
    )
    .map_err(knowledge_error)?;
    if !writable {
        for fragment in &mut scan.fragments {
            for hit in &mut fragment.hits {
                hit.writable = false;
            }
        }
    }
    Ok(scan)
}
fn public_node(captured: &Captured, node: &knowledge::NodeRef) -> Option<Value> {
    let candidate = captured
        .candidates
        .iter()
        .find(|c| &c.qualified.owner == node)?;
    Some(
        json!({"resource":candidate.resource,"title":candidate.file.title,"nodeRef":node,"reference":uri(node),"canOpen":true}),
    )
}
fn public_resolution(
    captured: &Captured,
    resolution: &knowledge::LinkResolution,
    original: &str,
) -> Value {
    let target = resolution
        .target
        .as_ref()
        .and_then(|n| public_node(captured, n));
    let asset = resolution.resource.as_ref().and_then(|r| {
        captured
            .assets
            .iter()
            .find(|a| &a.qualified.resource_ref == r)
    });
    let reference = target
        .as_ref()
        .and_then(|t| t["reference"].as_str())
        .map(str::to_owned)
        .or_else(|| asset.map(|a| a.qualified.resource_ref.clone()))
        .map(|mut r| {
            if let Some((_, suffix)) = original.split_once('#') {
                r.push('#');
                r.push_str(suffix);
            }
            r
        });
    json!({"status":resolution.status,"form":resolution.form,"fragment":resolution.fragment,"target":target,"asset":asset.map(|a|json!({"asset":a.id,"title":a.config.title,"reference":a.qualified.resource_ref})),"reference":reference,"canOpen":resolution.status=="resolved" && (target.as_ref().is_some_and(|t|t["canOpen"]==true)||asset.is_some())})
}
fn public_edge(captured: &Captured, edge: knowledge::LinkEdge) -> Value {
    let source = captured
        .inventory
        .nodes
        .iter()
        .find(|n| n.owner == edge.source)
        .unwrap();
    let raw = authored_literal(&source.source, &edge.occurrence).unwrap_or_default();
    let resolution = knowledge::resolve_link_with_resource_fragments(
        &edge.occurrence.target,
        source,
        &captured.inventory,
        &captured.resource_fragments,
    );
    json!({"source":public_node(captured,&edge.source),"occurrence":edge.occurrence,"raw":raw,"sourceHash":digest(source.source.as_bytes()),"resolution":public_resolution(captured,&resolution,&edge.occurrence.target),"editable":edge.source==captured.source().owner && edge.occurrence.destination_slot.is_some()})
}
fn authored_literal(source: &str, link: &knowledge::NativeLink) -> Option<String> {
    let start = if matches!(link.kind.as_str(), "native" | "nativeLabel")
        && link.from > 0
        && source.as_bytes().get(link.from - 1) == Some(&b'#')
    {
        link.from - 1
    } else {
        link.from
    };
    source.get(start..link.to).map(str::to_owned)
}
fn public_rendered(captured: &Captured, link: knowledge::RenderedLink) -> Value {
    let resolved = knowledge::resolve_link_with_resource_fragments(
        &link.destination,
        captured.source(),
        &captured.inventory,
        &captured.resource_fragments,
    );
    let resolution = public_resolution(captured, &resolved, &link.destination);
    // Original authored bytes are preserved by the separate source projection.
    // Generated absolute destinations cannot reveal physical server locators.
    let absolute = Path::new(&link.destination).is_absolute()
        || url::Url::parse(&link.destination).is_ok_and(|u| u.scheme() == "file");
    let destination = if absolute {
        resolution["reference"]
            .as_str()
            .unwrap_or("服务器绝对地址不可交付")
            .to_owned()
    } else {
        link.destination
    };
    json!({"compileIdentity":link.compile_identity,"target":link.target,"projectionGeneration":link.projection_generation,"occurrenceOrdinal":link.occurrence_ordinal,"fragments":link.fragments,"destination":destination,"origin":link.origin,"resolution":resolution})
}
fn occurrence(source: &str, requested: &Occurrence) -> Result<knowledge::NativeLink, ApiError> {
    let mut matching = knowledge::native_links_bounded(source, MAX_EDGES, 1024 * 1024)
        .links
        .into_iter()
        .filter(|l| {
            l.from == requested.from && l.to == requested.to && l.target == requested.target
        });
    let found = matching.next().ok_or_else(changed)?;
    if matching.next().is_some() {
        return Err(changed());
    }
    Ok(found)
}

async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Snapshot>,
) -> ApiResult {
    let check = auth.clone();
    let mut captured = locked(&state, move |inner| {
        let mut c = capture(inner, &check, req, true, false, true)?;
        let relevant = relevant_labels(&c);
        for index in 1..c.inventory.nodes.len() {
            if relevant.contains(&c.inventory.nodes[index].owner) {
                c.add_compile(inner, index);
            }
        }
        Ok(c)
    })
    .await?;
    compile(&state, &mut captured).await;
    locked(&state,move|inner|{
        recheck(inner,&auth,&captured,false)?;
        let writable=inner.rights(&captured.actor,&captured.resource)?.write && inner.file(&captured.resource)?.editable;
        let index=knowledge::link_index_bounded(&captured.inventory,&captured.source().owner,MAX_EDGES,1024*1024);
        let scan=mention_scan(&captured,writable)?;
        let mentions=scan.fragments.into_iter().map(|f|json!({"id":f.id,"nodeKey":f.node_key,"text":f.text,"coverage":f.coverage,"partialReason":f.partial_reason,"hits":f.hits.into_iter().map(|h|json!({"startScalar":h.start_scalar,"endScalar":h.end_scalar,"startUtf16":h.start_utf16,"endUtf16":h.end_utf16,"terms":h.terms,"writable":h.writable,"sourceRange":h.source_range,"lane":h.lane,"component":h.component,"candidates":h.candidates.iter().filter_map(|c|public_node(&captured,&c.target).map(|target|json!({"target":target,"fragment":c.fragment,"term":c.term,"provenance":c.provenance}))).collect::<Vec<_>>(),"targets":h.targets.iter().filter_map(|n|public_node(&captured,n)).collect::<Vec<_>>() })).collect::<Vec<_>>() })).collect::<Vec<_>>();
        let properties=knowledge::property_references(&captured.inventory,&captured.properties).into_iter().map(|p|json!({"source":public_node(&captured,&p.source),"fieldId":p.field_id,"itemPath":p.item_path,"resolution":public_resolution(&captured,&p.resolution,"")})).collect::<Vec<_>>();
        let rendered=captured.compilations.iter().find(|c|c.index==captured.selected).and_then(|c|c.output.as_ref()).filter(|o|o["ok"]==true).and_then(|o|serde_json::from_value::<weftext_engine::RenderNode>(o["tree"].clone()).ok()).map(|tree|knowledge::rendered_links(&tree,&format!("html:{}:{}",captured.request.session_id,captured.request.serial),"html",&captured.inventory.generation)).unwrap_or_default().into_iter().take(MAX_EDGES).map(|r|public_rendered(&captured,r)).collect::<Vec<_>>();
let diagnostic = knowledge::link_diagnostics(&captured.inventory, &captured.properties, captured.property_complete, MAX_EDGES);
        let diagnostics = json!({"generation":diagnostic.generation,"scope":diagnostic.scope,"coverage":diagnostic.coverage,"scannedNodes":diagnostic.scanned_nodes,"propertyIssues":diagnostic.property_issues.into_iter().map(|p|json!({"source":public_node(&captured,&p.source),"fieldId":p.field_id,"itemPath":p.item_path,"address":p.address,"resolution":public_resolution(&captured,&p.resolution,&p.address)})).collect::<Vec<_>>(),"issues":diagnostic.issues.into_iter().map(|e|public_edge(&captured,e)).collect::<Vec<_>>(),"nodes":diagnostic.nodes.into_iter().map(|n|json!({"node":public_node(&captured,&n.node),"incoming":n.incoming,"outgoing":n.outgoing,"invalidOutgoing":n.invalid_outgoing,"state":n.state})).collect::<Vec<_>>()});
let response=json!({"diagnostics":diagnostics,"basis":captured.basis(),"scope":"configuredAuthorizedManagedSources","links":{"coverage":index.coverage,"outgoing":index.outgoing.into_iter().map(|e|public_edge(&captured,e)).collect::<Vec<_>>(),"incomingId":index.incoming_id.into_iter().map(|e|public_edge(&captured,e)).collect::<Vec<_>>(),"incomingPath":index.incoming_path.into_iter().map(|e|public_edge(&captured,e)).collect::<Vec<_>>()},"renderedLinks":rendered,"propertyReferences":{"scope":"explicitAuthorizedFields","coverage":if captured.property_complete{"complete"}else{"partial"},"items":properties},"mentions":{"scope":"nativeTitlesAndAuthorizedNameFields","coverage":scan.coverage,"fragments":mentions},"writable":writable,"fileConversion":false});
        if serde_json::to_vec(&response).map_err(|_|changed())?.len()>MAX_OUTPUT {return Err(ApiError::bad("本次链接结果超过 8 MiB 交付预算；请缩小部署的获权范围。"));}
        Ok(Json(response))
    }).await
}
async fn proposal(
    state: &HostedState,
    auth: &Auth,
    req: Snapshot,
    snapshot: String,
    intent: Intent,
    write: bool,
) -> Result<(Captured, knowledge::SourcePatch), ApiError> {
    let check = auth.clone();
    let command = intent.clone();
    let mut captured = locked(state, move |inner| {
        let discovery = matches!(command, Intent::Mention { .. });
        let mut c = capture(inner, &check, req, false, write, discovery)?;
        if c.inventory.generation != snapshot {
            return Err(changed());
        }
        if let Intent::Link {
            occurrence: ref selected,
            form,
        } = command
        {
            if form == knowledge::LinkForm::File {
                return Err(file_form());
            }
            let selected = occurrence(&c.request.source, selected)?;
            let resolution = knowledge::resolve_link(&selected.target, c.source(), &c.inventory);
            if resolution.fragment.is_some()
                && let Some(target) = resolution.target
                && let Some(index) = c.inventory.nodes.iter().position(|n| n.owner == target)
            {
                c.add_compile(inner, index);
            }
        }
        Ok(c)
    })
    .await?;
    compile(state, &mut captured).await;
    let patch = match intent {
        Intent::Link {
            occurrence: selected,
            form,
        } => knowledge::convert_link_with_resource_fragments(
            captured.source(),
            &captured.inventory,
            selected.from,
            form,
            &captured.resource_fragments,
        )
        .map_err(knowledge_error)?,
        Intent::Mention {
            fragment_id,
            start_scalar,
            end_scalar,
            target,
            fragment: selected_fragment,
        } => {
            let scan = mention_scan(&captured, true)?;
            let fragment = scan
                .fragments
                .iter()
                .find(|f| f.id == fragment_id)
                .ok_or_else(changed)?;
            let hit = fragment
                .hits
                .iter()
                .find(|h| {
                    h.start_scalar == start_scalar
                        && h.end_scalar == end_scalar
                        && h.targets.contains(&target)
                })
                .ok_or_else(changed)?;
            knowledge::convert_mention_fragment(
                captured.source(),
                &captured.inventory,
                hit,
                &target,
                selected_fragment.as_deref(),
            )
            .map_err(knowledge_error)?
        }
    };
    Ok((captured, patch))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn source_values_and_schema_role_are_not_field_semantic_authority() {
        let mut private = Schema::bundled().unwrap();
        private
            .fields
            .iter_mut()
            .find(|field| field.id == "core/aliases")
            .unwrap()
            .key = "mysteryNames".into();
        private.fields.push(weftext_data::Field {
            id: "private/target-meaning".into(),
            key: "mysteryTarget".into(),
            shape: weftext_data::Shape::Scalar {
                semantic: "node_ref".into(),
            },
            nullable: false,
            default: None,
        });
        private
            .types
            .iter_mut()
            .find(|ty| ty.id == "library/work")
            .unwrap()
            .fields
            .push(weftext_data::FieldContribution {
                id: "private/target-meaning".into(),
                required: false,
            });
        let target = format!(
            "weftext-node://{}/{}",
            uuid::Uuid::new_v4(),
            uuid::Uuid::new_v4()
        );
        let source = format!(
            "#metadata((id: \"{}\", types: (\"work\",), mysteryNames: (\"OnlyPrivateSchemaMeaning\",), mysteryTarget: \"{target}\")) <weftext>\nOnlyPrivateSchemaMeaning",
            uuid::Uuid::new_v4()
        );
        // The exact same already-readable values acquire their meaning only
        // through private field definitions. Neither producer may run without
        // that field's separate read grant, even if Schema itself is readable.
        let original = field_semantics(&source, Some(&Schema::bundled().unwrap()), None).unwrap();
        let ungranted = field_semantics(&source, Some(&private), None).unwrap();
        assert_eq!(original.fingerprint, ungranted.fingerprint);
        assert!(
            ungranted.aliases.is_empty() && ungranted.references.is_empty() && ungranted.complete
        );
        let name = field_semantics(
            &source,
            Some(&private),
            Some(&BTreeSet::from(["core/aliases".into()])),
        )
        .unwrap();
        assert_eq!(name.aliases, vec!["OnlyPrivateSchemaMeaning"]);
        assert!(name.references.is_empty());
        let reference = field_semantics(
            &source,
            Some(&private),
            Some(&BTreeSet::from(["private/target-meaning".into()])),
        )
        .unwrap();
        assert!(reference.aliases.is_empty());
        assert_eq!(reference.references.len(), 1);
        assert_eq!(reference.references[0].field_id, "private/target-meaning");
        assert_eq!(reference.references[0].target, target);
    }

    #[test]
    fn original_field_grants_and_schema_witness_are_rechecked_but_source_only_reads_do_not_consume_schema()
     {
        let (_temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let source = doc["source"].as_str().unwrap();
        let plain = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        assert!(plain.schema_witness.is_none() && plain.semantic_fields.is_empty());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .fields
            .insert(
                "core/aliases".into(),
                FieldRights {
                    read: true,
                    write: false,
                },
            );
        let semantic = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        assert!(semantic.schema_witness.is_some());
        recheck(&mut inner, &auth, &semantic, false).unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .fields
            .get_mut("core/aliases")
            .unwrap()
            .read = false;
        assert!(recheck(&mut inner, &auth, &semantic, false).is_err());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .fields
            .get_mut("core/aliases")
            .unwrap()
            .read = true;
        let path = PathBuf::from(&semantic.schema_witness.as_ref().unwrap().0);
        std::fs::write(path, b"{\"version\":1,\"private-malformed-schema\":true}").unwrap();
        assert!(recheck(&mut inner, &auth, &semantic, false).is_err());
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .grants
            .get_mut("Shared")
            .unwrap()
            .fields
            .clear();
        let source_only = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        assert!(source_only.schema_witness.is_none());
        assert!(source_only.inventory.nodes[0].aliases.is_empty());
        assert!(source_only.properties.is_empty());
    }

    #[test]
    fn imported_member_semantics_recheck_uses_independent_member_fields_and_original_session() {
        let (_temp, state, auth, _, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let op = uuid::Uuid::new_v4().to_string();
        let admitted = inner
            .core
            .prepare_directory_admission(&root, &root, &op)
            .unwrap();
        inner
            .core
            .execute_operation(&admitted.preview.operation_id, &admitted.preview.plan_id)
            .unwrap();
        inner.config.directories.insert(
            "root".into(),
            query_scopes::Directory {
                path: String::new(),
                title: "Approved parent".into(),
                reference: admitted.target_ref,
                lifecycle: "live".into(),
            },
        );
        inner.config.content_import_slots.insert(
            "Native".into(),
            content_import::Slot {
                parent: "root".into(),
                name: "Native".into(),
                title: "Approved imported sources".into(),
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
                    members: Rights {
                        read: true,
                        write: true,
                        fields: BTreeMap::from([(
                            "tasks/status".into(),
                            FieldRights {
                                read: true,
                                write: false,
                            },
                        )]),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        let bytes = inner.core.capture_native_export(&root).unwrap().bytes;
        let op = uuid::Uuid::new_v4().to_string();
        let scope = format!("{}:content-import", inner.actor_scope("alice"));
        let prepared = inner
            .core
            .with_actor_for_root(&scope, &root, |c| {
                c.actor_feature_prepare_content_import(
                    "Native",
                    &op,
                    &digest(b"original native import"),
                    weftext_core::OperationRequest {
                        operation_id: op.clone(),
                        root: root.to_string_lossy().into_owned(),
                        intent: weftext_core::OperationIntent::ImportNative {
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
            .with_actor_for_root(&scope, &root, |c| {
                c.actor_feature_commit_with_losses(
                    "Native",
                    &op,
                    prepared.plan_id.as_deref().unwrap(),
                    true,
                )
            })
            .unwrap();
        let members = content_import::slot_member_ids(&inner, "Native").unwrap();
        assert!(members.len() >= 2);
        let selected = &members[0];
        let target = &members[1];
        let target_file = inner.file(target).unwrap();
        let target_owner = inner
            .core
            .qualified_node_at(&root, &root.join(target_file.path))
            .unwrap()
            .owner;
        let opened = inner.open("alice", selected).unwrap();
        let source = format!(
            "{}\n#link(\"{}\")[Original imported target]\n",
            opened["source"].as_str().unwrap(),
            uri(&target_owner)
        );
        let captured = capture(
            &mut inner,
            &auth,
            request(&opened, &source, 1),
            true,
            false,
            true,
        )
        .unwrap();
        assert!(!inner.policy.users["alice"].grants.contains_key(selected));
        assert_eq!(
            captured.semantic_fields[selected],
            BTreeSet::from(["tasks/status".into()])
        );
        recheck(&mut inner, &auth, &captured, false).unwrap();
        let index = knowledge::link_index_bounded(
            &captured.inventory,
            &captured.source().owner,
            MAX_EDGES,
            1024 * 1024,
        );
        let edge = index
            .outgoing
            .into_iter()
            .find(|e| e.occurrence.target == uri(&target_owner))
            .unwrap();
        assert_eq!(
            public_edge(&captured, edge)["resolution"]["target"]["resource"],
            target.as_str()
        );
        let member_fields = &mut inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .content_imports
            .get_mut("Native")
            .unwrap()
            .members
            .fields;
        member_fields.get_mut("tasks/status").unwrap().read = false;
        assert_eq!(
            recheck(&mut inner, &auth, &captured, false)
                .unwrap_err()
                .status,
            StatusCode::NOT_FOUND
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .content_imports
            .get_mut("Native")
            .unwrap()
            .members
            .fields
            .get_mut("tasks/status")
            .unwrap()
            .read = true;
        recheck(&mut inner, &auth, &captured, false).unwrap();
        let path = root.join(&inner.file(selected).unwrap().path);
        let original = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_extension("preserved")).unwrap();
        std::fs::write(&path, original).unwrap();
        assert!(recheck(&mut inner, &auth, &captured, false).is_err());
    }

    fn request(doc: &Value, source: &str, serial: u64) -> Snapshot {
        Snapshot {
            session_id: doc["sessionId"].as_str().unwrap().into(),
            source: source.into(),
            source_hash: digest(source.as_bytes()),
            serial,
        }
    }
    fn grant_target(inner: &mut HostedInner, actor: &str) {
        inner.policy.users.get_mut(actor).unwrap().grants.insert(
            "Target".into(),
            Rights {
                read: true,
                ..Default::default()
            },
        );
        inner.config.files.get_mut("Target").unwrap().editable = false;
    }

    #[test]
    fn hidden_locked_sources_are_never_read_or_counted_and_readonly_nodes_remain_candidates() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let locked_hidden = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(inner.config.root.join("Target/Target.typ"))
            .unwrap();
        let source = doc["source"].as_str().unwrap();
        let captured = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(captured.candidates.len(), 1);
        assert_eq!(captured.inventory.coverage, knowledge::Coverage::Complete);
        recheck(&mut inner, &auth, &captured, false).unwrap();
        drop(locked_hidden);
        grant_target(&mut inner, "alice");
        let captured = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(captured.candidates.len(), 2);
        assert!(!captured.candidates[1].file.editable);
        assert!(!author_file(&inner, "Target").unwrap().editable);
        assert_eq!(
            public_node(&captured, &captured.inventory.nodes[1].owner).unwrap()["canOpen"],
            true
        );
        inner.config.files.get_mut("Target").unwrap().expected_node = Some(knowledge::NodeRef {
            workspace_id: uuid::Uuid::new_v4().to_string(),
            node_id: uuid::Uuid::new_v4().to_string(),
        });
        assert!(author_file(&inner, "Target").is_err());
        assert!(recheck(&mut inner, &auth, &captured, false).is_err());
    }

    #[test]
    fn actor_epoch_equal_source_undo_and_original_identity_replacement_fence_delivery() {
        let (_temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let source = doc["source"].as_str().unwrap();
        let captured = capture(
            &mut inner,
            &auth,
            request(&doc, source, 1),
            true,
            false,
            false,
        )
        .unwrap();
        writing_api::observe_version(
            &mut inner.editor_versions,
            &captured.request.session_id,
            &format!("{source}later"),
            2,
        )
        .unwrap();
        writing_api::observe_version(
            &mut inner.editor_versions,
            &captured.request.session_id,
            source,
            3,
        )
        .unwrap();
        assert_eq!(
            recheck(&mut inner, &auth, &captured, false)
                .unwrap_err()
                .status,
            StatusCode::CONFLICT
        );
        let current = capture(
            &mut inner,
            &auth,
            request(&doc, source, 3),
            true,
            false,
            false,
        )
        .unwrap();
        assert_ne!(current.inventory.generation, captured.inventory.generation);
        inner.policy.users.get_mut("alice").unwrap().epoch += 1;
        assert!(recheck(&mut inner, &auth, &current, false).is_err());
        inner.policy.users.get_mut("alice").unwrap().epoch -= 1;
        let path = inner.config.root.join("Shared/Shared.typ");
        let saved = std::fs::read(&path).unwrap();
        std::fs::rename(&path, path.with_file_name("OwnedOriginal.typ")).unwrap();
        std::fs::write(&path, saved).unwrap();
        assert!(recheck(&mut inner, &auth, &current, false).is_err());
    }

    #[test]
    fn source_read_does_not_enlarge_target_or_field_only_authority_and_aliases_are_ambiguous() {
        let (_temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let target = inner
            .core
            .qualified_node_at(
                &inner.config.root,
                &inner.config.root.join("Target/Target.typ"),
            )
            .unwrap()
            .owner;
        let hidden = uri(&target);
        let unknown = format!(
            "weftext-node://{}/{}",
            target.workspace_id,
            uuid::Uuid::new_v4()
        );
        let source = format!(
            "{}\n#link(\"{hidden}\")[hidden authored address]\n#link(\"{unknown}\")[unknown authored address]\n",
            doc["source"].as_str().unwrap()
        );
        inner.policy.users.get_mut("alice").unwrap().grants.insert(
            "Target".into(),
            Rights {
                fields: BTreeMap::from([(
                    "core/aliases".into(),
                    FieldRights {
                        read: true,
                        write: false,
                    },
                )]),
                ..Default::default()
            },
        );
        let captured = capture(
            &mut inner,
            &auth,
            request(&doc, &source, 2),
            true,
            false,
            false,
        )
        .unwrap();
        assert_eq!(captured.candidates.len(), 1);
        let hidden_resolution =
            knowledge::resolve_link(&hidden, captured.source(), &captured.inventory);
        let unknown_resolution =
            knowledge::resolve_link(&unknown, captured.source(), &captured.inventory);
        assert_eq!(
            public_resolution(&captured, &hidden_resolution, &hidden),
            public_resolution(&captured, &unknown_resolution, &unknown)
        );
        assert!(public_node(&captured, &target).is_none());
        let alias = inner.config.files["Shared"].clone();
        inner.config.files.insert("Alias".into(), alias);
        inner.policy.users.get_mut("alice").unwrap().grants.insert(
            "Alias".into(),
            Rights {
                read: true,
                ..Default::default()
            },
        );
        assert!(
            capture(
                &mut inner,
                &auth,
                request(&doc, &source, 2),
                true,
                false,
                false
            )
            .is_err()
        );
    }

    #[test]
    fn outgoing_id_path_backlinks_and_copy_dto_use_only_authorized_coordinates() {
        let (_temp, state, auth, doc, _, _) = history_api::test_fixture();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let source_owner = inner
            .core
            .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
            .unwrap()
            .owner;
        let target = inner
            .core
            .open(root.join("Target/Target.typ"), Some(root.clone()))
            .unwrap();
        let target_source = format!(
            "{}\n#link(\"{}\")[ID incoming]\n#link(\"../Shared/Shared.typ\")[path incoming]\n",
            target.source,
            uri(&source_owner)
        );
        inner
            .core
            .save(weftext_core::SaveRequest {
                session_id: target.session_id.clone(),
                base_id: target.base_id,
                source: target_source,
                operation_id: uuid::Uuid::new_v4().to_string(),
            })
            .unwrap();
        inner.core.close_session(&target.session_id).unwrap();
        grant_target(&mut inner, "alice");
        let source = format!(
            "{}\n#link(\"../Target/Target.typ\")[*保留😀显示文字*]\n",
            doc["source"].as_str().unwrap()
        );
        let captured = capture(
            &mut inner,
            &auth,
            request(&doc, &source, 2),
            true,
            false,
            false,
        )
        .unwrap();
        let index = knowledge::link_index_bounded(
            &captured.inventory,
            &source_owner,
            MAX_EDGES,
            1024 * 1024,
        );
        assert_eq!(index.incoming_id.len(), 1);
        assert_eq!(index.incoming_path.len(), 1);
        let edge = index.outgoing.last().unwrap().clone();
        let patch = knowledge::convert_link(
            captured.source(),
            &captured.inventory,
            edge.occurrence.from,
            knowledge::LinkForm::Id,
        )
        .unwrap();
        assert!(patch.after.ends_with(")[*保留😀显示文字*]\n"));
        let dto = public_edge(&captured, edge);
        assert_eq!(dto["resolution"]["target"]["resource"], "Target");
        assert!(
            !dto.to_string()
                .contains(&root.to_string_lossy().into_owned())
        );
        recheck(&mut inner, &auth, &captured, false).unwrap();
        assert_eq!(
            std::fs::read_to_string(root.join("Shared/Shared.typ")).unwrap(),
            doc["source"].as_str().unwrap()
        );
    }

    #[tokio::test]
    async fn readonly_original_copy_preserves_broken_uri_and_file_forms_never_reveal_host_paths() {
        let (_temp, state, _auth, _doc, _, _) = history_api::test_fixture();
        let doc = state
            .inner
            .lock()
            .unwrap()
            .open("reader", "Shared")
            .unwrap();
        let auth = Auth {
            digest: digest(b"reader"),
        };
        let source = format!(
            "{}\n#link(\"file:///D:/authored/keep.typ#%E7%AB%A0\")[*原文字*]\n",
            doc["source"].as_str().unwrap()
        );
        let basis = request(&doc, &source, 2);
        writing_api::observe_version(
            &mut state.inner.lock().unwrap().editor_versions,
            &basis.session_id,
            &source,
            2,
        )
        .unwrap();
        let selected = knowledge::native_links(&source).last().unwrap().clone();
        let occurrence = Occurrence {
            from: selected.from,
            to: selected.to,
            target: selected.target,
        };
        let copied = copy(
            State(state.clone()),
            Extension(auth.clone()),
            Json(Copy {
                basis: basis.clone(),
                snapshot: "a".repeat(64),
                occurrence: occurrence.clone(),
                form: "original".into(),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(copied["address"], "file:///D:/authored/keep.typ#%E7%AB%A0");
        assert_eq!(
            copied["fullLink"],
            "#link(\"file:///D:/authored/keep.typ#%E7%AB%A0\")[*原文字*]"
        );
        assert_eq!(
            copy(
                State(state.clone()),
                Extension(auth.clone()),
                Json(Copy {
                    basis: basis.clone(),
                    snapshot: "a".repeat(64),
                    occurrence: occurrence.clone(),
                    form: "file".into()
                })
            )
            .await
            .unwrap_err()
            .code,
            "knowledgeLinksFileAddress"
        );
        assert_eq!(
            convert(
                State(state),
                Extension(auth),
                Json(Convert {
                    basis,
                    snapshot: "a".repeat(64),
                    intent: Intent::Link {
                        occurrence,
                        form: knowledge::LinkForm::Id
                    }
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
    }
}
async fn convert(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Convert>,
) -> ApiResult {
    let (captured, patch) =
        proposal(&state, &auth, req.basis, req.snapshot, req.intent, true).await?;
    locked(&state, move |inner| {
        recheck(inner, &auth, &captured, true)?;
        Ok(Json(
            json!({"basis":captured.basis(),"patch":patch,"saved":false}),
        ))
    })
    .await
}
async fn copy(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    Json(req): Json<Copy>,
) -> ApiResult {
    if req.form == "original" {
        return Ok(Json(locked(&state,move|inner|{
            let (actor,session,_)=qualify(inner,&auth,&req.basis,false,false)?;
            let root=inner.config.root.clone();let path=root.join(&inner.file(&session.resource)?.path);let scope=inner.actor_scope(&actor);
            let original=inner.core.with_actor_for_root(&scope,&root,|c|c.qualified_node_for_session_at(&root,&path,&req.basis.session_id)).map_err(|_|unavailable())?;
            if weftext_engine::inspect_metadata(&req.basis.source).id.as_deref()!=Some(original.owner.node_id.as_str()){return Err(changed());}
            let selected=occurrence(&req.basis.source,&req.occurrence)?;
            let full_link=authored_literal(&req.basis.source,&selected).ok_or_else(changed)?;
            inner.core.validate_snapshot_witness(&root,&path,&original.witness).map_err(|_|changed())?;
            Ok(json!({"basis":{"actor":actor,"actorEpoch":inner.actor_generation(&actor),"deployment":inner.config.deployment,"resource":session.resource,"sessionId":req.basis.session_id,"sourceHash":req.basis.source_hash,"serial":req.basis.serial,"snapshot":req.snapshot},"address":selected.target,"fullLink":full_link,"literal":true}))
        }).await?));
    }
    let form = match req.form.as_str() {
        "id" => knowledge::LinkForm::Id,
        "relative" => knowledge::LinkForm::Relative,
        "workspace" => knowledge::LinkForm::Workspace,
        "file" => return Err(file_form()),
        _ => return Err(ApiError::bad("请选择原文、稳定 ID、源相对或库内地址")),
    };
    let from = req.occurrence.from;
    let (captured, patch) = proposal(
        &state,
        &auth,
        req.basis,
        req.snapshot,
        Intent::Link {
            occurrence: req.occurrence,
            form,
        },
        false,
    )
    .await?;
    locked(&state,move|inner|{
        recheck(inner,&auth,&captured,false)?;
        let rewritten=knowledge::native_links(&patch.after).into_iter().find(|l|l.from==from).ok_or_else(changed)?;
        let full_link=authored_literal(&patch.after,&rewritten).ok_or_else(changed)?;
        Ok(Json(json!({"basis":captured.basis(),"address":rewritten.target,"fullLink":full_link,"literal":false})))
    }).await
}
