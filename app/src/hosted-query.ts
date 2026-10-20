import { action, diagnosticText, formatNativeValue, ui, fieldNames, type Cell, type DataSchema } from "./data-panel";
import { queryPlanControls } from "./query-plan-controls";
import { initialOccurrenceView, occurrenceViewControls, renderOccurrenceCards } from "./query-result-view";
import { directQueryField, parseQueryJson, parseQueryParameters, parseQueryPlan, queryResultState, type QueryPlan } from "./query-panel";
import { applyHostedScope, decodeHostedQuery, exactHostedDependencies, exactHostedScope, hostedDependencyMap, hostedScopeDraft, HostedQueryGate, type HostedProviderDescriptor, type HostedQueryDependencies, type HostedQueryResponse, type HostedQueryResult, type HostedQueryRow, type HostedSeriesPolicyDescriptor, type HostedScopeCatalog } from "./hosted-query-state";
import { hostedCachePrefix } from "./hosted-state";
import { HostedQueryDefinitions, type HostedDefinitionHost } from "./hosted-query-definitions";
import { mediaCellReferences } from "./hosted-media-state";
import { TagTree, tagPredicate, type TagMatchMode } from "./tag-tree";
import { tagPicker } from "./tag-picker";
import type { RegistryCatalog } from "./hosted-navigation-registry-state";

type Product = "title" | "body" | "headings" | "types" | "self";
type Resource = { resource: string; title: string; fields: string[]; products: Record<Product, boolean>; relations: string[]; definitions: boolean; definitionHistory: boolean };
type Definition = { resource?: string; key: string; label: string; revision?: string; plan?: QueryPlan; binding?: unknown };
type Catalog = HostedScopeCatalog & { formSchema?: DataSchema | null; resources: Resource[]; definitions: Definition[]; providers?: HostedProviderDescriptor[]; seriesPolicies?: HostedSeriesPolicyDescriptor[]; coverage?: "complete" | "partial"; providerCoverage?: "complete" | "partial"; unavailableDefinitions?: { resource: string; status: "unavailable" }[]; unavailableProviders?: { id: string; status: "unavailable" }[]; unavailableSeriesPolicies?: { id: string; status: "unavailable" }[] };
type Draft = HostedQueryDependencies & { resources: string[]; fields: string[]; products: Product[]; relations: string[]; plan: string; parameters: string; owner: string; definition: string; selection: "current" | "pinned"; revision: string; scopeDraft?: QueryPlan["scope"]; visual?: { where: string; columns: string; order: string }; tagFilter?: { reference: string; mode: TagMatchMode } };
type Request = <T>(path: string, body?: unknown) => Promise<T>;
const productNames: Record<Product, string> = { title: "标题", body: "呈现正文", headings: "真实深标题", types: "笔记类型", self: "自身稳定身份" };
const basePlan = (): QueryPlan => ({ scope: { descendants: true, lifecycle: "live" }, types: [], select: [], order: [] });
const initial = (): Draft => ({ resources: [], fields: [], products: [], relations: [], providers: {}, seriesPolicies: {}, plan: JSON.stringify(basePlan(), null, 2), parameters: "{}", owner: "", definition: "", selection: "current", revision: "" });
function select(label: string, options: [string, string][], value: string) { const control = ui("select", "data-input"); control.setAttribute("aria-label", label); for (const [key, text] of options) { const option = ui("option", "", text); option.value = key; control.append(option); } control.value = value; return control; }
function input(label: string, value: string, multiline = false) { const control = multiline ? ui("textarea", "data-input") : ui("input", "data-input"); control.setAttribute("aria-label", label); control.value = value; if (control instanceof HTMLTextAreaElement) { control.rows = 10; control.spellcheck = false; } const host = ui("label", "data-label", label); host.append(control); return { host, control }; }
const text = (cell?: Cell) => !cell ? "未投影" : cell.state === "value" ? formatNativeValue(cell.value) : ({ absent: "未填写", null: "none", invalid: "无效值", unavailable: "不可用" } as const)[cell.state];

/** Temporary controls reuse the native Query plan. Only the author buffer saves definitions. */
export class HostedQuery {
  private view = initialOccurrenceView();
  private epoch = 0;
  private key = "";
  private inputRevision = 0;
  private draft = initial();
  private catalog: Catalog = { resources: [], definitions: [] };
  private gate = new HostedQueryGate();
  private busy = false;
  private result?: HostedQueryResult;
  private resultInput?: string;
  private unreadable?: string;
  private message = ui("p", "hint");
  private controls = ui("section");
  private results = ui("section");
  private scopeControls = ui("section");
  private tagTree?: TagTree;
  private tagReadSerial = 0;
  private definitions?: HostedQueryDefinitions;
  constructor(private host: HTMLElement, private request: Request, private open: (resource: string) => Promise<void>, private output: (queryId: string, metadata: { columns: string[]; rowCount: number; coverage: string }) => void, author?: HostedDefinitionHost, private dependenciesChanged?: () => void, private openMedia?: (reference: string) => Promise<void>, private fieldActions?: { canEdit(resource: string, fieldId: string): boolean; open(resource: string, fieldId: string): Promise<void> }) { if (author) this.definitions = new HostedQueryDefinitions(request, author, () => this.draft.plan, resource => this.catalog.resources.some(item => item.resource === resource && item.definitions)); this.clear(false); }
  dependencies(): HostedQueryDependencies { return exactHostedDependencies(this.draft, this.catalog); }
  get version() { return this.inputRevision; }
  refuseResult(message: string) { this.gate.clear(); this.result = undefined; this.resultInput = undefined; this.message.textContent = message; this.renderResults(); }
  environmentChanged(message = "库配置已变化，原冻结结果保留为旧依据；计划和参数未改，请明确重新运行。") { this.tagReadSerial++; this.tagTree = undefined; this.inputRevision++; this.gate.changed(); if (this.result) this.result = { ...this.result, superseded: true }; this.message.textContent = message; this.renderResults(); }
  private dependencyBasis() { return JSON.stringify([this.draft.providers, this.draft.seriesPolicies, ...(["providers", "seriesPolicies"] as const).map(kind => Object.keys(this.draft[kind]).map(id => [id, this.catalog[kind]?.find(item => item.id === id)?.fingerprint ?? null]))]); }
  authorChanged(force = false) { void this.definitions?.authorChanged(force); }
  async refreshCatalog(): Promise<void> {
    const epoch = this.epoch;
    try { const catalog = await this.request<Catalog>("query/catalog"); if (epoch !== this.epoch) return; const previous = this.dependencyBasis(), previousScope = JSON.stringify([this.catalog.directories, this.catalog.trash]); this.catalog = catalog; if (previous !== this.dependencyBasis() || previousScope !== JSON.stringify([catalog.directories, catalog.trash])) { this.changed(); this.dependenciesChanged?.(); } this.renderControls(); }
    catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "当前定义目录刷新失败，原计划和冻结结果保留。"; }
  }
  clear(removeCache = true) {
    this.view = initialOccurrenceView(); this.tagReadSerial++; this.tagTree = undefined;
    this.epoch++; this.inputRevision++; this.gate.clear(); this.definitions?.clear(); this.busy = false; this.result = undefined; this.resultInput = undefined; this.draft = initial(); this.catalog = { resources: [], definitions: [] }; this.unreadable = undefined;
    this.host.replaceChildren(); this.host.hidden = true; this.controls.replaceChildren(); this.results.replaceChildren(); this.scopeControls.replaceChildren(); this.message.textContent = ""; this.key = "";
    this.dependenciesChanged?.();
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-query:")) sessionStorage.removeItem(key); this.key = "";
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); const epoch = this.epoch; this.key = hostedCachePrefix("query", deployment, actor, generation) + "controls";
    let raw: string | null = null;
    try { raw = sessionStorage.getItem(this.key); if (raw) { const parsed = JSON.parse(raw) as Draft; if (!Array.isArray(parsed.resources) || !Array.isArray(parsed.fields) || !Array.isArray(parsed.products) || !Array.isArray(parsed.relations) || typeof parsed.plan !== "string" || typeof parsed.parameters !== "string" || !["current", "pinned"].includes(parsed.selection) || parsed.tagFilter !== undefined && (!parsed.tagFilter || typeof parsed.tagFilter.reference !== "string" || !["exact", "subtree"].includes(parsed.tagFilter.mode))) throw new Error(); this.draft = { ...parsed, providers: hostedDependencyMap(parsed.providers), seriesPolicies: hostedDependencyMap(parsed.seriesPolicies), scopeDraft: hostedScopeDraft(parsed.scopeDraft) }; } }
    catch { this.unreadable = raw ?? ""; }
    this.host.hidden = false; this.host.replaceChildren(ui("h2", "", "托管高级 Query"), ui("p", "hint", "字段、正文产品、关系与保存定义分别授权。这里编辑临时计划；运行和结果导航均不保存正文。"), this.message, this.controls, this.results);
    this.renderControls();
    try {
      const catalog = await this.request<Catalog>("query/catalog"); if (epoch !== this.epoch) return; this.catalog = catalog;
      if (!raw) { const first = catalog.resources.find(item => item.fields.length) ?? catalog.resources[0]; this.draft.resources = first ? [first.resource] : []; this.draft.fields = first?.fields ?? []; const plan = basePlan(); plan.select = this.draft.fields.map(id => ({ name: id, expr: `field(${JSON.stringify(id)})` })); this.draft.plan = JSON.stringify(plan, null, 2); }
      if (!this.draft.owner) this.draft.owner = catalog.definitions[0]?.resource ?? catalog.resources.find(item => item.definitions)?.resource ?? "";
      this.renderControls();
    } catch (error) { if (epoch === this.epoch) this.message.textContent = error instanceof Error ? error.message : "查询目录暂不可用，原输入保留。"; }
    if (epoch === this.epoch) this.dependenciesChanged?.();
    if (epoch === this.epoch && this.definitions) { this.host.append(this.definitions.element); await this.definitions.connect(deployment, actor, generation); }
  }
  private fingerprint() { return JSON.stringify(this.draft); }
  private persist() { if (!this.key || this.unreadable !== undefined) return; try { sessionStorage.setItem(this.key, JSON.stringify(this.draft)); } catch { this.message.textContent = "无法保留查询控件输入，请先复制计划与参数；查询不会修改正文。"; } }
  private changed() { this.inputRevision++; this.gate.changed(); if (this.result) this.result = { ...this.result, superseded: true }; this.message.textContent = this.busy ? "输入已更新，旧请求不会替换新计划或参数。原结果若保留，会标明对应先前输入。" : "临时查询输入已更新，尚未运行当前计划；正文保持保留。"; this.persist(); this.renderResults(); }
  private writePlan(plan: QueryPlan) { this.draft.plan = JSON.stringify(plan, null, 2); this.draft.visual = undefined; this.changed(); this.renderControls(); }
  private checkboxes<T extends string>(legend: string, values: T[], chosen: T[], update: (items: T[]) => void) {
    const group = ui("fieldset"); group.append(ui("legend", "", legend));
    for (const value of values) { const label = ui("label", "data-label", legend === "获权语义产品" ? productNames[value as Product] : value); const control = ui("input"); control.type = "checkbox"; control.checked = chosen.includes(value); control.setAttribute("aria-label", `${legend} ${value}`); control.onchange = () => { const next = new Set(chosen); if (control.checked) next.add(value); else next.delete(value); update([...next]); }; label.prepend(control); group.append(label); }
    for (const value of chosen.filter(value => !values.includes(value))) group.append(ui("p", "hint", `保留的原选择 ${value} 当前不在获权目录；请重新选择后运行。`)); return group;
  }
  private dependencyControls(kind: "providers" | "seriesPolicies") {
    const title = kind === "providers" ? "获权 Provider" : "获权系列政策", group = ui("fieldset"); group.append(ui("legend", "", title));
    const descriptors = this.catalog[kind] ?? [], selection = this.draft[kind];
    const update = (id: string, fingerprint?: string) => { this.draft[kind] = Object.fromEntries([...Object.entries(selection).filter(([key]) => key !== id), ...(fingerprint ? [[id, fingerprint]] : [])]); this.changed(); this.dependenciesChanged?.(); this.renderControls(); };
    for (const item of descriptors) { const label = ui("label", "data-label", item.id), control = ui("input"); control.type = "checkbox"; control.checked = selection[item.id] === item.fingerprint; control.setAttribute("aria-label", `${title} ${item.id}`); control.onchange = () => update(item.id, control.checked ? item.fingerprint : undefined); label.prepend(control); group.append(label);
      const detail = kind === "providers" ? (() => { const provider = item as HostedProviderDescriptor; return `${provider.implementation.id} · ABI ${provider.implementation.abi} · ${provider.data.kind}${provider.data.key ? ` ${provider.data.key}` : ""}${provider.data.version ? ` / ${provider.data.version}` : ""} · ${(provider.queryNames ?? []).join("、")}`; })() : (() => { const policy = item as HostedSeriesPolicyDescriptor; return `${policy.key} · ${policy.kinds.join("、")} · ${policy.rule} · calendar ${policy.calendar ?? "不可用 / 未获权"} · zone ${policy.zone ?? "不可用 / 未获权"}${policy.status === "unavailable" ? " · 政策依赖不可用" : ""}；相关 Provider 仍须分别选择。`; })(); group.append(ui("p", "hint", detail)); }
    for (const [id, fingerprint] of Object.entries(selection)) { const item = descriptors.find(item => item.id === id); if (!item || item.fingerprint !== fingerprint) group.append(ui("p", "hint", `保留的原${title}选择 ${id}（${fingerprint}）当前不可用或已改版；请取消原选择或明确选择目录中的新版本，不会自动更新。`), action(`移除原${title}选择 ${id}`, () => update(id))); }
    if (!descriptors.length) group.append(ui("p", "hint", `本次没有可选的${title}。`)); return group;
  }
  private renderScopes() {
    this.scopeControls.replaceChildren(); let parsed: QueryPlan | undefined; try { parsed = parseQueryPlan(this.draft.plan); } catch { /* Preserve incomplete JSON verbatim. */ }
    const current = this.draft.scopeDraft ?? parsed?.scope ?? basePlan().scope, directories = this.catalog.directories ?? [], group = ui("fieldset"); group.append(ui("legend", "", "顶层查询范围"));
    if (this.catalog.scopeCoverage === "partial") group.append(ui("p", "hint", "获权范围目录部分可用；不可用项保留，不代表完整空范围。"));
    const options: [string, string][] = [["", "全部明确获权对象（无目录限制）"], ...directories.map(item => [item.reference, `${item.title || item.id} · ${item.kind === "node" ? "笔记目录" : "管理目录"} · ${item.lifecycle ?? "live"}${item.status === "unavailable" ? " · 不可用" : ""}`] as [string, string])];
    if (current.directory && !options.some(([value]) => value === current.directory)) options.push([current.directory, `保留原范围 ${current.directory}（不按同名替换）`]);
    const directory = select("托管查询目录范围", options, current.directory ?? ""), descendants = ui("input"), lifecycle = select("托管查询生命周期", [["live", "在库笔记"], ["trash", "回收站（只读）"]], current.lifecycle); descendants.type = "checkbox"; descendants.checked = current.descendants; descendants.setAttribute("aria-label", "托管查询包含子目录");
    for (const option of directory.options) { const item = directories.find(item => item.reference === option.value); if (item) option.disabled = item.status !== "available" || item.lifecycle === "trash" && this.catalog.trash !== true; }
    lifecycle.options[1].disabled = this.catalog.trash !== true;
    const update = () => { this.draft.scopeDraft = { ...(directory.value ? { directory: directory.value } : {}), descendants: descendants.checked, lifecycle: lifecycle.value as "live" | "trash" }; this.changed(); this.renderScopes(); };
    directory.onchange = update; descendants.onchange = update; lifecycle.onchange = update;
    const label = ui("label", "data-label", "包含子目录"); label.prepend(descendants); group.append(directory, label, lifecycle);
    try { exactHostedScope(current, this.catalog); } catch (error) { group.append(ui("p", "hint", error instanceof Error ? error.message : String(error))); }
    if (this.draft.scopeDraft) group.append(ui("p", "hint", "范围控件尚未应用到临时计划；运行已暂停。原 JSON 与嵌套范围保持保留。"));
    if (!parsed) group.append(ui("p", "hint", "完整计划 JSON 尚未有效；范围输入独立保留，应用不会替换或补全原 JSON。"));
    if (parsed?.advanced?.call || parsed?.advanced?.unionAll) group.append(ui("p", "hint", "此计划的外层范围须保持默认；请在实际扫描分支或保存定义内设置范围，不会自动改写嵌套计划。"));
    const apply = action("应用范围到临时计划", () => { try { exactHostedScope(current, this.catalog); const plan = applyHostedScope(this.draft.plan, current); this.draft.plan = plan; delete this.draft.scopeDraft; this.changed(); this.renderControls(); } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } });
    const cancel = action("放弃未应用范围输入", () => { delete this.draft.scopeDraft; this.changed(); this.renderControls(); }); cancel.disabled = !this.draft.scopeDraft;
    group.append(apply, cancel, ui("p", "hint", "只修改最外层 scope，不改 union、related 或保存定义调用中的嵌套范围。目录、回收站与字段/正文分别授权；结果不授恢复、删除或写入。")); this.scopeControls.append(group);
  }
  private renderControls() {
    const disclosures = new Map([...this.controls.querySelectorAll<HTMLDetailsElement>("details[data-query-disclosure]")].map(item => [item.dataset.queryDisclosure, item.open]));
    const active = document.activeElement, focus = active instanceof HTMLElement && this.controls.contains(active) ? active.getAttribute("aria-label") : null;
    const selection = active instanceof HTMLTextAreaElement || active instanceof HTMLInputElement && ["text", "search", "password", "email", "url", "tel"].includes(active.type) ? { from: active.selectionStart, to: active.selectionEnd } : undefined;
    this.controls.replaceChildren();
    if (this.unreadable !== undefined) { this.controls.append(ui("p", "hint", "查询恢复输入无法解析，原记录只读保留。"), ui("pre", "", this.unreadable), action("放弃损坏的查询恢复记录", () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initial(); this.renderControls(); })); return; }
    const draft = this.draft, resources = this.catalog.resources;
    this.controls.append(ui("p", "hint", this.catalog.coverage === "complete" ? "本次获权查询目录完整。" : this.catalog.coverage === "partial" ? "获权查询目录部分可用；未取得的定义来源保持不可用，不视为不存在。" : "查询目录完整性尚未取得。"));
    if (this.catalog.providerCoverage === "partial") this.controls.append(ui("p", "hint", "Provider 与系列政策目录部分可用；未取得的获权项保持不可用，不能按完整空目录处理。"));
    for (const missing of this.catalog.unavailableDefinitions ?? []) this.controls.append(ui("p", "hint", `定义来源 ${missing.resource} 当前不可用；不能换用同名定义或当前版本。`));
    for (const missing of this.catalog.unavailableProviders ?? []) this.controls.append(ui("p", "hint", `已获权 Provider ${missing.id} 当前不可用；原选择保留。`));
    for (const missing of this.catalog.unavailableSeriesPolicies ?? []) this.controls.append(ui("p", "hint", `已获权系列政策 ${missing.id} 当前不可用；原选择保留。`));
    this.controls.append(this.checkboxes("查询对象", resources.map(item => item.resource), draft.resources, items => { draft.resources = items; this.changed(); this.renderControls(); }));
    const selected = resources.filter(item => draft.resources.includes(item.resource));
    this.controls.append(this.checkboxes("获权字段", [...new Set(selected.flatMap(item => item.fields))], draft.fields, items => { draft.fields = items; this.changed(); this.renderControls(); }));
    this.controls.append(this.checkboxes("获权语义产品", (Object.keys(productNames) as Product[]).filter(product => selected.some(item => item.products[product])), draft.products, items => { draft.products = items; this.changed(); this.renderControls(); }));
    this.controls.append(this.checkboxes("获权关系", [...new Set(selected.flatMap(item => item.relations))], draft.relations, items => { draft.relations = items; this.changed(); this.renderControls(); }));
    this.controls.append(this.dependencyControls("providers"), this.dependencyControls("seriesPolicies"), ui("p", "hint", "Provider 与系列政策仅按本次准确指纹显式选择，不自动全选。这些选择也用于当前正文内嵌查询；保存定义的 current / pinned 选择仍独立，不会固定或改写本次 Provider 数据。"));
    const refresh = action("刷新获权 Provider 与查询目录", () => this.refreshCatalog()); refresh.disabled = this.busy; this.controls.append(refresh);
    const plan = input("托管完整查询计划（JSON）", draft.plan, true), parameters = input("托管查询参数（原生值 JSON）", draft.parameters, true); plan.control.oninput = () => { draft.plan = plan.control.value; this.changed(); this.renderScopes(); }; parameters.control.oninput = () => { draft.parameters = parameters.control.value; this.changed(); }; this.controls.append(plan.host, parameters.host); this.renderScopes(); this.controls.append(this.scopeControls);
    const visual = ui("details"); visual.dataset.queryDisclosure = "visual"; visual.append(ui("summary", "", "可视化条件与高级组合"));
    let parsed: QueryPlan | undefined; try { parsed = parseQueryPlan(draft.plan); } catch { /* Invalid source is kept verbatim. */ }
    if (parsed && !parsed.advanced && !parsed.unnest) { const proposal = structuredClone(parsed), opening = draft.plan, controls = ui("details"); controls.append(ui("summary", "", "属性筛选、排序与汇总控件"), queryPlanControls(proposal, this.catalog.formSchema ?? null, () => {}), action("采用控件提案到临时查询", () => { if (draft.plan !== opening) { this.message.textContent = "完整计划已变化，控件提案保留，未覆盖新计划。"; return; } this.writePlan(parseQueryPlan(JSON.stringify(proposal))); })); visual.append(controls); }
    const where = input("托管可视条件（CEL）", draft.visual?.where ?? parsed?.where ?? ""), columns = input("托管投影列（JSON）", draft.visual?.columns ?? JSON.stringify(parsed?.select ?? [], null, 2), true), order = input("托管排序（JSON）", draft.visual?.order ?? JSON.stringify(parsed?.order ?? [], null, 2), true);
    for (const control of [where.control, columns.control, order.control]) control.oninput = () => { draft.visual = { where: where.control.value, columns: columns.control.value, order: order.control.value }; this.changed(); };
    visual.append(where.host, columns.host, order.host, action("应用可视条件到临时计划", () => { try { const current = parseQueryPlan(draft.plan), selection = parseQueryJson(columns.control.value), sorting = parseQueryJson(order.control.value); this.writePlan(parseQueryPlan(JSON.stringify({ ...current, where: where.control.value || undefined, select: selection, order: sorting }))); } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } }));
    const combination = select("托管高级组合示例", [["union", "两个准确来源的 unionAll"], ["distinct", "按完整投影值 distinct"], ["parameters", "带原生 wanted 参数"], ["related", "沿获权关系查询"]], "union");
    visual.append(combination, action("将高级组合加入临时计划", () => { try { const current = parseQueryPlan(draft.plan), advanced = { ...current.advanced };
      if (combination.value === "union") advanced.unionAll = [structuredClone(current), structuredClone(current)];
      else if (combination.value === "distinct") advanced.distinct = structuredClone(current.select);
      else if (combination.value === "parameters") { advanced.parameters = [{ name: "wanted", shape: { kind: "scalar", semantic: "text" }, default: { kind: "text", value: "open" } }]; current.where = `(${current.where || "true"}) && status == wanted`; }
      else { const relation = draft.relations[0]; if (!relation) throw new Error("请先选择独立获权的关系。"); advanced.related = { relation, from: structuredClone(current), direction: "forward" }; }
      this.writePlan({ ...current, advanced });
    } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } }), ui("p", "hint", "示例仍是原生 Query 计划；运行由同一执行器核验完整 CEL、类型、关系和定义授权。参数、union、distinct、group、unnest 与保存定义调用可直接在完整计划中组合。"));
    this.controls.append(visual); this.renderTags(); this.renderDefinitions();
    const run = action(this.busy ? "正在运行托管 Query…" : "运行托管高级查询", () => this.run()); run.disabled = this.busy || !draft.resources.length; this.controls.append(run);
    for (const item of this.controls.querySelectorAll<HTMLDetailsElement>("details[data-query-disclosure]")) item.open = disclosures.get(item.dataset.queryDisclosure) ?? false;
    if (focus) { const restored = this.controls.querySelector<HTMLElement>(`[aria-label="${CSS.escape(focus)}"]`); restored?.focus({ preventScroll: true }); if (selection && selection.from !== null && selection.to !== null && (restored instanceof HTMLInputElement || restored instanceof HTMLTextAreaElement)) restored.setSelectionRange(selection.from, selection.to); }
  }
  private renderTags(): void {
    const section = ui("details"); section.dataset.queryDisclosure = "tags"; section.append(ui("summary", "", "标签范围 · 独立获权完整结构"));
    section.append(action("读取获权标签树", async () => { const epoch = this.epoch, serial = ++this.tagReadSerial; this.tagTree = undefined; try { const registry = await this.request<RegistryCatalog>("navigation/registry"); if (epoch !== this.epoch || serial !== this.tagReadSerial) return; this.tagTree = new TagTree(registry.tags.items, registry.tags.read && registry.tags.coverage === "complete"); this.renderControls(); } catch (error) { if (epoch === this.epoch && serial === this.tagReadSerial) this.message.textContent = String(error); } }));
    if (this.tagTree) { const chosen = this.draft.tagFilter ??= { reference: "", mode: "subtree" }; const mode = select("标签匹配模式", [["subtree", "此标签及子级"], ["exact", "仅此标签"]], chosen.mode); mode.onchange = () => { chosen.mode = mode.value as TagMatchMode; this.persist(); }; section.append(tagPicker(this.tagTree, { label: "查询标签", value: chosen.reference, changed: reference => { chosen.reference = reference ?? ""; this.persist(); } }), mode, action("将明确标签条件加入临时计划", () => { try { if (!chosen.reference || !this.tagTree?.byRef.has(chosen.reference)) throw new Error("请明确选择当前合格标签。"); const plan = parseQueryPlan(this.draft.plan); if (plan.advanced?.call || plan.advanced?.unionAll) throw new Error("保存定义调用／union 的条件须在实际扫描定义内修改，原计划保留。"); const field = this.catalog.formSchema?.fields.find(field => field.key === "tags"); if (!field) throw new Error("当前获权 Schema 没有可用的 tags 字段。"); const predicate = tagPredicate(chosen.reference, chosen.mode, `field(${JSON.stringify(field.id)})`); plan.where = plan.where ? `(${plan.where}) && (${predicate})` : predicate; this.writePlan(plan); } catch (error) { this.message.textContent = String(error); } }), ui("p", "hint", "默认包含自身及子级；模式写入原 Query 条件，保存和重开保持语义。注册表可读不授予笔记、字段或结果读取权。")); }
    this.controls.append(section);
  }
  private renderDefinitions() {
    const draft = this.draft, section = ui("details"); section.dataset.queryDisclosure = "saved"; section.append(ui("summary", "", "保存定义 · current / pinned / 调用"));
    const owners = this.catalog.resources.filter(item => item.definitions), owner = select("保存查询定义对象", owners.map(item => [item.resource, item.title || item.resource]), draft.owner); owner.onchange = () => { draft.owner = owner.value; draft.definition = ""; this.changed(); this.renderControls(); }; section.append(owner);
    const definitions = this.catalog.definitions.filter(item => item.resource === draft.owner), definition = select("保存查询定义", [["", "选择准确定义"], ...definitions.map(item => [item.key, item.label] as [string, string])], draft.definition); definition.onchange = () => { draft.definition = definition.value; const selected = definitions.find(item => item.key === definition.value); if (selected?.revision) draft.revision = selected.revision; this.changed(); }; section.append(definition);
    const selection = select("保存定义读取方式", [["current", "当前定义（每次读取当前数据）"], ["pinned", "固定定义版本（仍读取当前数据）"]], draft.selection); selection.onchange = () => { draft.selection = selection.value as Draft["selection"]; this.changed(); }; const revision = input("准确固定定义版本", draft.revision); revision.control.oninput = () => { draft.revision = revision.control.value; this.changed(); }; section.append(selection, revision.host);
    const read = action("读取获权保存定义", () => this.readDefinitions()); read.disabled = this.busy || !draft.owner; const adopt = action("以此保存定义创建临时调用", () => { try {
      if (!draft.owner || !draft.definition) throw new Error("请先选择准确保存定义。"); if (draft.selection === "pinned" && (!draft.revision || !owners.find(item => item.resource === draft.owner)?.definitionHistory)) throw new Error("固定定义需要准确版本和独立历史定义权限，不能改用当前定义。");
      const plan = basePlan(); plan.advanced = { call: { owner: draft.owner, selection: draft.selection === "pinned" ? { mode: "pinned", key: draft.definition, revision: draft.revision } : { mode: "current", key: draft.definition } } }; this.writePlan(plan);
    } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } }); section.append(read, adopt, ui("p", "hint", "读取或调用不会保存定义。现有定义的修改通过正文五档中的准确源码或查询构建器进入同一草稿；保存仍使用原保存按钮。")); this.controls.append(section);
  }
  private async readDefinitions() {
    if (this.busy) return; const epoch = this.epoch, fingerprint = this.fingerprint(), draft = structuredClone(this.draft); this.busy = true; this.renderControls();
    try { if (draft.selection === "pinned" && (!draft.definition || !draft.revision || !this.catalog.resources.find(item => item.resource === draft.owner)?.definitionHistory)) throw new Error("固定定义缺少准确 key、历史版本或独立权限。");
      const result = await this.request<{ resource: string; ownerRevision: string | null; definitions: Definition[]; coverage?: "complete" | "partial"; unavailable?: { key: string; label: string; status: "unavailable" }[] }>("query/definitions", { resource: draft.owner, ...(draft.definition ? { selection: { mode: draft.selection, key: draft.definition, ...(draft.selection === "pinned" ? { revision: draft.revision } : {}) } } : {}) });
      if (epoch !== this.epoch || this.fingerprint() !== fingerprint) return; if (result.resource !== draft.owner || !Array.isArray(result.definitions)) throw new Error("保存定义回执与原选择不一致。" );
      const incoming = result.definitions.map(item => ({ ...item, resource: result.resource, revision: result.ownerRevision ?? undefined }));
      if (draft.definition && (incoming.length !== 1 || incoming[0].key !== draft.definition)) throw new Error("保存定义回执没有保留准确 key，未采用同名替代定义。");
      // A historical read must not replace the catalog's current definition.
      if (draft.selection === "current") this.catalog.definitions = [...this.catalog.definitions.filter(item => item.resource !== result.resource || !!draft.definition && draft.definition !== item.key), ...incoming];
      this.message.textContent = `${draft.selection === "pinned" ? "准确固定" : "当前"}定义已读取 · ${result.ownerRevision ?? "版本不可用"} · ${incoming.length} 项${result.coverage === "partial" ? ` · 部分可用，${(result.unavailable ?? []).map(item => `${item.label} (${item.key})`).join("、") || "部分声明"} 尚不可作为已接纳定义执行` : ""}；未保存正文。`;
    } catch (error) { if (epoch === this.epoch && this.fingerprint() === fingerprint) this.message.textContent = error instanceof Error ? error.message : String(error); }
    finally { if (epoch === this.epoch) { this.busy = false; this.renderControls(); } }
  }
  private async run() {
    if (this.busy) return; const epoch = this.epoch, fingerprint = this.fingerprint(), draft = structuredClone(this.draft), token = this.gate.begin(fingerprint); this.busy = true; this.message.textContent = "正在按明确获权输入运行；等待期间的新计划和参数仍可编辑。"; this.renderControls();
    try { if (draft.scopeDraft) throw new Error("范围控件尚未应用到临时计划，请明确应用或放弃原范围输入后再运行。"); if (draft.resources.some(resource => !this.catalog.resources.some(item => item.resource === resource))) throw new Error("查询对象授权已变化，原输入保留，请重新选择。");
      for (const resource of draft.resources) { const scope = this.catalog.resources.find(item => item.resource === resource)!;
        if (draft.fields.some(field => !scope.fields.includes(field)) || draft.products.some(product => !scope.products[product]) || draft.relations.some(relation => !scope.relations.includes(relation))) throw new Error(`所选对象 ${resource} 没有全部请求字段、产品或关系的独立权限；请调整明确范围，原输入保留。`);
      }
      const dependencies = exactHostedDependencies(draft, this.catalog);
      const plan = parseQueryPlan(draft.plan); exactHostedScope(plan.scope, this.catalog);
      const response = await this.request<HostedQueryResponse>("query", { resources: draft.resources, fields: draft.fields, products: draft.products, relations: draft.relations, ...dependencies, parameters: parseQueryParameters(draft.parameters), plan });
      if (!this.gate.current(token, this.fingerprint())) return; this.result = decodeHostedQuery(response); this.resultInput = fingerprint; this.message.textContent = "已取得本次冻结结果；定义和个人正文草稿均未保存。";
    } catch (error) { if (this.gate.current(token, this.fingerprint())) { if (this.result) this.result = { ...this.result, superseded: true }; this.message.textContent = error instanceof Error ? error.message : String(error); } }
    finally { if (epoch === this.epoch) { this.busy = false; this.renderControls(); this.renderResults(); } }
  }
  private renderResults() {
    this.results.replaceChildren(); const result = this.result; if (!result) return;
    const current = this.resultInput === this.fingerprint() && !result.superseded;
    const status = ui("p", "hint", queryResultState(result)); status.setAttribute("role", "status"); this.results.append(status, ui("p", "hint", `范围：${result.coverage} · 当前性：${result.freshness}${result.limited ? " · 已达显示上限" : ""}${result.externalPending ? " · 外部稿待接纳" : ""}${current ? "" : " · 对应先前计划或参数"}`));
    if (result.aggregateExact === false) this.results.append(ui("p", "hint", "此汇总仅对应已知输入，不代表完整总数。"));
    for (const diagnostic of result.diagnostics) this.results.append(ui("p", "hint", diagnosticText(diagnostic)));
    this.results.append(occurrenceViewControls(this.view, result.columns, () => this.renderResults()));
    const table = ui("table"); table.setAttribute("aria-label", "托管高级查询冻结结果"); const head = ui("tr"); for (const column of result.columns) head.append(ui("th", "", column)); head.append(ui("th", "", "独立获权导航")); table.append(head);
    for (const row of result.rows) { const line = ui("tr"); for (const column of result.columns) { const cell = row.cells[column], value = ui("td", `cell-${cell?.state ?? "unprojected"}`, text(cell)); if (this.openMedia && cell) for (const reference of mediaCellReferences(cell)) { const open = action("独立查看获权资源", async () => { try { await this.openMedia!(reference); } catch (error) { this.message.textContent = error instanceof Error ? error.message : "资源当前不可用，查询输入保留。"; } }); open.disabled = !current; value.append(open); } line.append(value); } const target = ui("td"); if (row.canOpen === true && row.resource) { const open = action("打开获权正文", () => this.openRow(row)); open.disabled = !current; target.append(open); } else target.textContent = "无正文读取资格 / 派生行"; line.append(target); table.append(line); }
    if (this.view.kind === "table") this.results.append(table); else renderOccurrenceCards(this.results, result, this.view, original => { const row = original as HostedQueryRow; if (row.canOpen && row.resource) { const open = action("打开获权正文", () => this.openRow(row)); open.disabled = !current; return open; } return ui("span", "", "无正文读取资格 / 派生行"); });
    if (this.view.kind === "table" && this.view.columns.length) { [...table.querySelectorAll("tr")].forEach(line => { [...line.children].forEach((cell, index) => { if (index < result.columns.length && !this.view.columns.includes(result.columns[index])) (cell as HTMLElement).hidden = true; }); }); }
    if (current && result.freshness === "current" && this.fieldActions) { let plan: QueryPlan | null = null; try { plan = parseQueryPlan(this.draft.plan); } catch {} for (const row of result.rows) { if (row.readOnly || !row.resource || row.lifecycle === "trash") continue; for (const column of result.columns) { const field = directQueryField(plan, this.catalog.formSchema ?? null, column); if (field && this.fieldActions.canEdit(row.resource, field.id)) this.results.append(action(`编辑 ${row.resource} 的${fieldNames[field.key] ?? field.key}（独立字段权限）`, () => this.fieldActions!.open(row.resource!, field.id))); } } }
    if (result.rows.some(row => row.lifecycle === "trash")) this.results.append(ui("p", "hint", "本次含回收站来源，只读结果不提供恢复、删除或正文写入资格。")); if (!result.rows.length) this.results.append(ui("p", "hint", current && result.coverage === "complete" && result.freshness === "current" && !result.limited ? "本次明确范围内的完整空结果。" : "已知范围没有结果，不能断言完整为空。"));
    const output = action("将原冻结查询交给 Office", () => this.output(result.queryId, { columns: result.columns, rowCount: result.rows.length, coverage: result.coverage })); output.disabled = !current || this.busy; this.results.append(output);
  }
  private async openRow(row: HostedQueryRow) { if (row.canOpen !== true || !row.resource || !this.result || this.result.superseded || this.resultInput !== this.fingerprint()) return; try { await this.open(row.resource); } catch (error) { this.message.textContent = error instanceof Error ? error.message : String(error); } }
}
