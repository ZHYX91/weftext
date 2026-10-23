import test from "node:test";
import assert from "node:assert/strict";
import { bindHostedDraftReference, applySaveResult, draftDisposition, freezeHostedSave, mergeDefinitionReservations, hostedCachePrefix, sameHostedQueryAudience, sameHostedQueryIdentity, type HostedDraft } from "./hosted-state";
test("offline source compares its original base and preserves newer input after a receipt", () => {
  assert.equal(draftDisposition("base", {source:"offline",baseSource:"base"}), "restore");
  assert.equal(draftDisposition("someone else", {source:"offline",baseSource:"base"}), "compare");
  assert.equal(draftDisposition("someone else", {source:"old saved",baseSource:"old saved"}), "current");
  assert.equal(draftDisposition("sent", {source:"new typing",baseSource:"old",pending:{resource:"Shared",operationId:"one",source:"sent"}}), "unknown");
  assert.deepEqual(applySaveResult({source:"new typing",baseSource:"old"}, "sent"), {source:"new typing",baseSource:"sent"});
});
test("a qualified stable Node binds an unbound personal Draft without changing its original basis or operation, while a different Node never borrows the old Draft", () => {
  const original: HostedDraft = { source: "later actor input", baseSource: "original saved", pending: { resource: "Shared", operationId: "original-save", source: "sent" }, definitionReservations: ["reserved-key"] };
  const reference = "weftext-node://workspace/original", bound = bindHostedDraftReference(original, reference);
  assert.deepEqual(bound, { ...original, reference }); assert.equal(original.reference, undefined); assert.deepEqual(bindHostedDraftReference(bound, reference), bound);
  assert.throws(() => bindHostedDraftReference(bound, "weftext-node://workspace/another"), /身份与原草稿不同/); assert.equal(bound.reference, reference); assert.equal(bound.source, original.source); assert.equal(bound.baseSource, original.baseSource);
});
test("an asynchronous query denial may affect only its original actor, epoch, document session and input revision", () => {
  const captured = { epoch: 3, credential: "original", actor: "alice", deployment: "space", generation: 5, sessionId: "original-note", queryVersion: 8 };
  assert.equal(sameHostedQueryAudience(captured, { ...captured }), true);
  for (const changed of [{ epoch: 4 }, { credential: "new" }, { actor: "bob" }, { deployment: "other" }, { generation: 6 }, { sessionId: "reopened-note" }, { queryVersion: 10 }]) assert.equal(sameHostedQueryAudience(captured, { ...captured, ...changed }), false);
  for (const changed of [{ sessionId: "new-note" }, { queryVersion: 10 }, { sessionId: "new-note", queryVersion: 10 }]) { const later = { ...captured, ...changed }; assert.equal(sameHostedQueryIdentity(captured, later), true); assert.equal(sameHostedQueryAudience(captured, later), false); }
  assert.equal(sameHostedQueryIdentity(captured, { ...captured, generation: 6, queryVersion: 10 }), false); assert.equal(sameHostedQueryIdentity(captured, { ...captured, actor: "bob", sessionId: "new-note" }), false);
});
test("original save freezes reservation IDs while later typing and Undo retain the owner reservations", () => {
  const draft: HostedDraft = { source: "proposal", baseSource: "base", definitionReservations: ["first"] };
  const request = freezeHostedSave(draft, "session", "base-id", "save-one");
  draft.pending = { resource: "Shared", operationId: request.operationId, source: request.source, request };
  draft.source = "new typing"; draft.definitionReservations!.push("later");
  assert.deepEqual(request, { sessionId: "session", baseId: "base-id", operationId: "save-one", source: "proposal", definitionReservations: ["first"] });
  assert.deepEqual(applySaveResult(draft, "proposal"), { source: "new typing", baseSource: "proposal", definitionReservations: ["first", "later"] });
  const undo: HostedDraft = { ...applySaveResult(draft, "proposal"), source: "base" };
  assert.deepEqual(freezeHostedSave(undo, "new-session", "new-base", "save-two").definitionReservations, ["first", "later"]);
});
test("a pending-only directory cannot accumulate unbound IDs or prove admission; exact statuses clean the next Save", () => {
  const directory = Array.from({ length: 128 }, (_, index) => ({ operationId: `server-pending-${index}`, status: "reserved" }));
  assert.deepEqual(mergeDefinitionReservations(["applied", "unknown"], directory), ["applied", "unknown"]);
  assert.deepEqual(mergeDefinitionReservations(["applied", "unknown"], []), ["applied", "unknown"]);
  assert.deepEqual(mergeDefinitionReservations(["applied", "unknown"], [{ operationId: "unknown", status: "notRegistered" }]), ["applied", "unknown"]);
  const draft: HostedDraft = { source: "saved carrier", baseSource: "base", definitionReservations: ["applied", "cancelled", "unknown"] };
  const original = freezeHostedSave(draft, "session", "base", "save-first");
  draft.definitionReservations = mergeDefinitionReservations(draft.definitionReservations!, [{ operationId: "applied", status: "admitted" }, { operationId: "cancelled", status: "cancelled" }]);
  assert.deepEqual(freezeHostedSave(draft, "session", "new-base", "save-next").definitionReservations, ["unknown"]);
  assert.deepEqual(original.definitionReservations, ["applied", "cancelled", "unknown"]);
  assert.throws(() => freezeHostedSave({ ...draft, definitionReservations: directory.slice(0, 33).map(item => item.operationId) }, "session", "base", "blocked"), /32/);
});
test("a new server actor epoch cannot look up old source, preview, input or original-plan cache", () => {
  for (const kind of ["draft", "field", "review", "office", "query", "history", "library"] as const) {
    const old = hostedCachePrefix(kind, "deployment", "actor", 1), current = hostedCachePrefix(kind, "deployment", "actor", 2), original = { operationId: "one", source: "protected", preview: "old", value: "unfinished" };
    const storage = new Map([[old + "resource", original]]); // Failed removeItem leaves the actual old record.
    assert.equal(storage.get(current + "resource"), undefined); assert.equal(storage.get(old + "resource"), original);
    assert.equal(hostedCachePrefix(kind, "deployment", "actor", 1), old);
    assert.notEqual(hostedCachePrefix(kind, "a:b", "c", 1), hostedCachePrefix(kind, "a", "b:c", 1));
  }
  for (const generation of [NaN, -1, 0.5, Number.MAX_SAFE_INTEGER + 1]) assert.throws(() => hostedCachePrefix("office", "deployment", "actor", generation));
});
