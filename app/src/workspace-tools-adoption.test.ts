import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { DataPanel, type DocumentBasis, type PanelHost } from "./data-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { TestDocument, TestElement } from "./dom.test-support";

import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import type { WorkspaceTools } from "./workspace-tools";
import { message as uiMessage, setUiText, errorUiText } from "./ui-locale";

// Follow the existing native-scope-transition owner boundary: execute the real
// composition methods without importing the CSS/media/bootstrap dependency graph.
// DataPanel and its reminder read/retire/close owners below are the real classes.
const workspaceSource = ts.createSourceFile("workspace-tools.ts", readFileSync(new URL("./workspace-tools.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const workspaceClass = workspaceSource.statements.find(node => ts.isClassDeclaration(node) && node.name?.text === "WorkspaceTools") as ts.ClassDeclaration;
assert.ok(workspaceClass);
const methodNames = ["closeOwners", "assertCloseReady", "freezeForClose", "thawAfterClose", "button", "show", "showPanel", "captureInspectorOrigin", "closeInspector", "selectedDocumentChanged", "documentChanged"];
const methods = workspaceClass.members.filter(node => ts.isMethodDeclaration(node) && methodNames.includes(node.name.getText(workspaceSource)));
assert.equal(methods.length, methodNames.length);
const compiled = ts.transpileModule(`class TestedWorkspaceTools { ${methods.map(node => node.getText(workspaceSource)).join("\n")} }\nglobalThis.TestedWorkspaceTools = TestedWorkspaceTools;`, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
}).outputText;
const digest = (value: string) => createHash("sha256").update(value, "utf8").digest("hex");
const deferred = <T>() => { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; };
const turn = () => new Promise<void>(resolve => setImmediate(resolve));

class Element extends TestElement {
    inert = false; id = "";
    get firstChild(): TestElement | null { return this.childNodes[0] ?? null; }
    classList = { toggle: () => {}, add: () => {}, remove: () => {} };
    prepend(node: TestElement): void { this.insertBefore(node, this.childNodes[0] ?? null); }
    contains(node: unknown): boolean { return this.all().includes(node as TestElement); }
    matches(selector: string): boolean {
        if (selector === ":disabled") return this.disabled;
        if (selector.startsWith(".")) return selector.slice(1).split(".").every(token => this.className.split(/\s+/).includes(token));
        if (selector.startsWith("#")) return this.id === selector.slice(1);
        if (selector.startsWith("[data-")) return Object.hasOwn(this.dataset, selector.slice(6, -1).replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()));
        return selector === this.tag;
    }
    override querySelectorAll(selector = ""): Element[] { return this.all().slice(1).filter(node => selector.split(",").some(part => (node as Element).matches(part))) as Element[]; }
    querySelector(selector: string): Element | null { return this.querySelectorAll(selector)[0] ?? null; }
    closest(selector: string): Element | null { return this.matches(selector) ? this : this.parentNode instanceof Element ? this.parentNode.closest(selector) : null; }
    focus(): void { (this.ownerDocument as Document).activeElement = this; }
}
class Document extends TestDocument {
    activeElement: Element | null = null;
    override createElement(tag: string): Element { return new Element(tag, this); }
}
const basis = (name = "first", root = "library"): DocumentBasis => ({ inputId: `input-${name}`, root, path: `${root}/${name}.typ`, source: `${name} source`, sessionId: `session-${name}`, serial: 1, baseId: `base-${name}`, nodeId: `node-${name}`, workspaceId: `workspace-${root}` });
const schema = {
    schema: { fields: [{ id: "tasks/reminder", key: "reminder", shape: { kind: "scalar", semantic: "text" }, nullable: false }], definitions: {}, aliases: {}, types: [{ id: "tasks/task", fields: [{ id: "tasks/reminder", required: false }], requires: [], conflicts: [] }] },
    source: "schema source", revision: "schema-revision", roleRef: "schema-role", builtinDefault: false, canInstall: false, semanticDigest: digest("schema"),
};
function projection(document: DocumentBasis) {
    return { serial: document.serial, sourceHash: digest(document.source), schemaRevision: schema.revision, schemaHash: digest(schema.source), readOnly: false,
        projection: { qualification: "qualified", id: null, types: ["tasks/task"], resolvedTypes: ["tasks/task"], unknownFields: [], diagnostics: [], budgetExhausted: false, fields: [] } };
}
type SelectedTools = WorkspaceTools;
function fixture(t: TestContext, custom?: (route: string, body?: unknown) => Promise<unknown>) {
    const document = new Document(), previous = new Map<string, PropertyDescriptor | undefined>();
    for (const [key, value] of Object.entries({ document, localStorage: { getItem: () => null, setItem() {} }, HTMLInputElement: class {}, HTMLTextAreaElement: class {}, CSS: { escape: (text: string) => text } })) {
        previous.set(key, Object.getOwnPropertyDescriptor(globalThis, key)); Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
    }
    t.after(() => { for (const [key, value] of previous) { if (value) Object.defineProperty(globalThis, key, value); else Reflect.deleteProperty(globalThis, key); } });
    const state = { current: null as DocumentBasis | null }, requests: { route: string; body: unknown }[] = [], notices: unknown[] = [];
    const host = {
        getDocument: () => state.current, canEdit: () => true,
        request: async (route: string, body?: unknown) => {
            requests.push({ route, body });
            if (custom) return custom(route, body);
            if (route === "/api/data/schema") return schema;
            if (route === "/api/data/project") { const doc = state.current; assert.ok(doc); return projection(doc); }
            if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
            if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
            if (route === "/api/reminders/inspect") throw new Error("bounded inspect failure renders unavailable control");
            throw new Error(`Unexpected request ${route}`);
        }, notify: (...args: unknown[]) => notices.push(args), beforeAction: async () => {},
    } as unknown as PanelHost;
    const panel = new DataPanel(host), guard = new ProposalCloseGuard();
    const inspector = document.createElement("aside"), header = document.createElement("header"), panelBody = document.createElement("div");
    inspector.append(header, panelBody); document.body.append(inspector);
    const context = vm.createContext({ document, setTimeout, clearTimeout, uiMessage, setUiText, errorUiText });
    vm.runInContext(compiled, context);
    const TestedWorkspaceTools = context.TestedWorkspaceTools as new () => SelectedTools;
    const tools = Object.assign(new TestedWorkspaceTools(), {
        closeGuard: guard, host, propertyPanel: panel, inspector, header, panelBody, active: null,
        queryPanel: { refresh: async () => {}, assertCloseReady() {}, freezeForClose: () => true, thawAfterClose() {} },
        knowledgePanel: { refresh: async () => {}, attachSurface() {}, assertCloseReady() {}, freezeForClose: () => true, thawAfterClose() {} },
        outputPanel: { assertCloseReady() {}, freezeForClose: () => true, thawAfterClose() {} },
        libraryTrash: { assertCloseReady() {}, freezeForClose: () => true, thawAfterClose() {} },
        styles: new Map(), pendingImport: null,
    }) as SelectedTools;
    const select = async (next: DocumentBasis | null) => {
        state.current = next;
        await tools.selectedDocumentChanged();
        await turn();
    };
    return { document, state, requests, notices, panel, guard, tools, select, inspector };
}

test("delayed open adoption renews an already open empty property consumer while it owns focus", async t => {
    const f = fixture(t), opened = deferred<DocumentBasis>();
    const adopting = opened.promise.then(f.select);
    await f.tools.show("data");
    assert.equal(f.requests.length, 0);
    assert.match(f.panel.element.textContent ?? "", /打开笔记/);
    f.document.activeElement = f.panel.element as unknown as Element;
    opened.resolve(basis()); await adopting;
    assert.deepEqual(f.requests.slice(0, 2).map(item => item.route), ["/api/data/schema", "/api/data/project"]);
    assert.ok(f.requests.some(item => item.route === "/api/reminders/inspect"));
    assert.doesNotMatch(f.panel.element.textContent ?? "", /打开笔记后/);
    assert.ok(f.panel.element.textContent?.includes("提醒"));
    assert.equal(f.notices.length, 0);
});

test("selection without an open property pane starts no property/reminder reads", async t => {
    const f = fixture(t); await f.select(basis()); assert.equal(f.requests.length, 0);
});

test("selection leaves an unrelated query refresh timer owned by its existing path", async t => {
    const f = fixture(t); let calls = 0;
    Object.assign(f.tools, { active: "query" });
    Object.assign((f.tools as unknown as { queryPanel: object }).queryPanel, { documentChanged: () => { calls++; } });
    f.tools.documentChanged();
    await f.select(basis()); await new Promise<void>(resolve => setTimeout(resolve, 500));
    assert.equal(calls, 1); assert.equal(f.requests.length, 0);
});

test("selected context unavailable for comparison retires the consumer and starts no reads", async t => {
    const f = fixture(t); f.state.current = basis(); await f.tools.show("data"); await turn();
    const prior = f.requests.length;
    await f.select(null);
    assert.equal(f.requests.length, prior);
    assert.match(f.panel.element.textContent ?? "", /打开笔记/);
    const owner = f.panel as unknown as { basis: unknown; reminderDevice: { activeBasis: unknown } };
    assert.equal(owner.basis, null); assert.equal(owner.reminderDevice.activeBasis, null);
});

for (const stage of ["schema", "project"] as const) test(`late old-${stage} delivery cannot overwrite the newly selected document/root`, async t => {
    const old = basis("old"), next = basis("new", "second-library"), waiting = deferred<unknown>(), entered = deferred<void>();
    let held = false;
    const f = fixture(t, async (route, body) => {
        const request = body as { root?: string; sessionId?: string };
        if (!held && route === `/api/data/${stage}`) { held = true; entered.resolve(); return waiting.promise; }
        if (route === "/api/data/schema") return schema;
        if (route === "/api/data/project") return projection(request.sessionId === old.sessionId ? old : next);
        if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
        if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
        if (route === "/api/reminders/inspect") throw new Error("bounded inspect failure");
        throw new Error(route);
    });
    f.state.current = old;
    const oldRead = f.tools.show("data"); await entered.promise;
    assert.equal(f.tools.freezeForClose(), false);
    await f.select(next);
    const owner = f.panel as unknown as { basis: DocumentBasis; generation: number };
    const nextGeneration = owner.generation, newView = f.panel.element.textContent;
    waiting.resolve(stage === "schema" ? schema : projection(old)); await oldRead; await turn();
    assert.equal(owner.generation, nextGeneration); assert.equal(owner.basis.sessionId, next.sessionId); assert.equal(owner.basis.root, next.root);
    assert.equal(f.panel.element.textContent, newView);
    if (stage === "schema") assert.ok(!f.requests.some(item => item.route === "/api/data/project" && (item.body as { sessionId: string }).sessionId === old.sessionId));
    const inspected = f.requests.filter(item => item.route === "/api/reminders/inspect");
    assert.deepEqual(inspected.map(item => (item.body as { sessionId: string }).sessionId), [next.sessionId]);
    assert.equal(f.tools.freezeForClose(), true);
});

test("close remains busy across delayed selected-consumer reads; frozen selection starts no reads", async t => {
    const wait = deferred<unknown>(), entered = deferred<void>();
    const f = fixture(t, async route => {
        if (route === "/api/data/schema") { entered.resolve(); return wait.promise; }
        if (route === "/api/data/project") return projection(basis());
        if (route === "/api/data/references") return { nodes: [], complete: true, diagnostics: [] };
        if (route === "/api/navigation/tags") return { tags: [], complete: true, warnings: [] };
        if (route === "/api/reminders/inspect") throw new Error("bounded inspect failure");
        throw new Error(route);
    });
    await f.tools.show("data");
    const selecting = f.select(basis()); await entered.promise;
    assert.equal(f.tools.freezeForClose(), false);
    wait.resolve(schema); await selecting; await turn();
    assert.ok(f.requests.some(item => item.route === "/api/reminders/inspect"));
    assert.equal(f.tools.freezeForClose(), true);
    const count = f.requests.length; await f.select(basis("after-freeze")); assert.equal(f.requests.length, count);
});

test("new selection retains inactive raw drafts and reminder pending operations", async t => {
    const f = fixture(t), old = basis("old"), next = basis("new");
    f.state.current = old; await f.tools.show("data"); await turn();
    const owner = f.panel as unknown as { drafts: Map<string, unknown>; draftCache: Map<string, Map<string, unknown>>; reminderDevice: { pending: Map<string, unknown> } };
    const raw = { value: { kind: "text", value: "raw" }, changed: true, rawInputs: { invalid: "-" } };
    owner.drafts.set("retained-field", raw); owner.reminderDevice.pending.set("old-operation", { basis: old, route: "/api/reminders/library", request: { mutationId: "original-mutation" } });
    await f.select(next);
    const cached = owner.draftCache.get(JSON.stringify([old.workspaceId, old.nodeId]));
    assert.equal(cached?.get("retained-field"), raw);
    assert.equal(owner.reminderDevice.pending.size, 1);
    assert.deepEqual(owner.reminderDevice.pending.get("old-operation"), { basis: old, route: "/api/reminders/library", request: { mutationId: "original-mutation" } });
    assert.equal(f.tools.freezeForClose(), false);
});

test("each local selection/adoption path notifies properties after publishing current", () => {
    const main = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
    for (const name of ["acceptResolvedDocument", "addDocument", "activateDocument"]) {
        const owner = main.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === name) as ts.FunctionDeclaration;
        assert.ok(owner?.body, `Missing actual ${name} consumer`);
        const publications: ts.BinaryExpression[] = [], notifications: ts.CallExpression[] = [];
        const visit = (node: ts.Node): void => {
            if (ts.isBinaryExpression(node) && node.operatorToken.kind === ts.SyntaxKind.EqualsToken && node.left.getText(main) === "current") publications.push(node);
            if (ts.isCallExpression(node) && node.expression.getText(main) === "workspaceTools.selectedDocumentChanged") notifications.push(node);
            node.forEachChild(visit);
        };
        owner.body.forEachChild(visit);
        assert.equal(publications.length, 1, `${name}: current publication`);
        assert.equal(notifications.length, 1, `${name}: selection notification`);
        assert.ok(notifications[0].getStart(main) > publications[0].getStart(main), `${name}: notification before current publication`);
    }
});
