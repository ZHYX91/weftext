import test from "node:test";
import assert from "node:assert/strict";
import { KnowledgeReadQueue, type KnowledgeReadOutcome } from "./knowledge-read-queue";

function deferred<T>() {
    let resolve!: (value: T) => void, reject!: (error: unknown) => void;
    const promise = new Promise<T>((done, fail) => { resolve = done; reject = fail; });
    return { promise, resolve, reject };
}
const turn = () => new Promise<void>(resolve => setImmediate(resolve));

test("different demands retain one physical read and execute only the latest waiting input", async () => {
    const gates = { A: deferred<boolean>(), B: deferred<boolean>(), C: deferred<boolean>() };
    const started: string[] = []; let running = 0, peak = 0;
    const queue = new KnowledgeReadQueue<keyof typeof gates>(async input => {
        started.push(input); peak = Math.max(peak, ++running);
        try { return await gates[input].promise; }
        finally { running--; }
    });
    const first = queue.request("A"), skipped = queue.request("B"), latest = queue.request("C");
    assert.deepEqual(await first, { status: "retired" });
    assert.deepEqual(await skipped, { status: "retired" });
    assert.deepEqual(started, ["A"]); assert.equal(running, 1);
    assert.equal(queue.current, "C"); assert.equal(queue.join(), latest);
    let settled = false; void latest.then(() => { settled = true; }); await turn(); assert.equal(settled, false);
    gates.A.resolve(true); await turn();
    assert.deepEqual(started, ["A", "C"]); assert.equal(peak, 1);
    gates.C.resolve(true); assert.deepEqual(await latest, { status: "installed" });
    assert.equal(running, 0); assert.equal(queue.current, null); assert.equal(queue.join(), null);
});

test("returning to equal input cannot revive a retired active ticket", async () => {
    const firstGate = deferred<boolean>(), latestGate = deferred<boolean>(), started: string[] = [];
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        return await (started.length === 1 ? firstGate : latestGate).promise;
    });
    const first = queue.request("A"), skipped = queue.request("B"), latest = queue.request("A");
    assert.notEqual(latest, first); assert.equal(queue.join(), latest);
    assert.deepEqual(await first, { status: "retired" }); assert.deepEqual(await skipped, { status: "retired" });
    assert.deepEqual(started, ["A"]);
    firstGate.resolve(true); await turn(); assert.deepEqual(started, ["A", "A"]);
    latestGate.resolve(true); assert.deepEqual(await latest, { status: "installed" });
});

test("joined callers share their own original failure without rejecting the queue worker", async () => {
    const gate = deferred<boolean>(), original = new Error("original read failure");
    const queue = new KnowledgeReadQueue<string>(async () => await gate.promise);
    const first = queue.request("A"), joined = queue.join(); assert.equal(joined, first);
    gate.reject(original);
    const results = await Promise.all([first, joined]);
    for (const outcome of results) {
        assert.equal(outcome?.status, "failed");
        if (outcome?.status === "failed") assert.equal(outcome.error, original);
    }
    assert.equal(queue.current, null); assert.equal(queue.join(), null);
});

test("an old failure cannot replace its retirement or fail a newer demand", async () => {
    const gate = deferred<boolean>(), original = new Error("retired read failure"), started: string[] = [];
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        return input === "old" ? await gate.promise : true;
    });
    const old = queue.request("old"), latest = queue.request("new");
    assert.deepEqual(await old, { status: "retired" });
    gate.reject(original); assert.deepEqual(await latest, { status: "installed" });
    assert.deepEqual(await old, { status: "retired" }); assert.deepEqual(started, ["old", "new"]);
});

test("invalidating active and queued demands does not free the occupied physical slot", async () => {
    const activeGate = deferred<boolean>(), latestGate = deferred<boolean>(), started: string[] = [];
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        return await (input === "active" ? activeGate : latestGate).promise;
    });
    const active = queue.request("active"), queued = queue.request("discarded");
    queue.invalidate(); assert.equal(queue.current, null); assert.equal(queue.join(), null);
    assert.deepEqual(await active, { status: "retired" }); assert.deepEqual(await queued, { status: "retired" });
    const latest = queue.request("after invalidation");
    assert.deepEqual(started, ["active"]);
    activeGate.resolve(true); await turn(); assert.deepEqual(started, ["active", "after invalidation"]);
    latestGate.resolve(true); assert.deepEqual(await latest, { status: "installed" });
});

test("a cleared demand stays cleared when its old execution finishes and idle invalidation starts no work", async () => {
    const gate = deferred<boolean>(), started: string[] = [];
    const queue = new KnowledgeReadQueue<string>(async input => { started.push(input); return await gate.promise; });
    const old = queue.request("old"); queue.invalidate();
    assert.deepEqual(await old, { status: "retired" }); gate.resolve(true); await turn();
    queue.invalidate(); await turn(); assert.deepEqual(started, ["old"]);
    assert.equal(queue.current, null); assert.equal(queue.join(), null);
});

test("completed outcomes are not cached and the owner-provided input is not copied", async () => {
    const input = Object.freeze({ name: "same" }), seen: typeof input[] = [];
    const queue = new KnowledgeReadQueue<typeof input>(async current => { seen.push(current); return true; });
    const first = queue.request(input); assert.equal(queue.join(), first);
    assert.deepEqual(await first, { status: "installed" }); assert.equal(queue.join(), null);
    const next = queue.request(input); assert.notEqual(next, first);
    assert.deepEqual(await next, { status: "installed" });
    assert.equal(seen.length, 2); assert.equal(seen[0], input); assert.equal(seen[1], input);
});

test("requests made reentrantly by the executing read remain queued until that read settles", async () => {
    const activeGate = deferred<boolean>(), started: string[] = [];
    let queued!: Promise<KnowledgeReadOutcome>;
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        if (input === "first") { queued = queue.request("latest"); return await activeGate.promise; }
        return true;
    });
    const first = queue.request("first");
    assert.deepEqual(await first, { status: "retired" }); assert.equal(queue.join(), queued);
    assert.deepEqual(started, ["first"]);
    activeGate.resolve(true); assert.deepEqual(await queued, { status: "installed" });
    assert.deepEqual(started, ["first", "latest"]);
});

test("reentrant invalidation at execution start cannot let the old tail clear its successor", async () => {
    const activeGate = deferred<boolean>(), started: string[] = [];
    let successor!: Promise<KnowledgeReadOutcome>;
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        if (input === "first") { queue.invalidate(); successor = queue.request("successor"); return await activeGate.promise; }
        return true;
    });
    const first = queue.request("first");
    assert.deepEqual(await first, { status: "retired" }); assert.equal(queue.current, "successor");
    assert.deepEqual(started, ["first"]);
    activeGate.resolve(false); assert.deepEqual(await successor, { status: "installed" });
    assert.deepEqual(started, ["first", "successor"]);
});

test("retirement during the final execution step cannot deliver that ticket's apparent success", async () => {
    const gate = deferred<boolean>(), started: string[] = [];
    let successor!: Promise<KnowledgeReadOutcome>;
    const queue = new KnowledgeReadQueue<string>(async input => {
        started.push(input);
        if (input === "first") { await gate.promise; successor = queue.request("successor"); }
        return true;
    });
    const first = queue.request("first"); gate.resolve(true);
    assert.deepEqual(await first, { status: "retired" });
    assert.deepEqual(await successor, { status: "installed" }); assert.deepEqual(started, ["first", "successor"]);
});

test("failed source qualification retires without retrying, and synchronous throws preserve the error", async () => {
    let calls = 0;
    const queue = new KnowledgeReadQueue<string>(async () => { calls++; return false; });
    assert.deepEqual(await queue.request("stale source"), { status: "retired" });
    await turn(); assert.equal(calls, 1); assert.equal(queue.current, null);
    const failure = new Error("synchronous load failure"), throwing = new KnowledgeReadQueue<string>(() => { throw failure; });
    const outcome = await throwing.request("current"); assert.equal(outcome.status, "failed");
    if (outcome.status === "failed") assert.equal(outcome.error, failure);
    const undefinedFailure = new KnowledgeReadQueue<string>(() => { throw undefined; });
    assert.deepEqual(await undefinedFailure.request("current"), { status: "failed", error: undefined });
});
