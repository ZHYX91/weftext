import type { ChangeDesc } from "@codemirror/state";

/** A unique, exact main-source text run and its position in the reading viewport. */
export interface ReadingReturnAnchor {
    from: number;
    to: number;
    text: string;
    top: number;
}

/**
 * Follow edits before or after the run without guessing where deleted, rewritten
 * or copied text went. Boundary insertions belong outside the original run.
 */
export function mapReadingReturnAnchor(anchor: ReadingReturnAnchor, changes: ChangeDesc, source: string): ReadingReturnAnchor | null {
    if (anchor.from < 0 || anchor.from >= anchor.to || anchor.to > changes.length || source.length !== changes.newLength)
        return null;
    let touched = false;
    changes.iterChangedRanges((from, to) => {
        if (from === to ? anchor.from < from && from < anchor.to : from < anchor.to && anchor.from < to)
            touched = true;
    }, true);
    if (touched) return null;
    const from = changes.mapPos(anchor.from, 1), to = changes.mapPos(anchor.to, -1);
    return from < to && source.slice(from, to) === anchor.text ? { ...anchor, from, to } : null;
}
