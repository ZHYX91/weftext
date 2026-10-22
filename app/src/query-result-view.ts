import { uiDisplayParameter } from "./ui-locale";
import { displayMessage } from "./ui-locale";
import { message, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { fieldNames, formatNativeValue, ui, type Cell } from "./data-panel";
import { calendarDay, type QueryPlan, type QueryResult, type QueryRow } from "./query-panel";
export interface OccurrenceView {
    kind: "list" | "table" | "board" | "calendar";
    group: string;
    date: string;
    columns: string[];
    projection?: string;
    warning?: UiText;
}
export const initialOccurrenceView = (): OccurrenceView => ({ kind: "table", group: "status", date: "due", columns: [] });
export const occurrenceProjection = (plan: QueryPlan | null, schemaRevision?: string): string => JSON.stringify([plan?.select, plan?.group, schemaRevision]);
export function decodeOccurrenceView(raw: string | null): OccurrenceView {
    if (!raw)
        return initialOccurrenceView();
    try {
        const value = JSON.parse(raw) as OccurrenceView;
        if (!["list", "table", "board", "calendar"].includes(value.kind) || typeof value.group !== "string" || typeof value.date !== "string" || !Array.isArray(value.columns) || value.columns.some(column => typeof column !== "string") || value.projection !== undefined && typeof value.projection !== "string")
            throw new Error();
        return { kind: value.kind, group: value.group, date: value.date, columns: [...value.columns], projection: value.projection };
    }
    catch {
        return { ...initialOccurrenceView(), warning: message("queryView.invalidRecoveryRecord") };
    }
}
const label = (cell?: Cell): UiText => cell?.state === "value" ? formatNativeValue(cell.value) : ({ absent: message("queryValue.absent"), null: message("queryValue.explicitNull"), invalid: message("queryValue.invalid"), unavailable: message("queryValue.unavailable") } as const)[cell?.state as "absent"] ?? message("queryValue.unprojected");
export function occurrenceBuckets(rows: QueryRow[], field: string, calendar: boolean): {
    key: string;
    label: UiText;
    rows: QueryRow[];
}[] {
    const groups = new Map<string, {
        key: string;
        label: UiText;
        rows: QueryRow[];
    }>();
    for (const row of rows) {
        const cell = row.cells[field], day = cell?.state === "value" ? calendarDay(cell.value) : null;
        const key = calendar ? day ?? "@undated" : JSON.stringify(cell ?? { state: "unprojected" });
        const group: {
            key: string;
            label: UiText;
            rows: QueryRow[];
        } = groups.get(key) ?? { key, label: calendar ? day ?? message("queryView.undatedGroup") : label(cell), rows: [] };
        group.rows.push(row);
        groups.set(key, group);
    }
    return calendar ? [...groups.values()].sort((a, b) => a.key.localeCompare(b.key)) : [...groups.values()];
}
/** Personal display only. Row provenance and order in QueryResult are never mutated. */
export function occurrenceViewControls(view: OccurrenceView, columns: string[], update: () => void): HTMLElement {
    const host = ui("details", "embedded-query-view");
    host.append(ui("summary", "", message("queryView.disclosure")), ui("p", "embedded-query-help", message("queryView.personalDisplayHelp")));
    const picker = (role: string, name: UiText, options: [
        string,
        UiText
    ][], current: string, change: (value: string) => void) => {
        const select = ui("select");
        select.dataset.controlKey = JSON.stringify(["query-view", role]);
        setUiAttr(select, "aria-label", name);
        if (!options.some(([value]) => value === current))
            options.push([current, displayMessage("queryView.retainedUnprojectedChoice", { p0: uiDisplayParameter(current) })]);
        for (const [value, text] of options) {
            const option = ui("option", "", text);
            option.value = value;
            select.append(option);
        }
        select.value = current;
        select.onchange = () => { change(select.value); update(); };
        host.append(select);
    };
    picker("kind", message("queryView.kindLabel"), [["table", message("queryView.table")], ["list", message("queryView.list")], ["board", message("queryView.board")], ["calendar", message("queryView.calendar")]], view.kind, value => { view.kind = value as OccurrenceView["kind"]; });
    picker("group", message("queryView.boardColumnLabel"), columns.map(column => [column, fieldNames[column] ?? column]), view.group, value => { view.group = value; });
    picker("date", message("queryView.calendarColumnLabel"), columns.map(column => [column, fieldNames[column] ?? column]), view.date, value => { view.date = value; });
    for (const column of columns) {
        const checkbox = ui("input");
        checkbox.dataset.controlKey = JSON.stringify(["query-view", "column", column]);
        checkbox.type = "checkbox";
        checkbox.checked = !view.columns.length || view.columns.includes(column);
        setUiAttr(checkbox, "aria-label", displayMessage("queryView.visibleColumnLabel", { p0: uiDisplayParameter(fieldNames[column] ?? column) }));
        checkbox.onchange = () => {
            const selected = new Set(view.columns.length ? view.columns : columns);
            checkbox.checked ? selected.add(column) : selected.delete(column);
            view.columns = [...selected];
            if (!view.columns.length) {
                view.columns = ["@none"];
            }
            update();
        };
        const label = ui("label", "", fieldNames[column] ?? column);
        label.prepend(checkbox);
        host.append(label);
    }
    const reset = ui("button", "", message("queryView.resetPersonal"));
    reset.dataset.controlKey = JSON.stringify(["query-view", "reset"]);
    reset.type = "button";
    reset.onclick = () => { Object.assign(view, initialOccurrenceView()); delete view.warning; delete view.projection; update(); };
    host.append(reset);
    return host;
}
export function renderOccurrenceCards(host: HTMLElement, result: QueryResult, view: OccurrenceView, open: (row: QueryRow) => HTMLElement): void {
    const columns = result.columns ?? [...new Set(result.rows.flatMap(row => Object.keys(row.cells)))];
    const show = (container: HTMLElement, rows: QueryRow[]) => {
        for (const row of rows) {
            const card = ui("article", "embedded-query-card");
            card.append(open(row));
            for (const column of columns.filter(column => column !== "title" && (!view.columns.length || view.columns.includes(column))))
                card.append(ui("p", "", () => `${uiTextValue(fieldNames[column] ?? column)}：${uiTextValue(label(row.cells[column]))}`));
            container.append(card);
        }
    };
    if (view.kind === "list") {
        show(host, result.rows);
        return;
    }
    const field = view.kind === "calendar" ? view.date : view.group;
    if (!columns.includes(field))
        host.append(ui("p", "embedded-query-warning", displayMessage("queryView.missingProjectionColumn", { p0: uiDisplayParameter(fieldNames[field] ?? field) })));
    for (const group of occurrenceBuckets(result.rows, field, view.kind === "calendar")) {
        const lane = ui("section", "embedded-query-lane");
        lane.append(ui("h4", "", () => `${uiTextValue(group.label)} · ${uiTextValue(group.rows.length)}`));
        show(lane, group.rows);
        host.append(lane);
    }
}
