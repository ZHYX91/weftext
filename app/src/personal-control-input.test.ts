import assert from "node:assert/strict";
import test from "node:test";
import { PersonalControlInput } from "./personal-control-input";
import { captureAuthorInput, type AuthorInputSnapshot, type AuthorInputOwner } from "./author-input";
import { captureAuthorInput as pureCaptureInput, captureString, captureText, captureTextSlots, emptyCaptureInput, observeCaptureText } from "./capture-draft-state";

function fixture(run: (f: ReturnType<typeof setup>) => void) {
  class Control { value = "A"; dataset = {}; isConnected = true; readOnly = false; disabled = false; id = ""; type = "text"; selectionStart = 1; selectionEnd = 1; matches() { return false; } }
  const previous = [globalThis.HTMLInputElement, globalThis.HTMLTextAreaElement]; Object.assign(globalThis, { HTMLInputElement: Control, HTMLTextAreaElement: Control });
  function setup() {
    const listeners = new Map<string, ((event: unknown) => void)[]>(), document = { addEventListener(type: string, listener: (event: unknown) => void) { listeners.set(type, [...listeners.get(type) ?? [], listener]); } };
    const saved: AuthorInputSnapshot[] = []; let epoch = 1, audience = "alice", resource = "one", producerRevision = 0;
    const collector = new PersonalControlInput(document as unknown as Document, context => context.epoch === epoch && context.audience === audience, input => { saved.push(structuredClone(input)); });
    const session = (owner: AuthorInputOwner = { kind: "body", resource }) => { const selected = resource; return collector.session(audience, epoch, owner, () => resource === selected, undefined, () => producerRevision); };
    const control = (value = "A") => { const c = new Control(); c.value = value; c.selectionStart = c.selectionEnd = value.length; return c as unknown as HTMLInputElement; };
    const fire = (control: HTMLInputElement, type: string, data?: string, inputType = "insertText") => { for (const listener of listeners.get(type) ?? []) listener({ target: control, data, inputType }); };
    const type = (c: HTMLInputElement, text: string) => { fire(c, "beforeinput"); c.value = c.value.slice(0, c.selectionStart!) + text + c.value.slice(c.selectionEnd!); c.selectionStart = c.selectionEnd = c.value.length; fire(c, "input", text); };
    return { saved, collector, session, control, fire, type, actor(next: string) { audience = next; epoch++; }, resource(next: string) { resource = next; }, nextRevision() { producerRevision++; } };
  }
  try { run(setup()); } finally { Object.assign(globalThis, { HTMLInputElement: previous[0], HTMLTextAreaElement: previous[1] }); }
}
test("unregistered document search, filters and temporary parameters never enter personal recovery", () => fixture(f => { const search = f.control("PRIVATE"); f.type(search, "query"); assert.deepEqual(f.saved, []); }));
test("only positive actual field owner is retained even when another body is selected", () => fixture(f => { const input = f.control("PRIVATE"); f.session({ kind: "field", resource: "field-B", fieldId: "actual" }).bind(input, "value", [{ kind: "member", key: "name" }]); f.type(input, "X"); assert.equal(f.saved[0].text, "X"); assert.deepEqual(f.saved[0].context.owner, { kind: "field", resource: "field-B", fieldId: "actual" }); assert.ok(!f.saved[0].text.includes("PRIVATE")); }));
test("programmatic reset protects new baseline while retaining only proved old A and new X", () => fixture(f => { const input = f.control(""); f.session().bind(input, "body"); f.type(input, "A"); input.value = "ASECRET"; input.selectionStart = input.selectionEnd = 7; f.type(input, "X"); assert.equal(f.saved.at(-1)?.text, "A\nX"); }));
test("first input without beforeinput retains event payload only", () => fixture(f => { const input = f.control("BASE"); f.session().bind(input, "body"); input.value = "BASEX"; f.fire(input, "input", "X"); assert.equal(f.saved.at(-1)?.text, "X"); }));
test("same-value exact replacement still proves fresh author payload", () => fixture(f => { const input = f.control("PRIVATE"); f.session().bind(input, "body"); input.selectionStart = 0; input.selectionEnd = 3; f.fire(input, "beforeinput"); f.fire(input, "input", "PRI"); assert.equal(f.saved.at(-1)?.text, "PRI"); }));
test("normal IME proves observed text and unexpected programmatic composition value stays protected", () => fixture(f => { const input = f.control("BASE"); f.session().bind(input, "body"); f.fire(input, "compositionstart"); input.value = "BASE中文"; f.fire(input, "input", "中文", "insertCompositionText"); f.fire(input, "compositionend", "中文"); assert.equal(f.saved.at(-1)?.text, "中文"); const other = f.control("A"); f.session().bind(other, "other"); f.fire(other, "compositionstart"); other.value = "ASECRETX"; f.fire(other, "compositionend", "X"); assert.equal(f.saved.at(-1)?.text, "X"); }));
test("same DOM A-to-B-to-A during composition is retired through terminal events", () => fixture(f => { const input = f.control(), first = f.session(); first.bind(input, "one"); f.fire(input, "compositionstart"); first.retire(); f.resource("two"); assert.throws(() => f.session().bind(input, "two"), /旧组合/); f.resource("one"); assert.throws(() => f.session().bind(input, "one"), /旧组合/); input.value = "ASECRET"; f.fire(input, "compositionend", "SECRET"); f.fire(input, "input", "SECRET"); assert.deepEqual(f.saved, []); }));
test("clipboard lease captured before await rejects retired binding and equal-byte owner ABA", () => fixture(f => { const input = f.control(), session = f.session(); session.bind(input, "one"); const lease = captureAuthorInput(input)!; session.invalidate("one"); assert.throws(() => session.bind(input, "one"), /新控件/); session.bind(f.control(), "one"); assert.equal(lease.apply("SECRET"), false); assert.equal(input.value, "A"); assert.deepEqual(f.saved, []); }));
test("clipboard lease rejects newer input and producer revision, accepting only original live target", () => fixture(f => { const input = f.control(), session = f.session(); session.bind(input, "one"); const old = captureAuthorInput(input)!; f.nextRevision(); assert.equal(old.apply("OLD"), false); const next = captureAuthorInput(input)!; f.type(input, "X"); assert.equal(next.apply("OLD"), false); const current = captureAuthorInput(input)!; assert.equal(current.apply("PASTE"), true); assert.equal(f.saved.at(-1)?.text, "X\nPASTE"); }));
test("harmless repaint preserves proposal slot ID and previous proved ownership", () => fixture(f => { const session = f.session(), input = f.control(); session.bind(input, "one"); f.type(input, "X"); const first = f.saved.at(-1)!; Object.defineProperty(input, "isConnected", { value: false }); const next = f.control("AX"); session.bind(next, "one"); f.type(next, "Y"); assert.equal(f.saved.at(-1)?.id, first.id); assert.equal(f.saved.at(-1)?.text, "XY"); }));
test("password, disabled and old actor controls never publish even if mistakenly registered", () => fixture(f => { for (const type of ["password", "search", "hidden"]) { const input = f.control(); input.type = type; f.session().bind(input, type); f.type(input, "SECRET"); } const input = f.control(); f.session().bind(input, "old"); f.actor("bob"); f.type(input, "OLD"); assert.deepEqual(f.saved, []); }));
test("capture owner requires its own Core sink rather than borrowing current body recovery", () => fixture(f => { assert.throws(() => f.session({ kind: "capture", draftId: "actual" }), /Core/); }));

test("a completed composition still requires a new DOM node for another owner before its terminal input", () => fixture(f => { const input = f.control(), first = f.session(); first.bind(input, "one"); f.fire(input, "compositionstart"); input.value = "A中文"; f.fire(input, "compositionend", "中文"); const count = f.saved.length; input.value = "PROTECTED-B"; assert.throws(() => f.session({kind:"body",resource:"B"}).bind(input, "other"), /新控件/); f.fire(input, "input", "中文"); assert.equal(f.saved.length, count); }));

test("retired unapplied producer remains visible until its exact owner action acknowledges it", () => fixture(f => {
  const input = f.control("PRIVATE"), session = f.session({ kind: "library", role: "schema" }); session.bind(input, "source");
  const old = session.inspect(); f.type(input, "OWN"); const current = session.inspect();
  assert.equal(current.unapplied, true); assert.equal(current.hasObservedChanges, true);
  assert.equal(session.acknowledge({ producerId: "another", revision: current.revision }, "applied"), false);
  assert.equal(session.acknowledge(old, "applied"), true); assert.equal(session.inspect().unapplied, true);
  session.retire();
  const retained = f.collector.inspect().producers.find(item => item.producerId === current.producerId)!;
  assert.equal(retained.retired, true); assert.equal(retained.unapplied, true);
  assert.equal(session.acknowledge(current, "applied"), true);
  assert.equal(f.collector.inspect().producers.length, 0);
}));
test("collection freeze compares the complete actual producer set and stops real controls", () => fixture(f => {
  const first = f.session(), input = f.control(); first.bind(input, "one");
  const incomplete = f.collector.inspect(); const second = f.session({ kind: "library", role: "style" }); second.bind(f.control(), "source");
  assert.equal(f.collector.freeze(incomplete), false);
  const exact = f.collector.inspect(); assert.equal(f.collector.freeze(exact), true); assert.equal(input.readOnly, true);
  assert.throws(() => f.session(), /冻结/); assert.throws(() => first.bind(f.control(), "late"), /冻结/);
  f.collector.thaw(); assert.equal(input.readOnly, false);
}));
test("pending native input and clipboard deliveries cannot pass the producer freeze", () => fixture(f => {
  const session = f.session(), input = f.control(); session.bind(input, "one");
  f.fire(input, "compositionstart"); assert.equal(session.inspect().nativePending, true); assert.equal(session.freeze(session.inspect()), false);
  f.fire(input, "compositionend", "");
  const lease = session.capture(input)!; assert.equal(session.inspect().pendingDeliveries, 1);
  assert.equal(session.freeze(session.inspect()), false); lease.release(); assert.equal(session.inspect().pendingDeliveries, 0);
  assert.equal(session.freeze(session.inspect()), true); assert.equal(lease.apply("LATE"), false); assert.equal(input.readOnly, true);
  session.thaw(); assert.equal(input.readOnly, false);
}));
test("retirement during IME keeps an explicit pending producer without retaining old DOM/base", () => fixture(f => {
  const session = f.session(), input = f.control("PRIVATE"); session.bind(input, "one"); f.fire(input, "compositionstart"); session.retire();
  const state = session.inspect(); assert.equal(state.retired, true); assert.equal(state.nativePending, true);
  assert.equal(f.collector.freeze(f.collector.inspect()), false);
  assert.equal(session.acknowledge(state, "applied"), false);
  assert.equal(session.acknowledge(state, "discarded"), true); assert.equal(f.collector.inspect().producers.length, 0);
}));
test("actual choices track unapplied intent but only an explicit capture sink receives chosen raw", () => fixture(f => {
  const field = f.session({ kind: "field", resource: "other", fieldId: "tasks/enabled" }), input = f.control(); input.type = "checkbox"; field.bindChoice(input, "enabled");
  input.checked = true; f.fire(input, "change"); assert.equal(field.inspect().unapplied, true); assert.equal(f.saved.length, 0);
  const capture = f.collector.session("alice", 1, { kind: "capture", draftId: crypto.randomUUID() }, () => true, value => f.saved.push(value));
  const choice = f.control(); choice.type = "checkbox"; capture.bindChoice(choice, "enabled"); choice.checked = true; f.fire(choice, "change");
  assert.equal(f.saved[0].kind, "choice"); assert.equal(f.saved[0].text, ""); assert.equal(f.saved[0].rawValue, "true");
}));

test("all six Capture text roles keep programmatic defaults private and retain actual proofs across producer/IME changes", () => fixture(f => {
  for(const slot of captureTextSlots) {
    const input=emptyCaptureInput(), draftId=crypto.randomUUID();
    input[slot]=captureText("PRIVATE DEFAULT");
    assert.deepEqual(pureCaptureInput(input).texts[slot],[]);
    const sink=(snapshot:AuthorInputSnapshot)=>{if(snapshot.kind==="text")observeCaptureText(input,slot,snapshot);};
    const first=f.collector.session("alice",1,{kind:"capture",draftId},()=>true,sink);
    const control=f.control("PRIVATE DEFAULT");first.bind(control,slot);f.type(control,"X\ud800\0");
    const original=structuredClone(input.authorTexts[slot][0]);
    assert.equal(captureString(original.text),"X\ud800\0");
    assert.match(captureString(input[slot]),/PRIVATE DEFAULT/);
    control.value="PROGRAM RESTORE";control.selectionStart=control.selectionEnd=control.value.length;f.type(control,"Y");
    assert.doesNotMatch(JSON.stringify(pureCaptureInput(input)),/PRIVATE DEFAULT|PROGRAM RESTORE/);
    const retiredProof=structuredClone(input.authorTexts[slot][0]);first.retire();
    const second=f.collector.session("alice",1,{kind:"capture",draftId},()=>true,sink), next=f.control(control.value);second.bind(next,slot);f.type(next,"Z");
    assert.equal(input.authorTexts[slot].length,2);assert.deepEqual(input.authorTexts[slot][0],retiredProof);
    assert.equal(captureString(input.authorTexts[slot][1].text),"Z");
    // A restored full candidate is protected on a new IME producer as well.
    second.retire();const third=f.collector.session("alice",1,{kind:"capture",draftId},()=>true,sink), ime=f.control("PRIVATE IME");third.bind(ime,slot);
    f.fire(ime,"compositionstart");ime.value="PRIVATE IME中";f.fire(ime,"input","中","insertCompositionText");f.fire(ime,"compositionend","中");
    assert.equal(captureString(input.authorTexts[slot][2].text),"中");
    const restored=JSON.parse(JSON.stringify(input)) as typeof input;
    assert.deepEqual(pureCaptureInput(restored).texts[slot],input.authorTexts[slot]);
    assert.doesNotMatch(JSON.stringify(pureCaptureInput(restored)),/PRIVATE|PROGRAM/);
    third.retire();
  }
}));
