//! One current Draft per original physical file and qualified input scope.
//! Committed relocations may change its locator; no pathname or unbound fallback
//! can upgrade retained input into a different library's current authority.
use super::*;
use std::collections::BTreeSet;
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
impl Core {
    pub(crate) fn draft_session_scope(&self, session_id: &str) -> Result<QualifiedInputScope> {
        self.sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the source again"))?
            .file_scope
            .clone()
    }
    pub(crate) fn draft_scope_for_session(
        &self,
        session_id: &str,
        retained: Option<&Draft>,
    ) -> Result<QualifiedInputScope> {
        let scope = self.draft_session_scope(session_id)?;
        let session = &self.sessions[session_id];
        if session.draft_placement_pending {
            return Err(unproved_draft_placement());
        }
        if retained.is_some_and(|draft| draft.scope != scope) {
            return Err(CoreError::new(
                "privateEvidence",
                "Retained Draft has another original scope",
            ));
        }
        let path = self.object_draft_path(&session.stamp.identity, &scope)?;
        if let Some((original, _)) =
            self.read_draft_record(&path, &scope, &session.stamp.identity)?
        {
            if original.identity != session.stamp.identity {
                return Err(CoreError::new(
                    "privateEvidence",
                    "Draft physical object differs",
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
            for plan in self
                .operations
                .values()
                .filter(|plan| self.committed(&plan.operation_id))
            {
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
                        let guard = ParentGuard::acquire(&actual_root.join(".draft-prospective"))?;
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
        let identity = &draft.identity;
        let bytes = encode_draft_record(draft)?;
        let path = self.object_draft_path(identity, &draft.scope)?;
        // Inspect metadata before replacing the one current record.
        let _ = self.read_draft_record(&path, &draft.scope, &draft.identity)?;
        private_replace(&path, &bytes)
    }
    pub(super) fn read_draft_record(
        &self,
        path: &Path,
        scope: &QualifiedInputScope,
        identity: &FileIdentity,
    ) -> Result<Option<(Draft, Vec<u8>)>> {
        let guard = match ParentGuard::acquire(path) {
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
        let actual = ParentGuard::acquire(path)
            .and_then(|guard| guard.open_file(false, false))
            .and_then(|file| stamp(&file));
        if let (Ok(actual), Ok(scope)) = (actual, self.capture_file_save_scope(path)) {
            return self.object_draft_path(&actual.identity, &scope).unwrap();
        }
        self.legacy_draft_path(path)
    }
    pub(super) fn read_relocated_draft(&self, path: &Path) -> Result<Option<Draft>> {
        let guard = ParentGuard::acquire(path)?;
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
        if draft.scope != scope || &draft.identity != identity {
            return Err(CoreError::new(
                "privateEvidence",
                "Draft original participant differs",
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
        "The source position has no committed Draft relocation; the original proposal is unchanged and current input must remain in the editor",
    )
}
fn same_draft_path(a: &str, b: &str) -> bool {
    a.replace('/', "\\")
        .eq_ignore_ascii_case(&b.replace('/', "\\"))
}
