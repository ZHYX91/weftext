import assert from "node:assert/strict";
import test from "node:test";
import { HostedDraftCache } from "./hosted-draft-cache";
import { hostedCachePrefix, type HostedDraft } from "./hosted-state";

class Storage {
  values = new Map<string, string>(); writes: { key: string; length: number }[] = [];
  fail?: (key: string, value: string) => boolean;
  getItem(key: string) { return this.values.get(key) ?? null; }
  setItem(key: string, value: string) { if (this.fail?.(key, value)) throw new Error("quota"); this.values.set(key, value); this.writes.push({ key, length: value.length }); }
  removeItem(key: string) { this.values.delete(key); }
}
const prefix = hostedCachePrefix("draft", "test", "writer", 1);
const large = "中文😀\r\n".repeat(30000);
const create = (storage = new Storage()) => ({ storage, cache: new HostedDraftCache(storage, () => prefix) });
const fresh = (storage: Storage) => new HostedDraftCache(storage, () => prefix);

test("small recovery, frozen original save and reservations roundtrip through one codec", () => {
  const { cache, storage } = create();
  const draft: HostedDraft = { source: "新😀\ud800", baseSource: "旧\r\n", reference: "weftext-node://test/id", definitionReservations: ["r"], pending: { resource: "Shared", operationId: "op", source: "原请求", request: { sessionId: "session", baseId: "base", operationId: "op", source: "原请求", definitionReservations: ["r"] } } };
  cache.write("note", draft);
  assert.deepEqual(fresh(storage).read("note"), draft);
  assert.deepEqual(JSON.parse(storage.getItem(prefix + "note")!), draft);
});

test("large edits synchronously persist compact metadata and survive a fresh reader", () => {
  const { cache, storage } = create(); const draft: HostedDraft = { source: large, baseSource: large };
  cache.write("note", draft);
  const snapshotWrites = () => storage.writes.filter(write => write.length > 65536);
  assert.equal(snapshotWrites().length, 1);
  for (let i = 0; i < 80; i++) {
    draft.source = large.slice(0, large.length - i) + "不同😀" + String(i);
    cache.write("note", draft);
    if (i % 10 === 0) assert.deepEqual(fresh(storage).read("note"), draft);
  }
  assert.equal(snapshotWrites().length, 1);
  assert.ok(storage.writes.filter(write => write.key === prefix + "note").every(write => write.length < 4096));
  assert.deepEqual(fresh(storage).read("note"), draft);
});

test("a large unsaved replacement keeps its small original Base and independent pending request", () => {
  const { cache, storage } = create();
  const draft: HostedDraft = { source: large, baseSource: "原先小稿", pending: { resource: "Shared", operationId: "op", source: large + "旧提案", request: { sessionId: "s", baseId: "b", operationId: "op", source: large + "旧提案" } } };
  cache.write("note", draft);
  draft.source += "后到的新输入"; cache.write("note", draft);
  assert.deepEqual(fresh(storage).read("note"), draft);
  assert.equal(storage.writes.filter(write => write.length > 65536).length, 1);
});

test("failed metadata commit leaves the complete previous recovery and unknown operation untouched", () => {
  const { cache, storage } = create();
  const previous: HostedDraft = { source: large, baseSource: "旧", pending: { resource: "Shared", operationId: "unknown", source: large, request: { sessionId: "s", baseId: "b", operationId: "unknown", source: large } } };
  cache.write("note", previous);
  const original = storage.getItem(prefix + "note"); const keys = [...storage.values.keys()].sort();
  storage.fail = key => key === prefix + "note";
  assert.throws(() => cache.write("note", { source: "完全替换".repeat(100000), baseSource: "另一个基线" }), /quota/);
  assert.equal(storage.getItem(prefix + "note"), original);
  assert.deepEqual([...storage.values.keys()].sort(), keys);
  assert.deepEqual(fresh(storage).read("note"), previous);
});

test("snapshot write failure preserves literal recovery without a destructive conversion", () => {
  const { cache, storage } = create(); const draft: HostedDraft = { source: large, baseSource: large };
  const original = JSON.stringify(draft); storage.values.set(prefix + "note", original);
  storage.fail = key => key.startsWith("weftext-hosted-draft-text:");
  assert.deepEqual(cache.read("note"), draft);
  assert.throws(() => cache.write("note", { ...draft, source: large + "x" }), /quota/);
  assert.equal(storage.getItem(prefix + "note"), original);
  assert.deepEqual(fresh(storage).read("note"), draft);
});

test("missing, cross-resource and invalid ranges fail closed without rewriting original records", () => {
  const { cache, storage } = create(); cache.write("A", { source: large, baseSource: large });
  const raw = storage.getItem(prefix + "A")!;
  storage.values.set(prefix + "B", raw);
  assert.throws(() => fresh(storage).read("B"));
  const corrupt = JSON.parse(raw); corrupt.source.to = large.length + 1;
  storage.values.set(prefix + "A", JSON.stringify(corrupt));
  assert.throws(() => fresh(storage).read("A"));
  assert.equal(storage.getItem(prefix + "A"), JSON.stringify(corrupt));
  storage.values.set(prefix + "A", raw);
  const snapshot = [...storage.values.keys()].find(key => key.startsWith("weftext-hosted-draft-text:"))!;
  storage.values.delete(snapshot); assert.throws(() => fresh(storage).read("A"));
  assert.equal(storage.getItem(prefix + "A"), raw);
});

test("actor generations and metadata namespaces cannot borrow one another's snapshots", () => {
  const storage = new Storage(); let audience = prefix; const cache = new HostedDraftCache(storage, () => audience);
  cache.write("~text:A:reserved", { source: large, baseSource: large });
  audience = hostedCachePrefix("draft", "test", "other", 2);
  assert.equal(cache.read("~text:A:reserved"), undefined);
  cache.write("~text:A:reserved", { source: large + "其他", baseSource: "另一个" });
  assert.equal(cache.read("~text:A:reserved")!.source, large + "其他");
  audience = prefix; assert.equal(cache.read("~text:A:reserved")!.source, large);
});

test("replacement releases only snapshots no longer referenced by committed recovery", () => {
  const { cache, storage } = create(); cache.write("note", { source: large, baseSource: large });
  const oldKeys = [...storage.values.keys()].filter(key => key.startsWith("weftext-hosted-draft-text:"));
  const draft = { source: "另".repeat(200000), baseSource: "新基线" };
  cache.write("note", draft);
  assert.deepEqual(fresh(storage).read("note"), draft);
  assert.ok(oldKeys.every(key => !storage.values.has(key)));
});

test("a retained page cannot overwrite a newer recovery record or revive reclaimed snapshots", () => {
  const { cache, storage } = create(); const retainedPage = fresh(storage);
  cache.write("note", { source: large, baseSource: large }); retainedPage.read("note");
  const newer: HostedDraft = { source: "新".repeat(200000), baseSource: "另一个基线", pending: { resource: "Shared", operationId: "new-operation", source: "新".repeat(200000) } };
  cache.write("note", newer); const committed = storage.getItem(prefix + "note");
  assert.throws(() => retainedPage.write("note", { source: large + "旧页的新输入", baseSource: large }), /其他页面更新/);
  assert.equal(storage.getItem(prefix + "note"), committed);
  assert.deepEqual(fresh(storage).read("note"), newer);
});


test("a pending save without its frozen resource remains unsupported and preserves its original cache bytes", () => {
  const { storage } = create();
  for (const resource of [undefined, "", 42]) {
    const raw = JSON.stringify({ source: "later Y", baseSource: "B", pending: { resource, operationId: "original", source: "X" } });
    storage.values.set(prefix + "Alias", raw);
    assert.throws(() => fresh(storage).read("Alias"), /无法完整读取/);
    assert.equal(storage.values.get(prefix + "Alias"), raw);
  }
});
