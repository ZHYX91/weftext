import assert from "node:assert/strict";
import test from "node:test";
import { decodeTimelineList, TimelineReadModel } from "./timeline-view";
import { message, uiTextValue } from "./ui-locale";
import type { HistoryContext, HistoryList, HistoryVersion } from "./history-state";

const context = (name = "A"): HistoryContext => ({
    ownerKey: `buffer-${name}`, sessionId: `session-${name}`, workspaceId: "workspace", nodeId: `node-${name}`,
    root: "D:/library", path: `D:/library/${name}/${name}.typ`, baseId: `base-${name}`, source: "source", baseSource: "source",
    dirty: false, serial: 1, inputId: `input-${name}`,
});
const description = { explanation: message("history.retainedCommittedSource"), diagnostics: [], causes: [] };
const version = (id = "v1", time = 1000): HistoryVersion => ({
    versionId: id, revisionId: id, operationId: `save-${id}`, kind: "save", savedAtUnixMs: time, timeBasis: "commitClock",
    commitSequence: 1, status: "available", byteLength: 6, sha256: "abc", description,
});
const list = (current = context()): HistoryList => ({
    nodeId: current.nodeId!, workspaceId: current.workspaceId!, path: current.path, currentBaseId: current.baseId,
    currentSavedVersionId: "v1", currentness: "saved", versions: [version()],
    retention: { policy: "retainedCoreJournal", complete: false, knownVersions: 1, availableVersions: 1,
        oldestSavedAtUnixMs: 1000, newestSavedAtUnixMs: 1000, description },
    protectedDrafts: [], warnings: [],
});
const deferred = <T>() => { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done; }); return { promise, resolve }; };
// Reads after an awaited delivery must observe the model again; an earlier
// assert.equal(model.list, null) only describes the pre-delivery state.
const deliveredList = (model: TimelineReadModel): HistoryList | null => model.list;

test("timeline only lists a managed current session and leaves Draft input untouched", async () => {
    let current: HistoryContext | null = context();
    const calls: { route: string; body: unknown }[] = [];
    const original = structuredClone(current);
    const model = new TimelineReadModel({ getDocument: () => current, request: async <T>(route: string, body?: unknown) => {
        calls.push({ route, body }); return list(current!) as T;
    } });
    await model.update();
    assert.deepEqual(calls, [{ route: "/api/history/list", body: { sessionId: "session-A" } }]);
    assert.deepEqual(current, original);
    current = { ...current, source: "new unsaved text", serial: 2, inputId: "new-input", dirty: true };
    await model.update();
    assert.equal(calls.length, 1, "typing over the same saved base never reloads the sidebar or settles the input");
    current = { ...current, nodeId: undefined };
    await model.update();
    assert.equal(model.list, null);
    assert.equal(model.context, null);
    assert.equal(calls.length, 1);
    current = null;
    await model.update();
    assert.equal(model.loading, false);
});

test("late history replies cannot cross the captured node, session or saved base", async () => {
    for (const replacement of [context("B"), { ...context(), sessionId: "rebound-A" }, { ...context(), baseId: "new-base-A" }]) {
        let current = context();
        const first = deferred<HistoryList>(), second = deferred<HistoryList>();
        let requests = 0;
        const model = new TimelineReadModel({ getDocument: () => current,
            request: async <T>() => (++requests === 1 ? first.promise : second.promise) as Promise<T> });
        const oldRead = model.update();
        current = replacement;
        const newRead = model.update();
        first.resolve(list(context()));
        await oldRead;
        assert.equal(model.list, null);
        assert.equal(model.loading, true);
        second.resolve(list(replacement));
        await newRead;
        assert.equal(deliveredList(model)?.nodeId, replacement.nodeId);
        assert.equal(model.loading, false);
    }
});

test("timeline rejects mismatched scope and can explicitly retry the same read", async () => {
    const current = context();
    let wrongScope = true;
    const model = new TimelineReadModel({ getDocument: () => current, request: async <T>() => (wrongScope ? list(context("B")) : list(current)) as T });
    await model.update();
    assert.equal(model.list, null);
    assert.ok(model.error);
    assert.equal(model.loading, false);
    wrongScope = false;
    await model.update(true);
    assert.equal(model.error, null);
    assert.equal(deliveredList(model)?.nodeId, current.nodeId);
});

test("unavailable, empty and transport failure remain different timeline states", async () => {
    let current: HistoryContext | null = null;
    let fail = false;
    const model = new TimelineReadModel({ getDocument: () => current, request: async <T>() => {
        if (fail) throw new Error("network read failed");
        return { ...list(current!), versions: [] } as T;
    } });
    await model.update();
    assert.equal(model.context, null);
    current = context();
    await model.update();
    assert.deepEqual(model.list?.versions, []);
    assert.equal(model.error, null);
    fail = true;
    await model.update(true);
    assert.equal(model.list, null);
    assert.equal(uiTextValue(model.error!), "network read failed");
});

test("timeline preserves Core commit order even when clock timestamps go backwards", async () => {
    const current = context();
    const ordered = { ...list(current), versions: [{ ...version("v2", 1000), commitSequence: 2 }, version("v1", 2000)] };
    const model = new TimelineReadModel({ getDocument: () => current, request: async <T>() => ordered as T });
    await model.update();
    assert.deepEqual(model.list?.versions.map(item => item.versionId), ["v2", "v1"]);
});

test("wire decoding rejects malformed rows and dispose retires in-flight deliveries", async () => {
    const malformed = [
        { ...list(), versions: [version(), version()] },
        { ...list(), versions: [{ ...version(), savedAtUnixMs: -1 }] },
        { ...list(), retention: { ...list().retention, complete: true } },
        { ...list(), versions: [{ ...version(), description: "not a typed description" }] },
        { ...list(), protectedDrafts: [{ path: "A", baseSource: "", source: "", baseKnown: "yes" }] },
    ];
    for (const item of malformed) assert.throws(() => decodeTimelineList(item));
    const pending = deferred<HistoryList>();
    const model = new TimelineReadModel({ getDocument: () => context(), request: async <T>() => pending.promise as Promise<T> });
    const update = model.update();
    model.dispose();
    pending.resolve(list());
    await update;
    assert.equal(model.list, null);
    assert.equal(model.context, null);
});
