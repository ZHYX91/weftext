import assert from "node:assert/strict";
import { test } from "node:test";
import { canCommitContent, contentBytes, contentEnded, contentInspection, contentReceipt, decodeContentDraft, freezeContentImport, type ContentDraft, type ContentReceipt } from "./hosted-content-import-state";
import { hostedOpenedDraft } from "./hosted-state";
import { message } from "./ui-locale";
const digest = "a".repeat(64), target = { id: "slot", title: "固定槽", formats: ["bibtex" as const], available: true };
const base: ContentDraft = { slot: "slot", format: "bibtex", bytesBase64: btoa("@book{a,title={A}}"), bytes: 18, fileName: "work.bib", revision: 2 };
base.bytes = contentBytes(base.bytesBase64!).length;
const publication = { kind: "record" as const, value: [] };
const inspected = { revision: 2, value: { uploadDigest: digest, records: [{ suggestedKey: null, publication, originalRecord: { title: "A" }, unknownFields: {}, sources: [], conflicts: [] }], losses: [], canPrepare: true, reason: null } };
const request = freezeContentImport(base, target, inspected, "op");
const prepared: ContentReceipt = { slot: "slot", operationId: "op", status: "prepared", planId: "plan", preview: { summary: message("error.api.hostedContentImport.planSummary"), recordCount: 1, losses: [], sharedWrites: [] }, nodeRefs: [], commitAllowed: true };
test("content import freezes original bytes and rejects ABA, unsupported format and occupied slot", () => {
  assert.equal(request.bytesBase64, base.bytesBase64); const later = { ...base, revision: 4 }; assert.throws(() => freezeContentImport(later, target, inspected, "new"), /同值/);
  assert.throws(() => freezeContentImport(base, { ...target, available: false }, inspected, "new"), /不可新增/);
  assert.throws(() => freezeContentImport({ ...base, format: "native" }, target, inspected, "new"), /格式/);
});
test("conflicts and typed mapping rejection never become prepare permission", () => {
  assert.throws(() => contentInspection({ ...inspected.value, records: [{ ...inspected.value.records[0], conflicts: [{ key: "title", current: publication, imported: publication }] }] }, digest));
  const rejected = contentInspection({ ...inspected.value, canPrepare: false, reason: { explanation: message("error.api.hostedContentImport.proposalNotReady"), diagnostics: [], causes: [] } }, digest); assert.throws(() => freezeContentImport(base, target, { revision: 2, value: rejected }, "new"), /检查/);
  assert.throws(() => contentInspection(inspected.value, "b".repeat(64)), /准确原件/);
});
test("cached receipt is inert; later input must be explicitly reviewed without changing original request", () => {
  const draft = { ...base, job: { request, fileName: "work.bib", revision: 2, receipt: prepared } };
  assert.equal(canCommitContent(draft, false, 2), false); assert.equal(canCommitContent(draft, true, 2), true);
  draft.revision = 4; assert.equal(canCommitContent(draft, true, 2), false); assert.equal(canCommitContent(draft, true, 4), true); assert.equal(draft.job.request.bytesBase64, request.bytesBase64);
});
test("filtered status removes old previews and locators while preserving committed fact", () => {
  const old = { ...prepared, status: "committed", planId: null, commitAllowed: false, nodeRefs: [{ workspaceId: "w", nodeId: "n" }] };
  const fresh = contentReceipt(request, { ...old, planId: null, preview: {}, nodeRefs: [] }, old); assert.deepEqual(fresh.preview, {}); assert.deepEqual(fresh.nodeRefs, []); assert.equal(fresh.status, "committed");
  assert.throws(() => contentReceipt(request, prepared, old), /事实/); assert.throws(() => contentReceipt(request, { ...prepared, operationId: "new" }), /原编号/);
  assert.equal(contentEnded("resolved"), true); assert.equal(contentEnded("recoveryPending"), false);
});
test("reload retains both original request and later file; malformed record or bytes stop new effect", () => {
  const later = { ...base, fileName: "later.bib", bytesBase64: btoa("later"), bytes: 5, revision: 4, job: { request, fileName: "work.bib", revision: 2, receipt: prepared } };
  const restored = decodeContentDraft(JSON.stringify(later)); assert.equal(restored.fileName, "later.bib"); assert.equal(restored.job!.request.bytesBase64, request.bytesBase64);
  assert.throws(() => decodeContentDraft(JSON.stringify({ ...later, bytes: 8 })), /解析/); assert.throws(() => contentBytes("YR=="), /不完整/);
});
test("staged target cache normalizes only clean buffers, preserving unresolved requests and divergent Draft bases", () => {
  const clean = { inputId:"view",viewInputId:"view",source: "old", baseSource: "old", definitionReservations: ["reservation"] }; assert.deepEqual(hostedOpenedDraft("new", clean), { ...clean, source: "new", baseSource: "new" }); assert.equal(clean.source, "old");
  const dirty = { source: "personal", baseSource: "old" }; assert.deepEqual(hostedOpenedDraft("new", dirty), dirty);
  const unknown = { ...clean, pending: { resource: "Shared", operationId: "original", source: "proposal", request: { inputRevision: 1, sessionId: "old-session", baseId: "old-base", operationId: "original", source: "proposal" } } }; assert.deepEqual(hostedOpenedDraft("new", unknown), unknown);
});
