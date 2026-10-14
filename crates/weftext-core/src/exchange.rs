//! Native interchange consumes one frozen admitted library cut and enters the
//! same Core operation Plan as copy. Foreign journals/birth facts are never read.
use super::*;
use crate::operations::{
    AbsentDependency, CapturedFile, Mutation, NewBirth, change, ensure_absent, file_url,
};
use std::collections::{BTreeMap, BTreeSet};
use weftext_export::exchange::{
    NATIVE_FORMAT, NATIVE_VERSION, NativeEntry, NativeFileRole as Role, NativeLoss, NativeManifest,
    NativePackage, read_native_package, validate_member_path, validate_native_package,
    write_native_package,
};
use weftext_knowledge as knowledge;

fn native_error(error: weftext_export::exchange::NativeError) -> CoreError {
    CoreError::new(&error.code, error.message)
}
fn relative(root: &Path, path: &str) -> Result<String> {
    Path::new(path)
        .strip_prefix(root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            CoreError::new(
                "outsideRoot",
                "Native member is outside its selected library",
            )
        })
}
fn put(
    package: &mut NativePackage,
    path: String,
    bytes: Vec<u8>,
    entry: NativeEntry,
) -> Result<()> {
    if package.files.insert(path.clone(), bytes).is_some()
        || package.manifest.entries.insert(path, entry).is_some()
    {
        return Err(CoreError::new(
            "packagePath",
            "Native selection has a duplicate member",
        ));
    }
    Ok(())
}

impl Core {
    /// Whole admitted live library content, plus exact allowed resources/style
    /// and frozen config/schema. Raw unknown files and local history are excluded
    /// with explicit omissions; a native package is not a recovery backup.
    pub fn export_native_package(&mut self, root: impl AsRef<Path>) -> Result<Vec<u8>> {
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let frozen = self.admitted_compile_snapshot(&root, "live")?;
        if !frozen.snapshot.complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                "Native export requires a complete admitted live source cut",
            ));
        }
        let mut package = NativePackage {
            manifest: NativeManifest {
                format: NATIVE_FORMAT.into(),
                version: NATIVE_VERSION,
                package_id: fresh(),
                workspace_id: workspace.workspace_id,
                scope: "library".into(),
                schema_path: ".weftext-schema.json".into(),
                entries: BTreeMap::new(),
                losses: Vec::new(),
            },
            files: BTreeMap::new(),
        };
        for source in &frozen.snapshot.files {
            let Some(node_id) = &source.node_id else {
                continue;
            };
            let source_path = relative(&root, &source.path)?;
            let bytes = frozen.files.get(&source_path).cloned().ok_or_else(|| {
                CoreError::new(
                    "scopeChanged",
                    "Admitted source is missing from the frozen cut",
                )
            })?;
            if source.bytes.as_ref() != Some(&bytes) {
                return Err(CoreError::new(
                    "scopeChanged",
                    "Admitted source differs from the frozen native bytes",
                ));
            }
            let mut entry = NativeEntry::new(Role::Source, &bytes);
            entry.node_id = Some(node_id.clone());
            entry.revision = source.revision.clone();
            put(&mut package, source_path, bytes, entry)?;
            let owner_directory = Path::new(&source.path).parent().unwrap();
            let manifest_path = relative(
                &root,
                &display(&owner_directory.join("weftext.resources.json")),
            )?;
            if let Some(bytes) = frozen.files.get(&manifest_path) {
                let mut entry = NativeEntry::new(Role::ResourceManifest, bytes);
                entry.owner_node_id = Some(node_id.clone());
                put(&mut package, manifest_path.clone(), bytes.clone(), entry)?;
            }
            for resource in self.managed_resource_snapshot(&root, &source.path)? {
                let path = relative(&root, &resource.path)?;
                if resource.identity_status != "admitted"
                    || frozen.files.get(&path) != Some(&resource.bytes)
                    || frozen.files.get(&manifest_path) != Some(&resource.manifest_bytes)
                {
                    return Err(CoreError::new(
                        "resourceUnadmitted",
                        "Native resource binding/bytes differ from the frozen admitted cut",
                    ));
                }
                let role = if frozen
                    .style
                    .as_ref()
                    .is_some_and(|style| style.path == resource.path)
                {
                    Role::Style
                } else {
                    Role::Resource
                };
                let mut entry = NativeEntry::new(role, &resource.bytes);
                entry.owner_node_id = Some(node_id.clone());
                entry.resource_id = Some(resource.resource_id);
                entry.revision = resource.revision;
                put(&mut package, path, resource.bytes, entry)?;
            }
            let annotation_path = relative(
                &root,
                &display(&owner_directory.join("weftext.annotations.json")),
            )?;
            if let Some(bytes) = frozen.files.get(&annotation_path) {
                knowledge::decode_annotations(bytes)
                    .map_err(|e| CoreError::new(&e.code, e.message))?;
                let mut entry = NativeEntry::new(Role::Annotations, bytes);
                entry.owner_node_id = Some(node_id.clone());
                put(&mut package, annotation_path, bytes.clone(), entry)?;
            }
        }
        let schema = frozen
            .files
            .get(".weftext-schema.json")
            .cloned()
            .unwrap_or_else(|| {
                include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec()
            });
        weftext_data::Schema::from_author_json(&decode(&schema)?)
            .map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?;
        let entry = NativeEntry::new(Role::Schema, &schema);
        put(&mut package, ".weftext-schema.json".into(), schema, entry)?;
        if let Some(settings) = frozen.files.get(".weftext-settings.json") {
            let entry = NativeEntry::new(Role::Settings, settings);
            put(
                &mut package,
                ".weftext-settings.json".into(),
                settings.clone(),
                entry,
            )?;
        }
        for path in frozen
            .files
            .keys()
            .filter(|path| !package.files.contains_key(*path))
        {
            // These documented local controls are intentionally nonportable.
            if matches!(
                path.as_str(),
                ".weftext-workspace.json" | ".weftext-format" | ".weftext-mvp-commit.json"
            ) || path.starts_with(".weftext-trash/")
            {
                continue;
            }
            package.manifest.losses.push(NativeLoss {
                path: path.clone(), code: "unregisteredFileOmitted".into(),
                detail: "Unregistered/raw bytes are outside the admitted native content selection; use a complete backup to preserve local history and unknown files".into(),
            });
        }
        write_native_package(&package).map_err(native_error)
    }

    pub fn prepare_native_import(
        &mut self,
        root: impl AsRef<Path>,
        parent: impl AsRef<Path>,
        name: &str,
        bytes: Vec<u8>,
        operation_id: &str,
        allow_losses: bool,
    ) -> Result<OperationPreview> {
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(root.as_ref()),
            intent: OperationIntent::ImportNative {
                parent: display(parent.as_ref()),
                name: name.into(),
                bytes,
                allow_losses,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare_bibliography_import(
        &mut self,
        root: impl AsRef<Path>,
        parent: impl AsRef<Path>,
        name: &str,
        format: &str,
        bytes: Vec<u8>,
        operation_id: &str,
        allow_losses: bool,
    ) -> Result<OperationPreview> {
        self.prepare_operation(OperationRequest {
            operation_id: operation_id.into(),
            root: display(root.as_ref()),
            intent: OperationIntent::ImportBibliography {
                parent: display(parent.as_ref()),
                name: name.into(),
                format: format.into(),
                bytes,
                allow_losses,
            },
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn plan_native_import(
        &mut self,
        root: &Path,
        parent: &Path,
        name: &str,
        original: &[u8],
        allow_losses: bool,
        preview: &mut OperationPreview,
        dependencies: &mut Vec<CapturedFile>,
        absent: &mut Vec<AbsentDependency>,
    ) -> Result<Mutation> {
        let package = read_native_package(original).map_err(native_error)?;
        self.plan_content_import(
            root,
            parent,
            name,
            package,
            Some(original),
            allow_losses,
            preview,
            dependencies,
            absent,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn plan_bibliography_import(
        &mut self,
        root: &Path,
        parent: &Path,
        name: &str,
        format: &str,
        bytes: &[u8],
        allow_losses: bool,
        preview: &mut OperationPreview,
        dependencies: &mut Vec<CapturedFile>,
        absent: &mut Vec<AbsentDependency>,
    ) -> Result<Mutation> {
        let config = self.read_config(root, ConfigRole::Schema)?;
        let schema = match config.bytes {
            Some(bytes) => decode(&bytes)?,
            None => include_str!("../../../docs/design/examples/schema.authoring.json").into(),
        };
        let package = weftext_export::bibliography_import::build_bibliography_native_package(
            format,
            bytes,
            &schema,
            &preview.operation_id,
        )
        .map_err(|e| CoreError::new("bibliographyImport", e.to_string()))?;
        // The producer retains the actual uploaded bibliography and per-record
        // mappings as owned resources; there is no invented original native ZIP.
        self.plan_content_import(
            root,
            parent,
            name,
            package,
            None,
            allow_losses,
            preview,
            dependencies,
            absent,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn plan_content_import(
        &mut self,
        root: &Path,
        parent: &Path,
        name: &str,
        package: NativePackage,
        original_native: Option<&[u8]>,
        allow_losses: bool,
        preview: &mut OperationPreview,
        dependencies: &mut Vec<CapturedFile>,
        absent: &mut Vec<AbsentDependency>,
    ) -> Result<Mutation> {
        valid_name(name)?;
        let parent = absolute_path(parent)?;
        self.owned_parent(root, &parent)?;
        let target = parent.join(name);
        ensure_absent(&target)?;
        validate_native_package(&package).map_err(native_error)?;
        let workspace = self.require_workspace(root)?;
        let source_schema = weftext_data::Schema::from_author_json(&decode(
            &package.files[&package.manifest.schema_path],
        )?)
        .map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?;
        let config = self.read_config(root, ConfigRole::Schema)?;
        let destination_schema = match &config.bytes {
            Some(bytes) => weftext_data::Schema::from_author_json(&decode(bytes)?),
            None => weftext_data::Schema::bundled(),
        }
        .map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?;
        if serde_json::to_value(&source_schema)? != serde_json::to_value(&destination_schema)? {
            return Err(CoreError::new(
                "schemaMappingRequired",
                "The package's permanent schema IDs/shapes differ; explicit schema mapping is required before importing author sources",
            ));
        }
        let basis = self.snapshots[&config.base_id].clone();
        if let Some(captured) = basis.captured {
            dependencies.push(captured);
        } else {
            absent.push(AbsentDependency {
                path: basis.path,
                parent: basis.parent,
            });
        }

        let virtual_root = root
            .join(".weftext-package-input")
            .join(&package.manifest.package_id);
        let mut nodes = Vec::new();
        let mut resources = Vec::new();
        let mut node_map = BTreeMap::new();
        let mut revision_map = BTreeMap::new();
        let mut resource_map = BTreeMap::new();
        let mut resource_revision_map = BTreeMap::new();
        let mut annotation_map = BTreeMap::new();
        let mut definition_map = BTreeMap::new();
        let mut births = Vec::new();
        let mut owners = BTreeMap::new();
        let mut manifest_updates = BTreeMap::new();
        for (path, entry) in &package.manifest.entries {
            if entry.role != Role::Source {
                continue;
            }
            let owner = knowledge::NodeRef {
                workspace_id: package.manifest.workspace_id.clone(),
                node_id: entry.node_id.clone().unwrap(),
            };
            let new_owner = knowledge::NodeRef {
                workspace_id: workspace.workspace_id.clone(),
                node_id: fresh(),
            };
            let source = decode(&package.files[path])?;
            let inspected = weftext_engine::inspect_metadata(&source);
            if inspected.id.as_deref() != Some(owner.node_id.as_str()) {
                return Err(CoreError::new(
                    "packageIdentity",
                    "The manifest identity does not match the exact native root metadata",
                ));
            }
            let after_path = target.join(path);
            preview
                .node_map
                .insert(owner.node_id.clone(), new_owner.node_id.clone());
            node_map.insert(owner.clone(), new_owner.clone());
            births.push(NewBirth {
                path: display(&after_path),
                node_id: new_owner.node_id,
            });
            let revision = fresh();
            preview
                .revisions
                .insert(display(&after_path), revision.clone());
            revision_map.insert(owner.clone(), revision);
            match knowledge::definition_slots(&source, &owner) {
                Ok(slots) => {
                    for slot in slots {
                        definition_map.insert(slot.identity, fresh());
                    }
                }
                Err(_) if allow_losses => {}
                Err(e) => return Err(CoreError::new(&e.code, e.message)),
            }
            nodes.push(knowledge::NodeSnapshot {
                owner: owner.clone(),
                path: file_url(&virtual_root.join(path))?,
                library_path: path.clone(),
                revision_id: entry.revision.clone().unwrap_or_default(),
                source,
                title: inspected.title,
                aliases: vec![],
                labels: vec![],
                labels_complete: false,
            });
            owners.insert(path.clone(), owner);
        }
        for (path, entry) in &package.manifest.entries {
            if matches!(entry.role, Role::Resource | Role::Style) {
                let old_ref = format!(
                    "weftext-resource://{}/{}",
                    package.manifest.workspace_id,
                    entry.resource_id.as_ref().unwrap()
                );
                let new_ref = format!("weftext-resource://{}/{}", workspace.workspace_id, fresh());
                resource_map.insert(old_ref.clone(), new_ref.clone());
                preview
                    .resource_map
                    .insert(display(&target.join(path)), new_ref);
                let revision = fresh();
                preview
                    .revisions
                    .insert(display(&target.join(path)), revision.clone());
                resource_revision_map.insert(old_ref.clone(), revision);
                resources.push(knowledge::ResourceSnapshot {
                    key: old_ref,
                    path: file_url(&virtual_root.join(path))?,
                    revision_id: entry.revision.clone().unwrap_or_default(),
                });
            }
            if entry.role == Role::Annotations {
                let document = knowledge::decode_annotations(&package.files[path])
                    .map_err(|e| CoreError::new(&e.code, e.message))?;
                for record in document.records {
                    if annotation_map
                        .insert(record.annotation_id, fresh())
                        .is_some()
                    {
                        return Err(CoreError::new(
                            "duplicateAnnotation",
                            "Native package has duplicate annotation identities",
                        ));
                    }
                }
            }
        }
        let mut declared_resources = BTreeSet::new();
        for (path, entry) in &package.manifest.entries {
            if entry.role != Role::ResourceManifest {
                continue;
            }
            let mut value: serde_json::Value = serde_json::from_slice(&package.files[path])?;
            if value["version"] != 1 || !value["resources"].is_array() {
                return Err(CoreError::new(
                    "resourceManifest",
                    "Unsupported native resource manifest",
                ));
            }
            if value["resources"].as_array().unwrap().len() > 64 {
                return Err(CoreError::new(
                    "budget",
                    "Native Node resource registration exceeds the current 64-resource bound",
                ));
            }
            let owner_path = path
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or("");
            let owner_source = format!(
                "{owner_path}/{}.typ",
                owner_path.rsplit('/').next().unwrap_or("")
            );
            if owners.get(&owner_source).map(|owner| &owner.node_id) != entry.owner_node_id.as_ref()
            {
                return Err(CoreError::new(
                    "packageOwner",
                    "Resource manifest does not belong to its exact canonical source",
                ));
            }
            for resource in value["resources"].as_array_mut().unwrap() {
                let id = resource["resourceId"]
                    .as_str()
                    .ok_or_else(|| {
                        CoreError::new("resourceManifest", "Missing native resource identity")
                    })?
                    .to_string();
                let relative_path = resource["path"].as_str().ok_or_else(|| {
                    CoreError::new("resourceManifest", "Missing native resource path")
                })?;
                validate_member_path(relative_path).map_err(native_error)?;
                let resource_path = format!("{owner_path}/{relative_path}");
                let resource_entry =
                    package
                        .manifest
                        .entries
                        .get(&resource_path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "resourceMissing",
                                "Native manifest omits a registered resource",
                            )
                        })?;
                if !matches!(resource_entry.role, Role::Resource | Role::Style)
                    || resource_entry.resource_id.as_deref() != Some(&id)
                    || resource_entry.owner_node_id != entry.owner_node_id
                    || !declared_resources.insert(resource_path)
                {
                    return Err(CoreError::new(
                        "packageResource",
                        "Registered native resource binding is inconsistent",
                    ));
                }
                let old_ref = format!(
                    "weftext-resource://{}/{}",
                    package.manifest.workspace_id, id
                );
                resource["resourceId"] = resource_map[&old_ref].rsplit('/').next().unwrap().into();
            }
            manifest_updates.insert(
                display(&target.join(path)),
                serde_json::to_vec_pretty(&value)?,
            );
        }
        if declared_resources.len() != resources.len() {
            return Err(CoreError::new(
                "packageResource",
                "Every native resource must have one exact owned manifest registration",
            ));
        }
        let mut transfer = Vec::new();
        for (path, entry) in &package.manifest.entries {
            if matches!(entry.role, Role::Schema | Role::Settings) {
                continue;
            }
            let owner = owners.get(path).cloned();
            let kind = if owner.is_some() {
                knowledge::CopyFileKind::Source
            } else if entry.role == Role::Annotations {
                knowledge::CopyFileKind::Annotations
            } else {
                knowledge::CopyFileKind::Resource
            };
            let mut typed_refs = Vec::new();
            let mut definition_references = Vec::new();
            let mut complete = true;
            if let Some(owner) = &owner {
                let source = decode(&package.files[path])?;
                match weftext_data::typed_ref_closure(&source, &source_schema) {
                    Ok(closure) => {
                        complete = closure.complete;
                        typed_refs = closure
                            .slots
                            .into_iter()
                            .map(|slot| knowledge::TypedRefSlot {
                                from: slot.from,
                                to: slot.to,
                                target: slot.target,
                                kind: slot.kind,
                            })
                            .collect();
                    }
                    Err(_) => complete = false,
                }
                match knowledge::saved_query_reference_slots(&source, &source_schema, owner) {
                    Ok(slots) => {
                        typed_refs.extend(slots.typed_refs);
                        definition_references = slots.definition_references;
                    }
                    Err(_) => complete = false,
                }
                match weftext_engine::citation_work_slots(&source) {
                    Ok(slots) => {
                        typed_refs.extend(slots.into_iter().map(|slot| knowledge::TypedRefSlot {
                            from: slot.range.from,
                            to: slot.range.to,
                            target: slot.work,
                            kind: "node_ref".into(),
                        }))
                    }
                    Err(_) => complete = false,
                }
            }
            transfer.push(knowledge::CopyFile {
                source_path: file_url(&virtual_root.join(path))?,
                target_path: display(&target.join(path)),
                before: package.files[path].clone(),
                kind,
                owner,
                typed_refs,
                definition_references,
                typed_refs_complete: complete,
            });
        }
        let copy = knowledge::plan_copy(knowledge::CopyInput {
            files: transfer,
            node_map,
            resource_map,
            annotation_map,
            definition_map,
            revision_map,
            resource_revision_map,
            inventory: knowledge::InventorySnapshot {
                generation: preview.plan_id.clone(),
                coverage: knowledge::Coverage::Complete,
                nodes,
                resources,
            },
            allow_losses,
        })
        .map_err(|e| CoreError::new(&e.code, e.message))?;
        preview.losses = copy.losses;
        preview.losses.extend(
            package
                .manifest
                .losses
                .into_iter()
                .map(|loss| knowledge::Loss {
                    path: loss.path,
                    code: loss.code,
                    detail: loss.detail,
                }),
        );
        for (path, entry) in &package.manifest.entries {
            if matches!(entry.role, Role::Resource | Role::Style) && path.ends_with(".typ") {
                preview.losses.push(knowledge::Loss {
                    path: path.clone(), code: "resourceModuleContextPreserved".into(),
                    detail: "The Typst resource module is retained byte-for-byte. Application identities and root-relative dependencies inside resource modules are not rewritten; review its relocated context before use".into(),
                });
            }
        }
        if package
            .manifest
            .entries
            .values()
            .any(|entry| entry.role == Role::Settings)
        {
            preview.losses.push(knowledge::Loss {
                path: ".weftext-settings.json".into(), code: "settingsNotApplied".into(),
                detail: "Original settings and style selection remain in the preserved package; the destination library's settings remain authoritative. Imported style modules are owned resources, without automatic default-style activation".into(),
            });
        }
        if !allow_losses && !preview.losses.is_empty() {
            return Err(CoreError::new(
                "copyLoss",
                "Review the native import losses and explicitly choose allowLosses",
            ));
        }
        let mut output = copy
            .files
            .into_iter()
            .map(|file| FileChange {
                path: file.path,
                base_id: None,
                before: None,
                after: file.after,
            })
            .collect::<Vec<_>>();
        for file in &mut output {
            if let Some(bytes) = manifest_updates.remove(&file.path) {
                file.after = bytes;
            }
        }
        // Always preserve the complete original ZIP, including manifest/config,
        // source identities and unsupported discussions. It receives no admission.
        if let Some(original) = original_native {
            output.push(FileChange {
                path: display(&target.join(".weftext-import-originals/original.weftext")),
                base_id: None,
                before: None,
                after: original.to_vec(),
            });
        }
        let mut directories = BTreeSet::new();
        for file in &output {
            let relative = Path::new(&file.path)
                .strip_prefix(&target)
                .map_err(|_| CoreError::new("invalidPlan", "Native target escaped its new tree"))?;
            let mut parent = relative.parent();
            while let Some(path) = parent.filter(|p| !p.as_os_str().is_empty()) {
                directories.insert(display(path));
                parent = path.parent();
            }
        }
        if output.len() > 5000
            || output.iter().map(|f| f.after.len()).sum::<usize>() > 64 * 1024 * 1024
        {
            return Err(CoreError::new(
                "budget",
                "Native outputs plus retained original exceed the Core operation budget",
            ));
        }
        preview.changes.extend(
            output
                .iter()
                .map(|file| change(Path::new(&file.path), "import", file.after.len())),
        );
        Ok(Mutation::CreateTree {
            target: display(&target),
            directories: directories.into_iter().collect(),
            files: output,
            births,
            updates: vec![],
        })
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn native_library_round_trip_uses_fresh_core_identities_and_retains_original() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let origin = temp.path().join("Origin");
        let origin_info = core.create_library(&origin).unwrap();
        let note = core.create_note(&origin, &origin, "Note").unwrap();
        let original_id = note.node_id.clone().unwrap();
        let source = format!(
            "{}\r\n#link(\"weftext-node://{}/{}\")[Self]\r\n#read(\"data.txt\")",
            note.source, origin_info.workspace_id, original_id
        );
        core.save(SaveRequest {
            session_id: note.session_id,
            base_id: note.base_id,
            source: source.clone(),
            operation_id: fresh(),
        })
        .unwrap();
        let resource = core
            .prepare_import_resource(
                &origin,
                &note.path,
                "data.txt",
                b"exact\r\nresource".to_vec(),
                &fresh(),
            )
            .unwrap();
        core.execute_operation(&resource.operation_id, &resource.plan_id)
            .unwrap();
        let bytes = core.export_native_package(&origin).unwrap();
        let decoded = read_native_package(&bytes).unwrap();
        assert_eq!(decoded.files["Note/Note.typ"], source.as_bytes());
        assert!(!decoded.files.contains_key(".weftext-mvp-commit.json"));
        assert!(decoded.manifest.losses.is_empty());
        let destination = temp.path().join("Destination");
        let destination_info = core.create_library(&destination).unwrap();
        let operation_id = fresh();
        let preview = core
            .prepare_native_import(
                &destination,
                &destination,
                "Imported",
                bytes.clone(),
                &operation_id,
                false,
            )
            .unwrap();
        assert_eq!(preview.kind, "importNative");
        assert!(preview.losses.is_empty());
        assert_ne!(preview.node_map[&original_id], original_id);
        assert!(!destination.join("Imported").exists());
        let replay = core
            .prepare_native_import(
                &destination,
                &destination,
                "Imported",
                bytes.clone(),
                &operation_id,
                false,
            )
            .unwrap();
        assert_eq!(replay.plan_id, preview.plan_id);
        let receipt = core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(receipt.status, "committed");
        assert_eq!(
            fs::read(destination.join("Imported/.weftext-import-originals/original.weftext"))
                .unwrap(),
            bytes
        );
        let imported = core
            .open(
                destination.join("Imported/Note/Note.typ"),
                Some(destination.clone()),
            )
            .unwrap();
        assert_eq!(
            imported.node_id.as_ref(),
            Some(&preview.node_map[&original_id])
        );
        assert!(imported.source.contains(&format!(
            "weftext-node://{}/{}",
            destination_info.workspace_id,
            imported.node_id.unwrap()
        )));
        assert!(!imported.source.contains(&format!(
            "weftext-node://{}/{}",
            origin_info.workspace_id, original_id
        )));
        let resources = core
            .managed_resource_snapshot(&destination, &imported.path)
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].identity_status, "admitted");
        assert_eq!(resources[0].bytes, b"exact\r\nresource");
        let original_resource = decoded.manifest.entries["Note/data.txt"]
            .resource_id
            .as_ref()
            .unwrap();
        assert_ne!(&resources[0].resource_id, original_resource);
    }

    #[test]
    fn malformed_native_package_has_no_target_effect_and_settings_require_loss_choice() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let root = temp.path().join("Library");
        core.create_library(&root).unwrap();
        core.create_note(&root, &root, "Note").unwrap();
        let error = core
            .prepare_native_import(&root, &root, "Bad", b"not a zip".to_vec(), &fresh(), true)
            .unwrap_err();
        assert_eq!(error.code, "packageZip");
        assert!(!root.join("Bad").exists());
        let bytes = core.export_native_package(&root).unwrap();
        let mut package = read_native_package(&bytes).unwrap();
        let settings = b"{\"version\":1,\"defaultStyle\":{\"enabled\":false}}".to_vec();
        let entry = NativeEntry::new(Role::Settings, &settings);
        put(
            &mut package,
            ".weftext-settings.json".into(),
            settings,
            entry,
        )
        .unwrap();
        let with_settings = write_native_package(&package).unwrap();
        assert_eq!(
            core.prepare_native_import(
                &root,
                &root,
                "Reviewed",
                with_settings.clone(),
                &fresh(),
                false
            )
            .unwrap_err()
            .code,
            "copyLoss"
        );
        let preview = core
            .prepare_native_import(&root, &root, "Reviewed", with_settings, &fresh(), true)
            .unwrap();
        assert!(
            preview
                .losses
                .iter()
                .any(|loss| loss.code == "settingsNotApplied")
        );
        assert!(!root.join("Reviewed").exists());
        let mut schema: serde_json::Value =
            serde_json::from_slice(&package.files[".weftext-schema.json"]).unwrap();
        let type_id = schema["types"][0]["id"].clone();
        schema["aliases"]["ImportedAlternate"] = type_id;
        let schema_bytes = serde_json::to_vec(&schema).unwrap();
        package.manifest.entries.insert(
            ".weftext-schema.json".into(),
            NativeEntry::new(Role::Schema, &schema_bytes),
        );
        package
            .files
            .insert(".weftext-schema.json".into(), schema_bytes);
        assert_eq!(
            core.prepare_native_import(
                &root,
                &root,
                "Tampered",
                write_native_package(&package).unwrap(),
                &fresh(),
                true
            )
            .unwrap_err()
            .code,
            "schemaMappingRequired"
        );
        assert!(!root.join("Tampered").exists());
    }

    #[test]
    fn changed_destination_schema_cannot_execute_a_frozen_native_import() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let root = temp.path().join("Library");
        core.create_library(&root).unwrap();
        core.create_note(&root, &root, "Note").unwrap();
        let bytes = core.export_native_package(&root).unwrap();
        let preview = core
            .prepare_native_import(&root, &root, "Frozen", bytes, &fresh(), false)
            .unwrap();
        fs::write(
            root.join(".weftext-schema.json"),
            include_bytes!("../../../docs/design/examples/schema.authoring.json"),
        )
        .unwrap();
        assert_eq!(
            core.execute_operation(&preview.operation_id, &preview.plan_id)
                .unwrap_err()
                .code,
            "scopeChanged"
        );
        assert!(!root.join("Frozen").exists());
    }

    #[test]
    fn bibliography_import_creates_independent_real_paper_works_and_owned_originals() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let root = temp.path().join("Library");
        core.create_library(&root).unwrap();
        let upload = br#"[{"type":"article-journal","id":"sameDOI","title":"First","DOI":"10.1234/example","author":[{"literal":"One"},{"literal":"Two"}],"issued":{"date-parts":[[2026,9]]}},{"type":"article-journal","id":"sameDOI","title":"Second","DOI":"10.1234/example","author":[{"literal":"Three"}]}]"#.to_vec();
        let operation_id = fresh();
        let preview = core
            .prepare_bibliography_import(
                &root,
                &root,
                "Papers",
                "cslJson",
                upload.clone(),
                &operation_id,
                false,
            )
            .unwrap();
        assert_eq!(preview.kind, "importBibliography");
        assert_eq!(preview.node_map.len(), 2);
        assert!(preview.losses.is_empty());
        assert!(!root.join("Papers").exists());
        let repeated = core
            .prepare_bibliography_import(
                &root,
                &root,
                "Papers",
                "cslJson",
                upload.clone(),
                &operation_id,
                false,
            )
            .unwrap();
        assert_eq!(preview.plan_id, repeated.plan_id);
        let receipt = core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(receipt.node_ids.len(), 2);
        assert_ne!(receipt.node_ids[0], receipt.node_ids[1]);
        let first = core
            .open(
                root.join("Papers/paper-0001/paper-0001.typ"),
                Some(root.clone()),
            )
            .unwrap();
        let second = core
            .open(
                root.join("Papers/paper-0002/paper-0002.typ"),
                Some(root.clone()),
            )
            .unwrap();
        assert_ne!(first.node_id, second.node_id);
        let schema = weftext_data::Schema::bundled().unwrap();
        for note in [&first, &second] {
            let projected = weftext_data::project(&note.source, &schema);
            assert_eq!(projected.qualification, "qualified");
            assert!(projected.resolved_types.contains(&"library/paper".into()));
            assert!(projected.resolved_types.contains(&"library/work".into()));
            let resources = core.managed_resource_snapshot(&root, &note.path).unwrap();
            assert!(
                resources
                    .iter()
                    .all(|resource| resource.identity_status == "admitted")
            );
            let closure = weftext_data::typed_ref_closure(&note.source, &schema).unwrap();
            assert!(closure.complete);
            assert!(
                closure
                    .slots
                    .iter()
                    .filter(|slot| slot.kind == "resource_ref")
                    .all(|slot| resources
                        .iter()
                        .any(|resource| resource.resource_ref == slot.target))
            );
        }
        assert_eq!(
            fs::read(root.join("Papers/paper-0001/original.csl.json")).unwrap(),
            upload
        );
        assert!(
            !root
                .join("Papers/.weftext-import-originals/original.weftext")
                .exists()
        );
        let replay = core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(replay.node_ids, receipt.node_ids);
        let other = core
            .prepare_bibliography_import(
                &root,
                &root,
                "More Papers",
                "cslJson",
                upload,
                &fresh(),
                false,
            )
            .unwrap();
        assert!(
            other
                .node_map
                .values()
                .all(|id| !receipt.node_ids.contains(id))
        );
    }
}
