import { ApiError } from "./api-error";
import { locale, message,uiTextValue } from "./ui-locale";
import { TestDocument } from "./dom.test-support";
import assert from "node:assert/strict";
import test from "node:test";
import { NativeStyleController, nativeLanguageOptions, openLibraryStylePanel, type LibraryStyleHost, type LibraryStylePrepare, type LibraryStyleState } from "./style-panel";

const original = '#let library-style(body) = {\n  set text(size: 11pt)\n  // custom rule must stay exactly as authored\n  show heading: it => [【#it.body】]\n  body\n}\n';
function state(): LibraryStyleState {
  return { path:"Styles/library-style.typ",source:original,baseId:"v1",enabled:true,entryFunction:"library-style",resourceRef:null,installed:true,sourceHash:"a".repeat(64),syntaxValid:true,warnings:[],controls:[{key:"text.size",label:message("engine.style.label.bodySize"),kind:"text",value:"11pt",editable:true,reason:null},{key:"custom",label:message("common.unavailable"),kind:"text",value:"",editable:false,reason:{explanation:message("stylePanel.nativeParameterNotUnique"),diagnostics:[],causes:[]}}] };
}
const appliedState = () => ({ ...state(), source: original.replace("11pt", "12pt"), baseId: "v2", operationReceipt: { operationId: "operation-1", planId: "plan-1", status: "committed",path:"Styles/library-style.typ",nodeIds:[] } });
test("friendly locale choices submit original native expressions and keep unknown rules", () => {
  assert.ok(nativeLanguageOptions("text.lang", '"zh"').some(o => o.value === '"en"' && uiTextValue(o.label).includes("English")));
  assert.ok(nativeLanguageOptions("text.region", '"CN"').some(o => o.value === "none" && uiTextValue(o.label).includes("未指定")));
  assert.equal(nativeLanguageOptions("text.lang", "custom-language")[0].value, "custom-language");
  assert.deepEqual(nativeLanguageOptions("text.font", '"zh"'), []);
});
function host(overrides: Partial<LibraryStyleHost> = {}): LibraryStyleHost {
  return { read: async () => state(), operationId: () => "operation-1", prepare: async request => ({ operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: request.source ?? original.replace("11pt", "12pt"), summary: [message("engine.style.label.bodySize")],enabled:request.enabled??true,status:"prepared" }), apply: async () => appliedState(), ...overrides };
}

test("native controls propose only a scoped value; custom rules stay with the native owner", async () => {
  let request: LibraryStylePrepare | undefined;
  const controller = new NativeStyleController(host({ prepare: async input => { request = input; return { operationId: input.operationId, planId: "plan-1", beforeSource: original, afterSource: original.replace("11pt", "12pt"), summary: [message("engine.style.label.bodySize")],enabled:input.enabled??true,status:"prepared" }; } }));
  await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare();
  assert.deepEqual(request, { operationId: "operation-1", baseId: "v1", changes: { "text.size": "12pt" } });
  assert.ok(controller.plan!.afterSource.includes("show heading: it => [【#it.body】]"));
  assert.throws(() => controller.setControl("custom", "changed"), /没有唯一/);
});

test("advanced source is exact and excludes previous simplified-control proposals", async () => {
  let request: LibraryStylePrepare | undefined;
  const controller = new NativeStyleController(host({ prepare: async input => { request = input; return { operationId: input.operationId, planId: "plan-1", beforeSource: original, afterSource: input.source!, summary: [message("common.save")],enabled:input.enabled??true,status:"prepared" }; } }));
  await controller.load(); controller.setControl("text.size", "12pt");
  const source = original.replace("\n", "\r\n") + "// 完整自定义原文\r\n";
  controller.setSource(source); controller.setEnabled(false);
  assert.throws(() => controller.setControl("text.size", "13pt"), /完整原生源码/);
  await controller.prepare();
  assert.deepEqual(request, { operationId: "operation-1", baseId: "v1", source, enabled: false });
});

test("unknown preparation retries the immutable operation and preserves input", async () => {
  const requests: LibraryStylePrepare[] = []; let first = true;
  const controller = new NativeStyleController(host({ prepare: async request => { requests.push(request); if (first) { first = false; throw new Error("connection lost"); } return { operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: original.replace("11pt", "12pt"), summary: [],enabled:true,status:"prepared" }; } }));
  await controller.load(); controller.setControl("text.size", "12pt");
  await assert.rejects(controller.prepare(), /connection lost/);
  assert.equal(controller.uncertain, "prepare"); assert.throws(() => controller.setSource("new"), /结果待确认/);
  assert.throws(() => controller.resetDraft(), /结果待确认/);
  await controller.prepare(); assert.deepEqual(requests[0], requests[1]); assert.equal(controller.uncertain, null);
});

test("unknown apply confirms the same prepared plan before advancing the source basis", async () => {
  const calls: { operationId: string; planId: string }[] = []; let first = true;
  const controller = new NativeStyleController(host({ apply: async request => { calls.push(request); if (first) { first = false; throw Object.assign(new Error("unknown"), { code: "resultUnknown" }); } return appliedState(); } }));
  await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare();
  await assert.rejects(controller.apply()); assert.equal(controller.state!.baseId, "v1"); assert.equal(controller.dirty, true);
  await assert.rejects(controller.prepare(), /不能准备/); await controller.apply();
  assert.deepEqual(calls[0], calls[1]); assert.equal(controller.state!.baseId, "v2"); assert.equal(controller.dirty, false);
});

test("mismatched operation, before bytes or explicit source cannot be applied", async () => {
  for (const mismatch of [{ operationId: "other" }, { beforeSource: "different" }, { afterSource: "different" }]) {
    const controller = new NativeStyleController(host({ prepare: async request => ({ operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: request.source!, summary: [],enabled:true,status:"prepared", ...mismatch }) }));
    await controller.load(); controller.setSource(original + "// edit\n");
    await assert.rejects(controller.prepare(), /不一致/); await assert.rejects(controller.apply(), /先查看/); assert.equal(controller.dirty, true);
  }
});

test("known commit with unavailable fresh state keeps the receipt route distinct from unknown commit", async () => {
  const calls: { operationId: string; planId: string }[] = []; let first = true;
  let changed = 0;
  const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async request => { calls.push(request); if (first) { first = false; throw new ApiError("style_state_unavailable",message("common.unavailable"),[],{kind:"committedReceipt",receipt:appliedState().operationReceipt,cause:{code:"readUnavailable",explanation:message("common.unavailable"),diagnostics:[],causes:[]}}); } return appliedState(); } }));
  await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare(); await assert.rejects(controller.apply());
  assert.equal(controller.committedPendingRead, true); assert.equal(controller.uncertain, "apply"); assert.equal(controller.state!.baseId, "v1"); assert.equal(changed, 1);
  await controller.apply(); assert.deepEqual(calls[0], calls[1]); assert.equal(controller.committedPendingRead, false); assert.equal(controller.state!.baseId, "v2"); assert.equal(changed, 1);
});

test("style notifications require the original committed receipt and never treat unknown or unrelated errors as a write", async () => {
  for (const error of [Object.assign(new Error("network lost"), { code: "resultUnknown" }), Object.assign(new Error("wrong receipt"), { code: "style_state_unavailable", details: { receipt: { ...appliedState().operationReceipt, operationId: "another-operation" } } })]) {
    let changed = 0; const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async () => { throw error; } }));
    await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare(); assert.equal(changed, 0);
    await assert.rejects(controller.apply()); assert.equal(changed, 0); assert.equal(controller.committedPendingRead, false); assert.equal(controller.plan?.operationId, "operation-1");
  }
  let changed = 0; const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async () => ({ ...appliedState(), operationReceipt: { ...appliedState().operationReceipt, planId: "different-plan" } }) }));
  await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare(); await assert.rejects(controller.apply(), /回执与原计划不符/); assert.equal(changed, 0); assert.equal(controller.state?.baseId, "v1");
});

test("fresh reads and terminal conflicts preserve the current settings draft", async () => {
  const controller = new NativeStyleController(host({ apply: async () => { throw Object.assign(new Error("external style changed"), { code: "conflict" }); } }));
  await controller.load(); controller.setControl("text.size", "12pt");
  await assert.rejects(controller.load(), /不能用重新读取覆盖/); await controller.prepare(); await assert.rejects(controller.apply(), /external style changed/);
  assert.equal(controller.changes["text.size"], "12pt"); assert.equal(controller.uncertain, null); assert.equal(controller.plan, null);
  controller.resetDraft(); assert.equal(controller.dirty, false); assert.equal(controller.source, original);
});

test("a known committed style cannot lose its original plan after a later terminal-looking error", async () => {
  let calls = 0, changed = 0;
  const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async () => {
    if (++calls === 1) throw new ApiError("style_state_unavailable", message("common.unavailable"), [], { kind: "committedReceipt", receipt: appliedState().operationReceipt, cause: { code: "readUnavailable", explanation: message("common.unavailable"), diagnostics: [], causes: [] } });
    if (calls === 2) throw new ApiError("conflict", message("common.unavailable"));
    return appliedState();
  } }));
  await controller.load(); controller.setControl("text.size", "12pt"); await controller.prepare();
  await assert.rejects(controller.apply()); await assert.rejects(controller.apply());
  assert.equal(controller.committedPendingRead, true); assert.equal(controller.uncertain, "apply"); assert.equal(controller.plan!.operationId, "operation-1"); assert.equal(changed, 1);
  assert.throws(() => controller.resetDraft()); await controller.apply(); assert.equal(controller.state!.baseId, "v2"); assert.equal(changed, 1);
});

test("the actual style dialog updates labels and ARIA without replacing native inputs and retires on close", async () => {
  const previousDocument = Object.getOwnPropertyDescriptor(globalThis, "document"), previousLocale = locale.current, document = new TestDocument();
  Object.defineProperty(globalThis, "document", { configurable: true, value: document });
  try {
    locale.set("zh-CN"); const opened = openLibraryStylePanel(host()); await new Promise(resolve => setImmediate(resolve));
    const dialog = document.body.find("dialog"), heading = dialog.find("h2"), input = dialog.all().find(node => node.dataset.controlKey === JSON.stringify(["library-style", "text.size", "native-value"]))!, source = dialog.find("textarea"), close = dialog.find("button", uiTextValue(message("common.close")));
    input.value = "author-size(作者😀)"; const label = input.getAttribute("aria-label"); locale.set("en-US");
    assert.notEqual(input.getAttribute("aria-label"), label); assert.equal(input.value, "author-size(作者😀)"); assert.equal(source.value, original); assert.equal(dialog.all().find(node => node.dataset.controlKey === input.dataset.controlKey), input);
    const retiredHeading = heading.textContent; close.onclick!(); await opened; locale.set("zh-CN"); assert.equal(heading.textContent, retiredHeading); assert.equal(document.body.children.length, 0);
  } finally {
    locale.set(previousLocale); if (previousDocument) Object.defineProperty(globalThis, "document", previousDocument); else Reflect.deleteProperty(globalThis, "document");
  }
});
