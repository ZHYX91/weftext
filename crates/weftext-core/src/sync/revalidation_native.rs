//! Owned, non-wire native validation. The bounded application worker runs
//! outside Core's lock; the original private Plan and read cut stay here.
use super::revalidation_evidence::{self, PendingNativeInvocation};
use super::revalidation_writer::CapturedSemanticValidation;
use super::*;
use crate::backend::ParentGuard;
use crate::journal::Plan;
use crate::operations::{
    CapturedDirectory, SnapshotBinding, capture_optional, snapshot_tree, sorted_entries,
};
use crate::{
    Core, OperationIntent, OperationPreview, OperationRequest, Path, PathBuf, SnapshotWitness,
    absolute_path, display,
};
use weftext_messages::UiMessage;

pub enum SyncOperationPreparation {
    Prepared(Box<OperationPreview>),
    Native(Box<SyncNativePreparation>),
}

pub enum SyncReviewPreparation {
    Reviewed(SyncRevalidationReview),
    Native(Box<SyncNativeReview>),
}
/// A temporary read capture. It has no operation, Plan, or allocated revision.
pub struct SyncNativeReview {
    review: SyncRevalidationReview,
    identity: SyncIdentity,
    checked: super::revalidation::CheckedRevalidation,
    pending: Vec<PendingNativeInvocation>,
    memberships: Vec<(String, BTreeSet<String>)>,
    blobs: BTreeMap<String, Vec<u8>>,
    work: usize,
    native: NativeCapture,
}
impl SyncNativeReview {
    pub fn render_inputs(&self) -> &SyncNativeRenderInputs {
        &self.native.inputs
    }
}

/// Deliberately not Serialize/Deserialize/Clone. No HTTP field is a semantic
/// authority, and no operation exists until this exact capture is finished.
pub struct SyncNativePreparation {
    plan: Plan,
    semantic: CapturedSemanticValidation,
    native: NativeCapture,
}
impl SyncNativePreparation {
    pub fn render_inputs(&self) -> &SyncNativeRenderInputs {
        &self.native.inputs
    }
}

/// One shared bounded world, materialized into a worker request one note at a
/// time. Retaining 32 notes never retains 32 copies of a 64 MiB file map.
pub struct SyncNativeRenderInputs {
    root: PathBuf,
    files: BTreeMap<String, Vec<u8>>,
    style: Option<weftext_engine::StyleModule>,
    notes: Vec<(String, PathBuf)>,
}
impl SyncNativeRenderInputs {
    pub fn len(&self) -> usize {
        self.notes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }
    pub fn render_request(&self, index: usize) -> Option<weftext_engine::FrozenRenderRequest> {
        let (source, path) = self.notes.get(index)?;
        Some(weftext_engine::FrozenRenderRequest {
            source: source.clone(),
            path: path.clone(),
            root: self.root.clone(),
            files: self.files.clone(),
            style: self.style.clone(),
        })
    }
}

/// Produced only by the in-process adapter from its isolated worker result.
/// There is no serde input for installing a client supplied compilation proof.
pub struct SyncNativeRenderOutput {
    pub ok: bool,
    pub products: Option<weftext_engine::RenderProducts>,
    pub dependencies: Vec<weftext_engine::Dependency>,
    pub compiler_version: String,
}
impl From<weftext_engine::RenderOutput> for SyncNativeRenderOutput {
    fn from(output: weftext_engine::RenderOutput) -> Self {
        Self {
            ok: output.ok,
            products: output.products,
            dependencies: output.dependencies,
            compiler_version: output.compiler_version,
        }
    }
}

type NoteKey = (String, String, String);
struct NativeCapture {
    root: PathBuf,
    inputs: SyncNativeRenderInputs,
    keys: Vec<NoteKey>,
    owners: Vec<BTreeSet<ObjectKey>>,
    originals: BTreeMap<String, SnapshotWitness>,
    directories: Vec<CapturedDirectory>,
    directory_after: Vec<PathBuf>,
    object_files: BTreeMap<String, ObjectKey>,
    selected: BTreeSet<ObjectKey>,
    versions: BTreeMap<ObjectKey, Dependency>,
    settings_absent: bool,
    world_provenance: serde_json::Value,
}
struct NativeResult {
    products: BTreeMap<NoteKey, weftext_data::NoteProducts>,
    witnesses: Vec<SnapshotWitness>,
    reads: BTreeMap<ObjectKey, BTreeSet<ObjectKey>>,
    provenance: Vec<serde_json::Value>,
    unselected: BTreeSet<ObjectKey>,
}
fn unavailable(message: UiMessage) -> CoreError {
    CoreError::new("queryInvocationUnavailable", message)
}
fn relative(root: &Path, path: &Path) -> Result<String> {
    crate::history::history_relative_path(path, root)
        .filter(|path| !path.as_os_str().is_empty())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .ok_or_else(|| unavailable(UiMessage::error_core_sync_native_input_outside_original_root()))
}
fn same(a: &Path, b: &Path) -> bool {
    crate::history::history_relative_path(a, b).is_some_and(|path| path.as_os_str().is_empty())
}
fn style_from_files(
    files: &BTreeMap<String, Vec<u8>>,
    workspace: &str,
) -> Result<Option<weftext_engine::StyleModule>> {
    let settings: serde_json::Value = files
        .get(".weftext-settings.json")
        .map(|bytes| serde_json::from_slice(bytes))
        .transpose()?
        .unwrap_or_default();
    if settings["defaultStyle"]["enabled"] != true {
        return Ok(None);
    }
    let selector = settings["defaultStyle"]["resource"]
        .as_str()
        .ok_or_else(|| {
            unavailable(UiMessage::error_core_sync_exact_default_style_selector_unavailable())
        })?;
    if !selector.starts_with(&format!("weftext-resource://{workspace}/")) {
        return Err(unavailable(
            UiMessage::error_core_sync_exact_default_style_belongs_to_another_workspace(),
        ));
    }
    if settings["defaultStyle"]["entryFunction"]
        .as_str()
        .is_some_and(|entry| entry != "library-style")
    {
        return Err(unavailable(
            UiMessage::error_core_sync_selected_style_entry_function_unsupported(),
        ));
    }
    let registry: serde_json::Value =
        serde_json::from_slice(files.get("Styles/weftext.resources.json").ok_or_else(|| {
            unavailable(UiMessage::error_core_sync_original_style_resource_binding_missing())
        })?)?;
    let id = selector.rsplit('/').next().unwrap_or_default();
    uuid(id)?;
    if registry["version"] != 1 {
        return Err(CoreError::new(
            "styleSelection",
            UiMessage::error_core_sync_original_style_registry_version_unsupported(),
        ));
    }
    if !registry["resources"].as_array().is_some_and(|resources| {
        let exact = resources
            .iter()
            .filter(|entry| entry["resourceId"] == id)
            .collect::<Vec<_>>();
        exact.len() == 1 && exact[0]["path"] == "library-style.typ"
    }) {
        return Err(unavailable(
            UiMessage::error_core_sync_exact_style_resource_binding_ambiguous(),
        ));
    }
    let source = String::from_utf8(
        files
            .get("Styles/library-style.typ")
            .ok_or_else(|| unavailable(UiMessage::error_core_sync_original_style_source_missing()))?
            .clone(),
    )
    .map_err(|_| unavailable(UiMessage::error_core_sync_original_style_not_u_t_f8()))?;
    Ok(Some(weftext_engine::StyleModule {
        path: "Styles/library-style.typ".into(),
        source,
        enabled: true,
        entry_function: "library-style".into(),
    }))
}

impl Core {
    pub fn capture_sync_revalidation_review(
        &mut self,
        root: impl AsRef<Path>,
        identity: &SyncIdentity,
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
    ) -> Result<SyncReviewPreparation> {
        self.capture_sync_revalidation_review_for_operation(
            root,
            identity,
            resolutions,
            blobs,
            None,
        )
    }

    /// The caller may bind a read-only review to the original future request
    /// identity. No Plan, file revision or writer admission is allocated here.
    pub fn capture_sync_revalidation_review_for_operation(
        &mut self,
        root: impl AsRef<Path>,
        identity: &SyncIdentity,
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
        operation_id: Option<&str>,
    ) -> Result<SyncReviewPreparation> {
        if let Some(operation_id) = operation_id {
            crate::valid_operation_id(operation_id)?;
        }
        self.capture_sync_revalidation_review_seeded(
            root,
            identity,
            resolutions,
            blobs,
            &BTreeSet::new(),
            operation_id,
        )
    }

    fn capture_sync_revalidation_review_seeded(
        &mut self,
        root: impl AsRef<Path>,
        identity: &SyncIdentity,
        resolutions: &[SyncRevalidation],
        blobs: &BTreeMap<String, Vec<u8>>,
        native_candidates: &BTreeSet<ObjectKey>,
        operation_id: Option<&str>,
    ) -> Result<SyncReviewPreparation> {
        let root = absolute_path(root.as_ref())?;
        let mut review = self.review_sync_revalidation_seeded(
            &root,
            identity,
            resolutions,
            blobs,
            native_candidates,
        )?;
        review.operation_id = operation_id.map(str::to_owned);
        let local = self.sync_local_state(self.sync_setup_for(&root)?)?.0;
        let mut graph = self.read_sync_history(identity)?;
        for (digest, bytes) in blobs {
            graph.offer_blob(digest, bytes.clone())?;
        }
        let selected = review
            .reviewed_versions
            .iter()
            .map(|dependency| dependency.object.clone())
            .collect::<BTreeSet<_>>();
        let chosen = super::revalidation::chosen_revalidation_states(
            &graph,
            &selected,
            &review.resolutions,
        )?;
        let preliminary = super::revalidation::inspect_revalidation(
            &graph,
            &chosen,
            &local,
            &review.resolutions,
            &BTreeSet::new(),
        )?;
        if !preliminary.invocations.iter().any(|invocation| {
            weftext_data::query_products(&invocation.plan, &invocation.context.definitions)
                .is_ok_and(|products| !products.is_empty())
        }) {
            return Ok(SyncReviewPreparation::Reviewed(review));
        }
        let mut snapshots = vec![];
        let result = (|| {
            self.capture_sync_query_lifecycles(&root, &preliminary, &mut snapshots)?;
            let members =
                super::revalidation_writer::query_read_members(&root, &snapshots, &chosen, &local)?;
            if !members.is_subset(&selected) {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_sync_original_review_membership_changed_before_native_capture(),
                ));
            }
            let checked = super::revalidation::inspect_revalidation(
                &graph,
                &chosen,
                &local,
                &review.resolutions,
                &members,
            )?;
            let compound = super::relocate::requires_compound_relocation(&chosen, &local);
            let locations = if let Some(operation_id) = operation_id.filter(|_| compound) {
                super::compound::compound_locations(&root, operation_id, &chosen, &local)?
            } else {
                let mut locations = BTreeMap::new();
                for object in chosen.keys() {
                    review_location(
                        &root,
                        object,
                        &chosen,
                        &local,
                        operation_id,
                        &mut locations,
                        &mut BTreeSet::new(),
                    )?;
                }
                locations
            };
            let mut pending = vec![];
            for invocation in &checked.invocations {
                let lifecycles = weftext_data::required_query_lifecycles(
                    &invocation.plan,
                    &invocation.context.definitions,
                )
                .map_err(|_| {
                    unavailable(
                        UiMessage::error_core_sync_actual_review_lifecycle_closure_unavailable(),
                    )
                })?;
                let mut notes = vec![];
                for object in &invocation.members {
                    let ObjectKey::Node(id) = object else {
                        return Err(unavailable(
                            UiMessage::error_core_sync_actual_query_membership_not_node(),
                        ));
                    };
                    let state = &chosen[object].state;
                    let lifecycle = if super::revalidation::effective_retired(object, &chosen)? {
                        "trash"
                    } else {
                        "live"
                    };
                    if !lifecycles.iter().any(|required| required == lifecycle) {
                        continue;
                    }
                    let file = format!("{}.typ", state.name);
                    let source =
                        String::from_utf8(
                            graph
                                .blob(&state.files[&file])
                                .ok_or_else(|| {
                                    unavailable(
                                        UiMessage::error_core_sync_exact_review_source_missing(),
                                    )
                                })?
                                .to_vec(),
                        )
                        .map_err(|_| {
                            unavailable(UiMessage::error_core_sync_exact_review_source_not_u_t_f8())
                        })?;
                    notes.push(weftext_data::FrozenNote {
                        entity_key: format!("weftext-node://{}/{id}", identity.workspace_id),
                        path: display(&locations[object].join(&file)),
                        source,
                        revision: state.file_revisions[&file].clone(),
                        lifecycle: lifecycle.into(),
                        title: None,
                        body: None,
                        products: None,
                        authorized_fields: None,
                    });
                }
                let mut context = invocation.context.clone();
                for scope in
                    weftext_data::required_query_scopes(&invocation.plan, &context.definitions)
                        .map_err(|_| {
                            unavailable(UiMessage::error_core_sync_exact_review_scope_unavailable())
                        })?
                {
                    let (kind, workspace, id) = crate::directory_identity::parse_ref(&scope)?;
                    if workspace != identity.workspace_id {
                        return Err(unavailable(
                            UiMessage::error_core_sync_review_scope_belongs_to_another_workspace(),
                        ));
                    }
                    let object = if kind == "node" {
                        ObjectKey::Node(id.into())
                    } else if chosen.contains_key(&ObjectKey::Directory(id.into())) {
                        ObjectKey::Directory(id.into())
                    } else {
                        ObjectKey::RootDirectory(id.into())
                    };
                    context.scope_bindings.insert(
                        scope,
                        display(locations.get(&object).ok_or_else(|| {
                            unavailable(
                                UiMessage::error_core_sync_exact_review_scope_location_missing(),
                            )
                        })?),
                    );
                }
                let input = weftext_data::FrozenQueryInput {
                    cut_id: snapshots
                        .iter()
                        .map(|snapshot| snapshot.cut_id.as_str())
                        .collect::<Vec<_>>()
                        .join(":"),
                    schema_revision: invocation.schema_baseline.revision.clone(),
                    coverage: "complete".into(),
                    freshness: "stale".into(),
                    notes,
                    advanced: Some(context),
                };
                // Non-native actual calls are evaluated too; this same helper
                // completes them after all native note products are available.
                pending.push(PendingNativeInvocation {
                    input,
                    invocation: invocation.clone(),
                });
            }
            let mut after_files = BTreeMap::new();
            for (object, version) in &chosen {
                for (file, digest) in &version.state.files {
                    after_files.insert(
                        relative(&root, &locations[object].join(file))?,
                        graph
                            .blob(digest)
                            .ok_or_else(|| {
                                unavailable(
                                    UiMessage::error_core_sync_exact_native_review_blob_missing(),
                                )
                            })?
                            .to_vec(),
                    );
                }
            }
            let map_path = |path: &Path| {
                local
                    .iter()
                    .filter_map(|(object, old)| {
                        let new = locations.get(object)?;
                        let suffix =
                            crate::history::history_relative_path(path, Path::new(&old.directory))?;
                        let suffix = if matches!(object, ObjectKey::Node(_))
                            && suffix == Path::new(&format!("{}.typ", old.state.name))
                        {
                            PathBuf::from(format!("{}.typ", chosen[object].state.name))
                        } else {
                            suffix
                        };
                        Some((
                            Path::new(&old.directory).components().count(),
                            new.join(suffix),
                        ))
                    })
                    .max_by_key(|(depth, _)| *depth)
                    .map(|(_, path)| path)
                    .unwrap_or_else(|| path.to_path_buf())
            };
            let mut after_owners = BTreeMap::new();
            for (object, version) in &chosen {
                for file in version.state.files.keys() {
                    after_owners.insert(
                        relative(&root, &locations[object].join(file))?,
                        object.clone(),
                    );
                }
            }
            let native = self.capture_sync_native_inputs(
                &root,
                &graph,
                &pending,
                &local,
                &chosen,
                &review.reviewed_versions,
                map_path,
                after_files,
                after_owners,
                None,
            )?;
            let memberships = snapshots
                .iter()
                .map(|snapshot| {
                    (
                        if snapshot.inventory.status == "trashReadOnly" {
                            "trash"
                        } else {
                            "live"
                        }
                        .to_string(),
                        snapshot.membership_ids.iter().cloned().collect(),
                    )
                })
                .collect();
            Ok(SyncReviewPreparation::Native(Box::new(SyncNativeReview {
                review,
                identity: identity.clone(),
                checked,
                pending,
                memberships,
                blobs: blobs.clone(),
                work: native.inputs.len(),
                native,
            })))
        })();
        self.release_sync_query_lifecycles(&snapshots);
        result
    }

    pub fn finish_sync_native_review(
        &mut self,
        captured: Box<SyncNativeReview>,
        outputs: Vec<SyncNativeRenderOutput>,
    ) -> Result<SyncRevalidationReview> {
        match self.advance_sync_native_review(captured, outputs)? {
            SyncReviewPreparation::Reviewed(review) => Ok(review),
            SyncReviewPreparation::Native(_) => Err(CoreError::new(
                "syncDependencyRequired",
                UiMessage::error_core_sync_continue_exact_read_only_native_review_closure_before_preparing(),
            )),
        }
    }

    pub fn advance_sync_native_review(
        &mut self,
        captured: Box<SyncNativeReview>,
        outputs: Vec<SyncNativeRenderOutput>,
    ) -> Result<SyncReviewPreparation> {
        self.sync_local_owner()?;
        let mut captured = *captured;
        let root = &captured.native.root;
        let setup = self.sync_setup_for(root)?;
        if setup.identity != captured.identity {
            return Err(CoreError::new(
                "syncIdentity",
                UiMessage::error_core_sync_original_review_replica_changed(),
            ));
        }
        let local = self.sync_local_state(setup)?.0;
        let graph = self.read_sync_history(&captured.identity)?;
        let cut = super::revalidation::revalidation_cut(&graph, &local, &captured.native.selected)?;
        if cut.expected_installed != captured.review.expected_installed
            || cut.reviewed_versions != captured.review.reviewed_versions
        {
            return Err(CoreError::new(
                "syncConflictChanged",
                UiMessage::error_core_sync_exact_native_review_cut_changed_while_worker_ran(),
            ));
        }
        for (lifecycle, original) in &captured.memberships {
            let current = if lifecycle == "trash" {
                self.trash_admitted_snapshot(root)?
            } else {
                self.admitted_snapshot(root)?
            };
            let same = current.complete
                && current
                    .membership_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    == *original;
            self.release_sync_query_lifecycles(&[current]);
            if !same {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_sync_actual_native_review_membership_changed(),
                ));
            }
        }
        let result = self.finish_sync_native_inputs(&captured.native, outputs, true)?;
        if !result.unselected.is_empty() {
            let mut resolutions = captured.review.resolutions.clone();
            let current = graph.objects()?;
            for resolution in &mut resolutions {
                let mut declared = resolution
                    .reviewed_dependencies
                    .iter()
                    .map(|dependency| (dependency.object.clone(), dependency.clone()))
                    .collect::<BTreeMap<_, _>>();
                for object in result.reads.get(&resolution.object).into_iter().flatten() {
                    if result.unselected.contains(object) {
                        let view = current.get(object).ok_or_else(|| {
                            CoreError::new(
                                "syncDependencyRequired",
                                UiMessage::error_core_sync_actual_native_dependency_has_no_received_version(),
                            )
                        })?;
                        declared.insert(
                            object.clone(),
                            Dependency {
                                object: object.clone(),
                                revisions: view
                                    .versions
                                    .iter()
                                    .map(|version| version.revision.clone())
                                    .collect(),
                            },
                        );
                    }
                }
                resolution.reviewed_dependencies = declared.into_values().collect();
            }
            let candidates = captured
                .native
                .selected
                .union(&result.unselected)
                .cloned()
                .collect();
            let mut next = self.capture_sync_revalidation_review_seeded(
                root,
                &captured.identity,
                &resolutions,
                &captured.blobs,
                &candidates,
                captured.review.operation_id.as_deref(),
            )?;
            if let SyncReviewPreparation::Native(next) = &mut next {
                next.work = next.work.saturating_add(captured.work);
                if next.work > 32 {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_core_sync_native_dependency_discovery_exceeded_original32_note_worker_budget(),
                    ));
                }
            }
            return Ok(next);
        }
        revalidation_evidence::validate_native_invocations(captured.pending, &result.products)?;
        augment_native_dependencies(&mut captured.checked, &captured.native, &result.reads)?;
        for resolution in &mut captured.review.resolutions {
            resolution.reviewed_dependencies =
                captured.checked.dependency_reads[&resolution.object].clone();
        }
        let seeds = captured
            .review
            .resolutions
            .iter()
            .map(|resolution| resolution.object.clone())
            .chain(
                captured
                    .checked
                    .dependencies
                    .iter()
                    .map(|dependency| dependency.object.clone()),
            )
            .chain(result.reads.values().flatten().cloned())
            .collect();
        let final_selection =
            super::revalidation_writer::close_revalidation_selection(&graph, &local, &seeds)?;
        let final_cut = super::revalidation::revalidation_cut(&graph, &local, &final_selection)?;
        captured.review.expected_installed = final_cut.expected_installed;
        captured.review.reviewed_versions = final_cut.reviewed_versions;
        Ok(SyncReviewPreparation::Reviewed(captured.review))
    }

    /// Ordinary callers retain the existing synchronous metadata path. The
    /// application adapter uses this opaque handoff only for typed revalidation.
    pub fn capture_sync_operation(
        &mut self,
        request: OperationRequest,
    ) -> Result<SyncOperationPreparation> {
        if !matches!(
            request.intent,
            OperationIntent::RevalidateSync { .. } | OperationIntent::AdoptSyncExternal { .. }
        ) {
            return self
                .prepare_operation(request)
                .map(Box::new)
                .map(SyncOperationPreparation::Prepared);
        }
        self.sync_local_owner()?;
        crate::valid_operation_id(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        if let Some(original) = self.operations.get(&request.operation_id) {
            if original.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_sync_operation_id_already_names_different_input(),
                ));
            }
            return self
                .operation_details(&request.operation_id)
                .map(Box::new)
                .map(SyncOperationPreparation::Prepared);
        }
        let (mut plan, semantic) = match &request.intent {
            OperationIntent::RevalidateSync {
                identity,
                expected_installed,
                reviewed_versions,
                resolutions,
                blobs,
            } => {
                let input = self.prepare_sync_revalidation_input(
                    &request,
                    identity,
                    expected_installed,
                    reviewed_versions,
                    resolutions,
                    blobs,
                )?;
                let mut plan = self.build_sync_application_plan(input)?;
                let semantic =
                    self.capture_sync_revalidation_validation(&mut plan, resolutions, true)?;
                (plan, semantic)
            }
            OperationIntent::AdoptSyncExternal { .. } => {
                let (mut plan, semantic_required) =
                    self.build_sync_adoption_plan(request.clone())?;
                if !semantic_required {
                    return self
                        .stage_frozen_sync_application_plan(plan)
                        .map(Box::new)
                        .map(SyncOperationPreparation::Prepared);
                }
                let semantic = self.capture_sync_adoption_validation(&mut plan, true)?;
                (plan, semantic)
            }
            _ => unreachable!(),
        };
        if semantic.capture.pending_native.is_empty() || !semantic.capture.awaiting_native_only() {
            let evidence = self.finish_sync_semantic_validation(&mut plan, semantic)?;
            plan.sync_application.as_mut().unwrap().semantic_evidence = Some(evidence);
            return self
                .stage_frozen_sync_application_plan(plan)
                .map(Box::new)
                .map(SyncOperationPreparation::Prepared);
        }
        let root = absolute_path(Path::new(&request.root))?;
        let application = plan.sync_application.as_ref().unwrap();
        let local = self.sync_local_state(self.sync_setup_for(&root)?)?.0;
        let mut after_files = BTreeMap::new();
        for output in revalidation_evidence::original_outputs(plan.transaction.as_ref().unwrap()) {
            after_files.insert(relative(&root, Path::new(&output.path))?, output.after);
        }
        let mut after_owners = BTreeMap::new();
        for after in &application.after {
            for file in after.state.files.keys() {
                after_owners.insert(
                    relative(&root, &Path::new(&after.directory).join(file))?,
                    after.object.clone(),
                );
            }
        }
        let versions = if matches!(request.intent, OperationIntent::AdoptSyncExternal { .. }) {
            application
                .before
                .iter()
                .map(|before| Dependency {
                    object: before.object.clone(),
                    revisions: BTreeSet::from([before.revision.clone()]),
                })
                .collect()
        } else {
            application.expected_versions.clone()
        };
        let mut native_graph = self.read_sync_history(&application.identity)?;
        for (digest, bytes) in self.sync_local_state(self.sync_setup_for(&root)?)?.1 {
            native_graph.offer_blob(&digest, bytes.clone())?;
        }
        for (digest, bytes) in &application.blobs {
            native_graph.offer_blob(digest, bytes.clone())?;
        }
        let native = self.capture_sync_native_inputs(
            &root,
            &native_graph,
            &semantic.capture.pending_native,
            &local,
            &semantic.checked.states,
            &versions,
            |path| {
                PathBuf::from(
                    plan.transaction
                        .as_ref()
                        .unwrap()
                        .relocated_path(&display(path)),
                )
            },
            after_files,
            after_owners,
            Some(&plan),
        )?;
        self.validate_sync_native_plan(&plan, semantic.capture.witnesses())?;
        Ok(SyncOperationPreparation::Native(Box::new(
            SyncNativePreparation {
                plan,
                semantic,
                native,
            },
        )))
    }

    /// A late worker cannot replace or resurrect an original prepared,
    /// committed or cancelled Plan. Only the first exact request may stage.
    pub fn finish_sync_native_preparation(
        &mut self,
        prepared: Box<SyncNativePreparation>,
        outputs: Vec<SyncNativeRenderOutput>,
    ) -> Result<OperationPreview> {
        self.sync_local_owner()?;
        let mut prepared = *prepared;
        if let Some(original) = self.operations.get(&prepared.plan.operation_id) {
            if original.fingerprint != prepared.plan.fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_sync_original_operation_belongs_to_another_request(),
                ));
            }
            return self.operation_details(&original.operation_id);
        }
        self.validate_sync_native_plan(&prepared.plan, prepared.semantic.capture.witnesses())?;
        let result = self.finish_sync_native_inputs(&prepared.native, outputs, false)?;
        prepared.semantic.capture.add_native_witnesses(
            result.witnesses,
            result.reads.values().flatten().cloned().collect(),
        )?;
        augment_native_dependencies(
            &mut prepared.semantic.checked,
            &prepared.native,
            &result.reads,
        )?;
        if let OperationIntent::RevalidateSync { resolutions, .. } =
            &prepared.plan.transaction.as_ref().unwrap().request.intent
        {
            for resolution in resolutions {
                if prepared.semantic.checked.dependency_reads[&resolution.object]
                    != resolution.reviewed_dependencies
                {
                    return Err(CoreError::new(
                        "syncDependencyRequired",
                        UiMessage::error_core_sync_reviewed_native_dependency_cut_differs_from_exact_actual_reads(),
                    ));
                }
            }
        }
        prepared
            .semantic
            .capture
            .complete_native(&result.products)?;
        let mut evidence =
            self.finish_sync_semantic_validation(&mut prepared.plan, prepared.semantic)?;
        if prepared.native.settings_absent {
            evidence.absent_objects.insert(ObjectKey::Settings);
        }
        evidence.native_products = result.provenance;
        prepared
            .plan
            .sync_application
            .as_mut()
            .unwrap()
            .semantic_evidence = Some(evidence);
        self.validate_sync_native_plan(&prepared.plan, &[])?;
        self.stage_frozen_sync_application_plan(prepared.plan)
    }

    fn validate_sync_native_plan(&self, plan: &Plan, witnesses: &[SnapshotWitness]) -> Result<()> {
        self.validate_sync_recovery_cut(plan)?;
        self.validate_sync_plan_participants(plan)?;
        let transaction = plan.transaction.as_ref().unwrap();
        let root = Path::new(&transaction.request.root);
        self.validate_recovery_scope(transaction)?;
        for witness in witnesses {
            self.validate_snapshot_witness(root, &witness.binding.path, witness)?;
        }
        for file in &transaction.files {
            self.check_pending(Path::new(&file.path))?;
            let current = capture_optional(Path::new(&file.path))?;
            if current.as_ref().map(|file| (&file.stamp, &file.bytes))
                != Some((&file.stamp, &file.bytes))
            {
                return Err(CoreError::new(
                    "syncEvidenceChanged",
                    UiMessage::error_core_sync_original_private_plan_input_changed_while_native_worker_ran(),
                ));
            }
        }
        for (path, parent) in &transaction.write_parents {
            if ParentGuard::acquire(Path::new(path))?.directory_identity()? != *parent {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_sync_original_native_input_parent_changed(),
                ));
            }
        }
        for absent in &transaction.absent_dependencies {
            self.check_pending(Path::new(&absent.path))?;
            if capture_optional(Path::new(&absent.path))?.is_some()
                || ParentGuard::acquire(Path::new(&absent.path))?.directory_identity()?
                    != absent.parent
            {
                return Err(CoreError::new(
                    "scopeChanged",
                    UiMessage::error_core_sync_original_native_negative_dependency_changed(),
                ));
            }
        }
        let compound = matches!(
            transaction.mutation,
            crate::operations::Mutation::Compound { .. }
        );
        if compound {
            self.validate_compound_captured_membership(plan)?;
        }
        for directory in &transaction.directories {
            if compound
                && transaction.relocations().iter().any(|step| {
                    crate::history::history_relative_path(
                        Path::new(&directory.path),
                        Path::new(step.source),
                    )
                    .is_some()
                })
            {
                continue;
            }
            validate_directory(directory)?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_sync_native_inputs(
        &mut self,
        root: &Path,
        graph: &ReplicaGraph,
        invocations: &[PendingNativeInvocation],
        local: &BTreeMap<ObjectKey, super::replica::LocalObject>,
        states: &BTreeMap<ObjectKey, Version>,
        versions: &[Dependency],
        map_path: impl Fn(&Path) -> PathBuf,
        after_files: BTreeMap<String, Vec<u8>>,
        after_owners: BTreeMap<String, ObjectKey>,
        planned: Option<&Plan>,
    ) -> Result<NativeCapture> {
        let selected = states.keys().cloned().collect::<BTreeSet<_>>();
        let mut files = vec![];
        let mut directories = vec![];
        snapshot_tree(root, &mut files, &mut directories, false)?;
        let paths = files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>();
        let snapshots = self.read_bounded_snapshot(root, &paths, MAX_FILES)?;
        // Move every temporary base out before checking any item. A failed
        // original comparison must not leave the remaining bases in Core.
        let captured_witnesses = snapshots
            .files
            .into_iter()
            .map(|snapshot| {
                self.snapshots
                    .remove(&snapshot.base_id)
                    .map(|binding| SnapshotWitness { binding })
            })
            .collect::<Vec<_>>();
        let mut originals = BTreeMap::new();
        let mut render_files = BTreeMap::new();
        for (original, witness) in files.into_iter().zip(captured_witnesses) {
            let witness = witness.ok_or_else(|| {
                unavailable(UiMessage::error_core_sync_original_native_base_missing())
            })?;
            let captured = witness.binding.captured.as_ref().ok_or_else(|| {
                unavailable(UiMessage::error_core_sync_native_input_disappeared_during_capture())
            })?;
            if original.stamp != captured.stamp || original.bytes != captured.bytes {
                return Err(CoreError::new(
                    "syncEvidenceChanged",
                    UiMessage::error_core_sync_native_compile_input_changed_during_capture(),
                ));
            }
            let path = relative(root, &map_path(Path::new(&original.path)))?;
            if render_files.insert(path.clone(), original.bytes).is_some() {
                return Err(unavailable(
                    UiMessage::error_core_sync_native_after_input_locators_overlap(),
                ));
            }
            originals.insert(path, witness);
        }
        for directory in &directories {
            validate_directory(directory)?;
        }
        let directory_after = directories
            .iter()
            .map(|directory| map_path(Path::new(&directory.path)))
            .collect();
        let installed = selected
            .iter()
            .filter(|object| local.contains_key(*object))
            .cloned()
            .collect();
        let stamps = self.sync_installed_stamps(local, &installed, None)?;
        for object in &selected {
            let Some(installed) = local.get(object) else {
                continue;
            };
            for (file, digest) in &installed.state.files {
                let original_path = Path::new(&installed.directory).join(file);
                if planned
                    .is_some_and(|plan| super::adopt::adopted_removed_member(plan, &original_path))
                {
                    continue;
                }
                let logical =
                    relative(root, &map_path(&Path::new(&installed.directory).join(file)))?;
                let original = originals
                    .get(&logical)
                    .and_then(|witness| witness.binding.captured.as_ref())
                    .ok_or_else(|| {
                        CoreError::new(
                            "syncExternalChange",
                            UiMessage::error_core_sync_original_installed_native_input_missing(),
                        )
                    })?;
                let observed_adoption = planned.is_some_and(|plan| {
                    let transaction = plan.transaction.as_ref().unwrap();
                    matches!(
                        transaction.request.intent,
                        OperationIntent::AdoptSyncExternal { .. }
                    ) && original.stamp.identity
                        == stamps[&installed.state.file_revisions[file]].identity
                        && revalidation_evidence::original_outputs(transaction)
                            .iter()
                            .any(|output| {
                                same(Path::new(&output.path), Path::new(&original.path))
                                    && output.before.as_ref() == Some(&original.bytes)
                                    && output.after == original.bytes
                            })
                });
                if !observed_adoption
                    && (hash(&original.bytes) != *digest
                        || original.stamp != stamps[&installed.state.file_revisions[file]])
                {
                    return Err(CoreError::new(
                        "syncExternalChange",
                        UiMessage::error_core_sync_original_installed_native_input_has_external_edit_or_replacement(),
                    ));
                }
            }
        }
        let mut retired_files = BTreeSet::new();
        for object in &selected {
            let Some(installed) = local.get(object) else {
                continue;
            };
            for removed in super::member_retirement::qualify_member_retirements(
                graph,
                object,
                &installed.state,
                &states[object].state,
            )? {
                let original = Path::new(&installed.directory).join(&removed.relative_path);
                if let Some(plan) = planned
                    && !super::adopt::adopted_removed_member(plan, &original)
                {
                    let member = plan
                        .transaction
                        .as_ref()
                        .unwrap()
                        .retired_members()
                        .iter()
                        .find(|member| same(Path::new(&member.source), &original))
                        .ok_or_else(|| {
                            unavailable(
                                UiMessage::error_core_sync_native_resource_removal_has_no_original_plan_member_retirement(),
                            )
                        })?;
                    if &member.object != object
                        || member.relative_path != removed.relative_path
                        || member.resource_id != removed.resource_id
                        || member.digest != removed.digest
                        || member.file_revision != removed.file_revision
                        || member.identity != stamps[&removed.file_revision].identity
                    {
                        return Err(unavailable(
                            UiMessage::error_core_sync_native_resource_removal_differs_from_original_plan_f_i(),
                        ));
                    }
                }
                let logical = relative(root, &map_path(&original))?;
                if after_files.contains_key(&logical) {
                    return Err(unavailable(
                        UiMessage::error_core_sync_retired_resource_conflicts_with_planned_after_output(),
                    ));
                }
                retired_files.insert(logical);
            }
        }
        render_files.extend(after_files);
        for retired in &retired_files {
            render_files.remove(retired);
        }
        if render_files.len() > MAX_FILES
            || render_files.values().map(Vec::len).sum::<usize>() > MAX_BLOB_BYTES
        {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_native_after_input_exceeds5000_files64_mi_b(),
            ));
        }
        let mut object_files = BTreeMap::new();
        for (object, installed) in local {
            for file in installed.state.files.keys() {
                object_files.insert(
                    relative(root, &map_path(&Path::new(&installed.directory).join(file)))?,
                    object.clone(),
                );
            }
        }
        object_files.extend(after_owners);
        for retired in &retired_files {
            object_files.remove(retired);
        }
        let workspace = self.require_workspace(root)?.workspace_id;
        if render_files.contains_key(".weftext-settings.json")
            && (object_files.get(".weftext-settings.json") != Some(&ObjectKey::Settings)
                || !selected.contains(&ObjectKey::Settings))
        {
            return Err(CoreError::new(
                "syncDependencyRequired",
                UiMessage::error_core_sync_adopt_and_explicitly_review_original_settings_configuration_before_strong_native_validation(),
            ));
        }
        let style = style_from_files(&render_files, &workspace)?;
        if style.is_some() {
            let Some(ObjectKey::Node(owner)) = object_files.get("Styles/Styles.typ") else {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_sync_selected_style_has_no_original_admitted_node_owner(),
                ));
            };
            let owner_key = ObjectKey::Node(owner.clone());
            let state = &states
                .get(&owner_key)
                .ok_or_else(|| {
                    CoreError::new(
                        "syncDependencyRequired",
                        UiMessage::error_core_sync_original_style_owner_outside_reviewed_cut(),
                    )
                })?
                .state;
            let registry: serde_json::Value =
                serde_json::from_slice(&render_files["Styles/weftext.resources.json"])?;
            let settings: serde_json::Value =
                serde_json::from_slice(&render_files[".weftext-settings.json"])?;
            let reference = settings["defaultStyle"]["resource"].as_str().unwrap();
            let resource = reference.rsplit('/').next().unwrap();
            if !registry["resources"].as_array().is_some_and(|resources| {
                resources.iter().all(|entry| {
                    entry["resourceId"]
                        .as_str()
                        .is_some_and(|id| uuid(id).is_ok())
                })
            }) || state.files.get("weftext.resources.json")
                != Some(&hash(&render_files["Styles/weftext.resources.json"]))
                || state.files.get("library-style.typ")
                    != Some(&hash(&render_files["Styles/library-style.typ"]))
                || state.files.get("Styles.typ") != Some(&hash(&render_files["Styles/Styles.typ"]))
                || weftext_engine::inspect_metadata(&String::from_utf8_lossy(
                    &render_files["Styles/Styles.typ"],
                ))
                .id
                .as_ref()
                    != Some(owner)
            {
                return Err(CoreError::new(
                    "styleSelection",
                    UiMessage::error_core_sync_original_style_node_resource_role_does_not_match_exact_selected_state(),
                ));
            }
            if let Some(original) = originals
                .get("Styles/library-style.typ")
                .and_then(|witness| witness.binding.captured.as_ref())
            {
                if !self.admitted_resource(resource, &workspace, owner, &original.stamp.identity) {
                    return Err(CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_sync_selected_style_not_original_admitted_resource_object(),
                    ));
                }
            } else if let Some(plan) = planned {
                let path = root.join("Styles/library-style.typ");
                let transaction = plan.transaction.as_ref().unwrap();
                if transaction
                    .preview
                    .resource_map
                    .get(&display(&path))
                    .map(String::as_str)
                    != Some(reference)
                    || !revalidation_evidence::original_outputs(transaction)
                        .iter()
                        .any(|output| {
                            same(Path::new(&output.path), &path)
                                && output.before.is_none()
                                && output.after == render_files["Styles/library-style.typ"]
                        })
                {
                    return Err(CoreError::new(
                        "styleSelection",
                        UiMessage::error_core_sync_new_style_lacks_original_create_only_resource_materialization(),
                    ));
                }
            }
        }
        let settings_absent = !render_files.contains_key(".weftext-settings.json");
        let mut notes = BTreeMap::<NoteKey, (weftext_data::FrozenNote, BTreeSet<ObjectKey>)>::new();
        for invocation in invocations {
            for note in &invocation.input.notes {
                let key = (
                    note.entity_key.clone(),
                    note.revision.clone(),
                    hash(note.source.as_bytes()),
                );
                let entry = notes
                    .entry(key)
                    .or_insert_with(|| (note.clone(), BTreeSet::new()));
                if entry.0.path != note.path {
                    return Err(unavailable(
                        UiMessage::error_core_sync_one_exact_native_source_has_competing_after_locators(),
                    ));
                }
                entry
                    .1
                    .insert(ObjectKey::Node(invocation.invocation.owner.node_id.clone()));
            }
        }
        if notes.len() > 32 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_strong_native_validation_requires_at_most32_exact_notes(
                ),
            ));
        }
        let mut note_inputs = vec![];
        let mut keys = vec![];
        let mut owners = vec![];
        for (key, (note, readers)) in notes {
            if render_files
                .get(&relative(root, Path::new(&note.path))?)
                .map(Vec::as_slice)
                != Some(note.source.as_bytes())
            {
                return Err(unavailable(
                    UiMessage::error_core_sync_native_main_source_differs_from_original_exact_after_image(),
                ));
            }
            note_inputs.push((note.source, PathBuf::from(note.path)));
            keys.push(key);
            owners.push(readers);
        }
        let world_files = render_files
            .iter()
            .map(|(path, bytes)| serde_json::json!({"path":path,"digest":hash(bytes)}))
            .collect::<Vec<_>>();
        let style_provenance = style.as_ref().map(|style|serde_json::json!({"path":style.path,"digest":hash(style.source.as_bytes()),"enabled":style.enabled,"entryFunction":style.entry_function}));
        let world_digest = hash(&serde_json::to_vec(&(&world_files, &style_provenance))?);
        let world_provenance = serde_json::json!({"role":"frozenNativeInput","inputDigest":world_digest,"files":world_files,"style":style_provenance});
        if serde_json::to_vec(&world_provenance)?.len() > 4 * 1024 * 1024 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_sync_native_input_provenance_exceeds_bounded_metadata_budget(
                ),
            ));
        }
        Ok(NativeCapture {
            root: root.to_path_buf(),
            inputs: SyncNativeRenderInputs {
                root: root.to_path_buf(),
                files: render_files,
                style,
                notes: note_inputs,
            },
            keys,
            owners,
            originals,
            directories,
            directory_after,
            object_files,
            selected: selected.clone(),
            versions: versions
                .iter()
                .map(|dependency| (dependency.object.clone(), dependency.clone()))
                .collect(),
            settings_absent,
            world_provenance,
        })
    }

    fn finish_sync_native_inputs(
        &self,
        native: &NativeCapture,
        outputs: Vec<SyncNativeRenderOutput>,
        allow_discovery: bool,
    ) -> Result<NativeResult> {
        if outputs.len() != native.inputs.len() {
            return Err(unavailable(
                UiMessage::error_core_sync_native_worker_did_not_return_every_exact_compile(),
            ));
        }
        let mut products = BTreeMap::new();
        let mut witnesses = vec![];
        let mut reads = BTreeMap::<ObjectKey, BTreeSet<ObjectKey>>::new();
        let mut provenance = vec![native.world_provenance.clone()];
        let mut unselected = BTreeSet::new();
        for (index, output) in outputs.into_iter().enumerate() {
            let (_, source_path) = &native.inputs.notes[index];
            let identity = format!(
                "native-html:{}:{}:{}:{}",
                native.world_provenance["inputDigest"].as_str().unwrap(),
                native.keys[index].0,
                native.keys[index].1,
                native.keys[index].2
            );
            let mut paths = BTreeSet::new();
            if output.ok {
                paths.extend(
                    output
                        .dependencies
                        .iter()
                        .map(|dependency| dependency.path.clone()),
                );
                paths.insert(relative(&native.root, source_path)?);
                paths.insert(".weftext-settings.json".into());
                if let Some(style) = &native.inputs.style {
                    paths.insert(style.path.replace('\\', "/"));
                    paths.insert("Styles/weftext.resources.json".into());
                    paths.insert("Styles/Styles.typ".into());
                }
            }
            let mut actual = BTreeSet::new();
            for path in paths {
                let logical = relative(&native.root, &native.root.join(&path))?;
                if let Some(object) = native.object_files.get(&logical) {
                    if !native.selected.contains(object) {
                        if !allow_discovery {
                            return Err(CoreError::new(
                                "syncDependencyRequired",
                                UiMessage::error_core_sync_native_compiler_read_requires_explicit_exact_object_in_reviewed_cut(),
                            ));
                        }
                        unselected.insert(object.clone());
                    }
                    actual.insert(object.clone());
                }
                if let Some(original) = native.originals.get(&logical) {
                    self.validate_snapshot_witness(&native.root, &original.binding.path, original)?;
                    Core::merge_feature_witnesses(&mut witnesses, vec![original.clone()])?;
                } else if !native.inputs.files.contains_key(&logical) {
                    let negative = native_negative(native, Path::new(&logical))?;
                    self.validate_snapshot_witness(
                        &native.root,
                        &negative.binding.path,
                        &negative,
                    )?;
                    Core::merge_feature_witnesses(&mut witnesses, vec![negative])?;
                }
            }
            for owner in &native.owners[index] {
                reads
                    .entry(owner.clone())
                    .or_default()
                    .extend(actual.iter().filter(|object| *object != owner).cloned());
            }
            provenance.push(serde_json::json!({"entityKey":native.keys[index].0,"fileRevision":native.keys[index].1,
                "sourceHash":native.keys[index].2,"compileIdentity":identity,"target":"html","compilerVersion":output.compiler_version,"ok":output.ok,
                "dependencies":output.dependencies,"inputDigest":native.world_provenance["inputDigest"]}));
            let product = match (output.ok, output.products) {
                (true, Some(product)) if product.target == "html" => {
                    weftext_data::native_query_products(identity, product)
                }
                _ => {
                    let unavailable = || weftext_data::Cell::Unavailable {
                        reason: "The exact frozen native HTML compile did not qualify this product"
                            .into(),
                    };
                    weftext_data::NoteProducts {
                        compile_identity: identity,
                        target: "html".into(),
                        title: unavailable(),
                        body: unavailable(),
                        headings: unavailable(),
                    }
                }
            };
            products.insert(native.keys[index].clone(), product);
        }
        Ok(NativeResult {
            products,
            witnesses,
            reads,
            provenance,
            unselected,
        })
    }
}

fn validate_directory(directory: &CapturedDirectory) -> Result<()> {
    let path = Path::new(&directory.path);
    let guard = ParentGuard::acquire(&path.join(".native-capture-prospective"))?;
    let (entries, complete) = guard.directory_entries(MAX_FILES)?;
    if !complete
        || guard.directory_identity()? != directory.identity
        || sorted_entries(entries, path) != directory.entries
    {
        return Err(CoreError::new(
            "scopeChanged",
            UiMessage::error_core_sync_original_native_input_membership_changed_during_capture(),
        ));
    }
    Ok(())
}
fn native_negative(native: &NativeCapture, relative: &Path) -> Result<SnapshotWitness> {
    let requested = native.root.join(relative);
    let mut child = requested.as_path();
    while let Some(parent) = child.parent() {
        if let Some((original, _)) = native
            .directories
            .iter()
            .zip(&native.directory_after)
            .find(|(_, after)| same(after, parent))
        {
            let name = child
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    unavailable(UiMessage::error_core_sync_invalid_native_negative_dependency())
                })?;
            if original.entries.iter().any(|entry| {
                entry
                    .rsplit_once(':')
                    .is_some_and(|(entry, _)| entry.eq_ignore_ascii_case(name))
            }) {
                return Err(unavailable(
                    UiMessage::error_core_sync_compiler_omitted_existing_original_native_input(),
                ));
            }
            return Ok(SnapshotWitness {
                binding: SnapshotBinding {
                    path: display(&Path::new(&original.path).join(name)),
                    captured: None,
                    parent: original.identity.clone(),
                },
            });
        }
        child = parent;
    }
    Err(unavailable(
        UiMessage::error_core_sync_original_native_negative_namespace_unavailable(),
    ))
}
fn augment_native_dependencies(
    checked: &mut super::revalidation::CheckedRevalidation,
    native: &NativeCapture,
    reads: &BTreeMap<ObjectKey, BTreeSet<ObjectKey>>,
) -> Result<()> {
    for (owner, objects) in reads {
        let dependencies = checked.dependency_reads.get_mut(owner).ok_or_else(|| {
            unavailable(
                UiMessage::error_core_sync_native_invocation_has_no_original_semantic_owner(),
            )
        })?;
        let mut exact = dependencies
            .iter()
            .map(|dependency| (dependency.object.clone(), dependency.clone()))
            .collect::<BTreeMap<_, _>>();
        for object in objects {
            exact.insert(
                object.clone(),
                native
                    .versions
                    .get(object)
                    .ok_or_else(|| {
                        CoreError::new(
                            "syncDependencyRequired",
                            UiMessage::error_core_sync_native_dependency_has_no_exact_reviewed_received_versions(),
                        )
                    })?
                    .clone(),
            );
        }
        *dependencies = exact.into_values().collect();
    }
    Ok(())
}

fn review_location(
    root: &Path,
    object: &ObjectKey,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, super::replica::LocalObject>,
    operation_id: Option<&str>,
    output: &mut BTreeMap<ObjectKey, PathBuf>,
    visiting: &mut BTreeSet<ObjectKey>,
) -> Result<PathBuf> {
    if let Some(path) = output.get(object) {
        return Ok(path.clone());
    }
    if !visiting.insert(object.clone()) {
        return Err(unavailable(
            UiMessage::error_core_sync_reviewed_native_placement_contains_cycle(),
        ));
    }
    let state = &chosen
        .get(object)
        .ok_or_else(|| {
            unavailable(UiMessage::error_core_sync_exact_reviewed_native_parent_missing())
        })?
        .state;
    let path = if !matches!(object, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
        root.to_path_buf()
    } else if state.retired {
        if let Some(old) = local.get(object).filter(|old| old.state.retired) {
            PathBuf::from(&old.directory)
        } else {
            let operation_id = operation_id.ok_or_else(|| CoreError::new(
                "syncStructureRequired",
                UiMessage::error_core_sync_bind_review_to_original_operation_id_before_validating_newly_retired_native_source(),
            ))?;
            super::relocate::single_relocation_target(root, operation_id, state, local)?
        }
    } else {
        let parent = if let Some(parent) = &state.parent {
            review_location(root, parent, chosen, local, operation_id, output, visiting)?
        } else {
            root.to_path_buf()
        };
        parent.join(&state.name)
    };
    visiting.remove(object);
    output.insert(object.clone(), path.clone());
    Ok(path)
}

#[cfg(all(test, windows))]
#[path = "revalidation_native_tests.rs"]
mod tests;

#[cfg(all(test, windows))]
#[path = "revalidation_member_tests.rs"]
mod member_tests;
