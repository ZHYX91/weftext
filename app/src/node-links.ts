import { message, UiError, type UiText } from "./ui-locale";
import type { Inventory, InventoryEntry } from "./types";

export type NodeLinkInventory = Pick<Inventory, "root" | "workspaceId" | "complete">;
export type NodeLinkRequest = <T>(path: string, body: unknown) => Promise<T>;
const uuid = (value: unknown): value is string => typeof value === "string"
    && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(value);

/** This only controls discoverability. Every chosen identity is resolved again before delivery. */
export function nodeLinkCopyUnavailable(inventory: NodeLinkInventory | null | undefined, entries: readonly InventoryEntry[]): UiText | undefined {
    if (!inventory || typeof inventory.root !== "string" || !inventory.root || !uuid(inventory.workspaceId))
        return message("nodeLinks.libraryUnavailable");
    if (inventory.complete !== true)
        return message("nodeLinks.libraryIncomplete");
    if (!entries.length || entries.some(entry => entry.kind !== "node" || entry.identityStatus !== "admitted"
        || typeof entry.path !== "string" || !/\.typ$/i.test(entry.path) || !uuid(entry.nodeId)))
        return message("nodeLinks.selectionUnsupported");
    if (new Set(entries.map(entry => entry.nodeId)).size !== entries.length)
        return message("nodeLinks.duplicateSelection");
    return undefined;
}

/** Read-only qualification of the complete captured selection, in displayed order.
 * No clipboard or author edit occurs here. Paths never become the copied identity. */
export async function qualifyNodeLinks(inventory: NodeLinkInventory, entries: readonly InventoryEntry[], request: NodeLinkRequest,
    isCurrent: () => boolean): Promise<string[]> {
    const captured = { root: inventory.root, workspaceId: inventory.workspaceId, complete: inventory.complete };
    const selected = entries.map(entry => ({ ...entry }));
    const current = (): void => {
        if (!isCurrent() || inventory.root !== captured.root || inventory.workspaceId !== captured.workspaceId || inventory.complete !== captured.complete)
            throw new UiError(message("nodeLinks.selectionChanged"));
    };
    current();
    const unavailable = nodeLinkCopyUnavailable(captured, selected);
    if (unavailable) throw new UiError(unavailable);
    const references: string[] = [];
    for (const entry of selected) {
        current();
        const entityKey = `weftext-node://${captured.workspaceId}/${entry.nodeId}`;
        let result: unknown;
        try {
            result = await request<unknown>("/api/data/resolve", { root: captured.root, entityKey });
        }
        finally {
            // Reject a late result or error from a retired library/selection.
            current();
        }
        if (!result || typeof result !== "object" || Array.isArray(result))
            throw new UiError(message("nodeLinks.resolutionChanged"));
        const resolved = result as Record<string, unknown>;
        if (resolved.root !== captured.root || resolved.nodeId !== entry.nodeId || typeof resolved.path !== "string" || !resolved.path)
            throw new UiError(message("nodeLinks.resolutionChanged"));
        references.push(entityKey);
    }
    current();
    return references;
}
