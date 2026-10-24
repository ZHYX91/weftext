import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { QueryPanel } from "./query-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { LibraryCloseActivity } from "./library-native-close";
import { message, UiError, errorUiText } from "./ui-locale";
import { ApiError } from "./api-error";
import { DocumentSession, rebindCommittedSave } from "./session";
import { fileSaveReceipt, type FileSaveReceipt } from "./file-save-result";
import { originalCommitted, submitOriginal, TerminatedAttempt } from "./original-operation";

// Exercise the actual main consumers and WorkspaceTools methods. Substitute
// transport, rendered DOM and editor construction, not transition decisions.
const main = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = ["chooseLibrary", "createLibrary", "restoreNavigation", "activateDocument", "renderSidebar", "addDocument", "ownsOpenedSession", "saveDocument", "pendingSave"];
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
    libraryCloseActivity: new LibraryCloseActivity(), uiMessage: message, UiError,
    inventory: original, query: "original filter", selectedFolder: "C:/original/folder",
    collapsed: new Set(["original-collapsed"]), contentSelections: new Set(["original-selection"]),
    navigationContexts: new Map(), navigationGeneration: 0, documents: [current], current,
    activeSourceOrigin: "original-origin", bodyCloseFrozen: false, sidebarComposing: false,
    pathDialog: async () => next.root, flushLiteral: async () => {}, prepareSwitch: async () => {},
    api: async () => next,
    normalize: (path: string) => path.replaceAll("\\", "/").toLowerCase(),
    readOnlySource: () => false, flowScroll: null, flowHasProjection: false,
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
  return { context, query, workspace, original, next, effects, released, actualRenderSidebar, savedMain, assertOriginal, target };
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
  const opened = { ...prior, sessionId: "save-delivery", source: session.source, baseId: "fresh-base" };
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
