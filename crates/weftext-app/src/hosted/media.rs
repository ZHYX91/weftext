//! Exact configured ResourceRefs. Viewing, copying and discussing are separate grants.
use super::*;
use weftext_core::QualifiedResource;
use weftext_knowledge as knowledge;

const MAX_BYTES: usize = 32 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(900);
const CHUNK: usize = 64 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Resource {
    pub(super) owner: String,
    pub(super) expected_owner: knowledge::NodeRef,
    pub(super) path: String,
    pub(super) title: String,
    pub(super) reference: String,
    #[serde(default = "query_scopes::live")]
    pub(super) lifecycle: String,
}
#[derive(Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Rights {
    #[serde(default)]
    pub(super) read: bool,
    #[serde(default)]
    pub(super) download: bool,
}
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Purpose {
    View,
    Download,
}
#[derive(Clone)]
struct Ticket {
    actor: String,
    epoch: u64,
    asset: String,
    purpose: Purpose,
    binding: String,
    resource: Arc<QualifiedResource>,
    collection: Option<Arc<weftext_core::QualifiedResourceCollection>>,
    descriptor: Value,
    expires: Instant,
}
#[derive(Default)]
pub(super) struct Store {
    tickets: HashMap<String, Ticket>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Read {
    asset: String,
    revision: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChunkRequest {
    ticket: String,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Resolve {
    reference: String,
    revision: Option<String>,
}

pub(super) fn router() -> Router<HostedState> {
    Router::new()
        .route("/api/hosted/media", get(catalog))
        .route("/api/hosted/media/resolve", post(resolve))
        .route("/api/hosted/media/read", post(read))
        .route("/api/hosted/media/chunk", post(view_chunk))
        .route("/api/hosted/media/download/prepare", post(prepare_download))
        .route("/api/hosted/media/download/chunk", post(download_chunk))
}
fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn split_reference(value: &str) -> Option<(&str, &str)> {
    let (workspace, id) = value.strip_prefix("weftext-resource://")?.split_once('/')?;
    (canonical_uuid(workspace) && canonical_uuid(id)).then_some((workspace, id))
}
fn relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(':')
        && std::path::Path::new(path)
            .components()
            .all(|p| matches!(p, std::path::Component::Normal(_)))
}
pub(super) fn valid_config(config: &Config) -> bool {
    let mut refs = BTreeSet::new();
    config.resources.len() <= 128
        && config.resources.iter().all(|(id, r)| {
            let Some(owner) = config.files.get(&r.owner) else {
                return false;
            };
            valid_id(id)
                && !config.files.contains_key(id)
                && relative(&r.path)
                && !r.title.is_empty()
                && r.title.len() <= 500
                && matches!(r.lifecycle.as_str(), "live" | "trash")
                && r.lifecycle == owner.lifecycle
                && canonical_uuid(&r.expected_owner.workspace_id)
                && canonical_uuid(&r.expected_owner.node_id)
                && split_reference(&r.reference)
                    .is_some_and(|(w, _)| w == r.expected_owner.workspace_id)
                && owner
                    .expected_node
                    .as_ref()
                    .is_none_or(|n| n == &r.expected_owner)
                && refs.insert(&r.reference)
        })
}
pub(super) fn valid_rights(config: &Config, rights: &BTreeMap<String, Rights>) -> bool {
    rights.len() <= 128
        && rights
            .iter()
            .all(|(id, r)| config.resources.contains_key(id) && (!r.download || r.read))
}
pub(super) fn input_access(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    download: bool,
) -> Result<(), ApiError> {
    if let Some(slot) = asset
        .strip_suffix("/*")
        .filter(|s| inner.config.content_import_slots.contains_key(*s))
    {
        let r = inner
            .policy
            .users
            .get(actor)
            .filter(|u| u.enabled)
            .and_then(|u| u.content_imports.get(slot))
            .ok_or_else(unavailable)?;
        return if r.members.read && r.resources.read && (!download || r.resources.download) {
            Ok(())
        } else {
            Err(unavailable())
        };
    }
    if let Some(collection) = resource_collections::selection(&inner.config, asset)
        .or_else(|| resource_collections::split_member(&inner.config, asset).map(|(c, _)| c))
    {
        return resource_collections::input_access(inner, actor, collection, download);
    }
    if !inner.config.resources.contains_key(asset) {
        return Err(unavailable());
    }
    inner
        .policy
        .users
        .get(actor)
        .filter(|u| u.enabled)
        .and_then(|u| u.resources.get(asset))
        .filter(|r| r.read && (!download || r.download))
        .map(|_| ())
        .ok_or_else(unavailable)
}
pub(super) fn is_asset(config: &Config, id: &str) -> bool {
    config.resources.contains_key(id) || resource_collections::split_member(config, id).is_some()
}
pub(super) fn valid_input(config: &Config, id: &str) -> bool {
    is_asset(config, id)
        || resource_collections::selection(config, id).is_some()
        || content_import::selection(config, id).is_some()
}
/// Resolve a single registered member. Never derives a locator from a client UUID.
pub(super) fn resolve_asset(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
) -> Result<Resource, ApiError> {
    resolve_with_collection(inner, actor, asset).map(|(r, _)| r)
}
type ResolvedAsset = (
    Resource,
    Option<Arc<weftext_core::QualifiedResourceCollection>>,
);
fn resolve_with_collection(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
) -> Result<ResolvedAsset, ApiError> {
    input_access(inner, actor, asset, false)?;
    if let Some(configured) = inner.config.resources.get(asset) {
        return Ok((configured.clone(), None));
    }
    let (collection, member) =
        resource_collections::split_member(&inner.config, asset).ok_or_else(unavailable)?;
    let captured = Arc::new(resource_collections::capture(inner, actor, collection)?);
    let resource = resource_from_collection(inner, collection, member, &captured)?;
    inner
        .core
        .validate_resource_collection_witness(&inner.config.root, &captured.witness)
        .map_err(|_| changed())?;
    Ok((resource, Some(captured)))
}
/// Resolve only the member of this original cut. This performs no current
/// collection capture and grants no access by itself; callers authorize first
/// and retain the cut's final witness check before delivering a projection.
fn resource_from_collection(
    inner: &HostedInner,
    collection: &str,
    member: &str,
    captured: &weftext_core::QualifiedResourceCollection,
) -> Result<Resource, ApiError> {
    let mut matching = captured
        .entries
        .iter()
        .filter(|entry| entry.resource_ref.rsplit('/').next() == Some(member));
    let entry = matching.next().ok_or_else(unavailable)?;
    if matching.next().is_some() {
        return Err(unavailable());
    }
    let c = resource_collections::configured(inner, collection)?;
    let owner = &inner.file(&c.owner)?;
    let directory = std::path::Path::new(&owner.path)
        .parent()
        .ok_or_else(changed)?;
    let path = directory
        .join(&entry.relative_path)
        .to_string_lossy()
        .into_owned();
    let title = std::path::Path::new(&entry.relative_path)
        .file_name()
        .and_then(|p| p.to_str())
        .ok_or_else(changed)?
        .to_owned();
    Ok(Resource {
        owner: c.owner.clone(),
        expected_owner: captured.owner.clone(),
        path,
        title,
        reference: entry.resource_ref.clone(),
        lifecycle: c.lifecycle.clone(),
    })
}
fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "resourceVersion",
        UiMessage::error_api_hosted_media_resource_basis_changed(),
    )
}
fn gone() -> ApiError {
    ApiError::new(
        StatusCode::GONE,
        "mediaExpired",
        UiMessage::error_api_hosted_media_resource_delivery_expired(),
    )
}
pub(super) fn binding_hash(
    inner: &HostedInner,
    assets: &BTreeSet<String>,
) -> Result<String, ApiError> {
    let mut resources = BTreeMap::new();
    let mut collections = BTreeMap::new();
    let mut imports = BTreeMap::new();
    let mut owners = BTreeMap::new();
    for id in assets {
        if content_import::selection(&inner.config, id).is_some()
            || content_import::split_asset(&inner.config, id).is_some()
        {
            imports.insert(id, content_import::binding(inner, id)?);
            continue;
        }
        let owner = if let Some(r) = inner.config.resources.get(id) {
            resources.insert(id, r);
            r.owner.clone()
        } else {
            let collection = resource_collections::selection(&inner.config, id)
                .or_else(|| resource_collections::split_member(&inner.config, id).map(|(c, _)| c))
                .ok_or_else(unavailable)?;
            let c = resource_collections::configured(inner, collection)?;
            let owner = c.owner.clone();
            collections.insert(id, c);
            owner
        };
        owners.insert(
            owner.clone(),
            inner.file(&owner).ok().ok_or_else(unavailable)?,
        );
    }
    let mut value = json!({"root":inner.config.root,"resources":resources,"collections":collections,"owners":owners});
    if !imports.is_empty() {
        value["imports"] = json!(imports);
    }
    Ok(digest(&serde_json::to_vec(&value).map_err(|_| changed())?))
}
pub(super) fn capture_input(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
) -> Result<QualifiedResource, ApiError> {
    input_access(inner, actor, asset, false)?;
    let r = resolve_asset(inner, actor, asset)?;
    let owner = inner.file(&r.owner).ok().ok_or_else(unavailable)?;
    inner
        .core
        .qualified_resource_at(
            &inner.config.root,
            &inner.config.root.join(&owner.path),
            &r.expected_owner,
            &r.reference,
            &inner.config.root.join(&r.path),
            &r.lifecycle,
            None,
        )
        .map_err(|_| changed())
}
pub(super) fn exact(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    revision: &str,
) -> Result<QualifiedResource, ApiError> {
    input_access(inner, actor, asset, false)?;
    if revision.is_empty() || revision.len() > 100 {
        return Err(changed());
    }
    let r = resolve_asset(inner, actor, asset)?;
    let owner = inner.file(&r.owner).ok().ok_or_else(unavailable)?;
    inner
        .core
        .qualified_resource_at(
            &inner.config.root,
            &inner.config.root.join(&owner.path),
            &r.expected_owner,
            &r.reference,
            &inner.config.root.join(&r.path),
            &r.lifecycle,
            Some(revision),
        )
        .map_err(|_| changed())
}
pub(super) fn validate_original(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    original: &QualifiedResource,
) -> Result<(), ApiError> {
    input_access(inner, actor, asset, false)?;
    inner
        .core
        .validate_resource_witness(&inner.config.root, &original.witness)
        .map_err(|_| changed())?;
    let configured = resolve_asset(inner, actor, asset)?;
    if configured.expected_owner != original.owner
        || configured.reference != original.resource_ref
        || configured.lifecycle != original.lifecycle
    {
        return Err(changed());
    }
    Ok(())
}
fn profile(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    resource: &QualifiedResource,
    config: &Resource,
) -> Result<(Value, crate::media_api::ResourceMetadata), ApiError> {
    input_access(inner, actor, asset, false)?;
    if resource.revision.is_none() {
        return Err(changed());
    }
    if resource.witness.retained_bytes() > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_media_resource_delivery_budget(),
        ));
    }
    let (metadata, content_type) = crate::media_api::inspect_media(resource.bytes());
    let ready = resource.revision.as_ref().is_some_and(|rev| {
        crate::media_api::metadata_facts(&metadata, &resource.resource_ref, rev).is_some()
    });
    let download = input_access(inner, actor, asset, true).is_ok();
    let mut descriptor = json!({"asset":asset,"resourceId":asset,"owner":config.owner,"resource":resource.resource_ref,"name":config.title,"title":config.title,"revision":resource.revision,"bytes":resource.bytes().len(),"contentType":content_type,"metadata":metadata,"annotationReady":ready,"wholeAnnotationReady":resource.revision.is_some(),"currentness":if resource.revision.is_some(){"accepted"}else{"observed"},"lifecycle":resource.lifecycle,"download":download,"status":"ready"});
    if let Some((collection, _)) = resource_collections::split_member(&inner.config, asset) {
        descriptor["collection"] = collection.into();
    }
    if resource.witness.retained_bytes().saturating_add(
        serde_json::to_vec(&descriptor)
            .map_err(|_| changed())?
            .len(),
    ) > MAX_BYTES
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_media_resource_delivery_budget(),
        ));
    }
    Ok((descriptor, metadata))
}
pub(super) struct Context {
    pub(super) asset: String,
    pub(super) resource: Arc<QualifiedResource>,
    pub(super) metadata: crate::media_api::ResourceMetadata,
    pub(super) descriptor: Value,
    pub(super) collection: Option<Arc<weftext_core::QualifiedResourceCollection>>,
}
impl Context {
    fn owned_retained_bytes(&self) -> usize {
        self.resource.witness.retained_bytes()
            + serde_json::to_vec(&(&self.metadata, &self.descriptor)).map_or(MAX_BYTES, |v| v.len())
    }
    pub(super) fn retained_bytes(&self) -> usize {
        self.owned_retained_bytes() + self.collection.as_ref().map_or(0, |c| c.retained_bytes())
    }
    pub(super) fn validate(&self, inner: &HostedInner, actor: &str) -> Result<(), ApiError> {
        validate_original(inner, actor, &self.asset, &self.resource)?;
        if let Some(c) = &self.collection {
            inner
                .core
                .validate_resource_collection_witness(&inner.config.root, &c.witness)
                .map_err(|_| changed())?;
        }
        Ok(())
    }
}
pub(super) fn context(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    revision: Option<&str>,
) -> Result<Context, ApiError> {
    let resolved = resolve_with_collection(inner, actor, asset)?;
    let context = context_from_resolution(inner, actor, asset, revision, resolved)?;
    context.validate(inner, actor)?;
    Ok(context)
}
// Synchronous capture/projection only. Both callers retain their final
// Context::validate gate before any descriptor or original bytes can leave.
fn context_from_resolution(
    inner: &HostedInner,
    actor: &str,
    asset: &str,
    revision: Option<&str>,
    resolved: ResolvedAsset,
) -> Result<Context, ApiError> {
    let (configured, collection) = resolved;
    let owner = inner.file(&configured.owner).ok().ok_or_else(unavailable)?;
    let resource = Arc::new(
        inner
            .core
            .qualified_resource_at(
                &inner.config.root,
                &inner.config.root.join(&owner.path),
                &configured.expected_owner,
                &configured.reference,
                &inner.config.root.join(&configured.path),
                &configured.lifecycle,
                revision,
            )
            .map_err(|_| changed())?,
    );
    if resource
        .witness
        .retained_bytes()
        .saturating_add(collection.as_ref().map_or(0, |c| c.retained_bytes()))
        > MAX_BYTES
    {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_media_resource_target_basis_budget(),
        ));
    }
    let (descriptor, metadata) = profile(inner, actor, asset, &resource, &configured)?;
    let context = Context {
        asset: asset.into(),
        resource,
        metadata,
        descriptor,
        collection,
    };
    if context.retained_bytes() > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_media_resource_target_basis_budget(),
        ));
    }
    Ok(context)
}
pub(super) fn expire_revoked(inner: &mut HostedInner) {
    let now = Instant::now();
    let policy = &inner.policy;
    let config = &inner.config;
    inner.media.tickets.retain(|_, t| {
        t.expires > now
            && policy.users.get(&t.actor).is_some_and(|u| {
                u.enabled
                    && u.epoch == t.epoch
                    && if let Some((collection, _)) =
                        resource_collections::split_member(config, &t.asset)
                    {
                        u.resource_collections
                            .get(collection)
                            .or_else(|| {
                                content_import::split_member(config, collection).and_then(
                                    |(s, _)| u.content_imports.get(s).map(|r| &r.resources),
                                )
                            })
                            .is_some_and(|r| r.read && (t.purpose == Purpose::View || r.download))
                    } else {
                        u.resources
                            .get(&t.asset)
                            .is_some_and(|r| r.read && (t.purpose == Purpose::View || r.download))
                    }
            })
    });
}
fn retain(inner: &mut HostedInner, ticket: Ticket) -> Result<String, ApiError> {
    expire_revoked(inner);
    let bytes = ticket.resource.witness.retained_bytes()
        + ticket.collection.as_ref().map_or(0, |c| c.retained_bytes())
        + serde_json::to_vec(&ticket.descriptor)
            .map_err(|_| changed())?
            .len();
    if bytes > MAX_BYTES {
        return Err(ApiError::bad(
            UiMessage::error_api_hosted_media_resource_delivery_budget(),
        ));
    }
    let mut retained = inner
        .media
        .tickets
        .iter()
        .map(|(id, t)| {
            (
                id.clone(),
                t.resource.witness.retained_bytes()
                    + t.collection.as_ref().map_or(0, |c| c.retained_bytes())
                    + serde_json::to_vec(&t.descriptor).map_or(MAX_BYTES, |v| v.len()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates = inner
        .media
        .tickets
        .iter()
        .filter(|(_, t)| t.actor == ticket.actor)
        .map(|(id, t)| (id.clone(), t.expires))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(_, at)| *at);
    let mut evict = vec![];
    while retained.len() >= 32
        || retained
            .keys()
            .filter(|id| inner.media.tickets[*id].actor == ticket.actor)
            .count()
            >= 8
        || retained.values().sum::<usize>().saturating_add(bytes) > MAX_BYTES
    {
        let Some((id, _)) = candidates.first().cloned() else {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_media_resource_delivery_cache_budget(),
            ));
        };
        candidates.remove(0);
        retained.remove(&id);
        evict.push(id);
    }
    for id in evict {
        inner.media.tickets.remove(&id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    inner.media.tickets.insert(id.clone(), ticket);
    Ok(id)
}
fn catalog_rights(
    inner: &HostedInner,
    actor: &str,
    collection: &str,
) -> Option<resource_collections::Rights> {
    let user = inner.policy.users.get(actor).filter(|user| user.enabled)?;
    user.resource_collections
        .get(collection)
        .cloned()
        .or_else(|| content_import::resource_rights(inner, actor, collection))
        .filter(|rights| rights.read || rights.r#import)
}

fn unavailable_catalog_row(inner: &HostedInner, asset: &str, config: Option<Resource>) -> Value {
    let mut row = json!({"asset":asset,"status":"unavailable"});
    if let Some(config) = config {
        row["owner"] = config.owner.into();
        row["name"] = config.title.clone().into();
        row["title"] = config.title.into();
        row["lifecycle"] = config.lifecycle.into();
    }
    if let Some((collection, _)) = resource_collections::split_member(&inner.config, asset) {
        row["collection"] = collection.into();
    }
    row
}

enum CatalogResource {
    Ready(Context),
    Unavailable(Value),
}
impl CatalogResource {
    fn asset(&self) -> &str {
        match self {
            Self::Ready(context) => &context.asset,
            Self::Unavailable(row) => row["asset"].as_str().unwrap(),
        }
    }
}

/// One request-local cut. Collection Arcs are shared only among this catalog's
/// members; each resource still owns its exact independent Core witness.
struct CatalogCut {
    resources: Vec<CatalogResource>,
    collections: Vec<Value>,
    cuts: BTreeMap<String, Arc<weftext_core::QualifiedResourceCollection>>,
    bindings: BTreeSet<String>,
    binding: String,
    retained: usize,
    complete: bool,
}
impl CatalogCut {
    fn resolution(
        &self,
        inner: &HostedInner,
        actor: &str,
        asset: &str,
    ) -> Result<ResolvedAsset, ApiError> {
        input_access(inner, actor, asset, false)?;
        if let Some(configured) = inner.config.resources.get(asset) {
            return Ok((configured.clone(), None));
        }
        let (collection, member) =
            resource_collections::split_member(&inner.config, asset).ok_or_else(unavailable)?;
        let captured = self.cuts.get(collection).ok_or_else(changed)?;
        Ok((
            resource_from_collection(inner, collection, member, captured)?,
            Some(Arc::clone(captured)),
        ))
    }

    fn capture(inner: &HostedInner, actor: &str) -> Result<Self, ApiError> {
        Self::capture_with(inner, actor, |inner, id| {
            resource_collections::qualify(inner, id).map(Arc::new)
        })
    }

    fn capture_with(
        inner: &HostedInner,
        actor: &str,
        mut capture: impl FnMut(
            &HostedInner,
            &str,
        )
            -> Result<Arc<weftext_core::QualifiedResourceCollection>, ApiError>,
    ) -> Result<Self, ApiError> {
        let mut cut = Self {
            resources: vec![],
            collections: vec![],
            cuts: BTreeMap::new(),
            bindings: BTreeSet::new(),
            binding: String::new(),
            retained: 0,
            complete: true,
        };
        let mut assets = inner
            .config
            .resources
            .keys()
            .filter(|id| input_access(inner, actor, id, false).is_ok())
            .cloned()
            .collect::<Vec<_>>();
        cut.bindings.extend(assets.iter().cloned());
        let mut configured = inner.config.resource_collections.clone();
        for id in content_import::collection_ids(inner, actor)? {
            configured.insert(id.clone(), resource_collections::configured(inner, &id)?);
        }
        let mut retained_cuts = BTreeSet::new();
        for (id, config) in configured {
            let Some(rights) = catalog_rights(inner, actor, &id) else {
                continue;
            };
            cut.bindings.insert(format!("{id}/*"));
            let mut row = json!({"collection":id,"owner":config.owner,"title":resource_collections::title(inner,&id)?,"lifecycle":config.lifecycle,"read":rights.read,"download":rights.download,"import":rights.r#import,"status":"unavailable"});
            let row_bytes = serde_json::to_vec(&row).map_err(|_| changed())?.len();
            cut.retained = cut.retained.saturating_add(row_bytes);
            if cut.retained > MAX_BYTES {
                return Err(ApiError::bad(
                    UiMessage::error_api_hosted_media_resource_catalog_budget(),
                ));
            }
            match capture(inner, &id) {
                Ok(captured) => {
                    // Pointer identity is actual shared ownership. Equal owner,
                    // ref or content does not deduplicate distinct witnesses.
                    let pointer = Arc::as_ptr(&captured) as usize;
                    let added = if retained_cuts.contains(&pointer) {
                        0
                    } else {
                        captured.retained_bytes()
                    };
                    if cut.retained.saturating_add(added) <= MAX_BYTES {
                        cut.retained += added;
                        retained_cuts.insert(pointer);
                        if rights.read {
                            if captured.unrecognized_entries > 0 {
                                cut.complete = false;
                            }
                            for entry in &captured.entries {
                                assets.push(format!(
                                    "{id}/{}",
                                    entry.resource_ref.rsplit('/').next().ok_or_else(changed)?
                                ));
                            }
                        }
                        cut.cuts.insert(id, captured);
                        row["status"] = "ready".into();
                    } else {
                        cut.complete = false;
                    }
                }
                Err(_) => cut.complete = false,
            }
            cut.retained = cut
                .retained
                .saturating_sub(row_bytes)
                .saturating_add(serde_json::to_vec(&row).map_err(|_| changed())?.len());
            cut.collections.push(row);
        }
        for asset in assets {
            let resolved = cut.resolution(inner, actor, &asset);
            let config = resolved.as_ref().ok().map(|(config, _)| config.clone());
            let fallback = unavailable_catalog_row(inner, &asset, config);
            let fallback_bytes = serde_json::to_vec(&fallback).map_err(|_| changed())?.len();
            let ready = resolved
                .and_then(|resolved| context_from_resolution(inner, actor, &asset, None, resolved));
            let entry = match ready {
                Ok(context)
                    if cut.retained.saturating_add(context.owned_retained_bytes()) <= MAX_BYTES =>
                {
                    cut.retained += context.owned_retained_bytes();
                    CatalogResource::Ready(context)
                }
                _ => {
                    cut.complete = false;
                    cut.retained = cut.retained.saturating_add(fallback_bytes);
                    if cut.retained > MAX_BYTES {
                        return Err(ApiError::bad(
                            UiMessage::error_api_hosted_media_resource_catalog_budget(),
                        ));
                    }
                    CatalogResource::Unavailable(fallback)
                }
            };
            cut.resources.push(entry);
        }
        cut.binding = binding_hash(inner, &cut.bindings)?;
        cut.retained = cut
            .retained
            .saturating_add(cut.binding.len())
            .saturating_add(cut.bindings.iter().map(String::len).sum::<usize>());
        if cut.retained > MAX_BYTES {
            return Err(ApiError::bad(
                UiMessage::error_api_hosted_media_resource_catalog_budget(),
            ));
        }
        Ok(cut)
    }

    fn finish(self, inner: &HostedInner, actor: &str) -> Result<Value, ApiError> {
        self.finish_with(inner, actor, |inner, captured| {
            inner
                .core
                .validate_resource_collection_witness(&inner.config.root, &captured.witness)
                .map_err(|_| changed())
        })
    }

    fn finish_with(
        mut self,
        inner: &HostedInner,
        actor: &str,
        mut validate_collection: impl FnMut(
            &HostedInner,
            &weftext_core::QualifiedResourceCollection,
        ) -> Result<(), ApiError>,
    ) -> Result<Value, ApiError> {
        // Freeze both each configured locator and the owner's file binding.
        // An old cut must not prove itself after a valid configuration remap.
        if !binding_hash(inner, &self.bindings).is_ok_and(|binding| binding == self.binding) {
            return Err(changed());
        }
        let mut resources = vec![];
        for entry in std::mem::take(&mut self.resources) {
            let asset = entry.asset();
            // Recheck each alias before resource proof or descriptor delivery.
            // An import-only collection must not inherit its old read grant.
            if input_access(inner, actor, asset, false).is_err() {
                self.complete = false;
                continue;
            }
            let row = match entry {
                CatalogResource::Ready(context) => {
                    let configured = self
                        .resolution(inner, actor, &context.asset)
                        .map(|(config, _)| config);
                    let valid = configured.as_ref().is_ok_and(|config| {
                        config.expected_owner == context.resource.owner
                            && config.reference == context.resource.resource_ref
                            && config.lifecycle == context.resource.lifecycle
                    }) && inner
                        .core
                        .validate_resource_witness(&inner.config.root, &context.resource.witness)
                        .is_ok();
                    if valid {
                        context.descriptor
                    } else {
                        self.complete = false;
                        unavailable_catalog_row(inner, &context.asset, configured.ok())
                    }
                }
                CatalogResource::Unavailable(row) => row,
            };
            resources.push(row);
        }
        self.collections.retain_mut(|row| {
            let id = row["collection"].as_str().unwrap();
            let Some(rights) = catalog_rights(inner, actor, id) else {
                return false;
            };
            row["read"] = rights.read.into();
            row["download"] = rights.download.into();
            row["import"] = rights.r#import.into();
            true
        });
        let mut validated = BTreeMap::new();
        for (id, captured) in &self.cuts {
            if catalog_rights(inner, actor, id).is_none() {
                continue;
            }
            let pointer = Arc::as_ptr(captured) as usize;
            let valid = *validated
                .entry(pointer)
                .or_insert_with(|| validate_collection(inner, captured).is_ok());
            if !valid {
                self.complete = false;
                for row in &mut self.collections {
                    if row["collection"] == *id {
                        row["status"] = "unavailable".into();
                    }
                }
                resources.retain(|row| row["collection"] != *id);
            }
        }
        Ok(
            json!({"resources":resources,"collections":self.collections,"coverage":if self.complete{"complete"}else{"partial"},"scope":"configuredAuthorizedResources"}),
        )
    }
}

async fn catalog(State(state): State<HostedState>, Extension(auth): Extension<Auth>) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            CatalogCut::capture(inner, &actor)?.finish(inner, &actor)
        })
        .await?,
    ))
}
async fn prepare_delivery(
    state: HostedState,
    auth: Auth,
    req: Read,
    purpose: Purpose,
) -> ApiResult {
    Ok(Json(
        locked(&state, move |inner| {
            let actor = inner.actor(&auth)?;
            input_access(inner, &actor, &req.asset, purpose == Purpose::Download)?;
            let context = context(inner, &actor, &req.asset, Some(&req.revision))?;
            context.validate(inner, &actor)?;
            let resource = context.resource;
            let mut descriptor = context.descriptor;
            descriptor["purpose"] = value(purpose)?;
            descriptor["sha256"] = resource.content_hash.clone().into();
            descriptor["expiresInMs"] = 900000.into();
            let ticket = Ticket {
                actor: actor.clone(),
                epoch: inner.actor_generation(&actor),
                binding: binding_hash(inner, &BTreeSet::from([req.asset.clone()]))?,
                asset: req.asset,
                purpose,
                resource,
                collection: context.collection,
                descriptor: descriptor.clone(),
                expires: Instant::now() + TTL,
            };
            descriptor["ticket"] = retain(inner, ticket)?.into();
            Ok(descriptor)
        })
        .await?,
    ))
}
async fn read(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Read>,
) -> ApiResult {
    prepare_delivery(state, auth, req, Purpose::View).await
}
async fn prepare_download(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Read>,
) -> ApiResult {
    prepare_delivery(state, auth, req, Purpose::Download).await
}
async fn deliver(state: HostedState, auth: Auth, req: ChunkRequest, purpose: Purpose) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;expire_revoked(inner);
        let t=inner.media.tickets.get(&req.ticket).filter(|t|t.actor==actor && t.purpose==purpose).cloned().ok_or_else(gone)?;
        input_access(inner,&actor,&t.asset,purpose==Purpose::Download)?;
        if t.epoch!=inner.actor_generation(&actor) || binding_hash(inner,&BTreeSet::from([t.asset.clone()]))?!=t.binding {return Err(gone());}
        validate_original(inner,&actor,&t.asset,&t.resource).map_err(|_|gone())?;
        if let Some(c)=&t.collection {inner.core.validate_resource_collection_witness(&inner.config.root,&c.witness).map_err(|_|gone())?;}
        let original=t.resource.bytes();if req.offset>original.len(){return Err(ApiError::bad(UiMessage::error_api_hosted_media_resource_offset_invalid()));}
        let end=req.offset.saturating_add(CHUNK).min(original.len());let bytes=&original[req.offset..end];
        Ok(json!({"ticket":req.ticket,"purpose":purpose,"bytesBase64":base64::engine::general_purpose::STANDARD.encode(bytes),"offset":req.offset,"length":bytes.len(),"total":original.len(),"sha256":t.resource.content_hash,"done":end==original.len()}))
    }).await?))
}
async fn view_chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<ChunkRequest>,
) -> ApiResult {
    deliver(state, auth, req, Purpose::View).await
}
async fn download_chunk(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<ChunkRequest>,
) -> ApiResult {
    deliver(state, auth, req, Purpose::Download).await
}
async fn resolve(
    State(state): State<HostedState>,
    Extension(auth): Extension<Auth>,
    RequestJson(req): RequestJson<Resolve>,
) -> ApiResult {
    Ok(Json(locked(&state,move|inner| {
        let actor=inner.actor(&auth)?;
        let base=req.reference.split_once('#').map_or(req.reference.as_str(),|(base,_)|base);
        if req.reference.len()>2048 || split_reference(base).is_none(){return Err(ApiError::bad(UiMessage::error_api_hosted_media_stable_resource_reference_required()));}
        let mut candidates=inner.config.resources.iter().filter(|(id,r)|r.reference==base && input_access(inner,&actor,id,false).is_ok()).map(|(id,_)|id.clone()).collect::<BTreeSet<_>>();
        let mut bytes=0usize;
        let mut configured_collections=inner.config.resource_collections.clone();
        for id in content_import::collection_ids(inner,&actor)? {configured_collections.insert(id.clone(),resource_collections::configured(inner,&id)?);}
        for (id,c) in &configured_collections {
            if c.expected_owner.as_ref().is_some_and(|owner|owner.workspace_id!=split_reference(base).unwrap().0) || resource_collections::input_access(inner,&actor,id,false).is_err(){continue;}
            if let Ok(captured)=resource_collections::capture(inner,&actor,id) {
                bytes=bytes.saturating_add(captured.retained_bytes());
                if bytes>MAX_BYTES {return Err(ApiError::bad(UiMessage::error_api_hosted_media_resource_resolution_budget()));}
                if let Some(e)=captured.entries.iter().find(|e|e.resource_ref==base) {candidates.insert(format!("{id}/{}",e.resource_ref.rsplit('/').next().unwrap()));}
            }
        }
        // Multiple explicitly granted aliases of the same original object are
        // useful. Prove equality, then choose a deterministic authorized alias;
        // its independent download grant alone controls its delivery.
        let mut selected:Option<Context>=None;let mut failure=None;
        for candidate in candidates {
            match context(inner,&actor,&candidate,req.revision.as_deref()) {
                Ok(candidate)=>{
                    bytes=bytes.saturating_add(candidate.retained_bytes());
                    if bytes>MAX_BYTES {return Err(ApiError::bad(UiMessage::error_api_hosted_media_resource_resolution_budget()));}
                    if let Some(previous)=&selected {
                        if previous.resource.owner!=candidate.resource.owner || previous.resource.resource_ref!=candidate.resource.resource_ref
                            || previous.resource.revision!=candidate.resource.revision || previous.resource.content_hash!=candidate.resource.content_hash
                            || previous.resource.lifecycle!=candidate.resource.lifecycle {return Err(unavailable());}
                        candidate.validate(inner,&actor)?;
                    } else {selected=Some(candidate);}
                },Err(error)=>failure=Some(error),
            }
        }
        let context=selected.ok_or_else(||failure.unwrap_or_else(unavailable))?;
        let config=resolve_asset(inner,&actor,&context.asset)?;
        let resource=&context.resource;let revision=resource.revision.clone().ok_or_else(changed)?;
        let inventory=knowledge::InventorySnapshot {generation:String::new(),coverage:knowledge::Coverage::Complete,nodes:vec![],resources:vec![knowledge::ResourceSnapshot {key:resource.resource_ref.clone(),path:String::new(),revision_id:revision.clone()}]};
        let source=knowledge::NodeSnapshot {owner:config.expected_owner.clone(),path:String::new(),library_path:String::new(),revision_id:String::new(),source:String::new(),title:None,aliases:vec![],labels:vec![],labels_complete:false};
        let resolution=knowledge::resolve_link(&req.reference,&source,&inventory);
        if resolution.resource.as_deref()!=Some(&resource.resource_ref){return Err(unavailable());}
        let mut descriptor=context.descriptor.clone();
        let fragment=crate::knowledge_api::resource_fragment_metadata(&context.metadata,resolution.fragment.as_deref());
        descriptor["viewerTarget"]=json!({"resource":resource.resource_ref,"revision":revision,"fragment":resolution.fragment,"resourceFragment":fragment});
        context.validate(inner,&actor)?;Ok(descriptor)
    }).await?))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    #[test]
    fn reference_profile_rejects_path_aliases_and_partial_ids() {
        let ws = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4().to_string();
        assert!(split_reference(&format!("weftext-resource://{ws}/{id}")).is_some());
        for reference in [
            format!("weftext-resource://{ws}/{id}/other"),
            format!("weftext-resource://{ws}/{id}?path=private"),
            "file:///private/resource.png".into(),
            "../resource.png".into(),
        ] {
            assert!(split_reference(&reference).is_none());
        }
        assert!(!relative("../hidden.png"));
        assert!(!relative("C:/hidden.png"));
    }
    #[test]
    fn whole_resource_does_not_invent_media_geometry() {
        let metadata = crate::media_api::ResourceMetadata::Unsupported {
            description: ErrorDescription::new(
                UiMessage::error_api_hosted_media_original_retained(),
            ),
        };
        let target = crate::knowledge_api::target_from_resource_metadata(
            &json!({"kind":"whole_resource"}),
            "weftext-resource://known/resource",
            "exact-version",
            &metadata,
        )
        .unwrap();
        assert!(matches!(
            target,
            knowledge::AnnotationTarget::WholeResource { .. }
        ));
        assert!(
            crate::knowledge_api::target_from_resource_metadata(
                &json!({"kind":"image_region","x":0,"y":0,"rectWidth":1,"rectHeight":1}),
                "weftext-resource://known/resource",
                "exact-version",
                &metadata
            )
            .is_err()
        );
    }
    #[cfg(windows)]
    pub(in crate::hosted) fn fixture() -> (tempfile::TempDir, HostedState, Auth, String) {
        let (temp, state, auth, _, _, _) = history_api::test_fixture();
        let mut bytes = std::io::Cursor::new(vec![]);
        image::DynamicImage::new_rgba8(2, 2)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let source = root.join("Shared/Shared.typ");
        let owner = inner.core.qualified_node_at(&root, &source).unwrap().owner;
        let op = uuid::Uuid::new_v4().to_string();
        let plan = inner
            .core
            .prepare_import_resource(
                &root,
                &source,
                "private-locator.png",
                bytes.into_inner(),
                &op,
            )
            .unwrap();
        let reference = plan.resource_map.values().next().unwrap().clone();
        inner.core.execute_operation(&op, &plan.plan_id).unwrap();
        inner.config.resources.insert(
            "Diagram".into(),
            Resource {
                owner: "Shared".into(),
                expected_owner: owner,
                path: "Shared/private-locator.png".into(),
                title: "批准显示标题".into(),
                reference,
                lifecycle: "live".into(),
            },
        );
        let alice = inner.policy.users.get_mut("alice").unwrap();
        alice.resources.insert(
            "Diagram".into(),
            Rights {
                read: true,
                download: false,
            },
        );
        alice.grants.insert(
            "Shared".into(),
            super::super::Rights {
                annotations: review::ReviewRights {
                    read: true,
                    write: true,
                    target: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let revision = capture_input(&inner, "alice", "Diagram")
            .unwrap()
            .revision
            .unwrap();
        drop(inner);
        (temp, state, auth, revision)
    }
    #[cfg(windows)]
    fn collection_alias(state: &HostedState, download: bool) -> String {
        let mut inner = state.inner.lock().unwrap();
        let r = inner.config.resources["Diagram"].clone();
        let asset = format!("Owned/{}", r.reference.rsplit('/').next().unwrap());
        inner.config.resource_collections.insert(
            "Owned".into(),
            resource_collections::Collection {
                owner: r.owner,
                expected_owner: Some(r.expected_owner),
                from_import_slot: false,
                from_structure_slot: false,
                lifecycle: r.lifecycle,
                title: "公开集合".into(),
            },
        );
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resource_collections
            .insert(
                "Owned".into(),
                resource_collections::Rights {
                    read: true,
                    download,
                    r#import: false,
                },
            );
        asset
    }
    #[cfg(windows)]
    fn catalog_fixture() -> (tempfile::TempDir, HostedState, Auth) {
        let (temp, state, auth, _) = fixture();
        collection_alias(&state, false);
        let mut inner = state.inner.lock().unwrap();
        let root = inner.config.root.clone();
        let operation = uuid::Uuid::new_v4().to_string();
        let preview = inner
            .core
            .prepare_import_resources(
                &root,
                root.join("Shared/Shared.typ"),
                vec![
                    weftext_core::ResourceImportUpload {
                        name: "second.bin".into(),
                        bytes: b"second original".to_vec(),
                    },
                    weftext_core::ResourceImportUpload {
                        name: "third.bin".into(),
                        bytes: b"third original".to_vec(),
                    },
                ],
                &operation,
            )
            .unwrap();
        inner
            .core
            .execute_operation(&operation, &preview.plan_id)
            .unwrap();
        inner
            .policy
            .users
            .get_mut("alice")
            .unwrap()
            .resources
            .clear();
        drop(inner);
        (temp, state, auth)
    }
    #[cfg(windows)]
    #[test]
    fn catalog_reuses_one_collection_capture_and_final_check_for_three_exact_members() {
        let (_temp, state, _auth) = catalog_fixture();
        let inner = state.inner.lock().unwrap();
        let mut captures = 0;
        let cut = CatalogCut::capture_with(&inner, "alice", |inner, id| {
            captures += 1;
            resource_collections::qualify(inner, id).map(Arc::new)
        })
        .unwrap();
        assert_eq!(captures, 1);
        assert_eq!(cut.resources.len(), 3);
        let shared = &cut.cuts["Owned"];
        let mut owned = 0usize;
        for entry in &cut.resources {
            let CatalogResource::Ready(context) = entry else {
                panic!("current resource must qualify");
            };
            assert!(Arc::ptr_eq(context.collection.as_ref().unwrap(), shared));
            assert!(context.resource.revision.is_some());
            assert_eq!(
                context.resource.resource_ref,
                context.descriptor["resource"]
            );
            // Every ResourceWitness really owns its own owner/manifest/resource
            // cut. Only the shared collection Arc is deducted from this sum.
            owned += context.owned_retained_bytes();
        }
        let rows = cut
            .collections
            .iter()
            .map(|row| serde_json::to_vec(row).unwrap().len())
            .sum::<usize>();
        let bindings = cut.binding.len() + cut.bindings.iter().map(String::len).sum::<usize>();
        assert_eq!(
            cut.retained,
            shared.retained_bytes() + owned + rows + bindings
        );
        assert!(cut.retained <= MAX_BYTES);
        let mut final_checks = 0;
        let result = cut
            .finish_with(&inner, "alice", |inner, captured| {
                final_checks += 1;
                inner
                    .core
                    .validate_resource_collection_witness(&inner.config.root, &captured.witness)
                    .map_err(|_| changed())
            })
            .unwrap();
        assert_eq!(final_checks, 1);
        assert_eq!(result["coverage"], "complete");
        assert_eq!(result["resources"].as_array().unwrap().len(), 3);
        assert!(
            result["resources"]
                .as_array()
                .unwrap()
                .iter()
                .all(|row| row["status"] == "ready")
        );
    }
    #[cfg(windows)]
    #[test]
    fn catalog_final_collection_gate_removes_members_after_accepted_manifest_change() {
        let (_temp, state, _auth) = catalog_fixture();
        let mut inner = state.inner.lock().unwrap();
        let cut = CatalogCut::capture(&inner, "alice").unwrap();
        let root = inner.config.root.clone();
        let operation = uuid::Uuid::new_v4().to_string();
        let preview = inner
            .core
            .prepare_import_resource(
                &root,
                root.join("Shared/Shared.typ"),
                "late.bin",
                b"late".to_vec(),
                &operation,
            )
            .unwrap();
        inner
            .core
            .execute_operation(&operation, &preview.plan_id)
            .unwrap();
        let result = cut.finish(&inner, "alice").unwrap();
        assert_eq!(result["coverage"], "partial");
        assert_eq!(result["resources"], json!([]));
        assert_eq!(result["collections"][0]["status"], "unavailable");
        for hidden in [
            "private-locator",
            "second.bin",
            "third.bin",
            "late.bin",
            "weftext-resource://",
            "metadata",
        ] {
            assert!(!result.to_string().contains(hidden));
        }
    }
    #[cfg(windows)]
    #[test]
    fn catalog_final_resource_gate_withholds_changed_bytes_and_owner_replacement() {
        for replace_owner in [false, true] {
            let (_temp, state, _auth) = catalog_fixture();
            let inner = state.inner.lock().unwrap();
            let cut = CatalogCut::capture(&inner, "alice").unwrap();
            if replace_owner {
                let source = inner.config.root.join("Shared/Shared.typ");
                std::fs::rename(&source, source.with_extension("saved.typ")).unwrap();
                std::fs::write(&source, b"different physical owner").unwrap();
            } else {
                std::fs::write(
                    inner.config.root.join("Shared/second.bin"),
                    b"changed after capture",
                )
                .unwrap();
            }
            let result = cut.finish(&inner, "alice").unwrap();
            assert_eq!(result["coverage"], "partial");
            let rows = result["resources"].as_array().unwrap();
            if replace_owner {
                assert!(rows.is_empty());
                assert_eq!(result["collections"][0]["status"], "unavailable");
            } else {
                let changed = rows.iter().find(|row| row["name"] == "second.bin").unwrap();
                assert_eq!(changed["status"], "unavailable");
                for field in ["resource", "revision", "metadata", "contentType", "bytes"] {
                    assert!(changed.get(field).is_none());
                }
                assert_eq!(
                    rows.iter().filter(|row| row["status"] == "ready").count(),
                    2
                );
            }
        }
    }
    #[cfg(windows)]
    #[test]
    fn catalog_final_alias_read_check_withholds_revoked_members_and_keeps_import_only_summary() {
        for import_only in [false, true] {
            let (_temp, state, _auth) = catalog_fixture();
            let mut inner = state.inner.lock().unwrap();
            let cut = CatalogCut::capture(&inner, "alice").unwrap();
            let rights = inner
                .policy
                .users
                .get_mut("alice")
                .unwrap()
                .resource_collections
                .get_mut("Owned")
                .unwrap();
            rights.read = false;
            rights.r#import = import_only;
            let result = cut.finish(&inner, "alice").unwrap();
            assert_eq!(result["resources"], json!([]));
            let collections = result["collections"].as_array().unwrap();
            if import_only {
                assert_eq!(collections.len(), 1);
                assert_eq!(collections[0]["read"], false);
                assert_eq!(collections[0]["import"], true);
            } else {
                assert!(collections.is_empty());
            }
            for hidden in [
                "private-locator",
                "second.bin",
                "third.bin",
                "weftext-resource://",
                "metadata",
            ] {
                assert!(!result.to_string().contains(hidden));
            }
        }
    }
    #[cfg(windows)]
    #[test]
    fn catalog_final_binding_check_withholds_changed_configured_alias() {
        let (_temp, state, _auth, _revision) = fixture();
        let mut inner = state.inner.lock().unwrap();
        let cut = CatalogCut::capture(&inner, "alice").unwrap();
        inner
            .config
            .resources
            .get_mut("Diagram")
            .unwrap()
            .expected_owner
            .node_id = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            cut.finish(&inner, "alice").unwrap_err().status,
            StatusCode::CONFLICT
        );
    }
    #[cfg(windows)]
    #[test]
    fn catalog_final_binding_rejects_valid_collection_owner_remap_with_old_owner_intact() {
        let (_temp, state, _auth) = catalog_fixture();
        let mut inner = state.inner.lock().unwrap();
        let cut = CatalogCut::capture(&inner, "alice").unwrap();
        let root = inner.config.root.clone();
        let owner = inner
            .core
            .qualified_node_at(&root, &root.join("Target/Target.typ"))
            .unwrap()
            .owner;
        let collection = inner.config.resource_collections.get_mut("Owned").unwrap();
        collection.owner = "Target".into();
        collection.expected_owner = Some(owner);
        assert!(
            inner
                .core
                .qualified_node_at(&root, &root.join("Shared/Shared.typ"))
                .is_ok()
        );
        assert_eq!(
            cut.finish(&inner, "alice").unwrap_err().status,
            StatusCode::CONFLICT
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn collection_alias_preserves_stable_ref_and_each_alias_download_grant() {
        let (_temp, state, auth, revision) = fixture();
        let asset = collection_alias(&state, true);
        let reference = state.inner.lock().unwrap().config.resources["Diagram"]
            .reference
            .clone();
        let selected = resolve(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Resolve {
                reference,
                revision: Some(revision.clone()),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(selected["asset"], "Diagram");
        assert_eq!(selected["download"], false);
        assert_eq!(
            prepare_download(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(Read {
                    asset: "Diagram".into(),
                    revision: revision.clone()
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::NOT_FOUND
        );
        let allowed = prepare_download(
            State(state.clone()),
            Extension(auth),
            crate::RequestJson(Read { asset, revision }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(allowed["collection"], "Owned");
        assert_eq!(allowed["purpose"], "download");
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn collection_ticket_keeps_original_membership_and_import_only_catalog_hides_members() {
        let (_temp, state, auth, revision) = fixture();
        let asset = collection_alias(&state, false);
        let delivered = read(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Read { asset, revision }),
        )
        .await
        .unwrap()
        .0;
        let ticket = delivered["ticket"].as_str().unwrap().to_owned();
        {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let op = uuid::Uuid::new_v4().to_string();
            let plan = inner
                .core
                .prepare_import_resource(
                    &root,
                    root.join("Shared/Shared.typ"),
                    "later.bin",
                    b"later".to_vec(),
                    &op,
                )
                .unwrap();
            inner.core.execute_operation(&op, &plan.plan_id).unwrap();
        }
        assert_eq!(
            view_chunk(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(ChunkRequest { ticket, offset: 0 })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::GONE
        );
        {
            let mut inner = state.inner.lock().unwrap();
            let alice = inner.policy.users.get_mut("alice").unwrap();
            alice.resources.clear();
            alice.resource_collections.get_mut("Owned").unwrap().read = false;
            alice
                .resource_collections
                .get_mut("Owned")
                .unwrap()
                .r#import = true;
        }
        let result = catalog(State(state), Extension(auth)).await.unwrap().0;
        assert!(result["resources"].as_array().unwrap().is_empty());
        assert_eq!(result["collections"][0]["import"], true);
        assert!(!result.to_string().contains("later.bin"));
        assert!(!result.to_string().contains("private-locator"));
    }
    #[cfg(windows)]
    async fn open_view(state: &HostedState, auth: &Auth, revision: &str) -> Value {
        read(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Read {
                asset: "Diagram".into(),
                revision: revision.into(),
            }),
        )
        .await
        .unwrap()
        .0
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn view_does_not_grant_source_or_download_and_ticket_purpose_is_fixed() {
        let (_temp, state, auth, revision) = fixture();
        let descriptor = open_view(&state, &auth, &revision).await;
        assert_eq!(descriptor["name"], "批准显示标题");
        assert_eq!(descriptor["owner"], "Shared");
        assert!(!descriptor.to_string().contains("private-locator"));
        assert!(!descriptor.to_string().contains("Saved B"));
        assert!(
            state
                .inner
                .lock()
                .unwrap()
                .rights("alice", "Shared")
                .is_err()
        );
        let denied = prepare_download(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Read {
                asset: "Diagram".into(),
                revision: revision.clone(),
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(denied.status, StatusCode::NOT_FOUND);
        let ticket = descriptor["ticket"].as_str().unwrap().to_owned();
        assert_eq!(
            download_chunk(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(ChunkRequest {
                    ticket: ticket.clone(),
                    offset: 0
                })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::GONE
        );
        let chunk = view_chunk(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(ChunkRequest {
                ticket: ticket.clone(),
                offset: 0,
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(chunk["done"], true);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(chunk["bytesBase64"].as_str().unwrap())
            .unwrap();
        assert_eq!(digest(&bytes), descriptor["sha256"]);
        let bob = Auth {
            digest: digest(b"bob"),
        };
        assert_eq!(
            view_chunk(
                State(state),
                Extension(bob),
                crate::RequestJson(ChunkRequest { ticket, offset: 0 })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::GONE
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn visible_scope_does_not_read_locked_sibling_or_expose_its_name() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_temp, state, auth, _revision) = fixture();
        let hidden = {
            let mut inner = state.inner.lock().unwrap();
            let root = inner.config.root.clone();
            let source = root.join("Shared/Shared.typ");
            let op = uuid::Uuid::new_v4().to_string();
            let plan = inner
                .core
                .prepare_import_resource(
                    &root,
                    &source,
                    "HIDDEN_SIBLING.bin",
                    b"hidden bytes".to_vec(),
                    &op,
                )
                .unwrap();
            inner.core.execute_operation(&op, &plan.plan_id).unwrap();
            root.join("Shared/HIDDEN_SIBLING.bin")
        };
        let _locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(hidden)
            .unwrap();
        let result = catalog(State(state), Extension(auth)).await.unwrap().0;
        assert_eq!(result["coverage"], "complete");
        assert_eq!(result["resources"].as_array().unwrap().len(), 1);
        assert!(!result.to_string().contains("HIDDEN_SIBLING"));
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn changed_configuration_or_regrant_cannot_revive_original_delivery() {
        let (_temp, state, auth, revision) = fixture();
        let descriptor = open_view(&state, &auth, &revision).await;
        {
            let mut inner = state.inner.lock().unwrap();
            inner.config.resources.get_mut("Diagram").unwrap().title = "另一批准显示".into();
        }
        let ticket = descriptor["ticket"].as_str().unwrap().to_owned();
        assert_eq!(
            view_chunk(
                State(state.clone()),
                Extension(auth.clone()),
                crate::RequestJson(ChunkRequest { ticket, offset: 0 })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::GONE
        );
        let fresh = open_view(&state, &auth, &revision).await;
        let ticket = fresh["ticket"].as_str().unwrap().to_owned();
        {
            let mut inner = state.inner.lock().unwrap();
            let user = inner.policy.users.get_mut("alice").unwrap();
            user.epoch += 1;
            user.resources.clear();
            expire_revoked(&mut inner);
            let user = inner.policy.users.get_mut("alice").unwrap();
            user.epoch += 1;
            user.resources.insert(
                "Diagram".into(),
                Rights {
                    read: true,
                    download: true,
                },
            );
        }
        assert_eq!(
            view_chunk(
                State(state),
                Extension(auth),
                crate::RequestJson(ChunkRequest { ticket, offset: 0 })
            )
            .await
            .unwrap_err()
            .status,
            StatusCode::GONE
        );
    }
    #[cfg(windows)]
    #[tokio::test]
    async fn stable_resource_link_is_an_independent_viewer_not_a_source_open() {
        let (_temp, state, auth, revision) = fixture();
        let reference = state.inner.lock().unwrap().config.resources["Diagram"]
            .reference
            .clone();
        let descriptor = resolve(
            State(state.clone()),
            Extension(auth.clone()),
            crate::RequestJson(Resolve {
                reference: reference.clone(),
                revision: Some(revision),
            }),
        )
        .await
        .unwrap()
        .0;
        assert_eq!(descriptor["viewerTarget"]["resource"], reference);
        assert_eq!(descriptor["asset"], "Diagram");
        let hidden = resolve(
            State(state.clone()),
            Extension(Auth {
                digest: digest(b"bob"),
            }),
            crate::RequestJson(Resolve {
                reference,
                revision: None,
            }),
        )
        .await
        .unwrap_err();
        let missing = resolve(
            State(state),
            Extension(auth),
            crate::RequestJson(Resolve {
                reference: format!(
                    "weftext-resource://{}/{}",
                    uuid::Uuid::new_v4(),
                    uuid::Uuid::new_v4()
                ),
                revision: None,
            }),
        )
        .await
        .unwrap_err();
        assert_eq!(hidden.status, missing.status);
        assert_eq!(hidden.code, missing.code);
        assert_eq!(hidden.explanation, missing.explanation);
        assert_eq!(hidden.diagnostics, missing.diagnostics);
        assert_eq!(hidden.causes, missing.causes);
    }
}
