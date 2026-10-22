import test from "node:test";
import assert from "node:assert/strict";
import { ApiError } from "./api-error";
import { parseUiMessage, renderUiMessage } from "./ui-locale";

test("the actual local HTTP transport rejects duplicate fields before decoding and preserves safe message integers", async () => {
    const globals = globalThis as unknown as Record<string, unknown>;
    const saved = new Map(["location", "sessionStorage", "fetch"].map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)]));
    const replies: Response[] = [];
    Object.defineProperty(globalThis, "location", { configurable: true, value: { hash: "", pathname: "/", search: "" } });
    Object.defineProperty(globalThis, "sessionStorage", { configurable: true, value: { getItem: () => null } });
    Object.defineProperty(globalThis, "fetch", { configurable: true, value: async () => { const response = replies.shift(); assert.ok(response); return response; } });
    try {
        const { api } = await import("./api");
        for (const body of ['{"value":{"key":"common.save","key":"common.close","params":{}}}', '{"error":{"code":"first","code":"second","explanation":{"key":"common.save","params":{}},"diagnostics":[],"causes":[]}}']) {
            replies.push(new Response(body, { status: body.includes('"error"') ? 409 : 200 }));
            await assert.rejects(api("/api/test"), (error: unknown) => error instanceof ApiError && error.code === "invalidResponse");
        }
        replies.push(new Response('{"value":{"key":"reminder.missed","params":{"count":9007199254740991.0}}}'));
        const response = await api<{value: unknown}>("/api/test"), spec = parseUiMessage(response.value);
        assert.ok(spec); assert.match(renderUiMessage("en-US", spec), /9007199254740991/);
        replies.push(new Response('{"error":{"code":"unavailable","explanation":{"key":"common.operation","params":{"operationId":"原号\\u0000😀"}},"diagnostics":[{"origin":"typst","text":"原诊断\\ud800"}],"causes":[]}}', {status:409}));
        await assert.rejects(api("/api/test"), (error: unknown) => error instanceof ApiError && error.code === "unavailable" && error.diagnostics[0].text === "原诊断\ud800" && renderUiMessage("en-US", error.explanation).endsWith("原号\0😀"));
        const failure = { code: "sync-failure", explanation: { key: "common.unavailable", params: {} }, diagnostics: [], causes: [] };
        for (const error of [null, failure]) {
            const progress = { requestId: "original-sync", status: error ? "failed" : "running", sentObjects: 0, receivedObjects: 0, error };
            replies.push(new Response(JSON.stringify(progress), { status: 200 }));
            assert.deepEqual(await api("/api/test"), progress, "a successful result's own error slot is not an API failure envelope");
        }
        replies.push(new Response(JSON.stringify({ error: failure }), { status: 200 }));
        await assert.rejects(api("/api/test"), (error: unknown) => error instanceof ApiError && error.code === "sync-failure");
        assert.equal(replies.length, 0);
    } finally {
        for (const [name, descriptor] of saved) if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globals[name];
    }
});
