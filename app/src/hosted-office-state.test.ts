import assert from "node:assert/strict";
import test from "node:test";
import { freezeOfficeImportRequest, importReceipt, officeChunk, officeOutputPresentation, outputReceipt, type OfficeImportRequest, type OfficeOutputReceipt } from "./hosted-office-state";

const prepared: OfficeOutputReceipt = { requestId: "original", status: "prepared", filename: "report.docx", contentType: "application/docx", length: 6, digest: "a".repeat(64), losses: [] };
test("same original output cannot install replacement bytes from a late or inconsistent receipt", () => {
  assert.throws(() => outputReceipt("original", { ...prepared, requestId: "other" }));
  assert.throws(() => outputReceipt("original", { ...prepared, digest: "b".repeat(64) }, prepared));
  assert.throws(() => outputReceipt("original", { ...prepared, length: 7 }, prepared));
  const confirmed = outputReceipt("original", { requestId: "original", status: "confirmed" }, prepared);
  assert.equal(confirmed.digest, prepared.digest); assert.equal(confirmed.status, "confirmed");
  assert.throws(() => outputReceipt("original", prepared, confirmed));
  for (const status of ["expired", "failed", "notRegistered"] as const) {
    assert.deepEqual(outputReceipt("original", { ...prepared, status, preview: { source: "old source" } }, confirmed), { requestId: "original", status });
  }
});
test("a resumed download checks exact offset, bounded size, total, digest and completion", () => {
  const chunk = { bytesBase64: btoa("def"), offset: 3, length: 3, total: 6, sha256: prepared.digest!, done: true };
  assert.deepEqual([...officeChunk(chunk, 3, 6, prepared.digest!)], [100, 101, 102]);
  for (const malformed of [{ ...chunk, offset: 0 }, { ...chunk, total: 7 }, { ...chunk, sha256: "b".repeat(64) }, { ...chunk, done: false }, { ...chunk, length: 4 }, { ...chunk, bytesBase64: "%%%" }]) assert.throws(() => officeChunk(malformed, 3, 6, prepared.digest!));
});
test("output cache never redelivers preview or confirm/download authority before original lookup in current epoch", () => {
  const cached = { ...prepared, preview: { source: "old protected object" } };
  assert.equal(officeOutputPresentation("original", cached, undefined, 9), undefined);
  assert.equal(officeOutputPresentation("original", cached, { requestId: "original", epoch: 8 }, 9), undefined);
  assert.equal(officeOutputPresentation("original", cached, { requestId: "other", epoch: 9 }, 9), undefined);
  const verification = { requestId: "original", epoch: 9 };
  assert.equal(officeOutputPresentation("original", cached, verification, 9)?.preview, cached.preview);
  for (const status of ["notRegistered", "expired", "failed"] as const) {
    const restored = outputReceipt("original", { ...cached, status });
    assert.deepEqual(officeOutputPresentation("original", restored, verification, 9), { requestId: "original", status });
    assert.equal(officeOutputPresentation("original", restored, verification, 10), undefined);
  }
});
test("import recovery never rebinds an original operation or revives a committed effect", () => {
  const request = { resource: "slot", operationId: "original", bytesBase64: "" };
  const original = { resource: "slot", operationId: "original", status: "prepared", planId: "plan", preview: { losses: [] }, commitAllowed: true };
  assert.throws(() => importReceipt(request, { ...original, resource: "other" }));
  assert.throws(() => importReceipt(request, { ...original, operationId: "other" }));
  assert.throws(() => importReceipt(request, { ...original, planId: "replacement" }, original));
  const committed = importReceipt(request, { ...original, status: "committed" }, original);
  assert.throws(() => importReceipt(request, original, committed));
  assert.equal(importReceipt(request, { resource: "slot", operationId: "original", status: "prepared", commitAllowed: false }).commitAllowed, false);
});

test("pending output supplies only the original locator and cannot replace a prepared artifact", () => {
  const pending: OfficeOutputReceipt = { ...prepared, status: "pending", message: "still converting", preview: { source: "unverified replacement", legacyConversion: { backend: "other" } } };
  assert.deepEqual(outputReceipt("original", pending), { requestId: "original", status: "pending", message: "still converting" });
  for (const status of ["prepared", "confirmed"] as const) {
    const original = { ...prepared, status, preview: { legacyConversion: { backend: "configured", generated: { sha256: prepared.digest } } } };
    const before = structuredClone(original);
    assert.throws(() => outputReceipt("original", pending, original), { code: "resultUnknown" });
    assert.deepEqual(original, before);
  }
  assert.throws(() => outputReceipt("original", { ...prepared, status: "unsupported" } as unknown as OfficeOutputReceipt), { code: "resultUnknown" });
});

test("unknown and cancelled output receipts retain the original UUID without delivering cached artifacts", () => {
  for (const status of ["unknown", "cancelled"] as const) {
    const incoming = { ...prepared, status, message: "original result", preview: { source: "unverified source" } };
    assert.deepEqual(outputReceipt("original", incoming, prepared), { requestId: "original", status, message: "original result" });
    assert.throws(() => outputReceipt("other", incoming), { code: "resultUnknown" });
  }
});

test("incomplete output receipts never grant presentation or delivery authority", () => {
  const verification = { requestId: "original", epoch: 2 };
  for (const malformed of [
    { ...prepared, filename: "" }, { ...prepared, contentType: "" },
    { ...prepared, length: -1 }, { ...prepared, length: 64 * 1024 * 1024 + 1 }, { ...prepared, length: 1.5 },
    { ...prepared, digest: "not-a-digest" }, { ...prepared, losses: undefined },
    { ...prepared, losses: [{ code: "legacyOffice_commentsUnverified", message: null }] },
  ]) {
    assert.throws(() => officeOutputPresentation("original", malformed as OfficeOutputReceipt, verification, 2), { code: "resultUnknown" });
  }
});

test("Word import freezes the exact original, backend and caption decisions before later input changes", () => {
  for (const format of ["doc", "wps"] as const) {
    const input: OfficeImportRequest = { resource: "empty-slot", operationId: "original", bytesBase64: btoa(`original-${format}`), format, legacyBackend: "configured" };
    const frozen = freezeOfficeImportRequest(input);
    input.resource = "other-slot"; input.operationId = "replacement"; input.bytesBase64 = btoa("later file"); input.legacyBackend = "libreoffice";
    assert.deepEqual(frozen, { resource: "empty-slot", operationId: "original", bytesBase64: btoa(`original-${format}`), format, legacyBackend: "configured" });
  }
  const input: OfficeImportRequest = { resource: "empty-slot", operationId: "caption-original", bytesBase64: btoa("original-docx"), format: "docx", docxOptions: { contentHash: "a".repeat(64), captionPairings: [{ captionBlock: 2, objectBlock: 1, kind: "figure", captionText: "明确题注", position: "bottom" }] } };
  const frozen = freezeOfficeImportRequest(input);
  input.docxOptions!.contentHash = "b".repeat(64); input.docxOptions!.captionPairings[0].captionText = "later text"; input.docxOptions!.captionPairings.push({ captionBlock: 5, objectBlock: 4, kind: "table", captionText: "later choice", position: "top" });
  assert.deepEqual(frozen.docxOptions, { contentHash: "a".repeat(64), captionPairings: [{ captionBlock: 2, objectBlock: 1, kind: "figure", captionText: "明确题注", position: "bottom" }] });
  const existing = { resource: "empty-slot", operationId: "existing-docx", bytesBase64: btoa("original-docx") };
  assert.deepEqual(freezeOfficeImportRequest(existing), existing);
});

test("Word import refuses incompatible format choices without silently rewriting retained input", () => {
  const base: OfficeImportRequest = { resource: "empty-slot", operationId: "original", bytesBase64: btoa("original") };
  const captions = { contentHash: "a".repeat(64), captionPairings: [] };
  for (const input of [
    { ...base, resource: "" }, { ...base, operationId: "" }, { ...base, bytesBase64: "" },
    { ...base, format: "pdf" },
    { ...base, format: "doc" }, { ...base, format: "wps", legacyBackend: "" },
    { ...base, format: "docx", legacyBackend: "configured" }, { ...base, legacyBackend: "configured" },
    { ...base, format: "doc", legacyBackend: "configured", docxOptions: captions },
    { ...base, format: "wps", legacyBackend: "configured", docxOptions: captions },
    { ...base, format: "docx", docxOptions: { ...captions, contentHash: "old-original" } },
    { ...base, format: "docx", docxOptions: { ...captions, captionPairings: null } },
  ]) {
    const before = structuredClone(input);
    assert.throws(() => freezeOfficeImportRequest(input as OfficeImportRequest));
    assert.deepEqual(input, before);
  }
});

test("legacy import receipt stays tied to the original operation and cannot substitute its plan", () => {
  const request: OfficeImportRequest = { resource: "empty-slot", operationId: "legacy-original", bytesBase64: btoa("exact-doc"), format: "doc", legacyBackend: "configured" };
  const original = { resource: request.resource, operationId: request.operationId, status: "prepared", planId: "exact-plan", preview: { source: "converted document", losses: [{ code: "legacyOffice_commentsUnverified", message: "核对转换后的批注" }], legacyConversion: { status: "succeeded", backend: "configured", direction: "docToDocx", original: { filename: "original.doc", length: 9, sha256: "a".repeat(64) }, intermediateDocx: { filename: "converted.docx", length: 15, sha256: "b".repeat(64) } } }, commitAllowed: true };
  const before = structuredClone(request);
  assert.deepEqual(importReceipt(request, original), original);
  for (const replacement of [
    { ...original, resource: "other-slot" }, { ...original, operationId: "other-operation" },
    { ...original, planId: "replacement-plan" }, { ...original, status: "unknown" },
  ]) assert.throws(() => importReceipt(request, replacement, original), { code: "resultUnknown" });
  assert.deepEqual(request, before);
  for (const invalidPreview of [undefined, { source: "unverified" }, { losses: [{ code: "legacy", message: null }] }]) {
    assert.throws(() => importReceipt(request, { ...original, preview: invalidPreview } as Parameters<typeof importReceipt>[1]), { code: "resultUnknown" });
  }
});

test("pending import recovery hides unverified content while retaining the original plan identity", () => {
  const request: OfficeImportRequest = { resource: "empty-slot", operationId: "legacy-original", bytesBase64: btoa("exact-wps"), format: "wps", legacyBackend: "configured" };
  const original = { resource: request.resource, operationId: request.operationId, status: "prepared", planId: "exact-plan", preview: { source: "verified source", losses: [] }, commitAllowed: true };
  for (const status of ["pending", "recoveryPending"] as const) {
    const incoming = { resource: request.resource, operationId: request.operationId, status, planId: "unverified-plan", commitAllowed: true, preview: { source: "unverified replacement", losses: [{ code: "unverified", message: "unverified losses" }], legacyConversion: { backend: "other", original: { sha256: "b".repeat(64) } } } };
    assert.deepEqual(importReceipt(request, incoming), { resource: request.resource, operationId: request.operationId, status, commitAllowed: false });
    assert.throws(() => importReceipt(request, incoming, original), { code: "resultUnknown" });
    const recovered = importReceipt(request, { ...incoming, planId: undefined }, original);
    assert.deepEqual(recovered, { resource: request.resource, operationId: request.operationId, status, planId: "exact-plan", commitAllowed: false });
    assert.throws(() => importReceipt(request, { ...original, planId: "replacement-plan" }, recovered), { code: "resultUnknown" });
    assert.deepEqual(importReceipt(request, original, recovered), original);
  }
});
