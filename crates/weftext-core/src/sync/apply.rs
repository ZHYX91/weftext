//! Incoming content uses existing guarded Update, CreateTree and Relocate transactions. The original
//! plan also freezes installed portable versions; receipt recovery cannot echo
//! received content as a new local edit or lose its causal predecessor.
use super::replica::{LocalObject, SyncPublication};
use super::*;
use crate::{
    Core, OperationIntent, OperationPreview, OperationReceipt, OperationRequest, Path, PathBuf,
    absolute_path,
    backend::ParentGuard,
    display, fresh,
    journal::Plan,
    operations::{
        FileChange, FrozenOperation, Mutation, NewBirth, capture_optional, change, ensure_absent,
    },
};
use weftext_messages::UiMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncResolution {
    pub object: ObjectKey,
    /// Exact complete displayed conflict set. A newly received side invalidates
    /// this proposal; the caller must review it before making another proposal.
    pub revisions: BTreeSet<String>,
    pub state: ObjectState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncApplication {
    pub root: String,
    pub identity: SyncIdentity,
    pub before: Vec<LocalObject>,
    pub after: Vec<LocalObject>,
    pub expected_versions: Vec<Dependency>,
    #[serde(default)]
    pub selection: BTreeSet<ObjectKey>,
    pub blobs: BTreeMap<String, Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_evidence: Option<super::revalidation_writer::FrozenSemanticEvidence>,
}

impl Core {
    /// Recover the exact retained proposal by its original ID, including after
    /// a browser origin/port change. Never creates or executes a proposal.
    pub fn sync_operation_request(&self, operation_id: &str) -> Result<OperationRequest> {
        self.backend.check_path(&self.data_dir)?;
        self.sync_local_owner()?;
        self.operation_details(operation_id)?;
        let transaction = self.operations[operation_id].transaction.as_ref().unwrap();
        if !matches!(
            transaction.request.intent,
            OperationIntent::ApplySync { .. }
                | OperationIntent::MergeSync { .. }
                | OperationIntent::RevalidateSync { .. }
                | OperationIntent::CloneSync { .. }
                | OperationIntent::AdoptSyncExternal { .. }
        ) {
            return Err(invalid(
                UiMessage::error_core_sync_retained_sync_proposal_required(),
            ));
        }
        Ok(transaction.request.clone())
    }

    /// Abandon a frozen sync proposal only before its original install marker.
    /// Unlike recovery's keepCurrent, no assertion about current file bytes is
    /// needed: this operation has never entered the filesystem effect phase.
    pub fn cancel_prepared_sync(
        &mut self,
        operation_id: &str,
        plan_id: &str,
    ) -> Result<OperationPreview> {
        self.sync_local_owner()?;
        let preview = self.operation_details(operation_id)?;
        let plan = self.operations.get(operation_id).unwrap();
        let transaction = plan.transaction.as_ref().unwrap();
        if !matches!(
            transaction.request.intent,
            OperationIntent::ApplySync { .. }
                | OperationIntent::MergeSync { .. }
                | OperationIntent::RevalidateSync { .. }
                | OperationIntent::CloneSync { .. }
                | OperationIntent::AdoptSyncExternal { .. }
        ) {
            return Err(invalid(
                UiMessage::error_core_sync_frozen_proposal_cancellation_required(),
            ));
        }
        if preview.plan_id != plan_id {
            return Err(CoreError::new(
                "planConflict",
                UiMessage::error_core_sync_cancel_exact_original_sync_proposal(),
            ));
        }
        if matches!(preview.status.as_str(), "committed" | "resolved") {
            return Ok(preview);
        }
        if self
            .backend
            .entry_exists(self.journal_path(operation_id, "install"))?
        {
            return Err(CoreError::new(
                "recoveryPending",
                UiMessage::error_core_author_order_original_installation_may_have_effects_inspect_recovery_instead(),
            ));
        }
        self.resolve_record(operation_id, "cancelPrepared")?;
        self.operation_details(operation_id)
    }

    pub(crate) fn prepare_sync_application(
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
                    UiMessage::error_core_sync_replica_operation_already_names_different_input(),
                ));
            }
            return self.operation_details(&request.operation_id);
        }
        if let OperationIntent::RevalidateSync {
            identity,
            expected_installed,
            reviewed_versions,
            resolutions,
            blobs,
        } = &request.intent
        {
            let input = self.prepare_sync_revalidation_input(
                &request,
                identity,
                expected_installed,
                reviewed_versions,
                resolutions,
                blobs,
            )?;
            return self.prepare_sync_application_input(input);
        }
        let root = absolute_path(Path::new(&request.root))?;
        let setup = self.sync_setup_for(&root)?;
        let (identity, requested, resolutions, supplied) = match &request.intent {
            OperationIntent::ApplySync { identity, objects } => {
                (identity, objects.clone(), vec![], BTreeMap::new())
            }
            OperationIntent::MergeSync {
                identity,
                resolutions,
                blobs,
            } => (
                identity,
                resolutions.iter().map(|r| r.object.clone()).collect(),
                resolutions.clone(),
                blobs.clone(),
            ),
            _ => {
                return Err(invalid(
                    UiMessage::error_core_sync_replica_application_intent_required(),
                ));
            }
        };
        if identity != &setup.identity
            || self.require_workspace(&root)?.workspace_id != identity.workspace_id
        {
            return Err(invalid(
                UiMessage::error_core_sync_replica_binding_identity_mismatch(),
            ));
        }
        if requested.is_empty()
            || requested.len() > MAX_CHANGES
            || (!resolutions.is_empty() && requested.len() != resolutions.len())
        {
            return Err(invalid(
                UiMessage::error_core_sync_resolution_selection_bounded_nonempty(),
            ));
        }
        // Make every original committed local branch visible before inspecting
        // conflicts. Publication uses immutable outbox bytes, never current files.
        self.publish_sync(&root)?;
        let (local, _) = self.sync_local_state(setup)?;
        let mut graph = self.read_sync_history(identity)?;
        let current = graph.objects()?;
        let installed = graph.closure(&local.values().map(|v| v.commit.clone()).collect());
        let mut needed = requested.clone();
        loop {
            let previous = needed.len();
            let cut = needed
                .iter()
                .map(|key| {
                    current.get(key).ok_or_else(|| {
                        invalid(UiMessage::error_core_sync_selected_object_unavailable())
                    })
                })
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .flat_map(|v| v.versions.iter().map(|v| v.commit.clone()))
                .collect();
            // Any not-yet-installed compound transaction remains indivisible.
            for id in graph.closure(&cut).difference(&installed) {
                let manifest = graph.manifest(id).unwrap();
                needed.extend(manifest.changes.iter().map(|c| c.object.clone()));
                needed.extend(manifest.dependencies.iter().map(|d| d.object.clone()));
            }
            for key in needed.clone() {
                for v in &current[&key].versions {
                    if let Some(parent) = &v.state.parent {
                        needed.insert(parent.clone());
                    }
                    needed.extend(
                        graph
                            .manifest(&v.commit)
                            .unwrap()
                            .dependencies
                            .iter()
                            .map(|d| d.object.clone()),
                    );
                }
            }
            if needed.len() == previous {
                break;
            }
        }
        if graph.manifests.iter().any(|(id, m)| {
            m.changes.iter().any(|c| needed.contains(&c.object))
                && matches!(graph.status(id), Ok(Admission::Staged { .. }))
        }) {
            return Err(CoreError::new(
                "syncIncomplete",
                UiMessage::error_core_sync_hydrate_selected_incomplete_transaction_before_applying_content(),
            ));
        }
        let expected_versions: Vec<_> = needed
            .iter()
            .map(|k| Dependency {
                object: k.clone(),
                revisions: revisions(&current[k]),
            })
            .collect();
        let mut states = BTreeMap::new();
        for key in &needed {
            let view = &current[key];
            if let Some(resolution) = resolutions.iter().find(|r| &r.object == key) {
                if resolution.revisions != revisions(view) || view.versions.len() < 2 {
                    return Err(CoreError::new(
                        "syncConflictChanged",
                        UiMessage::error_core_sync_review_exact_current_conflict_branches_before_resolving(),
                    ));
                }
                for version in &view.versions {
                    if graph
                        .unsettled_dependencies(&version.commit)?
                        .iter()
                        .any(|dependency| dependency != key)
                    {
                        return Err(CoreError::new(
                            "syncSemanticDecisionRequired",
                            UiMessage::error_core_sync_reviewed_conflict_has_changed_semantic_dependencies_use_explicit_typed_revalidation_decision(),
                        ));
                    }
                }
            } else if view.versions.len() != 1 {
                return Err(CoreError::new(
                    "syncConflict",
                    UiMessage::error_core_sync_selected_transaction_has_unresolved_concurrent_versions(),
                ));
            } else {
                let version = &view.versions[0];
                if !graph.unsettled_dependencies(&version.commit)?.is_empty() {
                    return Err(CoreError::new(
                        "syncDependencyConflict",
                        UiMessage::error_core_sync_incoming_version_requires_revalidation_against_changed_semantic_dependencies(),
                    ));
                }
                states.insert(key.clone(), version.clone());
            }
        }
        let publication = if resolutions.is_empty() {
            if !supplied.is_empty() {
                return Err(invalid(UiMessage::error_core_sync_merge_blobs_unexpected()));
            }
            None
        } else {
            if supplied.len() > MAX_FILES
                || supplied.values().map(Vec::len).sum::<usize>() > MAX_BLOB_BYTES
            {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_sync_merge_content_exceeds_bounded_operation_budget(),
                ));
            }
            for (id, bytes) in &supplied {
                graph.offer_blob(id, bytes.clone())?;
            }
            let parents = needed
                .iter()
                .flat_map(|k| current[k].versions.iter().map(|v| v.commit.clone()))
                .collect();
            let mut resolutions = resolutions.clone();
            for resolution in &mut resolutions {
                resolution.state.file_revisions = resolution
                    .state
                    .files
                    .iter()
                    .map(|(path, digest)| {
                        let retained = current[&resolution.object]
                            .versions
                            .iter()
                            .find(|v| {
                                v.state.files.get(path) == Some(digest)
                                    && v.state.file_revisions.get(path)
                                        == resolution.state.file_revisions.get(path)
                            })
                            .and_then(|v| v.state.file_revisions.get(path))
                            .cloned();
                        (path.clone(), retained.unwrap_or_else(fresh))
                    })
                    .collect();
            }
            let manifest = Manifest {
                version: 1,
                workspace_id: identity.workspace_id.clone(),
                replica_id: setup.replica_id.clone(),
                operation_id: request.operation_id.clone(),
                parents,
                changes: resolutions
                    .iter()
                    .map(|r| Change {
                        object: r.object.clone(),
                        before: r.revisions.clone(),
                        revision: fresh(),
                        after: r.state.clone(),
                    })
                    .collect(),
                dependencies: needed
                    .difference(&requested)
                    .map(|k| Dependency {
                        object: k.clone(),
                        revisions: revisions(&current[k]),
                    })
                    .collect(),
            };
            let id = manifest.id()?;
            graph.offer_manifest(&id, manifest.clone())?;
            if graph.status(&id)? != Admission::Accepted {
                return Err(invalid(
                    UiMessage::error_core_sync_merged_content_or_predecessor_invalid(),
                ));
            }
            for c in &manifest.changes {
                states.insert(
                    c.object.clone(),
                    Version {
                        commit: id.clone(),
                        revision: c.revision.clone(),
                        state: c.after.clone(),
                    },
                );
            }
            let used: BTreeSet<_> = manifest.blobs().into_iter().cloned().collect();
            if supplied.keys().any(|k| !used.contains(k)) {
                return Err(invalid(
                    UiMessage::error_core_sync_merge_contains_unrelated_blobs(),
                ));
            }
            Some(SyncPublication {
                root: display(&root),
                identity: identity.clone(),
                manifest,
                blobs: supplied,
                locations: vec![],
            })
        };
        let identity = identity.clone();
        self.prepare_sync_application_input(super::relocate::RelocationInput {
            request,
            identity,
            fingerprint,
            local,
            states,
            expected_versions,
            selection: requested,
            publication,
            graph,
        })
    }

    pub(crate) fn prepare_sync_application_input(
        &mut self,
        input: super::relocate::RelocationInput,
    ) -> Result<OperationPreview> {
        let mut plan = self.build_sync_application_plan(input)?;
        self.freeze_sync_revalidation_if_needed(&mut plan)?;
        self.stage_frozen_sync_application_plan(plan)
    }

    /// Build the same original writer plan without allocating a second
    /// transaction or publishing a prepared decision before evidence exists.
    pub(crate) fn build_sync_application_plan(
        &mut self,
        input: super::relocate::RelocationInput,
    ) -> Result<Plan> {
        let super::relocate::RelocationInput {
            request,
            identity,
            fingerprint,
            local,
            states,
            expected_versions,
            selection: requested,
            publication,
            graph,
        } = input;
        let root = absolute_path(Path::new(&request.root))?;
        if states.iter().any(|(key, version)| {
            local.get(key).is_some_and(|old| {
                super::relocate::placement_changed(&old.state, &version.state)
                    || super::relocate::members_removed(&old.state, &version.state)
            })
        }) {
            return self.build_sync_relocation_plan(super::relocate::RelocationInput {
                request: request.clone(),
                identity: identity.clone(),
                fingerprint,
                local,
                states,
                expected_versions,
                selection: requested,
                publication,
                graph,
            });
        }
        let kind = if matches!(request.intent, OperationIntent::RevalidateSync { .. }) {
            "revalidateSync"
        } else if publication.is_some() {
            "mergeSync"
        } else {
            "applySync"
        };
        let plan_id = fresh();
        let mut preview = OperationPreview {
            operation_id: request.operation_id.clone(),
            plan_id: plan_id.clone(),
            kind: kind.into(),
            status: "prepared".into(),
            changes: vec![],
            node_map: BTreeMap::new(),
            new_node_id: None,
            revisions: BTreeMap::new(),
            resource_map: BTreeMap::new(),
            losses: vec![],
        };
        let mut locations = BTreeMap::new();
        let mut remaining: BTreeSet<_> = states.keys().cloned().collect();
        while !remaining.is_empty() {
            let prior = remaining.len();
            for key in remaining.clone() {
                let state = &states[&key].state;
                let directory = if state.retired {
                    let original = local.get(&key).filter(|old| old.state == *state).ok_or_else(|| {
                        CoreError::new("syncStructureRequired", UiMessage::error_core_sync_changed_or_new_retired_object_requires_original_guarded_structure_application())
                    })?;
                    PathBuf::from(&original.directory)
                } else if matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                    // Preserve a previously admitted root note's physical
                    // locator for content-only updates. Replica envelopes are
                    // local; an unchanged portable placement is not a request
                    // to move the original library into its own child folder.
                    if let Some(original) = local.get(&key).filter(|old| {
                        matches!(key, ObjectKey::Node(_))
                            && Path::new(&old.directory) == root
                            && !super::relocate::placement_changed(&old.state, state)
                    }) {
                        locations.insert(key.clone(), PathBuf::from(&original.directory));
                        remaining.remove(&key);
                        continue;
                    }
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
                locations.insert(key.clone(), directory);
                remaining.remove(&key);
            }
            if remaining.len() == prior {
                return Err(invalid(
                    UiMessage::error_core_sync_placement_parent_or_cycle(),
                ));
            }
        }
        let mut directory_claims = BTreeMap::new();
        for (key, directory) in &locations {
            if matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                let name = display(directory).to_lowercase();
                if directory_claims.insert(name.clone(), key.clone()).is_some()
                    || local.values().any(|o| {
                        &o.object != key
                            && matches!(o.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                            && o.directory.to_lowercase() == name
                    })
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_incoming_directory_occupied(),
                    ));
                }
            }
        }
        let mut created_directories = BTreeSet::new();
        let mut births = Vec::new();
        let mut before = Vec::new();
        let mut after = Vec::new();
        let mut blobs = BTreeMap::new();
        let mut files = Vec::new();
        let mut changes = Vec::new();
        let mut write_parents = Vec::new();
        let mut paths = BTreeSet::new();
        let mut resource_ids = BTreeSet::new();
        for (key, version) in states {
            let old = local.get(&key);
            let state = &version.state;
            let directory = &locations[&key];
            if old.is_some_and(|old| {
                state.parent != old.state.parent
                    || state.name != old.state.name
                    || state.retired != old.state.retired
                    || Path::new(&old.directory) != directory
                    || old.state.files.keys().any(|p| !state.files.contains_key(p))
            }) {
                return Err(CoreError::new(
                    "syncStructureRequired",
                    UiMessage::error_core_sync_placement_retirement_and_file_removal_require_guarded_structure_application(),
                ));
            }
            if old.is_none() && matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
                ensure_absent(&self.backend, directory)?;
            }
            let resources = validate_state(&graph, &key, state)?;
            if let ObjectKey::Node(id) = &key {
                if old.is_some() {
                    let canonical = if state.retired {
                        let path = directory.join(format!("{}.typ", state.name));
                        self.qualified_query_node_at(
                            &root,
                            &path,
                            &weftext_knowledge::NodeRef {
                                workspace_id: identity.workspace_id.clone(),
                                node_id: id.clone(),
                            },
                            "trash",
                        )?;
                        capture_optional(&self.backend, &path)?
                            .ok_or_else(|| invalid("The original retired source is unavailable"))?
                    } else {
                        self.managed_node(&root, directory, false)?
                    };
                    if self
                        .admitted_binding(&canonical.stamp.identity)?
                        .is_none_or(|b| b.node_id != *id)
                    {
                        return Err(invalid(
                            UiMessage::error_core_sync_original_node_admission_changed(),
                        ));
                    }
                } else {
                    births.push(NewBirth {
                        path: display(&directory.join(format!("{}.typ", state.name))),
                        node_id: id.clone(),
                    });
                }
                preview.node_map.insert(id.clone(), id.clone());
            }
            if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = &key
                && old.is_some()
            {
                let guard =
                    ParentGuard::acquire(&self.backend, &directory.join("weftext.directory.json"))?;
                if !self.directory_admitted(
                    &identity.workspace_id,
                    id,
                    &guard.directory_identity()?,
                )? {
                    return Err(invalid(
                        UiMessage::error_core_sync_original_directory_admission_missing(),
                    ));
                }
            }
            for (relative, digest) in &state.files {
                let path = directory.join(relative);
                if !paths.insert(display(&path).to_lowercase()) {
                    return Err(invalid(UiMessage::error_core_sync_incoming_files_overlap()));
                }
                self.check_pending(&path)?;
                let mut parent = path.parent().unwrap();
                while !self.backend.entry_exists(parent)? && parent != root {
                    created_directories.insert(display(
                        parent
                            .strip_prefix(&root)
                            .map_err(|_| invalid("Incoming file escaped root"))?,
                    ));
                    parent = parent
                        .parent()
                        .ok_or_else(|| invalid("Incoming file escaped root"))?;
                }
                let captured = if self.backend.entry_exists(path.parent().unwrap())? {
                    capture_optional(&self.backend, &path)?
                } else {
                    None
                };
                if captured.as_ref().map(|f| hash(&f.bytes)).as_ref()
                    != old.and_then(|o| o.state.files.get(relative))
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_actual_local_content_differs_from_installed_replica_base_preserve_and_explicitly_adopt_it_first(),
                    ));
                }
                if matches!(key, ObjectKey::Tags)
                    && captured
                        .as_ref()
                        .map(|f| self.accepted_revision(f).map(|revision| revision.is_none()))
                        .transpose()?
                        .unwrap_or(false)
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_tag_registry_original_observation_changed(),
                    ));
                }
                let bytes = graph
                    .blob(digest)
                    .ok_or_else(|| invalid(UiMessage::error_core_sync_incoming_content_missing()))?
                    .to_vec();
                if let Some(id) = resources.get(relative) {
                    if !resource_ids.insert(id.clone()) {
                        return Err(invalid(
                            UiMessage::error_core_sync_resource_identity_duplicate_across_objects(),
                        ));
                    }
                    let ObjectKey::Node(node_id) = &key else {
                        return Err(invalid(
                            UiMessage::error_core_sync_resource_node_owner_missing(),
                        ));
                    };
                    if let Some(file) = &captured {
                        if !self.admitted_resource(
                            id,
                            &identity.workspace_id,
                            node_id,
                            &file.stamp.identity,
                        )? {
                            return Err(invalid(
                                UiMessage::error_core_sync_incoming_resource_admission_mismatch(),
                            ));
                        }
                    } else {
                        preview.resource_map.insert(
                            display(&path),
                            format!("weftext-resource://{}/{id}", identity.workspace_id),
                        );
                    }
                } else if old.is_some()
                    && captured.is_none()
                    && relative != "weftext.annotations.json"
                    && relative != "weftext.resources.json"
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_new_file_content_role_required(),
                    ));
                }
                let revision = state.file_revisions[relative].clone();
                let prior = captured.as_ref().map(|f| f.bytes.clone());
                if let Some(file) = captured {
                    files.push(file);
                }
                let action = if prior.as_ref() == Some(&bytes) {
                    "readDependency"
                } else {
                    "receive"
                };
                preview.changes.push(change(&path, action, bytes.len()));
                preview.revisions.insert(display(&path), revision);
                if self.backend.entry_exists(path.parent().unwrap())? {
                    write_parents.push((
                        display(&path),
                        ParentGuard::acquire(&self.backend, &path)?.directory_identity()?,
                    ));
                }
                changes.push(FileChange {
                    path: display(&path),
                    base_id: None,
                    before: prior,
                    after: bytes.clone(),
                });
                blobs.insert(digest.clone(), bytes);
            }
            if let Some(old) = old {
                before.push(old.clone());
            }
            after.push(LocalObject {
                object: key,
                directory: display(directory),
                commit: version.commit,
                revision: version.revision,
                state: version.state,
            });
        }
        if changes.is_empty()
            || changes.len() > MAX_FILES
            || changes
                .iter()
                .map(|c| c.after.len() + c.before.as_ref().map_or(0, Vec::len))
                .sum::<usize>()
                > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_incoming_application_exceeds64_mi_b5000_file_transaction_budget(),
            ));
        }
        if before == after {
            return Err(CoreError::new(
                "syncCurrent",
                UiMessage::error_core_sync_selected_objects_already_have_these_installed_versions(),
            ));
        }
        let receipt = OperationReceipt {
            operation_id: request.operation_id.clone(),
            plan_id,
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
            identity: identity.clone(),
            before,
            after,
            expected_versions,
            selection: requested,
            blobs,
            semantic_evidence: None,
        });
        plan.sync_publication = publication;
        let mutation = if !births.is_empty()
            || !created_directories.is_empty()
            || after_has_new_object(plan.sync_application.as_ref().unwrap())
        {
            for relative in &created_directories {
                let directory = root.join(relative);
                if self.backend.entry_exists(directory.parent().unwrap())? {
                    write_parents.push((
                        display(&directory),
                        ParentGuard::acquire(&self.backend, &directory)?.directory_identity()?,
                    ));
                }
            }
            let root_member = root.join(".sync-root");
            write_parents.push((
                display(&root_member),
                ParentGuard::acquire(&self.backend, &root_member)?.directory_identity()?,
            ));
            let (output, updates): (Vec<_>, Vec<_>) =
                changes.into_iter().partition(|c| c.before.is_none());
            Mutation::CreateTree {
                target: display(&root),
                existing_target: true,
                directories: created_directories.into_iter().collect(),
                files: output,
                births,
                updates,
            }
        } else {
            Mutation::Update { files: changes }
        };
        plan.transaction = Some(FrozenOperation {
            backup_private_digest: None,
            request,
            preview: preview.clone(),
            files,
            directories: vec![],
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

    pub(crate) fn stage_frozen_sync_application_plan(
        &mut self,
        plan: Plan,
    ) -> Result<OperationPreview> {
        self.validate_sync_plan_participants(&plan)?;
        let preview = plan.transaction.as_ref().unwrap().preview.clone();
        self.stage(plan)?;
        Ok(preview)
    }

    pub(crate) fn freeze_sync_revalidation_if_needed(&mut self, plan: &mut Plan) -> Result<()> {
        let resolutions = match &plan.transaction.as_ref().unwrap().request.intent {
            OperationIntent::RevalidateSync { resolutions, .. } => resolutions.clone(),
            _ => return Ok(()),
        };
        let evidence = self.freeze_sync_revalidation_plan(plan, &resolutions)?;
        plan.sync_application.as_mut().unwrap().semantic_evidence = Some(evidence);
        Ok(())
    }

    pub(crate) fn validate_sync_application(&self, plan: &Plan) -> Result<()> {
        let Some(application) = &plan.sync_application else {
            return Ok(());
        };
        self.validate_sync_application_basis(application)?;
        self.publish_sync(&application.root)?;
        self.validate_sync_application_versions(plan, application)
    }
    /// Inspect the same original replica cut without publishing a fresh outbox.
    /// Committed decisions are handled by the recovery caller before this gate.
    pub(crate) fn validate_sync_recovery_cut(&self, plan: &Plan) -> Result<()> {
        let Some(application) = &plan.sync_application else {
            return Ok(());
        };
        self.validate_sync_application_basis(application)?;
        self.validate_sync_application_versions(plan, application)
    }
    fn validate_sync_application_basis(&self, application: &SyncApplication) -> Result<()> {
        self.sync_local_owner()?;
        let setup = self.sync_setup_for(Path::new(&application.root))?;
        if setup.identity != application.identity {
            return Err(invalid(
                UiMessage::error_core_sync_replica_identity_changed(),
            ));
        }
        let (local, _) = self.sync_local_state(setup)?;
        if application
            .before
            .iter()
            .any(|v| local.get(&v.object) != Some(v))
            || application.after.iter().any(|v| {
                !application.before.iter().any(|old| old.object == v.object)
                    && local.contains_key(&v.object)
            })
        {
            return Err(CoreError::new(
                "syncBasisChanged",
                UiMessage::error_core_sync_local_commit_changed_reviewed_replica_basis(),
            ));
        }
        Ok(())
    }
    fn validate_sync_application_versions(
        &self,
        plan: &Plan,
        application: &SyncApplication,
    ) -> Result<()> {
        let current = self.read_sync_history(&application.identity)?.objects()?;
        if application.expected_versions.iter().any(|expected| {
            current
                .get(&expected.object)
                .is_none_or(|v| revisions(v) != expected.revisions)
        }) {
            return Err(CoreError::new(
                "syncConflictChanged",
                UiMessage::error_core_sync_received_versions_changed_after_review_retain_original_proposal_and_review_again(),
            ));
        }
        if let Some(evidence) = &application.semantic_evidence {
            let transaction = plan.transaction.as_ref().ok_or_else(|| {
                invalid(
                    UiMessage::error_core_sync_semantic_evidence_original_transaction_required(),
                )
            })?;
            self.validate_sync_semantic_request_evidence(
                &transaction.request,
                application,
                evidence,
            )?;
        }
        Ok(())
    }
}
fn after_has_new_object(application: &SyncApplication) -> bool {
    application
        .after
        .iter()
        .any(|v| !application.before.iter().any(|old| old.object == v.object))
}
fn revisions(view: &ObjectView) -> BTreeSet<String> {
    view.versions.iter().map(|v| v.revision.clone()).collect()
}

pub(crate) fn validate_state(
    graph: &ReplicaGraph,
    key: &ObjectKey,
    state: &ObjectState,
) -> Result<BTreeMap<String, String>> {
    let mut resources = BTreeMap::new();
    if let ObjectKey::Node(id) = key {
        let source = state
            .files
            .get(&format!("{}.typ", state.name))
            .and_then(|b| graph.blob(b))
            .ok_or_else(|| invalid(UiMessage::error_core_sync_node_canonical_source_missing()))?;
        crate::check_size(source)?;
        if weftext_engine::inspect(&crate::decode(source)?).id.as_ref() != Some(id) {
            return Err(invalid(
                UiMessage::error_core_sync_incoming_canonical_node_identity_changed(),
            ));
        }
        if let Some(blob) = state.files.get("weftext.resources.json") {
            let manifest: serde_json::Value = serde_json::from_slice(graph.blob(blob).unwrap())?;
            let entries = manifest["resources"]
                .as_array()
                .filter(|_| manifest["version"] == 1)
                .ok_or_else(|| {
                    invalid(UiMessage::error_core_sync_owned_resource_manifest_invalid())
                })?;
            let mut ids = BTreeSet::new();
            let mut paths = BTreeSet::new();
            for entry in entries {
                let id = entry["resourceId"].as_str().ok_or_else(|| {
                    invalid(UiMessage::error_core_sync_resource_identity_missing())
                })?;
                let path = entry["path"]
                    .as_str()
                    .ok_or_else(|| invalid(UiMessage::error_core_sync_resource_path_missing()))?;
                uuid(id)?;
                super::relative(path)?;
                if !ids.insert(id)
                    || !paths.insert(path.to_lowercase())
                    || !state.files.contains_key(path)
                    || path == format!("{}.typ", state.name)
                    || matches!(
                        path,
                        "weftext.resources.json"
                            | "weftext.annotations.json"
                            | "weftext.directory.json"
                    )
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_owned_resource_binding_duplicate_or_invalid(),
                    ));
                }
                resources.insert(path.into(), id.into());
            }
        }
    }
    for (path, digest) in &state.files {
        let bytes = graph
            .blob(digest)
            .ok_or_else(|| invalid(UiMessage::error_core_sync_incoming_file_blob_missing()))?;
        if matches!(key, ObjectKey::Tags) {
            crate::directory_identity::validate_sync_tags(bytes)?;
        }
        if matches!(key, ObjectKey::Settings) {
            crate::author_order::validate_workspace_settings(bytes)?;
            crate::author_order::validate_workspace_settings_scope(bytes, &graph.workspace_id)?;
        }
        if matches!(key, ObjectKey::Rules) {
            crate::validate_library_rules(bytes)?;
        }
        if matches!(
            path.as_str(),
            "weftext.directory.json" | "weftext.annotations.json"
        ) || !matches!(key, ObjectKey::Node(_) | ObjectKey::Directory(_))
        {
            let _: serde_json::Value = serde_json::from_slice(bytes)?;
        }
        if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = key
            && path == "weftext.directory.json"
        {
            crate::directory_identity::validate_directory_marker(bytes)?;
            if serde_json::from_slice::<serde_json::Value>(bytes)?["directoryId"] != *id {
                return Err(invalid(
                    UiMessage::error_core_sync_directory_identity_changed(),
                ));
            }
        }
    }
    Ok(resources)
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod tests;
