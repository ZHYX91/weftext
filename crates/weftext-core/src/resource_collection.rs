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

fn diagnostic_witness_token(
    owner: &str,
    manifest: &SnapshotWitness,
    revision: Option<&str>,
) -> String {
    let stamp = manifest
        .binding
        .captured
        .as_ref()
        .map(|captured| resource_diagnostics::stamp_token(&captured.stamp))
        .unwrap_or_else(|| format!("absent:{:?}", manifest.binding.parent));
    format!("{owner}:{stamp}:{}", revision.unwrap_or("none"))
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
        self.backend.check_path(&self.data_dir)?;
        let root = absolute_path(root)?;
        let source_path = absolute_path(source_path)?;
        let workspace = self.require_workspace(&root)?;
        self.check_pending(&source_path)?;
        let guard = ParentGuard::acquire(&self.backend, &source_path)?;
        let identity = {
            let file = guard.open_placement_file()?;
            stamp(&file)?.identity
        };
        self.check_pending_identity(&identity)?;
        let mut expected = None;
        for plan in self
            .operations
            .values()
            .map(|plan| -> Result<_> {
                Ok(
                    if {
                        self.committed(&plan.operation_id)?
                            && plan.workspace_id.as_deref() == Some(&workspace.workspace_id)
                    } {
                        Some(plan)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let Some(operation) = &plan.transaction else {
                continue;
            };
            let (parent, name) = match &operation.request.intent {
                OperationIntent::ImportDocx { parent, name, .. }
                | OperationIntent::ImportDocxWithOptions { parent, name, .. } => (parent, name),
                _ => continue,
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
        let owner = self
            .qualified_query_node_at(&root, &owner_path, expected_owner, lifecycle)
            .inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.owner.qualify",
                    &expected_owner.node_id,
                    error,
                )
            })?;
        let owner_guard =
            ParentGuard::acquire(&self.backend, &owner_path).inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.owner.guard",
                    &expected_owner.node_id,
                    error,
                )
            })?;
        let identity = {
            let file = owner_guard.open_placement_file().inspect_err(|error| {
                resource_diagnostics::error("collection.owner.open", &expected_owner.node_id, error)
            })?;
            stamp(&file)
                .inspect_err(|error| {
                    resource_diagnostics::error(
                        "collection.owner.stamp",
                        &expected_owner.node_id,
                        error,
                    )
                })?
                .identity
        };
        let birth = self.admitted_binding(&identity)?.ok_or_else(|| {
            resource_diagnostics::branch(
                "collection.owner.birth",
                &expected_owner.node_id,
                "missingAdmission",
            );
            CoreError::new(
                "resourceOwner",
                "The original resource owner is unavailable",
            )
        })?;
        if birth.workspace_id != expected_owner.workspace_id
            || birth.node_id != expected_owner.node_id
        {
            resource_diagnostics::branch(
                "collection.owner.binding",
                &expected_owner.node_id,
                "ownerMismatch",
            );
            return Err(CoreError::new(
                "resourceOwner",
                "The selected resource owner changed",
            ));
        }
        let directory = owner_path.parent().unwrap();
        let manifest_path = directory.join("weftext.resources.json");
        self.check_pending(&manifest_path).inspect_err(|error| {
            resource_diagnostics::error(
                "collection.manifest.pending",
                &expected_owner.node_id,
                error,
            )
        })?;
        let guard = ParentGuard::acquire(&self.backend, &manifest_path).inspect_err(|error| {
            resource_diagnostics::error("collection.manifest.guard", &expected_owner.node_id, error)
        })?;
        if guard.directory_coordinates()? != owner.coordinate.parent().unwrap() {
            resource_diagnostics::branch(
                "collection.manifest.parent",
                &expected_owner.node_id,
                "parentMismatch",
            );
            return Err(CoreError::new(
                "staleBase",
                "The resource owner's actual directory changed",
            ));
        }
        let (captured, entries, unrecognized_entries, revision) = match guard
            .open_file(false, false)
        {
            Ok(mut handle) => {
                let observed = stamp(&handle).inspect_err(|error| {
                    resource_diagnostics::error(
                        "collection.manifest.stamp",
                        &expected_owner.node_id,
                        error,
                    )
                })?;
                self.check_pending_identity(&observed.identity)
                    .inspect_err(|error| {
                        resource_diagnostics::error(
                            "collection.manifest.identity_pending",
                            &expected_owner.node_id,
                            error,
                        )
                    })?;
                let bytes = read_source(&mut handle).inspect_err(|error| {
                    resource_diagnostics::error(
                        "collection.manifest.read",
                        &expected_owner.node_id,
                        error,
                    )
                })?;
                let captured = CapturedFile {
                    path: display(&manifest_path),
                    stamp: observed,
                    bytes,
                };
                let revision = self.accepted_revision(&captured)?.ok_or_else(|| {
                    resource_diagnostics::branch(
                        "collection.manifest.accepted",
                        &resource_diagnostics::stamp_token(&captured.stamp),
                        "noAcceptedRevision",
                    );
                    CoreError::new(
                        "resourceManifestUnavailable",
                        "Collection membership requires a currently accepted exact manifest",
                    )
                })?;
                let document: serde_json::Value = serde_json::from_slice(&captured.bytes)
                    .inspect_err(|_| {
                        resource_diagnostics::branch(
                            "collection.manifest.parse",
                            &expected_owner.node_id,
                            "invalidState",
                        )
                    })?;
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
                        || !self.resource_owner_birth_known(id, expected_owner)?
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
                self.validate_query_node_witness(&root, &owner.witness)
                    .inspect_err(|error| {
                        resource_diagnostics::error(
                            "collection.owner.recheck_before_release",
                            &expected_owner.node_id,
                            error,
                        )
                    })?;
                (Some(captured), entries, unrecognized, Some(revision))
            }
            Err(error) if error.code == "missing" => {
                self.require_resource_collection_uninitialized(&birth)?;
                (None, Vec::new(), 0, None)
            }
            Err(error) => {
                resource_diagnostics::error(
                    "collection.manifest.open",
                    &expected_owner.node_id,
                    &error,
                );
                return Err(error);
            }
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
        let diagnostic_token = resource_diagnostics::enabled().then(|| {
            diagnostic_witness_token(&expected_owner.node_id, &manifest, revision.as_deref())
        });
        self.validate_query_node_witness(&root, &owner.witness)
            .inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.owner.recheck_after_release",
                    diagnostic_token.as_deref().unwrap_or("disabled"),
                    error,
                )
            })?;
        self.validate_snapshot_witness(&root, &manifest_path, &manifest)
            .inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.manifest.recheck_after_release",
                    diagnostic_token.as_deref().unwrap_or("disabled"),
                    error,
                )
            })?;
        if let Some(token) = &diagnostic_token {
            resource_diagnostics::branch(
                "collection.capture",
                token,
                if revision.is_some() {
                    "accepted"
                } else {
                    "uninitialized"
                },
            );
        }
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
        for plan in self
            .operations
            .values()
            .map(|plan| -> Result<_> {
                Ok(
                    if {
                        self.committed(&plan.operation_id)?
                            && plan.workspace_id.as_deref() == Some(&owner.workspace_id)
                    } {
                        Some(plan)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let facts = self
                .read_observation_facts(&plan.operation_id)
                .map_err(|mut error| {
                    if error.code == "identityPending" {
                        error.code = "resourceManifestUnavailable".into();
                    }
                    error
                })?;
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
                    Mutation::Relocate { updates, .. } => {
                        paths.extend(updates.iter().map(|file| file.path.clone()))
                    }
                    Mutation::Compound {
                        updates,
                        created_files,
                        ..
                    } => paths.extend(
                        updates
                            .iter()
                            .chain(created_files)
                            .map(|file| file.path.clone()),
                    ),
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
                .admitted_binding(&source.stamp.identity)?
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
        let diagnostic_token = resource_diagnostics::enabled().then(|| {
            diagnostic_witness_token(
                &witness.owner_birth.node_id,
                &witness.manifest,
                witness.revision.as_deref(),
            )
        });
        if absolute_path(root)? != witness.root {
            resource_diagnostics::branch(
                "collection.recheck.root",
                diagnostic_token.as_deref().unwrap_or("disabled"),
                "rootMismatch",
            );
            return Err(CoreError::new(
                "outsideRoot",
                "Collection evidence belongs to another configured root",
            ));
        }
        self.validate_query_node_witness(root, &witness.owner_witness)
            .inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.recheck.owner",
                    diagnostic_token.as_deref().unwrap_or("disabled"),
                    error,
                )
            })?;
        self.validate_snapshot_witness(root, &witness.manifest.binding.path, &witness.manifest)
            .inspect_err(|error| {
                resource_diagnostics::error(
                    "collection.recheck.manifest",
                    diagnostic_token.as_deref().unwrap_or("disabled"),
                    error,
                )
            })?;
        if let Some(captured) = &witness.manifest.binding.captured {
            if self.accepted_revision(captured)? != witness.revision {
                resource_diagnostics::branch(
                    "collection.recheck.accepted",
                    diagnostic_token.as_deref().unwrap_or("disabled"),
                    "acceptedRevisionMismatch",
                );
                return Err(CoreError::new(
                    "resourceVersion",
                    "The original collection revision qualification changed",
                ));
            }
        } else {
            self.require_resource_collection_uninitialized(&witness.owner_birth)
                .inspect_err(|error| {
                    resource_diagnostics::error(
                        "collection.recheck.uninitialized",
                        diagnostic_token.as_deref().unwrap_or("disabled"),
                        error,
                    )
                })?;
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
