import { action, emptyNativeValue, fieldsForTypes, formatNativeValue, typeConflicts, ui, valueEditor } from "./data-panel";
import { hostedCachePrefix } from "./hosted-state";
import { canCommitStructure, clearStructureCapture, decodeStructureCatalog, decodeStructureDraft, freezeStructureRequest, initialStructureDraft, structureEnded, structureInput, structureReceipt, structureSelectors, type StructureCatalog, type StructureDraft, type StructureKind, type StructureReceipt, type StructureRequest } from "./hosted-structure-state";
type Request = <T>(path: string, body?: unknown) => Promise<T>;
export interface HostedStructureHost { protect(request: StructureRequest, phase: "prepare" | "commit"): Promise<void>; changed(receipt: StructureReceipt, request: StructureRequest): void | Promise<void>; open(result: NonNullable<StructureReceipt["result"]>, request: StructureRequest): Promise<void> }
const empty = (): StructureCatalog => ({ sources: [], slots: [], templates: [], scope: "configuredAuthorizedStructure", coverage: "complete" });
const labels: [StructureKind, string][] = [["create", "创建笔记"], ["rename", "改名"], ["move", "移动"], ["trash", "送到回收站"], ["restore", "恢复原回收对象"], ["copy", "复制独立节点"], ["template", "按原生模板创建"]];
export class HostedStructure {
  private epoch = 0;
  private catalogVersion = 0;
  private key = "";
  private draft: StructureDraft = initialStructureDraft();
  private catalog = empty();
  private busy = false;
  private unreadable?: string;
  private kind = ui("select");
  private source = ui("select");
  private destination = ui("select");
  private template = ui("select");
  private trashId = ui("input");
  private title = ui("input");
  private body = ui("textarea");
  private bodyMode = ui("select");
  private parameters = ui("textarea");
  private inputs = ui("select");
  private form = ui("section");
  private capture = ui("section");
  private captureKey = "";
  private fieldsForm?: HTMLFieldSetElement;
  private preview = ui("section");
  private message = ui("p", "hint");
  private coverage = ui("p", "hint");
  private prepareButton = action("准备原结构操作", () => this.run(() => this.prepare()));
  constructor(private host: HTMLElement, private request: Request, private bridge: HostedStructureHost) { this.clear(false); }
  get hasPending() { return !!this.draft.pending && !structureEnded(this.draft.pending.receipt?.status); }
  clear(removeCache = true) {
    this.epoch++; this.catalogVersion++; this.key = ""; this.draft = initialStructureDraft(); this.catalog = empty(); this.busy = false; this.unreadable = undefined;
    this.host.replaceChildren(); this.host.hidden = true; this.form.replaceChildren(); this.capture.replaceChildren(); this.captureKey = ""; this.fieldsForm = undefined; this.preview.replaceChildren(); this.message.textContent = ""; this.coverage.textContent = "";
    for (const control of [this.kind, this.source, this.destination, this.template, this.trashId, this.title, this.body, this.bodyMode, this.parameters, this.inputs]) { control.value = ""; control.disabled = false; } this.inputs.replaceChildren();
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-structure:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); this.key = hostedCachePrefix("structure", deployment, actor, generation) + "state";
    let raw: string | null = null; try { raw = sessionStorage.getItem(this.key); if (raw) this.draft = decodeStructureDraft(raw); } catch { this.unreadable = raw ?? ""; }
    this.host.hidden = false; this.coverage.setAttribute("aria-label", "托管结构目录完整性");
    const field = (label: string, control: HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement) => { control.setAttribute("aria-label", label); const wrapper = ui("label"); wrapper.append(ui("span", "", label), control); return wrapper; };
    this.kind.replaceChildren(...labels.map(([kind, label]) => { const option = ui("option", "", label); option.value = kind; return option; }));
    this.bodyMode.replaceChildren(...[["text", "原样文字"], ["markup", "原生 Typst markup"]].map(([value, text]) => { const option = ui("option", "", text); option.value = value; return option; }));
    this.body.rows = 5; this.parameters.rows = 4; this.body.spellcheck = this.parameters.spellcheck = false;
    this.inputs.multiple = true;
    this.form.append(field("结构操作", this.kind), field("结构源对象", this.source), field("批准的固定目标槽位", this.destination), field("结构原生模板", this.template), field("原回收操作编号", this.trashId), field("新笔记标题", this.title), field("新笔记正文", this.body), field("新笔记正文解释", this.bodyMode), this.capture, field("原生模板参数（JSON）", this.parameters), field("明确选取的获权引用输入", this.inputs), this.prepareButton);
    this.kind.onchange = () => { this.draft.kind = this.kind.value as StructureKind; this.inputChanged(); };
    this.source.onchange = () => { this.draft.source = this.source.value; this.inputChanged(); };
    this.destination.onchange = () => { this.draft.destination = this.destination.value; this.inputChanged(); };
    this.template.onchange = () => { this.draft.template = this.template.value; this.inputChanged(); };
    this.trashId.oninput = () => { this.draft.trashOperationId = this.trashId.value; this.inputChanged(); };
    this.title.oninput = () => { this.draft.title = this.title.value; this.inputChanged(); };
    this.body.oninput = () => { this.draft.body = this.body.value; this.inputChanged(); };
    this.bodyMode.onchange = () => { this.draft.bodyMode = this.bodyMode.value as "text" | "markup"; this.inputChanged(); };
    this.parameters.oninput = () => { this.draft.parametersText = this.parameters.value; this.inputChanged(); };
    this.inputs.onchange = () => { this.draft.inputs = [...this.inputs.selectedOptions].map(option => option.value); this.inputChanged(); };
    this.host.replaceChildren(ui("h2", "", "获权结构操作"), ui("p", "hint", "节点动作、目标接收与正文读取分别授权。名称和父位置使用批准的固定槽位；原个人正文不自动保存、复制或替换。"), action("刷新获权结构目录", () => this.refresh().catch(error => this.show(error))), this.coverage, this.form, this.message, this.preview);
    this.render(); await this.refresh();
  }
  async refresh() {
    const epoch = this.epoch, version = ++this.catalogVersion, value = decodeStructureCatalog(await this.request<StructureCatalog>("structure")); if (epoch !== this.epoch || version !== this.catalogVersion) return;
    this.catalog = value; this.options(this.source, value.sources.map(item => [item.resource, `${item.title}${item.status === "unavailable" ? "（当前不可用，恢复需原回执）" : ""}`]), this.draft.source);
    this.options(this.destination, value.slots.map(item => [item.resource, `${item.title} · ${item.parentTitle ?? item.parent} / ${item.name}${item.status === "unavailable" ? "（当前不可用）" : ""}`]), this.draft.destination);
    this.options(this.template, value.templates.map(item => [item.id, `${item.title}${item.status === "unavailable" ? "（当前不可用）" : ""}`]), this.draft.template);
    this.coverage.textContent = value.coverage === "partial" ? "结构目录部分可用；不可用获权位置保留，未当作完整空目录。" : "结构目录完整；仅列此用户明确获权的源与固定目标。"; this.render();
  }
  private options(select: HTMLSelectElement, values: string[][], current: string) { const first = ui("option", "", "请选择获权项…"); first.value = ""; select.replaceChildren(first, ...values.map(([value, label]) => { const option = ui("option", "", label); option.value = value; return option; })); if (current && !values.some(([value]) => value === current)) { const old = ui("option", "", `${current}（原选择当前不可用）`); old.value = current; select.append(old); } select.value = current; }
  setDocuments(documents: { resource: string; title: string }[]) { this.inputs.replaceChildren(...documents.map(item => { const option = ui("option", "", item.title); option.value = item.resource; option.selected = this.draft.inputs.includes(item.resource); return option; })); for (const id of this.draft.inputs.filter(id => !documents.some(item => item.resource === id))) { const old = ui("option", "", `${id}（原输入当前不可用）`); old.value = id; old.selected = true; this.inputs.append(old); } }
  private persist(value = this.draft) { if (!this.key || this.unreadable !== undefined) throw new Error("原结构恢复记录不可写，请先核对原记录。"); try { sessionStorage.setItem(this.key, JSON.stringify(value)); } catch { throw new Error("浏览器恢复缓存容量不足或不可写；结构输入仍保留，未发起新的结构写入。"); } }
  private keep() { try { this.persist(); } catch (error) { this.show(error); } }
  private show(error: unknown) { this.message.textContent = error instanceof Error ? error.message : "结构结果尚未确认，请保留原编号。"; }
  private inputChanged() { this.captureRawInputs(); this.draft.revision++; this.keep(); this.render(); }
  private fieldControls() { const seen = new Map<string, number>(); return [...(this.fieldsForm?.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input,select,textarea") ?? [])].map(control => { const shape = control.closest<HTMLElement>("[data-field-shape]")?.dataset.fieldShape ?? "", name = `${shape}:${control.tagName}:${control.type}:${control.getAttribute("aria-label") ?? control.labels?.[0]?.textContent ?? ""}`, ordinal = seen.get(name) ?? 0; seen.set(name, ordinal + 1); return { key: `${name}:${ordinal}`, control }; }); }
  private captureRawInputs() { for (const { key, control } of this.fieldControls()) { if (control.dataset.unmatchedRaw) continue; if (!control.validity.valid) this.draft.rawInputs[key] = control.value; else delete this.draft.rawInputs[key]; } }
  private renderCapture() {
    const d = this.draft, slot = this.catalog.slots.find(item => item.resource === d.destination), key = JSON.stringify([d.kind, d.destination, slot?.capture, slot?.captureStatus, d.types, Object.keys(d.fields)]);
    this.capture.hidden = d.kind !== "create"; if (this.captureKey === key) return;
    this.captureRawInputs(); this.captureKey = key; this.fieldsForm = undefined; this.capture.replaceChildren(); if (d.kind !== "create") return;
    const schema = slot?.capture;
    this.capture.append(ui("h3", "", "新笔记类型与原生属性"));
    if (d.types.length || Object.keys(d.fields).length || Object.keys(d.rawInputs).length) this.capture.append(action("明确清空本次类型与属性", () => { this.fieldsForm = undefined; this.captureKey = ""; this.draft = clearStructureCapture(this.draft); this.keep(); this.render(); }));
    if (!schema || slot.captureStatus === "unavailable") { this.capture.append(ui("p", "hint", "当前固定槽位的属性表单不可用；类型和值输入保留，未读取本地 Schema。")); for (const [id, value] of Object.entries(d.fields)) this.capture.append(ui("p", "", `${id}：${formatNativeValue(value)}`)); }
    else {
      const types = ui("select"); types.multiple = true; types.setAttribute("aria-label", "新笔记获权类型");
      for (const id of slot.types ?? []) { const option = ui("option", "", id); option.value = id; option.selected = d.types.includes(id); types.append(option); }
      for (const id of d.types.filter(id => !(slot.types ?? []).includes(id))) { const option = ui("option", "", `${id}（原类型当前不可用）`); option.value = id; option.selected = true; types.append(option); }
      types.onchange = () => { d.types = [...types.selectedOptions].map(option => option.value); this.inputChanged(); }; this.capture.append(types);
      for (const conflict of typeConflicts(schema, d.types)) this.capture.append(ui("p", "hint", conflict));
      const form = ui("fieldset"); this.fieldsForm = form; this.capture.append(form);
      for (const { field, required } of fieldsForTypes(schema, d.types)) { const row = ui("section"); row.dataset.fieldShape = JSON.stringify(field.shape); row.append(ui("p", "", `${field.id}${required ? "（必填）" : ""}`)); const value = d.fields[field.id];
        if (value) { row.append(valueEditor(field.shape, value, schema, next => { d.fields[field.id] = next; this.inputChanged(); }, `新笔记属性 ${field.id}`)); if (field.nullable) row.append(action(`将 ${field.id} 设为 none`, () => { d.fields[field.id] = { kind: "none" }; this.captureKey = ""; this.inputChanged(); })); row.append(action(`移除新笔记属性 ${field.id}`, () => { delete d.fields[field.id]; this.inputChanged(); })); }
        else row.append(action(`填写新笔记属性 ${field.id}`, () => { d.fields[field.id] = emptyNativeValue(field.shape, schema); this.inputChanged(); })); form.append(row);
      }
      for (const [id, value] of Object.entries(d.fields).filter(([id]) => !fieldsForTypes(schema, d.types).some(item => item.field.id === id))) this.capture.append(ui("p", "hint", `原属性 ${id} 未匹配当前所选类型，原值保留：${formatNativeValue(value)}`), action(`明确放弃原属性 ${id}`, () => { delete d.fields[id]; this.inputChanged(); }));
      for (const { key, control } of this.fieldControls()) if (d.rawInputs[key] !== undefined) { const raw = d.rawInputs[key]; control.value = raw; if (control.value !== raw) { control.dataset.unmatchedRaw = "true"; control.setCustomValidity("原输入不能由当前控件表示，请明确重新输入或放弃原文。"); } else control.dispatchEvent(new Event("input", { bubbles: false })); }
      const edited = (event: Event) => { if (event.target instanceof HTMLInputElement || event.target instanceof HTMLSelectElement || event.target instanceof HTMLTextAreaElement) { if (event.target.dataset.unmatchedRaw) { delete event.target.dataset.unmatchedRaw; event.target.setCustomValidity(""); } } this.captureRawInputs(); this.keep(); }; form.addEventListener("input", edited); form.addEventListener("change", edited);
      form.addEventListener("click", event => { if (!(event.target instanceof Element) || !event.target.closest("button")) return; const invalid = this.fieldControls().find(item => !item.control.validity.valid); if (invalid) { event.preventDefault(); event.stopPropagation(); invalid.control.reportValidity(); this.message.textContent = "属性输入尚未完成，原文保留，请完成输入后调整结构。"; } }, { capture: true });
    }
    const matched = new Set(this.fieldControls().filter(item => !item.control.dataset.unmatchedRaw).map(item => item.key)), unmapped = Object.entries(d.rawInputs).filter(([key]) => !matched.has(key));
    if (unmapped.length) this.capture.append(ui("p", "hint", "原无效输入没有匹配当前表单，原文保留；请重新核对或明确放弃后再准备。"), ...unmapped.map(([, text]) => ui("pre", "", text)), action("明确放弃未匹配的属性原文", () => { for (const [key] of unmapped) delete d.rawInputs[key]; this.inputChanged(); }));
  }
  private async run(work: () => Promise<void>) { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.render(); try { await work(); } catch (error) { if (epoch === this.epoch) this.show(error); } finally { if (epoch === this.epoch) { this.busy = false; this.render(); } } }
  private async prepare() {
    this.captureRawInputs(); if (this.fieldControls().some(({ control }) => !control.reportValidity())) { this.keep(); throw new Error("属性输入无效，原文已保留，未发起结构准备。"); }
    const slot = this.catalog.slots.find(item => item.resource === this.draft.destination); if (this.draft.kind === "create" && slot?.capture) { if (typeConflicts(slot.capture, this.draft.types).length || fieldsForTypes(slot.capture, this.draft.types).some(item => item.required && !this.draft.fields[item.field.id])) throw new Error("请核对类型冲突或填写必需属性，原输入保留。"); }
    const epoch = this.epoch, request = freezeStructureRequest(this.draft, this.catalog, crypto.randomUUID()), next = { ...this.draft, pending: { request, input: structureInput(this.draft), revision: this.draft.revision, selector: structureSelectors(this.draft, this.catalog), acceptLosses: false } };
    this.persist(next); this.draft = next; this.render(); await this.bridge.protect(request, "prepare"); if (epoch !== this.epoch) return;
    const value = await this.request<StructureReceipt>("structure/prepare", request); if (epoch === this.epoch) await this.receive(value);
  }
  private async receive(value: StructureReceipt) { const pending = this.draft.pending; if (!pending) return; pending.receipt = structureReceipt(pending.request, value, pending.receipt); this.keep(); this.message.textContent = value.status === "committed" ? "原结构操作已提交；当前个人草稿与后续输入保留，新对象仍需独立获权后明确打开。" : value.status === "prepared" ? "原计划已准备；请核对固定名称、位置、变化和损失后明确提交。" : structureEnded(value.status) ? "原结构操作已结束，未冒作已创建或保存。" : "原操作仍待核对，不会换编号重放当前输入。";
    if (value.status === "committed") { const epoch = this.epoch; await this.bridge.changed(value, pending.request); if (epoch === this.epoch) await this.refresh(); }
  }
  private render() {
    this.renderCapture();
    const d = this.draft; for (const [control, value] of [[this.kind, d.kind], [this.source, d.source], [this.destination, d.destination], [this.template, d.template], [this.trashId, d.trashOperationId], [this.title, d.title], [this.body, d.body], [this.bodyMode, d.bodyMode], [this.parameters, d.parametersText]] as const) if (control.value !== value) control.value = value;
    for (const control of [this.source, this.destination, this.template, this.trashId, this.title, this.body, this.bodyMode, this.parameters]) control.parentElement!.hidden = control === this.source ? d.kind === "create" || d.kind === "template" && d.template === "native-note" : control === this.destination ? d.kind === "trash" : control === this.template || control === this.parameters ? d.kind !== "template" : control === this.trashId ? d.kind !== "restore" : d.kind !== "create";
    this.prepareButton.disabled = this.busy || !!d.pending || this.unreadable !== undefined; this.preview.replaceChildren();
    if (this.unreadable !== undefined) { this.form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input,select,textarea").forEach(control => control.disabled = true); this.preview.append(ui("p", "hint", "结构恢复记录无法解析，原文保留，未发起新的操作。"), ui("pre", "", this.unreadable), action("明确放弃损坏的结构恢复记录", () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.draft = initialStructureDraft(); this.form.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>("input,select,textarea").forEach(control => control.disabled = false); this.render(); })); return; }
    const pending = d.pending; if (!pending) return; const receipt = pending.receipt, card = ui("section"); card.append(ui("h3", "", "原结构操作"), ui("p", "", `${pending.request.kind} · ${pending.request.resource} · ${pending.request.operationId}`), ui("p", "", `状态：${receipt?.status ?? "unknown"}`));
    const summary = receipt?.preview?.summary; if (summary) card.append(ui("pre", "", Array.isArray(summary) ? summary.join("\n") : summary));
    for (const loss of receipt?.preview?.losses ?? []) card.append(ui("p", "hint", `${loss.code ?? "影响"}：${loss.message}`));
    const accepted = ui("input"); accepted.type = "checkbox"; accepted.checked = pending.commit?.acceptLosses ?? pending.acceptLosses; accepted.disabled = !!pending.commit; accepted.setAttribute("aria-label", "接受原结构计划列出的损失"); accepted.onchange = () => { pending.acceptLosses = accepted.checked; this.keep(); }; const lossChoice = ui("label"); lossChoice.append(accepted, ui("span", "", "接受原结构计划列出的损失")); card.append(lossChoice);
    if (pending.revision !== d.revision || pending.input !== structureInput(d)) card.append(ui("p", "hint", "后来输入或同值往返已改变意图；旧计划不能提交当前新输入，请核对或结束原操作。"));
    const recover = action("查询原结构操作", () => this.run(async () => { const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/operation", { resource: pending.request.resource, operationId: pending.request.operationId }); if (epoch === this.epoch) await this.receive(value); })); recover.disabled = this.busy;
    const retry = action("按原请求重试结构准备", () => this.run(async () => { this.persist(); const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/prepare", pending.request); if (epoch === this.epoch) await this.receive(value); })); retry.disabled = this.busy || structureEnded(receipt?.status);
    const commit = action("确认提交原结构计划", () => this.run(async () => { if (!canCommitStructure(this.draft, this.catalog)) throw new Error("原输入或固定槽位资格已变化，未提交旧计划。"); if (receipt?.preview?.losses?.length && !pending.acceptLosses) throw new Error("请核对并明确接受原计划列出的损失。"); const epoch = this.epoch; pending.commit ??= { resource: pending.request.resource, operationId: pending.request.operationId, planId: pending.receipt!.planId!, acceptLosses: pending.acceptLosses }; this.persist(); await this.bridge.protect(pending.request, "commit"); if (epoch !== this.epoch) return; if (!canCommitStructure(this.draft, this.catalog)) throw new Error("保护草稿期间输入或固定槽位已变化，未发送旧计划。"); const value = await this.request<StructureReceipt>("structure/commit", pending.commit); if (epoch === this.epoch) await this.receive(value); })); commit.disabled = this.busy || !canCommitStructure(d, this.catalog);
    const settle = action("结束未安装的原结构操作", () => this.run(async () => { const epoch = this.epoch, value = await this.request<StructureReceipt>("structure/settle", { resource: pending.request.resource, operationId: pending.request.operationId }); if (epoch === this.epoch) await this.receive(value); })); settle.disabled = this.busy || structureEnded(receipt?.status); card.append(commit, recover, retry, settle);
    if (receipt?.status === "committed") { card.append(ui("pre", "", (receipt.nodeRefs ?? []).map(ref => `weftext-node://${ref.workspaceId}/${ref.nodeId}`).join("\n"))); if (receipt.result?.canOpen) { const result = receipt.result; card.append(action("明确打开获权结构结果", () => this.run(() => this.bridge.open(result, pending.request)))); } else card.append(ui("p", "hint", "已提交事实保留，当前没有独立获权的正文打开入口。")); }
    if (structureEnded(receipt?.status)) card.append(action("清除本页已结束结构回执，保留当前输入", () => { const next = { ...d, pending: undefined }; this.persist(next); this.draft = next; this.render(); })); this.preview.append(card);
  }
}
