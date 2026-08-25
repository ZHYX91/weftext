use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::pipeline::CancellationToken;
use crate::{
    FORMAT_PROBE_CONTRACT_VERSION, IMPORT_IR_CONTRACT_VERSION, IMPORT_PLAN_CONTRACT_VERSION,
    IMPORT_PROPOSAL_CONTRACT_VERSION, IMPORT_RECEIPT_CONTRACT_VERSION, ImportError,
    ImportErrorCode, ImportLimits, PortablePath, ProbeEvidence, ProbeReader,
    SOURCE_ARTIFACT_CONTRACT_VERSION, Sha256Digest, WORKER_REQUEST_CONTRACT_VERSION,
    WORKER_RESPONSE_CONTRACT_VERSION, sha256_bytes,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginClass {
    LocalFile,
    Clipboard,
    Download,
    ServerUpload,
    AgentProvided,
    TestFixture,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Unknown,
    FakeFixture,
    Pdf,
    Image,
    Html,
    Docx,
    Odt,
    Markdown,
    Tex,
    Csv,
    Xlsx,
    Ods,
    Epub,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceArtifact {
    pub contract_version: String,
    pub source_id: String,
    pub display_name: String,
    pub origin: OriginClass,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub extension_hint: Option<String>,
    pub detected_format: SourceFormat,
    pub mismatch_evidence: Vec<String>,
}

impl SourceArtifact {
    /// Inventories exact source bytes without retaining a filesystem locator.
    ///
    /// # Errors
    ///
    /// Returns an error for an unsafe display name, invalid limits, or an
    /// oversized source.
    pub fn from_bytes(
        display_name: impl Into<String>,
        origin: OriginClass,
        bytes: &[u8],
        limits: &ImportLimits,
    ) -> Result<Self, ImportError> {
        limits.validate()?;
        limits.check(
            "source byte length",
            usize_to_u64(bytes.len()),
            limits.max_source_bytes,
        )?;
        let display_name = display_name.into();
        validate_source_name(&display_name)?;
        let digest = sha256_bytes(bytes);
        let extension_hint = Path::new(&display_name)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase);
        Ok(Self {
            contract_version: SOURCE_ARTIFACT_CONTRACT_VERSION.to_owned(),
            source_id: format!("source-{}", &digest.as_str()[..24]),
            display_name,
            origin,
            byte_length: usize_to_u64(bytes.len()),
            sha256: digest,
            extension_hint,
            detected_format: SourceFormat::Unknown,
            mismatch_evidence: Vec::new(),
        })
    }

    pub(crate) fn validate(&self, limits: &ImportLimits) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            SOURCE_ARTIFACT_CONTRACT_VERSION,
            "source artifact",
        )?;
        validate_identifier(&self.source_id, "source id")?;
        validate_source_name(&self.display_name)?;
        limits.check(
            "source byte length",
            self.byte_length,
            limits.max_source_bytes,
        )?;
        if self.mismatch_evidence.len() > 32
            || self
                .mismatch_evidence
                .iter()
                .any(|evidence| evidence.len() > 1_024)
        {
            return invalid_contract("source mismatch evidence is outside its bounded contract");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EncryptionState {
    NotEncrypted,
    PasswordRequired,
    EncryptedUnsupported,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Blocking,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source_location: Option<ImportSourceLocation>,
    pub ir_node_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatProbe {
    pub contract_version: String,
    pub adapter: AdapterDescriptor,
    pub source_digest: Sha256Digest,
    pub evidence: ProbeEvidence,
    pub detected_format: SourceFormat,
    pub signature_confidence: Confidence,
    pub parser_confidence: Confidence,
    pub encryption: EncryptionState,
    pub signature_evidence: Vec<String>,
    pub mismatch_evidence: Vec<String>,
    pub active_content_detected: bool,
    pub page_count: Option<u32>,
    pub container_entry_count: Option<u32>,
    pub safe_to_plan: bool,
    pub diagnostics: Vec<ImportDiagnostic>,
}

impl FormatProbe {
    pub(crate) fn validate(
        &self,
        source: &SourceArtifact,
        limits: &ImportLimits,
    ) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            FORMAT_PROBE_CONTRACT_VERSION,
            "format probe",
        )?;
        self.adapter.validate()?;
        if self.source_digest != source.sha256 {
            return invalid_contract("format probe source digest does not match the artifact");
        }
        self.evidence.validate(source, limits)?;
        if let Some(pages) = self.page_count {
            limits.check("page count", u64::from(pages), u64::from(limits.max_pages))?;
        }
        if let Some(entries) = self.container_entry_count {
            limits.check(
                "container entry count",
                u64::from(entries),
                u64::from(limits.max_container_entries),
            )?;
        }
        validate_evidence(&self.signature_evidence, "signature evidence")?;
        validate_evidence(&self.mismatch_evidence, "mismatch evidence")?;
        validate_diagnostics(&self.diagnostics, limits)?;
        for diagnostic in &self.diagnostics {
            if let Some(location) = &diagnostic.source_location {
                location.validate(source, limits)?;
            }
        }
        if self.safe_to_plan
            && (self.encryption != EncryptionState::NotEncrypted
                || self.active_content_detected
                || self
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Blocking))
        {
            return invalid_contract(
                "only a proven unencrypted, inactive probe without blocking diagnostics can be marked safe to plan",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Confidence(u16);

impl Confidence {
    /// Creates a deterministic fixed-point confidence value.
    ///
    /// # Errors
    ///
    /// Returns an error when the value exceeds 100 percent.
    pub fn from_basis_points(value: u16) -> Result<Self, ImportError> {
        if value > 10_000 {
            return invalid_contract("confidence must be between 0 and 10,000 basis points");
        }
        Ok(Self(value))
    }

    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

impl Serialize for Confidence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u16(self.0)
    }
}

impl<'de> Deserialize<'de> for Confidence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::from_basis_points(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterDescriptor {
    pub adapter_id: String,
    pub adapter_version: String,
    pub supported_format: SourceFormat,
}

impl AdapterDescriptor {
    pub(crate) fn validate(&self) -> Result<(), ImportError> {
        validate_identifier(&self.adapter_id, "adapter id")?;
        validate_version_label(&self.adapter_version, "adapter version")
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterRoute {
    pub adapter: AdapterDescriptor,
    pub worker_id: String,
    pub worker_protocol_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SplitPolicy {
    SingleNode,
    TopLevelSections { maximum_nodes: u32 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePolicy {
    ExtractReferenced,
    SkipAll,
    ExtractAndRetainOriginal,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalOcrPolicy {
    Automatic,
    Always,
    Never,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum AgentEnhancementPolicy {
    Disabled,
    SelectedRegionsOnly { provider: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "mode")]
pub enum EgressDisclosure {
    None,
    AgentSelectedEvidence {
        provider: String,
        selected_node_ids: Vec<String>,
        disclosed_bytes: u64,
        retention: String,
        redaction: String,
    },
}

/// Exact local-IR selection reviewed before any evidence leaves Weftext.
///
/// Selection is deliberately created after deterministic extraction: IR node
/// identifiers do not exist when the format worker plan is first frozen.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AgentEnhancementSelection {
    pub provider: String,
    pub selected_node_ids: Vec<String>,
    pub disclosed_bytes: u64,
    pub retention: String,
    pub redaction: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequest {
    pub destination: PortablePath,
    pub split_policy: SplitPolicy,
    pub resource_policy: ResourcePolicy,
    pub local_ocr_policy: LocalOcrPolicy,
    pub agent_enhancement: AgentEnhancementPolicy,
    pub egress: EgressDisclosure,
}

impl PlanRequest {
    #[must_use]
    pub fn single_node(destination: PortablePath) -> Self {
        Self {
            destination,
            split_policy: SplitPolicy::SingleNode,
            resource_policy: ResourcePolicy::ExtractReferenced,
            local_ocr_policy: LocalOcrPolicy::Automatic,
            agent_enhancement: AgentEnhancementPolicy::Disabled,
            egress: EgressDisclosure::None,
        }
    }

    pub(crate) fn validate(&self) -> Result<(), ImportError> {
        if let SplitPolicy::TopLevelSections { maximum_nodes } = self.split_policy
            && maximum_nodes == 0
        {
            return invalid_contract("split policy maximum_nodes must be non-zero");
        }
        match (&self.agent_enhancement, &self.egress) {
            (AgentEnhancementPolicy::Disabled, EgressDisclosure::None) => Ok(()),
            (
                AgentEnhancementPolicy::SelectedRegionsOnly { provider },
                EgressDisclosure::AgentSelectedEvidence {
                    provider: disclosure_provider,
                    selected_node_ids,
                    retention,
                    redaction,
                    ..
                },
            ) if provider == disclosure_provider => {
                validate_identifier(provider, "agent provider")?;
                if selected_node_ids.len() > 10_000
                    || selected_node_ids
                        .iter()
                        .any(|id| validate_identifier(id, "selected IR node id").is_err())
                    || retention.is_empty()
                    || retention.len() > 512
                    || redaction.is_empty()
                    || redaction.len() > 512
                {
                    return invalid_contract(
                        "agent egress selection, retention, or redaction disclosure is invalid",
                    );
                }
                Ok(())
            }
            _ => invalid_contract(
                "agent enhancement and egress disclosure must be enabled or disabled together",
            ),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub contract_version: String,
    pub plan_id: String,
    pub proposed_root_id: String,
    pub source_digest: Sha256Digest,
    pub probe_digest: Sha256Digest,
    pub route: AdapterRoute,
    pub destination: PortablePath,
    pub split_policy: SplitPolicy,
    pub resource_policy: ResourcePolicy,
    pub local_ocr_policy: LocalOcrPolicy,
    pub agent_enhancement: AgentEnhancementPolicy,
    pub limits: ImportLimits,
    pub egress: EgressDisclosure,
}

impl ImportPlan {
    /// Freezes a probe, route, policy, egress disclosure, and resource limits.
    ///
    /// # Errors
    ///
    /// Returns an error for inconsistent or unserializable plan material.
    pub fn create(
        source: &SourceArtifact,
        probe: &FormatProbe,
        route: AdapterRoute,
        request: PlanRequest,
        limits: ImportLimits,
    ) -> Result<Self, ImportError> {
        source.validate(&limits)?;
        probe.validate(source, &limits)?;
        request.validate()?;
        limits.validate()?;
        let probe_bytes = serde_json::to_vec(probe).map_err(|e| ImportError::serialization(&e))?;
        let probe_digest = sha256_bytes(&probe_bytes);
        // Identity is minted once when the reviewable plan is created. Rendering
        // the same frozen plan remains byte deterministic, while a later import
        // of the same artifact cannot accidentally claim the same node identity.
        let proposed_root_id = Uuid::new_v4().to_string();
        let plan_id = compute_plan_id(
            &source.sha256,
            &probe_digest,
            &proposed_root_id,
            &route,
            &request,
            &limits,
        )?;
        let plan = Self {
            contract_version: IMPORT_PLAN_CONTRACT_VERSION.to_owned(),
            plan_id,
            proposed_root_id,
            source_digest: source.sha256.clone(),
            probe_digest,
            route,
            destination: request.destination,
            split_policy: request.split_policy,
            resource_policy: request.resource_policy,
            local_ocr_policy: request.local_ocr_policy,
            agent_enhancement: request.agent_enhancement,
            limits,
            egress: request.egress,
        };
        plan.validate(source, probe)?;
        Ok(plan)
    }

    /// Derives the exact post-extraction plan that authorizes one bounded
    /// selected-evidence agent call.
    ///
    /// The local worker is not rerun and the proposed node identity is
    /// preserved. Callers must review this derived plan before disclosing the
    /// selected evidence, then accept only a typed patch bound to its IR
    /// revision and selection.
    ///
    /// # Errors
    ///
    /// Returns an error when the base plan is invalid or already authorizes
    /// egress, or when the provider, selection, disclosure, or derived plan is
    /// invalid.
    pub fn authorize_agent_enhancement(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
        selection: AgentEnhancementSelection,
    ) -> Result<Self, ImportError> {
        self.validate(source, probe)?;
        if !matches!(self.agent_enhancement, AgentEnhancementPolicy::Disabled)
            || !matches!(self.egress, EgressDisclosure::None)
        {
            return invalid_contract(
                "agent enhancement can be authorized only from the local no-egress plan",
            );
        }
        if selection.selected_node_ids.is_empty() || selection.disclosed_bytes == 0 {
            return invalid_contract(
                "agent enhancement requires a non-empty IR selection and disclosed byte count",
            );
        }
        let mut derived = self.clone();
        derived.agent_enhancement = AgentEnhancementPolicy::SelectedRegionsOnly {
            provider: selection.provider.clone(),
        };
        derived.egress = EgressDisclosure::AgentSelectedEvidence {
            provider: selection.provider,
            selected_node_ids: selection.selected_node_ids,
            disclosed_bytes: selection.disclosed_bytes,
            retention: selection.retention,
            redaction: selection.redaction,
        };
        let request = derived.plan_request();
        derived.plan_id = compute_plan_id(
            &derived.source_digest,
            &derived.probe_digest,
            &derived.proposed_root_id,
            &derived.route,
            &request,
            &derived.limits,
        )?;
        derived.validate(source, probe)?;
        Ok(derived)
    }

    pub(crate) fn validate(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
    ) -> Result<(), ImportError> {
        source.validate(&self.limits)?;
        probe.validate(source, &self.limits)?;
        require_version(
            &self.contract_version,
            IMPORT_PLAN_CONTRACT_VERSION,
            "import plan",
        )?;
        validate_identifier(&self.plan_id, "plan id")?;
        let proposed_root_id = Uuid::parse_str(&self.proposed_root_id).map_err(|_| {
            ImportError::new(
                ImportErrorCode::InvalidContract,
                "proposed root id is not a UUID",
            )
        })?;
        if proposed_root_id.get_version_num() != 4
            || proposed_root_id.get_variant() != uuid::Variant::RFC4122
            || proposed_root_id.to_string() != self.proposed_root_id
        {
            return invalid_contract(
                "proposed root id must be one lowercase RFC 4122 UUIDv4 minted by the plan",
            );
        }
        self.route.adapter.validate()?;
        validate_identifier(&self.route.worker_id, "worker id")?;
        validate_version_label(
            &self.route.worker_protocol_version,
            "worker protocol version",
        )?;
        self.limits.validate()?;
        if self.source_digest != source.sha256 || self.route.adapter != probe.adapter {
            return invalid_contract("import plan does not match its source probe");
        }
        let bytes = serde_json::to_vec(probe).map_err(|e| ImportError::serialization(&e))?;
        if self.probe_digest != sha256_bytes(&bytes) {
            return invalid_contract("import plan probe digest is stale or forged");
        }
        let request = self.plan_request();
        request.validate()?;
        let expected_plan_id = compute_plan_id(
            &self.source_digest,
            &self.probe_digest,
            &self.proposed_root_id,
            &self.route,
            &request,
            &self.limits,
        )?;
        if self.plan_id != expected_plan_id {
            return invalid_contract("import plan id does not match its frozen plan material");
        }
        if let EgressDisclosure::AgentSelectedEvidence {
            selected_node_ids,
            disclosed_bytes,
            ..
        } = &self.egress
        {
            self.limits.check(
                "agent egress selected node count",
                usize_to_u64(selected_node_ids.len()),
                u64::from(self.limits.max_agent_selected_nodes),
            )?;
            self.limits.check(
                "agent disclosed bytes",
                *disclosed_bytes,
                self.limits.max_agent_output_bytes,
            )?;
        }
        Ok(())
    }

    fn plan_request(&self) -> PlanRequest {
        PlanRequest {
            destination: self.destination.clone(),
            split_policy: self.split_policy.clone(),
            resource_policy: self.resource_policy,
            local_ocr_policy: self.local_ocr_policy,
            agent_enhancement: self.agent_enhancement.clone(),
            egress: self.egress.clone(),
        }
    }
}

fn compute_plan_id(
    source_digest: &Sha256Digest,
    probe_digest: &Sha256Digest,
    proposed_root_id: &str,
    route: &AdapterRoute,
    request: &PlanRequest,
    limits: &ImportLimits,
) -> Result<String, ImportError> {
    let material = serde_json::to_vec(&(
        source_digest,
        probe_digest,
        proposed_root_id,
        route,
        request,
        limits,
    ))
    .map_err(|error| ImportError::serialization(&error))?;
    let digest = sha256_bytes(&material);
    Ok(format!("plan-{}", &digest.as_str()[..24]))
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoundingRegion {
    pub x_millionths: u32,
    pub y_millionths: u32,
    pub width_millionths: u32,
    pub height_millionths: u32,
}

impl BoundingRegion {
    fn validate(&self) -> Result<(), ImportError> {
        let million = 1_000_000_u32;
        if self.x_millionths > million
            || self.y_millionths > million
            || self.width_millionths > million
            || self.height_millionths > million
            || self.x_millionths.saturating_add(self.width_millionths) > million
            || self.y_millionths.saturating_add(self.height_millionths) > million
        {
            return invalid_ir("normalized source regions must remain within their page");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSourceLocation {
    pub source_digest: Sha256Digest,
    pub page: Option<u32>,
    pub region: Option<BoundingRegion>,
    pub byte_start: Option<u64>,
    pub byte_end: Option<u64>,
}

impl ImportSourceLocation {
    fn validate(&self, source: &SourceArtifact, limits: &ImportLimits) -> Result<(), ImportError> {
        if self.source_digest != source.sha256 {
            return invalid_ir("source location digest does not match the source artifact");
        }
        if let Some(page) = self.page
            && (page == 0 || page > limits.max_pages)
        {
            return invalid_ir("source page is outside the configured page limit");
        }
        if let Some(region) = &self.region {
            if self.page.is_none() {
                return invalid_ir("a source region requires a page number");
            }
            region.validate()?;
        }
        match (self.byte_start, self.byte_end) {
            (Some(start), Some(end)) if start <= end && end <= source.byte_length => {}
            (None, None) => {}
            _ => return invalid_ir("source byte evidence is incomplete or out of range"),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    LocalExtraction,
    LocalOcr,
    AgentEnhancement,
    DeterministicRendering,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvenanceRecord {
    pub kind: ProvenanceKind,
    pub component_id: String,
    pub component_version: String,
    pub input_digests: Vec<Sha256Digest>,
    pub output_digest: Option<Sha256Digest>,
    pub source_locations: Vec<ImportSourceLocation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ImportNodeKind {
    Section {
        level: u8,
        title: String,
        children: Vec<ImportNode>,
    },
    Paragraph {
        text: String,
    },
    Quote {
        depth: u8,
        text: String,
    },
    Listing {
        language: Option<String>,
        source: String,
    },
    ThematicBreak,
    List {
        ordered: bool,
        items: Vec<String>,
    },
    Table {
        header_rows: u16,
        rows: Vec<Vec<String>>,
    },
    Figure {
        resource_id: String,
        alt: String,
        caption: Option<String>,
    },
    Formula {
        notation: String,
        source: String,
    },
    Link {
        target: String,
        label: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportNode {
    pub id: String,
    pub kind: ImportNodeKind,
    pub confidence: Confidence,
    pub source_locations: Vec<ImportSourceLocation>,
    pub provenance: Vec<ProvenanceRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResource {
    pub id: String,
    pub locator: PortablePath,
    pub media_type: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub bytes: Vec<u8>,
    pub source_locations: Vec<ImportSourceLocation>,
    pub provenance: Vec<ProvenanceRecord>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDocument {
    pub contract_version: String,
    pub document_id: String,
    pub revision: Sha256Digest,
    pub source_digest: Sha256Digest,
    pub title: String,
    pub nodes: Vec<ImportNode>,
    pub resources: Vec<ImportResource>,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub provenance: Vec<ProvenanceRecord>,
}

impl ImportDocument {
    /// Creates provisional Weftext-owned IR and computes its exact revision.
    ///
    /// # Errors
    ///
    /// Returns an error when revision material cannot be serialized.
    pub fn create(
        document_id: impl Into<String>,
        source_digest: Sha256Digest,
        title: impl Into<String>,
        nodes: Vec<ImportNode>,
        resources: Vec<ImportResource>,
        diagnostics: Vec<ImportDiagnostic>,
        provenance: Vec<ProvenanceRecord>,
    ) -> Result<Self, ImportError> {
        let mut document = Self {
            contract_version: IMPORT_IR_CONTRACT_VERSION.to_owned(),
            document_id: document_id.into(),
            revision: sha256_bytes(b"pending"),
            source_digest,
            title: title.into(),
            nodes,
            resources,
            diagnostics,
            provenance,
        };
        document.revision = document.compute_revision()?;
        Ok(document)
    }

    /// Recomputes the exact content revision after a typed mutation.
    ///
    /// # Errors
    ///
    /// Returns an error when revision material cannot be serialized.
    pub fn recompute_revision(&mut self) -> Result<(), ImportError> {
        self.revision = self.compute_revision()?;
        Ok(())
    }

    /// Computes the digest of all IR authority except the revision field itself.
    ///
    /// # Errors
    ///
    /// Returns an error when revision material cannot be serialized.
    pub fn compute_revision(&self) -> Result<Sha256Digest, ImportError> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct RevisionMaterial<'a> {
            contract_version: &'a str,
            document_id: &'a str,
            source_digest: &'a Sha256Digest,
            title: &'a str,
            nodes: &'a [ImportNode],
            resources: &'a [ImportResource],
            diagnostics: &'a [ImportDiagnostic],
            provenance: &'a [ProvenanceRecord],
        }

        let bytes = serde_json::to_vec(&RevisionMaterial {
            contract_version: &self.contract_version,
            document_id: &self.document_id,
            source_digest: &self.source_digest,
            title: &self.title,
            nodes: &self.nodes,
            resources: &self.resources,
            diagnostics: &self.diagnostics,
            provenance: &self.provenance,
        })
        .map_err(|error| ImportError::serialization(&error))?;
        Ok(sha256_bytes(&bytes))
    }

    /// Validates IR identity, provenance, structure, resources, and limits.
    ///
    /// # Errors
    ///
    /// Returns an error for stale, forged, malformed, or over-limit IR.
    pub fn validate(&self, source: &SourceArtifact, plan: &ImportPlan) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            IMPORT_IR_CONTRACT_VERSION,
            "import IR",
        )?;
        validate_identifier(&self.document_id, "IR document id")?;
        if self.source_digest != source.sha256 || self.source_digest != plan.source_digest {
            return invalid_ir("IR source digest does not match its source and plan");
        }
        if self.revision != self.compute_revision()? {
            return invalid_ir("IR revision does not match its canonical serialized content");
        }
        validate_text(&self.title, "IR title", plan.limits.max_text_bytes)?;
        if self.title.trim().is_empty() {
            return invalid_ir("IR title must not be empty");
        }
        validate_diagnostics(&self.diagnostics, &plan.limits)?;
        for diagnostic in &self.diagnostics {
            if let Some(location) = &diagnostic.source_location {
                location.validate(source, &plan.limits)?;
            }
        }
        validate_provenance(&self.provenance, source, &plan.limits)?;

        let mut node_ids = BTreeSet::new();
        let mut node_count = 0_u64;
        let mut text_bytes = usize_to_u64(self.title.len());
        validate_nodes(
            &self.nodes,
            1,
            &mut node_ids,
            &mut node_count,
            &mut text_bytes,
            source,
            &plan.limits,
        )?;
        plan.limits.check(
            "IR node count",
            node_count,
            u64::from(plan.limits.max_ir_nodes),
        )?;
        plan.limits
            .check("IR text bytes", text_bytes, plan.limits.max_text_bytes)?;

        plan.limits.check(
            "IR resource count",
            usize_to_u64(self.resources.len()),
            u64::from(plan.limits.max_resource_count),
        )?;
        let mut resource_ids = BTreeSet::new();
        let mut resource_paths = BTreeSet::new();
        let mut resource_total = 0_u64;
        for resource in &self.resources {
            validate_identifier(&resource.id, "resource id")?;
            if !resource_ids.insert(resource.id.as_str()) {
                return invalid_ir("IR resource identifiers must be unique");
            }
            if !resource_paths.insert(resource.locator.as_str()) {
                return invalid_ir("IR resource locators must be unique");
            }
            if resource.byte_length != usize_to_u64(resource.bytes.len())
                || resource.sha256 != sha256_bytes(&resource.bytes)
            {
                return invalid_ir("IR resource length or digest does not match its bytes");
            }
            plan.limits.check(
                "resource byte length",
                resource.byte_length,
                plan.limits.max_resource_bytes,
            )?;
            resource_total = resource_total
                .checked_add(resource.byte_length)
                .ok_or_else(|| {
                    ImportError::new(
                        ImportErrorCode::LimitExceeded,
                        "resource byte total overflowed",
                    )
                })?;
            validate_media_type(&resource.media_type)?;
            for location in &resource.source_locations {
                location.validate(source, &plan.limits)?;
            }
            validate_provenance(&resource.provenance, source, &plan.limits)?;
        }
        plan.limits.check(
            "total IR resource bytes",
            resource_total,
            plan.limits.max_total_output_bytes,
        )?;
        validate_figure_resources(&self.nodes, &resource_ids)?;
        Ok(())
    }

    #[must_use]
    pub fn node_count(&self) -> usize {
        fn count(nodes: &[ImportNode]) -> usize {
            nodes
                .iter()
                .map(|node| {
                    1 + match &node.kind {
                        ImportNodeKind::Section { children, .. } => count(children),
                        _ => 0,
                    }
                })
                .sum()
        }
        count(&self.nodes)
    }

    #[must_use]
    pub fn contains_node(&self, id: &str) -> bool {
        fn contains(nodes: &[ImportNode], id: &str) -> bool {
            nodes.iter().any(|node| {
                node.id == id
                    || matches!(
                        &node.kind,
                        ImportNodeKind::Section { children, .. } if contains(children, id)
                    )
            })
        }
        contains(&self.nodes, id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedResource {
    pub locator: PortablePath,
    pub source_locator: Option<PortablePath>,
    pub media_type: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub bytes: Vec<u8>,
    pub embedded: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedNode {
    pub locator: PortablePath,
    pub node_id: String,
    pub document_file: String,
    pub exact_asciidoc: String,
    pub document_sha256: Sha256Digest,
    pub resource_references: Vec<PortablePath>,
    pub resources: Vec<ProposedResource>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportProposal {
    pub contract_version: String,
    pub proposal_id: String,
    pub source_digest: Sha256Digest,
    pub base_ir_revision: Sha256Digest,
    pub destination: PortablePath,
    pub nodes: Vec<ProposedNode>,
    pub conflicts: Vec<String>,
    pub warnings: Vec<String>,
    pub omissions: Vec<String>,
}

impl ImportProposal {
    pub(crate) fn create(
        source_digest: Sha256Digest,
        base_ir_revision: Sha256Digest,
        destination: PortablePath,
        nodes: Vec<ProposedNode>,
        conflicts: Vec<String>,
        warnings: Vec<String>,
        omissions: Vec<String>,
    ) -> Result<Self, ImportError> {
        let material = serde_json::to_vec(&(
            &source_digest,
            &base_ir_revision,
            &destination,
            &nodes,
            &conflicts,
            &warnings,
            &omissions,
        ))
        .map_err(|error| ImportError::serialization(&error))?;
        let digest = sha256_bytes(&material);
        Ok(Self {
            contract_version: IMPORT_PROPOSAL_CONTRACT_VERSION.to_owned(),
            proposal_id: format!("proposal-{}", &digest.as_str()[..24]),
            source_digest,
            base_ir_revision,
            destination,
            nodes,
            conflicts,
            warnings,
            omissions,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedProposal {
    proposal: ImportProposal,
    proposal_digest: Sha256Digest,
}

impl ValidatedProposal {
    pub(crate) fn new(proposal: ImportProposal, proposal_digest: Sha256Digest) -> Self {
        Self {
            proposal,
            proposal_digest,
        }
    }

    #[must_use]
    pub fn proposal(&self) -> &ImportProposal {
        &self.proposal
    }

    #[must_use]
    pub fn proposal_digest(&self) -> &Sha256Digest {
        &self.proposal_digest
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentVersion {
    pub component_id: String,
    pub version: String,
    pub artifact_digest: Option<Sha256Digest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum CommitResult {
    PreviewOnly,
    Committed {
        transaction_id: String,
        workspace_revision: String,
    },
    Cancelled,
    Failed {
        error_code: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReceipt {
    pub contract_version: String,
    pub receipt_id: String,
    pub created_at: String,
    pub source_digest: Sha256Digest,
    pub output_digests: Vec<Sha256Digest>,
    pub proposal_digest: Sha256Digest,
    pub ir_revision: Sha256Digest,
    pub components: Vec<ComponentVersion>,
    pub plan: ImportPlan,
    pub local_provenance: Vec<ProvenanceRecord>,
    pub agent_provenance: Vec<ProvenanceRecord>,
    pub egress: EgressDisclosure,
    pub warnings: Vec<String>,
    pub commit_result: CommitResult,
}

impl ImportReceipt {
    /// Records exact source, IR, proposal, component, egress, and commit evidence.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid timestamp or unserializable receipt material.
    pub fn create(
        created_at: impl Into<String>,
        source: &SourceArtifact,
        plan: &ImportPlan,
        document: &ImportDocument,
        proposal: &ValidatedProposal,
        components: Vec<ComponentVersion>,
        commit_result: CommitResult,
    ) -> Result<Self, ImportError> {
        let created_at = created_at.into();
        validate_explicit_offset_timestamp(&created_at)?;
        if source.sha256 != plan.source_digest
            || source.sha256 != document.source_digest
            || source.sha256 != proposal.proposal().source_digest
            || document.revision != proposal.proposal().base_ir_revision
        {
            return invalid_contract(
                "receipt source, plan, IR, and validated proposal authority do not agree",
            );
        }
        if matches!(&commit_result, CommitResult::Committed { .. })
            && !proposal.proposal().conflicts.is_empty()
        {
            return invalid_contract("a proposal with blocking conflicts cannot be committed");
        }
        if components.len() > 128 {
            return invalid_contract("receipt component count exceeds its bounded contract");
        }
        let mut component_ids = BTreeSet::new();
        for component in &components {
            validate_identifier(&component.component_id, "receipt component id")?;
            validate_version_label(&component.version, "receipt component version")?;
            if !component_ids.insert(component.component_id.as_str()) {
                return invalid_contract("receipt component identifiers must be unique");
            }
        }
        if let CommitResult::Committed {
            transaction_id,
            workspace_revision,
        } = &commit_result
        {
            validate_identifier(transaction_id, "commit transaction id")?;
            validate_identifier(workspace_revision, "committed workspace revision")?;
        }
        if let CommitResult::Failed { error_code } = &commit_result {
            validate_identifier(error_code, "commit failure code")?;
        }
        let mut output_digests = Vec::new();
        for node in &proposal.proposal().nodes {
            output_digests.push(node.document_sha256.clone());
            output_digests.extend(
                node.resources
                    .iter()
                    .map(|resource| resource.sha256.clone()),
            );
        }
        let all_provenance = collect_document_provenance(document)?;
        let local_provenance = all_provenance
            .iter()
            .filter(|record| record.kind != ProvenanceKind::AgentEnhancement)
            .cloned()
            .collect::<Vec<_>>();
        let agent_provenance = all_provenance
            .iter()
            .filter(|record| record.kind == ProvenanceKind::AgentEnhancement)
            .cloned()
            .collect::<Vec<_>>();
        let warnings = proposal.proposal().warnings.clone();
        let material = serde_json::to_vec(&(
            &created_at,
            &source.sha256,
            proposal.proposal_digest(),
            &document.revision,
            &output_digests,
            &components,
            plan,
            &local_provenance,
            &agent_provenance,
            &plan.egress,
            &warnings,
            &commit_result,
        ))
        .map_err(|error| ImportError::serialization(&error))?;
        let receipt_digest = sha256_bytes(&material);
        Ok(Self {
            contract_version: IMPORT_RECEIPT_CONTRACT_VERSION.to_owned(),
            receipt_id: format!("receipt-{}", &receipt_digest.as_str()[..24]),
            created_at,
            source_digest: source.sha256.clone(),
            output_digests,
            proposal_digest: proposal.proposal_digest().clone(),
            ir_revision: document.revision.clone(),
            components,
            plan: plan.clone(),
            local_provenance,
            agent_provenance,
            egress: plan.egress.clone(),
            warnings,
            commit_result,
        })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerNetworkPolicy {
    Denied,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkerRequest {
    pub contract_version: String,
    pub request_id: String,
    pub worker_id: String,
    pub worker_protocol_version: String,
    pub source: SourceArtifact,
    pub source_locator: PortablePath,
    pub plan: ImportPlan,
    pub network: WorkerNetworkPolicy,
    pub memory_limit_bytes: u64,
    pub page_limit: u32,
    pub entry_limit: u32,
    pub output_byte_limit: u64,
    pub format_options: Value,
}

impl WorkerRequest {
    pub(crate) fn validate(&self) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            WORKER_REQUEST_CONTRACT_VERSION,
            "worker request",
        )?;
        validate_identifier(&self.request_id, "worker request id")?;
        validate_identifier(&self.worker_id, "worker id")?;
        if self.worker_id != self.plan.route.worker_id
            || self.worker_protocol_version != self.plan.route.worker_protocol_version
            || self.source.sha256 != self.plan.source_digest
            || self.memory_limit_bytes != self.plan.limits.worker_memory_bytes
            || self.page_limit != self.plan.limits.max_pages
            || self.entry_limit != self.plan.limits.max_container_entries
            || self.output_byte_limit != self.plan.limits.max_total_output_bytes
        {
            return invalid_contract("worker request does not match its immutable import plan");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkerResource {
    pub locator: PortablePath,
    pub media_type: String,
    pub byte_length: u64,
    pub sha256: Sha256Digest,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct WorkerResponse {
    pub contract_version: String,
    pub request_id: String,
    pub worker_id: String,
    pub worker_protocol_version: String,
    pub source_digest: Sha256Digest,
    pub payload: Value,
    pub resources: Vec<WorkerResource>,
    pub diagnostics: Vec<ImportDiagnostic>,
    pub components: Vec<ComponentVersion>,
}

impl WorkerResponse {
    pub(crate) fn validate(
        &self,
        request: &WorkerRequest,
        limits: &ImportLimits,
    ) -> Result<(), ImportError> {
        require_version(
            &self.contract_version,
            WORKER_RESPONSE_CONTRACT_VERSION,
            "worker response",
        )?;
        if self.request_id != request.request_id
            || self.worker_id != request.worker_id
            || self.worker_protocol_version != request.worker_protocol_version
            || self.source_digest != request.source.sha256
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "worker response identity does not match its request",
            ));
        }
        let payload = serde_json::to_vec(&self.payload)
            .map_err(|error| ImportError::serialization(&error))?;
        limits.check(
            "worker JSON payload bytes",
            usize_to_u64(payload.len()),
            limits.max_total_output_bytes,
        )?;
        limits.check(
            "worker resource count",
            usize_to_u64(self.resources.len()),
            u64::from(limits.max_resource_count),
        )?;
        let mut total = usize_to_u64(payload.len());
        let mut paths = BTreeSet::new();
        for resource in &self.resources {
            if !paths.insert(resource.locator.as_str()) {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "worker resource locators must be unique",
                ));
            }
            if resource.byte_length != usize_to_u64(resource.bytes.len())
                || resource.sha256 != sha256_bytes(&resource.bytes)
            {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "worker resource length or digest does not match its bytes",
                ));
            }
            limits.check(
                "worker resource bytes",
                resource.byte_length,
                limits.max_resource_bytes,
            )?;
            total = total.checked_add(resource.byte_length).ok_or_else(|| {
                ImportError::new(
                    ImportErrorCode::LimitExceeded,
                    "worker output total overflowed",
                )
            })?;
            validate_media_type(&resource.media_type)?;
        }
        limits.check(
            "worker total output bytes",
            total,
            limits.max_total_output_bytes,
        )?;
        validate_diagnostics(&self.diagnostics, limits)?;
        for diagnostic in &self.diagnostics {
            if let Some(location) = &diagnostic.source_location {
                location.validate(&request.source, limits)?;
            }
        }
        let mut component_ids = BTreeSet::new();
        for component in &self.components {
            validate_identifier(&component.component_id, "worker component id")?;
            validate_version_label(&component.version, "worker component version")?;
            if !component_ids.insert(component.component_id.as_str()) {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    "worker component identifiers must be unique",
                ));
            }
        }
        Ok(())
    }
}

pub struct WorkerContext {
    session_root: PathBuf,
    cancellation: CancellationToken,
    deadline: Instant,
}

impl WorkerContext {
    pub(crate) fn new(
        session_root: PathBuf,
        cancellation: CancellationToken,
        deadline: Instant,
    ) -> Self {
        Self {
            session_root,
            cancellation,
            deadline,
        }
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled() || Instant::now() >= self.deadline
    }

    /// Reads one regular file beneath the isolated temporary session.
    ///
    /// # Errors
    ///
    /// Returns an error for cancellation, escape attempts, links, non-files,
    /// filesystem failures, or an over-limit input.
    pub fn read_bounded(
        &self,
        locator: &PortablePath,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, ImportError> {
        if self.is_cancelled() {
            return Err(ImportError::new(
                ImportErrorCode::Cancelled,
                "worker request was cancelled",
            ));
        }
        let path = resolve_temp_locator(&self.session_root, locator)?;
        let metadata = fs::symlink_metadata(&path)
            .map_err(|error| ImportError::io("inspect worker input", &error))?;
        if !metadata.is_file() || is_symlink_or_reparse(&metadata) {
            return Err(ImportError::new(
                ImportErrorCode::TemporaryStorage,
                "worker input must be a regular non-link file",
            ));
        }
        if metadata.len() > maximum_bytes {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "worker input exceeds its bounded read limit",
            ));
        }
        fs::read(path).map_err(|error| ImportError::io("read worker input", &error))
    }

    #[must_use]
    pub fn session_root(&self) -> &Path {
        &self.session_root
    }
}

pub trait ImportAdapter: Send + Sync {
    fn descriptor(&self) -> AdapterDescriptor;

    /// Inspects only exact ranges supplied by the common bounded evidence reader.
    ///
    /// # Errors
    ///
    /// Returns an error when evidence cannot be safely classified.
    fn probe(
        &self,
        source: &SourceArtifact,
        bounded_evidence: &mut ProbeReader<'_>,
        limits: &ImportLimits,
    ) -> Result<FormatProbe, ImportError>;

    /// Freezes the selected adapter route and reviewed policies.
    ///
    /// # Errors
    ///
    /// Returns an error when the probe or requested route cannot be planned.
    fn plan(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
        request: PlanRequest,
        limits: ImportLimits,
    ) -> Result<ImportPlan, ImportError>;

    /// Creates a worker request that contains only temporary relative locators.
    ///
    /// # Errors
    ///
    /// Returns an error when a bounded worker request cannot be constructed.
    fn worker_request(
        &self,
        source: &SourceArtifact,
        plan: &ImportPlan,
        source_locator: PortablePath,
    ) -> Result<WorkerRequest, ImportError>;

    /// Maps worker-internal output into Weftext-owned provisional IR.
    ///
    /// # Errors
    ///
    /// Returns an error for malformed, unsupported, or over-limit worker output.
    fn map_worker_response(
        &self,
        source: &SourceArtifact,
        plan: &ImportPlan,
        response: WorkerResponse,
    ) -> Result<ImportDocument, ImportError>;
}

/// Supervised format-worker boundary.
///
/// Production implementations must bridge to a separately sandboxed process
/// that enforces the request's memory, filesystem, network, and process limits.
/// An in-process implementation is suitable only for deterministic test fakes.
pub trait FormatWorker: Send + Sync + 'static {
    fn worker_id(&self) -> &str;
    fn protocol_version(&self) -> &str;

    /// Executes conversion inside the supplied cancellation, deadline, and temp boundary.
    ///
    /// # Errors
    ///
    /// Returns an error for cancellation or any worker/protocol failure.
    fn execute(
        &self,
        request: WorkerRequest,
        context: WorkerContext,
    ) -> Result<WorkerResponse, ImportError>;
}

/// Revalidates the complete immutable source/probe/plan/IR authority without
/// invoking an adapter or worker again.
///
/// # Errors
///
/// Returns an error for any stale digest, route, policy, limit, provenance, or
/// IR inconsistency.
pub fn validate_import_authority(
    source: &SourceArtifact,
    probe: &FormatProbe,
    plan: &ImportPlan,
    document: &ImportDocument,
) -> Result<(), ImportError> {
    source.validate(&plan.limits)?;
    probe.validate(source, &plan.limits)?;
    plan.validate(source, probe)?;
    document.validate(source, plan)
}

fn validate_nodes<'a>(
    nodes: &'a [ImportNode],
    depth: u16,
    ids: &mut BTreeSet<&'a str>,
    count: &mut u64,
    text_bytes: &mut u64,
    source: &SourceArtifact,
    limits: &ImportLimits,
) -> Result<(), ImportError> {
    if depth > limits.max_ir_depth {
        return Err(ImportError::new(
            ImportErrorCode::LimitExceeded,
            "IR nesting exceeds the configured depth limit",
        ));
    }
    for node in nodes {
        validate_identifier(&node.id, "IR node id")?;
        if !ids.insert(&node.id) {
            return invalid_ir("IR node identifiers must be unique");
        }
        *count = count.checked_add(1).ok_or_else(|| {
            ImportError::new(ImportErrorCode::LimitExceeded, "IR node count overflowed")
        })?;
        for location in &node.source_locations {
            location.validate(source, limits)?;
        }
        validate_provenance(&node.provenance, source, limits)?;
        match &node.kind {
            ImportNodeKind::Section {
                level,
                title,
                children,
            } => {
                if *level == 0 || *level > 8 {
                    return invalid_ir("IR section levels must be between 1 and 8");
                }
                add_text_bytes(text_bytes, title)?;
                validate_nodes(
                    children,
                    depth.saturating_add(1),
                    ids,
                    count,
                    text_bytes,
                    source,
                    limits,
                )?;
            }
            ImportNodeKind::Paragraph { text } => add_text_bytes(text_bytes, text)?,
            ImportNodeKind::Quote { depth, text } => {
                if *depth == 0 || *depth > 9 {
                    return invalid_ir("IR quotation depth must be between 1 and 9");
                }
                add_text_bytes(text_bytes, text)?;
            }
            ImportNodeKind::Listing { language, source } => {
                validate_listing_language(language.as_deref())?;
                add_text_bytes(text_bytes, source)?;
            }
            ImportNodeKind::ThematicBreak => {}
            ImportNodeKind::List { items, .. } => {
                if items.is_empty() {
                    return invalid_ir("IR lists must contain at least one item");
                }
                for item in items {
                    add_text_bytes(text_bytes, item)?;
                }
            }
            ImportNodeKind::Table { header_rows, rows } => {
                if rows.is_empty() || rows.iter().any(Vec::is_empty) {
                    return invalid_ir("IR tables must contain non-empty rows");
                }
                let width = rows[0].len();
                if rows.iter().any(|row| row.len() != width)
                    || usize::from(*header_rows) > rows.len()
                {
                    return invalid_ir("IR table dimensions or header row count are invalid");
                }
                for cell in rows.iter().flatten() {
                    add_text_bytes(text_bytes, cell)?;
                }
            }
            ImportNodeKind::Figure {
                resource_id,
                alt,
                caption,
            } => {
                validate_identifier(resource_id, "figure resource id")?;
                add_text_bytes(text_bytes, alt)?;
                if let Some(caption) = caption {
                    add_text_bytes(text_bytes, caption)?;
                }
            }
            ImportNodeKind::Formula { notation, source } => {
                validate_identifier(notation, "formula notation")?;
                add_text_bytes(text_bytes, source)?;
            }
            ImportNodeKind::Link { target, label } => {
                add_text_bytes(text_bytes, target)?;
                add_text_bytes(text_bytes, label)?;
            }
        }
    }
    Ok(())
}

fn validate_listing_language(language: Option<&str>) -> Result<(), ImportError> {
    if language.is_some_and(|language| {
        language.is_empty()
            || language.len() > 64
            || !language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'+' | b'-'))
    }) {
        return invalid_ir("IR listing language is invalid");
    }
    Ok(())
}

fn validate_figure_resources<'a>(
    nodes: &'a [ImportNode],
    resource_ids: &BTreeSet<&'a str>,
) -> Result<(), ImportError> {
    for node in nodes {
        match &node.kind {
            ImportNodeKind::Section { children, .. } => {
                validate_figure_resources(children, resource_ids)?;
            }
            ImportNodeKind::Figure { resource_id, .. }
                if !resource_ids.contains(resource_id.as_str()) =>
            {
                return invalid_ir("an IR figure refers to a missing resource");
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_provenance(
    records: &[ProvenanceRecord],
    source: &SourceArtifact,
    limits: &ImportLimits,
) -> Result<(), ImportError> {
    if records.len() > 1_000 {
        return invalid_ir("provenance record count exceeds the contract limit");
    }
    for record in records {
        validate_identifier(&record.component_id, "provenance component id")?;
        validate_version_label(&record.component_version, "provenance component version")?;
        if record.input_digests.len() > 32 || record.source_locations.len() > 1_000 {
            return invalid_ir("a provenance record exceeds its input or location limit");
        }
        for location in &record.source_locations {
            location.validate(source, limits)?;
        }
    }
    Ok(())
}

fn collect_document_provenance(
    document: &ImportDocument,
) -> Result<Vec<ProvenanceRecord>, ImportError> {
    fn push_unique(
        output: &mut Vec<ProvenanceRecord>,
        seen: &mut BTreeSet<Sha256Digest>,
        record: &ProvenanceRecord,
    ) -> Result<(), ImportError> {
        let bytes =
            serde_json::to_vec(record).map_err(|error| ImportError::serialization(&error))?;
        if seen.insert(sha256_bytes(&bytes)) {
            output.push(record.clone());
        }
        Ok(())
    }

    fn visit_nodes(
        nodes: &[ImportNode],
        output: &mut Vec<ProvenanceRecord>,
        seen: &mut BTreeSet<Sha256Digest>,
    ) -> Result<(), ImportError> {
        for node in nodes {
            for record in &node.provenance {
                push_unique(output, seen, record)?;
            }
            if let ImportNodeKind::Section { children, .. } = &node.kind {
                visit_nodes(children, output, seen)?;
            }
        }
        Ok(())
    }

    let mut output = Vec::new();
    let mut seen = BTreeSet::new();
    for record in &document.provenance {
        push_unique(&mut output, &mut seen, record)?;
    }
    visit_nodes(&document.nodes, &mut output, &mut seen)?;
    for resource in &document.resources {
        for record in &resource.provenance {
            push_unique(&mut output, &mut seen, record)?;
        }
    }
    Ok(output)
}

fn validate_diagnostics(
    diagnostics: &[ImportDiagnostic],
    limits: &ImportLimits,
) -> Result<(), ImportError> {
    limits.check(
        "diagnostic count",
        usize_to_u64(diagnostics.len()),
        u64::from(limits.max_diagnostics),
    )?;
    for diagnostic in diagnostics {
        validate_identifier(&diagnostic.code, "diagnostic code")?;
        if diagnostic.message.is_empty() || diagnostic.message.len() > 4_096 {
            return invalid_contract("diagnostic messages must contain 1 through 4,096 bytes");
        }
        if let Some(id) = &diagnostic.ir_node_id {
            validate_identifier(id, "diagnostic IR node id")?;
        }
    }
    Ok(())
}

fn validate_evidence(values: &[String], label: &str) -> Result<(), ImportError> {
    if values.len() > 64
        || values
            .iter()
            .any(|value| value.is_empty() || value.len() > 1_024)
    {
        return invalid_contract(&format!("{label} is outside its bounded contract"));
    }
    Ok(())
}

fn validate_media_type(value: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 127
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'+' | b'.' | b'-'))
        || !value.contains('/')
    {
        return invalid_contract("media types must use a bounded ASCII type/subtype spelling");
    }
    Ok(())
}

fn validate_text(value: &str, label: &str, maximum: u64) -> Result<(), ImportError> {
    if value.contains('\0')
        || value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return invalid_ir(&format!("{label} contains a forbidden control character"));
    }
    if usize_to_u64(value.len()) > maximum {
        return Err(ImportError::new(
            ImportErrorCode::LimitExceeded,
            format!("{label} exceeds its byte limit"),
        ));
    }
    Ok(())
}

fn add_text_bytes(total: &mut u64, value: &str) -> Result<(), ImportError> {
    validate_text(value, "IR text", u64::MAX)?;
    *total = total
        .checked_add(usize_to_u64(value.len()))
        .ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::LimitExceeded,
                "IR text byte total overflowed",
            )
        })?;
    Ok(())
}

fn validate_source_name(value: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 255
        || value.contains(['/', '\\', '\0'])
        || value == "."
        || value == ".."
        || value.chars().any(char::is_control)
    {
        return Err(ImportError::new(
            ImportErrorCode::InvalidSource,
            "source display names must be one bounded, non-special filename",
        ));
    }
    Ok(())
}

pub(crate) fn validate_identifier(value: &str, label: &str) -> Result<(), ImportError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return invalid_contract(&format!(
            "{label} has an invalid bounded identifier spelling"
        ));
    }
    Ok(())
}

fn validate_version_label(value: &str, label: &str) -> Result<(), ImportError> {
    validate_identifier(value, label)
}

pub(crate) fn require_version(
    actual: &str,
    expected: &str,
    label: &str,
) -> Result<(), ImportError> {
    if actual != expected {
        return invalid_contract(&format!(
            "{label} version `{actual}` is unsupported; expected `{expected}`"
        ));
    }
    Ok(())
}

fn validate_explicit_offset_timestamp(value: &str) -> Result<(), ImportError> {
    let bytes = value.as_bytes();
    if !value.is_ascii()
        || bytes.len() < 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
    {
        return invalid_contract("receipt timestamps require an explicit RFC 3339 offset");
    }
    let year = parse_fixed_decimal(bytes, 0, 4)?;
    let month = parse_fixed_decimal(bytes, 5, 2)?;
    let day = parse_fixed_decimal(bytes, 8, 2)?;
    let hour = parse_fixed_decimal(bytes, 11, 2)?;
    let minute = parse_fixed_decimal(bytes, 14, 2)?;
    let second = parse_fixed_decimal(bytes, 17, 2)?;
    if year == 0
        || !(1..=12).contains(&month)
        || day == 0
        || day > days_in_month(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return invalid_contract("receipt timestamp has an invalid calendar or clock value");
    }

    let mut offset_start = 19;
    if bytes.get(offset_start) == Some(&b'.') {
        offset_start += 1;
        let fraction_start = offset_start;
        while bytes.get(offset_start).is_some_and(u8::is_ascii_digit) {
            offset_start += 1;
        }
        if offset_start == fraction_start || offset_start - fraction_start > 9 {
            return invalid_contract(
                "receipt timestamp fractions require 1 through 9 decimal digits",
            );
        }
    }
    match bytes.get(offset_start) {
        Some(b'Z') if offset_start + 1 == bytes.len() => Ok(()),
        Some(b'+' | b'-')
            if offset_start + 6 == bytes.len() && bytes.get(offset_start + 3) == Some(&b':') =>
        {
            let offset_hour = parse_fixed_decimal(bytes, offset_start + 1, 2)?;
            let offset_minute = parse_fixed_decimal(bytes, offset_start + 4, 2)?;
            if offset_hour <= 23 && offset_minute <= 59 {
                Ok(())
            } else {
                invalid_contract("receipt timestamp offset is out of range")
            }
        }
        _ => invalid_contract("receipt timestamps require an explicit RFC 3339 offset"),
    }
}

fn parse_fixed_decimal(bytes: &[u8], start: usize, length: usize) -> Result<u32, ImportError> {
    let end = start.saturating_add(length);
    let Some(slice) = bytes.get(start..end) else {
        return invalid_contract("receipt timestamp is truncated");
    };
    if !slice.iter().all(u8::is_ascii_digit) {
        return invalid_contract("receipt timestamp contains a non-decimal date/time field");
    }
    Ok(slice
        .iter()
        .fold(0_u32, |value, digit| value * 10 + u32::from(*digit - b'0')))
}

const fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

const fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn resolve_temp_locator(root: &Path, locator: &PortablePath) -> Result<PathBuf, ImportError> {
    let mut path = root.to_path_buf();
    for component in locator.as_str().split('/') {
        path.push(component);
    }
    let parent = path.parent().ok_or_else(|| {
        ImportError::new(
            ImportErrorCode::TemporaryStorage,
            "worker locator has no temporary parent",
        )
    })?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| ImportError::io("resolve temporary root", &error))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| ImportError::io("resolve worker input parent", &error))?;
    if canonical_parent != canonical_root && !canonical_parent.starts_with(&canonical_root) {
        return Err(ImportError::new(
            ImportErrorCode::TemporaryStorage,
            "worker locator escapes the temporary session",
        ));
    }
    Ok(path)
}

fn is_symlink_or_reparse(metadata: &fs::Metadata) -> bool {
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

fn usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn invalid_contract<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(ImportErrorCode::InvalidContract, message))
}

fn invalid_ir<T>(message: &str) -> Result<T, ImportError> {
    Err(ImportError::new(ImportErrorCode::InvalidIr, message))
}

#[cfg(test)]
mod tests {
    use super::{
        AgentEnhancementSelection, Confidence, EgressDisclosure, ImportDocument, ImportNode,
        ImportNodeKind, ImportResource, OriginClass, SourceArtifact,
    };
    use crate::{
        FakeAdapter, ImportAdapter, ImportErrorCode, ImportLimits, PlanRequest, PortablePath,
    };

    #[test]
    fn source_and_ir_count_limits_fail_closed() {
        let source_limits = ImportLimits {
            max_source_bytes: 8,
            max_probe_bytes: 8,
            ..ImportLimits::default()
        };
        let error = SourceArtifact::from_bytes(
            "large.fake",
            OriginClass::TestFixture,
            b"123456789",
            &source_limits,
        )
        .expect_err("oversized source");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);

        let limits = ImportLimits {
            max_ir_nodes: 1,
            ..ImportLimits::default()
        };
        let bytes = b"WEFTEXT-FAKE/1\nTitle\nOne\nTwo\n";
        let source =
            SourceArtifact::from_bytes("two.fake", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, bytes, &limits).expect("probe");
        let plan = adapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Target").expect("path")),
                limits,
            )
            .expect("plan");
        let nodes = ["One", "Two"]
            .into_iter()
            .enumerate()
            .map(|(index, text)| ImportNode {
                id: format!("paragraph-{}", index + 1),
                kind: ImportNodeKind::Paragraph {
                    text: text.to_owned(),
                },
                confidence: Confidence::from_basis_points(10_000).expect("confidence"),
                source_locations: Vec::new(),
                provenance: Vec::new(),
            })
            .collect();
        let document = ImportDocument::create(
            "document-count-test",
            source.sha256.clone(),
            "Title",
            nodes,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");

        let error = document
            .validate(&source, &plan)
            .expect_err("node limit must reject");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);
    }

    #[test]
    fn plan_mints_fresh_root_identity_and_binds_it_to_plan_material() {
        let bytes = b"WEFTEXT-FAKE/1\nTitle\n";
        let limits = ImportLimits::default();
        let source =
            SourceArtifact::from_bytes("identity.fake", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, bytes, &limits).expect("probe");
        let request =
            || PlanRequest::single_node(PortablePath::parse("Target").expect("destination path"));
        let first = adapter
            .plan(&source, &probe, request(), limits.clone())
            .expect("first plan");
        let second = adapter
            .plan(&source, &probe, request(), limits)
            .expect("second plan");

        assert_ne!(first.proposed_root_id, second.proposed_root_id);
        let parsed = uuid::Uuid::parse_str(&first.proposed_root_id).expect("planned UUID");
        assert_eq!(parsed.get_version_num(), 4);
        assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122);

        let mut forged = first;
        forged.proposed_root_id = uuid::Uuid::new_v4().to_string();
        let error = forged
            .validate(&source, &probe)
            .expect_err("mutated plan identity");
        assert_eq!(error.code(), ImportErrorCode::InvalidContract);
    }

    #[test]
    fn post_extraction_agent_plan_preserves_identity_and_freezes_exact_egress() {
        let bytes = b"WEFTEXT-FAKE/1\nTitle\nParagraph\n";
        let limits = ImportLimits::default();
        let source =
            SourceArtifact::from_bytes("agent.fake", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, bytes, &limits).expect("probe");
        let local = adapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Target").expect("path")),
                limits,
            )
            .expect("local plan");
        let authorized = local
            .authorize_agent_enhancement(
                &source,
                &probe,
                AgentEnhancementSelection {
                    provider: "reviewed-provider".to_owned(),
                    selected_node_ids: vec!["paragraph-1".to_owned()],
                    disclosed_bytes: 512,
                    retention: "delete-after-call".to_owned(),
                    redaction: "none".to_owned(),
                },
            )
            .expect("derived plan");

        assert_eq!(authorized.proposed_root_id, local.proposed_root_id);
        assert_ne!(authorized.plan_id, local.plan_id);
        assert!(matches!(local.egress, EgressDisclosure::None));
        assert!(matches!(
            &authorized.egress,
            EgressDisclosure::AgentSelectedEvidence {
                provider,
                selected_node_ids,
                disclosed_bytes: 512,
                ..
            } if provider == "reviewed-provider" && selected_node_ids == &["paragraph-1"]
        ));
        let error = authorized
            .authorize_agent_enhancement(
                &source,
                &probe,
                AgentEnhancementSelection {
                    provider: "other".to_owned(),
                    selected_node_ids: vec!["paragraph-1".to_owned()],
                    disclosed_bytes: 1,
                    retention: "delete".to_owned(),
                    redaction: "none".to_owned(),
                },
            )
            .expect_err("egress cannot be silently re-authorized");
        assert_eq!(error.code(), ImportErrorCode::InvalidContract);
    }

    #[test]
    fn forged_digest_deserialization_is_rejected() {
        let value = serde_json::json!({
            "contractVersion": crate::SOURCE_ARTIFACT_CONTRACT_VERSION,
            "sourceId": "source-forged",
            "displayName": "input.fake",
            "origin": "test_fixture",
            "byteLength": 1,
            "sha256": "ABC",
            "extensionHint": "fake",
            "detectedFormat": "fake_fixture",
            "mismatchEvidence": []
        });
        assert!(serde_json::from_value::<SourceArtifact>(value).is_err());
        assert!(serde_json::from_value::<Confidence>(serde_json::json!(10_001)).is_err());
    }

    #[test]
    fn resource_byte_and_count_limits_are_enforced_before_preview() {
        let bytes = b"WEFTEXT-FAKE/1\nTitle\n";
        let limits = ImportLimits {
            max_resource_count: 1,
            max_resource_bytes: 4,
            ..ImportLimits::default()
        };
        let source =
            SourceArtifact::from_bytes("resources.fake", OriginClass::TestFixture, bytes, &limits)
                .expect("source");
        let adapter = FakeAdapter;
        let probe = crate::probe_source_bytes(&adapter, &source, bytes, &limits).expect("probe");
        let plan = adapter
            .plan(
                &source,
                &probe,
                PlanRequest::single_node(PortablePath::parse("Target").expect("path")),
                limits,
            )
            .expect("plan");
        let resource_bytes = b"12345".to_vec();
        let resource = ImportResource {
            id: "resource-1".to_owned(),
            locator: PortablePath::parse("resource.bin").expect("path"),
            media_type: "application/octet-stream".to_owned(),
            byte_length: 5,
            sha256: crate::sha256_bytes(&resource_bytes),
            bytes: resource_bytes,
            source_locations: Vec::new(),
            provenance: Vec::new(),
        };
        let document = ImportDocument::create(
            "document-resource-limit",
            source.sha256.clone(),
            "Title",
            Vec::new(),
            vec![resource],
            Vec::new(),
            Vec::new(),
        )
        .expect("IR");
        let error = document
            .validate(&source, &plan)
            .expect_err("resource bytes must fail");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);

        let mut two_resources = document.clone();
        two_resources.resources[0].bytes = b"1".to_vec();
        two_resources.resources[0].byte_length = 1;
        two_resources.resources[0].sha256 = crate::sha256_bytes(b"1");
        let mut second = two_resources.resources[0].clone();
        second.id = "resource-2".to_owned();
        second.locator = PortablePath::parse("resource-2.bin").expect("path");
        two_resources.resources.push(second);
        two_resources.recompute_revision().expect("revision");
        let error = two_resources
            .validate(&source, &plan)
            .expect_err("resource count must fail");
        assert_eq!(error.code(), ImportErrorCode::LimitExceeded);
    }

    #[test]
    fn receipt_timestamp_requires_a_real_calendar_value_and_explicit_offset() {
        for valid in [
            "2026-08-24T12:00:00+08:00",
            "2024-02-29T23:59:59.123456789Z",
        ] {
            super::validate_explicit_offset_timestamp(valid).expect("valid timestamp");
        }
        for invalid in [
            "2026-08-24T12:00:00",
            "2026-02-29T12:00:00Z",
            "2026-13-01T12:00:00Z",
            "2026-01-01T24:00:00Z",
            "2026-01-01T12:00:00+24:00",
            "2026-01-01T12:00:00.Z",
        ] {
            assert!(
                super::validate_explicit_offset_timestamp(invalid).is_err(),
                "accepted invalid timestamp: {invalid}"
            );
        }
    }
}
