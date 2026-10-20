import assert from "node:assert/strict";
import test from "node:test";
import { ProcessingSettingsController, defaultProcessingDocument, type ProcessingRead, type ProcessingRequest, type ProcessingSettingsHost } from "./processing-settings-panel";
function storage() { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; }
function state(): ProcessingRead { return { baseId: "base", beforeSource: '{"version":1,"custom":true}', processing: null, source: "shared-library-suggestion" }; }
function host(overrides: Partial<ProcessingSettingsHost> = {}): ProcessingSettingsHost { return { persistenceKey: "library:user:processing", read: async () => state(), prepare: async request => ({ operationId: request.operationId, planId: "plan", status: "prepared" }), status: async request => ({ operationId: request.operationId, planId: "plan", status: "prepared" }), settle: async request => ({ operationId: request.operationId, planId: "plan", status: "cancelled" }), apply: async ({ request }) => ({ operationId: request.operationId, planId: "plan", status: "committed" }), ...overrides }; }
test("uncertain requests persist before IO and reopening confirms the immutable original", async () => {
  const retained = storage(); let original: ProcessingRequest | undefined;
  const first = new ProcessingSettingsController(host({ prepare: async request => { original = request; throw new Error("network"); } }), retained);
  await first.load(); const document = defaultProcessingDocument(); document.defaults.languages = [{ language: "en", script: null, region: "GB" }]; first.edit(JSON.stringify(document));
  await assert.rejects(first.prepare(), /network/); assert.ok(first.pending); assert.throws(() => first.edit("{}"));
  let queried: ProcessingRequest | undefined;
  const reopened = new ProcessingSettingsController(host({ status: async request => { queried = request; return { operationId: request.operationId, planId: "plan", status: "prepared" }; } }), retained);
  await reopened.status(); assert.deepEqual(queried, original); assert.equal(reopened.pending?.operation?.planId, "plan");
});
test("prepared cancellation keeps settings input and only a durable cancelled receipt unlocks it", async () => {
  const retained = storage(); const controller = new ProcessingSettingsController(host(), retained); await controller.load();
  const input = JSON.stringify({ ...defaultProcessingDocument(), defaults: { ...defaultProcessingDocument().defaults, version: "2" } }); controller.edit(input); await controller.prepare();
  await controller.settle(); assert.equal(controller.pending, undefined); assert.equal(controller.source, input); assert.equal(controller.locked, false);
});
test("known stale preparation stays recoverable until its unknown original is durably ended", async () => {
  const retained = storage(); const requests: ProcessingRequest[] = [];
  const controller = new ProcessingSettingsController(host({ prepare: async request => { requests.push(request); throw Object.assign(new Error("stale"), { code: "staleSnapshot" }); }, status: async request => ({ operationId: request.operationId, planId: null, status: "notRegistered" }), settle: async request => { requests.push(request); return { operationId: request.operationId, planId: null, status: "cancelled" }; } }), retained);
  await controller.load(); await assert.rejects(controller.prepare(), /stale/); await controller.status(); assert.equal(controller.locked, true); await controller.settle(); assert.deepEqual(requests[0], requests[1]); assert.equal(controller.locked, false);
});
test("mismatched receipt keeps original request; a committed query refreshes settings", async () => {
  const retained = storage();
  const controller = new ProcessingSettingsController(host({ prepare: async () => ({ operationId: "other", planId: "wrong", status: "prepared" }) }), retained);
  await controller.load(); await assert.rejects(controller.prepare(), /不符/); assert.equal(controller.pending?.operation, undefined);
  const reopened = new ProcessingSettingsController(host({ status: async request => ({ operationId: request.operationId, planId: "plan", status: "committed" }), read: async () => ({ ...state(), baseId: "new", processing: defaultProcessingDocument() }) }), retained);
  await reopened.status(); assert.equal(reopened.state?.baseId, "new"); assert.equal(reopened.pending, undefined);
});
