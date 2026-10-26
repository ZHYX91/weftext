import { test } from "node:test";
import assert from "node:assert/strict";
import { fileTreeEntries } from "./file-tree";
import type { InventoryEntry } from "./types";

const root = "C:/Library";
const directory = (relative: string): InventoryEntry => ({ path: `${root}/${relative}`, relativePath: relative, name: relative.split("/").at(-1)!, parentPath: `${root}/${relative.slice(0, Math.max(0, relative.lastIndexOf("/")))}`.replace(/\/$/, ""), kind: "directory", identityStatus: "ordinary" });
const node = (relative: string, title?: string): InventoryEntry => ({ path: `${root}/${relative}/${relative.split("/").at(-1)}.typ`, relativePath: `${relative}/${relative.split("/").at(-1)}.typ`, name: `${relative.split("/").at(-1)}.typ`, parentPath: `${root}/${relative}`, kind: "node", identityStatus: "admitted", nodeId: `id:${relative}`, title });
const file = (relative: string): InventoryEntry => ({ ...directory(relative), kind: "file" });
const rows = (entries: InventoryEntry[], collapsed: string[] = [], query = "", scope: string | null = null) => fileTreeEntries(entries, scope, query, new Set(collapsed));

test("collapsing a merged node keeps the parent and only hides its descendants", () => {
    const entries = [directory("NativeTable"), node("NativeTable"), directory("NativeTable/Child"), node("NativeTable/Child"), directory("IdentityReject"), node("IdentityReject")];
    const initial = rows(entries);
    assert.deepEqual(initial.map(row => row.treePath), [`${root}/NativeTable`, `${root}/NativeTable/Child`, `${root}/IdentityReject`]);
    assert.equal(initial[0].path, `${root}/NativeTable/NativeTable.typ`);
    assert.equal(initial[0].nodeId, "id:NativeTable");
    assert.equal(initial[0].hasChildren, true);
    assert.equal(initial[1].hasChildren, false);
    assert.equal(initial[2].hasChildren, false);
    const folded = rows(entries, [`${root}/NativeTable`]);
    assert.deepEqual(folded.map(row => row.treePath), [`${root}/NativeTable`, `${root}/IdentityReject`]);
    assert.equal(folded[0].collapsed, true);
    assert.equal(folded[0].hasChildren, true);
    assert.deepEqual(rows(entries).map(row => row.treePath), initial.map(row => row.treePath));
});

test("leaf notes and empty directories have no expansion control; real files count as children", () => {
    const entries = [directory("Empty"), directory("Leaf"), node("Leaf"), directory("Files"), file("Files/image.png")];
    assert.deepEqual(rows(entries).map(row => row.hasChildren), [false, false, true, false]);
    assert.deepEqual(rows(entries, [`${root}/Files`]).map(row => row.name), ["Empty", "Leaf", "Files"]);
});

test("nested collapse uses directory coordinates, preserves siblings with shared prefixes and normalizes Windows paths", () => {
    const entries = [directory("A"), node("A"), directory("A/B"), node("A/B"), file("A/B/child.typ"), directory("AB"), node("AB")];
    assert.deepEqual(rows(entries, ["c:\\LIBRARY\\a\\b\\"]).map(row => row.treePath), [`${root}/A`, `${root}/A/B`, `${root}/AB`]);
    assert.deepEqual(rows(entries, ["c:\\library\\a"]).map(row => row.treePath), [`${root}/A`, `${root}/AB`]);
});

test("scope and search retain original behavior while folding never removes the scoped parent", () => {
    const entries = [directory("A"), node("A", "Parent"), directory("A/B"), node("A/B", "Needle"), directory("Outside"), node("Outside", "Needle")];
    assert.deepEqual(rows(entries, [`${root}/A`], "", `${root}/A`).map(row => row.treePath), [`${root}/A`]);
    const searched = rows(entries, [`${root}/A`], "needle", `${root}/A`);
    assert.deepEqual(searched.map(row => row.treePath), [`${root}/A/B`]);
    assert.equal(searched[0].path, `${root}/A/B/B.typ`);
    assert.equal(searched[0].folderPath, `${root}/A/B`);
});

test("projection keeps inventory order and original entries unchanged", () => {
    const entries = [directory("Z"), node("Z", "Last alphabetically"), file("outside.typ"), directory("A"), node("A", "First alphabetically")];
    const original = structuredClone(entries);
    const projected = rows(entries);
    assert.deepEqual(projected.map(row => row.name), ["Z", "outside.typ", "A"]);
    assert.deepEqual(projected.map(row => row.path), [`${root}/Z/Z.typ`, `${root}/outside.typ`, `${root}/A/A.typ`]);
    assert.deepEqual(entries, original);
});

test("entering a scope ignores collapsed ancestors but keeps folding within that scope", () => {
    const entries = [directory("A"), node("A"), directory("A/B"), node("A/B"), directory("A/B/C"), node("A/B/C"), directory("Outside"), node("Outside")];
    assert.deepEqual(rows(entries, [`${root}/A`], "", `${root}/A/B`).map(row => row.treePath), [`${root}/A/B`, `${root}/A/B/C`]);
    assert.deepEqual(rows(entries, [`${root}/A`, `${root}/A/B`], "", `${root}/A/B`).map(row => row.treePath), [`${root}/A/B`]);
    assert.deepEqual(rows(entries, [`${root}/A`], "", `${root}/Outside`).map(row => row.treePath), [`${root}/Outside`]);
});
