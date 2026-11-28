import assert from "node:assert/strict";
import test from "node:test";
import { cancellableQuery } from "./cancellable-query";

function deferred<T>() {
    let resolve!: (value: T) => void;
    let reject!: (reason: Error) => void;
    const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
    return { promise, resolve, reject };
}

test("cancelling during ticket preparation retires the ticket before execution", async () => {
    const controller = new AbortController();
    const prepared = deferred<{ runId: string }>();
    const retired: string[] = [];
    const pending = cancellableQuery(controller.signal, () => prepared.promise,
        () => assert.fail("a cancelled query started"), async id => { retired.push(id); });
    controller.abort();
    prepared.resolve({ runId: "original" });
    await assert.rejects(pending);
    assert.deepEqual(retired, ["original"]);
});

test("a running query waits for its original cancellation receipt", async () => {
    const controller = new AbortController();
    const started = deferred<void>();
    const finished = deferred<string>();
    const cancelled = deferred<void>();
    const pending = cancellableQuery(controller.signal, async () => ({ runId: "original" }),
        id => { assert.equal(id, "original"); started.resolve(); return finished.promise; },
        id => { assert.equal(id, "original"); return cancelled.promise; });
    await started.promise;
    controller.abort();
    finished.reject(new Error("queryCancelled"));
    let settled = false;
    const observed = assert.rejects(pending, /queryCancelled/).then(() => { settled = true; });
    await new Promise(resolve => setImmediate(resolve));
    assert.equal(settled, false);
    cancelled.resolve();
    await observed;
});
