import assert from "node:assert/strict";
import test from "node:test";
import { ProposalCloseGuard } from "./proposal-close-guard";

function deferred() { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; }
class Root extends EventTarget { inert = false; }

test("proposal close remains blocked across transport completion and actual host delivery", async () => {
    const guard = new ProposalCloseGuard(), response = deferred(), delivery = deferred();
    let delivered = false;
    const operation = guard.run(async () => { await response.promise; await delivery.promise; delivered = true; });
    assert.equal(guard.freeze(false), false);
    response.resolve(); await Promise.resolve();
    assert.equal(guard.freeze(false), false);
    assert.throws(() => guard.assertReady(false));
    delivery.resolve(); await operation;
    assert.equal(delivered, true);
    assert.equal(guard.freeze(false), true);
    await assert.rejects(guard.run(async () => { delivered = false; }));
    assert.equal(delivered, true);
    guard.thaw(); await guard.run(async () => { delivered = false; });
    assert.equal(delivered, false);
});

test("actual owner proposal and composition each prevent freeze; input gates restore prior inert state", () => {
    const guard = new ProposalCloseGuard(), root = new Root();
    guard.bind(root as unknown as HTMLElement);
    assert.equal(guard.freeze(true), false);
    root.dispatchEvent(new Event("compositionstart"));
    assert.equal(guard.freeze(false), false);
    root.dispatchEvent(new Event("compositionend"));
    let input = 0; root.addEventListener("input", () => { input++; });
    assert.equal(guard.freeze(false), true);
    assert.equal(root.inert, true);
    root.dispatchEvent(new Event("input", { cancelable: true }));
    assert.equal(input, 0);
    assert.throws(() => guard.bind(new Root() as unknown as HTMLElement));
    guard.thaw(); assert.equal(root.inert, false);
    root.dispatchEvent(new Event("input")); assert.equal(input, 1);
    root.inert = true; assert.equal(guard.freeze(false), true); guard.thaw();
    assert.equal(root.inert, true);
});

test("failed operations release their own busy lease once without acknowledging retained proposals", async () => {
    const guard = new ProposalCloseGuard();
    await assert.rejects(guard.run(async () => { throw new Error("delivery unavailable"); }));
    assert.equal(guard.freeze(true), false);
    const end = guard.begin(); end(); end();
    assert.doesNotThrow(() => guard.assertReady(false));
    assert.equal(guard.freeze(false), true);
});
