//! A read-only projection of the existing commit, admission and exact-byte
//! journal. Restoring is an ordinary guarded save, never a chain rewind.
use super::*;
use operations::{CapturedFile, Mutation, RevisionFact};
use std::collections::BTreeSet;
use weftext_messages::UiMessage;

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
    pub description: CoreDescription,
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
    pub description: CoreDescription,
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
    pub warnings: Vec<CoreDescription>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRead {
    #[serde(flatten)]
    pub version: HistoryVersion,
    pub source: String,
    pub read_only: bool,
    pub preview_status: String,
    pub preview_description: CoreDescription,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_issue: Option<RecoveryIssue>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRestoreStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<Receipt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_error: Option<RecoveryIssue>,
    pub operation_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<HistoryRestoreResult>,
}
struct HistoryContext {
    session: Session,
    birth: Birth,
    current: CapturedFile,
    in_trash: bool,
}
struct HistoryRecord {
    version: HistoryVersion,
    bytes: Option<Vec<u8>>,
    fact: Option<RevisionFact>,
}

impl Core {
    fn history_context(&self, session_id: &str) -> Result<HistoryContext> {
        self.check_actor_session(session_id)?;
        let session = self.sessions.get(session_id).cloned().ok_or_else(|| {
            CoreError::new(
                "sessionMissing",
                UiMessage::error_core_history_open_note_again(),
            )
        })?;
        self.check_pending(&session.path)?;
        let guard = ParentGuard::acquire(&self.backend, &session.path)?;
        let mut handle = guard.open_file(false, false)?;
        let observed = stamp(&handle)?;
        self.check_pending_identity(&observed.identity)?;
        let bytes = read_source(&mut handle)?;
        let birth = self.admitted_binding(&observed.identity)?.ok_or_else(|| {
            CoreError::new(
                "historyUnavailable",
                UiMessage::error_core_history_source_has_no_admitted_stable_note_identity(),
            )
        })?;
        if session.stamp.identity != observed.identity
            || session.node_id.as_ref() != Some(&birth.node_id)
            || weftext_engine::inspect_metadata(&decode(&bytes)?)
                .id
                .as_ref()
                != Some(&birth.node_id)
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_current_note_object_or_declaration_needs_identity_recovery_history_was_not_joined_by_name(),
            ));
        }
        let root = self
            .operations
            .values()
            .try_fold(None, |found, p| -> Result<Option<_>> {
                if found.is_some() {
                    Ok(found)
                } else {
                    Ok(
                        if {
                            matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                                && p.workspace_id.as_ref() == Some(&birth.workspace_id)
                                && self.committed(&p.operation_id)?
                        } {
                            Some(p)
                        } else {
                            None
                        },
                    )
                }
            })?
            .map(|p| PathBuf::from(&p.path))
            .ok_or_else(|| {
                CoreError::new(
                    "historyUnavailable",
                    UiMessage::error_core_history_original_library_admission_unavailable(),
                )
            })?;
        if !session.path.starts_with(&root) {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_note_outside_admitted_library(),
            ));
        }
        let in_trash = session.path.strip_prefix(&root).is_ok_and(|relative| {
            relative.components().next().is_some_and(|component| {
                component
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(".weftext-trash")
            })
        });
        if !is_canonical(&session.path) && !in_trash {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_source_not_at_canonical_live_placement(),
            ));
        }
        // Missing semantic configuration does not prevent exact source viewing.
        // Current discovery exclusions and current OS file access still apply.
        self.check_discovery_boundary(&root, session.path.parent().unwrap())?;
        self.check_history_placement(&birth, &session.path)?;
        Ok(HistoryContext {
            in_trash,
            current: CapturedFile {
                path: display(&session.path),
                bytes,
                stamp: observed,
            },
            session,
            birth,
        })
    }

    pub(crate) fn check_history_placement(&self, birth: &Birth, current: &Path) -> Result<()> {
        let places = self.node_placement_paths(birth)?;
        if places.len() != 1
            || !places
                .iter()
                .any(|p| same_placement_path(&self.backend, Path::new(p), current).unwrap_or(false))
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_no_committed_application_placement_proves_exact_object(),
            ));
        }
        Ok(())
    }
    pub(crate) fn node_placement_paths(&self, birth: &Birth) -> Result<BTreeSet<String>> {
        self.node_placement_paths_at(birth, None)
    }
    /// Project the same identity-proved placement history at an original
    /// committed decision. Missing legacy ordering cannot prove nonexistence.
    pub(crate) fn node_placement_paths_at(
        &self,
        birth: &Birth,
        through: Option<u64>,
    ) -> Result<BTreeSet<String>> {
        let mut places = BTreeSet::<String>::new();
        let mut birth_sequence = None;
        for p in self.committed_plans()?.into_iter() {
            if self.read_births(&p.operation_id)?.iter().any(|b| {
                b.node_id == birth.node_id
                    && b.workspace_id == birth.workspace_id
                    && b.identity == birth.identity
            }) {
                let sequence = self.history_commit_time(p)?.1;
                if let Some(limit) = through {
                    let sequence = sequence.ok_or_else(|| {
                        CoreError::new(
                            "historyUnavailable",
                            UiMessage::error_core_history_original_placement_ordering_unavailable(),
                        )
                    })?;
                    if sequence > limit {
                        continue;
                    }
                }
                birth_sequence = sequence;
                if p.kind == "createNote" {
                    places.insert(p.path.clone());
                }
                if let Some(t) = &p.transaction {
                    places.extend(
                        t.births()
                            .iter()
                            .filter(|b| b.node_id == birth.node_id)
                            .map(|b| b.path.clone()),
                    );
                }
            }
        }
        self.replay_placements_at(
            places,
            birth_sequence,
            &birth.identity,
            Some(&birth.node_id),
            None,
            through,
        )
    }
    pub(crate) fn replay_placements(
        &self,
        places: BTreeSet<String>,
        born: Option<u64>,
        identity: &FileIdentity,
        node: Option<&str>,
        directory: Option<&str>,
    ) -> Result<BTreeSet<String>> {
        self.replay_placements_at(places, born, identity, node, directory, None)
    }
    fn replay_placements_at(
        &self,
        mut places: BTreeSet<String>,
        born: Option<u64>,
        identity: &FileIdentity,
        node: Option<&str>,
        directory: Option<&str>,
        through: Option<u64>,
    ) -> Result<BTreeSet<String>> {
        let mut edges = Vec::new();
        for p in self.committed_plans()?.into_iter() {
            if let Some(t) = &p.transaction
                && matches!(
                    t.mutation,
                    Mutation::Relocate { .. } | Mutation::Compound { .. }
                )
            {
                let sequence = self.history_commit_time(p)?.1;
                if let Some(limit) = through {
                    let sequence = sequence.ok_or_else(|| {
                        CoreError::new(
                            "historyUnavailable",
                            UiMessage::error_core_history_original_placement_ordering_unavailable(),
                        )
                    })?;
                    if sequence > limit {
                        continue;
                    }
                }
                if !matches!((born,sequence),(Some(a),Some(b)) if b<=a) {
                    edges.push((sequence, t));
                }
            }
        }
        edges.sort_by_key(|e| e.0);
        for (_, t) in edges {
            // Each original step maps its own before coordinates once. A
            // final location is never fed through another step in the same Plan.
            let original_places = places.clone();
            let relocations = t.relocations();
            for prior in original_places {
                // A child explicitly extracted from an original moving tree
                // has one placement owner. Its ancestor must not add a second
                // candidate for the same original physical object.
                if let Some(relocation) = relocations
                    .iter()
                    .filter(|step| {
                        history_relative_path(Path::new(&prior), Path::new(step.source)).is_some()
                    })
                    .max_by_key(|step| Path::new(step.source).components().count())
                {
                    let source = relocation.source;
                    let target = relocation.target;
                    let old_leaf = relocation.old_leaf;
                    let new_leaf = relocation.new_leaf;
                    let leaf_identity = relocation.leaf_identity;
                    let source_identity = relocation.source_identity;
                    let members = relocation.members;
                    let Some(relative) =
                        history_relative_path(Path::new(&prior), Path::new(source))
                    else {
                        continue;
                    };
                    let exact = if node.is_some() && relative == Path::new(old_leaf) {
                        leaf_identity == identity
                    } else if directory.is_some() && relative.as_os_str().is_empty() {
                        source_identity == identity
                    } else {
                        members.iter().any(|m| {
                            if let Some(id) = node {
                                m.node_id.as_deref() == Some(id)
                                    && &m.identity == identity
                                    && Path::new(&m.relative_path) == relative
                            } else {
                                m.directory_id.as_deref() == directory
                                    && &m.parent == identity
                                    && Path::new(&m.relative_path).parent()
                                        == Some(relative.as_path())
                            }
                        }) || (node.is_some()
                            && t.files.iter().any(|f| {
                                &f.stamp.identity == identity
                                    && history_relative_path(Path::new(&f.path), Path::new(source))
                                        .as_ref()
                                        == Some(&relative)
                            }))
                            || (directory.is_some()
                                && t.directories.iter().any(|d| {
                                    &d.identity == identity
                                        && history_relative_path(
                                            Path::new(&d.path),
                                            Path::new(source),
                                        )
                                        .as_ref()
                                            == Some(&relative)
                                }))
                    };
                    if !exact {
                        continue;
                    }
                    let moved = if node.is_some() && relative == Path::new(old_leaf) {
                        Path::new(target).join(new_leaf)
                    } else {
                        Path::new(target).join(relative)
                    };
                    places.remove(&prior);
                    places.insert(display(&moved));
                }
            }
        }
        Ok(places)
    }
    pub(crate) fn history_commit_time(
        &self,
        plan: &journal::Plan,
    ) -> Result<(Option<u64>, Option<u64>)> {
        let v: serde_json::Value = serde_json::from_slice(
            &self
                .backend
                .read_bytes(self.journal_path(&plan.operation_id, "commit"))?,
        )?;
        if v["operationId"] != plan.operation_id
            || v["fingerprint"] != plan.fingerprint
            || v["storageProfile"] != STORAGE_PROFILE
        {
            return Err(CoreError::new(
                "historyUnavailable",
                UiMessage::error_core_history_original_commit_witness_invalid(),
            ));
        }
        Ok((v["savedAtUnixMs"].as_u64(), v["commitSequence"].as_u64()))
    }

    fn history_records(&self, context: &HistoryContext) -> Result<Vec<HistoryRecord>> {
        self.history_records_for(&context.birth)
    }
    fn history_records_for(&self, birth: &Birth) -> Result<Vec<HistoryRecord>> {
        let mut records = Vec::new();
        for p in self.committed_plans()?.into_iter() {
            let candidate = if matches!(p.kind.as_str(), "save" | "createNote") {
                (p.identity.as_ref() == Some(&birth.identity)
                    || (p.node_id.as_ref() == Some(&birth.node_id)
                        && p.workspace_id.as_ref() == Some(&birth.workspace_id)))
                .then(|| (p.path.clone(), p.after.clone()))
            } else {
                p.transaction.as_ref().and_then(|t| {
                    if p.workspace_id.as_ref() != Some(&birth.workspace_id) {
                        return None;
                    }
                    let identity_path = t
                        .files
                        .iter()
                        .find(|f| f.stamp.identity == birth.identity)
                        .map(|f| f.path.as_str());
                    if let Some(born) = t.births().iter().find(|b| b.node_id == birth.node_id)
                        && let Some(file) = t.created_files().iter().find(|file| file.path == born.path) {
                        return Some((file.path.clone(), file.after.clone()));
                    }
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
                                .find(|b| b.node_id == birth.node_id)
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
                        Mutation::Relocate { updates, .. } | Mutation::Compound { updates, .. } => identity_path.and_then(|original| {
                            let path = t.relocated_path(original);
                            let changed_revision = p.sync_application.as_ref().is_some_and(|application| {
                                application.before.iter().any(|object| {
                                    matches!(&object.object, sync::ObjectKey::Node(id) if id == &birth.node_id)
                                        && object.state.file_revisions.get(&format!("{}.typ", object.state.name)) != t.preview.revisions.get(&path)
                                })
                            });
                            updates.iter().find(|file| file.path == path && (file.before.as_ref() != Some(&file.after) || changed_revision)).map(|file| (file.path.clone(), file.after.clone()))
                        }),
                    }
                })
            };
            let Some((path, bytes)) = candidate else {
                continue;
            };
            let (saved_at, sequence) = self.history_commit_time(p)?;
            let mut version = HistoryVersion {
                version_id: p.operation_id.clone(),
                revision_id: None,
                operation_id: p.operation_id.clone(),
                kind: p.kind.clone(),
                saved_at_unix_ms: saved_at,
                time_basis: if saved_at.is_some() {
                    "commitClock"
                } else {
                    "unknown"
                }
                .into(),
                commit_sequence: sequence,
                status: "available".into(),
                byte_length: Some(bytes.len()),
                sha256: Some(hash(&bytes)),
                description: UiMessage::history_retained_committed_source().into(),
            };
            let checked = (|| -> Result<RevisionFact> {
                if matches!(p.kind.as_str(), "save" | "saveAs") {
                    self.validate_file_save_plan(p)?;
                    if !matches!(&p.file_save.as_ref().unwrap().scope, QualifiedInputScope::Library { binding } if binding.workspace_id == birth.workspace_id)
                    {
                        return Err(CoreError::new(
                            "historyUnavailable",
                            UiMessage::error_core_history_retained_save_different_scope(),
                        ));
                    }
                }
                // A cached plan is not a replacement for a removed/unreadable
                // retained record. Re-read and require its exact payload.
                let retained: journal::Plan = serde_json::from_slice(
                    &self
                        .backend
                        .read_bytes(self.journal_path(&p.operation_id, "plan"))?,
                )?;
                if serde_json::to_vec(&retained)? != serde_json::to_vec(p)? {
                    return Err(CoreError::new(
                        "invalidState",
                        UiMessage::error_core_history_retained_plan_changed(),
                    ));
                }
                let facts = self.read_observation_facts(&p.operation_id)?;
                let fact = facts
                    .into_iter()
                    .find(|f| f.path == path && f.stamp.identity == birth.identity)
                    .ok_or_else(|| {
                        CoreError::new(
                            "missing",
                            UiMessage::error_core_history_exact_committed_revision_witness_missing(
                            ),
                        )
                    })?;
                if fact.hash != hash(&bytes) || decode(&bytes).is_err() {
                    return Err(CoreError::new(
                        "invalidState",
                        UiMessage::error_core_history_exact_retained_source_or_integrity_witness_invalid(),
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
                        // Missing committed observations stay unavailable. This
                        // presentation state never admits a new ordinary source.
                        "missing" | "identityPending" => "missing",
                        "unsupportedFileSaveEvidence" => "unsupported",
                        "permissionDenied" | "io" | "busy" => "unreadable",
                        _ => "invalid",
                    }
                    .into();
                    version.byte_length = None;
                    version.sha256 = None;
                    version.description = e.into();
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
        for p in self
            .operations
            .values()
            .map(|p| -> Result<_> {
                Ok(
                    if {
                        self.committed(&p.operation_id)?
                            && p.workspace_id.as_ref() == Some(&birth.workspace_id)
                    } {
                        Some(p)
                    } else {
                        None
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
        {
            let bytes = match self
                .backend
                .read_bytes(self.journal_path(&p.operation_id, "revision-reservation"))
            {
                Ok(bytes) => bytes,
                Err(e) if e.code == "missing" => continue,
                Err(e) => return Err(e),
            };
            let r: operations::RevisionReservation = serde_json::from_slice(&bytes)?;
            if r.stamp.identity != birth.identity
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
            let retained: journal::Plan = serde_json::from_slice(
                &self
                    .backend
                    .read_bytes(self.journal_path(&p.operation_id, "plan"))?,
            )?;
            if serde_json::to_vec(&retained)? != serde_json::to_vec(p)? {
                return Err(CoreError::new(
                    "historyUnavailable",
                    UiMessage::error_core_history_committed_admission_plan_changed(),
                ));
            }
            let fact = RevisionFact {
                path: r.path,
                stamp: r.stamp,
                hash: hash(&r.bytes),
                revision: r.revision.clone(),
            };
            records.push(HistoryRecord {
                version: HistoryVersion {
                    version_id: r.revision.clone(),
                    revision_id: Some(r.revision),
                    operation_id: p.operation_id.clone(),
                    kind: "admittedSource".into(),
                    saved_at_unix_ms: time,
                    time_basis: if time.is_some() {
                        "commitClock"
                    } else {
                        "unknown"
                    }
                    .into(),
                    commit_sequence: sequence,
                    status: "available".into(),
                    byte_length: Some(r.bytes.len()),
                    sha256: Some(hash(&r.bytes)),
                    description: UiMessage::history_admitted_external_preimage().into(),
                },
                bytes: Some(r.bytes),
                fact: Some(fact),
            });
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
    pub(crate) fn current_saved_history_operation(
        &self,
        current: &QualifiedNode,
    ) -> Result<Option<String>> {
        if current.revision.is_none() {
            return Ok(None);
        }
        let birth = Birth {
            node_id: current.owner.node_id.clone(),
            workspace_id: current.owner.workspace_id.clone(),
            identity: current.identity.clone(),
        };
        let captured = current.captured_file();
        let records = self.history_records_for(&birth)?;
        let saved = records
            .iter()
            .filter(|record| {
                record.bytes.as_ref() == Some(&captured.bytes)
                    && record
                        .fact
                        .as_ref()
                        .is_some_and(|fact| fact.stamp == captured.stamp)
            })
            .max_by_key(|record| record.version.commit_sequence)
            .ok_or_else(|| {
                CoreError::new(
                    "currentSavedUnavailable",
                    UiMessage::error_core_history_exact_current_saved_operation_evidence_unavailable(),
                )
            })?;
        Ok(Some(saved.version.operation_id.clone()))
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
        let protected_drafts = self.history_protected_drafts(
            &context.birth.identity,
            &context.session.file_scope.clone()?,
        )?;
        let mut warnings = Vec::new();
        if current_saved_version_id.is_none() {
            warnings.push(UiMessage::history_current_source_not_saved().into());
        }
        Ok(HistoryTimeline {
            node_id: context.birth.node_id,
            workspace_id: context.birth.workspace_id,
            path: context.current.path,
            current_base_id: context.session.base_id,
            currentness: if current_saved_version_id.is_some() {
                "saved"
            } else {
                "external"
            }
            .into(),
            current_saved_version_id,
            retention: HistoryRetention {
                policy: "retainedCoreJournal".into(),
                complete: false,
                known_versions: records.len(),
                available_versions: available,
                oldest_saved_at_unix_ms: times.iter().min().copied(),
                newest_saved_at_unix_ms: times.iter().max().copied(),
                description: UiMessage::history_retention_limited().into(),
            },
            versions: records.into_iter().map(|r| r.version).collect(),
            protected_drafts,
            warnings,
        })
    }
    fn selected_history_record(
        &self,
        context: &HistoryContext,
        version_id: &str,
    ) -> Result<HistoryRead> {
        valid_operation_id(version_id)?;
        let record = self.history_records(context)?.into_iter().find(|r| r.version.version_id == version_id)
            .ok_or_else(|| CoreError::new("historyUnavailable", UiMessage::error_core_history_selected_retained_version_does_not_belong_to_note_latest_was_not_substituted()))?;
        let source = decode(&record.bytes.ok_or_else(|| {
            record
                .version
                .description
                .clone()
                .into_error("historyUnavailable")
        })?)?;
        Ok(HistoryRead {
            version: record.version,
            source,
            read_only: true,
            preview_status: "sourceOnly".into(),
            preview_description: UiMessage::history_source_only_preview().into(),
        })
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
            .ok_or_else(|| CoreError::new("currentSavedUnavailable", UiMessage::error_core_history_current_file_external_observation_or_exact_saved_witness_missing_reopen_and_explicitly_accept_it_before_comparing_to_current_saved_content()))?;
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

    fn history_protected_drafts(
        &self,
        identity: &FileIdentity,
        scope: &QualifiedInputScope,
    ) -> Result<Vec<DraftAlternative>> {
        let mut drafts = Vec::new();
        let mut total = 0usize;
        let mut records = 0usize;
        for entry in self
            .backend
            .read_dir(self.draft_directory("draft-history"))?
        {
            let path = entry?.path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let guard = ParentGuard::acquire(&self.backend, &path)?;
            let mut file = guard.open_read_guard_file()?;
            let Some((metadata, bytes)) = hosted::read_qualified_private_record(
                &mut file,
                |file| {
                    let metadata = draft_recovery::read_draft_metadata(file)?;
                    self.validate_original_input_scope(&metadata.scope)?;
                    if metadata.scope != *scope {
                        return Ok(None);
                    }
                    if metadata.identity != *identity {
                        return Ok(None);
                    }
                    let remaining = 16 * 1024 * 1024usize - total;
                    if records >= 128 {
                        return Err(CoreError::new(
                            "budget",
                            UiMessage::error_core_history_protected_draft_browsing_exceeds128_records_or16_mi_b(),
                        ));
                    }
                    records += 1;
                    Ok(Some((metadata, remaining)))
                },
            )?
            else {
                continue;
            };
            total += bytes.len();
            let draft: Draft = serde_json::from_slice(&bytes)?;
            if draft.identity != metadata.identity || draft.scope != metadata.scope {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Protected draft metadata changed during its qualified read",
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
                UiMessage::error_core_history_operation_not_history_restore(),
            )
        })?;
        self.authorize_restore_result(binding)?;
        self.retained_restore_result(plan)
    }
    pub(crate) fn restore_result_at(
        &self,
        plan: &journal::Plan,
        root: &Path,
        path: &Path,
    ) -> Result<HistoryRestoreResult> {
        let binding = plan.history_restore.as_ref().ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_core_history_operation_not_history_restore(),
            )
        })?;
        let current = self.qualified_node_at(root, path)?;
        if current.owner.node_id != binding.node_id
            || current.owner.workspace_id != binding.workspace_id
            || current.identity != binding.identity
        {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_explicitly_selected_restore_owner_differs(),
            ));
        }
        self.retained_restore_result(plan)
    }
    fn retained_restore_result(&self, plan: &journal::Plan) -> Result<HistoryRestoreResult> {
        self.validate_file_save_plan(plan)?;
        let binding = plan.history_restore.as_ref().ok_or_else(|| {
            CoreError::new(
                "operationConflict",
                UiMessage::error_core_history_operation_not_history_restore(),
            )
        })?;
        if !matches!(&plan.file_save.as_ref().unwrap().scope, QualifiedInputScope::Library { binding: scope } if scope.workspace_id == binding.workspace_id)
        {
            return Err(CoreError::new(
                "historyUnavailable",
                UiMessage::error_core_history_restore_different_owner(),
            ));
        }
        let retained: journal::Plan = serde_json::from_slice(
            &self
                .backend
                .read_bytes(self.journal_path(&plan.operation_id, "plan"))?,
        )?;
        if serde_json::to_vec(&retained)? != serde_json::to_vec(plan)? {
            return Err(CoreError::new(
                "historyUnavailable",
                UiMessage::error_core_history_original_committed_restore_plan_unavailable_or_changed(),
            ));
        }
        let mut protected_drafts = Vec::new();
        if binding.draft_records.len() > 128 {
            return Err(CoreError::new(
                "budget",
                "Original restore exceeds 128 protected Draft records",
            ));
        }
        let mut protected_bytes = 0usize;
        for name in &binding.draft_records {
            valid_operation_id(name)?;
            let path = self
                .draft_directory("draft-history")
                .join(format!("{name}.json"));
            let (draft, bytes) = self
                .read_draft_record(
                    &path,
                    &plan.file_save.as_ref().unwrap().scope,
                    plan.identity.as_ref().ok_or_else(|| {
                        CoreError::new(
                            "historyUnavailable",
                            "Original restore physical owner is missing",
                        )
                    })?,
                )?
                .ok_or_else(|| {
                    CoreError::new("historyUnavailable", "Original protected Draft is missing")
                })?;
            protected_bytes = protected_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| CoreError::new("budget", "Protected Draft byte count overflow"))?;
            if protected_bytes > 16 * 1024 * 1024 {
                return Err(CoreError::new(
                    "budget",
                    "Original protected Drafts exceed 16 MiB",
                ));
            }
            append_draft_alternatives(&mut protected_drafts, &draft);
        }
        Ok(HistoryRestoreResult {
            receipt: plan.receipt.clone().ok_or_else(|| {
                CoreError::new(
                    "invalidState",
                    UiMessage::error_core_history_original_save_receipt_missing(),
                )
            })?,
            source: decode(&plan.after)?,
            version_id: binding.version_id.clone(),
            protected_drafts,
            recovery_issue: None,
        })
    }
    fn authorize_restore_result(&self, binding: &RestoreBinding) -> Result<()> {
        let root = self
            .operations
            .values()
            .try_fold(None, |found, p| -> Result<Option<_>> {
                if found.is_some() {
                    Ok(found)
                } else {
                    Ok(
                        if {
                            matches!(p.kind.as_str(), "createLibrary" | "cloneSync")
                                && p.workspace_id.as_ref() == Some(&binding.workspace_id)
                                && self.committed(&p.operation_id)?
                        } {
                            Some(p)
                        } else {
                            None
                        },
                    )
                }
            })?
            .map(|p| PathBuf::from(&p.path))
            .ok_or_else(|| {
                CoreError::new(
                    "historyUnavailable",
                    UiMessage::error_core_history_original_library_admission_unavailable(),
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
                UiMessage::error_core_history_current_restore_owner_unavailable_or_no_longer_qualified(),
            ));
        }
        let path = Path::new(&owners[0].path);
        let guard = ParentGuard::acquire(&self.backend, path)?;
        let mut file = guard.open_file(false, false)?;
        let current = stamp(&file)?;
        if current.identity != binding.identity {
            return Err(CoreError::new(
                "identityPending",
                UiMessage::error_core_history_restored_note_object_was_replaced(),
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
        self.backend.check_path(&self.data_dir)?;
        valid_operation_id(operation_id)?;
        let Some(plan) = self.operations.get(operation_id) else {
            return Ok(HistoryRestoreStatus {
                operation_id: operation_id.into(),
                status: if self.resolved(operation_id)? {
                    "resolved"
                } else {
                    "missing"
                }
                .into(),
                result: None,
                receipt: None,
                result_error: None,
            });
        };
        if plan.history_restore.is_none() {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_history_operation_not_history_restore(),
            ));
        }
        let committed = self.committed(operation_id)?;
        let (result, result_error) = if committed {
            match self.restore_result(plan) {
                Ok(result) => (Some(result), None),
                Err(error) => (None, Some(error)),
            }
        } else {
            (None, None)
        };
        Ok(HistoryRestoreStatus {
            receipt: if committed {
                plan.receipt.clone()
            } else {
                None
            },
            result_error,
            operation_id: operation_id.into(),
            status: if committed {
                "committed"
            } else if self.resolved(operation_id)? {
                "resolved"
            } else {
                "recoveryPending"
            }
            .into(),
            result,
        })
    }
    /// Result-delivery fallback must still belong to the exact original input;
    /// an error code alone cannot distinguish conflict from a later read error.
    pub fn history_restore_status_for_request(
        &self,
        request: &HistoryRestoreRequest,
    ) -> Result<HistoryRestoreStatus> {
        valid_operation_id(&request.operation_id)?;
        self.check_restore_request(request)?;
        self.history_restore_status(&request.operation_id)
    }
    fn check_restore_request(&self, request: &HistoryRestoreRequest) -> Result<()> {
        if let Some(plan) = self.operations.get(&request.operation_id) {
            let fingerprint = hash(&serde_json::to_vec(request)?);
            if plan
                .history_restore
                .as_ref()
                .is_none_or(|binding| binding.fingerprint != fingerprint)
            {
                return Err(CoreError::new(
                    "operationConflict",
                    UiMessage::error_core_history_original_restore_input_immutable_operation_id_belongs_to_different_request(),
                ));
            }
        }
        Ok(())
    }
    pub fn restore_history(
        &mut self,
        request: HistoryRestoreRequest,
    ) -> Result<HistoryRestoreResult> {
        valid_operation_id(&request.operation_id)?;
        self.check_attempt_not_cancelled(&request.operation_id)?;
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        // Reconcile the original exact operation before current session/Base
        // checks. A lost response may have already advanced that Base.
        if let Some(plan) = self.operations.get(&request.operation_id) {
            self.check_restore_request(&request)?;
            if self.committed(&request.operation_id)? {
                return self.restore_result(plan);
            }
            return Err(CoreError::new(
                "recoveryPending",
                UiMessage::error_core_history_original_restore_may_have_effects_inspect_operation_and_do_not_create_another_save(),
            ));
        }
        if !matches!(request.draft_action.as_str(), "retain" | "none") {
            return Err(CoreError::new(
                "invalidDraftAction",
                UiMessage::error_core_history_choose_retain_or_none_explicitly(),
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
        self.restore_history_preflight_impl(request, fingerprint, None)
    }
    pub(crate) fn restore_history_at(
        &mut self,
        root: &Path,
        path: &Path,
        request: HistoryRestoreRequest,
    ) -> Result<HistoryRestoreResult> {
        if !matches!(request.draft_action.as_str(), "retain" | "none") {
            return Err(CoreError::new(
                "invalidDraftAction",
                UiMessage::error_core_history_choose_retain_or_none_explicitly(),
            ));
        }
        self.check_session_base(&request.session_id, &request.base_id)?;
        if self.sessions[&request.session_id].path != absolute_path(path)? {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_core_history_session_and_configured_owner_path_differ(),
            ));
        }
        let fingerprint = hash(&serde_json::to_vec(&request)?);
        self.restore_history_preflight_impl(request, fingerprint, Some((root, path)))
    }
    fn restore_history_preflight_impl(
        &mut self,
        request: HistoryRestoreRequest,
        fingerprint: String,
        explicit_owner: Option<(&Path, &Path)>,
    ) -> Result<HistoryRestoreResult> {
        let context = self.history_context(&request.session_id)?;
        if context.in_trash {
            return Err(CoreError::new(
                "readOnlyLifecycle",
                UiMessage::error_core_history_restore_note_from_trash_before_replacing_content_with_historical_version(),
            ));
        }
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
                UiMessage::error_core_history_unsaved_draft_exists_explicitly_retain_it_before_restoring(),
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
                        scope: self.draft_session_scope(&request.session_id)?,
                        path: context.current.path.clone(),
                        base_source: current_source.clone(),
                        source: source.clone(),
                        base_known: true,
                        identity: context.birth.identity.clone(),
                        alternatives: Vec::new(),
                        producer_id: None,
                    });
                }
            }
            for draft in to_protect {
                let name = fresh();
                journal::private_create(
                    &self.backend,
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
        let outcome = self.save_with_restore(
            SaveRequest {
                session_id: request.session_id,
                base_id: request.base_id,
                source: selected.source,
                operation_id: operation_id.clone(),
            },
            Some(binding),
        )?;
        let mut result = if let Some((root, path)) = explicit_owner {
            self.restore_result_at(&self.operations[&operation_id], root, path)?
        } else {
            self.restore_result(&self.operations[&operation_id])?
        };
        result.recovery_issue = outcome.recovery_issue;
        Ok(result)
    }
}
fn append_draft_alternatives(out: &mut Vec<DraftAlternative>, draft: &Draft) {
    for proposal in std::iter::once(DraftAlternative {
        path: draft.path.clone(),
        base_source: draft.base_source.clone(),
        source: draft.source.clone(),
        base_known: draft.base_known,
        producer_id: draft.producer_id.clone(),
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
pub(crate) fn history_relative_path(path: &Path, prefix: &Path) -> Option<PathBuf> {
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
pub(crate) fn same_placement_path(
    backend: &crate::BackendContext,
    expected: &Path,
    actual: &Path,
) -> Result<bool> {
    if !history_relative_path(expected, actual).is_some_and(|p| p.as_os_str().is_empty()) {
        return Ok(false);
    }
    // Resolve only these two explicit parent directories. Their final handle
    // coordinates preserve real OS names without Unicode case folding.
    let a = ParentGuard::acquire(backend, expected)?;
    let b = ParentGuard::acquire(backend, actual)?;
    Ok(a.directory_identity()? == b.directory_identity()?
        && a.directory_coordinates()? == b.directory_coordinates()?
        && expected
            .file_name()
            .zip(actual.file_name())
            .is_some_and(|(a, b)| {
                a == b
                    || a.to_string_lossy()
                        .eq_ignore_ascii_case(&b.to_string_lossy())
            }))
}
