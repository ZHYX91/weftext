import { UiError, message } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { decodeClientPreferences, hostedLocalePreferences, publicLocalePreferences, localLocalePreferences } from "./ui-locale-settings";
import type { UiLocale } from "./ui-locale";
class MemoryStorage implements Storage {
  readonly values = new Map<string, string>();
  get length() { return this.values.size; }
  clear() { this.values.clear(); }
  key(index: number) { return [...this.values.keys()][index] ?? null; }
  getItem(key: string) { return this.values.get(key) ?? null; }
  removeItem(key: string) { this.values.delete(key); }
  setItem(key: string, value: string) { this.values.set(key, value); }
}

test("public login language has an explicit separate namespace and contains no authenticated context", async () => {
  const storage = new MemoryStorage(), publicPage = publicLocalePreferences(storage, "https://example.test"), personal = hostedLocalePreferences(storage, "https://example.test", "anonymous");
  await publicPage.write(0, "en-US"); assert.equal((await personal.read()).revision, 0);
  assert.equal((await publicLocalePreferences(storage, "https://example.test").read()).preferences.uiLocale, "en-US");
  assert.equal((await publicLocalePreferences(storage, "https://different.test").read()).revision, 0);
  assert.throws(() => publicLocalePreferences(storage, "https://example.test/path"));
  assert.deepEqual(Object.keys(JSON.parse([...storage.values.values()][0])).sort(), ["preferences", "revision", "version"]);
});

test("hosted personal language survives authorization generations and stays isolated by actor and deployment", async () => {
  const storage = new MemoryStorage(), first = hostedLocalePreferences(storage, "deployment-A", "actor-A");
  assert.deepEqual(await first.read(), { revision: 0, preferences: { uiLocale: "zh-CN" } });
  await first.write(0, "en-US");
  assert.equal((await hostedLocalePreferences(storage, "deployment-A", "actor-A").read()).preferences.uiLocale, "en-US");
  assert.equal((await hostedLocalePreferences(storage, "deployment-A", "actor-B").read()).preferences.uiLocale, "zh-CN");
  assert.equal((await hostedLocalePreferences(storage, "deployment-B", "actor-A").read()).preferences.uiLocale, "zh-CN");
  assert.deepEqual([...storage.values], [["weftext-interface-locale:deployment-A:actor-A", JSON.stringify({ version: 1, revision: 1, preferences: { uiLocale: "en-US" } })]]);
});

test("stale hosted language choices and invalid tags preserve the current original setting", async () => {
  const storage = new MemoryStorage(), adapter = hostedLocalePreferences(storage, "deployment", "actor");
  await adapter.write(0, "en-US"); const original = [...storage.values];
  await assert.rejects(adapter.write(0, "zh-CN"), (error: unknown) => { assert.ok(error instanceof UiError); assert.deepEqual(error.spec, message("error.clientPreferencesConflict")); return true; });
  await assert.rejects(adapter.write(1, "en" as UiLocale), UiError);
  await assert.rejects(adapter.write(Number.NaN, "zh-CN"), UiError);
  assert.deepEqual([...storage.values], original);
});

test("unrecognized retained personal records are preserved instead of silently resetting or adopting them", async () => {
  const storage = new MemoryStorage(), key = "weftext-interface-locale:deployment:actor", adapter = hostedLocalePreferences(storage, "deployment", "actor");
  for (const original of ["broken", '{"version":1,"revision":1,"preferences":{"uiLocale":"en-US","uiLocale":"zh-CN"}}', " ".repeat(16 * 1024 + 1), JSON.stringify({ version: 1, revision: 0, preferences: { uiLocale: "en-US" } }), JSON.stringify({ version: 1, revision: 1, preferences: { uiLocale: "en" } }), JSON.stringify({ version: 1, revision: 1, preferences: { uiLocale: "en-US", documentLocale: "en-US" } })]) {
    storage.setItem(key, original); await assert.rejects(adapter.read()); await assert.rejects(adapter.write(0, "zh-CN")); assert.equal(storage.getItem(key), original);
  }
});

test("local preferences send only display choice and CAS revision to the authenticated Core endpoint", async () => {
  const calls: { path: string; body: unknown }[] = [];
  const adapter = localLocalePreferences(async <T>(path: string, body?: unknown) => { calls.push({ path, body }); return { revision: path.endsWith("update") ? 2 : 1, preferences: { uiLocale: "en-US" } } as T; });
  await adapter.read(); await adapter.write(1, "en-US");
  assert.deepEqual(calls, [{ path: "/api/client-preferences/read", body: {} }, { path: "/api/client-preferences/update", body: { expectedRevision: 1, preferences: { uiLocale: "en-US" } } }]);
  for (const value of [{ revision: 1.5, preferences: { uiLocale: "en-US" } }, { revision: 1, preferences: { uiLocale: "en-US" }, source: "private" }, { revision: 1, preferences: { uiLocale: "en" } }]) assert.throws(() => decodeClientPreferences(value));
});
