import { message, uiTextValue, errorUiText, locale } from "./ui-locale";
import { ApiError, apiErrorUiText } from "./api-error";
import { coreDescriptionUiText } from "./ui-diagnostics";
import test from "node:test";
import assert from "node:assert/strict";
import { fileSaveReceipt, fileSaveFailure } from "./file-save-result";
import { submitOriginal, TerminatedAttempt, deliverCommitted, committedDeliveryComplete, originalDeliveryErrors, type OriginalRequest } from "./original-operation";
import { DocumentSession, rebindCommittedSave } from "./session";

const frozen = { operationId: "original", sessionId: "writer", source: "submitted S", sourcePath: "C:/TEST/a.typ", baseId: "before" };
const document = { sessionId: frozen.sessionId, baseId: "actual", path: frozen.sourcePath, root: "C:/TEST", source: frozen.source, kind: "ordinary" };
const receipt = { operationId: frozen.operationId, status: "committed", baseId: "original-base", path: frozen.sourcePath, committedSource: frozen.source, document, documentSourceMatchesCommit: true };

test("resolved original save preserves its typed rejection instead of asking to confirm a cleared attempt", async () => {
  const rejection = new ApiError("invalidAuthorSource", message("error.invalidResponse"));
  const session = new DocumentSession({ ...document, source: "before B", baseId: "before" });
  session.edit("retained author input\r\n😀");
  const inputId = session.inputId, calls: string[] = [];
  let lastSave: typeof frozen | undefined = { ...frozen };
  const request: OriginalRequest = async <T>(route: string, body: unknown) => {
    calls.push(route);
    if (route === "/api/save") throw rejection;
    if (route === "/api/operations/status") return { operationId: frozen.operationId, status: "unknown" } as T;
    assert.equal(route, "/api/operations/settle");
    assert.equal((body as { operationId: string }).operationId, frozen.operationId);
    return { operationId: frozen.operationId, status: "resolved" } as T;
  };
  await assert.rejects(submitOriginal(request, "/api/save", { ...frozen }, "save"), error => {
    const failure = fileSaveFailure(error, false, !!lastSave);
    if (failure.clearAttempt) lastSave = undefined;
    assert.equal(lastSave, undefined);
    assert.equal(uiTextValue(failure.error), uiTextValue(errorUiText(rejection)));
    assert.notEqual(uiTextValue(failure.error), uiTextValue(message("shell.saveUnknownConfirmSameOperation")));
    return true;
  });
  assert.deepEqual(calls, ["/api/save", "/api/operations/status", "/api/operations/settle"]);
  assert.equal(session.source, "retained author input\r\n😀"); assert.equal(session.inputId, inputId); assert.equal(session.baseId, "before");
});

test("unconfirmed save retains the original attempt and input and retry only reads its original result", async () => {
  const unknown = new Error("connection ended"), calls: string[] = [];
  const lastSave = { ...frozen }, before = JSON.stringify(lastSave);
  const request: OriginalRequest = async <T>(route: string, body: unknown) => {
    calls.push(route); assert.equal((body as { operationId: string }).operationId, frozen.operationId);
    if (route === "/api/save") throw unknown;
    return { operationId: frozen.operationId, status: "unknown" } as T;
  };
  for (const retry of [false, true]) await assert.rejects(submitOriginal(request, "/api/save", lastSave, "save", retry), error => {
    const failure = fileSaveFailure(error, false, true);
    assert.equal(failure.clearAttempt, false);
    assert.equal(uiTextValue(failure.error), uiTextValue(message("shell.saveUnknownConfirmSameOperation")));
    return true;
  });
  assert.equal(calls.filter(route => route === "/api/save").length, 1);
  assert.equal(JSON.stringify(lastSave), before);
});

test("unresolved typed rejection and committed delivery failure preserve their separate presentation", async () => {
  const rejection = new ApiError("invalidAuthorSource", message("error.invalidResponse"));
  const request: OriginalRequest = async <T>(route: string) => {
    if (route === "/api/save") throw rejection;
    return { operationId: frozen.operationId, status: "prepared" } as T;
  };
  await assert.rejects(submitOriginal(request, "/api/save", { ...frozen }, "save"), error => {
    assert.equal(error, rejection);
    const failure = fileSaveFailure(error, false, true);
    assert.equal(failure.clearAttempt, false);
    assert.equal(uiTextValue(failure.error), uiTextValue(errorUiText(rejection)));
    return true;
  });
  const committed = await submitOriginal((async () => receipt) as OriginalRequest, "/api/save", { ...frozen }, "save");
  assert.equal(committed.status, "committed");
  const failure = fileSaveFailure(new Error("delivery failed"), true, true);
  assert.equal(failure.clearAttempt, false);
  assert.equal(uiTextValue(failure.error), uiTextValue(message("shell.committedDeliveryIncomplete")));
  assert.equal(committed.operationId, frozen.operationId); assert.equal(frozen.source, "submitted S");
  const contradictory = fileSaveFailure(new TerminatedAttempt(rejection), true, true);
  assert.equal(contradictory.clearAttempt, false);
  assert.equal(uiTextValue(contradictory.error), uiTextValue(message("shell.committedDeliveryIncomplete")));
});
const malformed = [
  { baseId: undefined }, { baseId: "" }, { baseId: 12 }, { path: undefined }, { path: 12 }, { path: "C:/TEST/other.typ" },
  { operationId: "other" }, { status: "complete" }, { status: ["saved"] }, { message: "old message protocol" },
  { document: undefined }, { document: { ...document, sessionId: undefined } }, { document: { ...document, baseId: 1 } },
  { document: { ...document, kind: ["ordinary"] } }, { document: { ...document, recoveredDraftConflict: "false" } },
  { committedSource: "other submission" }, { documentSourceMatchesCommit: undefined }, { documentSourceMatchesCommit: false },
  { documentError: {} }, { documentError: { code: "io", message: "unavailable" } },
];

test("file delivery keeps complete Draft provenance and rejects malformed baseline metadata", () => {
  const variant = { path: document.path, baseSource: "B", source: "S", baseKnown: true, producerId: "original-producer", role: {kind: "committedBaseline", operationId: "original-save"} };
  const withVariant = (value: unknown) => ({...receipt, document: {...document, recoveredDraftVariants: [value]}});
  assert.deepEqual(fileSaveReceipt(withVariant(variant), frozen).document?.recoveredDraftVariants, [variant]);
  for (const role of [{kind: "unknown"}, {kind: "proposal", operationId: "extra"}, {kind: "committedBaseline"}, {kind: "committedBaseline", operationId: ""}])
    assert.throws(() => fileSaveReceipt(withVariant({...variant, role}), frozen));
  assert.throws(() => fileSaveReceipt(withVariant({...variant, producerId: undefined}), frozen));
  assert.throws(() => fileSaveReceipt(withVariant({...variant, producerId: 12}), frozen));
});
test("initial and queried file saves share the exact original receipt and qualified-delivery contract", () => {
  for (const status of ["saved", "committed"]) {
    assert.equal(fileSaveReceipt({ ...receipt, status }, frozen).document?.baseId, "actual");
    for (const change of malformed) assert.throws(() => fileSaveReceipt({ ...receipt, status, ...change }, frozen));
  }
  const {document: _document, documentSourceMatchesCommit: _match, ...originalFact} = receipt;
  const unavailable = { ...originalFact, documentError: { code: "sourceIdentityChanged", explanation: message("shell.savedBasisChangedCheckDisk"), diagnostics: [], causes: [] } };
  assert.equal(fileSaveReceipt(unavailable, frozen).baseId, receipt.baseId);
  assert.equal(fileSaveReceipt({ ...receipt, document: { ...document, source: "later T" }, documentSourceMatchesCommit: false }, frozen).document?.source, "later T");
  assert.throws(() => fileSaveReceipt(receipt, { ...frozen, path: "C:/TEST/save-as.typ" }));
  assert.equal(fileSaveReceipt(receipt, { ...frozen, path: "c:\\TEST\\folder\\..\\a.typ" }).path, frozen.sourcePath);
});
test("bad initial and status file receipts preserve original attempt and input; later valid status heals without another author request", async () => {
  for (const change of malformed.filter(change => !("operationId" in change) && !("status" in change))) {
    let authors = 0, lookups = 0;
    const malformedAtTypedBoundary = Object.hasOwn(change,"documentError");
    const badLookups = malformedAtTypedBoundary ? 2 : 1;
    const session = new DocumentSession({ ...document, sessionId: "writer", source: "before B", baseId: "before" });
    session.edit("submitted S + input Y");
    const request: OriginalRequest = async <T>(route: string) => {
      if (route === "/api/save") { authors++; return { ...receipt, status: "saved", ...change } as T; }
      assert.equal(route, "/api/operations/status"); lookups++;
      return (lookups <= badLookups ? { ...receipt, ...change } : receipt) as T;
    };
    const payload = { operationId: frozen.operationId, sessionId: frozen.sessionId, baseId: frozen.baseId, source: frozen.source };
    for (const recoverFirst of [false, true]) {
      await assert.rejects(async () => fileSaveReceipt(await submitOriginal(request, "/api/save", payload, "save", recoverFirst), frozen));
      assert.equal(session.baseId, "before"); assert.equal(session.source, "submitted S + input Y");
    }
    const good = fileSaveReceipt(await submitOriginal(request, "/api/save", payload, "save", true), frozen);
    assert.equal(rebindCommittedSave(session, frozen, good.document!), true);
    assert.equal(session.baseId, "actual"); assert.equal(session.source, "submitted S + input Y");
    assert.equal(authors, 1); assert.equal(lookups, badLookups + 1);
  }
});

test("the actual file-save DTO keeps full Core recovery and API delivery causes with closed separate domains", () => {
  const core = {code:"draftWarning",explanation:message("shell.savedRecoveryUpdateUnavailable"),diagnostics:[{origin:"os" as const,text:"raw OS recovery"}],causes:[{code:"original",explanation:message("common.operation",{operationId:"original"}),diagnostics:[]}],currentSource:"exact recovery source\r\n😀",details:{kind:"aggregate" as const,groups:[{kind:"product" as const,code:"parallel",diagnostics:[{explanation:message("common.cancel"),diagnostics:[],causes:[]}]}]}};
  const api = {code:"sourceIdentityChanged",explanation:message("shell.savedBasisChangedCheckDisk"),diagnostics:[],causes:[],details:{kind:"core" as const,...core}};
  const good = fileSaveReceipt({...receipt,recoveryIssue:structuredClone(core)},frozen);
  assert.deepEqual(good.recoveryIssue,core);
  const unavailable = fileSaveReceipt({...receipt,document:undefined,documentError:structuredClone(api),recoveryIssue:core},frozen);
  assert.deepEqual(unavailable.documentError,api);assert.deepEqual(unavailable.recoveryIssue,core);assert.equal(unavailable.status,"committed");
  locale.set("en-US");assert.ok(uiTextValue(apiErrorUiText(unavailable.documentError!)).includes("raw OS recovery"));assert.ok(uiTextValue(coreDescriptionUiText({explanation:good.recoveryIssue!.explanation,diagnostics:good.recoveryIssue!.diagnostics,causes:good.recoveryIssue!.causes,details:good.recoveryIssue!.details})).includes("raw OS recovery"));locale.set("zh-CN");
  for(const change of [{recoveryIssue:{code:"old",message:"old"}},{recoveryIssue:{...core,details:{kind:"core",...core}}},{document:undefined,documentError:{code:"old",message:"old"}},{message:"old"},{revisionId:null},{recoveryIssue:null}]) assert.throws(()=>fileSaveReceipt({...receipt,...change},frozen));
});

test("the actual malformed-save delivery explanation changes language without changing the committed receipt or invoking delivery again", async () => {
  const original={...receipt,document:{...document}};const before=JSON.stringify(original);let attempts=0;
  const delivered=await deliverCommitted(original,async()=>{attempts++;fileSaveReceipt({...original,operationId:"another-operation"},frozen);});
  assert.equal(committedDeliveryComplete(delivered),false);assert.equal(delivered.operationId,frozen.operationId);assert.equal(delivered.document.source,frozen.source);
  const labels=originalDeliveryErrors(delivered);assert.equal(labels.length,1);
  try {
    locale.set("zh-CN");assert.match(uiTextValue(labels[0]),/原编号和输入保留/);
    locale.set("en-US");assert.match(uiTextValue(labels[0]),/original ID and input are retained/);
    assert.equal(attempts,1);assert.equal(JSON.stringify(original),before);assert.equal(JSON.stringify(delivered),before);assert.equal(delivered.document.sessionId,document.sessionId);
  } finally {locale.set("zh-CN");}
});
