//! The console's sole local-owner transport. Business commands come from the
//! shared fixed registry; no caller can select an arbitrary URI or credential.
use crate::client_protocol::CallKind;
use std::{fmt, path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
pub struct ProfileLocator {
    pub data_dir: PathBuf,
}
impl ProfileLocator {
    pub fn host_default() -> std::io::Result<Self> {
        Ok(Self {
            data_dir: crate::launch::default_data_dir()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionEffect {
    NotSubmitted,
    MayHaveBeenAccepted,
}

#[derive(Debug)]
pub struct LocalClientError {
    pub effect: SubmissionEffect,
    pub code: String,
    pub message: String,
}
impl fmt::Display for LocalClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for LocalClientError {}
impl LocalClientError {
    pub(crate) fn not_submitted(code: &str, message: impl Into<String>) -> Self {
        Self {
            effect: SubmissionEffect::NotSubmitted,
            code: code.into(),
            message: message.into(),
        }
    }
    pub(crate) fn uncertain(message: impl Into<String>) -> Self {
        Self {
            effect: SubmissionEffect::MayHaveBeenAccepted,
            code: "localResultUnknown".into(),
            message: message.into(),
        }
    }
}

/// Deliberately not Clone, Debug or Serialize. Only a fixed remote-sync call
/// accepts this transient sideband; it never enters frozen pure-input records.
pub struct RemoteSyncSecret(Vec<u8>);
impl RemoteSyncSecret {
    pub fn new(bytes: Vec<u8>) -> Result<Self, LocalClientError> {
        let secret = Self(bytes);
        if !(32..=512).contains(&secret.0.len())
            || !secret.0.iter().all(|byte| (0x21..=0x7e).contains(byte))
        {
            return Err(LocalClientError::not_submitted(
                "invalidCredential",
                "The transient sync credential is not a bounded header-safe token",
            ));
        }
        Ok(secret)
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.0
    }
    #[cfg(windows)]
    pub(crate) async fn receive(
        reader: &mut (impl tokio::io::AsyncRead + Unpin),
        length: u32,
    ) -> std::io::Result<Self> {
        use tokio::io::AsyncReadExt;
        if !(32..=512).contains(&length) {
            return Err(std::io::Error::other(
                "Transient credential length is invalid",
            ));
        }
        let mut secret = Self(vec![0; length as usize]);
        reader.read_exact(&mut secret.0).await?;
        if !secret.0.iter().all(|byte| (0x21..=0x7e).contains(byte)) {
            return Err(std::io::Error::other(
                "Transient credential bytes are invalid",
            ));
        }
        Ok(secret)
    }
}
impl Drop for RemoteSyncSecret {
    fn drop(&mut self) {
        for byte in &mut self.0 {
            // Volatile stores keep this owned allocation's clearing observable;
            // HTTP/OS buffers may have separate copies and are not claimed wiped.
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
    }
}

pub struct LocalCall {
    pub request_id: String,
    pub kind: CallKind,
    pub body: Vec<u8>,
    pub secret: Option<RemoteSyncSecret>,
}

pub struct LocalReply {
    pub request_id: String,
    pub owner_incarnation: String,
    pub http_status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnerIdentity {
    pub pid: u32,
    pub created: u64,
    pub incarnation: String,
    pub profile_identity: String,
    pub role: OwnerRole,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OwnerRole {
    Local,
    Hosted,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownReceipt {
    pub owner: OwnerIdentity,
    pub exit_code: u32,
}

pub struct LocalConnection {
    #[cfg(windows)]
    inner: crate::broker::windows::Connection,
}
impl LocalConnection {
    pub fn owner_identity(&self) -> OwnerIdentity {
        #[cfg(windows)]
        {
            self.inner.owner_identity()
        }
        #[cfg(not(windows))]
        {
            unreachable!("No local connection exists on an unsupported host")
        }
    }
    pub fn profile_identity(&self) -> &str {
        #[cfg(windows)]
        {
            self.inner.profile_identity()
        }
        #[cfg(not(windows))]
        {
            unreachable!("No local connection is created on an unsupported host")
        }
    }

    pub async fn call(&mut self, call: LocalCall) -> Result<LocalReply, LocalClientError> {
        #[cfg(windows)]
        {
            self.inner.call(call).await
        }
        #[cfg(not(windows))]
        {
            let _ = call;
            Err(LocalClientError::not_submitted(
                "unsupportedPlatform",
                "The local owner requires the qualified Windows backend",
            ))
        }
    }
}

pub async fn shutdown_existing(
    profile: &ProfileLocator,
    expected: &OwnerIdentity,
    limit: Duration,
) -> Result<ShutdownReceipt, LocalClientError> {
    #[cfg(windows)]
    {
        crate::broker::windows::shutdown_existing(profile, expected, limit).await
    }
    #[cfg(not(windows))]
    {
        let _ = (profile, expected, limit);
        Err(LocalClientError::not_submitted(
            "unsupportedPlatform",
            "Existing local owner control requires Windows",
        ))
    }
}

pub async fn inspect_existing(profile: &ProfileLocator) -> Result<OwnerIdentity, LocalClientError> {
    #[cfg(windows)]
    {
        crate::broker::windows::inspect_existing(profile).await
    }
    #[cfg(not(windows))]
    {
        let _ = profile;
        Err(LocalClientError::not_submitted(
            "unsupportedPlatform",
            "Existing owner inspection requires Windows",
        ))
    }
}

pub async fn connect_or_start(
    profile: &ProfileLocator,
    limit: Duration,
) -> Result<LocalConnection, LocalClientError> {
    #[cfg(windows)]
    {
        Ok(LocalConnection {
            inner: crate::broker::windows::connect_or_start(profile, limit).await?,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = (profile, limit);
        Err(LocalClientError::not_submitted(
            "unsupportedPlatform",
            "The local owner requires the qualified Windows backend",
        ))
    }
}
