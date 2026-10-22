//! Explicit adoption separates the installed causal predecessor from the exact
//! observed physical input. It does not weaken ordinary save/publication CAS.
use super::{
    apply::SyncApplication,
    replica::{LocalObject, SyncPublication},
    *,
};
use crate::{
    Core, OperationIntent, OperationPreview, OperationReceipt, OperationRequest, Path,
    absolute_path,
    backend::ParentGuard,
    display, fresh,
    journal::Plan,
    operations::{
        AbsentDependency, CapturedFile, FileChange, FrozenOperation, Mutation, capture_optional,
        change,
    },
};
use weftext_messages::UiMessage;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAdoptionCut {
    pub identity: SyncIdentity,
    pub expected_installed: Vec<Dependency>,
    pub changes: Vec<FileChange>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed_members: Vec<SyncRemovedMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncRemovedMember {
    pub path: String,
    pub base_id: String,
}

impl Core {
    /// Observe only explicitly selected installed objects at their original
    /// local locators. Reading allocates ordinary private snapshot bases only.
    pub fn read_sync_adoption_cut(
        &mut self,
        root: &Path,
        objects: &BTreeSet<ObjectKey>,
    ) -> Result<SyncAdoptionCut> {
        self.sync_local_owner()?;
        if objects.is_empty() || objects.len() > MAX_CHANGES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_select_finite_nonempty_installed_object_cut(),
            ));
        }
        let root = absolute_path(root)?;
        self.require_workspace(&root)?;
        let setup = self.sync_setup_for(&root)?;
        let identity = setup.identity.clone();
        let (local, known) = self.sync_local_state(setup)?;
        let mut graph = self.read_sync_history(&identity)?;
        for (digest, bytes) in known {
            graph.offer_blob(&digest, bytes.clone())?;
        }
        let mut expected_installed = Vec::new();
        let mut paths = Vec::new();
        let mut originals = Vec::new();
        for object in objects {
            let original = local.get(object).ok_or_else(|| {
                CoreError::new(
                    "syncBasisChanged",
                    UiMessage::error_core_sync_selected_object_not_installed_in_replica(),
                )
            })?;
            expected_installed.push(Dependency {
                object: object.clone(),
                revisions: BTreeSet::from([original.revision.clone()]),
            });
            for relative in original.state.files.keys() {
                if paths.len() == MAX_FILES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_adoption_cut_exceeds5000_files(),
                    ));
                }
                paths.push(display(&Path::new(&original.directory).join(relative)));
                originals.push((object.clone(), relative.clone()));
            }
        }
        let physical = self.sync_installed_stamps(&local, objects, None)?;
        let snapshot = self.read_bounded_snapshot(&root, &paths, MAX_FILES)?;
        let allocated: Vec<_> = snapshot.files.iter().map(|f| f.base_id.clone()).collect();
        let result = (|| {
            let mut changes = Vec::new();
            let mut removed_members = Vec::new();
            let mut total = 0usize;
            for (file, (object, relative)) in snapshot.files.into_iter().zip(originals) {
                let original = &local[&object];
                let Some(captured) = self.snapshots[&file.base_id].captured.as_ref() else {
                    self.validate_removed_resource_observation(
                        &root, &identity, &graph, original, &relative, &physical,
                    )?;
                    removed_members.push(SyncRemovedMember {
                        path: file.path,
                        base_id: file.base_id,
                    });
                    continue;
                };
                total += captured.bytes.len();
                if total > MAX_BLOB_BYTES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_adoption_cut_exceeds64_mi_b(),
                    ));
                }
                let stamp = &physical[&original.state.file_revisions[&relative]];
                if captured.stamp.identity != stamp.identity {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_sync_selected_installed_leaf_was_physically_replaced(
                        ),
                    ));
                }
                if captured.stamp != *stamp
                    || hash(&captured.bytes) != original.state.files[&relative]
                {
                    changes.push(FileChange {
                        path: file.path,
                        base_id: Some(file.base_id),
                        before: file.bytes.clone(),
                        after: file.bytes.unwrap(),
                    });
                }
            }
            if changes.is_empty() && removed_members.is_empty() {
                return Err(CoreError::new(
                    "noChange",
                    UiMessage::error_core_sync_selected_installed_objects_have_no_observed_external_change(),
                ));
            }
            Ok(SyncAdoptionCut {
                identity,
                expected_installed,
                changes,
                removed_members,
            })
        })();
        let retained: BTreeSet<_> = result
            .as_ref()
            .map(|cut| {
                cut.changes
                    .iter()
                    .filter_map(|c| c.base_id.as_deref())
                    .chain(
                        cut.removed_members
                            .iter()
                            .map(|member| member.base_id.as_str()),
                    )
                    .collect()
            })
            .unwrap_or_default();
        for base in allocated {
            if !retained.contains(base.as_str()) {
                self.snapshots.remove(&base);
            }
        }
        result
    }
    pub(crate) fn prepare_sync_adoption(
        &mut self,
        request: OperationRequest,
    ) -> Result<OperationPreview> {
        self.sync_local_owner()?;
        crate::valid_operation_id(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        if let Some(old) = self.operations.get(&request.operation_id) {
            if old.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_sync_original_operation_names_different_observed_content(
                    ),
                ));
            }
            return self.operation_details(&request.operation_id);
        }
        let (mut plan, semantic_required) = self.build_sync_adoption_plan(request)?;
        if semantic_required {
            let evidence = self.freeze_sync_adoption_plan(&mut plan)?;
            plan.sync_application.as_mut().unwrap().semantic_evidence = Some(evidence);
        }
        let preview = plan.transaction.as_ref().unwrap().preview.clone();
        self.stage(plan)?;
        Ok(preview)
    }

    pub(crate) fn build_sync_adoption_plan(
        &mut self,
        request: OperationRequest,
    ) -> Result<(Plan, bool)> {
        self.sync_local_owner()?;
        crate::valid_operation_id(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        let OperationIntent::AdoptSyncExternal {
            identity,
            expected_installed,
            changes,
            removed_members,
        } = &request.intent
        else {
            return Err(invalid(
                UiMessage::error_core_sync_explicit_external_adoption_intent_required(),
            ));
        };
        if changes.is_empty() && removed_members.is_empty()
            || changes.len().saturating_add(removed_members.len()) > MAX_FILES
            || expected_installed.is_empty()
            || expected_installed.len() > MAX_CHANGES
            || changes.iter().map(|c| c.after.len()).sum::<usize>() > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_adoption_requires_finite_nonempty_observed_cut(),
            ));
        }
        if changes
            .iter()
            .any(|c| c.before.as_ref() != Some(&c.after) || c.base_id.is_none())
        {
            return Err(CoreError::new(
                "basisRequired",
                UiMessage::error_core_sync_adopt_only_exact_current_bytes_with_their_original_observed_snapshot_basis(),
            ));
        }
        let root = absolute_path(Path::new(&request.root))?;
        let workspace = self.require_workspace(&root)?;
        let setup = self.sync_setup_for(&root)?;
        if &setup.identity != identity || workspace.workspace_id != identity.workspace_id {
            return Err(invalid(
                UiMessage::error_core_sync_selected_library_replica_family_mismatch(),
            ));
        }
        let replica_id = setup.replica_id.clone();
        let (local, known) = self.sync_local_state(setup)?;
        let mut before = BTreeMap::<ObjectKey, LocalObject>::new();
        for dependency in expected_installed {
            key(&dependency.object)?;
            let installed = local.get(&dependency.object).ok_or_else(|| {
                CoreError::new(
                    "syncBasisChanged",
                    UiMessage::error_core_sync_selected_installed_object_unavailable(),
                )
            })?;
            if dependency.revisions != BTreeSet::from([installed.revision.clone()])
                || before
                    .insert(dependency.object.clone(), installed.clone())
                    .is_some()
            {
                return Err(CoreError::new(
                    "syncBasisChanged",
                    UiMessage::error_core_sync_review_each_exact_installed_object_version_once_before_adopting(),
                ));
            }
        }
        let mut requested = BTreeMap::<String, &FileChange>::new();
        for input in changes {
            let path = absolute_path(Path::new(&input.path))?;
            let selected = before
                .values()
                .flat_map(|o| {
                    o.state
                        .files
                        .keys()
                        .map(move |p| Path::new(&o.directory).join(p))
                })
                .find(|p| same_path(p, &path));
            let selected = selected.ok_or_else(|| {
                CoreError::new(
                    "syncAdoptionScope",
                    UiMessage::error_core_sync_only_leaf_already_belonging_to_selected_installed_object_can_adopted(),
                )
            })?;
            if requested.insert(display(&selected), input).is_some() {
                return Err(invalid(
                    UiMessage::error_core_sync_observed_leaf_adoption_duplicate(),
                ));
            }
        }
        let mut requested_removed = BTreeMap::<String, &SyncRemovedMember>::new();
        for input in removed_members {
            let path = absolute_path(Path::new(&input.path))?;
            let selected = before.values().flat_map(|object| object.state.files.keys().map(move |relative| Path::new(&object.directory).join(relative)))
                .find(|selected| same_path(selected, &path))
                .ok_or_else(|| CoreError::new("syncAdoptionScope", UiMessage::error_core_sync_only_originally_installed_member_in_explicit_cut_can_observed_as_removed()))?;
            let selected = display(&selected);
            if requested.contains_key(&selected)
                || requested_removed.insert(selected, input).is_some()
            {
                return Err(invalid(
                    UiMessage::error_core_sync_competing_adoption_observations(),
                ));
            }
        }
        let mut files = Vec::<CapturedFile>::new();
        let selected = before.keys().cloned().collect();
        let physical = self.sync_installed_stamps(&before, &selected, None)?;
        let mut graph = self.read_sync_history(identity)?;
        for (digest, bytes) in known {
            graph.offer_blob(&digest, bytes.clone())?;
        }
        let mut output = Vec::new();
        let mut after = before.clone();
        let mut blobs = BTreeMap::new();
        let mut changed = BTreeSet::new();
        let mut total = 0usize;
        let mut member_count = 0usize;
        let mut removed_dependencies = Vec::new();
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: fresh(),
            kind: "adoptSyncExternal".into(),
            status: "prepared".into(),
            changes: vec![],
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: vec![],
        };
        for (object, original) in &before {
            if let Some(parent) = &original.state.parent
                && !before.contains_key(parent)
            {
                return Err(CoreError::new(
                    "syncDependencyRequired",
                    UiMessage::error_core_sync_include_installed_placement_parent_in_reviewed_adoption_cut(),
                ));
            }
            for (relative, digest) in &original.state.files {
                member_count += 1;
                if member_count > MAX_FILES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_complete_adoption_member_cut_exceeds5000_slots(),
                    ));
                }
                let path = Path::new(&original.directory).join(relative);
                self.check_pending(&path)?;
                let captured = capture_optional(&self.backend, &path)?;
                if let Some(input) = requested_removed.get(&display(&path)) {
                    self.verify_snapshot(&path, &input.base_id, captured.as_ref())?;
                    if captured.is_some() {
                        return Err(CoreError::new(
                            "staleBase",
                            UiMessage::error_core_sync_removed_member_observation_requires_original_negative_snapshot(),
                        ));
                    }
                    self.validate_removed_resource_observation(
                        &root, identity, &graph, original, relative, &physical,
                    )?;
                    let basis = &self.snapshots[&input.base_id];
                    removed_dependencies.push(AbsentDependency {
                        path: display(&path),
                        parent: basis.parent.clone(),
                    });
                    let state = &mut after.get_mut(object).unwrap().state;
                    state.files.remove(relative);
                    state.file_revisions.remove(relative);
                    changed.insert(object.clone());
                    preview
                        .changes
                        .push(change(&path, "adoptExternalRemoval", 0));
                    continue;
                }
                let captured = captured.ok_or_else(|| CoreError::new("syncExternalChange", UiMessage::error_core_sync_missing_installed_member_must_have_explicit_original_removed_members_observation()))?;
                total += captured.bytes.len();
                if total > MAX_BLOB_BYTES || files.len() == MAX_FILES {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_complete_adoption_input_cut_exceeds64_mi_b5000_files(),
                    ));
                }
                if let Some(input) = requested.get(&display(&path)) {
                    self.verify_snapshot(
                        &path,
                        input.base_id.as_deref().unwrap(),
                        Some(&captured),
                    )?;
                    if input.before.as_ref() != Some(&captured.bytes) {
                        return Err(CoreError::new(
                            "staleBase",
                            UiMessage::error_core_sync_observed_adoption_bytes_changed_after_review(
                            ),
                        ));
                    }
                    let revision = fresh();
                    let digest = hash(&captured.bytes);
                    let state = &mut after.get_mut(object).unwrap().state;
                    state.files.insert(relative.clone(), digest.clone());
                    state
                        .file_revisions
                        .insert(relative.clone(), revision.clone());
                    blobs.insert(digest, captured.bytes.clone());
                    changed.insert(object.clone());
                    preview.revisions.insert(display(&path), revision);
                    preview
                        .changes
                        .push(change(&path, "adoptExternal", captured.bytes.len()));
                    output.push(FileChange {
                        path: display(&path),
                        base_id: None,
                        before: Some(captured.bytes.clone()),
                        after: captured.bytes.clone(),
                    });
                } else if hash(&captured.bytes) != *digest
                    || captured.stamp != physical[&original.state.file_revisions[relative]]
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_unselected_object_member_differs_from_installed_version(),
                    ));
                }
                if physical[&original.state.file_revisions[relative]].identity
                    != captured.stamp.identity
                {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_sync_installed_leaf_no_longer_has_retained_original_physical_binding(),
                    ));
                }
                files.push(captured);
            }
        }
        for (id, bytes) in &blobs {
            graph.offer_blob(id, bytes.clone())?;
        }
        for (object, installed) in &after {
            let resources = super::apply::validate_state(&graph, object, &installed.state)?;
            if let ObjectKey::Node(id) = object {
                let source_path =
                    Path::new(&installed.directory).join(format!("{}.typ", installed.state.name));
                self.qualified_query_node_at(
                    &root,
                    &source_path,
                    &weftext_knowledge::NodeRef {
                        workspace_id: identity.workspace_id.clone(),
                        node_id: id.clone(),
                    },
                    if installed.state.retired {
                        "trash"
                    } else {
                        "live"
                    },
                )?;
                for (relative, resource_id) in resources {
                    let source = files
                        .iter()
                        .find(|f| {
                            same_path(
                                Path::new(&f.path),
                                &Path::new(&installed.directory).join(&relative),
                            )
                        })
                        .unwrap();
                    if !self.admitted_resource(
                        &resource_id,
                        &identity.workspace_id,
                        id,
                        &source.stamp.identity,
                    )? {
                        return Err(CoreError::new(
                            "identityConflict",
                            UiMessage::error_core_sync_adopted_resource_not_original_owner_bound_physical_object(),
                        ));
                    }
                }
                preview.node_map.insert(id.clone(), id.clone());
            }
            if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = object {
                let parent = ParentGuard::acquire(
                    &self.backend,
                    &Path::new(&installed.directory).join("weftext.directory.json"),
                )?
                .directory_identity()?;
                if !self.directory_admitted(&identity.workspace_id, id, &parent)? {
                    return Err(CoreError::new(
                        "identityConflict",
                        UiMessage::error_core_sync_adopted_directory_lacks_original_physical_admission(),
                    ));
                }
            }
        }
        let mut absent_dependencies = removed_dependencies;
        let semantic_required = self.validate_adoption_typed_cut(
            &root,
            &before,
            &after,
            &files,
            &changed,
            &mut absent_dependencies,
        )?;
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
        ] {
            let path = root.join(name);
            let captured = capture_optional(&self.backend, &path)?
                .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_adopt_original_workspace_control_is_unavailable()))?;
            total += captured.bytes.len();
            if total > MAX_BLOB_BYTES || files.len() == MAX_FILES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_sync_adoption_proof_exceeds_complete_input_budget(),
                ));
            }
            files.push(captured);
        }
        let parents = before.values().map(|o| o.commit.clone()).collect();
        let manifest = Manifest {
            version: 1,
            workspace_id: identity.workspace_id.clone(),
            replica_id,
            operation_id: request.operation_id.clone(),
            parents,
            changes: changed
                .iter()
                .map(|key| Change {
                    object: key.clone(),
                    before: BTreeSet::from([before[key].revision.clone()]),
                    revision: fresh(),
                    after: after[key].state.clone(),
                })
                .collect(),
            dependencies: before
                .keys()
                .filter(|k| !changed.contains(*k))
                .map(|k| Dependency {
                    object: k.clone(),
                    revisions: BTreeSet::from([before[k].revision.clone()]),
                })
                .collect(),
        };
        let commit = manifest.id()?;
        let after: Vec<_> = manifest
            .changes
            .iter()
            .map(|c| LocalObject {
                object: c.object.clone(),
                directory: before[&c.object].directory.clone(),
                commit: commit.clone(),
                revision: c.revision.clone(),
                state: c.after.clone(),
            })
            .collect();
        let mut write_parents = Vec::new();
        for file in &files {
            write_parents.push((
                file.path.clone(),
                ParentGuard::acquire(&self.backend, Path::new(&file.path))?.directory_identity()?,
            ));
        }
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
            "adoptSyncExternal",
        );
        plan.workspace_id = Some(identity.workspace_id.clone());
        plan.sync_publication = Some(SyncPublication {
            root: display(&root),
            identity: identity.clone(),
            manifest,
            blobs: blobs.clone(),
            locations: vec![],
        });
        plan.sync_application = Some(SyncApplication {
            root: display(&root),
            identity: identity.clone(),
            before: before.into_values().collect(),
            after,
            expected_versions: vec![],
            selection: BTreeSet::new(),
            blobs,
            semantic_evidence: None,
        });
        plan.transaction = Some(FrozenOperation {
            backup_private_digest: None,
            request,
            preview: preview.clone(),
            files,
            directories: vec![],
            mutation: Mutation::Update { files: output },
            receipt,
            hosted_destination: None,
            expected_membership: None,
            expected_trash_membership: None,
            absent_dependencies,
            write_parents,
        });
        Ok((plan, semantic_required))
    }

    /// An external absence is a sender observation, not a receiver delete
    /// permission. Both owner roles and the removed binding come from this
    /// replica's original installed cut, never the newer received heads.
    fn validate_removed_resource_observation(
        &self,
        root: &Path,
        identity: &SyncIdentity,
        graph: &ReplicaGraph,
        original: &LocalObject,
        relative: &str,
        physical: &BTreeMap<String, crate::FileStamp>,
    ) -> Result<()> {
        let failure = || {
            CoreError::new(
                "syncExternalChange",
                UiMessage::error_core_sync_only_original_registered_resource_removed_from_exact_observed_owner_manifest_can_adopted_as_absent(),
            )
        };
        let ObjectKey::Node(owner) = &original.object else {
            return Err(failure());
        };
        let resources = super::apply::validate_state(graph, &original.object, &original.state)?;
        let id = resources.get(relative).ok_or_else(failure)?;
        let stamp = &physical[&original.state.file_revisions[relative]];
        if !self.admitted_resource(id, &identity.workspace_id, owner, &stamp.identity)? {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_sync_removed_resource_lacks_original_owner_bound_physical_admission(),
            ));
        }
        let source_path =
            Path::new(&original.directory).join(format!("{}.typ", original.state.name));
        self.check_pending(&source_path)?;
        let source = capture_optional(&self.backend, &source_path)?.ok_or_else(failure)?;
        if source.stamp.identity
            != physical[&original.state.file_revisions[&format!("{}.typ", original.state.name)]]
                .identity
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_sync_removed_resource_s_canonical_owner_was_replaced(),
            ));
        }
        self.qualified_query_node_at(
            root,
            &source_path,
            &weftext_knowledge::NodeRef {
                workspace_id: identity.workspace_id.clone(),
                node_id: owner.clone(),
            },
            if original.state.retired {
                "trash"
            } else {
                "live"
            },
        )?;
        let manifest_path = Path::new(&original.directory).join("weftext.resources.json");
        self.check_pending(&manifest_path)?;
        let manifest = capture_optional(&self.backend, &manifest_path)?.ok_or_else(failure)?;
        if manifest.stamp.identity
            != physical[&original.state.file_revisions["weftext.resources.json"]].identity
        {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_core_sync_removed_resource_s_original_manifest_was_replaced(),
            ));
        }
        let document: serde_json::Value = serde_json::from_slice(&manifest.bytes)?;
        let entries = document["resources"]
            .as_array()
            .filter(|_| document["version"] == 1)
            .ok_or_else(failure)?;
        if entries.iter().any(|entry| {
            entry["resourceId"] == *id
                || entry["path"]
                    .as_str()
                    .is_some_and(|path| same_path(Path::new(path), Path::new(relative)))
        }) {
            return Err(failure());
        }
        Ok(())
    }

    pub(crate) fn sync_installed_stamps(
        &self,
        local: &BTreeMap<ObjectKey, LocalObject>,
        objects: &BTreeSet<ObjectKey>,
        leaves: Option<&BTreeSet<String>>,
    ) -> Result<BTreeMap<String, crate::FileStamp>> {
        let wanted: BTreeSet<_> = objects
            .iter()
            .flat_map(|o| local[o].state.file_revisions.values().cloned())
            .filter(|revision| leaves.is_none_or(|leaves| leaves.contains(revision)))
            .collect();
        let mut selected = BTreeMap::<String, (u64, crate::FileStamp)>::new();
        for plan in self.committed_plans()?.into_iter() {
            if matches!(plan.kind.as_str(), "save" | "saveAs")
                && self
                    .validate_file_save_plan(plan)
                    .inspect_err(|error| {
                        crate::resource_diagnostics::error(
                            "sync_installed.scope",
                            &plan.operation_id,
                            error,
                        )
                    })
                    .is_err()
            {
                continue;
            }
            let commit: serde_json::Value = serde_json::from_slice(
                &self
                    .backend
                    .read_bytes(self.journal_path(&plan.operation_id, "commit"))?,
            )?;
            let sequence = commit["commitSequence"].as_u64().unwrap_or(0);
            for fact in self
                .read_observation_facts(&plan.operation_id)?
                .into_iter()
                .filter(|f| wanted.contains(&f.revision))
            {
                if selected.get(&fact.revision).is_some_and(|(order, stamp)| {
                    stamp.identity != fact.stamp.identity
                        || *order == sequence && stamp != &fact.stamp
                }) {
                    return Err(CoreError::new(
                        "identityUnavailable",
                        UiMessage::error_core_sync_installed_revision_has_ambiguous_physical_evidence(),
                    ));
                }
                if selected
                    .get(&fact.revision)
                    .is_none_or(|(order, _)| *order < sequence)
                {
                    selected.insert(fact.revision, (sequence, fact.stamp));
                }
            }
        }
        for setup in self.sync_setups()? {
            for physical in &setup.physical_bindings {
                if !objects.contains(&physical.object) {
                    continue;
                }
                let Some(initial) = setup.initial.iter().find(|o| o.object == physical.object)
                else {
                    continue;
                };
                let Some(revision) = initial
                    .state
                    .file_revisions
                    .get(&physical.relative_path)
                    .filter(|r| wanted.contains(*r))
                else {
                    continue;
                };
                if let Some((_, stamp)) = selected.get(revision) {
                    if stamp.identity != physical.stamp.identity {
                        return Err(CoreError::new(
                            "identityUnavailable",
                            UiMessage::error_core_sync_initial_replica_physical_evidence_ambiguous(
                            ),
                        ));
                    }
                } else {
                    selected.insert(revision.clone(), (0, physical.stamp.clone()));
                }
            }
        }
        if !wanted.iter().all(|r| selected.contains_key(r)) {
            return Err(CoreError::new(
                "identityUnavailable",
                UiMessage::error_core_sync_legacy_installed_leaf_has_no_retained_original_physical_evidence_current_paths_cannot_reconstruct_it(),
            ));
        }
        Ok(selected
            .into_iter()
            .map(|(revision, (_, stamp))| (revision, stamp))
            .collect())
    }

    fn validate_adoption_typed_cut(
        &self,
        root: &Path,
        before: &BTreeMap<ObjectKey, LocalObject>,
        after: &BTreeMap<ObjectKey, LocalObject>,
        files: &[CapturedFile],
        changed: &BTreeSet<ObjectKey>,
        absent_dependencies: &mut Vec<AbsentDependency>,
    ) -> Result<bool> {
        let workspace = &self.sync_setup_for(root)?.identity.workspace_id;
        if !changed.iter().any(|k| matches!(k, ObjectKey::Node(_))) {
            return Ok(false);
        }
        let schema_path = root.join(".weftext-schema.json");
        let schema = if let Some(file) = files
            .iter()
            .find(|f| same_path(Path::new(&f.path), &schema_path))
        {
            weftext_data::Schema::from_author_json(&crate::decode(&file.bytes)?)
        } else {
            if self
                .sync_local_state(self.sync_setup_for(root)?)?
                .0
                .contains_key(&ObjectKey::Schema)
            {
                return Err(CoreError::new(
                    "syncDependencyRequired",
                    UiMessage::error_core_sync_include_installed_schema_in_reviewed_adoption_cut(),
                ));
            }
            self.adoption_absent_config(root, &schema_path, absent_dependencies)?;
            weftext_data::Schema::bundled()
        }
        .map_err(|e| CoreError::from_data("schemaUnavailable", e))?;
        if self
            .sync_local_state(self.sync_setup_for(root)?)?
            .0
            .contains_key(&ObjectKey::Rules)
            && !before.contains_key(&ObjectKey::Rules)
        {
            return Err(CoreError::new(
                "syncDependencyRequired",
                UiMessage::error_core_sync_include_installed_rules_in_reviewed_adoption_cut(),
            ));
        }
        if !before.contains_key(&ObjectKey::Rules) {
            let rules = root.join(".weftext-rules");
            if self.query_rules_ever_committed(root)? {
                return Err(CoreError::new(
                    "rulesUnavailable",
                    UiMessage::error_core_sync_previously_committed_rules_unavailable(),
                ));
            }
            self.adoption_absent_config(root, &rules, absent_dependencies)?;
        }
        let mut semantic_required = false;
        for object in changed {
            let ObjectKey::Node(_) = object else {
                continue;
            };
            let owner = &after[object];
            let path = Path::new(&owner.directory).join(format!("{}.typ", owner.state.name));
            let source = crate::decode(
                &files
                    .iter()
                    .find(|f| same_path(Path::new(&f.path), &path))
                    .unwrap()
                    .bytes,
            )?;
            let node_id = match object {
                ObjectKey::Node(id) => id.clone(),
                _ => unreachable!(),
            };
            let node = weftext_knowledge::NodeRef {
                workspace_id: workspace.clone(),
                node_id,
            };
            let citations = weftext_engine::citation_work_slots(&source)
                .map_err(|e| CoreError::from_engine("syncSemanticDecisionRequired", e))?;
            let saved = weftext_knowledge::saved_query_reference_slots(&source, &schema, &node)
                .map_err(|e| CoreError::from_knowledge("syncSemanticDecisionRequired", e))?;
            let definitions = weftext_knowledge::definition_slots(&source, &node)
                .map_err(|e| CoreError::from_knowledge("syncSemanticDecisionRequired", e))?;
            if !citations.is_empty()
                || !saved.typed_refs.is_empty()
                || !saved.definition_references.is_empty()
                || !definitions.is_empty()
                || !weftext_data::embedded_queries(&source).is_empty()
            {
                semantic_required = true;
            }
            let projection = weftext_data::project(&source, &schema);
            if projection.qualification != "qualified"
                || !projection.diagnostics.is_empty()
                || projection.fields.iter().any(|f| {
                    matches!(
                        f.cell,
                        weftext_data::Cell::Invalid { .. } | weftext_data::Cell::Unavailable { .. }
                    ) || f.required && matches!(f.cell, weftext_data::Cell::Absent)
                })
            {
                return Err(CoreError::new(
                    "invalidTypedSource",
                    UiMessage::error_core_sync_observed_author_declaration_not_valid_under_reviewed_schema(),
                ));
            }
            // Adoption preserves opaque author bytes. Only declared Shapes
            // assert references; unbound members are not copy/rewrite slots.
            let refs = weftext_data::declared_typed_ref_closure(&source, &schema)
                .map_err(|e| CoreError::from_data("invalidTypedSource", e))?;
            if !refs.complete {
                return Err(CoreError::new(
                    "invalidTypedSource",
                    UiMessage::error_core_sync_observed_typed_reference_closure_incomplete(),
                ));
            }
            for slot in refs.slots {
                let (kind, target_workspace, id) = if let Some(rest) =
                    slot.target.strip_prefix("weftext-resource://")
                {
                    let (workspace, id) = rest.split_once('/').ok_or_else(|| {
                        invalid(UiMessage::error_core_sync_resource_reference_identity_missing())
                    })?;
                    uuid(workspace)?;
                    uuid(id)?;
                    ("resource", workspace, id)
                } else {
                    crate::directory_identity::parse_ref(&slot.target)?
                };
                if target_workspace != workspace {
                    return Err(CoreError::new(
                        "syncDependencyRequired",
                        UiMessage::error_core_sync_typed_reference_requires_explicit_qualified_dependency_in_workspace(),
                    ));
                }
                let selected = match kind {
                    "node" => before.contains_key(&ObjectKey::Node(id.into())),
                    "tag" => before
                        .get(&ObjectKey::Tags)
                        .and_then(|o| o.state.files.get(".weftext-tags.json"))
                        .and_then(|_| {
                            files.iter().find(|f| {
                                same_path(Path::new(&f.path), &root.join(".weftext-tags.json"))
                            })
                        })
                        .and_then(|f| serde_json::from_slice::<serde_json::Value>(&f.bytes).ok())
                        .and_then(|v| v["tags"].as_array().cloned())
                        .is_some_and(|tags| tags.iter().any(|tag| tag["tagId"] == id)),
                    "resource" => before
                        .iter()
                        .filter(|(k, _)| matches!(k, ObjectKey::Node(_)))
                        .any(|(_, o)| {
                            files
                                .iter()
                                .find(|f| {
                                    same_path(
                                        Path::new(&f.path),
                                        &Path::new(&o.directory).join("weftext.resources.json"),
                                    )
                                })
                                .and_then(|f| {
                                    serde_json::from_slice::<serde_json::Value>(&f.bytes).ok()
                                })
                                .and_then(|v| v["resources"].as_array().cloned())
                                .is_some_and(|resources| {
                                    resources.iter().any(|r| r["resourceId"] == id)
                                })
                        }),
                    _ => false,
                };
                if !selected {
                    return Err(CoreError::new(
                        "syncDependencyRequired",
                        UiMessage::error_core_sync_include_every_actual_typed_target_in_exact_reviewed_adoption_cut(),
                    ));
                }
            }
        }
        Ok(semantic_required)
    }

    fn adoption_absent_config(
        &self,
        root: &Path,
        path: &Path,
        absent_dependencies: &mut Vec<AbsentDependency>,
    ) -> Result<()> {
        if capture_optional(&self.backend, path)?.is_some() {
            return Err(CoreError::new(
                "syncDependencyRequired",
                UiMessage::error_core_sync_actual_configuration_outside_installed_cut_must_explicitly_handled_before_adopting_source(),
            ));
        }
        self.require_workspace(root)?;
        absent_dependencies.push(AbsentDependency {
            path: display(path),
            parent: ParentGuard::acquire(&self.backend, path)?.directory_identity()?,
        });
        Ok(())
    }
}

/// Only this original no-write adoption can explain an installed member's
/// negative input. A prospective after state alone never grants this exception.
pub(crate) fn adopted_removed_member(plan: &Plan, path: &Path) -> bool {
    let (Some(application), Some(transaction), Some(publication)) = (
        &plan.sync_application,
        &plan.transaction,
        &plan.sync_publication,
    ) else {
        return false;
    };
    let OperationIntent::AdoptSyncExternal {
        identity,
        removed_members,
        ..
    } = &transaction.request.intent
    else {
        return false;
    };
    if identity != &application.identity
        || identity != &publication.identity
        || transaction.request.operation_id != plan.operation_id
        || publication.manifest.operation_id != plan.operation_id
        || transaction.request.root != application.root
        || !matches!(&transaction.mutation, Mutation::Update { files } if files.iter().all(|file| file.before.as_ref() == Some(&file.after)))
        || removed_members
            .iter()
            .filter(|member| same_path(Path::new(&member.path), path))
            .count()
            != 1
        || !transaction
            .absent_dependencies
            .iter()
            .any(|negative| same_path(Path::new(&negative.path), path))
    {
        return false;
    }
    application.before.iter().any(|before| {
        let ObjectKey::Node(_) = &before.object else {
            return false;
        };
        before.state.files.keys().any(|relative| {
            if relative == &format!("{}.typ", before.state.name)
                || matches!(
                    relative.as_str(),
                    "weftext.resources.json"
                        | "weftext.annotations.json"
                        | "weftext.directory.json"
                )
                || !same_path(&Path::new(&before.directory).join(relative), path)
            {
                return false;
            }
            application.after.iter().any(|after| {
                after.object == before.object
                    && after.directory == before.directory
                    && after.state.parent == before.state.parent
                    && after.state.name == before.state.name
                    && after.state.retired == before.state.retired
                    && !after.state.files.contains_key(relative)
                    && !after.state.file_revisions.contains_key(relative)
                    && publication.manifest.changes.iter().any(|change| {
                        change.object == before.object
                            && change.before == BTreeSet::from([before.revision.clone()])
                            && change.after == after.state
                    })
            })
        })
    })
}

fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}

#[cfg(all(test, windows))]
#[path = "adopt_tests.rs"]
mod tests;
