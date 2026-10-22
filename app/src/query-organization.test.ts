import { uiTextValue } from "./ui-locale";
import test from "node:test";
import assert from "node:assert/strict";
import { addPlanFilter, queryPlanFields } from "./query-plan-controls";
import { directQueryField, type QueryPlan, type QueryRow } from "./query-panel";
import { occurrenceBuckets, decodeOccurrenceView, occurrenceProjection } from "./query-result-view";
import { validateQueryArrangement, type EmbeddedQueryOccurrence } from "./embedded-query";
import { utf8Offset } from "./writing";
import type { DataSchema, Cell } from "./data-panel";
const schema: DataSchema = { definitions: {}, types: [], aliases: {}, fields: [{ id: "money/amount", key: "amount", shape: { kind: "scalar", semantic: "decimal" }, nullable: true }, { id: "tasks/project", key: "project", shape: { kind: "scalar", semantic: "node_ref" }, nullable: false }] };
const plan = (): QueryPlan => ({ scope: { descendants: true, lifecycle: "live" }, types: [], where: 'phones.exists(p, p.number == "重复" && p.label == "工作")', select: [{ name: "amount", expr: 'field("money/amount")' }], order: [] });

test("visible Query conditions preserve native decimal precision, reference type and existing same-item CEL", () => {
  const original = plan(), fields = queryPlanFields(schema, original);
  const next = addPlanFilter(original, fields.find(field => field.key === "amount")!, ">", { kind: "decimal", value: "9007199254740993.0000000001" }, schema);
  assert.equal(next.where, '(' + original.where + ') && (field("money/amount") > decimal("9007199254740993.0000000001"))'); assert.equal(original.where, plan().where);
  const ref = addPlanFilter(original, fields.find(field => field.key === "project")!, "==", { kind: "text", value: "weftext-node://owner/target" }, schema); assert.ok(ref.where!.includes('nodeRef("weftext-node://owner/target")'));
  assert.throws(() => addPlanFilter(original, fields[0], "bad", { kind: "text", value: "x" }, schema));
});
test("row writers require a direct permanent field, and aggregate, computed and producer rows remain read-only", () => {
  assert.equal(directQueryField(plan(), schema, "amount")?.id, "money/amount");
  const computed = plan(); computed.select[0].expr = 'field("money/amount") + decimal("1")'; assert.equal(directQueryField(computed, schema, "amount"), undefined);
  const grouped = plan(); grouped.group = { keys: [], aggregates: [{ name: "count", op: "count" }] }; assert.equal(directQueryField(grouped, schema, "amount"), undefined);
  const replaced = structuredClone(schema); replaced.fields[0].id = "money/new-amount"; assert.equal(directQueryField(plan(), replaced, "amount"), undefined);
});
test("personal board and calendar keep every row and distinguish absent, null, invalid and unprojected values", () => {
  const cells: (Cell | undefined)[] = [{ state: "absent" }, { state: "null" }, { state: "invalid",value:null,diagnostics:[] }, undefined, { state: "value", value: { kind: "date", value: "2026-10-19" } }, { state: "value", value: { kind: "text", value: "2026-10-19" } }];
  const rows: QueryRow[] = cells.map((cell, i): QueryRow => ({ key: String(i), cells: cell ? { due: cell } : {}, readOnly: true }));
  assert.equal(occurrenceBuckets(rows, "due", false).length, 6);
  const calendar = occurrenceBuckets(rows, "due", true); assert.equal(calendar.find(group => group.key === "2026-10-19")!.rows.length, 1); assert.equal(calendar.reduce((count, group) => count + group.rows.length, 0), rows.length);
  assert.match(uiTextValue(decodeOccurrenceView("broken").warning!), /无效/);
});
test("Query arrangement accepts only the exact adjacent AST evidence and rejects an outside-source mutation", () => {
  const source = "前😀\r\n甲查询\r\n\r\n乙查询\n后";
  const occurrence = (raw: string, key: string): EmbeddedQueryOccurrence => { const from = source.indexOf(raw), to = from + raw.length; return { id: key, key, label: key, kind: "definition", callFrom: utf8Offset(source, from), writable: true, builderSupported: true, runnable: true, range: { from: utf8Offset(source, from), to: utf8Offset(source, to), utf16From: from, utf16To: to, raw, writable: true } }; };
  const one = occurrence("甲查询", "one"), two = occurrence("乙查询", "two"), basis = { sessionId: "s", source, serial: 2 }, text = "乙查询\r\n\r\n甲查询", changed = source.slice(0, one.range.utf16From) + text + source.slice(two.range.utf16To), result = { source: changed, serial: 2, patches: [{ from: one.range.utf16From, to: two.range.utf16To, text }] };
  assert.equal(validateQueryArrangement(basis, two, [one, two], "up", result).length, 1);
  assert.throws(() => validateQueryArrangement(basis, two, [one, two], "up", { ...result, source: changed + "x" }));
  assert.throws(() => validateQueryArrangement(basis, one, [one, two], "up", result));
});

test("personal columns cannot silently bind the same output name after schema replacement", () => {
  const plan: QueryPlan = { scope: { descendants: true, lifecycle: "live" }, types: [], select: [{ name: "status", expr: "status" }], order: [] };
  const retained = occurrenceProjection(plan, "original-field-domain");
  assert.equal(occurrenceProjection(structuredClone(plan), "original-field-domain"), retained);
  assert.notEqual(occurrenceProjection(plan, "replacement-field-domain"), retained);
  assert.notEqual(occurrenceProjection({ ...plan, select: [{ name: "status", expr: "title" }] }, "original-field-domain"), retained);
});
