import assert from "node:assert/strict";
import test from "node:test";
import { CaptureController } from "./capture-controller";
import { CapturePersistence, captureString, captureText, emptyCaptureInput, type CaptureDraftRecord, type CaptureTransport } from "./capture-draft-state";

const initial = (): CaptureDraftRecord => ({ version: 2, draftId: crypto.randomUUID(), revision: 1, inputRevision: 1, inputCut: "full", lastMutation: { kind:"input", mutationId: crypto.randomUUID(), fingerprint: "a".repeat(64), resultingRevision: 1 }, currentInput: emptyCaptureInput(), pending: null, target: null });
const deferred = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };
function fixture(intercept?: (path: string, payload: Record<string, any>) => Promise<void>) {
  let record = initial();
  const calls: { path: string; payload: Record<string, any> }[] = [];
  const request: CaptureTransport = async <T>(path: string, body: unknown) => {
    const payload = structuredClone(body) as Record<string, any>;
    calls.push({ path, payload });
    await intercept?.(path, payload);
    if (path.endsWith("/flush")) return { draftId: record.draftId, recordRevision: record.revision, inputRevision: record.inputRevision, inputCut: "full", lastMutationId: record.lastMutation.mutationId, pendingOperationId: record.pending?.operationId ?? null, recordDigest: "b".repeat(64) } as T;
    if (path.endsWith("/mutation")) return { status: "supersededOrUnknown", draft: structuredClone(record) } as T;
    assert.equal(payload.expectedRevision, record.revision);
    record = structuredClone(record);
    record.revision++;
    if (path.endsWith("/input")) { record.currentInput = payload.input; record.inputRevision++; }
    else if (path.endsWith("/freeze")) { assert.equal(payload.request.inputRevision, record.inputRevision); record.pending = payload.request; }
    else throw new Error(`unexpected ${path}`);
    record.lastMutation = {kind:path.endsWith("/freeze")?"freeze":"input", mutationId: payload.mutationId, fingerprint: "a".repeat(64), resultingRevision: record.revision };
    return structuredClone(record) as T;
  };
  const controller = new CaptureController(new CapturePersistence(request), record);
  return { controller, calls, record: () => structuredClone(record) };
}
test("submission freezes the original wire and input cut before later raw input reaches Core", async () => {
  const first = deferred(), entered = deferred();
  let inputCalls = 0;
  const f = fixture(async path => { if (path.endsWith("/input") && ++inputCalls === 1) { entered.resolve(); await first.promise; } });
  const a = emptyCaptureInput(); a.body = captureText("A\ud800");
  f.controller.edit(a);
  await entered.promise;
  const payload = { operationId: crypto.randomUUID(), root: "R", parent: "R", body: "submitted\ud800", fields: {}, types: [] };
  const frozen = f.controller.freezeSubmission(payload, "capture");
  payload.body = "changed caller object";
  const b = emptyCaptureInput(); b.body = captureText("B\udc00");
  f.controller.edit(b);
  first.resolve();
  const original = await frozen;
  await f.controller.drain();
  const actual = f.record();
  assert.equal(JSON.parse(original.wire).body, "submitted\ud800");
  assert.equal(actual.pending!.wire, original.wire);
  assert.equal(captureString(actual.currentInput.body), "B\udc00");
  assert.ok(actual.pending!.inputRevision < actual.inputRevision);
  assert.deepEqual(f.calls.filter(call => !call.path.endsWith("/flush")).map(call => call.path.split("/").at(-1)), ["input", "input", "freeze", "input"]);
});
test("an unconfirmed command blocks subsequent submission and retries the same mutation/cut", async () => {
  let fail = true;
  const f = fixture(async path => { if (fail && path.endsWith("/input")) throw new Error("offline"); });
  const input = emptyCaptureInput(); input.body = captureText("never dropped"); f.controller.edit(input);
  await assert.rejects(f.controller.drain());
  const first = f.calls.find(call => call.path.endsWith("/input"))!;
  const next = emptyCaptureInput(); next.body = captureText("newer"); f.controller.edit(next);
  await assert.rejects(f.controller.freezeSubmission({ operationId: crypto.randomUUID(), root: "R", parent: "R" }, "capture"));
  assert.equal(f.calls.some(call => call.path.endsWith("/freeze")), false);
  fail = false;
  await f.controller.retryPersistence();
  const retried = f.calls.filter(call => call.path.endsWith("/input"))[1];
  assert.deepEqual(retried.payload, first.payload);
  assert.equal(captureString(f.record().currentInput.body), "newer");
  assert.ok(f.record().pending);
});
test("close first collects owned raw, proves its queue, and rejects a stale producer proof", async () => {
  const f = fixture();
  const proof = await f.controller.prepareClose(() => { const input = f.controller.state.input; input.body = captureText("last synchronous raw\ud800"); f.controller.edit(input); });
  assert.equal(captureString(f.record().currentInput.body), "last synchronous raw\ud800");
  assert.equal(proof.recordRevision, f.record().revision);
  assert.equal(f.controller.freezeClose({ ...proof, ownerGeneration: crypto.randomUUID() }), false);
  f.controller.edit(f.controller.state.input); // Equal bytes still invalidate the permit.
  assert.equal(f.controller.freezeClose(proof), false);
  await f.controller.drain();
  const fresh = await f.controller.prepareClose(() => {});
  assert.equal(f.controller.freezeClose(fresh), true);
  assert.throws(() => f.controller.edit(f.controller.state.input));
  f.controller.thawClose();
  f.controller.beginComposition();
  await assert.rejects(f.controller.prepareClose(() => {}));
  f.controller.endComposition();
});
test("retired structure raw is preserved unbound rather than attached to the replacement index", async () => {
  const f = fixture();
  const input = emptyCaptureInput();
  input.rawInputs = [{ fieldId: "tasks/amount", valuePath: [{ kind: "index", index: 0 }], controlRole: "decimal", text: captureText("-\ud800"), binding: "exact", author: null }];
  f.controller.edit(input);
  let invalidated = false;
  const session = f.controller.wrapSession({ bind: () => () => {}, bindChoice: () => () => {}, recordAction: () => {}, capture: () => undefined, inspect: () => { throw new Error("Not part of this raw-retirement test"); }, acknowledge: () => false, freeze: () => false, thaw: () => {}, retire: () => {}, invalidate: () => { invalidated = true; } }, "tasks/amount");
  session.invalidate("field", []);
  assert.equal(invalidated, true);
  await f.controller.drain();
  assert.deepEqual(f.record().currentInput.rawInputs, []);
  assert.equal(captureString(f.record().currentInput.detachedRawInputs[0].input.text), "-\ud800");
  assert.equal(f.record().currentInput.detachedRawInputs[0].input.valuePath[0].kind, "index");
});

test("a restarted author-only Core cut cannot become a full form by checkpoint, submission or close",async()=>{
 const record=initial();record.inputCut="authorOnly";
 const calls:string[]=[];
 const persistence={async mutate(){calls.push("mutate");return record;},async flush(){calls.push("flush");throw new Error("not a complete form");}};
 const controller=new CaptureController(persistence,JSON.parse(JSON.stringify(record)));
 assert.equal(controller.state.formReady,false);
 assert.throws(()=>controller.freezeSubmission({operationId:crypto.randomUUID()},"capture"),/检查点/);
 await assert.rejects(controller.prepareClose(()=>{}));
 assert.equal(controller.freezeClose({ownerGeneration:controller.ownerGeneration,editRevision:0}),false);
 assert.deepEqual(calls,[]);
});
