//! Frozen source-set task import over the shared Markdown Import IR and Core transaction.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use weftext_core::{
    CommittedWorkspaceTransaction, NodeId, RecoveryReport, StructuralAction,
    TaskImportDocumentInput, TaskImportDocumentPlan, TaskImportEdit, TaskImportIdentityMapping,
    TaskImportPlan, TaskImportSettings, WorkspaceImportAuthority, WorkspaceImportNode,
    WorkspaceImportTransactionState, WorkspaceRevision, WorkspaceTransactionPlan,
    analyze_query_source, analyze_task_source, commit_workspace_transaction_retaining_journal,
    finalize_committed_workspace_transaction, inspect_workspace_import_transaction,
    plan_import_tree, plan_task_import, publish_committed_workspace_transaction_receipt,
    read_committed_workspace_transaction_receipt_handoff, read_workspace_revision,
    recover_workspace_import_transaction, scan_workspace, validate_task_import_plan,
};
use weftext_import::{
    CancellationToken, CommitResult, ImportReceipt, ImportTempRoot, OriginClass, PortablePath,
    Sha256Digest, sha256_bytes,
};

use crate::{
    ImportPreviewBundle, IntakeError, IntakeErrorCode, MARKDOWN_SOURCE_BYTE_LIMIT,
    preview_markdown_import, validate_markdown_preview,
};

pub const TASK_IMPORT_BUNDLE_CONTRACT_VERSION: &str = "weftext.task-import-bundle.v1";
pub const TASK_IMPORT_RECEIPT_CONTRACT_VERSION: &str = "weftext.task-import-receipt.v1";
const TASK_PATCH_LANGUAGE: &str = "weftext-task-import-v1";
const MAX_TASK_SOURCE_DOCUMENTS: usize = 2_048;
const MAX_TASK_SOURCE_SET_BYTES: usize = 8 * 1024 * 1024;

/// Exact typed patch that bridges task-specific semantics into a reviewed common Import IR node.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportPatchEvidence {
    pub edit_index: u64,
    pub token: String,
    pub replacement_digest: Sha256Digest,
}

/// Common Markdown Import IR evidence for one exact member of the source set.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportDocumentEvidence {
    pub locator: String,
    pub destination_locator: String,
    pub derived_source_digest: Sha256Digest,
    pub patches: Vec<TaskImportPatchEvidence>,
    pub neutralized_unreviewed_tasks: u64,
    pub common_preview: ImportPreviewBundle,
}

/// One exact canonical node in the reviewed source-set proposal.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportProposedNode {
    pub source_locator: Option<String>,
    pub destination_locator: String,
    pub node_id: NodeId,
    pub document_file: String,
    pub exact_asciidoc: String,
    pub document_digest: Sha256Digest,
}

/// Immutable, complete preview authority for one explicit Markdown/Obsidian source set.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportPreviewBundle {
    pub contract_version: String,
    pub bundle_digest: Sha256Digest,
    pub workspace_root_id: NodeId,
    pub base_workspace_revision: WorkspaceRevision,
    pub destination_parent_id: NodeId,
    pub destination_parent_locator: String,
    pub destination_name: String,
    pub destination_root_locator: String,
    pub source_set_digest: Sha256Digest,
    pub source_documents: Vec<TaskImportDocumentInput>,
    pub task_plan: TaskImportPlan,
    pub evidence: Vec<TaskImportDocumentEvidence>,
    pub nodes: Vec<TaskImportProposedNode>,
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub preview_created_at: String,
}

/// Explicit values a caller must carry from the human-reviewed preview into commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportReview {
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub bundle_digest: Sha256Digest,
}

impl TaskImportReview {
    #[must_use]
    pub fn from_preview(bundle: &TaskImportPreviewBundle) -> Self {
        Self {
            proposal_id: bundle.proposal_id.clone(),
            proposal_digest: bundle.proposal_digest.clone(),
            bundle_digest: bundle.bundle_digest.clone(),
        }
    }
}

impl TaskImportPreviewBundle {
    fn compute_bundle_digest(&self) -> Result<Sha256Digest, IntakeError> {
        let material = serde_json::to_vec(&(
            &self.contract_version,
            self.workspace_root_id,
            &self.base_workspace_revision,
            self.destination_parent_id,
            &self.destination_parent_locator,
            &self.destination_name,
            &self.destination_root_locator,
            &self.source_set_digest,
            &self.source_documents,
            &self.task_plan,
            &self.evidence,
            &self.nodes,
            &self.proposal_id,
            &self.proposal_digest,
            &self.preview_created_at,
        ))
        .map_err(|error| super::serialization("serialize task import bundle authority", &error))?;
        Ok(sha256_bytes(&material))
    }
}

/// Durable evidence returned after the single Core transaction commits.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct TaskImportReceipt {
    pub contract_version: String,
    pub receipt_id: String,
    pub created_at: String,
    pub source_set_digest: Sha256Digest,
    pub reviewed_bundle_digest: Sha256Digest,
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub identities: Vec<TaskImportIdentityMapping>,
    pub nodes: Vec<TaskImportProposedNode>,
    pub common_receipts: Vec<ImportReceipt>,
    pub transaction: CommittedWorkspaceTransaction,
}

/// Exact result of consuming a reviewed task source-set bundle.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CommittedTaskImport {
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub transaction: CommittedWorkspaceTransaction,
    pub receipt: TaskImportReceipt,
}

/// Result of idempotently resolving a task-import journal/receipt handoff after interruption.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum TaskImportRecovery {
    ReceiptRecovered {
        committed: CommittedTaskImport,
        recovery: RecoveryReport,
    },
    AlreadyFinalized {
        committed: CommittedTaskImport,
        recovery: RecoveryReport,
    },
    RolledBack {
        recovery: RecoveryReport,
    },
}

/// Runs the complete explicit task-import preview without writing the workspace.
///
/// Each source is first shielded by deterministic typed patch placeholders and then sent through
/// the ordinary Markdown probe/plan/worker/IR/proposal route. The exact reviewed task/query edits
/// replace only those placeholders in the final canonical documents. A blocking task diagnostic
/// remains visible in the returned preview and makes the bundle non-committable.
///
/// # Errors
///
/// Returns a typed error for an unsafe source set, invalid settings or destination, Import IR
/// failure, ambiguous node mapping, or a target that Core cannot express safely.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn preview_task_import(
    workspace: impl AsRef<Path>,
    temp_root: &ImportTempRoot,
    destination_parent_id: NodeId,
    destination_name: impl Into<String>,
    source_documents: Vec<TaskImportDocumentInput>,
    settings: TaskImportSettings,
    created_at: impl Into<String>,
    cancellation: &CancellationToken,
) -> Result<TaskImportPreviewBundle, IntakeError> {
    let workspace = workspace.as_ref();
    let destination_name = destination_name.into();
    let created_at = created_at.into();
    validate_source_set_bounds(&source_documents)?;
    let base_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| super::workspace_error(&error))?;
    let (workspace_root_id, destination_parent_locator, destination_root_locator) =
        bind_destination(workspace, destination_parent_id, &destination_name)?;
    let task_plan = plan_task_import(&source_documents, settings)
        .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
    let source_targets = source_destination_locators(&source_documents, &destination_root_locator)?;

    let mut evidence = Vec::with_capacity(source_documents.len());
    let mut source_nodes = BTreeMap::new();
    for ((input, document_plan), destination_locator) in source_documents
        .iter()
        .zip(&task_plan.documents)
        .zip(&source_targets)
    {
        if input.locator != document_plan.locator {
            return super::invalid_bundle("task plan document order lost source-set identity");
        }
        let (derived_source, patches) = derive_placeholder_source(input, document_plan)?;
        let destination = PortablePath::parse(destination_locator.clone())
            .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
        let display_name = input.locator.rsplit('/').next().ok_or_else(|| {
            super::invalid_bundle_error("task source locator has no display filename")
        })?;
        let common_preview = preview_markdown_import(
            workspace,
            temp_root.clone(),
            display_name,
            OriginClass::LocalFile,
            derived_source.as_bytes().to_vec(),
            destination,
            false,
            created_at.clone(),
            cancellation.clone(),
        )?;
        let common_node = only_common_node(&common_preview)?;
        let (exact_asciidoc, neutralized_unreviewed_tasks) =
            apply_typed_patches(&common_node.exact_asciidoc, document_plan, &patches)?;
        let node_id = common_node.node_id.parse::<NodeId>().map_err(|error| {
            super::invalid_bundle_error(format!("invalid common task import node ID: {error}"))
        })?;
        let proposed = TaskImportProposedNode {
            source_locator: Some(input.locator.clone()),
            destination_locator: destination_locator.clone(),
            node_id,
            document_file: common_node.document_file.clone(),
            document_digest: sha256_bytes(exact_asciidoc.as_bytes()),
            exact_asciidoc,
        };
        if source_nodes
            .insert(destination_locator.to_lowercase(), proposed)
            .is_some()
        {
            return super::invalid_bundle(
                "task source documents collide at one portable destination node",
            );
        }
        evidence.push(TaskImportDocumentEvidence {
            locator: input.locator.clone(),
            destination_locator: destination_locator.clone(),
            derived_source_digest: sha256_bytes(derived_source.as_bytes()),
            patches,
            neutralized_unreviewed_tasks,
            common_preview,
        });
    }

    let mut nodes = complete_destination_tree(&destination_root_locator, &source_nodes)?;
    nodes.sort_by(|left, right| left.destination_locator.cmp(&right.destination_locator));
    let source_set_digest = source_set_digest(&source_documents)?;
    let proposal_digest = task_proposal_digest(&source_set_digest, &task_plan, &evidence, &nodes)?;
    let proposal_id = format!("task-proposal-{}", &proposal_digest.as_str()[..24]);
    let mut bundle = TaskImportPreviewBundle {
        contract_version: TASK_IMPORT_BUNDLE_CONTRACT_VERSION.to_owned(),
        bundle_digest: sha256_bytes(b"pending"),
        workspace_root_id,
        base_workspace_revision,
        destination_parent_id,
        destination_parent_locator,
        destination_name,
        destination_root_locator,
        source_set_digest,
        source_documents,
        task_plan,
        evidence,
        nodes,
        proposal_id,
        proposal_digest,
        preview_created_at: created_at,
    };
    bundle.bundle_digest = bundle.compute_bundle_digest()?;
    validate_task_import_preview(&bundle)?;

    if bundle.task_plan.is_committable() {
        let authority = task_authority(&bundle);
        plan_import_tree(
            workspace,
            &bundle.base_workspace_revision,
            authority,
            workspace_nodes(&bundle.nodes),
        )
        .map_err(|error| super::workspace_error(&error))?;
    }
    Ok(bundle)
}

/// Revalidates every exact source byte, frozen task identity, common Import IR proposal, typed
/// patch, destination mapping, and final canonical node without running a worker or minting IDs.
///
/// # Errors
///
/// Returns a typed error if any reviewed authority differs or has an unsupported contract.
#[allow(clippy::too_many_lines)]
pub fn validate_task_import_preview(bundle: &TaskImportPreviewBundle) -> Result<(), IntakeError> {
    if bundle.contract_version != TASK_IMPORT_BUNDLE_CONTRACT_VERSION {
        return super::invalid_bundle("unsupported task import bundle contract version");
    }
    if bundle.bundle_digest != bundle.compute_bundle_digest()? {
        return super::invalid_bundle("task import bundle digest differs from its exact authority");
    }
    validate_reviewed_destination_shape(bundle)?;
    validate_source_set_bounds(&bundle.source_documents)?;
    if bundle.source_set_digest != source_set_digest(&bundle.source_documents)? {
        return super::invalid_bundle("task import source-set digest is stale or forged");
    }
    validate_task_import_plan(&bundle.source_documents, &bundle.task_plan)
        .map_err(|error| super::invalid_bundle_error(error.to_string()))?;
    if bundle.evidence.len() != bundle.source_documents.len()
        || bundle.task_plan.documents.len() != bundle.source_documents.len()
    {
        return super::invalid_bundle(
            "task import evidence does not cover the complete source set",
        );
    }
    let expected_targets =
        source_destination_locators(&bundle.source_documents, &bundle.destination_root_locator)?;
    let mut expected_nodes = BTreeMap::new();
    for (((input, document_plan), destination), evidence) in bundle
        .source_documents
        .iter()
        .zip(&bundle.task_plan.documents)
        .zip(&expected_targets)
        .zip(&bundle.evidence)
    {
        if evidence.locator != input.locator
            || evidence.destination_locator != *destination
            || evidence.common_preview.base_workspace_revision != bundle.base_workspace_revision
            || evidence.common_preview.preview_receipt.created_at != bundle.preview_created_at
            || evidence.common_preview.source.display_name
                != input.locator.rsplit('/').next().ok_or_else(|| {
                    super::invalid_bundle_error("task source locator has no display filename")
                })?
        {
            return super::invalid_bundle(
                "task import document evidence lost source, destination, revision, or time authority",
            );
        }
        let (derived, expected_patches) = derive_placeholder_source(input, document_plan)?;
        if evidence.patches != expected_patches
            || evidence.derived_source_digest != sha256_bytes(derived.as_bytes())
            || evidence.common_preview.source_bytes != derived.as_bytes()
            || evidence.common_preview.plan.destination.as_str() != destination
        {
            return super::invalid_bundle(
                "task import typed patch or derived common source differs from the frozen plan",
            );
        }
        validate_markdown_preview(&evidence.common_preview)?;
        let common_node = only_common_node(&evidence.common_preview)?;
        if !common_node.resources.is_empty() {
            return super::invalid_bundle(
                "task import common evidence retained unreviewed resources",
            );
        }
        let (exact_asciidoc, neutralized_unreviewed_tasks) = apply_typed_patches(
            &common_node.exact_asciidoc,
            document_plan,
            &evidence.patches,
        )?;
        if evidence.neutralized_unreviewed_tasks != neutralized_unreviewed_tasks {
            return super::invalid_bundle(
                "task import unreviewed checklist neutralization evidence differs",
            );
        }
        let node_id = common_node.node_id.parse::<NodeId>().map_err(|error| {
            super::invalid_bundle_error(format!("invalid frozen task import node ID: {error}"))
        })?;
        expected_nodes.insert(
            destination.to_lowercase(),
            TaskImportProposedNode {
                source_locator: Some(input.locator.clone()),
                destination_locator: destination.clone(),
                node_id,
                document_file: common_node.document_file.clone(),
                document_digest: sha256_bytes(exact_asciidoc.as_bytes()),
                exact_asciidoc,
            },
        );
    }
    validate_complete_nodes(bundle, expected_nodes)?;
    let expected_proposal_digest = task_proposal_digest(
        &bundle.source_set_digest,
        &bundle.task_plan,
        &bundle.evidence,
        &bundle.nodes,
    )?;
    if bundle.proposal_digest != expected_proposal_digest
        || bundle.proposal_id
            != format!("task-proposal-{}", &expected_proposal_digest.as_str()[..24])
    {
        return super::invalid_bundle("task import proposal authority is stale or forged");
    }
    Ok(())
}

/// Commits only the exact reviewed task source-set proposal through one recoverable Core tree
/// transaction, durably publishes its external receipt, and only then finalizes Core's committed
/// journal. No adapter, worker, task conversion, or identity generation runs on this path.
///
/// # Errors
///
/// Returns a typed error for a stale target, blocking diagnostic, altered bundle, Core conflict,
/// transaction failure, receipt persistence failure, or inconsistent authority. A failure after
/// Core's commit marker deliberately leaves the journal for [`recover_previewed_task_import`].
pub fn commit_previewed_task_import(
    workspace: impl AsRef<Path>,
    bundle: &TaskImportPreviewBundle,
    review: &TaskImportReview,
    receipt_path: impl AsRef<Path>,
    created_at: impl Into<String>,
) -> Result<CommittedTaskImport, IntakeError> {
    let workspace = workspace.as_ref();
    let receipt_path = receipt_path.as_ref();
    let created_at = created_at.into();
    super::ensure_bundle_outside_workspace(workspace, receipt_path)?;
    reject_existing_receipt(receipt_path)?;
    let plan = reviewed_task_import_plan(workspace, bundle, review, &created_at)?;
    let transaction = commit_workspace_transaction_retaining_journal(&plan, receipt_path)
        .map_err(|error| super::workspace_error(&error))?;
    let committed = build_committed_task_import(bundle, transaction, &created_at)?;
    let receipt_bytes = task_import_receipt_bytes(&committed.receipt)?;
    publish_committed_workspace_transaction_receipt(
        workspace,
        &committed.transaction,
        receipt_path,
        &receipt_bytes,
    )
    .map_err(|error| super::workspace_error(&error))?;
    finalize_committed_workspace_transaction(workspace, &committed.transaction)
        .map_err(|error| super::workspace_error(&error))?;
    Ok(committed)
}

/// Idempotently resolves a task import interrupted before the durable receipt/Core-journal
/// handoff completed. Applying work is rolled back; committed work is verified against the exact
/// reviewed bundle, receives (or reuses) its durable receipt, and is then finalized.
///
/// # Errors
///
/// Returns a typed error for altered review authority, ambiguous journals, mismatched committed
/// bytes, foreign receipts, invalid timestamps, or receipt I/O. Ambiguous evidence is retained.
pub fn recover_previewed_task_import(
    workspace: impl AsRef<Path>,
    bundle: &TaskImportPreviewBundle,
    review: &TaskImportReview,
    receipt_path: impl AsRef<Path>,
    created_at: impl Into<String>,
) -> Result<TaskImportRecovery, IntakeError> {
    let workspace = workspace.as_ref();
    let receipt_path = receipt_path.as_ref();
    validate_review(bundle, review)?;
    validate_task_import_preview(bundle)?;
    if !bundle.task_plan.is_committable() {
        return Err(IntakeError::new(
            IntakeErrorCode::ProposalConflict,
            "task import preview has blocking diagnostics and has no commit to recover",
        ));
    }
    super::ensure_bundle_outside_workspace(workspace, receipt_path)?;
    let existing_receipt = match fs::symlink_metadata(receipt_path) {
        Ok(_) => {
            let bytes = super::read_regular_file_bounded(receipt_path, super::MAX_BUNDLE_BYTES)?;
            let receipt = parse_task_import_receipt_bytes(&bytes)?;
            Some((receipt, bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(super::io_error("inspect task import receipt", &error)),
    };
    let authority = task_authority(bundle);
    let transaction_state = inspect_workspace_import_transaction(workspace, &authority)
        .map_err(|error| super::workspace_error(&error))?;
    if existing_receipt.is_some()
        && matches!(
            &transaction_state,
            WorkspaceImportTransactionState::Prepared { .. }
                | WorkspaceImportTransactionState::Applying { .. }
        )
    {
        return super::invalid_bundle(
            "durable task import receipt contradicts an uncommitted Core journal",
        );
    }
    let recovery = recover_workspace_import_transaction(workspace, &authority)
        .map_err(|error| super::workspace_error(&error))?;
    if recovery.prepared_removed > 0 || recovery.applying_rolled_back > 0 {
        if existing_receipt.is_some() || !recovery.committed_transactions.is_empty() {
            return super::invalid_bundle(
                "task import recovery found contradictory rollback and commit receipt evidence",
            );
        }
        return Ok(TaskImportRecovery::RolledBack { recovery });
    }
    if recovery.committed_transactions.len() > 1 {
        return super::invalid_bundle(
            "task import recovery found multiple committed workspace transactions",
        );
    }
    if let Some(transaction) = recovery.committed_transactions.first().cloned() {
        let created_at = created_at.into();
        return recover_committed_task_import(
            workspace,
            bundle,
            receipt_path,
            &transaction,
            existing_receipt,
            recovery,
            &created_at,
        );
    }
    let Some((receipt, _)) = existing_receipt else {
        return Err(IntakeError::new(
            IntakeErrorCode::Workspace,
            "no unfinished task import transaction or durable receipt exists",
        ));
    };
    let committed = committed_from_receipt(bundle, &receipt)?;
    validate_workspace_root_identity(workspace, bundle)?;
    validate_committed_task_transaction_authority(bundle, &committed.transaction)?;
    Ok(TaskImportRecovery::AlreadyFinalized {
        committed,
        recovery,
    })
}

fn recover_committed_task_import(
    workspace: &Path,
    bundle: &TaskImportPreviewBundle,
    receipt_path: &Path,
    transaction: &CommittedWorkspaceTransaction,
    existing_receipt: Option<(TaskImportReceipt, Vec<u8>)>,
    recovery: RecoveryReport,
    created_at: &str,
) -> Result<TaskImportRecovery, IntakeError> {
    validate_target_binding(workspace, bundle)?;
    validate_committed_task_transaction(workspace, bundle, transaction)?;
    let staged = read_committed_workspace_transaction_receipt_handoff(workspace, transaction)
        .map_err(|error| super::workspace_error(&error))?;
    let recovered_receipt = existing_receipt.is_none();
    let (committed, receipt_bytes) = if let Some(handoff) = staged {
        let receipt = parse_task_import_receipt_bytes(&handoff.bytes)?;
        let committed = committed_from_receipt(bundle, &receipt)?;
        if let Some((existing, existing_bytes)) = &existing_receipt
            && (existing != &receipt || existing_bytes != &handoff.bytes)
        {
            return super::invalid_bundle(
                "external task receipt differs from the Core-staged exact receipt bytes",
            );
        }
        (committed, handoff.bytes)
    } else if let Some((receipt, bytes)) = existing_receipt {
        (committed_from_receipt(bundle, &receipt)?, bytes)
    } else {
        validate_receipt_timestamp(bundle, created_at)?;
        let committed = build_committed_task_import(bundle, transaction.clone(), created_at)?;
        let bytes = task_import_receipt_bytes(&committed.receipt)?;
        (committed, bytes)
    };
    if committed.transaction != *transaction {
        return super::invalid_bundle(
            "durable task import receipt differs from the retained Core journal",
        );
    }
    publish_committed_workspace_transaction_receipt(
        workspace,
        transaction,
        receipt_path,
        &receipt_bytes,
    )
    .map_err(|error| super::workspace_error(&error))?;
    finalize_committed_workspace_transaction(workspace, transaction)
        .map_err(|error| super::workspace_error(&error))?;
    if recovered_receipt {
        Ok(TaskImportRecovery::ReceiptRecovered {
            committed,
            recovery,
        })
    } else {
        Ok(TaskImportRecovery::AlreadyFinalized {
            committed,
            recovery,
        })
    }
}

/// Reads and fully validates a task import bundle from one regular non-link file.
///
/// # Errors
///
/// Returns a typed error for unsafe file state, bounds, JSON, unknown fields, or authority drift.
pub fn read_task_import_bundle(
    path: impl AsRef<Path>,
) -> Result<TaskImportPreviewBundle, IntakeError> {
    let bytes = super::read_regular_file_bounded(path, super::MAX_BUNDLE_BYTES)?;
    super::reject_duplicate_json_keys(&bytes)
        .map_err(|error| super::serialization("parse task import bundle JSON", &error))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| super::serialization("parse task import bundle JSON", &error))?;
    let bundle: TaskImportPreviewBundle = serde_json::from_value(value.clone())
        .map_err(|error| super::serialization("decode task import bundle contract", &error))?;
    let exact = serde_json::to_value(&bundle)
        .map_err(|error| super::serialization("normalize task import bundle contract", &error))?;
    if value != exact {
        return super::invalid_bundle(
            "task import bundle contains fields outside its exact contract",
        );
    }
    validate_task_import_preview(&bundle)?;
    Ok(bundle)
}

/// Atomically publishes one immutable task import bundle outside the target workspace.
///
/// # Errors
///
/// Returns a typed error for serialization, bounds, unsafe placement, overwrite, or I/O failure.
pub fn write_task_import_bundle(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    bundle: &TaskImportPreviewBundle,
) -> Result<(), IntakeError> {
    let bytes = serde_json::to_vec_pretty(bundle)
        .map_err(|error| super::serialization("serialize task import bundle", &error))?;
    super::write_bundle_bytes(workspace.as_ref(), path.as_ref(), &bytes)
}

/// Reads one exact durable task import receipt from a regular non-link file.
///
/// # Errors
///
/// Returns a typed error for unsafe file state, bounds, JSON, unknown fields, or normalization.
pub fn read_task_import_receipt(path: impl AsRef<Path>) -> Result<TaskImportReceipt, IntakeError> {
    let bytes = super::read_regular_file_bounded(path, super::MAX_BUNDLE_BYTES)?;
    parse_task_import_receipt_bytes(&bytes)
}

fn parse_task_import_receipt_bytes(bytes: &[u8]) -> Result<TaskImportReceipt, IntakeError> {
    super::reject_duplicate_json_keys(bytes)
        .map_err(|error| super::serialization("parse task import receipt JSON", &error))?;
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| super::serialization("parse task import receipt JSON", &error))?;
    let receipt: TaskImportReceipt = serde_json::from_value(value.clone())
        .map_err(|error| super::serialization("decode task import receipt contract", &error))?;
    let exact = serde_json::to_value(&receipt)
        .map_err(|error| super::serialization("normalize task import receipt contract", &error))?;
    if value != exact {
        return super::invalid_bundle(
            "task import receipt contains fields outside its exact contract",
        );
    }
    Ok(receipt)
}

fn task_import_receipt_bytes(receipt: &TaskImportReceipt) -> Result<Vec<u8>, IntakeError> {
    serde_json::to_vec_pretty(receipt)
        .map_err(|error| super::serialization("serialize task import receipt", &error))
}

/// Atomically publishes a task import receipt outside the workspace without overwrite.
///
/// # Errors
///
/// Returns a typed error for serialization, unsafe placement, overwrite, bounds, or I/O.
pub fn write_task_import_receipt(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    receipt: &TaskImportReceipt,
) -> Result<(), IntakeError> {
    let bytes = task_import_receipt_bytes(receipt)?;
    super::write_bundle_bytes(workspace.as_ref(), path.as_ref(), &bytes)
}

fn reject_existing_receipt(path: &Path) -> Result<(), IntakeError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Ok(_) => Err(IntakeError::new(
            IntakeErrorCode::Io,
            "task import receipt destination already exists",
        )),
        Err(error) => Err(super::io_error(
            "inspect task import receipt destination",
            &error,
        )),
    }
}

fn validate_review(
    bundle: &TaskImportPreviewBundle,
    review: &TaskImportReview,
) -> Result<(), IntakeError> {
    if review != &TaskImportReview::from_preview(bundle) {
        return super::invalid_bundle(
            "task import commit authority differs from the explicitly reviewed bundle",
        );
    }
    Ok(())
}

fn reviewed_task_import_plan(
    workspace: &Path,
    bundle: &TaskImportPreviewBundle,
    review: &TaskImportReview,
    created_at: &str,
) -> Result<WorkspaceTransactionPlan, IntakeError> {
    validate_review(bundle, review)?;
    let current_revision =
        read_workspace_revision(workspace).map_err(|error| super::workspace_error(&error))?;
    if current_revision != bundle.base_workspace_revision {
        return Err(IntakeError::new(
            IntakeErrorCode::StalePreview,
            format!(
                "stale task import preview: expected workspace revision {}, found {current_revision}",
                bundle.base_workspace_revision
            ),
        ));
    }
    validate_task_import_preview(bundle)?;
    validate_target_binding(workspace, bundle)?;
    if !bundle.task_plan.is_committable() {
        return Err(IntakeError::new(
            IntakeErrorCode::ProposalConflict,
            "task import preview has blocking diagnostics and cannot be committed",
        ));
    }
    validate_receipt_timestamp(bundle, created_at)?;
    let authority = task_authority(bundle);
    let plan = plan_import_tree(
        workspace,
        &bundle.base_workspace_revision,
        authority.clone(),
        workspace_nodes(&bundle.nodes),
    )
    .map_err(|error| super::workspace_error(&error))?;
    if plan.import_authority.as_ref() != Some(&authority) {
        return super::invalid_bundle("Core task import plan lost exact proposal authority");
    }
    validate_receipt_size_bound(bundle, &plan, created_at)?;
    Ok(plan)
}

fn validate_receipt_size_bound(
    bundle: &TaskImportPreviewBundle,
    plan: &WorkspaceTransactionPlan,
    created_at: &str,
) -> Result<(), IntakeError> {
    let placeholder = CommittedWorkspaceTransaction {
        plan_id: plan.plan_id.clone(),
        action: plan.action,
        base_revision: plan.base_revision.clone(),
        revision: plan.base_revision.clone(),
        path_changes: plan.path_changes.clone(),
        scope_summary: plan.scope_summary.clone(),
        promotion_summary: None,
        identity_map: plan.identity_map.clone(),
        captured_target: plan.captured_target.clone(),
        target_node_ids: plan.target_node_ids.clone(),
        draft_sensitive_node_ids: plan.draft_sensitive_node_ids.clone(),
        import_authority: plan.import_authority.clone(),
    };
    let committed = build_committed_task_import(bundle, placeholder, created_at)?;
    let bytes = task_import_receipt_bytes(&committed.receipt)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > super::MAX_BUNDLE_BYTES {
        return super::invalid_bundle(
            "reviewed task import would produce a receipt beyond the durable byte limit",
        );
    }
    Ok(())
}

fn validate_receipt_timestamp(
    bundle: &TaskImportPreviewBundle,
    created_at: &str,
) -> Result<(), IntakeError> {
    if created_at != bundle.preview_created_at {
        return super::invalid_bundle(
            "task import receipt timestamp must equal the reviewed preview timestamp",
        );
    }
    let evidence = bundle
        .evidence
        .first()
        .ok_or_else(|| super::invalid_bundle_error("task import has no common receipt evidence"))?;
    let validated = validate_markdown_preview(&evidence.common_preview)?;
    ImportReceipt::create(
        created_at,
        &evidence.common_preview.source,
        &evidence.common_preview.plan,
        &evidence.common_preview.document,
        &validated,
        evidence.common_preview.components.clone(),
        CommitResult::PreviewOnly,
    )
    .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
    Ok(())
}

fn build_committed_task_import(
    bundle: &TaskImportPreviewBundle,
    transaction: CommittedWorkspaceTransaction,
    created_at: &str,
) -> Result<CommittedTaskImport, IntakeError> {
    if transaction.import_authority.as_ref() != Some(&task_authority(bundle)) {
        return super::invalid_bundle("committed task import lost exact proposal authority");
    }
    let mut common_receipts = Vec::with_capacity(bundle.evidence.len());
    for evidence in &bundle.evidence {
        let validated = validate_markdown_preview(&evidence.common_preview)?;
        common_receipts.push(
            ImportReceipt::create(
                created_at,
                &evidence.common_preview.source,
                &evidence.common_preview.plan,
                &evidence.common_preview.document,
                &validated,
                evidence.common_preview.components.clone(),
                CommitResult::Committed {
                    transaction_id: transaction.plan_id.clone(),
                    workspace_revision: transaction.revision.to_string(),
                },
            )
            .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?,
        );
    }
    let receipt_id = task_receipt_id(bundle, created_at, &common_receipts, &transaction)?;
    let receipt = TaskImportReceipt {
        contract_version: TASK_IMPORT_RECEIPT_CONTRACT_VERSION.to_owned(),
        receipt_id,
        created_at: created_at.to_owned(),
        source_set_digest: bundle.source_set_digest.clone(),
        reviewed_bundle_digest: bundle.bundle_digest.clone(),
        proposal_id: bundle.proposal_id.clone(),
        proposal_digest: bundle.proposal_digest.clone(),
        identities: bundle.task_plan.identities.clone(),
        nodes: bundle.nodes.clone(),
        common_receipts,
        transaction: transaction.clone(),
    };
    Ok(CommittedTaskImport {
        proposal_id: bundle.proposal_id.clone(),
        proposal_digest: bundle.proposal_digest.clone(),
        transaction,
        receipt,
    })
}

fn committed_from_receipt(
    bundle: &TaskImportPreviewBundle,
    receipt: &TaskImportReceipt,
) -> Result<CommittedTaskImport, IntakeError> {
    validate_receipt_timestamp(bundle, &receipt.created_at)?;
    if receipt.contract_version != TASK_IMPORT_RECEIPT_CONTRACT_VERSION
        || receipt.source_set_digest != bundle.source_set_digest
        || receipt.reviewed_bundle_digest != bundle.bundle_digest
        || receipt.proposal_id != bundle.proposal_id
        || receipt.proposal_digest != bundle.proposal_digest
        || receipt.identities != bundle.task_plan.identities
        || receipt.nodes != bundle.nodes
    {
        return super::invalid_bundle("durable task import receipt has foreign review authority");
    }
    let expected =
        build_committed_task_import(bundle, receipt.transaction.clone(), &receipt.created_at)?;
    if &expected.receipt != receipt {
        return super::invalid_bundle("durable task import receipt differs from exact evidence");
    }
    Ok(expected)
}

fn validate_committed_task_transaction(
    workspace: &Path,
    bundle: &TaskImportPreviewBundle,
    transaction: &CommittedWorkspaceTransaction,
) -> Result<(), IntakeError> {
    validate_committed_task_transaction_authority(bundle, transaction)?;
    let current =
        read_workspace_revision(workspace).map_err(|error| super::workspace_error(&error))?;
    if current != transaction.revision {
        return Err(IntakeError::new(
            IntakeErrorCode::StalePreview,
            "committed task import workspace changed before receipt recovery",
        ));
    }
    for node in &bundle.nodes {
        let path = workspace
            .join(Path::new(&node.destination_locator))
            .join(&node.document_file);
        let bytes = super::read_regular_file_bounded(&path, 32 * 1024 * 1024)?;
        if bytes != node.exact_asciidoc.as_bytes() || sha256_bytes(&bytes) != node.document_digest {
            return super::invalid_bundle(format!(
                "committed task import document differs from reviewed bytes: {}",
                node.destination_locator
            ));
        }
    }
    Ok(())
}

fn validate_committed_task_transaction_authority(
    bundle: &TaskImportPreviewBundle,
    transaction: &CommittedWorkspaceTransaction,
) -> Result<(), IntakeError> {
    let plan_id = transaction.plan_id.parse::<NodeId>().map_err(|_| {
        super::invalid_bundle_error("committed task transaction plan ID is not UUIDv4")
    })?;
    if plan_id.to_string() != transaction.plan_id
        || WorkspaceRevision::parse(transaction.base_revision.as_str()).is_err()
        || WorkspaceRevision::parse(transaction.revision.as_str()).is_err()
    {
        return super::invalid_bundle(
            "committed task transaction carries non-canonical identity or revision evidence",
        );
    }
    if transaction.action != StructuralAction::Import
        || transaction.base_revision != bundle.base_workspace_revision
        || transaction.import_authority.as_ref() != Some(&task_authority(bundle))
    {
        return super::invalid_bundle(
            "committed Core transaction differs from reviewed task import authority",
        );
    }
    let mut expected_changes = bundle
        .nodes
        .iter()
        .map(|node| weftext_core::WorkspacePathChange {
            source_node_id: None,
            node_id: node.node_id,
            old_path: None,
            new_path: node.destination_locator.clone(),
        })
        .collect::<Vec<_>>();
    expected_changes.sort_by(|left, right| {
        left.new_path
            .cmp(&right.new_path)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    if transaction.path_changes != expected_changes {
        return super::invalid_bundle(
            "committed task import path/identity mapping differs from reviewed nodes",
        );
    }
    Ok(())
}

fn validate_workspace_root_identity(
    workspace: &Path,
    bundle: &TaskImportPreviewBundle,
) -> Result<(), IntakeError> {
    let inventory = scan_workspace(workspace);
    let root_id = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id);
    if !inventory.is_valid() || root_id != Some(bundle.workspace_root_id) {
        return super::invalid_bundle(
            "durable task receipt does not belong to this valid workspace root",
        );
    }
    Ok(())
}

fn validate_source_set_bounds(documents: &[TaskImportDocumentInput]) -> Result<(), IntakeError> {
    if documents.is_empty() || documents.len() > MAX_TASK_SOURCE_DOCUMENTS {
        return super::invalid_bundle(format!(
            "task import source set must contain 1..={MAX_TASK_SOURCE_DOCUMENTS} documents"
        ));
    }
    let mut total = 0_usize;
    let mut folded = BTreeSet::new();
    for document in documents {
        let bytes = document.source.len();
        if u64::try_from(bytes).unwrap_or(u64::MAX) > MARKDOWN_SOURCE_BYTE_LIMIT {
            return Err(IntakeError::new(
                IntakeErrorCode::LimitExceeded,
                format!(
                    "task import source exceeds the Markdown limit: {}",
                    document.locator
                ),
            ));
        }
        total = total.checked_add(bytes).ok_or_else(|| {
            IntakeError::new(
                IntakeErrorCode::LimitExceeded,
                "task source-set size overflowed",
            )
        })?;
        if total > MAX_TASK_SOURCE_SET_BYTES {
            return Err(IntakeError::new(
                IntakeErrorCode::LimitExceeded,
                format!("task import source set exceeds {MAX_TASK_SOURCE_SET_BYTES} bytes"),
            ));
        }
        if !folded.insert(document.locator.to_lowercase()) {
            return super::invalid_bundle(format!(
                "task source locators collide by portable case-folding: {}",
                document.locator
            ));
        }
    }
    Ok(())
}

fn bind_destination(
    workspace: &Path,
    parent_id: NodeId,
    destination_name: &str,
) -> Result<(NodeId, String, String), IntakeError> {
    let portable = PortablePath::parse(destination_name.to_owned())
        .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
    if portable.as_str() != destination_name || destination_name.contains('/') {
        return super::invalid_bundle(
            "task import destination name must be one portable component",
        );
    }
    let inventory = scan_workspace(workspace);
    if !inventory.is_valid() {
        return Err(IntakeError::new(
            IntakeErrorCode::Workspace,
            "task import target is not a valid canonical workspace",
        ));
    }
    let root = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .ok_or_else(|| IntakeError::new(IntakeErrorCode::Workspace, "workspace root has no ID"))?;
    let parent = inventory
        .nodes
        .iter()
        .find(|node| node.id == Some(parent_id))
        .ok_or_else(|| {
            IntakeError::new(
                IntakeErrorCode::Workspace,
                "destination parent is unavailable",
            )
        })?;
    let parent_locator = relative_locator(&inventory.root, &parent.path)?;
    let root_locator = if parent_locator.is_empty() {
        destination_name.to_owned()
    } else {
        format!("{parent_locator}/{destination_name}")
    };
    PortablePath::parse(root_locator.clone())
        .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
    Ok((root, parent_locator, root_locator))
}

fn validate_reviewed_destination_shape(
    bundle: &TaskImportPreviewBundle,
) -> Result<(), IntakeError> {
    let name = PortablePath::parse(bundle.destination_name.clone())
        .map_err(|error| super::invalid_bundle_error(error.to_string()))?;
    if name.as_str() != bundle.destination_name || bundle.destination_name.contains('/') {
        return super::invalid_bundle("task import destination name is not one portable component");
    }
    if bundle.destination_parent_locator.is_empty() {
        if bundle.destination_parent_id != bundle.workspace_root_id
            || bundle.destination_root_locator != bundle.destination_name
        {
            return super::invalid_bundle(
                "task import root-parent destination binding is internally inconsistent",
            );
        }
    } else {
        PortablePath::parse(bundle.destination_parent_locator.clone())
            .map_err(|error| super::invalid_bundle_error(error.to_string()))?;
        if bundle.destination_parent_id == bundle.workspace_root_id
            || bundle.destination_root_locator
                != format!(
                    "{}/{}",
                    bundle.destination_parent_locator, bundle.destination_name
                )
        {
            return super::invalid_bundle(
                "task import nested-parent destination binding is internally inconsistent",
            );
        }
    }
    PortablePath::parse(bundle.destination_root_locator.clone())
        .map_err(|error| super::invalid_bundle_error(error.to_string()))?;
    Ok(())
}

fn validate_target_binding(
    workspace: &Path,
    bundle: &TaskImportPreviewBundle,
) -> Result<(), IntakeError> {
    let (root_id, parent_locator, root_locator) = bind_destination(
        workspace,
        bundle.destination_parent_id,
        &bundle.destination_name,
    )?;
    if root_id != bundle.workspace_root_id
        || parent_locator != bundle.destination_parent_locator
        || root_locator != bundle.destination_root_locator
    {
        return Err(IntakeError::new(
            IntakeErrorCode::StalePreview,
            "task import destination parent or workspace identity changed after preview",
        ));
    }
    Ok(())
}

fn relative_locator(root: &Path, path: &Path) -> Result<String, IntakeError> {
    let relative = path.strip_prefix(root).map_err(|_| {
        IntakeError::new(
            IntakeErrorCode::Workspace,
            "destination parent escapes workspace",
        )
    })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(value) = component else {
            return super::invalid_bundle("destination parent locator is not portable");
        };
        parts.push(value.to_str().ok_or_else(|| {
            IntakeError::new(
                IntakeErrorCode::Workspace,
                "destination parent path is not UTF-8",
            )
        })?);
    }
    Ok(parts.join("/"))
}

fn source_destination_locators(
    documents: &[TaskImportDocumentInput],
    root_locator: &str,
) -> Result<Vec<String>, IntakeError> {
    let mut destinations = Vec::with_capacity(documents.len());
    let mut folded = BTreeSet::new();
    for document in documents {
        let source = PortablePath::parse(document.locator.clone())
            .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
        let (parent, file) = source
            .as_str()
            .rsplit_once('/')
            .map_or((None, source.as_str()), |(parent, file)| {
                (Some(parent), file)
            });
        let (stem, extension) = file.rsplit_once('.').ok_or_else(|| {
            super::invalid_bundle_error("task source locator must end in an explicit .md extension")
        })?;
        if stem.is_empty() || !extension.eq_ignore_ascii_case("md") {
            return super::invalid_bundle(
                "task source locator must end in an explicit non-empty .md filename",
            );
        }
        let relative = parent.map_or_else(|| stem.to_owned(), |parent| format!("{parent}/{stem}"));
        let destination = format!("{root_locator}/{relative}");
        PortablePath::parse(destination.clone())
            .map_err(|error| IntakeError::new(IntakeErrorCode::Import, error.to_string()))?;
        if !folded.insert(destination.to_lowercase()) {
            return super::invalid_bundle(
                "multiple task source documents map to one portable destination node",
            );
        }
        destinations.push(destination);
    }
    Ok(destinations)
}

fn derive_placeholder_source(
    input: &TaskImportDocumentInput,
    plan: &TaskImportDocumentPlan,
) -> Result<(String, Vec<TaskImportPatchEvidence>), IntakeError> {
    let mut derived = String::with_capacity(input.source.len());
    let mut patches = Vec::with_capacity(plan.edits.len());
    let mut cursor = 0_usize;
    for (index, edit) in plan.edits.iter().enumerate() {
        let start = usize::try_from(edit.source_range.start)
            .map_err(|_| super::invalid_bundle_error("task edit start exceeds host range"))?;
        let end = usize::try_from(edit.source_range.end)
            .map_err(|_| super::invalid_bundle_error("task edit end exceeds host range"))?;
        if start < cursor
            || end < start
            || end > input.source.len()
            || !input.source.is_char_boundary(start)
            || !input.source.is_char_boundary(end)
        {
            return super::invalid_bundle("task edit ranges are overlapping or not UTF-8 safe");
        }
        derived.push_str(&input.source[cursor..start]);
        let token = task_patch_token(&input.locator, edit, index)?;
        if input.source.contains(&token) {
            return super::invalid_bundle("task patch token collides with exact source bytes");
        }
        derived.push_str("```");
        derived.push_str(TASK_PATCH_LANGUAGE);
        derived.push('\n');
        derived.push_str(&token);
        derived.push_str("\n```\n");
        patches.push(TaskImportPatchEvidence {
            edit_index: u64::try_from(index).unwrap_or(u64::MAX),
            token,
            replacement_digest: sha256_bytes(edit.replacement.as_bytes()),
        });
        cursor = end;
    }
    derived.push_str(&input.source[cursor..]);
    Ok((derived, patches))
}

fn task_patch_token(
    locator: &str,
    edit: &TaskImportEdit,
    index: usize,
) -> Result<String, IntakeError> {
    let material = serde_json::to_vec(&(
        locator,
        index,
        edit.kind,
        &edit.source_range,
        &edit.replacement,
    ))
    .map_err(|error| super::serialization("serialize task patch token", &error))?;
    Ok(format!(
        "WEFTEXT_TASK_PATCH_{}",
        sha256_bytes(&material).as_str().to_ascii_uppercase()
    ))
}

fn apply_typed_patches(
    common_source: &str,
    plan: &TaskImportDocumentPlan,
    patches: &[TaskImportPatchEvidence],
) -> Result<(String, u64), IntakeError> {
    if plan.edits.len() != patches.len() {
        return super::invalid_bundle("task patch evidence does not cover every reviewed edit");
    }
    let mut output = String::with_capacity(common_source.len());
    let mut common_cursor = 0_usize;
    let mut reviewed_ranges = Vec::with_capacity(patches.len());
    for (index, (edit, patch)) in plan.edits.iter().zip(patches).enumerate() {
        if patch.edit_index != u64::try_from(index).unwrap_or(u64::MAX)
            || patch.replacement_digest != sha256_bytes(edit.replacement.as_bytes())
        {
            return super::invalid_bundle("task patch index or replacement digest is forged");
        }
        let listing = format!(
            "[source,{TASK_PATCH_LANGUAGE}]\n----\n{}\n----\n\n",
            patch.token
        );
        if common_source.match_indices(&listing).count() != 1 {
            return super::invalid_bundle(
                "common Import IR proposal lost or duplicated a typed task patch placeholder",
            );
        }
        let offset = common_source[common_cursor..]
            .find(&listing)
            .map(|relative| common_cursor + relative)
            .ok_or_else(|| {
                super::invalid_bundle_error(
                    "common Import IR reordered typed task patch placeholders",
                )
            })?;
        output.push_str(&common_source[common_cursor..offset]);
        let reviewed_start = output.len();
        output.push_str(&edit.replacement);
        reviewed_ranges.push(reviewed_start..output.len());
        common_cursor = offset + listing.len();
    }
    output.push_str(&common_source[common_cursor..]);
    if output.contains("WEFTEXT_TASK_PATCH_") {
        return super::invalid_bundle("typed task patch placeholder leaked into canonical output");
    }
    for block in analyze_query_source(&output).blocks {
        if !range_is_reviewed(&block.range, &reviewed_ranges) {
            return super::invalid_bundle(
                "common Markdown conversion produced an unreviewed canonical query block",
            );
        }
    }
    let analysis = analyze_task_source(&output);
    let mut neutralize = analysis
        .tasks
        .into_iter()
        .filter(|task| !range_is_reviewed(&task.range, &reviewed_ranges))
        .map(|task| usize::try_from(task.marker_range.start).map(|marker| marker.saturating_sub(1)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| super::invalid_bundle_error("canonical task range exceeds host range"))?;
    neutralize.sort_unstable();
    neutralize.dedup();
    for marker in neutralize.iter().rev().copied() {
        if output.as_bytes().get(marker) != Some(&b'[') || !output.is_char_boundary(marker) {
            return super::invalid_bundle(
                "canonical task parser returned a non-checkbox marker boundary",
            );
        }
        output.insert(marker, '\\');
    }
    Ok((output, u64::try_from(neutralize.len()).unwrap_or(u64::MAX)))
}

fn range_is_reviewed(range: &std::ops::Range<u64>, reviewed: &[std::ops::Range<usize>]) -> bool {
    let Ok(start) = usize::try_from(range.start) else {
        return false;
    };
    let Ok(end) = usize::try_from(range.end) else {
        return false;
    };
    reviewed
        .iter()
        .any(|allowed| allowed.start <= start && end <= allowed.end)
}

fn only_common_node(
    preview: &ImportPreviewBundle,
) -> Result<&weftext_import::ProposedNode, IntakeError> {
    if !preview.proposal.conflicts.is_empty()
        || preview.proposal.nodes.len() != 1
        || preview.proposal.nodes[0].locator != preview.plan.destination
    {
        return super::invalid_bundle(
            "task import common evidence must be one conflict-free destination node",
        );
    }
    Ok(&preview.proposal.nodes[0])
}

fn complete_destination_tree(
    root_locator: &str,
    source_nodes: &BTreeMap<String, TaskImportProposedNode>,
) -> Result<Vec<TaskImportProposedNode>, IntakeError> {
    let mut required = BTreeMap::<String, String>::new();
    required.insert(root_locator.to_lowercase(), root_locator.to_owned());
    for node in source_nodes.values() {
        let mut current = node.destination_locator.as_str();
        loop {
            if !current.starts_with(root_locator) {
                return super::invalid_bundle("task source destination escaped its import root");
            }
            if let Some(existing) = required.insert(current.to_lowercase(), current.to_owned())
                && existing != current
            {
                return super::invalid_bundle(
                    "task destination nodes collide by portable case-folding",
                );
            }
            if current == root_locator {
                break;
            }
            current = current
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .ok_or_else(|| {
                    super::invalid_bundle_error("task destination tree is disconnected")
                })?;
        }
    }
    let mut nodes = Vec::with_capacity(required.len());
    for (folded, locator) in required {
        if let Some(source) = source_nodes.get(&folded) {
            nodes.push(source.clone());
            continue;
        }
        let node_id = NodeId::new_v4();
        let name = locator
            .rsplit('/')
            .next()
            .ok_or_else(|| {
                super::invalid_bundle_error("synthetic task import node has no filename")
            })?
            .to_owned();
        let exact_asciidoc = synthetic_node_source(node_id);
        nodes.push(TaskImportProposedNode {
            source_locator: None,
            destination_locator: locator,
            node_id,
            document_file: format!("{name}.adoc"),
            document_digest: sha256_bytes(exact_asciidoc.as_bytes()),
            exact_asciidoc,
        });
    }
    Ok(nodes)
}

fn validate_complete_nodes(
    bundle: &TaskImportPreviewBundle,
    mut expected_source_nodes: BTreeMap<String, TaskImportProposedNode>,
) -> Result<(), IntakeError> {
    if bundle.nodes.is_empty() || bundle.nodes.len() > 10_000 {
        return super::invalid_bundle("task import node tree is empty or exceeds Core bounds");
    }
    let mut folded = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let all_locators = bundle
        .nodes
        .iter()
        .map(|node| node.destination_locator.as_str())
        .collect::<BTreeSet<_>>();
    for node in &bundle.nodes {
        PortablePath::parse(node.destination_locator.clone())
            .map_err(|error| super::invalid_bundle_error(error.to_string()))?;
        if !folded.insert(node.destination_locator.to_lowercase()) || !ids.insert(node.node_id) {
            return super::invalid_bundle("task import node path or identity is duplicated");
        }
        if !node
            .destination_locator
            .starts_with(&bundle.destination_root_locator)
            || (node.destination_locator != bundle.destination_root_locator
                && !node
                    .destination_locator
                    .starts_with(&format!("{}/", bundle.destination_root_locator)))
        {
            return super::invalid_bundle("task import node escapes the reviewed destination root");
        }
        if node.destination_locator != bundle.destination_root_locator {
            let parent = node
                .destination_locator
                .rsplit_once('/')
                .map(|(parent, _)| parent)
                .ok_or_else(|| super::invalid_bundle_error("task import node has no parent"))?;
            if !all_locators.contains(parent) {
                return super::invalid_bundle("task import node tree has a missing direct parent");
            }
        }
        let name = node.destination_locator.rsplit('/').next().ok_or_else(|| {
            super::invalid_bundle_error("task import node locator has no filename")
        })?;
        if node.document_file != format!("{name}.adoc")
            || node.document_digest != sha256_bytes(node.exact_asciidoc.as_bytes())
        {
            return super::invalid_bundle("task import canonical document name or digest differs");
        }
        if let Some(source_locator) = &node.source_locator {
            let expected = expected_source_nodes
                .remove(&node.destination_locator.to_lowercase())
                .ok_or_else(|| {
                    super::invalid_bundle_error("task import source-to-node mapping is forged")
                })?;
            if expected != *node || expected.source_locator.as_ref() != Some(source_locator) {
                return super::invalid_bundle(
                    "task import source node differs from common evidence",
                );
            }
        } else if node.exact_asciidoc != synthetic_node_source(node.node_id) {
            return super::invalid_bundle("synthetic task import node bytes are not deterministic");
        }
        if bundle.task_plan.is_committable() {
            let task = analyze_task_source(&node.exact_asciidoc);
            let query = analyze_query_source(&node.exact_asciidoc);
            if !task.diagnostics.is_empty() || !query.diagnostics.is_empty() {
                return super::invalid_bundle(
                    "committable task import output fails canonical task/query validation",
                );
            }
        }
    }
    if !expected_source_nodes.is_empty()
        || !bundle
            .nodes
            .iter()
            .any(|node| node.destination_locator == bundle.destination_root_locator)
    {
        return super::invalid_bundle("task import node tree does not cover every source document");
    }
    Ok(())
}

fn synthetic_node_source(node_id: NodeId) -> String {
    format!("---\nweftext:\n  id: \"{node_id}\"\n---\n= Imported task sources\n\n")
}

fn source_set_digest(documents: &[TaskImportDocumentInput]) -> Result<Sha256Digest, IntakeError> {
    let material = serde_json::to_vec(documents)
        .map_err(|error| super::serialization("serialize task import source set", &error))?;
    Ok(sha256_bytes(&material))
}

fn task_proposal_digest(
    source_set_digest: &Sha256Digest,
    task_plan: &TaskImportPlan,
    evidence: &[TaskImportDocumentEvidence],
    nodes: &[TaskImportProposedNode],
) -> Result<Sha256Digest, IntakeError> {
    let evidence_authority = evidence
        .iter()
        .map(|item| {
            (
                &item.locator,
                &item.destination_locator,
                &item.derived_source_digest,
                &item.patches,
                &item.common_preview.plan.plan_id,
                &item.common_preview.proposal.proposal_id,
                &item.common_preview.proposal_digest,
            )
        })
        .collect::<Vec<_>>();
    let material = serde_json::to_vec(&(
        TASK_IMPORT_BUNDLE_CONTRACT_VERSION,
        source_set_digest,
        task_plan,
        evidence_authority,
        nodes,
    ))
    .map_err(|error| super::serialization("serialize task import proposal authority", &error))?;
    Ok(sha256_bytes(&material))
}

fn task_authority(bundle: &TaskImportPreviewBundle) -> WorkspaceImportAuthority {
    WorkspaceImportAuthority {
        proposal_id: bundle.proposal_id.clone(),
        // The full bundle digest also binds workspace identity, base revision, destination parent,
        // exact source set, task identity mapping, common IR evidence, and final node bytes.
        proposal_digest: bundle.bundle_digest.to_string(),
    }
}

fn workspace_nodes(nodes: &[TaskImportProposedNode]) -> Vec<WorkspaceImportNode> {
    nodes
        .iter()
        .map(|node| WorkspaceImportNode {
            locator: node.destination_locator.clone(),
            node_id: node.node_id,
            document_file: node.document_file.clone(),
            exact_source: node.exact_asciidoc.clone(),
            document_sha256: node.document_digest.to_string(),
            resources: Vec::new(),
        })
        .collect()
}

fn task_receipt_id(
    bundle: &TaskImportPreviewBundle,
    created_at: &str,
    common_receipts: &[ImportReceipt],
    transaction: &CommittedWorkspaceTransaction,
) -> Result<String, IntakeError> {
    let material = serde_json::to_vec(&(
        TASK_IMPORT_RECEIPT_CONTRACT_VERSION,
        created_at,
        &bundle.bundle_digest,
        &bundle.source_set_digest,
        &bundle.proposal_id,
        &bundle.proposal_digest,
        &bundle.task_plan.identities,
        &bundle.nodes,
        common_receipts,
        transaction,
    ))
    .map_err(|error| super::serialization("serialize task import receipt authority", &error))?;
    Ok(format!(
        "task-receipt-{}",
        &sha256_bytes(&material).as_str()[..24]
    ))
}
