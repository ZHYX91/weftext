//! Closed Linux product-worker launch boundary. This crate owns no Core data.
use serde::{Deserialize, Serialize};
use std::fmt;

pub const MAX_FRAME: usize = 4096;
pub const MAX_TIMEOUT_MS: u64 = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Unsupported,
    Unavailable,
    Rejected,
    Io,
    Unknown,
}
#[derive(Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
}
impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn unknown(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unknown, message)
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::new(ErrorKind::Io, value.to_string())
    }
}
impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::new(ErrorKind::Rejected, value.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WorkerKind {
    Render,
    Notes,
    Recognition,
}
impl WorkerKind {
    pub fn fixed_argument(self) -> Result<&'static str> {
        match self {
            Self::Render => Ok("--render-worker"),
            Self::Notes => Ok("--notes-edit-worker"),
            Self::Recognition => Err(Error::new(
                ErrorKind::Unavailable,
                "no approved Linux recognition runtime",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Launch {
    pub version: u32,
    pub kind: WorkerKind,
    pub timeout_ms: u64,
}
impl Launch {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1 || self.timeout_ms == 0 || self.timeout_ms > MAX_TIMEOUT_MS {
            return Err(Error::new(
                ErrorKind::Rejected,
                "invalid launch version or timeout",
            ));
        }
        self.kind.fixed_argument()?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Cancel {
    pub version: u32,
    pub cancel: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Started {
    pub version: u32,
    pub job_id: String,
    pub pid: i32,
    pub image_sha256: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Completion {
    Exited,
    Cancelled,
    Timeout,
    Disconnected,
    SetupFailed,
}

/// Sent only after cgroup.events says populated=0 and all owned children reaped.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Receipt {
    pub version: u32,
    pub job_id: String,
    pub completion: Completion,
    pub exit_code: Option<i32>,
    pub signal: Option<i32>,
    pub reaped: u32,
    pub cgroup_empty: bool,
    pub pidfd_exited: bool,
}

pub fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T> {
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err(Error::new(ErrorKind::Rejected, "empty or oversized frame"));
    }
    Ok(serde_json::from_slice(bytes)?)
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{BrokerClient, OwnedWorker, run_daemon};

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn strict_closed_frames() {
        for raw in [
            br#"{"version":1,"version":1,"kind":"render","timeoutMs":1}"#.as_slice(),
            br#"{"version":1,"kind":"render","timeoutMs":1,"exe":"/bin/sh"}"#,
            br#"{"version":1,"kind":"shell","timeoutMs":1}"#,
            br#"{"version":1,"kind":"render","timeoutMs":1}{}"#,
            br#"{"version":1,"kind":"render","timeoutMs":-1}"#,
            b"",
        ] {
            assert!(decode::<Launch>(raw).is_err());
        }
        assert!(decode::<Launch>(&vec![b' '; MAX_FRAME + 1]).is_err());
        for timeout_ms in [0, MAX_TIMEOUT_MS + 1] {
            assert!(
                Launch {
                    version: 1,
                    kind: WorkerKind::Render,
                    timeout_ms
                }
                .validate()
                .is_err()
            );
        }
        assert!(
            Launch {
                version: 1,
                kind: WorkerKind::Render,
                timeout_ms: 1
            }
            .validate()
            .is_ok()
        );
        assert!(WorkerKind::Recognition.fixed_argument().is_err());
        assert!(decode::<Cancel>(br#"{"version":1,"cancel":true,"cancel":true}"#).is_err());
    }
}
