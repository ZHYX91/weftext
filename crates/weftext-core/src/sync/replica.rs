//! Local replica binding and publication outbox. The original Core plan stores
//! the exact portable result before author writes; its existing commit marker
//! decides whether that result may be published. Transport failures cannot undo
//! a saved author operation, and retries cannot allocate another causal branch.
use super::*;
use crate::{
    Core, Path, absolute_path, display, fresh,
    journal::Plan,
    operations::{FileChange, Mutation},
    valid_operation_id,
};
use weftext_messages::UiMessage;

type LocalState<'a> = (
    BTreeMap<ObjectKey, LocalObject>,
    BTreeMap<String, &'a Vec<u8>>,
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LocalObject {
    pub object: ObjectKey,
    pub directory: String,
    pub commit: String,
    pub revision: String,
    pub state: ObjectState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncSetup {
    pub root: String,
    pub replica_id: String,
    pub identity: SyncIdentity,
    pub initial: Vec<LocalObject>,
    pub bundle: SyncBundle,
    #[serde(default)]
    pub selection: Option<BTreeSet<ObjectKey>>,
    /// Original observed receiver objects, never retrofitted from current paths.
    #[serde(default)]
    pub physical_bindings: Vec<SyncPhysicalBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncPhysicalBinding {
    pub object: ObjectKey,
    pub relative_path: String,
    pub stamp: crate::FileStamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SyncPublication {
    pub root: String,
    pub identity: SyncIdentity,
    pub manifest: Manifest,
    pub blobs: BTreeMap<String, Vec<u8>>,
    /// Private physical locators, never part of the portable manifest.
    pub locations: Vec<(ObjectKey, String)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncBindingInfo {
    pub root: String,
    pub identity: SyncIdentity,
    pub replica_id: String,
    pub pending_publications: usize,
    pub partial: bool,
    pub selected_objects: Option<BTreeSet<ObjectKey>>,
    pub installed_objects: BTreeSet<ObjectKey>,
    pub installed_versions: Vec<(ObjectKey, String)>,
}

fn setup_fingerprint(root: &Path, workspace: &str) -> Result<String> {
    Ok(hash(&serde_json::to_vec(
        &serde_json::json!({"kind":"enableSync","root":display(root),"workspace":workspace}),
    )?))
}

impl Core {
    /// Immutable result of the original setup decision, independent of later
    /// replica state, publications or the current path's binding.
    pub(crate) fn original_sync_setup_fact(&self, plan: &Plan) -> Result<serde_json::Value> {
        let invalid = || {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_sync_original_setup_evidence_invalid(),
            )
        };
        let setup = plan.sync_setup.as_ref().ok_or_else(invalid)?;
        if plan.kind != "enableSync"
            || plan.transaction.is_some()
            || plan.before.is_some()
            || !plan.after.is_empty()
            || plan.identity.is_some()
            || plan.receipt.is_some()
            || !plan.files.is_empty()
            || plan.node_id.is_some()
            || plan.history_restore.is_some()
            || plan.sync_publication.is_some()
            || plan.sync_application.is_some()
            || plan.content_revision.is_some()
            || plan.path != setup.root
            || plan.workspace_id.as_ref() != Some(&setup.identity.workspace_id)
            || setup.selection.is_some()
            || setup_fingerprint(Path::new(&setup.root), &setup.identity.workspace_id)?
                != plan.fingerprint
        {
            return Err(invalid());
        }
        valid_operation_id(&setup.replica_id)?;
        let mut graph = ReplicaGraph::new(&setup.identity.workspace_id, &setup.identity.genesis)?;
        for (id, bytes) in &setup.bundle.blobs {
            graph.offer_blob(id, bytes.clone())?;
        }
        for (id, manifest) in &setup.bundle.manifests {
            graph.offer_manifest(id, manifest.clone())?;
        }
        if graph.status(&setup.identity.genesis)? != Admission::Accepted
            || setup
                .bundle
                .manifests
                .get(&setup.identity.genesis)
                .is_none_or(|genesis| {
                    genesis.operation_id != plan.operation_id
                        || genesis.replica_id != setup.replica_id
                })
        {
            return Err(invalid());
        }
        for original in &setup.initial {
            let manifest = setup
                .bundle
                .manifests
                .get(&original.commit)
                .ok_or_else(invalid)?;
            if graph.status(&original.commit)? != Admission::Accepted
                || !manifest.changes.iter().any(|change| {
                    change.object == original.object
                        && change.revision == original.revision
                        && change.after == original.state
                })
            {
                return Err(invalid());
            }
        }
        Ok(
            serde_json::json!({"operationId":plan.operation_id,"kind":"enableSync","path":setup.root,
            "root":setup.root,"workspaceId":setup.identity.workspace_id,"identity":setup.identity,"replicaId":setup.replica_id}),
        )
    }
    pub fn enable_sync(
        &mut self,
        root: impl AsRef<Path>,
        operation_id: &str,
    ) -> Result<SyncBindingInfo> {
        self.sync_local_owner()?;
        valid_operation_id(operation_id)?;
        let root = absolute_path(root.as_ref())?;
        let workspace = self.require_workspace(&root)?;
        let fingerprint = setup_fingerprint(&root, &workspace.workspace_id)?;
        if let Some(plan) = self.operations.get(operation_id).cloned() {
            if plan.fingerprint != fingerprint
                || plan.sync_setup.is_none()
                || self.resolved(operation_id)?
            {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_sync_operation_does_not_name_same_sync_setup(),
                ));
            }
            let setup = plan.sync_setup.as_ref().unwrap();
            // Setup has no author-tree writes; retry its exact retained bytes.
            self.receive_sync(&setup.identity, setup.bundle.clone())?;
            if !self.committed(operation_id)? {
                self.commit(&plan)?;
            }
            return self.sync_binding_info(&root);
        }
        if self
            .sync_setups()?
            .iter()
            .any(|s| s.root == display(&root) || s.identity.workspace_id == workspace.workspace_id)
        {
            return Err(CoreError::new(
                "syncAlreadyEnabled",
                UiMessage::error_core_sync_use_existing_replica_binding_and_original_operation(),
            ));
        }
        self.check_sync_setup_pending(&workspace.workspace_id)?;
        let captured = self.admitted_compile_snapshot(&root, "live")?;
        let trash = match self.trash_admitted_snapshot(&root) {
            Ok(trash) => trash,
            Err(error) => {
                self.release_admitted_cut(&captured.snapshot.cut_id);
                return Err(error);
            }
        };
        let setup = self.initial_sync_setup(
            &root,
            &workspace.workspace_id,
            operation_id,
            &captured,
            &trash,
        );
        self.release_admitted_cut(&captured.snapshot.cut_id);
        self.release_admitted_cut(&trash.cut_id);
        let setup = setup?;
        let mut plan = Plan::file(
            operation_id.into(),
            fingerprint,
            &root,
            None,
            vec![],
            None,
            "enableSync",
        );
        plan.workspace_id = Some(workspace.workspace_id);
        plan.sync_setup = Some(setup.clone());
        self.stage(plan.clone())?;
        self.receive_sync(&setup.identity, setup.bundle)?;
        self.commit(&plan)?;
        self.sync_binding_info(&root)
    }

    fn initial_sync_setup(
        &self,
        root: &Path,
        workspace_id: &str,
        operation_id: &str,
        captured: &crate::AdmittedCompileSnapshot,
        trash: &crate::AdmittedSnapshot,
    ) -> Result<SyncSetup> {
        if !captured.snapshot.complete || !trash.complete {
            return Err(CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_sync_sync_setup_requires_qualified_content_inventory(),
            ));
        }
        let tags = captured.snapshot.tags.as_ref().ok_or_else(|| {
            CoreError::new(
                "coverageIncomplete",
                UiMessage::error_core_sync_sync_setup_requires_original_complete_tags_capture(),
            )
        })?;
        self.qualified_tag_document_from_snapshot(root, tags)?;
        let trash_roots = self.original_trash_roots(root)?;
        let retired_root = |directory: &Path| -> Result<bool> {
            for original in &trash_roots {
                if crate::backend::namespace_path_compare(directory, &original.path)?
                    == std::cmp::Ordering::Equal
                {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        let replica_id = fresh();
        let genesis = Manifest {
            version: 1,
            workspace_id: workspace_id.into(),
            replica_id: replica_id.clone(),
            operation_id: operation_id.into(),
            parents: BTreeSet::new(),
            changes: vec![],
            dependencies: vec![],
        };
        let identity = SyncIdentity {
            workspace_id: workspace_id.into(),
            genesis: genesis.id()?,
        };
        let mut bundle = SyncBundle {
            manifests: BTreeMap::from([(identity.genesis.clone(), genesis)]),
            blobs: BTreeMap::new(),
        };
        let mut objects = BTreeMap::new();
        for source in captured
            .snapshot
            .files
            .iter()
            .chain(trash.files.iter())
            .filter(|f| f.node_id.is_some())
        {
            let path = Path::new(&source.path);
            let directory = path.parent().unwrap();
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            let object = ObjectKey::Node(source.node_id.clone().unwrap());
            if objects.contains_key(&object) {
                return Err(invalid(
                    UiMessage::error_core_sync_replica_setup_node_duplicate(),
                ));
            }
            objects.insert(
                object.clone(),
                LocalObject {
                    object,
                    directory: display(directory),
                    commit: String::new(),
                    revision: fresh(),
                    state: ObjectState {
                        file_revisions: BTreeMap::new(),
                        parent: None,
                        name,
                        retired: retired_root(directory)?,
                        files: BTreeMap::new(),
                    },
                },
            );
        }
        // Directory identity must already be admitted; a copied marker is not
        // made authoritative merely because it is present in this capture.
        let mut physical_bindings = Vec::new();
        for (relative, bytes) in &captured.files {
            let path = root.join(relative);
            if path
                .file_name()
                .is_some_and(|n| n == "weftext.directory.json")
                && !objects
                    .values()
                    .any(|o| Path::new(&o.directory) == path.parent().unwrap())
            {
                let value: serde_json::Value = serde_json::from_slice(bytes)?;
                let id = value["directoryId"].as_str().ok_or_else(|| {
                    invalid(UiMessage::error_core_sync_replica_directory_marker_identity_missing())
                })?;
                let reference = format!("weftext-directory://{workspace_id}/{id}");
                let resolved = self.resolve_navigation_target(root, &reference)?;
                if resolved.status != "managed"
                    || resolved.path.as_deref() != Some(&display(path.parent().unwrap()))
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_replica_directory_marker_not_admitted(),
                    ));
                }
                let object = if path.parent() == Some(root) {
                    ObjectKey::RootDirectory(id.into())
                } else {
                    ObjectKey::Directory(id.into())
                };
                objects.insert(
                    object.clone(),
                    LocalObject {
                        object,
                        directory: display(path.parent().unwrap()),
                        commit: String::new(),
                        revision: fresh(),
                        state: ObjectState {
                            file_revisions: BTreeMap::new(),
                            parent: None,
                            name: path
                                .parent()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into(),
                            retired: retired_root(path.parent().unwrap())?,
                            files: BTreeMap::new(),
                        },
                    },
                );
            }
        }
        for (relative, bytes) in &captured.files {
            let path = root.join(relative);
            if let Some(object) = config_key(root, &path) {
                if matches!(object, ObjectKey::Tags) {
                    crate::directory_identity::validate_sync_tags(bytes)?;
                }
                if matches!(object, ObjectKey::Settings) {
                    crate::author_order::validate_workspace_settings(bytes)?;
                    crate::author_order::validate_workspace_settings_scope(bytes, workspace_id)?;
                }
                if matches!(object, ObjectKey::Rules) {
                    crate::validate_library_rules(bytes)?;
                }
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                objects.insert(
                    object.clone(),
                    LocalObject {
                        object,
                        directory: display(root),
                        commit: String::new(),
                        revision: fresh(),
                        state: ObjectState {
                            file_revisions: BTreeMap::new(),
                            parent: None,
                            name,
                            retired: false,
                            files: BTreeMap::new(),
                        },
                    },
                );
            }
            if let Some(object) = owner(&objects, root, &path) {
                let tracked = objects.get_mut(&object).unwrap();
                let name = relative_to(&path, Path::new(&tracked.directory))?;
                super::relative(&name)?;
                let blob = hash(bytes);
                let observed = crate::operations::capture_optional(&self.backend, &path)?
                    .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_replica_captured_replica_file_disappeared()))?;
                let frozen = self.admitted_cuts[&captured.snapshot.cut_id]
                    .bases
                    .iter()
                    .filter_map(|id| self.snapshots.get(id))
                    .filter_map(|b| b.captured.as_ref())
                    .find(|f| {
                        crate::history::history_relative_path(Path::new(&f.path), &path)
                            .is_some_and(|p| p.as_os_str().is_empty())
                    })
                    .ok_or_else(|| invalid(UiMessage::error_core_sync_replica_original_cut_physical_evidence_missing()))?;
                if observed.bytes != *bytes || observed.stamp != frozen.stamp {
                    return Err(invalid(
                        UiMessage::error_core_sync_replica_captured_file_changed_during_binding(),
                    ));
                }
                physical_bindings.push(SyncPhysicalBinding {
                    object: tracked.object.clone(),
                    relative_path: name.clone(),
                    stamp: observed.stamp.clone(),
                });
                let revision = self.accepted_revision(&observed)?.unwrap_or_else(fresh);
                tracked.state.file_revisions.insert(name.clone(), revision);
                tracked.state.files.insert(name, blob.clone());
                bundle.blobs.insert(blob, bytes.clone());
            }
        }
        let parent_lookup = objects.clone();
        for tracked in objects.values_mut() {
            if matches!(tracked.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                && !tracked.state.retired
            {
                tracked.state.parent = if Path::new(&tracked.directory) == root {
                    None
                } else {
                    parent_id(
                        &parent_lookup,
                        root,
                        Path::new(&tracked.directory).parent().unwrap(),
                    )?
                };
            }
            if let ObjectKey::Node(id) = &tracked.object {
                let canonical = format!("{}.typ", tracked.state.name);
                let blob = tracked.state.files.get(&canonical).ok_or_else(|| {
                    invalid(
                        UiMessage::error_core_sync_replica_captured_node_canonical_source_missing(),
                    )
                })?;
                if weftext_engine::inspect(&crate::decode(&bundle.blobs[blob])?)
                    .id
                    .as_ref()
                    != Some(id)
                {
                    return Err(invalid(
                        UiMessage::error_core_sync_replica_captured_source_identity_changed(),
                    ));
                }
            }
        }
        // Initial checkpoint records use only real parent/dependency edges, so
        // choosing one note does not require downloading unrelated note bodies.
        let mut initial: Vec<_> = objects.into_values().collect();
        initial.sort_by_key(|o| Path::new(&o.directory).components().count());
        let mut birth_commits = BTreeMap::new();
        for tracked in &mut initial {
            let mut parents = BTreeSet::from([identity.genesis.clone()]);
            if let Some(parent) = &tracked.state.parent {
                parents.insert(birth_commits.get(parent).cloned().ok_or_else(|| {
                    invalid(UiMessage::error_core_sync_replica_initial_parent_checkpoint_missing())
                })?);
            }
            // An initial checkpoint still obeys the same lifetime rule as any
            // received history. The object already has original committed
            // admission; record its captured content birth before a separate
            // retirement successor, preserving the exact file revisions.
            let birth_revision = if tracked.state.retired {
                fresh()
            } else {
                tracked.revision.clone()
            };
            let mut birth_state = tracked.state.clone();
            birth_state.retired = false;
            let manifest = Manifest {
                version: 1,
                workspace_id: workspace_id.into(),
                replica_id: replica_id.clone(),
                operation_id: fresh(),
                parents,
                changes: vec![Change {
                    object: tracked.object.clone(),
                    before: BTreeSet::new(),
                    revision: birth_revision.clone(),
                    after: birth_state,
                }],
                dependencies: vec![],
            };
            tracked.commit = manifest.id()?;
            bundle.manifests.insert(tracked.commit.clone(), manifest);
            if tracked.state.retired {
                let retirement = Manifest {
                    version: 1,
                    workspace_id: workspace_id.into(),
                    replica_id: replica_id.clone(),
                    operation_id: fresh(),
                    parents: BTreeSet::from([tracked.commit.clone()]),
                    changes: vec![Change {
                        object: tracked.object.clone(),
                        before: BTreeSet::from([birth_revision]),
                        revision: tracked.revision.clone(),
                        after: tracked.state.clone(),
                    }],
                    dependencies: vec![],
                };
                tracked.commit = retirement.id()?;
                bundle.manifests.insert(tracked.commit.clone(), retirement);
            }
            birth_commits.insert(tracked.object.clone(), tracked.commit.clone());
        }
        Ok(SyncSetup {
            root: display(root),
            replica_id,
            identity,
            initial,
            bundle,
            selection: None,
            physical_bindings,
        })
    }

    pub fn sync_binding_info(&self, root: impl AsRef<Path>) -> Result<SyncBindingInfo> {
        self.backend.check_path(&self.data_dir)?;
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        let graph = self.read_sync_history(&setup.identity)?;
        let mut pending = 0;
        for publication in self.sync_publications(&setup.root)? {
            if !matches!(
                graph.status(&publication.manifest.id()?),
                Ok(Admission::Accepted)
            ) {
                pending += 1;
            }
        }
        let mut selected = setup.selection.clone();
        if let Some(selected) = &mut selected {
            for plan in self.operations.values() {
                if let Some(application) = &plan.sync_application
                    && application.root == setup.root
                    && self.committed(&plan.operation_id)?
                {
                    selected.extend(application.selection.iter().cloned());
                }
            }
        }
        let local = self.sync_local_state(setup)?.0;
        let installed_objects = local.keys().cloned().collect();
        let installed_versions = local
            .into_iter()
            .map(|(key, value)| (key, value.revision))
            .collect();
        Ok(SyncBindingInfo {
            root: setup.root.clone(),
            identity: setup.identity.clone(),
            replica_id: setup.replica_id.clone(),
            pending_publications: pending,
            partial: setup.selection.is_some(),
            selected_objects: selected,
            installed_objects,
            installed_versions,
        })
    }

    /// Publication drains original committed plans, never the latest filesystem
    /// bytes. Later edits cannot alter a pending earlier operation's transport.
    pub fn publish_sync(&self, root: impl AsRef<Path>) -> Result<SyncStatus> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        let mut bundle = SyncBundle::default();
        for publication in self.sync_publications(&setup.root)? {
            bundle
                .manifests
                .insert(publication.manifest.id()?, publication.manifest.clone());
            bundle.blobs.extend(
                publication
                    .blobs
                    .iter()
                    .map(|(id, bytes)| (id.clone(), bytes.clone())),
            );
        }
        self.receive_sync(&setup.identity, bundle)
    }

    pub(crate) fn check_sync_operation_access(&self, operation: &str) -> Result<()> {
        self.backend.check_path(&self.data_dir)?;
        if self
            .operations
            .get(operation)
            .is_some_and(|p| p.sync_setup.is_some() || p.sync_application.is_some())
        {
            self.sync_local_owner()?;
        }
        Ok(())
    }
    pub(crate) fn sync_local_owner(&self) -> Result<()> {
        if self.actor.is_some() {
            Err(CoreError::new(
                "syncOwnerRequired",
                UiMessage::error_core_sync_replica_management_requires_local_owner(),
            ))
        } else {
            Ok(())
        }
    }
    pub(crate) fn check_sync_setup_pending(&self, workspace: &str) -> Result<()> {
        for p in self.operations.values() {
            if p.sync_setup
                .as_ref()
                .is_some_and(|s| s.identity.workspace_id == workspace)
                && !self.committed(&p.operation_id)?
                && !self.resolved(&p.operation_id)?
            {
                return Err(CoreError::new(
                    "syncSetupPending",
                    UiMessage::error_core_sync_resume_or_resolve_original_replica_setup_before_allocating_another(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn sync_setups(&self) -> Result<Vec<&SyncSetup>> {
        // Classify original Plan roles before opening commit bodies. An
        // ordinary save is not a replica binding; relevant setup facts still
        // receive fresh original-commit qualification on every operation.
        let mut setups = Vec::new();
        for plan in self.operations.values() {
            if let Some(setup) = &plan.sync_setup
                && self.committed(&plan.operation_id)?
            {
                setups.push(setup);
            }
        }
        Ok(setups)
    }
    pub(crate) fn sync_setup_for(&self, root: &Path) -> Result<&SyncSetup> {
        let matches: Vec<_> = self
            .sync_setups()?
            .into_iter()
            .filter(|s| Path::new(&s.root) == root)
            .collect();
        if matches.len() != 1 {
            return Err(CoreError::new(
                "syncNotEnabled",
                UiMessage::error_core_sync_root_does_not_have_one_committed_replica_binding(),
            ));
        }
        Ok(matches.into_iter().next().unwrap())
    }
    fn sync_publications(&self, root: &str) -> Result<Vec<&SyncPublication>> {
        let mut records = Vec::new();
        for plan in self.operations.values() {
            if let Some(publication) = &plan.sync_publication
                && publication.root == root
                && self.committed(&plan.operation_id)?
            {
                if matches!(plan.kind.as_str(), "save" | "saveAs") {
                    self.validate_file_save_plan(plan)?;
                }
                let fact: serde_json::Value = serde_json::from_slice(
                    &self
                        .backend
                        .read_bytes(self.journal_path(&plan.operation_id, "commit"))?,
                )?;
                let sequence = fact["commitSequence"]
                    .as_u64()
                    .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_replica_publication_lacks_its_durable_commit_order()))?;
                records.push((sequence, publication));
            }
        }
        records.sort_by_key(|r| r.0);
        Ok(records.into_iter().map(|r| r.1).collect())
    }

    pub(crate) fn sync_local_state<'a>(&'a self, setup: &'a SyncSetup) -> Result<LocalState<'a>> {
        let mut objects: BTreeMap<_, _> = setup
            .initial
            .iter()
            .cloned()
            .map(|o| (o.object.clone(), o))
            .collect();
        let mut known_blobs: BTreeMap<_, _> = setup
            .bundle
            .blobs
            .iter()
            .map(|(id, bytes)| (id.clone(), bytes))
            .collect();
        let mut records = Vec::new();
        for p in self.operations.values() {
            if (p
                .sync_publication
                .as_ref()
                .is_some_and(|v| v.root == setup.root)
                || p.sync_application
                    .as_ref()
                    .is_some_and(|v| v.root == setup.root))
                && self.committed(&p.operation_id)?
            {
                if matches!(p.kind.as_str(), "save" | "saveAs") {
                    self.validate_file_save_plan(p)?;
                }
                let fact: serde_json::Value = serde_json::from_slice(
                    &self
                        .backend
                        .read_bytes(self.journal_path(&p.operation_id, "commit"))?,
                )?;
                let sequence = fact["commitSequence"]
                    .as_u64()
                    .ok_or_else(|| invalid(weftext_messages::UiMessage::error_core_replica_replica_event_lacks_commit_order()))?;
                records.push((sequence, p));
            }
        }
        records.sort_by_key(|r| r.0);
        for (_, plan) in records {
            if let Some(applied) = &plan.sync_application {
                objects.extend(applied.after.iter().cloned().map(|o| (o.object.clone(), o)));
                known_blobs.extend(applied.blobs.iter().map(|(id, bytes)| (id.clone(), bytes)));
                continue;
            }
            let prior = plan.sync_publication.as_ref().unwrap();
            let commit = prior.manifest.id()?;
            for c in &prior.manifest.changes {
                let location = prior
                    .locations
                    .iter()
                    .find(|(key, _)| key == &c.object)
                    .map(|(_, p)| p.clone())
                    .or_else(|| objects.get(&c.object).map(|o| o.directory.clone()))
                    .ok_or_else(|| {
                        invalid(
                            UiMessage::error_core_sync_replica_publication_object_locator_missing(),
                        )
                    })?;
                objects.insert(
                    c.object.clone(),
                    LocalObject {
                        object: c.object.clone(),
                        directory: location,
                        commit: commit.clone(),
                        revision: c.revision.clone(),
                        state: c.after.clone(),
                    },
                );
            }
            for (key, path) in &prior.locations {
                if let Some(o) = objects.get_mut(key) {
                    o.directory = path.clone();
                }
            }
            known_blobs.extend(prior.blobs.iter().map(|(id, bytes)| (id.clone(), bytes)));
        }
        Ok((objects, known_blobs))
    }

    pub(crate) fn freeze_sync_publication(&self, plan: &mut Plan) -> Result<()> {
        if plan.sync_setup.is_some()
            || plan.sync_publication.is_some()
            || plan.sync_application.is_some()
        {
            return Ok(());
        }
        if plan.transaction.as_ref().is_some_and(|t| {
            matches!(
                t.request.intent,
                crate::OperationIntent::BackupLibrary { .. }
            )
        }) {
            return Ok(());
        }
        let original_owner = if matches!(plan.kind.as_str(), "save" | "saveAs") {
            self.validate_file_save_plan(plan)?;
            match &plan.file_save.as_ref().unwrap().scope {
                crate::file_save_scope::QualifiedInputScope::Library { binding } => {
                    Some(&binding.workspace_id)
                }
                crate::file_save_scope::QualifiedInputScope::DeviceUnassigned => return Ok(()),
            }
        } else {
            plan.workspace_id.as_ref()
        };
        let matches: Vec<_> = self
            .sync_setups()?
            .into_iter()
            .filter(|s| {
                original_owner.is_none_or(|owner| owner == &s.identity.workspace_id)
                    && plan.transaction.as_ref().map_or_else(
                        || Path::new(&plan.path).starts_with(&s.root),
                        |t| t.request.root == s.root,
                    )
            })
            .collect();
        if matches.is_empty() {
            return Ok(());
        }
        if matches.len() != 1 {
            return Err(invalid(
                UiMessage::error_core_sync_replica_operation_crosses_bindings(),
            ));
        }
        let setup = &matches[0];
        let root = Path::new(&setup.root);
        // Hosted save publication is internal writer responsibility; it does
        // not expose any history or transport bytes to the authenticated actor.
        let (mut objects, known_blobs) = self.sync_local_state(setup)?;
        let before = objects.clone();
        let mut blobs = BTreeMap::new();
        let mut dependencies = BTreeSet::new();
        if let Some(transaction) = &plan.transaction {
            let mut tracked = BTreeSet::new();
            let mut leaves = BTreeSet::new();
            for file in &transaction.files {
                if let Some(key) = owner(&before, root, Path::new(&file.path)) {
                    let object = before.get(&key).ok_or_else(|| CoreError::new("syncExternalChange", UiMessage::error_core_sync_observed_configuration_has_no_installed_basis_explicitly_handle_it_before_publishing()))?;
                    let relative =
                        relative_to(Path::new(&file.path), Path::new(&object.directory))?;
                    if let Some(revision) = object.state.file_revisions.get(&relative) {
                        tracked.insert(key);
                        leaves.insert(revision.clone());
                    }
                }
            }
            let physical = self.sync_installed_stamps(&before, &tracked, Some(&leaves))?;
            for file in &transaction.files {
                if let Some(key) = owner(&before, root, Path::new(&file.path)) {
                    let object = before.get(&key).ok_or_else(|| CoreError::new("syncExternalChange", UiMessage::error_core_sync_observed_configuration_has_no_installed_basis_explicitly_handle_it_before_publishing()))?;
                    let relative =
                        relative_to(Path::new(&file.path), Path::new(&object.directory))?;
                    let stamp_changed = object
                        .state
                        .file_revisions
                        .get(&relative)
                        .map(|revision| {
                            physical
                                .get(revision)
                                .map(|stamp| stamp != &file.stamp)
                                .ok_or_else(|| {
                                    CoreError::new(
                                        "identityUnavailable",
                                        UiMessage::error_core_sync_installed_input_physical_evidence_unavailable(),
                                    )
                                })
                        })
                        .transpose()?
                        .unwrap_or(false);
                    if stamp_changed
                        || object
                            .state
                            .files
                            .get(&relative)
                            .is_some_and(|blob| *blob != hash(&file.bytes))
                    {
                        return Err(CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_observed_operation_input_differs_from_replica_base_explicitly_adopt_external_content_first(),
                        ));
                    }
                    dependencies.insert(key);
                }
            }
            match &transaction.mutation {
                Mutation::Compound { .. } => {
                    return Err(invalid(
                        UiMessage::error_core_sync_replica_compound_original_application_required(),
                    ));
                }
                Mutation::Update { files } => {
                    for file in files {
                        register_directory(&mut objects, root, file)?;
                    }
                    for file in files {
                        patch(&mut objects, root, file, &known_blobs, &mut blobs)?;
                    }
                }
                Mutation::CreateTree {
                    births,
                    files,
                    updates,
                    ..
                } => {
                    let mut entries: Vec<_> = births
                        .iter()
                        .map(|b| (Path::new(&b.path), Some(b), None))
                        .chain(
                            files
                                .iter()
                                .filter(|f| {
                                    Path::new(&f.path)
                                        .file_name()
                                        .is_some_and(|n| n == "weftext.directory.json")
                                })
                                .map(|f| (Path::new(&f.path), None, Some(f))),
                        )
                        .collect();
                    entries.sort_by_key(|(path, _, _)| path.components().count());
                    for (_, birth, marker) in entries {
                        if let Some(birth) = birth {
                            register_node(
                                &mut objects,
                                root,
                                Path::new(&birth.path),
                                &birth.node_id,
                            )?;
                        }
                        if let Some(marker) = marker {
                            register_directory(&mut objects, root, marker)?;
                        }
                    }
                    for file in files.iter().chain(updates) {
                        patch(&mut objects, root, file, &known_blobs, &mut blobs)?;
                    }
                }
                Mutation::Relocate {
                    source,
                    target,
                    old_leaf,
                    new_leaf,
                    ..
                } => {
                    let key = before
                        .iter()
                        .find(|(_, o)| {
                            Path::new(&o.directory) == Path::new(source)
                                && matches!(o.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                        })
                        .map(|(k, _)| k.clone())
                        .ok_or_else(|| {
                            invalid(
                                UiMessage::error_core_sync_replica_relocated_object_basis_missing(),
                            )
                        })?;
                    let parent = if Path::new(target).starts_with(root.join(".weftext-trash")) {
                        before[&key].state.parent.clone()
                    } else {
                        parent_id(&objects, root, Path::new(target).parent().unwrap())?
                    };
                    let node = objects.get_mut(&key).unwrap();
                    let source_hash = node.state.files.remove(old_leaf).ok_or_else(|| {
                        invalid(UiMessage::error_core_sync_replica_relocated_canonical_file_basis_missing())
                    })?;
                    node.state.files.insert(new_leaf.clone(), source_hash);
                    let revision = node.state.file_revisions.remove(old_leaf).ok_or_else(|| {
                        invalid(
                            UiMessage::error_core_sync_replica_relocated_canonical_revision_missing(
                            ),
                        )
                    })?;
                    node.state.file_revisions.insert(new_leaf.clone(), revision);
                    node.state.name = if matches!(key, ObjectKey::Directory(_)) {
                        Path::new(target)
                            .file_name()
                            .unwrap()
                            .to_string_lossy()
                            .into()
                    } else {
                        Path::new(new_leaf)
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .into()
                    };
                    node.state.parent = parent;
                    node.state.retired = Path::new(target).starts_with(root.join(".weftext-trash"));
                    for object in objects.values_mut() {
                        if let Ok(relative) = Path::new(&object.directory).strip_prefix(source) {
                            object.directory = if relative.as_os_str().is_empty() {
                                target.clone()
                            } else {
                                display(&Path::new(target).join(relative))
                            };
                        }
                    }
                }
            }
        } else if plan.kind == "createNote" {
            register_node(
                &mut objects,
                root,
                Path::new(&plan.path),
                plan.node_id.as_ref().ok_or_else(|| {
                    invalid(UiMessage::error_core_sync_replica_node_creation_identity_missing())
                })?,
            )?;
            patch(
                &mut objects,
                root,
                &FileChange {
                    path: plan.path.clone(),
                    base_id: None,
                    before: None,
                    after: plan.after.clone(),
                },
                &known_blobs,
                &mut blobs,
            )?;
        } else if matches!(plan.kind.as_str(), "save" | "saveAs") {
            patch(
                &mut objects,
                root,
                &FileChange {
                    path: plan.path.clone(),
                    base_id: None,
                    before: plan.before.clone(),
                    after: plan.after.clone(),
                },
                &known_blobs,
                &mut blobs,
            )?;
        } else {
            return Ok(());
        }
        let file_revisions: Vec<_> = if let Some(transaction) = &plan.transaction {
            transaction
                .preview
                .revisions
                .iter()
                .map(|(p, r)| (p.clone(), r.clone()))
                .collect()
        } else {
            plan.content_revision
                .clone()
                .or_else(|| plan.receipt.as_ref().map(|r| r.base_id.clone()))
                .map(|r| vec![(plan.path.clone(), r)])
                .unwrap_or_default()
        };
        for (path, revision) in file_revisions {
            if let Some(key) = owner(&objects, root, Path::new(&path)) {
                let object = objects.get_mut(&key).unwrap();
                let relative = relative_to(Path::new(&path), Path::new(&object.directory))?;
                if object.state.files.contains_key(&relative) {
                    object.state.file_revisions.insert(relative, revision);
                }
            }
        }
        for object in objects.values_mut() {
            for path in object.state.files.keys() {
                object
                    .state
                    .file_revisions
                    .entry(path.clone())
                    .or_insert_with(fresh);
            }
        }
        let mut changes = Vec::new();
        let mut parents = BTreeSet::new();
        let mut locations = Vec::new();
        for (key, after) in &objects {
            let original = before.get(key);
            if original.is_none_or(|o| o.directory != after.directory) {
                locations.push((key.clone(), after.directory.clone()));
            }
            if original.is_some_and(|o| o.state == after.state) {
                continue;
            }
            if let Some(o) = original {
                parents.insert(o.commit.clone());
            }
            if let Some(parent) = &after.state.parent {
                let p = before.get(parent);
                if let Some(p) = p {
                    dependencies.insert(p.object.clone());
                }
            }
            changes.push(Change {
                object: key.clone(),
                before: original
                    .map(|o| BTreeSet::from([o.revision.clone()]))
                    .unwrap_or_default(),
                revision: fresh(),
                after: after.state.clone(),
            });
        }
        if changes.is_empty() {
            return Ok(());
        }
        let dependencies: Vec<_> = dependencies
            .into_iter()
            .filter_map(|key| {
                before.get(&key).map(|o| {
                    parents.insert(o.commit.clone());
                    Dependency {
                        object: key,
                        revisions: BTreeSet::from([o.revision.clone()]),
                    }
                })
            })
            .collect();
        if parents.is_empty() {
            parents.insert(setup.identity.genesis.clone());
        }
        let manifest = Manifest {
            version: 1,
            workspace_id: setup.identity.workspace_id.clone(),
            replica_id: setup.replica_id.clone(),
            operation_id: plan.operation_id.clone(),
            parents,
            changes,
            dependencies,
        };
        manifest.id()?;
        plan.workspace_id = Some(setup.identity.workspace_id.clone());
        plan.sync_publication = Some(SyncPublication {
            root: setup.root.clone(),
            identity: setup.identity.clone(),
            manifest,
            blobs,
            locations,
        });
        Ok(())
    }

    /// Internal author-save qualification using the caller's already held leaf
    /// stamp. It grants no sync transport/management access and never reopens
    /// that share-zero file. A fresh editor Base is not an installed fact.
    pub(crate) fn validate_sync_source_stamp(
        &self,
        path: &Path,
        observed: &crate::FileStamp,
    ) -> Result<()> {
        for setup in self
            .sync_setups()?
            .into_iter()
            .filter(|s| path.starts_with(&s.root))
        {
            let local = self.sync_local_state(setup)?.0;
            let Some(key) = owner(&local, Path::new(&setup.root), path) else {
                continue;
            };
            let object = local.get(&key).ok_or_else(|| CoreError::new("syncExternalChange", UiMessage::error_core_sync_observed_configuration_has_no_installed_basis_explicitly_handle_it_before_saving()))?;
            let relative = relative_to(path, Path::new(&object.directory))?;
            let Some(revision) = object.state.file_revisions.get(&relative) else {
                continue;
            };
            let stamps = self.sync_installed_stamps(
                &local,
                &BTreeSet::from([key]),
                Some(&BTreeSet::from([revision.clone()])),
            )?;
            let installed = stamps.get(revision).ok_or_else(|| {
                CoreError::new(
                    "identityUnavailable",
                    UiMessage::error_core_sync_installed_source_physical_evidence_unavailable(),
                )
            })?;
            if installed != observed {
                return Err(CoreError::new(
                    "syncExternalChange",
                    UiMessage::error_core_sync_observed_file_differs_from_original_installed_physical_basis_explicitly_adopt_external_content_first(),
                ));
            }
        }
        Ok(())
    }
}

fn config_key(root: &Path, path: &Path) -> Option<ObjectKey> {
    if path.parent() != Some(root) {
        return None;
    }
    match path.file_name()?.to_str()? {
        ".weftext-schema.json" => Some(ObjectKey::Schema),
        ".weftext-settings.json" => Some(ObjectKey::Settings),
        ".weftext-tags.json" => Some(ObjectKey::Tags),
        ".weftext-rules" => Some(ObjectKey::Rules),
        _ => None,
    }
}
fn owner(
    objects: &BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    path: &Path,
) -> Option<ObjectKey> {
    // A canonical ordinary note may share the library root directory. The
    // envelope remains workspace control, never part of that note's payload.
    if path.parent() == Some(root)
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|name| {
                matches!(
                    name.to_ascii_lowercase().as_str(),
                    ".weftext-workspace.json" | ".weftext-mvp-commit.json" | ".weftext-format"
                )
            })
    {
        return None;
    }
    if let Some(key) = config_key(root, path) {
        return Some(key);
    }
    if path == root.join("weftext.directory.json") {
        return objects
            .keys()
            .find(|k| matches!(k, ObjectKey::RootDirectory(_)))
            .cloned();
    }
    objects
        .values()
        .filter(|o| {
            matches!(o.object, ObjectKey::Node(_) | ObjectKey::Directory(_))
                && path.starts_with(&o.directory)
        })
        .max_by_key(|o| Path::new(&o.directory).components().count())
        .map(|o| o.object.clone())
}
fn parent_id(
    objects: &BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    parent: &Path,
) -> Result<Option<ObjectKey>> {
    if parent == root {
        return Ok(None);
    }
    objects
        .values()
        .find_map(|o| {
            if Path::new(&o.directory) == parent {
                match &o.object {
                    ObjectKey::Node(_) | ObjectKey::Directory(_) => Some(o.object.clone()),
                    _ => None,
                }
            } else {
                None
            }
        })
        .map(Some)
        .ok_or_else(|| {
            CoreError::new(
                "syncUnmanagedParent",
                UiMessage::error_core_sync_admit_containing_directory_before_synchronizing_children(
                ),
            )
        })
}
fn relative_to(path: &Path, directory: &Path) -> Result<String> {
    Ok(path
        .strip_prefix(directory)
        .map_err(|_| invalid(UiMessage::error_core_sync_replica_object_file_outside_owner()))?
        .to_string_lossy()
        .replace('\\', "/"))
}
fn register_directory(
    objects: &mut BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    file: &FileChange,
) -> Result<()> {
    let path = Path::new(&file.path);
    if file.before.is_some()
        || path
            .file_name()
            .is_none_or(|n| n != "weftext.directory.json")
    {
        return Ok(());
    }
    crate::directory_identity::validate_directory_marker(&file.after)?;
    let marker: serde_json::Value = serde_json::from_slice(&file.after)?;
    let id = marker["directoryId"]
        .as_str()
        .ok_or_else(|| invalid(UiMessage::error_core_sync_replica_directory_identity_missing()))?;
    let directory = path.parent().unwrap();
    let key = if directory == root {
        ObjectKey::RootDirectory(id.into())
    } else {
        ObjectKey::Directory(id.into())
    };
    if objects.contains_key(&key) {
        return Err(invalid(
            UiMessage::error_core_sync_replica_directory_identity_already_present(),
        ));
    }
    let parent = if directory == root {
        None
    } else {
        parent_id(objects, root, directory.parent().unwrap())?
    };
    objects.insert(
        key.clone(),
        LocalObject {
            object: key,
            directory: display(directory),
            commit: String::new(),
            revision: String::new(),
            state: ObjectState {
                file_revisions: BTreeMap::new(),
                parent,
                name: directory.file_name().unwrap().to_string_lossy().into(),
                retired: false,
                files: BTreeMap::new(),
            },
        },
    );
    Ok(())
}
fn register_node(
    objects: &mut BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    path: &Path,
    id: &str,
) -> Result<()> {
    let key = ObjectKey::Node(id.into());
    if objects.contains_key(&key) {
        return Err(invalid(
            UiMessage::error_core_sync_replica_node_identity_already_present(),
        ));
    }
    let directory = path.parent().unwrap();
    let parent = parent_id(objects, root, directory.parent().unwrap())?;
    objects.insert(
        key.clone(),
        LocalObject {
            object: key,
            directory: display(directory),
            commit: String::new(),
            revision: String::new(),
            state: ObjectState {
                file_revisions: BTreeMap::new(),
                parent,
                name: path.file_stem().unwrap().to_string_lossy().into(),
                retired: false,
                files: BTreeMap::new(),
            },
        },
    );
    Ok(())
}
fn patch(
    objects: &mut BTreeMap<ObjectKey, LocalObject>,
    root: &Path,
    file: &FileChange,
    known: &BTreeMap<String, &Vec<u8>>,
    blobs: &mut BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    let path = Path::new(&file.path);
    let Some(key) = owner(objects, root, path) else {
        return Ok(());
    };
    if !objects.contains_key(&key) {
        objects.insert(
            key.clone(),
            LocalObject {
                object: key.clone(),
                directory: display(root),
                commit: String::new(),
                revision: String::new(),
                state: ObjectState {
                    file_revisions: BTreeMap::new(),
                    parent: None,
                    name: path.file_name().unwrap().to_string_lossy().into(),
                    retired: false,
                    files: BTreeMap::new(),
                },
            },
        );
    }
    let object = objects.get_mut(&key).unwrap();
    let relative = relative_to(path, Path::new(&object.directory))?;
    super::relative(&relative)?;
    let previous = object
        .state
        .files
        .get(&relative)
        .map(|id| {
            known
                .get(id)
                .copied()
                .or_else(|| blobs.get(id))
                .ok_or_else(|| invalid(UiMessage::error_core_sync_replica_original_blob_missing()))
        })
        .transpose()?;
    if previous.map(Vec::as_slice) != file.before.as_deref() {
        return Err(CoreError::new(
            "syncExternalChange",
            UiMessage::error_core_sync_current_bytes_differ_from_replica_s_observed_base_preserve_and_explicitly_adopt_external_content_before_publishing(),
        ));
    }
    let blob = hash(&file.after);
    object.state.files.insert(relative, blob.clone());
    blobs.insert(blob, file.after.clone());
    Ok(())
}

#[cfg(all(test, windows))]
#[path = "replica_tests.rs"]
mod tests;
