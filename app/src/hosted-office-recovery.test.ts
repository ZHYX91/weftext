import { TestDocument } from "./dom.test-support";
import test from "node:test";
import assert from "node:assert/strict";
import { HostedOffice } from "./hosted-office";
import type { OfficeImportReceipt, OfficeImportRequest, OfficeOutputReceipt, OfficeOutputRequest } from "./hosted-office-state";
import { locale, message, setUiText } from "./ui-locale";
import { OutputReadGate } from "./export-state";

const original = (format: "doc" | "wps" | "docx" = "doc"): OfficeImportRequest => ({ resource: "slot", operationId: "original-operation", bytesBase64: "ZXhhY3Qtb3JpZ2luYWw=", format, ...(format !== "docx" ? { legacyBackend: "configured" } : { docxOptions: { contentHash: "a".repeat(64), captionPairings: [] } }) });
const receipt = (request: OfficeImportRequest, status: string): OfficeImportReceipt => ({ resource: request.resource, operationId: request.operationId, status, planId: null, preview: {}, commitAllowed: false });
async function recover(request: OfficeImportRequest, status: string, authorized = true) {
  const calls: { path: string; body: unknown }[] = [], accepted: OfficeImportReceipt[] = [];
  const host = { epoch: 1, cache: { importJob: { request } }, authority: async () => authorized, acceptImport: (value: OfficeImportReceipt) => accepted.push(value),
    request: async (path: string, body: unknown) => { calls.push({ path, body: structuredClone(body) }); if (path === "office/import/operation") return receipt(request, status); if (path === "office/import/prepare") { (body as OfficeImportRequest).bytesBase64 = "transport mutation"; return { ...receipt(request, "prepared"), planId: "original-plan", preview: { summary: message("common.save"), losses: [] }, commitAllowed: true }; } return receipt(request, "cancelled"); },
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
  const request: OfficeOutputRequest = { outputLocale: "zh-CN", requestId: "original-output", resource: "slot", format: "doc", legacyBackend: "configured", reviewMode: "none", includeResolved: false, stylePolicy: "template", bindings: {} };
  const job = { request, receipt: undefined as OfficeOutputReceipt | undefined, phase: "prepare" as "prepare" | "confirm" | "download" | undefined, frozenArtifact: undefined as OfficeOutputReceipt | undefined };
  const host = { epoch: 1, cache: { output: job }, message: new TestDocument().createElement("p"), keep: () => true };
  const accept = (HostedOffice.prototype as unknown as { acceptOutput(this: unknown, value: OfficeOutputReceipt): void }).acceptOutput;
  accept.call(host, { requestId: request.requestId, status: "unknown", preview: { source: "unverified" } });
  assert.equal(job.phase, "prepare"); assert.deepEqual(job.receipt, { requestId: request.requestId, status: "unknown", preview: {} }); assert.equal(job.request, request);
  accept.call(host, { requestId: request.requestId, status: "cancelled", preview: null });
  assert.equal(job.phase, undefined); assert.equal(job.receipt?.status, "cancelled"); assert.equal(job.request, request); assert.match(host.message.textContent, /明确准备/);
});

test("an unknown output result cannot forget the identity or confirmation of an already frozen artifact", () => {
  const request: OfficeOutputRequest = { outputLocale: "zh-CN", requestId: "original-output", resource: "slot", format: "doc", legacyBackend: "configured", reviewMode: "none", includeResolved: false, stylePolicy: "template", bindings: {} };
  const job = { request, receipt: undefined as OfficeOutputReceipt | undefined, phase: "prepare" as "prepare" | "confirm" | "download" | undefined, frozenArtifact: undefined as OfficeOutputReceipt | undefined };
  const host = { epoch: 1, cache: { output: job }, message: new TestDocument().createElement("p"), keep: () => true };
  const accept = (HostedOffice.prototype as unknown as { acceptOutput(this: unknown, value: OfficeOutputReceipt): void }).acceptOutput;
  const artifact: OfficeOutputReceipt = { requestId: request.requestId, status: "prepared", filename: "original.doc", contentType: "application/msword", length: 3, digest: "a".repeat(64), losses: [], summary: message("common.save"), preview: null };
  accept.call(host, artifact); accept.call(host, { requestId: request.requestId, status: "unknown", preview: null });
  assert.throws(() => accept.call(host, { ...artifact, digest: "b".repeat(64) }), { code: "resultUnknown" });
  accept.call(host, { ...artifact, status: "confirmed" }); accept.call(host, { requestId: request.requestId, status: "unknown", preview: null });
  assert.throws(() => accept.call(host, artifact), { code: "resultUnknown" }); assert.equal(job.request, request);
});

test("authorization clear retires captured conversion explanations before a language event can run them", () => {
  const document = new TestDocument(), host = document.createElement("section"), editor = document.createElement("section"), status = document.createElement("p"), secret = document.createElement("p");
  document.body.append(host); host.append(editor, status); editor.append(secret); let reads = 0;
  setUiText(secret as unknown as Node, () => { reads++; return message("common.operation", { operationId: "prior private operation" }); });
  const target = { host, editor, message: status, templateGate: new OutputReadGate(), importGate: new OutputReadGate() };
  (HostedOffice.prototype as unknown as { clear(this: unknown, removeCache: boolean): void }).clear.call(target, false);
  const before = reads; locale.set("en-US"); assert.equal(reads, before); assert.equal(host.children.length, 0); assert.equal(status.textContent, ""); locale.set("zh-CN");
});

test("retrying a hosted import keeps the original accepted loss decision and rejects a substituted cached command", async () => {
  const request = original(), disclosure = { code: "safe-loss", explanation: message("common.close"), diagnostics: [], causes: [], productDiagnostics: [] };
  const current: OfficeImportReceipt = { ...receipt(request, "prepared"), planId: "original-plan", preview: { summary: message("common.save"), losses: [disclosure] }, commitAllowed: true };
  const command = { resource: request.resource, operationId: request.operationId, planId: "original-plan", acceptLosses: true }, calls: unknown[] = [];
  const job = { request, receipt: current, planId: "original-plan", commit: command };
  const host = { epoch: 1, cache: { importJob: job }, importVerified: { operationId: request.operationId, epoch: 1 }, persist() {}, authority: async () => true, acceptImport() {}, request: async (_path: string, body: unknown) => { calls.push(structuredClone(body)); return receipt(request, "committed"); } };
  const commit = (HostedOffice.prototype as unknown as { commitImport(this: unknown, accepted: boolean): Promise<void> }).commitImport;
  await commit.call(host, false); assert.deepEqual(calls, [command]); assert.equal(job.commit.acceptLosses, true);
  job.commit = { ...command, resource: "replacement-target" };
  await assert.rejects(commit.call(host, true), { code: "resultUnknown" }); assert.equal(calls.length, 1); assert.equal(job.request.resource, "slot");
});

test("retrying hosted confirmation preserves the frozen artifact and decision instead of accepting cache substitutions", async () => {
  const request: OfficeOutputRequest = { outputLocale: "en-US", requestId: "original", resource: "slot", format: "doc", legacyBackend: "configured", reviewMode: "none", includeResolved: false, stylePolicy: "template", bindings: {} };
  const current: OfficeOutputReceipt = { requestId: request.requestId, status: "prepared", filename: "original.doc", contentType: "application/msword", length: 3, digest: "a".repeat(64), summary: message("common.save"), preview: null, losses: [{ code: "safe-loss", explanation: message("common.close"), diagnostics: [], causes: [], productDiagnostics: [] }] };
  const command = { requestId: request.requestId, digest: current.digest!, acceptLosses: true }, calls: unknown[] = [], job = { request, receipt: current, confirm: command };
  const host = { epoch: 1, cache: { output: job }, outputVerified: { requestId: request.requestId, epoch: 1 }, owner: undefined, persist() {}, authority: async () => true, outputPath: () => "office/export/confirm", acceptOutput() {}, request: async (_path: string, body: unknown) => { calls.push(structuredClone(body)); return { ...current, status: "confirmed" }; } };
  const confirm = (HostedOffice.prototype as unknown as { confirmOutput(this: unknown, accepted: boolean): Promise<void> }).confirmOutput;
  await confirm.call(host, false); assert.deepEqual(calls, [command]); assert.equal(job.confirm.acceptLosses, true);
  job.confirm = { ...command, digest: "b".repeat(64) };
  await assert.rejects(confirm.call(host, true), { code: "resultUnknown" }); assert.equal(calls.length, 1); assert.equal(job.request.outputLocale, "en-US");
});
