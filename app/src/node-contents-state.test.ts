import test from "node:test";
import assert from "node:assert/strict";
import { contentsPreferenceKey, contentsSection, directContentsEntries, decodeContentsPreference, decodePendingOrder, defaultContentsPreference, insertedSibling, movedSibling } from "./node-contents-state";
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
test("order recovery binds original parent, whole request and original plan", () => {
  const pending = { request: { root: "root", parentRef: "parent", baseId: "basis", operationId: "once", children: ["weftext-node://w/a", "weftext-node://w/b"] }, beforeRefs:["weftext-node://w/b", "weftext-node://w/a"], status: "prepared", plan: { operationId: "once", planId: "plan", status: "prepared" } };
  assert.deepEqual(decodePendingOrder(JSON.stringify(pending), "root", "parent"), pending);
  assert.throws(() => decodePendingOrder(JSON.stringify(pending), "root", "same-name-new-parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, plan: { ...pending.plan, operationId: "another" } }), "root", "parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, request: { ...pending.request, children: ["weftext-node://w/a", "weftext-node://w/a"] } }), "root", "parent"));
});
