//! A read-only projection of the existing commit, admission and exact-byte
//! journal. Restoring is an ordinary guarded save, never a chain rewind.
use super::*;
use operations::{CapturedFile, Mutation, RevisionFact};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryVersion {
    pub version_id: String,
    pub revision_id: Option<String>,
    pub operation_id: String,
    pub kind: String,
    pub saved_at_unix_ms: Option<u64>,
    pub time_basis: String,
    pub commit_sequence: Option<u64>,
    pub status: String,
    pub byte_length: Option<usize>,
    pub sha256: Option<String>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRetention {
    pub policy: String,
    pub complete: bool,
    pub known_versions: usize,
    pub available_versions: usize,
    pub oldest_saved_at_unix_ms: Option<u64>,
    pub newest_saved_at_unix_ms: Option<u64>,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryTimeline {
    pub node_id: String,
    pub workspace_id: String,
    pub path: String,
    pub current_base_id: String,
    pub current_saved_version_id: Option<String>,
    pub currentness: String,
    pub versions: Vec<HistoryVersion>,
    pub retention: HistoryRetention,
    pub protected_drafts: Vec<DraftAlternative>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRead {
    #[serde(flatten)]
    pub version: HistoryVersion,
    pub source: String,
    pub read_only: bool,
    pub preview_status: String,
    pub preview_message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryCurrentSource {
    pub version_id: String,
    pub revision_id: Option<String>,
    pub source: String,
    pub base_id: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryComparison {
    pub selected: HistoryRead,
    pub current: HistoryCurrentSource,
    pub equal: bool,
    pub comparison: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryRestoreRequest {
    pub session_id: String,
    pub base_id: String,
    pub version_id: String,
    pub operation_id: String,
    pub draft_action: String,
    pub draft_source: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RestoreBinding {
    pub fingerprint: String,
    pub version_id: String,
    pub draft_records: Vec<String>,
    pub node_id: String,
    pub workspace_id: String,
    pub identity: FileIdentity,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRestoreResult {
    pub receipt: Receipt,
    pub source: String,
    pub version_id: String,
    pub protected_drafts: Vec<DraftAlternative>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRestoreStatus {
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HistoryRestoreResult>,
}
struct HistoryContext {
    session: Session,
    birth: Birth,
    current: CapturedFile,
}
struct HistoryRecord {
    version: HistoryVersion,
    bytes: Option<Vec<u8>>,
    fact: Option<RevisionFact>,
}

impl Core {
    fn history_context(&self, session_id: &str) -> Result<HistoryContext> {
        self.check_actor_session(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the note again"))?;
        self.check_pending(&session.path)?;
        let guard = ParentGuard::acquire(&session.path)?;
        let mut handle = guard.open_file(false, false)?;
        let observed = stamp(&handle)?;
        self.check_pending_identity(&observed.identity)?;
        let bytes = read_source(&mut handle)?;
        let birth = self.admitted_binding(&observed.identity).ok_or_else(|| {
            CoreError::new(
                "historyUnavailable",
                "This source has no admitted stable note identity",
            )
        })?;
        if session.stamp.identity != observed.identity
            || session.node_id.as_ref() != Some(&birth.node_id)
            || !is_canonical(&session.path)
            || weftext_engine::inspect_metadata(&decode(&bytes)?)
                .id
                .as_ref()
                != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityPending",
                "The current note object or declaration needs identity recovery; history was not joined by name",
            ));
        }
        let root = self
            .operations
            .values()
            .find(|p| {
                matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                    && p.workspace_id.as_ref() == Some(&birth.workspace_id)
                    && self.committed(&p.operation_id)
            })
            .map(|p| PathBuf::from(&p.path))
            .ok_or_else(|| {
                CoreError::new(
                    "historyUnavailable",
                    "The original library admission is unavailable",
                )
            })?;
        if !session.path.starts_with(&root) {
            return Err(CoreError::new(
                "identityPending",
                "The note is outside its admitted library",
            ));
        }
        // Missing semantic configuration does not prevent exact source viewing.
        // Current discovery exclusions and current OS file access still apply.
        self.check_discovery_boundary(&root, session.path.parent().unwrap())?;
        self.check_history_placement(&birth, &session.path)?;
        Ok(HistoryContext {
            current: CapturedFile {
                path: display(&session.path),
                bytes,
                stamp: observed,
            },
            session,
            birth,
        })
    }

    fn check_history_placement(&self, birth: &Birth, current: &Path) -> Result<()> {
        let mut places = BTreeSet::<String>::new();
        let mut birth_sequence = None;
        let mut relocations = Vec::new();
        for p in self
            .operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
        {
            if self.read_births(&p.operation_id)?.iter().any(|b| {
                b.node_id == birth.node_id
                    && b.workspace_id == birth.workspace_id
                    && b.identity == birth.identity
            }) {
                birth_sequence = self.history_commit_time(p)?.1;
                if p.kind == "createNote" {
                    places.insert(p.path.clone());
                }
                if let Some(t) = &p.transaction
                    && let Mutation::CreateTree { births, .. } = &t.mutation
                {
                    places.extend(
                        births
                            .iter()
                            .filter(|b| b.node_id == birth.node_id)
                            .map(|b| b.path.clone()),
                    );
                }
            }
            if let Some(t) = &p.transaction
                && let Mutation::Relocate {
                    source,
                    target,
                    old_leaf,
                    new_leaf,
                    ..
                } = &t.mutation
            {
                let sequence = self.history_commit_time(p)?.1;
                relocations.push((
                    sequence,
                    source.clone(),
                    target.clone(),
                    old_leaf.clone(),
                    new_leaf.clone(),
                ));
            }
        }
        // Reusing an old pathname for a new note must not attach the former
        // occupant's earlier relocations to this note's fresh birth.
        relocations.retain(|r| match (birth_sequence, r.0) {
            (Some(born), Some(moved)) => moved > born,
            _ => true,
        });
        relocations.sort_by_key(|r| r.0);
        // Legacy edges are applied only when their actual source is reached.
        // Each edge is consumed once, so cycles never invent an ordering.
        let mut remaining = relocations;
        while let Some(index) = remaining.iter().position(|(_, source, _, _, _)| {
            places
                .iter()
                .any(|p| history_relative_path(Path::new(p), Path::new(source)).is_some())
        }) {
            let (_, source, target, old_leaf, new_leaf) = remaining.remove(index);
            let prior = places.iter().cloned().collect::<Vec<_>>();
            for p in prior {
                if let Some(relative) = history_relative_path(Path::new(&p), Path::new(&source)) {
                    let moved = if relative == Path::new(&old_leaf) {
                        Path::new(&target).join(&new_leaf)
                    } else {
                        Path::new(&target).join(&relative)
                    };
                    places.remove(&p);
                    places.insert(display(&moved));
                }
            }
        }
        if places.len() != 1
            || !places.iter().any(|p| {
                p.replace('/', "\\")
                    .eq_ignore_ascii_case(&display(current).replace('/', "\\"))
            })
        {
            return Err(CoreError::new(
                "identityPending",
                "No committed application move or rename proves the current placement; recover identity before browsing history",
            ));
        }
        Ok(())
    }

    fn history_commit_time(&self, plan: &journal::Plan) -> Result<(Option<u64>, Option<u64>)> {
        let v: serde_json::Value =
            serde_json::from_slice(&fs::read(self.journal_path(&plan.operation_id, "commit"))?)?;
        if v["operationId"] != plan.operation_id
            || v["fingerprint"] != plan.fingerprint
            || v["storageProfile"] != STORAGE_PROFILE
        {
            return Err(CoreError::new(
                "historyUnavailable",
                "The original commit witness is invalid",
            ));
        }
        Ok((v["savedAtUnixMs"].as_u64(), v["commitSequence"].as_u64()))
    }

    fn history_records(&self, context: &HistoryContext) -> Result<Vec<HistoryRecord>> {
        let mut records = Vec::new();
        for p in self
            .operations
            .values()
            .filter(|p| self.committed(&p.operation_id))
        {
            let candidate = if matches!(p.kind.as_str(), "save" | "createNote") {
                (p.identity.as_ref() == Some(&context.birth.identity)
                    || (p.node_id.as_ref() == Some(&context.birth.node_id)
                        && p.workspace_id.as_ref() == Some(&context.birth.workspace_id)))
                .then(|| (p.path.clone(), p.after.clone()))
            } else {
                p.transaction.as_ref().and_then(|t| {
                    if p.workspace_id.as_ref() != Some(&context.birth.workspace_id) {
                        return None;
                    }
                    let identity_path = t
                        .files
                        .iter()
                        .find(|f| f.stamp.identity == context.birth.identity)
                        .map(|f| f.path.as_str());
                    match &t.mutation {
                        Mutation::Update { files } => files
                            .iter()
                            .find(|f| identity_path == Some(f.path.as_str()))
                            .map(|f| (f.path.clone(), f.after.clone())),
                        Mutation::CreateTree {
                            files,
                            updates,
                            births,
                            ..
                        } => {
                            let born = births
                                .iter()
                                .find(|b| b.node_id == context.birth.node_id)
                                .map(|b| b.path.as_str());
                            files
                                .iter()
                                .chain(updates)
                                .find(|f| {
                                    identity_path == Some(f.path.as_str())
                                        || born == Some(f.path.as_str())
                                })
                                .map(|f| (f.path.clone(), f.after.clone()))
                        }
                        Mutation::Relocate { .. } => None,
                    }
                })
            };
            let Some((path, bytes)) = candidate else {
                continue;
            };
            let (saved_at, sequence) = self.history_commit_time(p)?;
            let mut version = HistoryVersion {
                version_id: p.operation_id.clone(), revision_id: None, operation_id: p.operation_id.clone(), kind: p.kind.clone(),
                saved_at_unix_ms: saved_at, time_basis: if saved_at.is_some() { "commitClock" } else { "unknown" }.into(), commit_sequence: sequence,
                status: "available".into(), byte_length: Some(bytes.len()), sha256: Some(hash(&bytes)), message: "Exact retained committed source; dependencies and semantic configuration are not reconstructed".into(),
            };
            let checked = (|| -> Result<RevisionFact> {
                // A cached plan is not a replacement for a removed/unreadable
                // retained record. Re-read and require its exact payload.
                let retained: journal::Plan =
                    serde_json::from_slice(&fs::read(self.journal_path(&p.operation_id, "plan"))?)?;
                if serde_json::to_vec(&retained)? != serde_json::to_vec(p)? {
                    return Err(CoreError::new("invalidState", "Retained plan changed"));
                }
                let facts = self.read_observation_facts(&p.operation_id)?;
                let fact = facts
                    .into_iter()
                    .find(|f| f.path == path && f.stamp.identity == context.birth.identity)
                    .ok_or_else(|| {
                        CoreError::new("missing", "The exact committed revision witness is missing")
                    })?;
                if fact.hash != hash(&bytes) || decode(&bytes).is_err() {
                    return Err(CoreError::new(
                        "invalidState",
                        "The exact retained source or its integrity witness is invalid",
                    ));
                }
                // A declared ID alone never established membership; the fact
                // above also binds this note's admitted physical identity.
                Ok(fact)
            })();
            match checked {
                Ok(fact) => {
                    version.revision_id = Some(fact.revision.clone());
                    records.push(HistoryRecord {
                        version,
                        bytes: Some(bytes),
                        fact: Some(fact),
                    });
                }
                Err(e) => {
                    version.status = match e.code.as_str() {
                        "missing" => "missing",
                        "permissionDenied" | "io" | "busy" => "unreadable",
                        _ => "invalid",
                    }
                    .into();
                    version.byte_length = None;
                    version.sha256 = None;
                    version.message = e.message;
                    records.push(HistoryRecord {
                        version,
                        bytes: None,
                        fact: None,
                    });
                }
            }
        }
        // Saved clock time is for browsing. The separate sequence preserves
        // known causality for tied times and clock rollback; UUIDs never order it.
        // A strong operation may explicitly admit its exact external preimage.
        // This is an existing reservation + committed frozen read, not a new
        // history record, and must not be omitted merely because it was not the
        // operation's final output.
        for p in self.operations.values().filter(|p| {
            self.committed(&p.operation_id)
                && p.workspace_id.as_ref() == Some(&context.birth.workspace_id)
        }) {
            let bytes = match fs::read(self.journal_path(&p.operation_id, "revision-reservation")) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            let r: operations::RevisionReservation = serde_json::from_slice(&bytes)?;
            if r.stamp.identity != context.birth.identity
                || records
                    .iter()
                    .any(|record| record.version.revision_id.as_ref() == Some(&r.revision))
                || !p.transaction.as_ref().is_some_and(|t| {
                    t.files
                        .iter()
                        .any(|f| f.stamp == r.stamp && f.bytes == r.bytes && f.path == r.path)
                })
            {
                continue;
            }
            let (time, sequence) = self.history_commit_time(p)?;
            let retained: journal::Plan =
                serde_json::from_slice(&fs::read(self.journal_path(&p.operation_id, "plan"))?)?;
            if serde_json::to_vec(&retained)? != serde_json::to_vec(p)? {
                return Err(CoreError::new(
                    "historyUnavailable",
                    "Committed admission plan changed",
                ));
            }
            let fact = RevisionFact {
                path: r.path,
                stamp: r.stamp,
                hash: hash(&r.bytes),
                revision: r.revision.clone(),
            };
            records.push(HistoryRecord { version: HistoryVersion {
                version_id: r.revision.clone(), revision_id: Some(r.revision), operation_id: p.operation_id.clone(), kind: "admittedSource".into(),
                saved_at_unix_ms: time, time_basis: if time.is_some() { "commitClock" } else { "unknown" }.into(), commit_sequence: sequence,
                status: "available".into(), byte_length: Some(r.bytes.len()), sha256: Some(hash(&r.bytes)),
                message: "Exact external preimage explicitly admitted by the retained committed operation".into(),
            }, bytes: Some(r.bytes), fact: Some(fact) });
        }
        records.sort_by(|a, b| {
            b.version
                .saved_at_unix_ms
                .cmp(&a.version.saved_at_unix_ms)
                .then(b.version.commit_sequence.cmp(&a.version.commit_sequence))
        });
        Ok(records)
    }

    fn history_saved_record<'a>(
        records: &'a [HistoryRecord],
        context: &HistoryContext,
    ) -> Option<&'a HistoryRecord> {
        records
            .iter()
            .filter(|r| {
                r.bytes.as_ref() == Some(&context.current.bytes)
                    && r.fact
                        .as_ref()
                        .is_some_and(|f| f.stamp == context.current.stamp)
            })
            .max_by_key(|r| r.version.commit_sequence)
    }
    pub fn list_history(&self, session_id: &str) -> Result<HistoryTimeline> {
        let context = self.history_context(session_id)?;
        let records = self.history_records(&context)?;
        let current_saved_version_id =
            Self::history_saved_record(&records, &context).map(|r| r.version.version_id.clone());
        let times = records
            .iter()
            .filter_map(|r| r.version.saved_at_unix_ms)
            .collect::<Vec<_>>();
        let available = records.iter().filter(|r| r.bytes.is_some()).count();
        let protected_drafts = self.history_protected_drafts(&context.birth.identity)?;
        let mut warnings = Vec::new();
        if current_saved_version_id.is_none() {
            warnings.push("Current bytes are an unadmitted external observation or their saved witness is unavailable; they are not listed as a saved version".into());
        }
        Ok(HistoryTimeline {
            node_id: context.birth.node_id, workspace_id: context.birth.workspace_id, path: context.current.path, current_base_id: context.session.base_id,
            currentness: if current_saved_version_id.is_some() { "saved" } else { "external" }.into(), current_saved_version_id,
            retention: HistoryRetention { policy: "retainedCoreJournal".into(), complete: false, known_versions: records.len(), available_versions: available,
                oldest_saved_at_unix_ms: times.iter().min().copied(), newest_saved_at_unix_ms: times.iter().max().copied(),
                message: "Only committed source records still retained in this local Core journal are shown. Earlier/pruned records, keystrokes and other-device history are not guaranteed; an empty list does not prove the note was never saved".into() },
            versions: records.into_iter().map(|r| r.version).collect(), protected_drafts, warnings,
        })
    }
    fn selected_history_record(
        &self,
        context: &HistoryContext,
        version_id: &str,
    ) -> Result<HistoryRead> {
        valid_operation_id(version_id)?;
        let record = self.history_records(context)?.into_iter().find(|r| r.version.version_id == version_id)
            .ok_or_else(|| CoreError::new("historyUnavailable", "The selected retained version does not belong to this note; latest was not substituted"))?;
        let source = decode(&record.bytes.ok_or_else(|| {
            CoreError::new("historyUnavailable", record.version.message.clone())
        })?)?;
        Ok(HistoryRead { version: record.version, source, read_only: true, preview_status: "sourceOnly".into(),
            preview_message: "Exact source viewing remains available without its original dependencies or configuration. Historical semantic preview is unavailable; current settings are not substituted".into() })
    }
    pub fn read_history(&self, session_id: &str, version_id: &str) -> Result<HistoryRead> {
        let context = self.history_context(session_id)?;
        self.selected_history_record(&context, version_id)
    }
    pub fn compare_history(&self, session_id: &str, version_id: &str) -> Result<HistoryComparison> {
        let context = self.history_context(session_id)?;
        let selected = self.selected_history_record(&context, version_id)?;
        let records = self.history_records(&context)?;
        let saved = Self::history_saved_record(&records, &context)
            .ok_or_else(|| CoreError::new("currentSavedUnavailable", "The current file is an external observation or its exact saved witness is missing; reopen and explicitly accept it before comparing to current saved content"))?;
        let source = decode(&context.current.bytes)?;
        Ok(HistoryComparison {
            equal: selected.source == source,
            selected,
            current: HistoryCurrentSource {
                version_id: saved.version.version_id.clone(),
                revision_id: saved.version.revision_id.clone(),
                source,
                base_id: context.session.base_id,
                path: context.current.path,
            },
            comparison: "currentSaved".into(),
        })
    }

    fn history_protected_drafts(&self, identity: &FileIdentity) -> Result<Vec<DraftAlternative>> {
        let mut drafts = Vec::new();
        let mut total = 0usize;
        for entry in fs::read_dir(self.draft_directory("draft-history"))? {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let bytes = fs::read(path)?;
            let draft: Draft = serde_json::from_slice(&bytes)?;
            if draft.identity.as_ref() != Some(identity) {
                continue;
            }
            total += bytes.len();
            if drafts.len() >= 128 || total > 16 * 1024 * 1024 {
                return Err(CoreError::new(
                    "budget",
                    "Protected draft browsing exceeds 128 records or 16 MiB",
                ));
            }
            append_draft_alternatives(&mut drafts, &draft);
        }
        Ok(drafts)
    }
    fn restore_result(&self, plan: &journal::Plan) -> Result<HistoryRestoreResult> {
        let binding = plan.history_restore.as_ref().ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                "This operation is not a history restore",
            )
        })?;
        self.authorize_restore_result(binding)?;
        let retained: journal::Plan =
            serde_json::from_slice(&fs::read(self.journal_path(&plan.operation_id, "plan"))?)?;
        if serde_json::to_vec(&retained)? != serde_json::to_vec(plan)? {
            return Err(CoreError::new(
                "historyUnavailable",
                "The original committed restore plan is unavailable or changed",
            ));
        }
        let mut protected_drafts = Vec::new();
        for name in &binding.draft_records {
            valid_operation_id(name)?;
            let draft: Draft = serde_json::from_slice(&fs::read(
                self.draft_directory("draft-history")
                    .join(format!("{name}.json")),
            )?)?;
            append_draft_alternatives(&mut protected_drafts, &draft);
        }
        Ok(HistoryRestoreResult {
            receipt: plan.receipt.clone().ok_or_else(|| {
                CoreError::new("invalidState", "The original save receipt is missing")
            })?,
            source: decode(&plan.after)?,
            version_id: binding.version_id.clone(),
            protected_drafts,
        })
    }
    fn authorize_restore_result(&self, binding: &RestoreBinding) -> Result<()> {
        let root = self
            .operations
            .values()
            .find(|p| {
                matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                    && p.workspace_id.as_ref() == Some(&binding.workspace_id)
                    && self.committed(&p.operation_id)
            })
            .map(|p| PathBuf::from(&p.path))
            .ok_or_else(|| {
                CoreError::new(
                    "historyUnavailable",
                    "The original library admission is unavailable",
                )
            })?;
        let inventory = self.inventory(&root)?;
        let owners = inventory
            .entries
            .iter()
            .filter(|e| {
                e.node_id.as_ref() == Some(&binding.node_id) && e.identity_status == "admitted"
            })
            .collect::<Vec<_>>();
        if owners.len() != 1 {
            return Err(CoreError::new(
                "identityPending",
                "The current restore owner is unavailable or no longer qualified",
            ));
        }
        let path = Path::new(&owners[0].path);
        let guard = ParentGuard::acquire(path)?;
        let mut file = guard.open_file(false, false)?;
        let current = stamp(&file)?;
        if current.identity != binding.identity {
            return Err(CoreError::new(
                "identityPending",
                "The restored note object was replaced",
            ));
        }
        read_source(&mut file)?;
        self.check_pending(path)?;
        self.check_pending_identity(&current.identity)?;
        self.check_discovery_boundary(&root, path.parent().unwrap())?;
        self.check_history_placement(
            &Birth {
                node_id: binding.node_id.clone(),
                workspace_id: binding.workspace_id.clone(),
                identity: binding.identity.clone(),
            },
            path,
        )
    }
    pub fn history_restore_status(&self, operation_id: &str) -> Result<HistoryRestoreStatus> {
        valid_operation_id(operation_id)?;
        let Some(plan) = self.operations.get(operation_id) else {
            return Ok(HistoryRestoreStatus {
                operation_id: operation_id.into(),
                status: "missing".into(),
                result: None,
            });
        };
        if plan.history_restore.is_none() {
            return Err(CoreError::new(
                "operationConflict",
                "This operation is not a history restore",
            ));
        }
        let committed = self.committed(operation_id);
        Ok(HistoryRestoreStatus {
            operation_id: operation_id.into(),
            status: if committed {
                "committed"
            } else if self.resolved(operation_id) {
                "resolved"
            } else {
                "recoveryPending"
            }
            .into(),
            result: if committed {
                Some(self.restore_result(plan)?)
            } else {
                None
            },
        })
    }
    pub fn restore_history(
        &mut self,
        request: HistoryRestoreRequest,
    ) -> Result<HistoryRestoreResult> {
        valid_operation_id(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        // Reconcile the original exact operation before current session/Base
        // checks. A lost response may have already advanced that Base.
        if let Some(plan) = self.operations.get(&request.operation_id) {
            if plan
                .history_restore
                .as_ref()
                .is_none_or(|b| b.fingerprint != fingerprint)
            {
                return Err(CoreError::new(
                    "operationConflict",
                    "The original restore input is immutable; this operationId belongs to a different request",
                ));
            }
            if self.committed(&request.operation_id) {
                return self.restore_result(plan);
            }
            return Err(CoreError::new(
                "recoveryPending",
                "The original restore may have effects; inspect its operation and do not create another save",
            ));
        }
        if !matches!(request.draft_action.as_str(), "retain" | "none") {
            return Err(CoreError::new(
                "invalidDraftAction",
                "Choose retain or none explicitly",
            ));
        }
        self.check_session_base(&request.session_id, &request.base_id)?;
        self.restore_history_after_preflight(request, fingerprint)
    }
    pub(super) fn restore_history_after_preflight(
        &mut self,
        request: HistoryRestoreRequest,
        fingerprint: String,
    ) -> Result<HistoryRestoreResult> {
        let context = self.history_context(&request.session_id)?;
        let selected = self.selected_history_record(&context, &request.version_id)?;
        // The editor buffer belongs to the session's original Base. A raced
        // external observation must never be advertised as that draft's Base.
        let current_source = decode(&context.session.bytes)?;
        let persisted = self.read_draft(&context.session.path)?;
        let dirty_persisted = persisted.as_ref().is_some_and(|d| {
            d.source != current_source || !d.alternatives.is_empty() || !d.base_known
        });
        let dirty_buffer = request
            .draft_source
            .as_ref()
            .is_some_and(|s| s != &current_source);
        if request.draft_action == "none" && (dirty_persisted || dirty_buffer) {
            return Err(CoreError::new(
                "draftProtectionRequired",
                "An unsaved draft exists; explicitly retain it before restoring",
            ));
        }
        let mut draft_records = Vec::new();
        if request.draft_action == "retain" {
            let mut to_protect = persisted
                .filter(|_| dirty_persisted)
                .into_iter()
                .collect::<Vec<_>>();
            if let Some(source) = &request.draft_source {
                check_size(source.as_bytes())?;
                if dirty_buffer && !to_protect.iter().any(|d| d.source == *source) {
                    to_protect.push(Draft {
                        path: context.current.path.clone(),
                        base_source: current_source.clone(),
                        source: source.clone(),
                        base_known: true,
                        identity: Some(context.birth.identity.clone()),
                        alternatives: Vec::new(),
                    });
                }
            }
            for draft in to_protect {
                let name = fresh();
                journal::private_create(
                    &self
                        .draft_directory("draft-history")
                        .join(format!("{name}.json")),
                    &serde_json::to_vec(&draft)?,
                )?;
                draft_records.push(name);
            }
        }
        let operation_id = request.operation_id.clone();
        let binding = RestoreBinding {
            fingerprint,
            version_id: request.version_id,
            draft_records,
            node_id: context.birth.node_id,
            workspace_id: context.birth.workspace_id,
            identity: context.birth.identity,
        };
        self.save_with_restore(
            SaveRequest {
                session_id: request.session_id,
                base_id: request.base_id,
                source: selected.source,
                operation_id: operation_id.clone(),
            },
            Some(binding),
        )?;
        self.restore_result(&self.operations[&operation_id])
    }
}
fn append_draft_alternatives(out: &mut Vec<DraftAlternative>, draft: &Draft) {
    for proposal in std::iter::once(DraftAlternative {
        path: draft.path.clone(),
        base_source: draft.base_source.clone(),
        source: draft.source.clone(),
        base_known: draft.base_known,
    })
    .chain(draft.alternatives.iter().cloned())
    {
        if !out.iter().any(|d| {
            d.source == proposal.source
                && d.base_source == proposal.base_source
                && d.base_known == proposal.base_known
        }) {
            out.push(proposal);
        }
    }
}
fn history_relative_path(path: &Path, prefix: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    for prior in prefix.components() {
        let actual = components.next()?;
        if !actual
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&prior.as_os_str().to_string_lossy())
        {
            return None;
        }
    }
    Some(components.collect())
}
