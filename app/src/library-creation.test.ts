import { message, uiTextValue } from "./ui-locale";
import { apiErrorUiText } from "./api-error";
import assert from "node:assert/strict";
import test from "node:test";
import { LibraryCreationController, initialLibraryStyle, type LibraryCreationDefaults, type LibraryCreationHost, type LibraryCreationRequest } from "./library-creation";
const native = '// 😀 preserve template bytes\n#let library-style(body) = { set text(font: "Noto Serif CJK SC", lang: "zh", region: "CN"); body }\n';
function defaults(): LibraryCreationDefaults { return { source: native, sourceHash: "frozen", controls: [["text.lang", '"zh"'], ["text.region", '"CN"']].map(([key, value]) => { const from = native.indexOf(value); return { key, value, editable: true, range: { utf16From: from, utf16To: from + value.length, raw: value } }; }) }; }
function storage() { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; }
function host(overrides: Partial<LibraryCreationHost> = {}): LibraryCreationHost { return { defaults: async () => defaults(), create: async request => ({ operationId: request.operationId, root: request.path, workspaceId: "workspace", status: "committed" }), delivery: async operationId => ({ operationId, root: "D:\\Owned\\Library", workspaceId: "workspace", status: "committed" }), deliver: async () => {}, inspect: async operationId => ({ operationId, status: "pending", description: {explanation:message("common.cancel"),diagnostics:[],causes:[]}, allowedActions: [] }), resolve: async operationId => ({ operationId, status: "resolved", description: {explanation:message("common.cancel"),diagnostics:[],causes:[]} }), cancel: async request => ({ operationId: request.operationId, status: "resolved", description: {explanation:message("common.cancel"),diagnostics:[],causes:[]} }), ...overrides }; }
test("initial locale changes only certified native literal slots and preserves unknown region", () => {
  const styled = initialLibraryStyle(defaults(), '"en"', "none"); assert.equal(styled.enabled, true); assert.ok(styled.source.includes('lang: "en", region: none')); assert.ok(styled.source.includes('font: "Noto Serif CJK SC"')); assert.ok(styled.source.startsWith('// 😀 preserve template bytes\n'));
  assert.throws(() => initialLibraryStyle(defaults(), '"en"); panic("x")', "none"));
  const ambiguous = defaults(); ambiguous.controls.push(ambiguous.controls[0]); assert.throws(() => initialLibraryStyle(ambiguous, '"en"', "none"));
});
test("known committed creation survives failed delivery and reopening; retry reads the original fact without another author request", async () => {
  const retained = storage(); let authors = 0, reads = 0, callbacks = 0, fail = true;
  let original!: { operationId: string; root: string; workspaceId: string; status: string };
  const transport = host({ create: async request => { authors++; return original = { operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }; },
    delivery: async id => { reads++; assert.equal(id, original.operationId); return structuredClone(original); },
    deliver: async receipt => { callbacks++; assert.equal(receipt.operationId, original.operationId); if (fail) throw new Error("root note refresh failed"); } });
  const controller = new LibraryCreationController(transport, retained);
  await controller.load(); controller.change({ path: "D:\\Owned\\Library" }); controller.preview();
  await assert.rejects(controller.deliver(), /root note refresh failed/);
  assert.equal(controller.receipt?.operationId, original.operationId); assert.equal(controller.locked, true);
  const reopened = new LibraryCreationController(transport, retained);
  assert.equal(reopened.receipt?.operationId, original.operationId);
  await assert.rejects(reopened.deliver(), /root note refresh failed/);
  fail = false; await reopened.deliver();
  assert.equal(authors, 1); assert.equal(reads, 2); assert.equal(callbacks, 3);
  assert.equal(new LibraryCreationController(transport, retained).request, null);
});
test("a later terminal packet cannot erase a known committed library receipt", async () => {
  const controller = new LibraryCreationController(host({ delivery: async operationId => ({ operationId, root: "D:\\Owned\\Library", workspaceId: "workspace", status: "resolved" }) }), storage());
  await controller.load(); controller.change({ path: "D:\\Owned\\Library" }); controller.preview(); await controller.create();
  const original = structuredClone(controller.receipt);
  await assert.rejects(controller.deliver(), /原操作不一致/);
  assert.deepEqual(controller.receipt, original); assert.equal(controller.locked, true);
  await assert.rejects(controller.cancel(), /已确认提交/); assert.deepEqual(controller.receipt, original);
});
test("unknown creation preserves exact native bytes and operation across reopening without new defaults", async () => {
  const retained = storage(), requests: LibraryCreationRequest[] = [];
  const controller = new LibraryCreationController(host({ create: async request => { requests.push(request); throw new Error("network lost"); } }), retained);
  await controller.load(); controller.change({ path: "D:\\Owned\\Library", language: '"en"', region: "none" }); controller.preview(); await assert.rejects(controller.create()); assert.throws(() => controller.change({ region: '"GB"' }));
  const reopened = new LibraryCreationController(host({ defaults: async () => { throw new Error("must not replace template"); }, create: async request => { requests.push(request); return { operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }; } }), retained);
  await reopened.load(); await reopened.create(); assert.deepEqual(requests[0], requests[1]); assert.ok(requests[1].options.initialStyle!.source.includes("region: none"));
});
test("only explicit durable termination releases a rejected creation for new inputs", async () => {
  const controller = new LibraryCreationController(host({ create: async () => { throw Object.assign(new Error("path exists"), { code: "exists" }); } }), storage());
  await controller.load(); controller.change({ path: "D:\\Owned\\Existing" }); controller.preview(); await assert.rejects(controller.create()); assert.equal(controller.locked, true); await controller.inspect(); assert.equal(controller.locked, true); await controller.cancel(); assert.equal(controller.locked, false); controller.change({ path: "D:\\Owned\\New" }); controller.preview(); assert.equal(controller.proposal!.path, "D:\\Owned\\New");
});
test("a committed cancel result keeps the original request and storage failure prevents author IO", async () => {
  const retained = storage(); let writable = true, calls = 0;
  const controller = new LibraryCreationController(host({ create: async request => { calls++; return { operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }; }, cancel: async request => ({ operationId: request.operationId, status: "committed", description: {explanation:message("common.cancel"),diagnostics:[],causes:[]} }) }), { ...retained, setItem: (k, v) => { if (!writable) throw new Error("storage unavailable"); retained.setItem(k, v); } });
  await controller.load(); controller.change({ path: "D:\\Owned\\Library" }); controller.preview(); writable = false; await assert.rejects(controller.create(), /storage/); assert.equal(calls, 0); assert.equal(controller.request, null); writable = true; await controller.create(); await controller.cancel(); assert.ok(controller.request); assert.equal(controller.recovery?.status, "committed");
});

test("root preset choices survive an unknown creation unchanged and never become a layout authority", async () => {
  const retained = storage(), requests: LibraryCreationRequest[] = [], presetDefaults = { ...defaults(), rootPresets: [{ id: "tasks", title: "任务", plan: {} }, { id: "events", title: "事件", plan: {} }] };
  const controller = new LibraryCreationController(host({ defaults: async () => presetDefaults, create: async request => { requests.push(request); throw new Error("lost reply"); } }), retained);
  await controller.load(); controller.change({ path: "D:\\Owned\\Library", rootMode: "queries", rootTitle: "我的首页😀", queries: ["tasks", "events"] }); controller.preview(); await assert.rejects(controller.create());
  const reopened = new LibraryCreationController(host({ defaults: async () => { throw new Error("do not replace frozen input"); }, create: async request => { requests.push(request); return { operationId: request.operationId, root: request.path, workspaceId: "same", status: "committed", rootNotePath: request.path + "/Library.typ" }; } }), retained);
  await reopened.load(); const receipt = await reopened.create(); assert.deepEqual(requests[0], requests[1]); assert.deepEqual(requests[0].options.initialRoot, { title: "我的首页😀", queries: ["tasks", "events"] }); assert.ok(receipt.rootNotePath); assert.ok(new LibraryCreationController(host(), retained).request);
});

test("typed library delivery errors survive restart and retain all causes until the original fresh delivery succeeds", async () => {
  const retained=storage();let id="",bad=true,authors=0,deliveries=0;
  const error={code:"originalUnavailable",explanation:message("shell.savedBasisChangedCheckDisk"),diagnostics:[],causes:[{code:"originalCause",explanation:message("common.operation",{operationId:"exact-original"}),diagnostics:[]}],details:{kind:"currentSource" as const,currentSource:"raw exact source\r\n😀"}};
  const response=()=>({operationId:id,root:"D:\\Owned\\Library",workspaceId:"workspace",status:"committed",...(bad?{deliveryError:structuredClone(error)}:{})});
  const transport=host({create:async request=>{authors++;id=request.operationId;return response();},delivery:async operationId=>{assert.equal(operationId,id);return response();},deliver:async()=>{deliveries++;}});
  const first=new LibraryCreationController(transport,retained);await first.load();first.change({path:"D:\\Owned\\Library"});first.preview();await assert.rejects(first.deliver(),/交付尚未完成/);assert.equal(deliveries,0);assert.equal(authors,1);
  const reopened=new LibraryCreationController(transport,retained);assert.deepEqual(reopened.receipt?.deliveryError,error);assert.ok(uiTextValue(apiErrorUiText(reopened.receipt!.deliveryError!)).includes("exact-original"));
  await assert.rejects(reopened.deliver(),/交付尚未完成/);bad=false;await reopened.deliver();assert.equal(authors,1);assert.equal(deliveries,1);
});
