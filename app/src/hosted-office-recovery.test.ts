import test from "node:test";
import assert from "node:assert/strict";
import { HostedOffice } from "./hosted-office";
import type { OfficeImportReceipt, OfficeImportRequest, OfficeOutputReceipt, OfficeOutputRequest } from "./hosted-office-state";

const original = (format: "doc" | "wps" | "docx" = "doc"): OfficeImportRequest => ({ resource: "slot", operationId: "original-operation", bytesBase64: "ZXhhY3Qtb3JpZ2luYWw=", format, ...(format !== "docx" ? { legacyBackend: "configured" } : { docxOptions: { contentHash: "a".repeat(64), captionPairings: [] } }) });
const receipt = (request: OfficeImportRequest, status: string): OfficeImportReceipt => ({ resource: request.resource, operationId: request.operationId, status });
async function recover(request: OfficeImportRequest, status: string, authorized = true) {
  const calls: { path: string; body: unknown }[] = [], accepted: OfficeImportReceipt[] = [];
  const host = { epoch: 1, cache: { importJob: { request } }, authority: async () => authorized, acceptImport: (value: OfficeImportReceipt) => accepted.push(value),
    request: async (path: string, body: unknown) => { calls.push({ path, body: structuredClone(body) }); if (path === "office/import/operation") return receipt(request, status); if (path === "office/import/prepare") { (body as OfficeImportRequest).bytesBase64 = "transport mutation"; return { ...receipt(request, "prepared"), planId: "original-plan", preview: { losses: [] } }; } return receipt(request, "cancelled"); },
  };
  const method = (HostedOffice.prototype as unknown as { recoverImport(this: unknown): Promise<void> }).recoverImport;
  await method.call(host); return { calls, accepted };
}

test("legacy recovery resumes the exact original request only after a durable recoveryPending result", async () => {
  for (const format of ["doc", "wps"] as const) {
    const request = original(format), before = structuredClone(request); const result = await recover(request, "recoveryPending");
    assert.deepEqual(result.calls, [{ path: "office/import/operation", body: { resource: request.resource, operationId: request.operationId } }, { path: "office/import/prepare", body: before }]);
    assert.deepEqual(request, before); assert.equal(result.accepted[0].status, "prepared"); assert.equal(result.accepted[0].operationId, request.operationId);
  }
});

test("running legacy work stays a lookup and DOCX recovery never starts an external replay", async () => {
  for (const [format, status] of [["doc", "pending"], ["wps", "pending"], ["docx", "recoveryPending"]] as const) {
    const result = await recover(original(format), status); assert.equal(result.calls.length, 1); assert.equal(result.accepted[0].status, status);
  }
  const legacyDocx = await recover(original("docx"), "notRegistered"); assert.deepEqual(legacyDocx.calls, [{ path: "office/import/operation", body: { resource: "slot", operationId: "original-operation" } }, { path: "office/import/settle", body: { resource: "slot", operationId: "original-operation", format: "docx" } }]); assert.equal(legacyDocx.accepted[0].status, "cancelled");
});

test("lost current authority prevents resuming an original external result", async () => {
  const result = await recover(original(), "recoveryPending", false); assert.equal(result.calls.length, 1); assert.deepEqual(result.accepted, []);
});

test("an unregistered original is fenced with its frozen format, including pre-format DOCX requests", async () => {
  for (const format of ["docx", "doc", "wps", undefined] as const) for (const status of ["notRegistered", "notCommitted"]) {
    const request = original(format ?? "docx"); if (format === undefined) delete request.format;
    const result = await recover(request, status);
    assert.deepEqual(result.calls[1], { path: "office/import/settle", body: { resource: request.resource, operationId: request.operationId, format: format ?? "docx" } });
    assert.equal(result.accepted[0].status, "cancelled"); assert.equal(request.format, format);
  }
});

test("manual termination sends only the original target, UUID and frozen format", async () => {
  for (const format of ["docx", "doc", "wps", undefined] as const) {
    const request = original(format ?? "docx"); if (format === undefined) delete request.format;
    const calls: unknown[] = [], accepted: OfficeImportReceipt[] = [];
    const host = { epoch: 1, cache: { importJob: { request }, importDraft: { format: "wps", resource: "later-selection" } }, authority: async () => true, acceptImport: (value: OfficeImportReceipt) => accepted.push(value), request: async (path: string, body: unknown) => { calls.push({ path, body }); return receipt(request, "cancelled"); } };
    await (HostedOffice.prototype as unknown as { endImport(this: unknown): Promise<void> }).endImport.call(host);
    assert.deepEqual(calls, [{ path: "office/import/settle", body: { resource: "slot", operationId: "original-operation", format: format ?? "docx" } }]); assert.equal(accepted[0].operationId, request.operationId);
  }
});

test("unknown output keeps its original request locked, while a durable cancellation permits an explicit next action", () => {
  const request: OfficeOutputRequest = { requestId: "original-output", resource: "slot", format: "doc", legacyBackend: "configured", reviewMode: "none", includeResolved: false, stylePolicy: "template", bindings: {} };
  const job = { request, receipt: undefined as OfficeOutputReceipt | undefined, phase: "prepare" as "prepare" | "confirm" | "download" | undefined, frozenArtifact: undefined as OfficeOutputReceipt | undefined };
  const host = { epoch: 1, cache: { output: job }, message: { textContent: "" }, keep: () => true };
  const accept = (HostedOffice.prototype as unknown as { acceptOutput(this: unknown, value: OfficeOutputReceipt): void }).acceptOutput;
  accept.call(host, { requestId: request.requestId, status: "unknown", preview: { source: "unverified" } });
  assert.equal(job.phase, "prepare"); assert.deepEqual(job.receipt, { requestId: request.requestId, status: "unknown" }); assert.equal(job.request, request);
  accept.call(host, { requestId: request.requestId, status: "cancelled" });
  assert.equal(job.phase, undefined); assert.equal(job.receipt?.status, "cancelled"); assert.equal(job.request, request); assert.match(host.message.textContent, /明确准备/);
});

test("an unknown output result cannot forget the identity or confirmation of an already frozen artifact", () => {
  const request: OfficeOutputRequest = { requestId: "original-output", resource: "slot", format: "doc", legacyBackend: "configured", reviewMode: "none", includeResolved: false, stylePolicy: "template", bindings: {} };
  const job = { request, receipt: undefined as OfficeOutputReceipt | undefined, phase: "prepare" as "prepare" | "confirm" | "download" | undefined, frozenArtifact: undefined as OfficeOutputReceipt | undefined };
  const host = { epoch: 1, cache: { output: job }, message: { textContent: "" }, keep: () => true };
  const accept = (HostedOffice.prototype as unknown as { acceptOutput(this: unknown, value: OfficeOutputReceipt): void }).acceptOutput;
  const artifact: OfficeOutputReceipt = { requestId: request.requestId, status: "prepared", filename: "original.doc", contentType: "application/msword", length: 3, digest: "a".repeat(64), losses: [] };
  accept.call(host, artifact); accept.call(host, { requestId: request.requestId, status: "unknown" });
  assert.throws(() => accept.call(host, { ...artifact, digest: "b".repeat(64) }), { code: "resultUnknown" });
  accept.call(host, { ...artifact, status: "confirmed" }); accept.call(host, { requestId: request.requestId, status: "unknown" });
  assert.throws(() => accept.call(host, artifact), { code: "resultUnknown" }); assert.equal(job.request, request);
});
