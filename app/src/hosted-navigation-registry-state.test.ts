import assert from "node:assert/strict";
import test from "node:test";
import { registryIntent, registryReceipt, type RegistryRequest, type RegistryReceipt, type RegistryCatalog, type RegistryDraft } from "./hosted-navigation-registry-state";
const request: RegistryRequest = { operationId: "original", intent: { kind: "createTag", basis: "opaque-original", displayName: "原名", parentRef: null } };
const prepared: RegistryReceipt = { operationId: "original", status: "prepared", planId: "original-plan", preview: { kind: "createTag", displayName: "原名" }, commitAllowed: true, navigationRef: "weftext-tag://original", result: null };
test("navigation original plan and operation cannot be replaced by a late response", () => {
  assert.throws(() => registryReceipt(request, { ...prepared, operationId: "replacement" }));
  assert.throws(() => registryReceipt(request, { ...prepared, planId: "replacement-plan" }, prepared));
  assert.throws(() => registryReceipt(request, { ...prepared, status: "cancelled", commitAllowed: true }));
  assert.equal(registryReceipt(request, prepared).planId, "original-plan");
});
test("rename does not authorize move; parents exclude self and descendants while create sets only initial parent", () => {
  const root = { reference: "weftext-tag://workspace/root", tagId: "root", displayName: "root", parentTagId: null }, child = { reference: "weftext-tag://workspace/child", tagId: "child", displayName: "child", parentTagId: "root" };
  const catalog: RegistryCatalog = { directoryScope: "configuredAuthorizedDirectories", directoryCoverage: "complete", directories: [], admissionSlots: [], tags: { read: true, create: true, rename: true, move: false, basis: "original", coverage: "complete", items: [root, child] } };
  const draft: RegistryDraft = { kind: "moveTag", slot: "", reference: child.reference, displayName: "renamed", parentRef: null };
  assert.throws(() => registryIntent(draft, catalog), /move/); assert.deepEqual(registryIntent({ ...draft, kind: "renameTag" }, catalog), { kind: "renameTag", basis: "original", reference: child.reference, displayName: "renamed" });
  catalog.tags.move = true; assert.deepEqual(registryIntent(draft, catalog), { kind: "moveTag", basis: "original", reference: child.reference, parentRef: null });
  assert.throws(() => registryIntent({ ...draft, reference: root.reference, parentRef: child.reference }, catalog), /后代/); assert.throws(() => registryIntent({ ...draft, parentRef: child.reference }, catalog), /自身/);
  assert.deepEqual(registryIntent({ ...draft, kind: "createTag", parentRef: child.reference }, catalog), { kind: "createTag", basis: "original", displayName: "renamed", parentRef: child.reference });
  catalog.tags.coverage = "partial"; assert.throws(() => registryIntent(draft, catalog), /完整/); catalog.tags.coverage = "complete"; catalog.tags.read = false; assert.throws(() => registryIntent(draft, catalog), /完整/);
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
