import { ApiError } from "./api-error";
import { TestDocument } from "./dom.test-support";
import { UiError, message as uiMessage, locale, uiTextValue, setUiText, errorUiText, displayMessage, uiDisplayParameter } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { HostedDraftCache } from "./hosted-draft-cache";
import { PersonalInputRecovery } from "./personal-input-recovery";
import { PersonalInputStoreConflict, type PersonalInputBackend, type PersonalInputBundle } from "./personal-input-storage";
import { OrderedDraft } from "./ordered-draft";
import { applySaveResult, bindHostedDraftReference, draftDisposition, freezeHostedSave, hostedOpenedDraft, hostedDocumentDraft, qualifyHostedDraft } from "./hosted-state";
import { freezeOriginalAttempt } from "./original-operation";
import { unavailableCommittedHistory } from "./hosted-history-state";

// Execute the production lifecycle functions and their actual logout/pagehide
// registrations. DOM/editor panels and HTTP are the only substituted boundaries.
const sourceFile = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = new Set(["sessionAudience", "closeOwnedSession", "confirmHostedRetirementCuts", "confirmDraftBasis", "releaseViewSessions", "retryHostedRetirements", "capturedDraftAck", "queueHostedWriterRetirement", "retireHostedWriters", "hostedInput", "draftOrder", "bindHostedTransmission", "issueHostedInput", "persistHostedDraft", "hostedHistoryBasis", "finishUnavailableHistory", "save", "refreshOpenSession", "clearDisplay", "controls", "retain", "personalAudience", "unresolvedPersonalSave", "observePersonalWrite", "persistPersonalInputs", "open", "checkedSaveStatus", "readDocumentReference", "publishDocumentReference", "installDocument", "verifyPersonalIdentity", "recoverPersonalOperation", "qualifyPersonalOriginalRoute", "finishPersonalOperation"]);
const statements = sourceFile.statements.filter(node =>
  ts.isFunctionDeclaration(node) && !!node.name && names.has(node.name.text)
  || ts.isClassDeclaration(node) && node.name?.text === "HostedError"
  || ts.isExpressionStatement(node) && (node.getText(sourceFile).startsWith('el("logout").onclick') || node.getText(sourceFile).startsWith('window.addEventListener("pagehide"')));
const compiled = ts.transpileModule(statements.map(node => node.getText(sourceFile)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function deferred<T>() {
  let resolve!: (value: T) => void, reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
  return { promise, resolve, reject };
}

async function fixture() {
  const testDocument = new TestDocument();
  const nodes = new Map<string, any>(), listeners = new Map<string, (event: any) => void>();
  const calls: { url: string; options: RequestInit }[] = [];
  const storage = new Map<string, string>();
  const cleared: string[] = [];
  const el = (id: string) => {
    if (!nodes.has(id)) nodes.set(id, testDocument.createElement("div"));
    return nodes.get(id);
  };
  const context: any = {
    hostedWriterRetirements: new Map(), orderedDrafts:new WeakMap(), OrderedDraft, hostedDocumentDraft, hostedOpenedDraft, qualifyHostedDraft,
    applySaveResult, bindHostedDraftReference, draftDisposition, freezeHostedSave, freezeOriginalAttempt, unavailableCommittedHistory, crypto,
    ApiError, UiError, uiMessage, locale, uiTextValue, setUiText, errorUiText, displayMessage, uiDisplayParameter, localeDisposals: 0, interfaceLocale: { dispose() { context.localeDisposals++; } }, mountInterfaceLocale() {},
    el, window: { addEventListener(name: string, listener: (event: any) => void) { const prior = listeners.get(name); listeners.set(name, event => { prior?.(event); listener(event); }); } },
    document: { querySelectorAll() { return []; } },
    everydayNavigation: { clear() {} }, everydayBar: { replaceChildren() {} },
    personalControls: { clear() {} },
    // Chrome remains a presentation port in this lifecycle harness. Its title
    // stores and tab DOM are real so actual logout must clear them as well.
    openDocumentTitles: new Map(), openDocumentButtons: new Map(), documentTabs: testDocument.createElement("nav"), closeHostedInspector() {},
    renderEverydayBar() {}, renderHostedDocumentTabs() {}, syncHostedPanels() {}, noteInputAudience() { return null; }, noteInputRecovery: { count() { return 0; } },
    authoringEditable() { return false; }, actionVersion: 0,
    requestEpoch: 0, authorizationEpoch: 0, cachePrefix: "weftext-hosted-draft:space:alice:0:",
    hostedCapture: undefined, captureVersion: 0, capturePanel: testDocument.createElement("section"),
    doc: { sessionId: "alice-original", resource: "Shared", rights: { read: true, write: true } },
    draft: { source: "unsaved draft", baseSource: "saved source" },
    identity: { actor: "alice", deployment: "space", generation: 0 }, token: "alice-credential",
    verifiedPersonalAudience: JSON.stringify(["space", "alice"]),
    stableReference: undefined, retiredStructureSessions: new Set<string>(), retiredStructureSources: new Set<string>(),
    structureDocuments: [], compareRequired: false, busy: false, expiredSessionId: undefined, draftTimer: undefined,
    clearNodeNavigation() {}, clearTimeout, AbortSignal,
    status(message: unknown) { context.lastStatus = uiTextValue(message); },
    sessionStorage: { getItem(key: string) { return storage.get(key) ?? null; }, setItem(key: string, value: string) { storage.set(key, value); }, removeItem(key: string) { storage.delete(key); } },
    fetch: async (url: string, options: RequestInit) => { calls.push({ url, options }); return {ok:true,status:200,json:async()=>url.endsWith("/draft")?{status:"inputAcknowledged",evidence:"authorDraft",inputRevision:JSON.parse(options.body as string).inputRevision}:{closed:true},arrayBuffer: async () => new ArrayBuffer(0) }; },
    request: async () => ({ active: true }),
  };
  for (const name of ["latexAuthoring", "diagramAuthoring", "recognitionPanel", "source", "knowledgeLinks", "citations", "navigationRegistry", "contentImport", "ownerBackup", "fields", "review", "queries", "office", "history", "library", "media", "structures"]) {
    context[name] = { value: "unsaved draft", viewMode: "source", pausedSession: false, clear() { cleared.push(name); }, revoke() { cleared.push(name); }, hasPending() { return false; } };
  }
  context.draftCache = new HostedDraftCache(context.sessionStorage, () => context.cachePrefix);
  const proposals = new Map<string, PersonalInputBundle>();
  const proposalBackend: PersonalInputBackend = {
    async read(audience) { return structuredClone(proposals.get(audience)); },
    async compareAndWrite(audience, expectedCommit, metadata, texts) {
      assert.equal(proposals.get(audience)?.commitId ?? 0, expectedCommit);
      proposals.set(audience, { commitId: expectedCommit + 1, metadata, texts: new Map(texts) });
      return expectedCommit + 1;
    },
  };
  context.personalInputs = new PersonalInputRecovery(proposalBackend);
  await context.personalInputs.open(context.verifiedPersonalAudience);
  context.source.personalInputRecords = () => [];
  context.source.personalInputReference = (resource: string, reference: string) => { context.personalReferences ??= new Map(); context.personalReferences.set(resource, reference); };
  vm.createContext(context); vm.runInContext(compiled, context);
  return { context, nodes, listeners, calls, storage, cleared, proposals, proposalBackend };
}

test("logout releases the original document before clearing its credential and all panels", async () => {
  const f = await fixture();
  for (let index = 0; index < 40; index++) {
    f.context.doc = { sessionId: `alice-${index}`, resource: "Shared", rights: { read: true, write: true } };
    f.context.draft = { source: "saved", baseSource: "saved" };
    f.context.identity = { actor: "alice", deployment: "space", generation: 0 };
    f.context.token = "alice-credential";
    const tab = f.context.documentTabs.ownerDocument.createElement("button");
    tab.textContent = "Alice current note";
    f.context.openDocumentTitles.set("Shared", tab.textContent);
    f.context.openDocumentButtons.set("Shared", tab);
    f.context.documentTabs.append(tab);
    f.nodes.get("logout").onclick();
    await f.context.retryHostedRetirements();
    assert.equal(f.context.doc, undefined); assert.equal(f.context.token, ""); assert.equal(f.context.identity, undefined);
    assert.equal(f.context.openDocumentTitles.size, 0); assert.equal(f.context.openDocumentButtons.size, 0); assert.equal(f.context.documentTabs.children.length, 0);
    assert.equal(f.calls.length, index + 1, "logout must dispatch the owned-session close");
    assert.equal(f.calls[index].url, "/api/hosted/close");
    assert.equal((f.calls[index].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
    assert.equal(JSON.parse(f.calls[index].options.body as string).sessionId, `alice-${index}`);
    assert.equal(f.calls[index].options.keepalive, true);
  }
  assert.equal(f.calls.length, 40); assert.equal(f.cleared.length, 40 * 17);
  for (const name of ["latexAuthoring", "diagramAuthoring", "recognitionPanel"]) assert.equal(f.cleared.filter(item => item === name).length, 40);
});

test("cleanup of a late temporary session uses its original actor after another actor connects", async () => {
  const f = await fixture(), captured = f.context.sessionAudience();
  f.context.identity = { actor: "bob", deployment: "space", generation: 2 }; f.context.token = "bob-credential";
  f.context.doc = { sessionId: "bob-current", resource: "Other", rights: { read: true, write: true } };
  await f.context.closeOwnedSession("alice-late", captured);
  assert.equal(f.calls.length, 1);
  assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
  assert.equal(f.context.doc.sessionId, "bob-current"); assert.equal(f.context.token, "bob-credential");
  assert.equal(f.cleared.length, 0);
});

test("pagehide preserves a back-forward cached page, but final departure releases all owned coordinates", async () => {
  const f = await fixture(), handler = f.listeners.get("pagehide");
  assert.ok(handler);
  f.context.retiredStructureSessions.add("alice-retired");
  handler({ persisted: true }); assert.equal(f.calls.length, 0); assert.equal(f.context.localeDisposals, 0);
  handler({ persisted: false });
  await f.context.retryHostedRetirements();
  assert.equal(f.context.localeDisposals, 1);
  assert.deepEqual(f.calls.map(call => JSON.parse(call.options.body as string).sessionId).sort(), ["alice-original", "alice-retired"]);
  assert.equal(f.cleared.length, 0); assert.equal(f.context.source.value, "unsaved draft");
  assert.equal(f.storage.size, 1);
});

test("failed close delivery or browser storage does not block local logout and page departure", async () => {
  const f = await fixture();
  f.context.fetch = async () => { throw new Error("offline"); };
  assert.doesNotThrow(() => f.nodes.get("logout").onclick());
  await Promise.resolve();
  assert.equal(f.context.token, ""); assert.equal(f.context.doc, undefined); assert.equal(f.cleared.length, 17);
  const other = await fixture();
  other.context.sessionStorage.setItem = () => { throw new Error("quota"); };
  assert.doesNotThrow(() => other.listeners.get("pagehide")!({ persisted: false }));
  assert.equal(other.calls.length, 1);
});

test("an expired lease preserves draft and Undo, blocks saving, and does not reopen or retry the write", async () => {
  const f = await fixture(), original = f.context.doc, draft = f.context.draft;
  let requests = 0;
  f.context.request = async (path: string, data: { sessionId: string }) => {
    requests++; assert.equal(path, "session/keepalive"); assert.equal(data.sessionId, "alice-original");
    throw vm.runInContext('new HostedError(new ApiError("sessionExpired", uiMessage("common.unavailable")), 410)', f.context);
  };
  await f.context.refreshOpenSession();
  assert.equal(f.context.doc, original); assert.equal(f.context.draft, draft);
  assert.equal(f.context.source.value, "unsaved draft"); assert.equal(f.cleared.length, 0);
  assert.equal(f.context.compareRequired, true); assert.equal(f.nodes.get("save").disabled, true);
  assert.match(f.context.lastStatus, /草稿与 Undo 保留/);
  await f.context.refreshOpenSession(); assert.equal(requests, 1); assert.equal(f.calls.length, 0);
});

test("a late lease-expiry response cannot pause a newer document or actor", async () => {
  const f = await fixture();
  let reject!: (reason: unknown) => void;
  f.context.request = () => new Promise((_, fail) => { reject = fail; });
  const pending = f.context.refreshOpenSession();
  f.context.doc = { sessionId: "newer-session", resource: "Other", rights: { read: true, write: true } };
  f.context.token = "bob-credential"; f.context.identity = { actor: "bob", deployment: "space", generation: 0 };
  reject(vm.runInContext('new HostedError(new ApiError("sessionExpired", uiMessage("common.unavailable")), 410)', f.context));
  await pending;
  assert.equal(f.context.compareRequired, false); assert.equal(f.context.expiredSessionId, undefined);
  assert.equal(f.context.lastStatus, undefined); assert.equal(f.storage.size, 0);
});


test("alias recovery queries only the frozen resource, retains unknown input and adopts only after the original receipt", async () => {
  for (const terminal of [false, true]) {
    const f = await fixture(), c = f.context, originalDoc = c.doc;
    const operation = "original-save", reference = "weftext-node://workspace/node";
    const retained = { source: "later input Y", baseSource: "base B", basis: { sessionId: crypto.randomUUID(), baseId: crypto.randomUUID() }, reference, pending: { resource: "Shared", operationId: operation, source: "submitted X", request: { sessionId: "alice-original", baseId: "original-base", operationId: operation, source: "submitted X" } } };
    c.draft = retained;
    c.source.value = retained.source; c.source.pausedSession = true;
    c.source.checkNavigationCapacity = () => {}; c.source.flush = async () => {};
    let rebound = 0, installed = 0, acknowledged = 0;
    c.source.rebindResourceAlias = (resource: string) => { assert.equal(resource, "Alias"); rebound++; };
    c.nodeAudience = () => ({ ...c.sessionAudience(), epoch: c.requestEpoch, sessionId: c.doc.sessionId, source: c.source.value, serial: 1 });
    c.sameHostedNodeAudience = (left: unknown, right: unknown) => JSON.stringify(left) === JSON.stringify(right);
    c.cached = () => undefined; c.hostedOpenedDraft = hostedOpenedDraft; c.applySaveResult = applySaveResult;
    c.finishPersonalOperation = (id: string, state: string) => { assert.equal(id, operation); assert.equal(state, "committed"); acknowledged++; };
    c.scheduleCurrentDraft = () => {};
    c.installDocument = (opened: any, draft: any, stableReference: string, recovered: boolean, staged: boolean, alias: boolean) => { assert.equal(stableReference, reference); assert.equal(draft.reference, reference); assert.equal(recovered, true); assert.equal(staged, true); assert.equal(alias, true); c.source.rebindResourceAlias(opened.resource); assert.equal(draft.source, "later input Y"); assert.equal(draft.baseSource, "submitted X"); assert.equal(draft.pending, undefined); assert.equal(draft.basis, undefined, "alias recovery must not borrow the alias open's new Base"); c.doc = opened; c.draft = draft; installed++; };
    const requests: { path: string; data: any }[] = [];
    c.request = async (path: string, data?: any) => {
      requests.push({ path, data });
      if (path === "me") return c.identity;
      if (path === "open") return { resource: "Alias", sessionId: "new-alias-session", baseId: "unrelated-alias-base", source: terminal ? "submitted X" : "base B", rights: { read: true, write: true } };
      if (path === "navigation/ref") return { resource: "Alias", reference };
      if (path === "operation") { assert.deepEqual(JSON.parse(JSON.stringify(data)), { resource: "Shared", operationId: operation }); return { resource: "Shared", operationId: operation, status: terminal ? "committed" : "recoveryPending", baseId: "save-base" }; }
      throw new Error("unexpected author request: " + path);
    };
    if (terminal) await c.open("Alias");
    else { await assert.rejects(c.open("Alias"), /原别名保存尚未终结/); assert.equal(c.doc, originalDoc); assert.equal(c.draft, retained); assert.equal(retained.pending.resource, "Shared"); }
    assert.equal(installed, terminal ? 1 : 0); assert.equal(rebound, terminal ? 1 : 0); assert.equal(acknowledged, terminal ? 1 : 0);
    assert.equal(c.source.value, "later input Y");
    assert.equal(requests.filter(call => call.path === "operation").length, 1);
    assert.equal(requests.some(call => ["save", "operation/settle"].includes(call.path)), false);
    assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, terminal ? "alice-original" : "new-alias-session");
  }
});

async function navigationReferenceFixture() {
  const f = await fixture(), c = f.context, original = c.doc, originalDraft = c.draft;
  const opened = { resource: "Target", title: "Target title", sessionId: crypto.randomUUID(), baseId: crypto.randomUUID(), source: "saved B 😀", rights: { read: true, write: true } };
  const reference = "weftext-node://workspace/target", started = deferred<void>(), reply = deferred<any>(), producers: any[] = [], requests: any[] = [];
  Object.assign(c.source, {
    serial: 1, authorInputId: crypto.randomUUID(), checkNavigationCapacity() {}, flush: async () => {},
    open(sessionId: string, resource: string, value: string) {
      const bound = c.personalReferences?.get(resource);
      assert.equal(bound, reference, "the first target producer must already have its stable owner");
      producers.push({ sessionId, resource, value, reference: bound }); c.source.value = value;
    },
  });
  c.nodeAudience = () => ({ ...c.sessionAudience(), epoch: c.requestEpoch, sessionId: c.doc?.sessionId, source: c.source.value, serial: c.source.serial, inputId: c.source.authorInputId });
  c.sameHostedNodeAudience = (left: unknown, right: unknown) => JSON.stringify(left) === JSON.stringify(right);
  c.cached = (resource: string) => c.draftCache.read(resource);
  c.queries.authorChanged = () => {}; c.history.documentChanged = () => {}; c.knowledgeAuthorChanged = () => {};
  c.request = async (path: string, data?: any, bytes?: boolean, attempt?: number, signal?: AbortSignal, scope?: string) => {
    requests.push({ path, data, bytes, attempt, signal, scope });
    if (path === "me") return c.identity;
    if (path === "open") return opened;
    assert.equal(path, "navigation/ref"); assert.equal(data.sessionId, opened.sessionId);
    assert.equal(bytes, false); assert.equal(attempt, 0); assert.equal(scope, "stagedTarget"); assert.ok(signal instanceof AbortSignal);
    started.resolve(); return reply.promise;
  };
  return { ...f, original, originalDraft, opened, reference, started, reply, producers, requests };
}

test("a delayed staged reference keeps A's exact Draft and Undo owner until the first B producer has a stable identity", async () => {
  const f = await navigationReferenceFixture(), c = f.context, pending = c.open(f.opened.resource);
  await f.started.promise;
  assert.equal(c.doc, f.original); assert.equal(c.draft, f.originalDraft); assert.equal(c.source.value, "unsaved draft");
  assert.deepEqual(f.producers, []); assert.equal(c.personalReferences?.has(f.opened.resource) ?? false, false); assert.equal(f.calls.length, 0);
  assert.equal(c.draftCache.read(f.opened.resource), undefined);
  f.reply.resolve({ resource: f.opened.resource, reference: f.reference }); await pending;
  assert.equal(c.doc, f.opened); assert.equal(c.draft.reference, f.reference); assert.equal(c.source.value, f.opened.source);
  assert.equal(f.nodes.get("copy-reference").disabled, false, "publishing the qualified target must enable the original copy-reference entry synchronously");
  assert.deepEqual(f.producers, [{ sessionId: f.opened.sessionId, resource: f.opened.resource, value: f.opened.source, reference: f.reference }]);
  assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, f.original.sessionId);
  assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
});

test("installing a target at the retained-editor capacity leaves every current owner and reference unchanged", async () => {
  const f = await navigationReferenceFixture(), c = f.context, epoch = c.requestEpoch, inputId = c.source.authorInputId;
  c.source.checkNavigationCapacity = (resource: string) => { assert.equal(resource, f.opened.resource); throw new Error("retained editor capacity"); };
  assert.throws(() => c.installDocument(f.opened, hostedDocumentDraft(f.opened), f.reference), /retained editor capacity/);
  assert.equal(c.doc, f.original); assert.equal(c.draft, f.originalDraft); assert.equal(c.source.value, "unsaved draft"); assert.equal(c.source.authorInputId, inputId); assert.equal(c.requestEpoch, epoch);
  assert.equal(c.personalReferences?.size ?? 0, 0); assert.equal(c.stableReference, undefined); assert.deepEqual(f.producers, []); assert.equal(f.calls.length, 0);
  assert.equal(c.draftCache.read(f.opened.resource), undefined);
});

test("late target references cannot install over a different actor or over newer input in A", async () => {
  for (const changed of ["actor", "source"] as const) {
    const f = await navigationReferenceFixture(), c = f.context, pending = c.open(f.opened.resource);
    await f.started.promise;
    let expectedDoc = f.original;
    if (changed === "actor") {
      expectedDoc = { sessionId: "bob-current", resource: "Other", rights: { read: true, write: true } };
      c.doc = expectedDoc; c.token = "bob-credential"; c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.authorizationEpoch++; c.requestEpoch++;
    } else { c.source.value = "newer A input 😀"; c.source.serial++; }
    f.reply.resolve({ resource: f.opened.resource, reference: f.reference }); await pending;
    assert.equal(c.doc, expectedDoc); assert.equal(c.draft, f.originalDraft); assert.deepEqual(f.producers, []);
    assert.equal(c.source.value, changed === "source" ? "newer A input 😀" : "unsaved draft");
    assert.equal(c.personalReferences?.has(f.opened.resource) ?? false, false); assert.equal(c.draftCache.read(f.opened.resource), undefined);
    assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, f.opened.sessionId);
    assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
    assert.equal(c.token, changed === "actor" ? "bob-credential" : "alice-credential");
  }
});

test("failed or malformed staged references preserve A and close B using its original actor", async () => {
  for (const failure of ["network", "resource", "reference"] as const) {
    const f = await navigationReferenceFixture(), c = f.context, pending = c.open(f.opened.resource), rejected = assert.rejects(pending);
    await f.started.promise;
    if (failure === "network") f.reply.reject(new Error("reference unavailable"));
    else f.reply.resolve({ resource: failure === "resource" ? "unrelated" : f.opened.resource, reference: failure === "reference" ? "https://untrusted/node" : f.reference });
    await rejected;
    assert.equal(c.doc, f.original); assert.equal(c.draft, f.originalDraft); assert.equal(c.source.value, "unsaved draft"); assert.deepEqual(f.producers, []);
    assert.equal(c.personalReferences?.has(f.opened.resource) ?? false, false); assert.equal(c.draftCache.read(f.opened.resource), undefined);
    assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, f.opened.sessionId);
    assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
  }
});

test("a final retention failure after target qualification does not publish B or replace A's producer", async () => {
  const f = await navigationReferenceFixture(), c = f.context, retainOriginal = c.retain;
  let qualified = false;
  c.retain = () => { if (qualified) return Promise.reject(new Error("final retention failed")); return retainOriginal(); };
  const pending = c.open(f.opened.resource), rejected = assert.rejects(pending, /final retention failed/);
  await f.started.promise; qualified = true; f.reply.resolve({ resource: f.opened.resource, reference: f.reference }); await rejected;
  assert.equal(c.doc, f.original); assert.equal(c.draft, f.originalDraft); assert.equal(c.source.value, "unsaved draft"); assert.deepEqual(f.producers, []);
  assert.equal(c.personalReferences?.has(f.opened.resource) ?? false, false); assert.equal(c.draftCache.read(f.opened.resource), undefined);
  assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, f.opened.sessionId);
  assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
});

test("an acknowledged target keeps a failed close handle until the original credential confirms expiry", async()=>{
 const f=await fixture();const previous={...f.context.doc};const target={...previous,sessionId:"target",baseId:"base",source:"B"};
 Object.assign(f.context,{doc:target,draft:{source:"Y",baseSource:"B",basis:{sessionId:"target",baseId:"base"}},source:{value:"Y",serial:1,authorInputId:crypto.randomUUID()},request:async(_path:string,wire:any)=>({status:"inputAcknowledged",evidence:"authorDraft",inputRevision:wire.inputRevision})});
 f.context.queueHostedWriterRetirement(previous,target);let closed=0;
 f.context.fetch=async(_path:string,options:any)=>{closed++;assert.equal(options.headers.Authorization,"Bearer alice-credential");return {ok:false,status:closed===1?500:410,arrayBuffer:async()=>new ArrayBuffer(0)};};
 await assert.rejects(f.context.persistHostedDraft(target));assert.ok(f.context.hostedWriterRetirements.has(previous.sessionId));assert.equal(f.context.source.value,"Y");
 await f.context.persistHostedDraft(target);assert.equal(closed,2);assert.equal(f.context.hostedWriterRetirements.size,0);
});
test("departure retains both writers across a failed target ACK and retries only with their original actor",async()=>{
 const f=await fixture();const previous={...f.context.doc};const target={...previous,sessionId:"target",baseId:"base",source:"B"};
 Object.assign(f.context,{doc:target,draft:{source:"Y",baseSource:"B",basis:{sessionId:"target",baseId:"base"}},source:{value:"Y",serial:1,authorInputId:crypto.randomUUID()}});
 f.context.queueHostedWriterRetirement(previous,target);let accept=false;const calls:string[]=[];
 f.context.fetch=async(path:string,options:any)=>{assert.equal(options.headers.Authorization,"Bearer alice-credential");calls.push(path);return {ok:accept,status:accept?200:500,json:async()=>path.endsWith("/draft")?{status:"inputAcknowledged",evidence:"authorDraft",inputRevision:JSON.parse(options.body).inputRevision}:{closed:true},arrayBuffer:async()=>new ArrayBuffer(0)};};
 f.context.releaseViewSessions();await f.context.retryHostedRetirements();assert.equal(calls.some(path=>path.endsWith("/close")),false);assert.equal(f.context.hostedWriterRetirements.size,2);
 f.context.token="bob-credential";f.context.identity={actor:"bob",generation:1,deployment:"space"};accept=true;await f.context.retryHostedRetirements();
 assert.equal(f.context.hostedWriterRetirements.size,0);assert.equal(calls.filter(path=>path.endsWith("/close")).length,2);assert.equal(f.context.token,"bob-credential");
});

test("ordinary navigation cannot close a target before its dependent writer's cut is confirmed",async()=>{
 const f=await fixture(),c=f.context,previous={...c.doc},target={...previous,sessionId:"target",baseId:"base",source:"B"};
 Object.assign(c,{doc:target,draft:{source:"Y",baseSource:"B",basis:{sessionId:"target",baseId:"base"}},source:{value:"Y",serial:1,authorInputId:crypto.randomUUID()}});
 c.queueHostedWriterRetirement(previous,target);const audience=c.sessionAudience();let accept=false;const effects:string[]=[];
 c.request=async()=>{throw new Error("original Draft reply unavailable");};
 await assert.rejects(c.persistHostedDraft(target));
 c.fetch=async(path:string,options:any)=>{effects.push(path);assert.equal(options.headers.Authorization,"Bearer alice-credential");return {ok:accept,status:accept?200:500,json:async()=>path.endsWith("/draft")?{status:"inputAcknowledged",evidence:"authorDraft",inputRevision:JSON.parse(options.body).inputRevision}:{closed:true},arrayBuffer:async()=>new ArrayBuffer(0)};};
 c.doc={sessionId:"next-note"};c.token="bob-credential";
 await assert.rejects(c.closeOwnedSession(target.sessionId,audience));
 assert.equal(effects.some(path=>path.endsWith("/close")),false);assert.equal(c.hostedWriterRetirements.size,2);
 accept=true;await c.closeOwnedSession(target.sessionId,audience);await c.retryHostedRetirements();
 assert.equal(c.hostedWriterRetirements.size,0);assert.equal(effects.filter(path=>path.endsWith("/close")).length,2);
});

test("explicit comparison installs only its frozen known basis before the next actual ACK",async()=>{
 for(const aba of [false,true]){
  const f=await fixture(),c=f.context,selected=c.doc,inputId=crypto.randomUUID();selected.baseId="confirmed-base";selected.source="saved B";
  c.draft={source:"unknown Y",baseSource:"old B",inputId};c.source.value="unknown Y";c.source.serial=0;c.source.authorInputId=inputId;c.source.flush=async()=>{};
  c.knowledgeAuthorChanged=()=>{};c.retain=()=>{};c.compareRequired=true;
  let finish!:()=>void;const sent=new Promise<void>(resolve=>{finish=resolve;});let release!:(value:unknown)=>void;
  c.request=async(path:string,wire:any)=>{if(path==="draft/resolve"){finish();return new Promise(resolve=>{release=resolve;});}
   assert.equal(path,"draft");assert.equal(wire.baseId,"confirmed-base");assert.equal("retainedBaseSource"in wire,false);
   return {status:"inputAcknowledged",evidence:"authorDraft",inputRevision:wire.inputRevision};};
  const confirmation=c.confirmDraftBasis();await sent;if(aba)c.source.authorInputId=crypto.randomUUID();release({status:"draftRebased"});
  if(aba){await assert.rejects(confirmation);assert.equal(c.draft.basis,undefined);assert.equal(c.compareRequired,true);}
  else {await confirmation;assert.equal(c.draft.basis.sessionId,selected.sessionId);assert.equal(c.draft.basis.baseId,selected.baseId);assert.equal(c.compareRequired,false);await c.persistHostedDraft(selected);}
 }
});

test("navigation retirement replays the latest actual Y transmission after two lost replies without issuing S under revision 3", async () => {
  const f = await fixture(), c = f.context, previous = { ...c.doc }, captured = c.sessionAudience();
  const target = { ...previous, sessionId: "target-writer", baseId: "base-B", source: "B" };
  Object.assign(c, { doc: target, draft: { source: "S", baseSource: "B", basis: { sessionId: target.sessionId, baseId: target.baseId } },
    source: { value: "S", serial: 1, authorInputId: "input-S" } });
  c.queueHostedWriterRetirement(previous, target);
  const yStarted = deferred<void>(), yReply = deferred<any>(), retryStarted = deferred<void>(), retryReply = deferred<boolean>();
  const sent: any[] = [], closed: string[] = [];
  c.request = async (path: string, wire: any) => {
    assert.equal(path, "draft"); sent.push(structuredClone(wire));
    if (wire.inputRevision === 1) throw new Error("S ACK reply lost");
    yStarted.resolve(); return yReply.promise;
  };
  await assert.rejects(c.persistHostedDraft(target), /S ACK reply lost/);
  Object.assign(c.source, { value: "Y", serial: 2, authorInputId: "input-Y", historyBasis: async () => ({ source: "Y", serial: 2, inputId: "input-Y" }) });
  c.draft.source = "Y";
  const history = c.hostedHistoryBasis(true), historyFailure = assert.rejects(history, /Y ACK reply lost/);
  await yStarted.promise;
  c.fetch = async (path: string, options: any) => {
    assert.equal(options.headers.Authorization, "Bearer alice-credential");
    if (path.endsWith("/close")) { closed.push(JSON.parse(options.body).sessionId); return { ok: true, status: 200, json: async () => ({ closed: true }) }; }
    const wire = JSON.parse(options.body); sent.push(wire); retryStarted.resolve();
    const accepted = await retryReply.promise;
    return { ok: accepted, status: accepted ? 200 : 500, json: async () => ({ status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision }) };
  };
  c.doc = { sessionId: "Other", resource: "Other" }; c.source.value = "Other source"; c.token = "bob-credential"; c.requestEpoch++;
  const retirement = c.closeOwnedSession(target.sessionId, captured), retirementFailure = assert.rejects(retirement);
  assert.equal(closed.length, 0);
  yReply.reject(new Error("Y ACK reply lost")); await historyFailure; await retryStarted.promise;
  assert.equal(closed.length, 0); retryReply.resolve(false); await retirementFailure;
  assert.equal(c.hostedWriterRetirements.size, 2);
  assert.deepEqual(sent.map(wire => [wire.source, wire.inputRevision]), [["S", 1], ["Y", 2], ["Y", 2]]);
  c.fetch = async (path: string, options: any) => {
    assert.equal(options.headers.Authorization, "Bearer alice-credential");
    const wire = JSON.parse(options.body);
    if (path.endsWith("/close")) { closed.push(wire.sessionId); return { ok: true, status: 200, json: async () => ({ closed: true }) }; }
    sent.push(wire); return { ok: true, status: 200, json: async () => ({ status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision }) };
  };
  await c.retryHostedRetirements();
  assert.deepEqual(sent.map(wire => [wire.source, wire.inputRevision]), [["S", 1], ["Y", 2], ["Y", 2], ["Y", 2]]);
  assert.deepEqual(sent[1], sent[2]); assert.deepEqual(sent[1], sent[3]);
  assert.deepEqual(closed.sort(), [previous.sessionId, target.sessionId].sort()); assert.equal(c.hostedWriterRetirements.size, 0);
  assert.equal(c.doc.sessionId, "Other"); assert.equal(c.source.value, "Other source");
});

test("a Core-consumed cut stays retained after cleanup retry fails instead of manufacturing a new transmission", async () => {
  const f = await fixture(), c = f.context, previous = { ...c.doc }, captured = c.sessionAudience();
  const target = { ...previous, sessionId: "consumed-writer", baseId: "consumed-base", source: "B" };
  Object.assign(c, { doc: target, draft: { source: "S", baseSource: "B", basis: { sessionId: target.sessionId, baseId: target.baseId } },
    source: { value: "S", serial: 1, authorInputId: "input-S" } });
  c.request = async (_path: string, wire: any) => ({ status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision });
  await c.persistHostedDraft(target); c.queueHostedWriterRetirement(previous, target);
  c.doc = { sessionId: "Other" }; c.token = "bob-credential";
  const retries: any[] = [];
  c.fetch = async (path: string, options: any) => {
    assert.equal(path, "/api/hosted/draft", "neither writer may close without the exact cut");
    assert.equal(options.headers.Authorization, "Bearer alice-credential"); retries.push(JSON.parse(options.body));
    return { ok: false, status: 409 };
  };
  await assert.rejects(c.closeOwnedSession(target.sessionId, captured)); await c.retryHostedRetirements();
  assert.ok(retries.length >= 2); assert.ok(retries.every(wire => wire.inputRevision === 1 && wire.source === "S" && wire.baseId === "consumed-base"));
  assert.equal(c.hostedWriterRetirements.size, 2); assert.equal(c.hostedWriterRetirements.get(previous.sessionId).waitFor.transmission.inputRevision, 1);
});

test("unavailable-history cleanup retains Alice's late open handle when Bob connects and close delivery fails", async () => {
  const f = await fixture(), c = f.context, selected = c.doc;
  const record = { request: { resource: selected.resource, operationId: "unavailable-restore" }, status: { status: "committed", resultStatus: "unavailable" } };
  const started = deferred<void>(), opened = deferred<any>();
  c.source.flush = async () => {}; c.history.installStatus = () => {};
  c.request = async (path: string) => {
    if (path === "open") { started.resolve(); return opened.promise; }
    assert.equal(path, "history/status"); return record.status;
  };
  const pending = c.finishUnavailableHistory(record), failed = assert.rejects(pending);
  await started.promise;
  c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.token = "bob-credential"; c.authorizationEpoch++; c.requestEpoch++;
  c.doc = { sessionId: "bob-current", resource: "Other" };
  let accepts = false; const credentials: string[] = [];
  c.fetch = async (path: string, options: any) => {
    assert.equal(path, "/api/hosted/close"); assert.equal(JSON.parse(options.body).sessionId, "alice-late-history");
    credentials.push(options.headers.Authorization);
    return { ok: accepts, status: accepts ? 200 : 500, json: async () => ({ closed: true }) };
  };
  opened.resolve({ ...selected, sessionId: "alice-late-history" }); await failed;
  const queued = c.hostedWriterRetirements.get("alice-late-history");
  assert.ok(queued); assert.equal(queued.audience.actor, "alice"); assert.equal(queued.audience.credential, "alice-credential");
  accepts = true; await c.retryHostedRetirements();
  assert.deepEqual(credentials, ["Bearer alice-credential", "Bearer alice-credential"]); assert.equal(c.hostedWriterRetirements.size, 0);
  assert.equal(c.doc.sessionId, "bob-current"); assert.equal(c.token, "bob-credential");
});

test("unavailable-history reference failures and final retention rejection leave the original producer intact", async () => {
  for (const failure of ["network", "resource", "actor", "retention"] as const) {
    const f = await fixture(), c = f.context, selected = c.doc, originalDraft = c.draft;
    const fresh = { ...selected, sessionId: crypto.randomUUID(), source: "saved history source" };
    const record = { request: { resource: selected.resource, operationId: "unavailable-restore" }, status: { status: "committed", resultStatus: "unavailable" } };
    const started = deferred<void>(), reply = deferred<any>(), producers: any[] = [];
    Object.assign(c.source, { flush: async () => {}, open: (...args: any[]) => producers.push(args) });
    c.history.installStatus = () => {};
    if (failure === "retention") c.retain = () => Promise.reject(new Error("unavailable history retention failed"));
    c.request = async (path: string, data?: any, bytes?: boolean, attempt?: number, signal?: AbortSignal, scope?: string) => {
      if (path === "open") return fresh;
      if (path === "history/status") return record.status;
      assert.equal(path, "navigation/ref"); assert.equal(data.sessionId, fresh.sessionId);
      assert.equal(bytes, false); assert.equal(attempt, 0); assert.equal(scope, "stagedTarget"); assert.ok(signal instanceof AbortSignal);
      started.resolve(); return reply.promise;
    };
    const pending = c.finishUnavailableHistory(record), rejected = assert.rejects(pending); await started.promise;
    assert.equal(c.doc, selected); assert.equal(c.draft, originalDraft); assert.equal(c.source.value, "unsaved draft"); assert.deepEqual(producers, []); assert.equal(f.calls.length, 0);
    let expectedDoc = selected;
    if (failure === "actor") {
      expectedDoc = { sessionId: "bob-current", resource: "Other" }; c.doc = expectedDoc;
      c.token = "bob-credential"; c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.authorizationEpoch++; c.requestEpoch++;
    }
    if (failure === "network") reply.reject(new Error("unavailable history reference failed"));
    else reply.resolve({ resource: failure === "resource" ? "unrelated" : fresh.resource, reference: "weftext-node://workspace/original" });
    await rejected;
    assert.equal(c.doc, expectedDoc); assert.equal(c.draft, originalDraft); assert.equal(c.source.value, "unsaved draft"); assert.deepEqual(producers, []);
    assert.equal(c.personalReferences?.size ?? 0, 0); assert.equal(f.calls.length, 1); assert.equal(JSON.parse(f.calls[0].options.body as string).sessionId, fresh.sessionId);
    assert.equal((f.calls[0].options.headers as Record<string, string>).Authorization, "Bearer alice-credential");
  }
});

async function personalOperationFixture() {
  const f = await fixture(), c = f.context, audience = c.verifiedPersonalAudience;
  const original = { id: "original-input", owner: { kind: "body" as const, resource: "Shared", reference: "weftext-node://workspace/original" },
    origin: { baseLength: 0, pieces: [{ text: "own alias input 😀" }], detached: [], revision: 1 },
    pending: { operationId: "save-through-alias", route: "known" as const, resource: "Alias-B", status: "unresolved" as const } };
  await c.personalInputs.update(audience, [original]);
  const finished: any[] = [], requests: any[] = [];
  c.source.settlePersonalSave = (operationId: string, committed: boolean) => finished.push({ operationId, committed });
  return { ...f, audience, original, record: c.personalInputs.list(audience)[0], finished, requests };
}

async function unknownPersonalOperationFixture(status = "unresolved", ownerReference: string | null = "weftext-node://workspace/original") {
  const f = await fixture(), c = f.context, audience = c.verifiedPersonalAudience;
  const owner = { kind: "body", resource: "Shared", ...(ownerReference ? { reference: ownerReference } : {}) };
  f.proposals.set(audience, { commitId: 3, metadata: JSON.stringify({ version: 3, records: [{ kind: "text", id: "original-input", sequence: 1, owner,
    origin: { baseLength: 0, pieces: [{ text: "original own input 😀" }], detached: [], revision: 1 }, pending: { operationId: "unknown-save", status } }] }), texts: new Map() });
  c.personalInputs = new PersonalInputRecovery(f.proposalBackend); await c.personalInputs.open(audience);
  return { ...f, audience, record: c.personalInputs.list(audience)[0] };
}

test("unknown original routes block only unresolved saves that cannot be excluded by a fresh different node identity", async () => {
  const cases = [
    { name: "original resource", resource: "Shared", ownerReference: "weftext-node://workspace/original", currentReference: "weftext-node://workspace/other", blocked: true },
    { name: "alias of same node", resource: "Alias", ownerReference: "weftext-node://workspace/original", currentReference: "weftext-node://workspace/original", blocked: true },
    { name: "missing original reference", resource: "Other", ownerReference: null, currentReference: "weftext-node://workspace/other", blocked: true },
    { name: "missing current reference", resource: "Other", ownerReference: "weftext-node://workspace/original", currentReference: undefined, blocked: true },
    { name: "fresh different node", resource: "Other", ownerReference: "weftext-node://workspace/original", currentReference: "weftext-node://workspace/other", blocked: false },
  ];
  for (const scenario of cases) {
    const f = await unknownPersonalOperationFixture("unresolved", scenario.ownerReference), c = f.context;
    c.doc.resource = scenario.resource; c.stableReference = scenario.currentReference ? { resource: c.doc.resource, sessionId: c.doc.sessionId, reference: scenario.currentReference } : undefined;
    assert.equal(c.unresolvedPersonalSave(c.doc), scenario.blocked, scenario.name); c.controls(); assert.equal(f.nodes.get("save").disabled, scenario.blocked, scenario.name);
    assert.equal(c.personalInputs.pending(), true);
    if (scenario.blocked) { let requests = 0; c.request = async () => { requests++; throw new Error("unknown route must not issue an author request"); }; await c.save(); assert.equal(requests, 0); }
    assert.equal(c.source.value, "unsaved draft"); assert.equal(c.personalInputs.list(f.audience)[0].pending.route, "unknownOriginalRoute");
  }
  for (const terminal of ["committed", "cancelled", "resolved"]) {
    const f = await unknownPersonalOperationFixture(terminal), c = f.context; assert.equal(c.unresolvedPersonalSave(c.doc), false); c.controls(); assert.equal(f.nodes.get("save").disabled, false); assert.equal(c.personalInputs.pending(), false);
  }
});

test("an unknown original route cannot query or settle by borrowing its input owner's resource", async () => {
  const f = await unknownPersonalOperationFixture(), c = f.context, requests: string[] = [];
  c.request = async (path: string) => { requests.push(path); assert.equal(path, "me"); return c.identity; };
  await assert.rejects(c.recoverPersonalOperation(f.record, f.audience));
  assert.deepEqual(requests, ["me"]); assert.deepEqual(c.personalInputs.list(f.audience)[0], f.record); assert.equal(c.source.value, "unsaved draft"); assert.equal(c.personalInputs.pending(), true);
});

test("a known unresolved original remains a Save barrier after actual logout and a fresh recovery instance", async () => {
  const f = await personalOperationFixture(), c = f.context, original = c.doc;
  f.nodes.get("logout").onclick(); await c.retryHostedRetirements(); assert.equal(c.doc, undefined); assert.equal(c.identity, undefined);
  c.identity = { actor: "alice", deployment: "space", generation: 0 }; c.token = "alice-credential"; c.verifiedPersonalAudience = f.audience; c.cachePrefix = "weftext-hosted-draft:space:alice:0:";
  c.personalInputs = new PersonalInputRecovery(f.proposalBackend); await c.personalInputs.open(f.audience);
  c.doc = { ...original, resource: "Current-alias", sessionId: "relogin-session" }; c.draft = { source: "unsaved draft", baseSource: "saved source" };
  c.stableReference = { resource: c.doc.resource, sessionId: c.doc.sessionId, reference: f.original.owner.reference };
  let requests = 0, flushes = 0; c.source.flush = async () => { flushes++; }; c.request = async () => { requests++; throw new Error("a retained unresolved operation cannot issue a new Save"); };
  assert.equal(c.unresolvedPersonalSave(c.doc), true); c.controls(); assert.equal(f.nodes.get("save").disabled, true); await c.save();
  assert.equal(requests, 0); assert.equal(flushes, 0); assert.deepEqual(c.personalInputs.list(f.audience)[0].pending, f.record.pending); assert.equal(c.source.value, "unsaved draft");
});

test("proving a route does not settle its operation; only a terminal receipt permits the next actual prepared Save", async () => {
  const f = await unknownPersonalOperationFixture(), c = f.context, selected = c.doc, inputId = crypto.randomUUID(), baseId = crypto.randomUUID();
  Object.assign(selected, { sessionId: crypto.randomUUID(), baseId, source: "saved B" });
  c.draft = { source: "new X 😀", baseSource: selected.source, basis: { sessionId: selected.sessionId, baseId }, inputId };
  c.stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference: f.record.owner.reference };
  let prepared: any, posts = 0;
  Object.assign(c.source, {
    value: "new X 😀", serial: 1, authorInputId: inputId, flush: async () => {}, refreshQueries() {},
    beginPersonalSave: async (operationId: string, source: string) => { prepared = { id: "prepared-input", owner: { kind: "body", resource: selected.resource, reference: f.record.owner.reference }, origin: { baseLength: 0, pieces: [{ text: source }], detached: [], revision: 1 }, pending: { operationId, route: "known", resource: selected.resource, status: "unresolved" } }; },
    settlePersonalSave: (operationId: string, committed: boolean) => { if (prepared?.pending.operationId === operationId) prepared.pending.status = committed ? "committed" : "cancelled"; },
    personalInputRecords: () => prepared ? [prepared] : [],
  });
  c.queries = { authorChanged() {}, refreshCatalog: async () => {} }; c.knowledgeAuthorChanged = () => {};
  c.request = async (path: string, wire: any) => {
    if (path === "draft") return { status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision };
    assert.equal(path, "save"); posts++; assert.notEqual(wire.operationId, "unknown-save"); assert.equal(c.unresolvedPersonalSave(selected), true); assert.equal(c.unresolvedPersonalSave(selected, wire.operationId), false);
    return { resource: selected.resource, operationId: wire.operationId, status: "committed", baseId: crypto.randomUUID() };
  };
  await c.qualifyPersonalOriginalRoute({ kind: "coreReceipt", audience: f.audience, receipt: { operationId: "unknown-save", resource: "Alias-original", status: "recoveryPending" } });
  assert.equal(c.personalInputs.list(f.audience)[0].pending.route, "known"); assert.equal(c.unresolvedPersonalSave(selected), true); await c.save(); assert.equal(posts, 0);
  await c.finishPersonalOperation("unknown-save", "cancelled", { resource: "Alias-original", operationId: "unknown-save", status: "cancelled" });
  assert.equal(c.unresolvedPersonalSave(selected), false); c.controls(); assert.equal(f.nodes.get("save").disabled, false); await c.save();
  assert.equal(posts, 1); assert.equal(c.draft.pending, undefined); assert.equal(c.source.value, "new X 😀"); assert.equal(c.personalInputs.list(f.audience).find((record: any) => record.id === "original-input").pending.status, "cancelled");
});

test("a same-node unresolved operation merged during final retention prevents the new Save POST without inventing a terminal result", async () => {
  const f = await fixture(), c = f.context, audience = c.verifiedPersonalAudience, selected = c.doc, inputId = crypto.randomUUID(), baseId = crypto.randomUUID();
  Object.assign(selected, { sessionId: crypto.randomUUID(), baseId, source: "saved B" });
  c.draft = { source: "new X 😀", baseSource: selected.source, basis: { sessionId: selected.sessionId, baseId }, inputId };
  c.stableReference = { resource: selected.resource, sessionId: selected.sessionId, reference: "weftext-node://workspace/same-node" };
  let prepared: any, posted = 0; const settled: string[] = [];
  Object.assign(c.source, {
    value: "new X 😀", serial: 1, authorInputId: inputId, flush: async () => {}, refreshQueries() {},
    beginPersonalSave: async (operationId: string, source: string) => { prepared = { id: "new-prepared-input", owner: { kind: "body", resource: selected.resource, reference: c.stableReference.reference }, origin: { baseLength: 0, pieces: [{ text: source }], detached: [], revision: 1 }, pending: { operationId, route: "known", resource: selected.resource, status: "unresolved" } }; },
    settlePersonalSave: (operationId: string) => settled.push(operationId), personalInputRecords: () => prepared ? [prepared] : [],
  });
  c.queries = { authorChanged() {}, refreshCatalog: async () => {} }; c.knowledgeAuthorChanged = () => {};
  const write = f.proposalBackend.compareAndWrite.bind(f.proposalBackend); let injected = false;
  f.proposalBackend.compareAndWrite = async (...args) => {
    if (!injected && args[0] === audience && JSON.parse(args[2]).records.some((record: any) => record.id === "new-prepared-input")) {
      injected = true;
      const anotherPage = new PersonalInputRecovery(f.proposalBackend); await anotherPage.open(audience);
      await anotherPage.update(audience, [{ id: "older-unresolved-input", owner: { kind: "body", resource: "Original-alias", reference: c.stableReference.reference },
        origin: { baseLength: 0, pieces: [{ text: "other page original input" }], detached: [], revision: 1 },
        pending: { operationId: "older-unresolved-save", route: "known", resource: "Original-alias", status: "unresolved" } }]);
    }
    if ((f.proposals.get(args[0])?.commitId ?? 0) !== args[1]) throw new PersonalInputStoreConflict();
    return write(...args);
  };
  c.request = async (path: string, wire: any) => { if (path === "draft") return { status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision }; posted++; throw new Error("final guard must prevent every new author request"); };
  await c.save(); assert.equal(injected, true); assert.equal(posted, 0); assert.deepEqual(settled, []); assert.equal(c.source.value, "new X 😀");
  assert.equal(c.draft.pending.operationId, prepared.pending.operationId); assert.equal(c.draft.pending.request.operationId, prepared.pending.operationId);
  const retained = c.personalInputs.list(audience); assert.equal(retained.length, 2);
  assert.deepEqual(retained.find((record: any) => record.id === "new-prepared-input").pending, prepared.pending);
  assert.equal(retained.find((record: any) => record.id === "older-unresolved-input").pending.status, "unresolved");
  assert.equal(c.unresolvedPersonalSave(selected, prepared.pending.operationId), true);
  const reloaded = new PersonalInputRecovery(f.proposalBackend); await reloaded.open(audience); assert.deepEqual(reloaded.list(audience), retained);
});

test("an actual terminal receipt proves the original route durably before settling its input owner", async () => {
  for (const actorChanged of [false, true]) {
    const f = await unknownPersonalOperationFixture(), c = f.context, started = deferred<void>(), acknowledged = deferred<void>(), settled: any[] = [];
    const write = f.proposalBackend.compareAndWrite.bind(f.proposalBackend); let first = true;
    f.proposalBackend.compareAndWrite = async (...args) => { if (first && args[0] === f.audience) { first = false; started.resolve(); await acknowledged.promise; } return write(...args); };
    c.source.settlePersonalSave = (operationId: string, committed: boolean) => settled.push({ operationId, committed });
    const pending = c.finishPersonalOperation("unknown-save", "committed", { operationId: "unknown-save", resource: "Alias-original", status: "committed", baseId: "saved-base" });
    const finished = actorChanged ? assert.rejects(pending) : pending; await started.promise;
    assert.deepEqual(settled, []); assert.equal(JSON.parse(f.proposals.get(f.audience)!.metadata).records[0].pending.route, "unknownOriginalRoute");
    if (actorChanged) { c.token = "bob-credential"; c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.authorizationEpoch++; }
    acknowledged.resolve(); await finished;
    const record = c.personalInputs.list(f.audience)[0]; assert.deepEqual(record.owner, f.record.owner); assert.equal(record.id, f.record.id);
    assert.deepEqual(record.pending, { operationId: "unknown-save", route: "known", resource: "Alias-original", status: actorChanged ? "unresolved" : "committed" });
    assert.deepEqual(settled, actorChanged ? [] : [{ operationId: "unknown-save", committed: true }]); assert.equal(c.source.value, "unsaved draft");
  }
});

test("personal operation recovery uses its frozen alias route while the same input keeps its original owner", async () => {
  for (const initial of ["committed", "notCommitted"] as const) {
    const f = await personalOperationFixture(), c = f.context;
    c.request = async (path: string, data?: any) => {
      if (path === "me") return c.identity;
      f.requests.push({ path, data }); assert.equal(c.token, "alice-credential");
      assert.deepEqual(JSON.parse(JSON.stringify(data)), { resource: "Alias-B", operationId: "save-through-alias" });
      assert.ok(path === "operation" || path === "operation/settle");
      return { resource: "Alias-B", operationId: "save-through-alias", status: path === "operation" ? initial : "cancelled", baseId: "saved-base" };
    };
    await c.recoverPersonalOperation(f.record, f.audience);
    assert.deepEqual(f.requests.map(request => request.path), initial === "committed" ? ["operation"] : ["operation", "operation/settle"]);
    assert.deepEqual(f.finished, [{ operationId: "save-through-alias", committed: initial === "committed" }]);
    const retained = c.personalInputs.list(f.audience)[0]; assert.equal(retained.id, f.original.id); assert.deepEqual(retained.owner, f.original.owner);
    assert.deepEqual(retained.pending, { operationId: "save-through-alias", route: "known", resource: "Alias-B", status: initial === "committed" ? "committed" : "cancelled" });
    assert.equal(c.source.value, "unsaved draft"); assert.equal(f.calls.length, 0);
  }
});

test("personal recovery cannot accept another operation, its input-owner route, or a still-unknown original result", async () => {
  for (const malformed of ["operation", "resource", "unresolved"] as const) {
    const f = await personalOperationFixture(), c = f.context;
    c.request = async (path: string, data?: any) => {
      if (path === "me") return c.identity;
      assert.equal(path, "operation"); assert.equal(data.resource, "Alias-B");
      return { resource: malformed === "resource" ? "Shared" : "Alias-B", operationId: malformed === "operation" ? "another-save" : "save-through-alias", status: malformed === "unresolved" ? "recoveryPending" : "committed", baseId: "saved-base" };
    };
    await assert.rejects(c.recoverPersonalOperation(f.record, f.audience));
    assert.deepEqual(f.finished, []); assert.deepEqual(c.personalInputs.list(f.audience)[0], f.record); assert.equal(c.source.value, "unsaved draft");
  }
});

test("actor changes during personal recovery cannot settle Alice's original under Bob", async () => {
  for (const phase of ["identity", "operation"] as const) {
    const f = await personalOperationFixture(), c = f.context, started = deferred<void>(), reply = deferred<any>();
    c.request = async (path: string, data?: any) => {
      if (path === "me" && phase !== "identity") return c.identity;
      assert.equal(path, phase === "identity" ? "me" : "operation"); if (data) assert.equal(data.resource, "Alias-B");
      started.resolve(); return reply.promise;
    };
    const pending = c.recoverPersonalOperation(f.record, f.audience), rejected = phase === "identity" ? assert.rejects(pending) : pending;
    await started.promise;
    c.token = "bob-credential"; c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.authorizationEpoch++;
    reply.resolve(phase === "identity" ? { actor: "alice", deployment: "space", generation: 0 } : { resource: "Alias-B", operationId: "save-through-alias", status: "notCommitted" });
    await rejected;
    assert.deepEqual(f.finished, []); assert.deepEqual(c.personalInputs.list(f.audience)[0], f.record); assert.equal(c.token, "bob-credential"); assert.equal(c.source.value, "unsaved draft");
  }
});

test("a personal recovery response cannot settle a newer revision of the same input record", async () => {
  const f = await personalOperationFixture(), c = f.context, started = deferred<void>(), reply = deferred<any>();
  c.request = async (path: string) => { if (path === "me") return c.identity; assert.equal(path, "operation"); started.resolve(); return reply.promise; };
  const pending = c.recoverPersonalOperation(f.record, f.audience), rejected = assert.rejects(pending, /变化/); await started.promise;
  await c.personalInputs.update(f.audience, [{ ...f.original, origin: { ...f.original.origin, pieces: [{ text: "newer own input 😀" }], revision: 2 } }]);
  reply.resolve({ resource: "Alias-B", operationId: "save-through-alias", status: "committed", baseId: "saved-base" }); await rejected;
  assert.deepEqual(f.finished, []); const latest = c.personalInputs.list(f.audience)[0]; assert.equal(latest.pending.resource, "Alias-B"); assert.equal(latest.pending.status, "unresolved"); assert.equal(latest.origin.pieces[0].text, "newer own input 😀");
});

test("the actual normal Save consumer advances its confirmed Base while late Y remains the next author input", async () => {
  const f = await fixture(), c = f.context, selected = c.doc;
  const baseB = crypto.randomUUID(), savedBase = crypto.randomUUID(), inputX = crypto.randomUUID(), inputY = crypto.randomUUID();
  Object.assign(selected, { sessionId: crypto.randomUUID(), baseId: baseB, source: "B" });
  c.draft = { source: "X", baseSource: "B", basis: { sessionId: selected.sessionId, baseId: selected.baseId }, inputId: inputX };
  Object.assign(c.source, { value: "X", serial: 1, authorInputId: inputX, flush: async () => {}, beginPersonalSave: async () => {}, settlePersonalSave() {}, refreshQueries() {} });
  c.queries = { authorChanged() {}, refreshCatalog: async () => {} }; c.history.hasPending = () => false; c.knowledgeAuthorChanged = () => {}; c.finishPersonalOperation = async () => {};
  const started = deferred<any>(), committed = deferred<any>(), inputs: any[] = [];
  c.request = async (path: string, wire: any) => {
    if (path === "draft") { inputs.push(structuredClone(wire)); return { status: "inputAcknowledged", evidence: "authorDraft", inputRevision: wire.inputRevision }; }
    assert.equal(path, "save"); started.resolve(wire); return committed.promise;
  };
  const saving = c.save(), wire = await started.promise;
  Object.assign(c.source, { value: "late Y", serial: 2, authorInputId: inputY }); await c.retain();
  committed.resolve({ status: "committed", operationId: wire.operationId, resource: selected.resource, baseId: savedBase }); await saving;
  assert.equal(c.source.value, "late Y"); assert.equal(c.draft.source, "late Y"); assert.equal(c.draft.baseSource, "X"); assert.equal(c.draft.pending, undefined);
  assert.deepEqual(JSON.parse(JSON.stringify(c.draft.basis)), { sessionId: selected.sessionId, baseId: savedBase }); assert.equal(c.draft.inputId, inputY);
  await c.persistHostedDraft(selected);
  assert.deepEqual(inputs.map(input => [input.source, input.inputRevision, input.baseId]), [["X", 1, baseB], ["late Y", 2, savedBase]]);
  assert.ok(inputs.every(input => !("retainedBaseSource" in input))); assert.equal(wire.source, "X"); assert.equal(wire.baseId, baseB);
});
