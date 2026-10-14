import assert from "node:assert/strict";
import test from "node:test";
import { registryReceipt, type RegistryRequest, type RegistryReceipt } from "./hosted-navigation-registry-state";
const request: RegistryRequest = { operationId: "original", intent: { kind: "createTag", basis: "opaque-original", displayName: "原名" } };
const prepared: RegistryReceipt = { operationId: "original", status: "prepared", planId: "original-plan", preview: { kind: "createTag", displayName: "原名" }, commitAllowed: true, navigationRef: "weftext-tag://original", result: null };
test("navigation original plan and operation cannot be replaced by a late response", () => {
  assert.throws(() => registryReceipt(request, { ...prepared, operationId: "replacement" }));
  assert.throws(() => registryReceipt(request, { ...prepared, planId: "replacement-plan" }, prepared));
  assert.throws(() => registryReceipt(request, { ...prepared, status: "cancelled", commitAllowed: true }));
  assert.equal(registryReceipt(request, prepared).planId, "original-plan");
});
test("filtered prepared and committed receipts never inherit old private navigation preview", () => {
  const filtered: RegistryReceipt = { operationId: "original", status: "prepared", planId: null, preview: {}, commitAllowed: false, navigationRef: null, result: null };
  assert.deepEqual(registryReceipt(request, filtered, prepared), filtered);
  const committed = { ...filtered, status: "committed" }; assert.deepEqual(registryReceipt(request, committed, prepared), committed);
  assert.throws(() => registryReceipt(request, prepared, committed));
});
test("resolved and cancelled are terminal facts and cannot offer an invented installed target", () => {
  for (const status of ["resolved", "cancelled", "recoveryPending", "notRegistered"]) {
    const receipt = { ...prepared, status, planId: null, preview: {}, commitAllowed: false, navigationRef: null };
    assert.equal(registryReceipt(request, receipt).status, status);
    assert.throws(() => registryReceipt(request, { ...receipt, result: { kind: "tag", reference: "weftext-tag://made-up", status: "available" } }));
  }
});
