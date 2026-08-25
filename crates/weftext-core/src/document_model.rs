use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::AdjacentHeadingBody;

pub const DOCUMENT_CONTRACT_VERSION: u16 = 2;

pub type DocumentAnalysisStatus = weftext_asciidoc::AnalysisStatus;
pub type DocumentAdjacentHeadingBodyEligibility = weftext_asciidoc::AdjacentHeadingBodyEligibility;
pub type DocumentAdjacentHeadingBodyPresentation =
    weftext_asciidoc::AdjacentHeadingBodyPresentation;
pub type DocumentAdjacentHeadingBodyResolution = weftext_asciidoc::AdjacentHeadingBodyResolution;
pub type DocumentAdjacentHeadingBodyRule = weftext_asciidoc::AdjacentHeadingBodyRule;
pub type DocumentBlockSemantic = weftext_asciidoc::BlockSemantic;
pub type DocumentDegradation = weftext_asciidoc::Degradation;
pub type DocumentEffectCapability = weftext_asciidoc::EffectCapability;
pub type DocumentEffectDecision = weftext_asciidoc::EffectDecision;
pub type DocumentEffectEvidence = weftext_asciidoc::EffectEvidence;
pub type DocumentEffectOrigin = weftext_asciidoc::EffectOrigin;
pub type DocumentInlineSemantic = weftext_asciidoc::InlineSemantic;
pub type DocumentInlineKind = weftext_asciidoc::InlineKind;
pub type DocumentListItem = weftext_asciidoc::ListItem;
pub type DocumentListKind = weftext_asciidoc::ListKind;
pub type DocumentListModel = weftext_asciidoc::ListModel;
pub type DocumentMathNotation = weftext_asciidoc::MathNotation;
pub type DocumentTableCell = weftext_asciidoc::TableCell;
pub type DocumentTableCellStyle = weftext_asciidoc::TableCellStyle;
pub type DocumentTableModel = weftext_asciidoc::TableModel;
pub type DocumentTableRow = weftext_asciidoc::TableRow;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentProfileId {
    AsciiDocV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
// Wire capabilities are intentionally explicit booleans so older callers can
// ignore additive fields without interpreting an enum set.
#[allow(clippy::struct_excessive_bools)]
pub struct DocumentCapabilities {
    pub exact_source: bool,
    pub utf8_source_edits: bool,
    pub yaml_envelope: bool,
    pub max_heading_level: u8,
    pub actual_quote_depth: bool,
    pub block_ids: bool,
    pub managed_links: bool,
    pub protected_regions: bool,
    pub typed_blocks: bool,
    pub typed_inlines: bool,
    pub nested_lists: bool,
    pub typed_tables: bool,
    pub safe_render_input: bool,
    pub degradation_reports: bool,
    pub adjacent_heading_body_resolution: bool,
    pub typed_effect_evidence: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentProfileDescriptor {
    pub contract_version: u16,
    pub profile: DocumentProfileId,
    pub media_type: &'static str,
    pub canonical_extension: &'static str,
    pub capabilities: DocumentCapabilities,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentBlockKind {
    Frontmatter,
    DocumentTitle,
    DocumentSubtitle,
    Heading,
    Paragraph,
    Listing,
    Literal,
    FencedCode,
    Quote,
    List,
    Table,
    Image,
    BlockTitle,
    ThematicBreak,
    Html,
    Math,
    Mermaid,
    Passthrough,
    Comment,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentBlock {
    pub kind: DocumentBlockKind,
    pub start: u64,
    pub end: u64,
    pub text_start: u64,
    pub text_end: u64,
    pub text: String,
    pub heading_level: Option<u8>,
    pub quote_depth: Option<u64>,
    pub block_id: Option<String>,
    pub roles: Vec<String>,
    pub title: Option<String>,
    pub semantic: DocumentBlockSemantic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentDiagnosticCode {
    DuplicateBlockId,
    UnclosedFrontmatter,
    UnclosedFence,
    ProfileWarning,
    UnsafeActiveContent,
    ParserError,
    InvalidDocumentStructure,
    UnsupportedProfileSyntax,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDiagnostic {
    pub code: DocumentDiagnosticCode,
    pub start: u64,
    pub end: u64,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunInGroup {
    pub heading_block: u64,
    pub body_block: u64,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentModel {
    pub semantic_model_version: u16,
    pub status: DocumentAnalysisStatus,
    pub blocks: Vec<DocumentBlock>,
    pub inlines: Vec<DocumentInlineSemantic>,
    pub run_in_groups: Vec<RunInGroup>,
    pub adjacent_heading_bodies: Vec<DocumentAdjacentHeadingBodyResolution>,
    pub effects: Vec<DocumentEffectEvidence>,
    pub diagnostics: Vec<DocumentDiagnostic>,
    pub degradations: Vec<DocumentDegradation>,
    pub safe_html: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentLinkKind {
    Link,
    Embed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLinkOccurrence {
    pub kind: DocumentLinkKind,
    pub start: u64,
    pub end: u64,
    pub locator_start: u64,
    pub locator_end: u64,
    pub authored_locator: String,
    pub fragment: Option<String>,
    pub display_text: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSourceOccurrences {
    pub links: Vec<DocumentLinkOccurrence>,
    pub eligible_text_ranges: Vec<Range<u64>>,
    pub protected_ranges: Vec<Range<u64>>,
}

/// Format-neutral view data derived from a parsed exact-source model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentViewModel {
    pub contract_version: u16,
    pub semantic_model_version: u16,
    pub profile: DocumentProfileId,
    pub capabilities: DocumentCapabilities,
    pub status: DocumentAnalysisStatus,
    pub blocks: Vec<DocumentBlock>,
    pub inlines: Vec<DocumentInlineSemantic>,
    pub run_in_groups: Vec<RunInGroup>,
    pub adjacent_heading_bodies: Vec<DocumentAdjacentHeadingBodyResolution>,
    pub effects: Vec<DocumentEffectEvidence>,
    pub degradations: Vec<DocumentDegradation>,
    pub safe_html: String,
}

/// One complete analysis through the selected runtime document adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DocumentAnalysis {
    pub descriptor: DocumentProfileDescriptor,
    pub model: DocumentModel,
    pub occurrences: DocumentSourceOccurrences,
    pub view: DocumentViewModel,
    pub searchable_text: String,
}

/// Small syntax boundary beneath Core workspace and product contracts.
///
/// This is deliberately not a plugin registry. R1A has exactly one static
/// implementation and performs no runtime loading or format negotiation.
pub trait DocumentFormatAdapter: Sync {
    fn descriptor(&self) -> DocumentProfileDescriptor;
    fn parse(&self, source: &str, setting: AdjacentHeadingBody) -> DocumentModel;
    fn extract_occurrences(&self, source: &str, model: &DocumentModel)
    -> DocumentSourceOccurrences;
    fn searchable_text(&self, source: &str, model: &DocumentModel) -> String;
}

#[must_use]
pub fn active_document_adapter() -> &'static dyn DocumentFormatAdapter {
    &crate::asciidoc_adapter::ASCIIDOC_DOCUMENT_ADAPTER
}

#[must_use]
pub fn active_document_profile() -> DocumentProfileDescriptor {
    active_document_adapter().descriptor()
}

#[must_use]
pub fn analyze_document(source: &str, setting: AdjacentHeadingBody) -> DocumentAnalysis {
    analyze_document_with_adapter(active_document_adapter(), source, setting)
}

#[must_use]
pub fn document_adapter_for_profile(
    profile: DocumentProfileId,
) -> &'static dyn DocumentFormatAdapter {
    match profile {
        DocumentProfileId::AsciiDocV1 => &crate::asciidoc_adapter::ASCIIDOC_DOCUMENT_ADAPTER,
    }
}

#[must_use]
pub fn analyze_document_for_profile(
    profile: DocumentProfileId,
    source: &str,
    setting: AdjacentHeadingBody,
) -> DocumentAnalysis {
    analyze_document_with_adapter(document_adapter_for_profile(profile), source, setting)
}

/// Runs one explicitly supplied document adapter through the same Core model
/// assembly used by the active runtime adapter.
///
/// Explicit import/export adapters use this same contract without becoming a
/// second managed-document authority.
#[must_use]
pub fn analyze_document_with_adapter(
    adapter: &dyn DocumentFormatAdapter,
    source: &str,
    setting: AdjacentHeadingBody,
) -> DocumentAnalysis {
    let descriptor = adapter.descriptor();
    let model = adapter.parse(source, setting);
    let occurrences = adapter.extract_occurrences(source, &model);
    let searchable_text = adapter.searchable_text(source, &model);
    let view = DocumentViewModel {
        contract_version: descriptor.contract_version,
        semantic_model_version: model.semantic_model_version,
        profile: descriptor.profile,
        capabilities: descriptor.capabilities,
        status: model.status,
        blocks: model.blocks.clone(),
        inlines: model.inlines.clone(),
        run_in_groups: model.run_in_groups.clone(),
        adjacent_heading_bodies: model.adjacent_heading_bodies.clone(),
        effects: model.effects.clone(),
        degradations: model.degradations.clone(),
        safe_html: model.safe_html.clone(),
    };
    DocumentAnalysis {
        descriptor,
        model,
        occurrences,
        view,
        searchable_text,
    }
}

#[must_use]
pub fn parse_document(source: &str, setting: AdjacentHeadingBody) -> DocumentModel {
    active_document_adapter().parse(source, setting)
}

#[must_use]
pub fn extract_document_occurrences(
    source: &str,
    model: &DocumentModel,
) -> DocumentSourceOccurrences {
    active_document_adapter().extract_occurrences(source, model)
}

#[must_use]
pub fn searchable_document_text(source: &str, setting: AdjacentHeadingBody) -> String {
    let adapter = active_document_adapter();
    let model = adapter.parse(source, setting);
    adapter.searchable_text(source, &model)
}

#[must_use]
pub fn searchable_document_text_for_profile(
    profile: DocumentProfileId,
    source: &str,
    setting: AdjacentHeadingBody,
) -> String {
    let adapter = document_adapter_for_profile(profile);
    let model = adapter.parse(source, setting);
    adapter.searchable_text(source, &model)
}
