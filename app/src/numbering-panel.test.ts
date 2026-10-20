import assert from "node:assert/strict";
import test from "node:test";
import { NumberingController, type HeadingNumberingPreview, type NumberingHost } from "./numbering-panel";
function storage() { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; }
function host(overrides: Partial<NumberingHost> = {}): NumberingHost { return { source: () => "= 一、原有前缀", inspection: () => ({ sourceHash: "current", settings: null, editable: true, reason: null }), preview: async () => ({ beforeSource: "= 一、原有前缀", afterSource: "#set heading(numbering: \"1.1\")\n= 一、原有前缀" }), apply: async () => true, personalDefaultKey: "user:default", personalNoteKey: "user:note", displayPersonal: () => {}, ...overrides }; }
test("personal display and default changes never call author preview or apply", () => {
  const retained = storage(); let display = false;
  const controller = new NumberingController(host({ preview: async () => { throw new Error("author preview forbidden"); }, apply: async () => { throw new Error("author apply forbidden"); }, displayPersonal: value => { display = value; } }), retained);
  controller.setPersonal("show"); assert.equal(display, true); controller.setPersonalDefault(false); assert.equal(display, true); controller.setPersonal("default"); assert.equal(display, false); assert.equal(retained.getItem("user:note"), null);
});
test("late source changes and stale prepared inputs cannot modify the draft", async () => {
  let source = "= 一、原有前缀", release: (preview: HeadingNumberingPreview) => void = () => {};
  const controller = new NumberingController(host({ source: () => source, preview: async () => new Promise(resolve => { release = resolve; }) }), storage());
  const preparing = controller.prepare(); source += " changed"; release({ beforeSource: "= 一、原有前缀", afterSource: "new" }); await assert.rejects(preparing, /变化/); assert.equal(controller.preview, null);
  await assert.rejects(controller.apply(), /预览/);
});
test("unknown native numbering preserves source and invalidates document controls", async () => {
  const controller = new NumberingController(host({ inspection: () => ({ sourceHash: "unknown", settings: null, editable: false, reason: "custom-native-numbering-rule" }) }), storage());
  assert.throws(() => controller.change({ scheme: "chinese", start: 1, maxLevel: 3, excludeLabels: [] }), /原文/);
  await assert.rejects(controller.prepare(), /不能/);
  controller.setPersonal("show"); assert.equal(controller.personalVisible, true);
});
test("invalid edited input invalidates the previous valid preview before validation", async () => {
  const controller = new NumberingController(host(), storage()); await controller.prepare(); assert.ok(controller.preview);
  assert.throws(() => controller.change({ scheme: "decimal", start: 0, maxLevel: 6, excludeLabels: [] }));
  assert.equal(controller.preview, null); await assert.rejects(controller.apply(), /预览/);
  await assert.rejects(controller.prepare(), /不能/);
});
test("surface observations clear old source previews on revocation and bind to the original note", async () => {
  let readable = true, binding = "note-a", listener = () => {};
  const controller = new NumberingController(host({ readable: () => readable, binding: () => binding, observeState: (_element, callback) => { listener = callback; return () => { listener = () => {}; }; } }), storage());
  const stop = controller.observeSurface({} as HTMLElement, () => {}); await controller.prepare(); assert.ok(controller.preview);
  readable = false; listener(); assert.equal(controller.preview, null); assert.equal(controller.available, false); await assert.rejects(controller.apply());
  readable = true; binding = "note-b"; listener(); assert.equal(controller.available, false); stop();
});
test("composition prevents author preparation and application", async () => {
  let composing = false;
  const controller = new NumberingController(host({ composing: () => composing }), storage()); await controller.prepare();
  composing = true; await assert.rejects(controller.apply()); await assert.rejects(controller.prepare());
});
test("endnotes awaiting module qualification can propose but cannot apply a rejected preview", async () => {
  let accepted = false, applies = 0;
  const controller = new NumberingController(host({ inspection: () => ({ sourceHash: "current", settings: null, editable: false, reason: "requires-frozen-endnotes-module" }), preview: async () => { if (!accepted) throw new Error("本篇尾注模块核对未通过"); return { beforeSource: "= 一、原有前缀", afterSource: "qualified native source" }; }, apply: async () => { applies++; return true; } }), storage());
  controller.change({ scheme: "decimal", start: 3, maxLevel: 6, excludeLabels: [] });
  await assert.rejects(controller.prepare(), /模块核对/); assert.equal(controller.preview, null);
  await assert.rejects(controller.apply(), /预览/); assert.equal(applies, 0);
  accepted = true; await controller.prepare(); assert.ok(controller.preview); assert.equal(await controller.apply(), true); assert.equal(applies, 1);
});
