import { decodeProofreadingOptions, type ProofreadRule } from "./proofreading-panel";
import { defaultProcessingSettings, parseProcessingLanguages, processingLanguageTag, type ProcessingRuleScope, type ProcessingSettings } from "./processing-settings";

export interface ProcessingDocument { schemaVersion: 1; defaults: ProcessingSettings; rules: ProofreadRule[]; scopedAllowlist: { text: string; scope: ProcessingRuleScope }[] }
export interface ProcessingRead { baseId: string; beforeSource: string | null; processing: ProcessingDocument | null; warning?: string | null; source: string }
export interface ProcessingRequest { operationId: string; baseId: string; beforeSource: string | null; processing: ProcessingDocument | null }
export interface ProcessingOperation { operationId: string; planId: string | null; status: string; changes?: unknown[]; losses?: unknown[]; message?: string }
export interface ProcessingSettingsHost {
  persistenceKey: string;
  read(): Promise<ProcessingRead>;
  prepare(request: ProcessingRequest): Promise<ProcessingOperation>;
  status(request: ProcessingRequest): Promise<ProcessingOperation>;
  settle(request: ProcessingRequest): Promise<ProcessingOperation>;
  apply(request: { request: ProcessingRequest; planId: string }): Promise<ProcessingOperation>;
  onApplied?(): void | Promise<void>;
}
export const defaultProcessingDocument = (): ProcessingDocument => ({ schemaVersion: 1, defaults: defaultProcessingSettings(), rules: [], scopedAllowlist: [] });
export function decodeProcessingDocument(value: unknown): ProcessingDocument {
  const v = value as ProcessingDocument;
  if (!v || v.schemaVersion !== 1 || Object.keys(v).some(k => !["schemaVersion", "defaults", "rules", "scopedAllowlist"].includes(k))) throw new Error("不支持此处理设置版本或未知成员，原输入已保留。");
  const validated = decodeProofreadingOptions({ characters: true, typos: true, pairs: true, sensitive: true, rules: v.rules ?? [], allowlist: [], scopedAllowlist: v.scopedAllowlist ?? [], processing: v.defaults });
  return { schemaVersion: 1, defaults: validated.processing!, rules: validated.rules, scopedAllowlist: validated.scopedAllowlist! };
}
interface Retained { state: ProcessingRead; source: string; pending?: { request: ProcessingRequest; operation?: ProcessingOperation } }
/** Saves the frozen request before network IO. Reopening queries its original ID. */
export class ProcessingSettingsController {
  state: ProcessingRead | null = null;
  source = "";
  pending: Retained["pending"];
  busy = false;
  recoveryError: string | null = null;
  constructor(readonly host: ProcessingSettingsHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
    const raw = storage.getItem(host.persistenceKey); if (!raw) return;
    try {
      const saved = JSON.parse(raw) as Retained;
      if (!saved.state?.baseId || typeof saved.source !== "string" || !(saved.state.beforeSource === null || typeof saved.state.beforeSource === "string") || saved.pending && (!saved.pending.request?.operationId || saved.pending.request.baseId !== saved.state.baseId || saved.pending.request.beforeSource !== saved.state.beforeSource)) throw new Error("恢复记录格式不符");
      if (saved.pending?.request.processing) decodeProcessingDocument(saved.pending.request.processing);
      this.state = saved.state; this.source = saved.source; this.pending = saved.pending;
    } catch { this.recoveryError = raw; }
  }
  get locked(): boolean { return this.busy || !!this.pending || this.recoveryError !== null; }
  get dirty(): boolean { return !!this.state && this.source !== JSON.stringify(this.state.processing ?? defaultProcessingDocument(), null, 2); }
  async load(): Promise<void> {
    if (this.locked || this.dirty) throw new Error("原操作或设置草稿尚未处理，请保留输入并先确认原操作。");
    this.install(await this.host.read());
  }
  edit(source: string): void { if (this.locked) throw new Error("原操作待确认，当前输入已保留。"); this.source = source; this.retain(); }
  resetDraft(): void { if (this.locked || !this.state) throw new Error("请先确认原操作。"); this.source = JSON.stringify(this.state.processing ?? defaultProcessingDocument(), null, 2); this.retain(); }
  async prepare(reset = false): Promise<void> {
    if (!this.state || this.locked) throw new Error("请先读取或确认原处理设置操作。");
    const processing = reset ? null : decodeProcessingDocument(JSON.parse(this.source));
    this.pending = { request: { operationId: crypto.randomUUID(), baseId: this.state.baseId, beforeSource: this.state.beforeSource, processing } };
    try { this.retain(); } catch (error) { this.pending = undefined; throw error; }
    this.busy = true;
    try { this.accept(await this.host.prepare(structuredClone(this.pending.request))); } finally { this.busy = false; }
  }
  async status(): Promise<void> {
    if (!this.pending || this.busy) throw new Error("没有待确认的原操作。");
    this.busy = true;
    try {
      this.accept(await this.host.status(structuredClone(this.pending.request)));
      await this.finishTerminal();
    } finally { this.busy = false; }
  }
  async retryPrepare(): Promise<void> {
    if (!this.pending || this.busy) throw new Error("没有待重试的原准备操作。");
    this.busy = true;
    try { this.accept(await this.host.prepare(structuredClone(this.pending.request))); await this.finishTerminal(); } finally { this.busy = false; }
  }
  async settle(): Promise<void> {
    if (!this.pending || this.busy) throw new Error("没有待结束的原操作。");
    this.busy = true;
    try { this.accept(await this.host.settle(structuredClone(this.pending.request))); await this.finishTerminal(); } finally { this.busy = false; }
  }
  async apply(): Promise<void> {
    if (!this.pending?.operation?.planId || this.pending.operation.status !== "prepared" || this.busy) throw new Error("请先查看原操作预览。");
    this.busy = true;
    try {
      this.accept(await this.host.apply({ request: structuredClone(this.pending.request), planId: this.pending.operation.planId }));
      if ((this.pending?.operation?.status as string | undefined) === "committed") {
        const fresh = await this.host.read(); this.install(fresh); await this.host.onApplied?.();
      }
    } finally { this.busy = false; }
  }
  private accept(operation: ProcessingOperation): void {
    if (!this.pending || operation.operationId !== this.pending.request.operationId || !["prepared", "committed", "recoveryPending", "notRegistered", "notCommitted", "cancelled", "resolved"].includes(operation.status) || ["prepared", "committed", "recoveryPending"].includes(operation.status) && !operation.planId || this.pending.operation?.planId && operation.planId !== this.pending.operation.planId || this.pending.operation?.status === "committed" && operation.status !== "committed") throw new Error("回执与原处理设置操作不符，请继续查询原编号。");
    this.pending.operation = structuredClone(operation); this.retain();
  }
  private async finishTerminal(): Promise<void> {
    const status = this.pending?.operation?.status;
    if (status === "committed") { this.install(await this.host.read()); await this.host.onApplied?.(); }
    else if (status === "cancelled" || status === "resolved") { const draft = this.source; this.install(await this.host.read()); this.source = draft; this.retain(); }
  }
  private install(state: ProcessingRead): void { this.state = state; this.pending = undefined; this.source = JSON.stringify(state.processing ?? defaultProcessingDocument(), null, 2); this.retain(); }
  private retain(): void { this.storage.setItem(this.host.persistenceKey, JSON.stringify({ state: this.state, source: this.source, pending: this.pending })); }
}
const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); e.textContent = text; return e; };
export async function openProcessingSettingsPanel(host: ProcessingSettingsHost): Promise<void> {
  const controller = new ProcessingSettingsController(host);
  if (!controller.pending && !controller.dirty && !controller.recoveryError) await controller.load();
  const dialog = el("dialog"); dialog.className = "modal library-style-panel";
  const title = el("h2", "资料库共享处理默认与词表"); dialog.setAttribute("aria-label", title.textContent!);
  const source = el("textarea"); source.className = "text-input style-source"; source.rows = 14; source.setAttribute("aria-label", "共享处理设置 JSON");
  const languages = el("input"); languages.className = "text-input"; languages.setAttribute("aria-label", "共享处理语言默认建议");
  const message = el("p"); message.setAttribute("role", "status");
  const preview = el("pre"); const actions = el("div"); actions.className = "modal-actions";
  const buttons: HTMLButtonElement[] = [];
  const action = (text: string, run: () => Promise<void> | void) => { const b = el("button", text); b.type = "button"; b.className = "button-secondary"; b.onclick = () => { void (async () => { try { const promise = run(); draw(); await promise; message.textContent = controller.pending ? `原操作 ${controller.pending.request.operationId}：${controller.pending.operation?.status ?? "结果待确认"}` : "共享设置已读取；本次导入／校对仍需明确采用建议。"; } catch (error) { message.textContent = `${error instanceof Error ? error.message : String(error)}${controller.pending ? " 原请求已保留，请查询同一操作。" : ""}`; } draw(); })(); }; buttons.push(b); actions.append(b); return b; };
  const prepare = action("预览共享设置变更", () => controller.prepare());
  const query = action("查询原操作", () => controller.status());
  const retry = action("重试同一准备操作", () => controller.retryPrepare());
  const settle = action("取消／结束原预览并保留输入", () => controller.settle());
  const apply = action("应用已预览共享设置", () => controller.apply());
  const reset = action("放弃本次输入", () => controller.resetDraft());
  const clear = action("预览移除共享默认", () => controller.prepare(true));
  const close = action("关闭", () => { dialog.close(); });
  function draw(): void {
    source.value = controller.source; source.disabled = controller.locked;
    let parsed: ProcessingDocument | undefined; try { parsed = decodeProcessingDocument(JSON.parse(controller.source)); } catch { /* Preserve malformed source draft. */ }
    languages.value = parsed?.defaults.languages.map(processingLanguageTag).join(", ") ?? ""; languages.disabled = controller.locked || !parsed;
    prepare.disabled = controller.locked; query.disabled = controller.busy || !controller.pending; retry.disabled = controller.busy || !controller.pending; settle.disabled = controller.busy || !controller.pending; apply.disabled = controller.busy || controller.pending?.operation?.status !== "prepared"; reset.disabled = controller.locked; clear.disabled = controller.locked; close.disabled = controller.busy;
    preview.textContent = controller.pending ? JSON.stringify({ operationId: controller.pending.request.operationId, beforeProcessing: controller.state?.processing ?? null, afterProcessing: controller.pending.request.processing, result: controller.pending.operation }, null, 2) : "";
    if (controller.recoveryError) { preview.textContent = controller.recoveryError; message.textContent = "本机恢复记录无法解析，原文保留；未覆盖或发送设置。"; }
  }
  source.oninput = () => { try { controller.edit(source.value); } catch (e) { message.textContent = String(e); } };
  languages.onchange = () => { try { const value = decodeProcessingDocument(JSON.parse(source.value)); value.defaults.languages = parseProcessingLanguages(languages.value); controller.edit(JSON.stringify(value, null, 2)); draw(); } catch (e) { message.textContent = String(e); } };
  dialog.append(title, el("p", "这是库共享的处理建议与词表，经库设置权限、历史和同步保存。文档排版语言仍由原生 Typst 决定；个人选择不会自动写入这里。地区与文字体系可未知，空语言列表不从界面或系统推断。"), languages, source, message, preview, actions);
  document.body.append(dialog); draw(); dialog.showModal();
  await new Promise<void>(resolve => { dialog.addEventListener("close", () => { dialog.remove(); resolve(); }, { once: true }); dialog.addEventListener("cancel", e => { if (controller.busy) e.preventDefault(); }); });
}
