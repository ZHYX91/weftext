import assert from "node:assert/strict";
import test from "node:test";
import { PersonalInputRecovery, personalInputText, type PersonalInputRecord } from "./personal-input-recovery";
class Storage {
  data = new Map<string, string>(); writes: { key: string; size: number }[] = []; fail = false;
  getItem(key: string) { return this.data.get(key) ?? null; }
  setItem(key: string, value: string) { if (this.fail) throw new Error("quota"); this.data.set(key, value); this.writes.push({ key, size: value.length }); }
  removeItem(key: string) { this.data.delete(key); }
}
const audience = JSON.stringify(["deployment", "alice"]), other = JSON.stringify(["deployment", "bob"]);
function record(text = "OWN"): Omit<PersonalInputRecord, "sequence"> { return { id: "test-id", owner: { kind: "body", resource: "Shared", reference: "weftext-node://w/n" }, baseHash: "a".repeat(64), origin: { baseLength: 999, pieces: [{ base: 0, length: 500 }, { text }, { base: 500, length: 499 }], detached: [], revision: 1 } }; }
test("independent actor input survives protected cache clearing without holding its Base or token", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage); recovery.update(audience, [record()]);
  storage.setItem("weftext-hosted-draft:old", "PRIVATE_BASE"); for (const key of storage.data.keys()) if (key.startsWith("weftext-hosted-draft:")) storage.removeItem(key);
  const reader = new PersonalInputRecovery(storage); assert.equal(personalInputText(reader.list(audience)[0]), "OWN"); assert.deepEqual(reader.list(other), []);
  assert.ok(![...storage.data.values()].join().includes("PRIVATE_BASE"));
});
test("unchanged large input snapshots are not serialized again for each small edit", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage), large = "本人😀".repeat(30000);
  recovery.update(audience, [record(large)]);
  for (let i = 0; i < 30; i++) recovery.update(audience, [record(large + String(i))]);
  assert.equal(storage.writes.filter(write => write.size > 65536).length, 1);
  assert.equal(personalInputText(new PersonalInputRecovery(storage).list(audience)[0]), large + "29");
});
test("quota refusal keeps exact latest pure input in memory and previous committed storage intact", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage); recovery.update(audience, [record("first")]); const old = [...storage.data]; storage.fail = true;
  assert.throws(() => recovery.update(audience, [record("second")]), /quota/); assert.equal(personalInputText(recovery.list(audience)[0]), "second"); assert.deepEqual([...storage.data], old);
});
test("corrupt previous records are preserved while new live pure input remains recoverable", () => {
  const storage = new Storage(), key = `weftext-personal-input:${encodeURIComponent(audience)}:state`; storage.setItem(key, "corrupt original");
  const recovery = new PersonalInputRecovery(storage); assert.throws(() => recovery.update(audience, [record()]), /未覆盖/);
  assert.equal(storage.getItem(key), "corrupt original"); assert.equal(personalInputText(recovery.list(audience)[0]), "OWN");
});
test("same-text ABA invalidates captured recovery actions and retains original operation identity", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage);
  recovery.update(audience, [{ ...record(), pending: { operationId: "original-op", status: "unresolved" } }]); const first = recovery.list(audience)[0];
  recovery.update(audience, [record("changed")]); recovery.update(audience, [record()]);
  assert.throws(() => recovery.current(audience, first), /变化/); assert.equal(recovery.list(audience)[0].pending?.operationId, "original-op");
  recovery.settle(audience, recovery.list(audience)[0], "committed"); assert.deepEqual(recovery.list(audience)[0].pending, { operationId: "original-op", status: "committed" });
});
test("over-budget live input can be explicitly reduced until persistence is possible again", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage);
  assert.throws(() => recovery.update(audience, Array.from({ length: 130 }, (_, index) => ({ ...record(String(index)), id: `id:${index}` }))), /预算/);
  recovery.remove(audience, recovery.list(audience)[0]); assert.equal(recovery.list(audience).length, 129); assert.match(recovery.notice(audience)!, /只从内存/);
  recovery.remove(audience, recovery.list(audience)[0]); assert.equal(recovery.list(audience).length, 128); assert.equal(recovery.notice(audience), undefined);
  assert.equal(new PersonalInputRecovery(storage).list(audience).length, 128);
});

test("original committed candidate promotes S and cannot be downgraded by a later live unresolved retain", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage), update = { ...record("X"), pending: { operationId: "one", status: "unresolved" as const }, candidate: { operationId: "one", baseHash: "b".repeat(64), origin: { baseLength: 1000, pieces: [{ base: 0, length: 1000 }, { text: "Y" }], detached: [], revision: 2 } } };
  recovery.update(audience, [update]); recovery.settle(audience, recovery.list(audience)[0], "committed"); recovery.update(audience, [update]);
  const restored = new PersonalInputRecovery(storage).list(audience)[0]; assert.equal(restored.pending?.status, "committed"); assert.equal(restored.baseHash, "b".repeat(64)); assert.equal(personalInputText(restored), "Y"); assert.equal(restored.candidate, undefined);
});
test("field and library pure payloads keep their actual owner without a current body", () => {
  const storage = new Storage(), recovery = new PersonalInputRecovery(storage); recovery.update(audience, [{ ...record("FIELD"), owner: { kind: "field", resource: "other", fieldId: "actual" } }, { ...record("STYLE"), id: "style", owner: { kind: "library", role: "style" } }]);
  assert.deepEqual(new PersonalInputRecovery(storage).list(audience).map(item => item.owner), [{kind:"field",resource:"other",fieldId:"actual"},{kind:"library",role:"style"}]);
});
