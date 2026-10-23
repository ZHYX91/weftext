import assert from "node:assert/strict";
import test from "node:test";
import { ReminderOverview } from "./reminder-overview";
import { TestDocument, TestElement } from "./dom.test-support";
import { message, uiTextValue } from "./ui-locale";
import type { PanelHost } from "./data-panel";

class Element extends TestElement {
    contains(node: TestElement): boolean { return this.all().includes(node); }
    matches(selector: string): boolean { return selector === ":disabled" && this.disabled; }
}
class Document extends TestDocument { override createElement(tag: string): Element { return new Element(tag, this); } }

test("Reminder settings read failure retains a visible explicit discard for its original pending operation", async t => {
    const previous = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new Document();
    Object.defineProperty(globalThis, "document", { configurable: true, writable: true, value: document });
    t.after(() => { if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document"); });
    const host = { getDocument: () => null, openDocument: async () => {}, applyPatch: async () => {}, notify() {}, async request(route: string) {
        if (route === "/api/reminders/settings") throw new Error("settings unavailable");
        if (route === "/api/reminders/schedule") return { entries: [], issues: [], complete: true, next: null };
        if (route === "/api/reminders/records") return { records: [], next: null };
        throw new Error(`Unexpected mutation ${route}`);
    } } as unknown as PanelHost;
    const overview = new ReminderOverview(host), element = overview.element as unknown as Element;
    document.body.append(element);
    const pending = new Map([["original-library", { route: "/api/reminders/library", request: { mutationId: "original" } }]]);
    Object.assign(overview, { pending });
    await overview.refresh();
    assert.equal(pending.size, 1); assert.equal(overview.freezeForClose(), false);
    const discard = element.find("button", uiTextValue(message("reminder.discardPending")));
    discard.onclick?.();
    for (let turn = 0; turn < 40; turn++) await Promise.resolve();
    assert.equal(pending.size, 0); assert.doesNotThrow(() => overview.assertCloseReady());
    assert.equal(overview.freezeForClose(), true); overview.thawAfterClose(); overview.dispose();
});
