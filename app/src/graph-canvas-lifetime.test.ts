import test from "node:test";
import assert from "node:assert/strict";
import { disposeGraphCanvasTree } from "./knowledge-graph-canvas";

test("canvas tree disposal does not depend on a browser-realm Element global", () => {
    let selector = "";
    const root = { querySelectorAll(value: string) { selector = value; return []; } };
    assert.doesNotThrow(() => disposeGraphCanvasTree(root as unknown as ParentNode));
    assert.equal(selector, ".knowledge-graph-canvas");
});
