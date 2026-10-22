//! Draft paths follow committed same-object relocations, independently of UI
//! session rebinding. Migration retains every original private draft record.
use super::*;

use std::collections::BTreeSet;

const MAX_DRAFT_RECORD_BYTES: usize = 16 * 1024 * 1024;

fn encode_draft_record(record: &impl Serialize) -> Result<Vec<u8>> {
    let bytes = serde_json::to_vec(record)?;
    if bytes.len() > MAX_DRAFT_RECORD_BYTES {
        return Err(CoreError::new(
            "budget",
            "Draft record exceeds its byte budget",
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

    fn validate_producers(&self) -> Result<()> {
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
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct DraftReference {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<hosted::ActorLibraryBinding>,
    path: String,
    fingerprint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DraftMigration {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library: Option<hosted::ActorLibraryBinding>,
    identity: FileIdentity,
    consumed: Vec<DraftReference>,
}
impl Core {
    pub fn resume_recovered_draft(&mut self, request: ResumeDraftRequest) -> Result<()> {
        self.check_actor_session(&request.session_id)?;
        check_size(request.recovered_source.as_bytes())?;
        let session = self
            .sessions
            .get(&request.session_id)
            .cloned()
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        // A path-equivalent replacement, changed disk basis or pending operation
        // cannot transfer an original producer's continuation authority.
        self.check_pending(&session.path)?;
        let guard = ParentGuard::acquire(&session.path)?;
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
    /// Read back the complete guarded primary written by persist_draft. A UI
    /// acknowledgement or sessionStorage copy is never substituted for bytes.
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
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        if session.base_id != request.base_id {
            return Err(CoreError::new(
                "draftFlushMismatch",
                "The document base changed",
            ));
        }
        let binding = self
            .draft_library_for_session(&request.session_id, session.conflicting_draft.as_ref())?;
        let path = self.object_draft_path(&session.stamp.identity, &binding)?;
        let guard = ParentGuard::acquire(&path)?;
        let mut file = guard.open_file(false, false)?;
        let before = stamp(&file)?;
        if before.size > MAX_DRAFT_RECORD_BYTES as u64 {
            return Err(CoreError::new(
                "budget",
                "Draft record exceeds its byte budget",
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
        self.check_actor_library(&draft.library)?;
        let input = session.draft_input.as_ref().ok_or_else(|| {
            CoreError::new(
                "draftFlushMismatch",
                "This input producer has no acknowledged draft",
            )
        })?;
        if draft.library != binding
            || draft.identity.as_ref() != Some(&session.stamp.identity)
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
            &session.root,
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

    pub(crate) fn draft_library_for_session(
        &self,
        session_id: &str,
        retained: Option<&Draft>,
    ) -> Result<Option<hosted::ActorLibraryBinding>> {
        let binding = self.draft_session_library(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        if session.draft_placement_pending {
            return Err(unproved_draft_placement());
        }
        // Read and write select the same exact scoped primary. In particular,
        // an older unbound record cannot divert new input away from an existing
        // bound clean record. Corrupt primaries never permit legacy fallback.
        let primary = self.object_draft_path(&session.stamp.identity, &binding)?;
        if primary.exists() {
            let bytes = self
                .read_draft_record(&primary, &binding, false)?
                .ok_or_else(|| CoreError::new("privateEvidence", "Draft library scope differs"))?;
            let original: Draft = serde_json::from_slice(&bytes)?;
            if original.identity.as_ref() != Some(&session.stamp.identity)
                || original.library != binding
            {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Draft object provenance differs",
                ));
            }
            self.check_committed_draft_paths(
                &session.root,
                &session.stamp.identity,
                &original.path,
                &display(&session.path),
            )?;
            return Ok(binding);
        }
        // Preserve the original scope, including an explicitly unbound legacy
        // proposal. An unrelated record in an old pathname slot cannot retag
        // this session's object.
        if retained.is_some_and(|draft| draft.library.is_none()) {
            return Ok(None);
        }
        let unbound = self.object_draft_path(&session.stamp.identity, &None)?;
        if let Some(bytes) = self.read_draft_record(&unbound, &None, false)? {
            let original: Draft = serde_json::from_slice(&bytes)?;
            if original.identity.as_ref() == Some(&session.stamp.identity) {
                self.check_committed_draft_paths(
                    &session.root,
                    &session.stamp.identity,
                    &original.path,
                    &display(&session.path),
                )?;
                return Ok(None);
            }
        }
        if let Some(bytes) =
            self.read_draft_record(&self.legacy_draft_path(&session.path), &binding, true)?
        {
            let original: Draft = serde_json::from_slice(&bytes)?;
            if original.library.is_none()
                && original
                    .identity
                    .as_ref()
                    .is_none_or(|id| id == &session.stamp.identity)
                && same_draft_path(&original.path, &display(&session.path))
            {
                return Ok(None);
            }
        }
        let Some(original) = retained else {
            return Ok(binding);
        };
        // Never backfill a legacy recovered proposal with a new root binding.
        if original.library.is_none() {
            return Ok(None);
        }
        if original.library != binding {
            return Ok(None);
        }
        let root = &self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?
            .root;
        if std::iter::once(original.path.as_str())
            .chain(original.alternatives.iter().map(|p| p.path.as_str()))
            .any(|path| relative_draft_path(Path::new(path), root).is_none())
        {
            return Ok(None);
        }
        Ok(binding)
    }

    fn check_committed_draft_paths(
        &self,
        root: &Path,
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
            for plan in self
                .operations
                .values()
                .filter(|plan| self.committed(&plan.operation_id))
            {
                let Some(transaction) = &plan.transaction else {
                    continue;
                };
                if !same_draft_path(&transaction.request.root, &display(root)) {
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

    /// Read the same authoritative object slot used for this session's writes.
    /// An old pathname or the open-time UI snapshot never substitutes for the
    /// current primary and its complete competing proposals.
    pub(super) fn current_session_draft(&self, session_id: &str) -> Result<Option<Draft>> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
        let library =
            self.draft_library_for_session(session_id, session.conflicting_draft.as_ref())?;
        let path = self.object_draft_path(&session.stamp.identity, &library)?;
        let Some(bytes) = self.read_draft_record(&path, &library, false)? else {
            return Ok(None);
        };
        let draft: Draft = serde_json::from_slice(&bytes)?;
        if draft.identity.as_ref() != Some(&session.stamp.identity) || draft.library != library {
            return Err(CoreError::new(
                "privateEvidence",
                "Draft object provenance differs",
            ));
        }
        self.check_committed_draft_paths(
            &session.root,
            &session.stamp.identity,
            &draft.path,
            &display(&session.path),
        )?;
        Ok(Some(draft))
    }

    /// Only a producer's own previous input may be coalesced. Every other
    /// acknowledged proposal remains in the one authoritative object record,
    /// including after that producer's window or process has closed.
    pub(super) fn store_session_draft(
        &mut self,
        session_id: &str,
        mut draft: Draft,
    ) -> Result<Draft> {
        let session = self
            .sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?;
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
    /// One authoritative record per original file and original library scope.
    /// The actor is already isolated by draft_directory; the complete library
    /// binding is part of the key because an actor may use multiple libraries.
    fn object_draft_path(
        &self,
        identity: &FileIdentity,
        library: &Option<hosted::ActorLibraryBinding>,
    ) -> Result<PathBuf> {
        Ok(self.draft_directory("drafts").join(format!(
            "object-{}.json",
            hash(&serde_json::to_vec(&(identity, library))?)
        )))
    }

    pub(super) fn store_draft(&self, draft: &Draft) -> Result<()> {
        // Bound the complete proposal, including all retained alternatives,
        // before replacing the previous readable authoritative record.
        draft.validate_producers()?;
        let bytes = encode_draft_record(draft)?;
        let path = match &draft.identity {
            Some(identity) => self.object_draft_path(identity, &draft.library)?,
            None => self.legacy_draft_path(Path::new(&draft.path)),
        };
        // Inspect the original metadata before touching an existing payload.
        // A foreign scoped record must never become a writable local proposal.
        if path.exists() {
            let bytes = self
                .read_draft_record(&path, &draft.library, false)?
                .ok_or_else(|| CoreError::new("privateEvidence", "Draft library scope differs"))?;
            let original: Draft = serde_json::from_slice(&bytes)?;
            original.validate_producers()?;
        }
        private_replace(&path, &bytes)
    }

    fn read_draft_record(
        &self,
        path: &Path,
        library: &Option<hosted::ActorLibraryBinding>,
        allow_unbound: bool,
    ) -> Result<Option<Vec<u8>>> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let original = match hosted::read_actor_library_header(&mut file)? {
            hosted::ActorLibraryHeader::Bound(binding) => Some(binding),
            hosted::ActorLibraryHeader::LegacyUnbound => None,
        };
        if original != *library && !(allow_unbound && original.is_none()) {
            return Ok(None);
        }
        self.check_actor_library(&original)?;
        if file.metadata()?.len() > MAX_DRAFT_RECORD_BYTES as u64 {
            return Err(CoreError::new(
                "budget",
                "Draft record exceeds its byte budget",
            ));
        }
        let mut bytes = Vec::new();
        file.take(MAX_DRAFT_RECORD_BYTES as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > MAX_DRAFT_RECORD_BYTES {
            return Err(CoreError::new(
                "budget",
                "Draft record exceeds its byte budget",
            ));
        }
        Ok(Some(bytes))
    }

    fn draft_root_for_file(&self, path: &Path, identity: &FileIdentity) -> PathBuf {
        if let Some(root) = &self.actor_library_path {
            // binding.root is original volume-GUID evidence, not a caller path.
            // Keep the trusted, already qualified ordinary locator separately.
            return root.clone();
        }
        if let Some(session) = self
            .sessions
            .values()
            .filter(|session| {
                session.actor == self.actor
                    && session.stamp.identity == *identity
                    && relative_draft_path(path, &session.root).is_some()
            })
            .max_by_key(|session| session.root.components().count())
        {
            return session.root.clone();
        }
        self.operations
            .values()
            .filter(|plan| self.committed(&plan.operation_id))
            .filter_map(|plan| {
                if let Some(transaction) = &plan.transaction {
                    Some(PathBuf::from(&transaction.request.root))
                } else if plan.kind == "createLibrary" {
                    Some(PathBuf::from(&plan.path))
                } else {
                    None
                }
            })
            .filter(|root| relative_draft_path(path, root).is_some())
            .max_by_key(|root| root.components().count())
            .unwrap_or_else(|| path.parent().unwrap_or(path).to_path_buf())
    }

    // Test fixtures may inspect the current primary record, or deliberately
    // write an exact legacy pathname record for a target not yet materialized.
    #[cfg(test)]
    pub(super) fn draft_path(&self, path: &Path) -> PathBuf {
        let identity = ParentGuard::acquire(path)
            .and_then(|guard| guard.open_file(false, false))
            .and_then(|file| stamp(&file))
            .map(|stamp| stamp.identity);
        if let Ok(identity) = identity {
            let root = self.draft_root_for_file(path, &identity);
            if let Ok(binding) = self.draft_library_for_root(&root) {
                for scope in [binding, None] {
                    if let Ok(primary) = self.object_draft_path(&identity, &scope)
                        && primary.exists()
                    {
                        return primary;
                    }
                }
            }
        }
        self.legacy_draft_path(path)
    }

    pub(super) fn read_relocated_draft(&self, path: &Path) -> Result<Option<Draft>> {
        let guard = ParentGuard::acquire(path)?;
        let mut current = guard.open_file(false, false)?;
        let identity = stamp(&current)?.identity;
        let current_source = decode(&read_source(&mut current)?)?;
        let root = self.draft_root_for_file(path, &identity);
        self.read_relocated_draft_with_identity(&root, path, &identity, &current_source)
            .map(|(draft, _)| draft)
    }
    pub(super) fn read_relocated_draft_with_identity(
        &self,
        root: &Path,
        path: &Path,
        identity: &FileIdentity,
        current_source: &str,
    ) -> Result<(Option<Draft>, bool)> {
        let binding = self.draft_library_for_root(root)?;
        let mut lineage = BTreeSet::from([display(path)]);
        // Every edge must bind the canonical file and its actual directory.
        // Equal names or equal source bytes never substitute for this evidence.
        loop {
            let prior_len = lineage.len();
            for plan in self
                .operations
                .values()
                .filter(|p| self.committed(&p.operation_id))
            {
                if let Some(t) = &plan.transaction {
                    for relocation in t.relocations() {
                        let operations::RelocationRef {
                            source,
                            target,
                            old_leaf,
                            new_leaf,
                            source_identity,
                            leaf_identity,
                            ..
                        } = relocation;
                        let mut predecessors = Vec::new();
                        for prior in &lineage {
                            let Some(relative) =
                                relative_draft_path(Path::new(prior), Path::new(target))
                            else {
                                continue;
                            };
                            let Some(actual_root) =
                                path.ancestors().nth(relative.components().count())
                            else {
                                continue;
                            };
                            let actual_guard =
                                ParentGuard::acquire(&actual_root.join(".draft-prospective"))?;
                            if actual_guard.directory_identity()? != *source_identity {
                                continue;
                            }
                            let previous = if relative == Path::new(new_leaf) {
                                if leaf_identity != identity {
                                    continue;
                                }
                                Path::new(source).join(old_leaf)
                            } else {
                                Path::new(source).join(&relative)
                            };
                            predecessors.push(display(&previous));
                        }
                        lineage.extend(predecessors);
                    }
                }
            }
            if lineage.len() == prior_len {
                break;
            }
        }
        let current_path = display(path);
        let mut primary = None;
        for scope in [binding.clone(), None] {
            let record_path = self.object_draft_path(identity, &scope)?;
            let bytes = self.read_draft_record(&record_path, &scope, false)?;
            if bytes.is_none() && record_path.exists() {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Draft library scope differs",
                ));
            }
            if let Some(bytes) = bytes {
                let draft: Draft = serde_json::from_slice(&bytes)?;
                if draft.library != scope || draft.identity.as_ref() != Some(identity) {
                    return Err(CoreError::new(
                        "privateEvidence",
                        "Draft object provenance differs",
                    ));
                }
                if !lineage
                    .iter()
                    .any(|prior| same_draft_path(&draft.path, prior))
                {
                    // An external rename may leave this same file at a path
                    // with no committed relocation proof. Source viewing is
                    // still allowed, but must not migrate, consume or replace
                    // its authoritative proposal with a pathname candidate.
                    return Ok((None, true));
                }
                primary = Some((draft, bytes));
                break;
            }
        }
        // A clean or explicitly resolved primary is still authoritative. Never
        // recover an older pathname proposal over this deliberate current state.
        if let Some((draft, _)) = &primary
            && same_draft_path(&draft.path, &current_path)
        {
            return Ok((Some(draft.clone()), false));
        }
        let mut consumed = Vec::<DraftReference>::new();
        let mut migration_paths = lineage
            .iter()
            .map(|prior| {
                self.legacy_draft_path(Path::new(prior))
                    .with_extension("relocation.json")
            })
            .collect::<BTreeSet<_>>();
        for scope in [binding.clone(), None] {
            migration_paths.insert(
                self.object_draft_path(identity, &scope)?
                    .with_extension("relocation.json"),
            );
        }
        for migration_path in migration_paths {
            if let Some(bytes) = self.read_draft_record(&migration_path, &binding, true)? {
                let migration: DraftMigration = serde_json::from_slice(&bytes)?;
                if &migration.identity == identity {
                    for mut reference in migration.consumed {
                        reference.path = reference.path.replace('/', "\\").to_lowercase();
                        if !consumed.contains(&reference) {
                            consumed.push(reference);
                        }
                    }
                }
            }
        }
        let mut candidates = Vec::<Draft>::new();
        let mut imported = Vec::<DraftReference>::new();
        // On first relocation a genuine current-slot legacy proposal may
        // compete with the original primary. Merge it once, then consume its
        // exact fingerprint. Old source slots are inputs only without a primary.
        let ordered = std::iter::once(&current_path).chain(
            lineage
                .iter()
                .filter(|prior| *prior != &current_path && primary.is_none()),
        );
        for prior in ordered {
            let paths = BTreeSet::from([
                self.legacy_draft_path(Path::new(prior)),
                self.draft_directory("drafts")
                    .join(format!("{}.json", hash(prior.as_bytes()))),
            ]);
            for record_path in paths {
                let Some(bytes) = self.read_draft_record(&record_path, &binding, true)? else {
                    continue;
                };
                let mut draft: Draft = serde_json::from_slice(&bytes)?;
                let reference = DraftReference {
                    library: draft.library.clone(),
                    path: prior.replace('/', "\\").to_lowercase(),
                    fingerprint: hash(&bytes),
                };
                if consumed.contains(&reference)
                    // Existing primaries remain within their original scope.
                    // A different-scope legacy record is preserved verbatim,
                    // without creating a competing current authority or
                    // backfilling that record with this library's binding.
                    || primary.as_ref().is_some_and(|(original, _)| {
                        original.library != draft.library
                    })
                    || draft
                        .identity
                        .as_ref()
                        .is_some_and(|saved| saved != identity)
                    || (draft.identity.is_none() && (prior != &current_path || primary.is_some()))
                    || !same_draft_path(&draft.path, prior)
                {
                    continue;
                }
                if draft.identity.is_none() {
                    draft.base_known = false;
                }
                if draft.identity.is_some() {
                    imported.push(reference);
                }
                if draft.source != current_source || !draft.alternatives.is_empty() {
                    candidates.push(draft);
                }
            }
        }
        if let Some((draft, bytes)) = &primary {
            imported.push(DraftReference {
                library: draft.library.clone(),
                path: draft.path.replace('/', "\\").to_lowercase(),
                fingerprint: hash(bytes),
            });
            // Even a clean primary must move with its original object and
            // suppress old proposals after a later explicit resolution.
            candidates.push(draft.clone());
        }
        let Some(mut chosen) = candidates.first().cloned() else {
            return Ok((None, false));
        };
        let mut proposals = Vec::<DraftAlternative>::new();
        for draft in &candidates {
            proposals.push(DraftAlternative {
                path: draft.path.clone(),
                base_source: draft.base_source.clone(),
                source: draft.source.clone(),
                base_known: draft.base_known,
                producer_id: draft.producer_id.clone(),
            });
            proposals.extend(draft.alternatives.clone());
        }
        let mut alternatives = Vec::new();
        for proposal in proposals {
            if same_proposal(&proposal, &chosen.proposal())
                || alternatives
                    .iter()
                    .any(|p: &DraftAlternative| same_proposal(p, &proposal))
            {
                continue;
            }
            alternatives.push(proposal);
        }
        if candidates
            .iter()
            .any(|draft| draft.library != chosen.library)
        {
            chosen.library = None;
        }
        chosen.path = current_path;
        chosen.alternatives = alternatives;
        if !imported.is_empty() {
            // Publish the unique object record before its consumed marker. If
            // interrupted, that primary is already authoritative on restart;
            // every legacy record remains untouched as original private input.
            for reference in imported {
                if !consumed.contains(&reference) {
                    consumed.push(reference);
                }
            }
            let migration_path = match &chosen.identity {
                Some(saved) => self.object_draft_path(saved, &chosen.library)?,
                None => self.legacy_draft_path(path),
            }
            .with_extension("relocation.json");
            let migration = encode_draft_record(&DraftMigration {
                library: chosen.library.clone(),
                identity: identity.clone(),
                consumed,
            })?;
            // The old unique primary is an original private record just as a
            // legacy pathname record was. Preserve its exact bytes before
            // rebinding the authoritative locator; this archive is history
            // only and is never searched as a recovery/admission input.
            let _ = encode_draft_record(&chosen)?;
            if let Some((_, bytes)) = &primary {
                journal::private_create(
                    &self
                        .draft_directory("draft-history")
                        .join(format!("{}.json", fresh())),
                    bytes,
                )?;
            }
            self.store_draft(&chosen)?;
            private_replace(&migration_path, &migration)?;
        }
        Ok((Some(chosen), false))
    }
}
fn unproved_draft_placement() -> CoreError {
    CoreError::new(
        "identityPending",
        "The source position has no committed Draft relocation; the original proposal is unchanged and the current input must remain in the editor",
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
fn relative_draft_path(path: &Path, prefix: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    for component in prefix.components() {
        let current = components.next()?;
        if !current
            .as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&component.as_os_str().to_string_lossy())
        {
            return None;
        }
    }
    Some(components.collect())
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
