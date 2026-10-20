import test from "node:test";
import assert from "node:assert/strict";
import { EditorState } from "@codemirror/state";
import { history, isolateHistory, undo, redo, undoDepth } from "@codemirror/commands";
import { createSourceState, type SourceEditorEvent, type SourcePatch, type SourceSelection } from "./editor";
import { writingRange, utf8Offset, WritingController, type WritingBuffer, type WritingIntent, type WritingMap, type WritingResult } from "./writing";
import type { RenderNode } from "./types";
import { buildDocumentNotesSnapshot, copyDocumentNote, documentNotesNewInput, DocumentNotesInputTracker, performDocumentNotes, type DocumentNotesHost, type DocumentNotesIntent, type DocumentNotesProjection, type NotesSourceEntry } from "./document-notes";

function range(source: string, raw: string, from = 0) { const at = source.indexOf(raw, from); assert.ok(at >= 0, raw); return writingRange(source, at, at + raw.length); }
function node(key: string, role: string, identity: string, content = "", origin: ReturnType<typeof writingRange> | null = null, children?: RenderNode[]): RenderNode {
  return { key, kind: "element", tag: "span", text: null, attrs: { "data-weftext-role": role, [role.startsWith("footnote") || role.startsWith("endnote") ? "data-weftext-note-id" : "data-weftext-target-id"]: identity }, origin: origin ? { ...origin, path: "main.typ", isMain: true, syntaxKind: "FuncCall" } : null, writable: true,
    children: children ?? [{ key: `${key}:text`, kind: "text", tag: null, text: content, attrs: {}, origin: origin ? { ...origin, path: "main.typ", isMain: true, syntaxKind: "Text" } : null, writable: true, children: [] }] };
}
function projection(source: string, entries: NotesSourceEntry[], children: RenderNode[]): DocumentNotesProjection {
  return { source, sourceHash: `hash:${source}`, notes: { sourceHash: `hash:${source}`, syntaxValid: true, entries }, tree: node("root", "", "", "", null, children) };
}
function definition(source: string, construct: string, body: string): NotesSourceEntry { return { kind: "footnote-definition", construct: range(source, construct), body: range(source, body, source.indexOf(construct)), writable: true, label: /<([^>]+)>$/.exec(construct)?.[1] ?? null }; }

test("native declaration identities distinguish equal note text and keep repeated markers in one body", () => {
  const source = "甲#footnote[同文] <one>，乙#footnote(<one>)。丙#footnote[同文] <two>";
  const first = definition(source, "#footnote[同文] <one>", "同文"), second = definition(source, "#footnote[同文] <two>", "同文");
  const repeated: NotesSourceEntry = { kind: "footnote-reference", construct: range(source, "#footnote(<one>)"), target: "one", writable: true };
  const model = buildDocumentNotesSnapshot(projection(source, [first, repeated, second], [node("mark-one", "footnote-reference", "decl-one", "1", first.construct), node("mark-repeat", "footnote-reference", "decl-one", "1", repeated.construct), node("mark-two", "footnote-reference", "decl-two", "2", second.construct), node("body-two", "footnote-body", "decl-two", "同文", second.body!), node("body-one", "footnote-body", "decl-one", "同文", first.body!)]));
  assert.equal(model.available, true); assert.equal(model.notes.length, 2);
  assert.equal(model.notes[0].references.length, 2); assert.equal(model.notes[0].definition, first);
  assert.equal(model.notes[1].definition, second); assert.equal(model.notes[0].writable, true);
  assert.equal(model.notes[0].bodyRange?.raw, "同文"); assert.deepEqual(model.unlocated, []);
});

test("source hash, exact Unicode ranges and duplicate native body evidence fail closed", () => {
  const source = "😀#footnote[*中文*] <one>", entry = definition(source, "#footnote[*中文*] <one>", "*中文*");
  const p = projection(source, [entry], [node("marker", "footnote-reference", "decl", "1", entry.construct), node("body", "footnote-body", "decl", "中文", entry.body!)]);
  assert.equal(buildDocumentNotesSnapshot(p).notes[0].writable, true);
  assert.equal(buildDocumentNotesSnapshot({ ...p, sourceHash: "later-hash" }).available, false);
  assert.equal(buildDocumentNotesSnapshot({ ...p, notes: { ...p.notes, entries: [{ ...entry, construct: { ...entry.construct, raw: "other" } }] } }).available, false);
  assert.equal(buildDocumentNotesSnapshot({ ...p, tree: { ...p.tree!, children: [...p.tree!.children, node("other-body", "footnote-body", "decl", "中文", entry.body!)] } }).notes[0].writable, false);
  assert.equal(buildDocumentNotesSnapshot({ ...p, tree: { ...p.tree!, children: p.tree!.children.map(item => ({ ...item, attrs: { role: item.attrs["data-weftext-role"], "data-weftext-note-id": "decl" } })) } }).notes.length, 0, "untyped author HTML role does not establish semantics");
});

test("caption ownership joins the actual figure target, independent of text and DOM order", () => {
  const source = "#figure(table(columns:1,[甲]), caption:[同文]) <a>\n#figure(table(columns:1,[乙]), caption:[同文]) <b>";
  const first: NotesSourceEntry = { kind: "figure", construct: range(source, source.split("\n")[0]), body: range(source, "同文"), writable: true }, second: NotesSourceEntry = { kind: "figure", construct: range(source, source.split("\n")[1]), body: range(source, "同文", source.indexOf("\n")), writable: true };
  const a = node("figure-a", "figure", "target-a", "", first.construct); a.attrs["data-weftext-caption-kind"] = "table"; a.attrs["data-weftext-numbered"] = "true";
  const b = node("figure-b", "figure", "target-b", "", second.construct); b.attrs["data-weftext-caption-kind"] = "table";
  const model = buildDocumentNotesSnapshot(projection(source, [first, second], [a, b, node("caption-b", "caption-body", "target-b", "同文", second.body!), node("prefix-a", "caption-prefix", "target-a", "表 1："), node("caption-a", "caption-body", "target-a", "同文", first.body!)]));
  assert.equal(model.captions.length, 2); assert.equal(model.captions[0].entry, first); assert.equal(model.captions[0].prefix, "表 1："); assert.equal(model.captions[0].numbered, true);
  assert.equal(model.captions[1].entry, second); assert.equal(model.captions[1].prefix, ""); assert.equal(model.captions[0].writable, true);
});

test("equation body numbering has its own actual source and never borrows a caption counter", () => {
  const source = "$ x^2 $", equationRange = range(source, source), math = node("eq", "equation", "equation-one", "", equationRange); math.attrs["data-weftext-numbered"] = "true";
  const p = projection(source, [], [math, node("number", "equation-prefix", "equation-one", "(3)"), node("other-caption", "caption-prefix", "figure-other", "图 8：")]);
  p.structures = [{ kind: "displayMath", range: equationRange, body: range(source, "x^2"), runIn: false }];
  const model = buildDocumentNotesSnapshot(p); assert.equal(model.equations.length, 1); assert.equal(model.equations[0].prefix, "(3)"); assert.equal(model.equations[0].writable, true); assert.deepEqual(model.equations[0].range, equationRange);
  p.tree!.children.push({ ...math, key: "duplicate-output" }); assert.ok(buildDocumentNotesSnapshot(p).equations.every(item => !item.writable));
});

test("generated or foreign note bodies remain readable without acquiring a local writer", () => {
  const source = "#include \"other.typ\"", body = node("body", "footnote-body", "foreign", "外部正文"), marker = node("marker", "footnote-reference", "foreign", "1");
  const model = buildDocumentNotesSnapshot(projection(source, [], [marker, body]));
  assert.equal(model.notes.length, 1); assert.equal(model.notes[0].text, "外部正文"); assert.equal(model.notes[0].writable, false); assert.equal(model.notes[0].definition, null);
});

test("revocation recovery keeps newly entered Unicode without redisplaying old note contents", () => {
  assert.equal(documentNotesNewInput("私有原注😀结尾", "私有原注😀新输入🧑‍💻结尾"), "新输入🧑‍💻");
  assert.equal(documentNotesNewInput("甲😀乙", "甲😁乙"), "", "a replacement without input evidence is conservative");
  assert.equal(documentNotesNewInput("私有原注", "私有原注"), "");
  assert.equal(documentNotesNewInput("旧稿秘密", "新增甲旧稿秘密新增乙"), "");
  const tracker = new DocumentNotesInputTracker("旧稿秘密😀");
  tracker.input("新增甲旧稿秘密😀", { from: 0, to: 0, inputType: "insertText" });
  tracker.input("新增甲旧稿秘密😀新增乙", { from: 9, to: 9, inputType: "insertFromPaste" });
  assert.equal(tracker.newInput(), "新增甲\n新增乙");
  tracker.input("新增甲旧稿秘密😀", { from: 0, to: 0, inputType: "historyUndo" });
  assert.equal(tracker.newInput(), "新增甲");
  tracker.input("新增甲旧稿秘密😀新增乙", { from: 0, to: 0, inputType: "historyRedo" });
  assert.equal(tracker.newInput(), "新增甲\n新增乙");
  tracker.input("unproven combined browser change"); assert.equal(tracker.newInput(), "");
});

test("footnotes and authenticated endnotes keep separate kinds and ID scopes", () => {
  const source = "#footnote[脚注] <f>\n#endnote(id:\"end-one\")[尾注]", foot = definition(source, "#footnote[脚注] <f>", "脚注"), end: NotesSourceEntry = { kind: "endnote-definition", construct: range(source, "#endnote(id:\"end-one\")[尾注]"), body: range(source, "尾注"), target: "end-one", writable: true };
  const model = buildDocumentNotesSnapshot(projection(source, [foot, end], [node("fmark", "footnote-reference", "shared", "1", foot.construct), node("fbody", "footnote-body", "shared", "脚注", foot.body!), node("emark", "endnote-reference", "end-one", "1"), node("ebody", "endnote-body", "end-one", "尾注", end.body!)]));
  assert.deepEqual(model.notes.map(item => item.kind), ["footnote", "endnote"]); assert.equal(model.notes[1].definition, end);
});

class Buffer implements WritingBuffer {
  state: EditorState; generation = 0; composing = false; listeners = new Set<(event: SourceEditorEvent) => void>();
  constructor(source: string) { this.state = createSourceState(source, [history()]); }
  get source() { return this.state.doc.toString(); }
  get selection() { return { from: this.state.selection.main.from, to: this.state.selection.main.to }; }
  observe(listener: (event: SourceEditorEvent) => void) { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; }
  emit() { for (const listener of this.listeners) listener({ kind: "source", source: this.source, generation: this.generation, selection: this.selection, composing: this.composing }); }
  applyPatches(source: string, patches: readonly SourcePatch[], selection?: SourceSelection, event = "input.structure", isolated = true) {
    assert.equal(this.source, source); this.state = this.state.update({ changes: patches.map(patch => ({ from: patch.from, to: patch.to, insert: patch.insert })), selection: selection ? { anchor: selection.from, head: selection.to } : undefined, userEvent: event, annotations: isolated ? isolateHistory.of("full") : undefined }).state; this.generation++; this.emit();
  }
  select(from: number, to: number) { this.state = this.state.update({ selection: { anchor: from, head: to } }).state; }
  undo() { undo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit(); } }); }
  redo() { redo({ state: this.state, dispatch: transaction => { this.state = transaction.state; this.generation++; this.emit(); } }); }
}
function currentMap(source: string): WritingMap { return { sourceHash: `hash:${source}`, syntaxValid: true, literalRegions: [], structures: [], title: { status: "absent" }, subtitle: { status: "absent" }, bodyInsertion: writingRange(source, source.length, source.length) }; }
function harness(source: string, respond?: (request: DocumentNotesIntent, old: string) => Promise<WritingResult>) {
  const editor = new Buffer(source); let serial = 0, writable = true, readable = true, calls = 0;
  editor.observe(() => { serial++; });
  const controller = new WritingController({ editor, basis: () => ({ sessionId: "notes", source: editor.source, serial }), editable: () => writable, request: async request => {
    calls++; if (!respond) throw new Error("no adapter"); return respond(request.intent as unknown as DocumentNotesIntent, request.source);
  } }); controller.installMap(currentMap(source), source, serial);
  const host: DocumentNotesHost = { controller, editable: () => writable, projection: () => readable ? projection(editor.source, [], []) : null, perform: (intent, basis) => controller.execute(() => { if (!controller.isSnapshotCurrent(basis)) throw new Error("old basis"); return intent as unknown as WritingIntent; }) };
  return { editor, controller, host, calls: () => calls, revoke: () => { writable = false; readable = false; }, readonly: () => { writable = false; } };
}
test("note source proposals use one real shared Draft/Undo transaction and preserve unrelated bytes", async () => {
  const original = "甲\r\n#footnote[*原注*] <one>\r\n乙😀", old = range(original, "*原注*"), replacement = "*新注*\n\n- 第一项";
  const h = harness(original, async (intent, source) => {
    assert.equal(intent.kind, "nativeNote"); assert.equal(intent.action.kind, "setNoteBody"); if (intent.action.kind !== "setNoteBody") throw new Error("wrong action"); assert.deepEqual(intent.action.body, { mode: "source", value: replacement });
    const next = source.slice(0, old.utf16From) + replacement + source.slice(old.utf16To), map = currentMap(next);
    return { source: next, sourceHash: map.sourceHash, map, patches: [{ from: old.from, to: old.to, utf16From: old.utf16From, utf16To: old.utf16To, expected: old.raw, insert: replacement }], selection: { from: old.utf16From + replacement.length, to: old.utf16From + replacement.length } };
  });
  try {
    await performDocumentNotes(h.host, h.controller.snapshot, `hash:${original}`, { kind: "setNoteBody", construct: range(original, "#footnote[*原注*] <one>"), body: { mode: "source", value: replacement } });
    assert.equal(h.calls(), 1); assert.ok(h.editor.source.startsWith("甲\r\n#footnote[")); assert.ok(h.editor.source.endsWith("] <one>\r\n乙😀")); assert.equal(undoDepth(h.editor.state), 1);
    h.editor.undo(); assert.equal(h.editor.source, original); h.editor.redo(); assert.ok(h.editor.source.includes(replacement));
  } finally { h.controller.dispose(); }
});

test("clipboard rejection, revocation and stale proposals never mutate the shared Draft", async () => {
  const h = harness("正文😀"); const basis = h.controller.snapshot, hash = `hash:${basis.source}`;
  try {
    await assert.rejects(copyDocumentNote(h.host, basis, hash, "注释", async () => { throw new Error("clipboard denied"); }), /clipboard denied/);
    assert.equal(h.editor.source, "正文😀"); assert.equal(h.calls(), 0);
    h.readonly(); await assert.rejects(performDocumentNotes(h.host, basis, hash, { kind: "insertFootnote", at: writingRange(basis.source, 0, 0), body: { mode: "literal", value: "输入" } }), /写权限/);
    h.revoke(); await assert.rejects(copyDocumentNote(h.host, basis, hash, "旧注释", async () => assert.fail("must not touch clipboard after revocation")), /来源已变化/);
    assert.equal(h.calls(), 0); assert.equal(h.editor.source, "正文😀");
  } finally { h.controller.dispose(); }
});

test("a late native note response cannot overwrite newer source or revoked write authority", async () => {
  let finish!: (value: WritingResult) => void, started!: () => void; const sent = new Promise<void>(resolve => { started = resolve; });
  const h = harness("甲", async () => { started(); return new Promise(resolve => { finish = resolve; }); });
  try {
    const basis = h.controller.snapshot, pending = performDocumentNotes(h.host, basis, "hash:甲", { kind: "insertFootnote", at: writingRange("甲", 1, 1), body: { mode: "literal", value: "新注" } }); await sent;
    h.editor.applyPatches("甲", [{ from: 1, to: 1, expected: "", insert: "后输入" }]); h.revoke(); const next = "甲#footnote[新注]", map = currentMap(next);
    finish({ source: next, sourceHash: map.sourceHash, map, patches: [{ from: utf8Offset("甲", 1), to: utf8Offset("甲", 1), utf16From: 1, utf16To: 1, expected: "", insert: "#footnote[新注]" }], selection: { from: next.length, to: next.length } });
    await assert.rejects(pending, /旧提案未应用/); assert.equal(h.editor.source, "甲后输入"); assert.equal(h.calls(), 1);
  } finally { h.controller.dispose(); }
});
