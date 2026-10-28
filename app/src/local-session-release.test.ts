import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { message, uiTextValue, UiError, type UiText } from "./ui-locale";
import { LibraryCloseActivity } from "./library-native-close";
import { WritingController, writingRange, type WritingBuffer, type WritingMap } from "./writing";

// Execute the actual main owner port. Only the HTTP, message rendering and DOM
// presentation boundaries are substituted; Core/auth are tested by desktop_api.
const source = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const ports = source.statements.filter(node => ts.isFunctionDeclaration(node) && ["releaseOpenedSession","ownsOpenedSession","acceptResolvedDocument"].includes(node.name?.text ?? ""));
assert.equal(ports.length,3);
const compiled = ts.transpileModule(ports.map(node=>node.getText(source)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function fixture() {
  const calls: { path: string; body: { sessionId: string } }[] = [];
  const errors: any[] = [];
  const context: any = {
    libraryCloseActivity: new LibraryCloseActivity(),
    documents: [{ session: { opened: { sessionId: "adopted-original" } } }],
    uiMessage: message, UiError,
    setUiAttr: (node: {setAttribute(key: string, value: string):void}, key: string, spec: UiText) => node.setAttribute(key, uiTextValue(spec)),
    el: (_tag: string, _className: string, spec: UiText) => ({ get text() { return uiTextValue(spec); }, attributes: {} as Record<string, string>, children: [] as any[], removed: false, setAttribute(key: string, value: string) { this.attributes[key] = value; }, append(child: any) { this.children.push(child); }, remove() { this.removed = true; } }),
    button: (_text: string, action: () => void) => ({ action, setAttribute() {} }),
    toastHost: { append(item: any) { errors.push(item); } },
    api: async (path: string, body: { sessionId: string }) => { calls.push({ path, body }); return { released: true, sessionId: body.sessionId }; },
    navigationGeneration: 0,
    inventory: null,
    workspaceTools: { assertLibraryChange() {} },
    prepareSwitch: async () => undefined,
    readOnlySource: () => false,
    shell: { classList: { remove() {} } },
  };
  vm.createContext(context); vm.runInContext(compiled, context);
  return { context, calls, errors };
}
test("an adopted document cannot be released as a temporary OpenResult", async () => {
  const f = fixture();
  await assert.rejects(f.context.releaseOpenedSession("adopted-original"), (error: unknown) => error instanceof UiError && error.message === uiTextValue(message("desktop.sessionAlreadyAdopted")));
  assert.equal(f.calls.length, 0);
  assert.equal(f.context.documents[0].session.opened.sessionId, "adopted-original");
});

test("native close waits through the actual open-result adoption caller", async () => {
  const f = fixture();
  let continueSwitch!: () => void;
  f.context.prepareSwitch = () => new Promise<void>(resolve => { continueSwitch = resolve; });
  f.context.addDocument = (opened: any) => { f.context.documents.push({ session: { opened } }); };
  const adoption = f.context.acceptResolvedDocument({ sessionId: "newly-delivered", source: "body", baseId: "base", path: "note.typ", root: "library" });
  assert.throws(() => f.context.libraryCloseActivity.checkpoint());
  continueSwitch();
  await adoption;
  assert.equal(f.context.ownsOpenedSession("newly-delivered"), true);
  assert.equal(f.context.libraryCloseActivity.freeze(f.context.libraryCloseActivity.checkpoint()), true);
  assert.equal(f.calls.length, 0);
});
test("a comparison target is also owned by the live document and cannot be released", async () => {
  const f=fixture();
  f.context.documents[0].comparisonTarget={sessionId:"comparison-open-result"};
  await assert.rejects(f.context.releaseOpenedSession("comparison-open-result"), (error: unknown) => error instanceof UiError && error.message === uiTextValue(message("desktop.sessionAlreadyAdopted")));
  assert.equal(f.calls.length,0);
});
test("late temporary cleanup keeps the newly selected document and sends only the original ID", async () => {
  const f = fixture();
  f.context.documents = [{ session: { opened: { sessionId: "new-current-document" } } }];
  await f.context.releaseOpenedSession("late-unadopted-result");
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].path, "/api/session/release");
  assert.equal(f.calls[0].body.sessionId, "late-unadopted-result");
  assert.equal(f.context.documents[0].session.opened.sessionId, "new-current-document");
  assert.equal(f.errors.length, 0);
});
test("a failed release remains visible in the host after its requesting panel is gone", async () => {
  const f = fixture();
  let reject!: (error: Error) => void;
  f.context.api = () => new Promise((_resolve, failed) => { reject = failed; });
  const pending = f.context.releaseOpenedSession("original-unadopted-session");
  f.context.documents = [];
  reject(new Error("transport effect unknown"));
  await assert.rejects(pending, /effect unknown/);
  assert.equal(f.errors.length, 1);
  assert.equal(f.errors[0].attributes.role, "alert");
  assert.match(f.errors[0].text, /original-unadopted-session/);
  assert.equal(f.errors[0].removed, false);
  f.errors[0].children[0].action();
  assert.equal(f.errors[0].removed, true);
});
test("a mismatched acknowledgement cannot manufacture a successful session release", async () => {
  const f = fixture();
  f.context.api = async () => ({ released: true, sessionId: "other-session" });
  await assert.rejects(f.context.releaseOpenedSession("original-session"), (error: unknown) => error instanceof UiError && error.message === uiTextValue(message("desktop.sessionReleaseFailed", {sessionId: "original-session"})));
  assert.equal(f.errors.length, 1);
  assert.match(f.errors[0].text, /original-session/);
});
test("the main owner releases its fresh result after a later navigation wins during prepare", async () => {
  const f = fixture(); let resume!: () => void;
  f.context.prepareSwitch = () => new Promise<void>(resolve => { resume = resolve; });
  const pending = f.context.acceptResolvedDocument({sessionId: "late-open", source: "S"});
  ++f.context.navigationGeneration;
  resume(); await pending;
  assert.deepEqual(f.calls.map(call => call.body.sessionId), ["late-open"]);
  assert.equal(f.context.documents[0].session.opened.sessionId, "adopted-original");
});
test("the main owner cannot adopt a late result after the existing writer retired during settlement", async () => {
  const f = fixture(); let resume!: () => void, entered!: () => void;
  const settling = new Promise<void>(resolve => { entered = resolve; });
  const existing = {session: {opened: {sessionId: "old-session", nodeId: "node", workspaceId: "workspace"}, source: "author Y"}, writing: {settleForSessionChange: () => new Promise<void>(resolve => {resume = resolve; entered();})}};
  f.context.documents = [existing];
  const pending = f.context.acceptResolvedDocument({sessionId: "fresh-result", nodeId: "node", workspaceId: "workspace", source: "S"});
  await settling;
  f.context.documents = []; resume(); await pending;
  assert.deepEqual(f.calls.map(call => call.body.sessionId), ["fresh-result"]);
  assert.equal(existing.session.opened.sessionId, "old-session");
  assert.equal(existing.session.source, "author Y");
});

const bodyCloseNames = new Set(["bodyCloseFrozen", "bodyOwners", "preparedBody", "frozenBodyRenders", "bodyCloseParticipant"]);
const bodyClosePorts = source.statements.filter(node =>
  ts.isFunctionDeclaration(node) && ["bodySnapshot", "currentBody", "thawBodyClose"].includes(node.name?.text ?? "") ||
  ts.isVariableStatement(node) && node.declarationList.declarations.some(declaration => ts.isIdentifier(declaration.name) && bodyCloseNames.has(declaration.name.text)));
assert.equal(bodyClosePorts.length, 8);
const compiledBodyClose = ts.transpileModule(bodyClosePorts.map(node => node.getText(source).replace(/^export\s+/, "")).join("\n"), {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
}).outputText;

function bodyCloseFixture(pauseAt?: "digest" | "document-flush") {
  const original = "accepted body", later = "later staged literal";
  let resume!: () => void, reached!: () => void;
  const paused = new Promise<void>(resolve => { resume = resolve; });
  const entered = new Promise<void>(resolve => { reached = resolve; });
  const pause = async (point: typeof pauseAt): Promise<void> => {
    if (point === pauseAt) { reached(); await paused; }
  };
  let editorFreezes = 0;
  const editor: WritingBuffer & { freezeForClose(): boolean; thawAfterClose(): void } = {
    source: original, generation: 0, selection: { from: 0, to: 0 }, composing: false,
    observe: () => () => {}, select() {}, undo() {}, redo() {},
    applyPatches() { throw new Error("The staged literal must not be installed by the close regression"); },
    freezeForClose() { editorFreezes++; return true; }, thawAfterClose() {},
  };
  const session = {
    opened: { sessionId: "body-session", workspaceId: "workspace", nodeId: "node" },
    baseId: "base", source: original, serial: 3, path: "note.typ", savePending: false,
    lastDraftSource: original, lastDraftSerial: 3,
  };
  const writing = new WritingController({
    editor, basis: () => ({ sessionId: session.opened.sessionId, source: session.source, serial: session.serial }),
    editable: () => true, debounceMs: 60000,
    request: async () => { throw new Error("Pending literal input was unexpectedly submitted"); },
  });
  const sourceHash = "accepted-source-hash", role = () => ({ status: "absent", text: null, range: null, construct: null, encoding: null });
  const map: WritingMap = {
    sourceHash, syntaxValid: true,
    literalRegions: [{ range: writingRange(original, 0, original.length), text: original, role: "body", paragraphBreaks: [] }],
    structures: [], title: role(), subtitle: role(), bodyInsertion: writingRange(original, original.length, original.length), unavailableReason: null,
    notes: { sourceHash, syntaxValid: true, entries: [] },
    headingNumbering: { sourceHash, settings: null, editable: true, frozenEndnotesRequired: false, reason: null }, bodyBlocks: [],
  };
  assert.equal(writing.installMap(map, original, session.serial), true);
  const doc = {
    session, editor, writing, mode: "source",
    queries: { hasUnappliedAuthorInputs: false, freezeForClose: () => true, thawAfterClose() {} },
    authoringAssets: { assertCloseReady() {}, freezeForClose: () => true, thawAfterClose() {} },
  };
  const context: any = {
    documents: [doc], uiMessage: message, UiError, TextEncoder, clearTimeout,
    crypto: { randomUUID: () => "owner-generation", subtle: { digest: async () => { await pause("digest"); return new Uint8Array(32).fill(0xab).buffer; } } },
    persistDraft: async () => { session.lastDraftSource = session.source; session.lastDraftSerial = session.serial; },
    api: async (path: string, body: { sessionId: string; baseId: string; sourceDigest: string }) => {
      assert.equal(path, "/api/desktop/document-flush");
      await pause("document-flush");
      return { ...body, evidence:"authorDraft", recordDigest: "cd".repeat(32) };
    },
    readOnlySource: () => false, scheduleRender() {},
  };
  vm.createContext(context);
  vm.runInContext(`${compiledBodyClose}\nglobalThis.bodyCloseParticipant = bodyCloseParticipant;`, context);
  const stage = () => {
    writing.stageLiteral(map.literalRegions[0].range, original, later);
    assert.equal(writing.state.pending, true);
    assert.equal(writing.state.busy, false);
    assert.equal(writing.state.composing, false);
    assert.equal(session.source, original);
    assert.equal(editor.source, original);
    assert.equal(session.serial, 3);
    assert.equal(editor.generation, 0);
  };
  return { context, doc, writing, later, entered, resume, stage, editorFreezes: () => editorFreezes };
}

test("body close preparation rejects a staged literal arriving during digest or document flush without a source revision", async t => {
  for (const point of ["digest", "document-flush"] as const) {
    await t.test(point, async t => {
      const f = bodyCloseFixture(point); t.after(() => { f.resume(); f.writing.dispose(); });
      const preparing = f.context.bodyCloseParticipant.prepare({ closeAttemptId: point });
      await Promise.race([f.entered, preparing.then(() => { throw new Error("Close preparation skipped its controlled async boundary"); })]);
      f.stage(); f.resume();
      await assert.rejects(preparing, (error: unknown) => error instanceof UiError);
      assert.equal(f.writing.pendingText, f.later);
      assert.equal(f.editorFreezes(), 0);
    });
  }
});

test("body close freeze rejects staged literal input after preparation even when the accepted source basis is unchanged", async t => {
  const f = bodyCloseFixture(); t.after(() => f.writing.dispose());
  const request = { closeAttemptId: "after-prepared" };
  const proofs = await f.context.bodyCloseParticipant.prepare(request);
  assert.equal(proofs.length, 1);
  const snapshot = f.context.bodySnapshot(f.doc);
  assert.equal(f.context.currentBody(snapshot), true);
  f.stage();
  assert.equal(f.context.currentBody(snapshot), false);
  assert.equal(f.context.bodyCloseParticipant.freeze(request), false);
  assert.equal(f.writing.pendingText, f.later);
  assert.equal(f.editorFreezes(), 0);
});

test("comparison release settles the same handle in every writer retirement set",async()=>{
 const f=fixture();for(const doc of f.context.documents)doc.retiringSessions=new Set(["former-writer"]);
 await f.context.releaseOpenedSession("former-writer");assert.equal(f.context.documents[0].retiringSessions.size,0);assert.equal(f.calls.length,1);
});
