import assert from "node:assert/strict";
import test from "node:test";
import { PersonalControlInput, type PersonalControlContext } from "./personal-control-input";

test("form ownership rejects programmatic resets, stale composition scope and missing edit evidence", () => {
  class Control { value = "A"; dataset = {}; isConnected = true; readOnly = false; disabled = false; id = ""; type = "text"; selectionStart = 1; selectionEnd = 1; }
  const previous = [globalThis.HTMLInputElement, globalThis.HTMLTextAreaElement];
  Object.assign(globalThis, { HTMLInputElement: Control, HTMLTextAreaElement: Control });
  const listeners = new Map<string, ((event: unknown) => void)[]>(), document = { addEventListener(type: string, listener: (event: unknown) => void) { listeners.set(type, [...listeners.get(type) ?? [], listener]); } };
  const fire = (control: Control, type: string, data?: string, inputType = "insertText") => { for (const listener of listeners.get(type) ?? []) listener({ target: control, data, inputType }); };
  let context: PersonalControlContext = { audience: "actor", resource: "one", epoch: 1 }; const saved: { context: PersonalControlContext; text: string }[] = [];
  try {
    new PersonalControlInput(document as unknown as Document, () => context, (context, _id, text) => saved.push({ context, text }));
    const control = new Control(); fire(control, "compositionstart"); control.value = "ASECRETX"; fire(control, "compositionend", "X"); assert.equal(saved.at(-1)?.text, "X");
    assert.ok(saved.every(item => !item.text.includes("SECRET")));
    const normal = new Control(); fire(normal, "compositionstart"); normal.value = "A中文"; fire(normal, "input", "中文", "insertCompositionText"); fire(normal, "compositionend", "中文"); assert.equal(saved.at(-1)?.text, "中文");
    const stale = new Control(); fire(stale, "compositionstart"); const count = saved.length; context = { ...context, resource: "two" }; stale.value = "ABSECRET"; fire(stale, "compositionend", "BSECRET"); assert.equal(saved.length, count);
    const first = new Control(); first.value = "BASEX"; fire(first, "input", "X"); assert.equal(saved.at(-1)?.text, "X");
    const reset = new Control(); fire(reset, "beforeinput"); reset.value = "AA"; fire(reset, "input", "A"); reset.value = "AAPROTECTED"; reset.selectionStart = reset.selectionEnd = reset.value.length; fire(reset, "beforeinput"); reset.value += "Z"; fire(reset, "input", "Z"); assert.equal(saved.at(-1)?.text, "A\nZ");
  } finally { Object.assign(globalThis, { HTMLInputElement: previous[0], HTMLTextAreaElement: previous[1] }); }
});
