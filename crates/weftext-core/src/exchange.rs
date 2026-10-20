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
mod native_export;
pub use native_export::{NativeExport, NativeExportWitness};

fn native_error(error: weftext_export::exchange::NativeError) -> CoreError {
    CoreError::new(&error.code, error.message)
}
#[cfg(test)]
thread_local! {
    // Tests use this only to reproduce previously valid native inputs. It skips
    // the new role/name preflight, never original execution or recovery guards.
    static LEGACY_NATIVE_MARKER_INPUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
fn validate_native_navigation_roles(package: &NativePackage) -> Result<()> {
    #[cfg(test)]
    if LEGACY_NATIVE_MARKER_INPUT.with(std::cell::Cell::get) {
        return Ok(());
    }
    if package.manifest.entries.iter().any(|(path, entry)| {
        matches!(entry.role, Role::Resource | Role::Style)
            && path
                .rsplit('/')
                .next()
                .is_some_and(|leaf| leaf.eq_ignore_ascii_case("weftext.directory.json"))
    }) {
        return Err(CoreError::new(
            "nativeControlConflict",
            "A native Resource or Style cannot occupy the reserved directory marker name; retain the original package and explicitly rename that resource before importing",
        ));
    }
    Ok(())
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
        self.capture_native_export(root).map(|export| export.bytes)
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
    pub(crate) fn plan_docx_import(
        &mut self,
        root: &Path,
        parent: &Path,
        name: &str,
        bytes: &[u8],
        allow_losses: bool,
        options: Option<&weftext_export::docx::DocxImportOptions>,
        originals: &[weftext_export::docx::OriginalImportAsset],
        conversion_losses: &[weftext_export::exchange::NativeLoss],
        preview: &mut OperationPreview,
        dependencies: &mut Vec<CapturedFile>,
        absent: &mut Vec<AbsentDependency>,
    ) -> Result<Mutation> {
        let config = self.read_config(root, ConfigRole::Schema)?;
        let schema = match config.bytes {
            Some(bytes) => decode(&bytes)?,
            None => include_str!("../../../docs/design/examples/schema.authoring.json").into(),
        };
        let mut package = match options {
            Some(options) => weftext_export::docx::build_docx_native_package_with_options(
                bytes,
                &schema,
                &preview.operation_id,
                options,
            ),
            None => weftext_export::docx::build_docx_native_package(
                bytes,
                &schema,
                &preview.operation_id,
            ),
        }
        .map_err(|e| CoreError::new("docxImport", e.to_string()))?;
        weftext_export::docx::retain_docx_import_originals(
            &mut package,
            &preview.operation_id,
            originals,
            conversion_losses,
        )
        .map_err(|e| CoreError::new("docxImport", e.to_string()))?;
        let mut mutation = self.plan_content_import(
            root,
            parent,
            name,
            package,
            None,
            allow_losses,
            preview,
            dependencies,
            absent,
        )?;
        // This producer authored the exact new source/targets together. Rebind only
        // anchors whose full source preimage survived Core's verified copy map.
        if let Mutation::CreateTree { files, .. } = &mut mutation {
            let sources: BTreeMap<_, _> = files
                .iter()
                .filter(|f| f.path.ends_with(".typ"))
                .map(|f| (f.path.clone(), f.after.clone()))
                .collect();
            for file in files
                .iter_mut()
                .filter(|f| f.path.ends_with("weftext.annotations.json"))
            {
                let mut data: serde_json::Value = serde_json::from_slice(&file.after)?;
                for record in data["annotations"].as_array_mut().ok_or_else(|| {
                    CoreError::new("docxAnnotations", "Missing annotation records")
                })? {
                    if record["suggestion"].is_null() || record.get("copyTargetState").is_some() {
                        continue;
                    }
                    let anchor: Option<knowledge::SourceAnchor> =
                        serde_json::from_value(record["target"]["anchor"].clone()).ok();
                    if let Some(anchor) = anchor
                        && sources.iter().any(|(path, bytes)| {
                            preview.revisions.get(path) == Some(&anchor.revision_id)
                                && std::str::from_utf8(bytes)
                                    .is_ok_and(|source| anchor.source_range.verify(source))
                        })
                    {
                        record["suggestion"]["confirmedRevision"] = anchor.revision_id.into();
                        record["suggestion"]["requiresReconfirmation"] = false.into();
                    }
                }
                file.after = serde_json::to_vec_pretty(&data)?;
                if let Some(change) = preview
                    .changes
                    .iter_mut()
                    .find(|change| change.path == file.path)
                {
                    change.bytes = file.after.len();
                }
                knowledge::decode_annotations(&file.after)
                    .map_err(|e| CoreError::new(&e.code, e.message))?;
            }
        }
        Ok(mutation)
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
    pub(crate) fn plan_content_import(
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
        validate_native_navigation_roles(&package)?;
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
            if matches!(
                entry.role,
                Role::Schema | Role::Settings | Role::GeneratedBibliography
            ) {
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
            if entry.role == Role::GeneratedBibliography {
                preview.losses.push(knowledge::Loss {
                    path: path.clone(), code: "generatedBibliographyNotInstalled".into(),
                    detail: "The derived bibliography remains only in the retained original ZIP and is absent from the installed author tree. Regenerate it from the remapped, currently authorized Work data before compiling offline citations".into(),
                });
            }
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
        // The imported wrapper and ordinary ancestors are new Directory objects.
        // Allocate their markers in this same CreateTree; the original creation
        // handles supply admission, and publication can register parents before
        // their Node children. Resource-only folders remain owned Node members.
        let node_directories: BTreeSet<_> = births
            .iter()
            .map(|birth| Path::new(&birth.path).parent().unwrap().to_path_buf())
            .collect();
        let mut navigation_directories = BTreeSet::from([target.clone()]);
        for directory in &node_directories {
            let mut ancestor = directory.parent();
            while let Some(path) = ancestor.filter(|path| path.starts_with(&target)) {
                if !node_directories.contains(path) {
                    navigation_directories.insert(path.to_path_buf());
                }
                ancestor = path.parent();
            }
        }
        for directory in navigation_directories {
            output.push(FileChange {
                path: display(&directory.join("weftext.directory.json")),
                base_id: None,
                before: None,
                after: serde_json::to_vec_pretty(&serde_json::json!({
                    "directoryId": fresh(),
                }))?,
            });
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
        // Every portable output, including navigation markers and the retained
        // original, needs its original physical observation under the exact
        // revision published by this same Plan.
        for file in &output {
            preview
                .revisions
                .entry(file.path.clone())
                .or_insert_with(fresh);
        }
        preview.changes.extend(
            output
                .iter()
                .map(|file| change(Path::new(&file.path), "import", file.after.len())),
        );
        Ok(Mutation::CreateTree {
            existing_target: false,
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

    fn native_marker_resource_is_not_a_directory_birth(style: bool, placement: u8, alias: bool) {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let origin = temp.path().join("Origin");
        core.create_library(&origin).unwrap();
        let parent = core.create_note(&origin, &origin, "Parent").unwrap();
        if placement == 2 {
            core.create_note(&origin, origin.join("Parent"), "Child")
                .unwrap();
        }
        let claimed = fresh();
        let bytes = serde_json::to_vec(&serde_json::json!({"directoryId":claimed})).unwrap();
        let leaf = if alias {
            "WEFTEXT.DIRECTORY.JSON"
        } else {
            "weftext.directory.json"
        };
        let resource = core
            .prepare_import_resource(&origin, &parent.path, leaf, bytes.clone(), &fresh())
            .unwrap();
        core.execute_operation(&resource.operation_id, &resource.plan_id)
            .unwrap();
        // Start with the real owned-resource producer and export. A native
        // package may place owned resources below nested folders and may carry
        // Style roles; these remain untrusted author content, not directory
        // allocation requests. The standard package validator checks this input.
        let mut package =
            read_native_package(&core.export_native_package(&origin).unwrap()).unwrap();
        let old_path = format!("Parent/{leaf}");
        let target_path = if placement == 0 {
            old_path.clone()
        } else {
            format!("Parent/assets/{leaf}")
        };
        let mut entry = package.manifest.entries.remove(&old_path).unwrap();
        if style {
            entry.role = Role::Style;
        }
        package.manifest.entries.insert(target_path.clone(), entry);
        let payload = package.files.remove(&old_path).unwrap();
        package.files.insert(target_path.clone(), payload);
        if placement != 0 {
            let path = "Parent/weftext.resources.json";
            let mut value: serde_json::Value =
                serde_json::from_slice(&package.files[path]).unwrap();
            value["resources"][0]["path"] = format!("assets/{leaf}").into();
            let after = serde_json::to_vec(&value).unwrap();
            let mut entry = NativeEntry::new(Role::ResourceManifest, &after);
            entry.owner_node_id = package.manifest.entries[path].owner_node_id.clone();
            entry.revision = Some(fresh());
            package.manifest.entries.insert(path.into(), entry);
            package.files.insert(path.into(), after);
        }
        if placement == 2 {
            let members = package
                .files
                .keys()
                .filter(|path| path.starts_with("Parent/Child/"))
                .cloned()
                .collect::<Vec<_>>();
            for path in members {
                let target = path.replacen("Parent/Child/", "Parent/assets/Child/", 1);
                let entry = package.manifest.entries.remove(&path).unwrap();
                let bytes = package.files.remove(&path).unwrap();
                package.manifest.entries.insert(target.clone(), entry);
                package.files.insert(target, bytes);
            }
        }
        let package = write_native_package(&package).unwrap();
        let root = temp.path().join("Library");
        core.create_library(&root).unwrap();
        let operation = fresh();
        let error = core
            .prepare_native_import(&root, &root, "Imported", package.clone(), &operation, false)
            .unwrap_err();
        assert_eq!(error.code, "nativeControlConflict");
        assert!(!root.join("Imported").exists());
        assert!(!core.journal_path(&operation, "install").exists());
        assert_eq!(fs::read(origin.join(&old_path)).unwrap(), bytes);
        assert_eq!(
            read_native_package(&package).unwrap().files[&target_path],
            bytes
        );
    }

    #[test]
    fn native_marker_resource_at_node_root_cannot_allocate_a_directory() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(false, 0, alias);
        }
    }
    #[test]
    fn native_marker_style_at_node_root_cannot_allocate_a_directory() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(true, 0, alias);
        }
    }
    #[test]
    fn native_marker_resource_in_owned_folder_cannot_allocate_a_directory() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(false, 1, alias);
        }
    }
    #[test]
    fn native_marker_style_in_owned_folder_cannot_allocate_a_directory() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(true, 1, alias);
        }
    }
    #[test]
    fn native_marker_resource_at_nested_node_ancestor_cannot_replace_allocated_marker() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(false, 2, alias);
        }
    }
    #[test]
    fn native_marker_style_at_nested_node_ancestor_cannot_replace_allocated_marker() {
        for alias in [false, true] {
            native_marker_resource_is_not_a_directory_birth(true, 2, alias);
        }
    }

    fn legacy_native_marker_resource_keeps_its_original_role(
        style: bool,
        nested: bool,
        recover: bool,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let private = temp.path().join("private");
        let mut core = Core::new(&private).unwrap();
        let origin = temp.path().join("Origin");
        core.create_library(&origin).unwrap();
        let parent = core.create_note(&origin, &origin, "Parent").unwrap();
        let claimed = fresh();
        let bytes = serde_json::to_vec(&serde_json::json!({"directoryId":claimed})).unwrap();
        let resource = core
            .prepare_import_resource(
                &origin,
                &parent.path,
                "weftext.directory.json",
                bytes.clone(),
                &fresh(),
            )
            .unwrap();
        core.execute_operation(&resource.operation_id, &resource.plan_id)
            .unwrap();
        let mut package =
            read_native_package(&core.export_native_package(&origin).unwrap()).unwrap();
        let path = if nested {
            "Parent/assets/weftext.directory.json"
        } else {
            "Parent/weftext.directory.json"
        };
        let mut entry = package
            .manifest
            .entries
            .remove("Parent/weftext.directory.json")
            .unwrap();
        if style {
            entry.role = Role::Style;
        }
        let payload = package
            .files
            .remove("Parent/weftext.directory.json")
            .unwrap();
        package.files.insert(path.into(), payload);
        package.manifest.entries.insert(path.into(), entry);
        if nested {
            let manifest = "Parent/weftext.resources.json";
            let mut value: serde_json::Value =
                serde_json::from_slice(&package.files[manifest]).unwrap();
            value["resources"][0]["path"] = "assets/weftext.directory.json".into();
            let after = serde_json::to_vec(&value).unwrap();
            let mut entry = NativeEntry::new(Role::ResourceManifest, &after);
            entry.owner_node_id = package.manifest.entries[manifest].owner_node_id.clone();
            entry.revision = Some(fresh());
            package.files.insert(manifest.into(), after);
            package.manifest.entries.insert(manifest.into(), entry);
        }
        let package = write_native_package(&package).unwrap();
        let root = temp.path().join("Library");
        let workspace = core.create_library(&root).unwrap().workspace_id;
        // Reproduce a previously allowed input with the same real public
        // producer, creation handles and original Plan. Only the newly added
        // preflight is disabled during fixture preparation; no API/env flag or
        // execution/recovery/identity bypass exists in a product build.
        LEGACY_NATIVE_MARKER_INPUT.with(|flag| flag.set(true));
        let prepared =
            core.prepare_native_import(&root, &root, "Imported", package.clone(), &fresh(), false);
        LEGACY_NATIVE_MARKER_INPUT.with(|flag| flag.set(false));
        let preview = prepared.unwrap();
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        if recover {
            fs::rename(
                core.journal_path(&preview.operation_id, "commit"),
                temp.path().join("retained-original-commit"),
            )
            .unwrap();
        }
        drop(core);
        let mut core = Core::new(&private).unwrap();
        if recover {
            let inspection = core.inspect_recovery(&preview.operation_id).unwrap();
            assert_eq!(inspection.coherence, "knownAfter");
            core.resolve_recovery(&preview.operation_id, "adoptAfterImages")
                .unwrap();
        }
        let resource_path = root.join("Imported").join(path);
        let physical = ParentGuard::acquire(&resource_path)
            .unwrap()
            .directory_identity()
            .unwrap();
        assert!(
            !core
                .directory_admitted(&workspace, &claimed, &physical)
                .unwrap()
        );
        assert!(
            core.directory_admission_candidates(&workspace)
                .unwrap()
                .iter()
                .all(|(_, id, _)| id != &claimed)
        );
        let claimed_ref = format!("weftext-directory://{workspace}/{claimed}");
        assert_ne!(
            core.resolve_navigation_target(&root, &claimed_ref)
                .unwrap()
                .status,
            "managed"
        );
        let marker: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join("Imported/weftext.directory.json")).unwrap(),
        )
        .unwrap();
        let wrapper = format!(
            "weftext-directory://{workspace}/{}",
            marker["directoryId"].as_str().unwrap()
        );
        assert_eq!(
            core.resolve_navigation_target(&root, &wrapper)
                .unwrap()
                .status,
            "managed"
        );
        assert_eq!(fs::read(&resource_path).unwrap(), bytes);
        assert_eq!(
            fs::read(root.join("Imported/.weftext-import-originals/original.weftext")).unwrap(),
            package
        );
        let resources = core
            .managed_resource_snapshot(&root, root.join("Imported/Parent/Parent.typ"))
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].identity_status, "admitted");
        assert_eq!(resources[0].bytes, bytes);
        assert!(
            preview
                .resource_map
                .values()
                .any(|reference| reference == &resources[0].resource_ref)
        );
        assert!(core.committed(&preview.operation_id));
    }

    #[test]
    fn legacy_native_marker_resource_at_node_root_never_gains_navigation_after_commit_or_recovery()
    {
        for recover in [false, true] {
            legacy_native_marker_resource_keeps_its_original_role(false, false, recover);
        }
    }
    #[test]
    fn legacy_native_marker_style_at_node_root_never_gains_navigation_after_commit_or_recovery() {
        for recover in [false, true] {
            legacy_native_marker_resource_keeps_its_original_role(true, false, recover);
        }
    }
    #[test]
    fn legacy_native_marker_resource_in_owned_folder_never_gains_navigation_after_commit_or_recovery()
     {
        for recover in [false, true] {
            legacy_native_marker_resource_keeps_its_original_role(false, true, recover);
        }
    }
    #[test]
    fn legacy_native_marker_style_in_owned_folder_never_gains_navigation_after_commit_or_recovery()
    {
        for recover in [false, true] {
            legacy_native_marker_resource_keeps_its_original_role(true, true, recover);
        }
    }

    #[test]
    fn native_import_after_sync_enabled_admits_wrapper_in_original_plan_and_clones_its_typed_parent()
     {
        let temp = tempfile::tempdir().unwrap();
        let private = temp.path().join("private");
        let mut core = Core::new(&private).unwrap();
        let origin = temp.path().join("Origin");
        core.create_library(&origin).unwrap();
        let original = core.create_note(&origin, &origin, "Note").unwrap();
        let imported = core
            .prepare_import_resource(
                &origin,
                &original.path,
                "data.bin",
                vec![0, 255, 7],
                &fresh(),
            )
            .unwrap();
        core.execute_operation(&imported.operation_id, &imported.plan_id)
            .unwrap();
        let package = core.export_native_package(&origin).unwrap();
        let root = temp.path().join("Library");
        core.create_library(&root).unwrap();
        let identity = core.enable_sync(&root, &fresh()).unwrap().identity;
        let request = OperationRequest {
            operation_id: fresh(),
            root: display(&root),
            intent: OperationIntent::ImportNative {
                parent: display(&root),
                name: "Imported".into(),
                bytes: package.clone(),
                allow_losses: false,
            },
        };
        let preview = core.prepare_operation(request.clone()).unwrap();
        let node_id = preview.node_map[original.node_id.as_ref().unwrap()].clone();
        assert_ne!(Some(&node_id), original.node_id.as_ref());
        let transaction = core.operations[&preview.operation_id]
            .transaction
            .as_ref()
            .unwrap();
        let wrapper_marker = transaction
            .created_files()
            .iter()
            .find(|file| Path::new(&file.path) == root.join("Imported/weftext.directory.json"))
            .unwrap();
        let marker: serde_json::Value = serde_json::from_slice(&wrapper_marker.after).unwrap();
        let directory_id = marker["directoryId"].as_str().unwrap().to_owned();
        let directory = crate::sync::ObjectKey::Directory(directory_id.clone());
        let node = crate::sync::ObjectKey::Node(node_id.clone());
        let publication = core.operations[&preview.operation_id]
            .sync_publication
            .as_ref()
            .unwrap();
        assert!(publication.manifest.changes.iter().any(|change| {
            change.object == node && change.after.parent.as_ref() == Some(&directory)
        }));
        assert!(!root.join("Imported").exists());
        drop(core);
        let mut core = Core::new(&private).unwrap();
        let repeated = core.prepare_operation(request).unwrap();
        assert_eq!(repeated.plan_id, preview.plan_id);
        assert_eq!(repeated.node_map, preview.node_map);
        assert_eq!(repeated.resource_map, preview.resource_map);
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(
            fs::read(root.join("Imported/.weftext-import-originals/original.weftext")).unwrap(),
            package
        );
        let reference = format!(
            "weftext-directory://{}/{directory_id}",
            identity.workspace_id
        );
        assert_eq!(
            core.resolve_navigation_target(&root, &reference)
                .unwrap()
                .status,
            "managed"
        );
        assert!(
            core.sync_binding_info(&root)
                .unwrap()
                .installed_objects
                .contains(&directory)
        );
        core.publish_sync(&root).unwrap();
        let original_cut = BTreeSet::from([node.clone(), directory.clone()]);
        assert_eq!(
            core.read_sync_adoption_cut(&root, &original_cut)
                .unwrap_err()
                .code,
            "noChange"
        );
        drop(core);
        let mut core = Core::new(&private).unwrap();
        assert_eq!(
            core.read_sync_adoption_cut(&root, &original_cut)
                .unwrap_err()
                .code,
            "noChange"
        );
        let selected = BTreeSet::from([node]);
        let target = temp.path().join("Replica");
        let mut receiver = Core::new(temp.path().join("receiver-private")).unwrap();
        receiver
            .receive_sync(
                &identity,
                core.export_sync_objects(&identity, &selected).unwrap(),
            )
            .unwrap();
        let clone = receiver
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&target),
                intent: OperationIntent::CloneSync {
                    identity,
                    objects: selected,
                },
            })
            .unwrap();
        receiver
            .execute_operation(&clone.operation_id, &clone.plan_id)
            .unwrap();
        let source = receiver
            .open(target.join("Imported/Note/Note.typ"), Some(target.clone()))
            .unwrap();
        assert_eq!(source.node_id.as_ref(), Some(&node_id));
        assert_eq!(
            fs::read(&source.path).unwrap(),
            fs::read(root.join("Imported/Note/Note.typ")).unwrap()
        );
        assert_eq!(
            receiver
                .resolve_navigation_target(&target, &reference)
                .unwrap()
                .status,
            "managed"
        );
        let resources = receiver
            .managed_resource_snapshot(&target, &source.path)
            .unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].identity_status, "admitted");
        assert_eq!(resources[0].bytes, [0, 255, 7]);
        assert!(
            preview
                .resource_map
                .values()
                .any(|reference| reference == &resources[0].resource_ref)
        );
        let replay = core
            .execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert_eq!(replay.node_ids, vec![node_id]);
    }

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
    fn native_generated_bibliography_import_retains_original_without_author_resource_install() {
        let temp = tempfile::tempdir().unwrap();
        let mut core = Core::new(temp.path().join("private")).unwrap();
        let origin = temp.path().join("Origin");
        core.create_library(&origin).unwrap();
        let note = core.create_note(&origin, &origin, "Note").unwrap();
        let mut package =
            read_native_package(&core.export_native_package(&origin).unwrap()).unwrap();
        weftext_export::exchange::append_generated_bibliography(
            &mut package,
            note.node_id.as_deref().unwrap(),
            b"@article{work2026, title = {Exact title}, year = {2026}}\r\n".to_vec(),
        )
        .unwrap();
        let bytes = write_native_package(&package).unwrap();
        let destination = temp.path().join("Destination");
        core.create_library(&destination).unwrap();
        assert_eq!(
            core.prepare_native_import(
                &destination,
                &destination,
                "Imported",
                bytes.clone(),
                &fresh(),
                false
            )
            .unwrap_err()
            .code,
            "copyLoss"
        );
        assert!(!destination.join("Imported").exists());
        let preview = core
            .prepare_native_import(
                &destination,
                &destination,
                "Imported",
                bytes.clone(),
                &fresh(),
                true,
            )
            .unwrap();
        assert!(
            preview
                .losses
                .iter()
                .any(|loss| loss.code == "generatedBibliographyNotInstalled"
                    && loss.path == "Note/weftext.citations.bib")
        );
        assert!(preview.resource_map.is_empty());
        core.execute_operation(&preview.operation_id, &preview.plan_id)
            .unwrap();
        assert!(
            !destination
                .join("Imported/Note/weftext.citations.bib")
                .exists()
        );
        assert!(
            !destination
                .join("Imported/Note/weftext.resources.json")
                .exists()
        );
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
        assert_eq!(imported.node_id.as_ref(), preview.node_map.values().next());
        assert!(
            core.managed_resource_snapshot(&destination, &imported.path)
                .unwrap()
                .is_empty()
        );
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
