/** Transient history presentation only. Core owns versions, retention and Draft persistence. */
import type { DocumentBasis } from "./data-panel";
import type { Receipt } from "./types";

export interface HistoryContext extends DocumentBasis {
  baseSource: string;
  dirty: boolean;
  readOnly?: boolean;
  savePending?: boolean;
  unresolvedSave?: boolean;
}
export interface ProtectedHistoryDraft { path: string; baseSource: string; source: string; baseKnown: boolean }
export interface HistoryVersion {
  versionId: string;
  revisionId: string | null;
  operationId: string;
  kind: string;
  savedAtUnixMs: number | null;
  timeBasis: "commitClock" | "unknown";
  commitSequence: number | null;
  status: "available" | "missing" | "unreadable" | "invalid";
  byteLength: number | null;
  sha256: string | null;
  message: string;
}
export interface HistoryList {
  nodeId: string;
  workspaceId: string;
  path: string;
  currentBaseId: string;
  currentSavedVersionId: string | null;
  currentness: "saved" | "external";
  versions: HistoryVersion[];
  retention: {
    policy: "retainedCoreJournal";
    complete: false;
    knownVersions: number;
    availableVersions: number;
    oldestSavedAtUnixMs: number | null;
    newestSavedAtUnixMs: number | null;
    message: string;
  };
  protectedDrafts: ProtectedHistoryDraft[];
  warnings: string[];
}
export interface HistorySource extends HistoryVersion {
  source: string;
  readOnly: true;
  previewStatus: "sourceOnly";
  previewMessage: string;
}
export interface SavedHistoryComparison {
  selected: HistorySource;
  current: { versionId: string | null; revisionId: string | null; source: string; baseId: string; path: string };
  equal: boolean;
  comparison: "currentSaved";
}
export interface DraftHistoryComparison {
  selected: HistorySource;
  current: { source: string; serial: number; baseId: string; path: string };
  equal: boolean;
  comparison: "draftSnapshot";
}
export type HistoryComparison = SavedHistoryComparison | DraftHistoryComparison;
export interface HistoryRestoreRequest {
  sessionId: string;
  baseId: string;
  versionId: string;
  operationId: string;
  draftAction: "retain" | "none";
  draftSource?: string;
}
export interface HistoryRestoreResult {
  receipt: Receipt;
  source: string;
  versionId: string;
  protectedDrafts: ProtectedHistoryDraft[];
}
export interface HistoryOperationStatus {
  operationId: string;
  status: "missing" | "committed" | "recoveryPending" | "resolved";
  result?: HistoryRestoreResult;
}
export interface HistoryRestoreBasis extends HistoryContext { selectedVersionId: string; selectedSource: string }
export interface HistoryRestoreReview {
  basis: Readonly<HistoryRestoreBasis>;
  comparison: SavedHistoryComparison;
}
export interface HistoryTransport {
  id(): string;
  current(): HistoryContext | null;
  list(request: { sessionId: string }): Promise<HistoryList>;
  read(request: { sessionId: string; versionId: string }): Promise<HistorySource>;
  compare(request: { sessionId: string; versionId: string }): Promise<SavedHistoryComparison>;
  restore(request: HistoryRestoreRequest): Promise<HistoryRestoreResult>;
  status(request: { operationId: string }): Promise<HistoryOperationStatus>;
}

const clone = <T>(value: T): T => structuredClone(value);
const fail = (code: string, message: string): Error => Object.assign(new Error(message), { code });
const codeOf = (error: unknown): string => typeof error === "object" && error !== null && "code" in error ? String(error.code) : "";
export function uncertainHistoryOperation(error: unknown): boolean {
  const code = codeOf(error);
  return !code || ["unknown", "resultUnknown", "recoveryPending", "pending", "in-progress", "pending_reconciliation"].includes(code);
}
export function sameHistoryNote(a: HistoryContext, b: HistoryContext | null): boolean {
  return !!b && a.sessionId === b.sessionId && a.nodeId === b.nodeId && a.workspaceId === b.workspaceId;
}
export function sameHistoryDraft(a: HistoryContext, b: HistoryContext | null): boolean {
  return sameHistoryNote(a, b) && a.path === b!.path && a.baseId === b!.baseId && a.serial === b!.serial && a.source === b!.source;
}
/** The source owner uses this before applying a restore receipt; newer input always survives. */
export function restoreBufferDisposition(basis: HistoryRestoreBasis, current: HistoryContext | null): "replaceClean" | "retainDraft" | "unrelated" {
  if (!sameHistoryNote(basis, current)) return "unrelated";
  return basis.dirty || basis.source !== basis.baseSource || !sameHistoryDraft(basis, current) ? "retainDraft" : "replaceClean";
}
export function historyTimeLabel(version: Pick<HistoryVersion, "savedAtUnixMs" | "timeBasis">, locale = "zh-CN"): string {
  if (version.timeBasis !== "commitClock" || version.savedAtUnixMs === null || !Number.isFinite(version.savedAtUnixMs)) return "时间未知";
  const value = new Date(version.savedAtUnixMs);
  return Number.isNaN(value.getTime()) ? "时间未知" : value.toLocaleString(locale, { hour12: false });
}

/** A bounded exact-text diff. The two lanes always reconstruct their original input. */
export interface HistoryDiffPart { kind: "same" | "removed" | "added"; text: string }
export function historySourceDiff(before: string, after: string): HistoryDiffPart[] {
  if (before === after) return [{ kind: "same", text: before }];
  const lines = (source: string): string[] => source.match(/[^\r\n]*(?:\r\n|\r|\n)|[^\r\n]+$/g) ?? [];
  const a = lines(before), b = lines(after);
  let first = 0, lastA = a.length, lastB = b.length;
  while (first < lastA && first < lastB && a[first] === b[first]) first++;
  while (lastA > first && lastB > first && a[lastA - 1] === b[lastB - 1]) { lastA--; lastB--; }
  const result: HistoryDiffPart[] = [];
  const append = (kind: HistoryDiffPart["kind"], text: string): void => {
    if (!text) return;
    if (result.at(-1)?.kind === kind) result[result.length - 1].text += text;
    else result.push({ kind, text });
  };
  append("same", a.slice(0, first).join(""));
  const left = a.slice(first, lastA), right = b.slice(first, lastB);
  if ((left.length + 1) * (right.length + 1) > 250_000) {
    append("removed", left.join("")); append("added", right.join(""));
  } else {
    const width = right.length + 1;
    const table = new Uint32Array((left.length + 1) * width);
    for (let i = left.length - 1; i >= 0; i--) for (let j = right.length - 1; j >= 0; j--) {
      table[i * width + j] = left[i] === right[j] ? table[(i + 1) * width + j + 1] + 1 : Math.max(table[(i + 1) * width + j], table[i * width + j + 1]);
    }
    let i = 0, j = 0;
    while (i < left.length || j < right.length) {
      if (i < left.length && j < right.length && left[i] === right[j]) { append("same", left[i++]); j++; }
      else if (i < left.length && (j === right.length || table[(i + 1) * width + j] >= table[i * width + j + 1])) append("removed", left[i++]);
      else append("added", right[j++]);
    }
  }
  append("same", a.slice(lastA).join(""));
  return result;
}

/** One controller per open Core session. Reads never alter the shared source or Undo. */
export class HistoryController {
  list: HistoryList | null = null;
  selected: HistorySource | null = null;
  comparison: HistoryComparison | null = null;
  review: HistoryRestoreReview | null = null;
  busy = false;
  uncertain = false;
  operationState: HistoryOperationStatus["status"] | null = null;
  private generation = 0;
  private rejected = false;
  private pending: { request: Readonly<HistoryRestoreRequest>; basis: Readonly<HistoryRestoreBasis> } | null = null;
  constructor(readonly host: HistoryTransport, readonly note: Readonly<HistoryContext>) {}
  get operationId(): string | null { return this.pending?.request.operationId ?? null; }
  get pendingBasis(): Readonly<HistoryRestoreBasis> | null { return this.pending?.basis ?? null; }
  get locked(): boolean { return this.busy || this.pending !== null; }
  private context(): HistoryContext {
    const current = this.host.current();
    if (!sameHistoryNote(this.note, current)) throw fail("noteChanged", "请切回原笔记后继续；这里的准确旧版仍保持只读。");
    return clone(current!);
  }
  private writable(context: HistoryContext): void {
    if (context.readOnly) throw fail("readOnly", "当前笔记没有恢复写入资格；仍可查看保留的准确旧版。");
    if (context.savePending || context.unresolvedSave) throw fail("savePending", "请先处理当前保存的原操作，再准备历史恢复。");
  }
  async refresh(): Promise<boolean> {
    const current = this.context(), generation = ++this.generation;
    const result = await this.host.list({ sessionId: current.sessionId });
    if (generation !== this.generation || !sameHistoryNote(current, this.host.current())) return false;
    if (result.nodeId !== current.nodeId || result.workspaceId !== current.workspaceId) throw fail("identityPending", "历史身份与当前笔记不一致，请先处理身份变化。");
    this.list = clone(result);
    return true;
  }
  async select(versionId: string): Promise<boolean> {
    const current = this.context(), generation = ++this.generation;
    this.selected = null; this.comparison = null;
    const row = this.list?.versions.find(version => version.versionId === versionId);
    if (row && row.status !== "available") throw fail("historyUnavailable", row.message || "所选准确旧版不可用，不能用最新版本替代。");
    const result = await this.host.read({ sessionId: current.sessionId, versionId });
    if (generation !== this.generation || !sameHistoryNote(current, this.host.current())) return false;
    this.validateRead(result, versionId);
    this.selected = clone(result); return true;
  }
  private validateRead(value: HistorySource, versionId: string): void {
    if (value.versionId !== versionId || value.status !== "available" || value.readOnly !== true || typeof value.source !== "string") throw fail("historyUnavailable", "未取得所选版本的准确只读源码；没有取最新版本替代。");
  }
  async compareSaved(): Promise<boolean> {
    const current = this.context(), selected = this.selected;
    if (!selected) throw fail("selectVersion", "请先选择一个可用的已保存版本。");
    const generation = ++this.generation;
    const result = await this.host.compare({ sessionId: current.sessionId, versionId: selected.versionId });
    if (generation !== this.generation || !sameHistoryNote(current, this.host.current())) return false;
    this.validateRead(result.selected, selected.versionId);
    if (result.comparison !== "currentSaved" || typeof result.current.source !== "string" || !result.current.baseId) throw fail("historyUnavailable", "当前已保存比较基线不可用。");
    this.selected = clone(result.selected); this.comparison = clone(result); return true;
  }
  compareDraft(): void {
    const current = this.context();
    if (!this.selected) throw fail("selectVersion", "请先选择一个可用的已保存版本。");
    this.generation++;
    this.comparison = {
      selected: clone(this.selected), current: { source: current.source, serial: current.serial, baseId: current.baseId, path: current.path },
      equal: this.selected.source === current.source, comparison: "draftSnapshot",
    };
  }
  async prepareRestore(): Promise<HistoryRestoreReview> {
    if (this.locked) throw fail("operationPending", "请先查询或重试原恢复操作，不能另建恢复。");
    const context = this.context(); this.writable(context);
    if (!this.selected) throw fail("selectVersion", "请先选择一个可用的已保存版本。");
    const selectedId = this.selected.versionId, generation = ++this.generation;
    const comparison = await this.host.compare({ sessionId: context.sessionId, versionId: selectedId });
    this.validateRead(comparison.selected, selectedId);
    if (generation !== this.generation || this.selected?.versionId !== selectedId || !sameHistoryDraft(context, this.host.current())) throw fail("draftChanged", "准备期间所选版本、草稿或保存基线已变化，请重新查看；输入已保留。");
    if (comparison.comparison !== "currentSaved" || comparison.current.baseId !== context.baseId || comparison.current.source !== context.baseSource || comparison.current.path !== context.path) throw fail("staleBase", "磁盘已保存内容与编辑基线不同；请先比较并处理当前稿，不能覆盖。");
    this.review = { basis: Object.freeze({ ...context, selectedVersionId: selectedId, selectedSource: comparison.selected.source }), comparison: clone(comparison) };
    return this.review;
  }
  cancelRestore(): void {
    if (this.pending) throw fail("operationPending", "原恢复已经发出，请查询原结果；关闭窗口仍保留操作编号。");
    this.review = null;
  }
  async restore(draftAction: HistoryRestoreRequest["draftAction"]): Promise<HistoryRestoreResult> {
    if (this.pending) throw fail("operationPending", "请继续同一原恢复操作。");
    if (!this.review) throw fail("reviewRequired", "请先查看本次恢复预览。");
    const current = this.context(), basis = this.review.basis; this.writable(current);
    if (!sameHistoryDraft(basis, current)) throw fail("draftChanged", "确认前草稿或保存基线已变化，请重新准备；输入已保留。");
    if ((basis.dirty || basis.source !== basis.baseSource) && draftAction !== "retain") throw fail("draftProtectionRequired", "请明确选择保留未保存草稿，再恢复旧版。");
    const request = Object.freeze({ sessionId: basis.sessionId, baseId: basis.baseId, versionId: basis.selectedVersionId, operationId: this.host.id(), draftAction, ...(draftAction === "retain" ? { draftSource: basis.source } : {}) });
    this.pending = { request, basis }; this.operationState = null;
    return this.execute();
  }
  async retryOriginal(): Promise<HistoryRestoreResult> {
    if (!this.pending || this.busy) throw fail("operationPending", "没有可重试的原恢复，或原操作仍在处理。");
    return this.execute();
  }
  private validateRestore(result: HistoryRestoreResult): void {
    const pending = this.pending!;
    if (!result || result.receipt?.operationId !== pending.request.operationId || !["saved", "committed"].includes(result.receipt.status) || !result.receipt.baseId || typeof result.receipt.path !== "string" || result.source !== pending.basis.selectedSource || result.versionId !== pending.basis.selectedVersionId || !Array.isArray(result.protectedDrafts) || result.protectedDrafts.some(draft => !draft || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.path !== "string" || typeof draft.baseKnown !== "boolean")) throw fail("resultUnknown", "恢复回执不完整或不对应原操作；请查询原结果。");
  }
  private accept(result: HistoryRestoreResult): HistoryRestoreResult {
    this.validateRestore(result);
    this.pending = null; this.review = null; this.uncertain = false; this.operationState = "committed";
    return clone(result);
  }
  private async execute(): Promise<HistoryRestoreResult> {
    this.busy = true;
    try { return this.accept(await this.host.restore(clone(this.pending!.request))); }
    catch (error) {
      // Even an ordinary API error can arise while reading evidence after a
      // commit. Only the original operation status can prove no registered save.
      this.rejected = !uncertainHistoryOperation(error);
      this.uncertain = true; this.operationState = null;
      throw error;
    } finally { this.busy = false; }
  }
  async inspectOriginal(): Promise<HistoryRestoreResult | null> {
    if (!this.pending || this.busy) throw fail("operationPending", "没有可查询的原恢复，或查询仍在进行。");
    this.busy = true;
    try {
      const result = await this.host.status({ operationId: this.pending.request.operationId });
      if (result.operationId !== this.pending.request.operationId) throw fail("resultUnknown", "查询回执不对应原恢复操作。");
      this.operationState = result.status;
      if (result.status === "committed") {
        if (!result.result) throw fail("resultUnknown", "原操作已提交但准确回执未取得，请继续查询原结果。");
        return this.accept(result.result);
      }
      if (result.status === "resolved") { this.pending = null; this.review = null; this.uncertain = false; }
      else if (result.status === "missing" && this.rejected) { this.pending = null; this.review = null; this.uncertain = false; }
      else this.uncertain = true;
      return null;
    } catch (error) { this.uncertain = true; throw error; }
    finally { this.busy = false; }
  }
}
