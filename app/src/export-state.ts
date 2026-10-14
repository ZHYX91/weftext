/** Output owns a frozen draft and one artifact ID. It never writes author data. */
import type { DocumentBasis } from "./data-panel";

export type ExportFormat = "docx" | "pdf" | "xlsx" | "native" | "bibtex" | "csl-json" | "ris" | "source";
export interface EmbeddedQuerySelection { occurrenceId: string; queryId: string }
export type ExportStylePolicy = "template" | "source";
export interface OutputLoss { code: string; message: string; location?: string }
export interface FrozenQuery { queryId: string; label: string; rowCount: number; coverage: "complete" | "partial"; freshness: "current" | "stale"; limited?: boolean }
export interface OutputFile { name: string; bytesBase64: string }
export interface TemplateAnchor { name: string; kind: "content" | "cell" | "range" | "table"; label?: string; supported?: boolean; reason?: string }
export interface TemplateInspection { slots: TemplateAnchor[]; warnings: (string | OutputLoss)[] }
export interface OutputBinding { kind: "body" | "title" | "field" | "query" | "text"; field?: string; text?: string }
export interface ExportDraft { format: ExportFormat; stylePolicy: ExportStylePolicy; template?: OutputFile; bindings: Record<string, OutputBinding>; query?: FrozenQuery; reviewMode?: "none" | "comments" | "changes"; includeResolved?: boolean }
export interface ExportRequest {
  reviewMode?: "none" | "comments" | "changes"; includeResolved?: boolean;
  requestId: string; sessionId: string; source: string; serial: number; baseId: string;
  format: ExportFormat; stylePolicy: ExportStylePolicy; template?: OutputFile; bindings?: Record<string, OutputBinding>; queryId?: string;
  embeddedQueries?: EmbeddedQuerySelection[];
}
export interface PreparedOutput { id: string; requestId?: string; filename: string; contentType: string; length: number; digest: string; summary: string; losses: OutputLoss[]; preview?: unknown; status: "prepared" | "confirmed" }
export interface OutputJobFailure { id?: string; requestId?: string; status: "pending" | "failed"; code?: string; message?: string }
export interface ConfirmedOutput { id: string; status: string; digest?: string; length?: number }
export interface OutputTransport {
  id(): string;
  prepare(request: ExportRequest): Promise<PreparedOutput | OutputJobFailure>;
  confirm(request: { id: string; acceptLosses: boolean }): Promise<ConfirmedOutput>;
  download(request: { id: string }): Promise<Blob>;
  digest?(blob: Blob): Promise<string>;
}

export function uncertainOutput(error: unknown): boolean {
  const code = error && typeof error === "object" && "code" in error ? String(error.code) : "";
  return !code || ["unknown", "resultUnknown", "pending", "in-progress", "pending_reconciliation", "recoveryPending"].includes(code);
}
const malformed = (message: string): Error => Object.assign(new Error(message), { code: "resultUnknown" });
const clone = <T>(value: T): T => structuredClone(value);

export class OutputController {
  readonly basis: Readonly<DocumentBasis>;
  draft: ExportDraft;
  prepared: PreparedOutput | null = null;
  confirmed = false;
  received = false;
  busy = false;
  uncertain: "prepare" | "confirm" | "download" | null = null;
  private pending: ExportRequest | null = null;
  private confirmRequest: { id: string; acceptLosses: boolean } | null = null;
  readonly embeddedQueries: readonly EmbeddedQuerySelection[];

  constructor(readonly host: OutputTransport, basis: DocumentBasis, query?: FrozenQuery, embeddedQueries: readonly EmbeddedQuerySelection[] = []) {
    this.basis = Object.freeze(clone(basis));
    this.embeddedQueries = clone(embeddedQueries);
    this.draft = { format: query ? "xlsx" : "docx", stylePolicy: "template", bindings: {}, ...(query ? { query: clone(query) } : {}) };
  }
  get locked(): boolean { return this.busy || this.uncertain !== null; }
  setDraft(value: Partial<ExportDraft>): void {
    if (this.locked) throw new Error("原输出仍在处理或结果待确认，请继续同一操作；输入已保留。");
    const next = { ...this.draft, ...clone(value) };
    if (next.format !== this.draft.format) { next.template = undefined; next.bindings = {}; }
    this.draft = next; this.pending = null; this.confirmRequest = null; this.prepared = null; this.confirmed = false; this.received = false;
  }
  async prepare(): Promise<PreparedOutput> {
    if (this.busy || this.uncertain === "confirm" || this.uncertain === "download") throw new Error("请先确认或下载原输出，不能另行生成。");
    if (this.prepared && !this.pending) return this.prepared;
    if (!this.pending) {
      this.pending = {
        requestId: this.host.id(), sessionId: this.basis.sessionId, source: this.basis.source, serial: this.basis.serial, baseId: this.basis.baseId,
        format: this.draft.format, stylePolicy: this.draft.stylePolicy,
        ...(this.draft.format === "docx" ? { reviewMode: this.draft.reviewMode ?? "none", includeResolved: this.draft.includeResolved ?? false } : {}),
        ...(this.draft.template ? { template: clone(this.draft.template), bindings: clone(this.draft.bindings) } : {}),
        ...(this.draft.query ? { queryId: this.draft.query.queryId } : {}),
        ...(["docx", "pdf"].includes(this.draft.format) ? { embeddedQueries: clone([...this.embeddedQueries]) } : {}),
      };
    }
    this.busy = true;
    try {
      const result = await this.host.prepare(clone(this.pending));
      if (result.requestId !== undefined && result.requestId !== this.pending.requestId) throw malformed("生成回执与原请求不一致；请检查同一输出。");
      if (result.status === "failed") throw Object.assign(new Error(result.message ?? "本次输出生成失败，输入已保留。"), { code: "outputFailed" });
      if (result.status === "pending") throw Object.assign(new Error(result.message ?? "原输出仍在处理，输入与请求标识已保留；可再次检查同一输出。"), { code: "pending" });
      if (!("filename" in result)) throw malformed("生成回执没有可识别的文件状态；请检查同一输出。");
      if (!result.id || !["prepared", "confirmed"].includes(result.status) || !result.filename || !result.contentType || !Number.isSafeInteger(result.length) || result.length < 0 || !/^[a-f0-9]{64}$/i.test(result.digest) || !Array.isArray(result.losses)) throw malformed("生成回执缺少准确文件信息或与原输出不一致；请检查同一输出。");
      const saved = clone(result); this.prepared = saved; this.pending = null; this.confirmed = result.status === "confirmed"; this.received = false; this.uncertain = null; return saved;
    } catch (error) {
      if (uncertainOutput(error)) this.uncertain = "prepare";
      else { this.pending = null; this.uncertain = null; }
      throw error;
    } finally { this.busy = false; }
  }
  async confirm(acceptLosses: boolean): Promise<void> {
    if (!this.prepared || this.busy || this.uncertain === "prepare" || this.uncertain === "download") throw new Error("请先查看准确输出预览。");
    if (this.prepared.losses.length && !acceptLosses && !this.confirmRequest) throw new Error("请阅读并明确接受本次输出的损失，再确认文件。");
    this.confirmRequest ??= { id: this.prepared.id, acceptLosses };
    this.busy = true;
    try {
      const result = await this.host.confirm({ ...this.confirmRequest });
      if (result.id !== this.prepared.id || result.status !== "confirmed" || result.digest !== undefined && result.digest !== this.prepared.digest || result.length !== undefined && result.length !== this.prepared.length) throw malformed("确认回执与预览文件不一致；请确认原输出。");
      this.confirmed = true; this.uncertain = null; this.confirmRequest = null;
    } catch (error) { this.uncertain = uncertainOutput(error) ? "confirm" : null; if (!this.uncertain) this.confirmRequest = null; throw error; }
    finally { this.busy = false; }
  }
  async download(): Promise<Blob> {
    if (!this.prepared || !this.confirmed || this.busy || this.uncertain === "prepare" || this.uncertain === "confirm") throw new Error("请先确认本次预览的同一文件。");
    this.busy = true;
    try {
      const blob = await this.host.download({ id: this.prepared.id });
      if (blob.size !== this.prepared.length || this.host.digest && await this.host.digest(blob) !== this.prepared.digest) throw malformed("下载字节与已确认文件不一致；请重新下载同一文件。");
      this.received = true; this.uncertain = null; return blob;
    } catch (error) { this.uncertain = uncertainOutput(error) ? "download" : null; throw error; }
    finally { this.busy = false; }
  }
}

/** Independent template reads cannot replace a later file or format selection. */
export class OutputReadGate {
  private generation = 0;
  begin(): number { return ++this.generation; }
  accepts(generation: number): boolean { return generation === this.generation; }
  invalidate(): void { this.generation++; }
}

export type ExchangeFormat = "native" | "docx" | "bibtex" | "csl-json" | "ris";
export type ExchangeLoss = string | OutputLoss | { code: string; detail: string; path: string };
export interface BibliographyInspection { records: { conflicts: unknown[]; [key: string]: unknown }[]; losses: OutputLoss[] }
export function bibliographyConflictCount(result: BibliographyInspection): number {
  if (!Array.isArray(result.records) || !Array.isArray(result.losses) || result.records.some(record => !record || !Array.isArray(record.conflicts))) throw new Error("文献映射提案没有完整返回冲突状态，暂不能创建论文笔记。");
  return result.records.reduce((count, record) => count + record.conflicts.length, 0);
}
export interface ExchangeRequest { operationId: string; sessionId: string; baseId: string; root: string; parent: string; name: string; bytesBase64: string; format: ExchangeFormat }
export interface ExchangePlan { operationId: string; planId: string; summary?: string[]; losses: ExchangeLoss[]; changes?: unknown[]; preview?: unknown; contentPreview?: { source: string; truncated: boolean; comments: number; revisions: number; annotations: {author: string; body: string; excerpt: string | null; unlocated: boolean}[] }; status?: string; code?: string; message?: string }
export interface ExchangeReceipt { operationId?: string; planId?: string; status: string; document?: unknown; documents?: unknown[]; path?: string; root?: string; sessionId?: string; source?: string; code?: string; message?: string }
export interface ExchangeTransport { id(): string; prepare(request: ExchangeRequest): Promise<ExchangePlan>; commit(request: { operationId: string; planId: string; acceptLosses: boolean }): Promise<ExchangeReceipt> }

/** An uncertain import keeps its uploaded original and reuses one Core operation. */
export class ExchangeController {
  input: Omit<ExchangeRequest, "operationId"> | null = null;
  plan: ExchangePlan | null = null;
  receipt: ExchangeReceipt | null = null;
  busy = false;
  uncertain: "prepare" | "commit" | null = null;
  private pending: ExchangeRequest | null = null;
  private commitRequest: { operationId: string; planId: string; acceptLosses: boolean } | null = null;
  constructor(readonly host: ExchangeTransport) {}
  get locked(): boolean { return this.busy || this.uncertain !== null; }
  setInput(input: Omit<ExchangeRequest, "operationId">): void {
    if (this.locked) throw new Error("原导入结果待确认，不能重复创建；文件与输入已保留。");
    this.input = clone(input); this.plan = null; this.pending = null; this.commitRequest = null; this.receipt = null;
  }
  async prepare(): Promise<ExchangePlan> {
    if (!this.input || this.busy || this.uncertain === "commit" || this.receipt) throw new Error("请先选择原件和导入位置，或检查原导入结果。");
    if (this.plan && !this.pending) return this.plan;
    this.pending ??= { ...clone(this.input), operationId: this.host.id() };
    this.busy = true;
    try {
      const plan = await this.host.prepare(clone(this.pending));
      if (plan.status === "failed") throw Object.assign(new Error(plan.message ?? "导入计划准备失败，原件与输入已保留。"), { code: "exchangeFailed" });
      if (plan.status === "pending") throw Object.assign(new Error(plan.message ?? "原导入计划仍在处理，可再次检查同一操作。"), { code: "pending" });
      if (plan.operationId !== this.pending.operationId || !plan.planId || !Array.isArray(plan.losses)) throw malformed("导入预览与原操作不一致；原件与输入已保留。");
      this.plan = clone(plan); this.pending = null; this.uncertain = null; return this.plan;
    } catch (error) { if (uncertainOutput(error)) this.uncertain = "prepare"; else { this.pending = null; this.uncertain = null; } throw error; }
    finally { this.busy = false; }
  }
  async commit(acceptLosses: boolean): Promise<ExchangeReceipt> {
    if (!this.plan || this.busy || this.uncertain === "prepare") throw new Error("请先查看导入计划。");
    if (this.receipt) return this.receipt;
    if (this.plan.losses.length && !acceptLosses && !this.commitRequest) throw new Error("请先接受本次导入预览列明的损失。");
    this.commitRequest ??= { operationId: this.plan.operationId, planId: this.plan.planId, acceptLosses };
    this.busy = true;
    try {
      const result = await this.host.commit({ ...this.commitRequest });
      if (result.status === "failed") throw Object.assign(new Error(result.message ?? "导入未完成；原件和计划已保留。"), { code: "exchangeFailed" });
      if (!result || result.status !== "committed" || result.operationId !== undefined && result.operationId !== this.plan.operationId || result.planId !== undefined && result.planId !== this.plan.planId) throw malformed("导入尚未返回实际完成回执；请检查同一操作。");
      this.receipt = clone(result); this.uncertain = null; this.commitRequest = null; return this.receipt;
    } catch (error) { this.uncertain = uncertainOutput(error) ? "commit" : null; if (!this.uncertain) this.commitRequest = null; throw error; }
    finally { this.busy = false; }
  }
}
