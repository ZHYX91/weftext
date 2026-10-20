//! Snapshot-only knowledge algorithms. Core owns qualification and all file writes.
//! Inputs contain only the current authorized scope; coverage never describes hidden data.
mod annotations;
mod copy;
mod diagnostics;
mod heading_structure;
mod links;
mod mentions;
mod processing_settings;
mod proofreading;
mod relationship_graph;

pub use annotations::*;
pub use copy::*;
pub use diagnostics::*;
pub use heading_structure::*;
pub use links::*;
pub use mentions::*;
pub use processing_settings::*;
pub use proofreading::*;
pub use relationship_graph::*;

use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, KnowledgeError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeError {
    pub code: String,
    pub message: String,
}
impl KnowledgeError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for KnowledgeError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeRef {
    pub workspace_id: String,
    pub node_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Coverage {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSnapshot {
    pub owner: NodeRef,
    /// Canonical local file URL produced by the Core platform resolver.
    pub path: String,
    /// Canonical library-relative inventory path (slash separated).
    pub library_path: String,
    pub revision_id: String,
    pub source: String,
    pub title: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    /// Final semantic label set for this exact authorized target, not AST guesses.
    #[serde(default)]
    pub labels: Vec<String>,
    pub labels_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub key: String,
    pub path: String,
    pub revision_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventorySnapshot {
    pub generation: String,
    pub coverage: Coverage,
    pub nodes: Vec<NodeSnapshot>,
    #[serde(default)]
    pub resources: Vec<ResourceSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExactRange {
    pub from: usize,
    pub to: usize,
    pub utf16_from: usize,
    pub utf16_to: usize,
    pub raw: String,
}
impl ExactRange {
    pub fn new(source: &str, from: usize, to: usize) -> Result<Self> {
        let raw = source.get(from..to).ok_or_else(|| {
            KnowledgeError::new(
                "invalidRange",
                "The source range is not an exact UTF-8 range",
            )
        })?;
        Ok(Self {
            from,
            to,
            utf16_from: source[..from].encode_utf16().count(),
            utf16_to: source[..to].encode_utf16().count(),
            raw: raw.into(),
        })
    }
    pub fn verify(&self, source: &str) -> bool {
        Self::new(source, self.from, self.to).is_ok_and(|actual| actual == *self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePatch {
    pub range: ExactRange,
    pub replacement: String,
    pub after: String,
}
impl SourcePatch {
    pub fn new(source: &str, range: ExactRange, replacement: String) -> Result<Self> {
        if !range.verify(source) {
            return Err(KnowledgeError::new(
                "conflict",
                "The exact source preimage changed",
            ));
        }
        let after = format!(
            "{}{}{}",
            &source[..range.from],
            replacement,
            &source[range.to..]
        );
        let parsed = typst_syntax::Source::detached(&after);
        if !parsed.root().errors_and_warnings().0.is_empty() {
            return Err(KnowledgeError::new(
                "invalidSource",
                "The proposed native source does not parse",
            ));
        }
        Ok(Self {
            range,
            replacement,
            after,
        })
    }
}

pub(crate) fn ordinary_text_range(source: &str, range: &ExactRange) -> bool {
    use unicode_segmentation::UnicodeSegmentation;
    let parsed = typst_syntax::Source::detached(source);
    if !parsed.root().errors_and_warnings().0.is_empty() || !range.verify(source) {
        return false;
    }
    let editor = weftext_engine::inspect_editor(source);
    if editor.unavailable_reason.is_some()
        || !editor.literal_regions.iter().any(|region| {
            region.range.writable && region.range.from <= range.from && range.to <= region.range.to
        })
    {
        return false;
    }
    let boundaries: std::collections::BTreeSet<_> = source
        .grapheme_indices(true)
        .map(|(at, _)| at)
        .chain(std::iter::once(source.len()))
        .collect();
    if !boundaries.contains(&range.from) || !boundaries.contains(&range.to) {
        return false;
    }
    fn walk(node: &typst_syntax::LinkedNode<'_>, range: &ExactRange) -> bool {
        if node.get().cast::<typst_syntax::ast::Text>().is_some()
            && node.range().start <= range.from
            && range.to <= node.range().end
        {
            return true;
        }
        node.children().any(|child| walk(&child, range))
    }
    walk(&typst_syntax::LinkedNode::new(parsed.root()), range)
}

/// Text suggestions use one literal leaf, or the Editor's separately proved
/// empty trailing body slot. Never invent a neighboring nonempty preimage.
pub fn writable_suggestion_range(source: &str, range: &ExactRange) -> bool {
    ordinary_text_range(source, range)
        || range.from == range.to
            && range.verify(source)
            && (source.is_empty() || source.ends_with("\n\n"))
            && weftext_engine::inspect_editor(source)
                .body_insertion
                .is_some_and(|point| point.writable && point.from == range.from)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedFile {
    pub path: String,
    pub before: Option<Vec<u8>>,
    pub after: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Loss {
    pub path: String,
    pub code: String,
    pub detail: String,
}
