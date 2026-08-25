//! Harness-neutral contracts for supervised AI agent actions.
//!
//! The executable mutation broker remains process-local: an opaque Core plan is
//! never serialized or exposed to a harness. A separate control-plane audit
//! authority records bounded, body-free lifecycle evidence and detects broken
//! digest chains on restart.

mod audit;
mod broker;

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use weftext_core::NodeId;

pub use audit::{
    AgentAuditCommitOutcome, AgentAuditConfig, AgentAuditDecision, AgentAuditError,
    AgentAuditEvent, AgentAuditIdentity, AgentAuditLog, AgentAuditRecord, AgentAuditRecovery,
    AgentAuditRecoveryState, audit_target_digest, capability_digest,
};
pub use broker::{
    AgentApprovedAction, AgentAuthorizationContext, AgentBrokerConfig, AgentBrokerError,
    AgentBrokerTime, AgentCommitCompletion, AgentCommitReport, AgentCommitWork,
    AgentDecisionOutcome, AgentMutationBroker, AgentMutationPlan, AgentMutationRecord,
    AgentMutationState, AgentMutationStateKind,
};

/// A capability that may be granted to one delegated agent session.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    ReadWorkspace,
    SearchWorkspace,
    ProposeMutation,
    CommitApprovedMutation,
    ExternalEgress,
}

/// The explicit capability set granted to one actor or delegated session.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrant {
    capabilities: BTreeSet<AgentCapability>,
}

impl CapabilityGrant {
    /// Creates a grant from an explicit capability list.
    #[must_use]
    pub fn new(capabilities: impl IntoIterator<Item = AgentCapability>) -> Self {
        Self {
            capabilities: capabilities.into_iter().collect(),
        }
    }

    /// Returns whether this grant contains the requested capability.
    #[must_use]
    pub fn allows(&self, capability: AgentCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    /// Restricts two grants to their shared capabilities.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            capabilities: self
                .capabilities
                .intersection(&other.capabilities)
                .copied()
                .collect(),
        }
    }

    /// Iterates over granted capabilities in stable order.
    pub fn iter(&self) -> impl Iterator<Item = AgentCapability> + '_ {
        self.capabilities.iter().copied()
    }
}

/// Identifies the harness-controlled origin of a request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentOrigin {
    pub harness: String,
    pub adapter_version: String,
    pub session_id: String,
}

/// One logical resource named in an action without granting filesystem access.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAffectedResource {
    pub logical_id: String,
    pub owner_node_id: Option<NodeId>,
}

/// The primary target shown for an action and its preview effects.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum AgentActionTarget {
    WorkspaceScope,
    Node { node_id: NodeId },
    Resource { resource: AgentAffectedResource },
}

/// Data that an action would send beyond the Weftext trust boundary.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum AgentExternalEgress {
    None,
    Required {
        destination_id: String,
        data_classes: BTreeSet<String>,
    },
}

impl AgentExternalEgress {
    /// Returns whether the action requires the external-egress capability.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        matches!(self, Self::Required { .. })
    }
}

/// The confirmation rule attached to a mutation request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentConfirmationPolicy {
    ExplicitHumanApproval,
}

/// Deterministic identity of the typed agent intent and the exact Core plan it produced.
///
/// Only digests and closed schema identifiers cross the harness boundary. The executable Core
/// plan remains process-local and opaque to the agent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentMutationBinding {
    pub intent_schema: String,
    pub intent_digest: String,
    pub core_plan_schema: String,
    pub core_plan_digest: String,
}

impl AgentMutationBinding {
    /// Hashes one typed intent and one deterministic Core-plan projection with domain separation.
    ///
    /// # Errors
    ///
    /// Returns [`AgentBindingError`] if either typed value cannot be serialized as JSON.
    pub fn from_serializable<I: Serialize, P: Serialize>(
        intent_schema: impl Into<String>,
        intent: &I,
        core_plan_schema: impl Into<String>,
        core_plan: &P,
    ) -> Result<Self, AgentBindingError> {
        let intent_schema = intent_schema.into();
        let core_plan_schema = core_plan_schema.into();
        let intent_digest = digest_typed_material(&intent_schema, intent)?;
        let core_plan_digest = digest_typed_material(&core_plan_schema, core_plan)?;
        Ok(Self {
            intent_schema,
            intent_digest,
            core_plan_schema,
            core_plan_digest,
        })
    }
}

/// Failure to encode deterministic mutation-binding material.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AgentBindingError;

impl fmt::Display for AgentBindingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("could not encode deterministic agent mutation binding")
    }
}

impl std::error::Error for AgentBindingError {}

fn digest_typed_material<T: Serialize>(
    schema: &str,
    material: &T,
) -> Result<String, AgentBindingError> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct DigestMaterial<'a, T> {
        schema: &'a str,
        material: &'a T,
    }

    let bytes =
        serde_json::to_vec(&DigestMaterial { schema, material }).map_err(|_| AgentBindingError)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// One harness-neutral request to propose a typed Core action.
///
/// The executable plan is intentionally absent. It is supplied separately as
/// the generic, process-local payload of [`AgentMutationBroker`].
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentActionRequest {
    pub request_id: String,
    pub human_actor_id: String,
    pub delegated_client_id: String,
    pub workspace_scope_id: String,
    pub origin: AgentOrigin,
    pub required_capability: AgentCapability,
    pub action_id: String,
    pub binding: AgentMutationBinding,
    pub target: AgentActionTarget,
    pub affected_node_ids: Vec<NodeId>,
    pub affected_resources: Vec<AgentAffectedResource>,
    pub base_revision: String,
    pub external_egress: AgentExternalEgress,
    pub risk: AgentActionRisk,
    pub confirmation_policy: AgentConfirmationPolicy,
}

/// The risk class shown before an agent action can commit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionRisk {
    Ordinary,
    Bulk,
    Destructive,
    CrossWorkspace,
    Permission,
    ExternalEgress,
}

/// One human-readable effect in a deterministic Core preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentChangePreview {
    pub target: AgentActionTarget,
    pub summary: String,
}

/// SHA-256 binding of one exact deterministic preview.
#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AgentPreviewDigest(String);

impl AgentPreviewDigest {
    /// Returns the lowercase hexadecimal digest.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_hex(value: String) -> Self {
        Self(value)
    }
}

/// A revision-checked, self-contained action preview awaiting a decision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentActionPreview {
    pub schema: String,
    pub request: AgentActionRequest,
    pub changes: Vec<AgentChangePreview>,
    pub preview_digest: AgentPreviewDigest,
}

/// A human or policy decision for one exact preview.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "decision", deny_unknown_fields)]
pub enum ApprovalDecision {
    Approved {
        human_actor_id: String,
        preview_digest: AgentPreviewDigest,
    },
    Denied {
        human_actor_id: String,
        preview_digest: AgentPreviewDigest,
        reason: String,
    },
}

/// How a harness can stop in-flight work.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationMode {
    Prompt,
    RuntimeTermination,
    Unsupported,
}

/// Runtime capabilities observed and enforced by an adapter.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessCapabilities {
    pub session_events: bool,
    pub status_events: bool,
    pub approval_requests: bool,
    pub cancellation: CancellationMode,
}

/// A validated adapter handshake shown to callers and the UI.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessHandshake {
    pub harness: String,
    pub runtime_name: String,
    pub runtime_version: String,
    pub adapter_version: String,
    pub capabilities: HarnessCapabilities,
}

/// Whole-agent status exposed by a harness event stream.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionStatus {
    Idle,
    Running,
}

/// Events normalized from a harness without interpreting its durable payload schema.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", deny_unknown_fields)]
pub enum AgentRuntimeEvent {
    SessionEvent {
        session_id: String,
        event: Value,
    },
    SessionStatus {
        session_id: String,
        status: AgentSessionStatus,
    },
    SubagentStarted {
        parent_session_id: String,
        child_session_id: String,
    },
    SubagentFinished {
        payload: Value,
    },
    Unknown {
        method: String,
        params: Value,
    },
}

/// Harness-neutral sink used by a runtime bridge to merge lifecycle events
/// into a trusted product controller without exposing host methods to the
/// agent protocol.
pub trait AgentRuntimeController: Send + Sync {
    /// Merges one already-normalized runtime event.
    ///
    /// # Errors
    ///
    /// Returns a redacted controller failure when the event is foreign or the
    /// transient stream is unavailable.
    fn ingest_runtime_event(&self, event: AgentRuntimeEvent) -> Result<(), String>;

    /// Records a stable, redacted adapter failure code.
    ///
    /// # Errors
    ///
    /// Returns a redacted controller failure when durable audit cannot accept
    /// the event.
    fn record_adapter_crash(&self, error_code: &str) -> Result<(), String>;

    /// Records whole-runtime termination used for cancellation.
    ///
    /// # Errors
    ///
    /// Returns a redacted controller failure when durable audit cannot accept
    /// the event.
    fn record_runtime_terminated_for_cancellation(&self) -> Result<(), String>;
}

#[cfg(test)]
mod tests {
    use super::{AgentCapability, AgentMutationBinding, AgentOrigin, CapabilityGrant};

    #[test]
    fn delegated_capabilities_cannot_expand_actor_rights() {
        let actor = CapabilityGrant::new([
            AgentCapability::ReadWorkspace,
            AgentCapability::SearchWorkspace,
        ]);
        let session = CapabilityGrant::new([
            AgentCapability::ReadWorkspace,
            AgentCapability::CommitApprovedMutation,
        ]);

        let effective = actor.intersection(&session);

        assert!(effective.allows(AgentCapability::ReadWorkspace));
        assert!(!effective.allows(AgentCapability::SearchWorkspace));
        assert!(!effective.allows(AgentCapability::CommitApprovedMutation));
    }

    #[test]
    fn public_control_plane_inputs_reject_unknown_fields() {
        assert!(
            serde_json::from_value::<AgentOrigin>(serde_json::json!({
                "harness": "dsh",
                "adapterVersion": "1",
                "sessionId": "session",
                "workspacePath": "C:/must-not-be-authority"
            }))
            .is_err()
        );
        let binding = AgentMutationBinding::from_serializable(
            "weftext.test.intent.v1",
            &"intent",
            "weftext.test.plan.v1",
            &"plan",
        )
        .expect("binding encodes");
        let mut value = serde_json::to_value(binding).expect("binding serializes");
        value
            .as_object_mut()
            .expect("binding is an object")
            .insert("rawSource".to_owned(), serde_json::json!("forbidden"));
        assert!(serde_json::from_value::<AgentMutationBinding>(value).is_err());
    }
}
