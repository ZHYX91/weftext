import assert from "node:assert/strict";
import test from "node:test";
import { DocumentNotesInputTracker } from "./personal-input-text";
test("native textarea history never restores ownership by equal text after ABA", () => {
  const tracker = new DocumentNotesInputTracker("PROTECTED");
  tracker.input("OWN", { from: 0, to: 9, inputType: "insertText" });
  tracker.input("PROTECTED", { from: 0, to: 3, inputType: "insertText" });
  tracker.input("OWN", { from: 0, to: 9, inputType: "historyUndo" });
  tracker.input("PROTECTED", { from: 0, to: 3, inputType: "historyUndo" });
  assert.equal(tracker.newInput(), "");
});
test("disjoint edits retain only author text and ambiguous history keeps that text separately", () => {
  const tracker = new DocumentNotesInputTracker("PRIVATE-MIDDLE-PRIVATE");
  tracker.input("A" + "PRIVATE-MIDDLE-PRIVATE", { from: 0, to: 0, inputType: "insertText" });
  tracker.input("APRIVATE-MIDDLE-PRIVATEB", { from: 23, to: 23, inputType: "insertText" });
  assert.equal(tracker.newInput(), "A\nB");
  tracker.input("PRIVATE-MIDDLE-PRIVATEB", { from: 0, to: 1, inputType: "historyUndo" });
  assert.equal(tracker.newInput(), ""); assert.equal(tracker.recoveryInput(), "A\nB");
  assert.ok(!tracker.recoveryInput().includes("PRIVATE"));
});
test("out-of-range edit evidence cannot label programmatically inserted protected text as author input", () => {
  const tracker = new DocumentNotesInputTracker(""); tracker.input("A", { from: 0, to: 0, inputType: "insertText" });
  tracker.input("ASECRETX", { from: 7, to: 7, inputType: "insertText" });
  assert.equal(tracker.newInput(), ""); assert.equal(tracker.recoveryInput(), "A"); assert.ok(!tracker.recoveryInput().includes("SECRET"));
});
test("composition payload proves the exact replacement and never adopts a programmatic middle", () => {
  const normal = new DocumentNotesInputTracker("AB"); normal.observe("A中文B", { from: 1, to: 1, inputType: "insertCompositionText" }, "中文"); assert.equal(normal.newInput(), "中文");
  const reset = new DocumentNotesInputTracker("A"); reset.observe("ABSECRETX", { from: 1, to: 1, inputType: "insertCompositionText" }, "X"); assert.equal(reset.newInput(), ""); assert.equal(reset.recoveryInput(), "X");
  const first = new DocumentNotesInputTracker("BASEX"); first.observe("BASEX", undefined, "X"); assert.equal(first.recoveryInput(), "X");
  const equal = new DocumentNotesInputTracker("PRIVATE"); equal.observe("PRIVATE", { from: 0, to: 3, inputType: "insertText" }, "PRI"); assert.equal(equal.newInput(), "PRI");
});
