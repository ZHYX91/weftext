import test from "node:test";
import assert from "node:assert/strict";
import { hostedLinksPatch, requireHostedLinksBasis, sameHostedLinksContext, sameHostedLinksIdentity, type HostedLinksContext, type HostedLinksProposal } from "./hosted-knowledge-links-state";

const context: HostedLinksContext = { actor: "alice", deployment: "d", generation: 4, credential: "token", epoch: 5, authorization: 6, resource: "Shared", sessionId: "session", source: "😀\r\n#link(\"../目标/目标.typ#%E7%AB%A0\")[*原显示文字*]\r\n后文", serial: 7, writable: true, pending: false, compare: false };
const snapshot = "a".repeat(64), hash = "b".repeat(64);
function proposal(): HostedLinksProposal {
  const from = context.source.indexOf('"'), to = context.source.indexOf('"', from + 1) + 1, replacement = '"weftext-node://workspace/node#%E7%AB%A0"';
  return { basis: { actor: context.actor, actorEpoch: context.generation, deployment: context.deployment, resource: context.resource, sessionId: context.sessionId, serial: context.serial, sourceHash: hash, snapshot }, saved: false, patch: { range: { from: new TextEncoder().encode(context.source.slice(0, from)).length, to: new TextEncoder().encode(context.source.slice(0, to)).length, utf16From: from, utf16To: to, raw: context.source.slice(from, to) }, replacement, after: context.source.slice(0, from) + replacement + context.source.slice(to) } };
}
test("late links and clipboard proposals are fenced across actor, authority, navigation, composition input, read mode and equal-byte Undo ABA", () => {
  assert.equal(sameHostedLinksContext(context, { ...context }), true);
  for (const changed of [{ actor: "bob" }, { deployment: "other" }, { generation: 8 }, { credential: "next" }, { epoch: 9 }, { authorization: 9 }, { resource: "Other" }, { sessionId: "reopened" }, { source: context.source + "later" }, { serial: context.serial + 2 }, { writable: false }, { pending: true }, { compare: true }]) assert.equal(sameHostedLinksContext(context, { ...context, ...changed }), false);
  assert.equal(sameHostedLinksContext(context, null), false);
});
test("a known me revocation is identity bound even after typing or navigation; old actor or old authority cannot clear a new connected actor", () => {
  for (const changed of [{ source: "later input", serial: 9 }, { epoch: 8, sessionId: "reopened", resource: "Other" }, { writable: false, pending: true }]) { assert.equal(sameHostedLinksIdentity(context, { ...context, ...changed }), true); assert.equal(sameHostedLinksContext(context, { ...context, ...changed }), false); }
  for (const changed of [{ actor: "bob" }, { credential: "new token" }, { generation: 8 }, { authorization: 10 }, { deployment: "another" }]) assert.equal(sameHostedLinksIdentity(context, { ...context, ...changed }), false);
  assert.equal(sameHostedLinksIdentity(context, null), false);
});
test("one exact native destination splice preserves encoded fragment, explicit display markup and all outside Unicode/CRLF bytes", () => {
  const result = proposal(), patch = hostedLinksPatch(result, context, hash, snapshot);
  assert.equal(patch.expected, '"../目标/目标.typ#%E7%AB%A0"');
  assert.equal(patch.insert, '"weftext-node://workspace/node#%E7%AB%A0"');
  assert.ok(result.patch.after.startsWith("😀\r\n#link("));
  assert.ok(result.patch.after.endsWith(")[*原显示文字*]\r\n后文"));
  assert.equal(result.saved, false);
});
test("same-source old serial, other audience, snapshot substitution and saved-result promotion cannot grant a Draft writer", () => {
  const valid = proposal();
  for (const change of [{ actor: "bob" }, { actorEpoch: 5 }, { deployment: "other" }, { resource: "Other" }, { sessionId: "other" }, { serial: 9 }, { sourceHash: "c".repeat(64) }, { snapshot: "d".repeat(64) }]) assert.throws(() => requireHostedLinksBasis({ ...valid.basis, ...change }, context, hash, snapshot));
  assert.throws(() => hostedLinksPatch({ ...valid, saved: true as false }, context, hash, snapshot));
  for (const changed of [{ writable: false }, { pending: true }, { compare: true }]) assert.throws(() => hostedLinksPatch(valid, { ...context, ...changed }, hash, snapshot));
});
test("wrong raw, UTF-8/UTF-16 disagreement, surrogate/CRLF splits and unrelated after edits are rejected before the shared Undo transaction", () => {
  const valid = proposal();
  const altered = structuredClone(valid); altered.patch.after += "unrelated overwrite"; assert.throws(() => hostedLinksPatch(altered, context, hash, snapshot));
  for (const change of [{ from: valid.patch.range.from + 1 }, { to: valid.patch.range.to + 1 }, { raw: "different" }, { utf16From: -1 }, { utf16To: 1.5 }, { utf16To: context.source.length + 1 }]) assert.throws(() => hostedLinksPatch({ ...valid, patch: { ...valid.patch, range: { ...valid.patch.range, ...change } } }, context, hash, snapshot));
  for (const [from, to] of [[1, 2], [3, 4]]) { const split = structuredClone(valid); split.patch.range = { from, to, utf16From: from, utf16To: to, raw: context.source.slice(from, to) }; assert.throws(() => hostedLinksPatch(split, context, hash, snapshot)); }
});
