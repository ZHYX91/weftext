import test from "node:test";
import assert from "node:assert/strict";
import { NoteInputRecovery } from "./note-input-recovery";
test("new input recovery survives reopening while retaining its original account audience", () => {
  const entries = new Map<string, string>(), storage = { getItem: (key: string) => entries.get(key) ?? null, setItem: (key: string, value: string) => { entries.set(key, value); } };
  const recovery = new NoteInputRecovery(storage); recovery.add("deployment/alice", "新增甲\n新增乙😀");
  assert.equal(recovery.count("deployment/alice"), 1); assert.equal(recovery.count("deployment/bob"), 0);
  const reopened = new NoteInputRecovery(storage); assert.equal(reopened.count("deployment/alice"), 1); assert.equal(reopened.count("deployment/bob"), 0);
  assert.equal(JSON.stringify([...entries.values()]).includes("新增甲"), true);
});
test("storage rejection retains the independently recoverable live input", () => {
  const recovery = new NoteInputRecovery({ getItem: () => { throw new Error("blocked"); }, setItem: () => { throw new Error("blocked"); } });
  recovery.add("alice", "我的新输入"); assert.equal(recovery.count("alice"), 1); assert.equal(recovery.count("bob"), 0);
});
