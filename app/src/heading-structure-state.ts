export interface StructureHeading {
    kind: "heading" | "figure" | "table" | "equation" | "code" | "caption";
    nativeKind?: string | null;
    key: string;
    text: string;
    number?: string | null;
    level: number;
    range?: {
        utf16From: number;
        utf16To: number;
    } | null;
    generated: boolean;
}
export interface HeadingSnapshot {
    coverage: "complete" | "partial";
    truncated: boolean;
    items: StructureHeading[];
}
export interface HeadingEntry extends StructureHeading {
    parent: string;
    children: string[];
    descendants: number;
    depth: number;
}
export const DOCUMENT_ROOT = "@weftext-document-root";
/** -1 means no match has been visited yet. Search never opens author content. */
export function nextHeadingMatch(count: number, current: number, step: -1 | 1): number {
    if (count === 0) return -1;
    return current < 0 || current >= count ? step === 1 ? 0 : count - 1 : (current + step + count) % count;
}
/** Keep the selected match and its real ancestors inside the existing canvas budget. */
export function boundedHeadingMap(items: HeadingEntry[], selected: string, limit = 200): HeadingEntry[] {
    if (items.length <= limit) return items;
    const byKey = new Map(items.map(item => [item.key, item])), path: string[] = [];
    let item = byKey.get(selected);
    while (item && path.length < limit) { path.push(item.key); item = byKey.get(item.parent); }
    const keep = new Set<string>([items[0].key, ...path.slice(0, Math.max(0, limit - 1))]);
    for (const item of items) { if (keep.size >= limit) break; keep.add(item.key); }
    return items.filter(item => keep.has(item.key));
}
/** Ordered tree geometry for the heading cards; no source or numbering inference. */
export function headingMapLayout(items: HeadingEntry[], direction: "horizontal" | "vertical"): Map<string, { x: number; y: number }> {
    const ids = new Set(items.map(item => item.key)), slots = new Map<string, number>(), levels = new Map<string, number>();
    let leaves = 0;
    for (const item of items) levels.set(item.key, (levels.get(item.parent) ?? -1) + 1);
    for (let index = items.length - 1; index >= 0; index--) {
        const item = items[index], children = item.children.filter(key => ids.has(key));
        slots.set(item.key, children.length ? (slots.get(children[0])! + slots.get(children.at(-1)!)!) / 2 : leaves++);
    }
    return new Map(items.map(item => {
        const lane = leaves - 1 - slots.get(item.key)!, depth = levels.get(item.key)!;
        return [item.key, direction === "horizontal" ? { x: depth * 320 + 150, y: lane * 85 + 55 } : { x: lane * 280 + 150, y: depth * 115 + 55 }];
    }));
}
/** A fresh read's render key alone cannot locate a different older compile.
 * Validate the actual installed native display product before scrolling it. */
export function headingMatchesDisplay(tree: RenderNode | null | undefined, heading: StructureHeading): boolean {
    if (!tree || !Number.isSafeInteger(heading.level))
        return false;
    const matches: RenderNode[] = [], stack = [tree];
    while (stack.length) {
        const node = stack.pop()!;
        if (node.key === heading.key)
            matches.push(node);
        stack.push(...node.children);
    }
    if (matches.length !== 1)
        return false;
    const node = matches[0];
    const text = (item: RenderNode): string => ["heading-prefix", "caption-prefix", "equation-prefix"].includes(String(item.attrs["data-weftext-role"])) ? "" : item.kind === "text" ? item.text ?? "" : item.children.map(text).join("");
    if (node.kind === "tag")
        return false;
    if (heading.kind === "heading")
        return node.attrs["data-weftext-role"] === "heading" && Number(node.attrs["data-weftext-level"]) === heading.level && text(node).trim() === heading.text;
    if (heading.kind === "equation" && node.attrs["data-weftext-role"] === "equation")
        return node.attrs["data-weftext-numbered"] === "true" && heading.nativeKind === "equation" && text(node).trim() === heading.text;
    if (node.attrs["data-weftext-role"] !== "caption" || node.attrs["data-weftext-caption-kind"] !== heading.nativeKind)
        return false;
    const body = node.children.flatMap(function find(child): RenderNode[] { return child.attrs["data-weftext-role"] === "caption-body" ? [child] : child.children.flatMap(find); });
    return body.length <= 1 && text(body[0] ?? node).trim() === heading.text;
}
export function headingTree(items: StructureHeading[]): HeadingEntry[] {
    const root: HeadingEntry = { key: DOCUMENT_ROOT, kind: "heading", text: "", level: 0, generated: true, parent: "", children: [], descendants: items.length, depth: 0 };
    const result = [root], stack = [root];
    for (const item of items) {
        if (item.kind === "heading")
            while (stack.length > 1 && stack.at(-1)!.level >= item.level)
                stack.pop();
        const parent = stack.at(-1)!, entry = { ...item, parent: parent.key, children: [], descendants: 0, depth: parent.depth + 1 };
        parent.children.push(item.key);
        result.push(entry);
        if (item.kind === "heading")
            stack.push(entry);
    }
    const byKey = new Map(result.map(n => [n.key, n]));
    for (let i = result.length - 1; i > 0; i--) {
        const node = result[i], parent = byKey.get(node.parent)!;
        if (parent !== root)
            parent.descendants += node.descendants + 1;
    }
    return result;
}
export function visibleHeadings(tree: HeadingEntry[], scope: string, collapsed: Set<string>, search: string, depth: number): {
    items: HeadingEntry[];
    matches: string[];
} {
    const byKey = new Map(tree.map(n => [n.key, n])), root = byKey.get(scope) ?? tree[0], included = new Set<string>([root.key]), needle = search.trim().toLocaleLowerCase(), matches: string[] = [], hidden = new Set<string>();
    for (const node of tree) {
        if (node.key === root.key)
            continue;
        if (!included.has(node.parent))
            continue;
        included.add(node.key);
        if (needle && node.text.toLocaleLowerCase().includes(needle))
            matches.push(node.key);
    }
    // Search reveals matching paths without touching the saved collapse set or
    // the unfiltered native descendant counts. Clearing restores those branches.
    const searchPaths = new Set<string>();
    for (const key of matches) {
        let node = byKey.get(key);
        while (node && !searchPaths.has(node.key)) {
            searchPaths.add(node.key);
            node = byKey.get(node.parent);
        }
    }
    const items = tree.filter(node => {
        if (!included.has(node.key))
            return false;
        if (needle)
            return node.key === root.key || searchPaths.has(node.key);
        if (node.key !== root.key && (hidden.has(node.parent) || collapsed.has(node.parent) || depth > 0 && node.depth - root.depth > depth)) {
            hidden.add(node.key);
            return false;
        }
        return true;
    });
    if (needle && root.key !== DOCUMENT_ROOT && root.text.toLocaleLowerCase().includes(needle))
        matches.unshift(root.key);
    return { items, matches };
}
import type { RenderNode } from "./types";
