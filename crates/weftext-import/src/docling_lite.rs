use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::{
    AdapterDescriptor, AdapterRoute, AgentEnhancementPolicy, BoundingRegion, ComponentVersion,
    Confidence, DiagnosticSeverity, EgressDisclosure, EncryptionState, FormatProbe,
    IMPORT_PLAN_CONTRACT_VERSION, ImportAdapter, ImportDiagnostic, ImportDocument, ImportError,
    ImportErrorCode, ImportLimits, ImportNode, ImportNodeKind, ImportPlan, ImportResource,
    ImportSourceLocation, LocalOcrPolicy, PlanRequest, PortablePath, ProbeReader, ProvenanceKind,
    ProvenanceRecord, ResourcePolicy, Sha256Digest, SourceArtifact, SourceFormat, SplitPolicy,
    WORKER_REQUEST_CONTRACT_VERSION, WORKER_RESPONSE_CONTRACT_VERSION, WorkerNetworkPolicy,
    WorkerRequest, WorkerResource, WorkerResponse, derive_docling_pdf_probe, sha256_bytes,
};

pub const DOCLING_LITE_WORKER_PROTOCOL_VERSION: &str = "weftext.docling-lite-worker-json.v1";
pub const DOCLING_LITE_MAPPING_CONTRACT_VERSION: &str = "weftext.docling-lite-mapping.v1";
pub const DOCLING_LITE_ASSET_LOCK_VERSION: &str = "weftext.docling-lite-assets.v1";
pub const DOCLING_RELEASE_TAG: &str = "v0.52.2";
pub const DOCLING_RELEASE_COMMIT: &str = "ca9fe7a543b55a540dfa18b88f4f44591b5a928e";
pub const DOCLING_DOCUMENT_SCHEMA_NAME: &str = "DoclingDocument";
pub const DOCLING_DOCUMENT_SCHEMA_VERSION: &str = "1.10.0";

const ADAPTER_ID: &str = "weftext.pdf-docling-lite-adapter";
const UNAVAILABLE_ADAPTER_VERSION: &str = "0.52.2-unavailable";
pub(crate) const WORKER_ID: &str = "weftext.docling-lite-worker";
const INPUT_LOCATOR: &str = "input/source.pdf";
const OUTPUT_LOCATOR: &str = "output/docling-document.json";
const FORMULA_PLACEHOLDER: &str = "<!-- formula-not-decoded -->";
pub const DOCLING_LITE_INSTALLATION_LOCK_FILE: &str = "docling-lite-assets.lock.json";
const MAX_ASSET_LOCK_BYTES: u64 = 1024 * 1024;
const MIN_WORKER_RESPONSE_BYTES: u64 = 4096;
const REQUIRED_COMPONENTS: [(&str, DoclingLiteArtifactRole); 6] = [
    ("docling-rs", DoclingLiteArtifactRole::WorkerBinary),
    ("pdfium", DoclingLiteArtifactRole::NativeLibrary),
    ("onnx-runtime", DoclingLiteArtifactRole::EmbeddedComponent),
    ("layout-int8", DoclingLiteArtifactRole::Model),
    ("pp-ocr", DoclingLiteArtifactRole::Model),
    ("ocr-dictionary", DoclingLiteArtifactRole::Dictionary),
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoclingLiteCapability {
    pub available: bool,
    pub code: String,
    pub message: String,
    pub missing_pinned_evidence: Vec<String>,
    pub missing_isolation_evidence: Vec<String>,
    pub ambient_network_allowed: bool,
}

impl DoclingLiteCapability {
    #[must_use]
    pub fn unavailable() -> Self {
        Self::unavailable_for(
            vec![
                "a pinned docling.rs worker binary".to_owned(),
                "a pinned PDFium native library".to_owned(),
                "pinned ONNX Runtime evidence bound to the worker binary".to_owned(),
                "the pinned INT8 layout model".to_owned(),
                "the pinned English PP-OCRv3 model".to_owned(),
                "the pinned English OCR dictionary".to_owned(),
            ],
            Self::required_isolation_evidence(),
        )
    }

    fn required_isolation_evidence() -> Vec<String> {
        vec![
            "a proven deny-by-default network sandbox".to_owned(),
            "a proven per-process memory sandbox".to_owned(),
            "a proven filesystem/process-tree sandbox for the worker".to_owned(),
        ]
    }

    fn unavailable_for(
        missing_pinned_evidence: Vec<String>,
        missing_isolation_evidence: Vec<String>,
    ) -> Self {
        let code = if missing_pinned_evidence.is_empty() {
            "docling_lite_process_isolation_unavailable"
        } else {
            "docling_lite_installation_not_verified"
        };
        Self {
            available: false,
            code: code.to_owned(),
            message: if missing_pinned_evidence.is_empty() {
                "docling.rs Lite assets are pinned, but execution is disabled because this build cannot prove the required host isolation"
                    .to_owned()
            } else {
                "docling.rs Lite is disabled until every installed worker/runtime/model asset matches one complete reviewed lock"
                    .to_owned()
            },
            missing_pinned_evidence,
            missing_isolation_evidence,
            ambient_network_allowed: false,
        }
    }

    fn verified(target: &str, package_bytes: u64) -> Self {
        Self {
            available: true,
            code: "docling_lite_verified_offline".to_owned(),
            message: format!(
                "docling.rs {DOCLING_RELEASE_TAG} Lite installation for {target} is fully pinned ({package_bytes} bytes) and restricted to offline execution"
            ),
            missing_pinned_evidence: Vec::new(),
            missing_isolation_evidence: Vec::new(),
            ambient_network_allowed: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoclingLiteMappingContract {
    pub contract_version: String,
    pub worker_protocol_version: String,
    pub worker_schema_is_internal: bool,
    pub semantic_mappings: Vec<String>,
    pub unsupported_construct_policy: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoclingLiteArtifactRole {
    WorkerBinary,
    NativeLibrary,
    EmbeddedComponent,
    Model,
    Dictionary,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoclingLiteAssetPin {
    pub component: String,
    pub version: String,
    pub role: DoclingLiteArtifactRole,
    pub target: String,
    pub install_path: PortablePath,
    pub source_url: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub license: String,
    pub notice_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoclingLiteDistributionPin {
    pub artifact: String,
    pub target: String,
    pub source_url: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub license: String,
    pub notice_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoclingLiteAssetLock {
    pub lock_version: String,
    pub docling_release_tag: String,
    pub docling_release_commit: String,
    pub document_schema_name: String,
    pub document_schema_version: String,
    pub profile: String,
    pub target: String,
    pub network: WorkerNetworkPolicy,
    pub no_table_former: bool,
    pub ocr_language: String,
    pub total_package_bytes: u64,
    pub complete_for_execution: bool,
    pub missing_for_execution: Vec<String>,
    pub artifacts: Vec<DoclingLiteAssetPin>,
    pub distribution_archives: Vec<DoclingLiteDistributionPin>,
}

impl DoclingLiteAssetLock {
    /// Parses and validates reviewed lock metadata. An audit-only incomplete
    /// lock is valid metadata, but cannot enable the adapter.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown profile/release, duplicate or malformed
    /// pins, mutable/download-latest URLs, or inconsistent package byte totals.
    pub fn from_json(bytes: &[u8]) -> Result<Self, ImportError> {
        let lock: Self = serde_json::from_slice(bytes).map_err(|error| {
            ImportError::new(
                ImportErrorCode::InvalidContract,
                format!("docling.rs Lite asset lock is invalid JSON: {error}"),
            )
        })?;
        lock.validate_metadata()?;
        Ok(lock)
    }

    /// Returns the checked sum recorded by the artifact rows.
    ///
    /// # Errors
    ///
    /// Returns an error if the sum overflows.
    pub fn computed_total_bytes(&self) -> Result<u64, ImportError> {
        let mut physical_files = BTreeMap::new();
        let mut total = 0_u64;
        for artifact in &self.artifacts {
            if let Some((byte_length, sha256)) = physical_files.get(artifact.install_path.as_str())
            {
                if *byte_length != artifact.byte_length || *sha256 != &artifact.sha256 {
                    return Err(ImportError::new(
                        ImportErrorCode::InvalidContract,
                        "components sharing one installed file disagree on its bytes",
                    ));
                }
                continue;
            }
            physical_files.insert(
                artifact.install_path.as_str(),
                (artifact.byte_length, &artifact.sha256),
            );
            total = total.checked_add(artifact.byte_length).ok_or_else(|| {
                ImportError::new(
                    ImportErrorCode::LimitExceeded,
                    "docling.rs Lite package byte total overflowed",
                )
            })?;
        }
        Ok(total)
    }

    fn validate_metadata(&self) -> Result<(), ImportError> {
        if self.lock_version != DOCLING_LITE_ASSET_LOCK_VERSION
            || self.docling_release_tag != DOCLING_RELEASE_TAG
            || self.docling_release_commit != DOCLING_RELEASE_COMMIT
            || self.document_schema_name != DOCLING_DOCUMENT_SCHEMA_NAME
            || self.document_schema_version != DOCLING_DOCUMENT_SCHEMA_VERSION
            || self.profile != "pdfium-layout-int8-ppocrv3-en"
            || self.target.is_empty()
            || self.target.len() > 128
            || !self
                .target
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            || self.network != WorkerNetworkPolicy::Denied
            || !self.no_table_former
            || self.ocr_language != "en"
        {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite lock differs from the reviewed v0.52.2 offline INT8/English profile",
            ));
        }
        if self.complete_for_execution != self.missing_for_execution.is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite lock completeness flag disagrees with its missing evidence",
            ));
        }
        if self.artifacts.is_empty() || self.artifacts.len() > 64 {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite lock must contain a bounded artifact inventory",
            ));
        }
        let mut components = BTreeSet::new();
        let mut paths = BTreeMap::new();
        for artifact in &self.artifacts {
            let reviewed_role = REQUIRED_COMPONENTS
                .iter()
                .find_map(|(component, role)| (*component == artifact.component).then_some(*role));
            if artifact.component.is_empty()
                || artifact.component.len() > 128
                || artifact.version.is_empty()
                || artifact.version.len() > 128
                || artifact.target != self.target
                || artifact.byte_length == 0
                || artifact.license.is_empty()
                || artifact.license.len() > 256
                || artifact.notice_id.is_empty()
                || artifact.notice_id.len() > 128
                || !artifact.source_url.starts_with("https://")
                || artifact.source_url.len() > 2_048
                || artifact.source_url.contains("/latest")
                || artifact.source_url.contains("releases/latest")
                || !components.insert(artifact.component.as_str())
                || reviewed_role != Some(artifact.role)
                || (artifact.role == DoclingLiteArtifactRole::EmbeddedComponent
                    && artifact.component != "onnx-runtime")
            {
                return Err(ImportError::new(
                    ImportErrorCode::InvalidContract,
                    "docling.rs Lite lock contains an incomplete, duplicate, mutable, or cross-target artifact pin",
                ));
            }
            register_asset_path(&mut paths, artifact)?;
        }
        validate_embedded_components(&self.artifacts)?;
        if self.computed_total_bytes()? != self.total_package_bytes {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite lock totalPackageBytes differs from the checked artifact sum",
            ));
        }
        let mut distribution_targets = BTreeSet::new();
        for distribution in &self.distribution_archives {
            if distribution.artifact.is_empty()
                || distribution.artifact.len() > 128
                || distribution.target.is_empty()
                || distribution.target.len() > 128
                || distribution.byte_length == 0
                || !distribution.source_url.starts_with("https://")
                || distribution.source_url.len() > 2_048
                || distribution.source_url.contains("/latest")
                || distribution.license.is_empty()
                || distribution.license.len() > 256
                || distribution.notice_id.is_empty()
                || distribution.notice_id.len() > 128
                || !distribution_targets.insert(distribution.target.as_str())
            {
                return Err(ImportError::new(
                    ImportErrorCode::InvalidContract,
                    "docling.rs Lite lock contains malformed or duplicate distribution archive evidence",
                ));
            }
        }
        Ok(())
    }

    fn validate_complete(&self) -> Result<(), ImportError> {
        if !self.complete_for_execution || !self.missing_for_execution.is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                format!(
                    "docling.rs Lite lock is audit-only; missing: {}",
                    self.missing_for_execution.join(", ")
                ),
            ));
        }
        for (component, role) in REQUIRED_COMPONENTS {
            if !self
                .artifacts
                .iter()
                .any(|artifact| artifact.component == component && artifact.role == role)
            {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    format!(
                        "docling.rs Lite lock is missing required {component} {role:?} evidence"
                    ),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct PhysicalArtifactBinding {
    component: String,
    role: DoclingLiteArtifactRole,
    byte_length: u64,
    sha256: Sha256Digest,
}

fn register_asset_path(
    paths: &mut BTreeMap<String, PhysicalArtifactBinding>,
    artifact: &DoclingLiteAssetPin,
) -> Result<(), ImportError> {
    if let Some(existing) = paths.get(artifact.install_path.as_str()) {
        let worker_and_embedded_ort = (existing.component == "docling-rs"
            && existing.role == DoclingLiteArtifactRole::WorkerBinary
            && artifact.component == "onnx-runtime"
            && artifact.role == DoclingLiteArtifactRole::EmbeddedComponent)
            || (existing.component == "onnx-runtime"
                && existing.role == DoclingLiteArtifactRole::EmbeddedComponent
                && artifact.component == "docling-rs"
                && artifact.role == DoclingLiteArtifactRole::WorkerBinary);
        if !worker_and_embedded_ort
            || existing.byte_length != artifact.byte_length
            || existing.sha256 != artifact.sha256
        {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "only embedded ONNX Runtime may share the exact worker binary pin",
            ));
        }
        return Ok(());
    }
    paths.insert(
        artifact.install_path.as_str().to_owned(),
        PhysicalArtifactBinding {
            component: artifact.component.clone(),
            role: artifact.role,
            byte_length: artifact.byte_length,
            sha256: artifact.sha256.clone(),
        },
    );
    Ok(())
}

fn validate_embedded_components(artifacts: &[DoclingLiteAssetPin]) -> Result<(), ImportError> {
    let Some(onnx_runtime) = artifacts
        .iter()
        .find(|artifact| artifact.component == "onnx-runtime")
    else {
        return Ok(());
    };
    let worker = artifacts
        .iter()
        .find(|artifact| artifact.component == "docling-rs")
        .ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::InvalidContract,
                "embedded ONNX Runtime evidence has no worker binary",
            )
        })?;
    if onnx_runtime.role != DoclingLiteArtifactRole::EmbeddedComponent
        || worker.role != DoclingLiteArtifactRole::WorkerBinary
        || onnx_runtime.install_path != worker.install_path
        || onnx_runtime.byte_length != worker.byte_length
        || onnx_runtime.sha256 != worker.sha256
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "ONNX Runtime must be represented as a component embedded in the exact worker binary",
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoclingLiteInstalledArtifact {
    pub install_path: PortablePath,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DoclingModelPin {
    pub component: String,
    pub version: String,
    pub sha256: Sha256Digest,
    pub notice_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DoclingLiteWorkerCommand {
    pub protocol_version: String,
    pub request_id: String,
    pub source_digest: Sha256Digest,
    pub plan_id: String,
    pub input_locator: PortablePath,
    pub output_locator: PortablePath,
    pub docling_release_tag: String,
    pub docling_release_commit: String,
    pub document_schema_name: String,
    pub document_schema_version: String,
    pub target: String,
    pub local_ocr_policy: LocalOcrPolicy,
    pub ocr_language: String,
    pub layout_precision: String,
    pub no_table_former: bool,
    pub network: WorkerNetworkPolicy,
    pub page_limit: u32,
    pub memory_limit_bytes: u64,
    pub output_byte_limit: u64,
    pub model_pins: Vec<DoclingModelPin>,
}

impl DoclingLiteWorkerCommand {
    /// Validates the isolated, download-free JSON wire shape.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown release/schema, unsafe options or
    /// locators, missing/duplicate pins, network access, or zero limits.
    pub fn validate_boundary(&self) -> Result<(), ImportError> {
        if self.protocol_version != DOCLING_LITE_WORKER_PROTOCOL_VERSION
            || self.request_id.is_empty()
            || self.plan_id.is_empty()
            || self.input_locator.as_str() != INPUT_LOCATOR
            || self.output_locator.as_str() != OUTPUT_LOCATOR
            || self.docling_release_tag != DOCLING_RELEASE_TAG
            || self.docling_release_commit != DOCLING_RELEASE_COMMIT
            || self.document_schema_name != DOCLING_DOCUMENT_SCHEMA_NAME
            || self.document_schema_version != DOCLING_DOCUMENT_SCHEMA_VERSION
            || self.target.is_empty()
            || self.local_ocr_policy == LocalOcrPolicy::Never
            || self.ocr_language != "en"
            || self.layout_precision != "int8"
            || !self.no_table_former
            || self.network != WorkerNetworkPolicy::Denied
            || self.page_limit == 0
            || self.memory_limit_bytes == 0
            || self.output_byte_limit < MIN_WORKER_RESPONSE_BYTES
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "docling.rs Lite worker command violates its reviewed offline JSON boundary",
            ));
        }
        let mut components = BTreeSet::new();
        for pin in &self.model_pins {
            if pin.component.is_empty()
                || pin.component.len() > 128
                || pin.version.is_empty()
                || pin.version.len() > 128
                || pin.notice_id.is_empty()
                || pin.notice_id.len() > 128
                || !components.insert(pin.component.as_str())
            {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite component pins must be complete and unique",
                ));
            }
        }
        if self.model_pins.len() != REQUIRED_COMPONENTS.len()
            || REQUIRED_COMPONENTS
                .iter()
                .any(|(component, _)| !components.contains(component))
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "docling.rs Lite worker command is missing a required pinned component",
            ));
        }
        let json =
            serde_json::to_string(self).map_err(|error| ImportError::serialization(&error))?;
        if json.contains("http://")
            || json.contains("https://")
            || json.contains("downloadUrl")
            || json.contains("sourceUrl")
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "docling.rs Lite worker commands must not contain download locations",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoclingLiteWorkerStatus {
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DoclingLiteWorkerResponse {
    pub protocol_version: String,
    pub request_id: String,
    pub source_digest: Sha256Digest,
    pub status: DoclingLiteWorkerStatus,
    pub docling_document_json: Option<Value>,
    pub resources: Vec<WorkerResource>,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub components: Vec<ComponentVersion>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DoclingLiteGenericPayload {
    status: DoclingLiteWorkerStatus,
    docling_document_json: Option<Value>,
}

impl DoclingLiteWorkerResponse {
    /// Converts the generic Weftext worker envelope to the Docling wire
    /// contract. The only successful payload shape is the direct
    /// `DoclingDocument` object emitted by the real worker; status wrappers are
    /// reserved for failed or cancelled responses.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign worker/protocol/response contract or a
    /// malformed Docling status wrapper.
    pub fn try_from_generic(response: WorkerResponse) -> Result<Self, ImportError> {
        if response.contract_version != WORKER_RESPONSE_CONTRACT_VERSION
            || response.worker_id != WORKER_ID
            || response.worker_protocol_version != DOCLING_LITE_WORKER_PROTOCOL_VERSION
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "generic response does not belong to the docling.rs Lite worker",
            ));
        }
        let (status, docling_document_json) = if looks_like_docling_document(&response.payload) {
            (DoclingLiteWorkerStatus::Completed, Some(response.payload))
        } else {
            let has_document_field = response
                .payload
                .as_object()
                .is_some_and(|payload| payload.contains_key("doclingDocumentJson"));
            let payload: DoclingLiteGenericPayload = serde_json::from_value(response.payload)
                .map_err(|error| {
                    ImportError::new(
                        ImportErrorCode::WorkerProtocol,
                        format!("docling.rs Lite response payload is invalid: {error}"),
                    )
                })?;
            if !has_document_field
                || payload.status == DoclingLiteWorkerStatus::Completed
                || payload.docling_document_json.is_some()
            {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite status wrappers are limited to failed or cancelled responses with an explicit null document",
                ));
            }
            (payload.status, payload.docling_document_json)
        };
        Ok(Self {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: response.request_id,
            source_digest: response.source_digest,
            status,
            docling_document_json,
            resources: response.resources,
            diagnostics: response.diagnostics,
            components: response.components,
        })
    }

    /// Wraps the typed status/payload in the generic Weftext worker envelope.
    ///
    /// # Errors
    ///
    /// Returns an error if the typed payload cannot be serialized.
    pub fn into_generic(self) -> Result<WorkerResponse, ImportError> {
        let payload = match (self.status, self.docling_document_json) {
            (DoclingLiteWorkerStatus::Completed, Some(document))
                if looks_like_docling_document(&document) =>
            {
                document
            }
            (
                status @ (DoclingLiteWorkerStatus::Cancelled | DoclingLiteWorkerStatus::Failed),
                None,
            ) => serde_json::to_value(DoclingLiteGenericPayload {
                status,
                docling_document_json: None,
            })
            .map_err(|error| ImportError::serialization(&error))?,
            _ => {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite response status and document payload disagree",
                ));
            }
        };
        Ok(WorkerResponse {
            contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: self.request_id,
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: self.protocol_version,
            source_digest: self.source_digest,
            payload,
            resources: self.resources,
            diagnostics: self.diagnostics,
            components: self.components,
        })
    }

    /// Validates the process response against one exact worker command.
    ///
    /// # Errors
    ///
    /// Returns an error for identity/pin mismatch, inconsistent status/payload,
    /// corrupt resources, or response limits exceeded.
    pub fn validate_boundary(
        &self,
        command: &DoclingLiteWorkerCommand,
        limits: &ImportLimits,
    ) -> Result<(), ImportError> {
        command.validate_boundary()?;
        if self.protocol_version != DOCLING_LITE_WORKER_PROTOCOL_VERSION
            || self.request_id != command.request_id
            || self.source_digest != command.source_digest
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "docling.rs Lite response identity differs from its command",
            ));
        }
        match (self.status, self.docling_document_json.is_some()) {
            (DoclingLiteWorkerStatus::Completed, true)
            | (DoclingLiteWorkerStatus::Cancelled | DoclingLiteWorkerStatus::Failed, false) => {}
            _ => {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite response status and document payload disagree",
                ));
            }
        }
        let payload_bytes = self
            .docling_document_json
            .as_ref()
            .map(serde_json::to_vec)
            .transpose()
            .map_err(|error| ImportError::serialization(&error))?
            .map_or(0, |bytes| bytes.len());
        limits.check(
            "docling.rs Lite JSON payload bytes",
            usize_to_u64(payload_bytes),
            command.output_byte_limit,
        )?;
        limits.check(
            "docling.rs Lite resource count",
            usize_to_u64(self.resources.len()),
            u64::from(limits.max_resource_count),
        )?;
        let mut resource_total = 0_u64;
        let mut paths = BTreeSet::new();
        for resource in &self.resources {
            if !paths.insert(resource.locator.as_str())
                || resource.byte_length != usize_to_u64(resource.bytes.len())
                || resource.sha256 != sha256_bytes(&resource.bytes)
            {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite resource locator, digest, or length is corrupt",
                ));
            }
            limits.check(
                "docling.rs Lite resource bytes",
                resource.byte_length,
                limits.max_resource_bytes,
            )?;
            resource_total = checked_add(resource_total, resource.byte_length, "resource bytes")?;
        }
        let total = checked_add(usize_to_u64(payload_bytes), resource_total, "worker output")?;
        limits.check(
            "docling.rs Lite total output bytes",
            total,
            command.output_byte_limit,
        )?;
        let expected = command
            .model_pins
            .iter()
            .map(|pin| (pin.component.as_str(), pin))
            .collect::<BTreeMap<_, _>>();
        if self.components.len() != expected.len() {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "docling.rs Lite response did not attest every pinned component",
            ));
        }
        let mut seen = BTreeSet::new();
        for component in &self.components {
            let Some(pin) = expected.get(component.component_id.as_str()) else {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite response attested an unplanned component",
                ));
            };
            if !seen.insert(component.component_id.as_str())
                || component.version != pin.version
                || component.artifact_digest.as_ref() != Some(&pin.sha256)
            {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "docling.rs Lite response component evidence differs from the frozen pins",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct VerifiedInstallation {
    lock: DoclingLiteAssetLock,
    lock_digest: Sha256Digest,
    model_pins: Vec<DoclingModelPin>,
    installation_root: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ExecutionIsolation {
    #[default]
    Unavailable,
    #[cfg(test)]
    FixtureOnly,
}

#[derive(Clone, Debug, Default)]
pub struct DoclingLitePdfAdapter {
    installation: Option<VerifiedInstallation>,
    execution_isolation: ExecutionIsolation,
}

impl DoclingLitePdfAdapter {
    /// Verifies detached artifact bytes for mapping/tests without claiming
    /// executable installation or process-isolation capability.
    ///
    /// # Errors
    ///
    /// Returns an error for incomplete locks, target mismatch, missing/extra
    /// artifacts, or any length/digest mismatch.
    pub fn from_verified_assets(
        lock_json: &[u8],
        target: &str,
        installed: &[DoclingLiteInstalledArtifact],
    ) -> Result<Self, ImportError> {
        let lock = DoclingLiteAssetLock::from_json(lock_json)?;
        lock.validate_complete()?;
        if target != lock.target {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite installed target differs from the reviewed lock target",
            ));
        }
        let expected_paths = lock
            .artifacts
            .iter()
            .map(|artifact| artifact.install_path.as_str())
            .collect::<BTreeSet<_>>();
        if installed.len() != expected_paths.len() {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite installed artifact inventory is incomplete or contains extras",
            ));
        }
        let evidence = installed
            .iter()
            .map(|artifact| (artifact.install_path.as_str(), artifact))
            .collect::<BTreeMap<_, _>>();
        if evidence.len() != installed.len() {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite installed artifact paths are not unique",
            ));
        }
        for pin in &lock.artifacts {
            let Some(artifact) = evidence.get(pin.install_path.as_str()) else {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    format!(
                        "docling.rs Lite installed artifact is missing: {}",
                        pin.install_path
                    ),
                ));
            };
            if usize_to_u64(artifact.bytes.len()) != pin.byte_length
                || sha256_bytes(&artifact.bytes) != pin.sha256
            {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    format!(
                        "docling.rs Lite installed artifact failed its byte/SHA pin: {}",
                        pin.install_path
                    ),
                ));
            }
        }
        let model_pins = lock
            .artifacts
            .iter()
            .map(|artifact| DoclingModelPin {
                component: artifact.component.clone(),
                version: artifact.version.clone(),
                sha256: artifact.sha256.clone(),
                notice_id: artifact.notice_id.clone(),
            })
            .collect();
        Ok(Self {
            installation: Some(VerifiedInstallation {
                lock,
                lock_digest: sha256_bytes(lock_json),
                model_pins,
                installation_root: None,
            }),
            execution_isolation: ExecutionIsolation::Unavailable,
        })
    }

    /// Verifies one fixed installation directory without following links or
    /// accepting unlisted files as asset evidence. Verification alone does not
    /// claim that the host can sandbox the worker process.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe installation root/lock, incomplete pins,
    /// target mismatch, links/reparse points, hard links, size changes, or SHA mismatch.
    pub fn from_installation_directory(
        installation_root: impl AsRef<Path>,
        target: &str,
    ) -> Result<Self, ImportError> {
        let installation_root = canonical_regular_directory(installation_root.as_ref())?;
        let lock_path = installation_root.join(DOCLING_LITE_INSTALLATION_LOCK_FILE);
        let lock_json = read_regular_non_link(&lock_path, MAX_ASSET_LOCK_BYTES)?;
        let lock = DoclingLiteAssetLock::from_json(&lock_json)?;
        lock.validate_complete()?;
        if lock.target != target {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                format!(
                    "docling.rs Lite installation target {} differs from host target {target}",
                    lock.target
                ),
            ));
        }
        verify_installation_inventory(&installation_root, &lock)?;
        verify_installed_files(&installation_root, &lock)?;
        let model_pins = lock
            .artifacts
            .iter()
            .map(|artifact| DoclingModelPin {
                component: artifact.component.clone(),
                version: artifact.version.clone(),
                sha256: artifact.sha256.clone(),
                notice_id: artifact.notice_id.clone(),
            })
            .collect();
        Ok(Self {
            installation: Some(VerifiedInstallation {
                lock,
                lock_digest: sha256_bytes(&lock_json),
                model_pins,
                installation_root: Some(installation_root),
            }),
            execution_isolation: ExecutionIsolation::Unavailable,
        })
    }

    #[must_use]
    pub fn capability(&self) -> DoclingLiteCapability {
        self.installation
            .as_ref()
            .map_or_else(DoclingLiteCapability::unavailable, |installation| {
                if self.execution_isolation != ExecutionIsolation::Unavailable {
                    return DoclingLiteCapability::verified(
                        &installation.lock.target,
                        installation.lock.total_package_bytes,
                    );
                }
                let missing_pinned = if installation.installation_root.is_some() {
                    Vec::new()
                } else {
                    vec![
                        "artifacts bound to regular files in one fixed installation directory"
                            .to_owned(),
                    ]
                };
                DoclingLiteCapability::unavailable_for(
                    missing_pinned,
                    DoclingLiteCapability::required_isolation_evidence(),
                )
            })
    }

    #[must_use]
    pub fn mapping_contract(&self) -> DoclingLiteMappingContract {
        DoclingLiteMappingContract {
            contract_version: DOCLING_LITE_MAPPING_CONTRACT_VERSION.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            worker_schema_is_internal: true,
            semantic_mappings: vec![
                "Docling title/section_header refs -> document title and ImportNodeKind::Section".to_owned(),
                "Docling paragraph/text refs -> ImportNodeKind::Paragraph".to_owned(),
                "Docling list groups/list_item refs -> ImportNodeKind::List".to_owned(),
                "Docling table grid -> ImportNodeKind::Table with geometric fallback diagnostic".to_owned(),
                "Docling picture data URI -> WorkerResource/ImportResource and ImportNodeKind::Figure".to_owned(),
                "Docling formula or undecoded marker -> ImportNodeKind::Formula".to_owned(),
                "Docling page/bbox provenance -> normalized ImportSourceLocation".to_owned(),
            ],
            unsupported_construct_policy:
                "preserve bounded text/evidence where possible, diagnose schema loss, and reject bad schemas, references, geometry, or resource encodings"
                    .to_owned(),
        }
    }

    /// Returns the supervised process worker only when both installation and
    /// host isolation evidence are available. Current product targets fail
    /// closed here because their network/memory/filesystem sandbox is not yet
    /// proven under the pinned safe-Rust policy.
    ///
    /// # Errors
    ///
    /// Always returns the explicit capability blocker until a reviewed sandbox
    /// implementation can construct the worker.
    pub fn process_worker(&self) -> Result<Arc<crate::DoclingLiteProcessWorker>, ImportError> {
        Err(self.unavailable_error())
    }

    fn unavailable_error(&self) -> ImportError {
        let capability = self.capability();
        ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
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
        )
    }

    #[cfg(test)]
    fn enable_fixture_execution(mut self) -> Self {
        self.execution_isolation = ExecutionIsolation::FixtureOnly;
        self
    }

    fn verified(&self) -> Result<&VerifiedInstallation, ImportError> {
        self.installation
            .as_ref()
            .ok_or_else(|| self.unavailable_error())
    }

    fn command(
        &self,
        source: &SourceArtifact,
        plan: &ImportPlan,
        input_locator: PortablePath,
    ) -> Result<DoclingLiteWorkerCommand, ImportError> {
        let installation = self.verified()?;
        let command = DoclingLiteWorkerCommand {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: request_id(plan)?,
            source_digest: source.sha256.clone(),
            plan_id: plan.plan_id.clone(),
            input_locator,
            output_locator: PortablePath::parse(OUTPUT_LOCATOR)?,
            docling_release_tag: DOCLING_RELEASE_TAG.to_owned(),
            docling_release_commit: DOCLING_RELEASE_COMMIT.to_owned(),
            document_schema_name: DOCLING_DOCUMENT_SCHEMA_NAME.to_owned(),
            document_schema_version: DOCLING_DOCUMENT_SCHEMA_VERSION.to_owned(),
            target: installation.lock.target.clone(),
            local_ocr_policy: plan.local_ocr_policy,
            ocr_language: "en".to_owned(),
            layout_precision: "int8".to_owned(),
            no_table_former: true,
            network: WorkerNetworkPolicy::Denied,
            page_limit: plan.limits.max_pages,
            memory_limit_bytes: plan.limits.worker_memory_bytes,
            output_byte_limit: plan.limits.max_total_output_bytes,
            model_pins: installation.model_pins.clone(),
        };
        command.validate_boundary()?;
        Ok(command)
    }
}

impl ImportAdapter for DoclingLitePdfAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        let adapter_version = self.installation.as_ref().map_or_else(
            || UNAVAILABLE_ADAPTER_VERSION.to_owned(),
            |installation| format!("0.52.2-lock-{}", &installation.lock_digest.as_str()[..16]),
        );
        AdapterDescriptor {
            adapter_id: ADAPTER_ID.to_owned(),
            adapter_version,
            supported_format: SourceFormat::Pdf,
        }
    }

    fn probe(
        &self,
        source: &SourceArtifact,
        bounded_evidence: &mut ProbeReader<'_>,
        limits: &ImportLimits,
    ) -> Result<FormatProbe, ImportError> {
        let capability = self.capability();
        derive_docling_pdf_probe(
            source,
            bounded_evidence,
            limits,
            self.descriptor(),
            capability.available,
            &capability.message,
        )
    }

    fn plan(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
        request: PlanRequest,
        limits: ImportLimits,
    ) -> Result<ImportPlan, ImportError> {
        self.verified()?;
        if !self.capability().available {
            return Err(self.unavailable_error());
        }
        if probe.detected_format != SourceFormat::Pdf
            || !probe.safe_to_plan
            || probe.encryption != EncryptionState::NotEncrypted
            || probe.active_content_detected
            || probe.adapter != self.descriptor()
        {
            return Err(ImportError::new(
                ImportErrorCode::ProbeRejected,
                "docling.rs Lite only plans an exact, unencrypted, inactive PDF probe from this verified installation",
            ));
        }
        if request.local_ocr_policy == LocalOcrPolicy::Never {
            return Err(ImportError::new(
                ImportErrorCode::ProbeRejected,
                "no-OCR Docling execution is diagnostic-only and cannot become a committed PDF import plan",
            ));
        }
        if !matches!(&request.split_policy, SplitPolicy::SingleNode)
            || request.resource_policy != ResourcePolicy::ExtractReferenced
            || !matches!(&request.agent_enhancement, AgentEnhancementPolicy::Disabled)
            || !matches!(&request.egress, EgressDisclosure::None)
        {
            return Err(ImportError::new(
                ImportErrorCode::ProbeRejected,
                "docling.rs Lite conversion requires the reviewed offline single-node local worker phase",
            ));
        }
        ImportPlan::create(
            source,
            probe,
            AdapterRoute {
                adapter: self.descriptor(),
                worker_id: WORKER_ID.to_owned(),
                worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
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
        self.verified()?;
        if !self.capability().available {
            return Err(self.unavailable_error());
        }
        validate_frozen_plan(plan)?;
        if source.sha256 != plan.source_digest
            || source.detected_format != SourceFormat::Pdf
            || plan.route.adapter != self.descriptor()
            || plan.route.worker_id != WORKER_ID
            || plan.route.worker_protocol_version != DOCLING_LITE_WORKER_PROTOCOL_VERSION
        {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite worker request differs from its frozen source, adapter, or route",
            ));
        }
        plan.limits.validate()?;
        validate_local_worker_plan(plan)?;
        if source_locator.as_str() != INPUT_LOCATOR {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite reads only the fixed input/source.pdf session locator",
            ));
        }
        let command = self.command(source, plan, source_locator.clone())?;
        let format_options =
            serde_json::to_value(&command).map_err(|error| ImportError::serialization(&error))?;
        Ok(WorkerRequest {
            contract_version: WORKER_REQUEST_CONTRACT_VERSION.to_owned(),
            request_id: command.request_id,
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source: source.clone(),
            source_locator,
            plan: plan.clone(),
            network: WorkerNetworkPolicy::Denied,
            memory_limit_bytes: plan.limits.worker_memory_bytes,
            page_limit: plan.limits.max_pages,
            entry_limit: plan.limits.max_container_entries,
            output_byte_limit: plan.limits.max_total_output_bytes,
            format_options,
        })
    }

    fn map_worker_response(
        &self,
        source: &SourceArtifact,
        plan: &ImportPlan,
        response: WorkerResponse,
    ) -> Result<ImportDocument, ImportError> {
        self.verified()?;
        if !self.capability().available {
            return Err(self.unavailable_error());
        }
        validate_frozen_plan(plan)?;
        validate_local_worker_plan(plan)?;
        if source.sha256 != plan.source_digest || plan.route.adapter != self.descriptor() {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite mapping source/plan differs from the verified adapter",
            ));
        }
        let response = DoclingLiteWorkerResponse::try_from_generic(response)?;
        let command = self.command(source, plan, PortablePath::parse(INPUT_LOCATOR)?)?;
        response.validate_boundary(&command, &plan.limits)?;
        match response.status {
            DoclingLiteWorkerStatus::Cancelled => {
                return Err(ImportError::new(
                    ImportErrorCode::Cancelled,
                    "docling.rs Lite worker reported cancellation",
                ));
            }
            DoclingLiteWorkerStatus::Failed => {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerFailed,
                    "docling.rs Lite worker reported failure",
                ));
            }
            DoclingLiteWorkerStatus::Completed => {}
        }
        let document_json = response.docling_document_json.ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "completed docling.rs Lite response omitted its document",
            )
        })?;
        let mapper =
            DoclingDocumentMapper::new(source, plan, response.resources, response.diagnostics)?;
        let document = mapper.map(&document_json)?;
        document.validate(source, plan)?;
        Ok(document)
    }
}

/// Returns the exact reviewed target label for supported host builds. Unknown
/// targets remain unavailable instead of borrowing another platform's assets.
#[must_use]
pub const fn docling_lite_host_target() -> Option<&'static str> {
    if cfg!(all(target_arch = "x86_64", target_os = "linux")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_arch = "x86_64", target_os = "windows")) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

/// Validates the adapter-specific evidence retained by an immutable Docling
/// preview bundle without re-running the untrusted converter.
///
/// # Errors
///
/// Returns an error for a foreign route/profile, unsafe probe, mutable policy,
/// or incomplete component attestation.
pub fn validate_docling_lite_preview_evidence(
    probe: &FormatProbe,
    plan: &ImportPlan,
    components: &[ComponentVersion],
) -> Result<(), ImportError> {
    let version = plan.route.adapter.adapter_version.as_str();
    let reviewed_agent_route = match (&plan.agent_enhancement, &plan.egress) {
        (AgentEnhancementPolicy::Disabled, EgressDisclosure::None) => true,
        (
            AgentEnhancementPolicy::SelectedRegionsOnly { provider },
            EgressDisclosure::AgentSelectedEvidence {
                provider: disclosure_provider,
                selected_node_ids,
                disclosed_bytes,
                ..
            },
        ) => {
            provider == disclosure_provider && !selected_node_ids.is_empty() && *disclosed_bytes > 0
        }
        _ => false,
    };
    let lock_suffix = version.strip_prefix("0.52.2-lock-").ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::InvalidContract,
            "Docling preview adapter version does not bind a reviewed lock digest",
        )
    })?;
    if plan.route.adapter.adapter_id != ADAPTER_ID
        || plan.route.adapter.supported_format != SourceFormat::Pdf
        || lock_suffix.len() != 16
        || !lock_suffix.bytes().all(|byte| byte.is_ascii_hexdigit())
        || plan.route.worker_id != WORKER_ID
        || plan.route.worker_protocol_version != DOCLING_LITE_WORKER_PROTOCOL_VERSION
        || probe.adapter != plan.route.adapter
        || probe.detected_format != SourceFormat::Pdf
        || !probe.safe_to_plan
        || probe.encryption != EncryptionState::NotEncrypted
        || probe.active_content_detected
        || !matches!(plan.split_policy, SplitPolicy::SingleNode)
        || plan.resource_policy != ResourcePolicy::ExtractReferenced
        || !reviewed_agent_route
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "Docling preview differs from the reviewed offline single-node PDF route",
        ));
    }
    if components.len() != REQUIRED_COMPONENTS.len() {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "Docling preview omitted pinned component evidence",
        ));
    }
    let evidence = components
        .iter()
        .map(|component| (component.component_id.as_str(), component))
        .collect::<BTreeMap<_, _>>();
    if evidence.len() != components.len()
        || REQUIRED_COMPONENTS.iter().any(|(component, _)| {
            evidence.get(component).is_none_or(|evidence| {
                evidence.version.is_empty() || evidence.artifact_digest.is_none()
            })
        })
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "Docling preview component evidence is duplicate, incomplete, or unpinned",
        ));
    }
    Ok(())
}

fn canonical_regular_directory(path: &Path) -> Result<PathBuf, ImportError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| ImportError::io("inspect docling.rs Lite installation root", &error))?;
    if !metadata.is_dir() || linked_or_reparse(&metadata) {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installation root must be a regular non-link directory",
        ));
    }
    path.canonicalize()
        .map_err(|error| ImportError::io("resolve docling.rs Lite installation root", &error))
}

fn verify_installed_files(
    installation_root: &Path,
    lock: &DoclingLiteAssetLock,
) -> Result<(), ImportError> {
    for pin in &lock.artifacts {
        let path = installation_root.join(pin.install_path.as_str());
        verify_regular_ancestors(installation_root, &path)?;
        let bytes = read_regular_non_link(&path, pin.byte_length)?;
        if usize_to_u64(bytes.len()) != pin.byte_length || sha256_bytes(&bytes) != pin.sha256 {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                format!(
                    "docling.rs Lite installed artifact failed its byte/SHA pin: {}",
                    pin.install_path
                ),
            ));
        }
    }
    Ok(())
}

fn verify_installation_inventory(
    installation_root: &Path,
    lock: &DoclingLiteAssetLock,
) -> Result<(), ImportError> {
    let mut expected_files = lock
        .artifacts
        .iter()
        .map(|artifact| artifact.install_path.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    expected_files.insert(DOCLING_LITE_INSTALLATION_LOCK_FILE.to_owned());
    let mut expected_directories = BTreeSet::new();
    for path in &expected_files {
        let mut prefix = String::new();
        for component in path
            .split('/')
            .take(path.split('/').count().saturating_sub(1))
        {
            if !prefix.is_empty() {
                prefix.push('/');
            }
            prefix.push_str(component);
            expected_directories.insert(prefix.clone());
        }
    }
    let mut seen_files = BTreeSet::new();
    inspect_installation_directory(
        installation_root,
        installation_root,
        &expected_files,
        &expected_directories,
        &mut seen_files,
    )?;
    if seen_files != expected_files {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installation inventory is missing a pinned file",
        ));
    }
    Ok(())
}

fn inspect_installation_directory(
    root: &Path,
    directory: &Path,
    expected_files: &BTreeSet<String>,
    expected_directories: &BTreeSet<String>,
    seen_files: &mut BTreeSet<String>,
) -> Result<(), ImportError> {
    for entry in fs::read_dir(directory)
        .map_err(|error| ImportError::io("enumerate docling.rs Lite installation", &error))?
    {
        let entry = entry
            .map_err(|error| ImportError::io("inspect docling.rs Lite installation", &error))?;
        let entry_path = entry.path();
        let metadata = fs::symlink_metadata(&entry_path).map_err(|error| {
            ImportError::io("inspect docling.rs Lite installation entry", &error)
        })?;
        if linked_or_reparse(&metadata) {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite installation inventory contains a link or reparse point",
            ));
        }
        let relative = entry_path.strip_prefix(root).map_err(|_| {
            ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite installation entry escaped its root",
            )
        })?;
        let portable = relative
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    "docling.rs Lite installation entry name is not UTF-8",
                )
            })?
            .join("/");
        if metadata.is_dir() {
            if !expected_directories.contains(&portable) {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    format!("unexpected directory in docling.rs Lite installation: {portable}"),
                ));
            }
            inspect_installation_directory(
                root,
                &entry_path,
                expected_files,
                expected_directories,
                seen_files,
            )?;
        } else if metadata.is_file() && expected_files.contains(&portable) {
            seen_files.insert(portable);
        } else {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                format!("unexpected file in docling.rs Lite installation: {portable}"),
            ));
        }
    }
    Ok(())
}

fn verify_regular_ancestors(root: &Path, file: &Path) -> Result<(), ImportError> {
    let parent = file.parent().ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact has no parent directory",
        )
    })?;
    if !parent.starts_with(root) {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact escaped its installation root",
        ));
    }
    let relative = parent.strip_prefix(root).map_err(|_| {
        ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact escaped its installation root",
        )
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| ImportError::io("inspect docling.rs Lite artifact parent", &error))?;
        if !metadata.is_dir() || linked_or_reparse(&metadata) {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "docling.rs Lite artifact parents must be regular non-link directories",
            ));
        }
    }
    Ok(())
}

fn read_regular_non_link(path: &Path, maximum_bytes: u64) -> Result<Vec<u8>, ImportError> {
    let before = fs::symlink_metadata(path)
        .map_err(|error| ImportError::io("inspect docling.rs Lite installed artifact", &error))?;
    if !before.is_file() || linked_or_reparse(&before) || multiply_linked(&before) {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            format!(
                "docling.rs Lite installed artifact must be one regular, single-link file: {}",
                path.display()
            ),
        ));
    }
    if before.len() > maximum_bytes {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            format!(
                "docling.rs Lite installed artifact exceeds its pinned byte length: {}",
                path.display()
            ),
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt as _;
        // Stable Rust cannot report a Windows hard-link count here. An exclusive
        // read handle is stronger evidence for the operation we need here: it
        // refuses any already-open writer/deleter and prevents every path to the
        // same file identity from being opened for mutation while bytes are read.
        options.share_mode(0);
    }
    let mut file = options
        .open(path)
        .map_err(|error| ImportError::io("open docling.rs Lite installed artifact", &error))?;
    let opened = file
        .metadata()
        .map_err(|error| ImportError::io("inspect open docling.rs Lite artifact", &error))?;
    if !opened.is_file() || linked_or_reparse(&opened) || multiply_linked(&opened) {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact changed while it was opened",
        ));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(maximum_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| ImportError::io("hash docling.rs Lite installed artifact", &error))?;
    if usize_to_u64(bytes.len()) > maximum_bytes {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact grew beyond its pinned byte length",
        ));
    }
    let after = fs::symlink_metadata(path)
        .map_err(|error| ImportError::io("reinspect docling.rs Lite installed artifact", &error))?;
    if after.len() != before.len()
        || after.modified().ok() != before.modified().ok()
        || linked_or_reparse(&after)
        || multiply_linked(&after)
    {
        return Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            "docling.rs Lite installed artifact changed during verification",
        ));
    }
    Ok(bytes)
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

fn multiply_linked(metadata: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt as _;
        metadata.nlink() != 1
    }
    #[cfg(windows)]
    {
        // Windows verification opens the file with share_mode(0), so any
        // existing hard-link writer/deleter makes the open fail and no new one
        // can race the byte/SHA read. The process launcher repeats this check
        // immediately before executing pinned bytes.
        let _ = metadata;
        false
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

#[derive(Clone, Copy)]
struct PageSize {
    width: f64,
    height: f64,
}

#[derive(Clone)]
enum MappedBlock {
    Heading {
        level: u8,
        title: String,
        source_locations: Vec<ImportSourceLocation>,
    },
    Node(ImportNode),
}

struct DoclingDocumentMapper<'a> {
    source: &'a SourceArtifact,
    plan: &'a ImportPlan,
    pages: BTreeMap<u32, PageSize>,
    collections: BTreeMap<&'static str, &'a [Value]>,
    diagnostics: Vec<ImportDiagnostic>,
    resources: Vec<ImportResource>,
    worker_resources: Vec<WorkerResource>,
    body_seen: BTreeSet<String>,
    body_visiting: BTreeSet<String>,
    node_counter: u32,
    resource_counter: u32,
    decoded_resource_bytes: u64,
}

impl<'a> DoclingDocumentMapper<'a> {
    fn new(
        source: &'a SourceArtifact,
        plan: &'a ImportPlan,
        worker_resources: Vec<WorkerResource>,
        diagnostics: Vec<ImportDiagnostic>,
    ) -> Result<Self, ImportError> {
        plan.limits.check(
            "docling.rs Lite diagnostic count",
            usize_to_u64(diagnostics.len()),
            u64::from(plan.limits.max_diagnostics),
        )?;
        let decoded_resource_bytes =
            worker_resources.iter().try_fold(0_u64, |total, resource| {
                checked_add(total, resource.byte_length, "worker resource bytes")
            })?;
        Ok(Self {
            source,
            plan,
            pages: BTreeMap::new(),
            collections: BTreeMap::new(),
            diagnostics,
            resources: Vec::new(),
            worker_resources,
            body_seen: BTreeSet::new(),
            body_visiting: BTreeSet::new(),
            node_counter: 0,
            resource_counter: 0,
            decoded_resource_bytes,
        })
    }

    #[allow(clippy::too_many_lines)]
    fn map(mut self, root: &'a Value) -> Result<ImportDocument, ImportError> {
        let root = object_ref(root, "DoclingDocument root")?;
        if require_string(root, "schema_name", "DoclingDocument root")?
            != DOCLING_DOCUMENT_SCHEMA_NAME
        {
            return Err(schema_error(
                "DoclingDocument schema_name is not DoclingDocument",
            ));
        }
        if require_string(root, "version", "DoclingDocument root")?
            != DOCLING_DOCUMENT_SCHEMA_VERSION
        {
            return Err(schema_error(
                "DoclingDocument version is not the pinned 1.10.0 schema",
            ));
        }
        let name = require_string(root, "name", "DoclingDocument root")?;
        if name.trim().is_empty() {
            return Err(schema_error("DoclingDocument name must not be empty"));
        }
        ensure_allowed_root_keys(root)?;
        self.pages = parse_pages(root.get("pages"), &self.plan.limits)?;
        for collection in [
            "groups",
            "texts",
            "pictures",
            "tables",
            "key_value_items",
            "form_items",
            "field_regions",
            "field_items",
        ] {
            let values = optional_array(root, collection)?;
            validate_collection(collection, values)?;
            self.collections.insert(collection, values);
        }
        self.validate_all_refs(root)?;
        self.scan_unsupported()?;
        let body = root
            .get("body")
            .ok_or_else(|| schema_error("DoclingDocument body is missing"))?;
        validate_self_ref(body, "#/body", "body")?;
        let body = object_ref(body, "DoclingDocument body")?;
        let children = refs_array(body, "children", "DoclingDocument body")?;
        let mut blocks = Vec::new();
        for reference in children {
            self.resolve_body_ref(reference, &mut blocks)?;
        }
        if blocks.is_empty() {
            self.push_diagnostic(
                "schema_loss",
                DiagnosticSeverity::Warning,
                "DoclingDocument body contained no supported content",
                None,
                None,
            )?;
        }
        let mut title = name.to_owned();
        let mut content = Vec::new();
        for block in blocks {
            match block {
                MappedBlock::Heading {
                    level: 1,
                    title: heading_title,
                    ..
                } if content.is_empty() && title == name => title = heading_title,
                other => content.push(other),
            }
        }
        let mut cursor = 0_usize;
        let nodes = self.nest_sections(&content, &mut cursor, 0)?;
        if cursor != content.len() {
            return Err(schema_error(
                "DoclingDocument heading hierarchy could not be resolved",
            ));
        }
        for resource in std::mem::take(&mut self.worker_resources) {
            if self
                .resources
                .iter()
                .any(|existing| existing.locator == resource.locator)
            {
                return Err(schema_error(
                    "Docling data URI resource collides with a worker resource locator",
                ));
            }
            self.resource_counter = self.resource_counter.checked_add(1).ok_or_else(|| {
                ImportError::new(ImportErrorCode::LimitExceeded, "resource id overflowed")
            })?;
            self.resources.push(ImportResource {
                id: format!("docling-worker-resource-{}", self.resource_counter),
                locator: resource.locator,
                media_type: resource.media_type,
                byte_length: resource.byte_length,
                sha256: resource.sha256,
                bytes: resource.bytes,
                source_locations: Vec::new(),
                provenance: vec![provenance(self.source, Vec::new())],
            });
        }
        ImportDocument::create(
            format!("document-{}", &self.source.sha256.as_str()[..24]),
            self.source.sha256.clone(),
            title,
            nodes,
            self.resources,
            self.diagnostics,
            vec![provenance(self.source, Vec::new())],
        )
    }

    fn validate_all_refs(&self, root: &Map<String, Value>) -> Result<(), ImportError> {
        for (collection, values) in &self.collections {
            for (index, value) in values.iter().enumerate() {
                let context = format!("{collection}[{index}]");
                let item = object_ref(value, &context)?;
                let expected = format!("#/{collection}/{index}");
                if require_string(item, "self_ref", &context)? != expected {
                    return Err(schema_error(&format!(
                        "{context} self_ref is not its canonical array reference"
                    )));
                }
                let parent = item.get("parent").ok_or_else(|| {
                    schema_error(&format!("{context}.parent reference is missing"))
                })?;
                self.validate_ref(ref_string(parent, &format!("{context}.parent"))?)?;
                for field in ["children", "captions"] {
                    if let Some(refs) = item.get(field) {
                        for reference in ref_values(refs, &format!("{context}.{field}"))? {
                            self.validate_ref(reference)?;
                            self.validate_owned_parent(reference, &expected)?;
                        }
                    }
                }
                for field in ["references", "footnotes"] {
                    if let Some(refs) = item.get(field) {
                        for reference in ref_values(refs, &format!("{context}.{field}"))? {
                            self.validate_ref(reference)?;
                        }
                    }
                }
            }
        }
        for root_name in ["body", "furniture"] {
            if let Some(value) = root.get(root_name) {
                let item = object_ref(value, root_name)?;
                for reference in refs_array(item, "children", root_name)? {
                    self.validate_ref(reference)?;
                    self.validate_owned_parent(reference, &format!("#/{root_name}"))?;
                }
            }
        }
        Ok(())
    }

    fn validate_owned_parent(
        &self,
        reference: &str,
        expected_parent: &str,
    ) -> Result<(), ImportError> {
        let (collection, index) = parse_ref(reference)?;
        let child = object_ref(
            self.collection_value(collection, index)?,
            "Docling owned reference",
        )?;
        let parent = child
            .get("parent")
            .ok_or_else(|| schema_error("Docling owned reference has no parent"))?;
        if ref_string(parent, "Docling owned reference parent")? != expected_parent {
            return Err(schema_error(
                "Docling owned reference parent differs from its containing children/captions edge",
            ));
        }
        Ok(())
    }

    fn validate_ref(&self, reference: &str) -> Result<(), ImportError> {
        if matches!(reference, "#/body" | "#/furniture") {
            return Ok(());
        }
        let (collection, index) = parse_ref(reference)?;
        let Some(values) = self.collections.get(collection) else {
            return Err(schema_error(&format!(
                "DoclingDocument reference uses unknown collection: {reference}"
            )));
        };
        if values.get(index).is_none() {
            return Err(schema_error(&format!(
                "DoclingDocument reference is out of bounds: {reference}"
            )));
        }
        Ok(())
    }

    fn scan_unsupported(&mut self) -> Result<(), ImportError> {
        let form_count = [
            "key_value_items",
            "form_items",
            "field_regions",
            "field_items",
        ]
        .iter()
        .map(|name| self.collection(name).len())
        .sum::<usize>();
        if form_count > 0 {
            self.push_diagnostic(
                "form_unsupported",
                DiagnosticSeverity::Warning,
                "Docling form/key-value structures are retained only as source evidence and omitted from Weftext Import IR",
                None,
                None,
            )?;
        }
        let checkbox = self.collection("texts").iter().any(|item| {
            item.get("label")
                .and_then(Value::as_str)
                .is_some_and(|label| matches!(label, "checkbox_selected" | "checkbox_unselected"))
                || item
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(is_checkbox_text)
        });
        if checkbox {
            self.push_diagnostic(
                "checkbox_unsupported",
                DiagnosticSeverity::Warning,
                "Docling checkbox state has no dedicated provisional Import IR node; its bounded text is preserved",
                None,
                None,
            )?;
        }
        let furniture = self.collection("texts").iter().any(|item| {
            item.get("content_layer").and_then(Value::as_str) == Some("furniture")
                || item
                    .get("label")
                    .and_then(Value::as_str)
                    .is_some_and(|label| matches!(label, "page_header" | "page_footer"))
        });
        if furniture {
            self.push_diagnostic(
                "header_footer_unsupported",
                DiagnosticSeverity::Warning,
                "Docling page headers, footers, and furniture are omitted from body Import IR",
                None,
                None,
            )?;
        }
        Ok(())
    }

    fn resolve_body_ref(
        &mut self,
        reference: &str,
        output: &mut Vec<MappedBlock>,
    ) -> Result<(), ImportError> {
        self.validate_ref(reference)?;
        if usize_to_u64(self.body_visiting.len()) >= u64::from(self.plan.limits.max_ir_depth) {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "DoclingDocument body reference nesting exceeds the configured IR depth",
            ));
        }
        if !self.body_visiting.insert(reference.to_owned()) {
            return Err(schema_error(
                "DoclingDocument body contains a reference cycle",
            ));
        }
        if !self.body_seen.insert(reference.to_owned()) {
            return Err(schema_error(
                "DoclingDocument body contains a duplicate owned reference",
            ));
        }
        let (collection, index) = parse_ref(reference)?;
        match collection {
            "groups" => self.map_group(index, output)?,
            "texts" => self.map_text(index, output)?,
            "tables" => self.map_table(index, output)?,
            "pictures" => self.map_picture(index, output)?,
            "key_value_items" | "form_items" | "field_regions" | "field_items" => {}
            _ => {
                return Err(schema_error(&format!(
                    "unsupported body reference collection: {collection}"
                )));
            }
        }
        self.body_visiting.remove(reference);
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn map_group(
        &mut self,
        index: usize,
        output: &mut Vec<MappedBlock>,
    ) -> Result<(), ImportError> {
        let value = self.collection_value("groups", index)?.clone();
        let item = object_ref(&value, "Docling group")?;
        let label = require_string(item, "label", "Docling group")?;
        let children = refs_array(item, "children", "Docling group")?
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        if label != "list" {
            if !matches!(label, "section" | "inline" | "unspecified") {
                self.push_diagnostic(
                    "schema_loss",
                    DiagnosticSeverity::Warning,
                    &format!("Docling group label {label:?} was flattened into reading order"),
                    None,
                    None,
                )?;
            }
            for child in children {
                self.resolve_body_ref(&child, output)?;
            }
            return Ok(());
        }

        let mut pending_items = Vec::new();
        let mut pending_locations = Vec::new();
        let mut pending_ordered = None;
        for child in children {
            let (collection, child_index) = parse_ref(&child)?;
            let list_text = if collection == "texts" {
                let text = self.collection_value("texts", child_index)?.clone();
                let text_item = object_ref(&text, "Docling list item")?;
                (require_string(text_item, "label", "Docling list item")? == "list_item")
                    .then(|| {
                        let ordered = text_item
                            .get("enumerated")
                            .and_then(Value::as_bool)
                            .unwrap_or(false);
                        let value =
                            require_string(text_item, "text", "Docling list item")?.to_owned();
                        let locations = self.locations(text_item, Some(value.chars().count()))?;
                        Ok::<_, ImportError>((ordered, value, locations))
                    })
                    .transpose()?
            } else {
                None
            };
            if let Some((ordered, text, locations)) = list_text {
                if pending_ordered.is_some_and(|value| value != ordered) {
                    self.flush_list(
                        output,
                        pending_ordered.unwrap_or(false),
                        &mut pending_items,
                        &mut pending_locations,
                    )?;
                }
                pending_ordered = Some(ordered);
                pending_items.push(text);
                pending_locations.extend(locations);
                if !self.body_seen.insert(child.clone()) {
                    return Err(schema_error(
                        "DoclingDocument list contains a duplicate owned item reference",
                    ));
                }
                let child_value = self.collection_value("texts", child_index)?.clone();
                let child_item = object_ref(&child_value, "Docling list item")?;
                let nested = refs_array(child_item, "children", "Docling list item")?
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                if !nested.is_empty() {
                    self.flush_list(
                        output,
                        pending_ordered.unwrap_or(false),
                        &mut pending_items,
                        &mut pending_locations,
                    )?;
                    self.push_diagnostic(
                        "schema_loss",
                        DiagnosticSeverity::Warning,
                        "nested Docling list structure was flattened into sequential list blocks",
                        None,
                        None,
                    )?;
                    for nested_ref in nested {
                        self.resolve_body_ref(&nested_ref, output)?;
                    }
                    pending_ordered = None;
                }
            } else {
                if !pending_items.is_empty() {
                    self.flush_list(
                        output,
                        pending_ordered.unwrap_or(false),
                        &mut pending_items,
                        &mut pending_locations,
                    )?;
                }
                pending_ordered = None;
                self.resolve_body_ref(&child, output)?;
            }
        }
        if !pending_items.is_empty() {
            self.flush_list(
                output,
                pending_ordered.unwrap_or(false),
                &mut pending_items,
                &mut pending_locations,
            )?;
        }
        Ok(())
    }

    fn flush_list(
        &mut self,
        output: &mut Vec<MappedBlock>,
        ordered: bool,
        items: &mut Vec<String>,
        locations: &mut Vec<ImportSourceLocation>,
    ) -> Result<(), ImportError> {
        if items.is_empty() {
            return Ok(());
        }
        let source_locations = std::mem::take(locations);
        let node = self.node(
            ImportNodeKind::List {
                ordered,
                items: std::mem::take(items),
            },
            source_locations,
            9_500,
        )?;
        output.push(MappedBlock::Node(node));
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn map_text(&mut self, index: usize, output: &mut Vec<MappedBlock>) -> Result<(), ImportError> {
        let value = self.collection_value("texts", index)?.clone();
        let item = object_ref(&value, "Docling text item")?;
        let label = require_string(item, "label", "Docling text item")?.to_owned();
        let text = require_string(item, "text", "Docling text item")?.to_owned();
        let locations = self.locations(item, Some(text.chars().count()))?;
        if item.get("content_layer").and_then(Value::as_str) == Some("furniture")
            || matches!(label.as_str(), "page_header" | "page_footer")
        {
            return Ok(());
        }
        match label.as_str() {
            "title" => output.push(MappedBlock::Heading {
                level: 1,
                title: text,
                source_locations: locations,
            }),
            "section_header" => {
                let raw_level = item.get("level").and_then(Value::as_u64).ok_or_else(|| {
                    schema_error("Docling section_header is missing its integer level")
                })?;
                let level = u8::try_from(raw_level.saturating_add(1)).map_err(|_| {
                    schema_error("Docling section_header level exceeds supported IR depth")
                })?;
                if !(1..=8).contains(&level) {
                    return Err(schema_error(
                        "Docling section_header level is outside 0 through 7",
                    ));
                }
                output.push(MappedBlock::Heading {
                    level,
                    title: text,
                    source_locations: locations,
                });
            }
            "list_item" => {
                let ordered = item
                    .get("enumerated")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let node = self.node(
                    ImportNodeKind::List {
                        ordered,
                        items: vec![text],
                    },
                    locations,
                    9_500,
                )?;
                output.push(MappedBlock::Node(node));
            }
            "formula" => {
                let source = if text.trim().is_empty() {
                    "formula not decoded".to_owned()
                } else {
                    text
                };
                let node = self.node(
                    ImportNodeKind::Formula {
                        notation: "latex".to_owned(),
                        source,
                    },
                    locations,
                    9_000,
                )?;
                output.push(MappedBlock::Node(node));
            }
            "checkbox_selected" | "checkbox_unselected" => {
                let node = self.node(ImportNodeKind::Paragraph { text }, locations, 8_500)?;
                output.push(MappedBlock::Node(node));
            }
            "paragraph" | "text" if text.trim() == FORMULA_PLACEHOLDER => {
                let node = self.node(
                    ImportNodeKind::Formula {
                        notation: "docling-placeholder".to_owned(),
                        source: "formula not decoded".to_owned(),
                    },
                    locations.clone(),
                    0,
                )?;
                self.push_diagnostic(
                    "formula_placeholder",
                    DiagnosticSeverity::Warning,
                    "docling.rs Lite detected a formula region but CodeFormula/TableFormer-free Lite did not decode it",
                    locations.first().cloned(),
                    Some(node.id.clone()),
                )?;
                output.push(MappedBlock::Node(node));
            }
            "paragraph" | "text" | "caption" | "footnote" => {
                let node = self.node(ImportNodeKind::Paragraph { text }, locations, 9_500)?;
                output.push(MappedBlock::Node(node));
            }
            "code" => {
                let node =
                    self.node(ImportNodeKind::Paragraph { text }, locations.clone(), 8_500)?;
                self.push_diagnostic(
                    "schema_loss",
                    DiagnosticSeverity::Warning,
                    "Docling code block formatting was flattened to a paragraph by provisional Import IR",
                    locations.first().cloned(),
                    Some(node.id.clone()),
                )?;
                output.push(MappedBlock::Node(node));
            }
            _ => {
                let node =
                    self.node(ImportNodeKind::Paragraph { text }, locations.clone(), 7_500)?;
                self.push_diagnostic(
                    "schema_loss",
                    DiagnosticSeverity::Warning,
                    &format!(
                        "Docling text label {label:?} has no exact provisional Import IR representation; text was preserved"
                    ),
                    locations.first().cloned(),
                    Some(node.id.clone()),
                )?;
                output.push(MappedBlock::Node(node));
            }
        }
        Ok(())
    }

    fn map_table(
        &mut self,
        index: usize,
        output: &mut Vec<MappedBlock>,
    ) -> Result<(), ImportError> {
        let value = self.collection_value("tables", index)?.clone();
        let item = object_ref(&value, "Docling table")?;
        let locations = self.locations(item, Some(0))?;
        let data = object_ref(
            item.get("data")
                .ok_or_else(|| schema_error("Docling table data is missing"))?,
            "Docling table data",
        )?;
        let num_rows = usize_from_u64(
            require_u64(data, "num_rows", "Docling table data")?,
            "Docling table row count",
        )?;
        let num_cols = usize_from_u64(
            require_u64(data, "num_cols", "Docling table data")?,
            "Docling table column count",
        )?;
        if num_rows == 0 || num_cols == 0 {
            return Err(schema_error("Docling tables must have non-zero dimensions"));
        }
        let cell_total = num_rows.checked_mul(num_cols).ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling table dimensions overflowed",
            )
        })?;
        self.plan.limits.check(
            "Docling table cells",
            usize_to_u64(cell_total),
            u64::from(self.plan.limits.max_ir_nodes),
        )?;
        let grid = data
            .get("grid")
            .and_then(Value::as_array)
            .ok_or_else(|| schema_error("Docling table grid is missing or not an array"))?;
        if grid.len() != num_rows {
            return Err(schema_error(
                "Docling table grid row count differs from num_rows",
            ));
        }
        let mut rows = Vec::with_capacity(num_rows);
        let mut header_rows = 0_u16;
        let mut still_headers = true;
        let mut spans_lost = false;
        for (row_index, row) in grid.iter().enumerate() {
            let cells = row.as_array().ok_or_else(|| {
                schema_error(&format!(
                    "Docling table grid row {row_index} is not an array"
                ))
            })?;
            if cells.len() != num_cols {
                return Err(schema_error(
                    "Docling table grid width differs from num_cols",
                ));
            }
            let mut mapped_row = Vec::with_capacity(num_cols);
            let mut row_is_header = true;
            for cell in cells {
                let cell = object_ref(cell, "Docling table cell")?;
                mapped_row.push(require_string(cell, "text", "Docling table cell")?.to_owned());
                row_is_header &= cell
                    .get("column_header")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                spans_lost |= cell.get("row_span").and_then(Value::as_u64).unwrap_or(1) > 1
                    || cell.get("col_span").and_then(Value::as_u64).unwrap_or(1) > 1;
            }
            if still_headers && row_is_header {
                header_rows = header_rows
                    .checked_add(1)
                    .ok_or_else(|| schema_error("Docling table header row count exceeds u16"))?;
            } else {
                still_headers = false;
            }
            rows.push(mapped_row);
        }
        let node = self.node(
            ImportNodeKind::Table { header_rows, rows },
            locations.clone(),
            8_500,
        )?;
        self.push_diagnostic(
            "table_fallback",
            DiagnosticSeverity::Warning,
            "docling.rs Lite excludes TableFormer; this table uses Docling's geometric grid reconstruction",
            locations.first().cloned(),
            Some(node.id.clone()),
        )?;
        if spans_lost {
            self.push_diagnostic(
                "schema_loss",
                DiagnosticSeverity::Warning,
                "merged Docling table cell spans were flattened into the rectangular provisional Import IR grid",
                locations.first().cloned(),
                Some(node.id.clone()),
            )?;
        }
        output.push(MappedBlock::Node(node));
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn map_picture(
        &mut self,
        index: usize,
        output: &mut Vec<MappedBlock>,
    ) -> Result<(), ImportError> {
        let value = self.collection_value("pictures", index)?.clone();
        let item = object_ref(&value, "Docling picture")?;
        let locations = self.locations(item, Some(0))?;
        let caption = self.picture_caption(item)?;
        let Some(image) = item.get("image") else {
            let node = self.node(
                ImportNodeKind::Paragraph {
                    text: caption.clone().unwrap_or_else(|| {
                        format!("[Picture {} has no extractable image]", index + 1)
                    }),
                },
                locations.clone(),
                5_000,
            )?;
            self.push_diagnostic(
                "schema_loss",
                DiagnosticSeverity::Warning,
                "Docling picture had no embedded image bytes; a textual placeholder was preserved",
                locations.first().cloned(),
                Some(node.id.clone()),
            )?;
            output.push(MappedBlock::Node(node));
            return Ok(());
        };
        let image = object_ref(image, "Docling picture image")?;
        let media_type = require_string(image, "mimetype", "Docling picture image")?.to_owned();
        validate_media_type(&media_type)?;
        let uri = require_string(image, "uri", "Docling picture image")?;
        let bytes = decode_data_uri(uri, &media_type, self.plan.limits.max_resource_bytes)?;
        let byte_length = usize_to_u64(bytes.len());
        self.decoded_resource_bytes = checked_add(
            self.decoded_resource_bytes,
            byte_length,
            "decoded Docling picture bytes",
        )?;
        self.plan.limits.check(
            "decoded Docling picture total bytes",
            self.decoded_resource_bytes,
            self.plan.limits.max_total_output_bytes,
        )?;
        self.plan.limits.check(
            "decoded Docling picture count",
            usize_to_u64(
                self.resources
                    .len()
                    .saturating_add(self.worker_resources.len())
                    .saturating_add(1),
            ),
            u64::from(self.plan.limits.max_resource_count),
        )?;
        self.resource_counter = self.resource_counter.checked_add(1).ok_or_else(|| {
            ImportError::new(ImportErrorCode::LimitExceeded, "resource id overflowed")
        })?;
        let resource_id = format!("docling-picture-{}", self.resource_counter);
        let extension = media_extension(&media_type);
        if extension == "bin" {
            self.push_diagnostic(
                "schema_loss",
                DiagnosticSeverity::Warning,
                &format!("Docling picture media type {media_type:?} uses a generic .bin locator"),
                locations.first().cloned(),
                None,
            )?;
        }
        let locator = PortablePath::parse(format!(
            "resources/docling-picture-{}.{}",
            self.resource_counter, extension
        ))?;
        let sha256 = sha256_bytes(&bytes);
        // Materialize the exact image in the generic worker-resource shape
        // before lifting it into Weftext ImportResource authority.
        let worker_resource = WorkerResource {
            locator: locator.clone(),
            media_type: media_type.clone(),
            byte_length,
            sha256: sha256.clone(),
            bytes: bytes.clone(),
        };
        if worker_resource.byte_length != usize_to_u64(worker_resource.bytes.len())
            || worker_resource.sha256 != sha256_bytes(&worker_resource.bytes)
        {
            return Err(schema_error(
                "decoded Docling picture failed WorkerResource integrity",
            ));
        }
        self.resources.push(ImportResource {
            id: resource_id.clone(),
            locator,
            media_type,
            byte_length,
            sha256,
            bytes,
            source_locations: locations.clone(),
            provenance: vec![provenance(self.source, locations.clone())],
        });
        let node = self.node(
            ImportNodeKind::Figure {
                resource_id,
                alt: caption
                    .clone()
                    .unwrap_or_else(|| format!("Imported picture {}", index + 1)),
                caption,
            },
            locations,
            9_000,
        )?;
        output.push(MappedBlock::Node(node));
        Ok(())
    }

    fn picture_caption(&self, item: &Map<String, Value>) -> Result<Option<String>, ImportError> {
        let Some(captions) = item.get("captions") else {
            return Ok(None);
        };
        let refs = ref_values(captions, "Docling picture captions")?;
        let mut values = Vec::new();
        for reference in refs {
            let (collection, index) = parse_ref(reference)?;
            if collection != "texts" {
                return Err(schema_error(
                    "Docling picture caption must reference a text item",
                ));
            }
            let caption = object_ref(
                self.collection_value("texts", index)?,
                "Docling picture caption",
            )?;
            values.push(require_string(caption, "text", "Docling picture caption")?);
        }
        if values.is_empty() {
            Ok(None)
        } else {
            Ok(Some(values.join("\n")))
        }
    }

    fn locations(
        &self,
        item: &Map<String, Value>,
        max_chars: Option<usize>,
    ) -> Result<Vec<ImportSourceLocation>, ImportError> {
        let Some(prov) = item.get("prov") else {
            return Ok(Vec::new());
        };
        let values = prov
            .as_array()
            .ok_or_else(|| schema_error("Docling item prov must be an array"))?;
        if values.len() > 1_000 {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling item provenance count exceeds 1,000",
            ));
        }
        values
            .iter()
            .map(|value| parse_location(value, &self.pages, self.source, max_chars))
            .collect()
    }

    fn node(
        &mut self,
        kind: ImportNodeKind,
        source_locations: Vec<ImportSourceLocation>,
        confidence: u16,
    ) -> Result<ImportNode, ImportError> {
        self.node_counter = self.node_counter.checked_add(1).ok_or_else(|| {
            ImportError::new(ImportErrorCode::LimitExceeded, "IR node id overflowed")
        })?;
        self.plan.limits.check(
            "mapped Docling IR nodes",
            u64::from(self.node_counter),
            u64::from(self.plan.limits.max_ir_nodes),
        )?;
        Ok(ImportNode {
            id: format!("docling-node-{}", self.node_counter),
            kind,
            confidence: Confidence::from_basis_points(confidence)?,
            source_locations: source_locations.clone(),
            provenance: vec![provenance(self.source, source_locations)],
        })
    }

    fn nest_sections(
        &mut self,
        blocks: &[MappedBlock],
        cursor: &mut usize,
        parent_level: u8,
    ) -> Result<Vec<ImportNode>, ImportError> {
        let mut nodes = Vec::new();
        while let Some(block) = blocks.get(*cursor) {
            match block {
                MappedBlock::Heading { level, .. } if *level <= parent_level => break,
                MappedBlock::Heading {
                    level,
                    title,
                    source_locations,
                } => {
                    let level = *level;
                    let title = title.clone();
                    let source_locations = source_locations.clone();
                    *cursor += 1;
                    if level > parent_level.saturating_add(1) && parent_level > 0 {
                        self.push_diagnostic(
                            "schema_loss",
                            DiagnosticSeverity::Warning,
                            "Docling heading hierarchy skipped a level; the explicit level was preserved",
                            source_locations.first().cloned(),
                            None,
                        )?;
                    }
                    let children = self.nest_sections(blocks, cursor, level)?;
                    nodes.push(self.node(
                        ImportNodeKind::Section {
                            level,
                            title,
                            children,
                        },
                        source_locations,
                        9_500,
                    )?);
                }
                MappedBlock::Node(node) => {
                    nodes.push(node.clone());
                    *cursor += 1;
                }
            }
        }
        Ok(nodes)
    }

    fn collection(&self, name: &str) -> &[Value] {
        self.collections.get(name).copied().unwrap_or(&[])
    }

    fn collection_value(&self, name: &str, index: usize) -> Result<&Value, ImportError> {
        self.collection(name).get(index).ok_or_else(|| {
            schema_error(&format!(
                "DoclingDocument reference #/{name}/{index} is missing"
            ))
        })
    }

    fn push_diagnostic(
        &mut self,
        code: &str,
        severity: DiagnosticSeverity,
        message: &str,
        source_location: Option<ImportSourceLocation>,
        ir_node_id: Option<String>,
    ) -> Result<(), ImportError> {
        let next = self.diagnostics.len().checked_add(1).ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::LimitExceeded,
                "diagnostic count overflowed",
            )
        })?;
        self.plan.limits.check(
            "docling.rs Lite diagnostic count",
            usize_to_u64(next),
            u64::from(self.plan.limits.max_diagnostics),
        )?;
        self.diagnostics.push(ImportDiagnostic {
            code: code.to_owned(),
            severity,
            message: message.to_owned(),
            source_location,
            ir_node_id,
        });
        Ok(())
    }
}

fn parse_pages(
    value: Option<&Value>,
    limits: &ImportLimits,
) -> Result<BTreeMap<u32, PageSize>, ImportError> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    let pages = value
        .as_object()
        .ok_or_else(|| schema_error("DoclingDocument pages must be an object"))?;
    limits.check(
        "DoclingDocument page count",
        usize_to_u64(pages.len()),
        u64::from(limits.max_pages),
    )?;
    let mut output = BTreeMap::new();
    for (key, value) in pages {
        let page = key.parse::<u32>().map_err(|_| {
            schema_error("DoclingDocument page keys must be canonical positive integers")
        })?;
        if page == 0 || page.to_string() != *key || page > limits.max_pages {
            return Err(schema_error(
                "DoclingDocument page key is outside the configured one-based range",
            ));
        }
        let item = object_ref(value, "DoclingDocument page")?;
        if require_u64(item, "page_no", "DoclingDocument page")? != u64::from(page) {
            return Err(schema_error(
                "DoclingDocument page_no differs from its pages map key",
            ));
        }
        let size = object_ref(
            item.get("size")
                .ok_or_else(|| schema_error("DoclingDocument page size is missing"))?,
            "DoclingDocument page size",
        )?;
        let width = require_f64(size, "width", "DoclingDocument page size")?;
        let height = require_f64(size, "height", "DoclingDocument page size")?;
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return Err(schema_error(
                "DoclingDocument page dimensions must be finite and positive",
            ));
        }
        output.insert(page, PageSize { width, height });
    }
    Ok(output)
}

fn parse_location(
    value: &Value,
    pages: &BTreeMap<u32, PageSize>,
    source: &SourceArtifact,
    max_chars: Option<usize>,
) -> Result<ImportSourceLocation, ImportError> {
    let prov = object_ref(value, "Docling provenance")?;
    let page = u32::try_from(require_u64(prov, "page_no", "Docling provenance")?)
        .map_err(|_| schema_error("Docling provenance page_no exceeds u32"))?;
    let size = pages.get(&page).ok_or_else(|| {
        schema_error("Docling provenance page_no does not exist in the pages map")
    })?;
    let bbox = object_ref(
        prov.get("bbox")
            .ok_or_else(|| schema_error("Docling provenance bbox is missing"))?,
        "Docling bbox",
    )?;
    let left = require_f64(bbox, "l", "Docling bbox")?;
    let top = require_f64(bbox, "t", "Docling bbox")?;
    let right = require_f64(bbox, "r", "Docling bbox")?;
    let bottom = require_f64(bbox, "b", "Docling bbox")?;
    let origin = require_string(bbox, "coord_origin", "Docling bbox")?;
    if [left, top, right, bottom]
        .iter()
        .any(|coordinate| !coordinate.is_finite())
        || left < 0.0
        || right < left
        || right > size.width
    {
        return Err(schema_error(
            "Docling bbox horizontal coordinates are non-finite, inverted, or outside the page",
        ));
    }
    let (top_from_top, bottom_from_top) = match origin {
        "BOTTOMLEFT" => (size.height - top, size.height - bottom),
        "TOPLEFT" => (top, bottom),
        _ => {
            return Err(schema_error(
                "Docling bbox coord_origin must be BOTTOMLEFT or TOPLEFT",
            ));
        }
    };
    if top_from_top < 0.0 || bottom_from_top < top_from_top || bottom_from_top > size.height {
        return Err(schema_error(
            "Docling bbox vertical coordinates are inverted or outside the page",
        ));
    }
    if let Some(charspan) = prov.get("charspan") {
        let values = charspan
            .as_array()
            .filter(|values| values.len() == 2)
            .ok_or_else(|| schema_error("Docling provenance charspan must contain two integers"))?;
        let start = values[0]
            .as_u64()
            .ok_or_else(|| schema_error("Docling provenance charspan start is not an integer"))?;
        let end = values[1]
            .as_u64()
            .ok_or_else(|| schema_error("Docling provenance charspan end is not an integer"))?;
        if start > end || max_chars.is_some_and(|maximum| end > usize_to_u64(maximum)) {
            return Err(schema_error(
                "Docling provenance charspan is inverted or exceeds its item text",
            ));
        }
    }
    let x0 = normalized_millionths(left, size.width)?;
    let x1 = normalized_millionths(right, size.width)?;
    let y0 = normalized_millionths(top_from_top, size.height)?;
    let y1 = normalized_millionths(bottom_from_top, size.height)?;
    Ok(ImportSourceLocation {
        source_digest: source.sha256.clone(),
        page: Some(page),
        region: Some(BoundingRegion {
            x_millionths: x0,
            y_millionths: y0,
            width_millionths: x1.saturating_sub(x0),
            height_millionths: y1.saturating_sub(y0),
        }),
        byte_start: None,
        byte_end: None,
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn normalized_millionths(value: f64, maximum: f64) -> Result<u32, ImportError> {
    let normalized = (value / maximum * 1_000_000.0).round();
    if !(0.0..=1_000_000.0).contains(&normalized) {
        return Err(schema_error(
            "Docling bbox could not be normalized inside its page",
        ));
    }
    Ok(normalized as u32)
}

fn decode_data_uri(uri: &str, media_type: &str, maximum: u64) -> Result<Vec<u8>, ImportError> {
    let prefix = format!("data:{media_type};base64,");
    let encoded = uri.strip_prefix(&prefix).ok_or_else(|| {
        schema_error("Docling picture URI must be a base64 data URI matching its mimetype")
    })?;
    if encoded.is_empty() || encoded.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(schema_error(
            "Docling picture data URI must contain canonical unwrapped base64",
        ));
    }
    let maximum_encoded = maximum
        .checked_add(2)
        .and_then(|value| value.checked_div(3))
        .and_then(|value| value.checked_mul(4))
        .ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling picture encoded-size limit overflowed",
            )
        })?;
    if usize_to_u64(encoded.len()) > maximum_encoded || encoded.len() % 4 != 0 {
        return Err(ImportError::new(
            ImportErrorCode::LimitExceeded,
            "Docling picture data URI exceeds its decoded byte limit or has invalid base64 length",
        ));
    }
    let mut output = Vec::with_capacity((encoded.len() / 4).saturating_mul(3));
    for (chunk_index, chunk) in encoded.as_bytes().as_chunks::<4>().0.iter().enumerate() {
        let last = chunk_index + 1 == encoded.len() / 4;
        let a = base64_value(chunk[0])?;
        let b = base64_value(chunk[1])?;
        let c_pad = chunk[2] == b'=';
        let d_pad = chunk[3] == b'=';
        if (!last && d_pad) || (c_pad && !d_pad) {
            return Err(schema_error(
                "Docling picture data URI has invalid base64 padding",
            ));
        }
        let c = if c_pad { 0 } else { base64_value(chunk[2])? };
        let d = if d_pad { 0 } else { base64_value(chunk[3])? };
        if (c_pad && b & 0x0f != 0) || (d_pad && !c_pad && c & 0x03 != 0) {
            return Err(schema_error(
                "Docling picture data URI has non-canonical base64 tail bits",
            ));
        }
        output.push((a << 2) | (b >> 4));
        if !c_pad {
            output.push((b << 4) | (c >> 2));
        }
        if !d_pad {
            output.push((c << 6) | d);
        }
        if usize_to_u64(output.len()) > maximum {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling picture data URI decoded beyond its byte limit",
            ));
        }
    }
    Ok(output)
}

fn base64_value(value: u8) -> Result<u8, ImportError> {
    match value {
        b'A'..=b'Z' => Ok(value - b'A'),
        b'a'..=b'z' => Ok(value - b'a' + 26),
        b'0'..=b'9' => Ok(value - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err(schema_error(
            "Docling picture data URI contains a non-base64 character",
        )),
    }
}

fn parse_ref(reference: &str) -> Result<(&str, usize), ImportError> {
    let tail = reference
        .strip_prefix("#/")
        .ok_or_else(|| schema_error("Docling references must be local #/ JSON pointers"))?;
    let (collection, index) = tail.split_once('/').ok_or_else(|| {
        schema_error("Docling references must contain one collection and one index")
    })?;
    if collection.is_empty() || index.is_empty() || index.contains('/') {
        return Err(schema_error(
            "Docling reference has an invalid pointer shape",
        ));
    }
    let parsed = index
        .parse::<usize>()
        .map_err(|_| schema_error("Docling reference index is not an integer"))?;
    if parsed.to_string() != index {
        return Err(schema_error(
            "Docling reference index is not in canonical decimal form",
        ));
    }
    Ok((collection, parsed))
}

fn validate_collection(name: &str, values: &[Value]) -> Result<(), ImportError> {
    for (index, value) in values.iter().enumerate() {
        if !value.is_object() {
            return Err(schema_error(&format!(
                "DoclingDocument {name}[{index}] is not an object"
            )));
        }
    }
    Ok(())
}

fn ensure_allowed_root_keys(root: &Map<String, Value>) -> Result<(), ImportError> {
    const ALLOWED: [&str; 16] = [
        "schema_name",
        "version",
        "name",
        "origin",
        "furniture",
        "body",
        "groups",
        "texts",
        "pictures",
        "tables",
        "key_value_items",
        "form_items",
        "field_regions",
        "field_items",
        "pages",
        "annotations",
    ];
    if let Some(unknown) = root.keys().find(|key| !ALLOWED.contains(&key.as_str())) {
        return Err(schema_error(&format!(
            "DoclingDocument root contains an unknown 1.10.0 field: {unknown}"
        )));
    }
    Ok(())
}

fn optional_array<'a>(
    root: &'a Map<String, Value>,
    name: &str,
) -> Result<&'a [Value], ImportError> {
    root.get(name).map_or(Ok(&[]), |value| {
        value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| schema_error(&format!("DoclingDocument {name} must be an array")))
    })
}

fn validate_self_ref(value: &Value, expected: &str, context: &str) -> Result<(), ImportError> {
    let item = object_ref(value, context)?;
    if require_string(item, "self_ref", context)? != expected {
        return Err(schema_error(&format!(
            "{context} self_ref must be {expected}"
        )));
    }
    Ok(())
}

fn refs_array<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<Vec<&'a str>, ImportError> {
    let Some(value) = object.get(field) else {
        return Ok(Vec::new());
    };
    ref_values(value, &format!("{context}.{field}"))
}

fn ref_values<'a>(value: &'a Value, context: &str) -> Result<Vec<&'a str>, ImportError> {
    value
        .as_array()
        .ok_or_else(|| schema_error(&format!("{context} must be an array")))?
        .iter()
        .map(|value| ref_string(value, context))
        .collect()
}

fn ref_string<'a>(value: &'a Value, context: &str) -> Result<&'a str, ImportError> {
    let object = object_ref(value, context)?;
    if object.len() != 1 {
        return Err(schema_error(&format!(
            "{context} reference objects may contain only $ref"
        )));
    }
    require_string(object, "$ref", context)
}

fn object_ref<'a>(value: &'a Value, context: &str) -> Result<&'a Map<String, Value>, ImportError> {
    value
        .as_object()
        .ok_or_else(|| schema_error(&format!("{context} must be an object")))
}

fn require_string<'a>(
    object: &'a Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<&'a str, ImportError> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| schema_error(&format!("{context}.{field} must be a string")))
}

fn require_u64(
    object: &Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<u64, ImportError> {
    object
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| schema_error(&format!("{context}.{field} must be a non-negative integer")))
}

fn require_f64(
    object: &Map<String, Value>,
    field: &str,
    context: &str,
) -> Result<f64, ImportError> {
    object
        .get(field)
        .and_then(Value::as_f64)
        .ok_or_else(|| schema_error(&format!("{context}.{field} must be a number")))
}

fn schema_error(message: &str) -> ImportError {
    ImportError::new(ImportErrorCode::WorkerProtocol, message)
}

fn validate_frozen_plan(plan: &ImportPlan) -> Result<(), ImportError> {
    if plan.contract_version != IMPORT_PLAN_CONTRACT_VERSION {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "docling.rs Lite plan uses an unknown contract version",
        ));
    }
    plan.limits.validate()?;
    let request = PlanRequest {
        destination: plan.destination.clone(),
        split_policy: plan.split_policy.clone(),
        resource_policy: plan.resource_policy,
        local_ocr_policy: plan.local_ocr_policy,
        agent_enhancement: plan.agent_enhancement.clone(),
        egress: plan.egress.clone(),
    };
    let material = serde_json::to_vec(&(
        &plan.source_digest,
        &plan.probe_digest,
        &plan.proposed_root_id,
        &plan.route,
        &request,
        &plan.limits,
    ))
    .map_err(|error| ImportError::serialization(&error))?;
    let expected = format!("plan-{}", &sha256_bytes(&material).as_str()[..24]);
    if plan.plan_id != expected {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "docling.rs Lite plan id differs from its frozen plan material",
        ));
    }
    Ok(())
}

fn validate_local_worker_plan(plan: &ImportPlan) -> Result<(), ImportError> {
    if !matches!(&plan.split_policy, SplitPolicy::SingleNode)
        || plan.resource_policy != ResourcePolicy::ExtractReferenced
        || !matches!(&plan.agent_enhancement, AgentEnhancementPolicy::Disabled)
        || !matches!(&plan.egress, EgressDisclosure::None)
        || plan.local_ocr_policy == LocalOcrPolicy::Never
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidContract,
            "docling.rs Lite worker plans must use the reviewed offline single-node local phase",
        ));
    }
    Ok(())
}

fn request_id(plan: &ImportPlan) -> Result<String, ImportError> {
    plan.plan_id
        .strip_prefix("plan-")
        .filter(|suffix| !suffix.is_empty())
        .map(|suffix| format!("request-{suffix}"))
        .ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::InvalidContract,
                "docling.rs Lite plan id does not use the required plan- prefix",
            )
        })
}

fn looks_like_docling_document(value: &Value) -> bool {
    value.get("schema_name").is_some() || value.get("version").is_some()
}

fn provenance(
    source: &SourceArtifact,
    source_locations: Vec<ImportSourceLocation>,
) -> ProvenanceRecord {
    ProvenanceRecord {
        kind: ProvenanceKind::LocalExtraction,
        component_id: "docling-rs".to_owned(),
        component_version: "0.52.2".to_owned(),
        input_digests: vec![source.sha256.clone()],
        output_digest: None,
        source_locations,
    }
}

fn validate_media_type(value: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 127
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
        || !value.starts_with("image/")
    {
        return Err(schema_error(
            "Docling picture mimetype must be a bounded image/* media type",
        ));
    }
    Ok(())
}

fn media_extension(media_type: &str) -> &'static str {
    match media_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/tiff" => "tiff",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "bin",
    }
}

fn is_checkbox_text(value: &str) -> bool {
    value.starts_with("- [x] ") || value.starts_with("- [X] ") || value.starts_with("- [ ] ")
}

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn usize_from_u64(value: u64, label: &str) -> Result<usize, ImportError> {
    usize::try_from(value).map_err(|_| {
        ImportError::new(
            ImportErrorCode::LimitExceeded,
            format!("{label} exceeds this platform's addressable range"),
        )
    })
}

fn checked_add(left: u64, right: u64, label: &str) -> Result<u64, ImportError> {
    left.checked_add(right).ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::LimitExceeded,
            format!("docling.rs Lite {label} overflowed"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{
        DOCLING_DOCUMENT_SCHEMA_NAME, DOCLING_DOCUMENT_SCHEMA_VERSION,
        DOCLING_LITE_ASSET_LOCK_VERSION, DOCLING_LITE_WORKER_PROTOCOL_VERSION,
        DOCLING_RELEASE_COMMIT, DOCLING_RELEASE_TAG, DoclingLiteArtifactRole, DoclingLiteAssetLock,
        DoclingLiteAssetPin, DoclingLiteInstalledArtifact, DoclingLitePdfAdapter,
        DoclingLiteWorkerCommand, DoclingLiteWorkerResponse, DoclingLiteWorkerStatus,
        DoclingModelPin,
    };
    use crate::{
        AsciiDocV1ProposalValidator, CanonicalProposalValidator, ComponentVersion, ImportAdapter,
        ImportErrorCode, ImportLimits, ImportNodeKind, LocalOcrPolicy, OriginClass, PlanRequest,
        PortablePath, SourceArtifact, SourceFormat, WorkerNetworkPolicy, WorkerResource,
        WorkerResponse, sha256_bytes,
    };

    #[test]
    fn default_adapter_is_honestly_unavailable_but_mapper_contract_is_present() {
        let limits = ImportLimits::default();
        let bytes = test_pdf("", "");
        let source =
            SourceArtifact::from_bytes("renamed.bin", OriginClass::TestFixture, &bytes, &limits)
                .expect("source artifact");
        let adapter = DoclingLitePdfAdapter::default();
        let probe =
            crate::probe_source_bytes(&adapter, &source, &bytes, &limits).expect("bounded probe");

        assert_eq!(probe.detected_format, SourceFormat::Pdf);
        assert!(!probe.safe_to_plan);
        assert!(!adapter.capability().available);
        assert!(!adapter.capability().ambient_network_allowed);
        assert_eq!(
            adapter.mapping_contract().worker_protocol_version,
            DOCLING_LITE_WORKER_PROTOCOL_VERSION
        );
    }

    #[test]
    fn verified_installation_enables_probe_and_freezes_download_free_command() {
        let (adapter, _) = verified_adapter();
        let (source, probe, plan) = planned(&adapter);
        assert!(probe.safe_to_plan);
        let request = adapter
            .worker_request(
                &source,
                &plan,
                PortablePath::parse("input/source.pdf").expect("input path"),
            )
            .expect("worker request");
        let command: DoclingLiteWorkerCommand =
            serde_json::from_value(request.format_options.clone()).expect("typed command");
        command.validate_boundary().expect("valid command");
        let json = serde_json::to_string(&request.format_options).expect("serialize command");
        assert!(json.contains("\"network\":\"denied\""));
        assert!(json.contains("\"layoutPrecision\":\"int8\""));
        assert!(json.contains("\"ocrLanguage\":\"en\""));
        assert!(!json.contains("http://"));
        assert!(!json.contains("https://"));

        let error = adapter
            .worker_request(
                &source,
                &plan,
                PortablePath::parse("input/alternate.pdf").expect("alternate input path"),
            )
            .expect_err("alternate worker input locator");
        assert_eq!(error.code(), ImportErrorCode::InvalidContract);

        let mut request_json = serde_json::to_value(request).expect("request JSON");
        request_json
            .as_object_mut()
            .expect("request object")
            .insert("futureAuthority".to_owned(), serde_json::Value::Bool(true));
        assert!(
            serde_json::from_value::<crate::WorkerRequest>(request_json).is_err(),
            "the generic request envelope must not discard unknown authority"
        );
    }

    #[test]
    fn worker_request_rejects_a_plan_mutated_after_review() {
        let (adapter, _) = verified_adapter();
        let (source, _, mut plan) = planned(&adapter);
        plan.limits.max_pages -= 1;
        let error = adapter
            .worker_request(
                &source,
                &plan,
                PortablePath::parse("input/source.pdf").expect("input path"),
            )
            .expect_err("mutated plan");
        assert_eq!(error.code(), ImportErrorCode::InvalidContract);
    }

    #[test]
    fn no_ocr_policy_cannot_become_a_final_docling_plan_or_command() {
        let (adapter, _) = verified_adapter();
        let limits = ImportLimits::default();
        let bytes = test_pdf("", "");
        let source =
            SourceArtifact::from_bytes("fixture.pdf", OriginClass::TestFixture, &bytes, &limits)
                .expect("source");
        let probe = crate::probe_source_bytes(&adapter, &source, &bytes, &limits).expect("probe");
        let mut request =
            PlanRequest::single_node(PortablePath::parse("Imported").expect("destination"));
        request.local_ocr_policy = LocalOcrPolicy::Never;
        let error = adapter
            .plan(&source, &probe, request, limits)
            .expect_err("no-OCR final route");
        assert_eq!(error.code(), ImportErrorCode::ProbeRejected);

        let (source, _, plan) = planned(&adapter);
        let generic = adapter
            .worker_request(
                &source,
                &plan,
                PortablePath::parse("input/source.pdf").expect("input path"),
            )
            .expect("worker request");
        let mut command: DoclingLiteWorkerCommand =
            serde_json::from_value(generic.format_options).expect("typed command");
        command.local_ocr_policy = LocalOcrPolicy::Never;
        let error = command
            .validate_boundary()
            .expect_err("no-OCR worker command");
        assert_eq!(error.code(), ImportErrorCode::WorkerProtocol);
    }

    #[test]
    fn encrypted_and_active_pdf_evidence_never_reaches_a_worker_plan() {
        let (adapter, _) = verified_adapter();
        let limits = ImportLimits::default();
        for (label, bytes, expected_code) in [
            (
                "encrypted",
                test_pdf("", "/Encrypt 9 0 R"),
                "pdf_password_required",
            ),
            (
                "active",
                test_pdf("/OpenAction 2 0 R", ""),
                "pdf_active_content_detected",
            ),
        ] {
            let source = SourceArtifact::from_bytes(
                format!("{label}.pdf"),
                OriginClass::TestFixture,
                &bytes,
                &limits,
            )
            .expect("source");
            let probe =
                crate::probe_source_bytes(&adapter, &source, &bytes, &limits).expect("probe");
            assert!(!probe.safe_to_plan, "{label}");
            assert!(
                probe
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == expected_code),
                "{label}"
            );
            let error = adapter
                .plan(
                    &source,
                    &probe,
                    PlanRequest::single_node(PortablePath::parse("Imported").expect("destination")),
                    limits.clone(),
                )
                .expect_err(label);
            assert_eq!(error.code(), ImportErrorCode::ProbeRejected, "{label}");
        }
    }

    #[test]
    fn complete_lock_rejects_one_changed_installed_byte() {
        let (lock_json, mut artifacts) = fixture_lock_and_artifacts();
        artifacts[0].bytes.push(0);
        let error = DoclingLitePdfAdapter::from_verified_assets(
            &lock_json,
            "x86_64-test-offline",
            &artifacts,
        )
        .expect_err("changed artifact must fail");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[test]
    fn embedded_onnx_runtime_is_bound_to_worker_bytes_without_double_counting() {
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        let lock = DoclingLiteAssetLock::from_json(&lock_json).expect("fixture lock");
        let worker = lock
            .artifacts
            .iter()
            .find(|artifact| artifact.component == "docling-rs")
            .expect("worker pin");
        let runtime = lock
            .artifacts
            .iter()
            .find(|artifact| artifact.component == "onnx-runtime")
            .expect("embedded runtime pin");
        assert_eq!(runtime.role, DoclingLiteArtifactRole::EmbeddedComponent);
        assert_eq!(runtime.install_path, worker.install_path);
        assert_eq!(runtime.byte_length, worker.byte_length);
        assert_eq!(runtime.sha256, worker.sha256);
        assert_eq!(
            lock.computed_total_bytes().expect("physical total"),
            artifacts
                .iter()
                .map(|artifact| u64::try_from(artifact.bytes.len()).expect("length"))
                .sum::<u64>()
        );

        let mut forged: serde_json::Value =
            serde_json::from_slice(&lock_json).expect("fixture lock JSON");
        let pins = forged["artifacts"].as_array_mut().expect("artifact array");
        let runtime = pins
            .iter_mut()
            .find(|pin| pin["component"] == "onnx-runtime")
            .expect("runtime pin");
        runtime["sha256"] = serde_json::Value::String(sha256_bytes(b"invented DLL").to_string());
        let forged = serde_json::to_vec(&forged).expect("forged JSON");
        let error = DoclingLiteAssetLock::from_json(&forged)
            .expect_err("embedded component cannot claim independent bytes");
        assert_eq!(error.code(), ImportErrorCode::InvalidContract);
    }

    #[cfg(unix)]
    #[test]
    fn fixed_installation_directory_hashes_every_regular_asset_but_stays_isolation_gated() {
        let temporary = tempfile::tempdir().expect("installation root");
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        write_fixture_installation(temporary.path(), &lock_json, &artifacts);

        let adapter = DoclingLitePdfAdapter::from_installation_directory(
            temporary.path(),
            "x86_64-test-offline",
        )
        .expect("pinned installation");
        let capability = adapter.capability();
        assert!(!capability.available);
        assert!(capability.missing_pinned_evidence.is_empty());
        assert!(!capability.missing_isolation_evidence.is_empty());
        assert!(!capability.ambient_network_allowed);

        let changed = temporary.path().join(artifacts[0].install_path.as_str());
        std::fs::write(&changed, b"tampered").expect("tamper fixture");
        let error = DoclingLitePdfAdapter::from_installation_directory(
            temporary.path(),
            "x86_64-test-offline",
        )
        .expect_err("tampered installation");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[cfg(windows)]
    #[test]
    fn windows_installation_uses_exclusive_handles_for_stable_asset_evidence() {
        let temporary = tempfile::tempdir().expect("installation root");
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        write_fixture_installation(temporary.path(), &lock_json, &artifacts);
        let adapter = DoclingLitePdfAdapter::from_installation_directory(
            temporary.path(),
            "x86_64-test-offline",
        )
        .expect("pinned Windows installation");
        let capability = adapter.capability();
        assert!(!capability.available);
        assert!(capability.missing_pinned_evidence.is_empty());
        assert!(!capability.missing_isolation_evidence.is_empty());

        let changed = temporary.path().join(artifacts[0].install_path.as_str());
        std::fs::write(&changed, b"tampered").expect("tamper fixture");
        let error = DoclingLitePdfAdapter::from_installation_directory(
            temporary.path(),
            "x86_64-test-offline",
        )
        .expect_err("tampered installation");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[cfg(unix)]
    #[test]
    fn fixed_installation_directory_rejects_linked_assets() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("installation root");
        let external = tempfile::tempdir().expect("external root");
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        write_fixture_installation(temporary.path(), &lock_json, &artifacts);
        let linked = temporary.path().join(artifacts[0].install_path.as_str());
        let bytes = std::fs::read(&linked).expect("fixture worker");
        std::fs::remove_file(&linked).expect("remove worker");
        let external_worker = external.path().join("worker");
        std::fs::write(&external_worker, bytes).expect("external worker");
        symlink(&external_worker, &linked).expect("linked worker");

        let error = DoclingLitePdfAdapter::from_installation_directory(
            temporary.path(),
            "x86_64-test-offline",
        )
        .expect_err("linked installation");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[test]
    fn audit_lock_proves_total_but_cannot_enable_execution() {
        let bytes = include_bytes!("../docling-lite-assets.lock.json");
        let lock = DoclingLiteAssetLock::from_json(bytes).expect("audited lock metadata");
        assert_eq!(lock.computed_total_bytes().expect("sum"), 85_132_854);
        assert!(!lock.complete_for_execution);
        assert_eq!(lock.docling_release_tag, DOCLING_RELEASE_TAG);
        assert_eq!(lock.docling_release_commit, DOCLING_RELEASE_COMMIT);
    }

    #[test]
    fn rich_fixture_maps_cjk_rtl_provenance_list_table_and_picture() {
        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        let document =
            map_fixture(&adapter, &source, &plan, "rich.json").expect("mapped rich fixture");
        assert_eq!(document.title, "中文 العربية");
        assert!(document.node_count() >= 5);
        assert_eq!(document.resources.len(), 1);
        assert_eq!(document.resources[0].bytes, b"PNG");
        assert!(document.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                ImportNodeKind::Section { title, .. } if title == "章节 واحد"
            )
        }));
        assert!(
            document
                .diagnostics
                .iter()
                .any(|item| item.code == "table_fallback")
        );
        let first_location = document
            .nodes
            .iter()
            .find_map(|node| node.source_locations.first())
            .expect("page location");
        assert_eq!(first_location.page, Some(1));
        assert!(first_location.region.is_some());
    }

    #[test]
    fn official_v0522_born_digital_and_scanned_outputs_enter_import_ir() {
        fn has_bounded_page(nodes: &[crate::ImportNode], page: u32) -> bool {
            nodes.iter().any(|node| {
                node.source_locations
                    .iter()
                    .any(|location| location.page == Some(page) && location.region.is_some())
                    || matches!(
                        &node.kind,
                        ImportNodeKind::Section { children, .. }
                            if has_bounded_page(children, page)
                    )
            })
        }

        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        let born_digital =
            map_fixture(&adapter, &source, &plan, "official-v0.52.2-multi-page.json")
                .expect("map official multi-page output");
        assert!(
            born_digital.node_count() >= 30,
            "mapped {} nodes with diagnostics {:?}",
            born_digital.node_count(),
            born_digital.diagnostics
        );
        assert!(has_bounded_page(&born_digital.nodes, 5));
        let born_proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &test_pdf("", ""), &plan, &born_digital)
            .expect("render official multi-page IR");
        assert_eq!(born_proposal.proposal().nodes.len(), 1);
        assert_eq!(
            born_proposal.proposal().nodes[0].document_file,
            "Imported.adoc"
        );
        assert!(
            born_proposal.proposal().nodes[0]
                .exact_asciidoc
                .starts_with("---\nweftext:\n  id: \"")
        );

        let scanned = map_fixture(
            &adapter,
            &source,
            &plan,
            "official-v0.52.2-scanned-ocr.json",
        )
        .expect("map official scanned output");
        assert_eq!(scanned.node_count(), 1);
        assert!(scanned.nodes.iter().any(|node| {
            matches!(
                &node.kind,
                ImportNodeKind::Paragraph { text }
                    if text.contains("Docling bundles PDF document conversion")
            )
        }));
        AsciiDocV1ProposalValidator
            .render_and_validate(&source, &test_pdf("", ""), &plan, &scanned)
            .expect("render official scanned IR");
    }

    #[test]
    fn official_v0522_picture_output_becomes_bounded_node_resources() {
        fn figure_count(nodes: &[crate::ImportNode]) -> usize {
            nodes
                .iter()
                .map(|node| match &node.kind {
                    ImportNodeKind::Figure { .. } => 1,
                    ImportNodeKind::Section { children, .. } => figure_count(children),
                    _ => 0,
                })
                .sum()
        }

        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        let document = map_fixture(&adapter, &source, &plan, "official-v0.52.2-pictures.json")
            .expect("map official picture output");
        assert_eq!(document.resources.len(), 2);
        assert!(document.resources.iter().all(|resource| {
            resource.media_type == "image/png"
                && resource.byte_length > 10_000
                && resource.byte_length == u64::try_from(resource.bytes.len()).expect("length")
        }));
        assert_eq!(figure_count(&document.nodes), 2);
        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &test_pdf("", ""), &plan, &document)
            .expect("render official picture IR");
        assert_eq!(proposal.proposal().nodes[0].resources.len(), 2);
        assert_eq!(proposal.proposal().nodes[0].resource_references.len(), 2);
    }

    #[test]
    fn official_v0522_scanned_chart_table_preserves_geometric_table_and_figure() {
        fn table(nodes: &[crate::ImportNode]) -> Option<(u16, &[Vec<String>])> {
            for node in nodes {
                match &node.kind {
                    ImportNodeKind::Table { header_rows, rows } => {
                        return Some((*header_rows, rows));
                    }
                    ImportNodeKind::Section { children, .. } => {
                        if let Some(table) = table(children) {
                            return Some(table);
                        }
                    }
                    _ => {}
                }
            }
            None
        }

        fn figure_count(nodes: &[crate::ImportNode]) -> usize {
            nodes
                .iter()
                .map(|node| match &node.kind {
                    ImportNodeKind::Figure { .. } => 1,
                    ImportNodeKind::Section { children, .. } => figure_count(children),
                    _ => 0,
                })
                .sum()
        }

        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        let document = map_fixture(
            &adapter,
            &source,
            &plan,
            "official-v0.52.2-scanned-chart-table.json",
        )
        .expect("map official scanned chart/table output");

        let (header_rows, rows) = table(&document.nodes).expect("mapped table");
        assert_eq!(header_rows, 1);
        assert_eq!(rows.len(), 11);
        assert_eq!(
            rows[0],
            ["Aquifer system", "Volume km3", "Depth m", "Recharge %"]
        );
        assert_eq!(rows[1], ["Ogallala", "420", "120", "4.5"]);
        assert_eq!(document.resources.len(), 1);
        assert_eq!(figure_count(&document.nodes), 1);
        assert!(
            document
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "table_fallback")
        );

        let proposal = AsciiDocV1ProposalValidator
            .render_and_validate(&source, &test_pdf("", ""), &plan, &document)
            .expect("render official scanned chart/table IR");
        let node = &proposal.proposal().nodes[0];
        assert!(
            node.exact_asciidoc
                .contains("[cols=\"4*\",options=\"header\"]")
        );
        assert!(node.exact_asciidoc.contains("|Aquifer system"));
        assert!(node.exact_asciidoc.contains("|Ogallala"));
        assert_eq!(node.resources.len(), 1);
        assert_eq!(node.resource_references.len(), 1);
    }

    #[test]
    fn wrong_schema_bad_ref_and_bad_bbox_fail_closed() {
        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        for fixture in ["unknown-schema.json", "bad-ref.json", "bad-bbox.json"] {
            let error = map_fixture(&adapter, &source, &plan, fixture).expect_err(fixture);
            assert_eq!(error.code(), ImportErrorCode::WorkerProtocol, "{fixture}");
        }
    }

    #[test]
    fn oversized_data_uri_is_rejected_before_full_decode() {
        let (adapter, _) = verified_adapter();
        let limits = ImportLimits {
            max_resource_bytes: 2,
            ..ImportLimits::default()
        };
        let (source, _, plan) = planned_with_limits(&adapter, limits);
        let error = map_fixture(&adapter, &source, &plan, "oversize-data-uri.json")
            .expect_err("oversized URI");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);
    }

    #[test]
    fn embedded_and_envelope_resources_share_one_count_limit() {
        let (adapter, components) = verified_adapter();
        let limits = ImportLimits {
            max_resource_count: 1,
            ..ImportLimits::default()
        };
        let (source, _, plan) = planned_with_limits(&adapter, limits);
        let worker_bytes = b"worker-resource".to_vec();
        let response = WorkerResponse {
            contract_version: crate::WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: format!(
                "request-{}",
                plan.plan_id.strip_prefix("plan-").expect("plan id")
            ),
            worker_id: super::WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: source.sha256.clone(),
            payload: serde_json::from_str(fixture_json("rich.json")).expect("fixture JSON"),
            resources: vec![WorkerResource {
                locator: PortablePath::parse("resources/from-worker.bin").expect("resource path"),
                media_type: "application/octet-stream".to_owned(),
                byte_length: u64::try_from(worker_bytes.len()).expect("resource length"),
                sha256: sha256_bytes(&worker_bytes),
                bytes: worker_bytes,
            }],
            diagnostics: Vec::new(),
            components,
        };
        let error = adapter
            .map_worker_response(&source, &plan, response)
            .expect_err("combined resource count");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);
    }

    #[test]
    fn detached_asset_verification_cannot_bypass_product_isolation_gate() {
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        let gated = DoclingLitePdfAdapter::from_verified_assets(
            &lock_json,
            "x86_64-test-offline",
            &artifacts,
        )
        .expect("verified detached fixture");
        let enabled = gated.clone().enable_fixture_execution();
        let (source, _, plan) = planned(&enabled);
        let response = WorkerResponse {
            contract_version: crate::WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: format!(
                "request-{}",
                plan.plan_id.strip_prefix("plan-").expect("plan id")
            ),
            worker_id: super::WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: source.sha256.clone(),
            payload: serde_json::from_str(fixture_json("rich.json")).expect("fixture JSON"),
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components: required_fixture_rows()
                .into_iter()
                .map(|(component, _, _, bytes)| ComponentVersion {
                    component_id: component.to_owned(),
                    version: "fixture-v1".to_owned(),
                    artifact_digest: Some(sha256_bytes(bytes)),
                })
                .collect(),
        };
        let error = gated
            .map_worker_response(&source, &plan, response)
            .expect_err("product isolation gate");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[test]
    fn formula_and_unsupported_constructs_have_explicit_diagnostics() {
        let (adapter, _) = verified_adapter();
        let (source, _, plan) = planned(&adapter);
        let formula = map_fixture(&adapter, &source, &plan, "formula-placeholder.json")
            .expect("formula fixture");
        assert!(
            formula
                .diagnostics
                .iter()
                .any(|item| item.code == "formula_placeholder")
        );
        assert!(
            formula
                .nodes
                .iter()
                .any(|node| matches!(node.kind, ImportNodeKind::Formula { .. }))
        );

        let unsupported =
            map_fixture(&adapter, &source, &plan, "unsupported.json").expect("unsupported fixture");
        for code in [
            "form_unsupported",
            "checkbox_unsupported",
            "header_footer_unsupported",
        ] {
            assert!(unsupported.diagnostics.iter().any(|item| item.code == code));
        }
    }

    #[test]
    fn typed_response_round_trips_through_generic_bridge() {
        let (_, components) = verified_adapter();
        let typed = DoclingLiteWorkerResponse {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: "request-fixture".to_owned(),
            source_digest: sha256_bytes(b"source"),
            status: DoclingLiteWorkerStatus::Completed,
            docling_document_json: Some(serde_json::json!({
                "schema_name": "DoclingDocument",
                "version": "1.10.0"
            })),
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components,
        };
        let generic = typed.clone().into_generic().expect("generic envelope");
        assert_eq!(
            generic
                .payload
                .get("schema_name")
                .and_then(serde_json::Value::as_str),
            Some(DOCLING_DOCUMENT_SCHEMA_NAME)
        );
        assert!(
            generic.payload.get("status").is_none(),
            "completed responses must use the real worker's raw DoclingDocument payload"
        );
        let round_trip =
            DoclingLiteWorkerResponse::try_from_generic(generic).expect("typed response");
        assert_eq!(round_trip, typed);
    }

    #[test]
    fn completed_wrapper_and_unknown_worker_fields_fail_closed() {
        let (_, components) = verified_adapter();
        let typed = DoclingLiteWorkerResponse {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: "request-fixture".to_owned(),
            source_digest: sha256_bytes(b"source"),
            status: DoclingLiteWorkerStatus::Completed,
            docling_document_json: Some(serde_json::json!({
                "schema_name": DOCLING_DOCUMENT_SCHEMA_NAME,
                "version": DOCLING_DOCUMENT_SCHEMA_VERSION
            })),
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components,
        };
        let mut generic = typed.into_generic().expect("generic envelope");
        let document = generic.payload.clone();
        generic.payload = serde_json::json!({
            "status": "completed",
            "doclingDocumentJson": document
        });
        let error = DoclingLiteWorkerResponse::try_from_generic(generic)
            .expect_err("a second completed payload shape must be rejected");
        assert_eq!(error.code(), ImportErrorCode::WorkerProtocol);

        let resource = serde_json::json!({
            "locator": "resources/figure.png",
            "mediaType": "image/png",
            "byteLength": 3,
            "sha256": sha256_bytes(b"PNG"),
            "bytes": [80, 78, 71],
            "futureAuthority": true
        });
        assert!(
            serde_json::from_value::<crate::WorkerResource>(resource).is_err(),
            "worker resources must not discard unknown authority"
        );

        let response = serde_json::json!({
            "contractVersion": crate::WORKER_RESPONSE_CONTRACT_VERSION,
            "requestId": "request-fixture",
            "workerId": super::WORKER_ID,
            "workerProtocolVersion": DOCLING_LITE_WORKER_PROTOCOL_VERSION,
            "sourceDigest": sha256_bytes(b"source"),
            "payload": {
                "status": "failed",
                "doclingDocumentJson": null
            },
            "resources": [],
            "diagnostics": [],
            "components": [],
            "futureAuthority": true
        });
        assert!(serde_json::from_value::<WorkerResponse>(response).is_err());
    }

    #[test]
    fn isolated_command_requires_all_six_pins() {
        let pins = required_fixture_rows()
            .into_iter()
            .map(|(component, _, _, bytes)| DoclingModelPin {
                component: component.to_owned(),
                version: "fixture-v1".to_owned(),
                sha256: sha256_bytes(bytes),
                notice_id: format!("notice-{component}"),
            })
            .collect();
        let mut command = DoclingLiteWorkerCommand {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: "request-pdf-fixture".to_owned(),
            source_digest: sha256_bytes(b"%PDF-fixture"),
            plan_id: "plan-pdf-fixture".to_owned(),
            input_locator: PortablePath::parse("input/source.pdf").expect("input path"),
            output_locator: PortablePath::parse("output/docling-document.json")
                .expect("output path"),
            docling_release_tag: DOCLING_RELEASE_TAG.to_owned(),
            docling_release_commit: DOCLING_RELEASE_COMMIT.to_owned(),
            document_schema_name: "DoclingDocument".to_owned(),
            document_schema_version: "1.10.0".to_owned(),
            target: "x86_64-test-offline".to_owned(),
            local_ocr_policy: LocalOcrPolicy::Automatic,
            ocr_language: "en".to_owned(),
            layout_precision: "int8".to_owned(),
            no_table_former: true,
            network: WorkerNetworkPolicy::Denied,
            page_limit: 100,
            memory_limit_bytes: 512 * 1024 * 1024,
            output_byte_limit: 64 * 1024 * 1024,
            model_pins: pins,
        };
        command.validate_boundary().expect("valid wire shape");
        let mut extra = command.model_pins[0].clone();
        extra.component = "unexpected-runtime".to_owned();
        command.model_pins.push(extra);
        let error = command
            .validate_boundary()
            .expect_err("extra component authority");
        assert_eq!(error.code(), ImportErrorCode::WorkerProtocol);
        command.model_pins.pop();
        command.output_byte_limit = super::MIN_WORKER_RESPONSE_BYTES - 1;
        let error = command
            .validate_boundary()
            .expect_err("response limit below closed failure envelope");
        assert_eq!(error.code(), ImportErrorCode::WorkerProtocol);
    }

    fn verified_adapter() -> (DoclingLitePdfAdapter, Vec<ComponentVersion>) {
        let (lock_json, artifacts) = fixture_lock_and_artifacts();
        let adapter = DoclingLitePdfAdapter::from_verified_assets(
            &lock_json,
            "x86_64-test-offline",
            &artifacts,
        )
        .expect("verified fixture installation")
        .enable_fixture_execution();
        let components = required_fixture_rows()
            .into_iter()
            .map(|(component, _, _, bytes)| ComponentVersion {
                component_id: component.to_owned(),
                version: "fixture-v1".to_owned(),
                artifact_digest: Some(sha256_bytes(bytes)),
            })
            .collect();
        (adapter, components)
    }

    fn fixture_lock_and_artifacts() -> (Vec<u8>, Vec<DoclingLiteInstalledArtifact>) {
        let rows = required_fixture_rows();
        let mut physical_files = std::collections::BTreeMap::new();
        for (_, _, path, bytes) in &rows {
            if let Some(existing) = physical_files.insert(*path, *bytes) {
                assert_eq!(
                    existing, *bytes,
                    "shared fixture path must bind exact bytes"
                );
            }
        }
        let artifacts = physical_files
            .into_iter()
            .map(|(path, bytes)| DoclingLiteInstalledArtifact {
                install_path: PortablePath::parse(path).expect("fixture path"),
                bytes: bytes.to_vec(),
            })
            .collect::<Vec<_>>();
        let pins = rows
            .iter()
            .map(|(component, role, path, bytes)| DoclingLiteAssetPin {
                component: (*component).to_owned(),
                version: "fixture-v1".to_owned(),
                role: *role,
                target: "x86_64-test-offline".to_owned(),
                install_path: PortablePath::parse(*path).expect("fixture path"),
                source_url: format!("https://example.invalid/pinned/{component}/fixture-v1"),
                byte_length: u64::try_from(bytes.len()).expect("fixture length"),
                sha256: sha256_bytes(bytes),
                license: "fixture-only".to_owned(),
                notice_id: format!("notice-{component}"),
            })
            .collect::<Vec<_>>();
        let mut lock = DoclingLiteAssetLock {
            lock_version: DOCLING_LITE_ASSET_LOCK_VERSION.to_owned(),
            docling_release_tag: DOCLING_RELEASE_TAG.to_owned(),
            docling_release_commit: DOCLING_RELEASE_COMMIT.to_owned(),
            document_schema_name: "DoclingDocument".to_owned(),
            document_schema_version: "1.10.0".to_owned(),
            profile: "pdfium-layout-int8-ppocrv3-en".to_owned(),
            target: "x86_64-test-offline".to_owned(),
            network: WorkerNetworkPolicy::Denied,
            no_table_former: true,
            ocr_language: "en".to_owned(),
            total_package_bytes: 0,
            complete_for_execution: true,
            missing_for_execution: Vec::new(),
            artifacts: pins,
            distribution_archives: Vec::new(),
        };
        lock.total_package_bytes = lock.computed_total_bytes().expect("fixture physical total");
        (
            serde_json::to_vec(&lock).expect("serialize fixture lock"),
            artifacts,
        )
    }

    fn write_fixture_installation(
        root: &std::path::Path,
        lock_json: &[u8],
        artifacts: &[DoclingLiteInstalledArtifact],
    ) {
        std::fs::write(
            root.join(super::DOCLING_LITE_INSTALLATION_LOCK_FILE),
            lock_json,
        )
        .expect("fixture lock");
        for artifact in artifacts {
            let path = root.join(artifact.install_path.as_str());
            std::fs::create_dir_all(path.parent().expect("artifact parent"))
                .expect("artifact directories");
            std::fs::write(path, &artifact.bytes).expect("artifact bytes");
        }
    }

    type FixtureRow = (
        &'static str,
        DoclingLiteArtifactRole,
        &'static str,
        &'static [u8],
    );

    fn required_fixture_rows() -> Vec<FixtureRow> {
        vec![
            (
                "docling-rs",
                DoclingLiteArtifactRole::WorkerBinary,
                "bin/docling-rs",
                b"worker",
            ),
            (
                "pdfium",
                DoclingLiteArtifactRole::NativeLibrary,
                "lib/pdfium.bin",
                b"pdfium",
            ),
            (
                "onnx-runtime",
                DoclingLiteArtifactRole::EmbeddedComponent,
                "bin/docling-rs",
                b"worker",
            ),
            (
                "layout-int8",
                DoclingLiteArtifactRole::Model,
                "models/layout-int8.onnx",
                b"layout",
            ),
            (
                "pp-ocr",
                DoclingLiteArtifactRole::Model,
                "models/ppocr-en.onnx",
                b"ocr",
            ),
            (
                "ocr-dictionary",
                DoclingLiteArtifactRole::Dictionary,
                "models/en-dict.txt",
                b"dict",
            ),
        ]
    }

    fn test_pdf(catalog_extra: &str, trailer_extra: &str) -> Vec<u8> {
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

    fn planned(
        adapter: &DoclingLitePdfAdapter,
    ) -> (SourceArtifact, crate::FormatProbe, crate::ImportPlan) {
        planned_with_limits(adapter, ImportLimits::default())
    }

    fn planned_with_limits(
        adapter: &DoclingLitePdfAdapter,
        limits: ImportLimits,
    ) -> (SourceArtifact, crate::FormatProbe, crate::ImportPlan) {
        let bytes = test_pdf("", "");
        let mut source =
            SourceArtifact::from_bytes("fixture.pdf", OriginClass::TestFixture, &bytes, &limits)
                .expect("source");
        let probe = crate::probe_source_bytes(adapter, &source, &bytes, &limits).expect("probe");
        source.detected_format = probe.detected_format;
        source
            .mismatch_evidence
            .clone_from(&probe.mismatch_evidence);
        let plan = adapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Imported").expect("destination")),
                limits,
            )
            .expect("plan");
        (source, probe, plan)
    }

    fn map_fixture(
        adapter: &DoclingLitePdfAdapter,
        source: &SourceArtifact,
        plan: &crate::ImportPlan,
        fixture: &str,
    ) -> Result<crate::ImportDocument, crate::ImportError> {
        let json = fixture_json(fixture);
        let (_, components) = verified_adapter();
        let response = WorkerResponse {
            contract_version: crate::WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: format!(
                "request-{}",
                plan.plan_id.strip_prefix("plan-").expect("plan id")
            ),
            worker_id: super::WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: source.sha256.clone(),
            payload: serde_json::from_str(json).expect("fixture JSON"),
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components,
        };
        adapter.map_worker_response(source, plan, response)
    }

    fn fixture_json(fixture: &str) -> &'static str {
        match fixture {
            "rich.json" => include_str!("../tests/fixtures/docling-lite/rich.json"),
            "unknown-schema.json" => {
                include_str!("../tests/fixtures/docling-lite/unknown-schema.json")
            }
            "bad-ref.json" => include_str!("../tests/fixtures/docling-lite/bad-ref.json"),
            "bad-bbox.json" => include_str!("../tests/fixtures/docling-lite/bad-bbox.json"),
            "oversize-data-uri.json" => {
                include_str!("../tests/fixtures/docling-lite/oversize-data-uri.json")
            }
            "formula-placeholder.json" => {
                include_str!("../tests/fixtures/docling-lite/formula-placeholder.json")
            }
            "unsupported.json" => {
                include_str!("../tests/fixtures/docling-lite/unsupported.json")
            }
            "official-v0.52.2-multi-page.json" => {
                include_str!("../tests/fixtures/docling-lite/official-v0.52.2-multi-page.json")
            }
            "official-v0.52.2-scanned-ocr.json" => {
                include_str!("../tests/fixtures/docling-lite/official-v0.52.2-scanned-ocr.json")
            }
            "official-v0.52.2-pictures.json" => {
                include_str!("../tests/fixtures/docling-lite/official-v0.52.2-pictures.json")
            }
            "official-v0.52.2-scanned-chart-table.json" => include_str!(
                "../tests/fixtures/docling-lite/official-v0.52.2-scanned-chart-table.json"
            ),
            _ => panic!("unknown fixture"),
        }
    }
}
