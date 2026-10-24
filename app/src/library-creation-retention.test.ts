import assert from "node:assert/strict";
import test from "node:test";
import { TestDocument, TestElement } from "./dom.test-support";
import { LibraryCreationController, openLibraryCreation, type LibraryCreationDefaults, type LibraryCreationHost, type LibraryCreationReceipt, type LibraryCreationRequest } from "./library-creation";
import { assertNativeProposalOwnersCloseReady, freezeNativeProposalOwnersForClose, thawNativeProposalOwnersAfterClose } from "./native-proposal-owners";
import { message, uiTextValue } from "./ui-locale";

class CreationElement extends TestElement {
    style: Record<string, string> = {};
    inert = false;
    private readonly listeners = new Map<string, ((event: Event) => void)[]>();
    override addEventListener(...args: unknown[]): void {
        const [name, listener] = args as [string, (event: Event) => void];
        this.listeners.set(name, [...this.listeners.get(name) ?? [], listener]);
    }
    dispatch(name: string): void {
        const event = { type: name, target: this, preventDefault() {}, stopPropagation() {}, stopImmediatePropagation() {} } as unknown as Event;
        for (const listener of this.listeners.get(name) ?? []) listener(event);
    }
    focus(): void { (this.ownerDocument as CreationDocument).activeElement = this; }
}
class CreationDocument extends TestDocument {
    activeElement: TestElement | null = null;
    override createElement(tag: string): CreationElement { return new CreationElement(tag, this); }
}
const key = "weftext-library-creation-v1";
const native = '#let library-style(body) = { set text(lang: "zh", region: "CN"); body }';
function defaults(): LibraryCreationDefaults {
    return { source: native, sourceHash: "frozen", controls: [["text.lang", '"zh"'], ["text.region", '"CN"']].map(([key, value]) => {
        const from = native.indexOf(value);
        return { key, value, editable: true, range: { utf16From: from, utf16To: from + value.length, raw: value } };
    }) };
}
function fixture(overrides: Partial<LibraryCreationHost> = {}) {
    const document = new CreationDocument(), saved = new Map<string, string>();
    const storage = { getItem: (key: string) => saved.get(key) ?? null, setItem: (key: string, value: string) => { saved.set(key, value); }, removeItem: (key: string) => { saved.delete(key); } };
    Object.assign(globalThis, { document, HTMLElement: CreationElement, localStorage: storage });
    const host: LibraryCreationHost = {
        defaults: async () => defaults(),
        create: async request => ({ operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }),
        delivery: async operationId => ({ operationId, root: "D:/Owned/Library", workspaceId: "original", status: "committed" }),
        deliver: async () => {},
        inspect: async operationId => ({ operationId, status: "pending", description: { explanation: message("common.cancel"), diagnostics: [], causes: [] } }),
        resolve: async operationId => ({ operationId, status: "resolved", description: { explanation: message("common.cancel"), diagnostics: [], causes: [] } }),
        cancel: async request => ({ operationId: request.operationId, status: "cancelled", description: { explanation: message("common.cancel"), diagnostics: [], causes: [] } }),
        ...overrides,
    };
    return { document, saved, storage, host };
}
async function flush(): Promise<void> { for (let i = 0; i < 4; i++) await new Promise<void>(resolve => setImmediate(resolve)); }
function button(dialog: TestElement, id: Parameters<typeof message>[0]): CreationElement { return dialog.find("button", uiTextValue(message(id))) as CreationElement; }
async function click(button: TestElement): Promise<void> { button.onclick?.(); await flush(); }
function prepare(dialog: TestElement): void {
    const path = dialog.all().find(node => node.getAttribute("aria-label") === uiTextValue(message("libraryCreation.newLibraryPath")))!;
    path.value = "D:/Owned/Library"; path.oninput?.();
    button(dialog, "libraryCreation.previewCreation").onclick?.();
}
const unusedHost: LibraryCreationHost = new Proxy({} as LibraryCreationHost, { get() { throw new Error("Reopening must retain the original host and owner"); } });

test("committed creation hides and reopens its exact owner, then releases after original delivery", async () => {
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    let authors = 0, deliveries = 0, callbacks = 0;
    let original!: LibraryCreationReceipt;
    const f = fixture({
        create: async request => { authors++; return original = { operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }; },
        delivery: async id => { deliveries++; assert.equal(id, original.operationId); return structuredClone(original); },
        deliver: async () => { if (++callbacks === 1) throw new Error("refresh 500 after committed creation"); },
    });
    const firstFocus = f.document.createElement("textarea"); f.document.body.append(firstFocus); firstFocus.focus();
    const first = openLibraryCreation(f.host);
    assert.equal(openLibraryCreation(unusedHost), first);
    await flush();
    const dialog = f.document.body.find("dialog") as CreationElement;
    prepare(dialog); await click(button(dialog, "shell.createLibrary"));
    assert.match(dialog.textContent, /refresh 500/);
    const retained = f.saved.get(key)!;
    assert.ok(JSON.parse(retained).receipt);
    const oldClose = button(dialog, "common.close");
    await click(oldClose);
    assert.equal(dialog.open, false, "Close must hide a committed original whose UI delivery failed");
    assert.equal(dialog.isConnected, false);
    assert.equal(await first, null);
    assert.equal(f.document.activeElement, firstFocus);
    assert.equal(f.saved.get(key), retained);
    assert.throws(assertNativeProposalOwnersCloseReady);
    assert.equal(freezeNativeProposalOwnersForClose(), false);

    const laterFocus = f.document.createElement("textarea"); f.document.body.append(laterFocus); laterFocus.value = "Waiting Y 😀"; laterFocus.focus();
    const second = openLibraryCreation(unusedHost); await flush();
    assert.equal(f.document.body.find("dialog"), dialog, "Reopen must reuse the bound DOM and live controller");
    assert.equal(f.saved.get(key), retained);
    await click(button(dialog, "common.close")); assert.equal(await second, null);
    assert.equal(f.document.activeElement, laterFocus, "Each show restores its own caller's focus");
    const third = openLibraryCreation(unusedHost); await flush();
    await click(button(dialog, "libraryCreation.confirmSameCreation"));
    assert.deepEqual(await third, original);
    assert.equal(authors, 1); assert.equal(deliveries, 1); assert.equal(callbacks, 2);
    assert.equal(laterFocus.value, "Waiting Y 😀");
    assert.equal(f.saved.has(key), false);
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady, "The original owner must retire, without an orphan duplicate");

    const replacement = openLibraryCreation(f.host); await flush();
    const newDialog = f.document.body.find("dialog"); assert.notEqual(newDialog, dialog);
    prepare(newDialog);
    await click(oldClose);
    assert.equal(newDialog.open, true, "Late old Close cannot settle or retire a replacement");
    assert.throws(assertNativeProposalOwnersCloseReady);
    await click(button(newDialog, "common.close")); assert.equal(await replacement, null);
    assert.equal(f.saved.has(key), false);
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
});

test("unknown creation retains exact request while hidden until explicit durable cancellation and discard", async () => {
    const requests: LibraryCreationRequest[] = [];
    const f = fixture({ create: async request => { requests.push(structuredClone(request)); throw new Error("lost response"); } });
    const first = openLibraryCreation(f.host); await flush();
    const dialog = f.document.body.find("dialog") as CreationElement;
    prepare(dialog); await click(button(dialog, "shell.createLibrary"));
    const retained = f.saved.get(key)!;
    dialog.dispatch("cancel"); await flush();
    assert.equal(dialog.open, false); assert.equal(await first, null);
    assert.equal(f.saved.get(key), retained); assert.throws(assertNativeProposalOwnersCloseReady);
    const second = openLibraryCreation(unusedHost); await flush();
    await click(button(dialog, "libraryCreation.confirmSameCreation"));
    assert.equal(requests.length, 2); assert.deepEqual(requests[1], requests[0]);
    assert.throws(assertNativeProposalOwnersCloseReady);
    await click(button(dialog, "libraryCreation.endUnstartedOriginal"));
    assert.equal(JSON.parse(f.saved.get(key)!).request, null);
    assert.throws(assertNativeProposalOwnersCloseReady, "Remaining local input still belongs to the controller");
    await click(button(dialog, "common.close")); assert.equal(await second, null);
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
});

test("busy delivery, IME and native freeze prevent hiding or discarding the visible owner", async () => {
    let complete!: (receipt: LibraryCreationReceipt) => void;
    let pending!: LibraryCreationRequest;
    const f = fixture({ create: request => { pending = request; return new Promise(resolve => { complete = resolve; }); } });
    const result = openLibraryCreation(f.host); await flush();
    const dialog = f.document.body.find("dialog") as CreationElement;
    assert.equal(freezeNativeProposalOwnersForClose(), true);
    await click(button(dialog, "common.close")); assert.equal(dialog.open, true);
    thawNativeProposalOwnersAfterClose();
    prepare(dialog);
    dialog.dispatch("compositionstart");
    await click(button(dialog, "common.close")); assert.equal(dialog.open, true);
    dialog.dispatch("compositionend");
    await click(button(dialog, "shell.createLibrary"));
    await click(button(dialog, "common.close")); assert.equal(dialog.open, true);
    assert.throws(assertNativeProposalOwnersCloseReady);
    complete({ operationId: pending.operationId, root: pending.path, workspaceId: "original", status: "committed" }); await flush();
    assert.equal((await result)?.operationId, pending.operationId);
    assert.equal(dialog.open, false); assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
});

test("storage failure after commit keeps the hidden owner and receipt until retry completes", async () => {
    let writable = true, authors = 0, original!: LibraryCreationReceipt;
    const f = fixture({ create: async request => { authors++; writable = false; return original = { operationId: request.operationId, root: request.path, workspaceId: "original", status: "committed" }; }, delivery: async () => structuredClone(original) });
    const save = f.storage.setItem;
    f.storage.setItem = (key, value) => { if (!writable) throw new Error("storage unavailable after commit"); save(key, value); };
    const first = openLibraryCreation(f.host); await flush();
    const dialog = f.document.body.find("dialog"); prepare(dialog);
    await click(button(dialog, "shell.createLibrary"));
    assert.match(dialog.textContent, /storage unavailable/);
    await click(button(dialog, "common.close")); assert.equal(dialog.open, false); assert.equal(await first, null);
    assert.throws(assertNativeProposalOwnersCloseReady);
    writable = true;
    const second = openLibraryCreation(unusedHost); await flush();
    await click(button(dialog, "libraryCreation.confirmSameCreation")); assert.deepEqual(await second, original);
    assert.equal(authors, 1); assert.equal(f.saved.has(key), false);
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
});

test("failed storage initialization can reopen after recovery without leaving a registered owner", async () => {
    const f = fixture();
    const read = f.storage.getItem;
    f.storage.getItem = () => { throw new Error("storage temporarily unreadable"); };
    const freeze = LibraryCreationController.prototype.freezeForClose;
    let freezes = 0;
    LibraryCreationController.prototype.freezeForClose = function () { freezes++; return freeze.call(this); };
    try {
        const first = openLibraryCreation(f.host);
        assert.equal(openLibraryCreation(f.host), first);
        await assert.rejects(first, /storage temporarily unreadable/);
        assert.equal(f.document.body.all().some(node => node.tag === "dialog"), false);
        assert.equal(freezeNativeProposalOwnersForClose(), true);
        thawNativeProposalOwnersAfterClose();
        assert.equal(freezes, 0, "Failed construction must not leave an unreachable registered owner");
        f.storage.getItem = read;
        const recovered = openLibraryCreation(f.host); await flush();
        const dialog = f.document.body.find("dialog");
        assert.equal(freezeNativeProposalOwnersForClose(), true);
        thawNativeProposalOwnersAfterClose();
        assert.equal(freezes, 1, "Recovery installs exactly one actual owner");
        await click(button(dialog, "common.close")); assert.equal(await recovered, null);
        assert.equal(freezeNativeProposalOwnersForClose(), true);
        thawNativeProposalOwnersAfterClose(); assert.equal(freezes, 1);
    } finally { LibraryCreationController.prototype.freezeForClose = freeze; }
});

test("unreadable original recovery evidence may hide but cannot release native ownership", async () => {
    const f = fixture();
    const raw = '{"version":1,"request":"unreadable original"';
    f.saved.set(key, raw);
    const first = openLibraryCreation(unusedHost); await flush();
    const dialog = f.document.body.find("dialog");
    assert.ok(dialog.textContent.includes(raw));
    await click(button(dialog, "common.close"));
    assert.equal(dialog.open, false); assert.equal(await first, null);
    assert.equal(f.saved.get(key), raw);
    assert.throws(assertNativeProposalOwnersCloseReady);
    const second = openLibraryCreation(unusedHost); await flush();
    assert.equal(f.document.body.find("dialog"), dialog);
    await click(button(dialog, "common.close")); assert.equal(await second, null);
    assert.equal(freezeNativeProposalOwnersForClose(), false);
    assert.equal(f.saved.get(key), raw);
});
