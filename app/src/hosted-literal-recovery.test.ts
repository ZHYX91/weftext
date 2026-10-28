import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";
import { PersonalInputRecovery, personalInputText } from "./personal-input-recovery";
import type { PersonalInputBundle } from "./personal-input-storage";
import { personalInputId } from "./source-input-origin";
import { UiError, message as uiMessage } from "./ui-locale";

// Execute the actual mounted literal producer and its actual record consumer.
// The editor/HTTP are outside this test; the recovery codec/store are real.
const file = ts.createSourceFile("hosted-editor.ts", readFileSync(new URL("./hosted-editor.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
let callback: ts.ArrowFunction | undefined, records: ts.MethodDeclaration | undefined, bindReference: ts.MethodDeclaration | undefined;
const identityMethods: ts.MethodDeclaration[] = [];
function visit(node: ts.Node): void {
  if (ts.isPropertyAssignment(node) && node.name.getText(file) === "onPersonalInput" && ts.isArrowFunction(node.initializer)) callback = node.initializer;
  if (ts.isMethodDeclaration(node) && node.name.getText(file) === "personalInputRecords") records = node;
  if (ts.isMethodDeclaration(node) && node.name.getText(file) === "personalInputReference") bindReference = node;
  if (ts.isMethodDeclaration(node) && ["rebindResourceAlias", "beginPersonalSave", "settlePersonalSave"].includes(node.name.getText(file))) identityMethods.push(node);
  ts.forEachChild(node, visit);
}
visit(file);
assert.ok(callback && records && bindReference); assert.equal(identityMethods.length, 3);
const code = ts.transpileModule(`class Producer { ${records.getText(file)} ${bindReference.getText(file)} ${identityMethods.map(method => method.getText(file)).join("\n")} }
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

test("a qualified hosted input reference is bound before collection and cannot change a retained owner", async () => {
  let bundle: PersonalInputBundle | undefined, callbacks = 0;
  const store = new PersonalInputRecovery({ async read() { return structuredClone(bundle); }, async compareAndWrite(_audience, expected, metadata, texts) {
    assert.equal(expected, bundle?.commitId ?? 0); bundle = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1;
  } });
  const audience = JSON.stringify(["deployment", "alice"]); await store.open(audience);
  const context = vm.createContext({ UiError, uiMessage }); vm.runInContext(code, context);
  const prototype = vm.runInContext("Producer.prototype", context), id = personalInputId();
  let text = "early own input 😀";
  const reference = "weftext-node://workspace/node-A";
  const editor = { personalInput: () => ({ id, origin: { baseLength: 0, pieces: [{ text }], detached: [], revision: 1 } }) };
  const host: any = { resource: "A", retained: new Map(), literalPersonal: new Map(), personalReferences: new Map(),
    queryHost: { personalInputChanged() { callbacks++; } }, editor,
    editorIdentities: new WeakMap([[editor, { resource: "A", personalOwner: Object.freeze({ kind: "body", resource: "A", reference }) }]]) };
  prototype.personalInputReference.call(host, "A", reference);
  await store.update(audience, structuredClone(prototype.personalInputRecords.call(host)));
  const firstOwner = store.list(audience)[0].owner;
  text = "later own input 😀"; prototype.personalInputReference.call(host, "A", reference);
  await store.update(audience, structuredClone(prototype.personalInputRecords.call(host)));
  assert.deepEqual(store.list(audience)[0].owner, firstOwner);
  assert.equal(callbacks, 0, "identity preparation cannot collect a mounted producer with a changed owner");
  assert.throws(() => prototype.personalInputReference.call(host, "A", "weftext-node://workspace/other-node"));
  assert.equal(host.personalReferences.get("A"), reference);
  assert.equal(personalInputText(store.list(audience)[0]), "later own input 😀");
  assert.equal(bundle!.commitId, 2);
});

test("the same hosted editor keeps its first personal owner across an alias while each save keeps its original operation route", async () => {
  let bundle: PersonalInputBundle | undefined;
  const backend = { async read() { return structuredClone(bundle); }, async compareAndWrite(_audience: string, expected: number, metadata: string, texts: ReadonlyMap<string, string>) {
    assert.equal(expected, bundle?.commitId ?? 0); bundle = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1;
  } };
  const store = new PersonalInputRecovery(backend), audience = JSON.stringify(["deployment", "alice"]); await store.open(audience);
  const context = vm.createContext({ UiError, uiMessage }); vm.runInContext(code, context);
  const prototype = vm.runInContext("Producer.prototype", context), id = personalInputId(), reference = "weftext-node://workspace/same-node";
  let text = "original own input 😀", candidate: any;
  const editor = {
    personalInput: () => ({ id, baseHash: "a".repeat(64), origin: { baseLength: 0, pieces: [{ text }], detached: [], revision: 1 }, ...(candidate ? { candidate } : {}) }),
    async beginPersonalSave(operationId: string) { candidate = { operationId, baseHash: "b".repeat(64), origin: { baseLength: 0, pieces: [{ text }], detached: [], revision: 2 } }; },
    settlePersonalSave(operationId: string) { if (candidate?.operationId === operationId) candidate = undefined; },
  };
  const personalOwner = Object.freeze({ kind: "body", resource: "A", reference });
  const identity: { resource: string; personalOwner: typeof personalOwner; saveRoute?: { operationId: string; resource: string } } = { resource: "A", personalOwner };
  const host: any = { editor, resource: "A", sessionId: "session-A", paused: true, retained: new Map(), literalPersonal: new Map(),
    personalReferences: new Map([["A", reference]]), editorIdentities: new WeakMap([[editor, identity]]) };
  const update = () => store.update(audience, structuredClone(prototype.personalInputRecords.call(host)));
  await update(); prototype.personalInputReference.call(host, "B", reference); prototype.rebindResourceAlias.call(host, "B");
  host.sessionId = "session-B"; host.paused = false; text = "later own input under B 😀"; await update();
  assert.equal(host.editor, editor); assert.equal(identity.resource, "B"); assert.equal(store.list(audience).length, 1);
  assert.equal(store.list(audience)[0].id, id); assert.deepEqual(store.list(audience)[0].owner, personalOwner);
  host.paused = true; prototype.personalInputReference.call(host, "Different", "weftext-node://workspace/different-node");
  assert.throws(() => prototype.rebindResourceAlias.call(host, "Different"));
  assert.equal(host.resource, "B"); assert.equal(identity.resource, "B"); host.paused = false;
  await prototype.beginPersonalSave.call(host, "operation-B", text); await update();
  const pending = store.list(audience)[0];
  assert.ok(pending.pending?.route === "known");
  assert.equal(pending.pending!.resource, "B"); assert.equal(pending.pending!.operationId, "operation-B"); assert.deepEqual(pending.owner, personalOwner);
  host.paused = true; prototype.personalInputReference.call(host, "C", reference); prototype.rebindResourceAlias.call(host, "C"); host.paused = false;
  await assert.rejects(prototype.beginPersonalSave.call(host, "operation-B", text));
  assert.equal(identity.saveRoute!.operationId, "operation-B"); assert.equal(identity.saveRoute!.resource, "B");
  prototype.settlePersonalSave.call(host, "operation-B", true); await store.settle(audience, pending, "committed");
  assert.equal(identity.saveRoute, undefined);
  const reopened = new PersonalInputRecovery(backend); await reopened.open(audience);
  const recovered = reopened.list(audience)[0];
  assert.ok(recovered.pending?.route === "known");
  assert.deepEqual(recovered.owner, personalOwner); assert.equal(recovered.pending!.resource, "B"); assert.equal(recovered.pending!.status, "committed");
  assert.equal(personalInputText(recovered), text);
  await prototype.beginPersonalSave.call(host, "operation-C", text);
  prototype.settlePersonalSave.call(host, "operation-B", true);
  assert.equal(identity.saveRoute!.operationId, "operation-C"); assert.equal(identity.saveRoute!.resource, "C");
  assert.equal(prototype.personalInputRecords.call(host)[0].pending.resource, "C");
  assert.equal(store.list(audience)[0].pending!.operationId, "operation-B");
});

test("an alias during the first save hash cannot relabel a candidate collected before the outer session guard", async () => {
  let bundle: PersonalInputBundle | undefined, release!: () => void, candidate: any;
  const hash = new Promise<void>(resolve => { release = resolve; }), writes: Promise<unknown>[] = [];
  const store = new PersonalInputRecovery({ async read() { return structuredClone(bundle); }, async compareAndWrite(_audience, expected, metadata, texts) {
    assert.equal(expected, bundle?.commitId ?? 0); bundle = { commitId: expected + 1, metadata, texts: new Map(texts) }; return expected + 1;
  } });
  const audience = JSON.stringify(["deployment", "alice"]); await store.open(audience);
  const context = vm.createContext({ UiError, uiMessage }); vm.runInContext(code, context);
  const prototype = vm.runInContext("Producer.prototype", context), id = personalInputId(), reference = "weftext-node://workspace/same-node";
  const origin = () => ({ baseLength: 1, pieces: [{ base: 0, length: 1 }], detached: [], revision: 0 });
  const editor = {
    personalInput: () => ({ id, baseHash: "a".repeat(64), origin: origin(), ...(candidate ? { candidate } : {}) }),
    async beginPersonalSave(operationId: string) {
      await hash; candidate = { operationId, baseHash: "a".repeat(64), origin: origin() };
      // The independent editor-ready callback can collect before the wrapper's
      // awaited SHA continuation gets to reject a now-different session.
      queueMicrotask(() => writes.push(store.update(audience, structuredClone(prototype.personalInputRecords.call(host)))));
    },
    settlePersonalSave(operationId: string) { if (candidate?.operationId === operationId) candidate = undefined; },
  };
  const personalOwner = Object.freeze({ kind: "body", resource: "A", reference });
  const identity: { resource: string; personalOwner: typeof personalOwner; saveRoute?: { operationId: string; resource: string } } = { resource: "A", personalOwner };
  const host: any = { editor, resource: "A", sessionId: "session-A", paused: false, retained: new Map(), literalPersonal: new Map(),
    personalReferences: new Map([["A", reference]]), editorIdentities: new WeakMap([[editor, identity]]) };
  const pending = prototype.beginPersonalSave.call(host, "operation-A", "S"), rejected = assert.rejects(pending);
  host.paused = true; prototype.personalInputReference.call(host, "B", reference); prototype.rebindResourceAlias.call(host, "B");
  host.sessionId = "session-B"; host.paused = false; release(); await rejected; await Promise.all(writes);
  assert.equal(host.resource, "B"); assert.equal(candidate, undefined); assert.equal(identity.saveRoute, undefined);
  const retained = store.list(audience)[0]; assert.deepEqual(retained.owner, personalOwner);
  assert.ok(retained.pending?.route === "known"); assert.equal(retained.pending.operationId, "operation-A"); assert.equal(retained.pending.resource, "A");
  assert.equal(personalInputText(retained), "", "protected Base bytes are not copied into personal recovery");
});
