use super::*;
use weftext_messages::UiMessage;

const MAX_RESOLUTION_BYTES: u64 = 4096;

/// A decision in the original journal, never an author participant or a second
/// operation ledger. A missing Plan can only have an attempt cancellation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolutionRecord {
    pub operation_id: String,
    pub action: String,
    /// Original historical summary only. New decisions derive their UI
    /// description from typed action/facts and do not write display prose.
    #[serde(rename = "message", default, skip_serializing_if = "Option::is_none")]
    pub historical_message: Option<String>,
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
            UiMessage::error_core_journal_resolution_metadata_exceeds_bounded_budget(),
        ));
    }
    let record: ResolutionRecord = serde_json::from_slice(bytes).map_err(|error| {
        CoreError::new(
            "resolutionUnavailable",
            UiMessage::error_core_journal_original_resolution_metadata_invalid_preserve_it_without_retrying_operation(),
        ).diagnostic(DiagnosticOrigin::Json, error.to_string())
    })?;
    if record.operation_id != id
        || !matches!(
            record.action.as_str(),
            "keepCurrent" | "createRejected" | "cancelPrepared"
        )
        || record
            .historical_message
            .as_ref()
            .is_some_and(|text| text.len() > 2048)
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
            UiMessage::error_core_journal_resolution_identity_action_or_request_binding_invalid(),
        ));
    }
    Ok(record)
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Plan {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_save: Option<FileSaveData>,
    pub identity: Option<FileIdentity>,
    pub operation_id: String,
    pub fingerprint: String,
    pub path: String,
    pub kind: String,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
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
            file_save: None,
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
    pub description: CoreDescription,
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
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "platform", rename_all = "camelCase")]
pub enum RecoveryIdentity {
    Windows {
        volume: u32,
        index: String,
        created: String,
    },
    Linux {
        filesystem: [i32; 2],
        handle_type: i32,
        handle: Vec<u8>,
    },
}
impl From<&FileIdentity> for RecoveryIdentity {
    fn from(identity: &FileIdentity) -> Self {
        match identity {
            FileIdentity::Windows {
                volume,
                index,
                created,
            } => Self::Windows {
                volume: *volume,
                index: index.to_string(),
                created: created.to_string(),
            },
            FileIdentity::Linux {
                filesystem,
                handle_type,
                handle,
            } => Self::Linux {
                filesystem: *filesystem,
                handle_type: *handle_type,
                handle: handle.clone(),
            },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<CoreDescription>,
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CommitRecord {
    operation_id: String,
    fingerprint: String,
    receipt: Option<Receipt>,
    file_save: Option<FileSaveData>,
    storage_profile: String,
    #[serde(rename = "savedAtUnixMs")]
    _saved_at_unix_ms: Option<u64>,
    commit_sequence: u64,
}

fn decode_journal_plan(path: &Path, bytes: &[u8]) -> Result<Plan> {
    let plan: Plan = serde_json::from_slice(bytes)?;
    valid_operation_id(&plan.operation_id)?;
    if path.file_name().and_then(|n| n.to_str())
        != Some(format!("{}.plan.json", plan.operation_id).as_str())
    {
        return Err(CoreError::new(
            "resolutionUnavailable",
            UiMessage::error_core_journal_journal_plan_filename_and_original_identity_differ(),
        ));
    }
    if plan.kind == "cancelLibraryCreation" && !library_creation::is_cancellation(&plan) {
        return Err(CoreError::new("resolutionUnavailable",
            UiMessage::error_core_journal_retained_library_cancellation_has_author_participants_or_invalid_request()));
    }
    Ok(plan)
}

impl Core {
    pub(crate) fn journal_path(&self, id: &str, suffix: &str) -> PathBuf {
        self.data_dir
            .join("journal")
            .join(format!("{id}.{suffix}.json"))
    }
    pub(crate) fn load_journals(&mut self) -> Result<()> {
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let path = entry?.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".plan.json"))
            {
                let plan = self.read_journal_plan(&path)?;
                self.operations.insert(plan.operation_id.clone(), plan);
            }
        }
        // All committed decisions must have their original Plan, even if no
        // caller happens to query that operation during this process lifetime.
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let name = entry?.file_name();
            if let Some(id) = name
                .to_str()
                .and_then(|name| name.strip_suffix(".commit.json"))
            {
                self.committed_record(id)?;
            }
        }
        self.validate_resolution_namespace()
    }
    pub(crate) fn read_journal_plan(&self, path: &Path) -> Result<Plan> {
        let bytes = self.backend.read_bytes(path)?;
        decode_journal_plan(path, &bytes)
    }
    pub(crate) fn read_original_file_save_plan(&self, expected: &Plan) -> Result<Plan> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Metadata {
            file_save: FileSaveData,
            identity: FileIdentity,
        }
        let invalid = || {
            CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_save_plan_participants_or_receipt_invalid(),
            )
        };
        let path = self.journal_path(&expected.operation_id, "plan");
        let mut file = self.backend.open_read(&path)?;
        let (_, bytes) = hosted::read_qualified_private_record(&mut file, |file| {
            let fields = hosted::read_metadata_prefix(
                file,
                &[("fileSave", false), ("identity", false)],
                4096,
            )?
            .ok_or_else(invalid)?;
            let metadata: Metadata = hosted::decode_metadata(fields)?;
            if Some(&metadata.file_save) != expected.file_save.as_ref()
                || Some(&metadata.identity) != expected.identity.as_ref()
            {
                return Err(invalid());
            }
            self.validate_original_input_scope(&metadata.file_save.scope)?;
            Ok(Some(((), 8 * MAX_SOURCE_BYTES + 65536)))
        })?
        .ok_or_else(invalid)?;
        decode_journal_plan(&path, &bytes)
    }
    pub(crate) fn stage(&mut self, mut plan: Plan) -> Result<()> {
        if !self.operations.contains_key(&plan.operation_id) && self.resolved(&plan.operation_id)? {
            return Err(CoreError::new(
                "operationCancelled",
                UiMessage::error_core_journal_original_operation_was_cancelled_before_preparation(),
            ));
        }
        self.validate_operation_pending(&plan)?;
        self.freeze_sync_publication(&mut plan)?;
        // A plan contains complete exact preimage/afterimage before any user write.
        private_create(
            &self.backend,
            &self.journal_path(&plan.operation_id, "plan"),
            &serde_json::to_vec(&plan)?,
        )?;
        self.operations.insert(plan.operation_id.clone(), plan);
        Ok(())
    }
    fn committed_record(&self, id: &str) -> Result<Option<CommitRecord>> {
        valid_operation_id(id)?;
        let bytes = match self.backend.read_bytes(self.journal_path(id, "commit")) {
            Ok(bytes) => bytes,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(error) => return Err(error),
        };
        let original_plan = self.operations.get(id).ok_or_else(|| {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_journal_original_commit_evidence_invalid(),
            )
        })?;
        let record = decode_original_commit(original_plan, &bytes)?;
        Ok(Some(record))
    }
    pub(crate) fn commit(&self, plan: &Plan) -> Result<()> {
        let mut maximum = 0;
        for candidate in self.operations.values() {
            if let Some(record) = self.committed_record(&candidate.operation_id)? {
                maximum = maximum.max(record.commit_sequence);
            }
        }
        let sequence = maximum.checked_add(1).ok_or_else(|| {
            CoreError::new(
                "invalidState",
                UiMessage::error_core_journal_commit_sequence_exhausted(),
            )
        })?;
        let saved_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        private_create(
            &self.backend,
            &self.journal_path(&plan.operation_id, "commit"),
            &serde_json::to_vec(
                &serde_json::json!({"operationId":plan.operation_id,"fingerprint":plan.fingerprint,"receipt":plan.receipt,"fileSave":plan.file_save,"storageProfile":STORAGE_PROFILE,"savedAtUnixMs":saved_at,"commitSequence":sequence}),
            )?,
        )
    }
    pub(crate) fn commit_sequence(&self, id: &str) -> Result<u64> {
        self.committed_record(id)?
            .map(|record| record.commit_sequence)
            .ok_or_else(|| {
                CoreError::new(
                    "identityPending",
                    weftext_messages::UiMessage::error_core_backend_accepted_decision_missing(),
                )
            })
    }
    pub(crate) fn committed(&self, id: &str) -> Result<bool> {
        Ok(self.committed_record(id)?.is_some())
    }
    pub(crate) fn committed_plans(&self) -> Result<Vec<&Plan>> {
        self.backend.check_path(&self.data_dir)?;
        let mut plans = Vec::new();
        for plan in self.operations.values() {
            if self.committed(&plan.operation_id)? {
                plans.push(plan);
            }
        }
        Ok(plans)
    }
    pub(crate) fn resolution_record(&self, id: &str) -> Result<Option<ResolutionRecord>> {
        valid_operation_id(id)?;
        let mut file = match self.backend.open_read(self.journal_path(id, "resolution")) {
            Ok(file) => file,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(error) => return Err(error),
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
                    UiMessage::error_core_journal_unregistered_attempt_cannot_carry_author_recovery_decision(),
                ));
            }
            self.validate_unregistered_fence_namespace(id)?;
        } else if record.attempt.is_some() {
            return Err(CoreError::new(
                "resolutionUnavailable",
                UiMessage::error_core_journal_unregistered_attempt_fence_cannot_also_own_author_plan(),
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
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                CoreError::new(
                    "resolutionUnavailable",
                    UiMessage::error_core_journal_invalid_journal_leaf(),
                )
            })?;
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
                        UiMessage::error_core_journal_journal_alias_or_partial_evidence_retained(),
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
                    UiMessage::error_core_journal_unregistered_attempt_has_installation_or_unknown_retained_evidence(),
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
                            UiMessage::error_core_journal_unbound_reservation_preserved_for_inspection(),
                        ));
                    }
                }
            } else {
                let mut file = self.backend.open_read(&path)?;
                let binding = match hosted::read_actor_library_header(&mut file)? {
                    hosted::ActorLibraryHeader::Bound(binding) => binding,
                    _ => {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            UiMessage::error_core_journal_unbound_reservation_preserved_for_inspection(),
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
                            UiMessage::error_core_journal_reservation_exceeds_bounded_retained_budget(),
                        ));
                    }
                    if self.validate_preplan_reservation(id, suffix, &bytes)? != binding {
                        return Err(CoreError::new(
                            "resolutionUnavailable",
                            UiMessage::error_core_journal_reservation_library_header_differs(),
                        ));
                    }
                }
                binding
            };
            if scope.as_ref().is_some_and(|original| original != &binding) {
                return Err(CoreError::new(
                    "resolutionUnavailable",
                    UiMessage::error_core_journal_one_attempt_has_competing_original_library_scopes(
                    ),
                ));
            }
            scope = Some(binding);
        }
        Ok(scope)
    }
    fn validate_resolution_namespace(&self) -> Result<()> {
        for entry in self.backend.read_dir(self.data_dir.join("journal"))? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(|| {
                CoreError::new(
                    "resolutionUnavailable",
                    UiMessage::error_core_journal_journal_filename_invalid(),
                )
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
                    UiMessage::error_core_journal_only_attempt_cancellation_can_terminate_unregistered_operation(),
                ));
            }
            self.validate_unregistered_fence_namespace(id)?;
        }
        private_create(
            &self.backend,
            &path,
            &serde_json::to_vec(&ResolutionRecord {
                operation_id: id.into(),
                action: action.into(),
                historical_message: None,
                attempt: None,
            })?,
        )
    }
    pub(crate) fn check_pending(&self, path: &Path) -> Result<()> {
        self.backend.check_path(path)?;
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
            if !affects {
                continue;
            }
            let installing = p.transaction.is_none()
                || self
                    .backend
                    .entry_exists(self.journal_path(&p.operation_id, "install"))?;
            if installing && !self.committed(&p.operation_id)? && !self.resolved(&p.operation_id)? {
                return Err(CoreError::new(
                    "recoveryPending",
                    UiMessage::recovery_operation_pending(
                        weftext_messages::params::RecoveryOperationPending {
                            operation_id: p.operation_id.clone(),
                        },
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
                    || self
                        .backend
                        .entry_exists(self.journal_path(&plan.operation_id, "install"))?)
                && !self.committed(&plan.operation_id)?
                && !self.resolved(&plan.operation_id)?
            {
                return Err(CoreError::new(
                    "recoveryPending",
                    UiMessage::error_core_journal_file_object_has_unresolved_operation_path_aliases_do_not_bypass_recovery(),
                ));
            }
        }
        Ok(())
    }
    pub(crate) fn retry(&self, id: &str, fingerprint: &str) -> Result<Option<Receipt>> {
        if let Some(plan) = self.operations.get(id) {
            self.validate_file_save_plan(plan)?;
            if plan.fingerprint != fingerprint {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_journal_operation_id_already_belongs_to_different_input(),
                ));
            }
            if self.committed(id)? {
                return Ok(plan.receipt.clone());
            }
            return Err(CoreError::new(
                "recoveryPending",
                UiMessage::error_core_journal_original_operation_remains_retained_inspect_it_rather_than_executing_again(),
            ));
        }
        Ok(None)
    }
    pub fn recoveries(&self) -> Result<Vec<RecoveryInfo>> {
        self.backend.check_path(&self.data_dir)?;
        let mut entries = Vec::new();
        for p in self.operations.values() {
            if !self.committed(&p.operation_id)?
                && !self.resolved(&p.operation_id)?
                && (p.transaction.is_none()
                    || self
                        .backend
                        .entry_exists(self.journal_path(&p.operation_id, "install"))?)
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
        description: impl Into<CoreDescription>,
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
            description: description.into(),
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
        self.backend.check_path(&self.data_dir)?;
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
                    description: UiMessage::recovery_attempt_fenced().into(),
                    current_source: None,
                    coherence: "unavailable".into(),
                    members: Vec::new(),
                    allowed_actions: Vec::new(),
                });
            }
            return Err(CoreError::new(
                "operationMissing",
                UiMessage::error_core_journal_no_retained_operation_has_i_d(),
            ));
        };
        if library_creation::is_cancellation(plan) {
            return Ok(Self::recovery_info(
                plan,
                "resolved",
                UiMessage::recovery_creation_canceled(),
                None,
            ));
        }
        if plan.kind == "createLibrary" && plan.transaction.is_some() {
            return self.inspect_library_creation_recovery(plan);
        }
        if plan.transaction.is_some() {
            return self.inspect_transaction_recovery(plan);
        }
        if self.committed(operation_id)? {
            return Ok(Self::recovery_info(
                plan,
                "committed",
                UiMessage::recovery_original_decision_exists(),
                None,
            ));
        }
        if self.resolved(operation_id)? {
            return Ok(Self::recovery_info(
                plan,
                "resolved",
                UiMessage::recovery_current_file_retained(),
                None,
            ));
        }
        if plan.kind == "createLibrary" {
            return Ok(Self::recovery_info(
                plan,
                "pending",
                UiMessage::recovery_library_creation_incomplete(),
                None,
            ));
        }
        let guard = match ParentGuard::acquire(&self.backend, Path::new(&plan.path)) {
            Ok(g) => g,
            Err(e) => return Ok(Self::recovery_info(plan, "unavailable", e, None)),
        };
        let mut file = match guard.open_file(false, false) {
            Ok(f) => f,
            Err(e) if e.code == "missing" && plan.before.is_none() => {
                return Ok(Self::recovery_info(
                    plan,
                    "before",
                    UiMessage::recovery_create_target_absent(),
                    None,
                ));
            }
            Err(e) => return Ok(Self::recovery_info(plan, "unavailable", e, None)),
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
            UiMessage::recovery_bytes_observation_only(),
            String::from_utf8(bytes).ok(),
        ))
    }
    pub fn resolve_recovery(&mut self, operation_id: &str, action: &str) -> Result<RecoveryInfo> {
        self.check_sync_operation_access(operation_id)?;
        valid_operation_id(operation_id)?;
        let plan = self.operations.get(operation_id).cloned().ok_or_else(|| {
            CoreError::new(
                "operationMissing",
                UiMessage::error_core_journal_no_retained_operation_has_i_d(),
            )
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
                UiMessage::error_core_journal_m_v_p_only_supports_explicitly_keeping_current_files_merge_or_save_recovered_text_to_new_file(),
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
        result.description = UiMessage::recovery_current_files_retained().into();
        Ok(result)
    }
    pub fn operation_status(&self, operation_id: &str) -> Result<RecoveryInfo> {
        self.inspect_recovery(operation_id)
    }
    pub(crate) fn read_birth(&self, id: &str) -> Result<Option<Birth>> {
        match self.backend.read_bytes(self.journal_path(id, "birth")) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.code == "missing" => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn read_births(&self, id: &str) -> Result<Vec<Birth>> {
        let mut births = self.read_birth(id)?.into_iter().collect::<Vec<_>>();
        match self.backend.read_bytes(self.journal_path(id, "births")) {
            Ok(bytes) => births.extend(serde_json::from_slice::<Vec<Birth>>(&bytes)?),
            Err(e) if e.code == "missing" => {}
            Err(e) => return Err(e),
        }
        if let Some(plan) = self.operations.get(id) {
            let mut expected = std::collections::BTreeSet::new();
            if plan.kind == "newNode" {
                if let Some(node) = &plan.node_id {
                    expected.insert(node.clone());
                } else {
                    return Err(CoreError::new(
                        "identityPending",
                        weftext_messages::UiMessage::error_core_backend_committed_node_identity_missing(),
                    ));
                }
            }
            if let Some(transaction) = &plan.transaction {
                expected.extend(
                    transaction
                        .births()
                        .iter()
                        .map(|birth| birth.node_id.clone()),
                );
            }
            if !expected.is_empty() && self.committed(id)? {
                let actual: std::collections::BTreeSet<_> =
                    births.iter().map(|birth| birth.node_id.clone()).collect();
                if actual != expected
                    || births.len() != expected.len()
                    || births
                        .iter()
                        .any(|birth| Some(&birth.workspace_id) != plan.workspace_id.as_ref())
                {
                    return Err(CoreError::new(
                        "identityPending",
                        weftext_messages::UiMessage::error_core_backend_committed_birth_incomplete(
                        ),
                    ));
                }
            }
        }
        Ok(births)
    }
}
pub(crate) fn validate_original_commit_bytes(plan: &Plan, bytes: &[u8]) -> Result<()> {
    decode_original_commit(plan, bytes).map(|_| ())
}
fn decode_original_commit(plan: &Plan, bytes: &[u8]) -> Result<CommitRecord> {
    let invalid = || {
        CoreError::new(
            "invalidState",
            UiMessage::error_core_journal_original_commit_evidence_invalid(),
        )
    };
    if bytes.len() > 65536 {
        return Err(invalid());
    }
    let fact: CommitRecord = serde_json::from_slice(bytes)
        .map_err(|error| invalid().caused_by(CoreError::from(error)))?;
    if fact.operation_id != plan.operation_id
        || fact.fingerprint != plan.fingerprint
        || fact.storage_profile != STORAGE_PROFILE
        || serde_json::to_value(&fact.receipt)? != serde_json::to_value(&plan.receipt)?
        || serde_json::to_value(&fact.file_save)? != serde_json::to_value(&plan.file_save)?
        || fact.commit_sequence == 0
    {
        return Err(invalid());
    }
    Ok(fact)
}
pub(crate) fn private_create(
    backend: &crate::BackendContext,
    path: &Path,
    bytes: &[u8],
) -> Result<()> {
    let temporary = path.with_extension(format!("{}.pending", fresh()));
    let guard = ParentGuard::acquire(backend, &temporary)?;
    let mut file = guard.open_file(true, true)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    // Public record names appear only after the complete payload is flushed.
    backend::publish_private_create(backend, &temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod description_tests {
    use super::*;

    fn original_file_commit_plan() -> Plan {
        let mut plan = Plan::file(
            fresh(),
            hash(b"original frozen file intent"),
            Path::new("ordinary.typ"),
            None,
            "author 原文\0😀".as_bytes().to_vec(),
            None,
            "save",
        );
        plan.file_save = Some(FileSaveData::new(
            fresh(),
            Some(fresh()),
            QualifiedInputScope::DeviceUnassigned,
            fresh(),
        ));
        plan
    }

    fn original_commit_json(plan: &Plan) -> serde_json::Value {
        serde_json::json!({
            "operationId": plan.operation_id,
            "fingerprint": plan.fingerprint,
            "receipt": plan.receipt,
            "fileSave": plan.file_save,
            "storageProfile": STORAGE_PROFILE,
            "savedAtUnixMs": 1234,
            "commitSequence": 1,
        })
    }

    #[test]
    fn malformed_original_commit_keeps_its_raw_json_cause_and_frozen_author_input() {
        let plan = original_file_commit_plan();
        let retained_plan = serde_json::to_vec(&plan).unwrap();
        let bytes = b"{ malformed original commit";
        let expected_raw = serde_json::from_slice::<serde_json::Value>(bytes)
            .unwrap_err()
            .to_string();
        let error = validate_original_commit_bytes(&plan, bytes).unwrap_err();
        assert_eq!(error.code, "invalidState");
        assert_eq!(
            error.explanation,
            UiMessage::error_core_journal_original_commit_evidence_invalid()
        );
        assert!(error.diagnostics.is_empty());
        let causes = error.causes.as_slice();
        assert_eq!(causes.len(), 1);
        assert_eq!(causes[0].code, "invalidState");
        assert_eq!(causes[0].explanation, UiMessage::error_core_json_failure());
        assert_eq!(
            causes[0].diagnostics,
            vec![RawDiagnostic {
                origin: DiagnosticOrigin::Json,
                text: expected_raw,
            }]
        );
        let wire = serde_json::to_value(&error).unwrap();
        assert!(wire.get("message").is_none());
        let _ = error.explanation.render(weftext_messages::UiLocale::ZhCn);
        let _ = error.explanation.render(weftext_messages::UiLocale::EnUs);
        assert_eq!(serde_json::to_value(&error).unwrap(), wire);
        assert_eq!(serde_json::to_vec(&plan).unwrap(), retained_plan);
    }

    #[test]
    fn original_commit_scope_mismatch_and_budget_are_product_facts_without_fabricated_raw_text() {
        let plan = original_file_commit_plan();
        let retained_plan = serde_json::to_vec(&plan).unwrap();
        let original = original_commit_json(&plan);
        validate_original_commit_bytes(&plan, &serde_json::to_vec(&original).unwrap()).unwrap();

        let mut changed = original.clone();
        changed["fileSave"]["sessionId"] = fresh().into();
        let mismatch =
            validate_original_commit_bytes(&plan, &serde_json::to_vec(&changed).unwrap())
                .unwrap_err();
        assert_eq!(mismatch.code, "invalidState");
        assert_eq!(
            mismatch.explanation,
            UiMessage::error_core_journal_original_commit_evidence_invalid()
        );
        assert!(mismatch.diagnostics.is_empty());
        assert!(mismatch.causes.as_slice().is_empty());

        let oversized = vec![b' '; 65537];
        let budget = validate_original_commit_bytes(&plan, &oversized).unwrap_err();
        assert_eq!(budget.code, "invalidState");
        assert_eq!(budget.explanation, mismatch.explanation);
        assert!(budget.diagnostics.is_empty());
        assert!(budget.causes.as_slice().is_empty());
        assert_eq!(serde_json::to_vec(&plan).unwrap(), retained_plan);
    }

    #[test]
    fn invalid_resolution_keeps_original_json_diagnostic_separate_from_product_text() {
        let bytes = b"{ malformed original resolution";
        let error = decode_resolution(&fresh(), bytes).unwrap_err();
        assert_eq!(error.code, "resolutionUnavailable");
        assert_eq!(
            error.explanation.key(),
            "error.core.journal.originalResolutionMetadataInvalidPreserveItWithoutRetryingOperation"
        );
        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.origin == DiagnosticOrigin::Json
                    && !diagnostic.text.is_empty())
        );
        let wire = serde_json::to_value(&error).unwrap();
        assert!(wire.get("message").is_none());
        assert!(wire["explanation"].is_object());
    }
}
