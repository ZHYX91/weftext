import { test } from "node:test";
import assert from "node:assert/strict";
import { SyncFlow, decodeRemoteSyncProgress, decodeSyncAdmission, decodeSyncPending, decodeSyncPlan, freezeSyncAdoptionRequest, sameSyncIdentity, type SyncAdoptionCut, type SyncPending, type SyncRequest, type SyncPlan } from "./sync-state";
import { locale, message, uiTextValue } from "./ui-locale";
import { apiErrorUiText } from "./api-error";
import { coreDescriptionUiText, descriptionUiText } from "./ui-diagnostics";
import { knowledgeLossMessage } from "./knowledge-state";
const request: SyncRequest = { operationId: "original", root: "R", intent: { kind: "applySync", identity: { workspaceId: "W", genesis: "G" }, objects: [{ kind: "node", id: "N" }] } };
const plan: SyncPlan = { operationId: "original", planId: "frozen", kind: "applySync", status: "prepared", changes: [], nodeMap: {}, newNodeId: null, revisions: {}, resourceMap: {}, losses: [] };
const description = () => ({ explanation: message("common.save"), diagnostics: [{ origin: "dependency" as const, text: "original raw 原文😀\r\n<&>" }], causes: [{ code: "originalCause", explanation: message("common.close"), diagnostics: [] }] });
const apiFailure = () => ({ code: "transferFailed", ...description(), details: { kind: "core", code: "originalFacts", ...description(), details: { kind: "aggregate", groups: [{ kind: "product", code: "originalFacts", diagnostics: Array.from({ length: 40 }, description) }] } } });
test("pairing compares identity fields, independently of wire member order", () => {
  assert.ok(sameSyncIdentity({ workspaceId: "W", genesis: "G" }, JSON.parse('{"genesis":"G","workspaceId":"W"}')));
  assert.equal(sameSyncIdentity({ workspaceId: "W", genesis: "G" }, { workspaceId: "W", genesis: "other" }), false);
});
test("lost execute response survives reload; inspection never executes again", async () => {
  let stored: SyncPending | null = null; let calls: string[] = [];
  const host = { store: (p: SyncPending | null) => { stored = structuredClone(p); }, request: async <T>(path: string): Promise<T> => {
    calls.push(path); assert.equal(stored?.request.operationId, "original");
    if (path.endsWith("prepare")) return plan as T;
    if (path.endsWith("commit")) { assert.equal(stored?.attempted, true); throw new Error("response lost"); }
    return { operationId: plan.operationId, planId: plan.planId, kind: plan.kind, status: "committed", path: "R", nodeIds: [] } as T;
  } };
  const first = new SyncFlow(host); await first.prepare(request); await assert.rejects(first.commit());
  assert.throws(() => first.forget());
  const reloaded = new SyncFlow(host, stored); await reloaded.inspect();
  assert.equal(reloaded.pending?.status, "committed"); reloaded.forget();
  assert.equal(calls.filter(p => p.endsWith("commit")).length, 1); assert.equal(stored, null);
});
test("lost prepare retries immutable original input; stale or unrelated receipt is retained", async () => {
  let attempt = 0; const bodies: unknown[] = [];
  const flow = new SyncFlow({ store: () => {}, request: async <T>(_path: string, body: unknown): Promise<T> => {
    bodies.push(structuredClone(body)); if (!attempt++) throw new Error("lost"); return plan as T;
  } });
  const mutable = structuredClone(request); await assert.rejects(flow.prepare(mutable)); mutable.root = "OTHER";
  await flow.retryPrepare(); assert.deepEqual(bodies[0], bodies[1]);
  const bad = new SyncFlow({ store: () => {}, request: async <T>() => ({ operationId: plan.operationId, planId: "changed", kind: plan.kind, status: "committed", path: "R", nodeIds: [] }) as T }, flow.pending);
  await assert.rejects(bad.inspect(), /原准备/); assert.equal(bad.pending?.status, "prepared");
});
test("storage failure stops effects, failed preparation can be dismissed, unresolved execution cannot", async () => {
  let calls = 0;
  const flow = new SyncFlow({ store: () => { throw new Error("quota"); }, request: async <T>() => { calls++; return plan as T; } });
  await assert.rejects(flow.prepare(request), /quota/); assert.equal(calls, 0);
  const unknown = new SyncFlow({ store: () => {}, request: async <T>() => ({ operationId: "original", status: "unknown" }) as T }, { request, attempted: true, status: "unknown", plan });
  await assert.rejects(unknown.commit()); await assert.rejects(unknown.retryPrepare()); await assert.rejects(unknown.cancelPrepared()); assert.throws(() => unknown.forget());
  const failed = new SyncFlow({ store: () => {}, request: async () => { throw new Error("invalid"); } });
  await assert.rejects(failed.prepare(request)); failed.forget(); assert.equal(failed.pending, null);
});
test("known prepared original can be explicitly resolved; unknown cannot", async () => {
  let body: unknown;
  const flow = new SyncFlow({ store: () => {}, request: async <T>(_path: string, b: unknown) => { body = b; return { operationId: plan.operationId, planId: plan.planId, status: "resolved", path: "R", nodeIds: [] } as T; } }, { request, attempted: true, status: "prepared", plan });
  await flow.cancelPrepared(); assert.deepEqual(body, { operationId: "original", planId: "frozen" }); flow.forget();
});

test("original-ID lookup restores the exact retained proposal without preparation or execution", async () => {
  const request: SyncRequest = { operationId: "old", root: "original-root", intent: { kind: "cloneSync", identity: { workspaceId: "w", genesis: "g" }, objects: [{ kind: "node", id: "n" }] } };
  const plan: SyncPlan = { operationId: "old", planId: "p", kind: "cloneSync", status: "committed", changes: [], nodeMap: {}, newNodeId: null, revisions: {}, resourceMap: {}, losses: [] };
  let stored: SyncPending | null = null;
  const flow = new SyncFlow({ store: p => { stored = structuredClone(p); }, request: async <T>(route: string, body: unknown) => {
    assert.equal(route, "/api/sync/operation"); assert.deepEqual(body, { operationId: "old" }); return { request, plan } as T;
  } });
  await flow.recover("old"); assert.equal(flow.pending?.request.root, "original-root"); assert.deepEqual(stored, flow.pending);
  await assert.rejects(flow.recover("other")); await assert.rejects(flow.commit()); flow.forget();
});

test("adoption freezes explicit missing members separately from present empty-file bytes and checks the original pairing", () => {
  const identity = { workspaceId: "W", genesis: "G" }, cut: SyncAdoptionCut = { identity, expectedInstalled: [{ object: { kind: "node", id: "N" }, revisions: ["installed"] }],
    changes: [{ path: "R/N/empty.bin", baseId: "present-empty", before: [], after: [] }], removedMembers: [{ path: "R/N/removed.bin", baseId: "original-missing" }] };
  const frozen = freezeSyncAdoptionRequest("R", identity, cut, "adopt-original"), original = structuredClone(frozen);
  if (frozen.intent.kind !== "adoptSyncExternal") throw new Error("wrong intent");
  assert.deepEqual(frozen.intent.changes, cut.changes); assert.deepEqual(frozen.intent.removedMembers, cut.removedMembers);
  assert.equal("after" in frozen.intent.removedMembers![0], false); assert.equal(frozen.operationId, "adopt-original");
  cut.changes[0].baseId = "new-empty"; cut.changes[0].after.push(9); cut.removedMembers![0].path = "R/N/another.bin"; cut.removedMembers![0].baseId = "new-missing";
  cut.expectedInstalled[0].revisions.push("late-head"); identity.genesis = "new-genesis";
  assert.deepEqual(frozen, original);
  assert.throws(() => freezeSyncAdoptionRequest("R", { workspaceId: "W", genesis: "G" }, cut, "foreign"), /另一资料库/);
});

test("ordinary adoption without removed members retains its original wire shape", () => {
  const identity = { workspaceId: "W", genesis: "G" }, cut: SyncAdoptionCut = { identity, expectedInstalled: [], changes: [{ path: "R/N/N.typ", baseId: "observed", before: [65], after: [65] }] };
  const frozen = freezeSyncAdoptionRequest("R", identity, cut, "ordinary-adopt");
  assert.deepEqual(frozen, { operationId: "ordinary-adopt", root: "R", intent: { kind: "adoptSyncExternal", identity, expectedInstalled: [], changes: cut.changes } });
  assert.equal("removedMembers" in frozen.intent, false);
  cut.removedMembers = []; const explicitEmpty = freezeSyncAdoptionRequest("R", identity, cut, "empty-adopt");
  assert.equal(explicitEmpty.intent.kind, "adoptSyncExternal");
  if (explicitEmpty.intent.kind !== "adoptSyncExternal") throw new Error("wrong intent");
  assert.deepEqual(explicitEmpty.intent.removedMembers, []);
});

test("adoption missing-member bases survive lost prepare, JSON reload and unknown commit through the original operation", async () => {
  const identity = { workspaceId: "W", genesis: "G" }, cut: SyncAdoptionCut = { identity, expectedInstalled: [{ object: { kind: "node", id: "N" }, revisions: ["original-installed"] }],
    changes: [{ path: "R/N/weftext.resources.json", baseId: "observed-manifest", before: [123, 125], after: [123, 125] }], removedMembers: [{ path: "R/N/a.bin", baseId: "missing-a" }, { path: "R/N/b.bin", baseId: "missing-b" }] };
  const original = freezeSyncAdoptionRequest("R", identity, cut, "adopt-once"), plan: SyncPlan = { operationId: "adopt-once", planId: "original-plan", kind: "adoptSyncExternal", status: "prepared", changes: [], nodeMap: {}, newNodeId: null, revisions: {}, resourceMap: {}, losses: [] };
  let stored = "", prepares = 0, commits = 0; const calls: { path: string; body: unknown }[] = [];
  const host = {
    store: (pending: SyncPending | null): void => { stored = JSON.stringify(pending); },
    request: async <T>(path: string, body: unknown): Promise<T> => {
      calls.push({ path, body: structuredClone(body) });
      const retained = JSON.parse(stored) as SyncPending;
      if (path === "/api/operations/prepare") {
        assert.deepEqual(retained.request, original); if (++prepares === 1) throw new Error("prepare response lost");
        return plan as T;
      }
      if (path === "/api/operations/commit") { commits++; throw new Error("commit response lost"); }
      if (path === "/api/sync/operation") return { request: structuredClone(original), plan: { ...plan, status: "committed" } } as T;
      assert.equal(path, "/api/operations/status"); return { operationId: plan.operationId, planId: plan.planId, kind: plan.kind, status: "committed", path: "R", nodeIds: [] } as T;
    },
  };
  const first = new SyncFlow(host); await assert.rejects(first.prepare(original), /prepare response lost/);
  cut.removedMembers!.reverse(); cut.removedMembers![0].baseId = "later-missing"; cut.changes[0].after = [1];
  const reloaded = new SyncFlow(host, JSON.parse(stored) as SyncPending); await reloaded.retryPrepare();
  assert.deepEqual(calls[0], calls[1]); await assert.rejects(reloaded.commit(), /commit response lost/);
  assert.throws(() => reloaded.forget()); await assert.rejects(reloaded.retryPrepare());
  const unknown = new SyncFlow(host, JSON.parse(stored) as SyncPending); await unknown.inspect();
  assert.equal(unknown.pending?.status, "committed"); assert.deepEqual(unknown.pending?.request, original); assert.equal(commits, 1);
  unknown.forget(); const recovered = new SyncFlow(host); await recovered.recover("adopt-once");
  assert.deepEqual(recovered.pending?.request, original); assert.equal(prepares, 2); assert.equal(commits, 1);
  assert.deepEqual(calls.slice(2).map(call => call.body), [{ operationId: "adopt-once", planId: "original-plan" }, { operationId: "adopt-once" }, { operationId: "adopt-once" }]);
});

test("remote progress requires the actual machine status, all counters, identity and nullable complete ApiError", () => {
  const wire = { requestId: "transfer", identity: request.intent.identity, status: "failed", recordsDone: 2, recordsTotal: 3, bytesDone: 20, bytesTotal: 30, ...description(), error: apiFailure() };
  const decoded = decodeRemoteSyncProgress(JSON.parse(JSON.stringify(wire)));
  assert.equal(decoded.status, "failed"); assert.deepEqual(decoded.identity, request.intent.identity); assert.equal(decoded.error!.code, "transferFailed"); assert.deepEqual(decoded.error!.details, apiFailure().details); assert.equal(decoded.causes[0].code, "originalCause");
  locale.set("en-US"); assert.match(uiTextValue(descriptionUiText(decoded)), /Save/); assert.match(uiTextValue(apiErrorUiText(decoded.error!)), /original raw 原文😀\r\n<&>/); locale.set("zh-CN");
  for (const status of ["running", "complete", "failed", "stopped"]) assert.equal(decodeRemoteSyncProgress({ ...wire, status, error: null }).status, status);
  for (const change of [{ error: undefined }, { error: { code: "old", message: "old" } }, { identity: undefined }, { recordsDone: undefined }, { bytesDone: Number.MAX_SAFE_INTEGER + 1 }, { recordsTotal: -1 }, { status: "传输完成" }, { unknown: true }]) assert.throws(() => decodeRemoteSyncProgress({ ...wire, ...change }));
});

test("admission is a closed state union and rejected Core descriptions retain aggregate facts after JSON", () => {
  const reason = { ...description(), details: { kind: "aggregate", groups: [{ kind: "product", code: "exactEvidence", diagnostics: [description()] }] } };
  const decoded = decodeSyncAdmission(JSON.parse(JSON.stringify({ status: "rejected", reason })));
  assert.equal(decoded.status, "rejected"); if (decoded.status !== "rejected") throw new Error("wrong variant");
  locale.set("en-US"); assert.match(uiTextValue(coreDescriptionUiText(decoded.reason)), /Save/); assert.match(uiTextValue(coreDescriptionUiText(decoded.reason)), /exactEvidence/); locale.set("zh-CN");
  assert.deepEqual(decodeSyncAdmission({ status: "accepted" }), { status: "accepted" }); assert.deepEqual(decodeSyncAdmission({ status: "staged", missingParents: ["original-parent"], missingBlobs: ["original-blob"] }), { status: "staged", missingParents: ["original-parent"], missingBlobs: ["original-blob"] });
  for (const wire of [{ status: "accepted", reason }, { status: "staged", missingParents: [] }, { status: "rejected", reason: "old prose" }, { status: "拒绝", reason }]) assert.throws(() => decodeSyncAdmission(wire));
});

test("a JSON-restored original plan rebrands every loss and retains all maps without changing its author request", async () => {
  const loss = { path: '作者/原件😀.typ', code: "exactLoss", ...description(), productDiagnostics: Array.from({ length: 40 }, description) }, wire = { ...plan, nodeMap: { original: "node-original" }, newNodeId: "node-original", revisions: { original: "revision-original" }, resourceMap: { author: "resource-original" }, losses: [loss] };
  let retained = "";
  const host = { store: (value: SyncPending | null) => { retained = JSON.stringify(value); }, request: async <T>() => JSON.parse(JSON.stringify(wire)) as T };
  const first = new SyncFlow(host); await first.prepare(request); const original = retained, restored = new SyncFlow(host, JSON.parse(retained));
  locale.set("en-US"); assert.match(uiTextValue(knowledgeLossMessage(restored.pending!.plan!.losses[0])), /Save/); locale.set("zh-CN");
  assert.deepEqual(restored.pending!.request, request); assert.deepEqual(restored.pending!.plan!.resourceMap, wire.resourceMap); assert.equal(restored.pending!.plan!.newNodeId, "node-original"); assert.equal(restored.pending!.plan!.losses[0].productDiagnostics.length, 40); assert.equal(restored.pending!.plan!.losses[0].path, loss.path);
  const stale = JSON.parse(original); stale.plan.losses[0] = { message: "old prose" }; assert.throws(() => decodeSyncPending(stale)); assert.equal(retained, original);
  assert.throws(() => decodeSyncPlan({ ...wire, nodeMap: undefined })); assert.throws(() => decodeSyncPlan({ ...wire, losses: [{ ...loss, productDiagnostics: undefined }] }));
});

test("unknown commit keeps its frozen original until a real typed App receipt, and later status cannot undo that fact", async () => {
  let reply: unknown = { operationId: "original", planId: "frozen", status: "committed", path: "R", nodeIds: [], details: { receipt: "not a receipt authority" } };
  const writes: unknown[] = [], host = { store() {}, request: async <T>(route: string, body: unknown) => { writes.push(structuredClone(body)); return (route.endsWith("prepare") ? plan : reply) as T; } };
  const flow = new SyncFlow(host); await flow.prepare(request); await assert.rejects(flow.commit()); assert.equal(flow.pending!.status, "unknown"); assert.throws(() => flow.forget()); assert.deepEqual(flow.pending!.request, request);
  reply = { operationId: "original", planId: "frozen", kind: "applySync", status: "committed", path: "R", nodeIds: ["original-node"], documentError: apiFailure() };
  await flow.inspect(); assert.equal(flow.pending!.status, "committed"); assert.equal(flow.pending!.receipt!.documentError!.code, "transferFailed"); assert.deepEqual(flow.pending!.receipt!.nodeIds, ["original-node"]);
  const restored = decodeSyncPending(JSON.parse(JSON.stringify(flow.pending))); locale.set("en-US"); assert.match(uiTextValue(apiErrorUiText(restored.receipt!.documentError!)), /Save/); locale.set("zh-CN");
  assert.throws(() => decodeSyncPending({ ...restored, status: "prepared" }));
  reply = { operationId: "original", status: "unknown" }; await assert.rejects(flow.inspect()); assert.equal(flow.pending!.status, "committed"); assert.deepEqual(flow.pending!.request, request); assert.equal(writes.filter(value => (value as { planId?: string }).planId === "frozen").length, 1);
});
