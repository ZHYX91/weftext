import test from "node:test";
import assert from "node:assert/strict";
import { retainDefinitionReceipt, sameDefinitionBasis, validateDefinitionPatches, validatePreparedDefinition, type DefinitionAuthorContext, type DefinitionPrepareRequest, type DefinitionPrepared, type DefinitionProposal } from "./hosted-query-definition-state";

const source = "原稿😀\r\n保留";
const request: DefinitionPrepareRequest = { resource: "owner", sessionId: "session", baseId: "base", source, serial: 7, sourceHash: "exact-hash", operationId: "original-proposal", label: "native-query", plan: { scope: { descendants: true, lifecycle: "live" }, types: [], select: [], order: [] } };
const proposal: DefinitionProposal = { definition: { key: "server-key", label: request.label, binding: { revision: "server-schema" }, plan: request.plan }, source: source + "\n#metadata(definition)", sourceHash: request.sourceHash, serial: 7, patchUnit: "utf16", patches: [{ from: source.length, to: source.length, text: "\n#metadata(definition)", expected: "" }], draftOnly: true, canApply: true, recoveryOnly: false };
const prepared: DefinitionPrepared = { ...proposal, operationId: request.operationId, resource: request.resource, status: "reserved", key: proposal.definition.key };
const context: DefinitionAuthorContext = { inputId: "test-input", ...request, writable: true, pending: false, compare: false };

test("new definition validates original ID, server key, exact source serial/hash and UTF16 patch before joining Undo", () => {
  assert.equal(validatePreparedDefinition(request, prepared).basisSource, source);
  assert.deepEqual(validateDefinitionPatches(source, proposal), [{ from: source.length, to: source.length, insert: "\n#metadata(definition)", expected: "" }]);
  for (const changed of [{ operationId: "another" }, { resource: "other-owner" }, { key: "different-key" }, { sourceHash: "other-hash" }, { serial: 8 }, { patchUnit: "byte" }, { source: source + "unrelated rewrite" }]) assert.throws(() => validatePreparedDefinition(request, { ...prepared, ...changed } as DefinitionPrepared));
  assert.throws(() => validateDefinitionPatches(source, { ...proposal, patches: [{ from: 3, to: 3, text: "x" }] })); // Half a surrogate pair.
  assert.throws(() => validateDefinitionPatches(source, { ...proposal, patches: [{ from: 0, to: 1, text: "x", expected: "another" }] }));
});
test("equal bytes after source ABA, owner switch, new session/base or unknown save do not regain automatic proposal application", () => {
  assert.equal(sameDefinitionBasis(context, request, context.inputId), true);
  for (const changed of [{ serial: 9 }, { resource: "other" }, { sessionId: "reopened" }, { baseId: "new-base" }, { writable: false }, { pending: true }, { compare: true }]) assert.equal(sameDefinitionBasis({ ...context, ...changed }, request, context.inputId), false);
});
test("original operation terminal delivery drops earlier preview and never substitutes another owner's receipt", () => {
  const record = { operationId: request.operationId, resource: request.resource, request, receipt: { ...prepared, proposal } };
  const cancelled = retainDefinitionReceipt(record, { operationId: request.operationId, resource: request.resource, status: "cancelled" });
  assert.equal(cancelled.receipt?.proposal, undefined); assert.equal(cancelled.request, request);
  assert.throws(() => retainDefinitionReceipt(record, { operationId: request.operationId, resource: "other", status: "reserved" }));
});

test("a no-transaction ABA cannot auto-apply a frozen definition on equal source and serial",()=>{
 assert.equal(sameDefinitionBasis({...context,inputId:"new-input"},request,"frozen-input"),false);
 assert.equal(sameDefinitionBasis({...context,inputId:"frozen-input"},request,"frozen-input"),true);
});
