import { test } from "node:test";
import assert from "node:assert/strict";
import { SyncFlow, type SyncPending, type SyncRequest, type SyncPlan } from "./sync-state";
const request: SyncRequest = { operationId: "original", root: "R", intent: { kind: "applySync", identity: { workspaceId: "W", genesis: "G" }, objects: [{ kind: "node", id: "N" }] } };
const plan: SyncPlan = { operationId: "original", planId: "frozen", status: "prepared", changes: [], losses: [] };
test("lost execute response survives reload; inspection never executes again", async () => {
  let stored: SyncPending | null = null; let calls: string[] = [];
  const host = { store: (p: SyncPending | null) => { stored = structuredClone(p); }, request: async <T>(path: string): Promise<T> => {
    calls.push(path); assert.equal(stored?.request.operationId, "original");
    if (path.endsWith("prepare")) return plan as T;
    if (path.endsWith("commit")) { assert.equal(stored?.attempted, true); throw new Error("response lost"); }
    return { ...plan, status: "committed" } as T;
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
  const bad = new SyncFlow({ store: () => {}, request: async <T>() => ({ ...plan, planId: "changed", status: "committed" }) as T }, flow.pending);
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
  const flow = new SyncFlow({ store: () => {}, request: async <T>(_path: string, b: unknown) => { body = b; return { ...plan, status: "resolved" } as T; } }, { request, attempted: true, status: "prepared", plan });
  await flow.cancelPrepared(); assert.deepEqual(body, { operationId: "original", planId: "frozen" }); flow.forget();
});

test("original-ID lookup restores the exact retained proposal without preparation or execution", async () => {
  const request: SyncRequest = { operationId: "old", root: "original-root", intent: { kind: "cloneSync", identity: { workspaceId: "w", genesis: "g" }, objects: [{ kind: "node", id: "n" }] } };
  const plan: SyncPlan = { operationId: "old", planId: "p", status: "committed", changes: [], losses: [] };
  let stored: SyncPending | null = null;
  const flow = new SyncFlow({ store: p => { stored = structuredClone(p); }, request: async <T>(route: string, body: unknown) => {
    assert.equal(route, "/api/sync/operation"); assert.deepEqual(body, { operationId: "old" }); return { request, plan } as T;
  } });
  await flow.recover("old"); assert.equal(flow.pending?.request.root, "original-root"); assert.deepEqual(stored, flow.pending);
  await assert.rejects(flow.recover("other")); await assert.rejects(flow.commit()); flow.forget();
});
