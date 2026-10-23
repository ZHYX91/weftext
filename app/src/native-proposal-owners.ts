import { ProposalCloseGuard } from "./proposal-close-guard";
import { message, UiError, type UiText } from "./ui-locale";

export interface NativeProposalOwner {
    assertCloseReady(): void;
    freezeForClose(): boolean;
    thawAfterClose(): void;
}

// Live controllers, never inferred from mounted controls or recovery acknowledgements.
const owners = new Set<NativeProposalOwner>();
let frozen = false;
export function registerNativeProposalOwner(owner: NativeProposalOwner): () => void {
    if (frozen) throw new UiError(message("shell.closeAuthorInputPending"));
    owners.add(owner);
    return () => { owners.delete(owner); };
}
export function assertNativeProposalOwnersCloseReady(): void {
    for (const owner of owners) owner.assertCloseReady();
}
export function freezeNativeProposalOwnersForClose(): boolean {
    if (frozen) return true;
    const acquired: NativeProposalOwner[] = [];
    frozen = true;
    try {
        for (const owner of owners) {
            acquired.push(owner);
            if (!owner.freezeForClose()) throw new Error("proposal owner is unresolved");
        }
        return true;
    }
    catch {
        for (const owner of acquired.reverse()) owner.thawAfterClose();
        frozen = false;
        return false;
    }
}
export function thawNativeProposalOwnersAfterClose(): void {
    for (const owner of owners) owner.thawAfterClose();
    frozen = false;
}

/** Hiding a dialog does not release its proposals or asynchronous delivery. */
export function createNativeProposalOwner(_label: UiText, reveal?: () => void) {
    const guard = new ProposalCloseGuard();
    let pending = true, releasing = false, released = false;
    const assertMutable = (): void => {
        if (guard.frozen || released || releasing) throw new UiError(message("shell.closeAuthorInputPending"));
    };
    const finishRelease = (): void => {
        if (!releasing || pending) return;
        try { guard.assertReady(false); }
        catch { return; }
        released = true;
        unregister();
    };
    const owner = {
        assertCloseReady: (): void => {
            try { guard.assertReady(pending); }
            catch (error) { reveal?.(); throw error; }
        },
        freezeForClose: (): boolean => guard.freeze(pending),
        thawAfterClose: (): void => guard.thaw(),
        assertMutable,
        begin: (): (() => void) => {
            assertMutable();
            const end = guard.begin();
            return () => { end(); finishRelease(); };
        },
        setPending: (value: boolean): void => { assertMutable(); pending = value; },
        release: (): void => {
            if (released) return;
            if (pending) throw new UiError(message("shell.closeAuthorInputPending"));
            releasing = true;
            finishRelease();
        },
    };
    const unregister = registerNativeProposalOwner(owner);
    return owner;
}
