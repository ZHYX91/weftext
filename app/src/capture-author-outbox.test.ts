import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { CaptureAuthorOutbox } from "./capture-author-outbox";
import { captureAuthorFingerprint, captureInputFingerprint, captureAuthorInput, captureString, captureText, emptyCaptureInput, type CaptureAuthorInput } from "./capture-draft-state";
import { PersonalInputRecovery } from "./personal-input-recovery";
import type { PersonalInputBackend, PersonalInputBundle } from "./personal-input-storage";

const proved = (value: string) => ({text:captureText(value),provenance:{producerId:crypto.randomUUID(),inputId:"b".repeat(32),revision:1}});
const audience = JSON.stringify(["deployment", "alice"]);
test("Core and browser use the same exact author-input command fingerprint fixture", async () => {
  const fixture = JSON.parse(readFileSync(new URL("./capture-author-wire.fixture.json", import.meta.url), "utf8"));
  assert.equal(await captureAuthorFingerprint(fixture.expectedRevision, fixture.input), fixture.fingerprint);
});
class Backend implements PersonalInputBackend {
  bundle?: PersonalInputBundle;
  async read() { return structuredClone(this.bundle); }
  async compareAndWrite(_audience: string, expected: number, metadata: string, texts: ReadonlyMap<string, string>) {
    assert.equal(expected, this.bundle?.commitId ?? 0); this.bundle = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1;
  }
}
test("private field raw retains exact default and incomplete U16, while pure proposal contains proved own text and provenance only", () => {
  const input = emptyCaptureInput(), provenance = { producerId: crypto.randomUUID(), inputId: "a".repeat(32), revision: 4 };
  input.fields = [{ fieldId: "private/field", valueState: { kind: "valid", value: { kind: "text", value: "PRIVATE DEFAULT" } } }];
  input.templateId = "PRIVATE TEMPLATE";
  input.rawInputs = [{ fieldId: "private/field", valuePath: [{ kind: "member", key: "name" }], controlRole: "text", text: captureText("PRIVATE DEFAULT\ud800\0X"), binding: "exact", author: { text: captureText("\ud800\0X"), provenance } },
    { fieldId: "private/untouched", valuePath: [], controlRole: "text", text: captureText("UNTOUCHED SECRET"), binding: "exact", author: null }];
  const pure = captureAuthorInput(input);
  assert.equal(pure.rawInputs.length, 1); assert.equal(captureString(pure.rawInputs[0].text), "\ud800\0X"); assert.deepEqual(pure.rawInputs[0].provenance, provenance);
  assert.doesNotMatch(JSON.stringify(pure), /PRIVATE DEFAULT|PRIVATE TEMPLATE|UNTOUCHED SECRET|binding|formBasis/);
  assert.equal(captureString(input.rawInputs[0].text), "PRIVATE DEFAULT\ud800\0X");
});
test("lost original author reply uses its exact query and clears only after matching real Core proof", async () => {
  const backend = new Backend(), store = new PersonalInputRecovery(backend); await store.open(audience);
  const input = captureAuthorInput(emptyCaptureInput()); input.texts.body = [proved("own\ud800\0")];
  const command = { draftId: crypto.randomUUID(), expectedRevision: 7, mutationId: crypto.randomUUID(), input };
  const fingerprint = await captureAuthorFingerprint(7, input), original = structuredClone(command), calls: string[] = [];
  const record = { draftId: command.draftId, inputCut: "authorOnly" as const, revision: 8, lastMutation: { kind:"authorInput" as const, mutationId: command.mutationId, fingerprint, resultingRevision: 8 } };
  const outbox = new CaptureAuthorOutbox(store, audience, crypto.randomUUID(), {
    async write(actual) { calls.push("write"); assert.deepEqual(actual, original); assert.equal(store.captureTransports(audience).length, 1); throw new Error("controlled response loss"); },
    async lookup(actual) { calls.push("query"); assert.deepEqual(actual, original); return { status: "applied", draft: record }; }, lookupInput:async()=>{throw new Error("not a full-input test");}, fullInput:()=>undefined, recovery: actual => actual,
  });
  const pending = outbox.persist(command, 3); command.input.texts.body = [proved("later")];
  assert.deepEqual(await pending, record); assert.deepEqual(calls, ["write", "query"]); assert.deepEqual(store.captureTransports(audience), []);
});
test("a superseding writer cannot prove an old transport succeeded; process recovery queries and never resends it", async () => {
  const backend = new Backend(), store = new PersonalInputRecovery(backend); await store.open(audience);
  const input: CaptureAuthorInput = captureAuthorInput(emptyCaptureInput()); input.texts.body = [proved("OWN RAW\ud800")];
  const command = { draftId: crypto.randomUUID(), expectedRevision: 2, mutationId: crypto.randomUUID(), input };
  let writes = 0, lookups = 0;
  const port = { async write() { writes++; throw new Error("unavailable"); }, async lookup() { lookups++; return { status: "supersededOrUnknown", draft: { draftId: command.draftId, inputCut: "authorOnly" as const, revision: 4, lastMutation: { kind:"authorInput" as const, mutationId: crypto.randomUUID(), fingerprint: "a".repeat(64), resultingRevision: 4 } } }; }, lookupInput:async()=>{throw new Error("not a full-input test");}, fullInput:()=>undefined, recovery: (record: { draftId: string; inputCut:"authorOnly"; revision: number; lastMutation: { kind:"authorInput"; mutationId: string; fingerprint: string; resultingRevision: number } }) => record };
  const outbox = new CaptureAuthorOutbox(store, audience, crypto.randomUUID(), port);
  await assert.rejects(outbox.persist(command, 1)); assert.equal(store.captureTransports(audience).length, 1);
  const restarted = new PersonalInputRecovery(backend); await restarted.open(audience);
  const recovered = await new CaptureAuthorOutbox(restarted, audience, crypto.randomUUID(), port).recover(command.draftId);
  assert.equal(writes, 1); assert.equal(lookups, 2); assert.equal(recovered.length, 1); assert.equal(recovered[0].input.texts.body[0].text, "OWN RAW\ud800");
});
test("new current raw is durable while an older command waits; its old ACK retires only the frozen pending cut", async () => {
  const backend = new Backend(), store = new PersonalInputRecovery(backend); await store.open(audience);
  const input = captureAuthorInput(emptyCaptureInput()); input.texts.body = [proved("A\ud800")];
  const command = { draftId: crypto.randomUUID(), expectedRevision: 3, mutationId: crypto.randomUUID(), input };
  let release!: () => void, entered!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; }), ready = new Promise<void>(resolve => { entered = resolve; });
  const outbox = new CaptureAuthorOutbox(store, audience, crypto.randomUUID(), {
    async write(actual) { entered(); await gate; return { draftId: actual.draftId, inputCut: "authorOnly" as const, revision: 4, lastMutation: { kind:"authorInput" as const, mutationId: actual.mutationId, fingerprint: await captureAuthorFingerprint(3, actual.input), resultingRevision: 4 } }; },
    async lookup() { throw new Error("No query is needed"); }, lookupInput:async()=>{throw new Error("not a full-input test");}, fullInput:()=>undefined, recovery: record => record,
  });
  const pending = outbox.persist(command, 1); await ready;
  const next = structuredClone(input); next.texts.body = [proved("B\udc00\0")]; await outbox.stageCurrent(command.draftId, 3, 2, next);
  const restarted = new PersonalInputRecovery(backend); await restarted.open(audience);
  const cut = restarted.captureTransports(audience, command.draftId)[0]; assert.equal(cut.input.texts.body[0].text, "B\udc00\0"); assert.equal(cut.pending!.input.texts.body[0].text, "A\ud800"); assert.equal(cut.pending!.command.mutationId, command.mutationId);
  release(); await pending;
  const again = new PersonalInputRecovery(backend); await again.open(audience);
  const after = again.captureTransports(audience, command.draftId)[0]; assert.equal(after.input.texts.body[0].text, "B\udc00\0"); assert.equal(after.pending, null); assert.equal(after.baseRevision, 4);
});

test("Core and browser canonical full input fingerprints bind exactly the full candidate and proof",async()=>{
 const fixture=JSON.parse(readFileSync(new URL("./capture-author-wire.fixture.json",import.meta.url),"utf8")).fullInput;
 assert.equal(await captureInputFingerprint(fixture.expectedRevision,fixture.input),fixture.fingerprint);
 const changed=structuredClone(fixture.input);changed.fields[0].valueState.value.value[0].value.value+="X";
 assert.notEqual(await captureInputFingerprint(fixture.expectedRevision,changed),fixture.fingerprint);
});
