import { action, ui } from "./native-value-form";
import { validateQueryPredicate, type QueryDatePeriod, type QueryPeriodUnit, type QueryRelativeDate } from "./query-plan";
import { errorUiText, message, setUiAttr, setUiText, type UiText } from "./ui-locale";

export interface QueryDateField {
    id: string;
    label: UiText;
    kind: QueryRelativeDate["dateKind"];
}

export function queryRelativeDateControls(
    fields: readonly QueryDateField[], apply: (date: QueryRelativeDate) => void, initial?: QueryRelativeDate,
): HTMLFieldSetElement {
    const host = ui("fieldset", "query-date-controls");
    host.append(ui("legend", "", message("queryDates.title")));
    const choice = (label: UiText, options: [string, UiText][], value: string) => {
        const select = ui("select", "data-input");
        setUiAttr(select, "aria-label", label);
        for (const [id, name] of options) {
            const option = ui("option", "", name);
            option.value = id;
            select.append(option);
        }
        select.value = value;
        return select;
    };
    const input = (type: string, label: UiText, value: string) => {
        const control = ui("input", "data-input");
        control.type = type;
        control.value = value;
        setUiAttr(control, "aria-label", label);
        return control;
    };
    const available = initial && !fields.some(field => field.id === initial.field)
        ? [...fields, { id: initial.field, label: initial.field, kind: initial.dateKind }] : fields;
    const field = choice(message("queryDates.field"), available.map(item => [item.id, item.label]), initial?.field ?? fields[0]?.id ?? "");
    const preset = choice(message("queryDates.period"), [
        ["today", message("queryDates.today")], ["yesterday", message("queryDates.yesterday")],
        ["current", message("queryDates.current")], ["previous", message("queryDates.previous")],
        ["past", message("queryDates.past")], ["future", message("queryDates.future")],
        ["between", message("queryDates.between")],
    ], initial?.period.kind ?? "today");
    const original = initial?.period;
    const unit = choice(message("queryDates.unit"), [
        ["week", message("queryDates.week")], ["month", message("queryDates.month")],
        ["quarter", message("queryDates.quarter")], ["year", message("queryDates.year")],
    ], original && "unit" in original ? original.unit : "week");
    const days = input("number", message("queryDates.days"), original && "days" in original ? String(original.days) : "7");
    days.min = "1";
    days.max = "3660";
    days.step = "1";
    const start = input("date", message("queryDates.start"), original && "start" in original ? original.start : "");
    const end = input("date", message("queryDates.end"), original && "end" in original ? original.end : "");
    const zone = input("text", message("queryDates.zone"), initial?.timeZone ?? Intl.DateTimeFormat().resolvedOptions().timeZone ?? "UTC");
    zone.spellcheck = false;
    const draw = () => {
        unit.hidden = !["current", "previous"].includes(preset.value);
        days.hidden = !["past", "future"].includes(preset.value);
        start.hidden = end.hidden = preset.value !== "between";
    };
    draw();
    preset.onchange = draw;
    const error = ui("p", "data-warning");
    error.setAttribute("role", "status");
    host.append(field, preset, unit, days, start, end, zone, action(message("queryDates.adopt"), () => {
        const definition = available.find(item => item.id === field.value);
        if (!definition) return;
        try {
            const kind = preset.value as QueryDatePeriod["kind"];
            let period: QueryDatePeriod;
            switch (kind) {
                case "current":
                case "previous": period = { kind, unit: unit.value as QueryPeriodUnit }; break;
                case "past":
                case "future": period = { kind, days: days.valueAsNumber }; break;
                case "between": period = { kind, start: start.value, end: end.value }; break;
                case "today":
                case "yesterday": period = { kind }; break;
            }
            const condition = { field: definition.id, dateKind: definition.kind, timeZone: zone.value.trim(), period };
            validateQueryPredicate(condition);
            apply(condition);
            setUiText(error, "");
        } catch (failure) {
            setUiText(error, failure instanceof Error ? errorUiText(failure) : String(failure));
        }
    }), error);
    return host;
}
