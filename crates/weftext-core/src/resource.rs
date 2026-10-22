//! One explicitly selected owned resource. No sibling resource bytes or media
//! decoders participate in Core's identity and current-version qualification.
use super::*;
use operations::{CapturedFile, SnapshotBinding, read_blob, resource_path};
use weftext_messages::UiMessage;

pub struct QualifiedResource {
    pub owner: weftext_knowledge::NodeRef,
    pub resource_ref: String,
    pub lifecycle: String,
    pub revision: Option<String>,
    pub content_hash: String,
    pub witness: ResourceWitness,
}
pub enum QualifiedResourceInput {
    Present(Box<QualifiedResource>),
    Missing(Box<MissingResourceWitness>),
}
/// A bounded read-only negative observation for one previously registered
/// resource. It cannot supply a writable basis or a sustained absence lock.
/// A missing leaf is qualified only through a currently accepted manifest;
/// raw or unknown manifest versions remain errors, not partial input.
pub struct MissingResourceWitness {
    root: PathBuf,
    owner: weftext_knowledge::NodeRef,
    resource_ref: String,
    owner_witness: QueryNodeWitness,
    manifest_revision: String,
    manifest: SnapshotWitness,
    resource: SnapshotWitness,
}
impl MissingResourceWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.root.as_os_str().len() * 2
            + self.owner.workspace_id.capacity()
            + self.owner.node_id.capacity()
            + self.resource_ref.capacity()
            + self.manifest_revision.capacity()
            + self.owner_witness.retained_bytes()
            + self.manifest.retained_bytes()
            + self.resource.retained_bytes()
    }
}
impl QualifiedResource {
    pub fn bytes(&self) -> &[u8] {
        &self
            .witness
            .resource
            .binding
            .captured
            .as_ref()
            .unwrap()
            .bytes
    }
}
/// Read-only evidence; it cannot be deserialized into a write basis or grant.
pub struct ResourceWitness {
    root: PathBuf,
    owner_path: PathBuf,
    resource_path: PathBuf,
    owner: weftext_knowledge::NodeRef,
    resource_ref: String,
    lifecycle: String,
    revision: Option<String>,
    owner_witness: QueryNodeWitness,
    manifest: SnapshotWitness,
    resource: SnapshotWitness,
}
impl ResourceWitness {
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + [&self.root, &self.owner_path, &self.resource_path]
                .into_iter()
                .map(|p| p.as_os_str().len() * 2)
                .sum::<usize>()
            + self.owner.workspace_id.capacity()
            + self.owner.node_id.capacity()
            + self.resource_ref.capacity()
            + self.lifecycle.capacity()
            + self.revision.as_ref().map_or(0, String::capacity)
            + self.owner_witness.retained_bytes()
            + self.manifest.retained_bytes()
            + self.resource.retained_bytes()
    }
}

impl Core {
    #[allow(clippy::too_many_arguments)] // Exact configured owner, ref, locator, lifecycle and optional original version.
    pub fn qualified_resource_at(
        &self,
        root: &Path,
        owner_path: &Path,
        expected_owner: &weftext_knowledge::NodeRef,
        resource_ref: &str,
        resource_path_input: &Path,
        lifecycle: &str,
        expected_revision: Option<&str>,
    ) -> Result<QualifiedResource> {
        match self.qualify_resource_input(
            root,
            owner_path,
            expected_owner,
            resource_ref,
            resource_path_input,
            lifecycle,
            expected_revision,
            false,
        )? {
            QualifiedResourceInput::Present(resource) => Ok(*resource),
            QualifiedResourceInput::Missing(_) => Err(CoreError::new(
                "resourceMissing",
                UiMessage::error_core_resource_selected_resource_missing(),
            )),
        }
    }
    #[allow(clippy::too_many_arguments)]
    pub fn qualified_resource_input_at(
        &self,
        root: &Path,
        owner_path: &Path,
        expected_owner: &weftext_knowledge::NodeRef,
        resource_ref: &str,
        resource_path_input: &Path,
        lifecycle: &str,
    ) -> Result<QualifiedResourceInput> {
        self.qualify_resource_input(
            root,
            owner_path,
            expected_owner,
            resource_ref,
            resource_path_input,
            lifecycle,
            None,
            true,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn qualify_resource_input(
        &self,
        root: &Path,
        owner_path: &Path,
        expected_owner: &weftext_knowledge::NodeRef,
        resource_ref: &str,
        resource_path_input: &Path,
        lifecycle: &str,
        expected_revision: Option<&str>,
        allow_missing: bool,
    ) -> Result<QualifiedResourceInput> {
        let root = absolute_path(root)?;
        let owner_path = absolute_path(owner_path)?;
        let resource_path_input = absolute_path(resource_path_input)?;
        let prefix = format!("weftext-resource://{}/", expected_owner.workspace_id);
        let id = resource_ref.strip_prefix(&prefix).ok_or_else(|| {
            CoreError::new(
                "resourceIdentity",
                UiMessage::error_core_resource_selected_resource_ref_belongs_to_another_workspace(),
            )
        })?;
        valid_operation_id(id)?;
        let owner = self.qualified_query_node_at(&root, &owner_path, expected_owner, lifecycle)?;
        let directory = owner_path.parent().unwrap();
        let manifest_path = directory.join("weftext.resources.json");
        self.check_pending(&manifest_path)?;
        let manifest_guard = ParentGuard::acquire(&manifest_path)?;
        if manifest_guard.directory_coordinates()? != owner.coordinate.parent().unwrap() {
            return Err(CoreError::new(
                "staleBase",
                UiMessage::error_core_resource_resource_owner_s_actual_directory_changed(),
            ));
        }
        let mut manifest_handle = manifest_guard.open_file(false, false)?;
        let manifest_stamp = stamp(&manifest_handle)?;
        self.check_pending_identity(&manifest_stamp.identity)?;
        let manifest_bytes = read_source(&mut manifest_handle)?;
        let document: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
        let records = document["resources"]
            .as_array()
            .filter(|_| document["version"] == 1)
            .ok_or_else(|| {
                CoreError::new(
                    "resourceManifest",
                    UiMessage::error_core_resource_unsupported_owned_resource_manifest(),
                )
            })?;
        if records.len() > 64 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_operations_at_most64_registered_resource_descriptors(),
            ));
        }
        // Unknown or damaged sibling descriptors do not cause their files to
        // be opened. Only this selected stable ID may define its locator.
        let mut selected = records.iter().filter(|r| r["resourceId"] == id);
        let record = selected.next().ok_or_else(|| {
            CoreError::new(
                "resourceMissing",
                UiMessage::error_core_resource_selected_resource_ref_not_registered_by_owner(),
            )
        })?;
        if selected.next().is_some() {
            return Err(CoreError::new(
                "resourceManifest",
                UiMessage::error_core_resource_selected_resource_has_ambiguous_registration(),
            ));
        }
        let name = record["path"].as_str().ok_or_else(|| {
            CoreError::new(
                "resourcePath",
                UiMessage::error_core_resource_selected_resource_has_no_exact_relative_locator(),
            )
        })?;
        let declared = resource_path(directory, name)?;
        if !crate::history::history_relative_path(&resource_path_input, &declared)
            .is_some_and(|p| p.as_os_str().is_empty())
        {
            return Err(CoreError::new(
                "resourcePath",
                UiMessage::error_core_resource_configured_resource_locator_differs_from_selected_registration(),
            ));
        }
        self.check_pending(&resource_path_input)?;
        let resource_guard = ParentGuard::acquire(&resource_path_input)?;
        let mut resource_handle = match resource_guard.open_file(false, false) {
            Ok(handle) => handle,
            Err(error) if allow_missing && error.code == "missing" => {
                let manifest = SnapshotWitness {
                    binding: SnapshotBinding {
                        path: display(&manifest_path),
                        parent: manifest_guard.directory_identity()?,
                        captured: Some(CapturedFile {
                            path: display(&manifest_path),
                            bytes: manifest_bytes,
                            stamp: manifest_stamp,
                        }),
                    },
                };
                let manifest_revision = self
                    .accepted_revision(manifest.binding.captured.as_ref().unwrap())
                    .ok_or_else(|| {
                        CoreError::new(
                            "resourceManifest",
                            UiMessage::error_core_resource_missing_input_requires_currently_accepted_exact_registration(),
                        )
                    })?;
                if !self.resource_owner_birth_known(id, expected_owner) {
                    return Err(CoreError::new(
                        "resourceUnadmitted",
                        UiMessage::error_core_resource_missing_input_has_no_original_committed_owner_bound_resource_birth(),
                    ));
                }
                self.validate_query_node_witness(&root, &owner.witness)?;
                let resource = SnapshotWitness {
                    binding: SnapshotBinding {
                        path: display(&resource_path_input),
                        parent: resource_guard.directory_identity()?,
                        captured: None,
                    },
                };
                self.validate_snapshot_witness(&root, &resource_path_input, &resource)?;
                return Ok(QualifiedResourceInput::Missing(Box::new(
                    MissingResourceWitness {
                        root,
                        owner: expected_owner.clone(),
                        resource_ref: resource_ref.into(),
                        owner_witness: owner.witness,
                        manifest_revision,
                        manifest,
                        resource,
                    },
                )));
            }
            Err(error) => return Err(error),
        };
        let resource_stamp = stamp(&resource_handle)?;
        self.check_pending_identity(&resource_stamp.identity)?;
        let coordinate = backend::file_coordinates(&resource_handle)?;
        let actual_relative = coordinate
            .strip_prefix(owner.coordinate.parent().unwrap())
            .map_err(|_| {
                CoreError::new(
                    "resourceOwner",
                    UiMessage::error_core_resource_actual_resource_outside_qualified_owner_directory(),
                )
            })?;
        let declared_relative = declared.strip_prefix(directory).unwrap();
        if actual_relative.components().count() != declared_relative.components().count()
            || crate::history::history_relative_path(actual_relative, declared_relative).is_none()
        {
            return Err(CoreError::new(
                "resourcePath",
                UiMessage::error_core_resource_actual_resource_does_not_occupy_registered_locator(),
            ));
        }
        if !self.admitted_resource(
            id,
            &expected_owner.workspace_id,
            &expected_owner.node_id,
            &resource_stamp.identity,
        ) {
            return Err(CoreError::new(
                "resourceUnadmitted",
                UiMessage::error_core_resource_resource_object_has_no_original_owner_bound_core_birth(),
            ));
        }
        let bytes = read_blob(&mut resource_handle)?;
        let captured = CapturedFile {
            path: display(&resource_path_input),
            bytes,
            stamp: resource_stamp,
        };
        let revision = self.accepted_revision(&captured);
        if expected_revision.is_some_and(|v| revision.as_deref() != Some(v)) {
            return Err(CoreError::new(
                "resourceVersion",
                UiMessage::error_core_resource_exact_original_resource_version_no_longer_current_no_latest_or_historical_fallback_was_used(),
            ));
        }
        let content_hash = hash(&captured.bytes);
        self.validate_query_node_witness(&root, &owner.witness)?;
        let manifest = SnapshotWitness {
            binding: SnapshotBinding {
                path: display(&manifest_path),
                parent: manifest_guard.directory_identity()?,
                captured: Some(CapturedFile {
                    path: display(&manifest_path),
                    bytes: manifest_bytes,
                    stamp: manifest_stamp,
                }),
            },
        };
        let resource = SnapshotWitness {
            binding: SnapshotBinding {
                path: display(&resource_path_input),
                parent: resource_guard.directory_identity()?,
                captured: Some(captured),
            },
        };
        Ok(QualifiedResourceInput::Present(Box::new(
            QualifiedResource {
                owner: expected_owner.clone(),
                resource_ref: resource_ref.into(),
                lifecycle: lifecycle.into(),
                revision: revision.clone(),
                content_hash,
                witness: ResourceWitness {
                    root,
                    owner_path,
                    resource_path: resource_path_input,
                    owner: expected_owner.clone(),
                    resource_ref: resource_ref.into(),
                    lifecycle: lifecycle.into(),
                    revision,
                    owner_witness: owner.witness,
                    manifest,
                    resource,
                },
            },
        )))
    }
    pub(crate) fn resource_owner_birth_known(
        &self,
        id: &str,
        owner: &weftext_knowledge::NodeRef,
    ) -> bool {
        self.operations
            .values()
            .filter(|plan| self.committed(&plan.operation_id))
            .any(|plan| {
                fs::read(self.journal_path(&plan.operation_id, "resource-births"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<Vec<serde_json::Value>>(&bytes).ok())
                    .is_some_and(|births| {
                        births.iter().any(|birth| {
                            birth["resource_id"] == id
                                && birth["workspace_id"] == owner.workspace_id
                                && birth["node_id"] == owner.node_id
                                && serde_json::from_value::<FileIdentity>(birth["identity"].clone())
                                    .ok()
                                    .is_some_and(|identity| {
                                        self.admitted_resource(
                                            id,
                                            &owner.workspace_id,
                                            &owner.node_id,
                                            &identity,
                                        )
                                    })
                        })
                    })
            })
    }
    pub fn validate_missing_resource_witness(
        &self,
        root: &Path,
        witness: &MissingResourceWitness,
    ) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_resource_missing_resource_evidence_belongs_to_another_configured_root(),
            ));
        }
        self.validate_query_node_witness(root, &witness.owner_witness)?;
        self.validate_snapshot_witness(root, &witness.manifest.binding.path, &witness.manifest)?;
        self.validate_snapshot_witness(root, &witness.resource.binding.path, &witness.resource)?;
        if self
            .accepted_revision(witness.manifest.binding.captured.as_ref().unwrap())
            .as_deref()
            != Some(witness.manifest_revision.as_str())
        {
            return Err(CoreError::new(
                "resourceManifest",
                UiMessage::error_core_resource_original_registration_revision_qualification_changed(
                ),
            ));
        }
        Ok(())
    }
    pub fn validate_resource_witness(&self, root: &Path, witness: &ResourceWitness) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_resource_resource_evidence_belongs_to_another_configured_root(
                ),
            ));
        }
        self.validate_query_node_witness(root, &witness.owner_witness)?;
        self.validate_snapshot_witness(root, &witness.manifest.binding.path, &witness.manifest)?;
        self.validate_snapshot_witness(root, &witness.resource.binding.path, &witness.resource)?;
        // A committed no-op can select a new portable Revision while keeping
        // the same physical object, stamp and bytes. Preserve the original
        // qualification, including an observed None becoming accepted Some.
        let captured = witness.resource.binding.captured.as_ref().unwrap();
        if self.accepted_revision(captured) != witness.revision {
            return Err(CoreError::new(
                "resourceVersion",
                UiMessage::error_core_resource_original_resource_revision_qualification_changed(),
            ));
        }
        Ok(())
    }
    pub fn merge_resource_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &ResourceWitness,
    ) -> Result<()> {
        Self::merge_query_node_witness(pool, &witness.owner_witness)?;
        Self::merge_snapshot_witnesses(
            pool,
            vec![witness.manifest.clone(), witness.resource.clone()],
        )
    }

    /// A feature Plan also retains the owner's original absent dependencies.
    /// The selected resource and registration themselves must still exist.
    pub(crate) fn merge_resource_feature_witness(
        pool: &mut Vec<SnapshotWitness>,
        witness: &ResourceWitness,
    ) -> Result<()> {
        let result = (|| {
            let mut positive = vec![];
            Self::merge_snapshot_witnesses(
                &mut positive,
                vec![witness.manifest.clone(), witness.resource.clone()],
            )?;
            Self::merge_query_node_feature_witness(pool, &witness.owner_witness)?;
            Self::merge_feature_witnesses(pool, positive)
        })();
        if result.is_err() {
            pool.clear();
        }
        result
    }
}

#[cfg(all(test, windows))]
#[path = "resource_witness_tests.rs"]
mod tests;
