import test from "node:test";
import assert from "node:assert/strict";
import { DataPanel, QuickCapture, type DocumentBasis, type PanelHost } from "./data-panel";
import { emptyReferences, loadFormReferences, loadTagFormReferences } from "./native-value-form";
import { ReminderDeviceControl } from "./reminder-device-control";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { message, uiTextValue } from "./ui-locale";

/** Bounded DOM surface for the real owner/render/action code; no browser or UI. */
function testDom(context: { after(callback: () => void): void }) {
    class Element extends EventTarget {
        childNodes: Element[] = [];
        parent: Element | null = null;
        isConnected = true;
        inert = false;
        disabled = false;
        checked = false;
        id = "";
        className = "";
        dataset: Record<string, string> = {};
        attributes = new Map<string, string>();
        onclick?: () => void;
        onchange?: () => void;
        private text = "";
        constructor(readonly tagName: string, readonly nodeType = 1) { super(); }
        get ownerDocument() { return dom; }
        get textContent(): string { return this.text + this.childNodes.map(child => child.textContent).join(""); }
        set textContent(value: string) { this.replaceChildren(); this.text = value; }
        private connect(value: boolean): void { this.isConnected = value; this.childNodes.forEach(child => child.connect(value)); }
        appendChild(child: Element): Element { child.remove(); child.parent = this; child.connect(this.isConnected); this.childNodes.push(child); return child; }
        get firstChild(): Element | null { return this.childNodes[0] ?? null; }
        insertBefore(child: Element, before: Element | null): Element { child.remove(); const index = before ? this.childNodes.indexOf(before) : -1; child.parent = this; child.connect(this.isConnected); if (index < 0) this.childNodes.push(child); else this.childNodes.splice(index, 0, child); return child; }
        append(...children: Element[]): void { children.forEach(child => this.appendChild(child)); }
        prepend(child: Element): void { this.appendChild(child); this.childNodes.unshift(this.childNodes.pop()!); }
        replaceChildren(...children: Element[]): void { for (const child of [...this.childNodes]) child.remove(); this.text = ""; this.append(...children); }
        remove(): void { if (this.parent) this.parent.childNodes = this.parent.childNodes.filter(child => child !== this); this.parent = null; this.connect(false); }
        setAttribute(key: string, value: string): void { this.attributes.set(key, value); }
        removeAttribute(key: string): void { this.attributes.delete(key); }
        contains(child: Element): boolean { return child === this || this.childNodes.some(item => item.contains(child)); }
        matches(selector: string): boolean { return selector === ":disabled" && this.disabled; }
        querySelectorAll(selector: string): Element[] { return this.childNodes.flatMap(child => [...(child.tagName === selector ? [child] : []), ...child.querySelectorAll(selector)]); }
    }
    const dom = { createElement: (tag: string) => new Element(tag), createTextNode: (text: string) => { const node = new Element("#text", 3); node.textContent = text; return node; } };
    const prior = Object.getOwnPropertyDescriptor(globalThis, "document");
    Object.defineProperty(globalThis, "document", { value: dom, configurable: true });
    context.after(() => { if (prior) Object.defineProperty(globalThis, "document", prior); else Reflect.deleteProperty(globalThis, "document"); });
    return { create: dom.createElement, button: (root: Element, label: string) => {
        const button = root.querySelectorAll("button").find(button => button.textContent === label);
        assert.ok(button, `Visible action missing: ${label}`); return button;
    } };
}

const basis: DocumentBasis = { root: "library", path: "note.typ", source: "text", sessionId: "session", serial: 1, baseId: "base" };
const field = () => ({ value: { kind: "text" as const, value: "draft" }, changed: true, revision: 1, rawInputs: {} as Record<string, string> });
function reminderOwner() {
    const guard = new ProposalCloseGuard(), pending = new Map(), inFlight = new Set();
    const owner = Object.assign(Object.create(ReminderDeviceControl.prototype), { closeGuard: guard, pending, inFlight }) as ReminderDeviceControl;
    return { owner, guard, pending, inFlight };
}
function dataOwner() {
    const guard = new ProposalCloseGuard(), reminder = reminderOwner();
    const drafts = new Map<string, ReturnType<typeof field>>(), draftCache = new Map<string, Map<string, ReturnType<typeof field>>>();
    const typeDrafts = new Map<string, string[]>(), schemaDrafts = new Map();
    const owner = Object.assign(Object.create(DataPanel.prototype), {
        closeGuard: guard, reminderDevice: reminder.owner, drafts, draftCache, typeDrafts, typeInstalling: new Set(), schemaDrafts,
        element: { querySelectorAll() { throw new Error("Close proof must not scan DOM"); } },
    }) as DataPanel;
    return { owner, guard, reminder, drafts, draftCache, typeDrafts, schemaDrafts };
}
function captureOwner() {
    const guard = new ProposalCloseGuard();
    const owner = Object.assign(Object.create(QuickCapture.prototype), { closeGuard: guard, dirty: false, pending: null, inputRevision: 0 }) as QuickCapture;
    return { owner, guard };
}

test("DataPanel close checks current and inactive drafts, type and schema proposals without a DOM scan", () => {
    const { owner, drafts, draftCache, typeDrafts, schemaDrafts } = dataOwner();
    owner.assertCloseReady();
    drafts.set("field", field()); assert.throws(() => owner.assertCloseReady()); assert.equal(owner.freezeForClose(), false);
    drafts.clear();
    draftCache.set("inactive-note", new Map([["field", { ...field(), changed: false, rawInputs: { exact: "-" } }]]));
    assert.throws(() => owner.assertCloseReady()); draftCache.clear();
    typeDrafts.set("inactive-note", ["task"]); assert.equal(owner.freezeForClose(), false); typeDrafts.clear();
    schemaDrafts.set("another-library", { source: "{ incomplete" }); assert.equal(owner.freezeForClose(), false); schemaDrafts.clear();
    assert.equal(owner.freezeForClose(), true); owner.thawAfterClose(); owner.assertCloseReady();
});

test("DataPanel freezes its ReminderDevice owner only after its actual work and pending mutations settle", () => {
    const { owner, guard, reminder } = dataOwner();
    const finish = reminder.guard.begin();
    assert.equal(owner.freezeForClose(), false); assert.equal(guard.frozen, false); finish();
    reminder.pending.set("inactive-owner", { route: "/api/reminders/library", request: {} });
    assert.throws(() => owner.assertCloseReady()); assert.equal(owner.freezeForClose(), false);
    reminder.pending.clear(); assert.equal(owner.freezeForClose(), true);
    assert.equal(reminder.guard.frozen, true); owner.thawAfterClose(); assert.equal(reminder.guard.frozen, false);
});

test("an invalid raw field input becomes an owner draft before any close probe", () => {
    const { owner, drafts } = dataOwner();
    const draft = { ...field(), changed: false }; drafts.set("number", draft);
    const wrapper = { dataset: { fieldKey: "number" }, querySelector: () => ({}) };
    Object.assign(owner, {
        renderedIdentity: "note", draftIdentity: "note",
        element: { querySelectorAll: () => [wrapper] },
        formInputs: () => [["exact-number", { validity: { valid: false }, value: "-" }]],
    });
    (owner as unknown as { rememberInputs(): void }).rememberInputs();
    assert.equal(draft.changed, true); assert.deepEqual(draft.rawInputs, { "exact-number": "-" });
    Object.assign(owner, { element: { querySelectorAll() { throw new Error("No close-time DOM read"); } } });
    assert.equal(owner.freezeForClose(), false);
});

test("DataPanel remains busy across patch delivery and preserves later edits in its real draft owner", async () => {
    for (const laterInput of [false, true]) {
        const { owner, drafts } = dataOwner(); const draft = field(); drafts.set("field", draft);
        let deliver!: () => void, reached!: () => void;
        const delivery = new Promise<void>(resolve => { deliver = resolve; });
        const entered = new Promise<void>(resolve => { reached = resolve; });
        const host = {
            getDocument: () => basis,
            request: async () => ({ from: 0, to: 0, text: "patch" }),
            applyPatch: async () => { reached(); await delivery; },
            notify: () => {},
        } as unknown as PanelHost;
        Object.assign(owner, { host, element: { setAttribute() {}, removeAttribute() {} }, refresh: async () => {} });
        const applying = (owner as unknown as { applyField(key: string, value: unknown, basis: DocumentBasis): Promise<void> }).applyField("field", draft.value, basis);
        await entered; assert.equal(owner.freezeForClose(), false);
        if (laterInput) { draft.revision++; draft.rawInputs = { exact: "later invalid raw" }; }
        deliver(); await applying;
        assert.equal(drafts.has("field"), laterInput);
        assert.equal(owner.freezeForClose(), !laterInput);
    }
});

test("DataPanel applying an old note never deletes a same-key draft in the newly selected note", async () => {
    const { owner, drafts, draftCache } = dataOwner(); const original = field(); drafts.set("field", original);
    let deliver!: () => void, reached!: () => void;
    const delivery = new Promise<void>(resolve => { deliver = resolve; }), entered = new Promise<void>(resolve => { reached = resolve; });
    const host = { getDocument: () => basis, request: async () => ({ from: 0, to: 0, text: "patch" }), applyPatch: async () => { reached(); await delivery; }, notify() {} } as unknown as PanelHost;
    Object.assign(owner, { host, element: { setAttribute() {}, removeAttribute() {} }, refresh: async () => {} });
    const applying = (owner as unknown as { applyField(key: string, value: unknown, basis: DocumentBasis): Promise<void> }).applyField("field", original.value, basis);
    await entered;
    const next = new Map([["field", field()]]); draftCache.set("old-note", drafts); Object.assign(owner, { drafts: next });
    deliver(); await applying;
    assert.equal(drafts.has("field"), false); assert.equal(next.has("field"), true); assert.equal(owner.freezeForClose(), false);
});

test("QuickCapture rejects raw-only input, unresolved original creation and busy delivery; freeze blocks new input", () => {
    const { owner, guard } = captureOwner();
    (owner as unknown as { markInput(): void }).markInput();
    assert.equal(owner.freezeForClose(), false);
    Object.assign(owner, { dirty: false, pending: { operationId: "original" } });
    assert.throws(() => owner.assertCloseReady()); assert.equal(owner.freezeForClose(), false);
    Object.assign(owner, { pending: null }); const finish = guard.begin();
    assert.equal(owner.freezeForClose(), false); finish(); assert.equal(owner.freezeForClose(), true);
    (owner as unknown as { markInput(): void }).markInput(); owner.assertCloseReady();
    owner.thawAfterClose(); (owner as unknown as { markInput(): void }).markInput(); assert.throws(() => owner.assertCloseReady());
});

test("frozen DataPanel and Capture entry points start no background requests", async () => {
    const data = dataOwner(), capture = captureOwner();
    const host = { request() { throw new Error("No requests while frozen"); } };
    Object.assign(data.owner, { host }); Object.assign(capture.owner, { host });
    assert.equal(data.owner.freezeForClose(), true); assert.equal(capture.owner.freezeForClose(), true);
    await data.owner.refresh(); await capture.owner.initialize(); capture.owner.updateContext();
});

test("reference helpers cannot mutate a frozen owner's state or start requests", async () => {
    const references = emptyReferences(); references.state = "ready";
    const host = { request() { throw new Error("Frozen owner request"); } } as unknown as PanelHost;
    const container = { querySelectorAll() { throw new Error("Frozen owner DOM mutation"); } } as unknown as HTMLElement;
    await loadFormReferences(host, "library", references, container, () => false);
    await loadTagFormReferences(host, "library", references, container, () => false);
    assert.equal(references.state, "ready");
});

test("failed DataPanel reads retain an explicit discard action, blocked while busy and usable after failure", async context => {
    const dom = testDom(context), element = dom.create("aside");
    const { owner, drafts, draftCache, typeDrafts, schemaDrafts } = dataOwner();
    drafts.set("current", field()); draftCache.set("hidden", new Map([["raw", { ...field(), rawInputs: { exact: "-" } }]]));
    typeDrafts.set("hidden", ["task"]); schemaDrafts.set("library", { source: "{" });
    let rejectRead!: (error: Error) => void, requests = 0, notifications = 0;
    const host = { getDocument: () => basis, request: async () => { if (++requests === 1) throw new Error("offline"); await new Promise((_resolve, reject) => { rejectRead = reject; }); }, notify() { notifications++; } } as unknown as PanelHost;
    const reminder = new ReminderDeviceControl(host);
    Object.assign(owner, { host, element, reminderDevice: reminder, basis, generation: 0, schema: null, projection: null });
    await owner.refresh();
    const label = uiTextValue(message("dataPanel.discardAllFormChanges"));
    assert.equal(owner.freezeForClose(), false); assert.equal(drafts.size, 1); assert.equal(draftCache.size, 1);
    const discard = dom.button(element, label), reading = owner.refresh();
    discard.onclick?.(); assert.equal(notifications, 1); assert.equal(drafts.size, 1); assert.equal(draftCache.size, 1);
    rejectRead(new Error("still offline")); await reading;
    dom.button(element, label).onclick?.();
    assert.equal(drafts.size, 0); assert.equal(draftCache.size, 0); assert.equal(typeDrafts.size, 0); assert.equal(schemaDrafts.size, 0);
    assert.equal(element.querySelectorAll("button").some(button => button.textContent === label), false);
    assert.equal(owner.freezeForClose(), true);
});

test("a type selection reverted during Apply remains a new proposal after the original delivery", async context => {
    const dom = testDom(context), element = dom.create("aside"), { owner, typeDrafts } = dataOwner();
    const schema = { fields: [], definitions: {}, aliases: {}, types: ["A", "B"].map(id => ({ id, fields: [], requires: [], conflicts: [] })) };
    let deliver!: () => void, entered!: () => void;
    const delivered = new Promise<void>(resolve => { deliver = resolve; }), applying = new Promise<void>(resolve => { entered = resolve; });
    const host = { getDocument: () => basis, request: async () => ({ from: 0, to: 0, text: "patch" }), applyPatch: async () => { entered(); await delivered; }, notify() {} } as unknown as PanelHost;
    Object.assign(owner, { host, element, basis, schema, draftIdentity: "note", refresh: async () => {} });
    (owner as unknown as { renderTypes(schema: unknown, tokens: string[]): void }).renderTypes(schema, ["A"]);
    const checkbox = element.querySelectorAll("input")[1]; checkbox.checked = true; checkbox.onchange?.();
    const apply = (owner as unknown as { applyTypes(types: string[], basis: DocumentBasis, identity: string): Promise<void> }).applyTypes(["A", "B"], basis, "note");
    await applying; checkbox.checked = false; checkbox.onchange?.();
    assert.deepEqual(typeDrafts.get("note"), ["A"]); deliver(); await apply;
    assert.deepEqual(typeDrafts.get("note"), ["A"]); assert.equal(owner.freezeForClose(), false);
});

test("ReminderDevice retains original retry/discard across failed reads and retire, never discarding in-flight work", async context => {
    const dom = testDom(context); let documentBasis: DocumentBasis | null = basis;
    let finishRequest!: () => void, entered!: () => void;
    const delivered = new Promise<void>(resolve => { finishRequest = resolve; }), submitting = new Promise<void>(resolve => { entered = resolve; });
    const requests: unknown[] = [];
    const host = { getDocument: () => documentBasis, request: async (route: string, body: unknown) => { if (route === "/api/reminders/inspect") throw new Error("offline"); requests.push(body); entered(); await delivered; }, notify() {} } as unknown as PanelHost;
    const owner = new ReminderDeviceControl(host), root = owner.element as unknown as ReturnType<typeof dom.create>;
    const pending = new Map([["retained", { route: "/api/reminders/library", request: { mutationId: "original" }, basis }]]);
    Object.assign(owner, { pending });
    await owner.refresh(basis);
    const discardLabel = uiTextValue(message("reminder.discardPending")), retryLabel = uiTextValue(message("reminder.retryOriginal"));
    dom.button(root, discardLabel); documentBasis = null; owner.retire();
    dom.button(root, retryLabel).onclick?.(); await submitting;
    const busyDiscard = dom.button(root, discardLabel); assert.equal(busyDiscard.disabled, true); busyDiscard.onclick?.();
    assert.equal(pending.size, 1); assert.equal(owner.freezeForClose(), false);
    finishRequest(); await new Promise<void>(resolve => setImmediate(resolve));
    assert.deepEqual(requests, [{ mutationId: "original" }]); assert.equal(pending.size, 0); assert.equal(owner.freezeForClose(), true);
    owner.thawAfterClose(); pending.set("retained", { route: "/api/reminders/library", request: { mutationId: "unknown" }, basis }); owner.retire();
    dom.button(root, discardLabel).onclick?.(); await new Promise<void>(resolve => setImmediate(resolve));
    assert.equal(pending.size, 0); assert.equal(owner.freezeForClose(), true);
});

test("DataPanel keeps retained reminder actions reachable with no document, including after form discard", async context => {
    const dom = testDom(context), element = dom.create("aside"), { owner, drafts } = dataOwner();
    const host = { getDocument: () => null, notify() {} } as unknown as PanelHost;
    const reminder = new ReminderDeviceControl(host);
    Object.assign(reminder, { pending: new Map([["old-note", { route: "/api/reminders/library", request: { mutationId: "unknown" }, basis }]]) });
    drafts.set("form", field());
    Object.assign(owner, { host, element, reminderDevice: reminder, basis, generation: 0, reminderDisposers: [] });
    await owner.refresh();
    const label = uiTextValue(message("reminder.discardPending")); dom.button(element, label);
    dom.button(element, uiTextValue(message("dataPanel.discardAllFormChanges"))).onclick?.();
    assert.equal(drafts.size, 0); assert.equal(owner.freezeForClose(), false);
    dom.button(element, label).onclick?.(); await new Promise<void>(resolve => setImmediate(resolve));
    assert.equal(reminder.hasPending(), false); assert.equal(owner.freezeForClose(), true);
});
