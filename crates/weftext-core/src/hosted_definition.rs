//! Actor-owned definition identities and immutable Draft proposals. Allocation
//! is not a writer: admission requires the original committed source save.
use super::*;
use std::collections::BTreeSet;
use weftext_data::{FieldPatch, SavedQueryBinding};
use weftext_knowledge::NodeRef;

const MAX_SAVE_RESERVATIONS: usize = 32;
const MAX_PENDING_RESERVATIONS: usize = 128;
const MAX_RECORD_BYTES: usize = 2 * MAX_SOURCE_BYTES + 64 * 1024;
const MAX_LIST_BYTES: usize = 64 * 1024 * 1024;

#[cfg(all(test, windows))]
thread_local! {
    static FAILPOINT: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}
#[cfg(all(test, windows))]
pub(crate) fn set_test_failpoint(point: &'static str) {
    FAILPOINT.with(|active| active.set(Some(point)));
}
#[cfg(all(test, windows))]
pub(crate) fn test_failpoint(point: &str) -> Result<()> {
    FAILPOINT.with(|active| {
        if active.get() == Some(point) {
            active.set(None);
            Err(CoreError::new(
                "injectedFailure",
                format!("Interrupted after {point}"),
            ))
        } else {
            Ok(())
        }
    })
}

/// Internal service DTO. The host computes the intent fingerprint from the
/// exact registered Draft, label and plan; binding is never a client choice.
#[derive(Debug)]
pub struct HostedDefinitionReservationRequest {
    pub session_id: String,
    pub base_id: String,
    pub operation_id: String,
    pub intent_fingerprint: String,
    pub binding: SavedQueryBinding,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorDefinitionStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub key: Option<String>,
    pub owner: Option<NodeRef>,
    pub binding: Option<SavedQueryBinding>,
    pub save_operation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorDefinitionProposal {
    pub status: ActorDefinitionStatus,
    pub patch: Option<FieldPatch>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DefinitionOperation {
    /// None is permitted only for a cancelled request that never allocated.
    proposal: Option<DefinitionProposalRecord>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionProposalRecord {
    key: String,
    owner: NodeRef,
    identity: FileIdentity,
    initial_root: String,
    initial_session_id: String,
    initial_base_id: String,
    intent_fingerprint: String,
    binding: SavedQueryBinding,
    patch: FieldPatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct SaveDefinitionReservation {
    proposal_id: String,
    key: String,
    owner: NodeRef,
    identity: FileIdentity,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionSaveRecord {
    core_operation_id: String,
    save_operation_id: String,
    actor: String,
    resource: String,
    path: String,
    save_fingerprint: String,
    source_hash: String,
    reservations: Vec<SaveDefinitionReservation>,
}

fn conflict() -> CoreError {
    CoreError::new(
        "operationConflict",
        "The original definition actor, owner or intent differs",
    )
}
fn valid_resource(resource: &str) -> Result<()> {
    if resource.is_empty() || resource.len() > 256 || resource.chars().any(char::is_control) {
        return Err(CoreError::new(
            "invalidResource",
            "A bounded resource identity is required",
        ));
    }
    Ok(())
}
fn valid_fingerprint(fingerprint: &str) -> Result<()> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err(CoreError::new(
            "invalidIntent",
            "A lowercase SHA-256 author intent fingerprint is required",
        ));
    }
    Ok(())
}

fn utf16_byte_offset(source: &str, offset: usize) -> Option<usize> {
    let mut ordinal = 0usize;
    for (byte, character) in source.char_indices() {
        if ordinal == offset {
            return Some(byte);
        }
        ordinal = ordinal.checked_add(character.len_utf16())?;
        if ordinal > offset {
            return None;
        }
    }
    (ordinal == offset).then_some(source.len())
}

fn exact_insertion(patch: &FieldPatch) -> bool {
    patch.from == patch.to
        && utf16_byte_offset(&patch.source, patch.from)
            .and_then(|from| from.checked_add(patch.text.len()).map(|to| (from, to)))
            .is_some_and(|(from, to)| patch.source.get(from..to) == Some(patch.text.as_str()))
}

fn bounded_record(backend: &crate::BackendContext, path: &Path) -> Result<Option<Vec<u8>>> {
    let file = match backend.open_read(path) {
        Ok(file) => file,
        Err(e) if e.code == "missing" => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut bytes = vec![];
    file.take((MAX_RECORD_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(CoreError::new(
            "budget",
            "Definition record exceeds its byte budget",
        ));
    }
    Ok(Some(bytes))
}

pub(super) fn canonical_proposal_ids(ids: &[String]) -> Result<Vec<String>> {
    if ids.len() > MAX_SAVE_RESERVATIONS {
        return Err(CoreError::new(
            "budget",
            "At most 32 definition reservations per source save",
        ));
    }
    for id in ids {
        valid_operation_id(id)?;
        if Uuid::parse_str(id).unwrap().to_string() != *id {
            return Err(CoreError::new(
                "invalidOperationId",
                "A canonical proposal UUID is required",
            ));
        }
    }
    Ok(ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

impl Core {
    fn definition_record(&self, resource: &str, operation: &str) -> Result<Option<ActorOperation>> {
        valid_resource(resource)?;
        canonical_proposal_ids(&[operation.to_owned()])?;
        let Some(bytes) = bounded_record(&self.backend, &self.actor_record(operation)?)? else {
            return Ok(None);
        };
        let record: ActorOperation = serde_json::from_slice(&bytes)?;
        self.check_actor_library(&record.library)?;
        if self.actor.as_ref() != Some(&record.actor)
            || record.resource != resource
            || record.definition.is_none()
            || record.field.is_some()
            || record.feature.is_some()
            || !record.save_definition_reservations.is_empty()
            || record.history.is_some()
            || record.owner_backup.is_some()
        {
            return Err(conflict());
        }
        valid_operation_id(&record.core_operation_id)?;
        if let Some(proposal) = &record.definition.as_ref().unwrap().proposal {
            valid_fingerprint(&proposal.intent_fingerprint)?;
            valid_operation_id(&proposal.key)?;
            if Uuid::parse_str(&proposal.key).unwrap().to_string() != proposal.key
                || Uuid::parse_str(&proposal.key).unwrap().get_version_num() != 4
            {
                return Err(conflict());
            }
        } else if !record.cancelled || !record.fingerprint.is_empty() {
            return Err(conflict());
        }
        Ok(Some(record))
    }

    fn definition_owner(&self, root: &Path, session_id: &str) -> Result<(NodeRef, FileIdentity)> {
        self.actor_library_for_root(root)?;
        self.check_actor_session(session_id)?;
        let root = absolute_path(root)?;
        let workspace = self.require_workspace(&root)?;
        let session = &self.sessions[session_id];
        if !session.path.starts_with(&root)
            || session.node_id.is_none()
            || !is_canonical(&session.path)
        {
            return Err(CoreError::new(
                "unsupportedRole",
                "Definitions require an admitted canonical owner source",
            ));
        }
        let birth = self
            .admitted_binding(&session.stamp.identity)?
            .ok_or_else(conflict)?;
        if birth.workspace_id != workspace.workspace_id
            || Some(&birth.node_id) != session.node_id.as_ref()
        {
            return Err(conflict());
        }
        Ok((
            NodeRef {
                workspace_id: workspace.workspace_id,
                node_id: birth.node_id,
            },
            session.stamp.identity.clone(),
        ))
    }

    /// Freeze the exact first Draft patch and key in one durable actor record.
    /// The callback is pure; it must not write a source or call another Core.
    pub fn actor_definition_reserve(
        &mut self,
        resource: &str,
        root: &Path,
        request: HostedDefinitionReservationRequest,
        build: impl FnOnce(&str, &SavedQueryBinding) -> Result<FieldPatch>,
    ) -> Result<ActorDefinitionProposal> {
        valid_fingerprint(&request.intent_fingerprint)?;
        if let Some(record) = self.definition_record(resource, &request.operation_id)? {
            if let Some(proposal) = &record.definition.as_ref().unwrap().proposal
                && (proposal.intent_fingerprint != request.intent_fingerprint
                    || Path::new(&proposal.initial_root) != absolute_path(root)?
                    || proposal.initial_session_id != request.session_id
                    || proposal.initial_base_id != request.base_id
                    || serde_json::to_value(&proposal.binding)?
                        != serde_json::to_value(&request.binding)?)
            {
                return Err(conflict());
            }
            return self
                .actor_definition_details(resource, &request.operation_id)?
                .ok_or_else(conflict);
        }
        self.check_session_base(&request.session_id, &request.base_id)?;
        let (owner, identity) = self.definition_owner(root, &request.session_id)?;
        if request.binding.schema != format!("weftext-config://{}/schema", owner.workspace_id)
            || request.binding.language != "weftext.query.cel.v1"
        {
            return Err(CoreError::new(
                "schemaBasisChanged",
                "The initial definition schema role or language differs",
            ));
        }
        // require an original admitted configuration, never a supplied UUID.
        self.read_config_revision(root, ConfigRole::Schema, &request.binding.revision)?;
        if self
            .actor_definition_list(resource, root, &request.session_id)?
            .len()
            >= MAX_PENDING_RESERVATIONS
        {
            return Err(CoreError::new(
                "budget",
                "At most 128 pending definition reservations per owner",
            ));
        }
        let key = fresh();
        let patch = build(&key, &request.binding)?;
        check_size(patch.source.as_bytes())?;
        check_size(patch.text.as_bytes())?;
        if !exact_insertion(&patch)
            || weftext_engine::inspect_metadata(&patch.source)
                .id
                .as_deref()
                != Some(&owner.node_id)
            || !weftext_knowledge::definition_slots(&patch.source, &owner)
                .is_ok_and(|slots| slots.iter().any(|slot| slot.key == key))
            || !weftext_data::saved_queries(&patch.source).is_ok_and(|queries| {
                queries.iter().any(|query| {
                    query.key == key
                        && serde_json::to_value(&query.binding).ok()
                            == serde_json::to_value(&request.binding).ok()
                })
            })
        {
            return Err(CoreError::new(
                "definitionQualification",
                "The exact Draft patch lacks its qualified owner/key/binding",
            ));
        }
        let fingerprint = hash(&serde_json::to_vec(&serde_json::json!({
            "kind":"definitionReservation","actor":self.actor,"resource":resource,
            "root":display(&absolute_path(root)?),"owner":owner,"identity":identity,
            "session":request.session_id,"base":request.base_id,
            "intent":request.intent_fingerprint,"binding":request.binding,
        }))?);
        let record = ActorOperation {
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint,
            core_operation_id: fresh(),
            cancelled: false,
            field: None,
            feature: None,
            definition: Some(DefinitionOperation {
                proposal: Some(DefinitionProposalRecord {
                    key,
                    owner,
                    identity,
                    intent_fingerprint: request.intent_fingerprint,
                    initial_root: display(&absolute_path(root)?),
                    initial_session_id: request.session_id,
                    initial_base_id: request.base_id,
                    binding: request.binding,
                    patch,
                }),
            }),
            save_definition_reservations: vec![],
            history: None,
            owner_backup: None,
        };
        let bytes = Self::actor_operation_bytes(&record)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CoreError::new(
                "budget",
                "Definition proposal exceeds its durable budget",
            ));
        }
        journal::private_create(
            &self.backend,
            &self.actor_record(&request.operation_id)?,
            &bytes,
        )?;
        #[cfg(all(test, windows))]
        test_failpoint("reservationDurable")?;
        self.actor_definition_details(resource, &request.operation_id)?
            .ok_or_else(conflict)
    }

    pub fn actor_definition_retry(
        &self,
        resource: &str,
        operation: &str,
        intent_fingerprint: &str,
    ) -> Result<Option<ActorDefinitionProposal>> {
        valid_fingerprint(intent_fingerprint)?;
        let Some(record) = self.definition_record(resource, operation)? else {
            return Ok(None);
        };
        if let Some(proposal) = &record.definition.as_ref().unwrap().proposal
            && proposal.intent_fingerprint != intent_fingerprint
        {
            return Err(conflict());
        }
        self.actor_definition_details(resource, operation)
    }

    pub fn actor_definition_status(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorDefinitionStatus>> {
        self.backend.check_path(&self.data_dir)?;
        let Some(record) = self.definition_record(resource, operation)? else {
            return Ok(None);
        };
        let proposal = record.definition.as_ref().unwrap().proposal.as_ref();
        let mut state = if record.cancelled {
            "cancelled"
        } else {
            "reserved"
        };
        let mut saved = None;
        if let Some(proposal) = proposal {
            for plan in self.operations.values() {
                let Some(admissions) = self.definition_save_record(plan)? else {
                    continue;
                };
                if admissions.actor != record.actor
                    || admissions.resource != resource
                    || !admissions.reservations.iter().any(|r| {
                        r.proposal_id == operation
                            && r.key == proposal.key
                            && r.owner == proposal.owner
                            && r.identity == proposal.identity
                    })
                {
                    continue;
                }
                if self.committed(&plan.operation_id)? {
                    state = "admitted";
                    saved = Some(admissions.save_operation_id);
                    break;
                }
                if !self.resolved(&plan.operation_id)? {
                    state = "recoveryPending";
                    saved = Some(admissions.save_operation_id);
                }
            }
        }
        Ok(Some(ActorDefinitionStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            status: state.into(),
            key: proposal.map(|p| p.key.clone()),
            owner: proposal.map(|p| p.owner.clone()),
            binding: proposal.map(|p| p.binding.clone()),
            save_operation_id: saved,
        }))
    }

    /// The host separately authorizes delivery of the retained exact source.
    /// This read does not regenerate, execute, or apply the proposal.
    pub fn actor_definition_details(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorDefinitionProposal>> {
        let Some(record) = self.definition_record(resource, operation)? else {
            return Ok(None);
        };
        let status = self
            .actor_definition_status(resource, operation)?
            .ok_or_else(conflict)?;
        let patch = if status.status == "cancelled" {
            None
        } else {
            record.definition.unwrap().proposal.map(|p| p.patch)
        };
        Ok(Some(ActorDefinitionProposal { status, patch }))
    }

    pub fn actor_definition_settle(
        &mut self,
        resource: &str,
        operation: &str,
    ) -> Result<ActorDefinitionStatus> {
        let path = self.actor_record(operation)?;
        let mut record = match self.definition_record(resource, operation)? {
            Some(record) => record,
            None => ActorOperation {
                library: self.actor_library.clone(),
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: true,
                field: None,
                feature: None,
                definition: Some(DefinitionOperation { proposal: None }),
                save_definition_reservations: vec![],
                history: None,
                owner_backup: None,
            },
        };
        if let Some(status) = self.actor_definition_status(resource, operation)? {
            if status.status != "reserved" {
                return Ok(status);
            }
            record.cancelled = true;
            private_replace(&self.backend, &path, &Self::actor_operation_bytes(&record)?)?;
        } else {
            journal::private_create(&self.backend, &path, &Self::actor_operation_bytes(&record)?)?;
        }
        self.actor_definition_status(resource, operation)?
            .ok_or_else(conflict)
    }

    /// Only still-pending identities for this actor's opened owner are listed.
    /// Historical admission and cancellation remain queryable by original ID.
    pub fn actor_definition_list(
        &self,
        resource: &str,
        root: &Path,
        session_id: &str,
    ) -> Result<Vec<ActorDefinitionStatus>> {
        self.backend.check_path(&self.data_dir)?;
        valid_resource(resource)?;
        self.check_actor_session(session_id)?;
        self.check_session_base(session_id, &self.sessions[session_id].base_id)?;
        let (owner, identity) = self.definition_owner(root, session_id)?;
        let mut result = vec![];
        let mut bytes_used = 0usize;
        for entry in self.backend.read_dir(self.draft_directory("operations"))? {
            let path = entry?.path();
            let Some(operation) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().and_then(|s| s.to_str()) != Some("json")
                || Uuid::parse_str(operation).is_err()
            {
                continue;
            }
            let Some(bytes) = bounded_record(&self.backend, &path)? else {
                return Err(conflict());
            };
            bytes_used = bytes_used.checked_add(bytes.len()).ok_or_else(conflict)?;
            if bytes_used > MAX_LIST_BYTES {
                return Err(CoreError::new(
                    "budget",
                    "Definition enumeration exceeds 64 MiB",
                ));
            }
            let record: ActorOperation = serde_json::from_slice(&bytes)?;
            self.check_actor_library(&record.library)?;
            if self.actor.as_ref() != Some(&record.actor) {
                return Err(conflict());
            }
            if record.resource != resource {
                continue;
            }
            let Some(proposal) = record.definition.as_ref().and_then(|d| d.proposal.as_ref())
            else {
                continue;
            };
            if proposal.owner != owner || proposal.identity != identity {
                continue;
            }
            let status = self
                .actor_definition_status(resource, operation)?
                .ok_or_else(conflict)?;
            if matches!(status.status.as_str(), "reserved" | "recoveryPending") {
                result.push(status);
                if result.len() > MAX_PENDING_RESERVATIONS {
                    return Err(CoreError::new(
                        "budget",
                        "Pending definition enumeration exceeds 128 entries",
                    ));
                }
            }
        }
        result.sort_by(|a, b| a.operation_id.cmp(&b.operation_id));
        Ok(result)
    }

    pub fn actor_save_with_definition_reservations(
        &mut self,
        resource: &str,
        root: &Path,
        request: HostedSaveRequest,
        proposal_ids: &[String],
    ) -> Result<ActorSaveStatus> {
        self.actor_save_definitions_internal(resource, Some(root), request, proposal_ids)
    }

    pub(super) fn capture_definition_save_reservations(
        &self,
        resource: &str,
        root: &Path,
        request: &HostedSaveRequest,
        ids: &[String],
    ) -> Result<Vec<SaveDefinitionReservation>> {
        self.check_session_base(&request.session_id, &request.base_id)?;
        let (owner, identity) = self.definition_owner(root, &request.session_id)?;
        let slots =
            weftext_knowledge::definition_slots(&request.source, &owner).unwrap_or_default();
        let mut reservations = vec![];
        for id in ids {
            let record = self.definition_record(resource, id)?.ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "The original definition reservation is unavailable",
                )
            })?;
            let cancelled = record.cancelled;
            let Some(proposal) = record.definition.unwrap().proposal else {
                continue;
            };
            if proposal.owner != owner || proposal.identity != identity {
                return Err(conflict());
            }
            let present = slots.iter().any(|slot| slot.key == proposal.key);
            if !present {
                continue;
            }
            let status = self
                .actor_definition_status(resource, id)?
                .ok_or_else(conflict)?;
            if status.status == "admitted" {
                continue;
            }
            if cancelled {
                return Err(CoreError::new(
                    "operationCancelled",
                    "The definition reservation was durably cancelled",
                ));
            }
            if status.status == "recoveryPending" {
                return Err(CoreError::new(
                    "recoveryPending",
                    "The original definition save requires recovery",
                ));
            }
            if weftext_engine::inspect_metadata(&request.source)
                .id
                .as_deref()
                != Some(&owner.node_id)
            {
                return Err(CoreError::new(
                    "identityConflict",
                    "The qualified afterimage must retain the reserved definition owner",
                ));
            }
            let definition = weftext_data::saved_queries(&request.source)
                .map_err(|_| {
                    CoreError::new(
                        "definitionQualification",
                        "The reserved carrier is unavailable",
                    )
                })?
                .into_iter()
                .find(|query| query.key == proposal.key)
                .ok_or_else(conflict)?;
            if serde_json::to_value(&definition.binding)?
                != serde_json::to_value(&proposal.binding)?
            {
                return Err(CoreError::new(
                    "schemaBasisChanged",
                    "First admission must preserve the original reserved schema binding",
                ));
            }
            // Undo may omit this key. Its identity remains reserved, and raw or
            // malformed native source can still use the original source save.
            if present {
                reservations.push(SaveDefinitionReservation {
                    proposal_id: id.clone(),
                    key: proposal.key,
                    owner: proposal.owner,
                    identity: proposal.identity,
                });
            }
        }
        Ok(reservations)
    }

    pub(super) fn freeze_definition_save_reservations(
        &self,
        request: &HostedSaveRequest,
        record: &ActorOperation,
    ) -> Result<()> {
        if record.save_definition_reservations.is_empty() {
            return Ok(());
        }
        let session = self.sessions.get(&request.session_id).ok_or_else(|| {
            CoreError::new(
                "sessionMissing",
                "Open the source again before an explicitly new save",
            )
        })?;
        for reservation in &record.save_definition_reservations {
            let original = self
                .definition_record(&record.resource, &reservation.proposal_id)?
                .ok_or_else(conflict)?;
            if original.cancelled {
                return Err(CoreError::new(
                    "operationCancelled",
                    "The original definition reservation was cancelled before saving",
                ));
            }
            let original = original.definition.unwrap().proposal.ok_or_else(conflict)?;
            if original.key != reservation.key
                || original.owner != reservation.owner
                || original.identity != reservation.identity
                || session.stamp.identity != reservation.identity
            {
                return Err(conflict());
            }
        }
        let fingerprint = hash(&serde_json::to_vec(&serde_json::json!({
            "kind":"save","session":request.session_id,"base":request.base_id,
            "path":display(&session.path),"source":request.source,
        }))?);
        let admissions = DefinitionSaveRecord {
            core_operation_id: record.core_operation_id.clone(),
            save_operation_id: request.operation_id.clone(),
            actor: record.actor.clone(),
            resource: record.resource.clone(),
            path: display(&session.path),
            save_fingerprint: fingerprint,
            source_hash: hash(request.source.as_bytes()),
            reservations: record.save_definition_reservations.clone(),
        };
        let path = self.journal_path(&record.core_operation_id, "definition-reservations");
        let bytes = serde_json::to_vec(&admissions)?;
        if let Some(original) = bounded_record(&self.backend, &path)? {
            if original != bytes {
                return Err(conflict());
            }
        } else {
            journal::private_create(&self.backend, &path, &bytes)?;
        }
        #[cfg(all(test, windows))]
        test_failpoint("admissionsDurable")?;
        Ok(())
    }

    fn definition_save_record(&self, plan: &journal::Plan) -> Result<Option<DefinitionSaveRecord>> {
        let Some(bytes) = bounded_record(
            &self.backend,
            &self.journal_path(&plan.operation_id, "definition-reservations"),
        )?
        else {
            return Ok(None);
        };
        let record: DefinitionSaveRecord = serde_json::from_slice(&bytes)?;
        if record.core_operation_id != plan.operation_id
            || record.path != plan.path
            || record.save_fingerprint != plan.fingerprint
            || record.source_hash != hash(&plan.after)
            || plan.kind != "save"
            || plan.transaction.is_some()
            || record.reservations.len() > MAX_SAVE_RESERVATIONS
        {
            return Err(CoreError::new(
                "privateEvidence",
                "Definition admission differs from its original source save",
            ));
        }
        for reservation in &record.reservations {
            if plan.identity.as_ref() != Some(&reservation.identity)
                || weftext_engine::inspect_metadata(&decode(&plan.after)?)
                    .id
                    .as_deref()
                    != Some(&reservation.owner.node_id)
                || !weftext_knowledge::definition_slots(&decode(&plan.after)?, &reservation.owner)
                    .is_ok_and(|slots| slots.iter().any(|slot| slot.key == reservation.key))
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Original owner afterimage has no qualified reserved key",
                ));
            }
        }
        Ok(Some(record))
    }

    pub(crate) fn hosted_definition_key_admitted(
        &self,
        plan: &journal::Plan,
        owner: &NodeRef,
        key: &str,
    ) -> Result<bool> {
        if !self.committed(&plan.operation_id)? {
            return Ok(false);
        }
        Ok(self.definition_save_record(plan)?.is_some_and(|record| {
            record
                .reservations
                .iter()
                .any(|r| &r.owner == owner && r.key == key)
        }))
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_definition_tests.rs"]
mod tests;
