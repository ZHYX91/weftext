import test from "node:test";
import assert from "node:assert/strict";
import { LibraryTrashRestoreFlow, trashRestoreDraft } from "./library-trash";
import type { LibraryTrashHost, RestoreRequest } from "./library-trash";
import type { OpenedDocument } from "./types";

const document: OpenedDocument = { sessionId: "restored", path: "D:/notes/A/A.typ", root: "D:/notes", source: "恢复正文", baseId: "r", kind: "node" };
const entry = { operationId: "trash-original", name: "A" };
const plan = { operationId: "restore-once", planId: "frozen", kind: "restoreNode", status: "prepared", changes: [{ path: document.path, action: "create", bytes: 12 }] };
function harness(request: LibraryTrashHost["request"], created: LibraryTrashHost["created"] = async () => {}): LibraryTrashHost { return { request, created, notify: () => {} }; }

test("preparation does not execute; confirmation commits only the exact frozen IDs", async () => {
  const calls: { route: string; body: unknown }[] = [];
  const host = harness(async <T>(route: string, body?: unknown): Promise<T> => { calls.push({ route, body }); return (route.endsWith("prepare") ? plan : { ...plan, status: "committed", document }) as T; });
  const flow = new LibraryTrashRestoreFlow(host, () => "restore-once");
  await flow.prepare("D:/notes", entry, "D:/notes", "A");
  assert.equal(flow.pending?.confirmed, false);
  assert.deepEqual(calls.map(call => call.route), ["/api/operations/prepare"]);
  await flow.confirmAndCommit();
  assert.deepEqual(calls[1], { route: "/api/operations/commit", body: { operationId: "restore-once", planId: "frozen" } });
  assert.equal(flow.pending?.confirmed, true);
  assert.equal(flow.pending?.delivered, true);
});

test("unknown commit retains original request, plan and confirmation; reopen reconciliation never resubmits", async () => {
  const calls: string[] = []; let nextId = 0, deliveries = 0;
  const host = harness(async <T>(route: string): Promise<T> => {
    calls.push(route);
    if (route.endsWith("prepare")) return plan as T;
    if (route.endsWith("commit")) throw new Error("网络断开");
    return { ...plan, status: "committed", document } as T;
  }, async opened => { assert.equal(opened, document); deliveries++; });
  const flow = new LibraryTrashRestoreFlow(host, () => { nextId++; return "restore-once"; });
  await flow.prepare("D:/notes", entry, "D:/notes", "A");
  const original = flow.pending!.request;
  await assert.rejects(flow.confirmAndCommit());
  assert.equal(flow.pending?.state, "unknown"); assert.equal(flow.pending?.confirmed, true); assert.equal(flow.pending?.plan, plan);
  await assert.rejects(flow.prepare("D:/notes", entry, "D:/other", "B"));
  assert.equal(flow.pending?.request, original); assert.equal(nextId, 1);
  await flow.reconcile(); await flow.reconcile();
  assert.equal(calls.filter(route => route.endsWith("commit")).length, 1);
  assert.equal(calls.filter(route => route.endsWith("prepare")).length, 1);
  assert.equal(deliveries, 1);
  assert.equal(flow.pending?.delivered, true);
});

test("unknown preparation recovers same preview and still needs explicit confirmation", async () => {
  const calls: string[] = [];
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => {
    calls.push(route); if (route.endsWith("prepare")) throw new Error("准备响应丢失"); return plan as T;
  }), () => "restore-once");
  await assert.rejects(flow.prepare("D:/notes", entry, "D:/notes", "A"));
  assert.equal(flow.pending?.confirmed, false); assert.equal(flow.pending?.state, "unknown");
  await flow.reconcile();
  assert.equal(flow.pending?.state, "prepared"); assert.equal(flow.pending?.confirmed, false);
  assert.equal(calls.includes("/api/operations/commit"), false);
  flow.cancelPrepared(); assert.equal(flow.pending, null);
});

test("an unavailable original status keeps the restore blocked and never allocates a second ID", async () => {
  let ids = 0; const requests: RestoreRequest[] = [];
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string, body?: unknown): Promise<T> => {
    if (route.endsWith("prepare")) { requests.push(body as RestoreRequest); throw new Error("丢失响应"); }
    return { operationId: "restore-once", status: "unknown" } as T;
  }), () => { ids++; return "restore-once"; });
  await assert.rejects(flow.prepare("D:/notes", entry, "D:/notes", "A"));
  await flow.reconcile(); await flow.reconcile();
  await assert.rejects(flow.prepare("D:/notes", entry, "D:/notes", "A"));
  assert.equal(ids, 1); assert.equal(requests.length, 1); assert.equal(flow.pending?.state, "unknown");
});

test("a confirmed operation still prepared after status read never executes automatically", async () => {
  let commits = 0;
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => {
    if (route.endsWith("commit")) { commits++; throw new Error("未知提交"); } return plan as T;
  }), () => "restore-once");
  await flow.prepare("D:/notes", entry, "D:/notes", "A"); await assert.rejects(flow.confirmAndCommit());
  await flow.reconcile(); assert.equal(flow.pending?.state, "prepared"); assert.equal(flow.pending?.confirmed, true); assert.equal(commits, 1);
});

test("failed opening preserves successful restore and retries delivery without another commit", async () => {
  let deliveries = 0, commits = 0;
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => {
    if (route.endsWith("commit")) commits++; return (route.endsWith("prepare") ? plan : { ...plan, status: "committed", document }) as T;
  }, async () => { if (++deliveries === 1) throw new Error("编辑器仍在输入"); }), () => "restore-once");
  await flow.prepare("D:/notes", entry, "D:/notes", "A"); await assert.rejects(flow.confirmAndCommit());
  assert.equal(flow.pending?.state, "committed"); assert.equal(flow.pending?.delivered, false);
  await flow.deliverCommitted(); assert.equal(flow.pending?.delivered, true); assert.equal(commits, 1); assert.equal(deliveries, 2);
});

test("an unrelated status receipt cannot replace the original operation", async () => {
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => (route.endsWith("prepare") ? plan : { ...plan, operationId: "unrelated", status: "committed", document }) as T), () => "restore-once");
  await flow.prepare("D:/notes", entry, "D:/notes", "A");
  await assert.rejects(flow.reconcile()); assert.equal(flow.pending?.state, "unknown"); assert.equal(flow.pending?.request.operationId, "restore-once");
});

test("recoveryPending retains the confirmed original restore and only permits status inspection", async () => {
  let commits = 0;
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => {
    if (route.endsWith("prepare")) return plan as T;
    if (route.endsWith("commit")) { commits++; throw Object.assign(new Error("仍待文件恢复"), { code: "recoveryPending" }); }
    return { ...plan, status: "recoveryPending" } as T;
  }), () => "restore-once");
  await flow.prepare("D:/notes", entry, "D:/notes", "A"); await assert.rejects(flow.confirmAndCommit()); await flow.reconcile();
  assert.equal(flow.pending?.confirmed, true); assert.equal(flow.pending?.state, "unknown"); assert.equal(commits, 1);
  await assert.rejects(flow.prepare("D:/notes", entry, "D:/notes", "A"));
});

test("two roots from one retired operation keep independent restore drafts across refresh and reorder", () => {
  const entries = [
    { operationId: "clone-retired", nodeId: "root-a", name: "Same", path: "trash/A" },
    { operationId: "clone-retired", nodeId: "root-b", name: "Same", path: "trash/B" },
  ];
  const drafts = new Map<string, { name: string; parent: string }>();
  const a = trashRestoreDraft(drafts, "D:/notes", entries[0]);
  a.name = "Restore first"; a.parent = "D:/notes/First destination";
  const b = trashRestoreDraft(drafts, "D:/notes", entries[1]);
  assert.deepEqual(b, { name: "Same", parent: "D:/notes" });
  b.name = "Restore second"; b.parent = "D:/notes/Second destination";
  // New list objects and changed presentation cannot reassign either draft.
  const refreshed = [...entries].reverse().map(value => ({ ...value, name: "Renamed in list", path: "another display path" }));
  assert.deepEqual(refreshed.map(value => trashRestoreDraft(drafts, "D:/notes", value)), [
    { name: "Restore second", parent: "D:/notes/Second destination" },
    { name: "Restore first", parent: "D:/notes/First destination" },
  ]);
  assert.deepEqual(trashRestoreDraft(drafts, "D:/other", entries[0]), { name: "Same", parent: "D:/other" });
});

test("each root of the same retired operation prepares only its selected identity and independent destination", async () => {
  const entries = [
    { operationId: "clone-retired", nodeId: "root-a", name: "Same" },
    { operationId: "clone-retired", nodeId: "root-b", name: "Same" },
  ];
  const requests: RestoreRequest[] = [];
  let ids = 0;
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string, body?: unknown): Promise<T> => {
    assert.equal(route, "/api/operations/prepare");
    const request = body as RestoreRequest; requests.push(request);
    return { ...plan, operationId: request.operationId, planId: `plan-${request.operationId}` } as T;
  }), () => `restore-${++ids}`);
  await flow.prepare("D:/notes", entries[1], "D:/notes/B", "Second");
  flow.cancelPrepared();
  await flow.prepare("D:/notes", entries[0], "D:/notes/A", "First");
  assert.deepEqual(requests, [
    { operationId: "restore-1", root: "D:/notes", intent: { kind: "restoreNode", trashOperationId: "clone-retired", nodeId: "root-b", parent: "D:/notes/B", name: "Second" } },
    { operationId: "restore-2", root: "D:/notes", intent: { kind: "restoreNode", trashOperationId: "clone-retired", nodeId: "root-a", parent: "D:/notes/A", name: "First" } },
  ]);
});

test("unknown multi-root restore retains its exact selected root through status and explicit same-plan retry", async () => {
  const selected = { operationId: "clone-retired", nodeId: "root-b", name: "Same" };
  const requests: { route: string; body: unknown }[] = [];
  let ids = 0, commits = 0, delivered = 0;
  const restored = { ...document, nodeId: "root-b" };
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string, body?: unknown): Promise<T> => {
    requests.push({ route, body });
    if (route.endsWith("prepare")) return plan as T;
    if (route.endsWith("commit")) {
      if (++commits === 1) throw new Error("原提交响应丢失");
      return { ...plan, status: "committed", document: restored } as T;
    }
    return plan as T;
  }, async opened => { assert.equal(opened.nodeId, "root-b"); delivered++; }), () => { ids++; return "restore-once"; });
  await flow.prepare("D:/notes", selected, "D:/notes/B", "Second");
  const original = flow.pending!.request;
  selected.nodeId = "root-a"; selected.name = "Fresh list changed";
  await assert.rejects(flow.confirmAndCommit());
  await assert.rejects(flow.prepare("D:/notes", selected, "D:/notes/A", "First"));
  await flow.reconcile();
  assert.equal(commits, 1); assert.equal(flow.pending?.confirmed, true);
  assert.equal(flow.pending?.request, original); assert.equal(original.intent.nodeId, "root-b");
  assert.equal(Object.isFrozen(original.intent), true);
  await flow.confirmAndCommit();
  assert.equal(ids, 1); assert.equal(commits, 2); assert.equal(delivered, 1);
  assert.deepEqual(requests.filter(call => call.route.endsWith("commit")).map(call => call.body), [
    { operationId: "restore-once", planId: "frozen" }, { operationId: "restore-once", planId: "frozen" },
  ]);
  assert.equal(requests.filter(call => call.route.endsWith("prepare")).length, 1);
});

test("a successful receipt for another root is retained for inspection and never opens that root", async () => {
  let deliveries = 0, commits = 0;
  const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string): Promise<T> => {
    if (route.endsWith("prepare")) return plan as T;
    if (route.endsWith("commit")) { commits++; return { ...plan, status: "committed", document: { ...document, nodeId: "root-a" } } as T; }
    return { ...plan, status: "committed", document: { ...document, nodeId: "root-b" } } as T;
  }, async opened => { assert.equal(opened.nodeId, "root-b"); deliveries++; }), () => "restore-once");
  await flow.prepare("D:/notes", { operationId: "clone-retired", nodeId: "root-b", name: "Same" }, "D:/notes/B", "Second");
  await flow.confirmAndCommit();
  assert.equal(flow.pending?.state, "committed"); assert.equal(flow.pending?.delivered, false); assert.equal(deliveries, 0);
  await flow.reconcile();
  assert.equal(flow.pending?.delivered, true); assert.equal(deliveries, 1); assert.equal(commits, 1);
});

test("an ambiguous or mismatched retired selection is rejected without guessing another root", async () => {
  for (const code of ["trashSelectionRequired", "trashSelectionMismatch"]) {
    const requests: RestoreRequest[] = []; let ids = 0;
    const flow = new LibraryTrashRestoreFlow(harness(async <T>(route: string, body?: unknown): Promise<T> => {
      assert.equal(route, "/api/operations/prepare");
      const request = body as RestoreRequest; requests.push(request);
      if (requests.length === 1) throw Object.assign(new Error("请选择准确回收根"), { code });
      return { ...plan, operationId: request.operationId, planId: `plan-${request.operationId}` } as T;
    }), () => `restore-${++ids}`);
    const original = { operationId: "clone-retired", name: "Same", ...(code === "trashSelectionMismatch" ? { nodeId: "another-root" } : {}) };
    await assert.rejects(flow.prepare("D:/notes", original, "D:/notes", "Same"));
    assert.equal(flow.pending?.state, "failed"); assert.equal(requests.length, 1);
    assert.equal(flow.pending?.request.intent.nodeId, original.nodeId);
    if (code === "trashSelectionRequired") assert.equal(Object.hasOwn(requests[0].intent, "nodeId"), false);
    await flow.prepare("D:/notes", { operationId: "clone-retired", nodeId: "root-b", name: "Same" }, "D:/notes/B", "Selected");
    assert.equal(requests.length, 2); assert.equal(requests[1].intent.nodeId, "root-b");
  }
});
