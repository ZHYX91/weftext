import test from "node:test";
import assert from "node:assert/strict";
import { citationCandidateKey, citationInsertionByte, citationInputScope, type NativeCitationSnapshot } from "./native-citations";

const profile: NativeCitationSnapshot = { generation: "generation", coverage: "partial", choices: [{ reference: "work-a", title: "A", citationReady: true, suggestedKey: "zhang2026" }, { reference: "work-b", title: "B", citationReady: true, suggestedKey: "zhang2026" }], editor: { sourceHash: "hash", bindings: [{ key: "existing", work: "work-a" }], citations: [], targets: [], crossReferences: [], renameAvailable: true }, bindings: [], incoming: [], displayStatus: "unavailable", displayExplanation: "unconfirmed" };
test("retained citation inputs belong to one actor and stable owner rather than a replaced session or locator",()=>{const context={deployment:"server",actor:"alice",resource:"OldPath"};const scope=citationInputScope(context,"stable-owner");assert.equal(scope,citationInputScope({...context,resource:"MovedPath"},"stable-owner"));assert.notEqual(scope,citationInputScope({...context,actor:"bob"},"stable-owner"));assert.notEqual(scope,citationInputScope(context,"another-owner"));assert.notEqual(scope,citationInputScope({...context,deployment:"elsewhere"},"stable-owner"));});
test("citation key reuses stable Work binding even when the publication suggestion changes", () => {
  assert.equal(citationCandidateKey(profile, "work-a").key, "existing");
  assert.equal(citationCandidateKey({ ...profile, choices: profile.choices.map(choice => ({ ...choice, suggestedKey: "changed" })) }, "work-a").key, "existing");
  const multiple = { ...profile, editor: { ...profile.editor, bindings: [...profile.editor.bindings, { key: "alias", work: "work-a" }] } };
  assert.equal(citationCandidateKey(multiple, "work-a").key, "");
  assert.match(citationCandidateKey(multiple, "work-a").reason!, /多个绑定键/);
});
test("different Works and document labels never silently coalesce by suggested key", () => {
  const occupied = { ...profile, editor: { ...profile.editor, bindings: [{ key: "zhang2026", work: "work-a" }] } };
  assert.equal(citationCandidateKey(occupied, "work-b").key, "");
  assert.match(citationCandidateKey(occupied, "work-b").reason!, /冲突/);
  const invalid = { ...profile, choices: [{ ...profile.choices[1], suggestedKey: "张 2026" }] };
  assert.equal(citationCandidateKey(invalid, "work-b").key, "");
  assert.match(citationCandidateKey(invalid, "work-b").reason!, /ASCII/);
});
test("insertion offsets preserve UTF-8 bytes and reject a split supplementary scalar", () => {
  assert.equal(citationInsertionByte("中文😀\r\nbody", 4), 10);
  assert.throws(() => citationInsertionByte("中文😀\r\nbody", 3));
  assert.throws(() => citationInsertionByte("abc", -1));
  assert.throws(() => citationInsertionByte("abc", 4));
});
