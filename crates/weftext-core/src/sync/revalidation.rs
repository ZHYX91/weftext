//! Pure semantic review of one explicitly chosen received cut. No author IO,
//! staging, allocation or ambient inventory is performed here. The writer must
//! discharge every required physical/admission evidence item before staging.
use super::replica::LocalObject;
use super::*;
use weftext_data::{
    DefinitionRevision, FrozenAdvancedInput, FrozenDefinitionOwner, FrozenDefinitionSchema,
    QueryPlan, ResourceMetadata, Schema, ValidationContext,
};
use weftext_knowledge::NodeRef;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncRevalidation {
    pub object: ObjectKey,
    pub revisions: BTreeSet<String>,
    pub decision: SyncRevalidationDecision,
    pub state: ObjectState,
    pub reviewed_dependencies: Vec<Dependency>,
}

/// A preserved source keeps its native bindings and portable file revisions.
/// A repair is explicit author input; neither choice means "bind to latest".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SyncRevalidationDecision {
    Preserve { revision: String },
    Repair { base_revision: String },
}

impl SyncRevalidationDecision {
    fn base_revision(&self) -> &str {
        match self {
            Self::Preserve { revision } => revision,
            Self::Repair { base_revision } => base_revision,
        }
    }
}

/// An ordinary causal publication prepared for the existing application
/// writer. Pure preparation grants no current-author qualification: the
/// original writer still captures every semantic evidence obligation.
pub(crate) struct PreparedRevalidation {
    pub manifest: Manifest,
    pub states: BTreeMap<ObjectKey, Version>,
    pub publication_blobs: BTreeMap<String, Vec<u8>>,
    pub graph: ReplicaGraph,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoricalSemanticInput {
    pub object: ObjectKey,
    pub object_revision: String,
    pub file_revision: String,
    pub relative_path: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncAnnotationTargetStatus {
    pub owner: NodeRef,
    pub annotation_id: String,
    pub state: String,
    pub reason: String,
}

/// A request for trusted writer evidence, never a client-deserializable proof.
#[derive(Debug, Clone, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum RequiredSemanticEvidence {
    Object {
        object: ObjectKey,
        installed_revision: Option<String>,
    },
    Resource {
        owner: NodeRef,
        reference: String,
        file_revision: String,
    },
    Definition {
        owner: NodeRef,
        key: String,
        source_revision: String,
    },
    AbsentConfiguration {
        object: ObjectKey,
    },
    AnnotationTargets {
        owner: NodeRef,
        annotation_file_revision: String,
    },
    /// A closed actual call/selector is not allowed through as an unevaluated
    /// parameterized definition. The writer must supply its exact runtime cut.
    QueryInvocation {
        owner: NodeRef,
        key: String,
        source_revision: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeferredQueryValidation {
    pub owner: NodeRef,
    pub key: String,
    pub required_parameters: Vec<String>,
    pub dynamic_selectors: BTreeSet<String>,
}

#[derive(Clone, Serialize)]
pub(crate) struct InvocationValidation {
    pub owner: NodeRef,
    pub key: String,
    pub source_revision: String,
    pub plan: QueryPlan,
    pub baseline: Schema,
    pub schema_baseline: weftext_data::ProviderBaseline,
    pub definition: Option<weftext_data::QueryDefinitionInvocationSource>,
    pub context: FrozenAdvancedInput,
    pub members: BTreeSet<ObjectKey>,
}

/// A baseline-less trace entry means this exact root binding, never an
/// arbitrary equal Schema projection from another definition.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryValidationTrace {
    pub owner: NodeRef,
    pub key: String,
    pub source_revision: String,
    pub root_baseline: weftext_data::ProviderBaseline,
    pub definition: Option<weftext_data::QueryDefinitionInvocationSource>,
    #[serde(flatten)]
    pub dependencies: weftext_data::QueryProviderDependencies,
}
impl std::ops::Deref for QueryValidationTrace {
    type Target = weftext_data::QueryProviderDependencies;
    fn deref(&self) -> &Self::Target {
        &self.dependencies
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryValidationRequirements {
    pub owner: NodeRef,
    pub key: String,
    pub source_revision: String,
    pub root_baseline: weftext_data::ProviderBaseline,
    #[serde(flatten)]
    pub requirements: weftext_data::QueryProviderRequirements,
}
impl std::ops::Deref for QueryValidationRequirements {
    type Target = weftext_data::QueryProviderRequirements;
    fn deref(&self) -> &Self::Target {
        &self.requirements
    }
}

pub(crate) struct CheckedRevalidation {
    pub states: BTreeMap<ObjectKey, Version>,
    pub dependencies: Vec<Dependency>,
    pub historical_inputs: Vec<HistoricalSemanticInput>,
    pub required_evidence: Vec<RequiredSemanticEvidence>,
    /// Legitimate definitions with required root parameters are callable later;
    /// this never waives an accompanying QueryInvocation evidence obligation.
    pub deferred_queries: Vec<DeferredQueryValidation>,
    pub provider_requirements: Vec<QueryValidationRequirements>,
    pub provider_dependencies: Vec<QueryValidationTrace>,
    pub dependency_reads: BTreeMap<ObjectKey, Vec<Dependency>>,
    pub invocations: Vec<InvocationValidation>,
    pub annotation_targets: Vec<SyncAnnotationTargetStatus>,
}
impl CheckedRevalidation {
    /// Mandatory obligations for the original writer's captured cut. Merely
    /// checking semantics never makes this an applicable/committable Plan.
    pub(crate) fn evidence_obligations(&self) -> &[RequiredSemanticEvidence] {
        &self.required_evidence
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevalidationCutObject {
    pub object: ObjectKey,
    pub view: ObjectView,
    pub original_dependencies: Vec<(String, Vec<Dependency>)>,
    pub unsettled_dependencies: Vec<ObjectKey>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRevalidationCut {
    pub expected_installed: Vec<Dependency>,
    pub reviewed_versions: Vec<Dependency>,
    pub objects: Vec<RevalidationCutObject>,
    pub files: Vec<HistoricalSemanticInput>,
    pub coverage: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncRevalidationReview {
    pub expected_installed: Vec<Dependency>,
    pub reviewed_versions: Vec<Dependency>,
    /// Same exact decisions/after states, with actual semantic reads populated
    /// by Core. The user must review these before submitting prepare.
    pub resolutions: Vec<SyncRevalidation>,
    pub annotation_targets: Vec<SyncAnnotationTargetStatus>,
}

pub(crate) fn review_revalidation(
    mut graph: ReplicaGraph,
    local: &BTreeMap<ObjectKey, LocalObject>,
    selection: &BTreeSet<ObjectKey>,
    reviewed: &[SyncRevalidation],
    supplied: &BTreeMap<String, Vec<u8>>,
    query_members: &BTreeSet<ObjectKey>,
) -> Result<SyncRevalidationReview> {
    offer_proposed_blobs(&mut graph, supplied)?;
    let chosen = chosen_revalidation_states(&graph, selection, reviewed)?;
    let checked = inspect_revalidation(&graph, &chosen, local, reviewed, query_members)?;
    let selected = selection
        .union(&checked.states.keys().cloned().collect())
        .cloned()
        .collect();
    Ok(SyncRevalidationReview {
        expected_installed: installed_cut(local, &selected),
        reviewed_versions: received_cut(&graph, &selected)?,
        resolutions: reviewed
            .iter()
            .cloned()
            .map(|mut item| {
                item.reviewed_dependencies = checked.dependency_reads[&item.object].clone();
                item
            })
            .collect(),
        annotation_targets: checked.annotation_targets,
    })
}

fn installed_cut(
    local: &BTreeMap<ObjectKey, LocalObject>,
    selection: &BTreeSet<ObjectKey>,
) -> Vec<Dependency> {
    selection
        .iter()
        .filter_map(|object| {
            local.get(object).map(|installed| Dependency {
                object: object.clone(),
                revisions: BTreeSet::from([installed.revision.clone()]),
            })
        })
        .collect()
}

fn received_cut(graph: &ReplicaGraph, selection: &BTreeSet<ObjectKey>) -> Result<Vec<Dependency>> {
    let current = graph.objects()?;
    selection
        .iter()
        .map(|object| {
            current
                .get(object)
                .map(|view| Dependency {
                    object: object.clone(),
                    revisions: revisions(view),
                })
                .ok_or_else(|| {
                    refuse(
                        "syncConflictChanged",
                        "The explicitly reviewed received cut is unavailable",
                    )
                })
        })
        .collect()
}

fn offer_proposed_blobs(
    graph: &mut ReplicaGraph,
    supplied: &BTreeMap<String, Vec<u8>>,
) -> Result<()> {
    if supplied.len() > MAX_FILES
        || supplied
            .values()
            .try_fold(0_usize, |n, b| n.checked_add(b.len()))
            .is_none_or(|n| n > MAX_BLOB_BYTES)
    {
        return Err(refuse(
            "budget",
            "Revalidation blobs exceed 5000 files / 64 MiB",
        ));
    }
    for (digest, bytes) in supplied {
        graph.offer_blob(digest, bytes.clone())?;
    }
    Ok(())
}

fn refuse(code: &str, message: &str) -> CoreError {
    CoreError::new(code, message)
}
fn revisions(view: &ObjectView) -> BTreeSet<String> {
    view.versions.iter().map(|v| v.revision.clone()).collect()
}

/// `selection` must already be the original planner's transaction/placement
/// closure. No author inventory, allocation, staging or mutation occurs here.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_revalidation(
    mut graph: ReplicaGraph,
    local: &BTreeMap<ObjectKey, LocalObject>,
    selection: &BTreeSet<ObjectKey>,
    reviewed: &[SyncRevalidation],
    supplied: &BTreeMap<String, Vec<u8>>,
    replica_id: &str,
    operation_id: &str,
    query_members: &BTreeSet<ObjectKey>,
    expected_installed: &[Dependency],
    reviewed_versions: &[Dependency],
) -> Result<PreparedRevalidation> {
    uuid(replica_id)?;
    uuid(operation_id)?;
    let requested = reviewed
        .iter()
        .map(|r| r.object.clone())
        .collect::<BTreeSet<_>>();
    if requested.is_empty()
        || requested.len() != reviewed.len()
        || !requested.is_subset(selection)
        || selection.len() > MAX_CHANGES
        || supplied.len() > MAX_FILES
        || supplied
            .values()
            .try_fold(0_usize, |n, b| n.checked_add(b.len()))
            .is_none_or(|n| n > MAX_BLOB_BYTES)
    {
        return Err(invalid("Select one bounded exact revalidation proposal"));
    }
    let expected = expected_installed
        .iter()
        .map(|d| (d.object.clone(), d.revisions.clone()))
        .collect::<BTreeMap<_, _>>();
    let actual = installed_cut(local, selection)
        .into_iter()
        .map(|d| (d.object, d.revisions))
        .collect::<BTreeMap<_, _>>();
    if expected.len() != expected_installed.len() || expected != actual {
        return Err(refuse(
            "syncBasisChanged",
            "The explicitly reviewed installed cut changed before preparation",
        ));
    }
    let reviewed_cut = reviewed_versions
        .iter()
        .map(|d| (d.object.clone(), d.revisions.clone()))
        .collect::<BTreeMap<_, _>>();
    let received = received_cut(&graph, selection)?
        .into_iter()
        .map(|d| (d.object, d.revisions))
        .collect::<BTreeMap<_, _>>();
    if reviewed_cut.len() != reviewed_versions.len() || reviewed_cut != received {
        return Err(refuse(
            "syncConflictChanged",
            "The complete explicitly reviewed received transaction/read cut changed",
        ));
    }
    // This graph is an owned read from the original immutable store. A failed
    // proposal cannot publish or install its temporary local branch.
    offer_proposed_blobs(&mut graph, supplied)?;
    let current = graph.objects()?;
    let chosen = chosen_revalidation_states(&graph, selection, reviewed)?;
    check_revalidation(&graph, &chosen, local, reviewed, true, query_members)?;
    let manifest = Manifest {
        version: 1,
        workspace_id: graph.workspace_id.clone(),
        replica_id: replica_id.into(),
        operation_id: operation_id.into(),
        parents: selection
            .iter()
            .flat_map(|o| current[o].versions.iter().map(|v| v.commit.clone()))
            .collect(),
        changes: reviewed
            .iter()
            .map(|r| Change {
                object: r.object.clone(),
                before: r.revisions.clone(),
                revision: crate::fresh(),
                after: r.state.clone(),
            })
            .collect(),
        dependencies: selection
            .difference(&requested)
            .map(|o| Dependency {
                object: o.clone(),
                revisions: revisions(&current[o]),
            })
            .collect(),
    };
    let id = manifest.id()?;
    if supplied
        .keys()
        .any(|digest| !manifest.blobs().contains(digest))
    {
        return Err(invalid("Revalidation contains unrelated blobs"));
    }
    graph.offer_manifest(&id, manifest.clone())?;
    if graph.status(&id)? != Admission::Accepted {
        return Err(invalid(
            "Revalidated state has missing bytes or invalid causal predecessors",
        ));
    }
    let mut states = chosen;
    for change in &manifest.changes {
        states.insert(
            change.object.clone(),
            Version {
                commit: id.clone(),
                revision: change.revision.clone(),
                state: change.after.clone(),
            },
        );
    }
    Ok(PreparedRevalidation {
        manifest,
        states,
        publication_blobs: supplied.clone(),
        graph,
    })
}

pub(crate) fn chosen_revalidation_states(
    graph: &ReplicaGraph,
    selection: &BTreeSet<ObjectKey>,
    reviewed: &[SyncRevalidation],
) -> Result<BTreeMap<ObjectKey, Version>> {
    let requested = reviewed
        .iter()
        .map(|item| item.object.clone())
        .collect::<BTreeSet<_>>();
    if requested.is_empty()
        || requested.len() != reviewed.len()
        || !requested.is_subset(selection)
        || selection.len() > MAX_CHANGES
    {
        return Err(invalid("Select one bounded exact revalidation proposal"));
    }
    let current = graph.objects()?;
    let mut chosen = BTreeMap::new();
    for object in selection {
        let view = current.get(object).ok_or_else(|| {
            refuse(
                "syncIncomplete",
                "A selected received object is unavailable",
            )
        })?;
        let version = if let Some(item) = reviewed.iter().find(|r| &r.object == object) {
            let mut version = view
                .versions
                .iter()
                .find(|v| v.revision == item.decision.base_revision())
                .cloned()
                .ok_or_else(|| {
                    refuse(
                        "syncConflictChanged",
                        "The explicitly chosen base is no longer a current branch",
                    )
                })?;
            if matches!(item.decision, SyncRevalidationDecision::Preserve { .. })
                && version.state != item.state
            {
                return Err(refuse(
                    "syncConflictChanged",
                    "Preserve must retain the exact selected native state",
                ));
            }
            version.state = item.state.clone();
            version
        } else {
            let [version] = view.versions.as_slice() else {
                return Err(refuse(
                    "syncConflict",
                    "Every concurrent dependency needs an explicit decision",
                ));
            };
            if !graph.unsettled_dependencies(&version.commit)?.is_empty() {
                return Err(refuse(
                    "syncDependencyConflict",
                    "A changed dependency needs its own semantic decision",
                ));
            }
            version.clone()
        };
        chosen.insert(object.clone(), version);
    }
    Ok(chosen)
}

/// Mirror the original Manifest/causality portable-file invariant before any
/// proposed bytes can be used as a retained Schema or callee baseline. This is
/// an ephemeral check against the same accepted graph, not another authority.
fn validate_proposed_file_revisions(
    graph: &ReplicaGraph,
    chosen: &BTreeMap<ObjectKey, Version>,
) -> Result<()> {
    let mut bindings = BTreeMap::new();
    for manifest in graph.admitted.iter().map(|id| &graph.manifests[id]) {
        for change in &manifest.changes {
            for (path, revision) in &change.after.file_revisions {
                let binding = (&change.object, &change.after.files[path]);
                if bindings
                    .insert(revision, binding)
                    .is_some_and(|old| old != binding)
                {
                    return Err(invalid(
                        "A portable file revision was reused for other bytes or owner",
                    ));
                }
            }
        }
    }
    let mut file_count = 0_usize;
    for (object, version) in chosen {
        let state = &version.state;
        file_count = file_count.saturating_add(state.files.len());
        if file_count > MAX_FILES || state.files.is_empty() {
            return Err(invalid(
                "A content state must retain its exact bounded files",
            ));
        }
        if !state.files.keys().eq(state.file_revisions.keys())
            || state.file_revisions.values().collect::<BTreeSet<_>>().len()
                != state.file_revisions.len()
        {
            return Err(invalid(
                "Every portable file requires its exact distinct content revision",
            ));
        }
        for (path, revision) in &state.file_revisions {
            uuid(revision)?;
            let binding = (object, &state.files[path]);
            if bindings
                .insert(revision, binding)
                .is_some_and(|old| old != binding)
            {
                return Err(invalid(
                    "A portable file revision was reused for other bytes or owner",
                ));
            }
        }
    }
    Ok(())
}

/// `selection` is the original application planner's closed transaction cut,
/// not a namespace to discover. This reader copies metadata only, not blobs.
pub(crate) fn revalidation_cut(
    graph: &ReplicaGraph,
    local: &BTreeMap<ObjectKey, LocalObject>,
    selection: &BTreeSet<ObjectKey>,
) -> Result<SyncRevalidationCut> {
    if selection.is_empty() || selection.len() > MAX_CHANGES {
        return Err(refuse("budget", "Select a bounded nonempty reviewed cut"));
    }
    let current = graph.objects()?;
    let mut cut = SyncRevalidationCut {
        expected_installed: vec![],
        reviewed_versions: vec![],
        objects: vec![],
        files: vec![],
        coverage: "complete".into(),
    };
    if graph.manifests.iter().any(|(id, m)| {
        m.changes.iter().any(|c| selection.contains(&c.object))
            && matches!(graph.status(id), Ok(Admission::Staged { .. }))
    }) {
        cut.coverage = "partial".into();
    }
    for object in selection {
        let view = current
            .get(object)
            .ok_or_else(|| refuse("syncIncomplete", "Selected received object is unavailable"))?;
        if let Some(installed) = local.get(object) {
            cut.expected_installed.push(Dependency {
                object: object.clone(),
                revisions: BTreeSet::from([installed.revision.clone()]),
            });
        }
        cut.reviewed_versions.push(Dependency {
            object: object.clone(),
            revisions: revisions(view),
        });
        let mut originals = vec![];
        let mut unsettled = BTreeSet::new();
        for version in &view.versions {
            let manifest = graph
                .manifest(&version.commit)
                .ok_or_else(|| refuse("syncIncomplete", "Original received manifest is missing"))?;
            originals.push((version.revision.clone(), manifest.dependencies.clone()));
            unsettled.extend(graph.unsettled_dependencies(&version.commit)?);
            for (path, digest) in &version.state.files {
                if cut.files.len() == MAX_FILES {
                    return Err(refuse(
                        "budget",
                        "Reviewed file metadata exceeds 5000 records",
                    ));
                }
                cut.files.push(HistoricalSemanticInput {
                    object: object.clone(),
                    object_revision: version.revision.clone(),
                    file_revision: version.state.file_revisions.get(path).cloned().ok_or_else(
                        || refuse("syncIncomplete", "Received file revision is missing"),
                    )?,
                    relative_path: path.clone(),
                    digest: digest.clone(),
                });
            }
        }
        cut.objects.push(RevalidationCutObject {
            object: object.clone(),
            view: view.clone(),
            original_dependencies: originals,
            unsettled_dependencies: unsettled.into_iter().collect(),
        });
    }
    if serde_json::to_vec(&cut)?.len() > 16 * 1024 * 1024 {
        return Err(refuse("budget", "Reviewed cut JSON exceeds 16 MiB"));
    }
    Ok(cut)
}

struct Validator<'a> {
    graph: &'a ReplicaGraph,
    chosen: BTreeMap<ObjectKey, Version>,
    allow_discovery: bool,
    current: BTreeMap<ObjectKey, ObjectView>,
    local: &'a BTreeMap<ObjectKey, LocalObject>,
    used_blobs: BTreeSet<String>,
    bytes: usize,
    retained_metadata: usize,
    reads: BTreeSet<ObjectKey>,
    all_reads: BTreeSet<ObjectKey>,
    historical: BTreeMap<(ObjectKey, String), HistoricalSemanticInput>,
    evidence: BTreeMap<String, RequiredSemanticEvidence>,
    deferred: Vec<DeferredQueryValidation>,
    providers: Vec<QueryValidationRequirements>,
    provider_traces: Vec<QueryValidationTrace>,
    invocations: Vec<InvocationValidation>,
    query_members: &'a BTreeSet<ObjectKey>,
    annotation_targets: Vec<SyncAnnotationTargetStatus>,
}
impl Validator<'_> {
    fn evidence(&mut self, value: RequiredSemanticEvidence) -> Result<()> {
        let key = serde_json::to_string(&value)?;
        if self.evidence.len() == MAX_FILES && !self.evidence.contains_key(&key) {
            return Err(refuse("budget", "Semantic evidence exceeds 5000 records"));
        }
        self.evidence.insert(key, value);
        Ok(())
    }
    fn retain_metadata(&mut self, value: &impl Serialize) -> Result<()> {
        struct Counter(usize);
        impl std::io::Write for Counter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0 = self.0.saturating_add(bytes.len());
                if self.0 > MAX_BLOB_BYTES {
                    return Err(std::io::Error::other("semantic metadata budget"));
                }
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut counter = Counter(0);
        serde_json::to_writer(&mut counter, value)
            .map_err(|_| refuse("budget", "Semantic metadata exceeds 64 MiB"))?;
        self.retained_metadata = self.retained_metadata.saturating_add(counter.0);
        if self.bytes.saturating_add(self.retained_metadata) > MAX_BLOB_BYTES {
            return Err(refuse("budget", "Retained semantic closure exceeds 64 MiB"));
        }
        Ok(())
    }
    fn need(&mut self, object: &ObjectKey) -> Result<Version> {
        if !self.chosen.contains_key(object) && self.allow_discovery {
            let view = self.current.get(object).ok_or_else(|| {
                refuse(
                    "syncDependencyRequired",
                    "An actual semantic target has no accepted received state",
                )
            })?;
            let [version] = view.versions.as_slice() else {
                return Err(refuse(
                    "syncConflict",
                    &format!(
                        "Explicitly choose concurrent semantic dependency {}",
                        serde_json::to_string(object)?
                    ),
                ));
            };
            if !self
                .graph
                .unsettled_dependencies(&version.commit)?
                .is_empty()
            {
                return Err(refuse(
                    "syncDependencyConflict",
                    &format!(
                        "Explicitly revalidate changed semantic dependency {}",
                        serde_json::to_string(object)?
                    ),
                ));
            }
            self.chosen.insert(object.clone(), version.clone());
        }
        let version = self.chosen.get(object).cloned().ok_or_else(|| {
            refuse(
                "syncDependencyRequired",
                "Choose the exact semantic dependency state before revalidation",
            )
        })?;
        if !self.current.contains_key(object) {
            return Err(refuse(
                "syncDependencyRequired",
                "Semantic object has no accepted received version set",
            ));
        }
        self.reads.insert(object.clone());
        self.all_reads.insert(object.clone());
        self.evidence(RequiredSemanticEvidence::Object {
            object: object.clone(),
            installed_revision: self.local.get(object).map(|v| v.revision.clone()),
        })?;
        Ok(version)
    }
    fn blob(&mut self, digest: &str) -> Result<Vec<u8>> {
        let bytes = self
            .graph
            .blob(digest)
            .ok_or_else(|| refuse("syncIncomplete", "An exact semantic input blob is missing"))?;
        if self.used_blobs.insert(digest.into()) {
            self.bytes = self
                .bytes
                .checked_add(bytes.len())
                .ok_or_else(|| refuse("budget", "Semantic input budget overflow"))?;
            if self.bytes.saturating_add(self.retained_metadata) > MAX_BLOB_BYTES
                || self.used_blobs.len() > MAX_FILES
            {
                return Err(refuse(
                    "budget",
                    "Semantic inputs exceed 64 MiB / 5000 files",
                ));
            }
        }
        Ok(bytes.to_vec())
    }
    fn file(
        &mut self,
        _object: &ObjectKey,
        version: &Version,
        path: &str,
    ) -> Result<(String, Vec<u8>)> {
        let digest = version.state.files.get(path).ok_or_else(|| {
            refuse(
                "syncSemanticUnavailable",
                "The chosen state has no exact required input",
            )
        })?;
        let revision = version.state.file_revisions.get(path).ok_or_else(|| {
            refuse(
                "syncSemanticUnavailable",
                "The chosen input lacks its portable file revision",
            )
        })?;
        Ok((revision.clone(), self.blob(digest)?))
    }
    fn source(&mut self, object: &ObjectKey, version: &Version) -> Result<(String, String)> {
        let (revision, bytes) =
            self.file(object, version, &format!("{}.typ", version.state.name))?;
        let source = crate::decode(&bytes)?;
        let ObjectKey::Node(id) = object else {
            return Err(refuse(
                "syncSemanticUnavailable",
                "An exact Node source is required",
            ));
        };
        if weftext_engine::inspect_metadata(&source).id.as_ref() != Some(id) {
            return Err(refuse(
                "identityConflict",
                "Chosen source changes its canonical Node identity",
            ));
        }
        Ok((revision, source))
    }
    /// A binding revision is a portable FILE revision, not the enclosing
    /// object revision. Repeated retained records must agree on exact bytes.
    fn historical(&mut self, object: &ObjectKey, revision: &str) -> Result<(String, Vec<u8>)> {
        let mut found: Option<HistoricalSemanticInput> = None;
        for (commit, manifest) in &self.graph.manifests {
            if self.graph.status(commit)? != Admission::Accepted {
                continue;
            }
            for change in manifest.changes.iter().filter(|c| &c.object == object) {
                let path = match object {
                    ObjectKey::Node(_) => format!("{}.typ", change.after.name),
                    ObjectKey::Schema => ".weftext-schema.json".into(),
                    _ => {
                        return Err(refuse(
                            "syncSemanticUnavailable",
                            "Unsupported historical semantic role",
                        ));
                    }
                };
                if change
                    .after
                    .file_revisions
                    .get(&path)
                    .is_none_or(|r| r != revision)
                {
                    continue;
                }
                let digest = change
                    .after
                    .files
                    .get(&path)
                    .ok_or_else(|| {
                        refuse(
                            "syncIncomplete",
                            "Historical semantic blob descriptor is absent",
                        )
                    })?
                    .clone();
                let item = HistoricalSemanticInput {
                    object: object.clone(),
                    object_revision: change.revision.clone(),
                    file_revision: revision.into(),
                    relative_path: path,
                    digest,
                };
                if found.as_ref().is_some_and(|old| old.digest != item.digest) {
                    return Err(refuse(
                        "syncSemanticUnavailable",
                        "A retained file revision has ambiguous semantic bytes",
                    ));
                }
                found.get_or_insert(item);
            }
        }
        let found = found.ok_or_else(|| {
            refuse(
                "syncSemanticUnavailable",
                "The exact pinned semantic file revision is unavailable",
            )
        })?;
        let bytes = self.blob(&found.digest)?;
        self.historical
            .insert((object.clone(), revision.into()), found.clone());
        Ok((found.relative_path, bytes))
    }
    fn schema(&mut self) -> Result<Schema> {
        if !self.current.contains_key(&ObjectKey::Schema) {
            self.evidence(RequiredSemanticEvidence::AbsentConfiguration {
                object: ObjectKey::Schema,
            })?;
            return Schema::bundled()
                .map_err(|_| refuse("schemaUnavailable", "Bundled Schema is unavailable"));
        }
        let version = self.need(&ObjectKey::Schema)?;
        if version.state.retired {
            return Err(refuse("schemaUnavailable", "Chosen Schema is retired"));
        }
        let (_, bytes) = self.file(&ObjectKey::Schema, &version, ".weftext-schema.json")?;
        Schema::from_author_json(&crate::decode(&bytes)?).map_err(|_| {
            refuse(
                "schemaUnavailable",
                "The explicitly chosen Schema is invalid",
            )
        })
    }
    fn rules(&mut self) -> Result<()> {
        if !self.current.contains_key(&ObjectKey::Rules) {
            self.evidence(RequiredSemanticEvidence::AbsentConfiguration {
                object: ObjectKey::Rules,
            })?;
            return Ok(());
        }
        let version = self.need(&ObjectKey::Rules)?;
        if version.state.retired {
            return Err(refuse("rulesUnavailable", "Chosen Rules are retired"));
        }
        let (_, bytes) = self.file(&ObjectKey::Rules, &version, ".weftext-rules")?;
        crate::validate_library_rules(&bytes)
    }
    fn node(&self, id: &str) -> NodeRef {
        NodeRef {
            workspace_id: self.graph.workspace_id.clone(),
            node_id: id.into(),
        }
    }
    fn target(&self, reference: &str, family: &str) -> Result<String> {
        let prefix = format!("weftext-{family}://{}/", self.graph.workspace_id);
        let id = reference.strip_prefix(&prefix).ok_or_else(|| {
            refuse(
                "syncDependencyRequired",
                "A typed target is outside the selected workspace",
            )
        })?;
        uuid(id)?;
        Ok(id.into())
    }
    fn resource_context(
        &mut self,
        object: &ObjectKey,
        version: &Version,
    ) -> Result<ValidationContext> {
        let ObjectKey::Node(id) = object else {
            return Ok(ValidationContext::default());
        };
        let owner = self.node(id);
        let resources = super::apply::validate_state(self.graph, object, &version.state)?;
        let mut context = ValidationContext {
            owner: Some(format!(
                "weftext-node://{}/{}",
                owner.workspace_id, owner.node_id
            )),
            resources: vec![],
            resources_complete: true,
        };
        for (path, id) in resources {
            let reference = format!("weftext-resource://{}/{id}", self.graph.workspace_id);
            let (revision, _) = self.file(object, version, &path)?;
            self.evidence(RequiredSemanticEvidence::Resource {
                owner: owner.clone(),
                reference: reference.clone(),
                file_revision: revision,
            })?;
            context.resources.push(ResourceMetadata {
                resource_ref: reference,
                owner: context.owner.clone().unwrap(),
                lifecycle: if version.state.retired {
                    "trash"
                } else {
                    "live"
                }
                .into(),
            });
        }
        Ok(context)
    }
    fn typed_target(&mut self, kind: &str, reference: &str) -> Result<()> {
        match kind {
            "node_ref" => {
                let id = self.target(reference, "node")?;
                let object = ObjectKey::Node(id);
                let version = self.need(&object)?;
                self.source(&object, &version)?;
            }
            "tag_ref" => {
                let id = self.target(reference, "tag")?;
                let version = self.need(&ObjectKey::Tags)?;
                let (_, bytes) = self.file(&ObjectKey::Tags, &version, ".weftext-tags.json")?;
                crate::directory_identity::validate_sync_tags(&bytes)?;
                let tags: serde_json::Value = serde_json::from_slice(&bytes)?;
                if tags["tags"]
                    .as_array()
                    .is_none_or(|a| a.iter().filter(|r| r["tagId"] == id).count() != 1)
                {
                    return Err(refuse(
                        "syncSemanticUnavailable",
                        "The chosen tag registry has no unique typed target",
                    ));
                }
            }
            "resource_ref" => {
                let id = self.target(reference, "resource")?;
                let mut found = vec![];
                if self.allow_discovery {
                    // Resource IDs are owner-local declarations. Read only
                    // actual resource-manifest candidates, never every source
                    // body, before asserting their unique received owner.
                    let candidates =
                        self.current
                            .iter()
                            .filter(|(object, view)| {
                                matches!(object, ObjectKey::Node(_))
                                    && view.versions.iter().any(|v| {
                                        v.state.files.contains_key("weftext.resources.json")
                                    })
                            })
                            .map(|(object, _)| object.clone())
                            .collect::<Vec<_>>();
                    for candidate in candidates {
                        self.need(&candidate)?;
                    }
                }
                let candidates = self
                    .chosen
                    .iter()
                    .filter(|(o, _)| matches!(o, ObjectKey::Node(_)))
                    .map(|(o, v)| (o.clone(), v.clone()))
                    .collect::<Vec<_>>();
                for (object, version) in candidates {
                    if !version.state.files.contains_key("weftext.resources.json") {
                        continue;
                    }
                    self.need(&object)?;
                    let resources =
                        super::apply::validate_state(self.graph, &object, &version.state)?;
                    if resources.values().any(|r| r == &id) {
                        found.push(object.clone());
                    }
                }
                if found.len() != 1 {
                    return Err(refuse(
                        "syncDependencyRequired",
                        "Choose the unique owner of the actual typed resource",
                    ));
                }
                let object = found.remove(0);
                let version = self.need(&object)?;
                self.resource_context(&object, &version)?;
            }
            _ => {
                return Err(refuse(
                    "syncSemanticUnavailable",
                    "Typed target kind is unsupported",
                ));
            }
        }
        Ok(())
    }
    fn work(
        &mut self,
        reference: &str,
        schema: &Schema,
    ) -> Result<weftext_export::bibliography::FrozenWork> {
        let id = self.target(reference, "node")?;
        let object = ObjectKey::Node(id.clone());
        let version = self.need(&object)?;
        if version.state.retired {
            return Err(refuse(
                "citationWorkUnavailable",
                "Chosen citation Work is retired",
            ));
        }
        let (revision, source) = self.source(&object, &version)?;
        let (_, _, types) = weftext_data::narrow_query_metadata(&source, schema).map_err(|_| {
            refuse(
                "citationWorkUnavailable",
                "Chosen Work control is unavailable",
            )
        })?;
        if !types.iter().any(|t| t == "library/work") {
            return Err(refuse(
                "citationWorkType",
                "Chosen citation target is not a Work",
            ));
        }
        let profile = weftext_data::narrow_profile(
            &source,
            schema,
            &BTreeSet::from(["library/publication".into()]),
            None,
        )
        .map_err(|_| {
            refuse(
                "citationPublicationUnavailable",
                "Chosen Work publication is unavailable",
            )
        })?;
        let publication = profile
            .fields
            .iter()
            .find(|f| f.field_id == "library/publication")
            .and_then(|f| f.cell.native())
            .cloned()
            .ok_or_else(|| {
                refuse(
                    "citationPublicationUnavailable",
                    "Chosen Work publication is absent or invalid",
                )
            })?;
        Ok(weftext_export::bibliography::FrozenWork {
            entity_ref: reference.into(),
            source_basis: format!("file:{revision}"),
            publication,
        })
    }
    fn queries(
        &mut self,
        object: &ObjectKey,
        source_revision: &str,
        source: &str,
        current_schema: &Schema,
    ) -> Result<()> {
        let ObjectKey::Node(id) = object else {
            return Ok(());
        };
        let owner = self.node(id);
        let mut definitions = weftext_data::saved_queries(source).map_err(|_| {
            refuse(
                "definitionUnavailable",
                "Chosen saved definitions are malformed",
            )
        })?;
        weftext_knowledge::saved_query_reference_slots(source, current_schema, &owner).map_err(
            |_| {
                refuse(
                    "definitionUnavailable",
                    "Chosen Query references are not exact native controls",
                )
            },
        )?;
        weftext_knowledge::definition_slots(source, &owner).map_err(|_| {
            refuse(
                "definitionUnavailable",
                "Chosen definition keys are ambiguous or computed",
            )
        })?;
        for occurrence in weftext_data::embedded_queries(source) {
            let (Some(key), Some(plan), Some(binding)) =
                (occurrence.key, occurrence.plan, occurrence.binding)
            else {
                return Err(refuse(
                    "definitionUnavailable",
                    "An embedded Query lacks an exact plan or binding",
                ));
            };
            if !definitions.iter().any(|definition| {
                definition.key == key
                    && serde_json::to_value(&definition.plan).ok()
                        == serde_json::to_value(&plan).ok()
                    && serde_json::to_value(&definition.binding).ok()
                        == serde_json::to_value(&binding).ok()
            }) {
                definitions.push(weftext_data::SavedQuery {
                    key,
                    label: occurrence.source.label,
                    binding,
                    plan,
                });
            }
        }
        for definition in definitions {
            self.evidence(RequiredSemanticEvidence::Definition {
                owner: owner.clone(),
                key: definition.key.clone(),
                source_revision: source_revision.into(),
            })?;
            let role = format!("weftext-config://{}/schema", self.graph.workspace_id);
            if definition.binding.schema != role
                || definition.binding.language != "weftext.query.cel.v1"
            {
                return Err(refuse(
                    "schemaUnavailable",
                    "Chosen definition names another Schema authority or language",
                ));
            }
            let mut context = FrozenAdvancedInput {
                definitions: vec![FrozenDefinitionOwner {
                    owner: format!("weftext-node://{}/{}", owner.workspace_id, owner.node_id),
                    history: vec![DefinitionRevision {
                        revision: source_revision.into(),
                        source: source.into(),
                        current: true,
                    }],
                }],
                ..Default::default()
            };
            self.load_schema(&mut context, &definition.binding)?;
            for _ in 0..64 {
                let deps =
                    weftext_data::definition_dependencies(&definition.plan, &context.definitions)
                        .map_err(|_| {
                        refuse(
                            "definitionUnavailable",
                            "Query call closure is invalid or cyclic",
                        )
                    })?;
                let mut progress = false;
                for dep in deps {
                    let id = self.target(&dep.owner, "node")?;
                    let target = ObjectKey::Node(id.clone());
                    let chosen = self.need(&target)?;
                    let (current_revision, current_source) = self.source(&target, &chosen)?;
                    let index = match context
                        .definitions
                        .iter()
                        .position(|o| o.owner == dep.owner)
                    {
                        Some(index) => index,
                        None => {
                            progress = true;
                            context.definitions.push(FrozenDefinitionOwner {
                                owner: dep.owner.clone(),
                                history: vec![DefinitionRevision {
                                    revision: current_revision,
                                    source: current_source,
                                    current: true,
                                }],
                            });
                            context.definitions.len() - 1
                        }
                    };
                    if let weftext_data::DefinitionSelection::Pinned { revision, .. } =
                        &dep.selection
                        && !context.definitions[index]
                            .history
                            .iter()
                            .any(|r| &r.revision == revision)
                    {
                        let (_, bytes) = self.historical(&target, revision)?;
                        let historical_source = crate::decode(&bytes)?;
                        if weftext_engine::inspect_metadata(&historical_source)
                            .id
                            .as_ref()
                            != Some(&id)
                        {
                            return Err(refuse(
                                "identityConflict",
                                "Pinned callee source changes its logical owner identity",
                            ));
                        }
                        progress = true;
                        context.definitions[index].history.push(DefinitionRevision {
                            revision: revision.clone(),
                            source: historical_source,
                            current: false,
                        });
                    }
                    let called = weftext_data::select_definition(
                        &dep.selection,
                        &context.definitions[index].history,
                    )
                    .map_err(|_| {
                        refuse(
                            "definitionUnavailable",
                            "The exact current/pinned callee is missing",
                        )
                    })?;
                    self.evidence(RequiredSemanticEvidence::Definition {
                        owner: self.node(&id),
                        key: called.key.clone(),
                        source_revision: match &dep.selection {
                            weftext_data::DefinitionSelection::Pinned { revision, .. } => {
                                revision.clone()
                            }
                            _ => context.definitions[index]
                                .history
                                .iter()
                                .find(|r| r.current)
                                .unwrap()
                                .revision
                                .clone(),
                        },
                    })?;
                    self.load_schema(&mut context, &called.binding)?;
                }
                if !progress {
                    break;
                }
                if context.definitions.len() > 64 {
                    return Err(refuse("budget", "Query owner closure exceeds 64 records"));
                }
            }
            let baseline = context
                .schemas
                .iter()
                .find(|s| {
                    s.schema == definition.binding.schema
                        && s.revision == definition.binding.revision
                })
                .unwrap()
                .projection
                .clone();
            if !weftext_data::baseline_contract_compatible(&baseline, current_schema)
                .map_err(|_| refuse("schemaUnavailable", "Schema compatibility cannot be proved"))?
            {
                return Err(refuse(
                    "schemaBasisChanged",
                    "Original Query baseline is incompatible with the chosen current Schema",
                ));
            }
            weftext_data::validate_query_plan_with_context(&definition.plan, &baseline, &context)
                .map_err(|_| {
                refuse(
                    "invalidQuery",
                    "Query is not typed under its exact callee/baseline closure",
                )
            })?;
            let requirements =
                weftext_data::query_provider_requirements(&definition.plan, &baseline, &context)
                    .map_err(|_| {
                        refuse(
                            "providerUnavailable",
                            "Exact Provider/validator/series requirements cannot be proved",
                        )
                    })?;
            let dynamic = requirements
                .iter()
                .flat_map(|r| r.dynamic_selectors.iter().cloned())
                .collect::<BTreeSet<_>>();
            let has_call =
                !weftext_data::definition_dependencies(&definition.plan, &context.definitions)
                    .map_err(|_| refuse("invalidQuery", "Actual call closure is invalid"))?
                    .is_empty();
            let provider_call = requirements.iter().any(|r| !r.provider_calls.is_empty());
            // This is the original Data compiler/evaluator's definition-time
            // preflight. It checks every closed argument/Provider subtree even
            // when another required root parameter is legitimately unknown.
            let preflight = weftext_data::preflight_query_definition(
                &definition.plan,
                &baseline,
                &context,
                &weftext_data::QueryBudget::default(),
            )
            .map_err(|_| {
                refuse(
                    "queryInvocationUnavailable",
                    "Exact closed Query/callee/Provider preflight is invalid",
                )
            })?;
            let root_baseline = weftext_data::ProviderBaseline {
                schema: definition.binding.schema.clone(),
                revision: definition.binding.revision.clone(),
            };
            let parameters = preflight.deferred_parameters;
            if !parameters.is_empty() {
                self.deferred.push(DeferredQueryValidation {
                    owner: owner.clone(),
                    key: definition.key.clone(),
                    required_parameters: parameters,
                    dynamic_selectors: dynamic.clone(),
                });
            }
            if has_call
                || provider_call
                || preflight.provider_dependencies.baselines.iter().any(|b| {
                    !b.providers.is_empty()
                        || !b.series_policies.is_empty()
                        || !b.exact_provider_selections.is_empty()
                        || !b.series_selections.is_empty()
                })
            {
                let trace = QueryValidationTrace {
                    owner: owner.clone(),
                    key: definition.key.clone(),
                    source_revision: source_revision.into(),
                    root_baseline: root_baseline.clone(),
                    definition: None,
                    dependencies: preflight.provider_dependencies,
                };
                self.retain_metadata(&trace)?;
                self.provider_traces.push(trace);
            }
            // Data identifies every independently closed actual invocation.
            // A genuinely unknown branch remains deferred; a closed sibling
            // still executes against the original complete after-row cut.
            if !preflight.closed_invocations.is_empty() {
                self.evidence(RequiredSemanticEvidence::QueryInvocation {
                    owner: owner.clone(),
                    key: definition.key.clone(),
                    source_revision: source_revision.into(),
                })?;
                for closed in preflight.closed_invocations {
                    let mut invocation_context = context.clone();
                    invocation_context.parameters = closed.parameters;
                    let invocation = InvocationValidation {
                        owner: owner.clone(),
                        key: definition.key.clone(),
                        source_revision: source_revision.into(),
                        plan: closed.plan,
                        baseline: closed.baseline,
                        schema_baseline: closed
                            .schema_baseline
                            .unwrap_or_else(|| root_baseline.clone()),
                        definition: closed.definition,
                        context: invocation_context,
                        members: self.query_members.clone(),
                    };
                    self.retain_metadata(&invocation)?;
                    self.invocations.push(invocation);
                }
                for member in self.query_members.clone() {
                    if !matches!(member, ObjectKey::Node(_)) {
                        return Err(invalid("An actual Query row requires an exact Node owner"));
                    }
                    self.need(&member)?;
                }
            }
            for requirements in requirements {
                let requirements = QueryValidationRequirements {
                    owner: owner.clone(),
                    key: definition.key.clone(),
                    source_revision: source_revision.into(),
                    root_baseline: root_baseline.clone(),
                    requirements,
                };
                self.retain_metadata(&requirements)?;
                self.providers.push(requirements);
            }
            self.scopes(&definition.plan, &context)?;
        }
        Ok(())
    }
    fn load_schema(
        &mut self,
        context: &mut FrozenAdvancedInput,
        binding: &weftext_data::SavedQueryBinding,
    ) -> Result<()> {
        let role = format!("weftext-config://{}/schema", self.graph.workspace_id);
        if binding.schema != role || binding.language != "weftext.query.cel.v1" {
            return Err(refuse(
                "schemaUnavailable",
                "Callee Schema role/language is invalid",
            ));
        }
        if context
            .schemas
            .iter()
            .any(|s| s.schema == binding.schema && s.revision == binding.revision)
        {
            return Ok(());
        }
        let bytes = if let Some(current) =
            self.chosen.get(&ObjectKey::Schema).cloned().filter(|v| {
                v.state.file_revisions.get(".weftext-schema.json") == Some(&binding.revision)
            }) {
            self.file(&ObjectKey::Schema, &current, ".weftext-schema.json")?
                .1
        } else {
            self.historical(&ObjectKey::Schema, &binding.revision)?.1
        };
        let projection = Schema::from_author_json(&crate::decode(&bytes)?).map_err(|_| {
            refuse(
                "schemaUnavailable",
                "The exact retained Schema baseline is invalid",
            )
        })?;
        if context.schemas.len() >= 64 {
            return Err(refuse(
                "budget",
                "Query baseline closure exceeds 64 records",
            ));
        }
        context.schemas.push(FrozenDefinitionSchema {
            schema: role,
            revision: binding.revision.clone(),
            projection,
        });
        Ok(())
    }
    fn scopes(&mut self, plan: &QueryPlan, context: &FrozenAdvancedInput) -> Result<()> {
        let scopes =
            weftext_data::required_query_scopes(plan, &context.definitions).map_err(|_| {
                refuse(
                    "queryScopeUnavailable",
                    "Nested Query scope cannot be proved",
                )
            })?;
        for scope in scopes {
            let target = if scope.starts_with("weftext-node://") {
                ObjectKey::Node(self.target(&scope, "node")?)
            } else if scope.starts_with("weftext-directory://") {
                let id = self.target(&scope, "directory")?;
                if self.current.contains_key(&ObjectKey::Directory(id.clone())) {
                    if self
                        .current
                        .contains_key(&ObjectKey::RootDirectory(id.clone()))
                    {
                        return Err(refuse(
                            "queryScopeUnavailable",
                            "The exact directory identity has ambiguous roles",
                        ));
                    }
                    ObjectKey::Directory(id)
                } else if self
                    .current
                    .contains_key(&ObjectKey::RootDirectory(id.clone()))
                {
                    ObjectKey::RootDirectory(id)
                } else {
                    return Err(refuse(
                        "queryScopeUnavailable",
                        "The exact directory scope is unavailable",
                    ));
                }
            } else {
                return Err(refuse(
                    "queryScopeUnavailable",
                    "Semantic revalidation requires an exact stable Query scope",
                ));
            };
            let version = self.need(&target)?;
            super::apply::validate_state(self.graph, &target, &version.state)?;
        }
        weftext_data::required_query_lifecycles(plan, &context.definitions).map_err(|_| {
            refuse(
                "queryScopeUnavailable",
                "Query lifecycle closure is invalid",
            )
        })?;
        Ok(())
    }
    fn validate_node(&mut self, object: &ObjectKey, version: &Version) -> Result<()> {
        self.need(object)?;
        let schema = self.schema()?;
        self.rules()?;
        let (source_revision, source) = self.source(object, version)?;
        let context = self.resource_context(object, version)?;
        if !weftext_data::validate_note(&source, &schema, &context).is_valid() {
            return Err(refuse(
                "invalidTypedSource",
                "Chosen source violates known fields, required inputs or Type constraints",
            ));
        }
        let refs = weftext_data::declared_typed_ref_closure(&source, &schema).map_err(|_| {
            refuse(
                "invalidTypedSource",
                "Declared typed reference closure is unavailable",
            )
        })?;
        if !refs.complete {
            return Err(refuse(
                "invalidTypedSource",
                "A known field has invalid/computed reference controls",
            ));
        }
        for slot in &refs.slots {
            self.typed_target(&slot.kind, &slot.target)?;
        }
        if let Some(declaration) =
            weftext_engine::inspect_citation_declaration(&source).map_err(|_| {
                refuse(
                    "citationDeclarationUnavailable",
                    "Citation declaration is not exact native control",
                )
            })?
        {
            let mut works = vec![];
            for entry in declaration.entries {
                works.push(weftext_export::bibliography::CitationWork {
                    key: entry.key,
                    work: self.work(&entry.work, &schema)?,
                });
            }
            weftext_export::bibliography::project_bibliography(
                &works,
                weftext_export::bibliography::BibliographyFormat::Bibtex,
            )
            .map_err(|_| {
                refuse(
                    "citationPublicationUnavailable",
                    "Chosen publication values cannot produce an accurate bibliography",
                )
            })?;
        }
        self.queries(object, &source_revision, &source, &schema)?;
        if let Some(digest) = version.state.files.get("weftext.annotations.json") {
            let bytes = self.blob(digest)?;
            let annotations = weftext_knowledge::decode_annotations(&bytes)
                .map_err(|_| refuse("annotationUnavailable", "Chosen annotations are malformed"))?;
            let ObjectKey::Node(id) = object else {
                unreachable!()
            };
            for record in &annotations.records {
                self.annotation_targets.push(SyncAnnotationTargetStatus {
                    owner: self.node(id),
                    annotation_id: record.annotation_id.clone(),
                    state: "unavailable".into(),
                    reason: "preserved-awaiting-exact-target-qualification".into(),
                });
                if let Some(anchor) = record.target.anchor()
                    && anchor.node == self.node(id)
                    && version
                        .state
                        .file_revisions
                        .get(&format!("{}.typ", version.state.name))
                        != Some(&anchor.revision_id)
                    && let Err(error) = self.historical(object, &anchor.revision_id)
                    && error.code == "budget"
                {
                    return Err(error);
                }
            }
            self.evidence(RequiredSemanticEvidence::AnnotationTargets {
                owner: self.node(id),
                annotation_file_revision: version.state.file_revisions["weftext.annotations.json"]
                    .clone(),
            })?;
        }
        Ok(())
    }
}

/// Prove semantics from supplied immutable inputs. `required_evidence` is a
/// mandatory writer obligation, not a successful current-author qualification.
#[cfg(test)]
pub(crate) fn validate_revalidation(
    graph: &ReplicaGraph,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
    reviewed: &[SyncRevalidation],
) -> Result<CheckedRevalidation> {
    check_revalidation(graph, chosen, local, reviewed, true, &BTreeSet::new())
}

pub(crate) fn validate_revalidation_with_query_members(
    graph: &ReplicaGraph,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
    reviewed: &[SyncRevalidation],
    query_members: &BTreeSet<ObjectKey>,
) -> Result<CheckedRevalidation> {
    check_revalidation(graph, chosen, local, reviewed, true, query_members)
}

/// Read-only derivation for the review UI. Actual reads are returned from the
/// same production validators; the subsequent prepare must explicitly review
/// these exact versions. This result is never an operation or write authority.
pub(crate) fn inspect_revalidation(
    graph: &ReplicaGraph,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
    reviewed: &[SyncRevalidation],
    query_members: &BTreeSet<ObjectKey>,
) -> Result<CheckedRevalidation> {
    check_revalidation(graph, chosen, local, reviewed, false, query_members)
}

fn check_revalidation(
    graph: &ReplicaGraph,
    chosen: &BTreeMap<ObjectKey, Version>,
    local: &BTreeMap<ObjectKey, LocalObject>,
    reviewed: &[SyncRevalidation],
    enforce_review: bool,
    query_members: &BTreeSet<ObjectKey>,
) -> Result<CheckedRevalidation> {
    if reviewed.is_empty() || reviewed.len() > MAX_CHANGES || chosen.len() > MAX_CHANGES {
        return Err(refuse(
            "budget",
            "Revalidation needs one bounded explicit state cut",
        ));
    }
    validate_proposed_file_revisions(graph, chosen)?;
    let current = graph.objects()?;
    if graph.manifests.iter().any(|(id, manifest)| {
        manifest
            .changes
            .iter()
            .any(|change| chosen.contains_key(&change.object))
            && matches!(graph.status(id), Ok(Admission::Staged { .. }))
    }) {
        return Err(refuse(
            "syncIncomplete",
            "Hydrate the complete reviewed transaction before semantic revalidation",
        ));
    }
    let mut seen = BTreeSet::new();
    for item in reviewed {
        if !seen.insert(item.object.clone())
            || current
                .get(&item.object)
                .is_none_or(|v| revisions(v) != item.revisions)
            || item.revisions.is_empty()
        {
            return Err(refuse(
                "syncConflictChanged",
                "Review the complete received branch set exactly once",
            ));
        }
        let base = current[&item.object]
            .versions
            .iter()
            .find(|v| v.revision == item.decision.base_revision())
            .ok_or_else(|| {
                refuse(
                    "syncConflictChanged",
                    "Review the exact chosen current branch",
                )
            })?;
        if matches!(item.decision, SyncRevalidationDecision::Preserve { .. })
            && base.state != item.state
        {
            return Err(refuse(
                "syncConflictChanged",
                "Preserve cannot modify the selected native state",
            ));
        }
        if chosen
            .get(&item.object)
            .is_none_or(|v| v.state != item.state)
        {
            return Err(refuse(
                "syncConflictChanged",
                "Explicit after state differs from the reviewed proposal",
            ));
        }
    }
    for (object, version) in chosen {
        let view = current.get(object).ok_or_else(|| {
            refuse(
                "syncIncomplete",
                "Chosen object is not in accepted received history",
            )
        })?;
        if !seen.contains(object) && (view.versions.len() != 1 || view.versions[0] != *version) {
            return Err(refuse(
                "syncConflict",
                "Every concurrent or repaired dependency needs its own explicit resolution",
            ));
        }
    }
    let mut validator = Validator {
        graph,
        chosen: chosen.clone(),
        allow_discovery: !enforce_review,
        current,
        local,
        used_blobs: BTreeSet::new(),
        bytes: 0,
        retained_metadata: 0,
        reads: BTreeSet::new(),
        all_reads: BTreeSet::new(),
        historical: BTreeMap::new(),
        evidence: BTreeMap::new(),
        deferred: vec![],
        providers: vec![],
        provider_traces: vec![],
        invocations: vec![],
        query_members,
        annotation_targets: vec![],
    };
    for version in chosen.values() {
        for digest in version.state.files.values() {
            validator.blob(digest)?;
        }
    }
    let mut dependency_reads = BTreeMap::new();
    for item in reviewed {
        validator.reads.clear();
        let version = &chosen[&item.object];
        super::apply::validate_state(graph, &item.object, &version.state)?;
        match &item.object {
            ObjectKey::Node(_) => {
                validator.validate_node(&item.object, version)?;
            }
            ObjectKey::Schema => {
                validator.schema()?;
            }
            ObjectKey::Rules => {
                validator.rules()?;
            }
            _ => {
                validator.need(&item.object)?;
            }
        }
        let mut declared = BTreeSet::new();
        for dep in &item.reviewed_dependencies {
            if !declared.insert(dep.object.clone())
                || validator
                    .current
                    .get(&dep.object)
                    .is_none_or(|v| revisions(v) != dep.revisions)
            {
                return Err(refuse(
                    "syncConflictChanged",
                    "Review exact complete dependency versions before revalidation",
                ));
            }
        }
        validator.reads.remove(&item.object);
        dependency_reads.insert(
            item.object.clone(),
            validator
                .reads
                .iter()
                .map(|object| Dependency {
                    object: object.clone(),
                    revisions: revisions(&validator.current[object]),
                })
                .collect(),
        );
        if enforce_review && validator.reads != declared {
            return Err(refuse(
                "syncDependencyRequired",
                "The reviewed dependency cut differs from the actual semantic read closure",
            ));
        }
    }
    let dependencies = validator
        .all_reads
        .iter()
        .map(|object| Dependency {
            object: object.clone(),
            revisions: revisions(&validator.current[object]),
        })
        .collect();
    Ok(CheckedRevalidation {
        states: validator.chosen,
        dependencies,
        historical_inputs: validator.historical.into_values().collect(),
        required_evidence: validator.evidence.into_values().collect(),
        deferred_queries: validator.deferred,
        provider_requirements: validator.providers,
        provider_dependencies: validator.provider_traces,
        dependency_reads,
        invocations: validator.invocations,
        annotation_targets: validator.annotation_targets,
    })
}

#[cfg(test)]
#[path = "revalidation_tests.rs"]
mod tests;
