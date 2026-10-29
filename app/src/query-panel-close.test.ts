import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { QueryPanel } from "./query-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import type { DocumentBasis, NativeValue, SchemaField } from "./native-value-form";
import type { QueryRow } from "./query-panel";

function owner() {
    const closeGuard = new ProposalCloseGuard(), inlineDrafts = new Map(), draftFlags = new Set<string>();
    const captureForms = new Map(), navigationControls = new Map();
    const panel = Object.assign(Object.create(QueryPanel.prototype), {
        closeGuard, inlineDrafts, draftFlags, captureForms, navigationControls,
        savedPending: null, definitionName: "", thumbnailDisposers: [], renderResults() {},
        element: { querySelectorAll() { throw new Error("The close proof cannot inspect rendered controls"); } },
    }) as QueryPanel;
    return { panel, closeGuard, inlineDrafts, draftFlags, captureForms, navigationControls };
}

test("Query close covers invalid raw fields, advanced and parameter proposals and original save state", () => {
    const { panel, inlineDrafts, draftFlags } = owner();
    panel.assertCloseReady();
    inlineDrafts.set("retained-other-library-row", { raw: { exactDecimal: "-" } });
    assert.throws(() => panel.assertCloseReady()); assert.equal(panel.freezeForClose(), false);
    inlineDrafts.clear();
    for (const input of ["advanced", "parameters", "filter", "expression", "limit"]) {
        draftFlags.add(input); assert.throws(() => panel.assertCloseReady());
        assert.equal(panel.freezeForClose(), false); draftFlags.delete(input);
    }
    Object.assign(panel, { savedPending: { body: { operationId: "original" } } });
    assert.throws(() => panel.assertCloseReady());
    Object.assign(panel, { savedPending: null, definitionName: "unsaved-name" });
    assert.throws(() => panel.assertCloseReady());
    Object.assign(panel, { definitionName: "" });
    assert.equal(panel.freezeForClose(), true); panel.thawAfterClose();
});

test("Query checks every retained child owner and rolls earlier freezes back if a later owner refuses", () => {
    const { panel, captureForms, navigationControls } = owner();
    let blocked = true, frozen = false, thawed = 0;
    const capture = { assertCloseReady() { if (blocked) throw new Error("retained capture input"); }, freezeForClose() { frozen = true; return true; }, thawAfterClose() { frozen = false; thawed++; } };
    captureForms.set("inactive-library", capture);
    assert.throws(() => panel.assertCloseReady(), /retained capture/);
    blocked = false;
    navigationControls.set("inactive-navigation", { assertCloseReady() {}, freezeForClose() { return false; }, thawAfterClose() {} });
    assert.equal(panel.freezeForClose(), false); assert.equal(frozen, false); assert.equal(thawed, 1);
    navigationControls.clear();
    navigationControls.set("inactive-tags", { assertCloseReady() { throw new Error("original tag operation"); }, freezeForClose() { return false; }, thawAfterClose() {} });
    assert.throws(() => panel.assertCloseReady(), /original tag/);
});

function deferred() { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; }
interface InlineDraft { value: NativeValue; raw: Record<string, string>; binding: string; revision: number }
interface InlineOwner {
    retainInlineDraft(key: string, draft: InlineDraft, value?: NativeValue, raw?: Record<string, string>): void;
    applyInlineDraft(row: QueryRow, field: SchemaField, key: string, draft: InlineDraft): Promise<void>;
}
const inlineField: SchemaField = { id: "field", key: "amount", shape: { kind: "scalar", semantic: "int" }, nullable: false };
const sourceDigest = (source: string): string => createHash("sha256").update(source, "utf8").digest("hex");
function projectionResponse(basis: DocumentBasis) {
    return { serial: basis.serial, sourceHash: sourceDigest(basis.source), schemaRevision: null, schemaHash: sourceDigest("schema"), readOnly: false,
        projection: { qualification: "qualified", id: null, types: [], resolvedTypes: [], unknownFields: [], diagnostics: [], budgetExhausted: false,
            fields: [{ fieldId: "field", key: "amount", cell: { state: "value", value: { kind: "int", value: "0" } }, sourceRange: null, raw: null, writable: true, required: false, unbound: [] }] } };
}

for (const stage of ["transport", "delivery"] as const) {
    for (const edit of ["typed", "invalidRaw", "equalBytes", "replacement"] as const) {
        test(`Query inline ${stage} completion retains later ${edit} instead of acknowledging its close`, async () => {
            const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
            const transport = deferred(), delivery = deferred();
            const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
            inlineDrafts.set("row", draft);
            let submitted: NativeValue | undefined;
            Object.assign(panel, { async changeTaskField(_row: QueryRow, _key: string, value: NativeValue, _field: string, applied: () => void) {
                submitted = value; await transport.promise; await delivery.promise; applied();
            } });
            const applying = port.applyInlineDraft({} as QueryRow, inlineField, "row", draft);
            if (stage === "delivery") { transport.resolve(); await Promise.resolve(); }
            if (edit === "typed") port.retainInlineDraft("row", draft, { kind: "int", value: "2" });
            else if (edit === "invalidRaw") port.retainInlineDraft("row", draft, undefined, { integer: "-" });
            else if (edit === "equalBytes") port.retainInlineDraft("row", draft, { kind: "int", value: "1" });
            else { inlineDrafts.delete("row"); inlineDrafts.set("row", { ...draft, value: { kind: "int", value: "9" } }); }
            assert.deepEqual(submitted, { kind: "int", value: "1" });
            transport.resolve(); delivery.resolve(); await applying;
            assert.equal(inlineDrafts.size, 1); assert.throws(() => panel.assertCloseReady());
            assert.equal(panel.freezeForClose(), false);
            if (edit === "invalidRaw") assert.deepEqual(inlineDrafts.get("row").raw, { integer: "-" });
        });
    }
}

test("Query inline delivery consumes only the unchanged submitted draft", async () => {
    const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
    const delivery = deferred(), draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
    inlineDrafts.set("row", draft);
    Object.assign(panel, { async changeTaskField(_row: QueryRow, _key: string, _value: NativeValue, _field: string, applied: () => void) { await delivery.promise; applied(); } });
    const applying = port.applyInlineDraft({} as QueryRow, inlineField, "row", draft);
    assert.equal(panel.freezeForClose(), false); delivery.resolve(); await applying;
    assert.equal(inlineDrafts.size, 0); assert.doesNotThrow(() => panel.assertCloseReady());
});

for (const stage of ["data/edit", "applyPatch"] as const) test(`Query real ${stage} delivery accepts replacement characters and emoji while preserving a later invalid raw producer cut`, async () => {
    const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
    const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
    const basis: DocumentBasis = { inputId: "original-input", root: "/library", path: "note.typ", sessionId: "session", source: "source \ufffd \ud83d\ude00", serial: 1, baseId: "base" };
    const edit = deferred(), editReached = deferred(), delivery = deferred(), deliveryReached = deferred();
    let delivered = false;
    const failures: unknown[] = [];
    inlineDrafts.set("row", draft);
    Object.assign(panel, { root: basis.root, resultPlan: null, result: null, confirmChange: async () => true, host: {
        getDocument: () => basis, openDocument: async () => {}, notify(value: unknown, error: boolean) { if (error) failures.push(value); },
        async request(route: string) {
            if (route === "/api/data/project") return projectionResponse(basis);
            assert.equal(route, "/api/data/edit"); editReached.resolve(); await edit.promise; return { from: 0, to: 0, text: "applied" };
        },
        async applyPatch() { deliveryReached.resolve(); await delivery.promise; delivered = true; },
    } });
    const applying = port.applyInlineDraft({ key: "row", path: basis.path, lifecycle: "live", readOnly: false, cells: {} }, inlineField, "row", draft);
    await editReached.promise;
    if (stage === "applyPatch") { edit.resolve(); await deliveryReached.promise; }
    port.retainInlineDraft("row", draft, undefined, { integer: "-" });
    assert.equal(panel.freezeForClose(), false);
    edit.resolve(); delivery.resolve(); await applying;
    assert.deepEqual(failures, []); assert.equal(delivered, true);
    assert.deepEqual(inlineDrafts.get("row").raw, { integer: "-" });
    assert.equal(panel.freezeForClose(), false);
});

for (const stage of ["projection", "confirmation", "edit"] as const) for (const changed of ["document", "inputId", "sessionId", "source", "serial", "baseId", "path", "root", "workspaceId", "nodeId", "permission"] as const) {
    test(`Query real ${stage} ignores a changed ${changed} and preserves later invalid raw input`, async () => {
        const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
        const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
        const basis: DocumentBasis = { inputId: "original-input", root: "/library", path: "note.typ", sessionId: "session", source: "source", serial: 1, baseId: "base", workspaceId: "workspace", nodeId: "node" };
        let current = { ...basis }, edits = 0, deliveries = 0, confirmations = 0, editable = true;
        const pending = deferred(), reached = deferred();
        inlineDrafts.set("row", draft);
        Object.assign(panel, { root: basis.root, resultPlan: null, result: null,
            async confirmChange() { confirmations++; if (stage === "confirmation") { reached.resolve(); await pending.promise; } return true; },
            host: {
                getDocument: () => current, canEdit: () => editable, openDocument: async () => {}, notify() {},
                async request(route: string) {
                    if (route === "/api/data/project") {
                        if (stage === "projection") { reached.resolve(); await pending.promise; }
                        return projectionResponse(basis);
                    }
                    assert.equal(route, "/api/data/edit"); edits++;
                    if (stage === "edit") { reached.resolve(); await pending.promise; }
                    return { from: 0, to: 0, text: "applied" };
                },
                async applyPatch() { deliveries++; },
            } });
        const applying = port.applyInlineDraft({ key: "row", path: basis.path, lifecycle: "live", readOnly: false, cells: {} }, inlineField, "row", draft);
        await reached.promise;
        port.retainInlineDraft("row", draft, undefined, { integer: "-" });
        if (changed === "document") current = { ...basis, path: "other.typ", source: "other source", sessionId: "other-session", inputId: "other-input" };
        else if (changed === "permission") editable = false;
        else current = { ...current, [changed]: changed === "serial" ? current.serial + 2 : `${current[changed]}-changed` };
        pending.resolve(); await applying;
        assert.equal(confirmations, stage === "projection" ? 0 : 1);
        assert.equal(edits, stage === "edit" ? 1 : 0); assert.equal(deliveries, 0);
        assert.equal(inlineDrafts.get("row"), draft); assert.deepEqual(draft.raw, { integer: "-" });
        assert.equal(panel.freezeForClose(), false);
    });
}

test("Query source edits accept repeated ACKs without carrying ACK authority into host patch delivery", async () => {
    const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
    const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
    const basis: DocumentBasis = { inputId: "author", root: "/library", path: "note.typ", sessionId: "session", source: "source", serial: 1, baseId: "base", inputRevision: 1 };
    let delivered: DocumentBasis | undefined;
    inlineDrafts.set("row", draft);
    Object.assign(panel, { root: basis.root, resultPlan: null, result: null,
        async confirmChange() { basis.inputRevision!++; return true; },
        host: {
            getDocument: () => basis, openDocument: async () => {}, notify() {},
            async request(route: string) {
                basis.inputRevision!++;
                if (route === "/api/data/project") return projectionResponse(basis);
                assert.equal(route, "/api/data/edit"); return { from: 0, to: 0, text: "applied" };
            },
            async applyPatch(_patch: unknown, source: DocumentBasis) { delivered = source; },
        } });
    await port.applyInlineDraft({ key: "row", path: basis.path, lifecycle: "live", readOnly: false, cells: {} }, inlineField, "row", draft);
    assert.ok(delivered); assert.equal(Object.hasOwn(delivered, "inputRevision"), false);
    assert.equal(basis.inputRevision, 4); assert.equal(inlineDrafts.size, 0); assert.equal(panel.freezeForClose(), true);
});

for (const mismatch of ["serial", "sourceHash"] as const) test(`Query real projection rejects mismatched ${mismatch} before confirmation or editing`, async () => {
    const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
    const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: { integer: "-" }, binding: "binding", revision: 1 };
    const basis: DocumentBasis = { inputId: "original-input", root: "/library", path: "note.typ", sessionId: "session", source: "source", serial: 1, baseId: "base" };
    const response = projectionResponse(basis);
    if (mismatch === "serial") response.serial++;
    else response.sourceHash = sourceDigest("other source");
    let confirmations = 0, edits = 0, deliveries = 0;
    inlineDrafts.set("row", draft);
    Object.assign(panel, { root: basis.root, resultPlan: null, result: null, confirmChange: async () => { confirmations++; return true; }, host: {
        getDocument: () => basis, openDocument: async () => {}, notify() {},
        async request(route: string) {
            if (route === "/api/data/project") return response;
            assert.equal(route, "/api/data/edit"); edits++; return { from: 0, to: 0, text: "applied" };
        },
        async applyPatch() { deliveries++; },
    } });
    await port.applyInlineDraft({ key: "row", path: basis.path, lifecycle: "live", readOnly: false, cells: {} }, inlineField, "row", draft);
    assert.equal(confirmations, 0); assert.equal(edits, 0); assert.equal(deliveries, 0);
    assert.equal(inlineDrafts.get("row"), draft); assert.deepEqual(draft.raw, { integer: "-" });
    assert.equal(panel.freezeForClose(), false);
});

for (const [kind, source] of [["high", "source\ud800"], ["low", "source\udc00"]] as const) {
    test(`Query real projection rejects an unpaired ${kind} surrogate before any project request and retains the original input`, async () => {
        const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
        const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: { integer: "-" }, binding: "binding", revision: 1 };
        const basis: DocumentBasis = { inputId: "original-input", root: "/library", path: "note.typ", sessionId: "session", source, serial: 1, baseId: "base" };
        // A successful response for replacement text must never qualify the original
        // unpaired UTF16 input, even though an encoder would produce the same hash.
        const replacement = projectionResponse({ ...basis, source: "source\ufffd" });
        let projectReads = 0, confirmations = 0, edits = 0, deliveries = 0;
        inlineDrafts.set("row", draft);
        Object.assign(panel, { root: basis.root, resultPlan: null, result: null, confirmChange: async () => { confirmations++; return true; }, host: {
            getDocument: () => basis, openDocument: async () => {}, notify() {},
            async request(route: string) {
                if (route === "/api/data/project") { projectReads++; return replacement; }
                assert.equal(route, "/api/data/edit"); edits++; return { from: 0, to: 0, text: "applied" };
            },
            async applyPatch() { deliveries++; },
        } });
        await port.applyInlineDraft({ key: "row", path: basis.path, lifecycle: "live", readOnly: false, cells: {} }, inlineField, "row", draft);
        assert.equal(projectReads, 0); assert.equal(confirmations, 0); assert.equal(edits, 0); assert.equal(deliveries, 0);
        assert.equal(basis.source, source); assert.equal(basis.inputId, "original-input");
        assert.equal(inlineDrafts.get("row"), draft); assert.deepEqual(draft.raw, { integer: "-" });
        assert.deepEqual(draft.value, { kind: "int", value: "1" });
        assert.equal(panel.freezeForClose(), false);
    });
}
