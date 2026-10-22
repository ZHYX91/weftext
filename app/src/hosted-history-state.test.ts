import test from "node:test";
import assert from "node:assert/strict";
import { currentHostedHistoryResult, hostedHistoryPending, hostedHistoryStatus, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, historySourceDiff } from "./history-state";
import { createSourceState } from "./editor";
import { history, isolateHistory, undo, undoDepth } from "@codemirror/commands";

const record: HostedHistoryRecord = {
  request: { resource: "owner", sessionId: "session", baseId: "baseC", versionId: "versionA", operationId: "restore", source: "C😀\r\n", serial: 3, sourceHash: "hashC", draftAction: "retain" },
  basis: { resource: "owner", path: "resource:owner", root: "", nodeId: "node", workspaceId: "workspace", sessionId: "session", baseId: "baseC", baseSource: "C😀\r\n", source: "C😀\r\n", serial: 3, dirty: false, selectedVersionId: "versionA", selectedSource: "A\r\n" },
};
const committed: HostedHistoryStatus = { resource: "owner", operationId: "restore", status: "committed", resultStatus: "available", resultBasis: "current", currentRevision: "original-revision", result: { receipt: { operationId: "restore", status: "saved", path: "resource:owner", baseId: "baseA", revisionId: "original-revision" }, source: "A\r\n", versionId: "versionA", protectedDrafts: [{ path: "resource:owner", source: "D😀\r\n", baseSource: "C😀\r\n", baseKnown: true }] } };

test("a restore receipt proves the original revision as well as bytes; superseded/no-op ABA and external equal bytes cannot bind current", () => {
  assert.equal(currentHostedHistoryResult(record, "A\r\n", committed), true);
  assert.equal(currentHostedHistoryResult(record, "F\r\n", committed), false);
  for (const changed of [{ currentRevision: "later-equal-byte-revision" }, { resultBasis: "superseded" }, { resultBasis: "external", currentRevision: null }]) assert.equal(currentHostedHistoryResult(record, "A\r\n", { ...committed, ...changed } as HostedHistoryStatus), false);
  assert.throws(() => hostedHistoryStatus(record, { ...committed, operationId: "other" }));
  assert.throws(() => hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, receipt: { ...committed.result!.receipt, path: "D:/private/server/owner.typ" } } }));
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
