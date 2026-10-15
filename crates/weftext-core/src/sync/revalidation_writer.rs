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
    #[serde(default)]
    pub native_products: Vec<serde_json::Value>,
}

pub(crate) struct CapturedSemanticValidation {
    pub capture: revalidation_evidence::EvidenceCapture,
    pub checked: revalidation::CheckedRevalidation,
    historical_blobs: BTreeMap<String, Vec<u8>>,
}

fn refused(code: &str, message: &str) -> CoreError {
    CoreError::new(code, message)
}
fn same_path(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|p| p.as_os_str().is_empty())
}

pub(crate) fn query_read_members(
    root: &Path,
    snapshots: &[AdmittedSnapshot],
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
) -> Result<BTreeSet<ObjectKey>> {
    let mut members = snapshots
        .iter()
        .flat_map(|snapshot| &snapshot.membership_ids)
        .cloned()
        .map(ObjectKey::Node)
        .collect::<BTreeSet<_>>();
    for object in chosen
        .keys()
        .filter(|object| matches!(object, ObjectKey::Node(_)))
    {
        let retired = revalidation::effective_retired(object, chosen)?;
        let changed = match local.get(object) {
            Some(old) => local_retired(root, old)? != retired,
            None => true,
        };
        if changed
            && snapshots
                .iter()
                .any(|snapshot| (snapshot.inventory.status == "trashReadOnly") == retired)
        {
            members.insert(object.clone());
        }
    }
    Ok(members)
}

pub(crate) fn local_retired(root: &Path, local: &LocalObject) -> Result<bool> {
    Ok(crate::backend::namespace_relative_path(
        Path::new(&local.directory),
        &root.join(".weftext-trash"),
    )?
    .is_some())
}

fn query_lifecycles(checked: &revalidation::CheckedRevalidation) -> Result<BTreeSet<String>> {
    checked
        .invocations
        .iter()
        .try_fold(BTreeSet::new(), |mut lifecycles, invocation| {
            lifecycles.extend(
                weftext_data::required_query_lifecycles(
                    &invocation.plan,
                    &invocation.context.definitions,
                )
                .map_err(|_| {
                    refused(
                        "queryScopeUnavailable",
                        "Actual Query lifecycle closure is unavailable",
                    )
                })?,
            );
            Ok(lifecycles)
        })
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
    pub(crate) fn capture_sync_query_lifecycles(
        &mut self,
        root: &Path,
        checked: &revalidation::CheckedRevalidation,
        snapshots: &mut Vec<AdmittedSnapshot>,
    ) -> Result<bool> {
        let mut added = false;
        for lifecycle in query_lifecycles(checked)? {
            if snapshots.iter().any(|snapshot| {
                (snapshot.inventory.status == "trashReadOnly") == (lifecycle == "trash")
            }) {
                continue;
            }
            let snapshot = if lifecycle == "trash" {
                self.trash_admitted_snapshot(root)?
            } else {
                self.admitted_snapshot(root)?
            };
            snapshots.push(snapshot);
            added = true;
        }
        Ok(added)
    }

    pub(crate) fn release_sync_query_lifecycles(&mut self, snapshots: &[AdmittedSnapshot]) {
        for query in snapshots {
            for file in query.files.iter().chain(query.schema.iter()) {
                self.snapshots.remove(&file.base_id);
            }
            self.admitted_cuts.remove(&query.cut_id);
        }
    }

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
        let mut query = Vec::new();
        let prepared = (|| {
            self.capture_sync_query_lifecycles(&root, &preliminary, &mut query)?;
            let query_members = query_read_members(&root, &query, &chosen, &local)?;
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
        self.release_sync_query_lifecycles(&query);
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
        self.review_sync_revalidation_seeded(root, identity, resolutions, blobs, &BTreeSet::new())
    }

    pub(crate) fn review_sync_revalidation_seeded(
        &mut self,
        root: impl AsRef<Path>,
        identity: &SyncIdentity,
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
        native_candidates: &BTreeSet<ObjectKey>,
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
        let initial = resolutions
            .iter()
            .map(|resolution| resolution.object.clone())
            .chain(native_candidates.iter().cloned())
            .collect();
        let mut selected = close_revalidation_selection(&graph, &local, &initial)?;
        let mut query = Vec::new();
        let result = (|| {
            loop {
                let chosen =
                    revalidation::chosen_revalidation_states(&graph, &selected, resolutions)?;
                let members = query_read_members(&root, &query, &chosen, &local)?;
                let checked = revalidation::inspect_revalidation(
                    &graph,
                    &chosen,
                    &local,
                    resolutions,
                    &members,
                )?;
                if self.capture_sync_query_lifecycles(&root, &checked, &mut query)? {
                    continue;
                }
                let mut required = selected.clone();
                required.extend(checked.states.keys().cloned());
                required.extend(members);
                if checked.invocations.iter().any(|invocation| {
                    weftext_data::query_products(&invocation.plan, &invocation.context.definitions)
                        .is_ok_and(|products| !products.is_empty())
                }) && graph.objects()?.contains_key(&ObjectKey::Settings)
                {
                    required.insert(ObjectKey::Settings);
                }
                let closed = close_revalidation_selection(&graph, &local, &required)?;
                if closed == selected {
                    return revalidation::review_revalidation(
                        graph,
                        &local,
                        &closed,
                        resolutions,
                        blobs,
                        &query_read_members(&root, &query, &chosen, &local)?,
                    );
                }
                selected = closed;
            }
        })();
        self.release_sync_query_lifecycles(&query);
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
        let captured = self.capture_sync_revalidation_validation(plan, resolutions, false)?;
        self.finish_sync_semantic_validation(plan, captured)
    }

    pub(crate) fn capture_sync_revalidation_validation(
        &mut self,
        plan: &mut Plan,
        resolutions: &[SyncRevalidation],
        collect_native: bool,
    ) -> Result<CapturedSemanticValidation> {
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
        let mut query = Vec::new();
        let result = (|| {
            self.capture_sync_query_lifecycles(&root, &preliminary, &mut query)?;
            let query_members = query_read_members(&root, &query, &chosen, &local)?;
            self.capture_sync_revalidation_plan_mode(
                plan,
                &application,
                &graph,
                &local,
                &chosen,
                resolutions,
                &query,
                &query_members,
                collect_native,
            )
        })();
        self.release_sync_query_lifecycles(&query);
        result
    }

    /// The original no-write external-adoption Plan keeps its installed cut.
    /// It does not select received heads or grant a new native definition key.
    pub(crate) fn freeze_sync_adoption_plan(
        &mut self,
        plan: &mut Plan,
    ) -> Result<FrozenSemanticEvidence> {
        let captured = self.capture_sync_adoption_validation(plan, false)?;
        self.finish_sync_semantic_validation(plan, captured)
    }

    pub(crate) fn capture_sync_adoption_validation(
        &mut self,
        plan: &mut Plan,
        collect_native: bool,
    ) -> Result<CapturedSemanticValidation> {
        self.sync_local_owner()?;
        let application = plan
            .sync_application
            .as_ref()
            .ok_or_else(|| invalid("Adoption requires its original application"))?
            .clone();
        let transaction = plan
            .transaction
            .as_ref()
            .ok_or_else(|| invalid("Adoption requires its original no-write transaction"))?;
        if !matches!(
            transaction.request.intent,
            crate::OperationIntent::AdoptSyncExternal { .. }
        ) || transaction.request.operation_id != plan.operation_id
            || transaction.request.root != application.root
            || transaction.preview.operation_id != plan.operation_id
        {
            return Err(invalid(
                "Semantic adoption requires its exact original request",
            ));
        }
        let crate::operations::Mutation::Update { files: outputs } = &transaction.mutation else {
            return Err(invalid("Adoption must retain the original no-write Update"));
        };
        if outputs
            .iter()
            .any(|file| file.before.as_ref() != Some(&file.after))
        {
            return Err(invalid(
                "External adoption cannot rewrite the observed author bytes",
            ));
        }
        let root = PathBuf::from(&application.root);
        let setup = self.sync_setup_for(&root)?;
        let local = self.sync_local_state(setup)?.0;
        if setup.identity != application.identity
            || application
                .before
                .iter()
                .any(|before| local.get(&before.object) != Some(before))
        {
            return Err(refused(
                "syncBasisChanged",
                "The exact reviewed installed adoption basis changed",
            ));
        }
        let publication = plan
            .sync_publication
            .as_ref()
            .ok_or_else(|| invalid("Adoption requires its original publication"))?;
        if publication.identity != application.identity
            || publication.manifest.operation_id != plan.operation_id
            || publication.manifest.changes.len() != application.after.len()
        {
            return Err(invalid(
                "Adoption publication differs from the original application",
            ));
        }
        let mut chosen = application
            .before
            .iter()
            .map(|before| {
                (
                    before.object.clone(),
                    Version {
                        commit: before.commit.clone(),
                        revision: before.revision.clone(),
                        state: before.state.clone(),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut changed = BTreeSet::new();
        for after in &application.after {
            let before = local
                .get(&after.object)
                .ok_or_else(|| invalid("Adoption cannot create a new owner"))?;
            if !chosen.contains_key(&after.object)
                || !changed.insert(after.object.clone())
                || after.directory != before.directory
                || after.state.parent != before.state.parent
                || after.state.name != before.state.name
                || after.state.retired != before.state.retired
                || after
                    .state
                    .files
                    .keys()
                    .any(|file| !before.state.files.contains_key(file))
                || before
                    .state
                    .files
                    .keys()
                    .filter(|file| !after.state.files.contains_key(*file))
                    .any(|file| {
                        !super::adopt::adopted_removed_member(
                            plan,
                            &Path::new(&before.directory).join(file),
                        )
                    })
                || !publication.manifest.changes.iter().any(|change| {
                    change.object == after.object
                        && change.revision == after.revision
                        && change.before == BTreeSet::from([before.revision.clone()])
                        && change.after == after.state
                })
            {
                return Err(invalid(
                    "Adoption changes the original owner, placement or publication cut",
                ));
            }
            chosen.get_mut(&after.object).unwrap().state = after.state.clone();
        }
        let mut graph = self.read_sync_history(&application.identity)?;
        for (digest, bytes) in &application.blobs {
            graph.offer_blob(digest, bytes.clone())?;
        }
        let preliminary = revalidation::inspect_installed_adoption(
            &graph,
            &chosen,
            &local,
            &changed,
            &BTreeSet::new(),
        )?;
        let mut query = Vec::new();
        let result = (|| {
            self.capture_sync_query_lifecycles(&root, &preliminary, &mut query)?;
            let members = query_read_members(&root, &query, &chosen, &local)?;
            let checked = revalidation::inspect_installed_adoption(
                &graph, &chosen, &local, &changed, &members,
            )?;
            self.capture_sync_checked_plan_mode(
                plan,
                &application,
                &graph,
                &local,
                &chosen,
                &checked,
                &query,
                collect_native,
            )
        })();
        self.release_sync_query_lifecycles(&query);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_sync_revalidation_plan_mode(
        &mut self,
        plan: &mut Plan,
        application: &SyncApplication,
        graph: &ReplicaGraph,
        local: &BTreeMap<ObjectKey, LocalObject>,
        chosen: &BTreeMap<ObjectKey, Version>,
        resolutions: &[SyncRevalidation],
        query: &[AdmittedSnapshot],
        query_members: &BTreeSet<ObjectKey>,
        collect_native: bool,
    ) -> Result<CapturedSemanticValidation> {
        let checked = revalidation::validate_revalidation_with_query_members(
            graph,
            chosen,
            local,
            resolutions,
            query_members,
        )?;
        self.capture_sync_checked_plan_mode(
            plan,
            application,
            graph,
            local,
            chosen,
            &checked,
            query,
            collect_native,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_sync_checked_plan_mode(
        &mut self,
        plan: &mut Plan,
        application: &SyncApplication,
        graph: &ReplicaGraph,
        local: &BTreeMap<ObjectKey, LocalObject>,
        chosen: &BTreeMap<ObjectKey, Version>,
        checked: &revalidation::CheckedRevalidation,
        query: &[AdmittedSnapshot],
        collect_native: bool,
    ) -> Result<CapturedSemanticValidation> {
        let root = Path::new(&application.root);
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
        for query in query {
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
            let lifecycle = if local_retired(root, local)? {
                "trash"
            } else {
                "live"
            };
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
                    if local_retired(root, old)? {
                        "trash"
                    } else {
                        "live"
                    },
                    None,
                )?);
            }
        }
        let resource_refs = resources.iter().collect::<Vec<_>>();
        let after_locations = application
            .before
            .iter()
            .chain(&application.after)
            .map(|object| (object.object.clone(), PathBuf::from(&object.directory)))
            .collect();
        let query_refs = query.iter().collect::<Vec<_>>();
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
        let capture = if collect_native {
            revalidation_evidence::capture_evidence_for_native(self, &cut, checked)?
        } else {
            revalidation_evidence::capture_evidence(self, &cut, checked)?
        };
        let historical_blobs = checked
            .historical_inputs
            .iter()
            .map(|input| {
                graph
                    .blob(&input.digest)
                    .map(|bytes| (input.digest.clone(), bytes.to_vec()))
                    .ok_or_else(|| {
                        refused(
                            "syncIncomplete",
                            "Exact historical semantic bytes are missing",
                        )
                    })
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        Ok(CapturedSemanticValidation {
            capture,
            checked: checked.clone(),
            historical_blobs,
        })
    }

    pub(crate) fn finish_sync_semantic_validation(
        &mut self,
        plan: &mut Plan,
        captured: CapturedSemanticValidation,
    ) -> Result<FrozenSemanticEvidence> {
        let CapturedSemanticValidation {
            capture,
            checked,
            historical_blobs,
        } = captured;
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
            .any(|object| !checked.states.contains_key(object))
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
        if let Some(members) = reads.query_trash_membership {
            if transaction
                .expected_trash_membership
                .as_ref()
                .is_some_and(|old| old != &members)
            {
                return Err(refused(
                    "scopeChanged",
                    "The original Trash Query membership differs from the writer cut",
                ));
            }
            transaction.expected_trash_membership = Some(members);
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
            let bytes = historical_blobs[&input.digest].as_slice();
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
            native_products: vec![],
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
    #[cfg(test)]
    pub(crate) fn validate_sync_semantic_evidence(
        &self,
        operation_id: &str,
        application: &SyncApplication,
        evidence: &FrozenSemanticEvidence,
    ) -> Result<()> {
        let adoption = self
            .operations
            .get(operation_id)
            .and_then(|plan| plan.transaction.as_ref())
            .is_some_and(|transaction| {
                matches!(
                    transaction.request.intent,
                    crate::OperationIntent::AdoptSyncExternal { .. }
                )
            });
        self.validate_sync_semantic_cut(operation_id, application, evidence, adoption)
    }

    pub(crate) fn validate_sync_semantic_request_evidence(
        &self,
        request: &crate::OperationRequest,
        application: &SyncApplication,
        evidence: &FrozenSemanticEvidence,
    ) -> Result<()> {
        if request.root != application.root {
            return Err(invalid(
                "Semantic evidence belongs to another original request root",
            ));
        }
        self.validate_sync_semantic_cut(
            &request.operation_id,
            application,
            evidence,
            matches!(
                request.intent,
                crate::OperationIntent::AdoptSyncExternal { .. }
            ),
        )
    }

    fn validate_sync_semantic_cut(
        &self,
        operation_id: &str,
        application: &SyncApplication,
        evidence: &FrozenSemanticEvidence,
        adoption: bool,
    ) -> Result<()> {
        for object in &evidence.absent_objects {
            let name = match object {
                ObjectKey::Schema => ".weftext-schema.json",
                ObjectKey::Rules => ".weftext-rules",
                ObjectKey::Settings => ".weftext-settings.json",
                _ => return Err(invalid("Unsupported absent semantic configuration")),
            };
            self.check_pending_except(
                &Path::new(&application.root).join(name),
                Some(operation_id),
            )?;
        }
        let graph = self.read_sync_history(&application.identity)?;
        let current = graph.objects()?;
        let installed = if adoption {
            Some(
                self.sync_local_state(self.sync_setup_for(Path::new(&application.root))?)?
                    .0,
            )
        } else {
            None
        };
        if evidence.absent_objects.iter().any(|object| {
            if let Some(installed) = &installed {
                return installed.contains_key(object);
            }
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
