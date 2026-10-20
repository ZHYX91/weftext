import { fieldNames, formatNativeValue, ui, type Cell } from "./data-panel";
import { calendarDay, type QueryPlan, type QueryResult, type QueryRow } from "./query-panel";

export interface OccurrenceView { kind: "list" | "table" | "board" | "calendar"; group: string; date: string; columns: string[]; projection?: string; warning?: string }
export const initialOccurrenceView = (): OccurrenceView => ({ kind: "table", group: "status", date: "due", columns: [] });
export const occurrenceProjection = (plan: QueryPlan | null, schemaRevision?: string): string => JSON.stringify([plan?.select, plan?.group, schemaRevision]);
export function decodeOccurrenceView(raw: string | null): OccurrenceView { if (!raw) return initialOccurrenceView(); try { const value = JSON.parse(raw) as OccurrenceView; if (!["list", "table", "board", "calendar"].includes(value.kind) || typeof value.group !== "string" || typeof value.date !== "string" || !Array.isArray(value.columns) || value.columns.some(column => typeof column !== "string") || value.projection !== undefined && typeof value.projection !== "string") throw new Error(); return { kind: value.kind, group: value.group, date: value.date, columns: [...value.columns], projection: value.projection }; } catch { return { ...initialOccurrenceView(), warning: "本机视图恢复记录无效，暂用默认呈现；作者查询未变。" }; } }
const label = (cell?: Cell): string => cell?.state === "value" ? formatNativeValue(cell.value) : ({ absent: "未填写", null: "明确为空", invalid: "值无效", unavailable: "不可用" } as const)[cell?.state as "absent"] ?? "未投影";
export function occurrenceBuckets(rows: QueryRow[], field: string, calendar: boolean): { key: string; label: string; rows: QueryRow[] }[] {
  const groups = new Map<string, { key: string; label: string; rows: QueryRow[] }>();
  for (const row of rows) { const cell = row.cells[field], day = cell?.state === "value" ? calendarDay(cell.value) : null; const key = calendar ? day ?? "@undated" : JSON.stringify(cell ?? { state: "unprojected" }); const group = groups.get(key) ?? { key, label: calendar ? day ?? "不能放入单日格（缺值／非日期／错误）" : label(cell), rows: [] }; group.rows.push(row); groups.set(key, group); }
  return calendar ? [...groups.values()].sort((a, b) => a.key.localeCompare(b.key)) : [...groups.values()];
}
/** Personal display only. Row provenance and order in QueryResult are never mutated. */
export function occurrenceViewControls(view: OccurrenceView, columns: string[], update: () => void): HTMLElement {
  const host = ui("details", "embedded-query-view"); host.append(ui("summary", "", "视图"), ui("p", "embedded-query-help", "个人呈现：不修改作者 Query、条件、数据或其他人的视图。"));
  const picker = (name: string, options: [string, string][], current: string, change: (value: string) => void) => { const select = ui("select"); select.setAttribute("aria-label", name); if (!options.some(([value]) => value === current)) options.push([current, `${current}（未投影／保留选择）`]); for (const [value, text] of options) { const option = ui("option", "", text); option.value = value; select.append(option); } select.value = current; select.onchange = () => { change(select.value); update(); }; host.append(select); };
  picker("此查询的个人视图", [["table", "表格"], ["list", "列表"], ["board", "看板"], ["calendar", "日历"]], view.kind, value => { view.kind = value as OccurrenceView["kind"]; });
  picker("个人看板分组列", columns.map(column => [column, fieldNames[column] ?? column]), view.group, value => { view.group = value; });
  picker("个人日历日期列", columns.map(column => [column, fieldNames[column] ?? column]), view.date, value => { view.date = value; });
  for (const column of columns) { const checkbox = ui("input"); checkbox.type = "checkbox"; checkbox.checked = !view.columns.length || view.columns.includes(column); checkbox.setAttribute("aria-label", `个人显示列：${fieldNames[column] ?? column}`); checkbox.onchange = () => { const selected = new Set(view.columns.length ? view.columns : columns); checkbox.checked ? selected.add(column) : selected.delete(column); view.columns = [...selected]; if (!view.columns.length) { view.columns = ["@none"]; } update(); }; const label = ui("label", "", fieldNames[column] ?? column); label.prepend(checkbox); host.append(label); }
  const reset = ui("button", "", "恢复个人默认视图"); reset.type = "button"; reset.onclick = () => { Object.assign(view, initialOccurrenceView()); delete view.warning; delete view.projection; update(); }; host.append(reset); return host;
}
export function renderOccurrenceCards(host: HTMLElement, result: QueryResult, view: OccurrenceView, open: (row: QueryRow) => HTMLElement): void {
  const columns = result.columns ?? [...new Set(result.rows.flatMap(row => Object.keys(row.cells)))];
  const show = (container: HTMLElement, rows: QueryRow[]) => { for (const row of rows) { const card = ui("article", "embedded-query-card"); card.append(open(row)); for (const column of columns.filter(column => column !== "title" && (!view.columns.length || view.columns.includes(column)))) card.append(ui("p", "", `${fieldNames[column] ?? column}：${label(row.cells[column])}`)); container.append(card); } };
  if (view.kind === "list") { show(host, result.rows); return; }
  const field = view.kind === "calendar" ? view.date : view.group;
  if (!columns.includes(field)) host.append(ui("p", "embedded-query-warning", `所选列 ${fieldNames[field] ?? field} 未投影；不会换用同名其它字段，所有行仍保留在未投影分组。`));
  for (const group of occurrenceBuckets(result.rows, field, view.kind === "calendar")) { const lane = ui("section", "embedded-query-lane"); lane.append(ui("h4", "", `${group.label} · ${group.rows.length}`)); show(lane, group.rows); host.append(lane); }
}
