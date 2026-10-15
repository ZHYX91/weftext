//! Original-writer integration for typed sync semantic decisions. Readers use
//! the accepted received graph; physical evidence is captured only at explicit
//! installed locators and frozen into the existing transaction before stage.
use super::apply::SyncApplication;
use super::relocate::RelocationInput;
use super::replica::{LocalObject, SyncPhysicalBinding};
use super::revalidation::{
    self, HistoricalSemanticInput, RequiredSemanticEvidence, SyncRevalidation, SyncRevalidationCut,
    SyncRevalidationReview,
};
use super::revalidation_evidence::{
    self, ObjectQualification, SuppliedEvidenceCut, SuppliedObject,
};
use super::*;
use crate::{
    AdmittedSnapshot, Core, Path, PathBuf, QualifiedQueryNode, QualifiedQueryScope,
    QualifiedResource, absolute_path, display,
    journal::Plan,
    operations::{AbsentDependency, SnapshotBinding},
};

/// Derived validation evidence retained in the original SyncApplication. These
/// records explain the actual read, never authorize a new author transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FrozenSemanticEvidence {
    pub absent_objects: BTreeSet<ObjectKey>,
    pub historical_inputs: Vec<serde_json::Value>,
    pub provider_requirements: Vec<serde_json::Value>,
    pub provider_dependencies: Vec<serde_json::Value>,
    pub discharged: Vec<serde_json::Value>,
    pub deferred_queries: Vec<serde_json::Value>,
    pub annotation_targets: Vec<serde_json::Value>,
}

fn refused(code: &str, message: &str) -> CoreError {
    CoreError::new(code, message)
}
fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}

fn query_read_members(
    snapshot: &AdmittedSnapshot,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
) -> BTreeSet<ObjectKey> {
    let mut members = snapshot
        .membership_ids
        .iter()
        .cloned()
        .map(ObjectKey::Node)
        .collect::<BTreeSet<_>>();
    members.extend(
        chosen
            .iter()
            .filter(|(object, version)| {
                matches!(object, ObjectKey::Node(_))
                    && !version.state.retired
                    && local.get(*object).is_none_or(|old| old.state.retired)
            })
            .map(|(object, _)| object.clone()),
    );
    members
}

/// This is the same finite transaction/parent closure used by ApplySync. The
/// shared application planner can use it directly instead of duplicating it.
pub(crate) fn close_revalidation_selection(
    graph: &ReplicaGraph,
    local: &BTreeMap<ObjectKey, LocalObject>,
    requested: &BTreeSet<ObjectKey>,
) -> Result<BTreeSet<ObjectKey>> {
    if requested.is_empty() || requested.len() > MAX_CHANGES {
        return Err(invalid("Select one bounded nonempty received cut"));
    }
    let current = graph.objects()?;
    let installed = graph.closure(&local.values().map(|v| v.commit.clone()).collect());
    let mut needed = requested.clone();
    loop {
        if needed.len() > MAX_CHANGES {
            return Err(refused(
                "budget",
                "The original transaction/dependency closure exceeds 5000 objects",
            ));
        }
        let previous = needed.len();
        let cut = needed
            .iter()
            .map(|object| {
                current.get(object).ok_or_else(|| {
                    refused(
                        "syncIncomplete",
                        "A selected dependency has no accepted received state",
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|view| view.versions.iter().map(|v| v.commit.clone()))
            .collect();
        for id in graph.closure(&cut).difference(&installed) {
            let manifest = graph.manifest(id).ok_or_else(|| {
                refused(
                    "syncIncomplete",
                    "An original transaction manifest is missing",
                )
            })?;
            needed.extend(manifest.changes.iter().map(|c| c.object.clone()));
            needed.extend(manifest.dependencies.iter().map(|d| d.object.clone()));
        }
        for object in needed.clone() {
            let view = current.get(&object).ok_or_else(|| {
                refused(
                    "syncIncomplete",
                    "An original dependency has no accepted received state",
                )
            })?;
            for version in &view.versions {
                needed.extend(version.state.parent.iter().cloned());
                needed.extend(
                    graph
                        .manifest(&version.commit)
                        .ok_or_else(|| {
                            refused(
                                "syncIncomplete",
                                "An original dependency manifest is missing",
                            )
                        })?
                        .dependencies
                        .iter()
                        .map(|d| d.object.clone()),
                );
            }
        }
        if needed.len() == previous {
            return Ok(needed);
        }
    }
}

impl Core {
    /// The shared Apply/Merge planner calls this after its original operation-ID
    /// retry gate, then feeds the result to its unchanged original state writer.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn prepare_sync_revalidation_input(
        &mut self,
        request: &crate::OperationRequest,
        identity: &SyncIdentity,
        expected_installed: &[Dependency],
        reviewed_versions: &[Dependency],
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
    ) -> Result<RelocationInput> {
        self.sync_local_owner()?;
        let root = absolute_path(Path::new(&request.root))?;
        let setup = self.sync_setup_for(&root)?.clone();
        if &setup.identity != identity
            || self.require_workspace(&root)?.workspace_id != identity.workspace_id
        {
            return Err(invalid("Semantic installation names another local replica"));
        }
        self.publish_sync(&root)?;
        let local = self.sync_local_state(&setup)?.0;
        let graph = self.read_sync_history(identity)?;
        let requested = resolutions
            .iter()
            .map(|resolution| resolution.object.clone())
            .collect::<BTreeSet<_>>();
        let mut seeds = requested.clone();
        seeds.extend(resolutions.iter().flat_map(|resolution| {
            resolution
                .reviewed_dependencies
                .iter()
                .map(|dependency| dependency.object.clone())
        }));
        let mut selection = close_revalidation_selection(&graph, &local, &seeds)?;
        let mut temporary = self.read_sync_history(identity)?;
        for (digest, bytes) in blobs {
            temporary.offer_blob(digest, bytes.clone())?;
        }
        let chosen = revalidation::chosen_revalidation_states(&temporary, &selection, resolutions)?;
        let preliminary = revalidation::inspect_revalidation(
            &temporary,
            &chosen,
            &local,
            resolutions,
            &BTreeSet::new(),
        )?;
        let query = if preliminary.invocations.is_empty() {
            None
        } else {
            Some(self.admitted_snapshot(&root)?)
        };
        let query_members = query
            .as_ref()
            .map(|query| query_read_members(query, &chosen, &local))
            .unwrap_or_default();
        let prepared = (|| {
            selection.extend(query_members.iter().cloned());
            selection = close_revalidation_selection(&graph, &local, &selection)?;
            revalidation::prepare_revalidation(
                graph,
                &local,
                &selection,
                resolutions,
                blobs,
                &setup.replica_id,
                &request.operation_id,
                &query_members,
                expected_installed,
                reviewed_versions,
            )
        })();
        if let Some(query) = query {
            for file in query.files.iter().chain(query.schema.iter()) {
                self.snapshots.remove(&file.base_id);
            }
            self.admitted_cuts.remove(&query.cut_id);
        }
        let prepared = prepared?;
        Ok(RelocationInput {
            request: request.clone(),
            identity: identity.clone(),
            fingerprint: hash(&serde_json::to_vec(request)?),
            local,
            states: prepared.states,
            expected_versions: reviewed_versions.to_vec(),
            selection: requested,
            publication: Some(super::replica::SyncPublication {
                root: display(&root),
                identity: identity.clone(),
                manifest: prepared.manifest,
                blobs: prepared.publication_blobs,
                locations: vec![],
            }),
            graph: prepared.graph,
        })
    }
    /// Metadata candidates only. No author bodies, Plan IDs or file revisions
    /// are allocated. Actual semantic reads are selected by the next review.
    pub fn sync_revalidation_cut(
        &self,
        root: impl AsRef<Path>,
        objects: &BTreeSet<ObjectKey>,
    ) -> Result<SyncRevalidationCut> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        self.require_workspace(&root)?;
        let local = self.sync_local_state(setup)?.0;
        let graph = self.read_sync_history(&setup.identity)?;
        close_revalidation_selection(&graph, &local, objects)?;
        // A concurrent callee may not be discoverable from the older manifest's
        // dependency list. Return bounded metadata candidates so its exact
        // branch can be explicitly selected, without eagerly reading bodies.
        revalidation::revalidation_cut(&graph, &local, &graph.objects()?.into_keys().collect())
    }

    /// Review is read-only with respect to author bytes and operation state.
    /// Ordinary temporary Core snapshot bases are released before returning.
    pub fn review_sync_revalidation(
        &mut self,
        root: impl AsRef<Path>,
        identity: &SyncIdentity,
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
    ) -> Result<SyncRevalidationReview> {
        self.sync_local_owner()?;
        let root = absolute_path(root.as_ref())?;
        let setup = self.sync_setup_for(&root)?;
        if &setup.identity != identity
            || self.require_workspace(&root)?.workspace_id != identity.workspace_id
        {
            return Err(invalid("The semantic review names another local replica"));
        }
        let local = self.sync_local_state(setup)?.0;
        if blobs.len() > MAX_FILES
            || blobs
                .values()
                .try_fold(0_usize, |n, b| n.checked_add(b.len()))
                .is_none_or(|n| n > MAX_BLOB_BYTES)
        {
            return Err(refused(
                "budget",
                "Semantic review blobs exceed the original sync budget",
            ));
        }
        let mut graph = self.read_sync_history(identity)?;
        for (digest, bytes) in blobs {
            graph.offer_blob(digest, bytes.clone())?;
        }
        let initial = resolutions.iter().map(|r| r.object.clone()).collect();
        let mut selected = close_revalidation_selection(&graph, &local, &initial)?;
        let mut query = None;
        let result = (|| {
            loop {
                let chosen =
                    revalidation::chosen_revalidation_states(&graph, &selected, resolutions)?;
                let members = query
                    .as_ref()
                    .map(|q: &AdmittedSnapshot| query_read_members(q, &chosen, &local))
                    .unwrap_or_default();
                let checked = revalidation::inspect_revalidation(
                    &graph,
                    &chosen,
                    &local,
                    resolutions,
                    &members,
                )?;
                if !checked.invocations.is_empty() && query.is_none() {
                    query = Some(self.admitted_snapshot(&root)?);
                    continue;
                }
                let mut required = selected.clone();
                required.extend(checked.states.keys().cloned());
                required.extend(members);
                let closed = close_revalidation_selection(&graph, &local, &required)?;
                if closed == selected {
                    return revalidation::review_revalidation(
                        graph,
                        &local,
                        &closed,
                        resolutions,
                        blobs,
                        &query
                            .as_ref()
                            .map(|q| query_read_members(q, &chosen, &local))
                            .unwrap_or_default(),
                    );
                }
                selected = closed;
            }
        })();
        if let Some(query) = query {
            for file in query.files.iter().chain(query.schema.iter()) {
                self.snapshots.remove(&file.base_id);
            }
            self.admitted_cuts.remove(&query.cut_id);
        }
        result
    }

    /// Called by the shared planner once its exact original Plan exists and
    /// before stage. It does not stage/execute, allocate identities or invent a
    /// second writer. Existing evidence is compared before any new read joins.
    pub(crate) fn freeze_sync_revalidation_plan(
        &mut self,
        plan: &mut Plan,
        resolutions: &[SyncRevalidation],
    ) -> Result<FrozenSemanticEvidence> {
        self.sync_local_owner()?;
        let application = plan
            .sync_application
            .as_ref()
            .ok_or_else(|| invalid("A semantic decision requires its original SyncApplication"))?
            .clone();
        let root = PathBuf::from(&application.root);
        let setup = self.sync_setup_for(&root)?;
        let local = self.sync_local_state(setup)?.0;
        if setup.identity != application.identity
            || application
                .before
                .iter()
                .any(|old| local.get(&old.object) != Some(old))
        {
            return Err(refused(
                "syncBasisChanged",
                "The original installed cut changed during semantic preparation",
            ));
        }
        let mut graph = self.read_sync_history(&application.identity)?;
        for (digest, bytes) in &application.blobs {
            graph.offer_blob(digest, bytes.clone())?;
        }
        if let Some(publication) = &plan.sync_publication {
            for (digest, bytes) in &publication.blobs {
                graph.offer_blob(digest, bytes.clone())?;
            }
        }
        let selected = application
            .after
            .iter()
            .map(|after| after.object.clone())
            .collect::<BTreeSet<_>>();
        let chosen = revalidation::chosen_revalidation_states(&graph, &selected, resolutions)?;
        if application.after.iter().any(|after| {
            chosen
                .get(&after.object)
                .is_none_or(|v| v.state != after.state)
        }) {
            return Err(invalid(
                "The original writer after states differ from the semantic proposal",
            ));
        }
        let preliminary = revalidation::inspect_revalidation(
            &graph,
            &chosen,
            &local,
            resolutions,
            &BTreeSet::new(),
        )?;
        let query = if preliminary.invocations.is_empty() {
            None
        } else {
            Some(self.admitted_snapshot(&root)?)
        };
        let query_members = query
            .as_ref()
            .map(|q| query_read_members(q, &chosen, &local))
            .unwrap_or_default();
        let result = self.capture_sync_revalidation_plan(
            plan,
            &application,
            &graph,
            &local,
            &chosen,
            resolutions,
            query.as_ref(),
            &query_members,
        );
        if let Some(query) = query {
            for file in query.files.iter().chain(query.schema.iter()) {
                self.snapshots.remove(&file.base_id);
            }
            self.admitted_cuts.remove(&query.cut_id);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_sync_revalidation_plan(
        &mut self,
        plan: &mut Plan,
        application: &SyncApplication,
        graph: &ReplicaGraph,
        local: &BTreeMap<ObjectKey, LocalObject>,
        chosen: &BTreeMap<ObjectKey, Version>,
        resolutions: &[SyncRevalidation],
        query: Option<&AdmittedSnapshot>,
        query_members: &BTreeSet<ObjectKey>,
    ) -> Result<FrozenSemanticEvidence> {
        let root = Path::new(&application.root);
        let checked = revalidation::validate_revalidation_with_query_members(
            graph,
            chosen,
            local,
            resolutions,
            query_members,
        )?;
        let installed = chosen
            .keys()
            .filter(|object| local.contains_key(*object))
            .cloned()
            .collect::<BTreeSet<_>>();
        let stamps = self.sync_installed_stamps(local, &installed, None)?;
        let physical = installed
            .iter()
            .flat_map(|object| {
                local[object]
                    .state
                    .file_revisions
                    .iter()
                    .map(|(relative, revision)| SyncPhysicalBinding {
                        object: object.clone(),
                        relative_path: relative.clone(),
                        stamp: stamps[revision].clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut paths = installed
            .iter()
            .flat_map(|object| {
                local[object]
                    .state
                    .files
                    .keys()
                    .map(|relative| display(&Path::new(&local[object].directory).join(relative)))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for name in [
            ".weftext-workspace.json",
            ".weftext-format",
            ".weftext-mvp-commit.json",
            ".weftext-schema.json",
            ".weftext-rules",
        ] {
            let path = display(&root.join(name));
            if !paths
                .iter()
                .any(|original| same_path(Path::new(original), Path::new(&path)))
            {
                paths.push(path);
            }
        }
        let snapshot = self.read_bounded_snapshot(root, &paths, MAX_FILES)?;
        let mut witnesses = snapshot
            .files
            .into_iter()
            .map(|file| self.take_snapshot_witness(&file.base_id))
            .collect::<Result<Vec<_>>>()?;
        if let Some(query) = query {
            for file in query.files.iter().chain(query.schema.iter()) {
                Core::merge_feature_witnesses(
                    &mut witnesses,
                    vec![self.take_snapshot_witness(&file.base_id)?],
                )?;
            }
        }
        let mut nodes = BTreeMap::<ObjectKey, QualifiedQueryNode>::new();
        let mut directories = BTreeMap::<ObjectKey, QualifiedQueryScope>::new();
        let mut locators = BTreeMap::new();
        for object in &installed {
            let local = &local[object];
            let lifecycle = if local.state.retired { "trash" } else { "live" };
            let directory = Path::new(&local.directory);
            match object {
                ObjectKey::Node(id) => {
                    let path = directory.join(format!("{}.typ", local.state.name));
                    nodes.insert(
                        object.clone(),
                        self.qualified_query_node_at(
                            root,
                            &path,
                            &weftext_knowledge::NodeRef {
                                workspace_id: graph.workspace_id.clone(),
                                node_id: id.clone(),
                            },
                            lifecycle,
                        )?,
                    );
                    locators.insert(object.clone(), path);
                }
                ObjectKey::Directory(id) | ObjectKey::RootDirectory(id) => {
                    directories.insert(
                        object.clone(),
                        self.qualified_query_scope_at(
                            root,
                            directory,
                            &format!("weftext-directory://{}/{id}", graph.workspace_id),
                            lifecycle,
                        )?,
                    );
                    locators.insert(object.clone(), directory.join("weftext.directory.json"));
                }
                _ => {
                    locators.insert(
                        object.clone(),
                        directory.join(local.state.files.keys().next().unwrap()),
                    );
                }
            }
        }
        let objects = installed
            .iter()
            .map(|object| {
                (
                    object.clone(),
                    SuppliedObject {
                        path: &locators[object],
                        qualification: if let Some(node) = nodes.get(object) {
                            ObjectQualification::Node(node)
                        } else if let Some(directory) = directories.get(object) {
                            ObjectQualification::Directory(directory)
                        } else {
                            ObjectQualification::Configuration
                        },
                    },
                )
            })
            .collect();
        let mut resources = Vec::<QualifiedResource>::new();
        for obligation in checked.evidence_obligations() {
            let RequiredSemanticEvidence::Resource {
                owner, reference, ..
            } = obligation
            else {
                continue;
            };
            let object = ObjectKey::Node(owner.node_id.clone());
            let Some(old) = local.get(&object) else {
                continue;
            };
            let id = reference.rsplit('/').next().unwrap();
            let old_resources = super::apply::validate_state(graph, &object, &old.state)?;
            if let Some((relative, _)) = old_resources
                .iter()
                .find(|(_, resource)| resource.as_str() == id)
            {
                resources.push(self.qualified_resource_at(
                    root,
                    &locators[&object],
                    owner,
                    reference,
                    &Path::new(&old.directory).join(relative),
                    if old.state.retired { "trash" } else { "live" },
                    None,
                )?);
            }
        }
        let resource_refs = resources.iter().collect::<Vec<_>>();
        let after_locations = application
            .after
            .iter()
            .map(|object| (object.object.clone(), PathBuf::from(&object.directory)))
            .collect();
        let query_refs = query.into_iter().collect::<Vec<_>>();
        let cut = SuppliedEvidenceCut {
            root,
            graph,
            installed: local,
            chosen,
            physical: &physical,
            snapshots: &witnesses,
            objects: &objects,
            resources: &resource_refs,
            query_snapshots: &query_refs,
            after_locations: &after_locations,
            planned: Some(plan),
        };
        let capture = revalidation_evidence::capture_evidence(self, &cut, &checked)?;
        if !capture.unfulfilled().is_empty() {
            let gaps = capture
                .unfulfilled()
                .iter()
                .map(|item| serde_json::json!({"obligation": item.obligation, "gap": item.gap}))
                .collect::<Vec<_>>();
            return Err(refused(
                "syncEvidenceRequired",
                &format!(
                    "Exact original-writer evidence remains required: {}",
                    serde_json::to_string(&gaps)?
                ),
            ));
        }
        let reads = capture.into_read_set()?;
        if reads
            .read_objects
            .iter()
            .any(|object| !chosen.contains_key(object))
        {
            return Err(refused(
                "syncDependencyRequired",
                "Actual semantic evidence escaped the exact reviewed cut",
            ));
        }
        let transaction = plan
            .transaction
            .as_mut()
            .ok_or_else(|| invalid("Semantic evidence requires the original transaction"))?;
        for witness in reads.witnesses {
            let SnapshotBinding {
                path,
                captured,
                parent,
            } = witness.binding;
            if let Some(file) = captured {
                if let Some(existing) = transaction
                    .files
                    .iter()
                    .find(|old| same_path(Path::new(&old.path), Path::new(&file.path)))
                {
                    if existing.stamp != file.stamp || existing.bytes != file.bytes {
                        return Err(refused(
                            "syncEvidenceChanged",
                            "Original writer capture changed before semantic freezing",
                        ));
                    }
                } else {
                    transaction.files.push(file);
                    transaction.write_parents.push((path, parent));
                }
            } else if !transaction
                .absent_dependencies
                .iter()
                .any(|old| same_path(Path::new(&old.path), Path::new(&path)))
            {
                transaction
                    .absent_dependencies
                    .push(AbsentDependency { path, parent });
            }
        }
        if let Some(members) = reads.query_membership {
            if transaction
                .expected_membership
                .as_ref()
                .is_some_and(|old| old != &members)
            {
                return Err(refused(
                    "scopeChanged",
                    "The original Query membership differs from the writer cut",
                ));
            }
            transaction.expected_membership = Some(members);
        }
        let outputs = revalidation_evidence::original_outputs(transaction);
        if transaction.files.len() + outputs.iter().filter(|file| file.before.is_none()).count()
            > MAX_FILES
            || transaction
                .files
                .iter()
                .map(|file| file.bytes.len())
                .sum::<usize>()
                .saturating_add(outputs.iter().map(|file| file.after.len()).sum::<usize>())
                > MAX_BLOB_BYTES
        {
            return Err(refused(
                "budget",
                "Frozen semantic reads exceed the original writer transaction budget",
            ));
        }
        for target in &reads.annotation_targets {
            if target.state != "exact" {
                let message = format!(
                    "Annotation {} is preserved unchanged; target unavailable, awaiting explicit reconnection ({})",
                    target.annotation_id, target.reason
                );
                transaction.preview.losses.push(weftext_knowledge::Loss {
                    path: format!(
                        "weftext-node://{}/{}",
                        target.owner.workspace_id, target.owner.node_id
                    ),
                    code: "annotationTargetUnavailable".into(),
                    detail: message,
                });
            }
        }
        // A committed Definition discharge may name a pinned historical
        // source. Retain its exact accepted bytes in this same application;
        // later admission must not fall back to a current equal-looking file.
        let frozen_application = plan
            .sync_application
            .as_mut()
            .ok_or_else(|| invalid("Semantic evidence requires its original application"))?;
        for input in &checked.historical_inputs {
            let bytes = graph.blob(&input.digest).ok_or_else(|| {
                refused(
                    "syncIncomplete",
                    "Exact historical semantic bytes are missing",
                )
            })?;
            if frozen_application
                .blobs
                .get(&input.digest)
                .is_some_and(|old| old.as_slice() != bytes)
            {
                return Err(invalid(
                    "Historical semantic blob changes its accepted digest",
                ));
            }
            frozen_application
                .blobs
                .insert(input.digest.clone(), bytes.to_vec());
        }
        if frozen_application.blobs.len() > MAX_FILES
            || frozen_application
                .blobs
                .values()
                .try_fold(0usize, |sum, bytes| sum.checked_add(bytes.len()))
                .is_none_or(|sum| sum > MAX_BLOB_BYTES)
        {
            return Err(refused(
                "budget",
                "Retained semantic inputs exceed the original application budget",
            ));
        }
        let absent_objects = checked
            .required_evidence
            .iter()
            .filter_map(|evidence| match evidence {
                RequiredSemanticEvidence::AbsentConfiguration { object } => Some(object.clone()),
                _ => None,
            })
            .collect();
        Ok(FrozenSemanticEvidence {
            absent_objects,
            historical_inputs: checked
                .historical_inputs
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
            provider_requirements: checked
                .provider_requirements
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
            provider_dependencies: checked
                .provider_dependencies
                .iter()
                .chain(&reads.provider_dependencies)
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
            discharged: reads
                .discharged
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
            deferred_queries: checked
                .deferred_queries
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
            annotation_targets: reads
                .annotation_targets
                .iter()
                .map(serde_json::to_value)
                .collect::<std::result::Result<_, _>>()?,
        })
    }

    /// The shared execution/recovery cut gate invokes this before its first
    /// effect. Negative received dependencies cannot be protected by disk-only
    /// absent guards when a new remote configuration has arrived meanwhile.
    pub(crate) fn validate_sync_semantic_evidence(
        &self,
        operation_id: &str,
        application: &SyncApplication,
        evidence: &FrozenSemanticEvidence,
    ) -> Result<()> {
        for object in &evidence.absent_objects {
            let name = match object {
                ObjectKey::Schema => ".weftext-schema.json",
                ObjectKey::Rules => ".weftext-rules",
                _ => return Err(invalid("Unsupported absent semantic configuration")),
            };
            self.check_pending_except(
                &Path::new(&application.root).join(name),
                Some(operation_id),
            )?;
        }
        let graph = self.read_sync_history(&application.identity)?;
        let current = graph.objects()?;
        if evidence.absent_objects.iter().any(|object| {
            current.contains_key(object)
                || graph.manifests.iter().any(|(id, manifest)| {
                    manifest
                        .changes
                        .iter()
                        .any(|change| &change.object == object)
                        && matches!(graph.status(id), Ok(Admission::Staged { .. }))
                })
        }) {
            return Err(refused(
                "syncConflictChanged",
                "An explicitly absent semantic configuration arrived after review",
            ));
        }
        for value in &evidence.historical_inputs {
            let descriptor: HistoricalSemanticInput = serde_json::from_value(value.clone())?;
            if !graph.admitted.iter().any(|id| {
                graph.manifests[id].changes.iter().any(|change| {
                    change.object == descriptor.object
                        && change.revision == descriptor.object_revision
                        && change.after.file_revisions.get(&descriptor.relative_path)
                            == Some(&descriptor.file_revision)
                        && change.after.files.get(&descriptor.relative_path)
                            == Some(&descriptor.digest)
                })
            }) || graph.blob(&descriptor.digest).is_none()
            {
                return Err(refused(
                    "syncIncomplete",
                    "An exact retained semantic input is no longer available",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(all(test, windows))]
#[path = "revalidation_writer_tests.rs"]
mod tests;
