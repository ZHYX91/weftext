import assert from "node:assert/strict";
import test from "node:test";
import { QueryPanel } from "./query-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import type { NativeValue, SchemaField } from "./native-value-form";
import type { QueryRow } from "./query-panel";

function owner() {
    const closeGuard = new ProposalCloseGuard(), inlineDrafts = new Map(), draftFlags = new Set<string>();
    const captureForms = new Map(), periodNavigators = new Map(), navigationControls = new Map();
    const panel = Object.assign(Object.create(QueryPanel.prototype), {
        closeGuard, inlineDrafts, draftFlags, captureForms, periodNavigators, navigationControls,
        savedPending: null, definitionName: "", thumbnailDisposers: [], renderResults() {},
        element: { querySelectorAll() { throw new Error("The close proof cannot inspect rendered controls"); } },
    }) as QueryPanel;
    return { panel, closeGuard, inlineDrafts, draftFlags, captureForms, periodNavigators, navigationControls };
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
    const { panel, captureForms, periodNavigators, navigationControls } = owner();
    let blocked = true, frozen = false, thawed = 0;
    const capture = { assertCloseReady() { if (blocked) throw new Error("retained capture input"); }, freezeForClose() { frozen = true; return true; }, thawAfterClose() { frozen = false; thawed++; } };
    captureForms.set("inactive-library", capture);
    assert.throws(() => panel.assertCloseReady(), /retained capture/);
    blocked = false;
    periodNavigators.set("inactive-period", { assertCloseReady() {}, freezeForClose() { return false; }, thawAfterClose() {} });
    assert.equal(panel.freezeForClose(), false); assert.equal(frozen, false); assert.equal(thawed, 1);
    periodNavigators.clear();
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

for (const stage of ["data/edit", "applyPatch"] as const) test(`Query real ${stage} delivery preserves a later invalid raw producer cut`, async () => {
    const { panel, inlineDrafts } = owner(), port = panel as unknown as InlineOwner;
    const draft: InlineDraft = { value: { kind: "int", value: "1" }, raw: {}, binding: "binding", revision: 1 };
    const basis = { root: "/library", path: "note.typ", sessionId: "session", source: "source", serial: 1, baseId: "base" };
    const edit = deferred(), editReached = deferred(), delivery = deferred(), deliveryReached = deferred();
    let delivered = false;
    const failures: unknown[] = [];
    inlineDrafts.set("row", draft);
    Object.assign(panel, { root: basis.root, resultPlan: null, result: null, confirmChange: async () => true, host: {
        getDocument: () => basis, openDocument: async () => {}, notify(value: unknown, error: boolean) { if (error) failures.push(value); },
        async request(route: string) {
            if (route === "/api/data/project") return { qualification: "qualified", id: null, types: [], resolvedTypes: [], unknownFields: [], diagnostics: [], budgetExhausted: false,
                fields: [{ fieldId: "field", key: "amount", cell: { state: "value", value: { kind: "int", value: "0" } }, sourceRange: null, raw: null, writable: true, required: false, unbound: [] }] };
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
