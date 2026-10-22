//! One current Draft per original physical file and qualified input scope.
//! Committed relocations may change its locator; no pathname or unbound fallback
//! can upgrade retained input into a different library's current authority.
use super::*;
use std::collections::BTreeSet;
use weftext_messages::UiMessage;
const MAX_DRAFT_RECORD_BYTES: usize = 16 * 1024 * 1024;
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct DraftMetadata {
    pub scope: QualifiedInputScope,
    pub identity: FileIdentity,
}
pub(super) fn read_draft_metadata(file: &mut (impl Read + std::io::Seek)) -> Result<DraftMetadata> {
    let fields =
        hosted::read_metadata_prefix(file, &[("scope", false), ("identity", false)], 4096)?
            .ok_or_else(|| {
                CoreError::new(
                    "unsupportedInputScope",
                    "Draft lacks supported original metadata; original bytes remain unchanged",
                )
            })?;
    hosted::decode_metadata(fields)
}
fn encode_draft_record(record: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(record)?;
    if bytes.len() > MAX_DRAFT_RECORD_BYTES {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_core_draft_recovery_draft_record_exceeds_byte_budget(),
        ));
    }
    Ok(bytes)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftAlternative {
    pub path: String,
    pub base_source: String,
    pub source: String,
    pub base_known: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_id: Option<String>,
}

impl Draft {
    pub(super) fn proposal(&self) -> DraftAlternative {
        DraftAlternative {
            path: self.path.clone(),
            base_source: self.base_source.clone(),
            source: self.source.clone(),
            base_known: self.base_known,
            producer_id: self.producer_id.clone(),
        }
    }

    // All strict fields participate. Unlike source-only comparisons this also
    // changes when a competing variant arrives and the primary later returns
    // to the same text.
    pub(super) fn witness(&self) -> Result<String> {
        self.validate_producers()?;
        Ok(hash(&encode_draft_record(self)?))
    }

    pub(super) fn validate_producers(&self) -> Result<()> {
        for id in std::iter::once(self.producer_id.as_ref())
            .chain(self.alternatives.iter().map(|p| p.producer_id.as_ref()))
            .flatten()
        {
            if !Uuid::parse_str(id)
                .is_ok_and(|uuid| uuid.to_string() == *id && uuid.get_version_num() == 4)
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Draft input producer is not canonical",
                ));
            }
        }
        Ok(())
    }
}

/// A close verifier identifies the existing session and expected original
/// input. It does not save author content or accept a caller-provided path.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DocumentDraftFlush {
    pub session_id: String,
    pub base_id: String,
    pub source_digest: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDraftReceipt {
    pub session_id: String,
    pub base_id: String,
    pub source_digest: String,
    pub record_digest: String,
}

/// Explicitly continue the primary actually installed in a recovery editor.
/// The caller supplies the reviewed record, never a producer identifier.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResumeDraftRequest {
    pub session_id: String,
    pub record_witness: String,
    pub recovered_source: String,
}
impl Core {
    pub fn resume_recovered_draft(&mut self, request: ResumeDraftRequest) -> Result<()> {
        self.check_actor_session(&request.session_id)?;
        check_size(request.recovered_source.as_bytes())?;
        let session = self
            .sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?;
        // Establish the retained original authority before reading author
        // bytes for the explicit continuation; a current path is not a grant.
        let scope =
            self.draft_scope_for_session(&request.session_id, session.conflicting_draft.as_ref())?;
        self.validate_original_input_scope(&scope)?;
        // A path-equivalent replacement, changed disk basis or pending operation
        // cannot transfer an original producer's continuation authority.
        self.check_pending(&session.path)?;
        let guard = ParentGuard::acquire(&self.backend, &session.path)?;
        let mut file = guard.open_file(false, false)?;
        if stamp(&file)? != session.stamp
            || read_source(&mut file)? != session.bytes
            || stamp(&file)? != session.stamp
        {
            return Err(CoreError::new(
                "draftChanged",
                "The actual source basis changed before recovery continuation",
            ));
        }
        let draft = self
            .current_session_draft(&request.session_id)?
            .ok_or_else(|| {
                CoreError::new("draftChanged", "The reviewed recovery record is absent")
            })?;
        let witness = draft.witness()?;
        if witness != request.record_witness
            || session.draft_witness.as_ref() != Some(&witness)
            || draft.source != request.recovered_source
        {
            return Err(CoreError::new(
                "draftChanged",
                "Review the complete current recovery record again",
            ));
        }
        let producer = draft.producer_id.clone().ok_or_else(|| {
            CoreError::new(
                "draftProducerUnavailable",
                "This recovery record has no transferable producer",
            )
        })?;
        if session.draft_producer_id == producer
            && session
                .draft_input
                .as_ref()
                .is_some_and(|input| same_proposal(input, &draft.proposal()))
        {
            return Ok(()); // lost acknowledgement of this same in-memory CAS
        }
        if session.draft_input.is_some() {
            return Err(CoreError::new(
                "draftProducerUnavailable",
                "This session already acknowledged independent input",
            ));
        }
        if self
            .sessions
            .iter()
            .any(|(id, owner)| id != &request.session_id && owner.draft_producer_id == producer)
        {
            return Err(CoreError::new(
                "draftProducerActive",
                "The original input producer still has a live Core session",
            ));
        }
        let session = self.sessions.get_mut(&request.session_id).unwrap();
        session.draft_producer_id = producer;
        session.draft_input = Some(draft.proposal());
        Ok(())
    }

    pub fn flush_document_draft(
        &mut self,
        request: DocumentDraftFlush,
    ) -> Result<DocumentDraftReceipt> {
        self.check_actor_session(&request.session_id)?;
        if request.source_digest.len() != 64
            || !request
                .source_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(CoreError::new(
                "draftFlushMismatch",
                "Expected source digest is not canonical",
            ));
        }
        let session = self
            .sessions
            .get(&request.session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?;
        if session.base_id != request.base_id {
            return Err(CoreError::new(
                "draftFlushMismatch",
                "The document base changed",
            ));
        }
        let scope =
            self.draft_scope_for_session(&request.session_id, session.conflicting_draft.as_ref())?;
        let path = self.object_draft_path(&session.stamp.identity, &scope)?;
        let guard = ParentGuard::acquire(&self.backend, &path)?;
        let mut file = guard.open_file(false, false)?;
        // Read only original scope metadata before buffering author payload.
        if file_save_scope::read_input_scope(&mut file)? != scope {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_draft_input_scope_differs(),
            ));
        }
        self.validate_original_input_scope(&scope)?;
        let before = stamp(&file)?;
        if before.size > MAX_DRAFT_RECORD_BYTES as u64 {
            return Err(CoreError::new(
                "budget",
                UiMessage::error_core_draft_recovery_draft_record_exceeds_byte_budget(),
            ));
        }
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(MAX_DRAFT_RECORD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_DRAFT_RECORD_BYTES || stamp(&file)? != before {
            return Err(CoreError::new(
                "draftFlushMismatch",
                "The complete draft changed during readback",
            ));
        }
        let draft: Draft = serde_json::from_slice(&bytes)?;
        draft.validate_producers()?;
        self.validate_original_input_scope(&draft.scope)?;
        let input = session.draft_input.as_ref().ok_or_else(|| {
            CoreError::new(
                "draftFlushMismatch",
                "This input producer has no acknowledged draft",
            )
        })?;
        if draft.scope != scope
            || draft.identity != session.stamp.identity
            || hash(input.source.as_bytes()) != request.source_digest
            || !std::iter::once(draft.proposal())
                .chain(draft.alternatives.iter().cloned())
                .any(|p| same_proposal(&p, input))
        {
            return Err(CoreError::new(
                "draftFlushMismatch",
                "The persisted draft does not contain the prepared input",
            ));
        }
        self.check_committed_draft_paths(
            &scope,
            &session.stamp.identity,
            &draft.path,
            &display(&session.path),
        )?;
        Ok(DocumentDraftReceipt {
            session_id: request.session_id,
            base_id: request.base_id,
            source_digest: request.source_digest,
            record_digest: hash(&bytes),
        })
    }

    /// One guarded authority; retain original bytes for explicit archival.
    pub(super) fn current_session_draft_record(
        &self,
        session_id: &str,
    ) -> Result<Option<(Draft, Vec<u8>)>> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?;
        let scope = self.draft_scope_for_session(session_id, session.conflicting_draft.as_ref())?;
        let path = self.object_draft_path(&session.stamp.identity, &scope)?;
        let Some(bytes) = self.read_draft_record(&path, &scope)? else {
            return Ok(None);
        };
        let draft: Draft = serde_json::from_slice(&bytes)?;
        if draft.identity != session.stamp.identity || draft.scope != scope {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_draft_recovery_draft_object_provenance_differs(),
            ));
        }
        self.check_committed_draft_paths(
            &scope,
            &session.stamp.identity,
            &draft.path,
            &display(&session.path),
        )?;
        draft.validate_producers()?;
        Ok(Some((draft, bytes)))
    }

    pub(super) fn current_session_draft(&self, session_id: &str) -> Result<Option<Draft>> {
        Ok(self
            .current_session_draft_record(session_id)?
            .map(|(draft, _)| draft))
    }

    pub(super) fn store_session_draft(
        &mut self,
        session_id: &str,
        mut draft: Draft,
    ) -> Result<Draft> {
        let session =
            self.sessions.get(session_id).cloned().ok_or_else(|| {
                CoreError::new("sessionMissing", UiMessage::error_reopen_source())
            })?;
        if draft.producer_id.as_deref() != Some(&session.draft_producer_id) {
            return Err(CoreError::new("privateEvidence", "Draft producer differs"));
        }
        let latest = self.current_session_draft(session_id)?;
        let previous_witness = latest.as_ref().map(Draft::witness).transpose()?;
        let mut alternatives = Vec::<DraftAlternative>::new();
        if let Some(latest) = latest {
            for proposal in std::iter::once(latest.proposal()).chain(latest.alternatives) {
                if proposal.producer_id.as_deref() == Some(&session.draft_producer_id) {
                    continue;
                }
                if !alternatives.iter().any(|p| same_proposal(p, &proposal)) {
                    alternatives.push(proposal);
                }
            }
        }
        draft.alternatives = alternatives;
        let next_witness = draft.witness()?; // includes the complete merged budget
        self.store_draft(&draft)?;
        let session = self.sessions.get_mut(session_id).unwrap();
        // A persist acknowledgement does not mean that the user reviewed a
        // concurrent producer's newly arrived variants. Only an uninterrupted
        // sequence of this producer's own changes advances its review witness.
        if session.draft_witness == previous_witness {
            session.draft_witness = Some(next_witness);
        }
        session.draft_input = Some(draft.proposal());
        Ok(draft)
    }

    pub(crate) fn draft_session_scope(&self, session_id: &str) -> Result<QualifiedInputScope> {
        self.sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", UiMessage::error_reopen_source()))?
            .file_scope
            .clone()
    }
    pub(crate) fn draft_scope_for_session(
        &self,
        session_id: &str,
        retained: Option<&Draft>,
    ) -> Result<QualifiedInputScope> {
        let scope = self.draft_session_scope(session_id)?;
        self.validate_original_input_scope(&scope)?;
        let session = &self.sessions[session_id];
        if session.draft_placement_pending {
            return Err(unproved_draft_placement());
        }
        if retained.is_some_and(|draft| draft.scope != scope) {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_draft_retained_scope_differs(),
            ));
        }
        let path = self.object_draft_path(&session.stamp.identity, &scope)?;
        if let Some((original, _)) =
            self.read_draft_record(&path, &scope, &session.stamp.identity)?
        {
            if original.identity != session.stamp.identity {
                return Err(CoreError::new(
                    "privateEvidence",
                    UiMessage::error_core_draft_recovery_draft_object_provenance_differs(),
                ));
            }
            self.check_committed_draft_paths(
                &scope,
                &session.stamp.identity,
                &original.path,
                &display(&session.path),
            )?;
        }
        Ok(scope)
    }
    fn check_committed_draft_paths(
        &self,
        scope: &QualifiedInputScope,
        identity: &FileIdentity,
        first: &str,
        second: &str,
    ) -> Result<()> {
        if same_draft_path(first, second) {
            return Ok(());
        }
        let mut paths = BTreeSet::from([first.to_owned()]);
        loop {
            let previous = paths.len();
            for plan in self.committed_plans()?.into_iter() {
                let Some(transaction) = &plan.transaction else {
                    continue;
                };
                if !matches!(scope, QualifiedInputScope::Library { binding } if plan.workspace_id.as_ref() == Some(&binding.workspace_id))
                {
                    continue;
                }
                // These are original committed before/after coordinates for
                // this exact captured file. An old session's slot may now hold
                // a different participant, so never borrow its current FI.
                for file in transaction
                    .files
                    .iter()
                    .filter(|f| &f.stamp.identity == identity)
                {
                    let after = transaction.relocated_path(&file.path);
                    if paths.iter().any(|path| {
                        same_draft_path(path, &file.path) || same_draft_path(path, &after)
                    }) {
                        paths.insert(file.path.clone());
                        paths.insert(after);
                    }
                }
            }
            if paths.iter().any(|path| same_draft_path(path, second)) {
                return Ok(());
            }
            if paths.len() == previous {
                return Err(unproved_draft_placement());
            }
        }
    }

    fn current_draft_lineage(
        &self,
        scope: &QualifiedInputScope,
        path: &Path,
        identity: &FileIdentity,
    ) -> Result<BTreeSet<String>> {
        let mut lineage = BTreeSet::from([display(path)]);
        loop {
            let before = lineage.len();
            for plan in self
                .operations
                .values()
                .filter(|plan| self.committed(&plan.operation_id))
            {
                if !matches!(scope, QualifiedInputScope::Library { binding }
                    if plan.workspace_id.as_ref() == Some(&binding.workspace_id))
                {
                    continue;
                }
                let Some(transaction) = &plan.transaction else {
                    continue;
                };
                for operations::RelocationRef {
                    source,
                    target,
                    old_leaf,
                    new_leaf,
                    source_identity,
                    leaf_identity,
                    ..
                } in transaction.relocations()
                {
                    let mut predecessors = Vec::new();
                    for prior in &lineage {
                        let Some(relative) =
                            history::history_relative_path(Path::new(prior), Path::new(target))
                        else {
                            continue;
                        };
                        let Some(actual_root) = path.ancestors().nth(relative.components().count())
                        else {
                            continue;
                        };
                        let guard = ParentGuard::acquire(
                            &self.backend,
                            &actual_root.join(".draft-prospective"),
                        )?;
                        if guard.directory_identity()? != *source_identity {
                            continue;
                        }
                        let previous = if same_draft_path(&display(&relative), new_leaf) {
                            if leaf_identity != identity {
                                continue;
                            }
                            Path::new(source).join(old_leaf)
                        } else {
                            Path::new(source).join(relative)
                        };
                        predecessors.push(display(&previous));
                    }
                    lineage.extend(predecessors);
                }
            }
            if before == lineage.len() {
                return Ok(lineage);
            }
        }
    }

    fn object_draft_path(
        &self,
        identity: &FileIdentity,
        scope: &QualifiedInputScope,
    ) -> Result<PathBuf> {
        Ok(self.draft_directory("drafts").join(format!(
            "object-{}.json",
            hash(&serde_json::to_vec(&(identity, scope))?)
        )))
    }
    pub(super) fn store_draft(&self, draft: &Draft) -> Result<()> {
        self.validate_original_input_scope(&draft.scope)?;
        draft.validate_producers()?;
        let identity = &draft.identity;
        let bytes = encode_draft_record(draft)?;
        let path = self.object_draft_path(identity, &draft.scope)?;
        // Inspect metadata before replacing the one current record.
        let _ = self.read_draft_record(&path, &draft.scope, &draft.identity)?;
        private_replace(&self.backend, &path, &bytes)
    }
    pub(super) fn read_draft_record(
        &self,
        path: &Path,
        scope: &QualifiedInputScope,
        identity: &FileIdentity,
    ) -> Result<Option<(Draft, Vec<u8>)>> {
        let guard = match ParentGuard::acquire(&self.backend, path) {
            Ok(guard) => guard,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut file = match guard.open_read_guard_file() {
            Ok(file) => file,
            Err(error) if error.code == "missing" => return Ok(None),
            Err(error) => return Err(error),
        };
        let Some((metadata, bytes)) = hosted::read_qualified_private_record(&mut file, |file| {
            let metadata = read_draft_metadata(file)?;
            self.validate_original_input_scope(&metadata.scope)?;
            if metadata.scope != *scope || metadata.identity != *identity {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Draft original input scope or physical identity differs",
                ));
            }
            Ok(Some((metadata, MAX_DRAFT_RECORD_BYTES)))
        })?
        else {
            return Ok(None);
        };
        let draft: Draft = serde_json::from_slice(&bytes)?;
        if draft.scope != metadata.scope || draft.identity != metadata.identity {
            return Err(CoreError::new(
                "privateEvidence",
                "Draft metadata changed during its qualified read",
            ));
        }
        Ok(Some((draft, bytes)))
    }
    #[cfg(test)]
    pub(super) fn draft_path(&self, path: &Path) -> PathBuf {
        let actual = ParentGuard::acquire(&self.backend, path)
            .and_then(|guard| guard.open_file(false, false))
            .and_then(|file| stamp(&file));
        if let (Ok(actual), Ok(scope)) = (actual, self.capture_file_save_scope(path)) {
            return self.object_draft_path(&actual.identity, &scope).unwrap();
        }
        self.legacy_draft_path(path)
    }
    pub(super) fn read_relocated_draft(&self, path: &Path) -> Result<Option<Draft>> {
        let guard = ParentGuard::acquire(&self.backend, path)?;
        let mut current = guard.open_file(false, false)?;
        let identity = stamp(&current)?.identity;
        let source = decode(&read_source(&mut current)?)?;
        self.read_relocated_draft_with_identity(
            path.parent().unwrap_or(path),
            path,
            &identity,
            &source,
        )
        .map(|(draft, _)| draft)
    }
    pub(super) fn read_relocated_draft_with_identity(
        &self,
        _root: &Path,
        path: &Path,
        identity: &FileIdentity,
        _current_source: &str,
    ) -> Result<(Option<Draft>, bool)> {
        let scope = self.capture_file_save_scope(path)?;
        let primary = self.object_draft_path(identity, &scope)?;
        let Some((mut draft, bytes)) = self.read_draft_record(&primary, &scope, identity)? else {
            return Ok((None, false));
        };
        draft.validate_producers()?;
        if draft.scope != scope || &draft.identity != identity {
            return Err(CoreError::new(
                "privateEvidence",
                UiMessage::error_core_original_scope_draft_participant_differs(),
            ));
        }
        if !same_draft_path(&draft.path, &display(path)) {
            let lineage = self.current_draft_lineage(&scope, path, identity)?;
            if !lineage
                .iter()
                .any(|prior| same_draft_path(&draft.path, prior))
            {
                return Ok((None, true));
            }
            let mut next = draft.clone();
            next.path = display(path);
            let _ = encode_draft_record(&next)?;
            // Preserve the original before rebinding the unique current record.
            journal::private_create(
                &self.backend,
                &self
                    .draft_directory("draft-history")
                    .join(format!("{}.json", fresh())),
                &bytes,
            )?;
            draft = next;
            self.store_draft(&draft)?;
        }
        Ok((Some(draft), false))
    }
}
fn unproved_draft_placement() -> CoreError {
    CoreError::new(
        "identityPending",
        UiMessage::error_core_draft_recovery_source_position_has_no_committed_draft_relocation_original_proposal_unchanged_and_current_input_must_remain_in_editor(),
    )
}
fn same_proposal(a: &DraftAlternative, b: &DraftAlternative) -> bool {
    a.producer_id == b.producer_id
        && a.source == b.source
        && a.base_source == b.base_source
        && a.base_known == b.base_known
        && same_draft_path(&a.path, &b.path)
}
fn same_draft_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .eq_ignore_ascii_case(&b.replace('/', "\\"))
}

#[cfg(all(test, windows))]
mod flush_tests {
    use super::*;

    fn flush(core: &mut Core, doc: &OpenedDocument, source: &str) -> DocumentDraftReceipt {
        core.flush_document_draft(DocumentDraftFlush {
            session_id: doc.session_id.clone(),
            base_id: doc.base_id.clone(),
            source_digest: hash(source.as_bytes()),
        })
        .unwrap()
    }

    fn sources(core: &Core, doc: &OpenedDocument) -> Vec<String> {
        let draft = core
            .current_session_draft(&doc.session_id)
            .unwrap()
            .unwrap();
        std::iter::once(draft.source)
            .chain(draft.alternatives.into_iter().map(|p| p.source))
            .collect()
    }

    fn resume(core: &mut Core, doc: &OpenedDocument) -> Result<()> {
        core.resume_recovered_draft(ResumeDraftRequest {
            session_id: doc.session_id.clone(),
            record_witness: doc.recovered_draft_witness.clone().unwrap(),
            recovered_source: doc.recovered_draft.clone().unwrap(),
        })
    }

    #[test]
    fn explicit_recovery_continuation_after_restart_reuses_only_original_producer() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let state = temp.path().join("state");
        let mut core = Core::new(&state).unwrap();
        let a = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "last input").unwrap();
        let producer = core.sessions[&a.session_id].draft_producer_id.clone();
        drop(core);
        let mut core = Core::new(&state).unwrap();
        let reopened = core.open(&path, None).unwrap();
        assert_ne!(
            core.sessions[&reopened.session_id].draft_producer_id,
            producer
        );
        let record_path = core.draft_path(&path);
        let before = fs::read(&record_path).unwrap();
        resume(&mut core, &reopened).unwrap();
        resume(&mut core, &reopened).unwrap(); // same lost-ACK CAS
        assert_eq!(fs::read(&record_path).unwrap(), before);
        assert_eq!(
            core.sessions[&reopened.session_id].draft_producer_id,
            producer
        );
        core.persist_draft(&reopened.session_id, "continued input")
            .unwrap();
        assert_eq!(sources(&core, &reopened), vec!["continued input"]);
        flush(&mut core, &reopened, "continued input");
    }

    #[test]
    fn recovery_continuation_requires_explicit_session_retirement_and_complete_current_review() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let a = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "A").unwrap();
        let b = core.open(&path, None).unwrap();
        assert_eq!(
            resume(&mut core, &b).unwrap_err().code,
            "draftProducerActive"
        );
        core.close_session(&a.session_id).unwrap();
        let c = core.open(&path, None).unwrap();
        core.persist_draft(&c.session_id, "new competing input")
            .unwrap();
        let before = fs::read(core.draft_path(&path)).unwrap();
        assert_eq!(resume(&mut core, &b).unwrap_err().code, "draftChanged");
        assert_eq!(fs::read(core.draft_path(&path)).unwrap(), before);
        let d = core.open(&path, None).unwrap();
        core.close_session(&c.session_id).unwrap();
        resume(&mut core, &d).unwrap();
        core.persist_draft(&d.session_id, "continued C").unwrap();
        assert_eq!(sources(&core, &d), vec!["continued C", "A"]);
    }

    #[test]
    fn recovery_continuation_rejects_replaced_source_object_without_modifying_record() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let a = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "A").unwrap();
        core.close_session(&a.session_id).unwrap();
        let b = core.open(&path, None).unwrap();
        let before = fs::read(core.draft_path(&path)).unwrap();
        fs::rename(&path, temp.path().join("original.typ")).unwrap();
        fs::write(&path, "original").unwrap();
        assert_eq!(resume(&mut core, &b).unwrap_err().code, "draftChanged");
        assert_eq!(fs::read(core.draft_path(&path)).unwrap(), before);
    }

    #[test]
    fn interleaved_producers_preserve_acknowledged_input_across_restart_and_coalesce_only_their_own_input()
     {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let state = temp.path().join("state");
        let mut core = Core::new(&state).unwrap();
        let a = core.open(&path, None).unwrap();
        let b = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "A").unwrap();
        let original_receipt = flush(&mut core, &a, "A");
        core.persist_draft(&b.session_id, "B").unwrap();
        let later_receipt = flush(&mut core, &a, "A");
        assert_ne!(original_receipt.record_digest, later_receipt.record_digest);
        for i in 0..100 {
            core.persist_draft(&a.session_id, &format!("A{i}")).unwrap();
        }
        let mut saved = sources(&core, &a);
        saved.sort();
        assert_eq!(saved, vec!["A99", "B"]);
        flush(&mut core, &b, "B");
        drop(core);
        let mut core = Core::new(&state).unwrap();
        let reopened = core.open(&path, None).unwrap();
        let mut saved = sources(&core, &reopened);
        saved.sort();
        assert_eq!(saved, vec!["A99", "B"]);
        assert!(reopened.recovered_draft_conflict);
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn identical_text_from_distinct_producers_does_not_transfer_coalescing_authority() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let a = core.open(&path, None).unwrap();
        let b = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "same").unwrap();
        core.persist_draft(&b.session_id, "same").unwrap();
        core.persist_draft(&b.session_id, "B later").unwrap();
        flush(&mut core, &a, "same");
        let saved = sources(&core, &a);
        assert!(saved.contains(&"same".into()));
        assert!(saved.contains(&"B later".into()));
    }

    #[test]
    fn close_readbacks_survive_competing_writes_and_save_between_both_close_phases() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let a = core.open(&path, None).unwrap();
        let b = core.open(&path, None).unwrap();
        let c = core.open(&path, None).unwrap();
        core.persist_draft(&a.session_id, "A close input").unwrap();
        flush(&mut core, &a, "A close input"); // prepare readback
        core.persist_draft(&b.session_id, "B after prepare")
            .unwrap();
        flush(&mut core, &a, "A close input"); // frozen readback
        core.save(SaveRequest {
            operation_id: fresh(),
            session_id: c.session_id.clone(),
            base_id: c.base_id.clone(),
            source: "C committed".into(),
        })
        .unwrap();
        flush(&mut core, &a, "A close input"); // another accepted writer after verification
        let saved = sources(&core, &a);
        for expected in ["A close input", "B after prepare", "C committed"] {
            assert!(saved.iter().any(|s| s == expected));
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), "C committed");
        let stale = core
            .save(SaveRequest {
                operation_id: fresh(),
                session_id: b.session_id.clone(),
                base_id: "old-base".into(),
                source: "B unknown base".into(),
            })
            .unwrap_err();
        assert_eq!(stale.code, "staleBase");
        flush(&mut core, &a, "A close input");
    }

    #[test]
    fn recovery_resolution_rejects_full_record_aba_and_preserves_new_variants() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let seed = core.open(&path, None).unwrap();
        assert_eq!(
            core.save(SaveRequest {
                operation_id: fresh(),
                session_id: seed.session_id.clone(),
                base_id: "stale".into(),
                source: "X".into()
            })
            .unwrap_err()
            .code,
            "staleBase"
        );
        let a = core.open(&path, None).unwrap();
        let b = core.open(&path, None).unwrap();
        assert!(a.recovered_draft_conflict);
        core.persist_draft(&b.session_id, "Z").unwrap();
        let c = core.open(&path, None).unwrap();
        core.persist_draft(&c.session_id, "X").unwrap();
        let record_path = core.draft_path(&path);
        let before = fs::read(&record_path).unwrap();
        for action in ["discardRecovered", "mergeRecovered"] {
            assert_eq!(
                core.resolve_draft(&a.session_id, action, Some("reviewed before Z"))
                    .unwrap_err()
                    .code,
                "draftChanged"
            );
            assert_eq!(fs::read(&record_path).unwrap(), before);
        }
        let fresh = core.open(&path, None).unwrap();
        core.persist_draft(&fresh.session_id, "explicit merge")
            .unwrap();
        core.resolve_draft(&fresh.session_id, "mergeRecovered", Some("explicit merge"))
            .unwrap();
        let archives = fs::read_dir(core.draft_directory("draft-history"))
            .unwrap()
            .map(|e| fs::read(e.unwrap().path()).unwrap())
            .collect::<Vec<_>>();
        assert!(archives.iter().any(|bytes| {
            serde_json::from_slice::<Draft>(bytes)
                .is_ok_and(|d| d.alternatives.iter().any(|p| p.source == "Z"))
        }));
    }

    #[test]
    fn competing_proposal_budget_rejects_without_ack_or_replacing_previous_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let docs = (0..8)
            .map(|_| core.open(&path, None).unwrap())
            .collect::<Vec<_>>();
        for (i, doc) in docs.iter().take(7).enumerate() {
            core.persist_draft(
                &doc.session_id,
                &format!("{i}{}", "x".repeat(MAX_SOURCE_BYTES - 1)),
            )
            .unwrap();
        }
        let record_path = core.draft_path(&path);
        let before = fs::read(&record_path).unwrap();
        let last = &docs[7];
        let source = "y".repeat(MAX_SOURCE_BYTES);
        assert_eq!(
            core.persist_draft(&last.session_id, &source)
                .unwrap_err()
                .code,
            "budget"
        );
        assert_eq!(fs::read(&record_path).unwrap(), before);
        assert!(core.sessions[&last.session_id].draft_input.is_none());
        assert!(
            core.flush_document_draft(DocumentDraftFlush {
                session_id: last.session_id.clone(),
                base_id: last.base_id.clone(),
                source_digest: hash(source.as_bytes())
            })
            .is_err()
        );
    }

    #[test]
    fn document_flush_requires_exact_persisted_source_and_base_without_mutating_original() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let opened = core.open(&path, Some(temp.path().to_path_buf())).unwrap();
        let request = |source: &str, base: &str| DocumentDraftFlush {
            session_id: opened.session_id.clone(),
            base_id: base.into(),
            source_digest: hash(source.as_bytes()),
        };
        assert!(
            core.flush_document_draft(request("中文draft😀", &opened.base_id))
                .is_err()
        );
        core.persist_draft(&opened.session_id, "中文draft😀")
            .unwrap();
        let record_path = core.draft_path(&path);
        let original = fs::read(&record_path).unwrap();
        let receipt = core
            .flush_document_draft(request("中文draft😀", &opened.base_id))
            .unwrap();
        assert_eq!(receipt.record_digest, hash(&original));
        assert_eq!(receipt.source_digest, hash("中文draft😀".as_bytes()));
        assert!(
            core.flush_document_draft(request("later input", &opened.base_id))
                .is_err()
        );
        assert!(
            core.flush_document_draft(request("中文draft😀", "different-base"))
                .is_err()
        );
        assert_eq!(fs::read(&record_path).unwrap(), original);
    }

    #[test]
    fn document_flush_rejects_malformed_private_evidence_instead_of_ui_ack() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("note.typ");
        fs::write(&path, "original").unwrap();
        let mut core = Core::new(temp.path().join("state")).unwrap();
        let opened = core.open(&path, Some(temp.path().to_path_buf())).unwrap();
        core.persist_draft(&opened.session_id, "draft").unwrap();
        let record_path = core.draft_path(&path);
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
        record["uiAcknowledged"] = serde_json::json!(true);
        let changed = serde_json::to_vec(&record).unwrap();
        fs::write(&record_path, &changed).unwrap();
        assert!(
            core.flush_document_draft(DocumentDraftFlush {
                session_id: opened.session_id,
                base_id: opened.base_id,
                source_digest: hash(b"draft")
            })
            .is_err()
        );
        assert_eq!(fs::read(&record_path).unwrap(), changed);
    }
}
