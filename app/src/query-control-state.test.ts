import test from "node:test";
import assert from "node:assert/strict";
import { retainQueryChoice } from "./query-control-state";

test("schema choices retain a removed original field and never select its same-name successor", () => {
  const replacement = [{ value: "tasks/new-status", label: "状态" }];
  const retained = retainQueryChoice(replacement, "tasks/original-status", "状态");
  assert.equal(retained.unavailable, true); assert.deepEqual(retained.choices, [{ value: "tasks/original-status", label: "状态（当前不可用）", disabled: true }, ...replacement]);
  assert.deepEqual(replacement, [{ value: "tasks/new-status", label: "状态" }]);
  assert.equal(retainQueryChoice(replacement, "tasks/new-status", "状态").unavailable, false);
});

test("returning original identities recover their own choice and repeated invalidation does not grow labels", () => {
  const first = retainQueryChoice([], "field-id", "原字段"), second = retainQueryChoice([], "field-id", first.choices[0].label);
  assert.equal(second.choices[0].label, first.choices[0].label);
  const choices = [{ value: "field-id", label: "改名后的原字段" }]; assert.deepEqual(retainQueryChoice(choices, "field-id", first.choices[0].label), { choices, unavailable: false });
  assert.deepEqual(retainQueryChoice([], ""), { choices: [], unavailable: false });
});
