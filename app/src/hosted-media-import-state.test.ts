import assert from "node:assert/strict";
import test from "node:test";
import { canCommitResourceImport, decodeResourceImportDraft, freezeResourceImport, resourceCollection, resourceImportInput, resourceImportReceipt, type HostedResourceCollection, type ResourceImportDraft, type ResourceImportReceipt } from "./hosted-media-import-state";
import { message } from "./ui-locale";
const collection = (): HostedResourceCollection => ({ collection: "attachments", owner: "Shared", title: "授权资料", lifecycle: "live", read: false, download: false, import: true, status: "ready" });
const draft = (): ResourceImportDraft => ({ collection: "attachments", name: "原图😀.png", fileName: "原件.png", bytesBase64: "AQID", bytes: 3, revision: 2 });
const receipt = (): ResourceImportReceipt => ({ collection: "attachments", owner: "Shared", operationId: "original", status: "prepared", planId: "plan", preview: { name: "原图😀.png", bytes: 3, summary: message("error.api.hostedResourceCollections.importPlanSummary"), contentType: "image/png", metadata: { kind: "image", width: 12, height: 9 } }, resourceRefs: ["weftext-resource://workspace/core-assigned"], assets: ["attachments/core-assigned"], commitAllowed: true });
test("resource collection import is independent of read and download; Trash never gets an import writer", () => {
  assert.equal(resourceCollection(collection()).read, false); assert.equal(freezeResourceImport(draft(), collection(), "original").operationId, "original");
  assert.throws(() => resourceCollection({ ...collection(), download: true })); assert.throws(() => resourceCollection({ ...collection(), lifecycle: "trash" }));
  assert.throws(() => freezeResourceImport(draft(), { ...collection(), import: false }, "original")); assert.throws(() => freezeResourceImport(draft(), { ...collection(), status: "unavailable" }, "original"));
});
test("resource import freezes original upload name/bytes, including empty owned files, without a client resource key", () => {
  const input = draft(), request = freezeResourceImport(input, collection(), "original"); input.name = "后来名称"; input.bytesBase64 = "BAUG"; assert.equal(request.name, "原图😀.png"); assert.equal(request.bytesBase64, "AQID"); assert.deepEqual(Object.keys(request).sort(), ["bytesBase64", "collection", "name", "operationId"]);
  assert.equal(freezeResourceImport({ ...draft(), bytesBase64: "", bytes: 0 }, collection(), "zero").bytesBase64, "");
  assert.throws(() => freezeResourceImport({ ...draft(), name: "../private.png" }, collection(), "x")); assert.throws(() => freezeResourceImport({ ...draft(), bytes: 4 }, collection(), "x"));
  assert.throws(() => freezeResourceImport({ ...draft(), bytesBase64: "A".repeat(22369625) }, collection(), "x"));
});
test("later input and same-value ABA disable the original import commit without changing original request", () => {
  const input = draft(), request = freezeResourceImport(input, collection(), "original"); input.pending = { request, owner: "Shared", input: resourceImportInput(input), revision: input.revision, receipt: receipt() }; assert.equal(canCommitResourceImport(input, collection(), true), true);
  assert.equal(canCommitResourceImport(input, collection(), false), false, "a recovered preview is inert until the original operation is rechecked");
  input.name = "B"; input.revision++; assert.equal(canCommitResourceImport(input, collection(), true), false); input.name = request.name; input.revision++; assert.equal(canCommitResourceImport(input, collection(), true), false); assert.equal(input.pending.request.name, request.name);
  assert.equal(canCommitResourceImport({ ...input, revision: 2 }, { ...collection(), owner: "Remapped" }, true), false);
});
test("filtered original import facts replace old previews and locators; remapping does not invent cancellation", () => {
  const request = freezeResourceImport(draft(), collection(), "original"), old = receipt(), filtered: ResourceImportReceipt = { collection: "attachments", owner: "Remapped", operationId: "original", status: "committed", planId: null, preview: {}, resourceRefs: [], assets: [], commitAllowed: false };
  assert.deepEqual(resourceImportReceipt(request, filtered, old), filtered); assert.throws(() => resourceImportReceipt(request, { ...old, operationId: "another" })); assert.throws(() => resourceImportReceipt(request, { ...old, planId: "another" }, old)); assert.throws(() => resourceImportReceipt(request, { ...old, preview: { name: "replacement", bytes: 3 } }));
  assert.throws(() => resourceImportReceipt(request, old, { ...old, status: "committed", planId: null, commitAllowed: false }));
});
test("resource import recovery validates the complete original request and keeps unresolved records", () => {
  const input = draft(), request = freezeResourceImport(input, collection(), "original"); input.pending = { request, owner: "Shared", input: resourceImportInput(input), revision: input.revision }; assert.deepEqual(decodeResourceImportDraft(JSON.stringify(input)), input); assert.throws(() => decodeResourceImportDraft(JSON.stringify({ ...input, pending: { ...input.pending, request: { ...request, bytesBase64: "invalid!" } } })));
  assert.throws(() => freezeResourceImport(input, collection(), "new-id"), /核对或明确结束/);
  const stopped = resourceImportReceipt(request, { ...receipt(), status: "cancelled", planId: null, commitAllowed: false, preview: {}, resourceRefs: [], assets: [] }); assert.equal(stopped.status, "cancelled");
});
