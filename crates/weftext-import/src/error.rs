use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportErrorCode {
    InvalidContract,
    InvalidDigest,
    InvalidPath,
    InvalidSource,
    LimitExceeded,
    UnsupportedFormat,
    CapabilityUnavailable,
    ProbeRejected,
    WorkerFailed,
    WorkerProtocol,
    Cancelled,
    TimedOut,
    TemporaryStorage,
    InvalidIr,
    InvalidProposal,
    StaleAgentPatch,
    AgentPatchOutOfScope,
    InvalidAgentPatch,
    Serialization,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportError {
    code: ImportErrorCode,
    message: String,
}

impl ImportError {
    #[must_use]
    pub fn new(code: ImportErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn code(&self) -> ImportErrorCode {
        self.code
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn io(operation: &str, error: &std::io::Error) -> Self {
        Self::new(
            ImportErrorCode::TemporaryStorage,
            format!("{operation}: {error}"),
        )
    }

    pub(crate) fn serialization(error: &serde_json::Error) -> Self {
        Self::new(ImportErrorCode::Serialization, error.to_string())
    }
}

impl fmt::Display for ImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl std::error::Error for ImportError {}
