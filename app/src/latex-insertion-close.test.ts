import test from "node:test";
import assert from "node:assert/strict";
import { TestDocument } from "./dom.test-support";
import { LatexInsertionPanel } from "./latex-insertion";
import { assertNativeProposalOwnersCloseReady, freezeNativeProposalOwnersForClose, thawNativeProposalOwnersAfterClose } from "./native-proposal-owners";
import type { DocumentBasis, PanelHost, SourcePatch } from "./data-panel";
import type { RenderNode } from "./types";
import type { AuthorInputSession } from "./author-input";

const settle = () => new Promise<void>(resolve => setImmediate(resolve));
function deferred<T>() {
    let resolve!: (value: T) => void, reject!: (reason: unknown) => void;
    const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
    return { promise, resolve, reject };
}
const tree: RenderNode = { kind: "text", key: "preview", tag: null, text: "formula", children: [], origin: null, writable: false, attrs: {} };

function fixture(authorInputs?: AuthorInputSession) {
    const prior = Object.getOwnPropertyDescriptor(globalThis, "document"), doc = new TestDocument();
    Object.defineProperty(globalThis, "document", { configurable: true, value: doc });
    const preview = deferred<void>(), apply = deferred<void>(), prepare = deferred<void>();
    let previewDelayed = false, applyDelayed = false, prepareDelayed = false, applyFailure = false;
    let basis: DocumentBasis = { inputId: "test-input", sessionId: "session", baseId: "base", serial: 1, source: "original", path: "note", root: "" };
    const applied: SourcePatch[] = [];
    const host: PanelHost = {
        authorInputs: () => authorInputs,
        getDocument: () => basis,
        prepareAction: async () => { if (prepareDelayed) await prepare.promise; },
        request: async <T>(_path: string, value?: unknown) => {
            const data = value as { proposalSource: string; serial: number };
            if (previewDelayed) await preview.promise;
            return { ok: true, source: data.proposalSource, serial: data.serial, tree } as T;
        },
        applyPatch: async patch => {
            if (applyDelayed) await apply.promise;
            if (applyFailure) throw new Error("host delivery failed");
            applied.push(patch);
        },
        openDocument: async () => {}, notify: () => {},
    };
    const panel = new LatexInsertionPanel(host);
    return {
        panel, doc, preview, apply, prepare, applied,
        delayPreview: () => { previewDelayed = true; }, delayApply: () => { applyDelayed = true; },
        delayPrepare: () => { prepareDelayed = true; }, failApply: () => { applyFailure = true; },
        changeBasis: () => { basis = { ...basis, serial: basis.serial + 1, source: "changed" }; },
        async restore() {
            thawNativeProposalOwnersAfterClose();
            panel.clear(); preview.resolve(); apply.resolve(); prepare.resolve();
            await settle();
            assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
            if (prior) Object.defineProperty(globalThis, "document", prior); else Reflect.deleteProperty(globalThis, "document");
        },
    };
}

test("native close reveals a hidden LaTeX proposal even after its document basis changes", async () => {
    const f = fixture();
    try {
        await f.panel.open();
        const dialog = f.doc.body.children[0];
        dialog.find("textarea").value = "x";
        dialog.find("button", "关闭").onclick!();
        f.changeBasis();
        assert.equal(dialog.open, false);
        assert.throws(assertNativeProposalOwnersCloseReady);
        assert.equal(dialog.open, true);
        assert.equal(dialog.find("textarea").value, "x");
        dialog.find("button", "取消本次插入").onclick!();
        assert.equal(f.doc.body.children.length, 0);
        assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    } finally { await f.restore(); }
});

test("explicit LaTeX discard waits for a late preview before releasing native close ownership", async () => {
    const f = fixture();
    try {
        f.delayPreview(); await f.panel.open();
        const dialog = f.doc.body.children[0];
        dialog.find("textarea").value = "x";
        dialog.find("button", "转换并预览").onclick!();
        await settle();
        dialog.find("button", "取消本次插入").onclick!();
        assert.equal(f.doc.body.children.length, 0);
        assert.throws(assertNativeProposalOwnersCloseReady);
        assert.equal(freezeNativeProposalOwnersForClose(), false);
        f.preview.resolve(); await settle();
        assert.deepEqual(f.applied, []);
        assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    } finally { await f.restore(); }
});

test("LaTeX host application remains owned until delivery and cannot be cancelled mid-apply", async () => {
    const f = fixture();
    try {
        f.delayApply(); await f.panel.open();
        const dialog = f.doc.body.children[0];
        dialog.find("textarea").value = "x";
        dialog.find("button", "转换并预览").onclick!(); await settle();
        dialog.find("button", "确认插入原生公式").onclick!(); await settle();
        dialog.find("button", "取消本次插入").onclick!();
        assert.equal(dialog.open, true);
        assert.throws(assertNativeProposalOwnersCloseReady);
        f.apply.resolve(); await settle();
        assert.equal(f.applied.length, 1);
        assert.equal(f.doc.body.children.length, 0);
        assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    } finally { await f.restore(); }
});

test("failed LaTeX application keeps the original proposal and native close blocker", async () => {
    const f = fixture();
    try {
        f.failApply(); await f.panel.open();
        const dialog = f.doc.body.children[0];
        dialog.find("textarea").value = "x";
        dialog.find("button", "转换并预览").onclick!(); await settle();
        dialog.find("button", "确认插入原生公式").onclick!(); await settle();
        assert.equal(dialog.find("textarea").value, "x");
        assert.throws(assertNativeProposalOwnersCloseReady);
        dialog.find("button", "取消本次插入").onclick!();
        assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    } finally { await f.restore(); }
});

test("native close covers asynchronous dialog opening and refuses new proposals while frozen", async () => {
    const f = fixture();
    try {
        f.delayPrepare(); const opening = f.panel.open();
        assert.throws(assertNativeProposalOwnersCloseReady);
        assert.equal(freezeNativeProposalOwnersForClose(), false);
        f.prepare.resolve(); await opening;
        f.doc.body.children[0].find("button", "取消本次插入").onclick!();
        assert.equal(freezeNativeProposalOwnersForClose(), true);
        await assert.rejects(f.panel.open());
        assert.equal(f.doc.body.children.length, 0);
        thawNativeProposalOwnersAfterClose();
        await f.panel.open();
        assert.equal(f.doc.body.children.length, 1);
    } finally { await f.restore(); }
});

test("LaTeX cannot apply pending native composition and explicit discard acknowledges after retirement", async () => {
    let retired = false, nativePending = true;
    const session: AuthorInputSession = {
        bind: () => () => {}, bindChoice: () => () => {}, invalidate: () => {}, capture: () => undefined,
        inspect: () => ({ producerId: "formula", revision: 1, ackRevision: 0, audience: "owner", epoch: 1,
            owner: { kind: "body", resource: "note" }, retired, nativePending, hasObservedChanges: true,
            unapplied: nativePending, pendingDeliveries: 0, frozen: false }),
        acknowledge: (_checkpoint, disposition) => {
            assert.equal(disposition, "discarded");
            assert.equal(retired, true, "retirement fences late native events before discard acknowledgement");
            nativePending = false; return true;
        },
        freeze: () => false, thaw: () => {}, retire: () => { retired = true; },
    };
    const f = fixture(session);
    try {
        await f.panel.open(); const dialog = f.doc.body.children[0];
        dialog.find("textarea").value = "x";
        dialog.find("button", "转换并预览").onclick!(); await settle();
        dialog.find("button", "确认插入原生公式").onclick!(); await settle();
        assert.deepEqual(f.applied, []);
        assert.throws(assertNativeProposalOwnersCloseReady);
        dialog.find("button", "取消本次插入").onclick!();
        assert.equal(nativePending, false);
        assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
    } finally { await f.restore(); }
});

