//! Pending effects fence the original read and mutation scopes, including slots
//! that have not materialized. The scopes are derived from the retained Plan.
use super::*;

#[derive(Clone, Copy)]
enum ScopeKind {
    File { creates_entry: bool },
    Tree,
    Entries,
}
type Scope = (PathBuf, ScopeKind);

fn write_scopes(t: &FrozenOperation) -> Vec<Scope> {
    let mut scopes = Vec::new();
    let mut updates = |files: &[FileChange]| {
        scopes.extend(
            files
                .iter()
                .filter(|file| file.before.as_ref() != Some(&file.after))
                .map(|file| {
                    (
                        PathBuf::from(&file.path),
                        ScopeKind::File {
                            creates_entry: file.before.is_none(),
                        },
                    )
                }),
        );
    };
    match &t.mutation {
        Mutation::Update { files } => updates(files),
        Mutation::CreateTree {
            target,
            existing_target,
            directories,
            files,
            updates: changes,
            ..
        } => {
            updates(files);
            updates(changes);
            if !existing_target {
                scopes.push((PathBuf::from(target), ScopeKind::Tree));
            }
            scopes.extend(
                directories
                    .iter()
                    .map(|directory| (Path::new(target).join(directory), ScopeKind::Tree)),
            );
        }
        Mutation::Relocate {
            source,
            target,
            updates: changes,
            ..
        } => {
            updates(changes);
            scopes.extend([
                (PathBuf::from(source), ScopeKind::Tree),
                (PathBuf::from(target), ScopeKind::Tree),
            ]);
        }
        Mutation::Compound {
            relocations,
            updates: changes,
            created_files,
            created_directories,
            ..
        } => {
            updates(changes);
            updates(created_files);
            scopes.extend(relocations.iter().flat_map(|step| {
                [
                    (PathBuf::from(&step.source), ScopeKind::Tree),
                    (PathBuf::from(&step.target), ScopeKind::Tree),
                ]
            }));
            scopes.extend(
                created_directories
                    .iter()
                    .map(|directory| (PathBuf::from(directory), ScopeKind::Tree)),
            );
        }
    }
    scopes
}

fn read_scopes(t: &FrozenOperation) -> Vec<Scope> {
    let mut scopes = t
        .files
        .iter()
        .map(|file| {
            (
                PathBuf::from(&file.path),
                ScopeKind::File {
                    creates_entry: false,
                },
            )
        })
        .collect::<Vec<_>>();
    scopes.extend(
        t.directories
            .iter()
            .map(|directory| (PathBuf::from(&directory.path), ScopeKind::Entries)),
    );
    scopes.extend(t.absent_dependencies.iter().map(|absent| {
        (
            PathBuf::from(&absent.path),
            ScopeKind::File {
                creates_entry: false,
            },
        )
    }));
    scopes
}

fn intersects(left: &Scope, right: &Scope) -> Result<bool> {
    use ScopeKind::*;
    let same = |a: &Path, b: &Path| {
        crate::backend::namespace_path_compare(a, b).map(|order| order == std::cmp::Ordering::Equal)
    };
    let direct_child = |child: &Path, parent: &Path| {
        child
            .parent()
            .map_or(Ok(false), |actual| same(actual, parent))
    };
    match (left.1, right.1) {
        (Tree, Entries) => Ok(crate::backend::namespace_relative_path(&right.0, &left.0)?
            .is_some()
            || direct_child(&left.0, &right.0)?),
        (Entries, Tree) => Ok(crate::backend::namespace_relative_path(&left.0, &right.0)?
            .is_some()
            || direct_child(&right.0, &left.0)?),
        (File { creates_entry }, Entries) => Ok(creates_entry && direct_child(&left.0, &right.0)?),
        (Entries, File { creates_entry }) => Ok(creates_entry && direct_child(&right.0, &left.0)?),
        (Entries, Entries) => Ok(false),
        (Tree, Tree) => Ok(
            crate::backend::namespace_relative_path(&right.0, &left.0)?.is_some()
                || crate::backend::namespace_relative_path(&left.0, &right.0)?.is_some(),
        ),
        (Tree, File { .. }) => {
            Ok(crate::backend::namespace_relative_path(&right.0, &left.0)?.is_some())
        }
        (File { .. }, Tree) => {
            Ok(crate::backend::namespace_relative_path(&left.0, &right.0)?.is_some())
        }
        (File { .. }, File { .. }) => same(&left.0, &right.0),
    }
}

impl Core {
    pub(crate) fn validate_operation_pending(&self, plan: &journal::Plan) -> Result<()> {
        let Some(t) = &plan.transaction else {
            return Ok(());
        };
        let writes = write_scopes(t);
        // The original backup API explicitly permits a read of retained source
        // bytes. Its create-only destination remains an ordinary guarded write.
        let reads = if matches!(t.request.intent, OperationIntent::BackupLibrary { .. }) {
            Vec::new()
        } else {
            read_scopes(t)
        };
        for other in self.operations.values().filter(|other| {
            other.operation_id != plan.operation_id
                && !self.committed(&other.operation_id)
                && !self.resolved(&other.operation_id)
                && (other.transaction.is_none()
                    || self.journal_path(&other.operation_id, "install").exists())
        }) {
            let (other_writes, other_reads) = if let Some(transaction) = &other.transaction {
                (write_scopes(transaction), read_scopes(transaction))
            } else {
                (
                    vec![(
                        PathBuf::from(&other.path),
                        if other.kind == "createLibrary" || !other.files.is_empty() {
                            ScopeKind::Tree
                        } else {
                            ScopeKind::File {
                                creates_entry: other.before.is_none(),
                            }
                        },
                    )],
                    Vec::new(),
                )
            };
            for write in &writes {
                for scope in other_writes.iter().chain(&other_reads) {
                    if intersects(write, scope)? {
                        return Err(pending(&other.operation_id));
                    }
                }
            }
            for read in &reads {
                for write in &other_writes {
                    if intersects(read, write)? {
                        return Err(pending(&other.operation_id));
                    }
                }
            }
        }
        Ok(())
    }
}

fn pending(operation_id: &str) -> CoreError {
    CoreError::new(
        "recoveryPending",
        format!(
            "Original unfinished operation {operation_id} overlaps this Plan's frozen read or mutation scope; inspect its retained recovery before any effect"
        ),
    )
}
