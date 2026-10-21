import { AuthorInputScopes, type AuthorInputFactory } from "./author-input";
import { action, emptyNativeValue, formatNativeValue, ui, valueEditor, type Cell, type DataSchema, type NativeValue, type Shape, type FormReferences } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { TagTree } from "./tag-tree";
import { registryIntent, registryReceipt, type RegistryCatalog, type RegistryReceipt, type RegistryRequest } from "./hosted-navigation-registry-state";
import { tagPicker } from "./tag-picker";

type Grant = { resource: string; fieldId: string; write: boolean };
type Profile = { resource: string; basis: string | null; field: { fieldId: string; key: string; shape: Shape; nullable: boolean; required: boolean; cell: Cell; writable: boolean } };
type Proposal = { resource: string; fieldId: string; basis: string; value: NativeValue; operationId: string };
type TagCreation = { request: RegistryRequest; originalValue: NativeValue; receipt?: RegistryReceipt; phase?: "prepare" | "commit" | "settle" };
type Draft = { value: NativeValue; shape?: Shape; rawInputs?: Record<string, string>; pending?: Proposal; newTag?: { displayName: string; parentRef: string | null }; tagCreation?: TagCreation };
type Receipt = { status: string; cell?: Cell };
type Request = <T>(path: string, data?: unknown) => Promise<T>;
const schema: DataSchema = { definitions: {}, fields: [], types: [], aliases: {} };

export class HostedFields {
  private epoch = 0;
  private inputScopes: AuthorInputScopes;
  private prefix = "";
  private selected?: Grant;
  private profile?: Profile;
  private draft?: Draft;
  private compare = false;
  private busy = false;
  private grants: Grant[] = [];
  private form?: HTMLFieldSetElement;
  private formDraft?: Draft;
  private unreadableDraft?: string;
  private references: FormReferences = { nodes: [], state: "unavailable", messages: [] };
  private tagCatalog?: RegistryCatalog;
  private verifiedTagOperation?: string;
  private readonly picker = ui("select");
  private readonly editor = ui("section");
  private readonly message = ui("p", "hint");
  private readonly results = ui("section");
  constructor(private host: HTMLElement, private request: Request, private onQuery?: (id: string, metadata: { columns: string[]; rowCount: number; coverage: string }) => void, private changed?: () => void, authorInputs?: AuthorInputFactory) { this.inputScopes = new AuthorInputScopes(authorInputs); this.clear(false); }
  canEdit(resource: string, fieldId: string): boolean { return this.grants.some(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write); }
  async openGranted(resource: string, fieldId: string): Promise<void> { const grant = this.grants.find(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write); if (!grant) throw new Error("此字段没有独立写入权限"); await this.run(() => this.open(grant)); this.host.scrollIntoView({ block: "start" }); this.editor.querySelector<HTMLElement>("input,select,textarea")?.focus({ preventScroll: true }); }
  clear(removeCache = true) {
    this.inputScopes.clear(); this.epoch++; this.profile = undefined; this.selected = undefined; this.draft = undefined; this.references = { nodes: [], state: "unavailable", messages: [] }; this.tagCatalog = undefined; this.verifiedTagOperation = undefined;
    this.grants = []; this.busy = false; this.form = undefined; this.formDraft = undefined; this.unreadableDraft = undefined; this.picker.replaceChildren(); this.editor.replaceChildren(); this.results.replaceChildren();
    this.host.replaceChildren(); this.host.hidden = true; this.prefix = "";
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-field:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false);
    const epoch = this.epoch;
    const { fields } = await this.request<{ fields: Grant[] }>("fields");
    if (epoch !== this.epoch) return;
    this.prefix = hostedCachePrefix("field", deployment, actor, generation);
    this.grants = fields; this.host.hidden = !fields.length;
    this.host.replaceChildren(ui("h2", "", "获权字段与查询"), ui("p", "hint", "字段输入单独保存；查询仅使用当前获权字段。"));
    this.picker.setAttribute("aria-label", "选择获权字段");
    this.picker.replaceChildren(...fields.map((f, i) => { const option = ui("option", "", `${f.resource} · ${f.fieldId}`); option.value = String(i); return option; }));
    this.host.append(this.picker, action("读取字段", () => this.run(() => this.open(fields[Number(this.picker.value)]))), this.message, this.editor);
    const queryField = ui("select"); queryField.setAttribute("aria-label", "查询字段");
    for (const id of new Set(fields.map(f => f.fieldId))) { const option = ui("option", "", id); option.value = id; queryField.append(option); }
    const predicate = ui("input"); predicate.setAttribute("aria-label", "CEL 筛选条件"); predicate.placeholder = 'CEL 筛选，例如 status == "open"（可留空）';
    this.host.append(ui("h3", "", "字段 Query"), queryField, predicate, action("运行查询", () => this.run(async () => {
      const epoch = this.epoch;
      const response = await this.request<{ queryId: string; columns: string[]; result: { rows: { entityKey?: string; cells: Record<string, Cell> }[]; coverage: string; limited: boolean } }>("query", { fields: [queryField.value], plan: { where: predicate.value || null } });
      if (epoch !== this.epoch) return;
      const table = ui("table"); table.setAttribute("aria-label", "获权字段查询结果");
      for (const row of response.result.rows) {
        const tr = ui("tr"); tr.append(ui("th", "", row.entityKey ?? "分组"));
        for (const cell of Object.values(row.cells)) tr.append(ui("td", "", this.cellText(cell)));
        table.append(tr);
      }
      this.results.replaceChildren(ui("p", "", `获权范围 ${response.result.rows.length} 行${response.result.coverage === "partial" || response.result.limited ? " · 结果未完整" : ""}`), table);
      this.onQuery?.(response.queryId, { columns: response.columns, rowCount: response.result.rows.length, coverage: response.result.coverage });
    })), this.results);
  }
  private key(g = this.selected!) { return this.prefix + encodeURIComponent(g.resource) + ":" + encodeURIComponent(g.fieldId); }
  private controls(form: HTMLFieldSetElement) {
    const seen = new Map<string, number>();
    return [...form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input, select, textarea")].map(control => {
      const label = control.getAttribute("aria-label") ?? control.labels?.[0]?.textContent ?? "";
      const name = `${control.tagName}:${control.type}:${label}`; const ordinal = seen.get(name) ?? 0; seen.set(name, ordinal + 1);
      return { key: `${name}:${ordinal}`, control };
    });
  }
  private captureInputs() {
    if (!this.form || this.formDraft !== this.draft || !this.draft) return;
    const raw = { ...this.draft.rawInputs };
    for (const { key, control } of this.controls(this.form)) { if (!control.validity.valid) raw[key] = control.value; else delete raw[key]; }
    if (Object.keys(raw).length) this.draft.rawInputs = raw; else delete this.draft.rawInputs;
  }
  private restoreInputs(form: HTMLFieldSetElement, draft: Draft) {
    for (const { key, control } of this.controls(form)) if (draft.rawInputs && key in draft.rawInputs) {
      control.value = draft.rawInputs[key]; control.dispatchEvent(new Event("input", { bubbles: true }));
    }
  }
  private unmappedInputs(form: HTMLFieldSetElement, draft: Draft) {
    const keys = new Set(this.controls(form).map(({ key }) => key));
    return Object.entries(draft.rawInputs ?? {}).filter(([key]) => !keys.has(key));
  }
  private retain() { if (this.selected && this.draft) { this.captureInputs(); sessionStorage.setItem(this.key(), JSON.stringify(this.draft)); } }
  private cellText(cell: Cell) { return cell.value ? formatNativeValue(cell.value) : ({ absent: "未填写", null: "空值", invalid: "无效值", unavailable: "不可用", value: "" })[cell.state]; }
  private async run(work: () => Promise<void>) {
    if (this.busy) return; this.busy = true; const epoch = this.epoch; this.render();
    try { await work(); } catch (e) { if (epoch === this.epoch) this.message.textContent = e instanceof Error ? e.message : "操作未完成，输入已保留。"; }
    finally { if (epoch === this.epoch) { this.busy = false; this.render(); } }
  }
  private async open(grant: Grant) {
    if (!grant) return;
    this.retain(); this.inputScopes.clear();
    const epoch = this.epoch;
    const stored = sessionStorage.getItem(this.key(grant));
    this.selected = grant; this.profile = undefined; this.draft = undefined; this.unreadableDraft = undefined; this.tagCatalog = undefined; this.verifiedTagOperation = undefined;
    try { this.draft = stored ? JSON.parse(stored) as Draft : undefined; const job = this.draft?.tagCreation; if (job && (!job.request || typeof job.request.operationId !== "string" || !job.originalValue || job.request.intent?.kind !== "createTag" || job.request.intent.assignTo?.resource !== grant.resource || job.request.intent.assignTo?.fieldId !== grant.fieldId)) throw new Error("原标签操作不属于此字段。"); if (job?.receipt) registryReceipt(job.request, job.receipt); }
    catch { this.unreadableDraft = stored!; throw new Error("字段恢复记录无法解析，原记录仍保留；请先复制下方原文。"); }
    this.compare = !!stored;
    this.message.textContent = this.draft?.pending ? "有一次结果待核对的原操作；可先核对原操作，勿再次提交。" : "正在读取当前字段；原输入仍保留。";
    this.render();
    const profile = await this.request<Profile>("fields/read", { resource: grant.resource, fieldId: grant.fieldId });
    if (epoch !== this.epoch) return;
    this.profile = profile;
    this.references = { nodes: [], state: "unavailable", messages: [] };
    // Registry visibility is separate from field write permission; lack of tags.read
    // never removes an existing typed reference or prevents an exact field write.
    try { const catalog = await this.request<RegistryCatalog>("navigation/registry"); if (epoch !== this.epoch) return; this.references.tagTree = new TagTree(catalog.tags.items, catalog.tags.read && catalog.tags.coverage === "complete"); this.tagCatalog = catalog; this.references.state = "ready"; } catch (error) { if (epoch !== this.epoch) return; this.references.messages.push(String(error)); }
    this.draft ??= { value: profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? emptyNativeValue(profile.field.shape, schema) };
    this.draft.shape = profile.field.shape;
    this.message.textContent = this.draft.pending ? "有一次结果待核对的原操作；请先核对，勿再次提交。" : stored ? "输入已恢复，请与当前字段核对后采用新依据。" : "字段已读取，输入尚未保存。";
  }
  private render() {
    this.captureInputs(); this.form = undefined; this.formDraft = undefined;
    this.editor.replaceChildren(); const profile = this.profile, draft = this.draft;
    if (this.unreadableDraft && this.selected) {
      this.editor.append(ui("h3", "", `${this.selected.resource} · ${this.selected.fieldId}`), ui("pre", "", this.unreadableDraft), action("放弃无法解析的恢复记录", () => {
        sessionStorage.removeItem(this.key()); this.unreadableDraft = undefined; return this.run(() => this.open(this.selected!));
      })); return;
    }
    if (!draft || !this.selected) return;
    const selected = this.selected, epoch = this.epoch, inputs = this.inputScopes.get(draft, { kind: "field", resource: selected.resource, fieldId: selected.fieldId }, () => this.epoch === epoch && this.draft === draft && this.selected === selected);
    this.editor.append(ui("h3", "", `${this.selected.resource} · ${profile?.field.key ?? this.selected.fieldId}`));
    this.editor.append(ui("p", "", profile ? `当前字段：${this.cellText(profile.field.cell)}` : "当前字段未读取成功；保留的输入与原操作仍可核对。"));
    const form = ui("fieldset"); form.disabled = this.busy || !profile?.field.writable || !!draft.pending || !!draft.tagCreation;
    const shape = profile?.field.shape ?? draft.shape;
    if (shape) form.append(valueEditor(shape, draft.value, schema, value => {
      draft.value = value;
      try { this.retain(); this.message.textContent = "字段输入已保留，尚未保存。"; } catch { this.message.textContent = "无法保留恢复信息，请先复制输入。"; }
    }, "字段输入", 0, this.references, inputs ? { session: inputs, slot: "value", path: [] } : undefined));
    else form.append(ui("pre", "", formatNativeValue(draft.value)));
    if (profile?.field.nullable) form.append(action("设为空值", () => { draft.value = { kind: "none" }; this.retain(); this.render(); }));
    this.editor.append(form);
    this.restoreInputs(form, draft); this.form = form; this.formDraft = draft;
    const unmapped = this.unmappedInputs(form, draft);
    if (unmapped.length) this.editor.append(ui("p", "hint", "原输入未能对应当前表单；请核对并重新输入，或明确放弃这些原文后再保存。"), ...unmapped.map(([, text]) => ui("pre", "", text)), action("放弃未匹配的原输入", () => {
      for (const [key] of unmapped) delete draft.rawInputs![key]; this.retain(); this.render();
    }));
    form.addEventListener("input", () => { try { this.retain(); this.message.textContent = draft.rawInputs ? "字段输入尚未完成，原文已保留；请完成输入后再保存。" : "字段输入已保留，尚未保存。"; } catch { this.message.textContent = "无法保留恢复信息，请先复制输入。"; } });
    form.addEventListener("click", event => {
      if (!(event.target instanceof Element) || !event.target.closest("button")) return;
      const invalid = this.controls(form).find(({ control }) => !control.validity.valid)?.control;
      if (invalid) { event.preventDefault(); event.stopPropagation(); invalid.reportValidity(); this.message.textContent = "请先完成无效输入，再调整字段结构。"; }
    }, { capture: true });
    if (!profile?.field.writable) this.editor.append(ui("p", "hint", "当前字段只读；需确认静态值、已安装 Schema 与校验依赖授权。"));
    const save = action("保存字段", () => {
      if (this.unmappedInputs(form, draft).length) return;
      if (this.controls(form).some(({ control }) => !control.reportValidity())) { this.message.textContent = "字段输入无效，原文已保留；请完成输入后再保存。"; return; }
      return this.run(async () => {
        if (!profile?.basis || draft.pending || this.compare) return;
        const epoch = this.epoch;
        draft.pending = { resource: profile.resource, fieldId: profile.field.fieldId, basis: profile.basis, value: structuredClone(draft.value), operationId: crypto.randomUUID() };
        this.retain(); // Must succeed before any author request.
        const result = await this.request<Receipt>("fields/write", draft.pending);
        if (epoch !== this.epoch) return;
        this.applyReceipt(result);
      });
    }); save.disabled = this.busy || !profile?.basis || this.compare || !!draft.pending || !!draft.tagCreation || !!unmapped.length;
    const reread = action("重新读取字段", () => this.run(() => this.open(this.selected!))); reread.disabled = this.busy || !!draft.pending || !!draft.tagCreation;
    const rebase = action("已核对，采用当前字段依据", () => { this.compare = false; this.message.textContent = "依据已确认，输入保留；点击保存字段才提交。"; this.render(); });
    rebase.disabled = this.busy || !!draft.pending || !this.compare || !profile?.basis;
    const recover = action("核对原字段操作", () => this.run(async () => {
      if (!draft.pending) return;
      const epoch = this.epoch; const req = { resource: draft.pending.resource, fieldId: draft.pending.fieldId, operationId: draft.pending.operationId };
      let result = await this.request<Receipt>("fields/operation", req);
      if (epoch !== this.epoch) return;
      if (["notRegistered", "notCommitted"].includes(result.status)) result = await this.request<Receipt>("fields/settle", req);
      if (epoch !== this.epoch) return;
      this.applyReceipt(result);
    })); recover.disabled = this.busy || !draft.pending;
    this.editor.append(save, reread, rebase, recover);
    if (profile?.field.key === "tags" || draft.tagCreation) this.renderTagCreation();
  }
  private renderTagCreation(): void {
    const draft = this.draft!, profile = this.profile, section = ui("section"); section.append(ui("h3", "", "新建标签并首次赋值 · 同一原操作")); draft.newTag ??= { displayName: "", parentRef: null };
    const form = ui("fieldset"); form.disabled = this.busy || !!draft.pending || !!draft.tagCreation || !profile?.field.writable || this.compare;
    const name = ui("input", "data-input"); name.setAttribute("aria-label", "新建并赋值标签名称"); name.value = draft.newTag.displayName; const selected = this.selected!, epoch = this.epoch; this.inputScopes.get(draft, { kind: "field", resource: selected.resource, fieldId: selected.fieldId }, () => this.epoch === epoch && this.draft === draft && this.selected === selected)?.bind(name, "new-tag-name"); const duplicateNames = ui("p", "hint"); const names = () => { const count = this.tagCatalog?.tags.items.filter(tag => tag.displayName === name.value).length ?? 0; duplicateNames.textContent = count ? `已有 ${count} 个同名标签，准备后仍可明确创建新身份，不合并。请先在选择器中核对。` : ""; }; name.oninput = () => { draft.newTag!.displayName = name.value; this.retain(); names(); }; names(); form.append(name, duplicateNames);
    if (this.references.tagTree) form.append(tagPicker(this.references.tagTree, { label: "首次赋值新标签的父级", value: draft.newTag.parentRef ?? "", root: true, changed: parentRef => { draft.newTag!.parentRef = parentRef; this.retain(); } }));
    else form.append(ui("p", "hint", "完整标签结构不可用；保留字段输入，暂停新建并赋值。"));
    const prepare = action("预览新建并赋值", () => this.run(async () => {
      if (!profile?.basis || !profile.field.writable || this.compare || draft.pending || draft.tagCreation || !this.tagCatalog || !draft.newTag) return;
      const saved = profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? emptyNativeValue(profile.field.shape, schema);
      if (draft.rawInputs || JSON.stringify(saved) !== JSON.stringify(draft.value)) throw new Error("字段还有未保存输入。请先明确保存或放弃字段修改；新建不会替你保存这些输入。");
      const intent = registryIntent({ kind: "createTag", reference: "", slot: "", ...draft.newTag }, this.tagCatalog); if (intent.kind !== "createTag") return;
      intent.assignTo = { resource: profile.resource, fieldId: profile.field.fieldId, basis: profile.basis };
      const job: TagCreation = { request: { operationId: crypto.randomUUID(), intent }, originalValue: structuredClone(draft.value), phase: "prepare" }; draft.tagCreation = job; this.retain(); const epoch = this.epoch;
      const receipt = await this.request<RegistryReceipt>("navigation/registry/prepare", structuredClone(job.request)); if (epoch !== this.epoch || this.draft !== draft) return; await this.receiveTagCreation(receipt);
    })); prepare.disabled = !this.references.tagTree || !this.tagCatalog?.tags.create; form.append(prepare); section.append(form);
    const job = draft.tagCreation;
    if (job) {
      const receipt = this.verifiedTagOperation === job.request.operationId ? job.receipt : undefined;
      section.append(ui("p", "hint", `原操作 ${job.request.operationId} · ${receipt?.status ?? "待当前授权核对"}`)); if (receipt) section.append(ui("pre", "", JSON.stringify(receipt.preview, null, 2)));
      const commit = action("明确提交原新建与赋值计划", () => this.run(async () => { if (!receipt?.commitAllowed || !receipt.planId || job.phase) return; job.phase = "commit"; this.retain(); const epoch = this.epoch; const next = await this.request<RegistryReceipt>("navigation/registry/commit", { operationId: job.request.operationId, planId: receipt.planId }); if (epoch !== this.epoch || this.draft !== draft) return; await this.receiveTagCreation(next); })); commit.disabled = this.busy || !!job.phase || !receipt?.commitAllowed;
      const lookup = action("核对原标签新建与赋值操作", () => this.run(async () => { const epoch = this.epoch; const next = await this.request<RegistryReceipt>("navigation/registry/operation", { operationId: job.request.operationId }); if (epoch !== this.epoch || this.draft !== draft) return; await this.receiveTagCreation(next); })); lookup.disabled = this.busy;
      const cancel = action("取消原未提交新建与赋值", () => this.run(async () => { job.phase = "settle"; this.retain(); const epoch = this.epoch; const next = await this.request<RegistryReceipt>("navigation/registry/settle", { operationId: job.request.operationId }); if (epoch !== this.epoch || this.draft !== draft) return; await this.receiveTagCreation(next); })); cancel.disabled = this.busy || !!receipt && ["committed", "cancelled", "resolved"].includes(receipt.status);
      const fresh = action("结束已核对记录，继续字段编辑", () => { delete draft.tagCreation; this.verifiedTagOperation = undefined; this.retain(); this.render(); }); fresh.disabled = this.busy || !receipt || !["committed", "cancelled", "resolved"].includes(receipt.status); section.append(commit, lookup, cancel, fresh);
    }
    section.append(ui("p", "hint", "只写新标签的直接引用；父级不自动赋值。准备、提交、取消和重启均核对同一编号，正文草稿独立保留。")); this.editor.append(section);
  }
  private async receiveTagCreation(incoming: RegistryReceipt): Promise<void> {
    const draft = this.draft!, job = draft.tagCreation!; job.receipt = registryReceipt(job.request, incoming, job.receipt); job.phase = undefined; this.verifiedTagOperation = job.request.operationId; this.retain();
    if (incoming.status === "committed" && job.request.intent.kind === "createTag" && job.request.intent.assignTo) {
      this.changed?.();
      const epoch = this.epoch, selected = job.request.intent.assignTo, profile = await this.request<Profile>("fields/read", { resource: selected.resource, fieldId: selected.fieldId }); if (epoch !== this.epoch || this.draft !== draft) return;
      this.profile = profile; this.compare = true;
      if (JSON.stringify(draft.value) === JSON.stringify(job.originalValue)) draft.value = profile.field.cell.state === "null" ? { kind: "none" } : profile.field.cell.value ?? draft.value;
      this.references.tagTree = undefined; this.tagCatalog = undefined; this.retain(); this.message.textContent = "原新建与首次赋值已提交。字段已回读；其他草稿与期间输入保留，继续写入前请核对新依据。";
    } else this.message.textContent = incoming.status === "prepared" ? "新标签与首次赋值同一计划已准备，请核对后明确提交。" : "原标签操作事实已核对，未将未提交状态冒作成功。";
  }
  private applyReceipt(receipt: Receipt) {
    if (!this.draft) return;
    if (["committed", "cancelled", "resolved"].includes(receipt.status)) {
      if (receipt.status === "committed") this.changed?.();
      delete this.draft.pending; this.compare = true; if (this.profile) this.profile.basis = null;
      if (receipt.status === "committed" && receipt.cell?.value) this.draft.value = receipt.cell.value;
      if (receipt.status === "committed" && receipt.cell?.state === "null") this.draft.value = { kind: "none" };
      this.retain();
      this.message.textContent = receipt.status === "committed" ? "原字段操作已保存。继续编辑前，请重新读取并核对当前字段。" : receipt.status === "resolved" ? "原操作已明确结束，未确认提交；输入保留。请重新读取并核对。" : "原操作已取消，输入保留。请重新读取并核对。";
    } else this.message.textContent = "原操作尚未确认，请保留输入并继续核对。";
  }
}
