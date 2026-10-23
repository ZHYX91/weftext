import test from "node:test";
import assert from "node:assert/strict";
import { applyHostedScope, decodeHostedQuery, exactHostedDependencies, exactHostedScope, hostedDependencyMap, hostedScopeDraft, HostedQueryGate } from "./hosted-query-state";
import { queryDataResult } from "./query-result.test-support";
import { message } from "./ui-locale";

test("frozen query preserves zero-row column order and keeps partial/stale/diagnostics distinct", () => {
  const input = { scope:"authorizedProjection" as const,readOnly:true as const, queryId: "original", columns: ["second", "first"], result: queryDataResult({ coverage: "partial", freshness: "stale", diagnostics: [{explanation:message("common.unavailable"),diagnostics:[{origin:"dependency",text:"dependency unavailable"}],causes:[]}] }) };
  assert.deepEqual(decodeHostedQuery(input).columns, ["second", "first"]);
  assert.equal(decodeHostedQuery(input).coverage, "partial"); assert.equal(decodeHostedQuery(input).freshness, "stale");
  assert.throws(() => decodeHostedQuery({ ...input, columns: ["first", "first"] }));
  assert.throws(() => decodeHostedQuery({ ...input, result: { ...input.result, queryId: "replacement" } }));
  assert.throws(() => decodeHostedQuery({ ...input, result: { ...input.result, columns: ["first", "second"] } }));
});
test("explicit top-level scope application preserves every nested producer and refuses incomplete JSON without replacing it", () => {
  const nested = { scope: { directory: "old-native-ref", descendants: false, lifecycle: "trash" }, select: [{ name: "value", expr: "status" }] }, original = { select: [{ name: "status", expr: "status" }], advanced: { related: { relation: "parents", from: { advanced: { unionAll: [nested, nested] } } } } };
  const next = JSON.parse(applyHostedScope(JSON.stringify(original), { directory: "weftext-dir://workspace/directory", descendants: true, lifecycle: "live" })); assert.deepEqual(next.advanced, original.advanced); assert.deepEqual(next.select, original.select); assert.equal(next.types, undefined); assert.equal(next.scope.directory, "weftext-dir://workspace/directory");
  assert.equal(JSON.parse(applyHostedScope(JSON.stringify(next), { descendants: false, lifecycle: "live" })).scope.directory, undefined);
  for (const input of ['{ 尚未完成😀', '{"scope":{"lifecycle":"trash"},"select":']) assert.throws(() => applyHostedScope(input, { descendants: true, lifecycle: "live" }));
});
test("terminal calls and union producers keep default outer scope; actual nested branch scopes are never guessed or rewritten", () => {
  for (const advanced of [{ call: { owner: "owner", selection: { mode: "current", key: "key" } } }, { unionAll: [{ scope: { directory: "branch", descendants: false, lifecycle: "trash" } }] }]) {
    const source = JSON.stringify({ advanced }); for (const scope of [{ directory: "other", descendants: true, lifecycle: "live" as const }, { descendants: false, lifecycle: "live" as const }, { descendants: true, lifecycle: "trash" as const }]) assert.throws(() => applyHostedScope(source, scope), /实际扫描分支/);
    assert.deepEqual(JSON.parse(applyHostedScope(source, { descendants: true, lifecycle: "live" })).advanced, advanced);
  }
});
test("directory and Trash scopes require their independent exact catalog eligibility and never follow a same-id replacement", () => {
  const directory = { id: "area", title: "获权目录", reference: "native-old-ref", kind: "directory" as const, status: "available" as const }, catalog = { directories: [directory], trash: false };
  exactHostedScope({ directory: "native-old-ref", descendants: true, lifecycle: "live" }, catalog); exactHostedScope({ descendants: true, lifecycle: "live" }, {});
  assert.throws(() => exactHostedScope({ directory: "native-old-ref", descendants: true, lifecycle: "live" }, { ...catalog, directories: [{ ...directory, reference: "native-new-ref" }] }));
  assert.throws(() => exactHostedScope({ directory: "area", descendants: true, lifecycle: "live" }, { ...catalog, directories: [{ ...directory, status: "unavailable" }] }));
  assert.throws(() => exactHostedScope({ descendants: true, lifecycle: "trash" }, catalog)); assert.throws(() => exactHostedScope({ directory: "area", descendants: true, lifecycle: "trash" }, { ...catalog, trash: true }));
  exactHostedScope({ directory: "area", descendants: true, lifecycle: "trash" }, { trash: true, directories: [{ ...directory, lifecycle: "trash" }] });
  for (const invalid of [null, [], { descendants: true }, { descendants: true, lifecycle: "live", hidden: "path" }]) assert.throws(() => hostedScopeDraft(invalid));
});
test("query row lineage cannot fabricate navigation or an undeclared output column", () => {
  const input = { scope:"authorizedProjection" as const,readOnly:true as const, queryId: "original", columns: ["status"], result: queryDataResult({rows: [{ key: "row", entityKey:"entity",path:null,revision:null, readOnly: true, canOpen: true, cells: { status: { state: "value" as const, value: { kind: "text" as const, value: "open" } } } }],totalKnown:1}) };
  assert.throws(() => decodeHostedQuery(input));
  assert.equal(decodeHostedQuery({ ...input, result: { ...input.result, rows: [{ ...input.result.rows[0], resource: "authorized-target" }] } }).rows[0].resource, "authorized-target");
  assert.throws(() => decodeHostedQuery({ ...input, columns: [], result: { ...input.result, rows: [{ ...input.result.rows[0],resource:"authorized-target", canOpen: false }] } }));
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
