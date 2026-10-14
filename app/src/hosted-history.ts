import "./history-panel.css";
import { action, ui } from "./data-panel";
import { HistoryController, historySourceDiff, historyTimeLabel, type HistoryContext, type HistoryList, type HistoryComparison, type HistoryRestoreBasis, type HistorySource, type SavedHistoryComparison, type DraftHistoryComparison } from "./history-state";
import { hostedCachePrefix } from "./hosted-state";
import { hostedHistoryPending, hostedHistoryStatus, unavailableCommittedHistory, type HostedHistoryContext, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import type { EmbeddedQueryBasis } from "./embedded-query";

type Request = <T>(path: string, body: unknown) => Promise<T>;
export interface HostedHistoryHost {
  context(): HostedHistoryContext | null;
  basis(settle: boolean): Promise<EmbeddedQueryBasis>;
  pendingChanged(): void;
  restored(record: HostedHistoryRecord): Promise<"applied" | "superseded">;
  finishUnavailable(record: HostedHistoryRecord): Promise<void>;
}
const hash = async (value: string): Promise<string> => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value)))].map(byte => byte.toString(16).padStart(2, "0")).join("");
const labels: Record<HostedHistoryStatus["status"], string> = { notRegistered: "原编号未登记，仍需原请求核对或明确结束", notCommitted: "原恢复未提交，草稿保留", committed: "原恢复已新保存", recoveryPending: "原恢复需继续处理，不能再创建恢复", resolved: "原恢复已处理，未冒充新保存", cancelled: "原恢复已明确结束" };
function pane(label: string, source: string) { const section = ui("section", "history-source-pane"), content = ui("pre", "history-source", source); content.tabIndex = 0; content.setAttribute("aria-label", label); section.append(ui("h3", "", label), content); return section; }

/** Original restores survive closing a view; native history remains Core-owned. */
export class HostedHistory {
  private epoch = 0;
  private generation = 0;
  private key = "";
  private records: HostedHistoryRecord[] = [];
  private selectedOperation = "";
  private controller?: HistoryController;
  private note?: { resource: string; sessionId: string; nodeId: string; workspaceId: string };
  private busy = false;
  private shown = false;
  private draftComparison = false;
  private unreadable?: string;
  private dialog?: HTMLDialogElement;
  private message = ui("p", "history-status");
  private content = ui("section");
  private operations = ui("section");
  constructor(private host: HTMLElement, private request: Request, private source: HostedHistoryHost) { this.message.setAttribute("role", "status"); this.clear(false); }
  hasPending(resource?: string) { return this.records.some(record => (!resource || record.request.resource === resource) && hostedHistoryPending(record)); }
  clear(removeCache = true) {
    this.epoch++; this.generation++; this.key = ""; this.records = []; this.selectedOperation = ""; this.controller = undefined; this.note = undefined; this.busy = false; this.shown = false; this.unreadable = undefined; this.dialog?.remove(); this.dialog = undefined; this.content.replaceChildren(); this.operations.replaceChildren(); this.message.textContent = ""; this.host.replaceChildren(); this.host.hidden = true;
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-history:")) sessionStorage.removeItem(key);
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); this.key = hostedCachePrefix("history", deployment, actor, generation) + "restores";
    let raw: string | null = null;
    try { raw = sessionStorage.getItem(this.key); if (raw) { const records = JSON.parse(raw) as HostedHistoryRecord[]; if (!Array.isArray(records) || records.some(record => !record.request?.operationId || !record.request.resource || !record.basis || record.basis.resource !== record.request.resource || typeof record.request.source !== "string" || typeof record.basis.selectedSource !== "string")) throw new Error(); this.records = records.map(record => ({ ...record, status: undefined })); this.selectedOperation = this.records[0]?.request.operationId ?? ""; } } catch { this.unreadable = raw ?? ""; }
    this.mount(); this.render(); const epoch = this.epoch;
    for (const record of [...this.records]) { if (record.acknowledged) continue; try { await this.lookup(record); } catch (error) { if (epoch === this.epoch) this.report(error); } if (epoch !== this.epoch) return; }
    this.source.pendingChanged();
  }
  private mount() { this.host.replaceChildren(ui("h2", "", "托管笔记时间线"), this.message, this.content, this.operations); }
  documentChanged() { const current = this.source.context(); if (this.note && (!current || this.note.sessionId !== current.sessionId || this.note.resource !== current.resource)) { this.generation++; this.controller = undefined; this.note = undefined; this.shown = false; this.dialog?.remove(); this.dialog = undefined; } this.render(); }
  private context(note = this.note): HistoryContext | null { const current = this.source.context(); return current && note && current.resource === note.resource && current.sessionId === note.sessionId ? { ...current, path: `resource:${current.resource}`, nodeId: note.nodeId, workspaceId: note.workspaceId } : null; }
  async open() { await this.perform(async () => {
    const current = this.source.context(); if (!current) throw new Error("请先打开有历史读取资格的正文。");
    const epoch = this.epoch, list = await this.request<HistoryList>("history/list", { sessionId: current.sessionId });
    if (epoch !== this.epoch || this.source.context()?.sessionId !== current.sessionId) return;
    if (!list.nodeId || !list.workspaceId || list.path !== `resource:${current.resource}`) throw new Error("历史身份或对象依据不可用，未按同名笔记替代。");
    const note = { resource: current.resource, sessionId: current.sessionId, nodeId: list.nodeId, workspaceId: list.workspaceId }; this.note = note;
    this.controller = new HistoryController({ current: () => this.context(note), id: () => { throw new Error("托管恢复只由原操作持久层发起。"); }, list: body => this.request("history/list", body), read: body => this.request("history/read", body), compare: body => this.request("history/compare", body), restore: async () => { throw new Error("未调用本地恢复入口。"); }, status: async () => { throw new Error("未调用本地状态入口。"); } }, Object.freeze(this.context(note)!));
    this.controller.list = structuredClone(list); this.shown = true; this.draftComparison = current.dirty; this.message.textContent = "浏览、读取和比较保持只读；保留记录不等于完整历史。";
  }); }
  private persist(records = this.records, beforeEffect = false) { if (!this.key || this.unreadable !== undefined) throw new Error("历史恢复缓存不可用，未发起新的恢复。"); try { sessionStorage.setItem(this.key, JSON.stringify(records)); } catch { throw new Error(beforeEffect ? "浏览器无法保留原恢复编号和准确请求，尚未发送恢复；请先复制草稿。" : "浏览器无法保留恢复状态，请保留本页并查询原编号；未冒充缓存已保存。"); } }
  private report(error: unknown) { this.message.textContent = error instanceof Error ? error.message : String(error); }
  private async perform(run: () => Promise<void>) { if (this.busy) return; const epoch = this.epoch; this.busy = true; this.render(); try { await run(); } catch (error) { if (epoch === this.epoch) this.report(error); } finally { if (epoch === this.epoch) { this.busy = false; this.source.pendingChanged(); this.render(); } } }
  private render() {
    this.host.hidden = !this.shown && !this.records.length && this.unreadable === undefined; this.content.replaceChildren(); this.operations.replaceChildren();
    if (this.unreadable !== undefined) { this.content.append(ui("p", "history-warning", "原恢复缓存无法解析，原记录只读保留；不会替换编号或发起恢复。"), pane("原恢复缓存", this.unreadable)); return; }
    const controller = this.controller, list = controller?.list;
    if (this.shown && list) {
      this.content.append(ui("p", "history-help", `${list.path} · ${list.retention.message}`));
      const refresh = action("刷新保存记录", () => this.perform(async () => { await controller!.refresh(); })); refresh.disabled = this.busy; this.content.append(refresh);
      const versions = ui("ol", "history-versions"); for (const version of list.versions) { const item = ui("li", "history-version"), choose = action(`${historyTimeLabel(version)} · ${version.versionId}`, () => this.perform(async () => { await controller!.select(version.versionId); })); choose.disabled = this.busy || version.status !== "available"; choose.dataset.versionId = version.versionId; choose.setAttribute("aria-pressed", String(controller!.selected?.versionId === version.versionId)); item.append(choose, ui("span", "history-version-meta", `${version.status} · ${version.kind}${version.commitSequence !== null ? ` · 保存顺序 ${version.commitSequence}` : ""}`)); versions.append(item); } this.content.append(versions);
      for (const warning of list.warnings) this.content.append(ui("p", "history-warning", warning));
      if (controller!.selected) { this.drawSource(controller!.selected, controller!.comparison); const select = ui("select"); select.setAttribute("aria-label", "历史比较对象"); for (const [value, label] of [["saved", "当前已保存版本"], ["draft", "当前草稿快照"]]) select.append(Object.assign(ui("option", "", label), { value })); select.value = this.draftComparison ? "draft" : "saved"; select.onchange = () => this.draftComparison = select.value === "draft";
        const compare = action("比较所选历史版本", () => this.compare()), restore = action("恢复这个历史版本", () => this.prepare()); compare.disabled = this.busy; const context = this.source.context(); restore.disabled = this.busy || !context || context.readOnly || context.savePending || context.unresolvedSave || this.hasPending(context.resource); this.content.append(select, compare, restore); }
      for (const [index, draft] of list.protectedDrafts.entries()) { const details = ui("details", "history-draft"); details.append(ui("summary", "", `受保护个人草稿 ${index + 1}${draft.baseKnown ? "" : " · 原基线未知"}`), pane("受保护草稿", draft.source), pane("原草稿基线", draft.baseSource)); this.content.append(details); }
    }
    if (this.records.length) this.drawOperations();
  }
  private drawSource(selected: HistorySource, comparison: HistoryComparison | null) {
    if (!comparison) { this.content.append(pane("所选准确旧版 · 只读源码", selected.source), ui("p", "history-help", selected.previewMessage)); return; }
    const columns = ui("div", "history-comparison"), left = pane("所选准确旧版 · 只读", ""), right = pane(comparison.comparison === "draftSnapshot" ? `固定个人草稿快照 · serial ${comparison.current.serial}` : "固定当前已保存版本", "");
    for (const part of historySourceDiff(comparison.selected.source, comparison.current.source)) { if (part.kind !== "added") left.querySelector("pre")!.append(ui("span", part.kind === "removed" ? "history-removed" : "", part.text)); if (part.kind !== "removed") right.querySelector("pre")!.append(ui("span", part.kind === "added" ? "history-added" : "", part.text)); } columns.append(left, right); this.content.append(columns, ui("p", "history-help", `所选 ${comparison.selected.versionId} · 比较基线 ${comparison.current.baseId}；后续输入或保存不会改变这次固定比较，也未改正文。`));
  }
  private async compare() { await this.perform(async () => {
    if (!this.controller?.selected) return; if (!this.draftComparison) { await this.controller.compareSaved(); return; }
    const epoch = this.epoch, controller = this.controller, selected = controller.selected!, basis = await this.source.basis(false), sourceHash = await hash(basis.source);
    const result = await this.request<DraftHistoryComparison & { current: DraftHistoryComparison["current"] & { sourceHash: string } }>("history/compare-draft", { ...basis, sourceHash, versionId: selected.versionId });
    if (epoch !== this.epoch || controller !== this.controller || controller.selected?.versionId !== selected.versionId) return;
    if (result.comparison !== "draftSnapshot" || result.selected.versionId !== selected.versionId || result.selected.source !== selected.source || result.current.source !== basis.source || result.current.serial !== basis.serial || result.current.sourceHash !== sourceHash || result.current.path !== `resource:${this.note!.resource}`) throw new Error("草稿比较与本次准确快照不一致，未替换比较内容。");
    controller.comparison = structuredClone(result);
  }); }
  private async prepare() { await this.perform(async () => {
    if (!this.controller || !this.note || this.hasPending(this.note.resource)) throw new Error("请先处理原恢复操作。");
    await this.source.basis(true); const epoch = this.epoch, controller = this.controller, review = await controller.prepareRestore(); if (epoch !== this.epoch || controller !== this.controller) return;
    const dialog = ui("dialog", "history-restore"); dialog.setAttribute("aria-label", "托管历史恢复确认"); this.dialog = dialog;
    const protect = ui("input"); protect.type = "checkbox"; protect.setAttribute("aria-label", "保护当前草稿并恢复历史"); const label = ui("label", "history-protection", "保留当前及已有个人恢复草稿，再将旧版恢复为一次新保存"); label.prepend(protect);
    const confirm = action("确认历史恢复并新保存", () => { void this.perform(async () => { if (!protect.checked) return; const current = this.context(); if (!current || current.sessionId !== review.basis.sessionId || current.baseId !== review.basis.baseId || current.source !== review.basis.source || current.serial !== review.basis.serial || current.readOnly || current.savePending || current.unresolvedSave) throw new Error("确认前草稿或资格已变化，请重新准备；输入保留。");
      const sourceHash = await hash(review.basis.source); if (epoch !== this.epoch || this.context()?.serial !== review.basis.serial || this.context()?.source !== review.basis.source) return;
      const resource = this.note!.resource, operationId = crypto.randomUUID(), record: HostedHistoryRecord = { basis: { ...structuredClone(review.basis), resource }, request: { resource, sessionId: review.basis.sessionId, baseId: review.basis.baseId, versionId: review.basis.selectedVersionId, operationId, draftAction: "retain", source: review.basis.source, serial: review.basis.serial, sourceHash } };
      const records = [...this.records, record]; this.persist(records, true); this.records = records; this.selectedOperation = operationId; this.source.pendingChanged(); dialog.close(); this.dialog = undefined; await this.send(record);
    }); }); confirm.disabled = true; protect.onchange = () => confirm.disabled = !protect.checked; const cancel = action("取消历史恢复", () => { controller.cancelRestore(); dialog.close(); });
    dialog.append(ui("h2", "", "恢复为一次新保存"), ui("p", "history-help", "旧版会产生新的保存记录；后来的版本及个人草稿继续保留。请求期间的新输入会继续留在同一编辑器。"), pane("将新保存的准确旧版", review.basis.selectedSource), pane("当前已保存基线", review.basis.baseSource), label, cancel, confirm); dialog.onclose = () => { dialog.remove(); if (this.dialog === dialog) this.dialog = undefined; }; document.body.append(dialog); dialog.showModal();
  }); }
  private drawOperations() {
    this.operations.append(ui("h3", "", "原历史恢复操作")); const select = ui("select"); select.setAttribute("aria-label", "原历史恢复操作"); for (const record of this.records) select.append(Object.assign(ui("option", "", `${record.request.resource} · ${record.request.operationId}`), { value: record.request.operationId })); select.value = this.selectedOperation; select.onchange = () => { this.selectedOperation = select.value; this.render(); }; this.operations.append(select);
    const clear = action("清除本标签页已核对的恢复记录", () => this.perform(async () => { const retained = this.records.filter(record => !record.acknowledged && !["cancelled", "resolved"].includes(record.status?.status ?? "")); this.persist(retained); this.records = retained; if (!retained.some(record => record.request.operationId === this.selectedOperation)) this.selectedOperation = retained[0]?.request.operationId ?? ""; this.message.textContent = "已清除本标签页的终态恢复缓存；Core 历史和受保护个人草稿继续保留，未知操作未清除。"; })); clear.disabled = this.busy || !this.records.some(record => record.acknowledged || ["cancelled", "resolved"].includes(record.status?.status ?? "")); this.operations.append(clear);
    const record = this.records.find(item => item.request.operationId === this.selectedOperation); if (!record) return; this.operations.append(ui("p", "history-help", record.status ? `${labels[record.status.status]}${record.status.resultBasis && record.status.resultBasis !== "current" ? " · 原保存已非当前，只能比較" : ""}` : "结果待核对；原编号和准确请求保持不变。"));
    const check = action("查询原历史恢复", () => this.perform(() => this.lookup(record))), retry = action("按原编号重试历史恢复", () => this.perform(() => this.send(record))), settle = action("明确结束原历史恢复", () => this.perform(async () => { const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/settle", { resource: record.request.resource, operationId: record.request.operationId }); if (epoch !== this.epoch) return; this.install(record, incoming); })), apply = action("核对并应用原恢复回执", () => this.perform(() => this.acknowledge(record)));
    for (const button of [check, retry, settle, apply]) button.disabled = this.busy; retry.disabled ||= !!record.status && !["notRegistered", "notCommitted"].includes(record.status.status); settle.disabled ||= !!record.status && !["notRegistered", "notCommitted"].includes(record.status.status); apply.disabled ||= record.acknowledged === true || record.status?.status !== "committed" || !record.status.result || this.source.context()?.resource !== record.request.resource; this.operations.append(check, retry, settle, apply);
    if (unavailableCommittedHistory(record)) { const finish = action("结束本页回执应用，重新核对当前稿", () => this.perform(async () => { const epoch = this.epoch; if (!unavailableCommittedHistory(record)) throw new Error("只有已提交但原结果不可交付的操作可结束本页应用。"); await this.source.finishUnavailable(record); if (epoch !== this.epoch) return; record.acknowledged = true; this.persist(); this.source.pendingChanged(); this.message.textContent = "原保存事实及原稿已保留；本页回执应用已结束，当前稿须明确比较，未把旧恢复结果冒作当前稿。"; })); finish.disabled = this.busy || this.source.context()?.resource !== record.request.resource; this.operations.append(finish); }
    if (record.status?.status === "committed" && !record.status.result) this.operations.append(ui("p", "history-warning", "原恢复已保存，但当前资格无法交付原结果。只保留原操作事实，不用旧缓存正文或另发恢复。"));
    if (record.status?.result) { const details = ui("details"); details.append(ui("summary", "", "原恢复结果与受保护草稿 · 只读"), pane("原恢复已保存源码", record.status.result.source)); for (const draft of record.status.result.protectedDrafts) details.append(pane("原受保护草稿", draft.source), pane("该草稿原保存基线", draft.baseSource)); this.operations.append(details); }
  }
  private install(record: HostedHistoryRecord, incoming: HostedHistoryStatus) { record.status = hostedHistoryStatus(record, incoming); this.persist(); this.source.pendingChanged(); this.render(); }
  private async lookup(record: HostedHistoryRecord) { const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId }); if (epoch !== this.epoch) return; this.install(record, incoming); }
  private async send(record: HostedHistoryRecord) { this.persist(undefined, true); const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/restore", record.request); if (epoch !== this.epoch) return; this.install(record, incoming); if (incoming.status === "committed" && incoming.result && this.source.context()?.resource === record.request.resource) await this.acknowledge(record); }
  private async acknowledge(record: HostedHistoryRecord) { const epoch = this.epoch; if (record.acknowledged || record.status?.status !== "committed" || !record.status.result) return; const outcome = await this.source.restored(record); if (epoch !== this.epoch) return; record.acknowledged = true; this.persist(); this.source.pendingChanged(); this.message.textContent = outcome === "applied" ? "旧版已作为新版本保存；个人新输入与既有草稿、后续历史均保留。" : "原恢复已保存，但之后另有保存稿；当前稿与个人输入均保留，请比较核对，未用旧回执覆盖。"; }
}
