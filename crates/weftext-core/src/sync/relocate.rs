//! Pure incoming subtree relocation reuses the original namespace transaction.
//! Content/placement combinations stay pending until one complete plan exists.
use super::apply::SyncApplication;
use super::replica::{LocalObject, SyncPublication};
use super::*;
use crate::backend::ParentGuard;
use crate::journal::Plan;
use crate::operations::{FrozenOperation, capture_optional, snapshot_tree};
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
        if expected != *state {
            return Err(structure_required());
        }
        // Do not split an incoming compound transaction around an unrelated edit
        // or creation. The whole reviewed state must be representable here.
        for (other, version) in &states {
            if other != *key
                && local
                    .get(other)
                    .is_none_or(|old| old.state != version.state)
            {
                return Err(structure_required());
            }
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
        let mutation = self.prepare_relocation(
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
            if moved.is_some() {
                preview
                    .revisions
                    .extend(new.state.file_revisions.iter().map(|(name, revision)| {
                        (
                            display(&Path::new(&new.directory).join(name)),
                            revision.clone(),
                        )
                    }));
            }
            before.push(old.clone());
            after.push(new);
        }
        if files.len() > MAX_FILES
            || files.iter().map(|f| f.bytes.len()).sum::<usize>() > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Relocation including prerequisites exceeds the 64 MiB / 5000 file budget",
            ));
        }
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
            blobs: BTreeMap::new(),
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
