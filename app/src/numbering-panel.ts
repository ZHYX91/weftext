export type HeadingNumberingScheme = "none" | "decimal" | "chinese";
export interface HeadingNumberingIntent { scheme: HeadingNumberingScheme; start: number; maxLevel: number; excludeLabels: string[] }
export interface HeadingNumberingInspection { sourceHash: string; settings: HeadingNumberingIntent | null; editable: boolean; reason: string | null }
export interface HeadingNumberingPreview { beforeSource: string; afterSource: string; headings?: { text: string; before: string | null; after: string | null }[] }
export interface NumberingHost {
  authorInputs?(): import("./author-input").AuthorInputSession | undefined;
  closeGuard?(guard: () => boolean): () => void;
  source(): string;
  inspection(): HeadingNumberingInspection;
  preview(action: HeadingNumberingIntent): Promise<HeadingNumberingPreview>;
  apply(action: HeadingNumberingIntent): Promise<boolean>;
  personalDefaultKey: string;
  personalNoteKey: string;
  displayPersonal(enabled: boolean): void;
  readable?(): boolean;
  composing?(): boolean;
  binding?(): string;
  observeState?(element: HTMLElement, listener: () => void): () => void;
}
export class NumberingController {
  action: HeadingNumberingIntent;
  preview: HeadingNumberingPreview | null = null;
  personal: "default" | "show" | "hide" = "default";
  busy = false;
  valid = true;
  private generation = 0;
  private readonly binding: string;
  constructor(readonly host: NumberingHost, private storage: Pick<Storage, "getItem" | "setItem" | "removeItem"> = localStorage) {
    this.action = structuredClone(host.inspection().settings ?? { scheme: "decimal", start: 1, maxLevel: 6, excludeLabels: [] });
    this.binding = host.binding?.() ?? "";
    const value = storage.getItem(host.personalNoteKey); if (value === "show" || value === "hide") this.personal = value;
  }
  get personalVisible(): boolean { return this.personal === "default" ? this.storage.getItem(this.host.personalDefaultKey) === "show" : this.personal === "show"; }
  get available(): boolean { return this.host.readable?.() !== false && (this.host.binding?.() ?? "") === this.binding; }
  get canPropose(): boolean { const inspection = this.host.inspection(); return inspection.editable || inspection.reason === "requires-frozen-endnotes-module"; }
  invalidate(): void { this.preview = null; this.valid = false; this.generation++; }
  observeSurface(element: HTMLElement, changed: () => void): () => void {
    let source = this.host.source();
    return this.host.observeState?.(element, () => { const current = this.host.source(); if (current !== source || !this.available) { source = current; this.invalidate(); } changed(); }) ?? (() => {});
  }
  setPersonal(mode: "default" | "show" | "hide"): void {
    if (mode === "default") this.storage.removeItem(this.host.personalNoteKey); else this.storage.setItem(this.host.personalNoteKey, mode);
    this.personal = mode; this.host.displayPersonal(this.personalVisible);
  }
  setPersonalDefault(enabled: boolean): void { this.storage.setItem(this.host.personalDefaultKey, enabled ? "show" : "hide"); this.host.displayPersonal(this.personalVisible); }
  change(action: HeadingNumberingIntent): void {
    if (this.busy || !this.available || this.host.composing?.()) throw new Error("原编号预览仍在处理、输入法组合中或当前笔记已不可用。");
    this.preview = null; this.generation++; this.valid = false;
    if (!this.canPropose) throw new Error("当前原生编号规则无法无损修改，原文已保留。");
    if (!Number.isSafeInteger(action.start) || action.start < 1 || action.start > 1_000_000 || !Number.isSafeInteger(action.maxLevel) || action.maxLevel < 1 || action.maxLevel > 6 || action.excludeLabels.length > 200) throw new Error("请核对起始值、层级和排除范围。");
    this.action = structuredClone(action); this.valid = true;
  }
  async prepare(): Promise<void> {
    if (this.busy || !this.valid || !this.available || this.host.composing?.() || !this.canPropose) throw new Error("当前编号规则或输入不能准备修改。");
    const source = this.host.source(), action = structuredClone(this.action), generation = ++this.generation;
    this.busy = true;
    try {
      const preview = await this.host.preview(action);
      if (generation !== this.generation || !this.available || source !== this.host.source() || preview.beforeSource !== source) throw new Error("正文、权限或编号输入已经变化，请重新预览。");
      this.preview = structuredClone(preview);
    } finally { this.busy = false; }
  }
  async apply(): Promise<boolean> {
    if (this.busy || !this.preview || !this.available || this.host.composing?.() || this.preview.beforeSource !== this.host.source() || !this.canPropose) throw new Error("请先查看当前正文的编号预览。");
    this.busy = true;
    try { const applied = await this.host.apply(structuredClone(this.action)); if (applied) this.preview = null; return applied; } finally { this.busy = false; }
  }
  cancel(): void { this.generation++; this.preview = null; }
}
const el = <K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] => { const e = document.createElement(tag); e.textContent = text; return e; };
export async function openNumberingPanel(host: NumberingHost): Promise<void> {
  const focus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const controller = new NumberingController(host), dialog = el("dialog"); dialog.className = "modal library-style-panel"; dialog.setAttribute("aria-label", "当前笔记编号控制");
  const authorInputs = host.authorInputs?.(), stopCloseGuard = host.closeGuard?.(() => dialog.open);
  let composing = false;
  dialog.addEventListener("compositionstart", () => { composing = true; controller.invalidate(); draw(); });
  dialog.addEventListener("compositionend", () => { composing = false; changed(); });
  const title = el("h2", "当前笔记：编号控制");
  const personal = el("select"); personal.setAttribute("aria-label", "此篇个人辅助序号显示");
  for (const [value, text] of [["default", "跟随个人默认"], ["show", "此篇显示个人辅助序号"], ["hide", "此篇隐藏个人辅助序号"]]) { const o = el("option", text); o.value = value; personal.append(o); } personal.value = controller.personal;
  personal.onchange = () => { try { controller.setPersonal(personal.value as "default" | "show" | "hide"); } catch (e) { message.textContent = String(e); } };
  const ownDefault = el("input"); ownDefault.type = "checkbox"; ownDefault.checked = localStorage.getItem(host.personalDefaultKey) === "show"; ownDefault.onchange = () => { try { controller.setPersonalDefault(ownDefault.checked); } catch (e) { message.textContent = String(e); } };
  const defaultLabel = el("label", "个人默认显示辅助序号"); defaultLabel.append(ownDefault);
  const scheme = el("select"); scheme.setAttribute("aria-label", "文档编号方案"); for (const [value, text] of [["none", "无编号"], ["decimal", "十进制层级 · 1.1"], ["chinese", "中文层级 · 一.一"]]) { const o = el("option", text); o.value = value; scheme.append(o); } scheme.value = controller.action.scheme;
  const start = el("input"); start.type = "number"; start.min = "1"; start.max = "1000000"; start.value = String(controller.action.start); start.setAttribute("aria-label", "文档编号起始值");
  const level = el("input"); level.type = "number"; level.min = "1"; level.max = "6"; level.value = String(controller.action.maxLevel); level.setAttribute("aria-label", "文档编号最大层级");
  const exclude = el("input"); exclude.value = controller.action.excludeLabels.join(", "); exclude.placeholder = "稳定标题标记，用逗号分隔"; exclude.setAttribute("aria-label", "文档编号排除标题标记");
  authorInputs?.bind(start, "numbering-start"); authorInputs?.bind(level, "numbering-level"); authorInputs?.bind(exclude, "numbering-exclude");
  const message = el("p"); message.setAttribute("role", "status"); const preview = el("div");
  const actions = el("div"); actions.className = "modal-actions";
  const button = (text: string, run: () => void | Promise<void>) => { const b = el("button", text); b.type = "button"; b.className = "button-secondary"; b.onclick = () => { void (async () => { try { if (composing || host.composing?.()) throw new Error("请先完成输入法组合。"); const promise = run(); draw(); await promise; } catch (e) { message.textContent = e instanceof Error ? e.message : String(e); } draw(); })(); }; actions.append(b); return b; };
  const prepare = button("预览文档编号", () => controller.prepare());
  const apply = button("应用到文档（一次草稿修改）", async () => { if (await controller.apply()) message.textContent = "文档编号已写入当前原生草稿，可撤销；保存沿用原入口。"; });
  const close = button("关闭", () => { controller.cancel(); dialog.close(); });
  function changed(): void { try { controller.change({ scheme: scheme.value as HeadingNumberingScheme, start: Number(start.value), maxLevel: Number(level.value), excludeLabels: exclude.value.split(/[,，]+/).map(s => s.trim()).filter(Boolean) }); message.textContent = "文档编号输入尚未应用。"; } catch (e) { message.textContent = String(e); } draw(); }
  for (const input of [scheme, start, level, exclude]) { input.onchange = changed; input.oninput = event => { controller.invalidate(); if (!(event as InputEvent).isComposing && !composing) changed(); else draw(); }; }
  function draw(): void {
    if (!controller.available) controller.invalidate();
    const inspection = controller.available ? host.inspection() : { editable: false, reason: "当前笔记或读取权限已变化" }, canPropose = controller.available && controller.canPropose; for (const input of [scheme, start, level, exclude]) input.disabled = controller.busy || !canPropose;
    prepare.disabled = controller.busy || composing || !controller.valid || !canPropose; apply.disabled = controller.busy || composing || !controller.preview || !canPropose; close.disabled = controller.busy;
    preview.replaceChildren();
    if (canPropose && inspection.reason === "requires-frozen-endnotes-module") preview.append(el("p", "将核对本篇尾注模块后预览编号；核对未通过时保留原稿。"));
    else if (!inspection.editable) preview.append(el("p", `复杂原生规则已保留，文档控件暂不可无损改写：${inspection.reason ?? "未知规则"}。个人辅助显示仍可独立使用。`));
    if (controller.preview) {
      preview.append(el("h3", "原生文档改动预览"));
      for (const h of controller.preview.headings ?? []) preview.append(el("p", `${h.text}：${h.before ?? "无编号"} → ${h.after ?? "无编号"}`));
      const details = el("details"); details.append(el("summary", "准确原生源码前后对照"), el("pre", controller.preview.beforeSource), el("pre", controller.preview.afterSource)); preview.append(details);
    }
  }
  dialog.append(title, el("h3", "个人显示"), el("p", "个人辅助序号仅供当前用户导航；不会隐藏作者原有编号、修改源、原生引用或导出。恢复默认只移除此篇个人覆盖。"), personal, defaultLabel, el("h3", "文档编号：单篇原生覆盖"), el("p", "编号方案、起始值及排除项影响真实引用和输出；已有字面前缀保持原文。排除项必须对应当前源中唯一、已有的标题标记。"), scheme, start, level, exclude, preview, message, actions);
  document.body.append(dialog); const stopObserving = controller.observeSurface(dialog, draw); draw(); dialog.showModal(); await new Promise<void>(resolve => { dialog.addEventListener("close", () => { stopCloseGuard?.(); authorInputs?.retire(); stopObserving(); controller.cancel(); dialog.remove(); if (focus?.isConnected) focus.focus(); resolve(); }, { once: true }); dialog.addEventListener("cancel", e => { if (controller.busy || composing || host.composing?.()) e.preventDefault(); }); });
}
