#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fmt::Write as _;
use std::ops::Range;

use asciidork_ast::{
    AttrData, Block as AsciiDocBlock, BlockContent, BlockContext, CellContent, DocContent,
    EmptyMetadata, HorizontalAlignment, Inline, InlineNodes, ListItemTypeMeta,
    ListMarker as AsciiDocListMarker, ListVariant, MultiSourceLocation, Section, SourceLocation,
    VerticalAlignment,
};
use asciidork_core::JobSettings;
use asciidork_parser::Parser;
use asciidork_parser::parser::SourceFile;
use bumpalo::Bump;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PROFILE_ID: &str = "weftext.asciidoc.v1";
pub const SEMANTIC_MODEL_VERSION: u16 = 3;
pub const GENERATION_MARKER_FILE: &str = ".weftext-format";
pub const GENERATION_MARKER_V1: &[u8] = b"weftext.asciidoc.v1\n";

const MAX_DOCUMENT_HEADER_ATTRIBUTES: usize = 256;
const MAX_DOCUMENT_HEADER_ATTRIBUTE_NAME_BYTES: usize = 128;
/// Maximum UTF-8 byte length accepted for one literal document-header attribute value.
pub const MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES: usize = 4_096;
const MAX_DOCUMENT_HEADER_ISSUES: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProbe {
    AsciiDocV1,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationMarkerError {
    Missing,
    Unknown(Vec<u8>),
}

/// Probes the required root generation marker without touching the filesystem.
///
/// # Errors
///
/// Returns an error when the marker is missing or is not the exact accepted v1 bytes.
pub fn probe_generation_marker(
    marker: Option<&[u8]>,
) -> Result<GenerationProbe, GenerationMarkerError> {
    match marker {
        None => Err(GenerationMarkerError::Missing),
        Some(GENERATION_MARKER_V1) => Ok(GenerationProbe::AsciiDocV1),
        Some(bytes) => Err(GenerationMarkerError::Unknown(bytes.to_vec())),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockKind {
    Frontmatter,
    DocumentTitle,
    DocumentSubtitle,
    Heading,
    Paragraph,
    Listing,
    Literal,
    Quote,
    List,
    Table,
    Image,
    BlockTitle,
    Math,
    Mermaid,
    Passthrough,
    Comment,
    ThematicBreak,
    Other,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisStatus {
    #[default]
    Complete,
    Degraded,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportState {
    Full,
    Constrained,
    PreserveOnly,
    ProhibitedEffect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationKind {
    ParserFailure,
    InvalidStructure,
    UnsupportedBlock,
    UnsupportedInline,
    DisabledInclude,
    DisabledRemoteUri,
    DisabledPassthrough,
    ConstrainedMath,
    ConstrainedMermaid,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderFallback {
    EscapedSource,
    DisabledEffect,
    NoDerivedRendering,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Degradation {
    pub kind: DegradationKind,
    pub support_state: SupportState,
    pub range: Range<u64>,
    pub fallback: RenderFallback,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    Ordered,
    Unordered,
    Description,
    Callout,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub range: Range<u64>,
    pub text_range: Range<u64>,
    pub marker: String,
    pub text: String,
    pub depth: u8,
    pub checked: Option<bool>,
    pub children: Vec<ListItem>,
    pub unmodeled_continuations: Vec<Range<u64>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListModel {
    pub kind: ListKind,
    pub depth: u8,
    pub items: Vec<ListItem>,
}

/// Exact native spelling authored for one parser-confirmed unordered checklist marker.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistMarker {
    Open,
    CheckedX,
    CheckedStar,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistState {
    Todo,
    Completed,
}

/// One exact source edit in a parser-confirmed checklist branch lift recipe.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChecklistBranchLiftEdit {
    pub range: Range<u64>,
    pub replacement: String,
    pub kind: ChecklistBranchLiftEditKind,
}

/// Closed reason for an exact checklist branch lift edit.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case", tag = "type")]
pub enum ChecklistBranchLiftEditKind {
    OmitPrincipal,
    RemoveContinuationConnector,
    DedentDescendant { from_depth: u8, to_depth: u8 },
}

/// Parser-owned recipe for replacing a checklist branch and lifting its attached body.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChecklistPromotionBranchEvidence {
    pub source_replacement_range: Range<u64>,
    pub lift_edits: Vec<ChecklistBranchLiftEdit>,
    pub lifted_descendant_count: u32,
    pub lifted_continuation_count: u32,
    pub context_dependencies: Vec<ChecklistPromotionContextDependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChecklistPromotionContextDependency {
    pub kind: ChecklistPromotionContextDependencyKind,
    pub range: Range<u64>,
    pub target: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChecklistPromotionContextDependencyKind {
    RelativeLocator,
    ExplicitAnchor,
    ImplicitHeadingAnchor,
    NamedFootnote,
    NamedEndnote,
    DocumentAttributeReference,
    ConditionalDirective,
}

impl ChecklistPromotionBranchEvidence {
    /// Applies the parser-owned recipe to the captured source branch.
    ///
    /// This pure applicator does not authenticate caller-constructed evidence. Mutation planners
    /// must stale-check the document revision, re-analyze the current source, and exactly match the
    /// complete parser occurrence before using the returned body.
    #[must_use]
    pub fn destination_body(&self, source: &str) -> Option<String> {
        apply_checklist_lift_recipe(source, self)
    }
}

/// Parser identity and complete-branch evidence for one checklist occurrence.
///
/// The ordinal path alternates list and item ordinals, beginning with the root list ordinal in
/// parser traversal order. `branch_range` remains as a compatibility alias for
/// `promotion_branch.source_replacement_range`. `branch_complete` is true exactly when
/// `promotion_branch` is present.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistParserOccurrence {
    pub parser_ordinal_path: Vec<u32>,
    pub branch_range: Option<Range<u64>>,
    pub branch_complete: bool,
    pub promotion_branch: Option<ChecklistPromotionBranchEvidence>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct ChecklistParserOccurrenceSerde {
    parser_ordinal_path: Vec<u32>,
    branch_range: Option<Range<u64>>,
    branch_complete: bool,
    promotion_branch: Option<ChecklistPromotionBranchEvidence>,
}

impl<'de> Deserialize<'de> for ChecklistParserOccurrence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = ChecklistParserOccurrenceSerde::deserialize(deserializer)?;
        let consistent = match (&value.branch_range, &value.promotion_branch) {
            (None, None) => !value.branch_complete,
            (Some(branch), Some(promotion)) => {
                value.branch_complete && *branch == promotion.source_replacement_range
            }
            _ => false,
        };
        if !consistent {
            return Err(serde::de::Error::custom(
                "branchComplete, branchRange, and promotionBranch are inconsistent",
            ));
        }
        Ok(Self {
            parser_ordinal_path: value.parser_ordinal_path,
            branch_range: value.branch_range,
            branch_complete: value.branch_complete,
            promotion_branch: value.promotion_branch,
        })
    }
}

/// Lossless source evidence projected only from the native unordered-list parser branch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistEvidence {
    pub authored_marker: ChecklistMarker,
    pub state: ChecklistState,
    pub item_range: Range<u64>,
    pub marker_range: Range<u64>,
    pub description_range: Range<u64>,
    pub description: String,
    pub list_depth: u8,
    pub parser_occurrence: ChecklistParserOccurrence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TableCellStyle {
    AsciiDoc,
    Default,
    Emphasis,
    Header,
    Literal,
    Monospace,
    Strong,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalCellAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalCellAlignment {
    Top,
    Middle,
    Bottom,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    pub text: String,
    pub column_span: u8,
    pub row_span: u8,
    pub style: TableCellStyle,
    pub horizontal_alignment: HorizontalCellAlignment,
    pub vertical_alignment: VerticalCellAlignment,
    pub nested_asciidoc: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableModel {
    pub header: Option<TableRow>,
    pub body: Vec<TableRow>,
    pub footer: Option<TableRow>,
    pub column_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MathNotation {
    AsciiMath,
    LatexMath,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BlockSemantic {
    Frontmatter,
    DocumentTitle,
    DocumentSubtitle,
    Heading {
        level: u8,
    },
    Paragraph,
    Listing {
        language: Option<String>,
    },
    Literal,
    Quote {
        depth: Option<u64>,
        attribution: Option<String>,
        citation: Option<String>,
    },
    List {
        model: ListModel,
    },
    Table {
        model: TableModel,
    },
    Image {
        target: String,
        alt: Option<String>,
    },
    BlockTitle,
    Math {
        notation: MathNotation,
    },
    Mermaid,
    Passthrough,
    Comment,
    ThematicBreak,
    Unsupported {
        context: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InlineKind {
    Anchor,
    Bold,
    Italic,
    Monospace,
    Highlight,
    Superscript,
    Subscript,
    Quoted,
    RoleSpan,
    Passthrough,
    Unsupported,
    Xref,
    NativeLink,
    Image,
    Footnote,
    Endnote,
    Stem,
    LatexMath,
    Node,
    NodeEmbed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineSemantic {
    pub kind: InlineKind,
    pub range: Range<u64>,
    pub target_range: Option<Range<u64>>,
    pub label_range: Option<Range<u64>>,
    pub target: Option<String>,
    pub fragment: Option<String>,
    pub text: Option<String>,
    pub notation: Option<MathNotation>,
    pub roles: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub kind: BlockKind,
    pub range: Range<u64>,
    pub text_range: Range<u64>,
    pub text: String,
    pub heading_level: Option<u8>,
    pub quote_depth: Option<u64>,
    pub block_id: Option<String>,
    pub roles: Vec<String>,
    pub title: Option<String>,
    pub semantic: BlockSemantic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    UnclosedFrontmatter,
    AdditionalDocumentTitle,
    ParserWarning,
    ParserError,
    UnsafeInclude,
    ConditionalDisabled,
    ProcessorEffectDisabled,
    RemoteUri,
    PassthroughDisabled,
    QuoteSyntaxUnresolved,
    InvalidNodeLink,
    UnsupportedProfileSyntax,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub range: Range<u64>,
    pub message: String,
}

/// Root-level portable default used when a heading has neither Weftext presentation role.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacentHeadingBodyDefault {
    RunIn,
    #[default]
    Separate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacentHeadingBodyPresentation {
    RunIn,
    Separate,
}

/// Exact rule which selected an adjacent-heading/body presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacentHeadingBodyRule {
    ExplicitRunInRole,
    ExplicitSeparateRole,
    WorkspaceRunInDefault,
    WorkspaceSeparateDefault,
}

/// Why the following semantic block can or cannot participate in run-in presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AdjacentHeadingBodyEligibility {
    Eligible,
    NoFollowingBlock,
    FollowingBlockIsNotParagraph,
    NonWhitespaceSourceGap,
    NotOnImmediatelyFollowingPhysicalLine,
}

/// One typed, non-merging presentation decision for a body H1-H9.
///
/// Block indexes address `Analysis::blocks`. A present `body_block` always identifies an ordinary
/// paragraph. The heading and paragraph retain independent exact ranges and semantics.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdjacentHeadingBodyResolution {
    pub heading_block: u64,
    pub body_block: Option<u64>,
    pub presentation: AdjacentHeadingBodyPresentation,
    pub rule: AdjacentHeadingBodyRule,
    pub eligibility: AdjacentHeadingBodyEligibility,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectCapability {
    IncludeExpansion,
    WorkspaceRead,
    NetworkRead,
    ConditionalEvaluation,
    ProcessorExecution,
    PassthroughRendering,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectOrigin {
    IncludeDirective,
    ConditionalDirective,
    DocumentHeaderAttribute,
    BlockPassthrough,
    InlinePassthrough,
    ImageResource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectDecision {
    /// The declaration is recorded as processor state, but grants no executable capability.
    PreservedInert,
    /// The requested effect is denied at the v1 Core/renderer capability boundary.
    Denied,
}

/// Exact typed evidence for syntax which would require an active capability.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectEvidence {
    pub origin: EffectOrigin,
    pub required_capability: EffectCapability,
    pub decision: EffectDecision,
    pub range: Range<u64>,
    pub target: Option<String>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeFieldKind {
    Id,
    Icon,
    Aliases,
    ChildSort,
    ChildSortDirection,
    SiblingRank,
    AdjacentHeadingBody,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeListItem {
    pub range: Range<u64>,
    pub value_range: Range<u64>,
    pub value: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EnvelopeFieldValue {
    Scalar { value: String },
    StringList { items: Vec<EnvelopeListItem> },
    Opaque,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeField {
    pub kind: EnvelopeFieldKind,
    pub name: String,
    pub range: Range<u64>,
    pub key_range: Range<u64>,
    pub value_range: Range<u64>,
    pub value: EnvelopeFieldValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeIssueSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeIssueCode {
    MissingWeftextMapping,
    MissingRequiredField,
    LegacyTopLevelKey,
    UnknownTopLevelKey,
    DuplicateTopLevelKey,
    DuplicateField,
    InvalidStructure,
    InvalidValue,
    UnsafeYamlFeature,
    UnknownWeftextField,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeIssue {
    pub code: EnvelopeIssueCode,
    pub severity: EnvelopeIssueSeverity,
    pub range: Range<u64>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeSemantic {
    pub range: Range<u64>,
    pub content_range: Range<u64>,
    pub weftext_range: Option<Range<u64>>,
    pub weftext_key_range: Option<Range<u64>>,
    pub fields: Vec<EnvelopeField>,
    pub issues: Vec<EnvelopeIssue>,
    pub valid: bool,
}

/// Delimiter-only state for the leading managed-document YAML envelope.
///
/// This is intentionally owned by the Profile crate so inventory, document reads, parsers, and
/// narrow metadata edits cannot drift on byte-order marks, line endings, or delimiter ranges.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeProbeState {
    Absent,
    Closed,
    Unclosed,
}

/// Exact UTF-8 ranges for the optional leading YAML envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopeProbe {
    pub state: EnvelopeProbeState,
    pub range: Option<Range<u64>>,
    pub content_range: Option<Range<u64>>,
    pub body_start: u64,
}

/// Profile-owned delimiter and semantic result for a managed envelope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedEnvelopeAnalysis {
    pub probe: EnvelopeProbe,
    pub semantic: Option<EnvelopeSemantic>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeChildSort {
    Name,
    Manual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeChildSortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeAdjacentHeadingBody {
    RunIn,
    Separate,
}

/// One typed, narrow edit to the canonical `weftext` mapping.
///
/// A `None` optional value removes that field. `id` is deliberately not optional because a
/// managed document must retain identity. Every edit reparses the result through the same Profile
/// authority before returning it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManagedEnvelopePatch {
    Id(Uuid),
    Icon(Option<String>),
    Aliases(Vec<String>),
    ChildSort(Option<EnvelopeChildSort>),
    ChildSortDirection(Option<EnvelopeChildSortDirection>),
    SiblingRank(Option<u64>),
    AdjacentHeadingBody(Option<EnvelopeAdjacentHeadingBody>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManagedEnvelopePatchError {
    MissingEnvelope,
    UnclosedEnvelope,
    InvalidEnvelope,
    InvalidValue,
    UnsupportedRange,
}

impl fmt::Display for ManagedEnvelopePatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingEnvelope => "document YAML envelope is missing",
            Self::UnclosedEnvelope => "document YAML envelope is not closed",
            Self::InvalidEnvelope => "document YAML envelope is not canonical",
            Self::InvalidValue => "managed envelope patch value is invalid",
            Self::UnsupportedRange => "managed envelope patch range is invalid",
        })
    }
}

impl std::error::Error for ManagedEnvelopePatchError {}

/// Weftext's interpretation of one authored document-header attribute entry.
///
/// This classification never expands the value or performs the effect of a processor-control
/// attribute. It exists so every caller can distinguish stable note properties from `AsciiDoc`
/// processing state without implementing another header parser.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentHeaderAttributeKind {
    Descriptive,
    Custom,
    ProcessorControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentHeaderAttributeForm {
    Set,
    Unset,
}

/// Exact source evidence for one attribute entry in the `AsciiDoc` document header.
///
/// `range` covers every physical line owned by the entry, including the final line ending when
/// present. `value_range` is the exact, unexpanded authored value span. For a continued value it
/// intentionally spans the continuation markers and intervening line endings; such an entry is
/// never projected as a stable Weftext property. `continuation_ranges` identifies every physical
/// continuation line after the declaration line.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentHeaderAttribute {
    pub name: String,
    pub literal_value: Option<String>,
    pub kind: DocumentHeaderAttributeKind,
    pub form: DocumentHeaderAttributeForm,
    pub range: Range<u64>,
    pub name_range: Range<u64>,
    pub value_range: Range<u64>,
    pub continuation_ranges: Vec<Range<u64>>,
    pub projected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentHeaderIssueCode {
    ParserFailure,
    UnclosedEnvelope,
    InvalidName,
    DuplicateName,
    UnsupportedUnset,
    ContinuedValue,
    ValueTooLarge,
    AttributeLimitExceeded,
    ProcessorControl,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentHeaderIssue {
    pub code: DocumentHeaderIssueCode,
    pub range: Range<u64>,
    pub name: Option<String>,
    pub message: String,
}

/// Lossless, bounded projection of the `AsciiDoc` document header.
///
/// Only entries with `projected == true` are stable Weftext Properties. All other entries retain
/// exact source evidence and a diagnostic. `insertion_offset` is the profile-owned narrow-patch
/// location and never points into the YAML envelope or document body.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentHeaderSemantic {
    pub range: Range<u64>,
    pub insertion_offset: u64,
    pub attributes: Vec<DocumentHeaderAttribute>,
    pub issues: Vec<DocumentHeaderIssue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DocumentHeaderPatchError {
    InvalidName,
    InvalidValue,
    DuplicateName,
    UnclosedEnvelope,
    UnsupportedHeader,
}

impl fmt::Display for DocumentHeaderPatchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidName => "document property name is invalid or processor-owned",
            Self::InvalidValue => "document property value must be one bounded literal line",
            Self::DuplicateName => "document property is duplicated",
            Self::UnclosedEnvelope => "document YAML envelope is not closed",
            Self::UnsupportedHeader => {
                "document property cannot be patched through unsupported header syntax"
            }
        })
    }
}

impl std::error::Error for DocumentHeaderPatchError {}

/// Failure to encode or decode a canonical managed-node link display label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeLinkLabelCodecError {
    ProhibitedCharacter { byte_offset: usize },
    UnescapedReservedCharacter { byte_offset: usize, character: char },
    UnknownEscape { byte_offset: usize, character: char },
    TrailingEscape { byte_offset: usize },
}

impl fmt::Display for NodeLinkLabelCodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProhibitedCharacter { byte_offset } => write!(
                formatter,
                "node link label contains a prohibited character at byte {byte_offset}"
            ),
            Self::UnescapedReservedCharacter {
                byte_offset,
                character,
            } => write!(
                formatter,
                "node link label character {character:?} must be escaped at byte {byte_offset}"
            ),
            Self::UnknownEscape {
                byte_offset,
                character,
            } => write!(
                formatter,
                "node link label uses unknown escape \\{character} at byte {byte_offset}"
            ),
            Self::TrailingEscape { byte_offset } => write!(
                formatter,
                "node link label ends with an incomplete escape at byte {byte_offset}"
            ),
        }
    }
}

impl std::error::Error for NodeLinkLabelCodecError {}

/// Encodes authored display text for the bracketed label of `node:` and `node::` macros.
///
/// The result is canonical: backslash, brackets, colon, comma, and double quote are escaped with
/// one leading backslash. Line breaks, C0/C1 controls, and bidi formatting controls are rejected.
///
/// # Errors
///
/// Returns [`NodeLinkLabelCodecError::ProhibitedCharacter`] when the authored display contains a
/// character which cannot be represented in a managed-node label.
pub fn encode_node_link_label(label: &str) -> Result<String, NodeLinkLabelCodecError> {
    let mut encoded = String::with_capacity(label.len());
    for (byte_offset, character) in label.char_indices() {
        if prohibited_node_link_label_character(character) {
            return Err(NodeLinkLabelCodecError::ProhibitedCharacter { byte_offset });
        }
        if reserved_node_link_label_character(character) {
            encoded.push('\\');
        }
        encoded.push(character);
    }
    Ok(encoded)
}

/// Decodes one canonical bracketed `node:`/`node::` label payload, excluding the brackets.
///
/// Unknown escapes and unescaped reserved characters are rejected rather than normalized.
///
/// # Errors
///
/// Returns [`NodeLinkLabelCodecError`] when the encoded payload contains a prohibited character,
/// an unescaped reserved character, an unknown escape, or an incomplete trailing escape.
pub fn decode_node_link_label(encoded: &str) -> Result<String, NodeLinkLabelCodecError> {
    let mut decoded = String::with_capacity(encoded.len());
    let mut characters = encoded.char_indices();
    while let Some((byte_offset, character)) = characters.next() {
        if character == '\\' {
            let Some((escaped_offset, escaped)) = characters.next() else {
                return Err(NodeLinkLabelCodecError::TrailingEscape { byte_offset });
            };
            if prohibited_node_link_label_character(escaped) {
                return Err(NodeLinkLabelCodecError::ProhibitedCharacter {
                    byte_offset: escaped_offset,
                });
            }
            if !reserved_node_link_label_character(escaped) {
                return Err(NodeLinkLabelCodecError::UnknownEscape {
                    byte_offset,
                    character: escaped,
                });
            }
            decoded.push(escaped);
            continue;
        }
        if prohibited_node_link_label_character(character) {
            return Err(NodeLinkLabelCodecError::ProhibitedCharacter { byte_offset });
        }
        if reserved_node_link_label_character(character) {
            return Err(NodeLinkLabelCodecError::UnescapedReservedCharacter {
                byte_offset,
                character,
            });
        }
        decoded.push(character);
    }
    Ok(decoded)
}

const fn reserved_node_link_label_character(character: char) -> bool {
    matches!(character, '\\' | '[' | ']' | ':' | ',' | '"')
}

const fn prohibited_node_link_label_character(character: char) -> bool {
    matches!(
        character,
        '\u{0000}'..='\u{001f}'
            | '\u{007f}'..='\u{009f}'
            | '\u{061c}'
            | '\u{200e}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{206f}'
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    Node,
    NodeEmbed,
    Xref,
    Footnote,
    Endnote,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkOccurrence {
    pub kind: LinkKind,
    pub range: Range<u64>,
    pub target_range: Range<u64>,
    pub label_range: Range<u64>,
    pub target: String,
    pub fragment: Option<String>,
    pub display: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::struct_excessive_bools)]
pub struct ProfileCapabilities {
    pub exact_source: bool,
    pub utf8_source_edits: bool,
    pub yaml_envelope: bool,
    pub max_heading_level: u8,
    pub protected_regions: bool,
    pub safe_derived_html: bool,
    pub includes_enabled: bool,
    pub conditional_evaluation_enabled: bool,
    pub processor_execution_enabled: bool,
    pub remote_uris_enabled: bool,
    pub passthroughs_enabled: bool,
}

pub const PROFILE_CAPABILITIES: ProfileCapabilities = ProfileCapabilities {
    exact_source: true,
    utf8_source_edits: true,
    yaml_envelope: true,
    max_heading_level: 9,
    protected_regions: true,
    safe_derived_html: true,
    includes_enabled: false,
    conditional_evaluation_enabled: false,
    processor_execution_enabled: false,
    remote_uris_enabled: false,
    passthroughs_enabled: false,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub profile: &'static str,
    pub semantic_model_version: u16,
    pub status: AnalysisStatus,
    pub capabilities: ProfileCapabilities,
    pub envelope: Option<EnvelopeSemantic>,
    pub document_header: DocumentHeaderSemantic,
    pub checklists: Vec<ChecklistEvidence>,
    pub blocks: Vec<Block>,
    pub links: Vec<LinkOccurrence>,
    pub inlines: Vec<InlineSemantic>,
    pub adjacent_heading_bodies: Vec<AdjacentHeadingBodyResolution>,
    pub effects: Vec<EffectEvidence>,
    pub protected_ranges: Vec<Range<u64>>,
    pub diagnostics: Vec<Diagnostic>,
    pub degradations: Vec<Degradation>,
    pub searchable_text: String,
    pub safe_html: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceEdit {
    pub range: Range<usize>,
    pub replacement: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditPlanError {
    InvalidRange(Range<usize>),
    NotUtf8Boundary(Range<usize>),
    Overlap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceEditPlan {
    source_len: usize,
    edits: Vec<SourceEdit>,
}

impl SourceEditPlan {
    /// Validates and orders narrow exact-source edits.
    ///
    /// # Errors
    ///
    /// Returns an error when a range is outside the source, cuts through a
    /// UTF-8 code point, or overlaps another edit.
    pub fn new(source: &str, mut edits: Vec<SourceEdit>) -> Result<Self, EditPlanError> {
        edits.sort_by_key(|edit| (edit.range.start, edit.range.end));
        let mut previous_end = 0;
        for (index, edit) in edits.iter().enumerate() {
            if edit.range.start > edit.range.end || edit.range.end > source.len() {
                return Err(EditPlanError::InvalidRange(edit.range.clone()));
            }
            if !source.is_char_boundary(edit.range.start)
                || !source.is_char_boundary(edit.range.end)
            {
                return Err(EditPlanError::NotUtf8Boundary(edit.range.clone()));
            }
            if index > 0 && edit.range.start < previous_end {
                return Err(EditPlanError::Overlap);
            }
            previous_end = edit.range.end;
        }
        Ok(Self {
            source_len: source.len(),
            edits,
        })
    }

    #[must_use]
    pub fn apply(&self, source: &str) -> Option<String> {
        if source.len() != self.source_len {
            return None;
        }
        let mut next = source.to_owned();
        for edit in self.edits.iter().rev() {
            next.replace_range(edit.range.clone(), &edit.replacement);
        }
        Some(next)
    }
}

/// Analyzes only the `AsciiDoc` document header and returns exact, unexpanded attribute evidence.
///
/// This is the sole syntax authority for Weftext's stable Properties projection. The analysis is
/// bounded by the first document-body construct, records processor state without executing it, and
/// treats every offset as a UTF-8 byte offset into `source`.
#[must_use]
pub fn analyze_document_header(source: &str) -> DocumentHeaderSemantic {
    std::panic::catch_unwind(|| analyze_document_header_inner(source)).unwrap_or_else(|_| {
        DocumentHeaderSemantic {
            range: 0..0,
            insertion_offset: 0,
            attributes: Vec::new(),
            issues: vec![DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::ParserFailure,
                range: 0..source.len() as u64,
                name: None,
                message: "AsciiDoc document-header analysis aborted; no property mutation is safe"
                    .to_owned(),
            }],
        }
    })
}

/// Narrowly sets or removes one stable Weftext document property.
///
/// # Errors
///
/// Returns an error for processor-control names, unbounded values, ambiguous duplicate targets,
/// an unclosed YAML envelope, or a target expressed through unset/continuation syntax.
pub fn patch_document_header_attribute(
    source: &str,
    name: &str,
    value: Option<&str>,
) -> Result<String, DocumentHeaderPatchError> {
    let Some(edit) = plan_document_header_attribute_patch(source, name, value)? else {
        return Ok(source.to_owned());
    };
    SourceEditPlan::new(source, vec![edit])
        .map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?
        .apply(source)
        .ok_or(DocumentHeaderPatchError::UnsupportedHeader)
}

/// Plans at most one exact edit that sets or removes a stable document-header attribute.
/// Returning `None` means the decoded literal already equals the requested canonical value, or
/// the requested removal is already satisfied. The proposed edit is applied and the complete
/// header is revalidated before it is returned.
///
/// # Errors
///
/// Returns an error for processor-control names, unbounded values, ambiguous duplicate targets,
/// an unclosed YAML envelope, or a target expressed through unset/continuation syntax.
#[allow(clippy::too_many_lines)]
pub fn plan_document_header_attribute_patch(
    source: &str,
    name: &str,
    value: Option<&str>,
) -> Result<Option<SourceEdit>, DocumentHeaderPatchError> {
    if !valid_document_property_name(name) || document_header_attribute_is_processor_control(name) {
        return Err(DocumentHeaderPatchError::InvalidName);
    }
    let header = analyze_document_header(source);
    if header
        .issues
        .iter()
        .any(|issue| issue.code == DocumentHeaderIssueCode::UnclosedEnvelope)
    {
        return Err(DocumentHeaderPatchError::UnclosedEnvelope);
    }
    if header.issues.iter().any(|issue| {
        matches!(
            issue.code,
            DocumentHeaderIssueCode::ParserFailure
                | DocumentHeaderIssueCode::AttributeLimitExceeded
        )
    }) {
        return Err(DocumentHeaderPatchError::UnsupportedHeader);
    }

    let matches = header
        .attributes
        .iter()
        .filter(|attribute| attribute.name == name)
        .collect::<Vec<_>>();
    if matches.len() > 1 {
        return Err(DocumentHeaderPatchError::DuplicateName);
    }
    let encoded = value.map(validate_document_property_value).transpose()?;
    if let Some(attribute) = matches.first() {
        if !attribute.projected
            || attribute.form != DocumentHeaderAttributeForm::Set
            || !attribute.continuation_ranges.is_empty()
        {
            return Err(DocumentHeaderPatchError::UnsupportedHeader);
        }
        if encoded.is_some() && attribute.literal_value.as_deref() == encoded {
            validate_document_header_patch(source, name, encoded, header.range.end)?;
            return Ok(None);
        }
        let range = if encoded.is_some() {
            u64_range_to_usize(&attribute.value_range, source.len())?
        } else {
            u64_range_to_usize(&attribute.range, source.len())?
        };
        let replacement = encoded.unwrap_or_default().to_owned();
        let expected_header_end = adjusted_header_end(
            header.range.end,
            range.end.saturating_sub(range.start),
            replacement.len(),
        )?;
        let edit = SourceEdit { range, replacement };
        let plan = SourceEditPlan::new(source, vec![edit.clone()])
            .map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
        let patched = plan
            .apply(source)
            .ok_or(DocumentHeaderPatchError::UnsupportedHeader)?;
        validate_document_header_patch(&patched, name, encoded, expected_header_end)?;
        return Ok(Some(edit));
    }
    let Some(encoded) = encoded else {
        validate_document_header_patch(source, name, None, header.range.end)?;
        return Ok(None);
    };

    let insert_at = usize::try_from(header.insertion_offset)
        .ok()
        .filter(|offset| *offset <= source.len() && source.is_char_boundary(*offset))
        .ok_or(DocumentHeaderPatchError::UnsupportedHeader)?;
    let line_ending = preferred_document_line_ending(source);
    let mut insertion = String::new();
    if insert_at > 0
        && !source
            .get(..insert_at)
            .is_some_and(|prefix| prefix.ends_with(['\n', '\r']))
    {
        insertion.push_str(line_ending);
    }
    let _ = write!(insertion, ":{name}: {encoded}{line_ending}");
    let body_separator_len = if source
        .get(insert_at..)
        .is_some_and(|tail| !tail.is_empty() && !tail.starts_with(['\n', '\r']))
    {
        insertion.push_str(line_ending);
        line_ending.len()
    } else {
        0
    };
    let expected_header_end = adjusted_header_end(
        header.range.end,
        0,
        insertion.len().saturating_sub(body_separator_len),
    )?;
    let edit = SourceEdit {
        range: insert_at..insert_at,
        replacement: insertion,
    };
    let plan = SourceEditPlan::new(source, vec![edit.clone()])
        .map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
    let patched = plan
        .apply(source)
        .ok_or(DocumentHeaderPatchError::UnsupportedHeader)?;
    validate_document_header_patch(&patched, name, Some(encoded), expected_header_end)?;
    Ok(Some(edit))
}

fn analyze_document_header_inner(source: &str) -> DocumentHeaderSemantic {
    let mut blocks = Vec::new();
    let mut protected = Vec::new();
    let mut diagnostics = Vec::new();
    let (body_start, _) = probe_envelope(source, &mut blocks, &mut protected, &mut diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnclosedFrontmatter)
    {
        return DocumentHeaderSemantic {
            range: source.len() as u64..source.len() as u64,
            insertion_offset: source.len() as u64,
            attributes: Vec::new(),
            issues: vec![DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::UnclosedEnvelope,
                range: 0..source.len() as u64,
                name: None,
                message: "the leading Weftext YAML envelope is not closed".to_owned(),
            }],
        };
    }
    analyze_document_header_from(source, body_start)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocumentHeaderPhase {
    BeforeTitle,
    AfterTitle,
}

#[allow(clippy::too_many_lines)]
fn analyze_document_header_from(source: &str, body_start: usize) -> DocumentHeaderSemantic {
    let source_lines = lines(source)
        .filter(|line| line.start >= body_start)
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    let mut attributes = Vec::new();
    let mut seen = BTreeMap::<String, u64>::new();
    let mut index = 0;
    let mut phase = DocumentHeaderPhase::BeforeTitle;
    let mut native_header_lines = 0_u8;
    let mut after_title_attributes = false;
    let mut has_header_semantics = false;
    let mut header_end = body_start;
    let mut header_attribute_entries = 0_usize;
    let mut limit_reported = false;

    while let Some(line) = source_lines.get(index) {
        if line.text.is_empty() {
            if phase == DocumentHeaderPhase::BeforeTitle
                && prefix_has_later_header_construct(&source_lines, index + 1)
            {
                header_end = line.full_end;
                index += 1;
                continue;
            }
            break;
        }

        if line.text == "////" {
            let (next, end) = consume_header_comment_block(&source_lines, index);
            if phase == DocumentHeaderPhase::AfterTitle {
                after_title_attributes = true;
            }
            header_end = end;
            index = next;
            continue;
        }
        if line.text.starts_with("//") {
            if phase == DocumentHeaderPhase::AfterTitle {
                after_title_attributes = true;
            }
            header_end = line.full_end;
            index += 1;
            continue;
        }

        if phase == DocumentHeaderPhase::BeforeTitle && is_document_title_metadata(line.text) {
            if source_lines
                .get(index + 1)
                .is_some_and(|next| is_document_title_line(next.text))
            {
                header_end = line.full_end;
                index += 1;
                continue;
            }
            break;
        }
        if phase == DocumentHeaderPhase::BeforeTitle && is_document_title_line(line.text) {
            has_header_semantics = true;
            phase = DocumentHeaderPhase::AfterTitle;
            header_end = line.full_end;
            index += 1;
            continue;
        }

        if line.text.starts_with(':') {
            header_attribute_entries = header_attribute_entries.saturating_add(1);
            let over_limit = header_attribute_entries > MAX_DOCUMENT_HEADER_ATTRIBUTES;
            let mut discarded_issues = Vec::new();
            let issue_sink = if over_limit {
                &mut discarded_issues
            } else {
                &mut issues
            };
            let (attribute, next_index) =
                parse_document_header_attribute(source, &source_lines, index, issue_sink);
            has_header_semantics = true;
            header_end = source_lines
                .get(next_index.saturating_sub(1))
                .map_or(line.full_end, |last| last.full_end);
            if phase == DocumentHeaderPhase::AfterTitle {
                after_title_attributes = true;
            }
            index = next_index;

            if over_limit {
                if !limit_reported {
                    let range = attribute.as_ref().map_or_else(
                        || as_u64_range(line.start..line.full_end),
                        |entry| entry.range.clone(),
                    );
                    push_document_header_issue(
                        &mut issues,
                        DocumentHeaderIssue {
                            code: DocumentHeaderIssueCode::AttributeLimitExceeded,
                            range,
                            name: attribute.as_ref().map(|entry| entry.name.clone()),
                            message: format!(
                                "document header exceeds the {MAX_DOCUMENT_HEADER_ATTRIBUTES}-attribute limit"
                            ),
                        },
                    );
                    limit_reported = true;
                }
                continue;
            }
            let Some(mut attribute) = attribute else {
                continue;
            };
            if let Some(first_start) = seen.get(&attribute.name) {
                push_document_header_issue(
                    &mut issues,
                    DocumentHeaderIssue {
                        code: DocumentHeaderIssueCode::DuplicateName,
                        range: attribute.range.clone(),
                        name: Some(attribute.name.clone()),
                        message: format!(
                            "document-header attribute `{}` is duplicated; the first declaration starts at byte {first_start}",
                            attribute.name
                        ),
                    },
                );
                attribute.projected = false;
            } else {
                seen.insert(attribute.name.clone(), attribute.range.start);
            }
            attributes.push(attribute);
            continue;
        }

        if phase == DocumentHeaderPhase::AfterTitle
            && !after_title_attributes
            && native_header_lines < 2
            && is_native_document_header_line(line.text)
        {
            native_header_lines += 1;
            header_end = line.full_end;
            index += 1;
            continue;
        }
        break;
    }

    if !has_header_semantics {
        header_end = body_start;
    }
    DocumentHeaderSemantic {
        range: body_start as u64..header_end as u64,
        insertion_offset: header_end as u64,
        attributes,
        issues,
    }
}

#[allow(clippy::too_many_lines)]
fn parse_document_header_attribute(
    source: &str,
    source_lines: &[Line<'_>],
    index: usize,
    issues: &mut Vec<DocumentHeaderIssue>,
) -> (Option<DocumentHeaderAttribute>, usize) {
    let line = &source_lines[index];
    let Some(relative_colon) = line.text.get(1..).and_then(|tail| tail.find(':')) else {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::InvalidName,
                range: as_u64_range(line.start..line.end),
                name: None,
                message: "attribute-looking document-header line has no closing name delimiter"
                    .to_owned(),
            },
        );
        return (None, index + 1);
    };
    let colon = relative_colon + 1;
    let authored_name = &line.text[1..colon];
    let leading_unset = authored_name.starts_with('!');
    let trailing_unset = authored_name.ends_with('!');
    let form = if leading_unset || trailing_unset {
        DocumentHeaderAttributeForm::Unset
    } else {
        DocumentHeaderAttributeForm::Set
    };
    let name = authored_name
        .strip_prefix('!')
        .unwrap_or(authored_name)
        .strip_suffix('!')
        .unwrap_or_else(|| authored_name.strip_prefix('!').unwrap_or(authored_name));
    let name_start = line.start + 1 + usize::from(leading_unset);
    let name_end = name_start + name.len();
    let name_range = name_start..name_end;
    let valid_name = !leading_unset || !trailing_unset;
    let valid_name = valid_name && valid_document_property_name(name);

    if !valid_name {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::InvalidName,
                range: as_u64_range(name_range.clone()),
                name: Some(name.to_owned()),
                message:
                    "document-header property names must be bounded lowercase ASCII identifiers"
                        .to_owned(),
            },
        );
    }

    let raw_value_start = line.start + colon + 1;
    let raw_value = &line.text[colon + 1..];
    let leading_space_bytes = raw_value
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let trailing_space_bytes = raw_value
        .bytes()
        .rev()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    let whitespace_only = leading_space_bytes == raw_value.len();
    let value_start = if whitespace_only {
        line.end
    } else {
        raw_value_start + leading_space_bytes
    };
    let first_value_end = if whitespace_only {
        line.end
    } else {
        line.end.saturating_sub(trailing_space_bytes)
    };
    let mut value_end = first_value_end;
    let mut entry_end = line.full_end;
    let mut continuation_ranges = Vec::new();
    let mut next_index = index + 1;
    let mut continued = line
        .text
        .get(colon + 1 + leading_space_bytes..)
        .is_some_and(|value| value.ends_with(" \\"));
    while continued {
        let Some(next) = source_lines.get(next_index) else {
            break;
        };
        if next.text.is_empty() || next.start != entry_end {
            break;
        }
        continuation_ranges.push(as_u64_range(next.start..next.full_end));
        value_end = next.end;
        entry_end = next.full_end;
        continued = next.text.ends_with(" \\");
        next_index += 1;
    }
    let value_range = value_start..value_end;
    let value_too_large = value_range.end.saturating_sub(value_range.start)
        > MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES;
    if value_too_large {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::ValueTooLarge,
                range: as_u64_range(value_range.clone()),
                name: Some(name.to_owned()),
                message: format!(
                    "document-header property values are limited to {MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES} bytes"
                ),
            },
        );
    }
    if !continuation_ranges.is_empty()
        || line
            .text
            .get(colon + 1 + leading_space_bytes..)
            .is_some_and(|value| value.ends_with(" \\"))
    {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::ContinuedValue,
                range: as_u64_range(value_range.clone()),
                name: Some(name.to_owned()),
                message:
                    "continued attribute values are processor state, not stable document properties"
                        .to_owned(),
            },
        );
    }
    if form == DocumentHeaderAttributeForm::Unset {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::UnsupportedUnset,
                range: as_u64_range(line.start..entry_end),
                name: Some(name.to_owned()),
                message: "attribute unsets are processor state, not stable document properties"
                    .to_owned(),
            },
        );
    }
    let kind = document_header_attribute_kind(name);
    if kind == DocumentHeaderAttributeKind::ProcessorControl {
        push_document_header_issue(
            issues,
            DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::ProcessorControl,
                range: as_u64_range(line.start..entry_end),
                name: Some(name.to_owned()),
                message: format!(
                    "document-header attribute `{name}` controls AsciiDoc processing and is excluded from stable Properties"
                ),
            },
        );
    }
    let has_continuation = !continuation_ranges.is_empty()
        || line
            .text
            .get(colon + 1 + leading_space_bytes..)
            .is_some_and(|value| value.ends_with(" \\"));
    let has_bounded_literal = valid_name
        && form == DocumentHeaderAttributeForm::Set
        && !value_too_large
        && !has_continuation;
    let projected = has_bounded_literal && kind != DocumentHeaderAttributeKind::ProcessorControl;
    let literal_value = has_bounded_literal.then(|| {
        source
            .get(value_range.clone())
            .unwrap_or_default()
            .to_owned()
    });

    (
        Some(DocumentHeaderAttribute {
            name: name.to_owned(),
            literal_value,
            kind,
            form,
            range: as_u64_range(line.start..entry_end),
            name_range: as_u64_range(name_range),
            value_range: as_u64_range(value_range),
            continuation_ranges,
            projected,
        }),
        next_index,
    )
}

fn push_document_header_issue(issues: &mut Vec<DocumentHeaderIssue>, issue: DocumentHeaderIssue) {
    if issues.len() < MAX_DOCUMENT_HEADER_ISSUES {
        issues.push(issue);
    }
}

fn prefix_has_later_header_construct(source_lines: &[Line<'_>], mut index: usize) -> bool {
    while let Some(line) = source_lines.get(index) {
        if line.text.is_empty() || line.text.starts_with("//") {
            index += 1;
            continue;
        }
        return line.text.starts_with(':')
            || is_document_title_line(line.text)
            || (is_document_title_metadata(line.text)
                && source_lines
                    .get(index + 1)
                    .is_some_and(|next| is_document_title_line(next.text)));
    }
    false
}

fn consume_header_comment_block(source_lines: &[Line<'_>], index: usize) -> (usize, usize) {
    let mut next = index + 1;
    let mut end = source_lines[index].full_end;
    while let Some(line) = source_lines.get(next) {
        end = line.full_end;
        next += 1;
        if line.text == "////" {
            break;
        }
    }
    (next, end)
}

fn is_document_title_line(value: &str) -> bool {
    value.starts_with("= ") && !value.starts_with("== ")
}

fn is_document_title_metadata(value: &str) -> bool {
    (value.starts_with('[') && value.ends_with(']')) || value.starts_with("[[")
}

fn is_native_document_header_line(value: &str) -> bool {
    value
        .chars()
        .next()
        .is_some_and(|first| !first.is_whitespace() && first != '=' && first != '[' && first != ':')
}

fn valid_document_property_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_DOCUMENT_HEADER_ATTRIBUTE_NAME_BYTES
        && name.as_bytes()[0].is_ascii_lowercase()
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

fn validate_document_property_value(value: &str) -> Result<&str, DocumentHeaderPatchError> {
    if value.len() > MAX_DOCUMENT_HEADER_ATTRIBUTE_VALUE_BYTES
        || value.contains(['\r', '\n', '\0'])
        || value.ends_with(" \\")
    {
        return Err(DocumentHeaderPatchError::InvalidValue);
    }
    Ok(value)
}

fn document_header_attribute_kind(name: &str) -> DocumentHeaderAttributeKind {
    if document_header_attribute_is_processor_control(name) {
        DocumentHeaderAttributeKind::ProcessorControl
    } else if matches!(
        name,
        "author"
            | "copyright"
            | "description"
            | "email"
            | "keywords"
            | "lang"
            | "revdate"
            | "revnumber"
            | "revremark"
            | "status"
    ) {
        DocumentHeaderAttributeKind::Descriptive
    } else {
        DocumentHeaderAttributeKind::Custom
    }
}

fn document_header_attribute_is_processor_control(name: &str) -> bool {
    matches!(
        name,
        "allow-uri-read"
            | "attribute-missing"
            | "attribute-undefined"
            | "backend"
            | "basebackend"
            | "data-uri"
            | "docfilesuffix"
            | "docinfo"
            | "docinfodir"
            | "docinfosubs"
            | "doctype"
            | "doctitle"
            | "embedded"
            | "experimental"
            | "hardbreaks-option"
            | "icons"
            | "idprefix"
            | "idseparator"
            | "imagesdir"
            | "imagesoutdir"
            | "includedir"
            | "leveloffset"
            | "linkcss"
            | "max-include-depth"
            | "nofooter"
            | "noheader"
            | "outdir"
            | "outfile"
            | "outfilesuffix"
            | "partialsdir"
            | "relfileprefix"
            | "relfilesuffix"
            | "reproducible"
            | "safe-mode-level"
            | "safe-mode-name"
            | "sectanchors"
            | "sectids"
            | "sectlinks"
            | "sectnumlevels"
            | "sectnums"
            | "showtitle"
            | "source-highlighter"
            | "stem"
            | "stylesheet"
            | "stylesdir"
            | "toc"
            | "toclevels"
            | "webfonts"
            | "xrefstyle"
    )
}

fn u64_range_to_usize(
    range: &Range<u64>,
    source_len: usize,
) -> Result<Range<usize>, DocumentHeaderPatchError> {
    let start =
        usize::try_from(range.start).map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
    let end =
        usize::try_from(range.end).map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
    if start > end || end > source_len {
        return Err(DocumentHeaderPatchError::UnsupportedHeader);
    }
    Ok(start..end)
}

fn adjusted_header_end(
    current: u64,
    removed_bytes: usize,
    inserted_bytes: usize,
) -> Result<u64, DocumentHeaderPatchError> {
    let removed =
        u64::try_from(removed_bytes).map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
    let inserted =
        u64::try_from(inserted_bytes).map_err(|_| DocumentHeaderPatchError::UnsupportedHeader)?;
    current
        .checked_sub(removed)
        .and_then(|end| end.checked_add(inserted))
        .ok_or(DocumentHeaderPatchError::UnsupportedHeader)
}

fn validate_document_header_patch(
    patched: &str,
    name: &str,
    expected_value: Option<&str>,
    expected_header_end: u64,
) -> Result<(), DocumentHeaderPatchError> {
    let header = analyze_document_header(patched);
    if header.range.end != expected_header_end
        || header.issues.iter().any(|issue| {
            matches!(
                issue.code,
                DocumentHeaderIssueCode::ParserFailure
                    | DocumentHeaderIssueCode::UnclosedEnvelope
                    | DocumentHeaderIssueCode::AttributeLimitExceeded
            )
        })
    {
        return Err(DocumentHeaderPatchError::UnsupportedHeader);
    }
    let matches = header
        .attributes
        .iter()
        .filter(|attribute| attribute.name == name)
        .collect::<Vec<_>>();
    match expected_value {
        Some(expected) => {
            if matches.len() == 1
                && matches[0].projected
                && matches[0].literal_value.as_deref() == Some(expected)
            {
                Ok(())
            } else {
                Err(DocumentHeaderPatchError::UnsupportedHeader)
            }
        }
        None if matches.is_empty() => Ok(()),
        None => Err(DocumentHeaderPatchError::UnsupportedHeader),
    }
}

fn preferred_document_line_ending(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

#[must_use]
pub fn analyze(source: &str) -> Analysis {
    analyze_with_presentation(source, AdjacentHeadingBodyDefault::Separate)
}

/// Analyzes a document and resolves adjacent-heading/body presentation from the portable root
/// default. Explicit heading roles retain precedence. This is the shared authority used by Core
/// and the safe renderer; it never merges the two semantic blocks.
#[must_use]
pub fn analyze_with_presentation(
    source: &str,
    adjacent_heading_body: AdjacentHeadingBodyDefault,
) -> Analysis {
    std::panic::catch_unwind(|| analyze_inner(source, adjacent_heading_body)).unwrap_or_else(|_| {
        failed_analysis(
            source,
            "AsciiDoc analysis aborted; the complete source is preserved and rendering failed closed",
        )
    })
}

fn analyze_inner(source: &str, adjacent_heading_body: AdjacentHeadingBodyDefault) -> Analysis {
    let mut diagnostics = Vec::new();
    let mut blocks = Vec::new();
    let mut checklists = Vec::new();
    let mut native_inlines = Vec::new();
    let mut protected_ranges = Vec::new();
    let (body_offset, envelope) =
        probe_envelope(source, &mut blocks, &mut protected_ranges, &mut diagnostics);
    let document_header = analyze_document_header_from(source, body_offset);
    if document_header.range.start < document_header.range.end {
        protected_ranges.push(document_header.range.clone());
    }
    let body = &source[body_offset..];

    parse_native_ascii_doc(
        source,
        body,
        body_offset,
        &mut blocks,
        &mut checklists,
        &mut native_inlines,
        &mut protected_ranges,
        &mut diagnostics,
    );
    scan_profile_lines(
        source,
        body_offset,
        &mut blocks,
        &mut protected_ranges,
        &mut diagnostics,
    );
    normalize_ranges(&mut protected_ranges);
    deduplicate_blocks(&mut blocks);
    diagnose_additional_document_titles(
        source,
        body_offset,
        &blocks,
        &protected_ranges,
        &mut diagnostics,
    );
    diagnose_semantic_blocks(&blocks, &mut diagnostics);

    let (links, mut inlines) = scan_inline_semantics(source, &protected_ranges, &mut diagnostics);
    inlines.extend(native_inlines);
    normalize_inline_semantics(source, &mut inlines, &mut diagnostics);
    diagnose_inline_semantics(&inlines, &mut diagnostics);
    let adjacent_heading_bodies =
        resolve_adjacent_heading_bodies(source, &blocks, adjacent_heading_body);
    let effects = collect_effect_evidence(
        source,
        body_offset,
        &document_header,
        &blocks,
        &inlines,
        &protected_ranges,
    );
    diagnose_effects(&effects, &mut diagnostics);
    let degradations = build_degradations(source, &blocks, &inlines, &diagnostics);
    let status = if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.code,
            DiagnosticCode::ParserError
                | DiagnosticCode::UnclosedFrontmatter
                | DiagnosticCode::AdditionalDocumentTitle
        )
    }) {
        AnalysisStatus::Failed
    } else if degradations.is_empty() && diagnostics.is_empty() {
        AnalysisStatus::Complete
    } else {
        AnalysisStatus::Degraded
    };
    let searchable_text =
        build_searchable_text(source, &protected_ranges, &blocks, &document_header);
    let safe_html = render_safe_html(source, &blocks, &inlines, &adjacent_heading_bodies, status);

    Analysis {
        profile: PROFILE_ID,
        semantic_model_version: SEMANTIC_MODEL_VERSION,
        status,
        capabilities: PROFILE_CAPABILITIES,
        envelope,
        document_header,
        checklists,
        blocks,
        links,
        inlines,
        adjacent_heading_bodies,
        effects,
        protected_ranges,
        diagnostics,
        degradations,
        searchable_text,
        safe_html,
    }
}

fn diagnose_additional_document_titles(
    source: &str,
    body_offset: usize,
    blocks: &[Block],
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let accepted_title = blocks
        .iter()
        .find(|block| block.kind == BlockKind::DocumentTitle)
        .map(|block| block.range.clone());
    for line in lines(source).filter(|line| line.start >= body_offset) {
        if !line.text.starts_with("= ") || inside_ranges(line.start, protected) {
            continue;
        }
        let line_range = as_u64_range(line.start..line.full_end);
        let is_accepted = accepted_title.as_ref().is_some_and(|accepted| {
            accepted.start <= line_range.start && line_range.end <= accepted.end
        });
        if !is_accepted {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::AdditionalDocumentTitle,
                range: line_range,
                message: "a second or misplaced level-zero title is invalid Weftext AsciiDoc v1 structure"
                    .to_owned(),
            });
        }
    }
}

fn resolve_adjacent_heading_bodies(
    source: &str,
    blocks: &[Block],
    default: AdjacentHeadingBodyDefault,
) -> Vec<AdjacentHeadingBodyResolution> {
    blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| {
            block.kind == BlockKind::Heading
                && block
                    .heading_level
                    .is_some_and(|level| (1..=9).contains(&level))
        })
        .map(|(heading_index, heading)| {
            let explicit_run_in = heading.roles.iter().any(|role| role == "run-in");
            let explicit_separate = heading.roles.iter().any(|role| role == "separate");
            let (rule, requested) = if explicit_run_in {
                (
                    AdjacentHeadingBodyRule::ExplicitRunInRole,
                    AdjacentHeadingBodyPresentation::RunIn,
                )
            } else if explicit_separate {
                (
                    AdjacentHeadingBodyRule::ExplicitSeparateRole,
                    AdjacentHeadingBodyPresentation::Separate,
                )
            } else {
                match default {
                    AdjacentHeadingBodyDefault::RunIn => (
                        AdjacentHeadingBodyRule::WorkspaceRunInDefault,
                        AdjacentHeadingBodyPresentation::RunIn,
                    ),
                    AdjacentHeadingBodyDefault::Separate => (
                        AdjacentHeadingBodyRule::WorkspaceSeparateDefault,
                        AdjacentHeadingBodyPresentation::Separate,
                    ),
                }
            };

            let following = blocks.get(heading_index + 1);
            let body_index = following
                .filter(|block| block.kind == BlockKind::Paragraph)
                .map(|_| (heading_index + 1) as u64);
            let eligibility = match following {
                None => AdjacentHeadingBodyEligibility::NoFollowingBlock,
                Some(block) if block.kind != BlockKind::Paragraph => {
                    AdjacentHeadingBodyEligibility::FollowingBlockIsNotParagraph
                }
                Some(block) => {
                    let gap = exact_source_gap(source, heading.range.end, block.range.start);
                    let allowed_gap = if rule == AdjacentHeadingBodyRule::ExplicitRunInRole
                        || rule == AdjacentHeadingBodyRule::ExplicitSeparateRole
                    {
                        gap.is_some_and(explicit_paragraph_gap)
                    } else {
                        gap.is_some_and(|gap| gap.chars().all(char::is_whitespace))
                    };
                    if !allowed_gap {
                        AdjacentHeadingBodyEligibility::NonWhitespaceSourceGap
                    } else if rule == AdjacentHeadingBodyRule::WorkspaceRunInDefault
                        && heading.range.end != block.range.start
                    {
                        AdjacentHeadingBodyEligibility::NotOnImmediatelyFollowingPhysicalLine
                    } else {
                        AdjacentHeadingBodyEligibility::Eligible
                    }
                }
            };
            let presentation = if requested == AdjacentHeadingBodyPresentation::RunIn
                && eligibility == AdjacentHeadingBodyEligibility::Eligible
            {
                AdjacentHeadingBodyPresentation::RunIn
            } else {
                AdjacentHeadingBodyPresentation::Separate
            };
            AdjacentHeadingBodyResolution {
                heading_block: heading_index as u64,
                body_block: body_index,
                presentation,
                rule,
                eligibility,
            }
        })
        .collect()
}

fn exact_source_gap(source: &str, start: u64, end: u64) -> Option<&str> {
    let start = usize::try_from(start).ok()?;
    let end = usize::try_from(end).ok()?;
    source.get(start..end)
}

fn explicit_paragraph_gap(gap: &str) -> bool {
    lines(gap).all(|line| {
        line.text.trim().is_empty() || (line.text.starts_with('[') && line.text.ends_with(']'))
    })
}

fn collect_effect_evidence(
    source: &str,
    body_offset: usize,
    document_header: &DocumentHeaderSemantic,
    blocks: &[Block],
    inlines: &[InlineSemantic],
    protected: &[Range<u64>],
) -> Vec<EffectEvidence> {
    let mut effects = Vec::new();
    collect_directive_effects(source, body_offset, protected, &mut effects);

    effects.extend(
        document_header
            .attributes
            .iter()
            .filter(|attribute| attribute.kind == DocumentHeaderAttributeKind::ProcessorControl)
            .map(|attribute| {
                let decision = processor_attribute_effect_decision(&attribute.name);
                EffectEvidence {
                    origin: EffectOrigin::DocumentHeaderAttribute,
                    required_capability: EffectCapability::ProcessorExecution,
                    decision,
                    range: attribute.range.clone(),
                    target: Some(attribute.name.clone()),
                    message: if decision == EffectDecision::Denied {
                        "processor declaration is preserved, but processor execution is disabled"
                    } else {
                        "processor-control attribute is preserved as inert state and grants no processor execution capability"
                    }
                    .to_owned(),
                }
            }),
    );

    for block in blocks {
        match &block.semantic {
            BlockSemantic::Passthrough => effects.push(EffectEvidence {
                origin: EffectOrigin::BlockPassthrough,
                required_capability: EffectCapability::PassthroughRendering,
                decision: EffectDecision::Denied,
                range: block.range.clone(),
                target: None,
                message: "block passthrough rendering is disabled; exact source is retained"
                    .to_owned(),
            }),
            BlockSemantic::Image { target, .. } => {
                collect_external_image_effect(block.range.clone(), target, &mut effects);
            }
            _ => {}
        }
    }
    for inline in inlines {
        match inline.kind {
            InlineKind::Passthrough => effects.push(EffectEvidence {
                origin: EffectOrigin::InlinePassthrough,
                required_capability: EffectCapability::PassthroughRendering,
                decision: EffectDecision::Denied,
                range: inline.range.clone(),
                target: None,
                message: "inline passthrough rendering is disabled; exact source is retained"
                    .to_owned(),
            }),
            InlineKind::Image => {
                if let Some(target) = &inline.target {
                    collect_external_image_effect(inline.range.clone(), target, &mut effects);
                }
            }
            _ => {}
        }
    }
    effects.sort_by_key(|effect| {
        (
            effect.range.start,
            effect.range.end,
            effect.origin as u8,
            effect.required_capability as u8,
        )
    });
    effects.dedup_by(|left, right| {
        left.origin == right.origin
            && left.required_capability == right.required_capability
            && left.range == right.range
    });
    effects
}

fn processor_attribute_effect_decision(name: &str) -> EffectDecision {
    if matches!(
        name,
        "allow-uri-read" | "docinfo" | "linkcss" | "source-highlighter" | "stylesheet" | "webfonts"
    ) {
        EffectDecision::Denied
    } else {
        EffectDecision::PreservedInert
    }
}

fn collect_directive_effects(
    source: &str,
    body_offset: usize,
    protected: &[Range<u64>],
    effects: &mut Vec<EffectEvidence>,
) {
    for line in lines(source).filter(|line| line.start >= body_offset) {
        if inside_ranges(line.start, protected) {
            continue;
        }
        if let Some(target) = directive_target(line.text, "include::") {
            effects.push(EffectEvidence {
                origin: EffectOrigin::IncludeDirective,
                required_capability: EffectCapability::IncludeExpansion,
                decision: EffectDecision::Denied,
                range: as_u64_range(line.start..line.end),
                target: Some(target.to_owned()),
                message: "include source is preserved, but include expansion is disabled"
                    .to_owned(),
            });
            let resource_capability = match remote_uri_scheme(target) {
                Some("file") | None => EffectCapability::WorkspaceRead,
                Some(_) => EffectCapability::NetworkRead,
            };
            effects.push(EffectEvidence {
                origin: EffectOrigin::IncludeDirective,
                required_capability: resource_capability,
                decision: EffectDecision::Denied,
                range: as_u64_range(line.start..line.end),
                target: Some(target.to_owned()),
                message: if resource_capability == EffectCapability::NetworkRead {
                    "remote include loading is disabled; the URI remains exact source"
                } else {
                    "include workspace reads require a future validated Core locator capability"
                }
                .to_owned(),
            });
            continue;
        }
        if ["ifdef::", "ifndef::", "ifeval::", "endif::"]
            .iter()
            .any(|prefix| line.text.starts_with(prefix))
        {
            effects.push(EffectEvidence {
                origin: EffectOrigin::ConditionalDirective,
                required_capability: EffectCapability::ConditionalEvaluation,
                decision: EffectDecision::Denied,
                range: as_u64_range(line.start..line.end),
                target: None,
                message:
                    "conditional directive is preserved, but conditional evaluation is disabled"
                        .to_owned(),
            });
        }
    }
}

fn directive_target<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let remainder = line.strip_prefix(prefix)?;
    let open = remainder.find('[')?;
    let target = &remainder[..open];
    (!target.is_empty()).then_some(target)
}

fn collect_external_image_effect(
    range: Range<u64>,
    target: &str,
    effects: &mut Vec<EffectEvidence>,
) {
    let Some(scheme) = remote_uri_scheme(target) else {
        return;
    };
    let capability = if scheme == "file" {
        EffectCapability::WorkspaceRead
    } else {
        EffectCapability::NetworkRead
    };
    effects.push(EffectEvidence {
        origin: EffectOrigin::ImageResource,
        required_capability: capability,
        decision: EffectDecision::Denied,
        range,
        target: Some(target.to_owned()),
        message: "external image resource loading is disabled; the target remains exact source"
            .to_owned(),
    });
}

fn remote_uri_scheme(target: &str) -> Option<&'static str> {
    [
        ("https://", "https"),
        ("http://", "http"),
        ("ftp://", "ftp"),
        ("file://", "file"),
    ]
    .into_iter()
    .find_map(|(prefix, name)| target.starts_with(prefix).then_some(name))
}

fn diagnose_effects(effects: &[EffectEvidence], diagnostics: &mut Vec<Diagnostic>) {
    for effect in effects
        .iter()
        .filter(|effect| effect.decision == EffectDecision::Denied)
    {
        let code = match effect.origin {
            EffectOrigin::IncludeDirective
                if effect.required_capability == EffectCapability::NetworkRead =>
            {
                DiagnosticCode::RemoteUri
            }
            EffectOrigin::IncludeDirective => DiagnosticCode::UnsafeInclude,
            EffectOrigin::ConditionalDirective => DiagnosticCode::ConditionalDisabled,
            EffectOrigin::DocumentHeaderAttribute => DiagnosticCode::ProcessorEffectDisabled,
            EffectOrigin::BlockPassthrough | EffectOrigin::InlinePassthrough => {
                DiagnosticCode::PassthroughDisabled
            }
            EffectOrigin::ImageResource => DiagnosticCode::RemoteUri,
        };
        if !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.range == effect.range)
        {
            diagnostics.push(Diagnostic {
                code,
                range: effect.range.clone(),
                message: effect.message.clone(),
            });
        }
    }
}

fn diagnose_semantic_blocks(blocks: &[Block], diagnostics: &mut Vec<Diagnostic>) {
    for block in blocks {
        let (code, message) = match &block.semantic {
            BlockSemantic::Unsupported { context } => (
                DiagnosticCode::UnsupportedProfileSyntax,
                format!(
                    "AsciiDoc block context {context} is preserved exactly but has no typed v1 renderer"
                ),
            ),
            BlockSemantic::Passthrough => (
                DiagnosticCode::PassthroughDisabled,
                "passthrough content is preserved exactly and its effect is disabled".to_owned(),
            ),
            BlockSemantic::Math { notation } => (
                DiagnosticCode::ParserWarning,
                format!(
                    "{notation:?} content requires the constrained STEM renderer; the safe fallback displays exact source"
                ),
            ),
            BlockSemantic::Mermaid => (
                DiagnosticCode::ParserWarning,
                "Mermaid content requires the constrained diagram renderer; the safe fallback displays exact source"
                    .to_owned(),
            ),
            BlockSemantic::List { model } if list_has_unmodeled_continuations(model) => (
                DiagnosticCode::UnsupportedProfileSyntax,
                "list continuation blocks are preserved exactly but are not silently flattened into list-item text"
                    .to_owned(),
            ),
            BlockSemantic::Table { model } if table_has_nested_asciidoc(model) => (
                DiagnosticCode::UnsupportedProfileSyntax,
                "AsciiDoc table cells are preserved exactly but require a nested safe-rendering implementation"
                    .to_owned(),
            ),
            _ => continue,
        };
        if !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.range == block.range)
        {
            diagnostics.push(Diagnostic {
                code,
                range: block.range.clone(),
                message,
            });
        }
    }
}

fn list_has_unmodeled_continuations(model: &ListModel) -> bool {
    fn item_has_unmodeled(item: &ListItem) -> bool {
        !item.unmodeled_continuations.is_empty() || item.children.iter().any(item_has_unmodeled)
    }
    model.items.iter().any(item_has_unmodeled)
}

fn table_has_nested_asciidoc(model: &TableModel) -> bool {
    model
        .header
        .iter()
        .chain(model.body.iter())
        .chain(model.footer.iter())
        .flat_map(|row| &row.cells)
        .any(|cell| cell.nested_asciidoc)
}

fn build_degradations(
    _source: &str,
    blocks: &[Block],
    inlines: &[InlineSemantic],
    diagnostics: &[Diagnostic],
) -> Vec<Degradation> {
    let mut degradations = diagnostics
        .iter()
        .map(|diagnostic| {
            let (kind, support_state, fallback) = diagnostic_degradation(diagnostic, inlines);
            Degradation {
                kind,
                support_state,
                range: diagnostic.range.clone(),
                fallback,
                message: diagnostic.message.clone(),
            }
        })
        .collect::<Vec<_>>();
    for block in blocks {
        let (kind, message) = match block.semantic {
            BlockSemantic::Math { .. } => (
                DegradationKind::ConstrainedMath,
                "STEM rendering is constrained; exact source is the fallback",
            ),
            BlockSemantic::Mermaid => (
                DegradationKind::ConstrainedMermaid,
                "Mermaid rendering is constrained; exact source is the fallback",
            ),
            _ => continue,
        };
        degradations.push(Degradation {
            kind,
            support_state: SupportState::Constrained,
            range: block.range.clone(),
            fallback: RenderFallback::EscapedSource,
            message: message.to_owned(),
        });
    }
    degradations.sort_by_key(|item| (item.range.start, item.range.end, item.kind as u8));
    degradations.dedup_by(|left, right| left.kind == right.kind && left.range == right.range);
    degradations
}

fn diagnostic_degradation(
    diagnostic: &Diagnostic,
    inlines: &[InlineSemantic],
) -> (DegradationKind, SupportState, RenderFallback) {
    match diagnostic.code {
        DiagnosticCode::UnclosedFrontmatter | DiagnosticCode::ParserError => (
            DegradationKind::ParserFailure,
            SupportState::PreserveOnly,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::AdditionalDocumentTitle => (
            DegradationKind::InvalidStructure,
            SupportState::PreserveOnly,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::UnsafeInclude => (
            DegradationKind::DisabledInclude,
            SupportState::ProhibitedEffect,
            RenderFallback::DisabledEffect,
        ),
        DiagnosticCode::ConditionalDisabled | DiagnosticCode::ProcessorEffectDisabled => (
            DegradationKind::UnsupportedBlock,
            SupportState::ProhibitedEffect,
            RenderFallback::DisabledEffect,
        ),
        DiagnosticCode::RemoteUri => (
            DegradationKind::DisabledRemoteUri,
            SupportState::ProhibitedEffect,
            RenderFallback::DisabledEffect,
        ),
        DiagnosticCode::PassthroughDisabled => (
            DegradationKind::DisabledPassthrough,
            SupportState::ProhibitedEffect,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::InvalidNodeLink => (
            DegradationKind::UnsupportedInline,
            SupportState::PreserveOnly,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::UnsupportedProfileSyntax
            if inlines.iter().any(|inline| {
                inline.kind == InlineKind::Unsupported && inline.range == diagnostic.range
            }) || diagnostic.message.contains("inline construct") =>
        {
            (
                DegradationKind::UnsupportedInline,
                SupportState::PreserveOnly,
                RenderFallback::NoDerivedRendering,
            )
        }
        DiagnosticCode::UnsupportedProfileSyntax => (
            DegradationKind::UnsupportedBlock,
            SupportState::PreserveOnly,
            RenderFallback::NoDerivedRendering,
        ),
        DiagnosticCode::ParserWarning if diagnostic.message.contains("STEM renderer") => (
            DegradationKind::ConstrainedMath,
            SupportState::Constrained,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::ParserWarning if diagnostic.message.contains("diagram renderer") => (
            DegradationKind::ConstrainedMermaid,
            SupportState::Constrained,
            RenderFallback::EscapedSource,
        ),
        DiagnosticCode::QuoteSyntaxUnresolved | DiagnosticCode::ParserWarning => (
            DegradationKind::UnsupportedBlock,
            SupportState::PreserveOnly,
            RenderFallback::EscapedSource,
        ),
    }
}

fn failed_analysis(source: &str, message: &str) -> Analysis {
    let range = 0..source.len() as u64;
    Analysis {
        profile: PROFILE_ID,
        semantic_model_version: SEMANTIC_MODEL_VERSION,
        status: AnalysisStatus::Failed,
        capabilities: PROFILE_CAPABILITIES,
        envelope: None,
        document_header: DocumentHeaderSemantic {
            range: 0..0,
            insertion_offset: 0,
            attributes: Vec::new(),
            issues: vec![DocumentHeaderIssue {
                code: DocumentHeaderIssueCode::ParserFailure,
                range: range.clone(),
                name: None,
                message: message.to_owned(),
            }],
        },
        checklists: Vec::new(),
        blocks: Vec::new(),
        links: Vec::new(),
        inlines: Vec::new(),
        adjacent_heading_bodies: Vec::new(),
        effects: Vec::new(),
        protected_ranges: vec![range.clone()],
        diagnostics: vec![Diagnostic {
            code: DiagnosticCode::ParserError,
            range: range.clone(),
            message: message.to_owned(),
        }],
        degradations: vec![Degradation {
            kind: DegradationKind::ParserFailure,
            support_state: SupportState::PreserveOnly,
            range,
            fallback: RenderFallback::EscapedSource,
            message: message.to_owned(),
        }],
        searchable_text: String::new(),
        safe_html: escaped_source_fallback(source, "failed"),
    }
}

/// Probes only the exact leading envelope delimiters.
///
/// The opening delimiter must begin at byte zero. In particular, a UTF-8 byte-order mark or any
/// preceding whitespace makes the envelope absent. Both `LF` and `CRLF` are retained in the
/// returned ranges and the closing delimiter must be the complete physical line text `---`.
#[must_use]
pub fn probe_managed_envelope(source: &str) -> EnvelopeProbe {
    let Some(first) = lines(source).next() else {
        return absent_envelope_probe();
    };
    if first.text != "---" {
        return absent_envelope_probe();
    }
    for line in lines(source).skip(1) {
        if line.text == "---" {
            return EnvelopeProbe {
                state: EnvelopeProbeState::Closed,
                range: Some(as_u64_range(0..line.full_end)),
                content_range: Some(as_u64_range(first.full_end..line.start)),
                body_start: to_u64(line.full_end),
            };
        }
    }
    EnvelopeProbe {
        state: EnvelopeProbeState::Unclosed,
        range: Some(0..to_u64(source.len())),
        content_range: Some(to_u64(first.full_end)..to_u64(source.len())),
        body_start: to_u64(source.len()),
    }
}

/// Analyzes the canonical managed envelope without parsing the `AsciiDoc` body.
///
/// A semantic value is present only for a closed envelope. Its `valid` flag is the Profile-owned
/// canonical accept/reject decision; unknown inner fields remain exact warning-bearing evidence.
#[must_use]
pub fn analyze_managed_envelope(source: &str) -> ManagedEnvelopeAnalysis {
    let probe = probe_managed_envelope(source);
    let semantic = if probe.state == EnvelopeProbeState::Closed {
        let content_range = probe.content_range.as_ref().and_then(range_as_usize);
        let envelope_end = probe
            .range
            .as_ref()
            .and_then(|range| usize::try_from(range.end).ok());
        content_range
            .zip(envelope_end)
            .map(|(content, end)| analyze_closed_envelope(source, content.start, content.end, end))
    } else {
        None
    };
    ManagedEnvelopeAnalysis { probe, semantic }
}

/// Applies one Profile-owned narrow patch to a valid canonical envelope.
///
/// Unrelated source bytes, including unknown inner fields, comments, document-header bytes, body
/// bytes, and line endings, are never normalized. Invalid input and invalid output both fail
/// closed.
///
/// # Errors
///
/// Returns an error for a missing, unclosed, or non-canonical envelope; an invalid typed value;
/// or a source range that cannot be edited exactly.
pub fn patch_managed_envelope(
    source: &str,
    patch: ManagedEnvelopePatch,
) -> Result<String, ManagedEnvelopePatchError> {
    let analysis = analyze_managed_envelope(source);
    match analysis.probe.state {
        EnvelopeProbeState::Absent => return Err(ManagedEnvelopePatchError::MissingEnvelope),
        EnvelopeProbeState::Unclosed => return Err(ManagedEnvelopePatchError::UnclosedEnvelope),
        EnvelopeProbeState::Closed => {}
    }
    let envelope = analysis
        .semantic
        .as_ref()
        .ok_or(ManagedEnvelopePatchError::InvalidEnvelope)?;
    if !envelope.valid {
        return Err(ManagedEnvelopePatchError::InvalidEnvelope);
    }
    let prepared = prepare_managed_envelope_patch(patch)?;
    let existing = envelope
        .fields
        .iter()
        .find(|field| field.kind == prepared.kind);
    let edit = match (existing, prepared.encoded_value) {
        (Some(field), Some(value)) if prepared.kind == EnvelopeFieldKind::Aliases => SourceEdit {
            range: envelope_field_definition_range(source, field)?,
            replacement: format_aliases_field(&value, envelope_line_ending(source, envelope)),
        },
        (Some(field), Some(value)) => SourceEdit {
            range: range_as_usize(&field.value_range)
                .ok_or(ManagedEnvelopePatchError::UnsupportedRange)?,
            replacement: value,
        },
        (Some(field), None) => SourceEdit {
            range: envelope_field_definition_range(source, field)?,
            replacement: String::new(),
        },
        (None, Some(value)) => {
            let insert_at = envelope
                .weftext_range
                .as_ref()
                .and_then(|range| usize::try_from(range.end).ok())
                .filter(|offset| *offset <= source.len() && source.is_char_boundary(*offset))
                .ok_or(ManagedEnvelopePatchError::UnsupportedRange)?;
            let line_ending = envelope_line_ending(source, envelope);
            let replacement = if prepared.kind == EnvelopeFieldKind::Aliases {
                format_aliases_field(&value, line_ending)
            } else {
                format!("  {}: {value}{line_ending}", prepared.key)
            };
            SourceEdit {
                range: insert_at..insert_at,
                replacement,
            }
        }
        (None, None) => return Ok(source.to_owned()),
    };
    let plan = SourceEditPlan::new(source, vec![edit])
        .map_err(|_| ManagedEnvelopePatchError::UnsupportedRange)?;
    let patched = plan
        .apply(source)
        .ok_or(ManagedEnvelopePatchError::UnsupportedRange)?;
    let verification = analyze_managed_envelope(&patched);
    if verification.probe.state != EnvelopeProbeState::Closed
        || !verification.semantic.is_some_and(|semantic| semantic.valid)
    {
        return Err(ManagedEnvelopePatchError::InvalidEnvelope);
    }
    Ok(patched)
}

/// Creates the minimal canonical envelope for a new managed document.
///
/// Structural workspace creation delegates here so the Profile crate remains the only producer of
/// envelope spelling. Callers append the `AsciiDoc` document header/body separately.
///
/// # Errors
///
/// Returns an error unless `id` is `UUIDv4`.
pub fn new_managed_document_envelope(id: Uuid) -> Result<String, ManagedEnvelopePatchError> {
    if id.get_version_num() != 4 {
        return Err(ManagedEnvelopePatchError::InvalidValue);
    }
    Ok(format!(
        "---\nweftext:\n  id: {}\n---\n",
        encode_double_quoted_envelope_scalar(&id.hyphenated().to_string())
    ))
}

struct PreparedManagedEnvelopePatch {
    kind: EnvelopeFieldKind,
    key: &'static str,
    encoded_value: Option<String>,
}

fn prepare_managed_envelope_patch(
    patch: ManagedEnvelopePatch,
) -> Result<PreparedManagedEnvelopePatch, ManagedEnvelopePatchError> {
    let (kind, key, encoded_value) = match patch {
        ManagedEnvelopePatch::Id(value) => {
            if value.get_version_num() != 4 {
                return Err(ManagedEnvelopePatchError::InvalidValue);
            }
            (
                EnvelopeFieldKind::Id,
                "id",
                Some(encode_double_quoted_envelope_scalar(
                    &value.hyphenated().to_string(),
                )),
            )
        }
        ManagedEnvelopePatch::Icon(value) => {
            if value
                .as_deref()
                .is_some_and(|candidate| !is_canonical_envelope_icon_scalar(candidate))
            {
                return Err(ManagedEnvelopePatchError::InvalidValue);
            }
            (
                EnvelopeFieldKind::Icon,
                "icon",
                value.map(|candidate| encode_double_quoted_envelope_scalar(&candidate)),
            )
        }
        ManagedEnvelopePatch::Aliases(values) => {
            validate_envelope_alias_values(&values)?;
            (
                EnvelopeFieldKind::Aliases,
                "aliases",
                (!values.is_empty()).then(|| {
                    values
                        .iter()
                        .map(|value| encode_double_quoted_envelope_scalar(value))
                        .collect::<Vec<_>>()
                        .join("\n")
                }),
            )
        }
        ManagedEnvelopePatch::ChildSort(value) => (
            EnvelopeFieldKind::ChildSort,
            "child_sort",
            value.map(|value| match value {
                EnvelopeChildSort::Name => "name".to_owned(),
                EnvelopeChildSort::Manual => "manual".to_owned(),
            }),
        ),
        ManagedEnvelopePatch::ChildSortDirection(value) => (
            EnvelopeFieldKind::ChildSortDirection,
            "child_sort_direction",
            value.map(|value| match value {
                EnvelopeChildSortDirection::Ascending => "ascending".to_owned(),
                EnvelopeChildSortDirection::Descending => "descending".to_owned(),
            }),
        ),
        ManagedEnvelopePatch::SiblingRank(value) => {
            if value == Some(0) {
                return Err(ManagedEnvelopePatchError::InvalidValue);
            }
            (
                EnvelopeFieldKind::SiblingRank,
                "sibling_rank",
                value.map(|value| value.to_string()),
            )
        }
        ManagedEnvelopePatch::AdjacentHeadingBody(value) => (
            EnvelopeFieldKind::AdjacentHeadingBody,
            "adjacent_heading_body",
            value.map(|value| match value {
                EnvelopeAdjacentHeadingBody::RunIn => "run_in".to_owned(),
                EnvelopeAdjacentHeadingBody::Separate => "separate".to_owned(),
            }),
        ),
    };
    Ok(PreparedManagedEnvelopePatch {
        kind,
        key,
        encoded_value,
    })
}

fn validate_envelope_alias_values(values: &[String]) -> Result<(), ManagedEnvelopePatchError> {
    if values.len() > 256 {
        return Err(ManagedEnvelopePatchError::InvalidValue);
    }
    let mut seen = BTreeSet::<&str>::new();
    for value in values {
        if !is_valid_envelope_alias(value) || !seen.insert(value) {
            return Err(ManagedEnvelopePatchError::InvalidValue);
        }
    }
    Ok(())
}

fn is_valid_envelope_alias(value: &str) -> bool {
    !value.is_empty() && value.len() <= 1_024 && !value.chars().any(char::is_control)
}

fn encode_double_quoted_envelope_scalar(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len().saturating_add(2));
    encoded.push('"');
    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{0008}' => encoded.push_str("\\b"),
            '\u{000c}' => encoded.push_str("\\f"),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            character if character.is_control() => {
                let _ = write!(encoded, "\\u{:04X}", u32::from(character));
            }
            character => encoded.push(character),
        }
    }
    encoded.push('"');
    encoded
}

fn envelope_field_definition_range(
    source: &str,
    field: &EnvelopeField,
) -> Result<Range<usize>, ManagedEnvelopePatchError> {
    let start = usize::try_from(field.range.start)
        .ok()
        .filter(|start| *start <= source.len() && source.is_char_boundary(*start))
        .ok_or(ManagedEnvelopePatchError::UnsupportedRange)?;
    let physical_line_end = || {
        source[start..]
            .find('\n')
            .map_or(source.len(), |offset| start + offset + 1)
    };
    let end = match &field.value {
        EnvelopeFieldValue::StringList { items }
            if !items.is_empty()
                && range_as_usize(&field.value_range)
                    .and_then(|range| source.get(range))
                    .is_some_and(|exact| !exact.trim_start().starts_with('[')) =>
        {
            items
                .last()
                .and_then(|item| usize::try_from(item.range.end).ok())
                .unwrap_or(start)
        }
        _ => physical_line_end(),
    };
    if end < start || end > source.len() || !source.is_char_boundary(end) {
        return Err(ManagedEnvelopePatchError::UnsupportedRange);
    }
    Ok(start..end)
}

fn envelope_line_ending<'a>(source: &'a str, envelope: &EnvelopeSemantic) -> &'a str {
    let exact = range_as_usize(&envelope.range)
        .and_then(|range| source.get(range))
        .unwrap_or(source);
    if exact.contains("\r\n") { "\r\n" } else { "\n" }
}

fn format_aliases_field(encoded_values: &str, line_ending: &str) -> String {
    let mut field = format!("  aliases:{line_ending}");
    for encoded in encoded_values.split('\n') {
        let _ = write!(field, "    - {encoded}{line_ending}");
    }
    field
}

fn absent_envelope_probe() -> EnvelopeProbe {
    EnvelopeProbe {
        state: EnvelopeProbeState::Absent,
        range: None,
        content_range: None,
        body_start: 0,
    }
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn range_as_usize(range: &Range<u64>) -> Option<Range<usize>> {
    Some(usize::try_from(range.start).ok()?..usize::try_from(range.end).ok()?)
}

fn probe_envelope(
    source: &str,
    blocks: &mut Vec<Block>,
    protected: &mut Vec<Range<u64>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> (usize, Option<EnvelopeSemantic>) {
    let analysis = analyze_managed_envelope(source);
    match analysis.probe.state {
        EnvelopeProbeState::Absent => (0, None),
        EnvelopeProbeState::Closed => {
            let end = usize::try_from(analysis.probe.body_start).unwrap_or(source.len());
            push_source_block(
                blocks,
                BlockKind::Frontmatter,
                0..end,
                0..end,
                source,
                None,
                None,
                Vec::new(),
            );
            protected.push(as_u64_range(0..end));
            let envelope = analysis
                .semantic
                .expect("a closed managed envelope has semantic evidence");
            diagnostics.extend(envelope.issues.iter().map(|issue| Diagnostic {
                code: DiagnosticCode::UnsupportedProfileSyntax,
                range: issue.range.clone(),
                message: issue.message.clone(),
            }));
            (end, Some(envelope))
        }
        EnvelopeProbeState::Unclosed => {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::UnclosedFrontmatter,
                range: 0..source.len() as u64,
                message: "the leading Weftext YAML envelope is not closed".to_owned(),
            });
            protected.push(0..source.len() as u64);
            (source.len(), None)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EnvelopeScalarError {
    Invalid,
    UnsafeYamlFeature,
}

#[derive(Clone, Debug)]
struct EnvelopeMapping<'a> {
    key: &'a str,
    key_range: Range<usize>,
    raw: &'a str,
    value_range: Range<usize>,
}

#[allow(clippy::too_many_lines)]
fn analyze_closed_envelope(
    source: &str,
    content_start: usize,
    content_end: usize,
    envelope_end: usize,
) -> EnvelopeSemantic {
    let content_lines = lines(source)
        .filter(|line| line.start >= content_start && line.start < content_end)
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    let mut fields = Vec::new();
    let mut weftext_range = None;
    let mut weftext_key_range = None;
    let mut seen_weftext = false;
    let mut seen_top_level = Vec::<String>::new();
    let mut index = 0;

    while index < content_lines.len() {
        let line = &content_lines[index];
        let trimmed = line.text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        let block_end = envelope_block_end(&content_lines, index, 0);
        let block_range = line.start..envelope_lines_end(&content_lines, block_end, content_end);
        if line.text.contains('\t') {
            push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::UnsafeYamlFeature,
                EnvelopeIssueSeverity::Error,
                line.start..line.end,
                "tabs are not accepted in the canonical Weftext YAML envelope",
            );
            index = block_end.max(index + 1);
            continue;
        }
        if envelope_indentation(line.text) != 0 {
            push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::InvalidStructure,
                EnvelopeIssueSeverity::Error,
                block_range.clone(),
                "the Weftext YAML envelope must contain exactly one top-level mapping",
            );
            index = block_end.max(index + 1);
            continue;
        }
        let Some(mapping) = envelope_mapping(line) else {
            push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::InvalidStructure,
                EnvelopeIssueSeverity::Error,
                line.start..line.end,
                "top-level YAML content is not a canonical mapping entry",
            );
            index = block_end.max(index + 1);
            continue;
        };
        if seen_top_level.iter().any(|key| key == mapping.key) {
            push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::DuplicateTopLevelKey,
                EnvelopeIssueSeverity::Error,
                mapping.key_range.clone(),
                "the top-level YAML key is duplicated",
            );
        }
        seen_top_level.push(mapping.key.to_owned());

        match mapping.key {
            "weftext" => {
                if seen_weftext {
                    index = block_end.max(index + 1);
                    continue;
                }
                seen_weftext = true;
                weftext_range = Some(as_u64_range(block_range.clone()));
                weftext_key_range = Some(as_u64_range(mapping.key_range.clone()));
                if matches!(
                    decode_envelope_scalar(mapping.raw),
                    Err(EnvelopeScalarError::UnsafeYamlFeature)
                ) {
                    push_envelope_issue(
                        &mut issues,
                        EnvelopeIssueCode::UnsafeYamlFeature,
                        EnvelopeIssueSeverity::Error,
                        mapping.value_range.clone(),
                        "YAML tags, anchors, and aliases are not accepted in the weftext envelope",
                    );
                }
                if !mapping.raw.is_empty() && !mapping.raw.starts_with('#') {
                    push_envelope_issue(
                        &mut issues,
                        EnvelopeIssueCode::InvalidStructure,
                        EnvelopeIssueSeverity::Error,
                        mapping.value_range.clone(),
                        "the top-level weftext value must be a block mapping",
                    );
                }
                parse_weftext_fields(
                    &content_lines,
                    index + 1,
                    block_end,
                    &mut fields,
                    &mut issues,
                );
            }
            "_weftext" => push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::LegacyTopLevelKey,
                EnvelopeIssueSeverity::Error,
                mapping.key_range,
                "_weftext is retired; the canonical envelope key is weftext",
            ),
            _ => push_envelope_issue(
                &mut issues,
                EnvelopeIssueCode::UnknownTopLevelKey,
                EnvelopeIssueSeverity::Error,
                mapping.key_range,
                "only the top-level weftext mapping is canonical",
            ),
        }
        index = block_end.max(index + 1);
    }

    if !seen_weftext {
        push_envelope_issue(
            &mut issues,
            EnvelopeIssueCode::MissingWeftextMapping,
            EnvelopeIssueSeverity::Error,
            content_start..content_end,
            "the YAML envelope does not contain the required weftext mapping",
        );
    } else if !fields
        .iter()
        .any(|field| field.kind == EnvelopeFieldKind::Id)
    {
        let range = weftext_key_range
            .as_ref()
            .map_or(content_start..content_end, |range| {
                usize::try_from(range.start).unwrap_or(content_start)
                    ..usize::try_from(range.end).unwrap_or(content_end)
            });
        push_envelope_issue(
            &mut issues,
            EnvelopeIssueCode::MissingRequiredField,
            EnvelopeIssueSeverity::Error,
            range,
            "weftext.id is required for a canonical managed document",
        );
    }

    fields.sort_by_key(|field| (field.range.start, field.range.end));
    let valid = !issues
        .iter()
        .any(|issue| issue.severity == EnvelopeIssueSeverity::Error);
    EnvelopeSemantic {
        range: as_u64_range(0..envelope_end),
        content_range: as_u64_range(content_start..content_end),
        weftext_range,
        weftext_key_range,
        fields,
        issues,
        valid,
    }
}

#[allow(clippy::too_many_lines)]
fn parse_weftext_fields(
    content_lines: &[Line<'_>],
    start: usize,
    end: usize,
    fields: &mut Vec<EnvelopeField>,
    issues: &mut Vec<EnvelopeIssue>,
) {
    let mut seen = Vec::<String>::new();
    let mut index = start;
    while index < end {
        let line = &content_lines[index];
        let trimmed = line.text.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            index += 1;
            continue;
        }
        let field_end = envelope_block_end(&content_lines[..end], index, 2);
        let field_range = line.start..envelope_lines_end(content_lines, field_end, line.full_end);
        if line.text.contains('\t') {
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::UnsafeYamlFeature,
                EnvelopeIssueSeverity::Error,
                field_range.clone(),
                "tabs are not accepted in the canonical weftext mapping",
            );
            index = field_end.max(index + 1);
            continue;
        }
        if envelope_indentation(line.text) != 2 {
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::InvalidStructure,
                EnvelopeIssueSeverity::Error,
                field_range.clone(),
                "weftext fields must be shallow mapping entries indented by two spaces",
            );
            index = field_end.max(index + 1);
            continue;
        }
        let Some(mapping) = envelope_mapping(line) else {
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::InvalidStructure,
                EnvelopeIssueSeverity::Error,
                line.start..line.end,
                "weftext content is not a canonical field mapping",
            );
            index = field_end.max(index + 1);
            continue;
        };
        if seen.iter().any(|key| key == mapping.key) {
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::DuplicateField,
                EnvelopeIssueSeverity::Error,
                mapping.key_range.clone(),
                "a weftext field is duplicated",
            );
        }
        seen.push(mapping.key.to_owned());

        let kind = envelope_field_kind(mapping.key);
        let mut field = EnvelopeField {
            kind,
            name: mapping.key.to_owned(),
            range: as_u64_range(field_range.clone()),
            key_range: as_u64_range(mapping.key_range.clone()),
            value_range: as_u64_range(mapping.value_range.clone()),
            value: EnvelopeFieldValue::Opaque,
        };
        if kind == EnvelopeFieldKind::Unknown {
            field.value_range = as_u64_range(if mapping.raw.is_empty() {
                line.end..field_range.end
            } else {
                mapping.value_range.clone()
            });
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::UnknownWeftextField,
                EnvelopeIssueSeverity::Warning,
                mapping.key_range.clone(),
                "the unknown weftext field is preserved exactly but has no v1 semantics",
            );
            if envelope_block_has_unsafe_yaml(&mapping, content_lines, index + 1, field_end) {
                push_envelope_issue(
                    issues,
                    EnvelopeIssueCode::UnsafeYamlFeature,
                    EnvelopeIssueSeverity::Error,
                    field_range,
                    "YAML tags, anchors, and aliases are not accepted in the weftext envelope",
                );
            }
            fields.push(field);
            index = field_end.max(index + 1);
            continue;
        }

        if kind == EnvelopeFieldKind::Aliases {
            match parse_envelope_aliases(content_lines, index, field_end, &mapping) {
                Ok(items) => {
                    field.value_range = aliases_value_range(&mapping, &items);
                    field.value = EnvelopeFieldValue::StringList { items };
                }
                Err(error) => {
                    push_scalar_issue(
                        issues,
                        error,
                        field_range.clone(),
                        "weftext.aliases must be an ordered list of unique non-empty strings",
                    );
                }
            }
        } else if mapping.raw.is_empty()
            || content_lines[index + 1..field_end].iter().any(|line| {
                let trimmed = line.text.trim();
                !trimmed.is_empty() && !trimmed.starts_with('#')
            })
        {
            push_envelope_issue(
                issues,
                EnvelopeIssueCode::InvalidStructure,
                EnvelopeIssueSeverity::Error,
                field_range.clone(),
                "this canonical weftext field must contain one literal scalar",
            );
        } else {
            match decode_envelope_scalar(mapping.raw) {
                Ok(value) => {
                    field.value = EnvelopeFieldValue::Scalar {
                        value: value.clone(),
                    };
                    validate_envelope_scalar(kind, &value, &field.value_range, issues);
                }
                Err(error) => push_scalar_issue(
                    issues,
                    error,
                    mapping.value_range.clone(),
                    "the canonical weftext field does not contain a literal scalar",
                ),
            }
        }
        fields.push(field);
        index = field_end.max(index + 1);
    }
}

fn envelope_block_end(lines: &[Line<'_>], start: usize, owner_indent: usize) -> usize {
    let mut next = start + 1;
    while next < lines.len() {
        let trimmed = lines[next].text.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && envelope_indentation(lines[next].text) <= owner_indent
        {
            break;
        }
        next += 1;
    }
    next
}

fn envelope_lines_end(lines: &[Line<'_>], end: usize, fallback: usize) -> usize {
    end.checked_sub(1)
        .and_then(|index| lines.get(index))
        .map_or(fallback, |line| line.full_end)
}

fn envelope_indentation(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

fn envelope_mapping<'a>(line: &Line<'a>) -> Option<EnvelopeMapping<'a>> {
    let indent = envelope_indentation(line.text);
    let content = &line.text[indent..];
    let colon = content.find(':')?;
    let raw_key = &content[..colon];
    let key = raw_key.trim();
    if key.is_empty()
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    {
        return None;
    }
    let key_leading = raw_key.len() - raw_key.trim_start().len();
    let key_start = line.start + indent + key_leading;
    let after_colon = &content[colon + 1..];
    let value_leading = after_colon.len() - after_colon.trim_start_matches(' ').len();
    let raw = after_colon.trim();
    let value_start = line.start + indent + colon + 1 + value_leading;
    Some(EnvelopeMapping {
        key,
        key_range: key_start..key_start + key.len(),
        raw,
        value_range: value_start..value_start + raw.len(),
    })
}

fn envelope_field_kind(name: &str) -> EnvelopeFieldKind {
    match name {
        "id" => EnvelopeFieldKind::Id,
        "icon" => EnvelopeFieldKind::Icon,
        "aliases" => EnvelopeFieldKind::Aliases,
        "child_sort" => EnvelopeFieldKind::ChildSort,
        "child_sort_direction" => EnvelopeFieldKind::ChildSortDirection,
        "sibling_rank" => EnvelopeFieldKind::SiblingRank,
        "adjacent_heading_body" => EnvelopeFieldKind::AdjacentHeadingBody,
        _ => EnvelopeFieldKind::Unknown,
    }
}

fn decode_envelope_scalar(raw: &str) -> Result<String, EnvelopeScalarError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(EnvelopeScalarError::Invalid);
    }
    if contains_unsafe_yaml_feature(raw) {
        return Err(EnvelopeScalarError::UnsafeYamlFeature);
    }
    if raw.starts_with(['|', '>', '@', '`']) {
        return Err(EnvelopeScalarError::Invalid);
    }
    if raw.starts_with('"') {
        return decode_double_quoted_envelope_scalar(raw);
    }
    if raw.starts_with('\'') {
        if !raw.ends_with('\'') || raw.len() < 2 {
            return Err(EnvelopeScalarError::Invalid);
        }
        let inner = &raw[1..raw.len() - 1];
        let mut characters = inner.chars().peekable();
        while let Some(character) = characters.next() {
            if character == '\'' && characters.next_if_eq(&'\'').is_none() {
                return Err(EnvelopeScalarError::Invalid);
            }
        }
        return Ok(inner.replace("''", "'"));
    }
    if raw.contains(['#', '"', '\'', '[', ']', '{', '}']) || raw.contains(": ") {
        return Err(EnvelopeScalarError::Invalid);
    }
    if matches!(
        raw.to_ascii_lowercase().as_str(),
        "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "~"
    ) {
        return Err(EnvelopeScalarError::Invalid);
    }
    Ok(raw.to_owned())
}

fn decode_double_quoted_envelope_scalar(raw: &str) -> Result<String, EnvelopeScalarError> {
    if !raw.ends_with('"') || raw.len() < 2 {
        return Err(EnvelopeScalarError::Invalid);
    }
    let mut decoded = String::new();
    let mut characters = raw[1..raw.len() - 1].chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        let escaped = characters.next().ok_or(EnvelopeScalarError::Invalid)?;
        match escaped {
            '"' | '\\' | '/' => decoded.push(escaped),
            'b' => decoded.push('\u{0008}'),
            'f' => decoded.push('\u{000c}'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'u' => {
                let mut scalar = 0_u32;
                for _ in 0..4 {
                    scalar = scalar
                        .checked_mul(16)
                        .and_then(|value| {
                            characters
                                .next()
                                .and_then(|digit| digit.to_digit(16))
                                .and_then(|digit| value.checked_add(digit))
                        })
                        .ok_or(EnvelopeScalarError::Invalid)?;
                }
                decoded.push(char::from_u32(scalar).ok_or(EnvelopeScalarError::Invalid)?);
            }
            _ => return Err(EnvelopeScalarError::Invalid),
        }
    }
    Ok(decoded)
}

fn parse_envelope_aliases(
    lines: &[Line<'_>],
    field_index: usize,
    field_end: usize,
    mapping: &EnvelopeMapping<'_>,
) -> Result<Vec<EnvelopeListItem>, EnvelopeScalarError> {
    let mut items = if mapping.raw.is_empty() {
        let mut items = Vec::new();
        for line in &lines[field_index + 1..field_end] {
            let trimmed = line.text.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if line.text.contains('\t') {
                return Err(EnvelopeScalarError::UnsafeYamlFeature);
            }
            if envelope_indentation(line.text) != 4 {
                return Err(EnvelopeScalarError::Invalid);
            }
            let raw = trimmed
                .strip_prefix("- ")
                .ok_or(EnvelopeScalarError::Invalid)?;
            let raw_start = line.start + line.text.find(raw).ok_or(EnvelopeScalarError::Invalid)?;
            let value = decode_envelope_scalar(raw)?;
            if value.is_empty() {
                return Err(EnvelopeScalarError::Invalid);
            }
            items.push(EnvelopeListItem {
                range: as_u64_range(line.start..line.full_end),
                value_range: as_u64_range(raw_start..raw_start + raw.len()),
                value,
            });
        }
        if items.is_empty() {
            return Err(EnvelopeScalarError::Invalid);
        }
        items
    } else {
        parse_flow_envelope_aliases(mapping.raw, mapping.value_range.start)?
    };
    if items.len() > 256 {
        return Err(EnvelopeScalarError::Invalid);
    }
    let mut unique = Vec::<String>::new();
    for item in &items {
        if !is_valid_envelope_alias(&item.value) || unique.iter().any(|value| value == &item.value)
        {
            return Err(EnvelopeScalarError::Invalid);
        }
        unique.push(item.value.clone());
    }
    items.shrink_to_fit();
    Ok(items)
}

fn parse_flow_envelope_aliases(
    raw: &str,
    raw_start: usize,
) -> Result<Vec<EnvelopeListItem>, EnvelopeScalarError> {
    if !raw.starts_with('[') || !raw.ends_with(']') {
        return Err(EnvelopeScalarError::Invalid);
    }
    let inner = &raw[1..raw.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Vec::new());
    }
    let mut spans = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    let mut characters = inner.char_indices().peekable();
    while let Some((offset, character)) = characters.next() {
        match quote {
            Some('"') => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    quote = None;
                }
            }
            Some('\'') => {
                if character == '\'' {
                    if characters.peek().is_some_and(|(_, next)| *next == '\'') {
                        let _ = characters.next();
                    } else {
                        quote = None;
                    }
                }
            }
            Some(_) => return Err(EnvelopeScalarError::Invalid),
            None => match character {
                '"' | '\'' => quote = Some(character),
                ',' => {
                    spans.push(start..offset);
                    start = offset + 1;
                }
                '[' | ']' | '{' | '}' => return Err(EnvelopeScalarError::Invalid),
                _ => {}
            },
        }
    }
    if quote.is_some() || escaped {
        return Err(EnvelopeScalarError::Invalid);
    }
    spans.push(start..inner.len());
    spans
        .into_iter()
        .map(|span| {
            let segment = &inner[span.clone()];
            let leading = segment.len() - segment.trim_start().len();
            let trailing = segment.trim_end().len();
            let value_start = raw_start + 1 + span.start + leading;
            let value_end = raw_start + 1 + span.start + trailing;
            let exact = &inner[span.start + leading..span.start + trailing];
            let value = decode_envelope_scalar(exact)?;
            if value.is_empty() {
                return Err(EnvelopeScalarError::Invalid);
            }
            Ok(EnvelopeListItem {
                range: as_u64_range(value_start..value_end),
                value_range: as_u64_range(value_start..value_end),
                value,
            })
        })
        .collect()
}

fn aliases_value_range(mapping: &EnvelopeMapping<'_>, items: &[EnvelopeListItem]) -> Range<u64> {
    if !mapping.raw.is_empty() {
        return as_u64_range(mapping.value_range.clone());
    }
    items.first().zip(items.last()).map_or_else(
        || as_u64_range(mapping.value_range.clone()),
        |(first, last)| first.range.start..last.range.end,
    )
}

fn validate_envelope_scalar(
    kind: EnvelopeFieldKind,
    value: &str,
    range: &Range<u64>,
    issues: &mut Vec<EnvelopeIssue>,
) {
    let valid = match kind {
        EnvelopeFieldKind::Id => Uuid::parse_str(value).is_ok_and(|uuid| {
            uuid.get_version_num() == 4 && uuid.hyphenated().to_string() == value
        }),
        EnvelopeFieldKind::Icon => is_canonical_envelope_icon_scalar(value),
        EnvelopeFieldKind::ChildSort => matches!(value, "name" | "manual"),
        EnvelopeFieldKind::ChildSortDirection => matches!(value, "ascending" | "descending"),
        EnvelopeFieldKind::SiblingRank => value.parse::<u64>().is_ok_and(|rank| rank > 0),
        EnvelopeFieldKind::AdjacentHeadingBody => matches!(value, "run_in" | "separate"),
        EnvelopeFieldKind::Aliases | EnvelopeFieldKind::Unknown => true,
    };
    if !valid {
        let start = usize::try_from(range.start).unwrap_or(0);
        let end = usize::try_from(range.end).unwrap_or(start);
        push_envelope_issue(
            issues,
            EnvelopeIssueCode::InvalidValue,
            EnvelopeIssueSeverity::Error,
            start..end,
            "the weftext field value is outside the canonical v1 vocabulary",
        );
    }
}

/// Returns whether a decoded literal is one canonical v1 node-icon scalar.
///
/// Core presentation delegates here; it does not maintain another emoji/token acceptance grammar.
#[must_use]
pub fn is_canonical_envelope_icon_scalar(value: &str) -> bool {
    if let Some(name) = value.strip_prefix("weftext:") {
        return !name.is_empty()
            && name.len() <= 64
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && name
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_alphanumeric)
            && name
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric);
    }
    let scalars = value.chars().map(u32::from).collect::<Vec<_>>();
    if scalars.is_empty() || scalars.len() > 16 {
        return false;
    }
    if is_keycap_emoji_sequence(&scalars) {
        return true;
    }
    if scalars.iter().all(|scalar| is_regional_indicator(*scalar)) {
        return scalars.len() == 2;
    }
    let mut expect_base = true;
    let mut saw_base = false;
    for scalar in scalars {
        if is_envelope_emoji_base(scalar) {
            if !expect_base {
                return false;
            }
            expect_base = false;
            saw_base = true;
        } else if scalar == 0x200D {
            if expect_base || !saw_base {
                return false;
            }
            expect_base = true;
        } else if is_envelope_emoji_component(scalar) {
            if expect_base || !saw_base {
                return false;
            }
        } else {
            return false;
        }
    }
    saw_base && !expect_base
}

fn is_keycap_emoji_sequence(scalars: &[u32]) -> bool {
    let valid_key = scalars
        .first()
        .is_some_and(|scalar| matches!(scalar, 0x23 | 0x2A | 0x30..=0x39));
    valid_key && (matches!(scalars, [_, 0x20E3]) || matches!(scalars, [_, 0xFE0F, 0x20E3]))
}

fn is_regional_indicator(scalar: u32) -> bool {
    matches!(scalar, 0x1F1E6..=0x1F1FF)
}

fn is_envelope_emoji_base(scalar: u32) -> bool {
    matches!(
        scalar,
        0x00A9
            | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x2194..=0x21FF
            | 0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
            | 0x1F000..=0x1FAFF
            | 0x1FC00..=0x1FFFF
    ) && !is_regional_indicator(scalar)
}

fn is_envelope_emoji_component(scalar: u32) -> bool {
    matches!(
        scalar,
        0xFE0E | 0xFE0F | 0x1F3FB..=0x1F3FF | 0xE0020..=0xE007F
    )
}

fn envelope_block_has_unsafe_yaml(
    mapping: &EnvelopeMapping<'_>,
    lines: &[Line<'_>],
    start: usize,
    end: usize,
) -> bool {
    if contains_unsafe_yaml_feature(mapping.raw) {
        return true;
    }
    lines[start..end]
        .iter()
        .any(|line| contains_unsafe_yaml_feature(line.text))
}

fn contains_unsafe_yaml_feature(raw: &str) -> bool {
    let mut single_quoted = false;
    let mut double_quoted = false;
    let mut escaped = false;
    let mut previous = None;
    let mut characters = raw.chars().peekable();
    while let Some(character) = characters.next() {
        if double_quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                double_quoted = false;
            }
            previous = Some(character);
            continue;
        }
        if single_quoted {
            if character == '\'' {
                if characters.peek() == Some(&'\'') {
                    let _ = characters.next();
                } else {
                    single_quoted = false;
                }
            }
            previous = Some(character);
            continue;
        }
        match character {
            '"' => double_quoted = true,
            '\'' => single_quoted = true,
            '#' if previous.is_none_or(|value: char| value.is_whitespace()) => break,
            '&' | '*' | '!'
                if previous.is_none_or(|value: char| {
                    value.is_whitespace() || "[,{}:-?".contains(value)
                }) =>
            {
                return true;
            }
            _ => {}
        }
        previous = Some(character);
    }
    false
}

fn push_scalar_issue(
    issues: &mut Vec<EnvelopeIssue>,
    error: EnvelopeScalarError,
    range: Range<usize>,
    invalid_message: &str,
) {
    match error {
        EnvelopeScalarError::Invalid => push_envelope_issue(
            issues,
            EnvelopeIssueCode::InvalidValue,
            EnvelopeIssueSeverity::Error,
            range,
            invalid_message,
        ),
        EnvelopeScalarError::UnsafeYamlFeature => push_envelope_issue(
            issues,
            EnvelopeIssueCode::UnsafeYamlFeature,
            EnvelopeIssueSeverity::Error,
            range,
            "YAML tags, anchors, and aliases are not accepted in the weftext envelope",
        ),
    }
}

fn push_envelope_issue(
    issues: &mut Vec<EnvelopeIssue>,
    code: EnvelopeIssueCode,
    severity: EnvelopeIssueSeverity,
    range: Range<usize>,
    message: &str,
) {
    issues.push(EnvelopeIssue {
        code,
        severity,
        range: as_u64_range(range),
        message: message.to_owned(),
    });
}

#[allow(clippy::too_many_arguments)]
fn parse_native_ascii_doc(
    source: &str,
    body: &str,
    body_offset: usize,
    blocks: &mut Vec<Block>,
    checklists: &mut Vec<ChecklistEvidence>,
    inlines: &mut Vec<InlineSemantic>,
    protected: &mut Vec<Range<u64>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if body.is_empty() {
        return;
    }
    let bump = Bump::new();
    let mut parser = Parser::from_str(body, SourceFile::Tmp, &bump);
    let mut settings = JobSettings::secure();
    settings.strict = false;
    parser.apply_job_settings(settings);
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parser.parse())) {
        Ok(Ok(result)) => {
            let parser_branch_locations_are_clean = result.warnings.is_empty();
            for warning in result.warnings {
                diagnostics.push(map_parser_diagnostic(source, body_offset, warning));
            }
            if let Some(title) = result.document.title()
                && let Some(location) = title.main.loc().and_then(|loc| coalesce_location(&loc))
            {
                collect_inline_passthroughs(&title.main, source, body_offset, protected, inlines);
                push_document_title(blocks, location, &title.main, source, body_offset);
            }
            let checklist_start = checklists.len();
            collect_document_checklists(&result.document.content, source, body_offset, checklists);
            if !parser_branch_locations_are_clean {
                for checklist in &mut checklists[checklist_start..] {
                    checklist.parser_occurrence.branch_range = None;
                    checklist.parser_occurrence.branch_complete = false;
                    checklist.parser_occurrence.promotion_branch = None;
                }
            }
            collect_doc_content(
                &result.document.content,
                source,
                body_offset,
                blocks,
                protected,
                inlines,
            );
        }
        Ok(Err(errors)) => {
            diagnostics.extend(
                errors
                    .into_iter()
                    .map(|error| map_parser_error(source, body_offset, error)),
            );
            protected.push(as_u64_range(body_offset..source.len()));
        }
        Err(_) => {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::ParserError,
                range: as_u64_range(body_offset..source.len()),
                message: "AsciiDoc parser aborted on this source; processing failed closed"
                    .to_owned(),
            });
            protected.push(as_u64_range(body_offset..source.len()));
        }
    }
}

fn collect_document_checklists(
    content: &DocContent<'_>,
    source: &str,
    offset: usize,
    checklists: &mut Vec<ChecklistEvidence>,
) {
    let mut root_list_ordinal = 0_u32;
    match content {
        DocContent::Blocks(blocks) => collect_root_checklist_blocks(
            blocks,
            source,
            offset,
            checklists,
            &mut root_list_ordinal,
        ),
        DocContent::Sectioned { preamble, sections } => {
            if let Some(preamble) = preamble {
                collect_root_checklist_blocks(
                    preamble,
                    source,
                    offset,
                    checklists,
                    &mut root_list_ordinal,
                );
            }
            for section in sections {
                collect_root_checklist_section(
                    section,
                    source,
                    offset,
                    checklists,
                    &mut root_list_ordinal,
                );
            }
        }
    }
}

fn collect_root_checklist_section(
    section: &Section<'_>,
    source: &str,
    offset: usize,
    checklists: &mut Vec<ChecklistEvidence>,
    root_list_ordinal: &mut u32,
) {
    collect_root_checklist_blocks(
        &section.blocks,
        source,
        offset,
        checklists,
        root_list_ordinal,
    );
}

fn collect_root_checklist_blocks(
    blocks: &[AsciiDocBlock<'_>],
    source: &str,
    offset: usize,
    checklists: &mut Vec<ChecklistEvidence>,
    root_list_ordinal: &mut u32,
) {
    for block in blocks {
        match &block.content {
            BlockContent::List { depth, items, .. } => {
                let path = vec![*root_list_ordinal];
                *root_list_ordinal = root_list_ordinal.saturating_add(1);
                collect_checklist_items(items, *depth, source, offset, &path, checklists);
            }
            BlockContent::Compound(children) => collect_root_checklist_blocks(
                children,
                source,
                offset,
                checklists,
                root_list_ordinal,
            ),
            BlockContent::Section(section) => collect_root_checklist_section(
                section,
                source,
                offset,
                checklists,
                root_list_ordinal,
            ),
            _ => {}
        }
    }
}

fn collect_checklist_items(
    items: &[asciidork_ast::ListItem<'_>],
    depth: u8,
    source: &str,
    offset: usize,
    list_path: &[u32],
    checklists: &mut Vec<ChecklistEvidence>,
) {
    for (item_ordinal, item) in items.iter().enumerate() {
        let mut item_path = list_path.to_vec();
        item_path.push(u32::try_from(item_ordinal).unwrap_or(u32::MAX));
        if let Some(evidence) = checklist_evidence(item, depth, source, offset, &item_path) {
            checklists.push(evidence);
        }
        let mut nested_list_ordinal = 0_u32;
        collect_attached_checklist_blocks(
            &item.blocks,
            source,
            offset,
            &item_path,
            checklists,
            &mut nested_list_ordinal,
        );
    }
}

fn collect_attached_checklist_blocks(
    blocks: &[AsciiDocBlock<'_>],
    source: &str,
    offset: usize,
    parent_path: &[u32],
    checklists: &mut Vec<ChecklistEvidence>,
    nested_list_ordinal: &mut u32,
) {
    for block in blocks {
        match &block.content {
            BlockContent::List { depth, items, .. } => {
                let mut path = parent_path.to_vec();
                path.push(*nested_list_ordinal);
                *nested_list_ordinal = nested_list_ordinal.saturating_add(1);
                collect_checklist_items(items, *depth, source, offset, &path, checklists);
            }
            BlockContent::Compound(children) => collect_attached_checklist_blocks(
                children,
                source,
                offset,
                parent_path,
                checklists,
                nested_list_ordinal,
            ),
            BlockContent::Section(section) => collect_attached_checklist_blocks(
                &section.blocks,
                source,
                offset,
                parent_path,
                checklists,
                nested_list_ordinal,
            ),
            _ => {}
        }
    }
}

fn checklist_evidence(
    item: &asciidork_ast::ListItem<'_>,
    depth: u8,
    source: &str,
    offset: usize,
    parser_ordinal_path: &[u32],
) -> Option<ChecklistEvidence> {
    let ListItemTypeMeta::Checklist(checked, marker) = &item.type_meta else {
        return None;
    };
    let authored_marker = match marker.src.as_str() {
        "[ ]" => ChecklistMarker::Open,
        "[x]" => ChecklistMarker::CheckedX,
        "[*]" => ChecklistMarker::CheckedStar,
        _ => return None,
    };
    let marker_range = offset_source_location(marker.loc, offset, source.len())?;
    let principle_range = item
        .principle
        .loc()
        .and_then(|location| coalesce_location(&location))
        .and_then(|location| offset_source_location(location, offset, source.len()))
        .unwrap_or(marker_range.end..marker_range.end);
    let description_range = trim_leading_horizontal_space(source, principle_range.clone());
    let item_start = (offset + item.marker_src.loc.start as usize).min(source.len());
    let item_end = principle_range.end.max(marker_range.end);
    let item_range = item_start..item_end;
    let promotion_branch =
        checklist_promotion_branch_evidence(item, depth, source, offset, item_start, item_end);
    let branch_range = promotion_branch
        .as_ref()
        .map(|evidence| evidence.source_replacement_range.clone());
    let branch_complete = promotion_branch.is_some();
    let description = inline_visible_text(&item.principle)
        .trim_start_matches([' ', '\t'])
        .to_owned();
    Some(ChecklistEvidence {
        authored_marker,
        state: if *checked {
            ChecklistState::Completed
        } else {
            ChecklistState::Todo
        },
        item_range: as_u64_range(item_range),
        marker_range: as_u64_range(marker_range),
        description_range: as_u64_range(description_range),
        description,
        list_depth: depth,
        parser_occurrence: ChecklistParserOccurrence {
            parser_ordinal_path: parser_ordinal_path.to_vec(),
            branch_range,
            branch_complete,
            promotion_branch,
        },
    })
}

const MAX_CHECKLIST_PROMOTION_BYTES: usize = 4 * 1024 * 1024;
const MAX_CHECKLIST_PROMOTION_NODES: usize = 4_096;
const MAX_CHECKLIST_PROMOTION_DEPTH: usize = 64;

#[derive(Clone, Copy)]
struct ChecklistPromotionBudget {
    nodes_remaining: usize,
}

impl ChecklistPromotionBudget {
    const fn new() -> Self {
        Self {
            nodes_remaining: MAX_CHECKLIST_PROMOTION_NODES,
        }
    }

    fn visit(&mut self, depth: usize) -> Option<()> {
        if depth > MAX_CHECKLIST_PROMOTION_DEPTH || self.nodes_remaining == 0 {
            return None;
        }
        self.nodes_remaining -= 1;
        Some(())
    }
}

#[allow(clippy::too_many_arguments)]
fn checklist_promotion_branch_evidence(
    item: &asciidork_ast::ListItem<'_>,
    selected_depth: u8,
    source: &str,
    offset: usize,
    item_start: usize,
    item_end: usize,
) -> Option<ChecklistPromotionBranchEvidence> {
    let principal_end = parser_owned_line_end(source, item_end)?;
    let mut branch_end = principal_end;
    let mut edits = vec![ChecklistBranchLiftEdit {
        range: as_u64_range(item_start..principal_end),
        replacement: String::new(),
        kind: ChecklistBranchLiftEditKind::OmitPrincipal,
    }];

    let mut bounds_budget = ChecklistPromotionBudget::new();
    let mut previous_end = principal_end;
    for block in &item.blocks {
        let range = promotion_block_range(block, source, offset, &mut bounds_budget, 0)?;
        let block_end = parser_owned_line_end(source, range.end)?;
        if range.start < previous_end || block_end < range.end {
            return None;
        }

        let connectors = promotion_gap_connectors(source, previous_end..range.start)?;
        let is_nested_list = matches!(block.content, BlockContent::List { .. });
        if connectors.len() > 1 || (!is_nested_list && connectors.len() != 1) {
            return None;
        }
        edits.extend(connectors.into_iter().map(|range| ChecklistBranchLiftEdit {
            range: as_u64_range(range),
            replacement: String::new(),
            kind: ChecklistBranchLiftEditKind::RemoveContinuationConnector,
        }));
        previous_end = block_end;
        branch_end = block_end;
    }

    if next_nonblank_line_is_continuation(source, branch_end) {
        return None;
    }
    if branch_end.checked_sub(item_start)? > MAX_CHECKLIST_PROMOTION_BYTES {
        return None;
    }

    let mut edit_budget = ChecklistPromotionBudget::new();
    collect_descendant_lift_edits(
        &item.blocks,
        selected_depth,
        item.marker,
        source,
        offset,
        &mut edits,
        &mut edit_budget,
        0,
    )?;
    edits.sort_by_key(|edit| (edit.range.start, edit.range.end));

    let lifted_descendant_count = u32::try_from(
        edits
            .iter()
            .filter(|edit| {
                matches!(
                    edit.kind,
                    ChecklistBranchLiftEditKind::DedentDescendant { .. }
                )
            })
            .count(),
    )
    .ok()?;
    let principal_continuation_count = edits
        .iter()
        .filter(|edit| edit.kind == ChecklistBranchLiftEditKind::RemoveContinuationConnector)
        .count();
    let mut continuation_budget = ChecklistPromotionBudget::new();
    let descendant_continuation_count =
        count_descendant_continuations(&item.blocks, source, offset, &mut continuation_budget, 0)?;
    let lifted_continuation_count =
        u32::try_from(principal_continuation_count.checked_add(descendant_continuation_count)?)
            .ok()?;

    let mut evidence = ChecklistPromotionBranchEvidence {
        source_replacement_range: as_u64_range(item_start..branch_end),
        lift_edits: edits,
        lifted_descendant_count,
        lifted_continuation_count,
        context_dependencies: Vec::new(),
    };
    let destination = apply_checklist_lift_recipe(source, &evidence)?;
    evidence.context_dependencies = validate_promoted_checklist_body(&item.blocks, &destination)?;
    Some(evidence)
}

fn count_descendant_continuations(
    blocks: &[AsciiDocBlock<'_>],
    source: &str,
    offset: usize,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<usize> {
    let mut count = 0usize;
    for block in blocks {
        budget.visit(depth)?;
        match &block.content {
            BlockContent::List { items, .. } => {
                for item in items {
                    budget.visit(depth + 1)?;
                    let marker_range =
                        offset_source_location(item.marker_src.loc, offset, source.len())?;
                    let principle_range = item
                        .principle
                        .loc()
                        .and_then(|location| coalesce_location(&location))
                        .and_then(|location| offset_source_location(location, offset, source.len()))
                        .unwrap_or(marker_range.end..marker_range.end);
                    let item_end = principle_range.end.max(marker_range.end);
                    let mut previous_end = parser_owned_line_end(source, item_end)?;
                    for attached in &item.blocks {
                        let range =
                            promotion_block_range(attached, source, offset, budget, depth + 2)?;
                        let block_end = parser_owned_line_end(source, range.end)?;
                        if range.start < previous_end || block_end < range.end {
                            return None;
                        }
                        count = count.checked_add(
                            promotion_gap_connectors(source, previous_end..range.start)?.len(),
                        )?;
                        previous_end = block_end;
                    }
                    count = count.checked_add(count_descendant_continuations(
                        &item.blocks,
                        source,
                        offset,
                        budget,
                        depth + 2,
                    )?)?;
                }
            }
            BlockContent::Compound(children) => {
                count = count.checked_add(count_descendant_continuations(
                    children,
                    source,
                    offset,
                    budget,
                    depth + 1,
                )?)?;
            }
            BlockContent::Section(section) => {
                count = count.checked_add(count_descendant_continuations(
                    &section.blocks,
                    source,
                    offset,
                    budget,
                    depth + 1,
                )?)?;
            }
            _ => {}
        }
    }
    Some(count)
}

fn promotion_block_range(
    block: &AsciiDocBlock<'_>,
    source: &str,
    offset: usize,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<Range<usize>> {
    budget.visit(depth)?;
    let location = coalesce_location(&block.loc)?;
    let mut range = offset_source_location(location, offset, source.len())?;
    let metadata = offset_source_location(block.meta.start_loc, offset, source.len())?;
    range.start = range.start.min(metadata.start);
    range.end = range.end.max(metadata.end);

    match &block.content {
        BlockContent::Compound(children) => {
            extend_promotion_range_from_blocks(
                &mut range,
                children,
                source,
                offset,
                budget,
                depth + 1,
            )?;
        }
        BlockContent::Section(section) => {
            if let Some(heading) = section
                .heading
                .loc()
                .and_then(|location| coalesce_location(&location))
                .and_then(|location| offset_source_location(location, offset, source.len()))
            {
                range.start = range.start.min(heading.start);
                range.end = range.end.max(heading.end);
            }
            extend_promotion_range_from_blocks(
                &mut range,
                &section.blocks,
                source,
                offset,
                budget,
                depth + 1,
            )?;
        }
        BlockContent::List { items, .. } => {
            for item in items {
                budget.visit(depth + 1)?;
                let marker = offset_source_location(item.marker_src.loc, offset, source.len())?;
                range.start = range.start.min(marker.start);
                range.end = range.end.max(marker.end);
                if let Some(principle) = item
                    .principle
                    .loc()
                    .and_then(|location| coalesce_location(&location))
                    .and_then(|location| offset_source_location(location, offset, source.len()))
                {
                    range.end = range.end.max(principle.end);
                }
                extend_promotion_range_from_blocks(
                    &mut range,
                    &item.blocks,
                    source,
                    offset,
                    budget,
                    depth + 2,
                )?;
            }
        }
        _ => {}
    }

    valid_usize_source_range(source, &range).then_some(range)
}

fn extend_promotion_range_from_blocks(
    range: &mut Range<usize>,
    blocks: &[AsciiDocBlock<'_>],
    source: &str,
    offset: usize,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<()> {
    for block in blocks {
        let child = promotion_block_range(block, source, offset, budget, depth)?;
        range.start = range.start.min(child.start);
        range.end = range.end.max(child.end);
    }
    Some(())
}

fn parser_owned_line_end(source: &str, parser_end: usize) -> Option<usize> {
    if parser_end > source.len() || !source.is_char_boundary(parser_end) {
        return None;
    }
    let tail = &source[parser_end..];
    let newline = tail.find('\n');
    let full_end = newline.map_or(source.len(), |relative| parser_end + relative + 1);
    let mut content_end = newline.map_or(source.len(), |relative| parser_end + relative);
    if content_end > parser_end && source.as_bytes()[content_end - 1] == b'\r' {
        content_end -= 1;
    }
    source[parser_end..content_end]
        .bytes()
        .all(|byte| matches!(byte, b' ' | b'\t'))
        .then_some(full_end)
}

fn promotion_gap_connectors(source: &str, gap: Range<usize>) -> Option<Vec<Range<usize>>> {
    if !valid_usize_source_range(source, &gap) {
        return None;
    }
    let mut connectors = Vec::new();
    let mut cursor = gap.start;
    while cursor < gap.end {
        let relative_newline = source[cursor..gap.end].find('\n');
        let full_end = relative_newline.map_or(gap.end, |relative| cursor + relative + 1);
        let mut content_end = relative_newline.map_or(gap.end, |relative| cursor + relative);
        if content_end > cursor && source.as_bytes()[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        let line = &source[cursor..content_end];
        if line == "+" {
            connectors.push(cursor..full_end);
        } else if !line.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
            return None;
        }
        cursor = full_end;
    }
    (cursor == gap.end).then_some(connectors)
}

fn next_nonblank_line_is_continuation(source: &str, start: usize) -> bool {
    if start >= source.len() || !source.is_char_boundary(start) {
        return false;
    }
    let mut cursor = start;
    while cursor < source.len() {
        let relative_newline = source[cursor..].find('\n');
        let full_end = relative_newline.map_or(source.len(), |relative| cursor + relative + 1);
        let mut content_end = relative_newline.map_or(source.len(), |relative| cursor + relative);
        if content_end > cursor && source.as_bytes()[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        let line = &source[cursor..content_end];
        if line == "+" {
            return true;
        }
        if !line.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
            return false;
        }
        cursor = full_end;
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn collect_descendant_lift_edits(
    blocks: &[AsciiDocBlock<'_>],
    selected_depth: u8,
    selected_marker: AsciiDocListMarker,
    source: &str,
    offset: usize,
    edits: &mut Vec<ChecklistBranchLiftEdit>,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<()> {
    for block in blocks {
        budget.visit(depth)?;
        match &block.content {
            BlockContent::List {
                depth: list_depth,
                items,
                ..
            } => {
                if *list_depth <= selected_depth {
                    return None;
                }
                let to_depth = list_depth.checked_sub(1)?;
                for item in items {
                    budget.visit(depth + 1)?;
                    let marker_range =
                        offset_source_location(item.marker_src.loc, offset, source.len())?;
                    let marker_source = source.get(marker_range.clone())?;
                    let replacement =
                        lifted_list_marker(item.marker, selected_marker, marker_source)?;
                    edits.push(ChecklistBranchLiftEdit {
                        range: as_u64_range(marker_range),
                        replacement,
                        kind: ChecklistBranchLiftEditKind::DedentDescendant {
                            from_depth: *list_depth,
                            to_depth,
                        },
                    });
                    collect_descendant_lift_edits(
                        &item.blocks,
                        selected_depth,
                        selected_marker,
                        source,
                        offset,
                        edits,
                        budget,
                        depth + 2,
                    )?;
                }
            }
            BlockContent::Compound(children) => collect_descendant_lift_edits(
                children,
                selected_depth,
                selected_marker,
                source,
                offset,
                edits,
                budget,
                depth + 1,
            )?,
            BlockContent::Section(section) => collect_descendant_lift_edits(
                &section.blocks,
                selected_depth,
                selected_marker,
                source,
                offset,
                edits,
                budget,
                depth + 1,
            )?,
            _ => {}
        }
    }
    Some(())
}

fn lifted_list_marker(
    marker: AsciiDocListMarker,
    selected_marker: AsciiDocListMarker,
    source: &str,
) -> Option<String> {
    match marker {
        AsciiDocListMarker::Star(count) => {
            let replacement_count = if matches!(selected_marker, AsciiDocListMarker::Star(_)) {
                count.saturating_sub(1).max(1)
            } else {
                count
            };
            (source.len() == usize::from(count) && source.bytes().all(|byte| byte == b'*'))
                .then(|| "*".repeat(usize::from(replacement_count)))
        }
        AsciiDocListMarker::Dot(count) => (source.len() == usize::from(count)
            && source.bytes().all(|byte| byte == b'.'))
        .then(|| source.to_owned()),
        AsciiDocListMarker::Dash => (source == "-").then(|| source.to_owned()),
        AsciiDocListMarker::Digits(_) => source
            .strip_suffix('.')
            .filter(|digits| !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
            .map(|_| source.to_owned()),
        AsciiDocListMarker::Colons(count) => (source.len() == usize::from(count)
            && source.bytes().all(|byte| byte == b':'))
        .then(|| source.to_owned()),
        AsciiDocListMarker::SemiColons => (source == ";;").then(|| source.to_owned()),
        AsciiDocListMarker::Callout(_) => None,
    }
}

fn apply_checklist_lift_recipe(
    source: &str,
    evidence: &ChecklistPromotionBranchEvidence,
) -> Option<String> {
    let branch = u64_range_as_usize(&evidence.source_replacement_range)?;
    if !valid_usize_source_range(source, &branch)
        || branch.end.checked_sub(branch.start)? > MAX_CHECKLIST_PROMOTION_BYTES
        || evidence.lift_edits.len() > MAX_CHECKLIST_PROMOTION_NODES
    {
        return None;
    }

    let mut output = String::with_capacity(branch.end - branch.start);
    let mut cursor = branch.start;
    for edit in &evidence.lift_edits {
        let range = u64_range_as_usize(&edit.range)?;
        if !valid_usize_source_range(source, &range)
            || range.start < cursor
            || range.end > branch.end
        {
            return None;
        }
        output.push_str(source.get(cursor..range.start)?);
        output.push_str(&edit.replacement);
        cursor = range.end;
    }
    output.push_str(source.get(cursor..branch.end)?);
    Some(output)
}

fn u64_range_as_usize(range: &Range<u64>) -> Option<Range<usize>> {
    Some(usize::try_from(range.start).ok()?..usize::try_from(range.end).ok()?)
}

fn valid_usize_source_range(source: &str, range: &Range<usize>) -> bool {
    range.start <= range.end
        && range.end <= source.len()
        && source.is_char_boundary(range.start)
        && source.is_char_boundary(range.end)
}

#[derive(Debug, Eq, PartialEq)]
enum ChecklistPromotionSemanticToken {
    Block(u8),
    Compound(usize),
    Simple(String),
    Verbatim,
    Raw,
    Empty(u8, String),
    Table,
    Section(u8, String, usize),
    DocumentAttribute(String),
    QuotedParagraph(String, String, Option<String>),
    List(u8, usize),
    ListItem(u8, Option<bool>, String, usize),
    End,
}

fn validate_promoted_checklist_body(
    original_blocks: &[AsciiDocBlock<'_>],
    destination: &str,
) -> Option<Vec<ChecklistPromotionContextDependency>> {
    if destination.len() > MAX_CHECKLIST_PROMOTION_BYTES {
        return None;
    }

    let mut original_budget = ChecklistPromotionBudget::new();
    let mut original_signature = Vec::new();
    checklist_promotion_signature_blocks(
        original_blocks,
        &mut original_signature,
        &mut original_budget,
        0,
    )?;

    let bump = Bump::new();
    let mut parser = Parser::from_str(destination, SourceFile::Tmp, &bump);
    let mut settings = JobSettings::secure();
    settings.strict = true;
    parser.apply_job_settings(settings);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parser.parse()))
        .ok()?
        .ok()?;
    if !result.warnings.is_empty() || result.document.header.is_some() {
        return None;
    }
    let DocContent::Blocks(destination_blocks) = &result.document.content else {
        return None;
    };

    let mut destination_budget = ChecklistPromotionBudget::new();
    let mut destination_signature = Vec::new();
    checklist_promotion_signature_blocks(
        destination_blocks,
        &mut destination_signature,
        &mut destination_budget,
        0,
    )?;
    if original_signature != destination_signature {
        return None;
    }
    Some(checklist_promotion_context_dependencies(
        destination,
        destination_blocks,
    ))
}

/// Inventories parser-owned context dependencies in an exact checklist principal fragment.
///
/// Returned ranges are rebased to the complete source. This is a read-only evidence helper: an
/// invalid UTF-8 boundary, reversed range, or oversized fragment returns `None` rather than
/// weakening promotion safety.
#[must_use]
pub fn checklist_promotion_principal_context_dependencies(
    source: &str,
    principal_range: Range<u64>,
) -> Option<Vec<ChecklistPromotionContextDependency>> {
    let start = usize::try_from(principal_range.start).ok()?;
    let end = usize::try_from(principal_range.end).ok()?;
    if start > end
        || end > source.len()
        || !source.is_char_boundary(start)
        || !source.is_char_boundary(end)
        || end.checked_sub(start)? > MAX_CHECKLIST_PROMOTION_BYTES
    {
        return None;
    }
    let fragment = source.get(start..end)?;
    let offset = u64::try_from(start).ok()?;
    let mut dependencies = checklist_promotion_context_dependencies(fragment, &[]);
    for dependency in &mut dependencies {
        dependency.range = dependency.range.start.checked_add(offset)?
            ..dependency.range.end.checked_add(offset)?;
    }
    Some(dependencies)
}

fn checklist_promotion_context_dependencies(
    source: &str,
    native: &[AsciiDocBlock<'_>],
) -> Vec<ChecklistPromotionContextDependency> {
    let mut blocks = Vec::new();
    let mut protected = Vec::new();
    let mut native_inlines = Vec::new();
    collect_native_blocks(
        native,
        source,
        0,
        &mut blocks,
        &mut protected,
        &mut native_inlines,
    );
    normalize_ranges(&mut protected);
    let mut diagnostics = Vec::new();
    let (_, mut inlines) = scan_inline_semantics(source, &protected, &mut diagnostics);
    inlines.extend(native_inlines);
    normalize_inline_semantics(source, &mut inlines, &mut diagnostics);

    let mut dependencies = Vec::new();
    append_promotion_block_dependencies(&blocks, &mut dependencies);
    append_promotion_inline_dependencies(&inlines, &mut dependencies);
    append_promotion_effect_dependencies(source, &protected, &mut dependencies);
    dependencies.extend(promotion_attribute_references(source, &protected));
    dependencies.sort_by(|left, right| {
        (
            left.range.start,
            left.range.end,
            left.kind as u8,
            &left.target,
        )
            .cmp(&(
                right.range.start,
                right.range.end,
                right.kind as u8,
                &right.target,
            ))
    });
    dependencies.dedup();
    dependencies
}

fn append_promotion_block_dependencies(
    blocks: &[Block],
    dependencies: &mut Vec<ChecklistPromotionContextDependency>,
) {
    for block in blocks {
        if let BlockSemantic::Image { target, .. } = &block.semantic
            && !promotion_location_independent_locator(target)
        {
            dependencies.push(ChecklistPromotionContextDependency {
                kind: ChecklistPromotionContextDependencyKind::RelativeLocator,
                range: block.range.clone(),
                target: Some(target.clone()),
            });
        }
        if let Some(block_id) = &block.block_id {
            dependencies.push(ChecklistPromotionContextDependency {
                kind: ChecklistPromotionContextDependencyKind::ExplicitAnchor,
                range: block.range.clone(),
                target: Some(block_id.clone()),
            });
        }
        if block.kind == BlockKind::Heading {
            dependencies.push(ChecklistPromotionContextDependency {
                kind: ChecklistPromotionContextDependencyKind::ImplicitHeadingAnchor,
                range: block.range.clone(),
                target: None,
            });
        }
    }
}

fn append_promotion_inline_dependencies(
    inlines: &[InlineSemantic],
    dependencies: &mut Vec<ChecklistPromotionContextDependency>,
) {
    for inline in inlines {
        let kind = match inline.kind {
            InlineKind::Image | InlineKind::Xref | InlineKind::NativeLink
                if inline
                    .target
                    .as_deref()
                    .is_some_and(|target| !promotion_location_independent_locator(target)) =>
            {
                Some(ChecklistPromotionContextDependencyKind::RelativeLocator)
            }
            InlineKind::Anchor => Some(ChecklistPromotionContextDependencyKind::ExplicitAnchor),
            InlineKind::Footnote if inline.target.is_some() => {
                Some(ChecklistPromotionContextDependencyKind::NamedFootnote)
            }
            InlineKind::Endnote if inline.target.is_some() => {
                Some(ChecklistPromotionContextDependencyKind::NamedEndnote)
            }
            _ => None,
        };
        if let Some(kind) = kind {
            dependencies.push(ChecklistPromotionContextDependency {
                kind,
                range: inline.range.clone(),
                target: inline.target.clone(),
            });
        }
    }
}

fn append_promotion_effect_dependencies(
    source: &str,
    protected: &[Range<u64>],
    dependencies: &mut Vec<ChecklistPromotionContextDependency>,
) {
    let mut effects = Vec::new();
    collect_directive_effects(source, 0, protected, &mut effects);
    for effect in effects {
        let kind = match effect.origin {
            EffectOrigin::IncludeDirective
                if effect
                    .target
                    .as_deref()
                    .is_some_and(|target| !promotion_location_independent_locator(target)) =>
            {
                Some(ChecklistPromotionContextDependencyKind::RelativeLocator)
            }
            EffectOrigin::ConditionalDirective => {
                Some(ChecklistPromotionContextDependencyKind::ConditionalDirective)
            }
            _ => None,
        };
        if let Some(kind) = kind {
            dependencies.push(ChecklistPromotionContextDependency {
                kind,
                range: effect.range,
                target: effect.target,
            });
        }
    }
}

fn promotion_attribute_references(
    source: &str,
    protected: &[Range<u64>],
) -> Vec<ChecklistPromotionContextDependency> {
    let mut dependencies = Vec::new();
    let mut cursor = 0usize;
    while let Some(relative_start) = source[cursor..].find('{') {
        let start = cursor + relative_start;
        let Some(relative_end) = source[start + 1..].find('}') else {
            break;
        };
        let end = start + 1 + relative_end + 1;
        cursor = end;
        if start > 0 && source.as_bytes()[start - 1] == b'\\' || inside_ranges(start, protected) {
            continue;
        }
        let name = &source[start + 1..end - 1];
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '-')
            })
        {
            continue;
        }
        dependencies.push(ChecklistPromotionContextDependency {
            kind: ChecklistPromotionContextDependencyKind::DocumentAttributeReference,
            range: as_u64_range(start..end),
            target: Some(name.to_owned()),
        });
    }
    dependencies
}

fn promotion_location_independent_locator(target: &str) -> bool {
    if target.starts_with('/') || target.starts_with('\\') {
        return true;
    }
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut characters = scheme.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')
        })
}

fn checklist_promotion_signature_blocks(
    blocks: &[AsciiDocBlock<'_>],
    signature: &mut Vec<ChecklistPromotionSemanticToken>,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<()> {
    for block in blocks {
        checklist_promotion_signature_block(block, signature, budget, depth)?;
    }
    Some(())
}

fn checklist_promotion_signature_block(
    block: &AsciiDocBlock<'_>,
    signature: &mut Vec<ChecklistPromotionSemanticToken>,
    budget: &mut ChecklistPromotionBudget,
    depth: usize,
) -> Option<()> {
    budget.visit(depth)?;
    signature.push(ChecklistPromotionSemanticToken::Block(block.context as u8));
    match &block.content {
        BlockContent::Compound(children) => {
            signature.push(ChecklistPromotionSemanticToken::Compound(children.len()));
            checklist_promotion_signature_blocks(children, signature, budget, depth + 1)?;
        }
        BlockContent::Simple(inlines) => signature.push(ChecklistPromotionSemanticToken::Simple(
            inline_visible_text(inlines),
        )),
        BlockContent::Verbatim => signature.push(ChecklistPromotionSemanticToken::Verbatim),
        BlockContent::Raw => signature.push(ChecklistPromotionSemanticToken::Raw),
        BlockContent::Empty(metadata) => {
            let (kind, content) = match metadata {
                EmptyMetadata::Image { target, .. } => (0, target.src.to_string()),
                EmptyMetadata::DiscreteHeading { content, .. } => (1, inline_visible_text(content)),
                EmptyMetadata::Comment(comment) => (2, comment.src.to_string()),
                EmptyMetadata::None => (3, String::new()),
            };
            signature.push(ChecklistPromotionSemanticToken::Empty(kind, content));
        }
        BlockContent::Table(_) => signature.push(ChecklistPromotionSemanticToken::Table),
        BlockContent::Section(section) => {
            signature.push(ChecklistPromotionSemanticToken::Section(
                section.level,
                inline_visible_text(&section.heading),
                section.blocks.len(),
            ));
            checklist_promotion_signature_blocks(&section.blocks, signature, budget, depth + 1)?;
        }
        BlockContent::DocumentAttribute(name, _) => signature.push(
            ChecklistPromotionSemanticToken::DocumentAttribute(name.clone()),
        ),
        BlockContent::QuotedParagraph { quote, attr, cite } => {
            signature.push(ChecklistPromotionSemanticToken::QuotedParagraph(
                inline_visible_text(quote),
                attr.src.to_string(),
                cite.as_ref().map(|value| value.src.to_string()),
            ));
        }
        BlockContent::List { variant, items, .. } => {
            signature.push(ChecklistPromotionSemanticToken::List(
                *variant as u8,
                items.len(),
            ));
            for item in items {
                budget.visit(depth + 1)?;
                let (kind, checked) = match &item.type_meta {
                    ListItemTypeMeta::Checklist(value, _) => (1, Some(*value)),
                    ListItemTypeMeta::Callout(_) => (2, None),
                    ListItemTypeMeta::DescList { .. } => (3, None),
                    ListItemTypeMeta::None => (0, None),
                };
                signature.push(ChecklistPromotionSemanticToken::ListItem(
                    kind,
                    checked,
                    inline_visible_text(&item.principle),
                    item.blocks.len(),
                ));
                checklist_promotion_signature_blocks(&item.blocks, signature, budget, depth + 2)?;
                signature.push(ChecklistPromotionSemanticToken::End);
            }
        }
    }
    signature.push(ChecklistPromotionSemanticToken::End);
    Some(())
}

fn offset_source_location(
    location: SourceLocation,
    offset: usize,
    source_len: usize,
) -> Option<Range<usize>> {
    if location.include_depth != 0 {
        return None;
    }
    let start = offset.checked_add(location.start as usize)?;
    let end = offset.checked_add(location.end as usize)?;
    (start <= end && end <= source_len).then_some(start..end)
}

fn trim_leading_horizontal_space(source: &str, mut range: Range<usize>) -> Range<usize> {
    let Some(value) = source.get(range.clone()) else {
        return range;
    };
    let count = value
        .bytes()
        .take_while(|byte| matches!(byte, b' ' | b'\t'))
        .count();
    range.start = range.start.saturating_add(count);
    range
}

fn collect_doc_content(
    content: &DocContent<'_>,
    source: &str,
    offset: usize,
    blocks: &mut Vec<Block>,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    match content {
        DocContent::Blocks(native) => {
            collect_native_blocks(native, source, offset, blocks, protected, inlines);
        }
        DocContent::Sectioned { preamble, sections } => {
            if let Some(preamble) = preamble {
                collect_native_blocks(preamble, source, offset, blocks, protected, inlines);
            }
            for section in sections {
                collect_section(section, source, offset, blocks, protected, inlines);
            }
        }
    }
}

fn collect_section(
    section: &Section<'_>,
    source: &str,
    offset: usize,
    blocks: &mut Vec<Block>,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    if let Some(location) = section
        .heading
        .loc()
        .and_then(|loc| coalesce_location(&loc))
    {
        collect_inline_passthroughs(&section.heading, source, offset, protected, inlines);
        push_inline_block(
            blocks,
            BlockKind::Heading,
            location,
            &section.heading,
            source,
            offset,
            Some(section.level),
        );
        if let Some(block) = blocks.last_mut() {
            block.block_id = section.meta.attrs.id().map(|id| id.src.to_string());
            block.roles = section
                .meta
                .attrs
                .roles()
                .map(|role| role.src.to_string())
                .collect();
        }
    }
    collect_native_blocks(&section.blocks, source, offset, blocks, protected, inlines);
}

fn collect_native_blocks(
    native: &[AsciiDocBlock<'_>],
    source: &str,
    offset: usize,
    blocks: &mut Vec<Block>,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    for block in native {
        let kind = native_block_kind(block);
        collect_content_inline_passthroughs(&block.content, source, offset, protected, inlines);
        if let Some(location) = coalesce_location(&block.loc) {
            let range = offset_location(location, offset, source.len());
            let text_range = native_text_range(&block.content)
                .map(|location| offset_location(location, offset, source.len()))
                .or_else(|| native_source_content_range(block, source, offset, &range))
                .unwrap_or_else(|| range.clone());
            let roles = block
                .meta
                .attrs
                .iter()
                .flat_map(|attrs| attrs.roles.iter())
                .map(|role| role.src.to_string())
                .collect();
            push_source_block(
                blocks,
                kind,
                range.clone(),
                text_range,
                source,
                None,
                None,
                roles,
            );
            if let Some(parsed) = blocks.last_mut() {
                parsed.block_id = block.meta.attrs.id().map(|id| id.src.to_string());
                parsed.title = block
                    .meta
                    .title
                    .as_ref()
                    .map(|title| inline_visible_text(title));
                parsed.semantic = native_block_semantic(block, source, offset);
            }
            if matches!(
                kind,
                BlockKind::Listing
                    | BlockKind::Literal
                    | BlockKind::Passthrough
                    | BlockKind::Comment
            ) {
                protected.push(as_u64_range(range));
            }
        }
        match &block.content {
            BlockContent::Compound(children) => {
                collect_native_blocks(children, source, offset, blocks, protected, inlines);
            }
            BlockContent::Section(section) => {
                collect_section(section, source, offset, blocks, protected, inlines);
            }
            _ => {}
        }
    }
}

fn collect_content_inline_passthroughs(
    content: &BlockContent<'_>,
    source: &str,
    offset: usize,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    match content {
        BlockContent::Compound(blocks) => {
            for block in blocks {
                collect_content_inline_passthroughs(
                    &block.content,
                    source,
                    offset,
                    protected,
                    inlines,
                );
            }
        }
        BlockContent::Simple(nodes) => {
            collect_inline_passthroughs(nodes, source, offset, protected, inlines);
        }
        BlockContent::Empty(EmptyMetadata::DiscreteHeading { content, .. }) => {
            collect_inline_passthroughs(content, source, offset, protected, inlines);
        }
        BlockContent::Table(table) => {
            if let Some(row) = &table.header_row {
                collect_row_inline_passthroughs(row, source, offset, protected, inlines);
            }
            for row in &table.rows {
                collect_row_inline_passthroughs(row, source, offset, protected, inlines);
            }
            if let Some(row) = &table.footer_row {
                collect_row_inline_passthroughs(row, source, offset, protected, inlines);
            }
        }
        BlockContent::Section(section) => {
            collect_inline_passthroughs(&section.heading, source, offset, protected, inlines);
            for block in &section.blocks {
                collect_content_inline_passthroughs(
                    &block.content,
                    source,
                    offset,
                    protected,
                    inlines,
                );
            }
        }
        BlockContent::QuotedParagraph { quote, .. } => {
            collect_inline_passthroughs(quote, source, offset, protected, inlines);
        }
        BlockContent::List { items, .. } => {
            for item in items {
                collect_inline_passthroughs(&item.principle, source, offset, protected, inlines);
                if let ListItemTypeMeta::DescList {
                    description,
                    extra_terms,
                } = &item.type_meta
                {
                    if let Some(description) = description {
                        collect_content_inline_passthroughs(
                            &description.content,
                            source,
                            offset,
                            protected,
                            inlines,
                        );
                    }
                    for (term, _) in extra_terms {
                        collect_inline_passthroughs(term, source, offset, protected, inlines);
                    }
                }
                for block in &item.blocks {
                    collect_content_inline_passthroughs(
                        &block.content,
                        source,
                        offset,
                        protected,
                        inlines,
                    );
                }
            }
        }
        BlockContent::Verbatim
        | BlockContent::Raw
        | BlockContent::Empty(_)
        | BlockContent::DocumentAttribute(_, _) => {}
    }
}

fn collect_row_inline_passthroughs(
    row: &asciidork_ast::Row<'_>,
    source: &str,
    offset: usize,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    for cell in &row.cells {
        match &cell.content {
            CellContent::AsciiDoc(document) => {
                if let Some(title) = document.title() {
                    collect_inline_passthroughs(&title.main, source, offset, protected, inlines);
                }
                collect_doc_content_inline_passthroughs(
                    &document.content,
                    source,
                    offset,
                    protected,
                    inlines,
                );
            }
            CellContent::Literal(nodes) => {
                collect_inline_passthroughs(nodes, source, offset, protected, inlines);
            }
            CellContent::Default(paragraphs)
            | CellContent::Emphasis(paragraphs)
            | CellContent::Header(paragraphs)
            | CellContent::Monospace(paragraphs)
            | CellContent::Strong(paragraphs) => {
                for nodes in paragraphs {
                    collect_inline_passthroughs(nodes, source, offset, protected, inlines);
                }
            }
        }
    }
}

fn collect_doc_content_inline_passthroughs(
    content: &DocContent<'_>,
    source: &str,
    offset: usize,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    match content {
        DocContent::Blocks(blocks) => {
            for block in blocks {
                collect_content_inline_passthroughs(
                    &block.content,
                    source,
                    offset,
                    protected,
                    inlines,
                );
            }
        }
        DocContent::Sectioned { preamble, sections } => {
            if let Some(preamble) = preamble {
                for block in preamble {
                    collect_content_inline_passthroughs(
                        &block.content,
                        source,
                        offset,
                        protected,
                        inlines,
                    );
                }
            }
            for section in sections {
                collect_inline_passthroughs(&section.heading, source, offset, protected, inlines);
                for block in &section.blocks {
                    collect_content_inline_passthroughs(
                        &block.content,
                        source,
                        offset,
                        protected,
                        inlines,
                    );
                }
            }
        }
    }
}

fn collect_inline_passthroughs(
    nodes: &InlineNodes<'_>,
    source: &str,
    offset: usize,
    protected: &mut Vec<Range<u64>>,
    inlines: &mut Vec<InlineSemantic>,
) {
    for node in nodes.iter() {
        let range = as_u64_range(offset_location(node.loc, offset, source.len()));
        match &node.content {
            Inline::InlinePassthru(children) => {
                protected.push(range.clone());
                let raw = source.get(
                    usize::try_from(range.start).unwrap_or(0)
                        ..usize::try_from(range.end).unwrap_or(0),
                );
                let kind = if raw.is_some_and(|raw| raw.starts_with("pass:")) {
                    InlineKind::Passthrough
                } else {
                    InlineKind::Monospace
                };
                inlines.push(native_inline_span(
                    kind,
                    range,
                    inline_visible_text(children),
                    Vec::new(),
                ));
            }
            Inline::LitMono(value) => {
                protected.push(range.clone());
                inlines.push(native_inline_span(
                    InlineKind::Monospace,
                    range,
                    value.src.to_string(),
                    Vec::new(),
                ));
            }
            Inline::Bold(children)
            | Inline::Highlight(children)
            | Inline::Italic(children)
            | Inline::Mono(children)
            | Inline::Quote(_, children)
            | Inline::Superscript(children)
            | Inline::Subscript(children) => {
                let kind = match &node.content {
                    Inline::Bold(_) => InlineKind::Bold,
                    Inline::Highlight(_) => InlineKind::Highlight,
                    Inline::Italic(_) => InlineKind::Italic,
                    Inline::Mono(_) => InlineKind::Monospace,
                    Inline::Quote(_, _) => InlineKind::Quoted,
                    Inline::Superscript(_) => InlineKind::Superscript,
                    Inline::Subscript(_) => InlineKind::Subscript,
                    _ => unreachable!("matched inline span"),
                };
                inlines.push(native_inline_span(
                    kind,
                    range,
                    inline_visible_text(children),
                    Vec::new(),
                ));
                collect_inline_passthroughs(children, source, offset, protected, inlines);
            }
            Inline::TextSpan(attrs, children) => {
                inlines.push(native_inline_span(
                    InlineKind::RoleSpan,
                    range,
                    inline_visible_text(children),
                    attrs.roles().map(|role| role.src.to_string()).collect(),
                ));
                collect_inline_passthroughs(children, source, offset, protected, inlines);
            }
            Inline::InlineAnchor(target) | Inline::BiblioAnchor(target) => {
                inlines.push(InlineSemantic {
                    kind: InlineKind::Anchor,
                    range,
                    target_range: None,
                    label_range: None,
                    target: Some(target.to_string()),
                    fragment: None,
                    text: None,
                    notation: None,
                    roles: Vec::new(),
                });
            }
            Inline::Macro(macro_node) => {
                collect_native_macro_semantic(macro_node, range, offset, source, inlines);
            }
            _ => {}
        }
    }
}

fn native_inline_span(
    kind: InlineKind,
    range: Range<u64>,
    text: String,
    roles: Vec<String>,
) -> InlineSemantic {
    InlineSemantic {
        kind,
        range,
        target_range: None,
        label_range: None,
        target: None,
        fragment: None,
        text: Some(text),
        notation: None,
        roles,
    }
}

fn collect_native_macro_semantic(
    macro_node: &asciidork_ast::MacroNode<'_>,
    range: Range<u64>,
    offset: usize,
    source: &str,
    inlines: &mut Vec<InlineSemantic>,
) {
    let (kind, target, target_range, fragment, text) = match macro_node {
        asciidork_ast::MacroNode::Footnote { id, text } => (
            InlineKind::Footnote,
            id.as_ref().map(|id| id.src.to_string()),
            id.as_ref()
                .map(|id| as_u64_range(offset_location(id.loc, offset, source.len()))),
            None,
            text.as_ref().map(inline_visible_text),
        ),
        asciidork_ast::MacroNode::Image { target, attrs, .. } => (
            InlineKind::Image,
            Some(target.src.to_string()),
            Some(as_u64_range(offset_location(
                target.loc,
                offset,
                source.len(),
            ))),
            None,
            attrs.str_positional_at(0).map(str::to_owned),
        ),
        asciidork_ast::MacroNode::Link { target, attrs, .. } => (
            InlineKind::NativeLink,
            Some(target.src.to_string()),
            Some(as_u64_range(offset_location(
                target.loc,
                offset,
                source.len(),
            ))),
            None,
            attrs
                .as_ref()
                .and_then(|attrs| attrs.str_positional_at(0))
                .map(str::to_owned),
        ),
        asciidork_ast::MacroNode::Xref {
            target, linktext, ..
        } => {
            let (target_value, fragment) = target
                .src
                .split_once('#')
                .map_or((target.src.as_str(), None), |(target, fragment)| {
                    (target, Some(fragment.to_owned()))
                });
            (
                InlineKind::Xref,
                Some(target_value.to_owned()),
                Some(as_u64_range(offset_location(
                    target.loc,
                    offset,
                    source.len(),
                ))),
                fragment,
                linktext.as_ref().map(inline_visible_text),
            )
        }
        _ => (
            InlineKind::Unsupported,
            None,
            None,
            None,
            source
                .get(
                    usize::try_from(range.start).unwrap_or(0)
                        ..usize::try_from(range.end).unwrap_or(0),
                )
                .map(str::to_owned),
        ),
    };
    inlines.push(InlineSemantic {
        kind,
        range,
        target_range,
        label_range: None,
        target,
        fragment,
        text,
        notation: None,
        roles: Vec::new(),
    });
}

fn native_text_range(content: &BlockContent<'_>) -> Option<SourceLocation> {
    match content {
        BlockContent::Simple(nodes) => nodes.loc().and_then(|loc| coalesce_location(&loc)),
        BlockContent::QuotedParagraph { quote, .. } => {
            quote.loc().and_then(|loc| coalesce_location(&loc))
        }
        BlockContent::Empty(asciidork_ast::EmptyMetadata::DiscreteHeading { content, .. }) => {
            content.loc().and_then(|loc| coalesce_location(&loc))
        }
        _ => None,
    }
}

fn native_source_content_range(
    block: &AsciiDocBlock<'_>,
    source: &str,
    offset: usize,
    block_range: &Range<usize>,
) -> Option<Range<usize>> {
    if let BlockContent::Empty(EmptyMetadata::Image { target, .. }) = &block.content {
        return Some(offset_location(target.loc, offset, source.len()));
    }
    if !matches!(
        native_block_kind(block),
        BlockKind::Listing
            | BlockKind::Literal
            | BlockKind::Quote
            | BlockKind::Math
            | BlockKind::Mermaid
            | BlockKind::Passthrough
            | BlockKind::Comment
    ) {
        return None;
    }
    delimited_content_range(source, block_range)
}

fn delimited_content_range(source: &str, block_range: &Range<usize>) -> Option<Range<usize>> {
    let candidates = ["----", "....", "++++", "____", "////", "--"];
    let block_lines = lines(source)
        .filter(|line| line.start >= block_range.start && line.full_end <= block_range.end)
        .collect::<Vec<_>>();
    for (index, opening) in block_lines.iter().enumerate() {
        if !candidates.contains(&opening.text) {
            continue;
        }
        if let Some(closing) = block_lines
            .iter()
            .skip(index + 1)
            .find(|line| line.text == opening.text)
        {
            return Some(opening.full_end..closing.start);
        }
    }
    None
}

fn native_block_kind(block: &AsciiDocBlock<'_>) -> BlockKind {
    if block.meta.attrs.has_str_positional("mermaid") {
        return BlockKind::Mermaid;
    }
    if block.meta.attrs.has_str_positional("stem")
        || block.meta.attrs.has_str_positional("latexmath")
    {
        return BlockKind::Math;
    }
    match block.context {
        BlockContext::Paragraph => BlockKind::Paragraph,
        BlockContext::Listing => BlockKind::Listing,
        BlockContext::Literal => BlockKind::Literal,
        BlockContext::BlockQuote | BlockContext::QuotedParagraph | BlockContext::Verse => {
            BlockKind::Quote
        }
        BlockContext::OrderedList | BlockContext::UnorderedList | BlockContext::DescriptionList => {
            BlockKind::List
        }
        BlockContext::Table => BlockKind::Table,
        BlockContext::Image => BlockKind::Image,
        BlockContext::Passthrough => BlockKind::Passthrough,
        BlockContext::Comment => BlockKind::Comment,
        BlockContext::ThematicBreak => BlockKind::ThematicBreak,
        _ => BlockKind::Other,
    }
}

fn native_block_semantic(block: &AsciiDocBlock<'_>, source: &str, offset: usize) -> BlockSemantic {
    let kind = native_block_kind(block);
    match (&block.content, kind) {
        (_, BlockKind::Paragraph) => BlockSemantic::Paragraph,
        (_, BlockKind::Listing) => BlockSemantic::Listing {
            language: block.meta.attrs.source_language().map(str::to_owned),
        },
        (_, BlockKind::Literal) => BlockSemantic::Literal,
        (BlockContent::QuotedParagraph { attr, cite, .. }, BlockKind::Quote) => {
            BlockSemantic::Quote {
                depth: None,
                attribution: (!attr.is_empty()).then(|| attr.src.to_string()),
                citation: cite.as_ref().map(|value| value.src.to_string()),
            }
        }
        (_, BlockKind::Quote) => BlockSemantic::Quote {
            depth: None,
            attribution: block.meta.attrs.str_positional_at(1).map(str::to_owned),
            citation: block.meta.attrs.str_positional_at(2).map(str::to_owned),
        },
        (
            BlockContent::List {
                variant,
                depth,
                items,
            },
            BlockKind::List,
        ) => BlockSemantic::List {
            model: ListModel {
                kind: map_list_kind(*variant),
                depth: *depth,
                items: items
                    .iter()
                    .map(|item| semantic_list_item(item, *depth, source, offset))
                    .collect(),
            },
        },
        (BlockContent::Table(table), BlockKind::Table) => BlockSemantic::Table {
            model: semantic_table(table),
        },
        (BlockContent::Empty(EmptyMetadata::Image { target, attrs }), BlockKind::Image) => {
            BlockSemantic::Image {
                target: target.src.to_string(),
                alt: attrs.str_positional_at(0).map(str::to_owned),
            }
        }
        (_, BlockKind::Math) => BlockSemantic::Math {
            notation: block_math_notation(block, source, offset),
        },
        (_, BlockKind::Mermaid) => BlockSemantic::Mermaid,
        (_, BlockKind::Passthrough) => BlockSemantic::Passthrough,
        (_, BlockKind::Comment) => BlockSemantic::Comment,
        (_, BlockKind::ThematicBreak) => BlockSemantic::ThematicBreak,
        _ => BlockSemantic::Unsupported {
            context: format!("{:?}", block.context),
        },
    }
}

const fn map_list_kind(kind: ListVariant) -> ListKind {
    match kind {
        ListVariant::Ordered => ListKind::Ordered,
        ListVariant::Unordered => ListKind::Unordered,
        ListVariant::Description => ListKind::Description,
        ListVariant::Callout => ListKind::Callout,
    }
}

fn semantic_list_item(
    item: &asciidork_ast::ListItem<'_>,
    depth: u8,
    source: &str,
    offset: usize,
) -> ListItem {
    let principle_range = item
        .principle
        .loc()
        .and_then(|location| coalesce_location(&location))
        .map_or_else(
            || {
                let start = offset + item.marker_src.loc.end as usize;
                start..start
            },
            |location| offset_location(location, offset, source.len()),
        );
    let start = (offset + item.marker_src.loc.start as usize).min(source.len());
    let end = item.last_loc_end().map_or(principle_range.end, |end| {
        (offset + end as usize).min(source.len())
    });
    let mut children = Vec::new();
    let mut unmodeled_continuations = Vec::new();
    for child in &item.blocks {
        if contains_list_content(child) {
            collect_nested_list_items(child, source, offset, &mut children);
        } else if let Some(location) = coalesce_location(&child.loc) {
            unmodeled_continuations.push(as_u64_range(offset_location(
                location,
                offset,
                source.len(),
            )));
        }
    }
    let checked = match &item.type_meta {
        ListItemTypeMeta::Checklist(value, _) => Some(*value),
        _ => None,
    };
    ListItem {
        range: as_u64_range(start..end),
        text_range: as_u64_range(principle_range),
        marker: item.marker_src.src.to_string(),
        text: inline_visible_text(&item.principle),
        depth,
        checked,
        children,
        unmodeled_continuations,
    }
}

fn contains_list_content(block: &AsciiDocBlock<'_>) -> bool {
    match &block.content {
        BlockContent::List { .. } => true,
        BlockContent::Compound(children) => children.iter().any(contains_list_content),
        _ => false,
    }
}

fn collect_nested_list_items(
    block: &AsciiDocBlock<'_>,
    source: &str,
    offset: usize,
    items: &mut Vec<ListItem>,
) {
    match &block.content {
        BlockContent::List {
            depth,
            items: children,
            ..
        } => items.extend(
            children
                .iter()
                .map(|child| semantic_list_item(child, *depth, source, offset)),
        ),
        BlockContent::Compound(children) => {
            for child in children {
                collect_nested_list_items(child, source, offset, items);
            }
        }
        _ => {}
    }
}

fn semantic_table(table: &asciidork_ast::Table<'_>) -> TableModel {
    let header = table.header_row.as_ref().map(semantic_table_row);
    let body = table
        .rows
        .iter()
        .map(semantic_table_row)
        .collect::<Vec<_>>();
    let footer = table.footer_row.as_ref().map(semantic_table_row);
    let column_count = header
        .iter()
        .chain(body.iter())
        .chain(footer.iter())
        .map(|row| {
            row.cells
                .iter()
                .map(|cell| u64::from(cell.column_span))
                .sum::<u64>()
        })
        .max()
        .unwrap_or(0);
    TableModel {
        header,
        body,
        footer,
        column_count,
    }
}

fn semantic_table_row(row: &asciidork_ast::Row<'_>) -> TableRow {
    TableRow {
        cells: row
            .cells
            .iter()
            .map(|cell| TableCell {
                text: cell_visible_text(&cell.content),
                column_span: cell.col_span,
                row_span: cell.row_span,
                style: map_cell_style(&cell.content),
                horizontal_alignment: map_horizontal_alignment(cell.h_align),
                vertical_alignment: map_vertical_alignment(cell.v_align),
                nested_asciidoc: matches!(cell.content, CellContent::AsciiDoc(_)),
            })
            .collect(),
    }
}

fn cell_visible_text(content: &CellContent<'_>) -> String {
    match content {
        CellContent::AsciiDoc(document) => document_visible_text(&document.content),
        CellContent::Literal(nodes) => inline_visible_text(nodes),
        CellContent::Default(paragraphs)
        | CellContent::Emphasis(paragraphs)
        | CellContent::Header(paragraphs)
        | CellContent::Monospace(paragraphs)
        | CellContent::Strong(paragraphs) => paragraphs
            .iter()
            .map(inline_visible_text)
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn document_visible_text(content: &DocContent<'_>) -> String {
    let mut text = Vec::new();
    match content {
        DocContent::Blocks(blocks) => collect_blocks_visible_text(blocks, &mut text),
        DocContent::Sectioned { preamble, sections } => {
            if let Some(preamble) = preamble {
                collect_blocks_visible_text(preamble, &mut text);
            }
            for section in sections {
                text.push(inline_visible_text(&section.heading));
                collect_blocks_visible_text(&section.blocks, &mut text);
            }
        }
    }
    text.join("\n")
}

fn collect_blocks_visible_text(blocks: &[AsciiDocBlock<'_>], text: &mut Vec<String>) {
    for block in blocks {
        match &block.content {
            BlockContent::Simple(nodes) => text.push(inline_visible_text(nodes)),
            BlockContent::QuotedParagraph { quote, .. } => {
                text.push(inline_visible_text(quote));
            }
            BlockContent::Compound(children) => collect_blocks_visible_text(children, text),
            BlockContent::Section(section) => {
                text.push(inline_visible_text(&section.heading));
                collect_blocks_visible_text(&section.blocks, text);
            }
            _ => {}
        }
    }
}

const fn map_cell_style(content: &CellContent<'_>) -> TableCellStyle {
    match content {
        CellContent::AsciiDoc(_) => TableCellStyle::AsciiDoc,
        CellContent::Default(_) => TableCellStyle::Default,
        CellContent::Emphasis(_) => TableCellStyle::Emphasis,
        CellContent::Header(_) => TableCellStyle::Header,
        CellContent::Literal(_) => TableCellStyle::Literal,
        CellContent::Monospace(_) => TableCellStyle::Monospace,
        CellContent::Strong(_) => TableCellStyle::Strong,
    }
}

const fn map_horizontal_alignment(alignment: HorizontalAlignment) -> HorizontalCellAlignment {
    match alignment {
        HorizontalAlignment::Left => HorizontalCellAlignment::Left,
        HorizontalAlignment::Center => HorizontalCellAlignment::Center,
        HorizontalAlignment::Right => HorizontalCellAlignment::Right,
    }
}

const fn map_vertical_alignment(alignment: VerticalAlignment) -> VerticalCellAlignment {
    match alignment {
        VerticalAlignment::Top => VerticalCellAlignment::Top,
        VerticalAlignment::Middle => VerticalCellAlignment::Middle,
        VerticalAlignment::Bottom => VerticalCellAlignment::Bottom,
    }
}

fn block_math_notation(block: &AsciiDocBlock<'_>, source: &str, offset: usize) -> MathNotation {
    if block.meta.attrs.has_str_positional("latexmath") {
        return MathNotation::LatexMath;
    }
    let start = offset + block.meta.start_loc.start as usize;
    if source
        .get(start..)
        .is_some_and(|tail| tail.starts_with("[latexmath]"))
    {
        MathNotation::LatexMath
    } else {
        document_stem_notation(source, offset)
    }
}

fn document_stem_notation(source: &str, body_offset: usize) -> MathNotation {
    let mut in_header = false;
    for line in lines(source).filter(|line| line.start >= body_offset) {
        if line.text.is_empty() {
            if in_header {
                break;
            }
            continue;
        }
        if line.text.starts_with("= ") || line.text.starts_with(':') {
            in_header = true;
            if let Some(value) = line
                .text
                .strip_prefix(":stem:")
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                return if value == "latexmath" {
                    MathNotation::LatexMath
                } else {
                    MathNotation::AsciiMath
                };
            }
            continue;
        }
        if in_header {
            break;
        }
        return MathNotation::AsciiMath;
    }
    MathNotation::AsciiMath
}

fn push_inline_block(
    blocks: &mut Vec<Block>,
    kind: BlockKind,
    location: SourceLocation,
    nodes: &InlineNodes<'_>,
    source: &str,
    offset: usize,
    heading_level: Option<u8>,
) {
    let text_range = offset_location(location, offset, source.len());
    let mut range = text_range.clone();
    range.start = line_start(source, range.start);
    range.end = line_end(source, range.end);
    blocks.push(Block {
        kind,
        range: as_u64_range(range),
        text_range: as_u64_range(text_range),
        text: inline_visible_text(nodes),
        heading_level,
        quote_depth: None,
        block_id: None,
        roles: Vec::new(),
        title: None,
        semantic: default_block_semantic(kind, heading_level, None),
    });
}

fn inline_visible_text(nodes: &InlineNodes<'_>) -> String {
    let mut text = String::new();
    append_inline_visible_text(nodes, &mut text);
    text
}

fn append_inline_visible_text(nodes: &InlineNodes<'_>, text: &mut String) {
    for node in nodes.iter() {
        match &node.content {
            Inline::Bold(children)
            | Inline::Highlight(children)
            | Inline::Italic(children)
            | Inline::Mono(children)
            | Inline::Quote(_, children)
            | Inline::Superscript(children)
            | Inline::Subscript(children)
            | Inline::TextSpan(_, children)
            | Inline::InlinePassthru(children) => append_inline_visible_text(children, text),
            Inline::Macro(asciidork_ast::MacroNode::Footnote { id, text: body }) => {
                if let Some(body) = body {
                    append_inline_visible_text(body, text);
                } else if let Some(id) = id {
                    text.push_str(&id.src);
                }
            }
            Inline::Macro(asciidork_ast::MacroNode::Image { target, attrs, .. }) => {
                text.push_str(attrs.str_positional_at(0).unwrap_or(&target.src));
            }
            Inline::Macro(asciidork_ast::MacroNode::Link { target, attrs, .. }) => {
                let display = attrs
                    .as_ref()
                    .and_then(|attrs| attrs.str_positional_at(0))
                    .unwrap_or(&target.src);
                text.push_str(display);
            }
            Inline::Macro(asciidork_ast::MacroNode::Xref {
                target, linktext, ..
            }) => {
                if let Some(linktext) = linktext {
                    append_inline_visible_text(linktext, text);
                } else {
                    text.push_str(&target.src);
                }
            }
            Inline::Macro(_)
            | Inline::Discarded
            | Inline::InlineAnchor(_)
            | Inline::BiblioAnchor(_) => {}
            Inline::CurlyQuote(asciidork_ast::CurlyKind::RightDouble) => text.push('”'),
            Inline::CurlyQuote(asciidork_ast::CurlyKind::LeftDouble) => text.push('“'),
            Inline::CurlyQuote(asciidork_ast::CurlyKind::LeftSingle) => text.push('‘'),
            Inline::CurlyQuote(asciidork_ast::CurlyKind::RightSingle) => text.push('’'),
            Inline::CurlyQuote(asciidork_ast::CurlyKind::LegacyImplicitApostrophe) => {
                text.push('\'');
            }
            Inline::Newline | Inline::MultiCharWhitespace(_) => text.push(' '),
            Inline::LineBreak => text.push('\n'),
            Inline::LitMono(value) => text.push_str(&value.src),
            Inline::Text(value) | Inline::CalloutTuck(value) | Inline::LineComment(value) => {
                text.push_str(value);
            }
            Inline::CalloutNum(callout) => {
                let _ = write!(text, "<{}>", callout.number);
            }
            Inline::SpecialChar(asciidork_ast::SpecialCharKind::Ampersand) => text.push('&'),
            Inline::SpecialChar(asciidork_ast::SpecialCharKind::LessThan) => text.push('<'),
            Inline::SpecialChar(asciidork_ast::SpecialCharKind::GreaterThan) => text.push('>'),
            Inline::Symbol(asciidork_ast::SymbolKind::Copyright) => text.push_str("(C)"),
            Inline::Symbol(asciidork_ast::SymbolKind::Registered) => text.push_str("(R)"),
            Inline::Symbol(asciidork_ast::SymbolKind::Trademark) => text.push_str("(TM)"),
            Inline::Symbol(asciidork_ast::SymbolKind::EmDash) => text.push('—'),
            Inline::Symbol(asciidork_ast::SymbolKind::SpacedEmDash(_)) => text.push_str(" — "),
            Inline::Symbol(asciidork_ast::SymbolKind::Ellipsis) => text.push_str("..."),
            Inline::Symbol(asciidork_ast::SymbolKind::SingleRightArrow) => text.push_str("->"),
            Inline::Symbol(asciidork_ast::SymbolKind::DoubleRightArrow) => text.push_str("=>"),
            Inline::Symbol(asciidork_ast::SymbolKind::SingleLeftArrow) => text.push_str("<-"),
            Inline::Symbol(asciidork_ast::SymbolKind::DoubleLeftArrow) => text.push_str("<="),
        }
    }
}

fn push_document_title(
    blocks: &mut Vec<Block>,
    location: SourceLocation,
    nodes: &InlineNodes<'_>,
    source: &str,
    offset: usize,
) {
    let text_range = offset_location(location, offset, source.len());
    let mut range = text_range.clone();
    range.start = line_start(source, range.start);
    range.end = line_end(source, range.end);
    let visible = nodes.plain_text().concat();
    let authored = source.get(text_range.clone()).unwrap_or_default();
    let visible_parts = visible.split_once(": ");
    let authored_separator = authored.find(": ");

    if let (Some((title, subtitle)), Some(separator)) = (visible_parts, authored_separator) {
        let title_range = text_range.start..text_range.start + separator;
        let subtitle_start = text_range.start + separator + 2;
        blocks.push(Block {
            kind: BlockKind::DocumentTitle,
            range: as_u64_range(range.clone()),
            text_range: as_u64_range(title_range),
            text: title.to_owned(),
            heading_level: Some(0),
            quote_depth: None,
            block_id: None,
            roles: Vec::new(),
            title: None,
            semantic: BlockSemantic::DocumentTitle,
        });
        blocks.push(Block {
            kind: BlockKind::DocumentSubtitle,
            range: as_u64_range(range),
            text_range: as_u64_range(subtitle_start..text_range.end),
            text: subtitle.to_owned(),
            heading_level: None,
            quote_depth: None,
            block_id: None,
            roles: Vec::new(),
            title: None,
            semantic: BlockSemantic::DocumentSubtitle,
        });
    } else {
        push_inline_block(
            blocks,
            BlockKind::DocumentTitle,
            location,
            nodes,
            source,
            offset,
            Some(0),
        );
    }
}

#[allow(clippy::too_many_lines)]
fn scan_profile_lines(
    source: &str,
    body_offset: usize,
    blocks: &mut Vec<Block>,
    protected: &mut Vec<Range<u64>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let all_lines: Vec<_> = lines(source)
        .filter(|line| line.start >= body_offset)
        .collect();
    let mut pending_roles = Vec::new();
    let mut index = 0;
    while index < all_lines.len() {
        let line = &all_lines[index];
        let protected_here = inside_ranges(line.start, protected);
        let protected_starts_here = protected
            .iter()
            .any(|range| range.start == line.start as u64);
        if protected_here && !protected_starts_here {
            index += 1;
            continue;
        }
        if line.text.starts_with("[.") && line.text.ends_with(']') {
            pending_roles = line.text[2..line.text.len() - 1]
                .split('.')
                .map(str::to_owned)
                .collect();
            index += 1;
            continue;
        }
        if let Some((level, text_start)) = profile_heading(line.text) {
            let text_range = (line.start + text_start)..line.end;
            push_source_block(
                blocks,
                BlockKind::Heading,
                line.start..line.full_end,
                text_range,
                source,
                Some(level),
                None,
                std::mem::take(&mut pending_roles),
            );
            index += 1;
            continue;
        }
        if let Some((depth, text_start)) = profile_quote(line.text) {
            push_source_block(
                blocks,
                BlockKind::Quote,
                line.start..line.full_end,
                (line.start + text_start)..line.end,
                source,
                None,
                Some(depth),
                Vec::new(),
            );
            if depth > 9 {
                diagnostics.push(Diagnostic {
                    code: DiagnosticCode::UnsupportedProfileSyntax,
                    range: as_u64_range(line.start..line.full_end),
                    message: "quotation depth above nine is retained exactly but has no full v1 presentation support"
                        .to_owned(),
                });
            }
            if line.text[text_start..].is_empty() || line.text.ends_with(" +") {
                diagnostics.push(Diagnostic {
                    code: DiagnosticCode::QuoteSyntaxUnresolved,
                    range: as_u64_range(line.start..line.full_end),
                    message: "quotation blank-line or continuation semantics are not frozen; source is preserved without grouping"
                        .to_owned(),
                });
            }
            if all_lines
                .get(index + 1)
                .is_some_and(|next| next.start == line.full_end && next.text.starts_with("-- "))
            {
                let next = &all_lines[index + 1];
                diagnostics.push(Diagnostic {
                    code: DiagnosticCode::QuoteSyntaxUnresolved,
                    range: as_u64_range(line.start..next.full_end),
                    message: "marker-quote attribution semantics are not frozen; attribution was not attached"
                        .to_owned(),
                });
            }
        }
        if line.text.starts_with('.') && !line.text.starts_with("..") && line.text.len() > 1 {
            push_source_block(
                blocks,
                BlockKind::BlockTitle,
                line.start..line.full_end,
                (line.start + 1)..line.end,
                source,
                None,
                None,
                Vec::new(),
            );
        }
        if matches!(line.text, "[stem]" | "[latexmath]") {
            if let Some(end_index) = delimited_block_end(&all_lines, index + 1) {
                let end = all_lines[end_index].full_end;
                push_source_block(
                    blocks,
                    BlockKind::Math,
                    line.start..end,
                    all_lines[index + 2].start..all_lines[end_index].start,
                    source,
                    None,
                    None,
                    Vec::new(),
                );
                protected.push(as_u64_range(line.start..end));
                index = end_index + 1;
                continue;
            }
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::UnsupportedProfileSyntax,
                range: as_u64_range(line.start..line.full_end),
                message: "malformed STEM block was preserved without derived rendering".to_owned(),
            });
        }
        if line.text == "[mermaid]" {
            if let Some(end_index) = delimited_block_end(&all_lines, index + 1) {
                let end = all_lines[end_index].full_end;
                push_source_block(
                    blocks,
                    BlockKind::Mermaid,
                    line.start..end,
                    all_lines[index + 2].start..all_lines[end_index].start,
                    source,
                    None,
                    None,
                    Vec::new(),
                );
                protected.push(as_u64_range(line.start..end));
                index = end_index + 1;
                continue;
            }
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::UnsupportedProfileSyntax,
                range: as_u64_range(line.start..line.full_end),
                message: "malformed Mermaid block was preserved without derived rendering"
                    .to_owned(),
            });
        }
        index += 1;
    }
}

fn delimited_block_end(lines: &[Line<'_>], delimiter_index: usize) -> Option<usize> {
    let delimiter = lines.get(delimiter_index)?.text;
    if !matches!(delimiter, "----" | "...." | "++++") {
        return None;
    }
    lines
        .iter()
        .enumerate()
        .skip(delimiter_index + 1)
        .find_map(|(index, line)| (line.text == delimiter).then_some(index))
}

fn profile_heading(line: &str) -> Option<(u8, usize)> {
    let equals = line.bytes().take_while(|byte| *byte == b'=').count();
    if !(2..=10).contains(&equals) || line.as_bytes().get(equals) != Some(&b' ') {
        return None;
    }
    Some((u8::try_from(equals - 1).ok()?, equals + 1))
}

fn profile_quote(line: &str) -> Option<(u64, usize)> {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut depth = 0_u64;
    while bytes.get(index) == Some(&b'>') {
        depth += 1;
        index += 1;
        if bytes.get(index) == Some(&b' ') {
            index += 1;
        }
    }
    (depth > 0).then_some((depth, index))
}

#[allow(clippy::too_many_lines)]
fn scan_inline_semantics(
    source: &str,
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<LinkOccurrence>, Vec<InlineSemantic>) {
    let (mut links, node_macro_ranges) = scan_node_link_occurrences(source, protected, diagnostics);
    let mut scanner_protected = protected.to_vec();
    scanner_protected.extend(node_macro_ranges);
    normalize_ranges(&mut scanner_protected);
    for (needle, kind) in [
        ("xref:", LinkKind::Xref),
        ("footnote:", LinkKind::Footnote),
        ("endnote:", LinkKind::Endnote),
    ] {
        let mut cursor = 0;
        while let Some(found) = source[cursor..].find(needle) {
            let start = cursor + found;
            if inside_ranges(start, &scanner_protected) {
                cursor = start + needle.len();
                continue;
            }
            let value_start = start + needle.len();
            let Some(open_rel) = source[value_start..].find('[') else {
                diagnostics.push(malformed_inline_diagnostic(
                    start,
                    (start + needle.len()).min(source.len()),
                    needle,
                ));
                cursor = start + needle.len();
                continue;
            };
            let open = value_start + open_rel;
            if source[value_start..open].contains(['\n', '\r']) {
                diagnostics.push(malformed_inline_diagnostic(start, open, needle));
                cursor = open + 1;
                continue;
            }
            let Some(close) = closing_bracket(source, open) else {
                diagnostics.push(malformed_inline_diagnostic(start, source.len(), needle));
                cursor = open + 1;
                continue;
            };
            let raw_target = &source[value_start..open];
            let (target, fragment) = raw_target
                .split_once('#')
                .map_or((raw_target, None), |(target, fragment)| {
                    (target, Some(fragment.to_owned()))
                });
            let display =
                (!source[open + 1..close].is_empty()).then(|| source[open + 1..close].to_owned());
            links.push(LinkOccurrence {
                kind,
                range: as_u64_range(start..close + 1),
                target_range: as_u64_range(value_start..value_start + target.len()),
                label_range: as_u64_range(open + 1..close),
                target: target.to_owned(),
                fragment,
                display,
            });
            cursor = close + 1;
        }
    }
    links.sort_by_key(|link| (link.range.start, link.range.end));
    let mut inlines = links.iter().map(link_inline_semantic).collect::<Vec<_>>();
    scan_anchor_semantics(source, &scanner_protected, diagnostics, &mut inlines);
    scan_macro_semantics(
        source,
        &scanner_protected,
        diagnostics,
        "link:",
        InlineKind::NativeLink,
        None,
        &mut inlines,
    );
    scan_macro_semantics(
        source,
        &scanner_protected,
        diagnostics,
        "image::",
        InlineKind::Image,
        None,
        &mut inlines,
    );
    scan_macro_semantics(
        source,
        &scanner_protected,
        diagnostics,
        "image:",
        InlineKind::Image,
        None,
        &mut inlines,
    );
    scan_macro_semantics(
        source,
        &scanner_protected,
        diagnostics,
        "latexmath:",
        InlineKind::LatexMath,
        Some(MathNotation::LatexMath),
        &mut inlines,
    );
    scan_macro_semantics(
        source,
        &scanner_protected,
        diagnostics,
        "stem:",
        InlineKind::Stem,
        Some(document_stem_notation(source, 0)),
        &mut inlines,
    );
    scan_shorthand_xrefs(source, &scanner_protected, diagnostics, &mut inlines);
    scan_bare_uri_links(source, &scanner_protected, &mut inlines);
    inlines.sort_by_key(|inline| (inline.range.start, inline.range.end));
    inlines.dedup_by(|left, right| {
        left.kind == right.kind && left.range == right.range && left.target == right.target
    });
    (links, inlines)
}

fn scan_node_link_occurrences(
    source: &str,
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<LinkOccurrence>, Vec<Range<u64>>) {
    let mut links = Vec::new();
    let mut opaque_ranges = Vec::new();
    let mut cursor = 0;
    while let Some(found) = source[cursor..].find("node:") {
        let start = cursor + found;
        let (needle, kind) = if source[start..].starts_with("node::") {
            ("node::", LinkKind::NodeEmbed)
        } else {
            ("node:", LinkKind::Node)
        };
        if inside_ranges(start, protected) {
            cursor = start + needle.len();
            continue;
        }

        let value_start = start + needle.len();
        let failure_end = line_end(source, start);
        let Some(open_rel) = source[value_start..].find('[') else {
            diagnostics.push(malformed_inline_diagnostic(start, failure_end, needle));
            opaque_ranges.push(as_u64_range(start..failure_end));
            cursor = failure_end.max(value_start);
            continue;
        };
        let open = value_start + open_rel;
        if source[value_start..open].contains(['\n', '\r']) {
            diagnostics.push(malformed_inline_diagnostic(start, failure_end, needle));
            opaque_ranges.push(as_u64_range(start..failure_end));
            cursor = failure_end.max(value_start);
            continue;
        }
        let Some(close) = closing_bracket(source, open) else {
            diagnostics.push(malformed_inline_diagnostic(start, failure_end, needle));
            opaque_ranges.push(as_u64_range(start..failure_end));
            cursor = failure_end.max(open + 1);
            continue;
        };

        let macro_range = start..close + 1;
        opaque_ranges.push(as_u64_range(macro_range.clone()));
        let label_range = open + 1..close;
        let Ok(decoded_label) = decode_node_link_label(&source[label_range.clone()]) else {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::InvalidNodeLink,
                range: as_u64_range(macro_range),
                message: "node macro display label is not canonically escaped".to_owned(),
            });
            cursor = close + 1;
            continue;
        };

        let raw_target = &source[value_start..open];
        let (target, fragment) = raw_target
            .split_once('#')
            .map_or((raw_target, None), |(target, fragment)| {
                (target, Some(fragment.to_owned()))
            });
        if !is_canonical_node_id(target) {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::InvalidNodeLink,
                range: as_u64_range(macro_range.clone()),
                message: "node macro target must be a UUID".to_owned(),
            });
        }
        links.push(LinkOccurrence {
            kind,
            range: as_u64_range(macro_range),
            target_range: as_u64_range(value_start..value_start + target.len()),
            label_range: as_u64_range(label_range),
            target: target.to_owned(),
            fragment,
            display: (!decoded_label.is_empty()).then_some(decoded_label),
        });
        cursor = close + 1;
    }
    (links, opaque_ranges)
}

fn malformed_inline_diagnostic(start: usize, end: usize, construct: &str) -> Diagnostic {
    Diagnostic {
        code: DiagnosticCode::UnsupportedProfileSyntax,
        range: as_u64_range(start..end),
        message: format!(
            "malformed {construct} inline construct was preserved without interpretation"
        ),
    }
}

fn closing_bracket(source: &str, open: usize) -> Option<usize> {
    let mut escaped = false;
    for (relative, character) in source.get(open + 1..)?.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            ']' => return Some(open + 1 + relative),
            '\n' | '\r' => return None,
            _ => {}
        }
    }
    None
}

fn is_canonical_node_id(target: &str) -> bool {
    Uuid::parse_str(target)
        .is_ok_and(|uuid| uuid.get_version_num() == 4 && uuid.hyphenated().to_string() == target)
}

fn link_inline_semantic(link: &LinkOccurrence) -> InlineSemantic {
    let kind = match link.kind {
        LinkKind::Node => InlineKind::Node,
        LinkKind::NodeEmbed => InlineKind::NodeEmbed,
        LinkKind::Xref => InlineKind::Xref,
        LinkKind::Footnote => InlineKind::Footnote,
        LinkKind::Endnote => InlineKind::Endnote,
    };
    InlineSemantic {
        kind,
        range: link.range.clone(),
        target_range: Some(link.target_range.clone()),
        label_range: Some(link.label_range.clone()),
        target: (!link.target.is_empty()).then(|| link.target.clone()),
        fragment: link.fragment.clone(),
        text: link.display.clone(),
        notation: None,
        roles: Vec::new(),
    }
}

fn scan_anchor_semantics(
    source: &str,
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
    inlines: &mut Vec<InlineSemantic>,
) {
    for (opening, closing, prefix) in [("[[", "]]", 2_usize), ("[#", "]", 2_usize)] {
        let mut cursor = 0;
        while let Some(found) = source[cursor..].find(opening) {
            let start = cursor + found;
            if inside_ranges(start, protected) {
                cursor = start + opening.len();
                continue;
            }
            let value_start = start + prefix;
            let Some(close_rel) = source[value_start..].find(closing) else {
                diagnostics.push(malformed_inline_diagnostic(
                    start,
                    (start + opening.len()).min(source.len()),
                    "anchor",
                ));
                cursor = value_start;
                continue;
            };
            let close = value_start + close_rel;
            let raw = &source[value_start..close];
            if raw.contains(['\n', '\r']) {
                diagnostics.push(malformed_inline_diagnostic(
                    start,
                    close + closing.len(),
                    "anchor",
                ));
                cursor = close + closing.len();
                continue;
            }
            let (target, reftext) = raw
                .split_once(',')
                .map_or((raw, None), |(target, reftext)| (target, Some(reftext)));
            if target.is_empty() {
                diagnostics.push(malformed_inline_diagnostic(
                    start,
                    close + closing.len(),
                    "anchor",
                ));
            } else {
                inlines.push(InlineSemantic {
                    kind: InlineKind::Anchor,
                    range: as_u64_range(start..close + closing.len()),
                    target_range: Some(as_u64_range(value_start..value_start + target.len())),
                    label_range: None,
                    target: Some(target.to_owned()),
                    fragment: None,
                    text: reftext.map(str::to_owned),
                    notation: None,
                    roles: Vec::new(),
                });
            }
            cursor = close + closing.len();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn scan_macro_semantics(
    source: &str,
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
    needle: &str,
    kind: InlineKind,
    notation: Option<MathNotation>,
    inlines: &mut Vec<InlineSemantic>,
) {
    let mut cursor = 0;
    while let Some(found) = source[cursor..].find(needle) {
        let start = cursor + found;
        if needle == "image:" && source[start..].starts_with("image::") {
            cursor = start + "image::".len();
            continue;
        }
        if inside_ranges(start, protected) {
            cursor = start + needle.len();
            continue;
        }
        let value_start = start + needle.len();
        let Some(open_rel) = source[value_start..].find('[') else {
            diagnostics.push(malformed_inline_diagnostic(start, value_start, needle));
            cursor = value_start;
            continue;
        };
        let open = value_start + open_rel;
        if source[value_start..open].contains(['\n', '\r']) {
            diagnostics.push(malformed_inline_diagnostic(start, open, needle));
            cursor = open + 1;
            continue;
        }
        let Some(close) = closing_bracket(source, open) else {
            diagnostics.push(malformed_inline_diagnostic(start, source.len(), needle));
            cursor = open + 1;
            continue;
        };
        let target = &source[value_start..open];
        let text = &source[open + 1..close];
        inlines.push(InlineSemantic {
            kind,
            range: as_u64_range(start..close + 1),
            target_range: (!target.is_empty()).then(|| as_u64_range(value_start..open)),
            label_range: None,
            target: (!target.is_empty()).then(|| target.to_owned()),
            fragment: None,
            text: (!text.is_empty()).then(|| text.to_owned()),
            notation,
            roles: Vec::new(),
        });
        cursor = close + 1;
    }
}

fn scan_shorthand_xrefs(
    source: &str,
    protected: &[Range<u64>],
    diagnostics: &mut Vec<Diagnostic>,
    inlines: &mut Vec<InlineSemantic>,
) {
    let mut cursor = 0;
    while let Some(found) = source[cursor..].find("<<") {
        let start = cursor + found;
        if inside_ranges(start, protected) {
            cursor = start + 2;
            continue;
        }
        let value_start = start + 2;
        let Some(close_rel) = source[value_start..].find(">>") else {
            diagnostics.push(malformed_inline_diagnostic(start, value_start, "xref"));
            cursor = value_start;
            continue;
        };
        let close = value_start + close_rel;
        let raw = &source[value_start..close];
        if raw.contains(['\n', '\r']) {
            diagnostics.push(malformed_inline_diagnostic(start, close + 2, "xref"));
            cursor = close + 2;
            continue;
        }
        let (target, text) = raw
            .split_once(',')
            .map_or((raw, None), |(target, text)| (target, Some(text)));
        if target.is_empty() {
            diagnostics.push(malformed_inline_diagnostic(start, close + 2, "xref"));
        } else {
            inlines.push(InlineSemantic {
                kind: InlineKind::Xref,
                range: as_u64_range(start..close + 2),
                target_range: Some(as_u64_range(value_start..value_start + target.len())),
                label_range: None,
                target: Some(target.to_owned()),
                fragment: None,
                text: text.map(str::to_owned),
                notation: None,
                roles: Vec::new(),
            });
        }
        cursor = close + 2;
    }
}

fn scan_bare_uri_links(source: &str, protected: &[Range<u64>], inlines: &mut Vec<InlineSemantic>) {
    for scheme in ["https://", "http://", "ftp://", "mailto:"] {
        let mut cursor = 0;
        while let Some(found) = source[cursor..].find(scheme) {
            let start = cursor + found;
            if inside_ranges(start, protected)
                || source[..start].ends_with("link:")
                || source[..start].ends_with("include::")
            {
                cursor = start + scheme.len();
                continue;
            }
            let target_end = source[start..]
                .find(|character: char| character.is_whitespace() || matches!(character, '[' | ']'))
                .map_or(source.len(), |relative| start + relative);
            if target_end == start {
                cursor = start + scheme.len();
                continue;
            }
            let mut end = target_end;
            let mut text = None;
            if source.as_bytes().get(target_end) == Some(&b'[')
                && let Some(close) = closing_bracket(source, target_end)
            {
                text = (!source[target_end + 1..close].is_empty())
                    .then(|| source[target_end + 1..close].to_owned());
                end = close + 1;
            }
            inlines.push(InlineSemantic {
                kind: InlineKind::NativeLink,
                range: as_u64_range(start..end),
                target_range: Some(as_u64_range(start..target_end)),
                label_range: None,
                target: Some(source[start..target_end].to_owned()),
                fragment: None,
                text,
                notation: None,
                roles: Vec::new(),
            });
            cursor = end.max(start + scheme.len());
        }
    }
}

fn build_searchable_text(
    source: &str,
    protected: &[Range<u64>],
    blocks: &[Block],
    document_header: &DocumentHeaderSemantic,
) -> String {
    let mut searchable = String::new();
    for block in blocks.iter().filter(|block| {
        matches!(
            block.kind,
            BlockKind::DocumentTitle | BlockKind::DocumentSubtitle
        )
    }) {
        push_searchable_line(&mut searchable, &block.text);
    }
    for attribute in document_header
        .attributes
        .iter()
        .filter(|attribute| attribute.projected)
    {
        let mut property = attribute.name.clone();
        if let Some(value) = &attribute.literal_value {
            property.push_str(": ");
            property.push_str(value);
        }
        push_searchable_line(&mut searchable, &property);
    }
    for line in lines(source) {
        if !inside_ranges(line.start, protected) {
            push_searchable_line(&mut searchable, line.text);
        }
    }
    searchable
}

fn push_searchable_line(searchable: &mut String, line: &str) {
    if !searchable.is_empty() {
        searchable.push('\n');
    }
    searchable.push_str(line);
}

#[allow(clippy::too_many_lines)]
fn render_safe_html(
    source: &str,
    blocks: &[Block],
    inlines: &[InlineSemantic],
    adjacent_heading_bodies: &[AdjacentHeadingBodyResolution],
    status: AnalysisStatus,
) -> String {
    if status == AnalysisStatus::Failed {
        return escaped_source_fallback(source, "failed");
    }
    let mut html = format!(
        "<article data-weftext-profile=\"weftext-asciidoc-v1\" data-semantic-model-version=\"{SEMANTIC_MODEL_VERSION}\" data-analysis-status=\"{}\" data-inline-count=\"{}\">",
        match status {
            AnalysisStatus::Complete => "complete",
            AnalysisStatus::Degraded => "degraded",
            AnalysisStatus::Failed => "failed",
        },
        inlines.len()
    );
    for (block_index, block) in blocks.iter().enumerate() {
        let escaped = escape_html(&block.text);
        let attributes = safe_block_attributes(block, block_index, adjacent_heading_bodies);
        match &block.semantic {
            BlockSemantic::Frontmatter | BlockSemantic::Comment => {}
            BlockSemantic::DocumentTitle => {
                let _ = write!(
                    html,
                    "<h1 data-document-title=\"true\"{attributes}>{escaped}</h1>"
                );
            }
            BlockSemantic::DocumentSubtitle => {
                let _ = write!(
                    html,
                    "<p data-document-subtitle=\"true\"{attributes}>{escaped}</p>"
                );
            }
            BlockSemantic::Heading { level } => {
                let level = *level;
                let tag = level.clamp(1, 6);
                let _ = write!(
                    html,
                    "<h{tag} role=\"heading\" aria-level=\"{level}\" data-level=\"{level}\"{attributes}>{escaped}</h{tag}>"
                );
            }
            BlockSemantic::Paragraph => {
                let _ = write!(html, "<p{attributes}>{escaped}</p>");
            }
            BlockSemantic::Quote {
                depth,
                attribution,
                citation,
            } => {
                let depth = depth.or(block.quote_depth).unwrap_or(1);
                let _ = write!(
                    html,
                    "<blockquote data-depth=\"{depth}\"{attributes}><p>{escaped}</p>"
                );
                if attribution.is_some() || citation.is_some() {
                    html.push_str("<footer>");
                    if let Some(attribution) = attribution {
                        html.push_str(&escape_html(attribution));
                    }
                    if let Some(citation) = citation {
                        let _ = write!(html, "<cite>{}</cite>", escape_html(citation));
                    }
                    html.push_str("</footer>");
                }
                html.push_str("</blockquote>");
            }
            BlockSemantic::Listing { language } => {
                let language = language.as_deref().unwrap_or("");
                let _ = write!(
                    html,
                    "<pre data-weftext-listing=\"true\"{attributes}><code data-language=\"{}\">{escaped}</code></pre>",
                    escape_html(language)
                );
            }
            BlockSemantic::Literal => {
                let _ = write!(
                    html,
                    "<pre data-weftext-literal=\"true\"{attributes}>{escaped}</pre>"
                );
            }
            BlockSemantic::List { model } => {
                if list_has_unmodeled_continuations(model) {
                    let source = escape_html(block_source(source, block));
                    let _ = write!(
                        html,
                        "<pre data-weftext-fallback=\"escaped-source\" data-unsupported-context=\"list-continuation\"{attributes}>{source}</pre>"
                    );
                } else {
                    render_list_model(&mut html, model, &attributes);
                }
            }
            BlockSemantic::Table { model } => {
                if table_has_nested_asciidoc(model) {
                    let source = escape_html(block_source(source, block));
                    let _ = write!(
                        html,
                        "<pre data-weftext-fallback=\"escaped-source\" data-unsupported-context=\"nested-asciidoc-table-cell\"{attributes}>{source}</pre>"
                    );
                } else {
                    render_table_model(&mut html, model, block.title.as_deref(), &attributes);
                }
            }
            BlockSemantic::Image { target, alt } => {
                let accessible = alt
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .unwrap_or(target);
                let _ = write!(
                    html,
                    "<figure data-weftext-image-target=\"{}\"{attributes}><span role=\"img\" aria-label=\"{}\">{}</span>",
                    escape_html(target),
                    escape_html(accessible),
                    escape_html(accessible)
                );
                if let Some(title) = &block.title {
                    let _ = write!(html, "<figcaption>{}</figcaption>", escape_html(title));
                }
                html.push_str("</figure>");
            }
            BlockSemantic::BlockTitle => {
                let _ = write!(
                    html,
                    "<div data-weftext-block-title=\"true\"{attributes}>{escaped}</div>"
                );
            }
            BlockSemantic::Math { notation } => {
                let notation = match notation {
                    MathNotation::AsciiMath => "asciimath",
                    MathNotation::LatexMath => "latexmath",
                };
                let source = escape_html(block_source(source, block));
                let _ = write!(
                    html,
                    "<pre data-weftext-math=\"{notation}\" data-weftext-render=\"constrained-source-fallback\"{attributes}>{source}</pre>"
                );
            }
            BlockSemantic::Mermaid => {
                let source = escape_html(block_source(source, block));
                let _ = write!(
                    html,
                    "<pre data-weftext-diagram=\"mermaid\" data-weftext-render=\"constrained-source-fallback\"{attributes}>{source}</pre>"
                );
            }
            BlockSemantic::Passthrough => {
                let source = escape_html(block_source(source, block));
                let _ = write!(
                    html,
                    "<pre data-weftext-effect=\"disabled-passthrough\" data-weftext-fallback=\"escaped-source\"{attributes}>{source}</pre>"
                );
            }
            BlockSemantic::ThematicBreak => html.push_str("<hr>"),
            BlockSemantic::Unsupported { context } => {
                let source = escape_html(block_source(source, block));
                let _ = write!(
                    html,
                    "<pre data-weftext-fallback=\"escaped-source\" data-unsupported-context=\"{}\"{attributes}>{source}</pre>",
                    escape_html(context)
                );
            }
        }
    }
    html.push_str("</article>");
    html
}

fn safe_block_attributes(
    block: &Block,
    block_index: usize,
    adjacent_heading_bodies: &[AdjacentHeadingBodyResolution],
) -> String {
    let mut attributes = String::new();
    if let Some(block_id) = &block.block_id {
        let _ = write!(attributes, " data-block-id=\"{}\"", escape_html(block_id));
    }
    if !block.roles.is_empty() {
        let _ = write!(
            attributes,
            " data-roles=\"{}\"",
            escape_html(&block.roles.join(" "))
        );
    }
    if let Some(resolution) = adjacent_heading_bodies
        .iter()
        .find(|resolution| resolution.heading_block == block_index as u64)
    {
        let presentation = match resolution.presentation {
            AdjacentHeadingBodyPresentation::RunIn => "run_in",
            AdjacentHeadingBodyPresentation::Separate => "separate",
        };
        let _ = write!(
            attributes,
            " data-adjacent-heading-body=\"{presentation}\" data-adjacent-heading-body-rule=\"{}\" data-adjacent-heading-body-eligibility=\"{}\"",
            adjacent_heading_body_rule_name(resolution.rule),
            adjacent_heading_body_eligibility_name(resolution.eligibility),
        );
    }
    if let Some(resolution) = adjacent_heading_bodies.iter().find(|resolution| {
        resolution.presentation == AdjacentHeadingBodyPresentation::RunIn
            && resolution.body_block == Some(block_index as u64)
    }) {
        let _ = write!(
            attributes,
            " data-run-in-heading-block=\"{}\"",
            resolution.heading_block
        );
    }
    attributes
}

const fn adjacent_heading_body_rule_name(rule: AdjacentHeadingBodyRule) -> &'static str {
    match rule {
        AdjacentHeadingBodyRule::ExplicitRunInRole => "explicit_run_in_role",
        AdjacentHeadingBodyRule::ExplicitSeparateRole => "explicit_separate_role",
        AdjacentHeadingBodyRule::WorkspaceRunInDefault => "workspace_run_in_default",
        AdjacentHeadingBodyRule::WorkspaceSeparateDefault => "workspace_separate_default",
    }
}

const fn adjacent_heading_body_eligibility_name(
    eligibility: AdjacentHeadingBodyEligibility,
) -> &'static str {
    match eligibility {
        AdjacentHeadingBodyEligibility::Eligible => "eligible",
        AdjacentHeadingBodyEligibility::NoFollowingBlock => "no_following_block",
        AdjacentHeadingBodyEligibility::FollowingBlockIsNotParagraph => {
            "following_block_is_not_paragraph"
        }
        AdjacentHeadingBodyEligibility::NonWhitespaceSourceGap => "non_whitespace_source_gap",
        AdjacentHeadingBodyEligibility::NotOnImmediatelyFollowingPhysicalLine => {
            "not_on_immediately_following_physical_line"
        }
    }
}

fn block_source<'a>(source: &'a str, block: &Block) -> &'a str {
    let Ok(start) = usize::try_from(block.range.start) else {
        return "";
    };
    let Ok(end) = usize::try_from(block.range.end) else {
        return "";
    };
    source.get(start..end).unwrap_or("")
}

fn render_list_model(html: &mut String, model: &ListModel, attributes: &str) {
    let tag = match model.kind {
        ListKind::Ordered => "ol",
        ListKind::Unordered | ListKind::Callout => "ul",
        ListKind::Description => "dl",
    };
    let _ = write!(html, "<{tag} data-depth=\"{}\"{attributes}>", model.depth);
    for item in &model.items {
        if model.kind == ListKind::Description {
            let _ = write!(html, "<dt>{}</dt><dd>", escape_html(&item.text));
        } else {
            html.push_str("<li>");
            if let Some(checked) = item.checked {
                let _ = write!(
                    html,
                    "<input type=\"checkbox\" disabled{} aria-label=\"checklist state\">",
                    if checked { " checked" } else { "" }
                );
            }
            html.push_str(&escape_html(&item.text));
        }
        if !item.children.is_empty() {
            let child_model = ListModel {
                kind: model.kind,
                depth: item.depth.saturating_add(1),
                items: item.children.clone(),
            };
            render_list_model(html, &child_model, "");
        }
        html.push_str(if model.kind == ListKind::Description {
            "</dd>"
        } else {
            "</li>"
        });
    }
    let _ = write!(html, "</{tag}>");
}

fn render_table_model(
    html: &mut String,
    model: &TableModel,
    title: Option<&str>,
    attributes: &str,
) {
    let _ = write!(
        html,
        "<table data-column-count=\"{}\"{attributes}>",
        model.column_count
    );
    if let Some(title) = title {
        let _ = write!(html, "<caption>{}</caption>", escape_html(title));
    }
    if let Some(header) = &model.header {
        html.push_str("<thead>");
        render_table_row(html, header, true);
        html.push_str("</thead>");
    }
    html.push_str("<tbody>");
    for row in &model.body {
        render_table_row(html, row, false);
    }
    html.push_str("</tbody>");
    if let Some(footer) = &model.footer {
        html.push_str("<tfoot>");
        render_table_row(html, footer, false);
        html.push_str("</tfoot>");
    }
    html.push_str("</table>");
}

fn render_table_row(html: &mut String, row: &TableRow, header: bool) {
    html.push_str("<tr>");
    for cell in &row.cells {
        let tag = if header || cell.style == TableCellStyle::Header {
            "th"
        } else {
            "td"
        };
        let _ = write!(
            html,
            "<{tag} colspan=\"{}\" rowspan=\"{}\" data-horizontal-align=\"{:?}\" data-vertical-align=\"{:?}\">{}</{tag}>",
            cell.column_span,
            cell.row_span,
            cell.horizontal_alignment,
            cell.vertical_alignment,
            escape_html(&cell.text)
        );
    }
    html.push_str("</tr>");
}

fn escaped_source_fallback(source: &str, status: &str) -> String {
    format!(
        "<article data-weftext-profile=\"weftext-asciidoc-v1\" data-analysis-status=\"{status}\"><pre data-weftext-fallback=\"escaped-source\">{}</pre></article>",
        escape_html(source)
    )
}

fn escape_html(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn map_parser_diagnostic(
    source: &str,
    offset: usize,
    warning: asciidork_parser::Diagnostic,
) -> Diagnostic {
    map_diagnostic(source, offset, warning, DiagnosticCode::ParserWarning)
}

fn map_parser_error(
    source: &str,
    offset: usize,
    error: asciidork_parser::Diagnostic,
) -> Diagnostic {
    map_diagnostic(source, offset, error, DiagnosticCode::ParserError)
}

fn map_diagnostic(
    source: &str,
    offset: usize,
    diagnostic: asciidork_parser::Diagnostic,
    code: DiagnosticCode,
) -> Diagnostic {
    let line = lines(&source[offset..]).nth(diagnostic.line_num.saturating_sub(1) as usize);
    let range = line.map_or(offset..offset, |line| {
        let start = offset + line.start + diagnostic.underline_start as usize;
        let end = (start + diagnostic.underline_width as usize).min(offset + line.end);
        start..end
    });
    Diagnostic {
        code,
        range: as_u64_range(range),
        message: diagnostic.message,
    }
}

#[allow(clippy::too_many_arguments)]
fn push_source_block(
    blocks: &mut Vec<Block>,
    kind: BlockKind,
    range: Range<usize>,
    text_range: Range<usize>,
    source: &str,
    heading_level: Option<u8>,
    quote_depth: Option<u64>,
    roles: Vec<String>,
) {
    let safe_range = clamp_range(range, source.len());
    let safe_text = clamp_range(text_range, source.len());
    let text = source
        .get(safe_text.clone())
        .unwrap_or_default()
        .trim_end()
        .to_owned();
    blocks.push(Block {
        kind,
        range: as_u64_range(safe_range),
        text_range: as_u64_range(safe_text),
        text,
        heading_level,
        quote_depth,
        block_id: None,
        roles,
        title: None,
        semantic: default_block_semantic(kind, heading_level, quote_depth),
    });
}

fn default_block_semantic(
    kind: BlockKind,
    heading_level: Option<u8>,
    quote_depth: Option<u64>,
) -> BlockSemantic {
    match kind {
        BlockKind::Frontmatter => BlockSemantic::Frontmatter,
        BlockKind::DocumentTitle => BlockSemantic::DocumentTitle,
        BlockKind::DocumentSubtitle => BlockSemantic::DocumentSubtitle,
        BlockKind::Heading => BlockSemantic::Heading {
            level: heading_level.unwrap_or(1),
        },
        BlockKind::Paragraph => BlockSemantic::Paragraph,
        BlockKind::Listing => BlockSemantic::Listing { language: None },
        BlockKind::Literal => BlockSemantic::Literal,
        BlockKind::Quote => BlockSemantic::Quote {
            depth: quote_depth,
            attribution: None,
            citation: None,
        },
        BlockKind::List => BlockSemantic::List {
            model: ListModel {
                kind: ListKind::Unordered,
                depth: 1,
                items: Vec::new(),
            },
        },
        BlockKind::Table => BlockSemantic::Table {
            model: TableModel {
                header: None,
                body: Vec::new(),
                footer: None,
                column_count: 0,
            },
        },
        BlockKind::Image => BlockSemantic::Image {
            target: String::new(),
            alt: None,
        },
        BlockKind::BlockTitle => BlockSemantic::BlockTitle,
        BlockKind::Math => BlockSemantic::Math {
            notation: MathNotation::AsciiMath,
        },
        BlockKind::Mermaid => BlockSemantic::Mermaid,
        BlockKind::Passthrough => BlockSemantic::Passthrough,
        BlockKind::Comment => BlockSemantic::Comment,
        BlockKind::ThematicBreak => BlockSemantic::ThematicBreak,
        BlockKind::Other => BlockSemantic::Unsupported {
            context: "unknown".to_owned(),
        },
    }
}

fn deduplicate_blocks(blocks: &mut Vec<Block>) {
    blocks.sort_by_key(|block| {
        (
            block.range.start,
            block.range.end,
            block_kind_order(block.kind),
        )
    });
    let mut merged: Vec<Block> = Vec::with_capacity(blocks.len());
    for block in blocks.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.kind == block.kind
            && previous.range == block.range
            && previous.heading_level == block.heading_level
        {
            for role in block.roles {
                if !previous.roles.contains(&role) {
                    previous.roles.push(role);
                }
            }
            previous.quote_depth = previous.quote_depth.or(block.quote_depth);
            previous.block_id = previous.block_id.take().or(block.block_id);
            continue;
        }
        merged.push(block);
    }
    *blocks = merged;
}

const fn block_kind_order(kind: BlockKind) -> u8 {
    kind as u8
}

fn normalize_ranges(ranges: &mut Vec<Range<u64>>) {
    ranges.sort_by_key(|range| (range.start, range.end));
    let mut merged: Vec<Range<u64>> = Vec::new();
    for range in ranges.drain(..) {
        if let Some(last) = merged.last_mut()
            && range.start < last.end
        {
            last.end = last.end.max(range.end);
            continue;
        }
        merged.push(range);
    }
    *ranges = merged;
}

fn normalize_inline_semantics(
    source: &str,
    inlines: &mut Vec<InlineSemantic>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let observed = std::mem::take(inlines);
    let valid = observed
        .iter()
        .filter(|inline| valid_inline_ranges(source, inline))
        .cloned()
        .collect::<Vec<_>>();
    for discarded in observed
        .iter()
        .filter(|inline| !valid_inline_ranges(source, inline))
    {
        let equivalent = valid.iter().any(|candidate| {
            candidate.kind == discarded.kind
                && candidate.target == discarded.target
                && candidate.fragment == discarded.fragment
                && candidate.text == discarded.text
                && candidate.notation == discarded.notation
                && candidate.roles == discarded.roles
        });
        if !equivalent {
            diagnostics.push(Diagnostic {
                code: DiagnosticCode::ParserWarning,
                range: 0..0,
                message: format!(
                    "parser supplied a non-UTF-8 {:?} inline range; typed interpretation was discarded",
                    discarded.kind
                ),
            });
        }
    }
    *inlines = valid;
    inlines.sort_by_key(|inline| (inline.range.start, inline.range.end, inline.kind as u8));
    inlines.dedup_by(|left, right| {
        left.kind == right.kind && left.range == right.range && left.target == right.target
    });
}

fn valid_inline_ranges(source: &str, inline: &InlineSemantic) -> bool {
    valid_source_range(source, &inline.range)
        && inline
            .target_range
            .as_ref()
            .is_none_or(|range| valid_source_range(source, range))
        && inline
            .label_range
            .as_ref()
            .is_none_or(|range| valid_source_range(source, range))
}

fn valid_source_range(source: &str, range: &Range<u64>) -> bool {
    let Ok(start) = usize::try_from(range.start) else {
        return false;
    };
    let Ok(end) = usize::try_from(range.end) else {
        return false;
    };
    start <= end
        && end <= source.len()
        && source.is_char_boundary(start)
        && source.is_char_boundary(end)
}

fn diagnose_inline_semantics(inlines: &[InlineSemantic], diagnostics: &mut Vec<Diagnostic>) {
    for inline in inlines {
        let (code, message) = match inline.kind {
            InlineKind::Passthrough => (
                DiagnosticCode::PassthroughDisabled,
                "inline passthrough is preserved exactly and its effect is disabled".to_owned(),
            ),
            InlineKind::Unsupported => (
                DiagnosticCode::UnsupportedProfileSyntax,
                "native inline macro is preserved exactly but has no typed v1 renderer".to_owned(),
            ),
            _ => continue,
        };
        if !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code && diagnostic.range == inline.range)
        {
            diagnostics.push(Diagnostic {
                code,
                range: inline.range.clone(),
                message,
            });
        }
    }
}

fn offset_location(location: SourceLocation, offset: usize, source_len: usize) -> Range<usize> {
    clamp_range(
        (offset + location.start as usize)..(offset + location.end as usize),
        source_len,
    )
}

fn coalesce_location(location: &MultiSourceLocation) -> Option<SourceLocation> {
    (location.start_depth == location.end_depth).then_some(SourceLocation {
        start: location.start_pos,
        end: location.end_pos,
        include_depth: location.start_depth,
    })
}

fn clamp_range(range: Range<usize>, length: usize) -> Range<usize> {
    range.start.min(length)..range.end.min(length)
}

fn as_u64_range(range: Range<usize>) -> Range<u64> {
    range.start as u64..range.end as u64
}

fn inside_ranges(position: usize, ranges: &[Range<u64>]) -> bool {
    ranges
        .iter()
        .any(|range| range.start <= position as u64 && (position as u64) < range.end)
}

fn line_start(source: &str, position: usize) -> usize {
    source[..position.min(source.len())]
        .rfind('\n')
        .map_or(0, |index| index + 1)
}

fn line_end(source: &str, position: usize) -> usize {
    source[position.min(source.len())..]
        .find('\n')
        .map_or(source.len(), |index| position.min(source.len()) + index + 1)
}

#[derive(Clone, Copy)]
struct Line<'a> {
    start: usize,
    end: usize,
    full_end: usize,
    text: &'a str,
}

fn lines(source: &str) -> impl Iterator<Item = Line<'_>> {
    let mut start = 0;
    std::iter::from_fn(move || {
        if start >= source.len() {
            return None;
        }
        let relative_end = source[start..].find('\n');
        let full_end = relative_end.map_or(source.len(), |index| start + index + 1);
        let mut end = relative_end.map_or(source.len(), |index| start + index);
        if end > start && source.as_bytes()[end - 1] == b'\r' {
            end -= 1;
        }
        let line = Line {
            start,
            end,
            full_end,
            text: &source[start..end],
        };
        start = full_end;
        Some(line)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const UUID: &str = "550e8400-e29b-41d4-a716-446655440000";

    #[test]
    fn marker_absence_and_unknown_bytes_fail_closed() {
        assert_eq!(
            probe_generation_marker(None),
            Err(GenerationMarkerError::Missing)
        );
        assert_eq!(
            probe_generation_marker(Some(GENERATION_MARKER_V1)),
            Ok(GenerationProbe::AsciiDocV1)
        );
        assert!(matches!(
            probe_generation_marker(Some(b"future\n")),
            Err(GenerationMarkerError::Unknown(_))
        ));
    }

    #[test]
    fn preserves_exact_utf8_and_maps_profile_ranges() {
        let source = format!(
            "---\r\n_weftext:\r\n  id: \"{UUID}\"\r\n---\r\n= 标题 😀\r\n:description: مرحبا\r\n\r\n== 第一节\r\n正文\r\n"
        );
        let analysis = analyze(&source);
        assert_eq!(analysis.profile, PROFILE_ID);
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.kind == BlockKind::Frontmatter)
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.kind == BlockKind::DocumentTitle)
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.heading_level == Some(1))
        );
        for block in &analysis.blocks {
            assert!(source.is_char_boundary(usize::try_from(block.range.start).unwrap()));
            assert!(source.is_char_boundary(usize::try_from(block.range.end).unwrap()));
        }
    }

    #[test]
    fn recognizes_h1_through_h9_and_run_in_role() {
        let source = "= Main title: Subtitle\n\n[.run-in]\n== H1\n======= H6\n========== H9\n";
        let analysis = analyze(source);
        let title = analysis
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::DocumentTitle)
            .unwrap();
        let subtitle = analysis
            .blocks
            .iter()
            .find(|block| block.kind == BlockKind::DocumentSubtitle)
            .unwrap();
        assert_eq!(title.text, "Main title");
        assert_eq!(subtitle.text, "Subtitle");
        assert_eq!(
            &source[usize::try_from(subtitle.text_range.start).unwrap()
                ..usize::try_from(subtitle.text_range.end).unwrap()],
            "Subtitle"
        );
        assert!(
            analysis
                .safe_html
                .contains("data-document-subtitle=\"true\">Subtitle")
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| { block.heading_level == Some(1) && block.roles == ["run-in"] })
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.heading_level == Some(6))
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.heading_level == Some(9))
        );
    }

    #[test]
    fn recognizes_quotes_node_links_xrefs_and_notes_outside_protected_blocks() {
        let source = format!(
            "> > quoted\nnode:{UUID}#part[display] xref:local[Local] footnote:n[Note] endnote:e[End]\n\n[source]\n----\nnode:{UUID}[hidden]\n----\n"
        );
        let analysis = analyze(&source);
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.quote_depth == Some(2))
        );
        assert_eq!(
            analysis
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::Node)
                .count(),
            1
        );
        assert!(
            analysis
                .links
                .iter()
                .any(|link| link.kind == LinkKind::Xref)
        );
        assert!(
            analysis
                .links
                .iter()
                .any(|link| link.kind == LinkKind::Footnote)
        );
        assert!(
            analysis
                .links
                .iter()
                .any(|link| link.kind == LinkKind::Endnote)
        );
    }

    #[test]
    fn every_typed_inline_range_is_an_exact_utf8_byte_range_after_cjk_text() {
        let source = format!(
            "= 标题 😀\n\n正文 *粗体* 与 link:https://example.test[站点]，以及 node:{UUID}[本节点]。\n"
        );
        let analysis = analyze(&source);
        let invalid = analysis
            .inlines
            .iter()
            .filter(|inline| {
                let start = usize::try_from(inline.range.start).unwrap();
                let end = usize::try_from(inline.range.end).unwrap();
                start > end
                    || end > source.len()
                    || !source.is_char_boundary(start)
                    || !source.is_char_boundary(end)
            })
            .map(|inline| (inline.kind, inline.range.clone()))
            .collect::<Vec<_>>();
        assert!(
            invalid.is_empty(),
            "invalid ranges {invalid:?}; all inlines: {:?}",
            analysis
                .inlines
                .iter()
                .map(|inline| (inline.kind, inline.range.clone()))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn distinguishes_node_embed_from_inline_node_link() {
        let source = format!("node::{UUID}[]\nnode:{UUID}[link]\n");
        let analysis = analyze(&source);
        assert_eq!(
            analysis
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::NodeEmbed)
                .count(),
            1
        );
        assert_eq!(
            analysis
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::Node)
                .count(),
            1
        );
    }

    #[test]
    fn safe_boundary_reports_active_content_and_escapes_html() {
        let source =
            "include::https://example.invalid/secret.adoc[]\n\npass:[<script>alert(1)</script>]\n";
        let analysis = analyze(source);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::UnsafeInclude)
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::RemoteUri)
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::PassthroughDisabled)
        );
        assert!(!analysis.safe_html.contains("<script>"));
        assert!(analysis.safe_html.contains("&lt;script&gt;"));
    }

    #[test]
    fn math_and_mermaid_are_literal_derived_views() {
        let source =
            "[stem]\n----\nlatexmath:[x < y]\n----\n\n[mermaid]\n----\ngraph TD; A-->B\n----\n";
        let analysis = analyze(source);
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.kind == BlockKind::Math)
        );
        assert!(
            analysis
                .blocks
                .iter()
                .any(|block| block.kind == BlockKind::Mermaid)
        );
        assert!(analysis.safe_html.contains("data-weftext-math"));
        assert!(analysis.safe_html.contains("data-weftext-diagram"));
        assert!(analysis.safe_html.contains("x &lt; y"));
    }

    #[test]
    fn malformed_source_remains_exact_with_diagnostics() {
        let source = "---\n_weftext:\n  id: broken\n= still bytes\n";
        let analysis = analyze(source);
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == DiagnosticCode::UnclosedFrontmatter)
        );
        assert_eq!(analysis.protected_ranges.len(), 1);
        assert_eq!(analysis.protected_ranges[0], 0..source.len() as u64);
    }

    #[test]
    fn inline_passthroughs_are_exact_protected_ranges() {
        let source = "= Pass\n\npass:[cite:[hidden2024]] and +cite:[alsohidden2024]+.\n\nVisible cite:[shown2024].\n";
        let analysis = analyze(source);
        for needle in ["cite:[hidden2024]", "cite:[alsohidden2024]"] {
            let start = source.find(needle).unwrap() as u64;
            let end = start + needle.len() as u64;
            assert!(
                analysis
                    .protected_ranges
                    .iter()
                    .any(|range| range.start <= start && range.end >= end),
                "{needle} was not protected: {:?}",
                analysis.protected_ranges
            );
        }
        let visible = source.find("cite:[shown2024]").unwrap() as u64;
        assert!(
            analysis
                .protected_ranges
                .iter()
                .all(|range| !(range.start <= visible && range.end > visible))
        );
    }

    #[test]
    fn source_edit_plan_is_narrow_utf8_safe_and_revision_length_checked() {
        let source = "== 标题 😀\nbody\n";
        let start = source.find("标题").unwrap();
        let end = start + "标题".len();
        let plan = SourceEditPlan::new(
            source,
            vec![SourceEdit {
                range: start..end,
                replacement: "新标题".to_owned(),
            }],
        )
        .unwrap();
        assert_eq!(plan.apply(source).unwrap(), "== 新标题 😀\nbody\n");
        assert!(plan.apply("different length").is_none());
        assert!(matches!(
            SourceEditPlan::new(
                source,
                vec![SourceEdit {
                    range: start + 1..end,
                    replacement: String::new()
                }]
            ),
            Err(EditPlanError::NotUtf8Boundary(_))
        ));
    }

    #[test]
    fn five_thousand_section_performance_baseline_preserves_all_links() {
        use std::time::Instant;

        let mut source = String::from("= Performance baseline\n\n");
        for index in 0..5_000 {
            let _ = writeln!(source, "== Section {index}\nnode:{UUID}[Target]\n");
        }
        let started = Instant::now();
        let analysis = analyze(&source);
        let elapsed = started.elapsed();

        assert_eq!(
            analysis
                .links
                .iter()
                .filter(|link| link.kind == LinkKind::Node)
                .count(),
            5_000
        );
        assert!(analysis.blocks.len() >= 5_001);
        eprintln!(
            "R1B baseline: {} bytes, {} blocks, {} links in {elapsed:?}",
            source.len(),
            analysis.blocks.len(),
            analysis.links.len()
        );
    }

    #[test]
    fn parser_hardening_fixture_identifies_supported_migration_constructs() {
        let uuid = UUID;
        let cases = [
            "[#root-heading]\n== Root heading\n".to_owned(),
            format!("See node:{uuid}[子节点].\n"),
            "[source]\n----\n# literal heading\n----\n".to_owned(),
            "[source,rust]\n # literal heading\n \n [[Child]]\n\n".to_owned(),
        ];
        for source in cases {
            let analysis = analyze(&source);
            assert!(
                !analysis.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == DiagnosticCode::ParserError
                        && diagnostic.message.contains("aborted")
                }),
                "parser aborted for {source:?}"
            );
        }
    }

    #[test]
    fn parser_abort_protects_the_complete_body_fail_closed() {
        let source = concat!(
            "[source]\n----\n",
            "[.weftext-query,version=1,view=task-list]\n....\n",
            "from tasks as task\nscope workspace\nwhere task.closed = false\n",
            "select task.title\norder by task.title asc\nlimit 100\n....\n",
            "----\n",
        );
        let analysis = analyze(source);
        assert!(analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::ParserError
                && diagnostic.message.contains("failed closed")
        }));
        assert!(
            analysis
                .protected_ranges
                .iter()
                .any(|range| range.start == 0 && range.end == source.len() as u64)
        );
    }
}
