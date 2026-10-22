import { uiTextValue } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { decodeProjection, decodeSourcePatch, fieldsForTypes, moveOrderedValue, replaceNativeMember, scalarFromInput, switchUnionValue, typeConflicts, unionVariant, type DataSchema, type NativeValue, type Shape } from "./data-panel";
import { calendarDay, composeWhere, decodeViewPreference, defaultPreference, distinctFacetCounts, facetPredicate, hasDirectStatusProjection, parseQueryJson, parseQueryParameters, parseQueryPlan, pinEligibleRows, preferenceFieldErrors, queryResultState, queryRowLifecycle, scalarQueryLiteral, shapedQueryLiteral, viewPreferenceIdentity, type QueryPlan, type QueryResult, type QueryRow } from "./query-panel";

const textShape: Shape = { kind: "scalar", semantic: "text" };
const schema: DataSchema = {
  definitions: {},
  fields: [{ id: "tasks/status", key: "status", shape: { kind: "enum", values: ["open", "done", "cancelled"], open: false }, nullable: false }, { id: "core/tags", key: "tags", shape: { kind: "list", items: textShape, min: 0, max: null }, nullable: false }],
  types: [{ id: "base", fields: [{ id: "core/tags", required: false }], requires: [], conflicts: [] }, { id: "tasks/task", fields: [{ id: "tasks/status", required: true }], requires: ["base"], conflicts: ["event"] }, { id: "event", fields: [{ id: "tasks/status", required: false }], requires: [], conflicts: ["tasks/task"] }],
  aliases: { task: "tasks/task" },
};
const text = (value: string): NativeValue => ({ kind: "text", value });
const row = (key: string, tags: string[] = []): QueryRow => ({ key, entityKey: key, path: `${key}.typ`, cells: { tags: { state: "value", value: { kind: "array", value: tags.map(text) } } }, readOnly: false });

test("a type alias expands requirements and shared required fields once", () => {
  const fields = fieldsForTypes(schema, ["task", "tasks/task", "event", "unknown"]);
  assert.equal(fields.length, 2);
  assert.equal(fields.find(item => item.field.key === "status")?.required, true);
  assert.ok(typeConflicts(schema, ["task", "event"]).length > 0);
});

test("ordered repeated phone records retain labels and notes when moved", () => {
  const value: NativeValue = { kind: "array", value: [{ kind: "record", value: [{ key: "number", value: text("0086 138") }, { key: "label", value: text("工作") }, { key: "note", value: text("第一项") }] }, { kind: "record", value: [{ key: "number", value: text("0086 138") }, { key: "label", value: text("家庭") }, { key: "note", value: text("第二项") }] }] };
  const moved = moveOrderedValue(value, 0, 1);
  assert.equal(moved.kind, "array");
  if (moved.kind !== "array") return;
  assert.deepEqual(moved.value, [value.value[1], value.value[0]]);
  assert.equal(value.value.length, 2);
});

test("record editing preserves unknown keys and original member order", () => {
  const original: NativeValue = { kind: "record", value: [{ key: "number", value: text("00123") }, { key: "unknown-imported", value: { kind: "record", value: [{ key: "original", value: text("keep") }] } }, { key: "label", value: text("工作") }] };
  const updated = replaceNativeMember(original, "label", text("家庭"));
  assert.equal(updated.kind, "record");
  if (updated.kind !== "record") return;
  assert.deepEqual(updated.value.map(item => item.key), ["number", "unknown-imported", "label"]);
  assert.deepEqual(updated.value[1], original.value[1]);
  assert.equal((original.value[2].value as { value: string }).value, "工作");
});

test("exact numbers and phones never pass through Number or trim", () => {
  assert.deepEqual(scalarFromInput({ kind: "scalar", semantic: "int" }, schema, "9223372036854775807"), { kind: "int", value: "9223372036854775807" });
  assert.deepEqual(scalarFromInput({ kind: "scalar", semantic: "decimal" }, schema, "0.30"), { kind: "decimal", value: "0.30" });
  assert.deepEqual(scalarFromInput({ kind: "scalar", semantic: "float" }, schema, "-0.0"), { kind: "float", value: "-0.0" });
  assert.deepEqual(scalarFromInput({ kind: "scalar", semantic: "phone" }, schema, " 00123 "), text(" 00123 "));
  assert.throws(() => scalarFromInput({ kind: "scalar", semantic: "int" }, schema, "1.1"));
});

test("a union refuses two simultaneous target branches", () => {
  const shape: Extract<Shape, { kind: "union" }> = { kind: "union", variants: [{ kind: "record", members: { ref: { shape: textShape, required: true, nullable: false } }, unknown: "preserve" }, { kind: "record", members: { text: { shape: textShape, required: true, nullable: false } }, unknown: "preserve" }], discriminator: { kind: "member", keys: ["ref", "text"] } };
  assert.equal(unionVariant(shape, { kind: "record", value: [{ key: "ref", value: text("node") }] }, schema), 0);
  assert.equal(unionVariant(shape, { kind: "record", value: [{ key: "ref", value: text("node") }, { key: "text", value: text("same name") }] }, schema), -1);
  const withUnknown: NativeValue = { kind: "record", value: [{ key: "ref", value: text("node") }, { key: "imported-extra", value: text("must keep") }] };
  const switched = switchUnionValue(shape, withUnknown, 1, schema);
  assert.equal(switched.kind, "record");
  if (switched.kind === "record") { assert.ok(switched.value.some(item => item.key === "imported-extra")); assert.ok(!switched.value.some(item => item.key === "ref")); }
  assert.throws(() => switchUnionValue({ ...shape, variants: [shape.variants[0], textShape] }, withUnknown, 1, schema));
});

test("minimal patch joining preserves Chinese, emoji and untouched source bytes", () => {
  const source = "中文😀 a /* 原注释 */ b 后文";
  const fromA = source.indexOf("a"); const fromB = source.indexOf("b"); const after = source.replace("a", "甲").replace("b", "乙");
  const patch = decodeSourcePatch({ source: after, patches: [{ from: fromB, to: fromB + 1, text: "乙" }, { from: fromA, to: fromA + 1, text: "甲" }] }, source);
  assert.equal(source.slice(0, patch.from) + patch.text + source.slice(patch.to), after);
  assert.ok(patch.text.includes("/* 原注释 */"));
  assert.throws(() => decodeSourcePatch({ source: after, patches: [{ from: 11, to: 12, text: "甲" }] }, source));
  assert.throws(() => decodeSourcePatch({ patches: [{ from: fromA, to: fromB + 1, text: "x" }, { from: fromB, to: fromB + 1, text: "y" }] }, source));
});

test("projection adapter unwraps only the real projection", () => {
  const projection = { qualification: "qualified", id:null, types: [], resolvedTypes:[], fields: [], unknownFields: [], diagnostics: [],budgetExhausted:false };
  assert.deepEqual(decodeProjection({ projection }), projection);
  assert.equal(decodeProjection({ projection, readOnly: true }).readOnly, true);
});

test("typed reference facets keep reference and literal tag branches distinct", () => {
  const tagShape: Shape = { kind: "union", variants: [{ kind: "record", members: { ref: { shape: { kind: "scalar", semantic: "tag_ref" }, required: true, nullable: false } }, unknown: "preserve" }, { kind: "record", members: { text: { shape: textShape, required: true, nullable: false } }, unknown: "preserve" }], discriminator: { kind: "member", keys: ["ref", "text"] } };
  const typedSchema = { ...schema, fields: [{ ...schema.fields[1], shape: { kind: "list", items: tagShape, min: 0, max: null } as Shape }] };
  const target = "weftext-tag://workspace/tag";
  const ref = facetPredicate({ fieldId: "core/tags", key: "tags", count: 2, value: { kind: "record", value: [{ key: "ref", value: text(target) }] } }, typedSchema);
  const literal = facetPredicate({ fieldId: "core/tags", key: "tags", count: 1, value: { kind: "record", value: [{ key: "text", value: text(target) }] } }, typedSchema);
  assert.equal(ref, `field("core/tags").exists(item, has(item.ref) && item.ref == tagRef("${target}"))`);
  assert.equal(literal, `field("core/tags").exists(item, has(item.text) && item.text == "${target}")`);
  assert.equal(shapedQueryLiteral({ kind: "scalar", semantic: "exact_decimal" }, schema, text("0.30")), 'exactDecimal("0.30")');
  assert.equal(shapedQueryLiteral(textShape, schema, text(target)), JSON.stringify(target));
});

test("query view identities separate scopes, tag conditions and definition versions", () => {
  const identity = viewPreferenceIdentity("workspace", "stable-dir-a", ["task"], ["tagRef(a)"]);
  assert.notEqual(identity, viewPreferenceIdentity("workspace", "stable-dir-b", ["task"], ["tagRef(a)"]));
  assert.notEqual(identity, viewPreferenceIdentity("workspace", "stable-dir-a", ["task"], ["literal-tag(a)"]));
  assert.equal(identity, viewPreferenceIdentity("workspace", "stable-dir-a", ["task", "task"], ["tagRef(a)", "tagRef(a)"]));
  const saved = { owner: "node-owner", key: "definition", mode: "pinned", revision: "old" };
  assert.notEqual(viewPreferenceIdentity("workspace", "", [], [], saved), viewPreferenceIdentity("workspace", "", [], [], { ...saved, revision: "new" }));
  assert.notEqual(viewPreferenceIdentity("workspace", "", [], [], saved), viewPreferenceIdentity("workspace", "", [], [], { ...saved, mode: "current" }));
});

test("only direct fields offer actions and composite lifecycle is never guessed", () => {
  const plan: QueryPlan = { scope: { descendants: true, lifecycle: "live" }, types: ["task"], select: [{ name: "status", expr: 'field("tasks/status")' }], order: [] };
  assert.equal(hasDirectStatusProjection(plan, schema), true);
  assert.equal(hasDirectStatusProjection({ ...plan, select: [{ name: "status", expr: '"done"' }] }, schema), false);
  assert.equal(hasDirectStatusProjection({ ...plan, unnest: { array: "tags", binding: "item" } }, schema), false);
  const combined = { ...plan, advanced: { unionAll: [plan, { ...plan, scope: { descendants: true, lifecycle: "trash" as const } }] } };
  assert.equal(hasDirectStatusProjection(combined, schema), false);
  assert.equal(queryRowLifecycle(row("note"), combined), null);
  assert.equal(queryRowLifecycle({ ...row("note"), lifecycle: "trash" }, combined), "trash");
});

test("advanced plans preserve projection, sorting, producers and exact invocation parameters", () => {
  const plan: QueryPlan = { scope: { directory: "weftext-directory://workspace/id", descendants: false, lifecycle: "live" }, types: [], select: [{ name: "phone", expr: "item.number" }], order: [{ expr: "item.label", descending: true, nulls: "first" }, { expr: "title", descending: false, nulls: "last" }], unnest: { array: 'field("people/phones")', binding: "item" }, advanced: { parameters: [{ name: "amount", shape: { kind: "scalar", semantic: "decimal" }, default: { kind: "decimal", value: "0.30" } }] } };
  assert.deepEqual(parseQueryPlan(JSON.stringify(plan)), plan);
  const composite = { ...plan, advanced: { unionAll: [plan], distinct: [{ name: "phone", expr: "phone" }] } };
  assert.deepEqual(parseQueryPlan(JSON.stringify(composite)), composite);
  const parameters = parseQueryParameters('{"amount":{"kind":"decimal","value":"0.30"},"max":{"kind":"int","value":"9223372036854775807"},"explicitNull":{"kind":"none"}}');
  assert.deepEqual(parameters.amount, { kind: "decimal", value: "0.30" });
  assert.deepEqual(parameters.max, { kind: "int", value: "9223372036854775807" });
  assert.equal("omitted" in parameters, false);
  assert.throws(() => parseQueryParameters('{"amount":{"kind":"decimal","value":0.30}}'));
  assert.throws(() => parseQueryJson('{"x":1,"\\u0078":2}'));
  assert.throws(() => parseQueryPlan('{"scope":{"lifecycle":"archive"}}'));
  assert.throws(() => parseQueryPlan('{"scope":{},"unionAll":[]}'));
});

test("pinning partitions only current eligible rows and preserves query order", () => {
  const rows = [row("a"), row("b"), row("c")];
  assert.deepEqual(pinEligibleRows(rows, ["missing", "c", "a"]).map(item => item.key), ["a", "c", "b"]);
  assert.deepEqual(rows.map(item => item.key), ["a", "b", "c"]);
});

test("a removed view field never rebinds to a different same-key field", () => {
  const preference = { ...defaultPreference(), columns: ["status"], fieldBindings: { status: "tasks/status" } };
  assert.deepEqual(preferenceFieldErrors(preference, schema), []);
  const replaced = { ...schema, fields: [{ ...schema.fields[0], id: "another/status" }] };
  assert.equal(preferenceFieldErrors(preference, replaced).length, 1);
});

test("malformed local display preferences reset without changing author semantics", () => {
  const decoded = decodeViewPreference({ view: "unknown", density: 4, sort: null, descending: "false", columns: ["status", 9], pinned: { note: true }, fieldBindings: { status: "tasks/status", wrong: 7 } });
  assert.equal(decoded.view, defaultPreference().view);
  assert.equal(decoded.density, defaultPreference().density);
  assert.equal(decoded.sort, defaultPreference().sort);
  assert.equal(decoded.descending, defaultPreference().descending);
  assert.deepEqual(decoded.columns, ["status"]);
  assert.deepEqual(decoded.pinned, []);
  assert.deepEqual(decoded.fieldBindings, { status: "tasks/status" });
});

test("facet counts are distinct notes for each value with missing separate", () => {
  const rows = [row("a", ["重复", "重复", "另一个"]), row("a", ["重复"]), row("b", ["重复"]), { ...row("c"), cells: { tags: { state: "absent" as const } } }];
  const facets = distinctFacetCounts(rows, "tags");
  assert.equal(facets.find(item => item.value?.kind === "text" && item.value.value === "重复")?.count, 2);
  assert.equal(facets.find(item => item.missing)?.count, 1);
  assert.equal(facets.find(item => item.value?.kind === "text" && item.value.value === "另一个")?.count, 1);
  const nulls = distinctFacetCounts([{ ...row("null"), cells: { tags: { state: "null" } } }, { ...row("absent"), cells: { tags: { state: "absent" } } }], "tags");
  assert.equal(nulls.find(item => item.value?.kind === "none")?.count, 1);
  assert.equal(nulls.find(item => item.missing)?.count, 1);
  assert.deepEqual(distinctFacetCounts([{ ...row("aggregate", ["x"]), entityKey: undefined, readOnly: true }], "tags"), []);
});

test("civil calendar grouping never coerces instant or partial dates", () => {
  assert.equal(calendarDay({ kind: "date", value: "2026-10-13" }), "2026-10-13");
  assert.equal(calendarDay({ kind: "localDatetime", value: "2026-10-13T00:00:00" }), null);
  assert.equal(calendarDay(text("2026-10-12T16:00:00Z")), null);
  assert.equal(calendarDay({ kind: "record", value: [{ key: "year", value: { kind: "int", value: "2026" } }] }), null);
});

test("partial, stale, complete empty and limited output have distinct states", () => {
  const complete: QueryResult = { rows: [], coverage: "complete", freshness: "current", diagnostics: [], totalKnown: 0, limited: false };
  assert.equal(uiTextValue(queryResultState(complete)), "完整空结果");
  assert.match(uiTextValue(queryResultState({ ...complete, coverage: "partial" })), /部分/);
  assert.match(uiTextValue(queryResultState({ ...complete, freshness: "stale" })), /过期/);
  assert.match(uiTextValue(queryResultState({ ...complete, limited: true, totalKnown: 22 })), /显示 0/);
  assert.match(uiTextValue(queryResultState({ ...complete, snapshotMode: "admittedFrozen", freshness: "stale" })), /快照内完整空结果/);
  assert.match(uiTextValue(queryResultState({ ...complete, snapshotMode: "admittedFrozen", freshness: "stale" })), /当前性未持续确认/);
  assert.match(uiTextValue(queryResultState({ ...complete, snapshotMode: "admittedFrozen", superseded: true })), /先前读取/);
});

test("search and visual filters compose safely without altering typed date conditions", () => {
  const date = { id: "date", label: "截止日期", expr: 'field("tasks/due") == date("2026-10-13")' };
  const where = composeWhere('引号"与中文', true, [date], 'phones.exists(p, p.number == "138" && p.label == "工作")');
  assert.ok(where?.includes('title.contains("引号\\\"与中文")'));
  assert.ok(where?.includes(date.expr));
  assert.ok(where?.includes("phones.exists"));
  assert.equal(scalarQueryLiteral({ kind: "decimal", value: "0.30" }), 'decimal("0.30")');
  assert.throws(() => scalarQueryLiteral({ kind: "array", value: [text("x")] }));
});
