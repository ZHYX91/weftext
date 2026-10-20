import test from "node:test";
import assert from "node:assert/strict";
import { decodeContentsPreference, decodePendingOrder, insertedSibling, movedSibling } from "./node-contents-state";
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
test("order recovery binds original parent, whole request and original plan", () => {
  const pending = { request: { root: "root", parentRef: "parent", baseId: "basis", operationId: "once", children: ["weftext-node://w/a", "weftext-node://w/b"] }, beforeRefs:["weftext-node://w/b", "weftext-node://w/a"], status: "prepared", plan: { operationId: "once", planId: "plan", status: "prepared" } };
  assert.deepEqual(decodePendingOrder(JSON.stringify(pending), "root", "parent"), pending);
  assert.throws(() => decodePendingOrder(JSON.stringify(pending), "root", "same-name-new-parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, plan: { ...pending.plan, operationId: "another" } }), "root", "parent"));
  assert.throws(() => decodePendingOrder(JSON.stringify({ ...pending, request: { ...pending.request, children: ["weftext-node://w/a", "weftext-node://w/a"] } }), "root", "parent"));
});
