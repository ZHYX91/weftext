//! Explicit compilation inputs. A source grant never supplies sibling assets.
use super::*;
use weftext_core::{
    MissingResourceWitness, QualifiedResource, QualifiedResourceInput, QueryNodeWitness,
    SnapshotWitness,
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
pub(super) fn path<'a>(inner: &'a HostedInner, id: &str) -> Result<&'a str, ApiError> {
    inner
        .config
        .files
        .get(id)
        .map(|f| f.path.as_str())
        .or_else(|| inner.config.resources.get(id).map(|r| r.path.as_str()))
        .ok_or_else(unavailable)
}
pub(super) fn access(inner: &HostedInner, actor: &str, id: &str) -> Result<(), ApiError> {
    if inner.config.resources.contains_key(id) {
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
        if let Some(file) = inner.config.files.get(id) {
            files.insert(id, file);
        } else if inner.config.resources.contains_key(id) {
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
    if ids.len() > MAX_INPUTS {
        return Err(budget());
    }
    for id in ids {
        access(inner, actor, id)?;
    }
    let paths = ids
        .iter()
        .filter_map(|id| inner.config.files.get(id).map(|f| f.path.clone()))
        .collect::<BTreeSet<_>>();
    let mut files = inner
        .core
        .hosted_files(&inner.config.root, &paths.into_iter().collect::<Vec<_>>())
        .map_err(|_| unavailable())?;
    let mut bytes = files.values().map(Vec::len).sum::<usize>();
    if bytes > MAX_BYTES {
        return Err(budget());
    }
    for id in ids
        .iter()
        .filter(|id| inner.config.resources.contains_key(*id))
    {
        let resource = media::capture_input(inner, actor, id)?;
        bytes = bytes
            .checked_add(resource.bytes().len())
            .ok_or_else(budget)?;
        if bytes > MAX_BYTES {
            return Err(budget());
        }
        let path = path(inner, id)?.to_owned();
        if files
            .get(&path)
            .is_some_and(|old| old.as_slice() != resource.bytes())
        {
            return Err(changed());
        }
        files.insert(path, resource.bytes().to_vec());
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
    binding: String,
    entries: Arc<BTreeMap<String, Entry>>,
    bytes: usize,
}
type InputView<'a> = (&'a str, &'a [u8], bool);
impl Captured {
    pub(super) fn ids(&self) -> &BTreeSet<String> {
        &self.ids
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
        for id in &self.ids {
            access(inner, actor, id)?;
        }
        if binding_hash(inner, &self.ids)? != self.binding {
            return Err(changed());
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
    let ids = ids.iter().cloned().collect::<BTreeSet<_>>();
    if ids.len() > MAX_INPUTS {
        return Err(budget());
    }
    for id in &ids {
        access(inner, actor, id)?;
    }
    let binding = binding_hash(inner, &ids)?;
    let mut entries = BTreeMap::new();
    let mut bytes = binding.len();
    for id in &ids {
        let entry = if inner.config.resources.contains_key(id) {
            let qualified = if allow_missing {
                let r = &inner.config.resources[id];
                let owner = &inner.config.files[&r.owner];
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
            let path = path(inner, id)?.to_owned();
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
            let file = inner.config.files.get(id).ok_or_else(unavailable)?.clone();
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
        binding,
        entries: Arc::new(entries),
        bytes,
    };
    captured.validate(inner, actor)?;
    Ok(captured)
}
