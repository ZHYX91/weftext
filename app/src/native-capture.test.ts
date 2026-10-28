import test from "node:test";
import assert from "node:assert/strict";
import { NativeCapture } from "./native-capture";
import type { NativeSurface } from "./native-host";
import type { MountedLocalCapture } from "./capture-mount";
import { beginNativeRequest, nativeApiIdle, thawNativeApi } from "./native-api-gate";

const request = { surfaceId: "surface", surfaceEpoch: "epoch", closeAttemptId: "close-1" };
const original = { draftId: crypto.randomUUID(), operationId: crypto.randomUUID() };
function fixture() {
  const sent: Record<string, unknown>[] = [];
  let receive!: (value: Record<string, unknown>) => void;
  let busy = false, froze = false, prepareCount = 0;
  const proof = { draftId: original.draftId, ownerGeneration: crypto.randomUUID(), recordRevision: 7, inputRevision: 4, inputCut: "full" as const,
    lastMutationId: crypto.randomUUID(), recordDigest: "a".repeat(64), pendingOperationId: original.operationId, editRevision: 3 };
  const surface = { listen(listener: typeof receive) { receive = listener; return () => {}; }, post(value: Record<string, unknown>) { sent.push(value); } } as unknown as NativeSurface;
  const mounted = { assertCloseReady() { if (busy) throw new Error("Actual owner is still delivering"); }, form: { showError() {} },
    guard: { async prepare() { prepareCount++; return proof; }, freeze(value: unknown) { assert.equal(value, proof); froze = true; return true; }, thaw() { froze = false; } } } as unknown as MountedLocalCapture;
  const bridge = new NativeCapture(surface, mounted);
  return { bridge, sent, receive: (value: Record<string, unknown>) => receive(value), busy(value: boolean) { busy = value; }, froze: () => froze, prepares: () => prepareCount };
}

test("Capture waits for the actual original Library adoption, ignores other replies, and blocks close while pending", async () => {
  thawNativeApi();
  const f = fixture();
  let completed = false;
  const delivered = f.bridge.open(original).then(() => { completed = true; });
  assert.equal(f.sent[0].type, "capture-open");
  assert.equal(f.sent[0].operationId, original.operationId);
  await assert.rejects(f.bridge.participant.prepare(request));
  f.receive({ type: "capture-open-result", requestId: "another-request", accepted: true });
  await Promise.resolve(); assert.equal(completed, false);
  f.receive({ type: "capture-open-result", requestId: f.sent[0].requestId, accepted: true });
  await delivered; assert.equal(completed, true);
  const proofs = await f.bridge.participant.prepare(request);
  assert.equal(proofs[0].kind, "capture");
  assert.equal(proofs[0].pendingOperationId, original.operationId);
  assert.equal(f.bridge.participant.freeze({ ...request, closeAttemptId: "stale" }), false);
  assert.equal(f.bridge.participant.freeze(request), true);
  assert.equal(f.froze(), true);
  f.bridge.participant.thaw(); assert.equal(f.froze(), false); assert.equal(nativeApiIdle(), true);
});

test("negative delivery preserves the original for an explicit retry and real producer work refuses close", async () => {
  thawNativeApi();
  const f = fixture();
  const delivery = f.bridge.open(original);
  f.receive({ type: "capture-open-result", requestId: f.sent[0].requestId, accepted: false });
  await assert.rejects(delivery);
  f.busy(true); await assert.rejects(f.bridge.participant.prepare(request), /still delivering/);
  assert.equal(f.prepares(), 0);
  f.busy(false);
  const finish = beginNativeRequest(); await assert.rejects(f.bridge.participant.prepare(request)); finish();
  await f.bridge.participant.prepare(request);
  const late = beginNativeRequest(); assert.equal(f.bridge.participant.freeze(request), false); late();
  f.bridge.participant.thaw();
  const retry = f.bridge.open(original);
  assert.equal(f.sent[1].operationId, original.operationId);
  assert.notEqual(f.sent[1].requestId, f.sent[0].requestId);
  f.receive({ type: "capture-open-result", requestId: f.sent[1].requestId, accepted: true }); await retry;
});

test("Capture cannot admit a late result-opening request after its producer was frozen", async () => {
  thawNativeApi();
  const f = fixture();
  try {
    await f.bridge.participant.prepare(request);
    assert.equal(f.bridge.participant.freeze(request), true);
    const before = f.sent.length;
    const late = f.bridge.open(original);
    // Settle a wrongly admitted request too, so the regression fails without
    // leaving its original transport unresolved in the test process.
    if (f.sent.length !== before) f.receive({ type: "capture-open-result", requestId: f.sent.at(-1)!.requestId, accepted: true });
    await assert.rejects(late);
    assert.equal(f.sent.length, before);
  } finally { f.bridge.participant.thaw(); }
});


test("Capture close preparation synchronously excludes handoff, cancellation invalidates its late proof", async () => {
  thawNativeApi();
  let resolve!: (value: unknown) => void;
  const held = new Promise<unknown>(accept => { resolve=accept; });
  const proof = { draftId:crypto.randomUUID(), ownerGeneration:crypto.randomUUID(), recordRevision:1,inputRevision:1,inputCut:"full",lastMutationId:crypto.randomUUID(),recordDigest:"a".repeat(64),pendingOperationId:null,editRevision:0 };
  const surface={listen(){return()=>{};},post(){}} as unknown as NativeSurface;
  const mounted={assertCloseReady(){},form:{showError(){}},guard:{prepare:()=>held,freeze:()=>true,thaw(){}}} as unknown as MountedLocalCapture;
  const native=new NativeCapture(surface,mounted);
  const preparing=native.participant.prepare(request);
  assert.throws(()=>native.assertHandoffReady());
  native.participant.thaw();resolve(proof);
  await assert.rejects(preparing);assert.equal(native.participant.freeze(request),false);
  native.assertHandoffReady();native.dispose();assert.throws(()=>native.assertHandoffReady());
});
