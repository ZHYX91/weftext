import assert from "node:assert/strict";
import test from "node:test";
import { TagTree, decodeTagShortcutCollection, tagPredicate, type TreeTag } from "./tag-tree";
import { tagCancellationStatus, tagOperationStatus } from "./tag-navigation-state";
const tag = (tagId: string, displayName: string, parentTagId: string | null = null): TreeTag => ({ reference: `weftext-tag://workspace/${tagId}`, tagId, displayName, parentTagId });
test("tag forest intervals distinguish direct tags from subtree membership and reflect a move", () => {
  const a = tag("parent-a", "A"), b = tag("parent-b", "B"), leaf = tag("child", "child", a.tagId), tree = new TagTree([leaf, b, a], true);
  assert.equal(tree.contains(a.tagId, leaf.tagId), true); assert.equal(tree.contains(b.tagId, leaf.tagId), false); assert.equal(tree.contains(a.tagId, a.tagId), true);
  const moved = new TagTree([a, b, { ...leaf, parentTagId: b.tagId }], true); assert.equal(moved.contains(a.tagId, leaf.tagId), false); assert.equal(moved.contains(b.tagId, leaf.tagId), true); assert.equal(moved.byRef.get(leaf.reference)?.reference, leaf.reference);
});
test("longest legal tag chain is iterative, paths are on demand and choices remain bounded", () => {
  const tags = Array.from({ length: 5000 }, (_, i) => tag(`tag-${String(i).padStart(4, "0")}`, `层级${i}`, i ? `tag-${String(i - 1).padStart(4, "0")}` : null));
  const tree = new TagTree(tags, true), last = tags.at(-1)!;
  assert.equal(tree.byId.size, 5000); assert.equal(tree.contains(tags[0].tagId, last.tagId), true); assert.deepEqual(tree.path(last.reference).segments, ["层级4996", "层级4997", "层级4998", "层级4999"]); assert.equal(tree.path(last.reference).truncated, true);
  assert.equal(tree.path(last.reference, 5000).segments.length, 5000); assert.equal(tree.path(last.reference, 5000).truncated, false);
  assert.equal(tree.visible(new Set()).items.length, 1); assert.equal(tree.visible(new Set(tags.map(t => t.tagId))).total, 5000); assert.equal(tree.visible(new Set(tags.map(t => t.tagId))).items.length, 100);
  assert.equal(tree.page("", 4900).items.length, 100); assert.equal(tree.page("", 9999).offset, 4900); assert.equal(tree.page("", 0, tags[0].tagId).total, 0); assert.equal(tree.page("层级4999").items[0].reference, last.reference);
  assert.throws(() => new TagTree([...tags, tag("over", "over")], true), /5,000/);
});
test("same names and display paths retain identity, ID collisions lengthen, slash and Unicode remain segments", () => {
  const a = tag("12345678-a", "A/B"), b = tag("12345678-b", "A/B"), nfc = tag("nfc", "é", a.tagId), nfd = tag("nfd", "e\u0301", a.tagId), tree = new TagTree([a, b, nfc, nfd], true);
  assert.notEqual(tree.label(a.reference), tree.label(b.reference)); assert.ok(tree.path(a.reference).suffix.length > 8); assert.deepEqual(tree.path(nfc.reference).segments, ["A/B", "é"]); assert.deepEqual(tree.path(nfd.reference).segments, ["A/B", "e\u0301"]);
  assert.equal(tree.page("A/B").total, 2); assert.equal(tree.byRef.size, 4);
});
test("partial, cyclic, missing, duplicate and cross-workspace forests never become a complete selector", () => {
  const a = tag("a", "A"), b = tag("b", "B", "a");
  assert.throws(() => new TagTree([a], false), /完整/); assert.throws(() => new TagTree([{ ...a, parentTagId: "a" }], true), /自父/); assert.throws(() => new TagTree([b], true), /缺失/);
  assert.throws(() => new TagTree([{ ...a, parentTagId: "b" }, b], true), /环/); assert.throws(() => new TagTree([a, a], true)); assert.throws(() => new TagTree([a, { ...b, reference: "weftext-tag://other/b" }], true), /跨工作区/);
});
test("mode is embedded in native Query and preserved in a versioned shortcut collection", () => {
  const ref = tag("leaf", "leaf").reference, exact = tagPredicate(ref, "exact", 'field("core/tags")'), subtree = tagPredicate(ref, "subtree", 'field("core/tags")');
  assert.match(exact, /t\.ref == tagRef/); assert.doesNotMatch(exact, /tagInSubtree/); assert.match(subtree, /has\(t.ref\) \? tagInSubtree/); assert.ok(subtree.includes(JSON.stringify(ref)));
  const old = [{ id: "old", kind: "tag", targetRef: ref }], converted = decodeTagShortcutCollection(old); assert.equal(converted.items[0].matchMode, "exact"); assert.equal("matchMode" in old[0], false);
  assert.deepEqual(decodeTagShortcutCollection(JSON.parse(JSON.stringify(converted))), converted); assert.throws(() => decodeTagShortcutCollection({ version: 2, items: old }), /模式/);
  for (const mode of ["exact", "subtree"]) assert.equal(decodeTagShortcutCollection({ version: 2, items: [{ kind: "tag", matchMode: mode }] }).items[0].matchMode, mode);
  assert.throws(() => tagPredicate(ref, undefined as never), /明确/);
});
test("recovery uses the original operation and plan and never treats unknown status as committed", () => {
  const expected = { operationId: "original", planId: "original-plan" };
  assert.equal(tagOperationStatus(expected, { ...expected, status: "prepared" }), "prepared"); assert.equal(tagOperationStatus(expected, { ...expected, status: "committed" }), "committed");
  assert.throws(() => tagOperationStatus(expected, { ...expected, operationId: "new", status: "committed" })); assert.throws(() => tagOperationStatus(expected, { ...expected, planId: "new", status: "committed" })); assert.throws(() => tagOperationStatus(expected, { ...expected, status: "unknown" }));
});
test("cancellation confirms the original durable terminal state without discarding committed or uncertain work", () => {
  const expected = { operationId: "original", planId: "original-plan" };
  assert.equal(tagCancellationStatus(expected, { ...expected, status: "resolved" }), "resolved"); assert.equal(tagCancellationStatus(expected, { ...expected, status: "cancelled" }), "resolved"); assert.equal(tagCancellationStatus(expected, { ...expected, status: "committed" }), "committed");
  for (const status of ["prepared", "recoveryPending", "unknown"]) assert.throws(() => tagCancellationStatus(expected, { ...expected, status }));
  assert.throws(() => tagCancellationStatus(expected, { ...expected, operationId: "replacement", status: "resolved" }));
});
test("a lost or refused preparation cancels by original ID without requiring a replacement preparation", () => {
  const unknown = { operationId: "original", planId: null };
  assert.equal(tagCancellationStatus(unknown, { ...unknown, status: "resolved" }), "resolved", "a definite refusal still receives an original-ID durable fence");
  assert.equal(tagCancellationStatus(unknown, { ...unknown, planId: "already-prepared", status: "resolved" }), "resolved", "a lost prepare response may discover the original server Plan during cancellation");
  assert.equal(tagCancellationStatus(unknown, { ...unknown, planId: "already-committed", status: "committed" }), "committed");
  assert.equal(tagOperationStatus({ operationId: unknown.operationId, planId: "already-committed" }, { operationId: unknown.operationId, planId: "already-committed", status: "committed" }), "committed", "the discovered commit is inspected, not executed again");
  assert.throws(() => tagCancellationStatus(unknown, { ...unknown, status: "committed" }), /尚未确认/);
  assert.throws(() => tagCancellationStatus(unknown, { ...unknown, operationId: "another", status: "resolved" }));
  assert.throws(() => tagCancellationStatus({ operationId: "original", planId: "known-plan" }, { ...unknown, status: "resolved" }), /原导航操作/);
  assert.throws(() => tagCancellationStatus({ operationId: "original", planId: "known-plan" }, { ...unknown, planId: "replacement", status: "resolved" }), /原导航操作/);
});
