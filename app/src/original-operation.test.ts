import { message, uiTextValue } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { submitOriginal, committedDeliveryComplete, deliverCommitted, originalDeliveryErrors, TerminatedAttempt, type OriginalRequest } from "./original-operation";
const payload = { operationId: "original", source: "frozen" };
const digest = (wire: string) => createHash("sha256").update(wire).digest("hex");
const transport = (run: (path: string, body: unknown) => unknown): OriginalRequest => async <T>(path: string, body: unknown) => await run(path, body) as T;
test("ordinary postcommit error recovers the original receipt without another author request", async () => {
  const calls: string[]=[];
  const result=await submitOriginal(transport((path,body)=>{calls.push(path);if(path==="/save")throw Object.assign(new Error("draft delivery failed"),{code:"invalidState"});assert.deepEqual(body,{operationId:"original"});return {operationId:"original",status:"committed",baseId:"new"};}),"/save",payload,"save");
  assert.equal(result.status,"committed");assert.deepEqual(calls,["/save","/api/operations/status"]);
});
test("missing original releases only after the durable fence; damaged/unknown results preserve it", async () => {
  const calls:string[]=[];
  await assert.rejects(submitOriginal(transport((path,body)=>{calls.push(path);if(path==="/save")throw new Error("rejected");if(path.endsWith("status"))return {operationId:"original",status:"unknown"};assert.deepEqual(body,{operationId:"original",requestKind:"save",wireDigest:digest(JSON.stringify(payload))});return {operationId:"original",status:"resolved"};}),"/save",payload,"save"),TerminatedAttempt);
  assert.deepEqual(calls,["/save","/api/operations/status","/api/operations/settle"]);
  await assert.rejects(submitOriginal(transport(path=>{if(path.endsWith("status"))return {operationId:"original",status:"unknown"};throw new Error("fence unavailable");}),"/save",payload,"save",true),/fence unavailable/);
});
test("retry queries original before reinterpreting input and rejects another operation receipt", async () => {
  const calls:string[]=[];
  const result=await submitOriginal(transport(path=>{calls.push(path);return {operationId:"original",status:"committed"};}),"/capture",payload,"capture",true);
  assert.equal(result.status,"committed");assert.deepEqual(calls,["/api/operations/status"]);
  await assert.rejects(submitOriginal(transport(()=>({operationId:"other",status:"committed"})),"/capture",payload,"capture",true),/原操作/);
});
test("callback failure preserves receipt and is separately retryable delivery",async()=>{
  const receipt={operationId:"original",status:"committed",path:"fixed"};
  const result=await deliverCommitted(receipt,async()=>{throw Object.assign(new Error("refresh failed"),{code:"conflict"});});
  assert.deepEqual(receipt,{operationId:"original",status:"committed",path:"fixed"});assert.equal(result.status,"committed");assert.equal(result.deliveryError,undefined);assert.deepEqual(originalDeliveryErrors(result).map(uiTextValue),["refresh failed"]);assert.deepEqual(JSON.parse(JSON.stringify(result)),receipt);
  assert.equal(committedDeliveryComplete(result),false);
  let callbacks=0;const recovered=await deliverCommitted(result,async value=>{callbacks++;assert.equal(value.operationId,"original");assert.equal(value.path,"fixed");});
  assert.equal(callbacks,1);assert.equal(committedDeliveryComplete(recovered),true);assert.equal(recovered.deliveryError,undefined);
  const blocked=await deliverCommitted({...receipt,documentError:{code:"originalUnavailable",explanation:message("shell.savedBasisChangedCheckDisk"),diagnostics:[],causes:[]}},async()=>{throw Error("must not consume another document");});
  assert.equal(committedDeliveryComplete(blocked),false);assert.equal(blocked.status,"committed");
});

test("a rejected surrogate and later mutable input cannot change the original wire or fence", async () => {
  const input = { operationId: "original", source: "rejected \ud800", fields: { value: "frozen" } }, wire = JSON.stringify(input);
  const calls: string[] = [];
  await assert.rejects(submitOriginal(transport(async (path, body) => {
    calls.push(path);
    if (path === "/save") {
      assert.equal(JSON.stringify(body), wire);
      input.operationId = "later"; input.source = "later Y"; input.fields.value = "later field";
      await Promise.resolve(); throw new Error("JSON Unicode rejected");
    }
    if (path.endsWith("status")) { assert.deepEqual(body, { operationId: "original" }); return { operationId: "original", status: "unknown" }; }
    assert.deepEqual(body, { operationId: "original", requestKind: "save", wireDigest: digest(wire) });
    assert.ok(!JSON.stringify(body).includes("\\ud800"));
    return { operationId: "original", status: "resolved" };
  }), "/save", input, "save"), TerminatedAttempt);
  assert.deepEqual(calls, ["/save", "/api/operations/status", "/api/operations/settle"]);
  assert.equal(input.source, "later Y");
});

test("unproduced success aliases cannot acknowledge or deliver; the same original later recovers from its real committed status", async () => {
  for (const status of ["complete", "success", "after", "created", "unexpected"]) {
    const calls: string[] = []; let good = false, delivered = 0;
    const request = transport(path => { calls.push(path); return { operationId: "original", status: good ? "committed" : status, document: { source: "must not consume" } }; });
    await assert.rejects(submitOriginal(request, "/save", payload, "save"), /原操作/);
    assert.deepEqual(calls, ["/save", "/api/operations/status"]);
    const bad = await deliverCommitted({ operationId: "original", status }, async () => { delivered++; });
    assert.equal(committedDeliveryComplete(bad), false); assert.equal(delivered, 0);
    good = true;
    const recovered = await submitOriginal(request, "/save", payload, "save", true);
    assert.equal(recovered.status, "committed"); assert.equal(calls.filter(path => path === "/save").length, 1);
  }
  await assert.rejects(submitOriginal(transport(() => ({ operationId: "original", status: "saved" })), "/save", payload, "save", true), /原操作/);
});

test("actual typed delivery errors preserve every cause while old message bodies cannot qualify a receipt", async () => {
  const error = {code:"originalUnavailable",explanation:message("shell.savedBasisChangedCheckDisk"),diagnostics:[{origin:"os" as const,text:"exact original OS diagnostic"}],causes:[{code:"inner",explanation:message("common.operation",{operationId:"original"}),diagnostics:[]}],details:{kind:"currentSource" as const,currentSource:"raw author source \r\n😀"}};
  for (const slot of ["deliveryError","documentError"] as const) {
    const result = await submitOriginal(transport(() => ({operationId:"original",status:"committed",[slot]:structuredClone(error)})),"/capture",payload,"capture",true);
    assert.deepEqual(result[slot],error);assert.equal(committedDeliveryComplete(result),false);
    let called=false;const blocked=await deliverCommitted(result,async()=>{called=true;});assert.equal(called,false);assert.deepEqual(blocked[slot],error);
    await assert.rejects(submitOriginal(transport(() => ({operationId:"original",status:"committed",[slot]:{code:"old",message:"legacy"}})),"/capture",payload,"capture",true),/完整协议/);
  }
});
