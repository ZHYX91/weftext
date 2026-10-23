//! Original journal facts and device-local cancellation. No second receipt store.
use super::*;
use journal::{AttemptFence, ResolutionRecord};
use weftext_messages::UiMessage;

impl Core {
    pub(crate) fn validate_original_reservation_library(
        &self,
        binding: &hosted::ActorLibraryBinding,
    ) -> Result<String> {
        let invalid = || {
            CoreError::new(
                "resolutionUnavailable",
                UiMessage::error_core_local_operation_reservation_has_no_original_committed_library_identity(),
            )
        };
        let workspace = binding.workspace_id.as_ref().ok_or_else(invalid)?;
        valid_operation_id(workspace)?;
        let mut original = None;
        for plan in self.operations.values().filter(|plan| {
            matches!(plan.kind.as_str(), "createLibrary" | "cloneSync")
                && plan.workspace_id.as_ref() == Some(workspace)
        }) {
            if self.committed(&plan.operation_id)? && original.replace(plan).is_some() {
                return Err(invalid());
            }
        }
        let plan = original.ok_or_else(invalid)?;
        if self
            .materialized_identity(&plan.operation_id, Path::new(&plan.path))?
            .as_ref()
            != Some(&binding.root_identity)
        {
            return Err(invalid());
        }
        Ok(plan.path.clone())
    }
    pub(crate) fn validate_preplan_reservation(
        &self,
        id: &str,
        suffix: &str,
        bytes: &[u8],
    ) -> Result<hosted::ActorLibraryBinding> {
        match suffix {
            "definition-reservation" | "revision-reservation" => {
                self.validate_source_reservation(id, suffix, bytes)
            }
            "tag-reservation" | "directory-reservation" => {
                self.validate_navigation_reservation(id, suffix, bytes)
            }
            _ => Err(CoreError::new(
                "resolutionUnavailable",
                UiMessage::error_core_local_operation_unknown_pre_plan_evidence_cannot_prove_absence_of_author_effects(),
            )),
        }
    }
    pub(crate) fn check_attempt_not_cancelled(&self, id: &str) -> Result<()> {
        // A committed original wins over unrelated delivery metadata.
        if !self.committed(id)? && self.resolved(id)? {
            return Err(CoreError::new(
                "operationCancelled",
                UiMessage::error_core_local_operation_original_attempt_durably_terminated_no_effects_were_retried(),
            ));
        }
        Ok(())
    }
    /// Raw file-save fact reader. Hosted callers must first verify their actor,
    /// resource and original library binding; it needs no old live Session.
    pub(crate) fn validate_file_save_plan(&self, plan: &journal::Plan) -> Result<()> {
        if !matches!(plan.kind.as_str(), "save" | "saveAs") || plan.transaction.is_some() {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_original_scope_save_original_not_file_save(),
            ));
        }
        let data = plan
            .file_save
            .as_ref()
            .ok_or_else(file_save_scope::unsupported_scope)?;
        if data.version != 2 {
            return Err(file_save_scope::unsupported_scope());
        }
        valid_operation_id(&data.session_id)?;
        valid_operation_id(&data.draft_producer_id)?;
        if let Some(base) = &data.base_id {
            valid_operation_id(base)?;
        }
        if (plan.kind == "save") != data.base_id.is_some() {
            return Err(CoreError::new(
                "invalidState",
                UiMessage::error_core_original_scope_save_request_participants_differ(),
            ));
        }
        if file_save_scope::fingerprint(
            &plan.kind,
            Path::new(&plan.path),
            &decode(&plan.after)?,
            data,
            plan.history_restore.as_ref(),
        )? != plan.fingerprint
        {
            return Err(CoreError::new(
                "invalidState",
                UiMessage::error_core_original_scope_save_frozen_fingerprint_differs(),
            ));
        }
        self.validate_original_file_save_scope(&data.scope, Path::new(&plan.path))?;
        if let Some(workspace) = &plan.workspace_id
            && !matches!(&data.scope, QualifiedInputScope::Library { binding } if &binding.workspace_id == workspace)
        {
            return Err(CoreError::new(
                "invalidState",
                UiMessage::error_core_original_scope_save_sync_owner_differs(),
            ));
        }
        let invalid = || {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_original_scope_save_plan_participants_or_receipt_invalid(),
            )
        };
        let receipt = plan.receipt.as_ref().ok_or_else(invalid)?;
        if !matches!(plan.kind.as_str(), "save" | "saveAs")
            || plan.transaction.is_some()
            || plan.sync_setup.is_some()
            || plan.sync_application.is_some()
            || !plan.files.is_empty()
            || plan.node_id.is_some()
            || receipt.operation_id != plan.operation_id
            || receipt.path != plan.path
            || receipt.status != "saved"
            || plan.after.len() > MAX_SOURCE_BYTES
            || plan.content_revision != receipt.revision_id
            || plan.workspace_id.as_ref()
                != plan
                    .sync_publication
                    .as_ref()
                    .map(|publication| &publication.identity.workspace_id)
            || (plan.kind == "save"
                && (plan.before.is_none()
                    || plan.identity.is_none()
                    || plan.content_revision.is_none()))
            || (plan.kind == "saveAs"
                && (plan.before.is_some()
                    || plan.identity.is_some()
                    || plan.history_restore.is_some()
                    || plan.content_revision.is_some()))
        {
            return Err(invalid());
        }
        valid_operation_id(&receipt.base_id)?;
        if let Some(revision) = &receipt.revision_id {
            valid_operation_id(revision)?;
        }
        Ok(())
    }
    pub(crate) fn file_save_fact(&self, id: &str) -> Result<Option<Receipt>> {
        valid_operation_id(id)?;
        let Some(plan) = self.operations.get(id) else {
            return Ok(None);
        };
        if !matches!(plan.kind.as_str(), "save" | "saveAs") || plan.transaction.is_some() {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_local_operation_original_operation_not_file_save(),
            ));
        }
        self.validate_file_save_plan(plan)?;
        if self.committed(id)? {
            return plan.receipt.clone().map(Some).ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_local_operation_committed_file_save_has_no_original_receipt(),
                )
            });
        }
        Ok(None)
    }
    /// Public global lookup is local-owner only. Actor APIs use their scoped
    /// records before accessing the same internal Plan and commit fact.
    pub fn local_operation_status(&self, id: &str) -> Result<serde_json::Value> {
        self.backend.check_path(&self.data_dir)?;
        self.sync_local_owner()?;
        valid_operation_id(id)?;
        let Some(plan) = self.operations.get(id) else {
            let status = if self.resolved(id)? {
                "resolved"
            } else {
                "unknown"
            };
            return Ok(serde_json::json!({"operationId":id,"status":status}));
        };
        let committed = self.committed(id)?;
        if let Some(t) = &plan.transaction {
            if t.request.operation_id != id
                || t.preview.operation_id != id
                || t.receipt.operation_id != id
                || t.preview.plan_id != t.receipt.plan_id
                || t.preview.kind != plan.kind
                || operations::intent_kind(&t.request.intent) != plan.kind
                || t.receipt.status != "committed"
            {
                return Err(CoreError::new(
                    "invalidState",
                    UiMessage::error_core_original_scope_operation_transaction_identity_or_kind_differs(),
                ));
            }
            if committed {
                return Ok(serde_json::to_value(&t.receipt)?);
            }
            return Ok(serde_json::to_value(self.operation_details(id)?)?);
        }
        if matches!(plan.kind.as_str(), "save" | "saveAs") {
            self.validate_file_save_plan(plan)?;
            if committed {
                let mut fact = serde_json::to_value(plan.receipt.as_ref().unwrap())?;
                fact["status"] = serde_json::json!("committed");
                return Ok(fact);
            }
        }
        if plan.kind == "enableSync" {
            let mut fact = self.original_sync_setup_fact(plan)?;
            fact["status"] = serde_json::json!(if committed {
                "committed"
            } else if self.resolved(id)? {
                "resolved"
            } else {
                "recoveryPending"
            });
            return Ok(fact);
        }
        if !matches!(plan.kind.as_str(), "save" | "saveAs")
            && !library_creation::is_cancellation(plan)
        {
            return Err(CoreError::new(
                "invalidState",
                UiMessage::error_core_original_scope_operation_plan_result_contract_unsupported(),
            ));
        }
        Ok(
            serde_json::json!({"operationId":id,"status":if self.resolved(id)? {"resolved"} else {"recoveryPending"}}),
        )
    }
    /// Independent current-document delivery; never re-saves source or needs
    /// the old Session. Stable original admission qualifies a moved owner.
    pub fn reopen_local_save(&mut self, id: &str) -> Result<(String, OpenedDocument)> {
        self.sync_local_owner()?;
        let receipt = self.file_save_fact(id)?.ok_or_else(|| {
            CoreError::new(
                "notCommitted",
                UiMessage::error_core_local_operation_no_committed_original_file_save(),
            )
        })?;
        let plan = self.operations[id].clone();
        let source = decode(&plan.after)?;
        let identity = if plan.kind == "saveAs" {
            self.materialized_identity(id, Path::new(&plan.path))?
                .ok_or_else(|| {
                    CoreError::new(
                        "committedResultUnavailable",
                        UiMessage::error_core_original_scope_save_save_as_birth_unavailable(),
                    )
                })?
        } else {
            plan.identity.clone().ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_original_scope_save_physical_identity_missing(),
                )
            })?
        };
        let scope = &plan.file_save.as_ref().unwrap().scope;
        let (path, root) = match scope {
            QualifiedInputScope::Library { binding } => {
                let root = PathBuf::from(
                    self.validate_original_reservation_library(&binding.actor_binding())?,
                );
                let workspace = self.require_workspace(&root)?;
                let guard = ParentGuard::acquire(&self.backend, &root.join(".file-result-root"))?;
                if workspace.workspace_id != binding.workspace_id
                    || guard.directory_identity()? != binding.root_identity
                {
                    return Err(CoreError::new(
                        "committedResultUnavailable",
                        UiMessage::error_core_original_scope_save_original_library_unavailable(),
                    ));
                }
                let path = if let Some(birth) = self.admitted_binding(&identity)? {
                    if birth.workspace_id != binding.workspace_id {
                        return Err(CoreError::new(
                            "committedResultUnavailable",
                            UiMessage::error_core_original_scope_save_physical_owner_library_differs(),
                        ));
                    }
                    let inventory = self.inventory(&root)?;
                    let entries = inventory
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.identity_status == "admitted"
                                && entry.node_id.as_ref() == Some(&birth.node_id)
                        })
                        .collect::<Vec<_>>();
                    if entries.len() != 1 {
                        return Err(CoreError::new(
                            "committedResultUnavailable",
                            UiMessage::error_core_original_scope_save_admitted_location_not_unique(
                            ),
                        ));
                    }
                    let path = PathBuf::from(&entries[0].path);
                    if self.qualified_node_at(&root, &path)?.identity != identity {
                        return Err(CoreError::new(
                            "committedResultUnavailable",
                            UiMessage::error_core_original_scope_save_physical_object_differs(),
                        ));
                    }
                    path
                } else {
                    PathBuf::from(&receipt.path)
                };
                (path, Some(root))
            }
            QualifiedInputScope::DeviceUnassigned => (PathBuf::from(&receipt.path), None),
        };
        self.validate_current_file_save_scope(scope, &path)?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let file = guard.open_file(false, false)?;
        if stamp(&file)?.identity != identity {
            return Err(CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_core_local_operation_saved_path_now_names_another_physical_object(
                ),
            ));
        }
        drop(file);
        drop(guard);
        let document = self.open(&path, root)?;
        let reopened = &self.sessions[&document.session_id];
        if reopened.stamp.identity != identity || reopened.file_scope.as_ref().ok() != Some(scope) {
            self.sessions.remove(&document.session_id);
            return Err(CoreError::new(
                "committedResultUnavailable",
                UiMessage::error_core_original_scope_save_delivery_owner_or_scope_changed(),
            ));
        }
        Ok((source, document))
    }
    pub fn local_operation_root(&self, id: &str) -> Result<String> {
        self.backend.check_path(&self.data_dir)?;
        self.sync_local_owner()?;
        valid_operation_id(id)?;
        self.operations
            .get(id)
            .and_then(|plan| plan.transaction.as_ref())
            .map(|t| t.request.root.clone())
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    UiMessage::error_core_local_operation_no_original_transaction_root_retained(),
                )
            })
    }
    /// Missing is not proof of no effect. A flushed original resolution fences
    /// delayed requests before the client can release the original attempt.
    /// Existing prepared/installing Plans are preserved for explicit recovery.
    pub fn settle_local_attempt(
        &mut self,
        id: &str,
        request_kind: &str,
        wire_digest: &str,
    ) -> Result<serde_json::Value> {
        self.sync_local_owner()?;
        valid_operation_id(id)?;
        if self.operations.contains_key(id) || self.resolved(id)? {
            return self.local_operation_status(id);
        }
        let record = ResolutionRecord {
            operation_id: id.into(),
            action: "cancelPrepared".into(),
            historical_message: None,
            attempt: Some(AttemptFence {
                request_kind: request_kind.into(),
                wire_digest: wire_digest.into(),
            }),
        };
        let bytes = serde_json::to_vec(&record)?;
        journal::decode_resolution(id, &bytes)?;
        self.validate_unregistered_fence_namespace(id)?;
        journal::private_create(&self.backend, &self.journal_path(id, "resolution"), &bytes)?;
        self.local_operation_status(id)
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Core, PathBuf, PathBuf, OpenedDocument) {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("private");
        let root = temp.path().join("library");
        let mut core = Core::new(&data).unwrap();
        core.create_library(&root).unwrap();
        let document = core.create_note_fixture(&root, &root, "Note").unwrap();
        (temp, core, root, data, document)
    }
    #[test]
    fn original_file_fact_survives_postcommit_draft_failure_and_lost_session() {
        let (_temp, mut core, _root, data, doc) = fixture();
        let id = fresh();
        let source = format!("{}\nafter", doc.source);
        hosted::set_test_failpoint("saveDraftUpdate");
        let outcome = core
            .save(SaveRequest {
                session_id: doc.session_id.clone(),
                base_id: doc.base_id,
                source: source.clone(),
                operation_id: id.clone(),
            })
            .unwrap();
        assert_eq!(outcome.recovery_issue.unwrap().code, "injectedFailure");
        assert_eq!(fs::read_to_string(&doc.path).unwrap(), source);
        assert_eq!(
            core.sessions[&doc.session_id].base_id,
            outcome.receipt.base_id
        );
        let stored: journal::Plan =
            serde_json::from_slice(&fs::read(core.journal_path(&id, "plan")).unwrap()).unwrap();
        assert_eq!(stored.receipt.unwrap().base_id, outcome.receipt.base_id);
        // Delivery hints are not another durable receipt format.
        assert!(
            !String::from_utf8(fs::read(core.journal_path(&id, "plan")).unwrap())
                .unwrap()
                .contains("recoveryIssue")
        );
        let next = core
            .save(SaveRequest {
                session_id: doc.session_id,
                base_id: outcome.receipt.base_id.clone(),
                source: format!("{source}\nnext"),
                operation_id: fresh(),
            })
            .unwrap();
        assert!(next.recovery_issue.is_none());
        drop(core);
        let mut core = Core::new(data).unwrap();
        assert!(core.sessions.is_empty());
        let fact = core.local_operation_status(&id).unwrap();
        assert_eq!(fact["status"], "committed");
        assert_eq!(fact["baseId"], outcome.receipt.base_id);
        let (committed_source, current) = core.reopen_local_save(&id).unwrap();
        assert_eq!(committed_source, source);
        assert_ne!(current.source, committed_source);
    }
    #[test]
    fn committed_original_wins_but_uncommitted_resolution_errors_block_mutation() {
        let (_temp, mut core, root, _data, _doc) = fixture();
        let id = fresh();
        let preview = core
            .prepare_operation(OperationRequest {
                operation_id: id.clone(),
                root: display(&root),
                intent: OperationIntent::NewNode {
                    parent: display(&root),
                    name: "After".into(),
                    source: None,
                },
            })
            .unwrap();
        fs::write(core.journal_path(&id, "resolution"), b"{broken").unwrap();
        assert_eq!(
            core.execute_operation(&id, &preview.plan_id)
                .unwrap_err()
                .code,
            "resolutionUnavailable"
        );
        assert!(!root.join("After").exists());
        // Use another original, and corrupt only secondary delivery metadata
        // after Core has loaded and durably committed it.
        let (_committed_temp, mut core, root, _data, doc) = fixture();
        let committed = core.create_note(&root, &root, "Committed").unwrap();
        fs::write(
            core.journal_path(&committed.operation_id, "resolution"),
            b"{broken",
        )
        .unwrap();
        assert_eq!(
            core.execute_operation(&committed.operation_id, &committed.plan_id)
                .unwrap()
                .node_ids,
            committed.node_ids
        );
        assert_eq!(
            core.local_operation_status(&committed.operation_id)
                .unwrap()["status"],
            "committed"
        );
        assert_eq!(fs::read_to_string(doc.path).unwrap(), doc.source);
    }
    #[test]
    fn missing_attempt_fence_is_durable_and_precedes_save_draft_effects() {
        let (_temp, mut core, root, data, doc) = fixture();
        let id = fresh();
        assert_eq!(
            core.local_operation_status(&id).unwrap()["status"],
            "unknown"
        );
        assert_eq!(
            core.settle_local_attempt(&id, "save", &hash(b"frozen request"))
                .unwrap()["status"],
            "resolved"
        );
        assert!(!core.operations.contains_key(&id));
        let before = fs::read(&doc.path).unwrap();
        assert_eq!(
            core.save(SaveRequest {
                session_id: doc.session_id,
                base_id: doc.base_id,
                source: "different".into(),
                operation_id: id.clone()
            })
            .unwrap_err()
            .code,
            "operationCancelled"
        );
        assert_eq!(before, fs::read(&doc.path).unwrap());
        drop(core);
        let mut core = Core::new(data).unwrap();
        assert_eq!(
            core.local_operation_status(&id).unwrap()["status"],
            "resolved"
        );
        assert_eq!(
            core.create_note_with_id(&root, &root, "Delayed", &id)
                .unwrap_err()
                .code,
            "operationCancelled"
        );
        assert!(!root.join("Delayed").exists());
    }
    #[test]
    fn orphan_evidence_and_uuid_case_alias_cannot_be_declared_effect_free() {
        let (_temp, mut core, _root, _data, _doc) = fixture();
        let id = fresh();
        fs::write(
            core.journal_path(&id, "install"),
            b"retained unknown original",
        )
        .unwrap();
        assert_eq!(
            core.settle_local_attempt(&id, "operation", &hash(b"request"))
                .unwrap_err()
                .code,
            "resolutionUnavailable"
        );
        assert_eq!(
            core.settle_local_attempt(&id.to_uppercase(), "operation", &hash(b"request"))
                .unwrap_err()
                .code,
            "invalidOperationId"
        );
        assert!(!core.journal_path(&id, "resolution").exists());
    }
    #[test]
    fn new_attempt_fence_has_no_prose_and_historical_decision_bytes_stay_exact() {
        let (_temp, mut core, _root, _data, _doc) = fixture();
        let id = fresh();
        let status = core
            .settle_local_attempt(&id, "operation", &hash(b"exact original wire"))
            .unwrap();
        assert_eq!(status["status"], "resolved");
        let fence_bytes = fs::read(core.journal_path(&id, "resolution")).unwrap();
        let fence: serde_json::Value = serde_json::from_slice(&fence_bytes).unwrap();
        assert!(!fence.as_object().unwrap().contains_key("message"));
        assert_eq!(fence["attempt"]["wireDigest"], hash(b"exact original wire"));
        let original_id = fresh();
        let original = format!(
            "{{\n  \"operationId\":\"{original_id}\",\n  \"action\":\"cancelPrepared\",\n  \"message\":\"Original retained prose\"\n}}\n"
        );
        let path = core.journal_path(&original_id, "resolution");
        fs::write(&path, original.as_bytes()).unwrap();
        assert_eq!(
            core.local_operation_status(&original_id).unwrap()["status"],
            "resolved"
        );
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        let record = journal::decode_resolution(&original_id, original.as_bytes()).unwrap();
        assert_eq!(
            record.historical_message.as_deref(),
            Some("Original retained prose")
        );
    }

    #[test]
    fn strict_decision_reader_rejects_invalid_identity_action_duplicates_and_budget() {
        let id = fresh();
        let valid =
            serde_json::json!({"operationId":id,"action":"cancelPrepared","message":"retained"});
        assert!(journal::decode_resolution(&id, &serde_json::to_vec(&valid).unwrap()).is_ok());
        for invalid in [
            serde_json::json!({"operationId":fresh(),"action":"cancelPrepared","message":"retained"}),
            serde_json::json!({"operationId":id,"action":"madeUp","message":"retained"}),
            serde_json::json!({"operationId":id,"action":"cancelPrepared","message":"retained","extra":true}),
        ] {
            assert!(
                journal::decode_resolution(&id, &serde_json::to_vec(&invalid).unwrap()).is_err()
            );
        }
        assert!(journal::decode_resolution(&id,format!(r#"{{"operationId":"{id}","action":"cancelPrepared","action":"keepCurrent","message":"retained"}}"#).as_bytes()).is_err());
        assert!(journal::decode_resolution(&id, &vec![b' '; 4097]).is_err());
        let (_temp, core, _root, _data, _doc) = fixture();
        fs::write(
            core.journal_path(&id, "resolution"),
            serde_json::to_vec(
                &serde_json::json!({"operationId":id,"action":"keepCurrent","message":"retained"}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            core.resolved(&id).unwrap_err().code,
            "resolutionUnavailable"
        );
    }
    #[test]
    fn preplan_reservation_requires_its_original_operation_and_owner() {
        let (_temp, mut core, root, _data, doc) = fixture();
        let original = fresh();
        let basis = core
            .read_snapshot(&root, std::slice::from_ref(&doc.path))
            .unwrap()
            .files
            .remove(0);
        core.reserve_definition_key(&root, &doc.path, &basis.base_id, &original)
            .unwrap();
        let copied = fresh();
        fs::copy(
            core.journal_path(&original, "definition-reservation"),
            core.journal_path(&copied, "definition-reservation"),
        )
        .unwrap();
        assert_eq!(
            core.settle_local_attempt(&copied, "save", &hash(b"request"))
                .unwrap_err()
                .code,
            "resolutionUnavailable"
        );
        assert!(!core.journal_path(&copied, "resolution").exists());
        assert_eq!(
            core.settle_local_attempt(&original, "save", &hash(b"request"))
                .unwrap()["status"],
            "resolved"
        );
    }
    #[test]
    fn new_global_fact_and_settlement_are_local_owner_only() {
        let (_temp, mut core, _root, _data, _doc) = fixture();
        let id = fresh();
        core.with_actor("alice", |c| {
            assert!(c.local_operation_status(&id).is_err());
            assert!(
                c.settle_local_attempt(&id, "save", &hash(b"request"))
                    .is_err()
            );
            Ok(())
        })
        .unwrap();
        assert!(!core.journal_path(&id, "resolution").exists());
    }
    #[test]
    fn sync_setup_status_is_its_original_decision_without_current_binding_or_library_reads() {
        let (temp, mut core, root, _data, _doc) = fixture();
        let id = fresh();
        let setup = core.enable_sync(&root, &id).unwrap();
        let fact = core.local_operation_status(&id).unwrap();
        assert_eq!(fact["status"], "committed");
        assert_eq!(fact["kind"], "enableSync");
        assert_eq!(fact["replicaId"], setup.replica_id);
        assert_eq!(
            fact["identity"],
            serde_json::to_value(&setup.identity).unwrap()
        );
        // Lose the live root entirely. Status still reports only the original
        // immutable setup, never a guessed latest sync binding.
        fs::rename(&root, temp.path().join("retained-library")).unwrap();
        core.sessions.clear();
        assert_eq!(core.local_operation_status(&id).unwrap(), fact);
        core.operations
            .get_mut(&id)
            .unwrap()
            .sync_setup
            .as_mut()
            .unwrap()
            .root = "different".into();
        assert!(core.local_operation_status(&id).is_err());
    }
    #[test]
    fn unknown_kinds_and_corrupt_commit_evidence_are_errors_not_pending_or_resolved() {
        let (_temp, mut core, _root, _data, doc) = fixture();
        let id = fresh();
        core.save(SaveRequest {
            operation_id: id.clone(),
            session_id: doc.session_id,
            base_id: doc.base_id,
            source: doc.source,
        })
        .unwrap();
        let original = fs::read(core.journal_path(&id, "commit")).unwrap();
        fs::write(
            core.data_dir.join("retained-original-commit.json"),
            &original,
        )
        .unwrap();
        fs::write(core.journal_path(&id, "commit"), b"{broken").unwrap();
        assert!(core.local_operation_status(&id).is_err());
        fs::write(core.journal_path(&id, "commit"), original).unwrap();
        core.operations.get_mut(&id).unwrap().kind = "unknown-kind".into();
        assert!(core.local_operation_status(&id).is_err());
    }
}
