import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { createHash } from "node:crypto";
import * as projectionRead from "./data-projection-read";
import * as locale from "./ui-locale";
import * as recognition from "./recognition-state";

// Exercise the actual owners without mounting a browser or changing their public
// exports. UI rendering is replaced; owner state, asynchronous methods and guards
// are the candidate source, transpiled exactly as TypeScript executes them.
function prototype(file: string, name: string): object {
    const source = readFileSync(new URL(file, import.meta.url), "utf8") + `\nexport { ${name} as TestedOwner };`;
    const result = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } });
    const exports: Record<string, { prototype: object }> = {};
    runInNewContext(result.outputText, {
        exports, setTimeout, clearTimeout, URL,
        require: (id: string) => id === "./data-projection-read" ? projectionRead : id === "./proposal-close-guard" ? { ProposalCloseGuard }
            : id === "./ui-locale" ? { ...locale, setUiText: () => {}, disposeUiTree: () => {} }
                : id === "./recognition-state" ? recognition : {},
    });
    return exports.TestedOwner.prototype;
}
const recognitionPrototype = prototype("./recognition-panel.ts", "RecognitionPanel");
const historyPrototype = prototype("./history-panel.ts", "HistoryDialog");
const outputPrototype = prototype("./export-panel.ts", "OutputDialog");
const dialog = () => ({ open: false, showModal() { this.open = true; }, close() { this.open = false; }, remove() {} });
function outputOwner() {
    return Object.assign(Object.create(outputPrototype), {
        closeGuard: new ProposalCloseGuard(), dialog: dialog(), discarded: false,
        fieldReadGeneration: 0, fields: [],
        outputEdited: false, importEdited: false, deliveredImport: false, uploadedImport: null,
        output: { busy: false, uncertain: null, prepared: null, received: false, basis: { root: "owned" } },
        exchange: { busy: false, uncertain: null, input: null, receipt: null },
        unregisterClose() {}, onDiscard() {}, report() {},
    });
}

const outputBasis = { inputId: "frozen-input", sessionId: "frozen-session", source: "original 中文", serial: 1, baseId: "frozen-base", root: "library", path: "original.typ" };
function projectionWire(key = "status") {
    return { projection: { qualification: "qualified", id: null, types: ["task"], resolvedTypes: ["tasks/task"],
        fields: [{ fieldId: `tasks/${key}`, key, cell: { state: "value", value: { kind: "text", value: "open" } }, sourceRange: null, raw: null, writable: true, required: false, unbound: [] }],
        unknownFields: [], diagnostics: [], budgetExhausted: false },
        serial: outputBasis.serial, sourceHash: createHash("sha256").update(outputBasis.source).digest("hex"),
        schemaRevision: null, schemaHash: "a".repeat(64), readOnly: false };
}

test("output field reads use the frozen source after navigation and preserve later mapping input", async () => {
    const owner = outputOwner();
    let finish!: (value: unknown) => void, draws = 0;
    const pending = new Promise<unknown>(resolve => { finish = resolve; });
    owner.output.basis = Object.freeze({ ...outputBasis });
    const bindings = [{ slot: "original-slot", key: "original-field" }];
    owner.output.draft = { bindings };
    owner.drawBindings = () => { draws++; };
    owner.host = { getDocument() { throw new Error("A frozen output must not follow the active document"); }, async request(route: string, body: unknown) {
        assert.equal(route, "/api/data/project");
        assert.deepEqual(body, { sessionId: outputBasis.sessionId, source: outputBasis.source, serial: outputBasis.serial });
        return pending;
    } };
    const reading = owner.loadFields();
    bindings[0].key = "later-user-selection";
    finish(projectionWire()); await reading;
    assert.equal(owner.fields[0].key, "status"); assert.equal(draws, 1);
    assert.equal(owner.output.draft.bindings, bindings); assert.equal(bindings[0].key, "later-user-selection");
    assert.equal(owner.closeGuard.freeze(false), true);
});

for (const lateFailure of [false, true]) test(`an older output field ${lateFailure ? "failure" : "response"} cannot replace the latest fields or mapping`, async () => {
    const owner = outputOwner();
    let finish!: (value: unknown) => void, calls = 0, draws = 0;
    const pending = new Promise<unknown>(resolve => { finish = resolve; });
    owner.output.basis = Object.freeze({ ...outputBasis }); owner.output.draft = { bindings: [{ slot: "retained", key: "keep" }] };
    owner.drawBindings = () => { draws++; };
    owner.host = { async request() { return ++calls === 1 ? pending : projectionWire("newer"); } };
    const oldRead = owner.loadFields(); await owner.loadFields();
    finish(lateFailure ? { ...projectionWire(), sourceHash: "b".repeat(64) } : projectionWire("older"));
    await oldRead;
    assert.equal(owner.fields[0].key, "newer"); assert.equal(draws, 1);
    assert.equal(owner.output.draft.bindings[0].key, "keep"); assert.equal(owner.closeGuard.freeze(false), true);
});

test("hidden output/import proposals and unknown operations reject close and cancel cannot erase unknown", () => {
    const owner = outputOwner();
    owner.assertCloseReady();
    owner.outputEdited = true;
    assert.throws(() => owner.assertCloseReady());
    assert.equal(owner.freezeForClose(), false);
    owner.output.received = true;
    owner.assertCloseReady();
    owner.output.uncertain = "download";
    owner.discard();
    assert.equal(owner.discarded, false);
    assert.throws(() => owner.assertCloseReady());
    owner.output.uncertain = null;
    owner.discard();
    assert.equal(owner.discarded, true);
    assert.throws(() => owner.open());
});

test("known import receipt remains pending through delayed host delivery; freeze blocks later delivery", async () => {
    const owner = outputOwner();
    owner.importEdited = true;
    owner.exchange.receipt = { path: "copy" };
    let finish!: () => void;
    owner.host = { imported: () => new Promise<void>(resolve => { finish = resolve; }) };
    const delivered = owner.deliverReceipt({ path: "copy" });
    assert.throws(() => owner.assertCloseReady());
    assert.equal(owner.freezeForClose(), false);
    finish();
    await delivered;
    owner.assertCloseReady();
    assert.equal(owner.freezeForClose(), true);
    await assert.rejects(owner.deliverReceipt({ path: "copy" }));
    owner.thawAfterClose();
    owner.assertCloseReady();
});

test("restore preview, unknown original, and receipt delivery independently retain history owner", async () => {
    const owner = Object.assign(Object.create(historyPrototype), {
        closeGuard: new ProposalCloseGuard(), dialog: dialog(), acknowledging: false,
        controller: { operationId: null, review: {}, uncertain: false, busy: false, refresh: async () => {} },
        unacknowledged: null, status: { classList: { remove() {} } },
        drawControls() {}, drawOperation() {}, draw() {}, report() {},
    });
    assert.throws(() => owner.assertCloseReady());
    owner.controller.review = null;
    owner.controller.uncertain = true;
    assert.equal(owner.freezeForClose(), false);
    owner.controller.uncertain = false;
    owner.unacknowledged = { result: { receipt: { operationId: "original" } }, basis: {} };
    let finish!: () => void;
    owner.host = { restored: () => new Promise<void>(resolve => { finish = resolve; }), notify() {} };
    const delivery = owner.acknowledge();
    assert.throws(() => owner.assertCloseReady());
    finish();
    await delivery;
    owner.assertCloseReady();
    assert.equal(owner.freezeForClose(), true);
});

test("recognition retirement tombstones require exact authorized original results", () => {
    const owner = Object.assign(Object.create(recognitionPrototype), {
        closeGuard: new ProposalCloseGuard(), element: dialog(), identity: "owned", host: { identity: () => "owned" },
        retiredOperations: new Map([["owned", { operationId: "original", unreadable: false }]]),
        retiredProposals: new Set(), controller: { uncertain: false, busy: false, receipt: { operationId: "other", status: "committed" } },
    });
    owner.reconcileRetiredOperation();
    assert.throws(() => owner.assertCloseReady());
    owner.controller.receipt.operationId = "original";
    owner.controller.receipt.status = "prepared";
    owner.reconcileRetiredOperation();
    assert.equal(owner.freezeForClose(), false);
    owner.controller.receipt.status = "committed";
    owner.reconcileRetiredOperation();
    owner.assertCloseReady();
    owner.retiredProposals.add("owned");
    assert.throws(() => owner.assertCloseReady());
    owner.retiredProposals.clear();
    owner.controller.uncertain = true;
    assert.throws(() => owner.assertCloseReady());
});
