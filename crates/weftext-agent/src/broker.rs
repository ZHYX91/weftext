use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AgentActionPreview, AgentActionRequest, AgentActionTarget, AgentAffectedResource,
    AgentCapability, AgentChangePreview, AgentExternalEgress, AgentMutationBinding, AgentOrigin,
    AgentPreviewDigest, ApprovalDecision, CapabilityGrant,
};

const PREVIEW_SCHEMA: &str = "weftext.agent.mutation-preview.v1";
const HARD_MAX_RECORDS: usize = 65_536;
const HARD_MAX_COLLECTION_ITEMS: usize = 16_384;
const HARD_MAX_TEXT_BYTES: usize = 1_048_576;
const HARD_MAX_TTL_MILLIS: u64 = 2_592_000_000;
const INVALID_COMPLETION_ERROR: &str = "invalid_commit_completion";
const COMMIT_TIMEOUT_ERROR: &str = "commit_outcome_timeout";

/// Monotonic controller time in milliseconds.
///
/// The host supplies this value so tests and non-Tokio shells share the same
/// deterministic state machine. A broker rejects time moving backwards.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct AgentBrokerTime(u64);

impl AgentBrokerTime {
    /// Creates a broker timestamp from a monotonic millisecond counter.
    #[must_use]
    pub const fn from_millis(value: u64) -> Self {
        Self(value)
    }

    /// Returns the monotonic millisecond counter.
    #[must_use]
    pub const fn as_millis(self) -> u64 {
        self.0
    }

    fn checked_add(self, millis: u64) -> Option<Self> {
        self.0.checked_add(millis).map(Self)
    }
}

/// Hard bounds and expiry policy for one in-memory mutation broker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentBrokerConfig {
    pub max_records: usize,
    pub max_terminal_records: usize,
    pub max_affected_nodes: usize,
    pub max_affected_resources: usize,
    pub max_preview_changes: usize,
    pub max_egress_data_classes: usize,
    pub max_identifier_bytes: usize,
    pub max_summary_bytes: usize,
    pub max_decision_reason_bytes: usize,
    pub proposal_ttl_millis: u64,
    pub approval_ttl_millis: u64,
    pub commit_timeout_millis: u64,
    pub terminal_retention_millis: u64,
}

impl Default for AgentBrokerConfig {
    fn default() -> Self {
        Self {
            max_records: 256,
            max_terminal_records: 64,
            max_affected_nodes: 1_024,
            max_affected_resources: 1_024,
            max_preview_changes: 1_024,
            max_egress_data_classes: 32,
            max_identifier_bytes: 256,
            max_summary_bytes: 4_096,
            max_decision_reason_bytes: 1_024,
            proposal_ttl_millis: 300_000,
            approval_ttl_millis: 300_000,
            commit_timeout_millis: 120_000,
            terminal_retention_millis: 3_600_000,
        }
    }
}

impl AgentBrokerConfig {
    fn validate(&self) -> Result<(), AgentBrokerError> {
        validate_bounded_nonzero(self.max_records, HARD_MAX_RECORDS, "max_records")?;
        validate_bounded_nonzero(
            self.max_terminal_records,
            self.max_records,
            "max_terminal_records",
        )?;
        validate_bounded_nonzero(
            self.max_affected_nodes,
            HARD_MAX_COLLECTION_ITEMS,
            "max_affected_nodes",
        )?;
        validate_bounded_nonzero(
            self.max_affected_resources,
            HARD_MAX_COLLECTION_ITEMS,
            "max_affected_resources",
        )?;
        validate_bounded_nonzero(
            self.max_preview_changes,
            HARD_MAX_COLLECTION_ITEMS,
            "max_preview_changes",
        )?;
        validate_bounded_nonzero(
            self.max_egress_data_classes,
            HARD_MAX_COLLECTION_ITEMS,
            "max_egress_data_classes",
        )?;
        validate_bounded_nonzero(
            self.max_identifier_bytes,
            HARD_MAX_TEXT_BYTES,
            "max_identifier_bytes",
        )?;
        validate_bounded_nonzero(
            self.max_summary_bytes,
            HARD_MAX_TEXT_BYTES,
            "max_summary_bytes",
        )?;
        validate_bounded_nonzero(
            self.max_decision_reason_bytes,
            HARD_MAX_TEXT_BYTES,
            "max_decision_reason_bytes",
        )?;
        validate_ttl(self.proposal_ttl_millis, "proposal_ttl_millis")?;
        validate_ttl(self.approval_ttl_millis, "approval_ttl_millis")?;
        validate_ttl(self.commit_timeout_millis, "commit_timeout_millis")?;
        validate_ttl(self.terminal_retention_millis, "terminal_retention_millis")?;
        Ok(())
    }
}

fn validate_bounded_nonzero(
    value: usize,
    maximum: usize,
    field: &'static str,
) -> Result<(), AgentBrokerError> {
    if value == 0 || value > maximum {
        return Err(AgentBrokerError::InvalidConfig { field });
    }
    Ok(())
}

fn validate_ttl(value: u64, field: &'static str) -> Result<(), AgentBrokerError> {
    if value == 0 || value > HARD_MAX_TTL_MILLIS {
        return Err(AgentBrokerError::InvalidConfig { field });
    }
    Ok(())
}

/// Trusted, current control-plane facts used for authorization.
///
/// These grants are not accepted from an agent harness. The embedding Desktop
/// or Server constructs them from its authenticated actor, delegated session,
/// and workspace policy on every proposal, preview publication, and commit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAuthorizationContext {
    pub human_actor_id: String,
    pub delegated_client_id: String,
    pub workspace_scope_id: String,
    pub origin: AgentOrigin,
    pub actor_capabilities: CapabilityGrant,
    pub delegated_session_capabilities: CapabilityGrant,
    pub workspace_policy_capabilities: CapabilityGrant,
    pub current_revision: String,
}

impl AgentAuthorizationContext {
    /// Computes `actor ∩ delegated session ∩ workspace policy`.
    #[must_use]
    pub fn effective_capabilities(&self) -> CapabilityGrant {
        self.actor_capabilities
            .intersection(&self.delegated_session_capabilities)
            .intersection(&self.workspace_policy_capabilities)
    }

    fn allows(&self, capability: AgentCapability) -> bool {
        self.actor_capabilities.allows(capability)
            && self.delegated_session_capabilities.allows(capability)
            && self.workspace_policy_capabilities.allows(capability)
    }
}

/// Stable names for every broker phase, including required terminal outcomes.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMutationStateKind {
    Proposal,
    AwaitingApproval,
    Approved,
    CommitStarted,
    Committed,
    Failed,
    FailedIndeterminate,
    Denied,
    Cancelled,
    Expired,
}

/// Observable state for one supervised mutation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", deny_unknown_fields)]
pub enum AgentMutationState {
    Proposal,
    AwaitingApproval {
        preview_digest: AgentPreviewDigest,
    },
    Approved {
        preview_digest: AgentPreviewDigest,
        approved_by: String,
    },
    CommitStarted {
        preview_digest: AgentPreviewDigest,
    },
    Committed {
        new_revision: String,
    },
    Failed {
        error_code: String,
    },
    FailedIndeterminate {
        error_code: String,
    },
    Denied {
        reason: String,
    },
    Cancelled,
    Expired,
}

impl AgentMutationState {
    /// Returns the stable phase name without exposing state details.
    #[must_use]
    pub const fn kind(&self) -> AgentMutationStateKind {
        match self {
            Self::Proposal => AgentMutationStateKind::Proposal,
            Self::AwaitingApproval { .. } => AgentMutationStateKind::AwaitingApproval,
            Self::Approved { .. } => AgentMutationStateKind::Approved,
            Self::CommitStarted { .. } => AgentMutationStateKind::CommitStarted,
            Self::Committed { .. } => AgentMutationStateKind::Committed,
            Self::Failed { .. } => AgentMutationStateKind::Failed,
            Self::FailedIndeterminate { .. } => AgentMutationStateKind::FailedIndeterminate,
            Self::Denied { .. } => AgentMutationStateKind::Denied,
            Self::Cancelled => AgentMutationStateKind::Cancelled,
            Self::Expired => AgentMutationStateKind::Expired,
        }
    }

    const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Committed { .. }
                | Self::Failed { .. }
                | Self::FailedIndeterminate { .. }
                | Self::Denied { .. }
                | Self::Cancelled
                | Self::Expired
        )
    }
}

/// Bounded, serializable metadata for one in-memory broker record.
///
/// It intentionally contains neither the executable plan nor a durable-audit
/// assertion.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentMutationRecord {
    pub request: AgentActionRequest,
    pub preview: Option<AgentActionPreview>,
    pub state: AgentMutationState,
    pub created_at: AgentBrokerTime,
    pub updated_at: AgentBrokerTime,
    pub deadline: Option<AgentBrokerTime>,
}

/// An approval capability held only by the supervising process.
///
/// Private fields prevent construction outside this crate. The broker also
/// validates the token, actor, origin, scope, and exact preview binding again.
pub struct AgentApprovedAction {
    request_id: String,
    preview_digest: AgentPreviewDigest,
    approval_token: u64,
}

impl AgentApprovedAction {
    /// Returns the request identifier bound to this approval.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the exact approved preview digest.
    #[must_use]
    pub const fn preview_digest(&self) -> &AgentPreviewDigest {
        &self.preview_digest
    }
}

impl fmt::Debug for AgentApprovedAction {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentApprovedAction")
            .field("request_id", &self.request_id)
            .field("preview_digest", &self.preview_digest)
            .finish_non_exhaustive()
    }
}

/// Result of applying a human decision to an awaiting preview.
#[derive(Debug)]
pub enum AgentDecisionOutcome {
    Approved(AgentApprovedAction),
    Denied(Box<AgentMutationRecord>),
}

/// Outcome reported by the trusted Core transaction executor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentCommitCompletion {
    Committed { new_revision: String },
    Failed { error_code: String },
    FailedIndeterminate { error_code: String },
}

/// Single-use work containing the opaque typed plan.
///
/// The plan can only leave this wrapper by consuming the wrapper and invoking
/// one `FnOnce`. If that executor panics or disappears, the broker remains in
/// `commit_started` until it honestly becomes `failed_indeterminate`.
pub struct AgentCommitWork<P> {
    request_id: String,
    preview_digest: AgentPreviewDigest,
    lease_token: u64,
    plan: P,
}

impl<P> fmt::Debug for AgentCommitWork<P> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentCommitWork")
            .field("request_id", &self.request_id)
            .field("preview_digest", &self.preview_digest)
            .finish_non_exhaustive()
    }
}

impl<P> AgentCommitWork<P> {
    /// Returns the request identifier for UI correlation.
    #[must_use]
    pub fn request_id(&self) -> &str {
        &self.request_id
    }

    /// Returns the exact preview binding for UI correlation.
    #[must_use]
    pub const fn preview_digest(&self) -> &AgentPreviewDigest {
        &self.preview_digest
    }

    /// Consumes the opaque plan exactly once in a trusted executor.
    pub fn execute(self, executor: impl FnOnce(P) -> AgentCommitCompletion) -> AgentCommitReport {
        let completion = executor(self.plan);
        AgentCommitReport {
            request_id: self.request_id,
            preview_digest: self.preview_digest,
            lease_token: self.lease_token,
            completion,
        }
    }
}

/// Unforgeable in-process report produced by consuming [`AgentCommitWork`].
pub struct AgentCommitReport {
    request_id: String,
    preview_digest: AgentPreviewDigest,
    lease_token: u64,
    completion: AgentCommitCompletion,
}

/// An opaque typed Core plan whose deterministic identity is bound into the agent preview.
///
/// Implementations expose only a closed digest projection. The plan itself never becomes part of
/// an agent-facing request or MCP result.
pub trait AgentMutationPlan {
    /// Returns the exact typed-intent and Core-plan identity bound to this opaque plan.
    fn mutation_binding(&self) -> AgentMutationBinding;
}

impl fmt::Debug for AgentCommitReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentCommitReport")
            .field("request_id", &self.request_id)
            .field("preview_digest", &self.preview_digest)
            .field("completion", &self.completion)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum AgentBrokerError {
    InvalidConfig {
        field: &'static str,
    },
    InvalidRequest {
        field: &'static str,
        reason: &'static str,
    },
    Unauthorized {
        capability: AgentCapability,
    },
    IdentityMismatch {
        field: &'static str,
    },
    RevisionMismatch {
        expected: String,
        actual: String,
    },
    DuplicateRequest,
    CapacityExceeded,
    NotFound,
    InvalidTransition {
        current: AgentMutationStateKind,
        attempted: &'static str,
    },
    PreviewDigestMismatch,
    PlanBindingMismatch,
    CommitAlreadyStarted,
    CommitLeaseMismatch,
    ClockRegressed,
    TimeOverflow,
    PreviewEncoding,
    InternalInvariant,
}

impl fmt::Display for AgentBrokerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig { field } => write!(formatter, "invalid broker config: {field}"),
            Self::InvalidRequest { field, reason } => {
                write!(formatter, "invalid agent request field {field}: {reason}")
            }
            Self::Unauthorized { capability } => {
                write!(formatter, "effective grant lacks {capability:?}")
            }
            Self::IdentityMismatch { field } => {
                write!(formatter, "authorization identity mismatch: {field}")
            }
            Self::RevisionMismatch { expected, actual } => write!(
                formatter,
                "base revision mismatch: expected {expected}, found {actual}"
            ),
            Self::DuplicateRequest => formatter.write_str("agent request ID already exists"),
            Self::CapacityExceeded => formatter.write_str("agent broker capacity exceeded"),
            Self::NotFound => formatter.write_str("agent mutation record not found"),
            Self::InvalidTransition { current, attempted } => {
                write!(formatter, "cannot {attempted} while state is {current:?}")
            }
            Self::PreviewDigestMismatch => {
                formatter.write_str("approval does not match the exact preview")
            }
            Self::PlanBindingMismatch => {
                formatter.write_str("typed intent does not match the opaque Core plan")
            }
            Self::CommitAlreadyStarted => {
                formatter.write_str("commit already started; cancellation cannot claim to undo it")
            }
            Self::CommitLeaseMismatch => formatter.write_str("commit report lease mismatch"),
            Self::ClockRegressed => formatter.write_str("broker time moved backwards"),
            Self::TimeOverflow => formatter.write_str("broker deadline overflow"),
            Self::PreviewEncoding => formatter.write_str("could not encode deterministic preview"),
            Self::InternalInvariant => formatter.write_str("agent broker invariant violation"),
        }
    }
}

impl std::error::Error for AgentBrokerError {}

struct BrokerEntry<P> {
    request: AgentActionRequest,
    plan: Option<P>,
    preview: Option<AgentActionPreview>,
    state: AgentMutationState,
    created_at: AgentBrokerTime,
    updated_at: AgentBrokerTime,
    deadline: Option<AgentBrokerTime>,
    terminal_at: Option<AgentBrokerTime>,
    approval_token: Option<u64>,
    active_lease_token: Option<u64>,
}

impl<P> BrokerEntry<P> {
    fn snapshot(&self) -> AgentMutationRecord {
        AgentMutationRecord {
            request: self.request.clone(),
            preview: self.preview.clone(),
            state: self.state.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            deadline: self.deadline,
        }
    }

    fn finish(&mut self, state: AgentMutationState, now: AgentBrokerTime) {
        self.plan = None;
        self.state = state;
        self.updated_at = now;
        self.deadline = None;
        self.terminal_at = Some(now);
        self.approval_token = None;
        self.active_lease_token = None;
    }
}

/// In-memory, harness-neutral supervisor for typed mutation plans.
///
/// The broker is intentionally not `Serialize`: restart recovery and audit
/// durability belong to an embedding control plane, which must not infer a
/// durable commit outcome from this process-local state.
pub struct AgentMutationBroker<P> {
    config: AgentBrokerConfig,
    records: BTreeMap<String, BrokerEntry<P>>,
    last_observed_at: Option<AgentBrokerTime>,
    next_token: u64,
}

impl<P: AgentMutationPlan> AgentMutationBroker<P> {
    /// Creates an empty broker after validating every capacity and TTL bound.
    ///
    /// # Errors
    ///
    /// Returns [`AgentBrokerError::InvalidConfig`] when any configured bound or
    /// TTL is zero or exceeds the broker's hard safety ceiling.
    pub fn new(config: AgentBrokerConfig) -> Result<Self, AgentBrokerError> {
        config.validate()?;
        Ok(Self {
            config,
            records: BTreeMap::new(),
            last_observed_at: None,
            next_token: 1,
        })
    }

    /// Returns the number of retained active and bounded terminal records.
    #[must_use]
    pub fn record_count(&self) -> usize {
        self.records.len()
    }

    /// Returns bounded retained state metadata without exposing executable
    /// plans. A durable embedding controller uses this to audit automatic TTL
    /// and indeterminate-timeout transitions performed during maintenance.
    #[must_use]
    pub fn retained_states(&self) -> Vec<(String, AgentMutationState)> {
        self.records
            .iter()
            .map(|(request_id, entry)| (request_id.clone(), entry.state.clone()))
            .collect()
    }

    /// Stores an opaque typed plan as an unpreviewed proposal.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid or duplicate metadata, stale or mismatched
    /// authorization, insufficient effective capabilities, time errors, or
    /// exhausted active-record capacity.
    pub fn propose(
        &mut self,
        mut request: AgentActionRequest,
        plan: P,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentMutationRecord, AgentBrokerError> {
        self.advance(now)?;
        self.validate_authorization(authorization)?;
        self.validate_and_normalize_request(&mut request)?;
        if request.binding != plan.mutation_binding() {
            return Err(AgentBrokerError::PlanBindingMismatch);
        }
        Self::validate_identity(&request, authorization)?;
        Self::validate_revision(&request, authorization)?;
        Self::authorize_proposal(&request, authorization)?;

        if self.records.contains_key(&request.request_id) {
            return Err(AgentBrokerError::DuplicateRequest);
        }
        self.make_capacity()?;
        let deadline = Self::deadline(now, self.config.proposal_ttl_millis)?;
        let request_id = request.request_id.clone();
        let entry = BrokerEntry {
            request,
            plan: Some(plan),
            preview: None,
            state: AgentMutationState::Proposal,
            created_at: now,
            updated_at: now,
            deadline: Some(deadline),
            terminal_at: None,
            approval_token: None,
            active_lease_token: None,
        };
        let snapshot = entry.snapshot();
        self.records.insert(request_id, entry);
        Ok(snapshot)
    }

    /// Publishes the deterministic effects for one proposal.
    ///
    /// Current authorization and revision are checked again so revocation or a
    /// stale base takes effect before this next agent tool step.
    ///
    /// # Errors
    ///
    /// Returns an error when the proposal is absent or no longer previewable,
    /// authorization or revision changed, effects exceed bounds, or encoding
    /// the exact deterministic preview fails.
    pub fn publish_preview(
        &mut self,
        request_id: &str,
        changes: Vec<AgentChangePreview>,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentActionPreview, AgentBrokerError> {
        self.advance(now)?;
        self.validate_lookup_id(request_id)?;
        self.validate_authorization(authorization)?;
        let request = {
            let entry = self
                .records
                .get(request_id)
                .ok_or(AgentBrokerError::NotFound)?;
            Self::require_state(
                &entry.state,
                AgentMutationStateKind::Proposal,
                "publish preview",
            )?;
            let plan = entry
                .plan
                .as_ref()
                .ok_or(AgentBrokerError::InternalInvariant)?;
            if entry.request.binding != plan.mutation_binding() {
                return Err(AgentBrokerError::PlanBindingMismatch);
            }
            entry.request.clone()
        };
        Self::validate_identity(&request, authorization)?;
        Self::validate_revision(&request, authorization)?;
        Self::authorize_proposal(&request, authorization)?;
        self.validate_changes(&request, &changes)?;

        let preview_digest = digest_preview(&request, &changes)?;
        let preview = AgentActionPreview {
            schema: PREVIEW_SCHEMA.to_owned(),
            request,
            changes,
            preview_digest: preview_digest.clone(),
        };
        let deadline = Self::deadline(now, self.config.approval_ttl_millis)?;
        let entry = self
            .records
            .get_mut(request_id)
            .ok_or(AgentBrokerError::NotFound)?;
        entry.state = AgentMutationState::AwaitingApproval { preview_digest };
        entry.preview = Some(preview.clone());
        entry.updated_at = now;
        entry.deadline = Some(deadline);
        Ok(preview)
    }

    /// Applies one human decision to the exact current preview.
    ///
    /// # Errors
    ///
    /// Returns an error for an absent/non-awaiting record, actor or digest
    /// mismatch, an invalid denial reason, or broker time/token failure.
    pub fn decide(
        &mut self,
        request_id: &str,
        decision: ApprovalDecision,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentDecisionOutcome, AgentBrokerError> {
        self.advance(now)?;
        self.validate_lookup_id(request_id)?;
        self.validate_authorization(authorization)?;
        let (request, expected_digest) = {
            let entry = self
                .records
                .get(request_id)
                .ok_or(AgentBrokerError::NotFound)?;
            Self::require_state(
                &entry.state,
                AgentMutationStateKind::AwaitingApproval,
                "decide",
            )?;
            let preview = entry
                .preview
                .as_ref()
                .ok_or(AgentBrokerError::InternalInvariant)?;
            (entry.request.clone(), preview.preview_digest.clone())
        };
        Self::validate_identity(&request, authorization)?;
        Self::validate_revision(&request, authorization)?;
        let expected_actor = request.human_actor_id.clone();

        match decision {
            ApprovalDecision::Approved {
                human_actor_id,
                preview_digest,
            } => {
                Self::authorize_commit(&request, authorization)?;
                validate_exact_decision(
                    &expected_actor,
                    &expected_digest,
                    &human_actor_id,
                    &preview_digest,
                )?;
                let approval_token = self.issue_token()?;
                let deadline = Self::deadline(now, self.config.approval_ttl_millis)?;
                let entry = self
                    .records
                    .get_mut(request_id)
                    .ok_or(AgentBrokerError::NotFound)?;
                entry.state = AgentMutationState::Approved {
                    preview_digest: preview_digest.clone(),
                    approved_by: human_actor_id,
                };
                entry.updated_at = now;
                entry.deadline = Some(deadline);
                entry.approval_token = Some(approval_token);
                Ok(AgentDecisionOutcome::Approved(AgentApprovedAction {
                    request_id: request_id.to_owned(),
                    preview_digest,
                    approval_token,
                }))
            }
            ApprovalDecision::Denied {
                human_actor_id,
                preview_digest,
                reason,
            } => {
                validate_exact_decision(
                    &expected_actor,
                    &expected_digest,
                    &human_actor_id,
                    &preview_digest,
                )?;
                Self::validate_text(
                    "denial_reason",
                    &reason,
                    self.config.max_decision_reason_bytes,
                )?;
                let entry = self
                    .records
                    .get_mut(request_id)
                    .ok_or(AgentBrokerError::NotFound)?;
                entry.finish(AgentMutationState::Denied { reason }, now);
                let snapshot = entry.snapshot();
                self.prune_terminal_records(now);
                Ok(AgentDecisionOutcome::Denied(Box::new(snapshot)))
            }
        }
    }

    /// Revalidates identity, scope, revision, and current capabilities, then
    /// moves the opaque plan into a single-use commit work object.
    ///
    /// # Errors
    ///
    /// Returns an error if the approval is absent, expired, already consumed,
    /// or not bound to this preview; if current identity/revision/capabilities
    /// no longer authorize it; or if broker time/token state is invalid.
    pub fn start_commit(
        &mut self,
        approval: &AgentApprovedAction,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentCommitWork<P>, AgentBrokerError> {
        self.advance(now)?;
        self.validate_authorization(authorization)?;
        let request = {
            let entry = self
                .records
                .get(&approval.request_id)
                .ok_or(AgentBrokerError::NotFound)?;
            Self::require_state(
                &entry.state,
                AgentMutationStateKind::Approved,
                "start commit",
            )?;
            if entry.approval_token != Some(approval.approval_token) {
                return Err(AgentBrokerError::PreviewDigestMismatch);
            }
            let preview = entry
                .preview
                .as_ref()
                .ok_or(AgentBrokerError::InternalInvariant)?;
            if preview.preview_digest != approval.preview_digest {
                return Err(AgentBrokerError::PreviewDigestMismatch);
            }
            let plan = entry
                .plan
                .as_ref()
                .ok_or(AgentBrokerError::InternalInvariant)?;
            if entry.request.binding != plan.mutation_binding() {
                return Err(AgentBrokerError::PlanBindingMismatch);
            }
            entry.request.clone()
        };
        Self::validate_identity(&request, authorization)?;
        Self::validate_revision(&request, authorization)?;
        Self::authorize_commit(&request, authorization)?;

        let lease_token = self.issue_token()?;
        let deadline = Self::deadline(now, self.config.commit_timeout_millis)?;
        let entry = self
            .records
            .get_mut(&approval.request_id)
            .ok_or(AgentBrokerError::NotFound)?;
        let plan = entry
            .plan
            .take()
            .ok_or(AgentBrokerError::InternalInvariant)?;
        entry.state = AgentMutationState::CommitStarted {
            preview_digest: approval.preview_digest.clone(),
        };
        entry.updated_at = now;
        entry.deadline = Some(deadline);
        entry.approval_token = None;
        entry.active_lease_token = Some(lease_token);
        Ok(AgentCommitWork {
            request_id: approval.request_id.clone(),
            preview_digest: approval.preview_digest.clone(),
            lease_token,
            plan,
        })
    }

    /// Records the result of one consumed commit work object.
    ///
    /// # Errors
    ///
    /// Returns an error for an absent or timed-out commit, a mismatched lease or
    /// preview binding, a regressed clock, or an internal invariant violation.
    pub fn finish_commit(
        &mut self,
        report: AgentCommitReport,
        now: AgentBrokerTime,
    ) -> Result<AgentMutationRecord, AgentBrokerError> {
        self.advance(now)?;
        let entry = self
            .records
            .get_mut(&report.request_id)
            .ok_or(AgentBrokerError::NotFound)?;
        Self::require_state(
            &entry.state,
            AgentMutationStateKind::CommitStarted,
            "finish commit",
        )?;
        if entry.active_lease_token != Some(report.lease_token) {
            return Err(AgentBrokerError::CommitLeaseMismatch);
        }
        let preview = entry
            .preview
            .as_ref()
            .ok_or(AgentBrokerError::InternalInvariant)?;
        if preview.preview_digest != report.preview_digest {
            return Err(AgentBrokerError::CommitLeaseMismatch);
        }

        let terminal_state = completion_state(report.completion, self.config.max_identifier_bytes);
        entry.finish(terminal_state, now);
        let snapshot = entry.snapshot();
        self.prune_terminal_records(now);
        Ok(snapshot)
    }

    /// Cancels only work that has not begun committing.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid lookup metadata, mismatched identity, a
    /// terminal action, or a commit that has already started.
    pub fn cancel(
        &mut self,
        request_id: &str,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentMutationRecord, AgentBrokerError> {
        self.advance(now)?;
        self.validate_lookup_id(request_id)?;
        self.validate_authorization(authorization)?;
        let request = self
            .records
            .get(request_id)
            .ok_or(AgentBrokerError::NotFound)?
            .request
            .clone();
        Self::validate_identity(&request, authorization)?;

        let entry = self
            .records
            .get_mut(request_id)
            .ok_or(AgentBrokerError::NotFound)?;
        match entry.state.kind() {
            AgentMutationStateKind::Proposal
            | AgentMutationStateKind::AwaitingApproval
            | AgentMutationStateKind::Approved => {
                entry.finish(AgentMutationState::Cancelled, now);
                let snapshot = entry.snapshot();
                self.prune_terminal_records(now);
                Ok(snapshot)
            }
            AgentMutationStateKind::CommitStarted => Err(AgentBrokerError::CommitAlreadyStarted),
            current => Err(AgentBrokerError::InvalidTransition {
                current,
                attempted: "cancel",
            }),
        }
    }

    /// Returns one record only to the exact originating actor/client/session/scope.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid lookup metadata, a missing/inaccessible
    /// record, invalid authorization metadata, or a regressed clock.
    pub fn snapshot(
        &mut self,
        request_id: &str,
        authorization: &AgentAuthorizationContext,
        now: AgentBrokerTime,
    ) -> Result<AgentMutationRecord, AgentBrokerError> {
        self.advance(now)?;
        self.validate_lookup_id(request_id)?;
        self.validate_authorization(authorization)?;
        let entry = self
            .records
            .get(request_id)
            .ok_or(AgentBrokerError::NotFound)?;
        if Self::validate_identity(&entry.request, authorization).is_err() {
            return Err(AgentBrokerError::NotFound);
        }
        Ok(entry.snapshot())
    }

    /// Applies expiry and bounded terminal-record cleanup.
    ///
    /// # Errors
    ///
    /// Returns [`AgentBrokerError::ClockRegressed`] if `now` precedes a time
    /// already observed by this broker.
    pub fn maintain(&mut self, now: AgentBrokerTime) -> Result<(), AgentBrokerError> {
        self.advance(now)
    }

    fn advance(&mut self, now: AgentBrokerTime) -> Result<(), AgentBrokerError> {
        if self.last_observed_at.is_some_and(|last| now < last) {
            return Err(AgentBrokerError::ClockRegressed);
        }
        self.last_observed_at = Some(now);

        for entry in self.records.values_mut() {
            if entry.deadline.is_none_or(|deadline| now < deadline) {
                continue;
            }
            match entry.state.kind() {
                AgentMutationStateKind::Proposal
                | AgentMutationStateKind::AwaitingApproval
                | AgentMutationStateKind::Approved => {
                    entry.finish(AgentMutationState::Expired, now);
                }
                AgentMutationStateKind::CommitStarted => {
                    entry.finish(
                        AgentMutationState::FailedIndeterminate {
                            error_code: COMMIT_TIMEOUT_ERROR.to_owned(),
                        },
                        now,
                    );
                }
                AgentMutationStateKind::Committed
                | AgentMutationStateKind::Failed
                | AgentMutationStateKind::FailedIndeterminate
                | AgentMutationStateKind::Denied
                | AgentMutationStateKind::Cancelled
                | AgentMutationStateKind::Expired => {}
            }
        }
        self.prune_terminal_records(now);
        Ok(())
    }

    fn prune_terminal_records(&mut self, now: AgentBrokerTime) {
        let retention = self.config.terminal_retention_millis;
        self.records.retain(|_, entry| {
            entry.terminal_at.is_none_or(|terminal_at| {
                now.as_millis().saturating_sub(terminal_at.as_millis()) < retention
            })
        });

        let terminal_count = self
            .records
            .values()
            .filter(|entry| entry.state.is_terminal())
            .count();
        let excess = terminal_count.saturating_sub(self.config.max_terminal_records);
        self.remove_oldest_terminals(excess);
    }

    fn make_capacity(&mut self) -> Result<(), AgentBrokerError> {
        let needed = self
            .records
            .len()
            .saturating_add(1)
            .saturating_sub(self.config.max_records);
        self.remove_oldest_terminals(needed);
        if self.records.len() >= self.config.max_records {
            return Err(AgentBrokerError::CapacityExceeded);
        }
        Ok(())
    }

    fn remove_oldest_terminals(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let mut terminal_keys: Vec<_> = self
            .records
            .iter()
            .filter_map(|(key, entry)| {
                entry
                    .terminal_at
                    .map(|terminal_at| (terminal_at, key.clone()))
            })
            .collect();
        terminal_keys.sort_unstable();
        for (_, key) in terminal_keys.into_iter().take(count) {
            self.records.remove(&key);
        }
    }

    fn deadline(
        now: AgentBrokerTime,
        ttl_millis: u64,
    ) -> Result<AgentBrokerTime, AgentBrokerError> {
        now.checked_add(ttl_millis)
            .ok_or(AgentBrokerError::TimeOverflow)
    }

    fn issue_token(&mut self) -> Result<u64, AgentBrokerError> {
        let issued = self.next_token;
        self.next_token = self
            .next_token
            .checked_add(1)
            .ok_or(AgentBrokerError::InternalInvariant)?;
        Ok(issued)
    }

    fn validate_and_normalize_request(
        &self,
        request: &mut AgentActionRequest,
    ) -> Result<(), AgentBrokerError> {
        Self::validate_text(
            "request_id",
            &request.request_id,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "human_actor_id",
            &request.human_actor_id,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "delegated_client_id",
            &request.delegated_client_id,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "workspace_scope_id",
            &request.workspace_scope_id,
            self.config.max_identifier_bytes,
        )?;
        self.validate_origin(&request.origin)?;
        Self::validate_text(
            "action_id",
            &request.action_id,
            self.config.max_identifier_bytes,
        )?;
        self.validate_binding(&request.binding)?;
        Self::validate_text(
            "base_revision",
            &request.base_revision,
            self.config.max_identifier_bytes,
        )?;

        if request.affected_node_ids.len() > self.config.max_affected_nodes {
            return Err(AgentBrokerError::InvalidRequest {
                field: "affected_node_ids",
                reason: "too many values",
            });
        }
        let node_set: BTreeSet<_> = request.affected_node_ids.iter().copied().collect();
        if node_set.len() != request.affected_node_ids.len() {
            return Err(AgentBrokerError::InvalidRequest {
                field: "affected_node_ids",
                reason: "duplicate value",
            });
        }
        request.affected_node_ids = node_set.into_iter().collect();

        if request.affected_resources.len() > self.config.max_affected_resources {
            return Err(AgentBrokerError::InvalidRequest {
                field: "affected_resources",
                reason: "too many values",
            });
        }
        for resource in &request.affected_resources {
            self.validate_resource(resource)?;
        }
        let resource_set: BTreeSet<_> = request.affected_resources.iter().cloned().collect();
        if resource_set.len() != request.affected_resources.len() {
            return Err(AgentBrokerError::InvalidRequest {
                field: "affected_resources",
                reason: "duplicate value",
            });
        }
        request.affected_resources = resource_set.into_iter().collect();
        self.validate_target(&request.target)?;
        if !target_is_declared(request, &request.target) {
            return Err(AgentBrokerError::InvalidRequest {
                field: "target",
                reason: "target is absent from affected objects",
            });
        }

        if let AgentExternalEgress::Required {
            destination_id,
            data_classes,
        } = &request.external_egress
        {
            Self::validate_text(
                "external_egress.destination_id",
                destination_id,
                self.config.max_identifier_bytes,
            )?;
            if data_classes.is_empty() || data_classes.len() > self.config.max_egress_data_classes {
                return Err(AgentBrokerError::InvalidRequest {
                    field: "external_egress.data_classes",
                    reason: "value count is outside configured bounds",
                });
            }
            for data_class in data_classes {
                Self::validate_text(
                    "external_egress.data_classes",
                    data_class,
                    self.config.max_identifier_bytes,
                )?;
            }
        }
        Ok(())
    }

    fn validate_binding(&self, binding: &AgentMutationBinding) -> Result<(), AgentBrokerError> {
        Self::validate_text(
            "binding.intent_schema",
            &binding.intent_schema,
            self.config.max_identifier_bytes,
        )?;
        validate_digest("binding.intent_digest", &binding.intent_digest)?;
        Self::validate_text(
            "binding.core_plan_schema",
            &binding.core_plan_schema,
            self.config.max_identifier_bytes,
        )?;
        validate_digest("binding.core_plan_digest", &binding.core_plan_digest)
    }

    fn validate_authorization(
        &self,
        authorization: &AgentAuthorizationContext,
    ) -> Result<(), AgentBrokerError> {
        Self::validate_text(
            "authorization.human_actor_id",
            &authorization.human_actor_id,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "authorization.delegated_client_id",
            &authorization.delegated_client_id,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "authorization.workspace_scope_id",
            &authorization.workspace_scope_id,
            self.config.max_identifier_bytes,
        )?;
        self.validate_origin(&authorization.origin)?;
        Self::validate_text(
            "authorization.current_revision",
            &authorization.current_revision,
            self.config.max_identifier_bytes,
        )
    }

    fn validate_origin(&self, origin: &AgentOrigin) -> Result<(), AgentBrokerError> {
        Self::validate_text(
            "origin.harness",
            &origin.harness,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "origin.adapter_version",
            &origin.adapter_version,
            self.config.max_identifier_bytes,
        )?;
        Self::validate_text(
            "origin.session_id",
            &origin.session_id,
            self.config.max_identifier_bytes,
        )
    }

    fn validate_resource(&self, resource: &AgentAffectedResource) -> Result<(), AgentBrokerError> {
        Self::validate_text(
            "resource.logical_id",
            &resource.logical_id,
            self.config.max_identifier_bytes,
        )
    }

    fn validate_target(&self, target: &AgentActionTarget) -> Result<(), AgentBrokerError> {
        if let AgentActionTarget::Resource { resource } = target {
            self.validate_resource(resource)?;
        }
        Ok(())
    }

    fn validate_changes(
        &self,
        request: &AgentActionRequest,
        changes: &[AgentChangePreview],
    ) -> Result<(), AgentBrokerError> {
        if changes.is_empty() || changes.len() > self.config.max_preview_changes {
            return Err(AgentBrokerError::InvalidRequest {
                field: "changes",
                reason: "value count is outside configured bounds",
            });
        }
        for change in changes {
            self.validate_target(&change.target)?;
            if !target_is_declared(request, &change.target) {
                return Err(AgentBrokerError::InvalidRequest {
                    field: "changes.target",
                    reason: "preview target is absent from affected objects",
                });
            }
            Self::validate_text(
                "changes.summary",
                &change.summary,
                self.config.max_summary_bytes,
            )?;
        }
        Ok(())
    }

    fn validate_lookup_id(&self, request_id: &str) -> Result<(), AgentBrokerError> {
        Self::validate_text("request_id", request_id, self.config.max_identifier_bytes)
    }

    fn validate_text(
        field: &'static str,
        value: &str,
        max_bytes: usize,
    ) -> Result<(), AgentBrokerError> {
        if value.is_empty() {
            return Err(AgentBrokerError::InvalidRequest {
                field,
                reason: "must not be empty",
            });
        }
        if value.len() > max_bytes {
            return Err(AgentBrokerError::InvalidRequest {
                field,
                reason: "exceeds configured byte bound",
            });
        }
        if value.chars().any(char::is_control) {
            return Err(AgentBrokerError::InvalidRequest {
                field,
                reason: "contains control characters",
            });
        }
        Ok(())
    }

    fn validate_identity(
        request: &AgentActionRequest,
        authorization: &AgentAuthorizationContext,
    ) -> Result<(), AgentBrokerError> {
        if request.human_actor_id != authorization.human_actor_id {
            return Err(AgentBrokerError::IdentityMismatch {
                field: "human_actor_id",
            });
        }
        if request.delegated_client_id != authorization.delegated_client_id {
            return Err(AgentBrokerError::IdentityMismatch {
                field: "delegated_client_id",
            });
        }
        if request.workspace_scope_id != authorization.workspace_scope_id {
            return Err(AgentBrokerError::IdentityMismatch {
                field: "workspace_scope_id",
            });
        }
        if request.origin != authorization.origin {
            return Err(AgentBrokerError::IdentityMismatch { field: "origin" });
        }
        Ok(())
    }

    fn validate_revision(
        request: &AgentActionRequest,
        authorization: &AgentAuthorizationContext,
    ) -> Result<(), AgentBrokerError> {
        if request.base_revision != authorization.current_revision {
            return Err(AgentBrokerError::RevisionMismatch {
                expected: request.base_revision.clone(),
                actual: authorization.current_revision.clone(),
            });
        }
        Ok(())
    }

    fn authorize_proposal(
        request: &AgentActionRequest,
        authorization: &AgentAuthorizationContext,
    ) -> Result<(), AgentBrokerError> {
        Self::require_capability(authorization, AgentCapability::ProposeMutation)?;
        Self::require_capability(authorization, request.required_capability)?;
        if request.external_egress.is_required() {
            Self::require_capability(authorization, AgentCapability::ExternalEgress)?;
        }
        Ok(())
    }

    fn authorize_commit(
        request: &AgentActionRequest,
        authorization: &AgentAuthorizationContext,
    ) -> Result<(), AgentBrokerError> {
        Self::require_capability(authorization, AgentCapability::CommitApprovedMutation)?;
        Self::require_capability(authorization, request.required_capability)?;
        if request.external_egress.is_required() {
            Self::require_capability(authorization, AgentCapability::ExternalEgress)?;
        }
        Ok(())
    }

    fn require_capability(
        authorization: &AgentAuthorizationContext,
        capability: AgentCapability,
    ) -> Result<(), AgentBrokerError> {
        if !authorization.allows(capability) {
            return Err(AgentBrokerError::Unauthorized { capability });
        }
        Ok(())
    }

    fn require_state(
        state: &AgentMutationState,
        required: AgentMutationStateKind,
        attempted: &'static str,
    ) -> Result<(), AgentBrokerError> {
        if state.kind() != required {
            return Err(AgentBrokerError::InvalidTransition {
                current: state.kind(),
                attempted,
            });
        }
        Ok(())
    }
}

fn target_is_declared(request: &AgentActionRequest, target: &AgentActionTarget) -> bool {
    match target {
        AgentActionTarget::WorkspaceScope => {
            matches!(request.target, AgentActionTarget::WorkspaceScope)
        }
        AgentActionTarget::Node { node_id } => request.affected_node_ids.contains(node_id),
        AgentActionTarget::Resource { resource } => request.affected_resources.contains(resource),
    }
}

fn validate_digest(field: &'static str, value: &str) -> Result<(), AgentBrokerError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AgentBrokerError::InvalidRequest {
            field,
            reason: "must be a canonical lowercase SHA-256 digest",
        });
    }
    Ok(())
}

fn validate_exact_decision(
    expected_actor: &str,
    expected_digest: &AgentPreviewDigest,
    actual_actor: &str,
    actual_digest: &AgentPreviewDigest,
) -> Result<(), AgentBrokerError> {
    if expected_actor != actual_actor {
        return Err(AgentBrokerError::IdentityMismatch {
            field: "human_actor_id",
        });
    }
    if expected_digest != actual_digest {
        return Err(AgentBrokerError::PreviewDigestMismatch);
    }
    Ok(())
}

fn digest_preview(
    request: &AgentActionRequest,
    changes: &[AgentChangePreview],
) -> Result<AgentPreviewDigest, AgentBrokerError> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PreviewMaterial<'a> {
        schema: &'static str,
        request: &'a AgentActionRequest,
        changes: &'a [AgentChangePreview],
    }

    let encoded = serde_json::to_vec(&PreviewMaterial {
        schema: PREVIEW_SCHEMA,
        request,
        changes,
    })
    .map_err(|_| AgentBrokerError::PreviewEncoding)?;
    let digest = Sha256::digest(encoded);
    Ok(AgentPreviewDigest::from_hex(format!("{digest:x}")))
}

fn completion_state(completion: AgentCommitCompletion, max_bytes: usize) -> AgentMutationState {
    match completion {
        AgentCommitCompletion::Committed { new_revision } => {
            if valid_completion_text(&new_revision, max_bytes) {
                AgentMutationState::Committed { new_revision }
            } else {
                invalid_completion_state()
            }
        }
        AgentCommitCompletion::Failed { error_code } => {
            if valid_completion_text(&error_code, max_bytes) {
                AgentMutationState::Failed { error_code }
            } else {
                invalid_completion_state()
            }
        }
        AgentCommitCompletion::FailedIndeterminate { error_code } => {
            if valid_completion_text(&error_code, max_bytes) {
                AgentMutationState::FailedIndeterminate { error_code }
            } else {
                invalid_completion_state()
            }
        }
    }
}

fn valid_completion_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

fn invalid_completion_state() -> AgentMutationState {
    AgentMutationState::FailedIndeterminate {
        error_code: INVALID_COMPLETION_ERROR.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeSet;
    use std::str::FromStr;

    use weftext_core::NodeId;

    use super::{
        AgentAuthorizationContext, AgentBrokerConfig, AgentBrokerError, AgentBrokerTime,
        AgentCommitCompletion, AgentDecisionOutcome, AgentMutationBroker, AgentMutationPlan,
        AgentMutationState, AgentMutationStateKind,
    };
    use crate::{
        AgentActionRequest, AgentActionRisk, AgentActionTarget, AgentCapability,
        AgentChangePreview, AgentConfirmationPolicy, AgentExternalEgress, AgentMutationBinding,
        AgentOrigin, ApprovalDecision, CapabilityGrant,
    };

    const NODE_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[derive(Debug, Eq, PartialEq)]
    struct TestPlan {
        operation: &'static str,
    }

    impl AgentMutationPlan for TestPlan {
        fn mutation_binding(&self) -> AgentMutationBinding {
            mutation_binding()
        }
    }

    fn mutation_binding() -> AgentMutationBinding {
        AgentMutationBinding::from_serializable(
            "weftext.test.intent.v1",
            &"bounded-test-intent",
            "weftext.test.core-plan.v1",
            &"bounded-test-plan",
        )
        .expect("test binding encodes")
    }

    fn time(value: u64) -> AgentBrokerTime {
        AgentBrokerTime::from_millis(value)
    }

    fn node_id() -> NodeId {
        NodeId::from_str(NODE_ID).expect("fixture UUID is canonical v4")
    }

    fn second_node_id() -> NodeId {
        NodeId::from_str("550e8400-e29b-41d4-a716-446655440001")
            .expect("second fixture UUID is canonical v4")
    }

    fn all_capabilities() -> CapabilityGrant {
        CapabilityGrant::new([
            AgentCapability::ReadWorkspace,
            AgentCapability::SearchWorkspace,
            AgentCapability::ProposeMutation,
            AgentCapability::CommitApprovedMutation,
            AgentCapability::ExternalEgress,
        ])
    }

    fn authorization() -> AgentAuthorizationContext {
        AgentAuthorizationContext {
            human_actor_id: "human-1".to_owned(),
            delegated_client_id: "client-1".to_owned(),
            workspace_scope_id: "workspace-1:subtree-a".to_owned(),
            origin: AgentOrigin {
                harness: "test-harness".to_owned(),
                adapter_version: "1.0.0".to_owned(),
                session_id: "session-1".to_owned(),
            },
            actor_capabilities: all_capabilities(),
            delegated_session_capabilities: all_capabilities(),
            workspace_policy_capabilities: all_capabilities(),
            current_revision: "revision-1".to_owned(),
        }
    }

    fn request(request_id: &str) -> AgentActionRequest {
        AgentActionRequest {
            request_id: request_id.to_owned(),
            human_actor_id: "human-1".to_owned(),
            delegated_client_id: "client-1".to_owned(),
            workspace_scope_id: "workspace-1:subtree-a".to_owned(),
            origin: AgentOrigin {
                harness: "test-harness".to_owned(),
                adapter_version: "1.0.0".to_owned(),
                session_id: "session-1".to_owned(),
            },
            required_capability: AgentCapability::ProposeMutation,
            action_id: "rename-node".to_owned(),
            binding: mutation_binding(),
            target: AgentActionTarget::Node { node_id: node_id() },
            affected_node_ids: vec![node_id()],
            affected_resources: Vec::new(),
            base_revision: "revision-1".to_owned(),
            external_egress: AgentExternalEgress::None,
            risk: AgentActionRisk::Ordinary,
            confirmation_policy: AgentConfirmationPolicy::ExplicitHumanApproval,
        }
    }

    fn change(summary: &str) -> AgentChangePreview {
        AgentChangePreview {
            target: AgentActionTarget::Node { node_id: node_id() },
            summary: summary.to_owned(),
        }
    }

    fn broker() -> AgentMutationBroker<TestPlan> {
        AgentMutationBroker::new(AgentBrokerConfig::default())
            .expect("default broker config is valid")
    }

    fn create_preview(
        broker: &mut AgentMutationBroker<TestPlan>,
        auth: &AgentAuthorizationContext,
        request_id: &str,
        proposed_at: u64,
    ) -> crate::AgentActionPreview {
        let proposal = broker
            .propose(
                request(request_id),
                TestPlan {
                    operation: "rename",
                },
                auth,
                time(proposed_at),
            )
            .expect("proposal succeeds");
        assert_eq!(proposal.state.kind(), AgentMutationStateKind::Proposal);
        let preview = broker
            .publish_preview(
                request_id,
                vec![change("Rename the selected node")],
                auth,
                time(proposed_at + 1),
            )
            .expect("preview succeeds");
        assert_eq!(preview.schema, "weftext.agent.mutation-preview.v1");
        preview
    }

    fn approve(
        broker: &mut AgentMutationBroker<TestPlan>,
        auth: &AgentAuthorizationContext,
        request_id: &str,
        proposed_at: u64,
    ) -> super::AgentApprovedAction {
        let preview = create_preview(broker, auth, request_id, proposed_at);
        match broker
            .decide(
                request_id,
                ApprovalDecision::Approved {
                    human_actor_id: auth.human_actor_id.clone(),
                    preview_digest: preview.preview_digest,
                },
                auth,
                time(proposed_at + 2),
            )
            .expect("approval succeeds")
        {
            AgentDecisionOutcome::Approved(approved) => approved,
            AgentDecisionOutcome::Denied(_) => panic!("expected approval"),
        }
    }

    #[test]
    fn capability_intersection_cannot_be_expanded_by_any_one_grant() {
        let mut broker = broker();
        let mut auth = authorization();
        auth.delegated_session_capabilities = CapabilityGrant::new([
            AgentCapability::CommitApprovedMutation,
            AgentCapability::ExternalEgress,
        ]);

        let error = broker
            .propose(
                request("missing-propose"),
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect_err("delegated grant must constrain the actor");
        assert_eq!(
            error,
            AgentBrokerError::Unauthorized {
                capability: AgentCapability::ProposeMutation
            }
        );
        assert_eq!(broker.record_count(), 0);
    }

    #[test]
    fn egress_is_visible_and_requires_capability_at_proposal_and_commit() {
        let mut broker = broker();
        let mut egress_request = request("egress");
        egress_request.external_egress = AgentExternalEgress::Required {
            destination_id: "provider.example".to_owned(),
            data_classes: BTreeSet::from(["selected_text".to_owned()]),
        };
        egress_request.risk = AgentActionRisk::ExternalEgress;
        let mut no_egress = authorization();
        no_egress.workspace_policy_capabilities = CapabilityGrant::new([
            AgentCapability::ProposeMutation,
            AgentCapability::CommitApprovedMutation,
        ]);

        let error = broker
            .propose(
                egress_request.clone(),
                TestPlan {
                    operation: "egress",
                },
                &no_egress,
                time(0),
            )
            .expect_err("egress must fail closed");
        assert_eq!(
            error,
            AgentBrokerError::Unauthorized {
                capability: AgentCapability::ExternalEgress
            }
        );

        let auth = authorization();
        broker
            .propose(
                egress_request,
                TestPlan {
                    operation: "egress",
                },
                &auth,
                time(1),
            )
            .expect("authorized egress proposal succeeds");
        let preview = broker
            .publish_preview("egress", vec![change("Send selected text")], &auth, time(2))
            .expect("preview succeeds");
        assert!(preview.request.external_egress.is_required());
        let approved = match broker
            .decide(
                "egress",
                ApprovalDecision::Approved {
                    human_actor_id: auth.human_actor_id.clone(),
                    preview_digest: preview.preview_digest,
                },
                &auth,
                time(3),
            )
            .expect("approval succeeds")
        {
            AgentDecisionOutcome::Approved(approved) => approved,
            AgentDecisionOutcome::Denied(_) => panic!("expected approval"),
        };
        let mut revoked = auth.clone();
        revoked.workspace_policy_capabilities = CapabilityGrant::new([
            AgentCapability::ProposeMutation,
            AgentCapability::CommitApprovedMutation,
        ]);
        let error = broker
            .start_commit(&approved, &revoked, time(4))
            .expect_err("egress revocation applies before commit");
        assert_eq!(
            error,
            AgentBrokerError::Unauthorized {
                capability: AgentCapability::ExternalEgress
            }
        );
    }

    #[test]
    fn proposal_and_commit_reject_actor_origin_scope_client_and_revision_mismatch() {
        let auth = authorization();
        let mismatch_cases: Vec<(&str, AgentAuthorizationContext, &'static str)> = vec![
            (
                "actor",
                AgentAuthorizationContext {
                    human_actor_id: "human-2".to_owned(),
                    ..auth.clone()
                },
                "human_actor_id",
            ),
            (
                "client",
                AgentAuthorizationContext {
                    delegated_client_id: "client-2".to_owned(),
                    ..auth.clone()
                },
                "delegated_client_id",
            ),
            (
                "origin",
                AgentAuthorizationContext {
                    origin: AgentOrigin {
                        session_id: "session-2".to_owned(),
                        ..auth.origin.clone()
                    },
                    ..auth.clone()
                },
                "origin",
            ),
            (
                "scope",
                AgentAuthorizationContext {
                    workspace_scope_id: "workspace-1:other".to_owned(),
                    ..auth.clone()
                },
                "workspace_scope_id",
            ),
        ];

        for (request_id, mismatched, field) in &mismatch_cases {
            let mut test_broker = broker();
            let error = test_broker
                .propose(
                    request(request_id),
                    TestPlan { operation: "noop" },
                    mismatched,
                    time(0),
                )
                .expect_err("identity mismatch must fail");
            assert_eq!(error, AgentBrokerError::IdentityMismatch { field });
        }
        let mut stale = auth.clone();
        stale.current_revision = "revision-2".to_owned();
        let mut test_broker = broker();
        let error = test_broker
            .propose(
                request("stale"),
                TestPlan { operation: "noop" },
                &stale,
                time(0),
            )
            .expect_err("stale proposal must fail");
        assert_eq!(
            error,
            AgentBrokerError::RevisionMismatch {
                expected: "revision-1".to_owned(),
                actual: "revision-2".to_owned()
            }
        );

        let mut commit_broker = broker();
        let approved = approve(&mut commit_broker, &auth, "commit-identity", 10);
        for (_, mismatched, field) in &mismatch_cases {
            let error = commit_broker
                .start_commit(&approved, mismatched, time(13))
                .expect_err("commit identity mismatch must fail");
            assert_eq!(error, AgentBrokerError::IdentityMismatch { field });
        }
        let error = commit_broker
            .start_commit(&approved, &stale, time(13))
            .expect_err("stale commit must fail");
        assert!(matches!(error, AgentBrokerError::RevisionMismatch { .. }));
        let record = commit_broker
            .snapshot("commit-identity", &auth, time(13))
            .expect("failed attempts leave approval usable");
        assert_eq!(record.state.kind(), AgentMutationStateKind::Approved);
    }

    #[test]
    fn approval_is_bound_to_the_exact_preview_and_human_actor() {
        let auth = authorization();
        let mut first = broker();
        let preview = create_preview(&mut first, &auth, "exact", 0);
        let mut second = broker();
        let other_preview = create_preview(&mut second, &auth, "other", 0);
        assert_ne!(preview.preview_digest, other_preview.preview_digest);

        let error = first
            .decide(
                "exact",
                ApprovalDecision::Approved {
                    human_actor_id: auth.human_actor_id.clone(),
                    preview_digest: other_preview.preview_digest,
                },
                &auth,
                time(2),
            )
            .expect_err("another preview cannot be approved");
        assert_eq!(error, AgentBrokerError::PreviewDigestMismatch);
        let error = first
            .decide(
                "exact",
                ApprovalDecision::Approved {
                    human_actor_id: "human-2".to_owned(),
                    preview_digest: preview.preview_digest.clone(),
                },
                &auth,
                time(2),
            )
            .expect_err("another actor cannot approve");
        assert_eq!(
            error,
            AgentBrokerError::IdentityMismatch {
                field: "human_actor_id"
            }
        );

        let mut deterministic = broker();
        let same_preview = create_preview(&mut deterministic, &auth, "exact", 50);
        assert_eq!(preview.preview_digest, same_preview.preview_digest);
    }

    #[test]
    fn opaque_plan_binding_and_current_approval_authority_fail_closed() {
        let auth = authorization();
        let mut mismatched_request = request("binding-mismatch");
        mismatched_request.binding = AgentMutationBinding::from_serializable(
            "weftext.test.intent.v1",
            &"different-intent",
            "weftext.test.core-plan.v1",
            &"different-plan",
        )
        .expect("alternate binding encodes");
        let mut binding_broker = broker();
        assert_eq!(
            binding_broker
                .propose(
                    mismatched_request,
                    TestPlan { operation: "noop" },
                    &auth,
                    time(0),
                )
                .expect_err("opaque plan identity must match request"),
            AgentBrokerError::PlanBindingMismatch
        );

        let mut approval_broker = broker();
        let preview = create_preview(&mut approval_broker, &auth, "approval-auth", 10);
        let mut stale = auth.clone();
        stale.current_revision = "revision-2".to_owned();
        assert!(matches!(
            approval_broker
                .decide(
                    "approval-auth",
                    ApprovalDecision::Approved {
                        human_actor_id: auth.human_actor_id.clone(),
                        preview_digest: preview.preview_digest.clone(),
                    },
                    &stale,
                    time(12),
                )
                .expect_err("approval revalidates current revision"),
            AgentBrokerError::RevisionMismatch { .. }
        ));
        let mut revoked = auth.clone();
        revoked.delegated_session_capabilities = CapabilityGrant::new([
            AgentCapability::ProposeMutation,
            AgentCapability::ExternalEgress,
        ]);
        assert_eq!(
            approval_broker
                .decide(
                    "approval-auth",
                    ApprovalDecision::Approved {
                        human_actor_id: auth.human_actor_id.clone(),
                        preview_digest: preview.preview_digest,
                    },
                    &revoked,
                    time(12),
                )
                .expect_err("approval revalidates delegated capability"),
            AgentBrokerError::Unauthorized {
                capability: AgentCapability::CommitApprovedMutation
            }
        );
    }

    #[test]
    fn denial_is_terminal_and_discards_future_commit_work() {
        let auth = authorization();
        let mut broker = broker();
        let preview = create_preview(&mut broker, &auth, "deny", 0);
        let outcome = broker
            .decide(
                "deny",
                ApprovalDecision::Denied {
                    human_actor_id: auth.human_actor_id.clone(),
                    preview_digest: preview.preview_digest,
                    reason: "The scope is too broad".to_owned(),
                },
                &auth,
                time(2),
            )
            .expect("denial succeeds");
        let AgentDecisionOutcome::Denied(record) = outcome else {
            panic!("expected denial");
        };
        assert_eq!(
            record.state,
            AgentMutationState::Denied {
                reason: "The scope is too broad".to_owned()
            }
        );
        let error = broker
            .publish_preview("deny", vec![change("retry")], &auth, time(3))
            .expect_err("denied work is terminal");
        assert!(matches!(error, AgentBrokerError::InvalidTransition { .. }));
    }

    #[test]
    fn expiry_distinguishes_precommit_expiry_from_unknown_commit_outcome() {
        let config = AgentBrokerConfig {
            proposal_ttl_millis: 10,
            approval_ttl_millis: 10,
            commit_timeout_millis: 10,
            terminal_retention_millis: 100,
            ..AgentBrokerConfig::default()
        };
        let auth = authorization();

        let mut proposal_broker =
            AgentMutationBroker::new(config.clone()).expect("config is valid");
        proposal_broker
            .propose(
                request("proposal-expiry"),
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect("proposal succeeds");
        proposal_broker.maintain(time(10)).expect("expiry succeeds");
        assert_eq!(
            proposal_broker
                .snapshot("proposal-expiry", &auth, time(10))
                .expect("terminal record retained")
                .state,
            AgentMutationState::Expired
        );

        let mut approval_broker =
            AgentMutationBroker::new(config.clone()).expect("config is valid");
        create_preview(&mut approval_broker, &auth, "approval-expiry", 20);
        approval_broker
            .maintain(time(31))
            .expect("awaiting approval expires");
        assert_eq!(
            approval_broker
                .snapshot("approval-expiry", &auth, time(31))
                .expect("terminal record retained")
                .state,
            AgentMutationState::Expired
        );

        let mut commit_broker = AgentMutationBroker::new(config).expect("config is valid");
        let approved = approve(&mut commit_broker, &auth, "commit-timeout", 40);
        let work = commit_broker
            .start_commit(&approved, &auth, time(43))
            .expect("commit starts");
        assert_eq!(work.request_id(), "commit-timeout");
        commit_broker
            .maintain(time(53))
            .expect("commit timeout is processed");
        assert_eq!(
            commit_broker
                .snapshot("commit-timeout", &auth, time(53))
                .expect("terminal record retained")
                .state,
            AgentMutationState::FailedIndeterminate {
                error_code: "commit_outcome_timeout".to_owned()
            }
        );
        drop(work);

        let expiry_config = AgentBrokerConfig {
            proposal_ttl_millis: 10,
            approval_ttl_millis: 10,
            terminal_retention_millis: 100,
            ..AgentBrokerConfig::default()
        };
        let mut approved_broker = AgentMutationBroker::new(expiry_config).expect("config is valid");
        let approved = approve(&mut approved_broker, &auth, "approved-expiry", 60);
        approved_broker
            .maintain(time(72))
            .expect("approved action expires");
        assert_eq!(
            approved_broker
                .snapshot("approved-expiry", &auth, time(72))
                .expect("terminal record retained")
                .state,
            AgentMutationState::Expired
        );
        drop(approved);
    }

    #[test]
    fn cancellation_stops_future_work_but_never_claims_to_undo_started_commit() {
        let auth = authorization();
        let mut pending = broker();
        pending
            .propose(
                request("cancel-pending"),
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect("proposal succeeds");
        let cancelled = pending
            .cancel("cancel-pending", &auth, time(1))
            .expect("pending action cancels");
        assert_eq!(cancelled.state, AgentMutationState::Cancelled);

        let mut approved_broker = broker();
        let approved = approve(&mut approved_broker, &auth, "cancel-approved", 2);
        let cancelled = approved_broker
            .cancel("cancel-approved", &auth, time(5))
            .expect("approved action can be stopped before commit starts");
        assert_eq!(cancelled.state, AgentMutationState::Cancelled);
        let error = approved_broker
            .start_commit(&approved, &auth, time(5))
            .expect_err("cancelled approval cannot start");
        assert!(matches!(
            error,
            AgentBrokerError::InvalidTransition {
                current: AgentMutationStateKind::Cancelled,
                ..
            }
        ));

        let mut started = broker();
        let approved = approve(&mut started, &auth, "cancel-started", 10);
        let work = started
            .start_commit(&approved, &auth, time(13))
            .expect("commit starts");
        let error = started
            .cancel("cancel-started", &auth, time(14))
            .expect_err("started commit cannot be represented as cancelled");
        assert_eq!(error, AgentBrokerError::CommitAlreadyStarted);
        assert_eq!(
            started
                .snapshot("cancel-started", &auth, time(14))
                .expect("record remains visible")
                .state
                .kind(),
            AgentMutationStateKind::CommitStarted
        );
        drop(work);
    }

    #[test]
    fn approved_plan_starts_and_executes_only_once() {
        let auth = authorization();
        let mut broker = broker();
        let approved = approve(&mut broker, &auth, "single-use", 0);
        let work = broker
            .start_commit(&approved, &auth, time(3))
            .expect("first start succeeds");
        let replay_error = broker
            .start_commit(&approved, &auth, time(3))
            .expect_err("approval replay is rejected");
        assert_eq!(
            replay_error,
            AgentBrokerError::InvalidTransition {
                current: AgentMutationStateKind::CommitStarted,
                attempted: "start commit"
            }
        );

        let executions = Cell::new(0_u8);
        let report = work.execute(|plan| {
            executions.set(executions.get() + 1);
            assert_eq!(plan.operation, "rename");
            AgentCommitCompletion::Committed {
                new_revision: "revision-2".to_owned(),
            }
        });
        assert_eq!(executions.get(), 1);
        let committed = broker
            .finish_commit(report, time(4))
            .expect("commit result is recorded");
        assert_eq!(
            committed.state,
            AgentMutationState::Committed {
                new_revision: "revision-2".to_owned()
            }
        );
        let final_replay = broker
            .start_commit(&approved, &auth, time(5))
            .expect_err("terminal action cannot replay");
        assert!(matches!(
            final_replay,
            AgentBrokerError::InvalidTransition {
                current: AgentMutationStateKind::Committed,
                ..
            }
        ));
    }

    #[test]
    fn commit_capability_revocation_blocks_start_without_consuming_approval() {
        let auth = authorization();
        let mut broker = broker();
        let approved = approve(&mut broker, &auth, "revoked", 0);
        let mut revoked = auth.clone();
        revoked.actor_capabilities = CapabilityGrant::new([
            AgentCapability::ProposeMutation,
            AgentCapability::ExternalEgress,
        ]);
        let error = broker
            .start_commit(&approved, &revoked, time(3))
            .expect_err("current commit capability is mandatory");
        assert_eq!(
            error,
            AgentBrokerError::Unauthorized {
                capability: AgentCapability::CommitApprovedMutation
            }
        );
        broker
            .start_commit(&approved, &auth, time(3))
            .expect("approval remains usable with restored current authority");
    }

    #[test]
    fn executor_can_report_definite_and_indeterminate_failures() {
        let auth = authorization();
        let mut definite = broker();
        let approved = approve(&mut definite, &auth, "failed", 0);
        let report = definite
            .start_commit(&approved, &auth, time(3))
            .expect("commit starts")
            .execute(|_| AgentCommitCompletion::Failed {
                error_code: "core_rejected".to_owned(),
            });
        assert_eq!(
            definite
                .finish_commit(report, time(4))
                .expect("failure records")
                .state,
            AgentMutationState::Failed {
                error_code: "core_rejected".to_owned()
            }
        );

        let mut uncertain = broker();
        let approved = approve(&mut uncertain, &auth, "uncertain", 10);
        let report = uncertain
            .start_commit(&approved, &auth, time(13))
            .expect("commit starts")
            .execute(|_| AgentCommitCompletion::FailedIndeterminate {
                error_code: "transport_lost_after_dispatch".to_owned(),
            });
        assert_eq!(
            uncertain
                .finish_commit(report, time(14))
                .expect("indeterminate result records")
                .state,
            AgentMutationState::FailedIndeterminate {
                error_code: "transport_lost_after_dispatch".to_owned()
            }
        );
    }

    #[test]
    fn capacity_and_terminal_cleanup_are_bounded() {
        let config = AgentBrokerConfig {
            max_records: 2,
            max_terminal_records: 1,
            terminal_retention_millis: 5,
            ..AgentBrokerConfig::default()
        };
        let auth = authorization();
        let mut broker = AgentMutationBroker::new(config).expect("config is valid");
        for request_id in ["a", "b"] {
            broker
                .propose(
                    request(request_id),
                    TestPlan { operation: "noop" },
                    &auth,
                    time(0),
                )
                .expect("bounded proposal fits");
        }
        let error = broker
            .propose(request("c"), TestPlan { operation: "noop" }, &auth, time(0))
            .expect_err("active records cannot be evicted");
        assert_eq!(error, AgentBrokerError::CapacityExceeded);

        broker.cancel("a", &auth, time(1)).expect("a cancels");
        broker
            .propose(request("c"), TestPlan { operation: "noop" }, &auth, time(2))
            .expect("oldest terminal record makes room");
        assert_eq!(broker.record_count(), 2);
        assert_eq!(
            broker
                .snapshot("a", &auth, time(2))
                .expect_err("evicted terminal record is absent"),
            AgentBrokerError::NotFound
        );

        broker.cancel("b", &auth, time(3)).expect("b cancels");
        broker.cancel("c", &auth, time(4)).expect("c cancels");
        broker.maintain(time(4)).expect("count cleanup succeeds");
        assert_eq!(broker.record_count(), 1);
        broker
            .maintain(time(9))
            .expect("retention cleanup succeeds");
        assert_eq!(broker.record_count(), 0);
    }

    #[test]
    fn config_and_external_text_inputs_are_bounded() {
        let invalid_config = AgentBrokerConfig {
            max_records: 0,
            ..AgentBrokerConfig::default()
        };
        assert_eq!(
            AgentMutationBroker::<TestPlan>::new(invalid_config)
                .err()
                .expect("invalid config is rejected"),
            AgentBrokerError::InvalidConfig {
                field: "max_records"
            }
        );

        let config = AgentBrokerConfig {
            max_identifier_bytes: 64,
            max_preview_changes: 1,
            ..AgentBrokerConfig::default()
        };
        let mut broker = AgentMutationBroker::new(config).expect("config is valid");
        let auth = authorization();
        let error = broker
            .propose(
                request(&"x".repeat(65)),
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect_err("long request ID is rejected");
        assert!(matches!(
            error,
            AgentBrokerError::InvalidRequest {
                field: "request_id",
                ..
            }
        ));
    }

    #[test]
    fn affected_objects_egress_classes_and_preview_effects_are_bounded() {
        let auth = authorization();
        let collection_config = AgentBrokerConfig {
            max_affected_nodes: 1,
            max_preview_changes: 1,
            max_egress_data_classes: 1,
            ..AgentBrokerConfig::default()
        };
        let mut node_broker =
            AgentMutationBroker::new(collection_config.clone()).expect("config is valid");
        let mut too_many_nodes = request("too-many-nodes");
        too_many_nodes.affected_node_ids.push(second_node_id());
        let error = node_broker
            .propose(
                too_many_nodes,
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect_err("affected nodes are bounded");
        assert!(matches!(
            error,
            AgentBrokerError::InvalidRequest {
                field: "affected_node_ids",
                ..
            }
        ));

        let mut egress_broker =
            AgentMutationBroker::new(collection_config.clone()).expect("config is valid");
        let mut too_many_classes = request("too-many-classes");
        too_many_classes.external_egress = AgentExternalEgress::Required {
            destination_id: "provider".to_owned(),
            data_classes: BTreeSet::from(["selected_text".to_owned(), "node_metadata".to_owned()]),
        };
        let error = egress_broker
            .propose(
                too_many_classes,
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect_err("egress data classes are bounded");
        assert!(matches!(
            error,
            AgentBrokerError::InvalidRequest {
                field: "external_egress.data_classes",
                ..
            }
        ));

        let mut preview_broker =
            AgentMutationBroker::new(collection_config).expect("config is valid");
        preview_broker
            .propose(
                request("too-many-changes"),
                TestPlan { operation: "noop" },
                &auth,
                time(0),
            )
            .expect("proposal fits");
        let error = preview_broker
            .publish_preview(
                "too-many-changes",
                vec![change("first"), change("second")],
                &auth,
                time(1),
            )
            .expect_err("preview effects are bounded");
        assert!(matches!(
            error,
            AgentBrokerError::InvalidRequest {
                field: "changes",
                ..
            }
        ));
    }

    #[test]
    fn time_regression_and_deadline_overflow_fail_closed_without_panics() {
        let auth = authorization();
        let mut clock_broker = broker();
        clock_broker
            .propose(
                request("clock"),
                TestPlan { operation: "noop" },
                &auth,
                time(10),
            )
            .expect("proposal succeeds");
        assert_eq!(
            clock_broker
                .maintain(time(9))
                .expect_err("time cannot regress"),
            AgentBrokerError::ClockRegressed
        );

        let mut overflow = broker();
        let error = overflow
            .propose(
                request("overflow"),
                TestPlan { operation: "noop" },
                &auth,
                time(u64::MAX),
            )
            .expect_err("deadline overflow fails closed");
        assert_eq!(error, AgentBrokerError::TimeOverflow);
        assert_eq!(overflow.record_count(), 0);
    }
}
