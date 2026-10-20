import test from "node:test";
import assert from "node:assert/strict";
import { EditorState, type Transaction } from "@codemirror/state";
import { history, redo, undo, undoDepth } from "@codemirror/commands";
import { EditorView } from "@codemirror/view";
import { currentSourceStyleBasis, retireSourceStylesEffect, sourceStyleModeEffect, sourceStyles, sourceTokensEffect, type SourceStyleBasis } from "./source-style";
import type { SyntaxToken } from "./types";

const original = "= 真实中文输入法手工验收\n\n正文输入区：\n";
function state(source = original): EditorState { return EditorState.create({ doc: source, extensions: [sourceStyles, history(), EditorState.lineSeparator.of("\n")] }); }
function heading(source: string, from = source.indexOf("=")): SyntaxToken {
  const to = source.indexOf("\n", from);
  return { kind: "Heading", from, to, utf16From: from, utf16To: to };
}
function parse(editor: EditorState, tokens = [heading(editor.doc.toString())], basis: SourceStyleBasis = currentSourceStyleBasis(editor), styled = true): EditorState {
  return editor.update({ effects: sourceTokensEffect.of({ tokens, styled, basis }) }).state;
}
function decorations(editor: EditorState): { from: number; to: number; classes: string[]; style: unknown }[] {
  const result: { from: number; to: number; classes: string[]; style: unknown }[] = [];
  editor.field(sourceStyles).decorations.between(0, editor.doc.length, (from, to, value) => { result.push({ from, to, classes: String(value.spec.class ?? "").split(" "), style: value.spec.attributes?.["data-source-style"] }); });
  return result;
}
function classes(editor: EditorState): string[] { return decorations(editor).flatMap(item => item.classes); }

test("unrelated body input retains heading geometry but immediately retires every current parser class", () => {
  let editor = parse(state());
  assert.equal(editor.field(sourceStyles).status, "current");
  assert.ok(classes(editor).includes("cm-rendered-heading"));
  const before = editor.doc.toString(), at = before.indexOf("正文输入区：") + "正文输入区：".length;
  for (const character of ["j", "今", "今天"]) {
    editor = editor.update({ changes: { from: at, to: editor.doc.length - 1, insert: character } }).state;
    assert.equal(editor.field(sourceStyles).status, "pending");
    assert.ok(classes(editor).includes("cm-source-heading-layout"));
    assert.ok(classes(editor).includes("cm-source-appearance-heading"));
    assert.ok(classes(editor).every(name => !name.startsWith("cm-typst-") && name !== "cm-rendered" && name !== "cm-rendered-heading"));
    assert.ok(decorations(editor).every(item => item.style === "pending"));
  }
  const attributes = editor.facet(EditorView.contentAttributes).flatMap(value => typeof value === "function" ? [] : [value]);
  assert.equal(attributes[0]["data-source-style-status"], "pending");
  assert.match(String(attributes[0]["aria-description"]), /不代表当前语法结果/);
  assert.equal(editor.doc.toString(), before.slice(0, at) + "今天\n");
});

test("whole unchanged heading lines map through line insertions and retain their exact author bytes", () => {
  let editor = parse(state());
  const prefix = "😀 新的前置行\n";
  editor = editor.update({ changes: { from: 0, insert: prefix } }).state;
  const appearance = editor.field(sourceStyles).appearances[0];
  assert.equal(appearance.from, prefix.length);
  assert.equal(appearance.lineFrom, prefix.length);
  assert.equal(editor.doc.sliceString(appearance.from, appearance.to), original.slice(0, original.indexOf("\n")));
  assert.ok(decorations(editor).some(item => item.from === prefix.length && item.classes.includes("cm-source-heading-layout")));
  editor = editor.update({ changes: { from: 0, to: prefix.length, insert: "" } }).state;
  assert.equal(editor.field(sourceStyles).appearances[0].from, 0);
  assert.equal(editor.doc.toString(), original);
  assert.equal(editor.field(sourceStyles).status, "pending");
});

test("editing or deleting a heading, joining its line, and replacing the full buffer discard its cached geometry", () => {
  const parsed = parse(state());
  const length = original.indexOf("\n");
  for (const change of [
    { from: 0, to: 1, insert: "正文" },
    { from: 3, insert: "新" },
    { from: 0, to: length + 1, insert: "" },
    { from: 0, insert: "前缀" },
    { from: length, insert: "后缀" },
    { from: length, to: length + 2, insert: "" },
    { from: 0, to: original.length, insert: "完全替换的正文" },
    { from: 0, to: original.length, insert: original },
  ]) {
    const changed = parsed.update({ changes: change }).state;
    assert.equal(changed.field(sourceStyles).decorations.size, 0, JSON.stringify(change));
    assert.equal(changed.field(sourceStyles).appearances.length, 0);
  }
});

test("nonlocal syntax changes preserve only explicitly pending appearance until an exact new parser result clears it", () => {
  let editor = parse(state());
  editor = editor.update({ changes: { from: 0, insert: "/* 未闭合注释\n" } }).state;
  assert.ok(classes(editor).includes("cm-source-appearance-heading"));
  assert.ok(!classes(editor).includes("cm-typst-heading"));
  assert.equal(editor.field(sourceStyles).status, "pending");
  editor = parse(editor, []);
  assert.equal(editor.field(sourceStyles).status, "current");
  assert.equal(editor.field(sourceStyles).decorations.size, 0);
});

test("source, generation and retirement epoch independently fence late parser results including equal-byte ABA", () => {
  let editor = parse(state());
  const old = currentSourceStyleBasis(editor);
  const at = original.indexOf("正文");
  editor = editor.update({ changes: { from: at, insert: "新" } }).state;
  editor = editor.update({ changes: { from: at, to: at + 1, insert: "" } }).state;
  assert.equal(editor.doc.toString(), old.source);
  const pending = editor.field(sourceStyles);
  editor = parse(editor, [heading(original)], old);
  assert.equal(editor.field(sourceStyles), pending);
  const now = currentSourceStyleBasis(editor);
  editor = parse(editor, [], { ...now, source: now.source + "错误" });
  assert.equal(editor.field(sourceStyles), pending);
  editor = parse(editor, [], { ...now, generation: now.generation + 1 });
  assert.equal(editor.field(sourceStyles), pending);
  editor = editor.update({ effects: retireSourceStylesEffect.of(null) }).state;
  assert.equal(editor.field(sourceStyles).epoch, now.epoch + 1);
  assert.equal(editor.field(sourceStyles).status, "none");
  assert.equal(editor.field(sourceStyles).decorations.size, 0);
  const retired = editor.field(sourceStyles);
  editor = parse(editor, [heading(original)], now);
  assert.equal(editor.field(sourceStyles), retired);
  editor = parse(editor);
  assert.equal(editor.field(sourceStyles).status, "current");
  assert.ok(classes(editor).includes("cm-typst-heading"));
});

test("retirement dominates a queued old result in the same transaction and removes pending appearance", () => {
  let editor = parse(state());
  const old = currentSourceStyleBasis(editor);
  editor = editor.update({ effects: [retireSourceStylesEffect.of(null), sourceTokensEffect.of({ basis: old, tokens: [heading(original)], styled: true })] }).state;
  assert.equal(editor.field(sourceStyles).status, "none");
  assert.equal(editor.field(sourceStyles).decorations.size, 0);
  editor = parse(editor);
  editor = editor.update({ changes: { from: original.length, insert: "继续" } }).state;
  assert.equal(editor.field(sourceStyles).status, "pending");
  editor = editor.update({ effects: retireSourceStylesEffect.of(null) }).state;
  assert.equal(editor.field(sourceStyles).decorations.size, 0);
  assert.equal(editor.field(sourceStyles).status, "none");
});

test("presentation effects preserve the shared source, selection and Undo/Redo history", () => {
  let editor = parse(state());
  const initial = editor.doc.toString();
  editor = editor.update({ changes: { from: initial.length, insert: "真实输入" }, selection: { anchor: initial.length + 4 }, userEvent: "input.type" }).state;
  const edited = editor.doc.toString(), selection = editor.selection.main.from;
  assert.equal(undoDepth(editor), 1);
  for (let count = 0; count < 3; count++) {
    editor = parse(editor);
    editor = editor.update({ effects: retireSourceStylesEffect.of(null) }).state;
    assert.equal(editor.doc.toString(), edited);
    assert.equal(editor.selection.main.from, selection);
    assert.equal(undoDepth(editor), 1);
  }
  const dispatch = (transaction: Transaction) => { editor = transaction.state; };
  assert.equal(undo({ state: editor, dispatch }), true);
  assert.equal(editor.doc.toString(), initial);
  assert.equal(undoDepth(editor), 0);
  assert.equal(redo({ state: editor, dispatch }), true);
  assert.equal(editor.doc.toString(), edited);
});

test("a display-mode change preserves exact parser eligibility and leaves pending appearance pending", () => {
  let editor = state();
  const empty = editor.field(sourceStyles);
  editor = editor.update({ effects: sourceStyleModeEffect.of(true) }).state;
  assert.equal(editor.field(sourceStyles), empty);
  editor = parse(editor);
  const currentBasis = currentSourceStyleBasis(editor);
  editor = editor.update({ effects: sourceStyleModeEffect.of(false) }).state;
  assert.deepEqual(currentSourceStyleBasis(editor), currentBasis);
  assert.equal(editor.field(sourceStyles).status, "current");
  assert.ok(classes(editor).includes("cm-typst-heading"));
  assert.ok(!classes(editor).includes("cm-source-heading-layout"));
  assert.ok(!classes(editor).includes("cm-source-appearance-rendered"));
  editor = editor.update({ changes: { from: original.length, insert: "正文继续输入" }, selection: { anchor: original.length + 6 }, userEvent: "input.type" }).state;
  const pendingBasis = currentSourceStyleBasis(editor), selection = editor.selection.main.from;
  assert.equal(editor.field(sourceStyles).status, "pending");
  for (const styled of [true, false, true]) {
    editor = editor.update({ effects: sourceStyleModeEffect.of(styled) }).state;
    assert.deepEqual(currentSourceStyleBasis(editor), pendingBasis);
    assert.equal(editor.field(sourceStyles).status, "pending");
    assert.equal(editor.selection.main.from, selection);
    assert.equal(undoDepth(editor), 1);
    assert.equal(classes(editor).includes("cm-source-heading-layout"), styled);
    assert.equal(classes(editor).includes("cm-source-appearance-rendered"), styled);
    assert.ok(!classes(editor).includes("cm-typst-heading"));
  }
});
