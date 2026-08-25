use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use tempfile::Builder;

use crate::{
    AdjacentHeadingBody, NodeId, ResolvedNodeIcon, TRASH_NODE_NAME, WorkspaceSearchResult,
    read_node_document, resolve_node_icon_from_source, scan_workspace,
    searchable_document_text_for_profile,
};

const INDEX_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIndexStats {
    pub entries: usize,
    pub reparsed_documents: usize,
    pub reused_documents: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredIndex {
    version: u32,
    workspace_id: NodeId,
    entries: Vec<StoredEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct StoredEntry {
    id: NodeId,
    name: String,
    path: String,
    document_length: u64,
    modified_nanos: u64,
    searchable: String,
    icon: Option<ResolvedNodeIcon>,
}

#[derive(Debug)]
pub enum SearchIndexError {
    InvalidWorkspace,
    IndexInsideWorkspace,
    Io(std::io::Error),
    Json(serde_json::Error),
    Document(crate::DocumentError),
}

impl fmt::Display for SearchIndexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => formatter.write_str("workspace is incomplete while indexing"),
            Self::IndexInsideWorkspace => {
                formatter.write_str("derived search index must live outside the workspace")
            }
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => error.fmt(formatter),
            Self::Document(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SearchIndexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Document(error) => Some(error),
            Self::InvalidWorkspace | Self::IndexInsideWorkspace => None,
        }
    }
}

impl From<std::io::Error> for SearchIndexError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SearchIndexError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<crate::DocumentError> for SearchIndexError {
    fn from(error: crate::DocumentError) -> Self {
        Self::Document(error)
    }
}

/// Rebuilds the complete derived search index from portable workspace authority.
///
/// # Errors
///
/// Refuses an invalid workspace or an index path inside the workspace.
pub fn rebuild_workspace_search_index(
    root: &Path,
    index_path: &Path,
) -> Result<SearchIndexStats, SearchIndexError> {
    refresh_index(root, index_path, None, &HashSet::new())
}

/// Refreshes only new or externally changed documents while reconciling path changes by UUID.
///
/// # Errors
///
/// Refuses an invalid workspace or an index path inside the workspace.
pub fn refresh_workspace_search_index(
    root: &Path,
    index_path: &Path,
) -> Result<SearchIndexStats, SearchIndexError> {
    refresh_workspace_search_index_invalidating(root, index_path, [])
}

/// Refreshes the derived index and forces known committed node IDs to be reparsed.
///
/// Callers use this after a Core commit so correctness does not depend only on
/// filesystem timestamp granularity. Unrelated entries remain reusable.
///
/// # Errors
///
/// Refuses an invalid workspace or an index path inside the workspace.
pub fn refresh_workspace_search_index_invalidating(
    root: &Path,
    index_path: &Path,
    invalidated_node_ids: impl IntoIterator<Item = NodeId>,
) -> Result<SearchIndexStats, SearchIndexError> {
    let previous = load_index(index_path).ok();
    let invalidated_node_ids = invalidated_node_ids.into_iter().collect();
    refresh_index(root, index_path, previous, &invalidated_node_ids)
}

/// Queries an existing derived index without accessing workspace files.
///
/// # Errors
///
/// Returns an error when the index is absent, corrupt, or unsupported.
pub fn search_workspace_index(
    index_path: &Path,
    query: &str,
) -> Result<Vec<WorkspaceSearchResult>, SearchIndexError> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let index = load_index(index_path)?;
    let mut results = index
        .entries
        .iter()
        .filter_map(|entry| {
            let name_match = entry.name.to_lowercase().contains(&needle);
            let matching_line = entry
                .searchable
                .lines()
                .find(|line| line.to_lowercase().contains(&needle));
            (name_match || matching_line.is_some()).then(|| WorkspaceSearchResult {
                id: entry.id,
                name: entry.name.clone(),
                path: entry.path.clone(),
                snippet: matching_line
                    .map_or_else(|| "节点名称匹配".to_owned(), |line| line.trim().to_owned()),
                name_match,
                icon: entry.icon.clone(),
            })
        })
        .collect::<Vec<_>>();
    results.sort_by(|left, right| {
        right
            .name_match
            .cmp(&left.name_match)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(results)
}

fn refresh_index(
    root: &Path,
    index_path: &Path,
    previous: Option<StoredIndex>,
    invalidated_node_ids: &HashSet<NodeId>,
) -> Result<SearchIndexStats, SearchIndexError> {
    require_external_index_path(root, index_path)?;
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(SearchIndexError::InvalidWorkspace);
    }
    let workspace_id = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .ok_or(SearchIndexError::InvalidWorkspace)?;
    let previous = previous
        .filter(|index| index.version == INDEX_VERSION && index.workspace_id == workspace_id);
    let mut old = previous
        .map(|index| {
            index
                .entries
                .into_iter()
                .map(|entry| (entry.id, entry))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let trash = root.join(TRASH_NODE_NAME);
    let mut entries = Vec::new();
    let mut reparsed_documents = 0;
    let mut reused_documents = 0;
    for node in inventory
        .nodes
        .iter()
        .filter(|node| node.path != trash && !node.path.starts_with(&trash))
    {
        let id = node.id.ok_or(SearchIndexError::InvalidWorkspace)?;
        let (document_length, modified_nanos) = document_fingerprint(&node.document_path)?;
        let relative = node.path.strip_prefix(root).unwrap_or(&node.path);
        let path = relative.to_string_lossy().replace('\\', "/");
        let reusable = (!invalidated_node_ids.contains(&id))
            .then(|| old.remove(&id))
            .flatten()
            .filter(|entry| {
                entry.document_length == document_length && entry.modified_nanos == modified_nanos
            });
        if let Some(mut entry) = reusable {
            entry.name.clone_from(&node.name);
            entry.path = path;
            entries.push(entry);
            reused_documents += 1;
        } else {
            let snapshot = read_node_document(&node.path)?;
            entries.push(StoredEntry {
                id,
                name: node.name.clone(),
                path,
                document_length,
                modified_nanos,
                searchable: searchable_document_text_for_profile(
                    snapshot.profile,
                    &snapshot.source,
                    AdjacentHeadingBody::Separate,
                ),
                icon: resolve_node_icon_from_source(&snapshot.source),
            });
            reparsed_documents += 1;
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    persist_index(
        index_path,
        &StoredIndex {
            version: INDEX_VERSION,
            workspace_id,
            entries,
        },
    )?;
    let index = load_index(index_path)?;
    Ok(SearchIndexStats {
        entries: index.entries.len(),
        reparsed_documents,
        reused_documents,
    })
}

fn require_external_index_path(root: &Path, index_path: &Path) -> Result<(), SearchIndexError> {
    let canonical_root = fs::canonicalize(root)?;
    let absolute_index = if index_path.is_absolute() {
        index_path.to_path_buf()
    } else {
        std::env::current_dir()?.join(index_path)
    };
    let resolved_index = resolve_existing_ancestor(&absolute_index)?;
    let root_key = portable_comparison_key(&canonical_root);
    let index_key = portable_comparison_key(&resolved_index);
    let separator = std::path::MAIN_SEPARATOR;
    if index_key == root_key || index_key.starts_with(&format!("{root_key}{separator}")) {
        return Err(SearchIndexError::IndexInsideWorkspace);
    }
    Ok(())
}

/// Resolve an output path through every existing filesystem ancestor before
/// making a security decision. This catches `..`, symlinks, and Windows
/// junctions even when the final index file or some child directories do not
/// exist yet.
fn resolve_existing_ancestor(path: &Path) -> Result<PathBuf, SearchIndexError> {
    let mut cursor = path;
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(cursor) {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let name = cursor
                    .file_name()
                    .ok_or_else(|| std::io::Error::new(error.kind(), error.to_string()))?;
                missing.push(name.to_os_string());
                cursor = cursor
                    .parent()
                    .ok_or_else(|| std::io::Error::new(error.kind(), error.to_string()))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn portable_comparison_key(path: &Path) -> String {
    let value = path.to_string_lossy();
    let value = value.strip_prefix(r"\\?\").unwrap_or(&value);
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value.to_owned()
    }
}

fn document_fingerprint(path: &Path) -> Result<(u64, u64), SearchIndexError> {
    let metadata = fs::metadata(path)?;
    let modified = metadata
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let nanos = u64::try_from(modified.as_nanos()).unwrap_or(u64::MAX);
    Ok((metadata.len(), nanos))
}

fn load_index(path: &Path) -> Result<StoredIndex, SearchIndexError> {
    let bytes = fs::read(path)?;
    let index: StoredIndex = serde_json::from_slice(&bytes)?;
    if index.version != INDEX_VERSION {
        return Err(SearchIndexError::InvalidWorkspace);
    }
    Ok(index)
}

fn persist_index(path: &Path, index: &StoredIndex) -> Result<(), SearchIndexError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec(index)?;
    let mut staged = Builder::new()
        .prefix(".weftext-search-")
        .tempfile_in(parent)?;
    staged.write_all(&bytes)?;
    staged.flush()?;
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}
