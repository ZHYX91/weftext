//! Server-authoritative, bounded document collaboration.
//!
//! This is deliberately a linearized transform protocol, not a CRDT. Canonical
//! document bytes remain in the hosted workspace. The structures in this module
//! are transient coordination state and contain only a bounded edit history.

use std::collections::{BTreeMap, VecDeque};
use std::fmt::Write as _;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use weftext_core::{DocumentEdit, NodeId};

pub(crate) const WIRE_VERSION: &str = "weftext.collaboration.v1";
pub(crate) const HISTORY_LIMIT: usize = 128;
pub(crate) const MAX_OPERATIONS: usize = 64;
pub(crate) const MAX_REPLACEMENT_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const PRESENCE_TTL_SECONDS: i64 = 45;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TextOperation {
    pub start: u64,
    pub end: u64,
    pub replacement: String,
}

impl TextOperation {
    fn is_insertion(&self) -> bool {
        self.start == self.end
    }

    fn delta(&self) -> Result<i64, CollaborationError> {
        let removed = i64::try_from(self.end.saturating_sub(self.start))
            .map_err(|_| CollaborationError::InvalidOperations)?;
        let inserted = i64::try_from(self.replacement.len())
            .map_err(|_| CollaborationError::InvalidOperations)?;
        Ok(inserted - removed)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OperationRequest {
    pub wire_version: String,
    pub client_id: String,
    pub operation_id: String,
    pub epoch: u64,
    pub base_version: u64,
    pub base_revision: String,
    pub operations: Vec<TextOperation>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DirtyDraftRequest {
    pub wire_version: String,
    pub client_id: String,
    pub operation_id: String,
    pub epoch: u64,
    pub base_version: u64,
    pub base_revision: String,
    pub source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PresenceRequest {
    pub wire_version: String,
    pub client_id: String,
    pub epoch: u64,
    pub revision: String,
    pub cursor: u64,
    pub selection_start: u64,
    pub selection_end: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResyncRequest {
    pub wire_version: String,
    pub client_id: String,
    pub epoch: u64,
    pub revision: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct OperationKey {
    pub actor: String,
    pub client: String,
    pub operation: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RevisionComparison {
    pub expected_revision: String,
    pub actual_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentStateView {
    pub wire_version: &'static str,
    pub epoch: u64,
    pub version: u64,
    pub revision: String,
    pub frozen: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comparison: Option<RevisionComparison>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CollaborationError {
    UnsupportedWireVersion,
    InvalidClientId,
    InvalidOperationId,
    InvalidOperations,
    InvalidCursor,
    ReplayMismatch,
    EpochMismatch,
    VersionMismatch,
    HistoryUnavailable,
    OverlappingConcurrentEdit,
    Frozen,
    ExternalEdit,
}

impl CollaborationError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedWireVersion => "unsupported_collaboration_version",
            Self::InvalidClientId => "invalid_client_id",
            Self::InvalidOperationId => "invalid_operation_id",
            Self::InvalidOperations => "invalid_text_operations",
            Self::InvalidCursor => "invalid_presence_range",
            Self::ReplayMismatch => "collaboration_replay_mismatch",
            Self::EpochMismatch => "collaboration_epoch_mismatch",
            Self::VersionMismatch => "collaboration_version_mismatch",
            Self::HistoryUnavailable => "collaboration_resync_required",
            Self::OverlappingConcurrentEdit => "collaboration_conflict",
            Self::Frozen => "collaboration_frozen",
            Self::ExternalEdit => "external_edit_conflict",
        }
    }

    pub(crate) const fn freezes_document(self) -> bool {
        matches!(self, Self::OverlappingConcurrentEdit | Self::ExternalEdit)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedOperation {
    pub key: OperationKey,
    pub request_digest: String,
    pub request_base_revision: String,
    pub request_base_version: u64,
    pub applied_base_revision: String,
    pub applied_base_version: u64,
    pub operations: Vec<TextOperation>,
    pub next_source: String,
}

impl PreparedOperation {
    pub(crate) fn document_edits(&self) -> Vec<DocumentEdit> {
        self.operations
            .iter()
            .map(|operation| DocumentEdit {
                start: operation.start,
                end: operation.end,
                replacement: operation.replacement.clone(),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct AcceptedOperation {
    key: OperationKey,
    base_revision: String,
    base_version: u64,
    base_source: String,
    operations: Vec<TextOperation>,
}

#[derive(Clone, Debug)]
struct FreezeState {
    reason: &'static str,
    comparison: RevisionComparison,
}

#[derive(Clone, Debug)]
pub(crate) struct CollaborationDocument {
    epoch: u64,
    version: u64,
    revision: String,
    source: String,
    frozen: Option<FreezeState>,
    history: VecDeque<AcceptedOperation>,
}

impl CollaborationDocument {
    pub(crate) fn new(
        epoch: u64,
        version: u64,
        revision: String,
        source: String,
        frozen_reason: Option<&'static str>,
        expected_revision: Option<String>,
    ) -> Self {
        let frozen = frozen_reason.map(|reason| FreezeState {
            reason,
            comparison: RevisionComparison {
                expected_revision: expected_revision.unwrap_or_else(|| revision.clone()),
                actual_revision: revision.clone(),
            },
        });
        Self {
            epoch: epoch.max(1),
            version,
            revision,
            source,
            frozen,
            history: VecDeque::new(),
        }
    }

    pub(crate) fn state(&self) -> DocumentStateView {
        DocumentStateView {
            wire_version: WIRE_VERSION,
            epoch: self.epoch,
            version: self.version,
            revision: self.revision.clone(),
            frozen: self.frozen.is_some(),
            reason: self.frozen.as_ref().map(|freeze| freeze.reason),
            comparison: self.frozen.as_ref().map(|freeze| freeze.comparison.clone()),
        }
    }

    pub(crate) fn source(&self) -> &str {
        &self.source
    }

    pub(crate) fn reconcile_canonical(
        &mut self,
        revision: &str,
        source: &str,
    ) -> Result<(), CollaborationError> {
        if self.revision == revision && self.source == source {
            return Ok(());
        }
        let expected_revision = self.revision.clone();
        self.begin_new_epoch(
            revision.to_owned(),
            source.to_owned(),
            Some(FreezeState {
                reason: "external_edit",
                comparison: RevisionComparison {
                    expected_revision,
                    actual_revision: revision.to_owned(),
                },
            }),
        );
        Err(CollaborationError::ExternalEdit)
    }

    pub(crate) fn prepare_operation(
        &mut self,
        actor_id: &str,
        request: &OperationRequest,
    ) -> Result<PreparedOperation, CollaborationError> {
        validate_request_identity(
            &request.wire_version,
            &request.client_id,
            &request.operation_id,
        )?;
        let key = OperationKey {
            actor: actor_id.to_owned(),
            client: request.client_id.clone(),
            operation: request.operation_id.clone(),
        };
        self.prepare(
            key,
            request.epoch,
            request.base_version,
            &request.base_revision,
            request.operations.clone(),
            operation_request_digest(request),
        )
    }

    pub(crate) fn prepare_dirty_draft(
        &mut self,
        actor_id: &str,
        request: &DirtyDraftRequest,
    ) -> Result<PreparedOperation, CollaborationError> {
        validate_request_identity(
            &request.wire_version,
            &request.client_id,
            &request.operation_id,
        )?;
        let base_source = self
            .source_at(&request.base_revision, request.base_version)
            .ok_or(CollaborationError::HistoryUnavailable)?;
        let operations = single_span_diff(base_source, &request.source);
        let key = OperationKey {
            actor: actor_id.to_owned(),
            client: request.client_id.clone(),
            operation: request.operation_id.clone(),
        };
        self.prepare(
            key,
            request.epoch,
            request.base_version,
            &request.base_revision,
            operations,
            dirty_draft_request_digest(request),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare(
        &mut self,
        key: OperationKey,
        epoch: u64,
        base_version: u64,
        base_revision: &str,
        mut operations: Vec<TextOperation>,
        request_digest: String,
    ) -> Result<PreparedOperation, CollaborationError> {
        if self.frozen.is_some() {
            return Err(CollaborationError::Frozen);
        }
        if epoch != self.epoch {
            return Err(CollaborationError::EpochMismatch);
        }
        let base_source = self
            .source_at(base_revision, base_version)
            .ok_or_else(|| {
                if base_revision == self.revision {
                    CollaborationError::VersionMismatch
                } else {
                    CollaborationError::HistoryUnavailable
                }
            })?
            .to_owned();
        validate_operations(&base_source, &mut operations)?;

        if base_revision != self.revision || base_version != self.version {
            let history_start = self.history.iter().position(|accepted| {
                accepted.base_revision == base_revision && accepted.base_version == base_version
            });
            let Some(history_start) = history_start else {
                return Err(CollaborationError::HistoryUnavailable);
            };
            for accepted in self.history.iter().skip(history_start) {
                for prior in &accepted.operations {
                    for operation in &mut operations {
                        if transform_operation(operation, prior, &key, &accepted.key).is_err() {
                            self.freeze_overlap(base_revision);
                            return Err(CollaborationError::OverlappingConcurrentEdit);
                        }
                    }
                }
            }
            validate_operations(&self.source, &mut operations)?;
        }

        let next_source = apply_operations(&self.source, &operations)?;
        Ok(PreparedOperation {
            key,
            request_digest,
            request_base_revision: base_revision.to_owned(),
            request_base_version: base_version,
            applied_base_revision: self.revision.clone(),
            applied_base_version: self.version,
            operations,
            next_source,
        })
    }

    pub(crate) fn accept(
        &mut self,
        prepared: PreparedOperation,
        result_revision: String,
    ) -> DocumentStateView {
        let result_version = self.version.saturating_add(1);
        let accepted = AcceptedOperation {
            key: prepared.key,
            base_revision: self.revision.clone(),
            base_version: self.version,
            base_source: self.source.clone(),
            operations: prepared.operations,
        };
        self.source = prepared.next_source;
        self.revision = result_revision;
        self.version = result_version;
        self.history.push_back(accepted);
        if self.history.len() > HISTORY_LIMIT {
            self.history.pop_front();
        }
        self.state()
    }

    pub(crate) fn acknowledge_resync(
        &mut self,
        request: &ResyncRequest,
    ) -> Result<DocumentStateView, CollaborationError> {
        validate_wire_and_client(&request.wire_version, &request.client_id)?;
        if request.epoch != self.epoch || request.revision != self.revision {
            return Err(CollaborationError::EpochMismatch);
        }
        self.frozen = None;
        self.history.clear();
        Ok(self.state())
    }

    fn source_at(&self, revision: &str, version: u64) -> Option<&str> {
        if revision == self.revision && version == self.version {
            return Some(&self.source);
        }
        self.history
            .iter()
            .find(|accepted| accepted.base_revision == revision && accepted.base_version == version)
            .map(|accepted| accepted.base_source.as_str())
    }

    fn freeze_overlap(&mut self, expected_revision: &str) {
        self.epoch = self.epoch.saturating_add(1);
        self.version = 0;
        self.history.clear();
        self.frozen = Some(FreezeState {
            reason: "overlapping_concurrent_edit",
            comparison: RevisionComparison {
                expected_revision: expected_revision.to_owned(),
                actual_revision: self.revision.clone(),
            },
        });
    }

    fn begin_new_epoch(&mut self, revision: String, source: String, frozen: Option<FreezeState>) {
        self.epoch = self.epoch.saturating_add(1);
        self.version = 0;
        self.revision = revision;
        self.source = source;
        self.frozen = frozen;
        self.history.clear();
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Participant {
    pub actor_id: String,
    pub client_id: String,
    pub role: String,
    pub cursor: u64,
    pub selection_start: u64,
    pub selection_end: u64,
    pub expires_at: i64,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PresenceRegistry {
    participants: BTreeMap<(NodeId, String, String), Participant>,
}

impl PresenceRegistry {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn upsert(
        &mut self,
        node_id: NodeId,
        actor_id: &str,
        role: &str,
        source: &str,
        document: &DocumentStateView,
        request: &PresenceRequest,
        now: i64,
    ) -> Result<Vec<Participant>, CollaborationError> {
        validate_wire_and_client(&request.wire_version, &request.client_id)?;
        if request.epoch != document.epoch || request.revision != document.revision {
            return Err(CollaborationError::EpochMismatch);
        }
        validate_range(source, request.cursor, request.cursor)
            .and_then(|()| validate_range(source, request.selection_start, request.selection_end))
            .map_err(|_| CollaborationError::InvalidCursor)?;
        self.prune(now);
        self.participants.insert(
            (node_id, actor_id.to_owned(), request.client_id.clone()),
            Participant {
                actor_id: actor_id.to_owned(),
                client_id: request.client_id.clone(),
                role: role.to_owned(),
                cursor: request.cursor,
                selection_start: request.selection_start,
                selection_end: request.selection_end,
                expires_at: now.saturating_add(PRESENCE_TTL_SECONDS),
            },
        );
        Ok(self.for_node(node_id, now))
    }

    pub(crate) fn leave(
        &mut self,
        node_id: NodeId,
        actor_id: &str,
        client_id: &str,
        now: i64,
    ) -> Result<Vec<Participant>, CollaborationError> {
        validate_uuid(client_id).map_err(|()| CollaborationError::InvalidClientId)?;
        self.participants
            .remove(&(node_id, actor_id.to_owned(), client_id.to_owned()));
        Ok(self.for_node(node_id, now))
    }

    pub(crate) fn for_node(&mut self, node_id: NodeId, now: i64) -> Vec<Participant> {
        self.prune(now);
        self.participants
            .iter()
            .filter(|((candidate, _, _), _)| *candidate == node_id)
            .map(|(_, participant)| participant.clone())
            .collect()
    }

    fn prune(&mut self, now: i64) {
        self.participants
            .retain(|_, participant| participant.expires_at > now);
    }
}

pub(crate) fn validate_uuid(value: &str) -> Result<(), ()> {
    NodeId::from_str(value).map(|_| ()).map_err(|_| ())
}

pub(crate) fn validate_operation_request(
    request: &OperationRequest,
) -> Result<(), CollaborationError> {
    validate_request_identity(
        &request.wire_version,
        &request.client_id,
        &request.operation_id,
    )
}

pub(crate) fn validate_dirty_draft_request(
    request: &DirtyDraftRequest,
) -> Result<(), CollaborationError> {
    validate_request_identity(
        &request.wire_version,
        &request.client_id,
        &request.operation_id,
    )
}

fn validate_request_identity(
    wire_version: &str,
    client_id: &str,
    operation_id: &str,
) -> Result<(), CollaborationError> {
    validate_wire_and_client(wire_version, client_id)?;
    validate_uuid(operation_id).map_err(|()| CollaborationError::InvalidOperationId)
}

fn validate_wire_and_client(wire_version: &str, client_id: &str) -> Result<(), CollaborationError> {
    if wire_version != WIRE_VERSION {
        return Err(CollaborationError::UnsupportedWireVersion);
    }
    validate_uuid(client_id).map_err(|()| CollaborationError::InvalidClientId)
}

fn validate_operations(
    source: &str,
    operations: &mut [TextOperation],
) -> Result<(), CollaborationError> {
    if operations.is_empty() || operations.len() > MAX_OPERATIONS {
        return Err(CollaborationError::InvalidOperations);
    }
    let replacement_bytes = operations.iter().try_fold(0_usize, |total, operation| {
        total.checked_add(operation.replacement.len())
    });
    if replacement_bytes.is_none_or(|total| total > MAX_REPLACEMENT_BYTES) {
        return Err(CollaborationError::InvalidOperations);
    }
    for operation in operations.iter() {
        validate_range(source, operation.start, operation.end)?;
    }
    operations.sort_by(|left, right| {
        right
            .start
            .cmp(&left.start)
            .then_with(|| right.end.cmp(&left.end))
    });
    for pair in operations.windows(2) {
        let higher = &pair[0];
        let lower = &pair[1];
        if lower.end > higher.start
            || (lower.end == higher.start && (lower.is_insertion() || higher.is_insertion()))
        {
            return Err(CollaborationError::InvalidOperations);
        }
    }
    Ok(())
}

fn validate_range(source: &str, start: u64, end: u64) -> Result<(), CollaborationError> {
    let start = usize::try_from(start).map_err(|_| CollaborationError::InvalidOperations)?;
    let end = usize::try_from(end).map_err(|_| CollaborationError::InvalidOperations)?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
    {
        return Err(CollaborationError::InvalidOperations);
    }
    Ok(())
}

fn apply_operations(
    source: &str,
    operations: &[TextOperation],
) -> Result<String, CollaborationError> {
    let mut result = source.to_owned();
    for operation in operations {
        let start =
            usize::try_from(operation.start).map_err(|_| CollaborationError::InvalidOperations)?;
        let end =
            usize::try_from(operation.end).map_err(|_| CollaborationError::InvalidOperations)?;
        result.replace_range(start..end, &operation.replacement);
    }
    Ok(result)
}

fn transform_operation(
    incoming: &mut TextOperation,
    accepted: &TextOperation,
    incoming_key: &OperationKey,
    accepted_key: &OperationKey,
) -> Result<(), CollaborationError> {
    if incoming.is_insertion() && accepted.is_insertion() && incoming.start == accepted.start {
        if incoming_key > accepted_key {
            let shift = u64::try_from(accepted.replacement.len())
                .map_err(|_| CollaborationError::InvalidOperations)?;
            incoming.start = incoming
                .start
                .checked_add(shift)
                .ok_or(CollaborationError::InvalidOperations)?;
            incoming.end = incoming.start;
        }
        return Ok(());
    }

    if accepted.is_insertion() {
        let point = accepted.start;
        if incoming.end < point {
            return Ok(());
        }
        if incoming.start > point {
            return shift_operation(incoming, accepted.delta()?);
        }
        return Err(CollaborationError::OverlappingConcurrentEdit);
    }

    if incoming.is_insertion() {
        let point = incoming.start;
        if point < accepted.start {
            return Ok(());
        }
        if point > accepted.end {
            return shift_operation(incoming, accepted.delta()?);
        }
        return Err(CollaborationError::OverlappingConcurrentEdit);
    }

    if accepted.end <= incoming.start {
        return shift_operation(incoming, accepted.delta()?);
    }
    if incoming.end <= accepted.start {
        return Ok(());
    }
    Err(CollaborationError::OverlappingConcurrentEdit)
}

fn shift_operation(operation: &mut TextOperation, delta: i64) -> Result<(), CollaborationError> {
    operation.start = shift_offset(operation.start, delta)?;
    operation.end = shift_offset(operation.end, delta)?;
    Ok(())
}

fn shift_offset(value: u64, delta: i64) -> Result<u64, CollaborationError> {
    if delta >= 0 {
        value
            .checked_add(u64::try_from(delta).map_err(|_| CollaborationError::InvalidOperations)?)
            .ok_or(CollaborationError::InvalidOperations)
    } else {
        value
            .checked_sub(delta.unsigned_abs())
            .ok_or(CollaborationError::InvalidOperations)
    }
}

fn single_span_diff(base: &str, proposed: &str) -> Vec<TextOperation> {
    if base == proposed {
        return vec![TextOperation {
            start: u64::try_from(base.len()).unwrap_or(u64::MAX),
            end: u64::try_from(base.len()).unwrap_or(u64::MAX),
            replacement: String::new(),
        }];
    }
    let prefix = common_prefix_boundary(base, proposed);
    let suffix = common_suffix_boundary(&base[prefix..], &proposed[prefix..]);
    vec![TextOperation {
        start: u64::try_from(prefix).unwrap_or(u64::MAX),
        end: u64::try_from(base.len() - suffix).unwrap_or(u64::MAX),
        replacement: proposed[prefix..proposed.len() - suffix].to_owned(),
    }]
}

fn common_prefix_boundary(left: &str, right: &str) -> usize {
    let bytes = left
        .as_bytes()
        .iter()
        .zip(right.as_bytes())
        .take_while(|(left, right)| left == right)
        .count();
    let mut boundary = bytes;
    while boundary > 0 && (!left.is_char_boundary(boundary) || !right.is_char_boundary(boundary)) {
        boundary -= 1;
    }
    boundary
}

fn common_suffix_boundary(left: &str, right: &str) -> usize {
    let max = left.len().min(right.len());
    let bytes = left
        .as_bytes()
        .iter()
        .rev()
        .zip(right.as_bytes().iter().rev())
        .take(max)
        .take_while(|(left, right)| left == right)
        .count();
    let mut suffix = bytes;
    while suffix > 0
        && (!left.is_char_boundary(left.len() - suffix)
            || !right.is_char_boundary(right.len() - suffix))
    {
        suffix -= 1;
    }
    suffix
}

pub(crate) fn operation_request_digest(request: &OperationRequest) -> String {
    digest_json(request)
}

pub(crate) fn dirty_draft_request_digest(request: &DirtyDraftRequest) -> String {
    digest_json(request)
}

fn digest_json(value: &impl Serialize) -> String {
    let bytes = serde_json::to_vec(value).expect("closed collaboration request can serialize");
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        write!(&mut encoded, "{byte:02x}").expect("writing to a String cannot fail");
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACTOR: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
    const CLIENT_A: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
    const CLIENT_B: &str = "cccccccc-cccc-4ccc-8ccc-cccccccccccc";
    const CLIENT_C: &str = "dddddddd-dddd-4ddd-8ddd-dddddddddddd";
    const OP_A: &str = "11111111-1111-4111-8111-111111111111";
    const OP_B: &str = "22222222-2222-4222-8222-222222222222";

    fn request(
        client_id: &str,
        operation_id: &str,
        base_revision: &str,
        base_version: u64,
        operation: TextOperation,
    ) -> OperationRequest {
        OperationRequest {
            wire_version: WIRE_VERSION.to_owned(),
            client_id: client_id.to_owned(),
            operation_id: operation_id.to_owned(),
            epoch: 1,
            base_version,
            base_revision: base_revision.to_owned(),
            operations: vec![operation],
        }
    }

    fn accept_with_revision(
        document: &mut CollaborationDocument,
        request: &OperationRequest,
        revision: &str,
    ) {
        let prepared = document
            .prepare_operation(ACTOR, request)
            .expect("prepare operation");
        document.accept(prepared, revision.to_owned());
    }

    #[test]
    fn non_overlapping_concurrent_edits_transform_to_the_same_bytes() {
        let left = request(
            CLIENT_A,
            OP_A,
            "r0",
            0,
            TextOperation {
                start: 0,
                end: 1,
                replacement: "A".to_owned(),
            },
        );
        let right = request(
            CLIENT_B,
            OP_B,
            "r0",
            0,
            TextOperation {
                start: 4,
                end: 5,
                replacement: "Z".to_owned(),
            },
        );

        let mut left_first =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "hello".to_owned(), None, None);
        accept_with_revision(&mut left_first, &left, "r1");
        accept_with_revision(&mut left_first, &right, "r2");

        let mut right_first =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "hello".to_owned(), None, None);
        accept_with_revision(&mut right_first, &right, "r1b");
        accept_with_revision(&mut right_first, &left, "r2b");

        assert_eq!(left_first.source, "AellZ");
        assert_eq!(right_first.source, "AellZ");
    }

    #[test]
    fn same_position_insertions_use_operation_identity_tie_break() {
        let lower = request(
            CLIENT_A,
            OP_A,
            "r0",
            0,
            TextOperation {
                start: 1,
                end: 1,
                replacement: "A".to_owned(),
            },
        );
        let higher = request(
            CLIENT_B,
            OP_B,
            "r0",
            0,
            TextOperation {
                start: 1,
                end: 1,
                replacement: "B".to_owned(),
            },
        );

        let mut lower_first =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "xy".to_owned(), None, None);
        accept_with_revision(&mut lower_first, &lower, "r1");
        accept_with_revision(&mut lower_first, &higher, "r2");
        let mut higher_first =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "xy".to_owned(), None, None);
        accept_with_revision(&mut higher_first, &higher, "r1b");
        accept_with_revision(&mut higher_first, &lower, "r2b");

        assert_eq!(lower_first.source, "xABy");
        assert_eq!(higher_first.source, "xABy");
    }

    #[test]
    fn every_three_session_arrival_permutation_has_identical_tie_break_output() {
        let requests = [
            request(
                CLIENT_A,
                "30000000-0000-4000-8000-000000000001",
                "r0",
                0,
                TextOperation {
                    start: 1,
                    end: 1,
                    replacement: "A".to_owned(),
                },
            ),
            request(
                CLIENT_B,
                "30000000-0000-4000-8000-000000000002",
                "r0",
                0,
                TextOperation {
                    start: 1,
                    end: 1,
                    replacement: "B".to_owned(),
                },
            ),
            request(
                CLIENT_C,
                "30000000-0000-4000-8000-000000000003",
                "r0",
                0,
                TextOperation {
                    start: 1,
                    end: 1,
                    replacement: "C".to_owned(),
                },
            ),
        ];
        for permutation in [
            [0, 1, 2],
            [0, 2, 1],
            [1, 0, 2],
            [1, 2, 0],
            [2, 0, 1],
            [2, 1, 0],
        ] {
            let mut document =
                CollaborationDocument::new(1, 0, "r0".to_owned(), "xy".to_owned(), None, None);
            for (version, index) in permutation.into_iter().enumerate() {
                accept_with_revision(
                    &mut document,
                    &requests[index],
                    &format!("r{}", version + 1),
                );
            }
            assert_eq!(document.source, "xABCy");
        }
    }

    #[test]
    fn overlapping_concurrent_edit_freezes_and_requires_new_epoch_resync() {
        let first = request(
            CLIENT_A,
            OP_A,
            "r0",
            0,
            TextOperation {
                start: 1,
                end: 4,
                replacement: "one".to_owned(),
            },
        );
        let second = request(
            CLIENT_B,
            OP_B,
            "r0",
            0,
            TextOperation {
                start: 2,
                end: 5,
                replacement: "two".to_owned(),
            },
        );
        let mut document =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "abcdef".to_owned(), None, None);
        accept_with_revision(&mut document, &first, "r1");
        assert!(matches!(
            document.prepare_operation(ACTOR, &second),
            Err(CollaborationError::OverlappingConcurrentEdit)
        ));
        let state = document.state();
        assert!(state.frozen);
        assert_eq!(state.epoch, 2);
        assert_eq!(state.reason, Some("overlapping_concurrent_edit"));
        let resync = ResyncRequest {
            wire_version: WIRE_VERSION.to_owned(),
            client_id: CLIENT_B.to_owned(),
            epoch: 2,
            revision: "r1".to_owned(),
        };
        assert!(!document.acknowledge_resync(&resync).unwrap().frozen);
    }

    #[test]
    fn utf8_boundaries_and_stale_history_are_fail_closed() {
        let mut document =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "你a🙂".to_owned(), None, None);
        let invalid = request(
            CLIENT_A,
            OP_A,
            "r0",
            0,
            TextOperation {
                start: 1,
                end: 3,
                replacement: "x".to_owned(),
            },
        );
        assert!(matches!(
            document.prepare_operation(ACTOR, &invalid),
            Err(CollaborationError::InvalidOperations)
        ));
        let unavailable = request(
            CLIENT_A,
            OP_A,
            "unknown",
            0,
            TextOperation {
                start: 0,
                end: 0,
                replacement: "x".to_owned(),
            },
        );
        assert!(matches!(
            document.prepare_operation(ACTOR, &unavailable),
            Err(CollaborationError::HistoryUnavailable)
        ));
    }

    #[test]
    fn external_source_change_is_exact_and_frozen_without_becoming_authority() {
        let mut document =
            CollaborationDocument::new(1, 4, "r4".to_owned(), "old".to_owned(), None, None);
        assert_eq!(
            document.reconcile_canonical("external", "changed"),
            Err(CollaborationError::ExternalEdit)
        );
        let state = document.state();
        assert_eq!(state.epoch, 2);
        assert_eq!(state.version, 0);
        assert_eq!(state.revision, "external");
        assert_eq!(
            state.comparison,
            Some(RevisionComparison {
                expected_revision: "r4".to_owned(),
                actual_revision: "external".to_owned(),
            })
        );
    }

    #[test]
    fn offline_draft_rebases_only_when_its_single_changed_span_is_disjoint() {
        let mut document =
            CollaborationDocument::new(1, 0, "r0".to_owned(), "alpha beta".to_owned(), None, None);
        let online = request(
            CLIENT_A,
            OP_A,
            "r0",
            0,
            TextOperation {
                start: 0,
                end: 5,
                replacement: "ALPHA".to_owned(),
            },
        );
        accept_with_revision(&mut document, &online, "r1");
        let draft = DirtyDraftRequest {
            wire_version: WIRE_VERSION.to_owned(),
            client_id: CLIENT_B.to_owned(),
            operation_id: OP_B.to_owned(),
            epoch: 1,
            base_version: 0,
            base_revision: "r0".to_owned(),
            source: "alpha BETA".to_owned(),
        };
        let prepared = document
            .prepare_dirty_draft(ACTOR, &draft)
            .expect("disjoint draft rebase");
        assert_eq!(prepared.next_source, "ALPHA BETA");
    }

    #[test]
    fn presence_is_utf8_checked_actor_scoped_and_expires() {
        let node_id = NodeId::from_str("dddddddd-dddd-4ddd-8ddd-dddddddddddd").unwrap();
        let state = DocumentStateView {
            wire_version: WIRE_VERSION,
            epoch: 1,
            version: 0,
            revision: "r0".to_owned(),
            frozen: false,
            reason: None,
            comparison: None,
        };
        let request = PresenceRequest {
            wire_version: WIRE_VERSION.to_owned(),
            client_id: CLIENT_A.to_owned(),
            epoch: 1,
            revision: "r0".to_owned(),
            cursor: 3,
            selection_start: 0,
            selection_end: 3,
        };
        let mut registry = PresenceRegistry::default();
        let participants = registry
            .upsert(node_id, ACTOR, "editor", "你a", &state, &request, 10)
            .unwrap();
        assert_eq!(participants.len(), 1);
        assert_eq!(registry.for_node(node_id, 54).len(), 1);
        assert!(registry.for_node(node_id, 55).is_empty());
    }
}
