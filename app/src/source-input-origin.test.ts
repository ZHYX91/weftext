import assert from "node:assert/strict";
import test from "node:test";
import { EditorState, Transaction, type TransactionSpec } from "@codemirror/state";
import { history, undo, redo, isolateHistory, undoDepth } from "@codemirror/commands";
import { inputOriginKind, personalTextEffect, sourceInputOrigin, sourceInputOrigins, personalInputProjection, sourceInputCheckpoints, initializeInputHash, inputCheckpoint, beginInputCheckpoint, settleInputCheckpoint, restoredInputOrigin, restoreInputOrigin, inputTexts, replayPersonalInput, hasPersonalInput } from "./source-input-origin";

const base = "PRIVATE_BASE😀0123456789";
function editor() {
  let state = EditorState.create({ doc: base, extensions: [history(), sourceInputOrigins] });
  const host = { get state() { return state; }, dispatch(transaction: Transaction) { state = transaction.state; } };
  return { get state() { return state; }, origin: () => personalInputProjection(state).origin, projection: () => personalInputProjection(state), checkpoint: () => state.field(sourceInputCheckpoints), command(spec: TransactionSpec) { state = state.update(spec).state; }, change(from: number, to: number, insert: string, author = true) { state = state.update({ changes: { from, to, insert }, userEvent: "input.type", annotations: inputOriginKind.of(author ? "author" : "protected") }).state; }, undo: () => undo(host), redo: () => redo(host), isolate() { state = state.update({ annotations: isolateHistory.of("full") }).state; } };
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
  assert.deepEqual(inputTexts(personalInputProjection(state).origin), []); assert.throws(() => replayPersonalInput(personalInputProjection(state).origin, base));
});
test("same-value ABA and deletion of own text follow actual history rather than value lookup", () => {
  const e = editor(); e.change(0, 0, "OWN"); e.isolate(); e.change(0, 3, ""); e.isolate(); e.change(0, 0, "OWN");
  assert.deepEqual(inputTexts(e.origin()), ["OWN"]); e.undo(); assert.deepEqual(inputTexts(e.origin()), []); e.undo(); assert.deepEqual(inputTexts(e.origin()), ["OWN"]);
});
test("rich pure-input ownership commits with its native patch and follows one CM Undo/Redo", () => {
  let state = EditorState.create({ doc: base, extensions: [history(), sourceInputOrigins] });
  state = state.update({ changes: { from: 0, to: base.length, insert: "PRIVATE_TRANSFORMED_OWN" }, userEvent: "input.type", annotations: inputOriginKind.of("protected"), effects: personalTextEffect.of({ id: "literal", revision: 1, text: "OWN" }) }).state;
  const host = { get state() { return state; }, dispatch(transaction: Transaction) { state = transaction.state; } };
  assert.deepEqual(inputTexts(personalInputProjection(state).origin), ["OWN"]); undo(host); assert.deepEqual(inputTexts(personalInputProjection(state).origin), []);
  redo(host); assert.deepEqual(inputTexts(personalInputProjection(state).origin), ["OWN"]); assert.ok(!JSON.stringify(state.field(sourceInputOrigin)).includes("PRIVATE_TRANSFORMED"));
});

const digest = (value: string) => import("node:crypto").then(({ createHash }) => createHash("sha256").update(value).digest("hex"));
async function initialize(e: ReturnType<typeof editor>) { e.command(initializeInputHash(e.checkpoint().confirmed.id, await digest(base))); }
async function begin(e: ReturnType<typeof editor>, id: string) { e.command(beginInputCheckpoint(id, inputCheckpoint(e.state, await digest(e.state.doc.toString())))); }
test("confirmed saved S advances recovery without adding an Undo item or resetting original author atoms", async () => {
  const e = editor(); await initialize(e); e.change(base.length, base.length, "X"); const saved = e.state.doc.toString(), atoms = e.state.field(sourceInputOrigin); const depth = undoDepth(e.state);
  await begin(e, "original"); e.command(settleInputCheckpoint("original", true)); assert.equal(undoDepth(e.state), depth); assert.equal(e.state.field(sourceInputOrigin), atoms);
  e.change(saved.length, saved.length, "Y"); assert.equal(e.projection().baseHash, await digest(saved)); assert.equal(replayPersonalInput(e.origin(), saved), saved + "Y"); assert.deepEqual(inputTexts(e.origin()), ["Y"]);
});
test("held original Save freezes atoms before later Y and preserves its original operation ID", async () => {
  const e = editor(); await initialize(e); e.change(base.length, base.length, "X"); const saved = e.state.doc.toString(); await begin(e, "held"); e.change(saved.length, saved.length, "Y");
  assert.equal(e.projection().candidate?.operationId, "held"); assert.equal(replayPersonalInput(e.projection().candidate!.origin, saved), saved + "Y"); e.command(settleInputCheckpoint("held", true)); assert.equal(replayPersonalInput(e.origin(), saved), saved + "Y");
});
test("two Saves and Undo retain original own atom identity while confirmed hash never follows history", async () => {
  const e = editor(); await initialize(e); e.change(1, 1, "X"); const atom = e.state.field(sourceInputOrigin).pieces[1].id; await begin(e, "one"); e.command(settleInputCheckpoint("one", true)); e.isolate(); e.change(1, 2, ""); await begin(e, "two"); e.command(settleInputCheckpoint("two", true));
  e.undo(); assert.equal(e.state.field(sourceInputOrigin).pieces[1].id, atom); assert.equal(e.projection().baseHash, await digest(base)); assert.equal(replayPersonalInput(e.origin(), base), base.slice(0, 1) + "X" + base.slice(1)); e.redo(); assert.equal(e.projection().baseHash, await digest(base));
});
test("equal bytes after ABA cannot replace a frozen original descriptor", async () => {
  const e = editor(); await initialize(e); e.change(1, 1, "X"); await begin(e, "frozen"); const submitted = e.checkpoint().pending!.checkpoint; e.undo(); e.isolate(); e.change(1, 1, "X"); await begin(e, "frozen"); assert.equal(e.checkpoint().pending!.checkpoint, submitted); assert.ok(e.projection().candidate!.origin.pieces.some(piece => "text" in piece && piece.text === "X"));
});
test("native detached X included in exact S becomes copy-only; newer X2 revision stays active", async () => {
  const e = editor(); await initialize(e); e.command({ changes: { from: 0, to: base.length, insert: "NATIVE" }, annotations: inputOriginKind.of("protected"), effects: personalTextEffect.of({ id: "literal", revision: 1, text: "X" }) }); await begin(e, "native");
  e.command({ effects: personalTextEffect.of({ id: "literal", revision: 2, text: "X2" }) }); e.command(settleInputCheckpoint("native", true)); assert.deepEqual(e.origin().detached.map(item => item.text), ["X2"]); assert.throws(() => replayPersonalInput(e.origin(), "NATIVE"));
  const f = editor(); f.command({ changes: { from: 0, to: base.length, insert: "NATIVE" }, annotations: inputOriginKind.of("protected"), effects: personalTextEffect.of({ id: "literal", revision: 1, text: "X" }) }); await begin(f, "native2"); f.command(settleInputCheckpoint("native2", true)); f.change(6, 6, "Y"); assert.deepEqual(f.origin().copyOnly?.map(item => item.text), ["X"]); assert.equal(replayPersonalInput(f.origin(), "NATIVE"), "NATIVEY");
});
test("Undo restoring protected text absent from saved S stays opaque and never persists that text", async () => {
  const e = editor(); await initialize(e); e.change(0, base.length, ""); await begin(e, "deleted"); e.command(settleInputCheckpoint("deleted", true)); e.undo(); assert.deepEqual(e.origin().pieces, [{ opaque: base.length }]); assert.throws(() => replayPersonalInput(e.origin(), "")); assert.ok(!JSON.stringify(e.projection()).includes("PRIVATE"));
});
test("cancelled candidate and late initial hash cannot replace the current confirmed checkpoint", async () => {
  const e = editor(); const first = e.checkpoint().confirmed.id; await initialize(e); e.change(0, 0, "X"); await begin(e, "cancelled"); e.command(settleInputCheckpoint("cancelled", false)); assert.equal(e.projection().baseHash, await digest(base));
  await begin(e, "new"); e.command(settleInputCheckpoint("new", true)); e.command(initializeInputHash(first, "0".repeat(64))); assert.equal(e.projection().baseHash, await digest("X" + base)); assert.throws(() => e.command(settleInputCheckpoint("cancelled", true)), /回执/);
});
test("invalid recovered Base ranges fail closed rather than slicing protected text", () => {
  for (const piece of [{ base: -1, length: 2 }, { base: 0, length: -1 }, { base: 1.5, length: 2 }, { base: base.length, length: 1 }]) assert.throws(() => replayPersonalInput({ baseLength: base.length, pieces: [piece], detached: [], revision: 0 }, base));
});

test("restoring confirmed rich copy-only history keeps later Y replayable on the same verified Base", () => {
  let state = EditorState.create({ doc: "NATIVE-X", extensions: [history(), sourceInputOrigins] });
  const record = { baseLength: 8, pieces: [{ base: 0, length: 8 }, { text: "Y" }], detached: [], copyOnly: [{ id: "rich", revision: 1, text: "X" }], revision: 2 };
  const restored = restoredInputOrigin(record, state.field(sourceInputOrigin)); state = state.update({ changes: { from: 0, to: 8, insert: replayPersonalInput(record, "NATIVE-X") }, annotations: inputOriginKind.of("protected"), effects: restoreInputOrigin.of(restored) }).state;
  const projected = personalInputProjection(state).origin; assert.deepEqual(projected.detached, []); assert.deepEqual(projected.copyOnly, record.copyOnly); assert.equal(replayPersonalInput(projected, "NATIVE-X"), "NATIVE-XY");
});
