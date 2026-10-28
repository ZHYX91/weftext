import { literalSourceOffset, type WritingLiteral } from "./writing";

/** A DOM caret is useful only when its complete displayed literal is exact. */
export function verifiedTableCaretOffset(raw: string, text: string, displayed: string, prefix: string | null): number | null {
    if (displayed !== text || prefix === null || !text.startsWith(prefix)) return null;
    const offset = prefix.length;
    if (literalSourceOffset(raw, text, offset) === null) return null;
    // Browsers normally return grapheme boundaries. Refuse a partial cluster from
    // an unsupported or inconsistent point result rather than splitting it.
    if (offset === text.length) return offset;
    for (const part of new Intl.Segmenter(undefined, { granularity: "grapheme" }).segment(text)) {
        if (part.index >= offset) return part.index === offset ? offset : null;
    }
    return null;
}

/** Read the browser's actual hit test; coordinates never identify an author slot. */
export function tableCaretAtPoint(display: HTMLElement, region: WritingLiteral, point: { clientX: number; clientY: number }): number | null {
    if (display.textContent !== region.text) return null;
    const owner = display.ownerDocument as Document & {
        caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
        caretRangeFromPoint?: (x: number, y: number) => Range | null;
    };
    try {
        const position = owner.caretPositionFromPoint?.(point.clientX, point.clientY);
        const hit = position ? { node: position.offsetNode, offset: position.offset } : (() => {
            const range = owner.caretRangeFromPoint?.(point.clientX, point.clientY);
            return range ? { node: range.startContainer, offset: range.startOffset } : null;
        })();
        if (!hit || !display.contains(hit.node)) return null;
        const prefix = owner.createRange();
        prefix.selectNodeContents(display);
        prefix.setEnd(hit.node, hit.offset);
        return verifiedTableCaretOffset(region.range.raw, region.text, display.textContent ?? "", prefix.toString());
    } catch {
        return null;
    }
}
