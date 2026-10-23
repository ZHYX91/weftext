//! Actor-owned definition identities and immutable Draft proposals. Allocation
//! is not a writer: admission requires the original committed source save.
use super::*;
use std::collections::BTreeSet;
use weftext_data::{FieldPatch, SavedQueryBinding};
use weftext_knowledge::NodeRef;
use weftext_messages::UiMessage;

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
                UiMessage::error_core_test_interrupted(
                    weftext_messages::params::ErrorCoreTestInterrupted {
                        point: point.into(),
                    },
                ),
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

impl DefinitionOperation {
    pub(super) fn input_owner(&self) -> Option<&NodeRef> {
        self.proposal.as_ref().map(|proposal| &proposal.owner)
    }
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
#[derive(PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DefinitionSaveMetadata {
    core_operation_id: String,
    save_operation_id: String,
    actor: String,
    resource: String,
}
impl From<&DefinitionSaveRecord> for DefinitionSaveMetadata {
    fn from(record: &DefinitionSaveRecord) -> Self {
        Self {
            core_operation_id: record.core_operation_id.clone(),
            save_operation_id: record.save_operation_id.clone(),
            actor: record.actor.clone(),
            resource: record.resource.clone(),
        }
    }
}

fn conflict() -> CoreError {
    CoreError::new(
        "operationConflict",
        UiMessage::error_core_hosted_definition_original_definition_actor_owner_or_intent_differs(),
    )
}
fn valid_resource(resource: &str) -> Result<()> {
    if resource.is_empty() || resource.len() > 256 || resource.chars().any(char::is_control) {
        return Err(CoreError::new(
            "invalidResource",
            UiMessage::error_core_hosted_definition_bounded_resource_identity_required(),
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
            UiMessage::error_core_hosted_definition_lowercase_s_h_a256_author_intent_fingerprint_required(),
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

fn bounded_record(
    backend: &crate::BackendContext,
    path: &Path,
    accept: impl FnOnce(&DefinitionSaveMetadata) -> Result<bool>,
) -> Result<Option<(DefinitionSaveRecord, Vec<u8>)>> {
    let guard = match ParentGuard::acquire(backend, path) {
        Ok(guard) => guard,
        Err(error) if error.code == "missing" => return Ok(None),
        Err(error) => return Err(error),
    };
    let mut file = match guard.open_read_guard_file() {
        Ok(file) => file,
        Err(error) if error.code == "missing" => return Ok(None),
        Err(error) => return Err(error),
    };
    let Some((metadata, bytes)) = read_qualified_private_record(&mut file, |file| {
        let fields = read_metadata_prefix(
            file,
            &[
                ("coreOperationId", false),
                ("saveOperationId", false),
                ("actor", false),
                ("resource", false),
            ],
            4096,
        )?
        .ok_or_else(|| {
            CoreError::new(
                "privateEvidence",
                weftext_messages::UiMessage::error_core_hosted_definition_definition_admission_metadata_is_missing(),
            )
        })?;
        let metadata: DefinitionSaveMetadata = decode_metadata(fields)?;
        valid_operation_id(&metadata.core_operation_id)?;
        valid_operation_id(&metadata.save_operation_id)?;
        valid_resource(&metadata.resource)?;
        if metadata.actor.is_empty() || metadata.actor.len() > 256 {
            return Err(CoreError::new(
                "privateEvidence",
                weftext_messages::UiMessage::error_core_hosted_definition_invalid_definition_admission_actor(),
            ));
        }
        if !accept(&metadata)? {
            return Ok(None);
        }
        Ok(Some((metadata, MAX_RECORD_BYTES)))
    })?
    else {
        return Ok(None);
    };
    let record: DefinitionSaveRecord = serde_json::from_slice(&bytes)?;
    if DefinitionSaveMetadata::from(&record) != metadata {
        return Err(CoreError::new(
            "privateEvidence",
            weftext_messages::UiMessage::error_core_hosted_definition_definition_admission_metadata_changed_during_its_qualified_read(),
        ));
    }
    Ok(Some((record, bytes)))
}

pub(super) fn canonical_proposal_ids(ids: &[String]) -> Result<Vec<String>> {
    if ids.len() > MAX_SAVE_RESERVATIONS {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_hosted_definition_at_most32_definition_reservations_per_source_save(),
        ));
    }
    for id in ids {
        valid_operation_id(id)?;
        if Uuid::parse_str(id).unwrap().to_string() != *id {
            return Err(CoreError::new(
                "invalidOperationId",
                UiMessage::error_core_hosted_definition_canonical_proposal_u_u_i_d_required(),
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
        let Some(record) = self.read_actor_operation(operation, resource, MAX_RECORD_BYTES)? else {
            return Ok(None);
        };
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
                UiMessage::error_core_hosted_definition_definitions_require_admitted_canonical_owner_source(),
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
                UiMessage::error_core_hosted_definition_initial_definition_schema_role_or_language_differs(),
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
                UiMessage::error_core_hosted_definition_at_most128_pending_definition_reservations_per_owner(),
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
                UiMessage::error_core_hosted_definition_exact_draft_patch_lacks_qualified_owner_key_binding(),
            ));
        }
        let fingerprint = hash(&serde_json::to_vec(&serde_json::json!({
            "kind":"definitionReservation","actor":self.actor,"resource":resource,
            "root":display(&absolute_path(root)?),"owner":owner,"identity":identity,
            "session":request.session_id,"base":request.base_id,
            "intent":request.intent_fingerprint,"binding":request.binding,
        }))?);
        let record = ActorOperation {
            scope: ActorInputScope::Qualified {
                scope: self.draft_session_scope(&request.session_id)?,
            },
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
        let bytes = self.actor_operation_bytes(&record)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_definition_definition_proposal_exceeds_durable_budget(
                ),
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
                let ActorInputScope::Qualified { scope } = &record.scope else {
                    return Err(conflict());
                };
                let Some(admissions) = self.definition_save_record(
                    plan,
                    Some(scope),
                    Some((&record.actor, resource)),
                )?
                else {
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
                scope: self.actor_attempt_input_scope()?,
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
            private_replace(&self.backend, &path, &self.actor_operation_bytes(&record)?)?;
        } else {
            journal::private_create(&self.backend, &path, &self.actor_operation_bytes(&record)?)?;
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
            let Some((record, count)) = self.read_actor_operation_counted(
                operation,
                resource,
                (16 * 1024 * 1024).min(MAX_LIST_BYTES - bytes_used),
                true,
            )?
            else {
                continue;
            };
            bytes_used = bytes_used.checked_add(count).ok_or_else(conflict)?;
            if bytes_used > MAX_LIST_BYTES {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_hosted_definition_definition_enumeration_exceeds64_mi_b(),
                ));
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
                        UiMessage::error_core_hosted_definition_pending_definition_enumeration_exceeds128_entries(),
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
                    UiMessage::error_core_hosted_definition_original_definition_reservation_unavailable(),
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
                    UiMessage::error_core_hosted_definition_definition_reservation_was_durably_cancelled(),
                ));
            }
            if status.status == "recoveryPending" {
                return Err(CoreError::new(
                    "recoveryPending",
                    UiMessage::error_core_hosted_definition_original_definition_save_requires_recovery(),
                ));
            }
            if weftext_engine::inspect_metadata(&request.source)
                .id
                .as_deref()
                != Some(&owner.node_id)
            {
                return Err(CoreError::new(
                    "identityConflict",
                    UiMessage::error_core_hosted_definition_qualified_afterimage_must_retain_reserved_definition_owner(),
                ));
            }
            let definition = weftext_data::saved_queries(&request.source)
                .map_err(|_| {
                    CoreError::new(
                        "definitionQualification",
                        UiMessage::error_core_hosted_definition_reserved_carrier_unavailable(),
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
                    UiMessage::error_core_hosted_definition_first_admission_must_preserve_original_reserved_schema_binding(),
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
                UiMessage::error_core_hosted_definition_open_source_again_before_explicitly_new_save(),
            )
        })?;
        for reservation in &record.save_definition_reservations {
            let original = self
                .definition_record(&record.resource, &reservation.proposal_id)?
                .ok_or_else(conflict)?;
            if original.cancelled {
                return Err(CoreError::new(
                    "operationCancelled",
                    UiMessage::error_core_hosted_definition_original_definition_reservation_was_cancelled_before_saving(),
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
        let ActorInputScope::Qualified { scope } = &record.scope else {
            return Err(conflict());
        };
        let data = file_save_scope::FileSaveData::new(
            request.session_id.clone(),
            Some(request.base_id.clone()),
            scope.clone(),
            self.operations
                .get(&request.operation_id)
                .and_then(|plan| plan.file_save.as_ref())
                .map_or_else(
                    || session.draft_producer_id.clone(),
                    |data| data.draft_producer_id.clone(),
                ),
        );
        let fingerprint =
            file_save_scope::fingerprint("save", &session.path, &request.source, &data, None)?;
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
        let expected = DefinitionSaveMetadata::from(&admissions);
        if let Some((_, original)) = bounded_record(&self.backend, &path, |metadata| {
            if *metadata != expected {
                return Err(conflict());
            }
            Ok(true)
        })? {
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

    fn definition_save_record(
        &self,
        plan: &journal::Plan,
        scope: Option<&QualifiedInputScope>,
        audience: Option<(&str, &str)>,
    ) -> Result<Option<DefinitionSaveRecord>> {
        let path = self.journal_path(&plan.operation_id, "definition-reservations");
        if plan.kind != "save" || plan.transaction.is_some() {
            if path.try_exists()? {
                return Err(CoreError::new(
                    "privateEvidence",
                    weftext_messages::UiMessage::error_core_hosted_definition_definition_admission_references_another_plan_kind(),
                ));
            }
            return Ok(None);
        }
        self.validate_file_save_plan(plan)?;
        if scope.is_some_and(|scope| scope != &plan.file_save.as_ref().unwrap().scope) {
            return Ok(None);
        }
        let Some((record, _)) = bounded_record(&self.backend, &path, |metadata| {
            if metadata.core_operation_id != plan.operation_id {
                return Err(conflict());
            }
            Ok(audience.is_none_or(|(actor, resource)| {
                metadata.actor == actor && metadata.resource == resource
            }))
        })?
        else {
            return Ok(None);
        };
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
                UiMessage::error_core_hosted_definition_definition_admission_differs_from_original_source_save(),
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
                    UiMessage::error_core_hosted_definition_original_owner_afterimage_has_no_qualified_reserved_key(),
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
        if plan.kind != "save" {
            return Ok(false);
        }
        self.validate_file_save_plan(plan)?;
        if !matches!(&plan.file_save.as_ref().unwrap().scope, QualifiedInputScope::Library { binding } if binding.workspace_id == owner.workspace_id)
        {
            return Ok(false);
        }
        Ok(self
            .definition_save_record(plan, None, None)?
            .is_some_and(|record| {
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
