//! Closed owner recovery backup, around the original BackupLibrary Plan.
use super::*;
use operations::Mutation;
use weftext_messages::UiMessage;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OwnerBackupOperation {
    root: String,
    destination: String,
    preview: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorOwnerBackupStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub plan_id: Option<String>,
    pub preview: serde_json::Value,
}

/// Borrowed exact original afterimages. Packaging must remain bounded; it may
/// not recapture the current library or reopen a possibly replaced spool.
pub struct OwnerBackupArtifact<'a> {
    pub directories: Vec<String>,
    pub files: Vec<(String, &'a [u8])>,
}

impl Core {
    fn owner_backup_record(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorOperation>> {
        feature::valid_resource(resource)?;
        let Some(record) =
            self.read_actor_operation(operation, resource, MAX_SOURCE_BYTES + 4096)?
        else {
            return Ok(None);
        };
        if self.actor.as_ref() != Some(&record.actor)
            || record.resource != resource
            || record.owner_backup.is_none()
            || record.feature.is_some()
            || record.field.is_some()
            || record.definition.is_some()
            || record.history.is_some()
            || !record.save_definition_reservations.is_empty()
            || record
                .library
                .as_ref()
                .and_then(|binding| binding.workspace_id.as_ref())
                .is_none()
        {
            return Err(feature::conflict());
        }
        valid_operation_id(&record.core_operation_id)?;
        Ok(Some(record))
    }

    fn require_owner_backup_scope(&self) -> Result<(&library::ActorLibraryBinding, &Path)> {
        let binding = self
            .actor_library
            .as_ref()
            .filter(|b| b.workspace_id.is_some())
            .ok_or_else(|| {
                CoreError::new(
                    "libraryScopeRequired",
                    UiMessage::error_core_hosted_backup_owner_backup_requires_trusted_admitted_library_scope(),
                )
            })?;
        let root = self
            .actor_library_path
            .as_deref()
            .ok_or_else(feature::conflict)?;
        self.actor_library_for_root(root)?;
        Ok((binding, root))
    }

    pub fn actor_owner_backup_status(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorOwnerBackupStatus>> {
        self.require_owner_backup_scope()?;
        let Some(record) = self.owner_backup_record(resource, operation)? else {
            return Ok(None);
        };
        let backup = record.owner_backup.as_ref().unwrap();
        let detail = match self.operation_details(&record.core_operation_id) {
            Ok(detail) => {
                let transaction = self
                    .operations
                    .get(&record.core_operation_id)
                    .and_then(|p| p.transaction.as_ref())
                    .ok_or_else(feature::conflict)?;
                if !matches!(&transaction.request.intent, OperationIntent::BackupLibrary { destination }
                    if destination == &backup.destination)
                    || transaction.request.root != backup.root
                    || !matches!(
                        transaction.mutation,
                        Mutation::CreateTree {
                            existing_target: false,
                            ..
                        }
                    )
                {
                    return Err(feature::conflict());
                }
                Some(detail)
            }
            Err(error)
                if error.code == "operationMissing"
                    && !self.operations.contains_key(&record.core_operation_id)
                    && !self
                        .journal_path(&record.core_operation_id, "plan")
                        .exists() =>
            {
                None
            }
            Err(error) => return Err(error),
        };
        let status = match detail.as_ref().map(|p| p.status.as_str()) {
            Some("committed") => "committed",
            Some("recoveryPending") => "recoveryPending",
            Some("resolved") => {
                let resolution: serde_json::Value = serde_json::from_slice(&fs::read(
                    self.journal_path(&record.core_operation_id, "resolution"),
                )?)?;
                if resolution["action"] == "cancelPrepared" {
                    "cancelled"
                } else {
                    "resolved"
                }
            }
            Some("prepared") if !record.cancelled => "prepared",
            Some("prepared") | None if record.cancelled => "cancelled",
            None => "notCommitted",
            _ => {
                return Err(CoreError::new(
                    "invalidState",
                    UiMessage::error_core_hosted_backup_unknown_original_backup_state(),
                ));
            }
        };
        Ok(Some(ActorOwnerBackupStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            status: status.into(),
            plan_id: detail.map(|p| p.plan_id),
            preview: backup.preview.clone(),
        }))
    }

    pub fn actor_owner_backup_retry(
        &self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
    ) -> Result<Option<ActorOwnerBackupStatus>> {
        feature::valid_fingerprint(fingerprint)?;
        self.require_owner_backup_scope()?;
        let Some(record) = self.owner_backup_record(resource, operation)? else {
            return Ok(None);
        };
        if !(record.cancelled && record.fingerprint.is_empty()) && record.fingerprint != fingerprint
        {
            return Err(feature::conflict());
        }
        self.actor_owner_backup_status(resource, operation)
    }

    /// The host owns destination and preview. First persist the immutable fence
    /// before capture. A failed/pre-plan retry returns its original fact rather
    /// than taking a different backup under the same external request ID.
    pub fn actor_owner_backup_prepare(
        &mut self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
        destination: &Path,
        preview: serde_json::Value,
    ) -> Result<ActorOwnerBackupStatus> {
        feature::valid_resource(resource)?;
        if let Some(status) = self.actor_owner_backup_retry(resource, operation, fingerprint)? {
            return Ok(status);
        }
        let root = self.require_owner_backup_scope()?.1.to_path_buf();
        let destination = display(&absolute_path(destination)?);
        check_size(&serde_json::to_vec(&preview)?)?;
        let record = ActorOperation {
            scope: self.actor_input_scope_for_root(&root)?,
            library: self.actor_library.clone(),
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint.into(),
            core_operation_id: fresh(),
            cancelled: false,
            field: None,
            feature: None,
            definition: None,
            history: None,
            save_definition_reservations: vec![],
            owner_backup: Some(OwnerBackupOperation {
                root: display(&root),
                destination: destination.clone(),
                preview,
            }),
        };
        journal::private_create(
            &self.actor_record(operation)?,
            &self.actor_operation_bytes(&record)?,
        )?;
        #[cfg(all(test, windows))]
        definition_test_failpoint("ownerBackupActorDurable")?;
        self.prepare_operation(OperationRequest {
            operation_id: record.core_operation_id,
            root: display(&root),
            intent: OperationIntent::BackupLibrary { destination },
        })?;
        self.actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)
    }

    pub fn actor_owner_backup_commit(
        &mut self,
        resource: &str,
        operation: &str,
        plan_id: &str,
    ) -> Result<ActorOwnerBackupStatus> {
        let record = self
            .owner_backup_record(resource, operation)?
            .ok_or_else(feature::conflict)?;
        let status = self
            .actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)?;
        if status.plan_id.as_deref() != Some(plan_id) {
            return Err(feature::conflict());
        }
        if status.status == "prepared" {
            self.execute_operation(&record.core_operation_id, plan_id)?;
        }
        self.actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)
    }

    pub fn actor_owner_backup_settle(
        &mut self,
        resource: &str,
        operation: &str,
    ) -> Result<ActorOwnerBackupStatus> {
        let root = self.require_owner_backup_scope()?.1.to_path_buf();
        feature::valid_resource(resource)?;
        let path = self.actor_record(operation)?;
        let mut record = match self.owner_backup_record(resource, operation)? {
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
                definition: None,
                history: None,
                save_definition_reservations: vec![],
                owner_backup: Some(OwnerBackupOperation {
                    root: display(&root),
                    destination: String::new(),
                    preview: serde_json::Value::Null,
                }),
            },
        };
        if let Some(status) = self.actor_owner_backup_status(resource, operation)? {
            if matches!(
                status.status.as_str(),
                "committed" | "recoveryPending" | "resolved" | "cancelled"
            ) {
                return Ok(status);
            }
            if status.status == "prepared" {
                self.resolve_record(&record.core_operation_id, "cancelPrepared")?;
            }
            record.cancelled = true;
            private_replace(&path, &self.actor_operation_bytes(&record)?)?;
        } else {
            journal::private_create(&path, &self.actor_operation_bytes(&record)?)?;
        }
        self.actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)
    }

    pub fn actor_owner_backup_artifact(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<OwnerBackupArtifact<'_>> {
        let status = self
            .actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)?;
        if status.status != "committed" {
            return Err(CoreError::new(
                "artifactUnavailable",
                UiMessage::error_core_hosted_backup_original_backup_not_committed(),
            ));
        }
        self.owner_backup_artifact_from_plan(resource, operation)
    }

    /// Pure original-plan preflight. Packaging failures must be discovered
    /// before any backup filesystem effect, without re-reading the source cut.
    pub fn actor_owner_backup_preflight_artifact(
        &self,
        resource: &str,
        operation: &str,
        plan_id: &str,
    ) -> Result<OwnerBackupArtifact<'_>> {
        let status = self
            .actor_owner_backup_status(resource, operation)?
            .ok_or_else(feature::conflict)?;
        if status.status != "prepared" || status.plan_id.as_deref() != Some(plan_id) {
            return Err(CoreError::new(
                "artifactUnavailable",
                UiMessage::error_core_hosted_backup_exact_original_backup_not_prepared(),
            ));
        }
        self.owner_backup_artifact_from_plan(resource, operation)
    }

    fn owner_backup_artifact_from_plan(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<OwnerBackupArtifact<'_>> {
        let record = self
            .owner_backup_record(resource, operation)?
            .ok_or_else(feature::conflict)?;
        let transaction = self
            .operations
            .get(&record.core_operation_id)
            .and_then(|p| p.transaction.as_ref())
            .ok_or_else(feature::conflict)?;
        let Mutation::CreateTree {
            target,
            directories,
            files,
            updates,
            ..
        } = &transaction.mutation
        else {
            return Err(feature::conflict());
        };
        if !updates.is_empty() {
            return Err(feature::conflict());
        }
        let mut output = Vec::new();
        let mut bytes = 0usize;
        let mut manifest = 0usize;
        for file in files {
            let relative =
                crate::history::history_relative_path(Path::new(&file.path), Path::new(target))
                    .ok_or_else(feature::conflict)?;
            let relative = display(&relative).replace('\\', "/");
            if file.before.is_some()
                || relative.is_empty()
                || !(relative == "backup.json"
                    || relative.starts_with("library/")
                    || relative.starts_with("private/"))
            {
                return Err(feature::conflict());
            }
            manifest += usize::from(relative == "backup.json");
            bytes = bytes
                .checked_add(file.after.len())
                .ok_or_else(feature::conflict)?;
            output.push((relative, file.after.as_slice()));
        }
        if manifest != 1 || bytes > 64 * 1024 * 1024 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_backup_original_backup_artifact_exceeds_budget(),
            ));
        }
        let mut names = Vec::new();
        let mut seen = std::collections::BTreeMap::<String, String>::new();
        for (relative, _) in &output {
            if seen
                .insert(relative.to_ascii_lowercase(), relative.clone())
                .is_some()
            {
                return Err(feature::conflict());
            }
        }
        for directory in directories {
            let relative = Path::new(directory);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|part| !matches!(part, Component::Normal(_)))
            {
                return Err(feature::conflict());
            }
            let relative = display(relative).replace('\\', "/");
            if !(relative == "library"
                || relative == "private"
                || relative.starts_with("library/")
                || relative.starts_with("private/"))
            {
                return Err(feature::conflict());
            }
            let key = relative.to_ascii_lowercase();
            if let Some(prior) = seen.get(&key) {
                if prior != &relative || output.iter().any(|(file, _)| file == &relative) {
                    return Err(feature::conflict());
                }
                continue;
            }
            seen.insert(key, relative.clone());
            names.push(relative);
        }
        if output.len() + names.len() > 5000 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_hosted_backup_original_backup_members_exceed_their_budget(),
            ));
        }
        Ok(OwnerBackupArtifact {
            directories: names,
            files: output,
        })
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_backup_tests.rs"]
mod tests;
