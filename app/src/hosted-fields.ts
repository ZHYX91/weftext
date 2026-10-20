import { action, emptyNativeValue, formatNativeValue, ui, valueEditor, type Cell, type DataSchema, type NativeValue, type Shape } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";

type Grant = { resource: string; fieldId: string; write: boolean };
type Profile = { resource: string; basis: string | null; field: { fieldId: string; key: string; shape: Shape; nullable: boolean; required: boolean; cell: Cell; writable: boolean } };
type Proposal = { resource: string; fieldId: string; basis: string; value: NativeValue; operationId: string };
type Draft = { value: NativeValue; shape?: Shape; rawInputs?: Record<string, string>; pending?: Proposal };
type Receipt = { status: string; cell?: Cell };
type Request = <T>(path: string, data?: unknown) => Promise<T>;
const schema: DataSchema = { definitions: {}, fields: [], types: [], aliases: {} };

export class HostedFields {
  private epoch = 0;
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
  private readonly picker = ui("select");
  private readonly editor = ui("section");
  private readonly message = ui("p", "hint");
  private readonly results = ui("section");
  constructor(private host: HTMLElement, private request: Request, private onQuery?: (id: string, metadata: { columns: string[]; rowCount: number; coverage: string }) => void) { this.clear(false); }
  canEdit(resource: string, fieldId: string): boolean { return this.grants.some(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write); }
  async openGranted(resource: string, fieldId: string): Promise<void> { const grant = this.grants.find(grant => grant.resource === resource && grant.fieldId === fieldId && grant.write); if (!grant) throw new Error("此字段没有独立写入权限"); await this.run(() => this.open(grant)); this.host.scrollIntoView({ block: "start" }); this.editor.querySelector<HTMLElement>("input,select,textarea")?.focus({ preventScroll: true }); }
  clear(removeCache = true) {
    this.epoch++; this.profile = undefined; this.selected = undefined; this.draft = undefined;
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
    this.retain();
    const epoch = this.epoch;
    const stored = sessionStorage.getItem(this.key(grant));
    this.selected = grant; this.profile = undefined; this.draft = undefined; this.unreadableDraft = undefined;
    try { this.draft = stored ? JSON.parse(stored) as Draft : undefined; }
    catch { this.unreadableDraft = stored!; throw new Error("字段恢复记录无法解析，原记录仍保留；请先复制下方原文。"); }
    this.compare = !!stored;
    this.message.textContent = this.draft?.pending ? "有一次结果待核对的原操作；可先核对原操作，勿再次提交。" : "正在读取当前字段；原输入仍保留。";
    this.render();
    const profile = await this.request<Profile>("fields/read", { resource: grant.resource, fieldId: grant.fieldId });
    if (epoch !== this.epoch) return;
    this.profile = profile;
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
    this.editor.append(ui("h3", "", `${this.selected.resource} · ${profile?.field.key ?? this.selected.fieldId}`));
    this.editor.append(ui("p", "", profile ? `当前字段：${this.cellText(profile.field.cell)}` : "当前字段未读取成功；保留的输入与原操作仍可核对。"));
    const form = ui("fieldset"); form.disabled = this.busy || !profile?.field.writable || !!draft.pending;
    const shape = profile?.field.shape ?? draft.shape;
    if (shape) form.append(valueEditor(shape, draft.value, schema, value => {
      draft.value = value;
      try { this.retain(); this.message.textContent = "字段输入已保留，尚未保存。"; } catch { this.message.textContent = "无法保留恢复信息，请先复制输入。"; }
    }, "字段输入"));
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
    }); save.disabled = this.busy || !profile?.basis || this.compare || !!draft.pending || !!unmapped.length;
    const reread = action("重新读取字段", () => this.run(() => this.open(this.selected!))); reread.disabled = this.busy || !!draft.pending;
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
  }
  private applyReceipt(receipt: Receipt) {
    if (!this.draft) return;
    if (["committed", "cancelled", "resolved"].includes(receipt.status)) {
      delete this.draft.pending; this.compare = true; if (this.profile) this.profile.basis = null;
      if (receipt.status === "committed" && receipt.cell?.value) this.draft.value = receipt.cell.value;
      if (receipt.status === "committed" && receipt.cell?.state === "null") this.draft.value = { kind: "none" };
      this.retain();
      this.message.textContent = receipt.status === "committed" ? "原字段操作已保存。继续编辑前，请重新读取并核对当前字段。" : receipt.status === "resolved" ? "原操作已明确结束，未确认提交；输入保留。请重新读取并核对。" : "原操作已取消，输入保留。请重新读取并核对。";
    } else this.message.textContent = "原操作尚未确认，请保留输入并继续核对。";
  }
}
