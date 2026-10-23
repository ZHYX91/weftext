import assert from "node:assert/strict";
import test from "node:test";
import { PersonalInputRecovery, personalInputText, type CaptureTransportRecord, type PersonalInputRecord } from "./personal-input-recovery";
import { PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
class Backend implements PersonalInputBackend {
  data = new Map<string, PersonalInputBundle>(); fail = false; newLargeTexts = 0;
  async read(audience: string) { return this.data.has(audience) ? structuredClone(this.data.get(audience)!) : undefined; }
  async compareAndWrite(audience: string, expectedCommit: number, metadata: string, texts: ReadonlyMap<string, string>) {
    if (this.fail) throw new Error("quota"); const old = this.data.get(audience);
    if ((old?.commitId ?? 0) !== expectedCommit) throw new PersonalInputStoreConflict();
    for (const [id, text] of texts) if (!old?.texts.has(id) && text.length > 65536) this.newLargeTexts++;
    this.data.set(audience, { commitId: expectedCommit + 1, metadata, texts: new Map(texts) }); return expectedCommit + 1;
  }
}
const audience = JSON.stringify(["deployment", "alice"]), other = JSON.stringify(["deployment", "bob"]);
function record(text = "OWN"): Omit<PersonalInputRecord, "sequence" | "kind"> { return { id: "test-id", owner: { kind: "body", resource: "Shared", reference: "weftext-node://w/n" }, baseHash: "a".repeat(64), origin: { baseLength: 999, pieces: [{ base: 0, length: 500 }, { text }, { base: 500, length: 499 }], detached: [], revision: 1 } }; }
async function open(backend: Backend, selected = audience) { const recovery = new PersonalInputRecovery(backend); await recovery.open(selected); return recovery; }
test("one actor's pure text survives a new recovery instance without Base, token or another actor", async () => {
  const backend = new Backend(), recovery = await open(backend); await recovery.update(audience, [record()]);
  const reader = await open(backend); await reader.open(other);
  assert.equal(personalInputText(reader.list(audience)[0]), "OWN"); assert.deepEqual(reader.list(other), []);
  assert.ok(!backend.data.get(audience)!.metadata.includes("PRIVATE_BASE"));
});
test("large immutable UTF16 input is anchored once while later small edits use the existing codec", async () => {
  const backend = new Backend(), recovery = await open(backend), large = "本人😀\ud800".repeat(30000);
  await recovery.update(audience, [record(large)]);
  for (let i = 0; i < 30; i++) await recovery.update(audience, [record(large + String(i))]);
  assert.equal(backend.newLargeTexts, 1); assert.equal(personalInputText((await open(backend)).list(audience)[0]), large + "29");
});
test("quota failure preserves latest exact RAM input and previous committed bundle", async () => {
  const backend = new Backend(), recovery = await open(backend); await recovery.update(audience, [record("first")]); const previous = structuredClone(backend.data);
  backend.fail = true; await assert.rejects(recovery.update(audience, [record("second\ud800")]), /quota/);
  assert.equal(personalInputText(recovery.list(audience)[0]), "second\ud800"); assert.deepEqual(backend.data, previous);
});
test("corrupt/old format remains unchanged and newer live text is retained only in memory", async () => {
  const backend = new Backend(); backend.data.set(audience, { commitId: 1, metadata: "corrupt original", texts: new Map() });
  const recovery = new PersonalInputRecovery(backend); await assert.rejects(recovery.open(audience));
  await assert.rejects(recovery.update(audience, [record()])); assert.equal(backend.data.get(audience)!.metadata, "corrupt original"); assert.equal(personalInputText(recovery.list(audience)[0]), "OWN");
});
test("same-text ABA invalidates recovery actions and keeps the original operation ID", async () => {
  const recovery = await open(new Backend()); await recovery.update(audience, [{ ...record(), pending: { operationId: "original-op", status: "unresolved" } }]); const first = recovery.list(audience)[0];
  await recovery.update(audience, [record("changed")]); await recovery.update(audience, [record()]); assert.throws(() => recovery.current(audience, first), /变化/);
  assert.equal(recovery.list(audience)[0].pending?.operationId, "original-op"); await recovery.settle(audience, recovery.list(audience)[0], "committed"); assert.equal(recovery.list(audience)[0].pending?.status, "committed");
});
test("explicit reduction can bring an over-budget proposal set back within bounds", async () => {
  const backend = new Backend(), recovery = await open(backend);
  await assert.rejects(recovery.update(audience, Array.from({ length: 130 }, (_, index) => ({ ...record(String(index)), id: `id:${index}` }))), /预算/);
  await assert.rejects(recovery.remove(audience, recovery.list(audience)[0]), /预算/); assert.equal(recovery.list(audience).length, 129);
  await recovery.remove(audience, recovery.list(audience)[0]); assert.equal(recovery.list(audience).length, 128); assert.equal((await open(backend)).list(audience).length, 128);
});
test("original committed candidate promotes S and a later unresolved retain cannot downgrade it", async () => {
  const backend = new Backend(), recovery = await open(backend), update = { ...record("X"), pending: { operationId: "one", status: "unresolved" as const }, candidate: { operationId: "one", baseHash: "b".repeat(64), origin: { baseLength: 1000, pieces: [{ base: 0, length: 1000 }, { text: "Y" }], detached: [], revision: 2 } } };
  await recovery.update(audience, [update]); await recovery.settle(audience, recovery.list(audience)[0], "committed"); await recovery.update(audience, [update]);
  const restored = (await open(backend)).list(audience)[0]; assert.equal(restored.pending?.status, "committed"); assert.equal(restored.baseHash, "b".repeat(64)); assert.equal(personalInputText(restored), "Y"); assert.equal(restored.candidate, undefined);
});
test("independent producers merge by record identity after a real CAS conflict; same-record disagreement preserves both", async () => {
  const backend = new Backend(), a = await open(backend), b = await open(backend);
  await a.update(audience, [record("A")]); await b.update(audience, [{ ...record("B"), id: "B" }]);
  assert.deepEqual((await open(backend)).list(audience).map(record => record.id).sort(), ["B", "test-id"]);
  const c = await open(backend), d = await open(backend); await c.update(audience, [record("C")]); const durable = structuredClone(backend.data);
  await assert.rejects(d.update(audience, [record("D")]), PersonalInputStoreConflict); assert.equal(personalInputText(d.list(audience).find(record => record.id === "test-id")!), "D"); assert.deepEqual(backend.data, durable);
});
test("capture transport shares v3 storage, retains exact U16, rejects protected extras and needs exact Core mutation proof to retire", async () => {
  const backend = new Backend(), recovery = await open(backend), draftId = crypto.randomUUID(), producerId = crypto.randomUUID();
  const proposal = { kind: "captureTransport" as const, id: `capture:${draftId}:${producerId}`, owner: { kind: "capture" as const, draftId }, producerId, producerRevision: 7, baseRevision: 4, input: { texts:{name:[],title:[],body:[{text:"OWN\ud800\0\udc00",provenance:{producerId,inputId:"a".repeat(32),revision:1}}],rootInput:[],parentInput:[],parametersText:[]}, bodyMode: "text" as const, rawInputs: [], detachedRawInputs: [] } };
  const command = { kind:"fullInput" as const, expectedRevision: 4, mutationId: crypto.randomUUID(), fingerprint: "b".repeat(64) };
  await recovery.stageCaptureCurrent(audience, proposal); await recovery.freezeCaptureMutation(audience, proposal.id, { producerRevision: 7, command, input: proposal.input }); assert.deepEqual(recovery.list(audience), []);
  const read = await open(backend), restored = read.captureTransports(audience, draftId)[0]; assert.deepEqual(restored.input.texts.body, proposal.input.texts.body);
  const stored = structuredClone(backend.data); assert.throws(() => read.stageCaptureCurrent(audience, { ...proposal, input: { ...proposal.input, fields: ["PROTECTED"] } } as unknown as typeof proposal)); assert.deepEqual(backend.data, stored);
  assert.throws(() => read.acknowledgeCapture(audience, restored, { draftId, mutationId: crypto.randomUUID(), fingerprint: command.fingerprint, recordRevision: 5 }));
  await read.acknowledgeCapture(audience, restored, { draftId, mutationId: command.mutationId, fingerprint: command.fingerprint, recordRevision: 5 }); assert.deepEqual((await open(backend)).captureTransports(audience), []);
});

test("retired audience IO remains a whole-surface pending barrier after another actor opens",async()=>{
 let release!:()=>void;const gate=new Promise<void>(resolve=>{release=resolve;});let block=true;
 const backend=new Backend(),base=backend.compareAndWrite.bind(backend);
 backend.compareAndWrite=async(...args)=>{if(block)await gate;return base(...args);};
 const recovery=new PersonalInputRecovery(backend),old=JSON.stringify(["deployment","old-owner"]),next=JSON.stringify(["deployment","new-owner"]);
 await recovery.open(old);const pending=recovery.update(old,[record("old pending own")]);
 await recovery.open(next);assert.equal(recovery.pending(next),false);assert.equal(recovery.pending(),true);
 release();block=false;await pending;assert.equal(recovery.pending(),false);assert.deepEqual(recovery.list(next),[]);
});
