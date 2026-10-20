import { test } from "node:test";
import assert from "node:assert/strict";
import { diagnosticGroups, issueRuleKey, PersonalRelationRules, ruleMatches, visibleCandidate, type RelationIssue } from "./relationship-diagnostics";
test("target grouping preserves every independently locatable occurrence", () => {
  const source = { key: "a", title: "A", reference: "a" };
  const issues: RelationIssue[] = [{ source, address: "missing", status: "missing", from: 1, to: 5 }, { source, address: "missing", status: "missing", from: 8, to: 12 }];
  assert.equal(diagnosticGroups(issues, "target").length, 1);
  assert.deepEqual(diagnosticGroups(issues, "target")[0][1].map(i => i.from), [1, 8]);
  assert.equal(diagnosticGroups(issues, "occurrence").length, 2);
  assert.equal(issueRuleKey(issues[0]), issueRuleKey(issues[1]));
});
test("exact personal exclusions respect source scope and retain overlapping alternatives", () => {
  const rules = [{ kind: "term" as const, value: "山", source: "a" }, { kind: "target" as const, value: "one" }];
  assert.equal(visibleCandidate(rules, "a", "山", "two"), false);
  assert.equal(visibleCandidate(rules, "b", "山", "two"), true);
  assert.equal(visibleCandidate(rules, "a", "白云山", "two"), true);
  assert.equal(visibleCandidate(rules, "a", "白云山", "one"), false);
  assert.equal(ruleMatches(rules, "expected", "one"), false);
});
test("failed preference persistence is atomic; undo restores only personal rules", () => {
  let fail = false; const saved: unknown[] = [];
  const rules = new PersonalRelationRules(value => { if (fail) throw Error("disk"); saved.push(value); });
  rules.save([{ kind: "expected", value: "node" }]); fail = true;
  assert.throws(() => rules.save([])); assert.equal(rules.rules.length, 1);
  assert.throws(() => rules.undo()); assert.equal(rules.rules.length, 1);
  fail = false; rules.undo(); assert.deepEqual(rules.rules, []); assert.equal(saved.length, 2);
});
