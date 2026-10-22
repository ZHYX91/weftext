import { message } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { EditorState } from "@codemirror/state";
import { history, isolateHistory, undo, redo } from "@codemirror/commands";
import { createSourceState, type SourceEditorEvent, type SourcePatch, type SourceSelection } from "./editor";
import { writingRange, WritingController, type WritingMap, type WritingResult, type WritingBuffer } from "./writing";
import type { RenderNode } from "./types";
import { documentNotesHost, numberingHost, numberingPreviewHeadings, personalNumberingKeys, personalNumberingVisible, type AuthoringPanelsEnvironment, type AuthoringProjection } from "./authoring-panels-adapter";

function native(key: string, role: string, content: string, source: string, from: number, to: number, children?: RenderNode[]): RenderNode {
  return { key, kind: "element", tag: "span", text: null, attrs: { "data-weftext-role": role }, writable: true, origin: { ...writingRange(source, from, to), path: "main.typ", syntaxKind: "Heading", isMain: true }, children: children ?? [{ key: key + ":text", kind: "text", tag: null, text: content, attrs: {}, writable: true, origin: null, children: [] }] };
}
function tree(source: string, values: string[]): RenderNode {
  let at = 0; return native("root", "", "", source, 0, source.length, source.split("\n").flatMap((line, index) => {
    const from = at; at += line.length + 1; if (!line.startsWith("=")) return [];
    const title = line.replace(/^=+\s*/, ""); return [native("heading-" + index, "heading", "", source, from, from + line.length, [native("prefix-" + index, "heading-prefix", values.shift() ?? "", source, from, from + line.length), native("title-" + index, "", title, source, from, from + line.length)])];
  }));
}
function map(source: string): WritingMap { let at = 0; return { sourceHash: "hash:" + source, syntaxValid: true, bodyBlocks: [], unavailableReason: null, notes: { sourceHash: "hash:" + source, syntaxValid: true, entries: [] }, literalRegions: [], structures: source.split("\n").flatMap(line => { const from = at; at += line.length + 1; return line.startsWith("=") ? [{ kind: "heading", range: writingRange(source, from, from + line.length), body: writingRange(source, from + 2, from + line.length), level: 1, levelWritable: true, levelReason: null, label: null, language: null, table: null, runIn: false }] : []; }), title: { status: "absent", text: null, range: null, construct: null, encoding: null }, subtitle: { status: "absent", text: null, range: null, construct: null, encoding: null }, headingNumbering: { sourceHash: "hash:" + source, editable: true, frozenEndnotesRequired: false, reason: null, settings: null }, bodyInsertion: writingRange(source, source.length, source.length) }; }
function result(source: string, inserted = "// numbering\n"): WritingResult { const next = inserted + source, nextMap = map(next); return { source: next, sourceHash: nextMap.sourceHash, map: nextMap, selection: { from: inserted.length, to: inserted.length }, patches: [{ from: 0, to: 0, utf16From: 0, utf16To: 0, expected: "", insert: inserted }] }; }

test("native numbering previews pair identical titles by mapped source ranges, preserving literal prefixes", () => {
  const source = "= 一、同标题\n= 一、同标题", output = result(source);
  const rows = numberingPreviewHeadings(tree(source, ["1 ", "2 "]), tree(output.source, ["一 ", "二 "]), source, output, map(source).structures.map(item => item.range));
  assert.deepEqual(rows, [{ text: "一、同标题", before: "1", after: "一" }, { text: "一、同标题", before: "2", after: "二" }]);
});

test("personal numbering keys use stable user/library/note scopes and cannot alter author source", () => {
  const alice = personalNumberingKeys("alice", "library", "note"), bob = personalNumberingKeys("bob", "library", "note");
  assert.notEqual(alice.noteKey, bob.noteKey); assert.notEqual(alice.noteKey, personalNumberingKeys("alice", "other", "note").noteKey);
  const values = new Map<string, string>([[alice.defaultKey, "show"]]), storage = { getItem: (key: string) => values.get(key) ?? null };
  assert.equal(personalNumberingVisible(alice, storage), true); values.set(alice.noteKey, "hide"); assert.equal(personalNumberingVisible(alice, storage), false);
  assert.equal(personalNumberingVisible(bob, storage), false);
});

class Buffer implements WritingBuffer {
  state: EditorState; generation = 0; composing = false; listeners = new Set<(event: SourceEditorEvent) => void>();
  constructor(source: string) { this.state = createSourceState(source, [history()]); }
  get source() { return this.state.doc.toString(); } get selection() { return { from: this.state.selection.main.from, to: this.state.selection.main.to }; }
  observe(listener: (event: SourceEditorEvent) => void) { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; }
  emit() { for (const listener of this.listeners) listener({ kind: "source", source: this.source, generation: this.generation, selection: this.selection, composing: this.composing }); }
  applyPatches(source: string, patches: readonly SourcePatch[], selection?: SourceSelection, event = "input.structure", isolated = true) { assert.equal(this.source, source); this.state = this.state.update({ changes: patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), selection: selection ? { anchor: selection.from, head: selection.to } : undefined, userEvent: event, annotations: isolated ? isolateHistory.of("full") : undefined }).state; this.generation++; this.emit(); }
  select(from: number, to: number) { this.state = this.state.update({ selection: { anchor: from, head: to } }).state; }
  undo() { undo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit(); } }); } redo() { redo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit(); } }); }
}
function harness() {
  const editor = new Buffer("= 标题"), calls: string[] = []; let readable = true, writable = true, drift = false, previewReply: ((request: { source: string; proposalSource: string; serial: number }) => Promise<AuthoringProjection>) | undefined;
  const controller = new WritingController({ editor, basis: () => ({ sessionId: "notes", source: editor.source, serial: editor.generation }), editable: () => writable, request: async request => { calls.push("apply-edit"); return result(request.source, drift ? "// different\n" : undefined); } }); controller.installMap(map(editor.source), editor.source, 0);
  const env: AuthoringPanelsEnvironment = { controller, readable: () => readable, editable: () => writable, projection: () => ({ ok: true, source: editor.source, serial: editor.generation, tree: tree(editor.source, []) }), selection: () => editor.selection, revealSource: () => {}, requestEdit: async request => { calls.push("pure-edit"); return result(request.source); }, previewProposal: async request => { calls.push("compile-proposal"); assert.equal(editor.source, request.source, "preview must not install afterSource"); return previewReply ? previewReply(request) : { ok: true, source: request.proposalSource, serial: request.serial, tree: tree(request.proposalSource, ["一 "]) }; } };
  return { env, editor, controller, calls, drift: () => { drift = true; }, revoke: () => { writable = readable = false; }, preview: (fn: NonNullable<typeof previewReply>) => { previewReply = fn; } };
}
const action = { scheme: "chinese" as const, start: 1, maxLevel: 6, excludeLabels: [] };
const personal = { defaultKey: "personal-default", noteKey: "personal-note", display: () => {} };
test("note insertion requires a caret and never sends a selected range to the document end", () => {
  const h = harness(); try { h.editor.select(0, 2); assert.throws(() => documentNotesHost(h.env).insertionRange!(), /选中了正文范围/); h.editor.select(2, 2); assert.equal(documentNotesHost(h.env).insertionRange!().utf16From, 2); } finally { h.controller.dispose(); }
});
test("numbering opens a pure native afterSource preview before one shared Draft/Undo apply", async () => {
  const h = harness(), host = numberingHost(h.env, personal);
  try { const preview = await host.preview(action); assert.deepEqual(h.calls, ["pure-edit", "compile-proposal"]); assert.equal(h.editor.source, "= 标题"); assert.equal(preview.headings?.[0].after, "一"); await host.apply(action); assert.deepEqual(h.calls, ["pure-edit", "compile-proposal", "apply-edit"]); assert.equal(h.editor.source, preview.afterSource); h.editor.undo(); assert.equal(h.editor.source, "= 标题"); }
  finally { h.controller.dispose(); }
});
test("pending endnote qualification never bypasses the independent author write right", async () => {
  const h = harness();
  try {
    h.controller.map!.headingNumbering = { sourceHash: h.controller.map!.sourceHash, settings: null, editable: false, frozenEndnotesRequired: true, reason: { explanation: message("common.unavailable"), diagnostics: [], causes: [] } };
    let writable = true;
    h.env.editable = () => writable;
    const host = numberingHost(h.env, personal);
    assert.equal(host.inspection().frozenEndnotesRequired, true);
    writable = false;
    assert.equal(host.inspection().editable, false);
    assert.equal(host.inspection().frozenEndnotesRequired, false);
    assert.ok(host.inspection().availabilityMessage);
    await assert.rejects(host.preview(action), /写权限/);
    assert.deepEqual(h.calls, []);
    host.displayPersonal(true);
  } finally { h.controller.dispose(); }
});
test("numbering rejects wrong compile output, changed author authority and afterSource drift", async () => {
  const wrong = harness(); wrong.preview(async request => ({ ok: true, source: "substitute", serial: request.serial, tree: tree("substitute", []) }));
  try { await assert.rejects(numberingHost(wrong.env, personal).preview(action), /准确原生编译预览/); assert.equal(wrong.editor.source, "= 标题"); } finally { wrong.controller.dispose(); }
  const late = harness(); let finish!: (value: AuthoringProjection) => void; late.preview(async () => new Promise(resolve => { finish = resolve; })); const host = numberingHost(late.env, personal), pending = host.preview(action); await new Promise(resolve => setTimeout(resolve, 0)); late.revoke(); finish({ ok: true, source: result(late.editor.source).source, serial: 0, tree: tree(result(late.editor.source).source, ["一 "]) });
  try { await assert.rejects(pending, /写权限已变化/); assert.equal(late.editor.source, "= 标题"); } finally { late.controller.dispose(); }
  const drift = harness(), driftHost = numberingHost(drift.env, personal); try { await driftHost.preview(action); drift.drift(); await assert.rejects(driftHost.apply(action), /预览/); assert.equal(drift.editor.source, "= 标题"); } finally { drift.controller.dispose(); }
});
