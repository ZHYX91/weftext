import test from "node:test";
import assert from "node:assert/strict";
import { nodeFragmentOrigin, nodeReferenceParts, nodeTargetDraft, sameHostedNodeAudience, type HostedNodeAudience, type HostedNodeBasis, type HostedNodeFragmentResult } from "./hosted-navigation-state";

test("stable node fragment decoding keeps the original URI and refuses empty/malformed labels without a whole-note fallback", () => {
  const base = "weftext-node://workspace/node";
  assert.deepEqual(nodeReferenceParts(base), { base });
  assert.deepEqual(nodeReferenceParts(base + "#%E4%B8%AD%E6%96%87"), { base, fragment: "中文" });
  assert.deepEqual(nodeReferenceParts(base + "#中文"), { base, fragment: "中文" });
  for (const invalid of ["#", "#%", "#%FF", "#a#b", "#%00"]) assert.throws(() => nodeReferenceParts(base + invalid));
  assert.throws(() => nodeReferenceParts("file:///private.typ#label"));
});
test("label range delivery is bound to identity, exact current target/origin buffer and monotonic serial even after equal-source Undo", () => {
  const basis: HostedNodeAudience = { epoch: 1, authorization: 2, credential: "one", actor: "alice", deployment: "d", generation: 3, sessionId: "session", source: "same", serial: 4, navigation: 5 };
  assert.equal(sameHostedNodeAudience(basis, { ...basis }), true);
  for (const changed of [{ epoch: 2 }, { authorization: 3 }, { credential: "two" }, { actor: "bob" }, { deployment: "other" }, { generation: 4 }, { sessionId: "reopened" }, { source: "later" }, { serial: 6 }, { navigation: 7 }]) assert.equal(sameHostedNodeAudience(basis, { ...basis, ...changed }), false);
});
test("navigation stages the actor's actual local/recovered Draft and never settles a pending save or replaces its original basis", () => {
  assert.deepEqual(nodeTargetDraft("saved", "recovered", "old"), { source: "recovered", baseSource: "old" });
  const local = { source: "local", baseSource: "old", definitionReservations: ["original"] };
  assert.deepEqual(nodeTargetDraft("saved", "server draft", "server base", local), local);
  assert.notEqual(nodeTargetDraft("saved", undefined, undefined, local), local);
  assert.deepEqual(nodeTargetDraft("new saved", undefined, undefined, { source: "old saved", baseSource: "old saved" }), { source: "old saved", baseSource: "old saved" });
  assert.throws(() => nodeTargetDraft("saved", undefined, undefined, { ...local, pending: { resource: "Shared", operationId: "unknown", source: "sent" } }), /原保存/);
  for (const invalid of [null, {}, { source: "draft" }, { source: 2, baseSource: "base" }]) assert.throws(() => nodeTargetDraft("saved", undefined, undefined, invalid as unknown as typeof local), /恢复稿/);
});
test("only an exact Draft compile's unique main-source range is selected; saved, include, duplicate, wrong-resource and broken Unicode ranges cannot be reused", () => {
  const reference = "weftext-node://workspace/node#中文", source = "😀\r\n= 标题 <中文>\r\n正文";
  const basis: HostedNodeBasis = { sessionId: "session", source, serial: 2, sourceHash: "exactHash" };
  const from = 4, to = source.indexOf("\r\n", from), origin = { path: "resource:target", from: 6, to: 22, utf16From: from, utf16To: to, raw: source.slice(from, to), syntaxKind: "Heading", isMain: true };
  const result: HostedNodeFragmentResult = { resource: "target", reference, serial: 2, sourceHash: "exactHash", fragment: { label: "中文", status: "found", basis: "draft", sourceHash: "exactHash", origin } };
  assert.deepEqual(nodeFragmentOrigin(result, "target", reference, basis), origin);
  for (const status of ["missing", "ambiguous", "unavailable"] as const) assert.equal(nodeFragmentOrigin({ ...result, fragment: { ...result.fragment, status, origin: undefined } }, "target", reference, basis), undefined);
  for (const changed of [{ resource: "other" }, { reference: reference + "x" }, { serial: 3 }, { sourceHash: "old" }, { fragment: { ...result.fragment, basis: "saved" as const } }, { fragment: { ...result.fragment, label: "wrong" } }, { fragment: { ...result.fragment, origin: { ...origin, isMain: false } } }, { fragment: { ...result.fragment, origin: { ...origin, path: "private/include.typ" } } }, { fragment: { ...result.fragment, origin: { ...origin, raw: "different" } } }, { fragment: { ...result.fragment, origin: { ...origin, utf16From: 1, utf16To: 2, raw: source.slice(1, 2) } } }, { fragment: { ...result.fragment, origin: { ...origin, utf16From: 3, raw: source.slice(3, to) } } }]) assert.throws(() => nodeFragmentOrigin({ ...result, ...changed }, "target", reference, basis));
});
