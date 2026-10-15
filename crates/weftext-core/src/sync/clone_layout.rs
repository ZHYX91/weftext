//! Clone placement is frozen in the original CreateTree and SyncSetup. A Trash
//! slot is a receiver location, never a new portable identity or a second log.
use super::*;

pub(super) fn clone_locations(
    root: &Path,
    operation_id: &str,
    states: &BTreeMap<ObjectKey, Version>,
) -> Result<BTreeMap<ObjectKey, PathBuf>> {
    let mut locations = BTreeMap::<ObjectKey, PathBuf>::new();
    let mut remaining = states.keys().cloned().collect::<BTreeSet<_>>();
    while !remaining.is_empty() {
        let prior = remaining.len();
        for key in remaining.clone() {
            let state = &states[&key].state;
            let target = match &key {
                ObjectKey::Node(id) | ObjectKey::Directory(id) => {
                    // Resolve the logical parent even for a retired object, so
                    // a cycle or a missing member cannot be hidden by Trash.
                    let parent = if let Some(parent) = &state.parent {
                        let Some(path) = locations.get(parent) else {
                            continue;
                        };
                        path.as_path()
                    } else {
                        root
                    };
                    if state.retired {
                        let role = if matches!(key, ObjectKey::Node(_)) {
                            "node"
                        } else {
                            "directory"
                        };
                        root.join(".weftext-trash")
                            .join(format!("{operation_id}-{role}-{id}"))
                    } else {
                        // A child carried by a retired ancestor keeps its
                        // received state and original relative placement.
                        parent.join(&state.name)
                    }
                }
                _ if state.retired => {
                    return Err(invalid(
                        "Only a placed content object can occupy a Trash tree",
                    ));
                }
                _ => root.to_path_buf(),
            };
            locations.insert(key.clone(), target);
            remaining.remove(&key);
        }
        if remaining.len() == prior {
            return Err(invalid(
                "Selected placement graph is cyclic or lacks its parent",
            ));
        }
    }
    Ok(locations)
}

pub(super) fn validate_clone_namespace(transaction: &FrozenOperation) -> Result<()> {
    let Mutation::CreateTree {
        target,
        directories,
        files,
        ..
    } = &transaction.mutation
    else {
        return Err(invalid("Clone must retain its original CreateTree"));
    };
    let mut slots = directories
        .iter()
        .map(|relative| (Path::new(target).join(relative), true))
        .chain(files.iter().map(|file| (PathBuf::from(&file.path), false)))
        .collect::<Vec<_>>();
    let mut failure = None;
    slots.sort_by(|(left, _), (right, _)| {
        crate::backend::namespace_path_compare(left, right).unwrap_or_else(|error| {
            failure = Some(error);
            std::cmp::Ordering::Equal
        })
    });
    if let Some(error) = failure {
        return Err(error);
    }
    for adjacent in slots.windows(2) {
        let (left, directory) = &adjacent[0];
        let (right, _) = &adjacent[1];
        if crate::backend::namespace_path_compare(left, right)? == std::cmp::Ordering::Equal
            || (!directory && crate::backend::namespace_relative_path(right, left)?.is_some())
        {
            return Err(invalid(
                "Clone files and directories overlap in the actual namespace",
            ));
        }
    }
    Ok(())
}
