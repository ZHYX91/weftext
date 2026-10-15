import { test } from "node:test";
import assert from "node:assert/strict";
import { SyncFlow, freezeSyncAdoptionRequest, sameSyncIdentity, type SyncAdoptionCut, type SyncPending, type SyncRequest, type SyncPlan } from "./sync-state";
const request: SyncRequest = { operationId: "original", root: "R", intent: { kind: "applySync", identity: { workspaceId: "W", genesis: "G" }, objects: [{ kind: "node", id: "N" }] } };
const plan: SyncPlan = { operationId: "original", planId: "frozen", status: "prepared", changes: [], losses: [] };
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
  const original = freezeSyncAdoptionRequest("R", identity, cut, "adopt-once"), plan: SyncPlan = { operationId: "adopt-once", planId: "original-plan", status: "prepared", changes: [], losses: [] };
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
      assert.equal(path, "/api/operations/status"); return { ...plan, status: "committed" } as T;
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
