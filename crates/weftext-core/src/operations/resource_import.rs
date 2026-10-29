//! One original resource import freezes the entire upload and manifest decision.
//! The existing operation journal owns installation and unknown-result recovery.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceImportFile {
    pub name: String,
    pub bytes: Vec<u8>,
    pub resource_base_id: String,
}

#[derive(Debug, Clone)]
pub struct ResourceImportUpload {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// These are the original Plan's identities, in upload order. A prepared result
/// is a proposal, not proof that bytes have been installed or remain current.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceImportResult {
    pub name: String,
    pub resource_ref: String,
    pub exact_revision: String,
}

fn batch_error(code: &str, message: UiMessage) -> CoreError {
    CoreError::new(code, message)
}

fn validate_uploads<'a>(files: impl IntoIterator<Item = (&'a str, usize)>) -> Result<()> {
    let files: Vec<_> = files.into_iter().collect();
    if files.is_empty() {
        return Err(batch_error(
            "resourceBatchEmpty",
            UiMessage::error_core_operations_resource_batch_empty(),
        ));
    }
    if files.len() > 64 {
        return Err(batch_error(
            "budget",
            UiMessage::error_core_operations_at_most64_registered_resource_descriptors(),
        ));
    }
    if files
        .iter()
        .try_fold(0usize, |total, (_, size)| total.checked_add(*size))
        .is_none_or(|total| total > MAX_OPERATION_BYTES)
    {
        return Err(batch_error(
            "budget",
            UiMessage::error_core_operations_resource_exceeds64_mi_b(),
        ));
    }
    for (index, (name, _)) in files.iter().enumerate() {
        valid_name(name)?;
        for (previous, _) in &files[..index] {
            if backend::namespace_path_compare(Path::new(name), Path::new(previous))?.is_eq() {
                return Err(batch_error(
                    "resourceConflict",
                    UiMessage::error_core_operations_resource_batch_names_conflict(),
                ));
            }
        }
    }
    Ok(())
}

impl Core {
    pub fn prepare_import_resources(
        &mut self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        files: Vec<ResourceImportUpload>,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        validate_uploads(
            files
                .iter()
                .map(|file| (file.name.as_str(), file.bytes.len())),
        )?;
        // Consult the original request before making any new snapshots or IDs.
        if let Some(t) = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
        {
            if t.request.root == display(&root)
                && matches!(&t.request.intent, OperationIntent::ImportResources {node, files: original, ..}
                    if node == &display(&path) && original.len() == files.len()
                    && original.iter().zip(&files).all(|(a,b)| a.name == b.name && a.bytes == b.bytes))
            {
                return self.operation_details(operation_id);
            }
            return Err(batch_error(
                "operationConflict",
                UiMessage::error_core_operations_operation_id_belongs_to_different_import_request(),
            ));
        }
        let names = files
            .iter()
            .map(|file| file.name.clone())
            .collect::<Vec<_>>();
        let cut = self.read_resource_import_snapshot(&root, &path, &names)?;
        let frozen = files
            .into_iter()
            .zip(cut.files.iter().skip(1))
            .map(|(file, snapshot)| ResourceImportFile {
                name: file.name,
                bytes: file.bytes,
                resource_base_id: snapshot.base_id.clone(),
            })
            .collect();
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(&root),
            intent: OperationIntent::ImportResources {
                node: display(&path),
                files: frozen,
                manifest_base_id: cut.files[0].base_id.clone(),
            },
        })
    }

    /// A resource import has the existing 64 descriptor budget plus exactly one
    /// manifest. Do not silently reduce it through the generic 64-path reader.
    pub fn read_resource_import_snapshot(
        &mut self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        names: &[String],
    ) -> Result<SnapshotSet> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        validate_uploads(names.iter().map(|name| (name.as_str(), 0)))?;
        let directory = path.parent().ok_or_else(|| {
            batch_error(
                "invalidPath",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        let canonical = self.managed_node(&root, directory, false)?;
        if Path::new(&canonical.path) != path {
            return Err(batch_error(
                "unsupportedRole",
                UiMessage::error_core_operations_select_canonical_admitted_node_source(),
            ));
        }
        let paths: Vec<_> = std::iter::once(display(&directory.join("weftext.resources.json")))
            .chain(names.iter().map(|name| display(&directory.join(name))))
            .collect();
        self.read_bounded_snapshot(&root, &paths, 64 + 1)
    }

    pub(super) fn plan_resource_import(
        &self,
        root: &Path,
        node: &str,
        imports: &[ResourceImportFile],
        manifest_base_id: &str,
        preview: &mut OperationPreview,
        files: &mut Vec<CapturedFile>,
    ) -> Result<Mutation> {
        validate_uploads(
            imports
                .iter()
                .map(|file| (file.name.as_str(), file.bytes.len())),
        )?;
        let node_path = absolute_path(Path::new(node))?;
        let directory = node_path.parent().ok_or_else(|| {
            batch_error(
                "invalidPath",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        let canonical = self.managed_node(root, directory, false)?;
        if Path::new(&canonical.path) != node_path {
            return Err(batch_error(
                "unsupportedRole",
                UiMessage::error_core_operations_select_canonical_admitted_node_source(),
            ));
        }
        let owner = self
            .admitted_binding(&canonical.stamp.identity)?
            .ok_or_else(|| {
                batch_error(
                    "resourceOwner",
                    UiMessage::error_core_operations_resource_owner_missing(),
                )
            })?;
        let manifest_path = directory.join("weftext.resources.json");
        let manifest = capture_optional(&self.backend, &manifest_path)?;
        self.verify_snapshot(&manifest_path, manifest_base_id, manifest.as_ref())?;
        if manifest
            .as_ref()
            .is_some_and(|file| file.bytes.len() > MAX_SOURCE_BYTES)
        {
            return Err(batch_error(
                "budget",
                UiMessage::error_core_operations_owned_resource_manifest_exceeds2_mi_b(),
            ));
        }
        if manifest.is_none() {
            self.require_resource_collection_uninitialized(&owner)?;
        }
        let mut value: serde_json::Value = manifest
            .as_ref()
            .map(|f| serde_json::from_slice(&f.bytes))
            .transpose()?
            .unwrap_or_else(|| serde_json::json!({"version":1,"resources":[]}));
        if value["version"] != 1 || !value["resources"].is_array() {
            return Err(batch_error(
                "resourceManifest",
                UiMessage::error_core_operations_unsupported_resource_manifest(),
            ));
        }
        if value["resources"]
            .as_array()
            .unwrap()
            .len()
            .saturating_add(imports.len())
            > 64
        {
            return Err(batch_error(
                "budget",
                UiMessage::error_core_operations_at_most64_registered_resource_descriptors(),
            ));
        }
        let mut changes = Vec::with_capacity(imports.len() + 1);
        // All names and create-only baselines qualify before allocating results
        // or staging any journal participant. A last-item conflict is whole-batch.
        for import in imports {
            let target = directory.join(&import.name);
            for control in [
                &node_path,
                &manifest_path,
                &directory.join("weftext.annotations.json"),
            ] {
                if backend::namespace_path_compare(&target, control)?.is_eq() {
                    return Err(batch_error(
                        "resourceConflict",
                        UiMessage::error_core_operations_resource_cannot_replace_node_control_file(
                        ),
                    ));
                }
            }
            let current = capture_optional(&self.backend, &target)?;
            self.verify_snapshot(&target, &import.resource_base_id, current.as_ref())?;
            if current.is_some() {
                return Err(batch_error(
                    "exists",
                    UiMessage::error_core_operations_resource_import_create_only(),
                ));
            }
            for entry in value["resources"].as_array().unwrap() {
                if let Some(path) = entry["path"].as_str()
                    && backend::namespace_path_compare(&directory.join(path), &target)?.is_eq()
                {
                    return Err(batch_error(
                        "resourceConflict",
                        UiMessage::error_core_operations_manifest_already_has_path(),
                    ));
                }
            }
            changes.push(FileChange {
                path: display(&target),
                base_id: Some(import.resource_base_id.clone()),
                before: None,
                after: import.bytes.clone(),
            });
        }
        for (import, change) in imports.iter().zip(&changes) {
            let resource_id = fresh();
            let resource_ref = format!("weftext-resource://{}/{resource_id}", owner.workspace_id);
            value["resources"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"resourceId":resource_id,"path":import.name}));
            preview
                .resource_map
                .insert(change.path.clone(), resource_ref);
        }
        let manifest_after = serde_json::to_vec_pretty(&value)?;
        if manifest_after.len() > MAX_SOURCE_BYTES {
            return Err(batch_error(
                "budget",
                UiMessage::error_core_operations_owned_resource_manifest_exceeds2_mi_b(),
            ));
        }
        if imports
            .iter()
            .map(|file| file.bytes.len())
            .sum::<usize>()
            .saturating_add(manifest_after.len())
            > MAX_OPERATION_BYTES
        {
            return Err(batch_error(
                "budget",
                UiMessage::error_core_operations_resource_exceeds64_mi_b(),
            ));
        }
        changes.push(FileChange {
            path: display(&manifest_path),
            base_id: Some(manifest_base_id.into()),
            before: manifest.as_ref().map(|f| f.bytes.clone()),
            after: manifest_after,
        });
        if let Some(manifest) = manifest {
            files.push(manifest);
        }
        files.push(canonical);
        for c in &changes {
            preview
                .changes
                .push(change(Path::new(&c.path), "import", c.after.len()));
            preview.revisions.insert(c.path.clone(), fresh());
        }
        preview
            .node_map
            .insert(owner.node_id.clone(), owner.node_id);
        Ok(Mutation::Update { files: changes })
    }

    pub fn resource_import_results(&self, operation_id: &str) -> Result<Vec<ResourceImportResult>> {
        self.backend.check_path(&self.data_dir)?;
        self.check_sync_operation_access(operation_id)?;
        let t = self
            .operations
            .get(operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(|| {
                batch_error(
                    "operationMissing",
                    UiMessage::error_core_operations_no_frozen_operation_has_i_d(),
                )
            })?;
        let (node, names): (&str, Vec<&str>) = match &t.request.intent {
            OperationIntent::ImportResource { node, name, .. } => (node, vec![name]),
            OperationIntent::ImportResources { node, files, .. } => {
                (node, files.iter().map(|file| file.name.as_str()).collect())
            }
            _ => {
                return Err(batch_error(
                    "operationConflict",
                    UiMessage::error_core_operations_resource_batch_result_unavailable(),
                ));
            }
        };
        let directory = Path::new(node).parent().ok_or_else(|| {
            batch_error(
                "invalidPlan",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        names
            .into_iter()
            .map(|name| {
                let path = display(&directory.join(name));
                Ok(ResourceImportResult {
                    name: name.into(),
                    resource_ref: t.preview.resource_map.get(&path).cloned().ok_or_else(|| {
                        batch_error(
                            "invalidPlan",
                            UiMessage::error_core_operations_resource_batch_result_unavailable(),
                        )
                    })?,
                    exact_revision: t.preview.revisions.get(&path).cloned().ok_or_else(|| {
                        batch_error(
                            "invalidPlan",
                            UiMessage::error_core_operations_resource_batch_result_unavailable(),
                        )
                    })?,
                })
            })
            .collect()
    }

    pub fn resource_import_preview_at(
        &mut self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<OperationPreview> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        self.resource_import_results(operation_id)?;
        let t = self.operations[operation_id].transaction.as_ref().unwrap();
        let expected_owner = t.preview.node_map.values().next().cloned();
        if t.request.root != display(&root) || t.preview.node_map.len() != 1 {
            return Err(batch_error(
                "resourceOwner",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            ));
        }
        let directory = path.parent().ok_or_else(|| {
            batch_error(
                "invalidPath",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        let current = self.managed_node(&root, directory, false)?;
        if Path::new(&current.path) != path
            || self
                .admitted_binding(&current.stamp.identity)?
                .as_ref()
                .map(|owner| &owner.node_id)
                != expected_owner.as_ref()
        {
            return Err(batch_error(
                "resourceOwner",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            ));
        }
        self.operation_details(operation_id)
    }

    /// Qualify the original installed batch for a separate Draft delivery.
    /// This never saves author source or imports/reverts any resource.
    pub fn validate_resource_import_insertion(
        &self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<Vec<ResourceImportResult>> {
        self.resource_import_insertion_inputs(root, node_source, operation_id)
            .map(|(imports, _)| imports)
    }

    /// Return only the qualified original results and their current bytes.
    /// All capture witnesses stay local to this call; a Draft proposal has no
    /// reason to register persistent editor snapshot bases for resource data.
    pub fn resource_import_insertion_inputs(
        &self,
        root: impl AsRef<Path>,
        node_source: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<(Vec<ResourceImportResult>, Vec<Vec<u8>>)> {
        let root = absolute_path(root.as_ref())?;
        let path = absolute_path(node_source.as_ref())?;
        if self.committed_operation_receipt(operation_id)?.is_none() {
            return Err(batch_error(
                "operationPending",
                UiMessage::error_core_operations_resource_batch_committed_required(),
            ));
        }
        let results = self.resource_import_results(operation_id)?;
        let t = self.operations[operation_id].transaction.as_ref().unwrap();
        let owner_id = t.preview.node_map.values().next().cloned();
        let manifest_revision = t
            .preview
            .revisions
            .iter()
            .find(|(path, _)| {
                Path::new(path).file_name() == Some(std::ffi::OsStr::new("weftext.resources.json"))
            })
            .map(|(_, revision)| revision.clone());
        if t.request.root != display(&root) || t.preview.node_map.len() != 1 {
            return Err(batch_error(
                "resourceOwner",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            ));
        }
        let directory = path.parent().ok_or_else(|| {
            batch_error(
                "invalidPath",
                UiMessage::error_core_operations_missing_resource_owner(),
            )
        })?;
        let canonical = self.managed_node(&root, directory, false)?;
        let owner = self.admitted_binding(&canonical.stamp.identity)?;
        if Path::new(&canonical.path) != path
            || owner.as_ref().map(|owner| &owner.node_id) != owner_id.as_ref()
        {
            return Err(batch_error(
                "resourceOwner",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            ));
        }
        let mut facts = QualifiedReadObservation::read_snapshots(self)?;
        let (mut current, mut bindings) =
            self.capture_managed_resource_snapshot(&root, &path, &mut facts)?;
        facts.check_snapshot_records()?;
        drop(facts);
        // Qualify the manifest from this very resource cut, rather than a
        // separate earlier capture which could change before the second read.
        let manifest_base_id = current
            .first()
            .map(|resource| resource.manifest_base_id.as_str())
            .ok_or_else(|| {
                batch_error(
                    "resourceManifest",
                    UiMessage::error_core_operations_resource_batch_destination_changed(),
                )
            })?;
        let manifest_index = bindings
            .iter()
            .position(|(base_id, _)| base_id == manifest_base_id)
            .ok_or_else(|| {
                batch_error(
                    "resourceManifest",
                    UiMessage::error_core_operations_resource_batch_destination_changed(),
                )
            })?;
        let manifest = SnapshotWitness {
            binding: bindings.swap_remove(manifest_index).1,
        };
        let mut bytes = Vec::with_capacity(results.len());
        for result in &results {
            if let Some(resource) = current.iter_mut().find(|resource| {
                resource.resource_ref == result.resource_ref
                    && resource.node_id == owner_id.as_deref().unwrap_or_default()
                    && resource.revision.as_deref() == Some(result.exact_revision.as_str())
                    && Path::new(&resource.path) == directory.join(&result.name)
            }) {
                bytes.push(std::mem::take(&mut resource.bytes));
            } else {
                return Err(batch_error(
                    "resourceVersion",
                    UiMessage::error_core_operations_resource_batch_destination_changed(),
                ));
            }
        }
        self.validate_import_manifest(&root, &manifest, manifest_revision.as_deref())?;
        Ok((results, bytes))
    }

    fn validate_import_manifest(
        &self,
        root: &Path,
        manifest: &SnapshotWitness,
        expected_revision: Option<&str>,
    ) -> Result<()> {
        let captured = manifest.binding.captured.as_ref().ok_or_else(|| {
            batch_error(
                "resourceManifest",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            )
        })?;
        self.validate_snapshot_witness(root, &manifest.binding.path, manifest)?;
        if expected_revision.is_none()
            || self.accepted_revision(captured)?.as_deref() != expected_revision
        {
            return Err(batch_error(
                "resourceManifest",
                UiMessage::error_core_operations_resource_batch_destination_changed(),
            ));
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod tests;
