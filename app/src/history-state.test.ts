import { message, uiTextValue } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { HistoryController, historySourceDiff, historyTimeLabel, restoreBufferDisposition, type HistoryContext, type HistoryList, type HistoryOperationStatus, type HistoryRestoreRequest, type HistoryRestoreResult, type HistorySettlement, type HistorySource, type HistoryTransport, type SavedHistoryComparison } from "./history-state";
import { DocumentSession } from "./session";
import { createSourceState } from "./editor";
import { history, undoDepth } from "@codemirror/commands";

const original: HistoryContext = {
  sessionId: "session", root: "D:/notes", path: "D:/notes/笔记/笔记.typ", workspaceId: "workspace", nodeId: "node",
  source: "v3\r\n", baseSource: "v3\r\n", baseId: "base3", serial: 0, dirty: false,
};
const old: HistorySource = {
  versionId: "save1", operationId: "save1", revisionId: "revision1", kind: "save", savedAtUnixMs: 1234,
  timeBasis: "commitClock", commitSequence: 1, status: "available", byteLength: 4, sha256: "digest", description: { explanation: message("history.availableSource"), diagnostics: [], causes: [] },
  source: "v1\r\n", readOnly: true, previewStatus: "sourceOnly", previewDescription: { explanation: message("history.sourceOnlyHelp"), diagnostics: [], causes: [] },
};
const comparison = (context = original): SavedHistoryComparison => ({ selected: structuredClone(old), current: { versionId: "save3", revisionId: "revision3", source: context.baseSource, baseId: context.baseId, path: context.path }, equal: false, comparison: "currentSaved" });
const saved = (request: HistoryRestoreRequest): HistoryRestoreResult => ({ receipt: { operationId: request.operationId, status: "saved", path: original.path, baseId: "base4" }, source: old.source, versionId: request.versionId, protectedDrafts: request.draftSource === original.baseSource ? [] : [{ path: original.path, baseSource: original.baseSource, source: request.draftSource!, baseKnown: true }] });
const committed = (result: HistoryRestoreResult): HistoryOperationStatus => ({ operationId: result.receipt.operationId, status: "committed", receipt: structuredClone(result.receipt), result });
function fixture(context: HistoryContext = structuredClone(original)) {
  let current: HistoryContext | null = context;
  const requests: HistoryRestoreRequest[] = [];
  const list: HistoryList = {
    nodeId: "node", workspaceId: "workspace", path: original.path, currentBaseId: "base3", currentSavedVersionId: "save3", currentness: "saved",
    versions: [structuredClone(old)], retention: { policy: "retainedCoreJournal", complete: false, knownVersions: 1, availableVersions: 1, oldestSavedAtUnixMs: 1234, newestSavedAtUnixMs: 1234, description: {explanation: message("history.retentionUnknownHelp"), diagnostics: [], causes: []} }, warnings: [], protectedDrafts: [],
  };
  const host: HistoryTransport = {
    settle: async request => ({ operationId: request.operationId, status: "resolved" }),
    current: () => current, id: () => "restore4", list: async () => structuredClone(list), read: async () => structuredClone(old), compare: async () => comparison(),
    restore: async request => { requests.push(structuredClone(request)); return committed(saved(request)); }, status: async request => ({ ...request, status: "missing" }),
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
  assert.equal(uiTextValue(historyTimeLabel(f.list.versions[3])), "时间未知");
  assert.equal(uiTextValue(historyTimeLabel({ savedAtUnixMs: 2000, timeBasis: "unknown" })), "时间未知");
  assert.equal(uiTextValue(historyTimeLabel({ savedAtUnixMs: Number.NaN, timeBasis: "commitClock" })), "时间未知");
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
  const commit = deferred<HistoryOperationStatus>(); f.host.restore = request => { f.requests.push(structuredClone(request)); return commit.promise; };
  const pending = f.controller.restore("retain");
  f.setCurrent({ ...original, source: "typed while saving", serial: 1, dirty: true }); commit.resolve(committed(saved(f.requests[0]))); await pending;
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
  f.host.status = async () => committed(saved(f.requests[0]));
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
  f.host.settle = async () => { throw new Error("fence unavailable"); };
  await assert.rejects(f.controller.inspectOriginal(), /fence unavailable/);
  f.host.status = async request => { queried=request.operationId; return { ...request,status:"recoveryPending" }; };
  assert.equal(await f.controller.inspectOriginal(), null); assert.equal(queried, "restore4"); assert.equal(f.controller.operationId, "restore4");
  f.host.restore = async request => { f.requests.push(structuredClone(request)); return committed(saved(request)); };
  const result = await f.controller.retryOriginal(); assert.equal(result.receipt.operationId, "restore4");
  assert.deepEqual(f.requests[1], f.requests[0]); assert.equal(f.requests[1].draftSource, "draft A"); assert.equal(f.current().source, "draft B");
});

test("a malformed nested result preserves the known committed receipt until correct delivery", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => { f.requests.push(structuredClone(request)); return committed({ ...saved(request), source: "not selected" }); };
  await assert.rejects(f.controller.restore("retain"), { code: "resultUnknown" }); assert.equal(f.controller.operationId, "restore4");
  assert.equal(f.controller.operationState, "committed"); assert.equal(f.controller.uncertain, false);
  assert.deepEqual(f.controller.originalReceipt, saved(f.requests[0]).receipt);
  f.host.status = async () => committed(saved(f.requests[0]));
  const result = await f.controller.inspectOriginal(); assert.equal(result!.source, old.source); assert.equal(f.requests.length, 1); assert.equal(f.controller.operationId, null);
  const nested = fixture(); await nested.controller.select("save1"); await nested.controller.prepareRestore();
  nested.host.restore = async request => { nested.requests.push(structuredClone(request)); const good = saved(request); return { ...committed(good), result: { ...good, receipt: { ...good.receipt, baseId: 42 } as unknown as typeof good.receipt } }; };
  await assert.rejects(nested.controller.restore("retain"), { code: "resultUnknown" });
  assert.deepEqual(nested.controller.originalReceipt, saved(nested.requests[0]).receipt);
  nested.host.status = async () => committed(saved(nested.requests[0]));
  assert.equal((await nested.controller.inspectOriginal())?.source, old.source); assert.equal(nested.requests.length, 1);
});

test("a complete recovered envelope is consumed immediately without a second status request", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.status = async () => { throw new Error("complete delivery must not need another lookup"); };
  const result = await f.controller.restore("retain");
  assert.equal(result.source, old.source); assert.equal(f.controller.operationId, null);
  assert.deepEqual(f.controller.originalReceipt, result.receipt); assert.equal(f.requests.length, 1);
});

test("committed without result keeps receipt and ID through read failures, and retry only delivers status", async () => {
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => {
    f.requests.push(structuredClone(request));
    return { operationId: request.operationId, status: "committed", receipt: saved(request).receipt, resultError: { code: "readUnavailable", explanation: message("history.committedResultUnavailable"), diagnostics: [{origin:"os", text:"TEST retained result temporarily unavailable"}], causes: [] } };
  };
  await assert.rejects(f.controller.restore("retain"), /原恢复已提交/);
  const receipt = structuredClone(f.controller.originalReceipt);
  assert.equal(f.controller.operationState, "committed"); assert.equal(f.controller.uncertain, false); assert.equal(f.controller.operationId, "restore4");
  let settlements = 0; f.host.settle = async () => { settlements++; throw new Error("a known commit must not settle again"); };
  for (const inconsistent of [{ operationId: "restore4", status: "missing" }, { operationId: "restore4", status: "recoveryPending" }, { operationId: "different", status: "missing" }] as const) {
    f.host.status = async () => inconsistent;
    await assert.rejects(f.controller.inspectOriginal(), { code: "resultUnknown" });
    assert.deepEqual(f.controller.originalReceipt, receipt); assert.equal(f.controller.operationState, "committed");
  }
  assert.equal(settlements, 0);
  f.setCurrent({ ...original, source: "waiting input Y", dirty: true, serial: 1 });
  f.host.status = async () => { throw new Error("read still unavailable"); };
  await assert.rejects(f.controller.inspectOriginal(), /read still unavailable/);
  assert.deepEqual(f.controller.originalReceipt, receipt); assert.equal(f.controller.operationState, "committed");
  f.host.status = async request => ({ ...request, status: "committed", result: { ...saved(f.requests[0]), source: "malformed later delivery" } });
  await assert.rejects(f.controller.inspectOriginal(), { code: "resultUnknown" });
  assert.deepEqual(f.controller.originalReceipt, receipt); assert.equal(f.controller.uncertain, false);
  f.host.status = async () => committed(saved(f.requests[0]));
  const result = await f.controller.retryOriginal();
  assert.equal(result.source, old.source); assert.equal(f.requests.length, 1); assert.equal(f.current().source, "waiting input Y");
  assert.equal(f.controller.operationId, null);
});

test("a commit between missing and settlement preserves its flat fact and fetches the typed history result", async () => {
  for (const failSecondRead of [false, true]) for (const receiptDamage of [null, { baseId: 42 }, { baseId: undefined }, { baseId: "" }, { path: "" }]) {
    const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
    f.host.restore = async request => { f.requests.push(structuredClone(request)); throw new Error("response lost"); };
    await assert.rejects(f.controller.restore("retain"), /response lost/);
    let reads = 0;
    f.host.status = async request => {
      if (++reads === 1) return { ...request, status: "missing" };
      if (failSecondRead && reads === 2) throw new Error("typed result read failed");
      return committed(saved(f.requests[0]));
    };
    f.host.settle = async request => { assert.equal(request.operationId, "restore4"); assert.match(request.wireDigest, /^[a-f0-9]{64}$/); return { ...saved(f.requests[0]).receipt, ...receiptDamage, status: "committed" } as HistorySettlement; };
    if (failSecondRead) {
      await assert.rejects(f.controller.inspectOriginal(), /typed result read failed/);
      assert.equal(f.controller.operationState, "committed"); assert.equal(f.controller.uncertain, false);
      assert.equal(f.controller.originalReceipt?.baseId, receiptDamage ? undefined : "base4"); assert.equal(f.controller.operationId, "restore4");
      assert.equal((await f.controller.inspectOriginal())?.source, old.source);
    } else assert.equal((await f.controller.inspectOriginal())?.source, old.source);
    assert.equal(f.requests.length, 1); assert.equal(f.controller.operationId, null);
  }
});

test("an invalid first receipt cannot poison committed recovery, and later bad receipts cannot replace a valid one", async () => {
  const damages = [{ operationId: "other" }, { baseId: "" }, { status: "recoveryPending" }, { baseId: 42 }];
  for (const startWithValid of [false, true]) for (const damage of damages) {
    const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
    f.host.restore = async request => {
      f.requests.push(structuredClone(request));
      const result = saved(request);
      return { operationId: request.operationId, status: "committed", receipt: (startWithValid ? result.receipt : { ...result.receipt, ...damage }) as typeof result.receipt };
    };
    await assert.rejects(f.controller.restore("retain"));
    assert.equal(f.controller.operationId, "restore4"); assert.equal(f.controller.operationState, "committed"); assert.equal(f.controller.uncertain, false);
    assert.deepEqual(f.controller.originalReceipt, startWithValid ? saved(f.requests[0]).receipt : null);
    if (startWithValid) {
      f.host.status = async () => ({ ...committed(saved(f.requests[0])), receipt: { ...saved(f.requests[0]).receipt, ...damage } as HistoryRestoreResult["receipt"] });
      await assert.rejects(f.controller.inspectOriginal(), { code: "resultUnknown" });
      assert.deepEqual(f.controller.originalReceipt, saved(f.requests[0]).receipt);
    }
    f.host.status = async () => committed(saved(f.requests[0]));
    assert.equal((await f.controller.inspectOriginal())?.source, old.source);
    assert.equal(f.controller.operationId, null); assert.equal(f.requests.length, 1);
  }
  const f = fixture(); await f.controller.select("save1"); await f.controller.prepareRestore();
  f.host.restore = async request => { f.requests.push(structuredClone(request)); return { operationId: request.operationId, status: "recoveryPending", receipt: { ...saved(request).receipt, baseId: "unproven pending base" } }; };
  await assert.rejects(f.controller.restore("retain")); assert.equal(f.controller.originalReceipt, null);
  f.host.status = async () => committed(saved(f.requests[0]));
  assert.equal((await f.controller.inspectOriginal())?.source, old.source); assert.equal(f.requests.length, 1);
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
