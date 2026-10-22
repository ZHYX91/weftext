import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { PersonalInputRecovery, personalInputText } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";
import { personalInputId } from "./source-input-origin";

// Execute the actual mounted literal producer and its actual record consumer.
// The editor/HTTP are outside this test; the recovery codec/store are real.
const file = ts.createSourceFile("hosted-editor.ts", readFileSync(new URL("./hosted-editor.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
let callback: ts.ArrowFunction | undefined, records: ts.MethodDeclaration | undefined;
function visit(node: ts.Node): void {
  if (ts.isPropertyAssignment(node) && node.name.getText(file) === "onPersonalInput" && ts.isArrowFunction(node.initializer)) callback = node.initializer;
  if (ts.isMethodDeclaration(node) && node.name.getText(file) === "personalInputRecords") records = node;
  ts.forEachChild(node, visit);
}
visit(file);
assert.ok(callback && records);
const code = ts.transpileModule(`class Producer { ${records.getText(file)} }
function bind() { return ${callback.getText(file)}; }`, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;

test("mounted hosted literal proposals retain valid stable IDs and separate alias owners", async () => {
  let bundle: PersonalInputBundle | undefined;
  const store = new PersonalInputRecovery({
    async read() { return structuredClone(bundle); },
    async compareAndWrite(_audience, expected, metadata, texts) {
      assert.equal(expected, bundle?.commitId ?? 0);
      bundle = { commitId: expected + 1, metadata, texts: new Map(texts) };
      return expected + 1;
    },
  });
  const audience = JSON.stringify(["deployment", "alice"]);
  await store.open(audience);
  const identity = { resource: "alias:A" }, writes: Promise<unknown>[] = [], errors: Error[] = [];
  const context = vm.createContext({ identity, personalInputId });
  vm.runInContext(code, context);
  const host: any = {
    retained: new Map(), literalPersonal: new Map(),
    personalReferences: new Map([["alias:A", "weftext-node://11111111-1111-4111-8111-111111111111/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"], ["alias:B", "weftext-node://11111111-1111-4111-8111-111111111111/bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb"]]),
    error: (error: Error) => errors.push(error),
  };
  host.personalInputRecords = vm.runInContext("Producer.prototype.personalInputRecords", context);
  host.queryHost = { personalInputChanged: () => writes.push(store.update(audience, structuredClone(host.personalInputRecords.call(host)))) };
  const publish = vm.runInContext("bind", context).call(host), inputId = personalInputId();
  publish(inputId, "OWN_RICH_WAITING_😀", 1); await Promise.all(writes);
  assert.deepEqual(errors, []);
  const first = store.list(audience)[0];
  assert.match(first.id, /^[a-f0-9]{32}$/);
  assert.equal(first.origin.detached[0].id, inputId);
  publish(inputId, "OWN_RICH_NEW", 2); await Promise.all(writes);
  assert.equal(store.list(audience)[0].id, first.id);
  identity.resource = "alias:B";
  publish(inputId, "OWN_OTHER_ALIAS", 3); await Promise.all(writes);
  const retained = store.list(audience);
  assert.equal(retained.length, 2);
  assert.equal(new Set(retained.map(record => record.id)).size, 2);
  assert.deepEqual(retained.map(record => record.owner), [{ kind: "body", resource: "alias:A", reference: "weftext-node://11111111-1111-4111-8111-111111111111/aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa" }, { kind: "body", resource: "alias:B", reference: "weftext-node://11111111-1111-4111-8111-111111111111/bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb" }]);
  const restarted = new PersonalInputRecovery({ async read() { return structuredClone(bundle); }, async compareAndWrite() { throw Error("read only"); } });
  await restarted.open(audience);
  assert.deepEqual(restarted.list(audience).map(personalInputText), ["OWN_RICH_NEW", "OWN_OTHER_ALIAS"]);
});
