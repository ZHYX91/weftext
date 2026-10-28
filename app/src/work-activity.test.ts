import test from "node:test";
import assert from "node:assert/strict";
import { WorkActivity, type WorkActivityEvent } from "./work-activity";

test("original updates replace their own task and replay the latest outcome", () => {
    const activity = new WorkActivity();
    activity.task({ id: "original", title: "import", status: "running" });
    activity.task({ id: "original", title: "import", status: "waiting", detail: "unknown original" });
    const seen: WorkActivityEvent[] = [];
    const stop = activity.subscribe(event => seen.push(event));
    assert.equal(seen.length, 1);
    assert.equal(seen[0].kind, "task");
    if (seen[0].kind === "task") assert.equal(seen[0].task.status, "waiting");
    stop();
    activity.task({ id: "original", title: "import", status: "succeeded" });
    assert.equal(seen.length, 1);
});

test("bounded completed/result history does not evict uncertain original tasks", () => {
    const activity = new WorkActivity(1);
    activity.task({ id: "uncertain", title: "sync", status: "waiting" });
    activity.task({ id: "first", title: "export", status: "succeeded" });
    activity.task({ id: "last", title: "export", status: "succeeded" });
    activity.result({ id: "old", title: "old result" });
    activity.result({ id: "new", title: "new result" });
    const seen: WorkActivityEvent[] = [];
    activity.subscribe(event => seen.push(event));
    assert.deepEqual(seen.filter(event => event.kind === "task").map(event => event.task.id), ["uncertain", "last"]);
    assert.deepEqual(seen.filter(event => event.kind === "result").map(event => event.result.id), ["new"]);
});

test("a failing view observer cannot change outcomes or prevent another view receiving facts", () => {
    const activity = new WorkActivity();
    activity.subscribe(() => { throw new Error("broken view"); });
    let calls = 0;
    activity.subscribe(() => { calls++; });
    assert.doesNotThrow(() => activity.task({ id: "original", title: "export", status: "running" }));
    assert.equal(calls, 1);
});
