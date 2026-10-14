import test from "node:test";
import assert from "node:assert/strict";
import { applySaveResult, draftDisposition, hostedCachePrefix } from "./hosted-state";
test("offline source compares its original base and preserves newer input after a receipt", () => {
  assert.equal(draftDisposition("base", {source:"offline",baseSource:"base"}), "restore");
  assert.equal(draftDisposition("someone else", {source:"offline",baseSource:"base"}), "compare");
  assert.equal(draftDisposition("someone else", {source:"old saved",baseSource:"old saved"}), "current");
  assert.equal(draftDisposition("sent", {source:"new typing",baseSource:"old",pending:{operationId:"one",source:"sent"}}), "unknown");
  assert.deepEqual(applySaveResult({source:"new typing",baseSource:"old"}, "sent"), {source:"new typing",baseSource:"sent"});
});
test("a new server actor epoch cannot look up old source, preview, input or original-plan cache", () => {
  for (const kind of ["draft", "field", "review", "office", "query"] as const) {
    const old = hostedCachePrefix(kind, "deployment", "actor", 1), current = hostedCachePrefix(kind, "deployment", "actor", 2), original = { operationId: "one", source: "protected", preview: "old", value: "unfinished" };
    const storage = new Map([[old + "resource", original]]); // Failed removeItem leaves the actual old record.
    assert.equal(storage.get(current + "resource"), undefined); assert.equal(storage.get(old + "resource"), original);
    assert.equal(hostedCachePrefix(kind, "deployment", "actor", 1), old);
    assert.notEqual(hostedCachePrefix(kind, "a:b", "c", 1), hostedCachePrefix(kind, "a", "b:c", 1));
  }
  for (const generation of [NaN, -1, 0.5, Number.MAX_SAFE_INTEGER + 1]) assert.throws(() => hostedCachePrefix("office", "deployment", "actor", generation));
});
