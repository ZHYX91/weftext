import test from "node:test";
import assert from "node:assert/strict";
import { decodeExchangePlan, decodeExchangeReceipt, decodeExportError, decodeFrozenOfficeArtifact, decodeHostedImport, decodeHostedOutput, decodeLegacyCapabilities, decodeLegacyMetadata, decodeLocalOutput, decodeNativeLoss, decodeOfficePreview, decodeOutputLoss, decodeTemplateInspection, legacyMetadataUiText, officeLossUiText, officePreviewFacts, officePreviewNotices, outputFailureUiText } from "./office-wire";
import { locale, message, uiTextValue } from "./ui-locale";
import { OutputController } from "./export-state";
import { importReceipt, officeOutputPresentation, outputReceipt } from "./hosted-office-state";
import { templateInspectionFormat } from "./office-import-options";

const explanation = () => ({ explanation: message("common.operation", { operationId: "原号<key>😀" }), diagnostics: [{ origin: "externalTool", text: "ORIGINAL diagnostic 错误\r\n<&>" }], causes: [] });
const loss = () => ({ code: "unmapped", ...explanation(), productDiagnostics: Array.from({ length: 40 }, (_, index) => ({ explanation: message("common.operation", { operationId: String(index) }), diagnostics: [], causes: [] })) });
const artifact = () => ({ requestId: "original", status: "prepared" as const, filename: "作者名称.doc", contentType: "application/msword", length: 3, digest: "a".repeat(64), losses: [loss()], summary: message("common.save"), preview: null });
const metadata = () => ({ requestId: "original", status: "failed", code: "conversionFailed", ...explanation(), backend: "configured", backendVersion: null, direction: "docxToDoc", inputFormat: "docx", outputFormat: "doc", contentType: "application/msword", stage: "docxToLegacy", original: { filename: "作者原件.docx", length: 3, sha256: "a".repeat(64) }, intermediateDocx: null, generated: null, features: [{ feature: "comments", before: "present", after: "unverified", ...explanation() }], losses: [{ ...loss(), location: null }], requiresLossConfirmation: false, contentRoundtripVerified: false, nativeConversionRequiresSeparateReport: true });

test("losses have one typed protocol and retain every parallel product diagnostic", () => {
    const input = loss(), parsed = decodeOutputLoss(JSON.parse(JSON.stringify(input)));
    assert.equal(parsed.productDiagnostics.length, 40);
    for (const malformed of [{ code: "x", message: "old" }, { ...input, productDiagnostics: undefined }, { ...input, productDiagnostics: [null] }, { ...input, location: null }, { ...input, detail: "old" }, { ...input, explanation: { key: "unknown", params: {} } }]) assert.throws(() => decodeOutputLoss(malformed));
    assert.equal(decodeNativeLoss({ ...input, path: "original/source.typ" }).path, "original/source.typ");
    assert.throws(() => decodeNativeLoss(input));
    assert.throws(() => decodeExportError({ ...explanation(), productDiagnostics: [], code: "unexpected" }));
    locale.set("zh-CN"); const display = officeLossUiText(parsed), zh = uiTextValue(display);
    locale.set("en-US"); const en = uiTextValue(display);
    assert.notEqual(zh, en); assert.match(en, /Original operation/); assert.match(en, /ORIGINAL diagnostic 错误\r\n<&>/); assert.match(en, /39/);
    assert.deepEqual(parsed.diagnostics, input.diagnostics); locale.set("zh-CN");
});

test("template variants decode nulls explicitly while labels and binding names remain original", () => {
    const label = '{"key":"common.save","params":{}} 作者标签';
    const xlsx = { slots: [{ name: "binding.name", kind: "cell", label, description: message("common.save"), supported: false, reason: { ...explanation(), productDiagnostics: [] }, rows: 1, columns: 1, capacity: 1 }], warnings: [] };
    const parsed = decodeTemplateInspection(structuredClone(xlsx), "xlsx");
    locale.set("en-US"); assert.equal(parsed.slots[0].label, label); assert.equal(parsed.slots[0].name, "binding.name"); assert.equal(uiTextValue(parsed.slots[0].description), "Save");
    const docx = { slots: [{ name: "body", kind: "content", label, supported: true, placement: "body", description: null, reason: null }], warnings: [] };
    assert.equal(decodeTemplateInspection(docx, "docx").slots[0].reason, null);
    for (const format of ["doc", "wps"]) assert.equal(decodeTemplateInspection(JSON.parse(JSON.stringify(docx)), templateInspectionFormat(format)).slots[0].label, label);
    assert.throws(() => templateInspectionFormat("pdf"));
    assert.throws(() => decodeTemplateInspection(docx, "xlsx"));
    assert.throws(() => decodeTemplateInspection({ ...docx, warnings: ["old warning"] }, "docx"));
    assert.throws(() => decodeTemplateInspection({ ...docx, slots: [{ ...docx.slots[0], reason: "old reason" }] }, "docx")); locale.set("zh-CN");
});

test("Legacy public metadata is distinct from private receipts and keeps structured feature explanations", () => {
    const capabilities = { backends: [{ id: "configured", version: null, directions: [{ direction: "docToDocx", available: true, contentRoundtripVerified: false, ...explanation() }] }], ordinaryDocxRequiresOffice: false, executionLocation: "server" };
    assert.equal(decodeLegacyCapabilities(capabilities, "server").backends[0].version, null);
    assert.throws(() => decodeLegacyCapabilities(capabilities, "local"));
    assert.throws(() => decodeLegacyCapabilities({ ...capabilities, backends: [{ ...capabilities.backends[0], version: undefined }] }, "server"));
    const parsed = decodeLegacyMetadata(structuredClone(metadata()));
    assert.equal(parsed.losses[0].location, null); assert.equal(parsed.losses[0].productDiagnostics.length, 40);
    assert.throws(() => decodeLegacyMetadata({ ...metadata(), workspace: "private path" }));
    assert.throws(() => decodeLegacyMetadata({ ...metadata(), generated: { filename: "x", length: 3, sha256: "a".repeat(64), retainedPath: "private" } }));
    locale.set("en-US"); assert.match(legacyMetadataUiText(parsed).map(uiTextValue).join("\n"), /comments: Original operation/); locale.set("zh-CN");
});

test("preview decodes only producer slots, including Core warnings and engine diagnostics after cloning", () => {
    const authored = { key: "common.save", params: {} }, warning = { ...explanation(), details: { kind: "aggregate", groups: [{ kind: "product", diagnostics: [explanation()] }] } };
    const wire = { summary: message("common.save"), document: { source: "作者原文", warnings: [warning] }, query: { included: false, reason: message("common.close") }, warnings: [warning], diagnostics: [{ severity: "error", diagnostics: [{ origin: "typst", text: "unchanged compiler 中文" }], origin: null }], rows: [{ cells: { author: authored } }], embeddedQueries: [{ occurrenceId: "exact-occurrence", label: "作者querylabel", queryId: "original-query", scope: "previouslyDisplayedFrozenResult", cutId: "original-cut", coverage: "partial", freshness: "stale", limited: true, rowCount: 1, status: [message("common.close")], diagnostics: [warning], engineDiagnostics: [{ entityKey: "original-entity", diagnostics: [{ severity: "error", diagnostics: [{ origin: "typst", text: "engine evidence" }], origin: null }] }], productErrors: [{ entityKey: "original-product-entity", error: { code: "qualified", ...explanation() } }], columns: ["作者column"] }] };
    const parsed = decodeOfficePreview(JSON.parse(JSON.stringify(wire)))!;
    locale.set("en-US"); const notices = officePreviewNotices(parsed).map(uiTextValue).join("\n");
    assert.match(notices, /Save/); assert.match(notices, /Close/); assert.match(notices, /Original operation/); assert.match(notices, /unchanged compiler 中文/); assert.match(notices, /作者querylabel: Close/); assert.match(notices, /original-entity:.*engine evidence/s); assert.match(notices, /original-product-entity: Original operation/);
    const facts = officePreviewFacts(parsed) as typeof wire;
    assert.deepEqual(facts.rows[0].cells.author, authored); assert.equal(facts.document.source, "作者原文"); assert.equal("summary" in facts, false); assert.equal("warnings" in facts.document, false); assert.equal(facts.embeddedQueries[0].label, "作者querylabel"); assert.equal("status" in facts.embeddedQueries[0], false);
    assert.throws(() => decodeOfficePreview({ summary: { key: "unknown", params: {} } }));
    assert.throws(() => decodeOfficePreview({ warnings: ["old"] }));
    const interrupted = decodeOfficePreview({ requestId: "original", status: "unknown", code: "interrupted", executionLocation: "server", ...explanation() });
    assert.match(officePreviewNotices(interrupted).map(uiTextValue).join("\n"), /Original operation/); locale.set("zh-CN");
});

test("local and hosted output envelopes are closed variants and details are not lost", () => {
    const local = { id: "artifact", ...artifact(), expiresInSeconds: 3600 };
    const parsed = decodeLocalOutput(structuredClone(local)); assert.equal(parsed.status, "prepared");
    assert.throws(() => decodeHostedOutput(local)); assert.throws(() => decodeLocalOutput(artifact()));
    assert.deepEqual(decodeLocalOutput({ id: "original", requestId: "original", status: "pending" }), { id: "original", requestId: "original", status: "pending" });
    const failure = decodeLocalOutput({ id: "original", requestId: "original", status: "failed", code: "failed", ...explanation(), details: { kind: "core", code: "originalFailure", ...explanation(), details: { kind: "aggregate", groups: [{ kind: "product", diagnostics: [explanation()] }] } } });
    assert.equal(failure.status, "failed"); if (failure.status !== "failed") assert.fail();
    assert.match(uiTextValue(outputFailureUiText(failure)), /原号/);
    assert.equal(decodeLocalOutput({ id: "original", requestId: "original", status: "failed", code: "failed", ...explanation(), details: null }).status, "failed");
    assert.throws(() => decodeLocalOutput({ id: "original", requestId: "original", status: "failed", code: "failed", message: "old" }));
});

test("hosted terminal explanations cannot redeliver source or grant stale artifact authority", () => {
    const incoming = { requestId: "original", status: "failed", preview: { source: "protected old source", legacy: metadata() }, executionLocation: "server" };
    const parsed = outputReceipt("original", incoming);
    assert.equal(parsed.preview?.source, undefined); assert.equal(parsed.preview?.legacy?.code, "conversionFailed");
    assert.equal(officeOutputPresentation("original", parsed, undefined, 2), undefined);
    assert.equal(officeOutputPresentation("original", parsed, { requestId: "original", epoch: 1 }, 2), undefined);
    assert.equal(officeOutputPresentation("original", parsed, { requestId: "original", epoch: 2 }, 2)?.preview?.legacy?.code, "conversionFailed");
    assert.throws(() => outputReceipt("other", incoming));
    assert.throws(() => decodeHostedOutput({ ...artifact(), status: "expired" }));
    const locator = { requestId: "original", status: "confirmed", filename: "original.doc", contentType: "application/msword", length: 3, digest: "a".repeat(64) };
    assert.deepEqual(decodeFrozenOfficeArtifact(locator), locator);
    assert.throws(() => decodeHostedOutput(locator)); assert.throws(() => decodeFrozenOfficeArtifact({ ...locator, losses: [] }));
    assert.throws(() => outputReceipt("original", artifact(), decodeFrozenOfficeArtifact(locator)));
});

test("hosted import uses explicit null and authority flags without merging an old disclosure", () => {
    const request = { resource: "slot", operationId: "original", bytesBase64: "exact" };
    const prepared = { resource: "slot", operationId: "original", status: "prepared", planId: "plan", preview: { summary: message("common.save"), losses: [loss()] }, commitAllowed: true };
    assert.equal(importReceipt(request, JSON.parse(JSON.stringify(prepared))).preview.losses?.[0].productDiagnostics.length, 40);
    const redacted = { resource: "slot", operationId: "original", status: "prepared", planId: null, preview: {}, commitAllowed: false };
    assert.deepEqual(importReceipt(request, redacted, decodeHostedImport(prepared)), redacted);
    assert.throws(() => importReceipt(request, { ...prepared, planId: "replacement-after-redaction" }, decodeHostedImport(redacted), "plan"));
    assert.equal(importReceipt(request, prepared, decodeHostedImport(redacted), "plan").planId, "plan");
    assert.throws(() => importReceipt(request, { ...prepared, planId: "other" }, decodeHostedImport(prepared)));
    assert.throws(() => decodeHostedImport({ ...prepared, preview: { losses: [{ code: "old", message: "old" }] } }));
    assert.throws(() => decodeHostedImport({ ...prepared, preview: { losses: [{ ...loss(), location: "private server path" }] } }));
    assert.throws(() => decodeHostedImport({ ...redacted, commitAllowed: true }));
    assert.throws(() => decodeHostedImport({ resource: "slot", operationId: "original", status: "pending" }));
});

test("Core exchange plans and receipts do not accept old string losses or optional identities", () => {
    const plan = { operationId: "original", planId: "plan", kind: "importDocx", status: "prepared", changes: [], nodeMap: {}, newNodeId: null, revisions: {}, resourceMap: {}, losses: [{ path: "source", ...loss() }] };
    assert.equal(decodeExchangePlan(plan).losses[0].productDiagnostics.length, 40);
    assert.throws(() => decodeExchangePlan({ ...plan, losses: ["old"] }));
    assert.throws(() => decodeExchangePlan({ operationId: "original", planId: "plan", losses: [] }));
    const receipt = { operationId: "original", planId: "plan", status: "committed", path: "original-copy", nodeIds: ["created"] };
    assert.deepEqual(decodeExchangeReceipt(receipt), receipt); assert.throws(() => decodeExchangeReceipt({ status: "committed" }));
});

test("UI switching never enters a frozen output request and cloned query labels are explicitly rebranded", async () => {
    const requests: unknown[] = []; let attempts = 0;
    const controller = new OutputController({ id: () => "original", prepare: async request => { requests.push(structuredClone(request)); if (++attempts === 1) throw new Error("network lost"); return { id: "original", ...artifact(), expiresInSeconds: 3600 }; }, confirm: async () => ({ id: "original", ...artifact(), status: "confirmed", expiresInSeconds: 3600 }), download: async () => new Blob(["abc"]) }, { inputId: "test-input", sessionId: "s", source: "原始作者\r\n😀", serial: 1, baseId: "b", path: "original.typ", root: "root" }, { queryId: "q", label: message("common.save"), rowCount: 1, coverage: "complete", freshness: "current" });
    controller.setDraft({ outputLocale: "zh-CN" });
    await assert.rejects(controller.prepare()); locale.set("en-US");
    const result = await controller.prepare(); assert.deepEqual(requests[0], requests[1]); assert.equal((requests[1] as { outputLocale: string }).outputLocale, "zh-CN");
    assert.equal("label" in (requests[1] as object), false); assert.equal(uiTextValue(controller.draft.query!.label), "Save"); assert.equal(uiTextValue(result.summary), "Save");
    locale.set("zh-CN"); assert.equal(uiTextValue(result.summary), "保存"); assert.equal(controller.basis.source, "原始作者\r\n😀");
});
