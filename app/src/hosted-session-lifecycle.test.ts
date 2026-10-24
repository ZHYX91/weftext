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
import type { PersonalInputBundle } from "./personal-input-storage";
import { OrderedDraft } from "./ordered-draft";
import { applySaveResult, hostedOpenedDraft, hostedDocumentDraft, qualifyHostedDraft } from "./hosted-state";

// Execute the production lifecycle functions and their actual logout/pagehide
// registrations. DOM/editor panels and HTTP are the only substituted boundaries.
const sourceFile = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = new Set(["sessionAudience", "closeOwnedSession", "releaseViewSessions", "retryHostedRetirements", "capturedDraftAck", "queueHostedWriterRetirement", "retireHostedWriters", "hostedInput", "draftOrder", "persistHostedDraft", "refreshOpenSession", "clearDisplay", "controls", "retain", "personalAudience", "observePersonalWrite", "persistPersonalInputs", "open", "checkedSaveStatus"]);
const statements = sourceFile.statements.filter(node =>
  ts.isFunctionDeclaration(node) && !!node.name && names.has(node.name.text)
  || ts.isClassDeclaration(node) && node.name?.text === "HostedError"
  || ts.isExpressionStatement(node) && (node.getText(sourceFile).startsWith('el("logout").onclick') || node.getText(sourceFile).startsWith('window.addEventListener("pagehide"')));
const compiled = ts.transpileModule(statements.map(node => node.getText(sourceFile)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;

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
    hostedWriterRetirements: new Map(), orderedDrafts:new WeakMap(), OrderedDraft, hostedDocumentDraft, qualifyHostedDraft,
    ApiError, UiError, uiMessage, locale, uiTextValue, setUiText, errorUiText, displayMessage, uiDisplayParameter, localeDisposals: 0, interfaceLocale: { dispose() { context.localeDisposals++; } }, mountInterfaceLocale() {},
    el, window: { addEventListener(name: string, listener: (event: any) => void) { const prior = listeners.get(name); listeners.set(name, event => { prior?.(event); listener(event); }); } },
    document: { querySelectorAll() { return []; } },
    everydayNavigation: { clear() {} }, everydayBar: { replaceChildren() {} },
    personalControls: { clear() {} },
    renderEverydayBar() {}, noteInputAudience() { return null; }, noteInputRecovery: { count() { return 0; } },
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
  context.personalInputs = new PersonalInputRecovery({
    async read(audience) { return structuredClone(proposals.get(audience)); },
    async compareAndWrite(audience, expectedCommit, metadata, texts) {
      assert.equal(proposals.get(audience)?.commitId ?? 0, expectedCommit);
      proposals.set(audience, { commitId: expectedCommit + 1, metadata, texts: new Map(texts) });
      return expectedCommit + 1;
    },
  });
  await context.personalInputs.open(context.verifiedPersonalAudience);
  context.source.personalInputRecords = () => [];
  vm.createContext(context); vm.runInContext(compiled, context);
  return { context, nodes, listeners, calls, storage, cleared };
}

test("logout releases the original document before clearing its credential and all panels", async () => {
  const f = await fixture();
  for (let index = 0; index < 40; index++) {
    f.context.doc = { sessionId: `alice-${index}`, resource: "Shared", rights: { read: true, write: true } };
    f.context.draft = { source: "saved", baseSource: "saved" };
    f.context.identity = { actor: "alice", deployment: "space", generation: 0 };
    f.context.token = "alice-credential";
    f.nodes.get("logout").onclick();
    await f.context.retryHostedRetirements();
    assert.equal(f.context.doc, undefined); assert.equal(f.context.token, ""); assert.equal(f.context.identity, undefined);
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
    const retained = { source: "later input Y", baseSource: "base B", reference, pending: { resource: "Shared", operationId: operation, source: "submitted X", request: { sessionId: "alice-original", baseId: "original-base", operationId: operation, source: "submitted X" } } };
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
    c.installDocument = (opened: any, draft: any, recovered: boolean) => { assert.equal(recovered, true); assert.equal(draft.source, "later input Y"); assert.equal(draft.baseSource, "submitted X"); assert.equal(draft.pending, undefined); c.doc = opened; c.draft = draft; installed++; };
    const requests: { path: string; data: any }[] = [];
    c.request = async (path: string, data?: any) => {
      requests.push({ path, data });
      if (path === "me") return c.identity;
      if (path === "open") return { resource: "Alias", sessionId: "new-alias-session", source: terminal ? "submitted X" : "base B", rights: { read: true, write: true } };
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
