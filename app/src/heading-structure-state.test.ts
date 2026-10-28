import test from "node:test";
import assert from "node:assert/strict";
import { DOCUMENT_ROOT, headingTree, visibleHeadings, headingMatchesDisplay, type StructureHeading } from "./heading-structure-state";
import type { RenderNode } from "./types";
const heading = (key: string, text: string, level: number): StructureHeading => ({ key, kind: "heading", text, level, generated: true });
const tree = headingTree([heading("a", "文档根", 1), heading("b", "深层", 4), heading("c", "目标", 5), heading("d", "目标", 2), heading("e", "后记", 1)]);
test("document root is independent and real hierarchy survives skipped levels", () => { assert.equal(tree[0].key, DOCUMENT_ROOT); assert.notEqual(tree[1].key, DOCUMENT_ROOT); assert.equal(tree[2].parent, "a"); assert.equal(tree[3].parent, "b"); assert.equal(tree[4].parent, "a"); assert.equal(tree[1].descendants, 3); assert.equal(tree[1].children.length, 2); assert.equal(tree[0].descendants, 5); });
test("search reveals matching ancestor paths and clearing restores collapsed state", () => { const collapsed = new Set(["a"]), before = [...collapsed]; assert.deepEqual(visibleHeadings(tree, DOCUMENT_ROOT, collapsed, "", 0).items.map(n => n.key), [DOCUMENT_ROOT, "a", "e"]); const search = visibleHeadings(tree, DOCUMENT_ROOT, collapsed, "目标", 0); assert.deepEqual(search.matches, ["c", "d"]); assert.deepEqual(search.items.map(n => n.key), [DOCUMENT_ROOT, "a", "b", "c", "d"]); assert.deepEqual([...collapsed], before); assert.equal(tree[1].descendants, 3); assert.equal(visibleHeadings(tree, DOCUMENT_ROOT, collapsed, "", 0).items.length, 3); });
test("subtree and depth are separate and counts never reflect search filtering", () => { const view = visibleHeadings(tree, "b", new Set(), "", 1); assert.deepEqual(view.items.map(n => n.key), ["b", "c"]); assert.equal(view.items[0].descendants, 1); assert.deepEqual(visibleHeadings(tree, "a", new Set(), "", 1).items.map(n => n.key), ["a", "b", "d"]); });
test("generated display navigation rejects reused keys from a different native compile", () => { const text: RenderNode = { key: "t", kind: "text", tag: null, text: "生成标题", attrs: {}, children: [], origin: null, writable: false }, product: RenderNode = { key: "h", kind: "element", tag: "h3", text: null, attrs: { "data-weftext-role": "heading", "data-weftext-level": "7" }, children: [text], origin: null, writable: false }, target = heading("h", "生成标题", 7); assert.equal(headingMatchesDisplay(product, target), true); assert.equal(headingMatchesDisplay(product, { ...target, text: "另一旧标题" }), false); assert.equal(headingMatchesDisplay(product, { ...target, level: 6 }), false); assert.equal(headingMatchesDisplay({ ...product, attrs: {} }, target), false); assert.equal(headingMatchesDisplay({ ...product, children: [product] }, target), false); });

test("captions belong to the last actual heading without becoming heading ancestors", () => {
    const caption: StructureHeading = { key: "figure", kind: "figure", nativeKind: "figure", text: "资源", number: "1", level: 0, generated: false, range: { utf16From: 11, utf16To: 18 } };
    const items = headingTree([heading("a", "文档", 1), caption, heading("b", "深层", 10), { ...caption, key: "table", kind: "table", text: "数据" }, heading("c", "后记", 2)]);
    assert.equal(items.find(item => item.key === "figure")!.parent, "a");
    assert.equal(items.find(item => item.key === "b")!.parent, "a");
    assert.equal(items.find(item => item.key === "table")!.parent, "b");
    assert.equal(items.find(item => item.key === "c")!.parent, "a");
    assert.deepEqual(visibleHeadings(items, DOCUMENT_ROOT, new Set(), "数据", 0).items.map(item => item.key), [DOCUMENT_ROOT, "a", "b", "table"]);
});

test("caption display navigation verifies native kind and body as well as unique render identity", () => {
    const text: RenderNode = { key: "t", kind: "text", tag: null, text: "数据", attrs: {}, children: [], origin: null, writable: false };
    const body: RenderNode = { ...text, key: "body", kind: "element", text: null, attrs: { "data-weftext-role": "caption-body" }, children: [text] };
    const product: RenderNode = { ...body, key: "caption", attrs: { "data-weftext-role": "caption", "data-weftext-caption-kind": "table" }, children: [body] };
    const target: StructureHeading = { key: "caption", kind: "table", nativeKind: "table", level: 0, text: "数据", generated: true };
    assert.equal(headingMatchesDisplay(product, target), true);
    assert.equal(headingMatchesDisplay(product, { ...target, nativeKind: "figure" }), false);
    assert.equal(headingMatchesDisplay(product, { ...target, text: "旧数据" }), false);
    assert.equal(headingMatchesDisplay({ ...product, kind: "tag" }, target), false);
    assert.equal(headingMatchesDisplay({ ...product, children: [body, body] }, target), false);
    assert.equal(headingMatchesDisplay({ ...product, children: [body, product] }, target), false);
});

test("numbered equation display navigation excludes inline and unnumbered math", () => {
    const product: RenderNode = { key: "eq", kind: "element", tag: null, text: null, attrs: { "data-weftext-role": "equation", "data-weftext-numbered": "true" }, children: [{ key: "t", kind: "text", tag: null, text: "x", attrs: {}, children: [], origin: null, writable: false }], origin: null, writable: false };
    const target: StructureHeading = { key: "eq", kind: "equation", nativeKind: "equation", level: 0, text: "x", generated: true };
    assert.equal(headingMatchesDisplay(product, target), true);
    assert.equal(headingMatchesDisplay({ ...product, attrs: { ...product.attrs, "data-weftext-numbered": "false" } }, target), false);
});
