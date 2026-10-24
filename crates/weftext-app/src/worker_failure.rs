//! Private original worker facts. Binary output is not an OS diagnostic or a
//! product message, and is never serialized into the public API error details.
use std::{fmt, io};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Kind {
    Setup,
    InputEncoding,
    InputBudget,
    InputIo,
    StdoutIo,
    StderrIo,
    StdoutBudget,
    StderrBudget,
    Deadline,
    #[cfg(target_os = "linux")]
    Cancelled,
    Wait,
    NonzeroExit,
    OutputRejected,
    SupervisionLost,
}
#[derive(Default)]
pub(crate) struct StreamCapture {
    pub bytes: Vec<u8>,
    pub complete: bool,
    pub truncated: bool,
}
impl fmt::Debug for StreamCapture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StreamCapture")
            .field("bytes", &self.bytes.len())
            .field("complete", &self.complete)
            .field("truncated", &self.truncated)
            .finish()
    }
}
pub(crate) struct WorkerFailure {
    pub kind: Kind,
    pub exit: Option<crate::owned_process::ProcessExit>,
    #[cfg(target_os = "linux")]
    pub completion: Option<weftext_worker_broker::Completion>,
    pub stdin_complete: bool,
    pub stdout: StreamCapture,
    pub stderr: StreamCapture,
    pub cleanup_confirmed: bool,
    pub io_error: Option<io::Error>,
    pub cleanup_error: Option<io::Error>,
}
impl WorkerFailure {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            exit: None,
            #[cfg(target_os = "linux")]
            completion: None,
            stdin_complete: false,
            stdout: Default::default(),
            stderr: Default::default(),
            cleanup_confirmed: true,
            io_error: None,
            cleanup_error: None,
        }
    }
}
impl fmt::Debug for WorkerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WorkerFailure")
            .field("kind", &self.kind)
            .field("exit", &self.exit)
            .field("stdin_complete", &self.stdin_complete)
            .field("stdout", &self.stdout)
            .field("stderr", &self.stderr)
            .field("cleanup_confirmed", &self.cleanup_confirmed)
            .field(
                "io_error_kind",
                &self.io_error.as_ref().map(io::Error::kind),
            )
            .field(
                "io_error_code",
                &self.io_error.as_ref().and_then(io::Error::raw_os_error),
            )
            .field(
                "cleanup_error_kind",
                &self.cleanup_error.as_ref().map(io::Error::kind),
            )
            .field(
                "cleanup_error_code",
                &self
                    .cleanup_error
                    .as_ref()
                    .and_then(io::Error::raw_os_error),
            )
            .finish()
    }
}
impl fmt::Display for WorkerFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Owned worker failure: {:?}; cleanup confirmed: {}",
            self.kind, self.cleanup_confirmed
        )
    }
}
impl std::error::Error for WorkerFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.io_error
            .as_ref()
            .or(self.cleanup_error.as_ref())
            .map(|error| error as &dyn std::error::Error)
    }
}

// Only these hosted call sites may describe a worker failure. No caller text
// becomes a context, and public error wrapping does not emit another event.
#[derive(Clone, Copy, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum HostedContext {
    Editor,
    Preview,
    CitationRead,
    CitationEdit,
    NativeNotes,
}

const DIAGNOSTIC_BYTES: usize = 1024;
const DIAGNOSTIC_PENDING: usize = 4;

#[derive(serde::Serialize)]
#[serde(tag = "platform", rename_all = "lowercase")]
enum ExitSummary {
    #[cfg(windows)]
    Windows { code: u32 },
    #[cfg(target_os = "linux")]
    Linux {
        code: Option<i32>,
        signal: Option<i32>,
    },
}
impl From<crate::owned_process::ProcessExit> for ExitSummary {
    fn from(exit: crate::owned_process::ProcessExit) -> Self {
        match exit {
            #[cfg(windows)]
            crate::owned_process::ProcessExit::Windows { code } => Self::Windows { code },
            #[cfg(target_os = "linux")]
            crate::owned_process::ProcessExit::Linux { code, signal } => {
                Self::Linux { code, signal }
            }
        }
    }
}

// Debug here is solely the standard library's typed discriminant, never
// io::Error Display/Debug or an arbitrary source/cleanup error message.
struct IoKind(io::ErrorKind);
impl serde::Serialize for IoKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("{:?}", self.0))
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct IoSummary {
    kind: IoKind,
    os_code: Option<i32>,
}
impl From<&io::Error> for IoSummary {
    fn from(error: &io::Error) -> Self {
        Self {
            kind: IoKind(error.kind()),
            os_code: error.raw_os_error(),
        }
    }
}
#[derive(serde::Serialize)]
struct StreamSummary {
    bytes: usize,
    complete: bool,
    truncated: bool,
}
impl From<&StreamCapture> for StreamSummary {
    fn from(stream: &StreamCapture) -> Self {
        Self {
            bytes: stream.bytes.len(),
            complete: stream.complete,
            truncated: stream.truncated,
        }
    }
}
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SafeSummary {
    schema_version: u8,
    event: &'static str,
    context: HostedContext,
    kind: Kind,
    exit: Option<ExitSummary>,
    #[cfg(target_os = "linux")]
    completion: Option<weftext_worker_broker::Completion>,
    stdin_complete: bool,
    stdout: StreamSummary,
    stderr: StreamSummary,
    cleanup_confirmed: bool,
    io: Option<IoSummary>,
    cleanup_io: Option<IoSummary>,
}

// Queue only a fully formatted fixed-size safe record, never an ApiError,
// Arc<WorkerFailure>, raw stream, request or source object.
#[derive(Clone)]
struct DiagnosticLine {
    bytes: [u8; DIAGNOSTIC_BYTES],
    length: usize,
}
impl DiagnosticLine {
    fn new() -> Self {
        Self {
            bytes: [0; DIAGNOSTIC_BYTES],
            length: 0,
        }
    }
    fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.length]
    }
}
impl io::Write for DiagnosticLine {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.bytes.len() - self.length {
            return Err(io::ErrorKind::WriteZero.into());
        }
        self.bytes[self.length..self.length + bytes.len()].copy_from_slice(bytes);
        self.length += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn diagnostic_line(context: HostedContext, failure: &WorkerFailure) -> Option<DiagnosticLine> {
    use io::Write as _;
    let summary = SafeSummary {
        schema_version: 1,
        event: "weftextWorkerFailure",
        context,
        kind: failure.kind,
        exit: failure.exit.map(ExitSummary::from),
        #[cfg(target_os = "linux")]
        completion: failure.completion.clone(),
        stdin_complete: failure.stdin_complete,
        stdout: StreamSummary::from(&failure.stdout),
        stderr: StreamSummary::from(&failure.stderr),
        cleanup_confirmed: failure.cleanup_confirmed,
        io: failure.io_error.as_ref().map(IoSummary::from),
        cleanup_io: failure.cleanup_error.as_ref().map(IoSummary::from),
    };
    let mut line = DiagnosticLine::new();
    serde_json::to_writer(&mut line, &summary).ok()?;
    line.write_all(b"\n").ok()?;
    Some(line)
}

#[derive(Clone)]
struct DiagnosticQueue {
    sender: std::sync::mpsc::SyncSender<DiagnosticLine>,
}
impl DiagnosticQueue {
    fn start<W: io::Write + Send + 'static>(
        writer: W,
    ) -> io::Result<(Self, std::thread::JoinHandle<()>)> {
        let (sender, received) = std::sync::mpsc::sync_channel(DIAGNOSTIC_PENDING);
        let worker = std::thread::Builder::new()
            .name("weftext-worker-diagnostic".into())
            .spawn(move || write_diagnostics(writer, received))?;
        Ok((Self { sender }, worker))
    }
    fn try_submit(&self, line: DiagnosticLine) -> bool {
        self.sender.try_send(line).is_ok()
    }
}
fn write_diagnostics<W: io::Write>(
    mut writer: W,
    received: std::sync::mpsc::Receiver<DiagnosticLine>,
) {
    for line in received {
        if writer
            .write_all(line.as_bytes())
            .and_then(|()| writer.flush())
            .is_err()
        {
            break;
        }
    }
}
fn stderr_queue() -> Option<&'static DiagnosticQueue> {
    static QUEUE: std::sync::OnceLock<Option<DiagnosticQueue>> = std::sync::OnceLock::new();
    QUEUE
        .get_or_init(|| {
            // One best-effort writer for this process, no restart on sink
            // failure. Never join a blocked diagnostic sink during shutdown.
            DiagnosticQueue::start(io::stderr())
                .ok()
                .map(|(queue, _worker)| queue)
        })
        .as_ref()
}
fn observe_with<T>(
    context: HostedContext,
    result: &Result<T, crate::ApiError>,
    submit: impl FnOnce(DiagnosticLine),
) {
    if let Err(error) = result
        && let Some(failure) = error.private_worker_failure.as_ref()
        && let Some(line) = diagnostic_line(context, failure)
    {
        submit(line);
    }
}
pub(crate) fn observe_hosted_result<T>(
    context: HostedContext,
    result: Result<T, crate::ApiError>,
) -> Result<T, crate::ApiError> {
    observe_with(context, &result, |line| {
        if let Some(queue) = stderr_queue() {
            let _ = queue.try_submit(line);
        }
    });
    result
}

#[cfg(test)]
pub(crate) mod diagnostics_tests;
