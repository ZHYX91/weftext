use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{AgentCapability, AgentMutationStateKind, CancellationMode};

const RECORD_SCHEMA: &str = "weftext.agent.audit-record.v1";
const SEAL_SCHEMA: &str = "weftext.agent.audit-seal.v1";
const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const HARD_MAX_RECORDS: usize = 1_000_000;
const HARD_MAX_RECORD_BYTES: usize = 64 * 1024;
const HARD_MAX_IDENTIFIER_BYTES: usize = 1024;

/// Fixed identity of one supervised control-plane audit stream.
///
/// These are attribution identifiers, not bearer credentials. The schema has no
/// field for prompts, transcripts, document bodies, filesystem paths, or secrets.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAuditIdentity {
    pub human_actor_id: String,
    pub delegated_client_id: String,
    pub harness: String,
    pub adapter_version: String,
    pub session_id: String,
    pub workspace_scope_id: String,
}

/// A human decision retained without the free-form denial reason.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuditDecision {
    Approved,
    Denied,
}

/// Durable, redacted commit outcome.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "outcome", deny_unknown_fields)]
pub enum AgentAuditCommitOutcome {
    Committed { new_revision: String },
    Failed { error_code: String },
    FailedIndeterminate { error_code: String },
}

/// Closed, body-free lifecycle evidence for a supervised session.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "event", deny_unknown_fields)]
pub enum AgentAuditEvent {
    SessionOpened {
        runtime_wire_version: String,
        resume_supported: bool,
        cancellation: CancellationMode,
    },
    SessionReopened,
    Proposal {
        request_id: String,
        action_id: String,
        capability: AgentCapability,
        base_revision: String,
        intent_digest: String,
        core_plan_digest: String,
        target_digest: String,
    },
    PreviewPublished {
        request_id: String,
        preview_digest: String,
    },
    Decision {
        request_id: String,
        preview_digest: String,
        decision: AgentAuditDecision,
    },
    CommitStarted {
        request_id: String,
        preview_digest: String,
    },
    CommitOutcome {
        request_id: String,
        outcome: AgentAuditCommitOutcome,
    },
    Cancelled {
        request_id: String,
        previous_state: AgentMutationStateKind,
    },
    Expired {
        request_id: String,
        previous_state: AgentMutationStateKind,
    },
    CapabilitiesUpdated {
        capability_digest: String,
    },
    AdapterCrashed {
        error_code: String,
    },
    RuntimeTerminatedForCancellation,
}

impl AgentAuditEvent {
    fn request_id(&self) -> Option<&str> {
        match self {
            Self::Proposal { request_id, .. }
            | Self::PreviewPublished { request_id, .. }
            | Self::Decision { request_id, .. }
            | Self::CommitStarted { request_id, .. }
            | Self::CommitOutcome { request_id, .. }
            | Self::Cancelled { request_id, .. }
            | Self::Expired { request_id, .. } => Some(request_id),
            Self::SessionOpened { .. }
            | Self::SessionReopened
            | Self::CapabilitiesUpdated { .. }
            | Self::AdapterCrashed { .. }
            | Self::RuntimeTerminatedForCancellation => None,
        }
    }
}

/// One immutable record in the audit digest chain.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAuditRecord {
    pub schema: String,
    pub sequence: u64,
    pub timestamp_millis: u64,
    pub identity: AgentAuditIdentity,
    pub event: AgentAuditEvent,
    pub previous_digest: String,
    pub digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsignedRecord<'a> {
    schema: &'a str,
    sequence: u64,
    timestamp_millis: u64,
    identity: &'a AgentAuditIdentity,
    event: &'a AgentAuditEvent,
    previous_digest: &'a str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AuditSeal {
    schema: String,
    sequence: u64,
    record_digest: String,
    previous_seal_digest: String,
    digest: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsignedSeal<'a> {
    schema: &'a str,
    sequence: u64,
    record_digest: &'a str,
    previous_seal_digest: &'a str,
}

/// Durable audit capacity and encoding bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentAuditConfig {
    pub max_records: usize,
    pub max_record_bytes: usize,
    pub max_identifier_bytes: usize,
}

impl Default for AgentAuditConfig {
    fn default() -> Self {
        Self {
            max_records: 65_536,
            max_record_bytes: 16 * 1024,
            max_identifier_bytes: 256,
        }
    }
}

impl AgentAuditConfig {
    fn validate(&self) -> Result<(), AgentAuditError> {
        if self.max_records == 0 || self.max_records > HARD_MAX_RECORDS {
            return Err(AgentAuditError::InvalidConfig("max_records"));
        }
        if self.max_record_bytes == 0 || self.max_record_bytes > HARD_MAX_RECORD_BYTES {
            return Err(AgentAuditError::InvalidConfig("max_record_bytes"));
        }
        if self.max_identifier_bytes == 0 || self.max_identifier_bytes > HARD_MAX_IDENTIFIER_BYTES {
            return Err(AgentAuditError::InvalidConfig("max_identifier_bytes"));
        }
        Ok(())
    }
}

/// Honest restart interpretation of the last durable event for one request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuditRecoveryState {
    /// A proposal or preview existed, but its opaque executable plan did not survive restart.
    RequiresReproposal,
    /// Human approval existed, but the approved executable plan did not survive restart.
    ApprovedPlanUnavailable,
    /// A commit began and no durable outcome was recorded.
    CommitOutcomeIndeterminate,
    Committed,
    Failed,
    FailedIndeterminate,
    Denied,
    Cancelled,
    Expired,
}

/// Recovered request state derived exclusively from verified immutable records.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAuditRecovery {
    pub request_id: String,
    pub state: AgentAuditRecoveryState,
    pub last_sequence: u64,
}

/// Fail-closed durable audit errors.
#[derive(Debug)]
pub enum AgentAuditError {
    InvalidConfig(&'static str),
    InvalidIdentity(&'static str),
    InvalidEvent(&'static str),
    Io(std::io::Error),
    Encoding(serde_json::Error),
    UnexpectedEntry(String),
    BrokenSequence,
    BrokenDigestChain,
    BrokenSealChain,
    PartialAppend,
    IdentityMismatch,
    TimestampRegressed,
    InvalidTransition,
    CapacityExceeded,
    RecordTooLarge,
    ConcurrentWriter,
}

impl fmt::Display for AgentAuditError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(field) => write!(formatter, "invalid audit config: {field}"),
            Self::InvalidIdentity(field) => write!(formatter, "invalid audit identity: {field}"),
            Self::InvalidEvent(field) => write!(formatter, "invalid redacted audit event: {field}"),
            Self::Io(error) => write!(formatter, "audit storage I/O failed: {error}"),
            Self::Encoding(error) => write!(formatter, "audit encoding failed: {error}"),
            Self::UnexpectedEntry(name) => write!(formatter, "unexpected audit entry: {name}"),
            Self::BrokenSequence => formatter.write_str("audit sequence is missing or duplicated"),
            Self::BrokenDigestChain => formatter.write_str("audit record digest chain is invalid"),
            Self::BrokenSealChain => formatter.write_str("audit seal digest chain is invalid"),
            Self::PartialAppend => formatter.write_str("audit contains an interrupted append"),
            Self::IdentityMismatch => {
                formatter.write_str("audit belongs to another supervised identity")
            }
            Self::TimestampRegressed => formatter.write_str("audit timestamp moved backwards"),
            Self::InvalidTransition => {
                formatter.write_str("audit lifecycle transition is invalid or replayed")
            }
            Self::CapacityExceeded => formatter.write_str("audit record capacity exceeded"),
            Self::RecordTooLarge => formatter.write_str("audit record exceeds its byte bound"),
            Self::ConcurrentWriter => {
                formatter.write_str("another audit writer won the append race")
            }
        }
    }
}

impl std::error::Error for AgentAuditError {}

impl From<std::io::Error> for AgentAuditError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for AgentAuditError {
    fn from(error: serde_json::Error) -> Self {
        Self::Encoding(error)
    }
}

/// Append-only, digest-chained control-plane audit authority.
///
/// Each event and matching seal is an immutable numbered file. Creation uses a
/// same-directory temporary file, file flush, atomic rename to a previously
/// absent target, and immediate reopen verification. A record without its seal
/// (or vice versa) is an interrupted append and makes restart fail closed.
pub struct AgentAuditLog {
    root: PathBuf,
    identity: AgentAuditIdentity,
    config: AgentAuditConfig,
    records: Vec<AgentAuditRecord>,
    last_seal_digest: String,
    semantic: AuditSemanticState,
}

#[derive(Clone, Default)]
struct AuditSemanticState {
    opened: bool,
    requests: BTreeMap<String, AuditRequestState>,
}

#[derive(Clone)]
enum AuditRequestState {
    Proposed,
    Previewed(String),
    Approved(String),
    CommitStarted,
    Terminal,
}

impl fmt::Debug for AgentAuditLog {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AgentAuditLog")
            .field("root", &self.root)
            .field("identity", &self.identity)
            .field("record_count", &self.records.len())
            .finish_non_exhaustive()
    }
}

impl AgentAuditLog {
    /// Creates or reopens one audit stream and verifies every byte before use.
    ///
    /// # Errors
    ///
    /// Fails for invalid bounds or identity, storage errors, unexpected files,
    /// an interrupted append, or any broken sequence, identity, or digest chain.
    pub fn open(
        root: impl AsRef<Path>,
        identity: AgentAuditIdentity,
        config: AgentAuditConfig,
    ) -> Result<Self, AgentAuditError> {
        config.validate()?;
        validate_identity(&identity, &config)?;
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(root.join("records"))?;
        fs::create_dir_all(root.join("seals"))?;
        validate_root_entries(&root)?;
        let record_paths = numbered_paths(&root.join("records"), "json")?;
        let seal_paths = numbered_paths(&root.join("seals"), "seal")?;
        if record_paths.len() != seal_paths.len() {
            return Err(AgentAuditError::PartialAppend);
        }
        if record_paths.len() > config.max_records {
            return Err(AgentAuditError::CapacityExceeded);
        }

        let mut records = Vec::with_capacity(record_paths.len());
        let mut previous_digest = ZERO_DIGEST.to_owned();
        let mut previous_seal_digest = ZERO_DIGEST.to_owned();
        let mut last_timestamp = None;
        let mut semantic = AuditSemanticState::default();
        for (index, ((sequence, record_path), (seal_sequence, seal_path))) in
            record_paths.iter().zip(&seal_paths).enumerate()
        {
            let expected_sequence =
                u64::try_from(index + 1).map_err(|_| AgentAuditError::CapacityExceeded)?;
            if *sequence != expected_sequence || *seal_sequence != expected_sequence {
                return Err(AgentAuditError::BrokenSequence);
            }
            let record_bytes = read_bounded(record_path, config.max_record_bytes)?;
            let record: AgentAuditRecord = serde_json::from_slice(&record_bytes)?;
            verify_record(
                &record,
                expected_sequence,
                &previous_digest,
                &identity,
                &config,
            )?;
            if last_timestamp.is_some_and(|timestamp| record.timestamp_millis < timestamp) {
                return Err(AgentAuditError::TimestampRegressed);
            }
            let seal_bytes = read_bounded(seal_path, config.max_record_bytes)?;
            let seal: AuditSeal = serde_json::from_slice(&seal_bytes)?;
            verify_seal(
                &seal,
                expected_sequence,
                &record.digest,
                &previous_seal_digest,
            )?;
            apply_semantic_event(&mut semantic, &record.event, record.sequence)?;
            previous_digest.clone_from(&record.digest);
            previous_seal_digest.clone_from(&seal.digest);
            last_timestamp = Some(record.timestamp_millis);
            records.push(record);
        }

        Ok(Self {
            root,
            identity,
            config,
            records,
            last_seal_digest: previous_seal_digest,
            semantic,
        })
    }

    /// Appends, flushes, seals, and reopens one lifecycle event.
    ///
    /// # Errors
    ///
    /// Fails closed for invalid or oversized metadata, regressed time, exhausted
    /// capacity, storage failure, or a concurrent writer.
    pub fn append(
        &mut self,
        timestamp_millis: u64,
        event: AgentAuditEvent,
    ) -> Result<&AgentAuditRecord, AgentAuditError> {
        if self.records.len() >= self.config.max_records {
            return Err(AgentAuditError::CapacityExceeded);
        }
        if self
            .records
            .last()
            .is_some_and(|record| timestamp_millis < record.timestamp_millis)
        {
            return Err(AgentAuditError::TimestampRegressed);
        }
        validate_event(&event, &self.config)?;
        let sequence =
            u64::try_from(self.records.len() + 1).map_err(|_| AgentAuditError::CapacityExceeded)?;
        let mut next_semantic = self.semantic.clone();
        apply_semantic_event(&mut next_semantic, &event, sequence)?;
        let previous_digest = self
            .records
            .last()
            .map_or(ZERO_DIGEST, |record| record.digest.as_str());
        let digest = record_digest(
            sequence,
            timestamp_millis,
            &self.identity,
            &event,
            previous_digest,
        )?;
        let record = AgentAuditRecord {
            schema: RECORD_SCHEMA.to_owned(),
            sequence,
            timestamp_millis,
            identity: self.identity.clone(),
            event,
            previous_digest: previous_digest.to_owned(),
            digest,
        };
        let record_bytes = serde_json::to_vec(&record)?;
        ensure_size(&record_bytes, self.config.max_record_bytes)?;
        let record_path = self
            .root
            .join("records")
            .join(numbered_name(sequence, "json"));
        create_immutable(&record_path, &record_bytes)?;
        let reopened = read_bounded(&record_path, self.config.max_record_bytes)?;
        if reopened != record_bytes {
            return Err(AgentAuditError::BrokenDigestChain);
        }

        let seal_digest = seal_digest(sequence, &record.digest, &self.last_seal_digest)?;
        let seal = AuditSeal {
            schema: SEAL_SCHEMA.to_owned(),
            sequence,
            record_digest: record.digest.clone(),
            previous_seal_digest: self.last_seal_digest.clone(),
            digest: seal_digest,
        };
        let seal_bytes = serde_json::to_vec(&seal)?;
        ensure_size(&seal_bytes, self.config.max_record_bytes)?;
        let seal_path = self
            .root
            .join("seals")
            .join(numbered_name(sequence, "seal"));
        create_immutable(&seal_path, &seal_bytes)?;
        let reopened_seal = read_bounded(&seal_path, self.config.max_record_bytes)?;
        if reopened_seal != seal_bytes {
            return Err(AgentAuditError::BrokenSealChain);
        }
        self.last_seal_digest = seal.digest;
        self.records.push(record);
        self.semantic = next_semantic;
        self.records.last().ok_or(AgentAuditError::BrokenSequence)
    }

    /// Returns verified immutable records in sequence order.
    #[must_use]
    pub fn records(&self) -> &[AgentAuditRecord] {
        &self.records
    }

    /// Returns the stream identity verified during open.
    #[must_use]
    pub const fn identity(&self) -> &AgentAuditIdentity {
        &self.identity
    }

    /// Returns the last verified wall-clock timestamp, if any.
    #[must_use]
    pub fn last_timestamp_millis(&self) -> Option<u64> {
        self.records.last().map(|record| record.timestamp_millis)
    }

    /// Reconstructs terminal and nonterminal request outcomes without inventing
    /// an executable plan or a successful commit after restart.
    #[must_use]
    pub fn recovery_states(&self) -> Vec<AgentAuditRecovery> {
        let mut states = BTreeMap::<String, AgentAuditRecovery>::new();
        for record in &self.records {
            let Some(request_id) = record.event.request_id() else {
                continue;
            };
            let state = match &record.event {
                AgentAuditEvent::Proposal { .. } | AgentAuditEvent::PreviewPublished { .. } => {
                    AgentAuditRecoveryState::RequiresReproposal
                }
                AgentAuditEvent::Decision {
                    decision: AgentAuditDecision::Approved,
                    ..
                } => AgentAuditRecoveryState::ApprovedPlanUnavailable,
                AgentAuditEvent::Decision {
                    decision: AgentAuditDecision::Denied,
                    ..
                } => AgentAuditRecoveryState::Denied,
                AgentAuditEvent::CommitStarted { .. } => {
                    AgentAuditRecoveryState::CommitOutcomeIndeterminate
                }
                AgentAuditEvent::CommitOutcome { outcome, .. } => match outcome {
                    AgentAuditCommitOutcome::Committed { .. } => AgentAuditRecoveryState::Committed,
                    AgentAuditCommitOutcome::Failed { .. } => AgentAuditRecoveryState::Failed,
                    AgentAuditCommitOutcome::FailedIndeterminate { .. } => {
                        AgentAuditRecoveryState::FailedIndeterminate
                    }
                },
                AgentAuditEvent::Cancelled { .. } => AgentAuditRecoveryState::Cancelled,
                AgentAuditEvent::Expired { .. } => AgentAuditRecoveryState::Expired,
                AgentAuditEvent::SessionOpened { .. }
                | AgentAuditEvent::SessionReopened
                | AgentAuditEvent::CapabilitiesUpdated { .. }
                | AgentAuditEvent::AdapterCrashed { .. }
                | AgentAuditEvent::RuntimeTerminatedForCancellation => continue,
            };
            states.insert(
                request_id.to_owned(),
                AgentAuditRecovery {
                    request_id: request_id.to_owned(),
                    state,
                    last_sequence: record.sequence,
                },
            );
        }
        states.into_values().collect()
    }
}

fn apply_semantic_event(
    state: &mut AuditSemanticState,
    event: &AgentAuditEvent,
    sequence: u64,
) -> Result<(), AgentAuditError> {
    if let AgentAuditEvent::SessionOpened { .. } = event {
        if sequence != 1 || state.opened || !state.requests.is_empty() {
            return Err(AgentAuditError::InvalidTransition);
        }
        state.opened = true;
        return Ok(());
    }
    if !state.opened {
        return Err(AgentAuditError::InvalidTransition);
    }
    match event {
        AgentAuditEvent::SessionOpened { .. } => unreachable!("handled above"),
        AgentAuditEvent::SessionReopened
        | AgentAuditEvent::CapabilitiesUpdated { .. }
        | AgentAuditEvent::AdapterCrashed { .. }
        | AgentAuditEvent::RuntimeTerminatedForCancellation => Ok(()),
        AgentAuditEvent::Proposal { request_id, .. } => {
            if state.requests.contains_key(request_id) {
                return Err(AgentAuditError::InvalidTransition);
            }
            state
                .requests
                .insert(request_id.clone(), AuditRequestState::Proposed);
            Ok(())
        }
        AgentAuditEvent::PreviewPublished {
            request_id,
            preview_digest,
        } => {
            let request = state
                .requests
                .get_mut(request_id)
                .ok_or(AgentAuditError::InvalidTransition)?;
            if !matches!(request, AuditRequestState::Proposed) {
                return Err(AgentAuditError::InvalidTransition);
            }
            *request = AuditRequestState::Previewed(preview_digest.clone());
            Ok(())
        }
        AgentAuditEvent::Decision {
            request_id,
            preview_digest,
            decision,
        } => {
            let request = state
                .requests
                .get_mut(request_id)
                .ok_or(AgentAuditError::InvalidTransition)?;
            let AuditRequestState::Previewed(expected_digest) = request else {
                return Err(AgentAuditError::InvalidTransition);
            };
            if expected_digest != preview_digest {
                return Err(AgentAuditError::InvalidTransition);
            }
            *request = match decision {
                AgentAuditDecision::Approved => AuditRequestState::Approved(preview_digest.clone()),
                AgentAuditDecision::Denied => AuditRequestState::Terminal,
            };
            Ok(())
        }
        AgentAuditEvent::CommitStarted {
            request_id,
            preview_digest,
        } => {
            let request = state
                .requests
                .get_mut(request_id)
                .ok_or(AgentAuditError::InvalidTransition)?;
            let AuditRequestState::Approved(expected_digest) = request else {
                return Err(AgentAuditError::InvalidTransition);
            };
            if expected_digest != preview_digest {
                return Err(AgentAuditError::InvalidTransition);
            }
            *request = AuditRequestState::CommitStarted;
            Ok(())
        }
        AgentAuditEvent::CommitOutcome { request_id, .. } => {
            let request = state
                .requests
                .get_mut(request_id)
                .ok_or(AgentAuditError::InvalidTransition)?;
            if !matches!(request, AuditRequestState::CommitStarted) {
                return Err(AgentAuditError::InvalidTransition);
            }
            *request = AuditRequestState::Terminal;
            Ok(())
        }
        AgentAuditEvent::Cancelled {
            request_id,
            previous_state,
        } => apply_cancelled_event(state, request_id, *previous_state),
        AgentAuditEvent::Expired {
            request_id,
            previous_state,
        } => apply_terminal_from_precommit(state, request_id, *previous_state),
    }
}

fn apply_cancelled_event(
    state: &mut AuditSemanticState,
    request_id: &str,
    previous_state: AgentMutationStateKind,
) -> Result<(), AgentAuditError> {
    apply_terminal_from_precommit(state, request_id, previous_state)
}

fn apply_terminal_from_precommit(
    state: &mut AuditSemanticState,
    request_id: &str,
    previous_state: AgentMutationStateKind,
) -> Result<(), AgentAuditError> {
    let request = state
        .requests
        .get_mut(request_id)
        .ok_or(AgentAuditError::InvalidTransition)?;
    let observed = match request {
        AuditRequestState::Proposed => AgentMutationStateKind::Proposal,
        AuditRequestState::Previewed(_) => AgentMutationStateKind::AwaitingApproval,
        AuditRequestState::Approved(_) => AgentMutationStateKind::Approved,
        AuditRequestState::CommitStarted | AuditRequestState::Terminal => {
            return Err(AgentAuditError::InvalidTransition);
        }
    };
    if observed != previous_state {
        return Err(AgentAuditError::InvalidTransition);
    }
    *request = AuditRequestState::Terminal;
    Ok(())
}

fn validate_identity(
    identity: &AgentAuditIdentity,
    config: &AgentAuditConfig,
) -> Result<(), AgentAuditError> {
    for (field, value) in [
        ("human_actor_id", identity.human_actor_id.as_str()),
        ("delegated_client_id", identity.delegated_client_id.as_str()),
        ("harness", identity.harness.as_str()),
        ("adapter_version", identity.adapter_version.as_str()),
        ("session_id", identity.session_id.as_str()),
        ("workspace_scope_id", identity.workspace_scope_id.as_str()),
    ] {
        validate_identifier(value, config.max_identifier_bytes)
            .map_err(|()| AgentAuditError::InvalidIdentity(field))?;
    }
    Ok(())
}

fn validate_event(
    event: &AgentAuditEvent,
    config: &AgentAuditConfig,
) -> Result<(), AgentAuditError> {
    let mut values = Vec::new();
    match event {
        AgentAuditEvent::SessionOpened {
            runtime_wire_version,
            ..
        } => {
            values.push(("runtime_wire_version", runtime_wire_version.as_str()));
        }
        AgentAuditEvent::SessionReopened | AgentAuditEvent::RuntimeTerminatedForCancellation => {}
        AgentAuditEvent::Proposal {
            request_id,
            action_id,
            base_revision,
            intent_digest,
            core_plan_digest,
            target_digest,
            ..
        } => {
            values.extend([
                ("request_id", request_id.as_str()),
                ("action_id", action_id.as_str()),
                ("base_revision", base_revision.as_str()),
            ]);
            for (field, digest) in [
                ("intent_digest", intent_digest),
                ("core_plan_digest", core_plan_digest),
                ("target_digest", target_digest),
            ] {
                validate_digest(digest).map_err(|()| AgentAuditError::InvalidEvent(field))?;
            }
        }
        AgentAuditEvent::PreviewPublished {
            request_id,
            preview_digest,
        }
        | AgentAuditEvent::Decision {
            request_id,
            preview_digest,
            ..
        }
        | AgentAuditEvent::CommitStarted {
            request_id,
            preview_digest,
        } => {
            values.push(("request_id", request_id.as_str()));
            validate_digest(preview_digest)
                .map_err(|()| AgentAuditError::InvalidEvent("preview_digest"))?;
        }
        AgentAuditEvent::CommitOutcome {
            request_id,
            outcome,
        } => {
            values.push(("request_id", request_id.as_str()));
            match outcome {
                AgentAuditCommitOutcome::Committed { new_revision } => {
                    values.push(("new_revision", new_revision.as_str()));
                }
                AgentAuditCommitOutcome::Failed { error_code }
                | AgentAuditCommitOutcome::FailedIndeterminate { error_code } => {
                    validate_code(error_code)
                        .map_err(|()| AgentAuditError::InvalidEvent("error_code"))?;
                }
            }
        }
        AgentAuditEvent::Cancelled { request_id, .. }
        | AgentAuditEvent::Expired { request_id, .. } => {
            values.push(("request_id", request_id.as_str()));
        }
        AgentAuditEvent::CapabilitiesUpdated { capability_digest } => {
            validate_digest(capability_digest)
                .map_err(|()| AgentAuditError::InvalidEvent("capability_digest"))?;
        }
        AgentAuditEvent::AdapterCrashed { error_code } => {
            validate_code(error_code).map_err(|()| AgentAuditError::InvalidEvent("error_code"))?;
        }
    }
    for (field, value) in values {
        validate_identifier(value, config.max_identifier_bytes)
            .map_err(|()| AgentAuditError::InvalidEvent(field))?;
    }
    Ok(())
}

fn validate_identifier(value: &str, maximum: usize) -> Result<(), ()> {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        Err(())
    } else {
        Ok(())
    }
}

fn validate_digest(value: &str) -> Result<(), ()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(())
    }
}

fn validate_code(value: &str) -> Result<(), ()> {
    if value.is_empty()
        || value.len() > 128
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'_' | b'-' | b'.' | b':')
        })
    {
        Err(())
    } else {
        Ok(())
    }
}

fn validate_root_entries(root: &Path) -> Result<(), AgentAuditError> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !matches!(name.as_str(), "records" | "seals") || !entry.file_type()?.is_dir() {
            return Err(AgentAuditError::UnexpectedEntry(name));
        }
    }
    Ok(())
}

fn numbered_paths(
    directory: &Path,
    extension: &str,
) -> Result<Vec<(u64, PathBuf)>, AgentAuditError> {
    let mut paths = BTreeMap::new();
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            return Err(AgentAuditError::UnexpectedEntry(
                entry.file_name().to_string_lossy().into_owned(),
            ));
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let suffix = format!(".{extension}");
        let Some(stem) = name.strip_suffix(&suffix) else {
            return Err(
                if Path::new(&name)
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("tmp"))
                {
                    AgentAuditError::PartialAppend
                } else {
                    AgentAuditError::UnexpectedEntry(name)
                },
            );
        };
        if stem.len() != 20 || !stem.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(AgentAuditError::UnexpectedEntry(name));
        }
        let sequence = stem
            .parse::<u64>()
            .map_err(|_| AgentAuditError::UnexpectedEntry(name.clone()))?;
        if paths.insert(sequence, entry.path()).is_some() {
            return Err(AgentAuditError::BrokenSequence);
        }
    }
    Ok(paths.into_iter().collect())
}

fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, AgentAuditError> {
    let length = fs::metadata(path)?.len();
    if length > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err(AgentAuditError::RecordTooLarge);
    }
    let file = OpenOptions::new().read(true).open(path)?;
    let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(maximum));
    file.take(u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1))
        .read_to_end(&mut bytes)?;
    ensure_size(&bytes, maximum)?;
    Ok(bytes)
}

fn create_immutable(path: &Path, bytes: &[u8]) -> Result<(), AgentAuditError> {
    let parent = path.parent().ok_or_else(|| {
        AgentAuditError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "audit target has no parent",
        ))
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AgentAuditError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "audit target name is not UTF-8",
            ))
        })?;
    let temporary = parent.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let write_result = (|| {
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()
    })();
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temporary);
        return Err(AgentAuditError::Io(error));
    }
    drop(file);
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            if path.exists() {
                Err(AgentAuditError::ConcurrentWriter)
            } else {
                Err(AgentAuditError::Io(error))
            }
        }
    }
}

fn ensure_size(bytes: &[u8], maximum: usize) -> Result<(), AgentAuditError> {
    if bytes.len() > maximum {
        Err(AgentAuditError::RecordTooLarge)
    } else {
        Ok(())
    }
}

fn numbered_name(sequence: u64, extension: &str) -> String {
    format!("{sequence:020}.{extension}")
}

fn record_digest(
    sequence: u64,
    timestamp_millis: u64,
    identity: &AgentAuditIdentity,
    event: &AgentAuditEvent,
    previous_digest: &str,
) -> Result<String, AgentAuditError> {
    let bytes = serde_json::to_vec(&UnsignedRecord {
        schema: RECORD_SCHEMA,
        sequence,
        timestamp_millis,
        identity,
        event,
        previous_digest,
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn seal_digest(
    sequence: u64,
    record_digest: &str,
    previous_seal_digest: &str,
) -> Result<String, AgentAuditError> {
    let bytes = serde_json::to_vec(&UnsignedSeal {
        schema: SEAL_SCHEMA,
        sequence,
        record_digest,
        previous_seal_digest,
    })?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn verify_record(
    record: &AgentAuditRecord,
    sequence: u64,
    previous_digest: &str,
    identity: &AgentAuditIdentity,
    config: &AgentAuditConfig,
) -> Result<(), AgentAuditError> {
    if record.schema != RECORD_SCHEMA || record.sequence != sequence {
        return Err(AgentAuditError::BrokenSequence);
    }
    if &record.identity != identity {
        return Err(AgentAuditError::IdentityMismatch);
    }
    validate_event(&record.event, config)?;
    if record.previous_digest != previous_digest {
        return Err(AgentAuditError::BrokenDigestChain);
    }
    let expected = record_digest(
        record.sequence,
        record.timestamp_millis,
        &record.identity,
        &record.event,
        &record.previous_digest,
    )?;
    if record.digest != expected {
        return Err(AgentAuditError::BrokenDigestChain);
    }
    Ok(())
}

fn verify_seal(
    seal: &AuditSeal,
    sequence: u64,
    record_digest: &str,
    previous_seal_digest: &str,
) -> Result<(), AgentAuditError> {
    if seal.schema != SEAL_SCHEMA
        || seal.sequence != sequence
        || seal.record_digest != record_digest
        || seal.previous_seal_digest != previous_seal_digest
    {
        return Err(AgentAuditError::BrokenSealChain);
    }
    let expected = seal_digest(sequence, record_digest, previous_seal_digest)?;
    if seal.digest != expected {
        return Err(AgentAuditError::BrokenSealChain);
    }
    Ok(())
}

/// Stable digest for a closed capability intersection without exposing any
/// credential or policy source.
#[must_use]
pub fn capability_digest(capabilities: impl IntoIterator<Item = AgentCapability>) -> String {
    let stable = capabilities.into_iter().collect::<BTreeSet<_>>();
    let bytes = serde_json::to_vec(&stable).unwrap_or_default();
    format!("{:x}", Sha256::digest(bytes))
}

/// Stable digest of a logical action target. Callers must pass a closed typed
/// projection, never source bytes or a filesystem path.
///
/// # Errors
///
/// Returns an encoding error if the typed target cannot be serialized.
pub fn audit_target_digest<T: Serialize>(target: &T) -> Result<String, AgentAuditError> {
    let bytes = serde_json::to_vec(target)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn identity() -> AgentAuditIdentity {
        AgentAuditIdentity {
            human_actor_id: "human-1".to_owned(),
            delegated_client_id: "desktop-dsh".to_owned(),
            harness: "dsh".to_owned(),
            adapter_version: "1.0.0".to_owned(),
            session_id: "session-1".to_owned(),
            workspace_scope_id: "scope-1".to_owned(),
        }
    }

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn session_opened() -> AgentAuditEvent {
        AgentAuditEvent::SessionOpened {
            runtime_wire_version: "0.0.1".to_owned(),
            resume_supported: false,
            cancellation: CancellationMode::RuntimeTermination,
        }
    }

    fn append_approved(audit: &mut AgentAuditLog, request_id: &str, first: u64) -> u64 {
        audit
            .append(
                first,
                AgentAuditEvent::Proposal {
                    request_id: request_id.to_owned(),
                    action_id: "document_edit".to_owned(),
                    capability: AgentCapability::ProposeMutation,
                    base_revision: digest('1'),
                    intent_digest: digest('2'),
                    core_plan_digest: digest('3'),
                    target_digest: digest('4'),
                },
            )
            .unwrap();
        audit
            .append(
                first + 1,
                AgentAuditEvent::PreviewPublished {
                    request_id: request_id.to_owned(),
                    preview_digest: digest('5'),
                },
            )
            .unwrap();
        audit
            .append(
                first + 2,
                AgentAuditEvent::Decision {
                    request_id: request_id.to_owned(),
                    preview_digest: digest('5'),
                    decision: AgentAuditDecision::Approved,
                },
            )
            .unwrap();
        first + 3
    }

    #[test]
    fn append_flush_reopen_and_recover_without_bodies() {
        let temporary = tempdir().unwrap();
        let mut audit =
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()).unwrap();
        audit.append(1, session_opened()).unwrap();
        audit
            .append(
                2,
                AgentAuditEvent::Proposal {
                    request_id: "request-1".to_owned(),
                    action_id: "document_edit".to_owned(),
                    capability: AgentCapability::ProposeMutation,
                    base_revision: digest('1'),
                    intent_digest: digest('2'),
                    core_plan_digest: digest('3'),
                    target_digest: digest('4'),
                },
            )
            .unwrap();
        audit
            .append(
                3,
                AgentAuditEvent::PreviewPublished {
                    request_id: "request-1".to_owned(),
                    preview_digest: digest('5'),
                },
            )
            .unwrap();
        assert!(matches!(
            audit.append(
                4,
                AgentAuditEvent::AdapterCrashed {
                    error_code: "secret token must not become audit text".to_owned(),
                }
            ),
            Err(AgentAuditError::InvalidEvent("error_code"))
        ));
        drop(audit);

        let reopened =
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()).unwrap();
        assert_eq!(reopened.records().len(), 3);
        assert_eq!(
            reopened.recovery_states()[0].state,
            AgentAuditRecoveryState::RequiresReproposal
        );
        let encoded = serde_json::to_string(reopened.records()).unwrap();
        for forbidden in ["document body", "prompt", "transcript", "C:/workspace"] {
            assert!(!encoded.contains(forbidden));
        }
    }

    #[test]
    fn tamper_truncation_and_identity_changes_fail_closed() {
        let temporary = tempdir().unwrap();
        let mut audit =
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()).unwrap();
        audit.append(1, session_opened()).unwrap();
        drop(audit);
        let record = temporary.path().join("records/00000000000000000001.json");
        let original = fs::read(&record).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&original).unwrap();
        value["timestampMillis"] = serde_json::json!(9);
        fs::write(&record, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(matches!(
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()),
            Err(AgentAuditError::BrokenDigestChain)
        ));
        fs::write(&record, original).unwrap();
        fs::remove_file(&record).unwrap();
        assert!(matches!(
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()),
            Err(AgentAuditError::PartialAppend)
        ));

        let other = tempdir().unwrap();
        let mut audit =
            AgentAuditLog::open(other.path(), identity(), AgentAuditConfig::default()).unwrap();
        audit.append(1, session_opened()).unwrap();
        drop(audit);
        let mut foreign = identity();
        foreign.session_id = "foreign".to_owned();
        assert!(matches!(
            AgentAuditLog::open(other.path(), foreign, AgentAuditConfig::default()),
            Err(AgentAuditError::IdentityMismatch)
        ));
    }

    #[test]
    fn restart_marks_started_commit_indeterminate_and_retains_terminals() {
        let temporary = tempdir().unwrap();
        let mut audit =
            AgentAuditLog::open(temporary.path(), identity(), AgentAuditConfig::default()).unwrap();
        audit.append(1, session_opened()).unwrap();
        let unknown_commit = append_approved(&mut audit, "unknown", 2);
        audit
            .append(
                unknown_commit,
                AgentAuditEvent::CommitStarted {
                    request_id: "unknown".to_owned(),
                    preview_digest: digest('5'),
                },
            )
            .unwrap();
        let done_commit = append_approved(&mut audit, "done", unknown_commit + 1);
        audit
            .append(
                done_commit,
                AgentAuditEvent::CommitStarted {
                    request_id: "done".to_owned(),
                    preview_digest: digest('5'),
                },
            )
            .unwrap();
        audit
            .append(
                done_commit + 1,
                AgentAuditEvent::CommitOutcome {
                    request_id: "done".to_owned(),
                    outcome: AgentAuditCommitOutcome::Committed {
                        new_revision: digest('b'),
                    },
                },
            )
            .unwrap();
        assert!(matches!(
            audit.append(
                done_commit + 2,
                AgentAuditEvent::Proposal {
                    request_id: "done".to_owned(),
                    action_id: "document_edit".to_owned(),
                    capability: AgentCapability::ProposeMutation,
                    base_revision: digest('1'),
                    intent_digest: digest('2'),
                    core_plan_digest: digest('3'),
                    target_digest: digest('4'),
                }
            ),
            Err(AgentAuditError::InvalidTransition)
        ));
        let states = audit.recovery_states();
        assert_eq!(states[0].state, AgentAuditRecoveryState::Committed);
        assert_eq!(
            states[1].state,
            AgentAuditRecoveryState::CommitOutcomeIndeterminate
        );
    }
}
