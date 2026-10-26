import type { InventoryEntry } from "./types";

export interface FileTreeEntry extends InventoryEntry {
    /** The real directory used for browsing; path remains the source to open. */
    folderPath: string | null;
    /** Display position, separate from a merged node's canonical source path. */
    treePath: string;
    hasChildren: boolean;
    collapsed: boolean;
}
const normalized = (path: string): string => path.replaceAll("\\", "/").replace(/\/$/, "").toLowerCase();
function* ancestors(path: string): Generator<string> {
    for (let slash = path.lastIndexOf("/"); slash >= 0; slash = path.lastIndexOf("/")) {
        path = path.slice(0, slash);
        if (path) yield path;
    }
}
export function fileTreeEntries(
    entries: readonly InventoryEntry[],
    selectedFolder: string | null,
    query: string,
    collapsed: ReadonlySet<string>,
): FileTreeEntry[] {
    const directories = new Set(entries.filter(entry => entry.kind === "directory").map(entry => normalized(entry.path)));
    const paired = new Map(entries.filter(entry => entry.kind === "node").map(entry => [normalized(entry.parentPath), entry]));
    const navigation = entries.flatMap(entry => {
        if (entry.kind === "node" && directories.has(normalized(entry.parentPath))) return [];
        const primary = entry.kind === "directory" ? paired.get(normalized(entry.path)) : undefined;
        const projected = primary ? { ...primary, relativePath: entry.relativePath, name: entry.name } : entry;
        const folderPath = entry.kind === "directory" ? entry.path : null;
        return [{ ...projected, folderPath, treePath: folderPath ?? entry.path }];
    });
    const parents = new Set<string>();
    for (const entry of navigation) {
        for (const ancestor of ancestors(normalized(entry.treePath))) parents.add(ancestor);
    }
    const collapsedPaths = new Set([...collapsed].map(normalized));
    const scope = selectedFolder ? normalized(selectedFolder) : null;
    const needle = query.trim() ? query.toLocaleLowerCase() : "";
    return navigation.filter(entry => {
        const path = normalized(entry.treePath);
        if (scope && path !== scope && !path.startsWith(scope + "/")) return false;
        if (needle) return `${entry.name} ${entry.relativePath} ${entry.title ?? ""}`.toLocaleLowerCase().includes(needle);
        // Entering a folder establishes the visible tree root. A collapsed
        // ancestor outside that scope must not hide the root or its contents.
        return ![...ancestors(path)].some(parent => collapsedPaths.has(parent)
            && (!scope || parent === scope || parent.startsWith(scope + "/")));
    }).map(entry => ({
        ...entry,
        hasChildren: entry.folderPath !== null && parents.has(normalized(entry.folderPath)),
        collapsed: entry.folderPath !== null && collapsedPaths.has(normalized(entry.folderPath)),
    }));
}
