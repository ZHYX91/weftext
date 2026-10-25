import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { TestDocument } from "./dom.test-support";
import { LatexInsertionPanel } from "./latex-insertion";
import { DiagramController, diagramOccurrences, diagramPatch } from "./diagram-state";
import { action, ui, type PanelHost } from "./data-panel";
import { createNativeProposalOwner } from "./native-proposal-owners";
import { UiError, message as uiMessage, setUiText, setUiAttr, displayMessage, uiDisplayParameter, errorUiText, uiTextValue } from "./ui-locale";

const hostedSource = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const hostDeclaration = hostedSource.statements.find(node => ts.isVariableStatement(node) && node.declarationList.declarations.some(item => item.name.getText(hostedSource) === "authoringHost"));
assert.ok(hostDeclaration, "exercise the actual hosted authoring host");
const diagramSource = ts.createSourceFile("diagram-ui.ts", readFileSync(new URL("./diagram-ui.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const diagramDeclaration = diagramSource.statements.find(node => ts.isClassDeclaration(node) && node.name?.text === "DiagramPanel");
assert.ok(diagramDeclaration);
// Only the offline PNG renderer and document/editor ports are replaced. This
// executes the actual diagram controller, panel and hosted apply boundary.
const compiled = ts.transpileModule(hostDeclaration.getText(hostedSource) + "\n" + diagramDeclaration.getText(diagramSource).replace(/^export\s+/, "") + "\nglobalThis.host = authoringHost; globalThis.Panel = DiagramPanel;", { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
const settle = () => new Promise<void>(resolve => setImmediate(resolve));
const png = Uint8Array.from(Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+ip1sAAAAASUVORK5CYII=", "base64"));

function fixture(kind: "latex" | "diagram") {
  const prior = Object.getOwnPropertyDescriptor(globalThis, "document"), document = new TestDocument();
  Object.defineProperty(globalThis, "document", { configurable: true, value: document });
  const applied: any[] = [], notices: string[] = [];
  let driftDuringFlush = false;
  const source = { value: "Original", serial: 7, authorInputId: crypto.randomUUID(), pausedSession: false, selection: { from: 8, to: 8 },
    async flush() { if (driftDuringFlush) { driftDuringFlush = false; source.authorInputId = crypto.randomUUID(); } },
    async applyQueryPatches(patches: any[], basis: any, event: string) { applied.push({ patches, basis, event }); source.value = source.value.slice(0, patches[0].from) + patches[0].insert + source.value.slice(patches[0].to); source.serial++; source.authorInputId = crypto.randomUUID(); } };
  const context: any = { doc: { sessionId: "writer", baseId: "base", resource: "Shared", source: "Original" }, identity: { actor: "alice" }, draft: undefined, source,
    document, crypto, URL, Blob, Uint8Array, UiError, uiMessage, setUiText, setUiAttr, displayMessage, uiDisplayParameter, errorUiText,
    DiagramController, diagramOccurrences, diagramPatch, createNativeProposalOwner, action, ui,
    authoringEditable: () => true, retain() {}, authorInputs: () => undefined,
    status: (message: unknown) => notices.push(uiTextValue(message as any)),
    request: async (path: string, request: any) => { assert.equal(path, "editor/preview"); return { ok: true, source: request.proposalSource, serial: request.serial, tree: { kind: "text", key: "preview", text: "compiled formula" } }; },
    renderDiagramPng: async (source: string) => ({ source, png, width: 1, height: 1 }),
  };
  vm.createContext(context); vm.runInContext(compiled, context);
  const host = context.host as PanelHost;
  const panel = kind === "latex" ? new LatexInsertionPanel({ ...host, selection: () => source.selection, editable: () => true }) : new context.Panel(host);
  return { context, source, host, panel, document, applied, notices, drift: () => { driftDuringFlush = true; },
    restore() { panel.clear(); if (prior) Object.defineProperty(globalThis, "document", prior); else Reflect.deleteProperty(globalThis, "document"); } };
}

for (const kind of ["latex", "diagram"] as const) {
  for (const change of ["unchanged", "beforeApply", "duringFlush"] as const) {
    test(`actual hosted ${kind} authoring ${change === "unchanged" ? "applies the same input basis" : "rejects a same-byte ABA " + change}`, async () => {
      const f = fixture(kind);
      try {
        const basis = f.host.getDocument()!;
        assert.equal(basis.inputId, f.source.authorInputId, "the hosted panel receives the real author identity");
        await f.panel.open(); const dialog = f.document.body.children[0];
        if (kind === "latex") { dialog.find("textarea").value = "x"; dialog.find("button", "转换并预览").onclick!(); }
        else dialog.find("button", uiTextValue(uiMessage("diagramPanel.generatePngPreview"))).onclick!();
        await settle();
        const commit = dialog.find("button", uiTextValue(uiMessage(kind === "latex" ? "mathLatex.confirmInsert" : "diagramPanel.confirmImageWriteDraft")));
        assert.equal(commit.disabled, false);
        if (change === "beforeApply") f.source.authorInputId = crypto.randomUUID();
        if (change === "duringFlush") f.drift();
        commit.onclick!(); await settle();
        if (change === "unchanged") {
          assert.equal(f.applied.length, 1); assert.equal(f.applied[0].basis.inputId, basis.inputId);
          assert.equal(f.applied[0].event, "input.authoring.proposal"); assert.equal(f.document.body.children.length, 0);
          assert.ok(f.source.value.includes(kind === "latex" ? "$x$" : "// weftext-mermaid v1"));
        } else {
          assert.equal(f.applied.length, 0); assert.equal(f.source.value, basis.source); assert.equal(f.source.serial, basis.serial);
          assert.notEqual(f.source.authorInputId, basis.inputId); assert.equal(dialog.open, true);
          if (change === "duringFlush") assert.ok(dialog.textContent.includes(uiTextValue(uiMessage("hostedShell.hostedProposalBasisChanged"))));
        }
      } finally { f.restore(); }
    });
  }
}
