import test, { type TestContext } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { TestDocument, TestElement } from "./dom.test-support";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { LibraryCloseActivity } from "./library-native-close";
import { message as uiMessage, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, disposeUiTree } from "./ui-locale";

// Use the existing owner-test boundary: transpile the actual composition methods,
// not CSS/bootstrap/media imports. Browser focus fixup remains a browser check.
const main = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const workspace = ts.createSourceFile("workspace-tools.ts", readFileSync(new URL("./workspace-tools.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const owner = workspace.statements.find(node => ts.isClassDeclaration(node) && node.name?.text === "WorkspaceTools") as ts.ClassDeclaration;
const methodNames = ["show", "toggle", "refreshView"];
const methods = owner.members.filter(node => ts.isMethodDeclaration(node) && methodNames.includes(node.name.getText(workspace)));
assert.equal(methods.length, methodNames.length);
const workspaceCode = ts.transpileModule(`class Owner { ${methods.map(node => node.getText(workspace)).join("\n")} } globalThis.Owner = Owner;`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
const functions = ["button", "el", "setDocumentGroup", "clearDocumentGroups", "renderContent", "changeMode"].map(name => {
    const node = main.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === name);
    assert.ok(node); return node.getText(main);
});
const render = main.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === "renderDocument") as ts.FunctionDeclaration;
assert.ok(render.body);
const viewEnd = render.body.statements.findIndex(node => ts.isExpressionStatement(node) && node.expression.getText(main).startsWith('setDocumentGroup("view", "document-mode"'));
assert.ok(viewEnd >= 0);
// Only retain the real view-control construction prefix; body/editor rendering
// after this boundary is unrelated to restoration of the rebuilt chrome.
const viewCode = ts.transpileModule(functions.join("\n") + `\nfunction renderDocument() { ${render.body.statements.slice(0, viewEnd + 1).map(node => node.getText(main)).join("\n")} }`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
const turn = () => new Promise<void>(resolve => setImmediate(resolve));
const deferred = () => { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; };

class Element extends TestElement {
    inert = false; id = ""; focusCount = 0; onFocus?: () => void;
    classList = { toggle() {} };
    get firstChild(): TestElement | null { return this.childNodes[0] ?? null; }
    contains(node: unknown): boolean { return this.all().includes(node as TestElement); }
    matches(selector: string): boolean {
        return selector.split(",").some(part => {
            if (part === ":disabled") return this.disabled;
            if (part === "[hidden]") return this.hidden;
            if (part === "[inert]") return this.inert;
            const attribute = /^\[data-([\w-]+)(?:="([^"]*)")?\]$/.exec(part);
            if (attribute) {
                const key = attribute[1].replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase());
                return Object.hasOwn(this.dataset, key) && (attribute[2] === undefined || this.dataset[key] === attribute[2]);
            }
            return part === this.tag;
        });
    }
    closest(selector: string): Element | null { return this.matches(selector) ? this : this.parentNode instanceof Element ? this.parentNode.closest(selector) : null; }
    override querySelectorAll(selector = ""): Element[] {
        const parts = selector.split(" "), last = parts.pop()!;
        return this.all().slice(1).filter(node => node instanceof Element && node.matches(last) && (!parts.length || node.parentNode instanceof Element && !!node.parentNode.closest(parts.join(" ")))) as Element[];
    }
    querySelector(selector: string): Element | null { return this.querySelectorAll(selector)[0] ?? null; }
    getClientRects(): object[] { return this.isConnected && !this.closest("[hidden],[inert]") ? [{}] : []; }
    focus(): void { if (this.getClientRects().length && !this.disabled) { (this.ownerDocument as Document).activeElement = this; this.focusCount++; this.onFocus?.(); } }
    override remove(): void {
        const document = this.ownerDocument as Document, owned = this.contains(document.activeElement);
        super.remove(); if (owned) document.activeElement = document.body;
    }
}
class Document extends TestDocument {
    activeElement: TestElement | null = null;
    override createElement(tag: string): Element { return new Element(tag, this); }
}
function dom(t: TestContext) {
    const document = new Document(), previous = Object.getOwnPropertyDescriptor(globalThis, "document");
    Object.defineProperty(globalThis, "document", { configurable: true, value: document });
    t.after(() => { if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document"); });
    const append = (tag = "button") => { const element = document.createElement(tag); document.body.append(element); return element; };
    return { document, append };
}
function dockCommandFixture(t: TestContext) {
    const { document, append } = dom(t), opener = append(), outside = append();
    const view = append("section"), input = document.createElement("input"); view.append(input);
    input.value = "retained proposal";
    const state = { document: { sessionId: "session-a", root: "root-a", source: "author draft", undo: ["prior"] }, before: async () => {} };
    const visible = new Set<string>(), changes: string[] = [];
    const context = vm.createContext({ document, uiMessage, setUiText, errorUiText });
    vm.runInContext(workspaceCode, context);
    const tools = Object.assign(new context.Owner(), {
        closeGuard: new ProposalCloseGuard(),
        presentation: { visible: (kind: string) => visible.has(kind), reveal: (kind: string) => { visible.add(kind); changes.push(`reveal:${kind}`); }, hide: (kind: string) => { visible.delete(kind); changes.push(`hide:${kind}`); } },
        host: { beforeAction: () => state.before(), getDocument: () => state.document, notify(error: unknown) { throw error; } },
        propertyPanel: { element: view, refresh: async () => {} },
        queryPanel: { element: document.createElement("div"), refresh: async () => {} }, knowledgePanel: { refresh: async () => {} },
    });
    return { document, opener, outside, state, tools, visible, changes, view, input };
}

test("deferred workspace reveal preserves focus moved away during author settlement", async t => {
    const f = dockCommandFixture(t), wait = deferred(); f.state.before = () => wait.promise;
    f.opener.focus(); const opening = f.tools.show("data"); f.outside.focus(); wait.resolve(); await opening;
    assert.equal(f.document.activeElement, f.outside); assert.deepEqual(f.changes, ["reveal:data"]);
});
test("workspace view switch delegates presentation while keeping the focused proposal and author owner", async t => {
    const f = dockCommandFixture(t), author = f.state.document, parent = f.input.parentNode;
    await f.tools.show("data"); f.input.focus(); await f.tools.show("outline");
    assert.equal(f.document.activeElement, f.input);
    assert.equal(f.input.parentNode, parent); assert.equal(f.input.value, "retained proposal");
    assert.equal(f.state.document, author); assert.deepEqual(author.undo, ["prior"]);
    assert.equal(author.source, "author draft"); assert.deepEqual([...f.visible], ["data", "outline"]);
});
test("toolbar hide delegates to the dock without disposing its view or mutating author input", async t => {
    const f = dockCommandFixture(t), author = f.state.document;
    await f.tools.show("data"); await f.tools.toggle("data");
    assert.deepEqual(f.changes, ["reveal:data", "hide:data"]);
    assert.equal(f.view.isConnected, true); assert.equal(f.input.value, "retained proposal");
    assert.equal(f.state.document, author); assert.equal(author.source, "author draft");
});
test("frozen workspace commands cannot hide a view or move focus", async t => {
    const f = dockCommandFixture(t); await f.tools.show("data"); f.input.focus();
    assert.equal(f.tools.closeGuard.freeze(false), true);
    await assert.rejects(f.tools.toggle("data")); await assert.rejects(f.tools.show("outline"));
    assert.deepEqual(f.changes, ["reveal:data"]); assert.equal(f.visible.has("data"), true);
    assert.equal(f.document.activeElement, f.input);
});

function viewFixture(t: TestContext) {
    const { document, append } = dom(t), ribbon = append("nav"), content = append("main"), outside = append();
    const doc = { navigationKey: "doc-a", mode: "source", layout: "continuous", session: { opened: { root: "root" } }, scroll: {}, notesMount: undefined,
        editor: { composing: false, clearReveal() {}, view: { scrollDOM: { scrollTop: 0 } } }, queries: { detach() {}, modeChanged() {} }, deferredProjection: { clear() {} },
        writing: { state: { composing: false }, flush: async () => {}, settleForSessionChange: async () => {}, deferUntilCompositionEnds: async (run: () => Promise<void>) => run() } };
    const context: any = { document, current: doc, documents: [doc], navigationGeneration: 1, bodyCloseFrozen: false, recognitionSession: "root", recognitionPanel: { retireDisplay() {} },
        renderedDocument: null, renderedMode: null, flowScroll: null, flowHasProjection: false, pagedPreview: null, workspaceTools: { attachSurface() {} }, projectionMappings: [], renderHost: null,
        documentRibbonGroups: [], content, ribbon: { element: ribbon, setGroup: (_tab: string, _id: string, _label: unknown, nodes: Element[]) => ribbon.append(...nodes), clearGroup: () => ribbon.replaceChildren() },
        documentComparisonMessage: () => null, renderWelcome() {}, renderTopbar() {}, renderTabs() {}, renderStatus() {}, renderEverydayBar() {},
        uiMessage, uiTextValue, setUiText, setUiAttr, uiTextNode, disposeUiTree, icon: () => document.createElement("span"), CSS: { escape: (value: string) => value },
        libraryCloseActivity: new LibraryCloseActivity(), localStorage: { setItem() {} }, flushLiteral: async () => {}, activeSourceOrigin: null, previewVisible: true,
        modes: ["source", "read"].map(id => ({ id, title: id, short: id, hint: id })), showError(error: unknown) { throw error; },
    };
    vm.createContext(context); vm.runInContext(viewCode, context); context.renderContent();
    const control = (key: string): Element => { const element = ribbon.querySelector(`[data-document-control="${key}"]`); assert.ok(element); return element; };
    return { document, doc, context, outside, control };
}
for (const key of ["mode:source", "layout", "preview"] as const) test(`View rebuild restores the equivalent ${key} control by document and semantic identity`, async t => {
    const f = viewFixture(t), prior = f.control(key); prior.focus();
    if (key === "layout") { prior.value = "paged"; prior.onchange?.(); }
    else prior.onclick?.();
    await turn(); const next = f.control(key);
    assert.notEqual(next, prior); assert.equal(prior.isConnected, false); assert.equal(f.document.activeElement, next);
});
test("deferred View update preserves focus moved away while writer settlement is pending", async t => {
    const f = viewFixture(t), wait = deferred(), prior = f.control("layout");
    f.doc.writing.flush = () => wait.promise; prior.focus(); prior.value = "paged"; prior.onchange?.();
    f.outside.focus(); wait.resolve(); await turn(); assert.equal(f.document.activeElement, f.outside);
});
for (const changed of ["document", "generation", "frozen", "composition", "render-focus"] as const) test(`View restoration cannot overrule ${changed}`, t => {
    const f = viewFixture(t); f.control("layout").focus();
    if (changed === "document") f.context.current = { ...f.doc, navigationKey: "doc-b" };
    else if (changed === "generation") f.context.renderEverydayBar = () => { f.context.navigationGeneration++; };
    else if (changed === "frozen") f.context.bodyCloseFrozen = true;
    else if (changed === "composition") f.doc.writing.state.composing = true;
    else f.context.renderEverydayBar = () => f.outside.focus();
    f.context.renderContent(); assert.notEqual(f.document.activeElement, f.control("layout"));
    if (changed === "render-focus") assert.equal(f.document.activeElement, f.outside);
});

const hosted = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
function hostedHandler(target: string): string {
    const statement = hosted.statements.find(node => ts.isExpressionStatement(node) && ts.isCallExpression(node.expression)
        && node.expression.expression.getText(hosted) === `${target}.addEventListener` && node.expression.arguments[0]?.getText(hosted) === '"keydown"') as ts.ExpressionStatement;
    assert.ok(statement && ts.isCallExpression(statement.expression));
    return ts.transpileModule(`globalThis.handler = ${statement.expression.arguments[1].getText(hosted)};`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
}
test("Hosted document tabs keep plain-key roving while modified arrows reach global history", t => {
    const { document, append } = dom(t), buttons = [append(), append(), append()], steps: number[] = [];
    buttons[1].disabled = true;
    const context: any = { document, Element, openDocumentButtons: new Map(buttons.map((button, index) => [String(index), button])), source: { composing: false }, identity: {}, busy: false,
        stepNavigation: (step: number) => { steps.push(step); }, run: (run: () => void) => run() };
    vm.createContext(context); vm.runInContext(hostedHandler("documentTabs"), context); const tabHandler = context.handler;
    vm.runInContext(hostedHandler("document"), context); const globalHandler = context.handler;
    const key = (key: string, options: object = {}) => { const event = { key, keyCode: 0, target: document.activeElement, defaultPrevented: false, isComposing: false, altKey: false, ctrlKey: false, metaKey: false, shiftKey: false, preventDefault() { this.defaultPrevented = true; }, ...options }; tabHandler(event); globalHandler(event); return event; };
    buttons[0].focus(); assert.equal(key("ArrowLeft").defaultPrevented, true); assert.equal(document.activeElement, buttons[2]);
    key("Home"); assert.equal(document.activeElement, buttons[0]); key("End"); assert.equal(document.activeElement, buttons[2]);
    key("ArrowRight"); assert.equal(document.activeElement, buttons[0]);
    for (const options of [{ ctrlKey: true }, { metaKey: true }, { shiftKey: true }, { isComposing: true }, { keyCode: 229 }, { defaultPrevented: true }]) {
        buttons[0].focus(); const event = key("ArrowRight", options); assert.equal(document.activeElement, buttons[0]); assert.equal(event.defaultPrevented, "defaultPrevented" in options);
    }
    key("ArrowLeft", { altKey: true }); key("ArrowRight", { altKey: true });
    assert.deepEqual(steps, [-1, 1]); assert.equal(document.activeElement, buttons[0]);
});
