import test from "node:test";
import assert from "node:assert/strict";
import { decodeHostedQuery, exactHostedDependencies, hostedDependencyMap, HostedQueryGate } from "./hosted-query-state";

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
test("Provider execution selects exact fingerprints without following a current descriptor or expanding a policy's dependencies", () => {
  const temporal = "a".repeat(64), calendar = "b".repeat(64), policy = "c".repeat(64), catalog = { providers: [{ id: "temporal/rules", fingerprint: temporal }, { id: "calendars/private", fingerprint: calendar }], seriesPolicies: [{ id: "series/private", fingerprint: policy }] };
  assert.deepEqual(exactHostedDependencies({ providers: {}, seriesPolicies: {} }, catalog), { providers: {}, seriesPolicies: {} });
  const input = { providers: { "temporal/rules": temporal }, seriesPolicies: { "series/private": policy } }, frozen = exactHostedDependencies(input, catalog);
  assert.deepEqual(Object.keys(frozen.providers), ["temporal/rules"]); frozen.providers["temporal/rules"] = calendar; assert.equal(input.providers["temporal/rules"], temporal);
  assert.throws(() => exactHostedDependencies(input, { ...catalog, providers: [{ id: "temporal/rules", fingerprint: calendar }] }), /改版/);
  assert.throws(() => exactHostedDependencies(input, { ...catalog, seriesPolicies: [] }), /series\/private/);
  assert.deepEqual(input.providers, { "temporal/rules": temporal });
});
test("invalid Provider cache is retained as unreadable rather than silently choosing a current version", () => {
  for (const value of [null, [], "id", { id: "latest" }, { id: "A".repeat(64) }, { id: null }]) assert.throws(() => hostedDependencyMap(value));
  assert.deepEqual(hostedDependencyMap(undefined), {}); const raw = '{"__proto__":"' + "a".repeat(64) + '"}'; const cloned = hostedDependencyMap(JSON.parse(raw)); assert.equal(Object.getPrototypeOf(cloned), Object.prototype); assert.deepEqual(Object.keys(cloned), ["__proto__"]);
});
test("Provider selection, version ABA and clear invalidate an otherwise unchanged native plan and parameters", () => {
  const plan = '{"select":[]}', parameters = '{"day":{"kind":"date","value":"2026-10-14"}}', selected = { providers: { "temporal/rules": "a".repeat(64) }, seriesPolicies: {} }, fingerprint = () => JSON.stringify({ plan, parameters, ...selected }), gate = new HostedQueryGate();
  const first = gate.begin(fingerprint()); selected.providers["temporal/rules"] = "b".repeat(64); gate.changed(); selected.providers["temporal/rules"] = "a".repeat(64); gate.changed(); assert.equal(gate.current(first, fingerprint()), false);
  const resumed = gate.begin(fingerprint()); assert.equal(gate.current(resumed, fingerprint()), true); gate.clear(); assert.equal(gate.current(resumed, fingerprint()), false); assert.equal(JSON.parse(parameters).day.value, "2026-10-14");
});
