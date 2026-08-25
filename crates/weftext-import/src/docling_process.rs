use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::docling_lite::{
    DOCLING_DOCUMENT_SCHEMA_NAME, DOCLING_DOCUMENT_SCHEMA_VERSION,
    DOCLING_LITE_WORKER_PROTOCOL_VERSION, DoclingLiteWorkerCommand, WORKER_ID,
};
use crate::{
    ComponentVersion, FormatWorker, ImportError, ImportErrorCode, Sha256Digest,
    WORKER_RESPONSE_CONTRACT_VERSION, WorkerContext, WorkerRequest, WorkerResponse, sha256_bytes,
};

const STDERR_BYTE_LIMIT: u64 = 64 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// No-shell external Docling worker supervisor.
///
/// The type deliberately has no public constructor. A production constructor
/// must remain unavailable until its caller can supply a race-free host network,
/// memory, filesystem, and process-tree sandbox. A cleared environment and a
/// fixed current directory do not restrict the worker's Windows access token.
/// Unit fixtures exercise only the wire protocol and direct-child lifecycle;
/// they are not sandbox evidence and cannot turn into product capability.
#[derive(Clone, Debug)]
pub struct DoclingLiteProcessWorker {
    executable: PathBuf,
    executable_byte_length: u64,
    executable_sha256: Sha256Digest,
    fixed_arguments: Vec<OsString>,
    clean_environment: BTreeMap<OsString, OsString>,
}

impl DoclingLiteProcessWorker {
    #[cfg(test)]
    pub(crate) fn fixture(
        executable: PathBuf,
        executable_bytes: &[u8],
        clean_environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        Self {
            executable,
            executable_byte_length: u64::try_from(executable_bytes.len()).unwrap_or(u64::MAX),
            executable_sha256: sha256_bytes(executable_bytes),
            fixed_arguments: Vec::new(),
            clean_environment,
        }
    }

    fn verify_executable(&self) -> Result<(), ImportError> {
        let metadata = std::fs::symlink_metadata(&self.executable)
            .map_err(|error| process_io("inspect pinned Docling worker", &error))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "pinned Docling worker must be a regular non-link file",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt as _;
            if metadata.nlink() != 1 {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    "pinned Docling worker must have exactly one filesystem link",
                ));
            }
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt as _;
            const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
            if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Err(ImportError::new(
                    ImportErrorCode::CapabilityUnavailable,
                    "pinned Docling worker must not be a Windows reparse point",
                ));
            }
        }
        if metadata.len() != self.executable_byte_length {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "pinned Docling worker byte length changed before launch",
            ));
        }
        let bytes = std::fs::read(&self.executable)
            .map_err(|error| process_io("hash pinned Docling worker", &error))?;
        if sha256_bytes(&bytes) != self.executable_sha256 {
            return Err(ImportError::new(
                ImportErrorCode::CapabilityUnavailable,
                "pinned Docling worker SHA-256 changed before launch",
            ));
        }
        Ok(())
    }

    #[cfg(not(test))]
    fn validate_and_serialize_request(
        &self,
        _request: &WorkerRequest,
    ) -> Result<Vec<u8>, ImportError> {
        Err(ImportError::new(
            ImportErrorCode::CapabilityUnavailable,
            format!(
                "{} production sandbox is unavailable for this build",
                self.worker_id()
            ),
        ))
    }

    #[cfg(test)]
    fn validate_and_serialize_request(
        &self,
        request: &WorkerRequest,
    ) -> Result<Vec<u8>, ImportError> {
        request.validate()?;
        if request.worker_id != self.worker_id()
            || request.worker_protocol_version != self.protocol_version()
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "Docling process request targets a different worker boundary",
            ));
        }
        let command: DoclingLiteWorkerCommand =
            serde_json::from_value(request.format_options.clone()).map_err(|error| {
                ImportError::new(
                    ImportErrorCode::WorkerProtocol,
                    format!("invalid Docling worker command: {error}"),
                )
            })?;
        command.validate_boundary()?;
        if command.request_id != request.request_id
            || command.source_digest != request.source.sha256
            || command.memory_limit_bytes != request.memory_limit_bytes
            || command.output_byte_limit != request.output_byte_limit
            || command.page_limit != request.page_limit
            || command.input_locator != request.source_locator
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "Docling process command differs from the immutable worker request",
            ));
        }
        let request_json =
            serde_json::to_vec(request).map_err(|error| ImportError::serialization(&error))?;
        if u64::try_from(request_json.len()).unwrap_or(u64::MAX)
            > request.plan.limits.max_total_output_bytes
        {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling worker request JSON exceeds its bounded wire limit",
            ));
        }
        Ok(request_json)
    }

    fn process_command(&self, current_directory: &std::path::Path) -> Command {
        let mut process = Command::new(&self.executable);
        process
            .args(&self.fixed_arguments)
            .current_dir(current_directory)
            .env_clear()
            .envs(&self.clean_environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt as _;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            process.creation_flags(CREATE_NO_WINDOW);
        }
        process
    }
}

impl FormatWorker for DoclingLiteProcessWorker {
    fn worker_id(&self) -> &str {
        WORKER_ID
    }

    fn protocol_version(&self) -> &str {
        DOCLING_LITE_WORKER_PROTOCOL_VERSION
    }

    fn execute(
        &self,
        request: WorkerRequest,
        context: WorkerContext,
    ) -> Result<WorkerResponse, ImportError> {
        let request_json = self.validate_and_serialize_request(&request)?;
        if context.is_cancelled() {
            return Err(cancelled_error());
        }
        self.verify_executable()?;
        let mut child = self
            .process_command(context.session_root())
            .spawn()
            .map_err(|error| process_io("start pinned Docling worker", &error))?;
        let stdin = child.stdin.take().ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::WorkerFailed,
                "Docling worker stdin was not connected",
            )
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::WorkerFailed,
                "Docling worker stdout was not connected",
            )
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            ImportError::new(
                ImportErrorCode::WorkerFailed,
                "Docling worker stderr was not connected",
            )
        })?;
        let input = thread::spawn(move || write_worker_input(stdin, &request_json));
        let output_limit = request.output_byte_limit;
        let output = thread::spawn(move || capture_bounded(stdout, output_limit));
        let errors = thread::spawn(move || capture_bounded(stderr, STDERR_BYTE_LIMIT));

        let status = loop {
            if context.is_cancelled() {
                terminate_and_reap(&mut child)?;
                join_input(input)?;
                let _ = join_capture(output)?;
                let _ = join_capture(errors)?;
                return Err(cancelled_error());
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => thread::sleep(POLL_INTERVAL),
                Err(error) => {
                    terminate_and_reap(&mut child)?;
                    return Err(process_io("poll pinned Docling worker", &error));
                }
            }
        };
        join_input(input)?;
        let output = join_capture(output)?;
        let errors = join_capture(errors)?;
        if output.exceeded || errors.exceeded {
            return Err(ImportError::new(
                ImportErrorCode::LimitExceeded,
                "Docling worker stdout or stderr exceeded its byte limit",
            ));
        }
        if !status.success() {
            let detail = String::from_utf8_lossy(&errors.bytes);
            let detail = detail.trim();
            return Err(ImportError::new(
                ImportErrorCode::WorkerFailed,
                if detail.is_empty() {
                    format!("Docling worker exited with {status}")
                } else {
                    format!("Docling worker exited with {status}: {detail}")
                },
            ));
        }
        decode_worker_output(&output.bytes, &request)
    }
}

fn decode_worker_output(
    bytes: &[u8],
    request: &WorkerRequest,
) -> Result<WorkerResponse, ImportError> {
    let value: serde_json::Value = serde_json::from_slice(bytes).map_err(|error| {
        ImportError::new(
            ImportErrorCode::WorkerProtocol,
            format!("Docling worker emitted invalid response JSON: {error}"),
        )
    })?;
    let is_raw_document = value.as_object().is_some_and(|document| {
        document
            .get("schema_name")
            .and_then(serde_json::Value::as_str)
            == Some(DOCLING_DOCUMENT_SCHEMA_NAME)
            && document.get("version").and_then(serde_json::Value::as_str)
                == Some(DOCLING_DOCUMENT_SCHEMA_VERSION)
    });
    if !is_raw_document {
        return serde_json::from_slice(bytes).map_err(|error| {
            ImportError::new(
                ImportErrorCode::WorkerProtocol,
                format!("Docling worker emitted invalid typed failure JSON: {error}"),
            )
        });
    }

    let command: DoclingLiteWorkerCommand = serde_json::from_value(request.format_options.clone())
        .map_err(|error| {
            ImportError::new(
                ImportErrorCode::WorkerProtocol,
                format!("Docling worker command could not bind raw success output: {error}"),
            )
        })?;
    command.validate_boundary()?;
    let components = command
        .model_pins
        .into_iter()
        .map(|pin| ComponentVersion {
            component_id: pin.component,
            version: pin.version,
            artifact_digest: Some(pin.sha256),
        })
        .collect();
    Ok(WorkerResponse {
        contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
        request_id: request.request_id.clone(),
        worker_id: request.worker_id.clone(),
        worker_protocol_version: request.worker_protocol_version.clone(),
        source_digest: request.source.sha256.clone(),
        payload: value,
        resources: Vec::new(),
        diagnostics: Vec::new(),
        components,
    })
}

struct CapturedOutput {
    bytes: Vec<u8>,
    exceeded: bool,
}

fn capture_bounded(
    mut reader: impl std::io::Read,
    maximum: u64,
) -> std::io::Result<CapturedOutput> {
    let capacity = usize::try_from(maximum.min(1024 * 1024)).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    let mut exceeded = false;
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = maximum.saturating_sub(u64::try_from(bytes.len()).unwrap_or(u64::MAX));
        let keep = usize::try_from(remaining).unwrap_or(usize::MAX).min(count);
        bytes.extend_from_slice(&buffer[..keep]);
        if keep != count {
            exceeded = true;
        }
    }
    Ok(CapturedOutput { bytes, exceeded })
}

fn write_worker_input(mut stdin: impl std::io::Write, bytes: &[u8]) -> std::io::Result<()> {
    stdin.write_all(bytes)?;
    stdin.flush()
}

fn terminate_and_reap(child: &mut Child) -> Result<(), ImportError> {
    let kill_error = child.kill().err();
    child
        .wait()
        .map_err(|error| process_io("reap pinned Docling worker", &error))?;
    if let Some(error) = kill_error.filter(|error| error.kind() != std::io::ErrorKind::InvalidInput)
    {
        return Err(process_io("terminate pinned Docling worker", &error));
    }
    Ok(())
}

fn join_input(handle: thread::JoinHandle<std::io::Result<()>>) -> Result<(), ImportError> {
    match handle.join() {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Ok(Err(error)) => Err(process_io("write Docling worker request", &error)),
        Err(_) => Err(ImportError::new(
            ImportErrorCode::WorkerFailed,
            "Docling worker input supervisor panicked",
        )),
    }
}

fn join_capture(
    handle: thread::JoinHandle<std::io::Result<CapturedOutput>>,
) -> Result<CapturedOutput, ImportError> {
    match handle.join() {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(process_io("capture Docling worker output", &error)),
        Err(_) => Err(ImportError::new(
            ImportErrorCode::WorkerFailed,
            "Docling worker output supervisor panicked",
        )),
    }
}

fn cancelled_error() -> ImportError {
    ImportError::new(
        ImportErrorCode::Cancelled,
        "Docling worker process was cancelled and reaped",
    )
}

fn process_io(operation: &str, error: &std::io::Error) -> ImportError {
    ImportError::new(
        ImportErrorCode::WorkerFailed,
        format!("{operation}: {error}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docling_lite::{
        DOCLING_DOCUMENT_SCHEMA_NAME, DOCLING_DOCUMENT_SCHEMA_VERSION, DOCLING_RELEASE_COMMIT,
        DOCLING_RELEASE_TAG, DoclingLiteWorkerStatus, DoclingModelPin,
    };
    use crate::{
        AdapterDescriptor, AdapterRoute, CancellationToken, ComponentVersion, Confidence,
        EncryptionState, FORMAT_PROBE_CONTRACT_VERSION, FormatProbe, ImportLimits, ImportPlan,
        LocalOcrPolicy, OriginClass, PROBE_EVIDENCE_CONTRACT_VERSION, PlanRequest, PortablePath,
        ProbeEvidence, ProbeEvidenceSegment, SourceArtifact, SourceFormat,
        WORKER_REQUEST_CONTRACT_VERSION, WORKER_RESPONSE_CONTRACT_VERSION, WorkerNetworkPolicy,
    };
    use std::time::{Duration, Instant};

    const HELPER_SOURCE: &str = r#"
use std::io::{Read, Write};
use std::time::Duration;

fn main() {
    let mut request = Vec::new();
    std::io::stdin().read_to_end(&mut request).unwrap();
    match std::env::var("WEFTEXT_FIXTURE_MODE").as_deref() {
        Ok("success" | "typed") => print!("{}", std::env::var("WEFTEXT_FIXTURE_RESPONSE").unwrap()),
        Ok("malformed") => print!("{{not-json"),
        Ok("oversized") => {
            let bytes = vec![b'x'; 2 * 1024 * 1024];
            std::io::stdout().write_all(&bytes).unwrap();
        }
        Ok("failed") => {
            eprintln!("fixture worker failed closed");
            std::process::exit(7);
        }
        Ok("wait") => std::thread::sleep(Duration::from_secs(30)),
        _ => std::process::exit(9),
    }
}
"#;

    #[test]
    fn process_fixture_wraps_raw_success_with_clean_environment_and_exact_context() {
        let fixture = ProcessFixture::new("success");
        let (request, response) = request_and_response();
        let worker = fixture.worker(Some(&response));
        let result = worker
            .execute(request, fixture.context(Duration::from_secs(2)))
            .expect("worker response");
        assert_eq!(result, response);
    }

    #[test]
    fn process_fixture_preserves_a_typed_failed_response() {
        let fixture = ProcessFixture::new("typed");
        let (request, mut response) = request_and_response();
        response.payload = serde_json::json!({
            "status": DoclingLiteWorkerStatus::Failed,
            "doclingDocumentJson": null
        });
        let worker = fixture.worker(Some(&response));
        let result = worker
            .execute(request, fixture.context(Duration::from_secs(2)))
            .expect("typed worker failure response");
        assert_eq!(result, response);
    }

    #[test]
    fn process_fixture_rejects_a_different_worker_boundary_before_spawn() {
        let (mut request, _) = request_and_response();
        request.worker_id = "weftext.other-worker".to_owned();
        request.plan.route.worker_id = request.worker_id.clone();
        let worker = DoclingLiteProcessWorker::fixture(
            PathBuf::from("unreachable-worker"),
            b"",
            BTreeMap::new(),
        );

        let error = worker
            .execute(
                request,
                WorkerContext::new(
                    PathBuf::from("unreachable-session"),
                    CancellationToken::default(),
                    Instant::now() + Duration::from_secs(2),
                ),
            )
            .expect_err("different worker boundary");

        assert_eq!(error.code(), ImportErrorCode::WorkerProtocol);
    }

    #[test]
    fn process_fixture_kills_and_reaps_on_deadline() {
        let fixture = ProcessFixture::new("wait");
        let (request, _) = request_and_response();
        let started = Instant::now();
        let error = fixture
            .worker(None)
            .execute(request, fixture.context(Duration::from_millis(40)))
            .expect_err("deadline");
        assert_eq!(error.code(), ImportErrorCode::Cancelled);
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn process_fixture_kills_and_reaps_on_cancellation() {
        let fixture = ProcessFixture::new("wait");
        let (request, _) = request_and_response();
        let cancellation = CancellationToken::default();
        let trigger = cancellation.clone();
        let canceller = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            trigger.cancel();
        });
        let context = WorkerContext::new(
            fixture.session.path().to_path_buf(),
            cancellation,
            Instant::now() + Duration::from_secs(2),
        );
        let error = fixture
            .worker(None)
            .execute(request, context)
            .expect_err("cancelled");
        canceller.join().expect("canceller");
        assert_eq!(error.code(), ImportErrorCode::Cancelled);
    }

    #[test]
    fn process_fixture_rejects_malformed_and_oversized_output() {
        for (mode, expected) in [
            ("malformed", ImportErrorCode::WorkerProtocol),
            ("oversized", ImportErrorCode::LimitExceeded),
            ("failed", ImportErrorCode::WorkerFailed),
        ] {
            let fixture = ProcessFixture::new(mode);
            let (request, _) = request_and_response();
            let error = fixture
                .worker(None)
                .execute(request, fixture.context(Duration::from_secs(2)))
                .expect_err(mode);
            assert_eq!(error.code(), expected, "{mode}");
        }
    }

    #[test]
    fn process_fixture_detects_executable_tampering_before_spawn() {
        let fixture = ProcessFixture::new("success");
        let (request, response) = request_and_response();
        let worker = fixture.worker(Some(&response));
        std::fs::write(&fixture.executable, b"tampered").expect("tamper helper");
        let error = worker
            .execute(request, fixture.context(Duration::from_secs(2)))
            .expect_err("tampered executable");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    #[cfg(unix)]
    #[test]
    fn process_fixture_rejects_linked_executable_before_spawn() {
        use std::os::unix::fs::symlink;

        let fixture = ProcessFixture::new("success");
        let (request, response) = request_and_response();
        let linked = fixture.root.path().join("linked-worker");
        symlink(&fixture.executable, &linked).expect("worker symlink");
        let worker = DoclingLiteProcessWorker::fixture(
            linked,
            &fixture.executable_bytes,
            BTreeMap::from([
                (
                    OsString::from("WEFTEXT_FIXTURE_MODE"),
                    OsString::from("success"),
                ),
                (
                    OsString::from("WEFTEXT_FIXTURE_RESPONSE"),
                    OsString::from(serde_json::to_string(&response).expect("fixture response")),
                ),
            ]),
        );
        let error = worker
            .execute(request, fixture.context(Duration::from_secs(2)))
            .expect_err("linked executable");
        assert_eq!(error.code(), ImportErrorCode::CapabilityUnavailable);
    }

    struct ProcessFixture {
        root: tempfile::TempDir,
        session: tempfile::TempDir,
        executable: PathBuf,
        executable_bytes: Vec<u8>,
        mode: &'static str,
    }

    impl ProcessFixture {
        fn new(mode: &'static str) -> Self {
            let root = tempfile::tempdir().expect("helper build root");
            let source = root.path().join("fixture-worker.rs");
            std::fs::write(&source, HELPER_SOURCE).expect("helper source");
            let mut executable = root.path().join("fixture-worker");
            if cfg!(windows) {
                executable.set_extension("exe");
            }
            let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
            let status = Command::new(rustc)
                .args(["--edition=2021", "-o"])
                .arg(&executable)
                .arg(&source)
                .status()
                .expect("compile process helper");
            assert!(status.success(), "process helper must compile");
            let executable_bytes = std::fs::read(&executable).expect("helper bytes");
            let session = tempfile::tempdir().expect("worker session");
            std::fs::create_dir(session.path().join("input")).expect("input directory");
            std::fs::write(session.path().join("input/source.pdf"), b"%PDF-fixture")
                .expect("worker input");
            Self {
                root,
                session,
                executable,
                executable_bytes,
                mode,
            }
        }

        fn worker(&self, response: Option<&WorkerResponse>) -> DoclingLiteProcessWorker {
            debug_assert!(self.executable.starts_with(self.root.path()));
            let mut environment = BTreeMap::from([(
                OsString::from("WEFTEXT_FIXTURE_MODE"),
                OsString::from(self.mode),
            )]);
            if let Some(response) = response {
                let serialized = if self.mode == "success" {
                    serde_json::to_string(&response.payload)
                } else {
                    serde_json::to_string(response)
                }
                .expect("fixture response");
                environment.insert(
                    OsString::from("WEFTEXT_FIXTURE_RESPONSE"),
                    OsString::from(serialized),
                );
            }
            DoclingLiteProcessWorker::fixture(
                self.executable.clone(),
                &self.executable_bytes,
                environment,
            )
        }

        fn context(&self, duration: Duration) -> WorkerContext {
            WorkerContext::new(
                self.session.path().to_path_buf(),
                CancellationToken::default(),
                Instant::now() + duration,
            )
        }
    }

    #[allow(clippy::too_many_lines)]
    fn request_and_response() -> (WorkerRequest, WorkerResponse) {
        let limits = ImportLimits {
            max_source_bytes: 1024,
            max_probe_bytes: 1024,
            max_total_output_bytes: 1024 * 1024,
            max_resource_bytes: 1024 * 1024,
            max_agent_output_bytes: 1024 * 1024,
            worker_timeout_ms: 2_000,
            ..ImportLimits::default()
        };
        let mut source = SourceArtifact::from_bytes(
            "success.pdf",
            OriginClass::TestFixture,
            b"%PDF-fixture",
            &limits,
        )
        .expect("source");
        source.detected_format = SourceFormat::Pdf;
        let adapter = AdapterDescriptor {
            adapter_id: "weftext.pdf-docling-lite-adapter".to_owned(),
            adapter_version: "fixture-v1".to_owned(),
            supported_format: SourceFormat::Pdf,
        };
        let probe = FormatProbe {
            contract_version: FORMAT_PROBE_CONTRACT_VERSION.to_owned(),
            adapter: adapter.clone(),
            source_digest: source.sha256.clone(),
            evidence: ProbeEvidence {
                contract_version: PROBE_EVIDENCE_CONTRACT_VERSION.to_owned(),
                source_digest: source.sha256.clone(),
                source_byte_length: source.byte_length,
                byte_budget: limits.max_probe_bytes,
                inspected_bytes: source.byte_length,
                segments: vec![ProbeEvidenceSegment {
                    offset: 0,
                    byte_length: source.byte_length,
                    sha256: source.sha256.clone(),
                }],
            },
            detected_format: SourceFormat::Pdf,
            signature_confidence: Confidence::from_basis_points(9_900).expect("confidence"),
            parser_confidence: Confidence::from_basis_points(9_500).expect("confidence"),
            encryption: EncryptionState::NotEncrypted,
            signature_evidence: vec!["fixture".to_owned()],
            mismatch_evidence: Vec::new(),
            active_content_detected: false,
            page_count: None,
            container_entry_count: None,
            safe_to_plan: true,
            diagnostics: Vec::new(),
        };
        let plan = ImportPlan::create(
            &source,
            &probe,
            AdapterRoute {
                adapter,
                worker_id: WORKER_ID.to_owned(),
                worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            },
            PlanRequest::single_node(PortablePath::parse("Imported").expect("destination")),
            limits,
        )
        .expect("plan");
        let pins = fixture_pins();
        let request_id = "request-process-fixture".to_owned();
        let input_locator = PortablePath::parse("input/source.pdf").expect("input locator");
        let command = DoclingLiteWorkerCommand {
            protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            request_id: request_id.clone(),
            source_digest: source.sha256.clone(),
            plan_id: plan.plan_id.clone(),
            input_locator: input_locator.clone(),
            output_locator: PortablePath::parse("output/docling-document.json")
                .expect("output locator"),
            docling_release_tag: DOCLING_RELEASE_TAG.to_owned(),
            docling_release_commit: DOCLING_RELEASE_COMMIT.to_owned(),
            document_schema_name: DOCLING_DOCUMENT_SCHEMA_NAME.to_owned(),
            document_schema_version: DOCLING_DOCUMENT_SCHEMA_VERSION.to_owned(),
            target: "x86_64-test-offline".to_owned(),
            local_ocr_policy: LocalOcrPolicy::Automatic,
            ocr_language: "en".to_owned(),
            layout_precision: "int8".to_owned(),
            no_table_former: true,
            network: WorkerNetworkPolicy::Denied,
            page_limit: plan.limits.max_pages,
            memory_limit_bytes: plan.limits.worker_memory_bytes,
            output_byte_limit: plan.limits.max_total_output_bytes,
            model_pins: pins.clone(),
        };
        let request = WorkerRequest {
            contract_version: WORKER_REQUEST_CONTRACT_VERSION.to_owned(),
            request_id: request_id.clone(),
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source: source.clone(),
            source_locator: input_locator,
            plan,
            network: WorkerNetworkPolicy::Denied,
            memory_limit_bytes: command.memory_limit_bytes,
            page_limit: command.page_limit,
            entry_limit: ImportLimits::default().max_container_entries,
            output_byte_limit: command.output_byte_limit,
            format_options: serde_json::to_value(command).expect("command JSON"),
        };
        let components = pins
            .into_iter()
            .map(|pin| ComponentVersion {
                component_id: pin.component,
                version: pin.version,
                artifact_digest: Some(pin.sha256),
            })
            .collect();
        let response = WorkerResponse {
            contract_version: WORKER_RESPONSE_CONTRACT_VERSION.to_owned(),
            request_id,
            worker_id: WORKER_ID.to_owned(),
            worker_protocol_version: DOCLING_LITE_WORKER_PROTOCOL_VERSION.to_owned(),
            source_digest: source.sha256,
            payload: serde_json::json!({
                "schema_name": DOCLING_DOCUMENT_SCHEMA_NAME,
                "version": DOCLING_DOCUMENT_SCHEMA_VERSION
            }),
            resources: Vec::new(),
            diagnostics: Vec::new(),
            components,
        };
        (request, response)
    }

    fn fixture_pins() -> Vec<DoclingModelPin> {
        [
            "docling-rs",
            "pdfium",
            "onnx-runtime",
            "layout-int8",
            "pp-ocr",
            "ocr-dictionary",
        ]
        .into_iter()
        .map(|component| DoclingModelPin {
            component: component.to_owned(),
            version: "fixture-v1".to_owned(),
            sha256: sha256_bytes(component.as_bytes()),
            notice_id: format!("notice-{component}"),
        })
        .collect()
    }
}
