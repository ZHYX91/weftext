import { message, UiError } from "./ui-locale";

/** Volatile proposals stay with their actual owner until an explicit apply/discard.
 * Busy spans include host delivery, not merely the transport response. */
export class ProposalCloseGuard {
    private active = 0;
    private closeFrozen = false;
    private readonly composing = new Set<EventTarget>();
    private readonly roots = new Map<HTMLElement, boolean>();
    get frozen(): boolean { return this.closeFrozen; }
    assertMutable(): void {
        if (this.closeFrozen) throw new UiError(message("shell.closeAuthorInputPending"));
    }
    assertReady(unresolved: boolean): void {
        if (unresolved || this.active !== 0 || this.composing.size !== 0)
            throw new UiError(message("shell.closeAuthorInputPending"));
    }
    /** Owner-only retirement after its unresolved proposal has been retained separately. */
    retireComposition(target: EventTarget): void { this.composing.delete(target); }
    freeze(unresolved: boolean): boolean {
        try { this.assertReady(unresolved); }
        catch { return false; }
        if (!this.closeFrozen) {
            this.closeFrozen = true;
            for (const element of this.roots.keys()) {
                this.roots.set(element, element.inert);
                element.inert = true;
            }
        }
        return true;
    }
    thaw(): void {
        if (!this.closeFrozen) return;
        this.closeFrozen = false;
        for (const [element, inert] of this.roots) element.inert = inert;
    }
    begin(): () => void {
        this.assertMutable();
        this.active++;
        let ended = false;
        return () => { if (!ended) { ended = true; this.active--; } };
    }
    async run<T>(operation: () => Promise<T>): Promise<T> {
        const end = this.begin();
        try { return await operation(); }
        finally { end(); }
    }
    bind(element: HTMLElement): void {
        if (this.roots.has(element)) return;
        if (this.closeFrozen) throw new UiError(message("shell.closeAuthorInputPending"));
        this.roots.set(element, element.inert);
        const gate = (event: Event): void => {
            if (this.closeFrozen) { event.preventDefault(); event.stopImmediatePropagation(); return; }
            if (event.type === "compositionstart" && event.target) this.composing.add(event.target);
            if (event.type === "compositionend" && event.target) this.composing.delete(event.target);
        };
        for (const type of ["beforeinput", "input", "change", "click", "submit", "compositionstart", "compositionend"])
            element.addEventListener(type, gate, true);
    }
}
