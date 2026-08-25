use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use weftext_asciidoc::{
    AnalysisStatus, ChecklistBranchLiftEditKind, ChecklistMarker, ChecklistParserOccurrence,
    ChecklistState, LinkKind, SourceEdit, SourceEditPlan, decode_node_link_label,
    encode_node_link_label,
};

use crate::annotations::{
    AnnotationAnchorMigrationError, ResolvedAnnotationAnchor,
    rebuild_annotation_target_at_exact_range, resolve_annotation_anchor_range,
};
use crate::document::plan_document_edit_from_snapshot;
use crate::workspace_transaction::{
    TaskPromotionDisclosure, TaskPromotionSidecarState, TaskPromotionWorkspaceMaterial,
    acquire_clean_workspace_mutation_guard, commit_workspace_transaction_with_clean_guard,
    plan_task_promotion_workspace_transaction, task_promotion_workspace_plans_match,
    validate_task_promotion_annotation_snapshot,
    validate_workspace_transaction_draft_gate_for_commit,
};
use crate::{
    Anchor, AnnotationReplicaCompleteness, AnnotationSidecarExpectedState,
    AnnotationSidecarSnapshot, AnnotationStore, CommittedWorkspaceTransaction, DocumentEdit,
    DocumentEditPlan, DocumentError, DocumentProfileId, DocumentRevision, DocumentSnapshot, NodeId,
    TaskNodeState, WorkspaceDraftGatePreview, WorkspaceDraftGateToken, WorkspaceDraftRegistryView,
    WorkspaceInventory, WorkspaceReadScope, WorkspaceRevision, WorkspaceTransactionError,
    WorkspaceTransactionPlan, analyze_task_node_profile, parse_node_metadata, read_node_document,
    read_workspace_revision, scan_workspace,
};

const MAX_PROMOTION_TITLE_BYTES: usize = 4_096;
const MAX_PROMOTION_LABEL_BYTES: usize = 4_096;
const MAX_PROMOTED_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

/// Complete revision- and parser-bound authority for one native checklist occurrence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChecklistPromotionEvidence {
    pub source_node_id: NodeId,
    pub document_revision: DocumentRevision,
    pub parser_occurrence: ChecklistParserOccurrence,
    pub authored_marker: ChecklistMarker,
    pub state: ChecklistState,
    pub item_range: Range<u64>,
    pub marker_range: Range<u64>,
    pub description_range: Range<u64>,
    pub description: String,
    pub list_depth: u8,
}

/// Reviewed user choices for promoting one exact native checklist occurrence.
///
/// The generated task-node UUID is deliberately absent. Core generates it once while producing
/// the first plan and reuses that reviewed identity for every commit-time replan.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskPromotionRequest {
    pub evidence: ChecklistPromotionEvidence,
    pub parent_node_id: NodeId,
    pub portable_name: String,
    pub document_title: String,
    pub logical_link_label: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPromotionAnnotationDisposition {
    RetainedInSource,
    MigratedToTaskNode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskPromotionAnnotationDispositionRecord {
    pub annotation_id: uuid::Uuid,
    pub disposition: TaskPromotionAnnotationDisposition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPromotionAnnotationBlockerReason {
    StaleOrInvalidAnchor,
    CrossesPromotionBoundary,
    IntersectsPrincipalOrLiftEdit,
    AmbiguousBoundaryOwnership,
    InexactBlockGeometry,
    DestinationGeometryUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskPromotionAnnotationBlocker {
    pub annotation_id: uuid::Uuid,
    pub reason: TaskPromotionAnnotationBlockerReason,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskPromotionAnnotationSummary {
    pub replica_completeness: AnnotationReplicaCompleteness,
    pub expected_source_sidecar: AnnotationSidecarExpectedState,
    pub retained_in_source_count: u64,
    pub migrated_to_task_node_count: u64,
    pub source_sidecar_rewritten: bool,
    pub task_sidecar_created: bool,
    pub dispositions: Vec<TaskPromotionAnnotationDispositionRecord>,
}

/// Closed promotion-specific impact evidence. This is not a node-branch `scopeSummary`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskPromotionSummary {
    pub workspace_revision: WorkspaceRevision,
    pub source_node_id: NodeId,
    pub source_revision: DocumentRevision,
    pub next_source_revision: DocumentRevision,
    pub source_item_range: Range<u64>,
    pub source_marker_range: Range<u64>,
    pub source_description_range: Range<u64>,
    pub source_replacement_range: Range<u64>,
    pub source_state: ChecklistState,
    pub source_list_depth: u8,
    pub generated_node_id: NodeId,
    pub generated_title: String,
    pub generated_parent_node_id: NodeId,
    pub generated_portable_name: String,
    pub generated_path: String,
    pub initial_state: TaskNodeState,
    pub lifted_descendant_count: u64,
    pub lifted_continuation_count: u64,
    pub lifted_body_bytes: u64,
    pub annotations: TaskPromotionAnnotationSummary,
    pub replacement_link_label: String,
    pub replacement_source: String,
    pub affected_document_node_ids: Vec<NodeId>,
    pub byte_total: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AuthorizationBinding {
    Owner,
    Scoped(WorkspaceReadScope),
}

/// Opaque executable promotion plan.
#[derive(Clone)]
pub struct TaskPromotionPlan {
    request: TaskPromotionRequest,
    summary: TaskPromotionSummary,
    generated_node_id: NodeId,
    source_edit: SourceEdit,
    proposed_source: String,
    task_document_source: String,
    source_document_plan: DocumentEditPlan,
    source_sidecar: PlannedSidecars,
    annotation_sidecar_snapshot: AnnotationSidecarSnapshot,
    transaction: WorkspaceTransactionPlan,
    workspace_root: PathBuf,
    source_node_directory: PathBuf,
    destination_node_directory: PathBuf,
    authorization: AuthorizationBinding,
}

impl fmt::Debug for TaskPromotionPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskPromotionPlan")
            .field("summary", &self.summary)
            .field("source_edit", &self.source_edit)
            .field("authorization", &self.authorization.kind())
            .finish_non_exhaustive()
    }
}

impl TaskPromotionPlan {
    #[must_use]
    pub const fn summary(&self) -> &TaskPromotionSummary {
        &self.summary
    }

    #[must_use]
    pub fn source_edit(&self) -> &SourceEdit {
        &self.source_edit
    }

    #[must_use]
    pub fn proposed_source(&self) -> &str {
        &self.proposed_source
    }

    #[must_use]
    pub fn task_document_source(&self) -> &str {
        &self.task_document_source
    }

    /// Previews the shared authoritative draft gate for the exact promotion impact set.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed draft authority or inconsistent private plan evidence.
    pub fn preview_draft_gate(
        &self,
        registry: &WorkspaceDraftRegistryView,
    ) -> Result<WorkspaceDraftGatePreview, TaskPromotionError> {
        crate::preview_workspace_transaction_draft_gate(&self.transaction, registry)
            .map_err(TaskPromotionError::WorkspaceTransaction)
    }
}

impl AuthorizationBinding {
    const fn kind(&self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Scoped(_) => "scoped",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedSidecars {
    expected_source: TaskPromotionSidecarState,
    proposed_source_bytes: Option<Vec<u8>>,
    proposed_task_bytes: Option<Vec<u8>>,
    source_store: AnnotationStore,
    proposed_source_store: AnnotationStore,
    proposed_task_store: Option<AnnotationStore>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedTaskPromotion {
    pub summary: TaskPromotionSummary,
    pub transaction: CommittedWorkspaceTransaction,
}

#[derive(Debug)]
pub enum TaskPromotionError {
    InvalidRequest,
    InvalidWorkspace,
    InvalidScope,
    TargetUnavailable,
    AuthorizationChanged,
    StaleDocumentRevision,
    StaleWorkspaceRevision,
    ParserEvidenceMismatch,
    IncompletePromotionBranch,
    InvalidPortableName,
    DestinationConflict,
    InvalidDocumentTitle,
    InvalidLinkLabel,
    RelativeLocator,
    DocumentContextDependency,
    PromotedDocumentTooLarge,
    DocumentRead(DocumentError),
    DocumentIdentity,
    SourceEditPlan,
    DocumentPlan(DocumentError),
    AnnotationAuthority,
    AnnotationBlockers {
        blockers: Vec<TaskPromotionAnnotationBlocker>,
    },
    WorkspaceAuthorityUnavailable,
    WorkspaceGuard,
    WorkspaceTransaction(WorkspaceTransactionError),
    RecoveryRequired,
    ReviewedPlanMismatch,
    PostValidation,
}

impl fmt::Display for TaskPromotionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest => formatter.write_str("task promotion request is invalid"),
            Self::InvalidWorkspace => formatter.write_str("workspace inventory is invalid"),
            Self::InvalidScope => formatter.write_str("workspace read scope is invalid"),
            Self::TargetUnavailable => formatter.write_str("task promotion target is unavailable"),
            Self::AuthorizationChanged => {
                formatter.write_str("task promotion authorization changed after review")
            }
            Self::StaleDocumentRevision => {
                formatter.write_str("task promotion source document is stale")
            }
            Self::StaleWorkspaceRevision => {
                formatter.write_str("task promotion workspace is stale")
            }
            Self::ParserEvidenceMismatch => {
                formatter.write_str("task promotion parser evidence does not match exactly once")
            }
            Self::IncompletePromotionBranch => {
                formatter.write_str("checklist branch cannot be promoted losslessly")
            }
            Self::InvalidPortableName => {
                formatter.write_str("task promotion node name is not portable")
            }
            Self::DestinationConflict => {
                formatter.write_str("task promotion destination is occupied")
            }
            Self::InvalidDocumentTitle => {
                formatter.write_str("task promotion document title is not an exact safe header")
            }
            Self::InvalidLinkLabel => formatter.write_str("task promotion link label is invalid"),
            Self::RelativeLocator => formatter.write_str(
                "task promotion body contains a relative locator whose base would change",
            ),
            Self::DocumentContextDependency => formatter.write_str(
                "task promotion body depends on source-document context that cannot be moved losslessly",
            ),
            Self::PromotedDocumentTooLarge => {
                formatter.write_str("promoted task document exceeds the byte limit")
            }
            Self::DocumentRead(error) => write!(formatter, "task promotion read failed: {error}"),
            Self::DocumentIdentity => {
                formatter.write_str("task promotion document identity is inconsistent")
            }
            Self::SourceEditPlan => {
                formatter.write_str("task promotion source replacement is invalid")
            }
            Self::DocumentPlan(error) => write!(formatter, "task promotion edit failed: {error}"),
            Self::AnnotationAuthority => {
                formatter.write_str("task promotion annotation authority is invalid")
            }
            Self::AnnotationBlockers { .. } => {
                formatter.write_str("task promotion has annotation migration blockers")
            }
            Self::WorkspaceAuthorityUnavailable => {
                formatter.write_str("task promotion workspace authority is unavailable")
            }
            Self::WorkspaceGuard => formatter.write_str("workspace mutation guard failed"),
            Self::WorkspaceTransaction(error) => error.fmt(formatter),
            Self::RecoveryRequired => {
                formatter.write_str("workspace transaction recovery is required")
            }
            Self::ReviewedPlanMismatch => {
                formatter.write_str("fresh task promotion differs from the reviewed plan")
            }
            Self::PostValidation => {
                formatter.write_str("committed task promotion failed post-validation")
            }
        }
    }
}

impl std::error::Error for TaskPromotionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DocumentRead(error) | Self::DocumentPlan(error) => Some(error),
            Self::WorkspaceTransaction(error) => Some(error),
            _ => None,
        }
    }
}

/// Plans an owner-authorized checklist promotion without changing workspace bytes.
///
/// # Errors
///
/// Returns a typed error for invalid request/parser/annotation/destination/workspace authority.
pub fn plan_task_promotion_transaction(
    root: impl AsRef<Path>,
    request: &TaskPromotionRequest,
    annotation_sidecar_snapshot: &AnnotationSidecarSnapshot,
) -> Result<TaskPromotionPlan, TaskPromotionError> {
    plan_internal(
        root.as_ref(),
        request,
        annotation_sidecar_snapshot,
        None,
        None,
        None,
    )
}

/// Plans checklist promotion using one already-authorized source/parent projection.
///
/// # Errors
///
/// Hidden, missing, invalid, and unauthorized source or parent targets share one non-disclosing
/// `TargetUnavailable` result and no unauthorized document body is opened.
pub fn plan_task_promotion_transaction_scoped(
    root: impl AsRef<Path>,
    request: &TaskPromotionRequest,
    annotation_sidecar_snapshot: &AnnotationSidecarSnapshot,
    scope: &WorkspaceReadScope,
) -> Result<TaskPromotionPlan, TaskPromotionError> {
    plan_internal(
        root.as_ref(),
        request,
        annotation_sidecar_snapshot,
        Some(scope),
        None,
        None,
    )
}

/// Persists this promotion's authentic prepared v2 journal without applying it.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn prepare_task_promotion_recovery_fixture(
    plan: &TaskPromotionPlan,
) -> Result<PathBuf, TaskPromotionError> {
    crate::prepare_workspace_transaction_recovery_fixture(&plan.transaction)
        .map_err(TaskPromotionError::WorkspaceTransaction)
}

/// Commits through the production journal while injecting a crash at one step boundary.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn commit_task_promotion_with_injected_failure_for_recovery_fixture(
    plan: &TaskPromotionPlan,
    fail_after_steps: usize,
) -> Result<CommittedWorkspaceTransaction, TaskPromotionError> {
    crate::workspace_transaction::commit_workspace_transaction_with_injected_failure_for_recovery_fixture(
        &plan.transaction,
        fail_after_steps,
    )
    .map_err(TaskPromotionError::WorkspaceTransaction)
}

/// Injects a final semantic verification failure after all journal steps are applied.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn commit_task_promotion_with_injected_verification_failure_for_recovery_fixture(
    plan: &TaskPromotionPlan,
) -> Result<CommittedWorkspaceTransaction, TaskPromotionError> {
    crate::workspace_transaction::commit_workspace_transaction_with_injected_verification_failure_for_recovery_fixture(
        &plan.transaction,
    )
    .map_err(TaskPromotionError::WorkspaceTransaction)
}

/// Leaves an authentic applying v2 journal after exactly `applied_steps` for startup recovery.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn prepare_task_promotion_applying_recovery_fixture(
    plan: &TaskPromotionPlan,
    applied_steps: usize,
) -> Result<PathBuf, TaskPromotionError> {
    crate::workspace_transaction::prepare_workspace_transaction_applying_recovery_fixture(
        &plan.transaction,
        applied_steps,
    )
    .map_err(TaskPromotionError::WorkspaceTransaction)
}

/// Leaves an authentic committed v2 journal after applying and verifying every promotion step.
#[cfg(debug_assertions)]
#[doc(hidden)]
pub fn prepare_task_promotion_committed_recovery_fixture(
    plan: &TaskPromotionPlan,
) -> Result<PathBuf, TaskPromotionError> {
    crate::workspace_transaction::prepare_workspace_transaction_committed_recovery_fixture(
        &plan.transaction,
    )
    .map_err(TaskPromotionError::WorkspaceTransaction)
}

/// Commits an owner plan using Core's explicit empty draft-registry compatibility authority.
///
/// # Errors
///
/// Returns an error for request/authority changes, stale authority, recovery, or transaction
/// failure. The request is rechecked before filesystem I/O.
pub fn commit_task_promotion_transaction(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
) -> Result<CommittedTaskPromotion, TaskPromotionError> {
    require_owner(plan, request)?;
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let preview = plan.preview_draft_gate(&registry)?;
    let token = preview.executable_token.as_ref().ok_or_else(|| {
        TaskPromotionError::WorkspaceTransaction(WorkspaceTransactionError::DraftGateBlocked(
            Vec::new(),
        ))
    })?;
    commit_internal(plan, request, None, token, &registry)
}

/// Commits an owner plan with an explicit draft gate token and fresh registry observation.
///
/// # Errors
///
/// Returns ordinary commit errors plus draft blocker/authority failures.
pub fn commit_task_promotion_transaction_with_draft_gate(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
    token: &WorkspaceDraftGateToken,
    registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedTaskPromotion, TaskPromotionError> {
    require_owner(plan, request)?;
    commit_internal(plan, request, None, token, registry)
}

/// Commits a scoped plan only when the request and complete scope equal reviewed authority.
///
/// # Errors
///
/// Authorization mismatch is returned before filesystem I/O.
pub fn commit_task_promotion_transaction_scoped(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
    fresh_scope: &WorkspaceReadScope,
) -> Result<CommittedTaskPromotion, TaskPromotionError> {
    require_scoped(plan, request, fresh_scope)?;
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let preview = plan.preview_draft_gate(&registry)?;
    let token = preview.executable_token.as_ref().ok_or_else(|| {
        TaskPromotionError::WorkspaceTransaction(WorkspaceTransactionError::DraftGateBlocked(
            Vec::new(),
        ))
    })?;
    commit_internal(plan, request, Some(fresh_scope), token, &registry)
}

/// Scoped explicit-draft counterpart of
/// [`commit_task_promotion_transaction_with_draft_gate`].
///
/// # Errors
///
/// Returns authorization, draft, stale-plan, recovery, or transaction failures.
pub fn commit_task_promotion_transaction_scoped_with_draft_gate(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
    fresh_scope: &WorkspaceReadScope,
    token: &WorkspaceDraftGateToken,
    registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedTaskPromotion, TaskPromotionError> {
    require_scoped(plan, request, fresh_scope)?;
    commit_internal(plan, request, Some(fresh_scope), token, registry)
}

fn commit_internal(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
    scope: Option<&WorkspaceReadScope>,
    token: &WorkspaceDraftGateToken,
    registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedTaskPromotion, TaskPromotionError> {
    let guard = acquire_clean_workspace_mutation_guard(&plan.workspace_root)
        .map_err(|error| map_guard_error(&error))?;
    let fresh = plan_internal(
        &plan.workspace_root,
        request,
        &plan.annotation_sidecar_snapshot,
        scope,
        Some(&plan.summary.workspace_revision),
        Some(plan.generated_node_id),
    )?;
    if !reviewed_plan_matches(plan, &fresh) {
        return Err(TaskPromotionError::ReviewedPlanMismatch);
    }
    // `registry` must be the caller's current view from its draft-store commit boundary. Core's
    // filesystem lease cannot itself lock a device/session draft registry, so this final lookup is
    // deliberately adjacent to journal application after the complete fresh replan.
    validate_workspace_transaction_draft_gate_for_commit(&plan.transaction, token, registry)
        .map_err(TaskPromotionError::WorkspaceTransaction)?;
    let transaction = commit_workspace_transaction_with_clean_guard(&plan.transaction, &guard)
        .map_err(|error| map_commit_error(error, scope))?;
    drop(guard);
    Ok(CommittedTaskPromotion {
        summary: plan.summary.clone(),
        transaction,
    })
}

#[allow(clippy::too_many_lines)]
fn plan_internal(
    root: &Path,
    request: &TaskPromotionRequest,
    annotation_sidecar_snapshot: &AnnotationSidecarSnapshot,
    scope: Option<&WorkspaceReadScope>,
    expected_workspace_revision: Option<&WorkspaceRevision>,
    reviewed_generated_node_id: Option<NodeId>,
) -> Result<TaskPromotionPlan, TaskPromotionError> {
    validate_request(request)?;
    let inventory = authorized_inventory(root, request, scope)?;
    let source_node = locate_active_node(&inventory, request.evidence.source_node_id)?;
    let parent_node = locate_active_node(&inventory, request.parent_node_id)?;
    let source_node_directory = source_node.path.clone();
    let parent_node_directory = parent_node.path.clone();
    let first_workspace_revision = workspace_revision(root, scope)?;
    if expected_workspace_revision.is_some_and(|expected| expected != &first_workspace_revision) {
        return Err(TaskPromotionError::StaleWorkspaceRevision);
    }
    let source_snapshot = read_snapshot(&source_node_directory, request.evidence.source_node_id)?;
    if source_snapshot.revision != request.evidence.document_revision {
        return Err(TaskPromotionError::StaleDocumentRevision);
    }
    let source_analysis = weftext_asciidoc::analyze(&source_snapshot.source);
    let occurrences = source_analysis
        .checklists
        .iter()
        .filter(|candidate| occurrence_matches(candidate, &request.evidence))
        .collect::<Vec<_>>();
    let [occurrence] = occurrences.as_slice() else {
        return Err(TaskPromotionError::ParserEvidenceMismatch);
    };
    let promotion = occurrence
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .ok_or(TaskPromotionError::IncompletePromotionBranch)?;
    let generated_node_id = reviewed_generated_node_id.unwrap_or_else(NodeId::new_v4);
    if inventory
        .nodes
        .iter()
        .any(|node| node.id == Some(generated_node_id))
    {
        return Err(TaskPromotionError::DestinationConflict);
    }
    require_destination_available(
        &parent_node_directory,
        &request.portable_name,
        generated_node_id,
    )?;

    let lifted_body = promotion
        .destination_body(&source_snapshot.source)
        .ok_or(TaskPromotionError::IncompletePromotionBranch)?;
    require_location_independent_lifted_body(&lifted_body, promotion)?;
    let initial_state = match occurrence.state {
        ChecklistState::Todo => TaskNodeState::Todo,
        ChecklistState::Completed => TaskNodeState::Completed,
    };
    let (task_document_source, task_body_start) = build_task_document_source(
        generated_node_id,
        &request.document_title,
        initial_state,
        &lifted_body,
    )?;
    validate_task_document(
        &task_document_source,
        task_body_start,
        &lifted_body,
        generated_node_id,
        &request.document_title,
        initial_state,
    )?;

    let encoded_label = encode_node_link_label(&request.logical_link_label)
        .map_err(|_| TaskPromotionError::InvalidLinkLabel)?;
    let replacement_source = build_source_replacement(
        &source_snapshot.source,
        promotion,
        occurrence.list_depth,
        generated_node_id,
        &encoded_label,
    )?;
    let replacement_range =
        usize_range(&promotion.source_replacement_range, &source_snapshot.source)?;
    let source_edit = SourceEdit {
        range: replacement_range,
        replacement: replacement_source.clone(),
    };
    let proposed_source = SourceEditPlan::new(&source_snapshot.source, vec![source_edit.clone()])
        .map_err(|_| TaskPromotionError::SourceEditPlan)?
        .apply(&source_snapshot.source)
        .ok_or(TaskPromotionError::SourceEditPlan)?;
    validate_proposed_source(
        &source_snapshot.source,
        &proposed_source,
        &source_edit,
        occurrence,
        generated_node_id,
        &request.logical_link_label,
    )?;
    let source_document_plan = plan_document_edit_from_snapshot(
        &source_snapshot,
        [DocumentEdit {
            start: promotion.source_replacement_range.start,
            end: promotion.source_replacement_range.end,
            replacement: replacement_source.clone(),
        }],
    )
    .map_err(TaskPromotionError::DocumentPlan)?;
    if !source_document_plan.changed || source_document_plan.next_source() != proposed_source {
        return Err(TaskPromotionError::ReviewedPlanMismatch);
    }

    let (expected_source_sidecar_state, source_annotation_store, replica_completeness) =
        validate_task_promotion_annotation_snapshot(
            root,
            &source_node_directory,
            request.evidence.source_node_id,
            &first_workspace_revision,
            annotation_sidecar_snapshot,
        )
        .map_err(|_| TaskPromotionError::AnnotationAuthority)?;
    let expected_source_sidecar = (expected_source_sidecar_state, source_annotation_store);
    let source_sidecar = plan_annotation_migration(
        &source_snapshot,
        &proposed_source,
        &task_document_source,
        task_body_start,
        promotion,
        generated_node_id,
        expected_source_sidecar,
    )?;
    let destination_node_directory = parent_node_directory.join(&request.portable_name);
    let physical_generated_path =
        portable_relative_path(&inventory.root, &destination_node_directory)?;
    let generated_path = match scope {
        Some(scope) => {
            let parent_locator = scope
                .locator(request.parent_node_id)
                .ok_or(TaskPromotionError::TargetUnavailable)?;
            if parent_locator.is_empty() {
                request.portable_name.clone()
            } else {
                format!("{parent_locator}/{}", request.portable_name)
            }
        }
        None => physical_generated_path.clone(),
    };
    let final_workspace_revision = workspace_revision(root, scope)?;
    if first_workspace_revision != final_workspace_revision {
        return Err(TaskPromotionError::StaleWorkspaceRevision);
    }
    require_destination_available(
        &parent_node_directory,
        &request.portable_name,
        generated_node_id,
    )?;

    let mut dispositions = source_sidecar
        .proposed_source_store
        .annotations
        .iter()
        .map(|annotation| TaskPromotionAnnotationDispositionRecord {
            annotation_id: annotation.id,
            disposition: TaskPromotionAnnotationDisposition::RetainedInSource,
        })
        .chain(
            source_sidecar
                .proposed_task_store
                .iter()
                .flat_map(|store| &store.annotations)
                .map(|annotation| TaskPromotionAnnotationDispositionRecord {
                    annotation_id: annotation.id,
                    disposition: TaskPromotionAnnotationDisposition::MigratedToTaskNode,
                }),
        )
        .collect::<Vec<_>>();
    dispositions.sort_by_key(|record| record.annotation_id);
    let annotations = TaskPromotionAnnotationSummary {
        replica_completeness,
        expected_source_sidecar: promotion_expected_sidecar_state(&source_sidecar.expected_source),
        retained_in_source_count: to_u64(source_sidecar.proposed_source_store.annotations.len()),
        migrated_to_task_node_count: to_u64(
            source_sidecar
                .proposed_task_store
                .as_ref()
                .map_or(0, |store| store.annotations.len()),
        ),
        source_sidecar_rewritten: source_sidecar.proposed_source_bytes.is_some(),
        task_sidecar_created: source_sidecar.proposed_task_bytes.is_some(),
        dispositions,
    };
    let mut affected_document_node_ids = vec![request.evidence.source_node_id, generated_node_id];
    affected_document_node_ids.sort_unstable();
    let byte_total = to_u64(proposed_source.len())
        .saturating_add(to_u64(task_document_source.len()))
        .saturating_add(
            source_sidecar
                .proposed_source_bytes
                .as_ref()
                .map_or(0, |bytes| to_u64(bytes.len())),
        )
        .saturating_add(
            source_sidecar
                .proposed_task_bytes
                .as_ref()
                .map_or(0, |bytes| to_u64(bytes.len())),
        );
    let summary = TaskPromotionSummary {
        workspace_revision: final_workspace_revision.clone(),
        source_node_id: request.evidence.source_node_id,
        source_revision: source_snapshot.revision.clone(),
        next_source_revision: source_document_plan.next_revision.clone(),
        source_item_range: occurrence.item_range.clone(),
        source_marker_range: occurrence.marker_range.clone(),
        source_description_range: occurrence.description_range.clone(),
        source_replacement_range: promotion.source_replacement_range.clone(),
        source_state: occurrence.state,
        source_list_depth: occurrence.list_depth,
        generated_node_id,
        generated_title: request.document_title.clone(),
        generated_parent_node_id: request.parent_node_id,
        generated_portable_name: request.portable_name.clone(),
        generated_path: generated_path.clone(),
        initial_state,
        lifted_descendant_count: u64::from(promotion.lifted_descendant_count),
        lifted_continuation_count: u64::from(promotion.lifted_continuation_count),
        lifted_body_bytes: to_u64(lifted_body.len()),
        annotations,
        replacement_link_label: request.logical_link_label.clone(),
        replacement_source,
        affected_document_node_ids,
        byte_total,
    };
    let physical_path = |path: &Path| {
        std::fs::canonicalize(path).map_err(|error| {
            if scope.is_some() {
                TaskPromotionError::WorkspaceAuthorityUnavailable
            } else {
                TaskPromotionError::WorkspaceTransaction(WorkspaceTransactionError::Io(error))
            }
        })
    };
    let physical_root = physical_path(&inventory.root)?;
    let physical_source_node_directory = physical_path(&source_node_directory)?;
    let physical_parent_node_directory = physical_path(&parent_node_directory)?;
    let physical_destination_node_directory =
        physical_parent_node_directory.join(&request.portable_name);
    let mut physical_source_document_plan = source_document_plan.clone();
    physical_source_document_plan
        .node_directory
        .clone_from(&physical_source_node_directory);
    physical_source_document_plan.document_path =
        physical_path(&source_document_plan.document_path)?;
    let transaction = plan_task_promotion_workspace_transaction(TaskPromotionWorkspaceMaterial {
        root: physical_root,
        workspace_revision: final_workspace_revision,
        summary: summary.clone(),
        source_node_directory: physical_source_node_directory,
        source_document_plan: physical_source_document_plan,
        source_base: source_snapshot.source.clone(),
        destination_node_directory: physical_destination_node_directory,
        task_document_source: task_document_source.clone(),
        annotation_replica_completeness: replica_completeness,
        expected_source_sidecar: source_sidecar.expected_source.clone(),
        source_sidecar_bytes: source_sidecar.proposed_source_bytes.clone(),
        task_sidecar_bytes: source_sidecar.proposed_task_bytes.clone(),
        disclosure: if scope.is_some() {
            TaskPromotionDisclosure::Scoped
        } else {
            TaskPromotionDisclosure::Owner
        },
    })
    .map_err(|error| map_planning_workspace_error(error, scope))?;

    Ok(TaskPromotionPlan {
        request: request.clone(),
        summary,
        generated_node_id,
        source_edit,
        proposed_source,
        task_document_source,
        source_document_plan,
        source_sidecar,
        annotation_sidecar_snapshot: annotation_sidecar_snapshot.clone(),
        transaction,
        workspace_root: inventory.root,
        source_node_directory,
        destination_node_directory,
        authorization: scope.map_or(AuthorizationBinding::Owner, |scope| {
            AuthorizationBinding::Scoped(scope.clone())
        }),
    })
}

fn validate_request(request: &TaskPromotionRequest) -> Result<(), TaskPromotionError> {
    crate::portable_name::validate_portable_node_name(&request.portable_name, false)
        .map_err(|_| TaskPromotionError::InvalidPortableName)?;
    validate_reviewed_text(&request.document_title, MAX_PROMOTION_TITLE_BYTES, false)
        .map_err(|()| TaskPromotionError::InvalidDocumentTitle)?;
    validate_reviewed_text(&request.logical_link_label, MAX_PROMOTION_LABEL_BYTES, true)
        .map_err(|()| TaskPromotionError::InvalidLinkLabel)?;
    encode_node_link_label(&request.logical_link_label)
        .map_err(|_| TaskPromotionError::InvalidLinkLabel)?;
    let evidence = &request.evidence;
    if evidence.list_depth == 0
        || evidence.description.is_empty()
        || evidence.item_range.start > evidence.item_range.end
        || evidence.marker_range.start > evidence.marker_range.end
        || evidence.description_range.start > evidence.description_range.end
        || !evidence.parser_occurrence.branch_complete
        || evidence.parser_occurrence.promotion_branch.is_none()
        || evidence.parser_occurrence.branch_range
            != evidence
                .parser_occurrence
                .promotion_branch
                .as_ref()
                .map(|promotion| promotion.source_replacement_range.clone())
        || marker_state(evidence.authored_marker) != evidence.state
    {
        return Err(TaskPromotionError::InvalidRequest);
    }
    Ok(())
}

fn validate_reviewed_text(
    value: &str,
    maximum_bytes: usize,
    allow_edge_whitespace: bool,
) -> Result<(), ()> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || (!allow_edge_whitespace && value.trim_matches(char::is_whitespace) != value)
        || value.chars().any(|character| {
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
    {
        Err(())
    } else {
        Ok(())
    }
}

const fn marker_state(marker: ChecklistMarker) -> ChecklistState {
    match marker {
        ChecklistMarker::Open => ChecklistState::Todo,
        ChecklistMarker::CheckedX | ChecklistMarker::CheckedStar => ChecklistState::Completed,
    }
}

fn authorized_inventory(
    root: &Path,
    request: &TaskPromotionRequest,
    scope: Option<&WorkspaceReadScope>,
) -> Result<WorkspaceInventory, TaskPromotionError> {
    let inventory = scan_workspace(root);
    if let Some(scope) = scope {
        scope
            .validate_inventory(&inventory)
            .map_err(|_| TaskPromotionError::InvalidScope)?;
        if !scope.allows(request.evidence.source_node_id) || !scope.allows(request.parent_node_id) {
            return Err(TaskPromotionError::TargetUnavailable);
        }
    } else if !inventory.is_valid() {
        return Err(TaskPromotionError::InvalidWorkspace);
    }
    Ok(inventory)
}

fn locate_active_node(
    inventory: &WorkspaceInventory,
    node_id: NodeId,
) -> Result<&crate::NodeRecord, TaskPromotionError> {
    let matching = inventory
        .nodes
        .iter()
        .filter(|node| {
            node.id == Some(node_id)
                && node.metadata.is_some()
                && !crate::workspace_trash::is_trash_storage_path(&inventory.root, &node.path)
                && node.name != crate::TRASH_NODE_NAME
        })
        .collect::<Vec<_>>();
    let [node] = matching.as_slice() else {
        return Err(TaskPromotionError::TargetUnavailable);
    };
    Ok(node)
}

fn read_snapshot(
    node_directory: &Path,
    expected_node_id: NodeId,
) -> Result<DocumentSnapshot, TaskPromotionError> {
    let snapshot = read_node_document(node_directory).map_err(TaskPromotionError::DocumentRead)?;
    if snapshot.node_id != expected_node_id || snapshot.node_directory != node_directory {
        return Err(TaskPromotionError::DocumentIdentity);
    }
    Ok(snapshot)
}

fn occurrence_matches(
    candidate: &weftext_asciidoc::ChecklistEvidence,
    reviewed: &ChecklistPromotionEvidence,
) -> bool {
    candidate.authored_marker == reviewed.authored_marker
        && candidate.state == reviewed.state
        && candidate.item_range == reviewed.item_range
        && candidate.marker_range == reviewed.marker_range
        && candidate.description_range == reviewed.description_range
        && candidate.description == reviewed.description
        && candidate.list_depth == reviewed.list_depth
        && candidate.parser_occurrence == reviewed.parser_occurrence
}

fn require_destination_available(
    parent: &Path,
    reviewed_name: &str,
    generated_node_id: NodeId,
) -> Result<(), TaskPromotionError> {
    crate::portable_name::validate_portable_node_name(reviewed_name, false)
        .map_err(|_| TaskPromotionError::InvalidPortableName)?;
    let reviewed_key = crate::portable_name::portable_name_collision_key(reviewed_name);
    for entry in std::fs::read_dir(parent).map_err(|_| TaskPromotionError::DestinationConflict)? {
        let entry = entry.map_err(|_| TaskPromotionError::DestinationConflict)?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| TaskPromotionError::DestinationConflict)?;
        if crate::portable_name::portable_name_collision_key(&name) == reviewed_key {
            return Err(TaskPromotionError::DestinationConflict);
        }
    }
    if parent.join(generated_node_id.to_string()).exists() {
        return Err(TaskPromotionError::DestinationConflict);
    }
    Ok(())
}

fn build_task_document_source(
    generated_node_id: NodeId,
    title: &str,
    state: TaskNodeState,
    body: &str,
) -> Result<(String, usize), TaskPromotionError> {
    match state {
        TaskNodeState::Todo | TaskNodeState::Completed => {}
        _ => return Err(TaskPromotionError::InvalidRequest),
    }
    let profile = crate::TaskNodeProfile {
        profile: crate::TaskNodeProfileVersion::V1,
        state,
        priority: None,
        created: None,
        start: None,
        scheduled: None,
        due: None,
        closed: None,
        depends_on: Vec::new(),
    };
    let (source, body_start) =
        crate::task_node::build_task_node_document_source(generated_node_id, title, &profile, body)
            .ok_or(TaskPromotionError::InvalidRequest)?;
    if source.len() > MAX_PROMOTED_DOCUMENT_BYTES {
        return Err(TaskPromotionError::PromotedDocumentTooLarge);
    }
    Ok((source, body_start))
}

fn validate_task_document(
    source: &str,
    body_start: usize,
    expected_body: &str,
    generated_node_id: NodeId,
    expected_title: &str,
    expected_state: TaskNodeState,
) -> Result<(), TaskPromotionError> {
    if source.get(body_start..) != Some(expected_body) {
        return Err(TaskPromotionError::PostValidation);
    }
    let metadata = parse_node_metadata(source).map_err(|_| TaskPromotionError::PostValidation)?;
    if metadata.id != Some(generated_node_id) {
        return Err(TaskPromotionError::PostValidation);
    }
    let parser = weftext_asciidoc::analyze(source);
    if parser.status == AnalysisStatus::Failed {
        return Err(TaskPromotionError::PostValidation);
    }
    let profile = analyze_task_node_profile(source, Some(generated_node_id));
    let Some(decoded) = profile.profile else {
        return Err(TaskPromotionError::PostValidation);
    };
    if !profile.diagnostics.is_empty()
        || profile.title.as_ref().map(|title| title.title.as_str()) != Some(expected_title)
        || decoded.state != expected_state
        || decoded.closed.is_some()
        || !decoded.depends_on.is_empty()
    {
        return Err(TaskPromotionError::InvalidDocumentTitle);
    }
    Ok(())
}

fn require_location_independent_lifted_body(
    body: &str,
    promotion: &weftext_asciidoc::ChecklistPromotionBranchEvidence,
) -> Result<(), TaskPromotionError> {
    if weftext_asciidoc::analyze(body).status == AnalysisStatus::Failed {
        return Err(TaskPromotionError::IncompletePromotionBranch);
    }
    if promotion.context_dependencies.iter().any(|dependency| {
        dependency.kind
            == weftext_asciidoc::ChecklistPromotionContextDependencyKind::RelativeLocator
    }) {
        return Err(TaskPromotionError::RelativeLocator);
    }
    if promotion.context_dependencies.is_empty() {
        Ok(())
    } else {
        Err(TaskPromotionError::DocumentContextDependency)
    }
}

pub(crate) fn build_source_replacement(
    source: &str,
    promotion: &weftext_asciidoc::ChecklistPromotionBranchEvidence,
    list_depth: u8,
    generated_node_id: NodeId,
    encoded_label: &str,
) -> Result<String, TaskPromotionError> {
    let principal = promotion
        .lift_edits
        .iter()
        .find(|edit| edit.kind == ChecklistBranchLiftEditKind::OmitPrincipal)
        .ok_or(TaskPromotionError::IncompletePromotionBranch)?;
    let principal_range = usize_range(&principal.range, source)?;
    let _ = source
        .get(principal_range)
        .ok_or(TaskPromotionError::IncompletePromotionBranch)?;
    let branch_range = usize_range(&promotion.source_replacement_range, source)?;
    let suffix = physical_eol_suffix(
        source
            .get(branch_range)
            .ok_or(TaskPromotionError::IncompletePromotionBranch)?,
    );
    Ok(format!(
        "{} node:{}[{}]{}",
        "*".repeat(usize::from(list_depth)),
        generated_node_id,
        encoded_label,
        suffix
    ))
}

fn physical_eol_suffix(principal: &str) -> &'static str {
    if principal.ends_with("\r\n") {
        "\r\n"
    } else if principal.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn validate_proposed_source(
    original: &str,
    proposed: &str,
    edit: &SourceEdit,
    selected: &weftext_asciidoc::ChecklistEvidence,
    generated_node_id: NodeId,
    reviewed_label: &str,
) -> Result<(), TaskPromotionError> {
    if original.get(..edit.range.start) != proposed.get(..edit.range.start) {
        return Err(TaskPromotionError::PostValidation);
    }
    let proposed_suffix = edit.range.start.saturating_add(edit.replacement.len());
    if original.get(edit.range.end..) != proposed.get(proposed_suffix..) {
        return Err(TaskPromotionError::PostValidation);
    }
    let parser = weftext_asciidoc::analyze(proposed);
    if parser.status == AnalysisStatus::Failed {
        return Err(TaskPromotionError::PostValidation);
    }
    if parser.checklists.iter().any(|occurrence| {
        occurrence.parser_occurrence.parser_ordinal_path
            == selected.parser_occurrence.parser_ordinal_path
            && occurrence.item_range.start == u64::try_from(edit.range.start).unwrap_or(u64::MAX)
    }) {
        return Err(TaskPromotionError::PostValidation);
    }
    let branch = selected
        .parser_occurrence
        .promotion_branch
        .as_ref()
        .ok_or(TaskPromotionError::PostValidation)?
        .source_replacement_range
        .clone();
    let delta = i128::try_from(edit.replacement.len())
        .ok()
        .and_then(|replacement| {
            let removed = i128::try_from(edit.range.end.saturating_sub(edit.range.start)).ok()?;
            Some(replacement - removed)
        })
        .ok_or(TaskPromotionError::PostValidation)?;
    let original_parser = weftext_asciidoc::analyze(original);
    let expected_checklists = original_parser
        .checklists
        .iter()
        .filter(|occurrence| {
            occurrence.item_range.end <= branch.start || occurrence.item_range.start >= branch.end
        })
        .cloned()
        .map(|mut occurrence| {
            if occurrence.item_range.start >= branch.end {
                shift_checklist_evidence(&mut occurrence, delta)?;
            }
            Some(occurrence)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(TaskPromotionError::PostValidation)?;
    if parser.checklists != expected_checklists {
        return Err(TaskPromotionError::PostValidation);
    }
    let links = parser
        .links
        .iter()
        .filter(|link| {
            link.kind == LinkKind::Node
                && link.target == generated_node_id.to_string()
                && link.range.start
                    == u64::try_from(edit.range.start).unwrap_or(u64::MAX)
                        + u64::from(selected.list_depth)
                        + 1
        })
        .collect::<Vec<_>>();
    let [link] = links.as_slice() else {
        return Err(TaskPromotionError::PostValidation);
    };
    let label_range = usize_range(&link.label_range, proposed)?;
    let decoded = decode_node_link_label(
        proposed
            .get(label_range)
            .ok_or(TaskPromotionError::PostValidation)?,
    )
    .map_err(|_| TaskPromotionError::PostValidation)?;
    if decoded != reviewed_label
        || link.display.as_deref() != Some(reviewed_label)
        || link.fragment.is_some()
    {
        return Err(TaskPromotionError::PostValidation);
    }
    Ok(())
}

fn shift_checklist_evidence(
    evidence: &mut weftext_asciidoc::ChecklistEvidence,
    delta: i128,
) -> Option<()> {
    shift_range(&mut evidence.item_range, delta)?;
    shift_range(&mut evidence.marker_range, delta)?;
    shift_range(&mut evidence.description_range, delta)?;
    if let Some(range) = &mut evidence.parser_occurrence.branch_range {
        shift_range(range, delta)?;
    }
    if let Some(promotion) = &mut evidence.parser_occurrence.promotion_branch {
        shift_range(&mut promotion.source_replacement_range, delta)?;
        for edit in &mut promotion.lift_edits {
            shift_range(&mut edit.range, delta)?;
        }
    }
    Some(())
}

fn shift_range(range: &mut Range<u64>, delta: i128) -> Option<()> {
    range.start = u64::try_from(i128::from(range.start) + delta).ok()?;
    range.end = u64::try_from(i128::from(range.end) + delta).ok()?;
    Some(())
}

#[allow(clippy::too_many_lines)]
fn plan_annotation_migration(
    source_snapshot: &DocumentSnapshot,
    proposed_source: &str,
    task_source: &str,
    task_body_start: usize,
    promotion: &weftext_asciidoc::ChecklistPromotionBranchEvidence,
    generated_node_id: NodeId,
    observed: (TaskPromotionSidecarState, AnnotationStore),
) -> Result<PlannedSidecars, TaskPromotionError> {
    let (expected_source, source_store) = observed;
    let source_revision = &source_snapshot.revision;
    let proposed_source_revision = DocumentRevision::from_source(proposed_source);
    let task_revision = DocumentRevision::from_source(task_source);
    let mut proposed_source_store = AnnotationStore::empty(source_snapshot.node_id);
    let mut proposed_task_store = AnnotationStore::empty(generated_node_id);
    let mut blockers = Vec::new();

    for annotation in &source_store.annotations {
        if annotation.state == crate::AnnotationState::Orphaned {
            proposed_source_store.annotations.push(annotation.clone());
            continue;
        }
        match &annotation.target {
            Anchor::Document | Anchor::ResourceRegion { .. } => {
                proposed_source_store.annotations.push(annotation.clone());
            }
            _ => {
                let Ok(resolved) = resolve_annotation_anchor_range(
                    source_snapshot.profile,
                    &source_snapshot.source,
                    source_revision,
                    &annotation.target,
                ) else {
                    blockers.push(TaskPromotionAnnotationBlocker {
                        annotation_id: annotation.id,
                        reason: TaskPromotionAnnotationBlockerReason::StaleOrInvalidAnchor,
                    });
                    continue;
                };
                match annotation_destination(
                    &resolved,
                    promotion,
                    source_snapshot.source.len(),
                    proposed_source,
                    task_source,
                    task_body_start,
                ) {
                    Ok(AnnotationDestination::Source(destination)) => {
                        match rebuild_annotation_target_at_exact_range(
                            source_snapshot.profile,
                            proposed_source,
                            &proposed_source_revision,
                            &resolved,
                            &destination,
                        ) {
                            Ok(target) => {
                                let mut rebuilt = annotation.clone();
                                rebuilt.target = target;
                                proposed_source_store.annotations.push(rebuilt);
                            }
                            Err(error) => blockers.push(TaskPromotionAnnotationBlocker {
                                annotation_id: annotation.id,
                                reason: blocker_from_rebuild_error(error, &resolved),
                            }),
                        }
                    }
                    Ok(AnnotationDestination::Task(destination)) => {
                        match rebuild_annotation_target_at_exact_range(
                            DocumentProfileId::AsciiDocV1,
                            task_source,
                            &task_revision,
                            &resolved,
                            &destination,
                        ) {
                            Ok(target) => {
                                let mut rebuilt = annotation.clone();
                                rebuilt.target = target;
                                proposed_task_store.annotations.push(rebuilt);
                            }
                            Err(error) => blockers.push(TaskPromotionAnnotationBlocker {
                                annotation_id: annotation.id,
                                reason: blocker_from_rebuild_error(error, &resolved),
                            }),
                        }
                    }
                    Err(reason) => blockers.push(TaskPromotionAnnotationBlocker {
                        annotation_id: annotation.id,
                        reason,
                    }),
                }
            }
        }
    }
    if !blockers.is_empty() {
        return Err(TaskPromotionError::AnnotationBlockers { blockers });
    }
    validate_annotation_partition(
        &source_store,
        &proposed_source_store,
        &proposed_task_store,
        &proposed_source_revision,
        &task_revision,
    )?;
    proposed_source_store
        .validate(source_snapshot.node_id)
        .map_err(|_| TaskPromotionError::AnnotationAuthority)?;
    let source_changed = proposed_source_store != source_store;
    let proposed_source_bytes = source_changed
        .then(|| proposed_source_store.to_pretty_json())
        .transpose()
        .map_err(|_| TaskPromotionError::AnnotationAuthority)?
        .map(String::into_bytes);
    let proposed_task_store =
        (!proposed_task_store.annotations.is_empty()).then_some(proposed_task_store);
    let proposed_task_bytes = proposed_task_store
        .as_ref()
        .map(AnnotationStore::to_pretty_json)
        .transpose()
        .map_err(|_| TaskPromotionError::AnnotationAuthority)?
        .map(String::into_bytes);
    Ok(PlannedSidecars {
        expected_source,
        proposed_source_bytes,
        proposed_task_bytes,
        source_store,
        proposed_source_store,
        proposed_task_store,
    })
}

fn validate_annotation_partition(
    before: &AnnotationStore,
    source_after: &AnnotationStore,
    task_after: &AnnotationStore,
    source_revision: &DocumentRevision,
    task_revision: &DocumentRevision,
) -> Result<(), TaskPromotionError> {
    let mut before_ids = before
        .annotations
        .iter()
        .map(|annotation| annotation.id)
        .collect::<Vec<_>>();
    let mut after = source_after
        .annotations
        .iter()
        .map(|annotation| (annotation, source_revision, true))
        .chain(
            task_after
                .annotations
                .iter()
                .map(|annotation| (annotation, task_revision, false)),
        )
        .collect::<Vec<_>>();
    let mut after_ids = after
        .iter()
        .map(|(annotation, _, _)| annotation.id)
        .collect::<Vec<_>>();
    before_ids.sort_unstable();
    after_ids.sort_unstable();
    if before_ids != after_ids || after_ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(TaskPromotionError::AnnotationAuthority);
    }
    for (annotation, revision, retained_in_source) in after.drain(..) {
        let original = before
            .annotations
            .iter()
            .find(|candidate| candidate.id == annotation.id)
            .ok_or(TaskPromotionError::AnnotationAuthority)?;
        let preserved_orphan = original.state == crate::AnnotationState::Orphaned
            && retained_in_source
            && original.target == annotation.target;
        if !annotation_equal_except_target(original, annotation)
            || (!preserved_orphan
                && anchor_revision(&annotation.target)
                    .is_some_and(|value| value != revision.as_str()))
        {
            return Err(TaskPromotionError::AnnotationAuthority);
        }
    }
    Ok(())
}

fn annotation_equal_except_target(left: &crate::Annotation, right: &crate::Annotation) -> bool {
    left.id == right.id
        && left.kind == right.kind
        && left.appearance == right.appearance
        && left.suggested_source == right.suggested_source
        && left.labels == right.labels
        && left.thread == right.thread
        && left.state == right.state
        && left.resolution == right.resolution
        && left.created_at == right.created_at
        && left.updated_at == right.updated_at
}

fn anchor_revision(anchor: &Anchor) -> Option<&str> {
    match anchor {
        Anchor::TextRange { base_revision, .. }
        | Anchor::InsertionPoint { base_revision, .. }
        | Anchor::Block { base_revision, .. } => Some(base_revision),
        Anchor::Document | Anchor::ResourceRegion { .. } => None,
    }
}

enum AnnotationDestination {
    Source(ResolvedAnnotationAnchor),
    Task(ResolvedAnnotationAnchor),
}

fn annotation_destination(
    resolved: &ResolvedAnnotationAnchor,
    promotion: &weftext_asciidoc::ChecklistPromotionBranchEvidence,
    source_len: usize,
    proposed_source: &str,
    task_source: &str,
    task_body_start: usize,
) -> Result<AnnotationDestination, TaskPromotionAnnotationBlockerReason> {
    let branch = promotion.source_replacement_range.clone();
    match resolved {
        ResolvedAnnotationAnchor::InsertionPoint { position } => {
            if *position == branch.start || *position == branch.end {
                return Err(TaskPromotionAnnotationBlockerReason::AmbiguousBoundaryOwnership);
            }
            if *position < branch.start {
                return Ok(AnnotationDestination::Source(
                    ResolvedAnnotationAnchor::InsertionPoint {
                        position: *position,
                    },
                ));
            }
            if *position > branch.end {
                let mapped = transform_after_replacement(
                    *position,
                    &branch,
                    replacement_len(proposed_source, source_len, &branch)?,
                )?;
                return Ok(AnnotationDestination::Source(
                    ResolvedAnnotationAnchor::InsertionPoint { position: mapped },
                ));
            }
            let body_offset = lift_offset(*position, promotion, true)?;
            let task_position = add_offsets(task_body_start, body_offset, task_source.len())?;
            Ok(AnnotationDestination::Task(
                ResolvedAnnotationAnchor::InsertionPoint {
                    position: task_position,
                },
            ))
        }
        ResolvedAnnotationAnchor::TextRange { range }
        | ResolvedAnnotationAnchor::Block { range } => {
            let kind_block = matches!(resolved, ResolvedAnnotationAnchor::Block { .. });
            let destination = if range.end <= branch.start {
                range.clone()
            } else if range.start >= branch.end {
                let replacement_len = replacement_len(proposed_source, source_len, &branch)?;
                transform_range_after_replacement(range, &branch, replacement_len)?
            } else if branch.start <= range.start && range.end <= branch.end {
                if promotion
                    .lift_edits
                    .iter()
                    .any(|edit| range.start < edit.range.end && edit.range.start < range.end)
                {
                    return Err(
                        TaskPromotionAnnotationBlockerReason::IntersectsPrincipalOrLiftEdit,
                    );
                }
                let start = lift_offset(range.start, promotion, false)?;
                let end = lift_offset(range.end, promotion, false)?;
                let start = add_offsets(task_body_start, start, task_source.len())?;
                let end = add_offsets(task_body_start, end, task_source.len())?;
                let mapped = start..end;
                return Ok(AnnotationDestination::Task(if kind_block {
                    ResolvedAnnotationAnchor::Block { range: mapped }
                } else {
                    ResolvedAnnotationAnchor::TextRange { range: mapped }
                }));
            } else {
                return Err(TaskPromotionAnnotationBlockerReason::CrossesPromotionBoundary);
            };
            Ok(AnnotationDestination::Source(if kind_block {
                ResolvedAnnotationAnchor::Block { range: destination }
            } else {
                ResolvedAnnotationAnchor::TextRange { range: destination }
            }))
        }
    }
}

fn replacement_len(
    proposed_source: &str,
    original_len: usize,
    branch: &Range<u64>,
) -> Result<u64, TaskPromotionAnnotationBlockerReason> {
    let removed = branch
        .end
        .checked_sub(branch.start)
        .ok_or(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)?;
    let original_len = u64::try_from(original_len)
        .map_err(|_| TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)?;
    let proposed_len = u64::try_from(proposed_source.len())
        .map_err(|_| TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)?;
    proposed_len
        .checked_add(removed)
        .and_then(|value| value.checked_sub(original_len))
        .ok_or(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)
}

fn transform_after_replacement(
    offset: u64,
    branch: &Range<u64>,
    replacement_len: u64,
) -> Result<u64, TaskPromotionAnnotationBlockerReason> {
    offset
        .checked_sub(branch.end)
        .and_then(|tail| branch.start.checked_add(replacement_len)?.checked_add(tail))
        .ok_or(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)
}

fn transform_range_after_replacement(
    range: &Range<u64>,
    branch: &Range<u64>,
    replacement_len: u64,
) -> Result<Range<u64>, TaskPromotionAnnotationBlockerReason> {
    Ok(
        transform_after_replacement(range.start, branch, replacement_len)?
            ..transform_after_replacement(range.end, branch, replacement_len)?,
    )
}

fn lift_offset(
    source_offset: u64,
    promotion: &weftext_asciidoc::ChecklistPromotionBranchEvidence,
    insertion: bool,
) -> Result<u64, TaskPromotionAnnotationBlockerReason> {
    let branch = &promotion.source_replacement_range;
    if source_offset < branch.start || source_offset > branch.end {
        return Err(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable);
    }
    let mut delta = 0_i128;
    for edit in &promotion.lift_edits {
        if insertion && (source_offset == edit.range.start || source_offset == edit.range.end) {
            return Err(TaskPromotionAnnotationBlockerReason::AmbiguousBoundaryOwnership);
        }
        if edit.range.start < source_offset && source_offset < edit.range.end {
            return Err(TaskPromotionAnnotationBlockerReason::IntersectsPrincipalOrLiftEdit);
        }
        if edit.range.end <= source_offset {
            let removed = i128::from(edit.range.end.saturating_sub(edit.range.start));
            let added = i128::try_from(edit.replacement.len()).map_err(|_| {
                TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable
            })?;
            delta += added - removed;
        }
    }
    let relative = i128::from(source_offset.saturating_sub(branch.start)) + delta;
    u64::try_from(relative)
        .map_err(|_| TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)
}

fn add_offsets(
    body_start: usize,
    body_offset: u64,
    source_len: usize,
) -> Result<u64, TaskPromotionAnnotationBlockerReason> {
    let offset = u64::try_from(body_start)
        .ok()
        .and_then(|start| start.checked_add(body_offset))
        .ok_or(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable)?;
    if offset > u64::try_from(source_len).unwrap_or(u64::MAX) {
        return Err(TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable);
    }
    Ok(offset)
}

fn blocker_from_rebuild_error(
    error: AnnotationAnchorMigrationError,
    resolved: &ResolvedAnnotationAnchor,
) -> TaskPromotionAnnotationBlockerReason {
    if matches!(resolved, ResolvedAnnotationAnchor::Block { .. }) {
        TaskPromotionAnnotationBlockerReason::InexactBlockGeometry
    } else if matches!(
        error,
        AnnotationAnchorMigrationError::Ambiguous | AnnotationAnchorMigrationError::NotFound
    ) {
        TaskPromotionAnnotationBlockerReason::DestinationGeometryUnavailable
    } else {
        TaskPromotionAnnotationBlockerReason::StaleOrInvalidAnchor
    }
}

fn workspace_revision(
    root: &Path,
    scope: Option<&WorkspaceReadScope>,
) -> Result<WorkspaceRevision, TaskPromotionError> {
    read_workspace_revision(root).map_err(|error| {
        if scope.is_some() {
            TaskPromotionError::WorkspaceAuthorityUnavailable
        } else {
            TaskPromotionError::WorkspaceTransaction(WorkspaceTransactionError::Revision(error))
        }
    })
}

fn portable_relative_path(root: &Path, destination: &Path) -> Result<String, TaskPromotionError> {
    destination
        .strip_prefix(root)
        .ok()
        .and_then(Path::to_str)
        .map(|value| value.replace('\\', "/"))
        .filter(|value| !value.is_empty())
        .ok_or(TaskPromotionError::DestinationConflict)
}

fn usize_range(range: &Range<u64>, source: &str) -> Result<Range<usize>, TaskPromotionError> {
    let start = usize::try_from(range.start).map_err(|_| TaskPromotionError::SourceEditPlan)?;
    let end = usize::try_from(range.end).map_err(|_| TaskPromotionError::SourceEditPlan)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(TaskPromotionError::SourceEditPlan);
    }
    Ok(start..end)
}

fn require_owner(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
) -> Result<(), TaskPromotionError> {
    validate_request(request)?;
    if plan.request == *request && matches!(plan.authorization, AuthorizationBinding::Owner) {
        Ok(())
    } else {
        Err(TaskPromotionError::AuthorizationChanged)
    }
}

fn require_scoped(
    plan: &TaskPromotionPlan,
    request: &TaskPromotionRequest,
    scope: &WorkspaceReadScope,
) -> Result<(), TaskPromotionError> {
    validate_request(request)?;
    if plan.request == *request
        && matches!(&plan.authorization, AuthorizationBinding::Scoped(reviewed) if reviewed == scope)
    {
        Ok(())
    } else {
        Err(TaskPromotionError::AuthorizationChanged)
    }
}

fn reviewed_plan_matches(reviewed: &TaskPromotionPlan, fresh: &TaskPromotionPlan) -> bool {
    reviewed.request == fresh.request
        && reviewed.summary == fresh.summary
        && reviewed.generated_node_id == fresh.generated_node_id
        && reviewed.source_edit == fresh.source_edit
        && reviewed.proposed_source == fresh.proposed_source
        && reviewed.task_document_source == fresh.task_document_source
        && reviewed.source_document_plan == fresh.source_document_plan
        && reviewed.source_sidecar == fresh.source_sidecar
        && reviewed.annotation_sidecar_snapshot == fresh.annotation_sidecar_snapshot
        && reviewed.workspace_root == fresh.workspace_root
        && reviewed.source_node_directory == fresh.source_node_directory
        && reviewed.destination_node_directory == fresh.destination_node_directory
        && reviewed.authorization == fresh.authorization
        && task_promotion_workspace_plans_match(&reviewed.transaction, &fresh.transaction)
}

fn promotion_expected_sidecar_state(
    state: &TaskPromotionSidecarState,
) -> AnnotationSidecarExpectedState {
    match state {
        TaskPromotionSidecarState::Present { sha256 } => AnnotationSidecarExpectedState::Present {
            sha256: sha256.clone(),
        },
        TaskPromotionSidecarState::ConfirmedAbsent => {
            AnnotationSidecarExpectedState::ConfirmedAbsent
        }
    }
}

fn map_planning_workspace_error(
    error: WorkspaceTransactionError,
    scope: Option<&WorkspaceReadScope>,
) -> TaskPromotionError {
    match error {
        WorkspaceTransactionError::StaleRevision { .. } => {
            TaskPromotionError::StaleWorkspaceRevision
        }
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskPromotionError::RecoveryRequired
        }
        _ if scope.is_some() => TaskPromotionError::WorkspaceAuthorityUnavailable,
        error => TaskPromotionError::WorkspaceTransaction(error),
    }
}

fn map_guard_error(error: &WorkspaceTransactionError) -> TaskPromotionError {
    match error {
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskPromotionError::RecoveryRequired
        }
        _ => TaskPromotionError::WorkspaceGuard,
    }
}

fn map_commit_error(
    error: WorkspaceTransactionError,
    scope: Option<&WorkspaceReadScope>,
) -> TaskPromotionError {
    match error {
        WorkspaceTransactionError::StaleRevision { .. } => {
            TaskPromotionError::StaleWorkspaceRevision
        }
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskPromotionError::RecoveryRequired
        }
        WorkspaceTransactionError::DraftGateAuthorityMismatch
        | WorkspaceTransactionError::DraftGateBlocked(_) => {
            TaskPromotionError::WorkspaceTransaction(error)
        }
        _ if scope.is_some() => TaskPromotionError::WorkspaceAuthorityUnavailable,
        error => TaskPromotionError::WorkspaceTransaction(error),
    }
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
