import assert from "node:assert/strict";
import test from "node:test";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import ts from "typescript";
import { TestDocument, TestElement } from "./dom.test-support";
import { PersonalControlInput } from "./personal-control-input";
import { PersonalInputRecovery, PERSONAL_COPY_ONLY_SLOT, isPersonalCopy, personalInputText, type PersonalInputRecord } from "./personal-input-recovery";
import { PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
import { renderPersonalInputs } from "./personal-input-panel";
import { UiError, message as uiMessage, setUiText, setUiAttr, errorUiText } from "./ui-locale";

const audience = JSON.stringify(["deployment", "alice"]), bob = JSON.stringify(["deployment", "bob"]);
class Backend implements PersonalInputBackend {
    data = new Map<string, PersonalInputBundle>(); fail = false;
    gate?: () => Promise<void>;
    async read(key: string) { return this.data.has(key) ? structuredClone(this.data.get(key)!) : undefined; }
    async compareAndWrite(key: string, expected: number, metadata: string, texts: ReadonlyMap<string, string>) {
        if (this.gate) await this.gate();
        if (this.fail) throw new Error("quota");
        if ((this.data.get(key)?.commitId ?? 0) !== expected) throw new PersonalInputStoreConflict();
        this.data.set(key, { commitId: expected + 1, metadata, texts: new Map(texts) });
        return expected + 1;
    }
}
class Element extends TestElement {
    selectionStart: number | null = 0; selectionEnd: number | null = 0;
    get parentElement() { return this.parentNode; }
    focus() { (this.ownerDocument as DocumentBoundary).activeElement = this; }
    matches() { return false; }
}
class DocumentBoundary extends TestDocument {
    activeElement?: Element;
    listeners = new Map<string, ((event: any) => void)[]>();
    createElement(tag: string): Element { return new Element(tag, this); }
    addEventListener(type: string, listener: (event: any) => void) { this.listeners.set(type, [...this.listeners.get(type) ?? [], listener]); }
}
function deferred() {
    let resolve!: () => void;
    const promise = new Promise<void>(done => { resolve = done; });
    return { promise, resolve };
}
const file = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = new Set(["personalAudience", "observePersonalWrite", "personalCopyCurrent", "retirePersonalCopy", "editPersonalCopy", "finishPersonalCopy", "showPersonalInputs", "verifyPersonalIdentity", "discardRetiredPersonalProducer", "restorePersonalInput"]);
const functions = file.statements.filter(node => ts.isFunctionDeclaration(node) && !!node.name && names.has(node.name.text));
assert.equal(functions.length, names.size, "every selected hosted copy owner function must be the production function");
const compiled = ts.transpileModule(functions.map(node => node.getText(file)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
const unload = file.statements.find(node => ts.isExpressionStatement(node) && ts.isCallExpression(node.expression)
    && ts.isPropertyAccessExpression(node.expression.expression) && node.expression.expression.expression.getText(file) === "window"
    && node.expression.expression.name.text === "addEventListener" && node.expression.arguments[0]?.getText(file) === '"beforeunload"');
assert.ok(unload && ts.isExpressionStatement(unload) && ts.isCallExpression(unload.expression));
const unloadCallback = unload.expression.arguments[1].getText(file);

async function fixture(run: (f: Awaited<ReturnType<typeof setup>>) => Promise<void>) {
    const previous = [globalThis.HTMLInputElement, globalThis.HTMLTextAreaElement, globalThis.document];
    Object.assign(globalThis, { HTMLInputElement: Element, HTMLTextAreaElement: Element });
    async function setup() {
        const backend = new Backend(), recovery = new PersonalInputRecovery(backend), document = new DocumentBoundary();
        Object.assign(globalThis, { document });
        await recovery.open(audience); await recovery.open(bob);
        await recovery.update(audience, [{ id: "note-input", owner: { kind: "body", resource: "Shared", reference: "weftext-node://workspace/node" }, slot: "notes:memo:body",
            origin: { baseLength: 1000, pieces: [], detached: [{ id: "note-input", revision: 1, text: "甲乙😀\ud800" }], revision: 1 },
            pending: { operationId: "original-note-operation", status: "unresolved", route: "known", resource: "Shared" } }]);
        const original = recovery.list(audience)[0], panel = document.createElement("section"), list = document.createElement("div"), mount = document.createElement("section");
        document.body.append(panel);
        const requests: { path: string; body: unknown }[] = [], messages: unknown[] = [];
        const context: any = { document, personalInputs: recovery, personalInputList: list, personalCopyMount: mount,
            personalCopyVersion: 0, personalCopyEditor: undefined, authorizationEpoch: 1, token: "alice-credential",
            identity: { actor: "alice", deployment: "deployment", generation: 1 }, verifiedPersonalAudience: audience,
            crypto, structuredClone, UiError, uiMessage, setUiText, setUiAttr, errorUiText, isPersonalCopy, personalInputText, PERSONAL_COPY_ONLY_SLOT,
            renderPersonalInputs, el: (id: string) => { assert.equal(id, "personal-inputs"); return panel; },
            controls() {}, status: (message: unknown) => messages.push(message),
            structures: { hasPending: false }, media: { hasPending: false }, library: { hasPending: false }, history: { hasPending: () => false }, draft: undefined,
            authorizationsChanged() { throw new Error("unexpected authorization change branch"); },
        };
        context.request = async (path: string, body?: unknown) => { requests.push({ path, body }); assert.equal(path, "me", "copy editing does not call an author effect API"); return { ...context.identity }; };
        const collector = new PersonalControlInput(document as unknown as Document, value => value.audience === context.personalAudience() && value.epoch === context.authorizationEpoch, () => { throw new Error("copy must use its explicit sink"); });
        context.personalControls = collector;
        vm.createContext(context); vm.runInContext(compiled, context);
        context.showPersonalInputs();
        const closeGuard = vm.runInContext(ts.transpileModule(`(${unloadCallback})`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText, context);
        const closeBlocked = () => { let blocked = false; closeGuard({ preventDefault() { blocked = true; } }); return blocked; };
        const fire = (control: Element, type: string, data?: string, inputType = "insertText") => {
            for (const listener of document.listeners.get(type) ?? []) listener({ target: control, data, inputType, defaultPrevented: false });
        };
        const insert = (control: Element, text: string) => {
            control.selectionStart = control.selectionEnd = control.value.length;
            fire(control, "beforeinput"); control.value += text; control.selectionStart = control.selectionEnd = control.value.length; fire(control, "input", text);
        };
        const retire = (nextAudience = bob) => {
            context.retirePersonalCopy(); list.replaceChildren(); panel.replaceChildren(); collector.clear();
            context.authorizationEpoch++; context.token = "new-credential"; context.verifiedPersonalAudience = nextAudience;
            context.identity = { actor: nextAudience === bob ? "bob" : "alice", deployment: "deployment", generation: 2 };
        };
        return { context, backend, recovery, collector, document, original, panel, list, mount, requests, messages, fire, insert, retire, closeBlocked };
    }
    try { await run(await setup()); }
    finally { Object.assign(globalThis, { HTMLInputElement: previous[0], HTMLTextAreaElement: previous[1], document: previous[2] }); }
}

test("a recovered note opens a stable independent copy and Finish accepts only that copy", () => fixture(async f => {
    const original = structuredClone(f.original);
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, textarea = editor.textarea as Element;
    assert.equal(textarea.value, personalInputText(original)); assert.equal(textarea.readOnly, false);
    assert.equal(editor.session.inspect().unapplied, false, "an acknowledged initial seed is not new unapplied input");
    f.context.showPersonalInputs(); f.context.showPersonalInputs();
    assert.equal(f.context.personalCopyEditor.textarea, textarea, "list refresh must retain the same native editor");
    f.insert(textarea, "丙"); await f.recovery.flush(audience);
    const changed = f.recovery.current(audience, editor.record);
    assert.equal(personalInputText(changed), personalInputText(original) + "丙");
    assert.equal(changed.slot, PERSONAL_COPY_ONLY_SLOT); assert.equal(changed.baseHash, undefined); assert.equal(changed.pending, undefined); assert.equal(changed.candidate, undefined);
    assert.deepEqual(f.recovery.current(audience, original), original, "the original owner, input and operation remain unchanged");
    assert.equal(editor.session.inspect().unapplied, true, "ordinary Browser ACK does not apply a personal copy");
    await f.context.finishPersonalCopy(editor);
    assert.equal(f.context.personalCopyEditor, undefined); assert.equal(f.mount.children.length, 0);
    assert.equal(f.collector.inspect().producers.length, 0);
    assert.equal(personalInputText(f.recovery.list(audience).find(record => isPersonalCopy(record))!), personalInputText(original) + "丙");
    assert.deepEqual(f.recovery.current(audience, original), original);
}));

test("native composition and pending paste leases cannot Finish a copy", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, textarea = editor.textarea as Element;
    textarea.selectionStart = textarea.selectionEnd = textarea.value.length;
    f.fire(textarea, "compositionstart");
    await assert.rejects(f.context.finishPersonalCopy(editor));
    assert.equal(editor.session.inspect().nativePending, true);
    textarea.value += "中文"; f.fire(textarea, "compositionend", "中文"); await f.recovery.flush(audience);
    assert.equal(personalInputText(editor.record), personalInputText(f.original) + "中文");
    const lease = editor.session.capture(textarea)!;
    await assert.rejects(f.context.finishPersonalCopy(editor));
    assert.equal(editor.session.inspect().pendingDeliveries, 1); lease.release();
    await f.context.finishPersonalCopy(editor);
}));

test("quota refusal preserves the whole copy in RAM and cannot acknowledge its producer", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, durable = structuredClone(f.backend.data);
    f.backend.fail = true; f.insert(editor.textarea, "最新");
    await assert.rejects(editor.lastWrite, /quota/);
    await assert.rejects(f.context.finishPersonalCopy(editor), /quota/);
    assert.equal(editor.textarea.readOnly, false); assert.equal(editor.session.inspect().unapplied, true);
    assert.equal(f.closeBlocked(), true, "the production close guard retains the refused copy cut");
    assert.deepEqual(f.backend.data, durable); assert.equal(personalInputText(editor.record), personalInputText(f.original) + "最新");
    assert.equal(f.recovery.pending(), true); assert.deepEqual(f.recovery.current(audience, f.original), f.original);
    f.backend.fail = false; await f.context.finishPersonalCopy(editor);
    assert.equal(f.context.personalCopyEditor, undefined);
}));

test("a synchronous copy retain failure cannot Finish an older acknowledged record", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, update = f.recovery.update.bind(f.recovery);
    f.recovery.update = (() => { throw new Error("copy validation refused"); }) as typeof f.recovery.update;
    f.insert(editor.textarea, "未保留");
    await assert.rejects(f.context.finishPersonalCopy(editor));
    assert.equal(editor.session.inspect().unapplied, true); assert.equal(editor.retentionFailure, true);
    assert.equal(f.closeBlocked(), true);
    assert.equal(editor.textarea.value, personalInputText(f.original) + "未保留");
    assert.equal(f.context.personalCopyEditor, editor); assert.deepEqual(f.recovery.current(audience, f.original), f.original);
    f.recovery.update = update;
}));

test("a refused initial copy write remains RAM-only until explicitly reopened and acknowledged", () => fixture(async f => {
    const durable = structuredClone(f.backend.data);
    f.backend.fail = true;
    await assert.rejects(f.context.editPersonalCopy(f.original, audience), /quota/);
    assert.equal(f.context.personalCopyEditor, undefined); assert.deepEqual(f.backend.data, durable);
    const copy = f.recovery.list(audience).find(record => isPersonalCopy(record))!;
    assert.equal(personalInputText(copy), personalInputText(f.original)); assert.equal(f.recovery.pending(), true);
    assert.throws(() => f.recovery.personalCopySeed(audience, copy));
    f.backend.fail = false;
    await f.context.editPersonalCopy(copy, audience);
    const editor = f.context.personalCopyEditor;
    assert.equal(editor.record.id, copy.id); assert.equal(editor.textarea.value, personalInputText(f.original));
    assert.equal(editor.session.inspect().unapplied, false);
    await f.context.finishPersonalCopy(editor);
    assert.deepEqual(f.recovery.current(audience, f.original), f.original);
}));

test("a late copy creation ACK cannot mount an old actor's input", () => fixture(async f => {
    const entered = deferred(), release = deferred();
    f.backend.gate = async () => { entered.resolve(); await release.promise; };
    const opening = f.context.editPersonalCopy(f.original, audience);
    await entered.promise; f.retire(); release.resolve();
    await assert.rejects(opening);
    assert.equal(f.context.personalCopyEditor, undefined); assert.equal(f.mount.children.length, 0); assert.equal(f.panel.children.length, 0);
    assert.deepEqual(f.recovery.list(bob), []);
    assert.equal(f.recovery.list(audience).filter(record => isPersonalCopy(record)).length, 1, "old actor's acknowledged text remains in its own domain");
}));

test("retiring during a Finish ACK preserves the old producer barrier without changing the new account", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, entered = deferred(), release = deferred();
    f.backend.gate = async () => { entered.resolve(); await release.promise; };
    f.insert(editor.textarea, "迟到"); await entered.promise;
    const finishing = f.context.finishPersonalCopy(editor); await Promise.resolve(); await Promise.resolve();
    f.retire(); release.resolve(); await assert.rejects(finishing);
    const retired = f.collector.inspect().producers.find(item => item.producerId === editor.session.inspect().producerId)!;
    assert.equal(retired.retired, true); assert.equal(retired.unapplied, true);
    assert.equal(f.closeBlocked(), true, "another current actor does not hide the original producer close barrier");
    assert.equal(f.context.personalCopyEditor, undefined); assert.equal(f.panel.children.length, 0); assert.deepEqual(f.recovery.list(bob), []);
}));

test("an old producer's discard cannot delete its currently reopened editable copy", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const first = f.context.personalCopyEditor;
    f.insert(first.textarea, "甲"); await f.recovery.flush(audience);
    const previous = first.session.inspect(), record = structuredClone(first.record);
    f.context.retirePersonalCopy();
    await f.context.editPersonalCopy(record, audience);
    const active = f.context.personalCopyEditor;
    await assert.rejects(f.context.discardRetiredPersonalProducer({ ...previous, retired: true }, audience));
    assert.deepEqual(f.recovery.current(audience, record), record); assert.equal(f.context.personalCopyEditor, active);
    f.context.showPersonalInputs();
    const discard = f.list.all().find(node => node.tag === "button" && node.textContent === "明确放弃此退休表单的全部未应用输入")!;
    assert.equal(discard.disabled, true);
}));

test("a forged copy clone cannot choose author Restore and a stale copy row only focuses its existing editor", () => fixture(async f => {
    await f.context.editPersonalCopy(f.original, audience);
    const editor = f.context.personalCopyEditor, old = structuredClone(editor.record);
    await assert.rejects(f.context.restorePersonalInput({ ...old, slot: undefined, owner: { kind: "body", resource: "Forged" } }, audience));
    f.insert(editor.textarea, "后续"); await f.recovery.flush(audience);
    await f.context.editPersonalCopy(old, audience);
    assert.equal(f.context.personalCopyEditor, editor); assert.equal(f.document.activeElement, editor.textarea);
    assert.deepEqual(f.recovery.current(audience, f.original), f.original);
    assert.equal(f.requests.every(request => request.path === "me"), true);
}));
