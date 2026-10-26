import assert from "node:assert/strict";
import test from "node:test";
import { PersonalInputRecovery, personalInputText, type CaptureTransportRecord, type PersonalInputRecord } from "./personal-input-recovery";
import { PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
import { renderPersonalInputs } from "./personal-input-panel";
import { TestDocument } from "./dom.test-support";
import { message, uiTextValue } from "./ui-locale";
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
  const recovery = await open(new Backend()); await recovery.update(audience, [{ ...record(), pending: { operationId: "original-op", route: "known" as const, resource: "Shared", status: "unresolved" } }]); const first = recovery.list(audience)[0];
  await recovery.update(audience, [record("changed")]); await recovery.update(audience, [record()]); assert.throws(() => recovery.current(audience, first), /变化/);
  assert.equal(recovery.list(audience)[0].pending?.operationId, "original-op"); await recovery.settle(audience, recovery.list(audience)[0], "committed"); assert.equal(recovery.list(audience)[0].pending?.status, "committed");
});

test("an aliased input keeps its original owner while a new operation retains the actual alias route", async () => {
  const backend = new Backend(), recovery = await open(backend), original = record("before alias");
  await recovery.update(audience, [{ ...original, pending: { operationId: "save-A", route: "known" as const, resource: "Shared", status: "unresolved" } }]);
  await recovery.settle(audience, recovery.list(audience)[0], "committed");
  await recovery.update(audience, [{ ...original, origin: record("after alias 😀").origin, pending: { operationId: "save-B", route: "known" as const, resource: "Alias-B", status: "unresolved" } }]);
  const reopened = await open(backend), retained = reopened.list(audience)[0];
  assert.equal(retained.id, original.id); assert.deepEqual(retained.owner, original.owner); assert.equal(personalInputText(retained), "after alias 😀");
  assert.deepEqual(retained.pending, { operationId: "save-B", route: "known", resource: "Alias-B", status: "unresolved" });
  assert.equal(JSON.parse(backend.data.get(audience)!.metadata).version, 4);
  const durable = structuredClone(backend.data), ram = reopened.list(audience);
  assert.throws(() => reopened.update(audience, [{ ...original, pending: { operationId: "save-B", route: "known" as const, resource: "Shared", status: "unresolved" } }]), /原件保留/);
  assert.deepEqual(reopened.list(audience), ram); assert.deepEqual(backend.data, durable);
  assert.throws(() => reopened.update(audience, [{ ...original, owner: { kind: "body", resource: "Alias-B", reference: "weftext-node://w/n" } }]), /原件保留/);
  assert.deepEqual(reopened.list(audience), ram); assert.deepEqual(backend.data, durable);
});
test("settlement and later retains preserve the same original operation resource and terminal fact", async () => {
  for (const terminal of ["committed", "cancelled", "resolved"] as const) {
    const backend = new Backend(), recovery = await open(backend), original = record();
    const proposal = { ...original, pending: { operationId: "original-op", route: "known" as const, resource: "Alias-original", status: "unresolved" as const } };
    await recovery.update(audience, [proposal]); await recovery.settle(audience, recovery.list(audience)[0], terminal);
    await recovery.update(audience, [record("later input")]);
    await recovery.update(audience, [{ ...proposal, origin: record("latest input").origin }]);
    const restored = (await open(backend)).list(audience)[0];
    assert.deepEqual(restored.pending, { operationId: "original-op", route: "known", resource: "Alias-original", status: terminal });
    assert.equal(personalInputText(restored), "latest input");
    const durable = structuredClone(backend.data);
    assert.throws(() => recovery.update(audience, [{ ...proposal, pending: { ...proposal.pending, resource: "another-alias" } }]), /原件保留/);
    assert.deepEqual(backend.data, durable); const pending = recovery.list(audience)[0].pending; assert.ok(pending?.route === "known"); assert.equal(pending.resource, "Alias-original");
  }
});
test("pending operations require an explicit bounded original resource before changing RAM or durable metadata", async () => {
  for (const resource of [undefined, "", 7, "x".repeat(4097)]) {
    const backend = new Backend(), recovery = await open(backend); await recovery.update(audience, [record("retained")]);
    const durable = structuredClone(backend.data), ram = recovery.list(audience);
    assert.throws(() => recovery.update(audience, [{ ...record("rejected"), pending: { operationId: "one", route: "known", resource, status: "unresolved" } } as any]), /原件保留/);
    assert.deepEqual(recovery.list(audience), ram); assert.deepEqual(backend.data, durable);
  }
});
test("version 3 originals are archived exactly before upgrading an unproved operation route", async () => {
  const backend = new Backend(), old = { version: 3, records: [{ ...record("original v3 😀"), kind: "text", sequence: 1, pending: { operationId: "old-op", status: "unresolved" } }] };
  backend.data.set(audience, { commitId: 3, metadata: JSON.stringify(old), texts: new Map() }); const original = structuredClone(backend.data);
  const recovery = await open(backend), retained = recovery.list(audience)[0];
  assert.equal(personalInputText(retained), "original v3 😀"); assert.deepEqual(retained.owner, record().owner); assert.deepEqual(retained.pending, { operationId: "old-op", route: "unknownOriginalRoute", status: "unresolved" });
  const [archiveKey, archive] = [...backend.data].find(([key]) => key !== audience)!;
  assert.deepEqual(JSON.parse(archiveKey.slice("weftext-personal-input-original:".length)), [3, audience, 3]);
  assert.equal(archive.metadata, original.get(audience)!.metadata); assert.deepEqual(archive.texts, original.get(audience)!.texts);
  assert.equal(backend.data.get(audience)!.commitId, 4); assert.equal(JSON.parse(backend.data.get(audience)!.metadata).version, 4); assert.equal(recovery.pending(audience), true);
  const once = structuredClone(backend.data); await open(backend); assert.deepEqual(backend.data, once, "reopening v4 must not archive or migrate again");
  await recovery.update(audience, [record("new live input")]); assert.equal(personalInputText(recovery.list(audience)[0]), "new live input"); assert.equal(recovery.pending(audience), true);
  assert.throws(() => recovery.update(audience, [{ ...record(), pending: { operationId: "old-op", route: "known", resource: "Shared", status: "unresolved" } }]), /原件保留/);
  assert.throws(() => recovery.update(audience, [{ ...record(), pending: { operationId: "new-op", route: "known", resource: "Current", status: "unresolved" } }]), /原件保留/);
  assert.throws(() => recovery.settle(audience, recovery.list(audience)[0], "cancelled"), /原件保留/);
  assert.equal(recovery.list(audience)[0].pending!.operationId, "old-op"); assert.deepEqual([...backend.data].find(([key]) => key !== audience)![1], archive);
});

function legacy(backend: Backend, status = "unresolved", text = "original input"): PersonalInputBundle {
  const bundle = { commitId: 7, metadata: JSON.stringify({ version: 3, records: [{ ...record(text), kind: "text", sequence: 1, pending: { operationId: "original-op", status } }] }), texts: new Map<string, string>() };
  backend.data.set(audience, bundle); return structuredClone(bundle);
}
test("migration preserves every original UTF16 text reference and its complete metadata", async () => {
  const backend = new Backend(), snapshot = "a".repeat(32), own = "本人😀\ud800\0\udc00".repeat(15000);
  const old: any = { version: 3, records: [{ ...record(), kind: "text", sequence: 1, pending: { operationId: "original-op", status: "unresolved" } }] };
  old.records[0].origin.pieces = [{ text: { snapshot, from: 0, to: 0, insert: "" } }];
  const bundle = { commitId: 9, metadata: JSON.stringify(old), texts: new Map([[snapshot, own]]) }; backend.data.set(audience, structuredClone(bundle));
  const recovery = await open(backend); assert.equal(personalInputText(recovery.list(audience)[0]), own);
  const [archiveKey, backup] = [...backend.data].find(([key]) => key !== audience)!;
  assert.deepEqual(JSON.parse(archiveKey.slice("weftext-personal-input-original:".length)), [3, audience, 9]); assert.equal(backup.metadata, bundle.metadata); assert.deepEqual(backup.texts, bundle.texts);
});
test("archiving does not wrap or double-escape a metadata bundle near its original budget", async () => {
  const backend = new Backend(), original = legacy(backend, "unresolved", '\\"'.repeat(16000)), baseWrite = backend.compareAndWrite.bind(backend);
  const budget = original.metadata.length + 100;
  backend.compareAndWrite = async (...args) => { assert.ok(args[2].length <= budget, "archive must keep original metadata bytes instead of wrapping the JSON string"); return baseWrite(...args); };
  const recovery = await open(backend); assert.equal(personalInputText(recovery.list(audience)[0]), '\\"'.repeat(16000));
  assert.equal([...backend.data].find(([key]) => key !== audience)![1].metadata, original.metadata);
});
test("legacy terminal facts remain terminal without becoming a close barrier", async () => {
  for (const terminal of ["committed", "cancelled", "resolved"]) {
    const backend = new Backend(); legacy(backend, terminal); const recovery = await open(backend), retained = recovery.list(audience)[0];
    assert.equal(retained.pending!.status, terminal); assert.equal(retained.pending!.route, "unknownOriginalRoute"); assert.equal(recovery.pending(audience), false); assert.equal(recovery.pending(), false);
  }
});
test("unknown legacy routes need explicit same-audience original evidence before recovery can use a route", async () => {
  const backend = new Backend(); legacy(backend); const recovery = await open(backend), retained = recovery.list(audience)[0], before = structuredClone(backend.data);
  const wrongProofs: any[] = [
    { kind: "hostedDraft", audience: other, pending: { operationId: "original-op", resource: "Alias-B", request: { operationId: "original-op" } } },
    { kind: "hostedDraft", audience, pending: { operationId: "original-op", resource: "Alias-B", request: { operationId: "another-op" } } },
    { kind: "hostedDraft", audience, pending: { operationId: "another-op", resource: "Alias-B", request: { operationId: "original-op" } } },
    { kind: "coreReceipt", audience, receipt: { operationId: "original-op", resource: "Alias-B", status: "notRegistered" } },
    { kind: "coreReceipt", audience, receipt: { operationId: "another-op", resource: "Alias-B", status: "committed" } },
  ];
  for (const proof of wrongProofs) { assert.throws(() => recovery.proveOriginalRoute(audience, retained, proof), /原件保留/); assert.deepEqual(backend.data, before); }
  await recovery.proveOriginalRoute(audience, retained, { kind: "hostedDraft", audience, pending: { operationId: "original-op", resource: "Alias-B", request: { operationId: "original-op" } } });
  const proven = recovery.list(audience)[0]; assert.deepEqual(proven.owner, retained.owner); assert.equal(proven.id, retained.id); assert.equal(personalInputText(proven), personalInputText(retained));
  assert.deepEqual(proven.pending, { operationId: "original-op", route: "known", resource: "Alias-B", status: "unresolved" }); assert.equal(recovery.pending(audience), false);
  await recovery.settle(audience, proven, "cancelled"); assert.equal(recovery.list(audience)[0].pending!.status, "cancelled");
});
test("a scoped Core receipt can prove an unknown route while ordinary updates cannot create unknown operations", async () => {
  const backend = new Backend(); legacy(backend); const recovery = await open(backend), retained = recovery.list(audience)[0];
  await recovery.proveOriginalRoute(audience, retained, { kind: "coreReceipt", audience, receipt: { operationId: "original-op", resource: "Original-alias", status: "recoveryPending" } });
  assert.deepEqual(recovery.list(audience)[0].pending, { operationId: "original-op", route: "known", resource: "Original-alias", status: "unresolved" });
  assert.throws(() => recovery.update(audience, [{ ...record(), id: "new-input", pending: { operationId: "new-unknown", route: "unknownOriginalRoute", status: "unresolved" } }]), /原件保留/);
  assert.throws(() => recovery.update(audience, [{ ...record(), id: "new-input", pending: { operationId: "original-op", route: "known", resource: "wrong-alias", status: "unresolved" } }]), /原件保留/);
});
test("malformed closed v3 records cause no archive or active write", async () => {
  const mutate: ((value: any) => void)[] = [
    value => { value.unknown = true; }, value => { value.records[0].unknown = true; },
    value => { value.records[0].pending.unknown = true; }, value => { value.records[0].pending.status = "notCommitted"; },
    value => { value.records[0].owner = { kind: "field", resource: "Shared", fieldId: "field" }; },
    value => { value.records[0].origin.pieces[1].text = { snapshot: "a".repeat(32), from: 0, to: 0, insert: "" }; },
  ];
  for (const corrupt of mutate) {
    const backend = new Backend(), bundle = legacy(backend), value = JSON.parse(bundle.metadata); corrupt(value); backend.data.set(audience, { ...bundle, metadata: JSON.stringify(value) }); const original = structuredClone(backend.data);
    const recovery = new PersonalInputRecovery(backend); await assert.rejects(recovery.open(audience)); assert.deepEqual(backend.data, original);
  }
});
test("archive failure or failed readback leaves the complete active v3 original untouched", async () => {
  for (const failure of ["write", "readback"] as const) {
    const backend = new Backend(), original = legacy(backend), baseWrite = backend.compareAndWrite.bind(backend), baseRead = backend.read.bind(backend);
    backend.compareAndWrite = async (...args) => { if (failure === "write" && args[0] !== audience) throw new Error("archive quota"); return baseWrite(...args); };
    backend.read = async key => { const value = await baseRead(key); return failure === "readback" && key !== audience && value ? { ...value, metadata: value.metadata + "changed" } : value; };
    const recovery = new PersonalInputRecovery(backend); await assert.rejects(recovery.open(audience)); assert.deepEqual(backend.data.get(audience), original);
  }
});
test("an active migration CAS conflict adopts the newer v4 input instead of overwriting it", async () => {
  const backend = new Backend(), original = legacy(backend), baseWrite = backend.compareAndWrite.bind(backend); let attempts = 0;
  backend.compareAndWrite = async (...args) => {
    if (args[0] === audience) {
      attempts++; backend.data.set(audience, { commitId: 8, metadata: JSON.stringify({ version: 4, records: [{ ...record("newer page input 😀"), kind: "text", sequence: 4 }] }), texts: new Map() });
      throw new PersonalInputStoreConflict();
    }
    return baseWrite(...args);
  };
  const recovery = await open(backend); assert.equal(personalInputText(recovery.list(audience)[0]), "newer page input 😀"); assert.equal(attempts, 1); assert.equal(backend.data.get(audience)!.commitId, 8);
  const archive = [...backend.data].find(([key]) => key !== audience)![1]; assert.equal(archive.metadata, original.metadata);
});
test("a failed active migration is retriable from the untouched original and reuses its verified archive", async () => {
  const backend = new Backend(), original = legacy(backend), baseWrite = backend.compareAndWrite.bind(backend); let fail = true, archiveWrites = 0;
  backend.compareAndWrite = async (...args) => { if (args[0] !== audience) archiveWrites++; else if (fail) throw new Error("active quota"); return baseWrite(...args); };
  const first = new PersonalInputRecovery(backend); await assert.rejects(first.open(audience), /active quota/); assert.deepEqual(backend.data.get(audience), original); assert.equal(archiveWrites, 1);
  fail = false; const recovered = await open(backend); assert.equal(recovered.list(audience)[0].pending!.route, "unknownOriginalRoute"); assert.equal(archiveWrites, 1);
});
test("the unknown-route recovery row preserves copy access and gives no query, settle or restore writer", async () => {
  const backend = new Backend(); legacy(backend); const recovery = await open(backend), document = new TestDocument(), host = document.createElement("section");
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "document"); Object.defineProperty(globalThis, "document", { configurable: true, value: document });
  try {
    renderPersonalInputs(host as unknown as HTMLElement, recovery, audience, { restore: async () => { throw new Error("unexpected restore"); }, operation: async () => { throw new Error("unexpected operation"); }, editCopy: async () => { throw new Error("unexpected copy editing"); }, editingCopy: () => false, producers: () => [], discardProducer: async () => {}, changed() {} });
    assert.equal(host.find("textarea").value, "original input"); assert.equal(host.find("button", uiTextValue(message("personalInput.copyText"))).disabled, false);
    assert.equal(host.find("button", uiTextValue(message("personalInput.resolveOriginalSave"))).disabled, true); assert.equal(host.find("button", uiTextValue(message("personalInput.restoreDraft"))).disabled, true);
    assert.ok(host.textContent.includes(uiTextValue(message("personalInput.unknownOriginalRoute"))));
  } finally { if (descriptor) Object.defineProperty(globalThis, "document", descriptor); else Reflect.deleteProperty(globalThis, "document"); }
});

test("explicit reduction can bring an over-budget proposal set back within bounds", async () => {
  const backend = new Backend(), recovery = await open(backend);
  await assert.rejects(recovery.update(audience, Array.from({ length: 130 }, (_, index) => ({ ...record(String(index)), id: `id:${index}` }))), /预算/);
  await assert.rejects(recovery.remove(audience, recovery.list(audience)[0]), /预算/); assert.equal(recovery.list(audience).length, 129);
  await recovery.remove(audience, recovery.list(audience)[0]); assert.equal(recovery.list(audience).length, 128); assert.equal((await open(backend)).list(audience).length, 128);
});
test("original committed candidate promotes S and a later unresolved retain cannot downgrade it", async () => {
  const backend = new Backend(), recovery = await open(backend), update = { ...record("X"), pending: { operationId: "one", route: "known" as const, resource: "Shared", status: "unresolved" as const }, candidate: { operationId: "one", baseHash: "b".repeat(64), origin: { baseLength: 1000, pieces: [{ base: 0, length: 1000 }, { text: "Y" }], detached: [], revision: 2 } } };
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
test("a CAS merge cannot give the same original operation different routes through independent record IDs", async () => {
  const backend = new Backend(), first = await open(backend), second = await open(backend);
  await first.update(audience, [{ ...record("first own input"), id: "input-A", pending: { operationId: "same-original", route: "known", resource: "Alias-A", status: "unresolved" } }]);
  const original = structuredClone(backend.data);
  await assert.rejects(second.update(audience, [{ ...record("second own input"), id: "input-B", pending: { operationId: "same-original", route: "known", resource: "Alias-B", status: "unresolved" } }]), /原件保留/);
  assert.deepEqual(backend.data, original); assert.equal(personalInputText(second.list(audience)[0]), "second own input"); assert.equal(second.pending(audience), true);
  const durable = (await open(backend)).list(audience); assert.equal(durable.length, 1); assert.equal(durable[0].id, "input-A");
});
test("direct v4 reads refuse contradictory routes for the same operation without changing originals", async () => {
  for (const pending of [{ operationId: "same-original", route: "known", resource: "Alias-B", status: "unresolved" }, { operationId: "same-original", route: "unknownOriginalRoute", status: "unresolved" }]) {
    const backend = new Backend(); backend.data.set(audience, { commitId: 4, metadata: JSON.stringify({ version: 4, records: [
      { ...record("first own input"), id: "input-A", kind: "text", sequence: 1, pending: { operationId: "same-original", route: "known", resource: "Alias-A", status: "unresolved" } },
      { ...record("second own input"), id: "input-B", kind: "text", sequence: 1, pending },
    ] }), texts: new Map() });
    const original = structuredClone(backend.data), recovery = new PersonalInputRecovery(backend); await assert.rejects(recovery.open(audience), /原件保留/); assert.deepEqual(backend.data, original);
  }
});
test("capture transport shares v4 storage, retains exact U16, rejects protected extras and needs exact Core mutation proof to retire", async () => {
  const backend = new Backend(), recovery = await open(backend), draftId = crypto.randomUUID(), producerId = crypto.randomUUID();
  const proposal = { kind: "captureTransport" as const, id: `capture:${draftId}:${producerId}`, owner: { kind: "capture" as const, draftId }, producerId, producerRevision: 7, baseRevision: 4, input: { texts:{name:[],title:[],body:[{text:"OWN\ud800\0\udc00",provenance:{producerId,inputId:"a".repeat(32),revision:1}}],rootInput:[],parentInput:[],parametersText:[]}, bodyMode: "text" as const, rawInputs: [], detachedRawInputs: [] } };
  const command = { kind:"fullInput" as const, expectedRevision: 4, mutationId: crypto.randomUUID(), fingerprint: "b".repeat(64) };
  await recovery.stageCaptureCurrent(audience, proposal); await recovery.freezeCaptureMutation(audience, proposal.id, { producerRevision: 7, command, input: proposal.input }); assert.deepEqual(recovery.list(audience), []);
  const read = await open(backend), restored = read.captureTransports(audience, draftId)[0]; assert.deepEqual(restored.input.texts.body, proposal.input.texts.body);
  const stored = structuredClone(backend.data); assert.throws(() => read.stageCaptureCurrent(audience, { ...proposal, input: { ...proposal.input, fields: ["PROTECTED"] } } as unknown as typeof proposal)); assert.deepEqual(backend.data, stored);
  assert.throws(() => read.acknowledgeCapture(audience, restored, { draftId, mutationId: crypto.randomUUID(), fingerprint: command.fingerprint, recordRevision: 5 }));
  await read.acknowledgeCapture(audience, restored, { draftId, mutationId: command.mutationId, fingerprint: command.fingerprint, recordRevision: 5 }); assert.deepEqual((await open(backend)).captureTransports(audience), []);
});
test("valid v3 Capture transport upgrades without manufacturing an unknown text operation", async () => {
  const backend = new Backend(), recovery = await open(backend), draftId = crypto.randomUUID(), producerId = crypto.randomUUID();
  const proposal = { kind: "captureTransport" as const, id: `capture:${draftId}:${producerId}`, owner: { kind: "capture" as const, draftId }, producerId, producerRevision: 1, baseRevision: 1,
    input: { texts: { name: [], title: [], body: [{ text: "capture own 😀\ud800", provenance: { producerId, inputId: "a".repeat(32), revision: 1 } }], rootInput: [], parentInput: [], parametersText: [] }, bodyMode: "text" as const, rawInputs: [], detachedRawInputs: [] } };
  await recovery.stageCaptureCurrent(audience, proposal);
  const stored = backend.data.get(audience)!, v3 = JSON.parse(stored.metadata); v3.version = 3; backend.data.set(audience, { ...stored, metadata: JSON.stringify(v3) });
  const upgraded = await open(backend); assert.deepEqual(upgraded.captureTransports(audience)[0].input, proposal.input); assert.deepEqual(upgraded.list(audience), []); assert.equal(upgraded.pending(audience), false);
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
