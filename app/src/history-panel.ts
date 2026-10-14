import "./history-panel.css";
import {
  HistoryController, historySourceDiff, historyTimeLabel,
  type HistoryComparison, type HistoryContext, type HistoryRestoreBasis, type HistoryRestoreResult, type HistorySource,
} from "./history-state";

export interface HistoryPanelHost {
  getDocument(): HistoryContext | null;
  request<T>(path: string, body?: unknown): Promise<T>;
  /** Settle/persist actual author input before preparing a restore, never during browsing. */
  prepareRestore?(): Promise<void>;
  /** The source owner applies the saved base and durably preserves newer input and Undo. */
  restored(result: HistoryRestoreResult, basis: Readonly<HistoryRestoreBasis>): Promise<void>;
  /** Keep ordinary Save from racing a pending or uncertain restore. */
  restorePending?(basis: Readonly<HistoryRestoreBasis>, pending: boolean): void;
  notify(message: string, error?: boolean): void;
}

let nextHistoryId = 0;
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: string): HTMLElementTagNameMap[K] {
  const value = document.createElement(tag); value.className = className;
  if (text !== undefined) value.textContent = text;
  return value;
}
function button(label: string, run: () => void | Promise<void>, primary = false): HTMLButtonElement {
  const value = element("button", `history-button${primary ? " primary" : ""}`, label); value.type = "button";
  value.onclick = () => { void Promise.resolve(run()).catch(() => { /* Each action reports its retained state. */ }); };
  return value;
}
function sourcePane(label: string, source: string): HTMLElement {
  const section = element("section", "history-source-pane"); const heading = element("h3", "", label);
  heading.id = `history-source-heading-${++nextHistoryId}`;
  const text = element("pre", "history-source", source); text.tabIndex = 0; text.setAttribute("role", "region"); text.setAttribute("aria-labelledby", heading.id);
  section.append(heading, text); return section;
}
const availability: Record<string, string> = { available: "准确源码可读", missing: "准确源码缺失", unreadable: "准确源码不可读", invalid: "记录无效" };

class HistoryDialog {
  readonly dialog = element("dialog", "history-panel");
  private readonly controller: HistoryController;
  private readonly listHost = element("ol", "history-versions");
  private readonly pathLabel = element("p", "history-path");
  private readonly range = element("p", "history-help");
  private readonly warnings = element("div", "history-warnings");
  private readonly sourceHost = element("div", "history-content");
  private readonly protectedHost = element("section", "history-protected");
  private readonly status = element("p", "history-status");
  private readonly operationHost = element("div", "history-operation");
  private readonly compareField = element("fieldset", "history-compare-choice");
  private readonly compareButton: HTMLButtonElement;
  private readonly restoreButton: HTMLButtonElement;
  private readonly refreshButton: HTMLButtonElement;
  private readonly savedChoice = element("input");
  private readonly draftChoice = element("input");
  private readBusy = false;
  private readGeneration = 0;
  private chosenVersionId: string | null = null;
  private restoreDialog: HTMLDialogElement | null = null;
  private acknowledging = false;
  private unacknowledged: { result: HistoryRestoreResult; basis: Readonly<HistoryRestoreBasis> } | null = null;

  constructor(private readonly host: HistoryPanelHost, context: HistoryContext) {
    this.controller = new HistoryController({
      id: () => crypto.randomUUID(), current: () => host.getDocument(),
      list: request => host.request("/api/history/list", request),
      read: request => host.request("/api/history/read", request),
      compare: request => host.request("/api/history/compare", request),
      restore: request => { const basis = this.controller.pendingBasis; if (basis) host.restorePending?.(basis, true); return host.request("/api/history/restore", request); },
      status: request => host.request("/api/history/status", request),
    }, Object.freeze(structuredClone(context)));
    this.dialog.setAttribute("aria-label", "本地笔记时间线");
    const header = element("header", "history-header"); header.append(element("h2", "", "本地笔记时间线"), button("关闭时间线", () => this.dialog.close()));
    this.pathLabel.textContent = context.path;
    const intro = element("p", "history-help", "按这份笔记的稳定身份查看 Core 实际保留的已保存版本。浏览和比较保持只读，草稿与撤销记录继续由编辑器保留。");
    this.refreshButton = button("刷新保存记录", () => this.refresh());
    const side = element("aside", "history-sidebar"); side.setAttribute("aria-label", "已保存版本"); side.append(this.refreshButton, this.range, this.listHost);
    this.compareField.append(element("legend", "", "比较对象（本次读取固定）"));
    const group = `history-compare-${++nextHistoryId}`;
    for (const [control, value, label] of [[this.savedChoice, "currentSaved", "当前已保存版本"], [this.draftChoice, "draftSnapshot", "当前草稿快照"]] as const) {
      control.type = "radio"; control.name = group; control.value = value; control.setAttribute("aria-label", label);
      const item = element("label", "history-radio"); item.append(control, document.createTextNode(label)); this.compareField.append(item);
    }
    this.savedChoice.checked = !context.dirty;
    this.compareButton = button("比较所选版本", () => this.compare());
    this.restoreButton = button("恢复这个版本", () => this.prepareRestore(), true);
    const actions = element("div", "history-actions"); actions.append(this.compareField, this.compareButton, this.restoreButton);
    const main = element("section", "history-main"); main.append(this.warnings, this.sourceHost, actions, this.status, this.operationHost, this.protectedHost);
    const body = element("div", "history-layout"); body.append(side, main);
    this.status.setAttribute("role", "status"); this.status.setAttribute("aria-live", "polite");
    this.dialog.append(header, this.pathLabel, intro, body); document.body.append(this.dialog);
    this.draw();
  }
  async open(): Promise<void> {
    if (!this.dialog.open) this.dialog.showModal();
    if (!this.controller.list) await this.refresh();
    else this.draw();
  }
  private report(error: unknown, prefix = ""): void {
    const message = `${prefix}${error instanceof Error ? error.message : String(error)}`;
    this.status.textContent = message; this.status.classList.add("history-error"); this.host.notify(message, true);
  }
  private async read(action: () => Promise<unknown>): Promise<void> {
    const generation = ++this.readGeneration; this.readBusy = true; this.status.classList.remove("history-error"); this.status.textContent = "正在读取准确保存记录…"; this.drawControls();
    try { await action(); if (generation === this.readGeneration) this.status.textContent = ""; }
    catch (error) { if (generation === this.readGeneration) this.report(error); }
    finally { if (generation === this.readGeneration) { this.readBusy = false; this.draw(); } }
  }
  private async refresh(): Promise<void> { await this.read(async () => { await this.controller.refresh(); }); }
  private async select(versionId: string): Promise<void> {
    this.chosenVersionId = versionId;
    await this.read(async () => { await this.controller.select(versionId); });
  }
  private async compare(): Promise<void> {
    if (!this.savedChoice.checked && !this.draftChoice.checked) { this.report(new Error("请明确选择当前已保存版本或当前草稿快照。")); return; }
    if (this.draftChoice.checked) {
      try { this.controller.compareDraft(); this.status.textContent = "草稿快照已固定；继续输入不会改变本次比较。"; this.status.classList.remove("history-error"); this.draw(); }
      catch (error) { this.report(error); }
    } else await this.read(async () => { await this.controller.compareSaved(); });
  }
  private draw(): void {
    const list = this.controller.list;
    this.pathLabel.textContent = list?.path ?? this.controller.note.path;
    if (list) {
      const { oldestSavedAtUnixMs, newestSavedAtUnixMs } = list.retention;
      const times = oldestSavedAtUnixMs !== null && newestSavedAtUnixMs !== null ? `已知保存时间：${historyTimeLabel({ savedAtUnixMs: oldestSavedAtUnixMs, timeBasis: "commitClock" })} 至 ${historyTimeLabel({ savedAtUnixMs: newestSavedAtUnixMs, timeBasis: "commitClock" })}。` : "";
      this.range.textContent = `${list.retention.knownVersions} 条已知保存记录，${list.retention.availableVersions} 条准确源码可读。${times}仅包含本机 Core 实际保留记录；不保证更早、已清除、逐字输入或其他设备历史。空列表不代表从未保存。`;
    } else this.range.textContent = "只显示 Core 实际保留范围；列表为空也不代表这份笔记从未保存。";
    this.warnings.replaceChildren();
    if (list?.currentness === "external") this.warnings.append(element("p", "history-warning", "当前文件存在未接纳的外部修改。它不会冒充已保存版本；恢复前须先处理当前基线。"));
    for (const warning of list?.warnings ?? []) this.warnings.append(element("p", "history-warning", warning));
    this.listHost.replaceChildren();
    for (const version of list?.versions ?? []) {
      const row = element("li", "history-version");
      const choice = button(historyTimeLabel(version), () => this.select(version.versionId));
      choice.dataset.historyVersion = version.versionId; choice.setAttribute("aria-pressed", String(this.chosenVersionId === version.versionId));
      const sequence = version.commitSequence === null ? "提交顺序未知" : `提交 ${version.commitSequence}`;
      choice.setAttribute("aria-label", `${historyTimeLabel(version)}，${sequence}，${availability[version.status] ?? version.status}${version.versionId === list!.currentSavedVersionId ? "，当前已保存" : ""}`);
      choice.append(element("span", "history-version-meta", `${sequence} · ${availability[version.status] ?? version.status}${version.versionId === list!.currentSavedVersionId ? " · 当前已保存" : ""}`));
      choice.disabled = this.readBusy; row.append(choice);
      if (version.status !== "available" && version.message) row.append(element("p", "history-help", version.message)); this.listHost.append(row);
    }
    if (list && !list.versions.length) this.listHost.append(element("li", "history-help", "没有可列出的保留记录。无法据此判断过去是否保存过。"));
    this.sourceHost.replaceChildren();
    if (this.controller.comparison) this.drawComparison(this.controller.comparison);
    else if (this.controller.selected) this.drawSource(this.controller.selected);
    else if (this.chosenVersionId) this.sourceHost.append(element("p", "history-warning", "所选准确旧版当前不可用。没有使用最新版本替代。"));
    else this.sourceHost.append(element("p", "history-help", "选择一条已保存记录，查看准确的只读 Typst 源码。保存时间来自提交时设备时钟；列表按记录的保存时间浏览，实际提交顺序另列；设备时钟回拨不改变原提交关系。"));
    this.drawProtected(); this.drawControls(); this.drawOperation();
  }
  private drawSource(source: HistorySource): void {
    this.sourceHost.append(element("p", "history-help", `${historyTimeLabel(source)} · ${source.byteLength ?? "未知"} 字节 · ${source.versionId}`));
    this.sourceHost.append(sourcePane("所选已保存版本 · 只读准确源码", source.source));
    this.sourceHost.append(element("p", "history-help", "准确源码保持可读。当前仅提供源码查看；原配置或依赖缺失时不以当前设置替代旧版语义与排版。"));
  }
  private drawComparison(comparison: HistoryComparison): void {
    const label = comparison.comparison === "currentSaved" ? "当前已保存版本 · 本次读取固定" : `当前草稿快照 · 草稿 ${comparison.current.serial} · 本次点击固定`;
    this.sourceHost.append(element("p", "history-help", comparison.equal ? "两个固定版本的源码完全一致。" : "两个固定版本的源码不同。删除和新增分别高亮；准确文本保持只读。"));
    const columns = element("div", "history-comparison");
    const left = sourcePane("所选历史版本 · 只读", ""), right = sourcePane(label, "");
    const leftText = left.querySelector("pre")!, rightText = right.querySelector("pre")!;
    for (const part of historySourceDiff(comparison.selected.source, comparison.current.source)) {
      if (part.kind !== "added") leftText.append(element("span", part.kind === "removed" ? "history-removed" : "", part.text));
      if (part.kind !== "removed") rightText.append(element("span", part.kind === "added" ? "history-added" : "", part.text));
    }
    columns.append(left, right); this.sourceHost.append(columns);
    this.sourceHost.append(element("p", "history-help", `所选版本 ${comparison.selected.versionId} · 比较基线 ${comparison.current.baseId}${comparison.comparison === "draftSnapshot" ? "。这份快照未写入笔记；它与当前已保存版本分别显示。" : "。后续保存不会改变本次比较。"}`));
  }
  private drawProtected(): void {
    this.protectedHost.replaceChildren();
    const drafts = this.controller.list?.protectedDrafts ?? [];
    if (!drafts.length) return;
    this.protectedHost.append(element("h3", "", `Core 已保护的草稿 · ${drafts.length} 份`), element("p", "history-help", "这些准确草稿来自现有本机草稿恢复记录，可在重启后找回；不属于已保存版本。"));
    drafts.forEach((draft, index) => {
      const details = element("details", "history-draft"); details.append(element("summary", "", `受保护草稿 ${index + 1}${draft.baseKnown ? "" : " · 原基线未知"}`), element("p", "history-help", draft.path), sourcePane("受保护草稿 · 只读", draft.source));
      if (draft.baseKnown) { const base = element("details"); base.append(element("summary", "", "查看这份草稿的原保存基线"), sourcePane("原保存基线 · 只读", draft.baseSource)); details.append(base); }
      this.protectedHost.append(details);
    });
  }
  private drawControls(): void {
    const disabled = this.readBusy || this.controller.locked || this.unacknowledged !== null || this.acknowledging;
    this.refreshButton.disabled = this.readBusy || this.controller.busy;
    this.compareButton.disabled = !this.controller.selected || this.readBusy || this.controller.busy;
    this.restoreButton.disabled = !this.controller.selected || disabled || this.host.getDocument()?.readOnly === true;
    this.savedChoice.disabled = this.readBusy || this.controller.busy; this.draftChoice.disabled = this.savedChoice.disabled;
  }
  private drawOperation(): void {
    this.operationHost.replaceChildren();
    const pending = this.controller.pendingBasis;
    if (pending) {
      const state = this.controller.operationState;
      const message = state === "missing" ? "原操作尚无已登记结果，可重试完全相同的原操作。" : state === "recoveryPending" ? "原恢复仍需处理；请从应用的操作恢复入口处理后查询。" : "恢复结果待确认；请查询原操作，不会创建新的保存编号。";
      this.operationHost.append(element("p", "history-warning", `${message} 操作编号：${this.controller.operationId}`));
      const inspect = button("查询原恢复结果", () => this.inspect()); const retry = button("重试同一恢复操作", () => this.retry());
      inspect.disabled = this.controller.busy; retry.disabled = this.controller.busy || state === "recoveryPending";
      this.operationHost.append(inspect, retry);
    }
    if (this.unacknowledged) {
      const retry = button("重试回执应用与草稿保护", () => this.acknowledge()); retry.disabled = this.acknowledging;
      this.operationHost.append(element("p", "history-warning", "Core 已保存旧版；编辑器的回执应用或草稿持久化尚未完成。请继续处理这份已保存回执。"), retry);
    }
  }
  private async prepareRestore(): Promise<void> {
    await this.read(async () => {
      await this.host.prepareRestore?.();
      const review = await this.controller.prepareRestore();
      const dialog = element("dialog", "history-restore"); dialog.setAttribute("aria-label", "恢复为一次新保存");
      const heading = element("h2", "", "恢复为一次新保存");
      const description = element("p", "history-help", "确认后，所选旧版将产生一次新的普通保存；后来的保存记录继续保留。当前未保存草稿将存入 Core 的现有恢复记录，并保留在编辑器中。");
      const metadata = element("p", "history-help", `所选版本 ${review.basis.selectedVersionId} · 当前保存基线 ${review.basis.baseId}`);
      const sourceDetails = element("details", "history-draft"); sourceDetails.append(element("summary", "", "核对将新保存的准确旧版与当前保存内容"), sourcePane("将新保存的准确旧版", review.basis.selectedSource), sourcePane("当前已保存内容", review.comparison.current.source));
      const protect = element("input"); protect.type = "checkbox"; protect.setAttribute("aria-label", "保留未保存草稿");
      const choice = element("label", "history-protection"); choice.append(protect, document.createTextNode("保留未保存草稿及本机已有恢复草稿，再恢复这个版本"));
      const draftInfo = element("p", "history-help", review.basis.dirty ? "当前有未保存草稿，必须明确保护。确认期间的新输入也将继续保留。" : "当前编辑稿与保存基线一致；本机已有的其他恢复草稿同样保留。" );
      const note = element("p", "history-status"); note.setAttribute("role", "status");
      const confirm = button("确认恢复并新保存", async () => {
        if (!protect.checked) return;
        const basis = review.basis; confirm.disabled = true; cancel.disabled = true; protect.disabled = true;
        try {
          const result = await this.controller.restore("retain");
          this.unacknowledged = { result, basis }; await this.acknowledge();
          dialog.close();
        } catch (error) {
          this.report(error);
          note.textContent = error instanceof Error ? error.message : String(error);
          dialog.close();
        } finally {
          this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null);
          this.drawControls(); this.drawOperation();
        }
      }, true);
      confirm.disabled = true; protect.onchange = () => { confirm.disabled = !protect.checked; };
      const cancel = button("取消恢复", () => dialog.close()); cancel.autofocus = true;
      const actions = element("div", "history-actions"); actions.append(cancel, confirm);
      dialog.append(heading, description, metadata, sourceDetails, choice, draftInfo, note, actions);
      dialog.oncancel = event => { if (this.controller.busy) event.preventDefault(); };
      dialog.onclose = () => { if (!this.controller.operationId && this.controller.review) this.controller.cancelRestore(); dialog.remove(); this.restoreDialog = null; };
      document.body.append(dialog); this.restoreDialog = dialog; dialog.showModal();
    });
  }
  private async acknowledge(): Promise<void> {
    const pending = this.unacknowledged;
    if (!pending || this.acknowledging) return;
    this.acknowledging = true; this.drawControls(); this.drawOperation();
    try {
      await this.host.restored(pending.result, pending.basis);
      this.unacknowledged = null; this.host.restorePending?.(pending.basis, false);
      this.status.textContent = "所选旧版已作为新版本保存。受保护草稿和后来的版本继续保留。"; this.status.classList.remove("history-error");
      this.host.notify(this.status.textContent);
      try { await this.controller.refresh(); } catch (error) { this.report(error, "新版本已保存，但保存记录刷新失败："); }
      this.draw();
    } catch (error) { this.report(error, "Core 已保存，草稿与回执还需要继续处理："); throw error; }
    finally { this.acknowledging = false; this.drawControls(); this.drawOperation(); }
  }
  private async inspect(): Promise<void> {
    const basis = this.controller.pendingBasis;
    if (!basis) return;
    try {
      const attempt = this.controller.inspectOriginal(); this.drawControls(); this.drawOperation();
      const result = await attempt;
      if (result) { this.unacknowledged = { result, basis }; await this.acknowledge(); }
      else this.status.textContent = this.controller.operationState === "resolved" ? "原恢复已处理，未报告新保存。请刷新并重新核对当前稿。" : this.controller.operationId === null ? "Core 已确认被拒绝的原恢复没有登记保存。草稿保持原样，请处理原错误后重新准备。" : "查询没有取得已完成回执，原操作编号继续保留。";
    } catch (error) { this.report(error); }
    finally { this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null); this.drawControls(); this.drawOperation(); }
  }
  private async retry(): Promise<void> {
    const basis = this.controller.pendingBasis;
    if (!basis) return;
    try { const attempt = this.controller.retryOriginal(); this.drawControls(); this.drawOperation(); const result = await attempt; this.unacknowledged = { result, basis }; await this.acknowledge(); }
    catch (error) { this.report(error); }
    finally { this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null); this.drawControls(); this.drawOperation(); }
  }
  dispose(): void { this.readGeneration++; this.restoreDialog?.remove(); this.dialog.remove(); }
}

/** Closing/reopening preserves an uncertain operation and its immutable request. */
export class HistoryPanel {
  private readonly dialogs = new Map<string, HistoryDialog>();
  constructor(private readonly host: HistoryPanelHost) {}
  async open(): Promise<void> {
    const context = this.host.getDocument();
    if (!context?.nodeId || !context.workspaceId) { this.host.notify("请先打开一个身份已接纳的本地笔记。无法确认身份的文件须先处理，不能按同名猜测历史。", true); return; }
    let dialog = this.dialogs.get(context.sessionId);
    if (!dialog) { dialog = new HistoryDialog(this.host, context); this.dialogs.set(context.sessionId, dialog); }
    await dialog.open();
  }
  dispose(): void { for (const dialog of this.dialogs.values()) dialog.dispose(); this.dialogs.clear(); }
}
