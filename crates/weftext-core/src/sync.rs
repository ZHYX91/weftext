//! Same-owner portable content history. Admission is not installation: this
//! module never writes the author tree, grants hosted access, or copies jobs.
//! A receiving Core must separately qualify and journal the selected changes.
use super::{CoreError, Result, hash};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const MAX_COMMITS: usize = 2_000;
const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
mod store;
mod transport;
pub use store::{SyncBundle, SyncIdentity, SyncStatus};
pub use transport::SyncTransfer;
pub(crate) mod apply;
mod install;
mod relocate;
pub use apply::SyncResolution;
pub(crate) mod replica;
pub use replica::SyncBindingInfo;
const MAX_CHANGES: usize = 5_000;
const MAX_FILES: usize = 5_000;
const MAX_BLOB_BYTES: usize = 64 * 1024 * 1024;
const MAX_STORE_BYTES: usize = 256 * 1024 * 1024;
const MAX_BLOBS: usize = 20_000;

/// Only ordinary content roles can cross the replica boundary. In particular,
/// there is no job, approval, credential, actor policy, or private draft role.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ObjectKey {
    Node(String),
    Directory(String),
    RootDirectory(String),
    Schema,
    Settings,
    Tags,
}

/// A node is one conflict unit, including source, annotations and its owned
/// resources. Placement participates too: move/edit and trash/edit conflict.
/// File paths are relative to the object, not sender filesystem paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectState {
    pub parent: Option<ObjectKey>,
    pub name: String,
    pub retired: bool,
    pub files: BTreeMap<String, String>,
    /// Portable content revisions are distinct from local ReadBasis tokens and
    /// from the enclosing multi-file conflict-unit revision.
    pub file_revisions: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Change {
    pub object: ObjectKey,
    /// Every causal version of this object at the parent cut, including all
    /// conflict sides when explicitly resolving. Empty only for a true birth.
    pub before: BTreeSet<String>,
    pub revision: String,
    pub after: ObjectState,
}

/// A semantic prerequisite beyond the changed object, e.g. schema or parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Dependency {
    pub object: ObjectKey,
    pub revisions: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    pub version: u32,
    pub workspace_id: String,
    pub replica_id: String,
    pub operation_id: String,
    pub parents: BTreeSet<String>,
    pub changes: Vec<Change>,
    pub dependencies: Vec<Dependency>,
}

impl Manifest {
    /// Hash the entire typed, canonical payload. Unknown fields are rejected on
    /// decode; vector order is part of the payload, map/set order is canonical.
    pub fn id(&self) -> Result<String> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(invalid("Manifest exceeds 4 MiB"));
        }
        Ok(hash(&bytes))
    }

    fn validate(&self) -> Result<()> {
        if self.version != 1
            || (self.changes.is_empty() && !self.parents.is_empty())
            || self.changes.len() > MAX_CHANGES
            || self.dependencies.len() > MAX_CHANGES
            || self.parents.len() > MAX_COMMITS
        {
            return Err(invalid("Unsupported or oversized manifest"));
        }
        for id in [&self.workspace_id, &self.replica_id, &self.operation_id] {
            uuid(id)?;
        }
        for parent in &self.parents {
            digest(parent)?;
        }
        let mut objects = BTreeSet::new();
        let mut revisions = BTreeSet::new();
        let mut count = 0;
        for change in &self.changes {
            key(&change.object)?;
            uuid(&change.revision)?;
            if !objects.insert(&change.object)
                || !revisions.insert(&change.revision)
                || change.before.len() > MAX_COMMITS
                || change.before.contains(&change.revision)
            {
                return Err(invalid(
                    "Duplicate object/revision or invalid predecessor set",
                ));
            }
            for before in &change.before {
                uuid(before)?;
            }
            if !change
                .after
                .files
                .keys()
                .eq(change.after.file_revisions.keys())
                || change
                    .after
                    .file_revisions
                    .values()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != change.after.file_revisions.len()
            {
                return Err(invalid(
                    "Every portable file requires its exact distinct content revision",
                ));
            }
            for revision in change.after.file_revisions.values() {
                uuid(revision)?;
            }
            if let Some(parent) = &change.after.parent {
                key(parent)?;
                if !matches!(parent, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                    return Err(invalid(
                        "Placement parent must be a typed Node or Directory",
                    ));
                }
            }
            component(&change.after.name)?;
            count += change.after.files.len();
            if count > MAX_FILES || change.after.files.is_empty() {
                return Err(invalid(
                    "A content state must retain its exact bounded files",
                ));
            }
            let mut names = BTreeSet::new();
            for (path, blob) in &change.after.files {
                relative(path)?;
                if !names.insert(path.to_lowercase()) {
                    return Err(invalid("Case-colliding file names"));
                }
                digest(blob)?;
            }
            // Only these files carry workspace-level configuration; no private
            // state may be smuggled through an arbitrary config role.
            let config_file = match change.object {
                ObjectKey::Schema => Some(".weftext-schema.json"),
                ObjectKey::Settings => Some(".weftext-settings.json"),
                ObjectKey::Tags => Some(".weftext-tags.json"),
                ObjectKey::RootDirectory(_) => Some("weftext.directory.json"),
                _ => None,
            };
            if let Some(path) = config_file
                && (change.after.files.len() != 1
                    || !change.after.files.contains_key(path)
                    || change.after.parent.is_some())
            {
                return Err(invalid("Configuration must have its exact content role"));
            }
        }
        let mut dependencies = BTreeSet::new();
        for dependency in &self.dependencies {
            key(&dependency.object)?;
            if !dependencies.insert(&dependency.object)
                || dependency.revisions.is_empty()
                || dependency.revisions.len() > MAX_COMMITS
            {
                return Err(invalid("Invalid dependency cut"));
            }
            for revision in &dependency.revisions {
                uuid(revision)?;
            }
        }
        Ok(())
    }

    fn blobs(&self) -> BTreeSet<&String> {
        self.changes
            .iter()
            .flat_map(|c| c.after.files.values())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Admission {
    Accepted,
    Staged {
        missing_parents: BTreeSet<String>,
        missing_blobs: BTreeSet<String>,
    },
    Rejected {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Version {
    pub commit: String,
    pub revision: String,
    pub state: ObjectState,
}

/// All concurrent versions, plus causal common bases for an explicit merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectView {
    pub versions: Vec<Version>,
    pub common_bases: Vec<Version>,
}

/// A bounded in-memory verifier backed by the receiving Core's private store.
/// Staged bytes cannot become current by arriving last, or by matching a hash.
pub struct ReplicaGraph {
    workspace_id: String,
    genesis: String,
    manifests: BTreeMap<String, Manifest>,
    blobs: BTreeMap<String, Vec<u8>>,
    admitted: BTreeSet<String>,
    rejected: BTreeMap<String, String>,
    total_bytes: usize,
}

impl ReplicaGraph {
    pub fn new(workspace_id: &str, genesis: &str) -> Result<Self> {
        uuid(workspace_id)?;
        digest(genesis)?;
        Ok(Self {
            workspace_id: workspace_id.into(),
            genesis: genesis.into(),
            manifests: BTreeMap::new(),
            blobs: BTreeMap::new(),
            admitted: BTreeSet::new(),
            rejected: BTreeMap::new(),
            total_bytes: 0,
        })
    }

    pub fn offer_blob(&mut self, id: &str, bytes: Vec<u8>) -> Result<()> {
        digest(id)?;
        if bytes.len() > MAX_BLOB_BYTES || hash(&bytes) != id {
            return Err(invalid("Blob digest mismatch or size limit"));
        }
        if let Some(old) = self.blobs.get(id) {
            return if old == &bytes {
                Ok(())
            } else {
                Err(invalid("Blob collision"))
            };
        }
        if self.total_bytes + bytes.len() > MAX_STORE_BYTES || self.blobs.len() >= MAX_BLOBS {
            return Err(invalid("Replica content budget exceeded"));
        }
        self.total_bytes += bytes.len();
        self.blobs.insert(id.into(), bytes);
        self.advance();
        Ok(())
    }

    pub fn offer_manifest(&mut self, id: &str, manifest: Manifest) -> Result<Admission> {
        if manifest.workspace_id != self.workspace_id || manifest.id()? != id {
            return Err(invalid("Wrong workspace or manifest digest"));
        }
        if let Some(old) = self.manifests.get(id) {
            if old != &manifest {
                return Err(invalid("Manifest collision"));
            }
            return self.status(id);
        }
        if self.manifests.len() >= MAX_COMMITS {
            return Err(invalid("Replica history budget exceeded"));
        }
        let size = serde_json::to_vec(&manifest)?.len();
        if self.total_bytes + size > MAX_STORE_BYTES {
            return Err(invalid("Replica metadata/content budget exceeded"));
        }
        // Equivocation never silently replaces the original operation. This is
        // an integrity diagnostic, not a malicious-transport security claim.
        if self
            .manifests
            .values()
            .any(|m| m.replica_id == manifest.replica_id && m.operation_id == manifest.operation_id)
        {
            return Err(invalid(
                "Replica operation already has another exact manifest",
            ));
        }
        self.total_bytes += size;
        self.manifests.insert(id.into(), manifest);
        self.advance();
        self.status(id)
    }

    pub fn status(&self, id: &str) -> Result<Admission> {
        let manifest = self
            .manifests
            .get(id)
            .ok_or_else(|| invalid("Unknown manifest"))?;
        if self.admitted.contains(id) {
            return Ok(Admission::Accepted);
        }
        if let Some(reason) = self.rejected.get(id) {
            return Ok(Admission::Rejected {
                reason: reason.clone(),
            });
        }
        Ok(Admission::Staged {
            missing_parents: manifest
                .parents
                .difference(&self.admitted)
                .cloned()
                .collect(),
            missing_blobs: manifest
                .blobs()
                .into_iter()
                .filter(|id| !self.blobs.contains_key(*id))
                .cloned()
                .collect(),
        })
    }

    pub fn heads(&self) -> BTreeSet<String> {
        let parents: BTreeSet<_> = self
            .admitted
            .iter()
            .flat_map(|id| self.manifests[id].parents.iter().cloned())
            .collect();
        self.admitted.difference(&parents).cloned().collect()
    }

    pub fn manifest(&self, id: &str) -> Option<&Manifest> {
        self.manifests.get(id)
    }
    pub fn blob(&self, id: &str) -> Option<&[u8]> {
        self.blobs.get(id).map(Vec::as_slice)
    }

    pub fn objects(&self) -> Result<BTreeMap<ObjectKey, ObjectView>> {
        self.view_at(&self.heads())
    }

    /// Export the complete causal/transaction closure of selected objects,
    /// including every conflict branch. Unrelated histories need not travel.
    /// Producers should use actual dependency parents rather than making every
    /// edit depend on a global frontier; compound transactions remain indivisible.
    pub fn export_objects(&self, objects: &BTreeSet<ObjectKey>) -> Result<SyncBundle> {
        if objects.is_empty() || objects.len() > MAX_CHANGES {
            return Err(invalid("Select a bounded nonempty set of content objects"));
        }
        let view = self.objects()?;
        let mut cut = BTreeSet::new();
        for object in objects {
            let current = view
                .get(object)
                .ok_or_else(|| invalid("Selected object has no admitted content"))?;
            cut.extend(current.versions.iter().map(|v| v.commit.clone()));
        }
        let manifests: BTreeMap<_, _> = self
            .closure(&cut)
            .into_iter()
            .map(|id| {
                let manifest = self.manifests[&id].clone();
                (id, manifest)
            })
            .collect();
        let blobs = manifests
            .values()
            .flat_map(|m| m.blobs())
            .map(|id| (id.clone(), self.blobs[id].clone()))
            .collect();
        Ok(SyncBundle { manifests, blobs })
    }

    pub fn view_at(&self, cut: &BTreeSet<String>) -> Result<BTreeMap<ObjectKey, ObjectView>> {
        if !cut.is_subset(&self.admitted) {
            return Err(invalid("Cut includes unadmitted history"));
        }
        let closure = self.closure(cut);
        let mut candidates: BTreeMap<ObjectKey, Vec<Version>> = BTreeMap::new();
        for id in &closure {
            for change in &self.manifests[id].changes {
                candidates
                    .entry(change.object.clone())
                    .or_default()
                    .push(Version {
                        commit: id.clone(),
                        revision: change.revision.clone(),
                        state: change.after.clone(),
                    });
            }
        }
        Ok(candidates
            .into_iter()
            .map(|(object, versions)| {
                let current = self.maximal(&versions);
                let bases = if current.len() > 1 {
                    let common_commits = current
                        .iter()
                        .map(|v| self.closure(&self.manifests[&v.commit].parents))
                        .reduce(|a, b| a.intersection(&b).cloned().collect())
                        .unwrap_or_default();
                    let common: Vec<_> = versions
                        .iter()
                        .filter(|base| common_commits.contains(&base.commit))
                        .cloned()
                        .collect();
                    self.maximal(&common)
                } else {
                    Vec::new()
                };
                (
                    object,
                    ObjectView {
                        versions: current,
                        common_bases: bases,
                    },
                )
            })
            .collect())
    }

    /// An install consumer must also check cross-object semantic prerequisites.
    /// The returned map does not hide a conflicting object from unrelated ones.
    pub fn unsettled_dependencies(&self, id: &str) -> Result<Vec<ObjectKey>> {
        if !self.admitted.contains(id) {
            return Err(invalid("Manifest is not admitted"));
        }
        let current = self.objects()?;
        Ok(self.manifests[id]
            .dependencies
            .iter()
            .filter(|dependency| {
                // A transaction may read and then change the same object.
                // Its output fulfills that read prerequisite; comparing current
                // against its own preimage would incorrectly block every edit.
                let expected = self.manifests[id]
                    .changes
                    .iter()
                    .find(|c| c.object == dependency.object)
                    .map(|c| BTreeSet::from([c.revision.clone()]))
                    .unwrap_or_else(|| dependency.revisions.clone());
                current.get(&dependency.object).is_none_or(|v| {
                    v.versions
                        .iter()
                        .map(|v| v.revision.clone())
                        .collect::<BTreeSet<_>>()
                        != expected
                })
            })
            .map(|d| d.object.clone())
            .collect())
    }

    fn advance(&mut self) {
        loop {
            let ready: Vec<_> = self
                .manifests
                .iter()
                .filter(|(id, m)| {
                    !self.admitted.contains(*id)
                        && !self.rejected.contains_key(*id)
                        && m.parents.is_subset(&self.admitted)
                        && m.blobs().iter().all(|b| self.blobs.contains_key(*b))
                })
                .map(|(id, _)| id.clone())
                .collect();
            if ready.is_empty() {
                break;
            }
            for id in ready {
                match self.validate_causality(&id) {
                    Ok(()) => {
                        self.admitted.insert(id);
                    }
                    Err(e) => {
                        self.rejected.insert(id, e.message);
                    }
                }
            }
        }
    }

    fn validate_causality(&self, id: &str) -> Result<()> {
        let manifest = &self.manifests[id];
        let prior = self.view_at(&manifest.parents)?;
        // Pairing pins the exact genesis. Arrival order cannot select an
        // unrelated root, even if someone copied the WorkspaceId string.
        if manifest.parents.is_empty() && id != self.genesis {
            return Err(invalid("Unrelated genesis for this workspace"));
        }
        let existing: BTreeSet<_> = self
            .admitted
            .iter()
            .flat_map(|id| self.manifests[id].changes.iter().map(|c| &c.revision))
            .collect();
        for change in &manifest.changes {
            let before: BTreeSet<_> = prior
                .get(&change.object)
                .into_iter()
                .flat_map(|v| v.versions.iter().map(|v| v.revision.clone()))
                .collect();
            if before != change.before || existing.contains(&change.revision) {
                return Err(invalid(
                    "Predecessors do not match the complete parent cut or revision was reused",
                ));
            }
            // A retirement is a version, not absence. It retains the original
            // content, so missing objects cannot silently masquerade as deletes.
            if change.before.is_empty() && change.after.retired {
                return Err(invalid("Cannot retire an object that was never born"));
            }
        }
        // Repeated immutable content can keep its revision through a move or
        // merge. A revision may never be reassigned to other bytes or an owner.
        let mut file_versions = BTreeMap::new();
        for prior in self
            .admitted
            .iter()
            .map(|id| &self.manifests[id])
            .chain(std::iter::once(manifest))
        {
            for change in &prior.changes {
                for (path, revision) in &change.after.file_revisions {
                    let binding = (&change.object, &change.after.files[path]);
                    if file_versions
                        .insert(revision, binding)
                        .is_some_and(|old| old != binding)
                    {
                        return Err(invalid(
                            "A portable file revision was reused for other bytes or owner",
                        ));
                    }
                }
            }
        }
        for dependency in &manifest.dependencies {
            let actual: BTreeSet<_> = prior
                .get(&dependency.object)
                .into_iter()
                .flat_map(|v| v.versions.iter().map(|v| v.revision.clone()))
                .collect();
            if actual != dependency.revisions {
                return Err(invalid("Dependency does not match the parent cut"));
            }
        }
        Ok(())
    }

    fn closure(&self, cut: &BTreeSet<String>) -> BTreeSet<String> {
        let mut seen = BTreeSet::new();
        let mut queue: Vec<_> = cut.iter().cloned().collect();
        while let Some(id) = queue.pop() {
            if seen.insert(id.clone()) {
                queue.extend(self.manifests[&id].parents.iter().cloned());
            }
        }
        seen
    }
    fn maximal(&self, candidates: &[Version]) -> Vec<Version> {
        // Every admitted predecessor set was checked against its entire parent
        // cut. Following those edges avoids repeated whole-DAG traversals for
        // every pair in a long offline history.
        let superseded: BTreeSet<_> = candidates
            .iter()
            .flat_map(|v| {
                self.manifests[&v.commit]
                    .changes
                    .iter()
                    .flat_map(|c| c.before.iter())
            })
            .collect();
        candidates
            .iter()
            .filter(|v| !superseded.contains(&v.revision))
            .cloned()
            .collect()
    }
}

fn invalid(message: &str) -> CoreError {
    CoreError::new("invalidSync", message)
}
fn uuid(id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(id).is_ok_and(|u| u.to_string() == id) {
        Ok(())
    } else {
        Err(invalid("Expected a canonical UUID"))
    }
}
fn digest(id: &str) -> Result<()> {
    if id.len() == 64
        && id
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        Ok(())
    } else {
        Err(invalid("Expected a SHA-256 digest"))
    }
}
fn key(key: &ObjectKey) -> Result<()> {
    match key {
        ObjectKey::Node(id) | ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) => uuid(id),
        _ => Ok(()),
    }
}
fn component(name: &str) -> Result<()> {
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if name.is_empty()
        || name.len() > 240
        || name.ends_with(['.', ' '])
        || name
            .chars()
            .any(|c| c.is_control() || "\\/:*?\"<>|".contains(c))
        || matches!(name, "." | "..")
        || reserved
    {
        return Err(invalid("Unsafe portable path component"));
    }
    Ok(())
}
fn relative(path: &str) -> Result<()> {
    if path.len() > 4096 {
        return Err(invalid("Portable path is too long"));
    }
    for part in path.split('/') {
        component(part)?;
    }
    // These are always private/runtime identities, never author content.
    if path.split('/').any(|p| {
        matches!(
            p.to_ascii_lowercase().as_str(),
            ".git"
                | ".weftext-workspace.json"
                | ".weftext-mvp-commit.json"
                | ".weftext-format"
                | "instance.lock"
                | "hosted-policy.json"
        )
    }) {
        return Err(invalid("Runtime control file is not portable content"));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
