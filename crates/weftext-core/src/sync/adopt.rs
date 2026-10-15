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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAdoptionCut {
    pub identity: SyncIdentity,
    pub expected_installed: Vec<Dependency>,
    pub changes: Vec<FileChange>,
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
                "Select a finite nonempty installed object cut",
            ));
        }
        let root = absolute_path(root)?;
        self.require_workspace(&root)?;
        let setup = self.sync_setup_for(&root)?;
        let identity = setup.identity.clone();
        let local = self.sync_local_state(setup)?.0;
        let mut expected_installed = Vec::new();
        let mut paths = Vec::new();
        let mut originals = Vec::new();
        for object in objects {
            let original = local.get(object).ok_or_else(|| {
                CoreError::new(
                    "syncBasisChanged",
                    "Selected object is not installed in this replica",
                )
            })?;
            expected_installed.push(Dependency {
                object: object.clone(),
                revisions: BTreeSet::from([original.revision.clone()]),
            });
            for relative in original.state.files.keys() {
                if paths.len() == MAX_FILES {
                    return Err(CoreError::new("budget", "Adoption cut exceeds 5000 files"));
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
            let mut total = 0usize;
            for (file, (object, relative)) in snapshot.files.into_iter().zip(originals) {
                let original = &local[&object];
                let captured = self.snapshots[&file.base_id].captured.as_ref().ok_or_else(|| CoreError::new("syncExternalChange", "An installed member is absent; preserve it for an explicit structure decision"))?;
                total += captured.bytes.len();
                if total > MAX_BLOB_BYTES {
                    return Err(CoreError::new("budget", "Adoption cut exceeds 64 MiB"));
                }
                let stamp = &physical[&original.state.file_revisions[&relative]];
                if captured.stamp.identity != stamp.identity {
                    return Err(CoreError::new(
                        "identityConflict",
                        "The selected installed leaf was physically replaced",
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
            if changes.is_empty() {
                return Err(CoreError::new(
                    "noChange",
                    "The selected installed objects have no observed external change",
                ));
            }
            Ok(SyncAdoptionCut {
                identity,
                expected_installed,
                changes,
            })
        })();
        let retained: BTreeSet<_> = result
            .as_ref()
            .map(|cut| {
                cut.changes
                    .iter()
                    .filter_map(|c| c.base_id.as_deref())
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
                    "The original operation names different observed content",
                ));
            }
            return self.operation_details(&request.operation_id);
        }
        let OperationIntent::AdoptSyncExternal {
            identity,
            expected_installed,
            changes,
        } = &request.intent
        else {
            return Err(invalid("Expected explicit external adoption intent"));
        };
        if changes.is_empty()
            || changes.len() > MAX_FILES
            || expected_installed.is_empty()
            || expected_installed.len() > MAX_CHANGES
            || changes.iter().map(|c| c.after.len()).sum::<usize>() > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                "Adoption requires a finite nonempty observed cut",
            ));
        }
        if changes
            .iter()
            .any(|c| c.before.as_ref() != Some(&c.after) || c.base_id.is_none())
        {
            return Err(CoreError::new(
                "basisRequired",
                "Adopt only the exact current bytes with their original observed snapshot basis",
            ));
        }
        let root = absolute_path(Path::new(&request.root))?;
        let workspace = self.require_workspace(&root)?;
        let setup = self.sync_setup_for(&root)?;
        if &setup.identity != identity || workspace.workspace_id != identity.workspace_id {
            return Err(invalid(
                "Selected library belongs to another replica family",
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
                    "A selected installed object is unavailable",
                )
            })?;
            if dependency.revisions != BTreeSet::from([installed.revision.clone()])
                || before
                    .insert(dependency.object.clone(), installed.clone())
                    .is_some()
            {
                return Err(CoreError::new(
                    "syncBasisChanged",
                    "Review each exact installed object version once before adopting",
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
                    "Only a leaf already belonging to a selected installed object can be adopted",
                )
            })?;
            if requested.insert(display(&selected), input).is_some() {
                return Err(invalid("One observed leaf has multiple adoption proposals"));
            }
        }
        let mut files = Vec::<CapturedFile>::new();
        let selected = before.keys().cloned().collect();
        let physical = self.sync_installed_stamps(&before, &selected, None)?;
        let mut output = Vec::new();
        let mut after = before.clone();
        let mut blobs = BTreeMap::new();
        let mut changed = BTreeSet::new();
        let mut total = 0usize;
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
                    "Include the installed placement parent in the reviewed adoption cut",
                ));
            }
            for (relative, digest) in &original.state.files {
                let path = Path::new(&original.directory).join(relative);
                self.check_pending(&path)?;
                let captured = capture_optional(&path)?.ok_or_else(|| CoreError::new("syncExternalChange", "An installed object member is missing; preserve it for an explicit structure decision"))?;
                total += captured.bytes.len();
                if total > MAX_BLOB_BYTES || files.len() == MAX_FILES {
                    return Err(CoreError::new(
                        "budget",
                        "The complete adoption input cut exceeds 64 MiB/5000 files",
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
                            "Observed adoption bytes changed after review",
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
                        "An unselected object member differs from its installed version",
                    ));
                }
                if physical[&original.state.file_revisions[relative]].identity
                    != captured.stamp.identity
                {
                    return Err(CoreError::new(
                        "identityConflict",
                        "Installed leaf no longer has its retained original physical binding",
                    ));
                }
                files.push(captured);
            }
        }
        let mut graph = self.read_sync_history(identity)?;
        for (id, bytes) in known {
            graph.offer_blob(&id, bytes.clone())?;
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
                    ) {
                        return Err(CoreError::new(
                            "identityConflict",
                            "An adopted resource is not its original owner-bound physical object",
                        ));
                    }
                }
                preview.node_map.insert(id.clone(), id.clone());
            }
            if let ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) = object {
                let parent = ParentGuard::acquire(
                    &Path::new(&installed.directory).join("weftext.directory.json"),
                )?
                .directory_identity()?;
                if !self.directory_admitted(&identity.workspace_id, id, &parent)? {
                    return Err(CoreError::new(
                        "identityConflict",
                        "The adopted directory lacks its original physical admission",
                    ));
                }
            }
        }
        let mut absent_dependencies = Vec::new();
        self.validate_adoption_typed_cut(
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
            let captured = capture_optional(&path)?
                .ok_or_else(|| invalid("Original workspace control is unavailable"))?;
            total += captured.bytes.len();
            if total > MAX_BLOB_BYTES || files.len() == MAX_FILES {
                return Err(CoreError::new(
                    "budget",
                    "Adoption proof exceeds the complete input budget",
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
                ParentGuard::acquire(Path::new(&file.path))?.directory_identity()?,
            ));
        }
        let receipt = OperationReceipt {
            operation_id: request.operation_id.clone(), plan_id: preview.plan_id.clone(), status: "committed".into(), path: display(&root),
            node_ids: preview.node_map.keys().cloned().collect(), message: "The exact observed external content was explicitly adopted as a new portable version".into(),
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
            absent_dependencies,
            write_parents,
        });
        self.stage(plan)?;
        Ok(preview)
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
        for plan in self
            .operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
        {
            let commit: serde_json::Value = serde_json::from_slice(&std::fs::read(
                self.journal_path(&plan.operation_id, "commit"),
            )?)?;
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
                        "Installed revision has ambiguous physical evidence",
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
        for setup in self.sync_setups() {
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
                            "Initial replica physical evidence is ambiguous",
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
                "This legacy installed leaf has no retained original physical evidence; current paths cannot reconstruct it",
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
    ) -> Result<()> {
        let workspace = &self.sync_setup_for(root)?.identity.workspace_id;
        if !changed.iter().any(|k| matches!(k, ObjectKey::Node(_))) {
            return Ok(());
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
                    "Include the installed Schema in the reviewed adoption cut",
                ));
            }
            self.adoption_absent_config(root, &schema_path, absent_dependencies)?;
            weftext_data::Schema::bundled()
        }
        .map_err(|e| CoreError::new("schemaUnavailable", e.to_string()))?;
        if self
            .sync_local_state(self.sync_setup_for(root)?)?
            .0
            .contains_key(&ObjectKey::Rules)
            && !before.contains_key(&ObjectKey::Rules)
        {
            return Err(CoreError::new(
                "syncDependencyRequired",
                "Include the installed Rules in the reviewed adoption cut",
            ));
        }
        if !before.contains_key(&ObjectKey::Rules) {
            let rules = root.join(".weftext-rules");
            if self.query_rules_ever_committed(root)? {
                return Err(CoreError::new(
                    "rulesUnavailable",
                    "Previously committed Rules are unavailable",
                ));
            }
            self.adoption_absent_config(root, &rules, absent_dependencies)?;
        }
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
                .map_err(|e| CoreError::new("syncSemanticDecisionRequired", e.to_string()))?;
            let saved = weftext_knowledge::saved_query_reference_slots(&source, &schema, &node)
                .map_err(|e| CoreError::new("syncSemanticDecisionRequired", e.to_string()))?;
            let definitions = weftext_knowledge::definition_slots(&source, &node)
                .map_err(|e| CoreError::new("syncSemanticDecisionRequired", e.to_string()))?;
            if !citations.is_empty()
                || !saved.typed_refs.is_empty()
                || !saved.definition_references.is_empty()
                || !definitions.is_empty()
                || !weftext_data::embedded_queries(&source).is_empty()
            {
                return Err(CoreError::new(
                    "syncSemanticDecisionRequired",
                    "This observed source needs an explicit citation/Query semantic revalidation decision before adoption",
                ));
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
                    "The observed author declaration is not valid under the reviewed Schema",
                ));
            }
            let refs = weftext_data::typed_ref_closure(&source, &schema)
                .map_err(|e| CoreError::new("invalidTypedSource", e.to_string()))?;
            if !refs.complete {
                return Err(CoreError::new(
                    "invalidTypedSource",
                    "The observed typed reference closure is incomplete",
                ));
            }
            for slot in refs.slots {
                let (kind, target_workspace, id) =
                    if let Some(rest) = slot.target.strip_prefix("weftext-resource://") {
                        let (workspace, id) = rest.split_once('/').ok_or_else(|| {
                            invalid("Resource reference lacks its exact identity")
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
                        "A typed reference requires an explicit qualified dependency in this workspace",
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
                        "Include every actual typed target in the exact reviewed adoption cut",
                    ));
                }
            }
        }
        Ok(())
    }

    fn adoption_absent_config(
        &self,
        root: &Path,
        path: &Path,
        absent_dependencies: &mut Vec<AbsentDependency>,
    ) -> Result<()> {
        if capture_optional(path)?.is_some() {
            return Err(CoreError::new(
                "syncDependencyRequired",
                "An actual configuration outside the installed cut must be explicitly handled before adopting this source",
            ));
        }
        self.require_workspace(root)?;
        absent_dependencies.push(AbsentDependency {
            path: display(path),
            parent: ParentGuard::acquire(path)?.directory_identity()?,
        });
        Ok(())
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}

#[cfg(all(test, windows))]
#[path = "adopt_tests.rs"]
mod tests;
