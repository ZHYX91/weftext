import { applySourcePatches, sourceBoundary, type SourcePatch } from "./editor";
import { diagnosticText, fieldNames, formatNativeValue, type Cell } from "./data-panel";
import { parseQueryParameters, parseQueryPlan, queryResultState, queryRowLifecycle, type QueryPlan, type QueryResult, type QueryRow } from "./query-panel";
import type { Origin, RenderNode, ViewMode } from "./types";
import { textareaSourceDraft, utf8Offset } from "./writing";
import type { FrozenQuery, EmbeddedQuerySelection } from "./export-state";

export interface EmbeddedQueryBasis { sessionId: string; source: string; serial: number }
export interface EmbeddedQueryOccurrence {
  id: string; key: string | null; label: string; kind: "definition" | "call"; callFrom: number;
  range: { from: number; to: number; utf16From: number; utf16To: number; raw: string; writable: boolean };
  writable: boolean; builderSupported: boolean; runnable: boolean; plan?: QueryPlan | null; reason?: string | null; controlReason?: string | null;
  binding?: { schema: string; revision: string; language: string };
}
export interface EmbeddedQueryDiscovery { serial: number; source?: string; sourceHash: string; syntaxValid: boolean; occurrences: EmbeddedQueryOccurrence[] }
export interface EmbeddedQueryRun { serial: number; source?: string; sourceHash: string; occurrenceId: string; draftPreview: boolean; result: QueryResult }
export interface EmbeddedQueryEdit { serial?: number; sourceHash?: string; patchUnit?: string; source: string; patches: { from: number; to: number; text: string; byteFrom?: number; byteTo?: number; expected?: string }[] }
export interface EmbeddedQueryAssociation { label: string; origin: Origin | null; occurrence: EmbeddedQueryOccurrence | null; unique: boolean }

export function sameQueryBasis(left: EmbeddedQueryBasis | null, right: EmbeddedQueryBasis): boolean {
  return !!left && left.sessionId === right.sessionId && left.serial === right.serial && left.source === right.source;
}

/** The complete author range comes from the official AST, never displayed text. */
export function exactQueryOccurrence(source: string, occurrence: EmbeddedQueryOccurrence): boolean {
  const range = occurrence.range;
  return Number.isSafeInteger(range.utf16From) && Number.isSafeInteger(range.utf16To)
    && range.utf16From >= 0 && range.utf16To >= range.utf16From && range.utf16To <= source.length
    && sourceBoundary(source, range.utf16From) && sourceBoundary(source, range.utf16To)
    && source.slice(range.utf16From, range.utf16To) === range.raw
    && utf8Offset(source, range.utf16From) === range.from && utf8Offset(source, range.utf16To) === range.to;
}

/** Only consumed native metadata tags can host a query. Repeated output has no unique writer. */
export function associateEmbeddedQueries(tree: RenderNode, occurrences: readonly EmbeddedQueryOccurrence[]): Map<string, EmbeddedQueryAssociation> {
  const tags: RenderNode[] = [];
  const visit = (node: RenderNode): void => { if (node.kind === "tag" && typeof node.attrs["data-weftext-query-label"] === "string") tags.push(node); node.children.forEach(visit); };
  visit(tree);
  const matched = tags.map(tag => {
    const label = String(tag.attrs["data-weftext-query-label"]);
    const candidates = tag.origin?.isMain ? occurrences.filter(item => item.label === label && item.callFrom === tag.origin!.from) : [];
    return { tag, label, occurrence: candidates.length === 1 ? candidates[0] : null };
  });
  return new Map(matched.map(({ tag, label, occurrence }) => [tag.key, { label, origin: tag.origin, occurrence, unique: !!occurrence && matched.filter(item => item.occurrence?.id === occurrence.id).length === 1 }]));
}

export function embeddedQueryPolicy(mode: ViewMode, current: boolean, writable: boolean, builderSupported: boolean) {
  const edit = mode !== "read" && current && writable;
  return { sourceVisible: mode === "styled", sourcePreview: mode === "source", reveal: mode === "focus" && edit, builder: mode === "write" && edit && builderSupported, sourceDialog: mode === "write" && edit, edit };
}

/** A backend response must preserve all bytes outside the selected native carrier. */
export function validateEmbeddedEdit(basis: EmbeddedQueryBasis, occurrence: EmbeddedQueryOccurrence, result: EmbeddedQueryEdit, expectedHash?: string): SourcePatch[] {
  if (!exactQueryOccurrence(basis.source, occurrence) || !occurrence.writable || !occurrence.range.writable || result.serial !== undefined && result.serial !== basis.serial) throw new Error("查询来源依据已变化，提案保留，未应用修改。");
  const patches = result.patches.map(patch => ({ from: patch.from, to: patch.to, expected: basis.source.slice(patch.from, patch.to), insert: patch.text }));
  if (result.patchUnit !== undefined && result.patchUnit !== "utf16" || expectedHash !== undefined && result.sourceHash !== expectedHash || result.patches.some(patch => patch.byteFrom !== undefined && patch.byteFrom !== utf8Offset(basis.source, patch.from) || patch.byteTo !== undefined && patch.byteTo !== utf8Offset(basis.source, patch.to) || patch.expected !== undefined && patch.expected !== basis.source.slice(patch.from, patch.to))) throw new Error("查询补丁的来源字节依据不一致，未应用修改。");
  if (patches.some(patch => patch.from < occurrence.range.utf16From || patch.to > occurrence.range.utf16To)) throw new Error("查询修改越过了选中定义，草稿保持原样。");
  if (applySourcePatches(basis.source, patches) !== result.source) throw new Error("查询补丁与返回草稿不一致，未应用修改。");
  return patches;
}

/** Source serial and request generation both matter: even Q1 → Q2 → Q1 is a new draft. */
export class EmbeddedQueryRunGate {
  private generation = 0;
  begin(basis: EmbeddedQueryBasis, occurrenceId: string, sourceHash?: string) { return { basis: { ...basis }, occurrenceId, sourceHash, generation: ++this.generation }; }
  invalidate(): void { this.generation++; }
  current(token: ReturnType<EmbeddedQueryRunGate["begin"]>, basis: EmbeddedQueryBasis | null): boolean { return token.generation === this.generation && sameQueryBasis(basis, token.basis); }
  accepts(token: ReturnType<EmbeddedQueryRunGate["begin"]>, current: EmbeddedQueryBasis | null, result: EmbeddedQueryRun): boolean {
    return this.current(token, current) && result.serial === token.basis.serial && result.occurrenceId === token.occurrenceId && (result.source === undefined || result.source === token.basis.source) && (token.sourceHash === undefined || result.sourceHash === token.sourceHash);
  }
}

interface QueryPresentation {
  gate: EmbeddedQueryRunGate; result?: QueryResult; resultBasis?: EmbeddedQueryBasis; resultPlan?: QueryPlan;
  loading: boolean; error?: string; expanded: boolean; builderOpen: boolean; parameters: string; planDraft?: QueryPlan; planBasis?: EmbeddedQueryBasis; autoRunBasis?: EmbeddedQueryBasis;
}
export interface EmbeddedQueryHost {
  basis(): EmbeddedQueryBasis;
  mode(): ViewMode;
  editable(): boolean;
  request<T>(path: string, body: unknown): Promise<T>;
  prepareEdit(): Promise<void>;
  apply(patches: SourcePatch[], basis: EmbeddedQueryBasis, event: string): Promise<void>;
  reveal(occurrence: EmbeddedQueryOccurrence, host: HTMLElement): boolean;
  closeReveal(): void;
  openRow(row: QueryRow, plan: QueryPlan | null): Promise<void>;
  openOwner(origin: Origin): Promise<void>;
  output?(query: FrozenQuery): void | Promise<void>;
  notify(message: string, error?: boolean): void;
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] { const result = document.createElement(tag); result.className = className; result.textContent = text; return result; }
function action(label: string, run: () => void | Promise<void>, host: EmbeddedQueryHost): HTMLButtonElement { const button = element("button", "embedded-query-button", label); button.type = "button"; button.onclick = () => { void Promise.resolve().then(run).catch(error => host.notify(error instanceof Error ? error.message : String(error), true)); }; return button; }
function cellText(cell: Cell): string { return cell.state === "value" ? formatNativeValue(cell.value) : ({ absent: "未填写", null: "none", invalid: "值无效", unavailable: "不可用" } as const)[cell.state]; }
const presentationIdentity = (item: EmbeddedQueryOccurrence): string => JSON.stringify([item.key, item.label, item.kind, item.callFrom]);

/** Session-only presentation. Native source and the shared CodeMirror history remain authoritative. */
export class EmbeddedQueryController {
  private discovery: EmbeddedQueryDiscovery | null = null;
  private discoveryBasis: EmbeddedQueryBasis | null = null;
  private discoveryGeneration = 0;
  private states = new Map<string, QueryPresentation>();
  private mounts = new Map<HTMLElement, EmbeddedQueryAssociation>();
  private dialog: HTMLDialogElement | null = null;
  private associations = new Map<string, EmbeddedQueryAssociation>();
  constructor(private readonly host: EmbeddedQueryHost) {}
  get hasQueries(): boolean { return !!this.discovery?.occurrences.length; }
  get occurrences(): readonly EmbeddedQueryOccurrence[] { return this.discovery?.occurrences ?? []; }
  outputSelections(): EmbeddedQuerySelection[] {
    const basis = this.host.basis();
    if (!sameQueryBasis(this.discoveryBasis, basis)) return [];
    return this.occurrences.flatMap(item => {
      const state = this.states.get(presentationIdentity(item));
      return state?.result?.queryId && !state.loading && !state.error && !state.result.superseded && sameQueryBasis(state.resultBasis ?? null, basis)
        ? [{ occurrenceId: item.id, queryId: state.result.queryId }] : [];
    });
  }
  private state(item: EmbeddedQueryOccurrence): QueryPresentation { const key = presentationIdentity(item); let state = this.states.get(key); if (!state) { state = { gate: new EmbeddedQueryRunGate(), loading: false, expanded: false, builderOpen: false, parameters: "{}" }; this.states.set(key, state); } return state; }
  invalidate(): void { this.discoveryGeneration++; for (const state of this.states.values()) { state.gate.invalidate(); state.loading = false; } for (const [mount, association] of this.mounts) this.renderStatusOnly(mount, association); }
  async discover(basis: EmbeddedQueryBasis): Promise<boolean> {
    const generation = ++this.discoveryGeneration;
    const discovery = await this.host.request<EmbeddedQueryDiscovery>("/api/query/embedded/discover", basis);
    if (generation !== this.discoveryGeneration || !sameQueryBasis(this.host.basis(), basis) || discovery.serial !== basis.serial || discovery.source !== undefined && discovery.source !== basis.source) return false;
    if (discovery.occurrences.some(item => !exactQueryOccurrence(basis.source, item))) throw new Error("查询发现返回了不匹配的原生范围，保留源码。");
    this.discovery = discovery; this.discoveryBasis = { ...basis };
    const identities = new Set(discovery.occurrences.map(presentationIdentity));
    for (const [key, state] of this.states) if (!identities.has(key)) { state.gate.invalidate(); this.states.delete(key); }
    return true;
  }
  beginProjection(tree: RenderNode, basis: EmbeddedQueryBasis): void {
    this.mounts.clear();
    this.associations = associateEmbeddedQueries(tree, sameQueryBasis(this.discoveryBasis, basis) ? this.occurrences : []);
  }
  detach(): void { this.mounts.clear(); this.associations.clear(); }
  closeExpansion(): void { for (const state of this.states.values()) state.expanded = false; }
  modeChanged(): void { if (this.host.mode() !== "write") this.dialog?.close(); if (this.host.mode() !== "focus") this.closeExpansion(); }
  forNode(node: RenderNode): HTMLElement | null {
    const association = this.associations.get(node.key); if (!association) return null;
    const mount = element("section", "embedded-query"); mount.dataset.queryLabel = association.label; mount.setAttribute("aria-label", `正文查询：${association.label}`); this.mounts.set(mount, association); this.render(mount, association);
    const item = association.occurrence;
    if (item && item.runnable && this.current(association)) { const state = this.state(item), basis = this.host.basis(); if (!sameQueryBasis(state.autoRunBasis ?? null, basis)) { state.autoRunBasis = { ...basis }; queueMicrotask(() => { void this.run(association).catch(error => this.host.notify(error instanceof Error ? error.message : String(error), true)); }); } }
    return mount;
  }
  private current(association: EmbeddedQueryAssociation): boolean { return !!association.occurrence && sameQueryBasis(this.discoveryBasis, this.host.basis()) && exactQueryOccurrence(this.host.basis().source, association.occurrence); }
  private canWrite(association: EmbeddedQueryAssociation): boolean { return this.host.editable() && association.unique && association.occurrence?.writable === true && association.occurrence.range.writable; }
  private refresh(item: EmbeddedQueryOccurrence): void { for (const [mount, association] of this.mounts) if (association.occurrence?.id === item.id) this.render(mount, association); }
  private refreshResult(item: EmbeddedQueryOccurrence): void { for (const [mount, association] of this.mounts) if (association.occurrence?.id === item.id) { this.renderResultArea(mount, association, this.state(item)); this.renderStatusOnly(mount, association); } }
  private renderStatusOnly(mount: HTMLElement, association: EmbeddedQueryAssociation): void {
    const status = mount.querySelector<HTMLElement>(".embedded-query-basis"); if (status) status.textContent = this.statusText(association);
    for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-edit]")) button.disabled = !this.current(association) || !this.canWrite(association) || this.host.mode() === "read";
    if (association.occurrence) { const item = association.occurrence, state = this.state(item); for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-run]")) button.disabled = !this.current(association) || !item.runnable || state.loading; for (const button of mount.querySelectorAll<HTMLButtonElement>("[data-query-output]")) button.disabled = !this.current(association) || state.loading || !!state.result?.superseded || !sameQueryBasis(state.resultBasis ?? null, this.host.basis()); }
    if (association.occurrence) { const state = this.state(association.occurrence), status = mount.querySelector<HTMLElement>(".embedded-query-result-state"); if (status && state.result) status.textContent = queryResultState(state.result) + (sameQueryBasis(state.resultBasis ?? null, this.host.basis()) ? "" : " · 对应旧草稿"); }
  }
  private statusText(association: EmbeddedQueryAssociation): string {
    const item = association.occurrence;
    if (!item) return "来源没有可证明的本篇定义关联；保留原生呈现，不能从结果位置回写。";
    const state = this.state(item);
    if (!this.current(association)) return "来源依据已过期 · 当前草稿正在重新定位；旧结果不授予编辑资格。";
    if (state.resultBasis && !sameQueryBasis(state.resultBasis, this.host.basis())) return "对应旧定义 / 旧草稿的结果 · 当前定义尚未运行。";
    return "当前笔记草稿查询预览 · 运行和切换视图均不保存笔记。";
  }
  private render(mount: HTMLElement, association: EmbeddedQueryAssociation): void {
    mount.replaceChildren(); const item = association.occurrence;
    const heading = element("div", "embedded-query-heading"); heading.append(element("strong", "", association.label.replace(/^weftext-query-/, ""))); mount.append(heading, element("p", "embedded-query-basis", this.statusText(association)));
    if (!item) { if (association.origin && !association.origin.isMain) heading.append(action("查看来源笔记", () => this.host.openOwner(association.origin!), this.host)); return; }
    const state = this.state(item), mode = this.host.mode();
    const policy = embeddedQueryPolicy(mode, this.current(association), this.canWrite(association), item.builderSupported && !!item.plan);
    const run = action(state.loading ? "正在运行…" : state.result ? "刷新结果" : "运行草稿查询", () => this.run(association), this.host); run.dataset.queryRun = "run"; run.disabled = state.loading || !this.current(association) || !item.runnable; heading.append(run);
    if (policy.sourceVisible) { const source = element("pre", "embedded-query-native-source", item.range.raw); source.setAttribute("aria-label", "完整查询源码（在左侧共享源码编辑器中编辑）"); mount.append(source); }
    if (policy.sourcePreview) mount.append(element("p", "embedded-query-help", "查询结果预览；在完整源码编辑器中直接修改此定义。"));
    if (policy.reveal) { const toggle = action(state.expanded ? "收起查询源码" : "展开查询源码", () => { const expanded = !state.expanded; this.closeExpansion(); this.host.closeReveal(); state.expanded = expanded; for (const [other, entry] of this.mounts) this.render(other, entry); }, this.host); toggle.dataset.queryEdit = "source"; toggle.setAttribute("aria-expanded", String(state.expanded)); heading.append(toggle); }
    if (policy.builder) { const toggle = action(state.builderOpen ? "收起构建器" : "查询构建器", () => { state.builderOpen = !state.builderOpen; this.refresh(item); }, this.host); toggle.dataset.queryEdit = "builder"; toggle.setAttribute("aria-expanded", String(state.builderOpen)); heading.append(toggle); }
    if (policy.sourceDialog) { const source = action("编辑源码", () => this.editSource(association), this.host); source.dataset.queryEdit = "source"; heading.append(source); }
    if (mode === "write" && !item.builderSupported) mount.append(element("p", "embedded-query-warning", item.controlReason ?? item.reason ?? "构建器不能完整表达此原生查询；源码与条件保留，请明确选择编辑源码。"));
    if (!item.runnable) mount.append(element("p", "embedded-query-warning", `查询预览不可用：${item.reason ?? item.controlReason ?? "本篇来源无法取得完整执行资格。"}`));
    if (!association.unique || !item.writable) mount.append(element("p", "embedded-query-warning", item.reason ?? "此来源没有唯一可写出现项，查询定义保持只读。"));
    if (mode === "read") mount.append(element("p", "embedded-query-help", "阅读视图 · 查询定义与结果行只读。"));
    if (item.plan?.advanced?.call) { const call = item.plan.advanced.call; mount.append(element("p", "embedded-query-help", `调用 ${call.selection.mode === "pinned" ? `固定定义 ${call.selection.revision}` : "当前保存定义"}；本篇源码入口只修改调用，不反写定义 owner。`)); }
    if (mode !== "read") {
      const parameters = element("details", "embedded-query-parameters"); parameters.append(element("summary", "", "本次运行参数")); const input = element("textarea", "embedded-query-input"); input.value = state.parameters; input.rows = 3; input.setAttribute("aria-label", `${association.label} 运行参数（原生 Type/Value JSON）`); input.oninput = () => { state.parameters = input.value; state.gate.invalidate(); state.loading = false; if (state.result) { state.resultBasis = undefined; state.result = { ...state.result, superseded: true }; } this.refreshResult(item); }; parameters.append(input, element("p", "embedded-query-help", "参数只影响本次预览，不修改原生定义。")); mount.append(parameters);
    }
    mount.append(element("div", "embedded-query-results")); this.renderResultArea(mount, association, state);
    if (policy.builder && state.builderOpen) this.renderBuilder(mount, association, state);
    if (policy.reveal && state.expanded) { const source = element("div", "embedded-query-source-host"); mount.append(source); if (!this.host.reveal(item, source)) { state.expanded = false; source.replaceWith(element("p", "embedded-query-warning", "当前来源不能展开，草稿保持原样。")); } }
  }
  /** Result arrivals update only their derived area, leaving focused controls and source selection intact. */
  private renderResultArea(mount: HTMLElement, association: EmbeddedQueryAssociation, state: QueryPresentation): void {
    const area = mount.querySelector<HTMLElement>(".embedded-query-results"), item = association.occurrence; if (!area || !item) return;
    area.replaceChildren();
    const run = mount.querySelector<HTMLButtonElement>("[data-query-run]"); if (run) { run.textContent = state.loading ? "正在运行…" : state.result ? "刷新结果" : "运行草稿查询"; run.disabled = state.loading || !item.runnable || !this.current(association); }
    if (state.error) area.append(element("p", "embedded-query-warning", state.error));
    if (state.result) this.renderResult(area, state.result, state.resultPlan ?? null, sameQueryBasis(state.resultBasis ?? null, this.host.basis()));
    else area.append(element("p", "embedded-query-help", state.loading ? "正在计算此草稿定义…" : "此草稿定义尚未取得结果。"));
    if (state.result?.queryId && this.host.output) { const result = state.result; const output = action("输出本次查询结果", () => { if (!sameQueryBasis(state.resultBasis ?? null, this.host.basis()) || result.superseded) throw new Error("此结果对应旧定义 / 参数，请运行当前草稿后再输出。"); return this.host.output!({ queryId: result.queryId!, label: item.label, rowCount: result.rows.length, coverage: result.coverage, freshness: result.freshness, limited: result.limited }); }, this.host); output.dataset.queryOutput = "output"; output.disabled = state.loading || !!result.superseded || !sameQueryBasis(state.resultBasis ?? null, this.host.basis()); area.append(output); }
  }
  private async run(association: EmbeddedQueryAssociation): Promise<void> {
    const item = association.occurrence; if (!item || !this.current(association) || !item.runnable) return;
    const basis = { ...this.host.basis() }, state = this.state(item), parameters = parseQueryParameters(state.parameters), token = state.gate.begin(basis, item.id, this.discovery?.sourceHash);
    state.loading = true; state.error = undefined; this.refreshResult(item);
    try {
      const result = await this.host.request<EmbeddedQueryRun>("/api/query/embedded/run", { ...basis, occurrenceId: item.id, parameters });
      if (!state.gate.accepts(token, this.host.basis(), result)) return;
      state.result = result.result; state.resultBasis = basis; state.resultPlan = item.plan ?? undefined; state.error = undefined;
    } catch (error) { if (state.gate.current(token, this.host.basis())) { state.error = error instanceof Error ? error.message : String(error); if (state.result) state.result = { ...state.result, superseded: true }; } }
    finally { if (state.gate.current(token, this.host.basis())) { state.loading = false; this.refreshResult(item); } }
  }
  private renderResult(mount: HTMLElement, result: QueryResult, plan: QueryPlan | null, definitionCurrent: boolean): void {
    const status = element("p", `embedded-query-result-state ${result.coverage} ${result.freshness}`, queryResultState(result) + (definitionCurrent ? "" : " · 对应旧草稿 / 参数")); status.setAttribute("role", "status"); mount.append(status);
    // Keep these distinct even when the existing summary chooses one primary state.
    mount.append(element("p", "embedded-query-help", `数据范围：${result.coverage === "complete" ? "完整" : "partial / 部分"} · 数据当前性：${result.freshness}${result.limited ? " · 已达显示上限" : ""}${result.externalPending ? " · 外部稿待接纳" : ""}`));
    if (result.cutId) mount.append(element("p", "embedded-query-help", `读取版本：${result.cutId}`));
    if (result.aggregateExact === false) mount.append(element("p", "embedded-query-warning", "汇总仅对应已知输入，不代表完整总数。"));
    for (const diagnostic of result.diagnostics) mount.append(element("p", "embedded-query-warning", diagnosticText(diagnostic)));
    if (!result.rows.length) { mount.append(element("p", "embedded-query-empty", definitionCurrent && result.coverage === "complete" && result.freshness === "current" && !result.superseded ? "本次已说明范围内的完整空结果。" : "已知范围没有结果；不能据此断言全库为空。")); return; }
    const tableHost = element("div", "embedded-query-table-host"), table = element("table", "embedded-query-table"), header = element("tr");
    const columns = result.columns ?? [...new Set(result.rows.flatMap(row => Object.keys(row.cells)))];
    header.append(element("th", "", "笔记")); for (const column of columns) header.append(element("th", "", fieldNames[column] ?? column)); const thead = element("thead"); thead.append(header); table.append(thead); const tbody = element("tbody");
    for (const row of result.rows) { const tr = element("tr"), target = element("td"); const title = row.cells.title?.state === "value" ? formatNativeValue(row.cells.title.value) : row.path?.replaceAll("\\", "/").split("/").at(-1) ?? "汇总 / 生成行";
      if (row.path && queryRowLifecycle(row, plan)) target.append(action(title, () => this.host.openRow(row, plan), this.host)); else target.textContent = title; tr.append(target); for (const column of columns) tr.append(element("td", `cell-${row.cells[column]?.state ?? "unprojected"}`, row.cells[column] ? cellText(row.cells[column]) : "未投影")); tbody.append(tr);
    }
    table.append(tbody); tableHost.append(table); mount.append(tableHost);
  }
  private renderBuilder(mount: HTMLElement, association: EmbeddedQueryAssociation, state: QueryPresentation): void {
    const item = association.occurrence!, plan = state.planDraft ?? structuredClone(item.plan!); state.planDraft = plan; state.planBasis ??= { ...this.host.basis() };
    const builder = element("form", "embedded-query-builder"); builder.append(element("strong", "", "修改此查询定义"));
    const add = (label: string, value: string, update: (value: string) => void, multiline = false): HTMLInputElement | HTMLTextAreaElement => { const field = element("label", "embedded-query-field", label), input = multiline ? element("textarea", "embedded-query-input") : element("input", "embedded-query-input"); input.value = value; input.setAttribute("aria-label", `${item.label} ${label}`); input.oninput = () => update(input.value); field.append(input); builder.append(field); return input; };
    add("类型（逗号分隔）", plan.types.join(", "), value => { plan.types = value.split(",").map(item => item.trim()).filter(Boolean); });
    add("条件（CEL）", plan.where ?? "", value => { if (value.trim()) plan.where = value; else delete plan.where; }, true);
    const take = add("显示上限（留空为未设置）", plan.take == null ? "" : String(plan.take), value => { if (!value.trim()) delete plan.take; else plan.take = Number(value); }); if (take instanceof HTMLInputElement) { take.type = "number"; take.min = "1"; take.step = "1"; }
    const descendants = element("input"); descendants.type = "checkbox"; descendants.checked = plan.scope.descendants; descendants.onchange = () => { plan.scope.descendants = descendants.checked; }; const scope = element("label", "embedded-query-field", "包含子目录"); scope.append(descendants); builder.append(scope);
    builder.append(element("p", "embedded-query-help", "应用是一个共享草稿操作，可与源码输入一起撤销；不保存笔记。未改动的原生成员和注释由后端保留。"));
    const apply = action("应用查询修改", () => this.edit(association, { plan: parseQueryPlan(JSON.stringify(plan)) }, "input.query.builder", state.planBasis).then(() => { state.planDraft = undefined; state.planBasis = undefined; state.builderOpen = false; }), this.host); apply.dataset.queryEdit = "builder"; builder.onsubmit = event => { event.preventDefault(); apply.click(); }; builder.append(apply, action("放弃构建器输入", () => { state.planDraft = undefined; state.planBasis = undefined; state.builderOpen = false; this.refresh(item); }, this.host)); mount.append(builder);
  }
  private async edit(association: EmbeddedQueryAssociation, proposal: { plan?: QueryPlan; replacement?: string }, event: string, expectedBasis?: EmbeddedQueryBasis): Promise<void> {
    const item = association.occurrence;
    if (!item || !this.current(association) || !this.canWrite(association) || this.host.mode() === "read") throw new Error("当前查询没有可写来源；输入提案保留。");
    const basis = expectedBasis ?? { ...this.host.basis() };
    await this.host.prepareEdit();
    if (!sameQueryBasis(this.host.basis(), basis)) throw new Error("打开编辑入口后草稿已变化；共享草稿与输入提案均保留，请重新核对。");
    const result = await this.host.request<EmbeddedQueryEdit>("/api/query/embedded/edit", { ...basis, occurrenceId: item.id, ...proposal });
    if (!sameQueryBasis(this.host.basis(), basis) || this.host.mode() === "read" || !this.canWrite(association)) throw new Error("确认期间又有输入或编辑资格变化；提案保留，未覆盖共享草稿。");
    const patches = validateEmbeddedEdit(basis, item, result, this.discovery?.sourceHash); if (patches.length) await this.host.apply(patches, basis, event);
  }
  private async editSource(association: EmbeddedQueryAssociation): Promise<void> {
    const item = association.occurrence; if (!item || !this.current(association) || !this.canWrite(association) || this.host.mode() !== "write") return;
    const basis = { ...this.host.basis() }; this.dialog?.close();
    const dialog = element("dialog", "embedded-query-dialog"), form = element("form"), input = element("textarea", "embedded-query-input embedded-query-source-proposal"), feedback = element("p", "embedded-query-warning"); input.value = item.range.raw; input.rows = 14; input.spellcheck = false; input.setAttribute("aria-label", `${item.label} 尚未应用的原生源码提案`);
    form.append(element("h2", "", `编辑查询源码 · ${item.label}`), element("p", "embedded-query-help", "此窗口保留尚未应用的提案。取消不改共享草稿；确认只改打开时定位的查询范围，并进入同一撤销历史。"), input, feedback);
    const buttons = element("div", "embedded-query-heading"), confirm = action("确认应用到草稿", async () => { confirm.disabled = true; try { const replacement = textareaSourceDraft(item.range.raw, input.value); if (replacement !== item.range.raw) await this.edit(association, { replacement }, "input.query.source", basis); dialog.close(); } catch (error) { feedback.textContent = error instanceof Error ? error.message : String(error); } finally { confirm.disabled = false; } }, this.host); confirm.dataset.queryEdit = "source"; buttons.append(action("取消并保留共享草稿", () => dialog.close(), this.host), confirm); form.append(buttons); form.onsubmit = event => { event.preventDefault(); confirm.click(); }; dialog.append(form); document.body.append(dialog); this.dialog = dialog; dialog.addEventListener("close", () => { dialog.remove(); if (this.dialog === dialog) this.dialog = null; }, { once: true }); dialog.showModal(); input.focus();
  }
}
