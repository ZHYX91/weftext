import test from "node:test";
import assert from "node:assert/strict";
import { EditorState } from "@codemirror/state";
import { EditorView } from "@codemirror/view";
import { history, isolateHistory, undo, redo, undoDepth } from "@codemirror/commands";
import { applySourcePatches, createSourceState, sourceRevealEffect, sourceRevealExtensions, type SourceEditorEvent, type SourcePatch, type SourceSelection } from "./editor";
import { encodeWritingLiteral, exactWritingRange, groupWritingEmptySlots, groupWritingRenderNodes, literalRecoveryRaw, literalSourceOffset, rectangularTSV, textareaSourceDraft, utf8Offset, validateWritingResult, writingRange, WritingController, type WritingBuffer, type WritingHost, type WritingMap, type WritingRequest, type WritingResult } from "./writing";
import type { RenderNode } from "./types";
import { DocumentSession, canRebindSaveAs } from "./session";
import type { OpenedDocument } from "./types";

function map(source: string, from = 0, to = source.length, text = source.slice(from, to)): WritingMap {
  return { sourceHash: `hash:${source}`, syntaxValid: true, literalRegions: [{ range: writingRange(source, from, to), text, role: "body" }], structures: [], title: { status: "absent" }, subtitle: { status: "absent" }, bodyInsertion: writingRange(source, source.length, source.length) };
}
function result(source: string, from: number, to: number, text: string): WritingResult {
  const insert = encodeWritingLiteral(text), next = source.slice(0, from) + insert + source.slice(to);
  const nextMap = map(next, from, from + insert.length, text);
  return { source: next, sourceHash: nextMap.sourceHash, map: nextMap, patches: [{ from: utf8Offset(source, from), to: utf8Offset(source, to), utf16From: from, utf16To: to, expected: source.slice(from, to), insert }], selection: { from: from + insert.length, to: from + insert.length } };
}
class Buffer implements WritingBuffer {
  state: EditorState;
  generation = 0;
  composing = false;
  listeners = new Set<(event: SourceEditorEvent) => void>();
  constructor(source: string) { this.state = createSourceState(source, [history()]); }
  get source(): string { return this.state.doc.toString(); }
  get selection(): SourceSelection { return { from: this.state.selection.main.from, to: this.state.selection.main.to }; }
  observe(listener: (event: SourceEditorEvent) => void): () => void { this.listeners.add(listener); return () => this.listeners.delete(listener); }
  private emit(kind: SourceEditorEvent["kind"]): void { for (const listener of this.listeners) listener({ kind, source: this.source, generation: this.generation, selection: this.selection, composing: this.composing }); }
  applyPatches(source: string, patches: readonly SourcePatch[], selection?: SourceSelection, event = "input.native", isolated = true): void {
    assert.equal(this.source, source); applySourcePatches(source, patches);
    const transaction = this.state.update({ changes: patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), selection: selection ? { anchor: selection.from, head: selection.to } : undefined, userEvent: event, annotations: isolated ? isolateHistory.of("full") : undefined });
    this.state = transaction.state; if (transaction.docChanged) { this.generation++; this.emit("source"); }
  }
  select(from: number, to: number): void { this.state = this.state.update({ selection: { anchor: from, head: to } }).state; this.emit("selection"); }
  undo(): void { undo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit("source"); } }); }
  redo(): void { redo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit("source"); } }); }
  composition(value: boolean): void { this.composing = value; this.emit("composition"); }
}
function harness(source: string, request?: WritingHost["request"]) {
  const editor = new Buffer(source); let serial = 0; editor.observe(event => { if (event.kind === "source") serial++; });
  const calls: WritingRequest[] = [], recoveries: string[] = [];
  const controller = new WritingController({ editor, basis: () => ({ sessionId: "session", source: editor.source, serial }), editable: () => true, debounceMs: 60000, onPendingSource: draft => recoveries.push(draft), request: async intent => { calls.push(intent); if (request) return request(intent); if (intent.intent.kind !== "replaceLiteral") throw new Error("test transform unavailable"); return result(intent.source, intent.intent.range.utf16From, intent.intent.range.utf16To, intent.intent.text); } });
  controller.installMap(map(source), source, serial);
  return { editor, controller, calls, recoveries, serial: () => serial };
}

/** Controller focus/event fixture; this does not exercise browser or OS IME. */
function textareaFixture() {
  const previousDocument = Object.getOwnPropertyDescriptor(globalThis, "document"), previousFrame = Object.getOwnPropertyDescriptor(globalThis, "requestAnimationFrame");
  const documentState: { activeElement: unknown } = { activeElement: null }, frames: FrameRequestCallback[] = [];
  Object.defineProperty(globalThis, "document", { configurable: true, value: documentState });
  Object.defineProperty(globalThis, "requestAnimationFrame", { configurable: true, value: (callback: FrameRequestCallback) => { frames.push(callback); return frames.length; } });
  class Textarea extends EventTarget {
    value = ""; selectionStart = 0; selectionEnd = 0; readOnly = false; spellcheck = false; isConnected = true;
    dataset: Record<string, string> = {}; style = { height: "" }; scrollHeight = 28;
    focus(): void { documentState.activeElement = this; this.dispatchEvent(new Event("focus")); }
    blur(): void { if (documentState.activeElement === this) { documentState.activeElement = null; this.dispatchEvent(new Event("blur")); } }
    setSelectionRange(from: number, to: number): void { this.selectionStart = from; this.selectionEnd = to; }
    type(text: string): void { this.value = text; this.setSelectionRange(text.length, text.length); this.dispatchEvent(new Event("input")); }
  }
  return {
    input: () => new Textarea() as unknown as HTMLTextAreaElement & { type(text: string): void }, documentState,
    frames: () => { for (const callback of frames.splice(0)) callback(0); },
    restore: () => {
      if (previousDocument) Object.defineProperty(globalThis, "document", previousDocument); else Reflect.deleteProperty(globalThis, "document");
      if (previousFrame) Object.defineProperty(globalThis, "requestAnimationFrame", previousFrame); else Reflect.deleteProperty(globalThis, "requestAnimationFrame");
    },
  };
}

test("native patches preserve BOM, mixed EOL, astral text and unaffected bytes", () => {
  const source = "\uFEFF= 标题\r\n甲😀\r\n乙\n"; const from = source.indexOf("甲"), output = result(source, from, from + 3, "改😀#");
  const patches = validateWritingResult(source, output);
  assert.equal(applySourcePatches(source, patches), "\uFEFF= 标题\r\n改😀\\#\r\n乙\n");
  assert.equal(exactWritingRange(source, writingRange(source, from, from + 3)), true);
});

test("native byte/UTF16 mismatch, overlapping patches, surrogate and CRLF splitting reject atomically", () => {
  const source = "甲😀\r\n乙"; const output = result(source, 0, 1, "变"); output.patches[0].from = 1;
  assert.throws(() => validateWritingResult(source, output), /UTF-16/);
  assert.throws(() => applySourcePatches(source, [{ from: 1, to: 2, expected: source.slice(1, 2), insert: "" }]), /字符或换行/);
  assert.throws(() => applySourcePatches(source, [{ from: 4, to: 5, expected: "\n", insert: "" }]), /字符或换行/);
  assert.throws(() => applySourcePatches("abcd", [{ from: 0, to: 3, expected: "abc", insert: "x" }, { from: 2, to: 4, expected: "cd", insert: "y" }]), /重叠/);
});

test("local source reveal maps edits inside its slot and blocks boundary deletion without removing the full buffer", () => {
  const source = "before\r\n*中文*\r\nafter";
  let state = createSourceState(source, [sourceRevealExtensions]); const from = source.indexOf("*"); const to = from + 4;
  state = state.update({ effects: sourceRevealEffect.of({ from, to, raw: "*中文*" }) }).state;
  state = state.update({ changes: { from: from + 1, to: from + 3, insert: "新文字" }, userEvent: "input.type" }).state;
  assert.equal(state.doc.toString(), "before\r\n*新文字*\r\nafter");
  const rejected = state.update({ changes: { from: from - 1, to: from, insert: "" }, userEvent: "delete.backward" }).state;
  assert.equal(rejected.doc.toString(), state.doc.toString());
  state = state.update({ effects: sourceRevealEffect.of(null) }).state;
  state = state.update({ changes: { from: 0, to: 6, insert: "start" } }).state;
  assert.equal(state.doc.toString(), "start\r\n*新文字*\r\nafter");
});

test("auto-flushed escaped text can continue from a fresh native slot using the same CodeMirror history", async () => {
  const { editor, controller, calls, recoveries } = harness("原文");
  controller.stageLiteral(writingRange("原文", 0, 2), "原文", "新#文"); await controller.flush();
  assert.equal(editor.source, "新\\#文"); assert.equal(recoveries[0], "新\\#文"); assert.equal(controller.map?.literalRegions[0].text, "新#文");
  controller.stageLiteral(controller.activeRange!, "新#文", "新#文字"); await controller.flush();
  assert.equal(editor.source, "新\\#文字"); assert.equal(calls[1].source, "新\\#文"); assert.notEqual(calls[0].serial, calls[1].serial);
  await controller.undo(); assert.equal(editor.source, "原文"); await controller.redo(); assert.equal(editor.source, "新\\#文字"); controller.dispose();
});

test("empty then retyped body and view-only selection do not create a separate undo authority", async () => {
  const { editor, controller } = harness("内容"); controller.stageLiteral(writingRange("内容", 0, 2), "内容", ""); await controller.flush();
  assert.equal(editor.source, ""); controller.stageLiteral(controller.activeRange!, "", "重新写"); await controller.flush(); assert.equal(editor.source, "重新写");
  const depth = undoDepth(editor.state); editor.select(1, 2); assert.equal(undoDepth(editor.state), depth); await controller.undo(); assert.equal(editor.source, "内容"); controller.dispose();
});

test("focused visual Undo and Redo retire old inputs so the current preview can rebuild and typing can continue", async () => {
  const fixture = textareaFixture(), { editor, controller, serial } = harness("原文");
  try {
    const original = fixture.input(); controller.bindLiteral(original, controller.map!.literalRegions[0]); original.focus(); original.type("原文新字"); await controller.flush();
    assert.equal(controller.retainInput, true); assert.equal(editor.source, "原文新字");
    await controller.undo();
    assert.equal(editor.source, "原文"); assert.equal(controller.retainInput, false); assert.equal(original.readOnly, true);
    controller.installMap(map(editor.source), editor.source, serial());
    const restored = fixture.input(); controller.bindLiteral(restored, controller.map!.literalRegions[0]); fixture.frames();
    assert.equal(fixture.documentState.activeElement, restored); assert.equal(restored.value, "原文");
    await controller.redo();
    assert.equal(editor.source, "原文新字"); assert.equal(controller.retainInput, false); assert.equal(restored.readOnly, true);
    controller.installMap(map(editor.source), editor.source, serial());
    const continued = fixture.input(); controller.bindLiteral(continued, controller.map!.literalRegions[0]); fixture.frames(); continued.type("原文新字继续"); await controller.flush();
    assert.equal(editor.source, "原文新字继续"); assert.equal(controller.state.error, null);
  } finally { controller.dispose(); fixture.restore(); }
});

test("late native output cannot replace a newer draft, even after equal-byte ABA", async () => {
  let complete!: (result: WritingResult) => void;
  const { editor, controller } = harness("原文", () => new Promise(resolve => { complete = resolve; }));
  controller.stageLiteral(writingRange("原文", 0, 2), "原文", "待回写"); const pending = controller.flush();
  editor.applyPatches("原文", [{ from: 0, to: 2, expected: "原文", insert: "新稿" }]); editor.applyPatches("新稿", [{ from: 0, to: 2, expected: "新稿", insert: "原文" }]);
  complete(result("原文", 0, 2, "待回写")); await assert.rejects(pending, /旧提案未应用/); assert.equal(editor.source, "原文"); assert.equal(controller.pendingText, "待回写"); assert.equal(controller.recoverySource(), null); controller.dispose();
});

test("in-flight input replays only on the exact fresh returned slot and preserves later caret", async () => {
  let complete!: (value: WritingResult) => void; let first = true;
  const { editor, controller, calls } = harness("甲", request => { if (first) { first = false; return new Promise(resolve => { complete = resolve; }); } const intent = request.intent; if (intent.kind !== "replaceLiteral") throw new Error("wrong intent"); return Promise.resolve(result(request.source, intent.range.utf16From, intent.range.utf16To, intent.text)); });
  const range = writingRange("甲", 0, 1); controller.stageLiteral(range, "甲", "甲乙", { from: 1, to: 1 }); const pending = controller.flush(); controller.stageLiteral(range, "甲", "甲丙乙", { from: 2, to: 2 });
  complete(result("甲", 0, 1, "甲乙")); await pending;
  assert.equal(editor.source, "甲丙乙"); assert.equal(calls.length, 2); assert.deepEqual(editor.selection, { from: 2, to: 2 }); controller.dispose();
});

test("a selection moved during a slow visual write transfers the latest exact escaped source range when leaving the view", async () => {
  let complete!: (value: WritingResult) => void;
  const fixture = textareaFixture(), { editor, controller } = harness("原文", () => new Promise(resolve => { complete = resolve; }));
  try {
    const input = fixture.input(); controller.bindLiteral(input, controller.map!.literalRegions[0]); input.focus(); input.type("新#😀字");
    const pending = controller.flush(); input.setSelectionRange(1, 4); input.dispatchEvent(new Event("select"));
    complete(result("原文", 0, 2, "新#😀字")); await pending; await controller.settleForSessionChange();
    assert.equal(editor.source, "新\\#😀字"); assert.deepEqual(editor.selection, { from: 1, to: 5 });
  } finally { controller.dispose(); fixture.restore(); }
});

test("synthetic composition state defers commands and emits one shared undo unit; real OS IME remains a separate check", async () => {
  const { editor, controller, calls } = harness("甲"); const range = writingRange("甲", 0, 1);
  controller.stageLiteral(range, "甲", "甲"); controller.compositionStart(); controller.stageLiteral(range, "甲", "甲你好");
  let switched = false; const switching = controller.deferUntilCompositionEnds(() => { switched = true; });
  await assert.rejects(controller.flush(), /输入法组合/); assert.equal(switched, false); assert.equal(calls.length, 0);
  controller.compositionEnd(); await new Promise(resolve => setTimeout(resolve, 10)); await switching;
  assert.equal(editor.source, "甲你好"); assert.equal(switched, true); assert.equal(undoDepth(editor.state), 1); await controller.undo(); assert.equal(editor.source, "甲"); controller.dispose();
});

test("structural output is one shared undo unit and stale dialog snapshots are distinguishable", async () => {
  const { editor, controller } = harness("正文", request => Promise.resolve(result(request.source, 0, request.source.length, "标题\n正文")));
  const snapshot = controller.snapshot; await controller.execute({ kind: "setTitle", text: "标题" }); assert.equal(undoDepth(editor.state), 1); assert.equal(controller.isSnapshotCurrent(snapshot), false);
  await controller.undo(); assert.equal(editor.source, "正文"); assert.equal(controller.isSnapshotCurrent(snapshot), false); controller.dispose();
});

test("render grouping uses exact native provenance and rejects duplicate occurrences or equal text elsewhere", () => {
  const source = "甲\\#乙"; const sourceMap = map(source, 0, source.length, "甲#乙");
  const leaf = (key: string, from: number, to: number, text: string): RenderNode => ({ key, kind: "text", tag: null, text, children: [], attrs: {}, writable: true, origin: { path: "a.typ", ...writingRange(source, from, to), isMain: true, syntaxKind: "Text" } });
  const tree = (children: RenderNode[]): RenderNode => ({ key: "root", kind: "element", tag: "p", text: null, children, attrs: {}, origin: null, writable: false });
  const nodes = [leaf("a", 0, 1, "甲"), leaf("b", 1, 3, "#"), leaf("c", 3, 4, "乙")];
  assert.deepEqual(groupWritingRenderNodes(tree(nodes), sourceMap, source)[0].memberKeys, ["a", "b", "c"]);
  assert.equal(groupWritingRenderNodes(tree([...nodes, ...nodes]), sourceMap, source).length, 0);
  assert.equal(groupWritingRenderNodes(tree([leaf("wrong", 0, 4, "别的")]), sourceMap, source).length, 0);
});

test("rectangular TSV validation preserves repeated and empty cell values and refuses partial ragged paste", () => {
  assert.deepEqual(rectangularTSV("相同\t相同\r\n\t末格\r\n"), [["相同", "相同"], ["", "末格"]]);
  assert.throws(() => rectangularTSV("a\tb\nc"), /列数一致/);
});

test("native literal caret mapping supports mixed authored escapes, punctuation and Unicode escape without splitting astral text", () => {
  assert.equal(encodeWritingLiteral("1. #文字"), "1\\.\\u{20}\\#文字");
  assert.equal(encodeWritingLiteral(" \t\x01"), "\\u{20}\\u{9}\\u{1}");
  assert.equal(literalSourceOffset("a\\#😀.b", "a#😀.b", 4), 5);
  assert.equal(literalSourceOffset("\\u{1F600}.", "😀.", 2), 9);
  assert.equal(literalSourceOffset("\\u{1F600}.", "😀.", 1), null);
  assert.equal(literalSourceOffset("...", "…", 1), null);
  assert.equal(literalSourceOffset("甲\r\n乙", "甲 乙", 2), 3);
  assert.equal(literalRecoveryRaw("甲\r\n\r\n乙.", "甲\n\n乙.", "甲\n\n乙.新#字"), "甲\r\n\r\n乙.新\\#字");
});

function delayedHeadingRetirement() {
  const source = "= Title", editor = new Buffer(source); let serial = 0; editor.observe(event => { if (event.kind === "source") serial++; });
  let complete!: (value: WritingResult) => void, requests = 0; const saved: string[] = [];
  const controller = new WritingController({ editor, basis: () => ({ sessionId: "heading", source: editor.source, serial }), editable: () => true, debounceMs: 60000,
    request: () => { requests++; return new Promise(resolve => { complete = resolve; }); },
    onPendingSource: proposal => saved.push(proposal), onApplied: intermediate => { saved.length = 0; saved.push(intermediate.source); },
  });
  const initial = map(source, 2, source.length, "Title"); initial.literalRegions[0].role = "heading"; controller.installMap(initial, source, serial);
  const range = initial.literalRegions[0].range; controller.stageLiteral(range, "Title", "Title\n"); const pending = controller.flush(); controller.stageLiteral(range, "Title", "Title\nmore");
  const accepted = "= Title\n", next = map(accepted, 2, 7, "Title"); next.literalRegions[0].role = "heading";
  complete({ source: accepted, sourceHash: next.sourceHash, map: next, patches: [{ from: 2, to: 7, utf16From: 2, utf16To: 7, expected: "Title", insert: "Title\n" }], selection: { from: 8, to: 8 } });
  return { editor, controller, pending, saved, range, requests: () => requests };
}

test("later heading input remains a complete current-source recovery proposal after a delayed reply retires its native writer", async () => {
  const { editor, controller, pending, saved, range, requests } = delayedHeadingRetirement();
  await assert.rejects(pending, /完整恢复草稿/);
  assert.equal(editor.source, "= Title\n"); assert.equal(controller.pendingText, "Title\nmore"); assert.equal(controller.recoverySource(), "= Title\nmore"); assert.equal(saved.at(-1), "= Title\nmore");
  controller.stageLiteral(range, "Title", "Title\nmore 后续"); assert.equal(controller.recoverySource(), "= Title\nmore\\u{20}后续");
  await assert.rejects(controller.flush(), /完整恢复草稿/); assert.equal(requests(), 1); controller.dispose();
});

test("session settlement brings a qualified recovery-only proposal into shared CodeMirror with its own undo unit", async () => {
  const { editor, controller, pending } = delayedHeadingRetirement(); await assert.rejects(pending, /完整恢复草稿/);
  await controller.settleForSessionChange(); assert.equal(editor.source, "= Title\nmore"); assert.equal(controller.pendingText, null); assert.equal(controller.map, null);
  editor.undo(); assert.equal(editor.source, "= Title\n"); editor.undo(); assert.equal(editor.source, "= Title"); controller.dispose();
});

test("session settlement refuses a recovery-only proposal after equal-byte ABA and keeps later text", async () => {
  const { editor, controller, pending } = delayedHeadingRetirement(); await assert.rejects(pending, /完整恢复草稿/);
  editor.applyPatches(editor.source, [{ from: 0, to: editor.source.length, expected: editor.source, insert: "external" }]);
  editor.applyPatches(editor.source, [{ from: 0, to: editor.source.length, expected: editor.source, insert: "= Title\n" }]);
  await assert.rejects(controller.settleForSessionChange()); assert.equal(editor.source, "= Title\n"); assert.equal(controller.pendingText, "Title\nmore"); assert.equal(controller.recoverySource(), null); controller.dispose();
});

test("undo outside a local source reveal retires its decorations and preserves the full source history", () => {
  let state = createSourceState("before\nslot\nafter", [history(), sourceRevealExtensions]);
  state = state.update({ changes: { from: 0, to: 6, insert: "edited" }, userEvent: "input.type" }).state;
  state = state.update({ effects: sourceRevealEffect.of({ from: 7, to: 11, raw: "slot" }) }).state;
  assert.equal((state.facet(EditorView.decorations)[0] as { size: number }).size, 2);
  undo({ state, dispatch: transaction => { state = transaction.state; } });
  assert.equal(state.doc.toString(), "before\nslot\nafter"); assert.equal((state.facet(EditorView.decorations)[0] as { size: number }).size, 0);
});

test("raw code textarea edits preserve untouched mixed EOL, tabs and no-edit original bytes", () => {
  const original = "\t中文😀\r\nprint(1)\n末行\r";
  assert.equal(textareaSourceDraft(original, "\t中文😀\nprint(1)\n末行\n"), original);
  assert.equal(textareaSourceDraft(original, "\t中文😀\nprint(2)\n末行\n"), "\t中文😀\r\nprint(2)\n末行\r");
});

test("paragraph grouping requires both an official Parbreak gap and actual native paragraph boundaries", () => {
  const source = "甲\n\n乙", nativeMap = map(source); nativeMap.literalRegions[0].paragraphBreaks = [writingRange(source, 1, 3)];
  const leaf = (key: string, from: number, to: number, text: string): RenderNode => ({ key, kind: "text", tag: null, text, children: [], attrs: {}, writable: true, origin: { ...writingRange(source, from, to), path: "a.typ", isMain: true, syntaxKind: "Text" } });
  const paragraph = (key: string, children: RenderNode[]): RenderNode => ({ key, kind: "element", tag: "p", text: null, children, attrs: {}, origin: null, writable: false });
  const root: RenderNode = { ...paragraph("root", []), tag: "div", children: [paragraph("p1", [leaf("a", 0, 1, "甲")]), paragraph("p2", [leaf("b", 3, 4, "乙")])] };
  assert.equal(groupWritingRenderNodes(root, nativeMap, source).length, 1);
  assert.equal(groupWritingRenderNodes(paragraph("same", [leaf("a", 0, 1, "甲"), leaf("b", 3, 4, "乙")]), nativeMap, source).length, 0);
  delete nativeMap.literalRegions[0].paragraphBreaks; assert.equal(groupWritingRenderNodes(root, nativeMap, source).length, 0);
});

test("merely focusing a minimally escaped native literal preserves exact recovery bytes", () => {
  const source = "1. yes \\#", text = "1. yes #"; const { controller } = harness(source);
  controller.installMap(map(source, 0, source.length, text), source, 0); controller.stageLiteral(writingRange(source, 0, source.length), text, text);
  assert.equal(controller.recoverySource(), source); controller.dispose();
});

test("failed native qualification retains unapplied text and its explicit exact-source recovery proposal", async () => {
  const { controller, editor } = harness("甲", async () => { throw new Error("unsupported_edit"); });
  controller.stageLiteral(writingRange("甲", 0, 1), "甲", "保留#文字"); await assert.rejects(controller.flush(), /unsupported_edit/);
  assert.equal(editor.source, "甲"); assert.equal(controller.pendingText, "保留#文字"); assert.equal(controller.recoverySource(), "保留\\#文字"); controller.dispose();
});

test("two equal native slots target the chosen second range rather than the first matching string", async () => {
  const source = "相同|相同", { controller, editor, calls } = harness(source);
  const nativeMap = map(source); nativeMap.literalRegions = [{ range: writingRange(source, 0, 2), text: "相同", role: "tableCell" }, { range: writingRange(source, 3, 5), text: "相同", role: "tableCell" }]; controller.installMap(nativeMap, source, 0);
  controller.stageLiteral(nativeMap.literalRegions[1].range, "相同", "第二格"); await controller.flush();
  assert.equal(editor.source, "相同|第二格"); const intent = calls[0].intent; assert.equal(intent.kind, "replaceLiteral"); if (intent.kind === "replaceLiteral") assert.equal(intent.range.utf16From, 3); controller.dispose();
});

test("empty slot grouping requires a unique actual native owner and never chooses an empty table cell by order", () => {
  const source = "= #box[]", nativeMap = map(source, 7, 7, ""); nativeMap.literalRegions[0].role = "heading";
  nativeMap.structures = [{ kind: "heading", range: writingRange(source, 0, 8), body: writingRange(source, 2, 8), level: 1, runIn: false }];
  const heading: RenderNode = { key: "heading", kind: "element", tag: "h1", text: null, children: [], attrs: {}, writable: false, origin: { ...writingRange(source, 0, 8), path: "a.typ", isMain: true, syntaxKind: "Heading" } };
  assert.deepEqual(groupWritingEmptySlots(heading, nativeMap, source).map(slot => slot.nodeKey), ["heading"]);
  const duplicated: RenderNode = { key: "root", kind: "element", tag: "div", text: null, children: [heading, { ...heading, key: "heading2" }], attrs: {}, origin: null, writable: false };
  assert.equal(groupWritingEmptySlots(duplicated, nativeMap, source).length, 0);
  const tableSource = "#table(columns: 2, [], [])", first = tableSource.indexOf("[]") + 1, second = tableSource.lastIndexOf("[]") + 1;
  const tableMap = map(tableSource, first, first, ""); tableMap.literalRegions.push({ range: writingRange(tableSource, second, second), text: "", role: "tableCell" });
  tableMap.structures = [{ kind: "table", range: writingRange(tableSource, 0, tableSource.length), runIn: false, table: { columns: 2, rows: 1, cells: [tableMap.literalRegions[0].range, tableMap.literalRegions[1].range], gridWritable: true } }];
  const table: RenderNode = { ...heading, key: "table", tag: "table", origin: { ...writingRange(tableSource, 0, tableSource.length), path: "a.typ", isMain: true, syntaxKind: "FuncCall" }, children: [{ ...heading, key: "cell1", tag: "td" }, { ...heading, key: "cell2", tag: "td" }] };
  assert.equal(groupWritingEmptySlots(table, tableMap, tableSource).length, 0);
});

test("continuous input accepts minimal native patches while preserving untouched authored CRLF and punctuation", async () => {
  const source = "甲\r\n乙.", text = "甲 乙.", { controller, editor, recoveries } = harness(source, request => {
    const next = request.source + "新", nextMap = map(next, 0, next.length, "甲 乙.新");
    return Promise.resolve({ source: next, sourceHash: nextMap.sourceHash, map: nextMap, patches: [{ from: utf8Offset(source, source.length), to: utf8Offset(source, source.length), utf16From: source.length, utf16To: source.length, expected: "", insert: "新" }], selection: { from: next.length, to: next.length } });
  });
  controller.installMap(map(source, 0, source.length, text), source, 0); controller.stageLiteral(writingRange(source, 0, source.length), text, text + "新", { from: 3, to: 3 }); await controller.flush();
  assert.equal(editor.source, "甲\r\n乙.新"); assert.equal(recoveries[0], "甲\r\n乙.新"); assert.equal(controller.activeRange?.raw, "甲\r\n乙.新"); assert.deepEqual(editor.selection, { from: 4, to: 4 }); controller.dispose();
});

test("controller settles input typed during delayed reopening before the real file session rebind, preserving draft and Undo", async () => {
  const original: OpenedDocument = { sessionId: "old-session", path: "original.typ", root: ".", source: "原稿", baseId: "old-base", kind: "ordinary" };
  const session = new DocumentSession(original), editor = new Buffer(session.source); editor.observe(event => { if (event.kind === "source") session.edit(event.source); });
  let completeEdit!: (value: WritingResult) => void, completeOpen!: (value: OpenedDocument) => void;
  const calls: WritingRequest[] = [], controller = new WritingController({ editor, basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }), editable: () => true, debounceMs: 60000,
    request: request => { calls.push(request); return new Promise(resolve => { completeEdit = resolve; }); },
  });
  controller.installMap(map(session.source), session.source, session.serial);
  const reopening = new Promise<OpenedDocument>(resolve => { completeOpen = resolve; });
  const rebind = (async () => {
    const reopened = await reopening; assert.equal(canRebindSaveAs(original.source, reopened), true);
    await controller.settleForSessionChange();
    session.opened = reopened; session.acknowledgeSave(original.source, reopened.baseId, reopened.path);
  })();
  controller.stageLiteral(writingRange(original.source, 0, 2), "原稿", "原稿，继续#输入"); const editing = controller.flush();
  completeOpen({ ...original, sessionId: "new-session", path: "copy.typ", baseId: "new-base" });
  await Promise.resolve(); assert.equal(session.opened.sessionId, "old-session");
  completeEdit(result(original.source, 0, 2, "原稿，继续#输入")); await editing; await rebind;
  assert.equal(calls[0].sessionId, "old-session"); assert.equal(session.opened.sessionId, "new-session"); assert.equal(session.path, "copy.typ");
  assert.equal(session.source, "原稿，继续\\#输入"); assert.equal(editor.source, session.source); assert.equal(session.dirty, true); assert.equal(controller.pendingText, null);
  editor.undo(); assert.equal(session.source, "原稿"); assert.equal(session.opened.sessionId, "new-session"); assert.equal(session.dirty, false);
  editor.redo(); assert.equal(session.source, "原稿，继续\\#输入"); assert.equal(session.dirty, true); controller.dispose();
});

test("a reviewed-merge snapshot becomes stale during delayed resolution even when Undo restores equal source bytes", async () => {
  const original: OpenedDocument = { sessionId: "merge-session", path: "original.typ", root: ".", source: "原稿\r\n", baseId: "base", kind: "ordinary" };
  const session = new DocumentSession(original), editor = new Buffer(session.source); editor.observe(event => { if (event.kind === "source") session.edit(event.source); });
  const controller = new WritingController({ editor, basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }), editable: () => true, request: async () => { throw new Error("no native transform required"); } });
  const confirmed = controller.snapshot; let complete!: () => void;
  const resolving = new Promise<void>(resolve => { complete = resolve; });
  editor.applyPatches(editor.source, [{ from: 0, to: 2, expected: "原稿", insert: "后续输入" }]); editor.undo();
  complete(); await resolving; await controller.settleForSessionChange();
  assert.equal(session.source, confirmed.source); assert.equal(controller.isSnapshotCurrent(confirmed), false);
  assert.equal(session.opened.sessionId, original.sessionId); editor.redo(); assert.equal(session.source, "后续输入\r\n"); controller.dispose();
});
