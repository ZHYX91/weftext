import test from "node:test";
import assert from "node:assert/strict";
import { canCommitStructure, clearStructureCapture, decodeStructureCatalog, decodeStructureDraft, freezeStructureRequest, initialStructureDraft, structureEnded, structureInput, structureReceipt, structureSelectors, type StructureCatalog, type StructureDraft, type StructureReceipt } from "./hosted-structure-state";

const catalog: StructureCatalog = { scope: "configuredAuthorizedStructure", coverage: "complete", sources: [{ resource: "source", title: "Original", actions: ["rename", "move", "trash", "restore", "copy", "template"], status: "ready" }], slots: [{ resource: "slot", title: "Destination", name: "Fixed", parent: "approved-parent", create: true, receive: true, status: "ready", types: ["tasks/task"], capture: { types: [{ id: "tasks/task", fields: [{ id: "tasks/status", required: true }], requires: [], conflicts: [] }], fields: [{ id: "tasks/status", key: "status", shape: { kind: "enum", values: ["open", "done"], open: false }, nullable: false }], definitions: {}, aliases: {} } }], templates: [{ id: "native-note", title: "Native", builtin: true, status: "ready" }] };
function prepared(): StructureDraft {
  const draft = { ...initialStructureDraft(), kind: "move" as const, source: "source", destination: "slot" };
  const request = freezeStructureRequest(draft, catalog, "original");
  const receipt: StructureReceipt = { resource: "source", operationId: "original", status: "prepared", planId: "original-plan", preview: { summary: "Approved move" }, commitAllowed: true };
  return { ...draft, pending: { request, input: structureInput(draft), revision: draft.revision, selector: structureSelectors(draft, catalog), acceptLosses: false, receipt } };
}

test("structure capture freezes native trees and explicit inputs, leaving source selection and later form edits independent", () => {
  const draft = { ...initialStructureDraft(), destination: "slot", title: "中文😀", body: "#literal text", types: ["tasks/task"], fields: { "tasks/status": { kind: "text" as const, value: "done" } }, inputs: ["explicit-read"] };
  const original = freezeStructureRequest(draft, catalog, "original");
  draft.fields["tasks/status"].value = "open"; draft.inputs.push("later"); draft.body = "later source";
  assert.deepEqual(original, { resource: "slot", operationId: "original", kind: "create", destination: "slot", inputs: ["explicit-read"], capture: { title: "中文😀", body: "#literal text", bodyMode: "text", types: ["tasks/task"], fields: { "tasks/status": { kind: "text", value: "done" } } } });
  assert.equal("name" in original, false); assert.equal("parent" in original, false);
  assert.throws(() => freezeStructureRequest({ ...draft, rawInputs: { "unmapped-shape": "invalid number" } }, catalog, "never-sent"), /原文/);
  assert.throws(() => freezeStructureRequest(draft, { ...catalog, slots: [{ ...catalog.slots[0], capture: undefined }] }, "no-schema"), /表单权限/);
});

test("independent structure rights cannot be borrowed from a slot reader; restore alone accepts unavailable live source with the original trash ID", () => {
  const moved = prepared(); assert.equal(canCommitStructure(moved, catalog), true);
  assert.throws(() => freezeStructureRequest({ ...moved, pending: undefined }, { ...catalog, sources: [{ ...catalog.sources[0], actions: [] }] }, "denied"), /独立资格/);
  const draft = { ...initialStructureDraft(), kind: "restore" as const, source: "source", destination: "slot", trashOperationId: "original-trash" }, trashCatalog = { ...catalog, sources: [{ ...catalog.sources[0], status: "unavailable" as const }] };
  assert.equal(freezeStructureRequest(draft, trashCatalog, "restore-one").trashOperationId, "original-trash");
  assert.throws(() => freezeStructureRequest({ ...draft, trashOperationId: "" }, trashCatalog, "missing"), /原回收操作编号/);
  assert.throws(() => freezeStructureRequest({ ...draft, kind: "move" }, trashCatalog, "move-trash"), /独立资格/);
  assert.throws(() => freezeStructureRequest({ ...draft, kind: "create", source: "", destination: "slot" }, { ...catalog, slots: [{ ...catalog.slots[0], create: false }] }, "no-create"), /固定目标/);
});

test("a prepared operation cannot commit later input, same-text ABA, a refreshed selector, or an incomplete field input", () => {
  const draft = prepared(), original = structuredClone(draft.pending!.request);
  draft.body = "typed later"; assert.equal(canCommitStructure(draft, catalog), false);
  draft.body = ""; draft.revision++; assert.equal(canCommitStructure(draft, catalog), false);
  assert.deepEqual(draft.pending!.request, original);
  assert.equal(canCommitStructure(prepared(), { ...catalog, slots: [{ ...catalog.slots[0], parent: "different-approved-parent" }] }), false);
  const raw = prepared(); raw.rawInputs = { key: "incomplete" }; assert.equal(canCommitStructure(raw, catalog), false);
});

test("unknown restore retains the complete original prepare and commit; a corrupted commit locator cannot be recovered as another operation", () => {
  const draft = prepared(); draft.pending!.commit = { resource: "source", operationId: "original", planId: "original-plan", acceptLosses: false };
  const loaded = decodeStructureDraft(JSON.stringify(draft)); assert.deepEqual(loaded, draft);
  loaded.body = "newer form"; assert.deepEqual(loaded.pending!.request, draft.pending!.request); assert.deepEqual(loaded.pending!.commit, draft.pending!.commit);
  for (const changes of [{ resource: "other" }, { operationId: "replacement" }, { planId: "different" }]) assert.throws(() => decodeStructureDraft(JSON.stringify({ ...draft, pending: { ...draft.pending, commit: { ...draft.pending!.commit, ...changes } } })), /原结构提交请求/);
  assert.equal(structureEnded("recoveryPending"), false); assert.equal(structureEnded("notRegistered"), false); assert.equal(structureEnded("resolved"), true);
});

test("current status filtering replaces stale preview and result without erasing the committed original fact", () => {
  const request = prepared().pending!.request;
  const committed: StructureReceipt = { resource: request.resource, operationId: request.operationId, status: "committed", planId: "original-plan", preview: { summary: "private old preview" }, nodeRefs: [{ workspaceId: "w", nodeId: "n" }], result: { resource: "slot", nodeRef: { workspaceId: "w", nodeId: "n" }, canOpen: true, status: "ready" }, commitAllowed: false };
  assert.deepEqual(structureReceipt(request, { resource: request.resource, operationId: request.operationId, status: "committed", planId: null, commitAllowed: false }, committed), { resource: request.resource, operationId: request.operationId, status: "committed", planId: null, commitAllowed: false });
  assert.throws(() => structureReceipt(request, { ...committed, status: "cancelled" }, committed), /原结构事实/);
  assert.throws(() => structureReceipt(request, { ...committed, operationId: "other" }), /原对象和编号/);
  assert.throws(() => structureReceipt(request, { ...committed, status: "prepared", commitAllowed: true, preview: undefined }), /准确预览/);
  assert.throws(() => decodeStructureCatalog({ ...catalog, coverage: "unknown" } as unknown as StructureCatalog), /完整性/);
  assert.equal(decodeStructureCatalog({ ...catalog, coverage: "partial" }).coverage, "partial");
});

test("explicit capture disposal unlocks a plain destination without discarding other form input or rewriting a pending original operation", () => {
  const typed = { ...prepared(), title: "keep title", body: "keep body", parametersText: "{unfinished", types: ["tasks/task"], fields: { "tasks/status": { kind: "text" as const, value: "done" } }, rawInputs: { old: "invalid" } }, original = structuredClone(typed);
  const cleared = clearStructureCapture(typed);
  assert.deepEqual(typed, original); assert.equal(cleared.title, typed.title); assert.equal(cleared.body, typed.body); assert.equal(cleared.parametersText, typed.parametersText); assert.deepEqual(cleared.pending, typed.pending); assert.equal(cleared.revision, typed.revision + 1); assert.equal(canCommitStructure(cleared, catalog), false);
  assert.deepEqual(freezeStructureRequest({ ...cleared, kind: "create", pending: undefined, destination: "slot" }, { ...catalog, slots: [{ ...catalog.slots[0], capture: undefined }] }, "plain-create").capture, { title: "keep title", body: "keep body", bodyMode: "text" });
});
