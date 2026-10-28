import { message, errorUiText, type UiText } from "./ui-locale";
import { HeadingStructureView } from "./heading-structure-view";
import type { HeadingSnapshot, StructureHeading } from "./heading-structure-state";

/** Display identity stamped when the exact native render is accepted. No write authority. */
export interface OutlineBasis {
    sessionId: string;
    source: string;
    serial: number;
    inputId: string;
    path: string;
    root: string;
}
export interface OutlineSnapshot { basis: OutlineBasis; snapshot: HeadingSnapshot }
export interface DocumentOutlineHost {
    getDocument(): OutlineBasis | null;
    getOutlineSnapshot(): OutlineSnapshot | null;
    selectSource(from: number, to: number): void;
    revealHeading?(item: StructureHeading): boolean;
    notify(text: UiText, error?: boolean): void;
    canNavigate?(): boolean;
}
export function sameOutlineBasis(a: OutlineBasis | null, b: OutlineBasis | null): boolean {
    return a !== null && b !== null && a.sessionId === b.sessionId && a.source === b.source && a.serial === b.serial && a.inputId === b.inputId && a.path === b.path && a.root === b.root;
}
const copyBasis = (basis: OutlineBasis): OutlineBasis => ({ sessionId: basis.sessionId, source: basis.source, serial: basis.serial, inputId: basis.inputId, path: basis.path, root: basis.root });

/** Current-document navigation only: consumes accepted products without reading,
 * compiling, saving or acquiring an independent author/session owner. */
export class DocumentOutlineView {
    private readonly views = new Map<string, HeadingStructureView>();
    private active: HeadingStructureView | null = null;
    private installed: OutlineSnapshot | null = null;
    private presentation: { view: HeadingStructureView; basis: OutlineBasis | null; snapshot: HeadingSnapshot | null } | null = null;
    private disposed = false;

    constructor(private readonly container: HTMLElement, private readonly host: DocumentOutlineHost) { this.refresh(); }

    refresh(): void {
        if (this.disposed) return;
        const current = this.host.getDocument(), accepted = this.host.getOutlineSnapshot();
        this.display(current, current && accepted && sameOutlineBasis(accepted.basis, current) ? accepted : null);
    }

    /** Retire navigation immediately while author input or its projection is pending. */
    invalidate(): void {
        if (!this.disposed) this.display(this.host.getDocument(), null);
    }

    private display(current: OutlineBasis | null, accepted: OutlineSnapshot | null): void {
        const session = current?.sessionId ?? "";
        let view = this.views.get(session);
        if (!view) { view = new HeadingStructureView(); this.views.set(session, view); }
        if (this.active !== view) { this.active?.detach(); this.active = view; }
        const snapshot = accepted?.snapshot ?? null, previous = this.presentation;
        if (previous?.view === view && previous.snapshot === snapshot && (current === null && previous.basis === null || sameOutlineBasis(previous.basis, current))) return;
        const installed = accepted ? { basis: copyBasis(accepted.basis), snapshot: accepted.snapshot } : null;
        // Retire the capability before replacing its DOM, including while a
        // navigation filter is composing and its presentation update is deferred.
        this.installed = installed;
        this.presentation = { view, basis: installed?.basis ?? (current ? copyBasis(current) : null), snapshot };
        view.render(this.container, installed?.snapshot, item => this.open(installed, item.key),
            message(current ? "heading.waitForCurrentProjection" : "heading.openDocumentForOutline"));
    }

    private open(installed: OutlineSnapshot | null, key: string): void {
        if (this.disposed || this.host.canNavigate?.() === false) return;
        const current = this.host.getDocument(), accepted = this.host.getOutlineSnapshot();
        if (!installed || this.installed !== installed || !sameOutlineBasis(installed.basis, current) || !accepted || accepted.snapshot !== installed.snapshot || !sameOutlineBasis(accepted.basis, installed.basis)) {
            this.host.notify(message("heading.outlineBasisChanged"), true);
            this.refresh();
            return;
        }
        // The tree view adds presentation fields. Resolve only the unique
        // original native item, never a range supplied by a UI event payload.
        const items = installed.snapshot.items.filter(item => item.key === key);
        if (items.length !== 1) { this.host.notify(message("error.invalidResponse"), true); return; }
        const item = items[0];
        try {
            if (item.range) {
                const { utf16From: from, utf16To: to } = item.range;
                if (!Number.isSafeInteger(from) || !Number.isSafeInteger(to) || from < 0 || to < from || to > current!.source.length) {
                    this.host.notify(message("error.invalidResponse"), true);
                    return;
                }
                this.host.selectSource(from, to);
            } else if (!this.host.revealHeading?.(item)) this.host.notify(message("heading.generatedItemSourceUnavailable"));
        } catch (error) { this.host.notify(errorUiText(error), true); }
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true; this.installed = null; this.presentation = null;
        for (const view of this.views.values()) view.clear();
        this.views.clear(); this.active = null; this.container.replaceChildren();
    }
}
