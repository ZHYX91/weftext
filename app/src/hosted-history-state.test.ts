import test from "node:test";
import assert from "node:assert/strict";
import { currentHostedHistoryResult, hostedHistoryPending, hostedHistoryStatus, resumeHostedHistoryRecord, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, historySourceDiff } from "./history-state";
import { createSourceState } from "./editor";
import { history, isolateHistory, undo, undoDepth } from "@codemirror/commands";

const record: HostedHistoryRecord = {
  request: { resource: "owner", sessionId: "session", baseId: "baseC", versionId: "versionA", operationId: "restore", source: "C😀\r\n", serial: 3, sourceHash: "hashC", draftAction: "retain" },
  basis: { resource: "owner", path: "resource:owner", root: "", nodeId: "node", workspaceId: "workspace", sessionId: "session", baseId: "baseC", baseSource: "C😀\r\n", source: "C😀\r\n", serial: 3, dirty: false, selectedVersionId: "versionA", selectedSource: "A\r\n" },
};
const committed: HostedHistoryStatus = { resource: "owner", operationId: "restore", status: "committed", resultStatus: "available", resultBasis: "current", currentRevision: "original-revision", result: { receipt: { operationId: "restore", status: "saved", path: "resource:owner", baseId: "baseA", revisionId: "original-revision", message: "" }, source: "A\r\n", versionId: "versionA", protectedDrafts: [{ path: "resource:owner", source: "D😀\r\n", baseSource: "C😀\r\n", baseKnown: true }] } };

test("a restore receipt proves the original revision as well as bytes; superseded/no-op ABA and external equal bytes cannot bind current", () => {
  assert.equal(currentHostedHistoryResult(record, "A\r\n", committed), true);
  assert.equal(currentHostedHistoryResult(record, "F\r\n", committed), false);
  for (const changed of [{ currentRevision: "later-equal-byte-revision" }, { resultBasis: "superseded" }, { resultBasis: "external", currentRevision: null }]) assert.equal(currentHostedHistoryResult(record, "A\r\n", { ...committed, ...changed } as HostedHistoryStatus), false);
  assert.throws(() => hostedHistoryStatus(record, { ...committed, operationId: "other" }));
  const bad = hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, receipt: { ...committed.result!.receipt, path: "D:/private/server/owner.typ" } } });
  assert.equal(bad.status, "committed"); assert.equal(bad.result, undefined); assert.ok(bad.resultError);
});
test("hosted committed facts survive malformed results, downgrade packets and reconnect without reviving cached delivery rights", () => {
  const active = structuredClone(record);
  for (const change of [{ operationId: "other" }, { baseId: "" }, { baseId: 42 }, { path: "D:/private/secret.typ" }, { status: "resolved" }, { message: null }, { revisionId: undefined }, { revisionId: "" }, { revisionId: 42 }]) {
    const incoming = structuredClone(committed); Object.assign(incoming.result!.receipt, change);
    active.status = hostedHistoryStatus(active, incoming);
    assert.equal(active.status.status, "committed"); assert.equal(active.status.receipt, undefined);
    assert.equal(active.status.result, undefined); assert.ok(active.status.resultError); assert.equal(hostedHistoryPending(active), true);
    const recovered = hostedHistoryStatus(active, committed);
    assert.equal(recovered.resultStatus, "available"); assert.equal(recovered.receipt?.revisionId, "original-revision");
  }
  active.status = hostedHistoryStatus(active, committed);
  const original = structuredClone(active.status.receipt);
  for (const status of ["notRegistered", "notCommitted", "recoveryPending", "resolved", "cancelled"] as const) {
    active.status = hostedHistoryStatus(active, { operationId: "restore", resource: "owner", status, resultStatus: "unavailable" });
    assert.equal(active.status.status, "committed"); assert.deepEqual(active.status.receipt, original);
    assert.equal(active.status.result, undefined); assert.equal(active.status.resultBasis, undefined);
    assert.equal(active.status.currentRevision, undefined); assert.equal(hostedHistoryPending(active), true);
  }
  active.status = hostedHistoryStatus(active, committed);
  const resumed = resumeHostedHistoryRecord(active);
  assert.equal(resumed.status?.status, "committed"); assert.deepEqual(resumed.status?.receipt, original);
  assert.equal(resumed.status?.result, undefined); assert.equal(resumed.status?.resultBasis, undefined);
  resumed.status = hostedHistoryStatus(resumed, { ...committed, resource: "other" });
  assert.equal(hostedHistoryPending(resumed), true); assert.equal(resumed.status.result, undefined);
  resumed.status = hostedHistoryStatus(resumed, committed);
  assert.equal(currentHostedHistoryResult(resumed, "A\r\n", resumed.status), true);
  assert.deepEqual(resumed.request, record.request); assert.deepEqual(resumed.basis, record.basis);
});
test("committed unavailable drops earlier source and has an explicit comparison-only exit; unknown operations cannot use it", () => {
  const unavailable = hostedHistoryStatus({ ...record, status: committed }, { resource: "owner", operationId: "restore", status: "committed", resultStatus: "unavailable" });
  assert.equal(unavailable.result, undefined); assert.equal(unavailableCommittedHistory({ ...record, status: unavailable }), true);
  assert.equal(hostedHistoryPending({ ...record, status: unavailable }), true);
  assert.equal(hostedHistoryPending({ ...record, status: unavailable, acknowledged: true }), false);
  for (const status of ["notRegistered", "notCommitted", "recoveryPending", "resolved", "cancelled"] as const) assert.equal(unavailableCommittedHistory({ ...record, status: { ...unavailable, status } }), false);
  assert.equal(hostedHistoryPending({ ...record, status: { ...unavailable, status: "resolved" } }), false);
  assert.equal(hostedHistoryPending({ ...record, status: { ...unavailable, status: "cancelled" } }), false);
});
test("clean restore is one real shared Undo; dirty and late input remain author drafts and fixed comparisons reconstruct both originals", () => {
  assert.equal(restoreBufferDisposition(record.basis, record.basis), "replaceClean");
  assert.equal(restoreBufferDisposition({ ...record.basis, source: "D", dirty: true }, { ...record.basis, source: "E", serial: 4, dirty: true }), "retainDraft");
  let state = createSourceState(record.basis.source, [history()]);
  state = state.update({ changes: { from: 0, to: state.doc.length, insert: committed.result!.source }, userEvent: "input.history.restore", annotations: isolateHistory.of("full") }).state;
  assert.equal(undoDepth(state), 1); const target = { get state() { return state; }, dispatch(transaction: { state: typeof state }) { state = transaction.state; } }; assert.equal(undo(target), true); assert.equal(state.doc.toString(), record.basis.source);
  const parts = historySourceDiff("A\r\n", "D😀\r\n"); assert.equal(parts.filter(part => part.kind !== "added").map(part => part.text).join(""), "A\r\n"); assert.equal(parts.filter(part => part.kind !== "removed").map(part => part.text).join(""), "D😀\r\n");
});
