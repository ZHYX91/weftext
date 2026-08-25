use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::content_boundary::{linked_or_reparse, validate_managed_node_path};
use crate::frontmatter::new_node_document;
use crate::{
    ASCIIDOC_V1_MARKER, NodeId, WORKSPACE_FORMAT_MARKER_FILE, canonical_document_path,
    parse_node_metadata, read_node_document,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatedNode {
    pub id: NodeId,
    pub path: PathBuf,
    pub document_path: PathBuf,
}

/// Create a workspace root as a same-named directory/document node.
///
/// # Errors
///
/// Returns an error when the target name is invalid, its parent is missing, the
/// target already exists, or an atomic filesystem operation fails.
pub fn create_workspace(root: impl AsRef<Path>) -> Result<CreatedNode, WorkspaceError> {
    let root = root.as_ref();
    reject_nested_workspace(root)?;
    create_node_at(root, true)
}

/// Create a child node below an existing Weftext node.
///
/// # Errors
///
/// Returns an error when the parent is not a valid Weftext node, the child name
/// is invalid, the target already exists, or an atomic filesystem operation fails.
pub fn create_child_node(
    parent: impl AsRef<Path>,
    name: &str,
) -> Result<CreatedNode, WorkspaceError> {
    validate_name(name)?;
    let selected_parent = parent.as_ref();
    let metadata = fs::symlink_metadata(selected_parent).map_err(WorkspaceError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(WorkspaceError::InvalidParent(
            "parent node cannot be a link or reparse point".to_owned(),
        ));
    }
    let parent = fs::canonicalize(selected_parent).map_err(WorkspaceError::Io)?;
    validate_existing_node(&parent)?;
    validate_managed_node_path(&parent.join(name))
        .map_err(|error| WorkspaceError::InvalidParent(error.to_string()))?;
    create_node_at(&parent.join(name), false)
}

fn create_node_at(target: &Path, write_format_marker: bool) -> Result<CreatedNode, WorkspaceError> {
    if target.exists() {
        return Err(WorkspaceError::AlreadyExists(target.to_path_buf()));
    }
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| WorkspaceError::InvalidName("node name must be UTF-8".to_owned()))?;
    validate_name(name)?;
    let parent = target.parent().ok_or_else(|| {
        WorkspaceError::InvalidName("node must have a parent directory".to_owned())
    })?;
    if !parent.is_dir() {
        return Err(WorkspaceError::ParentMissing(parent.to_path_buf()));
    }

    let id = NodeId::new_v4();
    let staging = parent.join(format!(".__weftext-transaction-{id}"));
    if staging.exists() {
        return Err(WorkspaceError::AlreadyExists(staging));
    }
    fs::create_dir(&staging).map_err(WorkspaceError::Io)?;
    let staged_document = canonical_document_path(&staging, name);
    let result = (|| {
        if write_format_marker {
            let mut marker = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(staging.join(WORKSPACE_FORMAT_MARKER_FILE))
                .map_err(WorkspaceError::Io)?;
            marker
                .write_all(ASCIIDOC_V1_MARKER)
                .map_err(WorkspaceError::Io)?;
            marker.sync_all().map_err(WorkspaceError::Io)?;
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staged_document)
            .map_err(WorkspaceError::Io)?;
        file.write_all(new_node_document(id).as_bytes())
            .map_err(WorkspaceError::Io)?;
        file.sync_all().map_err(WorkspaceError::Io)?;
        drop(file);
        fs::rename(&staging, target).map_err(WorkspaceError::Io)?;
        Ok(CreatedNode {
            id,
            path: target.to_path_buf(),
            document_path: canonical_document_path(target, name),
        })
    })();
    if result.is_err() && staging.exists() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn reject_nested_workspace(target: &Path) -> Result<(), WorkspaceError> {
    let parent = target.parent().ok_or_else(|| {
        WorkspaceError::InvalidName("workspace must have a parent directory".to_owned())
    })?;
    if !parent.is_dir() {
        return Ok(());
    }
    let parent = fs::canonicalize(parent).map_err(WorkspaceError::Io)?;
    for ancestor in parent.ancestors() {
        let marker = ancestor.join(WORKSPACE_FORMAT_MARKER_FILE);
        match fs::symlink_metadata(&marker) {
            Ok(_) => {
                return Err(WorkspaceError::InvalidParent(format!(
                    "workspace root cannot be nested beneath existing format marker {}",
                    marker.display()
                )));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(WorkspaceError::Io(error)),
        }
    }
    Ok(())
}

fn validate_existing_node(path: &Path) -> Result<(), WorkspaceError> {
    validate_managed_node_path(path)
        .map_err(|error| WorkspaceError::InvalidParent(error.to_string()))?;
    let snapshot = read_node_document(path)
        .map_err(|error| WorkspaceError::InvalidParent(error.to_string()))?;
    let metadata = parse_node_metadata(&snapshot.source)
        .map_err(|error| WorkspaceError::InvalidParent(error.to_string()))?;
    if metadata.id.is_none() {
        return Err(WorkspaceError::InvalidParent(
            "parent node is missing weftext.id".to_owned(),
        ));
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), WorkspaceError> {
    validate_node_name(name, false)
}

pub(crate) fn validate_node_name(
    name: &str,
    allow_reserved_trash: bool,
) -> Result<(), WorkspaceError> {
    crate::portable_name::validate_portable_node_name(name, allow_reserved_trash)
        .map_err(|message| WorkspaceError::InvalidName(message.to_owned()))
}

/// Shared syntax-only seam for resource names whose caller applies its own
/// role-specific reservations and error category.
pub(crate) fn validate_portable_path_component(
    name: &str,
    allow_reserved_trash: bool,
) -> Result<(), WorkspaceError> {
    crate::portable_name::validate_portable_name_component(name, allow_reserved_trash)
        .map_err(|message| WorkspaceError::InvalidName(message.to_owned()))
}

#[derive(Debug)]
pub enum WorkspaceError {
    InvalidName(String),
    InvalidParent(String),
    ParentMissing(PathBuf),
    AlreadyExists(PathBuf),
    Io(io::Error),
}

impl fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(message) | Self::InvalidParent(message) => {
                formatter.write_str(message)
            }
            Self::ParentMissing(path) => {
                write!(formatter, "parent directory is missing: {}", path.display())
            }
            Self::AlreadyExists(path) => {
                write!(formatter, "target already exists: {}", path.display())
            }
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WorkspaceError {}
