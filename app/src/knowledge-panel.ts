import "./knowledge-view.css";
import { ProofreadingPanel, type ProofreadOptions, type ProofreadResult } from "./proofreading-panel";
import { canApplySuggestion, isGraphemeRange, KnowledgeReadGate, knowledgeLossMessage, layoutMentions, nodeIdentity, retainPendingKnowledgePlan, sameKnowledgeBasis, uncertainKnowledgeOperation, validateMediaRegion } from "./knowledge-state";
import type { AnnotationResolution, KnowledgeBasis, KnowledgeLoss, KnowledgeMention, KnowledgeNodeRef, MediaRegionInput } from "./knowledge-state";
import type { Origin, RenderNode } from "./types";
import { storedMediaSelection } from "./media-selection";
import { trashRestoreDraft, type LibraryTrashEntry } from "./library-trash";

export interface KnowledgeContext extends KnowledgeBasis {
  root: string;
  nodeId?: string;
  workspaceId?: string;
  dirty: boolean;
  selection?: { from: number; to: number; visibleExcerpt?: string; elementRole?: string; nodeKey?: string };
}
export interface KnowledgeLink {
  source: KnowledgeNodeRef;
  occurrence: { from: number; to: number; utf16From: number; utf16To: number; target: string; kind: string; raw?: string; destinationSlot?: unknown };
  resolution: { status: string; target?: KnowledgeNodeRef; fragment?: string; form?: string };
}
export interface KnowledgeAnnotation {
  id: string;
  annotationId?: string;
  body: string;
  purpose?: string;
  reviewState?: "open" | "resolved";
  suggestionState?: "pending" | "accepted" | "rejected";
  replacement?: string;
  visibleExcerpt?: string | { text: string };
  targetResolution?: AnnotationResolution;
  targetSummary?: string;
  sourceLocation?: { from: number; to: number };
  navigateTarget?: string;
  resourceLocation?: { resource: string; revision: string; target: unknown };
  author?: string;
  createdAt?: string;
  verified?: boolean;
  replies?: { id?: string; body: string; author?: string; createdAt?: string }[];
  target?: unknown;
}
export interface KnowledgeTemplate {
  id: string;
  title: string;
  description?: string;
  source?: string;
  sourceNode?: KnowledgeNodeRef;
  sourceRevision?: string;
  available?: boolean;
  parameters?: { name: string; title?: string; type?: string; kind?: "markup" | "string" | "bool" | "int"; required?: boolean; nullable?: boolean; default?: string | boolean | null }[];
}
export type KnowledgeTemplateValue = string | boolean | null;
export interface KnowledgeSnapshot {
  links?: { generation: string | number; coverage: "complete" | "partial"; outgoing: KnowledgeLink[]; incomingId: KnowledgeLink[]; incomingPath: KnowledgeLink[]; warnings?: string[] };
  renderedLinks?: { compileIdentity: string; target: string; projectionGeneration: string; occurrenceOrdinal: number; fragments: string[]; destination: string; origin: string }[];
  propertyReferences?: { coverage: "complete" | "partial"; items: { source: KnowledgeNodeRef; fieldId: string; fieldTitle?: string; itemPath: string[]; resolution: KnowledgeLink["resolution"] }[]; diagnostics?: string[] };
  mentions?: { generation: string | number; coverage: "complete" | "partial"; fragments: { id: string; text?: string; nodeKey?: string; coverage?: string; hits: KnowledgeMention[] }[] };
  annotations?: KnowledgeAnnotation[];
  annotationStatus?: string;
  annotationDiagnostics?: string[];
  templates?: KnowledgeTemplate[];
  trash?: LibraryTrashEntry[];
  warnings?: string[];
}
export interface KnowledgePlan {
  operationId: string;
  planId: string;
  kind?: string;
  status?: string;
  changes?: { path?: string; action?: string; bytes?: number; before?: string; after?: string }[];
  preview?: unknown;
  losses?: (string | KnowledgeLoss)[];
  [key: string]: unknown;
}
export interface KnowledgeReceipt { status: string; message?: string; operationId?: string; path?: string; [key: string]: unknown }
export interface KnowledgeIntent { kind: string; [key: string]: unknown }
export interface KnowledgeTextMapping { nodeKey: string; node: Text | HTMLElement; text: string; origin?: Origin | null }
export interface KnowledgeHost {
  proofread?(context: KnowledgeContext, options: ProofreadOptions): Promise<ProofreadResult>;
  context(): KnowledgeContext | null;
  load(context: KnowledgeContext): Promise<KnowledgeSnapshot>;
  /** A fresh preparation must qualify the current Draft and freeze every effect. */
  prepare(intent: KnowledgeIntent, context: KnowledgeContext): Promise<KnowledgePlan>;
  commit(plan: KnowledgePlan, context: KnowledgeContext): Promise<KnowledgeReceipt>;
  navigate(target: string | KnowledgeNodeRef): void | Promise<void>;
  notify(message: string, error?: boolean): void;
  inspectOperation?(operationId: string, context: KnowledgeContext): Promise<KnowledgeReceipt>;
  previewTemplate?(id: string, parameters: Record<string, KnowledgeTemplateValue>, context: KnowledgeContext): Promise<{ tree?: RenderNode | null; source?: string; diagnostics?: { message: string }[] }>;
  linkCopy?(link: KnowledgeLink, form: string, context: KnowledgeContext): Promise<{ address: string; fullLink: string }>;
  mountMedia?(container: HTMLElement, selected: (target: MediaRegionInput | null) => void, initial?: MediaRegionInput): { dispose(): void };
  selectSource?(from: number, to: number): void;
  copy?(text: string): Promise<void>;
  beforeAction?(): Promise<void>;
  canEditSource?(): boolean;
}
export type KnowledgePanelTab = "links" | "mentions" | "annotations" | "proofreading" | "files" | "templates";
export interface KnowledgePanel {
  refresh(): Promise<void>;
  selectTab(tab: KnowledgePanelTab): void;
  attachSurface(surface: HTMLElement | null, mappings?: KnowledgeTextMapping[]): void;
  dispose(): void;
}

const resolutionLabels: Record<string, string> = { exact: "准确定位", mapped: "已映射", stale: "待重新定位", ambiguous: "定位有歧义", orphaned: "目标已失效", unavailable: "目标不可用", resolved: "目标有效", missing: "目标缺失", fragmentMissing: "段落标签缺失", invalid: "地址无效", external: "外部链接" };
const formLabels: Record<string, string> = { id: "稳定 ID", relative: "源相对路径（最短有效）", workspace: "库内路径", file: "本机绝对路径" };
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: string): HTMLElementTagNameMap[K] {
  const value = document.createElement(tag); value.className = className; if (text !== undefined) value.textContent = text; return value;
}
function action(label: string, run: () => unknown | Promise<unknown>, className = "knowledge-button"): HTMLButtonElement {
  const value = element("button", className, label); value.type = "button";
  value.onclick = () => { void Promise.resolve(run()).catch(() => { /* Caller reports and retains its draft. */ }); }; return value;
}
function field(label: string, control: HTMLElement): HTMLLabelElement { const value = element("label", "knowledge-field"); value.append(element("span", "", label), control); return value; }
function input(value = "", placeholder = ""): HTMLInputElement { const control = element("input", "knowledge-input"); control.value = value; control.placeholder = placeholder; return control; }
function textarea(value = "", placeholder = ""): HTMLTextAreaElement { const control = element("textarea", "knowledge-input"); control.value = value; control.placeholder = placeholder; control.rows = 3; return control; }
function select(options: [string, string][]): HTMLSelectElement { const control = element("select", "knowledge-input"); for (const [id, label] of options) { const option = element("option", "", label); option.value = id; control.append(option); } return control; }
function nodeLabel(ref: KnowledgeNodeRef): string { return ref.title || ref.path?.replaceAll("\\", "/").split("/").at(-1) || "无标题笔记"; }
function excerpt(annotation: KnowledgeAnnotation): string { return typeof annotation.visibleExcerpt === "string" ? annotation.visibleExcerpt : annotation.visibleExcerpt?.text ?? ""; }
function nativeTree(node: RenderNode): Node {
  if (node.kind === "text") return document.createTextNode(node.text ?? "");
  if (node.kind === "tag") return document.createDocumentFragment();
  const allowed = new Set(["p", "div", "span", "strong", "em", "s", "blockquote", "ul", "ol", "li", "table", "thead", "tbody", "tr", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "br"]);
  const value = element(allowed.has(node.tag ?? "") ? node.tag as "div" : "div");
  if (node.kind === "frame") value.textContent = "此布局需要分页预览";
  else { for (const child of node.children ?? []) value.append(nativeTree(child)); if (node.text) value.append(document.createTextNode(node.text)); }
  return value;
}

/** Mount beside the shared editor. No independent source, history or persistence store. */
export function mountKnowledgePanel(container: HTMLElement, host: KnowledgeHost): KnowledgePanel {
  container.classList.add("knowledge-panel"); container.setAttribute("aria-label", "知识与批注");
  const gate = new KnowledgeReadGate();
  const proofreading = new ProofreadingPanel({ context: host.context, scan: (context, options) => host.proofread ? host.proofread(context, options) : Promise.reject(new Error("校对接口不可用")), perform, selectSource: host.selectSource, beforeAction: host.beforeAction, notify: message => host.notify(message, true) });
  let snapshot: KnowledgeSnapshot | null = null;
  let snapshotBasis: KnowledgeContext | null = null;
  let tab = "links";
  let enabled = localStorage.getItem("weftext-potential-links") !== "false";
  let surface: HTMLElement | null = null;
  let mappings: KnowledgeTextMapping[] = [];
  let overlay: HTMLElement | null = null;
  let popover: HTMLElement | null = null;
  let popoverAnchor: HTMLElement | null = null;
  let pendingPlan: { plan: KnowledgePlan; context: KnowledgeContext } | null = null;
  let disposed = false, busy = false;
  let annotationBody = "", suggestionReplacement = "", annotationKind = "comment", annotationTarget = "whole_document";
  let annotationSelection: KnowledgeContext["selection"];
  let annotationSelectionBasis: KnowledgeContext | null = null;
  const replies = new Map<string, string>();
  const reconfirmations = new Map<string, { replacement: string; basis: KnowledgeContext | null; selection?: KnowledgeContext["selection"] }>();
  const fileDrafts = new Map<string, { name: string; parent: string; backup: string; allowLosses: boolean; restores: Map<string, { name: string; parent: string }> }>();
  const templateDrafts = new Map<string, { name: string; parent: string; parameters: Record<string, KnowledgeTemplateValue>; presence: Record<string, "omitted" | "value" | "none"> }>();
  type RegistrationSlot = { name: string; kind: "markup" | "string" | "bool" | "int"; from: number; to: number; nullable?: boolean; default?: KnowledgeTemplateValue };
  const registrationDrafts = new Map<string, { title: string; description: string; templateId?: string; basis: KnowledgeContext | null; parameters: RegistrationSlot[]; open: boolean; slotsOpen: boolean; slot: { name: string; kind: RegistrationSlot["kind"]; nullable: boolean; defaultMode: "omitted" | "value" | "none"; defaultValue: KnowledgeTemplateValue } }>();
  let renderedBasis: KnowledgeContext | null = null;
  let deferredRender = false;
  let mediaDraft: MediaRegionInput | null = null;
  const mediaContainer = element("div", "knowledge-media-viewer");
  let mediaController: { dispose(): void } | null = null;
  let mediaSession: string | null = null;
  const status = element("div", "knowledge-status"); status.setAttribute("aria-live", "polite");
  const onBlur = (): void => { if (deferredRender) queueMicrotask(() => { if (!disposed && !(container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select"))) { deferredRender = false; render(); } }); };
  container.addEventListener("focusout", onBlur);

  function report(error: unknown): void { const message = error instanceof Error ? error.message : String(error); status.textContent = message; host.notify(message, true); }
  async function copy(text: string): Promise<void> { try { if (host.copy) await host.copy(text); else await navigator.clipboard.writeText(text); host.notify("已复制。"); } catch (error) { report(error); } }
  function navigate(target: string | KnowledgeNodeRef): void { void Promise.resolve(host.navigate(target)).catch(report); }
  function section(title: string, description?: string): HTMLElement { const value = element("section", "knowledge-section"); value.append(element("h3", "", title)); if (description) value.append(element("p", "knowledge-help", description)); return value; }
  function closePopover(restoreFocus = false): void { popover?.remove(); popover = null; const previous = popoverAnchor; popoverAnchor = null; if (restoreFocus && previous?.isConnected) previous.focus(); }
  function clearOverlay(): void { closePopover(); overlay?.remove(); overlay = null; }
  function knownMentions(): { fragment: NonNullable<KnowledgeSnapshot["mentions"]>["fragments"][number]; hit: KnowledgeMention }[] {
    return snapshot?.mentions?.fragments.flatMap(fragment => layoutMentions(fragment.hits).map(hit => ({ fragment, hit }))) ?? [];
  }
  function mentionIntent(fragmentId: string, hit: KnowledgeMention, target: KnowledgeNodeRef): KnowledgeIntent {
    return { kind: "mentionConvert", generation: snapshot?.mentions?.generation, fragmentId, startScalar: hit.startScalar, endScalar: hit.endScalar, target };
  }
  async function confirmPlan(title: string, plan: KnowledgePlan): Promise<boolean> {
    const dialog = element("dialog", "knowledge-dialog"); dialog.append(element("h2", "", title), element("p", "knowledge-help", "核对本次完整变动。确认只应用这份准备结果。"));
    const preview = element("div", "knowledge-plan-preview");
    const verbs: Record<string, string> = { create: "新建", write: "写入", update: "更新", replace: "替换", copy: "复制", move: "移动", rename: "改名", trash: "送到回收站", restore: "恢复", delete: "移除原位置", remove: "移除原位置", relocateFrom: "移出原位置", relocateTo: "移到新位置", backup: "备份到", import: "写入导入内容", style: "更新库样式" };
    if (plan.kind === "trashNode") preview.append(element("p", "knowledge-warning", "笔记节点将进入资料库回收站，可通过恢复入口还原。"));
    if (plan.kind === "copyNode" || plan.kind === "backupLibrary") preview.append(element("p", "knowledge-help", "创建独立副本；下列列出本次复制的准确目标。"));
    if (plan.draftOnly === true) preview.append(element("p", "knowledge-help", "本次修改当前共享草稿，可用一次撤销还原；保存时才写入正文文件。"));
    const fileChanges = plan.changes?.filter(change => Boolean(change.path)) ?? [];
    if (fileChanges.length) {
      preview.append(element("h3", "", `本次涉及 ${fileChanges.length} 个文件变化`));
      const paths = element("ul", "knowledge-change-list");
      for (const change of fileChanges) { const item = element("li"); item.append(element("strong", "", verbs[change.action ?? ""] ?? "变更"), element("span", "knowledge-address", change.path)); if (change.bytes !== undefined) item.append(element("small", "knowledge-help", `${change.bytes.toLocaleString()} 字节`)); paths.append(item); } preview.append(paths);
    }
    const currentSource = host.context()?.source;
    const edit = plan.edit as { from?: number; to?: number; text?: string } | undefined;
    if (currentSource !== undefined && edit && typeof edit.from === "number" && typeof edit.to === "number" && typeof edit.text === "string") {
      preview.append(element("h3", "", "所选范围的变化"));
      const pair = element("div", "knowledge-before-after");
      const before = element("div"); before.append(element("strong", "", "原内容"), element("pre", "knowledge-replacement", currentSource.slice(edit.from, edit.to)));
      const after = element("div"); after.append(element("strong", "", "应用后"), element("pre", "knowledge-replacement", edit.text)); pair.append(before, after); preview.append(pair);
    }
    const detail = plan.preview;
    if (typeof detail === "string") preview.append(element("p", "knowledge-annotation-body", detail));
    else if (detail && typeof detail === "object") {
      const previewRecord = detail as Record<string, unknown>;
      const asRecord = (value: unknown): Record<string, unknown> => value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {};
      const stateName = (value: unknown): string => typeof value === "string" ? ({ open: "未解决", resolved: "已解决", pending: "待接受", accepted: "已接受", rejected: "已拒绝" } as Record<string, string>)[value] ?? value : "";
      const targetSummary = (value: unknown): string => {
        const target = asRecord(value), anchor = asRecord(target.anchor), range = asRecord(anchor.sourceRange);
        const kind = String(target.kind ?? "");
        if (["sourceRange", "source_range", "sourceElement", "source_element"].includes(kind)) return `${kind === "sourceElement" || kind === "source_element" ? "源元素" : "源范围"} UTF-16 [${range.utf16From ?? target.from ?? "?"}, ${range.utf16To ?? target.to ?? "?"})${typeof anchor.visibleExcerpt === "string" ? `：${anchor.visibleExcerpt}` : ""}`;
        if (["wholeDocument", "whole_document"].includes(kind)) return "整篇笔记";
        if (kind === "pdfRegion" || kind === "pdf_region") { const rect = asRecord(target.rect); return `PDF 第 ${Number(target.pageIndex ?? 0) + 1} 页，矩形 (${rect.x ?? target.x}, ${rect.y ?? target.y}, ${rect.width ?? target.rectWidth}, ${rect.height ?? target.rectHeight}) pt`; }
        if (kind === "imageRegion" || kind === "image_region") { const rect = asRecord(target.rect); return `图片矩形 (${rect.x ?? target.x}, ${rect.y ?? target.y}, ${rect.width ?? target.rectWidth}, ${rect.height ?? target.rectHeight}) 像素`; }
        if (kind === "mediaInterval" || kind === "media_interval") { const timebase = asRecord(target.timebase); return `媒体区间 [${target.startTick}, ${target.endTick}) ticks；每秒 ${timebase.numerator ?? target.timebase}/${timebase.denominator ?? target.timebaseDenominator ?? 1} ticks`; }
        return "准确目标由本次准备结果固定";
      };
      const pair = (title: string, before: string, after: string): void => {
        const group = section(title), columns = element("div", "knowledge-before-after");
        for (const [label, text] of [["修改前", before], ["修改后", after]]) { const column = element("div"); column.append(element("strong", "", label), element("pre", "knowledge-replacement", text)); columns.append(column); }
        group.append(columns); preview.append(group);
      };
      const annotationText = (record: Record<string, unknown>): string => {
        const lines: string[] = [];
        if (typeof record.body === "string") lines.push(`讨论：${record.body}`);
        if (typeof record.reviewState === "string") lines.push(`讨论状态：${stateName(record.reviewState)}`);
        if (record.target) lines.push(`目标：${targetSummary(record.target)}`);
        const suggestion = asRecord(record.suggestion);
        if (typeof suggestion.replacement === "string") lines.push(`建议文字：${suggestion.replacement}`);
        if (typeof suggestion.state === "string") lines.push(`建议状态：${stateName(suggestion.state)}`);
        return lines.join("\n");
      };
      const annotationRecords = (source: unknown): Record<string, unknown>[] | null => {
        if (typeof source !== "string") return null;
        try { const value = JSON.parse(source); return Array.isArray(value?.annotations) ? value.annotations.map(asRecord) : null; } catch { return null; }
      };
      const templateText = (value: unknown): string => {
        const template = asRecord(value); if (!Object.keys(template).length) return "尚未登记";
        const lines = [`模板名称：${typeof template.title === "string" ? template.title : ""}`];
        if (typeof template.description === "string" && template.description) lines.push(`用途：${template.description}`);
        const sourceNode = asRecord(template.sourceNode); const context = host.context();
        if (typeof sourceNode.path === "string") lines.push(`来源：${sourceNode.path}`);
        else if (context && sourceNode.nodeId === context.nodeId && sourceNode.workspaceId === context.workspaceId) lines.push(`来源：${context.path}`);
        const slots = Array.isArray(template.parameters) ? template.parameters : Array.isArray(template.slots) ? template.slots : [];
        if (!slots.length) lines.push("参数：整篇模板，无参数范围");
        for (const entry of slots) { const slot = asRecord(entry), range = asRecord(slot.range); const kind = String(slot.kind ?? "markup"); const defaults = !("default" in slot) ? "必填" : slot.default === null ? "默认空值" : typeof slot.default === "boolean" ? `默认${slot.default ? "是" : "否"}` : typeof slot.default === "string" ? `默认：${slot.default === "" ? "空字符串" : slot.default}` : "默认内容见准确准备结果"; lines.push(`参数 ${slot.name}：${({ markup: "正文文字", string: "字符串", bool: "布尔值", int: "整数" } as Record<string, string>)[kind] ?? kind}，${defaults}${range.utf16From !== undefined ? `；UTF-16 [${range.utf16From}, ${range.utf16To})` : ""}`); }
        return lines.join("\n");
      };
      if (previewRecord.templateAfter) pair("模板登记变化", templateText(previewRecord.templateBefore), templateText(previewRecord.templateAfter));
      if (Array.isArray(previewRecord.files)) for (const entry of previewRecord.files) {
        const file = asRecord(entry); if (file.action === "readDependency") continue;
        const path = typeof file.path === "string" ? file.path : "文件内容";
        if (previewRecord.templateAfter && path.replaceAll("\\", "/").endsWith("/.weftext-settings.json")) continue;
        const before = typeof file.before === "string" ? file.before : "", after = typeof file.after === "string" ? file.after : "";
        const annotationsAfter = annotationRecords(file.after), annotationsBefore = annotationRecords(file.before) ?? [];
        if (annotationsAfter) {
          const previous = new Map(annotationsBefore.map(record => [record.annotationId, record]));
          for (const record of annotationsAfter) { const old = previous.get(record.annotationId); if (JSON.stringify(old) !== JSON.stringify(record)) pair("批注变化", old ? annotationText(old) : "新批注", annotationText(record)); }
        } else if (before || after) pair(path, before, after);
      }
      const source = asRecord(previewRecord.source); if (typeof source.before === "string" && typeof source.after === "string" && !Array.isArray(previewRecord.files)) pair("正文变化", source.before, source.after);
      const suggestion = asRecord(previewRecord.suggestion); if (typeof suggestion.before === "string" && typeof suggestion.after === "string") pair("建议状态", stateName(suggestion.before), stateName(suggestion.after));
      if (previewRecord.reconfirmation === true) preview.append(element("p", "knowledge-warning", "本次重新确认建议的准确目标和替换文字，建议仍保持待接受。"));
      if (previewRecord.targetAfter) preview.append(element("p", "knowledge-annotation-body", `新目标：${targetSummary(previewRecord.targetAfter)}`));
      if (typeof previewRecord.replacementAfter === "string") preview.append(element("pre", "knowledge-replacement", previewRecord.replacementAfter));
      const labels: Record<string, string> = { beforeSource: "源修改前", sourceBefore: "源修改前", afterSource: "源修改后", sourceAfter: "源修改后", annotationBefore: "批注修改前", annotationAfter: "批注修改后", annotationsBefore: "讨论修改前", annotationsAfter: "讨论修改后", before: "修改前", after: "修改后", summary: "变化说明", message: "说明", target: "目标", excerpt: "所选文字", replacement: "建议替换文字", address: "目标地址", fullLink: "完整原生链接", reviewState: "讨论状态", suggestionState: "建议状态" };
      for (const [key, value] of Object.entries(detail)) {
        if (!labels[key]) continue;
        const group = section(labels[key]);
        if (typeof value === "string") group.append(element("pre", "knowledge-replacement", value));
        else if (value && typeof value === "object") {
          const record = value as Record<string, unknown>;
          for (const [fieldName, label] of [["body", "批注内容"], ["visibleExcerpt", "已存摘录"], ["replacement", "建议文字"], ["reviewState", "讨论状态"], ["suggestionState", "建议状态"]]) {
            const fieldValue = record[fieldName]; if (typeof fieldValue === "string") group.append(element("p", "knowledge-annotation-body", `${label}：${({ open: "未解决", resolved: "已解决", pending: "待接受", accepted: "已接受", rejected: "已拒绝" } as Record<string, string>)[fieldValue] ?? fieldValue}`));
          }
        }
        if (group.children.length > 1) preview.append(group);
      }
    }
    if (plan.losses?.length) { const losses = section("本次损失与不能处理的内容"); for (const loss of plan.losses) losses.append(element("p", "knowledge-warning", knowledgeLossMessage(loss))); preview.append(losses); }
    if (!preview.children.length) preview.append(element("p", "knowledge-help", "准备结果已固定。本次确认应用上方选定的操作。"));
    dialog.append(preview);
    const diagnostics = element("details", "knowledge-diagnostics"); diagnostics.append(element("summary", "", "查看准备结果详情")); const raw = element("pre", "knowledge-plan", JSON.stringify(plan, null, 2)); diagnostics.append(raw); dialog.append(diagnostics);
    const actions = element("div", "knowledge-actions"); dialog.append(actions); document.body.append(dialog);
    return new Promise(resolve => {
      const finish = (value: boolean): void => { dialog.close(); dialog.remove(); resolve(value); };
      actions.append(action("取消", () => finish(false)), action("确认应用", () => finish(true), "knowledge-button primary"));
      dialog.addEventListener("cancel", event => { event.preventDefault(); finish(false); }); dialog.showModal();
    });
  }
  async function perform(title: string, intent: KnowledgeIntent): Promise<boolean> {
    if (busy) return false;
    if (pendingPlan) { host.notify("上次操作结果仍待确认，请先检查原操作。", true); return false; }
    busy = true; status.textContent = "正在准备准确变动…";
    try {
      const actionBasis = host.context();
      await host.beforeAction?.();
      const context = host.context(); if (!context) throw new Error("请先打开当前笔记。");
      if (["mentionConvert", "linkConvert"].includes(intent.kind) && host.canEditSource?.() === false) throw new Error("请先切换到编辑视图，再转换链接。阅读模式只浏览和复制内容。");
      if (!sameKnowledgeBasis(actionBasis, context)) throw new Error("当前输入已进入共享草稿，请核对更新后的目标并重新操作。输入已保留。");
      const plan = await host.prepare(intent, context);
      if (disposed) return false;
      if (!sameKnowledgeBasis(context, host.context())) throw new Error("准备期间草稿或当前笔记已变化，请重新操作。输入已保留。");
      if (!await confirmPlan(title, plan)) { status.textContent = "已取消，输入保留。"; return false; }
      if (disposed) return false;
      if (!sameKnowledgeBasis(context, host.context())) throw new Error("确认时草稿已变化，本次准备结果未应用。请重新准备。");
      pendingPlan = { plan, context };
      const receipt = await host.commit(plan, context);
      if (uncertainKnowledgeOperation(receipt.status)) { status.textContent = "操作结果尚未确认，保留原操作以供检查。"; render(); return false; }
      pendingPlan = null;
      if (!["committed", "saved", "complete", "success"].includes(receipt.status)) throw new Error(receipt.message || `操作未完成：${receipt.status}`);
      status.textContent = receipt.message || "操作已完成。"; host.notify(status.textContent); await refresh(); return true;
    } catch (error) {
      if (pendingPlan && !retainPendingKnowledgePlan(pendingPlan.plan.draftOnly === true, error)) pendingPlan = null;
      report(error); render(); return false;
    } finally { busy = false; }
  }
  async function inspectPending(): Promise<void> {
    if (!pendingPlan || !host.inspectOperation) return;
    try {
      const receipt = await host.inspectOperation(pendingPlan.plan.operationId, pendingPlan.context);
      if (["committed", "saved", "complete", "success", "conflict", "failed", "rejected"].includes(receipt.status)) { pendingPlan = null; await refresh(); }
      status.textContent = receipt.message || `原操作状态：${receipt.status}`;
    } catch (error) { report(error); }
    render();
  }

  function chooser(fragmentId: string, hit: KnowledgeMention, text: string): HTMLElement {
    const value = element("div", "knowledge-chooser"); value.append(element("strong", "", text || hit.terms.join(" / ")));
    let selected: KnowledgeNodeRef | null = null;
    const convert = action("转换为原生链接", async () => { if (selected) await perform("转换潜在链接", mentionIntent(fragmentId, hit, selected)); }, "knowledge-button primary"); convert.disabled = true;
    const group = element("div", "knowledge-targets"); group.setAttribute("role", "radiogroup"); group.setAttribute("aria-label", "选择准确链接目标");
    const name = `mention-${crypto.randomUUID()}`;
    for (const target of hit.targets) {
      const radio = input(); radio.type = "radio"; radio.name = name; radio.value = nodeIdentity(target);
      const label = element("label", "knowledge-target"); label.append(radio, element("span", "", nodeLabel(target)));
      if (target.path) label.append(element("small", "", target.path));
      radio.onchange = () => { selected = target; convert.disabled = !hit.writable || host.canEditSource?.() === false || !sameKnowledgeBasis(snapshotBasis, host.context()); }; group.append(label);
    }
    value.append(group);
    if (!hit.writable) value.append(element("p", "knowledge-help", "只读发现：当前来源没有准确可写范围。"));
    else if (host.canEditSource?.() === false) value.append(element("p", "knowledge-help", "阅读模式可查看候选；切换到编辑视图后可转换链接。"));
    value.append(convert); return value;
  }
  function showMentionPopover(anchor: HTMLElement, fragmentId: string, hit: KnowledgeMention, text: string): void {
    if (popover?.dataset.mention === `${fragmentId}:${hit.startScalar}:${hit.endScalar}`) return;
    closePopover(); popoverAnchor = anchor; popover = element("div", "knowledge-popover"); popover.dataset.mention = `${fragmentId}:${hit.startScalar}:${hit.endScalar}`; popover.setAttribute("role", "dialog"); popover.setAttribute("aria-label", "潜在链接目标");
    popover.append(chooser(fragmentId, hit, text), action("关闭", () => closePopover(true))); document.body.append(popover);
    const rect = anchor.getBoundingClientRect(); popover.style.left = `${Math.min(Math.max(8, rect.left), Math.max(8, innerWidth - 330))}px`; popover.style.top = `${Math.min(rect.bottom + 5, Math.max(8, innerHeight - popover.offsetHeight - 8))}px`;
  }
  function textRange(mapping: KnowledgeTextMapping, start: number, end: number): Range | null {
    if (!isGraphemeRange(mapping.text, start, end) || mapping.node instanceof HTMLTextAreaElement) return null;
    const node = mapping.node instanceof Text ? mapping.node : mapping.node.firstChild;
    if (!(node instanceof Text) || node.textContent !== mapping.text) return null;
    const range = document.createRange(); range.setStart(node, start); range.setEnd(node, end); return range;
  }
  function drawOverlay(): void {
    clearOverlay();
    if (!enabled || !surface || !sameKnowledgeBasis(snapshotBasis, host.context()) || !surface.isConnected) return;
    const base = surface.getBoundingClientRect(); overlay = element("div", "knowledge-overlays"); overlay.setAttribute("aria-label", "潜在链接提示"); surface.append(overlay);
    const overflow = new Map<string, { rect: DOMRect; count: number }>();
    for (const { fragment, hit } of knownMentions()) {
      const mapping = mappings.find(item => item.nodeKey === (fragment.nodeKey ?? fragment.id)); if (!mapping) continue;
      const range = textRange(mapping, hit.startUtf16, hit.endUtf16); if (!range) continue;
      const rects = [...range.getClientRects()];
      if (hit.lane === undefined) {
        const key = `${fragment.id}:${hit.component}`, previous = overflow.get(key); const rect = rects.at(-1); if (rect) overflow.set(key, { rect, count: (previous?.count ?? 0) + 1 }); continue;
      }
      for (const rect of rects) {
        if (rect.width <= 0) continue;
        const underline = action("", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16)), "knowledge-underline");
        underline.setAttribute("aria-label", `${hit.terms.join(" / ")}：${hit.targets.length} 个候选目标，选择后转换`); underline.style.left = `${rect.left - base.left + surface.scrollLeft}px`; underline.style.top = `${rect.bottom - base.top + surface.scrollTop + hit.lane * 5 - 3}px`; underline.style.width = `${rect.width}px`;
        underline.addEventListener("mouseenter", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16))); underline.addEventListener("focus", () => showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16))); overlay.append(underline);
        underline.addEventListener("keydown", event => { if (event.key === "Enter" || event.key === " ") { event.preventDefault(); showMentionPopover(underline, fragment.id, hit, mapping.text.slice(hit.startUtf16, hit.endUtf16)); popover?.querySelector<HTMLInputElement>('input[type="radio"]')?.focus(); } });
      }
    }
    for (const { rect, count } of overflow.values()) {
      const more = action(`+${count}`, () => { tab = "mentions"; render(); container.querySelector<HTMLElement>(".knowledge-mentions")?.focus(); }, "knowledge-overflow"); more.setAttribute("aria-label", `更多 ${count} 个重叠范围，打开完整候选列表`); more.style.left = `${rect.right - base.left}px`; more.style.top = `${rect.bottom - base.top + 7}px`; overlay.append(more);
    }
  }
  const resize = (): void => { if (!disposed) drawOverlay(); };
  const escape = (event: KeyboardEvent): void => { if (event.key === "Escape" && popover) { closePopover(true); event.stopPropagation(); } };
  window.addEventListener("resize", resize); window.addEventListener("keydown", escape);

  function renderLinks(body: HTMLElement): void {
    const data = snapshot?.links;
    if (!data) { body.append(element("p", "knowledge-help", "当前对象尚无可用链接索引。")); return; }
    body.append(element("p", "knowledge-help", data.coverage === "partial" ? "当前获权范围扫描不完整，下列为已知链接。" : "当前获权扫描范围已完成。"));
    for (const [title, entries] of [["出链", data.outgoing], ["ID 反链", data.incomingId], ["当前路径解析入链", data.incomingPath]] as const) {
      const group = section(title); body.append(group);
      if (!entries.length) { group.append(element("p", "knowledge-empty", data.coverage === "complete" ? "该扫描范围没有链接。" : "暂未发现；扫描不完整。")); continue; }
      for (const entry of entries) {
        const row = element("article", "knowledge-card");
        const target = entry.resolution.target;
        row.append(action(title === "出链" ? target ? nodeLabel(target) : entry.occurrence.target : nodeLabel(entry.source), () => navigate(title === "出链" ? target ?? entry.occurrence.target : entry.source), "knowledge-link-title"));
        row.append(element("span", "knowledge-badge", resolutionLabels[entry.resolution.status] ?? entry.resolution.status), element("p", "knowledge-address", entry.occurrence.target));
        const controls = element("div", "knowledge-actions"); controls.append(action("复制地址", () => copy(entry.occurrence.target)));
        const raw = entry.occurrence.raw ?? (snapshotBasis && entry.source.nodeId === snapshotBasis.nodeId ? snapshotBasis.source.slice(entry.occurrence.utf16From, entry.occurrence.utf16To) : undefined);
        if (raw !== undefined) controls.append(action("复制完整链接", () => copy(raw)));
        if (title === "出链") {
          const format = select(Object.entries(formLabels)); format.setAttribute("aria-label", "链接转换形式"); if (entry.resolution.form) format.value = entry.resolution.form;
          const convert = action("转换", () => perform("转换链接形式", { kind: "linkConvert", generation: data.generation, occurrence: entry.occurrence, form: format.value }));
          convert.disabled = entry.resolution.status !== "resolved" || !entry.occurrence.destinationSlot || host.canEditSource?.() === false;
          controls.append(format, convert);
          if (host.linkCopy) {
            const copyForm = async (full: boolean): Promise<void> => { const context = host.context(); if (!context || !sameKnowledgeBasis(snapshotBasis, context)) { report("链接信息已变化，请刷新后复制。原源仍保留。"); return; } try { const result = await host.linkCopy!(entry, format.value, context); await copy(full ? result.fullLink : result.address); } catch (error) { report(error); } };
            controls.append(action("复制所选地址", () => copyForm(false)), action("复制所选完整链接", () => copyForm(true)));
          }
          if (convert.disabled) row.append(element("p", "knowledge-help", host.canEditSource?.() === false ? "阅读模式可浏览和复制链接；切换到编辑视图后可转换。" : "当前链接不能快捷转换；保留原生源与诊断。"));
        }
        row.append(controls); group.append(row);
      }
    }
    const displayed = section("当前阅读预览中的链接", "来自当前连续阅读预览的实际显示项。生成或来源不明确的项可导航与复制，不提供源转换。"); body.append(displayed);
    const rendered = snapshot?.renderedLinks ?? [];
    if (!rendered.length) displayed.append(element("p", "knowledge-empty", "当前预览没有可用显示链接。"));
    for (const link of rendered) {
      const card = element("article", "knowledge-card"); card.append(action(link.destination, () => navigate(link.destination), "knowledge-link-title"), element("span", "knowledge-badge", `显示项 ${link.occurrenceOrdinal + 1} · 只读`), action("复制显示地址", () => copy(link.destination))); displayed.append(card);
    }
    const properties = snapshot?.propertyReferences;
    const context = host.context();
    const identity = context?.workspaceId && context.nodeId ? `${context.workspaceId}/${context.nodeId}` : null;
    if (properties && identity) {
      const own = properties.items.filter(item => nodeIdentity(item.source) === identity);
      const incoming = properties.items.filter(item => item.resolution.target && nodeIdentity(item.resolution.target) === identity);
      for (const [title, items, outgoing] of [["属性出引用", own, true], ["属性入引用", incoming, false]] as const) {
        const group = section(title, properties.coverage === "complete" ? "来自当前字段定义确认的结构化引用。" : "属性引用扫描不完整，下列为已知结果。"); body.append(group);
        if (!items.length) group.append(element("p", "knowledge-empty", properties.coverage === "complete" ? "当前扫描范围没有属性引用。" : "暂未发现；扫描不完整。"));
        for (const item of items) {
          const target = outgoing ? item.resolution.target : item.source;
          const card = element("article", "knowledge-card"); card.append(element("strong", "", item.fieldTitle || "结构化属性"), element("span", "knowledge-badge", resolutionLabels[item.resolution.status] ?? item.resolution.status));
          if (target) card.append(action(nodeLabel(target), () => navigate(target), "knowledge-link-title"));
          const details = element("details", "knowledge-diagnostics"); details.append(element("summary", "", "字段位置详情"), element("pre", "knowledge-address", [item.fieldId, ...item.itemPath].join(" / "))); card.append(details); group.append(card);
        }
      }
      for (const diagnostic of properties.diagnostics ?? []) body.append(element("p", "knowledge-warning", diagnostic));
    }
  }
  function renderMentions(body: HTMLElement): void {
    const setting = element("label", "knowledge-setting"); const check = input(); check.type = "checkbox"; check.checked = enabled; check.onchange = () => { enabled = check.checked; localStorage.setItem("weftext-potential-links", String(enabled)); drawOverlay(); }; setting.append(check, document.createTextNode("正文显示双下划线提示")); body.append(setting);
    body.append(element("p", "knowledge-help", "每个范围的全部目标都在此列出。正文最多两层提示；更多重叠范围仍可分别操作。"));
    const list = element("div", "knowledge-mentions"); list.tabIndex = -1; body.append(list);
    if (!snapshot?.mentions) { list.append(element("p", "knowledge-empty", "当前预览尚无可用潜在链接扫描。")); return; }
    list.append(element("p", "knowledge-help", snapshot.mentions.coverage === "partial" ? "扫描不完整，下面保留全部已知候选。" : "当前获权扫描已完成。"));
    const hits = knownMentions();
    if (!hits.length) list.append(element("p", "knowledge-empty", snapshot.mentions.coverage === "complete" ? "该扫描范围没有潜在链接。" : "暂未发现；扫描不完整。"));
    for (const { fragment, hit } of hits) {
      const card = element("article", "knowledge-card"); card.append(element("span", "knowledge-badge", `[${hit.startScalar}, ${hit.endScalar})${hit.lane === undefined ? " · 更多重叠" : ""}`), chooser(fragment.id, hit, fragment.text?.slice(hit.startUtf16, hit.endUtf16) ?? hit.terms.join(" / "))); list.append(card);
    }
  }

  function mediaEntry(body: HTMLElement): void {
    const group = section("资源区域", "选择实际资源后，在预览中确定区域。页面尺寸、原始像素、资源版本和音视频时间单位由获权资源读取提供。"); body.append(group);
    const sessionId = host.context()?.sessionId ?? null;
    if (sessionId !== mediaSession) { mediaController?.dispose(); mediaController = null; mediaContainer.replaceChildren(); mediaSession = sessionId; mediaDraft = null; }
    if (host.mountMedia && !mediaController) mediaController = host.mountMedia(mediaContainer, target => { mediaDraft = target; }, mediaDraft ?? undefined);
    if (!host.mountMedia) group.append(element("p", "knowledge-empty", "实际资源预览尚不可用。请选择源范围或整篇批注；已输入讨论会保留。"));
    else group.append(mediaContainer);
  }
  function renderAnnotations(body: HTMLElement): void {
    const diagnostics = snapshot?.annotationDiagnostics ?? [];
    if (snapshot?.annotationStatus && !["ready", "valid", "available", "missing"].includes(snapshot.annotationStatus)) body.append(element("p", "knowledge-warning", `批注文件：${snapshot.annotationStatus}。原件保留，不能覆盖为空文件。`));
    for (const message of diagnostics) body.append(element("p", "knowledge-warning", message));
    const create = section("添加独立批注"); body.append(create);
    const target = select([["whole_document", "整篇笔记"], ["source_range", "当前准确选区"], ["source_element", "当前选中元素"], ["media", "PDF / 图片 / 音视频区域"]]); target.value = annotationTarget;
    target.onchange = () => { annotationTarget = target.value; if (annotationTarget === "source_range" || annotationTarget === "source_element") { annotationSelectionBasis = host.context(); annotationSelection = annotationSelectionBasis?.selection ? { ...annotationSelectionBasis.selection } : undefined; } render(); }; create.append(field("目标", target));
    if (annotationTarget === "source_range" || annotationTarget === "source_element") {
      const capture = action("取当前选区", () => { annotationSelectionBasis = host.context(); annotationSelection = annotationSelectionBasis?.selection ? { ...annotationSelectionBasis.selection } : undefined; render(); }); create.append(capture);
      create.append(element("p", "knowledge-help", annotationSelection ? `源码 UTF-16 [${annotationSelection.from}, ${annotationSelection.to})。${host.context()?.dirty ? "请先保存源，再重新取准确选区；批注草稿会保留。" : "提交时重新验证准确源范围。"}` : "请在正文或源码中选择准确范围，再取当前选区。"));
      if (annotationSelection?.visibleExcerpt) create.append(element("blockquote", "knowledge-excerpt", annotationSelection.visibleExcerpt));
    }
    if (annotationTarget === "media") mediaEntry(create);
    const kind = select([["comment", "讨论"], ["mark", "标记"], ["suggestion", "修改建议"]]); kind.value = annotationKind; kind.onchange = () => { annotationKind = kind.value; render(); }; create.append(field("用途", kind));
    const entry = textarea(annotationBody, "写下讨论或说明…"); entry.oninput = () => { annotationBody = entry.value; }; create.append(field("批注内容", entry));
    if (annotationKind === "suggestion") { const replacement = textarea(suggestionReplacement, "要替换成的文字"); replacement.oninput = () => { suggestionReplacement = replacement.value; }; create.append(field("建议替换文字", replacement), element("p", "knowledge-help", "接受建议会在同一次 Core 事务中修改源与接受状态。")); }
    create.append(action("准备并保存批注", async () => {
      if (!annotationBody.trim()) { report("请输入批注内容。"); return; }
      let annotationAnchor: unknown = { kind: "whole_document" };
      if (annotationTarget === "media") { if (!mediaDraft) { report("请在实际资源预览中选择一个准确区域。批注输入已保留。"); return; } const error = validateMediaRegion(mediaDraft); if (error) { report(error); return; } annotationAnchor = { ...mediaDraft }; }
      else if (annotationTarget !== "whole_document") {
        const context = host.context(); if (!context || !annotationSelection || !sameKnowledgeBasis(annotationSelectionBasis, context) || !isGraphemeRange(context.source, annotationSelection.from, annotationSelection.to)) { report("选区对应的源版本已变化，请重新取完整字符范围。批注输入已保留。"); return; }
        if (annotationTarget === "source_element" && !annotationSelection.elementRole) { report("当前选区没有已验证元素角色，请选择源范围或整篇批注。"); return; }
        annotationAnchor = { kind: annotationTarget, ...annotationSelection };
      }
      if (await perform("保存独立批注", { kind: "annotationCreate", target: annotationAnchor, body: annotationBody, purpose: annotationKind, replacement: annotationKind === "suggestion" ? suggestionReplacement : undefined })) { annotationBody = ""; suggestionReplacement = ""; render(); }
    }, "knowledge-button primary"));
    const threads = section("讨论与建议"); body.append(threads);
    if (!snapshot?.annotations?.length) threads.append(element("p", "knowledge-empty", "当前可读范围暂无批注。"));
    for (const annotation of snapshot?.annotations ?? []) {
      const id = annotation.annotationId ?? annotation.id;
      const card = element("article", "knowledge-card"); card.append(element("span", "knowledge-badge", annotation.reviewState === "resolved" ? "已解决" : "未解决"), element("span", `knowledge-badge ${annotation.targetResolution === "exact" || annotation.targetResolution === "mapped" ? "" : "warning"}`, resolutionLabels[annotation.targetResolution ?? "unavailable"] ?? "目标不可用"));
      if (annotation.targetSummary) card.append(element("p", "knowledge-help", annotation.targetSummary));
      if (excerpt(annotation)) card.append(element("blockquote", "knowledge-excerpt", excerpt(annotation)));
      card.append(element("p", "knowledge-annotation-body", annotation.body));
      if (annotation.author || annotation.createdAt) card.append(element("p", "knowledge-help", [annotation.author, annotation.createdAt].filter(Boolean).join(" · ")));
      if (annotation.purpose === "suggestion") { card.append(element("span", "knowledge-badge", ({ pending: "待接受", accepted: "已接受", rejected: "已拒绝" })[annotation.suggestionState ?? "pending"])); if (annotation.replacement !== undefined) card.append(element("pre", "knowledge-replacement", annotation.replacement)); }
      const controls = element("div", "knowledge-actions");
      controls.append(action(annotation.reviewState === "resolved" ? "重新打开" : "解决", () => perform("更新讨论状态", { kind: "annotationResolve", annotationId: id, resolved: annotation.reviewState !== "resolved" })));
      if (annotation.targetResolution === "exact" || annotation.targetResolution === "mapped") {
        if (annotation.sourceLocation && host.selectSource) controls.append(action("回到目标", () => { if (!sameKnowledgeBasis(snapshotBasis, host.context())) { report("源版本已变化，请刷新后重新定位。讨论已保留。"); return; } host.selectSource!(annotation.sourceLocation!.from, annotation.sourceLocation!.to); }));
        else if (annotation.navigateTarget) controls.append(action("回到目标", () => navigate(annotation.navigateTarget!)));
        else if (annotation.resourceLocation) controls.append(action("回到媒体目标", () => {
          if (!sameKnowledgeBasis(snapshotBasis, host.context())) { report("当前笔记已变化，请刷新后重新定位。讨论已保留。"); return; }
          const location = annotation.resourceLocation!, target = storedMediaSelection(location.target);
          if (!target || target.resource !== location.resource || target.revision !== location.revision) { report("准确媒体目标目前不可用；讨论已保留。"); return; }
          mediaController?.dispose(); mediaController = null; mediaContainer.replaceChildren(); mediaDraft = target; annotationTarget = "media"; tab = "annotations"; render(); mediaContainer.scrollIntoView({ block: "nearest" });
        }));
      }
      else if (!(annotation.purpose === "suggestion" && annotation.suggestionState === "pending")) controls.append(action("重新定位到当前目标", async () => {
        if (annotationTarget === "media") { if (!mediaDraft || validateMediaRegion(mediaDraft)) { report("请先在真实资源预览中选择当前准确区域。讨论已保留。"); return; } await perform("重新定位批注", { kind: "annotationReattach", annotationId: id, target: { ...mediaDraft } }); return; }
        const selection = host.context()?.selection; if (!selection) { report("请先选择当前准确目标。讨论已保留。"); return; } await perform("重新定位批注", { kind: "annotationReattach", annotationId: id, target: { kind: "source_range", ...selection } });
      }));
      if (annotation.purpose === "suggestion" && annotation.suggestionState === "pending") {
        const accept = action("检查并接受", () => perform("同时应用建议与接受状态", { kind: "annotationAccept", annotationId: id })); accept.disabled = !canApplySuggestion(annotation); controls.append(accept, action("拒绝建议", () => perform("拒绝修改建议", { kind: "annotationReject", annotationId: id })));
        let draft = reconfirmations.get(id); if (!draft) { draft = { replacement: annotation.replacement ?? "", basis: null }; reconfirmations.set(id, draft); }
        const reconsider = element("details", "knowledge-reconfirm"); reconsider.append(element("summary", "", "重新确认建议目标与文字"), element("p", "knowledge-help", "目标变化后，请选择已保存源中的准确范围并重新确认。现有建议与讨论会保留，接受仍是单独操作。"));
        const replacement = textarea(draft.replacement); replacement.oninput = () => { draft!.replacement = replacement.value; }; reconsider.append(field("重新确认的建议文字", replacement));
        reconsider.append(action("取当前准确选区", () => { const context = host.context(); draft!.basis = context; draft!.selection = context?.selection ? { ...context.selection } : undefined; const caption = reconsider.querySelector<HTMLElement>(".knowledge-reconfirm-selection"); if (caption) caption.textContent = draft!.selection ? `已选择 UTF-16 [${draft!.selection.from}, ${draft!.selection.to})。` : "当前没有准确选区。"; }));
        reconsider.append(element("p", "knowledge-help knowledge-reconfirm-selection", draft.selection ? `已选择 UTF-16 [${draft.selection.from}, ${draft.selection.to})。` : "请先在正文或源码中选择目标，再取当前准确选区。"));
        reconsider.append(action("准备重新确认", async () => {
          const context = host.context(); const selection = draft!.selection;
          if (!context || context.dirty || !selection || !sameKnowledgeBasis(draft!.basis, context) || !isGraphemeRange(context.source, selection.from, selection.to, true) || selection.from === selection.to && !draft!.replacement) { report("请先保存源并重新取当前完整字符范围，插入建议也可取正文光标位置。建议文字已保留。"); return; }
          await perform("重新确认建议的目标与文字", { kind: "annotationReconfirm", annotationId: id, target: { kind: selection.from === selection.to ? "source_point" : "source_range", ...selection }, replacement: draft!.replacement });
        })); card.append(reconsider);
      }
      card.append(controls);
      for (const reply of annotation.replies ?? []) { const value = element("div", "knowledge-reply"); value.append(element("p", "knowledge-annotation-body", reply.body)); if (reply.author || reply.createdAt) value.append(element("small", "", [reply.author, reply.createdAt].filter(Boolean).join(" · "))); card.append(value); }
      const reply = textarea(replies.get(id) ?? "", "回复这条讨论…"); reply.setAttribute("aria-label", `回复批注 ${id}`); reply.oninput = () => replies.set(id, reply.value); card.append(reply, action("保存回复", async () => { const text = replies.get(id) ?? ""; if (!text.trim()) return; if (await perform("保存批注回复", { kind: "annotationReply", annotationId: id, body: text })) { replies.delete(id); render(); } })); threads.append(card);
    }
  }
  function renderFiles(body: HTMLElement): void {
    const context = host.context(); if (!context) return;
    const managed = Boolean(context.nodeId && context.workspaceId);
    const nodeDirectory = context.path.replace(/[/\\][^/\\]+$/, "");
    let draft = fileDrafts.get(context.sessionId);
    if (!draft) { draft = { name: context.path.replaceAll("\\", "/").split("/").at(-1)?.replace(/\.typ$/i, "") ?? "", parent: context.root, backup: "", allowLosses: false, restores: new Map() }; fileDrafts.set(context.sessionId, draft); }
    const group = section("笔记节点操作", managed ? "同名目录与主文件作为一个节点操作。每次预览准确文件变动，稳定 ID 保持同一对象。" : "当前是普通文件。节点操作需要已纳管的资料库笔记。"); body.append(group);
    const name = input(draft.name); name.oninput = () => { draft!.name = name.value; };
    const parent = input(draft.parent); parent.oninput = () => { draft!.parent = parent.value; }; group.append(field("新名称 / 复制名称", name), field("目标父目录（完整路径）", parent));
    const lossChoice = element("label", "knowledge-setting"); const allowLosses = input(); allowLosses.type = "checkbox"; allowLosses.checked = draft.allowLosses; allowLosses.onchange = () => { draft!.allowLosses = allowLosses.checked; }; lossChoice.append(allowLosses, document.createTextNode("复制时允许保留未能迁移的引用并保存原件")); group.append(lossChoice, element("p", "knowledge-help", "选择后，复制预览会列出未能迁移的引用与保留原件。每项损失仍需核对后确认。"));
    if (managed && context.dirty) group.append(element("p", "knowledge-warning", "当前有未保存草稿，请先保存再操作节点。"));
    const actions = element("div", "knowledge-actions");
    for (const [label, kind] of [["改名", "renameNode"], ["移动", "moveNode"], ["复制为独立节点", "copyNode"], ["送到回收站", "trashNode"]]) {
      const control = action(label, () => perform(label, { kind, source: nodeDirectory, ...(kind === "renameNode" || kind === "copyNode" ? { name: name.value } : {}), ...(kind === "moveNode" || kind === "copyNode" ? { parent: parent.value } : {}), ...(kind === "copyNode" && draft!.allowLosses ? { allowLosses: true } : {}) })); control.disabled = !managed || context.dirty; actions.append(control);
    }
    group.append(actions);
    const backup = section("资料库备份", "备份已保存的资料库文件到明确的新目录；预览全部复制项和准确目标。"); const destination = input(draft.backup, "D:\\Backups\\我的资料库副本"); destination.oninput = () => { draft!.backup = destination.value; }; backup.append(field("备份目标（完整路径）", destination), action("准备备份", () => perform("创建资料库备份", { kind: "backupLibrary", destination: destination.value }))); body.append(backup);
    const trash = section("回收站", "这里仅提供明确恢复入口。恢复前检查目标名称、位置与身份。没有永久删除操作。"); body.append(trash);
    if (!snapshot?.trash?.length) trash.append(element("p", "knowledge-empty", "暂无可读恢复记录。"));
    for (const entry of snapshot?.trash ?? []) { const card = element("article", "knowledge-card"); const restoreDraft = trashRestoreDraft(draft.restores, context.root, entry); const restoreName = input(restoreDraft.name), restoreParent = input(restoreDraft.parent); restoreName.oninput = () => { restoreDraft.name = restoreName.value; }; restoreParent.oninput = () => { restoreDraft.parent = restoreParent.value; }; card.append(element("strong", "", entry.name)); if (entry.path) card.append(element("p", "knowledge-address", entry.path)); card.append(field("恢复名称", restoreName), field("恢复父目录", restoreParent), action("准备恢复", () => perform("恢复笔记节点", { kind: "restoreNode", trashOperationId: entry.operationId, ...(entry.nodeId !== undefined ? { nodeId: entry.nodeId } : {}), parent: restoreParent.value, name: restoreName.value }))); trash.append(card); }
  }
  function templateValueControl(kind: string, value: KnowledgeTemplateValue | undefined, changed: (value: KnowledgeTemplateValue) => void): HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement {
    if (kind === "bool") { const control = select([["false", "否"], ["true", "是"]]); control.value = value === true ? "true" : "false"; control.onchange = () => changed(control.value === "true"); return control; }
    if (kind === "int") { const control = input(typeof value === "string" ? value : ""); control.inputMode = "numeric"; control.pattern = "[+-]?[0-9]+"; control.placeholder = "准确整数"; control.oninput = () => changed(control.value); return control; }
    const control = textarea(typeof value === "string" ? value : ""); control.rows = kind === "markup" ? 4 : 2; control.oninput = () => changed(control.value); return control;
  }
  function renderTemplateRegistration(body: HTMLElement): void {
    const context = host.context(); if (!context) return;
    let draft = registrationDrafts.get(context.sessionId);
    if (!draft) { draft = { title: "", description: "", basis: null, parameters: [], open: false, slotsOpen: false, slot: { name: "", kind: "markup", nullable: false, defaultMode: "omitted", defaultValue: "" } }; registrationDrafts.set(context.sessionId, draft); }
    const registration = element("details", "knowledge-template-registration"); registration.open = draft.open; registration.ontoggle = () => { if (registration.isConnected) draft!.open = registration.open; }; registration.append(element("summary", "", draft.templateId ? "更新原生模板登记" : "把当前笔记登记为模板")); body.append(registration);
    const group = section("原生模板登记", "模板继续使用这篇原生笔记作为来源。可以直接登记整篇，也可为准确选中的字面量设置参数。"); registration.append(group);
    const title = input(draft.title), description = textarea(draft.description); title.oninput = () => { draft!.title = title.value; }; description.oninput = () => { draft!.description = description.value; }; group.append(field("模板名称", title), field("用途说明", description));
    if (context.dirty) group.append(element("p", "knowledge-warning", "请先保存当前笔记，再登记模板和准确参数范围。输入会保留。"));
    const details = element("details"); details.open = draft.slotsOpen; details.ontoggle = () => { if (details.isConnected) draft!.slotsOpen = details.open; }; details.append(element("summary", "", `参数范围（${draft.parameters.length} 项，可不设置）`)); group.append(details);
    for (const [index, parameter] of draft.parameters.entries()) { const row = element("article", "knowledge-card"); row.append(element("strong", "", parameter.name), element("p", "knowledge-help", `${({ markup: "正文文字", string: "字符串字面量", bool: "布尔字面量", int: "整数字面量" })[parameter.kind]} · UTF-16 [${parameter.from}, ${parameter.to})`), action("移除此参数", () => { draft!.parameters.splice(index, 1); render(); })); details.append(row); }
    const slotName = input(draft.slot.name, "例如 title、body"); slotName.oninput = () => { draft!.slot.name = slotName.value; };
    const kind = select([["markup", "正文文字"], ["string", "完整字符串字面量（含引号）"], ["bool", "布尔字面量"], ["int", "整数字面量"]]); kind.dataset.knowledgeFocus = "template-register-kind"; kind.value = draft.slot.kind; kind.onchange = () => { draft!.slot.kind = kind.value as RegistrationSlot["kind"]; draft!.slot.defaultValue = kind.value === "bool" ? false : ""; if (kind.value === "markup") { draft!.slot.nullable = false; if (draft!.slot.defaultMode === "none") draft!.slot.defaultMode = "omitted"; } render(); };
    details.append(field("参数名称", slotName), field("参数类型", kind));
    const nullable = element("label", "knowledge-setting"), nullCheck = input(); nullCheck.dataset.knowledgeFocus = "template-register-nullable"; nullCheck.type = "checkbox"; nullCheck.checked = draft.slot.nullable; nullCheck.disabled = draft.slot.kind === "markup"; nullCheck.onchange = () => { draft!.slot.nullable = nullCheck.checked; if (!nullCheck.checked && draft!.slot.defaultMode === "none") draft!.slot.defaultMode = "omitted"; render(); }; nullable.append(nullCheck, document.createTextNode("允许空值")); details.append(nullable);
    const defaultChoices: [string, string][] = [["omitted", "必填，不设默认值"], ["value", "提供默认值"]]; if (draft.slot.nullable) defaultChoices.push(["none", "默认使用空值"]);
    const mode = select(defaultChoices); mode.dataset.knowledgeFocus = "template-register-default-mode"; mode.value = draft.slot.defaultMode; mode.onchange = () => { draft!.slot.defaultMode = mode.value as "omitted" | "value" | "none"; render(); }; details.append(field("默认值", mode));
    if (draft.slot.defaultMode === "value") details.append(field("默认内容", templateValueControl(draft.slot.kind, draft.slot.defaultValue, value => { draft!.slot.defaultValue = value; })));
    details.append(action("把当前选区加入参数", () => {
      const current = host.context(), selection = current?.selection;
      if (!current || current.dirty || !selection || !isGraphemeRange(current.source, selection.from, selection.to)) { report("请保存源并选择准确的完整字符范围。参数输入已保留。"); return; }
      if (draft!.basis && !sameKnowledgeBasis(draft!.basis, current)) { report("已有参数范围对应的源版本已变化。请移除失效范围后重新选择。"); return; }
      if (!draft!.slot.name.trim() || draft!.parameters.some(parameter => parameter.name === draft!.slot.name)) { report("请输入尚未使用的参数名称。"); return; }
      if (draft!.parameters.some(parameter => parameter.from < selection.to && selection.from < parameter.to)) { report("参数范围不能包含或交叉已有参数，请分别选择。"); return; }
      if (draft!.slot.kind === "int" && draft!.slot.defaultMode === "value" && (typeof draft!.slot.defaultValue !== "string" || !/^[+-]?\d+$/.test(draft!.slot.defaultValue))) { report("默认整数请使用准确十进制整数文字，不会转换为近似数值。"); return; }
      draft!.parameters.push({ name: draft!.slot.name, kind: draft!.slot.kind, from: selection.from, to: selection.to, nullable: draft!.slot.nullable, ...(draft!.slot.defaultMode === "value" ? { default: draft!.slot.defaultValue } : draft!.slot.defaultMode === "none" ? { default: null } : {}) }); draft!.basis = current; draft!.slot.name = ""; render();
    }));
    if (draft.parameters.length) details.append(action("清空参数范围并重新选择", () => { draft!.parameters = []; draft!.basis = null; render(); }));
    const save = action(draft.templateId ? "准备更新模板登记" : "准备登记模板", async () => {
      const current = host.context(); if (!current || current.dirty || !current.nodeId || !current.workspaceId) { report("请先打开并保存资料库中的原生笔记。模板输入已保留。"); return; }
      if (!draft!.title.trim()) { report("请输入模板名称。"); return; }
      if (draft!.parameters.length && !sameKnowledgeBasis(draft!.basis, current)) { report("参数范围的源版本已变化，请重新选择准确范围。"); return; }
      await perform("登记当前原生笔记为模板", { kind: "templateRegister", ...(draft!.templateId ? { templateId: draft!.templateId } : {}), title: draft!.title, description: draft!.description, parameters: draft!.parameters });
    }, "knowledge-button primary"); save.disabled = context.dirty || !context.nodeId || !context.workspaceId; group.append(save);
    if (draft.templateId) group.append(action("改为登记新的模板", () => { draft!.templateId = undefined; render(); }));
  }
  function renderTemplates(body: HTMLElement): void {
    const templates = snapshot?.templates ?? [];
    body.append(element("p", "knowledge-help", "参数按声明类型传入原生模板。创建前预览源、资源、身份映射与损失，确认同一份准备结果。"));
    renderTemplateRegistration(body);
    if (!templates.length) { body.append(element("p", "knowledge-empty", "当前资料库没有可用创建模板。")); return; }
    for (const template of templates) {
      const key = `${host.context()?.sessionId}:${template.id}`; let draft = templateDrafts.get(key);
      if (!draft) { draft = { name: "", parent: host.context()?.root ?? "", parameters: {}, presence: {} }; for (const parameter of template.parameters ?? []) draft.presence[parameter.name] = parameter.required && parameter.default === undefined ? "value" : "omitted"; templateDrafts.set(key, draft); }
      const card = section(template.title, template.description); const name = input(draft.name, "新笔记名称"), parent = input(draft.parent); name.oninput = () => { draft!.name = name.value; }; parent.oninput = () => { draft!.parent = parent.value; }; card.append(field("新笔记名称", name), field("目标父目录", parent));
      if (template.sourceNode) card.append(action(`来源：${nodeLabel(template.sourceNode)}`, () => navigate(template.sourceNode!), "knowledge-link-title"));
      if (template.available === false) card.append(element("p", "knowledge-warning", "登记的来源版本或参数范围目前不可用。请检查来源，必要时明确更新登记。"));
      const context = host.context(); if (template.sourceNode && context?.nodeId === template.sourceNode.nodeId && context.workspaceId === template.sourceNode.workspaceId) card.append(action("更新当前模板登记", () => { const registration = registrationDrafts.get(context.sessionId); if (!registration) return; registration.templateId = template.id; registration.title = template.title; registration.description = template.description ?? ""; registration.parameters = []; registration.basis = null; registration.open = true; registration.slotsOpen = true; render(); container.querySelector<HTMLElement>(".knowledge-body")?.scrollTo({ top: 0 }); }));
      const parameters = draft.parameters;
      const currentParameters = (): Record<string, KnowledgeTemplateValue> => {
        const values: Record<string, KnowledgeTemplateValue> = {};
        for (const parameter of template.parameters ?? []) { const mode = draft!.presence[parameter.name] ?? "omitted"; if (mode === "value") values[parameter.name] = parameter.kind === "bool" ? parameters[parameter.name] === true : parameters[parameter.name] ?? ""; else if (mode === "none") values[parameter.name] = null; }
        return values;
      };
      for (const parameter of template.parameters ?? []) {
        const mode = draft.presence[parameter.name] ?? "omitted";
        const control = templateValueControl(parameter.kind ?? "markup", parameter.name in parameters ? parameters[parameter.name] : mode === "omitted" ? parameter.default : undefined, value => { parameters[parameter.name] = value; }); control.required = Boolean(parameter.required); control.disabled = mode !== "value";
        const choices: [string, string][] = [];
        if (!parameter.required || parameter.default !== undefined) choices.push(["omitted", parameter.default !== undefined ? "使用模板默认值" : "不提供此参数"]);
        choices.push(["value", "填写文字"]); if (parameter.nullable) choices.push(["none", "使用空值"]);
        if (choices.length > 1) { const presence = select(choices); presence.value = mode; presence.setAttribute("aria-label", `${parameter.title ?? parameter.name}的输入方式`); presence.onchange = () => { draft!.presence[parameter.name] = presence.value as "omitted" | "value" | "none"; control.disabled = presence.value !== "value"; if (presence.value === "value" && !(parameter.name in parameters)) parameters[parameter.name] = parameter.kind === "bool" ? control.value === "true" : control.value; }; card.append(field(parameter.title ?? parameter.name, presence)); }
        card.append(field(choices.length > 1 ? "参数值" : parameter.title ?? parameter.name, control));
      }
      const preview = element("div", "knowledge-template-preview");
      if (template.source) { const source = element("pre", "knowledge-template-source", template.source); const details = element("details"); details.append(element("summary", "", "原生模板源码"), source); card.append(details); }
      const actions = element("div", "knowledge-actions");
      const inspect = action("预览模板", async () => { const context = host.context(); if (!context || !host.previewTemplate) return; const values = currentParameters(); try { const result = await host.previewTemplate(template.id, values, context); if (!sameKnowledgeBasis(context, host.context()) || JSON.stringify(values) !== JSON.stringify(currentParameters())) { report("模板参数或当前草稿已变化，请重新预览。输入已保留。"); return; } preview.replaceChildren(); if (result.tree) preview.append(nativeTree(result.tree)); else if (result.source) preview.append(element("pre", "knowledge-template-source", result.source)); for (const diagnostic of result.diagnostics ?? []) preview.append(element("p", "knowledge-warning", diagnostic.message)); } catch (error) { report(error); } }); inspect.disabled = !host.previewTemplate || template.available === false;
      const create = action("准备创建并打开", () => perform("从原生模板创建笔记", { kind: "templateCreate", templateId: template.id, parent: parent.value, name: name.value, parameters: currentParameters() }), "knowledge-button primary"); create.disabled = template.available === false;
      actions.append(inspect, create); card.append(actions, preview); body.append(card);
    }
  }
  function render(): void {
    if (disposed) return;
    deferredRender = false;
    const focused = document.activeElement instanceof HTMLElement && container.contains(document.activeElement) ? document.activeElement : null;
    const focusKey = sameKnowledgeBasis(renderedBasis, host.context()) ? focused?.dataset.knowledgeFocus : undefined;
    renderedBasis = host.context();
    const scroll = container.querySelector(".knowledge-body")?.scrollTop ?? 0;
    container.replaceChildren();
    const heading = element("div", "knowledge-heading"); heading.append(element("strong", "", "知识与批注"), action("刷新", refresh, "knowledge-refresh")); container.append(heading);
    const tabs = element("div", "knowledge-tabs"); tabs.setAttribute("role", "tablist"); tabs.setAttribute("aria-label", "知识面板");
    for (const [id, label] of [["links", "链接"], ["mentions", "潜在链接"], ["annotations", "批注"], ["proofreading", "校对"], ["files", "文件"], ["templates", "模板"]]) { const control = action(label, () => { tab = id; render(); }, "knowledge-tab"); control.setAttribute("role", "tab"); control.setAttribute("aria-selected", String(tab === id)); control.tabIndex = tab === id ? 0 : -1; control.onkeydown = event => { if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return; event.preventDefault(); const ids = ["links", "mentions", "annotations", "proofreading", "files", "templates"]; const index = ids.indexOf(tab); tab = event.key === "Home" ? ids[0] : event.key === "End" ? ids.at(-1)! : ids[(index + (event.key === "ArrowRight" ? 1 : ids.length - 1)) % ids.length]; render(); container.querySelector<HTMLButtonElement>('[aria-selected="true"]')?.focus(); }; tabs.append(control); }
    container.append(tabs, status);
    if (pendingPlan) { const pending = element("div", "knowledge-warning", `原操作 ${pendingPlan.plan.operationId} 结果待确认。`); if (host.inspectOperation) pending.append(action("检查原操作", inspectPending)); container.append(pending); }
    const body = element("div", "knowledge-body"); body.setAttribute("role", "tabpanel"); container.append(body);
    if (!host.context()) { body.append(element("p", "knowledge-empty", "打开笔记后查看链接、批注和文件操作。")); return; }
    body.append(element("p", "knowledge-help knowledge-current", host.context()!.path));
    for (const message of snapshot?.warnings ?? []) body.append(element("p", "knowledge-warning", message));
    if (tab === "links") renderLinks(body); else if (tab === "mentions") renderMentions(body); else if (tab === "annotations") renderAnnotations(body); else if (tab === "proofreading") proofreading.render(body); else if (tab === "files") renderFiles(body); else renderTemplates(body);
    body.scrollTop = scroll;
    if (focusKey) container.querySelector<HTMLElement>(`[data-knowledge-focus="${CSS.escape(focusKey)}"]`)?.focus();
  }
  async function refresh(): Promise<void> {
    const context = host.context(); const generation = gate.begin();
    if (!context) { snapshot = null; snapshotBasis = null; clearOverlay(); render(); return; }
    if (!sameKnowledgeBasis(snapshotBasis, context)) { snapshot = null; snapshotBasis = null; clearOverlay(); render(); }
    status.textContent = "正在读取当前获权知识信息…";
    try {
      const next = await host.load(context); if (disposed || !gate.accepts(generation, context, host.context())) return;
      snapshot = next; snapshotBasis = context; status.textContent = "";
      if (container.contains(document.activeElement) && document.activeElement?.matches("input,textarea,select")) deferredRender = true; else render();
      drawOverlay();
    } catch (error) { if (gate.accepts(generation, context, host.context())) { snapshot = null; snapshotBasis = null; clearOverlay(); report(error); render(); } }
  }
  render();
  return {
    refresh,
    selectTab(next) { tab = next; render(); },
    attachSurface(next, nextMappings = []) { clearOverlay(); surface = next; mappings = nextMappings; if (surface) surface.classList.add("knowledge-surface"); drawOverlay(); },
    dispose() { disposed = true; gate.invalidate(); clearOverlay(); mediaController?.dispose(); window.removeEventListener("resize", resize); window.removeEventListener("keydown", escape); container.removeEventListener("focusout", onBlur); container.replaceChildren(); },
  };
}
