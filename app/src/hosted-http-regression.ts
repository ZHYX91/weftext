// Invoked by scripts/test_hosted_history.py against its disposable hosted server.
// Production functions and OrderedDraft run unchanged; only DOM/editor boundaries
// use the existing lifecycle fixture. HTTP always crosses hosted authorization.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import vm from "node:vm";
import ts from "typescript";
import { TestDocument } from "./dom.test-support";
import { ApiError } from "./api-error";
import { UiError, message as uiMessage, locale, uiTextValue, setUiText, errorUiText, displayMessage, uiDisplayParameter } from "./ui-locale";
import { HostedDraftCache } from "./hosted-draft-cache";
import { PersonalInputRecovery } from "./personal-input-recovery";
import { OrderedDraft } from "./ordered-draft";
import { applySaveResult, freezeHostedSave, hostedOpenedDraft, hostedDocumentDraft, qualifyHostedDraft } from "./hosted-state";
import { freezeOriginalAttempt } from "./original-operation";
import { unavailableCommittedHistory } from "./hosted-history-state";

const config = JSON.parse(readFileSync(0, "utf8"));
const sourceFile = ts.createSourceFile("hosted.ts", readFileSync(new URL("./hosted.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const names = new Set(["sessionAudience", "closeOwnedSession", "confirmHostedRetirementCuts", "releaseViewSessions", "retryHostedRetirements", "capturedDraftAck", "queueHostedWriterRetirement", "retireHostedWriters", "hostedInput", "draftOrder", "bindHostedTransmission", "issueHostedInput", "persistHostedDraft", "hostedHistoryBasis", "save", "retain", "personalAudience", "observePersonalWrite", "persistPersonalInputs", "open", "checkedSaveStatus"]);
const statements = sourceFile.statements.filter(node => ts.isFunctionDeclaration(node) && !!node.name && names.has(node.name.text) || ts.isClassDeclaration(node) && node.name?.text === "HostedError");
assert.equal(statements.length, names.size + 1, "all selected production functions must exist");
const compiled = ts.transpileModule(statements.map(node => node.getText(sourceFile)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;

async function fixture() {
  const fixtureFile = ts.createSourceFile("hosted-session-lifecycle.test.ts", readFileSync(new URL("./hosted-session-lifecycle.test.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const declaration = fixtureFile.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === "fixture");
  assert.ok(declaration, "reuse the lifecycle DOM fixture without executing its tests");
  const scope = vm.createContext({ compiled, TestDocument, ApiError, UiError, uiMessage, locale, uiTextValue, setUiText, errorUiText, displayMessage, uiDisplayParameter, HostedDraftCache, PersonalInputRecovery, OrderedDraft, hostedOpenedDraft, hostedDocumentDraft, qualifyHostedDraft, applySaveResult, freezeHostedSave, freezeOriginalAttempt, unavailableCommittedHistory, vm, crypto, AbortSignal, clearTimeout, structuredClone, assert });
  vm.runInContext(ts.transpileModule(declaration.getText(fixtureFile), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText, scope);
  const f = await vm.runInContext("fixture()", scope), c = f.context;
  Object.assign(c, { crypto, freezeHostedSave, freezeOriginalAttempt, applySaveResult, hostedOpenedDraft });
  const identity = await api("me");
  Object.assign(c, { token: config.token, identity, verifiedPersonalAudience: JSON.stringify([identity.deployment, identity.actor]) });
  await c.personalInputs.open(c.verifiedPersonalAudience);
  const target = config.target;
  Object.assign(c, { doc: target, draft: hostedDocumentDraft(target), source: {
    value: target.source, serial: 0, authorInputId: crypto.randomUUID(), viewMode: "source", pausedSession: false,
    flush: async () => {}, checkNavigationCapacity() {}, personalInputRecords: () => [],
    beginPersonalSave: async () => {}, settlePersonalSave() {}, refreshQueries() {},
  } });
  c.nodeAudience = () => ({ ...c.sessionAudience(), epoch: c.requestEpoch, sessionId: c.doc.sessionId, source: c.source.value, serial: c.source.serial });
  c.sameHostedNodeAudience = (left: unknown, right: unknown) => JSON.stringify(left) === JSON.stringify(right);
  c.cached = () => undefined;
  c.controls = () => {};
  c.knowledgeAuthorChanged = () => {};
  c.queries.authorChanged = () => {};
  c.queries.refreshCatalog = async () => {};
  c.finishPersonalOperation = async (operationId: string, state: string) => { assert.equal(state, "committed"); c.source.settlePersonalSave(operationId, true); };
  return c;
}

// Credentials come through stdin and never enter result records or diagnostics.
async function api(path: string, body?: unknown, credential = config.token) {
  const response = await fetch(config.base + "/api/hosted/" + path, {
    method: body === undefined ? "GET" : "POST", headers: { "Content-Type": "application/json", Authorization: "Bearer " + credential },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const result = await response.json();
  assert.equal(response.status, 200, path + ": " + JSON.stringify(result));
  assert.equal(response.headers.get("Cache-Control"), "no-store");
  return result;
}

function input(c: any, value: string) { c.source.value = value; c.source.serial++; c.source.authorInputId = crypto.randomUUID(); }
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>(done => { resolve = done; });
  return { promise, resolve };
}

async function navigationRetry() {
  const c = await fixture(), target = c.doc, previous = config.previous;
  const stale = target.source + "\nretained S", newer = target.source + "\nNEW ACKNOWLEDGED Y";
  input(c, stale); c.draft.source = stale;
  const effects: { path: string; body: any }[] = [];
  c.fetch = async (path: string, options: RequestInit) => {
    const body = JSON.parse(options.body as string), route = path.slice("/api/hosted/".length);
    effects.push({ path: route, body });
    assert.equal((options.headers as Record<string, string>).Authorization === "Bearer " + config.token, true, "retirement retains its original credential");
    const ack = await api(route, body);
    return { ok: true, status: 200, json: async () => ack, arrayBuffer: async () => new ArrayBuffer(0) };
  };
  const ySeen = deferred<void>(), yAck = deferred<any>(), installed = deferred<void>();
  c.request = async (path: string, body?: any) => {
    effects.push({ path, body });
    const result = await api(path, body);
    if (path === "draft") {
      if (body.source === stale) throw new Error("deliberately discarded durable S ACK");
      assert.equal(body.source, newer); ySeen.resolve();
      await yAck.promise;
    }
    return result;
  };
  c.installDocument = (opened: any, retained: any) => { c.doc = opened; c.draft = retained; c.source.value = retained.source; c.source.serial = 0; c.source.authorInputId = opened.viewInputId; installed.resolve(); };
  c.source.historyBasis = async () => ({ sessionId: target.sessionId, source: c.source.value, serial: c.source.serial, inputId: c.source.authorInputId });
  c.queueHostedWriterRetirement(previous, target);
  await assert.rejects(c.persistHostedDraft(target), /discarded durable S ACK/);
  input(c, newer);
  const history = c.hostedHistoryBasis(false).then(() => false, () => true);
  await ySeen.promise;
  const opening = c.open("Other");
  await Promise.race([installed.promise, opening.then(() => { throw new Error("navigation ended without adopting Other"); })]);
  assert.equal(c.doc.resource, "Other");
  yAck.resolve(undefined);
  assert.equal(await history, true, "late history ACK cannot deliver the retired document basis");
  await opening; await c.retryHostedRetirements();
  const drafts = effects.filter(effect => effect.path === "draft");
  assert.ok(drafts.length >= 3, "retirement must requalify the exact original Y cut");
  assert.equal(drafts[0].body.source, stale); assert.equal(drafts[0].body.inputRevision, 1);
  assert.equal(drafts[1].body.source, newer); assert.equal(drafts[1].body.inputRevision, 2);
  for (const retry of drafts.slice(2)) assert.deepEqual(retry.body, drafts[1].body, "retirement must not mint stale S revision 3");
  assert.deepEqual(effects.filter(effect => effect.path === "close").map(effect => effect.body.sessionId).sort(), [previous.sessionId, target.sessionId].sort());
  assert.equal(c.hostedWriterRetirements.size, 0);
  return { effects, stale, newer, currentResource: c.doc.resource };
}

async function repeatedSave() {
  const c = await fixture(), initialBase = c.doc.baseId, initialSource = c.doc.source;
  const sources = ["S", "Y", "Z"].map(letter => initialSource + "\nSAVE " + letter), late = initialSource + "\nLATE INPUT AFTER Z";
  const effects: { path: string; body: any; baseId?: string; status?: string }[] = [];
  let saves = 0;
  c.request = async (path: string, body: any) => {
    const result = await api(path, body);
    effects.push({ path, body, baseId: result.baseId, status: result.status });
    if (path === "save") {
      assert.equal(result.status, "committed");
      // Actual input arriving after durable save but before its response is adopted.
      input(c, sources[++saves] ?? late); await c.retain();
    }
    return result;
  };
  input(c, sources[0]);
  const bases = [initialBase];
  for (const [index, sent] of sources.entries()) {
    await c.save();
    assert.equal(saves, index + 1, "actual hosted save() must dispatch every save");
    assert.equal(c.doc.source, sent); assert.notEqual(c.doc.baseId, bases[index]); bases.push(c.doc.baseId);
    assert.equal(c.draft.basis.sessionId, c.doc.sessionId); assert.equal(c.draft.basis.baseId, c.doc.baseId);
    assert.equal(c.draft.baseSource, sent); assert.equal(c.draft.pending, undefined);
    assert.equal(c.source.value, sources[index + 1] ?? late); assert.equal(c.draft.source, c.source.value);
  }
  const drafts = effects.filter(effect => effect.path === "draft");
  const writes = effects.filter(effect => effect.path === "save");
  for (let index = 0; index < 3; index++) {
    assert.equal(drafts[index].body.baseId, bases[index], "draft ACK must use the exact most recent committed Base");
    assert.equal(drafts[index].body.inputRevision, index + 1);
    assert.equal(writes[index].body.baseId, bases[index]); assert.equal(writes[index].body.inputRevision, index + 1);
    assert.equal(writes[index].body.source, sources[index]);
  }
  await c.persistHostedDraft(c.doc);
  return { effects, sources, late, bases, currentSource: c.source.value, savedSource: c.doc.source };
}

async function consumedCut() {
  const c = await fixture(), target = c.doc, previous = config.previous;
  const submitted = target.source + "\nCONSUMED ORIGINAL CUT";
  input(c, submitted); c.draft.source = submitted;
  let original: any;
  const effects: { path: string; body: any; status: number }[] = [];
  c.request = async (path: string, body?: any) => {
    const result = await api(path, body);
    if (path === "draft") { original = body; throw new Error("deliberately discarded consumed cut ACK"); }
    return result;
  };
  c.fetch = async (path: string, options: RequestInit) => {
    assert.equal((options.headers as Record<string, string>).Authorization === "Bearer " + config.token, true, "retry uses the original owner");
    const response = await fetch(config.base + path, options);
    effects.push({ path, body: JSON.parse(options.body as string), status: response.status });
    return response;
  };
  c.installDocument = (opened: any, retained: any) => { c.doc = opened; c.draft = retained; c.source.value = retained.source; };
  c.queueHostedWriterRetirement(previous, target);
  await assert.rejects(c.persistHostedDraft(target), /discarded consumed cut ACK/);
  assert.equal(original.inputRevision, 1);
  const sourceHash = createHash("sha256").update(submitted).digest("hex");
  await api("editor/map", { sessionId: target.sessionId, source: submitted, sourceHash, serial: c.source.serial });
  const restored = await api("history/restore", { resource: target.resource, sessionId: target.sessionId, baseId: target.baseId,
    source: submitted, sourceHash, serial: c.source.serial, versionId: config.versionId, operationId: crypto.randomUUID(), inputRevision: original.inputRevision, draftAction: "retain" });
  assert.equal(restored.status, "committed");
  await assert.rejects(c.open("Other"));
  assert.equal(c.doc.resource, "Other");
  c.token = "another-current-credential";
  await c.retryHostedRetirements();
  assert.deepEqual([...c.hostedWriterRetirements.keys()].sort(), [previous.sessionId, target.sessionId].sort());
  assert.ok(effects.length >= 2);
  for (const effect of effects) {
    assert.equal(effect.path, "/api/hosted/draft", "a refused original cut must not close either writer");
    assert.equal(effect.status, 409); assert.deepEqual(effect.body, original, "consumed-cut cleanup cannot issue revision 2");
  }
  for (const entry of c.hostedWriterRetirements.values()) {
    assert.equal(entry.audience.credential === config.token, true);
  }
  assert.equal(c.hostedWriterRetirements.get(previous.sessionId).waitFor.transmission.inputRevision, 1);
  return { effects, submitted, restoredSource: restored.result.source, original, retirementCount: c.hostedWriterRetirements.size };
}

const result = config.scenario === "navigationRetry" ? await navigationRetry() : config.scenario === "repeatedSave" ? await repeatedSave() : config.scenario === "consumedCut" ? await consumedCut() : assert.fail("unknown hosted regression scenario");
process.stdout.write(JSON.stringify(result));
