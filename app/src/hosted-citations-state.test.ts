import assert from "node:assert/strict";
import test from "node:test";
import { citationPatches, type CitationProposal } from "./hosted-citations-state";
import { sameHostedLinksContext, type HostedLinksContext } from "./hosted-knowledge-links-state";
const context: HostedLinksContext = { actor: "a", deployment: "d", generation: 2, credential: "token", authorization: 4, epoch: 5, resource: "note", sessionId: "session", source: "中文😀\r\nbody", serial: 7, writable: true, pending: false, compare: false };
const hash = "a".repeat(64), text = "\n@paper_1\n";
const proposal: CitationProposal = { resource: context.resource, serial: context.serial, sourceHash: hash, patchUnit: "utf16", source: context.source + text, patches: [{ from: context.source.length, to: context.source.length, text, expected: "", byteFrom: new TextEncoder().encode(context.source).length, byteTo: new TextEncoder().encode(context.source).length }] };
test("citation changes enter only proved shared-source patches and preserve complex original bytes", () => {
  assert.deepEqual(citationPatches(proposal, context, hash), [{ from: context.source.length, to: context.source.length, expected: "", insert: text }]);
  for (const invalid of [{ ...proposal, resource: "other" }, { ...proposal, serial: 6 }, { ...proposal, sourceHash: "b".repeat(64) }, { ...proposal, source: "replacement" }, { ...proposal, patches: [{ ...proposal.patches[0], byteFrom: 0 }] }, { ...proposal, patches: [{ ...proposal.patches[0], from: 3, to: 3 }] }]) assert.throws(() => citationPatches(invalid, context, hash));
  for (const blocked of [{ ...context, writable: false }, { ...context, pending: true }, { ...context, compare: true }]) assert.throws(() => citationPatches(proposal, blocked, hash));
});
test("equal source bytes do not revive a citation proposal after Undo or input change", () => {
  assert.equal(sameHostedLinksContext(context, { ...context, serial: 9 }), false);
  assert.throws(() => citationPatches(proposal, { ...context, serial: 9 }, hash));
  assert.equal(sameHostedLinksContext(context, { ...context, generation: 3 }), false);
});
