import test from "node:test";
import assert from "node:assert/strict";
import { LibraryTrashRestoreFlow } from "./library-trash";
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
