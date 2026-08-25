//! Shared, workspace-write-free import preview orchestration and exact Core commit binding.

#![forbid(unsafe_code)]

mod agent_enhancement;
mod task_import;

pub use agent_enhancement::{
    AGENT_ENHANCEMENT_PREVIEW_CONTRACT_VERSION, AGENT_IMPORT_EVIDENCE_CONTRACT_VERSION,
    AgentEnhancementPreview, AgentEvidenceSelection, AgentImportEvidence,
    apply_approved_agent_patch, prepare_agent_enhancement, read_agent_enhancement_preview,
    read_agent_evidence_selection, read_agent_import_patch, validate_agent_enhancement_preview,
    write_agent_enhancement_preview, write_agent_import_evidence,
};
pub use task_import::{
    CommittedTaskImport, TASK_IMPORT_BUNDLE_CONTRACT_VERSION, TASK_IMPORT_RECEIPT_CONTRACT_VERSION,
    TaskImportDocumentEvidence, TaskImportPatchEvidence, TaskImportPreviewBundle,
    TaskImportProposedNode, TaskImportReceipt, TaskImportRecovery, TaskImportReview,
    commit_previewed_task_import, preview_task_import, read_task_import_bundle,
    read_task_import_receipt, recover_previewed_task_import, validate_task_import_preview,
    write_task_import_bundle, write_task_import_receipt,
};

use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Write as _};
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::{self, DeserializeSeed as _, Visitor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use weftext_core::{
    CommittedWorkspaceTransaction, NodeId, WorkspaceImportAuthority, WorkspaceImportNode,
    WorkspaceImportResource, WorkspaceRevision, commit_workspace_transaction, plan_import_tree,
    read_workspace_revision,
};
use weftext_import::{
    AsciiDocV1ProposalValidator, CancellationToken, CanonicalProposalValidator, CommitResult,
    ComponentVersion, DoclingLiteCapability, DoclingLitePdfAdapter, FakeAdapter, FakeWorker,
    FormatProbe, FormatWorker, IMPORT_PLAN_CONTRACT_VERSION, ImportAdapter, ImportDocument,
    ImportLimits, ImportPipeline, ImportPlan, ImportProposal, ImportReceipt, ImportTempRoot,
    IntakeRequest, LocalOcrPolicy, MarkdownCompatibilityAdapter, MarkdownCompatibilityWorker,
    OriginClass, PlanRequest, PortablePath, PreviewedImport, ResourcePolicy, Sha256Digest,
    SourceArtifact, ValidatedProposal, docling_lite_host_target, markdown_compatibility_descriptor,
    probe_source_bytes, replay_docling_pdf_probe, sha256_bytes,
    validate_docling_lite_preview_evidence, validate_import_authority,
    validate_markdown_compatibility_preview_evidence,
};

pub const INTAKE_BUNDLE_CONTRACT_VERSION: &str = "weftext.intake-preview-bundle.v1";
const MAX_BUNDLE_BYTES: u64 = 64 * 1024 * 1024;
pub const FAKE_SOURCE_BYTE_LIMIT: u64 = 1024 * 1024;
pub const PDF_SOURCE_BYTE_LIMIT: u64 = 64 * 1024 * 1024;
pub const MARKDOWN_SOURCE_BYTE_LIMIT: u64 = 16 * 1024 * 1024;

/// Immutable evidence reviewed before a later Core import commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ImportPreviewBundle {
    pub contract_version: String,
    pub bundle_digest: Sha256Digest,
    pub source_bytes: Vec<u8>,
    pub source: SourceArtifact,
    pub probe: FormatProbe,
    pub plan: ImportPlan,
    pub document: ImportDocument,
    pub proposal: ImportProposal,
    pub proposal_digest: Sha256Digest,
    pub components: Vec<ComponentVersion>,
    pub base_workspace_revision: WorkspaceRevision,
    pub preview_receipt: ImportReceipt,
}

impl ImportPreviewBundle {
    fn create(
        source_bytes: Vec<u8>,
        preview: PreviewedImport,
        base_workspace_revision: WorkspaceRevision,
        preview_receipt: ImportReceipt,
    ) -> Result<Self, IntakeError> {
        let proposal = preview.proposal.proposal().clone();
        let proposal_digest = preview.proposal.proposal_digest().clone();
        let mut bundle = Self {
            contract_version: INTAKE_BUNDLE_CONTRACT_VERSION.to_owned(),
            bundle_digest: sha256_bytes(b"pending"),
            source_bytes,
            source: preview.source,
            probe: preview.probe,
            plan: preview.plan,
            document: preview.document,
            proposal,
            proposal_digest,
            components: preview.components,
            base_workspace_revision,
            preview_receipt,
        };
        bundle.bundle_digest = bundle.compute_digest()?;
        Ok(bundle)
    }

    fn compute_digest(&self) -> Result<Sha256Digest, IntakeError> {
        let material = serde_json::to_vec(&(
            &self.contract_version,
            &self.source_bytes,
            &self.source,
            &self.probe,
            &self.plan,
            &self.document,
            &self.proposal,
            &self.proposal_digest,
            &self.components,
            &self.base_workspace_revision,
            &self.preview_receipt,
        ))
        .map_err(|error| serialization("serialize preview bundle authority", &error))?;
        Ok(sha256_bytes(&material))
    }
}

/// Exact result of consuming one reviewed preview bundle once.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommittedImport {
    pub proposal_id: String,
    pub proposal_digest: Sha256Digest,
    pub transaction: CommittedWorkspaceTransaction,
    pub receipt: ImportReceipt,
}

/// Runs the shared deterministic fake adapter through probe, plan, worker, IR, proposal, and
/// `PreviewOnly` receipt. The import pipeline receives no workspace handle.
///
/// # Errors
///
/// Returns a typed error for invalid source/destination, stale or invalid workspace authority,
/// temporary storage, worker, IR, proposal, or timestamp failure.
pub fn preview_fake_import(
    workspace: impl AsRef<Path>,
    temp_root: ImportTempRoot,
    display_name: impl Into<String>,
    origin: OriginClass,
    source_bytes: Vec<u8>,
    destination: PortablePath,
    created_at: impl Into<String>,
) -> Result<ImportPreviewBundle, IntakeError> {
    let workspace = workspace.as_ref();
    let base_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    let limits = fake_limits();
    let pipeline = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator));
    let preview = pipeline
        .preview(
            IntakeRequest {
                display_name: display_name.into(),
                origin,
                bytes: source_bytes.clone(),
                plan: PlanRequest::single_node(destination),
                limits,
                cancellation: CancellationToken::default(),
            },
            &FakeAdapter,
            Arc::new(FakeWorker::success()),
        )
        .map_err(|error| import_error(&error))?;
    let preview_receipt = preview
        .receipt(created_at, CommitResult::PreviewOnly)
        .map_err(|error| import_error(&error))?;
    ImportPreviewBundle::create(
        source_bytes,
        preview,
        base_workspace_revision,
        preview_receipt,
    )
}

/// Converts an explicitly selected inert Markdown file through the shared Import IR preview path.
/// Markdown is never treated as a managed workspace representation, and this stage cannot write
/// to the workspace.
///
/// # Errors
///
/// Returns a typed error for invalid UTF-8, active content, unsafe source evidence, bounded
/// conversion failures, or invalid workspace/destination authority.
#[allow(clippy::too_many_arguments)]
pub fn preview_markdown_import(
    workspace: impl AsRef<Path>,
    temp_root: ImportTempRoot,
    display_name: impl Into<String>,
    origin: OriginClass,
    source_bytes: Vec<u8>,
    destination: PortablePath,
    retain_original: bool,
    created_at: impl Into<String>,
    cancellation: CancellationToken,
) -> Result<ImportPreviewBundle, IntakeError> {
    let workspace = workspace.as_ref();
    let base_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    let mut request = PlanRequest::single_node(destination);
    request.resource_policy = if retain_original {
        ResourcePolicy::ExtractAndRetainOriginal
    } else {
        ResourcePolicy::SkipAll
    };
    request.local_ocr_policy = LocalOcrPolicy::Never;
    let pipeline = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator));
    let preview = pipeline
        .preview(
            IntakeRequest {
                display_name: display_name.into(),
                origin,
                bytes: source_bytes.clone(),
                plan: request,
                limits: markdown_limits(),
                cancellation,
            },
            &MarkdownCompatibilityAdapter,
            Arc::new(MarkdownCompatibilityWorker),
        )
        .map_err(|error| import_error(&error))?;
    let preview_receipt = preview
        .receipt(created_at, CommitResult::PreviewOnly)
        .map_err(|error| import_error(&error))?;
    ImportPreviewBundle::create(
        source_bytes,
        preview,
        base_workspace_revision,
        preview_receipt,
    )
}

/// Inspects the fixed Docling Lite installation directory and reports all
/// missing pin/isolation evidence without attempting a conversion or download.
#[must_use]
pub fn docling_lite_capability(installation_root: impl AsRef<Path>) -> DoclingLiteCapability {
    let Some(target) = docling_lite_host_target() else {
        return unavailable_docling_capability(
            "docling_lite_host_target_unreviewed",
            "this host target has no reviewed Docling Lite asset profile",
            vec!["a reviewed lock for the current host target".to_owned()],
        );
    };
    match DoclingLitePdfAdapter::from_installation_directory(installation_root, target) {
        Ok(adapter) => adapter.capability(),
        Err(error) => unavailable_docling_capability(
            "docling_lite_installation_not_verified",
            "the fixed Docling Lite installation did not pass complete byte/SHA/link validation",
            vec![error.to_string()],
        ),
    }
}

/// Runs the pinned PDF adapter through the same immutable preview bundle used
/// by fake acceptance and later Core commit. No fallback converter is selected.
///
/// # Errors
///
/// Returns `CapabilityUnavailable` while the fixed assets or mandatory host
/// sandbox cannot be proven; otherwise returns normal bounded pipeline errors.
#[allow(clippy::too_many_arguments)]
pub fn preview_docling_pdf_import(
    workspace: impl AsRef<Path>,
    temp_root: ImportTempRoot,
    installation_root: impl AsRef<Path>,
    display_name: impl Into<String>,
    origin: OriginClass,
    source_bytes: Vec<u8>,
    destination: PortablePath,
    created_at: impl Into<String>,
    cancellation: CancellationToken,
) -> Result<ImportPreviewBundle, IntakeError> {
    let workspace = workspace.as_ref();
    let target = docling_lite_host_target().ok_or_else(|| {
        IntakeError::new(
            IntakeErrorCode::CapabilityUnavailable,
            "this host target has no reviewed Docling Lite asset profile",
        )
    })?;
    let adapter = DoclingLitePdfAdapter::from_installation_directory(installation_root, target)
        .map_err(|error| capability_error(&error))?;
    let capability = adapter.capability();
    if !capability.available {
        return Err(IntakeError::new(
            IntakeErrorCode::CapabilityUnavailable,
            format!(
                "{}; missing: {}",
                capability.message,
                capability
                    .missing_pinned_evidence
                    .iter()
                    .chain(capability.missing_isolation_evidence.iter())
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }
    let worker = adapter
        .process_worker()
        .map_err(|error| capability_error(&error))?;
    let base_workspace_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    let pipeline = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator));
    let preview = pipeline
        .preview(
            IntakeRequest {
                display_name: display_name.into(),
                origin,
                bytes: source_bytes.clone(),
                plan: PlanRequest::single_node(destination),
                limits: docling_lite_limits(),
                cancellation,
            },
            &adapter,
            worker,
        )
        .map_err(|error| import_error(&error))?;
    let preview_receipt = preview
        .receipt(created_at, CommitResult::PreviewOnly)
        .map_err(|error| import_error(&error))?;
    ImportPreviewBundle::create(
        source_bytes,
        preview,
        base_workspace_revision,
        preview_receipt,
    )
}

/// Revalidates every byte and contract field in a preview bundle without running its worker.
///
/// # Errors
///
/// Returns a typed error for an altered, stale, foreign, over-limit, or nondeterministic bundle.
pub fn validate_fake_preview(
    bundle: &ImportPreviewBundle,
) -> Result<ValidatedProposal, IntakeError> {
    if bundle.plan.route.adapter != FakeAdapter.descriptor() {
        return invalid_bundle("import preview is not the reviewed fake acceptance route");
    }
    validate_preview_bundle(bundle)
}

/// Revalidates an immutable Markdown compatibility preview without re-running its parser.
///
/// # Errors
///
/// Returns a typed error when the bundle is not the reviewed Markdown route or any authority
/// field has changed.
pub fn validate_markdown_preview(
    bundle: &ImportPreviewBundle,
) -> Result<ValidatedProposal, IntakeError> {
    if bundle.plan.route.adapter != markdown_compatibility_descriptor() {
        return invalid_bundle("import preview is not the reviewed Markdown compatibility route");
    }
    validate_preview_bundle(bundle)
}

/// Revalidates an immutable fake or pinned Docling preview without re-running
/// its worker. This is the shared authority consumed by CLI and Desktop commit.
///
/// # Errors
///
/// Returns a typed error for an altered, stale, foreign, over-limit, or
/// internally inconsistent bundle.
pub fn validate_preview_bundle(
    bundle: &ImportPreviewBundle,
) -> Result<ValidatedProposal, IntakeError> {
    if bundle.contract_version != INTAKE_BUNDLE_CONTRACT_VERSION {
        return invalid_bundle("unsupported import preview bundle contract version");
    }
    if bundle.bundle_digest != bundle.compute_digest()? {
        return invalid_bundle("import preview bundle digest does not match its exact authority");
    }
    let limits = bundle.plan.limits.clone();
    limits.validate().map_err(|error| import_error(&error))?;
    let mut expected_source = SourceArtifact::from_bytes(
        bundle.source.display_name.clone(),
        bundle.source.origin,
        &bundle.source_bytes,
        &limits,
    )
    .map_err(|error| import_error(&error))?;
    expected_source.detected_format = bundle.probe.detected_format;
    expected_source
        .mismatch_evidence
        .clone_from(&bundle.probe.mismatch_evidence);
    if expected_source != bundle.source {
        return invalid_bundle("source artifact differs from its exact bundled bytes");
    }
    validate_import_authority(
        &bundle.source,
        &bundle.probe,
        &bundle.plan,
        &bundle.document,
    )
    .map_err(|error| import_error(&error))?;
    validate_adapter_preview_evidence(bundle, &limits)?;
    let validated = AsciiDocV1ProposalValidator
        .validate(
            &bundle.source,
            &bundle.source_bytes,
            &bundle.plan,
            &bundle.document,
            bundle.proposal.clone(),
        )
        .map_err(|error| import_error(&error))?;
    if validated.proposal_digest() != &bundle.proposal_digest {
        return invalid_bundle("proposal digest differs from deterministic proposal authority");
    }
    let preview_receipt = ImportReceipt::create(
        bundle.preview_receipt.created_at.clone(),
        &bundle.source,
        &bundle.plan,
        &bundle.document,
        &validated,
        bundle.components.clone(),
        CommitResult::PreviewOnly,
    )
    .map_err(|error| import_error(&error))?;
    if preview_receipt != bundle.preview_receipt {
        return invalid_bundle("PreviewOnly receipt differs from validated bundle authority");
    }
    Ok(validated)
}

fn validate_adapter_preview_evidence(
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<(), IntakeError> {
    if bundle.plan.route.adapter == FakeAdapter.descriptor() {
        validate_fake_adapter_evidence(bundle, limits)?;
    } else if bundle.plan.route.adapter == markdown_compatibility_descriptor() {
        validate_markdown_adapter_evidence(bundle, limits)?;
    } else if bundle.plan.route.adapter.supported_format == weftext_import::SourceFormat::Pdf {
        validate_pdf_adapter_evidence(bundle, limits)?;
    } else {
        return invalid_bundle("import preview uses an unknown, unreviewed adapter route");
    }
    if bundle.plan.route.adapter == FakeAdapter.descriptor() {
        let worker = FakeWorker::success();
        let expected_components = vec![ComponentVersion {
            component_id: worker.worker_id().to_owned(),
            version: worker.protocol_version().to_owned(),
            artifact_digest: None,
        }];
        if bundle.components != expected_components {
            return invalid_bundle("fake import component evidence is not the acceptance worker");
        }
    }
    Ok(())
}

fn validate_fake_adapter_evidence(
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<(), IntakeError> {
    if *limits != fake_limits() {
        return invalid_bundle("fake import preview does not use the bounded acceptance limits");
    }
    let (probe_source, expected_probe) = replay_adapter_probe(&FakeAdapter, bundle, limits)?;
    if expected_probe != bundle.probe || probe_source != bundle.source {
        return invalid_bundle("format probe differs from a fresh bounded fake signature probe");
    }
    validate_fake_plan(bundle, &expected_probe, limits)
}

fn validate_markdown_adapter_evidence(
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<(), IntakeError> {
    if *limits != markdown_limits() {
        return invalid_bundle(
            "Markdown import preview does not use the bounded compatibility limits",
        );
    }
    let (probe_source, expected_probe) =
        replay_adapter_probe(&MarkdownCompatibilityAdapter, bundle, limits)?;
    if expected_probe != bundle.probe || probe_source != bundle.source {
        return invalid_bundle(
            "format probe differs from a fresh bounded Markdown compatibility probe",
        );
    }
    validate_markdown_compatibility_preview_evidence(
        &bundle.probe,
        &bundle.plan,
        &bundle.components,
    )
    .map_err(|error| import_error(&error))
}

fn validate_pdf_adapter_evidence(
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<(), IntakeError> {
    if *limits != docling_lite_limits() {
        return invalid_bundle("PDF import preview does not use the reviewed Docling limits");
    }
    let mut probe_source = source_from_bundle_bytes(bundle, limits)?;
    let expected_probe = replay_docling_pdf_probe(
        &probe_source,
        &bundle.source_bytes,
        limits,
        bundle.plan.route.adapter.clone(),
    )
    .map_err(|error| import_error(&error))?;
    apply_probe_source_fields(&mut probe_source, &expected_probe);
    if expected_probe != bundle.probe || probe_source != bundle.source {
        return invalid_bundle(
            "PDF probe and detected source fields differ from a fresh exact-byte replay",
        );
    }
    validate_docling_lite_preview_evidence(&bundle.probe, &bundle.plan, &bundle.components)
        .map_err(|error| import_error(&error))
}

fn replay_adapter_probe(
    adapter: &dyn ImportAdapter,
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<(SourceArtifact, FormatProbe), IntakeError> {
    let mut source = source_from_bundle_bytes(bundle, limits)?;
    let probe = probe_source_bytes(adapter, &source, &bundle.source_bytes, limits)
        .map_err(|error| import_error(&error))?;
    apply_probe_source_fields(&mut source, &probe);
    Ok((source, probe))
}

fn source_from_bundle_bytes(
    bundle: &ImportPreviewBundle,
    limits: &ImportLimits,
) -> Result<SourceArtifact, IntakeError> {
    SourceArtifact::from_bytes(
        bundle.source.display_name.clone(),
        bundle.source.origin,
        &bundle.source_bytes,
        limits,
    )
    .map_err(|error| import_error(&error))
}

fn apply_probe_source_fields(source: &mut SourceArtifact, probe: &FormatProbe) {
    source.detected_format = probe.detected_format;
    source
        .mismatch_evidence
        .clone_from(&probe.mismatch_evidence);
}

/// Commits only the exact proposal contained in a reviewed bundle and creates the final receipt.
/// The adapter and worker are deliberately absent from this path.
///
/// # Errors
///
/// Returns a typed error for a stale/tampered bundle, proposal conflicts, Core validation,
/// transaction failure, or inconsistent receipt authority.
pub fn commit_previewed_import(
    workspace: impl AsRef<Path>,
    bundle: &ImportPreviewBundle,
    created_at: impl Into<String>,
) -> Result<CommittedImport, IntakeError> {
    let workspace = workspace.as_ref();
    let current_revision =
        read_workspace_revision(workspace).map_err(|error| workspace_error(&error))?;
    if current_revision != bundle.base_workspace_revision {
        return Err(IntakeError::new(
            IntakeErrorCode::StalePreview,
            format!(
                "stale import preview: expected workspace revision {}, found {current_revision}",
                bundle.base_workspace_revision
            ),
        ));
    }
    let validated = validate_preview_bundle(bundle)?;
    let proposal = validated.proposal();
    if !proposal.conflicts.is_empty() {
        return Err(IntakeError::new(
            IntakeErrorCode::ProposalConflict,
            "an import proposal with blocking conflicts cannot be committed",
        ));
    }
    let authority = WorkspaceImportAuthority {
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: validated.proposal_digest().to_string(),
    };
    let imported_nodes = proposal
        .nodes
        .iter()
        .map(|node| {
            let node_id = node.node_id.parse::<NodeId>().map_err(|error| {
                invalid_bundle_error(format!("invalid proposal node identity: {error}"))
            })?;
            Ok(WorkspaceImportNode {
                locator: node.locator.as_str().to_owned(),
                node_id,
                document_file: node.document_file.clone(),
                exact_source: node.exact_asciidoc.clone(),
                document_sha256: node.document_sha256.to_string(),
                resources: node
                    .resources
                    .iter()
                    .map(|resource| WorkspaceImportResource {
                        locator: resource.locator.as_str().to_owned(),
                        bytes: resource.bytes.clone(),
                        sha256: resource.sha256.to_string(),
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, IntakeError>>()?;
    let transaction = plan_import_tree(
        workspace,
        &bundle.base_workspace_revision,
        authority.clone(),
        imported_nodes,
    )
    .map_err(|error| workspace_error(&error))?;
    if transaction.import_authority.as_ref() != Some(&authority) {
        return invalid_bundle("Core transaction did not preserve exact import authority");
    }
    let transaction =
        commit_workspace_transaction(&transaction).map_err(|error| workspace_error(&error))?;
    if transaction.import_authority.as_ref() != Some(&authority) {
        return invalid_bundle("committed transaction lost exact import authority");
    }
    let receipt = ImportReceipt::create(
        created_at,
        &bundle.source,
        &bundle.plan,
        &bundle.document,
        &validated,
        bundle.components.clone(),
        CommitResult::Committed {
            transaction_id: transaction.plan_id.clone(),
            workspace_revision: transaction.revision.to_string(),
        },
    )
    .map_err(|error| import_error(&error))?;
    if receipt.proposal_digest != bundle.proposal_digest {
        return invalid_bundle("committed receipt lost exact proposal authority");
    }
    Ok(CommittedImport {
        proposal_id: proposal.proposal_id.clone(),
        proposal_digest: validated.proposal_digest().clone(),
        transaction,
        receipt,
    })
}

/// Reads and fully validates an exact preview bundle from a regular non-link file.
///
/// # Errors
///
/// Returns a typed error for unsafe file state, size, JSON, unknown fields, or invalid authority.
pub fn read_preview_bundle(path: impl AsRef<Path>) -> Result<ImportPreviewBundle, IntakeError> {
    let bytes = read_regular_file_bounded(path.as_ref(), MAX_BUNDLE_BYTES)?;
    reject_duplicate_json_keys(&bytes)
        .map_err(|error| serialization("parse import preview bundle JSON", &error))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| serialization("parse import preview bundle JSON", &error))?;
    let bundle: ImportPreviewBundle = serde_json::from_value(value.clone())
        .map_err(|error| serialization("decode import preview bundle contract", &error))?;
    let exact_contract = serde_json::to_value(&bundle)
        .map_err(|error| serialization("normalize import preview bundle contract", &error))?;
    if value != exact_contract {
        return invalid_bundle("import preview bundle contains fields outside its exact contract");
    }
    validate_preview_bundle(&bundle)?;
    Ok(bundle)
}

/// Writes a preview bundle once, atomically, outside the workspace being imported into.
///
/// # Errors
///
/// Returns a typed error for path escape, overwrite, size, serialization, write, or sync failure.
pub fn write_preview_bundle(
    workspace: impl AsRef<Path>,
    path: impl AsRef<Path>,
    bundle: &ImportPreviewBundle,
) -> Result<(), IntakeError> {
    let workspace = workspace.as_ref();
    let path = path.as_ref();
    ensure_bundle_outside_workspace(workspace, path)?;
    let bytes = serde_json::to_vec_pretty(bundle)
        .map_err(|error| serialization("serialize import preview bundle", &error))?;
    write_bundle_bytes(workspace, path, &bytes)
}

pub(crate) fn write_bundle_bytes(
    workspace: &Path,
    path: &Path,
    bytes: &[u8],
) -> Result<(), IntakeError> {
    ensure_bundle_outside_workspace(workspace, path)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_BUNDLE_BYTES {
        return invalid_bundle("serialized import preview bundle exceeds the byte limit");
    }
    if path.exists() {
        return Err(IntakeError::new(
            IntakeErrorCode::Io,
            "import preview bundle destination already exists",
        ));
    }
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| IntakeError::new(IntakeErrorCode::Io, "bundle filename is not UTF-8"))?;
    let temporary = parent.join(format!(".{name}.weftext-{}.tmp", uuid::Uuid::new_v4()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| io_error("create temporary preview bundle", &error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("write temporary preview bundle", &error))?;
        file.sync_all()
            .map_err(|error| io_error("sync temporary preview bundle", &error))?;
        fs::hard_link(&temporary, path)
            .map_err(|error| io_error("publish preview bundle without overwrite", &error))?;
        sync_directory(parent)
            .map_err(|error| io_error("sync published preview bundle directory", &error))?;
        if fs::remove_file(&temporary).is_ok() {
            sync_directory(parent)
                .map_err(|error| io_error("sync preview bundle cleanup directory", &error))?;
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> std::io::Result<()> {
    use std::os::windows::fs::OpenOptionsExt as _;

    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?
        .sync_all()
}

#[derive(Clone, Copy)]
struct DuplicateKeyDetector;

impl<'de> de::DeserializeSeed<'de> for DuplicateKeyDetector {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for DuplicateKeyDetector {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut keys = BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate JSON object key: {key}"
                )));
            }
            map.next_value_seed(Self)?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        while sequence.next_element_seed(Self)?.is_some() {}
        Ok(())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(())
    }
}

pub(crate) fn reject_duplicate_json_keys(bytes: &[u8]) -> Result<(), serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    DuplicateKeyDetector.deserialize(&mut deserializer)?;
    deserializer.end()
}

/// Reads one regular non-link file under a strict byte ceiling.
///
/// # Errors
///
/// Returns a typed error for links, non-files, over-limit input, races, or I/O failure.
pub fn read_regular_file_bounded(
    path: impl AsRef<Path>,
    maximum_bytes: u64,
) -> Result<Vec<u8>, IntakeError> {
    let path = path.as_ref();
    reject_linked_existing_ancestors(path)?;
    let file = open_regular_file_nofollow(path)?;
    let metadata = file
        .metadata()
        .map_err(|error| io_error(&format!("inspect bounded input {}", path.display()), &error))?;
    if !metadata.is_file() || linked_or_reparse(&metadata) {
        return Err(IntakeError::new(
            IntakeErrorCode::Io,
            format!(
                "bounded input must be one regular non-link file: {}",
                path.display()
            ),
        ));
    }
    if metadata.len() > maximum_bytes {
        return Err(IntakeError::new(
            IntakeErrorCode::LimitExceeded,
            format!(
                "bounded input exceeds {maximum_bytes} bytes: {}",
                path.display()
            ),
        ));
    }
    let mut bytes = Vec::new();
    file.take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(&format!("read bounded input {}", path.display()), &error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > maximum_bytes {
        return Err(IntakeError::new(
            IntakeErrorCode::LimitExceeded,
            format!(
                "bounded input grew beyond {maximum_bytes} bytes: {}",
                path.display()
            ),
        ));
    }
    reject_linked_existing_ancestors(path)?;
    Ok(bytes)
}

fn open_regular_file_nofollow(path: &Path) -> Result<File, IntakeError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options.open(path).map_err(|error| {
        io_error(
            &format!("open bounded non-link input {}", path.display()),
            &error,
        )
    })
}

/// Produces a UTC RFC 3339 timestamp without a locale or timezone dependency.
///
/// # Errors
///
/// Returns a typed error when the system clock is outside the supported four-digit range.
pub fn rfc3339_utc_now() -> Result<String, IntakeError> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| IntakeError::new(IntakeErrorCode::Time, "system clock precedes Unix epoch"))?
        .as_secs();
    let days = i64::try_from(seconds / 86_400).map_err(|_| {
        IntakeError::new(
            IntakeErrorCode::Time,
            "system clock exceeds the supported timestamp range",
        )
    })?;
    let second_of_day = seconds % 86_400;
    let (year, month, day) = civil_date_from_unix_days(days);
    if !(0..=9_999).contains(&year) {
        return Err(IntakeError::new(
            IntakeErrorCode::Time,
            "system clock exceeds the four-digit RFC 3339 year range",
        ));
    }
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3_600,
        (second_of_day % 3_600) / 60,
        second_of_day % 60
    ))
}

fn validate_fake_plan(
    bundle: &ImportPreviewBundle,
    expected_probe: &FormatProbe,
    limits: &ImportLimits,
) -> Result<(), IntakeError> {
    let plan = &bundle.plan;
    if plan.contract_version != IMPORT_PLAN_CONTRACT_VERSION {
        return invalid_bundle("unsupported import plan contract version");
    }
    if plan.source_digest != bundle.source.sha256 {
        return invalid_bundle("import plan source digest differs from source artifact");
    }
    let probe_bytes = serde_json::to_vec(expected_probe)
        .map_err(|error| serialization("serialize bounded probe", &error))?;
    if plan.probe_digest != sha256_bytes(&probe_bytes) {
        return invalid_bundle("import plan probe digest is stale or forged");
    }
    let worker = FakeWorker::success();
    if plan.route.adapter != FakeAdapter.descriptor()
        || plan.route.worker_id != worker.worker_id()
        || plan.route.worker_protocol_version != worker.protocol_version()
    {
        return invalid_bundle("import plan route is not the reviewed fake acceptance route");
    }
    let mut request = PlanRequest::single_node(plan.destination.clone());
    request.agent_enhancement = plan.agent_enhancement.clone();
    request.egress = plan.egress.clone();
    if plan.split_policy != request.split_policy
        || plan.resource_policy != request.resource_policy
        || plan.local_ocr_policy != request.local_ocr_policy
    {
        return invalid_bundle("import plan policy differs from fake acceptance policy");
    }
    let planned_node_id = plan
        .proposed_root_id
        .parse::<NodeId>()
        .map_err(|error| invalid_bundle_error(format!("invalid planned node identity: {error}")))?;
    if planned_node_id.to_string() != plan.proposed_root_id {
        return invalid_bundle("planned import node identity is not canonical UUIDv4");
    }
    let material = serde_json::to_vec(&(
        &plan.source_digest,
        &plan.probe_digest,
        &plan.proposed_root_id,
        &plan.route,
        &request,
        limits,
    ))
    .map_err(|error| serialization("serialize frozen import plan", &error))?;
    let expected_plan_id = format!("plan-{}", &sha256_bytes(&material).as_str()[..24]);
    if plan.plan_id != expected_plan_id {
        return invalid_bundle("import plan ID differs from its frozen plan material");
    }
    Ok(())
}

fn fake_limits() -> ImportLimits {
    ImportLimits {
        max_source_bytes: FAKE_SOURCE_BYTE_LIMIT,
        max_text_bytes: 8 * 1024 * 1024,
        max_resource_bytes: 8 * 1024 * 1024,
        max_total_output_bytes: 16 * 1024 * 1024,
        worker_memory_bytes: 256 * 1024 * 1024,
        ..ImportLimits::default()
    }
}

fn markdown_limits() -> ImportLimits {
    ImportLimits {
        max_source_bytes: MARKDOWN_SOURCE_BYTE_LIMIT,
        max_text_bytes: 16 * 1024 * 1024,
        max_resource_bytes: MARKDOWN_SOURCE_BYTE_LIMIT,
        max_total_output_bytes: 64 * 1024 * 1024,
        worker_memory_bytes: 256 * 1024 * 1024,
        worker_timeout_ms: 30 * 1_000,
        cancellation_grace_ms: 1_000,
        ..ImportLimits::default()
    }
}

fn docling_lite_limits() -> ImportLimits {
    ImportLimits {
        max_source_bytes: PDF_SOURCE_BYTE_LIMIT,
        max_pages: 500,
        max_ir_nodes: 50_000,
        max_text_bytes: 32 * 1024 * 1024,
        max_resource_count: 2_000,
        max_resource_bytes: 32 * 1024 * 1024,
        max_total_output_bytes: 256 * 1024 * 1024,
        worker_memory_bytes: 2 * 1024 * 1024 * 1024,
        worker_timeout_ms: 5 * 60 * 1_000,
        cancellation_grace_ms: 2_000,
        ..ImportLimits::default()
    }
}

fn unavailable_docling_capability(
    code: &str,
    message: &str,
    missing_pinned_evidence: Vec<String>,
) -> DoclingLiteCapability {
    DoclingLiteCapability {
        available: false,
        code: code.to_owned(),
        message: message.to_owned(),
        missing_pinned_evidence,
        missing_isolation_evidence: vec![
            "a proven deny-by-default network sandbox".to_owned(),
            "a proven per-process memory sandbox".to_owned(),
            "a proven filesystem/process-tree sandbox for the worker".to_owned(),
        ],
        ambient_network_allowed: false,
    }
}

fn ensure_bundle_outside_workspace(workspace: &Path, bundle: &Path) -> Result<(), IntakeError> {
    reject_linked_existing_ancestors(workspace)?;
    reject_linked_existing_ancestors(bundle)?;
    let canonical_workspace =
        fs::canonicalize(workspace).map_err(|error| io_error("resolve workspace root", &error))?;
    let parent = bundle
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let canonical_parent =
        fs::canonicalize(parent).map_err(|error| io_error("resolve bundle parent", &error))?;
    reject_linked_existing_ancestors(&canonical_parent)?;
    if canonical_parent.starts_with(canonical_workspace) {
        return Err(IntakeError::new(
            IntakeErrorCode::InvalidBundle,
            "preview bundle must be written outside the workspace",
        ));
    }
    Ok(())
}

fn reject_linked_existing_ancestors(path: &Path) -> Result<(), IntakeError> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| io_error("resolve current directory for path safety", &error))?
            .join(path)
    };
    for ancestor in absolute.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(metadata) if linked_or_reparse(&metadata) => {
                return Err(IntakeError::new(
                    IntakeErrorCode::Io,
                    format!(
                        "intake path crosses a link or reparse point: {}",
                        ancestor.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("inspect intake path ancestry", &error)),
        }
    }
    Ok(())
}

fn civil_date_from_unix_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let provisional_year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = provisional_year + i64::from(month <= 2);
    (year, month, day)
}

fn linked_or_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntakeErrorCode {
    Import,
    CapabilityUnavailable,
    Workspace,
    InvalidBundle,
    StalePreview,
    ProposalConflict,
    LimitExceeded,
    Io,
    Serialization,
    Time,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IntakeError {
    pub code: IntakeErrorCode,
    pub message: String,
}

impl IntakeError {
    #[must_use]
    pub fn new(code: IntakeErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for IntakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for IntakeError {}

fn invalid_bundle<T>(message: impl Into<String>) -> Result<T, IntakeError> {
    Err(invalid_bundle_error(message))
}

fn invalid_bundle_error(message: impl Into<String>) -> IntakeError {
    IntakeError::new(IntakeErrorCode::InvalidBundle, message)
}

fn import_error(error: &impl fmt::Display) -> IntakeError {
    IntakeError::new(IntakeErrorCode::Import, error.to_string())
}

fn capability_error(error: &impl fmt::Display) -> IntakeError {
    IntakeError::new(IntakeErrorCode::CapabilityUnavailable, error.to_string())
}

fn workspace_error(error: &impl fmt::Display) -> IntakeError {
    IntakeError::new(IntakeErrorCode::Workspace, error.to_string())
}

fn io_error(operation: &str, error: &std::io::Error) -> IntakeError {
    IntakeError::new(IntakeErrorCode::Io, format!("{operation}: {error}"))
}

fn serialization(operation: &str, error: &serde_json::Error) -> IntakeError {
    IntakeError::new(
        IntakeErrorCode::Serialization,
        format!("{operation}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftext_core::{create_workspace, scan_workspace};
    use weftext_import::{
        AdapterDescriptor, AdapterRoute, AgentImportPatch, AgentPatchOperation, Confidence,
        FormatWorker, ImportError, ImportNode, ImportNodeKind, ImportSourceLocation, ProbeReader,
        SourceFormat, WORKER_REQUEST_CONTRACT_VERSION, WORKER_RESPONSE_CONTRACT_VERSION,
        WorkerContext, WorkerNetworkPolicy, WorkerRequest, WorkerResponse,
        derive_docling_pdf_probe,
    };

    const PDF_FIXTURE_WORKER: &str = "weftext.docling-lite-worker";
    const PDF_FIXTURE_PROTOCOL: &str = "weftext.docling-lite-worker-json.v1";

    #[derive(Clone, Copy)]
    struct PdfFixtureAdapter;

    impl ImportAdapter for PdfFixtureAdapter {
        fn descriptor(&self) -> AdapterDescriptor {
            AdapterDescriptor {
                adapter_id: "weftext.pdf-docling-lite-adapter".to_owned(),
                adapter_version: "0.52.2-lock-0123456789abcdef".to_owned(),
                supported_format: SourceFormat::Pdf,
            }
        }

        fn probe(
            &self,
            source: &SourceArtifact,
            evidence: &mut ProbeReader<'_>,
            limits: &ImportLimits,
        ) -> Result<FormatProbe, ImportError> {
            derive_docling_pdf_probe(
                source,
                evidence,
                limits,
                self.descriptor(),
                true,
                "test capability",
            )
        }

        fn plan(
            &self,
            source: &SourceArtifact,
            probe: &FormatProbe,
            request: PlanRequest,
            limits: ImportLimits,
        ) -> Result<ImportPlan, ImportError> {
            ImportPlan::create(
                source,
                probe,
                AdapterRoute {
                    adapter: self.descriptor(),
                    worker_id: PDF_FIXTURE_WORKER.to_owned(),
                    worker_protocol_version: PDF_FIXTURE_PROTOCOL.to_owned(),
                },
                request,
                limits,
            )
        }

        fn worker_request(
            &self,
            source: &SourceArtifact,
            plan: &ImportPlan,
            source_locator: PortablePath,
        ) -> Result<WorkerRequest, ImportError> {
            Ok(WorkerRequest {
                contract_version: WORKER_REQUEST_CONTRACT_VERSION.to_owned(),
                request_id: format!("request-{}", &plan.plan_id[5..]),
                worker_id: PDF_FIXTURE_WORKER.to_owned(),
                worker_protocol_version: PDF_FIXTURE_PROTOCOL.to_owned(),
                source: source.clone(),
                source_locator,
                plan: plan.clone(),
                network: WorkerNetworkPolicy::Denied,
                memory_limit_bytes: plan.limits.worker_memory_bytes,
                page_limit: plan.limits.max_pages,
                entry_limit: plan.limits.max_container_entries,
                output_byte_limit: plan.limits.max_total_output_bytes,
                format_options: serde_json::json!({"fixture": true}),
            })
        }

        fn map_worker_response(
            &self,
            source: &SourceArtifact,
            _plan: &ImportPlan,
            _response: WorkerResponse,
        ) -> Result<ImportDocument, ImportError> {
            ImportDocument::create(
                format!("document-{}", &source.sha256.as_str()[..24]),
                source.sha256.clone(),
                "PDF fixture",
                vec![ImportNode {
                    id: "paragraph-1".to_owned(),
                    kind: ImportNodeKind::Paragraph {
                        text: "locally extracted text".to_owned(),
                    },
                    confidence: Confidence::from_basis_points(9_000)?,
                    source_locations: vec![ImportSourceLocation {
                        source_digest: source.sha256.clone(),
                        page: Some(1),
                        region: None,
                        byte_start: None,
                        byte_end: None,
                    }],
                    provenance: Vec::new(),
                }],
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )
        }
    }

    struct PdfFixtureWorker;

    fn pdf_fixture_components() -> Vec<ComponentVersion> {
        [
            "docling-rs",
            "pdfium",
            "onnx-runtime",
            "layout-int8",
            "pp-ocr",
            "ocr-dictionary",
        ]
        .into_iter()
        .map(|component| ComponentVersion {
            component_id: component.to_owned(),
            version: "fixture-1".to_owned(),
            artifact_digest: Some(sha256_bytes(component.as_bytes())),
        })
        .collect()
    }

    impl FormatWorker for PdfFixtureWorker {
        fn worker_id(&self) -> &str {
            PDF_FIXTURE_WORKER
        }

        fn protocol_version(&self) -> &str {
            PDF_FIXTURE_PROTOCOL
        }

        fn execute(
            &self,
            request: WorkerRequest,
            _context: WorkerContext,
        ) -> Result<WorkerResponse, ImportError> {
            Ok(WorkerResponse {
                contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
                request_id: request.request_id,
                worker_id: PDF_FIXTURE_WORKER.to_owned(),
                worker_protocol_version: PDF_FIXTURE_PROTOCOL.to_owned(),
                source_digest: request.source.sha256,
                payload: serde_json::json!({"fixture": true}),
                resources: Vec::new(),
                diagnostics: Vec::new(),
                components: pdf_fixture_components(),
            })
        }
    }

    fn classic_pdf(catalog_extra: &str, trailer_extra: &str) -> Vec<u8> {
        let mut bytes =
            format!("%PDF-1.7\n1 0 obj\n<< /Type /Catalog {catalog_extra} >>\nendobj\n")
                .into_bytes();
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!(
                "xref\n0 2\n0000000000 65535 f \n0000000009 00000 n \ntrailer\n<< /Size 2 /Root 1 0 R {trailer_extra} >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        bytes
    }

    fn preview_pdf_fixture(
        workspace: &Path,
        temporary: &Path,
        bytes: Vec<u8>,
    ) -> ImportPreviewBundle {
        let base_workspace_revision = read_workspace_revision(workspace).expect("revision");
        let temp_root =
            ImportTempRoot::initialize(temporary.join("pdf-intake-temp")).expect("temp root");
        let preview = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator))
            .preview(
                IntakeRequest {
                    display_name: "fixture.pdf".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes: bytes.clone(),
                    plan: PlanRequest::single_node(PortablePath::parse("Imported").unwrap()),
                    limits: docling_lite_limits(),
                    cancellation: CancellationToken::default(),
                },
                &PdfFixtureAdapter,
                Arc::new(PdfFixtureWorker),
            )
            .expect("PDF fixture preview");
        let receipt = preview
            .receipt("2026-08-24T00:00:00Z", CommitResult::PreviewOnly)
            .expect("receipt");
        ImportPreviewBundle::create(bytes, preview, base_workspace_revision, receipt)
            .expect("bundle")
    }

    fn forged_safe_pdf_bundle(workspace: &Path, source_bytes: Vec<u8>) -> ImportPreviewBundle {
        let limits = docling_lite_limits();
        let mut source = SourceArtifact::from_bytes(
            "forged.pdf",
            OriginClass::TestFixture,
            &source_bytes,
            &limits,
        )
        .expect("source");
        let mut probe = replay_docling_pdf_probe(
            &source,
            &source_bytes,
            &limits,
            PdfFixtureAdapter.descriptor(),
        )
        .expect("actual PDF probe");
        assert!(
            probe.active_content_detected
                || probe.encryption != weftext_import::EncryptionState::NotEncrypted
        );
        probe.active_content_detected = false;
        probe.encryption = weftext_import::EncryptionState::NotEncrypted;
        probe.safe_to_plan = true;
        probe.diagnostics.clear();
        source.detected_format = probe.detected_format;
        source
            .mismatch_evidence
            .clone_from(&probe.mismatch_evidence);
        let plan = PdfFixtureAdapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Forged").unwrap()),
                limits,
            )
            .expect("internally self-consistent forged plan");
        let document = ImportDocument::create(
            format!("document-{}", &source.sha256.as_str()[..24]),
            source.sha256.clone(),
            "Forged PDF",
            vec![ImportNode {
                id: "paragraph-1".to_owned(),
                kind: ImportNodeKind::Paragraph {
                    text: "locally extracted text".to_owned(),
                },
                confidence: Confidence::from_basis_points(9_000).unwrap(),
                source_locations: vec![ImportSourceLocation {
                    source_digest: source.sha256.clone(),
                    page: Some(1),
                    region: None,
                    byte_start: None,
                    byte_end: None,
                }],
                provenance: Vec::new(),
            }],
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("forged IR");
        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &source_bytes, &plan, &document)
            .expect("forged proposal");
        let preview = PreviewedImport {
            source,
            probe,
            plan,
            document,
            proposal,
            components: pdf_fixture_components(),
        };
        let receipt = preview
            .receipt("2026-08-24T00:00:00Z", CommitResult::PreviewOnly)
            .expect("forged receipt");
        ImportPreviewBundle::create(
            source_bytes,
            preview,
            read_workspace_revision(workspace).expect("revision"),
            receipt,
        )
        .expect("jointly forged bundle and digest")
    }

    #[test]
    fn cli_and_desktop_can_share_one_preview_without_rerunning_worker() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let temp_root =
            ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("intake temp");
        let bytes = b"WEFTEXT-FAKE/1\nImported\nshared path\n".to_vec();
        let preview = preview_fake_import(
            &workspace,
            temp_root,
            "shared.fake",
            OriginClass::LocalFile,
            bytes,
            PortablePath::parse("Imported").unwrap(),
            "2026-08-24T00:00:00Z",
        )
        .expect("preview");
        validate_fake_preview(&preview).expect("validate preview");
        assert!(!workspace.join("Imported").exists());

        let committed = commit_previewed_import(&workspace, &preview, "2026-08-24T00:00:01Z")
            .expect("commit exact preview");
        assert_eq!(committed.proposal_digest, preview.proposal_digest);
        assert!(workspace.join("Imported/Imported.adoc").is_file());
        assert!(scan_workspace(&workspace).is_valid());
        assert!(matches!(
            commit_previewed_import(&workspace, &preview, "2026-08-24T00:00:02Z"),
            Err(IntakeError {
                code: IntakeErrorCode::StalePreview,
                ..
            })
        ));
    }

    #[test]
    fn pdf_bundle_replay_rejects_joint_source_probe_plan_and_digest_forgery() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");

        let valid = preview_pdf_fixture(&workspace, temporary.path(), classic_pdf("", ""));
        validate_preview_bundle(&valid).expect("valid exact-byte PDF preview");

        for (label, bytes) in [
            ("active", classic_pdf("/OpenAction 9 0 R", "")),
            ("encrypted", classic_pdf("", "/Encrypt 9 0 R")),
        ] {
            let forged = forged_safe_pdf_bundle(&workspace, bytes);
            assert_eq!(forged.bundle_digest, forged.compute_digest().unwrap());
            let error = validate_preview_bundle(&forged).expect_err(label);
            assert_eq!(error.code, IntakeErrorCode::InvalidBundle, "{label}");
            let commit = commit_previewed_import(&workspace, &forged, "2026-08-24T00:00:01Z")
                .expect_err("commit must replay source-derived probe");
            assert_eq!(commit.code, IntakeErrorCode::InvalidBundle, "{label}");
            assert!(!workspace.join("Forged").exists());
        }
    }

    #[test]
    fn pdf_agent_prepare_apply_and_final_validation_replay_source_probe_authority() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let local = preview_pdf_fixture(&workspace, temporary.path(), classic_pdf("", ""));
        let selection = AgentEvidenceSelection {
            provider: "reviewed-provider".to_owned(),
            selected_node_ids: vec!["paragraph-1".to_owned()],
            retention: "delete-after-call".to_owned(),
            redaction: "none".to_owned(),
        };
        let approved = prepare_agent_enhancement(&local, selection.clone(), "2026-08-24T00:00:01Z")
            .expect("PDF selected-evidence preview");
        let patch = AgentImportPatch::create(
            local.document.revision.clone(),
            selection.selected_node_ids.clone(),
            vec![AgentPatchOperation::CorrectText {
                node_id: "paragraph-1".to_owned(),
                expected_text_digest: sha256_bytes(b"locally extracted text"),
                replacement: "reviewed correction".to_owned(),
            }],
            selection.provider,
            "reviewed-model",
            approved.authorized_bundle.plan.egress.clone(),
        )
        .expect("typed patch");
        let enhanced = apply_approved_agent_patch(&approved, &patch, "2026-08-24T00:00:02Z")
            .expect("enhanced PDF bundle");
        validate_preview_bundle(&enhanced).expect("enhanced source/probe replay");

        let forged = forged_safe_pdf_bundle(&workspace, classic_pdf("/OpenAction 9 0 R", ""));
        let error = prepare_agent_enhancement(
            &forged,
            AgentEvidenceSelection {
                provider: "reviewed-provider".to_owned(),
                selected_node_ids: vec!["paragraph-1".to_owned()],
                retention: "delete-after-call".to_owned(),
                redaction: "none".to_owned(),
            },
            "2026-08-24T00:00:03Z",
        )
        .expect_err("agent prepare must replay malicious source bytes");
        assert_eq!(error.code, IntakeErrorCode::InvalidBundle);
    }

    #[test]
    fn serialized_bundle_is_exact_and_unknown_fields_fail_closed() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let temp_root =
            ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("intake temp");
        let preview = preview_fake_import(
            &workspace,
            temp_root,
            "shared.fake",
            OriginClass::LocalFile,
            b"WEFTEXT-FAKE/1\nImported\nexact\n".to_vec(),
            PortablePath::parse("Imported").unwrap(),
            "2026-08-24T00:00:00Z",
        )
        .expect("preview");
        let bundle_path = temporary.path().join("preview.json");
        write_preview_bundle(&workspace, &bundle_path, &preview).expect("write bundle");
        assert_eq!(read_preview_bundle(&bundle_path).unwrap(), preview);
        assert!(matches!(
            write_preview_bundle(&workspace, &bundle_path, &preview),
            Err(IntakeError {
                code: IntakeErrorCode::Io,
                ..
            })
        ));

        let mut value: Value = serde_json::from_slice(&fs::read(&bundle_path).unwrap()).unwrap();
        value["unknown"] = Value::Bool(true);
        fs::write(&bundle_path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(
            read_preview_bundle(&bundle_path),
            Err(IntakeError {
                code: IntakeErrorCode::Serialization,
                ..
            })
        ));
    }

    #[test]
    fn pdf_capability_and_preview_fail_closed_without_fixed_assets_or_host_sandbox() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let workspace = temporary.path().join("Workspace");
        create_workspace(&workspace).expect("workspace");
        let installation = temporary.path().join("docling-lite");
        std::fs::create_dir(&installation).expect("installation directory");
        let capability = docling_lite_capability(&installation);
        assert!(!capability.available);
        assert!(!capability.ambient_network_allowed);

        let temp_root =
            ImportTempRoot::initialize(temporary.path().join("intake-temp")).expect("temp root");
        let error = preview_docling_pdf_import(
            &workspace,
            temp_root,
            &installation,
            "fixture.pdf",
            OriginClass::TestFixture,
            b"%PDF-1.7\n".to_vec(),
            PortablePath::parse("Imported").expect("destination"),
            "2026-08-24T00:00:00Z",
            CancellationToken::default(),
        )
        .expect_err("unavailable PDF adapter");
        assert_eq!(error.code, IntakeErrorCode::CapabilityUnavailable);
        assert!(!workspace.join("Imported").exists());
    }

    #[cfg(unix)]
    #[test]
    fn bounded_reader_never_follows_a_final_symlink() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary root");
        let source = temporary.path().join("source.fake");
        let link = temporary.path().join("link.fake");
        fs::write(&source, b"WEFTEXT-FAKE/1\nImported\n").expect("source");
        symlink(&source, &link).expect("symlink");
        assert!(matches!(
            read_regular_file_bounded(&link, FAKE_SOURCE_BYTE_LIMIT),
            Err(IntakeError {
                code: IntakeErrorCode::Io,
                ..
            })
        ));
    }
}
