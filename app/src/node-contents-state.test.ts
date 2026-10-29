import test from "node:test";
import assert from "node:assert/strict";
import { changedContentsSelection, contentsActionEntry, contentsBatchSelectable, contentsPreferenceKey, contentsSection, directContentsEntries, decodeAuthoredOrder, decodeContentsPreference, decodePendingOrder, defaultContentsPreference, insertedSibling, isTerminalOrderStatus, movedSibling } from "./node-contents-state";
import { canonicalNodeSources } from "./file-tree";
import type { InventoryEntry } from "./types";
test("author reordering is explicit and preserves exact stable identities", () => {
  const refs = ["same-name:first", "same-name:second", "other"];
  assert.deepEqual(movedSibling(refs, refs[1], -1), [refs[1], refs[0], refs[2]]);
  assert.deepEqual(movedSibling(refs, refs[0], -1), refs);
  assert.deepEqual(movedSibling(refs, "same-name", 1), refs);
  assert.deepEqual(insertedSibling(refs, refs[0], refs[2], true), [refs[1], refs[2], refs[0]]);
  assert.deepEqual(refs, ["same-name:first", "same-name:second", "other"]);
  assert.equal(decodeContentsPreference({ sort: "path", thumbnails: true }).sort, "path");
  assert.equal(decodeContentsPreference({ sort: "manual" }).sort, "name");
});
test("folder sections distinguish media, notes, directories and blocked identities without granting reads", () => {
  assert.equal(contentsSection({ kind: "directory", path: "D:/library/photo.png" }), "nodes");
  assert.equal(contentsSection({ kind: "node", path: "D:/library/A.typ" }), "nodes");
  assert.equal(contentsSection({ kind: "file", path: "D:/library/Legacy.TYP" }), "files");
  assert.equal(contentsSection({ kind: "file", path: "D:/library/A.PNG" }), "album");
  assert.equal(contentsSection({ kind: "file", path: "D:/library/A.pdf" }), "album");
  assert.equal(contentsSection({ kind: "file", path: "D:/library/A.mp4" }), "files");
  assert.equal(contentsSection({ kind: "blocked", path: "D:/library/A.png" }), "files");
});
test("node contents excludes its own canonical note and merges only an actual child node's display title", () => {
  const entry = (path: string, parentPath: string, kind: "directory" | "node" | "file", title?: string) => ({ path, parentPath, kind, title, name: path.split("/").at(-1)!, relativePath: path.slice(11), identityStatus: kind === "node" ? "admitted" : "ordinary" });
  const own = entry("D:/library/A/A.typ", "D:/library/A", "node", "Own title");
  const child = entry("D:/library/A/B", "D:/library/A", "directory");
  const canonical = entry("D:/library/A/B/B.typ", "D:/library/A/B", "node", "Child title");
  const ordinary = entry("D:/library/A/Loose.typ", "D:/library/A", "file");
  const rows = directContentsEntries([own, child, canonical, ordinary], "D:\\library\\A\\");
  assert.deepEqual(rows.map(row => [row.path, row.title]), [[child.path, "Child title"], [ordinary.path, undefined]]);
  assert.equal(rows[0].kind, "directory");
  assert.equal(rows[1], ordinary);
  assert.equal(child.title, undefined);
});
test("externally moved and conflicting known nodes stay visible and cannot supply a folder's canonical title", () => {
  const entry = (path: string, parentPath: string, kind: "directory" | "node" | "file", title?: string, identityStatus = "ordinary") => ({ path, parentPath, kind, title, identityStatus, name: path.split("/").at(-1)!, relativePath: path.slice(11) });
  const own = entry("D:/library/A/A.typ", "D:/library/A", "node", "Own title", "admitted");
  const moved = entry("D:/library/A/Moved.typ", "D:/library/A", "node", "Moved title", "bindingConflict");
  const child = entry("D:/library/A/B", "D:/library/A", "directory");
  const wrong = entry("D:/library/A/B/Moved.typ", "D:/library/A/B", "node", "Wrong canonical title", "bindingConflict");
  const conflictingCanonical = entry("D:/library/A/B/B.typ", "D:/library/A/B", "node", "Conflicting title", "bindingConflict");
  const looseCanonical = entry("D:/library/A/A.typ", "D:/library/A", "file", "Ordinary same coordinate");
  const rows = directContentsEntries([own, moved, child, wrong, conflictingCanonical, looseCanonical], "D:/library/A");
  assert.deepEqual(rows, [moved, child, looseCanonical]);
  assert.equal(rows[0].identityStatus, "bindingConflict");
  assert.equal(rows[1].title, undefined);
});
test("each folder retains independent section positions and ignores invalid persisted geometry", () => {
  const value = decodeContentsPreference({ sections: {
    nodes: { collapsed: true, limit: 48, scroll: 122 },
    album: { collapsed: false, limit: Infinity, scroll: -1 },
    files: { collapsed: "yes", limit: 0, scroll: NaN },
  } });
  assert.deepEqual(value.sections.nodes, { collapsed: true, limit: 48, scroll: 122 });
  assert.deepEqual(value.sections.album, { collapsed: false, limit: 24, scroll: 0 });
  assert.deepEqual(value.sections.files, { collapsed: false, limit: 24, scroll: 0 });
  const fresh = defaultContentsPreference();
  fresh.sections.nodes.collapsed = true;
  assert.equal(defaultContentsPreference().sections.nodes.collapsed, false);
  assert.equal(contentsPreferenceKey("D:\\Library\\", undefined, "D:\\Library\\A"), contentsPreferenceKey("d:/library", undefined, "d:/library/a/"));
  assert.notEqual(contentsPreferenceKey("D:/library", "workspace-a", "D:/library/A"), contentsPreferenceKey("D:/library", "workspace-b", "D:/library/A"));
  assert.notEqual(contentsPreferenceKey("D:/library", undefined, "D:/library/A"), contentsPreferenceKey("D:/library", undefined, "D:/library/B"));
});
test("a current folder keeps its conflicting or unadmitted same-coordinate main source visible", () => {
  for (const identityStatus of ["bindingConflict", "duplicate", "unadmitted", "invalid", "missing"]) {
    const main: InventoryEntry = { path: "D:/library/A/A.typ", parentPath: "D:/library/A", relativePath: "A/A.typ", name: "A.typ", kind: "node", identityStatus };
    assert.deepEqual(directContentsEntries([main], "D:\\library\\A\\"), [main]);
  }
  const admitted: InventoryEntry = { path: "D:/library/A/A.typ", parentPath: "D:/library/A", relativePath: "A/A.typ", name: "A.typ", kind: "node", identityStatus: "admitted" };
  assert.deepEqual(directContentsEntries([admitted], "D:/library/A"), []);
});
test("node content actions bind the captured admitted source while directory browsing keeps its coordinate", () => {
  const folder: InventoryEntry = { path: "D:/library/A", parentPath: "D:/library", relativePath: "A", name: "A", kind: "directory", identityStatus: "ordinary" };
  const source: InventoryEntry = { path: "D:/library/A/A.typ", parentPath: folder.path, relativePath: "A/A.typ", name: "A.typ", kind: "node", identityStatus: "admitted", nodeId: "admitted-a", title: "Actual title" };
  const captured = canonicalNodeSources([folder, source]);
  assert.equal(contentsActionEntry(folder, captured), source);
  assert.equal(contentsActionEntry(source, captured), source);
  assert.equal(contentsActionEntry({ ...folder, path: "d:\\LIBRARY\\a\\" }, captured), source);
  assert.equal(folder.path, "D:/library/A");
  for (const identityStatus of ["bindingConflict", "duplicate", "unadmitted"]) {
    assert.equal(contentsActionEntry(folder, canonicalNodeSources([folder, { ...source, identityStatus }])), folder);
  }
  assert.equal(contentsActionEntry(folder, canonicalNodeSources([folder, { ...source, path: "D:/library/A/Other.typ" }])), folder);
});
test("order recovery binds original parent, whole request and original plan", () => {
  const pending = { request: { root: "root", parentRef: "parent", baseId: "basis", operationId: "once", children: ["weftext-node://w/a", "weftext-node://w/b"] }, beforeRefs:["weftext-node://w/b", "weftext-node://w/a"], status: "prepared", plan: { operationId: "once", planId: "plan", status: "prepared" } };
  assert.deepEqual(decodePendingOrder(JSON.stringify(pending), "root", "parent"), pending);
  assert.throws(() => decodePendingOrder(JSON.stringify(pending), "root", "same-name-new-parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, plan: { ...pending.plan, operationId: "another" } }), "root", "parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, request: { ...pending.request, children: ["weftext-node://w/a", "weftext-node://w/a"] } }), "root", "parent"));
});

test("fresh order retirement requires an exact, internally consistent parent projection", () => {
  const read = { parentRef: "parent", baseId: "fresh", beforeSource: null, complete: true, members: [{ targetRef: "a", path: "A", name: "A" }, { targetRef: "b", path: "B", name: "B" }], orderedRefs: ["b", "a"] };
  assert.deepEqual(decodeAuthoredOrder(read, "parent"), read);
  assert.equal(decodeAuthoredOrder({ ...read, complete: false }, "parent").complete, false);
  for (const malformed of [
    { ...read, parentRef: "same-name-replacement" }, { ...read, baseId: "" },
    { ...read, complete: undefined }, { ...read, orderedRefs: ["a", "a"] },
    { ...read, orderedRefs: ["a", "other"] }, { ...read, orderedRefs: ["a"] },
    { ...read, members: [read.members[0], read.members[0]] },
    { ...read, members: [{ ...read.members[0], path: null }, read.members[1]] },
  ]) assert.throws(() => decodeAuthoredOrder(malformed, "parent"));
  for (const status of ["committed", "resolved", "cancelled"]) assert.equal(isTerminalOrderStatus(status), true);
  for (const status of ["unknown", "prepared", "preparing", "failed", ""]) assert.equal(isTerminalOrderStatus(status), false);
});

test("range selection follows only the current displayed sequence and excludes unresolved identities", () => {
  const rows = [{ path: "node/z", eligible: true }, { path: "node/conflict", eligible: false }, { path: "album/a", eligible: true }, { path: "file/b", eligible: true }];
  const initial = new Set(["retained/not-displayed"]);
  const range = changedContentsSelection(initial, rows, "album/a", "node/z", { range: true });
  assert.deepEqual([...range.selected], ["node/z", "album/a"]);
  assert.equal(range.anchor, "node/z");
  assert.deepEqual([...initial], ["retained/not-displayed"]);
  const extended = changedContentsSelection(initial, rows, "album/a", "node/z", { range: true, additive: true });
  assert.deepEqual([...extended.selected], ["retained/not-displayed", "node/z", "album/a"]);
  assert.deepEqual([...changedContentsSelection(initial, rows, "file/b", "old/filtered-anchor", { range: true }).selected], ["file/b"]);
  assert.deepEqual([...changedContentsSelection(initial, rows, "file/b", "node/conflict", { range: true }).selected], ["file/b"]);
  assert.deepEqual([...changedContentsSelection(initial, rows, "retired/target", "node/z", { range: true }).selected], [...initial]);
  assert.deepEqual([...changedContentsSelection(initial, rows, "node/conflict", "node/z", { range: true }).selected], ["node/z"]);
});

test("point selection toggles without opening or upgrading a diagnostic item, and eligibility is explicit", () => {
  const rows = [{ path: "a", eligible: true }, { path: "conflict", eligible: false }];
  const point = changedContentsSelection(new Set(["other"]), rows, "a", undefined);
  assert.deepEqual([...point.selected], ["a"]);
  const toggled = changedContentsSelection(point.selected, rows, "a", point.anchor, { toggle: true });
  assert.deepEqual([...toggled.selected], []);
  assert.deepEqual([...changedContentsSelection(new Set(["a"]), rows, "conflict", "a", { additive: true, toggle: true }).selected], ["a", "conflict"]);
  const base: InventoryEntry = { path: "A", parentPath: "root", relativePath: "A", name: "A", kind: "file", identityStatus: "ordinary" };
  assert.equal(contentsBatchSelectable(base), true);
  assert.equal(contentsBatchSelectable({ ...base, kind: "directory", identityStatus: "unmanaged" }), true);
  assert.equal(contentsBatchSelectable({ ...base, kind: "node", identityStatus: "admitted", nodeId: "exact-admitted-id" }), true);
  assert.equal(contentsBatchSelectable({ ...base, kind: "node", identityStatus: "admitted" }), false);
  for (const identityStatus of ["bindingConflict", "boundaryConflict", "unadmitted", "duplicate", "missing", "recoveryPending", "unsupportedRules", "future-state"])
    assert.equal(contentsBatchSelectable({ ...base, kind: "node", identityStatus, nodeId: "not-authority" }), false);
  assert.equal(contentsBatchSelectable({ ...base, kind: "blocked" }), false);
});
