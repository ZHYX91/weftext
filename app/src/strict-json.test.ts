import test from "node:test";
import assert from "node:assert/strict";
import { parseStrictJson, readStrictJsonResponse, StrictJsonError } from "./strict-json";

test("raw response JSON rejects repeated decoded members at every depth", async () => {
    for (const body of ['{"error":{"code":"one","code":"two"}}', '{"items":[{"name":1,"n\\u0061me":2}]}', '{"__proto__":{},"__proto__":{}}']) {
        await assert.rejects(readStrictJsonResponse(new Response(body)), (error: unknown) => error instanceof StrictJsonError && error.reason === "duplicateMember");
    }
    assert.deepEqual(await readStrictJsonResponse(new Response('{"author":"a\\u0000\\ud800","first":{"name":1},"second":{"name":2}}')), { author: "a\0\ud800", first: { name: 1 }, second: { name: 2 } });
});

test("the JSON boundary preserves numbers and exact author strings, and bounds traversal", () => {
    const source = '{"raw":"{\\\"a\\\":1,\\\"a\\\":2}","n":9007199254740991.0,"e":1e0,"negativeZero":-0.0}';
    const value = parseStrictJson(source) as {raw: string; n: number; e: number; negativeZero: number};
    assert.equal(value.raw, '{"a":1,"a":2}'); assert.equal(value.n, Number.MAX_SAFE_INTEGER); assert.equal(value.e, 1); assert.ok(Object.is(value.negativeZero, -0));
    assert.throws(() => parseStrictJson("[".repeat(258) + "0" + "]".repeat(258)), (error: unknown) => error instanceof StrictJsonError && error.reason === "nestingBudget");
    assert.throws(() => parseStrictJson('{"a":1,}'), SyntaxError);
});
