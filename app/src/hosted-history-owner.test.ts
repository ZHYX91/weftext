import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { ApiError, decodeApiError } from "./api-error";
import { readStrictJsonResponse } from "./strict-json";
import { TestDocument } from "./dom.test-support";
import { OrderedDraft } from "./ordered-draft";
import { sameHostedQueryAudience, sameHostedQueryIdentity } from "./hosted-state";
import { unavailableCommittedHistory } from "./hosted-history-state";
import { UiError, message as uiMessage, locale, uiTextValue, setUiText, displayMessage, uiDisplayParameter } from "./ui-locale";

const sourcePath = new URL("./hosted.ts", import.meta.url);
const sourceFile = ts.createSourceFile("hosted.ts", readFileSync(sourcePath, "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = new Set(["acknowledgeHistory", "finishUnavailableHistory", "readDocumentReference", "request", "clearDisplay", "sessionAudience", "closeOwnedSession", "releaseViewSessions", "confirmHostedRetirementCuts", "retryHostedRetirements", "capturedDraftAck", "hostedInput", "draftOrder", "bindHostedTransmission", "issueHostedInput", "retireHostedWriters"]);
const functions = sourceFile.statements.filter(node => ts.isFunctionDeclaration(node) && !!node.name && names.has(node.name.text) || ts.isClassDeclaration(node) && node.name?.text === "HostedError");
assert.equal(functions.length, names.size + 1, "exercise every production request/error/cleanup consumer");
const compiled = ts.transpileModule(functions.map(node => node.getText(sourceFile)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}
const json = (value: unknown, status = 200) => new Response(JSON.stringify(value), { status, headers: { "Content-Type": "application/json" } });
const unavailable = () => json({ error: { code: "unavailable", explanation: uiMessage("common.unavailable"), diagnostics: [], causes: [] } }, 404);

function fixture() {
  const document = new TestDocument(), nodes = new Map<string, any>();
  const calls: { path: string; credential: string; body: any }[] = [], cleared: string[] = [];
  const initial = { resource: "AlicePrivate", sessionId: "alice-original", baseId: "alice-base", source: "saved Alice", rights: { read: true, write: true, history: true, download: true } };
  const fresh = { ...initial, sessionId: "alice-late-history" }, bob = { ...initial, resource: "BobPrivate", sessionId: "bob-current", source: "saved Bob" };
  const opened = deferred<Response>(), openStarted = deferred<void>();
  let acceptLateClose = false;
  const el = (id: string) => { if (!nodes.has(id)) nodes.set(id, document.createElement("div")); return nodes.get(id); };
  const context: any = {
    doc: initial, draft: { source: "Alice draft", baseSource: initial.source }, identity: { actor: "alice", deployment: "space", generation: 0 }, token: "alice-credential",
    requestEpoch: 0, authorizationEpoch: 0, recoveryContinuations: new WeakMap(), hostedWriterRetirements: new Map(), orderedDrafts: new WeakMap(), OrderedDraft,
    sameHostedQueryAudience, sameHostedQueryIdentity, unavailableCommittedHistory, readStrictJsonResponse, ApiError, decodeApiError, UiError, uiMessage, uiTextValue, setUiText, displayMessage, uiDisplayParameter, locale,
    document: { querySelectorAll() { return []; } }, el, AbortSignal, clearTimeout,
    hostedCapture: undefined, captureVersion: 0, capturePanel: document.createElement("section"), interfaceLocale: { dispose() {} }, mountInterfaceLocale() {},
    verifiedPersonalAudience: "alice-personal", personalControls: { clear() {} }, persistPersonalInputs: async () => undefined,
    everydayNavigation: { clear() {} }, everydayBar: document.createElement("div"), actionVersion: 0, busy: false, draftTimer: undefined, expiredSessionId: undefined,
    openDocumentTitles: new Map(), openDocumentButtons: new Map(), documentTabs: document.createElement("nav"), closeHostedInspector() {},
    retiredStructureSessions: new Set(), retiredStructureSources: new Set(), structureDocuments: [], clearNodeNavigation() {}, compareRequired: false,
    cachePrefix: "alice-cache", stableReference: undefined, draftCache: { clearMemory() {} }, sessionStorage: { removeItem() {} }, controls() {},
    status(message: unknown) { context.lastStatus = uiTextValue(message as any); },
    fetch: async (url: string, options: RequestInit) => {
      const call = { path: url.replace("/api/hosted/", ""), credential: (options.headers as Record<string, string>).Authorization, body: options.body ? JSON.parse(String(options.body)) : undefined };
      calls.push(call);
      if (call.path === "open") {
        if (call.credential === "Bearer alice-credential") { openStarted.resolve(); return opened.promise; }
        assert.equal(call.credential, "Bearer bob-credential"); assert.equal(call.body.resource, bob.resource); return json(bob);
      }
      if (call.path === "history/status") {
        assert.equal(call.body.resource, initial.resource); return unavailable();
      }
      if (call.path === "navigation/ref") {
        assert.equal(call.credential, "Bearer alice-credential");
        assert.ok([initial.sessionId, fresh.sessionId].includes(call.body.sessionId));
        return json({ resource: initial.resource, reference: "weftext-node://workspace/alice-node" });
      }
      if (call.path === "close") {
        if (call.body.sessionId === fresh.sessionId && !acceptLateClose) return json({ closed: false }, 500);
        return json({ closed: true });
      }
      throw new Error(`unexpected transport ${call.path}`);
    },
  };
  for (const name of ["latexAuthoring", "diagramAuthoring", "recognitionPanel", "source", "knowledgeLinks", "citations", "navigationRegistry", "contentImport", "ownerBackup", "fields", "review", "queries", "office", "history", "library", "media", "structures"])
    context[name] = { version: 0, clear() { cleared.push(name); if (name === "source") context.source.value = ""; }, revoke() { cleared.push(name); } };
  Object.assign(context.source, { value: "Alice draft", serial: 1, authorInputId: "alice-input", flush: async () => {} });
  vm.createContext(context); vm.runInContext(compiled, context);
  return { context, initial, fresh, bob, calls, cleared, opened, openStarted, acceptLateClose() { acceptLateClose = true; } };
}

function historyRecord(entry: "acknowledgeHistory" | "finishUnavailableHistory") {
  return { request: { resource: "AlicePrivate", operationId: "alice-restore" }, status: { resource: "AlicePrivate", operationId: "alice-restore", status: "committed", resultStatus: entry === "acknowledgeHistory" ? "available" : "unavailable", ...(entry === "acknowledgeHistory" ? { result: { source: "historical Alice" } } : {}) } };
}

for (const entry of ["acknowledgeHistory", "finishUnavailableHistory"] as const) {
  test(`${entry} cannot query Alice history using Bob or retire Bob after Alice's late open`, async () => {
    const f = fixture(), c = f.context, record = historyRecord(entry), originalRecord = structuredClone(record);
    const pending = c[entry](record), rejected = assert.rejects(pending);
    await f.openStarted.promise;
    c.clearDisplay(uiMessage("common.close"), true); await c.retryHostedRetirements();
    c.identity = { actor: "bob", deployment: "space", generation: 1 }; c.token = "bob-credential";
    const bob = await c.request("open", { resource: f.bob.resource });
    c.doc = bob; c.draft = { source: "unsaved Bob", baseSource: bob.source }; c.source.value = "unsaved Bob";
    const bobDraft = c.draft, clearCount = f.cleared.length;
    f.opened.resolve(json(f.fresh)); await rejected; await c.retryHostedRetirements();
    assert.equal(f.calls.filter(call => call.path === "history/status").length, 0, JSON.stringify(f.calls));
    assert.equal(c.doc, bob); assert.equal(c.draft, bobDraft); assert.equal(c.source.value, "unsaved Bob");
    assert.equal(c.token, "bob-credential"); assert.equal(c.identity.actor, "bob"); assert.equal(f.cleared.length, clearCount);
    assert.equal(f.calls.filter(call => call.path === "close" && call.body.sessionId === bob.sessionId).length, 0);
    const handle = c.hostedWriterRetirements.get(f.fresh.sessionId);
    assert.ok(handle); assert.equal(handle.audience.credential, "alice-credential"); assert.equal(handle.audience.actor, "alice");
    assert.deepEqual(record, originalRecord, "stale delivery retains the original receipt");
    f.acceptLateClose(); await c.retryHostedRetirements();
    const freshCloses = f.calls.filter(call => call.path === "close" && call.body.sessionId === f.fresh.sessionId);
    assert.ok(freshCloses.length >= 2); assert.ok(freshCloses.every(call => call.credential === "Bearer alice-credential"));
    assert.equal(c.hostedWriterRetirements.size, 0); assert.equal(c.doc, bob); assert.equal(c.draft, bobDraft);
  });

  test(`${entry} still clears a genuinely current owner's view on history authorization 404`, async () => {
    const f = fixture(), c = f.context;
    const pending = c[entry](historyRecord(entry)), rejected = assert.rejects(pending);
    await f.openStarted.promise; f.opened.resolve(json(f.fresh)); await rejected; await c.retryHostedRetirements();
    assert.equal(f.calls.filter(call => call.path === "history/status" && call.credential === "Bearer alice-credential").length, 1);
    assert.equal(c.doc, undefined); assert.equal(c.draft, undefined); assert.equal(c.source.value, ""); assert.equal(f.cleared.length, 17);
    assert.ok(f.calls.some(call => call.path === "close" && call.body.sessionId === f.initial.sessionId && call.credential === "Bearer alice-credential"));
    const handle = c.hostedWriterRetirements.get(f.fresh.sessionId);
    assert.ok(handle); assert.equal(handle.audience.credential, "alice-credential"); assert.equal(c.identity.actor, "alice");
  });
}
