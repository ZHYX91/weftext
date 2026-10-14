import test from "node:test";
import assert from "node:assert/strict";
import { decodeHostedQuery, HostedQueryGate } from "./hosted-query-state";

test("frozen query preserves zero-row column order and keeps partial/stale/diagnostics distinct", () => {
  const input = { queryId: "original", columns: ["second", "first"], result: { rows: [], coverage: "partial" as const, freshness: "stale" as const, diagnostics: [{ message: "dependency unavailable" }], totalKnown: 0, limited: false } };
  assert.deepEqual(decodeHostedQuery(input).columns, ["second", "first"]);
  assert.equal(decodeHostedQuery(input).coverage, "partial"); assert.equal(decodeHostedQuery(input).freshness, "stale");
  assert.throws(() => decodeHostedQuery({ ...input, columns: ["first", "first"] }));
  assert.throws(() => decodeHostedQuery({ ...input, result: { ...input.result, queryId: "replacement" } }));
  assert.throws(() => decodeHostedQuery({ ...input, result: { ...input.result, columns: ["first", "second"] } }));
});
test("query row lineage cannot fabricate navigation or an undeclared output column", () => {
  const input = { queryId: "original", columns: ["status"], result: { rows: [{ key: "row", readOnly: true, canOpen: true, cells: { status: { state: "value" as const, value: { kind: "text" as const, value: "open" } } } }], coverage: "complete" as const, freshness: "current" as const, diagnostics: [], totalKnown: 1, limited: false } };
  assert.throws(() => decodeHostedQuery(input));
  assert.equal(decodeHostedQuery({ ...input, result: { ...input.result, rows: [{ ...input.result.rows[0], resource: "authorized-target" }] } }).rows[0].resource, "authorized-target");
  assert.throws(() => decodeHostedQuery({ ...input, columns: [], result: { ...input.result, rows: [{ ...input.result.rows[0], canOpen: false }] } }));
});
test("late plans cannot restore results after edited parameters, ABA input or actor clear", () => {
  const gate = new HostedQueryGate(), original = gate.begin("Q1"); assert.equal(gate.current(original, "Q1"), true);
  gate.changed(); assert.equal(gate.current(original, "Q1"), false);
  const next = gate.begin("Q2"); assert.equal(gate.current(next, "Q1"), false); gate.changed(); gate.changed(); assert.equal(gate.current(next, "Q2"), false);
  const current = gate.begin("Q1"); gate.clear(); assert.equal(gate.current(current, "Q1"), false);
});
