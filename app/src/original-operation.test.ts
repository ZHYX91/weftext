import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { submitOriginal, deliverCommitted, TerminatedAttempt, type OriginalRequest } from "./original-operation";
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
  assert.equal(result,receipt);assert.equal(result.status,"committed");assert.deepEqual(result.deliveryError,{message:"refresh failed"});
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
