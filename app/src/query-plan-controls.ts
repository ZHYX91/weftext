/** Visible controls edit the existing native QueryPlan; unknown CEL stays intact. */
import { action, fieldNames, resolveShape, typeNames, ui, valueEditor, emptyNativeValue, type DataSchema, type NativeValue, type Shape, type FormReferences } from "./data-panel";
import { shapedQueryLiteral, type QueryPlan } from "./query-panel";

export interface PlanField { key: string; expr: string; label: string; shape: Shape }
export function queryPlanFields(schema: DataSchema | null, plan: QueryPlan): PlanField[] {
  const fields: PlanField[] = [{ key: "title", expr: "title", label: "标题", shape: { kind: "scalar", semantic: "text" } }];
  for (const field of schema?.fields ?? []) fields.push({ key: field.key, expr: `field(${JSON.stringify(field.id)})`, label: fieldNames[field.key] ?? field.key, shape: field.shape });
  for (const column of plan.select) if (!fields.some(field => field.expr === column.expr)) fields.push({ key: column.name, expr: column.expr, label: fieldNames[column.name] ?? column.name, shape: { kind: "scalar", semantic: "text" } });
  return fields;
}
export function addPlanFilter(plan: QueryPlan, field: PlanField, op: string, value: NativeValue, schema: DataSchema): QueryPlan {
  const expr = field.expr;
  if (!["==", "!=", "<", "<=", ">", ">=", "contains", "absent", "null"].includes(op)) throw new Error("不支持的筛选操作");
  const condition = op === "absent" ? `isAbsent(${expr})` : op === "null" ? `isNull(${expr})` : op === "contains" ? (() => { if (value.kind !== "text") throw new Error("包含仅支持文字值"); return `${expr}.contains(${JSON.stringify(value.value)})`; })() : `${expr} ${op} ${shapedQueryLiteral(field.shape, schema, value)}`;
  return { ...structuredClone(plan), where: plan.where?.trim() ? `(${plan.where}) && (${condition})` : condition };
}
const emptySchema = (): DataSchema => ({ definitions: {}, fields: [], types: [], aliases: {} });
function select(label: string, options: [string, string][], value: string): HTMLSelectElement { const input = ui("select", "data-input"); input.setAttribute("aria-label", label); for (const [key, name] of options) { const option = ui("option", "", name); option.value = key; input.append(option); } input.value = value; return input; }
function labeled(label: string, input: HTMLElement): HTMLLabelElement { const wrapper = ui("label", "data-label", label); wrapper.append(input); return wrapper; }

/** This proposal is owned by its caller, and applies only through its existing Draft writer. */
export function queryPlanControls(plan: QueryPlan, schema: DataSchema | null, changed: () => void, references?: FormReferences): HTMLElement {
  const host = ui("section", "query-plan-controls"), actualSchema = schema ?? emptySchema(), fields = queryPlanFields(schema, plan);
  const error = ui("p", "data-warning"); error.setAttribute("role", "status");
  const safely = (run: () => void) => { try { run(); error.textContent = ""; } catch (e) { error.textContent = String(e instanceof Error ? e.message : e); } };
  if (schema?.types.length) {
    const types = ui("fieldset"); types.append(ui("legend", "", "笔记类型"));
    for (const type of schema.types) { const alias = Object.entries(schema.aliases).find(([, id]) => id === type.id)?.[0] ?? type.id, input = ui("input"); input.type = "checkbox"; input.checked = plan.types.some(token => token === alias || token === type.id); input.setAttribute("aria-label", `查询类型：${typeNames[alias] ?? alias}`); input.onchange = () => { plan.types = plan.types.filter(token => token !== alias && token !== type.id); if (input.checked) plan.types.push(alias); changed(); }; types.append(labeled(typeNames[alias] ?? alias, input)); }
    host.append(types);
  }
  const chips = ui("div", "query-condition-chips"), filterHost = ui("section");
  const drawConditions = () => { chips.replaceChildren(); if (plan.where) chips.append(ui("code", "", plan.where), action("移除现有条件", () => { delete plan.where; changed(); drawConditions(); })); else chips.append(ui("p", "data-help", "未设置筛选条件")); }; drawConditions();
  const field = select("筛选属性", fields.map((field, i) => [String(i), field.label]), "0"), operator = select("筛选运算", [["==", "等于"], ["!=", "不等于"], ["contains", "包含文字"], ["<", "小于"], [">", "大于"], ["absent", "未填写"], ["null", "明确为空"]], "=="), valueHost = ui("div"); let value: NativeValue = { kind: "text", value: "" };
  const drawValue = () => safely(() => { const shape = resolveShape(fields[Number(field.value)].shape, actualSchema); if (!["scalar", "enum", "union"].includes(shape.kind)) { valueHost.replaceChildren(ui("p", "data-help", "多值／嵌套条件请使用下方 CEL，保留同项含义。")); return; } value = emptyNativeValue(shape, actualSchema); valueHost.replaceChildren(valueEditor(shape, value, actualSchema, next => { value = next; }, "筛选值", 0, references)); }); field.onchange = drawValue; drawValue();
  filterHost.append(ui("h4", "", "筛选"), chips, field, operator, valueHost, action("添加筛选条件", () => safely(() => { const invalid = valueHost.querySelector<HTMLInputElement>(":invalid"); if (invalid) { invalid.reportValidity(); return; } const next = addPlanFilter(plan, fields[Number(field.value)], operator.value, value, actualSchema); plan.where = next.where; changed(); drawConditions(); })));
  const advanced = ui("details"); advanced.append(ui("summary", "", "高级：完整 CEL 条件")); const expression = ui("textarea", "data-input"); expression.value = plan.where ?? ""; expression.setAttribute("aria-label", "完整 CEL 条件提案"); advanced.append(expression, action("采用完整条件", () => { if (expression.value.trim()) plan.where = expression.value; else delete plan.where; changed(); drawConditions(); })); filterHost.append(advanced); host.append(filterHost);
  const columns = ui("fieldset"); columns.append(ui("legend", "", "显示字段（作者投影）"));
  for (const entry of fields) { const input = ui("input"); input.type = "checkbox"; input.checked = plan.select.some(column => column.expr === entry.expr); input.setAttribute("aria-label", `显示字段：${entry.label}`); input.onchange = () => { plan.select = plan.select.filter(column => column.expr !== entry.expr); if (input.checked) { if (plan.select.some(column => column.name === entry.key)) { input.checked = false; error.textContent = "已有同名输出列；请保留原列或在源码明确改名。"; return; } plan.select.push({ name: entry.key, expr: entry.expr }); } changed(); }; columns.append(labeled(entry.label, input)); } host.append(columns);
  const sorting = ui("section"), orderList = ui("div");
  const drawOrder = () => { orderList.replaceChildren(); plan.order.forEach((order, i) => orderList.append(ui("code", "", `${order.expr} · ${order.descending ? "降序" : "升序"} · 空值${order.nulls === "first" ? "优先" : "置后"}`), action(`移除排序 ${i + 1}`, () => { plan.order.splice(i, 1); changed(); drawOrder(); }))); }; drawOrder();
  const sort = select("排序属性", fields.map((field, i) => [String(i), field.label]), "0"), direction = select("排序方向", [["asc", "升序"], ["desc", "降序"]], "asc"); sorting.append(ui("h4", "", "排序"), orderList, sort, direction, action("添加排序", () => { plan.order.push({ expr: fields[Number(sort.value)].expr, descending: direction.value === "desc", nulls: "last" }); changed(); drawOrder(); })); host.append(sorting);
  const group = ui("details"); group.append(ui("summary", "", "分组与汇总（作者查询）"));
  if (plan.group) group.append(ui("pre", "", JSON.stringify(plan.group, null, 2)));
  const groupField = select("汇总分组属性", [["", "所有行汇总"], ...fields.map((field, i) => [String(i), field.label] as [string, string])], ""), aggregate = select("汇总方式", [["count", "计数"], ["sum", "求和"], ["min", "最小值"], ["max", "最大值"]], "count"), aggregateField = select("汇总值属性", fields.map((field, i) => [String(i), field.label]), "0");
  group.append(groupField, aggregate, aggregateField, ui("p", "data-help", "采用后替换当前分组／汇总；结果没有原字段写入目标。这里只改本次作者查询，个人视图分组在“视图”中设置。"), action("采用分组汇总", () => { const selected = groupField.value === "" ? null : fields[Number(groupField.value)]; plan.group = { keys: selected ? [{ name: selected.key, expr: selected.expr }] : [], aggregates: [{ name: aggregate.value === "count" ? "count" : "amount", op: aggregate.value as "count" | "sum" | "min" | "max", ...(aggregate.value === "count" ? {} : { expr: fields[Number(aggregateField.value)].expr }) }] }; changed(); }), action("移除作者分组汇总", () => { delete plan.group; changed(); })); host.append(group);
  const limit = ui("input", "data-input"); limit.type = "number"; limit.min = "1"; limit.max = "10000"; limit.step = "1"; limit.value = plan.take == null ? "" : String(plan.take); limit.setAttribute("aria-label", "查询显示上限"); limit.oninput = () => { if (!limit.value) delete plan.take; else if (limit.validity.valid) plan.take = Number(limit.value); changed(); }; host.append(labeled("显示上限", limit), error);
  return host;
}

export function queryCallControls(plan: QueryPlan): HTMLElement {
  const host = ui("section", "query-call-controls"), call = plan.advanced!.call!;
  host.append(ui("p", "data-help", `共享定义归属：${call.owner}；只修改本次调用，不修改归属笔记。`));
  const mode = select("调用定义版本方式", [["current", "当前定义"], ["pinned", "固定指定版本"]], call.selection.mode), revision = ui("input", "data-input"); revision.setAttribute("aria-label", "调用的准确固定版本"); revision.value = call.selection.mode === "pinned" ? call.selection.revision : "";
  const selection = () => { call.selection = mode.value === "pinned" ? { mode: "pinned", key: call.selection.key, revision: revision.value } : { mode: "current", key: call.selection.key }; revision.disabled = mode.value !== "pinned"; }; mode.onchange = selection; revision.oninput = selection; selection(); host.append(mode, revision);
  const args = ui("div"), draw = () => { args.replaceChildren(); for (const [name, expression] of Object.entries(call.arguments ?? {})) { const row = ui("label", "data-label", name), input = ui("input", "data-input"); input.value = expression; input.setAttribute("aria-label", `调用参数 ${name} 的 CEL 表达式`); input.oninput = () => { call.arguments![name] = input.value; }; row.append(input, action(`移除调用参数 ${name}`, () => { delete call.arguments![name]; draw(); })); args.append(row); } }; draw();
  const name = ui("input", "data-input"); name.setAttribute("aria-label", "新增调用参数名称"); const error = ui("p", "data-warning"); host.append(ui("p", "data-help", "下面的 CEL 参数是作者调用内容；本次预览的原生值参数仍在独立“本次运行参数”中。"), args, name, action("添加调用参数", () => { if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name.value) || Object.hasOwn(call.arguments ?? {}, name.value)) { error.textContent = "参数名称无效或重复，原输入保留。"; return; } call.arguments ??= {}; call.arguments[name.value] = "null"; name.value = ""; error.textContent = ""; draw(); }), error); return host;
}
