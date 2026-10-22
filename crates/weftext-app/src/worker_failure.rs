//! Private original worker facts. Binary output is not an OS diagnostic or a
//! product message, and is never serialized into the public API error details.
use std::{fmt, io};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
