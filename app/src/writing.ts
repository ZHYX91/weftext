import { applySourcePatches, sourceBoundary, type SourceEditorEvent, type SourcePatch, type SourceSelection } from "./editor";
import { encodeLiteral } from "./session";
import type { RenderNode } from "./types";
import { ChangeSet } from "@codemirror/state";
import { DocumentNotesInputTracker } from "./personal-input-text";
import { personalInputId, type PersonalText } from "./source-input-origin";
import { sourceByteOffsets } from "./source-offsets";

/** Addresses are native-parser products scoped to one source, never saved IDs. */
export interface WritingRange { from: number; to: number; utf16From: number; utf16To: number; raw: string; writable: boolean }
export interface WritingLiteral { range: WritingRange; text: string; role: string; paragraphBreaks?: WritingRange[] }
export interface WritingTableCell { row: number; column: number; rowspan: number; colspan: number; body: WritingRange; range: WritingRange; header: boolean }
export interface WritingTable { columns: number; rows: number; cells: WritingRange[]; gridWritable: boolean; logicalCells?: WritingTableCell[]; topologyWritable?: boolean; unavailableReason?: string | null; headerRows?: number; headerColumns?: number }
export interface WritingStructure { kind: string; range: WritingRange; body?: WritingRange | null; level?: number | null; label?: string | null; language?: string | null; runIn: boolean; table?: WritingTable | null }
export interface WritingRole { status: string; text?: string | null; range?: WritingRange | null; construct?: WritingRange | null }
export interface WritingMap {
  bodyBlocks?: { kind: string; range: WritingRange; body: WritingRange }[];
  sourceHash: string; syntaxValid: boolean; literalRegions: WritingLiteral[]; structures: WritingStructure[];
  title: WritingRole; subtitle: WritingRole; bodyInsertion?: WritingRange | null; unavailableReason?: string | null;
  notes?: import("./document-notes").NotesSourceMap;
  headingNumbering?: import("./numbering-panel").HeadingNumberingInspection;
}
export type WritingBlock =
  | { kind: "image"; path: string; alt: string }
  | { kind: "paragraph"; text: string }
  | { kind: "heading"; text: string; level: number }
  | { kind: "bulletList" | "numberedList"; items: string[] }
  | { kind: "termList"; items: { term: string; description: string }[] }
  | { kind: "math"; text: string; display: boolean }
  | { kind: "table"; columns: number; cells: string[] }
  | { kind: "clipboardTable"; rows: number; columns: number; cells: { row: number; column: number; rowspan: number; colspan: number; text: string; header: boolean }[] }
  | { kind: "link"; target: string; text: string }
  | { kind: "raw"; text: string; language?: string | null; block: boolean };
export type TableAction =
  | { kind: "insertRow" | "removeRow" | "insertColumn" | "removeColumn"; index: number }
  | { kind: "moveRow" | "moveColumn"; from: number; to: number }
  | { kind: "paste"; row: number; column: number; text: string }
  | { kind: "clearRange" | "merge"; top: number; left: number; bottom: number; right: number }
  | { kind: "split"; row: number; column: number }
  | { kind: "setHeader"; rows: number; columns: number }
  | { kind: "transpose" }
  | { kind: "pasteRange"; row: number; column: number; text: string; expand: boolean }
  | { kind: "pasteCells"; row: number; column: number; rows: number; columns: number; cells: { row: number; column: number; rowspan: number; colspan: number; text: string; header: boolean }[]; expand: boolean };
export type WritingIntent =
  | { kind: "convertBodyBlock"; range: WritingRange; target: string; level?: number }
  | { kind: "moveBodyBlock"; range: WritingRange; direction: "up" | "down" }
  | import("./document-notes").DocumentNotesIntent
  | { kind: "headingNumbering"; action: import("./numbering-panel").HeadingNumberingIntent }
  | { kind: "replaceLiteral"; range: WritingRange; text: string }
  | { kind: "insertBlock"; at: number; block: WritingBlock }
  | { kind: "setHeadingLevel"; range: WritingRange; level: number }
  | { kind: "setTitle" | "setSubtitle"; text: string | null }
  | { kind: "setRunIn"; range: WritingRange; enabled: boolean }
  | { kind: "setInline"; range: WritingRange; format: "strong" | "emph" | "strike" }
  | { kind: "replaceMath"; range: WritingRange; text: string; display: boolean }
  | { kind: "replaceRaw"; range: WritingRange; text: string; language: string | null; block: boolean }
  | { kind: "setTableCell"; range: WritingRange; text: string }
  | { kind: "tableEdit"; range: WritingRange; action: TableAction }
  | { kind: "listEdit"; range: WritingRange; action: { kind: "split"; at: number } | { kind: "indent" | "outdent" } };
export interface WritingBasis { sessionId: string; source: string; serial: number }
export interface WritingSnapshot extends WritingBasis { generation: number }
export interface WritingSurfaceLease { basis(): WritingSnapshot; active(): boolean }
export interface WritingRequest extends WritingBasis { sourceHash: string; intent: WritingIntent }
export interface NativeWritingPatch { from: number; to: number; utf16From: number; utf16To: number; expected: string; insert: string }
export interface WritingResult { source: string; sourceHash: string; patches: NativeWritingPatch[]; selection: SourceSelection; map: WritingMap }
export interface WritingTransition { before: WritingSnapshot; after: WritingSnapshot; patches: readonly SourcePatch[]; map: WritingMap }
export interface WritingBuffer {
  readonly source: string; readonly generation: number; readonly selection: SourceSelection; readonly composing: boolean;
  applyPatches(source: string, patches: readonly SourcePatch[], selection?: SourceSelection, userEvent?: string, isolated?: boolean, personalText?: PersonalText): void;
  observe(listener: (event: SourceEditorEvent) => void): () => void;
  select(from: number, to: number, focus?: boolean): void; undo(): void; redo(): void;
}
export interface WritingState { pending: boolean; busy: boolean; composing: boolean; error: string | null; mapCurrent: boolean }
export interface WritingHost {
  actionAvailable?(): boolean;
  refreshMap?(basis: WritingBasis): Promise<WritingMap>;
  editor: WritingBuffer;
  basis(): WritingBasis;
  editable(): boolean;
  request(request: WritingRequest): Promise<WritingResult>;
  onState?(state: WritingState): void;
  onError?(error: Error): void;
  onPendingSource?(source: string): void;
  /** Pure author-entered text, excluding the initial protected literal value. */
  onPersonalInput?(id: string, text: string): void;
  onApplied?(result: WritingResult): void;
  debounceMs?: number;
}
interface MapBasis extends WritingBasis { generation: number; map: WritingMap }
interface LiteralInput {
  basis: MapBasis; region: WritingLiteral; text: string; acceptedText: string;
  selection: SourceSelection; generation: number; composing: boolean; compositionCommit: boolean;
  input?: HTMLTextAreaElement; failure: Error | null;
  append: boolean;
  personalText?: PersonalText;
  /** Checkpoint proposal only; it never becomes a rich-edit capability. */
  recoveryOnly?: { basis: MapBasis; range: WritingRange; prefix: string };
}
interface InputBinding { input: HTMLTextAreaElement; region: WritingLiteral; append: boolean; valid: boolean }

export function utf8Offset(source: string, utf16: number): number { return new TextEncoder().encode(source.slice(0, utf16)).length; }
/** Mirrors the native literal codec, including periods at numbered-list starts. */
export function encodeWritingLiteral(text: string): string {
  let encoded = "";
  for (const character of text) {
    const point = character.codePointAt(0)!;
    if (character === " " || character === "\t" || (point < 32 || point >= 0x7f && point <= 0x9f) && character !== "\n" && character !== "\r") encoded += `\\u{${point.toString(16)}}`;
    else encoded += encodeLiteral(character).replace(/\./g, "\\.");
  }
  return encoded;
}
/** Table-cell newlines are visible hard breaks, rather than native soft wraps. */
export function encodeWritingTableLiteral(text: string): string { return text.replace(/\r\n|\r/g, "\n").split("\n").map(encodeWritingLiteral).join("\\\n"); }
/** Map a native literal caret without guessing shorthand or generated strings. */
export function literalSourceOffset(raw: string, text: string, index: number): number | null {
  if (!Number.isSafeInteger(index) || index < 0 || index > text.length || !sourceBoundary(text, index)) return null;
  if (raw === text) return index;
  return literalStops(raw, text)?.get(index) ?? null;
}
export function literalTextOffset(raw: string, text: string, index: number): number | null {
  if (!Number.isSafeInteger(index) || index < 0 || index > raw.length || !sourceBoundary(raw, index)) return null;
  if (raw === text) return index;
  const stops = literalStops(raw, text);
  if (!stops) return null;
  for (const [textAt, sourceAt] of stops) if (sourceAt === index) return textAt;
  return null;
}
function literalStops(raw: string, text: string): Map<number, number> | null {
  let at = 0, decoded = ""; const stops = new Map<number, number>([[0, 0]]);
  while (at < raw.length) {
    let value: string;
    if (raw[at] === "\\") {
      const unicode = /^\\u\{([a-fA-F0-9]{1,6})\}/.exec(raw.slice(at));
      if (unicode) { const point = Number.parseInt(unicode[1], 16); if (point > 0x10ffff || point >= 0xd800 && point <= 0xdfff) return null; value = String.fromCodePoint(point); at += unicode[0].length; }
      else { const next = raw.codePointAt(at + 1); if (next == null) return null; value = String.fromCodePoint(next); at += 1 + value.length; }
    } else if (/\s/.test(raw[at])) {
      const whitespace = /^[\t \r\n]+/.exec(raw.slice(at));
      if (!whitespace) { value = raw[at++]; }
      else { const rawSpace = whitespace[0], remainder = text.slice(decoded.length); value = remainder.startsWith(rawSpace) ? rawSpace : (rawSpace.match(/\n/g)?.length ?? 0) >= 2 && remainder.startsWith("\n\n") ? "\n\n" : " "; at += rawSpace.length; }
    } else { value = String.fromCodePoint(raw.codePointAt(at)!); at += value.length; }
    decoded += value; stops.set(decoded.length, at);
  }
  return decoded === text ? stops : null;
}
/** Recovery uses the same unchanged prefix/suffix, without normalizing trivia. */
export function literalRecoveryRaw(raw: string, before: string, after: string, tableCell = false): string {
  if (before === after) return raw;
  let from = 0, oldTo = before.length, newTo = after.length;
  while (from < before.length && from < after.length && before[from] === after[from]) from++;
  while (from > 0 && (!sourceBoundary(before, from) || !sourceBoundary(after, from))) from--;
  while (oldTo > from && newTo > from && before[oldTo - 1] === after[newTo - 1]) { oldTo--; newTo--; }
  while (oldTo < before.length && (!sourceBoundary(before, oldTo) || !sourceBoundary(after, newTo))) { oldTo++; newTo++; }
  const sourceFrom = literalSourceOffset(raw, before, from), sourceTo = literalSourceOffset(raw, before, oldTo);
  const encode = tableCell ? encodeWritingTableLiteral : encodeWritingLiteral;
  return sourceFrom == null || sourceTo == null ? encode(after) : raw.slice(0, sourceFrom) + encode(after.slice(from, newTo)) + raw.slice(sourceTo);
}
/** Textarea normalizes EOL; preserve original raw-code bytes outside the edit. */
export function textareaSourceDraft(source: string, text: string): string {
  const normalized = source.replace(/\r\n|\r/g, "\n");
  if (normalized === text) return source;
  let from = 0, oldTo = normalized.length, newTo = text.length;
  while (from < normalized.length && from < text.length && normalized[from] === text[from]) from++;
  while (from > 0 && (!sourceBoundary(normalized, from) || !sourceBoundary(text, from))) from--;
  while (oldTo > from && newTo > from && normalized[oldTo - 1] === text[newTo - 1]) { oldTo--; newTo--; }
  while (oldTo < normalized.length && (!sourceBoundary(normalized, oldTo) || !sourceBoundary(text, newTo))) { oldTo++; newTo++; }
  const originalAt = (index: number) => { let at = 0, logical = 0; while (logical < index) { if (source[at] === "\r" && source[at + 1] === "\n") at += 2; else at++; logical++; } return at; };
  const eol = /\r\n|\n|\r/.exec(source)?.[0] ?? "\n";
  return source.slice(0, originalAt(from)) + text.slice(from, newTo).replace(/\n/g, eol) + source.slice(originalAt(oldTo));
}
export function exactWritingRange(source: string, range: WritingRange): boolean {
  return matchingWritingRange(source, range, offset => utf8Offset(source, offset));
}
function matchingWritingRange(source: string, range: WritingRange, byteOffset: (offset: number) => number): boolean {
  return Number.isSafeInteger(range.utf16From) && Number.isSafeInteger(range.utf16To) && range.utf16From >= 0 && range.utf16To >= range.utf16From && range.utf16To <= source.length && sourceBoundary(source, range.utf16From) && sourceBoundary(source, range.utf16To) && source.slice(range.utf16From, range.utf16To) === range.raw && byteOffset(range.utf16From) === range.from && byteOffset(range.utf16To) === range.to;
}
function writingRangeValidator(source: string): (range: WritingRange) => boolean {
  const offsets = sourceByteOffsets(source);
  return range => matchingWritingRange(source, range, offset => offsets[offset]);
}
export function writingRange(source: string, from: number, to: number): WritingRange { return { from: utf8Offset(source, from), to: utf8Offset(source, to), utf16From: from, utf16To: to, raw: source.slice(from, to), writable: true }; }

/** HTTP/worker output is checked atomically before it can enter DraftUndo. */
export function validateWritingResult(source: string, result: WritingResult): SourcePatch[] {
  if (!result || !result.map || result.sourceHash !== result.map.sourceHash || typeof result.source !== "string") throw new Error("原生编辑结果缺少一致的当前编辑地图。");
  const offsets = sourceByteOffsets(source);
  const patches = result.patches.map(patch => {
    if (offsets[patch.utf16From] !== patch.from || offsets[patch.utf16To] !== patch.to) throw new Error("原生编辑的字节与 UTF-16 范围不一致。");
    return { from: patch.utf16From, to: patch.utf16To, expected: patch.expected, insert: patch.insert };
  });
  if (applySourcePatches(source, patches) !== result.source) throw new Error("原生编辑提案与完整草稿不一致，未应用修改。");
  const { from, to } = result.selection;
  if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to < from || to > result.source.length || !sourceBoundary(result.source, from) || !sourceBoundary(result.source, to)) throw new Error("原生编辑未返回有效选区。");
  const validRange = writingRangeValidator(result.source);
  for (const region of result.map.literalRegions) {
    if (!validRange(region.range)) throw new Error("原生编辑返回了不匹配的文字范围。");
    for (const separator of region.paragraphBreaks ?? []) if (!validRange(separator) || separator.utf16From < region.range.utf16From || separator.utf16To > region.range.utf16To) throw new Error("原生编辑返回了不匹配的段落边界。");
  }
  for (const structure of result.map.structures) if (!validRange(structure.range)) throw new Error("原生编辑返回了不匹配的结构范围。");
  return patches;
}

/** TSV stays a single all-or-nothing native command, including repeated cells. */
export function rectangularTSV(text: string): string[][] {
  const rows = text.replace(/\r\n/g, "\n").replace(/\r/g, "\n").replace(/\n$/, "").split("\n").map(row => row.split("\t"));
  const columns = rows[0].length;
  if (!columns || rows.some(row => row.length !== columns)) throw new Error("表格粘贴需要每行列数一致的 TSV。");
  if (rows.length * columns > 10000) throw new Error("这次表格粘贴超过 10,000 个单元格。");
  return rows;
}

export interface WritingRenderGroup { region: WritingLiteral; firstKey: string; memberKeys: string[] }
export interface WritingEmptySlot { nodeKey: string; region: WritingLiteral }
/** First native structure at an exact range, matching the parser's original order. */
export function writingStructureIndex(map: WritingMap | null): Map<string, WritingStructure> {
  const index = new Map<string, WritingStructure>();
  for (const structure of map?.structures ?? []) { const key = `${structure.range.utf16From}:${structure.range.utf16To}`; if (!index.has(key)) index.set(key, structure); }
  return index;
}
/** Join only actual native leaves with unique source coverage and equal semantics. */
export function groupWritingRenderNodes(tree: RenderNode, map: WritingMap, source: string): WritingRenderGroup[] {
  const leaves: RenderNode[] = []; const paragraphs = new Map<string, string>();
  const visit = (node: RenderNode, paragraph = "") => { if (node.kind === "element" && node.tag === "p") paragraph = node.key; if (node.kind === "text") { leaves.push(node); paragraphs.set(node.key, paragraph); } for (const child of node.children) visit(child, paragraph); };
  visit(tree);
  const validRange = writingRangeValidator(source);
  // Source order narrows candidates; restore actual render order before proving
  // coverage, so reordered or duplicate occurrences still cannot gain a writer.
  const bySource = leaves.map((node, order) => ({ node, order })).filter(item => item.node.origin?.isMain && Number.isSafeInteger(item.node.origin.utf16From) && Number.isSafeInteger(item.node.origin.utf16To)).sort((a, b) => a.node.origin!.utf16From - b.node.origin!.utf16From || a.order - b.order);
  const lowerBound = (from: number) => { let left = 0, right = bySource.length; while (left < right) { const mid = (left + right) >>> 1; if (bySource[mid].node.origin!.utf16From < from) left = mid + 1; else right = mid; } return left; };
  const groups: WritingRenderGroup[] = [];
  for (const region of map.literalRegions) {
    if (!region.range.writable || !validRange(region.range)) continue;
    const candidates: typeof bySource = [];
    for (let i = lowerBound(region.range.utf16From); i < bySource.length && bySource[i].node.origin!.utf16From <= region.range.utf16To; i++) if (bySource[i].node.origin!.utf16To <= region.range.utf16To) candidates.push(bySource[i]);
    const selected = candidates.sort((a, b) => a.order - b.order).map(item => item.node);
    if (!selected.length || selected.some(node => !node.writable || !node.origin || source.slice(node.origin.utf16From, node.origin.utf16To) !== node.origin.raw)) continue;
    let position = region.range.utf16From;
    let complete = true, text = ""; let previous: RenderNode | null = null;
    // Official Parbreak ranges have no HTML Text leaf. Accept such a gap only
    // when the actual consumer tree also crosses a native paragraph boundary.
    for (const node of selected) {
      const origin = node.origin!;
      if (origin.utf16From !== position) {
        const certified = region.paragraphBreaks?.filter(range => range.utf16From >= position && range.utf16To <= origin.utf16From).sort((a, b) => a.utf16From - b.utf16From) ?? [];
        let gapEnd = position;
        for (const range of certified) { if (range.utf16From !== gapEnd || !validRange(range)) { complete = false; break; } gapEnd = range.utf16To; }
        if (!complete || gapEnd !== origin.utf16From || !previous || !paragraphs.get(previous.key) || !paragraphs.get(node.key) || paragraphs.get(previous.key) === paragraphs.get(node.key)) { complete = false; break; }
        text += certified.map(() => "\n\n").join("");
      }
      text += node.text ?? ""; position = origin.utf16To; previous = node;
    }
    if (!complete || text !== region.text || position !== region.range.utf16To || new Set(selected.map(node => node.key)).size !== selected.length) continue;
    groups.push({ region, firstKey: selected[0].key, memberKeys: selected.map(node => node.key) });
  }
  return groups;
}

/** Empty inputs require an actual unique native owner, including exact cell identity. */
export function groupWritingEmptySlots(tree: RenderNode, map: WritingMap, source: string): WritingEmptySlot[] {
  const nodes: RenderNode[] = [];
  const visit = (node: RenderNode) => { if (node.kind === "element") nodes.push(node); for (const child of node.children) visit(child); };
  visit(tree);
  const validRange = writingRangeValidator(source);
  const slots: WritingEmptySlot[] = [];
  for (const region of map.literalRegions) {
    const range = region.range;
    if (!range.writable || range.utf16From !== range.utf16To || region.text !== "" || !validRange(range)) continue;
    const owner = map.structures.filter(structure => structure.body && structure.body.utf16From <= range.utf16From && structure.body.utf16To >= range.utf16To && ["heading", "bulletListItem", "numberedListItem", "termListItem"].includes(structure.kind)).sort((a, b) => a.range.utf16To - a.range.utf16From - (b.range.utf16To - b.range.utf16From))[0];
    let matches: RenderNode[] = [];
    if (owner && validRange(owner.range)) matches = nodes.filter(node => {
      const origin = node.origin, tag = node.tag?.toLowerCase();
      const semanticTag = owner.kind === "heading" ? /^h[1-6]$/.test(tag ?? "") || node.attrs.role === "heading" : owner.kind === "termListItem" ? tag === "dd" : tag === "li";
      return semanticTag && origin?.isMain && origin.utf16From === owner.range.utf16From && origin.utf16To === owner.range.utf16To && origin.raw === owner.range.raw;
    });
    else {
      // A table-wide origin cannot identify one of several empty cells. Only
      // a cell/content-block's exact native provenance may open its writer.
      const cell = map.structures.flatMap(structure => structure.table?.cells ?? []).filter(cell => cell.utf16From === range.utf16From && cell.utf16To === range.utf16To && cell.raw === range.raw);
      if (cell.length === 1) matches = nodes.filter(node => {
        const origin = node.origin;
        if (!["td", "th"].includes(node.tag ?? "") || !origin?.isMain || source.slice(origin.utf16From, origin.utf16To) !== origin.raw) return false;
        return origin.utf16From === range.utf16From && origin.utf16To === range.utf16To || origin.syntaxKind === "ContentBlock" && origin.raw === "[]" && origin.utf16From + 1 === range.utf16From && origin.utf16To - 1 === range.utf16To;
      });
    }
    if (matches.length === 1) slots.push({ nodeKey: matches[0].key, region });
  }
  return slots.filter(slot => slots.filter(other => other.nodeKey === slot.nodeKey).length === 1);
}

/** Shares the real source editor/history. Maps are disposable native products. */
export class WritingController {
  private mapBasis: MapBasis | null = null;
  private active: LiteralInput | null = null;
  private running: Promise<void> | null = null;
  private timer?: ReturnType<typeof setTimeout>;
  private installing = false;
  private disposed = false;
  private stopObserve: () => void;
  private afterInput: { run: () => void | Promise<void>; resolve: () => void; reject: (error: unknown) => void }[] = [];
  private failure: Error | null = null;
  private bindings = new Set<InputBinding>();
  private focusSelection = false;
  private surfaces = new Set<{ element: HTMLElement; update(transition?: WritingTransition): void; lease?: WritingSurfaceLease; connected: boolean }>();

  constructor(private host: WritingHost) {
    this.stopObserve = host.editor.observe(event => {
      if (event.kind === "source" && !this.installing) {
        this.mapBasis = null;
        if (this.active && this.active.text !== this.active.acceptedText) this.report(new Error("源码已变化，可视输入仍保留，请先复制文字再合并。"));
        else {
          this.active = null;
          // Undo/Redo and source edits retire the old visual values. Keeping a
          // focused old textarea mounted would suppress the current preview
          // forever and let its next input target a stale native range.
          for (const binding of this.bindings) {
            binding.valid = false; binding.input.readOnly = true;
            if (typeof document !== "undefined" && document.activeElement === binding.input) { this.focusSelection = true; binding.input.blur(); }
          }
          this.bindings.clear();
        }
      }
      this.notify();
      if (!event.composing && !this.active?.composing) void this.drainDeferred();
    });
  }
  get map(): WritingMap | null { return this.currentMap()?.map ?? null; }
  get state(): WritingState { return { pending: !!this.active && this.active.text !== this.active.acceptedText, busy: !!this.running, composing: this.host.editor.composing || !!this.active?.composing, error: this.failure?.message ?? this.active?.failure?.message ?? null, mapCurrent: !!this.currentMap() }; }
  get pendingText(): string | null { return this.state.pending ? this.active!.text : null; }
  get activeRange(): WritingRange | null { return this.active?.region.range ?? null; }
  get sourceSelection(): SourceSelection { return this.host.editor.selection; }
  restoreNavigationFocus(): void { this.focusSelection = true; }
  async prepareCommand(): Promise<void> {
    await this.flush();if(this.host.actionAvailable?.()===false)throw new Error("当前笔记已切换，未打开原笔记的旧操作。"); if(this.currentMap())return;
    const basis=this.host.basis();
    if(!this.host.refreshMap)throw new Error("请等待当前原生编辑地图后重试。");
    const map=await this.host.refreshMap(basis);
    if(this.host.actionAvailable?.()===false)throw new Error("准备期间笔记已切换，原稿保留。");
    if(!this.installMap(map,basis.source,basis.serial))throw new Error("准备操作期间出现新输入，请重新选择；原稿已保留。");
  }
  get snapshot(): WritingSnapshot { return { ...this.host.basis(), generation: this.host.editor.generation }; }
  isSnapshotCurrent(snapshot: WritingSnapshot): boolean { const now = this.snapshot; return now.sessionId === snapshot.sessionId && now.source === snapshot.source && now.serial === snapshot.serial && now.generation === snapshot.generation; }
  /** Valid literal input and qualified commands retain presentation; idle grid focus does not. */
  get retainInput(): boolean {
    return typeof document !== "undefined" && (
      [...this.bindings].some(binding => binding.valid && binding.input.isConnected && document.activeElement === binding.input)
      || !!this.currentMap() && [...this.surfaces].some(surface => surface.element.isConnected && surface.lease && this.isSnapshotCurrent(surface.lease.basis()) && surface.lease.active())
    ) || !!this.active?.input && (this.state.pending || this.state.busy || this.state.composing);
  }
  /** A surface's explicit command/gesture lease uses its existing qualified native snapshot. */
  observeSurface(element: HTMLElement, update: (transition?: WritingTransition) => void, lease?: WritingSurfaceLease): () => void {
    const surface = { element, update, lease, connected: false }; this.surfaces.add(surface);
    const blur = () => queueMicrotask(() => this.notify()); element.addEventListener("focusout", blur);
    return () => { this.surfaces.delete(surface); element.removeEventListener("focusout", blur); };
  }
  /** Notify view hosts after a surface releases a non-text focus/gesture lease. */
  surfaceStateChanged(): void { if (!this.disposed) this.notify(); }
  selectRange(range: WritingRange): void { const basis = this.currentMap(); if (!basis || !exactWritingRange(basis.source, range)) throw new Error("表格来源已变化，请等待当前预览。"); this.host.editor.select(range.utf16From, range.utf16To, false); }
  installMap(map: WritingMap, source: string, serial: number): boolean {
    const basis = this.host.basis();
    if (basis.source !== source || basis.serial !== serial || this.host.editor.source !== source || !map || !map.sourceHash) return false;
    this.mapBasis = { ...basis, generation: this.host.editor.generation, map };
    this.failure = null; this.notify(); return true;
  }
  private currentMap(): MapBasis | null {
    const basis = this.host.basis(), saved = this.mapBasis;
    return saved && saved.sessionId === basis.sessionId && saved.serial === basis.serial && saved.source === basis.source && saved.generation === this.host.editor.generation && this.host.editor.source === basis.source ? saved : null;
  }
  private requireMap(): MapBasis {
    if (!this.host.editable()) throw new Error("当前视图只读，请切换到编辑视图。");
    const basis = this.currentMap();
    if (!basis) throw new Error("当前编辑地图正在更新，请等待最新预览或使用源码编辑。");
    return basis;
  }
  private unchanged(basis: MapBasis): boolean {
    const current = this.host.basis();
    return !this.disposed && current.sessionId === basis.sessionId && current.serial === basis.serial && current.source === basis.source && this.host.editor.source === basis.source && this.host.editor.generation === basis.generation && this.host.editable();
  }
  private notify(): void {
    this.host.onState?.(this.state);
    for (const surface of this.surfaces) {
      if (surface.element.isConnected) { surface.connected = true; surface.update(); }
      else if (surface.connected) this.surfaces.delete(surface);
    }
  }
  private report(error: unknown): Error { const failure = error instanceof Error ? error : new Error(String(error)); this.failure = failure; this.host.onError?.(failure); this.notify(); return failure; }
  private apply(basis: MapBasis, result: WritingResult, event: string, isolated: boolean, selection = result.selection, personalText?: PersonalText): void {
    if (!this.unchanged(basis)) throw new Error("草稿在处理期间已变化，旧提案未应用；当前输入保留。");
    const patches = validateWritingResult(basis.source, result);
    this.installing = true;
    try { this.host.editor.applyPatches(basis.source, patches, selection, event, isolated, personalText); } finally { this.installing = false; }
    const current = this.host.basis();
    if (current.source !== result.source || this.host.editor.source !== result.source) throw new Error("共享草稿未接受原生编辑结果。");
    const transition: WritingTransition = { before: basis, after: this.snapshot, patches, map: result.map };
    for (const surface of this.surfaces) if (surface.element.isConnected) surface.update(transition);
    this.installMap(result.map, current.source, current.serial);
    this.refreshBindings(basis.source, patches, result.map);
    this.host.onApplied?.(result);
  }
  private refreshBindings(source: string, patches: SourcePatch[], map: WritingMap): void {
    const changes = ChangeSet.of(patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), source.length, "\n");
    for (const binding of this.bindings) {
      if (!binding.input.isConnected) { this.bindings.delete(binding); continue; }
      if (binding.input === this.active?.input) continue;
      if (binding.append && map.bodyInsertion?.writable) { binding.region = { range: map.bodyInsertion, text: "", role: "bodyInsertion" }; binding.valid = true; continue; }
      const old = binding.region.range;
      const affected = patches.some(patch => patch.from < old.utf16To && patch.to > old.utf16From || patch.from === patch.to && patch.from > old.utf16From && patch.from < old.utf16To);
      if (affected) { binding.valid = false; binding.input.readOnly = true; continue; }
      const from = changes.mapPos(old.utf16From, 1), to = changes.mapPos(old.utf16To, -1);
      const candidates = map.literalRegions.filter(region => region.range.writable && region.range.utf16From === from && region.range.utf16To === to && region.range.raw === old.raw && region.text === binding.region.text);
      binding.valid = candidates.length === 1;
      if (binding.valid) { binding.region = candidates[0]; binding.input.dataset.sourceFrom = String(from); binding.input.dataset.sourceTo = String(to); }
      binding.input.readOnly = !binding.valid;
    }
  }

  /** Factory runs after pending text settles, so it cannot use an old origin. */
  async execute(intent: WritingIntent | (() => WritingIntent), expectedSource?: string): Promise<void> {
    await this.deferUntilCompositionEnds(async () => {
      await this.flush();
      if(this.host.actionAvailable?.()===false)throw new Error("当前对象已切换，未执行旧结构动作。");
      if (this.running) throw new Error("另一项结构修改正在处理中。");
      const basis = this.requireMap();
      const selectedIntent = typeof intent === "function" ? intent() : intent;
      const run = (async () => {
        const result = await this.host.request({ sessionId: basis.sessionId, source: basis.source, serial: basis.serial, sourceHash: basis.map.sourceHash, intent: selectedIntent });
        if (expectedSource !== undefined && result.source !== expectedSource) throw new Error("原生编辑结果与已预览提案不一致；当前草稿保留，请重新预览。");
        this.apply(basis, result, "input.structure", true);
        const input = this.active?.input; this.active = null;
        if (input && typeof document !== "undefined" && document.activeElement === input) { this.focusSelection = true; input.blur(); }
      })();
      this.running = run; this.notify();
      try { await run; } catch (error) { throw this.report(error); } finally { this.running = null; this.notify(); }
    });
  }
  async undo(): Promise<void> { await this.deferUntilCompositionEnds(async () => { await this.flush(); this.active = null; this.host.editor.undo(); }); }
  async redo(): Promise<void> { await this.deferUntilCompositionEnds(async () => { await this.flush(); this.active = null; this.host.editor.redo(); }); }
  /** Rebinding a file keeps accepted and checkpointed later input in one Draft. */
  async settleForSessionChange(): Promise<void> {
    await this.deferUntilCompositionEnds(async () => {
      try { await this.flush(); }
      catch (failure) {
        const active = this.active, recovery = active?.recoveryOnly;
        const proposal = this.recoverySource();
        if (!active || !recovery || proposal == null || !this.unchanged(recovery.basis) || !exactWritingRange(recovery.basis.source, recovery.range)) throw failure;
        const { range, prefix, basis } = recovery;
        const insert = literalRecoveryRaw(range.raw, prefix + active.acceptedText, prefix + active.text);
        const patch = { from: range.utf16From, to: range.utf16To, expected: range.raw, insert };
        if (applySourcePatches(basis.source, [patch]) !== proposal) throw new Error("恢复提案与当前完整草稿不一致，未切换文件会话。");
        // This is an explicitly requested local source recovery, not a renewed
        // visual writer or a persistent save. The host will compile/checkpoint
        // the resulting shared draft using its current file session.
        this.active = null; this.installing = true;
        try { this.host.editor.applyPatches(basis.source, [patch], { from: patch.from + insert.length, to: patch.from + insert.length }, "input.recovery", true); }
        catch (error) { this.active = active; throw error; }
        finally { this.installing = false; }
        this.mapBasis = null; this.failure = null;
      }
      this.active = null;
      for (const binding of this.bindings) { binding.valid = false; binding.input.readOnly = true; binding.input.blur(); }
      this.bindings.clear(); this.notify();
    });
  }

  deferUntilCompositionEnds(run: () => void | Promise<void>): Promise<void> {
    if (this.disposed) return Promise.reject(new Error("编辑会话已关闭。"));
    if (!this.state.composing) return Promise.resolve().then(run);
    return new Promise((resolve, reject) => { this.afterInput.push({ run, resolve, reject }); });
  }
  private async drainDeferred(): Promise<void> {
    if (this.state.composing || this.disposed) return;
    const pending = this.afterInput.splice(0);
    for (const action of pending) { try { await action.run(); action.resolve(); } catch (error) { action.reject(error); } }
  }

  /** Stage native preedit without replacing its DOM or stealing composition. */
  stageLiteral(range: WritingRange, initialText: string, text: string, selection: SourceSelection = { from: text.length, to: text.length }, input?: HTMLTextAreaElement, append = false): void {
    if (!this.active) {
      const basis = this.requireMap();
      const insertion = basis.map.bodyInsertion;
      const matches = append && insertion?.writable && insertion.utf16From === range.utf16From && insertion.raw === range.raw ? [{ range: insertion, text: "", role: "bodyInsertion" }] : basis.map.literalRegions.filter(region => region.range.writable && region.text === initialText && region.range.utf16From === range.utf16From && region.range.utf16To === range.utf16To && region.range.raw === range.raw);
      if (matches.length !== 1 || !exactWritingRange(basis.source, range)) throw new Error("这段文字没有唯一的当前原生写入范围。");
      this.active = { basis, region: matches[0], text: initialText, acceptedText: initialText, selection, generation: 0, composing: false, compositionCommit: false, input, failure: null, append };
    }
    const active = this.active;
    if (input && active.input && active.input !== input) throw new Error("请先完成当前文字区域，再编辑另一处。");
    if (!input && active.region.range.utf16From !== range.utf16From) throw new Error("文字输入目标已变化。");
    active.text = text; active.selection = selection; active.generation++;
    if (!active.recoveryOnly) { active.failure = null; this.failure = null; }
    clearTimeout(this.timer);
    if (!active.recoveryOnly && !active.composing && !this.host.editor.composing) this.timer = setTimeout(() => { void this.flush().catch(error => this.report(error)); }, this.host.debounceMs ?? 180);
    const proposal = this.recoverySource();
    if (proposal != null) this.host.onPendingSource?.(proposal);
    this.notify();
  }
  compositionStart(): void { if (this.active) { this.active.composing = true; clearTimeout(this.timer); this.notify(); } }
  compositionEnd(): void {
    if (this.active) { this.active.composing = false; this.active.compositionCommit = true; }
    this.notify();
    // final input may follow compositionend in the host; capture it first.
    this.timer = setTimeout(() => { void this.flush().then(() => this.drainDeferred()).catch(error => this.report(error)); }, 0);
  }
  recoverySource(): string | null {
    const active = this.active;
    if (active?.recoveryOnly) {
      const { basis, range, prefix } = active.recoveryOnly;
      if (!this.unchanged(basis) || !exactWritingRange(basis.source, range)) return null;
      const encoded = literalRecoveryRaw(range.raw, prefix + active.acceptedText, prefix + active.text, active.region.role === "tableCell");
      return basis.source.slice(0, range.utf16From) + encoded + basis.source.slice(range.utf16To);
    }
    if (!active || !this.unchanged(active.basis) || !exactWritingRange(active.basis.source, active.region.range)) return null;
    if (active.text === active.acceptedText) return active.basis.source;
    const range = active.region.range;
    const source = active.basis.source;
    const prefix = active.append ? !source || source.endsWith("\n\n") ? "" : source.endsWith("\n") ? "\n" : "\n\n" : "";
    const encoded = (active.append ? encodeWritingLiteral(active.text) : literalRecoveryRaw(range.raw, active.acceptedText, active.text, active.region.role === "tableCell")) || (["heading", "bulletListItem", "numberedListItem", "termListItem"].includes(active.region.role) ? "#box[]" : "");
    return source.slice(0, range.utf16From) + prefix + encoded + source.slice(range.utf16To);
  }
  async flush(): Promise<void> {
    clearTimeout(this.timer);
    if (this.state.composing) throw new Error("当前输入法组合尚未完成，文字已保留。");
    if (this.running) { await this.running; if (this.active && this.active.text !== this.active.acceptedText) await this.flush(); return; }
    if (!this.active || this.active.text === this.active.acceptedText) return;
    const active = this.active;
    const run = (async () => {
      while (this.active === active && active.text !== active.acceptedText) {
        if (active.composing) return;
        if (active.recoveryOnly) throw new Error("后续输入已保留为完整恢复草稿；文字结构已变化，请复制文字并在源码中合并。");
        if (!this.unchanged(active.basis)) throw new Error("文字回写时源码已变化，输入保留在当前文字框中。");
        const submitted = active.text, inputGeneration = active.generation, caret = { ...active.selection }, basis = active.basis, personalText = active.personalText && { ...active.personalText };
        const result = await this.host.request({ sessionId: basis.sessionId, source: basis.source, serial: basis.serial, sourceHash: basis.map.sourceHash, intent: active.append ? { kind: "insertBlock", at: active.region.range.from, block: { kind: "paragraph", text: submitted } } : { kind: "replaceLiteral", range: active.region.range, text: submitted } });
        // Match a fresh native slot by its exact replacement interval. Equal
        // text elsewhere (e.g. two table cells) is never a targeting fallback.
        const patches = validateWritingResult(basis.source, result);
        const changes = ChangeSet.of(patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), basis.source.length, "\n");
        const from = changes.mapPos(active.region.range.utf16From, -1), end = changes.mapPos(active.region.range.utf16To, 1);
        const next = result.map.literalRegions.filter(region => region.range.writable && region.text === submitted && (active.append || !submitted ? region.range.utf16From >= from && region.range.utf16To <= end : region.range.utf16From === from && region.range.utf16To === end));
        let selected = result.selection;
        if (next.length === 1) {
          // Selection can move while the native request is running without a
          // text input event. Use that latest selection only when it still
          // addresses the submitted text; newer text will get its own reply.
          const acceptedCaret = active.text === submitted ? active.selection : caret;
          const from = literalSourceOffset(next[0].range.raw, submitted, acceptedCaret.from), to = literalSourceOffset(next[0].range.raw, submitted, acceptedCaret.to);
          if (from != null && to != null) selected = { from: next[0].range.utf16From + from, to: next[0].range.utf16From + to };
        }
        this.apply(basis, result, active.compositionCommit ? "input.compose" : "input.type", active.compositionCommit, selected, personalText);
        if (personalText && active.generation === inputGeneration) this.host.onPersonalInput?.(personalText.id, "");
        active.compositionCommit = false;
        active.acceptedText = submitted;
        if (next.length !== 1) {
          if (active.generation !== inputGeneration && active.text !== submitted) {
            const current = this.currentMap()!;
            const prefix = active.append ? !basis.source || basis.source.endsWith("\n\n") ? "" : basis.source.endsWith("\n") ? "\n" : "\n\n" : "";
            active.recoveryOnly = { basis: current, range: writingRange(result.source, from, end), prefix };
            active.basis = current;
            // onApplied may replace an earlier proposal's persistence timer.
            // Publish the complete latest input against the accepted source
            // after that callback, and keep it available on every later edit.
            const recovery = this.recoverySource();
            if (recovery != null) this.host.onPendingSource?.(recovery);
            throw new Error("中间输入已写入草稿，后续文字已保留为完整恢复草稿；新结构需要重新定位。");
          }
          this.active = null;
          if (active.input) {
            for (const binding of this.bindings) if (binding.input === active.input) binding.valid = false;
            if (typeof document !== "undefined" && document.activeElement === active.input) this.focusSelection = true;
            active.input.readOnly = true; active.input.blur();
          }
          break;
        }
        active.basis = this.currentMap()!; active.region = next[0]; active.append = false;
        for (const binding of this.bindings) if (binding.input === active.input) { binding.region = next[0]; binding.append = false; binding.valid = true; }
        if (active.input) { active.input.dataset.sourceFrom = String(next[0].range.utf16From); active.input.dataset.sourceTo = String(next[0].range.utf16To); }
      }
    })();
    this.running = run; this.notify();
    try { await run; } catch (error) { active.failure = this.report(error); throw active.failure; } finally { this.running = null; this.notify(); }
  }

  /** Native text input: multiline typing, source selection, shared undo, IME. */
  bindLiteral(input: HTMLTextAreaElement, region: WritingLiteral, append = false): () => void {
    input.dataset.personalInputManaged = "literal";
    const binding = { input, region, append, valid: true }; this.bindings.add(binding);
    input.value = region.text; input.spellcheck = false; input.dataset.sourceFrom = String(region.range.utf16From); input.dataset.sourceTo = String(region.range.utf16To);
    const inputId = personalInputId(); let tracker = new DocumentNotesInputTracker(input.value);
    let edit: { from: number; to: number; inputType: string } | undefined;
    let composition: { tracker: DocumentNotesInputTracker; from: number; to: number } | undefined;
    const capture = (event?: Event) => {
      const data = event && "data" in event ? (event as InputEvent | CompositionEvent).data : undefined;
      if (composition) { tracker = composition.tracker.copy(); tracker.observe(input.value, { from: composition.from, to: composition.to, inputType: "insertCompositionText" }, data); }
      else if (event) tracker.observe(input.value, edit, data ?? (event as InputEvent).dataTransfer?.getData("text/plain"));
      else tracker.input(input.value, edit);
      edit = undefined;
      this.host.onPersonalInput?.(inputId, tracker.recoveryInput());
      try { if (!binding.valid) throw new Error("这段文字入口已过期，请等待当前预览后继续。"); this.stageLiteral(binding.region.range, binding.region.text, input.value, { from: input.selectionStart, to: input.selectionEnd }, input, binding.append); if (this.active?.input === input) this.active.personalText = { id: inputId, text: tracker.recoveryInput() }; }
      catch (error) { this.report(error); }
      select();
      input.style.height = "auto"; input.style.height = `${Math.max(input.scrollHeight, 28)}px`;
    };
    const focus = () => {
      if (this.active?.input && this.active.input !== input) {
        const old = this.active.input; void this.flush().then(() => { this.active = null; capture(); }).catch(() => old.focus());
      } else capture();
    };
    const compositionstart = () => { capture(); composition = { tracker: tracker.copy(), from: input.selectionStart, to: input.selectionEnd }; this.compositionStart(); };
    const compositionend = (event: CompositionEvent) => { capture(event); composition = undefined; this.compositionEnd(); };
    const blur = () => { if (this.active?.input === input && !this.active.composing) void this.flush().then(() => { if (this.active?.input === input && this.active.text === this.active.acceptedText && typeof document !== "undefined" && document.activeElement !== input) { this.active = null; this.notify(); } }).catch(() => undefined); };
    const select = () => {
      const active = this.active?.input === input ? this.active : null;
      const current = active?.region ?? binding.region;
      if (active) active.selection = { from: input.selectionStart, to: input.selectionEnd };
      if (!this.currentMap()) return;
      if (active && active.text !== active.acceptedText) return;
      const text = active?.acceptedText ?? current.text;
      const from = literalSourceOffset(current.range.raw, text, input.selectionStart), to = literalSourceOffset(current.range.raw, text, input.selectionEnd);
      if (from == null || to == null) return;
      this.host.editor.select(current.range.utf16From + from, current.range.utf16From + to, false);
    };
    const keydown = (event: KeyboardEvent) => {
      if (event.isComposing || this.state.composing) return;
      if ((event.ctrlKey || event.metaKey) && ["z", "y"].includes(event.key.toLowerCase())) { event.preventDefault(); void (event.key.toLowerCase() === "y" || event.shiftKey ? this.redo() : this.undo()).catch(error => this.report(error)); }
      else if ((event.ctrlKey || event.metaKey) && ["b", "i"].includes(event.key.toLowerCase())) { event.preventDefault(); select(); const format = event.key.toLowerCase() === "b" ? "strong" : "emph"; void this.execute(() => ({ kind: "setInline", range: this.selectionRange(), format })).catch(error => this.report(error)); }
      else if (event.key === "Enter" && !event.shiftKey && !(event.ctrlKey || event.metaKey || event.altKey)) {
        const enclosingList = (range: WritingRange) => this.map?.structures.filter(item => ["bulletListItem", "numberedListItem"].includes(item.kind) && item.body && item.body.utf16From <= range.utf16From && item.body.utf16To >= range.utf16To).sort((a, b) => a.range.utf16To - a.range.utf16From - (b.range.utf16To - b.range.utf16From))[0] ?? null;
        if (enclosingList(binding.region.range)) {
          event.preventDefault(); capture();
          const at = input.selectionStart;
          void this.execute(() => { const structure = enclosingList(this.active?.region.range ?? binding.region.range); if (!structure) throw new Error("当前列表项已变化，请重新选择。"); return { kind: "listEdit", range: structure.range, action: { kind: "split", at } }; }).catch(error => this.report(error));
        } else if (binding.region.role === "body" || binding.append) {
          event.preventDefault(); edit = { from: input.selectionStart, to: input.selectionEnd, inputType: "insertText" }; input.setRangeText("\n\n", input.selectionStart, input.selectionEnd, "end"); capture();
        }
      }
    };
    const beforeinput = (event: InputEvent) => {
      if (event.inputType === "historyUndo" || event.inputType === "historyRedo") { event.preventDefault(); void (event.inputType === "historyUndo" ? this.undo() : this.redo()).catch(error => this.report(error)); }
      else edit = { from: input.selectionStart, to: input.selectionEnd, inputType: event.inputType };
    };
    input.addEventListener("focus", focus); input.addEventListener("input", capture); input.addEventListener("compositionstart", compositionstart); input.addEventListener("compositionend", compositionend); input.addEventListener("blur", blur); input.addEventListener("select", select); input.addEventListener("keyup", select); input.addEventListener("keydown", keydown); input.addEventListener("beforeinput", beforeinput);
    if (this.focusSelection && this.host.editor.selection.from >= region.range.utf16From && this.host.editor.selection.to <= region.range.utf16To) {
      const selected = this.host.editor.selection, focusSnapshot = this.snapshot;
      requestAnimationFrame(() => {
        if (!this.focusSelection || !input.isConnected || !this.currentMap() || !this.isSnapshotCurrent(focusSnapshot)) return;
        this.focusSelection = false;
        const toTextOffset = (sourceAt: number): number | null => literalTextOffset(region.range.raw, region.text, sourceAt - region.range.utf16From);
        const from = toTextOffset(selected.from), to = toTextOffset(selected.to);
        if (from == null || to == null) return;
        input.setSelectionRange(from, to); input.focus();
      });
    }
    return () => { this.bindings.delete(binding); input.removeEventListener("focus", focus); input.removeEventListener("input", capture); input.removeEventListener("compositionstart", compositionstart); input.removeEventListener("compositionend", compositionend); input.removeEventListener("blur", blur); input.removeEventListener("select", select); input.removeEventListener("keyup", select); input.removeEventListener("keydown", keydown); input.removeEventListener("beforeinput", beforeinput); };
  }
  bindBodyInput(input: HTMLTextAreaElement): () => void {
    const range = this.requireMap().map.bodyInsertion;
    if (!range?.writable) throw new Error("当前没有可安全添加内容的正文位置。");
    input.placeholder = this.map?.literalRegions.length ? "继续写作…" : "从这里开始写作…";
    input.setAttribute("aria-label", "新段落文字输入");
    return this.bindLiteral(input, { range, text: "", role: "bodyInsertion" }, true);
  }
  /** Caller additionally requires a writable node in the current native tree. */
  regionForOrigin(origin: { isMain: boolean; utf16From: number; utf16To: number; raw: string } | null | undefined): WritingLiteral | null {
    const basis = this.currentMap();
    if (!basis || !origin?.isMain || basis.source.slice(origin.utf16From, origin.utf16To) !== origin.raw) return null;
    const matches = basis.map.literalRegions.filter(region => region.range.writable && region.range.utf16From <= origin.utf16From && region.range.utf16To >= origin.utf16To && exactWritingRange(basis.source, region.range));
    return matches.length === 1 ? matches[0] : null;
  }
  structure(kind?: string): WritingStructure | null {
    const selection = this.host.editor.selection;
    return this.map?.structures.filter(item => (!kind || item.kind === kind) && item.range.utf16From <= selection.from && item.range.utf16To >= selection.to).sort((a, b) => a.range.utf16To - a.range.utf16From - (b.range.utf16To - b.range.utf16From))[0] ?? null;
  }
  selectionRange(): WritingRange {
    const basis = this.requireMap(); let selection = this.host.editor.selection;
    if (this.active && this.active.basis.source === basis.source) {
      const active = this.active, from = literalSourceOffset(active.region.range.raw, active.acceptedText, active.selection.from), to = literalSourceOffset(active.region.range.raw, active.acceptedText, active.selection.to);
      if (from == null || to == null) throw new Error("此文字选区没有可证明的源位置，请使用源码选区设置格式。");
      selection = { from: active.region.range.utf16From + from, to: active.region.range.utf16From + to };
    }
    if (selection.from === selection.to) throw new Error("请先选择要设置格式的文字。");
    return writingRange(basis.source, selection.from, selection.to);
  }
  insertionPoint(): number {
    const map = this.requireMap().map;
    if (!map.bodyInsertion?.writable) throw new Error(map.unavailableReason || "当前没有可安全添加内容的正文位置。");
    return map.bodyInsertion.from;
  }
  dispose(): void { this.disposed = true; clearTimeout(this.timer); this.stopObserve(); this.bindings.clear(); this.surfaces.clear(); const error = new Error("编辑会话已关闭，未应用待处理动作。"); for (const action of this.afterInput.splice(0)) action.reject(error); }
}
