import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { InventoryEntry } from "./types";
import { canonicalNodeSources } from "./file-tree";
export type ContentSort = "name" | "path" | "authored";
export type ContentsSection = "nodes" | "album" | "files";
export interface ContentsSectionPreference { collapsed: boolean; limit: number; scroll: number }
export interface ContentsPreference {
    sort: ContentSort;
    thumbnails: boolean;
    summaries: boolean;
    density: "compact" | "comfortable";
    sections: Record<ContentsSection, ContentsSectionPreference>;
}
const defaultSection = (): ContentsSectionPreference => ({ collapsed: false, limit: 24, scroll: 0 });
export const defaultContentsPreference = (): ContentsPreference => ({ sort: "name", thumbnails: true, summaries: false, density: "compact", sections: { nodes: defaultSection(), album: defaultSection(), files: defaultSection() } });
export function decodeContentsPreference(value: unknown): ContentsPreference {
    const item = value as Partial<ContentsPreference> | null, defaults = defaultContentsPreference();
    const section = (name: ContentsSection): ContentsSectionPreference => {
        const saved = item?.sections?.[name];
        return {
            collapsed: typeof saved?.collapsed === "boolean" ? saved.collapsed : false,
            limit: Number.isSafeInteger(saved?.limit) && saved!.limit >= 24 && saved!.limit <= 5000 ? saved!.limit : 24,
            scroll: typeof saved?.scroll === "number" && Number.isFinite(saved.scroll) && saved.scroll >= 0 && saved.scroll <= 10000000 ? saved.scroll : 0,
        };
    };
    return { sort: item && ["name", "path", "authored"].includes(item.sort ?? "") ? item.sort! : defaults.sort,
        thumbnails: typeof item?.thumbnails === "boolean" ? item.thumbnails : defaults.thumbnails,
        summaries: typeof item?.summaries === "boolean" ? item.summaries : defaults.summaries,
        density: item?.density === "comfortable" ? "comfortable" : "compact",
        sections: { nodes: section("nodes"), album: section("album"), files: section("files") } };
}
/** Display grouping never grants a media read or changes the inventory identity. */
export function contentsSection(entry: Pick<InventoryEntry, "kind" | "path">): ContentsSection {
    if (entry.kind === "directory" || entry.kind === "node") return "nodes";
    return entry.kind !== "blocked" && /\.(?:png|jpe?g|gif|webp|bmp|tiff?|avif|heic|heif|pdf)$/i.test(entry.path) ? "album" : "files";
}
const normalizeContentsPath = (path: string) => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
export function directContentsEntries(entries: readonly InventoryEntry[], folder: string): InventoryEntry[] {
    const selected = normalizeContentsPath(folder);
    // A known birth retains kind=node after an external move or identity
    // conflict. Only its admitted canonical source represents this folder;
    // every other visible source stays available for investigation.
    const canonical = canonicalNodeSources(entries);
    return entries.filter(entry => normalizeContentsPath(entry.parentPath) === selected && normalizeContentsPath(entry.path) !== selected
        && canonical.get(selected) !== entry).map(entry => {
        if (entry.kind !== "directory") return entry;
        const source = canonical.get(normalizeContentsPath(entry.path));
        return source?.title ? { ...entry, title: source.title } : entry;
    });
}
/** A node's display directory and exact admitted source have different jobs.
 * Open/menu actions may use this captured source; browsing keeps the directory.
 * An ordinary, missing or conflicting pair never gains a guessed note target. */
export function contentsActionEntry(entry: InventoryEntry, canonicalSources: ReadonlyMap<string, InventoryEntry>): InventoryEntry {
    return entry.kind === "directory" ? canonicalSources.get(normalizeContentsPath(entry.path)) ?? entry : entry;
}
/** Selection is presentation state, never write permission. Bulk selection
 * excludes blocked and unresolved identities; explicit single selection can
 * still expose their existing diagnostics without inventing a healthy node. */
export function contentsBatchSelectable(entry: InventoryEntry): boolean {
    return entry.kind === "node" ? entry.identityStatus === "admitted" && !!entry.nodeId
        : entry.kind !== "blocked" && ["ordinary", "unmanaged", "admitted"].includes(entry.identityStatus);
}
export interface ContentsSelectionRow { path: string; eligible: boolean }
export interface ContentsSelectionIntent { range?: boolean; additive?: boolean; toggle?: boolean }
/** The caller supplies exactly the expanded, rendered rows in display order.
 * Hidden pages/sections/filter results must never extend a range implicitly. */
export function changedContentsSelection(current: ReadonlySet<string>, displayed: readonly ContentsSelectionRow[], target: string, anchor: string | undefined, intent: ContentsSelectionIntent = {}): { selected: Set<string>; anchor: string | undefined } {
    const end = displayed.findIndex(row => row.path === target);
    if (end < 0) return { selected: new Set(current), anchor };
    if (intent.range) {
        const found = displayed.findIndex(row => row.path === anchor && row.eligible), start = found < 0 ? end : found;
        const selected = intent.additive ? new Set(current) : new Set<string>();
        for (const row of displayed.slice(Math.min(start, end), Math.max(start, end) + 1))
            if (row.eligible) selected.add(row.path);
        return { selected, anchor: found < 0 ? target : anchor };
    }
    const selected = intent.additive || intent.toggle ? new Set(current) : new Set<string>();
    if (intent.toggle && selected.has(target)) selected.delete(target);
    else selected.add(target);
    return { selected, anchor: target };
}
/** Preferences belong to an exact library and browsing folder, including ordinary folders. */
export function contentsPreferenceKey(root: string, workspaceId: string | undefined, folder: string): string {
    return `weftext-node-contents:v2:${encodeURIComponent(JSON.stringify([workspaceId ?? normalizeContentsPath(root), normalizeContentsPath(folder)]))}`;
}
export interface AuthoredMember {
    targetRef: string;
    path: string;
    name: string;
}
export interface AuthoredOrder {
    parentRef: string;
    baseId: string;
    beforeSource: string | null;
    members: AuthoredMember[];
    orderedRefs: string[];
    complete: boolean;
}
/** A fresh order is usable only when its complete member projection and order
 * describe the same exact parent and identities. Never repair a malformed read
 * with a previous list or with the requested order. */
export function decodeAuthoredOrder(value: unknown, parentRef: string): AuthoredOrder {
    const read = value as Partial<AuthoredOrder> | null;
    if (!read || read.parentRef !== parentRef || typeof read.baseId !== "string" || !read.baseId
        || read.beforeSource !== null && typeof read.beforeSource !== "string" || typeof read.complete !== "boolean"
        || !Array.isArray(read.members) || !Array.isArray(read.orderedRefs)
        || read.members.some(member => !member || typeof member.targetRef !== "string" || !member.targetRef || typeof member.path !== "string" || !member.path || typeof member.name !== "string")
        || read.orderedRefs.some(ref => typeof ref !== "string" || !ref))
        throw new UiError(uiMessage("nodeContents.parentReceiptMismatch"));
    const members = new Set(read.members.map(member => member.targetRef)), ordered = new Set(read.orderedRefs);
    if (members.size !== read.members.length || ordered.size !== read.orderedRefs.length
        || members.size !== ordered.size || [...members].some(ref => !ordered.has(ref)))
        throw new UiError(uiMessage("nodeContents.parentReceiptMismatch"));
    return read as AuthoredOrder;
}
export function isTerminalOrderStatus(status: string): boolean {
    return status === "committed" || status === "resolved" || status === "cancelled";
}
export interface OrderPlan {
    operationId: string;
    planId: string;
    status: string;
    changes?: {
        path: string;
        action: string;
        bytes: number;
    }[];
}
export interface PendingOrder {
    request: {
        root: string;
        parentRef: string;
        children: string[];
        baseId: string;
        operationId: string;
    };
    beforeRefs: string[];
    plan?: OrderPlan;
    attempted?: boolean;
    status: string;
}
export function movedSibling(refs: readonly string[], target: string, offset: -1 | 1): string[] {
    const index = refs.indexOf(target), next = index + offset;
    if (index < 0 || next < 0 || next >= refs.length)
        return [...refs];
    const result = [...refs];
    [result[index], result[next]] = [result[next], result[index]];
    return result;
}
export function insertedSibling(refs: readonly string[], source: string, target: string, after: boolean): string[] {
    if (source === target || !refs.includes(source) || !refs.includes(target))
        return [...refs];
    const result = refs.filter(ref => ref !== source);
    result.splice(result.indexOf(target) + Number(after), 0, source);
    return result;
}
export function decodePendingOrder(raw: string, root: string, parentRef: string): PendingOrder {
    if (raw.length > 1024 * 1024)
        throw new UiError(uiMessage("contentsState.authorOrderRecordBudget"));
    const value = JSON.parse(raw) as PendingOrder, request = value?.request;
    if (!request || request.root !== root || request.parentRef !== parentRef || !request.baseId || !request.operationId || !Array.isArray(request.children)
        || request.children.length > 5000 || request.children.some(ref => typeof ref !== "string" || !/^weftext-(node|directory):\/\//.test(ref)) || new Set(request.children).size !== request.children.length
        || !Array.isArray(value.beforeRefs) || value.beforeRefs.length !== request.children.length || new Set(value.beforeRefs).size !== value.beforeRefs.length || value.beforeRefs.some(ref => !request.children.includes(ref))
        || typeof value.status !== "string" || value.plan && (value.plan.operationId !== request.operationId || !value.plan.planId))
        throw new UiError(uiMessage("contentsState.authorOrderRecordMismatch"));
    return value;
}
