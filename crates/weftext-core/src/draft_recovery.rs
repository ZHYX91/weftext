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
#[serde(rename_all = "camelCase")]
pub struct DraftAlternative {
    pub path: String,
    pub base_source: String,
    pub source: String,
    pub base_known: bool,
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

    pub(super) fn retained_draft_alternatives(
        draft: Option<&Draft>,
        incoming: &str,
    ) -> Vec<DraftAlternative> {
        let Some(draft) = draft else {
            return Vec::new();
        };
        let primary = DraftAlternative {
            path: draft.path.clone(),
            base_source: draft.base_source.clone(),
            source: draft.source.clone(),
            base_known: draft.base_known,
        };
        let mut alternatives = Vec::<DraftAlternative>::new();
        for proposal in std::iter::once(primary).chain(draft.alternatives.iter().cloned()) {
            if (proposal.source == incoming
                && proposal.base_source == draft.base_source
                && proposal.base_known == draft.base_known)
                || alternatives.iter().any(|v| {
                    v.source == proposal.source
                        && v.base_source == proposal.base_source
                        && v.base_known == proposal.base_known
                })
            {
                continue;
            }
            alternatives.push(proposal);
        }
        alternatives
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
        let bytes = encode_draft_record(draft)?;
        let path = match &draft.identity {
            Some(identity) => self.object_draft_path(identity, &draft.library)?,
            None => self.legacy_draft_path(Path::new(&draft.path)),
        };
        // Inspect the original metadata before touching an existing payload.
        // A foreign scoped record must never become a writable local proposal.
        if path.exists()
            && self
                .read_draft_record(&path, &draft.library, false)?
                .is_none()
        {
            return Err(CoreError::new(
                "privateEvidence",
                "Draft library scope differs",
            ));
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
            });
            proposals.extend(draft.alternatives.clone());
        }
        let mut alternatives = Vec::new();
        for proposal in proposals {
            if proposal.source == current_source
                || (proposal.source == chosen.source && proposal.base_source == chosen.base_source)
                || alternatives.iter().any(|p: &DraftAlternative| {
                    p.source == proposal.source && p.base_source == proposal.base_source
                })
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
