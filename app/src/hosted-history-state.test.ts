import test from "node:test";
import assert from "node:assert/strict";
import { currentHostedHistoryResult, hostedHistoryPending, hostedHistoryStatus, resumeHostedHistoryRecord, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, historySourceDiff, historyRecoveryIssueUiText } from "./history-state";
import { apiErrorUiText, decodeApiErrorBody } from "./api-error";
import { locale, message, uiTextValue } from "./ui-locale";
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
  const bad = hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, receipt: { ...committed.result!.receipt, path: "D:/private/server/owner.typ" } } });
  assert.equal(bad.status, "committed"); assert.equal(bad.result, undefined); assert.ok(bad.guardReason); assert.equal(bad.resultError, undefined);
});
test("hosted committed facts survive malformed results, downgrade packets and reconnect without reviving cached delivery rights", () => {
  const active = structuredClone(record);
  for (const change of [{ operationId: "other" }, { baseId: "" }, { baseId: 42 }, { path: "D:/private/secret.typ" }, { status: "resolved" }, { message: null }, { revisionId: undefined }, { revisionId: "" }, { revisionId: 42 }]) {
    const incoming = structuredClone(committed); Object.assign(incoming.result!.receipt, change);
    active.status = hostedHistoryStatus(active, incoming);
    assert.equal(active.status.status, "committed"); assert.equal(active.status.receipt, undefined);
    assert.equal(active.status.result, undefined); assert.ok(active.status.guardReason); assert.equal(active.status.resultError, undefined); assert.equal(hostedHistoryPending(active), true);
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

test("a real scoped delivery ApiError retains its code, causes and nested Core facts after JSON while client guards remain separate", () => {
  const rawDescription = { explanation: message("common.close"), diagnostics: [{ origin: "dependency", text: "original raw 错误😀\r\n" }], causes: [] };
  const rawError = { code: "deliveryFailed", explanation: message("common.save"), diagnostics: rawDescription.diagnostics, causes: [{code:"originalCause",explanation:rawDescription.explanation,diagnostics:rawDescription.diagnostics}], details: { kind: "core", code: "coverage", ...rawDescription, details: { kind: "aggregate", groups: [{ kind: "product", code: "exactOriginal", diagnostics: [rawDescription] }] } } };
  const error = decodeApiErrorBody(rawError)!; assert.ok(error);
  const prior = { ...record, status: hostedHistoryStatus(record, committed) }, incoming = JSON.parse(JSON.stringify({ operationId: "restore", resource: "owner", status: "committed", resultStatus: "unavailable", resultError: rawError }));
  const status = hostedHistoryStatus(prior, incoming);
  assert.equal(status.status, "committed"); assert.equal(status.result, undefined); assert.equal(status.guardReason, undefined); assert.equal(status.resultError!.code, "deliveryFailed");
  assert.deepEqual(status.resultError!.details, error.details); assert.deepEqual(status.resultError!.causes, error.causes); assert.equal(status.receipt!.revisionId, "original-revision");
  locale.set("en-US"); assert.match(uiTextValue(apiErrorUiText(status.resultError!)), /Save/); assert.match(uiTextValue(apiErrorUiText(status.resultError!)), /original raw 错误😀\r\n/); locale.set("zh-CN");
  const invalid = hostedHistoryStatus(prior, { ...incoming, resultError: "old untyped error" });
  assert.equal(invalid.status, "committed"); assert.ok(invalid.guardReason); assert.equal(invalid.resultError, undefined); assert.equal(invalid.result, undefined);
  const resumed = resumeHostedHistoryRecord({ ...prior, status }); assert.equal(resumed.status!.resultError, undefined); assert.equal(resumed.status!.result, undefined); assert.deepEqual(resumed.request, record.request);
});

test("History recovery warnings use the CoreError domain including its aggregate details and exact current source", () => {
  const authored = '\uFEFF作者😀\r\n{"key":"common.save","params":{}}', description = { explanation: message("common.close"), diagnostics: [{ origin: "dependency" as const, text: "raw tool 原文\r\n" }], causes: [] };
  const core = { code: "draftProtection", ...description, currentSource: authored, details: { kind: "aggregate" as const, groups: [{ kind: "product" as const, code: "originalFacts", diagnostics: [description] }] } };
  const displayed = historyRecoveryIssueUiText(JSON.parse(JSON.stringify(core)));
  locale.set("en-US"); assert.match(uiTextValue(displayed), /Close/); assert.ok(uiTextValue(displayed).includes(authored)); assert.match(uiTextValue(displayed), /originalFacts/); locale.set("zh-CN");
  assert.equal(core.currentSource, authored);
  const safeIssue = { code: "draftProtection", explanation: message("common.close"), diagnostics: [], causes: [] };
  const available = hostedHistoryStatus(record, JSON.parse(JSON.stringify({ ...committed, result: { ...committed.result!, recoveryIssue: safeIssue } })));
  assert.equal(available.resultStatus, "available"); assert.equal(uiTextValue(historyRecoveryIssueUiText(available.result!.recoveryIssue!)), "关闭");
  for (const issue of [{ code: "old", message: "old warning" }, { ...safeIssue, details: { kind: "currentSource", currentSource: authored } }]) {
    const invalid = hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, recoveryIssue: issue } } as unknown as HostedHistoryStatus);
    assert.equal(invalid.status, "committed"); assert.equal(invalid.result, undefined); assert.ok(invalid.guardReason);
  }
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
