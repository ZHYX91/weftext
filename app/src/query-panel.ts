import { action, cellLabels, decodeProjection, decodeSourcePatch, diagnosticText, fieldNames, formatNativeValue, nativeMember, nativeFormControls, QuickCapture, resolveShape, scalarFromInput, typeNames, ui, unionVariant, valueEditor, loadTagFormReferences, type FormReferences, type Cell, type DataSchema, type DocumentBasis, type NativeValue, type NoteProjection, type PanelHost, type Shape, type SourcePatch, type SchemaField } from "./data-panel";
import { mountListThumbnail, type ListPreview } from "./list-thumbnail";
import type { OpenedDocument } from "./types";
import type { FrozenQuery } from "./export-state";
import { TagTree, tagPredicate, decodeTagShortcutCollection, type TagMatchMode } from "./tag-tree";
import { tagPicker } from "./tag-picker";
import { tagCancellationStatus, tagOperationStatus } from "./tag-navigation-state";

export interface QueryPlan {
  scope: { directory?: string; descendants: boolean; lifecycle: "live" | "trash" };
  types: string[];
  where?: string;
  select: { name: string; expr: string }[];
  order: { expr: string; descending: boolean; nulls: "first" | "last" }[];
  take?: number;
  group?: { keys: { name: string; expr: string }[]; aggregates: { name: string; op: "count" | "sum" | "min" | "max"; expr?: string }[] };
  unnest?: { array: string; binding: string };
  advanced?: {
    parameters?: { name: string; shape: Shape; nullable?: boolean; default?: NativeValue }[];
    unionAll?: QueryPlan[];
    distinct?: { name: string; expr: string }[];
    related?: { relation: string; from: QueryPlan; direction?: "forward" | "inverse" };
    call?: { owner: string; selection: { mode: "current"; key: string } | { mode: "pinned"; key: string; revision: string }; arguments?: Record<string, string> };
  };
}

/** Reject duplicate JSON members before parsing: a plan editor must not silently choose the last value. */
export function parseQueryJson(source: string): unknown {
  const parsed: unknown = JSON.parse(source); let position = 0;
  const whitespace = (): void => { while (/\s/.test(source[position] ?? "") && position < source.length) position++; };
  const string = (): string => { const start = position++; while (position < source.length) { const char = source[position++]; if (char === "\\") position++; else if (char === '"') return JSON.parse(source.slice(start, position)) as string; } throw new Error("JSON 字符串不完整。"); };
  const value = (depth: number): void => {
    if (depth > 100) throw new Error("查询计划嵌套过深。"); whitespace(); const char = source[position];
    if (char === '"') { string(); return; }
    if (char === "{") { position++; whitespace(); const keys = new Set<string>(); if (source[position] === "}") { position++; return; } for (;;) { whitespace(); const key = string(); if (keys.has(key)) throw new Error(`JSON 成员重复：${key}`); keys.add(key); whitespace(); position++; value(depth + 1); whitespace(); if (source[position++] === "}") return; } }
    if (char === "[") { position++; whitespace(); if (source[position] === "]") { position++; return; } for (;;) { value(depth + 1); whitespace(); if (source[position++] === "]") return; } }
    while (position < source.length && !/[\s,}\]]/.test(source[position])) position++;
  };
  value(0); return parsed;
}

/** Structural checks only; the shared compiler owns CEL, schema and producer validation. */
export function parseQueryPlan(source: string): QueryPlan {
  const parsed = parseQueryJson(source); let nodes = 0;
  const visit = (value: unknown, depth: number): QueryPlan => {
    if (++nodes > 1024 || depth > 64) throw new Error("查询计划超过嵌套或节点预算。");
    if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("查询计划必须是对象。"); const plan = value as Record<string, unknown>;
    const allowed = ["scope", "types", "where", "select", "order", "take", "group", "unnest", "advanced"]; const unknown = Object.keys(plan).find(key => !allowed.includes(key)); if (unknown) throw new Error(`未知查询成员：${unknown}`);
    const scope = plan.scope ?? { descendants: true, lifecycle: "live" }; if (!scope || typeof scope !== "object" || Array.isArray(scope)) throw new Error("scope 必须是对象。"); const actualScope = scope as Record<string, unknown>;
    if (Object.keys(actualScope).some(key => !["directory", "descendants", "lifecycle"].includes(key)) || actualScope.directory != null && typeof actualScope.directory !== "string" || actualScope.descendants != null && typeof actualScope.descendants !== "boolean" || actualScope.lifecycle != null && !["live", "trash"].includes(String(actualScope.lifecycle))) throw new Error("scope 范围成员无效。");
    if (plan.types != null && (!Array.isArray(plan.types) || plan.types.some(item => typeof item !== "string"))) throw new Error("types 必须是类型字符串数组。");
    for (const key of ["select", "order"]) if (plan[key] != null && (!Array.isArray(plan[key]) || (plan[key] as unknown[]).some(item => !item || typeof item !== "object" || typeof (item as Record<string, unknown>).expr !== "string"))) throw new Error(`${key} 必须是表达式数组。`);
    if (plan.where != null && typeof plan.where !== "string" || plan.take != null && (!Number.isSafeInteger(plan.take) || Number(plan.take) < 1)) throw new Error("where 或 take 无效。");
    const advanced = plan.advanced as QueryPlan["advanced"]; if (advanced != null) { if (typeof advanced !== "object" || Array.isArray(advanced)) throw new Error("advanced 必须是对象。"); if (advanced.unionAll) { if (!Array.isArray(advanced.unionAll) || !advanced.unionAll.length) throw new Error("unionAll 必须包含来源计划。"); advanced.unionAll.forEach(branch => visit(branch, depth + 1)); } if (advanced.related) visit(advanced.related.from, depth + 1); }
    return { ...plan, scope: { ...actualScope, descendants: actualScope.descendants ?? true, lifecycle: actualScope.lifecycle ?? "live" }, types: plan.types ?? [], select: plan.select ?? [], order: plan.order ?? [] } as QueryPlan;
  }; return visit(parsed, 0);
}

export function parseQueryParameters(source: string): Record<string, NativeValue> {
  const parsed = parseQueryJson(source); if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) throw new Error("参数必须是名称到原生值的对象。");
  const check = (value: unknown, depth = 0): void => { if (depth > 64 || !value || typeof value !== "object" || Array.isArray(value)) throw new Error("参数原生值无效。"); const native = value as Record<string, unknown>; if (native.kind === "none" && Object.keys(native).length === 1) return; if (native.kind === "array" && Array.isArray(native.value)) { native.value.forEach(item => check(item, depth + 1)); return; } if (native.kind === "record" && Array.isArray(native.value)) { const keys = new Set<string>(); for (const item of native.value) { if (!item || typeof item.key !== "string" || keys.has(item.key)) throw new Error("参数记录成员重复或无效。"); keys.add(item.key); check(item.value, depth + 1); } return; } if (native.kind === "bool" && typeof native.value === "boolean" || ["text", "int", "float", "decimal", "date", "time", "localDatetime", "duration"].includes(String(native.kind)) && typeof native.value === "string") return; throw new Error("参数须使用带 kind 的原生值；准确数字使用字符串。"); };
  Object.values(parsed).forEach(value => check(value)); return parsed as Record<string, NativeValue>;
}

interface PeriodLookup { period: NativeValue; bounds: NativeValue; policy: { multiplicity: string; scope?: { kind?: string } }; matches: QueryRow[]; coverage: "complete" | "partial"; canCreate: boolean; cutId?: string; diagnostics?: unknown[] }
/** Period creation is a separate explicit Core action after actual resolution. */
class PeriodNavigator {
  readonly element = ui("details", "query-period-navigation");
  private readonly series = ui("input", "data-input");
  private readonly kind = selectControl([["day", "日"], ["week", "周"], ["month", "月"], ["quarter", "季度"], ["year", "年"]], "day");
  private readonly key = ui("input", "data-input");
  private readonly resultHost = ui("div", "period-lookup-result");
  private readonly parent = ui("input", "data-input");
  private readonly name = ui("input", "data-input");
  private readonly title = ui("input", "data-input");
  private readonly body = ui("textarea", "data-input");
  private readonly createButton: HTMLButtonElement;
  private result: PeriodLookup | null = null;
  private pending: Record<string, unknown> | null = null;
  private loading = false;
  private lookupGeneration = 0;
  constructor(private readonly host: PanelHost, private root: string, scope: () => string) {
    this.element.append(ui("summary", "", "周期笔记 · 先找已有对象")); this.series.placeholder = "系列 key，例如 work-log"; this.series.setAttribute("aria-label", "周期系列准确标识"); this.key.placeholder = "准确周期 key，例如 2026-10-13 或 2026-W42"; this.key.setAttribute("aria-label", "准确周期 key");
    const lookup = ui("form", "period-lookup-form"); const submit = ui("button", "data-button", "查找已有周期笔记"); submit.type = "submit"; lookup.append(labeled("系列", this.series), labeled("周期种类", this.kind), labeled("周期 key", this.key), submit, action("上一周期", () => this.lookup(-1), "data-button compact"), action("下一周期", () => this.lookup(1), "data-button compact")); lookup.onsubmit = event => { event.preventDefault(); void this.lookup(); };
    this.element.append(lookup, ui("p", "data-help", "日期与周期 key 必须明确输入，不补本机日期、时区或未知精度。查找范围由系列策略决定。"), this.resultHost);
    const creation = ui("details", "period-create-form"); creation.append(ui("summary", "", "明确创建新的周期笔记")); this.parent.value = scope(); this.parent.setAttribute("aria-label", "周期笔记保存位置"); this.name.placeholder = "笔记文件夹名称"; this.name.setAttribute("aria-label", "新周期笔记名称"); this.title.setAttribute("aria-label", "周期笔记可选作者标题"); this.body.rows = 4; this.body.setAttribute("aria-label", "周期笔记原生 Typst 正文"); this.createButton = action("创建周期笔记并打开", () => this.create(), "data-button primary"); this.createButton.disabled = true;
    creation.append(labeled("保存位置", this.parent), labeled("文件夹名称 *", this.name), labeled("作者标题（可选）", this.title), labeled("正文（原生 Typst）", this.body), this.createButton, ui("p", "data-help", "唯一系列仅在准确完整范围没有已有对象时开放创建；Core 在提交时重新核对完整范围与并发唯一性。")); this.element.append(creation);
    for (const input of [this.series, this.key, this.kind]) input.addEventListener("change", () => { this.lookupGeneration++; this.loading = false; this.result = null; this.createButton.disabled = !this.pending; this.resultHost.replaceChildren(ui("p", "data-help", "条件已变化，请重新查找。")); });
  }
  private async lookup(steps = 0): Promise<void> {
    if (this.loading) return; if (!this.series.value || !this.key.value) { this.host.notify("请输入准确系列与周期 key。", true); return; }
    const generation = ++this.lookupGeneration; const basis = { seriesKey: this.series.value, kind: this.kind.value, key: this.key.value }; this.loading = true; this.createButton.disabled = true; this.resultHost.replaceChildren(ui("p", "data-help", "正在解析准确周期并查找已有对象…"));
    try {
      const result = await this.host.request<PeriodLookup>("/api/data/period/resolve", { root: this.root, ...basis, ...(steps ? { steps } : {}) }); if (generation !== this.lookupGeneration || this.series.value !== basis.seriesKey || this.kind.value !== basis.kind || this.key.value !== basis.key) return; this.result = result; const actualKey = nativeMember(result.period, "key"); if (actualKey?.kind === "text") this.key.value = actualKey.value;
      this.resultHost.replaceChildren(ui("p", "data-help", `准确周期：${formatNativeValue(result.period)}`), ui("p", "data-help", `期间：${formatNativeValue(result.bounds)}`), ui("p", result.coverage === "complete" ? "query-result-status" : "data-warning", `${result.coverage === "complete" ? "已纳管冻结范围完整" : "已知部分范围"} · ${result.matches.length} 个已有对象 · ${result.policy.multiplicity === "unique" ? "唯一系列" : "允许多个对象的系列"}`));
      for (const row of result.matches) this.resultHost.append(action(rowTitle(row), async () => { try { await this.host.prepareAction?.(); if (row.entityKey) { if (!this.host.openResolved) throw new Error("当前会话缺少稳定目标打开入口。"); const opened = await this.host.request<OpenedDocument>("/api/data/open", { root: this.root, entityKey: row.entityKey }); await this.host.openResolved(opened); } else if (row.path) await this.host.openDocument(row.path, this.root); else throw new Error("已有周期目标不可用。"); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); } }, "query-row-title"));
      result.diagnostics?.forEach(item => this.resultHost.append(ui("p", "data-warning", diagnosticText(item)))); if (!result.matches.length) this.resultHost.append(ui("p", "data-help", result.coverage === "complete" ? "本次准确范围未找到已有周期笔记。新建仍需下方明确动作。" : "已知范围未找到对象，尚不能断言不存在。")); this.createButton.disabled = !result.canCreate && !this.pending;
    } catch (error) { if (generation === this.lookupGeneration) { this.result = null; this.resultHost.replaceChildren(ui("p", "data-warning", error instanceof Error ? error.message : String(error))); this.createButton.disabled = !this.pending; } }
    finally { if (generation === this.lookupGeneration) this.loading = false; }
  }
  private async create(): Promise<void> {
    if ((!this.result?.canCreate && !this.pending) || this.loading) return; if (!this.name.value || !this.parent.value) { this.host.notify("请输入新周期笔记的名称与保存位置。", true); return; }
    const actualKey = this.result ? nativeMember(this.result.period, "key") : undefined; this.pending ??= { root: this.root, parent: this.parent.value, name: this.name.value, seriesKey: this.series.value, kind: this.kind.value, key: actualKey?.kind === "text" ? actualKey.value : this.key.value, title: this.title.value || undefined, body: this.body.value, operationId: crypto.randomUUID() }; const submitted = this.pending; let receiptReceived = false; this.loading = true; this.createButton.disabled = true;
    try {
      const result = await this.host.request<{ path?: string; root?: string; sessionId?: string }>("/api/data/period/create", this.pending); if (!result.path || !result.sessionId) throw new Error("周期创建未返回真实对象，输入与原操作仍保留。"); receiptReceived = true;
      if (this.host.created) await this.host.created(result); else await this.host.openDocument(result.path, result.root ?? this.root); const unchanged = this.name.value === submitted.name && this.parent.value === submitted.parent && (this.title.value || undefined) === submitted.title && this.body.value === submitted.body && this.series.value === submitted.seriesKey && this.kind.value === submitted.kind && this.key.value === submitted.key; this.pending = null; if (unchanged) { this.name.value = ""; this.title.value = ""; this.body.value = ""; } this.host.notify(unchanged ? "已创建并打开准确周期笔记。" : "提交的周期笔记已创建并打开。等待期间的新输入仍保留。"); this.loading = false; await this.lookup();
    } catch (error) { const code = error && typeof error === "object" && "code" in error ? String(error.code) : "unknown"; if (!receiptReceived && !["unknown", "pending", "resultUnknown"].includes(code)) { this.pending = null; this.result = null; } this.resultHost.append(ui("p", "data-warning", `${error instanceof Error ? error.message : String(error)}${this.pending ? ` 原操作保留（${String(this.pending.seriesKey)} / ${String(this.pending.key)}）；重试确认同一对象。` : " 原输入保留，请重新查找后再创建。"}`)); }
    finally { this.loading = false; this.createButton.disabled = !this.pending && !this.result?.canCreate; this.createButton.textContent = this.pending ? "确认上次周期创建并打开" : "创建周期笔记并打开"; }
  }
}
interface TagRegistry { tags: { tagRef: string; tagId: string; displayName: string; parentTagId: string | null }[]; baseId: string; complete: boolean; warnings?: unknown[] }
interface NavigationPreparation { targetRef: string; preview: { operationId: string; planId: string; kind?: string; changes?: unknown[] } }
class NavigationControls {
  readonly element = ui("details", "query-registered-navigation");
  private readonly tagsHost = ui("div", "registered-tag-list");
  private readonly status = ui("p", "data-help");
  private readonly createName = ui("input", "data-input");
  private readonly parentHost = ui("div");
  private parentRef: string | null = null;
  private tree: TagTree | null = null;
  private search = "";
  private offset = 0;
  private expanded = new Set<string>();
  private recoveryError?: string;
  private busy = false;
  private registryRead = 0;
  private registry: TagRegistry | null = null;
  private pending: { route: string; body: Record<string, unknown>; prepared?: NavigationPreparation; addKind?: "directory" | "tag"; label: string; confirmed?: boolean; context?: DocumentBasis; cancelRequested?: boolean; recoveredPlanId?: string } | null = null;
  constructor(private host: PanelHost, private root: string, private workspaceId: string, private scopePath: () => string, private chooseTag: (ref: string, label: string, mode: TagMatchMode) => void, private saveShortcut: (ref: string, label: string, kind: "directory" | "tag", matchMode?: TagMatchMode) => void, private tagField: () => string | undefined, private registryChanged: () => void) {
    this.element.append(ui("summary", "", "登记标签与目录入口"), action("读取登记标签", () => this.loadTags()), action("把当前目录添加为快捷入口", () => this.addDirectory()));
    try { const raw = localStorage.getItem(this.pendingKey()); if (raw) { const saved = JSON.parse(raw); if (!saved || !["/api/navigation/create-tag", "/api/navigation/rename-tag", "/api/navigation/move-tag", "/api/navigation/admit-directory"].includes(saved.route) || saved.body?.root !== root || typeof saved.body.operationId !== "string" || typeof saved.label !== "string" || saved.prepared && (saved.prepared.preview?.operationId !== saved.body.operationId || typeof saved.prepared.preview?.planId !== "string") || saved.recoveredPlanId !== undefined && (typeof saved.recoveredPlanId !== "string" || !saved.recoveredPlanId || saved.prepared && saved.prepared.preview.planId !== saved.recoveredPlanId)) throw new Error("导航恢复记录无效，请保留原记录。"); this.pending = saved; } } catch (error) { this.recoveryError = String(error); this.status.textContent = this.recoveryError; }
    this.createName.placeholder = "新标签显示名称"; this.createName.setAttribute("aria-label", "新标签显示名称"); this.element.append(this.tagsHost, this.createName, this.parentHost, action("新建登记标签", () => this.createTag(false)), action("新建并赋给当前已保存笔记", () => this.createTag(true)), action("确认上次导航登记操作", () => this.resume()), action("取消原未提交导航操作", async () => { if (this.busy || !this.pending) return; this.pending.cancelRequested = true; this.persistPending(); await this.resume(); }), this.status, ui("p", "data-help", "完整标签树独立于笔记范围；树项默认含此标签及子级。赋值只写直接标签。名称中的斜杠不拆层级，入口保存明确筛选模式。"));
  }
  private async createTag(assign: boolean): Promise<void> {
    try {
      if (!this.createName.value) return; if (!this.registry) await this.loadTags(); if (!this.registry || !this.tree) return;
      const basis = assign ? this.host.getDocument() : null, fieldId = this.tagField();
      if (assign && (!basis || basis.root !== this.root || basis.dirty !== false || this.host.canEdit?.() === false || !fieldId || !this.host.applyOperationReceipt)) throw new Error("首次赋值需要当前可写、已保存的准确笔记。请先明确保存草稿；本操作不会自动保存整篇草稿。");
      await this.mutate("/api/navigation/create-tag", { displayName: this.createName.value, parentRef: this.parentRef, baseId: this.registry.baseId, ...(assign && basis ? { assignment: { path: basis.path, baseId: basis.baseId, fieldId } } : {}) }, this.createName.value, "tag", basis ?? undefined);
    } catch (error) { this.status.textContent = String(error); }
  }
  private pendingKey(): string { return `weftext-navigation-operation:${JSON.stringify([this.workspaceId, this.root])}`; }
  private persistPending(): void { if (this.pending) localStorage.setItem(this.pendingKey(), JSON.stringify(this.pending)); else localStorage.removeItem(this.pendingKey()); }
  private async loadTags(): Promise<void> {
    const read = ++this.registryRead;
    this.status.textContent = "正在读取实际标签注册表…";
    this.tree = null; this.createName.oninput = null; this.parentHost.replaceChildren(); this.tagsHost.replaceChildren();
    try { const registry = await this.host.request<TagRegistry>("/api/navigation/tags", { root: this.root }); if (read !== this.registryRead) return; this.registry = registry; this.tree = new TagTree(registry.tags.map(tag => ({ ...tag, reference: tag.tagRef })), registry.complete === true); this.renderTags(); this.parentHost.append(tagPicker(this.tree, { label: "新标签父级", value: this.parentRef ?? "", root: true, changed: ref => { this.parentRef = ref; } })); const duplicates = ui("div", "data-help"); const showNames = () => { duplicates.replaceChildren(); const same = this.registry!.tags.filter(tag => tag.displayName === this.createName.value); if (same.length) duplicates.append(ui("p", "", `已有 ${same.length} 个同名标签。请核对，仍创建将产生新身份。`), ...same.slice(0, 100).map(tag => ui("p", "", this.tree!.label(tag.tagRef)))); }; this.createName.oninput = showNames; showNames(); this.parentHost.append(duplicates); this.status.textContent = this.recoveryError ?? "已读取完整标签结构；笔记结果仍按自己的获权范围判断。";
    } catch (error) { if (read !== this.registryRead) return; this.registry = null; this.tagsHost.replaceChildren(); this.status.textContent = `标签注册表不可用：${error instanceof Error ? error.message : String(error)}`; }
  }
  private renderTags(): void {
    const tree = this.tree; this.tagsHost.replaceChildren(); if (!tree) return;
    const search = ui("input", "data-input"); search.type = "search"; search.value = this.search; search.setAttribute("aria-label", "标签树搜索名称或身份"); const rows = ui("div");
    const render = () => { rows.replaceChildren(); const page = this.search ? tree.page(this.search, this.offset) : tree.visible(this.expanded, this.offset); this.offset = page.offset; rows.append(ui("p", "data-help", `完整结构 ${tree.byId.size} 项 · ${this.search ? "搜索命中" : "已展开"} ${page.total} 项 · 当前 ${page.total ? page.offset + 1 : 0}–${page.offset + page.items.length} · 每段最多 100`));
      for (const tag of page.items) {
        const row = ui("div", "registered-tag-row"), label = tree.label(tag.reference); row.setAttribute("role", "group"); row.setAttribute("aria-label", label);
        if (tree.children.get(tag.tagId)?.length) {
          const expand = action(this.expanded.has(tag.tagId) ? "收起子级" : "展开子级", () => { if (this.expanded.has(tag.tagId)) this.expanded.delete(tag.tagId); else this.expanded.add(tag.tagId); render(); rows.querySelector<HTMLButtonElement>(`[data-expand-tag="${CSS.escape(tag.tagId)}"]`)?.focus({ preventScroll: true }); }, "data-button compact"); expand.dataset.expandTag = tag.tagId; expand.setAttribute("aria-expanded", String(this.expanded.has(tag.tagId))); row.append(expand);
        }
        row.append(action(`${label} · 此标签及子级`, () => this.chooseTag(tag.reference, label, "subtree"), "query-facet"), action("仅此标签", () => this.chooseTag(tag.reference, label, "exact"), "data-button compact"), action("保存含子级入口", () => this.saveShortcut(tag.reference, label, "tag", "subtree"), "data-button compact"), action("保存精确入口", () => this.saveShortcut(tag.reference, label, "tag", "exact"), "data-button compact"), action("改名", async () => { const name = window.prompt("标签的新显示名称", tag.displayName); if (name == null || name === tag.displayName || !name) return; await this.mutate("/api/navigation/rename-tag", { tagRef: tag.reference, displayName: name, baseId: this.registry!.baseId }, name); }, "data-button compact"), action("换父／移至根", () => this.move(tag.reference), "data-button compact"));
        const full = ui("details"); full.append(ui("summary", "", "完整分段路径")); full.ontoggle = () => { if (full.open && full.childElementCount === 1) full.append(ui("p", "", tree.label(tag.reference, true))); }; row.append(full); rows.append(row);
      }
      const focusPage = (direction: string) => (rows.querySelector<HTMLButtonElement>(`.tag-page-${direction}:not(:disabled)`) ?? rows.querySelector<HTMLButtonElement>(".tag-page-previous:not(:disabled), .tag-page-next:not(:disabled)") ?? search).focus({ preventScroll: true });
      const previous = action("上一段标签", () => { this.offset -= 100; render(); focusPage("previous"); }, "data-button tag-page-previous"), next = action("下一段标签", () => { this.offset += 100; render(); focusPage("next"); }, "data-button tag-page-next"); previous.disabled = page.offset === 0; next.disabled = page.offset + page.items.length >= page.total; rows.append(previous, next);
    }; search.oninput = () => { this.search = search.value; this.offset = 0; render(); }; this.tagsHost.append(search, rows); render();
  }
  private async move(reference: string): Promise<void> {
    const tree = this.tree, tag = tree?.byRef.get(reference); if (!tree || !tag || !this.registry) return;
    const parent = tag.parentTagId === null ? null : tree.byId.get(tag.parentTagId)!.reference;
    const chosen = await new Promise<{ parent: string | null } | null>(resolve => { const dialog = ui("dialog", "data-confirm-dialog"), form = ui("form"); form.method = "dialog"; let next = parent; form.append(ui("h2", "", "标签换父／移至根"), ui("p", "", tree.label(reference)), tagPicker(tree, { label: "新父标签", value: parent ?? "", root: true, excludedRoot: tag.tagId, changed: ref => { next = ref; } })); const cancel = ui("button", "data-button", "取消"); cancel.value = "cancel"; const prepare = ui("button", "data-button primary", "预览换父"); prepare.value = "prepare"; form.append(cancel, prepare); dialog.append(form); document.body.append(dialog); dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "prepare"; dialog.remove(); resolve(accepted ? { parent: next } : null); }, { once: true }); dialog.showModal(); });
    if (chosen && chosen.parent !== parent) await this.mutate("/api/navigation/move-tag", { tagRef: reference, parentRef: chosen.parent, baseId: this.registry.baseId }, tree.label(reference));
  }
  private async addDirectory(): Promise<void> {
    try {
      const path = this.scopePath(); const target = await this.host.request<NavigationTarget>(path.startsWith("weftext-directory://") || path.startsWith("weftext-node://") ? "/api/navigation/resolve" : "/api/navigation/capture", { root: this.root, ...(path.startsWith("weftext-directory://") || path.startsWith("weftext-node://") ? { targetRef: path } : { path }) });
      if (target.status === "managed" && target.targetRef) { this.saveShortcut(target.targetRef, target.label, "directory"); this.status.textContent = "目录快捷入口已添加。"; return; }
      if (target.status !== "unmanaged" || !target.path) throw new Error(`目录目标${target.status === "missing" ? "缺失" : "身份不可用"}`);
      await this.mutate("/api/navigation/admit-directory", { path: target.path }, target.label, "directory");
    } catch (error) { this.status.textContent = error instanceof Error ? error.message : String(error); }
  }
  private async mutate(route: string, payload: Record<string, unknown>, label: string, addKind?: "directory" | "tag", context?: DocumentBasis): Promise<void> {
    if (this.recoveryError) { this.status.textContent = this.recoveryError; return; }
    if (this.pending) { await this.resume(); return; } this.pending = { route, body: { root: this.root, ...payload, operationId: crypto.randomUUID() }, label, addKind, context }; try { this.persistPending(); await this.resume(); } catch (error) { this.status.textContent = `原操作记录无法可靠保存，未发送新操作：${String(error)}`; }
  }
  private async resume(): Promise<void> {
    if (this.busy) return;
    if (!this.pending) { this.status.textContent = this.recoveryError ?? "目前没有结果待确认的导航登记操作。"; return; } const pending = this.pending; this.busy = true;
    try {
      this.persistPending(); // A retry must satisfy the same durable-before-effect boundary.
      let recovering = !!pending.confirmed;
      if (!pending.cancelRequested) {
        if (!pending.prepared) { pending.prepared = await this.host.request<NavigationPreparation>(pending.route, structuredClone(pending.body)); if (pending.prepared.preview.operationId !== pending.body.operationId) throw new Error("准备结果不属于原操作。"); this.persistPending(); }
        if (!pending.confirmed) { if (!await this.confirm(pending)) pending.cancelRequested = true; else pending.confirmed = true; this.persistPending(); }
      }
      if (pending.cancelRequested) {
        const expected = { operationId: String(pending.body.operationId), planId: pending.prepared?.preview.planId ?? pending.recoveredPlanId ?? null };
        const cancelled = await this.host.request<{ operationId: string; planId: string | null; status: string }>("/api/navigation/cancel", { root: this.root, ...expected });
        if (tagCancellationStatus(expected, cancelled) === "resolved") { this.pending = null; this.persistPending(); this.status.textContent = "原操作已在服务端实际结束；原输入与现有入口保留。"; return; }
        if (typeof cancelled.planId !== "string") throw new Error("原已提交操作缺少准确计划编号，记录保留。");
        pending.recoveredPlanId = cancelled.planId; this.persistPending(); recovering = true;
      }
      const planId = pending.prepared?.preview.planId ?? pending.recoveredPlanId;
      if (!planId) throw new Error("原导航操作没有准确计划编号，记录保留。");
      const expected = { operationId: String(pending.body.operationId), planId }; let committed = false, receipt: { operationId?: string; planId?: string; status?: string; receipt?: { status?: string } } | undefined;
      if (recovering) { receipt = await this.host.request("/api/operations/status", { operationId: expected.operationId, context: pending.context }); committed = tagOperationStatus(expected, receipt!) === "committed"; }
      if (pending.cancelRequested && !committed) throw new Error("原取消报告已提交，但回读未确认提交；记录保留，未再次执行原计划。");
      if (!committed) { receipt = await this.host.request("/api/operations/commit", { ...expected, context: pending.context }); const status = receipt?.receipt?.status ?? receipt?.status; if (!status || status !== "committed") throw new Error(`导航登记尚未确认：${status ?? "未知"}`); }
      if (pending.context) {
        await this.host.applyOperationReceipt?.(receipt, pending.context);
        const current = this.host.getDocument();
        if (current && current.root === pending.context.root && current.path === pending.context.path && current.sessionId !== pending.context.sessionId) this.host.notify("原标签创建与首次赋值已提交；笔记会话已重开，当前草稿和撤销记录保留。请比较或重新打开原笔记确认磁盘结果。");
      }
      if (!pending.route.endsWith("admit-directory")) this.registryChanged();
      if (pending.addKind && pending.prepared) this.saveShortcut(pending.prepared.targetRef, pending.label, pending.addKind, pending.addKind === "tag" ? "subtree" : undefined);
      else if (pending.addKind) this.host.notify("原操作已提交。首次准备回执未保留，未猜测导航目标；请从完整登记目录选择并添加入口。");
      if (pending.route.endsWith("create-tag") && this.createName.value === pending.body.displayName) this.createName.value = ""; this.pending = null; this.persistPending(); this.status.textContent = "导航登记已完成。"; await this.loadTags();
    } catch (error) { let persistence = ""; try { this.persistPending(); } catch (storageError) { persistence = ` 恢复记录目前不可写：${String(storageError)}`; } this.status.textContent = `${error instanceof Error ? error.message : String(error)} 原操作与输入保留，可确认同一操作。${persistence}`; }
    finally { this.busy = false; }
  }
  private async confirm(pending: NonNullable<NavigationControls["pending"]>): Promise<boolean> {
    return await new Promise(resolve => {
      const dialog = ui("dialog", "data-confirm-dialog"), form = ui("form"); form.method = "dialog";
      const title = pending.route.endsWith("admit-directory") ? "登记目录导航身份" : pending.route.endsWith("rename-tag") ? "修改登记标签名称" : pending.route.endsWith("move-tag") ? "标签换父／移至根" : pending.context ? "新建标签并首次赋值" : "登记新标签";
      form.append(ui("h2", "", title), ui("p", "", pending.label));
      if (pending.route.endsWith("admit-directory")) form.append(ui("p", "data-help", "为选定目录登记稳定导航身份，并添加本机快捷入口。"));
      if (pending.context) form.append(ui("p", "data-help", `同时将新标签直接赋给已保存笔记：${pending.context.path}。新建与首次赋值为同一原计划，取消会一起取消。`));
      const preview = ui("details"); preview.append(ui("summary", "", "查看准确登记计划"), ui("pre", "", JSON.stringify(pending.prepared?.preview, null, 2))); form.append(preview);
      const cancel = ui("button", "data-button", "取消"); cancel.value = "cancel"; const commit = ui("button", "data-button primary", "执行已预览的登记"); commit.value = "apply"; form.append(cancel, commit); dialog.append(form); document.body.append(dialog);
      dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "apply"; dialog.remove(); resolve(accepted); }, { once: true }); dialog.showModal();
    });
  }
}

export interface QueryRow { key: string; entityKey?: string; path?: string; revision?: string; lifecycle?: "live" | "trash"; cells: Record<string, Cell>; readOnly: boolean; preview?: { state: "absent" | "building" | "ready" | "failed" | "unsupported" | "unavailable"; url?: string; message?: string } }
export function queryRowLifecycle(row: QueryRow, plan: QueryPlan | null): "live" | "trash" | null { if (row.lifecycle === "live" || row.lifecycle === "trash") return row.lifecycle; if (plan?.advanced?.unionAll || plan?.advanced?.related || plan?.advanced?.call) return null; return plan?.scope.lifecycle ?? null; }
export interface QueryResult { rows: QueryRow[]; columns?: string[]; coverage: "complete" | "partial"; freshness: "current" | "stale"; diagnostics: unknown[]; totalKnown: number; limited: boolean; queryId?: string; cutId?: string; schemaRevision?: string; facets?: QueryFacet[]; snapshotMode?: string; snapshotLabel?: string; externalPending?: boolean; aggregateExact?: boolean; rawScope?: { complete?: boolean; pendingCount?: number }; superseded?: boolean }
export interface QueryFacet { fieldId: string; key: string; label?: string; value?: NativeValue; expr?: string; count: number; missing?: boolean }
export type ResultView = "list" | "table" | "board" | "calendar" | "chart";
interface Condition { id: string; label: string; expr: string; tag?: { reference: string; label: string; mode: TagMatchMode } }
export interface ViewPreference { view: ResultView; density: "compact" | "comfortable"; summaries: boolean; thumbnails: boolean; columns: string[]; pinned: string[]; sort: string; descending: boolean; groupField: string; dateField: string; chartField: string; fieldBindings?: Record<string, string> }
interface Shortcut { id: string; label: string; kind: "query" | "node" | "savedQuery" | "directory" | "tag"; plan?: QueryPlan; entityKey?: string; path?: string; targetRef?: string; definition?: SavedSelection; matchMode?: TagMatchMode }
interface NavigationTarget { targetRef?: string | null; path?: string | null; status: string; kind: string; label: string; displayName?: string }
interface SavedQuery { key: string; label: string; plan: QueryPlan }
interface SavedDefinitions { definitions: SavedQuery[]; ownerRevision: string | null; unavailable?: unknown[] }
interface SavedSelection { owner: string; key: string; mode: "current" | "pinned"; revision?: string; label: string }
type RowPreview = ListPreview;

export const defaultPreference = (): ViewPreference => ({ view: "list", density: "comfortable", summaries: true, thumbnails: false, columns: ["status", "due", "project", "tags"], pinned: [], sort: "title", descending: false, groupField: "status", dateField: "due", chartField: "count" });
export function queryResultState(result: QueryResult): string {
  if (result.superseded) return "先前读取的结果 · 刷新尚未成功 / 草稿已变化";
  if (result.snapshotMode === "admittedFrozen") return `${result.coverage === "complete" ? "已纳管冻结快照完整" : "已纳管冻结快照部分"} · ${result.totalKnown} 项${result.limited ? ` · 显示 ${result.rows.length} 项` : ""}${result.freshness === "stale" ? " · 当前性未持续确认" : ""}${result.coverage === "complete" && !result.totalKnown ? " · 快照内完整空结果" : ""}`;
  if (result.freshness === "stale") return "结果已过期，请刷新";
  if (result.coverage === "partial") return `部分结果 · 已知 ${result.totalKnown} 项${result.limited ? " · 显示上限已到" : ""}`;
  if (result.limited) return `扫描完整 · 已知 ${result.totalKnown} 项 · 当前显示 ${result.rows.length} 项`;
  return result.rows.length ? `完整结果 · ${result.totalKnown} 项` : "完整空结果";
}
export function pinEligibleRows(rows: QueryRow[], pins: string[]): QueryRow[] {
  const wanted = new Set(pins); return [...rows.filter(row => wanted.has(row.entityKey ?? row.key)), ...rows.filter(row => !wanted.has(row.entityKey ?? row.key))];
}
export function preferenceFieldErrors(preference: ViewPreference, schema: DataSchema): string[] {
  return Object.entries(preference.fieldBindings ?? {}).filter(([key, id]) => [preference.sort, ...preference.columns, ...(preference.view === "board" ? [preference.groupField] : []), ...(preference.view === "calendar" ? [preference.dateField] : []), ...(preference.view === "chart" ? [preference.chartField] : [])].includes(key) && !schema.fields.some(field => field.id === id && field.key === key)).map(([key]) => `本机视图字段已失效：${fieldNames[key] ?? key}。请重新选择真实字段或恢复默认；不会绑定同名新字段。`);
}
export function decodeViewPreference(value: unknown): ViewPreference {
  const preference = defaultPreference(); if (!value || typeof value !== "object" || Array.isArray(value)) return preference; const saved = value as Record<string, unknown>;
  if (["list", "table", "board", "calendar", "chart"].includes(String(saved.view))) preference.view = saved.view as ResultView;
  if (saved.density === "compact" || saved.density === "comfortable") preference.density = saved.density;
  for (const key of ["summaries", "thumbnails", "descending"] as const) if (typeof saved[key] === "boolean") preference[key] = saved[key];
  for (const key of ["sort", "groupField", "dateField", "chartField"] as const) if (typeof saved[key] === "string" && saved[key]) preference[key] = saved[key];
  for (const key of ["columns", "pinned"] as const) if (Array.isArray(saved[key])) preference[key] = saved[key].filter((item): item is string => typeof item === "string");
  if (saved.fieldBindings && typeof saved.fieldBindings === "object" && !Array.isArray(saved.fieldBindings)) preference.fieldBindings = Object.fromEntries(Object.entries(saved.fieldBindings).filter((item): item is [string, string] => typeof item[1] === "string"));
  return preference;
}
export function viewPreferenceIdentity(root: string, scope: string, types: string[], predicates: string[], saved?: { owner: string; key: string; mode: string; revision?: string }): string {
  return saved ? JSON.stringify(["saved-query", saved.owner, saved.key, saved.mode, saved.mode === "pinned" ? saved.revision ?? "" : "current"]) : JSON.stringify(["query-context", root, scope, [...new Set(types)].sort(), [...new Set(predicates)].sort()]);
}
export function hasDirectStatusProjection(plan: QueryPlan | null, schema: DataSchema | null): boolean { const field = schema?.fields.find(field => field.key === "status"); const column = plan?.select.find(column => column.name === "status"); return !!field && !!column && [fieldExpr(field.id), field.key].includes(column.expr.trim()) && !plan?.group && !plan?.unnest && !plan?.advanced; }
export function directQueryField(plan: QueryPlan | null, schema: DataSchema | null, column: string): SchemaField | undefined { if (!plan || plan.group || plan.unnest || plan.advanced) return; const projection = plan.select.find(item => item.name === column); return schema?.fields.find(field => projection && [fieldExpr(field.id), field.key].includes(projection.expr.trim())); }
export function distinctFacetCounts(rows: QueryRow[], column: string): { value?: NativeValue; count: number; missing: boolean }[] {
  const facets = new Map<string, { value?: NativeValue; entities: Set<string>; missing: boolean }>();
  for (const row of rows) {
    if (row.readOnly && !row.entityKey) continue;
    const cell = row.cells[column]; const values = cell?.state === "value" && cell.value ? cell.value.kind === "array" ? cell.value.value : [cell.value] : cell?.state === "absent" ? [undefined] : cell?.state === "null" ? [{ kind: "none" } as NativeValue] : [];
    for (const value of values) { const key = value ? canonicalNative(value) : "@absent"; const facet = facets.get(key) ?? { value, entities: new Set<string>(), missing: !value }; facet.entities.add(row.entityKey ?? row.key); facets.set(key, facet); }
  }
  return [...facets.values()].map(({ value, entities, missing }) => ({ value, count: entities.size, missing }));
}
function canonicalNative(value: NativeValue): string { if (value.kind === "record") return JSON.stringify({ kind: "record", value: [...value.value].sort((a, b) => a.key < b.key ? -1 : a.key > b.key ? 1 : 0).map(item => [item.key, canonicalNative(item.value)]) }); if (value.kind === "array") return JSON.stringify({ kind: "array", value: value.value.map(canonicalNative) }); return JSON.stringify(value); }
export function scalarQueryLiteral(value: NativeValue): string {
  if (value.kind === "none") return "null"; if (value.kind === "bool") return String(value.value);
  if (value.kind === "int") return value.value; if (value.kind === "float") return /[.eE]/.test(value.value) ? value.value : `${value.value}.0`;
  if (value.kind === "decimal") return `decimal(${JSON.stringify(value.value)})`;
  if (value.kind === "date") return `date(${JSON.stringify(value.value)})`;
  if (value.kind === "time") return `time(${JSON.stringify(value.value)})`;
  if (value.kind === "localDatetime") return `localDatetime(${JSON.stringify(value.value)})`;
  if (value.kind === "duration") return `duration(${JSON.stringify(value.value)})`;
  if (value.kind === "text") return JSON.stringify(value.value);
  throw new Error("嵌套值请使用同项 CEL 条件，不能按显示文字比较。");
}
export function shapedQueryLiteral(shape: Shape, schema: DataSchema, value: NativeValue): string {
  const resolved = resolveShape(shape, schema); if (resolved.kind === "union") { const branch = unionVariant(resolved, value, schema); if (branch < 0) throw new Error("查询值不能唯一确定类型分支。"); return shapedQueryLiteral(resolved.variants[branch], schema, value); }
  if (resolved.kind === "scalar") { const constructor = ({ node_ref: "nodeRef", tag_ref: "tagRef", resource_ref: "resourceRef", instant: "instant", exact_decimal: "exactDecimal", exact_seconds: "exactSeconds" } as Record<string, string>)[resolved.semantic]; if (constructor) { if (value.kind !== "text") throw new Error("准确标量表示不匹配。"); return `${constructor}(${JSON.stringify(value.value)})`; } } return scalarQueryLiteral(value);
}
export function facetPredicate(facet: QueryFacet, schema: DataSchema): string {
  if (facet.expr) return facet.expr; const expr = fieldExpr(facet.fieldId); if (facet.missing) return `isAbsent(${expr})`; if (facet.value?.kind === "none") return `isNull(${expr})`; if (!facet.value) throw new Error("导航值缺失。");
  const definition = schema.fields.find(field => field.id === facet.fieldId); if (!definition) throw new Error("导航字段已经失效。"); const shape = resolveShape(definition.shape, schema);
  if (shape.kind !== "list") return `${expr} == ${shapedQueryLiteral(shape, schema, facet.value)}`;
  let item = resolveShape(shape.items, schema); if (item.kind === "union") { const branch = unionVariant(item, facet.value, schema); if (branch < 0) throw new Error("导航值分支不明确。"); item = resolveShape(item.variants[branch], schema); }
  if (facet.value.kind === "record" && item.kind === "record") { const key = nativeMember(facet.value, "ref") ? "ref" : nativeMember(facet.value, "text") ? "text" : undefined; if (!key || !item.members[key]) throw new Error("这个嵌套导航值请使用同项 CEL 条件。"); const value = nativeMember(facet.value, key)!; return `${expr}.exists(item, has(item.${key}) && item.${key} == ${shapedQueryLiteral(item.members[key].shape, schema, value)})`; }
  return `${expr}.exists(item, item == ${shapedQueryLiteral(item, schema, facet.value)})`;
}
export function composeWhere(search: string, includeBody: boolean, conditions: Condition[], expression: string): string | undefined {
  const pieces = conditions.map(condition => `(${condition.expr})`);
  if (search) { const literal = JSON.stringify(search); pieces.unshift(includeBody ? `(title.contains(${literal}) || body.contains(${literal}))` : `(title.contains(${literal}))`); }
  if (expression.trim()) pieces.push(`(${expression.trim()})`); return pieces.length ? pieces.join(" && ") : undefined;
}
/** Civil date grouping uses the exact day string, never a Date/host timezone. */
export function calendarDay(value: NativeValue | undefined): string | null { return value?.kind === "date" && /^\d{4}-\d{2}-\d{2}$/.test(value.value) ? value.value : null; }
const fieldExpr = (fieldId: string): string => `field(${JSON.stringify(fieldId)})`;
const rowTitle = (row: QueryRow): string => row.cells.title?.state === "value" && row.cells.title.value ? formatNativeValue(row.cells.title.value) || row.path || row.key : row.path?.replaceAll("\\", "/").split("/").at(-1) || (row.readOnly && !row.entityKey ? Object.entries(row.cells).filter(([name]) => !["count", "amount"].includes(name)).map(([, cell]) => stateText(cell)).join(" · ") || "汇总" : row.entityKey || row.key);
const stateText = (cell: Cell | undefined): string => !cell ? "未投影" : cell.state === "value" ? formatNativeValue(cell.value) : cellLabels[cell.state];
function labeled(label: string, input: HTMLElement): HTMLElement { const element = ui("label", "query-control-label", label); element.append(input); return element; }
function selectControl(options: [string, string][], value: string): HTMLSelectElement { const input = ui("select", "data-input"); options.forEach(([key, label]) => { const option = ui("option", "", label); option.value = key; input.append(option); }); input.value = value; return input; }

export class QueryPanel {
  private inlineDrafts = new Map<string, { value: NativeValue; raw: Record<string, string>; binding: string }>();
  readonly element = ui("section", "query-panel");
  private root: string | null = null;
  private workspaceId?: string;
  private directory: string | null = null;
  private scopeIdentity: string | null = null;
  private navigationDirectory: string | null = null;
  private navigationScopeIdentity: string | null = null;
  private descendants = true;
  private lifecycle: "live" | "trash" = "live";
  private schema: DataSchema | null = null;
  private result: QueryResult | null = null;
  private resultPlan: QueryPlan | null = null;
  private search = "";
  private includeBody = false;
  private expression = "";
  private conditions: Condition[] = [];
  private types: string[] = [];
  private preference = defaultPreference();
  private shortcuts: Shortcut[] = [];
  private shortcutError?: string;
  private tagReferences: FormReferences = { nodes: [], state: "unavailable", messages: [] };
  private recent: { entityKey: string; path: string; label: string; lifecycle?: "live" | "trash"; revision?: string }[] = [];
  private selected: string | null = null;
  private generation = 0;
  private loading = false;
  private error: string | null = null;
  private group: QueryPlan["group"];
  private take = 200;
  private captureDialog: HTMLDialogElement | null = null;
  private captureDialogs = new Map<string, HTMLDialogElement>();
  private captureForms = new Map<string, QuickCapture>();
  private exactPlan: QueryPlan | null = null;
  private advancedDraft = "";
  private parameterDraft = "{}";
  private advancedOpen = false;
  private savedSelection: SavedSelection | null = null;
  private savedPlan: QueryPlan | null = null;
  private savedPending: { body: Record<string, unknown>; basis: NonNullable<ReturnType<PanelHost["getDocument"]>> } | null = null;
  private definitionRevision: string | null = null;
  private definitionName = "";
  private thumbnailDisposers: (() => void)[] = [];
  private previewCache = new Map<string, Promise<RowPreview>>();
  private searchTimer?: ReturnType<typeof setTimeout>;
  private periodNavigators = new Map<string, PeriodNavigator>();
  private navigationControls = new Map<string, NavigationControls>();
  private readonly resultsHost = ui("div", "query-results");
  private readonly controlsHost = ui("div", "query-controls");
  private readonly navigationHost = ui("nav", "query-facets");
  constructor(private readonly host: PanelHost, private readonly exportResult?: (query: FrozenQuery) => void | Promise<void>) { this.element.setAttribute("aria-label", "查询与列表"); this.navigationHost.setAttribute("aria-label", "类型、标签与属性值导航"); this.element.append(this.controlsHost, this.navigationHost, this.resultsHost); this.renderControls(); this.renderResults(); }
  setLibrary(library: { root: string; workspaceId?: string } | null): void {
    if (this.root !== library?.root || this.workspaceId !== library?.workspaceId) this.tagReferences = { nodes: [], state: "unavailable", messages: [] };
    if (this.root === library?.root && this.workspaceId === library?.workspaceId) return; this.generation++; this.captureDialog?.close(); this.root = library?.root ?? null; this.captureDialog = this.root ? this.captureDialogs.get(this.root) ?? null : null; this.workspaceId = library?.workspaceId; this.directory = null; this.scopeIdentity = null; this.navigationDirectory = null; this.navigationScopeIdentity = null; this.schema = null; this.result = null; this.conditions = []; this.types = []; this.selected = null; this.savedSelection = null; this.savedPlan = null; this.exactPlan = null; this.advancedDraft = ""; this.parameterDraft = "{}"; this.expression = ""; this.group = undefined; this.definitionRevision = null; this.loadPreferences(); this.renderControls(); this.renderResults(); if (this.root && this.workspaceId) void this.refresh();
  }
  setScope(directory: string | null, identity?: string): void { if (directory === this.navigationDirectory && (identity ?? null) === this.navigationScopeIdentity) return; this.navigationDirectory = directory; this.navigationScopeIdentity = identity ?? null; this.applyScope(directory, identity); }
  private applyScope(directory: string | null, identity?: string): void { this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; this.exactPlan = null; this.group = undefined; this.directory = directory; this.scopeIdentity = identity ?? null; this.result = null; this.generation++; this.loadPreferences(); this.renderControls(); if (this.root && this.workspaceId) void this.refresh(); }
  documentChanged(): void { const document = this.host.getDocument(); const selected = document?.root === this.root && document.nodeId && this.workspaceId ? `weftext-node://${this.workspaceId}/${document.nodeId}` : null; if (selected !== this.selected) { this.selected = selected; this.renderResults(); } this.renderControls(); }
  setSearch(text: string, includeBody: boolean): void { if (text === this.search && includeBody === this.includeBody) return; this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; this.search = text; this.includeBody = includeBody; this.renderControls(); clearTimeout(this.searchTimer); if (this.root && this.workspaceId) this.searchTimer = setTimeout(() => { void this.refresh(); }, 300); }
  setPlan(plan: QueryPlan): void { this.adoptPlan(plan); this.renderControls(); void this.refresh(); }
  private adoptPlan(plan: QueryPlan): void { this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; this.exactPlan = structuredClone(plan); this.advancedDraft = JSON.stringify(plan, null, 2); this.directory = plan.scope.directory ?? null; this.scopeIdentity = this.directory?.startsWith("weftext-") ? this.directory : null; this.descendants = plan.scope.descendants; this.lifecycle = plan.scope.lifecycle; this.types = [...plan.types]; this.conditions = []; this.search = ""; this.expression = ""; this.group = plan.group; this.take = plan.take ?? 200; this.loadPreferences(); }
  private libraryIdentity(): string { return this.workspaceId ? `weftext-workspace://${this.workspaceId}` : this.root ?? ""; }
  private preferenceKey(): string { const predicates = [...this.conditions.map(item => item.expr), ...((this.exactPlan?.where ?? this.expression).trim() ? [(this.exactPlan?.where ?? this.expression).trim()] : []), ...(this.search ? [composeWhere(this.search, this.includeBody, [], "")!] : []), ...(this.exactPlan ? [JSON.stringify(this.exactPlan)] : [])]; return `weftext-query-view:${viewPreferenceIdentity(this.libraryIdentity(), this.scopeIdentity ?? this.directory ?? "@library", this.types, predicates, this.savedSelection ?? undefined)}`; }
  private contextChanged(): void { this.loadPreferences(); this.renderControls(); void this.refresh(); }
  private loadPreferences(): void {
    this.preference = defaultPreference(); this.shortcuts = []; this.recent = []; this.shortcutError = undefined;
    try { this.preference = decodeViewPreference(JSON.parse(localStorage.getItem(this.preferenceKey()) ?? "null")); } catch { /* Display preferences do not change author semantics. */ }
    try { const shortcutKey = `weftext-query-shortcuts:${this.libraryIdentity()}`, raw = localStorage.getItem(shortcutKey), collection = decodeTagShortcutCollection(raw ? JSON.parse(raw) : { version: 2, items: [] }); if (collection.items.some(item => !item || typeof item.id !== "string" || typeof item.label !== "string" || !["query", "node", "savedQuery", "directory", "tag"].includes(String(item.kind)))) throw new Error("快捷入口项目格式无效。"); this.shortcuts = collection.items as unknown as Shortcut[]; if (raw) localStorage.setItem(shortcutKey, JSON.stringify({ version: 2, items: this.shortcuts })); } catch (error) { this.shortcutError = `快捷入口恢复失败，原记录保留，暂停入口写入：${String(error)}`; }
    try { const recent: unknown = JSON.parse(localStorage.getItem(`weftext-query-recents:${this.libraryIdentity()}`) ?? "[]"); if (Array.isArray(recent)) this.recent = recent.filter((item): item is typeof this.recent[number] => item && typeof item.entityKey === "string" && typeof item.path === "string" && typeof item.label === "string"); } catch { /* Recent navigation is independent from the original shortcuts. */ }
  }
  private savePreferences(): void { this.preference.fieldBindings ??= {}; for (const key of [...this.preference.columns, this.preference.sort, this.preference.groupField, this.preference.dateField, this.preference.chartField]) { const id = this.fieldByKey(key); if (id && !this.preference.fieldBindings[key]) this.preference.fieldBindings[key] = id; } localStorage.setItem(this.preferenceKey(), JSON.stringify(this.preference)); }
  private bindPreferenceField(key: string): void { const id = this.fieldByKey(key); if (id) { this.preference.fieldBindings ??= {}; this.preference.fieldBindings[key] = id; } }
  private fieldByKey(key: string): string | undefined { return this.schema?.fields.find(field => field.key === key)?.id; }
  private async resolveScopeLocator(generation: number): Promise<void> { if (!this.root || !this.scopeIdentity) return; const targetRef = this.scopeIdentity; const target = await this.host.request<NavigationTarget>("/api/navigation/resolve", { root: this.root, targetRef }); if (generation !== this.generation) return; if (target.status !== "managed" || target.targetRef !== targetRef || !target.path) throw new Error("当前目录身份缺失或不可用，未按旧路径替换。"); this.directory = target.path; }
  getPlan(): QueryPlan {
    if (this.savedSelection && this.savedPlan) return structuredClone(this.savedPlan);
    if (this.exactPlan) { const plan = structuredClone(this.exactPlan); plan.types = [...this.types]; plan.where = composeWhere(this.search, this.includeBody, this.conditions, plan.where ?? ""); return plan; }
    const columns: { name: string; expr: string }[] = [{ name: "title", expr: "title" }, { name: "types", expr: "types" }];
    if (this.preference.summaries && !this.group) columns.push({ name: "summary", expr: "body" });
    const requested = new Set(this.preference.columns);
    if (this.preference.view === "board") requested.add(this.preference.groupField);
    if (this.preference.view === "calendar") requested.add(this.preference.dateField);
    if (this.preference.view === "chart") requested.add(this.preference.chartField);
    for (const key of requested) { const id = this.fieldByKey(key); if (id) columns.push({ name: key, expr: fieldExpr(id) }); }
    const sort = this.preference.sort === "title" ? "title" : this.fieldByKey(this.preference.sort) ? fieldExpr(this.fieldByKey(this.preference.sort)!) : this.preference.sort;
    return { scope: { ...(this.scopeIdentity || this.directory ? { directory: this.scopeIdentity ?? this.directory! } : {}), descendants: this.descendants, lifecycle: this.lifecycle }, types: [...this.types], where: composeWhere(this.search, this.includeBody, this.conditions, this.expression), select: this.group ? [] : columns, order: this.group ? [] : [{ expr: sort, descending: this.preference.descending, nulls: "last" }], take: this.take, ...(this.group ? { group: this.group } : {}) };
  }
  async refresh(): Promise<void> {
    clearTimeout(this.searchTimer);
    if (this.savedSelection) { await this.runSaved(this.savedSelection); return; }
    if (!this.root || !this.workspaceId) { this.renderResults(); return; } const root = this.root; const generation = ++this.generation; this.loading = true; this.error = null; if (this.result) this.result = { ...this.result, freshness: "stale", superseded: true }; this.renderResults();
    try {
      const schemaResult = await this.host.request<DataSchema | { schema: DataSchema }>("/api/data/schema", { root }); if (generation !== this.generation) return; this.schema = "schema" in schemaResult ? schemaResult.schema : schemaResult; await this.resolveScopeLocator(generation); if (generation !== this.generation) return; this.renderControls();
      const preferenceErrors = this.exactPlan ? [] : preferenceFieldErrors(this.preference, this.schema!); if (preferenceErrors.length) throw new Error(preferenceErrors.join("；")); const plan = this.getPlan(); const parameters = parseQueryParameters(this.parameterDraft); const result = await this.host.request<QueryResult>("/api/query/run", { root, plan, parameters }); if (generation !== this.generation) return;
      this.result = result; this.resultPlan = plan; this.previewCache.clear(); this.renderNavigation();
      if (this.selected && !result.rows.some(row => (row.entityKey ?? row.key) === this.selected)) this.host.notify(result.coverage === "complete" && result.freshness === "current" ? "已选笔记已移出当前结果、删除或不可用。正文草稿仍保留。" : "当前结果不完整，暂不能确定已选笔记是否仍在此范围。");
    } catch (error) { if (generation === this.generation) this.error = error instanceof Error ? error.message : String(error); }
    finally { if (generation === this.generation) { this.loading = false; this.renderResults(); } }
  }
  private renderControls(): void {
    this.controlsHost.replaceChildren(); const heading = ui("div", "query-heading"); heading.append(ui("h2", "", "资料库查询"), action("刷新", () => this.refresh(), "data-button compact"));
    const capture = action("快速记录", () => this.showCapture(), "data-button primary"); capture.disabled = !this.workspaceId; heading.append(capture); this.controlsHost.append(heading);
    if (this.savedSelection) { this.controlsHost.append(ui("p", "query-result-status", `${this.savedSelection.mode === "pinned" ? "固定定义" : "当前定义"}：${this.savedSelection.label}${this.definitionRevision ? ` · ${this.definitionRevision}` : ""} · 每次读取当前数据`), action("复制定义为临时查询并编辑", () => { const plan = this.savedPlan; this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; if (plan) this.setPlan(plan); else { this.renderControls(); void this.refresh(); } })); }
    if (this.exactPlan && !this.savedSelection) this.controlsHost.append(ui("p", "query-result-status", "正在使用完整 Query 计划。投影、排序与来源保持该定义；目录导航会回到可视化查询。"));
    const searchForm = ui("form", "query-search-form"); const search = ui("input", "data-input"); search.type = "search"; search.placeholder = "搜索标题…"; search.value = this.search; search.setAttribute("aria-label", "查询标题文字"); search.oninput = () => { this.search = search.value; }; const run = ui("button", "data-button", "查询"); run.type = "submit"; searchForm.append(search, run); searchForm.onsubmit = event => { event.preventDefault(); void this.refresh(); }; this.controlsHost.append(searchForm);
    const scopeRow = ui("div", "query-scope-row"); scopeRow.append(ui("span", "query-scope", this.directory ? `目录：${this.directory}` : "整个资料库")); if (this.directory) scopeRow.append(action("全库", () => this.applyScope(null), "data-button compact"));
    const descendants = ui("input"); descendants.type = "checkbox"; descendants.checked = this.descendants; descendants.onchange = () => { this.descendants = descendants.checked; void this.refresh(); }; scopeRow.append(labeled("包含子目录", descendants));
    const body = ui("input"); body.type = "checkbox"; body.checked = this.includeBody; body.onchange = () => { this.includeBody = body.checked; void this.refresh(); }; scopeRow.append(labeled("搜索正文", body));
    const lifecycle = selectControl([["live", "在库笔记"], ["trash", "回收站（只读）"]], this.lifecycle); lifecycle.onchange = () => { this.lifecycle = lifecycle.value as "live" | "trash"; void this.refresh(); }; scopeRow.append(labeled("范围", lifecycle)); this.controlsHost.append(scopeRow);
    const chips = ui("div", "query-chips"); this.types.forEach(token => chips.append(action(`类型：${typeNames[token] ?? token} ×`, () => { this.types = this.types.filter(item => item !== token); this.contextChanged(); }, "query-chip"))); this.conditions.forEach(condition => { chips.append(action(`${condition.label} ×`, () => { this.conditions = this.conditions.filter(item => item.id !== condition.id); this.contextChanged(); }, "query-chip")); if (condition.tag) chips.append(action(condition.tag.mode === "subtree" ? "改为仅此标签" : "改为此标签及子级", () => { const tag = condition.tag!; this.conditions = this.conditions.filter(item => item.id !== condition.id); this.applyTag(tag.reference, tag.label, tag.mode === "subtree" ? "exact" : "subtree"); }, "data-button compact")); }); this.controlsHost.append(chips);
    const presentation = ui("div", "query-presentation"); const view = selectControl([["list", "列表"], ["table", "表格"], ["board", "看板"], ["calendar", "日历"], ["chart", "图表"]], this.preference.view); view.onchange = () => { this.preference.view = view.value as ResultView; this.savePreferences(); this.renderControls(); void this.refresh(); }; presentation.append(labeled("视图", view));
    const density = selectControl([["comfortable", "详细"], ["compact", "紧凑"]], this.preference.density); density.onchange = () => { this.preference.density = density.value as ViewPreference["density"]; this.savePreferences(); this.renderResults(); }; presentation.append(labeled("密度", density));
    const sort = selectControl([["title", "标题"], ...(this.schema?.fields.filter(field => { const shape = resolveShape(field.shape, this.schema!); return shape.kind === "scalar" || shape.kind === "enum"; }).map(field => [field.key, fieldNames[field.key] ?? field.key] as [string, string]) ?? [])], this.preference.sort); sort.disabled = !!this.savedSelection || !!this.exactPlan; sort.onchange = () => { this.preference.sort = sort.value; this.bindPreferenceField(sort.value); this.savePreferences(); void this.refresh(); }; presentation.append(labeled("排序", sort)); const descending = action(this.preference.descending ? "降序 ↓" : "升序 ↑", () => { this.preference.descending = !this.preference.descending; this.savePreferences(); this.renderControls(); void this.refresh(); }, "data-button compact"); descending.disabled = !!this.savedSelection || !!this.exactPlan; presentation.append(descending);
    const summary = ui("input"); summary.type = "checkbox"; summary.checked = this.preference.summaries; summary.onchange = () => { this.preference.summaries = summary.checked; this.savePreferences(); void this.refresh(); }; presentation.append(labeled("正文摘要", summary)); const thumbnails = ui("input"); thumbnails.type = "checkbox"; thumbnails.checked = this.preference.thumbnails; thumbnails.onchange = () => { this.preference.thumbnails = thumbnails.checked; this.savePreferences(); this.renderResults(); }; presentation.append(labeled("缩略图", thumbnails)); this.controlsHost.append(presentation);
    if (this.schema) this.renderBuilder();
    this.renderAdvancedPlan(); this.renderRegisteredNavigation(); this.renderPeriodNavigation(); this.renderSavedQueries(); this.renderShortcuts();
    if (this.savedSelection) for (const control of this.controlsHost.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(".query-search-form input,.query-search-form button,.query-chips button,.query-scope-row input,.query-scope-row select,.query-builder input,.query-builder select,.query-builder textarea,.query-builder button")) if (!control.classList.contains("query-view-field")) control.disabled = true;
    if (this.exactPlan && !this.savedSelection) for (const control of this.controlsHost.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>(".query-scope-row input,.query-scope-row select,.query-builder input,.query-builder select,.query-builder textarea,.query-builder button")) if (!control.classList.contains("query-view-field")) control.disabled = true;
  }
  private renderBuilder(): void {
    const schema = this.schema!; const builder = ui("details", "query-builder"); builder.append(ui("summary", "", "筛选、字段与汇总"));
    const type = selectControl([["", "添加类型条件…"], ...schema.types.map(item => { const alias = Object.entries(schema.aliases).find(([, id]) => id === item.id)?.[0] ?? item.id; return [alias, typeNames[alias] ?? alias] as [string, string]; })], ""); type.onchange = () => { if (type.value && !this.types.includes(type.value)) this.types.push(type.value); this.contextChanged(); }; builder.append(labeled("类型", type));
    const filterForm = ui("form", "query-filter-builder"); const field = selectControl(schema.fields.map(item => [item.id, fieldNames[item.key] ?? item.key]), schema.fields[0]?.id ?? ""); const operator = selectControl([["==", "等于"], ["!=", "不等于"], [">", "大于"], [">=", "大于等于"], ["<", "小于"], ["<=", "小于等于"], ["contains", "包含文字"], ["absent", "未填写"], ["null", "明确 none"]], "=="); const value = ui("input", "data-input"); value.setAttribute("aria-label", "筛选值（准确输入）"); value.placeholder = "准确值"; const branchesHost = ui("div"); let filterShape: Shape | undefined;
    const updateFilterShape = (): void => { branchesHost.replaceChildren(); const selected = schema.fields.find(item => item.id === field.value); if (!selected) return; const shape = resolveShape(selected.shape, schema); filterShape = shape; if (shape.kind === "union") { const variants = shape.variants.map((variant, index) => ({ shape: resolveShape(variant, schema), index })).filter(item => item.shape.kind === "scalar" || item.shape.kind === "enum"); if (!variants.length) { filterShape = undefined; return; } const branch = selectControl(variants.map(item => [String(item.index), item.shape.kind === "scalar" ? item.shape.semantic : "enum"]), String(variants[0].index)); filterShape = variants[0].shape; branch.onchange = () => { filterShape = variants.find(item => item.index === Number(branch.value))?.shape; }; branchesHost.append(labeled("值类型（不自动转换）", branch)); } }; field.onchange = updateFilterShape; updateFilterShape();
    const add = ui("button", "data-button", "添加筛选"); add.type = "submit"; filterForm.append(labeled("字段", field), labeled("条件", operator), branchesHost, value, add); builder.append(filterForm);
    filterForm.onsubmit = event => {
      event.preventDefault(); try {
        const selected = schema.fields.find(item => item.id === field.value)!; const expr = fieldExpr(selected.id); let condition: string;
        if (operator.value === "absent") condition = `isAbsent(${expr})`; else if (operator.value === "null") condition = `isNull(${expr})`;
        else { const resolved = filterShape; if (!resolved || !["scalar", "enum"].includes(resolved.kind)) throw new Error("多值与嵌套属性请使用下方同项 CEL 条件。所有 label / note 条件必须作用于同一项。"); const native = scalarFromInput(resolved, schema, value.value); if (operator.value === "contains") { if (native.kind !== "text") throw new Error("包含文字仅支持文字字段。"); condition = `${expr}.contains(${JSON.stringify(native.value)})`; } else condition = `${expr} ${operator.value} ${shapedQueryLiteral(resolved, schema, native)}`; }
        this.conditions.push({ id: crypto.randomUUID(), label: `${fieldNames[selected.key] ?? selected.key} ${operator.selectedOptions[0].textContent} ${value.value}`, expr: condition }); this.contextChanged();
      } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); }
    };
    const expression = ui("textarea", "data-input query-expression"); expression.value = this.expression; expression.rows = 3; expression.placeholder = 'phones.exists(p, p.number == "138…" && p.label == "工作")'; expression.setAttribute("aria-label", "高级 CEL 条件"); expression.oninput = () => { this.expression = expression.value; }; builder.append(labeled("CEL 条件（与上方筛选同时满足）", expression), action("应用条件", () => this.refresh()));
    const columns = ui("fieldset", "query-columns"); columns.append(ui("legend", "", "投影字段")); schema.fields.forEach(item => { const checkbox = ui("input"); checkbox.type = "checkbox"; checkbox.checked = this.preference.columns.includes(item.key); checkbox.onchange = () => { if (checkbox.checked) { this.preference.columns.push(item.key); this.bindPreferenceField(item.key); } else this.preference.columns = this.preference.columns.filter(key => key !== item.key); this.savePreferences(); void this.refresh(); }; columns.append(labeled(fieldNames[item.key] ?? item.key, checkbox)); }); builder.append(columns);
    const fieldOptions = this.exactPlan || this.savedSelection ? [...new Set([...(this.resultPlan?.select ?? this.getPlan().select).map(item => item.name), ...Object.keys(this.result?.rows[0]?.cells ?? {})])].filter(name => !["title", "summary", "types"].includes(name)).map(name => [name, fieldNames[name] ?? name] as [string, string]) : schema.fields.map(item => [item.key, fieldNames[item.key] ?? item.key] as [string, string]);
    if (this.preference.view === "board") { const groupField = selectControl(fieldOptions, this.preference.groupField); groupField.classList.add("query-view-field"); groupField.onchange = () => { this.preference.groupField = groupField.value; this.bindPreferenceField(groupField.value); this.savePreferences(); void this.refresh(); }; builder.append(labeled("看板分组字段（只分组当前结果）", groupField)); }
    if (this.preference.view === "calendar") { const dateField = selectControl(fieldOptions, this.preference.dateField); dateField.classList.add("query-view-field"); dateField.onchange = () => { this.preference.dateField = dateField.value; this.bindPreferenceField(dateField.value); this.savePreferences(); void this.refresh(); }; builder.append(labeled("日历日期字段", dateField)); }
    if (this.preference.view === "chart") { const chartField = selectControl([["count", "汇总 count"], ...fieldOptions], this.preference.chartField); chartField.classList.add("query-view-field"); chartField.onchange = () => { this.preference.chartField = chartField.value; this.bindPreferenceField(chartField.value); this.savePreferences(); void this.refresh(); }; builder.append(labeled("图表数值字段", chartField)); }
    const aggregateForm = ui("form", "query-aggregate-builder"); const groupKey = selectControl([["", "不按字段分组"], ...fieldOptions], this.group?.keys[0]?.name ?? ""); const op = selectControl([["count", "计数（笔记数）"], ["sum", "求和"], ["min", "最小"], ["max", "最大"]], this.group?.aggregates[0]?.op ?? "count"); const amount = selectControl(fieldOptions, this.group?.aggregates[0]?.expr ? this.preference.chartField : fieldOptions[0]?.[0] ?? ""); const aggregateSubmit = ui("button", "data-button", "运行汇总"); aggregateSubmit.type = "submit"; aggregateForm.append(labeled("分组", groupKey), labeled("汇总", op), labeled("数值字段", amount), aggregateSubmit); aggregateForm.onsubmit = event => { event.preventDefault(); this.group = { keys: groupKey.value ? [{ name: groupKey.value, expr: fieldExpr(this.fieldByKey(groupKey.value)!) }] : [], aggregates: [{ name: op.value === "count" ? "count" : "amount", op: op.value as "count", ...(op.value === "count" ? {} : { expr: fieldExpr(this.fieldByKey(amount.value)!) }) }] }; this.preference.chartField = op.value === "count" ? "count" : "amount"; this.renderControls(); void this.refresh(); }; builder.append(aggregateForm); if (this.group) builder.append(ui("p", "data-help", "汇总行只读；部分输入的数值只代表已知部分。"), action("返回笔记结果", () => { this.group = undefined; this.renderControls(); void this.refresh(); }));
    const limit = ui("input", "data-input"); limit.type = "number"; limit.min = "1"; limit.max = "10000"; limit.value = String(this.take); limit.onchange = () => { const next = Number(limit.value); if (Number.isInteger(next) && next >= 1 && next <= 10000) { this.take = next; void this.refresh(); } }; builder.append(labeled("显示上限", limit), action("恢复本机视图默认值", () => { this.preference = defaultPreference(); this.savePreferences(); this.renderControls(); void this.refresh(); })); this.controlsHost.append(builder);
  }
  private renderAdvancedPlan(): void {
    const section = ui("details", "query-advanced-plan"); section.open = this.advancedOpen; section.ontoggle = () => { this.advancedOpen = section.open; }; section.append(ui("summary", "", "高级 Query · 完整计划与参数"));
    const planInput = ui("textarea", "data-input query-plan-input"); planInput.rows = 14; planInput.spellcheck = false; planInput.value = this.advancedDraft || JSON.stringify(this.getPlan(), null, 2); planInput.setAttribute("aria-label", "完整 Query 计划 JSON"); planInput.oninput = () => { this.advancedDraft = planInput.value; };
    const parameterInput = ui("textarea", "data-input query-expression"); parameterInput.rows = 4; parameterInput.spellcheck = false; parameterInput.value = this.parameterDraft; parameterInput.setAttribute("aria-label", "本次调用的原生参数 JSON"); parameterInput.oninput = () => { this.parameterDraft = parameterInput.value; };
    const feedback = ui("p", "data-help", "CEL 条件、投影和排序使用同一编译器。高级来源在 advanced 中；参数是本次输入，不成为另一份笔记内容。"); feedback.setAttribute("aria-live", "polite");
    section.append(action("载入当前实际计划（替换此编辑框）", () => { this.advancedDraft = JSON.stringify(this.getPlan(), null, 2); planInput.value = this.advancedDraft; }), labeled("完整计划", planInput), labeled("本次调用参数", parameterInput));
    section.append(action("运行完整计划", async () => { try { const plan = parseQueryPlan(planInput.value); parseQueryParameters(parameterInput.value); this.parameterDraft = parameterInput.value; this.advancedOpen = true; this.setPlan(plan); } catch (error) { feedback.className = "data-warning"; feedback.textContent = error instanceof Error ? error.message : String(error); } }, "data-button primary"), action("按本次参数重跑", async () => { try { parseQueryParameters(parameterInput.value); this.parameterDraft = parameterInput.value; await this.refresh(); } catch (error) { feedback.className = "data-warning"; feedback.textContent = error instanceof Error ? error.message : String(error); } }));
    if (this.exactPlan && !this.savedSelection) section.append(action("返回可视化查询", () => { this.exactPlan = null; this.group = undefined; this.expression = ""; this.conditions = []; this.loadPreferences(); this.renderControls(); void this.refresh(); }));
    const multivalue = this.schema?.fields.find(field => field.key === "phones" && resolveShape(field.shape, this.schema!).kind === "list") ?? this.schema?.fields.find(field => resolveShape(field.shape, this.schema!).kind === "list");
    if (multivalue) section.append(action("载入多值展开示例（暂不运行）", () => { const plan: QueryPlan = { scope: { ...(this.scopeIdentity || this.directory ? { directory: this.scopeIdentity ?? this.directory! } : {}), descendants: this.descendants, lifecycle: this.lifecycle }, types: [...this.types], select: [{ name: "title", expr: "title" }, { name: "item", expr: "item" }], order: [], unnest: { array: fieldExpr(multivalue.id), binding: "item" }, take: this.take }; this.advancedDraft = JSON.stringify(plan, null, 2); planInput.value = this.advancedDraft; }));
    section.append(feedback, ui("p", "data-help", 'select 支持命名 CEL 投影；order 可含多项并明确 nulls。unnest 为 {array, binding}。advanced 支持 parameters、unionAll、distinct、related 与 call。参数例如 {"limit":{"kind":"int","value":"200"}}；准确数字保字符串，none 与未提供不同。'));
    this.controlsHost.append(section);
  }
  private renderNavigation(): void {
    this.navigationHost.replaceChildren(); if (!this.result || !this.schema) return;
    const data = this.result; const header = ui("p", "data-help", data.facets ? data.coverage === "complete" ? "当前范围属性导航" : "当前范围属性导航 · 已知计数" : "已加载结果属性导航 · 已知计数"); this.navigationHost.append(header);
    const facets: QueryFacet[] = data.facets ?? ["tags", "status", "project"].flatMap(key => { const id = this.fieldByKey(key); if (!id) return []; return distinctFacetCounts(data.rows, key).map(facet => ({ fieldId: id, key, ...facet })); });
    const types = distinctFacetCounts(data.rows, "types"); types.forEach(facet => { if (facet.value?.kind !== "text") return; const token = facet.value.value; this.navigationHost.append(action(`${typeNames[token] ?? token} · ${facet.count}`, () => { this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; if (!this.types.includes(token)) this.types.push(token); this.contextChanged(); }, "query-facet")); });
    for (const facet of facets) {
      const label = facet.missing ? "未填写" : facet.label ?? formatNativeValue(facet.value); const button = action(`${fieldNames[facet.key] ?? facet.key}：${label} · ${facet.count}`, () => {
        try {
          const expr = facetPredicate(facet, this.schema!);
          this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; this.conditions.push({ id: crypto.randomUUID(), label: `${fieldNames[facet.key] ?? facet.key}：${label}`, expr }); this.contextChanged();
        } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); }
      }, "query-facet"); this.navigationHost.append(button);
    }
  }
  private renderShortcuts(): void {
    const shortcuts = ui("details", "query-shortcuts"); shortcuts.append(ui("summary", "", "本机快捷入口与最近打开"));
    if (this.shortcutError) { const key = `weftext-query-shortcuts:${this.libraryIdentity()}`, raw = localStorage.getItem(key); shortcuts.append(ui("p", "data-warning", this.shortcutError), ui("pre", "", raw ?? ""), action("备份原记录并重置无效入口", () => { if (raw !== null) localStorage.setItem(`${key}:preserved:${crypto.randomUUID()}`, raw); localStorage.setItem(key, JSON.stringify({ version: 2, items: [] })); this.loadPreferences(); this.renderControls(); })); }
    shortcuts.append(action("把当前条件添加为本机快捷入口", () => { const label = window.prompt("快捷入口名称", this.savedSelection?.label || this.search || "当前查询"); if (!label) return; this.shortcuts.push(this.savedSelection ? { id: crypto.randomUUID(), label, kind: "savedQuery", definition: { ...this.savedSelection } } : { id: crypto.randomUUID(), label, kind: "query", plan: this.getPlan() }); this.saveShortcuts(); this.renderControls(); }));
    this.shortcuts.forEach((shortcut, index) => { const row = ui("div", "query-shortcut-row"), label = shortcut.kind === "tag" ? `${shortcut.label} · ${shortcut.matchMode === "subtree" ? "此标签及子级" : "仅此标签"}` : shortcut.label; row.append(action(label, async () => { if (shortcut.kind === "directory" || shortcut.kind === "tag") await this.openNavigationShortcut(shortcut); else if (shortcut.kind === "savedQuery" && shortcut.definition) await this.runSaved(shortcut.definition); else if (shortcut.kind === "query" && shortcut.plan) this.setPlan(shortcut.plan); else if (shortcut.path) await this.openTarget(shortcut.path, shortcut.entityKey); }, "data-button")); const up = action("上移", () => { if (index === 0) return; [this.shortcuts[index - 1], this.shortcuts[index]] = [this.shortcuts[index], this.shortcuts[index - 1]]; this.saveShortcuts(); this.renderControls(); }, "data-button compact"); up.disabled = index === 0; row.append(up, action("移除入口", () => { this.shortcuts = this.shortcuts.filter(item => item.id !== shortcut.id); this.saveShortcuts(); this.renderControls(); }, "data-button compact")); shortcuts.append(row); });
    for (const item of this.recent) shortcuts.append(action(`最近：${item.label}${item.lifecycle === "trash" ? " · 回收站只读" : ""}`, () => this.openTarget(item.path, item.entityKey, item.lifecycle, item.revision), "query-recent")); shortcuts.append(ui("p", "data-help", "入口、置顶与视图偏好保存在本机，不修改笔记内容。移除入口不删除目标。")); this.controlsHost.append(shortcuts);
  }
  private saveShortcuts(): void { if (this.shortcutError) { this.host.notify(this.shortcutError, true); throw new Error(this.shortcutError); } decodeTagShortcutCollection({ version: 2, items: this.shortcuts }); localStorage.setItem(`weftext-query-shortcuts:${this.libraryIdentity()}`, JSON.stringify({ version: 2, items: this.shortcuts })); }
  private renderRegisteredNavigation(): void {
    if (!this.root || !this.workspaceId) return;
    const root = this.root, workspaceId = this.workspaceId, identity = JSON.stringify([root, workspaceId]), current = () => this.root === root && this.workspaceId === workspaceId;
    let controls = this.navigationControls.get(identity);
    if (!controls) {
      controls = new NavigationControls(this.host, root, workspaceId, () => this.scopeIdentity ?? this.directory ?? root, (targetRef, label, mode) => { if (current()) this.applyTag(targetRef, label, mode); }, (targetRef, label, kind, matchMode) => {
        if (!current()) throw new Error("原资料库的导航操作已保留，请返回原库核对后添加入口。");
        if (kind === "tag" && matchMode !== "exact" && matchMode !== "subtree") throw new Error("新标签入口必须明确筛选模式。");
        if (!this.shortcuts.some(item => item.targetRef === targetRef && item.kind === kind && item.matchMode === matchMode)) this.shortcuts.push({ id: crypto.randomUUID(), label, kind, targetRef, ...(kind === "tag" ? { matchMode } : {}) });
        this.saveShortcuts(); this.renderControls();
      }, () => current() ? this.fieldByKey("tags") : undefined, () => { if (current()) this.tagRegistryChanged(); });
      this.navigationControls.set(identity, controls);
    }
    this.controlsHost.append(controls.element);
  }
  private tagRegistryChanged(): void { this.generation++; this.loading = false; this.tagReferences.tagTree = undefined; if (this.result) this.result = { ...this.result, freshness: "stale" }; this.renderResults(); }
  private applyTag(targetRef: string, label: string, mode: TagMatchMode): void { try { const field = this.schema?.fields.find(field => field.key === "tags"); if (!field || !this.schema) throw new Error("当前 schema 没有可用的 tags 字段。"); const expr = tagPredicate(targetRef, mode, fieldExpr(field.id)); this.savedSelection = null; this.savedPlan = null; this.definitionRevision = null; if (!this.conditions.some(condition => condition.expr === expr)) this.conditions.push({ id: crypto.randomUUID(), label: `标签：${label} · ${mode === "subtree" ? "此标签及子级" : "仅此标签"}`, expr, tag: { reference: targetRef, label, mode } }); this.contextChanged(); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); } }
  private async openNavigationShortcut(shortcut: Shortcut): Promise<void> { try { if (!shortcut.targetRef) throw new Error("快捷入口缺少稳定目标身份。"); const root = this.root, workspace = this.workspaceId, target = await this.host.request<NavigationTarget>("/api/navigation/resolve", { root, targetRef: shortcut.targetRef }); if (root !== this.root || workspace !== this.workspaceId) return; if (target.status !== "managed" || target.targetRef !== shortcut.targetRef) throw new Error(`目标${target.status === "missing" ? "已缺失" : "身份不可用"}`); if (shortcut.kind === "tag") { if (!shortcut.matchMode) throw new Error("标签入口没有明确模式。"); this.applyTag(shortcut.targetRef, `${target.displayName ?? target.label} · ${shortcut.targetRef.slice(shortcut.targetRef.lastIndexOf("/") + 1)}`, shortcut.matchMode); } else if (target.path) this.applyScope(target.path, shortcut.targetRef); else throw new Error("目录当前位置不可用。"); } catch (error) { this.host.notify(`快捷目标不可用：${error instanceof Error ? error.message : String(error)}。入口保留，未猜测同名目标。`, true); } }
  private renderSavedQueries(): void {
    const section = ui("details", "query-saved-definitions"); section.append(ui("summary", "", "原生保存 Query · 当前 / 固定定义")); const document = this.host.getDocument();
    if (!document?.nodeId || !this.workspaceId || document.root !== this.root) { section.append(ui("p", "data-help", "打开当前资料库中的笔记，才能读取或保存该笔记拥有的 Query 定义。")); this.controlsHost.append(section); return; }
    const owner = `weftext-node://${this.workspaceId}/${document.nodeId}`; const definitionsHost = ui("div", "saved-query-definitions"); const load = action("读取当前笔记的保存定义", async () => { load.disabled = true; try { const result = await this.host.request<SavedDefinitions>("/api/query/definitions", { root: this.root, owner }); definitionsHost.replaceChildren(ui("p", "data-help", `当前定义笔记版本：${result.ownerRevision ?? "尚无保留版本"}`)); if (!result.definitions.length) definitionsHost.append(ui("p", "data-empty", "此笔记还没有保存的 Query 定义。")); result.unavailable?.forEach(item => definitionsHost.append(ui("p", "data-warning", diagnosticText(item))));
      for (const definition of result.definitions) {
        const row = ui("section", "saved-query-row"); const displayLabel = definition.label.replace(/^weftext-query-/, ""); row.append(ui("h3", "", displayLabel)); const mode = selectControl([["current", "当前定义"], ["pinned", "固定指定版本"]], "current"); const revision = ui("input", "data-input"); revision.value = result.ownerRevision ?? ""; revision.setAttribute("aria-label", `${displayLabel} 的固定定义版本`); row.append(labeled("读取方式", mode), labeled("固定版本", revision), action("打开并查询", () => this.runSaved({ owner, key: definition.key, label: displayLabel, mode: mode.value as "current" | "pinned", ...(mode.value === "pinned" ? { revision: revision.value } : {}) }), "data-button primary"), action("用当前条件更新此定义并保存笔记", () => this.saveDefinition(definition.label, definition.key))); definitionsHost.append(row);
      }
    } catch (error) { definitionsHost.replaceChildren(ui("p", "data-warning", error instanceof Error ? error.message : String(error))); } finally { load.disabled = false; } }); section.append(load, definitionsHost);
    const label = ui("input", "data-input"); label.value = this.definitionName; label.oninput = () => { this.definitionName = label.value; }; label.placeholder = "例如：未完成任务（文字、数字、- 或 _）"; label.setAttribute("aria-label", "新的保存 Query 定义名称"); section.append(label, action(this.savedPending ? "确认上次保存查询与笔记" : "保存当前 Query 与当前笔记", () => this.saveDefinition(label.value)), ui("p", "data-help", "此按钮会保存当前笔记的完整草稿与 Query 定义。首次保存前可在属性 → 集中 schema 中预览并安装本库 schema。固定版本只固定定义；历史不可用时会报错。")); this.controlsHost.append(section);
  }
  private renderPeriodNavigation(): void {
    if (!this.root || !this.schema?.fields.some(item => item.key === "period")) return; let navigator = this.periodNavigators.get(this.root); if (!navigator) { navigator = new PeriodNavigator(this.host, this.root, () => this.directory?.startsWith("weftext-") ? this.root! : this.directory ?? this.root!); this.periodNavigators.set(this.root, navigator); } this.controlsHost.append(navigator.element);
  }
  private async saveDefinition(label: string, key?: string): Promise<void> {
    if (!this.host.savedDefinition) { this.host.notify("当前会话没有保存 Query 的回执接入，尚未执行保存。", true); return; }
    try { await this.host.prepareAction?.(); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); return; }
    const basis = this.host.getDocument(); if (!basis?.nodeId || basis.root !== this.root || (!label.trim() && !this.savedPending)) { this.host.notify("请打开当前资料库笔记并填写定义名称。", true); return; }
    const nativeLabel = label.startsWith("weftext-query-") ? label : `weftext-query-${label}`; if (!this.savedPending && !/^weftext-query-[\p{L}\p{N}_-]+$/u.test(nativeLabel)) { this.host.notify("定义名称请使用文字、数字、- 或 _，原输入保留。", true); return; }
    this.savedPending ??= { basis: { ...basis }, body: { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, baseId: basis.baseId, key, label: nativeLabel, plan: this.getPlan(), operationId: crypto.randomUUID() } }; const pending = this.savedPending;
    let receiptReceived = false;
    try { const result = await this.host.request<{ document: unknown; definition: SavedQuery; definitionRevision: string }>("/api/query/definition/save", pending.body); receiptReceived = true; await this.host.savedDefinition(result, pending.basis); this.savedPending = null; this.host.notify("Query 定义与当前笔记草稿已保存。"); this.renderControls(); await this.refresh(); }
    catch (error) { const code = error && typeof error === "object" && "code" in error ? String(error.code) : "unknown"; if (!receiptReceived && !["unknown", "pending", "resultUnknown"].includes(code)) this.savedPending = null; this.host.notify(`${error instanceof Error ? error.message : String(error)}${this.savedPending ? " 原草稿保留；重试会确认同一保存操作。" : " 原草稿保留。"}`, true); this.renderControls(); }
  }
  private async runSaved(selection: SavedSelection): Promise<void> {
    if (!this.root || (selection.mode === "pinned" && !selection.revision)) { this.host.notify("固定定义需要准确的历史版本。", true); return; }
    const priorPreference = this.preferenceKey(); const generation = ++this.generation; this.loading = true; this.error = null; this.savedSelection = selection; if (this.preferenceKey() !== priorPreference) this.loadPreferences(); if (this.result) this.result = { ...this.result, freshness: "stale", superseded: true }; this.renderControls(); this.renderResults();
    try {
      const parameters = parseQueryParameters(this.parameterDraft); const result = await this.host.request<QueryResult & { result?: QueryResult; definitionRevision?: string; definition?: SavedQuery; definitionChanged?: boolean }>("/api/query/definition/run", { root: this.root, owner: selection.owner, parameters, selection: { mode: selection.mode, key: selection.key, ...(selection.mode === "pinned" ? { revision: selection.revision } : {}) } });
      if (generation !== this.generation) return; const previous = this.definitionRevision; this.definitionRevision = result.definitionRevision ?? selection.revision ?? null; this.result = result.result ?? result; this.previewCache.clear(); this.savedPlan = result.definition?.plan ?? null; this.resultPlan = this.savedPlan;
      if (this.savedPlan) { this.exactPlan = structuredClone(this.savedPlan); this.directory = this.savedPlan.scope.directory ?? null; this.scopeIdentity = this.directory?.startsWith("weftext-") ? this.directory : null; this.descendants = this.savedPlan.scope.descendants; this.lifecycle = this.savedPlan.scope.lifecycle; this.types = [...this.savedPlan.types]; this.conditions = []; this.search = ""; this.expression = ""; this.group = this.savedPlan.group; }
      await this.resolveScopeLocator(generation); if (generation !== this.generation) return;
      if (selection.mode === "current" && (result.definitionChanged || (previous && this.definitionRevision !== previous))) this.host.notify("保存的 Query 定义已更新，结果按本次显示版本重新计算。"); else if (selection.mode === "pinned" && result.definitionChanged) this.host.notify("固定的定义与当前定义不同。本次仍使用明确选定的历史版本和当前数据。"); this.renderNavigation();
    } catch (error) { if (generation === this.generation) { this.error = error instanceof Error ? error.message : String(error); this.result = null; } }
    finally { if (generation === this.generation) { this.loading = false; this.renderControls(); this.renderResults(); } }
  }
  private renderResults(): void {
    this.thumbnailDisposers.forEach(dispose => dispose()); this.thumbnailDisposers = [];
    const scrollTop = this.resultsHost.scrollTop; this.resultsHost.replaceChildren(); this.resultsHost.className = `query-results ${this.preference.density}`; this.resultsHost.setAttribute("aria-busy", String(this.loading));
    if (!this.root) { this.resultsHost.append(ui("p", "data-empty", "打开资料库后搜索与浏览笔记。")); return; }
    if (!this.workspaceId) { this.resultsHost.append(ui("p", "data-help", "当前是普通文件夹。结构化查询需要已创建的资料库；文件仍可从导航直接打开。")); return; }
    if (this.loading) this.resultsHost.append(ui("p", "query-result-status", "正在读取当前范围…"));
    if (this.error) { const error = ui("div", "query-error"); error.setAttribute("role", "alert"); error.append(ui("p", "data-warning", `查询失败：${this.error}`), action("重试查询", () => this.refresh())); this.resultsHost.append(error); }
    if (!this.result) return;
    const status = ui("p", `query-result-status ${this.result.coverage} ${this.result.freshness}`, queryResultState(this.result)); status.setAttribute("role", "status"); this.resultsHost.append(status);
    if (this.exportResult) {
      const result = this.result;
      const output = action("输出本次查询结果", async () => {
        if (!result.queryId) { this.host.notify("本次结果没有可验证的冻结输出标识，请重新查询。", true); return; }
        await this.exportResult!({ queryId: result.queryId, label: result.snapshotLabel ?? "当前显示的查询", rowCount: result.rows.length, coverage: result.coverage, freshness: result.freshness, limited: result.limited });
      }, "data-button compact"); output.disabled = this.loading || !result.queryId || !!result.superseded; this.resultsHost.append(output);
    }
    if (this.result.cutId) this.resultsHost.append(ui("p", "data-help query-basis", `读取版本：${this.result.cutId}`));
    if (this.result.snapshotMode === "perFileObserved") this.resultsHost.append(ui("p", "data-help query-basis", "快照范围：逐文件观察，各文件绑定实际读取版本。读取期间的外部修改待下次刷新确认。"));
    if (this.result.snapshotMode === "admittedFrozen") this.resultsHost.append(ui("p", "data-help query-basis", `${this.result.snapshotLabel ?? "已纳管笔记冻结快照"}。结果绑定本次准确输入；刷新读取新快照，字段动作仍需重新准备当前目标。`));
    if (this.result.rawScope?.pendingCount) this.resultsHost.append(ui("p", "data-help", `另有 ${this.result.rawScope.pendingCount} 项待纳管材料，属于文件浏览范围。`));
    if (this.result.aggregateExact === false) this.resultsHost.append(ui("p", "data-warning", "汇总数值仅对应已知输入，不代表完整总数。"));
    if (this.result.diagnostics.length) { const diagnostics = ui("details", "query-diagnostics"); diagnostics.append(ui("summary", "", `${this.result.diagnostics.length} 项查询提示`)); this.result.diagnostics.forEach(item => diagnostics.append(ui("p", "data-warning", diagnosticText(item)))); this.resultsHost.append(diagnostics); }
    if (!this.result.rows.length) this.resultsHost.append(ui("p", "data-empty", this.result.coverage === "complete" && this.result.snapshotMode === "admittedFrozen" ? "本次已纳管冻结快照内没有匹配笔记。" : this.result.coverage === "complete" && this.result.freshness === "current" ? "当前条件没有匹配笔记。" : "当前已知范围没有匹配行；不能据此断言全库没有。"));
    const rows = pinEligibleRows(this.result.rows, this.preference.pinned);
    if (this.preference.view === "list") this.renderList(rows, this.resultsHost);
    else if (this.preference.view === "table") this.renderTable(rows, this.resultsHost);
    else if (this.preference.view === "board") this.renderBoard(rows);
    else if (this.preference.view === "calendar") this.renderCalendar(rows);
    else this.renderChart(rows);
    if (!["list", "table"].includes(this.preference.view)) { const alternative = ui("details", "query-accessible-table"); alternative.append(ui("summary", "", "查看等价数据表")); this.renderTable(rows, alternative); this.resultsHost.append(alternative); }
    this.resultsHost.scrollTop = scrollTop;
  }
  private rowOpen(row: QueryRow): HTMLButtonElement {
    const button = action(rowTitle(row), async () => {
      if (!row.path) { this.host.notify("汇总 / 生成行没有正文写入目标。", true); return; }
      try { const lifecycle = queryRowLifecycle(row, this.resultPlan); if (!lifecycle) throw new Error("此行缺少准确生命周期，不能猜测当前笔记代替原来源。"); let path = row.path; if (row.entityKey) { const opened = await this.openStrong(row.entityKey, lifecycle, row.revision); path = opened.path; } else { if (lifecycle === "trash") throw new Error("回收站行缺少可验证身份，不能按路径打开。"); await this.host.openDocument(path, this.root ?? undefined); } this.selected = row.entityKey ?? row.key; this.recent = [{ entityKey: this.selected, path, label: rowTitle(row), lifecycle, ...(lifecycle === "trash" ? { revision: row.revision } : {}) }, ...this.recent.filter(item => item.entityKey !== this.selected || item.lifecycle !== lifecycle)].slice(0, 12); localStorage.setItem(`weftext-query-recents:${this.libraryIdentity()}`, JSON.stringify(this.recent)); this.renderControls(); this.renderResults(); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); }
    }, "query-row-title"); button.disabled = !row.path; if (row.cells.title?.state !== "value") button.title = "使用文件名 / 路径显示回退；作者标题仍为空"; if ((row.entityKey ?? row.key) === this.selected) button.setAttribute("aria-current", "true"); return button;
  }
  private rowControls(row: QueryRow): HTMLElement {
    const controls = ui("div", "query-row-actions"); const key = row.entityKey ?? row.key; const pinned = this.preference.pinned.includes(key);
    const pin = action(pinned ? "取消置顶" : "置顶", () => { this.preference.pinned = pinned ? this.preference.pinned.filter(item => item !== key) : [...this.preference.pinned, key]; this.savePreferences(); this.renderResults(); }, "data-button compact"); pin.setAttribute("aria-pressed", String(pinned)); controls.append(pin);
    if (row.entityKey && row.path && queryRowLifecycle(row, this.resultPlan) === "live") controls.append(action("加到快捷入口", () => { if (!this.shortcuts.some(item => item.entityKey === row.entityKey)) this.shortcuts.push({ id: crypto.randomUUID(), label: rowTitle(row), kind: "node", entityKey: row.entityKey, path: row.path }); this.saveShortcuts(); this.renderControls(); }, "data-button compact"));
    const status = row.cells.status;
    if (!row.readOnly && row.entityKey && row.path && queryRowLifecycle(row, this.resultPlan) === "live" && this.schema && !this.result?.superseded && !this.result?.externalPending && (this.result?.freshness === "current" || this.result?.snapshotMode === "admittedFrozen") && this.host.canEdit?.() !== false) {
      const edit = ui("details", "query-inline-fields"); edit.append(ui("summary", "", "就地编辑真实属性"), ui("p", "data-help", "每次重新核对实际笔记与字段，修改进入该笔记草稿，可撤销；保存仍由原保存按钮完成。"));
      for (const [column, cell] of Object.entries(row.cells)) {
        const field = directQueryField(this.resultPlan, this.schema, column); if (!field || cell.state !== "value" || !cell.value) continue;
        const section = ui("section"), shape = resolveShape(field.shape, this.schema); if (!["scalar", "enum", "union"].includes(shape.kind) && field.key !== "tags") continue;
        const key = JSON.stringify([this.root, row.entityKey, field.id]), binding = JSON.stringify([field.shape, this.schema.definitions]), draft = this.inlineDrafts.get(key) ?? { value: structuredClone(cell.value), raw: {}, binding };
        section.append(ui("strong", "", fieldNames[field.key] ?? field.key));
        if (draft.binding !== binding) { section.append(ui("p", "data-warning", "字段形状已变化，原输入保留，未绑定同名新字段。"), ui("pre", "", JSON.stringify(draft, null, 2)), action("放弃此字段的旧输入", () => { this.inlineDrafts.delete(key); this.renderResults(); })); edit.append(section); continue; }
        section.append(valueEditor(field.shape, draft.value, this.schema, next => { draft.value = next; this.inlineDrafts.set(key, draft); }, `结果属性：${fieldNames[field.key] ?? field.key}`, 0, this.tagReferences));
        if (field.key === "tags" && this.root) section.append(action("读取完整标签选择目录", async () => { const root = this.root!, generation = this.generation; await loadTagFormReferences(this.host, root, this.tagReferences, this.element, () => root === this.root && generation === this.generation); }));
        for (const [id, control] of nativeFormControls(section)) if (id in draft.raw) { control.value = draft.raw[id]; control.dispatchEvent(new Event("input", { bubbles: true })); }
        const retain = () => { draft.raw = Object.fromEntries(nativeFormControls(section).filter(([, control]) => !control.validity.valid).map(([id, control]) => [id, control.value])); this.inlineDrafts.set(key, draft); }; section.addEventListener("input", retain); section.addEventListener("change", retain);
        section.append(action("应用此字段到笔记草稿", async () => { const invalid = section.querySelector<HTMLInputElement>(":invalid"); if (invalid) { invalid.reportValidity(); return; } await this.changeTaskField(row, field.key, draft.value, field.id, () => { this.inlineDrafts.delete(key); }); })); edit.append(section);
      }
      if (edit.querySelector("section")) controls.append(edit);
    }
    if (!row.readOnly && queryRowLifecycle(row, this.resultPlan) === "live" && row.path && hasDirectStatusProjection(this.resultPlan, this.schema) && status?.state === "value" && status.value?.kind === "text" && ["open", "done", "cancelled"].includes(status.value.value)) {
      const next = status.value.value === "done" ? "open" : "done"; controls.append(action(`状态 → ${next === "done" ? "已完成" : "未完成"}`, () => this.changeTaskField(row, "status", { kind: "text", value: next }), "data-button compact"));
    }
    return controls;
  }
  private renderList(rows: QueryRow[], container: HTMLElement): void {
    const list = ui("div", "query-list"); for (const row of rows) { const card = ui("article", `query-card${(row.entityKey ?? row.key) === this.selected ? " selected" : ""}`); card.append(this.rowOpen(row)); if (row.cells.title?.state !== "value") card.append(ui("span", "query-fallback", "文件名回退"));
      if (this.preference.summaries) { const summary = row.cells.summary; card.append(ui("p", "query-excerpt", summary?.state === "value" ? formatNativeValue(summary.value).slice(0, 180) || "正文为空" : `正文摘要：${summary ? cellLabels[summary.state] : "未投影"}`)); }
      const metadata = ui("dl", "query-card-fields"); for (const [name, cell] of Object.entries(row.cells).filter(([name]) => !["title", "summary", "types"].includes(name))) { metadata.append(ui("dt", "", fieldNames[name] ?? name), ui("dd", `cell-${cell.state}`, stateText(cell))); } card.append(metadata);
      if (this.preference.thumbnails) { const thumbnail = ui("div", "query-thumbnail-host"); card.append(thumbnail); this.renderThumbnail(thumbnail, row); }
      card.append(this.rowControls(row)); list.append(card); } container.append(list);
  }
  private renderThumbnail(container: HTMLElement, row: QueryRow): void {
    if (queryRowLifecycle(row, this.resultPlan) !== "live") { container.append(ui("p", "query-preview-state", "缩略图不可用；此入口仅提供已确认当前生命周期的资源预览。")); return; }
    if (!row.entityKey || !row.revision || !this.root || !this.host.requestBlob) { container.append(ui("p", "query-preview-state", "缩略图不可用（缺少准确笔记版本或资源读取入口）")); return; }
    const root = this.root, key = `${root}:${row.entityKey}:${row.revision}`;
    const thumbnail = mountListThumbnail(container, { viewport:this.resultsHost,
      resolve: () => { let pending=this.previewCache.get(key); if(!pending){pending=this.host.request<RowPreview>("/api/query/preview",{root,entityKey:row.entityKey,revision:row.revision});this.previewCache.set(key,pending);}return pending; },
      read: (descriptor, revision) => this.host.requestBlob!("/api/query/preview-resource",{root,entityKey:row.entityKey,revision,resource:descriptor.resource,resourceRevision:descriptor.revision}),
    }); this.thumbnailDisposers.push(() => thumbnail.dispose());
  }
  private renderTable(rows: QueryRow[], container: HTMLElement): void {
    const columns = [...new Set([...(this.result?.columns ?? []), ...rows.flatMap(row => Object.keys(row.cells))])].filter(column => column !== "summary"); const table = ui("table", "query-table"); const caption = ui("caption", "", `${this.result?.coverage === "partial" ? "已知部分" : "当前"}查询数据 · ${rows.length} 行`); table.append(caption); const head = ui("thead"); const heading = ui("tr"); heading.append(ui("th", "", "笔记 / 行")); columns.filter(column => column !== "title").forEach(column => { const cell = ui("th", "", fieldNames[column] ?? column); cell.scope = "col"; heading.append(cell); }); heading.append(ui("th", "", "操作")); head.append(heading); table.append(head); const body = ui("tbody");
    for (const row of rows) { const line = ui("tr"); const title = ui("th"); title.scope = "row"; title.append(this.rowOpen(row)); line.append(title); for (const column of columns.filter(column => column !== "title")) line.append(ui("td", `cell-${row.cells[column]?.state ?? "absent"}`, stateText(row.cells[column]))); const controls = ui("td"); controls.append(this.rowControls(row)); line.append(controls); body.append(line); } table.append(body); const scroll = ui("div", "query-table-scroll"); scroll.append(table); container.append(scroll);
  }
  private renderBoard(rows: QueryRow[]): void {
    const column = this.preference.groupField; const groups = new Map<string, { label: string; rows: QueryRow[] }>(); for (const row of rows) { const cell = row.cells[column]; const key = cell?.state === "value" && cell.value ? canonicalNative(cell.value) : `@${cell?.state ?? "unprojected"}`; const group = groups.get(key) ?? { label: stateText(cell), rows: [] }; group.rows.push(row); groups.set(key, group); }
    this.resultsHost.append(ui("p", "data-help", `看板按 ${fieldNames[column] ?? column} 展示当前查询行；缺值与错误各自分组。`)); const board = ui("div", "query-board");
    for (const group of groups.values()) { const lane = ui("section", "query-board-lane"); lane.append(ui("h3", "", `${group.label} · ${group.rows.length}`)); this.renderList(group.rows, lane); board.append(lane); } this.resultsHost.append(board);
  }
  private renderCalendar(rows: QueryRow[]): void {
    const column = this.preference.dateField; const days = new Map<string, QueryRow[]>(); const undated: QueryRow[] = [];
    for (const row of rows) { const cell = row.cells[column]; const day = cell?.state === "value" ? calendarDay(cell.value) : null; if (!day) undated.push(row); else days.set(day, [...(days.get(day) ?? []), row]); }
    this.resultsHost.append(ui("p", "data-help", `日历使用 ${fieldNames[column] ?? column} 的准确民用日期。缺值、部分精度、时刻或错误保留在下方，未添加本机时区。`)); const calendar = ui("div", "query-calendar");
    for (const [day, items] of [...days].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)) { const cell = ui("section", "query-calendar-day"); const time = ui("time", "", day); time.dateTime = day; const heading = ui("h3"); heading.append(time); cell.append(heading); for (const row of items) cell.append(this.rowOpen(row), this.rowControls(row)); calendar.append(cell); } this.resultsHost.append(calendar);
    if (undated.length) { const other = ui("details", "query-undated"); other.open = true; other.append(ui("summary", "", `不能放入单日格（${undated.length} 项）`)); this.renderTable(undated, other); this.resultsHost.append(other); }
  }
  private renderChart(rows: QueryRow[]): void {
    const column = this.preference.chartField; const samples = rows.flatMap(row => { const cell = row.cells[column]; if (cell?.state !== "value" || !cell.value || !["int", "decimal", "float"].includes(cell.value.kind)) return []; const value = cell.value as Extract<NativeValue, { value: string }>; const amount = Number(value.value); return Number.isFinite(amount) ? [{ row, value, amount }] : []; });
    this.resultsHost.append(ui("p", "data-help", `图表展示 ${fieldNames[column] ?? column}。条形长度用于视觉比较，标签保留准确数值；未添加隐式汇总。`));
    if (!samples.length) { this.resultsHost.append(ui("p", "data-empty", "此投影没有可绘制数值。可选择数值字段，或在筛选面板明确运行计数 / 汇总。")); return; }
    const max = Math.max(...samples.map(item => Math.abs(item.amount)), 1); const chart = ui("div", "query-chart"); for (const sample of samples) { const row = ui("div", "query-chart-row"); const label = sample.row.path ? this.rowOpen(sample.row) : ui("span", "", Object.entries(sample.row.cells).filter(([name]) => name !== column).map(([, cell]) => stateText(cell)).join(" · ") || "汇总"); const track = ui("div", "query-chart-track"); const bar = ui("div", `query-chart-bar${sample.amount < 0 ? " negative" : ""}`); bar.style.width = `${Math.max(0.5, Math.abs(sample.amount) / max * 100)}%`; track.setAttribute("aria-hidden", "true"); track.append(bar); row.append(label, track, ui("span", "query-chart-value", formatNativeValue(sample.value))); chart.append(row); } this.resultsHost.append(chart);
    if (samples.length !== rows.length) this.resultsHost.append(ui("p", "data-warning", `${rows.length - samples.length} 行缺值、值无效或不是数值；仍可在等价数据表查看。`));
  }
  private async changeTaskField(row: QueryRow, key: string, value: NativeValue, expectedFieldId = this.schema?.fields.find(field => field.key === key)?.id, applied?: () => void): Promise<void> {
    if (this.host.canEdit?.() === false) { this.host.notify("当前会话不能写入属性，输入保留。", true); return; }
    if (row.readOnly || !row.path || queryRowLifecycle(row, this.resultPlan) !== "live") { this.host.notify("这个行没有真实当前字段写入目标。", true); return; }
    try {
      await this.host.prepareAction?.();
      const target = row.entityKey ? await this.openStrong(row.entityKey) : { path: row.path, nodeId: undefined }; if (!row.entityKey) await this.host.openDocument(target.path, this.root ?? undefined); const basis = this.host.getDocument(); if (!basis || (target.nodeId ? basis.nodeId !== target.nodeId : basis.path !== target.path)) throw new Error("当前文档目标身份不匹配，未修改字段。");
      const projection = decodeProjection(await this.host.request<NoteProjection | { projection: NoteProjection }>("/api/data/project", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial })); const projected = projection.fields.find(field => field.key === key); if (!projected?.writable || projected.cell.state !== "value" || !expectedFieldId || projected.fieldId !== expectedFieldId) throw new Error("当前字段不可安全编辑或身份已变化。旧行不授予写入资格。");
      if (!await this.confirmChange(key, projected.cell.value, value)) return;
      const result = await this.host.request<SourcePatch | { edit: SourcePatch } | { patches: SourcePatch[]; source?: string }>("/api/data/edit", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial, key, path: [], value }); await this.host.applyPatch(decodeSourcePatch(result, basis.source), basis); applied?.(); if (this.result) { this.result.freshness = "stale"; this.result.superseded = true; } this.renderResults(); this.host.notify(`${fieldNames[key] ?? key} 已应用到真实笔记草稿，保存后写入文件。列表需刷新读取磁盘现状。`);
    } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); }
  }
  private async openTarget(path: string, entityKey?: string, lifecycle: "live" | "trash" = "live", revision?: string): Promise<void> {
    try { if (entityKey) await this.openStrong(entityKey, lifecycle, revision); else { if (lifecycle === "trash") throw new Error("回收站目标缺少稳定身份。"); await this.host.openDocument(path, this.root ?? undefined); } }
    catch (error) { this.host.notify(`入口目标不可用：${error instanceof Error ? error.message : String(error)}。未按路径或同名目标替换。`, true); }
  }
  private async openStrong(entityKey: string, lifecycle: "live" | "trash" = "live", revision?: string): Promise<OpenedDocument> { if (!this.host.openResolved) throw new Error("当前会话没有稳定目标打开入口；未按旧路径替换。"); if (lifecycle === "trash" && !revision) throw new Error("回收站目标缺少准确来源依据；未改读当前笔记。"); const opened = await this.host.request<OpenedDocument>("/api/data/open", { root: this.root, entityKey, lifecycle, ...(lifecycle === "trash" ? { revision } : {}) }); await this.host.openResolved(opened); return opened; }
  private async confirmChange(key: string, before: NativeValue | undefined, after: NativeValue): Promise<boolean> {
    return await new Promise(resolve => { const dialog = ui("dialog", "data-confirm-dialog"); const form = ui("form"); form.method = "dialog"; form.append(ui("h2", "", `修改字段：${fieldNames[key] ?? key}`), ui("p", "", `${formatNativeValue(before)} → ${formatNativeValue(after)}`), ui("p", "data-help", "只更新当前真实笔记的这个字段，正文保持在同一草稿。")); const cancel = ui("button", "data-button", "取消"); cancel.value = "cancel"; const apply = ui("button", "data-button primary", "应用到草稿"); apply.value = "apply"; form.append(cancel, apply); dialog.append(form); document.body.append(dialog); dialog.addEventListener("close", () => { const accepted = dialog.returnValue === "apply"; dialog.remove(); resolve(accepted); }, { once: true }); dialog.showModal(); });
  }
  private async showCapture(): Promise<void> {
    if (!this.root || !this.workspaceId) return; if (this.captureDialog) { this.captureForms.get(this.root)?.updateContext(); this.captureDialog.showModal(); return; } if (this.scopeIdentity) { try { await this.resolveScopeLocator(this.generation); } catch (error) { this.host.notify(error instanceof Error ? error.message : String(error), true); return; } } const dialog = ui("dialog", "capture-dialog"); this.captureDialog = dialog; this.captureDialogs.set(this.root, dialog); const captureHost: PanelHost = { ...this.host, ...(this.host.openTemplates ? { openTemplates: async () => { dialog.close(); await this.host.openTemplates!(); } } : {}) }; const capture = new QuickCapture(captureHost, { root: this.root, parent: this.directory ?? this.root }); this.captureForms.set(this.root, capture); dialog.append(action("关闭并保留本次记录", () => { dialog.close(); }, "data-button capture-close"), capture.element); document.body.append(dialog); dialog.showModal(); await capture.initialize();
  }
}
