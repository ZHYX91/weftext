//! One explicitly selected owned resource. No sibling resource bytes or media
//! decoders participate in Core's identity and current-version qualification.
use super::*;
use operations::{CapturedFile, SnapshotBinding, read_blob, resource_path};

pub struct QualifiedResource {
    pub owner: weftext_knowledge::NodeRef,
    pub resource_ref: String,
    pub lifecycle: String,
    pub revision: Option<String>,
    pub content_hash: String,
    pub witness: ResourceWitness,
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
        let root = absolute_path(root)?;
        let owner_path = absolute_path(owner_path)?;
        let resource_path_input = absolute_path(resource_path_input)?;
        let prefix = format!("weftext-resource://{}/", expected_owner.workspace_id);
        let id = resource_ref.strip_prefix(&prefix).ok_or_else(|| {
            CoreError::new(
                "resourceIdentity",
                "The selected ResourceRef belongs to another workspace",
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
                "The resource owner's actual directory changed",
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
                CoreError::new("resourceManifest", "Unsupported owned resource manifest")
            })?;
        if records.len() > 64 {
            return Err(CoreError::new(
                "budget",
                "At most 64 registered resource descriptors",
            ));
        }
        // Unknown or damaged sibling descriptors do not cause their files to
        // be opened. Only this selected stable ID may define its locator.
        let mut selected = records.iter().filter(|r| r["resourceId"] == id);
        let record = selected.next().ok_or_else(|| {
            CoreError::new(
                "resourceMissing",
                "The selected ResourceRef is not registered by this owner",
            )
        })?;
        if selected.next().is_some() {
            return Err(CoreError::new(
                "resourceManifest",
                "The selected resource has ambiguous registration",
            ));
        }
        let name = record["path"].as_str().ok_or_else(|| {
            CoreError::new(
                "resourcePath",
                "The selected resource has no exact relative locator",
            )
        })?;
        let declared = resource_path(directory, name)?;
        if !crate::history::history_relative_path(&resource_path_input, &declared)
            .is_some_and(|p| p.as_os_str().is_empty())
        {
            return Err(CoreError::new(
                "resourcePath",
                "The configured resource locator differs from its selected registration",
            ));
        }
        self.check_pending(&resource_path_input)?;
        let resource_guard = ParentGuard::acquire(&resource_path_input)?;
        let mut resource_handle = resource_guard.open_file(false, false)?;
        let resource_stamp = stamp(&resource_handle)?;
        self.check_pending_identity(&resource_stamp.identity)?;
        let coordinate = backend::file_coordinates(&resource_handle)?;
        let actual_relative = coordinate
            .strip_prefix(owner.coordinate.parent().unwrap())
            .map_err(|_| {
                CoreError::new(
                    "resourceOwner",
                    "The actual resource is outside its qualified owner directory",
                )
            })?;
        let declared_relative = declared.strip_prefix(directory).unwrap();
        if actual_relative.components().count() != declared_relative.components().count()
            || crate::history::history_relative_path(actual_relative, declared_relative).is_none()
        {
            return Err(CoreError::new(
                "resourcePath",
                "The actual resource does not occupy its registered locator",
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
                "The resource object has no original owner-bound Core birth",
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
                "The exact original resource version is no longer current; no latest or historical fallback was used",
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
        Ok(QualifiedResource {
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
        })
    }
    pub fn validate_resource_witness(&self, root: &Path, witness: &ResourceWitness) -> Result<()> {
        if absolute_path(root)? != witness.root {
            return Err(CoreError::new(
                "outsideRoot",
                "Resource evidence belongs to another configured root",
            ));
        }
        self.validate_query_node_witness(root, &witness.owner_witness)?;
        self.validate_snapshot_witness(root, &witness.manifest.binding.path, &witness.manifest)?;
        self.validate_snapshot_witness(root, &witness.resource.binding.path, &witness.resource)?;
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
}
