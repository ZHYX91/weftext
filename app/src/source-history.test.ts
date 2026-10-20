import test from "node:test";
import assert from "node:assert/strict";
import { Annotation, EditorState, Transaction } from "@codemirror/state";
import { history, isolateHistory, redo, undo, undoDepth } from "@codemirror/commands";
import { sourceCompositionHistory, SourceHistoryRequests, type SourceHistoryCommand } from "./source-history";

function harness() {
  let rawComposing = false, finalDomFlushed = false;
  const tasks: { run: () => void; cancelled: boolean }[] = [], events: string[] = [], delays: number[] = [];
  const waiting: (() => void)[] = [];
  let execute = (command: SourceHistoryCommand) => { events.push(command); return true; };
  const history = new SourceHistoryRequests({
    composing: () => rawComposing,
    execute: command => execute(command),
    settled: () => { events.push("settled"); for (const action of waiting.splice(0)) action(); },
    schedule: (run, delay) => { const task = { run, cancelled: false }; tasks.push(task); delays.push(delay); return () => { task.cancelled = true; }; },
  });
  return { history, events, delays,
    begin() { rawComposing = true; history.compositionStart(); },
    end(rawRemains = false) { rawComposing = rawRemains; history.compositionEnd(); },
    raw(value: boolean) { rawComposing = value; },
    flushFinalDom() { finalDomFlushed = true; events.push("final-dom-flushed"); },
    task() { const task = tasks.shift(); assert.ok(task); if (!task.cancelled) task.run(); },
    taskCount: () => tasks.length,
    execute(run: typeof execute) { execute = run; },
    assertFlushed() { assert.equal(finalDomFlushed, true); },
    deferOutside(action: () => void) { if (history.composing) waiting.push(action); else action(); },
  };
}

test("ordinary host history remains synchronous and returns the original command result", () => {
  const h = harness();
  assert.equal(h.history.request("undo"), true);
  assert.deepEqual(h.events, ["undo"]);
  h.execute(() => false);
  assert.equal(h.history.request("redo"), false);
  assert.equal(h.taskCount(), 0);
});

test("final DOM settles before ordered Undo/Redo, then mode/save actions are released", () => {
  const h = harness(); h.begin();
  h.execute(command => { h.assertFlushed(); assert.equal(h.history.composing, true); h.events.push(command); return true; });
  for (const command of ["undo", "undo", "redo"] as const) assert.equal(h.history.request(command), true);
  h.deferOutside(() => h.events.push("mode-switch"));
  h.end();
  // compositionend's first microtask must still report composing. Otherwise
  // a mode switch can retire the queue before its document Undo runs.
  assert.equal(h.history.composing, true);
  h.deferOutside(() => h.events.push("save"));
  assert.deepEqual(h.events, []);
  h.flushFinalDom(); h.task();
  assert.deepEqual(h.events, ["final-dom-flushed", "undo", "undo", "redo", "settled", "mode-switch", "save"]);
  assert.equal(h.history.composing, false);
});

test("composition cancellation still waits for host settlement before applying history", () => {
  const h = harness(); h.begin(); h.history.request("undo"); h.end();
  assert.deepEqual(h.events, []);
  h.task();
  assert.deepEqual(h.events, ["undo", "settled"]);
});

test("late host composition flags are retried with backoff, without a second end or early outside release", () => {
  const h = harness(); h.begin(); h.history.request("undo"); h.history.request("redo");
  h.deferOutside(() => h.events.push("save")); h.end(true);
  h.task(); h.task();
  assert.deepEqual(h.delays, [0, 16, 32]);
  assert.deepEqual(h.events, []); assert.equal(h.history.composing, true);
  h.raw(false); h.flushFinalDom(); h.task();
  assert.deepEqual(h.events, ["final-dom-flushed", "undo", "redo", "settled", "save"]);
  assert.equal(h.history.composing, false); assert.equal(h.taskCount(), 0);
});

test("a delayed host retry is cancelled by a new composition or disposal and cannot revive retired requests", () => {
  const h = harness(); h.begin(); h.history.request("undo"); h.end(true); h.task();
  h.begin(); h.task(); assert.deepEqual(h.events, []);
  h.history.request("redo"); h.history.retire(); h.end(true); h.task();
  h.raw(false); h.task(); assert.deepEqual(h.events, ["settled"]);
  h.begin(); h.history.request("undo"); h.end(true); h.task();
  h.history.dispose(); h.raw(false); h.task();
  assert.deepEqual(h.events, ["settled"]); assert.equal(h.taskCount(), 0);
});

test("an explicit composing input-event hint cannot run history before its end", () => {
  const h = harness();
  h.history.request("undo", true);
  assert.equal(h.history.composing, true);
  assert.deepEqual(h.events, []);
  h.history.compositionEnd(); h.task();
  assert.deepEqual(h.events, ["undo", "settled"]);
});

test("source/session/permission retirement drops old requests but releases settled outside actions", () => {
  const h = harness(); h.begin(); h.history.request("undo");
  h.deferOutside(() => h.events.push("outside")); h.end();
  h.history.retire();
  assert.equal(h.history.composing, true);
  h.task();
  assert.deepEqual(h.events, ["settled", "outside"]);
  assert.equal(h.history.request("redo"), true);
  assert.deepEqual(h.events, ["settled", "outside", "redo"]);
});

test("retirement while draining prevents later old commands without clearing host history", () => {
  const h = harness(); h.begin();
  h.history.request("undo"); h.history.request("redo");
  h.execute(command => { h.events.push(command); h.history.retire(); return true; });
  h.end(); h.task();
  assert.deepEqual(h.events, ["undo", "settled"]);
});

test("a new composition before the settlement task retains requests until the new final DOM", () => {
  const h = harness(); h.begin(); h.history.request("undo"); h.end();
  h.begin(); h.task();
  assert.deepEqual(h.events, []);
  h.history.request("redo"); h.end(); h.flushFinalDom(); h.task();
  assert.deepEqual(h.events, ["final-dom-flushed", "undo", "redo", "settled"]);
});

test("disposal cancels pending history, its task and future commands", () => {
  const h = harness(); h.begin(); h.history.request("undo"); h.end();
  h.history.dispose(); h.task();
  assert.deepEqual(h.events, []);
  assert.equal(h.history.request("redo"), false);
  h.history.compositionEnd();
  assert.equal(h.taskCount(), 0);
});

test("the first late native composition input does not absorb an earlier ordinary edit or composition", () => {
  let epoch: number | null = null;
  let state = EditorState.create({ doc: "正文：", extensions: [history(), sourceCompositionHistory(() => epoch, () => false)] });
  state = state.update({ changes: { from: state.doc.length, insert: "之前" }, userEvent: "input.type", annotations: isolateHistory.of("full") }).state;
  epoch = 1;
  state = state.update({ changes: { from: state.doc.length, insert: "中" }, userEvent: "input.type.compose" }).state;
  state = state.update({ changes: { from: state.doc.length, insert: "文" }, userEvent: "input.type.compose" }).state;
  assert.equal(undoDepth(state), 2);
  const dispatch = (transaction: Transaction) => { state = transaction.state; };
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：之前");
  assert.equal(redo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：之前中文");
  epoch = 2;
  state = state.update({ changes: { from: state.doc.length, insert: "下次" }, userEvent: "input.type.compose" }).state;
  assert.equal(undoDepth(state), 3);
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：之前中文");
});

test("cross-line selection deletion, intermediate preedit and late final commit remain one composition Undo", () => {
  let epoch: number | null = null;
  const original = "头\n旧段落\n尾";
  let state = EditorState.create({ doc: original, extensions: [history(), sourceCompositionHistory(() => epoch, () => false)] });
  epoch = 1;
  state = state.update({ changes: { from: 2, to: 6, insert: "" }, userEvent: "input", annotations: Transaction.time.of(1) }).state;
  // CM may still emit compose.start after its own initial selection deletion.
  // The composition remains one unit even after the ordinary 500ms join gap.
  state = state.update({ changes: { from: 2, insert: "zhong" }, userEvent: "input.type.compose.start", annotations: Transaction.time.of(3000) }).state;
  state = state.update({ changes: { from: 2, to: 7, insert: "中文\n" }, userEvent: "input.type.compose", annotations: Transaction.time.of(6000) }).state;
  assert.equal(undoDepth(state), 1);
  const dispatch = (transaction: Transaction) => { state = transaction.state; };
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), original);
  assert.equal(redo({ state, dispatch }), true); assert.equal(state.doc.toString(), "头\n中文\n尾");
});

test("a settled composition is isolated from adjacent ordinary typing on both sides", () => {
  let epoch: number | null = null;
  let state = EditorState.create({ doc: "正文：", extensions: [history(), sourceCompositionHistory(() => epoch, () => false)] });
  state = state.update({ changes: { from: state.doc.length, insert: "先" }, userEvent: "input.type", annotations: Transaction.time.of(10000) }).state;
  epoch = 1;
  state = state.update({ changes: { from: state.doc.length, insert: "中" }, userEvent: "input.type.compose", annotations: Transaction.time.of(10001) }).state;
  state = state.update({ changes: { from: state.doc.length, insert: "文" }, userEvent: "input.type.compose", annotations: Transaction.time.of(10002) }).state;
  epoch = null;
  state = state.update({ annotations: isolateHistory.of("after") }).state;
  state = state.update({ changes: { from: state.doc.length, insert: "后" }, userEvent: "input.type", annotations: Transaction.time.of(10003) }).state;
  assert.equal(undoDepth(state), 3);
  const dispatch = (transaction: Transaction) => { state = transaction.state; };
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：先中文");
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：先");
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文：");
});

test("cancel without input creates no Undo entry; native commands and their annotations retain their original history", () => {
  let epoch: number | null = 1;
  const native = Annotation.define<boolean>(), witness = Annotation.define<string>();
  let state = EditorState.create({ doc: "正文", extensions: [history(), sourceCompositionHistory(() => epoch, transaction => !!transaction.annotation(native))] });
  state = state.update({ selection: { anchor: 0 } }).state;
  epoch = null;
  state = state.update({ annotations: isolateHistory.of("after") }).state;
  assert.equal(undoDepth(state), 0);
  epoch = 2;
  const command = state.update({ changes: { from: 2, insert: "命令" }, userEvent: "input.native", annotations: [native.of(true), witness.of("exact command"), isolateHistory.of("full")] });
  assert.equal(command.annotation(Transaction.userEvent), "input.native");
  assert.equal(command.annotation(witness), "exact command");
  state = command.state;
  const input = state.update({ changes: { from: 4, insert: "中文" }, userEvent: "input.type.compose", annotations: witness.of("retained native annotation") });
  assert.equal(input.annotation(Transaction.userEvent), "input.type.compose.start");
  assert.equal(input.annotation(witness), "retained native annotation");
  state = input.state;
  const dispatch = (transaction: Transaction) => { state = transaction.state; };
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文命令");
  assert.equal(undo({ state, dispatch }), true); assert.equal(state.doc.toString(), "正文");
});
