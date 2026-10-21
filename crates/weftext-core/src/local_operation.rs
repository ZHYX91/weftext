//! Original journal facts and device-local cancellation. No second receipt store.
use super::*;
use journal::{AttemptFence, ResolutionRecord};

impl Core {
    pub(crate) fn validate_original_reservation_library(
        &self,
        binding: &hosted::ActorLibraryBinding,
    ) -> Result<String> {
        let invalid = || {
            CoreError::new(
                "resolutionUnavailable",
                "Reservation has no original committed library identity",
            )
        };
        let workspace = binding.workspace_id.as_ref().ok_or_else(invalid)?;
        valid_operation_id(workspace)?;
        let plan = self
            .operations
            .values()
            .find(|p| {
                matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                    && p.workspace_id.as_ref() == Some(workspace)
                    && self.committed(&p.operation_id)
            })
            .ok_or_else(invalid)?;
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
                "Unknown pre-Plan evidence cannot prove absence of author effects",
            )),
        }
    }
    pub(crate) fn check_attempt_not_cancelled(&self, id: &str) -> Result<()> {
        // A committed original wins over unrelated delivery metadata.
        if !self.committed(id) && self.resolved(id)? {
            return Err(CoreError::new(
                "operationCancelled",
                "This original attempt is durably terminated; no effects were retried",
            ));
        }
        Ok(())
    }
    /// Raw file-save fact reader. Hosted callers must first verify their actor,
    /// resource and original library binding; it needs no old live Session.
    pub(crate) fn file_save_fact(&self, id: &str) -> Result<Option<Receipt>> {
        valid_operation_id(id)?;
        let Some(plan) = self.operations.get(id) else {
            return Ok(None);
        };
        if !matches!(plan.kind.as_str(), "save" | "saveAs") || plan.transaction.is_some() {
            return Err(CoreError::new(
                "operationConflict",
                "The original operation is not a file save",
            ));
        }
        if self.committed(id) {
            return plan.receipt.clone().map(Some).ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    "Committed file save has no original receipt",
                )
            });
        }
        Ok(None)
    }
    /// Public global lookup is local-owner only. Actor APIs use their scoped
    /// records before accessing the same internal Plan and commit fact.
    pub fn local_operation_status(&self, id: &str) -> Result<serde_json::Value> {
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
        if let Some(t) = &plan.transaction {
            if self.committed(id) {
                return Ok(serde_json::to_value(&t.receipt)?);
            }
            return Ok(serde_json::to_value(self.operation_details(id)?)?);
        }
        if matches!(plan.kind.as_str(), "save" | "saveAs")
            && let Some(receipt) = self.file_save_fact(id)?
        {
            let mut fact = serde_json::to_value(receipt)?;
            fact["status"] = serde_json::json!("committed");
            return Ok(fact);
        }
        Ok(
            serde_json::json!({"operationId":id,"status":if self.resolved(id)? {"resolved"} else {"recoveryPending"}}),
        )
    }
    /// Independent current-document delivery; never re-saves source or needs
    /// the old Session. Stable original admission qualifies a moved owner.
    pub fn reopen_local_save(&mut self, id: &str) -> Result<(String, OpenedDocument)> {
        self.sync_local_owner()?;
        let receipt = self
            .file_save_fact(id)?
            .ok_or_else(|| CoreError::new("notCommitted", "No committed original file save"))?;
        let plan = self.operations[id].clone();
        let source = decode(&plan.after)?;
        let birth = plan
            .identity
            .as_ref()
            .and_then(|identity| self.admitted_binding(identity));
        let (path, root) = if let Some(birth) = &birth {
            let root = self
                .operations
                .values()
                .find(|plan| {
                    matches!(plan.kind.as_str(), "createLibrary" | "cloneSync")
                        && plan.workspace_id.as_ref() == Some(&birth.workspace_id)
                        && self.committed(&plan.operation_id)
                })
                .map(|plan| PathBuf::from(&plan.path))
                .ok_or_else(|| {
                    CoreError::new(
                        "committedResultUnavailable",
                        "Original library admission is unavailable",
                    )
                })?;
            let inventory = self.inventory(&root)?;
            let entry = inventory
                .entries
                .iter()
                .find(|entry| {
                    entry.identity_status == "admitted"
                        && entry.node_id.as_ref() == Some(&birth.node_id)
                })
                .ok_or_else(|| {
                    CoreError::new(
                        "committedResultUnavailable",
                        "Original saved owner has no current admitted location",
                    )
                })?;
            let qualified = self.qualified_node_at(&root, Path::new(&entry.path))?;
            if qualified.identity != birth.identity {
                return Err(CoreError::new(
                    "committedResultUnavailable",
                    "The current saved owner is a different physical object",
                ));
            }
            (PathBuf::from(&entry.path), Some(root))
        } else {
            (PathBuf::from(&receipt.path), None)
        };
        let document = self.open(&path, root)?;
        if let Some(identity) = &plan.identity
            && self.sessions[&document.session_id].stamp.identity != *identity
        {
            return Err(CoreError::new(
                "committedResultUnavailable",
                "The saved path now names another physical object",
            ));
        }
        Ok((source, document))
    }
    pub fn local_operation_root(&self, id: &str) -> Result<String> {
        self.sync_local_owner()?;
        valid_operation_id(id)?;
        self.operations
            .get(id)
            .and_then(|plan| plan.transaction.as_ref())
            .map(|t| t.request.root.clone())
            .ok_or_else(|| {
                CoreError::new(
                    "operationMissing",
                    "No original transaction root is retained",
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
            message:
                "The missing original attempt is durably fenced; no author effects were created"
                    .into(),
            attempt: Some(AttemptFence {
                request_kind: request_kind.into(),
                wire_digest: wire_digest.into(),
            }),
        };
        let bytes = serde_json::to_vec(&record)?;
        journal::decode_resolution(id, &bytes)?;
        self.validate_unregistered_fence_namespace(id)?;
        journal::private_create(&self.journal_path(id, "resolution"), &bytes)?;
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
}
