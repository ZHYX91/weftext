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
import { KnowledgeReadQueue } from "./knowledge-read-queue";

class Element extends TestElement {
    inert = false;
    classList = { add() {}, remove() {} };
    listeners = new Map<string, (() => void)[]>();
    addEventListener(type?: string, callback?: () => void): void { if (type && callback) this.listeners.set(type, [...this.listeners.get(type) ?? [], callback]); }
    removeEventListener(): void {}
    querySelector(): null { return null; }
    contains(node: unknown): boolean { return this.all().includes(node as TestElement); }
    matches(selector: string): boolean { return selector.split(",").includes(this.tag); }
    focus(): void { (this.ownerDocument as Document).activeElement = this; }
    fire(type: string): void { for (const callback of this.listeners.get(type) ?? []) (callback as (event: object) => void)({ type, target: this, preventDefault() {}, stopImmediatePropagation() {} }); }
}
class Document extends TestDocument { activeElement: Element | null = null; override createElement(tag: string): Element { return new Element(tag, this); } }
const deferred = <T>() => { let resolve!: (value: T) => void, reject!: (error: unknown) => void; const promise = new Promise<T>((r, fail) => { resolve = r; reject = fail; }); return { promise, resolve, reject }; };
const turn = () => new Promise<void>(resolve => setImmediate(resolve));
const snapshot = (marker: string) => ({ graph: { marker }, headingStructure: null, links: null, diagnostics: null, renderedLinks: [], citations: null, mentions: null, annotations: [], annotationStatus: "valid", templates: [], trash: [], warnings: [], errors: [], annotationDiagnostics: [], propertyReferences: { coverage: "complete", items: [], diagnostics: [] } });
function fixture() {
    const document = new Document();
    const source = readFileSync(new URL("./knowledge-panel.ts", import.meta.url), "utf8")
        .replace("assertCloseReady, freezeForClose, thawAfterClose,", "assertCloseReady, freezeForClose, thawAfterClose, testPerform: perform, testInspect: inspectPreparation, testInspectPending: inspectPending, testStrict: () => refresh(true), testPending: () => pendingPlan, testRetry: () => perform(pendingPreparation.title, pendingPreparation.intent, pendingPreparation), testDiscard: discardAuthorProposals,");
    const exports: Record<string, any> = {};
    class View { clear() {} render() {} closePreviews() {} assertCloseReady() {} freezeForClose() { return true; } thawAfterClose() {} }
    const proofreads: Proofreading[] = [];
    class Proofreading extends View { disposed = false; constructor() { super(); proofreads.push(this); } dispose() { this.disposed = true; } }
    const graphs: any[] = [];
    class Graph extends View { override render(...args: any[]) { graphs.push(args[1]); } }
    runInNewContext(ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText, {
        exports, document, HTMLElement: Element, crypto, structuredClone, queueMicrotask,
        window: { addEventListener() {}, removeEventListener() {} }, localStorage: { getItem() { return null; } },
        require: (id: string) => id === "./proposal-close-guard" ? { ProposalCloseGuard }
            : id === "./ui-locale" ? { ...locale, setUiText: (el: Element, value: locale.UiText) => { el.textContent = locale.uiTextValue(value); }, setUiAttr() {}, disposeUiTree() {} }
            : id === "./knowledge-state" ? state : id === "./original-operation" ? original
            : id === "./knowledge-read-queue" ? { KnowledgeReadQueue }
            : id === "./proofreading-panel" ? { ProofreadingPanel: Proofreading }
            : id === "./relationship-graph-view" ? { RelationshipGraphView: Graph }
            : id === "./relationship-diagnostics" ? { PersonalRelationRules: View } : {},
    });
    let context: any = null;
    const requests: any[] = [];
    let lookup = "unknown";
    const load = deferred<any>();
    const loads: ({ context: any } & ReturnType<typeof deferred<any>>)[] = [], notices: unknown[] = [];
    const physical = { active: 0, maximum: 0 };
    const host = {
        context: () => context, notify: (...args: unknown[]) => notices.push(args), load: (basis: any) => {
            const response = loads.length === 0 ? load : deferred<any>();
            loads.push({ context: structuredClone(basis), ...response });
            physical.active++; physical.maximum = Math.max(physical.maximum, physical.active);
            return response.promise.finally(() => { physical.active--; });
        },
        prepare: async (intent: any, basis: any, id: string) => { requests.push({ intent, basis, id }); throw new Error("transport lost"); },
        inspectOperation: async (id: string) => ({ operationId: id, status: lookup }),
    };
    const root = document.createElement("div");
    const panel = exports.mountKnowledgePanel(root, host);
    panel.selectTab("graph");
    return { panel, root, document, host, requests, load, loads, graphs, notices, physical, proofreads,
        useContext() { context = { sessionId: "session", serial: 1, inputId: "input", inputRevision: 1, baseId: "base", path: "note", source: "text", root: "owned", workspaceId: "workspace", nodeId: "note", dirty: false }; },
        changeContext(change: any) { context = change === null ? null : { ...context, ...change }; }, mutateContext(change: any) { Object.assign(context, change); },
        resolveOriginal() { lookup = "resolved"; } };
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
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
});

test("local knowledge has no duplicate outline tab, background scan or external outline mount", () => {
    const f = fixture();
    assert.equal(f.panel.backgroundEnabled(), false);
    assert.equal(f.panel.attachOutline, undefined);
    assert.equal(f.root.all().some(node => node.getAttribute("role") === "tab" && node.textContent === locale.uiTextValue(locale.message("knowledge.headingStructure"))), false);
});

test("identical in-flight knowledge refreshes share one read and install once, while a completed refresh reads again", async () => {
    const f = fixture(); f.useContext();
    const first = f.panel.refresh(), second = f.panel.refresh(), third = f.panel.testStrict();
    assert.equal(f.loads.length, 1);
    let settled = 0; void first.then(() => settled++); void second.then(() => settled++);
    await turn(); assert.equal(settled, 0);
    f.load.resolve(snapshot("first")); await Promise.all([first, second, third]);
    assert.equal(f.graphs.filter(value => value?.marker === "first").length, 1);
    const again = f.panel.refresh(); assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("fresh")); await again;
    assert.equal(f.graphs.at(-1)?.marker, "fresh");
});

test("a shared knowledge failure is reported once and every strict waiter rejects the original error", async () => {
    const f = fixture(); f.useContext();
    const ordinary = f.panel.refresh(), strict = f.panel.testStrict(), another = f.panel.testStrict(), error = new Error("original read failure");
    const rejected = assert.rejects(strict, value => value === error), alsoRejected = assert.rejects(another, value => value === error);
    f.load.reject(error); await Promise.all([ordinary, rejected, alsoRejected]);
    assert.equal(f.loads.length, 1); assert.equal(f.notices.length, 1);
    const retry = f.panel.refresh(); assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("retry")); await retry;
    assert.equal(f.graphs.at(-1)?.marker, "retry");
});

test("a repeated Draft ACK shares the same pending knowledge read while the next completed refresh reads again", async () => {
    const f = fixture(); f.useContext();
    const first = f.panel.refresh(); f.changeContext({ inputRevision: 2 });
    const repeated = f.panel.refresh(), strict = f.panel.testStrict();
    assert.equal(f.loads.length, 1, "an ACK of the same input cannot start another library scan");
    assert.equal(f.loads[0].context.inputRevision, 1, "sharing does not rewrite the original captured request");
    f.load.resolve(snapshot("same input")); await Promise.all([first, repeated, strict]);
    assert.equal(f.graphs.filter(value => value?.marker === "same input").length, 1);
    const fresh = f.panel.refresh(); assert.equal(f.loads.length, 2);
    assert.equal(f.loads[1].context.inputRevision, 2);
    f.loads[1].resolve(snapshot("fresh")); await fresh;
    assert.equal(f.graphs.at(-1)?.marker, "fresh");
});

for (const [name, change] of Object.entries({
    source: { source: "new source" }, serial: { serial: 2 }, inputId: { inputId: "ABA input" },
    session: { sessionId: "new owner" }, base: { baseId: "new base" }, path: { path: "other note" }, root: { root: "other root" },
    workspace: { workspaceId: "other workspace" }, node: { nodeId: "other node" }, dirty: { dirty: true },
    selection: { selection: { from: 0, to: 1 } },
})) test(`knowledge reads do not coalesce a changed ${name} and cannot install the superseded result`, async () => {
    const f = fixture(); f.useContext();
    const first = f.panel.refresh(); f.changeContext(change); const second = f.panel.refresh();
    assert.equal(f.loads.length, 1, "a new input must wait for the physical read to end");
    f.load.resolve(snapshot("old")); await first; await turn();
    assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("current")); await second;
    assert.equal(f.graphs.at(-1)?.marker, "current");
    assert.equal(f.graphs.some(value => value?.marker === "old"), false);
});

for (const [name, selection] of Object.entries({ from: { from: 1 }, to: { to: 3 }, excerpt: { visibleExcerpt: "changed" }, role: { elementRole: "equation" }, key: { nodeKey: "other" } }))
test(`knowledge read identity includes selection ${name}`, async () => {
    const f = fixture(); f.useContext();
    const original = { from: 0, to: 2, visibleExcerpt: "text", elementRole: "paragraph", nodeKey: "node" };
    f.changeContext({ selection: original }); const first = f.panel.refresh();
    f.changeContext({ selection: { ...original, ...selection } }); const second = f.panel.refresh();
    assert.equal(f.loads.length, 1);
    f.load.resolve(snapshot("old")); await first; await turn();
    assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("current")); await second;
    assert.equal(f.graphs.some(value => value?.marker === "old"), false);
});

test("a host context mutated in place cannot alter the captured read or qualify its old response", async () => {
    const f = fixture(); f.useContext(); const first = f.panel.refresh();
    f.mutateContext({ source: "mutated" });
    assert.equal(f.loads[0].context.source, "text");
    f.load.resolve(snapshot("old")); await first;
    assert.equal(f.graphs.some(value => value?.marker === "old"), false);
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
});

test("selection and Draft ACK changes alone retain the source-qualified read and end its loading state", async () => {
    const f = fixture(); f.useContext(); const reading = f.panel.refresh();
    f.changeContext({ selection: { from: 1, to: 2, visibleExcerpt: "e", elementRole: "paragraph", nodeKey: "selection" }, inputRevision: 2 });
    f.panel.contextChanged();
    f.load.resolve(snapshot("same source")); await reading;
    assert.equal(f.loads.length, 1, "moving the caret does not launch a new scan");
    assert.equal(f.graphs.at(-1)?.marker, "same source");
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
    assert.equal(f.root.textContent.includes(locale.uiTextValue(locale.message("knowledge.readingAuthorizedKnowledge"))), false);
});

test("multiple changed knowledge demands execute only the latest and retire each superseded strict waiter", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ source: "B", serial: 2, inputId: "input-B" });
    const middle = f.panel.testStrict(), middleAgain = f.panel.testStrict();
    const rejected = assert.rejects(middle, /知识读取依据或库环境已变化/), rejectedAgain = assert.rejects(middleAgain, /知识读取依据或库环境已变化/);
    f.changeContext({ source: "C", serial: 3, inputId: "input-C" }); const latest = f.panel.refresh();
    await Promise.all([old, rejected, rejectedAgain]);
    assert.equal(f.loads.length, 1); assert.equal(f.physical.active, 1, "retiring callers cannot release the actual load");
    f.load.reject(new Error("retired physical failure")); await turn();
    assert.equal(f.loads.length, 2); assert.equal(f.loads[1].context.source, "C");
    const latestStrict = f.panel.testStrict(); f.loads[1].resolve(snapshot("latest")); await Promise.all([latest, latestStrict]);
    assert.equal(f.graphs.at(-1)?.marker, "latest"); assert.equal(f.physical.maximum, 1); assert.equal(f.physical.active, 0);
    assert.equal(f.notices.length, 0, "an old failure cannot report against the latest demand");
});

test("synchronous context retirement prevents a returned retained owner from reviving its old physical read", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.testStrict();
    const rejected = assert.rejects(old, /知识读取依据或库环境已变化/);
    f.changeContext({ sessionId: "B", path: "B", source: "B", inputId: "input-B" }); f.panel.contextChanged(); await rejected;
    f.useContext(); f.panel.contextChanged(); const returned = f.panel.refresh();
    assert.equal(f.loads.length, 1); f.load.resolve(snapshot("old A")); await turn();
    assert.equal(f.loads.length, 2); assert.equal(f.loads[1].context.sessionId, "session");
    assert.equal(f.graphs.some(value => value?.marker === "old A"), false);
    f.loads[1].resolve(snapshot("new A")); await returned;
    assert.equal(f.graphs.at(-1)?.marker, "new A"); assert.equal(f.physical.maximum, 1);
});

test("a queued knowledge demand rechecks its source before sending and never retries an unrequested successor", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ source: "queued", serial: 2, inputId: "queued" }); const queued = f.panel.testStrict();
    const rejected = assert.rejects(queued, /知识读取依据或库环境已变化/);
    f.changeContext({ source: "unrequested", serial: 3, inputId: "unrequested" });
    f.load.resolve(snapshot("old")); await Promise.all([old, rejected]); await turn();
    assert.equal(f.loads.length, 1); assert.equal(f.physical.active, 0);
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
});

test("a failed read whose unobserved source has changed retires its own loading state", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ source: "new input", inputId: "new input" }); f.load.reject(new Error("old input failed")); await old;
    assert.equal(f.notices.length, 0);
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
});

test("mutation in a retired physical library cannot retire the queued library owner", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ root: "other library", sessionId: "other session", path: "other note", workspaceId: "other workspace", inputId: "other input" });
    const next = f.panel.testStrict(); f.panel.environmentChanged("owned");
    f.load.resolve(snapshot("old library")); await old; await turn();
    assert.equal(f.loads.length, 2); f.loads[1].resolve(snapshot("current library")); await next;
    assert.equal(f.graphs.at(-1)?.marker, "current library"); assert.equal(f.physical.maximum, 1);
});

test("invalidating a completed Knowledge snapshot does not automatically read again", async () => {
    const f = fixture(); f.useContext(); const first = f.panel.refresh(); f.load.resolve(snapshot("complete")); await first;
    f.panel.environmentChanged("owned"); await turn(); assert.equal(f.loads.length, 1);
    const explicit = f.panel.refresh(); assert.equal(f.loads.length, 2); f.loads[1].resolve(snapshot("explicit")); await explicit;
    assert.equal(f.graphs.at(-1)?.marker, "explicit");
});

for (const retirement of ["stop", "freeze", "dispose", "empty"] as const) test(`${retirement} clears queued Knowledge without letting the old physical tail restart it`, async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ source: "queued", serial: 2, inputId: "queued" }); const queued = f.panel.refresh();
    if (retirement === "freeze") { assert.equal(f.panel.freezeForClose(), true); f.panel.thawAfterClose(); }
    else if (retirement === "dispose") f.panel.dispose();
    else if (retirement === "empty") { f.changeContext(null); await f.panel.refresh(); }
    else f.root.find("button", locale.uiTextValue(locale.message("knowledge.stopWaiting"))).onclick?.();
    await Promise.all([old, queued]); assert.equal(f.physical.active, 1);
    assert.equal(f.root.all().some(node => node.tag === "button" && node.textContent === locale.uiTextValue(locale.message("knowledge.stopWaiting"))), false);
    f.load.resolve(snapshot("retired")); await turn();
    assert.equal(f.loads.length, 1); assert.equal(f.physical.active, 0);
    assert.equal(f.graphs.some(value => value?.marker === "retired"), false);
});

test("settling an older read does not remove the newer read's sharing identity", async () => {
    const f = fixture(); f.useContext(); const old = f.panel.refresh();
    f.changeContext({ serial: 2 }); const current = f.panel.refresh();
    assert.equal(f.loads.length, 1);
    f.load.resolve(snapshot("old")); await old; await turn();
    const duplicate = f.panel.refresh(); assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("current")); await Promise.all([current, duplicate]);
    assert.equal(f.graphs.filter(value => value?.marker === "current").length, 1);
});

test("knowledge environment refresh retains focused author controls until their deferred render", async () => {
    const f = fixture(); f.useContext(); const first = f.panel.refresh(); f.load.resolve(snapshot("initial")); await first;
    const input = f.document.createElement("input"); input.value = "unapplied form"; f.root.append(input); input.focus();
    f.panel.environmentChanged("owned");
    assert.equal(input.parentNode, f.root);
    const next = f.panel.refresh(), same = f.panel.refresh(); assert.equal(f.loads.length, 2);
    assert.equal(input.parentNode, f.root);
    f.loads[1].resolve(snapshot("fresh")); await Promise.all([next, same]);
    assert.equal(input.parentNode, f.root); assert.equal(input.value, "unapplied form");
    f.document.activeElement = null; f.root.fire("focusout"); await turn();
    assert.equal(input.parentNode, undefined); assert.equal(f.graphs.at(-1)?.marker, "fresh");
});

test("library mutation retires equal-source knowledge reads but an unrelated library leaves them shared", async () => {
    const f = fixture(); f.useContext();
    const first = f.panel.refresh(); f.panel.environmentChanged("unrelated"); const same = f.panel.refresh();
    assert.equal(f.loads.length, 1);
    f.panel.environmentChanged("OWNED/"); const fresh = f.panel.refresh();
    assert.equal(f.loads.length, 1);
    f.load.resolve(snapshot("before mutation")); await Promise.all([first, same]); await turn();
    assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("after mutation")); await fresh;
    assert.equal(f.graphs.at(-1)?.marker, "after mutation");
    assert.equal(f.graphs.some(value => value?.marker === "before mutation"), false);
});

for (const retirement of ["freeze", "stop", "empty"] as const) test(`a ${retirement} knowledge owner cannot rejoin its retired same-basis read`, async () => {
    const f = fixture(); f.useContext(); const first = f.panel.refresh();
    if (retirement === "freeze") { assert.equal(f.panel.freezeForClose(), true); f.panel.thawAfterClose(); }
    else if (retirement === "stop") { f.root.find("button", locale.uiTextValue(locale.message("knowledge.stopWaiting"))).onclick?.(); }
    else { f.changeContext(null); await f.panel.refresh(); f.useContext(); }
    const fresh = f.panel.refresh(); assert.equal(f.loads.length, 1);
    f.load.resolve(snapshot("old")); await first; await turn();
    assert.equal(f.loads.length, 2);
    f.loads[1].resolve(snapshot("fresh")); await fresh;
    assert.equal(f.graphs.some(value => value?.marker === "old"), false);
    assert.equal(f.graphs.at(-1)?.marker, "fresh");
});

test("disposed knowledge owners cannot install or start another read", async () => {
    const f = fixture(); f.useContext(); const first = f.panel.refresh();
    f.panel.dispose(); f.load.resolve(snapshot("old")); await first; await f.panel.refresh();
    assert.equal(f.proofreads[0].disposed, true);
    assert.equal(f.loads.length, 1); assert.equal(f.root.childNodes.length, 0);
    assert.equal(f.graphs.some(value => value?.marker === "old"), false);
});

test("retired knowledge reads remain quiet for background callers and reject strict readback", async () => {
    const f = fixture(); f.useContext();
    const ordinary = f.panel.refresh(), strict = f.panel.testStrict();
    const rejected = assert.rejects(strict, /知识读取依据或库环境已变化/);
    f.panel.environmentChanged("owned"); await Promise.all([ordinary, rejected]);
    f.load.resolve(snapshot("retired")); await turn();
    assert.equal(f.graphs.some(value => value?.marker === "retired"), false);
    assert.equal(f.notices.length, 0);
});

test("a committed knowledge action starts fresh strict readback and retains its original when that read fails", async () => {
    const f = fixture(); f.useContext();
    Object.assign(f.host, { prepare: async (_intent: unknown, _basis: unknown, id: string) => ({ operationId: id, planId: "plan" }), commit: async (plan: any) => ({ status: "committed", operationId: plan.operationId }) });
    const old = f.panel.refresh(), applying = f.panel.testPerform("preview", { kind: "backupLibrary" });
    await turn(); f.document.body.find("button", locale.uiTextValue(locale.message("knowledge.confirmApply"))).onclick?.(); await turn();
    assert.equal(f.loads.length, 1, "post-commit readback must wait for the pre-commit physical scan to end");
    assert.equal(f.panel.freezeForClose(), false, "the action remains an active close participant through readback");
    const duplicate = f.panel.refresh(); assert.equal(f.loads.length, 1);
    f.load.resolve(snapshot("before commit")); await old; await turn();
    assert.equal(f.loads.length, 2, "post-commit readback needs its own actual scan");
    f.loads[1].reject(new Error("readback failed")); assert.equal(await applying, false); await duplicate;
    assert.ok(f.panel.testPending()); assert.throws(() => f.panel.assertCloseReady());
    assert.equal(f.graphs.some(value => value?.marker === "before commit"), false);
});

test("a retired post-commit readback retains the original and a later inspection requires another fresh read", async () => {
    const f = fixture(); f.useContext();
    Object.assign(f.host, { prepare: async (_intent: unknown, _basis: unknown, id: string) => ({ operationId: id, planId: "plan" }), commit: async (plan: any) => ({ status: "committed", operationId: plan.operationId }), inspectOperation: async (id: string) => ({ status: "committed", operationId: id }) });
    const applying = f.panel.testPerform("preview", { kind: "backupLibrary" });
    await turn(); f.document.body.find("button", locale.uiTextValue(locale.message("knowledge.confirmApply"))).onclick?.(); await turn();
    f.panel.environmentChanged("owned"); assert.equal(await applying, false);
    const original = f.panel.testPending(); assert.ok(original);
    const background = f.panel.refresh(), inspected = f.panel.testInspectPending(); await turn();
    assert.equal(f.loads.length, 1, "retirement does not release the original physical slot");
    assert.equal(f.panel.freezeForClose(), false);
    f.load.resolve(snapshot("retired")); await turn();
    assert.equal(f.loads.length, 2, "only inspection readback runs; the superseded background intent never starts");
    f.loads[1].resolve(snapshot("confirmed")); await inspected;
    assert.equal(f.panel.testPending(), null); f.panel.assertCloseReady();
    await background;
    assert.equal(f.graphs.at(-1)?.marker, "confirmed");
});

test("known committed delivery errors retire pre-commit knowledge without clearing the original plan", async () => {
    const f = fixture(); f.useContext();
    Object.assign(f.host, { prepare: async (_intent: unknown, _basis: unknown, id: string) => ({ operationId: id, planId: "plan" }), commit: async (plan: any) => ({ status: "committed", operationId: plan.operationId, documentError: { code: "committedResultUnavailable", explanation: locale.message("common.unavailable"), diagnostics: [], causes: [] } }) });
    const old = f.panel.refresh(), applying = f.panel.testPerform("preview", { kind: "backupLibrary" });
    await turn(); f.document.body.find("button", locale.uiTextValue(locale.message("knowledge.confirmApply"))).onclick?.();
    assert.equal(await applying, false); assert.ok(f.panel.testPending());
    assert.equal(f.loads.length, 1, "an incomplete delivery does not invent a successful readback");
    f.load.resolve(snapshot("pre-commit")); await old;
    assert.equal(f.graphs.some(value => value?.marker === "pre-commit"), false);
});
