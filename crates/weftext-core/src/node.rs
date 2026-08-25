use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::content_boundary::{
    BoundaryAction, CONTENT_RULES_FILE_NAME, ContentRules, linked_or_reparse, portable_path,
    reject_linked_existing_ancestors,
};
use crate::{
    FrontmatterDiagnostic, NodeId, NodeMetadata, WorkspaceDocumentGeneration,
    canonical_document_path, is_unmanaged_markdown_path, parse_node_metadata_with_diagnostics,
    workspace_document_format,
};

const MAX_METADATA_PREFIX_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRecord {
    pub id: Option<NodeId>,
    pub name: String,
    pub path: PathBuf,
    pub document_path: PathBuf,
    pub parent_id: Option<NodeId>,
    pub metadata: Option<NodeMetadata>,
    pub metadata_diagnostics: Vec<FrontmatterDiagnostic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceContentKind {
    ManagedNode,
    UnmanagedDirectory,
    UnmanagedMarkdown,
    Resource,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceContentEntry {
    pub kind: WorkspaceContentKind,
    pub name: String,
    pub relative_path: String,
    pub parent_relative_path: Option<String>,
    pub node_id: Option<NodeId>,
    pub owner_node_id: Option<NodeId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryIssueCode {
    RootMissing,
    RootNotDirectory,
    NonUtf8Name,
    SymlinkUnsupported,
    MissingNodeDocument,
    DocumentUnreadable,
    MissingIdentity,
    InvalidMetadata,
    DuplicateIdentity,
    WorkspaceSettingOutsideRoot,
    InvalidContentRules,
    CanonicalDocumentBoundary,
    InvalidWorkspaceGeneration,
    TrashReconciliationRequired,
    LegacyTrashMigrationRequired,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryIssue {
    pub code: InventoryIssueCode,
    pub path: PathBuf,
    pub message: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorkspaceInventory {
    pub root: PathBuf,
    pub generation: WorkspaceDocumentGeneration,
    pub nodes: Vec<NodeRecord>,
    pub content: Vec<WorkspaceContentEntry>,
    pub trash_items: Vec<crate::WorkspaceTrashItem>,
    pub legacy_trash_format: bool,
    pub issues: Vec<InventoryIssue>,
    pub(crate) boundaries: Vec<ContentBoundaryRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ContentBoundaryRecord {
    pub(crate) relative_path: String,
    pub(crate) ignored: bool,
}

impl WorkspaceInventory {
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty() && !self.nodes.is_empty()
    }

    #[must_use]
    pub fn ordered_children(&self, parent: NodeId) -> Vec<&NodeRecord> {
        let sort = self
            .nodes
            .iter()
            .find(|node| node.id == Some(parent))
            .and_then(|node| node.metadata)
            .map_or(crate::ChildSort::default(), |metadata| metadata.child_sort);
        let mut children = self
            .nodes
            .iter()
            .filter(|node| node.parent_id == Some(parent))
            .collect::<Vec<_>>();
        match sort.mode {
            crate::SortMode::Name => {
                children.sort_by(|left, right| {
                    crate::navigation::natural_name_cmp(&left.name, &right.name)
                        .then_with(|| left.path.cmp(&right.path))
                });
                if sort.direction == crate::SortDirection::Descending {
                    children.reverse();
                }
            }
            crate::SortMode::Manual => children.sort_by(|left, right| {
                let left_rank = left
                    .metadata
                    .and_then(|metadata| metadata.sibling_order.rank);
                let right_rank = right
                    .metadata
                    .and_then(|metadata| metadata.sibling_order.rank);
                left_rank
                    .is_none()
                    .cmp(&right_rank.is_none())
                    .then_with(|| left_rank.cmp(&right_rank))
                    .then_with(|| crate::navigation::natural_name_cmp(&left.name, &right.name))
                    .then_with(|| left.path.cmp(&right.path))
            }),
        }
        children
    }
}

pub fn scan_workspace(root: impl AsRef<Path>) -> WorkspaceInventory {
    let selected_root = root.as_ref().to_path_buf();
    let mut inventory = WorkspaceInventory {
        root: selected_root.clone(),
        generation: WorkspaceDocumentGeneration::Unsupported,
        ..WorkspaceInventory::default()
    };
    if !validate_workspace_root(&selected_root, &mut inventory) {
        return inventory;
    }
    let root = match fs::canonicalize(&selected_root) {
        Ok(root) => root,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                &selected_root,
                &format!("cannot resolve workspace root: {error}"),
            ));
            return inventory;
        }
    };
    if let Err(error) = reject_linked_existing_ancestors(&root) {
        inventory.issues.push(issue(
            InventoryIssueCode::SymlinkUnsupported,
            &selected_root,
            &error.to_string(),
        ));
        return inventory;
    }
    inventory.root.clone_from(&root);

    for ancestor in root.parent().into_iter().flat_map(Path::ancestors) {
        let marker = ancestor.join(crate::WORKSPACE_FORMAT_MARKER_FILE);
        match fs::symlink_metadata(&marker) {
            Ok(_) => {
                inventory.issues.push(issue(
                    InventoryIssueCode::InvalidWorkspaceGeneration,
                    &marker,
                    "selected workspace root is nested beneath another format marker",
                ));
                return rebase_inventory_paths(inventory, &root, &selected_root);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                inventory.issues.push(issue(
                    InventoryIssueCode::InvalidWorkspaceGeneration,
                    &marker,
                    &format!("cannot inspect ancestor workspace marker: {error}"),
                ));
                return rebase_inventory_paths(inventory, &root, &selected_root);
            }
        }
    }

    if workspace_document_format(&root).generation != WorkspaceDocumentGeneration::AsciiDocV1 {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidWorkspaceGeneration,
            &root.join(crate::WORKSPACE_FORMAT_MARKER_FILE),
            "workspace format marker is unreadable or has an unsupported value",
        ));
        return rebase_inventory_paths(inventory, &root, &selected_root);
    }
    inventory.generation = WorkspaceDocumentGeneration::AsciiDocV1;

    let rules = match ContentRules::load(&root) {
        Ok(rules) => rules,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::InvalidContentRules,
                &root.join(CONTENT_RULES_FILE_NAME),
                &error.to_string(),
            ));
            return rebase_inventory_paths(inventory, &root, &selected_root);
        }
    };
    if rules.classify(CONTENT_RULES_FILE_NAME, false).is_some() {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidContentRules,
            &root.join(CONTENT_RULES_FILE_NAME),
            "content rules cannot classify their own authority file",
        ));
        return rebase_inventory_paths(inventory, &root, &selected_root);
    }

    let mut seen = HashMap::<NodeId, PathBuf>::new();
    scan_directory(&root, &root, None, &rules, &mut inventory, &mut seen);
    scan_workspace_trash_authority(&root, &rules, &mut inventory, &mut seen);
    inventory
        .nodes
        .sort_by(|left, right| left.path.cmp(&right.path));
    inventory
        .content
        .sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    rebase_inventory_paths(inventory, &root, &selected_root)
}

fn rebase_inventory_paths(
    mut inventory: WorkspaceInventory,
    canonical_root: &Path,
    selected_root: &Path,
) -> WorkspaceInventory {
    for node in &mut inventory.nodes {
        node.path = rebase_inventory_path(&node.path, canonical_root, selected_root);
        node.document_path =
            rebase_inventory_path(&node.document_path, canonical_root, selected_root);
    }
    for item in &mut inventory.trash_items {
        item.item_path = rebase_inventory_path(&item.item_path, canonical_root, selected_root);
        item.payload_path =
            rebase_inventory_path(&item.payload_path, canonical_root, selected_root);
    }
    for issue in &mut inventory.issues {
        issue.path = rebase_inventory_path(&issue.path, canonical_root, selected_root);
    }
    inventory.root = selected_root.to_path_buf();
    inventory
}

fn rebase_inventory_path(path: &Path, canonical_root: &Path, selected_root: &Path) -> PathBuf {
    if let Ok(relative) = path.strip_prefix(canonical_root) {
        return selected_root.join(relative);
    }
    for (canonical_ancestor, selected_ancestor) in canonical_root
        .ancestors()
        .skip(1)
        .zip(selected_root.ancestors().skip(1))
    {
        let Ok(relative) = path.strip_prefix(canonical_ancestor) else {
            continue;
        };
        let candidate = selected_ancestor.join(relative);
        if let (Ok(canonical_candidate), Ok(canonical_path)) =
            (fs::canonicalize(&candidate), fs::canonicalize(path))
            && canonical_candidate == canonical_path
        {
            return candidate;
        }
    }
    path.to_path_buf()
}

fn validate_workspace_root(root: &Path, inventory: &mut WorkspaceInventory) -> bool {
    let root_metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            inventory.issues.push(issue(
                InventoryIssueCode::RootMissing,
                root,
                "workspace root does not exist",
            ));
            return false;
        }
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                root,
                &format!("cannot inspect workspace root: {error}"),
            ));
            return false;
        }
    };
    if linked_or_reparse(&root_metadata) {
        inventory.issues.push(issue(
            InventoryIssueCode::SymlinkUnsupported,
            root,
            "workspace root cannot be a link or reparse point",
        ));
        return false;
    }
    if !root_metadata.is_dir() {
        inventory.issues.push(issue(
            InventoryIssueCode::RootNotDirectory,
            root,
            "workspace root is not a directory",
        ));
        return false;
    }
    true
}

fn scan_workspace_trash_authority(
    root: &Path,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
    seen: &mut HashMap<NodeId, PathBuf>,
) {
    let trash = root.join(crate::workspace_trash::TRASH_DIRECTORY_NAME);
    let metadata = match fs::symlink_metadata(&trash) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                &trash,
                &format!("cannot inspect Workspace Trash: {error}"),
            ));
            return;
        }
    };
    if linked_or_reparse(&metadata) || !metadata.is_dir() {
        inventory.issues.push(issue(
            InventoryIssueCode::TrashReconciliationRequired,
            &trash,
            "Workspace Trash must be a regular non-link directory",
        ));
        return;
    }
    let root_parent_id = inventory
        .nodes
        .iter()
        .find(|node| node.path == root)
        .and_then(|node| node.id);
    let expected = canonical_document_path(&trash, crate::workspace_trash::TRASH_DIRECTORY_NAME);
    let (node_id, metadata, metadata_diagnostics) = inspect_node_document(&expected, inventory);
    if let Some(id) = node_id
        && let Some(first) = seen.insert(id, trash.clone())
    {
        inventory.issues.push(issue(
            InventoryIssueCode::DuplicateIdentity,
            &trash,
            &format!("node ID is already used by {}", first.display()),
        ));
    }
    inventory.nodes.push(NodeRecord {
        id: node_id,
        name: crate::workspace_trash::TRASH_DIRECTORY_NAME.to_owned(),
        path: trash.clone(),
        document_path: expected,
        parent_id: root_parent_id,
        metadata,
        metadata_diagnostics,
    });
    inventory.content.push(WorkspaceContentEntry {
        kind: WorkspaceContentKind::ManagedNode,
        name: crate::workspace_trash::TRASH_DIRECTORY_NAME.to_owned(),
        relative_path: crate::workspace_trash::TRASH_DIRECTORY_NAME.to_owned(),
        parent_relative_path: Some(String::new()),
        node_id,
        owner_node_id: None,
    });

    let active_ids = seen
        .iter()
        .map(|(id, path)| (*id, path.clone()))
        .collect::<BTreeMap<_, _>>();
    let inspection =
        crate::workspace_trash::inspect_workspace_trash_store(root, rules, &active_ids);
    inventory.legacy_trash_format = inspection.legacy_format;
    inventory.trash_items = inspection.items;
    for trash_issue in inspection.issues {
        let code = if trash_issue.duplicate_identity {
            InventoryIssueCode::DuplicateIdentity
        } else if inspection.legacy_format {
            InventoryIssueCode::LegacyTrashMigrationRequired
        } else {
            InventoryIssueCode::TrashReconciliationRequired
        };
        inventory
            .issues
            .push(issue(code, &trash_issue.path, &trash_issue.message));
    }
}

fn scan_directory(
    root: &Path,
    directory: &Path,
    parent_id: Option<NodeId>,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
    seen: &mut HashMap<NodeId, PathBuf>,
) {
    let Some(name) = directory.file_name().and_then(|value| value.to_str()) else {
        inventory.issues.push(issue(
            InventoryIssueCode::NonUtf8Name,
            directory,
            "node directory name is not UTF-8",
        ));
        return;
    };
    let expected = canonical_document_path(directory, name);
    let (node_id, metadata, metadata_diagnostics) = inspect_node_document(&expected, inventory);

    if let Ok(relative_document) = portable_path(root, &expected)
        && rules.classify(&relative_document, false).is_some()
    {
        inventory.issues.push(issue(
            InventoryIssueCode::CanonicalDocumentBoundary,
            &expected,
            "a managed node's canonical document cannot be classified separately",
        ));
    }

    if parent_id.is_some()
        && metadata.is_some_and(|value| value.presentation.adjacent_heading_body_explicit)
    {
        inventory.issues.push(issue(
            InventoryIssueCode::WorkspaceSettingOutsideRoot,
            &expected,
            "workspace presentation settings are valid only on the root node",
        ));
    }
    if parent_id.is_none() && metadata.is_some_and(|value| value.sibling_order.rank.is_some()) {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidMetadata,
            &expected,
            "weftext.sibling_rank is valid only on a non-root node with a parent",
        ));
    }

    if let Some(id) = node_id
        && let Some(first) = seen.insert(id, directory.to_path_buf())
    {
        inventory.issues.push(issue(
            InventoryIssueCode::DuplicateIdentity,
            directory,
            &format!("node ID is already used by {}", first.display()),
        ));
    }

    inventory.nodes.push(NodeRecord {
        id: node_id,
        name: name.to_owned(),
        path: directory.to_path_buf(),
        document_path: expected.clone(),
        parent_id,
        metadata,
        metadata_diagnostics,
    });

    let relative_path = portable_path(root, directory).unwrap_or_default();
    inventory.content.push(WorkspaceContentEntry {
        kind: WorkspaceContentKind::ManagedNode,
        name: name.to_owned(),
        parent_relative_path: parent_locator(&relative_path),
        relative_path,
        node_id,
        owner_node_id: None,
    });

    let child_directories = collect_entries(root, directory, &expected, node_id, rules, inventory);
    for child in child_directories.into_values() {
        scan_directory(root, &child, node_id, rules, inventory, seen);
    }
}

fn inspect_node_document(
    expected: &Path,
    inventory: &mut WorkspaceInventory,
) -> (
    Option<NodeId>,
    Option<NodeMetadata>,
    Vec<FrontmatterDiagnostic>,
) {
    let metadata = match fs::symlink_metadata(expected) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            inventory.issues.push(issue(
                InventoryIssueCode::MissingNodeDocument,
                expected,
                "content directory has no same-named document for the selected generation",
            ));
            return (None, None, Vec::new());
        }
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                expected,
                &format!("cannot inspect node document: {error}"),
            ));
            return (None, None, Vec::new());
        }
    };
    if linked_or_reparse(&metadata) {
        inventory.issues.push(issue(
            InventoryIssueCode::SymlinkUnsupported,
            expected,
            "node document cannot be a link or reparse point",
        ));
        return (None, None, Vec::new());
    }
    if !metadata.is_file() {
        inventory.issues.push(issue(
            InventoryIssueCode::MissingNodeDocument,
            expected,
            "content directory has no same-named document for the selected generation",
        ));
        return (None, None, Vec::new());
    }
    let source = match read_metadata_prefix(expected) {
        Ok(source) => source,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                expected,
                &format!("cannot read node document: {error}"),
            ));
            return (None, None, Vec::new());
        }
    };
    let parsed = parse_node_metadata_with_diagnostics(&source);
    match parsed {
        Ok((parsed, diagnostics)) => {
            if parsed.id.is_none() {
                inventory.issues.push(issue(
                    InventoryIssueCode::MissingIdentity,
                    expected,
                    "node document has no weftext.id",
                ));
            }
            (parsed.id, Some(parsed), diagnostics)
        }
        Err(crate::FrontmatterError::MissingIdentity) => {
            inventory.issues.push(issue(
                InventoryIssueCode::MissingIdentity,
                expected,
                "node document has no weftext.id",
            ));
            (None, None, Vec::new())
        }
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::InvalidMetadata,
                expected,
                &error.to_string(),
            ));
            (None, None, Vec::new())
        }
    }
}

/// Reads only the bounded YAML envelope needed by inventory. This prevents
/// workspace topology discovery from parsing or retaining unauthorized body
/// content before an access scope has been established.
fn read_metadata_prefix(path: &Path) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let limit = u64::try_from(MAX_METADATA_PREFIX_BYTES)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut reader = BufReader::new(file).take(limit);
    let mut prefix = String::new();
    let mut line = String::new();
    let mut saw_opening = false;
    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }
        if prefix.len().saturating_add(read) > MAX_METADATA_PREFIX_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "document YAML envelope exceeds the 1 MiB inventory limit",
            ));
        }
        let marker = line
            .trim_end_matches(['\r', '\n'])
            .strip_prefix('\u{feff}')
            .unwrap_or_else(|| line.trim_end_matches(['\r', '\n']));
        prefix.push_str(&line);
        if !saw_opening {
            if marker != "---" {
                break;
            }
            saw_opening = true;
        } else if marker == "---" {
            break;
        }
    }
    Ok(prefix)
}

fn collect_entries(
    root: &Path,
    directory: &Path,
    expected: &Path,
    owner_node_id: Option<NodeId>,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
) -> BTreeMap<String, PathBuf> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                directory,
                &format!("cannot enumerate node directory: {error}"),
            ));
            return BTreeMap::new();
        }
    };
    let mut child_directories = BTreeMap::<String, PathBuf>::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some((sort_name, child_path)) =
            classify_entry(root, &path, expected, owner_node_id, rules, inventory)
        {
            child_directories.insert(sort_name, child_path);
        }
    }

    child_directories
}

#[allow(clippy::too_many_lines)]
fn classify_entry(
    root: &Path,
    path: &Path,
    expected: &Path,
    owner_node_id: Option<NodeId>,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
) -> Option<(String, PathBuf)> {
    if path == expected {
        return None;
    }
    let file_name = path.file_name()?;
    let Some(file_name) = file_name.to_str() else {
        inventory.issues.push(issue(
            InventoryIssueCode::NonUtf8Name,
            path,
            "workspace entry name is not UTF-8",
        ));
        return None;
    };
    if path.parent() == Some(root)
        && file_name.eq_ignore_ascii_case(crate::workspace_trash::TRASH_DIRECTORY_NAME)
    {
        if file_name != crate::workspace_trash::TRASH_DIRECTORY_NAME {
            inventory.issues.push(issue(
                InventoryIssueCode::TrashReconciliationRequired,
                path,
                "Workspace Trash path has a non-canonical case-fold collision",
            ));
        }
        return None;
    }
    if file_name == crate::ANNOTATIONS_FILE_NAME {
        inspect_reserved_annotation_sidecar(root, path, rules, inventory);
        return None;
    }
    if skip_reserved_entry(root, path, file_name, inventory) {
        return None;
    }
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                path,
                &format!("cannot inspect workspace entry: {error}"),
            ));
            return None;
        }
    };
    if linked_or_reparse(&metadata) {
        inventory.issues.push(issue(
            InventoryIssueCode::SymlinkUnsupported,
            path,
            "links and reparse points are not followed or classified",
        ));
        return None;
    }
    let relative = match portable_path(root, path) {
        Ok(relative) => relative,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::NonUtf8Name,
                path,
                &error.to_string(),
            ));
            return None;
        }
    };
    let action = rules.classify(&relative, metadata.is_dir());
    match (action, metadata.is_dir(), metadata.is_file()) {
        (Some(BoundaryAction::Ignore), _, _) => {
            inventory.boundaries.push(ContentBoundaryRecord {
                relative_path: relative,
                ignored: true,
            });
        }
        (Some(BoundaryAction::Unmanaged), true, _) => {
            inventory.boundaries.push(ContentBoundaryRecord {
                relative_path: relative.clone(),
                ignored: false,
            });
            inventory.content.push(WorkspaceContentEntry {
                kind: WorkspaceContentKind::UnmanagedDirectory,
                name: file_name.to_owned(),
                parent_relative_path: parent_locator(&relative),
                relative_path: relative,
                node_id: None,
                owner_node_id: None,
            });
            scan_unmanaged_tree(root, path, rules, inventory);
        }
        (Some(BoundaryAction::Unmanaged), _, true) => {
            inventory.boundaries.push(ContentBoundaryRecord {
                relative_path: relative.clone(),
                ignored: false,
            });
            push_visible_file(path, file_name, relative, None, inventory);
        }
        (None, true, _) => return Some((file_name.to_lowercase(), path.to_path_buf())),
        (None, _, true) => {
            push_visible_file(path, file_name, relative, owner_node_id, inventory);
        }
        _ => {}
    }
    None
}

fn inspect_reserved_annotation_sidecar(
    root: &Path,
    path: &Path,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
) {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::DocumentUnreadable,
                path,
                &format!("cannot inspect annotation sidecar: {error}"),
            ));
            return;
        }
    };
    if linked_or_reparse(&metadata) {
        inventory.issues.push(issue(
            InventoryIssueCode::SymlinkUnsupported,
            path,
            "annotation sidecar cannot be a link or reparse point",
        ));
        return;
    }
    if !metadata.is_file() {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidMetadata,
            path,
            "weftext.annotations.json is reserved for one regular node-local sidecar file",
        ));
        return;
    }
    let relative = match portable_path(root, path) {
        Ok(relative) => relative,
        Err(error) => {
            inventory.issues.push(issue(
                InventoryIssueCode::NonUtf8Name,
                path,
                &error.to_string(),
            ));
            return;
        }
    };
    if rules.classify(&relative, false).is_some() {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidContentRules,
            path,
            "the reserved node annotation sidecar cannot be classified as unmanaged or ignored",
        ));
    }
}

fn skip_reserved_entry(
    root: &Path,
    path: &Path,
    file_name: &str,
    inventory: &mut WorkspaceInventory,
) -> bool {
    if file_name == ".git" || file_name.starts_with(".__weftext-transaction-") {
        return true;
    }
    if file_name == crate::WORKSPACE_FORMAT_MARKER_FILE {
        if path != root.join(crate::WORKSPACE_FORMAT_MARKER_FILE) {
            inventory.issues.push(issue(
                InventoryIssueCode::InvalidWorkspaceGeneration,
                path,
                "workspace format marker is valid only at the selected workspace root",
            ));
        }
        return true;
    }
    if file_name != CONTENT_RULES_FILE_NAME {
        return false;
    }
    if path != root.join(CONTENT_RULES_FILE_NAME) {
        inventory.issues.push(issue(
            InventoryIssueCode::InvalidContentRules,
            path,
            "content rules authority is valid only at the selected workspace root",
        ));
    }
    true
}

fn scan_unmanaged_tree(
    root: &Path,
    directory: &Path,
    rules: &ContentRules,
    inventory: &mut WorkspaceInventory,
) {
    let mut entries =
        match fs::read_dir(directory).and_then(std::iter::Iterator::collect::<Result<Vec<_>, _>>) {
            Ok(entries) => entries,
            Err(error) => {
                inventory.issues.push(issue(
                    InventoryIssueCode::DocumentUnreadable,
                    directory,
                    &format!("cannot enumerate unmanaged directory: {error}"),
                ));
                return;
            }
        };
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            inventory.issues.push(issue(
                InventoryIssueCode::NonUtf8Name,
                &path,
                "workspace entry name is not UTF-8",
            ));
            continue;
        };
        if skip_reserved_entry(root, &path, file_name, inventory) {
            continue;
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                inventory.issues.push(issue(
                    InventoryIssueCode::DocumentUnreadable,
                    &path,
                    &format!("cannot inspect unmanaged entry: {error}"),
                ));
                continue;
            }
        };
        if linked_or_reparse(&metadata) {
            inventory.issues.push(issue(
                InventoryIssueCode::SymlinkUnsupported,
                &path,
                "links and reparse points are not followed or classified",
            ));
            continue;
        }
        let relative = match portable_path(root, &path) {
            Ok(relative) => relative,
            Err(error) => {
                inventory.issues.push(issue(
                    InventoryIssueCode::NonUtf8Name,
                    &path,
                    &error.to_string(),
                ));
                continue;
            }
        };
        if rules.classify(&relative, metadata.is_dir()) == Some(BoundaryAction::Ignore) {
            inventory.boundaries.push(ContentBoundaryRecord {
                relative_path: relative,
                ignored: true,
            });
            continue;
        }
        if metadata.is_dir() {
            inventory.content.push(WorkspaceContentEntry {
                kind: WorkspaceContentKind::UnmanagedDirectory,
                name: file_name.to_owned(),
                parent_relative_path: parent_locator(&relative),
                relative_path: relative,
                node_id: None,
                owner_node_id: None,
            });
            scan_unmanaged_tree(root, &path, rules, inventory);
        } else if metadata.is_file() {
            push_visible_file(&path, file_name, relative, None, inventory);
        }
    }
}

fn push_visible_file(
    path: &Path,
    name: &str,
    relative_path: String,
    owner_node_id: Option<NodeId>,
    inventory: &mut WorkspaceInventory,
) {
    let kind = if owner_node_id.is_none() && is_unmanaged_markdown_path(path) {
        WorkspaceContentKind::UnmanagedMarkdown
    } else {
        WorkspaceContentKind::Resource
    };
    inventory.content.push(WorkspaceContentEntry {
        kind,
        name: name.to_owned(),
        parent_relative_path: parent_locator(&relative_path),
        relative_path,
        node_id: None,
        owner_node_id,
    });
}

fn parent_locator(relative_path: &str) -> Option<String> {
    if relative_path.is_empty() {
        None
    } else {
        Some(
            relative_path
                .rsplit_once('/')
                .map_or_else(String::new, |(parent, _)| parent.to_owned()),
        )
    }
}

fn issue(code: InventoryIssueCode, path: &Path, message: &str) -> InventoryIssue {
    InventoryIssue {
        code,
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorkspaceIndex {
    by_id: HashMap<NodeId, PathBuf>,
    by_path: HashMap<PathBuf, NodeId>,
}

impl WorkspaceIndex {
    /// Rebuild the derived identity-to-path lookup from a valid inventory.
    ///
    /// # Errors
    ///
    /// Returns the first inventory issue, or an identity-specific issue if an
    /// invalid inventory reaches index construction.
    pub fn rebuild(inventory: &WorkspaceInventory) -> Result<Self, InventoryIssueCode> {
        if !inventory.is_valid() {
            return Err(inventory
                .issues
                .first()
                .map_or(InventoryIssueCode::RootMissing, |issue| issue.code));
        }
        let mut index = Self::default();
        for node in &inventory.nodes {
            let id = node.id.ok_or(InventoryIssueCode::MissingIdentity)?;
            if index.by_id.insert(id, node.path.clone()).is_some() {
                return Err(InventoryIssueCode::DuplicateIdentity);
            }
            index.by_path.insert(node.path.clone(), id);
        }
        Ok(index)
    }

    #[must_use]
    pub fn path_for(&self, id: NodeId) -> Option<&Path> {
        self.by_id.get(&id).map(PathBuf::as_path)
    }

    #[must_use]
    pub fn id_for(&self, path: &Path) -> Option<NodeId> {
        self.by_path.get(path).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}
