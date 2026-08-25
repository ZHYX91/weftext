use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::Builder;

use crate::content_boundary::{linked_or_reparse, validate_managed_node_path};
use crate::frontmatter::validate_node_metadata_scope;
use crate::{
    ASCIIDOC_V1_MARKER, DocumentProfileId, NodeId, NodeMetadataScope, WORKSPACE_FORMAT_MARKER_FILE,
    WorkspaceDocumentGeneration, canonical_document_path, parse_node_metadata,
};

const TRANSACTION_PREFIX: &str = ".__weftext-transaction-document-";

/// Digest of the exact UTF-8 bytes in one node document revision.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct DocumentRevision(String);

impl DocumentRevision {
    /// Computes the revision of an exact source byte sequence.
    #[must_use]
    pub fn from_source(source: &str) -> Self {
        Self(format!("{:x}", Sha256::digest(source.as_bytes())))
    }

    /// Parses a canonical lowercase SHA-256 revision.
    ///
    /// # Errors
    ///
    /// Returns an error unless `value` contains exactly 64 lowercase hexadecimal characters.
    pub fn parse(value: &str) -> Result<Self, DocumentError> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(DocumentError::InvalidRevision(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }

    /// Returns the canonical digest text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DocumentRevision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Exact source and identity observed for one node document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentSnapshot {
    pub profile: DocumentProfileId,
    pub node_id: NodeId,
    pub node_directory: PathBuf,
    pub document_path: PathBuf,
    pub revision: DocumentRevision,
    pub source: String,
}

/// One exact UTF-8 byte-range replacement against a base revision.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEdit {
    pub start: u64,
    pub end: u64,
    pub replacement: String,
}

/// Deterministic preview of a document edit that has not committed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentEditPlan {
    pub node_id: NodeId,
    pub node_directory: PathBuf,
    pub document_path: PathBuf,
    pub base_revision: DocumentRevision,
    pub next_revision: DocumentRevision,
    pub edits: Vec<DocumentEdit>,
    pub old_length: u64,
    pub new_length: u64,
    pub changed: bool,
    next_source: String,
}

impl DocumentEditPlan {
    /// Returns the exact planned source for a UI diff or approved commit.
    #[must_use]
    pub fn next_source(&self) -> &str {
        &self.next_source
    }
}

/// Verified result of an atomic single-document commit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedDocument {
    pub node_id: NodeId,
    pub document_path: PathBuf,
    pub revision: DocumentRevision,
    pub length: u64,
}

/// Reads a valid Weftext node document without normalizing any source bytes.
///
/// # Errors
///
/// Returns an error when the node path is invalid, linked, unreadable, not UTF-8,
/// or lacks valid unambiguous Weftext identity metadata.
pub fn read_node_document(
    node_directory: impl AsRef<Path>,
) -> Result<DocumentSnapshot, DocumentError> {
    let node_directory = node_directory.as_ref();
    validate_managed_node_path(node_directory)
        .map_err(|error| DocumentError::ContentBoundary(error.to_string()))?;
    let directory_metadata = fs::symlink_metadata(node_directory).map_err(DocumentError::Io)?;
    if linked_or_reparse(&directory_metadata) {
        return Err(DocumentError::SymlinkUnsupported(
            node_directory.to_path_buf(),
        ));
    }
    if !directory_metadata.is_dir() {
        return Err(DocumentError::InvalidNodePath(node_directory.to_path_buf()));
    }
    let name = node_directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DocumentError::InvalidNodePath(node_directory.to_path_buf()))?;
    let is_workspace_root = match workspace_generation_for_node(node_directory)? {
        (WorkspaceDocumentGeneration::AsciiDocV1, is_workspace_root) => is_workspace_root,
        (WorkspaceDocumentGeneration::Unsupported, _) => {
            return Err(DocumentError::InvalidWorkspaceFormat(
                node_directory.to_path_buf(),
            ));
        }
    };
    let document_path = canonical_document_path(node_directory, name);
    let document_metadata = fs::symlink_metadata(&document_path).map_err(DocumentError::Io)?;
    if linked_or_reparse(&document_metadata) {
        return Err(DocumentError::SymlinkUnsupported(document_path));
    }
    if !document_metadata.is_file() {
        return Err(DocumentError::InvalidNodePath(document_path));
    }
    let bytes = fs::read(&document_path).map_err(DocumentError::Io)?;
    let source =
        String::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8(document_path.clone()))?;
    let metadata = parse_node_metadata(&source)
        .map_err(|error| DocumentError::InvalidMetadata(error.to_string()))?;
    validate_metadata_scope(&metadata, is_workspace_root)?;
    let node_id = metadata.id.ok_or(DocumentError::MissingIdentity)?;
    let revision = DocumentRevision::from_source(&source);
    Ok(DocumentSnapshot {
        profile: DocumentProfileId::AsciiDocV1,
        node_id,
        node_directory: node_directory.to_path_buf(),
        document_path,
        revision,
        source,
    })
}

fn workspace_generation_for_node(
    node_directory: &Path,
) -> Result<(WorkspaceDocumentGeneration, bool), DocumentError> {
    let mut selected = None;
    for ancestor in node_directory.ancestors() {
        let marker = ancestor.join(WORKSPACE_FORMAT_MARKER_FILE);
        let metadata = match fs::symlink_metadata(&marker) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(DocumentError::Io(error)),
        };
        if linked_or_reparse(&metadata) || !metadata.is_file() {
            return Err(DocumentError::InvalidWorkspaceFormat(marker));
        }
        let bytes = fs::read(&marker).map_err(DocumentError::Io)?;
        if bytes != ASCIIDOC_V1_MARKER || selected.is_some() {
            return Err(DocumentError::InvalidWorkspaceFormat(marker));
        }
        selected = Some((
            WorkspaceDocumentGeneration::AsciiDocV1,
            ancestor == node_directory,
        ));
    }
    selected.ok_or_else(|| {
        DocumentError::InvalidWorkspaceFormat(node_directory.join(WORKSPACE_FORMAT_MARKER_FILE))
    })
}

/// Builds a deterministic, non-mutating edit plan against one exact revision.
///
/// # Errors
///
/// Returns an error for a stale revision, invalid/overlapping UTF-8 ranges,
/// invalid resulting metadata, or an attempted node identity change.
pub fn plan_document_edit(
    node_directory: impl AsRef<Path>,
    base_revision: &DocumentRevision,
    edits: impl IntoIterator<Item = DocumentEdit>,
) -> Result<DocumentEditPlan, DocumentError> {
    let snapshot = read_node_document(node_directory)?;
    require_revision(base_revision, &snapshot.revision)?;
    plan_document_edit_from_snapshot(&snapshot, edits)
}

pub(crate) fn plan_document_edit_from_snapshot(
    snapshot: &DocumentSnapshot,
    edits: impl IntoIterator<Item = DocumentEdit>,
) -> Result<DocumentEditPlan, DocumentError> {
    let edits = canonicalize_edits(edits, &snapshot.source)?;
    let next_source = apply_edits(&snapshot.source, &edits);
    let next_metadata = parse_node_metadata(&next_source)
        .map_err(|error| DocumentError::InvalidMetadata(error.to_string()))?;
    let (_, is_workspace_root) = workspace_generation_for_node(&snapshot.node_directory)?;
    validate_metadata_scope(&next_metadata, is_workspace_root)?;
    let next_id = next_metadata.id.ok_or(DocumentError::MissingIdentity)?;
    if next_id != snapshot.node_id {
        return Err(DocumentError::IdentityChanged {
            expected: snapshot.node_id,
            actual: next_id,
        });
    }
    let next_revision = DocumentRevision::from_source(&next_source);
    Ok(DocumentEditPlan {
        node_id: snapshot.node_id,
        node_directory: snapshot.node_directory.clone(),
        document_path: snapshot.document_path.clone(),
        base_revision: snapshot.revision.clone(),
        next_revision: next_revision.clone(),
        edits,
        old_length: u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX),
        new_length: u64::try_from(next_source.len()).unwrap_or(u64::MAX),
        changed: snapshot.revision != next_revision,
        next_source,
    })
}

fn validate_metadata_scope(
    metadata: &crate::NodeMetadata,
    is_workspace_root: bool,
) -> Result<(), DocumentError> {
    let scope = if is_workspace_root {
        NodeMetadataScope::WorkspaceRoot
    } else {
        NodeMetadataScope::Node
    };
    validate_node_metadata_scope(metadata, scope)
        .map_err(|error| DocumentError::InvalidMetadata(error.to_string()))
}

/// Atomically commits one previously previewed document edit.
///
/// # Errors
///
/// Returns an error when the target revision or identity changed after preview,
/// staging/persistence fails, or post-commit verification differs from the plan.
pub fn commit_document_edit(plan: &DocumentEditPlan) -> Result<CommittedDocument, DocumentError> {
    let current = read_node_document(&plan.node_directory)?;
    require_identity(plan.node_id, current.node_id)?;
    require_revision(&plan.base_revision, &current.revision)?;
    if !plan.changed {
        return Ok(committed_from_snapshot(current));
    }

    let mut staged = Builder::new()
        .prefix(TRANSACTION_PREFIX)
        .tempfile_in(&plan.node_directory)
        .map_err(DocumentError::Io)?;
    staged
        .as_file()
        .set_permissions(
            fs::metadata(&plan.document_path)
                .map_err(DocumentError::Io)?
                .permissions(),
        )
        .map_err(DocumentError::Io)?;
    staged
        .write_all(plan.next_source.as_bytes())
        .map_err(DocumentError::Io)?;
    staged.flush().map_err(DocumentError::Io)?;
    staged.as_file().sync_all().map_err(DocumentError::Io)?;
    verify_staged_source(&staged, &plan.next_revision)?;

    let latest = read_node_document(&plan.node_directory)?;
    require_identity(plan.node_id, latest.node_id)?;
    require_revision(&plan.base_revision, &latest.revision)?;

    let persisted = staged
        .persist(&plan.document_path)
        .map_err(|error| DocumentError::Persist(error.error))?;
    persisted.sync_all().map_err(DocumentError::Io)?;

    let committed = read_node_document(&plan.node_directory)?;
    require_identity(plan.node_id, committed.node_id)?;
    if committed.revision != plan.next_revision {
        return Err(DocumentError::VerificationFailed {
            expected: plan.next_revision.clone(),
            actual: committed.revision,
        });
    }
    Ok(committed_from_snapshot(committed))
}

fn canonicalize_edits(
    edits: impl IntoIterator<Item = DocumentEdit>,
    source: &str,
) -> Result<Vec<DocumentEdit>, DocumentError> {
    let mut edits = edits.into_iter().collect::<Vec<_>>();
    edits.sort_by(|left, right| {
        left.start
            .cmp(&right.start)
            .then_with(|| left.end.cmp(&right.end))
            .then_with(|| left.replacement.cmp(&right.replacement))
    });
    let source_len = u64::try_from(source.len()).unwrap_or(u64::MAX);
    let mut previous_end = 0;
    for (index, edit) in edits.iter().enumerate() {
        if edit.start > edit.end || edit.end > source_len {
            return Err(DocumentError::InvalidEditRange {
                start: edit.start,
                end: edit.end,
                source_len,
            });
        }
        if index > 0 && edit.start < previous_end {
            return Err(DocumentError::OverlappingEdits);
        }
        let start = usize::try_from(edit.start).map_err(|_| DocumentError::InvalidEditRange {
            start: edit.start,
            end: edit.end,
            source_len,
        })?;
        let end = usize::try_from(edit.end).map_err(|_| DocumentError::InvalidEditRange {
            start: edit.start,
            end: edit.end,
            source_len,
        })?;
        if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
            return Err(DocumentError::NonCharacterBoundary {
                start: edit.start,
                end: edit.end,
            });
        }
        previous_end = edit.end;
    }
    Ok(edits)
}

fn apply_edits(source: &str, edits: &[DocumentEdit]) -> String {
    let replacement_growth = edits.iter().fold(0_usize, |growth, edit| {
        growth.saturating_add(edit.replacement.len())
    });
    let mut result = String::with_capacity(source.len().saturating_add(replacement_growth));
    let mut cursor = 0;
    for edit in edits {
        let start = usize::try_from(edit.start).unwrap_or(source.len());
        let end = usize::try_from(edit.end).unwrap_or(source.len());
        result.push_str(&source[cursor..start]);
        result.push_str(&edit.replacement);
        cursor = end;
    }
    result.push_str(&source[cursor..]);
    result
}

fn verify_staged_source(
    staged: &tempfile::NamedTempFile,
    expected: &DocumentRevision,
) -> Result<(), DocumentError> {
    let mut reopened = staged.reopen().map_err(DocumentError::Io)?;
    let mut bytes = Vec::new();
    reopened
        .read_to_end(&mut bytes)
        .map_err(DocumentError::Io)?;
    let source = String::from_utf8(bytes)
        .map_err(|_| DocumentError::InvalidUtf8(staged.path().to_path_buf()))?;
    let actual = DocumentRevision::from_source(&source);
    if &actual != expected {
        return Err(DocumentError::VerificationFailed {
            expected: expected.clone(),
            actual,
        });
    }
    Ok(())
}

fn require_revision(
    expected: &DocumentRevision,
    actual: &DocumentRevision,
) -> Result<(), DocumentError> {
    if expected == actual {
        Ok(())
    } else {
        Err(DocumentError::StaleRevision {
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}

fn require_identity(expected: NodeId, actual: NodeId) -> Result<(), DocumentError> {
    if expected == actual {
        Ok(())
    } else {
        Err(DocumentError::IdentityChanged { expected, actual })
    }
}

fn committed_from_snapshot(snapshot: DocumentSnapshot) -> CommittedDocument {
    CommittedDocument {
        node_id: snapshot.node_id,
        document_path: snapshot.document_path,
        revision: snapshot.revision,
        length: u64::try_from(snapshot.source.len()).unwrap_or(u64::MAX),
    }
}

/// Structured document read, preview, and commit failures.
#[derive(Debug)]
pub enum DocumentError {
    InvalidNodePath(PathBuf),
    AmbiguousDocumentGeneration(PathBuf),
    InvalidWorkspaceFormat(PathBuf),
    SymlinkUnsupported(PathBuf),
    InvalidUtf8(PathBuf),
    InvalidMetadata(String),
    MissingIdentity,
    InvalidRevision(String),
    StaleRevision {
        expected: DocumentRevision,
        actual: DocumentRevision,
    },
    IdentityChanged {
        expected: NodeId,
        actual: NodeId,
    },
    InvalidEditRange {
        start: u64,
        end: u64,
        source_len: u64,
    },
    NonCharacterBoundary {
        start: u64,
        end: u64,
    },
    OverlappingEdits,
    VerificationFailed {
        expected: DocumentRevision,
        actual: DocumentRevision,
    },
    ContentBoundary(String),
    Io(std::io::Error),
    Persist(std::io::Error),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodePath(path) => {
                write!(formatter, "invalid Weftext node path: {}", path.display())
            }
            Self::AmbiguousDocumentGeneration(path) => write!(
                formatter,
                "node contains both Markdown and AsciiDoc canonical documents: {}",
                path.display()
            ),
            Self::InvalidWorkspaceFormat(path) => write!(
                formatter,
                "managed document requires exact weftext.asciidoc.v1 workspace marker: {}",
                path.display()
            ),
            Self::SymlinkUnsupported(path) => {
                write!(
                    formatter,
                    "linked node path is unsupported: {}",
                    path.display()
                )
            }
            Self::InvalidUtf8(path) => {
                write!(formatter, "node document is not UTF-8: {}", path.display())
            }
            Self::InvalidMetadata(message) => write!(formatter, "invalid node metadata: {message}"),
            Self::MissingIdentity => formatter.write_str("node document is missing weftext.id"),
            Self::InvalidRevision(value) => write!(formatter, "invalid document revision: {value}"),
            Self::StaleRevision { expected, actual } => write!(
                formatter,
                "stale document revision: expected {expected}, found {actual}"
            ),
            Self::IdentityChanged { expected, actual } => write!(
                formatter,
                "document edit changes node identity from {expected} to {actual}"
            ),
            Self::InvalidEditRange {
                start,
                end,
                source_len,
            } => write!(
                formatter,
                "invalid document edit range {start}..{end} for {source_len} source bytes"
            ),
            Self::NonCharacterBoundary { start, end } => write!(
                formatter,
                "document edit range {start}..{end} is not on UTF-8 character boundaries"
            ),
            Self::OverlappingEdits => formatter.write_str("document edits overlap"),
            Self::VerificationFailed { expected, actual } => write!(
                formatter,
                "committed document verification failed: expected {expected}, found {actual}"
            ),
            Self::ContentBoundary(message) => write!(
                formatter,
                "document is outside the managed content boundary: {message}"
            ),
            Self::Io(error) => write!(formatter, "document I/O failed: {error}"),
            Self::Persist(error) => {
                write!(formatter, "atomic document replacement failed: {error}")
            }
        }
    }
}

impl std::error::Error for DocumentError {}

/// Format-neutral name for an exact managed-document read.
pub type ExactSourceDocumentSnapshot = DocumentSnapshot;

/// Format-neutral name for one UTF-8 source replacement.
pub type Utf8SourceEdit = DocumentEdit;

/// Format-neutral name for a revision-checked source patch plan.
pub type Utf8SourcePatchPlan = DocumentEditPlan;
