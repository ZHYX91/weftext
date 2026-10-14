import assert from "node:assert/strict";
import test from "node:test";
import { importReceipt, officeChunk, outputReceipt, type OfficeOutputReceipt } from "./hosted-office-state";

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
