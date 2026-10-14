//! Incoming subtree placement and content share the original namespace decision.
use super::apply::SyncApplication;
use super::replica::{LocalObject, SyncPublication};
use super::*;
use crate::backend::ParentGuard;
use crate::journal::Plan;
use crate::operations::{
    FileChange, FrozenOperation, Mutation, capture_optional, change, snapshot_tree,
};
use crate::{Core, OperationPreview, OperationReceipt, OperationRequest, Path, display, fresh};

pub(crate) struct RelocationInput {
    pub request: OperationRequest,
    pub identity: SyncIdentity,
    pub fingerprint: String,
    pub local: BTreeMap<ObjectKey, LocalObject>,
    pub states: BTreeMap<ObjectKey, Version>,
    pub expected_versions: Vec<Dependency>,
    pub selection: BTreeSet<ObjectKey>,
    pub publication: Option<SyncPublication>,
    pub graph: ReplicaGraph,
}

pub(crate) fn placement_changed(old: &ObjectState, new: &ObjectState) -> bool {
    old.name != new.name || old.parent != new.parent || old.retired != new.retired
}

fn structure_required() -> CoreError {
    CoreError::new(
        "syncStructureRequired",
        "Combined structure/content changes require one complete guarded plan; received versions are retained",
    )
}

impl Core {
    pub(crate) fn prepare_sync_relocation(
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
        let [(key, incoming)] = changed.as_slice() else {
            return Err(structure_required());
        };
        let ObjectKey::Node(node_id) = key else {
            return Err(structure_required());
        };
        let original = &local[*key];
        let root = crate::absolute_path(Path::new(&request.root))?;
        let source = Path::new(&original.directory);
        let state = &incoming.state;
        if original.state.retired && state.retired {
            return Err(structure_required());
        }
        let mut expected = original.state.clone();
        let old_leaf = format!("{}.typ", expected.name);
        let new_leaf = format!("{}.typ", state.name);
        let blob = expected
            .files
            .remove(&old_leaf)
            .ok_or_else(structure_required)?;
        let revision = expected
            .file_revisions
            .remove(&old_leaf)
            .ok_or_else(structure_required)?;
        expected.files.insert(new_leaf.clone(), blob);
        expected.file_revisions.insert(new_leaf, revision);
        expected.name = state.name.clone();
        expected.parent = state.parent.clone();
        expected.retired = state.retired;
        if expected
            .files
            .keys()
            .any(|name| !state.files.contains_key(name))
        {
            return Err(structure_required());
        }
        // All changed content participates below; no independent creation or
        // additional placement can be silently split out of this transaction.
        if states.keys().any(|object| !local.contains_key(object)) {
            return Err(structure_required());
        }
        let parent = if let Some(parent) = &state.parent {
            let parent = local.get(parent).ok_or_else(structure_required)?;
            if parent.state.retired
                || Path::new(&parent.directory).starts_with(root.join(".weftext-trash"))
            {
                return Err(structure_required());
            }
            Path::new(&parent.directory)
        } else {
            &root
        };
        let target = if state.retired {
            root.join(".weftext-trash").join(&request.operation_id)
        } else {
            parent.join(&state.name)
        };
        let kind = if publication.is_some() {
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
        let mut mutation = self.prepare_relocation(
            &root,
            source,
            &target,
            state.retired || original.state.retired,
            &mut vec![],
            &mut preview,
        )?;
        let mut files = vec![];
        let mut directories = vec![];
        snapshot_tree(source, &mut files, &mut directories, false)?;
        // Preserve arbitrary local companion files, but every managed file must
        // still be the exact installed portable preimage. Raw edits need adoption.
        let mut write_parents = Vec::new();
        let mut updates = Vec::new();
        let mut blobs = BTreeMap::new();
        let mut file_paths = BTreeSet::new();
        let mut resource_ids = BTreeSet::new();
        let mut before = Vec::new();
        let mut after = Vec::new();
        for (object, old) in &local {
            let moved = Path::new(&old.directory).strip_prefix(source).ok();
            if moved.is_none() && !states.contains_key(object) {
                continue;
            }
            let mut new = old.clone();
            if let Some(relative) = moved {
                new.directory = if relative.as_os_str().is_empty() {
                    display(&target)
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
                if object == *key && name == format!("{}.typ", new.state.name) {
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
                if !file_paths.insert(display(&path).to_lowercase()) {
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
                        return Err(structure_required());
                    }
                    if capture_optional(&preimage)?.is_some() {
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
                    if moved.is_none() {
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
                let bytes = graph.blob(digest).ok_or_else(structure_required)?.to_vec();
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
        if files.len() + updates.iter().filter(|c| c.before.is_none()).count() > MAX_FILES
            || files.iter().map(|f| f.bytes.len()).sum::<usize>()
                + updates.iter().map(|f| f.after.len()).sum::<usize>()
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
        let Mutation::Relocate {
            updates: frozen, ..
        } = &mut mutation
        else {
            unreachable!()
        };
        *frozen = updates;
        if !preview.node_map.contains_key(node_id) {
            return Err(structure_required());
        }
        let receipt = OperationReceipt { operation_id: request.operation_id.clone(), plan_id: preview.plan_id.clone(), status: "committed".into(), path: display(&target), node_ids: preview.node_map.keys().cloned().collect(), message: "Received subtree placement committed through the original namespace decision; files and drafts retained".into() };
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
        });
        plan.sync_publication = publication;
        plan.transaction = Some(FrozenOperation {
            request,
            preview: preview.clone(),
            files,
            directories,
            mutation,
            receipt,
            expected_membership: None,
            absent_dependencies: vec![],
            write_parents,
        });
        self.stage(plan)?;
        Ok(preview)
    }
}
