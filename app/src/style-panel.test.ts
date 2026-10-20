import assert from "node:assert/strict";
import test from "node:test";
import { NativeStyleController, nativeLanguageOptions, type LibraryStyleHost, type LibraryStylePrepare, type LibraryStyleState } from "./style-panel";

const original = '#let library-style(body) = {\n  set text(size: 11pt)\n  // custom rule must stay exactly as authored\n  show heading: it => [【#it.body】]\n  body\n}\n';
function state(): LibraryStyleState {
  return { path: "Styles/library-style.typ", source: original, baseId: "v1", enabled: true, controls: [{ key: "text.size", label: "正文大小", kind: "number", value: 11, unit: "pt", min: 1, max: 100, editable: true }, { key: "custom", label: "自定义规则", kind: "text", value: "", editable: false, reason: "没有唯一的原生参数" }] };
}
const appliedState = () => ({ ...state(), source: original.replace("11pt", "12pt"), baseId: "v2", operationReceipt: { operationId: "operation-1", planId: "plan-1", status: "committed" } });
test("friendly locale choices submit original native expressions and keep unknown rules", () => {
  assert.ok(nativeLanguageOptions("text.lang", '"zh"').some(o => o.value === '"en"' && o.label.includes("English")));
  assert.ok(nativeLanguageOptions("text.region", '"CN"').some(o => o.value === "none" && o.label.includes("未指定")));
  assert.equal(nativeLanguageOptions("text.lang", "custom-language")[0].value, "custom-language");
  assert.deepEqual(nativeLanguageOptions("text.font", '"zh"'), []);
});
function host(overrides: Partial<LibraryStyleHost> = {}): LibraryStyleHost {
  return { read: async () => state(), operationId: () => "operation-1", prepare: async request => ({ operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: request.source ?? original.replace("11pt", "12pt"), summary: ["正文大小"] }), apply: async () => appliedState(), ...overrides };
}

test("native controls propose only a scoped value; custom rules stay with the native owner", async () => {
  let request: LibraryStylePrepare | undefined;
  const controller = new NativeStyleController(host({ prepare: async input => { request = input; return { operationId: input.operationId, planId: "plan-1", beforeSource: original, afterSource: original.replace("11pt", "12pt"), summary: ["正文大小"] }; } }));
  await controller.load(); controller.setControl("text.size", 12); await controller.prepare();
  assert.deepEqual(request, { operationId: "operation-1", baseId: "v1", changes: { "text.size": 12 } });
  assert.ok(controller.plan!.afterSource.includes("show heading: it => [【#it.body】]"));
  assert.throws(() => controller.setControl("custom", "changed"), /没有唯一/);
});

test("advanced source is exact and excludes previous simplified-control proposals", async () => {
  let request: LibraryStylePrepare | undefined;
  const controller = new NativeStyleController(host({ prepare: async input => { request = input; return { operationId: input.operationId, planId: "plan-1", beforeSource: original, afterSource: input.source!, summary: ["原生源码"] }; } }));
  await controller.load(); controller.setControl("text.size", 12);
  const source = original.replace("\n", "\r\n") + "// 完整自定义原文\r\n";
  controller.setSource(source); controller.setEnabled(false);
  assert.throws(() => controller.setControl("text.size", 13), /完整原生源码/);
  await controller.prepare();
  assert.deepEqual(request, { operationId: "operation-1", baseId: "v1", source, enabled: false });
});

test("unknown preparation retries the immutable operation and preserves input", async () => {
  const requests: LibraryStylePrepare[] = []; let first = true;
  const controller = new NativeStyleController(host({ prepare: async request => { requests.push(request); if (first) { first = false; throw new Error("connection lost"); } return { operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: original.replace("11pt", "12pt"), summary: [] }; } }));
  await controller.load(); controller.setControl("text.size", 12);
  await assert.rejects(controller.prepare(), /connection lost/);
  assert.equal(controller.uncertain, "prepare"); assert.throws(() => controller.setSource("new"), /结果待确认/);
  assert.throws(() => controller.resetDraft(), /结果待确认/);
  await controller.prepare(); assert.deepEqual(requests[0], requests[1]); assert.equal(controller.uncertain, null);
});

test("unknown apply confirms the same prepared plan before advancing the source basis", async () => {
  const calls: { operationId: string; planId: string }[] = []; let first = true;
  const controller = new NativeStyleController(host({ apply: async request => { calls.push(request); if (first) { first = false; throw Object.assign(new Error("unknown"), { code: "resultUnknown" }); } return appliedState(); } }));
  await controller.load(); controller.setControl("text.size", 12); await controller.prepare();
  await assert.rejects(controller.apply()); assert.equal(controller.state!.baseId, "v1"); assert.equal(controller.dirty, true);
  await assert.rejects(controller.prepare(), /不能准备/); await controller.apply();
  assert.deepEqual(calls[0], calls[1]); assert.equal(controller.state!.baseId, "v2"); assert.equal(controller.dirty, false);
});

test("mismatched operation, before bytes or explicit source cannot be applied", async () => {
  for (const mismatch of [{ operationId: "other" }, { beforeSource: "different" }, { afterSource: "different" }]) {
    const controller = new NativeStyleController(host({ prepare: async request => ({ operationId: request.operationId, planId: "plan-1", beforeSource: original, afterSource: request.source!, summary: [], ...mismatch }) }));
    await controller.load(); controller.setSource(original + "// edit\n");
    await assert.rejects(controller.prepare(), /不一致/); await assert.rejects(controller.apply(), /先查看/); assert.equal(controller.dirty, true);
  }
});

test("known commit with unavailable fresh state keeps the receipt route distinct from unknown commit", async () => {
  const calls: { operationId: string; planId: string }[] = []; let first = true;
  let changed = 0;
  const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async request => { calls.push(request); if (first) { first = false; throw Object.assign(new Error("committed; current settings unavailable"), { code: "style_state_unavailable", details: { receipt: appliedState().operationReceipt } }); } return appliedState(); } }));
  await controller.load(); controller.setControl("text.size", 12); await controller.prepare(); await assert.rejects(controller.apply());
  assert.equal(controller.committedPendingRead, true); assert.equal(controller.uncertain, "apply"); assert.equal(controller.state!.baseId, "v1"); assert.equal(changed, 1);
  await controller.apply(); assert.deepEqual(calls[0], calls[1]); assert.equal(controller.committedPendingRead, false); assert.equal(controller.state!.baseId, "v2"); assert.equal(changed, 1);
});

test("style notifications require the original committed receipt and never treat unknown or unrelated errors as a write", async () => {
  for (const error of [Object.assign(new Error("network lost"), { code: "resultUnknown" }), Object.assign(new Error("wrong receipt"), { code: "style_state_unavailable", details: { receipt: { ...appliedState().operationReceipt, operationId: "another-operation" } } })]) {
    let changed = 0; const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async () => { throw error; } }));
    await controller.load(); controller.setControl("text.size", 12); await controller.prepare(); assert.equal(changed, 0);
    await assert.rejects(controller.apply()); assert.equal(changed, 0); assert.equal(controller.committedPendingRead, false); assert.equal(controller.plan?.operationId, "operation-1");
  }
  let changed = 0; const controller = new NativeStyleController(host({ committed: () => { changed++; }, apply: async () => ({ ...appliedState(), operationReceipt: { ...appliedState().operationReceipt, planId: "different-plan" } }) }));
  await controller.load(); controller.setControl("text.size", 12); await controller.prepare(); await assert.rejects(controller.apply(), /回执与原计划不符/); assert.equal(changed, 0); assert.equal(controller.state?.baseId, "v1");
});

test("fresh reads and terminal conflicts preserve the current settings draft", async () => {
  const controller = new NativeStyleController(host({ apply: async () => { throw Object.assign(new Error("external style changed"), { code: "conflict" }); } }));
  await controller.load(); controller.setControl("text.size", 12);
  await assert.rejects(controller.load(), /不能用重新读取覆盖/); await controller.prepare(); await assert.rejects(controller.apply(), /external style changed/);
  assert.equal(controller.changes["text.size"], 12); assert.equal(controller.uncertain, null); assert.equal(controller.plan, null);
  controller.resetDraft(); assert.equal(controller.dirty, false); assert.equal(controller.source, original);
});
