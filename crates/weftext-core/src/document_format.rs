use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Managed-document generations recognized by runtime boundaries.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceDocumentGeneration {
    AsciiDocV1,
    #[default]
    Unsupported,
}

/// Stable description of the currently selected workspace document layout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDocumentFormat {
    pub generation: WorkspaceDocumentGeneration,
    pub canonical_extension: &'static str,
    pub media_type: &'static str,
}

pub const CURRENT_WORKSPACE_DOCUMENT_FORMAT: WorkspaceDocumentFormat = WorkspaceDocumentFormat {
    generation: WorkspaceDocumentGeneration::AsciiDocV1,
    canonical_extension: "adoc",
    media_type: "text/asciidoc",
};

pub const ASCIIDOC_WORKSPACE_DOCUMENT_FORMAT: WorkspaceDocumentFormat = WorkspaceDocumentFormat {
    generation: WorkspaceDocumentGeneration::AsciiDocV1,
    canonical_extension: "adoc",
    media_type: "text/asciidoc",
};

pub const UNSUPPORTED_WORKSPACE_DOCUMENT_FORMAT: WorkspaceDocumentFormat =
    WorkspaceDocumentFormat {
        generation: WorkspaceDocumentGeneration::Unsupported,
        canonical_extension: "",
        media_type: "application/octet-stream",
    };

pub const WORKSPACE_FORMAT_MARKER_FILE: &str = weftext_asciidoc::GENERATION_MARKER_FILE;
pub const ASCIIDOC_V1_MARKER: &[u8] = weftext_asciidoc::GENERATION_MARKER_V1;

/// Fixed content classification for visible, non-canonical Markdown files.
///
/// This is deliberately independent from the managed-document generation:
/// a future managed format must not silently reclassify unmanaged Markdown as
/// a resource, or reclassify its own extension as unmanaged Markdown.
pub const UNMANAGED_MARKDOWN_EXTENSION: &str = "md";

/// Selects the canonical runtime generation from the exact root marker.
/// Missing, malformed, unknown, or unreadable markers fail closed.
#[must_use]
pub fn workspace_document_format(root: &Path) -> WorkspaceDocumentFormat {
    match fs::read(root.join(WORKSPACE_FORMAT_MARKER_FILE)) {
        Ok(bytes) if bytes == ASCIIDOC_V1_MARKER => ASCIIDOC_WORKSPACE_DOCUMENT_FORMAT,
        Ok(_) | Err(_) => UNSUPPORTED_WORKSPACE_DOCUMENT_FORMAT,
    }
}

/// Returns the canonical managed-document filename for an explicit generation.
#[must_use]
pub fn canonical_document_file_name_for(
    generation: WorkspaceDocumentGeneration,
    node_name: &str,
) -> Option<String> {
    let extension = match generation {
        WorkspaceDocumentGeneration::AsciiDocV1 => "adoc",
        WorkspaceDocumentGeneration::Unsupported => return None,
    };
    Some(format!("{node_name}.{extension}"))
}

/// Returns the canonical managed-document path for an explicit generation.
#[must_use]
pub fn canonical_document_path_for(
    generation: WorkspaceDocumentGeneration,
    node_directory: &Path,
    node_name: &str,
) -> Option<PathBuf> {
    canonical_document_file_name_for(generation, node_name)
        .map(|file_name| node_directory.join(file_name))
}

/// Returns the portable canonical-document locator for an explicit generation.
#[must_use]
pub fn canonical_document_locator_for(
    generation: WorkspaceDocumentGeneration,
    node_locator: &str,
    node_name: &str,
) -> Option<String> {
    let file_name = canonical_document_file_name_for(generation, node_name)?;
    Some(if node_locator.is_empty() {
        file_name
    } else {
        format!("{node_locator}/{file_name}")
    })
}

/// Returns the one canonical managed-document filename for `node_name`.
#[must_use]
pub fn canonical_document_file_name(node_name: &str) -> String {
    format!(
        "{node_name}.{}",
        CURRENT_WORKSPACE_DOCUMENT_FORMAT.canonical_extension
    )
}

/// Returns the canonical managed-document path beneath a validated node directory.
#[must_use]
pub fn canonical_document_path(node_directory: &Path, node_name: &str) -> PathBuf {
    node_directory.join(canonical_document_file_name(node_name))
}

/// Returns the portable canonical-document locator for a node locator and name.
#[must_use]
pub fn canonical_document_locator(node_locator: &str, node_name: &str) -> String {
    let file_name = canonical_document_file_name(node_name);
    if node_locator.is_empty() {
        file_name
    } else {
        format!("{node_locator}/{file_name}")
    }
}

/// Reports whether a visible ordinary file belongs to the unmanaged Markdown
/// content class. Canonical managed documents are identified separately.
#[must_use]
pub fn is_unmanaged_markdown_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some(UNMANAGED_MARKDOWN_EXTENSION)
}

/// Removes the one canonical managed-document suffix from a node locator.
///
/// `.md` locators belong to explicit importer input and are deliberately not
/// accepted by the normal managed-link runtime.
#[must_use]
pub fn strip_optional_canonical_extension(locator: &str) -> &str {
    locator.strip_suffix(".adoc").unwrap_or(locator)
}
