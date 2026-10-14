import test from "node:test";
import assert from "node:assert/strict";
import { applySaveResult, draftDisposition } from "./hosted-state";
test("offline source compares its original base and preserves newer input after a receipt", () => {
  assert.equal(draftDisposition("base", {source:"offline",baseSource:"base"}), "restore");
  assert.equal(draftDisposition("someone else", {source:"offline",baseSource:"base"}), "compare");
  assert.equal(draftDisposition("someone else", {source:"old saved",baseSource:"old saved"}), "current");
  assert.equal(draftDisposition("sent", {source:"new typing",baseSource:"old",pending:{operationId:"one",source:"sent"}}), "unknown");
  assert.deepEqual(applySaveResult({source:"new typing",baseSource:"old"}, "sent"), {source:"new typing",baseSource:"sent"});
});
