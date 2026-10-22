//! One owned native export cut. The host must authorize whole live-library
//! read/export and omission-path disclosure before entering this producer.
//! Namespace enumeration never reads omitted file bodies. Directory guards
//! prove physical placement and observed membership, not arbitrary future raw
//! absence after the last validation has returned.
use super::*;
use crate::operations::{
    CapturedDirectory, SnapshotBinding, read_blob, resource_path, sorted_entries,
};
use weftext_messages::UiMessage;

const MAX_MEMBERS: usize = 5000;
const MAX_BYTES: usize = 64 * 1024 * 1024;

/// ZIP bytes, their original manifest and the witness of the same input capture.
/// A host may derive another package purely from these bytes, but must retain
/// this witness and independently authorize every additional input it uses.
pub struct NativeExport {
    pub bytes: Vec<u8>,
    pub manifest: NativeManifest,
    pub witness: NativeExportWitness,
}

/// Owned, nonportable read evidence. It grants neither reading nor writing and
/// is not a backup, persistent ledger, or strong negative filesystem scope.
pub struct NativeExportWitness {
    root: PathBuf,
    workspace_id: String,
    admissions: Vec<(String, FileIdentity)>,
    files: Vec<SnapshotWitness>,
    directories: Vec<CapturedDirectory>,
    observations: Vec<ObservedFile>,
    omitted_directories: Vec<(String, FileIdentity)>,
    resources: Vec<ResourceBinding>,
}
struct ObservedFile {
    path: String,
    parent: FileIdentity,
    stamp: FileStamp,
}
struct NativeNamespace {
    directories: Vec<CapturedDirectory>,
    observations: Vec<ObservedFile>,
    omitted_directories: Vec<(String, FileIdentity)>,
}
struct ResourceBinding {
    path: String,
    node_id: String,
    resource_id: String,
}
impl NativeExportWitness {
    /// Account all retained input bytes and owned namespace/binding metadata.
    /// The caller separately charges NativeExport.bytes and its public manifest.
    pub fn retained_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.root.capacity() * 2
            + self.workspace_id.capacity()
            + self.admissions.capacity() * std::mem::size_of::<(String, FileIdentity)>()
            + self
                .admissions
                .iter()
                .map(|(id, _)| id.capacity())
                .sum::<usize>()
            + self.files.capacity() * std::mem::size_of::<SnapshotWitness>()
            + self
                .files
                .iter()
                .map(SnapshotWitness::retained_bytes)
                .sum::<usize>()
            + self.directories.capacity() * std::mem::size_of::<CapturedDirectory>()
            + self
                .directories
                .iter()
                .map(|d| {
                    d.path.capacity()
                        + d.entries.capacity() * std::mem::size_of::<String>()
                        + d.entries.iter().map(String::capacity).sum::<usize>()
                })
                .sum::<usize>()
            + self.observations.capacity() * std::mem::size_of::<ObservedFile>()
            + self
                .observations
                .iter()
                .map(|f| f.path.capacity())
                .sum::<usize>()
            + self.omitted_directories.capacity() * std::mem::size_of::<(String, FileIdentity)>()
            + self
                .omitted_directories
                .iter()
                .map(|(path, _)| path.capacity())
                .sum::<usize>()
            + self.resources.capacity() * std::mem::size_of::<ResourceBinding>()
            + self
                .resources
                .iter()
                .map(|r| r.path.capacity() + r.node_id.capacity() + r.resource_id.capacity())
                .sum::<usize>()
    }
}

impl Core {
    /// Capture admitted live content, owned resources/annotations and exact
    /// config once, then build a ZIP from those same retained bytes. This does
    /// not invoke ambient inventory, read raw/ignored/Trash bodies or retain
    /// read bases in Core. Private internal subtrees are omitted at their roots.
    pub fn capture_native_export(&mut self, root: impl AsRef<Path>) -> Result<NativeExport> {
        let root = absolute_path(root.as_ref())?;
        let workspace_id = self.require_workspace(&root)?.workspace_id;
        let admissions = self.native_export_admissions(&workspace_id)?;
        let NativeNamespace {
            directories,
            observations,
            omitted_directories,
        } = self.native_export_namespace(&root)?;
        let mut witness = NativeExportWitness {
            root: root.clone(),
            workspace_id: workspace_id.clone(),
            admissions,
            files: vec![],
            directories,
            observations,
            omitted_directories,
            resources: vec![],
        };
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
            ".weftext-rules",
            ".weftext-schema.json",
            ".weftext-settings.json",
            ".weftext-tags.json",
        ] {
            let input = self.native_export_file(&witness, &root.join(name))?;
            if name == ".weftext-rules" {
                match &input.binding.captured {
                    Some(file) => crate::validate_library_rules(&file.bytes)?,
                    None if self.query_rules_ever_committed(&root)? => {
                        return Err(CoreError::new(
                            "rulesUnavailable",
                            UiMessage::error_core_exchange_previously_committed_library_rules_missing(),
                        ));
                    }
                    None => {}
                }
            }
            witness.files.push(input);
            native_input_budget(&witness)?;
        }
        let mut package = NativePackage {
            manifest: NativeManifest {
                format: NATIVE_FORMAT.into(),
                version: NATIVE_VERSION,
                package_id: fresh(),
                workspace_id,
                scope: "library".into(),
                schema_path: ".weftext-schema.json".into(),
                entries: BTreeMap::new(),
                losses: vec![],
            },
            files: BTreeMap::new(),
        };
        for (node_id, identity) in &witness.admissions.clone() {
            let matches = witness
                .observations
                .iter()
                .filter(|f| &f.stamp.identity == identity)
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err(CoreError::new(
                    "coverageIncomplete",
                    UiMessage::error_core_exchange_admitted_node_has_no_unique_current_physical_member(),
                ));
            }
            let member = matches[0];
            let source_path = PathBuf::from(&member.path);
            let birth = Birth {
                node_id: node_id.clone(),
                workspace_id: witness.workspace_id.clone(),
                identity: identity.clone(),
            };
            let places = self.node_placement_paths(&birth)?;
            if places.len() != 1
                || !places
                    .iter()
                    .any(|path| same_path(Path::new(path), &source_path))
            {
                return Err(CoreError::new(
                    "coverageIncomplete",
                    UiMessage::error_core_exchange_admitted_node_lacks_exact_original_placement(),
                ));
            }
            if is_trash(&root, &source_path)? {
                self.native_export_trash_member(&witness, &source_path)?;
                continue;
            }
            native_incoming_budget(&witness, member.stamp.size)?;
            let qualified = self.qualified_node_at(&root, &source_path)?;
            if qualified.owner.node_id != *node_id
                || qualified.captured_file().stamp != member.stamp
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_admitted_live_member_changed_during_capture(),
                ));
            }
            let source = qualified.captured_file();
            let mut entry = NativeEntry::new(Role::Source, &source.bytes);
            entry.node_id = Some(node_id.clone());
            entry.revision = qualified.revision.clone();
            put(
                &mut package,
                relative(&root, &source.path)?,
                source.bytes.clone(),
                entry,
            )?;
            let owner = source_path.parent().unwrap();
            witness.files.push(qualified.witness);
            native_input_budget(&witness)?;
            let manifest =
                self.native_export_file(&witness, &owner.join("weftext.resources.json"))?;
            if let Some(file) = &manifest.binding.captured {
                let mut entry = NativeEntry::new(Role::ResourceManifest, &file.bytes);
                entry.owner_node_id = Some(node_id.clone());
                put(
                    &mut package,
                    relative(&root, &file.path)?,
                    file.bytes.clone(),
                    entry,
                )?;
                let data: serde_json::Value = serde_json::from_slice(&file.bytes)?;
                if data["version"] != 1 {
                    return Err(CoreError::new(
                        "resourceManifest",
                        UiMessage::error_core_operations_unsupported_resource_manifest_version(),
                    ));
                }
                let resources = data["resources"].as_array().ok_or_else(|| {
                    CoreError::new(
                        "resourceManifest",
                        UiMessage::error_core_operations_missing_resources_array(),
                    )
                })?;
                if resources.len() > 64 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_exchange_at_most64_registered_resources_per_owner(),
                    ));
                }
                let mut ids = BTreeSet::new();
                for resource in resources {
                    let id = resource["resourceId"].as_str().ok_or_else(|| {
                        CoreError::new(
                            "resourceManifest",
                            UiMessage::error_core_operations_missing_resource_i_d(),
                        )
                    })?;
                    valid_operation_id(id)?;
                    if !ids.insert(id) {
                        return Err(CoreError::new(
                            "resourceManifest",
                            UiMessage::error_core_operations_duplicate_resource_i_d(),
                        ));
                    }
                    let name = resource["path"].as_str().ok_or_else(|| {
                        CoreError::new(
                            "resourceManifest",
                            UiMessage::error_core_operations_missing_resource_path(),
                        )
                    })?;
                    let path = resource_path(owner, name)?;
                    if is_private_tree(&root, &path) || is_trash(&root, &path)? {
                        return Err(CoreError::new(
                            "resourcePath",
                            UiMessage::error_core_exchange_resource_inside_omitted_internal_subtree(
                            ),
                        ));
                    }
                    let input = self.native_export_file(&witness, &path)?;
                    let file = input.binding.captured.as_ref().ok_or_else(|| {
                        CoreError::new(
                            "resourceMissing",
                            UiMessage::error_core_operations_registered_resource_file_absent(),
                        )
                    })?;
                    if !self.admitted_resource(
                        id,
                        &witness.workspace_id,
                        node_id,
                        &file.stamp.identity,
                    ) {
                        return Err(CoreError::new(
                            "resourceUnadmitted",
                            UiMessage::error_core_exchange_resource_lacks_exact_physical_admission(
                            ),
                        ));
                    }
                    let mut entry = NativeEntry::new(Role::Resource, &file.bytes);
                    entry.owner_node_id = Some(node_id.clone());
                    entry.resource_id = Some(id.into());
                    entry.revision = self.accepted_revision(file);
                    put(
                        &mut package,
                        relative(&root, &file.path)?,
                        file.bytes.clone(),
                        entry,
                    )?;
                    witness.resources.push(ResourceBinding {
                        path: display(&path),
                        node_id: node_id.clone(),
                        resource_id: id.into(),
                    });
                    witness.files.push(input);
                    native_input_budget(&witness)?;
                }
            }
            witness.files.push(manifest);
            let annotation =
                self.native_export_file(&witness, &owner.join("weftext.annotations.json"))?;
            if let Some(file) = &annotation.binding.captured {
                knowledge::decode_annotations(&file.bytes)
                    .map_err(|e| CoreError::from_knowledge(&e.code.clone(), e))?;
                let mut entry = NativeEntry::new(Role::Annotations, &file.bytes);
                entry.owner_node_id = Some(node_id.clone());
                put(
                    &mut package,
                    relative(&root, &file.path)?,
                    file.bytes.clone(),
                    entry,
                )?;
            }
            witness.files.push(annotation);
            native_input_budget(&witness)?;
        }
        let schema = captured_named(&witness, ".weftext-schema.json")
            .map(|f| f.bytes.clone())
            .unwrap_or_else(|| {
                include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/design/examples/schema.authoring.json"
                ))
                .to_vec()
            });
        weftext_data::Schema::from_author_json(&decode(&schema)?)
            .map_err(|e| CoreError::from_data("schemaUnavailable", e))?;
        let mut schema_entry = NativeEntry::new(Role::Schema, &schema);
        if let Some(file) = captured_named(&witness, ".weftext-schema.json") {
            schema_entry.revision = self.accepted_revision(file);
        }
        put(
            &mut package,
            ".weftext-schema.json".into(),
            schema,
            schema_entry,
        )?;
        if let Some(settings) = captured_named(&witness, ".weftext-settings.json") {
            let entry = NativeEntry::new(Role::Settings, &settings.bytes);
            put(
                &mut package,
                ".weftext-settings.json".into(),
                settings.bytes.clone(),
                entry,
            )?;
        }
        if let Some(tags) = captured_named(&witness, ".weftext-tags.json") {
            self.qualified_tag_document_from_captured(&root, tags)?;
            // Current portable packages carry v2. Historical v1 bytes stay in
            // the original library; upgrade them explicitly before exporting.
            crate::directory_identity::validate_sync_tags(&tags.bytes)?;
            put(
                &mut package,
                ".weftext-tags.json".into(),
                tags.bytes.clone(),
                NativeEntry::new(Role::Tags, &tags.bytes),
            )?;
        } else {
            self.validate_tag_registry_absence(&root)?;
        }
        native_style(&mut package)?;
        let empty_rules = captured_named(&witness, ".weftext-rules").is_none_or(|file| {
            serde_json::from_slice::<serde_json::Value>(&file.bytes).is_ok_and(|value| {
                value == serde_json::json!({"version":1,"ignored":[],"unmanaged":[]})
            })
        });
        for path in witness.observations.iter().map(|f| f.path.as_str()).chain(
            witness
                .omitted_directories
                .iter()
                .map(|(path, _)| path.as_str()),
        ) {
            let path = relative(&root, path)?;
            if package.files.contains_key(&path)
                || path.starts_with(".weftext-trash/")
                || matches!(
                    path.as_str(),
                    ".weftext-workspace.json" | ".weftext-format" | ".weftext-mvp-commit.json"
                )
                || (path == ".weftext-rules" && empty_rules)
            {
                continue;
            }
            package.manifest.losses.push(NativeLoss {
                path,
                code: "unregisteredFileOmitted".into(),
                description: UiMessage::loss_core_native_export_unregistered_file_omitted().into(),
                product_diagnostics: Vec::new(),
            });
        }
        native_input_budget(&witness)?;
        // Acquire all exact selected inputs together, checking the original
        // metadata cut. Build only while their read protection coexists.
        let _held = self.native_export_guards(&witness)?;
        let bytes = write_native_package(&package).map_err(native_error)?;
        self.native_export_membership(&witness)?;
        Ok(NativeExport {
            bytes,
            manifest: package.manifest,
            witness,
        })
    }

    /// Revalidate original input bytes, physical identities, parent/root
    /// objects, admission facts and observed raw/ignored/Trash membership.
    /// The host still performs current ACL/config validation at delivery.
    pub fn validate_native_export_witness(
        &self,
        root: impl AsRef<Path>,
        witness: &NativeExportWitness,
    ) -> Result<()> {
        if !same_path(&absolute_path(root.as_ref())?, &witness.root) {
            return Err(CoreError::new(
                "outsideRoot",
                UiMessage::error_core_exchange_native_export_witness_belongs_to_another_root(),
            ));
        }
        let _held = self.native_export_guards(witness)?;
        self.native_export_membership(witness)
    }

    fn native_export_admissions(&self, workspace: &str) -> Result<Vec<(String, FileIdentity)>> {
        let mut births = BTreeMap::new();
        for plan in self
            .operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
        {
            for birth in self.read_births(&plan.operation_id)? {
                if birth.workspace_id != workspace {
                    continue;
                }
                if births
                    .get(&birth.node_id)
                    .is_some_and(|identity| identity != &birth.identity)
                {
                    return Err(CoreError::new(
                        "coverageIncomplete",
                        UiMessage::error_core_exchange_competing_admitted_node_identities(),
                    ));
                }
                births.insert(birth.node_id, birth.identity);
                if births.len() > MAX_MEMBERS {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_exchange_native_export_exceeds5000_admitted_nodes(),
                    ));
                }
            }
        }
        Ok(births.into_iter().collect())
    }

    fn native_export_namespace(&self, root: &Path) -> Result<NativeNamespace> {
        let mut queue = vec![root.to_path_buf()];
        let mut directories = vec![];
        let mut observations = vec![];
        let mut omitted = vec![];
        let mut metadata_bytes = root.as_os_str().len() * 2 + std::mem::size_of::<PathBuf>();
        while let Some(path) = queue.pop() {
            let guard = ParentGuard::acquire(&path.join(".native-export-membership"))?;
            let (entries, complete) = guard.directory_entries(MAX_MEMBERS)?;
            if !complete {
                return Err(CoreError::new(
                    "coverageIncomplete",
                    UiMessage::error_core_exchange_native_export_membership_enumeration_incomplete(
                    ),
                ));
            }
            let parent = guard.directory_identity()?;
            let directory = CapturedDirectory {
                path: display(&path),
                identity: parent.clone(),
                entries: sorted_entries(entries.clone(), &path),
            };
            native_metadata_charge(
                &mut metadata_bytes,
                std::mem::size_of::<CapturedDirectory>()
                    + directory.path.capacity()
                    + directory.entries.capacity() * std::mem::size_of::<String>()
                    + directory
                        .entries
                        .iter()
                        .map(String::capacity)
                        .sum::<usize>(),
            )?;
            directories.push(directory);
            for (name, attrs) in entries {
                let child = path.join(name);
                if attrs & (0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0 {
                    return Err(CoreError::new(
                        "unsupportedPath",
                        UiMessage::error_core_exchange_native_membership_includes_reparse_offline_or_placeholder_object(),
                    ));
                }
                if attrs & 0x10 != 0 {
                    if is_private_tree(root, &child) {
                        let omitted_guard =
                            ParentGuard::acquire(&child.join(".native-export-omitted"))?;
                        let omitted_path = display(&child);
                        native_metadata_charge(
                            &mut metadata_bytes,
                            omitted_path.capacity() + std::mem::size_of::<(String, FileIdentity)>(),
                        )?;
                        omitted.push((omitted_path, omitted_guard.directory_identity()?));
                    } else {
                        native_metadata_charge(
                            &mut metadata_bytes,
                            child.capacity() * 2 + std::mem::size_of::<PathBuf>(),
                        )?;
                        queue.push(child);
                    }
                } else {
                    self.check_pending(&child)?;
                    let leaf = ParentGuard::acquire(&child)?;
                    let handle = leaf.open_placement_file()?;
                    let actual = stamp(&handle)?;
                    self.check_pending_identity(&actual.identity)?;
                    let observed = ObservedFile {
                        path: display(&child),
                        parent: parent.clone(),
                        stamp: actual,
                    };
                    native_metadata_charge(
                        &mut metadata_bytes,
                        observed.path.capacity() + std::mem::size_of::<ObservedFile>(),
                    )?;
                    observations.push(observed);
                }
                if directories.len() + observations.len() + queue.len() + omitted.len()
                    > MAX_MEMBERS
                {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_exchange_native_export_namespace_exceeds5000_members(
                        ),
                    ));
                }
            }
        }
        Ok(NativeNamespace {
            directories,
            observations,
            omitted_directories: omitted,
        })
    }

    fn native_export_file(
        &self,
        witness: &NativeExportWitness,
        path: &Path,
    ) -> Result<SnapshotWitness> {
        self.check_pending(path)?;
        let guard = ParentGuard::acquire(path)?;
        let parent = guard.directory_identity()?;
        let observed = witness
            .observations
            .iter()
            .find(|f| same_path(Path::new(&f.path), path));
        if let Some(observed) = observed {
            native_incoming_budget(witness, observed.stamp.size)?;
        }
        let captured = match guard.open_file(false, false) {
            Ok(mut handle) => {
                let actual = stamp(&handle)?;
                if observed.is_none_or(|f| f.stamp != actual || f.parent != parent) {
                    return Err(CoreError::new(
                        "scopeChanged",
                        UiMessage::error_core_exchange_native_input_differs_from_original_namespace_observation(),
                    ));
                }
                self.check_pending_identity(&actual.identity)?;
                Some(CapturedFile {
                    path: display(path),
                    bytes: read_blob(&mut handle)?,
                    stamp: actual,
                })
            }
            Err(error) if error.code == "missing" && observed.is_none() => None,
            Err(error) => return Err(error),
        };
        Ok(SnapshotWitness {
            binding: SnapshotBinding {
                path: display(path),
                parent,
                captured,
            },
        })
    }

    fn native_export_trash_member(&self, witness: &NativeExportWitness, path: &Path) -> Result<()> {
        let relative = path.strip_prefix(&witness.root).map_err(|_| {
            CoreError::new(
                "outsideRoot",
                UiMessage::error_core_exchange_trash_member_outside_root(),
            )
        })?;
        let tree = witness
            .root
            .join(relative.components().take(2).collect::<PathBuf>());
        let identity = witness
            .directories
            .iter()
            .find(|d| same_path(Path::new(&d.path), &tree))
            .map(|d| &d.identity)
            .ok_or_else(|| {
                CoreError::new(
                    "coverageIncomplete",
                    UiMessage::error_core_exchange_trash_tree_not_in_captured_membership(),
                )
            })?;
        if !self
            .operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
            .any(|p| {
                p.transaction.as_ref().is_some_and(|t| {
                    t.is_trash()
                        && matches!(&t.mutation,
                Mutation::Relocate { target, source_identity, .. }
                if same_path(Path::new(target), &tree) && source_identity == identity)
                })
            })
        {
            return Err(CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_exchange_trash_member_has_no_original_physical_tree_placement(
                ),
            ));
        }
        Ok(())
    }

    fn native_export_guards(
        &self,
        witness: &NativeExportWitness,
    ) -> Result<(Vec<ParentGuard>, Vec<File>)> {
        let mut guards = vec![];
        let mut handles = vec![];
        for directory in &witness.directories {
            let guard = ParentGuard::acquire(
                &Path::new(&directory.path).join(".native-export-membership"),
            )?;
            if guard.directory_identity()? != directory.identity {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_native_export_parent_root_object_changed(),
                ));
            }
            guards.push(guard);
        }
        for (path, identity) in &witness.omitted_directories {
            let guard = ParentGuard::acquire(&Path::new(path).join(".native-export-omitted"))?;
            if guard.directory_identity()? != *identity {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_native_export_omitted_subtree_object_changed(),
                ));
            }
            guards.push(guard);
        }
        for input in &witness.files {
            let binding = &input.binding;
            let path = Path::new(&binding.path);
            self.check_pending(path)?;
            let guard = ParentGuard::acquire(path)?;
            if guard.directory_identity()? != binding.parent {
                return Err(CoreError::new(
                    "staleBase",
                    UiMessage::error_core_exchange_native_export_input_parent_changed(),
                ));
            }
            match (&binding.captured, guard.open_read_guard_file()) {
                (Some(original), Ok(mut handle)) => {
                    let actual = stamp(&handle)?;
                    self.check_pending_identity(&actual.identity)?;
                    if actual != original.stamp || read_blob(&mut handle)? != original.bytes {
                        return Err(CoreError::new(
                            "staleBase",
                            UiMessage::error_core_exchange_native_export_input_object_or_bytes_changed(),
                        ));
                    }
                    handles.push(handle);
                }
                (None, Err(error)) if error.code == "missing" => {}
                (_, Err(error)) => return Err(error),
                _ => {
                    return Err(CoreError::new(
                        "staleBase",
                        UiMessage::error_core_exchange_native_export_absent_input_became_present(),
                    ));
                }
            }
            guards.push(guard);
        }
        if self.require_workspace(&witness.root)?.workspace_id != witness.workspace_id
            || self.native_export_admissions(&witness.workspace_id)? != witness.admissions
        {
            return Err(CoreError::new(
                "scopeChanged",
                UiMessage::error_core_exchange_native_export_workspace_or_admitted_membership_changed(),
            ));
        }
        for resource in &witness.resources {
            let input = witness
                .files
                .iter()
                .find(|f| f.binding.path == resource.path)
                .unwrap();
            let file = input.binding.captured.as_ref().unwrap();
            if !self.admitted_resource(
                &resource.resource_id,
                &witness.workspace_id,
                &resource.node_id,
                &file.stamp.identity,
            ) {
                return Err(CoreError::new(
                    "resourceUnadmitted",
                    UiMessage::error_core_exchange_native_resource_admission_changed(),
                ));
            }
        }
        self.native_export_membership(witness)?;
        Ok((guards, handles))
    }

    fn native_export_membership(&self, witness: &NativeExportWitness) -> Result<()> {
        for (path, identity) in &witness.omitted_directories {
            let guard = ParentGuard::acquire(&Path::new(path).join(".native-export-omitted"))?;
            if guard.directory_identity()? != *identity {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_native_export_omitted_subtree_object_changed(),
                ));
            }
        }
        for directory in &witness.directories {
            let guard = ParentGuard::acquire(
                &Path::new(&directory.path).join(".native-export-membership"),
            )?;
            let (entries, complete) = guard.directory_entries(MAX_MEMBERS)?;
            if !complete
                || guard.directory_identity()? != directory.identity
                || sorted_entries(entries, Path::new(&directory.path)) != directory.entries
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_native_export_observed_membership_changed(),
                ));
            }
        }
        for observed in &witness.observations {
            let guard = ParentGuard::acquire(Path::new(&observed.path))?;
            let handle = guard.open_placement_file()?;
            if guard.directory_identity()? != observed.parent || stamp(&handle)? != observed.stamp {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_exchange_native_export_observed_file_identity_or_metadata_changed(),
                ));
            }
        }
        Ok(())
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|path| path.as_os_str().is_empty())
}
fn is_trash(root: &Path, path: &Path) -> Result<bool> {
    let relative = path.strip_prefix(root).map_err(|_| {
        CoreError::new(
            "outsideRoot",
            UiMessage::error_core_exchange_native_member_outside_root(),
        )
    })?;
    Ok(relative.components().next().is_some_and(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(".weftext-trash")
    }))
}
fn is_private_tree(root: &Path, path: &Path) -> bool {
    path.strip_prefix(root).is_ok_and(|relative| {
        relative.components().any(|part| {
            let name = part.as_os_str().to_string_lossy().to_ascii_lowercase();
            name.starts_with(".weftext-") && name != ".weftext-trash"
        })
    })
}
fn captured_named<'a>(witness: &'a NativeExportWitness, name: &str) -> Option<&'a CapturedFile> {
    let path = witness.root.join(name);
    witness
        .files
        .iter()
        .find(|file| same_path(Path::new(&file.binding.path), &path))
        .and_then(|file| file.binding.captured.as_ref())
}
fn native_input_budget(witness: &NativeExportWitness) -> Result<()> {
    let total = witness
        .files
        .iter()
        .filter_map(|file| file.binding.captured.as_ref())
        .map(|file| file.bytes.len())
        .try_fold(0usize, usize::checked_add);
    if witness.files.len() > MAX_MEMBERS
        || total.is_none_or(|total| total > MAX_BYTES)
        || witness.retained_bytes() > MAX_BYTES
    {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_exchange_native_export_selected_retained_inputs_exceed5000_files_or64_mi_b(),
        ));
    }
    Ok(())
}
fn native_metadata_charge(total: &mut usize, bytes: usize) -> Result<()> {
    *total = total
        .checked_add(bytes)
        .filter(|total| *total <= MAX_BYTES)
        .ok_or_else(|| {
            CoreError::new(
                "budget",
                UiMessage::error_core_exchange_native_export_namespace_metadata_exceeds64_mi_b(),
            )
        })?;
    Ok(())
}
fn native_incoming_budget(witness: &NativeExportWitness, bytes: u64) -> Result<()> {
    if usize::try_from(bytes)
        .ok()
        .and_then(|bytes| witness.retained_bytes().checked_add(bytes))
        .is_none_or(|total| total > MAX_BYTES)
    {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_exchange_native_export_selected_input_exceeds_retained_byte_budget(),
        ));
    }
    Ok(())
}
fn native_style(package: &mut NativePackage) -> Result<()> {
    let settings: serde_json::Value = package
        .files
        .get(".weftext-settings.json")
        .map(|bytes| serde_json::from_slice(bytes))
        .transpose()?
        .unwrap_or_else(|| serde_json::json!({"version":1}));
    if settings["defaultStyle"]["enabled"] != true {
        return Ok(());
    }
    let selected = settings["defaultStyle"]["resource"]
        .as_str()
        .and_then(|value| {
            value.strip_prefix(&format!(
                "weftext-resource://{}/",
                package.manifest.workspace_id
            ))
        })
        .ok_or_else(|| {
            CoreError::new(
                "styleSelection",
                UiMessage::error_core_exchange_enabled_style_has_no_exact_resource_in_workspace(),
            )
        })?;
    if settings["defaultStyle"]["entryFunction"]
        .as_str()
        .is_some_and(|name| name != "library-style")
    {
        return Err(CoreError::new(
            "styleSelection",
            UiMessage::error_core_exchange_unsupported_style_entry_function(),
        ));
    }
    let entry = package
        .manifest
        .entries
        .get_mut("Styles/library-style.typ")
        .filter(|entry| {
            entry.role == Role::Resource && entry.resource_id.as_deref() == Some(selected)
        })
        .ok_or_else(|| {
            CoreError::new(
                "styleSelection",
                UiMessage::error_core_exchange_selected_style_not_admitted_live_style_resource(),
            )
        })?;
    decode(&package.files["Styles/library-style.typ"])?;
    entry.role = Role::Style;
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    struct Fixture {
        _temp: tempfile::TempDir,
        root: PathBuf,
        source: PathBuf,
        core: Core,
    }
    impl Fixture {
        fn new(style: bool) -> Self {
            let temp = tempfile::tempdir().unwrap();
            let root = temp.path().join("Library");
            let mut core = Core::new(temp.path().join("private")).unwrap();
            core.create_library(&root).unwrap();
            let note = core.create_note_fixture(&root, &root, "Note").unwrap();
            let source = PathBuf::from(&note.path);
            let plan = core
                .prepare_import_resource(
                    &root,
                    &source,
                    "owned.bin",
                    b"exact resource\r\n".to_vec(),
                    &fresh(),
                )
                .unwrap();
            core.execute_operation(&plan.operation_id, &plan.plan_id)
                .unwrap();
            fs::write(
                root.join("Note/weftext.annotations.json"),
                b"{\"version\":1,\"annotations\":[],\"unknown\":\"exact retained annotation\"}",
            )
            .unwrap();
            if style {
                let original = core.read_library_style(&root).unwrap();
                let plan = core
                    .prepare_library_style(
                        &root,
                        &original.base_id,
                        &original.source,
                        true,
                        &fresh(),
                    )
                    .unwrap();
                core.execute_operation(&plan.operation_id, &plan.plan_id)
                    .unwrap();
            }
            Self {
                _temp: temp,
                root,
                source,
                core,
            }
        }
        fn capture(&mut self) -> NativeExport {
            self.core.capture_native_export(&self.root).unwrap()
        }
        fn error(&self, witness: &NativeExportWitness) -> String {
            self.core
                .validate_native_export_witness(&self.root, witness)
                .unwrap_err()
                .code
        }
    }

    #[test]
    fn native_export_owned_zip_manifest_and_witness_share_one_complete_cut() {
        let mut f = Fixture::new(true);
        let before = (f.core.snapshots.len(), f.core.admitted_cuts.len());
        let output = f.capture();
        assert_eq!((f.core.snapshots.len(), f.core.admitted_cuts.len()), before);
        let package = read_native_package(&output.bytes).unwrap();
        assert_eq!(
            serde_json::to_value(&package.manifest).unwrap(),
            serde_json::to_value(&output.manifest).unwrap()
        );
        for path in [
            "Note/Note.typ",
            "Note/owned.bin",
            "Note/weftext.resources.json",
            "Note/weftext.annotations.json",
            ".weftext-settings.json",
            "Styles/library-style.typ",
        ] {
            assert_eq!(package.files[path], fs::read(f.root.join(path)).unwrap());
        }
        assert_eq!(
            package.manifest.entries["Styles/library-style.typ"].role,
            Role::Style
        );
        assert!(package.files.contains_key(".weftext-schema.json"));
        assert!(!package.files.contains_key(".weftext-mvp-commit.json"));
        let exact_inputs: usize = output
            .witness
            .files
            .iter()
            .filter_map(|file| file.binding.captured.as_ref())
            .map(|file| file.bytes.len())
            .sum();
        assert!(output.witness.retained_bytes() >= exact_inputs);
        f.core
            .validate_native_export_witness(&f.root, &output.witness)
            .unwrap();
        assert!(output.manifest.losses.is_empty());
    }

    #[test]
    fn native_export_witness_rejects_saved_source_and_each_selected_dependency_change() {
        let mut f = Fixture::new(false);
        let output = f.capture();
        let note = f.core.open(&f.source, Some(f.root.clone())).unwrap();
        f.core
            .save(SaveRequest {
                session_id: note.session_id,
                base_id: note.base_id,
                source: format!("{}\nNew saved source", note.source),
                operation_id: fresh(),
            })
            .unwrap();
        assert_eq!(f.error(&output.witness), "staleBase");
        for path in [
            "Note/owned.bin",
            "Note/weftext.resources.json",
            "Note/weftext.annotations.json",
            ".weftext-schema.json",
            ".weftext-settings.json",
            ".weftext-rules",
            "Styles/library-style.typ",
        ] {
            let mut f = Fixture::new(true);
            let output = f.capture();
            let target = f.root.join(path);
            let mut bytes = fs::read(&target).unwrap_or_default();
            bytes.push(b' ');
            fs::write(&target, bytes).unwrap();
            assert_eq!(f.error(&output.witness), "staleBase", "{path}");
        }
    }

    #[test]
    fn native_export_witness_rejects_same_bytes_file_parent_and_root_replacements() {
        for path in [
            "Note/Note.typ",
            "Note/owned.bin",
            "Note/weftext.annotations.json",
            ".weftext-settings.json",
        ] {
            let mut f = Fixture::new(true);
            let output = f.capture();
            let target = f.root.join(path);
            let bytes = fs::read(&target).unwrap();
            fs::rename(&target, target.with_extension("parked")).unwrap();
            fs::write(&target, bytes).unwrap();
            assert_eq!(f.error(&output.witness), "staleBase", "{path}");
        }
        let mut f = Fixture::new(false);
        let output = f.capture();
        let bytes = fs::read(&f.source).unwrap();
        fs::rename(f.root.join("Note"), f.root.join("Parked")).unwrap();
        fs::create_dir(f.root.join("Note")).unwrap();
        fs::write(&f.source, bytes).unwrap();
        assert_eq!(f.error(&output.witness), "scopeChanged");
        let mut f = Fixture::new(false);
        let output = f.capture();
        let old = f.root.with_extension("parked");
        fs::rename(&f.root, &old).unwrap();
        fs::create_dir(&f.root).unwrap();
        assert_eq!(f.error(&output.witness), "scopeChanged");
    }

    #[test]
    fn native_export_observes_raw_ignored_and_trash_membership_without_reading_omitted_bodies() {
        let mut f = Fixture::new(false);
        fs::create_dir(f.root.join("Ignored")).unwrap();
        fs::write(
            f.root.join(".weftext-rules"),
            b"{\"version\":1,\"ignored\":[\"Ignored\"],\"unmanaged\":[]}",
        )
        .unwrap();
        let raw = f.root.join("Ignored/unadmitted.typ");
        let blocked_read = OpenOptions::new()
            .write(true)
            .create_new(true)
            .share_mode(2)
            .open(&raw)
            .unwrap();
        blocked_read.set_len((MAX_BYTES + 1) as u64).unwrap();
        fs::create_dir(f.root.join(".weftext-history")).unwrap();
        fs::create_dir(f.root.join(".weftext-drafts")).unwrap();
        fs::create_dir(f.root.join(".weftext-credentials")).unwrap();
        for path in [
            ".weftext-history/private.bin",
            ".weftext-drafts/private.bin",
            ".weftext-credentials/private.bin",
        ] {
            fs::write(f.root.join(path), b"must never enter native ZIP").unwrap();
        }
        let output = f.capture();
        let package = read_native_package(&output.bytes).unwrap();
        assert!(!package.files.contains_key("Ignored/unadmitted.typ"));
        assert!(
            package
                .manifest
                .losses
                .iter()
                .any(|loss| loss.path == "Ignored/unadmitted.typ")
        );
        assert!(
            package
                .files
                .keys()
                .all(|path| !path.starts_with(".weftext-history/")
                    && !path.starts_with(".weftext-drafts/")
                    && !path.starts_with(".weftext-credentials/"))
        );
        assert!(
            output
                .witness
                .files
                .iter()
                .all(|file| !file.binding.path.ends_with("unadmitted.typ"))
        );
        f.core
            .validate_native_export_witness(&f.root, &output.witness)
            .unwrap();
        fs::write(
            f.root.join(".weftext-history/new-private.bin"),
            b"unselected history",
        )
        .unwrap();
        f.core
            .validate_native_export_witness(&f.root, &output.witness)
            .unwrap();
        fs::write(f.root.join("Ignored/new.txt"), b"new omitted member").unwrap();
        assert_eq!(f.error(&output.witness), "scopeChanged");
        drop(blocked_read);

        let mut f = Fixture::new(false);
        let raw = f.root.join("raw.txt");
        fs::write(&raw, b"same").unwrap();
        let output = f.capture();
        fs::rename(&raw, f.root.join("parked.txt")).unwrap();
        fs::write(&raw, b"same").unwrap();
        assert_eq!(f.error(&output.witness), "scopeChanged");

        let mut f = Fixture::new(false);
        let output = f.capture();
        let plan = f
            .core
            .prepare_operation(OperationRequest {
                operation_id: fresh(),
                root: display(&f.root),
                intent: OperationIntent::TrashNode {
                    source: display(&f.root.join("Note")),
                },
            })
            .unwrap();
        f.core
            .execute_operation(&plan.operation_id, &plan.plan_id)
            .unwrap();
        assert_eq!(f.error(&output.witness), "missing");
        let after_trash = f.capture();
        let package = read_native_package(&after_trash.bytes).unwrap();
        assert!(!package.files.contains_key("Note/Note.typ"));
        assert!(
            !package
                .files
                .keys()
                .any(|path| path.starts_with(".weftext-trash/"))
        );
        fs::write(f.root.join(".weftext-trash/new.txt"), b"new trash member").unwrap();
        assert_eq!(f.error(&after_trash.witness), "scopeChanged");
    }

    #[test]
    fn native_export_fails_explicitly_for_excluded_admission_and_namespace_budget() {
        let mut f = Fixture::new(false);
        fs::write(
            f.root.join(".weftext-rules"),
            b"{\"version\":1,\"ignored\":[\"Note\"],\"unmanaged\":[]}",
        )
        .unwrap();
        assert_eq!(
            f.core.capture_native_export(&f.root).err().unwrap().code,
            "excludedBoundary"
        );
        let mut f = Fixture::new(false);
        fs::create_dir(f.root.join("raw")).unwrap();
        for index in 0..MAX_MEMBERS {
            File::create(f.root.join(format!("raw/{index:04}.txt"))).unwrap();
        }
        assert_eq!(
            f.core.capture_native_export(&f.root).err().unwrap().code,
            "budget"
        );
        assert!(!f.root.join("manifest.json").exists());
    }
}
