use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::content_boundary::{CONTENT_RULES_FILE_NAME, linked_or_reparse};
use crate::{InventoryIssueCode, WorkspaceContentKind, scan_workspace};

pub(crate) const WORKSPACE_TRANSACTION_PREFIX: &str = ".__weftext-transaction-workspace-";

/// Digest of the complete portable workspace state observed by a structural plan.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct WorkspaceRevision(String);

impl WorkspaceRevision {
    /// Parses a canonical lowercase SHA-256 workspace revision.
    ///
    /// # Errors
    ///
    /// Returns an error unless `value` contains exactly 64 lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, WorkspaceRevisionError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(WorkspaceRevisionError::InvalidRevision(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical digest text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkspaceRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Computes the revision of portable authority under one valid workspace root.
///
/// # Errors
///
/// Returns an error for an invalid inventory, unreadable or non-UTF-8 entry,
/// linked path, path escape, or filesystem failure.
pub fn read_workspace_revision(
    root: impl AsRef<Path>,
) -> Result<WorkspaceRevision, WorkspaceRevisionError> {
    let root = root.as_ref();
    let inventory = scan_workspace(root);
    read_workspace_revision_from_inventory(root, &inventory)
}

fn read_workspace_revision_from_inventory(
    root: &Path,
    inventory: &crate::WorkspaceInventory,
) -> Result<WorkspaceRevision, WorkspaceRevisionError> {
    let trash_storage_only = !inventory.issues.is_empty()
        && inventory
            .issues
            .iter()
            .all(|issue| crate::workspace_trash::is_trash_storage_path(root, &issue.path));
    if !inventory.is_valid() && !trash_storage_only {
        return Err(WorkspaceRevisionError::InvalidInventory(
            inventory
                .issues
                .first()
                .map_or(InventoryIssueCode::RootMissing, |issue| issue.code),
        ));
    }
    let mut records = Vec::new();
    let canonical_root = fs::canonicalize(root).map_err(WorkspaceRevisionError::Io)?;
    for node in &inventory.nodes {
        if crate::workspace_trash::is_trash_storage_path(root, &node.path) {
            continue;
        }
        if node.path != root {
            push_directory_record(root, &node.path, &canonical_root, &mut records)?;
        }
        push_file_record(root, &node.document_path, &canonical_root, &mut records)?;
        let annotation_sidecar = node.path.join(crate::ANNOTATIONS_FILE_NAME);
        match fs::symlink_metadata(&annotation_sidecar) {
            Ok(_) => push_file_record(root, &annotation_sidecar, &canonical_root, &mut records)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(WorkspaceRevisionError::Io(error)),
        }
    }
    for entry in &inventory.content {
        if entry.kind == WorkspaceContentKind::ManagedNode {
            continue;
        }
        let path = root.join(Path::new(&entry.relative_path));
        if entry.kind == WorkspaceContentKind::UnmanagedDirectory {
            push_directory_record(root, &path, &canonical_root, &mut records)?;
        } else {
            push_file_record(root, &path, &canonical_root, &mut records)?;
        }
    }
    let rules_path = root.join(CONTENT_RULES_FILE_NAME);
    if rules_path.exists() {
        push_file_record(root, &rules_path, &canonical_root, &mut records)?;
    }
    let format_marker = root.join(crate::WORKSPACE_FORMAT_MARKER_FILE);
    if format_marker.exists() {
        push_file_record(root, &format_marker, &canonical_root, &mut records)?;
    }
    let trash = root.join(crate::TRASH_DIRECTORY_NAME);
    if trash.exists() {
        push_portable_tree_records(root, &trash, &canonical_root, &mut records)?;
    }
    records.sort();
    records.dedup();

    let mut hasher = Sha256::new();
    hasher.update(b"weftext.workspace.revision.v1\0");
    for record in records {
        hasher.update(record);
        hasher.update([0]);
    }
    Ok(WorkspaceRevision(format!("{:x}", hasher.finalize())))
}

fn push_portable_tree_records(
    root: &Path,
    directory: &Path,
    canonical_root: &Path,
    records: &mut Vec<Vec<u8>>,
) -> Result<(), WorkspaceRevisionError> {
    push_directory_record(root, directory, canonical_root, records)?;
    let mut entries = fs::read_dir(directory)
        .map_err(WorkspaceRevisionError::Io)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(WorkspaceRevisionError::Io)?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(WorkspaceRevisionError::Io)?;
        if linked_or_reparse(&metadata) {
            return Err(WorkspaceRevisionError::SymlinkUnsupported(path));
        }
        if metadata.is_dir() {
            push_portable_tree_records(root, &path, canonical_root, records)?;
        } else if metadata.is_file() {
            push_file_record(root, &path, canonical_root, records)?;
        } else {
            return Err(WorkspaceRevisionError::PathEscape(path));
        }
    }
    Ok(())
}

fn push_directory_record(
    root: &Path,
    path: &Path,
    canonical_root: &Path,
    records: &mut Vec<Vec<u8>>,
) -> Result<(), WorkspaceRevisionError> {
    let metadata = fs::symlink_metadata(path).map_err(WorkspaceRevisionError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(WorkspaceRevisionError::SymlinkUnsupported(
            path.to_path_buf(),
        ));
    }
    if !metadata.is_dir() {
        return Err(WorkspaceRevisionError::PathEscape(path.to_path_buf()));
    }
    ensure_resolved_inside(path, canonical_root)?;
    records.push(format!("D\0{}", portable_relative_path(root, path)?).into_bytes());
    Ok(())
}

fn push_file_record(
    root: &Path,
    path: &Path,
    canonical_root: &Path,
    records: &mut Vec<Vec<u8>>,
) -> Result<(), WorkspaceRevisionError> {
    let metadata = fs::symlink_metadata(path).map_err(WorkspaceRevisionError::Io)?;
    if linked_or_reparse(&metadata) {
        return Err(WorkspaceRevisionError::SymlinkUnsupported(
            path.to_path_buf(),
        ));
    }
    if !metadata.is_file() {
        return Err(WorkspaceRevisionError::PathEscape(path.to_path_buf()));
    }
    ensure_resolved_inside(path, canonical_root)?;
    let relative = portable_relative_path(root, path)?;
    let bytes = fs::read(path).map_err(WorkspaceRevisionError::Io)?;
    let digest = Sha256::digest(&bytes);
    let mut record = format!("F\0{relative}\0{}\0", bytes.len()).into_bytes();
    record.extend_from_slice(&digest);
    records.push(record);
    Ok(())
}

fn ensure_resolved_inside(
    path: &Path,
    canonical_root: &Path,
) -> Result<(), WorkspaceRevisionError> {
    let canonical = fs::canonicalize(path).map_err(WorkspaceRevisionError::Io)?;
    if canonical.starts_with(canonical_root) {
        Ok(())
    } else {
        Err(WorkspaceRevisionError::PathEscape(path.to_path_buf()))
    }
}

pub(crate) fn portable_relative_path(
    root: &Path,
    path: &Path,
) -> Result<String, WorkspaceRevisionError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| WorkspaceRevisionError::PathEscape(path.to_path_buf()))?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(value) => pieces.push(
                value
                    .to_str()
                    .ok_or_else(|| WorkspaceRevisionError::NonUtf8Path(path.to_path_buf()))?,
            ),
            _ => return Err(WorkspaceRevisionError::PathEscape(path.to_path_buf())),
        }
    }
    if pieces.is_empty() {
        return Err(WorkspaceRevisionError::PathEscape(path.to_path_buf()));
    }
    Ok(pieces.join("/"))
}

/// Fail-closed workspace fingerprint failures.
#[derive(Debug)]
pub enum WorkspaceRevisionError {
    InvalidInventory(InventoryIssueCode),
    InvalidRevision(String),
    NonUtf8Path(PathBuf),
    SymlinkUnsupported(PathBuf),
    PathEscape(PathBuf),
    Io(std::io::Error),
}

impl fmt::Display for WorkspaceRevisionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInventory(code) => {
                write!(formatter, "workspace inventory is invalid: {code:?}")
            }
            Self::InvalidRevision(value) => {
                write!(formatter, "invalid workspace revision: {value}")
            }
            Self::NonUtf8Path(path) => {
                write!(formatter, "workspace path is not UTF-8: {}", path.display())
            }
            Self::SymlinkUnsupported(path) => {
                write!(
                    formatter,
                    "linked workspace path is unsupported: {}",
                    path.display()
                )
            }
            Self::PathEscape(path) => {
                write!(
                    formatter,
                    "workspace path escapes the selected root: {}",
                    path.display()
                )
            }
            Self::Io(error) => write!(formatter, "workspace revision I/O failed: {error}"),
        }
    }
}

impl std::error::Error for WorkspaceRevisionError {}
