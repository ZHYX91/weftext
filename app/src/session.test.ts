import test from "node:test";
import assert from "node:assert/strict";
import { DocumentSession, encodeLiteral, validLiteralRange, recoverySource, canRebindSaveAs, rebindCommittedSave, sourceConsumerNeedsComparison, uniqueRetainedDrafts } from "./session";
import type { DraftInput, OpenedDocument, RenderOutput } from "./types";
import { createSourceState } from "./editor";
import { relativeFileTarget, absoluteFileTarget, quoteTypstString } from "./links";
import { locale, message, uiTextValue } from "./ui-locale";

const opened: OpenedDocument = { sessionId: "s", path: "a.typ", root: ".", source: "原文", baseId: "b", kind: "file" };
const render = (source: string, serial: number): RenderOutput => ({ source, serial } as RenderOutput);

test("retained comparison preserves independent producers, roles and unknown input without text coalescing", () => {
  const input: DraftInput = { path: "a.typ", baseSource: "S", source: "same 😀", baseKnown: true, producerId: "A", role: {kind: "proposal"} };
  const independent = { ...input, producerId: "B" };
  const baseline: DraftInput = { ...input, role: {kind: "committedBaseline", operationId: "one"} };
  const later: DraftInput = { ...baseline, role: {kind: "committedBaseline", operationId: "two"} };
  const unknown: DraftInput = { path: "a.typ", baseSource: "S", source: "\ud800", baseKnown: true };
  assert.deepEqual(uniqueRetainedDrafts([input, {...input}, independent, baseline, later, unknown, {...unknown}]),
    [input, independent, baseline, later, unknown, {...unknown}]);
  assert.equal(unknown.source.charCodeAt(0), 0xd800);
});

test("only recovery actually installed by this editor establishes an explicit continuation request", () => {
  const recovered = { ...opened, recoveredDraft: "已恢复输入", recoveredDraftWitness: "a".repeat(64) };
  const session = new DocumentSession(recovered);
  assert.deepEqual(session.recoveryContinuation, { sessionId: opened.sessionId, recordWitness: "a".repeat(64), recoveredSource: "已恢复输入" });
  session.edit("继续编辑");
  assert.equal(session.recoveryContinuation?.recoveredSource, "已恢复输入");
  session.opened = { ...recovered, sessionId: "another", recoveredDraft: "另一位作者" };
  assert.equal(session.recoveryContinuation?.sessionId, opened.sessionId);
  assert.equal(new DocumentSession(opened).recoveryContinuation, null);
});

test("qualified same-Node relocation pauses consumers while equal-byte edits retain the original writer and Draft basis", () => {
  const original = { ...opened, path: "D:\\Library\\Left\\Left.typ", nodeId: "node", workspaceId: "workspace" };
  const moved = { ...original, path: "D:/Library/Renamed/Renamed.typ", sessionId: "fresh", baseId: "current", source: "当前保存稿" };
  const session = new DocumentSession(original); session.edit("个人草稿");
  assert.equal(sourceConsumerNeedsComparison(session.opened, moved), true);
  session.edit(original.source); session.edit("个人草稿");
  assert.equal(sourceConsumerNeedsComparison(session.opened, moved), true);
  assert.equal(session.opened.sessionId, "s"); assert.equal(session.baseId, "b"); assert.equal(session.baseSource, "原文"); assert.equal(session.source, "个人草稿");
  // The existing explicit comparison installs its actually reopened session.
  session.opened = moved; session.acknowledgeSave(moved.source, moved.baseId, moved.path);
  assert.equal(sourceConsumerNeedsComparison(session.opened), false); assert.equal(session.source, "个人草稿");
});

test("case/separator aliases and same-path content changes preserve normal source consumers", () => {
  const original = { path: "D:\\Library\\Note\\Note.typ", nodeId: "node", workspaceId: "workspace" };
  assert.equal(sourceConsumerNeedsComparison(original, { ...original, path: "d:/library/note/note.typ" }), false);
  assert.equal(sourceConsumerNeedsComparison(original, original), false);
  assert.equal(sourceConsumerNeedsComparison(original), false);
});

test("an unrelated target or unqualified path never asserts a same-Node placement decision", () => {
  const original = { path: "D:/Library/A/A.typ", nodeId: "node", workspaceId: "workspace" };
  assert.equal(sourceConsumerNeedsComparison(original, { ...original, path: "D:/Library/B/B.typ", nodeId: "other" }), false);
  assert.equal(sourceConsumerNeedsComparison(original, { ...original, path: "D:/Library/B/B.typ", workspaceId: "other" }), false);
  assert.equal(sourceConsumerNeedsComparison({ path: "A.typ" }, { path: "B.typ" }), false);
});

test("native link strings preserve Unicode and use Typst control escapes rather than JSON escapes", () => {
  assert.equal(quoteTypstString('中文/"A"\\B\b\f\n\t\u2028😀'), '"中文/\\"A\\"\\\\B\\u{8}\\u{c}\\u{a}\\u{9}\\u{2028}😀"');
});

test("a save completing during typing advances its base without replacing the later draft", () => {
  const session = new DocumentSession(opened);
  session.edit("提交内容");
  const submitted = session.source;
  session.edit("提交内容，继续输入");
  session.acknowledgeSave(submitted, "next");
  assert.equal(session.source, "提交内容，继续输入");
  assert.equal(session.baseSource, "提交内容");
  assert.equal(session.baseId, "next");
  assert.equal(session.dirty, true);
});

test("old preview cannot become current even after undo returns to equal bytes", () => {
  const session = new DocumentSession(opened);
  const old = render(session.source, session.serial);
  session.edit("改动");
  session.edit("原文");
  assert.equal(session.acceptProjection(old), false);
  assert.equal(session.projection, null);
  assert.equal(session.acceptProjection(render("原文", session.serial)), true);
});

test("recovered draft remains visibly dirty against real committed bytes", () => {
  const session = new DocumentSession({ ...opened, recoveredDraft: "恢复输入" });
  assert.equal(session.source, "恢复输入");
  assert.equal(session.baseSource, "原文");
  assert.equal(session.dirty, true);
  assert.equal(session.recovered, true);
});

test("literal editing refuses changed raw ranges and escapes native control punctuation", () => {
  assert.equal(validLiteralRange("中文😀文本", 2, 4, "😀"), true);
  assert.equal(validLiteralRange("新的😀文本", 2, 4, "旧文"), false);
  assert.equal(validLiteralRange("abc", -1, 2, "ab"), false);
  assert.equal(encodeLiteral("中文 #x[*强调*]\\"), "中文 \\#x\\[\\*强调\\*\\]\\\\");
});

test("an older draft persistence response cannot acknowledge newer input", () => {
  const session = new DocumentSession(opened);
  session.acknowledgeDraft("新", 4, 1);
  session.acknowledgeDraft("旧", 2, 2);
  assert.equal(session.lastDraftSource, "新");
  assert.equal(session.lastDraftSerial, 4);
});

test("save completion persists newer uncommitted rich input instead of the older shared buffer", () => {
  const session = new DocumentSession(opened);
  const proposal = { source: "原文", from: 0, to: 2, raw: "原文", text: "后续 *文字*", nodeKey: "source-key" };
  session.acknowledgeSave("原文", "saved");
  assert.equal(recoverySource(session.source, proposal), "后续 \\*文字\\*");
  session.edit("已在源码修改");
  assert.equal(recoverySource(session.source, proposal), "已在源码修改");
});

test("empty-body rich recovery proposal includes the native sibling paragraph boundary", () => {
  assert.equal(recoverySource("旧段落", { source: "旧段落", from: 3, to: 3, raw: "", text: "新段落", nodeKey: "@body" }), "旧段落\n\n新段落\n");
});

test("real CodeMirror state preserves BOM, mixed EOL, and exact UTF16 source coordinates", () => {
  const source = "\uFEFF= 标题\r\n中文😀\r\n末行\n";
  const state = createSourceState(source);
  assert.equal(state.doc.toString(), source);
  const from = source.indexOf("中文");
  const changed = state.update({ changes: { from, to: from + 2, insert: "改写" } }).state;
  assert.equal(changed.doc.toString(), "\uFEFF= 标题\r\n改写😀\r\n末行\n");
});

test("file link picking retains target meaning for Chinese names and literal fragment punctuation", () => {
  assert.equal(relativeFileTarget("D:\\库\\A\\A.typ", "D:\\库\\B\\题目#100%.typ"), "../B/%E9%A2%98%E7%9B%AE%23100%25.typ");
  assert.equal(absoluteFileTarget("D:\\库\\A.typ"), "file:///D:/%E5%BA%93/A.typ");
});

test("Save As reopens only the exact saved target and never qualifies its old proposal against an external version", () => {
  const original = new DocumentSession(opened);
  original.edit("另存为内容 A");
  const submitted = original.source;
  original.edit("继续输入 C");
  const external = { ...opened, path: "b.typ", baseId: "external", source: "外部内容 B" };
  assert.equal(canRebindSaveAs(submitted, external), false);
  assert.equal(original.path, "a.typ");
  assert.equal(original.source, "继续输入 C");
  assert.equal(original.baseId, "b");
  assert.equal(canRebindSaveAs(submitted, { ...external, source: submitted }), true);
  assert.equal(canRebindSaveAs(submitted, { ...external, source: submitted, recoveredDraftConflict: true }), false);
});

test("original save delivery rebinds a lost Session with its actual base while preserving newer input and editor state", () => {
  const session = new DocumentSession({ ...opened, nodeId: "node", workspaceId: "workspace" });
  session.edit("submitted"); const frozen = { sessionId: session.opened.sessionId, source: session.source };
  session.edit("submitted + waiting input");
  const sourceState = createSourceState(session.source), selection = sourceState.selection;
  const fresh = { ...session.opened, sessionId: "reopened", source: "submitted", baseId: "actual-fresh-base" };
  assert.equal(rebindCommittedSave(session, frozen, fresh), true);
  assert.equal(session.source, "submitted + waiting input"); assert.equal(session.baseSource, "submitted");
  assert.equal(session.baseId, "actual-fresh-base"); assert.equal(session.opened.sessionId, "reopened");
  assert.equal(sourceState.doc.toString(), "submitted + waiting input"); assert.equal(sourceState.selection, selection);
});
test("a later current source or changed owner/Session cannot borrow the original receipt base", () => {
  for (const change of [{ source: "later committed T" }, { nodeId: "different" }, { workspaceId: "different" }]) {
    const session = new DocumentSession({ ...opened, nodeId: "node", workspaceId: "workspace" }); session.edit("new input Y");
    const before = structuredClone(session.opened);
    assert.equal(rebindCommittedSave(session, { sessionId: "s", source: "submitted S" }, { ...session.opened, source: "submitted S", baseId: "actual-T-base", ...change }), false);
    assert.deepEqual(session.opened, before); assert.equal(session.source, "new input Y"); assert.equal(session.baseId, "b");
  }
});
test("an ordinary file within an admitted workspace retains that exact workspace on qualified save delivery", () => {
  const session = new DocumentSession({ ...opened, workspaceId: "workspace" });
  session.edit("input Y");
  assert.equal(rebindCommittedSave(session, { sessionId: opened.sessionId, source: "saved S" }, { ...opened, workspaceId: "foreign", source: "saved S" }), false);
  assert.equal(rebindCommittedSave(session, { sessionId: opened.sessionId, source: "saved S" }, { ...opened, workspaceId: "workspace", source: "saved S", baseId: "fresh" }), true);
  assert.equal(session.baseId, "fresh"); assert.equal(session.source, "input Y");
});

test("qualified ordinary save delivery can restore its actual ancestor owner after a default-parent open", () => {
  const session = new DocumentSession({ ...opened, path: "D:/A/nested/ordinary.typ", root: "D:/A/nested" });
  session.edit("Y");
  assert.equal(rebindCommittedSave(session, { sessionId: opened.sessionId, source: "S" }, { ...session.opened, workspaceId: "qualified-A", root: "D:/A", source: "S", baseId: "fresh" }), true);
  assert.equal(session.source, "Y");
  assert.equal(session.opened.workspaceId, "qualified-A");
});

test("a live Session error remains visible across locale and equal-source projection updates until actual edit or acknowledged save", () => {
  const session = new DocumentSession({ ...opened, workspaceId: "qualified" }), note = message("common.save");
  session.error = note; const serial = session.serial;
  locale.set("en-US"); assert.equal(uiTextValue(session.error), "Save"); assert.equal(session.source, opened.source); assert.equal(session.baseId, opened.baseId); assert.equal(session.serial, serial);
  session.edit(opened.source); assert.equal(session.error, note);
  session.acceptProjection(render(session.source, session.serial)); assert.equal(session.error, note);
  session.edit("作者新增😀\r\n"); assert.equal(session.error, null);
  session.error = note;
  assert.equal(rebindCommittedSave(session, { sessionId: opened.sessionId, source: "saved" }, { ...opened, workspaceId: "qualified", sessionId: "fresh", source: "saved", baseId: "actual" }), true);
  assert.equal(session.error, null); assert.equal(session.source, "作者新增😀\r\n"); assert.equal(session.baseId, "actual"); locale.set("zh-CN");
});

test("an unacknowledged same-value ABA cannot borrow an earlier source ACK", () => {
  const session = new DocumentSession(opened);
  session.edit("S");
  session.acknowledgeDraft("S", session.serial, 1);
  assert.equal(session.acknowledgedInputRevision, 1);
  session.edit("Y"); session.edit("S");
  assert.equal(session.acknowledgedInputRevision, null);
  session.acknowledgeDraft("S", session.serial, 3);
  assert.equal(session.acknowledgedInputRevision, 3);
});
