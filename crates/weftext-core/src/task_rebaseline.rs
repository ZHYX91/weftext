use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::ser::SerializeStruct as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use weftext_asciidoc::{
    AnalysisStatus, ChecklistEvidence, ChecklistMarker, SourceEdit, SourceEditPlan,
    checklist_promotion_principal_context_dependencies, encode_node_link_label,
};

use crate::source_lexing::{
    decode_attribute_value, find_unquoted_equals, split_comma_parts, trim_range,
};
use crate::{
    AnnotationReplicaCompleteness, DocumentRevision, InventoryIssueCode, NodeId, TaskDateTime,
    TaskDiagnosticCode, TaskId, TaskNodePriority, TaskNodeProfile, TaskNodeProfileVersion,
    TaskNodeState, TaskNodeTemporal, TaskOccurrence, TaskPhase, TaskPriority, TaskResolution,
    WorkspaceDocumentGeneration, WorkspaceRevision, analyze_task_node_profile, analyze_task_source,
    parse_node_metadata, read_node_document, read_workspace_revision, scan_workspace,
    suggest_portable_node_name,
};

pub const TASK_REBASELINE_SCHEMA: &str = "weftext.task-rebaseline/v1";
pub const TASK_REBASELINE_MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const TASK_REBASELINE_MAX_TOTAL_SOURCE_BYTES: usize = 512 * 1024 * 1024;
pub const TASK_REBASELINE_MAX_DOCUMENTS: usize = 100_000;
pub const TASK_REBASELINE_MAX_OCCURRENCES: usize = 100_000;
pub const TASK_REBASELINE_MAX_QUERIES: usize = 100_000;
pub const TASK_REBASELINE_MAX_BLOCKERS: usize = 10_000;
pub const TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES: usize = 4 * 1024 * 1024;
pub const TASK_REBASELINE_MAX_TOTAL_PREVIEW_BYTES: usize = 8 * 1024 * 1024;
pub const TASK_REBASELINE_MAX_PLAN_JSON_BYTES: usize = 16 * 1024 * 1024;

const MAX_TITLE_BYTES: usize = 4_096;
const MAX_LINK_LABEL_BYTES: usize = 4_096;
const MAX_DESTINATION_CONTENT_RULE_MATCH_WORK: usize = 10_000_000;
const MAX_LEGACY_ANALYSIS_SCAN_WORK: usize = 64 * 1024 * 1024;
const MAX_LEGACY_METADATA_SEPARATOR_WORK: usize = 10_000;
const PLAN_DIGEST_DOMAIN: &[u8] = b"weftext.task-rebaseline.plan/v1\0";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineScope {
    OwnerWorkspace,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselinePhysicalInventoryBinding {
    WorkspaceRevisionOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineExternalSnapshotBinding {
    NotProvided,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselinePortableInventoryBinding {
    FullOwnerWorkspace,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselinePreStateBinding {
    pub workspace_revision: WorkspaceRevision,
    pub portable_inventory_binding: TaskRebaselinePortableInventoryBinding,
    pub physical_inventory_binding: TaskRebaselinePhysicalInventoryBinding,
    pub external_snapshot_binding: TaskRebaselineExternalSnapshotBinding,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineBlockerCode {
    InvalidLegacyTask,
    MalformedMacroResidue,
    ParserAlignmentUnproven,
    IncompleteStructuredBranch,
    DuplicateLegacyTaskId,
    RecurrenceUnsupported,
    UnresolvedDependency,
    AmbiguousDependency,
    SelfDependency,
    DependencyCycle,
    RelativeLocator,
    DocumentContextDependency,
    NestedStructuredBranchOverlap,
    DestinationNameUnavailable,
    DestinationContentBoundary,
    DestinationCollision,
    AnnotationReplicaIncomplete,
    AnnotationAuthorityInvalid,
    AnnotationMigrationRequired,
    TrashRestoreRequired,
    QueryPopulationEquivalenceUnproven,
    InvalidTaskQuery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineBlocker {
    pub code: TaskRebaselineBlockerCode,
    pub source_node_id: Option<NodeId>,
    pub old_task_id: Option<TaskId>,
    pub dependency_task_id: Option<TaskId>,
    pub range: Option<Range<u64>>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineSourceKind {
    ActiveManagedDocument,
    TrashPayloadDocument,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineOccurrenceDisposition {
    ProposedTaskNode,
    Blocked,
    ProtectedLiteral,
    TrashRestoreRequired,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineOccurrenceInventory {
    pub source_kind: TaskRebaselineSourceKind,
    pub source_node_id: NodeId,
    pub document_revision: DocumentRevision,
    pub document_locator: String,
    pub macro_range: Range<u64>,
    pub item_range: Option<Range<u64>>,
    pub marker_range: Option<Range<u64>>,
    pub description_range: Option<Range<u64>>,
    pub raw_macro: String,
    pub raw_item: String,
    pub old_task_id: Option<TaskId>,
    pub generated_node_id: Option<NodeId>,
    pub disposition: TaskRebaselineOccurrenceDisposition,
    pub blocker_codes: Vec<TaskRebaselineBlockerCode>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineIdentityMapping {
    pub source_node_id: NodeId,
    pub old_task_id: TaskId,
    pub generated_node_id: NodeId,
    pub destination_parent_node_id: NodeId,
    pub destination_node_locator: String,
    pub destination_portable_name: String,
    pub document_title: String,
    pub link_label: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineTaskFields {
    pub state: TaskNodeState,
    pub priority: Option<TaskNodePriority>,
    pub created: Option<String>,
    pub start: Option<String>,
    pub scheduled: Option<String>,
    pub due: Option<String>,
    pub closed: Option<String>,
    pub depends_on: Vec<NodeId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineProposal {
    pub old_task_id: TaskId,
    pub generated_node_id: NodeId,
    pub fields: TaskRebaselineTaskFields,
    pub source_replacement_range: Range<u64>,
    pub expected_source: String,
    pub replacement_source: String,
    pub proposed_task_source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum TaskRebaselineAnnotationInventory {
    Present {
        sha256: String,
        annotation_count: u64,
    },
    ConfirmedAbsent,
    Unproven,
    Invalid,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineSourcePreview {
    pub source_node_id: NodeId,
    pub document_revision: DocumentRevision,
    pub document_locator: String,
    pub original_source: String,
    pub proposed_source: String,
    pub annotations: TaskRebaselineAnnotationInventory,
    pub proposals: Vec<TaskRebaselineProposal>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRebaselineQueryDisposition {
    CanonicalUnchanged,
    ConversionBlocked,
    InvalidBlocked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselineQueryInventory {
    pub source_node_id: NodeId,
    pub document_revision: DocumentRevision,
    pub document_locator: String,
    pub range: Range<u64>,
    pub body_range: Range<u64>,
    pub raw_source: String,
    pub disposition: TaskRebaselineQueryDisposition,
}

/// Closed, path-safe Owner preview. It intentionally contains no commit token, journal steps,
/// absolute root, scoped projection, or external-snapshot claim.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskRebaselinePlan {
    pub schema: String,
    pub scope: TaskRebaselineScope,
    pub preview_only: bool,
    pub committable: bool,
    pub conversion_ready: bool,
    pub base_workspace_revision: WorkspaceRevision,
    pub pre_state: TaskRebaselinePreStateBinding,
    pub annotation_replica_completeness: AnnotationReplicaCompleteness,
    pub occurrences: Vec<TaskRebaselineOccurrenceInventory>,
    pub identity_map: Vec<TaskRebaselineIdentityMapping>,
    pub source_previews: Vec<TaskRebaselineSourcePreview>,
    pub queries: Vec<TaskRebaselineQueryInventory>,
    pub blockers: Vec<TaskRebaselineBlocker>,
    pub plan_digest: String,
}

impl TaskRebaselinePlan {
    #[must_use]
    pub fn conversion_ready(&self) -> bool {
        self.conversion_ready
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskRebaselineError {
    InvalidWorkspace(InventoryIssueCode),
    UnsupportedGeneration(WorkspaceDocumentGeneration),
    WorkspaceRevisionUnavailable,
    DocumentRead(NodeId),
    InvalidDocumentIdentity(NodeId),
    ResourceLimitExceeded(&'static str),
    StaleWorkspaceRevision,
    InvalidReviewedPlan,
    GeneratedIdentityExhausted,
}

impl fmt::Display for TaskRebaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::UnsupportedGeneration(generation) => {
                write!(
                    formatter,
                    "task rebaseline requires AsciiDoc v1, got {generation:?}"
                )
            }
            Self::WorkspaceRevisionUnavailable => {
                formatter.write_str("task rebaseline workspace revision is unavailable")
            }
            Self::DocumentRead(node_id) => {
                write!(formatter, "task rebaseline could not read node {node_id}")
            }
            Self::InvalidDocumentIdentity(node_id) => {
                write!(
                    formatter,
                    "task rebaseline document identity differs for node {node_id}"
                )
            }
            Self::ResourceLimitExceeded(limit) => {
                write!(formatter, "task rebaseline exceeded the {limit} limit")
            }
            Self::StaleWorkspaceRevision => {
                formatter.write_str("task rebaseline workspace changed after review")
            }
            Self::InvalidReviewedPlan => {
                formatter.write_str("task rebaseline reviewed plan is invalid or was modified")
            }
            Self::GeneratedIdentityExhausted => {
                formatter.write_str("task rebaseline could not allocate a fresh node identity")
            }
        }
    }
}

impl std::error::Error for TaskRebaselineError {}

/// An opaque, revision-bound capture of one complete local filesystem workspace view.
///
/// It does not prove ACL identity or grant Owner permission. A local caller must establish that
/// authorization before capture. Hosted callers require a future backend-signed authority and
/// cannot construct hosted completeness through this type. It is intentionally neither
/// serializable nor constructible from fields, and its redacted `Debug` never exposes the path.
pub struct LocalTaskRebaselineAuthority {
    pub(crate) root: PathBuf,
    pub(crate) workspace_revision: WorkspaceRevision,
    pub(crate) annotation_replica_completeness: AnnotationReplicaCompleteness,
}

impl fmt::Debug for LocalTaskRebaselineAuthority {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalTaskRebaselineAuthority")
            .field("workspace_revision", &self.workspace_revision)
            .field(
                "annotation_replica_completeness",
                &self.annotation_replica_completeness,
            )
            .finish_non_exhaustive()
    }
}

/// Captures a complete local workspace view without changing any workspace bytes.
///
/// Filesystem access is the only capability checked here. The caller remains responsible for ACL
/// and Owner authorization; this entry point never constructs hosted-replica completeness.
///
/// # Errors
///
/// Returns a typed error when the workspace is unavailable, invalid, not `AsciiDoc` v1, or has no
/// readable portable revision.
pub fn capture_local_task_rebaseline_authority(
    root: impl AsRef<Path>,
) -> Result<LocalTaskRebaselineAuthority, TaskRebaselineError> {
    capture_task_rebaseline_owner_authority(
        root.as_ref(),
        AnnotationReplicaCompleteness::CompleteLocalWorkspace,
    )
}

fn capture_task_rebaseline_owner_authority(
    root: &Path,
    annotation_replica_completeness: AnnotationReplicaCompleteness,
) -> Result<LocalTaskRebaselineAuthority, TaskRebaselineError> {
    let root =
        fs::canonicalize(root).map_err(|_| TaskRebaselineError::WorkspaceRevisionUnavailable)?;
    let inventory = scan_workspace(&root);
    if !inventory.is_valid() {
        return Err(TaskRebaselineError::InvalidWorkspace(
            inventory
                .issues
                .first()
                .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
        ));
    }
    if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        return Err(TaskRebaselineError::UnsupportedGeneration(
            inventory.generation,
        ));
    }
    let workspace_revision = read_workspace_revision(&root)
        .map_err(|_| TaskRebaselineError::WorkspaceRevisionUnavailable)?;
    Ok(LocalTaskRebaselineAuthority {
        root,
        workspace_revision,
        annotation_replica_completeness,
    })
}

/// Inventories and previews the complete local workspace for an already-authorized Owner caller.
///
/// There is deliberately no scoped counterpart and no alternate-parent request in package 1.
/// This local entry point accepts only a complete local filesystem capture. Hosted, partial, and
/// unknown replicas require a future backend authority contract and cannot call this planner.
///
/// # Errors
///
/// Returns a typed error for stale/invalid authority, unreadable documents, or resource ceilings.
pub fn plan_task_rebaseline(
    authority: &LocalTaskRebaselineAuthority,
) -> Result<TaskRebaselinePlan, TaskRebaselineError> {
    plan_internal(authority, None)
}

/// Rebuilds a reviewed preview from current portable authority while reusing exactly the reviewed
/// generated node IDs. A changed workspace, altered DTO, occupied ID, or changed preview fails.
///
/// # Errors
///
/// Returns a typed error when authority is stale or any reviewed field, evidence, mapping, limit,
/// or regenerated preview differs.
pub fn revalidate_task_rebaseline_plan(
    authority: &LocalTaskRebaselineAuthority,
    reviewed: &TaskRebaselinePlan,
) -> Result<TaskRebaselinePlan, TaskRebaselineError> {
    validate_reviewed_shape(reviewed, authority.annotation_replica_completeness)?;
    let fresh = plan_internal(authority, Some(reviewed))?;
    if &fresh == reviewed {
        Ok(fresh)
    } else {
        Err(TaskRebaselineError::InvalidReviewedPlan)
    }
}

/// Validation-only counterpart for callers that do not need the rebuilt value.
///
/// # Errors
///
/// Returns the same typed validation and staleness errors as revalidation.
pub fn validate_task_rebaseline_plan(
    authority: &LocalTaskRebaselineAuthority,
    reviewed: &TaskRebaselinePlan,
) -> Result<(), TaskRebaselineError> {
    revalidate_task_rebaseline_plan(authority, reviewed).map(drop)
}

/// Decodes one bounded, closed task-rebaseline preview without granting workspace authority.
///
/// This first-stage decoder rejects oversized input, duplicate JSON keys, non-canonical `UUIDv4`
/// text, unknown fields, resource-shape violations, and a changed digest. A caller must still pass
/// the result to [`validate_task_rebaseline_plan`] with a current local authority before relying on
/// its workspace evidence.
///
/// # Errors
///
/// Returns a typed error for oversized, malformed, duplicate-key, structurally invalid, or
/// tampered JSON.
pub fn decode_task_rebaseline_plan_json(
    bytes: &[u8],
) -> Result<TaskRebaselinePlan, TaskRebaselineError> {
    if bytes.len() > TASK_REBASELINE_MAX_PLAN_JSON_BYTES {
        return Err(TaskRebaselineError::ResourceLimitExceeded(
            "reviewed plan JSON bytes",
        ));
    }
    crate::workspace_transaction::reject_duplicate_json_keys(bytes)
        .map_err(|_| TaskRebaselineError::InvalidReviewedPlan)?;
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| TaskRebaselineError::InvalidReviewedPlan)?;
    validate_canonical_uuid_json_fields(&value)?;
    let plan: TaskRebaselinePlan =
        serde_json::from_value(value).map_err(|_| TaskRebaselineError::InvalidReviewedPlan)?;
    validate_reviewed_shape(&plan, plan.annotation_replica_completeness)?;
    Ok(plan)
}

#[derive(Clone)]
struct RawMacro {
    range: Range<u64>,
    item_range: Range<u64>,
    protected: bool,
}

struct ActiveDocument {
    node_id: NodeId,
    node_directory: PathBuf,
    locator: String,
    source: String,
    revision: DocumentRevision,
    raw_macros: Vec<RawMacro>,
    annotation_inventory: TaskRebaselineAnnotationInventory,
    candidate_by_raw: Vec<Option<usize>>,
    raw_old_task_ids: Vec<Option<TaskId>>,
    raw_blocker_codes: Vec<Vec<TaskRebaselineBlockerCode>>,
}

struct Candidate {
    document: usize,
    task: TaskOccurrence,
    checklist: Option<ChecklistEvidence>,
    old_task_id: TaskId,
    title: String,
    name: Option<String>,
    generated_node_id: Option<NodeId>,
    destination_locator: Option<String>,
    blockers: BTreeSet<TaskRebaselineBlockerCode>,
}

struct LegacyAnalysisBudget {
    remaining_inventory_occurrences: usize,
    lexical_macro_starts: usize,
    scan_work: usize,
    metadata_separator_work: usize,
}

impl LegacyAnalysisBudget {
    const fn new() -> Self {
        Self {
            remaining_inventory_occurrences: TASK_REBASELINE_MAX_OCCURRENCES,
            lexical_macro_starts: 0,
            scan_work: 0,
            metadata_separator_work: 0,
        }
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "the global planner keeps one auditable workspace-wide orchestration order"
)]
pub(crate) fn plan_internal(
    authority: &LocalTaskRebaselineAuthority,
    reviewed: Option<&TaskRebaselinePlan>,
) -> Result<TaskRebaselinePlan, TaskRebaselineError> {
    let root = authority.root.as_path();
    let annotation_replica_completeness = authority.annotation_replica_completeness;
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(TaskRebaselineError::InvalidWorkspace(
            inventory
                .issues
                .first()
                .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
        ));
    }
    if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        return Err(TaskRebaselineError::UnsupportedGeneration(
            inventory.generation,
        ));
    }
    let content_rules = crate::content_boundary::ContentRules::load(root).map_err(|_| {
        TaskRebaselineError::InvalidWorkspace(InventoryIssueCode::InvalidContentRules)
    })?;
    if inventory.nodes.len() > TASK_REBASELINE_MAX_DOCUMENTS {
        return Err(TaskRebaselineError::ResourceLimitExceeded("document count"));
    }
    let base_revision = read_workspace_revision(root)
        .map_err(|_| TaskRebaselineError::WorkspaceRevisionUnavailable)?;
    if base_revision != authority.workspace_revision {
        return Err(TaskRebaselineError::StaleWorkspaceRevision);
    }
    if reviewed.is_some_and(|plan| plan.base_workspace_revision != base_revision) {
        return Err(TaskRebaselineError::StaleWorkspaceRevision);
    }

    let mut active_nodes = inventory
        .nodes
        .iter()
        .filter(|node| {
            !crate::workspace_trash::is_trash_storage_path(root, &node.path)
                && node.name != crate::TRASH_NODE_NAME
        })
        .collect::<Vec<_>>();
    active_nodes.sort_by_key(|node| relative_locator(root, &node.document_path));

    let mut documents = Vec::new();
    let mut candidates = Vec::new();
    let mut blockers = Vec::new();
    let mut total_source_bytes = 0_usize;
    let mut total_evidence_bytes = 0_usize;
    let mut legacy_analysis_budget = LegacyAnalysisBudget::new();
    for node in active_nodes {
        let node_id = node.id.ok_or(TaskRebaselineError::InvalidWorkspace(
            InventoryIssueCode::MissingIdentity,
        ))?;
        let snapshot = read_node_document(&node.path)
            .map_err(|_| TaskRebaselineError::DocumentRead(node_id))?;
        if snapshot.node_id != node_id || snapshot.node_directory != node.path {
            return Err(TaskRebaselineError::InvalidDocumentIdentity(node_id));
        }
        if snapshot.source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES {
            return Err(TaskRebaselineError::ResourceLimitExceeded("document bytes"));
        }
        total_source_bytes = total_source_bytes.saturating_add(snapshot.source.len());
        if total_source_bytes > TASK_REBASELINE_MAX_TOTAL_SOURCE_BYTES {
            return Err(TaskRebaselineError::ResourceLimitExceeded(
                "total source bytes",
            ));
        }
        preflight_legacy_analysis_work(&snapshot.source, &mut legacy_analysis_budget)?;
        let locator = relative_locator(root, &node.document_path);
        let mut raw_macros = raw_macros(
            &snapshot.source,
            legacy_analysis_budget.remaining_inventory_occurrences,
        )?;
        legacy_analysis_budget.remaining_inventory_occurrences = legacy_analysis_budget
            .remaining_inventory_occurrences
            .saturating_sub(raw_macros.len());
        preflight_occurrence_evidence(&locator, &raw_macros, &mut total_evidence_bytes)?;
        let parser = weftext_asciidoc::analyze(&snapshot.source);
        let parser_failed = parser.status == AnalysisStatus::Failed;
        if !parser_failed {
            mark_protected_raw_macros(&mut raw_macros, &parser.protected_ranges);
        }
        let checklist_by_range = checklist_evidence_by_range(&parser.checklists);
        let legacy = analyze_task_source(&snapshot.source);
        let legacy_by_macro_range = legacy_tasks_by_macro_range(&legacy);
        let diagnostic_ranges = DiagnosticRangeIndex::new(&legacy.diagnostics);
        let document_index = documents.len();
        let mut candidate_by_raw = vec![None; raw_macros.len()];
        let mut raw_old_task_ids = vec![None; raw_macros.len()];
        let mut raw_blocker_codes = vec![Vec::new(); raw_macros.len()];
        if parser_failed {
            push_blocker(
                &mut blockers,
                blocker(
                    TaskRebaselineBlockerCode::ParserAlignmentUnproven,
                    Some(node_id),
                    None,
                    None,
                    Some(0..u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX)),
                    "canonical AsciiDoc analysis failed for the complete source document",
                ),
            )?;
        }
        for (raw_index, raw) in raw_macros.iter().enumerate() {
            raw_old_task_ids[raw_index] =
                crate::task::legacy_task_id_from_closed_macro(&snapshot.source, &raw.range);
            if parser_failed {
                raw_blocker_codes[raw_index]
                    .push(TaskRebaselineBlockerCode::ParserAlignmentUnproven);
                continue;
            }
            if raw.protected {
                continue;
            }
            let matching = legacy_by_macro_range
                .get(&(raw.range.start, raw.range.end))
                .map_or(&[][..], Vec::as_slice);
            let [task_index] = matching else {
                let code = diagnostic_ranges.code_for(&raw.item_range);
                push_blocker(
                    &mut blockers,
                    blocker(
                        code,
                        Some(node_id),
                        None,
                        None,
                        Some(raw.range.clone()),
                        blocker_message(code),
                    ),
                )?;
                raw_blocker_codes[raw_index].push(code);
                continue;
            };
            let task = &legacy.tasks[*task_index];
            let metadata = task.metadata.as_ref().expect("matching metadata");
            let mut candidate = Candidate {
                document: document_index,
                task: task.clone(),
                checklist: aligned_checklist_evidence(
                    &parser.checklists,
                    &checklist_by_range,
                    task,
                    &metadata.range,
                ),
                old_task_id: metadata.id,
                title: task.description.clone(),
                name: suggest_portable_node_name(&task.description),
                generated_node_id: None,
                destination_locator: None,
                blockers: BTreeSet::new(),
            };
            if !task.valid {
                add_candidate_blocker(
                    &mut candidate,
                    diagnostic_ranges.code_for(&task.range),
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    metadata.range.clone(),
                )?;
            }
            if candidate.checklist.is_none() {
                add_candidate_blocker(
                    &mut candidate,
                    TaskRebaselineBlockerCode::ParserAlignmentUnproven,
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    metadata.range.clone(),
                )?;
            }
            if candidate
                .checklist
                .as_ref()
                .is_some_and(|checklist| checklist.parser_occurrence.promotion_branch.is_none())
            {
                add_candidate_blocker(
                    &mut candidate,
                    TaskRebaselineBlockerCode::IncompleteStructuredBranch,
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    task.range.clone(),
                )?;
            }
            if metadata.recurrence.is_some() || metadata.repeat_from.is_some() {
                add_candidate_blocker(
                    &mut candidate,
                    TaskRebaselineBlockerCode::RecurrenceUnsupported,
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    metadata.range.clone(),
                )?;
            }
            if !valid_reviewed_text(&candidate.title, MAX_TITLE_BYTES, false)
                || encode_node_link_label(&candidate.title).is_err()
                || candidate.title.len() > MAX_LINK_LABEL_BYTES
                || candidate.name.is_none()
            {
                add_candidate_blocker(
                    &mut candidate,
                    TaskRebaselineBlockerCode::DestinationNameUnavailable,
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    task.description_range.clone(),
                )?;
            }
            let mut context_dependencies = candidate
                .checklist
                .as_ref()
                .and_then(|checklist| checklist.parser_occurrence.promotion_branch.as_ref())
                .map(|promotion| promotion.context_dependencies.clone())
                .unwrap_or_default();
            match checklist_promotion_principal_context_dependencies(
                &snapshot.source,
                task.description_range.clone(),
            ) {
                Some(principal_dependencies) => {
                    context_dependencies.extend(principal_dependencies);
                }
                None => {
                    add_candidate_blocker(
                        &mut candidate,
                        TaskRebaselineBlockerCode::ParserAlignmentUnproven,
                        &mut blockers,
                        node_id,
                        Some(metadata.id),
                        task.description_range.clone(),
                    )?;
                }
            }
            for dependency in context_dependencies {
                let code = if dependency.kind
                    == weftext_asciidoc::ChecklistPromotionContextDependencyKind::RelativeLocator
                {
                    TaskRebaselineBlockerCode::RelativeLocator
                } else {
                    TaskRebaselineBlockerCode::DocumentContextDependency
                };
                add_candidate_blocker(
                    &mut candidate,
                    code,
                    &mut blockers,
                    node_id,
                    Some(metadata.id),
                    dependency.range,
                )?;
            }
            let candidate_index = candidates.len();
            candidates.push(candidate);
            candidate_by_raw[raw_index] = Some(candidate_index);
        }
        documents.push(ActiveDocument {
            node_id,
            node_directory: node.path.clone(),
            locator,
            source: snapshot.source,
            revision: snapshot.revision,
            raw_macros,
            annotation_inventory: TaskRebaselineAnnotationInventory::Unproven,
            candidate_by_raw,
            raw_old_task_ids,
            raw_blocker_codes,
        });
    }

    diagnose_duplicate_ids(&mut candidates, &documents, &mut blockers)?;
    diagnose_nested_overlaps(&mut candidates, &documents, &mut blockers)?;

    let mut trash_occurrences = Vec::new();
    let mut inventoried_document_count = documents.len();
    let trash_old_task_ids = inventory_trash_occurrences(
        &inventory,
        &mut trash_occurrences,
        &mut blockers,
        &mut total_source_bytes,
        &mut legacy_analysis_budget,
        &mut inventoried_document_count,
        &mut total_evidence_bytes,
    )?;

    let mut occupied_ids = inventory
        .nodes
        .iter()
        .filter_map(|node| node.id)
        .map(|id| id.to_string())
        .collect::<BTreeSet<_>>();
    for item in &inventory.trash_items {
        occupied_ids.extend(item.node_locators.keys().map(ToString::to_string));
    }
    let all_old_ids = candidates
        .iter()
        .map(|candidate| candidate.old_task_id.to_string())
        .chain(
            documents
                .iter()
                .flat_map(|document| document.raw_old_task_ids.iter().flatten())
                .map(ToString::to_string),
        )
        .collect::<BTreeSet<_>>();
    occupied_ids.extend(all_old_ids.iter().cloned());
    occupied_ids.extend(
        trash_old_task_ids
            .into_iter()
            .map(|task_id| task_id.to_string()),
    );
    let reviewed_ids = reviewed_identity_map(reviewed)?;
    assign_mappings(
        root,
        &content_rules,
        &mut candidates,
        &documents,
        &mut blockers,
        &mut occupied_ids,
        reviewed_ids.as_ref(),
    )?;

    inventory_annotations(&mut documents, &mut candidates, &mut blockers)?;
    diagnose_dependencies(&mut candidates, &documents, &mut blockers)?;

    let mut source_previews = build_source_previews(&documents, &mut candidates, &mut blockers)?;
    source_previews.sort_by(|left, right| left.document_locator.cmp(&right.document_locator));
    let mut occurrences = active_occurrence_inventory(&documents, &candidates);
    occurrences.extend(trash_occurrences);
    if occurrences.len() > TASK_REBASELINE_MAX_OCCURRENCES {
        return Err(TaskRebaselineError::ResourceLimitExceeded(
            "task occurrence count",
        ));
    }
    let queries = inventory_queries(&documents, &mut blockers)?;
    if queries.len() > TASK_REBASELINE_MAX_QUERIES {
        return Err(TaskRebaselineError::ResourceLimitExceeded("query count"));
    }

    let mut identity_map = candidates
        .iter()
        .filter_map(|candidate| candidate_mapping(candidate, &documents))
        .collect::<Vec<_>>();
    identity_map.sort_by(|left, right| {
        left.old_task_id
            .cmp(&right.old_task_id)
            .then_with(|| left.source_node_id.cmp(&right.source_node_id))
    });
    occurrences.sort_by(|left, right| {
        left.document_locator
            .cmp(&right.document_locator)
            .then_with(|| left.macro_range.start.cmp(&right.macro_range.start))
    });
    sort_blockers(&mut blockers);
    if blockers.len() > TASK_REBASELINE_MAX_BLOCKERS {
        return Err(TaskRebaselineError::ResourceLimitExceeded("blocker count"));
    }
    let closing_revision = read_workspace_revision(root)
        .map_err(|_| TaskRebaselineError::WorkspaceRevisionUnavailable)?;
    if closing_revision != base_revision || closing_revision != authority.workspace_revision {
        return Err(TaskRebaselineError::StaleWorkspaceRevision);
    }
    let conversion_ready = blockers.is_empty();
    let pre_state = TaskRebaselinePreStateBinding {
        workspace_revision: base_revision.clone(),
        portable_inventory_binding: TaskRebaselinePortableInventoryBinding::FullOwnerWorkspace,
        physical_inventory_binding: TaskRebaselinePhysicalInventoryBinding::WorkspaceRevisionOnly,
        external_snapshot_binding: TaskRebaselineExternalSnapshotBinding::NotProvided,
    };
    let mut plan = TaskRebaselinePlan {
        schema: TASK_REBASELINE_SCHEMA.to_owned(),
        scope: TaskRebaselineScope::OwnerWorkspace,
        preview_only: true,
        committable: false,
        conversion_ready,
        base_workspace_revision: base_revision,
        pre_state,
        annotation_replica_completeness,
        occurrences,
        identity_map,
        source_previews,
        queries,
        blockers,
        plan_digest: String::new(),
    };
    ensure_plan_json_budget(&plan)?;
    validate_reviewed_resource_shape(&plan)?;
    plan.plan_digest = plan_digest(&plan)?;
    Ok(plan)
}

fn legacy_tasks_by_macro_range(
    analysis: &crate::TaskSourceAnalysis,
) -> BTreeMap<(u64, u64), Vec<usize>> {
    let mut by_range = BTreeMap::new();
    for (index, task) in analysis.tasks.iter().enumerate() {
        if let Some(metadata) = &task.metadata {
            by_range
                .entry((metadata.range.start, metadata.range.end))
                .or_insert_with(Vec::new)
                .push(index);
        }
    }
    by_range
}

fn preflight_legacy_analysis_work(
    source: &str,
    budget: &mut LegacyAnalysisBudget,
) -> Result<(), TaskRebaselineError> {
    let bytes = source.as_bytes();
    let mut line_start = 0_usize;
    while line_start < source.len() {
        let line_end = crate::source_lexing::line_end(source, line_start, source.len());
        let mut commas_to_right = 0_usize;
        for index in (line_start..line_end).rev() {
            if bytes[index] == b',' {
                commas_to_right = commas_to_right.saturating_add(1);
            }
            if bytes.get(index..index.saturating_add("task:[".len())) != Some(b"task:[") {
                continue;
            }
            budget.lexical_macro_starts = budget.lexical_macro_starts.checked_add(1).ok_or(
                TaskRebaselineError::ResourceLimitExceeded("legacy lexical task start count"),
            )?;
            if budget.lexical_macro_starts > TASK_REBASELINE_MAX_OCCURRENCES {
                return Err(TaskRebaselineError::ResourceLimitExceeded(
                    "legacy lexical task start count",
                ));
            }
            budget.scan_work = budget.scan_work.checked_add(line_end - index).ok_or(
                TaskRebaselineError::ResourceLimitExceeded("legacy task analysis scan work"),
            )?;
            if budget.scan_work > MAX_LEGACY_ANALYSIS_SCAN_WORK {
                return Err(TaskRebaselineError::ResourceLimitExceeded(
                    "legacy task analysis scan work",
                ));
            }
            budget.metadata_separator_work = budget
                .metadata_separator_work
                .checked_add(commas_to_right)
                .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                    "legacy task metadata separator work",
                ))?;
            if budget.metadata_separator_work > MAX_LEGACY_METADATA_SEPARATOR_WORK {
                return Err(TaskRebaselineError::ResourceLimitExceeded(
                    "legacy task metadata separator work",
                ));
            }
        }
        line_start = next_physical_line_start(source, line_end);
    }
    Ok(())
}

fn raw_macros(source: &str, maximum_count: usize) -> Result<Vec<RawMacro>, TaskRebaselineError> {
    let mut macros = Vec::new();
    let mut line_start = 0_usize;
    while line_start < source.len() {
        let physical_end = crate::source_lexing::line_end(source, line_start, source.len());
        let mut line_end = physical_end;
        while line_end > line_start && matches!(source.as_bytes()[line_end - 1], b' ' | b'\t') {
            line_end -= 1;
        }
        let mut cursor = line_start;
        while cursor < line_end {
            let Some(relative) = source[cursor..line_end].find("task:[") else {
                break;
            };
            let start = cursor + relative;
            let open = start + "task:".len();
            let end = crate::source_lexing::find_closing_bracket(source, open, line_end)
                .map_or(line_end, |close| close + 1);
            if macros.len() == maximum_count {
                return Err(TaskRebaselineError::ResourceLimitExceeded(
                    "task occurrence count",
                ));
            }
            macros.push(RawMacro {
                range: to_u64_range(start..end),
                item_range: to_u64_range(line_start..line_end),
                protected: false,
            });
            cursor = end;
        }
        line_start = next_physical_line_start(source, physical_end);
    }
    Ok(macros)
}

fn mark_protected_raw_macros(macros: &mut [RawMacro], protected: &[Range<u64>]) {
    for raw in macros {
        if let Ok(start) = usize::try_from(raw.range.start) {
            raw.protected = contains_offset(protected, start);
        }
    }
}

fn next_physical_line_start(source: &str, line_end: usize) -> usize {
    let bytes = source.as_bytes();
    match bytes.get(line_end) {
        Some(b'\r') if bytes.get(line_end + 1) == Some(&b'\n') => line_end + 2,
        Some(b'\r' | b'\n') => line_end + 1,
        _ => source.len(),
    }
}

fn checklist_matches_legacy(
    checklist: &ChecklistEvidence,
    task: &TaskOccurrence,
    metadata_range: Range<u64>,
) -> bool {
    checklist.item_range == task.range
        && checklist.marker_range.start.checked_add(1) == Some(task.marker_range.start)
        && task.marker_range.end.checked_add(1) == Some(checklist.marker_range.end)
        && u16::from(checklist.list_depth) == task.list_depth
        && checklist_state(checklist.authored_marker) == task.state
        && checklist.description_range.start == task.description_range.start
        && metadata_range.start >= checklist.description_range.start
        && metadata_range.end <= checklist.description_range.end
}

type ChecklistEvidenceKey = (u64, u64, u64, u64, u16, u8, u64);

fn checklist_evidence_by_range(
    checklists: &[ChecklistEvidence],
) -> BTreeMap<ChecklistEvidenceKey, Vec<usize>> {
    let mut by_range = BTreeMap::new();
    for (index, checklist) in checklists.iter().enumerate() {
        let Some(key) = checklist_evidence_key(checklist) else {
            continue;
        };
        by_range.entry(key).or_insert_with(Vec::new).push(index);
    }
    by_range
}

fn checklist_evidence_key(checklist: &ChecklistEvidence) -> Option<ChecklistEvidenceKey> {
    Some((
        checklist.item_range.start,
        checklist.item_range.end,
        checklist.marker_range.start.checked_add(1)?,
        checklist.marker_range.end.checked_sub(1)?,
        u16::from(checklist.list_depth),
        checklist_state_key(checklist.authored_marker),
        checklist.description_range.start,
    ))
}

fn task_checklist_evidence_key(task: &TaskOccurrence) -> ChecklistEvidenceKey {
    (
        task.range.start,
        task.range.end,
        task.marker_range.start,
        task.marker_range.end,
        task.list_depth,
        task_state_key(task.state),
        task.description_range.start,
    )
}

fn aligned_checklist_evidence(
    checklists: &[ChecklistEvidence],
    by_range: &BTreeMap<ChecklistEvidenceKey, Vec<usize>>,
    task: &TaskOccurrence,
    metadata_range: &Range<u64>,
) -> Option<ChecklistEvidence> {
    let indexes = by_range.get(&task_checklist_evidence_key(task))?;
    let mut matching = indexes.iter().filter_map(|index| {
        let checklist = checklists.get(*index)?;
        checklist_matches_legacy(checklist, task, metadata_range.clone()).then_some(checklist)
    });
    let checklist = matching.next()?;
    matching.next().is_none().then(|| checklist.clone())
}

const fn checklist_state_key(marker: ChecklistMarker) -> u8 {
    match marker {
        ChecklistMarker::Open => 0,
        ChecklistMarker::CheckedX | ChecklistMarker::CheckedStar => 1,
    }
}

const fn task_state_key(state: crate::TaskState) -> u8 {
    match state {
        crate::TaskState::Open => 0,
        crate::TaskState::Closed => 1,
    }
}

const fn checklist_state(marker: ChecklistMarker) -> crate::TaskState {
    match marker {
        ChecklistMarker::Open => crate::TaskState::Open,
        ChecklistMarker::CheckedX | ChecklistMarker::CheckedStar => crate::TaskState::Closed,
    }
}

struct DiagnosticRangeIndex {
    self_dependency: OverlapRangeIndex,
    malformed_or_duplicate: OverlapRangeIndex,
}

impl DiagnosticRangeIndex {
    fn new(diagnostics: &[crate::TaskDiagnostic]) -> Self {
        Self {
            self_dependency: OverlapRangeIndex::new(
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code == TaskDiagnosticCode::SelfDependency)
                    .map(|diagnostic| diagnostic.range.clone()),
            ),
            malformed_or_duplicate: OverlapRangeIndex::new(
                diagnostics
                    .iter()
                    .filter(|diagnostic| {
                        matches!(
                            diagnostic.code,
                            TaskDiagnosticCode::MalformedMacro | TaskDiagnosticCode::DuplicateMacro
                        )
                    })
                    .map(|diagnostic| diagnostic.range.clone()),
            ),
        }
    }

    fn code_for(&self, range: &Range<u64>) -> TaskRebaselineBlockerCode {
        if self.self_dependency.overlaps(range) {
            TaskRebaselineBlockerCode::SelfDependency
        } else if self.malformed_or_duplicate.overlaps(range) {
            TaskRebaselineBlockerCode::MalformedMacroResidue
        } else {
            TaskRebaselineBlockerCode::InvalidLegacyTask
        }
    }
}

struct OverlapRangeIndex {
    starts: Vec<u64>,
    prefix_max_ends: Vec<u64>,
}

impl OverlapRangeIndex {
    fn new(ranges: impl Iterator<Item = Range<u64>>) -> Self {
        let mut ranges = ranges.collect::<Vec<_>>();
        ranges.sort_by_key(|range| (range.start, range.end));
        let starts = ranges.iter().map(|range| range.start).collect::<Vec<_>>();
        let mut maximum_end = 0_u64;
        let prefix_max_ends = ranges
            .into_iter()
            .map(|range| {
                maximum_end = maximum_end.max(range.end);
                maximum_end
            })
            .collect();
        Self {
            starts,
            prefix_max_ends,
        }
    }

    fn overlaps(&self, range: &Range<u64>) -> bool {
        let before_end = self.starts.partition_point(|start| *start < range.end);
        before_end > 0 && self.prefix_max_ends[before_end - 1] > range.start
    }
}

fn add_candidate_blocker(
    candidate: &mut Candidate,
    code: TaskRebaselineBlockerCode,
    blockers: &mut Vec<TaskRebaselineBlocker>,
    node_id: NodeId,
    old_task_id: Option<TaskId>,
    range: Range<u64>,
) -> Result<(), TaskRebaselineError> {
    if candidate.blockers.insert(code) {
        push_blocker(
            blockers,
            blocker(
                code,
                Some(node_id),
                old_task_id,
                None,
                Some(range),
                blocker_message(code),
            ),
        )?;
    }
    Ok(())
}

fn push_blocker(
    blockers: &mut Vec<TaskRebaselineBlocker>,
    value: TaskRebaselineBlocker,
) -> Result<(), TaskRebaselineError> {
    if blockers.len() == TASK_REBASELINE_MAX_BLOCKERS {
        return Err(TaskRebaselineError::ResourceLimitExceeded("blocker count"));
    }
    blockers.push(value);
    Ok(())
}

fn blocker(
    code: TaskRebaselineBlockerCode,
    source_node_id: Option<NodeId>,
    old_task_id: Option<TaskId>,
    dependency_task_id: Option<TaskId>,
    range: Option<Range<u64>>,
    message: impl Into<String>,
) -> TaskRebaselineBlocker {
    TaskRebaselineBlocker {
        code,
        source_node_id,
        old_task_id,
        dependency_task_id,
        range,
        message: message.into(),
    }
}

const fn blocker_message(code: TaskRebaselineBlockerCode) -> &'static str {
    match code {
        TaskRebaselineBlockerCode::InvalidLegacyTask => {
            "legacy structured task syntax or lifecycle fields are invalid"
        }
        TaskRebaselineBlockerCode::MalformedMacroResidue => {
            "unprotected trailing task macro residue is malformed or ambiguous"
        }
        TaskRebaselineBlockerCode::ParserAlignmentUnproven => {
            "legacy evidence does not align with exactly one canonical checklist occurrence"
        }
        TaskRebaselineBlockerCode::IncompleteStructuredBranch => {
            "the complete attached AsciiDoc checklist branch cannot be proven"
        }
        TaskRebaselineBlockerCode::DuplicateLegacyTaskId => {
            "legacy task ID is declared by multiple workspace occurrences"
        }
        TaskRebaselineBlockerCode::RecurrenceUnsupported => {
            "task-node v1 has no accepted recurrence or repeat-from target"
        }
        TaskRebaselineBlockerCode::UnresolvedDependency => {
            "legacy dependency does not resolve to one planned task node"
        }
        TaskRebaselineBlockerCode::AmbiguousDependency => {
            "legacy dependency resolves to multiple structured occurrences"
        }
        TaskRebaselineBlockerCode::SelfDependency => "legacy task depends on its own identity",
        TaskRebaselineBlockerCode::DependencyCycle => {
            "planned task-node dependency graph contains a cycle"
        }
        TaskRebaselineBlockerCode::RelativeLocator => {
            "attached task content contains a relative locator whose base would change"
        }
        TaskRebaselineBlockerCode::DocumentContextDependency => {
            "attached task content depends on source-document context"
        }
        TaskRebaselineBlockerCode::NestedStructuredBranchOverlap => {
            "nested structured task branches overlap"
        }
        TaskRebaselineBlockerCode::DestinationNameUnavailable => {
            "the exact title has no unsuffixed, untruncated portable node-name suggestion"
        }
        TaskRebaselineBlockerCode::DestinationContentBoundary => {
            "the prospective task-node path is not classified as managed content"
        }
        TaskRebaselineBlockerCode::DestinationCollision => {
            "the default owning-node destination has an exact or portable case-fold collision"
        }
        TaskRebaselineBlockerCode::AnnotationReplicaIncomplete => {
            "annotation sidecar presence or absence cannot be proven from this replica"
        }
        TaskRebaselineBlockerCode::AnnotationAuthorityInvalid => {
            "annotation sidecar authority is malformed, conflicted, or changed"
        }
        TaskRebaselineBlockerCode::AnnotationMigrationRequired => {
            "non-empty annotations require a separately proven anchor-migration plan"
        }
        TaskRebaselineBlockerCode::TrashRestoreRequired => {
            "legacy task authority in a valid Trash payload must be restored before rebaseline"
        }
        TaskRebaselineBlockerCode::QueryPopulationEquivalenceUnproven => {
            "task query population equivalence is not proven by read-only package 1"
        }
        TaskRebaselineBlockerCode::InvalidTaskQuery => {
            "task query is invalid and cannot be converted losslessly"
        }
    }
}

fn diagnose_duplicate_ids(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<(), TaskRebaselineError> {
    let mut by_id = BTreeMap::<TaskId, Vec<usize>>::new();
    for (index, candidate) in candidates.iter().enumerate() {
        by_id.entry(candidate.old_task_id).or_default().push(index);
    }
    for (task_id, indexes) in by_id.into_iter().filter(|(_, indexes)| indexes.len() > 1) {
        for index in indexes {
            let candidate = &mut candidates[index];
            let document = &documents[candidate.document];
            let range = candidate.task.metadata.as_ref().map_or_else(
                || candidate.task.range.clone(),
                |metadata| metadata.range.clone(),
            );
            add_candidate_blocker(
                candidate,
                TaskRebaselineBlockerCode::DuplicateLegacyTaskId,
                blockers,
                document.node_id,
                Some(task_id),
                range,
            )?;
        }
    }
    Ok(())
}

fn diagnose_nested_overlaps(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<(), TaskRebaselineError> {
    let mut ranges = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            candidate_branch_range(candidate)
                .map(|range| (candidate.document, range.start, range.end, index))
        })
        .collect::<Vec<_>>();
    ranges.sort_unstable();
    let mut overlapped = BTreeSet::new();
    let mut longest: Option<(usize, u64, usize)> = None;
    for (document, start, end, index) in ranges {
        if let Some((prior_document, prior_end, prior_index)) = longest
            && prior_document == document
            && start < prior_end
        {
            overlapped.insert(prior_index);
            overlapped.insert(index);
        }
        if longest.is_none_or(|(prior_document, prior_end, _)| {
            prior_document != document || end > prior_end
        }) {
            longest = Some((document, end, index));
        }
    }
    for index in overlapped {
        let document = &documents[candidates[index].document];
        let old_id = candidates[index].old_task_id;
        let range = candidate_branch_range(&candidates[index])
            .unwrap_or_else(|| candidates[index].task.range.clone());
        add_candidate_blocker(
            &mut candidates[index],
            TaskRebaselineBlockerCode::NestedStructuredBranchOverlap,
            blockers,
            document.node_id,
            Some(old_id),
            range,
        )?;
    }
    Ok(())
}

fn candidate_branch_range(candidate: &Candidate) -> Option<Range<u64>> {
    candidate
        .checklist
        .as_ref()?
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .map(|promotion| promotion.source_replacement_range.clone())
}

fn reviewed_identity_map(
    reviewed: Option<&TaskRebaselinePlan>,
) -> Result<Option<BTreeMap<TaskId, NodeId>>, TaskRebaselineError> {
    let Some(reviewed) = reviewed else {
        return Ok(None);
    };
    let mut ids = BTreeMap::new();
    for mapping in &reviewed.identity_map {
        if mapping.old_task_id.to_string() == mapping.generated_node_id.to_string()
            || ids
                .insert(mapping.old_task_id, mapping.generated_node_id)
                .is_some()
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    if reviewed
        .queries
        .iter()
        .any(|query| query.disposition == TaskRebaselineQueryDisposition::CanonicalUnchanged)
    {
        return Err(TaskRebaselineError::InvalidReviewedPlan);
    }
    Ok(Some(ids))
}

fn assign_mappings(
    root: &Path,
    content_rules: &crate::content_boundary::ContentRules,
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
    occupied_ids: &mut BTreeSet<String>,
    reviewed_ids: Option<&BTreeMap<TaskId, NodeId>>,
) -> Result<(), TaskRebaselineError> {
    let parent_locators = documents
        .iter()
        .map(|document| relative_locator(root, &document.node_directory))
        .collect::<Vec<_>>();
    preflight_destination_content_rule_work(content_rules, candidates, &parent_locators)?;
    let mut destination_keys = BTreeMap::<(NodeId, String), Vec<usize>>::new();
    for (index, candidate) in candidates.iter_mut().enumerate() {
        if !candidate_can_receive_mapping(candidate) {
            continue;
        }
        let reviewed_id = reviewed_ids.and_then(|ids| ids.get(&candidate.old_task_id).copied());
        let generated = if reviewed_ids.is_some() {
            reviewed_id.ok_or(TaskRebaselineError::InvalidReviewedPlan)?
        } else {
            fresh_node_id(occupied_ids)?
        };
        if generated.to_string() == candidate.old_task_id.to_string()
            || !occupied_ids.insert(generated.to_string())
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
        candidate.generated_node_id = Some(generated);
        let document = &documents[candidate.document];
        let name = candidate.name.as_ref().expect("checked name");
        let parent_locator = &parent_locators[candidate.document];
        candidate.destination_locator = Some(if parent_locator.is_empty() {
            name.clone()
        } else {
            format!("{parent_locator}/{name}")
        });
        if crate::content_boundary::validate_managed_node_path_with_rules(
            root,
            &document.node_directory.join(name),
            content_rules,
        )
        .is_err()
        {
            let old_task_id = candidate.old_task_id;
            let range = candidate.task.description_range.clone();
            add_candidate_blocker(
                candidate,
                TaskRebaselineBlockerCode::DestinationContentBoundary,
                blockers,
                document.node_id,
                Some(old_task_id),
                range,
            )?;
            continue;
        }
        destination_keys
            .entry((
                document.node_id,
                crate::portable_name::portable_name_collision_key(name),
            ))
            .or_default()
            .push(index);
    }
    if let Some(reviewed_ids) = reviewed_ids {
        let assigned = candidates
            .iter()
            .filter_map(|candidate| candidate.generated_node_id.map(|_| candidate.old_task_id))
            .collect::<BTreeSet<_>>();
        if reviewed_ids.keys().copied().collect::<BTreeSet<_>>() != assigned {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    diagnose_destination_collisions(candidates, documents, blockers, destination_keys)
}

fn candidate_can_receive_mapping(candidate: &Candidate) -> bool {
    !candidate
        .blockers
        .contains(&TaskRebaselineBlockerCode::DuplicateLegacyTaskId)
        && !candidate
            .blockers
            .contains(&TaskRebaselineBlockerCode::RecurrenceUnsupported)
        && !candidate
            .blockers
            .contains(&TaskRebaselineBlockerCode::InvalidLegacyTask)
        && !candidate
            .blockers
            .contains(&TaskRebaselineBlockerCode::MalformedMacroResidue)
        && !candidate
            .blockers
            .contains(&TaskRebaselineBlockerCode::ParserAlignmentUnproven)
        && candidate.name.is_some()
}

fn preflight_destination_content_rule_work(
    content_rules: &crate::content_boundary::ContentRules,
    candidates: &[Candidate],
    parent_locators: &[String],
) -> Result<(), TaskRebaselineError> {
    let mut total = 0_usize;
    for candidate in candidates
        .iter()
        .filter(|candidate| candidate_can_receive_mapping(candidate))
    {
        let parent_locator = parent_locators
            .get(candidate.document)
            .ok_or(TaskRebaselineError::InvalidReviewedPlan)?;
        let name = candidate.name.as_ref().expect("filtered name");
        let destination_locator = if parent_locator.is_empty() {
            name.clone()
        } else {
            format!("{parent_locator}/{name}")
        };
        let work = content_rules
            .managed_node_classification_work_upper_bound(&destination_locator, name)
            .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                "destination content-rule match work",
            ))?;
        total = total
            .checked_add(work)
            .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                "destination content-rule match work",
            ))?;
        if total > MAX_DESTINATION_CONTENT_RULE_MATCH_WORK {
            return Err(TaskRebaselineError::ResourceLimitExceeded(
                "destination content-rule match work",
            ));
        }
    }
    Ok(())
}

fn diagnose_destination_collisions(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
    destination_keys: BTreeMap<(NodeId, String), Vec<usize>>,
) -> Result<(), TaskRebaselineError> {
    let mut occupied_by_parent = BTreeMap::<NodeId, Option<BTreeSet<String>>>::new();
    for ((parent_id, _), indexes) in &destination_keys {
        if occupied_by_parent.contains_key(parent_id) {
            continue;
        }
        let document = indexes
            .first()
            .and_then(|index| candidates.get(*index))
            .and_then(|candidate| documents.get(candidate.document))
            .ok_or(TaskRebaselineError::InvalidReviewedPlan)?;
        let entries = fs::read_dir(&document.node_directory)
            .map_err(|_| TaskRebaselineError::DocumentRead(*parent_id))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| TaskRebaselineError::DocumentRead(*parent_id))?;
        let keys = entries
            .into_iter()
            .map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(crate::portable_name::portable_name_collision_key)
            })
            .collect::<Option<BTreeSet<_>>>();
        occupied_by_parent.insert(*parent_id, keys);
    }

    for ((parent_id, key), indexes) in destination_keys {
        let occupied = occupied_by_parent
            .get(&parent_id)
            .is_none_or(|keys| keys.as_ref().is_none_or(|keys| keys.contains(&key)));
        if occupied || indexes.len() > 1 {
            for index in indexes {
                let candidate = &mut candidates[index];
                add_candidate_blocker(
                    candidate,
                    TaskRebaselineBlockerCode::DestinationCollision,
                    blockers,
                    parent_id,
                    Some(candidate.old_task_id),
                    candidate.task.description_range.clone(),
                )?;
            }
        }
    }
    Ok(())
}

fn fresh_node_id(occupied: &BTreeSet<String>) -> Result<NodeId, TaskRebaselineError> {
    for _ in 0..1_024 {
        let id = NodeId::new_v4();
        if !occupied.contains(&id.to_string()) {
            return Ok(id);
        }
    }
    Err(TaskRebaselineError::GeneratedIdentityExhausted)
}

fn inventory_annotations(
    documents: &mut [ActiveDocument],
    candidates: &mut [Candidate],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<(), TaskRebaselineError> {
    let mut candidates_by_document = vec![Vec::new(); documents.len()];
    for (index, candidate) in candidates.iter().enumerate() {
        candidates_by_document[candidate.document].push(index);
    }
    for (document_index, document) in documents.iter_mut().enumerate() {
        if document.raw_macros.is_empty() {
            document.annotation_inventory = TaskRebaselineAnnotationInventory::ConfirmedAbsent;
            continue;
        }
        let candidate_indexes = &candidates_by_document[document_index];
        let Ok((expected_state, store)) =
            crate::workspace_transaction::observe_annotation_sidecar_at_authorized_node(
                &document.node_directory,
                document.node_id,
            )
        else {
            document.annotation_inventory = TaskRebaselineAnnotationInventory::Invalid;
            for &index in candidate_indexes {
                let old_id = candidates[index].old_task_id;
                let range = candidates[index].task.range.clone();
                add_candidate_blocker(
                    &mut candidates[index],
                    TaskRebaselineBlockerCode::AnnotationAuthorityInvalid,
                    blockers,
                    document.node_id,
                    Some(old_id),
                    range,
                )?;
            }
            continue;
        };
        let annotation_count = store.annotations.len();
        document.annotation_inventory = match expected_state {
            crate::workspace_transaction::TaskPromotionSidecarState::Present { sha256 } => {
                TaskRebaselineAnnotationInventory::Present {
                    sha256,
                    annotation_count: u64::try_from(annotation_count).unwrap_or(u64::MAX),
                }
            }
            crate::workspace_transaction::TaskPromotionSidecarState::ConfirmedAbsent => {
                TaskRebaselineAnnotationInventory::ConfirmedAbsent
            }
        };
        if annotation_count > 0 {
            for &index in candidate_indexes {
                let old_id = candidates[index].old_task_id;
                let range = candidates[index].task.range.clone();
                add_candidate_blocker(
                    &mut candidates[index],
                    TaskRebaselineBlockerCode::AnnotationMigrationRequired,
                    blockers,
                    document.node_id,
                    Some(old_id),
                    range,
                )?;
            }
        }
    }
    Ok(())
}

fn diagnose_dependencies(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<(), TaskRebaselineError> {
    let mut declarations = BTreeMap::<TaskId, Vec<usize>>::new();
    for (index, candidate) in candidates.iter().enumerate() {
        declarations
            .entry(candidate.old_task_id)
            .or_default()
            .push(index);
    }
    let (graph, reverse_edges) =
        collect_dependency_graph(candidates, documents, blockers, &declarations)?;
    block_dependency_cycles(candidates, documents, blockers, &declarations, &graph)?;
    propagate_blocked_dependency_targets(
        candidates,
        documents,
        blockers,
        &declarations,
        reverse_edges,
    )
}

type DependencyGraph = BTreeMap<TaskId, Vec<TaskId>>;
type ReverseDependencyEdges = BTreeMap<TaskId, Vec<(usize, Range<u64>)>>;

fn collect_dependency_graph(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
    declarations: &BTreeMap<TaskId, Vec<usize>>,
) -> Result<(DependencyGraph, ReverseDependencyEdges), TaskRebaselineError> {
    let mut graph = BTreeMap::<TaskId, Vec<TaskId>>::new();
    let mut reverse_edges = BTreeMap::<TaskId, Vec<(usize, Range<u64>)>>::new();
    for index in 0..candidates.len() {
        let Some((task_id, dependencies, range)) =
            candidates[index].task.metadata.as_ref().map(|metadata| {
                (
                    metadata.id,
                    metadata.dependencies.clone(),
                    dependency_attribute_range(metadata),
                )
            })
        else {
            continue;
        };
        if candidates[index].generated_node_id.is_none() {
            continue;
        }
        graph.entry(task_id).or_default();
        let node_id = documents[candidates[index].document].node_id;
        for dependency in dependencies {
            reverse_edges
                .entry(dependency)
                .or_default()
                .push((index, range.clone()));
            match dependency_resolution(dependency, declarations, candidates) {
                DependencyResolution::Valid => {
                    graph.entry(task_id).or_default().push(dependency);
                }
                DependencyResolution::Unresolved => record_dependency_blocker(
                    &mut candidates[index],
                    blockers,
                    DependencyBlockerEvidence {
                        node_id,
                        old_task_id: task_id,
                        dependency,
                        range: range.clone(),
                    },
                    TaskRebaselineBlockerCode::UnresolvedDependency,
                    blocker_message(TaskRebaselineBlockerCode::UnresolvedDependency),
                )?,
                DependencyResolution::Ambiguous => record_dependency_blocker(
                    &mut candidates[index],
                    blockers,
                    DependencyBlockerEvidence {
                        node_id,
                        old_task_id: task_id,
                        dependency,
                        range: range.clone(),
                    },
                    TaskRebaselineBlockerCode::AmbiguousDependency,
                    blocker_message(TaskRebaselineBlockerCode::AmbiguousDependency),
                )?,
                DependencyResolution::Blocked => record_dependency_blocker(
                    &mut candidates[index],
                    blockers,
                    DependencyBlockerEvidence {
                        node_id,
                        old_task_id: task_id,
                        dependency,
                        range: range.clone(),
                    },
                    TaskRebaselineBlockerCode::UnresolvedDependency,
                    "legacy dependency target is blocked from task-node mapping",
                )?,
            }
        }
    }
    Ok((graph, reverse_edges))
}

enum DependencyResolution {
    Valid,
    Unresolved,
    Ambiguous,
    Blocked,
}

fn dependency_resolution(
    dependency: TaskId,
    declarations: &BTreeMap<TaskId, Vec<usize>>,
    candidates: &[Candidate],
) -> DependencyResolution {
    let Some(targets) = declarations.get(&dependency) else {
        return DependencyResolution::Unresolved;
    };
    let [target] = targets.as_slice() else {
        return DependencyResolution::Ambiguous;
    };
    if candidates[*target].generated_node_id.is_none() || !candidates[*target].blockers.is_empty() {
        DependencyResolution::Blocked
    } else {
        DependencyResolution::Valid
    }
}

struct DependencyBlockerEvidence {
    node_id: NodeId,
    old_task_id: TaskId,
    dependency: TaskId,
    range: Range<u64>,
}

fn record_dependency_blocker(
    candidate: &mut Candidate,
    blockers: &mut Vec<TaskRebaselineBlocker>,
    evidence: DependencyBlockerEvidence,
    code: TaskRebaselineBlockerCode,
    message: &str,
) -> Result<(), TaskRebaselineError> {
    candidate.blockers.insert(code);
    push_blocker(
        blockers,
        blocker(
            code,
            Some(evidence.node_id),
            Some(evidence.old_task_id),
            Some(evidence.dependency),
            Some(evidence.range),
            message,
        ),
    )
}

fn block_dependency_cycles(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
    declarations: &BTreeMap<TaskId, Vec<usize>>,
    graph: &DependencyGraph,
) -> Result<(), TaskRebaselineError> {
    for component in strongly_connected_components(graph)
        .into_iter()
        .filter(|component| component.len() > 1)
    {
        for task_id in component {
            if let Some(index) = declarations
                .get(&task_id)
                .and_then(|indexes| (indexes.len() == 1).then_some(indexes[0]))
            {
                let node_id = documents[candidates[index].document].node_id;
                let range = candidates[index].task.range.clone();
                add_candidate_blocker(
                    &mut candidates[index],
                    TaskRebaselineBlockerCode::DependencyCycle,
                    blockers,
                    node_id,
                    Some(task_id),
                    range,
                )?;
            }
        }
    }
    Ok(())
}

fn propagate_blocked_dependency_targets(
    candidates: &mut [Candidate],
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
    declarations: &BTreeMap<TaskId, Vec<usize>>,
    mut reverse_edges: ReverseDependencyEdges,
) -> Result<(), TaskRebaselineError> {
    let mut queued = BTreeSet::new();
    let mut queue = VecDeque::new();
    for (task_id, indexes) in declarations {
        if let [index] = indexes.as_slice()
            && !candidates[*index].blockers.is_empty()
            && queued.insert(*task_id)
        {
            queue.push_back(*task_id);
        }
    }
    while let Some(dependency) = queue.pop_front() {
        for (index, range) in reverse_edges.remove(&dependency).unwrap_or_default() {
            if !candidates[index].blockers.is_empty() {
                continue;
            }
            let node_id = documents[candidates[index].document].node_id;
            let old_task_id = candidates[index].old_task_id;
            candidates[index]
                .blockers
                .insert(TaskRebaselineBlockerCode::UnresolvedDependency);
            push_blocker(
                blockers,
                blocker(
                    TaskRebaselineBlockerCode::UnresolvedDependency,
                    Some(node_id),
                    Some(old_task_id),
                    Some(dependency),
                    Some(range),
                    "legacy dependency target is blocked from task-node mapping",
                ),
            )?;
            if declarations
                .get(&old_task_id)
                .is_some_and(|indexes| indexes.as_slice() == [index])
                && queued.insert(old_task_id)
            {
                queue.push_back(old_task_id);
            }
        }
    }
    Ok(())
}

fn dependency_attribute_range(metadata: &crate::TaskMetadata) -> Range<u64> {
    metadata
        .attributes
        .iter()
        .find(|attribute| attribute.name == "depends-on")
        .map_or_else(
            || metadata.range.clone(),
            |attribute| attribute.value_range.clone(),
        )
}

fn build_source_previews(
    documents: &[ActiveDocument],
    candidates: &mut [Candidate],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<Vec<TaskRebaselineSourcePreview>, TaskRebaselineError> {
    let mappings = candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .generated_node_id
                .map(|node_id| (candidate.old_task_id, node_id))
        })
        .collect::<BTreeMap<_, _>>();
    let mut candidates_by_document = vec![Vec::new(); documents.len()];
    for (index, candidate) in candidates.iter().enumerate() {
        candidates_by_document[candidate.document].push(index);
    }
    let mut previews = Vec::new();
    let mut total_preview_bytes = 0_usize;
    for (document_index, document) in documents.iter().enumerate() {
        if document.raw_macros.is_empty() {
            continue;
        }
        let mut proposals = Vec::new();
        let mut edits = Vec::new();
        for &index in &candidates_by_document[document_index] {
            let candidate = &mut candidates[index];
            if !candidate.blockers.is_empty() {
                continue;
            }
            let Some(proposal) = build_proposal(document, candidate, &mappings) else {
                let old_task_id = candidate.old_task_id;
                let range = candidate.task.range.clone();
                add_candidate_blocker(
                    candidate,
                    TaskRebaselineBlockerCode::InvalidLegacyTask,
                    blockers,
                    document.node_id,
                    Some(old_task_id),
                    range,
                )?;
                continue;
            };
            let edit_range = usize_range(&proposal.source_replacement_range, &document.source)
                .ok_or(TaskRebaselineError::InvalidReviewedPlan)?;
            edits.push(SourceEdit {
                range: edit_range,
                replacement: proposal.replacement_source.clone(),
            });
            proposals.push(proposal);
        }
        edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
        let proposed_source = if edits.is_empty() {
            document.source.clone()
        } else {
            SourceEditPlan::new(&document.source, edits)
                .ok()
                .and_then(|plan| plan.apply(&document.source))
                .ok_or(TaskRebaselineError::InvalidReviewedPlan)?
        };
        let document_preview_bytes = proposals.iter().fold(
            document.source.len().checked_add(proposed_source.len()),
            |total, proposal: &TaskRebaselineProposal| {
                total
                    .and_then(|bytes| bytes.checked_add(proposal.expected_source.len()))
                    .and_then(|bytes| bytes.checked_add(proposal.replacement_source.len()))
                    .and_then(|bytes| bytes.checked_add(proposal.proposed_task_source.len()))
            },
        );
        total_preview_bytes = total_preview_bytes
            .checked_add(document_preview_bytes.ok_or(
                TaskRebaselineError::ResourceLimitExceeded("total preview bytes"),
            )?)
            .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                "total preview bytes",
            ))?;
        if total_preview_bytes > TASK_REBASELINE_MAX_TOTAL_PREVIEW_BYTES {
            return Err(TaskRebaselineError::ResourceLimitExceeded(
                "total preview bytes",
            ));
        }
        previews.push(TaskRebaselineSourcePreview {
            source_node_id: document.node_id,
            document_revision: document.revision.clone(),
            document_locator: document.locator.clone(),
            original_source: document.source.clone(),
            proposed_source,
            annotations: document.annotation_inventory.clone(),
            proposals,
        });
    }
    Ok(previews)
}

fn build_proposal(
    document: &ActiveDocument,
    candidate: &Candidate,
    mappings: &BTreeMap<TaskId, NodeId>,
) -> Option<TaskRebaselineProposal> {
    let metadata = candidate.task.metadata.as_ref()?;
    let checklist = candidate.checklist.as_ref()?;
    let promotion = checklist.parser_occurrence.promotion_branch.as_ref()?;
    if !promotion.context_dependencies.is_empty() {
        return None;
    }
    let generated_node_id = candidate.generated_node_id?;
    let fields = mapped_task_fields(&candidate.task, metadata, mappings)?;
    let profile = TaskNodeProfile {
        profile: TaskNodeProfileVersion::V1,
        state: fields.state,
        priority: fields.priority,
        created: fields
            .created
            .as_deref()
            .and_then(|value| TaskNodeTemporal::parse(value).ok()),
        start: fields
            .start
            .as_deref()
            .and_then(|value| TaskNodeTemporal::parse(value).ok()),
        scheduled: fields
            .scheduled
            .as_deref()
            .and_then(|value| TaskNodeTemporal::parse(value).ok()),
        due: fields
            .due
            .as_deref()
            .and_then(|value| TaskNodeTemporal::parse(value).ok()),
        closed: fields
            .closed
            .as_deref()
            .and_then(|value| TaskNodeTemporal::parse(value).ok()),
        depends_on: fields.depends_on.clone(),
    };
    let body = promotion.destination_body(&document.source)?;
    let (proposed_task_source, body_start) = crate::task_node::build_task_node_document_source(
        generated_node_id,
        &candidate.title,
        &profile,
        &body,
    )?;
    if proposed_task_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
        || proposed_task_source.get(body_start..) != Some(body.as_str())
    {
        return None;
    }
    let analysis = analyze_task_node_profile(&proposed_task_source, Some(generated_node_id));
    if weftext_asciidoc::analyze(&proposed_task_source).status == AnalysisStatus::Failed
        || analysis.profile.as_ref() != Some(&profile)
        || !analysis.diagnostics.is_empty()
        || analysis.title.as_ref().map(|title| title.title.as_str())
            != Some(candidate.title.as_str())
        || parse_node_metadata(&proposed_task_source).ok()?.id != Some(generated_node_id)
    {
        return None;
    }
    let encoded_label = encode_node_link_label(&candidate.title).ok()?;
    let replacement_source = crate::task_promotion_transaction::build_source_replacement(
        &document.source,
        promotion,
        checklist.list_depth,
        generated_node_id,
        &encoded_label,
    )
    .ok()?;
    let range = promotion.source_replacement_range.clone();
    let source_range = usize_range(&range, &document.source)?;
    Some(TaskRebaselineProposal {
        old_task_id: metadata.id,
        generated_node_id,
        fields,
        source_replacement_range: range,
        expected_source: document.source[source_range].to_owned(),
        replacement_source,
        proposed_task_source,
    })
}

fn mapped_task_fields(
    task: &TaskOccurrence,
    metadata: &crate::TaskMetadata,
    mappings: &BTreeMap<TaskId, NodeId>,
) -> Option<TaskRebaselineTaskFields> {
    let state = match task.state {
        crate::TaskState::Open => match metadata.phase.unwrap_or(TaskPhase::Todo) {
            TaskPhase::Todo => TaskNodeState::Todo,
            TaskPhase::InProgress => TaskNodeState::InProgress,
            TaskPhase::OnHold => TaskNodeState::OnHold,
        },
        crate::TaskState::Closed => {
            match metadata.resolution.unwrap_or(TaskResolution::Completed) {
                TaskResolution::Completed => TaskNodeState::Completed,
                TaskResolution::Cancelled => TaskNodeState::Cancelled,
            }
        }
    };
    let priority = metadata
        .attributes
        .iter()
        .any(|attribute| attribute.name == "priority")
        .then_some(match metadata.priority {
            TaskPriority::Lowest => TaskNodePriority::Lowest,
            TaskPriority::Low => TaskNodePriority::Low,
            TaskPriority::Normal => TaskNodePriority::Normal,
            TaskPriority::Medium => TaskNodePriority::Medium,
            TaskPriority::High => TaskNodePriority::High,
            TaskPriority::Highest => TaskNodePriority::Highest,
        });
    let depends_on = metadata
        .dependencies
        .iter()
        .map(|dependency| mappings.get(dependency).copied())
        .collect::<Option<Vec<_>>>()?;
    Some(TaskRebaselineTaskFields {
        state,
        priority,
        created: temporal_text(metadata.created.as_ref()),
        start: temporal_text(metadata.start.as_ref()),
        scheduled: temporal_text(metadata.scheduled.as_ref()),
        due: temporal_text(metadata.due.as_ref()),
        closed: temporal_text(metadata.closed.as_ref()),
        depends_on,
    })
}

fn temporal_text(value: Option<&TaskDateTime>) -> Option<String> {
    value.map(|value| match value {
        TaskDateTime::Date(value) | TaskDateTime::Instant(value) => value.clone(),
    })
}

fn active_occurrence_inventory(
    documents: &[ActiveDocument],
    candidates: &[Candidate],
) -> Vec<TaskRebaselineOccurrenceInventory> {
    let mut occurrences = Vec::new();
    for document in documents {
        for (raw_index, raw) in document.raw_macros.iter().enumerate() {
            let candidate =
                document.candidate_by_raw[raw_index].and_then(|index| candidates.get(index));
            let (
                old_task_id,
                generated_node_id,
                disposition,
                blocker_codes,
                item_range,
                marker_range,
                description_range,
            ) = if raw.protected {
                (
                    None,
                    None,
                    TaskRebaselineOccurrenceDisposition::ProtectedLiteral,
                    Vec::new(),
                    None,
                    None,
                    None,
                )
            } else if let Some(candidate) = candidate {
                let blocker_codes = candidate.blockers.iter().copied().collect::<Vec<_>>();
                (
                    Some(candidate.old_task_id),
                    candidate.generated_node_id,
                    if blocker_codes.is_empty() {
                        TaskRebaselineOccurrenceDisposition::ProposedTaskNode
                    } else {
                        TaskRebaselineOccurrenceDisposition::Blocked
                    },
                    blocker_codes,
                    Some(candidate.task.range.clone()),
                    Some(candidate.task.marker_range.clone()),
                    Some(candidate.task.description_range.clone()),
                )
            } else {
                let blocker_codes = document.raw_blocker_codes[raw_index].clone();
                (
                    document.raw_old_task_ids[raw_index],
                    None,
                    TaskRebaselineOccurrenceDisposition::Blocked,
                    if blocker_codes.is_empty() {
                        vec![TaskRebaselineBlockerCode::MalformedMacroResidue]
                    } else {
                        blocker_codes
                    },
                    Some(raw.item_range.clone()),
                    None,
                    None,
                )
            };
            occurrences.push(TaskRebaselineOccurrenceInventory {
                source_kind: TaskRebaselineSourceKind::ActiveManagedDocument,
                source_node_id: document.node_id,
                document_revision: document.revision.clone(),
                document_locator: document.locator.clone(),
                macro_range: raw.range.clone(),
                item_range,
                marker_range,
                description_range,
                raw_macro: usize_range(&raw.range, &document.source)
                    .map(|range| document.source[range].to_owned())
                    .unwrap_or_default(),
                raw_item: usize_range(&raw.item_range, &document.source)
                    .map(|range| document.source[range].to_owned())
                    .unwrap_or_default(),
                old_task_id,
                generated_node_id,
                disposition,
                blocker_codes,
            });
        }
    }
    occurrences
}

fn preflight_occurrence_evidence(
    document_locator: &str,
    raw_macros: &[RawMacro],
    total_evidence_bytes: &mut usize,
) -> Result<(), TaskRebaselineError> {
    for raw in raw_macros {
        add_evidence_bytes(
            total_evidence_bytes,
            document_locator
                .len()
                .checked_add(u64_range_len(&raw.range)?)
                .and_then(|bytes| bytes.checked_add(u64_range_len(&raw.item_range).ok()?))
                .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                    "total occurrence evidence bytes",
                ))?,
        )?;
    }
    Ok(())
}

fn u64_range_len(range: &Range<u64>) -> Result<usize, TaskRebaselineError> {
    usize::try_from(range.end.checked_sub(range.start).ok_or(
        TaskRebaselineError::ResourceLimitExceeded("total occurrence evidence bytes"),
    )?)
    .map_err(|_| TaskRebaselineError::ResourceLimitExceeded("total occurrence evidence bytes"))
}

fn add_evidence_bytes(total: &mut usize, additional: usize) -> Result<(), TaskRebaselineError> {
    *total = total
        .checked_add(additional)
        .ok_or(TaskRebaselineError::ResourceLimitExceeded(
            "total occurrence evidence bytes",
        ))?;
    if *total > TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES {
        return Err(TaskRebaselineError::ResourceLimitExceeded(
            "total occurrence evidence bytes",
        ));
    }
    Ok(())
}

fn candidate_mapping(
    candidate: &Candidate,
    documents: &[ActiveDocument],
) -> Option<TaskRebaselineIdentityMapping> {
    Some(TaskRebaselineIdentityMapping {
        source_node_id: documents[candidate.document].node_id,
        old_task_id: candidate.old_task_id,
        generated_node_id: candidate.generated_node_id?,
        destination_parent_node_id: documents[candidate.document].node_id,
        destination_node_locator: candidate.destination_locator.clone()?,
        destination_portable_name: candidate.name.clone()?,
        document_title: candidate.title.clone(),
        link_label: candidate.title.clone(),
    })
}

fn inventory_trash_occurrences(
    inventory: &crate::WorkspaceInventory,
    occurrences: &mut Vec<TaskRebaselineOccurrenceInventory>,
    blockers: &mut Vec<TaskRebaselineBlocker>,
    total_source_bytes: &mut usize,
    legacy_analysis_budget: &mut LegacyAnalysisBudget,
    inventoried_document_count: &mut usize,
    total_evidence_bytes: &mut usize,
) -> Result<BTreeSet<TaskId>, TaskRebaselineError> {
    let mut old_task_ids = BTreeSet::new();
    {
        let mut state = TrashInventoryState {
            occurrences,
            blockers,
            old_task_ids: &mut old_task_ids,
            total_source_bytes,
            legacy_analysis_budget,
            inventoried_document_count,
            total_evidence_bytes,
        };
        for item in &inventory.trash_items {
            for (node_id, locator) in &item.node_locators {
                inventory_trash_document(item, *node_id, locator, &mut state)?;
            }
        }
    }
    Ok(old_task_ids)
}

struct TrashInventoryState<'a> {
    occurrences: &'a mut Vec<TaskRebaselineOccurrenceInventory>,
    blockers: &'a mut Vec<TaskRebaselineBlocker>,
    old_task_ids: &'a mut BTreeSet<TaskId>,
    total_source_bytes: &'a mut usize,
    legacy_analysis_budget: &'a mut LegacyAnalysisBudget,
    inventoried_document_count: &'a mut usize,
    total_evidence_bytes: &'a mut usize,
}

fn inventory_trash_document(
    item: &crate::WorkspaceTrashItem,
    node_id: NodeId,
    locator: &str,
    state: &mut TrashInventoryState<'_>,
) -> Result<(), TaskRebaselineError> {
    if *state.inventoried_document_count == TASK_REBASELINE_MAX_DOCUMENTS {
        return Err(TaskRebaselineError::ResourceLimitExceeded("document count"));
    }
    *state.inventoried_document_count += 1;
    let directory = item
        .payload_path
        .parent()
        .unwrap_or(&item.payload_path)
        .join(Path::new(locator));
    let snapshot =
        read_node_document(&directory).map_err(|_| TaskRebaselineError::DocumentRead(node_id))?;
    if snapshot.node_id != node_id {
        return Err(TaskRebaselineError::InvalidDocumentIdentity(node_id));
    }
    if snapshot.source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES {
        return Err(TaskRebaselineError::ResourceLimitExceeded("document bytes"));
    }
    *state.total_source_bytes = state
        .total_source_bytes
        .saturating_add(snapshot.source.len());
    if *state.total_source_bytes > TASK_REBASELINE_MAX_TOTAL_SOURCE_BYTES {
        return Err(TaskRebaselineError::ResourceLimitExceeded(
            "total source bytes",
        ));
    }
    preflight_legacy_analysis_work(&snapshot.source, state.legacy_analysis_budget)?;
    let document_locator = format!(
        ".weftext-trash/{}/payload/{locator}/{}.adoc",
        item.manifest.trash_item_id(),
        directory
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("document")
    );
    let mut raw_macros = raw_macros(
        &snapshot.source,
        state.legacy_analysis_budget.remaining_inventory_occurrences,
    )?;
    state.legacy_analysis_budget.remaining_inventory_occurrences = state
        .legacy_analysis_budget
        .remaining_inventory_occurrences
        .saturating_sub(raw_macros.len());
    preflight_occurrence_evidence(&document_locator, &raw_macros, state.total_evidence_bytes)?;
    let parser = weftext_asciidoc::analyze(&snapshot.source);
    let parser_failed = parser.status == AnalysisStatus::Failed;
    if !parser_failed {
        mark_protected_raw_macros(&mut raw_macros, &parser.protected_ranges);
    }
    let legacy = analyze_task_source(&snapshot.source);
    if parser_failed {
        push_blocker(
            state.blockers,
            blocker(
                TaskRebaselineBlockerCode::ParserAlignmentUnproven,
                Some(node_id),
                None,
                None,
                Some(0..u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX)),
                "canonical AsciiDoc analysis failed for the complete Trash payload document",
            ),
        )?;
    }
    append_trash_occurrences(
        &snapshot,
        node_id,
        &document_locator,
        parser_failed,
        &legacy,
        raw_macros,
        state,
    )
}

fn append_trash_occurrences(
    snapshot: &crate::DocumentSnapshot,
    node_id: NodeId,
    document_locator: &str,
    parser_failed: bool,
    legacy: &crate::TaskSourceAnalysis,
    raw_macros: Vec<RawMacro>,
    state: &mut TrashInventoryState<'_>,
) -> Result<(), TaskRebaselineError> {
    let legacy_by_macro_range = legacy_tasks_by_macro_range(legacy);
    for raw in raw_macros {
        let matching = legacy_by_macro_range
            .get(&(raw.range.start, raw.range.end))
            .and_then(|indexes| {
                let [index] = indexes.as_slice() else {
                    return None;
                };
                legacy.tasks.get(*index)
            });
        let old_task_id =
            crate::task::legacy_task_id_from_closed_macro(&snapshot.source, &raw.range);
        state.old_task_ids.extend(old_task_id);
        let protected = raw.protected && !parser_failed;
        let blocker_codes = trash_occurrence_blockers(
            state.blockers,
            node_id,
            old_task_id,
            &raw,
            protected,
            parser_failed,
        )?;
        state.occurrences.push(TaskRebaselineOccurrenceInventory {
            source_kind: TaskRebaselineSourceKind::TrashPayloadDocument,
            source_node_id: node_id,
            document_revision: snapshot.revision.clone(),
            document_locator: document_locator.to_owned(),
            macro_range: raw.range.clone(),
            item_range: matching
                .map(|task| task.range.clone())
                .or(Some(raw.item_range.clone())),
            marker_range: matching.map(|task| task.marker_range.clone()),
            description_range: matching.map(|task| task.description_range.clone()),
            raw_macro: usize_range(&raw.range, &snapshot.source)
                .map(|range| snapshot.source[range].to_owned())
                .unwrap_or_default(),
            raw_item: usize_range(&raw.item_range, &snapshot.source)
                .map(|range| snapshot.source[range].to_owned())
                .unwrap_or_default(),
            old_task_id,
            generated_node_id: None,
            disposition: if protected {
                TaskRebaselineOccurrenceDisposition::ProtectedLiteral
            } else {
                TaskRebaselineOccurrenceDisposition::TrashRestoreRequired
            },
            blocker_codes,
        });
    }
    Ok(())
}

fn trash_occurrence_blockers(
    blockers: &mut Vec<TaskRebaselineBlocker>,
    node_id: NodeId,
    old_task_id: Option<TaskId>,
    raw: &RawMacro,
    protected: bool,
    parser_failed: bool,
) -> Result<Vec<TaskRebaselineBlockerCode>, TaskRebaselineError> {
    if protected {
        return Ok(Vec::new());
    }
    push_blocker(
        blockers,
        blocker(
            TaskRebaselineBlockerCode::TrashRestoreRequired,
            Some(node_id),
            old_task_id,
            None,
            Some(raw.range.clone()),
            blocker_message(TaskRebaselineBlockerCode::TrashRestoreRequired),
        ),
    )?;
    let mut codes = vec![TaskRebaselineBlockerCode::TrashRestoreRequired];
    if parser_failed {
        codes.push(TaskRebaselineBlockerCode::ParserAlignmentUnproven);
    }
    Ok(codes)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LegacyTaskQueryEvidence {
    range: Range<u64>,
    body_range: Range<u64>,
    valid: bool,
}

// Rebaseline is the only active-code boundary that recognizes the retired query surface. This
// scanner inventories exact legacy evidence for conversion review; it never returns an executable
// plan and is deliberately not exported from this module.
fn scan_legacy_task_queries(source: &str) -> Vec<LegacyTaskQueryEvidence> {
    let protected = weftext_asciidoc::analyze(source).protected_ranges;
    let mut evidence = Vec::new();
    let mut line_start = 0_usize;
    while line_start < source.len() {
        let line_end = legacy_line_end(source, line_start);
        let line = trim_horizontal_range(source, line_start..line_end);
        let protected_line = protected.iter().any(|range| {
            range.start <= u64::try_from(line.start).unwrap_or(u64::MAX)
                && u64::try_from(line.start).unwrap_or(u64::MAX) < range.end
        });
        if protected_line || !legacy_query_header_prefix(source, &line) {
            line_start = legacy_next_line_start(source, line_end);
            continue;
        }
        let (is_task_query, header_valid) = parse_legacy_task_query_header(source, line.clone());
        let opening_start = legacy_next_line_start(source, line_end);
        let opening_end = legacy_line_end(source, opening_start);
        let has_opening =
            opening_start < source.len() && source.get(opening_start..opening_end) == Some("....");
        if !has_opening {
            if is_task_query {
                evidence.push(LegacyTaskQueryEvidence {
                    range: to_u64_range(line.clone()),
                    body_range: to_u64_range(line_end..line_end),
                    valid: false,
                });
            }
            line_start = legacy_next_line_start(source, line_end);
            continue;
        }
        let body_start = legacy_next_line_start(source, opening_end);
        let mut cursor = body_start;
        let mut closing = None;
        while cursor < source.len() {
            let end = legacy_line_end(source, cursor);
            if source.get(cursor..end) == Some("....") {
                closing = Some((cursor, end));
                break;
            }
            cursor = legacy_next_line_start(source, end);
        }
        let (body_end, block_end, next) = closing.map_or(
            (source.len(), source.len(), source.len()),
            |(closing_start, closing_end)| {
                (
                    closing_start,
                    closing_end,
                    legacy_next_line_start(source, closing_end),
                )
            },
        );
        if is_task_query {
            evidence.push(LegacyTaskQueryEvidence {
                range: to_u64_range(line.start..block_end),
                body_range: to_u64_range(body_start..body_end),
                valid: header_valid
                    && closing.is_some()
                    && legacy_task_query_body_is_well_formed(&source[body_start..body_end]),
            });
        }
        line_start = next;
    }
    evidence
}

fn parse_legacy_task_query_header(source: &str, range: Range<usize>) -> (bool, bool) {
    if !source[range.clone()].ends_with(']') {
        return (false, false);
    }
    let Ok(parts) = split_comma_parts(source, range.start + 1..range.end - 1) else {
        return (false, false);
    };
    if parts.is_empty() || &source[trim_range(source, parts[0].clone())] != "query" {
        return (false, false);
    }
    let mut seen = BTreeSet::new();
    let mut task_source = false;
    let mut valid = true;
    for part in parts.into_iter().skip(1) {
        let part = trim_range(source, part);
        let Some(equals) = find_unquoted_equals(source, part.clone()) else {
            valid = false;
            continue;
        };
        let name = &source[trim_range(source, part.start..equals)];
        let value_range = trim_range(source, equals + 1..part.end);
        let Some(value) = decode_attribute_value(source, value_range) else {
            valid = false;
            continue;
        };
        if name == "source" {
            task_source |= value == "tasks";
        }
        if !matches!(name, "source" | "view") || !seen.insert(name) {
            valid = false;
            continue;
        }
        match name {
            "source" => {
                valid &= matches!(value.as_str(), "tasks" | "nodes");
            }
            "view" => {
                valid &= matches!(
                    value.as_str(),
                    "table" | "list" | "task-list" | "board" | "calendar" | "timeline" | "gallery"
                );
            }
            _ => unreachable!(),
        }
    }
    valid &= seen.contains("source");
    (task_source, valid)
}

fn legacy_task_query_body_is_well_formed(body: &str) -> bool {
    if body.contains('\0') {
        return false;
    }
    let mut previous_rank = 0_u8;
    let mut seen = BTreeSet::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let clause = trimmed.split_ascii_whitespace().next().unwrap_or_default();
        let rank = match clause {
            "scope" => 1,
            "where" => 2,
            "group" => 3,
            "sort" => 4,
            "select" => 5,
            "limit" => 6,
            _ => return false,
        };
        if rank < previous_rank || !seen.insert(clause) {
            return false;
        }
        previous_rank = rank;
    }
    true
}

fn legacy_query_header_prefix(source: &str, range: &Range<usize>) -> bool {
    let value = &source[range.clone()];
    value.starts_with("[query")
        && value
            .as_bytes()
            .get("[query".len())
            .is_some_and(|byte| matches!(byte, b',' | b']'))
}

fn trim_horizontal_range(source: &str, mut range: Range<usize>) -> Range<usize> {
    while range.start < range.end && matches!(source.as_bytes()[range.start], b' ' | b'\t') {
        range.start += 1;
    }
    while range.start < range.end && matches!(source.as_bytes()[range.end - 1], b' ' | b'\t') {
        range.end -= 1;
    }
    range
}

fn legacy_line_end(source: &str, start: usize) -> usize {
    source.as_bytes()[start..]
        .iter()
        .position(|byte| matches!(byte, b'\r' | b'\n'))
        .map_or(source.len(), |relative| start + relative)
}

fn legacy_next_line_start(source: &str, end: usize) -> usize {
    match source.as_bytes().get(end..) {
        Some([b'\r', b'\n', ..]) => end + 2,
        Some([b'\r' | b'\n', ..]) => end + 1,
        _ => source.len(),
    }
}

fn inventory_queries(
    documents: &[ActiveDocument],
    blockers: &mut Vec<TaskRebaselineBlocker>,
) -> Result<Vec<TaskRebaselineQueryInventory>, TaskRebaselineError> {
    let mut queries = Vec::new();
    let mut total_query_bytes = 0_usize;
    for document in documents {
        for block in scan_legacy_task_queries(&document.source) {
            let raw_range = usize_range(&block.range, &document.source)
                .ok_or(TaskRebaselineError::InvalidReviewedPlan)?;
            let (disposition, blocker_code) = if block.valid {
                (
                    TaskRebaselineQueryDisposition::ConversionBlocked,
                    Some(TaskRebaselineBlockerCode::QueryPopulationEquivalenceUnproven),
                )
            } else {
                (
                    TaskRebaselineQueryDisposition::InvalidBlocked,
                    Some(TaskRebaselineBlockerCode::InvalidTaskQuery),
                )
            };
            if queries.len() == TASK_REBASELINE_MAX_QUERIES {
                return Err(TaskRebaselineError::ResourceLimitExceeded("query count"));
            }
            total_query_bytes = total_query_bytes
                .checked_add(document.locator.len())
                .and_then(|bytes| bytes.checked_add(raw_range.len()))
                .ok_or(TaskRebaselineError::ResourceLimitExceeded(
                    "total query evidence bytes",
                ))?;
            if total_query_bytes > TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES {
                return Err(TaskRebaselineError::ResourceLimitExceeded(
                    "total query evidence bytes",
                ));
            }
            if let Some(code) = blocker_code {
                push_blocker(
                    blockers,
                    blocker(
                        code,
                        Some(document.node_id),
                        None,
                        None,
                        Some(block.range.clone()),
                        blocker_message(code),
                    ),
                )?;
            }
            queries.push(TaskRebaselineQueryInventory {
                source_node_id: document.node_id,
                document_revision: document.revision.clone(),
                document_locator: document.locator.clone(),
                range: block.range.clone(),
                body_range: block.body_range.clone(),
                raw_source: document.source[raw_range].to_owned(),
                disposition,
            });
        }
    }
    queries.sort_by(|left, right| {
        left.document_locator
            .cmp(&right.document_locator)
            .then_with(|| left.range.start.cmp(&right.range.start))
    });
    Ok(queries)
}

pub(crate) fn validate_reviewed_shape(
    reviewed: &TaskRebaselinePlan,
    completeness: AnnotationReplicaCompleteness,
) -> Result<(), TaskRebaselineError> {
    validate_reviewed_resource_shape(reviewed)?;
    if reviewed.schema != TASK_REBASELINE_SCHEMA
        || reviewed.scope != TaskRebaselineScope::OwnerWorkspace
        || !reviewed.preview_only
        || reviewed.committable
        || reviewed.conversion_ready != reviewed.blockers.is_empty()
        || reviewed.pre_state.workspace_revision != reviewed.base_workspace_revision
        || reviewed.pre_state.portable_inventory_binding
            != TaskRebaselinePortableInventoryBinding::FullOwnerWorkspace
        || reviewed.pre_state.physical_inventory_binding
            != TaskRebaselinePhysicalInventoryBinding::WorkspaceRevisionOnly
        || reviewed.pre_state.external_snapshot_binding
            != TaskRebaselineExternalSnapshotBinding::NotProvided
        || reviewed.annotation_replica_completeness != completeness
        || completeness != AnnotationReplicaCompleteness::CompleteLocalWorkspace
        || WorkspaceRevision::parse(reviewed.base_workspace_revision.as_str()).is_err()
        || !valid_sha256(&reviewed.plan_digest)
    {
        return Err(TaskRebaselineError::InvalidReviewedPlan);
    }
    let mut generated = BTreeSet::new();
    let mut old = BTreeSet::new();
    for mapping in &reviewed.identity_map {
        if mapping.old_task_id.to_string() == mapping.generated_node_id.to_string()
            || !generated.insert(mapping.generated_node_id)
            || !old.insert(mapping.old_task_id)
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    if reviewed.plan_digest != plan_digest(reviewed)? {
        return Err(TaskRebaselineError::InvalidReviewedPlan);
    }
    Ok(())
}

fn validate_reviewed_resource_shape(
    reviewed: &TaskRebaselinePlan,
) -> Result<(), TaskRebaselineError> {
    ensure_plan_json_budget(reviewed).map_err(|_| TaskRebaselineError::InvalidReviewedPlan)?;
    if reviewed.occurrences.len() > TASK_REBASELINE_MAX_OCCURRENCES
        || reviewed.identity_map.len() > TASK_REBASELINE_MAX_OCCURRENCES
        || reviewed.source_previews.len() > TASK_REBASELINE_MAX_DOCUMENTS
        || reviewed.queries.len() > TASK_REBASELINE_MAX_QUERIES
        || reviewed.blockers.len() > TASK_REBASELINE_MAX_BLOCKERS
        || (!reviewed.plan_digest.is_empty() && !valid_sha256(&reviewed.plan_digest))
    {
        return Err(TaskRebaselineError::InvalidReviewedPlan);
    }
    validate_occurrence_resources(&reviewed.occurrences)?;
    validate_identity_mapping_resources(&reviewed.identity_map)?;
    validate_preview_resources(&reviewed.source_previews)?;
    validate_query_and_blocker_resources(&reviewed.queries, &reviewed.blockers)
}

fn validate_occurrence_resources(
    occurrences: &[TaskRebaselineOccurrenceInventory],
) -> Result<(), TaskRebaselineError> {
    let invalid = || TaskRebaselineError::InvalidReviewedPlan;
    let mut evidence_bytes = 0_usize;
    for occurrence in occurrences {
        if !valid_locator(&occurrence.document_locator)
            || DocumentRevision::parse(occurrence.document_revision.as_str()).is_err()
            || !valid_range(&occurrence.macro_range)
            || occurrence
                .item_range
                .as_ref()
                .is_some_and(|range| !valid_range(range))
            || occurrence
                .marker_range
                .as_ref()
                .is_some_and(|range| !valid_range(range))
            || occurrence
                .description_range
                .as_ref()
                .is_some_and(|range| !valid_range(range))
            || occurrence.raw_macro.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
            || occurrence.raw_item.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
            || occurrence.blocker_codes.len() > 22
            || occurrence
                .blocker_codes
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != occurrence.blocker_codes.len()
        {
            return Err(invalid());
        }
        evidence_bytes = evidence_bytes
            .checked_add(occurrence.document_locator.len())
            .and_then(|bytes| bytes.checked_add(occurrence.raw_macro.len()))
            .and_then(|bytes| bytes.checked_add(occurrence.raw_item.len()))
            .ok_or_else(invalid)?;
    }
    if evidence_bytes > TASK_REBASELINE_MAX_TOTAL_EVIDENCE_BYTES {
        return Err(invalid());
    }
    Ok(())
}

fn validate_identity_mapping_resources(
    mappings: &[TaskRebaselineIdentityMapping],
) -> Result<(), TaskRebaselineError> {
    for mapping in mappings {
        if !valid_locator(&mapping.destination_node_locator)
            || !valid_reviewed_text(
                &mapping.destination_portable_name,
                crate::MAX_PORTABLE_NODE_NAME_BYTES,
                false,
            )
            || !valid_reviewed_text(&mapping.document_title, MAX_TITLE_BYTES, false)
            || !valid_reviewed_text(&mapping.link_label, MAX_LINK_LABEL_BYTES, false)
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    Ok(())
}

fn validate_preview_resources(
    previews: &[TaskRebaselineSourcePreview],
) -> Result<(), TaskRebaselineError> {
    let invalid = || TaskRebaselineError::InvalidReviewedPlan;
    let mut preview_bytes = 0_usize;
    for preview in previews {
        if !valid_locator(&preview.document_locator)
            || DocumentRevision::parse(preview.document_revision.as_str()).is_err()
            || preview.original_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
            || preview.proposed_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
            || preview.proposals.len() > TASK_REBASELINE_MAX_OCCURRENCES
        {
            return Err(invalid());
        }
        if let TaskRebaselineAnnotationInventory::Present {
            sha256,
            annotation_count,
        } = &preview.annotations
            && (!valid_sha256(sha256) || *annotation_count > 100_000)
        {
            return Err(invalid());
        }
        preview_bytes = preview_bytes
            .checked_add(preview.original_source.len())
            .and_then(|bytes| bytes.checked_add(preview.proposed_source.len()))
            .ok_or_else(invalid)?;
        for proposal in &preview.proposals {
            if !valid_range(&proposal.source_replacement_range)
                || proposal.expected_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
                || proposal.replacement_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
                || proposal.proposed_task_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
                || proposal.fields.depends_on.len() > TASK_REBASELINE_MAX_OCCURRENCES
                || proposal
                    .fields
                    .depends_on
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != proposal.fields.depends_on.len()
                || [
                    proposal.fields.created.as_deref(),
                    proposal.fields.start.as_deref(),
                    proposal.fields.scheduled.as_deref(),
                    proposal.fields.due.as_deref(),
                    proposal.fields.closed.as_deref(),
                ]
                .into_iter()
                .flatten()
                .any(|value| TaskNodeTemporal::parse(value).is_err())
            {
                return Err(invalid());
            }
            preview_bytes = preview_bytes
                .checked_add(proposal.expected_source.len())
                .and_then(|bytes| bytes.checked_add(proposal.replacement_source.len()))
                .and_then(|bytes| bytes.checked_add(proposal.proposed_task_source.len()))
                .ok_or_else(invalid)?;
        }
    }
    if preview_bytes > TASK_REBASELINE_MAX_TOTAL_PREVIEW_BYTES {
        return Err(invalid());
    }
    Ok(())
}

fn validate_query_and_blocker_resources(
    queries: &[TaskRebaselineQueryInventory],
    blockers: &[TaskRebaselineBlocker],
) -> Result<(), TaskRebaselineError> {
    for query in queries {
        if !valid_locator(&query.document_locator)
            || DocumentRevision::parse(query.document_revision.as_str()).is_err()
            || !valid_range(&query.range)
            || !valid_range(&query.body_range)
            || query.raw_source.len() > TASK_REBASELINE_MAX_DOCUMENT_BYTES
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    for blocker in blockers {
        if !valid_reviewed_text(&blocker.message, 4_096, true)
            || blocker
                .range
                .as_ref()
                .is_some_and(|range| !valid_range(range))
        {
            return Err(TaskRebaselineError::InvalidReviewedPlan);
        }
    }
    Ok(())
}

fn validate_canonical_uuid_json_fields(
    value: &serde_json::Value,
) -> Result<(), TaskRebaselineError> {
    match value {
        serde_json::Value::Object(object) => {
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "sourceNodeId"
                        | "oldTaskId"
                        | "dependencyTaskId"
                        | "generatedNodeId"
                        | "destinationParentNodeId"
                ) && !value.is_null()
                    && value
                        .as_str()
                        .and_then(|text| text.parse::<NodeId>().ok())
                        .is_none()
                {
                    return Err(TaskRebaselineError::InvalidReviewedPlan);
                }
                if key == "dependsOn"
                    && value.as_array().is_some_and(|items| {
                        items.iter().any(|item| {
                            item.as_str()
                                .and_then(|text| text.parse::<NodeId>().ok())
                                .is_none()
                        })
                    })
                {
                    return Err(TaskRebaselineError::InvalidReviewedPlan);
                }
                validate_canonical_uuid_json_fields(value)?;
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                validate_canonical_uuid_json_fields(item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_range(range: &Range<u64>) -> bool {
    range.start <= range.end
}

fn valid_locator(locator: &str) -> bool {
    !locator.is_empty()
        && locator.len() <= 32_768
        && !locator.contains('\\')
        && locator
            .split('/')
            .all(|segment| !segment.is_empty() && !matches!(segment, "." | ".."))
}

fn plan_digest(plan: &TaskRebaselinePlan) -> Result<String, TaskRebaselineError> {
    let mut writer = BoundedHashWriter::new(TASK_REBASELINE_MAX_PLAN_JSON_BYTES);
    serde_json::to_writer(&mut writer, &UnsignedTaskRebaselinePlan(plan))
        .map_err(|_| TaskRebaselineError::ResourceLimitExceeded("reviewed plan JSON bytes"))?;
    Ok(format!("{:x}", writer.finish()))
}

fn ensure_plan_json_budget(plan: &TaskRebaselinePlan) -> Result<(), TaskRebaselineError> {
    let digest_reserve = if plan.plan_digest.is_empty() { 64 } else { 0 };
    let limit = TASK_REBASELINE_MAX_PLAN_JSON_BYTES.saturating_sub(digest_reserve);
    let mut writer = BoundedCountWriter::new(limit);
    serde_json::to_writer(&mut writer, plan)
        .map_err(|_| TaskRebaselineError::ResourceLimitExceeded("reviewed plan JSON bytes"))
}

struct UnsignedTaskRebaselinePlan<'a>(&'a TaskRebaselinePlan);

impl Serialize for UnsignedTaskRebaselinePlan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let plan = self.0;
        let mut state = serializer.serialize_struct("TaskRebaselinePlan", 14)?;
        state.serialize_field("schema", &plan.schema)?;
        state.serialize_field("scope", &plan.scope)?;
        state.serialize_field("previewOnly", &plan.preview_only)?;
        state.serialize_field("committable", &plan.committable)?;
        state.serialize_field("conversionReady", &plan.conversion_ready)?;
        state.serialize_field("baseWorkspaceRevision", &plan.base_workspace_revision)?;
        state.serialize_field("preState", &plan.pre_state)?;
        state.serialize_field(
            "annotationReplicaCompleteness",
            &plan.annotation_replica_completeness,
        )?;
        state.serialize_field("occurrences", &plan.occurrences)?;
        state.serialize_field("identityMap", &plan.identity_map)?;
        state.serialize_field("sourcePreviews", &plan.source_previews)?;
        state.serialize_field("queries", &plan.queries)?;
        state.serialize_field("blockers", &plan.blockers)?;
        state.serialize_field("planDigest", "")?;
        state.end()
    }
}

struct BoundedCountWriter {
    written: usize,
    limit: usize,
}

impl BoundedCountWriter {
    const fn new(limit: usize) -> Self {
        Self { written: 0, limit }
    }
}

impl Write for BoundedCountWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.written = self
            .written
            .checked_add(bytes.len())
            .filter(|written| *written <= self.limit)
            .ok_or_else(|| io::Error::other("task rebaseline JSON budget exceeded"))?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedHashWriter {
    hasher: Sha256,
    written: usize,
    limit: usize,
}

impl BoundedHashWriter {
    fn new(limit: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(PLAN_DIGEST_DOMAIN);
        Self {
            hasher,
            written: 0,
            limit,
        }
    }

    fn finish(self) -> impl std::fmt::LowerHex {
        self.hasher.finalize()
    }
}

impl Write for BoundedHashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.written = self
            .written
            .checked_add(bytes.len())
            .filter(|written| *written <= self.limit)
            .ok_or_else(|| io::Error::other("task rebaseline JSON budget exceeded"))?;
        self.hasher.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn strongly_connected_components(graph: &BTreeMap<TaskId, Vec<TaskId>>) -> Vec<Vec<TaskId>> {
    let mut visited = BTreeSet::new();
    let mut finish = Vec::new();
    for start in graph.keys().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut stack = vec![(start, 0_usize)];
        while let Some((node, next)) = stack.last_mut() {
            let neighbors = graph.get(node).map_or(&[][..], Vec::as_slice);
            if *next < neighbors.len() {
                let candidate = neighbors[*next];
                *next += 1;
                if visited.insert(candidate) {
                    stack.push((candidate, 0));
                }
            } else {
                finish.push(*node);
                stack.pop();
            }
        }
    }
    let mut reverse = graph
        .keys()
        .copied()
        .map(|task_id| (task_id, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (source, targets) in graph {
        for target in targets {
            reverse.entry(*target).or_default().push(*source);
        }
    }
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for start in finish.into_iter().rev() {
        if !assigned.insert(start) {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![start];
        while let Some(node) = stack.pop() {
            component.push(node);
            for next in reverse.get(&node).map_or(&[][..], Vec::as_slice) {
                if assigned.insert(*next) {
                    stack.push(*next);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components
}

fn relative_locator(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()
        })
        .map(|parts| parts.join("/"))
        .unwrap_or_default()
}

fn valid_reviewed_text(value: &str, maximum_bytes: usize, allow_edge_whitespace: bool) -> bool {
    !value.is_empty()
        && value.len() <= maximum_bytes
        && (allow_edge_whitespace || value.trim_matches(char::is_whitespace) == value)
        && !value.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '\u{061c}'
                        | '\u{200e}'
                        | '\u{200f}'
                        | '\u{202a}'..='\u{202e}'
                        | '\u{2066}'..='\u{206f}'
                )
        })
}

fn contains_offset(ranges: &[Range<u64>], offset: usize) -> bool {
    let offset = u64::try_from(offset).unwrap_or(u64::MAX);
    let insertion = ranges.partition_point(|range| range.start <= offset);
    insertion > 0 && offset < ranges[insertion - 1].end
}

fn usize_range(range: &Range<u64>, source: &str) -> Option<Range<usize>> {
    let start = usize::try_from(range.start).ok()?;
    let end = usize::try_from(range.end).ok()?;
    (start <= end
        && end <= source.len()
        && source.is_char_boundary(start)
        && source.is_char_boundary(end))
    .then_some(start..end)
}

fn to_u64_range(range: Range<usize>) -> Range<u64> {
    u64::try_from(range.start).unwrap_or(u64::MAX)..u64::try_from(range.end).unwrap_or(u64::MAX)
}

fn sort_blockers(blockers: &mut Vec<TaskRebaselineBlocker>) {
    blockers.sort_by(|left, right| {
        left.source_node_id
            .cmp(&right.source_node_id)
            .then_with(|| {
                left.range
                    .as_ref()
                    .map(|range| (range.start, range.end))
                    .cmp(&right.range.as_ref().map(|range| (range.start, range.end)))
            })
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.old_task_id.cmp(&right.old_task_id))
            .then_with(|| left.dependency_task_id.cmp(&right.dependency_task_id))
    });
    blockers.dedup();
}
