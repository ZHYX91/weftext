import test from "node:test";
import assert from "node:assert/strict";
import { decodeLocalMediaResource, decodeResourceList, decodeResourceMetadata, decodeThumbnailResponse } from "./media-wire";
import { decodeMediaDescriptor, mediaFingerprint, mediaTicket, mediaResolved } from "./hosted-media-state";
import { decodeKnowledgeDelivery, decodeKnowledgeDiagnosticFields, decodeKnowledgeLoss, decodeKnowledgeTemplatePreview, knowledgeDeliveryErrors, knowledgeLossMessage } from "./knowledge-state";
import { coreDescriptionUiText, descriptionUiText, engineDiagnosticUiText } from "./ui-diagnostics";
import { locale, message, uiTextValue } from "./ui-locale";

const original = "original diagnostic 作者 \ud800";
const description = () => ({ explanation: message("common.unavailable"), diagnostics: [{ origin: "externalTool" as const, text: original }], causes: [] });
const raw = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const resource = () => ({ resourceId: "image", resource: "weftext-resource://workspace/image", nodeId: "node", name: "作者原名.png", revision: "revision", readBasis: "read-basis", bytes: 3, contentType: "image/png", metadata: { kind: "image" as const, width: 12, height: 9 }, annotationReady: true, currentness: "accepted" });
const hosted = () => ({ asset: "image", resourceId: "image", owner: "owner", resource: "weftext-resource://workspace/image", name: "作者原名", title: "作者标题", revision: "revision", bytes: 3, contentType: "image/png", metadata: { kind: "unsupported" as const, ...description() }, annotationReady: false, wholeAnnotationReady: true, currentness: "accepted" as const, lifecycle: "live" as const, download: false, status: "ready" as const });
const snapshot = () => ({ graph: { native: { key: "common.unavailable", params: {} } }, headingStructure: {}, links: {}, diagnostics: { coverage: "partial", issues: [], nodes: [] }, renderedLinks: [], propertyReferences: { coverage: "partial", items: [], diagnostics: [description()] }, citations: null, mentions: {}, annotations: [], annotationStatus: "invalid", annotationDiagnostics: [{ code: "annotationInvalid", ...description() }], templates: [], trash: [], warnings: [{ ...description(), details: { kind: "aggregate", groups: [{ kind: "product", diagnostics: [description()] }] } }], errors: [{ path: null, error: { code: "readFailed", ...description() } }] });

test("media metadata decodes every PDF page fact and rejects the old string protocol", () => {
    const pdf = { kind: "pdf", pages: [{ width: 100.5, height: 200, rotation: 90, extractionId: null, extractedScalars: null }, { width: 300, height: 400, rotation: 0, extractionId: "exact-extraction", extractedScalars: 0 }] };
    assert.deepEqual(decodeResourceMetadata(pdf), pdf);
    assert.throws(() => decodeResourceMetadata({ kind: "pdf", pages: [{ width: 100, height: 200, rotation: 0 }] }));
    assert.throws(() => decodeResourceMetadata({ ...pdf, pages: [{ ...pdf.pages[0], extractedScalars: -1 }] }));
    assert.throws(() => decodeResourceMetadata({ kind: "unsupported", message: "legacy" }));
    assert.throws(() => decodeResourceMetadata({ kind: "unsupported", ...description(), authorField: "unchanged" }));
    assert.deepEqual(decodeResourceMetadata({ kind: "audio", timebase: { numerator: 1000, denominator: 1 }, durationTick: 0 }), { kind: "audio", timebase: { numerator: 1000, denominator: 1 }, durationTick: 0 });
    assert.throws(() => decodeResourceMetadata({ kind: "video", width: 12, height: 9, timebase: { numerator: 1000, denominator: 0 }, durationTick: 1 }));
});

test("local resource list retains unknown byte/revision facts and all typed warnings", () => {
    const value = { resources: [{ ...resource(), revision: null, bytes: null, annotationReady: false, currentness: "observed" }], warnings: [description(), description()] };
    const list = decodeResourceList(raw(value));
    assert.equal(list.resources[0].bytes, null);
    assert.equal(list.resources[0].revision, null);
    assert.equal(list.resources[0].readBasis, "read-basis");
    assert.equal(list.resources[0].name, "作者原名.png");
    assert.equal(list.warnings.length, 2);
    assert.ok(uiTextValue(descriptionUiText(list.warnings[0])).includes(original));
    assert.throws(() => decodeResourceList({ resources: [] }));
    assert.throws(() => decodeResourceList({ resources: [], warnings: ["legacy"] }));
    assert.throws(() => decodeLocalMediaResource({ ...resource(), bytes: -1 }));
});

test("server thumbnails have exact nullable basis facts and never client loading/failure states", () => {
    const basis = { sourceRevision: null, sourceBaseId: "observed-basis", sourceBasisKind: "observed" };
    const ready = decodeThumbnailResponse({ ...basis, state: "ready", descriptor: resource(), previewBasis: "currentResourceAtRowSource" });
    assert.equal(ready.state, "ready");
    assert.equal(ready.sourceRevision, null);
    const unavailable = decodeThumbnailResponse({ sourceRevision: "commit", sourceBaseId: null, sourceBasisKind: "committed", state: "unavailable", ...raw(description()) });
    assert.equal(unavailable.state, "unavailable");
    assert.throws(() => decodeThumbnailResponse({ ...basis, state: "ready", descriptor: resource() }));
    assert.throws(() => decodeThumbnailResponse({ ...basis, sourceRevision: "other", state: "absent", ...description() }));
    for (const state of ["building", "failed"]) assert.throws(() => decodeThumbnailResponse({ ...basis, state, ...description() }));
    assert.throws(() => decodeThumbnailResponse({ ...basis, state: "absent", message: "legacy" }));
});

test("hosted descriptor and ticket reconstruction preserve metadata brands without language-dependent identity", () => {
    const decoded = decodeMediaDescriptor(raw(hosted()));
    assert.equal(decoded.status, "ready");
    if (decoded.status !== "ready") return;
    const before = mediaFingerprint(decoded);
    const cloned = decodeMediaDescriptor(structuredClone(decoded));
    assert.equal(cloned.status, "ready");
    if (cloned.status !== "ready" || cloned.metadata.kind !== "unsupported") return;
    assert.ok(uiTextValue(descriptionUiText(cloned.metadata)).includes(original));
    const observed = decodeMediaDescriptor({ ...hosted(), revision: null, currentness: "observed", wholeAnnotationReady: false });
    assert.equal(observed.status, "ready");
    if (observed.status === "ready") assert.equal(observed.revision, null);
    const ticket = mediaTicket(decoded, "view", raw({ ...hosted(), ticket: "ticket", purpose: "view", sha256: "a".repeat(64), expiresInMs: 1000 }));
    assert.equal(ticket.metadata.kind, "unsupported");
    if (ticket.metadata.kind !== "unsupported") return;
    const display = descriptionUiText(ticket.metadata), previous = locale.current;
    try {
        locale.set("zh-CN"); const chinese = uiTextValue(display);
        locale.set("en-US"); const english = uiTextValue(display);
        assert.notEqual(chinese, english);
        assert.ok(english.includes(original));
        assert.equal(mediaFingerprint(decoded), before);
        assert.equal(decoded.title, "作者标题");
    } finally { locale.set(previous); }
    assert.deepEqual(decodeMediaDescriptor({ asset: "missing", status: "unavailable", collection: "collection" }), { asset: "missing", status: "unavailable", collection: "collection" });
    assert.throws(() => mediaTicket(decoded, "view", { ...ticket, download: true }));
    const resolved = mediaResolved({ ...hosted(), viewerTarget: { resource: hosted().resource, revision: "revision", fragment: null, resourceFragment: { status: "none" } } }, hosted().resource);
    assert.equal(resolved.revision, "revision");
    assert.throws(() => mediaResolved({ ...resolved, viewerTarget: { ...resolved.viewerTarget, fragment: null, resourceFragment: { status: "found", pageIndex: 0 } } }, hosted().resource + "#page=1"));
});

test("Knowledge decodes separate diagnostic channels while retaining unrelated relationship/author objects", () => {
    const wire = raw(snapshot()), decoded = decodeKnowledgeDiagnosticFields(wire);
    assert.equal(decoded.graph, wire.graph);
    assert.equal(decoded.diagnostics, wire.diagnostics);
    assert.equal(decoded.errors[0].path, null);
    assert.equal(decoded.errors[0].error.code, "readFailed");
    assert.ok(uiTextValue(coreDescriptionUiText(decoded.warnings[0])).includes(original));
    assert.equal(decoded.propertyReferences.diagnostics.length, 1);
    const restored = decodeKnowledgeDiagnosticFields(structuredClone(decoded));
    assert.ok(uiTextValue(coreDescriptionUiText(restored.warnings[0])).includes(original));
    assert.throws(() => decodeKnowledgeDiagnosticFields({ ...wire, warnings: ["legacy"] }));
    assert.throws(() => decodeKnowledgeDiagnosticFields({ ...wire, errors: [{ error: wire.errors[0].error }] }));
    assert.throws(() => decodeKnowledgeDiagnosticFields({ ...wire, annotationDiagnostics: [{ ...description() }] }));
    assert.throws(() => decodeKnowledgeDiagnosticFields({ ...wire, propertyReferences: { ...wire.propertyReferences, diagnostics: [wire.warnings[0]] } }));
});

test("Knowledge losses retain every parallel product diagnostic and exact native path after JSON", () => {
    const loss = decodeKnowledgeLoss(raw({ path: "原始/文件.typ", code: "unsupported", ...description(), productDiagnostics: Array.from({ length: 48 }, (_, index) => ({ ...description(), diagnostics: [{ origin: "dependency", text: `original-${index}` }] })) }));
    const display = knowledgeLossMessage(loss);
    assert.equal(loss.productDiagnostics.length, 48);
    assert.ok(uiTextValue(display).includes("原始/文件.typ"));
    assert.ok(uiTextValue(display).includes("original-47"));
    assert.throws(() => decodeKnowledgeLoss("legacy"));
    assert.throws(() => decodeKnowledgeLoss({ code: "old", message: "legacy" }));
});

test("template Engine diagnostics and delivery failures retain their own facts", () => {
    const diagnostic = { severity: "warning", diagnostics: [{ origin: "typst", text: original }], origin: null };
    const preview = decodeKnowledgeTemplatePreview({ source: "作者原文", tree: null, diagnostics: [diagnostic], ok: false, previewBasis: "registeredSource", sourceNode: null });
    assert.equal(preview.source, "作者原文");
    assert.ok(uiTextValue(engineDiagnosticUiText(preview.diagnostics[0])).includes(original));
    assert.throws(() => decodeKnowledgeTemplatePreview({ ...preview, diagnostics: [{ severity: "warning", message: "legacy", origin: null }] }));
    const receipt = decodeKnowledgeDelivery(raw({ status: "committed", operationId: "original", documentError: { code: "deliveryFailed", ...description() }, affectedDocuments: [{ priorSessionId: "other", documentDisposition: "rebound", documentError: { code: "otherDeliveryFailed", ...description() } }] }));
    assert.equal(receipt.status, "committed");
    assert.equal(receipt.operationId, "original");
    assert.equal(knowledgeDeliveryErrors(receipt).length, 2);
    assert.throws(() => decodeKnowledgeDelivery({ status: "committed", documentError: { message: "legacy" } }));
});
