import {message} from "./ui-locale";
import assert from "node:assert/strict";
import test from "node:test";
import { ProcessingSettingsController, defaultProcessingDocument, type ProcessingRead, type ProcessingRequest, type ProcessingSettingsHost } from "./processing-settings-panel";
function storage() { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; }
function state(): ProcessingRead { return { baseId: "base", beforeSource: '{"version":1,"custom":true}', processing: null,roleRef:"settings-ref",revision:null,warning:null,source: "shared-library-suggestion" }; }
function operation(operationId:string,status="prepared",planId:string|null="plan") {return {operationId,planId,status,resource:"library.processing",preview:{beforeSource:state().beforeSource,afterSource:"updated original settings",summary:message("common.save")},intentKind:"updateFiles",resourceRefs:[],nodeRefs:[]};}
function host(overrides: Partial<ProcessingSettingsHost> = {}): ProcessingSettingsHost { return { persistenceKey: "library:user:processing", read: async () => state(), prepare: async request => operation(request.operationId), status: async request => operation(request.operationId), settle: async request => operation(request.operationId,"cancelled"), apply: async ({ request }) => operation(request.operationId,"committed"), ...overrides }; }
test("uncertain requests persist before IO and reopening confirms the immutable original", async () => {
  const retained = storage(); let original: ProcessingRequest | undefined;
  const first = new ProcessingSettingsController(host({ prepare: async request => { original = request; throw new Error("network"); } }), retained);
  await first.load(); const document = defaultProcessingDocument(); document.defaults.languages = [{ language: "en", script: null, region: "GB" }]; first.edit(JSON.stringify(document));
  await assert.rejects(first.prepare(), /network/); assert.ok(first.pending); assert.throws(() => first.edit("{}"));
  let queried: ProcessingRequest | undefined;
  const reopened = new ProcessingSettingsController(host({ status: async request => { queried = request; return operation(request.operationId); } }), retained);
  await reopened.status(); assert.deepEqual(queried, original); assert.equal(reopened.pending?.operation?.planId, "plan");
});
test("prepared cancellation keeps settings input and only a durable cancelled receipt unlocks it", async () => {
  const retained = storage(); const controller = new ProcessingSettingsController(host(), retained); await controller.load();
  const input = JSON.stringify({ ...defaultProcessingDocument(), defaults: { ...defaultProcessingDocument().defaults, version: "2" } }); controller.edit(input); await controller.prepare();
  await controller.settle(); assert.equal(controller.pending, undefined); assert.equal(controller.source, input); assert.equal(controller.locked, false);
});
test("known stale preparation stays recoverable until its unknown original is durably ended", async () => {
  const retained = storage(); const requests: ProcessingRequest[] = [];
  const controller = new ProcessingSettingsController(host({ prepare: async request => { requests.push(request); throw Object.assign(new Error("stale"), { code: "staleSnapshot" }); }, status: async request => ({ operationId: request.operationId, planId: null, status: "notRegistered" }), settle: async request => { requests.push(request); return operation(request.operationId,"cancelled",null); } }), retained);
  await controller.load(); await assert.rejects(controller.prepare(), /stale/); await controller.status(); assert.equal(controller.locked, true); await controller.settle(); assert.deepEqual(requests[0], requests[1]); assert.equal(controller.locked, false);
});
test("mismatched receipt keeps original request; a committed query refreshes settings", async () => {
  const retained = storage();
  const controller = new ProcessingSettingsController(host({ prepare: async () => operation("other") }), retained);
  await controller.load(); await assert.rejects(controller.prepare(), /不符/); assert.equal(controller.pending?.operation, undefined);
  const reopened = new ProcessingSettingsController(host({ status: async request => operation(request.operationId,"committed"), read: async () => ({ ...state(), baseId: "new", processing: defaultProcessingDocument() }) }), retained);
  await reopened.status(); assert.equal(reopened.state?.baseId, "new"); assert.equal(reopened.pending, undefined);
});

test("confirmed settings mutations invalidate before a failing fresh read across commit and original-operation recovery", async () => {
  for (const method of ["apply", "status", "retryPrepare", "settle"] as const) {
    const retained = storage(); let changed = 0, applied = 0;
    const target = host({ committed: () => { changed++; assert.equal(controller.pending?.operation?.status, "committed"); }, onApplied: () => { applied++; } });
    const controller = new ProcessingSettingsController(target, retained); await controller.load();
    controller.edit(JSON.stringify(defaultProcessingDocument())); await controller.prepare();
    assert.equal(changed, 0, "Draft and preview do not change the shared author environment");
    const committed = async (request: ProcessingRequest) => operation(request.operationId,"committed");
    target.read = async () => { throw new Error("fresh read unavailable"); };
    target.status = committed; target.prepare = committed; target.settle = committed;
    await assert.rejects(controller[method](), /fresh read unavailable/);
    assert.equal(changed, 1); assert.equal(applied, 0); assert.equal(controller.pending?.operation?.status, "committed");
    assert.match(retained.getItem(target.persistenceKey)!, /"status":"committed"/);
    target.read = async () => ({ ...state(), baseId: "fresh" }); await controller.status();
    assert.equal(changed, 1); assert.equal(applied, 1); assert.equal(controller.pending, undefined); assert.equal(controller.state?.baseId, "fresh");
  }
});

test("an unrelated settings receipt cannot notify a successful mutation", async () => {
  let changed = 0; const target = host({ committed: () => { changed++; }, status: async () => operation("unrelated","committed") });
  const controller = new ProcessingSettingsController(target, storage()); await controller.load(); await controller.prepare();
  await assert.rejects(controller.status(), /不符/); assert.equal(changed, 0); assert.equal(controller.pending?.operation?.status, "prepared");
});
