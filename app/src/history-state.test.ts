import test from "node:test";
import assert from "node:assert/strict";
import { HistoryController, historySourceDiff, historyTimeLabel, restoreBufferDisposition, type HistoryContext, type HistoryList, type HistoryRestoreRequest, type HistoryRestoreResult, type HistorySource, type HistoryTransport, type SavedHistoryComparison } from "./history-state";
import { DocumentSession } from "./session";
import { createSourceState } from "./editor";
import { history, undoDepth } from "@codemirror/commands";

const original: HistoryContext = {
  sessionId: "session", root: "D:/notes", path: "D:/notes/笔记/笔记.typ", workspaceId: "workspace", nodeId: "node",
  source: "v3\r\n", baseSource: "v3\r\n", baseId: "base3", serial: 0, dirty: false,
};
const old: HistorySource = {
  versionId: "save1", operationId: "save1", revisionId: "revision1", kind: "save", savedAtUnixMs: 1234,
  timeBasis: "commitClock", commitSequence: 1, status: "available", byteLength: 4, sha256: "digest", message: "",
  source: "v1\r\n", readOnly: true, previewStatus: "sourceOnly", previewMessage: "准确源码可读；历史排版未提供。",
};
const comparison = (context = original): SavedHistoryComparison => ({ selected: structuredClone(old), current: { versionId: "save3", revisionId: "revision3", source: context.baseSource, baseId: context.baseId, path: context.path }, equal: false, comparison: "currentSaved" });
const saved = (request: HistoryRestoreRequest): HistoryRestoreResult => ({ receipt: { operationId: request.operationId, status: "saved", path: original.path, baseId: "base4", message: "saved" }, source: old.source, versionId: request.versionId, protectedDrafts: request.draftSource === original.baseSource ? [] : [{ path: original.path, baseSource: original.baseSource, source: request.draftSource!, baseKnown: true }] });
function fixture(context: HistoryContext = structuredClone(original)) {
  let current: HistoryContext | null = context;
  const requests: HistoryRestoreRequest[] = [];
  const list: HistoryList = {
    nodeId: "node", workspaceId: "workspace", path: original.path, currentBaseId: "base3", currentSavedVersionId: "save3", currentness: "saved",
    versions: [structuredClone(old)], retention: { policy: "retainedCoreJournal", complete: false, knownVersions: 1, availableVersions: 1, oldestSavedAtUnixMs: 1234, newestSavedAtUnixMs: 1234, message: "只显示实际保留记录" }, warnings: [], protectedDrafts: [],
  };
  const host: HistoryTransport = {
    current: () => current, id: () => "restore4", list: async () => structuredClone(list), read: async () => structuredClone(old), compare: async () => comparison(),
    restore: async request => { requests.push(structuredClone(request)); return saved(request); }, status: async request => ({ ...request, status: "missing" }),
  };
  const controller = new HistoryController(host, Object.freeze(structuredClone(context)));
  return { controller, host, requests, list, setCurrent: (value: HistoryContext | null) => { current = value; }, current: () => current! };
}
const coded = (code: string): Error => Object.assign(new Error(code), { code });
const deferred = <T>() => { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; };

test("unknown times never borrow filesystem time, and the UI preserves Core-provided ordering", async () => {
  const f = fixture();
  f.list.versions = [
    { ...old, versionId: "save3", commitSequence: 3, savedAtUnixMs: 1000 },
    { ...old, versionId: "save2", commitSequence: 2, savedAtUnixMs: 2000 },
    { ...old, versionId: "save1", commitSequence: 1, savedAtUnixMs: 2000 },
    { ...old, versionId: "legacy", commitSequence: null, savedAtUnixMs: null, timeBasis: "unknown" },
  ];
  await f.controller.refresh();
  assert.deepEqual(f.controller.list!.versions.map(row => row.versionId), ["save3", "save2", "save1", "legacy"]);
  assert.equal(historyTimeLabel(f.list.versions[3]), "时间未知");
  assert.equal(historyTimeLabel({ savedAtUnixMs: 2000, timeBasis: "unknown" }), "时间未知");
  assert.equal(historyTimeLabel({ savedAtUnixMs: Number.NaN, timeBasis: "commitClock" }), "时间未知");
});

test("history identity is scoped to stable node identity; a same-name copy and replaced session cannot receive delayed reads", async () => {
  const f = fixture(), read = deferred<HistorySource>(); f.host.read = () => read.promise;
  const pending = f.controller.select("save1");
  f.setCurrent({ ...original, nodeId: "copy-node" }); read.resolve(old);
  assert.equal(await pending, false); assert.equal(f.controller.selected, null);
  await assert.rejects(f.controller.refresh(), { code: "noteChanged" });
  f.setCurrent({ ...original, sessionId: "replacement-session" });
  await assert.rejects(f.controller.select("save1"), { code: "noteChanged" });
});

test("managed path change preserves same note timeline while a list from another identity is rejected", async () => {
  const f = fixture(); f.setCurrent({ ...original, path: "D:/notes/移动/改名.typ" }); f.list.path = f.current().path;
  assert.equal(await f.controller.refresh(), true); assert.equal(f.controller.list!.path, f.current().path);
  f.list.nodeId = "same-name-other-node";
  await assert.rejects(f.controller.refresh(), { code: "identityPending" });
});

test("unavailable or wrong-version records never show latest as the requested old source", async () => {
  const f = fixture(); f.list.versions[0].status = "missing"; await f.controller.refresh();
  let reads = 0; f.host.read = async () => { reads++; return { ...old, versionId: "save3", source: "v3" }; };
  await assert.rejects(f.controller.select("save1"), { code: "historyUnavailable" }); assert.equal(reads, 0);
  f.list.versions[0].status = "available"; await f.controller.refresh();
  await assert.rejects(f.controller.select("save1"), { code: "historyUnavailable" }); assert.equal(f.controller.selected, null);
});

test("browse and saved/draft comparisons preserve the exact shared Draft and CodeMirror Undo", async () => {
  const source = "\uFEFF= 标题\r\n草稿😀\n";
  const session = new DocumentSession({ sessionId: original.sessionId, root: original.root, path: original.path, source: original.baseSource, baseId: original.baseId, kind: "node", nodeId: original.nodeId, workspaceId: original.workspaceId });
  session.edit(source);
  let editor = createSourceState(original.source, [history()]); editor = editor.update({ changes: { from: 0, to: editor.doc.length, insert: source } }).state;
  const f = fixture({ ...original, source, serial: session.serial, dirty: true });
  const snapshot = () => f.controller.comparison!;
  await f.controller.refresh(); await f.controller.select("save1"); await f.controller.compareSaved();
  assert.equal(snapshot().comparison, "currentSaved"); assert.equal(snapshot().current.source, original.baseSource);
  f.controller.compareDraft();
  assert.equal(snapshot().comparison, "draftSnapshot"); assert.equal(snapshot().current.source, source);
  f.setCurrent({ ...f.current(), source: source + "后续", serial: 2 });
  assert.equal(snapshot().current.source, source);
  assert.equal(session.source, source); assert.equal(editor.doc.toString(), source); assert.equal(undoDepth(editor), 1); assert.equal(f.requests.length, 0);
});

test("late saved comparison cannot replace a subsequently chosen draft snapshot", async () => {
  const f = fixture({ ...original, source: "unsaved", dirty: true }), read = deferred<SavedHistoryComparison>();
  await f.controller.select("save1"); f.host.compare = () => read.promise;
  const pending = f.controller.compareSaved(); f.controller.compareDraft(); read.resolve(comparison());
  assert.equal(await pending, false); assert.equal(f.controller.comparison!.comparison, "draftSnapshot");
});

test("restore cancellation has no save, no new operation and no buffer change", async () => {
  const f = fixture({ ...original, source: "must keep", dirty: true, serial: 2 });
  await f.controller.select("save1"); await f.controller.prepareRestore(); f.controller.cancelRestore();
  assert.equal(f.controller.review, null); assert.equal(f.controller.operationId, null); assert.equal(f.requests.length, 0); assert.equal(f.current().source, "must keep");
});

test("restore freezes current Base and requires explicit durable draft protection", async () => {
  const f = fixture({ ...original, source: "protected draft\r\n", dirty: true, serial: 2 });
  await f.controller.select("save1"); const review = await f.controller.prepareRestore();
  await assert.rejects(f.controller.restore("none"), { code: "draftProtectionRequired" }); assert.equal(f.requests.length, 0);
  const result = await f.controller.restore("retain");
  assert.deepEqual(f.requests[0], { sessionId: original.sessionId, baseId: "base3", versionId: "save1", operationId: "restore4", draftAction: "retain", draftSource: "protected draft\r\n" });
  assert.equal(result.source, old.source); assert.equal(result.versionId, "save1"); assert.equal(result.receipt.operationId, "restore4"); assert.equal(result.protectedDrafts[0].source, "protected draft\r\n");
  assert.equal(restoreBufferDisposition(review.basis, f.current()), "retainDraft"); assert.equal(f.current().source, "protected draft\r\n");
});

test("input after review invalidates confirmation; input during commit survives receipt application", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.setCurrent({ ...original, source: "typed before confirm", serial: 1, dirty: true });
  await assert.rejects(f.controller.restore("retain"), { code: "draftChanged" }); assert.equal(f.requests.length, 0);
  f.setCurrent(structuredClone(original)); const review = await f.controller.prepareRestore();
  const commit = deferred<HistoryRestoreResult>(); f.host.restore = request => { f.requests.push(structuredClone(request)); return commit.promise; };
  const pending = f.controller.restore("retain");
  f.setCurrent({ ...original, source: "typed while saving", serial: 1, dirty: true }); commit.resolve(saved(f.requests[0])); await pending;
  assert.equal(restoreBufferDisposition(review.basis, f.current()), "retainDraft");
  assert.equal(restoreBufferDisposition(review.basis, original), "replaceClean");
  assert.equal(restoreBufferDisposition(review.basis, { ...original, nodeId: "other" }), "unrelated");
});

test("external modification before restore review or after preparation cannot be acknowledged as a save", async () => {
  const f = fixture(); await f.controller.select("save1");
  f.host.compare = async () => ({ ...comparison(), current: { ...comparison().current, source: "external", baseId: "externalBase" } });
  await assert.rejects(f.controller.prepareRestore(), { code: "staleBase" }); assert.equal(f.requests.length, 0);
  f.host.compare = async () => comparison(); await f.controller.prepareRestore();
  f.host.restore = async () => { throw coded("conflict"); };
  await assert.rejects(f.controller.restore("retain"), { code: "conflict" });
  assert.equal(f.controller.operationId, "restore4");
  assert.equal(await f.controller.inspectOriginal(), null);
  assert.equal(f.controller.operationId, null); assert.equal(f.controller.review, null); assert.equal(f.current().baseId, "base3");
});

test("postcommit evidence-read failure keeps the original operation until its actual status is known", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => { f.requests.push(structuredClone(request)); throw coded("historyUnavailable"); };
  await assert.rejects(f.controller.restore("retain"), { code: "historyUnavailable" });
  assert.equal(f.controller.operationId, "restore4");
  f.host.status = async request => ({ ...request, status: "committed", result: saved(f.requests[0]) });
  const result = await f.controller.inspectOriginal();
  assert.equal(result!.source, old.source); assert.equal(f.requests.length, 1);
});

test("lost restore response queries and retries the same frozen request, even as the active Draft changes", async () => {
  const f = fixture({ ...original, source: "draft A", dirty: true, serial: 1 });
  await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => { f.requests.push(structuredClone(request)); throw new TypeError("network response lost"); };
  await assert.rejects(f.controller.restore("retain"), /network response lost/);
  assert.equal(f.controller.operationId, "restore4"); assert.equal(f.controller.uncertain, true);
  f.setCurrent({ ...f.current(), source: "draft B", serial: 2 });
  await assert.rejects(f.controller.prepareRestore(), { code: "operationPending" }); await assert.rejects(f.controller.restore("retain"), { code: "operationPending" });
  let queried = ""; f.host.status = async request => { queried = request.operationId; return { ...request, status: "missing" }; };
  assert.equal(await f.controller.inspectOriginal(), null); assert.equal(queried, "restore4"); assert.equal(f.controller.operationId, "restore4");
  f.host.restore = async request => { f.requests.push(structuredClone(request)); return saved(request); };
  const result = await f.controller.retryOriginal(); assert.equal(result.receipt.operationId, "restore4");
  assert.deepEqual(f.requests[1], f.requests[0]); assert.equal(f.requests[1].draftSource, "draft A"); assert.equal(f.current().source, "draft B");
});

test("committed original result reconciles without another save; malformed receipts stay uncertain", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => { f.requests.push(structuredClone(request)); return { ...saved(request), source: "not selected" }; };
  await assert.rejects(f.controller.restore("retain"), { code: "resultUnknown" }); assert.equal(f.controller.operationId, "restore4");
  f.host.status = async request => ({ ...request, status: "committed", result: saved(f.requests[0]) });
  const result = await f.controller.inspectOriginal(); assert.equal(result!.source, old.source); assert.equal(f.requests.length, 1); assert.equal(f.controller.operationId, null);
});

test("pending recovery cannot cancel its operation; resolved rollback clears only the operation, not input", async () => {
  const f = fixture({ ...original, source: "draft", dirty: true }); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async () => { throw coded("recoveryPending"); }; await assert.rejects(f.controller.restore("retain"));
  assert.throws(() => f.controller.cancelRestore(), { code: "operationPending" });
  f.host.status = async request => ({ ...request, status: "recoveryPending" }); assert.equal(await f.controller.inspectOriginal(), null); assert.equal(f.controller.operationId, "restore4");
  f.host.status = async request => ({ ...request, status: "resolved" }); assert.equal(await f.controller.inspectOriginal(), null); assert.equal(f.controller.operationId, null); assert.equal(f.current().source, "draft");
});

test("exact diff lanes reconstruct BOM, Unicode, mixed endings and trailing whitespace, including bounded large inputs", () => {
  for (const [before, after] of [
    ["\uFEFF= 旧题\r\nA😀\n\n结尾 \r", "\uFEFF= 新题\r\nA😀\n新增\n结尾 \r"],
    ["", "\n"], ["x\r\n", "x\n"], ["a\na\nb\n", "a\nb\na\n"],
    [Array.from({ length: 700 }, (_, i) => `old ${i}\r\n`).join(""), Array.from({ length: 700 }, (_, i) => `new ${i}\n`).join("")],
  ]) {
    const diff = historySourceDiff(before, after);
    assert.equal(diff.filter(part => part.kind !== "added").map(part => part.text).join(""), before);
    assert.equal(diff.filter(part => part.kind !== "removed").map(part => part.text).join(""), after);
  }
});
