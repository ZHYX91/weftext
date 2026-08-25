//! Weftext-owned, workspace-write-free content intake contracts.
//!
//! This crate stops at a validated [`ImportProposal`]. Committing that exact
//! proposal is deliberately a separate Core transaction concern.

#![forbid(unsafe_code)]

mod agent_patch;
mod contract;
mod digest;
mod docling_lite;
mod docling_process;
mod error;
mod fake;
mod limits;
mod markdown;
mod path;
mod pdf_probe;
mod pipeline;
mod probe;
mod proposal;
mod temp;

pub use agent_patch::{AgentImportPatch, AgentPatchOperation, apply_agent_patch};
pub use contract::{
    AdapterDescriptor, AdapterRoute, AgentEnhancementPolicy, AgentEnhancementSelection,
    BoundingRegion, CommitResult, ComponentVersion, Confidence, DiagnosticSeverity,
    EgressDisclosure, EncryptionState, FormatProbe, FormatWorker, ImportAdapter, ImportDiagnostic,
    ImportDocument, ImportNode, ImportNodeKind, ImportPlan, ImportProposal, ImportReceipt,
    ImportResource, ImportSourceLocation, LocalOcrPolicy, OriginClass, PlanRequest, ProposedNode,
    ProposedResource, ProvenanceKind, ProvenanceRecord, ResourcePolicy, SourceArtifact,
    SourceFormat, SplitPolicy, ValidatedProposal, WorkerContext, WorkerNetworkPolicy,
    WorkerRequest, WorkerResource, WorkerResponse, validate_import_authority,
};
pub use digest::{Sha256Digest, sha256_bytes};
pub use docling_lite::{
    DOCLING_DOCUMENT_SCHEMA_NAME, DOCLING_DOCUMENT_SCHEMA_VERSION, DOCLING_LITE_ASSET_LOCK_VERSION,
    DOCLING_LITE_INSTALLATION_LOCK_FILE, DOCLING_LITE_MAPPING_CONTRACT_VERSION,
    DOCLING_LITE_WORKER_PROTOCOL_VERSION, DOCLING_RELEASE_COMMIT, DOCLING_RELEASE_TAG,
    DoclingLiteArtifactRole, DoclingLiteAssetLock, DoclingLiteAssetPin, DoclingLiteCapability,
    DoclingLiteDistributionPin, DoclingLiteInstalledArtifact, DoclingLiteMappingContract,
    DoclingLitePdfAdapter, DoclingLiteWorkerCommand, DoclingLiteWorkerResponse,
    DoclingLiteWorkerStatus, DoclingModelPin, docling_lite_host_target,
    validate_docling_lite_preview_evidence,
};
pub use docling_process::DoclingLiteProcessWorker;
pub use error::{ImportError, ImportErrorCode};
pub use fake::{FakeAdapter, FakeWorker, FakeWorkerMode};
pub use limits::ImportLimits;
pub use markdown::{
    MarkdownCompatibilityAdapter, MarkdownCompatibilityWorker, markdown_compatibility_descriptor,
    validate_markdown_compatibility_preview_evidence,
};
pub use path::PortablePath;
pub use pdf_probe::{derive_docling_pdf_probe, replay_docling_pdf_probe};
pub use pipeline::{CancellationToken, ImportPipeline, IntakeRequest, PreviewedImport};
pub use probe::{
    PROBE_EVIDENCE_CONTRACT_VERSION, ProbeEvidence, ProbeEvidenceSegment, ProbeReader,
    probe_source_bytes,
};
pub use proposal::{AsciiDocV1ProposalValidator, CanonicalProposalValidator};
pub use temp::{CleanupReport, ImportTempRoot, TempSession};

pub const SOURCE_ARTIFACT_CONTRACT_VERSION: &str = "weftext.import.source-artifact.v1";
pub const FORMAT_PROBE_CONTRACT_VERSION: &str = "weftext.import.format-probe.v1";
pub const IMPORT_PLAN_CONTRACT_VERSION: &str = "weftext.import.plan.v1";
pub const IMPORT_IR_CONTRACT_VERSION: &str = "weftext.import-ir.v1";
pub const IMPORT_PROPOSAL_CONTRACT_VERSION: &str = "weftext.import.proposal.v1";
pub const IMPORT_RECEIPT_CONTRACT_VERSION: &str = "weftext.import.receipt.v1";
pub const AGENT_PATCH_CONTRACT_VERSION: &str = "weftext.import-agent-patch.v1";
pub const WORKER_REQUEST_CONTRACT_VERSION: &str = "weftext.import-worker-request.v1";
pub const WORKER_RESPONSE_CONTRACT_VERSION: &str = "weftext.import-worker-response.v1";
