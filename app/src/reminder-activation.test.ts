import assert from "node:assert/strict";
import test from "node:test";
import { activateReminderNotification, openReminderActivationMember } from "./reminder-activation";
import { TestDocument, TestElement } from "./dom.test-support";
import type { PanelHost } from "./data-panel";
import type { OpenedDocument } from "./types";
import { freezeNativeProposalOwnersForClose, thawNativeProposalOwnersAfterClose } from "./native-proposal-owners";

const activationKey = "00000000-0000-4000-8000-000000000001";
const owner = { workspaceId: "00000000-0000-4000-8000-000000000002", nodeId: "00000000-0000-4000-8000-000000000003" };
const second = { ...owner, nodeId: "00000000-0000-4000-8000-000000000004" };
const member = (value = owner) => ({ root: "D:/owned-test", owner: value, resolution: { triggerInstant: "2026-10-25T09:00:00Z" } });
const opened = { sessionId: "original-new-session", source: "task", baseId: "base", root: "D:/owned-test", path: "D:/owned-test/Task/Task.typ", ...owner } as OpenedDocument;
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => resolve = done); return { promise, resolve }; }
const flush = async () => { for (let step = 0; step < 30; step++) await Promise.resolve(); };
class Element extends TestElement {
    private readonly events = new Map<string, (() => void)[]>();
    override addEventListener(type?: string, listener?: () => void): void { if (type && listener) this.events.set(type, [...this.events.get(type) ?? [], listener]); }
    override close(): void { super.close(); for (const listener of this.events.get("close") ?? []) listener(); }
}
class Document extends TestDocument { override createElement(tag: string): Element { return new Element(tag, this); } }
function install(t: test.TestContext) {
    const previous = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new Document();
    Object.defineProperty(globalThis, "document", { configurable: true, value: document });
    t.after(() => {
        thawNativeProposalOwnersAfterClose();
        for (const element of [...document.body.children]) if (element.tag === "dialog") (element as Element).close();
        if (previous) Object.defineProperty(globalThis, "document", previous); else Reflect.deleteProperty(globalThis, "document");
    });
    return document;
}
function fixture(activation: unknown) {
    const calls: { route: string; body?: any }[] = [], adopted: OpenedDocument[] = [], released: string[] = [], errors: unknown[] = [];
    let current: any = null;
    const host = {
        getDocument: () => current, prepareAction: async () => {}, applyPatch: async () => {}, openDocument: async () => {},
        notify: (error: unknown) => errors.push(error), releaseOpenedSession: async (id: string) => { released.push(id); },
        openResolvedConfirmed: async (document: OpenedDocument) => { adopted.push(document); current = document; return true; },
        request: async (route: string, body?: unknown) => { calls.push({ route, body }); return route.endsWith("/open") ? { ...opened, ...(body as any).owner } : activation; },
    } as unknown as PanelHost;
    return { host, calls, adopted, released, errors };
}

test("one original notification member waits for the real document owner before completing", async () => {
    const f = fixture({ candidates: [member()], issues: [] }), received = deferred<void>();
    const adopt = f.host.openResolvedConfirmed!;
    f.host.openResolvedConfirmed = async document => { await received.promise; return adopt(document); };
    let completed = false;
    const pending = activateReminderNotification(f.host, activationKey).then(() => { completed = true; });
    await flush();
    assert.equal(completed, false);
    assert.deepEqual(f.calls, [{ route: "/api/reminders/activation", body: { activationKey } }, { route: "/api/reminders/activation/open", body: { activationKey, owner } }]);
    received.resolve(); await pending;
    assert.equal(completed, true); assert.equal(f.adopted.length, 1);
});

test("a superseded navigation cannot acknowledge the original notification as opened", async () => {
    const f = fixture({ candidates: [member()], issues: [] });
    f.host.openResolvedConfirmed = async () => false;
    await assert.rejects(activateReminderNotification(f.host, activationKey));
    assert.equal(f.adopted.length, 0);
});

test("retired selection releases a late unadopted result without opening another task", async () => {
    const f = fixture(null), response = deferred<OpenedDocument>(); let current = true;
    f.host.request = async () => response.promise as any;
    const pending = openReminderActivationMember(f.host, activationKey, owner, () => current);
    await flush(); current = false; response.resolve(opened);
    await assert.rejects(pending);
    assert.deepEqual(f.released, [opened.sessionId]); assert.deepEqual(f.adopted, []);
});

test("a summary requires a chosen original member and never opens the first one implicitly", async t => {
    const document = install(t), f = fixture({ candidates: [member(), member(second)], issues: [] });
    await activateReminderNotification(f.host, activationKey);
    const dialog = document.body.find("dialog");
    assert.equal(dialog.open, true); assert.equal(f.calls.length, 1);
    const choices = dialog.all().filter(element => element.tag === "section").flatMap(row => row.children.filter(child => child.tag === "button"));
    assert.equal(choices.length, 2); choices[1].onclick?.(); await flush();
    assert.deepEqual(f.calls.at(-1)?.body, { activationKey, owner: second });
    assert.equal(f.adopted[0].nodeId, second.nodeId); assert.equal(dialog.isConnected, false);
});

test("expired and restricted notifications remain explicit and frozen views reject navigation", async t => {
    const document = install(t), empty = fixture({ candidates: [], issues: [] });
    await activateReminderNotification(empty.host, activationKey);
    assert.equal(empty.calls.length, 1); (document.body.find("dialog") as Element).close();
    const limited = fixture({ candidates: [member()], issues: [{ owner: second }] });
    await activateReminderNotification(limited.host, activationKey);
    assert.equal(limited.calls.length, 1);
    assert.equal(freezeNativeProposalOwnersForClose(), true);
    for (const element of document.body.all()) if (element.tag === "button") element.onclick?.();
    await flush(); assert.equal(limited.calls.length, 1); assert.equal(limited.adopted.length, 0);
});

test("malformed activation keys and invalid returned members never open documents", async () => {
    const f = fixture({ candidates: [{ ...member(), owner: { ...owner, nodeId: "wrong" } }], issues: [] });
    await assert.rejects(activateReminderNotification(f.host, "not-original")); assert.equal(f.calls.length, 0);
    await assert.rejects(activateReminderNotification(f.host, activationKey)); assert.equal(f.calls.length, 1); assert.equal(f.adopted.length, 0);
});
