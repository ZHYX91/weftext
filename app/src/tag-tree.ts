import { message, setUiText, setUiAttr, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** One complete frozen forest. Paths are presentation, never identities or assignments. */
export interface TreeTag {
    reference: string;
    tagId: string;
    displayName: string;
    parentTagId: string | null;
}
export type TagMatchMode = "exact" | "subtree";
export interface TagPath {
    segments: string[];
    truncated: boolean;
    suffix: string;
}
export class TagTree {
    readonly byId = new Map<string, TreeTag>();
    readonly byRef = new Map<string, TreeTag>();
    readonly children = new Map<string | null, string[]>();
    readonly order: string[] = [];
    private readonly intervals = new Map<string, [
        number,
        number
    ]>();
    private readonly suffixes = new Map<string, string>();
    constructor(items: TreeTag[], complete: boolean) {
        if (!complete)
            throw new UiError(message("tagTree.incompleteTree"));
        if (items.length > 5000)
            throw new UiError(message("tagTree.budgetExceeded"));
        let workspace: string | undefined;
        for (const item of items) {
            if (!item || typeof item.reference !== "string" || !item.reference || typeof item.tagId !== "string" || !item.tagId || typeof item.displayName !== "string" || !item.displayName || item.parentTagId !== null && typeof item.parentTagId !== "string" || this.byId.has(item.tagId) || this.byRef.has(item.reference))
                throw new UiError(message("tagTree.invalidIdentityOrParent"));
            const parts = /^weftext-tag:\/\/([^/#?]+)\/([^/#?]+)$/.exec(item.reference);
            if (!parts || parts[2] !== item.tagId || workspace !== undefined && workspace !== parts[1])
                throw new UiError(message("tagTree.crossWorkspaceIdentity"));
            workspace = parts[1];
            this.byId.set(item.tagId, item);
            this.byRef.set(item.reference, item);
            const siblings = this.children.get(item.parentTagId) ?? [];
            siblings.push(item.tagId);
            this.children.set(item.parentTagId, siblings);
        }
        for (const item of items)
            if (item.parentTagId !== null && (!this.byId.has(item.parentTagId) || item.parentTagId === item.tagId))
                throw new UiError(message("tagTree.invalidParent"));
        const stack = [...(this.children.get(null) ?? [])].reverse().map(id => ({ id, exit: false }));
        while (stack.length) {
            const { id, exit } = stack.pop()!;
            if (exit) {
                this.intervals.get(id)![1] = this.order.length;
                continue;
            }
            this.intervals.set(id, [this.order.length, -1]);
            this.order.push(id);
            stack.push({ id, exit: true });
            for (const child of [...(this.children.get(id) ?? [])].reverse())
                stack.push({ id: child, exit: false });
        }
        if (this.order.length !== items.length)
            throw new UiError(message("tagTree.cycle"));
        const ids = [...this.byId.keys()].sort();
        const common = (a: string, b = "") => {
            let i = 0;
            while (i < Math.min(a.length, b.length) && a[i] === b[i])
                i++;
            return i;
        };
        ids.forEach((id, i) => this.suffixes.set(id, id.slice(0, Math.max(8, common(id, ids[i - 1]) + 1, common(id, ids[i + 1]) + 1))));
    }
    contains(rootId: string, candidateId: string): boolean { const root = this.intervals.get(rootId), candidate = this.intervals.get(candidateId); return !!root && !!candidate && root[0] <= candidate[0] && candidate[0] < root[1]; }
    path(reference: string, limit = 4): TagPath {
        let tag = this.byRef.get(reference);
        if (!tag)
            throw new UiError(message("tagTree.unavailableTag"));
        const suffix = this.suffixes.get(tag.tagId)!;
        const segments: string[] = [];
        while (tag && segments.length < limit) {
            segments.push(tag.displayName);
            tag = tag.parentTagId === null ? undefined : this.byId.get(tag.parentTagId);
        }
        return { segments: segments.reverse(), truncated: !!tag, suffix };
    }
    label(reference: string, full = false): string { const path = this.path(reference, full ? 5000 : 4); return `${path.truncated ? "… › " : ""}${path.segments.map(segment => JSON.stringify(segment)).join(" › ")} · ${path.suffix}`; }
    page(search = "", offset = 0, excludedRoot?: string, size = 100): {
        items: TreeTag[];
        total: number;
        offset: number;
    } {
        const wanted = search.toLocaleLowerCase(), matches = this.order.filter(id => { const tag = this.byId.get(id)!; return (!excludedRoot || !this.contains(excludedRoot, id)) && (!wanted || tag.displayName.toLocaleLowerCase().includes(wanted) || tag.reference.toLocaleLowerCase().includes(wanted)); });
        const start = Math.max(0, Math.min(offset, Math.max(0, Math.ceil(matches.length / size) - 1) * size));
        return { items: matches.slice(start, start + size).map(id => this.byId.get(id)!), total: matches.length, offset: start };
    }
    visible(expanded: ReadonlySet<string>, offset = 0, size = 100): {
        items: TreeTag[];
        total: number;
        offset: number;
    } {
        const ids: string[] = [], stack = [...(this.children.get(null) ?? [])].reverse();
        while (stack.length) {
            const id = stack.pop()!;
            ids.push(id);
            if (expanded.has(id))
                for (const child of [...(this.children.get(id) ?? [])].reverse())
                    stack.push(child);
        }
        const start = Math.max(0, Math.min(offset, Math.max(0, Math.ceil(ids.length / size) - 1) * size));
        return { items: ids.slice(start, start + size).map(id => this.byId.get(id)!), total: ids.length, offset: start };
    }
}
export function tagPredicate(reference: string, mode: TagMatchMode, field = "tags"): string {
    if (mode !== "exact" && mode !== "subtree")
        throw new UiError(message("tagTree.invalidMatchMode"));
    const value = `tagRef(${JSON.stringify(reference)})`;
    return mode === "subtree" ? `${field}.exists(t, has(t.ref) ? tagInSubtree(t.ref, ${value}) : false)` : `${field}.exists(t, has(t.ref) && t.ref == ${value})`;
}
/** Called only while converting an existing persisted v1 shortcut collection. */
export function upgradeTagShortcuts<T extends {
    kind: string;
    matchMode?: unknown;
}>(items: T[]): (T & {
    matchMode?: TagMatchMode;
})[] {
    return items.map(item => {
        if (!item || typeof item !== "object" || typeof item.kind !== "string")
            throw new UiError(message("tagTree.invalidLegacyShortcut"));
        if (item.kind !== "tag")
            return item as T & {
                matchMode?: TagMatchMode;
            };
        if (item.matchMode === undefined)
            return { ...item, matchMode: "exact" };
        if (item.matchMode !== "exact" && item.matchMode !== "subtree")
            throw new UiError(message("tagTree.invalidSavedMode"));
        return item as T & {
            matchMode: TagMatchMode;
        };
    });
}
export function decodeTagShortcutCollection(value: unknown): {
    version: 2;
    items: Record<string, unknown>[];
} {
    if (Array.isArray(value))
        return { version: 2, items: upgradeTagShortcuts(value) };
    const collection = value as {
        version?: unknown;
        items?: unknown;
    };
    if (!collection || collection.version !== 2 || !Array.isArray(collection.items))
        throw new UiError(message("tagTree.invalidShortcutCollection"));
    for (const item of collection.items)
        if (!item || typeof item !== "object" || item.kind === "tag" && item.matchMode !== "exact" && item.matchMode !== "subtree")
            throw new UiError(message("tagTree.missingShortcutMode"));
    return { version: 2, items: collection.items };
}
