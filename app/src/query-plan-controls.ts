import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import type { AuthorInputSession } from "./author-input";
import { message, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Visible controls edit the existing native QueryPlan; unknown CEL stays intact. */
import { action, fieldNames, resolveShape, typeNames, ui, valueEditor, emptyNativeValue, type DataSchema, type NativeValue, type Shape, type FormReferences } from "./data-panel";
import { shapedQueryLiteral, type QueryPlan } from "./query-panel";
export interface PlanField {
    key: string;
    expr: string;
    label: UiText;
    shape: Shape;
}
export function queryPlanFields(schema: DataSchema | null, plan: QueryPlan): PlanField[] {
    const fields: PlanField[] = [{ key: "title", expr: "title", label: message("queryColumn.title"), shape: { kind: "scalar", semantic: "text" } }];
    for (const field of schema?.fields ?? [])
        fields.push({ key: field.key, expr: `field(${JSON.stringify(field.id)})`, label: fieldNames[field.key] ?? field.key, shape: field.shape });
    for (const column of plan.select)
        if (!fields.some(field => field.expr === column.expr))
            fields.push({ key: column.name, expr: column.expr, label: fieldNames[column.name] ?? column.name, shape: { kind: "scalar", semantic: "text" } });
    return fields;
}
export function addPlanFilter(plan: QueryPlan, field: PlanField, op: string, value: NativeValue, schema: DataSchema): QueryPlan {
    const expr = field.expr;
    if (!["==", "!=", "<", "<=", ">", ">=", "contains", "absent", "null"].includes(op))
        throw new UiError(message("queryValidation.unsupportedOperator"));
    const condition = op === "absent" ? `isAbsent(${expr})` : op === "null" ? `isNull(${expr})` : op === "contains" ? (() => {
        if (value.kind !== "text")
            throw new UiError(message("queryValidation.containsTextValueRequired"));
        return `${expr}.contains(${JSON.stringify(value.value)})`;
    })() : `${expr} ${op} ${shapedQueryLiteral(field.shape, schema, value)}`;
    return { ...structuredClone(plan), where: plan.where?.trim() ? `(${plan.where}) && (${condition})` : condition };
}
const emptySchema = (): DataSchema => ({ definitions: {}, fields: [], types: [], aliases: {} });
function select(label: UiText, options: [
    string,
    UiText
][], value: string): HTMLSelectElement {
    const input = ui("select", "data-input");
    setUiAttr(input, "aria-label", label);
    for (const [key, name] of options) {
        const option = ui("option", "", name);
        option.value = key;
        input.append(option);
    }
    input.value = value;
    return input;
}
function labeled(label: UiText, input: HTMLElement): HTMLLabelElement { const wrapper = ui("label", "data-label", label); wrapper.append(input); return wrapper; }
/** This proposal is owned by its caller, and applies only through its existing Draft writer. */
export function queryPlanControls(plan: QueryPlan, schema: DataSchema | null, changed: () => void, references?: FormReferences, inputs?: AuthorInputSession): HTMLElement {
    const host = ui("section", "query-plan-controls"), actualSchema = schema ?? emptySchema(), fields = queryPlanFields(schema, plan);
    const error = ui("p", "data-warning");
    error.setAttribute("role", "status");
    const safely = (run: () => void) => {
        try {
            run();
            setUiText(error, "");
        }
        catch (e) {
            setUiText(error, String(e instanceof Error ? errorUiText(e) : e));
        }
    };
    if (schema?.types.length) {
        const types = ui("fieldset");
        types.append(ui("legend", "", message("queryPlan.noteTypes")));
        for (const type of schema.types) {
            const alias = Object.entries(schema.aliases).find(([, id]) => id === type.id)?.[0] ?? type.id, input = ui("input");
            input.type = "checkbox";
            input.checked = plan.types.some(token => token === alias || token === type.id);
            setUiAttr(input, "aria-label", displayMessage("queryPlan.typeLabel", { p0: uiDisplayParameter(typeNames[alias] ?? alias) }));
            input.onchange = () => {
                plan.types = plan.types.filter(token => token !== alias && token !== type.id);
                if (input.checked)
                    plan.types.push(alias);
                changed();
            };
            types.append(labeled(typeNames[alias] ?? alias, input));
        }
        host.append(types);
    }
    const chips = ui("div", "query-condition-chips"), filterHost = ui("section");
    const drawConditions = () => {
        chips.replaceChildren();
        if (plan.where)
            chips.append(ui("code", "", plan.where), action(message("queryPlan.removeCondition"), () => { delete plan.where; changed(); drawConditions(); }));
        else
            chips.append(ui("p", "data-help", message("queryPlan.noCondition")));
    };
    drawConditions();
    const field = select(message("queryPlan.filterFieldLabel"), fields.map((field, i) => [String(i), field.label]), "0"), operator = select(message("queryPlan.filterOperatorLabel"), [["==", message("queryOperator.equals")], ["!=", message("queryOperator.notEquals")], ["contains", message("queryOperator.containsText")], ["<", message("queryOperator.lessThan")], [">", message("queryOperator.greaterThan")], ["absent", message("queryValue.absent")], ["null", message("queryValue.explicitNull")]], "=="), valueHost = ui("div");
    let value: NativeValue = { kind: "text", value: "" };
    const drawValue = () => safely(() => {
        const shape = resolveShape(fields[Number(field.value)].shape, actualSchema);
        if (!["scalar", "enum", "union"].includes(shape.kind)) {
            valueHost.replaceChildren(ui("p", "data-help", message("queryPlan.nestedFilterHelp")));
            return;
        }
        value = emptyNativeValue(shape, actualSchema);
        valueHost.replaceChildren(valueEditor(shape, value, actualSchema, next => { value = next; }, message("queryPlan.filterValueLabel"), { parameterId: fields[Number(field.value)].expr, path: [] }, 0, references, inputs ? { session: inputs, slot: "plan-filter" } : undefined));
    });
    field.onchange = () => {
        if (field.isConnected === false)
            return;
        inputs?.invalidate("plan-filter");
        drawValue();
    };
    drawValue();
    filterHost.append(ui("h4", "", message("queryPlan.filterTitle")), chips, field, operator, valueHost, action(message("queryPlan.addCondition"), () => safely(() => {
        const invalid = valueHost.querySelector<HTMLInputElement>(":invalid");
        if (invalid) {
            invalid.reportValidity();
            return;
        }
        const next = addPlanFilter(plan, fields[Number(field.value)], operator.value, value, actualSchema);
        plan.where = next.where;
        changed();
        drawConditions();
    })));
    const advanced = ui("details");
    advanced.append(ui("summary", "", message("queryPlan.advancedCelDisclosure")));
    const expression = ui("textarea", "data-input");
    expression.value = plan.where ?? "";
    inputs?.bind(expression, "plan-cel");
    setUiAttr(expression, "aria-label", message("queryPlan.celProposalLabel"));
    advanced.append(expression, action(message("queryPlan.adoptCondition"), () => {
        if (expression.value.trim())
            plan.where = expression.value;
        else
            delete plan.where;
        changed();
        drawConditions();
    }));
    filterHost.append(advanced);
    host.append(filterHost);
    const columns = ui("fieldset");
    columns.append(ui("legend", "", message("queryPlan.projectionLegend")));
    for (const entry of fields) {
        const input = ui("input");
        input.type = "checkbox";
        input.checked = plan.select.some(column => column.expr === entry.expr);
        setUiAttr(input, "aria-label", displayMessage("queryPlan.projectionFieldLabel", { p0: uiDisplayParameter(entry.label) }));
        input.onchange = () => {
            plan.select = plan.select.filter(column => column.expr !== entry.expr);
            if (input.checked) {
                if (plan.select.some(column => column.name === entry.key)) {
                    input.checked = false;
                    setUiText(error, message("queryPlan.outputNameConflict"));
                    return;
                }
                plan.select.push({ name: entry.key, expr: entry.expr });
            }
            changed();
        };
        columns.append(labeled(entry.label, input));
    }
    host.append(columns);
    const sorting = ui("section"), orderList = ui("div");
    const drawOrder = () => { orderList.replaceChildren(); plan.order.forEach((order, i) => orderList.append(ui("code", "", displayMessage("queryPlan.sortSummary", { p0: uiDisplayParameter(order.expr), p1: uiDisplayParameter(order.descending ? message("queryPlan.descending") : message("queryPlan.ascending")), p2: uiDisplayParameter(order.nulls === "first" ? message("queryPlan.nullsFirst") : message("queryPlan.nullsLast")) })), action(displayMessage("queryPlan.removeSort", { p0: i + 1 }), () => { plan.order.splice(i, 1); changed(); drawOrder(); }))); };
    drawOrder();
    const sort = select(message("queryPlan.sortFieldLabel"), fields.map((field, i) => [String(i), field.label]), "0"), direction = select(message("queryPlan.sortDirectionLabel"), [["asc", message("queryPlan.ascending")], ["desc", message("queryPlan.descending")]], "asc");
    sorting.append(ui("h4", "", message("queryPanel.sortLabel")), orderList, sort, direction, action(message("queryPlan.addSort"), () => { plan.order.push({ expr: fields[Number(sort.value)].expr, descending: direction.value === "desc", nulls: "last" }); changed(); drawOrder(); }));
    host.append(sorting);
    const group = ui("details");
    group.append(ui("summary", "", message("queryPlan.groupingDisclosure")));
    if (plan.group)
        group.append(ui("pre", "", JSON.stringify(plan.group, null, 2)));
    const groupField = select(message("queryPlan.groupFieldLabel"), [["", message("queryPlan.aggregateAllRows")], ...fields.map((field, i) => [String(i), field.label] as [
            string,
            UiText
        ])], ""), aggregate = select(message("queryPlan.aggregateFunctionLabel"), [["count", message("queryAggregate.count")], ["sum", message("queryAggregate.sum")], ["min", message("queryAggregate.minimumValue")], ["max", message("queryAggregate.maximumValue")]], "count"), aggregateField = select(message("queryPlan.aggregateFieldLabel"), fields.map((field, i) => [String(i), field.label]), "0");
    group.append(groupField, aggregate, aggregateField, ui("p", "data-help", message("queryPlan.adoptGroupingHelp")), action(message("queryPlan.adoptGrouping"), () => { const selected = groupField.value === "" ? null : fields[Number(groupField.value)]; plan.group = { keys: selected ? [{ name: selected.key, expr: selected.expr }] : [], aggregates: [{ name: aggregate.value === "count" ? "count" : "amount", op: aggregate.value as "count" | "sum" | "min" | "max", ...(aggregate.value === "count" ? {} : { expr: fields[Number(aggregateField.value)].expr }) }] }; changed(); }), action(message("queryPlan.removeGrouping"), () => { delete plan.group; changed(); }));
    host.append(group);
    const limit = ui("input", "data-input");
    limit.type = "number";
    limit.min = "1";
    limit.max = "10000";
    limit.step = "1";
    limit.value = plan.take == null ? "" : String(plan.take);
    inputs?.bind(limit, "plan-take");
    setUiAttr(limit, "aria-label", message("queryPlan.takeLabel"));
    limit.oninput = () => {
        if (!limit.value)
            delete plan.take;
        else if (limit.validity.valid)
            plan.take = Number(limit.value);
        changed();
    };
    host.append(labeled(message("queryPanel.displayLimitLabel"), limit), error);
    return host;
}
export function queryCallControls(plan: QueryPlan, inputs?: AuthorInputSession): HTMLElement {
    const host = ui("section", "query-call-controls"), call = plan.advanced!.call!;
    host.append(ui("p", "data-help", displayMessage("queryCall.ownerHelp", { p0: uiDisplayParameter(call.owner) })));
    const mode = select(message("queryCall.selectionModeLabel"), [["current", message("queryPanel.currentDefinition")], ["pinned", message("queryDefinition.pinExactRevision")]], call.selection.mode), revision = ui("input", "data-input");
    setUiAttr(revision, "aria-label", message("queryCall.pinnedRevisionLabel"));
    revision.value = call.selection.mode === "pinned" ? call.selection.revision : "";
    inputs?.bind(revision, "call-revision");
    const selection = () => { call.selection = mode.value === "pinned" ? { mode: "pinned", key: call.selection.key, revision: revision.value } : { mode: "current", key: call.selection.key }; revision.disabled = mode.value !== "pinned"; };
    mode.onchange = selection;
    revision.oninput = selection;
    selection();
    host.append(mode, revision);
    const args = ui("div"), draw = () => {
        args.replaceChildren();
        for (const [name, expression] of Object.entries(call.arguments ?? {})) {
            const row = ui("label", "data-label", name), input = ui("input", "data-input");
            input.value = expression;
            inputs?.bind(input, `call-argument:${name}`);
            setUiAttr(input, "aria-label", displayMessage("queryCall.argumentExpressionLabel", { p0: uiDisplayParameter(name) }));
            input.oninput = () => { call.arguments![name] = input.value; };
            row.append(input, action(displayMessage("queryCall.removeArgument", { p0: uiDisplayParameter(name) }), () => { inputs?.invalidate(`call-argument:${name}`); delete call.arguments![name]; draw(); }));
            args.append(row);
        }
    };
    draw();
    const name = ui("input", "data-input");
    setUiAttr(name, "aria-label", message("queryCall.newArgumentNameLabel"));
    inputs?.bind(name, "call-new-argument");
    const error = ui("p", "data-warning");
    host.append(ui("p", "data-help", message("queryCall.argumentHelp")), args, name, action(message("queryCall.addArgument"), () => {
        if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name.value) || Object.hasOwn(call.arguments ?? {}, name.value)) {
            setUiText(error, message("queryCall.invalidArgumentName"));
            return;
        }
        call.arguments ??= {};
        call.arguments[name.value] = "null";
        name.value = "";
        setUiText(error, "");
        draw();
    }), error);
    return host;
}
