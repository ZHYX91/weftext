//! Extra raw companions may travel with an original tree. Admission is derived
//! from the original committed records, never from a new name or file contents.
use super::*;
use std::ffi::OsString;

pub(super) struct Budget {
    used: usize,
    files: BTreeSet<String>,
    directories: BTreeSet<String>,
}
impl Budget {
    pub(super) fn new(t: &FrozenOperation) -> Result<Self> {
        // Count actual original objects once, including namespace parents that
        // were captured more than once. Create-only outputs have no FI yet.
        let originals = t
            .files
            .iter()
            .map(|file| &file.stamp.identity)
            .chain(t.directories.iter().map(|directory| &directory.identity))
            .map(serde_json::to_string)
            .collect::<std::result::Result<BTreeSet<_>, _>>()?;
        let created = match &t.mutation {
            Mutation::Compound {
                updates,
                created_files,
                created_directories,
                ..
            } => updates
                .iter()
                .chain(created_files)
                .filter(|file| file.before.is_none())
                .count()
                .saturating_add(created_directories.len()),
            _ => 0,
        };
        Ok(Self {
            used: originals.len().saturating_add(created),
            files: BTreeSet::new(),
            directories: BTreeSet::new(),
        })
    }
    pub(super) fn reserve(&mut self, count: usize) -> Result<()> {
        self.used = self
            .used
            .checked_add(count)
            .filter(|used| *used <= MAX_OPERATION_FILES)
            .ok_or_else(|| {
                CoreError::new(
                    "coverageIncomplete",
                    "Compound metadata members exceed 5000",
                )
            })?;
        Ok(())
    }
    fn remaining(&self) -> usize {
        MAX_OPERATION_FILES.saturating_sub(self.used)
    }
    fn observe(&mut self, identity: &FileIdentity, directory: bool) -> Result<()> {
        let identities = if directory {
            &mut self.directories
        } else {
            &mut self.files
        };
        identities.insert(serde_json::to_string(identity)?);
        Ok(())
    }
    /// Finish a complete pass while its original namespace handles remain
    /// held. Only these already-budgeted observations are matched against
    /// history; no collection of all historical identities is constructed.
    pub(super) fn validate(&self, core: &Core, t: &FrozenOperation) -> Result<()> {
        if self.files.is_empty() && self.directories.is_empty() {
            return Ok(());
        }
        let workspace = core.require_workspace(Path::new(&t.request.root))?;
        for plan in core
            .operations
            .values()
            .filter(|plan| core.committed(&plan.operation_id))
        {
            if plan.workspace_id.as_deref() == Some(workspace.workspace_id.as_str()) {
                for birth in core.read_births(&plan.operation_id)? {
                    if birth.workspace_id == workspace.workspace_id
                        && self
                            .files
                            .contains(&serde_json::to_string(&birth.identity)?)
                    {
                        return Err(changed_member());
                    }
                }
            }
            let bytes = match fs::read(core.journal_path(&plan.operation_id, "resource-births")) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error.into()),
            };
            for birth in serde_json::from_slice::<Vec<ResourceBirth>>(&bytes)? {
                if birth.workspace_id == workspace.workspace_id
                    && self
                        .files
                        .contains(&serde_json::to_string(&birth.identity)?)
                {
                    return Err(changed_member());
                }
            }
        }
        if !self.directories.is_empty()
            && core.any_directory_identity_admitted(&workspace.workspace_id, &self.directories)?
        {
            return Err(changed_member());
        }
        Ok(())
    }
}

fn changed_member() -> CoreError {
    CoreError::new(
        "scopeChanged",
        "A newly admitted member changed the original moving tree",
    )
}

/// Enumerate only new names through the retained original root. Every name is
/// reserved before appending to a queue, using one budget for all moving roots
/// of a complete validation. Bodies are neither read nor admitted.
pub(super) fn validate_additions(
    namespace: &mut NamespaceGuard,
    additions: Vec<(PathBuf, OsString, u32)>,
    budget: &mut Budget,
) -> Result<()> {
    if additions.is_empty() {
        return Ok(());
    }
    let mut queue = additions;
    while let Some((parent, name, attributes)) = queue.pop() {
        if attributes & (0x400 | 0x1000 | 0x40000 | 0x100000 | 0x400000) != 0 {
            return Err(CoreError::new(
                "unsupportedPath",
                "An extra companion is reparse, offline or a cloud placeholder",
            ));
        }
        let relative = parent.join(name);
        let directory = attributes & 0x10 != 0;
        let identity = if directory {
            let (identity, entries, complete) =
                namespace.source_directory_snapshot(&relative, budget.remaining())?;
            if !complete {
                return Err(CoreError::new(
                    "coverageIncomplete",
                    "Compound metadata members exceed 5000",
                ));
            }
            budget.reserve(entries.len())?;
            queue.extend(
                entries
                    .into_iter()
                    .map(|(name, attributes)| (relative.clone(), name, attributes)),
            );
            identity
        } else {
            let file = namespace.open_source_placement(&relative)?;
            stamp(&file)?.identity
        };
        budget.observe(&identity, directory)?;
    }
    Ok(())
}
