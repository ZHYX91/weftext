import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { AuthoringAssetsAdapter } from "./authoring-assets-adapter";

test("local and hosted module adapters pass exact session/owner selectors and never a Save request", async () => {
  for (const prefix of ["/api/editor/endnotes-module", "editor/endnotes-module"]) {
    const source = "// fixed module", sha256 = createHash("sha256").update(source).digest("hex"), identity = { scope: "actor-scope", owner: "weftext-node://w/n", binding: "owner-collection-config", modulePath: "weftext-endnotes.typ" }, calls: { path: string; body?: Record<string, unknown> }[] = [], stored = new Map<string, string>(); let installed = false, changed = 0;
    const adapter = new AuthoringAssetsAdapter({ prefix, context: () => ({ sessionId: "current-session", audience: "deployment:alice" }), changed: () => { changed++; }, storage: { getItem: key => stored.get(key) ?? null, setItem: (key, value) => { stored.set(key, value); } }, request: async <T>(path: string, body?: unknown): Promise<T> => {
      const value = body as Record<string, unknown> | undefined; calls.push({ path, body: value });
      if (path === prefix) { assert.equal(body, undefined); return { name: "weftext-endnotes.typ", source, sha256, version: "1" } as T; }
      assert.equal(value?.sessionId, "current-session");
      if (path.endsWith("/catalog")) return { identity, coverage: "complete", owner: identity.owner, resources: installed ? [{ resourceRef: "new-module", owner: identity.owner, modulePath: identity.modulePath }] : [] } as T;
      assert.deepEqual(value?.identity, identity);
      if (path.endsWith("/prepare")) { assert.ok(stored.size > 0, "original operation must be retained before resource mutation"); return { operationId: value?.operationId, status: "prepared", planId: "original-plan", commitAllowed: true } as T; }
      if (path.endsWith("/commit")) { installed = true; return { operationId: value?.operationId, status: "committed", planId: "original-plan", commitAllowed: false, resourceRefs: ["new-module"] } as T; }
      if (path.endsWith("/read")) { assert.equal(value?.resourceRef, "new-module"); return { resourceRef: "new-module", source, sha256 } as T; }
      if (path.endsWith("/authorized")) return { authorized: true } as T;
      throw new Error(path);
    } });
    assert.equal((await adapter.install()).resourceRef, "new-module"); assert.equal(changed, 1); assert.ok(calls.every(call => !/save|source-edit|write-file/.test(call.path))); assert.ok([...stored.keys()].every(key => key.includes(encodeURIComponent("deployment:alice"))));
  }
});

test("unknown module preparation blocks close until the original Core operation is settled", async () => {
  const source = "// fixed module", sha256 = createHash("sha256").update(source).digest("hex");
  const identity = { scope: "actor-scope", owner: "weftext-node://w/n", binding: "collection", modulePath: "weftext-endnotes.typ" };
  const stored = new Map<string, string>();
  let original = "";
  const adapter = new AuthoringAssetsAdapter({ prefix: "/module", context: () => ({ sessionId: "session", audience: "local:author" }), changed() {}, storage: { getItem: key => stored.get(key) ?? null, setItem: (key, value) => { stored.set(key, value); } }, request: async <T>(path: string, body?: unknown): Promise<T> => {
    const value = body as Record<string, unknown>;
    if (path === "/module") return { name: identity.modulePath, source, sha256, version: "1" } as T;
    if (path.endsWith("/catalog")) return { identity, coverage: "complete", owner: identity.owner, resources: [] } as T;
    if (path.endsWith("/prepare")) { original = String(value.operationId); throw new Error("original response was lost"); }
    if (path.endsWith("/status")) { assert.equal(value.operationId, original); return { operationId: original, status: "cancelled", commitAllowed: false } as T; }
    throw new Error(path);
  } });
  await assert.rejects(adapter.install(), /lost/);
  assert.throws(() => adapter.assertCloseReady());
  assert.equal(adapter.freezeForClose(), false);
  adapter.invalidate();
  assert.throws(() => adapter.assertCloseReady(), "retiring display cannot settle the original operation");
  assert.equal((await adapter.recover())?.status, "cancelled");
  adapter.assertCloseReady();
  assert.equal(adapter.freezeForClose(), true);
  await assert.rejects(adapter.install());
  adapter.thawAfterClose();
  adapter.assertCloseReady();
});
