import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import ts from "typescript";
import { TestDocument, TestElement } from "./dom.test-support";
import { ProposalCloseGuard } from "./proposal-close-guard";
import * as locale from "./ui-locale";
import * as state from "./knowledge-state";
import * as original from "./original-operation";

class Element extends TestElement {
    inert = false;
    classList = { add() {}, remove() {} };
    listeners = new Map<string, (() => void)[]>();
    addEventListener(type?: string, callback?: () => void): void { if (type && callback) this.listeners.set(type, [...this.listeners.get(type) ?? [], callback]); }
    removeEventListener(): void {}
    querySelector(): null { return null; }
    contains(): boolean { return false; }
    focus(): void {}
    fire(type: string): void { for (const callback of this.listeners.get(type) ?? []) (callback as (event: object) => void)({ type, target: this, preventDefault() {}, stopImmediatePropagation() {} }); }
}
class Document extends TestDocument { override createElement(tag: string): Element { return new Element(tag, this); } }
const deferred = <T>() => { let resolve!: (value: T) => void; const promise = new Promise<T>(r => { resolve = r; }); return { promise, resolve }; };
function fixture() {
    const document = new Document();
    const source = readFileSync(new URL("./knowledge-panel.ts", import.meta.url), "utf8")
        .replace("assertCloseReady, freezeForClose, thawAfterClose,", "assertCloseReady, freezeForClose, thawAfterClose, testPerform: perform, testInspect: inspectPreparation, testRetry: () => perform(pendingPreparation.title, pendingPreparation.intent, pendingPreparation), testDiscard: discardAuthorProposals,");
    const exports: Record<string, any> = {};
    class View { clear() {} render() {} closePreviews() {} assertCloseReady() {} freezeForClose() { return true; } thawAfterClose() {} }
    runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText, {
        exports, document, HTMLElement: Element, crypto, structuredClone, queueMicrotask,
        window: { addEventListener() {}, removeEventListener() {} }, localStorage: { getItem() { return null; } },
        require: (id: string) => id === "./proposal-close-guard" ? { ProposalCloseGuard }
            : id === "./ui-locale" ? { ...locale, setUiText: (el: Element, value: locale.UiText) => { el.textContent = locale.uiTextValue(value); }, setUiAttr() {}, disposeUiTree() {} }
            : id === "./knowledge-state" ? state : id === "./original-operation" ? original
            : id === "./proofreading-panel" ? { ProofreadingPanel: View }
            : id === "./relationship-graph-view" ? { RelationshipGraphView: View }
            : id === "./relationship-diagnostics" ? { PersonalRelationRules: View } : {},
    });
    let context: any = null;
    const requests: any[] = [];
    let lookup = "unknown";
    const load = deferred<any>();
    const host = {
        context: () => context, notify() {}, load: () => load.promise,
        prepare: async (intent: any, basis: any, id: string) => { requests.push({ intent, basis, id }); throw new Error("transport lost"); },
        inspectOperation: async (id: string) => ({ operationId: id, status: lookup }),
    };
    const root = document.createElement("div");
    const panel = exports.mountKnowledgePanel(root, host);
    panel.selectTab("graph");
    return { panel, root, requests, load, useContext() { context = { sessionId: "session", serial: 1, baseId: "base", path: "note", source: "text", root: "owned", dirty: false }; }, resolveOriginal() { lookup = "resolved"; } };
}

test("lost knowledge preparation retains its exact original ID and request until real status resolves it", async () => {
    const f = fixture(); f.useContext();
    assert.equal(await f.panel.testPerform("preview", { kind: "backupLibrary", destination: "owned-copy" }), false);
    assert.throws(() => f.panel.assertCloseReady());
    assert.equal(f.panel.freezeForClose(), false);
    assert.throws(() => f.panel.testDiscard());
    await f.panel.testRetry();
    assert.equal(f.requests.length, 2);
    assert.deepEqual(f.requests[1], f.requests[0]);
    await f.panel.testInspect();
    assert.throws(() => f.panel.assertCloseReady());
    f.resolveOriginal(); await f.panel.testInspect();
    f.panel.assertCloseReady();
});

test("knowledge author input requires explicit discard and frozen owner rejects new proposals", async () => {
    const f = fixture();
    f.panel.selectTab("annotations"); f.root.fire("input");
    assert.throws(() => f.panel.assertCloseReady());
    f.panel.testDiscard(); f.panel.assertCloseReady();
    assert.equal(f.panel.freezeForClose(), true);
    await assert.rejects(f.panel.testPerform("late", { kind: "backupLibrary" }));
    assert.throws(() => f.panel.selectTab("files"));
    f.panel.thawAfterClose(); f.panel.assertCloseReady();
});

test("frozen knowledge owner invalidates late snapshot reads", async () => {
    const f = fixture(); f.useContext();
    const refresh = f.panel.refresh();
    assert.equal(f.panel.freezeForClose(), true);
    f.load.resolve({}); await refresh;
    f.panel.assertCloseReady(); f.panel.thawAfterClose();
});

test("local knowledge has no duplicate outline tab, background scan or external outline mount", () => {
    const f = fixture();
    assert.equal(f.panel.backgroundEnabled(), false);
    assert.equal(f.panel.attachOutline, undefined);
    assert.equal(f.root.all().some(node => node.getAttribute("role") === "tab" && node.textContent === locale.uiTextValue(locale.message("knowledge.headingStructure"))), false);
});
