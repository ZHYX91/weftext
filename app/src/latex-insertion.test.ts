import { TestDocument, TestElement } from "./dom.test-support";
import test from "node:test";
import assert from "node:assert/strict";
import { LatexInsertionPanel } from "./latex-insertion";
import type { DocumentBasis, PanelHost, SourcePatch } from "./data-panel";
import type { RenderNode } from "./types";

/** Event/ownership fixture only; real native preview remains candidate coverage. */
type ElementStub = TestElement;
const settle = () => new Promise<void>(resolve => setImmediate(resolve));
const tree: RenderNode = { kind: "text", key: "preview", tag: null, text: "compiled document", children: [], origin: null, writable: false, attrs: {} };

function fixture() {
  const prior = Object.getOwnPropertyDescriptor(globalThis, "document"), testDocument = new TestDocument(), body = testDocument.body;
  Object.defineProperty(globalThis, "document", { configurable: true, value: testDocument });
  let basis: DocumentBasis | null = { inputId: "test-input", sessionId: "session", baseId: "base", serial: 1, source: "甲乙丙丁", path: "resource:note", root: "" }, selection = { from: 0, to: 1 }, editable = true;
  const requests: { source: string; proposalSource: string; serial: number }[] = [], applied: SourcePatch[] = [];
  let deferred: ((value: unknown) => void) | null = null, delay = false;
  const host: PanelHost = {
    getDocument: () => basis,
    request: async <T>(_path: string, data?: unknown): Promise<T> => {
      const request = data as typeof requests[number]; requests.push(request);
      const result = { ok: true, source: request.proposalSource, serial: request.serial, tree };
      if (delay) return await new Promise<T>(resolve => { deferred = value => resolve(value as T); });
      return result as T;
    },
    applyPatch: async patch => { applied.push(patch); }, openDocument: async () => {}, notify: () => {},
  };
  const panel = new LatexInsertionPanel({ ...host, selection: () => selection, editable: () => editable });
  return { panel, body, requests, applied, select: (from: number, to: number) => { selection = { from, to }; },
    revoke: () => { editable = false; basis = null; }, delay: () => { delay = true; }, resolve: () => { const request = requests.at(-1)!; deferred?.({ ok: true, source: request.proposalSource, serial: request.serial, tree }); },
    restore: () => { panel.clear(); if (prior) Object.defineProperty(globalThis, "document", prior); else Reflect.deleteProperty(globalThis, "document"); },
  };
}

test("reopening LaTeX insertion at a different unchanged-source selection freezes a new target and requires a fresh preview", async () => {
  const f = fixture();
  try {
    await f.panel.open(); const first = f.body.children[0]; first.find("textarea").value = "x"; first.find("button", "转换并预览").onclick!(); await settle();
    first.find("button", "关闭").onclick!(); assert.deepEqual(f.applied, []);
    f.select(2, 3); await f.panel.open(); const second = f.body.children.at(-1)!;
    assert.notEqual(second, first); assert.equal(second.find("button", "确认插入原生公式").disabled, true);
    second.find("textarea").value = "y"; second.find("button", "转换并预览").onclick!(); await settle();
    assert.equal(f.requests.at(-1)!.proposalSource, "甲乙$y$丁"); second.find("button", "确认插入原生公式").onclick!(); await settle();
    assert.deepEqual(f.applied, [{ from: 2, to: 3, text: "$y$" }]); assert.equal(first.find("textarea").value, "x", "the previous slot's retained original remains separate from the new insertion capability");
  } finally { f.restore(); }
});

test("revocation clears retained LaTeX dialogs and a late preview cannot recreate or confirm old document content", async () => {
  const f = fixture();
  try {
    f.delay(); await f.panel.open(); const dialog = f.body.children[0]; dialog.find("textarea").value = "x"; dialog.find("button", "转换并预览").onclick!();
    await settle(); assert.equal(f.requests.length, 1); f.revoke(); f.panel.clear(); f.resolve(); await settle();
    assert.equal(f.body.children.length, 0); assert.deepEqual(f.applied, []); await assert.rejects(f.panel.open(), /可编辑笔记/);
  } finally { f.restore(); }
});
