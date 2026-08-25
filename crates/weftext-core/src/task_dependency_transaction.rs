use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use weftext_asciidoc::{
    DocumentHeaderPatchError, SourceEdit, SourceEditPlan, plan_document_header_attribute_patch,
};

use crate::document::plan_document_edit_from_snapshot;
use crate::task_dependency_graph::{
    TaskGraphDiagnostic, TaskGraphDiagnosticCode, TaskGraphNode, TaskGraphPolicy,
    TaskGraphTargetClassification, resolve_task_dependency_graph,
};
use crate::task_node::analyze_task_node_profile_analysis;
use crate::workspace_transaction::{
    acquire_clean_workspace_mutation_guard, commit_workspace_transaction_with_clean_guard,
    plan_task_document_transaction_from_document_plan,
    validate_workspace_transaction_draft_gate_for_commit,
};
use crate::{
    CommittedWorkspaceTransaction, DocumentEdit, DocumentEditPlan, DocumentError, DocumentRevision,
    DocumentSnapshot, NodeId, TaskNodeActionEvidence, TaskNodeAttributeKind, TaskNodeDiagnostic,
    TaskNodeProfile, WorkspaceDraftGatePreview, WorkspaceDraftGateToken,
    WorkspaceDraftRegistryView, WorkspaceInventory, WorkspaceReadScope, WorkspaceRevision,
    WorkspaceTransactionError, WorkspaceTransactionPlan, read_node_document,
    read_workspace_revision, scan_workspace,
};

const DEPENDS_ON_ATTRIBUTE: &str = "weftext-task-depends-on";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskNodeDependencyReplacementRequest {
    pub evidence: TaskNodeActionEvidence,
    pub depends_on: Vec<NodeId>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskNodeDependencyReplacementDiagnosticCode {
    UnresolvedDependency,
    NonTaskDependency,
    InvalidDependencyTarget,
    DependencyCycle,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeDependencyReplacementDiagnostic {
    pub code: TaskNodeDependencyReplacementDiagnosticCode,
    pub source_node_id: NodeId,
    pub target_node_id: Option<NodeId>,
    pub range: Range<u64>,
    pub related_node_ids: Vec<NodeId>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeDependencyReplacementSummary {
    pub node_id: NodeId,
    pub workspace_revision: WorkspaceRevision,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub base_profile_revision: DocumentRevision,
    pub next_profile_revision: DocumentRevision,
    pub before_depends_on: Vec<NodeId>,
    pub after_depends_on: Vec<NodeId>,
    pub source_edit_count: u64,
}

#[derive(Clone)]
pub struct TaskNodeDependencyReplacementPlan {
    request: TaskNodeDependencyReplacementRequest,
    summary: TaskNodeDependencyReplacementSummary,
    source_edits: Vec<SourceEdit>,
    proposed_source: String,
    document_plan: DocumentEditPlan,
    transaction: Option<WorkspaceTransactionPlan>,
    workspace_root: PathBuf,
    node_directory: PathBuf,
    authorization: AuthorizationBinding,
}

impl fmt::Debug for TaskNodeDependencyReplacementPlan {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TaskNodeDependencyReplacementPlan")
            .field("summary", &self.summary)
            .field("source_edits", &self.source_edits)
            .field("authorization", &self.authorization.kind())
            .finish_non_exhaustive()
    }
}

impl TaskNodeDependencyReplacementPlan {
    #[must_use]
    pub const fn summary(&self) -> &TaskNodeDependencyReplacementSummary {
        &self.summary
    }

    /// Returns the reviewed source-level edits without exposing filesystem plan internals.
    #[must_use]
    pub fn source_edits(&self) -> &[SourceEdit] {
        &self.source_edits
    }

    /// Previews the standard draft gate for a changing plan. A verified no-op returns `None`.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed draft authority or an internally inconsistent plan.
    pub fn preview_draft_gate(
        &self,
        registry: &WorkspaceDraftRegistryView,
    ) -> Result<Option<WorkspaceDraftGatePreview>, TaskNodeDependencyReplacementError> {
        self.transaction
            .as_ref()
            .map(|transaction| {
                crate::preview_workspace_transaction_draft_gate(transaction, registry)
                    .map_err(TaskNodeDependencyReplacementError::WorkspaceTransaction)
            })
            .transpose()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum AuthorizationBinding {
    Owner,
    Scoped(WorkspaceReadScope),
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
pub struct CommittedTaskNodeDependencyReplacement {
    pub summary: TaskNodeDependencyReplacementSummary,
    pub transaction: Option<CommittedWorkspaceTransaction>,
}

#[derive(Debug)]
pub enum TaskNodeDependencyReplacementError {
    InvalidWorkspace,
    InvalidScope,
    TargetUnavailable,
    DependencyUnavailable,
    AuthorizationChanged,
    RootTaskIneligible,
    DependencyLimitExceeded,
    DuplicateDependency,
    SelfDependency,
    StaleDocumentRevision,
    StaleProfileRevision,
    StaleWorkspaceRevision,
    DocumentRead(DocumentError),
    DocumentIdentity,
    InvalidCurrentProfile {
        diagnostics: Vec<TaskNodeDiagnostic>,
    },
    HeaderPatch(DocumentHeaderPatchError),
    SourceEditPlan,
    PostValidation {
        diagnostics: Vec<TaskNodeDiagnostic>,
    },
    InvalidProposedGraph {
        diagnostics: Vec<TaskNodeDependencyReplacementDiagnostic>,
    },
    DocumentPlan(DocumentError),
    WorkspaceAuthorityUnavailable,
    WorkspaceGuard,
    WorkspaceTransaction(WorkspaceTransactionError),
    RecoveryRequired,
    ReviewedPlanMismatch,
    PostCommitValidation,
}

impl fmt::Display for TaskNodeDependencyReplacementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => formatter.write_str("workspace inventory is invalid"),
            Self::InvalidScope => formatter.write_str("workspace read scope is invalid"),
            Self::TargetUnavailable => {
                formatter.write_str("task dependency replacement target is unavailable")
            }
            Self::DependencyUnavailable => formatter
                .write_str("one or more task dependencies are unavailable to this mutation"),
            Self::AuthorizationChanged => formatter
                .write_str("task dependency replacement authorization changed after review"),
            Self::RootTaskIneligible => {
                formatter.write_str("workspace root is ineligible for a task-node profile")
            }
            Self::DependencyLimitExceeded => formatter
                .write_str("task dependency replacement exceeds the document-header value limit"),
            Self::DuplicateDependency => {
                formatter.write_str("task dependency replacement contains a duplicate node ID")
            }
            Self::SelfDependency => {
                formatter.write_str("task dependency replacement cannot depend on its source node")
            }
            Self::StaleDocumentRevision => {
                formatter.write_str("task dependency evidence has a stale document revision")
            }
            Self::StaleProfileRevision => {
                formatter.write_str("task dependency evidence has a stale profile revision")
            }
            Self::StaleWorkspaceRevision => {
                formatter.write_str("workspace changed while task dependencies were reviewed")
            }
            Self::DocumentRead(error) => write!(formatter, "task document read failed: {error}"),
            Self::DocumentIdentity => {
                formatter.write_str("task document identity differs from inventory authority")
            }
            Self::InvalidCurrentProfile { .. } => {
                formatter.write_str("source does not contain one locally valid task-node profile")
            }
            Self::HeaderPatch(error) => {
                write!(formatter, "task dependency header patch failed: {error}")
            }
            Self::SourceEditPlan => {
                formatter.write_str("task dependency replacement is not one exact source edit plan")
            }
            Self::PostValidation { .. } => formatter
                .write_str("task dependency replacement failed complete profile validation"),
            Self::InvalidProposedGraph { .. } => {
                formatter.write_str("authorized proposed task dependency graph is invalid")
            }
            Self::DocumentPlan(error) => {
                write!(formatter, "task dependency document plan failed: {error}")
            }
            Self::WorkspaceAuthorityUnavailable => {
                formatter.write_str("authorized workspace state is unavailable for this mutation")
            }
            Self::WorkspaceGuard => formatter.write_str("workspace mutation guard is unavailable"),
            Self::WorkspaceTransaction(error) => error.fmt(formatter),
            Self::RecoveryRequired => {
                formatter.write_str("workspace transaction recovery is required")
            }
            Self::ReviewedPlanMismatch => {
                formatter.write_str("fresh task dependency plan differs from the reviewed plan")
            }
            Self::PostCommitValidation => {
                formatter.write_str("committed task dependency replacement failed post-validation")
            }
        }
    }
}

impl std::error::Error for TaskNodeDependencyReplacementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::DocumentRead(error) | Self::DocumentPlan(error) => Some(error),
            Self::HeaderPatch(error) => Some(error),
            Self::WorkspaceTransaction(error) => Some(error),
            _ => None,
        }
    }
}

/// Plans an owner-authorized complete replacement of one task node's dependency set.
///
/// Planning scans inventory first, rejects duplicate/self requests before opening any managed
/// dependency document, then reads and parses every active authorized document at most once.
/// The private complete workspace revision is a raw-byte digest and may hash hidden bytes in the
/// scoped form; hidden documents are never UTF-8 decoded, parsed, classified, or diagnosed.
///
/// # Errors
///
/// Returns an error for invalid authority/evidence, unsafe header syntax, or any dependency error
/// remaining in the complete proposed mutation graph.
pub fn plan_task_node_dependency_replacement_transaction(
    root: impl AsRef<Path>,
    request: &TaskNodeDependencyReplacementRequest,
) -> Result<TaskNodeDependencyReplacementPlan, TaskNodeDependencyReplacementError> {
    plan_internal(root.as_ref(), request, None, None)
}

/// Plans the same replacement using only one already-authorized managed-node scope.
///
/// Hidden and missing dependency targets are both reported as generic unavailable graph evidence.
/// The private v1 workspace revision still hashes portable bytes, but hidden documents never enter
/// the parser/classification path.
///
/// # Errors
///
/// Returns the owner errors plus invalid-scope or non-disclosing unavailable authority errors.
pub fn plan_task_node_dependency_replacement_transaction_scoped(
    root: impl AsRef<Path>,
    request: &TaskNodeDependencyReplacementRequest,
    scope: &WorkspaceReadScope,
) -> Result<TaskNodeDependencyReplacementPlan, TaskNodeDependencyReplacementError> {
    plan_internal(root.as_ref(), request, Some(scope), None)
}

/// Commits an owner plan under Core's empty draft-registry compatibility authority.
///
/// # Errors
///
/// Returns an error for request/authority changes, a stale reviewed plan, recovery requirements,
/// or recoverable transaction failure.
pub fn commit_task_node_dependency_replacement_transaction(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
) -> Result<CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementError> {
    require_owner(plan, request)?;
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let token = plan
        .transaction
        .as_ref()
        .map(|transaction| {
            crate::preview_workspace_transaction_draft_gate(transaction, &registry)
                .map_err(TaskNodeDependencyReplacementError::WorkspaceTransaction)
                .and_then(|preview| {
                    preview.executable_token.ok_or_else(|| {
                        TaskNodeDependencyReplacementError::WorkspaceTransaction(
                            WorkspaceTransactionError::DraftGateBlocked(Vec::new()),
                        )
                    })
                })
        })
        .transpose()?;
    commit_internal(plan, request, None, token.as_ref(), Some(&registry))
}

/// Commits an owner plan using a fresh explicit draft-registry authority.
///
/// A verified no-op has no workspace transaction and therefore requires `token == None`.
///
/// # Errors
///
/// Returns the ordinary commit errors plus draft blocker/authority failures.
pub fn commit_task_node_dependency_replacement_transaction_with_draft_gate(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
    token: Option<&WorkspaceDraftGateToken>,
    registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementError> {
    require_owner(plan, request)?;
    commit_internal(plan, request, None, token, Some(registry))
}

/// Commits a scoped plan only when the complete fresh scope and request equal reviewed authority.
///
/// # Errors
///
/// Returns [`TaskNodeDependencyReplacementError::AuthorizationChanged`] before document I/O for
/// any owner/scoped, request, or scope mismatch.
pub fn commit_task_node_dependency_replacement_transaction_scoped(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
    fresh_scope: &WorkspaceReadScope,
) -> Result<CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementError> {
    require_scoped(plan, request, fresh_scope)?;
    let registry = WorkspaceDraftRegistryView::empty_authority();
    let token = plan
        .transaction
        .as_ref()
        .map(|transaction| {
            crate::preview_workspace_transaction_draft_gate(transaction, &registry)
                .map_err(TaskNodeDependencyReplacementError::WorkspaceTransaction)
                .and_then(|preview| {
                    preview.executable_token.ok_or_else(|| {
                        TaskNodeDependencyReplacementError::WorkspaceTransaction(
                            WorkspaceTransactionError::DraftGateBlocked(Vec::new()),
                        )
                    })
                })
        })
        .transpose()?;
    commit_internal(
        plan,
        request,
        Some(fresh_scope),
        token.as_ref(),
        Some(&registry),
    )
}

/// Scoped explicit-draft counterpart of
/// [`commit_task_node_dependency_replacement_transaction_with_draft_gate`].
///
/// # Errors
///
/// Returns authorization, draft gate, stale plan, recovery, or transaction failures.
pub fn commit_task_node_dependency_replacement_transaction_scoped_with_draft_gate(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
    fresh_scope: &WorkspaceReadScope,
    token: Option<&WorkspaceDraftGateToken>,
    registry: &WorkspaceDraftRegistryView,
) -> Result<CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementError> {
    require_scoped(plan, request, fresh_scope)?;
    commit_internal(plan, request, Some(fresh_scope), token, Some(registry))
}

fn commit_internal(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
    scope: Option<&WorkspaceReadScope>,
    token: Option<&WorkspaceDraftGateToken>,
    registry: Option<&WorkspaceDraftRegistryView>,
) -> Result<CommittedTaskNodeDependencyReplacement, TaskNodeDependencyReplacementError> {
    match (&plan.transaction, token, registry) {
        (Some(transaction), Some(token), Some(registry)) => {
            validate_workspace_transaction_draft_gate_for_commit(transaction, token, registry)
                .map_err(TaskNodeDependencyReplacementError::WorkspaceTransaction)?;
        }
        (None, None, _) => {}
        _ => return Err(TaskNodeDependencyReplacementError::AuthorizationChanged),
    }

    let guard = acquire_clean_workspace_mutation_guard(&plan.workspace_root)
        .map_err(|error| map_guard_error(&error))?;
    let fresh = plan_internal(
        &plan.workspace_root,
        request,
        scope,
        Some(&plan.summary.workspace_revision),
    )?;
    if !reviewed_plan_matches(plan, &fresh) {
        return Err(TaskNodeDependencyReplacementError::ReviewedPlanMismatch);
    }

    let transaction = match &plan.transaction {
        Some(transaction) => Some(
            commit_workspace_transaction_with_clean_guard(transaction, &guard)
                .map_err(|error| map_commit_error(error, scope))?,
        ),
        None => None,
    };
    verify_committed_source(plan)?;
    drop(guard);
    Ok(CommittedTaskNodeDependencyReplacement {
        summary: plan.summary.clone(),
        transaction,
    })
}

#[allow(clippy::too_many_lines)]
fn plan_internal(
    root: &Path,
    request: &TaskNodeDependencyReplacementRequest,
    scope: Option<&WorkspaceReadScope>,
    expected_workspace_revision: Option<&WorkspaceRevision>,
) -> Result<TaskNodeDependencyReplacementPlan, TaskNodeDependencyReplacementError> {
    let inventory = authorized_inventory(root, request.evidence.node_id, scope)?;
    let source_node = locate_authorized_node(&inventory, request.evidence.node_id)?;
    if source_node.path == inventory.root {
        return Err(TaskNodeDependencyReplacementError::RootTaskIneligible);
    }
    let source_node_directory = source_node.path.clone();
    validate_requested_dependencies(request)?;
    if scope.is_some_and(|scope| {
        request
            .depends_on
            .iter()
            .any(|node_id| !scope.allows(*node_id))
    }) {
        return Err(TaskNodeDependencyReplacementError::DependencyUnavailable);
    }

    let source_snapshot = read_snapshot(&source_node_directory, request.evidence.node_id)?;
    require_action_evidence(&source_snapshot, &request.evidence)?;
    let source_parser = weftext_asciidoc::analyze(&source_snapshot.source);
    let before_analysis =
        analyze_task_node_profile_analysis(&source_snapshot.source, None, &source_parser);
    let before = valid_current_profile(before_analysis)?;
    let mut canonical_depends_on = request.depends_on.clone();
    canonical_depends_on.sort_unstable();
    let canonical_value = (!canonical_depends_on.is_empty()).then(|| {
        canonical_depends_on
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    });
    let source_edit = plan_document_header_attribute_patch(
        &source_snapshot.source,
        DEPENDS_ON_ATTRIBUTE,
        canonical_value.as_deref(),
    )
    .map_err(TaskNodeDependencyReplacementError::HeaderPatch)?;
    let source_edits = source_edit.into_iter().collect::<Vec<_>>();
    let proposed_source = SourceEditPlan::new(&source_snapshot.source, source_edits.clone())
        .map_err(|_| TaskNodeDependencyReplacementError::SourceEditPlan)?
        .apply(&source_snapshot.source)
        .ok_or(TaskNodeDependencyReplacementError::SourceEditPlan)?;
    let proposed_parser = weftext_asciidoc::analyze(&proposed_source);
    let proposed_analysis = analyze_task_node_profile_analysis(
        &proposed_source,
        Some(request.evidence.node_id),
        &proposed_parser,
    );
    let after = valid_proposed_profile(proposed_analysis.clone())?;
    let mut expected_after = before.clone();
    expected_after.depends_on.clone_from(&canonical_depends_on);
    if after != expected_after {
        return Err(TaskNodeDependencyReplacementError::PostValidation {
            diagnostics: proposed_analysis.diagnostics,
        });
    }

    let mut nodes = BTreeMap::new();
    let mut classifications = BTreeMap::new();
    insert_graph_profile(
        request.evidence.node_id,
        &proposed_analysis,
        &mut nodes,
        &mut classifications,
    );

    let mut records = inventory
        .nodes
        .iter()
        .filter_map(|node| node.id.map(|node_id| (node_id, node)))
        .filter(|(node_id, node)| {
            *node_id != request.evidence.node_id
                && scope.is_none_or(|scope| scope.allows(*node_id))
                && !crate::workspace_trash::is_trash_storage_path(&inventory.root, &node.path)
        })
        .collect::<Vec<_>>();
    records.sort_by_key(|(node_id, _)| *node_id);
    for (node_id, node) in records {
        if node.path == inventory.root {
            classifications.insert(node_id, TaskGraphTargetClassification::Invalid);
            continue;
        }
        let snapshot = read_snapshot(&node.path, node_id)?;
        let parser = weftext_asciidoc::analyze(&snapshot.source);
        let analysis = analyze_task_node_profile_analysis(&snapshot.source, None, &parser);
        insert_graph_profile(node_id, &analysis, &mut nodes, &mut classifications);
    }

    let graph =
        resolve_task_dependency_graph(&nodes, &classifications, TaskGraphPolicy::MutationStrict);
    let relevant_node_ids = dependency_closure(request.evidence.node_id, &nodes);
    let requested_targets_valid = canonical_depends_on
        .iter()
        .all(|node_id| graph.valid_node_ids.contains(node_id));
    if !graph.valid_node_ids.contains(&request.evidence.node_id) || !requested_targets_valid {
        return Err(TaskNodeDependencyReplacementError::InvalidProposedGraph {
            diagnostics: graph
                .diagnostics
                .iter()
                .filter(|diagnostic| relevant_node_ids.contains(&diagnostic.source_node_id))
                .map(graph_diagnostic)
                .collect(),
        });
    }

    let first_workspace_revision = workspace_revision(root, scope)?;
    if expected_workspace_revision.is_some_and(|expected| expected != &first_workspace_revision) {
        return Err(TaskNodeDependencyReplacementError::StaleWorkspaceRevision);
    }
    let final_workspace_revision = workspace_revision(root, scope)?;
    if first_workspace_revision != final_workspace_revision {
        return Err(TaskNodeDependencyReplacementError::StaleWorkspaceRevision);
    }
    let document_plan = plan_document_edit_from_snapshot(
        &source_snapshot,
        source_edits
            .iter()
            .map(document_edit)
            .collect::<Result<Vec<_>, _>>()?,
    )
    .map_err(TaskNodeDependencyReplacementError::DocumentPlan)?;
    if document_plan.next_source() != proposed_source {
        return Err(TaskNodeDependencyReplacementError::ReviewedPlanMismatch);
    }
    let transaction = document_plan
        .changed
        .then(|| {
            plan_task_document_transaction_from_document_plan(
                root,
                &final_workspace_revision,
                request.evidence.node_id,
                &document_plan,
                &source_snapshot.source,
                &before.depends_on,
                &canonical_depends_on,
            )
            .map_err(|error| map_planning_workspace_error(error, scope))
        })
        .transpose()?;
    let next_revision = DocumentRevision::from_source(&proposed_source);
    let source_edit_count = u64::try_from(source_edits.len()).unwrap_or(u64::MAX);
    Ok(TaskNodeDependencyReplacementPlan {
        request: request.clone(),
        summary: TaskNodeDependencyReplacementSummary {
            node_id: request.evidence.node_id,
            workspace_revision: final_workspace_revision,
            base_revision: source_snapshot.revision.clone(),
            next_revision: next_revision.clone(),
            base_profile_revision: before_analysis_revision(&source_snapshot),
            next_profile_revision: next_revision,
            before_depends_on: before.depends_on,
            after_depends_on: canonical_depends_on,
            source_edit_count,
        },
        source_edits,
        proposed_source,
        document_plan,
        transaction,
        workspace_root: inventory.root,
        node_directory: source_node_directory,
        authorization: scope.map_or(AuthorizationBinding::Owner, |scope| {
            AuthorizationBinding::Scoped(scope.clone())
        }),
    })
}

fn authorized_inventory(
    root: &Path,
    source_node_id: NodeId,
    scope: Option<&WorkspaceReadScope>,
) -> Result<WorkspaceInventory, TaskNodeDependencyReplacementError> {
    let inventory = scan_workspace(root);
    if let Some(scope) = scope {
        scope
            .validate_inventory(&inventory)
            .map_err(|_| TaskNodeDependencyReplacementError::InvalidScope)?;
        if !scope.allows(source_node_id) {
            return Err(TaskNodeDependencyReplacementError::TargetUnavailable);
        }
    } else if !inventory.is_valid() {
        return Err(TaskNodeDependencyReplacementError::InvalidWorkspace);
    }
    Ok(inventory)
}

fn locate_authorized_node(
    inventory: &WorkspaceInventory,
    node_id: NodeId,
) -> Result<&crate::NodeRecord, TaskNodeDependencyReplacementError> {
    let matches = inventory
        .nodes
        .iter()
        .filter(|node| {
            node.id == Some(node_id)
                && node.metadata.is_some()
                && !crate::workspace_trash::is_trash_storage_path(&inventory.root, &node.path)
        })
        .collect::<Vec<_>>();
    let [node] = matches.as_slice() else {
        return Err(TaskNodeDependencyReplacementError::TargetUnavailable);
    };
    Ok(node)
}

fn validate_requested_dependencies(
    request: &TaskNodeDependencyReplacementRequest,
) -> Result<(), TaskNodeDependencyReplacementError> {
    let mut encoded_length = 0_usize;
    for (index, dependency) in request.depends_on.iter().enumerate() {
        if index > 0 {
            encoded_length = encoded_length
                .checked_add(1)
                .ok_or(TaskNodeDependencyReplacementError::DependencyLimitExceeded)?;
        }
        encoded_length = encoded_length
            .checked_add(dependency.to_string().len())
            .ok_or(TaskNodeDependencyReplacementError::DependencyLimitExceeded)?;
        if encoded_length > weftext_asciidoc::MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES {
            return Err(TaskNodeDependencyReplacementError::DependencyLimitExceeded);
        }
    }
    let mut unique = BTreeSet::new();
    for dependency in &request.depends_on {
        if *dependency == request.evidence.node_id {
            return Err(TaskNodeDependencyReplacementError::SelfDependency);
        }
        if !unique.insert(*dependency) {
            return Err(TaskNodeDependencyReplacementError::DuplicateDependency);
        }
    }
    Ok(())
}

fn workspace_revision(
    root: &Path,
    scope: Option<&WorkspaceReadScope>,
) -> Result<WorkspaceRevision, TaskNodeDependencyReplacementError> {
    read_workspace_revision(root).map_err(|error| {
        if scope.is_some() {
            TaskNodeDependencyReplacementError::WorkspaceAuthorityUnavailable
        } else {
            TaskNodeDependencyReplacementError::WorkspaceTransaction(
                WorkspaceTransactionError::Revision(error),
            )
        }
    })
}

fn read_snapshot(
    node_directory: &Path,
    expected_node_id: NodeId,
) -> Result<DocumentSnapshot, TaskNodeDependencyReplacementError> {
    let snapshot = read_node_document(node_directory)
        .map_err(TaskNodeDependencyReplacementError::DocumentRead)?;
    if snapshot.node_id != expected_node_id || snapshot.node_directory != node_directory {
        return Err(TaskNodeDependencyReplacementError::DocumentIdentity);
    }
    Ok(snapshot)
}

fn require_action_evidence(
    snapshot: &DocumentSnapshot,
    evidence: &TaskNodeActionEvidence,
) -> Result<(), TaskNodeDependencyReplacementError> {
    if snapshot.revision != evidence.revision {
        return Err(TaskNodeDependencyReplacementError::StaleDocumentRevision);
    }
    if snapshot.revision != evidence.profile_revision {
        return Err(TaskNodeDependencyReplacementError::StaleProfileRevision);
    }
    Ok(())
}

fn valid_current_profile(
    analysis: crate::TaskNodeProfileAnalysis,
) -> Result<TaskNodeProfile, TaskNodeDependencyReplacementError> {
    if analysis.title.is_none() || !analysis.diagnostics.is_empty() {
        return Err(TaskNodeDependencyReplacementError::InvalidCurrentProfile {
            diagnostics: analysis.diagnostics,
        });
    }
    analysis
        .profile
        .ok_or(TaskNodeDependencyReplacementError::InvalidCurrentProfile {
            diagnostics: Vec::new(),
        })
}

fn valid_proposed_profile(
    analysis: crate::TaskNodeProfileAnalysis,
) -> Result<TaskNodeProfile, TaskNodeDependencyReplacementError> {
    if analysis.title.is_none() || !analysis.diagnostics.is_empty() {
        return Err(TaskNodeDependencyReplacementError::PostValidation {
            diagnostics: analysis.diagnostics,
        });
    }
    analysis
        .profile
        .ok_or(TaskNodeDependencyReplacementError::PostValidation {
            diagnostics: Vec::new(),
        })
}

fn insert_graph_profile(
    node_id: NodeId,
    analysis: &crate::TaskNodeProfileAnalysis,
    nodes: &mut BTreeMap<NodeId, TaskGraphNode>,
    classifications: &mut BTreeMap<NodeId, TaskGraphTargetClassification>,
) {
    let classification = if let (Some(profile), Some(_)) = (&analysis.profile, &analysis.title) {
        let dependency_range = analysis
            .attributes
            .iter()
            .find(|attribute| attribute.kind == TaskNodeAttributeKind::DependsOn)
            .map_or(0..0, |attribute| attribute.value_range.clone());
        nodes.insert(
            node_id,
            TaskGraphNode {
                node_id,
                state: profile.state,
                depends_on: profile.depends_on.clone(),
                dependency_range,
            },
        );
        TaskGraphTargetClassification::Valid
    } else if analysis.has_reserved_evidence {
        TaskGraphTargetClassification::Invalid
    } else {
        TaskGraphTargetClassification::NonTask
    };
    classifications.insert(node_id, classification);
}

fn graph_diagnostic(diagnostic: &TaskGraphDiagnostic) -> TaskNodeDependencyReplacementDiagnostic {
    TaskNodeDependencyReplacementDiagnostic {
        code: match diagnostic.code {
            TaskGraphDiagnosticCode::UnresolvedDependency => {
                TaskNodeDependencyReplacementDiagnosticCode::UnresolvedDependency
            }
            TaskGraphDiagnosticCode::NonTaskDependency => {
                TaskNodeDependencyReplacementDiagnosticCode::NonTaskDependency
            }
            TaskGraphDiagnosticCode::InvalidDependencyTarget => {
                TaskNodeDependencyReplacementDiagnosticCode::InvalidDependencyTarget
            }
            TaskGraphDiagnosticCode::DependencyCycle => {
                TaskNodeDependencyReplacementDiagnosticCode::DependencyCycle
            }
        },
        source_node_id: diagnostic.source_node_id,
        target_node_id: diagnostic.target_node_id,
        range: diagnostic.range.clone(),
        related_node_ids: diagnostic.related_node_ids.clone(),
        message: diagnostic.message.clone(),
    }
}

fn dependency_closure(
    source_node_id: NodeId,
    nodes: &BTreeMap<NodeId, TaskGraphNode>,
) -> BTreeSet<NodeId> {
    let mut closure = BTreeSet::new();
    let mut pending = vec![source_node_id];
    while let Some(node_id) = pending.pop() {
        if !closure.insert(node_id) {
            continue;
        }
        if let Some(node) = nodes.get(&node_id) {
            pending.extend(node.depends_on.iter().copied());
        }
    }
    closure
}

fn document_edit(edit: &SourceEdit) -> Result<DocumentEdit, TaskNodeDependencyReplacementError> {
    Ok(DocumentEdit {
        start: u64::try_from(edit.range.start)
            .map_err(|_| TaskNodeDependencyReplacementError::SourceEditPlan)?,
        end: u64::try_from(edit.range.end)
            .map_err(|_| TaskNodeDependencyReplacementError::SourceEditPlan)?,
        replacement: edit.replacement.clone(),
    })
}

fn before_analysis_revision(snapshot: &DocumentSnapshot) -> DocumentRevision {
    snapshot.revision.clone()
}

fn require_owner(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
) -> Result<(), TaskNodeDependencyReplacementError> {
    if plan.request == *request && matches!(plan.authorization, AuthorizationBinding::Owner) {
        Ok(())
    } else {
        Err(TaskNodeDependencyReplacementError::AuthorizationChanged)
    }
}

fn require_scoped(
    plan: &TaskNodeDependencyReplacementPlan,
    request: &TaskNodeDependencyReplacementRequest,
    scope: &WorkspaceReadScope,
) -> Result<(), TaskNodeDependencyReplacementError> {
    if plan.request == *request
        && matches!(&plan.authorization, AuthorizationBinding::Scoped(reviewed) if reviewed == scope)
    {
        Ok(())
    } else {
        Err(TaskNodeDependencyReplacementError::AuthorizationChanged)
    }
}

fn reviewed_plan_matches(
    reviewed: &TaskNodeDependencyReplacementPlan,
    fresh: &TaskNodeDependencyReplacementPlan,
) -> bool {
    reviewed.request == fresh.request
        && reviewed.summary == fresh.summary
        && reviewed.source_edits == fresh.source_edits
        && reviewed.proposed_source == fresh.proposed_source
        && reviewed.document_plan == fresh.document_plan
        && reviewed.workspace_root == fresh.workspace_root
        && reviewed.node_directory == fresh.node_directory
        && reviewed.authorization == fresh.authorization
        && reviewed.transaction.is_some() == fresh.transaction.is_some()
}

fn verify_committed_source(
    plan: &TaskNodeDependencyReplacementPlan,
) -> Result<(), TaskNodeDependencyReplacementError> {
    let snapshot = read_snapshot(&plan.node_directory, plan.summary.node_id)?;
    if snapshot.revision != plan.summary.next_revision || snapshot.source != plan.proposed_source {
        return Err(TaskNodeDependencyReplacementError::PostCommitValidation);
    }
    let analysis = crate::analyze_task_node_profile(&snapshot.source, Some(plan.summary.node_id));
    if analysis.profile_revision != plan.summary.next_profile_revision
        || analysis.profile.as_ref().map(|profile| &profile.depends_on)
            != Some(&plan.summary.after_depends_on)
        || !analysis.diagnostics.is_empty()
    {
        return Err(TaskNodeDependencyReplacementError::PostCommitValidation);
    }
    Ok(())
}

fn map_planning_workspace_error(
    error: WorkspaceTransactionError,
    scope: Option<&WorkspaceReadScope>,
) -> TaskNodeDependencyReplacementError {
    match error {
        WorkspaceTransactionError::StaleRevision { .. } => {
            TaskNodeDependencyReplacementError::StaleWorkspaceRevision
        }
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskNodeDependencyReplacementError::RecoveryRequired
        }
        _ if scope.is_some() => TaskNodeDependencyReplacementError::WorkspaceAuthorityUnavailable,
        error => TaskNodeDependencyReplacementError::WorkspaceTransaction(error),
    }
}

fn map_guard_error(error: &WorkspaceTransactionError) -> TaskNodeDependencyReplacementError {
    match error {
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskNodeDependencyReplacementError::RecoveryRequired
        }
        _ => TaskNodeDependencyReplacementError::WorkspaceGuard,
    }
}

fn map_commit_error(
    error: WorkspaceTransactionError,
    scope: Option<&WorkspaceReadScope>,
) -> TaskNodeDependencyReplacementError {
    match error {
        WorkspaceTransactionError::StaleRevision { .. } => {
            TaskNodeDependencyReplacementError::StaleWorkspaceRevision
        }
        WorkspaceTransactionError::RecoveryRequired(_)
        | WorkspaceTransactionError::RecoveryRequiredWithCause { .. } => {
            TaskNodeDependencyReplacementError::RecoveryRequired
        }
        WorkspaceTransactionError::DraftGateAuthorityMismatch
        | WorkspaceTransactionError::DraftGateBlocked(_) => {
            TaskNodeDependencyReplacementError::WorkspaceTransaction(error)
        }
        _ if scope.is_some() => TaskNodeDependencyReplacementError::WorkspaceAuthorityUnavailable,
        error => TaskNodeDependencyReplacementError::WorkspaceTransaction(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_commit_errors_discard_hidden_path_material() {
        let scope = WorkspaceReadScope::default();
        for error in [
            WorkspaceTransactionError::VerificationFailed(
                "hidden C:/private/node.adoc verification detail".to_owned(),
            ),
            WorkspaceTransactionError::Io(std::io::Error::other(
                "hidden C:/private/node.adoc I/O detail",
            )),
        ] {
            let mapped = map_commit_error(error, Some(&scope));
            assert!(matches!(
                mapped,
                TaskNodeDependencyReplacementError::WorkspaceAuthorityUnavailable
            ));
            let displayed = mapped.to_string();
            assert!(!displayed.contains("private"));
            assert!(!displayed.contains("node.adoc"));
        }
    }
}
