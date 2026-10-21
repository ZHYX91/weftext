import { DocumentNotesInputTracker } from "./personal-input-text";
export { DocumentNotesInputTracker } from "./personal-input-text";
import { exactWritingRange, type WritingController, type WritingRange, type WritingSnapshot, type WritingStructure } from "./writing";
import type { RenderNode } from "./types";

/** Disposable products of the same native compile. Labels/text never identify a note. */
export interface NotesSourceEntry {
  kind: string; construct: WritingRange; body?: WritingRange | null; label?: string | null;
  target?: string | null; writable: boolean; unavailableReason?: string | null;
}
export interface NotesSourceMap { sourceHash: string; syntaxValid: boolean; entries: NotesSourceEntry[] }
export interface DocumentNotesProjection {
  source: string; sourceHash: string; notes: NotesSourceMap; tree: RenderNode | null; structures?: WritingStructure[];
}
export interface NoteBodyInput { mode: "literal" | "source"; value: string }
export type NotesNumbering = { kind: "none" } | { kind: "pattern"; pattern: string };
export type DocumentNotesAction =
  | { kind: "insertFootnote" | "insertEndnote"; at: WritingRange; body: NoteBodyInput }
  | { kind: "repeatFootnote" | "repeatEndnote"; at: WritingRange; definition: WritingRange }
  | { kind: "setNoteBody"; construct: WritingRange; body: NoteBodyInput }
  | { kind: "removeNote"; construct: WritingRange; allReferences: boolean }
  | { kind: "setCaption"; construct: WritingRange; body: NoteBodyInput; position: "top" | "bottom" | null; numbering: NotesNumbering | null }
  | { kind: "wrapCaption"; object: WritingRange; body: NoteBodyInput; category: "figure" | "table" | "equation"; position: "top" | "bottom"; numbering: NotesNumbering }
  | { kind: "setEquationNumbering"; construct: WritingRange; numbering: NotesNumbering };
export interface DocumentNotesIntent { kind: "nativeNote"; action: DocumentNotesAction }
export interface DocumentNoteReference { key: string; range: WritingRange | null; definition: boolean; writable: boolean }
export interface DocumentNote {
  id: string; kind: "footnote" | "endnote"; number: string; text: string;
  definition: NotesSourceEntry | null; references: DocumentNoteReference[]; bodyRange: WritingRange | null;
  writable: boolean; reason: string | null;
}
export interface DocumentCaption {
  id: string; category: string; prefix: string; text: string; position: "top" | "bottom" | null;
  numbered: boolean | null; entry: NotesSourceEntry | null; objectRange: WritingRange | null;
  writable: boolean; reason: string | null;
}
export interface DocumentCaptionObject { key: string; category: "figure" | "table" | "equation"; range: WritingRange; title: string }
export interface DocumentEquation { id: string; range: WritingRange | null; numbered: boolean | null; prefix: string; writable: boolean }
export interface DocumentNotesSnapshot {
  sourceHash: string; notes: DocumentNote[]; captions: DocumentCaption[]; objects: DocumentCaptionObject[];
  equations: DocumentEquation[]; unlocated: NotesSourceEntry[]; available: boolean; reason: string | null;
}
export interface DocumentNotesHost {
  controller: WritingController;
  projection(): DocumentNotesProjection | null;
  readable?(): boolean;
  editable(): boolean;
  perform(intent: DocumentNotesIntent, basis: WritingSnapshot): Promise<void>;
  /** A true native caret range; the default is the declared body insertion. */
  insertionRange?(): WritingRange;
  revealSource?(range: WritingRange): void;
  revealRendered?(key: string): boolean;
  ensureEndnoteModule?(): Promise<void>;
  recoverEndnoteModule?(): Promise<void>;
  retryEndnoteModule?(): Promise<void>;
  restartEndnoteModule?(): Promise<void>;
  settleEndnoteModule?(): Promise<void>;
  /** Preserve only newly entered text when the old source becomes unreadable. */
  onPendingInput?(value: string): void;
  onError?(error: Error): void;
}
export interface DocumentNotesMount { refresh(): void; open(mode?: "footnote" | "endnote" | "caption", targetId?: string): Promise<void>; dispose(): void }
const openPanels = new WeakMap<WritingController, Set<() => void>>();
/** Called after the host revokes the projection, before disposing its controller. */
export function retireDocumentNotes(controller: WritingController): void { for (const retire of openPanels.get(controller) ?? []) retire(); }

const attr = (node: RenderNode, key: string): string | null => typeof node.attrs[key] === "string" ? node.attrs[key] as string : null;
const role = (node: RenderNode) => attr(node, "data-weftext-role");
function descendants(node: RenderNode): RenderNode[] { return [node, ...node.children.flatMap(descendants)]; }
function text(node: RenderNode): string { return node.kind === "text" ? node.text ?? "" : node.children.map(text).join(""); }
function exactOrigin(source: string, node: RenderNode): WritingRange | null {
  const origin = node.origin;
  if (!origin?.isMain) return null;
  const range = { ...origin, writable: node.writable };
  return exactWritingRange(source, range) ? range : null;
}
function inside(outer: WritingRange, inner: WritingRange): boolean { return outer.from <= inner.from && outer.to >= inner.to; }
function sameRange(a: WritingRange, b: WritingRange): boolean { return a.from === b.from && a.to === b.to && a.raw === b.raw; }
function sourceEntry(source: string, entries: NotesSourceEntry[], node: RenderNode, kinds: string[]): NotesSourceEntry | null {
  const origin = exactOrigin(source, node);
  if (!origin) return null;
  const matches = entries.filter(entry => kinds.includes(entry.kind) && exactWritingRange(source, entry.construct) && inside(entry.construct, origin));
  return matches.length === 1 ? matches[0] : null;
}
function bodyEntry(source: string, entries: NotesSourceEntry[], body: RenderNode, kinds: string[]): NotesSourceEntry | null {
  const origins = descendants(body).map(node => exactOrigin(source, node)).filter((range): range is WritingRange => !!range);
  const matches = entries.filter(entry => kinds.includes(entry.kind) && entry.body && exactWritingRange(source, entry.construct) && exactWritingRange(source, entry.body) && origins.some(origin => inside(entry.body!, origin) || sameRange(entry.body!, origin)));
  return matches.length === 1 ? matches[0] : null;
}
function booleanAttr(node: RenderNode, key: string): boolean | null { const value = node.attrs[key]; return value === true || value === "true" ? true : value === false || value === "false" ? false : null; }

/** Relation joins use typed producer IDs and exact native provenance, never HTML proximity. */
export function buildDocumentNotesSnapshot(projection: DocumentNotesProjection): DocumentNotesSnapshot {
  const { source, sourceHash, notes: map, tree } = projection;
  const unavailable = (reason: string): DocumentNotesSnapshot => ({ sourceHash, notes: [], captions: [], objects: [], equations: [], unlocated: [], available: false, reason });
  if (!map || !sourceHash || map.sourceHash !== sourceHash || !map.syntaxValid || !tree) return unavailable("等待同一草稿的有效原生注释与题注投影。");
  if (map.entries.some(entry => !exactWritingRange(source, entry.construct) || entry.body && (!exactWritingRange(source, entry.body) || !inside(entry.construct, entry.body)))) return unavailable("注释来源范围与当前草稿不一致，原稿保留。");
  const nodes = descendants(tree), used = new Set<NotesSourceEntry>(), notes: DocumentNote[] = [];
  for (const kind of ["footnote", "endnote"] as const) {
    const bodies = nodes.filter(node => role(node) === `${kind}-body`), markers = nodes.filter(node => role(node) === `${kind}-reference`);
    const ids = new Set([...markers, ...bodies].map(node => attr(node, "data-weftext-note-id")).filter((id): id is string => !!id));
    for (const id of ids) {
      const bodyNodes = bodies.filter(node => attr(node, "data-weftext-note-id") === id), markerNodes = markers.filter(node => attr(node, "data-weftext-note-id") === id);
      const definitions = [`${kind}-definition`], markerKinds = [...definitions, `${kind}-reference`, "reference"];
      let definition = bodyNodes.length === 1 ? bodyEntry(source, map.entries, bodyNodes[0], definitions) : null;
      const references = markerNodes.map(node => {
        const entry = sourceEntry(source, map.entries, node, markerKinds);
        if (entry) { used.add(entry); if (definitions.includes(entry.kind)) definition ??= entry; }
        return { key: node.key, range: entry?.construct ?? exactOrigin(source, node), definition: !!entry && definitions.includes(entry.kind), writable: !!entry?.writable };
      });
      // Authenticated bundled endnotes carry their explicit stable ID; only a
      // unique matching source definition may join its otherwise module origin.
      if (kind === "endnote" && !definition && bodyNodes.length === 1) {
        const candidates = map.entries.filter(entry => definitions.includes(entry.kind) && (entry.target === id || entry.label === id));
        if (candidates.length === 1) definition = candidates[0];
      }
      if (definition) used.add(definition);
      const bodyRange = definition?.body ?? null;
      const uniqueReferences = references.every(reference => !reference.range || references.filter(other => other.range && sameRange(reference.range!, other.range)).length === 1);
      const writable = bodyNodes.length === 1 && !!definition?.writable && !!bodyRange?.writable && uniqueReferences;
      notes.push({ id, kind, number: markerNodes[0] ? text(markerNodes[0]) : "", text: bodyNodes.length === 1 ? text(bodyNodes[0]) : "", definition, references, bodyRange, writable,
        reason: writable ? null : bodyNodes.length !== 1 ? "注释正文缺失或重复，不能猜测所属对象。" : definition?.unavailableReason || "此注释没有唯一的当前主源写地址，请编辑准确来源。" });
    }
  }
  const captions: DocumentCaption[] = [];
  for (const node of nodes.filter(node => role(node) === "figure")) {
    const id = attr(node, "data-weftext-target-id"); if (!id) continue;
    const owners = nodes.filter(other => role(other) === "figure" && attr(other, "data-weftext-target-id") === id);
    const bodyNodes = nodes.filter(other => role(other) === "caption-body" && attr(other, "data-weftext-target-id") === id);
    const captionNodes = nodes.filter(other => role(other) === "caption" && attr(other, "data-weftext-target-id") === id);
    const prefixes = nodes.filter(other => role(other) === "caption-prefix" && attr(other, "data-weftext-target-id") === id);
    const entry = sourceEntry(source, map.entries, node, ["figure"]);
    if (entry) used.add(entry);
    const writable = owners.length === 1 && bodyNodes.length <= 1 && captionNodes.length <= 1 && !!entry?.writable;
    const position = attr(captionNodes[0] ?? node, "data-weftext-caption-position");
    captions.push({ id, category: attr(captionNodes[0] ?? node, "data-weftext-caption-kind") ?? "unknown", prefix: prefixes.length === 1 ? text(prefixes[0]) : "", text: bodyNodes.length === 1 ? text(bodyNodes[0]) : "", position: position === "top" || position === "bottom" ? position : null,
      numbered: booleanAttr(node, "data-weftext-numbered"), entry, objectRange: entry?.construct ?? exactOrigin(source, node), writable, reason: writable ? null : entry?.unavailableReason || "对象或题注来源不唯一；保留原生内容，使用源码入口。" });
  }
  const equationEntries = map.entries.filter(entry => entry.kind === "equation");
  const equationRanges = [...equationEntries.map(entry => ({ ...entry.construct, writable: entry.writable })), ...(projection.structures ?? []).filter(structure => structure.kind === "displayMath").map(structure => structure.range)].filter((range, index, ranges) => exactWritingRange(source, range) && ranges.findIndex(other => sameRange(other, range)) === index);
  const candidateObjects = [...(projection.structures ?? []).filter(structure => ["table", "image"].includes(structure.kind)), ...equationRanges.map(range => ({ kind: "displayMath", range }))];
  const objects: DocumentCaptionObject[] = candidateObjects.filter(structure => structure.range.writable && exactWritingRange(source, structure.range) && !captions.some(caption => caption.objectRange && inside(caption.objectRange, structure.range))).map(structure => ({ key: `${structure.kind}:${structure.range.from}:${structure.range.to}`, category: structure.kind === "table" ? "table" : structure.kind === "displayMath" ? "equation" : "figure", range: structure.range, title: `${structure.kind === "table" ? "表格" : structure.kind === "displayMath" ? "公式" : "图片"} · 源码 ${structure.range.utf16From + 1}–${structure.range.utf16To}` }));
  const equations: DocumentEquation[] = nodes.filter(node => role(node) === "equation").map(node => {
    const origin = exactOrigin(source, node), matches = origin ? equationRanges.filter(range => inside(range, origin)) : [], target = attr(node, "data-weftext-target-id");
    const prefixes = target ? nodes.filter(other => role(other) === "equation-prefix" && attr(other, "data-weftext-target-id") === target) : [];
    if (matches.length === 1) for (const entry of equationEntries) if (sameRange(entry.construct, matches[0])) used.add(entry);
    return { id: target ?? node.key, range: matches.length === 1 ? matches[0] : null, numbered: booleanAttr(node, "data-weftext-numbered"), prefix: prefixes.length === 1 ? text(prefixes[0]) : "", writable: matches.length === 1 && matches[0].writable };
  });
  for (const equation of equations) if (equation.range && equations.filter(other => other.range && sameRange(other.range, equation.range!)).length !== 1) equation.writable = false;
  return { sourceHash, notes, captions, objects, equations, unlocated: map.entries.filter(entry => !used.has(entry) && !["reference", "footnote-reference", "endnote-reference"].includes(entry.kind)), available: true, reason: null };
}

export function requireDocumentNotesBasis(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash?: string): DocumentNotesProjection {
  const projection = host.projection();
  if (!projection || !host.controller.isSnapshotCurrent(basis) || projection.source !== basis.source || sourceHash && projection.sourceHash !== sourceHash) throw new Error("注释提案的来源已变化；输入保留，未应用到新草稿。");
  return projection;
}
export async function performDocumentNotes(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash: string, action: DocumentNotesAction): Promise<void> {
  requireDocumentNotesBasis(host, basis, sourceHash);
  if (!host.editable()) throw new Error("当前没有正文写权限；注释输入保留，未改变原稿。");
  if (host.controller.state.composing) throw new Error("请先完成输入法组合，当前注释输入保留。");
  await host.controller.flush(); requireDocumentNotesBasis(host, basis, sourceHash);
  await host.perform({ kind: "nativeNote", action }, basis);
}
export async function copyDocumentNote(host: DocumentNotesHost, basis: WritingSnapshot, sourceHash: string, value: string, write: (value: string) => Promise<void> = value => {
  if (!navigator.clipboard?.writeText) throw new Error("当前环境不能写剪贴板；请选择文字使用系统复制。"); return navigator.clipboard.writeText(value);
}): Promise<void> {
  requireDocumentNotesBasis(host, basis, sourceHash);
  await write(value);
  // Copy never mutates Draft, even when permission is denied or the source
  // changes while the browser clipboard permission request is outstanding.
  requireDocumentNotesBasis(host, basis, sourceHash);
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", value?: string): HTMLElementTagNameMap[K] { const node = document.createElement(tag); node.className = className; if (value != null) node.textContent = value; return node; }
function label(title: string, input: HTMLElement): HTMLElement { const field = element("label", "document-notes-field"); field.append(element("span", "", title), input); return field; }
function select(choices: [string, string][], value: string, title: string): HTMLSelectElement { const node = element("select", "document-notes-input"); node.setAttribute("aria-label", title); for (const [key, text] of choices) { const option = element("option", "", text); option.value = key; node.append(option); } node.value = value; return node; }
function button(title: string, work: () => void | Promise<void>, report: (error: unknown) => void): HTMLButtonElement { const node = element("button", "document-notes-button", title); node.type = "button"; node.onclick = () => { Promise.resolve().then(work).catch(report); }; return node; }
function failure(error: unknown): Error { return error instanceof Error ? error : new Error(String(error)); }
const categoryName = (value: string) => value === "table" ? "表" : value === "equation" ? "公式" : value === "figure" ? "图" : value;
/** Single proven replacement helper, retained for callers that have one edit. */
export function documentNotesNewInput(before: string, after: string): string {
  let from = 0, to = before.length, end = after.length;
  while (from < to && from < end && before[from] === after[from]) from++;
  while (to > from && end > from && before[to - 1] === after[end - 1]) { to--; end--; }
  // Only a pure contiguous insertion is provable without edit history.
  return from === to ? after.slice(from, end) : "";
}
function insertion(host: DocumentNotesHost, source: string): WritingRange {
  const range = host.insertionRange?.() ?? host.controller.map?.bodyInsertion;
  if (!range?.writable || range.from !== range.to || !exactWritingRange(source, range)) throw new Error("当前没有可证明的注释插入位置；请先在正文放置光标或等待最新编辑地图。");
  return range;
}

export function mountDocumentNotes(container: HTMLElement, host: DocumentNotesHost): DocumentNotesMount {
  let disposed = false, repainting = false;
  // Summary buttons are not editors: observing an unfocusable sentinel keeps
  // them fresh without retaining an obsolete projection after a dialog closes.
  const observation = element("span", "document-notes-observer"); observation.hidden = true;
  const report = (error: unknown) => host.onError?.(failure(error));
  const refresh = () => {
    if (disposed || repainting) return; repainting = true;
    try {
      const projection = host.projection(), summary = projection && projection.source === host.controller.snapshot.source ? buildDocumentNotesSnapshot(projection) : null;
      const controls = element("div", "document-notes-tools"); controls.setAttribute("role", "group"); controls.setAttribute("aria-label", "脚注、尾注与题注");
      for (const [mode, title] of [["footnote", "脚注"], ["endnote", "尾注"], ["caption", "题注与编号"]] as const) { const trigger = button(title, () => openDocumentNotes(host, mode), report); trigger.disabled = !summary?.available; controls.append(trigger); }
      container.replaceChildren(controls);
      const state = element("p", "document-notes-status", summary?.available ? `${summary.notes.filter(note => note.kind === "footnote").length} 条脚注 · ${summary.notes.filter(note => note.kind === "endnote").length} 条尾注 · ${summary.captions.length} 个图表对象` : summary?.reason ?? "等待当前获权草稿的注释投影。"); state.setAttribute("aria-live", "polite"); container.append(state, observation);
    } finally { repainting = false; }
  };
  const stop = host.controller.observeSurface(observation, refresh); refresh();
  return { refresh, open: (mode, targetId) => openDocumentNotes(host, mode, targetId), dispose() { disposed = true; stop(); container.replaceChildren(); } };
}

/** All edits are reviewed proposals over one captured Draft, applied by the shared adapter. */
export async function openDocumentNotes(host: DocumentNotesHost, initialMode: "footnote" | "endnote" | "caption" = "footnote", targetId?: string): Promise<void> {
  await host.controller.deferUntilCompositionEnds(async () => { await host.controller.flush(); });
  const basis = host.controller.snapshot, projection = requireDocumentNotesBasis(host, basis), model = buildDocumentNotesSnapshot(projection);
  if (!model.available) throw new Error(model.reason ?? "当前注释投影不可用。");
  const dialog = element("dialog", "document-notes-dialog"), form = element("form", "document-notes-panel"), title = element("h2", "", "脚注、尾注与题注"), status = element("p", "document-notes-status"), error = element("p", "document-notes-error"), list = element("section", "document-notes-list"), editor = element("section", "document-notes-editor");
  dialog.setAttribute("aria-label", "脚注、尾注与题注"); error.setAttribute("role", "alert"); error.hidden = true; status.setAttribute("aria-live", "polite"); form.onsubmit = event => event.preventDefault();
  const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  let mode = initialMode, busy = false, disposed = false, localComposition = false, pendingValue = "", dirty = false, retainedInput = false, tracker = new DocumentNotesInputTracker("");
  const navigation: { afterClose?: () => void } = {};
  const report = (value: unknown) => { const message = failure(value); error.textContent = message.message; error.hidden = false; host.onError?.(message); };
  const current = () => { requireDocumentNotesBasis(host, basis, model.sourceHash); return !disposed; };
  const close = () => { if (localComposition || host.controller.state.composing || busy) { status.textContent = "请先结束当前输入或等待原操作完成；未应用文字仍在面板。"; return; } dialog.close(); };
  const run = async (work: () => Promise<void>) => {
    if (busy || localComposition) return; busy = true; error.hidden = true; updateState();
    try { current(); await work(); if (!disposed) { dirty = false; dialog.close(); } } catch (value) { if (!disposed) report(value); }
    finally { busy = false; if (!disposed) updateState(); }
  };
  const perform = (action: DocumentNotesAction) => {
    if (dirty && !["setNoteBody", "setCaption", "wrapCaption", "insertFootnote", "insertEndnote", "setEquationNumbering"].includes(action.kind)) throw new Error("内容尚未应用；请先应用内容或明确取消提案，再操作标记。当前输入保留。");
    return run(() => performDocumentNotes(host, basis, model.sourceHash, action));
  };
  const toolbar = element("div", "document-notes-tools"); toolbar.setAttribute("role", "tablist");
  const tabs = (["footnote", "endnote", "caption"] as const).map(kind => {
    const tab = button(kind === "footnote" ? "脚注" : kind === "endnote" ? "尾注" : "题注与编号", () => { if (dirty) { status.textContent = "当前输入尚未应用；先应用或明确取消本次提案后切换。"; return; } mode = kind; render(); }, report); tab.setAttribute("role", "tab"); toolbar.append(tab); return tab;
  });
  toolbar.onkeydown = event => {
    if (localComposition || event.isComposing || host.controller.state.composing || busy) return;
    const at = tabs.indexOf(document.activeElement as HTMLButtonElement); if (at < 0) return;
    const next = event.key === "ArrowRight" ? (at + 1) % tabs.length : event.key === "ArrowLeft" ? (at + tabs.length - 1) % tabs.length : event.key === "Home" ? 0 : event.key === "End" ? tabs.length - 1 : -1;
    if (next >= 0) { event.preventDefault(); tabs[next].focus(); if (!dirty) tabs[next].click(); }
  };
  const reveal = (range: WritingRange | null, key?: string) => {
    current(); if (dirty || localComposition || busy) throw new Error("当前输入尚未应用；请先应用或明确取消提案后跳转，输入保留。");
    if (range && !exactWritingRange(basis.source, range)) throw new Error("来源无法准确定位。");
    navigation.afterClose = () => { current(); const rendered = key ? host.revealRendered?.(key) : false; if (!rendered && range && host.editable()) host.revealSource?.(range); };
    dialog.close();
  };
  const editorText = (value: string, textMode: "literal" | "source") => {
    const input = element("textarea", "document-notes-input document-notes-body"); input.rows = 7; input.value = value; input.setAttribute("aria-label", "注释或题注内容"); input.spellcheck = false; pendingValue = value; tracker = new DocumentNotesInputTracker(value); retainedInput = false;
    let edit: { from: number; to: number; inputType: string } | undefined;
    let composition: { tracker: DocumentNotesInputTracker; from: number; to: number } | undefined;
    input.addEventListener("beforeinput", event => { edit = { from: input.selectionStart, to: input.selectionEnd, inputType: (event as InputEvent).inputType }; });
    const track = (event: Event) => { const data = (event as InputEvent | CompositionEvent).data; if (composition) { tracker = composition.tracker.copy(); tracker.observe(input.value, { from: composition.from, to: composition.to, inputType: "insertCompositionText" }, data); } else tracker.observe(input.value, edit, data ?? (event as InputEvent).dataTransfer?.getData("text/plain")); edit = undefined; pendingValue = input.value; dirty = true; };
    input.oninput = track;
    input.addEventListener("compositionstart", () => { composition = { tracker: tracker.copy(), from: input.selectionStart, to: input.selectionEnd }; localComposition = true; updateState(); }); input.addEventListener("compositionend", event => { track(event); composition = undefined; localComposition = false; updateState(); });
    const format = select([["literal", "纯文字（按原生文字编码）"], ["source", "原生 Typst 内容（保留格式与子内容）"]], textMode, "注释内容模式"); format.onchange = () => { dirty = true; };
    editor.append(label("内容模式", format), label("内容", input), element("p", "document-notes-help", "原生模式编辑准确内容槽，可保留段落、强调、列表、链接和已支持公式；编号由原生编译派生。"));
    return { input, body: (): NoteBodyInput => ({ mode: format.value as "literal" | "source", value: pendingValue }) };
  };
  const numberingControls = (preserve: boolean, initial: "none" | "pattern" = "pattern") => {
    const option = select([...(preserve ? [["preserve", "保留当前编号规则"] as [string, string]] : []), ["none", "仅描述题注，不编号"], ["pattern", "按原生方案编号"]], preserve ? "preserve" : initial, "题注编号方式"), pattern = element("input", "document-notes-input"); pattern.value = "1"; pattern.setAttribute("aria-label", "原生编号方案"); pattern.maxLength = 256;
    option.onchange = () => { pattern.disabled = option.value !== "pattern"; dirty = true; }; pattern.oninput = () => { dirty = true; }; pattern.disabled = option.value !== "pattern";
    editor.append(label("编号", option), label("原生编号方案（如 1、(1)、一）", pattern));
    const read = (): NotesNumbering | null => option.value === "preserve" ? null : option.value === "none" ? { kind: "none" } : { kind: "pattern", pattern: pattern.value };
    return Object.assign(read, { selectMode(value: "none" | "pattern") { option.value = value; pattern.disabled = value !== "pattern"; } });
  };
  const showNote = (note: DocumentNote) => {
    dirty = false; editor.replaceChildren(element("h3", "", `${note.kind === "footnote" ? "脚注" : "尾注"} ${note.number || "（原生编号）"}`));
    if (!host.editable()) { editor.append(element("p", "document-notes-reading", note.text)); for (let index = 0; index < note.references.length; index++) { const reference = note.references[index]; editor.append(button(`跳到正文第 ${index + 1} 处标记`, () => reveal(reference.range, reference.key), report)); } updateState(); return; }
    const edit = editorText(note.bodyRange?.raw ?? note.text, "source");
    if (!note.writable) { edit.input.readOnly = true; editor.append(element("p", "document-notes-help", note.reason ?? "当前只读。")); }
    const apply = button("应用此注释内容", () => perform({ kind: "setNoteBody", construct: note.definition!.construct, body: edit.body() }), report); apply.dataset.write = "true"; apply.dataset.capable = String(note.writable); editor.append(apply);
    for (let index = 0; index < note.references.length; index++) {
      const reference = note.references[index], row = element("div", "document-notes-reference");
      row.append(button(`跳到正文第 ${index + 1} 处标记`, () => reveal(reference.range, reference.key), report));
      if (reference.range) { const remove = button(reference.definition && note.references.length > 1 ? "删除此定义标记并保留其它引用正文" : "删除此处标记", () => perform({ kind: "removeNote", construct: reference.range!, allReferences: false }), report); remove.dataset.write = "true"; remove.dataset.capable = String(note.writable && reference.writable); row.append(remove); }
      editor.append(row);
    }
    const repeat = button("在原正文位置重复引用此注释", () => perform({ kind: note.kind === "footnote" ? "repeatFootnote" : "repeatEndnote", at: insertion(host, basis.source), definition: note.definition!.construct }), report); repeat.dataset.write = "true"; repeat.dataset.capable = String(note.writable); editor.append(repeat);
    const confirm = element("label", "document-notes-confirm"), checkbox = element("input"); checkbox.type = "checkbox"; checkbox.setAttribute("aria-label", "确认删除此注释全部标记与正文"); confirm.append(checkbox, document.createTextNode(`同时删除这条注释的正文和全部 ${note.references.length} 处标记；可用一次撤销恢复。`));
    const removeAll = button("删除这条注释及全部引用", () => { if (!checkbox.checked) throw new Error("请先确认全部标记和正文的删除范围，或取消并保留原稿。"); return perform({ kind: "removeNote", construct: note.definition!.construct, allReferences: true }); }, report); removeAll.dataset.write = "true"; removeAll.dataset.capable = String(note.writable); editor.append(confirm, removeAll); updateState();
  };
  const showCaption = (caption: DocumentCaption) => {
    dirty = false; editor.replaceChildren(element("h3", "", `${categoryName(caption.category)}题注 · ${caption.prefix || "无编号"}`), element("p", "document-notes-help", "此题注只属于所选原生对象；相同文字不合并对象。"));
    if (!host.editable()) { editor.append(element("p", "document-notes-reading", caption.text)); updateState(); return; }
    const edit = editorText(caption.entry?.body?.raw ?? caption.text, "source"), position = select([["preserve", "保留当前上下位置"], ["top", "对象上方"], ["bottom", "对象下方"]], "preserve", "题注位置"); editor.append(label("位置", position)); const numbering = numberingControls(true);
    if (!caption.writable) { edit.input.readOnly = true; editor.append(element("p", "document-notes-help", caption.reason ?? "当前只读。")); }
    const apply = button("应用所选对象的题注与编号", () => perform({ kind: "setCaption", construct: caption.entry!.construct, body: edit.body(), position: position.value === "preserve" ? null : position.value as "top" | "bottom", numbering: numbering() }), report); apply.dataset.write = "true"; apply.dataset.capable = String(caption.writable); editor.append(apply, button("跳到所属对象源码", () => reveal(caption.objectRange), report)); updateState();
  };
  const showEquation = (equation: DocumentEquation) => {
    dirty = false; pendingValue = ""; tracker = new DocumentNotesInputTracker(""); editor.replaceChildren(element("h3", "", "公式本体编号"), element("p", "document-notes-help", `当前本体编号：${equation.numbered ? equation.prefix || "按原生规则生成" : "无编号或未证明"}。这里仅改变所选公式本体编号；描述题注单独编辑。`));
    if (!host.editable()) { updateState(); return; }
    const numbering = numberingControls(true), apply = button("应用公式本体编号", () => { const chosen = numbering(); if (!chosen || !equation.range) throw new Error("请明确选择本体编号方式；未改变公式或题注。"); return perform({ kind: "setEquationNumbering", construct: equation.range, numbering: chosen }); }, report);
    apply.dataset.write = "true"; apply.dataset.capable = String(equation.writable); editor.append(apply, button("跳到所选公式源码", () => reveal(equation.range), report)); if (!equation.writable) editor.append(element("p", "document-notes-help", "该原生公式没有唯一主源写地址；保留原式，请从获权来源编辑。")); updateState();
  };
  const showNew = () => {
    dirty = false; editor.replaceChildren(element("h3", "", mode === "caption" ? "为明确对象添加题注" : `插入${mode === "footnote" ? "脚注" : "尾注"}`));
    if (!host.editable()) { editor.replaceChildren(element("p", "document-notes-help", "选择已显示的注释或题注查看文字与正文标记。")); updateState(); return; }
    const edit = editorText("", "literal");
    if (mode === "caption") {
      const object = select(model.objects.map(item => [item.key, item.title]), model.objects[0]?.key ?? "", "题注所属对象"), position = select([["bottom", "对象下方"], ["top", "对象上方"]], "bottom", "题注位置"); editor.append(label("所属对象", object), label("位置", position)); const numbering = numberingControls(false, model.objects[0]?.category === "equation" ? "none" : "pattern");
      object.onchange = () => { dirty = true; numbering.selectMode(model.objects.find(item => item.key === object.value)?.category === "equation" ? "none" : "pattern"); };
      editor.append(element("p", "document-notes-help", "选择真实图片、表格或独立公式。公式本体编号与描述题注分别控制；已有本体编号时，新增编号题注须由原生适配器证明不会双重编号。"));
      const apply = button("为所选对象添加题注", () => { const chosen = model.objects.find(item => item.key === object.value); if (!chosen) throw new Error("请选择有唯一来源的对象；未按附近文字猜测配对。"); return perform({ kind: "wrapCaption", object: chosen.range, category: chosen.category, body: edit.body(), position: position.value as "top" | "bottom", numbering: numbering()! }); }, report); apply.dataset.write = "true"; apply.dataset.capable = String(model.objects.length > 0); editor.append(apply);
    } else {
      editor.append(element("p", "document-notes-help", host.insertionRange ? "在打开面板时正文的准确光标位置插入。" : "在本篇已认证的正文末尾插入；不会按画面位置猜测源范围。"));
      if (mode === "endnote") editor.append(element("p", "document-notes-help", "尾注使用随笔记交付的原生模块，独立于脚注编号；缺模块时先单独安装资源，再重新打开本提案。"));
      if (mode === "endnote" && host.ensureEndnoteModule) editor.append(button("准备本篇尾注模块资源", () => { if (dirty) throw new Error("当前注释输入尚未应用；请保留文字或取消本次提案后准备模块资源。原输入未改变。"); return run(async () => { await host.ensureEndnoteModule!(); }); }, report));
      if (mode === "endnote" && host.recoverEndnoteModule) editor.append(button("核对原尾注模块安装", async () => { if (busy) return; busy = true; updateState(); try { await host.recoverEndnoteModule!(); } catch (value) { report(value); } finally { busy = false; updateState(); } }, report));
      if (mode === "endnote" && host.retryEndnoteModule) editor.append(button("按原编号重试尾注模块安装", () => { if (dirty) throw new Error("当前注释文字尚未应用，输入保留；请先保留文字或取消提案后重试原模块安装。"); return run(async () => { await host.retryEndnoteModule!(); }); }, report));
      if (mode === "endnote" && host.restartEndnoteModule) editor.append(button("已明确取消后重新准备尾注模块", () => { if (dirty) throw new Error("当前注释输入保留，请先应用或取消此提案后重新准备模块。"); return run(async () => { await host.restartEndnoteModule!(); }); }, report));
      if (mode === "endnote" && host.settleEndnoteModule) editor.append(button("结束未安装的原尾注模块操作", () => { if (dirty) throw new Error("当前注释输入保留，请先取消或应用本提案后处理原模块操作。"); return run(async () => { await host.settleEndnoteModule!(); }); }, report));
      const apply = button(`插入${mode === "footnote" ? "脚注" : "尾注"}`, () => perform({ kind: mode === "footnote" ? "insertFootnote" : "insertEndnote", at: insertion(host, basis.source), body: edit.body() }), report); apply.dataset.write = "true"; apply.dataset.capable = "true"; editor.append(apply);
    }
    updateState();
  };
  function updateState() {
    let valid = false; try { valid = current(); } catch { /* Keep input; never refresh its target by text or row. */ }
    const readable = host.readable?.() ?? !!host.projection();
    for (const input of Array.from(form.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("input,textarea,select"))) {
      if (input.dataset.documentNotesRecovery === "true") continue;
      if (!readable) { if (input instanceof HTMLTextAreaElement) input.value = ""; else if (input instanceof HTMLInputElement && input.type !== "checkbox") input.value = ""; input.disabled = true; }
    }
    if (!readable) {
      list.replaceChildren(); editor.replaceChildren();
      if (dirty && !retainedInput) {
        const added = tracker.recoveryInput(); retainedInput = true;
        if (added) { host.onPendingInput?.(added); const recovery = element("textarea", "document-notes-input"); recovery.value = added; recovery.readOnly = true; recovery.dataset.documentNotesRecovery = "true"; recovery.setAttribute("aria-label", "保留的本次新输入"); editor.append(label("本次新输入（原件文字已隐藏，可系统复制）", recovery)); }
      }
      status.textContent = "原来源当前不可读；已隐藏原文，未应用的新输入单独保留供复制或恢复。";
    }
    else if (!valid) status.textContent = "草稿或来源已变化；本提案只读，未应用输入保留。关闭后重新打开当前注释。";
    else status.textContent = busy ? "正在核对并应用到共享草稿…" : `${mode === "caption" ? "题注只改变所选对象" : "标记与正文属于同一个原生注释"}；应用后可用正文撤销恢复。`;
    for (const control of Array.from(form.querySelectorAll<HTMLButtonElement>("button[data-write]"))) control.disabled = busy || localComposition || !valid || !host.editable() || control.dataset.capable === "false";
    for (const control of tabs) control.disabled = busy || localComposition;
  }
  function render() {
    list.replaceChildren(); tabs.forEach((tab, index) => tab.setAttribute("aria-selected", String(["footnote", "endnote", "caption"][index] === mode)));
    if (host.editable()) list.append(button(mode === "caption" ? "添加题注" : `新增${mode === "footnote" ? "脚注" : "尾注"}`, () => { if (dirty) { status.textContent = "当前内容尚未应用，请先应用或取消提案。"; return; } showNew(); }, report));
    if (mode === "caption") {
      for (const caption of model.captions) list.append(button(`${categoryName(caption.category)} · ${caption.prefix || "无编号"} · ${caption.text.slice(0, 70) || "尚无题注"}`, () => { if (dirty) { status.textContent = "当前内容尚未应用，请先应用或取消提案。"; return; } showCaption(caption); }, report));
      for (const equation of model.equations) list.append(button(`公式本体编号 · ${equation.prefix || (equation.numbered ? "原生编号" : "无编号")}`, () => { if (dirty) { status.textContent = "当前内容尚未应用，请先应用或取消提案。"; return; } showEquation(equation); }, report));
    } else {
      for (const note of model.notes.filter(note => note.kind === mode)) {
        const card = element("article", "document-notes-card"); card.append(button(`${note.number || "原生编号"} · ${note.text.slice(0, 70) || "空注释"} · ${note.references.length} 处标记`, () => { if (dirty) { status.textContent = "当前内容尚未应用，请先应用或取消提案。"; return; } showNote(note); }, report));
        card.append(button("复制注释文字", () => copyDocumentNote(host, basis, model.sourceHash, note.text), report)); list.append(card);
      }
    }
    for (const entry of model.unlocated.filter(entry => host.editable() && (mode === "caption" ? entry.kind === "figure" : entry.kind.startsWith(mode)))) {
      const card = element("article", "document-notes-card"); card.append(element("p", "document-notes-help", `合法原生来源未取得唯一显示关系：${entry.unavailableReason || "尚无可写投影"}`), button("查看准确原生来源", () => reveal(entry.construct), report)); list.append(card);
    }
    const wanted = targetId && (mode === "caption" ? model.captions.find(item => item.id === targetId) ?? model.equations.find(item => item.id === targetId) : model.notes.find(item => item.id === targetId && item.kind === mode));
    if (wanted) { targetId = undefined; if ("references" in wanted) showNote(wanted); else if ("category" in wanted) showCaption(wanted); else showEquation(wanted); } else showNew();
  }
  const cancel = button("取消未应用提案", close, report); form.append(title, toolbar, status, error, list, editor, cancel); dialog.append(form); document.body.append(dialog);
  const stop = host.controller.observeSurface(dialog, updateState);
  const retire = () => { updateState(); dialog.close(); }, panels = openPanels.get(host.controller) ?? new Set<() => void>(); panels.add(retire); openPanels.set(host.controller, panels);
  dialog.addEventListener("cancel", event => { event.preventDefault(); close(); });
  try { render(); dialog.showModal(); tabs[["footnote", "endnote", "caption"].indexOf(mode)].focus(); await new Promise<void>(resolve => dialog.addEventListener("close", () => resolve(), { once: true })); }
  finally { panels.delete(retire); stop(); dialog.remove(); if (navigation.afterClose) { try { navigation.afterClose(); } catch (value) { report(value); } } else if (previousFocus?.isConnected && host.projection()) previousFocus.focus({ preventScroll: true }); disposed = true; }
}
