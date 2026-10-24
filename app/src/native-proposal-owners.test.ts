import test from "node:test";
import assert from "node:assert/strict";
import { createNativeProposalOwner, registerNativeProposalOwner, assertNativeProposalOwnersCloseReady, freezeNativeProposalOwnersForClose, thawNativeProposalOwnersAfterClose } from "./native-proposal-owners";

test("discard waits for its original async delivery before releasing the actual owner", () => {
    const owner = createNativeProposalOwner("test");
    const end = owner.begin();
    owner.setPending(false);
    owner.release();
    assert.throws(assertNativeProposalOwnersCloseReady);
    assert.equal(freezeNativeProposalOwnersForClose(), false);
    assert.throws(() => owner.begin());
    end(); end();
    assert.doesNotThrow(assertNativeProposalOwnersCloseReady);
});

test("a failed freeze thaws already acquired owners and preserves unresolved owner", () => {
    let held = false;
    const unregister = registerNativeProposalOwner({ assertCloseReady() {}, freezeForClose() { held = true; return true; }, thawAfterClose() { held = false; } });
    const unresolved = createNativeProposalOwner("pending");
    try {
        assert.equal(freezeNativeProposalOwnersForClose(), false);
        assert.equal(held, false);
        assert.throws(assertNativeProposalOwnersCloseReady);
    } finally {
        unresolved.setPending(false); unresolved.release(); unregister();
    }
});

test("successful freeze rejects a new dialog and late author writes until close is abandoned", () => {
    const owner = createNativeProposalOwner("test");
    owner.setPending(false);
    try {
        assert.equal(freezeNativeProposalOwnersForClose(), true);
        assert.throws(() => createNativeProposalOwner("late"));
        assert.throws(() => owner.begin());
        assert.throws(() => owner.setPending(true));
        thawNativeProposalOwnersAfterClose();
        owner.setPending(true);
        assert.throws(assertNativeProposalOwnersCloseReady);
    } finally {
        thawNativeProposalOwnersAfterClose(); owner.setPending(false); owner.release();
    }
});

test("hidden unresolved owner reveals its original proposal without discarding it", () => {
    let reveals = 0;
    const owner = createNativeProposalOwner("test", () => reveals++);
    try {
        assert.throws(assertNativeProposalOwnersCloseReady);
        assert.equal(reveals, 1);
        assert.throws(assertNativeProposalOwnersCloseReady);
    } finally { owner.setPending(false); owner.release(); }
});

test("retiring one real composition target cannot clear another still active input", async () => {
    const { ProposalCloseGuard } = await import("./proposal-close-guard");
    const guard = new ProposalCloseGuard();
    const listeners = new Map<string, (event: Event) => void>();
    guard.bind({ inert: false, addEventListener(type: string, handler: (event: Event) => void) { listeners.set(type, handler); } } as unknown as HTMLElement);
    const first = new EventTarget(), second = new EventTarget();
    listeners.get("compositionstart")!({ type: "compositionstart", target: first } as unknown as Event);
    listeners.get("compositionstart")!({ type: "compositionstart", target: second } as unknown as Event);
    guard.retireComposition(first);
    assert.throws(() => guard.assertReady(false));
    guard.retireComposition(second);
    assert.doesNotThrow(() => guard.assertReady(false));
});
