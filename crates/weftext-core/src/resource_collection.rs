//! Explicit owner-bound resource membership projected from its original
//! accepted manifest. No registered resource body or ambient tree is read.
use super::*;
use operations::{CapturedFile, Mutation, SnapshotBinding, resource_path};
use std::collections::BTreeSet;

pub struct ResourceCollectionEntry {
    pub resource_ref: String,
    /// The original manifest's owner-directory-relative locator.
    pub relative_path: String,
}
pub struct QualifiedResourceCollection {
    pub owner: weftext_knowledge::NodeRef,
    pub lifecycle: String,
    pub initialized: bool,
    /// Exact accepted manifest version; None only for genuinely uninitialized.
    pub revision: Option<String>,
    pub entries: Vec<ResourceCollectionEntry>,
    /// Unknown, malformed, ambiguous or unadmitted descriptors were retained
    /// in the original manifest, but do not acquire resource read authority.
    pub unrecognized_entries: usize,
    pub witness: ResourceCollectionWitness,
}
impl QualifiedResourceCollection {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.owner.workspace_id.capacity()
            + self.owner.node_id.capacity()
            + self.lifecycle.capacity()
            + self.revision.as_ref().map_or(0, String::capacity)
            + self.entries.capacity() * std::mem::size_of::<ResourceCollectionEntry>()
            + self
                .entries
                .iter()
                .map(|entry| entry.resource_ref.capacity() + entry.relative_path.capacity())
                .sum::<usize>()
            + self.witness.retained_bytes()
    }
}
/// Opaque original owner, membership and version evidence. An absent manifest
/// is a read-only negative observation, never a lock or writable read basis.
pub struct ResourceCollectionWitness {
    root: PathBuf,
    owner_birth: Birth,
    owner_witness: QueryNodeWitness,
    manifest: SnapshotWitness,
    revision: Option<String>,
}
impl ResourceCollectionWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.root.as_os_str().len() * 2
            + self.owner_birth.node_id.capacity()
            + self.owner_birth.workspace_id.capacity()
            + self.owner_witness.retained_bytes()
            + self.manifest.retained_bytes()
            + self.revision.as_ref().map_or(0, String::capacity)
    }
}

impl Core {
    /// Qualify the actual original DOCX-produced Node in one preconfigured
    /// import source slot. Moving it requires an explicit expected NodeRef at
    /// the new slot; its original producer is not authority for another path.
    pub fn qualified_imported_query_node_at(
        &self,
        root: &Path,
        source_path: &Path,
    ) -> Result<QualifiedQueryNode> {
        let root = absolute_path(root)?;
        let source_path = absolute_path(source_path)?;
        let workspace = self.require_workspace(&root)?;
        self.check_pending(&source_path)?;
        let guard = ParentGuard::acquire(&source_path)?;
        let identity = stamp(&guard.open_placement_file()?)?.identity;
        self.check_pending_identity(&identity)?;
        let mut expected = None;
        for plan in self.operations.values().filter(|plan| {
            self.committed(&plan.operation_id)
                && plan.workspace_id.as_deref() == Some(&workspace.workspace_id)
        }) {
            let Some(operation) = &plan.transaction else {
                continue;
            };
            let OperationIntent::ImportDocx { parent, name, .. } = &operation.request.intent else {
                continue;
            };
            let Mutation::CreateTree {
                existing_target: false,
                target,
                births,
                files,
                ..
            } = &operation.mutation
            else {
                continue;
            };
            let requested_target = Path::new(parent).join(name);
            if !crate::history::history_relative_path(Path::new(target), &requested_target)
                .is_some_and(|relative| relative.as_os_str().is_empty())
            {
                continue;
            }
            for birth in self
                .read_births(&plan.operation_id)?
                .into_iter()
                .filter(|birth| {
                    birth.workspace_id == workspace.workspace_id && birth.identity == identity
                })
            {
                let producer = births.iter().find(|created| {
                    created.node_id == birth.node_id
                        && is_canonical(Path::new(&created.path))
                        && crate::history::history_relative_path(
                            &source_path,
                            Path::new(&created.path),
                        )
                        .is_some_and(|relative| relative.as_os_str().is_empty())
                        && crate::history::history_relative_path(
                            Path::new(&created.path),
                            Path::new(target),
                        )
                        .is_some_and(|relative| !relative.as_os_str().is_empty())
                        && files
                            .iter()
                            .any(|file| file.path == created.path && file.before.is_none())
                });
                if producer.is_none() {
                    continue;
                }
                self.history_commit_time(plan)?;
                if expected
                    .replace(weftext_knowledge::NodeRef {
                        workspace_id: birth.workspace_id,
                        node_id: birth.node_id,
                    })
                    .is_some()
                {
                    return Err(CoreError::new(
                        "resourceOwner",
                        "Original imported source admission is ambiguous",
                    ));
                }
            }
        }
        let expected = expected.ok_or_else(|| {
            CoreError::new(
                "resourceOwner",
                "This exact slot has no committed original DOCX source producer",
            )
        })?;
        self.qualified_query_node_at(&root, &source_path, &expected, "live")
    }

    pub fn qualified_resource_collection_at(
        &self,
        root: &Path,
        owner_path: &Path,
        expected_owner: &weftext_knowledge::NodeRef,
        lifecycle: &str,
    ) -> Result<QualifiedResourceCollection> {
        let root = absolute_path(root)?;
        let owner_path = absolute_path(owner_path)?;
        let owner = self.qualified_query_node_at(&root, &owner_path, expected_owner, lifecycle)?;
        let owner_guard = ParentGuard::acquire(&owner_path)?;
        let identity = stamp(&owner_guard.open_placement_file()?)?.identity;
        let birth = self.admitted_binding(&identity).ok_or_else(|| {
            CoreError::new(
                "resourceOwner",
                "The original resource owner is unavailable",
            )
        })?;
        if birth.workspace_id != expected_owner.workspace_id
            || birth.node_id != expected_owner.node_id
        {
            return Err(CoreError::new(
                "resourceOwner",
                "The selected resource owner changed",
            ));
        }
        let directory = owner_path.parent().unwrap();
        let manifest_path = directory.join("weftext.resources.json");
        self.check_pending(&manifest_path)?;
        let guard = ParentGuard::acquire(&manifest_path)?;
        if guard.directory_coordinates()? != owner.coordinate.parent().unwrap() {
            return Err(CoreError::new(
                "staleBase",
                "The resource owner's actual directory changed",
            ));
        }
        let (captured, entries, unrecognized_entries, revision) = match guard
            .open_file(false, false)
        {
            Ok(mut handle) => {
                let observed = stamp(&handle)?;
                self.check_pending_identity(&observed.identity)?;
                let bytes = read_source(&mut handle)?;
                let captured = CapturedFile {
                    path: display(&manifest_path),
                    stamp: observed,
                    bytes,
                };
                let revision = self.accepted_revision(&captured).ok_or_else(|| {
                    CoreError::new(
                        "resourceManifestUnavailable",
                        "Collection membership requires a currently accepted exact manifest",
                    )
                })?;
                let document: serde_json::Value = serde_json::from_slice(&captured.bytes)?;
                let records = document["resources"]
                    .as_array()
                    .filter(|_| document["version"] == 1)
                    .ok_or_else(|| {
                        CoreError::new("resourceManifest", "Unsupported owned resource manifest")
                    })?;
                if records.len() > 64 {
                    return Err(CoreError::new(
                        "budget",
                        "At most 64 registered resource descriptors",
                    ));
                }
                let mut entries = Vec::new();
                for record in records {
                    let (Some(id), Some(name)) =
                        (record["resourceId"].as_str(), record["path"].as_str())
                    else {
                        continue;
                    };
                    if Uuid::parse_str(id)
                        .ok()
                        .is_none_or(|uuid| uuid.to_string() != id)
                        || resource_path(directory, name).is_err()
                        || !self.resource_owner_birth_known(id, expected_owner)
                        || records.iter().filter(|r| r["resourceId"] == id).count() != 1
                        || records
                            .iter()
                            .filter(|r| {
                                r["path"].as_str().is_some_and(|other| {
                                    crate::history::history_relative_path(
                                        Path::new(other),
                                        Path::new(name),
                                    )
                                    .is_some_and(|relative| relative.as_os_str().is_empty())
                                })
                            })
                            .count()
                            != 1
                    {
                        continue;
                    }
                    entries.push(ResourceCollectionEntry {
                        resource_ref: format!(
                            "weftext-resource://{}/{id}",
                            expected_owner.workspace_id
                        ),
                        relative_path: name.into(),
                    });
                }
                let unrecognized = records.len() - entries.len();
                self.validate_query_node_witness(&root, &owner.witness)?;
                (Some(captured), entries, unrecognized, Some(revision))
            }
            Err(error) if error.code == "missing" => {
                self.require_resource_collection_uninitialized(&birth)?;
                (None, Vec::new(), 0, None)
            }
            Err(error) => return Err(error),
        };
        let manifest = SnapshotWitness {
            binding: SnapshotBinding {
                path: display(&manifest_path),
                parent: guard.directory_identity()?,
                captured,
            },
        };
        // Recheck the original observation after its handle is released;
        // consumers retain this exact witness for their final read decision.
        self.validate_query_node_witness(&root, &owner.witness)?;
        self.validate_snapshot_witness(&root, &manifest_path, &manifest)?;
        Ok(QualifiedResourceCollection {
            owner: expected_owner.clone(),
            lifecycle: lifecycle.into(),
            initialized: revision.is_some(),
            revision: revision.clone(),
            entries,
            unrecognized_entries,
            witness: ResourceCollectionWitness {
                root,
                owner_birth: birth,
                owner_witness: owner.witness,
                manifest,
                revision,
            },
        })
    }

    /// A missing registered manifest cannot become a fresh collection merely
    /// because its old path was renamed or is occupied by a different owner.
    /// This projects original observation facts at their original identity-
    /// proved placement; it does not create another membership ledger.
    pub(crate) fn require_resource_collection_uninitialized(&self, owner: &Birth) -> Result<()> {
        for plan in self.operations.values().filter(|plan| {
            self.committed(&plan.operation_id)
                && plan.workspace_id.as_deref() == Some(&owner.workspace_id)
        }) {
            let facts = self.read_observation_facts(&plan.operation_id)?;
            let mut paths = facts
                .iter()
                .map(|fact| fact.path.clone())
                .collect::<BTreeSet<_>>();
            if let Some(operation) = &plan.transaction {
                // A retained committed write is still an initialization fact
                // when an observation file was pruned or is unavailable.
                match &operation.mutation {
                    Mutation::Update { files } => {
                        paths.extend(files.iter().map(|file| file.path.clone()))
                    }
                    Mutation::CreateTree { files, updates, .. } => {
                        paths.extend(files.iter().chain(updates).map(|file| file.path.clone()))
                    }
                    Mutation::Relocate { updates, .. } | Mutation::Compound { updates, .. } => {
                        paths.extend(updates.iter().map(|file| file.path.clone()))
                    }
                }
            }
            paths.extend(
                plan.files
                    .iter()
                    .map(|(name, _)| display(&Path::new(&plan.path).join(name))),
            );
            let manifests = paths
                .iter()
                .filter(|path| {
                    Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case("weftext.resources.json"))
                })
                .collect::<Vec<_>>();
            if manifests.is_empty() {
                continue;
            }
            let sequence = self.history_commit_time(plan)?.1.ok_or_else(|| {
                CoreError::new(
                    "resourceManifestUnavailable",
                    "Original collection placement ordering is unavailable",
                )
            })?;
            let places = self
                .node_placement_paths_at(owner, Some(sequence))
                .map_err(|_| {
                    CoreError::new(
                        "resourceManifestUnavailable",
                        "Original collection placement evidence is unavailable",
                    )
                })?;
            if manifests.iter().any(|path| {
                places.iter().any(|source| {
                    let manifest_path = Path::new(source)
                        .parent()
                        .unwrap()
                        .join("weftext.resources.json");
                    crate::history::history_relative_path(Path::new(path), &manifest_path)
                        .is_some_and(|relative| relative.as_os_str().is_empty())
                })
            }) {
                return Err(CoreError::new(
                    "resourceManifestUnavailable",
                    "The original committed resource manifest is missing; it cannot be reinitialized",
                ));
            }
        }
        Ok(())
    }

    /// Recheck an original create-only manifest decision at normal commit and
    /// recovery. A delayed first-import Plan may not reset a collection which
    /// another committed decision initialized and an external tool removed.
    pub(crate) fn validate_resource_collection_initialization(
        &self,
        operation: &operations::FrozenOperation,
    ) -> Result<()> {
        if let OperationIntent::ImportResource { node, .. } = &operation.request.intent
            && let Mutation::Update { files } = &operation.mutation
            && files.iter().any(|file| {
                file.before.is_none()
                    && Path::new(&file.path).file_name()
                        == Some(std::ffi::OsStr::new("weftext.resources.json"))
            })
        {
            let source = operation
                .files
                .iter()
                .find(|file| {
                    crate::history::history_relative_path(Path::new(&file.path), Path::new(node))
                        .is_some_and(|relative| relative.as_os_str().is_empty())
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "resourceOwner",
                        "Original import owner evidence is unavailable",
                    )
                })?;
            let birth = self
                .admitted_binding(&source.stamp.identity)
                .ok_or_else(|| {
                    CoreError::new(
                        "resourceOwner",
                        "Original import owner admission is unavailable",
                    )
                })?;
            self.require_resource_collection_uninitialized(&birth)?;
        }
        Ok(())
    }

    pub fn validate_resource_collection_witness(
        &self,
        root: &Path,
        witness: &ResourceCollectionWitness,
    ) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                "Collection evidence belongs to another configured root",
            ));
        }
        self.validate_query_node_witness(root, &witness.owner_witness)?;
        self.validate_snapshot_witness(root, &witness.manifest.binding.path, &witness.manifest)?;
        if let Some(captured) = &witness.manifest.binding.captured {
            if self.accepted_revision(captured) != witness.revision {
                return Err(CoreError::new(
                    "resourceVersion",
                    "The original collection revision qualification changed",
                ));
            }
        } else {
            self.require_resource_collection_uninitialized(&witness.owner_birth)?;
        }
        Ok(())
    }

    /// Merge only original positive entities into the existing guarded Plan
    /// dependency pool. Never-initialized membership is read-only; a caller
    /// needs an explicitly reviewed ImportResource to initialize it.
    pub fn merge_resource_collection_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &ResourceCollectionWitness,
    ) -> Result<()> {
        if witness.manifest.binding.captured.is_none() {
            pool.clear();
            return Err(CoreError::new(
                "basisRequired",
                "An uninitialized collection cannot supply a guarded positive Plan dependency",
            ));
        }
        Self::merge_query_node_witness(pool, &witness.owner_witness)?;
        Self::merge_snapshot_witnesses(pool, vec![witness.manifest.clone()])
    }
}

#[cfg(all(test, windows))]
#[path = "resource_collection_tests.rs"]
mod tests;
