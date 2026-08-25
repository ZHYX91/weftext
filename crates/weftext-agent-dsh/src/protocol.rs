use std::collections::BTreeSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use weftext_agent::{CancellationMode, HarnessCapabilities, HarnessHandshake};

use crate::DshError;

/// Wire-stable server identity documented by DSH.
pub const DSH_RUNTIME_NAME: &str = "deepseek-harness-sdk-runtime";
/// DSH's current preview wire version documented by its SDK protocol.
pub const DSH_WIRE_VERSION: &str = "0.0.1";

/// Parameters for the process-wide DSH SDK initialization request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DshInitialize {
    pub cwd: PathBuf,
    pub provider: String,
    pub model: String,
    pub max_tokens: Option<u64>,
}

impl DshInitialize {
    pub(crate) fn to_params(&self) -> Result<Value, DshError> {
        if self.provider.trim().is_empty() {
            return Err(DshError::InvalidConfiguration(
                "DSH provider cannot be empty".to_owned(),
            ));
        }
        if self.model.trim().is_empty() {
            return Err(DshError::InvalidConfiguration(
                "DSH model cannot be empty".to_owned(),
            ));
        }
        if self.max_tokens == Some(0) {
            return Err(DshError::InvalidConfiguration(
                "DSH max_tokens must be positive".to_owned(),
            ));
        }
        let cwd = self.cwd.to_str().ok_or_else(|| {
            DshError::InvalidConfiguration("DSH cwd must be valid UTF-8".to_owned())
        })?;
        let mut params = json!({
            "cwd": cwd,
            "provider": self.provider,
            "model": self.model,
        });
        if let Some(max_tokens) = self.max_tokens {
            params["maxTokens"] = json!(max_tokens);
        }
        Ok(params)
    }
}

/// One prompt enqueued into one DSH SDK session.
#[derive(Clone, Debug, PartialEq)]
pub struct DshPrompt {
    pub session_id: String,
    pub content_blocks: Vec<Value>,
}

impl DshPrompt {
    /// Creates one ordinary text prompt.
    #[must_use]
    pub fn text(session_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            content_blocks: vec![json!({"type": "text", "text": text.into()})],
        }
    }

    pub(crate) fn to_params(&self) -> Result<Value, DshError> {
        if self.session_id.trim().is_empty() {
            return Err(DshError::InvalidConfiguration(
                "DSH session_id cannot be empty".to_owned(),
            ));
        }
        if self.content_blocks.is_empty() {
            return Err(DshError::InvalidConfiguration(
                "DSH content_blocks cannot be empty".to_owned(),
            ));
        }
        Ok(json!({
            "sessionId": self.session_id,
            "contentBlocks": self.content_blocks,
        }))
    }
}

/// Durable queue receipt returned by `session/prompt`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DshPromptReceipt {
    pub message_id: String,
}

/// Versions this Weftext adapter has tested and accepts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DshCompatibilityPolicy {
    supported_versions: BTreeSet<String>,
}

impl Default for DshCompatibilityPolicy {
    fn default() -> Self {
        Self::new([DSH_WIRE_VERSION])
    }
}

impl DshCompatibilityPolicy {
    /// Creates an exact allowlist of accepted DSH wire versions.
    #[must_use]
    pub fn new(versions: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            supported_versions: versions.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns the accepted versions in stable order.
    pub fn supported_versions(&self) -> impl Iterator<Item = &str> {
        self.supported_versions.iter().map(String::as_str)
    }

    pub(crate) fn validate(
        &self,
        runtime_name: String,
        runtime_version: String,
    ) -> Result<HarnessHandshake, DshError> {
        if runtime_name != DSH_RUNTIME_NAME {
            return Err(DshError::IncompatibleRuntimeName {
                expected: DSH_RUNTIME_NAME.to_owned(),
                actual: runtime_name,
            });
        }
        if !self.supported_versions.contains(&runtime_version) {
            return Err(DshError::UnsupportedRuntimeVersion {
                actual: runtime_version,
                supported: self.supported_versions.iter().cloned().collect(),
            });
        }
        Ok(HarnessHandshake {
            harness: "dsh".to_owned(),
            runtime_name,
            runtime_version,
            adapter_version: env!("CARGO_PKG_VERSION").to_owned(),
            capabilities: HarnessCapabilities {
                session_events: true,
                status_events: true,
                approval_requests: false,
                cancellation: CancellationMode::RuntimeTermination,
            },
        })
    }
}
