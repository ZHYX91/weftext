use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::{InventoryIssueCode, NodeId, WorkspaceDocumentGeneration, WorkspaceInventory};

const MAX_SCOPED_NODES: usize = 100_000;

/// One already-authorized logical node placement supplied by a caller that
/// owns ACL policy. The locator contains only visible ancestors and uses `/`
/// separators without a leading slash; the canonical workspace root uses an
/// empty locator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceNodeProjection {
    pub node_id: NodeId,
    pub parent_node_id: Option<NodeId>,
    pub locator: String,
}

impl WorkspaceNodeProjection {
    #[must_use]
    pub fn new(
        node_id: NodeId,
        parent_node_id: Option<NodeId>,
        locator: impl Into<String>,
    ) -> Self {
        Self {
            node_id,
            parent_node_id,
            locator: locator.into(),
        }
    }
}

/// Closed logical projection that authorizes which managed documents a
/// derived Core index may open. Construction validates the projected tree so
/// hidden physical ancestors cannot reappear through path/depth fields.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceReadScope {
    nodes: BTreeMap<NodeId, ScopedNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScopedNode {
    parent_node_id: Option<NodeId>,
    locator: String,
    depth: u16,
}

impl WorkspaceReadScope {
    /// Builds a bounded, cycle-free projection with unique logical locators.
    ///
    /// # Errors
    ///
    /// Returns an error for duplicates, invalid portable locators, missing
    /// projected parents, non-direct parent/locator relationships, cycles, or
    /// excessive depth/count.
    pub fn new(
        projections: impl IntoIterator<Item = WorkspaceNodeProjection>,
    ) -> Result<Self, WorkspaceScopeError> {
        let projections = projections.into_iter().collect::<Vec<_>>();
        if projections.len() > MAX_SCOPED_NODES {
            return Err(WorkspaceScopeError::TooManyNodes);
        }
        let mut raw = BTreeMap::<NodeId, (Option<NodeId>, String)>::new();
        let mut locators = BTreeSet::new();
        for projection in projections {
            validate_locator(&projection.locator)?;
            if projection.parent_node_id == Some(projection.node_id) {
                return Err(WorkspaceScopeError::Cycle(projection.node_id));
            }
            if !locators.insert(projection.locator.clone()) {
                return Err(WorkspaceScopeError::DuplicateLocator(projection.locator));
            }
            if raw
                .insert(
                    projection.node_id,
                    (projection.parent_node_id, projection.locator),
                )
                .is_some()
            {
                return Err(WorkspaceScopeError::DuplicateNode(projection.node_id));
            }
        }
        for (node_id, (parent, locator)) in &raw {
            match parent {
                Some(parent_id) => {
                    let Some((_, parent_locator)) = raw.get(parent_id) else {
                        return Err(WorkspaceScopeError::MissingParent {
                            node_id: *node_id,
                            parent_id: *parent_id,
                        });
                    };
                    let logical_parent = locator.rsplit_once('/').map_or("", |(parent, _)| parent);
                    if logical_parent != parent_locator {
                        return Err(WorkspaceScopeError::ParentLocatorMismatch(*node_id));
                    }
                }
                None if locator.contains('/') => {
                    return Err(WorkspaceScopeError::ParentLocatorMismatch(*node_id));
                }
                None => {}
            }
        }

        let mut nodes = BTreeMap::new();
        for (node_id, (parent, locator)) in &raw {
            let mut cursor = *node_id;
            let mut seen = BTreeSet::new();
            let mut depth = 0_u16;
            while let Some(parent_id) = raw.get(&cursor).and_then(|(parent, _)| *parent) {
                if !seen.insert(cursor) {
                    return Err(WorkspaceScopeError::Cycle(*node_id));
                }
                depth = depth
                    .checked_add(1)
                    .ok_or(WorkspaceScopeError::DepthExceeded(*node_id))?;
                cursor = parent_id;
            }
            nodes.insert(
                *node_id,
                ScopedNode {
                    parent_node_id: *parent,
                    locator: locator.clone(),
                    depth,
                },
            );
        }
        Ok(Self { nodes })
    }

    #[must_use]
    pub fn allows(&self, node_id: NodeId) -> bool {
        self.nodes.contains_key(&node_id)
    }

    #[must_use]
    pub fn parent_node_id(&self, node_id: NodeId) -> Option<NodeId> {
        self.nodes
            .get(&node_id)
            .and_then(|node| node.parent_node_id)
    }

    #[must_use]
    pub fn locator(&self, node_id: NodeId) -> Option<&str> {
        self.nodes.get(&node_id).map(|node| node.locator.as_str())
    }

    #[must_use]
    pub fn depth(&self, node_id: NodeId) -> Option<u16> {
        self.nodes.get(&node_id).map(|node| node.depth)
    }

    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.nodes.keys().copied()
    }

    /// Validates only global workspace authority and the managed nodes selected by this scope.
    /// Metadata failures and duplicate identities wholly outside the projection do not influence a
    /// scoped read, while marker/content-rule failures and any ambiguity involving a selected ID
    /// remain fail-closed.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid global authority, a missing/ambiguous selected node, or an
    /// inventory issue attached to a selected node document.
    pub fn validate_inventory(
        &self,
        inventory: &WorkspaceInventory,
    ) -> Result<(), WorkspaceScopeInventoryError> {
        if inventory.generation != WorkspaceDocumentGeneration::AsciiDocV1 {
            return Err(WorkspaceScopeInventoryError::GlobalAuthority(
                InventoryIssueCode::InvalidWorkspaceGeneration,
            ));
        }
        if let Some(issue) = inventory
            .issues
            .iter()
            .find(|issue| global_authority_issue(inventory, issue))
        {
            return Err(WorkspaceScopeInventoryError::GlobalAuthority(issue.code));
        }

        for node_id in self.node_ids() {
            let matching = inventory
                .nodes
                .iter()
                .filter(|node| node.id == Some(node_id))
                .collect::<Vec<_>>();
            let [node] = matching.as_slice() else {
                return Err(if matching.is_empty() {
                    WorkspaceScopeInventoryError::MissingNode(node_id)
                } else {
                    WorkspaceScopeInventoryError::AmbiguousNode(node_id)
                });
            };
            if node.metadata.is_none() {
                return Err(WorkspaceScopeInventoryError::InvalidNode {
                    node_id,
                    code: InventoryIssueCode::InvalidMetadata,
                });
            }
            if let Some(issue) = inventory
                .issues
                .iter()
                .find(|issue| issue.path == node.path || issue.path == node.document_path)
            {
                return Err(WorkspaceScopeInventoryError::InvalidNode {
                    node_id,
                    code: issue.code,
                });
            }
        }
        Ok(())
    }
}

fn global_authority_issue(inventory: &WorkspaceInventory, issue: &crate::InventoryIssue) -> bool {
    match issue.code {
        InventoryIssueCode::RootMissing
        | InventoryIssueCode::RootNotDirectory
        | InventoryIssueCode::InvalidWorkspaceGeneration
        | InventoryIssueCode::InvalidContentRules
        | InventoryIssueCode::TrashReconciliationRequired
        | InventoryIssueCode::LegacyTrashMigrationRequired => true,
        InventoryIssueCode::SymlinkUnsupported => issue.path == inventory.root,
        InventoryIssueCode::NonUtf8Name
        | InventoryIssueCode::MissingNodeDocument
        | InventoryIssueCode::DocumentUnreadable
        | InventoryIssueCode::MissingIdentity
        | InventoryIssueCode::InvalidMetadata
        | InventoryIssueCode::DuplicateIdentity
        | InventoryIssueCode::WorkspaceSettingOutsideRoot
        | InventoryIssueCode::CanonicalDocumentBoundary => false,
    }
}

fn validate_locator(locator: &str) -> Result<(), WorkspaceScopeError> {
    if locator.contains('\\')
        || locator.starts_with('/')
        || locator.ends_with('/')
        || locator.chars().any(char::is_control)
        || locator
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        if locator.is_empty() {
            return Ok(());
        }
        return Err(WorkspaceScopeError::InvalidLocator(locator.to_owned()));
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkspaceScopeError {
    TooManyNodes,
    DuplicateNode(NodeId),
    DuplicateLocator(String),
    InvalidLocator(String),
    MissingParent { node_id: NodeId, parent_id: NodeId },
    ParentLocatorMismatch(NodeId),
    Cycle(NodeId),
    DepthExceeded(NodeId),
}

impl fmt::Display for WorkspaceScopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyNodes => formatter.write_str("workspace read scope exceeds 100,000 nodes"),
            Self::DuplicateNode(node_id) => write!(formatter, "duplicate scoped node {node_id}"),
            Self::DuplicateLocator(locator) => {
                write!(formatter, "duplicate scoped locator `{locator}`")
            }
            Self::InvalidLocator(locator) => {
                write!(formatter, "invalid scoped locator `{locator}`")
            }
            Self::MissingParent { node_id, parent_id } => {
                write!(
                    formatter,
                    "scoped node {node_id} has missing parent {parent_id}"
                )
            }
            Self::ParentLocatorMismatch(node_id) => {
                write!(
                    formatter,
                    "scoped node {node_id} is not a direct child of its projected parent"
                )
            }
            Self::Cycle(node_id) => {
                write!(formatter, "scoped node {node_id} participates in a cycle")
            }
            Self::DepthExceeded(node_id) => {
                write!(formatter, "scoped node {node_id} exceeds supported depth")
            }
        }
    }
}

impl std::error::Error for WorkspaceScopeError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkspaceScopeInventoryError {
    GlobalAuthority(InventoryIssueCode),
    MissingNode(NodeId),
    AmbiguousNode(NodeId),
    InvalidNode {
        node_id: NodeId,
        code: InventoryIssueCode,
    },
}

impl fmt::Display for WorkspaceScopeInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GlobalAuthority(code) => {
                write!(formatter, "invalid global workspace authority: {code:?}")
            }
            Self::MissingNode(node_id) => {
                write!(formatter, "scoped node {node_id} is unavailable")
            }
            Self::AmbiguousNode(node_id) => {
                write!(formatter, "scoped node {node_id} is ambiguous")
            }
            Self::InvalidNode { node_id, code } => {
                write!(formatter, "scoped node {node_id} is invalid: {code:?}")
            }
        }
    }
}

impl std::error::Error for WorkspaceScopeInventoryError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_promoted_roots_without_hidden_path_segments() {
        let root = NodeId::new_v4();
        let promoted = NodeId::new_v4();
        let child = NodeId::new_v4();
        let scope = WorkspaceReadScope::new([
            WorkspaceNodeProjection::new(root, None, ""),
            WorkspaceNodeProjection::new(promoted, None, "Visible"),
            WorkspaceNodeProjection::new(child, Some(promoted), "Visible/Child"),
        ])
        .expect("valid projection");
        assert_eq!(scope.locator(promoted), Some("Visible"));
        assert_eq!(scope.depth(promoted), Some(0));
        assert_eq!(scope.parent_node_id(child), Some(promoted));
        assert_eq!(scope.depth(child), Some(1));
    }

    #[test]
    fn rejects_hidden_segments_missing_parents_and_cycles() {
        let first = NodeId::new_v4();
        let second = NodeId::new_v4();
        assert!(matches!(
            WorkspaceReadScope::new([WorkspaceNodeProjection::new(
                first,
                None,
                "Hidden/Visible"
            )]),
            Err(WorkspaceScopeError::ParentLocatorMismatch(id)) if id == first
        ));
        assert!(matches!(
            WorkspaceReadScope::new([WorkspaceNodeProjection::new(first, Some(second), "Visible")]),
            Err(WorkspaceScopeError::MissingParent { .. })
        ));
        assert!(matches!(
            WorkspaceReadScope::new([
                WorkspaceNodeProjection::new(first, Some(second), "B/A"),
                WorkspaceNodeProjection::new(second, Some(first), "A/B"),
            ]),
            Err(WorkspaceScopeError::ParentLocatorMismatch(_) | WorkspaceScopeError::Cycle(_))
        ));
    }
}
