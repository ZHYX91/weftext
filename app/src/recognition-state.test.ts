import test from "node:test";
import assert from "node:assert/strict";
import { RecognitionController, decodeRecognitionLocator, type RecognitionOptions, type RecognitionReceipt, type RecognitionJob, type RecognitionTransport } from "./recognition-state";
const options: RecognitionOptions = { purpose: "note", languages: ["eng"], pages: [{ page: 1 }], dpi: 300, scheme: "none", numbering: "keep" };
const target = { sessionId: "TEST-session", name: "TEST-note" };
const job: RecognitionJob = { version: 1, jobId: "TEST-1", status: "complete", inputSha256: "a".repeat(64), optionsDigest: "b".repeat(64), expiresAt: 100 };
const receipt: RecognitionReceipt = { operationId: "TEST-2", status: "prepared", planId: "TEST-plan", commitAllowed: true, preview: { losses: [] }, nodeRefs: [] };
function setup() { let identity = "TEST-alice-epoch1", index = 0; const calls: { path: string; body: unknown }[] = []; const stored: unknown[] = []; const transport: RecognitionTransport = { identity: () => identity, id: () => `TEST-${++index}`, persist: l => stored.push(structuredClone(l)), remove() {}, async request<T>(path: string, body?: unknown): Promise<T> { calls.push({ path, body }); if (path === "/start") return structuredClone(job) as T; return structuredClone(receipt) as T; } }; const controller = new RecognitionController(transport); return { controller, transport, calls, stored, identity: (value: string) => { identity = value; } }; }
test("prepare and commit freeze original IDs, unknown commit is queried without replay", async () => { const s = setup(); await s.controller.start(target, "pdf", "TEST", options); await s.controller.prepare(target, { purpose: "note", title: "TEST", blocks: [], fields: [], acceptPartial: false }); const original = structuredClone(s.controller.locator); s.transport.request = async (path, body) => { s.calls.push({ path, body }); if (path === "/commit") throw new Error("TEST lost receipt"); return { ...receipt, status: "committed", commitAllowed: false } as never; }; await assert.rejects(s.controller.commit(true)); assert.equal(s.controller.uncertain, true); await assert.rejects(s.controller.commit(true)); await s.controller.lookup(); assert.equal(s.controller.receipt?.status, "committed"); assert.equal(s.controller.locator?.operationId, original?.operationId); assert.equal(s.calls.filter(c => c.path === "/prepare").length, 1); assert.equal(s.calls.filter(c => c.path === "/commit").length, 1); assert.equal(s.calls.filter(c => c.path === "/operation").length, 1); assert.ok(s.stored.every(v => !JSON.stringify(v).includes("bytesBase64"))); });
test("before-effect locator storage failure sends no prepare", async () => { const s = setup(); await s.controller.start(target, "pdf", "TEST", options); s.transport.persist = () => { throw new Error("TEST quota"); }; await assert.rejects(s.controller.prepare(target, { purpose: "note", title: "TEST", blocks: [], fields: [], acceptPartial: false }), /quota/); assert.equal(s.calls.filter(c => c.path === "/prepare").length, 0); });
test("retirement and identity change reject late result", async () => { const s = setup(); let resolve!: (value: RecognitionJob) => void; s.transport.request = () => new Promise(done => { resolve = done as never; }); const pending = s.controller.start(target, "pdf", "TEST", options); s.controller.retire(); resolve(job); await pending; assert.equal(s.controller.job, undefined); const second = s.controller.start(target, "pdf", "TEST", options); s.identity("TEST-bob"); resolve({ ...job, jobId: "TEST-2" }); await second; assert.equal(s.controller.job, undefined); });
test("reload locator contains no private evidence and only queries original operation", async () => { const s = setup(); const locator = decodeRecognitionLocator(JSON.stringify({ version: 1, identity: "TEST-alice-epoch1", target, operationId: "TEST-2", planId: "TEST-plan" }), "TEST-alice-epoch1"); s.controller.restore(locator); await s.controller.lookup(); assert.deepEqual(s.calls.map(c => c.path), ["/operation"]); assert.throws(() => decodeRecognitionLocator(JSON.stringify(locator), "TEST-bob")); });
test("same stable root rebinds transient session for recovery while preserving exact original destination", async () => { const s = setup(); s.transport.target = () => ({ sessionId: "TEST-reopened-session", parent: "OTHER", name: "OTHER" }); s.controller.restore({ version: 1, identity: "TEST-alice-epoch1", target: { sessionId: "OLD", parent: "TEST-original-parent", name: "TEST-original-name" }, operationId: "TEST-2", planId: "TEST-plan" }); await s.controller.lookup(); assert.deepEqual((s.calls[0].body as { target: unknown }).target, { sessionId: "TEST-reopened-session", parent: "TEST-original-parent", name: "TEST-original-name" }); s.controller.hide(); assert.equal(s.controller.receipt, undefined); assert.equal(s.controller.locator?.operationId, "TEST-2"); await s.controller.lookup(); assert.equal(s.calls.length, 2); });
test("notRegistered observation alone cannot release an unknown prepare for a new operation", async () => { const s = setup(); s.controller.restore({ version: 1, identity: "TEST-alice-epoch1", target, operationId: "TEST-2" }); s.transport.request = async () => ({ operationId: "TEST-2", status: "notRegistered", commitAllowed: false, preview: {}, nodeRefs: [] }) as never; await s.controller.lookup(); assert.throws(() => s.controller.finish()); assert.equal(s.controller.locator?.operationId, "TEST-2"); });
test("interrupted checkpoints and expired confirmation cannot prepare a new durable operation", async () => { const review = { purpose: "note" as const, title: "TEST", blocks: [], fields: [], acceptPartial: true }; for (const value of [{ ...job, confirmationExpired: true }, { ...job, status: "interrupted" as const, result: { version: 1, status: "partial" as const, input: { sha256: job.inputSha256, bytes: 1, mediaType: "application/pdf" }, pages: [], workerFinalValidation: false } }]) { const s = setup(); s.controller.job = value; await assert.rejects(s.controller.prepare(target, review)); assert.equal(s.calls.length, 0); assert.equal(s.stored.length, 0); } });

test("a committed recognition receipt retires its original environment after a root switch without restoring the old panel", async () => {
  const s = setup(), changed: string[] = []; s.transport.committed = identity => { changed.push(identity); };
  await s.controller.start(target, "pdf", "TEST", options);
  await s.controller.prepare(target, { purpose: "note", title: "TEST", blocks: [], fields: [], acceptPartial: false });
  assert.deepEqual(changed, [], "recognition and preparation do not write author files");
  const original = structuredClone(s.controller.locator!); let release!: (value: RecognitionReceipt) => void;
  s.transport.request = () => new Promise(resolve => { release = resolve as never; });
  const commit = s.controller.commit(true); s.controller.detach(); s.identity("TEST-other-root");
  release({ ...receipt, status: "committed", commitAllowed: false }); await commit;
  assert.deepEqual(changed, [original.identity]); assert.equal(Boolean(s.controller.receipt), false); assert.equal(Boolean(s.controller.locator), false);
  s.identity(original.identity); s.controller.restore(original);
  s.transport.request = async () => ({ ...receipt, status: "committed", commitAllowed: false }) as never;
  await s.controller.lookup(); assert.equal(s.controller.receipt?.status, "committed"); assert.equal(s.controller.uncertain, false);
  assert.deepEqual(changed, [original.identity], "status recovery of the same commit does not invent another mutation");
});

test("only the original recognition operation and plan can notify a committed environment", async () => {
  for (const changedReceipt of [{ ...receipt, operationId: "another-operation" }, { ...receipt, planId: "another-plan" }]) {
    const s = setup(), changed: string[] = []; s.transport.committed = identity => { changed.push(identity); };
    s.controller.restore({ version: 1, identity: "TEST-alice-epoch1", target, operationId: receipt.operationId, planId: receipt.planId });
    s.transport.request = async () => ({ ...changedReceipt, status: "committed", commitAllowed: false }) as never;
    await assert.rejects(s.controller.lookup(), /回执无法对应原导入计划/);
    assert.deepEqual(changed, []); assert.equal(s.controller.uncertain, true); assert.equal(s.controller.locator?.planId, receipt.planId);
  }
});

test("a rendering notification failure cannot make a confirmed recognition commit uncertain", async () => {
  const s = setup(); s.controller.restore({ version: 1, identity: "TEST-alice-epoch1", target, operationId: receipt.operationId, planId: receipt.planId });
  s.transport.request = async () => ({ ...receipt, status: "committed", commitAllowed: false }) as never;
  s.transport.committed = () => { assert.equal(s.controller.receipt?.status, "committed"); assert.equal(s.controller.uncertain, false); assert.ok(s.stored.length); throw new Error("render failed"); };
  await assert.rejects(s.controller.lookup(), /render failed/);
  assert.equal(s.controller.receipt?.status, "committed"); assert.equal(s.controller.uncertain, false);
  await s.controller.lookup(); assert.equal(s.controller.receipt?.status, "committed");
});
