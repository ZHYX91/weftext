import assert from "node:assert/strict";
import test from "node:test";
import { planMathSelections } from "./math-selection";
import type { RenderNode, Origin } from "./types";

const source = "$frac(x, x) + 中$";
function origin(from: number, to: number): Origin {
  return { path: "note.typ", from: Buffer.byteLength(source.slice(0, from)), to: Buffer.byteLength(source.slice(0, to)),
    utf16From: from, utf16To: to, raw: source.slice(from, to), syntaxKind: "Math", isMain: true };
}
function node(key: string, tag: string, from: number, to: number, children: RenderNode[] = [], text: string | null = null): RenderNode {
  return { key, tag, kind: "element", text, origin: origin(from, to), children, attrs: {}, writable: false };
}
function fixture() {
  const x1 = node("x1", "mi", 6, 7, [], "x"), x2 = node("x2", "mi", 9, 10, [], "x");
  const fraction = node("f", "mfrac", 1, 11, [x1, x2]);
  return { math: node("m", "math", 0, source.length, [fraction, node("zh", "mi", 14, 15, [], "中")]), x1, x2 };
}

test("math selections use distinct native occurrences for repeated x and keep fraction/whole scopes", () => {
  const { math } = fixture(), plan = planMathSelections(math, math, source);
  assert.deepEqual(plan.candidates.map(c => [c.kind, c.from, c.to]), [["whole", 0, 16], ["subexpression", 1, 11], ["symbol", 6, 7], ["symbol", 9, 10], ["symbol", 14, 15]]);
});
test("generated repeated appearance never attaches same source to all copies", () => {
  const { math, x1 } = fixture(); math.children.push({ ...x1, key: "repeated" });
  assert.equal(planMathSelections(math, math, source).candidates.some(c => c.from === 6 && c.to === 7), false);
});
test("wrappers in one ancestor chain share one exact occurrence", () => {
  const { math, x1 } = fixture(); math.children[0].children[0] = node("wrapper", "msub", 6, 7, [x1]);
  assert.equal(planMathSelections(math, math, source).candidates.filter(c => c.from === 6 && c.to === 7).length, 1);
});
test("one generated expression selects its complete outer object including all mathematical parts", () => {
  const { math } = fixture();
  const brace = node("brace", "mo", 1, 11, [], "{"), matrix = node("matrix", "mtable", 1, 11);
  math.children[0] = node("cases", "mrow", 1, 11, [brace, matrix]);
  const candidates = planMathSelections(math, math, source).candidates;
  assert.equal(candidates.find(c => c.from === 1 && c.to === 11)?.nodeKey, "cases");
  assert.equal(candidates.some(c => c.nodeKey === "brace" || c.nodeKey === "matrix"), false);
});
test("stale source, imported/foreign origins and mismatched byte offsets are unavailable", () => {
  const { math, x1 } = fixture(); x1.origin!.from++;
  assert.equal(planMathSelections(math, math, source).candidates.some(c => c.nodeKey === x1.key), false);
  const { math: foreign } = fixture(); foreign.origin!.isMain = false;
  assert.equal(planMathSelections(foreign, foreign, source).candidates.some(c => c.kind === "whole"), false);
  assert.equal(planMathSelections(math, math, source + "changed").candidates.length > 0, true); // unchanged ranges still exact; caller checks session/current.
  assert.equal(planMathSelections(math, math, source.replace("frac", "sqrt")).candidates.some(c => c.kind === "whole"), false);
});
test("an absent current equation and a glyph with no origin never use text search", () => {
  const { math, x1 } = fixture(); x1.origin = null;
  assert.equal(planMathSelections(math, math, source).candidates.some(c => c.nodeKey === "x1"), false);
  const { math: other } = fixture();
  assert.equal(planMathSelections(math, other, source).candidates.length, 0);
});
test("mathematical fine selection never splits a combining grapheme or surrogate pair", () => {
  const value = "$é + 𝑥$", encode = (s: string) => Buffer.byteLength(s);
  const make = (key: string, from: number, to: number) => ({ ...node(key, "mi", 0, 1), origin: {
    ...origin(0, 1), from: encode(value.slice(0, from)), to: encode(value.slice(0, to)),
    utf16From: from, utf16To: to, raw: value.slice(from, to),
  } });
  const splitMark = make("split-mark", 1, 2), fullMark = make("full-mark", 1, 3), splitSurrogate = make("split-surrogate", 6, 7), fullSurrogate = make("full-surrogate", 6, 8);
  const math: RenderNode = { ...node("m", "math", 0, 1), origin: null, children: [splitMark, fullMark, splitSurrogate, fullSurrogate] };
  assert.deepEqual(planMathSelections(math, math, value).candidates.map(c => c.nodeKey), ["full-mark", "full-surrogate"]);
});
