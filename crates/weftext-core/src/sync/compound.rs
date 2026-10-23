//! Independent installed placements and content share one original sync decision.
use super::apply::SyncApplication;
use super::relocate::{RelocationInput, placement_changed};
use super::replica::LocalObject;
use super::*;
use crate::backend::{NamespaceGuard, ParentGuard};
use crate::journal::Plan;
use crate::operations::{
    FileChange, FrozenOperation, Mutation, NewBirth, RelocationParent, RelocationStep,
    RetiredMember, capture_optional, change, snapshot_tree,
};
use crate::{Core, OperationPreview, OperationReceipt, Path, PathBuf, display, fresh};
use weftext_messages::UiMessage;

fn structure_required() -> CoreError {
    CoreError::new(
        "syncStructureRequired",
        UiMessage::error_core_sync_combined_structure_content_changes_require_one_complete_guarded_plan_received_versions_retained(),
    )
}
struct CompoundBudget {
    files: usize,
    bytes: usize,
}
/// A read-only projection of this request's final locations. It allocates no
/// identity or reservation; the original writer still freezes every FI/slot.
pub(crate) fn compound_locations(
    root: &Path,
    operation_id: &str,
    states: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
) -> Result<BTreeMap<ObjectKey, PathBuf>> {
    let mut locations = BTreeMap::<ObjectKey, PathBuf>::new();
    let mut remaining = states.keys().cloned().collect::<BTreeSet<_>>();
    while !remaining.is_empty() {
        let prior = remaining.len();
        for object in remaining.clone() {
            let state = &states[&object].state;
            let directory = match &object {
                ObjectKey::Node(id) | ObjectKey::Directory(id) => {
                    if let Some(original) = local.get(&object).filter(|old| {
                        matches!(object, ObjectKey::Node(_))
                            && Path::new(&old.directory) == root
                            && !placement_changed(&old.state, state)
                    }) {
                        locations.insert(object.clone(), PathBuf::from(&original.directory));
                        remaining.remove(&object);
                        continue;
                    }
                    let parent = if let Some(parent) = &state.parent {
                        let Some(parent) = locations.get(parent) else {
                            continue;
                        };
                        parent.as_path()
                    } else {
                        root
                    };
                    if state.retired {
                        if let Some(original) = local.get(&object).filter(|old| {
                            old.state.retired && !placement_changed(&old.state, state)
                        }) {
                            PathBuf::from(&original.directory)
                        } else {
                            let role = if matches!(object, ObjectKey::Node(_)) {
                                "node"
                            } else {
                                "directory"
                            };
                            root.join(".weftext-trash")
                                .join(format!("{operation_id}-{role}-{id}"))
                        }
                    } else {
                        parent.join(&state.name)
                    }
                }
                _ if state.retired => {
                    return Err(invalid(
                        UiMessage::error_core_sync_retirement_requires_placed_content(),
                    ));
                }
                _ => root.to_path_buf(),
            };
            locations.insert(object.clone(), directory);
            remaining.remove(&object);
        }
        if remaining.len() == prior {
            return Err(invalid(
                UiMessage::error_core_sync_compound_placement_parent_or_cycle(),
            ));
        }
    }
    Ok(locations)
}
fn original_creation_path(path: &Path, placements: &[(PathBuf, PathBuf)]) -> Result<PathBuf> {
    let mut original = None;
    for (source, target) in placements {
        if let Some(relative) = crate::backend::namespace_relative_path(path, target)? {
            let depth = target.components().count();
            if original.as_ref().is_none_or(|(prior, _)| depth > *prior) {
                original = Some((depth, source.join(relative)));
            }
        }
    }
    Ok(original.map_or_else(|| path.into(), |(_, original)| original))
}
fn snapshot_moving_tree(
    backend: &crate::BackendContext,
    source: &Path,
    placements: &[(PathBuf, PathBuf)],
    files: &mut Vec<crate::operations::CapturedFile>,
    directories: &mut Vec<crate::operations::CapturedDirectory>,
) -> Result<()> {
    for (other, _) in placements {
        if crate::backend::namespace_relative_path(source, other)?
            .is_some_and(|relative| !relative.as_os_str().is_empty())
        {
            return Ok(());
        }
    }
    snapshot_tree(backend, source, files, directories, false)
}

fn plan_parent_directories(
    backend: &crate::BackendContext,
    root: &Path,
    original: &Path,
    final_path: &Path,
    placements: &[(PathBuf, PathBuf)],
    created: &mut BTreeSet<String>,
    write_parents: &mut Vec<(String, crate::FileIdentity)>,
) -> Result<()> {
    let mut original = original.parent().ok_or_else(structure_required)?;
    let mut final_path = final_path.parent().ok_or_else(structure_required)?;
    while !backend.entry_exists(original)? {
        if original == root || !final_path.starts_with(root) {
            return Err(structure_required());
        }
        if created.insert(display(final_path)) && created.len() > MAX_FILES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_too_many_compound_create_only_directories(),
            ));
        }
        if backend.entry_exists(original.parent().unwrap())?
            && !placements.iter().any(|(_, target)| {
                crate::history::history_relative_path(final_path, target).is_some()
            })
        {
            write_parents.push((
                display(final_path),
                ParentGuard::acquire(backend, original)?.directory_identity()?,
            ));
        }
        original = original.parent().ok_or_else(structure_required)?;
        final_path = final_path.parent().ok_or_else(structure_required)?;
    }
    Ok(())
}
impl CompoundBudget {
    fn charge(&mut self, bytes: usize, files: usize) -> Result<()> {
        let next_bytes = self.bytes.checked_add(bytes).ok_or_else(|| {
            CoreError::new(
                "budget",
                UiMessage::error_core_sync_compound_byte_budget_overflow(),
            )
        })?;
        let next_files = self.files.checked_add(files).ok_or_else(|| {
            CoreError::new(
                "budget",
                UiMessage::error_core_sync_compound_file_budget_overflow(),
            )
        })?;
        if next_bytes > MAX_BLOB_BYTES || next_files > MAX_FILES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_compound_proof_exceeds64_mi_b5000_files(),
            ));
        }
        self.bytes = next_bytes;
        self.files = next_files;
        Ok(())
    }
}

impl Core {
    pub(crate) fn build_sync_compound_plan(&mut self, input: RelocationInput) -> Result<Plan> {
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
        if (changed.is_empty()
            && !states.iter().any(|(object, version)| {
                local
                    .get(object)
                    .is_some_and(|old| super::relocate::members_removed(&old.state, &version.state))
            }))
            || changed.len() > MAX_FILES
            || states.keys().any(|object| {
                !local.contains_key(object)
                    && !matches!(
                        object,
                        ObjectKey::Node(_)
                            | ObjectKey::Directory(_)
                            | ObjectKey::Rules
                            | ObjectKey::Schema
                            | ObjectKey::Settings
                    )
            })
        {
            return Err(structure_required());
        }
        let root = crate::absolute_path(Path::new(&request.root))?;
        let locations = compound_locations(&root, &request.operation_id, &states, &local)?;
        let mut placements = Vec::new();
        let mut changed_indices = BTreeMap::new();
        let mut directory_sources = BTreeSet::new();
        for (key, _) in &changed {
            if !matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                return Err(structure_required());
            }
            let original = &local[*key];
            let source = PathBuf::from(&original.directory);
            if matches!(key, ObjectKey::Directory(_)) {
                directory_sources.insert(source.clone());
            }
            let target = locations[*key].clone();
            changed_indices.insert((*key).clone(), placements.len());
            placements.push((source, target));
        }
        for (index, (source, target)) in placements.iter().enumerate() {
            let original_parent = original_creation_path(target.parent().unwrap(), &placements)?;
            let moving_parent =
                crate::backend::namespace_path_compare(&original_parent, target.parent().unwrap())?
                    != std::cmp::Ordering::Equal;
            for (other, (other_source, other_target)) in placements.iter().enumerate() {
                let within = |a: &Path, b: &Path| {
                    crate::backend::namespace_relative_path(a, b).map(|relative| relative.is_some())
                };
                if (index != other
                    && (crate::backend::namespace_path_compare(source, other_source)?
                        == std::cmp::Ordering::Equal
                        || crate::backend::namespace_path_compare(target, other_target)?
                            == std::cmp::Ordering::Equal))
                    || (!moving_parent && within(target.parent().unwrap(), other_source)?)
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
        let mut created_directories = BTreeSet::new();
        let mut trash_parents = Vec::new();
        let trash = root.join(".weftext-trash");
        if placements
            .iter()
            .any(|(_, target)| target.starts_with(&trash))
            && !self.backend.entry_exists(&trash)?
        {
            created_directories.insert(display(&trash));
            trash_parents.push((
                display(&trash),
                ParentGuard::acquire(&self.backend, &trash)?.directory_identity()?,
            ));
        }
        let source_identities = placements
            .iter()
            .map(|(source, _)| {
                ParentGuard::acquire(&self.backend, &source.join(".original-moving-participant"))?
                    .directory_identity()
            })
            .collect::<Result<Vec<_>>>()?;
        let mut needs_parking = false;
        let mut source_forest = false;
        for (source, target) in &placements {
            for (other, other_target) in &placements {
                if source != other
                    && (crate::backend::namespace_relative_path(source, other)?
                        .is_some_and(|relative| !relative.as_os_str().is_empty())
                        || crate::backend::namespace_relative_path(target, other_target)?
                            .is_some_and(|relative| !relative.as_os_str().is_empty()))
                {
                    source_forest = true;
                    needs_parking = true;
                }
                if source != other
                    && crate::backend::namespace_path_compare(target, other)?
                        == std::cmp::Ordering::Equal
                {
                    needs_parking = true;
                }
            }
        }
        for (index, (source, target)) in placements.iter().enumerate() {
            let (object, incoming) = changed[index];
            let original = &local[object];
            let original_parent = original_creation_path(target.parent().unwrap(), &placements)?;
            let moving_parent =
                crate::backend::namespace_path_compare(&original_parent, target.parent().unwrap())?
                    != std::cmp::Ordering::Equal;
            let trash_source = source.starts_with(root.join(".weftext-trash"));
            let trash_target = target.starts_with(root.join(".weftext-trash"));
            if moving_parent
                || !self.backend.entry_exists(target.parent().unwrap())?
                || trash_source
                || trash_target
            {
                // Qualify the original source before its final parent exists
                // at that coordinate. A moving parent's existing directory FI
                // remains distinct from a genuinely create-only parent.
                let (canonical, expected_directory) = if directory_sources.contains(source) {
                    let (marker, identity) = if trash_source {
                        let ObjectKey::Directory(id) = object else {
                            unreachable!()
                        };
                        self.qualified_query_scope_at(
                            &root,
                            source,
                            &format!("weftext-directory://{}/{id}", identity.workspace_id),
                            "trash",
                        )?;
                        let marker = capture_optional(
                            &self.backend,
                            &source.join("weftext.directory.json"),
                        )?
                        .ok_or_else(structure_required)?;
                        let parent = ParentGuard::acquire(&self.backend, Path::new(&marker.path))?
                            .directory_identity()?;
                        (marker, parent)
                    } else {
                        self.qualified_directory_marker(&root, source)?
                    };
                    (marker, Some(identity))
                } else {
                    let canonical = if trash_source {
                        let ObjectKey::Node(id) = object else {
                            unreachable!()
                        };
                        let path = source.join(format!("{}.typ", original.state.name));
                        self.qualified_query_node_at(
                            &root,
                            &path,
                            &weftext_knowledge::NodeRef {
                                workspace_id: identity.workspace_id.clone(),
                                node_id: id.clone(),
                            },
                            "trash",
                        )?;
                        capture_optional(&self.backend, &path)?.ok_or_else(structure_required)?
                    } else {
                        self.managed_node(&root, source, false)?
                    };
                    (canonical, None)
                };
                let namespace =
                    NamespaceGuard::acquire(&self.backend, source, source.parent().unwrap())?;
                let source_identity = namespace.source_identity()?;
                if expected_directory.is_some_and(|identity| identity != source_identity) {
                    return Err(CoreError::new(
                        "conflict",
                        UiMessage::error_core_sync_original_directory_changed_while_qualifying_planned_parent(),
                    ));
                }
                let old_leaf = Path::new(&canonical.path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                let new_leaf = if directory_sources.contains(source) {
                    old_leaf.clone()
                } else {
                    format!("{}.typ", incoming.state.name)
                };
                let mut held = namespace.open_source_file(&old_leaf, false)?;
                if crate::stamp(&held)? != canonical.stamp
                    || crate::operations::read_blob(&mut held)? != canonical.bytes
                {
                    return Err(CoreError::new(
                        "conflict",
                        UiMessage::error_core_sync_original_source_changed_while_qualifying_planned_parent(),
                    ));
                }
                let target_parent_identity = if self.backend.entry_exists(&original_parent)? {
                    let identity = ParentGuard::acquire(
                        &self.backend,
                        &original_parent.join(".original-compound-parent"),
                    )?
                    .directory_identity()?;
                    let original_slot = original_parent.join(target.file_name().unwrap());
                    match ParentGuard::acquire(
                        &self.backend,
                        &original_slot.join(".original-compound-slot"),
                    ) {
                        Ok(guard) if source_identities.contains(&guard.directory_identity()?) => {}
                        Ok(_) => {
                            return Err(CoreError::new(
                                "exists",
                                UiMessage::error_core_sync_compound_final_child_slot_occupied_by_another_directory(),
                            ));
                        }
                        Err(error) if error.code == "missing" => {}
                        Err(error) => return Err(error),
                    }
                    RelocationParent::Existing(identity)
                } else {
                    RelocationParent::Created {
                        created_directory: display(target.parent().unwrap()),
                    }
                };
                relocations.push(RelocationStep {
                    source: display(source),
                    target: display(target),
                    parking: None,
                    source_identity,
                    source_parent_identity: ParentGuard::acquire(&self.backend, source)?
                        .directory_identity()?,
                    target_parent_identity,
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
                snapshot_moving_tree(
                    &self.backend,
                    source,
                    &placements,
                    &mut files,
                    &mut directories,
                )?;
                continue;
            }
            let mut occupant = None;
            for (index, (other, _)) in placements.iter().enumerate() {
                if source != other
                    && crate::backend::namespace_path_compare(target, other)?
                        == std::cmp::Ordering::Equal
                {
                    occupant = Some(&source_identities[index]);
                }
            }
            let mutation = if directory_sources.contains(source) {
                self.prepare_directory_relocation_with_target(
                    &root,
                    source,
                    target,
                    &mut vec![],
                    &mut preview,
                    occupant,
                )?
            } else {
                self.prepare_relocation_with_target(
                    &root,
                    source,
                    target,
                    false,
                    &mut vec![],
                    &mut preview,
                    occupant,
                )?
            };
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
            let source_parent_identity =
                ParentGuard::acquire(&self.backend, source)?.directory_identity()?;
            relocations.push(RelocationStep {
                source: original_source,
                target: original_target,
                parking: None,
                source_identity,
                source_parent_identity,
                target_parent_identity: RelocationParent::Existing(target_parent_identity),
                old_leaf,
                new_leaf,
                leaf_identity,
                members,
            });
            snapshot_moving_tree(
                &self.backend,
                source,
                &placements,
                &mut files,
                &mut directories,
            )?;
        }
        let mut parking_parents = Vec::new();
        if needs_parking {
            for (index, step) in relocations.iter_mut().enumerate() {
                let parking_parent = if source_forest {
                    root.as_path()
                } else {
                    Path::new(&step.source).parent().unwrap()
                };
                let parking =
                    parking_parent.join(format!("WeftextMove-{}-{index}", request.operation_id));
                if source_forest {
                    parking_parents.push((
                        display(&parking),
                        ParentGuard::acquire(&self.backend, &parking)?.directory_identity()?,
                    ));
                }
                crate::operations::ensure_absent(&self.backend, &parking)?;
                preview.changes.push(change(&parking, "relocateVia", 0));
                step.parking = Some(display(&parking));
            }
        }
        // Preserve arbitrary local companion files, but every managed file must
        // still be the exact installed portable preimage. Raw edits need adoption.
        let mut budget = CompoundBudget {
            files: files.len(),
            bytes: files.iter().map(|file| file.bytes.len()).sum(),
        };
        let mut write_parents = parking_parents;
        write_parents.extend(trash_parents);
        let mut updates = Vec::new();
        let mut retired_members = Vec::new();
        let mut created_files = Vec::new();
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
                .filter_map(|(index, (source, _))| {
                    crate::history::history_relative_path(Path::new(&old.directory), source)
                        .map(|relative| (index, relative))
                })
                .max_by_key(|(index, _)| placements[*index].0.components().count());
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
                                UiMessage::error_core_sync_installed_subtree_member_missing(),
                            )
                        })?;
                    if hash(&file.bytes) != *digest {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_subtree_has_external_edit_preserve_and_adopt_it_first(),
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
                        .admitted_binding(&captured.stamp.identity)?
                        .is_none_or(|b| b.node_id != *id || b.workspace_id != identity.workspace_id)
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_moved_node_identity_differs_from_original_admission(),
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
                            )? {
                                return Err(CoreError::new(
                                    "syncExternalChange",
                                    UiMessage::error_core_sync_moved_resource_identity_differs_from_original_admission(),
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
                            UiMessage::error_core_sync_moved_directory_identity_differs_from_original_admission(),
                        ));
                    }
                }
            } else {
                // A placement parent or semantic prerequisite participates as an
                // exact read, held through the same decision even though unmoved.
                for (name, digest) in &old.state.files {
                    let path = Path::new(&old.directory).join(name);
                    let captured =
                        capture_optional(&self.backend, &path)?.ok_or_else(structure_required)?;
                    budget.charge(captured.bytes.len(), 1)?;
                    if hash(&captured.bytes) != *digest {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_relocation_prerequisite_has_external_change(
                            ),
                        ));
                    }
                    write_parents.push((
                        display(&path),
                        ParentGuard::acquire(&self.backend, &path)?.directory_identity()?,
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
                    .admitted_binding(&file.stamp.identity)?
                    .is_none_or(|b| b.node_id != *id || b.workspace_id != identity.workspace_id)
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_unmoved_content_participant_lost_original_identity(),
                    ));
                }
            }
            let resources = super::apply::validate_state(&graph, object, &new.state)?;
            let old_name = |name: &str| {
                if matches!(object, ObjectKey::Node(_))
                    && changed_indices.contains_key(object)
                    && name == format!("{}.typ", new.state.name)
                {
                    format!("{}.typ", old.state.name)
                } else {
                    name.to_string()
                }
            };
            for removed in super::member_retirement::qualify_member_retirements(
                &graph, object, &old.state, &new.state,
            )? {
                let source = Path::new(&old.directory).join(&removed.relative_path);
                let captured = files
                    .iter()
                    .find(|file| Path::new(&file.path) == source)
                    .ok_or_else(structure_required)?;
                let ObjectKey::Node(owner) = object else {
                    return Err(structure_required());
                };
                if !self.admitted_resource(
                    &removed.resource_id,
                    &identity.workspace_id,
                    owner,
                    &captured.stamp.identity,
                )? {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_sync_retired_resource_lost_original_physical_admission(),
                    ));
                }
                let guard = ParentGuard::acquire(&self.backend, &source)?;
                let mut file = guard.open_file(false, false)?;
                if crate::stamp(&file)? != captured.stamp
                    || crate::operations::read_blob(&mut file)? != captured.bytes
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_retired_resource_changed_during_planning(),
                    ));
                }
                let retention = root
                    .join(".weftext-trash")
                    .join(format!("{}-members", request.operation_id));
                crate::operations::ensure_absent(&self.backend, &retention)?;
                created_directories.insert(display(&retention));
                if !self.backend.entry_exists(&trash)? {
                    created_directories.insert(display(&trash));
                    if !write_parents
                        .iter()
                        .any(|(path, _)| Path::new(path) == trash)
                    {
                        write_parents.push((
                            display(&trash),
                            ParentGuard::acquire(&self.backend, &trash)?.directory_identity()?,
                        ));
                    }
                } else if !write_parents
                    .iter()
                    .any(|(path, _)| Path::new(path) == retention)
                {
                    write_parents.push((
                        display(&retention),
                        ParentGuard::acquire(&self.backend, &retention)?.directory_identity()?,
                    ));
                }
                let target =
                    retention.join(format!("{}-{}", retired_members.len(), removed.resource_id));
                preview.changes.push(change(
                    &Path::new(&new.directory).join(&removed.relative_path),
                    "retireMember",
                    captured.bytes.len(),
                ));
                preview
                    .changes
                    .push(change(&target, "retainOriginal", captured.bytes.len()));
                retired_members.push(RetiredMember {
                    source: display(&source),
                    target: display(&target),
                    identity: captured.stamp.identity.clone(),
                    parent: guard.directory_identity()?,
                    attributes: crate::backend::attributes(&file)?,
                    object: object.clone(),
                    relative_path: removed.relative_path,
                    resource_id: removed.resource_id,
                    digest: removed.digest,
                    file_revision: removed.file_revision,
                });
            }
            for (name, digest) in &new.state.files {
                let path = Path::new(&new.directory).join(name);
                if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                    return Err(invalid(
                        UiMessage::error_core_sync_compound_incoming_files_overlap(),
                    ));
                }
                let preimage = Path::new(&old.directory).join(old_name(name));
                let captured = files.iter().find(|f| Path::new(&f.path) == preimage);
                let expected = old.state.files.get(&old_name(name));
                if captured.map(|f| hash(&f.bytes)).as_ref() != expected {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_new_incoming_filename_occupied_by_local_content(
                        ),
                    ));
                }
                if captured.is_none() {
                    if !self.backend.entry_exists(preimage.parent().unwrap())? {
                        plan_parent_directories(
                            &self.backend,
                            &root,
                            &preimage,
                            &path,
                            &placements,
                            &mut created_directories,
                            &mut write_parents,
                        )?;
                    }
                    if self.backend.entry_exists(preimage.parent().unwrap())?
                        && capture_optional(&self.backend, &preimage)?.is_some()
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_incoming_creation_occupied(),
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
                    if moved.is_none() && self.backend.entry_exists(preimage.parent().unwrap())? {
                        write_parents.push((
                            display(&path),
                            ParentGuard::acquire(&self.backend, &preimage)?.directory_identity()?,
                        ));
                    }
                }
                if let Some(resource_id) = resources.get(name) {
                    if !resource_ids.insert(resource_id.clone()) {
                        return Err(invalid(
                            UiMessage::error_core_sync_compound_resource_owner_identity_duplicate(),
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
                        )? {
                            return Err(CoreError::new(
                                "syncExternalChange",
                                UiMessage::error_core_sync_incoming_resource_changed_admitted_identity(),
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
                    && captured
                        .map(|f| self.accepted_revision(f).map(|revision| revision.is_none()))
                        .transpose()?
                        .unwrap_or(false)
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_registry_no_longer_has_original_observed_admission(),
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
        // New Nodes, Directory parents and root configuration share the same
        // create-only payload and actual materialization as CreateTree.
        for (object, version) in states
            .iter()
            .filter(|(object, _)| !local.contains_key(*object))
        {
            if matches!(object, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                let directory = &locations[object];
                // The final address may be inside an original moving tree.
                // Its before absence and existing parent identities belong to
                // that same tree at its original address, while all published
                // outputs and materialization facts keep their final address.
                let original_directory = original_creation_path(directory, &placements)?;
                self.check_pending(directory)?;
                self.check_pending(&original_directory)?;
                crate::operations::ensure_absent(&self.backend, &original_directory)?;
                let resources = super::apply::validate_state(&graph, object, &version.state)?;
                if matches!(object, ObjectKey::Directory(_))
                    && !version.state.files.contains_key("weftext.directory.json")
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_new_directory_marker_missing(),
                    ));
                }
                for (name, digest) in &version.state.files {
                    let path = directory.join(name);
                    let original = original_directory.join(name);
                    self.check_pending(&path)?;
                    self.check_pending(&original)?;
                    plan_parent_directories(
                        &self.backend,
                        &root,
                        &original,
                        &path,
                        &placements,
                        &mut created_directories,
                        &mut write_parents,
                    )?;
                    if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                        return Err(invalid(
                            UiMessage::error_core_sync_new_node_participant_overlap(),
                        ));
                    }
                    let bytes = graph.blob(digest).ok_or_else(structure_required)?;
                    budget.charge(bytes.len(), 1)?;
                    if let Some(resource) = resources.get(name) {
                        if !resource_ids.insert(resource.clone()) {
                            return Err(invalid(
                                UiMessage::error_core_sync_new_resource_owner_identity_duplicate(),
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
                if let ObjectKey::Node(id) = object {
                    births.push(NewBirth {
                        path: display(&directory.join(format!("{}.typ", version.state.name))),
                        node_id: id.clone(),
                    });
                    preview.node_map.insert(id.clone(), id.clone());
                }
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
                    UiMessage::error_core_sync_previously_committed_rules_cannot_reconstructed_as_new_missing_configuration(),
                ));
            }
            self.check_pending(&path)?;
            crate::operations::ensure_absent(&self.backend, &path)?;
            let digest = &version.state.files[name];
            let bytes = graph.blob(digest).ok_or_else(structure_required)?;
            budget.charge(bytes.len(), 1)?;
            self.validate_update(&root, &path, bytes)?;
            if matches!(object, ObjectKey::Schema) {
                weftext_data::Schema::from_author_json(&crate::decode(bytes)?)
                    .map_err(|error| CoreError::from_data("invalidSchema", error))?;
            }
            if !file_paths.insert(display(&path).replace('/', "\\").to_ascii_lowercase()) {
                return Err(invalid(
                    UiMessage::error_core_sync_new_configuration_participant_overlap(),
                ));
            }
            write_parents.push((
                display(&path),
                ParentGuard::acquire(&self.backend, &path)?.directory_identity()?,
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
                UiMessage::error_core_sync_compound_directory_budget_exceeded(),
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
                UiMessage::error_core_sync_relocation_including_prerequisites_exceeds64_mi_b5000_file_budget(),
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
                            UiMessage::error_core_sync_installed_physical_evidence_unavailable(),
                        )
                    })?;
                if captured.stamp != *stamp || hash(&captured.bytes) != *digest {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_compound_input_differs_from_exact_installed_physical_version_adopt_external_content_first(),
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
                let captured =
                    capture_optional(&self.backend, &path)?.ok_or_else(structure_required)?;
                budget.charge(captured.bytes.len(), 1)?;
                files.push(captured);
                write_parents.push((
                    display(&path),
                    ParentGuard::acquire(&self.backend, &path)?.directory_identity()?,
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
                UiMessage::error_core_sync_compound_proof_exceeds64_mi_b5000_files(),
            ));
        }
        let mutation = Mutation::Compound {
            relocations,
            updates,
            created_files,
            created_directories: created_directories.into_iter().collect(),
            births,
            retired_members,
        };
        let receipt = OperationReceipt {
            operation_id: request.operation_id.clone(),
            plan_id: preview.plan_id.clone(),
            status: "committed".into(),
            path: display(&root),
            node_ids: preview.node_map.keys().cloned().collect(),
        };
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
            expected_trash_membership: None,
            absent_dependencies: vec![],
            write_parents,
        });
        Ok(plan)
    }

    pub(crate) fn validate_sync_plan_participants(&self, plan: &Plan) -> Result<()> {
        let transaction = plan.transaction.as_ref().unwrap();
        if !matches!(transaction.mutation, Mutation::Compound { .. }) {
            return Ok(());
        }
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
                    UiMessage::error_core_sync_compound_participant_addresses_overlap(),
                ));
            }
        }
        if let Mutation::Compound { created_files, .. } = &transaction.mutation {
            for file in created_files {
                if !finals.insert(file.path.replace('/', "\\").to_ascii_lowercase()) {
                    return Err(invalid(
                        UiMessage::error_core_sync_compound_create_output_overlap(),
                    ));
                }
            }
        }
        crate::operations::compound::validate_compound_namespace(transaction)?;
        self.validate_compound_pending(plan)
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
