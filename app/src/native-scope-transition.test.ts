import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { QueryPanel } from "./query-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { LibraryCloseActivity } from "./library-native-close";
import { message, UiError, errorUiText, uiTextValue, displayMessage, uiDisplayParameter } from "./ui-locale";
import { ApiError } from "./api-error";
import { DocumentSession, rebindCommittedSave, canRebindSaveAs, uniqueRetainedDrafts } from "./session";
import { restoreBufferDisposition } from "./history-state";
import { fileSaveReceipt, fileSaveFailure, type FileSaveReceipt } from "./file-save-result";
import { originalCommitted, submitOriginal, TerminatedAttempt } from "./original-operation";

// Exercise the actual main consumers and WorkspaceTools methods. Substitute
// transport, rendered DOM and editor construction, not transition decisions.
const main = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = ["chooseLibrary", "createLibrary", "restoreNavigation", "activateDocument", "renderSidebar", "addDocument", "ownsOpenedSession", "retireUnownedSessions", "retireHandedOffWriters", "setComparisonTarget", "saveDocument", "pendingSave", "historyContext", "showDraftComparison"];
const functions = main.statements.filter(node => ts.isFunctionDeclaration(node) && names.includes(node.name?.text ?? ""));
assert.equal(functions.length, names.length);
const workspaceSource = ts.createSourceFile("workspace-tools.ts", readFileSync(new URL("./workspace-tools.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const workspaceClass = workspaceSource.statements.find(node => ts.isClassDeclaration(node) && node.name?.text === "WorkspaceTools") as ts.ClassDeclaration;
assert.ok(workspaceClass);
const methods = workspaceClass.members.filter(node => ts.isMethodDeclaration(node) && ["assertLibraryChange", "assertLibraryDeparture", "setLibrary"].includes(node.name.getText(workspaceSource)));
assert.equal(methods.length, 3);
const compiled = ts.transpileModule(`${functions.map(node => node.getText(main)).join("\n")}\nclass WorkspaceScopePort { ${methods.map(node => node.getText(workspaceSource)).join("\n")} }\nglobalThis.WorkspaceScopePort = WorkspaceScopePort;`, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
}).outputText;
const creationConsumers = ts.transpileModule(main.statements.filter(node => ts.isFunctionDeclaration(node) && ["createNote", "acceptCreatedDocument"].includes(node.name?.text ?? "")).map(node => node.getText(main)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function consumerInitializer(declaration: string, name: string): string {
  const variable = main.statements.filter(ts.isVariableStatement).flatMap(node => [...node.declarationList.declarations]).find(node => node.name.getText(main) === declaration);
  assert.ok(variable?.initializer && ts.isNewExpression(variable.initializer));
  const options = variable.initializer.arguments?.[0]; assert.ok(options && ts.isObjectLiteralExpression(options));
  const property = options.properties.find(node => ts.isPropertyAssignment(node) && node.name.getText(main) === name);
  assert.ok(property && ts.isPropertyAssignment(property));
  return property.initializer.getText(main);
}
const deliveryConsumers = ts.transpileModule(`globalThis.historyRestored = ${consumerInitializer("historyPanel", "restored")}; globalThis.structuralDelivery = ${consumerInitializer("workspaceTools", "applyReceipt")}; globalThis.queryDelivery = ${consumerInitializer("workspaceTools", "savedDefinition")};`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

function fixture() {
  const original = { root: "C:/original", workspaceId: "original-workspace", entries: [] as any[] };
  const next = { root: "C:/next", workspaceId: "next-workspace", entries: [] as any[] };
  const effects: string[] = [], released: string[] = [];
  const query: any = Object.create(QueryPanel.prototype);
  Object.assign(query, {
    closeGuard: new ProposalCloseGuard(), draftFlags: new Set<string>(), definitionName: "",
    root: original.root, workspaceId: original.workspaceId,
    directory: null, scopeIdentity: null, navigationDirectory: null, navigationScopeIdentity: null,
    environmentEpoch: 0, generation: 0, captureDialog: null, captureDialogs: new Map(),
    refreshQueue: { invalidate() { effects.push("query-invalidate"); } },
    loadPreferences() {}, renderControls() {}, renderResults() {}, refresh: async () => {},
  });
  const current = { session: { opened: { sessionId: "current-original", root: original.root } } };
  const context: any = {
    libraryCloseActivity: new LibraryCloseActivity(), uiMessage: message, UiError, displayMessage, uiDisplayParameter, uniqueRetainedDrafts, restoreBufferDisposition,
    inventory: original, query: "original filter", selectedFolder: "C:/original/folder",
    collapsed: new Set(["original-collapsed"]), contentSelections: new Set(["original-selection"]),
    navigationContexts: new Map(), navigationGeneration: 0, documents: [current], current,
    activeSourceOrigin: "original-origin", bodyCloseFrozen: false, sidebarComposing: false,
    pathDialog: async () => next.root, flushLiteral: async () => {}, prepareSwitch: async () => {},
    api: async () => next,
    normalize: (path: string) => path.replaceAll("\\", "/").toLowerCase(),
    readOnlySource: () => false, fileSaveFailure, flowScroll: null, flowHasProjection: false,
    persistDraft: async () => { effects.push("persist-draft"); },
    releaseOpenedSession: async (id: string) => { released.push(id); },
    renderContent: () => { effects.push("content"); }, scheduleRender: () => { effects.push("schedule"); },
    toast: () => { effects.push("toast"); }, pendingCreation: null,
    acceptCreatedDocument: async () => { effects.push("adopt-created"); },
    refreshInventory: async () => { effects.push("refresh-inventory"); },
    nodeContents: { dispose() { effects.push("dispose-node-contents"); } },
    navigationLayout: {
      captureSidebarState() { effects.push("capture-layout"); },
      setContext() { effects.push("set-layout"); },
    },
    DocumentSession: class { constructor() { effects.push("construct-session"); throw new Error("Unexpected editor allocation"); } },
    structuredClone,
  };
  vm.createContext(context); vm.runInContext(compiled, context);
  const workspace: any = new context.WorkspaceScopePort();
  Object.assign(workspace, {
    closeGuard: new ProposalCloseGuard(), queryPanel: query, library: original,
    invalidateQueryEnvironment() { effects.push("invalidate-environment"); },
  });
  context.workspaceTools = workspace;
  const actualRenderSidebar = context.renderSidebar;
  context.renderSidebar = () => { workspace.setLibrary(context.inventory, context.selectedFolder); effects.push("sidebar"); };
  const savedMain = () => ({
    inventory: context.inventory, current: context.current, query: context.query, folder: context.selectedFolder,
    collapsed: [...context.collapsed], selection: [...context.contentSelections], origin: context.activeSourceOrigin,
  });
  const assertOriginal = (before: ReturnType<typeof savedMain>) => {
    assert.equal(context.inventory, before.inventory); assert.equal(context.current, before.current);
    assert.equal(context.query, before.query); assert.equal(context.selectedFolder, before.folder);
    assert.deepEqual([...context.collapsed], before.collapsed); assert.deepEqual([...context.contentSelections], before.selection);
    assert.equal(context.activeSourceOrigin, before.origin);
    assert.equal(workspace.library, original); assert.equal(query.root, original.root);
  };
  const target = () => {
    const prior = { sessionId: "target-original", root: next.root, workspaceId: next.workspaceId, nodeId: "node", source: "base", path: "C:/next/note.typ" };
    const doc = {
      session: { opened: prior, baseSource: "base", acknowledgeSave() { effects.push("acknowledge-save"); } },
      writing: { settleForSessionChange: async () => {}, restoreNavigationFocus() { effects.push("restore-focus"); } },
      editor: { clearTokens() { effects.push("clear-tokens"); } }, queries: { invalidate() { effects.push("invalidate-document"); } },
      mode: "source", scroll: {}, sourceTrashed: false,
    };
    const saved = { inventory: next, query: "target-filter", folder: null, collapsed: ["target-collapsed"], scroll: 9, contentScroll: 7, selection: ["target-selection"] };
    context.documents.push(doc); context.navigationContexts.set(doc, saved);
    return { doc, saved, prior };
  };
  return { context, query, workspace, original, next, effects, released, actualRenderSidebar, savedMain, assertOriginal, target, creationConsumers: () => vm.runInContext(creationConsumers, context), deliveryConsumers: () => vm.runInContext(deliveryConsumers, context) };
}

test("chooseLibrary rejects a different library before publishing main state or touching panels", async () => {
  const f = fixture(), before = f.savedMain(); f.query.draftFlags.add("advanced");
  await assert.rejects(f.context.chooseLibrary(), (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.deepEqual(f.effects, []); assert.equal(f.query.draftFlags.has("advanced"), true);
});

test("chooseLibrary rechecks the original Query owner after a deferred inventory response", async () => {
  const f = fixture(), before = f.savedMain(), response = deferred<any>(), entered = deferred<void>();
  f.context.api = async () => { entered.resolve(); return response.promise; };
  const switching = f.context.chooseLibrary(); await entered.promise;
  f.query.definitionName = "later unsaved definition"; response.resolve(f.next);
  await assert.rejects(switching, (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.deepEqual(f.effects, []); assert.equal(f.query.definitionName, "later unsaved definition");
});

test("an admitted library change publishes the same library to main, WorkspaceTools and Query", async () => {
  const f = fixture(); await f.context.chooseLibrary();
  assert.equal(f.context.inventory, f.next); assert.equal(f.workspace.library, f.next);
  assert.equal(f.query.root, f.next.root); assert.equal(f.query.workspaceId, f.next.workspaceId);
  assert.equal(f.context.selectedFolder, null); assert.equal(f.context.query, ""); assert.equal(f.context.collapsed.size, 0);
  assert.equal(f.effects.filter(effect => effect === "sidebar").length, 1);
});

test("createLibrary delivery rejection leaves the exact committed receipt available and performs no invalidation or adoption", async () => {
  const f = fixture(), before = f.savedMain();
  const receipt = { operationId: "original-create", library: f.next, workspaceId: f.next.workspaceId, root: f.next.root, rootNotePath: "C:/next/root.typ", rootDocument: { sessionId: "created-root" } };
  const serialized = JSON.stringify(receipt); let delivered: any;
  f.context.openLibraryCreation = async (options: any) => {
    f.query.draftFlags.add("parameters"); delivered = receipt; await options.deliver(receipt);
  };
  await assert.rejects(f.context.createLibrary(), (error: unknown) => error instanceof UiError);
  assert.equal(delivered, receipt); assert.equal(JSON.stringify(receipt), serialized);
  f.assertOriginal(before); assert.deepEqual(f.effects, []);
});

test("createLibrary refuses entry while the original Query proposal is still unresolved", async () => {
  const f = fixture(), before = f.savedMain(); f.query.draftFlags.add("advanced");
  let opened = false;
  f.context.openLibraryCreation = async () => { opened = true; };
  await assert.rejects(f.context.createLibrary(), (error: unknown) => error instanceof UiError);
  assert.equal(opened, false); f.assertOriginal(before); assert.deepEqual(f.effects, []);
  assert.equal(f.query.draftFlags.has("advanced"), true);
});

test("departure guards a transient Query proposal even when no library or current document is selected", async () => {
  const f = fixture();
  f.context.inventory = f.context.current = null; f.context.documents = [];
  f.workspace.library = null; f.query.root = null; f.query.workspaceId = undefined;
  f.query.definitionName = "unapplied definition without a selected library";
  let opened = false;
  f.context.openLibraryCreation = async () => { opened = true; };
  assert.doesNotThrow(() => f.workspace.assertLibraryChange(null));
  assert.throws(() => f.workspace.assertLibraryDeparture(), (error: unknown) => error instanceof UiError);
  await assert.rejects(f.context.createLibrary(), (error: unknown) => error instanceof UiError);
  assert.equal(opened, false); assert.equal(f.context.current, null); assert.equal(f.context.inventory, null);
  assert.equal(f.workspace.library, null); assert.equal(f.query.root, null);
  assert.equal(f.query.definitionName, "unapplied definition without a selected library"); assert.deepEqual(f.effects, []);
});

test("restoreNavigation rejects before replacing inventory, folder, filter or selected items", () => {
  const f = fixture(), before = f.savedMain(), { doc, saved } = f.target();
  const serialized = JSON.stringify(saved); f.query.draftFlags.add("filter");
  assert.throws(() => f.context.restoreNavigation(doc), (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.equal(f.context.navigationContexts.get(doc), saved); assert.equal(JSON.stringify(saved), serialized);
});

test("activateDocument refuses a blocked saved library before changing the active document or its original Session", async () => {
  const f = fixture(), before = f.savedMain(), { doc, saved, prior } = f.target();
  const serialized = JSON.stringify(saved); f.query.draftFlags.add("advanced");
  await assert.rejects(f.context.activateDocument(doc), (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.equal(doc.session.opened, prior);
  assert.equal(f.context.navigationContexts.get(doc), saved); assert.equal(JSON.stringify(saved), serialized);
  assert.deepEqual(f.effects, []); assert.deepEqual(f.released, []);
});

test("activateDocument rechecks deferred inventory without adopting an unrelated Session", async () => {
  const f = fixture(), before = f.savedMain(), { doc, saved, prior } = f.target();
  const serialized = JSON.stringify(saved), response = deferred<any>(), entered = deferred<void>();
  const fresh = { ...prior, sessionId: "fresh-unadopted", baseId: "new-base" };
  f.context.api = async (route: string) => {
    if (route === "/api/open") return { ...fresh, sessionId: prior.sessionId };
    assert.equal(route, "/api/library/open"); entered.resolve(); return response.promise;
  };
  const switching = f.context.activateDocument(doc); await entered.promise;
  f.query.draftFlags.add("advanced"); response.resolve({ ...f.next, entries: [] });
  await assert.rejects(switching, (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.equal(doc.session.opened, prior);
  assert.equal(f.context.navigationContexts.get(doc), saved); assert.equal(JSON.stringify(saved), serialized);
  assert.deepEqual(f.effects, []); assert.deepEqual(f.released, []);
});

test("renderSidebar rejects a context mismatch before disposing the current contents or changing navigation layout", () => {
  const f = fixture(), originalContents = f.context.nodeContents;
  f.context.inventory = f.next; f.query.draftFlags.add("advanced");
  assert.throws(() => f.actualRenderSidebar(), (error: unknown) => error instanceof UiError);
  assert.deepEqual(f.effects, []); assert.equal(f.context.nodeContents, originalContents);
  assert.equal(f.workspace.library, f.original); assert.equal(f.query.root, f.original.root);
});

test("addDocument rejects before allocating an editor or adopting the incoming Session", () => {
  const f = fixture(), originalCurrent = f.context.current;
  f.context.inventory = f.next; f.query.draftFlags.add("advanced");
  assert.throws(() => f.context.addDocument({ sessionId: "unadopted" }), (error: unknown) => error instanceof UiError);
  assert.deepEqual(f.effects, []); assert.equal(f.context.current, originalCurrent); assert.equal(f.context.documents.length, 1);
});

test("the actual WorkspaceTools setter rejects a frozen Query even while its own guard is mutable", () => {
  const f = fixture(); assert.equal(f.query.closeGuard.freeze(false), true);
  assert.doesNotThrow(() => f.workspace.closeGuard.assertMutable());
  assert.throws(() => f.workspace.assertLibraryChange(f.next), (error: unknown) => error instanceof UiError);
  assert.throws(() => f.workspace.setLibrary(f.next), (error: unknown) => error instanceof UiError);
  assert.equal(f.workspace.library, f.original); assert.equal(f.query.root, f.original.root); assert.deepEqual(f.effects, []);
});

test("activateDocument retains the exact original Session when navigation publication is blocked after persistence", async () => {
  const f = fixture(), before = f.savedMain(), { doc, saved, prior } = f.target();
  const serialized = JSON.stringify(saved), persistence = deferred<void>(), entered = deferred<void>();
  const fresh = { ...prior, baseId: "new-base" };
  f.context.api = async (route: string) => route === "/api/open" ? fresh : { ...f.next, entries: [] };
  f.context.sidebar = { querySelector: () => null };
  Object.assign(doc.editor, { view: { focus() {} } });
  f.context.persistDraft = async (owner: any) => {
    assert.equal(owner, doc); assert.equal(owner.session.opened, fresh);
    entered.resolve(); await persistence.promise;
  };
  const switching = f.context.activateDocument(doc); await entered.promise;
  assert.equal(f.context.current, before.current);
  assert.equal(f.context.navigationContexts.get(doc), saved);
  f.query.draftFlags.add("advanced"); persistence.resolve();
  await assert.rejects(switching, (error: unknown) => error instanceof UiError);
  f.assertOriginal(before); assert.equal(doc.session.opened, fresh);
  assert.equal(f.context.ownsOpenedSession(fresh.sessionId), true);
  assert.equal(f.context.navigationContexts.get(doc), saved); assert.equal(JSON.stringify(saved), serialized);
  assert.deepEqual(f.released, []);
  assert.deepEqual(f.effects, ["acknowledge-save", "clear-tokens", "invalidate-document"]);
});

test("saveDocument retains its exact committed receipt and adopted Session when refreshed library identity is blocked", async () => {
  const f = fixture();
  const prior = { sessionId: "save-writer", root: f.original.root, workspaceId: f.original.workspaceId, nodeId: "node", source: "before", baseId: "old-base", path: `${f.original.root}/note.typ`, kind: "managed" };
  const session = new DocumentSession(prior); session.edit("submitted original text");
  const opened = { ...prior, source: session.source, baseId: "fresh-base" };
  const receipt = { operationId: "original-save", status: "committed", path: prior.path, baseId: "committed-base", committedSource: session.source, document: opened, documentSourceMatchesCommit: true };
  const doc: any = {
    session, writing: { state: { composing: false }, settleForSessionChange: async () => {} },
    editor: { clearTokens() { f.effects.push("clear-tokens"); } }, sourceTrashed: false,
  };
  f.context.current = doc; f.context.documents = [doc]; const before = f.savedMain();
  f.query.draftFlags.add("advanced");
  // The stub must model an actual Core input ACK before the save may freeze
  // S; an unacknowledged source cannot gain author-commit eligibility.
  f.context.persistDraft = async (owner: any) => {
    const revision = owner.session.issueDraftRevision();
    owner.session.acknowledgeDraft(owner.session.source, owner.session.serial, revision);
  };
  let request: any, validated: FileSaveReceipt | undefined;
  const routes: string[] = [];
  Object.assign(f.context, {
    historyPanel: { hasPending: () => false }, historyContext: () => ({}), refreshChrome() {},
    operationId: () => receipt.operationId, originalCommitted, submitOriginal, TerminatedAttempt,
    ApiError, errorUiText, rebindCommittedSave,
    fileSaveReceipt(value: unknown, frozen: any) { validated = fileSaveReceipt(value, frozen); return validated; },
    api: async (route: string, payload: any) => {
      routes.push(route);
      if (route === "/api/save") {
        request = doc.lastSave;
        assert.equal(payload.operationId, receipt.operationId); assert.equal(payload.sessionId, prior.sessionId);
        assert.equal(payload.source, receipt.committedSource);
        return receipt;
      }
      assert.equal(route, "/api/library/open");
      assert.equal(payload.path, f.original.root);
      assert.equal(doc.session.opened, opened);
      return { ...f.original, workspaceId: "replaced-workspace-at-same-root" };
    },
  });
  await assert.rejects(f.context.saveDocument(doc), (error: unknown) => error instanceof UiError);
  f.assertOriginal(before);
  assert.ok(request); assert.ok(validated); assert.equal(doc.lastSave, request);
  assert.equal(request.receipt, validated); assert.deepEqual(validated, receipt);
  assert.equal(request.operationId, receipt.operationId); assert.equal(request.deliverySessionId, opened.sessionId);
  assert.equal(doc.session.opened, opened); assert.equal(f.context.ownsOpenedSession(opened.sessionId), true);
  assert.equal(doc.session.savePending, false); assert.deepEqual(f.released, []);
  assert.deepEqual(routes, ["/api/save", "/api/library/open"]);
  assert.equal(f.effects.includes("sidebar"), false);
});

test("saveDocument clears a resolved original attempt while retaining its typed rejection and exact author input", async () => {
  const f = fixture();
  const prior = { sessionId: "save-writer", root: f.original.root, workspaceId: f.original.workspaceId, nodeId: "node", source: "before", baseId: "old-base", path: `${f.original.root}/note.typ`, kind: "managed" };
  const session = new DocumentSession(prior); session.edit("retained author input\r\n😀");
  const source = session.source, inputId = session.inputId;
  const doc: any = { session, writing: { state: { composing: false } }, sourceTrashed: false };
  const rejection = new ApiError("invalidAuthorSource", message("error.invalidResponse"));
  const routes: string[] = [];
  f.context.current = doc; f.context.documents = [doc];
  Object.assign(f.context, {
    historyPanel: { hasPending: () => false }, historyContext: () => ({}), refreshChrome() {},
    operationId: () => "resolved-save", originalCommitted, submitOriginal, TerminatedAttempt, ApiError, errorUiText, fileSaveReceipt,
    persistDraft: async () => { const revision = session.issueDraftRevision(); session.acknowledgeDraft(session.source, session.serial, revision); },
    api: async (route: string, payload: any) => {
      routes.push(route); assert.equal(payload.operationId, "resolved-save");
      if (route === "/api/save") { assert.equal(payload.source, source); throw rejection; }
      if (route === "/api/operations/status") return { operationId: "resolved-save", status: "unknown" };
      assert.equal(route, "/api/operations/settle"); return { operationId: "resolved-save", status: "resolved" };
    },
  });
  await assert.rejects(f.context.saveDocument(doc), error => error instanceof TerminatedAttempt);
  assert.deepEqual(routes, ["/api/save", "/api/operations/status", "/api/operations/settle"]);
  assert.equal(doc.lastSave, undefined); assert.equal(session.savePending, false);
  assert.equal(uiTextValue(session.error!), uiTextValue(errorUiText(rejection)));
  assert.notEqual(uiTextValue(session.error!), uiTextValue(message("shell.saveUnknownConfirmSameOperation")));
  assert.equal(session.source, source); assert.equal(session.inputId, inputId); assert.equal(session.baseId, prior.baseId);
  assert.equal(session.opened, prior); assert.deepEqual(f.released, []);
});

test("activateDocument publishes an admitted context without inventing another writer producer", async () => {
  const f = fixture(), { doc, saved, prior } = f.target(), serialized = JSON.stringify(saved);
  const fresh = { ...prior, baseId: "fresh-base" };
  const latest = { ...f.next, entries: [] };
  f.context.api = async (route: string) => route === "/api/open" ? fresh : latest;
  f.context.sidebar = { querySelector: () => null };
  Object.assign(doc.editor, { view: { focus() { f.effects.push("focus"); } } });
  await f.context.activateDocument(doc);
  assert.equal(f.context.current, doc); assert.equal(f.context.inventory, latest);
  assert.equal(f.workspace.library, latest); assert.equal(f.query.root, latest.root);
  assert.equal(f.context.query, saved.query); assert.equal(f.context.selectedFolder, saved.folder);
  assert.notEqual(f.context.navigationContexts.get(doc), saved); assert.equal(JSON.stringify(saved), serialized);
  assert.equal(f.context.navigationContexts.get(doc).inventory, latest);
  assert.equal(doc.session.opened, fresh); assert.equal(f.context.ownsOpenedSession(fresh.sessionId), true);
  assert.deepEqual(f.released, []); assert.equal(f.effects.includes("focus"), true);
});

test("replacing a comparison retires only unowned result Sessions and keeps the writer", async () => {
  const f = fixture(), { doc, prior } = f.target();
  const first = { ...prior, sessionId: "old-comparison", source: "external-before" };
  Object.assign(doc, { comparisonTarget: first });
  let next = { ...prior, sessionId: "comparison-1", source: "external-current" };
  f.context.api = async (route: string) => route === "/api/open" ? next : f.next;
  f.context.sidebar = { querySelector: () => null };
  Object.assign(doc.editor, { view: { focus() {} } });
  await f.context.activateDocument(doc);
  assert.deepEqual(f.released, [first.sessionId]);
  next = { ...next, sessionId: "comparison-2" };
  f.context.current = f.context.documents[0];
  await f.context.activateDocument(doc);
  assert.deepEqual(f.released, [first.sessionId, "comparison-1"]);
  assert.equal(doc.session.opened, prior);
  assert.equal(f.context.ownsOpenedSession(prior.sessionId), true);
  assert.equal(f.context.ownsOpenedSession(next.sessionId), true);
});

test("SaveAs retires the original writer only after newer target input is acknowledged", async () => {
  const f = fixture(), savedReply = deferred<any>(), sent = deferred<void>(), targetAck = deferred<void>(), atTargetAck = deferred<void>();
  const prior = { sessionId: "original-writer", root: f.original.root, source: "B", baseId: "base-B", path: `${f.original.root}/ordinary.typ`, kind: "ordinary" };
  const session = new DocumentSession(prior); session.edit("S");
  const opened = { ...prior, sessionId: "target-writer", source: "S", baseId: "target-base", path: `${f.original.root}/copy.typ` };
  const receipt = { operationId: "save-as-original", status: "saved", path: opened.path, baseId: "committed-base", committedSource: "S", document: opened, documentSourceMatchesCommit: true };
  const doc: any = { session, writing: { state: { composing: false }, settleForSessionChange: async () => {} }, editor: { clearTokens() {} }, sourceTrashed: false };
  f.context.current = doc; f.context.documents = [doc];
  Object.assign(f.context, {
    historyPanel: { hasPending: () => false }, historyContext: () => ({}), refreshChrome() {}, renderSidebar() {},
    operationId: () => receipt.operationId, originalCommitted, submitOriginal, TerminatedAttempt, ApiError, errorUiText,
    rebindCommittedSave, canRebindSaveAs, fileSaveReceipt,
    persistDraft: async (owner: any) => {
      if (owner.session.opened.sessionId === opened.sessionId) { atTargetAck.resolve(); await targetAck.promise; }
      const revision = owner.session.issueDraftRevision(); owner.session.acknowledgeDraft(owner.session.source, owner.session.serial, revision);
    },
    api: async (route: string) => {
      if (route === "/api/save-as") { sent.resolve(); return savedReply.promise; }
      assert.equal(route, "/api/library/open"); return f.original;
    },
  });
  const saving = f.context.saveDocument(doc, opened.path);
  await sent.promise; session.edit("Y"); savedReply.resolve(receipt);
  await atTargetAck.promise;
  assert.deepEqual(f.released, []); assert.equal(session.source, "Y");
  targetAck.resolve(); await saving;
  assert.deepEqual(f.released, [prior.sessionId]); assert.equal(session.opened.sessionId, opened.sessionId);
  assert.equal(session.source, "Y"); assert.equal(session.dirty, true); assert.equal(doc.lastSave, undefined);
});

test("SaveAs retries delivery on the acknowledged target writer after inventory fails", async () => {
  const f = fixture(), reply = deferred<any>(), sent = deferred<void>();
  const prior = { sessionId: "original", root: f.original.root, source: "B", baseId: "before", path: `${f.original.root}/ordinary.typ`, kind: "ordinary" };
  const session = new DocumentSession(prior); session.edit("S");
  const target = { ...prior, sessionId: "target-1", source: "S", baseId: "target-base", path: `${f.original.root}/copy.typ` };
  const receipt = { operationId: "save-as", status: "saved", path: target.path, baseId: "original-base", committedSource: "S", document: target, documentSourceMatchesCommit: true };
  const extra = { ...target, sessionId: "target-2" };
  const doc: any = { session, writing: { state: { composing: false }, settleForSessionChange: async () => {} }, editor: { clearTokens() {} }, sourceTrashed: false };
  f.context.current = doc; f.context.documents = [doc];
  const acknowledgements: string[] = [];
  let inventories = 0, authors = 0, lookups = 0, originalRefresh = 0;
  Object.assign(f.context, {
    historyPanel: { hasPending: () => false }, historyContext: () => ({}), refreshChrome() {}, renderSidebar() {},
    operationId: () => receipt.operationId, originalCommitted, submitOriginal, TerminatedAttempt, ApiError, errorUiText, rebindCommittedSave, canRebindSaveAs, fileSaveReceipt,
    persistDraft: async (owner: any) => { acknowledgements.push(owner.session.opened.sessionId); owner.session.acknowledgeDraft(owner.session.source, owner.session.serial, owner.session.issueDraftRevision()); },
    api: async (route: string, body: any) => {
      if (route === "/api/save-as") { authors++; sent.resolve(); return reply.promise; }
      if (route === "/api/operations/status") { lookups++; return { ...receipt, status: "committed", document: extra }; }
      if (route === "/api/open") { assert.equal(body.sessionId, target.sessionId); originalRefresh++; return target; }
      assert.equal(route, "/api/library/open");
      if (++inventories === 1) throw new Error("inventory failed after ACK");
      return f.original;
    },
  });
  const saving = f.context.saveDocument(doc, target.path);
  await sent.promise; session.edit("Y"); reply.resolve(receipt);
  await assert.rejects(saving);
  assert.equal(session.opened.sessionId, target.sessionId); assert.ok(doc.lastSave);
  assert.deepEqual(f.released, [prior.sessionId]);
  await f.context.saveDocument(doc);
  assert.equal(authors, 1); assert.equal(lookups, 1); assert.equal(originalRefresh, 1);
  assert.equal(session.opened.sessionId, target.sessionId); assert.equal(session.source, "Y"); assert.equal(session.dirty, true);
  assert.equal(acknowledgements.includes(extra.sessionId), false);
  assert.deepEqual(f.released, [prior.sessionId, extra.sessionId]); assert.equal(doc.lastSave, undefined);
});

test("a created document retains its adopted writer on original delivery retry", async () => {
  const f = fixture(); f.creationConsumers();
  const opened = { sessionId: "created-1", root: f.original.root, workspaceId: f.original.workspaceId, nodeId: "born", kind: "managed", path: `${f.original.root}/Born/Born.typ`, source: "B", baseId: "base" };
  let additions = 0, inventories = 0;
  f.context.addDocument = (value: any) => { additions++; const doc = { session: new DocumentSession(value) }; f.context.documents.push(doc); f.context.current = doc; };
  f.context.refreshInventory = async () => { if (++inventories === 1) throw new Error("inventory failed after adoption"); };
  await assert.rejects(f.context.acceptCreatedDocument(opened, "original-birth"));
  const doc = f.context.current; doc.session.edit("Y");
  await f.context.acceptCreatedDocument({ ...opened, sessionId: "created-2" }, "original-birth");
  assert.equal(additions, 1); assert.equal(doc.session.opened.sessionId, opened.sessionId); assert.equal(doc.session.source, "Y");
  assert.deepEqual(f.released, ["created-2"]); assert.equal(doc.comparisonTarget, undefined);
  await assert.rejects(f.context.acceptCreatedDocument({ ...opened, sessionId: "unrelated-birth" }, "other-operation"));
  assert.equal(doc.session.opened.sessionId, opened.sessionId); assert.equal(doc.comparisonTarget.sessionId, "unrelated-birth");
});

test("an already flying library switch cannot redirect the new-note dialog target", async () => {
  const f = fixture(); f.creationConsumers();
  const library = deferred<any>(), prompted = deferred<void>(), name = deferred<string>();
  let authors = 0, description = "";
  Object.assign(f.context, {
    pendingCreation: null,
    api: async () => library.promise,
    modal: async (_title: any, text: any) => { description = uiTextValue(text); prompted.resolve(); return name.promise; },
    creation: async () => { authors++; },
  });
  const switching = f.context.chooseLibrary();
  const creating = f.context.createNote(); await prompted.promise;
  assert.ok(description.includes("C:/original/folder"));
  library.resolve(f.next); await switching;
  name.resolve("Chosen for A"); await assert.rejects(creating, (error: unknown) => error instanceof UiError);
  assert.equal(authors, 0); assert.equal(f.context.inventory, f.next);
});

function deliveryFixture() {
  const f = fixture(); f.deliveryConsumers();
  const prior = { sessionId: "writer", root: f.original.root, workspaceId: f.original.workspaceId, nodeId: "node", kind: "managed", path: `${f.original.root}/Note/Note.typ`, source: "S", baseId: "old-base" };
  const session = new DocumentSession(prior);
  const doc: any = { navigationKey: "note-ui", session, mode: "source", sourceTrashed: false,
    writing: { state: { composing: false }, retainInput: false, recoverySource: () => undefined, settleForSessionChange: async () => {} },
    editor: { source: "S", generation: 0, clearTokens() {}, applyCommitted(value: string) { this.source = value; this.generation++; session.edit(value, session.inputId); }, apply(_from: number, _to: number, value: string) { this.source = value; this.generation++; session.edit(value); } },
    queries: { invalidate() {} },
  };
  Object.assign(f.context, { current: doc, documents: [doc], originalCommitted, refreshChrome() {}, refreshInventory: async () => {},
    persistDraft: async () => { session.acknowledgeDraft(session.source, session.serial, session.issueDraftRevision()); },
  });
  f.context.workspaceTools.documentChanged = () => {};
  const basis = f.context.historyContext(doc);
  const comparison = { ...prior, sessionId: "comparison", source: "X", baseId: "comparison-base", recoveredDraftConflict: true };
  const result = { receipt: { operationId: "restore", status: "saved", path: prior.path, baseId: "restore-commit" }, source: "X", protectedDrafts: [] };
  const fact = { ...result.receipt, status: "committed", committedSource: "X", documentSourceMatchesCommit: true, document: comparison };
  return { ...f, doc, prior, session, basis, comparison, result, fact };
}

test("history delivery keeps later Y on its original writer and Base without automatic merge", async () => {
  const f = deliveryFixture(), routes: string[] = [];
  f.session.edit("Y"); f.doc.editor.source = "Y";
  f.context.api = async (route: string) => { routes.push(route); assert.equal(route, "/api/operations/status"); return f.fact; };
  await assert.rejects(f.context.historyRestored(f.result, f.basis), (error: unknown) => error instanceof UiError);
  assert.equal(f.session.opened, f.prior); assert.equal(f.session.source, "Y"); assert.equal(f.session.baseSource, "S"); assert.equal(f.session.baseId, "old-base");
  assert.equal(f.doc.comparisonTarget, f.comparison); assert.deepEqual(routes, ["/api/operations/status"]); assert.deepEqual(f.released, []);
});

test("clean history delivery freshly qualifies the original writer and releases the unused comparison", async () => {
  const f = deliveryFixture(), original = { ...f.prior, source: "X", baseId: "actual-X", recoveredDraftConflict: false };
  const routes: string[] = [];
  f.context.api = async (route: string, body: any) => { routes.push(route); if (route === "/api/operations/status") return f.fact; assert.equal(route, "/api/open"); assert.equal(body.sessionId, f.prior.sessionId); return original; };
  await f.context.historyRestored(f.result, f.basis);
  assert.equal(f.session.opened, original); assert.equal(f.session.source, "X"); assert.equal(f.session.baseId, "actual-X");
  assert.deepEqual(routes, ["/api/operations/status", "/api/open"]); assert.deepEqual(f.released, [f.comparison.sessionId]);
});

test("Y typed during history writer qualification still cannot inherit restored X", async () => {
  const f = deliveryFixture(), response = deferred<any>(), entered = deferred<void>();
  f.context.api = async (route: string) => { if (route === "/api/operations/status") return f.fact; assert.equal(route, "/api/open"); entered.resolve(); return response.promise; };
  const restoring = f.context.historyRestored(f.result, f.basis); await entered.promise;
  f.session.edit("Y"); f.doc.editor.source = "Y";
  response.resolve({ ...f.prior, source: "X", baseId: "actual-X", recoveredDraftConflict: false });
  await assert.rejects(restoring);
  assert.equal(f.session.opened, f.prior); assert.equal(f.session.source, "Y"); assert.equal(f.session.baseId, "old-base"); assert.equal(f.doc.comparisonTarget, f.comparison);
});

test("structural results cannot adopt an independent same-byte Session for a current or child writer", async () => {
  for (const child of [false, true]) {
    const f = deliveryFixture(); f.session.edit("Y"); f.doc.editor.source = "Y";
    const comparison = { ...f.prior, sessionId: "external-stamp-comparison" };
    const receipt = { operationId: "move", status: "committed", ...(child ? { affectedDocuments: [{ priorSessionId: f.prior.sessionId, documentDisposition: "moved", document: comparison }] } : { documentDisposition: "moved", document: comparison }) };
    await assert.rejects(f.context.structuralDelivery(receipt, { sessionId: child ? "parent" : f.prior.sessionId, root: f.prior.root, source: "S", serial: 0 }), (error: unknown) => error instanceof UiError);
    assert.equal(f.session.opened, f.prior); assert.equal(f.session.source, "Y"); assert.equal(f.session.baseId, "old-base"); assert.equal(f.doc.comparisonTarget, comparison);
  }
});

test("unadopted structural child and closed Query results retire their Sessions", async () => {
  const f = deliveryFixture();
  await f.context.structuralDelivery({ operationId: "move", status: "committed", affectedDocuments: [{ priorSessionId: "closed-child", document: { ...f.prior, sessionId: "orphan-child" }, documentDisposition: "moved" }] }, { sessionId: "closed-parent", root: f.prior.root });
  await assert.rejects(f.context.queryDelivery({ document: { ...f.prior, sessionId: "orphan-query", source: "X" }, committedSource: "X", documentSourceMatchesCommit: true }, { sessionId: "closed-query", root: f.prior.root }));
  assert.deepEqual(f.released, ["orphan-child", "orphan-query"]);
});

test("comparison cancellation releases the fresh result and retains the current writer and comparison", async () => {
  const f = deliveryFixture(), fresh = { ...f.comparison, sessionId: "fresh-comparison" };
  f.doc.comparisonTarget = f.comparison;
  f.context.api = async () => fresh; f.context.modal = async () => null;
  await f.context.showDraftComparison(f.doc);
  assert.equal(f.session.opened, f.prior); assert.equal(f.doc.comparisonTarget, f.comparison); assert.deepEqual(f.released, [fresh.sessionId]);
});

test("a successful explicit comparison retires old handles only after the new writer ACK", async () => {
  const f = deliveryFixture(), fresh = { ...f.comparison, sessionId: "fresh-comparison" }, gate = deferred<void>(), entered = deferred<void>();
  f.doc.comparisonTarget = f.comparison;
  f.context.api = async (route: string) => route === "/api/draft/resolve" ? {} : fresh;
  f.context.modal = async () => ({ action: "mergeRecovered", source: "M" });
  f.context.persistDraft = async () => { if (f.session.opened.sessionId === fresh.sessionId) { entered.resolve(); await gate.promise; } f.session.acknowledgeDraft(f.session.source, f.session.serial, f.session.issueDraftRevision()); };
  const comparing = f.context.showDraftComparison(f.doc); await entered.promise;
  assert.deepEqual(f.released, []); gate.resolve(); await comparing;
  assert.equal(f.session.opened.sessionId, fresh.sessionId); assert.equal(f.session.source, "M"); assert.equal(f.doc.comparisonTarget, undefined);
  assert.deepEqual(f.released, [f.prior.sessionId, f.comparison.sessionId]);
});

test("literal ABA without an ACK or source transaction cannot consume a clean history or Query result", async () => {
 for (const action of ["history", "query", "structure"]) {
  const f=deliveryFixture(); const basis={...f.basis};
  f.session.inputId=crypto.randomUUID(); f.session.inputId=crypto.randomUUID();
  assert.equal(f.session.source,"S");assert.equal(f.session.serial,0);assert.equal(f.doc.editor.generation,0);
  f.context.api=async()=>f.fact;
  if(action==="history") await assert.rejects(f.context.historyRestored(f.result,basis));
  else if(action==="query") await assert.rejects(f.context.queryDelivery({...f.fact,document:{...f.comparison,sessionId:f.prior.sessionId}},basis));
  else await assert.rejects(f.context.structuralDelivery({operationId:"generated",status:"committed",documentDisposition:"updated",document:{...f.prior,source:"X",baseId:"generated"}},basis));
  assert.equal(f.session.source,"S");assert.equal(f.doc.editor.source,"S");assert.equal(f.session.baseId,"old-base");
 }
});

test("a fully qualified Query delivery succeeds and only an input identity ABA blocks its equal-source counterpart",async()=>{
 for(const aba of [false,true]){
  const f=deliveryFixture(),basis={...f.basis};
  const document={...f.prior,source:"X",baseId:"generated-base",recoveredDraftConflict:false};
  if(aba)f.session.inputId=crypto.randomUUID();
  const delivery=f.context.queryDelivery({document,committedSource:"X",documentSourceMatchesCommit:true},basis);
  if(aba){await assert.rejects(delivery);assert.equal(f.session.source,"S");assert.equal(f.session.baseId,"old-base");}
  else{await delivery;assert.equal(f.session.source,"X");assert.equal(f.session.baseId,"generated-base");}
 }
});

test("SaveAs installs the actual new saved-view identity only when the frozen author input has not changed",async()=>{
 for(const aba of [false,true]) {
  const f=fixture(),sent=deferred<void>(),reply=deferred<any>();const prior={sessionId:"original",root:f.original.root,source:"B",baseId:"B-base",path:`${f.original.root}/original.typ`,kind:"ordinary"};
  const session=new DocumentSession(prior);session.edit("S");const originalInput=session.inputId;
  const target={...prior,sessionId:"target",viewInputId:crypto.randomUUID(),source:"S",baseId:"target-base",path:`${f.original.root}/copy.typ`};
  const doc:any={session,writing:{state:{composing:false},settleForSessionChange:async()=>{}},editor:{authorInputId:session.inputId,clearTokens(){}},sourceTrashed:false};Object.assign(f.context,{current:doc,documents:[doc]});let acknowledged:string|undefined;
  Object.assign(f.context,{historyPanel:{hasPending:()=>false},historyContext:()=>({}),refreshChrome(){},renderSidebar(){},operationId:()=>"save-as",originalCommitted,submitOriginal,TerminatedAttempt,ApiError,errorUiText,rebindCommittedSave,canRebindSaveAs,fileSaveReceipt,
   persistDraft:async()=>{if(session.opened.sessionId==="target")acknowledged=session.inputId;session.acknowledgeDraft(session.source,session.serial,session.issueDraftRevision());},
   api:async(route:string)=>{if(route==="/api/save-as"){sent.resolve();return reply.promise;}assert.equal(route,"/api/library/open");return f.original;}});
  const saving=f.context.saveDocument(doc,target.path);await sent.promise;
  if(aba){session.inputId=crypto.randomUUID();session.inputId=crypto.randomUUID();doc.editor.authorInputId=session.inputId;}
  const current=session.inputId;reply.resolve({operationId:"save-as",status:"saved",path:target.path,baseId:target.baseId,committedSource:"S",document:target,documentSourceMatchesCommit:true});await saving;
  assert.equal(session.serial,1);assert.equal(session.source,"S");assert.equal(acknowledged,aba?current:target.viewInputId);if(aba)assert.notEqual(acknowledged,originalInput);
 }
});
