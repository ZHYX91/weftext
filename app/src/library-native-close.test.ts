import test from "node:test";
import assert from "node:assert/strict";
import { LibraryCloseActivity } from "./library-native-close";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

test("Library close tracks the entire caller after its transport has resolved", async () => {
  const activity = new LibraryCloseActivity();
  const release = activity.begin();
  await Promise.resolve();
  assert.throws(() => activity.checkpoint());
  release();
  const checkpoint = activity.checkpoint();
  assert.equal(activity.freeze(checkpoint), true);
  assert.throws(() => activity.begin());
  activity.thaw();
  const next = activity.begin(); next();
  assert.equal(activity.freeze(activity.checkpoint()), true);
});

test("completed intervening work invalidates a prepared Library close", () => {
  const activity = new LibraryCloseActivity();
  const prepared = activity.checkpoint();
  const outer = activity.begin(), inner = activity.begin();
  outer(); outer();
  assert.throws(() => activity.checkpoint());
  inner();
  assert.equal(activity.freeze(prepared), false);
  assert.equal(activity.freeze(activity.checkpoint()), true);
});

test("Library callback ownership includes delayed delivery and releases on rejection", async () => {
  const activity = new LibraryCloseActivity();
  let reject!: (error: Error) => void;
  const held = new Promise<void>((_resolve, no) => { reject = no; });
  const running = activity.run(() => held);
  assert.throws(() => activity.checkpoint());
  reject(new Error("delivery failed"));
  await assert.rejects(running, /delivery failed/);
  assert.equal(activity.freeze(activity.checkpoint()), true);
  let invoked = false;
  await assert.rejects(activity.run(async () => { invoked = true; }));
  assert.equal(invoked, false);
});

test("the actual Library modal retains its activity until the user closes it", async () => {
  const source = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
  const modal = source.statements.find(node => ts.isFunctionDeclaration(node) && node.name?.text === "modal")!;
  const activity = new LibraryCloseActivity();
  const element = () => ({ append() {}, addEventListener() {}, showModal() {}, close() {}, remove() {} });
  const context: any = { libraryCloseActivity: activity, document: { activeElement: null, body: element() }, el: element, button: element, setUiAttr() {}, displayMessage: () => "close", uiDisplayParameter: (value: unknown) => value, setTimeout() {} };
  vm.createContext(context);
  vm.runInContext(ts.transpileModule(modal.getText(source), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText, context);
  let dismiss!: (value: string | null) => void;
  const result = context.modal("title", "description", (_form: unknown, close: typeof dismiss) => { dismiss = close; });
  await Promise.resolve();
  assert.throws(() => activity.checkpoint());
  dismiss("accepted");
  assert.equal(await result, "accepted");
  assert.equal(activity.freeze(activity.checkpoint()), true);
});
