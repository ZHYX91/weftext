//! Draft paths follow committed same-object relocations, independently of UI
//! session rebinding. Migration retains every original private draft record.
use super::*;
use operations::Mutation;
use std::collections::BTreeSet;

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
        match File::open(self.draft_path(&session.path)) {
            Ok(mut file) => match hosted::read_actor_library_header(&mut file)? {
                hosted::ActorLibraryHeader::LegacyUnbound => return Ok(None),
                hosted::ActorLibraryHeader::Bound(original)
                    if Some(&original) != binding.as_ref() =>
                {
                    return Ok(None);
                }
                _ => {}
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
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
    fn draft_migration_path(&self, path: &Path) -> PathBuf {
        self.draft_path(path).with_extension("relocation.json")
    }
    pub(super) fn read_relocated_draft(&self, path: &Path) -> Result<Option<Draft>> {
        let guard = ParentGuard::acquire(path)?;
        let mut current = guard.open_file(false, false)?;
        let identity = stamp(&current)?.identity;
        let current_source = decode(&read_source(&mut current)?)?;
        self.read_relocated_draft_with_identity(path, &identity, &current_source)
    }
    pub(super) fn read_relocated_draft_with_identity(
        &self,
        path: &Path,
        identity: &FileIdentity,
        current_source: &str,
    ) -> Result<Option<Draft>> {
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
                if let Some(t) = &plan.transaction
                    && let Mutation::Relocate {
                        source,
                        target,
                        old_leaf,
                        new_leaf,
                        source_identity,
                        leaf_identity,
                        ..
                    } = &t.mutation
                {
                    let mut predecessors = Vec::new();
                    for prior in &lineage {
                        let Some(relative) =
                            relative_draft_path(Path::new(prior), Path::new(target))
                        else {
                            continue;
                        };
                        let Some(actual_root) = path.ancestors().nth(relative.components().count())
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
            if lineage.len() == prior_len {
                break;
            }
        }
        let mut consumed = Vec::<DraftReference>::new();
        for prior in &lineage {
            match fs::read(self.draft_migration_path(Path::new(prior))) {
                Ok(bytes) => {
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
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let current_path = display(path);
        let mut candidates = Vec::<Draft>::new();
        let mut imported = Vec::<DraftReference>::new();
        let mut displaced_record = None;
        // Prefer an actual current-path proposal when both locations have drafts,
        // and keep the other exact proposals visible as alternatives.
        let ordered =
            std::iter::once(&current_path).chain(lineage.iter().filter(|p| *p != &current_path));
        for prior in ordered {
            let bytes = match fs::read(self.draft_path(Path::new(prior))) {
                Ok(bytes) => bytes,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // Exact old path-hash records remain readable while new
                    // records use normalized Windows spellings.
                    let legacy = self
                        .draft_directory("drafts")
                        .join(format!("{}.json", hash(prior.as_bytes())));
                    match fs::read(legacy) {
                        Ok(bytes) => bytes,
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(e) => return Err(e.into()),
                    }
                }
                Err(e) => return Err(e.into()),
            };
            let parsed: Draft = serde_json::from_slice(&bytes)?;
            let reference = DraftReference {
                library: parsed.library.clone(),
                path: prior.replace('/', "\\").to_lowercase(),
                fingerprint: hash(&bytes),
            };
            if prior != &current_path && consumed.contains(&reference) {
                continue;
            }
            let draft = parsed;
            self.check_actor_library(&draft.library)?;
            if (draft
                .identity
                .as_ref()
                .is_some_and(|saved| saved != identity)
                || (draft.identity.is_none() && prior != &current_path))
                || !same_draft_path(&draft.path, prior)
            {
                // Preserve unknown or replaced-object records without attaching
                // them to a newly occupied pathname.
                if prior == &current_path {
                    displaced_record = Some(bytes);
                }
                continue;
            }
            let mut draft = draft;
            if draft.identity.is_none() {
                draft.base_known = false;
            }
            if prior != &current_path {
                imported.push(reference);
            }
            if draft.source != current_source || !draft.alternatives.is_empty() {
                candidates.push(draft);
            }
        }
        let Some(mut chosen) = candidates.first().cloned() else {
            return Ok(None);
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
            // Publish the complete merged record first. A crash before its
            // migration marker only repeats a deduplicating merge on restart.
            if let Some(bytes) = displaced_record {
                journal::private_create(
                    &self
                        .draft_directory("draft-history")
                        .join(format!("{}.json", fresh())),
                    &bytes,
                )?;
            }
            private_replace(&self.draft_path(path), &serde_json::to_vec(&chosen)?)?;
            for reference in imported {
                if !consumed.contains(&reference) {
                    consumed.push(reference);
                }
            }
            private_replace(
                &self.draft_migration_path(path),
                &serde_json::to_vec(&DraftMigration {
                    library: chosen.library.clone(),
                    identity: identity.clone(),
                    consumed,
                })?,
            )?;
        }
        Ok(Some(chosen))
    }
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
