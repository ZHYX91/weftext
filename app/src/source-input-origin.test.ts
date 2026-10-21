import assert from "node:assert/strict";
import test from "node:test";
import { EditorState, Transaction } from "@codemirror/state";
import { history, undo, redo, isolateHistory } from "@codemirror/commands";
import { inputOriginKind, personalTextEffect, sourceInputOrigin, sourceInputOrigins, inputTexts, replayPersonalInput, hasPersonalInput } from "./source-input-origin";

const base = "PRIVATE_BASE😀0123456789";
function editor() {
  let state = EditorState.create({ doc: base, extensions: [history(), sourceInputOrigins] });
  const host = { get state() { return state; }, dispatch(transaction: Transaction) { state = transaction.state; } };
  return { get state() { return state; }, origin: () => state.field(sourceInputOrigin), change(from: number, to: number, insert: string, author = true) { state = state.update({ changes: { from, to, insert }, userEvent: "input.type", annotations: inputOriginKind.of(author ? "author" : "protected") }).state; }, undo: () => undo(host), redo: () => redo(host), isolate() { state = state.update({ annotations: isolateHistory.of("full") }).state; } };
}
test("only actual inserted text is retained, with separate base spans between edits", () => {
  const e = editor(); e.change(0, 0, "OWN_A"); e.change(e.state.doc.length, e.state.doc.length, "OWN_B");
  assert.deepEqual(inputTexts(e.origin()), ["OWN_A", "OWN_B"]);
  assert.equal(replayPersonalInput(e.origin(), base), "OWN_A" + base + "OWN_B");
  assert.ok(!JSON.stringify(e.origin()).includes("PRIVATE_BASE"));
});
test("grouped undo/redo restores provenance through the existing history", () => {
  const e = editor(); e.change(0, 0, "A"); e.change(1, 1, "B");
  assert.equal(e.undo(), true); assert.equal(e.state.doc.toString(), base); assert.deepEqual(inputTexts(e.origin()), []);
  assert.equal(hasPersonalInput(e.origin()), false);
  assert.equal(e.redo(), true); assert.deepEqual(inputTexts(e.origin()), ["AB"]);
  assert.equal(replayPersonalInput(e.origin(), base), e.state.doc.toString());
});
test("undo restoring deleted protected text never labels that text as authored", () => {
  const e = editor(); e.change(0, 12, ""); e.undo();
  assert.deepEqual(inputTexts(e.origin()), []); assert.equal(replayPersonalInput(e.origin(), base), base);
  e.redo(); assert.deepEqual(inputTexts(e.origin()), []); assert.ok(!JSON.stringify(e.origin()).includes("PRIVATE_BASE"));
});
test("a native replacement does not store its old source and preserves displaced author input separately", () => {
  const e = editor(); e.change(2, 2, "OWN"); e.isolate(); e.change(0, e.state.doc.length, "PRIVATE_REWRITTEN_OWN", false);
  assert.deepEqual(inputTexts(e.origin()), ["OWN"]); assert.ok(!JSON.stringify(e.origin()).includes("PRIVATE_REWRITTEN"));
  assert.throws(() => replayPersonalInput(e.origin(), base));
  e.undo(); assert.equal(replayPersonalInput(e.origin(), base), base.slice(0, 2) + "OWN" + base.slice(2));
  e.redo(); assert.deepEqual(inputTexts(e.origin()), ["OWN"]);
});
test("default command replacements cannot classify copied old source as personal text", () => {
  let state = EditorState.create({ doc: base, extensions: [sourceInputOrigins] });
  state = state.update({ changes: { from: 0, insert: base }, userEvent: "input" }).state;
  assert.deepEqual(inputTexts(state.field(sourceInputOrigin)), []); assert.throws(() => replayPersonalInput(state.field(sourceInputOrigin), base));
});
test("same-value ABA and deletion of own text follow actual history rather than value lookup", () => {
  const e = editor(); e.change(0, 0, "OWN"); e.isolate(); e.change(0, 3, ""); e.isolate(); e.change(0, 0, "OWN");
  assert.deepEqual(inputTexts(e.origin()), ["OWN"]); e.undo(); assert.deepEqual(inputTexts(e.origin()), []); e.undo(); assert.deepEqual(inputTexts(e.origin()), ["OWN"]);
});
test("rich pure-input ownership commits with its native patch and follows one CM Undo/Redo", () => {
  let state = EditorState.create({ doc: base, extensions: [history(), sourceInputOrigins] });
  state = state.update({ changes: { from: 0, to: base.length, insert: "PRIVATE_TRANSFORMED_OWN" }, userEvent: "input.type", annotations: inputOriginKind.of("protected"), effects: personalTextEffect.of({ id: "literal", text: "OWN" }) }).state;
  const host = { get state() { return state; }, dispatch(transaction: Transaction) { state = transaction.state; } };
  assert.deepEqual(inputTexts(state.field(sourceInputOrigin)), ["OWN"]); undo(host); assert.deepEqual(inputTexts(state.field(sourceInputOrigin)), []);
  redo(host); assert.deepEqual(inputTexts(state.field(sourceInputOrigin)), ["OWN"]); assert.ok(!JSON.stringify(state.field(sourceInputOrigin)).includes("PRIVATE_TRANSFORMED"));
});
