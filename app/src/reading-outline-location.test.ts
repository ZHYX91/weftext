import { test } from "node:test";
import assert from "node:assert/strict";
import { headingTree, readingHeadingKeys, type StructureHeading } from "./heading-structure-state";
import { readingHeadingAt, readingHeadingLine, readingHeadingPositions, readingOutlineRow } from "./reading-outline-location";
import type { RenderNode } from "./types";

const heading = (key: string, level = 1, kind: StructureHeading["kind"] = "heading", generated = false): StructureHeading => ({
    key, level, kind, text: "同名章节", generated,
});
const display = (key: string, body = "同名章节", level = 1): RenderNode => ({
    key, kind: "element", tag: "h2", text: null, writable: false, origin: null,
    attrs: {"data-weftext-role": "heading", "data-weftext-level": String(level)},
    children: [{key: key + "-text", kind: "text", tag: null, text: body, children: [], attrs: {}, origin: null, writable: false}],
});
const box = (key: string, top: number, height = 20) => ({key, top, height, width: 100});

test("only exact unique native headings enter the reading index, independent of their text being repeated", () => {
    const items = [heading("chapter-a"), heading("chapter-b", 1, "heading", true), heading("image", 1, "figure"),
        heading("wrong-text"), heading("wrong-level"), heading("dual"), heading("duplicate"), heading("duplicate")];
    const root = {...display("root"), tag: "div", children: [
        display("chapter-a"), display("chapter-b"), display("image"), display("wrong-text", "过期标题"),
        display("wrong-level", "同名章节", 2), display("dual"), display("dual"), display("duplicate"), display("unindexed"),
    ]};
    assert.deepEqual([...readingHeadingKeys(root, items)].sort(), ["chapter-a", "chapter-b"]);
});

test("scroll selection handles before-first, thresholds, repeated coordinates, hidden and invalid boxes", () => {
    const index = readingHeadingPositions(new Set(["a","b","simultaneous","hidden","duplicate","nan"]), [
        box("b", 400), box("simultaneous", 400), box("a", 100), box("caption", 200), box("hidden", 300, 0),
        box("duplicate", 280), box("duplicate", 380), box("nan", NaN),
    ]);
    assert.deepEqual(index, [{key:"a",top:100}, {key:"b",top:400}]);
    for (const [offset,key] of [[0,undefined],[99,undefined],[100,"a"],[399,"a"],[400,"b"],[800,"b"],[NaN,undefined]] as const)
        assert.equal(readingHeadingAt(index, offset), key);
});

test("the final short section can be marked when there is no more room to scroll", () => {
    const index = [{ key: "first", top: 100 }, { key: "last", top: 940 }];
    assert.equal(readingHeadingAt(index, readingHeadingLine(600, 250, 1000)), "first");
    assert.equal(readingHeadingAt(index, readingHeadingLine(750, 250, 1000)), "last");
    assert.equal(readingHeadingAt(index, readingHeadingLine(0, 1000, 1000)), "first");
});

test("when a heading row is folded, signal only containment on an existing ancestor", () => {
    const tree = headingTree([heading("chapter-a"), heading("inside-a", 2), heading("illustration", 2, "figure"), heading("chapter-b")]);
    const ents = new Map(tree.map(row => [row.key, row]));
    assert.deepEqual(readingOutlineRow("inside-a", ents, new Set(["chapter-a","chapter-b"])), {key:"chapter-a",exact:false});
    assert.deepEqual(readingOutlineRow("inside-a", ents, new Set(["inside-a"])), {key:"inside-a",exact:true});
    assert.equal(readingOutlineRow("inside-a", ents, new Set(["chapter-b"])), undefined);
    assert.equal(readingOutlineRow("illustration", ents, new Set(["chapter-a","illustration"])), undefined);
});
