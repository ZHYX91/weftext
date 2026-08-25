use std::fmt;
use std::path::Path;

use serde::Serialize;

use crate::{
    AdjacentHeadingBody, DocumentError, NodeId, ResolvedNodeIcon, TRASH_NODE_NAME,
    read_node_document, resolve_node_icon_from_source, scan_workspace,
    searchable_document_text_for_profile,
};

/// One rebuildable workspace-search match.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSearchResult {
    pub id: NodeId,
    pub name: String,
    pub path: String,
    pub snippet: String,
    pub name_match: bool,
    pub icon: Option<ResolvedNodeIcon>,
}

/// Failure while reading the rebuildable workspace-search view.
#[derive(Debug)]
pub enum WorkspaceSearchError {
    InvalidWorkspace,
    Document(DocumentError),
}

impl fmt::Display for WorkspaceSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => {
                formatter.write_str("workspace is incomplete while searching")
            }
            Self::Document(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WorkspaceSearchError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidWorkspace => None,
            Self::Document(error) => Some(error),
        }
    }
}

impl From<DocumentError> for WorkspaceSearchError {
    fn from(error: DocumentError) -> Self {
        Self::Document(error)
    }
}

/// Searches node names, ordinary user frontmatter, and visible Markdown body.
///
/// System metadata and Trash are excluded from this default rebuildable view.
///
/// # Errors
///
/// Returns an error when the workspace is incomplete or a node document cannot
/// be read safely.
pub fn search_workspace(
    root: &Path,
    query: &str,
) -> Result<Vec<WorkspaceSearchResult>, WorkspaceSearchError> {
    search_workspace_selected(root, query, None)
}

/// Searches only documents present in a pre-authorized logical projection.
/// Inventory reads bounded metadata for topology, then the scope check occurs
/// before any document body is opened. Returned paths use the projected
/// locator and therefore cannot contain hidden physical ancestors.
///
/// # Errors
///
/// Returns an error when the workspace or supplied projection is inconsistent,
/// or an authorized node document cannot be read safely.
pub fn search_workspace_scoped(
    root: &Path,
    query: &str,
    scope: &crate::WorkspaceReadScope,
) -> Result<Vec<WorkspaceSearchResult>, WorkspaceSearchError> {
    search_workspace_selected(root, query, Some(scope))
}

fn search_workspace_selected(
    root: &Path,
    query: &str,
    scope: Option<&crate::WorkspaceReadScope>,
) -> Result<Vec<WorkspaceSearchResult>, WorkspaceSearchError> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let inventory = scan_workspace(root);
    if scope.map_or_else(
        || !inventory.is_valid(),
        |scope| scope.validate_inventory(&inventory).is_err(),
    ) {
        return Err(WorkspaceSearchError::InvalidWorkspace);
    }
    let trash_path = root.join(TRASH_NODE_NAME);
    let mut results = Vec::new();
    for node in &inventory.nodes {
        if node.path == trash_path || node.path.starts_with(&trash_path) {
            continue;
        }
        let Some(id) = node.id else { continue };
        if scope.is_some_and(|scope| !scope.allows(id)) {
            continue;
        }
        let snapshot = read_node_document(&node.path)?;
        let searchable = searchable_document_text_for_profile(
            snapshot.profile,
            &snapshot.source,
            AdjacentHeadingBody::Separate,
        );
        let name_match = node.name.to_lowercase().contains(&needle);
        let matching_line = searchable
            .lines()
            .find(|line| line.to_lowercase().contains(&needle));
        if !name_match && matching_line.is_none() {
            continue;
        }
        let path = scope
            .and_then(|scope| scope.locator(id).map(ToOwned::to_owned))
            .unwrap_or_else(|| {
                node.path
                    .strip_prefix(root)
                    .unwrap_or(&node.path)
                    .to_string_lossy()
                    .replace('\\', "/")
            });
        results.push(WorkspaceSearchResult {
            id,
            name: node.name.clone(),
            path,
            snippet: matching_line
                .map_or_else(|| "节点名称匹配".to_owned(), |line| line.trim().to_owned()),
            name_match,
            icon: resolve_node_icon_from_source(&snapshot.source),
        });
    }
    results.sort_by(|left, right| {
        right
            .name_match
            .cmp(&left.name_match)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DocumentEdit, commit_document_edit, create_workspace, plan_create_child_node,
        plan_document_edit, read_node_document,
    };
    use tempfile::tempdir;

    #[test]
    fn searches_document_header_properties_and_body_but_not_system_metadata() {
        let temp = tempdir().expect("tempdir");
        let workspace = temp.path().join("Workspace");
        let created = create_workspace(&workspace).expect("workspace");
        let child =
            plan_create_child_node(&workspace, created.id, "NamedMatch").expect("create child");
        crate::commit_workspace_transaction(&child).expect("commit child");

        let snapshot = read_node_document(&workspace).expect("read");
        let source = snapshot.source.clone()
            + "= Workspace\n:keywords: user-search-tag\n\nVisible body phrase\n";
        let plan = plan_document_edit(
            &workspace,
            &snapshot.revision,
            [DocumentEdit {
                start: 0,
                end: u64::try_from(snapshot.source.len()).expect("length"),
                replacement: source,
            }],
        )
        .expect("edit plan");
        commit_document_edit(&plan).expect("edit commit");

        assert_eq!(
            search_workspace(&workspace, "user-search-tag")
                .expect("tag search")
                .len(),
            1
        );
        assert_eq!(
            search_workspace(&workspace, "Visible body")
                .expect("body search")
                .len(),
            1
        );
        assert!(
            search_workspace(&workspace, "weftext")
                .expect("system search")
                .is_empty()
        );
        let named = search_workspace(&workspace, "NamedMatch").expect("name search");
        assert_eq!(named[0].name, "NamedMatch");
        assert!(named[0].name_match);
    }
}
