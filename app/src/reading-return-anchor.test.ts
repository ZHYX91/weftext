import assert from "node:assert/strict";
import test from "node:test";
import { EditorState, type ChangeSpec } from "@codemirror/state";
import { mapReadingReturnAnchor } from "./reading-return-anchor";

const state = EditorState.create({ doc: "first paragraph\n\n想保留的段落 📚\n\nlast paragraph" });
const text = "想保留的段落 📚";
const from = state.doc.toString().indexOf(text), to = from + text.length;
const anchor = { from, to, text, top: 156.5 };
function edit(changes: ChangeSpec) {
    const transaction = state.update({ changes });
    return mapReadingReturnAnchor(anchor, transaction.changes, transaction.newDoc.toString());
}

test("moves the same UTF-16 run after additions or deletions above the reading position", () => {
    assert.deepEqual(edit({ from: 0, insert: "new paragraphs\n\n" }), { ...anchor, from: from + 16, to: to + 16 });
    assert.deepEqual(edit({ from: 0, to: from }), { ...anchor, from: 0, to: text.length });
});

test("keeps surrounding and exact-boundary edits out of the original run", () => {
    assert.deepEqual(edit([]), anchor);
    assert.deepEqual(edit([{ from, insert: "before" }, { from: to, insert: "after" }]), { ...anchor, from: from + 6, to: to + 6 });
    assert.deepEqual(edit({ from: to, to: state.doc.length, insert: "after" }), anchor);
});

test("never jumps to another copy or follows a run whose interior was edited", () => {
    assert.equal(edit([{ from, to }, { from: state.doc.length, insert: text }]), null);
    assert.equal(edit({ from: from + 1, insert: "x" }), null);
    assert.equal(edit({ from: from + 1, to: from + 2, insert: text[1] }), null);
    assert.equal(edit({ from: to - 1, to }), null);
});

test("a full document replacement retires the anchor even if the new source repeats its text", () => {
    assert.equal(edit({ from: 0, to: state.doc.length, insert: "Unrelated replacement\n\n" + text }), null);
});
