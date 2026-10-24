use super::*;
use crate::{ApiError, ApiErrorDetails};
use axum::{http::StatusCode, response::IntoResponse};
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use weftext_messages::{DiagnosticOrigin, ErrorDescription, UiMessage};

const PRIVATE: &str = "PRIVATE-WORKER-DIAGNOSTIC-ORIGINAL";

pub(crate) fn original_failure(kind: Kind) -> ApiError {
    let mut failure = WorkerFailure::new(kind);
    failure.stdin_complete = true;
    failure.stdout = StreamCapture {
        bytes: [PRIVATE.as_bytes(), b"\0\xff-source"].concat(),
        complete: true,
        truncated: false,
    };
    failure.stderr = StreamCapture {
        bytes: [PRIVATE.as_bytes(), b" C:\\private\\source SECRET=raw\0\xff"].concat(),
        complete: false,
        truncated: true,
    };
    failure.cleanup_confirmed = kind != Kind::SupervisionLost;
    failure.io_error = Some(io::Error::other(format!("{PRIVATE} private input path")));
    failure.cleanup_error = (!failure.cleanup_confirmed)
        .then(|| io::Error::other(format!("{PRIVATE} private cleanup path")));
    #[cfg(windows)]
    {
        failure.exit = Some(crate::owned_process::ProcessExit::Windows { code: u32::MAX });
    }
    #[cfg(target_os = "linux")]
    {
        failure.exit = Some(crate::owned_process::ProcessExit::Linux {
            code: None,
            signal: Some(9),
        });
    }
    let mut original = ApiError::worker_failure(failure).diagnostic(DiagnosticOrigin::Os, PRIVATE);
    original.details = Some(Box::new(ApiErrorDetails::CurrentSource {
        current_source: PRIVATE.into(),
    }));
    original.causes = ErrorDescription::new(original.explanation.clone())
        .try_caused_by(
            "privateOriginalCause",
            ErrorDescription::new(UiMessage::error_compiler_output_invalid())
                .diagnostic(DiagnosticOrigin::Json, PRIVATE),
        )
        .unwrap()
        .causes;
    original
}

pub(crate) async fn assert_public_response(
    error: ApiError,
    status: StatusCode,
    code: &str,
    explanation: UiMessage,
) {
    let response = error.into_response();
    assert_eq!(response.status(), status);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        json!({"error":{"code":code,"explanation":explanation,"diagnostics":[],"causes":[]}})
    );
    assert!(!std::str::from_utf8(&bytes).unwrap().contains(PRIVATE));
}
pub(crate) async fn assert_safe_wrapper(
    error: ApiError,
    retained: &Arc<WorkerFailure>,
    status: StatusCode,
    code: &str,
    explanation: UiMessage,
) {
    assert!(Arc::ptr_eq(
        error.private_worker_failure.as_ref().unwrap(),
        retained
    ));
    let source = std::error::Error::source(&error)
        .unwrap()
        .downcast_ref::<WorkerFailure>()
        .unwrap();
    assert!(std::ptr::eq(source, retained.as_ref()));
    assert!(!format!("{error:?}").contains(PRIVATE));
    assert!(!format!("{error:?}").contains("C:\\private"));
    let raw = retained.stderr.bytes.clone();
    let cleanup = retained.cleanup_confirmed;
    assert_public_response(error, status, code, explanation).await;
    assert_eq!(retained.stderr.bytes, raw);
    assert_eq!(retained.cleanup_confirmed, cleanup);
}

fn summary(context: HostedContext, error: &ApiError) -> DiagnosticLine {
    diagnostic_line(context, error.private_worker_failure.as_ref().unwrap()).unwrap()
}
fn keys(value: &Value) -> Vec<&str> {
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    keys
}

#[test]
fn summary_is_closed_bounded_and_never_serializes_original_error_or_streams() {
    let kinds = [
        Kind::Setup,
        Kind::InputEncoding,
        Kind::InputBudget,
        Kind::InputIo,
        Kind::StdoutIo,
        Kind::StderrIo,
        Kind::StdoutBudget,
        Kind::StderrBudget,
        Kind::Deadline,
        #[cfg(target_os = "linux")]
        Kind::Cancelled,
        Kind::Wait,
        Kind::NonzeroExit,
        Kind::OutputRejected,
        Kind::SupervisionLost,
    ];
    let contexts = [
        HostedContext::Editor,
        HostedContext::Preview,
        HostedContext::CitationRead,
        HostedContext::CitationEdit,
        HostedContext::NativeNotes,
    ];
    for kind in kinds {
        let error = original_failure(kind);
        let original = error.private_worker_failure.as_ref().unwrap();
        for context in contexts {
            let line = summary(context, &error);
            assert!(line.length <= DIAGNOSTIC_BYTES && line.as_bytes().ends_with(b"\n"));
            let text = std::str::from_utf8(line.as_bytes()).unwrap();
            for hidden in [
                PRIVATE,
                "C:\\private",
                "SECRET",
                "input path",
                "cleanup path",
            ] {
                assert!(!text.contains(hidden));
            }
            let value: Value = serde_json::from_slice(line.as_bytes()).unwrap();
            assert_eq!(
                keys(&value),
                [
                    "cleanupConfirmed",
                    "cleanupIo",
                    #[cfg(target_os = "linux")]
                    "completion",
                    "context",
                    "event",
                    "exit",
                    "io",
                    "kind",
                    "schemaVersion",
                    "stderr",
                    "stdinComplete",
                    "stdout"
                ]
            );
            assert_eq!(value["event"], "weftextWorkerFailure");
            assert_eq!(value["schemaVersion"], 1);
            assert_eq!(value["kind"], serde_json::to_value(kind).unwrap());
            assert_eq!(value["context"], serde_json::to_value(context).unwrap());
            assert_eq!(keys(&value["stdout"]), ["bytes", "complete", "truncated"]);
            assert_eq!(
                value["stdout"]["bytes"].as_u64(),
                Some(original.stdout.bytes.len() as u64)
            );
            assert_eq!(value["stdout"]["complete"], true);
            assert_eq!(value["stderr"]["complete"], false);
            assert_eq!(value["stderr"]["truncated"], true);
            assert_eq!(value["cleanupConfirmed"], original.cleanup_confirmed);
            assert_eq!(keys(&value["io"]), ["kind", "osCode"]);
            assert_eq!(value["io"]["kind"], "Other");
            assert_eq!(value["io"]["osCode"], Value::Null);
        }
    }
}

#[cfg(windows)]
#[test]
fn summary_preserves_original_full_unsigned_windows_exit_codes() {
    for code in [0xc0000005, u32::MAX] {
        let mut failure = WorkerFailure::new(Kind::NonzeroExit);
        failure.exit = Some(crate::owned_process::ProcessExit::Windows { code });
        failure.io_error = Some(io::Error::from_raw_os_error(5));
        let line = diagnostic_line(HostedContext::Preview, &failure).unwrap();
        let value: Value = serde_json::from_slice(line.as_bytes()).unwrap();
        assert_eq!(value["exit"], json!({"platform":"windows","code":code}));
        assert_eq!(value["exit"]["code"].as_u64(), Some(u64::from(code)));
        assert_eq!(value["io"]["osCode"], 5);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn summary_preserves_the_original_linux_code_or_signal_without_reinterpreting() {
    for exit in [
        crate::owned_process::ProcessExit::Linux {
            code: Some(7),
            signal: None,
        },
        crate::owned_process::ProcessExit::Linux {
            code: None,
            signal: Some(9),
        },
    ] {
        let mut failure = WorkerFailure::new(Kind::NonzeroExit);
        failure.exit = Some(exit);
        let line = diagnostic_line(HostedContext::Preview, &failure).unwrap();
        let value: Value = serde_json::from_slice(line.as_bytes()).unwrap();
        let crate::owned_process::ProcessExit::Linux { code, signal } = exit;
        assert_eq!(
            value["exit"],
            json!({"platform":"linux","code":code,"signal":signal})
        );
    }
}

#[test]
fn summary_never_invents_exit_or_cleanup_confirmation() {
    let mut failure = WorkerFailure::new(Kind::SupervisionLost);
    failure.cleanup_confirmed = false;
    failure.cleanup_error = Some(io::Error::from_raw_os_error(5));
    let value: Value = serde_json::from_slice(
        diagnostic_line(HostedContext::CitationEdit, &failure)
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(value["kind"], "supervisionLost");
    assert_eq!(value["exit"], Value::Null);
    assert_eq!(value["cleanupConfirmed"], false);
    assert_eq!(value["cleanupIo"]["osCode"], 5);
    assert_eq!(value["io"], Value::Null);
}

#[test]
fn largest_scalar_counts_and_os_codes_still_fit_one_fixed_record() {
    let stream = || StreamSummary {
        bytes: usize::MAX,
        complete: false,
        truncated: true,
    };
    let mut safe = SafeSummary {
        schema_version: 1,
        event: "weftextWorkerFailure",
        context: HostedContext::CitationRead,
        kind: Kind::SupervisionLost,
        exit: None,
        #[cfg(target_os = "linux")]
        completion: Some(weftext_worker_broker::Completion::SetupFailed),
        stdin_complete: false,
        stdout: stream(),
        stderr: stream(),
        cleanup_confirmed: false,
        io: Some(IoSummary {
            kind: IoKind(io::ErrorKind::PermissionDenied),
            os_code: Some(i32::MAX),
        }),
        cleanup_io: Some(IoSummary {
            kind: IoKind(io::ErrorKind::UnexpectedEof),
            os_code: Some(i32::MIN),
        }),
    };
    #[cfg(windows)]
    {
        safe.exit = Some(ExitSummary::Windows { code: u32::MAX });
    }
    #[cfg(target_os = "linux")]
    {
        safe.exit = Some(ExitSummary::Linux {
            code: None,
            signal: Some(i32::MAX),
        });
    }
    let mut line = DiagnosticLine::new();
    serde_json::to_writer(&mut line, &safe).unwrap();
    io::Write::write_all(&mut line, b"\n").unwrap();
    assert!(line.length <= DIAGNOSTIC_BYTES);
    let value: Value = serde_json::from_slice(line.as_bytes()).unwrap();
    assert_eq!(value["stdout"]["bytes"].as_u64(), Some(usize::MAX as u64));
    assert_eq!(value["cleanupConfirmed"], false);
    assert_eq!(value["cleanupIo"]["osCode"], i32::MIN);
}

#[test]
fn observer_emits_only_for_original_private_failure_and_leaves_result_unchanged() {
    let mut events = Vec::new();
    for value in [
        json!({"ok":true,"source":PRIVATE}),
        json!({"ok":false,"diagnostics":[PRIVATE]}),
    ] {
        let result: Result<_, ApiError> = Ok(value.clone());
        observe_with(HostedContext::Editor, &result, |line| events.push(line));
        let Ok(actual) = result else {
            panic!("Observer changed the successful result")
        };
        assert_eq!(actual, value);
    }
    for error in [
        ApiError::bad(UiMessage::error_compiler_output_invalid()),
        ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "busy",
            UiMessage::error_api_hosted_editor_preview_busy(),
        ),
    ] {
        let result: Result<Value, _> = Err(error);
        observe_with(HostedContext::Preview, &result, |line| events.push(line));
        let Err(original) = result else {
            panic!("Observer changed the error result")
        };
        assert!(original.private_worker_failure.is_none());
    }
    assert!(events.is_empty());
    let error = original_failure(Kind::Deadline);
    let retained = error.private_worker_failure.as_ref().unwrap().clone();
    let result: Result<Value, _> = Err(error);
    observe_with(HostedContext::CitationRead, &result, |line| {
        events.push(line)
    });
    assert_eq!(events.len(), 1);
    let Err(original) = result else {
        panic!("Observer changed the worker error result")
    };
    assert!(Arc::ptr_eq(
        original.private_worker_failure.as_ref().unwrap(),
        &retained
    ));
}

#[tokio::test]
async fn observed_busy_keeps_its_actual_public_response_without_a_fake_worker() {
    let result: Result<Value, _> = Err(ApiError::new(
        StatusCode::TOO_MANY_REQUESTS,
        "busy",
        UiMessage::error_api_hosted_preview_busy(),
    ));
    observe_with(HostedContext::Preview, &result, |_| {
        panic!("No worker was started")
    });
    let Err(error) = result else {
        panic!("Observer changed the busy error result")
    };
    assert!(error.private_worker_failure.is_none());
    assert_public_response(
        error,
        StatusCode::TOO_MANY_REQUESTS,
        "busy",
        UiMessage::error_api_hosted_preview_busy(),
    )
    .await;
}

struct FailingWriter {
    fail_write: bool,
    calls: Arc<(AtomicUsize, AtomicUsize)>,
}
impl io::Write for FailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls.0.fetch_add(1, Ordering::SeqCst);
        if self.fail_write {
            Err(io::ErrorKind::BrokenPipe.into())
        } else {
            Ok(bytes.len())
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.calls.1.fetch_add(1, Ordering::SeqCst);
        Err(io::ErrorKind::BrokenPipe.into())
    }
}
#[tokio::test]
async fn broken_write_and_flush_cannot_replace_original_worker_or_safe_response() {
    for fail_write in [true, false] {
        let calls = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
        let (queue, writer) = DiagnosticQueue::start(FailingWriter {
            fail_write,
            calls: calls.clone(),
        })
        .unwrap();
        let error = original_failure(Kind::SupervisionLost);
        let retained = error.private_worker_failure.as_ref().unwrap().clone();
        let result: Result<Value, _> = Err(error);
        observe_with(HostedContext::Editor, &result, |line| {
            assert!(queue.try_submit(line));
        });
        drop(queue);
        writer.join().unwrap();
        assert_eq!(calls.0.load(Ordering::SeqCst), 1);
        assert_eq!(calls.1.load(Ordering::SeqCst), usize::from(!fail_write));
        let Err(original) = result else {
            panic!("Observer changed the worker error result")
        };
        let safe = ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "previewUnavailable",
            UiMessage::error_api_hosted_editor_authorized_preview_unavailable(),
        )
        .inherit_worker_failure_from(&original);
        assert!(!retained.cleanup_confirmed);
        assert_safe_wrapper(
            safe,
            &retained,
            StatusCode::UNPROCESSABLE_ENTITY,
            "previewUnavailable",
            UiMessage::error_api_hosted_editor_authorized_preview_unavailable(),
        )
        .await;
    }
}

struct BlockedWriter {
    entered: mpsc::SyncSender<()>,
    release: mpsc::Receiver<()>,
    first: bool,
}
impl io::Write for BlockedWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.first {
            self.first = false;
            let _ = self.entered.try_send(());
            let _ = self.release.recv();
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[test]
fn a_blocked_sink_never_makes_the_caller_wait_and_pending_records_stay_bounded() {
    use std::time::Duration;
    let (entered, waiting) = mpsc::sync_channel(1);
    let (release, blocked) = mpsc::sync_channel(1);
    let (queue, writer) = DiagnosticQueue::start(BlockedWriter {
        entered,
        release: blocked,
        first: true,
    })
    .unwrap();
    let error = original_failure(Kind::Deadline);
    assert!(queue.try_submit(summary(HostedContext::Preview, &error)));
    let started = waiting.recv_timeout(Duration::from_secs(2));
    let caller_queue = queue.clone();
    let (done, completion) = mpsc::sync_channel(1);
    let caller = std::thread::spawn(move || {
        let error = original_failure(Kind::Deadline);
        let retained = error.private_worker_failure.as_ref().unwrap().clone();
        let result: Result<Value, _> = Err(error);
        let mut accepted = 0;
        for _ in 0..=DIAGNOSTIC_PENDING {
            observe_with(HostedContext::Preview, &result, |line| {
                accepted += usize::from(caller_queue.try_submit(line));
            });
        }
        let Err(original) = result else {
            panic!("Observer changed the worker error result")
        };
        let same = Arc::ptr_eq(original.private_worker_failure.as_ref().unwrap(), &retained);
        let _ = done.try_send((accepted, same));
    });
    let observed = completion.recv_timeout(Duration::from_secs(2));
    // Always unblock before any assertion or joining, including a regression.
    let _ = release.try_send(());
    drop(queue);
    caller.join().unwrap();
    writer.join().unwrap();
    assert!(started.is_ok());
    assert_eq!(observed.unwrap(), (DIAGNOSTIC_PENDING, true));
}

#[test]
fn summary_queue_owns_only_safe_fixed_records_and_drops_a_disconnected_sink() {
    let (sender, receiver) = mpsc::sync_channel(DIAGNOSTIC_PENDING);
    drop(receiver);
    let queue = DiagnosticQueue { sender };
    let error = original_failure(Kind::OutputRejected);
    let retained = error.private_worker_failure.as_ref().unwrap();
    let count = Arc::strong_count(retained);
    assert!(!queue.try_submit(summary(HostedContext::CitationEdit, &error)));
    assert_eq!(Arc::strong_count(retained), count);
    assert!(
        std::mem::size_of::<DiagnosticLine>() <= DIAGNOSTIC_BYTES + std::mem::size_of::<usize>()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn safe_summary_retains_every_typed_broker_completion() {
    use weftext_worker_broker::Completion;
    for completion in [
        Completion::Exited,
        Completion::Cancelled,
        Completion::Timeout,
        Completion::Disconnected,
        Completion::SetupFailed,
    ] {
        let mut failure = WorkerFailure::new(Kind::SupervisionLost);
        failure.completion = Some(completion.clone());
        let value: Value = serde_json::from_slice(
            diagnostic_line(HostedContext::Editor, &failure)
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        assert_eq!(
            value["completion"],
            serde_json::to_value(completion).unwrap()
        );
        assert_eq!(value["exit"], Value::Null);
    }
}
