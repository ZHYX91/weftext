use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fs;

use serde::Serialize;

use crate::{
    InventoryIssueCode, NodeId, NodeRecord, WorkspaceContentEntry, WorkspaceContentKind,
    WorkspaceInventory, WorkspaceItemIcon, WorkspaceItemIconFallback, derive_workspace_item_icon,
    resolve_node_icon_from_source,
};

pub const NAVIGATION_PROJECTION_VERSION: u16 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceNavigationProjection {
    pub version: u16,
    pub root_node_id: NodeId,
    pub hierarchy: Vec<NavigationNode>,
    pub contents: Vec<NavigationContentItem>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationNode {
    pub node_id: NodeId,
    pub name: String,
    pub parent_node_id: Option<NodeId>,
    pub locator: String,
    pub depth: usize,
    pub child_count: usize,
    pub display_icon: WorkspaceItemIcon,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationContentItem {
    pub kind: WorkspaceContentKind,
    pub name: String,
    pub locator: String,
    pub parent_locator: Option<String>,
    pub node_id: Option<NodeId>,
    pub owner_node_id: Option<NodeId>,
    pub display_icon: WorkspaceItemIcon,
}

#[derive(Debug)]
pub enum NavigationProjectionError {
    InvalidInventory(InventoryIssueCode),
    DocumentUnreadable(std::io::Error),
}

impl std::fmt::Display for NavigationProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInventory(code) => {
                write!(
                    formatter,
                    "workspace inventory cannot produce navigation: {code:?}"
                )
            }
            Self::DocumentUnreadable(error) => {
                write!(formatter, "navigation icon source is unreadable: {error}")
            }
        }
    }
}

impl std::error::Error for NavigationProjectionError {}

/// Builds the versioned, rebuildable navigation projection consumed by every
/// Weftext shell. The projection is derived only from a current Core inventory;
/// it performs no directory discovery of its own.
///
/// # Errors
///
/// Returns [`NavigationProjectionError`] when the supplied inventory is invalid
/// or a managed node's canonical document cannot be read for icon resolution.
pub fn build_workspace_navigation(
    inventory: &WorkspaceInventory,
) -> Result<WorkspaceNavigationProjection, NavigationProjectionError> {
    if !inventory.is_valid() && !has_only_trash_storage_issues(inventory) {
        return Err(NavigationProjectionError::InvalidInventory(
            inventory
                .issues
                .first()
                .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
        ));
    }
    let root = inventory
        .nodes
        .iter()
        .find(|node| node.parent_id.is_none())
        .and_then(|node| node.id)
        .ok_or(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::MissingIdentity,
        ))?;
    let visible_nodes = inventory
        .nodes
        .iter()
        .filter(|node| !is_trash_storage_node(&inventory.root, node))
        .collect::<Vec<_>>();
    let node_by_id = visible_nodes
        .iter()
        .copied()
        .filter_map(|node| node.id.map(|id| (id, node)))
        .collect::<HashMap<_, _>>();
    if node_by_id.len() != visible_nodes.len() {
        return Err(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::MissingIdentity,
        ));
    }
    let mut hierarchy = Vec::with_capacity(visible_nodes.len());
    let mut visited = HashSet::with_capacity(visible_nodes.len());
    append_hierarchy(
        inventory,
        root,
        0,
        &node_by_id,
        &mut visited,
        &mut hierarchy,
    )?;
    if visited.len() != visible_nodes.len() {
        return Err(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::InvalidMetadata,
        ));
    }

    let hierarchy_position = hierarchy
        .iter()
        .enumerate()
        .map(|(position, node)| (node.node_id, position))
        .collect::<HashMap<_, _>>();
    let mut entries = inventory
        .content
        .iter()
        .filter(|entry| !is_trash_storage_content(entry))
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        content_group(left)
            .cmp(&content_group(right))
            .then_with(|| match (left.node_id, right.node_id) {
                (Some(left), Some(right)) => hierarchy_position
                    .get(&left)
                    .cmp(&hierarchy_position.get(&right)),
                _ => natural_name_cmp(
                    left.parent_relative_path.as_deref().unwrap_or(""),
                    right.parent_relative_path.as_deref().unwrap_or(""),
                )
                .then_with(|| natural_name_cmp(&left.name, &right.name))
                .then_with(|| left.relative_path.cmp(&right.relative_path)),
            })
    });
    let contents = entries
        .into_iter()
        .map(|entry| navigation_content(entry, &node_by_id, &inventory.root))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(WorkspaceNavigationProjection {
        version: NAVIGATION_PROJECTION_VERSION,
        root_node_id: root,
        hierarchy,
        contents,
    })
}

fn append_hierarchy(
    inventory: &WorkspaceInventory,
    id: NodeId,
    depth: usize,
    node_by_id: &HashMap<NodeId, &NodeRecord>,
    visited: &mut HashSet<NodeId>,
    output: &mut Vec<NavigationNode>,
) -> Result<(), NavigationProjectionError> {
    if !visited.insert(id) {
        return Err(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::InvalidMetadata,
        ));
    }
    let node = node_by_id
        .get(&id)
        .ok_or(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::MissingIdentity,
        ))?;
    let children = inventory
        .ordered_children(id)
        .into_iter()
        .filter(|child| !is_trash_storage_node(&inventory.root, child))
        .collect::<Vec<_>>();
    let source = fs::read_to_string(&node.document_path)
        .map_err(NavigationProjectionError::DocumentUnreadable)?;
    let fallback = if depth == 0 {
        WorkspaceItemIconFallback::WorkspaceRoot
    } else {
        WorkspaceItemIconFallback::ManagedNode
    };
    output.push(NavigationNode {
        node_id: id,
        name: node.name.clone(),
        parent_node_id: node.parent_id,
        locator: portable_locator(&inventory.root, &node.path)?,
        depth,
        child_count: children.len(),
        display_icon: derive_workspace_item_icon(resolve_node_icon_from_source(&source), fallback),
    });
    for child in children {
        let child_id = child.id.ok_or(NavigationProjectionError::InvalidInventory(
            InventoryIssueCode::MissingIdentity,
        ))?;
        append_hierarchy(inventory, child_id, depth + 1, node_by_id, visited, output)?;
    }
    Ok(())
}

fn is_trash_storage_node(root: &std::path::Path, node: &NodeRecord) -> bool {
    crate::workspace_trash::is_trash_storage_path(root, &node.path)
}

fn is_trash_storage_content(entry: &WorkspaceContentEntry) -> bool {
    entry.kind == WorkspaceContentKind::ManagedNode && entry.relative_path == crate::TRASH_NODE_NAME
}

fn has_only_trash_storage_issues(inventory: &WorkspaceInventory) -> bool {
    !inventory.nodes.is_empty()
        && !inventory.issues.is_empty()
        && inventory.issues.iter().all(|issue| {
            crate::workspace_trash::is_trash_storage_path(&inventory.root, &issue.path)
        })
}

fn navigation_content(
    entry: &WorkspaceContentEntry,
    node_by_id: &HashMap<NodeId, &NodeRecord>,
    root: &std::path::Path,
) -> Result<NavigationContentItem, NavigationProjectionError> {
    let fallback = match entry.kind {
        WorkspaceContentKind::UnmanagedDirectory => WorkspaceItemIconFallback::UnmanagedFolder,
        WorkspaceContentKind::UnmanagedMarkdown => WorkspaceItemIconFallback::UnmanagedMarkdown,
        WorkspaceContentKind::Resource => WorkspaceItemIconFallback::OrdinaryFile,
        WorkspaceContentKind::ManagedNode => {
            let node = entry.node_id.and_then(|id| node_by_id.get(&id).copied());
            if node.is_some_and(|node| node.path == root) {
                WorkspaceItemIconFallback::WorkspaceRoot
            } else if node.is_some_and(|node| {
                node.parent_id.is_some()
                    && node.name.eq_ignore_ascii_case(crate::TRASH_NODE_NAME)
                    && node.path.parent() == Some(root)
            }) {
                WorkspaceItemIconFallback::Trash
            } else {
                WorkspaceItemIconFallback::ManagedNode
            }
        }
    };
    let explicit = entry
        .node_id
        .and_then(|id| node_by_id.get(&id).copied())
        .map(|node| fs::read_to_string(&node.document_path))
        .transpose()
        .map_err(NavigationProjectionError::DocumentUnreadable)?
        .and_then(|source| resolve_node_icon_from_source(&source));
    Ok(NavigationContentItem {
        kind: entry.kind,
        name: entry.name.clone(),
        locator: entry.relative_path.clone(),
        parent_locator: entry.parent_relative_path.clone(),
        node_id: entry.node_id,
        owner_node_id: entry.owner_node_id,
        display_icon: derive_workspace_item_icon(explicit, fallback),
    })
}

fn portable_locator(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<String, NavigationProjectionError> {
    path.strip_prefix(root)
        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            NavigationProjectionError::InvalidInventory(InventoryIssueCode::InvalidMetadata)
        })
}

const fn content_group(entry: &WorkspaceContentEntry) -> u8 {
    match entry.kind {
        WorkspaceContentKind::ManagedNode => 0,
        WorkspaceContentKind::UnmanagedDirectory => 1,
        WorkspaceContentKind::UnmanagedMarkdown => 2,
        WorkspaceContentKind::Resource => 3,
    }
}

pub(crate) fn natural_name_cmp(left: &str, right: &str) -> Ordering {
    natural_cmp(left, right)
}

fn natural_cmp(left: &str, right: &str) -> Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let mut left_chars = left.chars().peekable();
    let mut right_chars = right.chars().peekable();
    loop {
        match (left_chars.peek().copied(), right_chars.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_char), Some(right_char))
                if left_char.is_ascii_digit() && right_char.is_ascii_digit() =>
            {
                let left_digits = take_digits(&mut left_chars);
                let right_digits = take_digits(&mut right_chars);
                let left_trimmed = left_digits.trim_start_matches('0');
                let right_trimmed = right_digits.trim_start_matches('0');
                let left_number = if left_trimmed.is_empty() {
                    "0"
                } else {
                    left_trimmed
                };
                let right_number = if right_trimmed.is_empty() {
                    "0"
                } else {
                    right_trimmed
                };
                let ordering = left_number
                    .len()
                    .cmp(&right_number.len())
                    .then_with(|| left_number.cmp(right_number))
                    .then_with(|| left_digits.len().cmp(&right_digits.len()));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(left_char), Some(right_char)) => {
                left_chars.next();
                right_chars.next();
                let ordering = left_char.cmp(&right_char);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

fn take_digits<I>(chars: &mut std::iter::Peekable<I>) -> String
where
    I: Iterator<Item = char>,
{
    let mut digits = String::new();
    while chars.peek().is_some_and(char::is_ascii_digit) {
        if let Some(value) = chars.next() {
            digits.push(value);
        }
    }
    digits
}

#[cfg(test)]
mod tests {
    use super::natural_cmp;

    #[test]
    fn natural_names_compare_numeric_runs() {
        assert!(natural_cmp("Node 2", "Node 10").is_lt());
        assert!(natural_cmp("node 02", "node 2").is_gt());
        assert!(natural_cmp("Alpha", "beta").is_lt());
    }
}
