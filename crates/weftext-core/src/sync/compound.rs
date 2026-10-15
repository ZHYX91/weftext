//! Independent installed placements and content share one original sync decision.
use super::apply::SyncApplication;
use super::relocate::{RelocationInput, placement_changed};
use super::replica::LocalObject;
use super::*;
use crate::backend::{NamespaceGuard, ParentGuard};
use crate::journal::Plan;
use crate::operations::{
    FileChange, FrozenOperation, Mutation, NewBirth, RelocationParent, RelocationStep,
    capture_optional, change, snapshot_tree,
};
use crate::{Core, OperationPreview, OperationReceipt, Path, PathBuf, display, fresh};

fn structure_required() -> CoreError {
    CoreError::new(
        "syncStructureRequired",
        "Combined structure/content changes require one complete guarded plan; received versions are retained",
    )
}
struct CompoundBudget {
    files: usize,
    bytes: usize,
}
fn plan_parent_directories(
    root: &Path,
    original: &Path,
    final_path: &Path,
    placements: &[(PathBuf, PathBuf)],
    created: &mut BTreeSet<String>,
    write_parents: &mut Vec<(String, crate::FileIdentity)>,
) -> Result<()> {
    let mut original = original.parent().ok_or_else(structure_required)?;
    let mut final_path = final_path.parent().ok_or_else(structure_required)?;
    while !original.exists() {
        if original == root || !final_path.starts_with(root) {
            return Err(structure_required());
        }
        if created.insert(display(final_path)) && created.len() > MAX_FILES {
            return Err(CoreError::new(
                "budget",
                "Too many compound create-only directories",
            ));
        }
        if original.parent().unwrap().exists()
            && !placements.iter().any(|(_, target)| {
                crate::history::history_relative_path(final_path, target).is_some()
            })
        {
            write_parents.push((
                display(final_path),
                ParentGuard::acquire(original)?.directory_identity()?,
            ));
        }
        original = original.parent().ok_or_else(structure_required)?;
        final_path = final_path.parent().ok_or_else(structure_required)?;
    }
    Ok(())
}
impl CompoundBudget {
    fn charge(&mut self, bytes: usize, files: usize) -> Result<()> {
        let next_bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| CoreError::new("budget", "Compound byte budget overflow"))?;
        let next_files = self
            .files
            .checked_add(files)
            .ok_or_else(|| CoreError::new("budget", "Compound file budget overflow"))?;
        if next_bytes > MAX_BLOB_BYTES || next_files > MAX_FILES {
            return Err(CoreError::new(
                "budget",
                "Compound proof exceeds 64 MiB / 5000 files",
            ));
        }
        self.bytes = next_bytes;
        self.files = next_files;
        Ok(())
    }
}

impl Core {
    pub(crate) fn prepare_sync_compound(
        &mut self,
        input: RelocationInput,
    ) -> Result<OperationPreview> {
        let RelocationInput {
            request,
            identity,
            fingerprint,
            local,
            states,
            expected_versions,
            selection,
            publication,
            graph,
        } = input;
        let changed: Vec<_> = states
            .iter()
            .filter(|(key, version)| {
                local
                    .get(key)
                    .is_some_and(|old| placement_changed(&old.state, &version.state))
            })
            .collect();
        if changed.is_empty()
            || changed.len() > MAX_FILES
            || states.keys().any(|object| {
                !local.contains_key(object)
                    && !matches!(
                        object,
                        ObjectKey::Node(_)
                            | ObjectKey::Rules
                            | ObjectKey::Schema
                            | ObjectKey::Settings
                    )
            })
        {
            return Err(structure_required());
        }
        let root = crate::absolute_path(Path::new(&request.root))?;
        let mut locations: BTreeMap<ObjectKey, PathBuf> = BTreeMap::new();
        let mut remaining: BTreeSet<_> = states.keys().cloned().collect();
        while !remaining.is_empty() {
            let prior = remaining.len();
            for object in remaining.clone() {
                let state = &states[&object].state;
                if state.retired {
                    return Err(structure_required());
                }
                let directory = if matches!(object, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                    let parent = if let Some(parent) = &state.parent {
                        let Some(parent) = locations.get(parent) else {
                            continue;
                        };
                        parent
                    } else {
                        &root
                    };
                    parent.join(&state.name)
                } else {
                    root.clone()
                };
                locations.insert(object.clone(), directory);
                remaining.remove(&object);
            }
            if remaining.len() == prior {
                return Err(invalid(
                    "Compound placement lacks a reviewed parent or is cyclic",
                ));
            }
        }
        let mut placements = Vec::new();
        let mut changed_indices = BTreeMap::new();
        for (key, incoming) in &changed {
            let ObjectKey::Node(_) = key else {
                return Err(structure_required());
            };
            let original = &local[*key];
            if original.state.retired || incoming.state.retired {
                return Err(structure_required());
            }
            let source = PathBuf::from(&original.directory);
            let target = locations[*key].clone();
            changed_indices.insert((*key).clone(), placements.len());
            placements.push((source, target));
        }
        for (index, (source, target)) in placements.iter().enumerate() {
            for (other, (other_source, other_target)) in placements.iter().enumerate() {
                let within = |a: &Path, b: &Path| {
                    crate::backend::namespace_relative_path(a, b).map(|relative| relative.is_some())
                };
                if (index != other
                    && (within(source, other_source)? || within(target, other_target)?))
                    || within(target.parent().unwrap(), other_source)?
                {
                    return Err(structure_required());
                }
            }
        }
        let kind = if matches!(
            request.intent,
            crate::OperationIntent::RevalidateSync { .. }
        ) {
            "revalidateSync"
        } else if publication.is_some() {
            "mergeSync"
        } else {
            "applySync"
        };
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: fresh(),
            kind: kind.into(),
            status: "prepared".into(),
            changes: vec![],
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: vec![],
        };
        let mut relocations = Vec::new();
        let mut files = vec![];
        let mut directories = vec![];
        for (source, target) in &placements {
            if !target.parent().unwrap().exists() {
                // New parents have no physical identity yet. Qualify the same
                // original source through its existing ancestor, then bind its
                // destination to this Plan's create-only directory.
                let canonical = self.managed_node(&root, source, false)?;
                let namespace = NamespaceGuard::acquire(source, source.parent().unwrap())?;
                let old_leaf = Path::new(&canonical.path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                let new_leaf = format!("{}.typ", crate::operations::node_name(target)?);
                let mut held = namespace.open_source_file(&old_leaf, false)?;
                if crate::stamp(&held)? != canonical.stamp
                    || crate::operations::read_blob(&mut held)? != canonical.bytes
                {
                    return Err(CoreError::new(
                        "conflict",
                        "Original source changed while qualifying a planned parent",
                    ));
                }
                relocations.push(RelocationStep {
                    source: display(source),
                    target: display(target),
                    source_identity: namespace.source_identity()?,
                    source_parent_identity: ParentGuard::acquire(source)?.directory_identity()?,
                    target_parent_identity: RelocationParent::Created {
                        created_directory: display(target.parent().unwrap()),
                    },
                    old_leaf,
                    new_leaf,
                    leaf_identity: canonical.stamp.identity,
                    members: vec![],
                });
                preview
                    .changes
                    .push(change(source, "relocateFrom", canonical.bytes.len()));
                preview
                    .changes
                    .push(change(target, "relocateTo", canonical.bytes.len()));
                drop(held);
                drop(namespace);
                snapshot_tree(source, &mut files, &mut directories, false)?;
                continue;
            }
            let mutation =
                self.prepare_relocation(&root, source, target, false, &mut vec![], &mut preview)?;
            let Mutation::Relocate {
                source: original_source,
                target: original_target,
                source_identity,
                target_parent_identity,
                old_leaf,
                new_leaf,
                leaf_identity,
                members,
                create_trash_parent,
                ..
            } = mutation
            else {
                unreachable!()
            };
            if create_trash_parent {
                return Err(structure_required());
            }
            let source_parent_identity = ParentGuard::acquire(source)?.directory_identity()?;
            relocations.push(RelocationStep {
                source: original_source,
                target: original_target,
                source_identity,
                source_parent_identity,
                target_parent_identity: RelocationParent::Existing(target_parent_identity),
                old_leaf,
                new_leaf,
                leaf_identity,
                members,
            });
            snapshot_tree(source, &mut files, &mut directories, false)?;
        }
        // Preserve arbitrary local companion files, but every managed file must
        // still be the exact installed portable preimage. Raw edits need adoption.
        let mut budget = CompoundBudget {
            files: files.len(),
            bytes: files.iter().map(|file| file.bytes.len()).sum(),
        };
        let mut write_parents = Vec::new();
        let mut updates = Vec::new();
        let mut created_files = Vec::new();
        let mut created_directories = BTreeSet::new();
        let mut births = Vec::new();
        let mut blobs = BTreeMap::new();
        let mut file_paths = BTreeSet::new();
        let mut resource_ids = BTreeSet::new();
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (object, old) in &local {
            let moved = placements
                .iter()
                .enumerate()
                .find_map(|(index, (source, _))| {
                    crate::history::history_relative_path(Path::new(&old.directory), source)
                        .map(|relative| (index, relative))
                });
            if moved.is_none() && !states.contains_key(object) {
                continue;
            }
            let mut new = old.clone();
            if let Some((index, relative)) = &moved {
                let target = &placements[*index].1;
                new.directory = if relative.as_os_str().is_empty() {
                    display(target)
                } else {
                    display(&target.join(relative))
                };
                for (name, digest) in &old.state.files {
                    let path = Path::new(&old.directory).join(name);
                    let file = files
                        .iter()
                        .find(|f| Path::new(&f.path) == path)
                        .ok_or_else(|| {
                            CoreError::new(
                                "syncExternalChange",
                                "Installed subtree member is missing",
                            )
                        })?;
                    if hash(&file.bytes) != *digest {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "Subtree has an external edit; preserve and adopt it first",
                        ));
                    }
                    self.check_pending(&path)?;
                }
                if let ObjectKey::Node(id) = object {
                    let path = Path::new(&old.directory).join(format!("{}.typ", old.state.name));
                    let captured = files
                        .iter()
                        .find(|f| Path::new(&f.path) == path)
                        .ok_or_else(structure_required)?;
                    if self
                        .admitted_binding(&captured.stamp.identity)
                        .is_none_or(|b| b.node_id != *id || b.workspace_id != identity.workspace_id)
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "Moved Node identity differs from its original admission",
                        ));
                    }
                    if let Some(manifest) = files.iter().find(|f| {
                        Path::new(&f.path)
                            == Path::new(&old.directory).join("weftext.resources.json")
                    }) {
                        let manifest: serde_json::Value = serde_json::from_slice(&manifest.bytes)?;
                        for entry in manifest["resources"]
                            .as_array()
                            .ok_or_else(structure_required)?
                        {
                            let resource_id = entry["resourceId"]
                                .as_str()
                                .ok_or_else(structure_required)?;
                            let resource_path = Path::new(&old.directory)
                                .join(entry["path"].as_str().ok_or_else(structure_required)?);
                            let resource = files
                                .iter()
                                .find(|f| Path::new(&f.path) == resource_path)
                                .ok_or_else(structure_required)?;
                            if !self.admitted_resource(
                                resource_id,
                                &identity.workspace_id,
                                id,
                                &resource.stamp.identity,
                            ) {
                                return Err(CoreError::new(
                                    "syncExternalChange",
                                    "Moved resource identity differs from its original admission",
                                ));
                            }
                        }
                    }
                    preview.node_map.insert(id.clone(), id.clone());
                }
                if let ObjectKey::Directory(id) = object {
                    let directory = directories
                        .iter()
                        .find(|d| d.path == old.directory)
                        .ok_or_else(structure_required)?;
                    if !self.directory_admitted(&identity.workspace_id, id, &directory.identity)? {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "Moved directory identity differs from its original admission",
                        ));
                    }
                }
            } else {
                // A placement parent or semantic prerequisite participates as an
                // exact read, held through the same decision even though unmoved.
                for (name, digest) in &old.state.files {
                    let path = Path::new(&old.directory).join(name);
                    let captured = capture_optional(&path)?.ok_or_else(structure_required)?;
                    budget.charge(captured.bytes.len(), 1)?;
                    if hash(&captured.bytes) != *digest {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "Relocation prerequisite has an external change",
                        ));
                    }
                    write_parents.push((
                        display(&path),
                        ParentGuard::acquire(&path)?.directory_identity()?,
                    ));
                    files.push(captured);
                }
            }
            if let Some(version) = states.get(object) {
                new.commit = version.commit.clone();
                new.revision = version.revision.clone();
                new.state = version.state.clone();
            }
            if moved.is_none()
                && let ObjectKey::Node(id) = object
            {
                let path = Path::new(&old.directory).join(format!("{}.typ", old.state.name));
                let file = files
                    .iter()
                    .find(|f| Path::new(&f.path) == path)
                    .ok_or_else(structure_required)?;
                if self
                    .admitted_binding(&file.stamp.identity)
                    .is_none_or(|b| b.node_id != *id || b.workspace_id != identity.workspace_id)
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        "Unmoved content participant lost its original identity",
                    ));
                }
            }
            let resources = super::apply::validate_state(&graph, object, &new.state)?;
            let old_name = |name: &str| {
                if changed_indices.contains_key(object) && name == format!("{}.typ", new.state.name)
                {
                    format!("{}.typ", old.state.name)
                } else {
                    name.to_string()
                }
            };
            let retained: BTreeSet<_> = new.state.files.keys().map(|name| old_name(name)).collect();
            if old.state.files.keys().any(|name| !retained.contains(name)) {
                return Err(structure_required());
            }
            for (name, digest) in &new.state.files {
                let path = Path::new(&new.directory).join(name);
                if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                    return Err(invalid("Compound incoming content files overlap"));
                }
                let preimage = Path::new(&old.directory).join(old_name(name));
                let captured = files.iter().find(|f| Path::new(&f.path) == preimage);
                let expected = old.state.files.get(&old_name(name));
                if captured.map(|f| hash(&f.bytes)).as_ref() != expected {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        "A new incoming filename is occupied by local content",
                    ));
                }
                if captured.is_none() {
                    if !preimage.parent().unwrap().exists() {
                        plan_parent_directories(
                            &root,
                            &preimage,
                            &path,
                            &placements,
                            &mut created_directories,
                            &mut write_parents,
                        )?;
                    }
                    if preimage.parent().unwrap().exists() && capture_optional(&preimage)?.is_some()
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            "Incoming creation is occupied",
                        ));
                    }
                    if !resources.contains_key(name)
                        && !matches!(
                            name.as_str(),
                            "weftext.annotations.json" | "weftext.resources.json"
                        )
                    {
                        return Err(structure_required());
                    }
                    if moved.is_none() && preimage.parent().unwrap().exists() {
                        write_parents.push((
                            display(&path),
                            ParentGuard::acquire(&preimage)?.directory_identity()?,
                        ));
                    }
                }
                if let Some(resource_id) = resources.get(name) {
                    if !resource_ids.insert(resource_id.clone()) {
                        return Err(invalid(
                            "Compound incoming resource identity is duplicated across owners",
                        ));
                    }
                    let ObjectKey::Node(owner) = object else {
                        return Err(structure_required());
                    };
                    if let Some(file) = captured {
                        if !self.admitted_resource(
                            resource_id,
                            &identity.workspace_id,
                            owner,
                            &file.stamp.identity,
                        ) {
                            return Err(CoreError::new(
                                "syncExternalChange",
                                "Incoming resource changed its admitted identity",
                            ));
                        }
                    } else {
                        preview.resource_map.insert(
                            display(&path),
                            format!("weftext-resource://{}/{resource_id}", identity.workspace_id),
                        );
                    }
                }
                if matches!(object, ObjectKey::Tags)
                    && captured.is_some_and(|f| self.accepted_revision(f).is_none())
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        "Registry no longer has its original observed admission",
                    ));
                }
                let blob = graph.blob(digest).ok_or_else(structure_required)?;
                budget.charge(blob.len(), usize::from(captured.is_none()))?;
                let bytes = blob.to_vec();
                let prior = captured.map(|f| f.bytes.clone());
                if prior.as_ref() != Some(&bytes) {
                    preview.changes.push(change(&path, "receive", bytes.len()));
                }
                preview
                    .revisions
                    .insert(display(&path), new.state.file_revisions[name].clone());
                blobs.insert(digest.clone(), bytes.clone());
                updates.push(FileChange {
                    path: display(&path),
                    base_id: None,
                    before: prior,
                    after: bytes,
                });
            }
            before.push(old.clone());
            after.push(new);
        }
        // New Nodes and root configuration share the same create-only payload,
        // actual materialization and birth facts as the original CreateTree.
        for (object, version) in states
            .iter()
            .filter(|(object, _)| !local.contains_key(*object))
        {
            if let ObjectKey::Node(id) = object {
                let directory = &locations[object];
                if placements.iter().any(|(_, target)| {
                    crate::history::history_relative_path(directory, target).is_some()
                }) {
                    return Err(structure_required());
                }
                self.check_pending(directory)?;
                crate::operations::ensure_absent(directory)?;
                let resources = super::apply::validate_state(&graph, object, &version.state)?;
                for (name, digest) in &version.state.files {
                    let path = directory.join(name);
                    self.check_pending(&path)?;
                    plan_parent_directories(
                        &root,
                        &path,
                        &path,
                        &placements,
                        &mut created_directories,
                        &mut write_parents,
                    )?;
                    if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                        return Err(invalid("New Node overlaps another compound participant"));
                    }
                    let bytes = graph.blob(digest).ok_or_else(structure_required)?;
                    budget.charge(bytes.len(), 1)?;
                    if let Some(resource) = resources.get(name) {
                        if !resource_ids.insert(resource.clone()) {
                            return Err(invalid(
                                "New resource identity is duplicated across owners",
                            ));
                        }
                        preview.resource_map.insert(
                            display(&path),
                            format!("weftext-resource://{}/{resource}", identity.workspace_id),
                        );
                    }
                    preview
                        .revisions
                        .insert(display(&path), version.state.file_revisions[name].clone());
                    preview
                        .changes
                        .push(change(&path, "receiveCreate", bytes.len()));
                    created_files.push(FileChange {
                        path: display(&path),
                        base_id: None,
                        before: None,
                        after: bytes.to_vec(),
                    });
                    blobs.insert(digest.clone(), bytes.to_vec());
                }
                births.push(NewBirth {
                    path: display(&directory.join(format!("{}.typ", version.state.name))),
                    node_id: id.clone(),
                });
                preview.node_map.insert(id.clone(), id.clone());
                after.push(LocalObject {
                    object: object.clone(),
                    directory: display(directory),
                    commit: version.commit.clone(),
                    revision: version.revision.clone(),
                    state: version.state.clone(),
                });
                continue;
            }
            let name = match object {
                ObjectKey::Rules => ".weftext-rules",
                ObjectKey::Schema => ".weftext-schema.json",
                ObjectKey::Settings => ".weftext-settings.json",
                _ => return Err(structure_required()),
            };
            super::apply::validate_state(&graph, object, &version.state)?;
            if version.state.retired
                || version.state.parent.is_some()
                || version.state.files.len() != 1
                || !version.state.files.contains_key(name)
            {
                return Err(structure_required());
            }
            let path = root.join(name);
            if matches!(object, ObjectKey::Rules) && self.query_rules_ever_committed(&root)? {
                return Err(CoreError::new(
                    "rulesUnavailable",
                    "Previously committed Rules cannot be reconstructed as a new missing configuration",
                ));
            }
            self.check_pending(&path)?;
            crate::operations::ensure_absent(&path)?;
            let digest = &version.state.files[name];
            let bytes = graph.blob(digest).ok_or_else(structure_required)?;
            budget.charge(bytes.len(), 1)?;
            self.validate_update(&root, &path, bytes)?;
            if matches!(object, ObjectKey::Schema) {
                weftext_data::Schema::from_author_json(&crate::decode(bytes)?)
                    .map_err(|error| CoreError::new("invalidSchema", error.to_string()))?;
            }
            if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                return Err(invalid("New configuration overlaps a compound participant"));
            }
            write_parents.push((
                display(&path),
                ParentGuard::acquire(&path)?.directory_identity()?,
            ));
            preview
                .changes
                .push(change(&path, "receiveCreate", bytes.len()));
            preview
                .revisions
                .insert(display(&path), version.state.file_revisions[name].clone());
            created_files.push(FileChange {
                path: display(&path),
                base_id: None,
                before: None,
                after: bytes.to_vec(),
            });
            blobs.insert(digest.clone(), bytes.to_vec());
            after.push(LocalObject {
                object: object.clone(),
                directory: display(&root),
                commit: version.commit.clone(),
                revision: version.revision.clone(),
                state: version.state.clone(),
            });
        }
        for directory in &created_directories {
            self.check_pending(Path::new(directory))?;
        }
        for step in &relocations {
            if let RelocationParent::Created { created_directory } = &step.target_parent_identity
                && !created_directories.contains(created_directory)
            {
                return Err(structure_required());
            }
        }
        if directories.len() + created_directories.len() > MAX_FILES {
            return Err(CoreError::new(
                "budget",
                "Compound directory budget exceeded",
            ));
        }
        if files.len() + updates.iter().filter(|c| c.before.is_none()).count() + created_files.len()
            > MAX_FILES
            || files.iter().map(|f| f.bytes.len()).sum::<usize>()
                + updates.iter().map(|f| f.after.len()).sum::<usize>()
                + created_files
                    .iter()
                    .map(|file| file.after.len())
                    .sum::<usize>()
                > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Relocation including prerequisites exceeds the 64 MiB / 5000 file budget",
            ));
        }
        for file in &files {
            self.check_pending(Path::new(&file.path))?;
        }
        // Portable installed evidence was frozen when the corresponding real
        // Core decision committed. A fresh read cannot reconstruct continuity.
        let installed = before
            .iter()
            .map(|object| object.object.clone())
            .collect::<BTreeSet<_>>();
        let physical = self.sync_installed_stamps(&local, &installed, None)?;
        for object in &before {
            for (name, digest) in &object.state.files {
                let path = Path::new(&object.directory).join(name);
                let captured = files
                    .iter()
                    .find(|file| {
                        crate::history::history_relative_path(Path::new(&file.path), &path)
                            .is_some_and(|relative| relative.as_os_str().is_empty())
                    })
                    .ok_or_else(structure_required)?;
                let stamp = physical
                    .get(&object.state.file_revisions[name])
                    .ok_or_else(|| {
                        CoreError::new(
                            "identityUnavailable",
                            "Installed physical evidence is unavailable",
                        )
                    })?;
                if captured.stamp != *stamp || hash(&captured.bytes) != *digest {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        "Compound input differs from its exact installed physical version; adopt external content first",
                    ));
                }
            }
            if let ObjectKey::Node(id) = &object.object {
                preview.node_map.insert(id.clone(), id.clone());
            }
        }
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
        ] {
            let path = root.join(name);
            if !files.iter().any(|file| Path::new(&file.path) == path) {
                let captured = capture_optional(&path)?.ok_or_else(structure_required)?;
                budget.charge(captured.bytes.len(), 1)?;
                files.push(captured);
                write_parents.push((
                    display(&path),
                    ParentGuard::acquire(&path)?.directory_identity()?,
                ));
            }
        }
        if files.len()
            + updates
                .iter()
                .filter(|change| change.before.is_none())
                .count()
            + created_files.len()
            > MAX_FILES
            || files.iter().map(|file| file.bytes.len()).sum::<usize>()
                + updates
                    .iter()
                    .map(|change| change.after.len())
                    .sum::<usize>()
                + created_files
                    .iter()
                    .map(|file| file.after.len())
                    .sum::<usize>()
                > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Compound proof exceeds 64 MiB / 5000 files",
            ));
        }
        let mutation = Mutation::Compound {
            relocations,
            updates,
            created_files,
            created_directories: created_directories.into_iter().collect(),
            births,
        };
        let receipt = OperationReceipt { operation_id: request.operation_id.clone(), plan_id: preview.plan_id.clone(), status: "committed".into(), path: display(&root), node_ids: preview.node_map.keys().cloned().collect(), message: "Independent received placements and content committed through one original namespace decision; drafts retained".into() };
        let mut plan = Plan::file(
            request.operation_id.clone(),
            fingerprint,
            &root,
            None,
            vec![],
            None,
            kind,
        );
        plan.workspace_id = Some(identity.workspace_id.clone());
        plan.sync_application = Some(SyncApplication {
            root: display(&root),
            identity,
            before,
            after,
            expected_versions,
            selection,
            blobs,
            semantic_evidence: None,
        });
        plan.sync_publication = publication;
        plan.transaction = Some(FrozenOperation {
            backup_private_digest: None,
            request,
            preview: preview.clone(),
            files,
            directories,
            mutation,
            receipt,
            hosted_destination: None,
            expected_membership: None,
            absent_dependencies: vec![],
            write_parents,
        });
        self.freeze_sync_revalidation_if_needed(&mut plan)?;
        let transaction = plan.transaction.as_ref().unwrap();
        let mut originals = BTreeSet::new();
        let mut finals = BTreeSet::new();
        for file in &transaction.files {
            let original = file.path.replace('/', "\\").to_ascii_lowercase();
            let final_path = transaction
                .relocated_path(&file.path)
                .replace('/', "\\")
                .to_ascii_lowercase();
            if !originals.insert(original) || !finals.insert(final_path) {
                return Err(invalid(
                    "Compound original/final participant addresses overlap",
                ));
            }
        }
        if let Mutation::Compound { created_files, .. } = &transaction.mutation {
            for file in created_files {
                if !finals.insert(file.path.replace('/', "\\").to_ascii_lowercase()) {
                    return Err(invalid(
                        "Compound create-only output overlaps an original participant",
                    ));
                }
            }
        }
        crate::operations::compound::validate_compound_namespace(transaction)?;
        self.validate_compound_pending(&plan)?;
        let preview = transaction.preview.clone();
        self.stage(plan)?;
        Ok(preview)
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[test]
    fn compound_budget_rejects_next_duplicate_blob_member_before_afterimage_allocation() {
        let mut budget = CompoundBudget {
            files: 2,
            bytes: MAX_BLOB_BYTES - 8,
        };
        budget.charge(8, 1).unwrap();
        assert_eq!(budget.charge(1, 1).unwrap_err().code, "budget");
        assert_eq!(budget.files, 3);
        assert_eq!(budget.bytes, MAX_BLOB_BYTES);
        let mut files = CompoundBudget {
            files: MAX_FILES,
            bytes: 0,
        };
        assert_eq!(files.charge(0, 1).unwrap_err().code, "budget");
        assert_eq!(files.files, MAX_FILES);
    }
}
