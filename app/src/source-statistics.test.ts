import assert from "node:assert/strict";
import test from "node:test";
import { EditorState } from "@codemirror/state";
import { countSourceCodePoints, LONG_SOURCE_LINE, sourceCodePointCount, sourceLongLines } from "./source-statistics";

const state = (source: string) => EditorState.create({ doc: source, extensions: [EditorState.lineSeparator.of("\n"), sourceCodePointCount, sourceLongLines] });
function verify(value: EditorState) {
  const source = value.doc.toString();
  assert.equal(value.field(sourceCodePointCount), [...source].length);
  assert.equal(countSourceCodePoints(source), [...source].length);
  const expected: number[] = []; let from = 0;
  for (const line of source.split("\n")) { if (line.length > LONG_SOURCE_LINE) expected.push(from); from += line.length + 1; }
  assert.deepEqual(value.field(sourceLongLines), expected);
}

test("source statistics count Unicode code points without replacing unmatched surrogates", () => {
  for (const source of ["", "中文😀e\u0301\r\n", "\ud800", "\udc00", "\ud800\ud800\udc00", "\ud800\n\udc00", "x\ud800y\udc00z"]) verify(state(source));
});

test("splices join and split surrogate pairs at either boundary and across adjacent edits", () => {
  const examples = [
    { source: "\ud800x\udc00", changes: [{ from: 1, to: 2, insert: "" }] },
    { source: "\ud800xy\udc00", changes: [{ from: 1, to: 2, insert: "" }, { from: 2, to: 3, insert: "" }] },
    { source: "a\udc00", changes: [{ from: 1, insert: "\ud800" }] },
    { source: "\ud800a", changes: [{ from: 1, insert: "\udc00" }] },
    { source: "😀😀", changes: [{ from: 1, to: 3, insert: "中" }] },
    { source: "😀x😀", changes: [{ from: 0, to: 1, insert: "" }, { from: 4, to: 5, insert: "" }] },
  ];
  for (const example of examples) { const before = state(example.source); verify(before.update({ changes: example.changes }).state); verify(before); }
});

test("incremental counts match independent string iteration across many sparse edits", () => {
  let seed = 7943;
  const random = (limit: number) => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % limit; };
  const units = ["a", "中", "😀", "\ud800", "\udc00", "\n", "\r", "e\u0301", ""];
  let current = state("\ud800\n\udc00中文😀".repeat(50));
  for (let iteration = 0; iteration < 1600; iteration++) {
    const previous = current, length = previous.doc.length, split = Math.floor(length / 2);
    const firstFrom = random(split + 1), firstTo = firstFrom + random(split - firstFrom + 1);
    const secondFrom = split + random(length - split + 1), secondTo = secondFrom + random(length - secondFrom + 1);
    const changes = [{ from: firstFrom, to: firstTo, insert: units[random(units.length)] }, { from: secondFrom, to: secondTo, insert: units[random(units.length)] }];
    current = previous.update({ changes }).state; verify(current); verify(previous);
  }
});

test("long-line tracking handles splits, merges, replacement and removal without changing source", () => {
  const large = "A".repeat(LONG_SOURCE_LINE + 10), original = large + "\n短行\n" + large;
  let current = state(original); verify(current);
  current = current.update({ changes: { from: LONG_SOURCE_LINE + 1, insert: "\n" } }).state; verify(current);
  current = current.update({ changes: { from: LONG_SOURCE_LINE + 1, to: LONG_SOURCE_LINE + 2, insert: "" } }).state; verify(current);
  current = current.update({ changes: { from: 0, to: current.doc.length, insert: "短行\n" } }).state; verify(current);
  assert.deepEqual(current.field(sourceLongLines), []);
  current = current.update({ changes: [{ from: 0, insert: large + "\n" }, { from: current.doc.length, insert: large }] }).state; verify(current);
  current = current.update({ changes: { from: large.length, to: large.length + 1, insert: "" } }).state; verify(current);
});

test("sparse edits retain exactly the long lines found by an independent complete scan", () => {
  let seed = 4179;
  const random = (limit: number) => { seed = (Math.imul(seed, 1103515245) + 12345) >>> 0; return seed % limit; };
  const original = "前\n" + "A".repeat(LONG_SOURCE_LINE + 1) + "\n" + "中".repeat(LONG_SOURCE_LINE - 1) + "\n尾";
  const inserts = ["", "\n", "😀\n", "\ud800", "x", "\udc00"];
  for (let iteration = 0; iteration < 80; iteration++) {
    let current = state(original);
    for (let edit = 0; edit < 5; edit++) {
      const length = current.doc.length, split = Math.floor(length / 2), from = random(split + 1), next = split + random(length - split + 1);
      current = current.update({ changes: [{ from, to: Math.min(split, from + random(12)), insert: inserts[random(inserts.length)] }, { from: next, to: Math.min(length, next + random(12)), insert: inserts[random(inserts.length)] }] }).state;
      verify(current);
    }
  }
});
