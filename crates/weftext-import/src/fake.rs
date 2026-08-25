use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    AdapterDescriptor, AdapterRoute, ComponentVersion, Confidence, DiagnosticSeverity,
    EncryptionState, FORMAT_PROBE_CONTRACT_VERSION, FormatProbe, FormatWorker, ImportAdapter,
    ImportDiagnostic, ImportDocument, ImportError, ImportErrorCode, ImportLimits, ImportNode,
    ImportNodeKind, ImportPlan, ImportSourceLocation, PlanRequest, PortablePath, ProbeReader,
    ProvenanceKind, ProvenanceRecord, SourceArtifact, SourceFormat,
    WORKER_REQUEST_CONTRACT_VERSION, WORKER_RESPONSE_CONTRACT_VERSION, WorkerContext,
    WorkerNetworkPolicy, WorkerRequest, WorkerResponse, sha256_bytes,
};

const SIGNATURE: &[u8] = b"WEFTEXT-FAKE/1\n";
const ADAPTER_ID: &str = "weftext.fake-adapter";
const ADAPTER_VERSION: &str = "1";
const WORKER_ID: &str = "weftext.fake-worker";
const WORKER_PROTOCOL_VERSION: &str = "weftext.fake-worker-json.v1";

#[derive(Clone, Copy, Debug, Default)]
pub struct FakeAdapter;

impl ImportAdapter for FakeAdapter {
    fn descriptor(&self) -> AdapterDescriptor {
        AdapterDescriptor {
            adapter_id: ADAPTER_ID.to_owned(),
            adapter_version: ADAPTER_VERSION.to_owned(),
            supported_format: SourceFormat::FakeFixture,
        }
    }

    fn probe(
        &self,
        source: &SourceArtifact,
        evidence_reader: &mut ProbeReader<'_>,
        limits: &ImportLimits,
    ) -> Result<FormatProbe, ImportError> {
        let bounded_evidence = evidence_reader.read_head(
            source
                .byte_length
                .min(limits.max_probe_bytes)
                .min(u64::try_from(SIGNATURE.len()).unwrap_or(u64::MAX)),
        )?;
        let signature_matches = bounded_evidence.starts_with(SIGNATURE);
        let extension_matches = source.extension_hint.as_deref() == Some("fake");
        let mut mismatch_evidence = Vec::new();
        if signature_matches && !extension_matches {
            mismatch_evidence.push(format!(
                "signature identifies fake_fixture while extension hint is {:?}",
                source.extension_hint
            ));
        }
        let diagnostics = if signature_matches {
            Vec::new()
        } else {
            vec![ImportDiagnostic {
                code: "fake_signature_missing".to_owned(),
                severity: DiagnosticSeverity::Blocking,
                message: "the fixture does not begin with the bounded fake format signature"
                    .to_owned(),
                source_location: None,
                ir_node_id: None,
            }]
        };
        Ok(FormatProbe {
            contract_version: FORMAT_PROBE_CONTRACT_VERSION.to_owned(),
            adapter: self.descriptor(),
            source_digest: source.sha256.clone(),
            evidence: evidence_reader.evidence(),
            detected_format: if signature_matches {
                SourceFormat::FakeFixture
            } else {
                SourceFormat::Unknown
            },
            signature_confidence: Confidence::from_basis_points(if signature_matches {
                10_000
            } else {
                0
            })?,
            parser_confidence: Confidence::from_basis_points(if signature_matches {
                10_000
            } else {
                0
            })?,
            encryption: EncryptionState::NotEncrypted,
            signature_evidence: if signature_matches {
                vec!["exact WEFTEXT-FAKE/1 signature".to_owned()]
            } else {
                Vec::new()
            },
            mismatch_evidence,
            active_content_detected: false,
            page_count: None,
            container_entry_count: None,
            safe_to_plan: signature_matches,
            diagnostics,
        })
    }

    fn plan(
        &self,
        source: &SourceArtifact,
        probe: &FormatProbe,
        request: PlanRequest,
        limits: ImportLimits,
    ) -> Result<ImportPlan, ImportError> {
        if probe.detected_format != SourceFormat::FakeFixture || !probe.safe_to_plan {
            return Err(ImportError::new(
                ImportErrorCode::UnsupportedFormat,
                "fake adapter only plans exact fake fixtures",
            ));
        }
        ImportPlan::create(
            source,
            probe,
            AdapterRoute {
                adapter: self.descriptor(),
                worker_id: WORKER_ID.to_owned(),
                worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
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
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            source: source.clone(),
            source_locator,
            plan: plan.clone(),
            network: WorkerNetworkPolicy::Denied,
            memory_limit_bytes: plan.limits.worker_memory_bytes,
            page_limit: plan.limits.max_pages,
            entry_limit: plan.limits.max_container_entries,
            output_byte_limit: plan.limits.max_total_output_bytes,
            format_options: json!({"fixtureDialect": "weftext.fake.v1"}),
        })
    }

    fn map_worker_response(
        &self,
        source: &SourceArtifact,
        _plan: &ImportPlan,
        response: WorkerResponse,
    ) -> Result<ImportDocument, ImportError> {
        if !response.resources.is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "fake worker protocol does not define extracted resources",
            ));
        }
        let payload: FakePayload = serde_json::from_value(response.payload).map_err(|error| {
            ImportError::new(
                ImportErrorCode::WorkerProtocol,
                format!("fake worker payload is invalid: {error}"),
            )
        })?;
        if payload.title.trim().is_empty() {
            return Err(ImportError::new(
                ImportErrorCode::InvalidIr,
                "fake worker produced an empty document title",
            ));
        }
        let provenance = ProvenanceRecord {
            kind: ProvenanceKind::LocalExtraction,
            component_id: WORKER_ID.to_owned(),
            component_version: WORKER_PROTOCOL_VERSION.to_owned(),
            input_digests: vec![source.sha256.clone()],
            output_digest: None,
            source_locations: Vec::new(),
        };
        let nodes = payload
            .paragraphs
            .into_iter()
            .enumerate()
            .map(|(index, paragraph)| ImportNode {
                id: format!("paragraph-{}", index + 1),
                kind: ImportNodeKind::Paragraph { text: paragraph },
                confidence: Confidence::from_basis_points(10_000)
                    .expect("10,000 basis points is valid"),
                source_locations: vec![ImportSourceLocation {
                    source_digest: source.sha256.clone(),
                    page: None,
                    region: None,
                    byte_start: None,
                    byte_end: None,
                }],
                provenance: vec![provenance.clone()],
            })
            .collect();
        ImportDocument::create(
            format!("document-{}", &source.sha256.as_str()[..24]),
            source.sha256.clone(),
            payload.title,
            nodes,
            Vec::new(),
            response.diagnostics,
            vec![provenance],
        )
    }
}

#[derive(Clone, Debug)]
pub enum FakeWorkerMode {
    Success,
    WaitUntilCancelled { poll_interval: Duration },
    Fail { message: String },
    Panic,
}

#[derive(Clone, Debug)]
pub struct FakeWorker {
    mode: FakeWorkerMode,
}

impl Default for FakeWorker {
    fn default() -> Self {
        Self::success()
    }
}

impl FakeWorker {
    #[must_use]
    pub const fn success() -> Self {
        Self {
            mode: FakeWorkerMode::Success,
        }
    }

    #[must_use]
    pub const fn wait_until_cancelled(poll_interval: Duration) -> Self {
        Self {
            mode: FakeWorkerMode::WaitUntilCancelled { poll_interval },
        }
    }

    #[must_use]
    pub fn fail(message: impl Into<String>) -> Self {
        Self {
            mode: FakeWorkerMode::Fail {
                message: message.into(),
            },
        }
    }

    #[must_use]
    pub const fn panic() -> Self {
        Self {
            mode: FakeWorkerMode::Panic,
        }
    }
}

impl FormatWorker for FakeWorker {
    fn worker_id(&self) -> &str {
        WORKER_ID
    }

    fn protocol_version(&self) -> &str {
        WORKER_PROTOCOL_VERSION
    }

    fn execute(
        &self,
        request: WorkerRequest,
        context: WorkerContext,
    ) -> Result<WorkerResponse, ImportError> {
        match &self.mode {
            FakeWorkerMode::Success => {}
            FakeWorkerMode::WaitUntilCancelled { poll_interval } => {
                while !context.is_cancelled() {
                    thread::sleep(*poll_interval);
                }
                return Err(ImportError::new(
                    ImportErrorCode::Cancelled,
                    "fake worker observed cancellation",
                ));
            }
            FakeWorkerMode::Fail { message } => {
                return Err(ImportError::new(
                    ImportErrorCode::WorkerFailed,
                    message.clone(),
                ));
            }
            FakeWorkerMode::Panic => panic!("injected fake worker panic"),
        }

        let bytes = context.read_bounded(&request.source_locator, request.source.byte_length)?;
        if sha256_bytes(&bytes) != request.source.sha256
            || u64::try_from(bytes.len()).unwrap_or(u64::MAX) != request.source.byte_length
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "fake worker input does not match the planned source digest and length",
            ));
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| {
            ImportError::new(
                ImportErrorCode::InvalidSource,
                "fake fixture source must be UTF-8",
            )
        })?;
        let fixture_body =
            text.strip_prefix(std::str::from_utf8(SIGNATURE).expect("ASCII signature"));
        let Some(fixture_body) = fixture_body else {
            return Err(ImportError::new(
                ImportErrorCode::InvalidSource,
                "fake fixture signature is missing",
            ));
        };
        let mut lines = fixture_body.lines();
        let title = lines.next().unwrap_or_default().to_owned();
        let paragraphs = lines
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let payload = serde_json::to_value(FakePayload { title, paragraphs })
            .map_err(|error| ImportError::serialization(&error))?;
        Ok(WorkerResponse {
            contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id: request.request_id,
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: request.source.sha256,
            payload,
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components: vec![ComponentVersion {
                component_id: WORKER_ID.to_owned(),
                version: WORKER_PROTOCOL_VERSION.to_owned(),
                artifact_digest: None,
            }],
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FakePayload {
    title: String,
    paragraphs: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{FakeAdapter, FakeWorker};
    use crate::{
        AsciiDocV1ProposalValidator, CancellationToken, CommitResult, ImportLimits, ImportPipeline,
        ImportTempRoot, IntakeRequest, OriginClass, PlanRequest, PortablePath,
    };
    use std::sync::Arc;

    #[test]
    fn fake_pipeline_preserves_cjk_and_produces_a_preview_receipt() {
        let base = tempfile::tempdir().expect("temporary directory");
        let temp_root =
            ImportTempRoot::initialize(base.path().join("imports")).expect("temporary root");
        let pipeline = ImportPipeline::new(temp_root, Arc::new(AsciiDocV1ProposalValidator));
        let workspace = base.path().join("workspace-sentinel");
        std::fs::create_dir(&workspace).expect("workspace sentinel directory");
        std::fs::write(workspace.join("unchanged.txt"), b"workspace authority")
            .expect("workspace sentinel bytes");
        let bytes = "WEFTEXT-FAKE/1\n文缕导入\n中文与 Latin mixed spacing。\n第二段 ✅\n"
            .as_bytes()
            .to_vec();

        let preview = pipeline
            .preview(
                IntakeRequest {
                    display_name: "证据.fake".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes,
                    plan: PlanRequest::single_node(PortablePath::parse("导入结果").expect("path")),
                    limits: ImportLimits::default(),
                    cancellation: CancellationToken::default(),
                },
                &FakeAdapter,
                Arc::new(FakeWorker::success()),
            )
            .expect("preview import");
        let receipt = preview
            .receipt("2026-08-24T12:00:00+08:00", CommitResult::PreviewOnly)
            .expect("receipt");

        let exact_source = &preview.proposal.proposal().nodes[0].exact_asciidoc;
        assert!(exact_source.contains("= 文缕导入"));
        assert!(exact_source.contains("中文与 Latin mixed spacing。"));
        assert!(exact_source.contains("第二段 ✅"));
        assert_eq!(receipt.source_digest, preview.source.sha256);
        assert_eq!(receipt.ir_revision, preview.document.revision);
        assert_eq!(
            std::fs::read(workspace.join("unchanged.txt")).expect("read workspace sentinel"),
            b"workspace authority"
        );
        assert_eq!(
            base.path()
                .join("imports")
                .read_dir()
                .expect("read root")
                .count(),
            1
        );
    }
}
