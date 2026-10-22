import { nativeLanguageOptions } from "./style-panel";
import type { Inventory, OpenedDocument } from "./types";

export interface InitialLibraryStyle { source: string; enabled: boolean }
export interface InitialRootNote { title: string; queries: string[] }
export interface LibraryCreateOptions { initialStyle?: InitialLibraryStyle | null; initialRoot?: InitialRootNote }
export interface LibraryCreationRequest { path: string; options: LibraryCreateOptions; operationId: string }
export interface LibraryCreationReceipt { operationId: string; root: string; workspaceId: string; status: string; rootNotePath?: string; library?: Inventory; rootDocument?: OpenedDocument; deliveryError?: { message: string } }
interface CreationSlot { key: string; value: string; editable: boolean; range: { utf16From: number; utf16To: number; raw: string } | null }
export interface LibraryCreationDefaults { source: string; sourceHash: string; controls: CreationSlot[]; rootPresets?: { id: string; title: string; plan: unknown }[] }
export interface LibraryCreationRecovery { operationId: string; status: string; message: string; allowedActions?: string[]; [key: string]: unknown }
export interface LibraryCreationHost {
  defaults(): Promise<LibraryCreationDefaults>;
  create(request: LibraryCreationRequest): Promise<LibraryCreationReceipt>;
  delivery(operationId: string): Promise<LibraryCreationReceipt>;
  deliver(receipt: LibraryCreationReceipt): Promise<void>;
  inspect(operationId: string): Promise<LibraryCreationRecovery>;
  resolve(operationId: string, action: string): Promise<LibraryCreationRecovery>;
  cancel?(request: LibraryCreationRequest): Promise<LibraryCreationRecovery>;
}
interface CreationDraft { path: string; language: string; region: string; defaults: LibraryCreationDefaults | null; rootMode?: "none" | "blank" | "queries"; rootTitle?: string; queries?: string[] }
interface RetainedCreation { version: 1; draft: CreationDraft; request: LibraryCreationRequest | null; receipt: LibraryCreationReceipt | null }
const key = "weftext-library-creation-v1";
function defaultsValid(value: LibraryCreationDefaults): boolean {
  return !!value && typeof value.source === "string" && typeof value.sourceHash === "string" && Array.isArray(value.controls) && ["text.lang", "text.region"].every(key => {
    const matches = value.controls.filter(c => c.key === key), slot = matches[0];
    return matches.length === 1 && slot.editable && slot.range && Number.isSafeInteger(slot.range.utf16From) && Number.isSafeInteger(slot.range.utf16To) && slot.range.utf16From >= 0 && slot.range.utf16From < slot.range.utf16To && slot.range.utf16To <= value.source.length && value.source.slice(slot.range.utf16From, slot.range.utf16To) === slot.range.raw && slot.value === slot.range.raw;
  });
}
export function initialLibraryStyle(defaults: LibraryCreationDefaults, language: string, region: string): InitialLibraryStyle {
  if (!defaultsValid(defaults)) throw new Error("初始化样式没有唯一可证明的原生语言／地区参数，原设置已保留。");
  if (!/^"[A-Za-z]{2,8}"$/.test(language) || !(region === "none" || /^"(?:[A-Za-z]{2}|[0-9]{3})"$/.test(region))) throw new Error("请选择有效的原生语言／地区代码，未知地区使用 none。");
  const changes = [["text.lang", language], ["text.region", region]].map(([key, value]) => ({ ...defaults.controls.find(c => c.key === key)!.range!, value })).sort((a, b) => b.utf16From - a.utf16From);
  if (changes[1].utf16To > changes[0].utf16From) throw new Error("初始化样式参数范围重叠，未改写原文。");
  let source = defaults.source;
  for (const change of changes) source = source.slice(0, change.utf16From) + change.value + source.slice(change.utf16To);
  return { source, enabled: true };
}
function validRequest(value: LibraryCreationRequest): boolean {
  if (value?.options?.initialRoot && (typeof value.options.initialRoot.title !== "string" || !Array.isArray(value.options.initialRoot.queries) || value.options.initialRoot.queries.some(id => typeof id !== "string"))) return false;
  return !!value && typeof value.path === "string" && !!value.path && /^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(value.operationId) && !!value.options && !!value.options.initialStyle && typeof value.options.initialStyle.source === "string" && typeof value.options.initialStyle.enabled === "boolean";
}
/** The exact native bytes and original ID are retained before the first write. */
export class LibraryCreationController {
  draft: CreationDraft = { path: "", language: '"zh"', region: '"CN"', defaults: null };
  request: LibraryCreationRequest | null = null;
  receipt: LibraryCreationReceipt | null = null;
  proposal: LibraryCreationRequest | null = null;
  recovery: LibraryCreationRecovery | null = null;
  recoveryError: string | null = null;
  busy = false;
  constructor(readonly host: LibraryCreationHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
    const raw = storage.getItem(key); if (!raw) return;
    try {
      const value = JSON.parse(raw) as RetainedCreation;
      if (value.version !== 1 || !value.draft || typeof value.draft.path !== "string" || typeof value.draft.language !== "string" || typeof value.draft.region !== "string" || value.draft.defaults !== null && !defaultsValid(value.draft.defaults) || value.request !== null && !validRequest(value.request) || value.receipt && (!value.request || value.receipt.operationId !== value.request.operationId || typeof value.receipt.root !== "string" || typeof value.receipt.workspaceId !== "string")) throw new Error("invalid creation record");
      this.draft = value.draft; this.request = value.request; this.receipt = value.receipt;
    } catch { this.recoveryError = raw; }
  }
  get locked(): boolean { return this.busy || this.request !== null || this.recoveryError !== null; }
  async load(): Promise<void> {
    if (this.request || this.recoveryError) return;
    if (this.draft.defaults) { if (!this.draft.defaults.rootPresets) this.draft.defaults.rootPresets = (await this.host.defaults()).rootPresets; this.retain(); return; }
    const defaults = await this.host.defaults(); if (!defaultsValid(defaults)) throw new Error("产品初始化样式不能安全解析；尚未创建任何文件。");
    this.draft.defaults = structuredClone(defaults); this.draft.language = defaults.controls.find(c => c.key === "text.lang")!.value; this.draft.region = defaults.controls.find(c => c.key === "text.region")!.value; this.retain();
  }
  change(update: Partial<Pick<CreationDraft, "path" | "language" | "region" | "rootMode" | "rootTitle" | "queries">>): void {
    if (this.locked) throw new Error("原创建结果仍待确认，路径、排版选择与操作编号已保留。");
    this.draft = { ...this.draft, ...update }; this.proposal = null; this.retain();
  }
  preview(): void {
    if (this.locked || !this.draft.defaults) throw new Error("请先读取初始化样式或确认原创建操作。");
    const path = this.draft.path.trim();
    if (!/^(?:[A-Za-z]:[\\/]|\\\\[^\\]+\\[^\\]+[\\/]|\/)/.test(path)) throw new Error("请输入尚不存在的新文件夹完整路径。");
    this.proposal = { path, options: { initialStyle: initialLibraryStyle(this.draft.defaults, this.draft.language, this.draft.region) }, operationId: crypto.randomUUID() };
    if (this.draft.rootMode && this.draft.rootMode !== "none") {
      const queries = this.draft.rootMode === "queries" ? this.draft.queries ?? [] : [];
      if (queries.some(id => !this.draft.defaults!.rootPresets?.some(p => p.id === id)) || new Set(queries).size !== queries.length) throw new Error("所选查询不在当前初始化清单中，未创建。");
      this.proposal.options.initialRoot = { title: this.draft.rootTitle ?? (this.draft.rootMode === "blank" ? "" : "工作台"), queries: [...queries] };
    }
  }
  async create(): Promise<LibraryCreationReceipt> {
    if (this.busy || this.recoveryError || !this.request && !this.proposal) throw new Error("请先预览创建内容，或确认原创建操作。");
    if (!this.request) {
      this.request = structuredClone(this.proposal!);
      try { this.retain(); } catch (error) { this.request = null; throw error; }
    }
    this.busy = true;
    try {
      const receipt = this.receipt ? await this.host.delivery(this.request.operationId) : await this.host.create(structuredClone(this.request));
      if (receipt.operationId !== this.request.operationId || receipt.status !== "committed" || !receipt.workspaceId || !receipt.root || this.receipt && (receipt.workspaceId !== this.receipt.workspaceId || receipt.root !== this.receipt.root)) throw new Error("创建回执与原操作不一致，请继续核对原编号。");
      this.receipt = structuredClone(receipt); this.retain(); return receipt;
    } finally { this.busy = false; }
  }
  async inspect(): Promise<void> {
    if (!this.request || this.busy) throw new Error("没有待核对的原创建操作。");
    this.busy = true;
    try { this.installRecovery(await this.host.inspect(this.request.operationId)); } finally { this.busy = false; }
  }
  async resolve(action: string): Promise<void> {
    if (!this.request || this.busy || !this.recovery?.allowedActions?.includes(action)) throw new Error("请先检查原操作当前允许的恢复动作。");
    this.busy = true;
    try { this.installRecovery(await this.host.resolve(this.request.operationId, action)); } finally { this.busy = false; }
  }
  async cancel(): Promise<void> {
    if (!this.request || this.busy || !this.host.cancel) throw new Error("当前不能结束原创建操作，请检查其恢复信息。");
    this.busy = true;
    try { this.installRecovery(await this.host.cancel(structuredClone(this.request))); } finally { this.busy = false; }
  }
  async deliver(): Promise<LibraryCreationReceipt> {
    const receipt = await this.create();
    this.busy = true;
    try {
      if (receipt.deliveryError) throw new Error(`资料库已提交，交付尚未完成：${receipt.deliveryError.message}`);
      await this.host.deliver(receipt);
      this.storage.removeItem(key);
      return receipt;
    } finally { this.busy = false; }
  }
  private installRecovery(result: LibraryCreationRecovery): void {
    if (!this.request || result.operationId !== this.request.operationId) throw new Error("恢复结果与原创建操作不一致。");
    if (this.receipt && result.status !== "committed") throw new Error("原资料库已确认提交，后续状态不一致；原回执与请求保留。");
    this.recovery = structuredClone(result);
    if (result.status === "resolved" || result.status === "cancelled") { this.request = null; this.proposal = null; this.receipt = null; }
    this.retain();
  }
  private retain(): void {
    const old = this.storage.getItem(key);
    if (old) { const saved = JSON.parse(old) as RetainedCreation; const ended = this.recovery && ["resolved", "cancelled"].includes(this.recovery.status) && this.recovery.operationId === saved.request?.operationId; if (saved.request && saved.request.operationId !== this.request?.operationId && !ended) throw new Error(`另一窗口仍保留原创建操作 ${saved.request.operationId}，请先核对该操作。`); }
    this.storage.setItem(key, JSON.stringify({ version: 1, draft: this.draft, request: this.request, receipt: this.receipt }));
  }
}

const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); e.textContent = text; return e; };
let activeCreation: Promise<LibraryCreationReceipt | null> | null = null;
export function openLibraryCreation(host: LibraryCreationHost): Promise<LibraryCreationReceipt | null> {
  if (!activeCreation) activeCreation = showLibraryCreation(host).finally(() => { activeCreation = null; });
  return activeCreation;
}
async function showLibraryCreation(host: LibraryCreationHost): Promise<LibraryCreationReceipt | null> {
  const controller = new LibraryCreationController(host); await controller.load();
  const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const dialog = el("dialog"); dialog.className = "modal library-creation-dialog"; dialog.setAttribute("aria-label", "创建资料库与文档排版语言");
  const path = el("input"); path.className = "text-input"; path.setAttribute("aria-label", "新资料库完整路径"); path.placeholder = "D:\\Documents\\我的资料库";
  const language = el("select"), region = el("select"); language.className = region.className = "text-input"; language.setAttribute("aria-label", "初始文档排版语言"); region.setAttribute("aria-label", "初始文档排版地区");
  const message = el("p"); message.setAttribute("role", "status"); const preview = el("div"), actions = el("div"); actions.className = "modal-actions"; actions.style.flexWrap = "wrap";
  const rootMode = el("select"); rootMode.setAttribute("aria-label", "根节点初始内容");
  for (const [value, label] of [["none", "保持原方式，不创建根笔记"], ["blank", "空白普通根笔记"], ["queries", "普通根笔记与所选查询"]]) { const option = el("option", label); option.value = value; rootMode.append(option); }
  const rootTitle = el("input"); rootTitle.setAttribute("aria-label", "根笔记标题（可留空）");
  const queryChoices = el("fieldset"); queryChoices.append(el("legend", "选择初始查询"));
  for (const preset of controller.draft.defaults?.rootPresets ?? []) { const label = el("label", preset.title), input = el("input"); input.type = "checkbox"; input.value = preset.id; input.setAttribute("aria-label", `初始查询：${preset.title}`); input.onchange = () => { controller.change({ queries: [...queryChoices.querySelectorAll<HTMLInputElement>("input:checked")].map(input => input.value) }); draw(); }; label.prepend(input); queryChoices.append(label); }
  let composing = false, finish: (value: LibraryCreationReceipt | null) => void = () => {};
  const button = (label: string, run: () => Promise<void> | void) => { const b = el("button", label); b.type = "button"; b.className = "button-secondary"; b.onclick = () => { void (async () => { try { if (composing) return; const promise = run(); draw(); await promise; } catch (error) { message.textContent = `${error instanceof Error ? error.message : String(error)}${controller.request ? ` 原操作 ${controller.request.operationId} 与全部选择已保留。` : ""}`; } draw(); })(); }; actions.append(b); return b; };
  const prepare = button("预览创建", () => { controller.preview(); message.textContent = "请核对路径和文档排版默认，再创建资料库。"; });
  const create = button("创建资料库", async () => { const receipt = await controller.deliver(); finish(receipt); });
  const inspect = button("查看原操作恢复信息", () => controller.inspect());
  const cancel = button("结束尚未开始的原创建", () => controller.cancel());
  const close = button("关闭", () => finish(null));
  function choice(input: HTMLSelectElement, key: string, current: string): void { input.replaceChildren(); for (const option of nativeLanguageOptions(key, current)) { const o = el("option", option.label); o.value = option.value; o.selected = option.value === current; input.append(o); } }
  function draw(): void {
    path.value = controller.draft.path; choice(language, "text.lang", controller.draft.language); choice(region, "text.region", controller.draft.region);
    for (const input of [path, language, region]) input.disabled = controller.locked;
    rootMode.value = controller.draft.rootMode ?? "none"; rootMode.disabled = controller.locked;
    rootTitle.value = controller.draft.rootTitle ?? (rootMode.value === "queries" ? "工作台" : ""); rootTitle.hidden = rootMode.value === "none"; rootTitle.disabled = controller.locked;
    queryChoices.hidden = rootMode.value !== "queries"; queryChoices.disabled = controller.locked;
    queryChoices.querySelectorAll<HTMLInputElement>("input").forEach(input => { input.checked = controller.draft.queries?.includes(input.value) ?? false; });
    prepare.disabled = controller.locked; create.disabled = controller.busy || !controller.request && !controller.proposal; create.textContent = controller.request ? "确认同一创建操作" : "创建资料库"; inspect.disabled = controller.busy || !controller.request; cancel.disabled = controller.busy || !controller.request || !host.cancel; close.disabled = controller.busy;
    preview.replaceChildren();
    const request = controller.request ?? controller.proposal;
    if (request?.options.initialRoot) {
      const root = request.options.initialRoot, name = request.path.replace(/[\\/]+$/, "").split(/[\\/]/).at(-1)!;
      preview.append(el("p", `新增普通根笔记：${request.path}/${name}.typ；标题：${root.title || "无标题"}`), el("p", "根目录仍是原库目录；此主文件使用普通 Node 身份、历史、同步和权限。不会生成示例任务或项目，也不会在重开／升级时补回内容。"));
      if (root.queries.length) { preview.append(el("p", "本次明确安装随附集中 schema（.weftext-schema.json），供所选类型、字段和各自独立的查询定义使用。全库在库范围，包含子目录，每项最多显示 200 行；不复制结果为正文。")); for (const id of root.queries) { const preset = controller.draft.defaults?.rootPresets?.find(p => p.id === id); preview.append(el("h3", preset?.title ?? id), el("pre", JSON.stringify(preset?.plan, null, 2))); } }
    }
    if (request) { const details = el("details"); details.append(el("summary", "高级：本次初始化的原生样式"), el("pre", request.options.initialStyle!.source)); preview.append(el("p", `目标：${request.path}`), el("p", `原操作：${request.operationId}`), el("p", "库默认样式及其资源身份与资料库一起创建。地区未知保持未指定；不会自动简繁转换，也不修改个人界面语言。"), details); }
    if (controller.recoveryError) preview.append(el("p", "原创建恢复记录无法安全解析，原文保留供核对，未发送新创建。"), el("pre", controller.recoveryError));
    if (controller.recovery) {
      preview.append(el("p", `原操作状态：${controller.recovery.status}。${controller.recovery.message}`));
      for (const action of controller.recovery.allowedActions ?? []) {
        const b = el("button", ({ keepCurrent: "保留当前状态并结束原操作", adoptAfterImages: "核验并确认原计划已写入的内容", resumeOriginalPlan: "仅续写原计划缺失部分" } as Record<string, string>)[action] ?? action); b.type = "button"; b.disabled = controller.busy; b.onclick = () => { void controller.resolve(action).then(draw).catch(error => { message.textContent = String(error); draw(); }); }; preview.append(b);
      }
    }
  }
  path.oninput = () => { try { controller.change({ path: path.value }); preview.replaceChildren(); create.disabled = true; } catch (e) { message.textContent = String(e); } };
  language.onchange = () => { try { controller.change({ language: language.value }); draw(); } catch (e) { message.textContent = String(e); } };
  region.onchange = () => { try { controller.change({ region: region.value }); draw(); } catch (e) { message.textContent = String(e); } };
  rootMode.onchange = () => { controller.change({ rootMode: rootMode.value as CreationDraft["rootMode"] }); draw(); };
  rootTitle.oninput = () => { controller.change({ rootTitle: rootTitle.value }); preview.replaceChildren(); create.disabled = true; };
  dialog.addEventListener("compositionstart", () => { composing = true; }); dialog.addEventListener("compositionend", () => { composing = false; }); dialog.addEventListener("keydown", event => event.stopPropagation());
  dialog.append(el("h2", "创建资料库"), el("p", "输入尚不存在的新文件夹。初始文档语言与地区将写入同一份可编辑的库默认样式；单篇和局部仍可使用原生覆盖。"), path, el("label", "文档排版语言"), language, el("label", "文档排版地区"), region, el("label", "根节点初始内容"), rootMode, rootTitle, queryChoices, preview, message, actions);
  document.body.append(dialog); draw(); dialog.showModal(); path.focus();
  return new Promise(resolve => { finish = value => { if (controller.busy || composing) return; dialog.close(); dialog.remove(); if (previousFocus?.isConnected) previousFocus.focus(); resolve(value); }; dialog.addEventListener("cancel", event => { event.preventDefault(); finish(null); }); });
}
