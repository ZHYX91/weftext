import { action, formatNativeValue, ui, type Cell } from "./data-panel";
import { OutputReadGate, type OutputFile, type TemplateInspection } from "./export-state";
import { importReceipt, officeChunk, officeFormats, officeOutputPresentation, outputReceipt, type OfficeBinding, type OfficeDraft, type OfficeFormat, type OfficeImportReceipt, type OfficeImportRequest, type OfficeOutputReceipt, type OfficeOutputRequest, type OfficeOutputVerification } from "./hosted-office-state";
import { hostedCachePrefix } from "./hosted-state";
import { nodeSourceHash } from "./hosted-navigation-state";
import { sameHostedLinksIdentity, type HostedLinksIdentity } from "./hosted-knowledge-links-state";
import type { CitationWork } from "./hosted-citations-state";
import type { EmbeddedQueryBasis } from "./embedded-query";

export type HostedOfficeContext = { resource: string; source: string; sourceHash?: string; dirty: boolean; pending: boolean; sessionId?: string; baseId?: string; serial?: number; paused?: boolean };
export interface HostedOfficeHost { identity(): HostedLinksIdentity | null; basis(): Promise<EmbeddedQueryBasis>; embeddedQueries(): { occurrenceId: string; queryId: string }[]; authorizationChanged(message: string, current: { actor: string; deployment: string; generation: number }): void }
export type HostedOfficeQuery = { queryId: string; columns: string[]; rowCount: number; coverage: string };
type Resource = { resource: string; title: string; export: boolean; download: boolean; formats?: OfficeFormat[]; downloads?: OfficeFormat[] };
type Target = { resource: string; title: string; available?: boolean };
type NativeCapability = { enabled: boolean; read: boolean; export: boolean; download: boolean; scope: "savedWholeLibrary" };
type Request = <T>(path: string, body?: unknown) => Promise<T>;
type OutputJob = { request: OfficeOutputRequest; receipt?: OfficeOutputReceipt; confirm?: { requestId: string; digest: string; acceptLosses: boolean }; acceptLosses?: boolean; phase?: "prepare" | "confirm" | "download"; query?: HostedOfficeQuery; received?: boolean };
type ImportJob = { request: OfficeImportRequest; receipt?: OfficeImportReceipt; name: string; commit?: { resource: string; operationId: string; planId: string; acceptLosses: boolean }; acceptLosses?: boolean; phase?: "prepare" | "commit"; notified?: boolean };
type Cache = { draft: OfficeDraft; output?: OutputJob; importDraft: { resource: string; file?: OutputFile }; importJob?: ImportJob };
const initial = (): Cache => ({ draft: { resource: "", format: "docx", reviewMode: "none", includeResolved: false, queryId: "", stylePolicy: "template", bindings: {} }, importDraft: { resource: "" } });
const terminalImport = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
const outputDone = (status?: string) => ["failed", "expired", "notRegistered"].includes(status ?? "");
function select(label: string, options: [string, string][], current: string) {
  const control = ui("select", "data-input"); control.setAttribute("aria-label", label);
  for (const [value, text] of options) { const item = ui("option", "", text); item.value = value; control.append(item); } control.value = current; return control;
}
function checkbox(label: string, checked: boolean, changed: (value: boolean) => void) {
  const input = ui("input"); input.type = "checkbox"; input.checked = checked; input.setAttribute("aria-label", label); input.onchange = () => changed(input.checked);
  const host = ui("label", "data-label", label); host.prepend(input); return host;
}
async function fileBytes(file: File, limit: number): Promise<OutputFile> {
  if (file.size > limit) throw new Error(`文件超过 ${limit / 1024 / 1024} MiB 上限，原件仍保留。`);
  return new Promise((resolve, reject) => {
    const reader = new FileReader(); reader.onerror = () => reject(new Error("文件读取失败，原件仍保留。"));
    reader.onload = () => typeof reader.result === "string" && reader.result.includes(",") ? resolve({ name: file.name, bytesBase64: reader.result.slice(reader.result.indexOf(",") + 1) }) : reject(new Error("未取得完整文件字节。")); reader.readAsDataURL(file);
  });
}
async function sha256(blob: Blob) { return [...new Uint8Array(await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()))].map(b => b.toString(16).padStart(2, "0")).join(""); }
function losses(items: { code: string; message: string; location?: string }[]) {
  const host = ui("section", "output-losses"); host.append(ui("h4", "", items.length ? `转换损失与限制 · ${items.length} 项` : "本次转换未报告损失"));
  const list = ui("ul"); for (const item of items) list.append(ui("li", "", item.message + (item.location ? `（${item.location}）` : ""))); host.append(list); return host;
}
/** Only text and typed cells are drawn; no returned markup is executed. */
function preview(value: unknown, columns: string[] = []) {
  const section = ui("details", "output-detail"); section.append(ui("summary", "", "查看准确输出预览"));
  if (!value || typeof value !== "object") { if (typeof value === "string") section.append(ui("pre", "", value)); return section; }
  const record = value as Record<string, unknown>;
  for (const [key, label] of [["title", "标题"], ["body", "正文"], ["source", "原生源码"], ["summary", "说明"]]) {
    const text = record[key]; if (typeof text === "string") section.append(ui("p", "", label), ui("pre", "", text));
    else if (key === "source" && text && typeof text === "object") { const pair = text as { before?: string; after?: string }; if (typeof pair.before === "string") section.append(ui("p", "", "原文"), ui("pre", "", pair.before)); if (typeof pair.after === "string") section.append(ui("p", "", "拟议结果"), ui("pre", "", pair.after)); }
  }
  const rows = Array.isArray(record.rows) ? record.rows : [];
  const order = Array.isArray(record.columns) && record.columns.every(c => typeof c === "string") ? record.columns as string[] : columns;
  const query = record.query as { rowCount?: number; coverage?: string } | undefined;
  if (query && typeof query.rowCount === "number") section.append(ui("p", "hint", `原查询共 ${query.rowCount} 行${query.coverage === "partial" ? "，结果未完整" : ""}；下方为冻结结果预览。`));
  if (rows.length || order.length) {
    const table = ui("table"); table.setAttribute("aria-label", "冻结查询输出预览");
    const names = order.length ? order : Object.keys((rows[0] as { cells?: Record<string, Cell> } | undefined)?.cells ?? {});
    const header = ui("tr"); for (const name of names) header.append(ui("th", "", name)); table.append(header);
    for (const row of rows) { const cells = (row as { cells?: Record<string, Cell> }).cells ?? {}; const tr = ui("tr"); for (const name of names) { const cell = cells[name]; tr.append(ui("td", "", cell?.value ? formatNativeValue(cell.value) : ({ absent: "未填写", null: "空值", invalid: "无效", unavailable: "不可用", value: "" })[cell?.state ?? "unavailable"])); } table.append(tr); } section.append(table);
  }
  return section;
}

/** Frozen output jobs and actor import transactions survive this tab's reload. */
export class HostedOffice {
  private epoch = 0;
  private key = "";
  private cache: Cache = initial();
  private resources: Resource[] = [];
  private queries: HostedOfficeQuery[] = [];
  private targets: Target[] = [];
  private works: CitationWork[] = [];
  private native?: NativeCapability;
  private unreadable?: string;
  private outputVerified?: OfficeOutputVerification;
  private busy = false;
  private original?: File;
  private templateGate = new OutputReadGate();
  private importGate = new OutputReadGate();
  private message = ui("p", "hint");
  private editor = ui("section");
  private activeUrl?: string;
  constructor(private host: HTMLElement, private request: Request, private context: () => HostedOfficeContext | null, private flush: () => Promise<void>, private changed?: (resource: string) => void, private owner?: HostedOfficeHost) { this.clear(false); }
  clear(removeCache = true) {
    this.epoch++; this.templateGate.invalidate(); this.importGate.invalidate(); this.busy = false; this.cache = initial(); this.resources = []; this.queries = []; this.targets = []; this.works = []; this.native = undefined; this.original = undefined; this.unreadable = undefined; this.outputVerified = undefined;
    if (this.activeUrl) URL.revokeObjectURL(this.activeUrl); this.activeUrl = undefined;
    this.host.replaceChildren(); this.host.hidden = true; this.editor.replaceChildren(); this.message.textContent = ""; this.key = "";
    if (removeCache) for (const key of Object.keys(sessionStorage)) if (key.startsWith("weftext-hosted-office:")) sessionStorage.removeItem(key);
    this.key = "";
  }
  async connect(deployment: string, actor: string, generation: number) {
    this.clear(false); const epoch = this.epoch, identity = this.owner?.identity(); this.key = hostedCachePrefix("office", deployment, actor, generation) + "state";
    if (this.owner && (!identity || identity.deployment !== deployment || identity.actor !== actor || identity.generation !== generation)) { this.clear(false); return; }
    let raw: string | null = null;
    try {
      raw = sessionStorage.getItem(this.key);
      if (raw) { const stored = JSON.parse(raw) as Cache; if (!stored?.draft || !stored.importDraft || !officeFormats.includes(stored.draft.format) || !stored.draft.bindings) throw new Error();
        if (stored.output?.receipt) stored.output.receipt = outputReceipt(stored.output.request.requestId, stored.output.receipt);
        if (stored.importJob?.receipt) importReceipt(stored.importJob.request, stored.importJob.receipt);
        this.cache = stored;
      }
    } catch { this.unreadable = raw ?? ""; }
    this.mount(); this.message.textContent = "正在核对当前身份与输出目录；原请求保留。";
    const results = await Promise.allSettled([this.request<{ resources: Resource[]; queries?: HostedOfficeQuery[]; native?: NativeCapability }>("export"), this.request<{ targets: Target[] }>("office/import-targets"), this.request<{ works: CitationWork[] }>("citations")]);
    if (epoch !== this.epoch || !await this.authority(identity)) return;
    if (results[0].status === "fulfilled") { this.resources = results[0].value.resources; this.queries = results[0].value.queries ?? []; this.native = results[0].value.native; }
    if (results[1].status === "fulfilled") this.targets = results[1].value.targets;
    if (results[2].status === "fulfilled") this.works = results[2].value.works;
    const draft = this.cache.draft;
    if (!draft.resource) draft.resource = this.context()?.resource ?? this.resources[0]?.resource ?? "";
    if (!this.cache.importDraft.resource) this.cache.importDraft.resource = this.targets[0]?.resource ?? "";
    if (results.some(r => r.status === "rejected")) this.message.textContent = "Office 目录未完整取得；原请求保留，可直接核对原结果或重新读取目录。";
    else if (this.cache.output || this.cache.importJob) this.message.textContent = "已恢复本标签页的原 Office 请求；核对原结果后再继续。";
    this.render();
  }
  setQuery(queryId: string, metadata?: Omit<HostedOfficeQuery, "queryId">) {
    if (!this.key || this.unreadable !== undefined) return;
    const previous = this.queries.find(q => q.queryId === queryId);
    const item = { queryId, columns: metadata?.columns ?? previous?.columns ?? [], rowCount: metadata?.rowCount ?? previous?.rowCount ?? 0, coverage: metadata?.coverage ?? previous?.coverage ?? "unknown" };
    const existing = this.queries.findIndex(q => q.queryId === queryId); if (existing < 0) this.queries.push(item); else this.queries[existing] = item;
    if (!this.cache.output && !this.busy) { this.cache.draft.format = "xlsx"; this.cache.draft.queryId = queryId; this.cache.draft.template = undefined; this.cache.draft.inspection = undefined; this.cache.draft.bindings = {}; this.keep(); }
    this.host.hidden = false; this.render();
  }
  private mount() { this.host.hidden = false; this.host.replaceChildren(ui("h2", "", "Office 导入与输出"), ui("p", "hint", "先核对准确预览与转换限制，再确认下载或导入。输入与原请求在本标签页保留。"), this.message, this.editor); }
  private persist() { if (!this.key || this.unreadable !== undefined) throw new Error("Office 恢复记录不可写，请先核对并保留原记录。"); try { sessionStorage.setItem(this.key, JSON.stringify(this.cache)); } catch { throw new Error("浏览器恢复缓存容量不足或不可写；原件和输入仍在本页，请先保留原件。新请求和提交会暂停。"); } }
  private keep() { try { this.persist(); return true; } catch (error) { this.message.textContent = error instanceof Error ? error.message : "Office 恢复缓存不可写；新请求和提交会暂停。"; return false; } }
  private async run(work: () => Promise<void>) {
    if (this.busy) return; this.busy = true; const epoch = this.epoch; this.render();
    try { await work(); } catch (error) { if (epoch === this.epoch) { this.outputVerified = undefined; this.message.textContent = error instanceof Error ? error.message : "操作未完成，原件与原请求已保留，请核对原结果。"; } }
    finally { if (epoch === this.epoch) { this.busy = false; this.render(); } }
  }
  private async authority(identity: HostedLinksIdentity | null | undefined): Promise<boolean> {
    if (!this.owner) return true;
    if (!identity || !sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    const current = await this.request<{ actor: string; deployment: string; generation: number }>("me");
    if (!sameHostedLinksIdentity(identity, this.owner.identity())) return false;
    if (current.actor !== identity.actor || current.deployment !== identity.deployment || current.generation !== identity.generation) { this.owner.authorizationChanged("授权策略已变化，旧输出、文献与缓存展示已清除，请重新连接。", current); return false; }
    return true;
  }
  private allowed(resource: Resource | undefined, format: OfficeFormat): boolean { return !!resource && (resource.formats?.includes(format) ?? (resource.export && format === "docx")); }
  private outputPath(suffix: string): string { return `${["docx", "xlsx"].includes(this.cache.output?.request.format ?? this.cache.draft.format) ? "office/export" : "export"}/${suffix}`; }
  private acceptOutput(result: OfficeOutputReceipt) {
    const job = this.cache.output; if (!job) return;
    const receipt = outputReceipt(job.request.requestId, result, job.receipt); job.receipt = receipt; this.outputVerified = { requestId: job.request.requestId, epoch: this.epoch };
    job.phase = receipt.status === "pending" ? "prepare" : undefined;
    if (receipt.status === "confirmed") delete job.confirm;
    if (this.keep()) this.message.textContent = ({ pending: "原输出仍在生成，请查询原结果。", prepared: "输出已准备，请核对预览与损失后确认。", confirmed: "原输出已确认，可下载同一准确文件。", failed: "原输出已失败；输入保留，可明确准备一份新输出。", expired: "原输出已失效；输入保留，可明确准备一份新输出。", notRegistered: "原输出未登记；原输入保留，可明确准备一份新输出。" })[receipt.status];
  }
  private async prepareOutput() {
    if (this.cache.output) return;
    const epoch = this.epoch, identity = this.owner?.identity(), draft = structuredClone(this.cache.draft);
    const bib = ["bibtex", "csl-json", "ris"].includes(draft.format), citationDraft = bib && (draft.bibliographyScope ?? "citationWorks") === "citationWorks" && draft.bibliographySource !== "saved";
    const body = draft.format === "docx" && (!draft.template || draft.reviewMode !== "none" || Object.values(draft.bindings).some(b => b.kind === "body" || b.kind === "title")) || draft.format === "pdf" || citationDraft;
    if (body) await this.flush();
    if (epoch !== this.epoch) return;
    const resource = this.resources.find(r => r.resource === draft.resource);
    const office = draft.format === "docx" || draft.format === "xlsx";
    const request: OfficeOutputRequest = { requestId: crypto.randomUUID(), format: draft.format, reviewMode: draft.format === "docx" ? draft.reviewMode : "none", includeResolved: draft.format === "docx" && draft.includeResolved, stylePolicy: draft.stylePolicy, bindings: office ? structuredClone(draft.bindings) : {}, ...(office && draft.template ? { template: draft.template } : {}) };
    if (body) {
      const context = this.context();
      if (!this.allowed(resource, draft.format) || !context || context.paused || context.resource !== draft.resource) throw new Error("请打开所选获权正文，再准备当前草稿输出。");
      if (draft.format === "docx" && draft.reviewMode !== "none" && (context.pending || context.dirty)) throw new Error("带批注或修订的输出须使用已保存正文；请先核对原保存结果并保存，再明确生成。个人草稿仍保留。");
      request.resource = draft.resource; request.source = context.source;
      if (draft.format === "pdf" || citationDraft || draft.includeEmbedded) {
        if (!this.owner || !context.sessionId || !context.baseId || context.serial === undefined) throw new Error("准确当前草稿会话不可用，未发送输出。");
        const basis = await this.owner.basis(), current = this.context();
        if (!current || context.resource !== current.resource || context.sessionId !== current.sessionId || context.baseId !== current.baseId || context.serial !== current.serial || context.source !== current.source || basis.sessionId !== context.sessionId || basis.serial !== context.serial || basis.source !== context.source) throw new Error("登记期间正文已变化，草稿保留，请重新准备准确输出。");
        Object.assign(request, { sessionId: context.sessionId, baseId: context.baseId, serial: context.serial, sourceHash: await nodeSourceHash(context.source) });
        if (draft.includeEmbedded && ["docx", "pdf"].includes(draft.format)) { const selections = this.owner.embeddedQueries(); if (!selections.length) throw new Error("当前草稿没有准确已显示的内嵌 Query 结果；未自动重跑查询。"); request.embeddedQueries = structuredClone(selections); }
        const afterHash = this.context(); if (!afterHash || afterHash.source !== context.source || afterHash.serial !== context.serial || afterHash.sessionId !== context.sessionId) throw new Error("冻结期间出现新输入，未发送旧稿输出。");
      }
      if (draft.format === "docx" && Object.values(draft.bindings).some(binding => binding.kind === "query")) {
        if (!draft.queryId) throw new Error("请为模板的查询槽位选择已显示的获权查询结果。");
        request.queryId = draft.queryId;
      }
    } else if (draft.format === "docx") {
      if (!this.allowed(resource, "docx")) throw new Error("原模板字段输出对象不可用，请明确重新选择。"); request.resource = draft.resource;
      if (draft.includeEmbedded) throw new Error("仅字段模板没有准确正文；请取消内嵌结果选择或明确绑定正文后输出。");
      if (Object.values(draft.bindings).some(b => b.kind === "query")) { if (!draft.queryId) throw new Error("请明确选择模板的冻结查询结果。"); request.queryId = draft.queryId; }
    } else if (draft.format === "xlsx") {
      if (draft.queryId) request.queryId = draft.queryId;
      else if (!Object.values(draft.bindings).some(b => b.kind === "field")) throw new Error("请先选择已显示的获权字段查询结果或明确绑定字段。");
      if (Object.values(draft.bindings).some(b => b.kind === "field")) { if (!this.allowed(resource, "xlsx")) throw new Error("原字段输出对象当前不可用，请明确重新选择。"); request.resource = draft.resource; }
    } else if (bib) {
      if ((draft.bibliographyScope ?? "citationWorks") === "selectedWorks") { const ids = draft.works ?? []; if (!ids.length || ids.some(id => !this.works.some(w => w.resource === id && w.status === "ready" && w.export))) throw new Error("请明确选择可导出的获权 Work；原选择保留。"); request.works = [...ids]; }
      else { if (!this.allowed(resource, draft.format)) throw new Error("原文献输出对象当前不可用，请明确重新选择。"); request.resource = draft.resource; }
    } else if (draft.format === "native" && !(this.native?.enabled && this.native.read && this.native.export)) {
      throw new Error("整库原生包需要部署者启用和独立整库读取/导出权限，未扩大当前笔记授权。");
    }
    if (bib) request.bibliographyScope = draft.bibliographyScope ?? "citationWorks";
    if (!await this.authority(identity) || epoch !== this.epoch) return;
    this.cache.output = { request, phase: "prepare", query: this.queries.find(q => q.queryId === draft.queryId) };
    this.persist(); // No request can begin without its recoverable exact identity/input.
    const result = await this.request<OfficeOutputReceipt>(this.outputPath("prepare"), request);
    if (await this.authority(identity) && epoch === this.epoch) this.acceptOutput(result);
  }
  private async recoverOutput() {
    const job = this.cache.output; if (!job) return; const epoch = this.epoch, identity = this.owner?.identity();
    const result = await this.request<OfficeOutputReceipt>(this.outputPath("status"), { requestId: job.request.requestId });
    if (await this.authority(identity) && epoch === this.epoch) this.acceptOutput(result);
  }
  private canDownload(request: OfficeOutputRequest): boolean {
    if (request.format === "native") return !!this.native?.enabled && this.native.read && this.native.export && this.native.download;
    if (request.bibliographyScope === "selectedWorks") return !!request.works?.length && request.works.every(id => this.works.some(w => w.resource === id && w.download));
    if (request.resource) { const resource = this.resources.find(r => r.resource === request.resource); return !!resource && (resource.downloads?.includes(request.format) ?? resource.download); }
    return request.format === "xlsx" && !!request.queryId;
  }
  private async confirmOutput(acceptLosses: boolean) {
    const job = this.cache.output, receipt = job && officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch);
    if (!job || receipt?.status !== "prepared" || !receipt.digest) return;
    if (receipt.losses?.length && !acceptLosses) throw new Error("请先阅读并明确接受本次输出的损失。" );
    const epoch = this.epoch, identity = this.owner?.identity(); job.confirm ??= { requestId: job.request.requestId, digest: receipt.digest, acceptLosses }; job.phase = "confirm"; this.persist();
    const result = await this.request<OfficeOutputReceipt>(this.outputPath("confirm"), job.confirm);
    if (await this.authority(identity) && epoch === this.epoch) this.acceptOutput(result);
  }
  private async downloadOutput() {
    const job = this.cache.output, receipt = job && officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch);
    if (!job || receipt?.status !== "confirmed" || !receipt.digest || receipt.length === undefined) return;
    const epoch = this.epoch, identity = this.owner?.identity(); job.phase = "download"; this.persist(); const chunks: Uint8Array<ArrayBuffer>[] = [];
    for (let offset = 0; offset < receipt.length;) {
      const result = await this.request<Parameters<typeof officeChunk>[0]>(this.outputPath("chunk"), { requestId: job.request.requestId, offset });
      if (epoch !== this.epoch) return;
      const bytes = officeChunk(result, offset, receipt.length, receipt.digest); chunks.push(new Uint8Array(bytes)); offset += bytes.length;
    }
    const blob = new Blob(chunks, { type: receipt.contentType });
    if (blob.size !== receipt.length || await sha256(blob) !== receipt.digest) throw new Error("下载文件与已确认摘要不一致，未交付；可重新下载原输出。" );
    if (!await this.authority(identity) || epoch !== this.epoch) return;
    this.download(blob, receipt.filename!); job.phase = undefined; job.received = true; this.keep(); this.message.textContent = "已下载本次确认的准确文件。";
  }
  private download(blob: Blob, name: string) {
    const url = URL.createObjectURL(blob); this.activeUrl = url; const link = ui("a"); link.href = url; link.download = name; link.click();
    setTimeout(() => { URL.revokeObjectURL(url); if (this.activeUrl === url) this.activeUrl = undefined; }, 1000);
  }
  private acceptImport(result: OfficeImportReceipt) {
    const job = this.cache.importJob; if (!job) return;
    const receipt = importReceipt(job.request, result, job.receipt); job.receipt = receipt; job.phase = ["pending", "notRegistered", "notCommitted", "recoveryPending"].includes(receipt.status) ? job.phase ?? "prepare" : undefined;
    const retained = this.keep();
    if (receipt.status === "committed") {
      const target = this.targets.find(target => target.resource === job.request.resource); if (target) target.available = false;
      if (retained) this.message.textContent = "原 Word 导入已提交，原件与回执保留。请刷新并核对新文档；个人草稿保持保留。";
      if (!job.notified) { job.notified = true; this.keep(); this.changed?.(job.request.resource); }
    } else if (retained) this.message.textContent = receipt.status === "prepared" ? "导入计划已准备，请核对原文和损失后明确提交。" : terminalImport(receipt.status) ? "原导入操作已结束，未冒作提交成功；原件保留。" : "原导入结果尚待核对，保留原件并继续查询原操作。";
  }
  private async prepareImport() {
    if (this.cache.importJob) return;
    const file = this.cache.importDraft.file, resource = this.cache.importDraft.resource;
    if (!file || !this.targets.some(t => t.resource === resource && t.available !== false)) throw new Error("请选择获权的空导入目标与 Word 原件。");
    const epoch = this.epoch; const request = { resource, operationId: crypto.randomUUID(), bytesBase64: file.bytesBase64 };
    this.cache.importJob = { request, name: file.name, phase: "prepare" }; delete this.cache.importDraft.file; this.persist();
    const result = await this.request<OfficeImportReceipt>("office/import/prepare", request);
    if (epoch === this.epoch) this.acceptImport(result);
  }
  private async recoverImport() {
    const job = this.cache.importJob; if (!job) return; const epoch = this.epoch; const request = { resource: job.request.resource, operationId: job.request.operationId };
    let result = await this.request<OfficeImportReceipt>("office/import/operation", request);
    if (epoch !== this.epoch) return;
    if (["notRegistered", "notCommitted"].includes(result.status)) result = await this.request<OfficeImportReceipt>("office/import/settle", request);
    if (epoch === this.epoch) this.acceptImport(result);
  }
  private async commitImport(acceptLosses: boolean) {
    const job = this.cache.importJob, receipt = job?.receipt;
    if (!job || receipt?.status !== "prepared" || !receipt.planId || receipt.commitAllowed === false) return;
    if (receipt.preview?.losses?.length && !acceptLosses) throw new Error("请先接受准确导入计划列明的损失。");
    const epoch = this.epoch; job.commit ??= { resource: job.request.resource, operationId: job.request.operationId, planId: receipt.planId, acceptLosses }; job.phase = "commit"; this.persist();
    const result = await this.request<OfficeImportReceipt>("office/import/commit", job.commit);
    if (epoch === this.epoch) this.acceptImport(result);
  }
  private render() {
    this.editor.replaceChildren();
    if (this.unreadable !== undefined) { this.editor.append(ui("p", "hint", "Office 恢复记录无法解析，原记录只读保留。请先复制原文。"), ui("pre", "", this.unreadable), action("放弃损坏的 Office 恢复记录", () => { sessionStorage.removeItem(this.key); this.unreadable = undefined; this.cache = initial(); this.render(); })); return; }
    this.renderOutput(); this.renderImport();
  }
  private renderOutput() {
    const draft = this.cache.draft, job = this.cache.output; const form = ui("fieldset"); form.disabled = this.busy || !!job;
    form.append(ui("legend", "", "文档、文献与原生输出"));
    const formats: [string, string][] = [["docx", "Word 文档（.docx）"], ["xlsx", "Excel 工作簿（.xlsx）"], ["pdf", "PDF 当前草稿"], ["bibtex", "BibTeX 文献"], ["csl-json", "CSL JSON 文献"], ["ris", "RIS 文献"]];
    if (this.native?.enabled && this.native.read && this.native.export) formats.push(["native", "已保存整库原生包"]);
    else if (draft.format === "native") formats.push(["native", "原整库输出选择（当前不可用）"]);
    const format = select("Office 输出格式", formats, draft.format);
    format.onchange = () => { draft.format = format.value as OfficeDraft["format"]; delete draft.template; delete draft.inspection; draft.bindings = {}; this.templateGate.invalidate(); this.keep(); this.render(); };
    form.append(format);
    if (draft.format === "docx") {
      const options: [string, string][] = this.resources.filter(r => this.allowed(r, "docx")).map(r => [r.resource, r.title || r.resource]);
      if (!options.some(([id]) => id === draft.resource) && draft.resource) options.push([draft.resource, "原选择（当前未在输出目录）"]);
      const resource = select("Word 输出对象", options, draft.resource); resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); }; form.append(resource);
      const mode = select("Word 审阅输出", [["none", "正文版"], ["comments", "带批注审阅版"], ["changes", "带待审文字修订版"]], draft.reviewMode); mode.onchange = () => { draft.reviewMode = mode.value as OfficeDraft["reviewMode"]; this.keep(); };
      form.append(mode, checkbox("包含已解决批注", draft.includeResolved, checked => { draft.includeResolved = checked; this.keep(); }), ui("p", "hint", "正文版可输出当前准确草稿；批注与修订版需要已保存的对应正文。"));
      if (Object.values(draft.bindings).some(binding => binding.kind === "query")) {
        const query = select("Word 模板槽位冻结查询", [["", "选择已显示的获权查询"], ...this.queries.map(q => [q.queryId, `${q.columns.join("、") || "获权字段查询"}${q.coverage === "partial" ? " · 结果未完整" : ""}`] as [string, string])], draft.queryId);
        query.onchange = () => { draft.queryId = query.value; this.keep(); this.render(); }; form.append(query);
      }
    } else if (draft.format === "xlsx") {
      const options: [string, string][] = [["", "选择已显示的字段查询"]];
      for (const q of this.queries) options.push([q.queryId, `${q.columns.join("、") || "获权字段查询"} · ${q.coverage === "unknown" ? "原结果" : `${q.rowCount} 行`}${q.coverage === "partial" ? " · 结果未完整" : ""}`]);
      if (draft.queryId && !options.some(([id]) => id === draft.queryId)) options.push([draft.queryId, "原查询结果（可核对原输出）"]);
      const query = select("Excel 冻结查询结果", options, draft.queryId); query.onchange = () => { draft.queryId = query.value; this.keep(); this.render(); }; form.append(query);
    } else if (draft.format === "native") {
      form.append(ui("p", "hint", "明确输出整库已保存内容（savedWholeLibrary）；不保存当前个人草稿。此能力由部署者与用户整库权限独立授权。"));
      if (!(this.native?.enabled && this.native.read && this.native.export)) form.append(ui("p", "hint", "当前没有整库原生包导出资格。"));
    } else {
      const bib = draft.format !== "pdf";
      if (bib) {
        const scope = select("文献输出范围", [["citationWorks", "所选正文的准确引用 Work"], ["selectedWorks", "明确选择获权 Work"]], draft.bibliographyScope ?? "citationWorks");
        scope.onchange = () => { draft.bibliographyScope = scope.value as "citationWorks" | "selectedWorks"; this.keep(); this.render(); }; form.append(scope);
      }
      if (bib && draft.bibliographyScope === "selectedWorks") {
        const known = new Set(this.works.map(w => w.resource));
        for (const work of this.works) { const choice = checkbox(`${work.title} · ${work.resource}`, draft.works?.includes(work.resource) ?? false, checked => { const selected = new Set(draft.works ?? []); if (checked) selected.add(work.resource); else selected.delete(work.resource); draft.works = [...selected]; this.keep(); this.render(); }); const control = choice.querySelector("input")!; control.disabled = work.status !== "ready" || !work.export; form.append(choice); }
        for (const id of (draft.works ?? []).filter(id => !known.has(id))) form.append(ui("p", "hint", `原选择 ${id} 当前不可用；不会自动替换。`), action(`明确移除原 Work ${id}`, () => { draft.works = draft.works?.filter(value => value !== id); this.keep(); this.render(); }));
        form.append(ui("p", "hint", "仅输出明确选择并独立获权的 publication 数据；不读取 Work 正文。"));
      } else {
        const options: [string, string][] = this.resources.filter(r => this.allowed(r, draft.format)).map(r => [r.resource, r.title]);
        if (draft.resource && !options.some(([id]) => id === draft.resource)) options.push([draft.resource, "原对象（当前不可用）"]);
        const resource = select("输出正文对象", options, draft.resource); resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); }; form.append(resource);
        if (bib) { const source = select("文献正文依据", [["draft", "当前准确草稿"], ["saved", "服务端已保存正文"]], draft.bibliographySource ?? "draft"); source.onchange = () => { draft.bibliographySource = source.value as "draft" | "saved"; this.keep(); }; form.append(source); }
      }
    }
    if (["docx", "pdf"].includes(draft.format)) form.append(checkbox("包含当前已显示的内嵌 Query 冻结结果", draft.includeEmbedded ?? false, checked => { draft.includeEmbedded = checked; this.keep(); }), ui("p", "hint", "未选择时不包含内嵌结果；选择后只使用当前准确原 queryId，不能自动重跑或换新版。"));
    if (["docx", "xlsx"].includes(draft.format)) {
    const style = select("Office 样式策略", [["template", "使用模板样式"], ["source", "保留支持的源码样式"]], draft.stylePolicy); style.onchange = () => { draft.stylePolicy = style.value as OfficeDraft["stylePolicy"]; this.keep(); }; form.append(style);
    const template = ui("input"); template.type = "file"; template.accept = draft.format === "docx" ? ".docx" : ".xlsx"; template.setAttribute("aria-label", "Office 输出模板");
    template.onchange = () => { const file = template.files?.[0]; if (!file) return; const epoch = this.epoch, generation = this.templateGate.begin(), format = draft.format; void this.run(async () => {
      const bytes = await fileBytes(file, 16 * 1024 * 1024); if (epoch !== this.epoch || !this.templateGate.accepts(generation)) return;
      const inspection = await this.request<TemplateInspection>("office/template", { format, bytesBase64: bytes.bytesBase64 });
      if (epoch !== this.epoch || !this.templateGate.accepts(generation)) return;
      draft.template = bytes; draft.inspection = inspection; draft.bindings = {}; if (this.keep()) this.message.textContent = "模板已检查，请明确选择需要填入的槽位。";
    }); };
    form.append(template);
    if (draft.template) form.append(ui("p", "", `模板：${draft.template.name}`), action("移除 Office 模板", () => { delete draft.template; delete draft.inspection; draft.bindings = {}; this.templateGate.invalidate(); this.keep(); this.render(); }));
    for (const warning of draft.inspection?.warnings ?? []) form.append(ui("p", "hint", typeof warning === "string" ? warning : warning.message));
    for (const slot of draft.inspection?.slots ?? []) {
      const options: [string, string][] = [["", "保留模板原内容"], ...(draft.format === "docx" ? [["body", "正文"], ["title", "标题"]] as [string, string][] : []), ["text", "固定文字"], ["query", "冻结查询结果"], ["field", "独立获权原生字段"]];
      const binding = select(`模板槽位 ${slot.name}`, options, draft.bindings[slot.name]?.kind ?? ""); binding.disabled = slot.supported === false;
      binding.onchange = () => { if (!binding.value) delete draft.bindings[slot.name]; else draft.bindings[slot.name] = { kind: binding.value as OfficeBinding["kind"], ...(binding.value === "text" ? { text: draft.bindings[slot.name]?.text ?? "" } : binding.value === "field" ? { field: draft.bindings[slot.name]?.field ?? "" } : {}) }; this.keep(); this.render(); };
      form.append(ui("p", "", slot.label ?? slot.name), binding);
      if (slot.supported === false) form.append(ui("p", "hint", slot.reason ?? "当前槽位位置不支持填入。"));
      if (draft.bindings[slot.name]?.kind === "text") { const text = ui("textarea"); text.value = draft.bindings[slot.name].text ?? ""; text.setAttribute("aria-label", `模板固定文字 ${slot.name}`); text.oninput = () => { draft.bindings[slot.name].text = text.value; this.keep(); }; form.append(text); }
      if (draft.bindings[slot.name]?.kind === "field") { const field = ui("input", "data-input"); field.value = draft.bindings[slot.name].field ?? ""; field.setAttribute("aria-label", `模板原生字段 ${slot.name}`); field.oninput = () => { draft.bindings[slot.name].field = field.value; this.keep(); }; form.append(field, ui("p", "hint", "填写准确 FieldID（如 tasks/status）；字段读取需独立授权，正文可读不授予属性读取。")); }
    }
    if (Object.values(draft.bindings).some(b => b.kind === "field") && draft.format === "xlsx") { const resource = select("字段输出对象", this.resources.filter(r => this.allowed(r, "xlsx")).map(r => [r.resource, r.title]), draft.resource); resource.onchange = () => { draft.resource = resource.value; this.keep(); this.render(); }; form.append(resource); }
    }
    const prepare = action("准备 Office 输出", () => this.run(() => this.prepareOutput()));
    prepare.disabled = draft.format === "native" ? !(this.native?.enabled && this.native.read && this.native.export) : draft.format === "xlsx" ? !draft.queryId && !Object.values(draft.bindings).some(b => b.kind === "field") : ["bibtex", "csl-json", "ris"].includes(draft.format) && draft.bibliographyScope === "selectedWorks" ? !(draft.works?.length) : !this.resources.some(r => r.resource === draft.resource && this.allowed(r, draft.format));
    form.append(prepare); this.editor.append(form);
    if (!job) return;
    const card = ui("section", "output-status"), receipt = officeOutputPresentation(job.request.requestId, job.receipt, this.outputVerified, this.epoch); card.append(ui("h3", "", "原 Office 输出"), ui("p", "", job.request.requestId), ui("p", "", receipt?.summary ?? "原请求已保留，请先按当前授权核对结果；缓存不作为原预览的交付资格。"));
    if (receipt?.filename) card.append(ui("p", "", `${receipt.filename} · ${receipt.length} 字节`));
    if (receipt?.losses) card.append(losses(receipt.losses)); if (receipt?.preview) card.append(preview(receipt.preview, job.query?.columns));
    const accepted = checkbox("接受本次输出的损失", job.confirm?.acceptLosses ?? job.acceptLosses ?? false, value => { job.acceptLosses = value; this.keep(); }); const check = accepted.querySelector("input")!;
    check.disabled = this.busy || !!job.phase || !!job.confirm || receipt?.status !== "prepared";
    const confirm = action("确认 Office 输出", () => this.run(() => this.confirmOutput(check.checked))); confirm.disabled = this.busy || receipt?.status !== "prepared" || !!job.phase;
    const recover = action("核对原 Office 输出", () => this.run(() => this.recoverOutput())); recover.disabled = this.busy;
    const download = action("下载已确认 Office 文件", () => this.run(() => this.downloadOutput())); download.disabled = this.busy || receipt?.status !== "confirmed" || !!job.phase || !this.canDownload(job.request);
    if (!this.canDownload(job.request)) card.append(ui("p", "hint", "当前没有这份原输出的独立下载资格，未把导出权限当下载权限。"));
    card.append(accepted, confirm, recover, download);
    const fresh = action("明确准备一份新 Office 输出", () => { delete this.cache.output; this.outputVerified = undefined; this.keep(); this.render(); }); fresh.disabled = this.busy || !receipt || !!job.phase || !outputDone(receipt?.status) && !job.received;
    card.append(fresh); this.editor.append(card);
  }
  private renderImport() {
    const draft = this.cache.importDraft, job = this.cache.importJob; const form = ui("fieldset"); form.disabled = this.busy || !!job; form.append(ui("legend", "", "Word 导入"));
    const options: [string, string][] = this.targets.map(t => [t.resource, (t.title || t.resource) + (t.available === false ? "（已有文档，不能另行导入）" : "")]); if (draft.resource && !options.some(([id]) => id === draft.resource)) options.push([draft.resource, "原导入目标（当前不在目录）"]);
    const target = select("Word 导入目标", options, draft.resource); target.onchange = () => { draft.resource = target.value; this.keep(); }; form.append(target);
    const file = ui("input"); file.type = "file"; file.accept = ".docx"; file.setAttribute("aria-label", "Word 导入原件");
    file.onchange = () => { const selected = file.files?.[0]; if (!selected) return; const epoch = this.epoch, generation = this.importGate.begin(); this.original = selected; delete draft.file; this.keep(); void this.run(async () => {
      const bytes = await fileBytes(selected, 16 * 1024 * 1024); if (epoch !== this.epoch || !this.importGate.accepts(generation)) return;
      draft.file = bytes; if (this.keep()) this.message.textContent = "Word 原件已读取，准备导入前会核验恢复缓存可写。";
    }); }; form.append(file);
    if (draft.file || this.original) form.append(ui("p", "", `保留原件：${draft.file?.name ?? this.original?.name}`));
    const prepare = action("准备 Word 导入", () => this.run(() => this.prepareImport())); prepare.disabled = !draft.file || !this.targets.some(t => t.resource === draft.resource && t.available !== false); form.append(prepare); this.editor.append(form);
    if (!job) return;
    const card = ui("section", "output-status"); card.append(ui("h3", "", "原 Word 导入操作"), ui("p", "", `原件：${job.name}`));
    const receipt = job.receipt; const summary = receipt?.preview?.summary; if (summary) card.append(ui("p", "", Array.isArray(summary) ? summary.join("；") : summary));
    if (receipt?.preview?.losses) card.append(losses(receipt.preview.losses)); if (receipt?.preview?.source) card.append(preview({ source: receipt.preview.source }));
    const accepted = checkbox("接受本次 Word 导入的损失", job.commit?.acceptLosses ?? job.acceptLosses ?? false, value => { job.acceptLosses = value; this.keep(); }); const check = accepted.querySelector("input")!;
    check.disabled = this.busy || !!job.phase || !!job.commit || receipt?.status !== "prepared" || receipt.commitAllowed === false;
    const commit = action("确认提交 Word 导入", () => this.run(() => this.commitImport(check.checked))); commit.disabled = this.busy || !!job.phase || receipt?.status !== "prepared" || !receipt.planId || receipt.commitAllowed === false;
    const recover = action("核对原 Word 导入操作", () => this.run(() => this.recoverImport())); recover.disabled = this.busy;
    const end = action("结束原 Word 导入操作并保留原件", () => this.run(async () => { const epoch = this.epoch; const result = await this.request<OfficeImportReceipt>("office/import/settle", { resource: job.request.resource, operationId: job.request.operationId }); if (epoch === this.epoch) this.acceptImport(result); })); end.disabled = this.busy || terminalImport(receipt?.status);
    card.append(accepted, commit, recover, end);
    const fresh = action("明确准备另一份 Word 导入", () => { this.cache.importDraft.file = { name: job.name, bytesBase64: job.request.bytesBase64 }; delete this.cache.importJob; this.keep(); this.render(); }); fresh.disabled = this.busy || !terminalImport(receipt?.status); card.append(fresh); this.editor.append(card);
  }
}
