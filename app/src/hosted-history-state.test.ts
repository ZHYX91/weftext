import ts from "typescript";
import vm from "node:vm";
import { readFileSync } from "node:fs";
import { OrderedDraft } from "./ordered-draft";
import { UiError } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { currentHostedHistoryResult, hostedHistoryPending, hostedHistoryStatus, resumeHostedHistoryRecord, unavailableCommittedHistory, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { restoreBufferDisposition, historySourceDiff, historyRecoveryIssueUiText } from "./history-state";
import { apiErrorUiText, decodeApiErrorBody } from "./api-error";
import { locale, message, uiTextValue } from "./ui-locale";
import { createSourceState } from "./editor";
import { history, isolateHistory, undo, undoDepth } from "@codemirror/commands";

const record: HostedHistoryRecord = {
  request: { resource: "owner", sessionId: "session", baseId: "baseC", versionId: "versionA", operationId: "restore", source: "C😀\r\n", serial: 3, sourceHash: "hashC", draftAction: "retain" },
  basis: { inputId:"00000000-0000-4000-8000-000000000001", resource: "owner", path: "resource:owner", root: "", nodeId: "node", workspaceId: "workspace", sessionId: "session", baseId: "baseC", baseSource: "C😀\r\n", source: "C😀\r\n", serial: 3, dirty: false, selectedVersionId: "versionA", selectedSource: "A\r\n" },
};
const committed: HostedHistoryStatus = { resource: "owner", operationId: "restore", status: "committed", resultStatus: "available", resultBasis: "current", currentRevision: "original-revision", result: { receipt: { operationId: "restore", status: "saved", path: "resource:owner", baseId: "baseA", revisionId: "original-revision" }, source: "A\r\n", versionId: "versionA", protectedDrafts: [{ path: "resource:owner", source: "D😀\r\n", baseSource: "C😀\r\n", baseKnown: true }] } };

test("a restore receipt proves the original revision as well as bytes; superseded/no-op ABA and external equal bytes cannot bind current", () => {
  assert.equal(currentHostedHistoryResult(record, "A\r\n", committed), true);
  assert.equal(currentHostedHistoryResult(record, "F\r\n", committed), false);
  for (const changed of [{ currentRevision: "later-equal-byte-revision" }, { resultBasis: "superseded" }, { resultBasis: "external", currentRevision: null }]) assert.equal(currentHostedHistoryResult(record, "A\r\n", { ...committed, ...changed } as HostedHistoryStatus), false);
  assert.throws(() => hostedHistoryStatus(record, { ...committed, operationId: "other" }));
  const bad = hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, receipt: { ...committed.result!.receipt, path: "D:/private/server/owner.typ" } } });
  assert.equal(bad.status, "committed"); assert.equal(bad.result, undefined); assert.ok(bad.guardReason); assert.equal(bad.resultError, undefined);
});
test("hosted committed facts survive malformed results, downgrade packets and reconnect without reviving cached delivery rights", () => {
  const active = structuredClone(record);
  for (const change of [{ operationId: "other" }, { baseId: "" }, { baseId: 42 }, { path: "D:/private/secret.typ" }, { status: "resolved" }, { message: null }, { revisionId: undefined }, { revisionId: "" }, { revisionId: 42 }]) {
    const incoming = structuredClone(committed); Object.assign(incoming.result!.receipt, change);
    active.status = hostedHistoryStatus(active, incoming);
    assert.equal(active.status.status, "committed"); assert.equal(active.status.receipt, undefined);
    assert.equal(active.status.result, undefined); assert.ok(active.status.guardReason); assert.equal(active.status.resultError, undefined); assert.equal(hostedHistoryPending(active), true);
    const recovered = hostedHistoryStatus(active, committed);
    assert.equal(recovered.resultStatus, "available"); assert.equal(recovered.receipt?.revisionId, "original-revision");
  }
  active.status = hostedHistoryStatus(active, committed);
  const original = structuredClone(active.status.receipt);
  for (const status of ["notRegistered", "notCommitted", "recoveryPending", "resolved", "cancelled"] as const) {
    active.status = hostedHistoryStatus(active, { operationId: "restore", resource: "owner", status, resultStatus: "unavailable" });
    assert.equal(active.status.status, "committed"); assert.deepEqual(active.status.receipt, original);
    assert.equal(active.status.result, undefined); assert.equal(active.status.resultBasis, undefined);
    assert.equal(active.status.currentRevision, undefined); assert.equal(hostedHistoryPending(active), true);
  }
  active.status = hostedHistoryStatus(active, committed);
  const resumed = resumeHostedHistoryRecord(active);
  assert.equal(resumed.status?.status, "committed"); assert.deepEqual(resumed.status?.receipt, original);
  assert.equal(resumed.status?.result, undefined); assert.equal(resumed.status?.resultBasis, undefined);
  resumed.status = hostedHistoryStatus(resumed, { ...committed, resource: "other" });
  assert.equal(hostedHistoryPending(resumed), true); assert.equal(resumed.status.result, undefined);
  resumed.status = hostedHistoryStatus(resumed, committed);
  assert.equal(currentHostedHistoryResult(resumed, "A\r\n", resumed.status), true);
  assert.deepEqual(resumed.request, record.request); assert.deepEqual(resumed.basis, record.basis);
});

test("a real scoped delivery ApiError retains its code, causes and nested Core facts after JSON while client guards remain separate", () => {
  const rawDescription = { explanation: message("common.close"), diagnostics: [{ origin: "dependency", text: "original raw 错误😀\r\n" }], causes: [] };
  const rawError = { code: "deliveryFailed", explanation: message("common.save"), diagnostics: rawDescription.diagnostics, causes: [{code:"originalCause",explanation:rawDescription.explanation,diagnostics:rawDescription.diagnostics}], details: { kind: "core", code: "coverage", ...rawDescription, details: { kind: "aggregate", groups: [{ kind: "product", code: "exactOriginal", diagnostics: [rawDescription] }] } } };
  const error = decodeApiErrorBody(rawError)!; assert.ok(error);
  const prior = { ...record, status: hostedHistoryStatus(record, committed) }, incoming = JSON.parse(JSON.stringify({ operationId: "restore", resource: "owner", status: "committed", resultStatus: "unavailable", resultError: rawError }));
  const status = hostedHistoryStatus(prior, incoming);
  assert.equal(status.status, "committed"); assert.equal(status.result, undefined); assert.equal(status.guardReason, undefined); assert.equal(status.resultError!.code, "deliveryFailed");
  assert.deepEqual(status.resultError!.details, error.details); assert.deepEqual(status.resultError!.causes, error.causes); assert.equal(status.receipt!.revisionId, "original-revision");
  locale.set("en-US"); assert.match(uiTextValue(apiErrorUiText(status.resultError!)), /Save/); assert.match(uiTextValue(apiErrorUiText(status.resultError!)), /original raw 错误😀\r\n/); locale.set("zh-CN");
  const invalid = hostedHistoryStatus(prior, { ...incoming, resultError: "old untyped error" });
  assert.equal(invalid.status, "committed"); assert.ok(invalid.guardReason); assert.equal(invalid.resultError, undefined); assert.equal(invalid.result, undefined);
  const resumed = resumeHostedHistoryRecord({ ...prior, status }); assert.equal(resumed.status!.resultError, undefined); assert.equal(resumed.status!.result, undefined); assert.deepEqual(resumed.request, record.request);
});

test("History recovery warnings use the CoreError domain including its aggregate details and exact current source", () => {
  const authored = '\uFEFF作者😀\r\n{"key":"common.save","params":{}}', description = { explanation: message("common.close"), diagnostics: [{ origin: "dependency" as const, text: "raw tool 原文\r\n" }], causes: [] };
  const core = { code: "draftProtection", ...description, currentSource: authored, details: { kind: "aggregate" as const, groups: [{ kind: "product" as const, code: "originalFacts", diagnostics: [description] }] } };
  const displayed = historyRecoveryIssueUiText(JSON.parse(JSON.stringify(core)));
  locale.set("en-US"); assert.match(uiTextValue(displayed), /Close/); assert.ok(uiTextValue(displayed).includes(authored)); assert.match(uiTextValue(displayed), /originalFacts/); locale.set("zh-CN");
  assert.equal(core.currentSource, authored);
  const safeIssue = { code: "draftProtection", explanation: message("common.close"), diagnostics: [], causes: [] };
  const available = hostedHistoryStatus(record, JSON.parse(JSON.stringify({ ...committed, result: { ...committed.result!, recoveryIssue: safeIssue } })));
  assert.equal(available.resultStatus, "available"); assert.equal(uiTextValue(historyRecoveryIssueUiText(available.result!.recoveryIssue!)), "关闭");
  for (const issue of [{ code: "old", message: "old warning" }, { ...safeIssue, details: { kind: "currentSource", currentSource: authored } }]) {
    const invalid = hostedHistoryStatus(record, { ...committed, result: { ...committed.result!, recoveryIssue: issue } } as unknown as HostedHistoryStatus);
    assert.equal(invalid.status, "committed"); assert.equal(invalid.result, undefined); assert.ok(invalid.guardReason);
  }
});
test("committed unavailable drops earlier source and has an explicit comparison-only exit; unknown operations cannot use it", () => {
  const unavailable = hostedHistoryStatus({ ...record, status: committed }, { resource: "owner", operationId: "restore", status: "committed", resultStatus: "unavailable" });
  assert.equal(unavailable.result, undefined); assert.equal(unavailableCommittedHistory({ ...record, status: unavailable }), true);
  assert.equal(hostedHistoryPending({ ...record, status: unavailable }), true);
  assert.equal(hostedHistoryPending({ ...record, status: unavailable, acknowledged: true }), false);
  for (const status of ["notRegistered", "notCommitted", "recoveryPending", "resolved", "cancelled"] as const) assert.equal(unavailableCommittedHistory({ ...record, status: { ...unavailable, status } }), false);
  assert.equal(hostedHistoryPending({ ...record, status: { ...unavailable, status: "resolved" } }), false);
  assert.equal(hostedHistoryPending({ ...record, status: { ...unavailable, status: "cancelled" } }), false);
});
test("clean restore is one real shared Undo; dirty and late input remain author drafts and fixed comparisons reconstruct both originals", () => {
  assert.equal(restoreBufferDisposition(record.basis, record.basis), "replaceClean");
  assert.equal(restoreBufferDisposition({ ...record.basis, source: "D", dirty: true }, { ...record.basis, source: "E", serial: 4, dirty: true }), "retainDraft");
  let state = createSourceState(record.basis.source, [history()]);
  state = state.update({ changes: { from: 0, to: state.doc.length, insert: committed.result!.source }, userEvent: "input.history.restore", annotations: isolateHistory.of("full") }).state;
  assert.equal(undoDepth(state), 1); const target = { get state() { return state; }, dispatch(transaction: { state: typeof state }) { state = transaction.state; } }; assert.equal(undo(target), true); assert.equal(state.doc.toString(), record.basis.source);
  const parts = historySourceDiff("A\r\n", "D😀\r\n"); assert.equal(parts.filter(part => part.kind !== "added").map(part => part.text).join(""), "A\r\n"); assert.equal(parts.filter(part => part.kind !== "removed").map(part => part.text).join(""), "D😀\r\n");
});

// Execute the real hosted consumer and ACK queue; replace HTTP and UI ports.
const hostedSource=ts.createSourceFile("hosted.ts",readFileSync(new URL("./hosted.ts",import.meta.url),"utf8"),ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
const consumerNames=["acknowledgeHistory","readDocumentReference","publishDocumentReference","queueHostedWriterRetirement","retireHostedWriters","hostedInput","draftOrder","bindHostedTransmission","issueHostedInput","persistHostedDraft"];
const hostedFunctions=hostedSource.statements.filter(node=>ts.isFunctionDeclaration(node)&&consumerNames.includes(node.name?.text??""));
assert.equal(hostedFunctions.length,consumerNames.length);
const hostedConsumer=ts.transpileModule(hostedFunctions.map(node=>node.getText(hostedSource)).join("\n"),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
function historyConsumerFixture(later=false) {
 const original={resource:"owner",sessionId:"session",baseId:"baseC",viewInputId:"00000000-0000-4000-8000-000000000001",source:"C😀\r\n",rights:{write:true}};
 const fresh={...original,sessionId:original.sessionId,baseId:"baseA",source:"A\r\n"};
 const calls:any[]=[],closed:string[]=[],closeAudiences:any[]=[], elements=new Map<string,any>(),references=new Map<string,string>(),producers:any[]=[],historyApplications:string[]=[];
 let referenceReply:Promise<any>|undefined,referenceStarted!:()=>void;
 const started=new Promise<void>(resolve=>{referenceStarted=resolve;});
 let rejectAck=false;
 const source={value:later?"new Y":"C😀\r\n",serial:later?4:3,authorInputId:later?crypto.randomUUID():"00000000-0000-4000-8000-000000000001",disabled:false,
  flush:async()=>{},open(session:string,resource:string,value:string){assert.ok(references.has(resource),"history must bind the producer before opening it");producers.push({session,resource,value,reference:references.get(resource)});this.value=value;},refreshQueries(){},
  personalInputReference(resource:string,reference:string){references.set(resource,reference);},
  applyHistorySource(value:string){historyApplications.push(value);this.value=value;this.serial++;}};
 const context:any={doc:original,draft:{source:source.value,baseSource:original.source,basis:{sessionId:original.sessionId,baseId:original.baseId}},source,requestEpoch:1,authorizationEpoch:0,compareRequired:false,
  token:"original",identity:{actor:"alice",deployment:"space",generation:0},AbortSignal,
  OrderedDraft,orderedDrafts:new WeakMap(),hostedWriterRetirements:new Map(),currentHostedHistoryResult,restoreBufferDisposition,
  uiMessage:message,UiError,HostedError:class extends Error{},sessionAudience:()=>({credential:context.token,authorization:context.authorizationEpoch,actor:context.identity.actor,deployment:context.identity.deployment,generation:context.identity.generation}),
  closeOwnedSession:async(id:string,audience:any)=>{closed.push(id);closeAudiences.push(audience);},
  el:(id:string)=>{if(!elements.has(id))elements.set(id,{});return elements.get(id);},setUiText(){},retain(){},controls(){},
  history:{installStatus:(original:HostedHistoryRecord,incoming:HostedHistoryStatus)=>hostedHistoryStatus(original,incoming),documentChanged(){ }},
  queries:{authorChanged(){},refreshCatalog:async()=>{}},knowledgeAuthorChanged(){},
  request:async(path:string,body:any,bytes?:boolean,attempt?:number,signal?:AbortSignal,scope?:string)=>{calls.push([path,body]);if(path==="open")return fresh;if(path==="navigation/ref"){assert.equal(body.sessionId,original.sessionId);assert.equal(bytes,false);assert.equal(attempt,0);assert.equal(scope,"stagedTarget");assert.ok(signal instanceof AbortSignal);referenceStarted();return referenceReply??{resource:original.resource,reference:"weftext-node://workspace/original"};}if(path==="history/status")return committed;if(path==="draft"){if(rejectAck)throw new Error("ACK delivery failed");return {status:"inputAcknowledged",evidence:"authorDraft",inputRevision:body.inputRevision};}throw new Error(`unexpected ${path}`);},
 };
 vm.createContext(context);vm.runInContext(hostedConsumer,context);
 return {context,original,fresh,calls,closed,closeAudiences,elements,references,producers,historyApplications,started,delayReference:(reply:Promise<any>)=>{referenceReply=reply;},reject:(value:boolean)=>{rejectAck=value;}};
}
test("real hosted restore consumer preserves Y on its original Base and persists that basis before retiring the old writer",async()=>{
 const f=historyConsumerFixture(true), original=structuredClone(record);original.status=structuredClone(committed);
 assert.equal(await f.context.acknowledgeHistory(original),"superseded");assert.equal(f.context.source.value,"new Y");assert.equal(f.context.draft.baseSource,"C😀\r\n");assert.equal(f.context.compareRequired,true);
 const input=f.calls.find(([path])=>path==="draft")[1];assert.equal(input.source,"new Y");assert.equal(input.retainedBaseSource,undefined);assert.equal(input.baseId,"baseC");assert.equal(f.calls.some(([path])=>path==="draft/resolve"),false);assert.deepEqual(f.closed,[]);
});
test("real hosted clean restore installs the committed output and acknowledges the target before writer retirement",async()=>{
 const f=historyConsumerFixture(),original=structuredClone(record);original.status=structuredClone(committed);
 assert.equal(await f.context.acknowledgeHistory(original),"applied");assert.equal(f.context.source.value,"A\r\n");assert.equal(f.context.draft.baseSource,"A\r\n");assert.equal(f.context.compareRequired,false);
 const input=f.calls.find(([path])=>path==="draft")[1];assert.equal(input.baseId,"baseA");assert.equal(input.retainedBaseSource,undefined);assert.deepEqual(f.closed,[]);
});
test("a late history identity leaves the original source and producer installed until its exact target reference is ready",async()=>{
 const f=historyConsumerFixture(),original=structuredClone(record);original.status=structuredClone(committed);
 let release!:(value:any)=>void;f.delayReference(new Promise(resolve=>{release=resolve;}));
 const pending=f.context.acknowledgeHistory(original);await f.started;
 assert.equal(f.context.doc,f.original);assert.equal(f.context.source.value,"C😀\r\n");assert.deepEqual(f.producers,[]);assert.deepEqual(f.historyApplications,[]);assert.equal(f.references.size,0);
 release({resource:"owner",reference:"weftext-node://workspace/original"});assert.equal(await pending,"applied");
 assert.deepEqual(f.historyApplications,["A\r\n"]);assert.deepEqual(f.producers,[{session:"session",resource:"owner",value:"A\r\n",reference:"weftext-node://workspace/original"}]);
 assert.equal(f.context.stableReference.reference,"weftext-node://workspace/original");
});
test("typing while a history reference is delayed retains the new author input on its original Base",async()=>{
 const f=historyConsumerFixture(),original=structuredClone(record);original.status=structuredClone(committed);
 let release!:(value:any)=>void;f.delayReference(new Promise(resolve=>{release=resolve;}));
 const pending=f.context.acknowledgeHistory(original);await f.started;
 Object.assign(f.context.source,{value:"new Y while reference waits 😀",serial:4,authorInputId:crypto.randomUUID()});
 release({resource:"owner",reference:"weftext-node://workspace/original"});assert.equal(await pending,"superseded");
 assert.equal(f.context.source.value,"new Y while reference waits 😀");assert.deepEqual(f.historyApplications,[]);assert.equal(f.context.draft.baseSource,"C😀\r\n");assert.equal(f.context.compareRequired,true);
 const input=f.calls.find(([path])=>path==="draft")[1];assert.equal(input.source,"new Y while reference waits 😀");assert.equal(input.baseId,"baseC");assert.equal(input.retainedBaseSource,undefined);
});
test("a late history reference cannot lend Alice's producer to Bob and staged cleanup retains Alice's audience",async()=>{
 const f=historyConsumerFixture(),original=structuredClone(record);original.status=structuredClone(committed);f.fresh.sessionId="alice-staged-history";
 let release!:(value:any)=>void;f.delayReference(new Promise(resolve=>{release=resolve;}));
 const pending=f.context.acknowledgeHistory(original),rejected=assert.rejects(pending);await f.started;
 const bob={resource:"Other",sessionId:"bob-current"};f.context.doc=bob;f.context.token="bob-credential";f.context.identity={actor:"bob",deployment:"space",generation:1};f.context.authorizationEpoch++;f.context.requestEpoch++;
 release({resource:"owner",reference:"weftext-node://workspace/original"});await rejected;
 assert.equal(f.context.doc,bob);assert.equal(f.context.source.value,"C😀\r\n");assert.deepEqual(f.producers,[]);assert.deepEqual(f.historyApplications,[]);assert.equal(f.references.size,0);
 assert.deepEqual(f.closed,["alice-staged-history"]);assert.equal(f.closeAudiences[0].actor,"alice");assert.equal(f.closeAudiences[0].credential,"original");assert.equal(f.context.token,"bob-credential");
});
test("history reference rejection or final retention failure keeps the original source and producer",async()=>{
 for(const failure of ["network","identity","retention"] as const){
  const f=historyConsumerFixture(),original=structuredClone(record);original.status=structuredClone(committed);
  let release!:(value:any)=>void,reject!:(reason:unknown)=>void;f.delayReference(new Promise((resolve,fail)=>{release=resolve;reject=fail;}));
  if(failure==="retention")f.context.retain=()=>{throw new Error("history final retention failed");};
  const pending=f.context.acknowledgeHistory(original),rejected=assert.rejects(pending);await f.started;
  if(failure==="network")reject(new Error("history reference unavailable"));
  else release({resource:failure==="identity"?"unrelated":"owner",reference:"weftext-node://workspace/original"});
  await rejected;assert.equal(f.context.doc,f.original);assert.equal(f.context.source.value,"C😀\r\n");assert.deepEqual(f.producers,[]);assert.deepEqual(f.historyApplications,[]);assert.equal(f.references.size,0);assert.deepEqual(f.closed,[]);
 }
});
test("failed target ACK keeps the old hosted writer handle until the same target's actual ACK succeeds",async()=>{
 const f=historyConsumerFixture(true),original=structuredClone(record);original.status=structuredClone(committed);f.reject(true);
 await assert.rejects(f.context.acknowledgeHistory(original),/ACK delivery failed/);assert.deepEqual(f.closed,[]);assert.equal(f.context.doc.sessionId,"session");
 f.reject(false);await f.context.persistHostedDraft(f.context.doc);assert.deepEqual(f.closed,[]);assert.equal(f.context.hostedWriterRetirements.size,0);assert.equal(f.context.draft.baseSource,"C😀\r\n");
});
