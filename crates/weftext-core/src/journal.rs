use super::*;

const MAX_RESOLUTION_BYTES: u64 = 4096;

/// A decision in the original journal, never an author participant or a second
/// operation ledger. A missing Plan can only have an attempt cancellation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolutionRecord {
    pub operation_id: String,
    pub action: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempt: Option<AttemptFence>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AttemptFence {
    pub request_kind: String,
    pub wire_digest: String,
}

pub(crate) fn decode_resolution(id: &str, bytes: &[u8]) -> Result<ResolutionRecord> {
    valid_operation_id(id)?;
    if bytes.len() as u64 > MAX_RESOLUTION_BYTES {
        return Err(CoreError::new(
            "resolutionUnavailable",
            "Resolution metadata exceeds its bounded budget",
        ));
    }
    let record: ResolutionRecord = serde_json::from_slice(bytes).map_err(|_| {
        CoreError::new(
            "resolutionUnavailable",
            "Original resolution metadata is invalid; preserve it without retrying the operation",
        )
    })?;
    if record.operation_id != id
        || !matches!(
            record.action.as_str(),
            "keepCurrent" | "createRejected" | "cancelPrepared"
        )
        || record.message.len() > 2048
        || record.attempt.as_ref().is_some_and(|attempt| {
            record.action != "cancelPrepared"
                || !matches!(
                    attempt.request_kind.as_str(),
                    "save"
                        | "saveAs"
                        | "newNode"
                        | "capture"
                        | "period"
                        | "operation"
                        | "createLibrary"
                )
                || attempt.wire_digest.len() != 64
                || !attempt.wire_digest.bytes().all(|b| b.is_ascii_hexdigit())
        })
    {
        return Err(CoreError::new(
            "resolutionUnavailable",
            "Resolution identity, action or request binding is invalid",
        ));
    }
    Ok(record)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Plan {
    pub operation_id: String,
    pub fingerprint: String,
    pub path: String,
    pub kind: String,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
    pub identity: Option<FileIdentity>,
    pub receipt: Option<Receipt>,
    pub files: Vec<(String, Vec<u8>)>,
    pub node_id: Option<String>,
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub transaction: Option<operations::FrozenOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub history_restore: Option<history::RestoreBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_setup: Option<sync::replica::SyncSetup>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_publication: Option<sync::replica::SyncPublication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_application: Option<sync::apply::SyncApplication>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_revision: Option<String>,
}
impl Plan {
    pub fn file(
        operation_id: String,
        fingerprint: String,
        path: &Path,
        before: Option<Vec<u8>>,
        after: Vec<u8>,
        identity: Option<FileIdentity>,
        kind: &str,
    ) -> Self {
        Self {
            operation_id,
            fingerprint,
            path: display(path),
            before,
            after,
            identity,
            kind: kind.into(),
            receipt: None,
            files: Vec::new(),
            node_id: None,
            workspace_id: None,
            transaction: None,
            history_restore: None,
            sync_setup: None,
            sync_publication: None,
            sync_application: None,
            content_revision: None,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub operation_id: String,
    pub path: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_source: Option<String>,
    pub after_source: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_source: Option<String>,
    pub coherence: String,
    pub members: Vec<RecoveryMember>,
    pub allowed_actions: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryBlob {
    pub present: bool,
    pub byte_length: usize,
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}
impl RecoveryBlob {
    pub(crate) fn from_bytes(bytes: Option<&[u8]>) -> Self {
        let Some(bytes) = bytes else {
            return Self {
                present: false,
                byte_length: 0,
                sha256: None,
                source: None,
                base64: None,
            };
        };
        let source = String::from_utf8(bytes.to_vec()).ok();
        Self {
            present: true,
            byte_length: bytes.len(),
            sha256: Some(hash(bytes)),
            base64: source.is_none().then(|| encode_base64(bytes)),
            source,
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryIdentity {
    pub volume: u32,
    pub index: String,
    pub created: String,
}
impl From<&FileIdentity> for RecoveryIdentity {
    fn from(identity: &FileIdentity) -> Self {
        Self {
            volume: identity.volume,
            index: identity.index.to_string(),
            created: identity.created.to_string(),
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryMember {
    pub kind: String,
    pub role: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_path: Option<String>,
    pub before: RecoveryBlob,
    pub after: RecoveryBlob,
    pub current: RecoveryBlob,
    pub state: String,
    pub matches_before: bool,
    pub matches_after: bool,
    pub identity_binding: String,
    pub expected_identity: Option<RecoveryIdentity>,
    pub current_identity: Option<RecoveryIdentity>,
    pub message: String,
}
fn encode_base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut encoded = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let bits = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        encoded.push(ALPHABET[((bits >> 18) & 63) as usize] as char);
        encoded.push(ALPHABET[((bits >> 12) & 63) as usize] as char);
        encoded.push(if chunk.len() > 1 {
            ALPHABET[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        encoded.push(if chunk.len() > 2 {
            ALPHABET[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    encoded
}
impl Core {
    pub(crate) fn journal_path(&self, id: &str, suffix: &str) -> PathBuf {
        self.data_dir
            .join("journal")
            .join(format!("{id}.{suffix}.json"))
    }
    pub(crate) fn load_journals(&mut self) -> Result<()> {
        for entry in fs::read_dir(self.data_dir.join("journal"))? {
            let path = entry?.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".plan.json"))
            {
                let bytes = fs::read(&path)?;
                let plan: Plan = serde_json::from_slice(&bytes)?;
                valid_operation_id(&plan.operation_id)?;
                if path.file_name().and_then(|n| n.to_str())
                    != Some(format!("{}.plan.json", plan.operation_id).as_str())
                {
                    return Err(CoreError::new(
                        "resolutionUnavailable",
                        "Journal Plan filename and original identity differ",
                    ));
                }
                if plan.kind == "cancelLibraryCreation" && !library_creation::is_cancellation(&plan)
                {
                    return Err(CoreError::new(
                        "resolutionUnavailable",
                        "A retained library cancellation has author participants or an invalid request",
                    ));
                }
                self.operations.insert(plan.operation_id.clone(), plan);
            }
        }
        self.validate_resolution_namespace()
    }
    pub(crate) fn stage(&mut self, mut plan: Plan) -> Result<()> {
        if !self.operations.contains_key(&plan.operation_id) && self.resolved(&plan.operation_id)? {
            return Err(CoreError::new(
                "operationCancelled",
                "The original operation was cancelled before preparation",
            ));
        }
        self.validate_operation_pending(&plan)?;
        self.freeze_sync_publication(&mut plan)?;
        // A plan contains complete exact preimage/afterimage before any user write.
        private_create(
            &self.journal_path(&plan.operation_id, "plan"),
            &serde_json::to_vec(&plan)?,
        )?;
        self.operations.insert(plan.operation_id.clone(), plan);
        Ok(())
    }
    pub(crate) fn commit(&self, plan: &Plan) -> Result<()> {
        // One Core instance owns this directory. Sequence records the durable
        // decision order independently of the display clock (which may move
        // backwards); legacy records deliberately have neither field.
        let sequence = self
            .operations
            .values()
            .filter_map(|p| {
                let bytes = fs::read(self.journal_path(&p.operation_id, "commit")).ok()?;
                let fact: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
                (fact["operationId"] == p.operation_id
                    && fact["fingerprint"] == p.fingerprint
                    && fact["storageProfile"] == STORAGE_PROFILE)
                    .then(|| fact["commitSequence"].as_u64())
                    .flatten()
            })
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| CoreError::new("invalidState", "Commit sequence exhausted"))?;
        let saved_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|d| u64::try_from(d.as_millis()).ok());
        private_create(
            &self.journal_path(&plan.operation_id, "commit"),
            &serde_json::to_vec(
                &serde_json::json!({"operationId":plan.operation_id,"fingerprint":plan.fingerprint,"receipt":plan.receipt,"storageProfile":STORAGE_PROFILE,"savedAtUnixMs":saved_at,"commitSequence":sequence}),
            )?,
        )
    }
    pub(crate) fn committed(&self, id: &str) -> bool {
        let Some(plan) = self.operations.get(id) else {
            return false;
        };
        fs::read(self.journal_path(id, "commit"))
            .inspect_err(|error| resource_diagnostics::io_error("committed.read", id, error))
            .ok()
            .and_then(|b| {
                serde_json::from_slice::<serde_json::Value>(&b)
                    .inspect_err(|_| {
                        resource_diagnostics::branch("committed.parse", id, "invalidState")
                    })
                    .ok()
            })
            .is_some_and(|c| {
                let matches = c["operationId"] == id
                    && c["fingerprint"] == plan.fingerprint
                    && c["storageProfile"] == STORAGE_PROFILE;
                if !matches {
                    resource_diagnostics::branch("committed.match", id, "recordMismatch");
                }
                matches
            })
    }
    pub(crate) fn resolution_record(&self, id: &str) -> Result<Option<ResolutionRecord>> {
        valid_operation_id(id)?;
        let mut file = match File::open(self.journal_path(id, "resolution")) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_RESOLUTION_BYTES + 1)
            .read_to_end(&mut bytes)?;
        let record = decode_resolution(id, &bytes)?;
        if !self.operations.contains_key(id) {
            if record.action != "cancelPrepared" {
                return Err(CoreError::new(
                    "resolutionUnavailable",
                    "An unregistered attempt cannot carry an author recovery decision",
                ));
            }
            self.validate_unregistered_fence_namespace(id)?;
        } else if record.attempt.is_some() {
            return Err(CoreError::new(
                "resolutionUnavailable",
                "An unregistered attempt fence cannot also own an author Plan",
            ));
        }
        Ok(Some(record))
    }
    pub(crate) fn validate_unregistered_fence_namespace(&self, id: &str) -> Result<()> {
        self.unregistered_attempt_library(id, true, &[]).map(|_| ())
    }
    /// Only typed allocations produced before an author Plan may accompany a
    /// missing-attempt decision. Body reads can be omitted for foreign backup
    /// routing; selected payloads are validated from their actual guarded file.
    pub(crate) fn unregistered_attempt_library(
        &self,
        id: &str,
        full: bool,
        protected: &[operations::CapturedFile],
    ) -> Result<Option<hosted::ActorLibraryBinding>> {
        valid_operation_id(id)?;
        let mut scope = None;
        for entry in fs::read_dir(self.data_dir.join("journal"))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name
                .to_str()
                .ok_or_else(|| CoreError::new("resolutionUnavailable", "Invalid journal leaf"))?;
            let prefix = format!("{id}.");
            if !name
                .get(..prefix.len())
                .is_some_and(|p| p.eq_ignore_ascii_case(&prefix))
            {
                continue;
            }
            let suffix = name
                .strip_prefix(&prefix)
                .and_then(|name| name.strip_suffix(".json"))
                .ok_or_else(|| {
                    CoreError::new(
                        "resolutionUnavailable",
                        "Journal alias or partial evidence is retained",
                    )
                })?;
            if suffix == "resolution" {
                continue;
            }
            if !matches!(
                suffix,
                "definition-reservation"
                    | "revision-reservation"
                    | "tag-reservation"
                    | "directory-reservation"
            ) {
                return Err(CoreError::new(
                    "resolutionUnavailable",
                    "An unregistered attempt has installation or unknown retained evidence",
                ));
            }
            let path = entry.path();
            let binding = if let Some(original) =
                protected.iter().find(|file| Path::new(&file.path) == path)
            {
                let mut cursor = std::io::Cursor::new(&original.bytes);
                match hosted::read_actor_library_header(&mut cursor)? {
                    hosted::ActorLibraryHeader::Bound(binding) => binding,
                    _ => {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            "Unbound reservation is preserved for inspection",
                        ));
                    }
                }
            } else {
                let mut file = File::open(&path)?;
                let binding = match hosted::read_actor_library_header(&mut file)? {
                    hosted::ActorLibraryHeader::Bound(binding) => binding,
                    _ => {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            "Unbound reservation is preserved for inspection",
                        ));
                    }
                };
                if full {
                    file.seek(SeekFrom::Start(0))?;
                    let mut bytes = Vec::new();
                    Read::by_ref(&mut file)
                        .take((8 * MAX_SOURCE_BYTES + 65537) as u64)
                        .read_to_end(&mut bytes)?;
                    if bytes.len() > 8 * MAX_SOURCE_BYTES + 65536 {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            "Reservation exceeds its bounded retained budget",
                        ));
                    }
                    if self.validate_preplan_reservation(id, suffix, &bytes)? != binding {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            "Reservation library header differs",
                        ));
                    }
                }
                binding
            };
            if scope.as_ref().is_some_and(|original| original != &binding) {
                return Err(CoreError::new(
                    "resolutionUnavailable",
                    "One attempt has competing original library scopes",
                ));
            }
            scope = Some(binding);
        }
        Ok(scope)
    }
    fn validate_resolution_namespace(&self) -> Result<()> {
        for entry in fs::read_dir(self.data_dir.join("journal"))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                CoreError::new("resolutionUnavailable", "Journal filename is invalid")
            })?;
            if let Some(id) = name.strip_suffix(".resolution.json") {
                self.resolution_record(id)?;
            }
        }
        Ok(())
    }
    pub(crate) fn resolved(&self, id: &str) -> Result<bool> {
        let resolution = self.resolution_record(id)?;
        Ok(resolution.is_some()
            || self
                .operations
                .get(id)
                .is_some_and(library_creation::is_cancellation))
    }
    pub(crate) fn resolve_record(&self, id: &str, action: &str) -> Result<()> {
        let path = self.journal_path(id, "resolution");
        if self.resolved(id)? {
            return Ok(());
        }
        if !self.operations.contains_key(id) {
            if action != "cancelPrepared" {
                return Err(CoreError::new(
                    "resolutionUnavailable",
                    "Only an attempt cancellation can terminate an unregistered operation",
                ));
            }
            self.validate_unregistered_fence_namespace(id)?;
        }
        private_create(
            &path,
            &serde_json::to_vec(&ResolutionRecord {
                operation_id: id.into(),
                action: action.into(),
                message: "Current files were preserved; original plan remains retained".into(),
                attempt: None,
            })?,
        )
    }
    pub(crate) fn check_pending(&self, path: &Path) -> Result<()> {
        self.check_pending_except(path, None)
    }
    pub(crate) fn check_pending_except(&self, path: &Path, except: Option<&str>) -> Result<()> {
        for p in self.operations.values() {
            if Some(p.operation_id.as_str()) == except {
                continue;
            }
            let pending_path = Path::new(&p.path);
            let a = display(pending_path).to_lowercase();
            let b = display(path).to_lowercase();
            let affects = a == b
                || (p.kind == "createLibrary" && b.starts_with(&(a + "\\")))
                || p.transaction.as_ref().is_some_and(|t| t.affects(path));
            let installing =
                p.transaction.is_none() || self.journal_path(&p.operation_id, "install").exists();
            if affects
                && installing
                && !self.committed(&p.operation_id)
                && !self.resolved(&p.operation_id)?
            {
                return Err(CoreError::new(
                    "recoveryPending",
                    format!(
                        "Operation {} is unresolved; inspect its before/after recovery record and explicitly keep the current file before reopening",
                        p.operation_id
                    ),
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn check_pending_identity(&self, identity: &FileIdentity) -> Result<()> {
        self.check_pending_identity_except(identity, None)
    }
    pub(crate) fn check_pending_identity_except(
        &self,
        identity: &FileIdentity,
        except: Option<&str>,
    ) -> Result<()> {
        for plan in self.operations.values() {
            if Some(plan.operation_id.as_str()) == except {
                continue;
            }
            if (plan.identity.as_ref() == Some(identity)
                || plan
                    .transaction
                    .as_ref()
                    .is_some_and(|t| t.has_identity(identity)))
                && (plan.transaction.is_none()
                    || self.journal_path(&plan.operation_id, "install").exists())
                && !self.committed(&plan.operation_id)
                && !self.resolved(&plan.operation_id)?
            {
                return Err(CoreError::new(
                    "recoveryPending",
                    "This file object has an unresolved operation; path aliases do not bypass recovery",
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn retry(&self, id: &str, fingerprint: &str) -> Result<Option<Receipt>> {
        if let Some(plan) = self.operations.get(id) {
            if plan.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    "This operationId already belongs to different input",
                ));
            }
            if self.committed(id) {
                return Ok(plan.receipt.clone());
            }
            return Err(CoreError::new(
                "recoveryPending",
                "The original operation remains retained; inspect it rather than executing again",
            ));
        }
        Ok(None)
    }
    pub fn recoveries(&self) -> Result<Vec<RecoveryInfo>> {
        let mut entries = Vec::new();
        for p in self.operations.values() {
            if !self.committed(&p.operation_id)
                && !self.resolved(&p.operation_id)?
                && (p.transaction.is_none()
                    || self.journal_path(&p.operation_id, "install").exists())
            {
                entries.push(self.inspect_recovery(&p.operation_id)?);
            }
        }
        entries.sort_by(|a, b| a.operation_id.cmp(&b.operation_id));
        Ok(entries)
    }
    fn recovery_info(
        plan: &Plan,
        status: &str,
        message: &str,
        current: Option<String>,
    ) -> RecoveryInfo {
        RecoveryInfo {
            operation_id: plan.operation_id.clone(),
            path: plan.path.clone(),
            status: status.into(),
            before_source: plan
                .before
                .as_ref()
                .and_then(|x| String::from_utf8(x.clone()).ok()),
            after_source: String::from_utf8(plan.after.clone()).unwrap_or_default(),
            message: message.into(),
            current_source: current,
            coherence: match status {
                "before" => "knownBefore",
                "after" => "knownAfter",
                "thirdState" => "thirdState",
                _ => "unavailable",
            }
            .into(),
            members: Vec::new(),
            allowed_actions: if matches!(status, "committed" | "resolved") {
                Vec::new()
            } else {
                vec!["keepCurrent".into()]
            },
        }
    }
    pub fn inspect_recovery(&self, operation_id: &str) -> Result<RecoveryInfo> {
        self.check_sync_operation_access(operation_id)?;
        valid_operation_id(operation_id)?;
        let Some(plan) = self.operations.get(operation_id) else {
            if self.resolved(operation_id)? {
                return Ok(RecoveryInfo {
                    operation_id: operation_id.into(),
                    path: String::new(),
                    status: "resolved".into(),
                    before_source: None,
                    after_source: String::new(),
                    message: "The original attempt is durably fenced without author participants"
                        .into(),
                    current_source: None,
                    coherence: "unavailable".into(),
                    members: Vec::new(),
                    allowed_actions: Vec::new(),
                });
            }
            return Err(CoreError::new(
                "operationMissing",
                "No retained operation has this ID",
            ));
        };
        if library_creation::is_cancellation(plan) {
            return Ok(Self::recovery_info(
                plan,
                "resolved",
                "The exact original creation request was durably canceled; no author path was inspected or activated",
                None,
            ));
        }
        if plan.kind == "createLibrary" && plan.transaction.is_some() {
            return self.inspect_library_creation_recovery(plan);
        }
        if plan.transaction.is_some() {
            return self.inspect_transaction_recovery(plan);
        }
        if self.committed(operation_id) {
            return Ok(Self::recovery_info(
                plan,
                "committed",
                "The durable original decision exists; this operation must not be executed again",
                None,
            ));
        }
        if self.resolved(operation_id)? {
            return Ok(Self::recovery_info(
                plan,
                "resolved",
                "The current file was explicitly retained; original before/after are preserved",
                None,
            ));
        }
        if plan.kind == "createLibrary" {
            return Ok(Self::recovery_info(
                plan,
                "pending",
                "An incomplete library creation is retained. No automatic activation, cleanup or rollback is performed",
                None,
            ));
        }
        let guard = match ParentGuard::acquire(Path::new(&plan.path)) {
            Ok(g) => g,
            Err(e) => return Ok(Self::recovery_info(plan, "unavailable", &e.message, None)),
        };
        let mut file = match guard.open_file(false, false) {
            Ok(f) => f,
            Err(e) if e.code == "missing" && plan.before.is_none() => {
                return Ok(Self::recovery_info(
                    plan,
                    "before",
                    "The create-only target is absent; the plan remains retained",
                    None,
                ));
            }
            Err(e) => return Ok(Self::recovery_info(plan, "unavailable", &e.message, None)),
        };
        let bytes = read_source(&mut file)?;
        let identity = stamp(&file)?.identity;
        let same_object = plan.identity.as_ref().is_none_or(|i| i == &identity);
        let status = if !same_object {
            "thirdState"
        } else if plan.before.as_ref() == Some(&bytes) {
            "before"
        } else if bytes == plan.after {
            "after"
        } else {
            "thirdState"
        };
        Ok(Self::recovery_info(
            plan,
            status,
            "Matching bytes are an observation, not proof that a crashed operation owns the current object. No automatic write is permitted",
            String::from_utf8(bytes).ok(),
        ))
    }
    pub fn resolve_recovery(&mut self, operation_id: &str, action: &str) -> Result<RecoveryInfo> {
        self.check_sync_operation_access(operation_id)?;
        valid_operation_id(operation_id)?;
        let plan = self.operations.get(operation_id).cloned().ok_or_else(|| {
            CoreError::new("operationMissing", "No retained operation has this ID")
        })?;
        if plan.kind == "createLibrary" && plan.transaction.is_some() {
            return self.resolve_library_creation_recovery(&plan, action);
        }
        if plan.transaction.is_some() {
            return self.resolve_transaction_recovery(&plan, action);
        }
        if action != "keepCurrent" {
            return Err(CoreError::new(
                "unsupportedRecovery",
                "This MVP only supports explicitly keeping current files; merge or save recovered text to a new file",
            ));
        }
        let inspected = self.inspect_recovery(operation_id)?;
        if inspected.status == "committed" {
            return Ok(inspected);
        }
        // Resolution records only; no path write, delete, rollback, retirement or re-execution.
        self.resolve_record(operation_id, action)?;
        let mut result = inspected;
        result.status = "resolved".into();
        result.message = "Current files retained. Full original before/after and the unresolved proposal remain in the private journal".into();
        Ok(result)
    }
    pub fn operation_status(&self, operation_id: &str) -> Result<RecoveryInfo> {
        self.inspect_recovery(operation_id)
    }
    pub(crate) fn read_birth(&self, id: &str) -> Result<Option<Birth>> {
        match fs::read(self.journal_path(id, "birth")) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub(crate) fn read_births(&self, id: &str) -> Result<Vec<Birth>> {
        let mut births = self.read_birth(id)?.into_iter().collect::<Vec<_>>();
        match fs::read(self.journal_path(id, "births")) {
            Ok(bytes) => births.extend(serde_json::from_slice::<Vec<Birth>>(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        Ok(births)
    }
}
pub(crate) fn private_create(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.pending", fresh()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    // Public record names appear only after the complete payload is flushed.
    backend::publish_private_create(&temporary, path)?;
    Ok(())
}
