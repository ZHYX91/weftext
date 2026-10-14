//! Hosted requests use the same guarded writer, with actor-owned draft and
//! operation records. This module grants no permissions: the server authorizes
//! configured resources before calling it, and again before disclosing results.
use super::*;
use std::collections::BTreeMap;

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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ActorOperation {
    actor: String,
    resource: String,
    fingerprint: String,
    core_operation_id: String,
    #[serde(default)]
    cancelled: bool,
}

impl Core {
    pub fn with_actor<T>(
        &mut self,
        actor: &str,
        action: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        if actor.is_empty() || actor.len() > 256 || self.actor.is_some() {
            return Err(CoreError::new(
                "invalidActor",
                "A non-nested authenticated actor scope is required",
            ));
        }
        let root = self.data_dir.join("actors").join(hash(actor.as_bytes()));
        for directory in ["drafts", "draft-history", "operations"] {
            fs::create_dir_all(root.join(directory))?;
        }
        let _guards = ["drafts", "draft-history", "operations"]
            .map(|directory| ParentGuard::acquire(&root.join(directory).join(".scope")))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
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
                "Open the source in the current actor session",
            ));
        }
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
            return Err(CoreError::new("budget", "Policy exceeds 2 MiB"));
        }
        private_replace(&self.data_dir.join("hosted-policy.json"), bytes)
    }

    fn actor_record(&self, operation: &str) -> Result<PathBuf> {
        valid_operation_id(operation)?;
        if self.actor.is_none() {
            return Err(CoreError::new(
                "invalidActor",
                "Authenticated actor required",
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
        if self.actor.as_ref() != Some(&record.actor) {
            return Err(CoreError::new(
                "invalidActor",
                "Operation belongs to a different actor",
            ));
        }
        let saved = self.saved_operation_source(&record.core_operation_id)?;
        Ok(Some(ActorSaveStatus {
            operation_id: operation.into(),
            resource: record.resource,
            status: if record.cancelled {
                "cancelled"
            } else {
                saved.as_ref().map_or("notCommitted", |s| s.status.as_str())
            }
            .into(),
            base_id: saved.and_then(|s| s.receipt.map(|r| r.base_id)),
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
                actor: self.actor.clone().unwrap(),
                resource: resource.into(),
                fingerprint: String::new(),
                core_operation_id: fresh(),
                cancelled: false,
            },
            Err(e) => return Err(e.into()),
        };
        if record.resource != resource || self.actor.as_ref() != Some(&record.actor) {
            return Err(CoreError::new(
                "operationConflict",
                "Operation resource or actor differs",
            ));
        }
        if let Some(status) = self.actor_save_status(operation)?
            && status.status != "notCommitted"
        {
            return Ok(status);
        }
        record.cancelled = true;
        private_replace(&path, &serde_json::to_vec(&record)?)?;
        self.actor_save_status(operation)?
            .ok_or_else(|| CoreError::new("invalidState", "Missing settled record"))
    }
    pub fn actor_save(
        &mut self,
        resource: &str,
        request: HostedSaveRequest,
    ) -> Result<ActorSaveStatus> {
        let path = self.actor_record(&request.operation_id)?;
        check_size(request.source.as_bytes())?;
        let fingerprint = hash(&serde_json::to_vec(&serde_json::json!({
            "resource":resource,"session":request.session_id,"base":request.base_id,"source":request.source
        }))?);
        let record = match fs::read(&path) {
            Ok(bytes) => {
                let record: ActorOperation = serde_json::from_slice(&bytes)?;
                if record.cancelled {
                    return Err(CoreError::new(
                        "operationCancelled",
                        "The original request was durably cancelled; use an explicitly new proposal",
                    ));
                }
                if record.fingerprint != fingerprint {
                    return Err(CoreError::new(
                        "operationConflict",
                        "This operation already names a different proposal",
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
                let record = ActorOperation {
                    actor: self.actor.clone().unwrap(),
                    resource: resource.into(),
                    fingerprint,
                    core_operation_id: fresh(),
                    cancelled: false,
                };
                journal::private_create(&path, &serde_json::to_vec(&record)?)?;
                record
            }
            Err(e) => return Err(e.into()),
        };
        self.save(SaveRequest {
            session_id: request.session_id,
            base_id: request.base_id,
            source: request.source,
            operation_id: record.core_operation_id,
        })?;
        self.actor_save_status(&request.operation_id)?
            .ok_or_else(|| {
                CoreError::new("invalidState", "The original operation record is missing")
            })
    }

    /// Explicit allowlisted file set; no ambient tree enumeration or fallback.
    /// All handles stay guarded until the complete bounded snapshot is captured.
    pub fn hosted_files(&self, root: &Path, paths: &[String]) -> Result<BTreeMap<String, Vec<u8>>> {
        let root = absolute_path(root)?;
        if paths.len() > 64 {
            return Err(CoreError::new("budget", "At most 64 explicit inputs"));
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
                    "Explicit inputs must be relative child paths",
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
                return Err(CoreError::new("budget", "Hosted snapshot exceeds 32 MiB"));
            }
            files.insert(relative.clone(), bytes);
        }
        Ok(files)
    }
}
