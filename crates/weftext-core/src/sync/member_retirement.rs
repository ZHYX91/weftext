//! Metadata obligations for an original structure Plan's removed resources.
//!
//! This produces no physical authority. The caller must supply the exact
//! installed before state and explicitly selected after state, and retain the
//! original Plan's owner admission, file/parent identities, namespace effects,
//! Pending guards and recovery proof. No retention path is selected here.
use super::{
    MAX_FILES, MAX_MANIFEST_BYTES, ObjectKey, ObjectState, ReplicaGraph, Result, component, digest,
    invalid, key, relative, uuid,
};
use crate::CoreError;
use std::{cmp::Ordering, collections::BTreeSet, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetiredResourceMember {
    pub relative_path: String,
    pub resource_id: String,
    pub digest: String,
    pub file_revision: String,
}

pub(crate) fn qualify_member_retirements(
    graph: &ReplicaGraph,
    object: &ObjectKey,
    installed: &ObjectState,
    chosen: &ObjectState,
) -> Result<Vec<RetiredResourceMember>> {
    key(object)?;
    let before_names = validate_input(graph, installed)?;
    let after_names = validate_input(graph, chosen)?;
    let before_resources = super::apply::validate_state(graph, object, installed)?;
    let after_resources = super::apply::validate_state(graph, object, chosen)?;
    let failure = || {
        CoreError::new(
            "syncStructureRequired",
            "Only exact original registered Resource leaves can join member retirement; other missing roles need their original structure proof",
        )
    };

    // This is only the existing canonical-name continuation. Source identity,
    // placement and its actual rename remain obligations of the original Plan.
    let old_source = format!("{}.typ", installed.name);
    let canonical_renamed = matches!(object, ObjectKey::Node(_)) && installed.name != chosen.name;
    let mut removed = Vec::new();
    for path in before_names {
        if chosen.files.contains_key(path) {
            if let Some(id) = before_resources.get(path)
                && after_resources.get(path) != Some(id)
            {
                return Err(failure());
            }
            continue;
        }
        if canonical_renamed && path == old_source {
            continue;
        }
        if !matches!(object, ObjectKey::Node(_)) {
            return Err(failure());
        }
        let resource_id = before_resources.get(path).ok_or_else(failure)?;
        // An old Ref cannot silently rebind to another path, and a namespace
        // alias cannot impersonate disappearance of the original file.
        if after_resources.values().any(|id| id == resource_id)
            || contains_name(&after_names, path)?
        {
            return Err(failure());
        }
        removed.push(RetiredResourceMember {
            relative_path: path.into(),
            resource_id: resource_id.clone(),
            digest: installed.files[path].clone(),
            file_revision: installed.file_revisions[path].clone(),
        });
    }
    Ok(removed)
}

fn validate_input<'a>(graph: &ReplicaGraph, state: &'a ObjectState) -> Result<Vec<&'a str>> {
    component(&state.name)?;
    if state.files.is_empty()
        || state.files.len() > MAX_FILES
        || !state.files.keys().eq(state.file_revisions.keys())
        || state.file_revisions.values().collect::<BTreeSet<_>>().len()
            != state.file_revisions.len()
        || serde_json::to_vec(state)?.len() > MAX_MANIFEST_BYTES
    {
        return Err(invalid(
            "Member retirement requires bounded exact file revisions",
        ));
    }
    if let Some(parent) = &state.parent {
        key(parent)?;
        if !matches!(parent, ObjectKey::Node(_) | ObjectKey::Directory(_)) {
            return Err(invalid("Member retirement has an invalid placement parent"));
        }
    }
    let mut names = Vec::<&str>::new();
    for (name, blob) in &state.files {
        relative(name)?;
        digest(blob)?;
        uuid(&state.file_revisions[name])?;
        if graph.blob(blob).is_none() {
            return Err(invalid("Member retirement lacks an exact historical blob"));
        }
        let (index, found) = find_name(&names, name)?;
        if found {
            return Err(invalid("Member retirement has aliased file names"));
        }
        names.insert(index, name);
    }
    Ok(names)
}

fn contains_name(names: &[&str], name: &str) -> Result<bool> {
    Ok(find_name(names, name)?.1)
}

// A fallible binary search retains the backend's actual namespace comparator,
// rather than inventing a Unicode case-folding filesystem contract.
fn find_name(names: &[&str], name: &str) -> Result<(usize, bool)> {
    let (mut start, mut end) = (0, names.len());
    while start < end {
        let middle = start + (end - start) / 2;
        match crate::backend::namespace_path_compare(Path::new(names[middle]), Path::new(name))? {
            Ordering::Less => start = middle + 1,
            Ordering::Greater => end = middle,
            Ordering::Equal => return Ok((middle, true)),
        }
    }
    Ok((start, false))
}

#[cfg(all(test, windows))]
#[path = "member_retirement_tests.rs"]
mod tests;
