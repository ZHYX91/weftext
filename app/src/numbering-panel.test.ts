import { locale, message, UiError, uiTextValue } from "./ui-locale";
import { TestDocument, TestElement } from "./dom.test-support";
import assert from "node:assert/strict";
import test from "node:test";
import { decodeHeadingNumberingInspection, openNumberingPanel, NumberingController, type HeadingNumberingPreview, type NumberingHost } from "./numbering-panel";
function storage() { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; }
function host(overrides: Partial<NumberingHost> = {}): NumberingHost { return { source: () => "= 一、原有前缀", inspection: () => ({ sourceHash: "current", settings: null, editable: true, frozenEndnotesRequired: false, reason: null }), preview: async () => ({ beforeSource: "= 一、原有前缀", afterSource: "#set heading(numbering: \"1.1\")\n= 一、原有前缀" }), apply: async () => true, personalDefaultKey: "user:default", personalNoteKey: "user:note", displayPersonal: () => {}, ...overrides }; }
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
  const controller = new NumberingController(host({ inspection: () => ({ sourceHash: "unknown", settings: null, editable: false, frozenEndnotesRequired: false, reason: { explanation: message("common.unavailable"), diagnostics: [], causes: [] } }) }), storage());
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
  const controller = new NumberingController(host({ inspection: () => ({ sourceHash: "current", settings: null, editable: false, frozenEndnotesRequired: true, reason: { explanation: message("common.unavailable"), diagnostics: [], causes: [] } }), preview: async () => { if (!accepted) throw new Error("本篇尾注模块核对未通过"); return { beforeSource: "= 一、原有前缀", afterSource: "qualified native source" }; }, apply: async () => { applies++; return true; } }), storage());
  controller.change({ scheme: "decimal", start: 3, maxLevel: 6, excludeLabels: [] });
  await assert.rejects(controller.prepare(), /模块核对/); assert.equal(controller.preview, null);
  await assert.rejects(controller.apply(), /预览/); assert.equal(applies, 0);
  accepted = true; await controller.prepare(); assert.ok(controller.preview); assert.equal(await controller.apply(), true); assert.equal(applies, 1);
});

test("numbering admission uses the required producer boolean independently of translated reasons", async () => {
  const raw = { sourceHash: "current", settings: null, editable: false, frozenEndnotesRequired: true, reason: { explanation: message("common.close"), diagnostics: [], causes: [] } };
  const decoded = decodeHeadingNumberingInspection(JSON.parse(JSON.stringify(raw)));
  const controller = new NumberingController(host({ inspection: () => decoded }), storage()); assert.equal(controller.canPropose, true); await controller.prepare();
  for (const invalid of [{ ...raw, frozenEndnotesRequired: undefined }, { ...raw, frozenEndnotesRequired: "true" }, { ...raw, editable: true }, { ...raw, reason: "old reason" }]) assert.throws(() => decodeHeadingNumberingInspection(invalid));
  const blocked = new NumberingController(host({ inspection: () => ({ ...decoded, frozenEndnotesRequired: false, reason: { explanation: message("numberingPanel.endnoteModulePreviewHint"), diagnostics: [], causes: [] } }) }), storage()); assert.equal(blocked.canPropose, false);
});

test("the actual numbering dialog keeps author input while errors and ARIA follow locale and retire on close", async () => {
  const names = ["document", "HTMLElement", "localStorage"] as const, saved = names.map(name => Object.getOwnPropertyDescriptor(globalThis, name)), previousLocale = locale.current, document = new TestDocument(), retained = storage(), create = document.createElement.bind(document);
  document.createElement = tag => {
    const node = create(tag);
    if (tag === "dialog") { const listeners = new Map<string, (() => void)[]>(); Object.assign(node, { addEventListener(type: string, listener: () => void) { listeners.set(type, [...(listeners.get(type) ?? []), listener]); }, close() { node.open = false; for (const listener of listeners.get("close") ?? []) listener(); } }); }
    return node;
  };
  Object.defineProperty(globalThis, "document", { configurable: true, value: document }); Object.defineProperty(globalThis, "HTMLElement", { configurable: true, value: TestElement }); Object.defineProperty(globalThis, "localStorage", { configurable: true, value: retained });
  try {
    locale.set("zh-CN"); const opened = openNumberingPanel(host({ preview: async () => { throw new UiError(message("common.operation", { operationId: "author-operation😀" })); } }));
    const dialog = document.body.find("dialog"), heading = dialog.find("h2"), start = dialog.all().find(node => node.getAttribute("aria-label") === uiTextValue(message("numberingPanel.documentStart")))!, prepare = dialog.find("button", uiTextValue(message("numberingPanel.previewDocumentNumbering"))), close = dialog.find("button", uiTextValue(message("common.close")));
    start.value = "0003"; prepare.onclick!(); await new Promise(resolve => setImmediate(resolve)); const status = dialog.all().find(node => node.getAttribute("role") === "status")!, chinese = status.textContent, label = start.getAttribute("aria-label"); locale.set("en-US");
    assert.notEqual(status.textContent, chinese); assert.ok(status.textContent.includes("author-operation😀")); assert.notEqual(start.getAttribute("aria-label"), label); assert.equal(start.value, "0003"); assert.ok(dialog.all().includes(start));
    const retired = heading.textContent; close.onclick!(); await opened; locale.set("zh-CN"); assert.equal(heading.textContent, retired); assert.equal(document.body.children.length, 0);
  } finally {
    locale.set(previousLocale); names.forEach((name, index) => { if (saved[index]) Object.defineProperty(globalThis, name, saved[index]!); else Reflect.deleteProperty(globalThis, name); });
  }
});
