import test from "node:test";
import assert from "node:assert/strict";
import { decodeApiError, apiErrorUiText } from "./api-error";
import { TestDocument } from "./dom.test-support";
import { HostedLibrary } from "./hosted-library";
import { HostedCitations, type HostedCitationHost } from "./hosted-citations";
import { citationPatches, decodeCitationCatalog, decodeCitationDrafts } from "./hosted-citations-state";
import type { HostedLinksContext } from "./hosted-knowledge-links-state";
import { decodeNativeCitationSnapshot } from "./native-citations";
import { decodeProcessingDocument, defaultProcessingDocument, ProcessingSettingsController } from "./processing-settings-panel";
import { NativeStyleController, nativeLanguageOptions, nativeStyleControlReason } from "./style-panel";
import { decodeAppliedLibraryStyle, decodeLibraryCatalog, decodeLibraryProfile, decodeLibraryReceipt, decodeLibraryStylePlan, decodeLibraryStyleState, decodeNativeStyleControl, decodeProcessingOperation, decodeProcessingRead } from "./style-wire";
import { locale, message, setUiText, uiTextValue } from "./ui-locale";

const original = '#let library-style(body) = {\r\n set text(lang: "zh", region: "CN", size: 12pt)\r\n // 作者😀\r\n body\r\n}';
const description = () => ({ explanation: message("common.operation", { operationId: "original😀" }), diagnostics: [{ origin: "dependency", text: "original tool 错误\r\n<&>" }], causes: [] });
const control = () => ({ key: "text.lang", label: message("common.save"), kind: "text", value: '"zh"', editable: true, reason: null });
const state = () => ({ path: "Styles/library-style.typ", source: original, baseId: "base", enabled: true, entryFunction: "library-style", resourceRef: null, installed: true, sourceHash: "a".repeat(64), syntaxValid: true, controls: [control()], warnings: [message("common.close")] });
const plan = () => ({ operationId: "original", planId: "plan", beforeSource: original, afterSource: original.replace('"zh"', '"en"'), enabled: true, status: "prepared", summary: [message("common.save")] });
const receipt = () => ({ operationId: "original", planId: "plan", status: "committed", path: "Styles/library-style.typ", nodeIds: [] });
const settings = () => ({ baseId: "base", roleRef: "workspace-settings", revision: null, beforeSource: '{"version":1,"custom":"作者😀"}', processing: null, warning: null, source: "shared-library-suggestion" });
const operation = (operationId: string, status = "prepared") => ({ operationId, resource: "library.processing", status, planId: "plan", preview: { beforeSource: settings().beforeSource, afterSource: '{"version":1,"custom":"作者😀","processing":{}}', summary: message("common.save") }, intentKind: "updateFiles", resourceRefs: [], nodeRefs: [] });
function storage() { const items = new Map<string, string>(); return { getItem: (key: string) => items.get(key) ?? null, setItem: (key: string, value: string) => { items.set(key, value); }, removeItem: (key: string) => { items.delete(key); } }; }

test("native style labels are explicitly decoded while source and locale expressions stay exact", () => {
    const raw = JSON.parse(JSON.stringify(state())), decoded = decodeLibraryStyleState(raw);
    locale.set("en-US"); assert.equal(uiTextValue(decoded.controls[0].label), "Save"); assert.equal(uiTextValue(decoded.warnings[0]), "Close");
    assert.equal(decoded.source, original); assert.equal(decoded.controls[0].value, '"zh"');
    assert.equal(nativeLanguageOptions("text.lang", "custom-native-expression")[0].value, "custom-native-expression");
    assert.equal(nativeLanguageOptions("text.lang", '"zh"').find(value => value.value === '"en"')?.value, '"en"');
    const authorDescriptor = '{"key":"common.save","params":{}}'; assert.equal(decodeNativeStyleControl({ ...control(), value: authorDescriptor }).value, authorDescriptor);
    const unavailable = decodeNativeStyleControl({ ...control(), editable: false, reason: description() });
    assert.match(uiTextValue(nativeStyleControlReason(unavailable.reason!)), /Original operation/); assert.match(uiTextValue(nativeStyleControlReason(unavailable.reason!)), /original tool 错误\r\n<&>/);
    for (const malformed of [{ ...control(), label: "old label" }, { ...control(), reason: "old reason" }, { ...control(), kind: "number", value: 12 }, { ...control(), reason: undefined }, { ...control(), label: { key: "unknown", params: {} } }]) assert.throws(() => decodeNativeStyleControl(malformed));
    assert.throws(() => decodeLibraryStyleState({ ...state(), warnings: ["old warning"] })); locale.set("zh-CN");
});

test("local plans and committed refreshes have closed typed envelopes and full original identities", () => {
    const decoded = decodeLibraryStylePlan(structuredClone(plan())); locale.set("en-US"); assert.equal(uiTextValue(decoded.summary[0]), "Save"); locale.set("zh-CN");
    assert.equal(decoded.beforeSource, original); assert.equal(decoded.afterSource, plan().afterSource);
    assert.throws(() => decodeLibraryStylePlan({ ...plan(), summary: ["old summary"] }));
    assert.throws(() => decodeLibraryStylePlan({ ...plan(), unexpected: true }));
    assert.throws(() => decodeLibraryStylePlan({ ...plan(), enabled: undefined }));
    const applied = decodeAppliedLibraryStyle({ ...state(), operationReceipt: receipt() }); assert.equal(applied.operationReceipt.planId, "plan");
    assert.throws(() => decodeAppliedLibraryStyle({ ...state(), operationReceipt: { operationId: "original", planId: "plan", status: "committed" } }));
    assert.throws(() => decodeLibraryStyleState({ ...state(), sourceHash: "bad" }));
});

test("style retries freeze raw native inputs and UI locale never enters the transaction", async () => {
    const requests: unknown[] = []; let calls = 0;
    const controller = new NativeStyleController({ read: async () => state(), operationId: () => "original", prepare: async request => { requests.push(structuredClone(request)); if (++calls === 1) { request.changes!["text.lang"] = "transport mutation"; throw new Error("network lost"); } return structuredClone(plan()); }, apply: async () => ({ ...state(), source: plan().afterSource, baseId: "new", operationReceipt: receipt() }) });
    await controller.load(); controller.setControl("text.lang", '"en"'); await assert.rejects(controller.prepare());
    locale.set("en-US"); const prepared = await controller.prepare();
    assert.deepEqual(requests, [{ operationId: "original", baseId: "base", changes: { "text.lang": '"en"' } }, { operationId: "original", baseId: "base", changes: { "text.lang": '"en"' } }]);
    assert.equal(uiTextValue(prepared.summary[0]), "Save"); assert.equal(controller.state!.source, original); assert.throws(() => controller.setControl("text.lang", 12 as unknown as string));
    await controller.apply(); assert.equal(controller.state!.source, plan().afterSource); locale.set("zh-CN");
});

test("only an explicitly typed committed-receipt error can confirm a failed style refresh", async () => {
    let changed = 0, fail = true;
    const failure = decodeApiError({ code: "style_state_unavailable", ...description(), details: { kind: "committedReceipt", receipt: receipt(), cause: { code: "freshReadFailed", ...description() } } })!;
    const controller = new NativeStyleController({ read: async () => state(), operationId: () => "original", prepare: async () => plan(), apply: async () => { if (fail) throw failure; return { ...state(), source: plan().afterSource, operationReceipt: receipt() }; }, committed: () => { changed++; } });
    await controller.load(); controller.setControl("text.lang", '"en"'); await controller.prepare(); await assert.rejects(controller.apply());
    assert.equal(controller.committedPendingRead, true); assert.equal(changed, 1); fail = false; await controller.apply(); assert.equal(changed, 1);
    const forged = new NativeStyleController({ read: async () => state(), operationId: () => "original", prepare: async () => plan(), apply: async () => { throw Object.assign(new Error("not a typed receipt"), { code: "style_state_unavailable", details: { receipt: receipt() } }); }, committed: () => { assert.fail("unknown objects cannot establish a committed effect"); } });
    await forged.load(); forged.setControl("text.lang", '"en"'); await forged.prepare(); await assert.rejects(forged.apply()); assert.equal(forged.committedPendingRead, false);
});

test("hosted role titles and style warnings are product fields but directory titles are authored", () => {
    const authored = '{"key":"common.save","params":{}} 作者目录';
    const catalog = decodeLibraryCatalog({ roles: [{ role: "style", title: message("common.save"), read: true, write: true }], styleUse: true, reorderNodes: true, orderParents: [{ reference: "weftext-node://w/n", title: authored }] });
    const profile = decodeLibraryProfile({ role: "style", basis: "opaque", source: original, installed: true, enabled: true, controls: [control()], warnings: [description()] });
    locale.set("en-US"); assert.equal(uiTextValue(catalog.roles[0].title), "Save"); assert.equal(catalog.orderParents[0].title, authored); assert.equal(profile.source, original);
    assert.equal(profile.warnings![0].diagnostics[0].text, description().diagnostics[0].text);
    assert.throws(() => decodeLibraryProfile({ role: "schema", basis: "opaque", source: "{}", installed: true, warnings: [] }));
    assert.throws(() => decodeLibraryCatalog({ ...catalog, roles: [{ ...catalog.roles[0], title: "old title" }] })); locale.set("zh-CN");
});

test("hosted prepared summary is one message and filtered receipts cannot recreate old authority", () => {
    const prepared = { role: "style", operationId: "original", status: "prepared", planId: "plan", preview: { beforeSource: original, afterSource: plan().afterSource, summary: message("common.save"), enabled: true }, commitAllowed: true };
    const decoded = decodeLibraryReceipt(JSON.parse(JSON.stringify(prepared))); locale.set("en-US"); assert.equal(uiTextValue(decoded.preview.summary), "Save"); locale.set("zh-CN");
    assert.deepEqual(decodeLibraryReceipt({ ...prepared, planId: null, preview: {}, commitAllowed: false }).preview, {});
    assert.throws(() => decodeLibraryReceipt({ ...prepared, preview: { ...prepared.preview, summary: [message("common.save")] } }));
    assert.throws(() => decodeLibraryReceipt({ ...prepared, commitAllowed: false }));
    assert.throws(() => decodeLibraryReceipt({ ...prepared, preview: {}, commitAllowed: true }));
    assert.throws(() => decodeLibraryReceipt({ ...prepared, planId: null, preview: { summary: message("common.save") }, commitAllowed: false }));
    assert.throws(() => decodeLibraryReceipt({ ...prepared, status: "notRegistered", planId: null, commitAllowed: false }));
});

test("processing warnings retain invalid author values and all original diagnostics across JSON recovery", () => {
    const invalid = { schemaVersion: 900, custom: ["原始作者😀", { key: "common.save", params: {} }] };
    const warning = { code: "invalid_request", ...description(), details: { kind: "productDiagnostics", code: null, diagnostics: Array.from({ length: 40 }, description) } };
    const wire = { ...settings(), processing: invalid, warning }, decoded = decodeProcessingRead(JSON.parse(JSON.stringify(wire)), decodeProcessingDocument);
    assert.deepEqual(decoded.processing, invalid); assert.equal(decoded.beforeSource, settings().beforeSource);
    locale.set("en-US"); assert.match(uiTextValue(apiErrorUiText(decoded.warning!)), /Original operation/); assert.match(uiTextValue(apiErrorUiText(decoded.warning!)), /original tool 错误/); locale.set("zh-CN");
    assert.throws(() => decodeProcessingRead({ ...wire, warning: null }, decodeProcessingDocument));
    assert.throws(() => decodeProcessingRead({ ...settings(), warning: "old warning" }, decodeProcessingDocument));
    assert.throws(() => decodeProcessingRead({ ...settings(), warning: undefined }, decodeProcessingDocument));
});

test("processing operation decoder distinguishes the real notRegistered and actor fence variants", () => {
    const pending = decodeProcessingOperation(structuredClone(operation("original"))); locale.set("en-US"); assert.equal(uiTextValue(pending.preview!.summary), "Save"); locale.set("zh-CN");
    assert.deepEqual(decodeProcessingOperation({ operationId: "original", status: "notRegistered", planId: null }), { operationId: "original", status: "notRegistered", planId: null });
    assert.equal(decodeProcessingOperation({ ...operation("original", "cancelled"), planId: null, preview: null, intentKind: "unknown" }).status, "cancelled");
    assert.throws(() => decodeProcessingOperation({ ...operation("original"), resource: "other-resource" }));
    assert.throws(() => decodeProcessingOperation({ operationId: "original", status: "prepared", planId: "plan" }));
    assert.throws(() => decodeProcessingOperation({ ...operation("original"), message: "old" }));
});

test("processing recovery rebrands delivered facts while replaying only its original author request", async () => {
    const retained = storage(), calls: unknown[] = [];
    const host = { persistenceKey: "current-library:processing", read: async () => settings(), prepare: async (request: { operationId: string }) => { calls.push(structuredClone(request)); return operation(request.operationId); }, status: async (request: { operationId: string }) => operation(request.operationId), settle: async (request: { operationId: string }) => operation(request.operationId, "cancelled"), apply: async ({ request }: { request: { operationId: string } }) => operation(request.operationId, "committed") };
    const controller = new ProcessingSettingsController(host, retained); await controller.load(); const document = defaultProcessingDocument(); document.defaults.languages = [{ language: "en", script: null, region: "GB" }]; controller.edit(JSON.stringify(document)); await controller.prepare();
    const before = structuredClone(controller.pending!.request), reopened = new ProcessingSettingsController(host, retained);
    locale.set("en-US"); assert.equal(uiTextValue(reopened.pending!.operation!.preview!.summary), "Save"); await reopened.retryPrepare(); assert.deepEqual(calls[0], before); assert.deepEqual(calls[1], before); assert.equal(reopened.pending!.request.beforeSource, settings().beforeSource); locale.set("zh-CN");
});

test("malformed processing recovery preserves its raw record without installing a partial basis or request", () => {
    const retained = storage(), base = { state: settings(), source: original, pending: { request: { operationId: "original", baseId: "base", beforeSource: settings().beforeSource, processing: defaultProcessingDocument() }, operation: operation("original") } };
    const host = { persistenceKey: "current-library:processing", read: async () => settings(), prepare: async () => operation("original"), status: async () => operation("original"), settle: async () => operation("original", "cancelled"), apply: async () => operation("original", "committed") };
    for (const mutate of [(value: any) => { value.pending.request.processing = false; }, (value: any) => { value.pending.operation.preview.summary = "old summary"; }, (value: any) => { value.pending.operation = false; }, (value: any) => { value.pending = false; }]) {
        const value = structuredClone(base); mutate(value); const raw = JSON.stringify(value); retained.setItem(host.persistenceKey, raw);
        const reopened = new ProcessingSettingsController(host, retained);
        assert.equal(reopened.recoveryError, raw); assert.equal(reopened.state, null); assert.equal(reopened.pending, undefined); assert.equal(reopened.source, ""); assert.equal(reopened.locked, true); assert.equal(retained.getItem(host.persistenceKey), raw);
    }
});

test("hosted cache presentation cannot commit until its original operation is verified in this connection", async () => {
    const calls: unknown[] = [], beforeSource = original, afterSource = plan().afterSource;
    const draft = { source: beforeSource, baseSource: beforeSource, enabled: true, baseEnabled: true, changes: { "text.lang": '"en"' }, revision: 1, pending: { request: { role: "style", basis: "opaque", operationId: "original", changes: { "text.lang": '"en"' } }, input: { source: beforeSource, changes: { "text.lang": '"en"' }, enabled: true }, revision: 1, planId: "plan", receipt: { role: "style", operationId: "original", status: "prepared", planId: "plan", preview: { beforeSource, afterSource, summary: message("common.save") }, commitAllowed: true }, commit: undefined as { role: string; operationId: string; planId: string } | undefined } };
    const host = { epoch: 1, draft, verifiedOperation: undefined as string | undefined, writable: () => true, needsComparison: () => false, retain() {}, install() {}, request: async (_path: string, value: { operationId: string }) => { calls.push(structuredClone(value)); value.operationId = "transport mutation"; return {}; } };
    const commit = (HostedLibrary.prototype as unknown as { commit(this: unknown): Promise<void> }).commit;
    await assert.rejects(commit.call(host)); assert.deepEqual(calls, []);
    host.verifiedOperation = "original"; await commit.call(host); await commit.call(host); assert.deepEqual(calls, [{ role: "style", operationId: "original", planId: "plan" }, { role: "style", operationId: "original", planId: "plan" }]);
    assert.equal(draft.pending.commit!.operationId, "original");
});

test("authorization clear retires detached reusable labels and clears original values synchronously", () => {
    const document = new TestDocument(), host = document.createElement("section"), editor = document.createElement("section"), source = document.createElement("textarea"), messageNode = document.createElement("p");
    document.body.append(host); host.append(editor, messageNode); editor.append(source); source.value = original; let reads = 0;
    setUiText(messageNode as unknown as Node, () => { reads++; return message("common.operation", { operationId: "prior operation" }); });
    const target = { host, editor, source, message: messageNode, picker: document.createElement("select"), enabled: document.createElement("input"), orderHost: document.createElement("section"), actions: document.createElement("section"), inputScopes: { clear() {} }, pendingRoles: new Set(), epoch: 1 };
    (HostedLibrary.prototype as unknown as { clear(this: unknown, remove: boolean): void }).clear.call(target, false);
    const before = reads; locale.set("en-US"); assert.equal(reads, before); assert.equal(source.value, ""); assert.equal(messageNode.textContent, ""); assert.equal(host.children.length, 0); locale.set("zh-CN");
});

test("a newly redacted hosted receipt retires its prior summary before the next locale change", () => {
    const previousDocument = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new TestDocument(), actions = document.createElement("section"), oldSummary = document.createElement("p");
    document.body.append(actions); actions.append(oldSummary); let reads = 0;
    setUiText(oldSummary as unknown as Node, () => { reads++; return message("common.operation", { operationId: "retired private operation" }); });
    Object.defineProperty(globalThis, "document", { configurable: true, value: document });
    try {
        const target = { actions, role: "style", busy: false, verifiedOperation: "original", writable: () => false, needsComparison: () => false, draft: { source: original, changes: {}, baseSource: original, revision: 0, pending: { request: { role: "style", operationId: "original" }, revision: 0, receipt: decodeLibraryReceipt({ role: "style", operationId: "original", status: "prepared", planId: null, preview: {}, commitAllowed: false }) } } };
        (HostedLibrary.prototype as unknown as { renderActions(this: unknown): void }).renderActions.call(target);
        const before = reads; locale.set("en-US"); assert.equal(reads, before); assert.doesNotMatch(actions.textContent, /retired private operation/); assert.equal(actions.find("button", uiTextValue(message("hostedLibrary.confirmOriginalConfigurationPlan"))).disabled, true);
    }
    finally {
        locale.set("zh-CN");
        if (previousDocument) Object.defineProperty(globalThis, "document", previousDocument); else Reflect.deleteProperty(globalThis, "document");
    }
});

test("hosted citation catalog decodes its actual capability branches without inventing reason or title placeholders", () => {
    const title = '{"key":"common.close","params":{}} 作者😀', ready = { resource: "work", reference: "entity:work", title, status: "ready", use: true, export: true, download: true }, unavailable = { resource: "unavailable", title: "原题", status: "unavailable", use: true, export: false, download: false };
    const decoded = decodeCitationCatalog(JSON.parse(JSON.stringify({ works: [ready, unavailable], coverage: "partial" })));
    assert.equal(decoded.works[0].title, title); assert.equal(decoded.works[1].status, "unavailable");
    for (const invalid of [{ ...ready, title: null }, { ...ready, use: undefined }, { ...ready, reason: null }, { ...ready, titlePlaceholder: message("common.close") }, { ...ready, export: "false" }, { ...unavailable, reference: "entity:work" }, { ...unavailable, export: true }]) assert.throws(() => decodeCitationCatalog({ works: [invalid], coverage: "complete" }));
    const cache = JSON.parse('{"__proto__":{"key":"作者😀","work":"entity:work"}}'), drafts = decodeCitationDrafts(cache); assert.equal(Object.getPrototypeOf(drafts), null); assert.equal(drafts.__proto__.key, "作者😀"); assert.deepEqual(JSON.parse(JSON.stringify(drafts)), cache);
    for (const invalid of [[], { note: [] }, { note: { key: message("common.close"), work: "work" } }, { note: { key: "raw", work: "work", label: "old UI" } }]) assert.throws(() => decodeCitationDrafts(invalid));
});

test("citation proposals keep exact author bytes and reject undeclared outer and patch fields", () => {
    const context: HostedLinksContext = { actor: "actor", deployment: "deployment", generation: 1, credential: "credential", authorization: 1, epoch: 1, resource: "note", sessionId: "session", source: "作者😀\r\n", serial: 1, writable: true, pending: false, compare: false }, byte = new TextEncoder().encode(context.source).length;
    const patch = { from: context.source.length, to: context.source.length, text: "@原始键", expected: "", byteFrom: byte, byteTo: byte }, value = { resource: "note", serial: 1, sourceHash: "hash", patchUnit: "utf16", source: context.source + patch.text, patches: [patch] };
    assert.equal(citationPatches(value, context, "hash")[0].insert, "@原始键"); assert.throws(() => citationPatches({ ...value, message: "old response" }, context, "hash")); assert.throws(() => citationPatches({ ...value, patches: [{ ...patch, reason: "old response" }] }, context, "hash"));
});

test("actual hosted citations retain author controls on locale changes and reject an A-to-B-to-A late editor result", async () => {
    const names = ["document", "sessionStorage"] as const, saved = names.map(name => Object.getOwnPropertyDescriptor(globalThis, name)), previousLocale = locale.current, document = new TestDocument(), retained = storage(), host = document.createElement("section"); document.body.append(host);
    Object.defineProperty(globalThis, "document", { configurable: true, value: document }); Object.defineProperty(globalThis, "sessionStorage", { configurable: true, value: retained });
    host.querySelectorAll = () => host.all().filter(node => ["input", "textarea", "select"].includes(node.tag));
    const initial: HostedLinksContext = { actor: "actor", deployment: "deployment", generation: 1, credential: "credential", authorization: 1, epoch: 1, resource: "note-a", sessionId: "session-a", source: "作者😀", serial: 1, writable: true, pending: false, compare: false }; let context = initial, release = (_value: unknown) => {}, arrived = () => {}, mismatch = false, retiredBeforeNotification = false, requestedHash = "";
    const started = new Promise<void>(resolve => { arrived = resolve; }), identity = () => initial;
    let controller: HostedCitations;
    const owner: HostedCitationHost = { context: () => context, identity, basis: async () => ({ sessionId: context.sessionId, source: context.source, serial: context.serial }), apply: async () => { throw new Error("no proposal may apply"); }, authorizationChanged: () => { retiredBeforeNotification = host.hidden && host.children.length === 0; } };
    const request = async <T,>(path: string, body?: unknown): Promise<T> => {
        if (path === "me") return { actor: "actor", deployment: "deployment", generation: mismatch ? 2 : 1 } as T;
        if (path === "citations") return { works: [{ resource: "work", reference: "entity:work", title: "作者 title 😀", status: "ready", use: true, export: false, download: false }], coverage: "complete" } as T;
        if (path === "citations/editor") { requestedHash = (body as { sourceHash: string }).sourceHash; arrived(); return new Promise<T>(resolve => { release = value => resolve(value as T); }); }
        throw new Error("unexpected endpoint");
    };
    try {
        locale.set("zh-CN"); controller = new HostedCitations(host as unknown as HTMLElement, request, owner); await controller.connect("deployment", "actor", 1);
        const input = host.find("input"), select = host.find("select"), label = input.getAttribute("aria-label"); input.value = "author_key"; input.oninput!(); locale.set("en-US"); assert.notEqual(input.getAttribute("aria-label"), label); assert.equal(host.find("input"), input); assert.equal(input.value, "author_key"); assert.ok(select.textContent.includes("作者 title 😀"));
        const internal = controller as unknown as { readEditor(): Promise<void>; nativeProfile?: unknown; nativeInputs: Map<string, unknown>; authority(value: HostedLinksContext): Promise<boolean> }, reading = internal.readEditor(); await started;
        context = { ...initial, resource: "note-b", sessionId: "session-b" }; controller.documentChanged(); context = initial; controller.documentChanged();
        const profile = { ownerReference: "entity:note-a", generation: "native-generation", coverage: "complete", choices: [], editor: { sourceHash: requestedHash, bindings: [], citations: [], targets: [], crossReferences: [], renameAvailable: false, renameReason: null }, bindings: [], incoming: [], links: [], bibliographyOwnership: "unavailable", displayStatus: "unavailable", displayExplanation: message("common.operation", { operationId: "retired private generation" }) };
        assert.doesNotThrow(() => decodeNativeCitationSnapshot(JSON.parse(JSON.stringify(profile)), "hosted")); release({ resource: initial.resource, serial: initial.serial, sourceHash: requestedHash, profile }); await reading;
        assert.equal(internal.nativeProfile, undefined); assert.doesNotMatch(host.textContent, /retired private generation/);
        const currentInput = host.find("input"); currentInput.value = "protected draft"; internal.nativeInputs.set("private-owner", { search: "protected search" }); mismatch = true; await internal.authority(initial);
        assert.equal(retiredBeforeNotification, true); assert.equal(currentInput.value, ""); assert.equal(internal.nativeInputs.size, 0); const cleared = host.textContent; locale.set("zh-CN"); assert.equal(host.textContent, cleared);
    } finally {
        locale.set(previousLocale); names.forEach((name, index) => { if (saved[index]) Object.defineProperty(globalThis, name, saved[index]!); else Reflect.deleteProperty(globalThis, name); });
    }
});
