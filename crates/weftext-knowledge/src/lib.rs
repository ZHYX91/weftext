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
use weftext_messages::{ErrorDescription, UiMessage};

pub type Result<T> = std::result::Result<T, KnowledgeError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KnowledgeError {
    pub code: String,
    #[serde(flatten)]
    pub description: Box<ErrorDescription>,
    pub product_diagnostics: Vec<ErrorDescription>,
}
impl KnowledgeError {
    pub fn new(code: &str, description: impl Into<ErrorDescription>) -> Self {
        Self {
            code: code.into(),
            description: Box::new(description.into()),
            product_diagnostics: Vec::new(),
        }
    }
    pub fn into_parts(self) -> (ErrorDescription, Vec<ErrorDescription>) {
        (*self.description, self.product_diagnostics)
    }
    pub fn with_diagnostics(mut self, diagnostics: Vec<ErrorDescription>) -> Self {
        self.product_diagnostics.extend(diagnostics);
        self
    }
    pub fn from_engine(code: &str, error: weftext_engine::EngineError) -> Self {
        let (description, diagnostics) = error.into_parts();
        Self::new(code, description).with_diagnostics(diagnostics)
    }
    pub fn from_data(code: &str, error: weftext_data::DataError) -> Self {
        let (kind, description, details) = error.into_parts();
        let source_code = match kind {
            weftext_data::DataErrorKind::Invalid => "dataInvalid",
            weftext_data::DataErrorKind::Unavailable => "dataUnavailable",
            weftext_data::DataErrorKind::Coverage => "dataCoverage",
            weftext_data::DataErrorKind::Budget => "dataBudget",
            weftext_data::DataErrorKind::NativeBudget => "dataNativeBudget",
            weftext_data::DataErrorKind::ClosedExpressionUnavailable => {
                "dataClosedExpressionUnavailable"
            }
        };
        Self::with_cause(
            code,
            UiMessage::error_knowledge_data_qualification(),
            source_code,
            description,
            details
                .map(weftext_data::DataErrorDetails::into_diagnostics)
                .unwrap_or_default(),
        )
    }
    pub(crate) fn with_cause(
        code: &str,
        explanation: UiMessage,
        source_code: &str,
        cause: ErrorDescription,
        mut diagnostics: Vec<ErrorDescription>,
    ) -> Self {
        let description = match ErrorDescription::new(explanation).try_caused_by(source_code, cause)
        {
            Ok(description) => description,
            Err(boundary) => {
                let header = ErrorDescription::new(boundary.original.explanation.clone());
                diagnostics.insert(0, boundary.original);
                boundary
                    .outer
                    .try_caused_by(boundary.code, header)
                    .expect("a fresh product header and one shallow cause fit the fixed budget")
            }
        };
        Self {
            code: code.into(),
            description: Box::new(description),
            product_diagnostics: diagnostics,
        }
    }
    pub fn from_losses(code: &str, losses: Vec<Loss>) -> Self {
        let mut diagnostics = Vec::new();
        for loss in losses {
            let item = Self::with_cause(
                "copyLoss",
                UiMessage::error_knowledge_copy_loss_at_path(
                    weftext_messages::params::ErrorKnowledgeCopyLossAtPath { path: loss.path },
                ),
                &loss.code,
                loss.description,
                loss.product_diagnostics,
            );
            let (description, related) = item.into_parts();
            diagnostics.push(description);
            diagnostics.extend(related);
        }
        Self::new(code, UiMessage::error_knowledge_copy_losses_declined())
            .with_diagnostics(diagnostics)
    }
}
impl std::fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.description)?;
        for diagnostic in &self.product_diagnostics {
            write!(f, "\n{diagnostic}")?;
        }
        Ok(())
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
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_knowledge_source_edit_utf_eight_range_invalid(),
                ),
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
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_knowledge_source_edit_source_preimage_changed(),
                ),
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
                weftext_messages::ErrorDescription::new(
                    weftext_messages::UiMessage::error_knowledge_source_edit_proposed_source_invalid(),
                ),
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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Loss {
    pub path: String,
    pub code: String,
    #[serde(flatten)]
    pub description: ErrorDescription,
    pub product_diagnostics: Vec<ErrorDescription>,
}
