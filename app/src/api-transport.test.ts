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
        assert.equal(replies.length, 0);
    } finally {
        for (const [name, descriptor] of saved) if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globals[name];
    }
});

test("the native Draft branch uses the same strict typed error decoder without HTTP or legacy message adoption", async () => {
    const globals = globalThis as unknown as Record<string, unknown>;
    const names = ["window", "location", "sessionStorage", "fetch"], saved = new Map(names.map(name => [name, Object.getOwnPropertyDescriptor(globalThis, name)]));
    let listener!: (event: {data: unknown}) => void;
    let reply: unknown;
    const sent: {type: string; surfaceId: string; surfaceEpoch: string; requestId: string; request: {body: unknown}}[] = [];
    Object.defineProperty(globalThis, "location", { configurable: true, value: { hash: "", pathname: "/", search: "" } });
    Object.defineProperty(globalThis, "sessionStorage", { configurable: true, value: { getItem: () => null } });
    Object.defineProperty(globalThis, "fetch", { configurable: true, value: () => { throw new Error("A bound native Draft call must not use HTTP"); } });
    Object.defineProperty(globalThis, "window", { configurable: true, value: { chrome: {webview: {
        addEventListener(_kind: string, callback: typeof listener) { listener = callback; },
        postMessage(envelope: typeof sent[number]) { sent.push(envelope); listener({data: {type: "core-draft-result", surfaceId: envelope.surfaceId, surfaceEpoch: envelope.surfaceEpoch, requestId: envelope.requestId, body: reply}}); },
    }} } });
    try {
        const {api} = await import("./api"), {bindNativeDraftSurface} = await import("./native-draft");
        bindNativeDraftSurface({surfaceId: "owned-surface", surfaceEpoch: "owned-epoch"});
        const body = {sessionId: "7b67f37a-5c73-4a41-9951-798e67c639d5", source: "本人原文😀\r\n"};
        reply = {error: {code: "draftChanged", explanation: {key: "common.operation", params: {operationId: "original-id"}}, diagnostics: [{origin: "typst", text: "原诊断\ud800"}], causes: []}};
        await assert.rejects(api("/api/draft", body), (error: unknown) => error instanceof ApiError && error.code === "draftChanged" && error.diagnostics[0].text === "原诊断\ud800" && renderUiMessage("en-US", error.explanation).endsWith("original-id"));
        reply = {error: {code: "draftChanged", message: "old untyped message", diagnostics: [], causes: []}};
        await assert.rejects(api("/api/draft", body), (error: unknown) => error instanceof ApiError && error.code === "invalidResponse");
        assert.equal(sent.length, 2); assert.deepEqual(sent[0].request.body, body); assert.deepEqual(sent[1].request.body, body);
        assert.notEqual(sent[0].requestId, sent[1].requestId);
    } finally {
        for (const [name, descriptor] of saved) if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globals[name];
    }
});
