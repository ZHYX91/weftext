export type SourceRange = { start: number; end: number };

export type DocumentAnalysisStatus = "complete" | "degraded" | "failed";
export type DocumentBlockKind =
  | "frontmatter"
  | "document_title"
  | "document_subtitle"
  | "heading"
  | "paragraph"
  | "listing"
  | "literal"
  | "fenced_code"
  | "quote"
  | "list"
  | "table"
  | "image"
  | "block_title"
  | "thematic_break"
  | "html"
  | "math"
  | "mermaid"
  | "passthrough"
  | "comment"
  | "unsupported";

export type DocumentListKind = "ordered" | "unordered" | "description" | "callout";
export type DocumentMathNotation = "ascii_math" | "latex_math";
export type DocumentTableCellStyle = "ascii_doc" | "default" | "emphasis" | "header" | "literal" | "monospace" | "strong";
export type DocumentHorizontalCellAlignment = "left" | "center" | "right";
export type DocumentVerticalCellAlignment = "top" | "middle" | "bottom";

export type DocumentListItem = {
  range: SourceRange;
  textRange: SourceRange;
  marker: string;
  text: string;
  depth: number;
  checked: boolean | null;
  children: DocumentListItem[];
  unmodeledContinuations: SourceRange[];
};

export type DocumentListModel = {
  kind: DocumentListKind;
  depth: number;
  items: DocumentListItem[];
};

export type DocumentTableCell = {
  text: string;
  columnSpan: number;
  rowSpan: number;
  style: DocumentTableCellStyle;
  horizontalAlignment: DocumentHorizontalCellAlignment;
  verticalAlignment: DocumentVerticalCellAlignment;
  nestedAsciidoc: boolean;
};

export type DocumentTableRow = { cells: DocumentTableCell[] };
export type DocumentTableModel = {
  header: DocumentTableRow | null;
  body: DocumentTableRow[];
  footer: DocumentTableRow | null;
  columnCount: number;
};

export type DocumentBlockSemantic =
  | { kind: "frontmatter" }
  | { kind: "document_title" }
  | { kind: "document_subtitle" }
  | { kind: "heading"; level: number }
  | { kind: "paragraph" }
  | { kind: "listing"; language: string | null }
  | { kind: "literal" }
  | { kind: "quote"; depth: number | null; attribution: string | null; citation: string | null }
  | { kind: "list"; model: DocumentListModel }
  | { kind: "table"; model: DocumentTableModel }
  | { kind: "image"; target: string; alt: string | null }
  | { kind: "block_title" }
  | { kind: "math"; notation: DocumentMathNotation }
  | { kind: "mermaid" }
  | { kind: "passthrough" }
  | { kind: "comment" }
  | { kind: "thematic_break" }
  | { kind: "unsupported"; context: string };

export type DocumentBlock = {
  kind: DocumentBlockKind;
  start: number;
  end: number;
  textStart: number;
  textEnd: number;
  text: string;
  headingLevel: number | null;
  quoteDepth: number | null;
  blockId: string | null;
  roles: string[];
  title: string | null;
  semantic: DocumentBlockSemantic;
};

export type DocumentInlineKind =
  | "anchor"
  | "bold"
  | "italic"
  | "monospace"
  | "highlight"
  | "superscript"
  | "subscript"
  | "quoted"
  | "role_span"
  | "passthrough"
  | "unsupported"
  | "xref"
  | "native_link"
  | "image"
  | "footnote"
  | "endnote"
  | "stem"
  | "latex_math"
  | "node"
  | "node_embed";

export type DocumentInlineSemantic = {
  kind: DocumentInlineKind;
  range: SourceRange;
  targetRange: SourceRange | null;
  target: string | null;
  fragment: string | null;
  text: string | null;
  notation: DocumentMathNotation | null;
  roles: string[];
};

export type DocumentDiagnosticCode =
  | "duplicate_block_id"
  | "unclosed_frontmatter"
  | "unclosed_fence"
  | "profile_warning"
  | "unsafe_active_content"
  | "parser_error"
  | "unsupported_profile_syntax";

export type DocumentDiagnostic = {
  code: DocumentDiagnosticCode;
  start: number;
  end: number;
  message: string;
};

export type DocumentDegradation = {
  kind: "parser_failure" | "unsupported_block" | "unsupported_inline" | "disabled_include" | "disabled_remote_uri" | "disabled_passthrough" | "constrained_math" | "constrained_mermaid";
  supportState: "full" | "constrained" | "preserve_only" | "prohibited_effect";
  range: SourceRange;
  fallback: "escaped_source" | "disabled_effect" | "no_derived_rendering";
  message: string;
};

export type DocumentModel = {
  semanticModelVersion: 1;
  status: DocumentAnalysisStatus;
  blocks: DocumentBlock[];
  inlines: DocumentInlineSemantic[];
  runInGroups: Array<{ headingBlock: number; bodyBlock: number }>;
  diagnostics: DocumentDiagnostic[];
  degradations: DocumentDegradation[];
  safeHtml: string;
};

export type DocumentProfileId = "ascii_doc_v1";

export type DocumentCapabilities = {
  exactSource: boolean;
  utf8SourceEdits: boolean;
  yamlEnvelope: boolean;
  maxHeadingLevel: number;
  actualQuoteDepth: boolean;
  blockIds: boolean;
  managedLinks: boolean;
  protectedRegions: boolean;
  typedBlocks: boolean;
  typedInlines: boolean;
  nestedLists: boolean;
  typedTables: boolean;
  safeRenderInput: boolean;
  degradationReports: boolean;
};

export type DocumentProfile = {
  contractVersion: 2;
  profile: DocumentProfileId;
  mediaType: "text/asciidoc";
  canonicalExtension: "adoc";
  capabilities: DocumentCapabilities;
};

export type DocumentViewModel = {
  contractVersion: 2;
  semanticModelVersion: 1;
  profile: DocumentProfileId;
  capabilities: DocumentCapabilities;
  status: DocumentAnalysisStatus;
  blocks: DocumentBlock[];
  inlines: DocumentInlineSemantic[];
  runInGroups: DocumentModel["runInGroups"];
  degradations: DocumentDegradation[];
  safeHtml: string;
};

export type WorkspaceDocumentFormat = {
  generation: "ascii_doc_v1";
  canonicalExtension: "adoc";
  mediaType: "text/asciidoc";
};

export type NodeMetadataDiagnostic = {
  code: "unknown_weftext_field";
  field: string;
  range: SourceRange;
  message: string;
};

export type NodeMetadataProjection = {
  schema: "weftext.node-metadata.v1";
  id: string;
  icon: string | null;
  resolvedIcon: { kind: "emoji" | "built_in"; value: string; glyph: string } | null;
  aliases: string[];
  childSort: "name" | "manual";
  childSortDirection: "ascending" | "descending";
  siblingRank: number | null;
  adjacentHeadingBody: "separate" | "run_in" | null;
  diagnostics: NodeMetadataDiagnostic[];
};

const BLOCK_KINDS = new Set<DocumentBlockKind>([
  "frontmatter", "document_title", "document_subtitle", "heading", "paragraph", "listing", "literal", "fenced_code", "quote", "list", "table", "image", "block_title", "thematic_break", "html", "math", "mermaid", "passthrough", "comment", "unsupported",
]);
const INLINE_KINDS = new Set<DocumentInlineKind>([
  "anchor", "bold", "italic", "monospace", "highlight", "superscript", "subscript", "quoted", "role_span", "passthrough", "unsupported", "xref", "native_link", "image", "footnote", "endnote", "stem", "latex_math", "node", "node_embed",
]);
const DIAGNOSTIC_CODES = new Set<DocumentDiagnosticCode>([
  "duplicate_block_id", "unclosed_frontmatter", "unclosed_fence", "profile_warning", "unsafe_active_content", "parser_error", "unsupported_profile_syntax",
]);
const DEGRADATION_KINDS = new Set<DocumentDegradation["kind"]>([
  "parser_failure", "unsupported_block", "unsupported_inline", "disabled_include", "disabled_remote_uri", "disabled_passthrough", "constrained_math", "constrained_mermaid",
]);
const SUPPORT_STATES = new Set<DocumentDegradation["supportState"]>(["full", "constrained", "preserve_only", "prohibited_effect"]);
const FALLBACKS = new Set<DocumentDegradation["fallback"]>(["escaped_source", "disabled_effect", "no_derived_rendering"]);
const MATH_NOTATIONS = new Set<DocumentMathNotation>(["ascii_math", "latex_math"]);

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasOnlyKeys(value: Record<string, unknown>, keys: readonly string[]) {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key)) && keys.every((key) => key in value);
}

function isNonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && Number(value) >= 0;
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isSourceRange(value: unknown): value is SourceRange {
  return isRecord(value)
    && isNonNegativeSafeInteger(value.start)
    && isNonNegativeSafeInteger(value.end)
    && Number(value.start) <= Number(value.end);
}

/** Validates the one Core-owned shallow node-metadata projection. */
export function isNodeMetadataProjection(value: unknown): value is NodeMetadataProjection {
  if (!isRecord(value) || !hasOnlyKeys(value, [
    "schema", "id", "icon", "resolvedIcon", "aliases", "childSort", "childSortDirection",
    "siblingRank", "adjacentHeadingBody", "diagnostics",
  ])) return false;
  if (value.schema !== "weftext.node-metadata.v1"
    || typeof value.id !== "string"
    || !/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(value.id)
    || (value.icon !== null && typeof value.icon !== "string")
    || !isStringArray(value.aliases)
    || !["name", "manual"].includes(String(value.childSort))
    || !["ascending", "descending"].includes(String(value.childSortDirection))
    || (value.siblingRank !== null && (!Number.isSafeInteger(value.siblingRank) || Number(value.siblingRank) <= 0))
    || ![null, "separate", "run_in"].includes(value.adjacentHeadingBody as null | string)
    || !Array.isArray(value.diagnostics)) return false;
  if (value.resolvedIcon !== null) {
    if (!isRecord(value.resolvedIcon)
      || !hasOnlyKeys(value.resolvedIcon, ["kind", "value", "glyph"])
      || !["emoji", "built_in"].includes(String(value.resolvedIcon.kind))
      || typeof value.resolvedIcon.value !== "string"
      || typeof value.resolvedIcon.glyph !== "string"
      || value.resolvedIcon.value !== value.icon) return false;
  }
  return value.diagnostics.every((diagnostic) => isRecord(diagnostic)
    && hasOnlyKeys(diagnostic, ["code", "field", "range", "message"])
    && diagnostic.code === "unknown_weftext_field"
    && typeof diagnostic.field === "string"
    && isSourceRange(diagnostic.range)
    && typeof diagnostic.message === "string");
}

function isListItem(value: unknown, depth = 0): value is DocumentListItem {
  if (depth > 128 || !isRecord(value) || !isSourceRange(value.range) || !isSourceRange(value.textRange)) return false;
  return typeof value.marker === "string"
    && typeof value.text === "string"
    && isNonNegativeSafeInteger(value.depth)
    && (value.checked === null || typeof value.checked === "boolean")
    && Array.isArray(value.children)
    && value.children.every((child) => isListItem(child, depth + 1))
    && Array.isArray(value.unmodeledContinuations)
    && value.unmodeledContinuations.every(isSourceRange);
}

function isListModel(value: unknown): value is DocumentListModel {
  return isRecord(value)
    && ["ordered", "unordered", "description", "callout"].includes(String(value.kind))
    && isNonNegativeSafeInteger(value.depth)
    && Array.isArray(value.items)
    && value.items.every((item) => isListItem(item));
}

function isTableRow(value: unknown): value is DocumentTableRow {
  return isRecord(value) && Array.isArray(value.cells) && value.cells.every((cell) => isRecord(cell)
    && typeof cell.text === "string"
    && isNonNegativeSafeInteger(cell.columnSpan) && Number(cell.columnSpan) > 0
    && isNonNegativeSafeInteger(cell.rowSpan) && Number(cell.rowSpan) > 0
    && ["ascii_doc", "default", "emphasis", "header", "literal", "monospace", "strong"].includes(String(cell.style))
    && ["left", "center", "right"].includes(String(cell.horizontalAlignment))
    && ["top", "middle", "bottom"].includes(String(cell.verticalAlignment))
    && typeof cell.nestedAsciidoc === "boolean");
}

function isTableModel(value: unknown): value is DocumentTableModel {
  return isRecord(value)
    && (value.header === null || isTableRow(value.header))
    && Array.isArray(value.body) && value.body.every(isTableRow)
    && (value.footer === null || isTableRow(value.footer))
    && isNonNegativeSafeInteger(value.columnCount);
}

function isBlockSemantic(value: unknown, blockKind: DocumentBlockKind): value is DocumentBlockSemantic {
  if (!isRecord(value)) return false;
  const semanticKind = String(value.kind);
  const expectedKind = blockKind === "fenced_code" ? "listing" : blockKind === "html" ? "unsupported" : blockKind;
  if (semanticKind !== expectedKind) return false;
  if (["frontmatter", "document_title", "document_subtitle", "paragraph", "literal", "block_title", "mermaid", "passthrough", "comment", "thematic_break"].includes(semanticKind)) return true;
  if (semanticKind === "heading") return isNonNegativeSafeInteger(value.level) && Number(value.level) >= 1 && Number(value.level) <= 9;
  if (semanticKind === "listing") return isNullableString(value.language);
  if (semanticKind === "quote") return (value.depth === null || isNonNegativeSafeInteger(value.depth)) && isNullableString(value.attribution) && isNullableString(value.citation);
  if (semanticKind === "list") return isListModel(value.model);
  if (semanticKind === "table") return isTableModel(value.model);
  if (semanticKind === "image") return typeof value.target === "string" && isNullableString(value.alt);
  if (semanticKind === "math") return MATH_NOTATIONS.has(value.notation as DocumentMathNotation);
  if (semanticKind === "unsupported") return typeof value.context === "string";
  return false;
}

function isDocumentBlock(value: unknown): value is DocumentBlock {
  if (!isRecord(value) || !BLOCK_KINDS.has(value.kind as DocumentBlockKind)) return false;
  const kind = value.kind as DocumentBlockKind;
  if (!isNonNegativeSafeInteger(value.start) || !isNonNegativeSafeInteger(value.end) || !isNonNegativeSafeInteger(value.textStart) || !isNonNegativeSafeInteger(value.textEnd)) return false;
  if (Number(value.start) > Number(value.textStart) || Number(value.textStart) > Number(value.textEnd) || Number(value.textEnd) > Number(value.end)) return false;
  if (typeof value.text !== "string" || !isNullableString(value.blockId) || !isStringArray(value.roles) || !isNullableString(value.title)) return false;
  if (value.headingLevel !== null && (!isNonNegativeSafeInteger(value.headingLevel) || Number(value.headingLevel) < 1 || Number(value.headingLevel) > 9)) return false;
  if (value.quoteDepth !== null && (!isNonNegativeSafeInteger(value.quoteDepth) || Number(value.quoteDepth) < 1)) return false;
  return isBlockSemantic(value.semantic, kind);
}

function isInline(value: unknown): value is DocumentInlineSemantic {
  return isRecord(value)
    && INLINE_KINDS.has(value.kind as DocumentInlineKind)
    && isSourceRange(value.range)
    && (value.targetRange === null || isSourceRange(value.targetRange))
    && isNullableString(value.target)
    && isNullableString(value.fragment)
    && isNullableString(value.text)
    && (value.notation === null || MATH_NOTATIONS.has(value.notation as DocumentMathNotation))
    && isStringArray(value.roles);
}

/** Validates the complete Core AsciiDoc semantic-model v1 wire contract. */
export function isDocumentModel(value: unknown): value is DocumentModel {
  if (!isRecord(value)
    || value.semanticModelVersion !== 1
    || !["complete", "degraded", "failed"].includes(String(value.status))
    || !Array.isArray(value.blocks) || !value.blocks.every(isDocumentBlock)
    || !Array.isArray(value.inlines) || !value.inlines.every(isInline)
    || !Array.isArray(value.runInGroups)
    || !Array.isArray(value.diagnostics)
    || !Array.isArray(value.degradations)
    || typeof value.safeHtml !== "string") return false;
  const blocks = value.blocks as DocumentBlock[];
  const groupsValid = value.runInGroups.every((group) => isRecord(group)
    && isNonNegativeSafeInteger(group.headingBlock)
    && isNonNegativeSafeInteger(group.bodyBlock)
    && Number(group.headingBlock) < blocks.length
    && Number(group.bodyBlock) < blocks.length
    && blocks[Number(group.headingBlock)]?.kind === "heading"
    && blocks[Number(group.bodyBlock)]?.kind === "paragraph");
  if (!groupsValid) return false;
  const diagnosticsValid = value.diagnostics.every((diagnostic) => isRecord(diagnostic)
    && DIAGNOSTIC_CODES.has(diagnostic.code as DocumentDiagnosticCode)
    && isNonNegativeSafeInteger(diagnostic.start)
    && isNonNegativeSafeInteger(diagnostic.end)
    && Number(diagnostic.start) <= Number(diagnostic.end)
    && typeof diagnostic.message === "string");
  if (!diagnosticsValid) return false;
  return value.degradations.every((degradation) => isRecord(degradation)
    && DEGRADATION_KINDS.has(degradation.kind as DocumentDegradation["kind"])
    && SUPPORT_STATES.has(degradation.supportState as DocumentDegradation["supportState"])
    && isSourceRange(degradation.range)
    && FALLBACKS.has(degradation.fallback as DocumentDegradation["fallback"])
    && typeof degradation.message === "string");
}

function isDocumentCapabilities(value: unknown): value is DocumentCapabilities {
  if (!isRecord(value) || value.maxHeadingLevel !== 9) return false;
  return [
    "exactSource", "utf8SourceEdits", "yamlEnvelope", "actualQuoteDepth", "blockIds", "managedLinks", "protectedRegions",
    "typedBlocks", "typedInlines", "nestedLists", "typedTables", "safeRenderInput", "degradationReports",
  ].every((field) => value[field] === true);
}

export function isDocumentProfile(value: unknown): value is DocumentProfile {
  return isRecord(value)
    && value.contractVersion === 2
    && value.profile === "ascii_doc_v1"
    && value.mediaType === "text/asciidoc"
    && value.canonicalExtension === "adoc"
    && isDocumentCapabilities(value.capabilities);
}

export function isDocumentViewModel(value: unknown): value is DocumentViewModel {
  if (!isRecord(value)
    || value.contractVersion !== 2
    || value.semanticModelVersion !== 1
    || value.profile !== "ascii_doc_v1"
    || !isDocumentCapabilities(value.capabilities)) return false;
  return isDocumentModel({
    semanticModelVersion: value.semanticModelVersion,
    status: value.status,
    blocks: value.blocks,
    inlines: value.inlines,
    runInGroups: value.runInGroups,
    diagnostics: [],
    degradations: value.degradations,
    safeHtml: value.safeHtml,
  });
}
