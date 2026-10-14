//! Explicit compilation inputs. A source grant never supplies sibling assets.
use super::*;
use weftext_core::{
    MissingResourceWitness, QualifiedResource, QualifiedResourceCollection, QualifiedResourceInput,
    QueryNodeWitness, SnapshotWitness,
};

const MAX_INPUTS: usize = 64;
const MAX_BYTES: usize = 32 * 1024 * 1024;

fn changed() -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "staleEnvironment",
        "获权输入或准确资源版本已变化，请保留输入并重新核对。",
    )
}
fn budget() -> ApiError {
    ApiError::bad("本次输入及其原始依据超出 32 MiB 或 64 个输入的预算")
}
pub(super) fn path(inner: &HostedInner, actor: &str, id: &str) -> Result<String, ApiError> {
    if let Ok(file) = inner.file(id) {
        return Ok(file.path.clone());
    }
    Ok(media::resolve_asset(inner, actor, id)?.path)
}
pub(super) fn access(inner: &HostedInner, actor: &str, id: &str) -> Result<(), ApiError> {
    if media::valid_input(&inner.config, id) {
        media::input_access(inner, actor, id, false)
    } else {
        inner.rights(actor, id).map(|_| ())
    }
}
/// Includes the asset's configured owner, without publishing physical paths.
pub(super) fn binding_hash(
    inner: &HostedInner,
    ids: &BTreeSet<String>,
) -> Result<String, ApiError> {
    let mut files = BTreeMap::new();
    let mut assets = BTreeSet::new();
    for id in ids {
        if let Ok(file) = inner.file(id) {
            files.insert(id, file);
        } else if media::valid_input(&inner.config, id) {
            assets.insert(id.clone());
        } else {
            return Err(unavailable());
        }
    }
    Ok(digest(&serde_json::to_vec(&json!({"root":inner.config.root,"files":files,"assets":media::binding_hash(inner,&assets)?})).map_err(|_|changed())?))
}
/// Fresh read used by existing digest checks. Original retained evidence is
/// separately rechecked before delivery or merged into the original Plan.
pub(super) fn read_files(
    inner: &HostedInner,
    actor: &str,
    ids: &[String],
) -> Result<BTreeMap<String, Vec<u8>>, ApiError> {
    let (ids, collections) = resource_collections::expand_inputs(inner, actor, ids)?;
    if ids.len() > MAX_INPUTS {
        return Err(budget());
    }
    for id in &ids {
        access(inner, actor, id)?;
    }
    let paths = ids
        .iter()
        .filter_map(|id| inner.file(id).ok().map(|f| f.path.clone()))
        .collect::<BTreeSet<_>>();
    let mut files = inner
        .core
        .hosted_files(&inner.config.root, &paths.into_iter().collect::<Vec<_>>())
        .map_err(|_| unavailable())?;
    let mut bytes = files.values().map(Vec::len).sum::<usize>()
        + collections
            .iter()
            .map(|(_, c)| c.retained_bytes())
            .sum::<usize>();
    if bytes > MAX_BYTES {
        return Err(budget());
    }
    for id in ids.iter().filter(|id| media::is_asset(&inner.config, id)) {
        let resource = media::capture_input(inner, actor, id)?;
        bytes = bytes
            .checked_add(resource.bytes().len())
            .ok_or_else(budget)?;
        if bytes > MAX_BYTES {
            return Err(budget());
        }
        let path = path(inner, actor, id)?;
        if files
            .get(&path)
            .is_some_and(|old| old.as_slice() != resource.bytes())
        {
            return Err(changed());
        }
        files.insert(path, resource.bytes().to_vec());
    }
    for (_, collection) in collections {
        inner
            .core
            .validate_resource_collection_witness(&inner.config.root, &collection.witness)
            .map_err(|_| changed())?;
    }
    Ok(files)
}

enum Evidence {
    File {
        witness: SnapshotWitness,
        node: Option<Box<QueryNodeWitness>>,
    },
    Resource(Box<QualifiedResource>),
    MissingResource(Box<MissingResourceWitness>),
}
struct Entry {
    path: String,
    bytes: Option<Vec<u8>>,
    accepted: bool,
    evidence: Evidence,
}
#[derive(Clone)]
pub(super) struct Captured {
    ids: BTreeSet<String>,
    selections: BTreeSet<String>,
    collections: BTreeMap<String, Arc<QualifiedResourceCollection>>,
    binding: String,
    entries: Arc<BTreeMap<String, Entry>>,
    bytes: usize,
}
type InputView<'a> = (&'a str, &'a [u8], bool);
impl Captured {
    pub(super) fn ids(&self) -> &BTreeSet<String> {
        &self.ids
    }
    pub(super) fn authority_ids(&self) -> BTreeSet<String> {
        self.ids.union(&self.selections).cloned().collect()
    }
    pub(super) fn matches_selection(&self, selections: &[String]) -> bool {
        selections.iter().cloned().collect::<BTreeSet<_>>() == self.selections
    }
    pub(super) fn members_for(&self, selections: &[String]) -> Result<BTreeSet<String>, ApiError> {
        let mut ids = BTreeSet::new();
        for selected in selections {
            if !self.selections.contains(selected) {
                return Err(changed());
            }
            if let Some(collection) = selected.strip_suffix("/*") {
                if !self.collections.contains_key(collection) {
                    return Err(changed());
                }
                let prefix = format!("{collection}/");
                ids.extend(
                    self.ids
                        .iter()
                        .filter(|id| id.starts_with(&prefix))
                        .cloned(),
                );
            } else {
                if !self.ids.contains(selected) {
                    return Err(changed());
                }
                ids.insert(selected.clone());
            }
        }
        Ok(ids)
    }
    pub(super) fn collection_revisions(&self) -> BTreeMap<String, Option<String>> {
        self.collections
            .iter()
            .map(|(id, c)| (id.clone(), c.revision.clone()))
            .collect()
    }
    pub(super) fn retained_bytes(&self) -> usize {
        self.bytes
    }
    pub(super) fn entry(&self, id: &str) -> Result<Option<InputView<'_>>, ApiError> {
        self.entries
            .get(id)
            .map(|e| {
                e.bytes
                    .as_ref()
                    .map(|bytes| (e.path.as_str(), bytes.as_slice(), e.accepted))
            })
            .ok_or_else(changed)
    }
    pub(super) fn files(&self) -> Result<BTreeMap<String, Vec<u8>>, ApiError> {
        let mut files = BTreeMap::new();
        for e in self.entries.values() {
            let bytes = e.bytes.as_ref().ok_or_else(changed)?;
            if files.get(&e.path).is_some_and(|old: &Vec<u8>| old != bytes) {
                return Err(changed());
            }
            files.insert(e.path.clone(), bytes.clone());
        }
        Ok(files)
    }
    pub(super) fn revisions(&self) -> BTreeMap<String, Option<String>> {
        self.entries
            .iter()
            .filter_map(|(id, e)| match &e.evidence {
                Evidence::Resource(r) => Some((id.clone(), r.revision.clone())),
                _ => None,
            })
            .collect()
    }
    pub(super) fn validate(&self, inner: &HostedInner, actor: &str) -> Result<(), ApiError> {
        let authority = self.authority_ids();
        for id in &authority {
            access(inner, actor, id)?;
        }
        if binding_hash(inner, &authority)? != self.binding {
            return Err(changed());
        }
        for collection in self.collections.values() {
            inner
                .core
                .validate_resource_collection_witness(&inner.config.root, &collection.witness)
                .map_err(|_| changed())?;
        }
        for e in self.entries.values() {
            match &e.evidence {
                Evidence::File { witness, node } => {
                    inner
                        .core
                        .validate_snapshot_witness(
                            &inner.config.root,
                            inner.config.root.join(&e.path),
                            witness,
                        )
                        .map_err(|_| changed())?;
                    if let Some(node) = node {
                        inner
                            .core
                            .validate_query_node_witness(&inner.config.root, node)
                            .map_err(|_| changed())?;
                    }
                }
                Evidence::Resource(resource) => inner
                    .core
                    .validate_resource_witness(&inner.config.root, &resource.witness)
                    .map_err(|_| changed())?,
                Evidence::MissingResource(witness) => inner
                    .core
                    .validate_missing_resource_witness(&inner.config.root, witness)
                    .map_err(|_| changed())?,
            }
        }
        Ok(())
    }
    pub(super) fn plan_witnesses(&self) -> Result<Vec<SnapshotWitness>, ApiError> {
        let mut result = vec![];
        for collection in self.collections.values() {
            Core::merge_resource_collection_witness(&mut result, &collection.witness)
                .map_err(|_| changed())?;
        }
        for e in self.entries.values() {
            // Missing inputs are read-only Query evidence, never a write basis.
            if e.bytes.is_none() {
                return Err(changed());
            }
            match &e.evidence {
                Evidence::File { witness, node } => {
                    Core::merge_snapshot_witnesses(&mut result, vec![witness.clone()])
                        .map_err(|_| changed())?;
                    if let Some(node) = node {
                        Core::merge_query_node_witness(&mut result, node).map_err(|_| changed())?;
                    }
                }
                Evidence::Resource(resource) => {
                    Core::merge_resource_witness(&mut result, &resource.witness)
                        .map_err(|_| changed())?
                }
                Evidence::MissingResource(_) => return Err(changed()),
            }
        }
        Ok(result)
    }
}
pub(super) fn capture(
    inner: &mut HostedInner,
    actor: &str,
    ids: &[String],
) -> Result<Captured, ApiError> {
    capture_inputs(inner, actor, ids, false)
}
pub(super) fn capture_query(
    inner: &mut HostedInner,
    actor: &str,
    ids: &[String],
) -> Result<Captured, ApiError> {
    capture_inputs(inner, actor, ids, true)
}
fn capture_inputs(
    inner: &mut HostedInner,
    actor: &str,
    ids: &[String],
    allow_missing: bool,
) -> Result<Captured, ApiError> {
    let selections = ids.iter().cloned().collect::<BTreeSet<_>>();
    let (expanded, collections) = resource_collections::expand_inputs(inner, actor, ids)?;
    let ids = expanded.into_iter().collect::<BTreeSet<_>>();
    let collections = collections.into_iter().collect::<BTreeMap<_, _>>();
    if ids.len() > MAX_INPUTS {
        return Err(budget());
    }
    for id in &ids {
        access(inner, actor, id)?;
    }
    let authority = ids.union(&selections).cloned().collect();
    let binding = binding_hash(inner, &authority)?;
    let mut entries = BTreeMap::new();
    let mut bytes = binding.len() + selections.iter().map(String::len).sum::<usize>();
    for (id, c) in &collections {
        bytes = bytes
            .saturating_add(id.len())
            .saturating_add(c.retained_bytes());
        if bytes > MAX_BYTES {
            return Err(budget());
        }
    }
    for id in &ids {
        let entry = if media::is_asset(&inner.config, id) {
            let qualified = if allow_missing {
                let r = media::resolve_asset(inner, actor, id)?;
                let owner = &inner.file(&r.owner)?;
                inner
                    .core
                    .qualified_resource_input_at(
                        &inner.config.root,
                        &inner.config.root.join(&owner.path),
                        &r.expected_owner,
                        &r.reference,
                        &inner.config.root.join(&r.path),
                        &r.lifecycle,
                    )
                    .map_err(|_| changed())?
            } else {
                QualifiedResourceInput::Present(Box::new(media::capture_input(inner, actor, id)?))
            };
            let path = path(inner, actor, id)?;
            match qualified {
                QualifiedResourceInput::Present(resource) => {
                    bytes = bytes.saturating_add(resource.witness.retained_bytes());
                    if bytes
                        .saturating_add(id.len())
                        .saturating_add(path.len())
                        .saturating_add(resource.bytes().len())
                        > MAX_BYTES
                    {
                        return Err(budget());
                    }
                    let content = resource.bytes().to_vec();
                    Entry {
                        path,
                        bytes: Some(content),
                        accepted: resource.revision.is_some(),
                        evidence: Evidence::Resource(resource),
                    }
                }
                QualifiedResourceInput::Missing(witness) => {
                    bytes = bytes.saturating_add(witness.retained_bytes());
                    Entry {
                        path,
                        bytes: None,
                        accepted: false,
                        evidence: Evidence::MissingResource(witness),
                    }
                }
            }
        } else {
            let file = inner.file(id).ok().ok_or_else(unavailable)?.clone();
            let absolute = inner.config.root.join(&file.path);
            let node = file
                .expected_node
                .as_ref()
                .map(|owner| {
                    inner.core.qualified_query_node_at(
                        &inner.config.root,
                        &absolute,
                        owner,
                        &file.lifecycle,
                    )
                })
                .transpose()
                .map_err(|_| changed())?
                .map(|node| Box::new(node.witness));
            let snapshot = inner
                .core
                .read_snapshot(
                    &inner.config.root,
                    &[absolute.to_string_lossy().into_owned()],
                )
                .map_err(|_| changed())?
                .files
                .remove(0);
            let witness = inner
                .core
                .take_snapshot_witness(&snapshot.base_id)
                .map_err(|_| changed())?;
            let content = snapshot.bytes;
            if content.is_none() && !allow_missing {
                return Err(changed());
            }
            bytes = bytes
                .saturating_add(witness.retained_bytes())
                .saturating_add(node.as_ref().map_or(0, |n| n.retained_bytes()));
            Entry {
                path: file.path,
                bytes: content,
                accepted: snapshot.currentness == "accepted",
                evidence: Evidence::File { witness, node },
            }
        };
        bytes = bytes
            .saturating_add(id.len())
            .saturating_add(entry.path.len())
            .saturating_add(entry.bytes.as_ref().map_or(0, Vec::len));
        if bytes > MAX_BYTES {
            return Err(budget());
        }
        entries.insert(id.clone(), entry);
    }
    let captured = Captured {
        ids,
        selections,
        collections,
        binding,
        entries: Arc::new(entries),
        bytes,
    };
    captured.validate(inner, actor)?;
    Ok(captured)
}
