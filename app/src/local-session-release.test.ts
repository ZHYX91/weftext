import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

// Execute the actual main owner port. Only the HTTP, message rendering and DOM
// presentation boundaries are substituted; Core/auth are tested by desktop_api.
const source = ts.createSourceFile("main.ts", readFileSync(new URL("./main.ts", import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TS);
const ports = source.statements.filter(node => ts.isFunctionDeclaration(node) && ["releaseOpenedSession","ownsOpenedSession"].includes(node.name?.text ?? ""));
assert.equal(ports.length,2);
const compiled = ts.transpileModule(ports.map(node=>node.getText(source)).join("\n"), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None } }).outputText;
function fixture() {
  const calls: { path: string; body: { sessionId: string } }[] = [];
  const errors: any[] = [];
  const context: any = {
    documents: [{ session: { opened: { sessionId: "adopted-original" } } }],
    message: (key: string, params: object = {}) => ({ key, params }),
    t: (value: { key: string; params: object }) => `${value.key}:${JSON.stringify(value.params)}`,
    el: (_tag: string, _className: string, text: string) => ({ text, attributes: {} as Record<string, string>, children: [] as any[], removed: false, setAttribute(key: string, value: string) { this.attributes[key] = value; }, append(child: any) { this.children.push(child); }, remove() { this.removed = true; } }),
    button: (_text: string, action: () => void) => ({ action, setAttribute() {} }),
    toastHost: { append(item: any) { errors.push(item); } },
    api: async (path: string, body: { sessionId: string }) => { calls.push({ path, body }); return { released: true, sessionId: body.sessionId }; },
  };
  vm.createContext(context); vm.runInContext(compiled, context);
  return { context, calls, errors };
}
test("an adopted document cannot be released as a temporary OpenResult", async () => {
  const f = fixture();
  await assert.rejects(f.context.releaseOpenedSession("adopted-original"), /sessionAlreadyAdopted/);
  assert.equal(f.calls.length, 0);
  assert.equal(f.context.documents[0].session.opened.sessionId, "adopted-original");
});
test("a comparison target is also owned by the live document and cannot be released", async () => {
  const f=fixture();
  f.context.documents[0].comparisonTarget={sessionId:"comparison-open-result"};
  await assert.rejects(f.context.releaseOpenedSession("comparison-open-result"),/sessionAlreadyAdopted/);
  assert.equal(f.calls.length,0);
});
test("late temporary cleanup keeps the newly selected document and sends only the original ID", async () => {
  const f = fixture();
  f.context.documents = [{ session: { opened: { sessionId: "new-current-document" } } }];
  await f.context.releaseOpenedSession("late-unadopted-result");
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].path, "/api/session/release");
  assert.equal(f.calls[0].body.sessionId, "late-unadopted-result");
  assert.equal(f.context.documents[0].session.opened.sessionId, "new-current-document");
  assert.equal(f.errors.length, 0);
});
test("a failed release remains visible in the host after its requesting panel is gone", async () => {
  const f = fixture();
  let reject!: (error: Error) => void;
  f.context.api = () => new Promise((_resolve, failed) => { reject = failed; });
  const pending = f.context.releaseOpenedSession("original-unadopted-session");
  f.context.documents = [];
  reject(new Error("transport effect unknown"));
  await assert.rejects(pending, /effect unknown/);
  assert.equal(f.errors.length, 1);
  assert.equal(f.errors[0].attributes.role, "alert");
  assert.match(f.errors[0].text, /original-unadopted-session/);
  assert.equal(f.errors[0].removed, false);
  f.errors[0].children[0].action();
  assert.equal(f.errors[0].removed, true);
});
test("a mismatched acknowledgement cannot manufacture a successful session release", async () => {
  const f = fixture();
  f.context.api = async () => ({ released: true, sessionId: "other-session" });
  await assert.rejects(f.context.releaseOpenedSession("original-session"), /sessionReleaseUnconfirmed/);
  assert.equal(f.errors.length, 1);
  assert.match(f.errors[0].text, /original-session/);
});
