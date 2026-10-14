//! Durable actor intent and reviewed preview around the existing Core writer.
//! The host authorizes participants and filters this trusted result for delivery.
use super::*;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorFeatureStatus {
    pub operation_id: String,
    pub resource: String,
    pub status: String,
    pub plan_id: Option<String>,
    /// Exact host-provided author preview, never a copied Core OperationPreview.
    pub preview: serde_json::Value,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct FeatureOperation {
    preview: serde_json::Value,
}

fn conflict() -> CoreError {
    CoreError::new(
        "operationConflict",
        "The original reviewed actor, resource or intent differs",
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

impl Core {
    fn actor_feature_record(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorOperation>> {
        valid_resource(resource)?;
        let file = match File::open(self.actor_record(operation)?) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        // The record is service-owned, but bounded reads also fail closed on a
        // damaged or replaced private file rather than allocating without limit.
        let mut bytes = vec![];
        file.take((MAX_SOURCE_BYTES + 4097) as u64)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_SOURCE_BYTES + 4096 {
            return Err(CoreError::new(
                "budget",
                "Reviewed actor record exceeds its budget",
            ));
        }
        let record: ActorOperation = serde_json::from_slice(&bytes)?;
        if self.actor.as_ref() != Some(&record.actor)
            || record.resource != resource
            || record.feature.is_none()
            || record.field.is_some()
        {
            return Err(conflict());
        }
        valid_operation_id(&record.core_operation_id)?;
        Ok(Some(record))
    }

    pub fn actor_feature_status(
        &self,
        resource: &str,
        operation: &str,
    ) -> Result<Option<ActorFeatureStatus>> {
        let Some(record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        let detail = match self.operation_details(&record.core_operation_id) {
            Ok(detail) => {
                let transaction = self.operations[&record.core_operation_id]
                    .transaction
                    .as_ref()
                    .unwrap();
                if !matches!(
                    transaction.request.intent,
                    OperationIntent::UpdateFiles { .. }
                ) {
                    return Err(conflict());
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
                    "Unknown original feature operation state",
                ));
            }
        };
        Ok(Some(ActorFeatureStatus {
            operation_id: operation.into(),
            resource: resource.into(),
            status: status.into(),
            plan_id: detail.map(|p| p.plan_id),
            preview: record.feature.unwrap().preview,
        }))
    }

    /// A retry reads only the original durable record and Core plan; no current
    /// source, new dependencies, or newly generated preview can replace it.
    pub fn actor_feature_retry(
        &self,
        resource: &str,
        operation: &str,
        fingerprint: &str,
    ) -> Result<Option<ActorFeatureStatus>> {
        valid_fingerprint(fingerprint)?;
        let Some(record) = self.actor_feature_record(resource, operation)? else {
            return Ok(None);
        };
        if !(record.cancelled && record.fingerprint.is_empty()) && record.fingerprint != fingerprint
        {
            return Err(conflict());
        }
        self.actor_feature_status(resource, operation)
    }

    /// Bind the original authenticated intent and author preview before Core
    /// prepares any filesystem participant. A crash or prepare failure in this
    /// gap stays notCommitted; its retry cannot regenerate a different plan.
    pub fn actor_feature_prepare(
        &mut self,
        resource: &str,
        external_operation_id: &str,
        fingerprint: &str,
        mut request: OperationRequest,
        preview: serde_json::Value,
        dependencies: &[String],
    ) -> Result<ActorFeatureStatus> {
        if let Some(status) =
            self.actor_feature_retry(resource, external_operation_id, fingerprint)?
        {
            return Ok(status);
        }
        let OperationIntent::UpdateFiles { changes } = &request.intent else {
            return Err(CoreError::new(
                "unsupportedIntent",
                "Hosted review currently permits only UpdateFiles",
            ));
        };
        if changes.is_empty()
            || changes.len() > 64
            || dependencies.len() > 64
            || request.root.len() > 64 * 1024
            || changes.iter().any(|c| c.path.len() > 64 * 1024)
            || changes
                .iter()
                .try_fold(0usize, |total, change| {
                    total
                        .checked_add(change.before.as_ref().map_or(0, Vec::len))?
                        .checked_add(change.after.len())
                })
                .is_none_or(|bytes| bytes > 32 * 1024 * 1024)
        {
            return Err(CoreError::new(
                "budget",
                "Hosted review exceeds its explicit participant or byte budget",
            ));
        }
        for dependency in dependencies {
            valid_operation_id(dependency)?;
        }
        check_size(&serde_json::to_vec(&preview)?)?;
        let record = ActorOperation {
            actor: self.actor.clone().unwrap(),
            resource: resource.into(),
            fingerprint: fingerprint.into(),
            core_operation_id: fresh(),
            cancelled: false,
            field: None,
            feature: Some(FeatureOperation { preview }),
        };
        journal::private_create(
            &self.actor_record(external_operation_id)?,
            &serde_json::to_vec(&record)?,
        )?;
        request.operation_id = record.core_operation_id.clone();
        self.prepare_dependent_feature_operation(request, &serde_json::json!({
            "kind":"hostedReviewedFeature","actor":record.actor,"resource":record.resource,"fingerprint":record.fingerprint,
        }), dependencies)?;
        self.actor_feature_status(resource, external_operation_id)?
            .ok_or_else(|| CoreError::new("invalidState", "Missing original feature operation"))
    }

    pub fn actor_feature_commit(
        &mut self,
        resource: &str,
        operation: &str,
        plan_id: &str,
    ) -> Result<ActorFeatureStatus> {
        valid_operation_id(plan_id)?;
        let record = self
            .actor_feature_record(resource, operation)?
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "Prepare the original reviewed operation first",
                )
            })?;
        let status = self.actor_feature_status(resource, operation)?.unwrap();
        if status.plan_id.as_deref() != Some(plan_id) {
            return Err(CoreError::new(
                "planConflict",
                "Commit the exact original reviewed plan",
            ));
        }
        // Committed receipts are idempotent. Resolved, cancelled, and installed
        // plans are never re-executed through this hosted wrapper.
        if status.status != "prepared" {
            return Ok(status);
        }
        self.execute_operation(&record.core_operation_id, plan_id)?;
        self.actor_feature_status(resource, operation)?
            .ok_or_else(|| CoreError::new("invalidState", "Missing original feature operation"))
    }

    /// End an unknown or never-installed original proposal durably. Once the
    /// install marker exists, status/recovery remain authoritative and no
    /// tombstone may cancel or mask possibly committed filesystem effects.
    pub fn actor_feature_settle(
        &mut self,
        resource: &str,
        operation: &str,
    ) -> Result<ActorFeatureStatus> {
        let path = self.actor_record(operation)?;
        let mut record = match self.actor_feature_record(resource, operation)? {
            Some(record) => record,
            None => ActorOperation {
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: true,
                field: None,
                feature: Some(FeatureOperation {
                    preview: serde_json::Value::Null,
                }),
            },
        };
        if let Some(status) = self.actor_feature_status(resource, operation)? {
            if matches!(
                status.status.as_str(),
                "committed" | "recoveryPending" | "resolved" | "cancelled"
            ) {
                return Ok(status);
            }
            if status.status == "prepared" {
                // Resolving Core first prevents any direct original-plan replay,
                // including a crash before the actor tombstone is persisted.
                self.resolve_record(&record.core_operation_id, "cancelPrepared")?;
            }
            record.cancelled = true;
            private_replace(&path, &serde_json::to_vec(&record)?)?;
        } else {
            journal::private_create(&path, &serde_json::to_vec(&record)?)?;
        }
        self.actor_feature_status(resource, operation)?
            .ok_or_else(|| CoreError::new("invalidState", "Missing settled feature operation"))
    }
}

#[cfg(all(test, windows))]
#[path = "hosted_feature_tests.rs"]
mod tests;
