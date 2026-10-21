//! Hosted history restores retain the original actor operation and reuse the
//! ordinary source save. Status facts never require ambient owner discovery.
use super::*;
use weftext_knowledge::NodeRef;

const MAX_RECORD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorHistoryRestoreStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub result: Option<HistoryRestoreResult>,
    pub result_status: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HistoryOperation {
    original: Option<OriginalHistoryRestore>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OriginalHistoryRestore {
    request: HistoryRestoreRequest,
    intent_fingerprint: String,
    root: String,
    path: String,
    owner: NodeRef,
    identity: FileIdentity,
}

fn conflict() -> CoreError {
    CoreError::new(
        "operationConflict",
        "The original actor history operation or owner differs",
    )
}
fn valid_fingerprint(fingerprint: &str) -> Result<()> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(CoreError::new(
            "invalidIntent",
            "A service-owned lowercase SHA-256 intent fingerprint is required",
        ));
    }
    Ok(())
}
fn valid_resource(resource: &str) -> Result<()> {
    if resource.is_empty() || resource.len() > 256 || resource.chars().any(char::is_control) {
        return Err(CoreError::new(
            "invalidResource",
            "A bounded configured resource is required",
        ));
    }
    Ok(())
}
fn fingerprint(
    actor: &str,
    resource: &str,
    core_operation: &str,
    original: &OriginalHistoryRestore,
) -> Result<String> {
    Ok(hash(&serde_json::to_vec(&serde_json::json!({
        "kind":"hostedHistoryRestore","actor":actor,"resource":resource,
        "coreOperationId":core_operation,"original":original,
    }))?))
}
fn internal_request(record: &ActorOperation) -> Result<HistoryRestoreRequest> {
    let mut request = record
        .history
        .as_ref()
        .and_then(|h| h.original.as_ref())
        .ok_or_else(conflict)?
        .request
        .clone();
    request.operation_id = record.core_operation_id.clone();
    Ok(request)
}

impl Core {
    fn actor_history_record(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorOperation>> {
        valid_resource(resource)?;
        valid_operation_id(operation)?;
        if Uuid::parse_str(operation).unwrap().to_string() != operation {
            return Err(CoreError::new(
                "invalidOperationId",
                "A canonical history operation UUID is required",
            ));
        }
        let file = match File::open(self.actor_record(operation)?) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let mut bytes = vec![];
        file.take((MAX_RECORD_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CoreError::new(
                "budget",
                "The retained history request exceeds 16 MiB",
            ));
        }
        let record: ActorOperation = serde_json::from_slice(&bytes)?;
        self.check_actor_library(&record.library)?;
        if self.actor.as_ref() != Some(&record.actor)
            || record.resource != resource
            || record.history.is_none()
            || record.field.is_some()
            || record.feature.is_some()
            || record.definition.is_some()
            || record.owner_backup.is_some()
            || !record.save_definition_reservations.is_empty()
        {
            return Err(conflict());
        }
        valid_operation_id(&record.core_operation_id)?;
        if let Some(original) = &record.history.as_ref().unwrap().original {
            valid_fingerprint(&original.intent_fingerprint)?;
            if original.request.operation_id != operation
                || record.fingerprint
                    != fingerprint(&record.actor, resource, &record.core_operation_id, original)?
            {
                return Err(conflict());
            }
        } else if !record.cancelled || !record.fingerprint.is_empty() {
            return Err(conflict());
        }
        Ok(Some(record))
    }

    fn actor_history_plan<'a>(
        &'a self,
        record: &ActorOperation,
    ) -> Result<Option<&'a journal::Plan>> {
        let Some(plan) = self.operations.get(&record.core_operation_id) else {
            return Ok(None);
        };
        let original = record
            .history
            .as_ref()
            .and_then(|h| h.original.as_ref())
            .ok_or_else(conflict)?;
        let request = internal_request(record)?;
        let binding = plan.history_restore.as_ref().ok_or_else(conflict)?;
        if binding.fingerprint != hash(&serde_json::to_vec(&request)?)
            || binding.node_id != original.owner.node_id
            || binding.workspace_id != original.owner.workspace_id
            || binding.identity != original.identity
            || plan.identity.as_ref() != Some(&original.identity)
            || Path::new(&plan.path) != Path::new(&original.path)
            || plan.kind != "save"
            || plan.transaction.is_some()
        {
            return Err(conflict());
        }
        Ok(Some(plan))
    }

    /// This summary carries original durable facts only, without current source
    /// or retained Draft delivery. The host still authorizes the named resource.
    pub fn actor_history_restore_status(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorHistoryRestoreStatus>> {
        let Some(record) = self.actor_history_record(resource, operation)? else {
            return Ok(None);
        };
        let plan = self.actor_history_plan(&record)?;
        let state = if let Some(plan) = plan {
            if self.committed(&plan.operation_id) {
                "committed"
            } else if self.resolved(&plan.operation_id)? {
                "resolved"
            } else {
                "recoveryPending"
            }
        } else if record.cancelled {
            "cancelled"
        } else {
            "notCommitted"
        };
        Ok(Some(ActorHistoryRestoreStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            status: state.into(),
            result: None,
            result_status: if state == "committed" {
                "notRequested"
            } else {
                "notCommitted"
            }
            .into(),
        }))
    }

    pub fn actor_history_restore_retry(
        &self,
        resource: &str,
        operation: &str,
        intent_fingerprint: &str,
    ) -> Result<Option<ActorHistoryRestoreStatus>> {
        valid_fingerprint(intent_fingerprint)?;
        let Some(record) = self.actor_history_record(resource, operation)? else {
            return Ok(None);
        };
        if let Some(original) = record.history.as_ref().unwrap().original.as_ref()
            && original.intent_fingerprint != intent_fingerprint
        {
            return Err(conflict());
        }
        self.actor_history_restore_status(resource, operation)
    }

    /// Qualify only the current configured path against the original owner. A
    /// pending original write may make delivery unavailable without erasing it.
    pub fn actor_history_restore_owner_at(
        &self,
        resource: &str,
        root: &Path,
        path: &Path,
        operation: &str,
    ) -> Result<()> {
        let record = self
            .actor_history_record(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "The original history operation is unavailable",
                )
            })?;
        let Some(original) = record.history.as_ref().unwrap().original.as_ref() else {
            return Ok(());
        };
        let current = self.qualified_node_at(root, path)?;
        if current.owner != original.owner || current.identity != original.identity {
            return Err(CoreError::new(
                "identityPending",
                "The configured object differs from the original restore owner",
            ));
        }
        Ok(())
    }

    pub fn actor_history_restore_result_at(
        &self,
        resource: &str,
        root: &Path,
        path: &Path,
        operation: &str,
    ) -> Result<HistoryRestoreResult> {
        let record = self
            .actor_history_record(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "The original history operation is unavailable",
                )
            })?;
        let plan = self.actor_history_plan(&record)?.ok_or_else(|| {
            CoreError::new(
                "notCommitted",
                "The original restore has no source save plan",
            )
        })?;
        if !self.committed(&plan.operation_id) {
            return Err(CoreError::new(
                "recoveryPending",
                "The original restore is not committed",
            ));
        }
        self.actor_history_restore_owner_at(resource, root, path, operation)?;
        let mut result = self.restore_result_at(plan, root, path)?;
        result.receipt.operation_id = operation.into();
        Ok(result)
    }
    /// Compare the latest exact saved operation, not just equal content or its
    /// reusable RevisionId. A later same-byte save is a different operation.
    pub fn actor_history_restore_current_at(
        &self,
        resource: &str,
        root: &Path,
        path: &Path,
        operation: &str,
    ) -> Result<bool> {
        let record = self
            .actor_history_record(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "The original history operation is unavailable",
                )
            })?;
        let plan = self.actor_history_plan(&record)?.ok_or_else(|| {
            CoreError::new(
                "notCommitted",
                "The original restore has no source save plan",
            )
        })?;
        if !self.committed(&plan.operation_id) {
            return Err(CoreError::new(
                "recoveryPending",
                "The original restore is not committed",
            ));
        }
        let original = record.history.as_ref().unwrap().original.as_ref().unwrap();
        let current = self.qualified_node_at(root, path)?;
        if current.owner != original.owner || current.identity != original.identity {
            return Err(CoreError::new(
                "identityPending",
                "The configured object differs from the original restore owner",
            ));
        }
        Ok(self.current_saved_history_operation(&current)?.as_deref()
            == Some(record.core_operation_id.as_str()))
    }

    /// New requests alone enter the existing history Draft protection and save.
    /// Retrying a pre-plan failure does not regenerate another source proposal.
    pub fn actor_history_restore(
        &mut self,
        resource: &str,
        root: &Path,
        path: &Path,
        request: HistoryRestoreRequest,
        intent_fingerprint: &str,
    ) -> Result<ActorHistoryRestoreStatus> {
        self.actor_library_for_root(root)?;
        valid_fingerprint(intent_fingerprint)?;
        if let Some(mut status) =
            self.actor_history_restore_retry(resource, &request.operation_id, intent_fingerprint)?
        {
            let record = self
                .actor_history_record(resource, &request.operation_id)?
                .unwrap();
            if let Some(original) = record.history.as_ref().unwrap().original.as_ref()
                && serde_json::to_value(&original.request)? != serde_json::to_value(&request)?
            {
                return Err(conflict());
            }
            if status.status == "committed" {
                match self.actor_history_restore_result_at(
                    resource,
                    root,
                    path,
                    &request.operation_id,
                ) {
                    Ok(result) => {
                        status.result = Some(result);
                        status.result_status = "available".into();
                    }
                    Err(_) => status.result_status = "unavailable".into(),
                }
            }
            return Ok(status);
        }
        if !matches!(request.draft_action.as_str(), "retain" | "none") {
            return Err(CoreError::new(
                "invalidDraftAction",
                "Choose retain or none explicitly",
            ));
        }
        if let Some(source) = &request.draft_source {
            check_size(source.as_bytes())?
        }
        self.check_session_base(&request.session_id, &request.base_id)?;
        let current = self.qualified_node_for_session_at(root, path, &request.session_id)?;
        // Before binding the external operation, prove the selected exact
        // historical source belongs to this opened owner. No latest fallback.
        self.read_history(&request.session_id, &request.version_id)?;
        let original = OriginalHistoryRestore {
            request: request.clone(),
            intent_fingerprint: intent_fingerprint.into(),
            root: display(&absolute_path(root)?),
            path: display(&absolute_path(path)?),
            owner: current.owner,
            identity: current.identity,
        };
        let core_operation_id = fresh();
        let record = ActorOperation {
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint(
                self.actor.as_ref().unwrap(),
                resource,
                &core_operation_id,
                &original,
            )?,
            core_operation_id,
            cancelled: false,
            field: None,
            feature: None,
            definition: None,
            save_definition_reservations: vec![],
            history: Some(HistoryOperation {
                original: Some(original),
            }),
            owner_backup: None,
        };
        let bytes = Self::actor_operation_bytes(&record)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CoreError::new(
                "budget",
                "The history request exceeds its durable budget",
            ));
        }
        journal::private_create(&self.actor_record(&request.operation_id)?, &bytes)?;
        #[cfg(all(test, windows))]
        definition_test_failpoint("historyActorDurable")?;
        self.restore_history_at(root, path, internal_request(&record)?)?;
        let mut status = self
            .actor_history_restore_status(resource, &request.operation_id)?
            .ok_or_else(conflict)?;
        status.result = Some(self.actor_history_restore_result_at(
            resource,
            root,
            path,
            &request.operation_id,
        )?);
        status.result_status = "available".into();
        Ok(status)
    }

    /// Unknown requests receive a durable actor/category fence. Installed
    /// original source plans remain authoritative and cannot be cancelled here.
    pub fn actor_history_restore_settle(
        &mut self,
        resource: &str,
        operation: &str,
    ) -> Result<ActorHistoryRestoreStatus> {
        let mut record = match self.actor_history_record(resource, operation)? {
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
                definition: None,
                save_definition_reservations: vec![],
                history: Some(HistoryOperation { original: None }),
                owner_backup: None,
            },
        };
        let path = self.actor_record(operation)?;
        if let Some(status) = self.actor_history_restore_status(resource, operation)? {
            if status.status != "notCommitted" {
                return Ok(status);
            }
            record.cancelled = true;
            private_replace(&path, &Self::actor_operation_bytes(&record)?)?;
        } else {
            journal::private_create(&path, &Self::actor_operation_bytes(&record)?)?;
        }
        self.actor_history_restore_status(resource, operation)?
            .ok_or_else(conflict)
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_history_tests.rs"]
mod tests;
