import test from "node:test";
import assert from "node:assert/strict";
import { decodeContentInspection, decodeContentImportReceipt, decodeContentMembers, decodeContentTargets, decodePublicImportLoss, decodeResourceImportReceipt, importLossUiText } from "./hosted-import-wire";
import { canCommitContent, contentReceipt, decodeContentDraft, type ContentDraft } from "./hosted-content-import-state";
import { canCommitResourceImport, decodeResourceImportDraft, encodeResourceImportDraft, resourceImportInput, resourceImportReceipt, type ResourceImportDraft } from "./hosted-media-import-state";
import { descriptionUiText } from "./ui-diagnostics";
import { locale, message, uiTextValue } from "./ui-locale";

const digest = "a".repeat(64), original = "external original 诊断\u0000 العربية 😀";
const raw = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T;
const explanation = () => ({ explanation: message("common.unavailable"), diagnostics: [{ origin: "externalTool" as const, text: original }], causes: [] });
const loss = () => ({ code: "producer-native-code", ...explanation(), productDiagnostics: Array.from({ length: 48 }, (_, index) => ({ ...explanation(), diagnostics: [{ origin: "dependency", text: `parallel-original-${index}` }] })), location: "作者原路径/第1条" });
const publicLoss = () => ({ code: "generatedBibliographyNotInstalled", explanation: message("loss.core.hostedFeature.generatedBibliography"), diagnostics: [], causes: [], productDiagnostics: [] });
const content = () => ({ slot: "approved-slot", operationId: "original-operation", status: "prepared", planId: "original-plan", preview: { summary: message("error.api.hostedContentImport.planSummary"), losses: [publicLoss()], recordCount: 1, sharedWrites: ["schema.write", "tags.create"] }, nodeRefs: [{ workspaceId: "workspace", nodeId: "node" }], commitAllowed: true });
const resource = () => ({ collection: "attachments", owner: "original-owner", operationId: "resource-operation", status: "prepared", planId: "resource-plan", preview: { summary: message("error.api.hostedResourceCollections.importPlanSummary"), files: [{ name: "原图 العربية 😀.png", bytes: 3, contentType: "application/octet-stream", metadata: { kind: "unsupported", ...explanation() } }], totalBytes: 3 }, resourceRefs: [], assets: [], results: [], commitAllowed: true });
const collection = () => ({ collection: "attachments", owner: "original-owner", title: "作者配置标题", lifecycle: "live" as const, read: false, download: false, import: true, status: "ready" as const });

test("bibliography inspection decodes only declared reasons and losses, preserving original author objects", () => {
    const author = { key: "common.unavailable", params: {}, title: "pending", explanation: { key: "not-a-product-message", params: {} } };
    const record = { suggestedKey: "作者-key", publication: { kind: "record", value: [{ key: "title", value: { kind: "text", value: "open" } }] }, originalRecord: author, unknownFields: { raw: author }, sources: [{ path: "[0].title", originalField: "标题" }], conflicts: [] };
    const decoded = decodeContentInspection({ uploadDigest: digest, records: [record], losses: [raw(loss())], canPrepare: false, reason: raw(explanation()) });
    assert.equal(decoded.records![0].originalRecord, author);
    assert.equal(decoded.records![0].unknownFields, record.unknownFields);
    assert.deepEqual(decoded.records![0].publication, record.publication);
    assert.ok(uiTextValue(descriptionUiText(decoded.reason!)).includes(original));
    assert.equal(decoded.losses[0].productDiagnostics.length, 48);
    const display = importLossUiText(decoded.losses[0]);
    assert.ok(uiTextValue(display).includes("parallel-original-47"));
    assert.ok(uiTextValue(display).includes("作者原路径/第1条"));
    const restored = decodeContentInspection(structuredClone(decoded));
    assert.ok(uiTextValue(descriptionUiText(restored.reason!)).includes(original));
    assert.throws(() => decodeContentInspection({ ...decoded, reason: "legacy explanation" }));
    assert.throws(() => decodeContentInspection({ ...decoded, reason: null }));
    assert.throws(() => decodeContentInspection({ ...decoded, canPrepare: true }));
});

test("native and bibliography inspection are distinct closed endpoint variants", () => {
    assert.deepEqual(decodeContentInspection({ uploadDigest: digest, recordCount: 0, losses: [], canPrepare: true }), { uploadDigest: digest, recordCount: 0, losses: [], canPrepare: true });
    assert.throws(() => decodeContentInspection({ uploadDigest: digest, recordCount: 1, losses: [], canPrepare: true, reason: null }));
    assert.throws(() => decodeContentInspection({ uploadDigest: digest, records: [], losses: [], canPrepare: true }));
    assert.throws(() => decodeContentInspection({ uploadDigest: digest, recordCount: 1, losses: [{ message: "legacy" }], canPrepare: true }));
    assert.throws(() => decodeContentInspection({ uploadDigest: digest, recordCount: -1, losses: [], canPrepare: true }));
    assert.throws(() => decodeContentInspection({ uploadDigest: digest, records: [{ conflicts: [] }], losses: [], canPrepare: true, reason: null }));
});

test("public import losses admit finite safe producer meanings without guessing private diagnostics", () => {
    const decoded = decodePublicImportLoss(raw(publicLoss()));
    assert.equal(decoded.code, "generatedBibliographyNotInstalled");
    assert.throws(() => decodePublicImportLoss({ ...publicLoss(), code: "private-producer-code" }));
    assert.throws(() => decodePublicImportLoss({ ...publicLoss(), location: "private-path" }));
    assert.throws(() => decodePublicImportLoss({ code: "content-import-loss", message: "legacy" }));
    assert.throws(() => decodeContentImportReceipt({ ...content(), preview: { ...content().preview, summary: [message("common.unavailable")] } }));
    assert.throws(() => decodeContentImportReceipt({ ...content(), preview: { ...content().preview, privateDecision: "not public" } }));
});

test("content receipt cache rebrands messages and keeps original operation facts independent of interface language", () => {
    const request = { slot: "approved-slot", operationId: "original-operation", format: "bibtex" as const, bytesBase64: "AQID" };
    const draft: ContentDraft = { slot: request.slot, format: request.format, fileName: "later原件.bib", bytes: 3, bytesBase64: "BAUG", revision: 7, job: { request, fileName: "original原件.bib", revision: 2, receipt: contentReceipt(request, raw(content())) } };
    const restored = decodeContentDraft(JSON.stringify(draft)), persisted = JSON.stringify(restored), previous = locale.current;
    assert.equal(canCommitContent(restored, false, 7), false);
    try {
        locale.set("zh-CN"); const chinese = uiTextValue(restored.job!.receipt!.preview.summary!);
        locale.set("en-US"); assert.notEqual(uiTextValue(restored.job!.receipt!.preview.summary!), chinese);
        assert.equal(JSON.stringify(restored), persisted);
        assert.equal(restored.job!.request.bytesBase64, "AQID");
        assert.equal(restored.bytesBase64, "BAUG");
        assert.equal(restored.fileName, "later原件.bib");
    } finally { locale.set(previous); }
    const committed = contentReceipt(request, { ...content(), status: "committed", planId: null, preview: {}, nodeRefs: [], commitAllowed: false }, restored.job!.receipt);
    assert.equal(committed.status, "committed"); assert.deepEqual(committed.preview, {}); assert.deepEqual(committed.nodeRefs, []);
    assert.throws(() => contentReceipt(request, content(), committed));
    assert.throws(() => decodeContentImportReceipt({ ...content(), status: "cancelled" }));
    assert.throws(() => decodeContentDraft(JSON.stringify({ ...draft, job: { ...draft.job, phase: "retry-with-new-id" } })));
});

test("resource locator cache retains original file identity, nullable PDF facts and dynamic unsupported metadata without upload bytes", () => {
    const incoming = raw(resource()), request = { collection: incoming.collection, operationId: incoming.operationId, files: [{ name: incoming.preview.files[0].name, bytes: 3, sha256: digest }], wireDigest: digest };
    const draft: ResourceImportDraft = { version: 2, collection: request.collection, files: [{ ...request.files[0], fileName: "原件.png", bytesBase64: "AQID" }], revision: 4 };
    draft.pending = { request, owner: incoming.owner, input: resourceImportInput(draft), revision: 4, receipt: resourceImportReceipt(request, incoming) };
    const restored = decodeResourceImportDraft(encodeResourceImportDraft(draft));
    assert.equal(restored.files[0].bytesBase64, undefined);
    assert.equal(restored.pending!.receipt!.preview.files![0].metadata.kind, "unsupported");
    const metadata = restored.pending!.receipt!.preview.files![0].metadata;
    if (metadata.kind !== "unsupported") return;
    assert.ok(uiTextValue(descriptionUiText(metadata)).includes(original));
    assert.equal(canCommitResourceImport(restored, collection(), false), false);
    assert.equal(canCommitResourceImport(restored, collection(), true), true);
    assert.equal(canCommitResourceImport(restored, { ...collection(), import: false }, true), false);
    const pdf = decodeResourceImportReceipt({ ...incoming, preview: { ...incoming.preview, files: [{ ...incoming.preview.files[0], metadata: { kind: "pdf", pages: [{ width: 100, height: 200, rotation: 0, extractionId: null, extractedScalars: null }] } }] } });
    assert.equal(pdf.preview.files![0].metadata.kind, "pdf");
    assert.throws(() => decodeResourceImportReceipt({ ...incoming, preview: { ...incoming.preview, summary: "legacy" } }));
    assert.throws(() => decodeResourceImportReceipt({ ...incoming, preview: { ...incoming.preview, files: [{ ...incoming.preview.files[0], metadata: { kind: "unsupported", message: "legacy" } }] } }));
    assert.throws(() => decodeResourceImportDraft(JSON.stringify({ ...draft, pending: { ...draft.pending, request: { ...request, bytesBase64: "YR==" } } })));
});

test("filtered resource status replaces stale preview and terminal journal facts cannot become another outcome", () => {
    const incoming = resource(), request = { collection: incoming.collection, operationId: incoming.operationId, files: [{ name: incoming.preview.files[0].name, bytes: 3, sha256: digest }], wireDigest: digest }, first = resourceImportReceipt(request, incoming);
    const committed = resourceImportReceipt(request, { ...incoming, owner: "remapped-owner", status: "committed", planId: null, preview: {}, resourceRefs: [], assets: [], commitAllowed: false }, first);
    assert.equal(committed.status, "committed"); assert.deepEqual(committed.preview, {}); assert.deepEqual(committed.assets, []);
    assert.throws(() => resourceImportReceipt(request, { ...committed, status: "resolved" }, committed));
    assert.throws(() => resourceImportReceipt(request, incoming, committed));
    const cancelled = { ...committed, status: "cancelled" };
    assert.throws(() => resourceImportReceipt(request, { ...cancelled, status: "prepared" }, cancelled));
    assert.throws(() => decodeResourceImportReceipt({ ...committed, planId: "stale-plan" }));
});

test("catalog and member decoders preserve author titles and never turn locators into read permission", () => {
    const title = "pending 作者标题", targets = decodeContentTargets({ scope: "configuredImportSlots", targets: [{ id: "slot", title, formats: ["native"], available: false }] });
    assert.equal(targets[0].title, title);
    const members = decodeContentMembers({ scope: "configuredImportSlotMembers", coverage: "partial", members: [{ resource: "slot/node", nodeRef: { workspaceId: "workspace", nodeId: "node" }, title, status: "unavailable", canOpen: false }] }, "slot");
    assert.equal(members.members[0].title, title); assert.equal(members.members[0].canOpen, false);
    assert.throws(() => decodeContentMembers({ ...members, members: [{ ...members.members[0], canOpen: true }] }, "slot"));
    assert.throws(() => decodeContentMembers(members, "another-slot"));
});
