import test from "node:test";
import assert from "node:assert/strict";
import { nodeLinkCopyUnavailable, qualifyNodeLinks, type NodeLinkInventory, type NodeLinkRequest } from "./node-links";
import type { InventoryEntry } from "./types";
import type { UiText } from "./ui-locale";

const workspaceId = "10000000-0000-4000-8000-000000000001";
const nodeId = (index: number) => `20000000-0000-4000-8000-${index.toString(16).padStart(12, "0")}`;
const inventory = (): NodeLinkInventory => ({ root: "D:/library", workspaceId, complete: true });
const entry = (index: number, overrides: Partial<InventoryEntry> = {}): InventoryEntry => ({
    path: `D:/library/Note${index}/Note${index}.typ`, parentPath: `D:/library/Note${index}`, relativePath: `Note${index}/Note${index}.typ`,
    name: `Note${index}.typ`, kind: "node", identityStatus: "admitted", nodeId: nodeId(index), ...overrides,
});
const key = (reason: UiText | undefined): string | undefined => typeof reason === "object" ? reason.key : undefined;
function resolver(calls: { path: string; body: unknown }[]): NodeLinkRequest {
    return async <T>(path: string, body: unknown): Promise<T> => {
        calls.push({ path, body });
        const { root, entityKey } = body as { root: string; entityKey: string };
        // A proven rename/move is allowed; the returned path is not the stable link.
        return { root, nodeId: entityKey.split("/").at(-1), path: "D:/library/Moved/Moved.typ" } as T;
    };
}

test("menu eligibility rejects incomplete libraries, ordinary/mixed/conflicting sources and duplicate identities before any request", async () => {
    assert.equal(nodeLinkCopyUnavailable(inventory(), [entry(1)]), undefined);
    assert.equal(key(nodeLinkCopyUnavailable(undefined, [entry(1)])), "nodeLinks.libraryUnavailable");
    assert.equal(key(nodeLinkCopyUnavailable({ ...inventory(), workspaceId: "not-a-workspace" }, [entry(1)])), "nodeLinks.libraryUnavailable");
    assert.equal(key(nodeLinkCopyUnavailable({ ...inventory(), complete: false }, [entry(1)])), "nodeLinks.libraryIncomplete");
    assert.equal(key(nodeLinkCopyUnavailable(inventory(), [])), "nodeLinks.selectionUnsupported");
    const unsupported: Partial<InventoryEntry>[] = [{ kind: "directory" }, { kind: "file" }, { kind: "blocked" }, { path: "D:/library/photo.png" },
        { identityStatus: "duplicate" }, { identityStatus: "bindingConflict" }, { identityStatus: "unadmitted" }, { nodeId: undefined }, { nodeId: "../claimed-path" }, { nodeId: nodeId(1).toUpperCase().replace("000000000001", "00000000000A") }];
    let requests = 0;
    const unexpected: NodeLinkRequest = async () => { requests++; throw new Error("No request should occur for an ineligible batch"); };
    for (const override of unsupported) {
        const selected = [entry(1), entry(2, override)];
        assert.equal(key(nodeLinkCopyUnavailable(inventory(), selected)), "nodeLinks.selectionUnsupported");
        await assert.rejects(qualifyNodeLinks(inventory(), selected, unexpected, () => true));
    }
    assert.equal(key(nodeLinkCopyUnavailable(inventory(), [entry(1), entry(2, { nodeId: nodeId(1) })])), "nodeLinks.duplicateSelection");
    await assert.rejects(qualifyNodeLinks({ ...inventory(), complete: false }, [entry(1)], unexpected, () => true));
    await assert.rejects(qualifyNodeLinks(inventory(), [entry(1), entry(1)], unexpected, () => true));
    assert.equal(requests, 0);
});

test("all targets are freshly resolved in display order and only stable references are returned after a proven path move", async () => {
    const calls: { path: string; body: unknown }[] = [];
    const selected = [entry(3), entry(1), entry(2)];
    const links = await qualifyNodeLinks(inventory(), selected, resolver(calls), () => true);
    assert.deepEqual(links, [3, 1, 2].map(index => `weftext-node://${workspaceId}/${nodeId(index)}`));
    assert.deepEqual(calls, links.map(entityKey => ({ path: "/api/data/resolve", body: { root: "D:/library", entityKey } })));
    assert.ok(links.every(link => !link.includes("D:") && !link.includes("Moved") && !link.includes("Note")));
    assert.deepEqual(selected, [entry(3), entry(1), entry(2)], "qualification does not mutate selection or inventory entries");
});

test("canonical non-v4 identities keep the existing Core UUID boundary", async () => {
    const scope = { ...inventory(), workspaceId: "019a2b3c-4d5e-7abc-8123-456789abcdef" };
    const selected = [entry(1, { nodeId: "019a2b3c-4d5e-7def-9234-56789abcdef0" })];
    assert.equal(nodeLinkCopyUnavailable(scope, selected), undefined);
    assert.deepEqual(await qualifyNodeLinks(scope, selected, resolver([]), () => true), [
        "weftext-node://019a2b3c-4d5e-7abc-8123-456789abcdef/019a2b3c-4d5e-7def-9234-56789abcdef0",
    ]);
    for (const malformed of ["019A2B3C-4D5E-7DEF-9234-56789ABCDEF0", "019a2b3c4d5e7def923456789abcdef0", "{019a2b3c-4d5e-7def-9234-56789abcdef0}"])
        assert.notEqual(nodeLinkCopyUnavailable(scope, [entry(1, { nodeId: malformed })]), undefined);
});

test("a batch has no smaller client truncation: every supplied valid note is qualified and returned", async () => {
    const selected = Array.from({ length: 129 }, (_, index) => entry(index + 1)), calls: { path: string; body: unknown }[] = [];
    const links = await qualifyNodeLinks(inventory(), selected, resolver(calls), () => true);
    assert.equal(calls.length, selected.length);
    assert.deepEqual(links, selected.map(item => `weftext-node://${workspaceId}/${item.nodeId}`));
});

test("wrong library/identity and malformed resolver receipts stop the complete batch with no delivery", async () => {
    for (const malformed of [null, [], "a path is not a receipt", {}, { root: "D:/other", nodeId: nodeId(1), path: "D:/other/A.typ" },
        { root: 1, nodeId: nodeId(1), path: "D:/library/A.typ" }, { root: "D:/library", nodeId: nodeId(2), path: "D:/library/A.typ" },
        { root: "D:/library", nodeId: 1, path: "D:/library/A.typ" }, { root: "D:/library", nodeId: nodeId(1), path: null },
        { root: "D:/library", nodeId: nodeId(1), path: "" }]) {
        let requests = 0, externalWrites = 0;
        const request: NodeLinkRequest = async <T>(): Promise<T> => { requests++; return malformed as T; };
        await assert.rejects(async () => { await qualifyNodeLinks(inventory(), [entry(1), entry(2)], request, () => true); externalWrites++; });
        assert.equal(requests, 1);
        assert.equal(externalWrites, 0);
    }
});

test("selection retirement before or during a read rejects a late result and never reads the next target", async () => {
    let current = false, requests = 0, externalWrites = 0;
    const never: NodeLinkRequest = async () => { requests++; throw new Error("Unexpected retired request"); };
    await assert.rejects(qualifyNodeLinks(inventory(), [entry(1)], never, () => current));
    assert.equal(requests, 0);
    current = true;
    let finish!: (value: unknown) => void;
    const delayed: NodeLinkRequest = async <T>(): Promise<T> => { requests++; return await new Promise<unknown>(resolve => { finish = resolve; }) as T; };
    const result = qualifyNodeLinks(inventory(), [entry(1), entry(2)], delayed, () => current).then(() => { externalWrites++; });
    current = false; finish({ root: "D:/library", nodeId: nodeId(1), path: "D:/library/renamed.typ" });
    await assert.rejects(result);
    assert.equal(requests, 1);
    assert.equal(externalWrites, 0);
});

test("in-place scope changes cannot retarget a captured batch even if the caller retains the inventory object", async () => {
    const captured = inventory(); let requests = 0;
    const changed: NodeLinkRequest = async <T>(): Promise<T> => {
        requests++; captured.workspaceId = "30000000-0000-4000-8000-000000000001";
        return { root: "D:/library", nodeId: nodeId(1), path: "D:/library/A.typ" } as T;
    };
    await assert.rejects(qualifyNodeLinks(captured, [entry(1), entry(2)], changed, () => true));
    assert.equal(requests, 1);
});

test("a later resolver failure returns no prefix; retired errors cannot disclose an old scope's failure details", async () => {
    const calls: { path: string; body: unknown }[] = [], read = resolver(calls); let requests = 0, externalWrites = 0;
    const failure = new Error("Current target is unavailable");
    const refused: NodeLinkRequest = async <T>(path: string, body: unknown): Promise<T> => { if (++requests === 2) throw failure; return read<T>(path, body); };
    await assert.rejects(async () => { await qualifyNodeLinks(inventory(), [entry(1), entry(2), entry(3)], refused, () => true); externalWrites++; }, error => error === failure);
    assert.equal(requests, 2); assert.equal(externalWrites, 0);
    let current = true;
    const retired: NodeLinkRequest = async () => { current = false; throw new Error("OLD_PROTECTED_FAILURE_DETAIL"); };
    await assert.rejects(qualifyNodeLinks(inventory(), [entry(1)], retired, () => current), error => error instanceof Error && !error.message.includes("OLD_PROTECTED_FAILURE_DETAIL"));
});
