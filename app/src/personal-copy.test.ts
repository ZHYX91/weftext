import assert from "node:assert/strict";
import test from "node:test";
import { PersonalControlInput } from "./personal-control-input";
import { captureAuthorInput, type AuthorInputOwner, type AuthorInputSession, type AuthorInputSink, type AuthorInputSnapshot } from "./author-input";
import { PersonalInputRecovery, PERSONAL_COPY_ONLY_SLOT, isPersonalCopy, personalInputText, type PersonalCopySeed, type PersonalInputRecord } from "./personal-input-recovery";
import { PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
import { hasPersonalInput } from "./source-input-origin";

// The resolver overlays the private candidates on the actual product modules.
// Only storage delivery and the DOM event boundary are substituted. These
// composition events are regression inputs, not an OS IME acceptance claim.
class Backend implements PersonalInputBackend {
  readonly data = new Map<string, PersonalInputBundle>();
  fail = false;
  wait?: Promise<void>;
  async read(audience: string): Promise<PersonalInputBundle | undefined> { return structuredClone(this.data.get(audience)); }
  async compareAndWrite(audience: string, expected: number, metadata: string, texts: ReadonlyMap<string, string>): Promise<number> {
    if (this.wait) await this.wait;
    if (this.fail) throw new Error("copy recovery quota");
    if ((this.data.get(audience)?.commitId ?? 0) !== expected) throw new PersonalInputStoreConflict();
    this.data.set(audience, { commitId: expected + 1, metadata, texts: new Map(texts) });
    return expected + 1;
  }
}
const audience = JSON.stringify(["deployment", "alice"]), otherAudience = JSON.stringify(["deployment", "bob"]);
const originalOwner = { kind: "body" as const, resource: "Original", reference: "weftext-node://workspace/original" };
function originalInput(text: string) {
  return {
    id: "original-input", owner: originalOwner, baseHash: "a".repeat(64),
    origin: { baseLength: 4, pieces: [{ base: 0, length: 4 }, ...(text ? [{ text }] : [])], detached: [], revision: 1 },
    candidate: { operationId: "original-save", baseHash: "b".repeat(64), origin: { baseLength: 4, pieces: [{ base: 0, length: 4 }, { text: "CANDIDATE_ONLY_NOT_A_COPY_SEED" }], detached: [], revision: 2 } },
    pending: { operationId: "original-save", route: "known" as const, resource: "Original-route", status: "unresolved" as const },
  };
}
async function prepare(text = "本人种子😀\ud800\0\udc00") {
  const backend = new Backend(), recovery = new PersonalInputRecovery(backend), update = originalInput(text);
  await recovery.open(audience); await recovery.update(audience, [update]);
  const original = recovery.list(audience)[0];
  return { backend, recovery, original, update, text };
}
async function reopen(backend: Backend): Promise<PersonalInputRecovery> { const recovery = new PersonalInputRecovery(backend); await recovery.open(audience); return recovery; }
function deferred() { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; }

class Control {
  value = ""; dataset: Record<string, string> = {}; isConnected = true; readOnly = false; disabled = false;
  id = ""; type = "text"; selectionStart = 0; selectionEnd = 0;
  matches(): boolean { return false; }
}
function setupControls() {
  const listeners = new Map<string, ((event: any) => void)[]>(), state = { audience, epoch: 1, lease: 0 };
  const document = { addEventListener(type: string, listener: (event: any) => void) { listeners.set(type, [...listeners.get(type) ?? [], listener]); } };
  const defaults: AuthorInputSnapshot[] = [];
  const collector = new PersonalControlInput(document as unknown as Document, context => context.audience === state.audience && context.epoch === state.epoch, input => { defaults.push(structuredClone(input)); });
  const control = (value: string) => { const control = new Control(); control.value = value; control.selectionStart = control.selectionEnd = value.length; return control as unknown as HTMLTextAreaElement; };
  const fire = (control: HTMLTextAreaElement, type: string, data: string | null = null, inputType = "insertText") => {
    for (const listener of listeners.get(type) ?? []) listener({ target: control, data, inputType, defaultPrevented: false });
  };
  const replace = (control: HTMLTextAreaElement, from: number, to: number, text: string, inputType = "insertText") => {
    control.selectionStart = from; control.selectionEnd = to; fire(control, "beforeinput", null, inputType);
    control.value = control.value.slice(0, from) + text + control.value.slice(to);
    control.selectionStart = control.selectionEnd = from + text.length;
    fire(control, "input", inputType.startsWith("insert") ? text : null, inputType);
  };
  const session = (owner: AuthorInputOwner, sink?: AuthorInputSink): AuthorInputSession => {
    const capturedAudience = state.audience, capturedEpoch = state.epoch;
    return collector.session(capturedAudience, capturedEpoch, owner, () => state.audience === capturedAudience && state.epoch === capturedEpoch, sink, () => state.lease);
  };
  return { collector, defaults, state, control, fire, replace, session };
}
async function controls(run: (f: ReturnType<typeof setupControls>) => Promise<void>) {
  const names = ["HTMLInputElement", "HTMLTextAreaElement"] as const;
  const descriptors = names.map(name => Object.getOwnPropertyDescriptor(globalThis, name));
  for (const name of names) Object.defineProperty(globalThis, name, { configurable: true, writable: true, value: Control });
  const f = setupControls();
  try { await run(f); }
  finally {
    f.collector.clear();
    names.forEach((name, index) => { const descriptor = descriptors[index]; if (descriptor) Object.defineProperty(globalThis, name, descriptor); else Reflect.deleteProperty(globalThis, name); });
  }
}
function bindCopy(f: ReturnType<typeof setupControls>, recovery: PersonalInputRecovery, copy: PersonalInputRecord) {
  const snapshots: AuthorInputSnapshot[] = [], writes: Promise<unknown>[] = [], failures: unknown[] = [];
  const input = f.control(personalInputText(copy)), seed = recovery.personalCopySeed(audience, copy);
  const sink: AuthorInputSink = snapshot => {
    snapshots.push(structuredClone(snapshot));
    if (snapshot.context.owner.kind === "capture") throw new Error("copy sink cannot own a Core Capture draft");
    const write = recovery.update(snapshot.context.audience, [{
      id: snapshot.id, owner: snapshot.context.owner, slot: PERSONAL_COPY_ONLY_SLOT, path: [],
      producer: { producerId: snapshot.producerId, revision: snapshot.producerRevision },
      origin: { baseLength: 0, pieces: [], detached: [], copyOnly: [{ id: snapshot.id, revision: snapshot.revision, text: snapshot.text }], revision: snapshot.revision },
    }]);
    writes.push(write); void write.catch(error => { failures.push(error); });
  };
  const session = f.session(copy.owner, sink), before = session.inspect();
  session.bind(input, PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed });
  return { input, seed, session, before, snapshots, writes, failures, drain: () => Promise.all(writes), current: () => recovery.list(audience).find(record => record.id === copy.id)! };
}

test("a personal copy retains only confirmed personal text and leaves the original input and operation unchanged", async () => {
  const f = await prepare(), original = structuredClone(f.original), copy = await f.recovery.createPersonalCopy(audience, f.original);
  assert.notEqual(copy.id, original.id); assert.deepEqual(copy.owner, original.owner); assert.equal(isPersonalCopy(copy), true);
  assert.equal(personalInputText(copy), f.text); assert.ok(!personalInputText(copy).includes("CANDIDATE_ONLY_NOT_A_COPY_SEED"));
  assert.equal("baseHash" in copy, false); assert.equal("candidate" in copy, false); assert.equal("pending" in copy, false);
  assert.equal(copy.origin.baseLength, 0); assert.deepEqual(copy.origin.pieces, []); assert.deepEqual(copy.origin.detached, []);
  assert.deepEqual(copy.origin.copyOnly, [{ id: copy.id, revision: 0, text: f.text }]);
  assert.deepEqual(f.recovery.current(audience, original), original);
  const persisted = await reopen(f.backend); assert.deepEqual(persisted.current(audience, original), original); assert.deepEqual(persisted.current(audience, copy), copy);
  const before = structuredClone(f.backend.data);
  assert.throws(() => f.recovery.update(audience, [{ ...copy, slot: undefined }]), /原件保留/);
  assert.throws(() => f.recovery.update(audience, [{ ...copy, baseHash: "a".repeat(64) }]), /原件保留/);
  assert.throws(() => f.recovery.update(audience, [{ ...copy, candidate: original.candidate, pending: original.pending }]), /原件保留/);
  assert.throws(() => f.recovery.update(audience, [{ ...copy, id: "invented-copy" }]), /原件保留/);
  assert.deepEqual(f.backend.data, before); assert.deepEqual(f.recovery.current(audience, original), original);
});

test("an empty copy keeps its empty entry and can reopen as a personal editing target", async () => controls(async f => {
  const prepared = await prepare(""), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original);
  assert.deepEqual(copy.origin.copyOnly, [{ id: copy.id, revision: 0, text: "" }]); assert.equal(hasPersonalInput(copy.origin), true);
  const persisted = await reopen(prepared.backend), restored = persisted.list(audience).find(record => record.id === copy.id)!;
  const binding = bindCopy(f, persisted, restored); assert.equal(binding.input.value, ""); assert.deepEqual(binding.session.inspect(), binding.before);
  f.replace(binding.input, 0, 0, "第一段😀"); await binding.drain(); assert.equal(personalInputText(binding.current()), "第一段😀");
  assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));

test("seeded insertion and deletion retain the whole edited copy rather than only later new characters", async () => controls(async f => {
  const prepared = await prepare("完整种子😀"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  f.replace(binding.input, 2, 2, "新增"); await binding.drain(); assert.equal(personalInputText(binding.current()), "完整新增种子😀");
  f.replace(binding.input, 0, 2, "", "deleteContentBackward"); await binding.drain(); assert.equal(personalInputText(binding.current()), "新增种子😀");
  assert.equal(binding.snapshots.at(-1)!.text, "新增种子😀"); assert.equal(binding.snapshots.at(-1)!.id, copy.id);
  assert.equal(binding.session.inspect().unapplied, true, "transport acknowledgement cannot apply the copy owner");
  const persisted = await reopen(prepared.backend); assert.equal(personalInputText(persisted.list(audience).find(record => record.id === copy.id)!), "新增种子😀");
  assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));

test("deleting the whole seeded copy preserves copyOnly[0] through capture, acknowledgement and reopen", async () => controls(async f => {
  const prepared = await prepare("全部删除😀"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  f.replace(binding.input, 0, binding.input.value.length, "", "deleteContentBackward"); await binding.drain();
  const current = binding.current(); assert.equal(current.origin.copyOnly!.length, 1); assert.equal(current.origin.copyOnly![0].text, ""); assert.equal(hasPersonalInput(current.origin), true);
  const lease = captureAuthorInput(binding.input); assert.ok(lease); lease.release();
  const checkpoint = binding.session.inspect(); assert.equal(binding.session.freeze(checkpoint), true); await prepared.recovery.flush(audience);
  assert.equal(binding.session.acknowledge(checkpoint, "applied"), true); binding.session.retire();
  const persisted = await reopen(prepared.backend), empty = persisted.list(audience).find(record => record.id === copy.id)!;
  assert.equal(isPersonalCopy(empty), true); assert.equal(empty.origin.copyOnly!.length, 1); assert.equal(empty.origin.copyOnly![0].text, ""); assert.equal("baseHash" in empty, false);
  assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));

test("an untouched durable seed does not become unapplied and explicit Finish preserves all of it", async () => controls(async f => {
  const prepared = await prepare("未改动的完整本人文字😀"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  assert.deepEqual(binding.session.inspect(), binding.before); assert.deepEqual(binding.snapshots, []); assert.equal(binding.session.inspect().hasObservedChanges, false);
  const lease = binding.session.capture(binding.input)!; assert.ok(lease); assert.equal(binding.session.freeze(binding.session.inspect()), false); lease.release();
  const checkpoint = binding.session.inspect(); assert.equal(binding.session.freeze(checkpoint), true); assert.equal(binding.input.readOnly, true);
  await prepared.recovery.flush(audience); assert.equal(personalInputText(prepared.recovery.current(audience, copy)), prepared.text);
  assert.equal(binding.session.acknowledge(checkpoint, "applied"), true); binding.session.retire(); assert.equal(f.collector.inspect().producers.length, 0);
  const persisted = await reopen(prepared.backend); assert.equal(personalInputText(persisted.current(audience, copy)), prepared.text); assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));

test("observed composition replacements retain the seed and the full final personal copy", async () => controls(async f => {
  const prepared = await prepare("前缀😀后缀"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  binding.input.selectionStart = 4; binding.input.selectionEnd = 4; f.fire(binding.input, "compositionstart");
  binding.input.value = "前缀😀中后缀"; f.fire(binding.input, "input", "中", "insertCompositionText"); await binding.drain();
  assert.equal(personalInputText(binding.current()), "前缀😀中后缀"); assert.equal(binding.session.freeze(binding.session.inspect()), false);
  binding.input.value = "前缀😀中文后缀"; f.fire(binding.input, "compositionend", "中文"); f.fire(binding.input, "input", "中文", "insertCompositionText"); await binding.drain();
  assert.equal(personalInputText(binding.current()), "前缀😀中文后缀"); assert.equal(binding.session.inspect().nativePending, false);
  const checkpoint = binding.session.inspect(); assert.equal(binding.session.freeze(checkpoint), true); await prepared.recovery.flush(audience);
  assert.equal(binding.session.acknowledge(checkpoint, "applied"), true); binding.session.retire();
  const persisted = await reopen(prepared.backend); assert.equal(personalInputText(persisted.list(audience).find(record => record.id === copy.id)!), "前缀😀中文后缀"); assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));

test("finishing the copy cannot acknowledge the original unapplied form producer", async () => controls(async f => {
  const prepared = await prepare("OWN"), originalControl = f.control("BASE"), originalSession = f.session(originalOwner);
  originalSession.bind(originalControl, "original-note"); f.replace(originalControl, 4, 4, "OWN"); const originalCheckpoint = originalSession.inspect();
  const copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  f.replace(binding.input, binding.input.value.length, binding.input.value.length, " copy"); await binding.drain();
  const checkpoint = binding.session.inspect(); assert.notEqual(checkpoint.producerId, originalCheckpoint.producerId); assert.equal(binding.session.freeze(checkpoint), true);
  await prepared.recovery.flush(audience); assert.equal(binding.session.acknowledge(checkpoint, "applied"), true); binding.session.retire();
  assert.deepEqual(originalSession.inspect(), originalCheckpoint); assert.equal(originalSession.inspect().unapplied, true);
  assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
}));

test("only a current durable opaque seed may bind the exact personal copy owner and slot", async () => controls(async f => {
  const prepared = await prepare("准确种子"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), seed = prepared.recovery.personalCopySeed(audience, copy);
  const sink: AuthorInputSink = () => { throw new Error("rejected bindings cannot publish"); }, original = structuredClone(prepared.backend.data);
  assert.throws(() => f.session(copy.owner, sink).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: {} as PersonalCopySeed }));
  assert.throws(() => f.session({ kind: "body", resource: "Other", reference: originalOwner.reference }, sink).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  assert.throws(() => f.session({ kind: "body", resource: originalOwner.resource }, sink).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  assert.throws(() => f.session(copy.owner, sink).bind(f.control("准确种子"), "ordinary-field", [], { personalCopy: seed }));
  assert.throws(() => f.session(copy.owner, sink).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [{ kind: "member", key: "value" }], { personalCopy: seed }));
  assert.throws(() => f.session(copy.owner).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  assert.throws(() => f.session(copy.owner, sink).bind(f.control("PROGRAMMATIC_BASE"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  f.state.audience = otherAudience; f.state.epoch++;
  assert.throws(() => f.session(copy.owner, sink).bind(f.control("准确种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  assert.deepEqual(f.defaults, []); assert.deepEqual(prepared.backend.data, original); assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
}));

test("an unacknowledged copy and an older sequence cannot mint or reuse an initial-text proof", async () => controls(async f => {
  const prepared = await prepare("已确认种子"), gate = deferred(); prepared.backend.wait = gate.promise;
  const creating = prepared.recovery.createPersonalCopy(audience, prepared.original), pending = prepared.recovery.list(audience).find(isPersonalCopy)!;
  assert.ok(pending); assert.throws(() => prepared.recovery.personalCopySeed(audience, pending));
  gate.resolve(); const copy = await creating; prepared.backend.wait = undefined; const seed = prepared.recovery.personalCopySeed(audience, copy);
  await prepared.recovery.update(audience, [{ ...copy, origin: { ...copy.origin, revision: 1, copyOnly: [{ id: copy.id, revision: 1, text: "更新的副本" }] } }]);
  assert.throws(() => f.session(copy.owner, () => {}).bind(f.control("已确认种子"), PERSONAL_COPY_ONLY_SLOT, [], { personalCopy: seed }));
  const current = prepared.recovery.list(audience).find(record => record.id === copy.id)!; const binding = bindCopy(f, prepared.recovery, current); assert.equal(binding.input.value, "更新的副本");
  assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
}));

test("programmatic secret resets stay protected even after the initial personal copy was seeded", async () => controls(async f => {
  const prepared = await prepare("已知本人文字"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  binding.input.value = "PROGRAMMATIC_SECRET_NEVER_OWNED"; const end = binding.input.value.length; f.replace(binding.input, end, end, "新输入"); await binding.drain();
  assert.equal(personalInputText(binding.current()), "已知本人文字\n新输入"); assert.ok(!personalInputText(binding.current()).includes("PROGRAMMATIC_SECRET_NEVER_OWNED"));
  assert.ok(binding.snapshots.at(-1)!.rawValue.includes("PROGRAMMATIC_SECRET_NEVER_OWNED"));
  assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
}));

test("retired composition and another actor cannot append late text to the original personal copy", async () => controls(async f => {
  const prepared = await prepare("原副本"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy);
  f.fire(binding.input, "compositionstart"); binding.input.value = "原副本中"; f.fire(binding.input, "input", "中", "insertCompositionText"); await binding.drain();
  const last = structuredClone(binding.current()), deliveries = binding.snapshots.length; binding.session.retire(); f.state.audience = otherAudience; f.state.epoch++;
  binding.input.value = "原副本中LATE_SECRET"; f.fire(binding.input, "compositionend", "LATE_SECRET"); f.fire(binding.input, "input", "LATE_SECRET"); await binding.drain();
  assert.equal(binding.snapshots.length, deliveries); assert.deepEqual(binding.current(), last); assert.equal(binding.session.inspect().nativePending, true);
  assert.equal(binding.session.acknowledge(binding.session.inspect(), "applied"), false); assert.equal(f.collector.freeze(f.collector.inspect()), false);
  await prepared.recovery.open(otherAudience); assert.deepEqual(prepared.recovery.list(otherAudience), []); assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
}));

test("a failed fork keeps the original durable, retains the new RAM copy and cannot issue a seed before acknowledgement", async () => {
  const prepared = await prepare("原件必须保留😀"), durable = structuredClone(prepared.backend.data); prepared.backend.fail = true;
  await assert.rejects(prepared.recovery.createPersonalCopy(audience, prepared.original), /quota/);
  const copy = prepared.recovery.list(audience).find(isPersonalCopy)!; assert.ok(copy); assert.equal(personalInputText(copy), prepared.text); assert.equal(prepared.recovery.pending(audience), true);
  assert.throws(() => prepared.recovery.personalCopySeed(audience, copy)); assert.deepEqual(prepared.backend.data, durable); assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
  prepared.backend.fail = false; await prepared.recovery.retry(audience); assert.ok(prepared.recovery.personalCopySeed(audience, copy));
  const persisted = await reopen(prepared.backend); assert.deepEqual(persisted.current(audience, prepared.original), prepared.original); assert.equal(personalInputText(persisted.current(audience, copy)), prepared.text);
});

test("copy edit quota failures retain full RAM input and both close barriers without applying either owner", async () => controls(async f => {
  const prepared = await prepare("原始本人副本"), copy = await prepared.recovery.createPersonalCopy(audience, prepared.original), binding = bindCopy(f, prepared.recovery, copy), durable = structuredClone(prepared.backend.data);
  prepared.backend.fail = true; f.replace(binding.input, binding.input.value.length, binding.input.value.length, " 未持久输入😀");
  await assert.rejects(binding.drain(), /quota/); assert.equal(personalInputText(binding.current()), "原始本人副本 未持久输入😀");
  assert.deepEqual(prepared.backend.data, durable); assert.equal(prepared.recovery.pending(audience), true); assert.equal(binding.session.inspect().unapplied, true); assert.equal(binding.session.inspect().ackRevision, binding.before.ackRevision);
  assert.deepEqual(prepared.recovery.current(audience, prepared.original), prepared.original);
  prepared.backend.fail = false; await prepared.recovery.retry(audience); assert.equal(binding.session.inspect().unapplied, true, "a retry ACK is still not the owner's Finish action");
  const persisted = await reopen(prepared.backend); assert.equal(personalInputText(persisted.list(audience).find(record => record.id === copy.id)!), "原始本人副本 未持久输入😀"); assert.deepEqual(persisted.current(audience, prepared.original), prepared.original);
}));
