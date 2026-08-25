use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use uuid::Uuid;

use crate::{
    CanonicalProposalValidator, CommitResult, ComponentVersion, FormatProbe, FormatWorker,
    ImportAdapter, ImportDocument, ImportError, ImportErrorCode, ImportLimits, ImportPlan,
    ImportReceipt, ImportTempRoot, OriginClass, PlanRequest, PortablePath, SourceArtifact,
    SourceFormat, TempSession, ValidatedProposal, WorkerContext, WorkerRequest, WorkerResponse,
    probe_source_bytes,
};

#[derive(Clone, Debug, Default)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub struct PreviewedImport {
    pub source: SourceArtifact,
    pub probe: FormatProbe,
    pub plan: ImportPlan,
    pub document: ImportDocument,
    pub proposal: ValidatedProposal,
    pub components: Vec<ComponentVersion>,
}

impl PreviewedImport {
    /// Records the exact preview and a caller-supplied Core commit outcome.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid timestamp or inconsistent receipt input.
    pub fn receipt(
        &self,
        created_at: impl Into<String>,
        commit_result: CommitResult,
    ) -> Result<ImportReceipt, ImportError> {
        ImportReceipt::create(
            created_at,
            &self.source,
            &self.plan,
            &self.document,
            &self.proposal,
            self.components.clone(),
            commit_result,
        )
    }
}

pub struct ImportPipeline {
    temp_root: ImportTempRoot,
    proposal_validator: Arc<dyn CanonicalProposalValidator>,
}

#[derive(Clone, Debug)]
pub struct IntakeRequest {
    pub display_name: String,
    pub origin: OriginClass,
    pub bytes: Vec<u8>,
    pub plan: PlanRequest,
    pub limits: ImportLimits,
    pub cancellation: CancellationToken,
}

impl ImportPipeline {
    #[must_use]
    pub fn new(
        temp_root: ImportTempRoot,
        proposal_validator: Arc<dyn CanonicalProposalValidator>,
    ) -> Self {
        Self {
            temp_root,
            proposal_validator,
        }
    }

    /// Runs bounded intake through probe, plan, worker, IR, and canonical preview.
    ///
    /// # Errors
    ///
    /// Returns an error when any contract, safety limit, cancellation, worker,
    /// IR, temporary-storage, or canonical-proposal check fails.
    pub fn preview(
        &self,
        intake: IntakeRequest,
        adapter: &dyn ImportAdapter,
        worker: Arc<dyn FormatWorker>,
    ) -> Result<PreviewedImport, ImportError> {
        let IntakeRequest {
            display_name,
            origin,
            bytes,
            plan: request,
            limits,
            cancellation,
        } = intake;
        limits.validate()?;
        if cancellation.is_cancelled() {
            return cancelled();
        }
        let mut source = SourceArtifact::from_bytes(display_name, origin, &bytes, &limits)?;
        source.validate(&limits)?;

        let probe = probe_source_bytes(adapter, &source, &bytes, &limits)?;
        if probe.adapter != adapter.descriptor() {
            return Err(ImportError::new(
                ImportErrorCode::InvalidContract,
                "adapter descriptor changed between discovery and probe",
            ));
        }
        if !probe.safe_to_plan {
            return Err(ImportError::new(
                ImportErrorCode::ProbeRejected,
                "the bounded format probe did not authorize conversion planning",
            ));
        }
        source.detected_format = probe.detected_format;
        source
            .mismatch_evidence
            .clone_from(&probe.mismatch_evidence);

        let plan = adapter.plan(&source, &probe, request, limits)?;
        plan.validate(&source, &probe)?;
        if cancellation.is_cancelled() {
            return cancelled();
        }

        let mut session = self.temp_root.start_session(&Uuid::new_v4().to_string())?;
        let source_locator = source_input_locator(source.detected_format)?;
        session.write_file(&source_locator, &bytes, plan.limits.max_source_bytes)?;
        let worker_request = adapter.worker_request(&source, &plan, source_locator)?;
        worker_request.validate()?;
        if worker.worker_id() != worker_request.worker_id
            || worker.protocol_version() != worker_request.worker_protocol_version
        {
            return Err(ImportError::new(
                ImportErrorCode::WorkerProtocol,
                "selected worker does not implement the planned worker protocol",
            ));
        }

        let response = execute_worker(worker, worker_request.clone(), &mut session, &cancellation)?;
        response.validate(&worker_request, &plan.limits)?;
        let components = response.components.clone();
        let document = adapter.map_worker_response(&source, &plan, response)?;
        document.validate(&source, &plan)?;
        let proposal = self
            .proposal_validator
            .render_and_validate(&source, &bytes, &plan, &document)?;
        Ok(PreviewedImport {
            source,
            probe,
            plan,
            document,
            proposal,
            components,
        })
    }
}

fn source_input_locator(format: SourceFormat) -> Result<PortablePath, ImportError> {
    // Some reviewed parsers use the terminal extension as part of format dispatch. The locator is
    // derived from the bounded probe result, never from the untrusted display name.
    let locator = match format {
        SourceFormat::Pdf => "input/source.pdf",
        _ => "input/source.bin",
    };
    PortablePath::parse(locator)
}

fn execute_worker(
    worker: Arc<dyn FormatWorker>,
    request: WorkerRequest,
    session: &mut TempSession,
    cancellation: &CancellationToken,
) -> Result<WorkerResponse, ImportError> {
    let timeout = Duration::from_millis(request.plan.limits.worker_timeout_ms);
    let grace = Duration::from_millis(request.plan.limits.cancellation_grace_ms);
    let deadline = Instant::now()
        .checked_add(timeout)
        .unwrap_or_else(Instant::now);
    let context = WorkerContext::new(session.path().to_path_buf(), cancellation.clone(), deadline);
    let (sender, receiver) = mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name("weftext-import-worker".to_owned())
        .spawn(move || {
            let result = worker.execute(request, context);
            let _ = sender.send(result);
        })
        .map_err(|error| {
            ImportError::new(
                ImportErrorCode::WorkerFailed,
                format!("failed to start the supervised worker thread: {error}"),
            )
        })?;

    loop {
        if cancellation.is_cancelled() {
            return stop_worker(
                &receiver,
                handle,
                session,
                grace,
                ImportErrorCode::Cancelled,
                "worker request was cancelled",
            );
        }
        let now = Instant::now();
        if now >= deadline {
            cancellation.cancel();
            return stop_worker(
                &receiver,
                handle,
                session,
                grace,
                ImportErrorCode::TimedOut,
                "worker exceeded its time limit",
            );
        }
        let wait = deadline
            .saturating_duration_since(now)
            .min(Duration::from_millis(10));
        match receiver.recv_timeout(wait) {
            Ok(result) => {
                join_worker(handle)?;
                if Instant::now() >= deadline {
                    cancellation.cancel();
                    return Err(ImportError::new(
                        ImportErrorCode::TimedOut,
                        "worker exceeded its time limit",
                    ));
                }
                if cancellation.is_cancelled() {
                    return Err(ImportError::new(
                        ImportErrorCode::Cancelled,
                        "worker request was cancelled",
                    ));
                }
                return result;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                join_worker(handle)?;
                return Err(ImportError::new(
                    ImportErrorCode::WorkerFailed,
                    "worker terminated without a response",
                ));
            }
        }
    }
}

fn stop_worker(
    receiver: &mpsc::Receiver<Result<WorkerResponse, ImportError>>,
    handle: thread::JoinHandle<()>,
    session: &mut TempSession,
    grace: Duration,
    code: ImportErrorCode,
    message: &str,
) -> Result<WorkerResponse, ImportError> {
    match receiver.recv_timeout(grace) {
        Ok(_) | Err(mpsc::RecvTimeoutError::Disconnected) => {
            join_worker(handle)?;
        }
        Err(mpsc::RecvTimeoutError::Timeout) => {
            session.preserve_for_recovery();
            drop(handle);
        }
    }
    Err(ImportError::new(code, message))
}

fn join_worker(handle: thread::JoinHandle<()>) -> Result<(), ImportError> {
    handle.join().map_err(|_| {
        ImportError::new(
            ImportErrorCode::WorkerFailed,
            "worker panicked inside its supervised execution boundary",
        )
    })
}

fn cancelled<T>() -> Result<T, ImportError> {
    Err(ImportError::new(
        ImportErrorCode::Cancelled,
        "import was cancelled",
    ))
}

#[cfg(test)]
mod tests {
    use super::{CancellationToken, ImportPipeline, IntakeRequest, source_input_locator};
    use crate::{
        AsciiDocV1ProposalValidator, CanonicalProposalValidator, FakeAdapter, FakeWorker,
        ImportErrorCode, ImportLimits, ImportTempRoot, OriginClass, PlanRequest, PortablePath,
        SourceFormat,
    };
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn probed_pdf_uses_a_fixed_pdf_worker_locator() {
        assert_eq!(
            source_input_locator(SourceFormat::Pdf)
                .expect("PDF locator")
                .as_str(),
            "input/source.pdf"
        );
        assert_eq!(
            source_input_locator(SourceFormat::Markdown)
                .expect("generic locator")
                .as_str(),
            "input/source.bin"
        );
    }

    #[test]
    fn frozen_plan_rendering_is_byte_deterministic_without_reusing_identity() {
        let base = tempfile::tempdir().expect("temp directory");
        let pipeline = pipeline(&base);

        let first = run_success(&pipeline).expect("first preview");
        let rerendered = AsciiDocV1ProposalValidator
            .render_and_validate(
                &first.source,
                &fixture_bytes(),
                &first.plan,
                &first.document,
            )
            .expect("rerender frozen plan");

        assert_eq!(first.proposal.proposal(), rerendered.proposal());
        assert_eq!(
            first.proposal.proposal_digest(),
            rerendered.proposal_digest()
        );

        let second = run_success(&pipeline).expect("second preview");

        assert_eq!(first.document, second.document);
        assert_ne!(first.plan.proposed_root_id, second.plan.proposed_root_id);
        assert_ne!(
            first.proposal.proposal().nodes[0].node_id,
            second.proposal.proposal().nodes[0].node_id
        );
        assert_eq!(session_directory_count(&base), 0);
    }

    #[test]
    fn cancellation_stops_the_worker_and_cleans_temporary_state() {
        let base = tempfile::tempdir().expect("temp directory");
        let pipeline = pipeline(&base);
        let cancellation = CancellationToken::default();
        let trigger = cancellation.clone();
        let thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            trigger.cancel();
        });

        let error = pipeline
            .preview(
                IntakeRequest {
                    display_name: "cancel.fake".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes: fixture_bytes(),
                    plan: request(),
                    limits: ImportLimits::default(),
                    cancellation,
                },
                &FakeAdapter,
                Arc::new(FakeWorker::wait_until_cancelled(Duration::from_millis(1))),
            )
            .expect_err("cancelled import");
        thread.join().expect("cancellation trigger");

        assert_eq!(error.code(), ImportErrorCode::Cancelled);
        assert_eq!(session_directory_count(&base), 0);
    }

    #[test]
    fn timeout_stops_the_worker_and_cleans_temporary_state() {
        let base = tempfile::tempdir().expect("temp directory");
        let pipeline = pipeline(&base);
        let limits = ImportLimits {
            worker_timeout_ms: 20,
            cancellation_grace_ms: 200,
            ..ImportLimits::default()
        };

        let error = pipeline
            .preview(
                IntakeRequest {
                    display_name: "timeout.fake".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes: fixture_bytes(),
                    plan: request(),
                    limits,
                    cancellation: CancellationToken::default(),
                },
                &FakeAdapter,
                Arc::new(FakeWorker::wait_until_cancelled(Duration::from_millis(1))),
            )
            .expect_err("timed out import");

        assert_eq!(error.code(), ImportErrorCode::TimedOut);
        assert_eq!(session_directory_count(&base), 0);
    }

    #[test]
    fn worker_panic_is_contained_and_temporary_state_is_cleaned() {
        let base = tempfile::tempdir().expect("temp directory");
        let pipeline = pipeline(&base);

        let error = pipeline
            .preview(
                IntakeRequest {
                    display_name: "panic.fake".to_owned(),
                    origin: OriginClass::TestFixture,
                    bytes: fixture_bytes(),
                    plan: request(),
                    limits: ImportLimits::default(),
                    cancellation: CancellationToken::default(),
                },
                &FakeAdapter,
                Arc::new(FakeWorker::panic()),
            )
            .expect_err("panicking worker");

        assert_eq!(error.code(), ImportErrorCode::WorkerFailed);
        assert_eq!(session_directory_count(&base), 0);
    }

    fn pipeline(base: &tempfile::TempDir) -> ImportPipeline {
        let root =
            ImportTempRoot::initialize(base.path().join("import-temp")).expect("import temp root");
        ImportPipeline::new(root, Arc::new(AsciiDocV1ProposalValidator))
    }

    fn run_success(
        pipeline: &ImportPipeline,
    ) -> Result<super::PreviewedImport, crate::ImportError> {
        pipeline.preview(
            IntakeRequest {
                display_name: "deterministic.fake".to_owned(),
                origin: OriginClass::TestFixture,
                bytes: fixture_bytes(),
                plan: request(),
                limits: ImportLimits::default(),
                cancellation: CancellationToken::default(),
            },
            &FakeAdapter,
            Arc::new(FakeWorker::success()),
        )
    }

    fn fixture_bytes() -> Vec<u8> {
        b"WEFTEXT-FAKE/1\nDeterministic\nSame input, same IR and exact source.\n".to_vec()
    }

    fn request() -> PlanRequest {
        PlanRequest::single_node(PortablePath::parse("Imported").expect("path"))
    }

    fn session_directory_count(base: &tempfile::TempDir) -> usize {
        std::fs::read_dir(base.path().join("import-temp"))
            .expect("read import temp root")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.starts_with("session-"))
            })
            .count()
    }
}
