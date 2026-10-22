//! Hosted requests use the same guarded writer, with actor-owned draft and
//! operation records. This module grants no permissions: the server authorizes
//! configured resources before calling it, and again before disclosing results.
use super::*;
use std::collections::BTreeMap;
use weftext_messages::UiMessage;
#[path = "hosted_library.rs"]
mod library;
pub(crate) use library::{
    ActorLibraryBinding, ActorLibraryHeader, read_actor_library_header, read_first_metadata,
};
#[path = "hosted_backup.rs"]
mod backup;
pub use backup::{ActorOwnerBackupStatus, OwnerBackupArtifact};
#[path = "hosted_content_import.rs"]
mod content_import;
pub use content_import::{ActorContentImportReceipt, ContentImportMember};
#[path = "hosted_feature.rs"]
mod feature;
pub use feature::{
    ActorFeatureStatus, ActorStructureNode, ActorStructureReceipt, FeatureTreeWitness,
    HostedNavigationIntent,
};
#[path = "hosted_definition.rs"]
mod definition;
#[cfg(all(test, windows))]
pub(crate) use definition::set_test_failpoint;
#[cfg(all(test, windows))]
pub(crate) use definition::test_failpoint as definition_test_failpoint;
pub use definition::{
    ActorDefinitionProposal, ActorDefinitionStatus, HostedDefinitionReservationRequest,
};
#[path = "hosted_history.rs"]
mod actor_history;
pub use actor_history::ActorHistoryRestoreStatus;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostedSaveRequest {
    pub session_id: String,
    pub base_id: String,
    pub source: String,
    pub operation_id: String,
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn hosted_schema_guard_requires_admitted_file_and_blocks_external_write_during_decision() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("library");
        let mut core = Core::new(temp.path().join("state")).unwrap();
        core.create_library(&root).unwrap();
        let missing = core.read_config(&root, ConfigRole::Schema).unwrap();
        assert!(
            core.with_hosted_schema(&root, &missing, |_| Ok(()))
                .is_err()
        );
        let bytes = include_bytes!("../../../docs/design/examples/schema.authoring.json").to_vec();
        let operation = fresh();
        let plan = core
            .prepare_config_save(&root, ConfigRole::Schema, None, bytes.clone(), &operation)
            .unwrap();
        core.execute_operation(&operation, &plan.plan_id).unwrap();
        let installed = core.read_config(&root, ConfigRole::Schema).unwrap();
        let path = root.join(".weftext-schema.json");
        core.with_hosted_schema(&root, &installed, |_| {
            assert!(fs::write(&path, b"changed").is_err());
            Ok(())
        })
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(&path, b"external").unwrap();
        assert!(
            core.with_hosted_schema(&root, &installed, |_| Ok(()))
                .is_err()
        );
        fs::write(&path, bytes).unwrap();
        assert!(
            core.with_hosted_schema(&root, &installed, |_| Ok(()))
                .is_err()
        );
    }

    #[test]
    fn actor_drafts_are_separate_and_scope_is_restored_after_error() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "base").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let alice = core
            .with_actor("space:alice", |c| c.open(&path, None))
            .unwrap();
        core.with_actor("space:alice", |c| {
            c.persist_draft(&alice.session_id, "alice private")
        })
        .unwrap();
        let bob = core
            .with_actor("space:bob", |c| c.open(&path, None))
            .unwrap();
        assert!(bob.recovered_draft.is_none());
        assert!(
            core.with_actor("space:bob", |c| c
                .persist_draft(&alice.session_id, "hijack"))
                .is_err()
        );
        assert!(core.actor.is_none());
        assert!(core.open(&path, None).unwrap().recovered_draft.is_none());
        assert_eq!(
            core.with_actor("space:alice", |c| c.open(&path, None))
                .unwrap()
                .recovered_draft
                .as_deref(),
            Some("alice private")
        );
    }
    #[test]
    fn committed_actor_operation_is_queryable_after_restart_only_in_its_scope() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        let data = temp.path().join("state");
        fs::write(&path, "base").unwrap();
        let id = fresh();
        {
            let mut core = Core::new(&data).unwrap();
            let doc = core
                .with_actor("space:alice", |c| c.open(&path, None))
                .unwrap();
            let result = core
                .with_actor("space:alice", |c| {
                    c.actor_save(
                        "note",
                        HostedSaveRequest {
                            session_id: doc.session_id,
                            base_id: doc.base_id,
                            source: "after".into(),
                            operation_id: id.clone(),
                        },
                    )
                })
                .unwrap();
            assert_eq!(result.status, "committed");
        }
        let mut core = Core::new(&data).unwrap();
        assert_eq!(
            core.with_actor("space:alice", |c| c.actor_save_status(&id))
                .unwrap()
                .unwrap()
                .status,
            "committed"
        );
        assert!(
            core.with_actor("space:bob", |c| c.actor_save_status(&id))
                .unwrap()
                .is_none()
        );
        assert_eq!(fs::read_to_string(&path).unwrap(), "after");
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorSaveStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub base_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_issue: Option<RecoveryIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ActorInputScope {
    NoInput,
    Qualified { scope: QualifiedInputScope },
}
pub(crate) fn read_actor_input_header(
    file: &mut (impl Read + std::io::Seek),
) -> Result<ActorInputScope> {
    let value = read_first_metadata(file, "scope")?.ok_or_else(|| {
        CoreError::new(
            "unsupportedInputScope",
            UiMessage::error_core_original_scope_actor_unsupported_original_scope(),
        )
    })?;
    let scope: ActorInputScope = serde_json::from_slice(&value)?;
    if let ActorInputScope::Qualified { scope } = &scope {
        scope.validate()?;
    }
    Ok(scope)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActorOperation {
    scope: ActorInputScope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<ActorLibraryBinding>,
    actor: String,
    resource: String,
    fingerprint: String,
    core_operation_id: String,
    #[serde(default)]
    cancelled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    field: Option<FieldOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    feature: Option<feature::FeatureOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    definition: Option<definition::DefinitionOperation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    save_definition_reservations: Vec<definition::SaveDefinitionReservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    history: Option<actor_history::HistoryOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    owner_backup: Option<backup::OwnerBackupOperation>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FieldOperation {
    field_id: String,
    cell: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorFieldStatus {
    pub operation_id: String,
    pub resource: String,
    pub field_id: String,
    pub status: String,
    pub cell: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_issue: Option<RecoveryIssue>,
}

impl Core {
    /// Keep the installed, admitted schema behind a share-zero handle through
    /// the field writer's decision. A missing/bundled default cannot provide a
    /// protected filesystem absence witness, so it remains read-only here.
    pub fn with_hosted_schema<T>(
        &mut self,
        root: &Path,
        expected: &ConfigDocument,
        action: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        let root = absolute_path(root)?;
        self.require_workspace(&root)?;
        let path = root.join(".weftext-schema.json");
        let guard = ParentGuard::acquire(&path)?;
        let mut file = guard.open_file(false, false)?;
        let bytes = read_source(&mut file)?;
        let captured = operations::CapturedFile {
            path: display(&path),
            bytes,
            stamp: stamp(&file)?,
        };
        if expected.path != captured.path
            || expected.bytes.as_ref() != Some(&captured.bytes)
            || expected.revision.is_none()
            || self.accepted_revision(&captured) != expected.revision
        {
            return Err(CoreError::new(
                "schemaBasisChanged",
                UiMessage::error_core_hosted_installed_schema_witness_unavailable(),
            ));
        }
        action(self)
    }

    pub fn with_actor<T>(
        &mut self,
        actor: &str,
        action: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if actor.is_empty() || actor.len() > 256 || self.actor.is_some() {
            return Err(CoreError::new(
                "invalidActor",
                UiMessage::error_core_hosted_non_nested_authenticated_actor_scope_required(),
            ));
        }
        let root = self.data_dir.join("actors").join(hash(actor.as_bytes()));
        for directory in [
            "drafts",
            "draft-history",
            "operations",
            capture_draft::CAPTURE_DIRECTORY,
        ] {
            fs::create_dir_all(root.join(directory))?;
        }
        let _guards = [
            "drafts",
            "draft-history",
            "operations",
            capture_draft::CAPTURE_DIRECTORY,
        ]
        .map(|directory| ParentGuard::acquire(&root.join(directory).join(".scope")))
        .into_iter()
        .collect::<Result<Vec<_>>>()?;
        let staging = self
            .data_dir
            .join(capture_draft::CAPTURE_STAGING_DIRECTORY)
            .join(hash(actor.as_bytes()));
        fs::create_dir_all(&staging)?;
        let _staging_guard = ParentGuard::acquire(&staging.join(".capture-staging-scope"))?;
        self.actor = Some(actor.into());
        let result = action(self);
        self.actor = None;
        result
    }

    pub(crate) fn draft_directory(&self, name: &str) -> PathBuf {
        match &self.actor {
            Some(actor) => self
                .data_dir
                .join("actors")
                .join(hash(actor.as_bytes()))
                .join(name),
            None => self.data_dir.join(name),
        }
    }

    pub(crate) fn check_actor_session(&self, session_id: &str) -> Result<()> {
        if self
            .sessions
            .get(session_id)
            .is_none_or(|session| session.actor != self.actor)
        {
            return Err(CoreError::new(
                "sessionMissing",
                UiMessage::error_core_hosted_open_source_in_current_actor_session(),
            ));
        }
        self.actor_library_for_session(session_id)?;
        Ok(())
    }

    pub fn close_session(&mut self, session_id: &str) -> Result<()> {
        self.check_actor_session(session_id)?;
        self.sessions.remove(session_id);
        Ok(())
    }

    /// Service-owned policy survives restarts; bootstrap configuration cannot
    /// silently restore a revoked grant. No client chooses this filesystem path.
    pub fn hosted_policy(&self) -> Result<Option<Vec<u8>>> {
        match fs::read(self.data_dir.join("hosted-policy.json")) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn store_hosted_policy(&self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > MAX_SOURCE_BYTES {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_policy_exceeds2_mi_b(),
            ));
        }
        private_replace(&self.data_dir.join("hosted-policy.json"), bytes)
    }

    pub(super) fn actor_input_scope_for_root(&self, root: &Path) -> Result<ActorInputScope> {
        Ok(ActorInputScope::Qualified {
            scope: self.capture_file_save_scope(&root.join(".actor-input-prospective"))?,
        })
    }
    pub(super) fn actor_attempt_input_scope(&self) -> Result<ActorInputScope> {
        self.actor_library_path
            .as_ref()
            .map_or(Ok(ActorInputScope::NoInput), |root| {
                self.actor_input_scope_for_root(root)
            })
    }
    fn validate_actor_input_record(&self, record: &ActorOperation) -> Result<()> {
        Self::validate_actor_input_shape(record)?;
        match &record.scope {
            ActorInputScope::NoInput => {
                if self.operations.contains_key(&record.core_operation_id) {
                    return Err(CoreError::new(
                        "privateEvidence",
                        UiMessage::error_core_original_scope_actor_no_input_references_plan(),
                    ));
                }
                self.validate_unregistered_fence_namespace(&record.core_operation_id)?;
            }
            ActorInputScope::Qualified { scope } => {
                if (record
                    .feature
                    .as_ref()
                    .is_some_and(|feature| !feature.is_empty_fence())
                    || record.owner_backup.is_some())
                    && !matches!(scope, QualifiedInputScope::Library { .. })
                {
                    return Err(CoreError::new(
                        "privateEvidence",
                        UiMessage::error_core_original_scope_actor_qualified_library_required(),
                    ));
                }
                self.validate_original_input_scope(scope)?;
                for owner in record
                    .definition
                    .as_ref()
                    .and_then(|definition| definition.input_owner())
                    .into_iter()
                    .chain(
                        record
                            .history
                            .as_ref()
                            .and_then(|history| history.input_owner()),
                    )
                {
                    if !matches!(scope, QualifiedInputScope::Library { binding } if binding.workspace_id == owner.workspace_id)
                    {
                        return Err(CoreError::new(
                            "privateEvidence",
                            UiMessage::error_core_original_scope_actor_proposal_owner_differs(),
                        ));
                    }
                }
                if let Some(plan) = self.operations.get(&record.core_operation_id) {
                    if matches!(plan.kind.as_str(), "save" | "saveAs") {
                        self.validate_file_save_plan(plan)?;
                        if &plan.file_save.as_ref().unwrap().scope != scope {
                            return Err(CoreError::new(
                                "operationConflict",
                                UiMessage::error_core_original_scope_actor_file_save_scope_differs(
                                ),
                            ));
                        }
                    } else if let Some(workspace) = &plan.workspace_id
                        && !matches!(scope, QualifiedInputScope::Library { binding } if &binding.workspace_id == workspace)
                    {
                        return Err(CoreError::new(
                            "privateEvidence",
                            UiMessage::error_core_original_scope_actor_transaction_owner_differs(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    fn validate_actor_input_shape(record: &ActorOperation) -> Result<()> {
        match &record.scope {
            ActorInputScope::NoInput => {
                let empty = record.cancelled
                    && record.fingerprint.is_empty()
                    && record.library.is_none()
                    && record.save_definition_reservations.is_empty()
                    && record
                        .field
                        .as_ref()
                        .is_none_or(|field| field.cell.is_null())
                    && record
                        .feature
                        .as_ref()
                        .is_none_or(|feature| feature.is_empty_fence())
                    && record
                        .definition
                        .as_ref()
                        .is_none_or(|definition| definition.input_owner().is_none())
                    && record
                        .history
                        .as_ref()
                        .is_none_or(|history| history.input_owner().is_none())
                    && record.owner_backup.is_none();
                if !empty {
                    return Err(CoreError::new(
                        "unsupportedInputScope",
                        UiMessage::error_core_original_scope_actor_no_input_requires_empty_cancelled_fence(),
                    ));
                }
            }
            ActorInputScope::Qualified { scope } => scope.validate()?,
        }
        Ok(())
    }
    pub(crate) fn validate_no_input_actor_fence(&self, path: &Path, bytes: &[u8]) -> Result<()> {
        if bytes.len() > 4096 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_original_scope_actor_no_input_metadata_budget(),
            ));
        }
        let record: ActorOperation = serde_json::from_slice(bytes)?;
        if record.scope != ActorInputScope::NoInput {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_actor_fence_scope_changed(),
            ));
        }
        self.validate_actor_input_record(&record)?;
        valid_operation_id(&record.core_operation_id)?;
        feature::valid_resource(&record.resource)?;
        let relative = history::history_relative_path(path, &self.data_dir.join("actors"))
            .ok_or_else(|| {
                CoreError::new(
                    "privateEvidence",
                    UiMessage::error_core_original_scope_actor_fence_outside_namespace(),
                )
            })?;
        let parts = relative
            .components()
            .map(|part| part.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                CoreError::new(
                    "privateEvidence",
                    UiMessage::error_core_original_scope_actor_fence_namespace_invalid(),
                )
            })?;
        if parts.len() != 3
            || parts[0] != hash(record.actor.as_bytes())
            || parts[1] != "operations"
            || record.actor.is_empty()
            || record.actor.len() > 256
        {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_actor_fence_namespace_differs(),
            ));
        }
        let leaf = parts[2].strip_suffix(".json").ok_or_else(|| {
            CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_actor_fence_leaf_invalid(),
            )
        })?;
        valid_operation_id(leaf)?;
        Ok(())
    }
    fn validate_actor_file_input(&self, record: &ActorOperation) -> Result<()> {
        let ActorInputScope::Qualified { scope } = &record.scope else {
            if record.cancelled && record.fingerprint.is_empty() {
                return Ok(());
            }
            return Err(CoreError::new(
                "unsupportedInputScope",
                UiMessage::error_core_original_scope_actor_original_owner_unqualified(),
            ));
        };
        scope.validate()?;
        if let Some(plan) = self.operations.get(&record.core_operation_id) {
            self.validate_file_save_plan(plan)?;
            if &plan.file_save.as_ref().unwrap().scope != scope {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_original_scope_actor_file_save_scope_differs(),
                ));
            }
        }
        Ok(())
    }
    fn actor_record(&self, operation: &str) -> Result<PathBuf> {
        valid_operation_id(operation)?;
        if self.actor.is_none() {
            return Err(CoreError::new(
                "invalidActor",
                UiMessage::error_core_hosted_authenticated_actor_required(),
            ));
        }
        Ok(self
            .draft_directory("operations")
            .join(format!("{operation}.json")))
    }
    pub fn actor_save_status(&self, operation: &str) -> Result<Option<ActorSaveStatus>> {
        let path = self.actor_record(operation)?;
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let record: ActorOperation = serde_json::from_slice(&bytes)?;
        self.validate_actor_input_record(&record)?;
        self.check_actor_library(&record.library)?;
        if record.feature.is_some()
            || record.definition.is_some()
            || record.owner_backup.is_some()
            || record.history.is_some()
            || record.owner_backup.is_some()
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_hosted_operation_belongs_to_feature_or_definition_proposal(),
            ));
        }
        if self.actor.as_ref() != Some(&record.actor) {
            return Err(CoreError::new(
                "invalidActor",
                UiMessage::error_core_hosted_operation_belongs_to_different_actor(),
            ));
        }
        if self
            .operations
            .get(&record.core_operation_id)
            .is_some_and(|plan| plan.kind != "save")
        {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_hosted_scoped_source_save_references_different_operation_kind(
                ),
            ));
        }
        self.validate_actor_file_input(&record)?;
        let saved = self.file_save_fact(&record.core_operation_id)?;
        Ok(Some(ActorSaveStatus {
            operation_id: operation.into(),
            resource: record.resource,
            status: if record.cancelled {
                "cancelled"
            } else {
                if saved.is_some() {
                    "committed"
                } else if self.resolved(&record.core_operation_id)? {
                    "resolved"
                } else if self.operations.contains_key(&record.core_operation_id) {
                    "recoveryPending"
                } else {
                    "notCommitted"
                }
            }
            .into(),
            base_id: saved.map(|r| r.base_id),
            recovery_issue: None,
        }))
    }
    /// Fence a request whose response was lost before allowing a new proposal.
    /// The durable tombstone also prevents a delayed original request from
    /// committing after an earlier status lookup said notRegistered.
    pub fn actor_settle(&mut self, resource: &str, operation: &str) -> Result<ActorSaveStatus> {
        let path = self.actor_record(operation)?;
        let mut record = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<ActorOperation>(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => ActorOperation {
                scope: self.actor_attempt_input_scope()?,
                library: self.actor_library.clone(),
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: false,
                field: None,
                feature: None,
                definition: None,
                save_definition_reservations: vec![],
                history: None,
                owner_backup: None,
            },
            Err(e) => return Err(e.into()),
        };
        if record.resource != resource || self.actor.as_ref() != Some(&record.actor) {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_hosted_operation_resource_or_actor_differs(),
            ));
        }
        if let Some(status) = self.actor_save_status(operation)?
            && status.status != "notCommitted"
        {
            return Ok(status);
        }
        record.cancelled = true;
        private_replace(&path, &self.actor_operation_bytes(&record)?)?;
        self.actor_save_status(operation)?.ok_or_else(|| {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_hosted_missing_settled_record(),
            )
        })
    }
    pub fn actor_save(
        &mut self,
        resource: &str,
        request: HostedSaveRequest,
    ) -> Result<ActorSaveStatus> {
        self.actor_save_definitions_internal(resource, None, request, &[])
    }
    pub(super) fn actor_save_definitions_internal(
        &mut self,
        resource: &str,
        root: Option<&Path>,
        request: HostedSaveRequest,
        proposal_ids: &[String],
    ) -> Result<ActorSaveStatus> {
        let path = self.actor_record(&request.operation_id)?;
        check_size(request.source.as_bytes())?;
        let ids = definition::canonical_proposal_ids(proposal_ids)?;
        let mut intent = serde_json::json!({
            "resource":resource,"session":request.session_id,"base":request.base_id,"source":request.source
        });
        if !ids.is_empty() {
            intent["definitionReservations"] = serde_json::to_value(&ids)?;
        }
        let fingerprint = hash(&serde_json::to_vec(&intent)?);
        let record = match fs::read(&path) {
            Ok(bytes) => {
                let record: ActorOperation = serde_json::from_slice(&bytes)?;
                self.validate_actor_input_record(&record)?;
                self.check_actor_library(&record.library)?;
                if record.actor != *self.actor.as_ref().unwrap()
                    || record.resource != resource
                    || record.field.is_some()
                    || record.feature.is_some()
                    || record.definition.is_some()
                    || record.owner_backup.is_some()
                    || record.history.is_some()
                    || record.owner_backup.is_some()
                {
                    return Err(CoreError::new(
                        "operationConflict",
                        UiMessage::error_core_hosted_original_actor_save_differs(),
                    ));
                }
                if record.cancelled {
                    return Err(CoreError::new(
                        "operationCancelled",
                        UiMessage::error_core_hosted_original_request_was_durably_cancelled_use_explicitly_new_proposal(),
                    ));
                }
                if record.fingerprint != fingerprint {
                    return Err(CoreError::new(
                        "operationConflict",
                        UiMessage::error_core_hosted_operation_already_names_different_proposal(),
                    ));
                }
                if let Some(status) = self.actor_save_status(&request.operation_id)?
                    && status.status != "notCommitted"
                {
                    return Ok(status);
                }
                record
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                self.check_actor_session(&request.session_id)?;
                let reservations = if ids.is_empty() {
                    vec![]
                } else {
                    self.capture_definition_save_reservations(
                        resource,
                        root.ok_or_else(|| {
                            CoreError::new(
                                "basisRequired",
                                UiMessage::error_core_hosted_definition_owner_library_required(),
                            )
                        })?,
                        &request,
                        &ids,
                    )?
                };
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
                    definition: None,
                    save_definition_reservations: reservations,
                    history: None,
                    owner_backup: None,
                };
                journal::private_create(&path, &self.actor_operation_bytes(&record)?)?;
                record
            }
            Err(e) => return Err(e.into()),
        };
        #[cfg(all(test, windows))]
        definition_test_failpoint("actorSaveDurable")?;
        self.freeze_definition_save_reservations(&request, &record)?;
        let outcome = self.save(SaveRequest {
            session_id: request.session_id,
            base_id: request.base_id,
            source: request.source,
            operation_id: record.core_operation_id,
        })?;
        let mut status = self
            .actor_save_status(&request.operation_id)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_original_operation_record_missing(),
                )
            })?;
        status.recovery_issue = outcome.recovery_issue;
        Ok(status)
    }

    /// A narrow receipt discloses neither the internal source/base nor another
    /// field's operation. Callers authorize the named field before this lookup.
    pub fn actor_field_status(
        &self,
        resource: &str,
        field_id: &str,
        operation: &str,
    ) -> Result<Option<ActorFieldStatus>> {
        let bytes = match fs::read(self.actor_record(operation)?) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let record: ActorOperation = serde_json::from_slice(&bytes)?;
        self.validate_actor_input_record(&record)?;
        self.check_actor_library(&record.library)?;
        let field = record
            .field
            .filter(|f| f.field_id == field_id)
            .filter(|_| record.resource == resource)
            .ok_or_else(|| {
                CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_hosted_operation_field_or_resource_differs(),
                )
            })?;
        let status = self.actor_save_status(operation)?.ok_or_else(|| {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_hosted_missing_original_operation(),
            )
        })?;
        Ok(Some(ActorFieldStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            field_id: field_id.into(),
            cell: (status.status == "committed").then_some(field.cell),
            status: status.status,
            recovery_issue: status.recovery_issue,
        }))
    }

    pub fn actor_field_retry(
        &self,
        resource: &str,
        field_id: &str,
        operation: &str,
        fingerprint: &str,
    ) -> Result<Option<ActorFieldStatus>> {
        let status = self.actor_field_status(resource, field_id, operation)?;
        if status.is_some() {
            let record: ActorOperation =
                serde_json::from_slice(&fs::read(self.actor_record(operation)?)?)?;
            if !record.cancelled && record.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_hosted_original_field_intent_differs(),
                ));
            }
        }
        Ok(status)
    }

    /// Durable intent binding precedes the exact Core source save. A retry
    /// returns the original status, including notCommitted; it never derives a
    /// second source proposal from a newer field profile or ephemeral session.
    pub fn actor_field_save(
        &mut self,
        resource: &str,
        field_id: &str,
        fingerprint: &str,
        cell: serde_json::Value,
        request: HostedSaveRequest,
    ) -> Result<ActorFieldStatus> {
        check_size(request.source.as_bytes())?;
        check_size(&serde_json::to_vec(&cell)?)?;
        if field_id.is_empty() || field_id.len() > 256 || fingerprint.len() != 64 {
            return Err(CoreError::new(
                "invalidField",
                UiMessage::error_core_hosted_invalid_field_intent(),
            ));
        }
        let path = self.actor_record(&request.operation_id)?;
        match fs::read(&path) {
            Ok(bytes) => {
                let record: ActorOperation = serde_json::from_slice(&bytes)?;
                self.validate_actor_input_record(&record)?;
                self.check_actor_library(&record.library)?;
                let status = self
                    .actor_field_status(resource, field_id, &request.operation_id)?
                    .ok_or_else(|| {
                        CoreError::new(
                            "invalidState",
                            UiMessage::error_core_hosted_missing_original_field_operation(),
                        )
                    })?;
                if !record.cancelled && record.fingerprint != fingerprint {
                    return Err(CoreError::new(
                        "operationConflict",
                        UiMessage::error_core_hosted_original_field_intent_differs(),
                    ));
                }
                return Ok(status);
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        self.check_actor_session(&request.session_id)?;
        let record = ActorOperation {
            scope: ActorInputScope::Qualified {
                scope: self.draft_session_scope(&request.session_id)?,
            },
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint.into(),
            core_operation_id: fresh(),
            cancelled: false,
            field: Some(FieldOperation {
                field_id: field_id.into(),
                cell,
            }),
            feature: None,
            definition: None,
            save_definition_reservations: vec![],
            history: None,
            owner_backup: None,
        };
        journal::private_create(&path, &self.actor_operation_bytes(&record)?)?;
        let outcome = self.save(SaveRequest {
            session_id: request.session_id,
            base_id: request.base_id,
            source: request.source,
            operation_id: record.core_operation_id,
        })?;
        let mut status = self
            .actor_field_status(resource, field_id, &request.operation_id)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_missing_original_field_operation(),
                )
            })?;
        status.recovery_issue = outcome.recovery_issue;
        Ok(status)
    }

    pub fn actor_field_settle(
        &mut self,
        resource: &str,
        field_id: &str,
        operation: &str,
    ) -> Result<ActorFieldStatus> {
        if self
            .actor_field_status(resource, field_id, operation)?
            .is_none()
        {
            let record = ActorOperation {
                scope: self.actor_attempt_input_scope()?,
                library: self.actor_library.clone(),
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: true,
                field: Some(FieldOperation {
                    field_id: field_id.into(),
                    cell: serde_json::Value::Null,
                }),
                feature: None,
                definition: None,
                save_definition_reservations: vec![],
                history: None,
                owner_backup: None,
            };
            journal::private_create(
                &self.actor_record(operation)?,
                &self.actor_operation_bytes(&record)?,
            )?;
        } else {
            self.actor_settle(resource, operation)?;
        }
        self.actor_field_status(resource, field_id, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_missing_settled_field_operation(),
                )
            })
    }

    /// Explicit allowlisted file set; no ambient tree enumeration or fallback.
    /// All handles stay guarded until the complete bounded snapshot is captured.
    pub fn hosted_files(&self, root: &Path, paths: &[String]) -> Result<BTreeMap<String, Vec<u8>>> {
        let root = absolute_path(root)?;
        if paths.len() > 64 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_at_most64_explicit_inputs(),
            ));
        }
        let mut guards = Vec::new();
        let mut handles = Vec::new();
        for relative in paths {
            let relative_path = Path::new(relative);
            if relative_path
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
                || relative.is_empty()
            {
                return Err(CoreError::new(
                    "outsideRoot",
                    UiMessage::error_core_hosted_explicit_inputs_must_relative_child_paths(),
                ));
            }
            let path = root.join(relative_path);
            self.check_pending(&path)?;
            let guard = ParentGuard::acquire(&path)?;
            let file = guard.open_file(false, false)?;
            self.check_pending_identity(&stamp(&file)?.identity)?;
            guards.push(guard);
            handles.push((relative, file));
        }
        let mut files = BTreeMap::new();
        let mut total = 0;
        for (relative, mut file) in handles {
            let bytes = operations::read_blob(&mut file)?;
            total += bytes.len();
            if total > 32 * 1024 * 1024 {
                return Err(CoreError::new(
                    "budget",
                    UiMessage::error_core_hosted_hosted_snapshot_exceeds32_mi_b(),
                ));
            }
            files.insert(relative.clone(), bytes);
        }
        Ok(files)
    }
}
