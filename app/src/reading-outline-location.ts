import { DOCUMENT_ROOT, type HeadingEntry } from "./heading-structure-state";

export interface ReadingHeadingBox {
    key: string;
    /** Document-local block offset, not a source position or a scroll target. */
    top: number;
    width: number;
    height: number;
}
export interface ReadingHeadingPosition { key: string; top: number }

/** Rebuilt for a fresh layout, not for each scroll event. Inputs use display order. */
export function readingHeadingPositions(
    eligible: ReadonlySet<string>, boxes: readonly ReadingHeadingBox[],
): ReadingHeadingPosition[] {
    const seen = new Set<string>(), duplicated = new Set<string>();
    for (const box of boxes) {
        if (!eligible.has(box.key)) continue;
        if (seen.has(box.key)) duplicated.add(box.key);
        else seen.add(box.key);
    }
    const ordered = boxes.filter(box => eligible.has(box.key) && !duplicated.has(box.key) &&
        Number.isFinite(box.top) && Number.isFinite(box.width) && Number.isFinite(box.height) &&
        box.width > 0 && box.height > 0).sort((a, b) => a.top - b.top);
    // At the same vertical position stick with the first visual entry; the
    // reading line alone cannot distinguish simultaneous columns.
    return ordered.filter((box, i) => !i || box.top !== ordered[i - 1].top).map(({ key, top }) => ({ key, top }));
}

/** At the document bottom, a short final section may never cross the reading line. */
export function readingHeadingLine(offset: number, viewport: number, content: number): number {
    const endVisible = content > viewport && content - viewport - offset <= 1;
    return offset + (endVisible ? Math.max(0, viewport - 1) : viewport / 3);
}

/** Last section whose heading the reading line has reached. No marker in the prefatory area. */
export function readingHeadingAt(index: readonly ReadingHeadingPosition[], line: number): string | undefined {
    if (!Number.isFinite(line)) return undefined;
    let first = 0, past = index.length;
    while (first < past) {
        const middle = first + Math.floor((past - first) / 2);
        if (index[middle].top <= line) first = middle + 1;
        else past = middle;
    }
    return index[first - 1]?.key;
}

/** A collapsed ancestor may show containment, never impersonate the exact heading. */
export function readingOutlineRow(
    key: string, byKey: ReadonlyMap<string, HeadingEntry>, shown: ReadonlySet<string>,
): { key: string; exact: boolean } | undefined {
    const known = new Set<string>();
    for (let row = byKey.get(key); row && row.key !== DOCUMENT_ROOT && !known.has(row.key); row = byKey.get(row.parent)) {
        if (row.kind !== "heading") return undefined;
        if (shown.has(row.key)) return { key: row.key, exact: row.key === key };
        known.add(row.key);
    }
    return undefined;
}

/** Read-only geometry for the current continuous render. It never scrolls or moves focus. */
export class ReadingOutlineTracker {
    private readonly lifetime = new AbortController();
    private readonly resize: ResizeObserver;
    private frame = 0;
    private dirty = true;
    private index: ReadingHeadingPosition[] = [];
    private currentKey: string | undefined;

    constructor(
        private readonly scroller: HTMLElement,
        private readonly surface: HTMLElement,
        private readonly eligible: ReadonlySet<string>,
        private readonly isCurrent: () => boolean,
        private readonly publish: (key: string | undefined) => void,
    ) {
        const signal = this.lifetime.signal;
        scroller.addEventListener("scroll", this.schedule, { passive: true, signal });
        surface.addEventListener("load", this.invalidate, { capture: true, signal });
        surface.ownerDocument.fonts?.addEventListener("loadingdone", this.invalidate, { signal });
        this.resize = new ResizeObserver(this.invalidate);
        this.resize.observe(scroller);
        this.resize.observe(surface);
        this.schedule();
    }

    private invalidate = (): void => { this.dirty = true; this.schedule(); };
    private schedule = (): void => {
        if (!this.frame && !this.lifetime.signal.aborted) this.frame = requestAnimationFrame(this.measure);
    };
    private measure = (): void => {
        this.frame = 0;
        if (this.lifetime.signal.aborted || !this.isCurrent()) return;
        const { scroller, surface } = this;
        if (this.dirty) {
            const origin = scroller.getBoundingClientRect().top + scroller.clientTop - scroller.scrollTop;
            const boxes: ReadingHeadingBox[] = [];
            for (const node of surface.querySelectorAll<HTMLElement>("[data-document-node-key]")) {
                const key = node.dataset.documentNodeKey;
                if (!key || !this.eligible.has(key)) continue;
                const bounds = node.getBoundingClientRect();
                boxes.push({ key, top: bounds.top - origin, width: bounds.width, height: bounds.height });
            }
            this.index = readingHeadingPositions(this.eligible, boxes);
            this.dirty = false;
        }
        const key = readingHeadingAt(this.index, readingHeadingLine(scroller.scrollTop, scroller.clientHeight, scroller.scrollHeight));
        if (key !== this.currentKey) { this.currentKey = key; this.publish(key); }
    };
    dispose(): void {
        cancelAnimationFrame(this.frame);
        this.frame = 0;
        this.lifetime.abort();
        this.resize.disconnect();
    }
}
