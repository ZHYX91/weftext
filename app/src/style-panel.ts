/** Library style UI. Native module bytes and Core transactions own persistence. */
export type NativeStyleValue = string | number | boolean;
export interface NativeStyleControl {
  key: string;
  label: string;
  kind: "text" | "number" | "select" | "boolean";
  value: NativeStyleValue;
  editable: boolean;
  reason?: string;
  options?: { label: string; value: NativeStyleValue }[];
  min?: number;
  max?: number;
  unit?: string;
}
export interface LibraryStyleState {
  path: string;
  source: string;
  baseId: string;
  enabled: boolean;
  entryFunction?: string;
  controls: NativeStyleControl[];
  warnings?: string[];
}
export interface LibraryStylePrepare {
  operationId: string;
  baseId: string;
  source?: string;
  changes?: Record<string, NativeStyleValue>;
  enabled?: boolean;
}
export interface LibraryStylePlan {
  operationId: string;
  planId: string;
  beforeSource: string;
  afterSource: string;
  summary: string[];
  enabled?: boolean;
}
export interface LibraryStyleHost {
  read(): Promise<LibraryStyleState>;
  prepare(request: LibraryStylePrepare): Promise<LibraryStylePlan>;
  apply(request: { operationId: string; planId: string }): Promise<LibraryStyleState>;
  openSource?(path: string): void | Promise<void>;
  onApplied?(state: LibraryStyleState): void | Promise<void>;
  operationId?(): string;
}

function resultUnknown(error: unknown): boolean {
  const code = error && typeof error === "object" && "code" in error ? String(error.code) : undefined;
  return code === undefined || ["unknown", "resultUnknown", "pending", "style_state_unavailable"].includes(code);
}

/** Keeps an uncertain request immutable and retries it with its original ID. */
export class NativeStyleController {
  state: LibraryStyleState | null = null;
  source = "";
  enabled = true;
  changes: Record<string, NativeStyleValue> = {};
  plan: LibraryStylePlan | null = null;
  busy = false;
  uncertain: "prepare" | "apply" | null = null;
  committedPendingRead = false;
  private request: LibraryStylePrepare | null = null;

  constructor(readonly host: LibraryStyleHost) {}

  get advanced(): boolean { return this.state !== null && this.source !== this.state.source; }
  get dirty(): boolean {
    return this.state !== null && (this.advanced || Object.keys(this.changes).length > 0 || this.enabled !== this.state.enabled);
  }
  get locked(): boolean { return this.busy || this.uncertain !== null; }

  async load(): Promise<void> {
    if (this.dirty || this.locked) throw new Error("设置草稿尚未处理，不能用重新读取覆盖。请先应用或关闭当前草稿。");
    this.install(await this.host.read());
  }

  setSource(source: string): void { this.assertEditable(); this.source = source; this.plan = null; this.request = null; }
  setEnabled(enabled: boolean): void { this.assertEditable(); this.enabled = enabled; this.plan = null; this.request = null; }
  resetDraft(): void { this.assertEditable(); this.install(this.state!); }
  setControl(key: string, value: NativeStyleValue): void {
    this.assertEditable();
    if (this.advanced) throw new Error("已编辑完整原生源码；先恢复原文才可使用简化控件。");
    const control = this.state?.controls.find(item => item.key === key);
    if (!control?.editable) throw new Error(control?.reason ?? "该规则没有唯一可写的原生参数，请使用源码入口。");
    if (typeof value !== typeof control.value) throw new Error("设置值类型不符合原生控件。");
    if (control.kind === "number" && (typeof value !== "number" || !Number.isFinite(value) || (control.min !== undefined && value < control.min) || (control.max !== undefined && value > control.max))) throw new Error("设置数值超出支持范围。");
    if (control.kind === "select" && !control.options?.some(option => option.value === value)) throw new Error("请选择提供的原生选项，或使用源码入口。");
    if (value === control.value) delete this.changes[key]; else this.changes[key] = value;
    this.plan = null; this.request = null;
  }

  async prepare(): Promise<LibraryStylePlan> {
    if (!this.state || this.busy || this.uncertain === "apply") throw new Error("当前设置不能准备新操作。");
    if (!this.request) {
      if (!this.dirty) throw new Error("没有待应用的设置改动。");
      this.request = {
        operationId: this.host.operationId?.() ?? crypto.randomUUID(),
        baseId: this.state.baseId,
        ...(this.advanced ? { source: this.source } : Object.keys(this.changes).length ? { changes: { ...this.changes } } : {}),
        ...(this.enabled !== this.state.enabled ? { enabled: this.enabled } : {}),
      };
    }
    this.busy = true;
    try {
      const plan = await this.host.prepare({ ...this.request, ...(this.request.changes ? { changes: { ...this.request.changes } } : {}) });
      if (plan.operationId !== this.request.operationId || !plan.planId || plan.beforeSource !== this.state.source || (this.request.source !== undefined && plan.afterSource !== this.request.source)) {
        this.request = null; this.plan = null; this.uncertain = null;
        throw Object.assign(new Error("准备结果与当前原生源码或操作不一致；草稿已保留。"), { code: "stylePlanMismatch" });
      }
      this.plan = plan; this.uncertain = null; return plan;
    } catch (error) {
      if (resultUnknown(error)) this.uncertain = "prepare";
      else { this.request = null; this.plan = null; this.uncertain = null; }
      throw error;
    } finally { this.busy = false; }
  }

  async apply(): Promise<LibraryStyleState> {
    if (!this.plan || this.busy || this.uncertain === "prepare") throw new Error("请先查看准备好的原生源码变更。");
    this.busy = true;
    try {
      const state = await this.host.apply({ operationId: this.plan.operationId, planId: this.plan.planId });
      this.install(state); return state;
    } catch (error) {
      if (resultUnknown(error)) {
        this.uncertain = "apply";
        if (error && typeof error === "object" && "code" in error && error.code === "style_state_unavailable") this.committedPendingRead = true;
      }
      else { this.request = null; this.plan = null; this.uncertain = null; }
      throw error;
    } finally { this.busy = false; }
  }

  private assertEditable(): void {
    if (!this.state || this.locked) throw new Error("操作仍在处理或结果待确认；设置草稿已保留，请确认同一操作。");
  }
  private install(state: LibraryStyleState): void {
    this.state = state; this.source = state.source; this.enabled = state.enabled;
    this.changes = {}; this.plan = null; this.request = null; this.uncertain = null; this.committedPendingRead = false;
  }
}

function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text = ""): HTMLElementTagNameMap[K] {
  const result = document.createElement(tag); result.className = className; result.textContent = text; return result;
}
export function nativeStyleControlReason(reason: string): string {
  return ({
    "syntax-invalid": "样式语法尚未有效，请在源码中修复。",
    "missing-or-ambiguous-entry": "没有唯一的库样式入口，请使用完整源码。",
    "missing-or-ambiguous-slot": "此参数未声明或存在多处规则，请使用源码。",
    "computed-or-unsupported-value": "此参数使用计算表达式或高级原生值，请使用源码。",
    "shadowed-binding": "此名称已被自定义，简化控件无法确认其原生含义。",
    "unverified-standard-binding": "无法确认这条规则使用原生标准元素，请使用源码。",
    "unproven-scope": "此规则的作用范围无法由简化控件确认，请使用源码。",
  } as Record<string, string>)[reason] ?? reason;
}

/** Both local and hosted panels edit the same inspected native parameter. */
export function nativeStyleControlInput(control: NativeStyleControl, value: NativeStyleValue): HTMLInputElement | HTMLSelectElement {
  if (control.kind === "select") {
    const select = element("select", "text-input");
    for (const [index, option] of (control.options ?? []).entries()) {
      const entry = element("option", "", option.label); entry.value = String(index); entry.selected = option.value === value; select.append(entry);
    }
    return select;
  }
  const input = element("input", "text-input"); input.type = control.kind === "number" ? "number" : control.kind === "boolean" ? "checkbox" : "text";
  if (control.kind === "boolean") input.checked = Boolean(value); else input.value = String(value);
  if (control.min !== undefined) input.min = String(control.min); if (control.max !== undefined) input.max = String(control.max);
  if (control.kind === "number") input.step = "any";
  return input;
}

/** Import style-panel.css and retain this returned controller per library.
 * Closing an uncertain operation retains its immutable request in the controller.
 */
export async function openLibraryStylePanel(host: LibraryStyleHost, controller = new NativeStyleController(host)): Promise<NativeStyleController> {
  if (!controller.state) await controller.load();
  const dialog = element("dialog", "modal library-style-panel");
  const header = element("div", "modal-heading");
  const heading = element("h2", "", "资料库默认样式"); heading.id = `style-heading-${crypto.randomUUID()}`;
  const close = element("button", "button-secondary compact", "关闭"); close.type = "button";
  header.append(heading, close); dialog.setAttribute("aria-labelledby", heading.id);
  const description = element("p", "modal-description", "字体、间距和边框来自同一份原生 Typst 模块。应用时只修改库设置和样式文件，笔记正文保持原样。单篇和局部规则遵循 Typst 的原生作用域。");
  const path = element("p", "style-path");
  const notices = element("div", "style-notices");
  const form = element("form", "modal-form");
  const enabledLabel = element("label", "style-enabled");
  const enabled = element("input"); enabled.type = "checkbox";
  enabledLabel.append(enabled, document.createTextNode("自动应用这份库默认样式"));
  const controls = element("div", "style-controls");
  const advanced = element("details", "style-native-source");
  advanced.append(element("summary", "", "高级：编辑完整原生 Typst 样式源码"));
  advanced.append(element("p", "field-help", "无法识别的规则会完整保留。编辑源码后，简化控件暂停，准备操作将使用这里的精确原文。自定义 transformation 可能固定样式，其覆盖效果由原生 Typst 决定。"));
  const sourceLabel = element("label", "field-label", "样式模块源码");
  const source = element("textarea", "text-input style-source"); source.spellcheck = false; source.rows = 16;
  sourceLabel.append(source); advanced.append(sourceLabel);
  if (host.openSource) {
    const sourceEntry = element("button", "text-button", "打开样式文件源码"); sourceEntry.type = "button";
    sourceEntry.onclick = () => { void Promise.resolve(host.openSource!(controller.state!.path)).catch(showError); };
    advanced.append(sourceEntry);
  }
  const preview = element("div", "style-plan"); preview.hidden = true;
  const status = element("p", "style-status"); status.setAttribute("role", "status"); status.setAttribute("aria-live", "polite");
  const actions = element("div", "modal-actions");
  const reset = element("button", "button-secondary", "恢复已读设置"); reset.type = "button";
  const prepare = element("button", "button-secondary", "查看变更"); prepare.type = "button";
  const apply = element("button", "button-primary", "应用库样式"); apply.type = "button";
  actions.append(reset, prepare, apply); form.append(enabledLabel, controls, notices, advanced, preview, status, actions); dialog.append(header, description, path, form);
  const inputs: { input: HTMLInputElement | HTMLSelectElement; control: NativeStyleControl }[] = [];

  function showError(error: unknown): void {
    status.classList.add("style-error");
    status.textContent = error instanceof Error ? error.message : String(error);
    if (controller.committedPendingRead) status.textContent += " 原操作已提交；重试将重新读取当前设置，操作记录已保留。";
    else if (controller.uncertain) status.textContent += " 操作结果待确认，重试将确认同一操作。设置草稿已保留。";
    refresh();
  }
  function edited(): void { status.classList.remove("style-error"); status.textContent = "设置草稿尚未应用。"; preview.hidden = true; refresh(); }
  function refresh(): void {
    enabled.disabled = controller.locked; source.disabled = controller.locked;
    for (const item of inputs) item.input.disabled = !item.control.editable || controller.advanced || controller.locked;
    prepare.disabled = controller.busy || controller.uncertain === "apply" || (!controller.dirty && !controller.uncertain);
    apply.disabled = controller.busy || !controller.plan || controller.uncertain === "prepare";
    prepare.textContent = controller.uncertain === "prepare" ? "确认同一准备操作" : "查看变更";
    apply.textContent = controller.committedPendingRead ? "重新读取已提交设置" : controller.uncertain === "apply" ? "确认同一保存操作" : "应用库样式";
    reset.disabled = controller.locked || !controller.dirty;
    close.disabled = controller.busy;
  }
  function drawState(): void {
    const state = controller.state!; path.textContent = state.path; enabled.checked = controller.enabled; source.value = controller.source;
    controls.replaceChildren(); notices.replaceChildren(); inputs.length = 0;
    for (const warning of state.warnings ?? []) notices.append(element("p", "field-help style-warning", warning));
    for (const control of state.controls) {
      const currentValue = controller.changes[control.key] ?? control.value;
      const label = element("label", "field-label", control.label + (control.unit ? `（${control.unit}）` : ""));
      const input = nativeStyleControlInput(control, currentValue);
      input.onchange = () => {
        try {
          const value = control.kind === "select" ? control.options![Number(input.value)].value : control.kind === "boolean" ? (input as HTMLInputElement).checked : control.kind === "number" ? Number(input.value) : input.value;
          controller.setControl(control.key, value); edited();
        } catch (error) { showError(error); }
      };
      label.append(input); if (control.reason) label.append(element("span", "field-help", nativeStyleControlReason(control.reason))); controls.append(label); inputs.push({ input, control });
    }
    if (!state.controls.length) notices.append(element("p", "field-help", "这份模块没有可无损编辑的简化控件。请使用原生源码入口；全部现有规则保持原样。"));
    refresh();
  }
  enabled.onchange = () => { try { controller.setEnabled(enabled.checked); edited(); } catch (error) { showError(error); } };
  source.oninput = () => { try { controller.setSource(source.value); edited(); } catch (error) { showError(error); } };
  reset.onclick = () => { try { controller.resetDraft(); drawState(); preview.hidden = true; status.classList.remove("style-error"); status.textContent = "已恢复本次读取的原生样式；磁盘没有改动。"; } catch (error) { showError(error); } };
  prepare.onclick = () => {
    void (async () => {
      const request = controller.prepare(); refresh();
      try {
        const plan = await request; preview.replaceChildren(element("h3", "", "即将保存的原生变更"));
        const summary = element("ul"); for (const line of plan.summary) summary.append(element("li", "", line)); preview.append(summary);
        const columns = element("div", "style-source-comparison");
        for (const [caption, value] of [["操作前", plan.beforeSource], ["操作后", plan.afterSource]]) {
          const label = element("label", "field-label", caption); const area = element("textarea", "text-input style-source"); area.readOnly = true; area.value = value; area.rows = 10; label.append(area); columns.append(label);
        }
        preview.append(columns); preview.hidden = false; status.classList.remove("style-error"); status.textContent = "已准备，检查后应用。"; refresh();
      } catch (error) { showError(error); }
    })();
  };
  apply.onclick = () => {
    void (async () => {
      const request = controller.apply(); refresh();
      try {
        const state = await request; drawState(); preview.hidden = true; status.classList.remove("style-error"); status.textContent = "库样式已保存，受影响预览将重新编译。";
        try { await host.onApplied?.(state); } catch (error) { showError(error); status.textContent = `库样式已保存；刷新预览时遇到问题：${error instanceof Error ? error.message : String(error)}`; }
      } catch (error) { showError(error); }
    })();
  };
  form.onsubmit = event => { event.preventDefault(); prepare.click(); };
  document.body.append(dialog); drawState(); dialog.showModal();
  await new Promise<void>(resolve => {
    function dismiss(): void {
      if (controller.busy) { status.textContent = "操作仍在处理，设置草稿已保留。"; return; }
      dialog.close(); dialog.remove(); resolve();
    }
    close.onclick = dismiss; dialog.addEventListener("cancel", event => { event.preventDefault(); dismiss(); });
  });
  return controller;
}
