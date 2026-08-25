use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use tempfile::Builder;
use uuid::Uuid;

use crate::content_boundary::validate_managed_file_path;
use crate::{
    ANNOTATIONS_FILE_NAME, NodeId, WorkspaceRevision, read_workspace_revision, scan_workspace,
};

const MAX_RESOURCE_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug)]
pub struct ResourceImportPlan {
    pub plan_id: String,
    pub node_id: NodeId,
    pub name: String,
    pub byte_length: usize,
    pub base_revision: WorkspaceRevision,
    workspace_root: PathBuf,
    target_path: PathBuf,
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedResource {
    pub node_id: NodeId,
    pub name: String,
    pub byte_length: usize,
    pub workspace_revision: WorkspaceRevision,
}

#[derive(Debug)]
pub enum ResourceImportError {
    InvalidWorkspace,
    NodeUnavailable,
    InvalidName,
    ReservedName,
    Empty,
    TooLarge,
    AlreadyExists,
    StaleWorkspace,
    ContentBoundary(String),
    Io(std::io::Error),
    Revision(String),
}

impl fmt::Display for ResourceImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWorkspace => formatter.write_str("workspace inventory is invalid"),
            Self::NodeUnavailable => formatter.write_str("resource owner node is unavailable"),
            Self::InvalidName => formatter.write_str("resource name is not a portable file name"),
            Self::ReservedName => formatter.write_str("resource name is reserved"),
            Self::Empty => formatter.write_str("resource is empty"),
            Self::TooLarge => formatter.write_str("resource exceeds the 32 MiB Stage 1C limit"),
            Self::AlreadyExists => formatter.write_str("resource already exists"),
            Self::StaleWorkspace => formatter.write_str("workspace changed after resource preview"),
            Self::ContentBoundary(message) => {
                write!(
                    formatter,
                    "resource target crosses the content boundary: {message}"
                )
            }
            Self::Io(error) => write!(formatter, "resource I/O failed: {error}"),
            Self::Revision(message) => {
                write!(formatter, "resource revision check failed: {message}")
            }
        }
    }
}

impl std::error::Error for ResourceImportError {}

/// Plans creation of one immutable resource owned by a node.
///
/// # Errors
///
/// Fails for invalid workspaces, missing nodes, unsafe or reserved names,
/// empty/oversized content, conflicts, or workspace-revision failures.
pub fn plan_import_resource(
    root: impl AsRef<Path>,
    node_id: NodeId,
    name: &str,
    bytes: Vec<u8>,
) -> Result<ResourceImportPlan, ResourceImportError> {
    let root = root.as_ref();
    let inventory = scan_workspace(root);
    if !inventory.is_valid() {
        return Err(ResourceImportError::InvalidWorkspace);
    }
    validate_resource_name(name)?;
    if bytes.is_empty() {
        return Err(ResourceImportError::Empty);
    }
    if bytes.len() > MAX_RESOURCE_BYTES {
        return Err(ResourceImportError::TooLarge);
    }
    let node = inventory
        .nodes
        .into_iter()
        .find(|node| node.id == Some(node_id))
        .ok_or(ResourceImportError::NodeUnavailable)?;
    let target_path = node.path.join(name);
    validate_managed_file_path(root, &target_path)
        .map_err(|error| ResourceImportError::ContentBoundary(error.to_string()))?;
    if target_path.exists() {
        return Err(ResourceImportError::AlreadyExists);
    }
    let base_revision = read_workspace_revision(root)
        .map_err(|error| ResourceImportError::Revision(error.to_string()))?;
    Ok(ResourceImportPlan {
        plan_id: Uuid::new_v4().to_string(),
        node_id,
        name: name.to_owned(),
        byte_length: bytes.len(),
        base_revision,
        workspace_root: root.to_path_buf(),
        target_path,
        bytes,
    })
}

/// Atomically commits a previously reviewed resource plan.
///
/// # Errors
///
/// Fails without overwriting when the workspace changed, the target appeared,
/// staging/persistence failed, or verification did not reproduce the bytes.
pub fn commit_import_resource(
    plan: ResourceImportPlan,
) -> Result<ImportedResource, ResourceImportError> {
    let current = read_workspace_revision(&plan.workspace_root)
        .map_err(|error| ResourceImportError::Revision(error.to_string()))?;
    if current != plan.base_revision {
        return Err(ResourceImportError::StaleWorkspace);
    }
    if plan.target_path.exists() {
        return Err(ResourceImportError::AlreadyExists);
    }
    let parent = plan
        .target_path
        .parent()
        .ok_or(ResourceImportError::NodeUnavailable)?;
    let mut staged = Builder::new()
        .prefix(".__weftext-resource-")
        .tempfile_in(parent)
        .map_err(ResourceImportError::Io)?;
    staged
        .write_all(&plan.bytes)
        .and_then(|()| staged.flush())
        .and_then(|()| staged.as_file().sync_all())
        .map_err(ResourceImportError::Io)?;
    staged
        .persist_noclobber(&plan.target_path)
        .map_err(|error| ResourceImportError::Io(error.error))?;
    let verified = fs::read(&plan.target_path).map_err(ResourceImportError::Io)?;
    if verified != plan.bytes {
        return Err(ResourceImportError::Io(std::io::Error::other(
            "persisted resource verification failed",
        )));
    }
    let workspace_revision = read_workspace_revision(&plan.workspace_root)
        .map_err(|error| ResourceImportError::Revision(error.to_string()))?;
    Ok(ImportedResource {
        node_id: plan.node_id,
        name: plan.name,
        byte_length: plan.byte_length,
        workspace_revision,
    })
}

fn validate_resource_name(name: &str) -> Result<(), ResourceImportError> {
    if name.is_empty()
        || name.trim() != name
        || Path::new(name).components().count() != 1
        || !matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
        || name.chars().any(|character| {
            character.is_control()
                || matches!(
                    character,
                    '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
                )
        })
        || name.ends_with(['.', ' '])
    {
        return Err(ResourceImportError::InvalidName);
    }
    let lower = name.to_ascii_lowercase();
    if lower == ANNOTATIONS_FILE_NAME
        || lower == ".git"
        || lower.starts_with(".__weftext-transaction-")
        || lower.starts_with(".__weftext-resource-")
    {
        return Err(ResourceImportError::ReservedName);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_workspace, read_node_document};
    use tempfile::tempdir;

    #[test]
    fn imports_a_resource_without_touching_the_node_document() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("Workspace");
        let created = create_workspace(&root).expect("workspace");
        let before = read_node_document(&root).expect("document");
        let plan = plan_import_resource(&root, created.id, "diagram.png", b"png-bytes".to_vec())
            .expect("plan");
        let committed = commit_import_resource(plan).expect("commit");
        assert_eq!(committed.name, "diagram.png");
        assert_eq!(
            fs::read(root.join("diagram.png")).expect("resource"),
            b"png-bytes"
        );
        assert_eq!(
            read_node_document(&root).expect("document").source,
            before.source
        );
    }

    #[test]
    fn imports_markdown_as_a_resource_and_refuses_stale_plans() {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("Workspace");
        let created = create_workspace(&root).expect("workspace");
        let markdown = plan_import_resource(
            &root,
            created.id,
            "attachment.md",
            b"# Exact attachment\n".to_vec(),
        )
        .expect("Markdown resource plan");
        commit_import_resource(markdown).expect("Markdown resource commit");
        let inventory = scan_workspace(&root);
        assert!(inventory.is_valid(), "{:?}", inventory.issues);
        assert!(inventory.content.iter().any(|entry| {
            entry.kind == crate::WorkspaceContentKind::Resource
                && entry.relative_path == "attachment.md"
                && entry.owner_node_id == Some(created.id)
        }));
        let plan =
            plan_import_resource(&root, created.id, "image.png", b"first".to_vec()).expect("plan");
        fs::write(root.join("other.bin"), b"external").expect("external");
        assert!(matches!(
            commit_import_resource(plan),
            Err(ResourceImportError::StaleWorkspace)
        ));
    }
}
