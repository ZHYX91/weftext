import { action, decodeProjection, diagnosticText, fieldNames, formatNativeValue, ui, type NoteProjection, type PanelHost } from "./data-panel";
import { bibliographyConflictCount, ExchangeController, OutputController, OutputReadGate, type BibliographyInspection, type DocxImportAnalysis, type ExchangeFormat, type ExchangeLoss, type ExchangeReceipt, type ExportFormat, type EmbeddedQuerySelection, type ExportStylePolicy, type FrozenQuery, type LegacyCapabilities, type OutputBinding, type OutputFile, type TemplateInspection } from "./export-state";
import type { OpenedDocument } from "./types";
import { backendLabel, captionKinds, kindLabel, legacyBackendChoices, selectedCaptionOptions, type CaptionChoice } from "./office-import-options";

const formats: [ExportFormat, string][] = [["docx", "Word 文档（.docx）"], ["doc", "旧版 Word 文档（.doc，外部转换）"], ["wps", "金山文字（.wps，外部转换）"], ["pdf", "PDF 文档（含已显示正文查询）"], ["xlsx", "Excel 工作簿（.xlsx）"], ["native", "Weftext 原生内容包"], ["bibtex", "BibTeX 文献（.bib）"], ["csl-json", "CSL JSON 文献"], ["ris", "RIS 文献（.ris）"], ["source", "准确 Typst 源码（.typ）"]];
const isLegacy = (format: string): boolean => ["doc", "wps"].includes(format);
const isBibliography = (format: string): boolean => ["bibtex", "csl-json", "ris"].includes(format);
const executionNote = "外部转换由当前 Weftext 执行端运行：桌面版使用本机，WebUI 使用当前 Server；浏览器电脑安装的软件不能供 Server 调用。普通 DOCX 不需要 Office。";
let nextId = 0;
function labeled(label: string, control: HTMLElement): HTMLElement {
  const result = ui("div", "output-control"); control.id ||= `output-control-${++nextId}`;
  const heading = ui("label", "data-label", label); heading.htmlFor = control.id; result.append(heading, control); return result;
}
function select(options: [string, string][], current: string): HTMLSelectElement {
  const result = ui("select", "data-input"); for (const [value, label] of options) { const item = ui("option", "", label); item.value = value; result.append(item); } result.value = current; return result;
}
function lossText(loss: ExchangeLoss): string { if (typeof loss === "string") return loss; const message = "message" in loss ? loss.message : loss.detail; const location = "location" in loss ? loss.location : "path" in loss ? loss.path : undefined; return `${message}${location ? `（${location}）` : ""}`; }
function lossList(losses: ExchangeLoss[]): HTMLElement {
  const section = ui("section", "output-losses"); section.append(ui("h3", "", losses.length ? `本次转换的损失与限制 · ${losses.length} 项` : "本次转换未报告损失"));
  if (losses.length) { const list = ui("ul"); losses.forEach(loss => list.append(ui("li", "", lossText(loss)))); section.append(list); } return section;
}
function fileBytes(file: File): Promise<OutputFile> {
  if (file.size > 64 * 1024 * 1024) return Promise.reject(new Error("文件超过 64 MiB 输入上限；请选择较小原件。"));
  return new Promise((resolve, reject) => {
    const reader = new FileReader(); reader.onerror = () => reject(new Error("文件读取失败；原输入已保留。"));
    reader.onload = () => { const value = reader.result; if (typeof value !== "string" || !value.includes(",")) reject(new Error("未取得完整文件字节。")); else resolve({ name: file.name, bytesBase64: value.slice(value.indexOf(",") + 1) }); };
    reader.readAsDataURL(file);
  });
}
async function digest(blob: Blob): Promise<string> { const value = await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()); return [...new Uint8Array(value)].map(byte => byte.toString(16).padStart(2, "0")).join(""); }

/** Safe text-only preview: server-provided markup is never executed. */
function previewContent(value: unknown, label = "查看冻结输入与完整预览"): HTMLElement {
  const details = ui("details", "output-detail"); details.append(ui("summary", "", label));
  const names: Record<string, string> = { format: "格式", target: "排版目标", paragraphCount: "段落数", rowCount: "结果行数", columns: "列顺序", rows: "结果行", title: "标题", text: "文字", body: "正文", sheets: "工作表", name: "名称", path: "路径", scope: "范围", source: "原生源码", sourceBytes: "源码字节数", files: "文件", fileCount: "文件数", coverage: "完整性", freshness: "当前性", limited: "有显示上限", summary: "说明", stylePolicy: "样式策略", publication: "正式出版属性", suggestedKey: "原记录键", originalRecord: "完整原记录", unknownFields: "未知字段（保留）", sources: "字段来源", originalField: "原字段", conflicts: "字段冲突", current: "嵌入的准确原值", imported: "外部标准字段值", key: "字段" };
  const draw = (current: unknown, depth: number): HTMLElement => {
    if (current === null || typeof current !== "object") return ui("span", "output-preview-value", current === null ? "明确为空" : String(current));
    if ("kind" in current && typeof current.kind === "string" && ["none", "text", "int", "float", "decimal", "date", "time", "localDatetime", "duration", "bool", "array", "record"].includes(current.kind)) {
      try { return ui("span", "output-preview-value", formatNativeValue(current as Parameters<typeof formatNativeValue>[0])); } catch { /* Render the exact structure below. */ }
    }
    if (depth > 12) return ui("span", "output-preview-value", diagnosticText(current));
    if (Array.isArray(current)) { const list = ui("ol", "output-preview-list"); current.forEach(item => { const row = ui("li"); row.append(draw(item, depth + 1)); list.append(row); }); return list; }
    const list = ui("dl", "output-preview-record"); Object.entries(current).forEach(([key, item]) => { list.append(ui("dt", "", names[key] ?? key)); const body = ui("dd"); body.append(draw(item, depth + 1)); list.append(body); }); return list;
  };
  details.append(draw(value, 0)); return details;
}

export interface OutputPanelHost extends PanelHost { embeddedQueries?(): EmbeddedQuerySelection[]; imported?(receipt: ExchangeReceipt): Promise<void> }

class OutputDialog {
  readonly dialog = ui("dialog", "modal output-panel");
  private readonly output: OutputController;
  private readonly exchange: ExchangeController;
  private readonly gate = new OutputReadGate();
  private readonly importGate = new OutputReadGate();
  private readonly importReadGate = new OutputReadGate();
  private readonly capabilityGate = new OutputReadGate();
  private readonly format: HTMLSelectElement;
  private readonly style: HTMLSelectElement;
  private readonly review: HTMLSelectElement;
  private readonly resolved = ui("input");
  private readonly outputLegacy = ui("div", "output-legacy");
  private readonly importLegacy = ui("div", "output-legacy");
  private readonly outputBackend = select([], "");
  private readonly importBackend = select([], "");
  private readonly outputLegacyNote = ui("p", "data-help");
  private readonly importLegacyNote = ui("p", "data-help");
  private readonly outputProbe: HTMLButtonElement;
  private readonly importProbe: HTMLButtonElement;
  private readonly cancelOutput: HTMLButtonElement;
  private readonly cancelImport: HTMLButtonElement;
  private readonly template = ui("input", "data-input");
  private readonly templateNote = ui("p", "data-help");
  private readonly bindingHost = ui("div", "output-bindings");
  private readonly outputPreview = ui("div", "output-preview");
  private readonly outputStatus = ui("p", "output-status");
  private readonly acceptOutput = ui("input");
  private readonly prepareButton: HTMLButtonElement;
  private readonly confirmButton: HTMLButtonElement;
  private readonly downloadButton: HTMLButtonElement;
  private readonly importFile = ui("input", "data-input");
  private readonly importFormat = select([["native", "Weftext 原生内容包"], ["docx", "Word 正文、批注与修订（.docx）"], ["doc", "旧版 Word 文档（.doc，外部转换）"], ["wps", "金山文字（.wps，外部转换）"], ["bibtex", "BibTeX 文献（.bib）"], ["csl-json", "CSL JSON 文献"], ["ris", "RIS 文献（.ris）"]], "native");
  private readonly parent = ui("input", "data-input");
  private readonly name = ui("input", "data-input");
  private readonly importFileNote = ui("p", "data-help", "请选择原生内容包。导入会先检查原件并预览新副本。" );
  private readonly importPreview = ui("div", "output-preview");
  private readonly importMapping = ui("div", "output-preview");
  private readonly importStatus = ui("p", "output-status");
  private readonly acceptImport = ui("input");
  private readonly importPrepare: HTMLButtonElement;
  private readonly importCommit: HTMLButtonElement;
  private uploadedImport: OutputFile | null = null;
  private inspection: TemplateInspection | null = null;
  private fields: { key: string; label: string }[] = [];
  private readingTemplate = false;
  private templateError = false;
  private readingImport = false;
  private readingMapping = false;
  private importReadError = false;
  private mappingError = false;
  private mapping: BibliographyInspection | null = null;
  private docxAnalysis: DocxImportAnalysis | null = null;
  private captionChoices: CaptionChoice[] = [];
  private captionError = false;
  private capabilities: LegacyCapabilities | null = null;
  private checkingCapabilities = false;
  private capabilityError = "";
  private cancellingOutput = false;
  private cancellingImport = false;

  constructor(private readonly host: OutputPanelHost, basis: NonNullable<ReturnType<PanelHost["getDocument"]>>, query?: FrozenQuery, embedded: EmbeddedQuerySelection[] = []) {
    this.output = new OutputController({ id: () => crypto.randomUUID(), prepare: request => host.request("/api/export/prepare", request), confirm: request => host.request("/api/export/confirm", request), cancel: request => host.request("/api/export/legacy-cancel", request), download: request => { if (!host.requestBlob) throw new Error("当前环境没有二进制下载入口。"); return host.requestBlob("/api/export/download", request); }, digest }, basis, query, embedded);
    this.exchange = new ExchangeController({ id: () => crypto.randomUUID(), prepare: request => host.request("/api/exchange/prepare", request), commit: request => host.request("/api/exchange/commit", request), cancel: request => host.request("/api/export/legacy-cancel", request) });
    const heading = ui("h2", "", "输出与导入"); heading.id = `output-heading-${++nextId}`; this.dialog.setAttribute("aria-labelledby", heading.id);
    const header = ui("header", "output-header"); header.append(heading, action("关闭", () => this.dialog.close(), "data-button compact"));
    const basisInfo = ui("p", "data-help", `笔记输入已固定：${basis.path} · 草稿版本 ${basis.serial}。关闭后重新打开会保留本次输入与操作。`);
    const sourceDetails = ui("details", "output-detail"); sourceDetails.append(ui("summary", "", "查看本次原生源码"), ui("pre", "output-source", basis.source));
    const outputSection = ui("section", "output-section"); outputSection.append(ui("h3", "", "生成文件"));
    outputSection.append(ui("p", "data-help", `DOCX／DOC／WPS／PDF 随文输出已固定 ${embedded.length} 份正文查询结果，保留原运行状态；缺少结果的位置会明确说明。`));
    if (query) outputSection.append(ui("p", query.coverage === "partial" || query.freshness === "stale" || query.limited ? "data-warning" : "data-help", `查询输入已固定：${query.label} · ${query.rowCount} 行 · ${query.coverage === "complete" ? "完整范围" : "部分范围"}${query.limited ? " · 已有结果上限" : ""}${query.freshness === "stale" ? " · 当前性未确认" : ""}。输出保留本次行与列顺序。`));
    this.format = select(formats, this.output.draft.format); this.format.onchange = () => this.change(() => { const format = this.format.value as ExportFormat; this.output.setDraft({ format, legacyBackend: legacyBackendChoices(this.capabilities, format, false)[0]?.[0] }); this.inspection = null; this.templateError = false; this.readingTemplate = false; this.template.value = ""; this.templateNote.classList.remove("data-warning"); this.gate.invalidate(); this.drawBindings(); });
    this.style = select([["template", "采用输出模板样式（默认）"], ["source", "保留笔记原生样式"]], "template"); this.style.onchange = () => this.change(() => this.output.setDraft({ stylePolicy: this.style.value as ExportStylePolicy }));
    this.template.type = "file"; this.template.onchange = () => { void this.inspectTemplate(); };
    this.acceptOutput.type = "checkbox"; this.acceptOutput.onchange = () => this.drawState();
    const grid = ui("div", "output-grid"); grid.append(labeled("输出格式", this.format), labeled("Office 样式策略", this.style));
    this.review = select([["none", "正文版"], ["comments", "带批注审阅版"], ["changes", "批注及待审文字修订"]], this.output.draft.reviewMode ?? "none");
    this.review.onchange = () => this.change(() => this.output.setDraft({ reviewMode: this.review.value as "none" | "comments" | "changes" }));
    this.resolved.type = "checkbox"; this.resolved.checked = this.output.draft.includeResolved ?? false;
    this.resolved.onchange = () => this.change(() => this.output.setDraft({ includeResolved: this.resolved.checked }));
    grid.append(labeled("Word 审阅内容（需已保存正文）", this.review), labeled("同时导出已解决批注", this.resolved));
    this.outputBackend.onchange = () => this.change(() => this.output.setDraft({ legacyBackend: this.outputBackend.value }));
    this.importBackend.onchange = () => this.refreshImportInput();
    this.outputProbe = action("重新检查执行端软件", () => this.loadLegacyCapabilities(), "data-button compact");
    this.importProbe = action("重新检查执行端软件", () => this.loadLegacyCapabilities(), "data-button compact");
    this.outputLegacy.append(labeled("外部转换软件", this.outputBackend), ui("p", "data-help", executionNote), ui("p", "data-help", "DOC／WPS 先按本次模板、样式、审阅与查询选择生成 DOCX，再由所选软件转换。两步的损失均在预览中列明。"), this.outputLegacyNote, this.outputProbe);
    this.importLegacy.append(labeled("外部转换软件", this.importBackend), ui("p", "data-help", executionNote), ui("p", "data-help", "准备计划时才运行外部转换，保留准确原件和 DOCX 中间文件。未明确配对的题注保留为普通文字，并在损失中说明结构限制。"), this.importLegacyNote, this.importProbe);
    this.prepareButton = action("生成并查看预览", () => this.prepare(), "data-button primary"); this.confirmButton = action("确认本次文件", () => this.confirm(), "data-button"); this.downloadButton = action("下载已确认文件", () => this.download(), "data-button");
    this.cancelOutput = action("取消外部转换", () => this.cancelLegacy(false), "data-button");
    const buttons = ui("div", "output-actions"); buttons.append(this.prepareButton, this.confirmButton, this.downloadButton, this.cancelOutput);
    outputSection.append(grid, this.outputLegacy, labeled("Office 模板文件（可选）", this.template), this.templateNote, this.bindingHost, this.outputPreview, labeled("已阅读并接受上方列明的损失", this.acceptOutput), this.outputStatus, buttons);
    this.outputStatus.setAttribute("role", "status"); this.outputStatus.setAttribute("aria-live", "polite");
    const importSection = ui("details", "output-import"); importSection.append(ui("summary", "", "导入 Word、原生内容包或文献记录"));
    this.importFile.type = "file"; this.importFile.accept = ".zip,.weftext"; this.importFile.onchange = () => { void this.readImport(); };
    this.parent.value = basis.root; this.name.placeholder = "例如：导入副本"; this.name.value = "导入副本";
    for (const input of [this.parent, this.name]) input.oninput = () => this.refreshImportInput();
    this.importFormat.onchange = () => { if (this.exchange.locked) { this.importFormat.value = this.exchange.input?.format ?? "native"; return; } this.importGate.invalidate(); this.readingMapping = false; this.resetImportAnalysis(); this.fillBackends(this.importBackend, this.importFormat.value, true); if (!this.readingImport) { this.refreshImportInput(); void this.inspectImport(); } this.drawState(); };
    this.acceptImport.type = "checkbox"; this.acceptImport.onchange = () => this.drawState();
    this.importPrepare = action("检查原件并预览导入", () => this.prepareImport(), "data-button"); this.importCommit = action("创建已预览的新副本", () => this.commitImport(), "data-button primary");
    this.cancelImport = action("取消外部转换", () => this.cancelLegacy(true), "data-button");
    const importActions = ui("div", "output-actions"); importActions.append(this.importPrepare, this.importCommit, this.cancelImport);
    importSection.append(labeled("导入格式", this.importFormat), this.importLegacy, labeled("原件文件", this.importFile), this.importFileNote, labeled("导入位置", this.parent), labeled("新副本目录名称", this.name), ui("p", "data-help", "原件按本次上传的完整字节检查。创建使用预览中的同一操作；输入及未确认操作会保留。原生包导出的是已纳管的在库内容，导入不恢复旧设备的本机身份或恢复日志。文献导入创建新论文笔记，正式出版属性及原记录随笔记保留。"), this.importMapping, this.importPreview, labeled("已阅读并接受导入计划列明的损失", this.acceptImport), this.importStatus, importActions);
    this.importStatus.setAttribute("role", "status"); this.importStatus.setAttribute("aria-live", "polite");
    this.dialog.append(header, basisInfo, sourceDetails, outputSection, importSection);
    this.dialog.oncancel = event => { event.preventDefault(); this.dialog.close(); };
    document.body.append(this.dialog); this.drawState(); void this.loadFields(); void this.loadLegacyCapabilities();
  }
  open(): void { if (!this.dialog.open) this.dialog.showModal(); }
  private change(run: () => void): void {
    try { run(); this.outputStatus.textContent = "选择已变化，请生成新的冻结文件。"; this.outputPreview.replaceChildren(); this.acceptOutput.checked = false; }
    catch (error) { this.report(error, this.outputStatus); }
    this.drawState();
  }
  private report(error: unknown, target: HTMLElement): void {
    target.classList.add("data-warning"); target.textContent = `${error instanceof Error ? error.message : String(error)} 输入与原件仍保留。`;
  }
  private fillBackends(control: HTMLSelectElement, format: string, importing: boolean, retained = control.value): void {
    const choices = legacyBackendChoices(this.capabilities, format, importing); control.replaceChildren();
    if (!choices.length) choices.push(["", this.checkingCapabilities ? "正在检查当前执行端…" : "当前方向没有可用软件"]);
    else if (retained && !choices.some(choice => choice[0] === retained)) choices.unshift(["", "原软件本方向已不可用，请重新选择"]);
    for (const [value, label] of choices) { const option = ui("option", "", label); option.value = value; control.append(option); }
    control.value = choices.some(choice => choice[0] === retained) ? retained : choices[0][0];
  }
  private legacyAvailable(format: string, backend: string | undefined, importing: boolean): boolean {
    return !isLegacy(format) || legacyBackendChoices(this.capabilities, format, importing).some(choice => choice[0] === backend);
  }
  private legacyNote(format: string, importing: boolean): string {
    if (this.checkingCapabilities) return "正在检查当前执行端软件及本方向的实际支持…";
    if (this.capabilityError) return `本方向暂不可用：${this.capabilityError}`;
    const direction = format === "doc" ? importing ? "docToDocx" : "docxToDoc" : importing ? "wpsToDocx" : "docxToWps";
    const lines = this.capabilities?.backends.map(backend => { const item = backend.directions.find(value => value.direction === direction); return `${backendLabel(backend.id)}${backend.version ? ` ${backend.version}` : ""}：${item?.available ? `可调用${item.contentRoundtripVerified ? "" : "，内容往返尚未验证"}` : "不可用"}${item?.reason ? `（${item.reason}）` : ""}`; }) ?? [];
    return lines.length ? lines.join("；") : "当前执行端未报告可用的旧格式软件。请安装受支持软件后重新检查。";
  }
  private async loadLegacyCapabilities(): Promise<void> {
    if (this.output.locked || this.exchange.locked) return;
    const generation = this.capabilityGate.begin(); this.checkingCapabilities = true; this.capabilityError = ""; this.drawState();
    try {
      const result = await this.host.request<LegacyCapabilities>("/api/export/legacy-capabilities", {}); if (!this.capabilityGate.accepts(generation)) return;
      if (!Array.isArray(result.backends) || result.backends.some(backend => !backend.id || !Array.isArray(backend.directions) || backend.directions.some(direction => typeof direction.direction !== "string" || typeof direction.available !== "boolean"))) throw new Error("执行端没有返回完整的软件方向能力。");
      this.capabilities = result;
      if (!this.output.locked) { this.fillBackends(this.outputBackend, this.output.draft.format, false, this.output.draft.legacyBackend); if (isLegacy(this.output.draft.format) && !this.output.draft.legacyBackend && this.outputBackend.value) this.output.setDraft({ legacyBackend: this.outputBackend.value }); }
      if (!this.exchange.locked) { this.fillBackends(this.importBackend, this.importFormat.value, true); if (isLegacy(this.importFormat.value)) this.refreshImportInput(); }
    } catch (error) { if (this.capabilityGate.accepts(generation)) { this.capabilities = null; this.capabilityError = error instanceof Error ? error.message : String(error); } }
    finally { if (this.capabilityGate.accepts(generation)) { this.checkingCapabilities = false; this.drawState(); } }
  }
  private async cancelLegacy(importing: boolean): Promise<void> {
    const controller = importing ? this.exchange : this.output; const status = importing ? this.importStatus : this.outputStatus;
    if (importing ? this.cancellingImport : this.cancellingOutput) return;
    if (importing) this.cancellingImport = true; else this.cancellingOutput = true; this.drawState();
    try { await controller.cancel(); if (controller.busy || controller.uncertain === "prepare") { status.classList.remove("data-warning"); status.textContent = "已请求取消外部转换，正在等待原操作结果；文件、输入及原操作标识保留。关闭窗口不会取消转换。"; } }
    catch (error) { this.report(error, status); }
    finally { if (importing) this.cancellingImport = false; else this.cancellingOutput = false; this.drawState(); }
  }
  private async loadFields(): Promise<void> {
    try {
      const basis = this.output.basis; const result = await this.host.request<NoteProjection | { projection: NoteProjection }>("/api/data/project", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial });
      this.fields = decodeProjection(result).fields.map(field => ({ key: field.key, label: `${fieldNames[field.key] ?? field.key}（${field.key}）` })); this.drawBindings();
    } catch { this.templateNote.textContent = "当前笔记属性不可读取。正文、标题及实际查询仍可选择；已有映射保留。"; }
  }
  private async inspectTemplate(): Promise<void> {
    const file = this.template.files?.[0]; const retained = this.output.draft.template; if (!file && !retained) return;
    if (this.output.locked) return;
    const generation = this.gate.begin(); const selectedFormat = this.output.draft.format;
    if (!["docx", "doc", "wps", "xlsx"].includes(selectedFormat)) return;
    const format = selectedFormat === "xlsx" ? "xlsx" : "docx";
    this.output.setDraft({ bindings: {} }); this.outputPreview.replaceChildren(); this.acceptOutput.checked = false;
    this.outputStatus.classList.remove("data-warning"); this.outputStatus.textContent = "正在检查本次模板；尚未生成新的输出文件。";
    this.readingTemplate = true; this.templateNote.classList.remove("data-warning"); this.templateNote.textContent = "正在读取模板并检查标准槽位…"; this.drawState();
    try {
      const uploaded = file ? await fileBytes(file) : retained!; if (!this.gate.accepts(generation)) return;
      this.output.setDraft({ template: uploaded, bindings: {} }); this.inspection = null; this.outputPreview.replaceChildren(); this.acceptOutput.checked = false;
      const result = await this.host.request<TemplateInspection>("/api/export/template", { format, bytesBase64: uploaded.bytesBase64 }); if (!this.gate.accepts(generation)) return;
      if (!Array.isArray(result.slots) || !Array.isArray(result.warnings)) throw new Error("模板检查没有返回可识别的标准槽位。");
      this.inspection = result; this.templateError = false; this.templateNote.textContent = "模板标准槽位已检查。未映射的槽位保留模板已有内容。"; this.outputStatus.textContent = "模板已检查。请选择要填入各槽位的实际内容，再生成预览。";
    } catch (error) { if (this.gate.accepts(generation)) { this.templateError = true; this.report(error, this.templateNote); this.outputStatus.classList.add("data-warning"); this.outputStatus.textContent = "本次模板检查失败；尚未生成新的输出文件。请重新检查或移除模板。"; } }
    finally { if (this.gate.accepts(generation)) { this.readingTemplate = false; this.drawBindings(); this.drawState(); } }
  }
  private drawBindings(): void {
    this.bindingHost.replaceChildren();
    if (!this.output.draft.template && !this.templateError) return;
    const title = ui("div", "output-template-heading"); title.append(ui("strong", "", this.output.draft.template?.name ?? this.template.files?.[0]?.name ?? "已选择的模板"), action("移除模板", () => this.change(() => { this.output.setDraft({ template: undefined, bindings: {} }); this.inspection = null; this.templateError = false; this.templateNote.classList.remove("data-warning"); this.template.value = ""; this.drawBindings(); }), "data-button compact")); this.bindingHost.append(title);
    if (this.templateError) this.bindingHost.append(action("重新检查本次模板", () => this.inspectTemplate(), "data-button compact"));
    if (!this.inspection) { this.bindingHost.append(ui("p", "data-warning", "本次模板尚未通过检查，请重新检查或明确移除模板后生成。")); this.drawState(); return; }
    this.inspection.warnings.forEach(warning => this.bindingHost.append(ui("p", "data-warning", lossText(warning))));
    if (!this.inspection.slots.length) this.bindingHost.append(ui("p", "data-help", "模板没有可填写的标准槽位。生成时由转换器检查该格式的实际行为。"));
    for (const slot of this.inspection.slots) {
      if (slot.supported === false) { this.bindingHost.append(ui("p", "data-warning", `${slot.label ?? slot.name}：${slot.reason ?? "此类模板槽位尚不支持填写，保留已有内容。"}`)); continue; }
      const options: [string, string][] = [["", "保留模板已有内容"]];
      if (slot.kind === "content") options.push(["body", "笔记正文"], ["title", "笔记标题"]);
      if (slot.kind === "cell") options.push(["title", "笔记标题"]);
      if (["content", "cell"].includes(slot.kind)) { options.push(...this.fields.map(field => [`field:${field.key}`, `属性 · ${field.label}`] as [string, string])); options.push(["text", "本次固定文字"]); }
      if (["content", "range", "table"].includes(slot.kind) && this.output.draft.query) options.push(["query", "本次固定查询结果"]);
      const existing = this.output.draft.bindings[slot.name]; const value = existing?.kind === "field" ? `field:${existing.field}` : existing?.kind ?? "";
      if (value && !options.some(option => option[0] === value)) options.push([value, `保留已有映射 · ${value}`]);
      const input = select(options, value); const row = labeled(`${slot.label ?? slot.name} · ${slot.kind === "content" ? "内容控件" : slot.kind === "cell" ? "单元格名称" : slot.kind === "range" ? "具名区域" : "表格"}`, input);
      const text = ui("textarea", "data-input"); text.rows = 2; text.value = existing?.text ?? ""; text.hidden = value !== "text"; text.setAttribute("aria-label", `${slot.label ?? slot.name} 的固定文字`); row.append(text);
      const update = (): void => this.change(() => { const bindings = { ...this.output.draft.bindings }; const choice = input.value; if (!choice) delete bindings[slot.name]; else bindings[slot.name] = choice.startsWith("field:") ? { kind: "field", field: choice.slice(6) } : choice === "text" ? { kind: "text", text: text.value } : { kind: choice as OutputBinding["kind"] }; this.output.setDraft({ bindings }); text.hidden = choice !== "text"; });
      input.onchange = update; text.oninput = update; this.bindingHost.append(row);
    }
    this.drawState();
  }
  private async prepare(): Promise<void> {
    if (!this.output.uncertain && !this.output.prepared && !this.legacyAvailable(this.output.draft.format, this.output.draft.legacyBackend, false)) { this.report(new Error("当前方向没有可用的外部转换软件，请检查执行端依赖。"), this.outputStatus); return; }
    this.outputStatus.classList.remove("data-warning"); this.outputStatus.textContent = this.output.uncertain ? "正在检查原输出结果…" : "正在生成并验证冻结文件…";
    const promise = this.output.prepare(); this.drawState();
    try {
      const result = await promise; this.outputPreview.replaceChildren(ui("p", "output-summary", result.summary), ui("p", "data-help", `${result.filename} · ${result.length.toLocaleString("zh-CN")} 字节`), lossList(result.losses));
      if (result.preview !== undefined) this.outputPreview.append(previewContent(result.preview));
      const identity = ui("details", "output-detail"); identity.append(ui("summary", "", "文件信息"), ui("p", "data-help", `文件 ${result.id}`), ui("p", "data-help", `SHA-256 ${result.digest}`)); this.outputPreview.append(identity);
      this.acceptOutput.checked = false; this.outputStatus.textContent = this.output.confirmed ? "该文件已确认，下载将交付同一文件字节。" : "准确文件已生成，等待确认；下载将交付同一文件字节。";
    } catch (error) { this.report(error, this.outputStatus); }
    finally { this.drawState(); }
  }
  private async confirm(): Promise<void> {
    const promise = this.output.confirm(this.acceptOutput.checked); this.outputStatus.textContent = "正在确认已预览的同一文件…"; this.drawState();
    try { await promise; this.outputStatus.classList.remove("data-warning"); this.outputStatus.textContent = "该文件已确认，可以下载。"; } catch (error) { this.report(error, this.outputStatus); } finally { this.drawState(); }
  }
  private async download(): Promise<void> {
    const promise = this.output.download(); this.outputStatus.textContent = "正在接收并核对已确认的文件…"; this.drawState();
    try {
      const blob = await promise; const url = URL.createObjectURL(blob); const link = ui("a"); link.href = url; link.download = this.output.prepared!.filename; document.body.append(link); link.click(); link.remove(); setTimeout(() => URL.revokeObjectURL(url), 60_000);
      this.outputStatus.classList.remove("data-warning"); this.outputStatus.textContent = "已收到并核对确认的文件字节，已交给浏览器下载。";
    } catch (error) { this.report(error, this.outputStatus); } finally { this.drawState(); }
  }
  private async readImport(): Promise<void> {
    const file = this.importFile.files?.[0]; if (!file || this.exchange.locked) return;
    const generation = this.importReadGate.begin(); this.importGate.invalidate(); this.readingMapping = false; this.resetImportAnalysis(); this.readingImport = true; this.importPreview.replaceChildren(); this.drawState();
    try { const uploaded = await fileBytes(file); if (!this.importReadGate.accepts(generation)) return; this.uploadedImport = uploaded; this.importReadError = false; this.importFileNote.textContent = `已保留本次上传原件：${uploaded.name} · ${file.size.toLocaleString("zh-CN")} 字节`; this.refreshImportInput(); }
    catch (error) { if (this.importReadGate.accepts(generation)) { this.importReadError = true; this.report(error, this.importStatus); } }
    finally { if (this.importReadGate.accepts(generation)) { this.readingImport = false; this.drawState(); } }
    if (!this.importReadError && this.importReadGate.accepts(generation)) await this.inspectImport();
  }
  private resetImportAnalysis(): void { this.mapping = null; this.mappingError = false; this.docxAnalysis = null; this.captionChoices = []; this.captionError = false; this.importMapping.replaceChildren(); }
  private async inspectImport(): Promise<void> {
    if (this.exchange.locked || this.readingImport) return;
    const generation = this.importGate.begin(); this.resetImportAnalysis(); this.readingMapping = false;
    const format = this.importFormat.value; const uploaded = this.uploadedImport;
    if (uploaded && !this.importReadError) this.refreshImportInput();
    if (format === "native" || isLegacy(format) || !uploaded || this.importReadError) { this.drawState(); return; }
    this.readingMapping = true; this.importStatus.textContent = format === "docx" ? "正在检查 Word 原件的题注及对象；尚未配对任何题注…" : "正在检查文献完整记录与字段映射…"; this.drawState();
    try {
      if (format === "docx") {
        const analysis = await this.host.request<DocxImportAnalysis>("/api/exchange/inspect", { format, bytesBase64: uploaded.bytesBase64 }); if (!this.importGate.accepts(generation)) return;
        if (!/^[a-f0-9]{64}$/i.test(analysis.contentHash) || !Array.isArray(analysis.captions) || !Array.isArray(analysis.objects) || analysis.captions.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || typeof item.text !== "string" || typeof item.suggestedText !== "string") || analysis.objects.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || !captionKinds.some(kind => kind[0] === item.kind))) throw new Error("Word 题注检查没有返回完整原件标识和可配对对象。");
        this.docxAnalysis = analysis; this.captionChoices = analysis.captions.map(caption => ({ enabled: false, captionBlock: caption.blockIndex, objectBlock: "", kind: "", captionText: caption.suggestedText, position: "" }));
        this.drawCaptionPairings(); this.refreshImportInput(); this.importStatus.classList.remove("data-warning"); this.importStatus.textContent = "Word 原件已检查。题注默认保留普通文字；可明确配对后再准备新副本计划。"; return;
      }
      const result = await this.host.request<BibliographyInspection>("/api/exchange/inspect", { format, bytesBase64: uploaded.bytesBase64 }); if (!this.importGate.accepts(generation)) return;
      const conflicts = bibliographyConflictCount(result); this.mapping = result;
      this.importMapping.replaceChildren(ui("p", "output-summary", `本次原件包含 ${result.records.length} 条文献记录，将分别创建新的论文笔记。`), lossList(result.losses), previewContent(result.records, "查看全部文献记录、字段来源与冲突"));
      if (conflicts) { this.importMapping.prepend(ui("p", "data-warning", `发现 ${conflicts} 项字段冲突，暂不能创建。请查看冲突并修复导入原件，再上传检查。`)); this.importStatus.textContent = "文献映射含有待处理冲突；完整原件与提案已保留。"; }
      else { this.importStatus.classList.remove("data-warning"); this.importStatus.textContent = "文献映射已检查。继续准备实际创建计划，再确认本次损失。"; }
    } catch (error) { if (this.importGate.accepts(generation)) { this.mappingError = format !== "docx"; this.report(error, this.importStatus); this.importMapping.replaceChildren(ui("p", "data-warning", format === "docx" ? "本次无法取得题注配对提案。继续导入将按普通文字保留题注，结构损失以实际导入预览为准。" : "请先取得无冲突的完整文献映射提案。"), action(format === "docx" ? "重新检查 Word 题注" : "重新检查本次文献原件", () => this.inspectImport(), "data-button compact")); } }
    finally { if (this.importGate.accepts(generation)) { this.readingMapping = false; this.drawState(); } }
  }
  private drawCaptionPairings(): void {
    const analysis = this.docxAnalysis; this.importMapping.replaceChildren(); if (!analysis) return;
    this.importMapping.append(ui("h4", "", `Word 题注候选 · ${analysis.captions.length} 段`), ui("p", "data-help", "默认将 SEQ 域显示文字保留为普通正文，并在导入损失中说明编号／题注结构。不会按相邻位置猜所属对象。勾选确认前，请明确选择对象、类别、题注正文和显示位置；选项与本次原件绑定。"));
    if (!analysis.captions.length) { this.importMapping.append(ui("p", "data-help", "本次原件未发现可配对的独立 SEQ 题注段落。")); return; }
    analysis.captions.forEach((caption, index) => {
      const choice = this.captionChoices[index]; const row = ui("fieldset", "output-caption-pairing"); row.append(ui("legend", "", `正文块 ${caption.blockIndex + 1} · SEQ ${caption.sequence}`), ui("p", "output-preview-value", caption.text));
      const object = select([["", "请选择题注所属对象"], ...analysis.objects.filter(item => item.blockIndex !== caption.blockIndex).map(item => [String(item.blockIndex), `正文块 ${item.blockIndex + 1} · ${kindLabel(item.kind)}`] as [string, string])], choice.objectBlock);
      const kind = select([["", "请选择题注类别"], ...captionKinds], choice.kind); const position = select([["", "请选择显示位置"], ["top", "对象上方"], ["bottom", "对象下方"]], choice.position);
      const text = ui("textarea", "data-input"); text.rows = 2; text.value = choice.captionText;
      const enabled = ui("input"); enabled.type = "checkbox"; enabled.checked = choice.enabled;
      const update = (): void => { if (this.exchange.locked) return; Object.assign(choice, { enabled: enabled.checked, objectBlock: object.value, kind: kind.value, captionText: text.value, position: position.value }); this.refreshImportInput(); };
      object.onchange = update; kind.onchange = update; position.onchange = update; text.oninput = update; enabled.onchange = update;
      const grid = ui("div", "output-grid"); grid.append(labeled("所属对象", object), labeled("题注类别", kind), labeled("显示位置", position));
      row.append(grid, labeled("题注正文（请核对，可编辑）", text), labeled("确认此对象、类别、文字和位置，转换为题注", enabled)); this.importMapping.append(row);
    });
  }
  private refreshImportInput(): void {
    if (!this.uploadedImport || this.exchange.locked) return;
    try { const basis = this.output.basis; const format = this.importFormat.value as ExchangeFormat; const docxOptions = format === "docx" ? selectedCaptionOptions(this.docxAnalysis, this.captionChoices) : undefined; this.exchange.setInput({ sessionId: basis.sessionId, baseId: basis.baseId, root: basis.root, parent: this.parent.value, name: this.name.value, format, bytesBase64: this.uploadedImport.bytesBase64, ...(isLegacy(format) ? { legacyBackend: this.importBackend.value } : {}), ...(docxOptions ? { docxOptions } : {}) }); this.captionError = false; this.importPreview.replaceChildren(); this.acceptImport.checked = false; this.importStatus.classList.remove("data-warning"); this.importStatus.textContent = "原件与位置已保留，等待检查与预览。"; } catch (error) { this.captionError = true; this.importPreview.replaceChildren(); this.acceptImport.checked = false; this.report(error, this.importStatus); } this.drawState();
  }
  private async prepareImport(): Promise<void> {
    if (!this.exchange.uncertain && (this.importReadError || this.mappingError || this.captionError || this.readingImport || this.readingMapping || isBibliography(this.importFormat.value) && (!this.mapping || bibliographyConflictCount(this.mapping)))) { this.report(new Error("请先完成原件检查，修正题注选择或文献映射冲突，再准备创建计划。"), this.importStatus); return; }
    if (!this.exchange.uncertain && !this.exchange.plan && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true)) { this.report(new Error("当前方向没有可用的外部转换软件，请检查执行端依赖。"), this.importStatus); return; }
    if (!this.parent.value || !this.name.value) { this.report(new Error("请输入导入位置与新副本名称。"), this.importStatus); return; }
    if (!this.exchange.locked && !this.exchange.plan) { this.refreshImportInput(); if (this.captionError) return; }
    const promise = this.exchange.prepare(); this.importStatus.textContent = "正在检查完整原件并准备导入计划…"; this.drawState();
    try {
      const plan = await promise; this.importPreview.replaceChildren(...(plan.summary ?? []).map(line => ui("p", "output-summary", line)), lossList(plan.losses));
      if (plan.contentPreview) { const p = plan.contentPreview; this.importPreview.append(ui("h4", "", `Word 导入：${p.comments} 条批注，${p.revisions} 条待审修订`), ui("p", "data-help", "正文采用修订前基线；插入和删除保留为待审建议。原 Word 文件同时保存。"), ui("pre", "output-source-preview", p.source.replace(/^#metadata[^\n]*\n/, ""))); if (p.truncated) this.importPreview.append(ui("p", "data-help", "此处仅显示前 12000 字符；导入保留完整正文。")); for (const a of p.annotations) this.importPreview.append(ui("p", "", `${a.author}：${a.body}（${a.unlocated ? "未定位，保留在原件资料上" : a.excerpt || "插入点／回复"}）`)); }
      if (plan.changes) this.importPreview.append(previewContent(plan.changes));
      if (plan.preview !== undefined) this.importPreview.append(previewContent(plan.preview));
      if (plan.legacyConversion !== undefined) this.importPreview.append(previewContent(plan.legacyConversion, "查看外部软件转换与原件、中间文件信息"));
      this.acceptImport.checked = false; this.importStatus.classList.remove("data-warning"); this.importStatus.textContent = "计划已准备；确认后创建上述新副本。";
    }
    catch (error) { this.report(error, this.importStatus); } finally { this.drawState(); }
  }
  private async commitImport(): Promise<void> {
    const promise = this.exchange.commit(this.acceptImport.checked); this.importStatus.textContent = "正在提交已预览的同一导入计划…"; this.drawState();
    try { const receipt = await promise; const opened = await this.deliverReceipt(receipt); this.importStatus.classList.remove("data-warning"); this.importStatus.textContent = opened ? "导入已提交并已打开实际新笔记。" : `导入已提交。实际新副本目录：${receipt.path}`; }
    catch (error) { this.report(error, this.importStatus); } finally { this.drawState(); }
  }
  private async deliverReceipt(receipt: ExchangeReceipt): Promise<boolean> {
    const candidate = receipt.document ?? receipt;
    if (candidate && typeof candidate === "object" && "sessionId" in candidate && "source" in candidate && typeof candidate.sessionId === "string" && typeof candidate.source === "string") {
      if (this.host.created) await this.host.created(candidate as OpenedDocument); else if (this.host.openResolved) await this.host.openResolved(candidate as OpenedDocument); else throw new Error("导入已提交，但当前环境缺少打开真实新副本的入口；保留实际回执。");
      return true;
    } else if (receipt.path) { await this.host.imported?.({ ...receipt, root: receipt.root ?? this.output.basis.root }); return false; }
    else throw Object.assign(new Error("导入已提交，但实际回执没有返回新副本位置；请检查原操作。"), { code: "receiptOpenUnavailable" });
  }
  private drawState(): void {
    const outputFormat = this.output.draft.format; const office = ["docx", "doc", "wps", "xlsx"].includes(outputFormat); const word = ["docx", "doc", "wps"].includes(outputFormat); const outputLocked = this.output.locked || this.readingTemplate;
    this.format.value = outputFormat; this.style.value = this.output.draft.stylePolicy; this.review.value = this.output.draft.reviewMode ?? "none"; this.resolved.checked = this.output.draft.includeResolved ?? false;
    this.format.disabled = outputLocked; this.style.disabled = outputLocked || !office; this.template.disabled = outputLocked || !office; this.template.accept = outputFormat === "xlsx" ? ".xlsx" : ".docx";
    this.review.disabled = outputLocked || !word; this.resolved.disabled = outputLocked || !word;
    this.outputLegacy.hidden = !isLegacy(outputFormat); this.importLegacy.hidden = !isLegacy(this.importFormat.value);
    if (!this.output.locked) this.fillBackends(this.outputBackend, outputFormat, false, this.output.draft.legacyBackend);
    if (!this.exchange.locked) this.fillBackends(this.importBackend, this.importFormat.value, true);
    this.outputBackend.disabled = outputLocked || this.checkingCapabilities || !legacyBackendChoices(this.capabilities, outputFormat, false).length;
    this.importBackend.disabled = this.exchange.locked || this.readingImport || this.readingMapping || this.checkingCapabilities || !legacyBackendChoices(this.capabilities, this.importFormat.value, true).length;
    this.outputLegacyNote.textContent = this.legacyNote(outputFormat, false); this.importLegacyNote.textContent = this.legacyNote(this.importFormat.value, true);
    this.outputLegacyNote.classList.toggle("data-warning", isLegacy(outputFormat) && !this.legacyAvailable(outputFormat, this.output.draft.legacyBackend, false));
    this.importLegacyNote.classList.toggle("data-warning", isLegacy(this.importFormat.value) && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true));
    this.outputProbe.disabled = this.output.locked || this.exchange.locked || this.checkingCapabilities; this.importProbe.disabled = this.outputProbe.disabled;
    if (!this.readingTemplate && !this.output.draft.template && !this.templateNote.classList.contains("data-warning")) this.templateNote.textContent = office ? this.output.draft.format === "xlsx" && !this.output.draft.query ? "未选择文件时输出当前笔记的正式属性。需要结果表时，从查询中的“输出本次查询结果”进入。模板中的具名单元格或区域会列为可选择的槽位。" : "未选择文件时使用内置输出模板。已有标准内容控件或工作簿名称会作为可选择的槽位。" : this.output.draft.format === "native" ? "原生包输出已保存的整个资料库，须先保存所有打开的库内草稿。包含纳管内容与可交付依赖；恢复历史和本机执行证据不随包转移。" : "此格式直接消费冻结原生输入，不使用 Office 模板。";
    this.bindingHost.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | HTMLButtonElement>("input,textarea,select,button").forEach(input => { input.disabled = outputLocked; });
    this.prepareButton.disabled = this.output.busy || this.readingTemplate || this.templateError || !!this.output.draft.template && !this.inspection || this.output.uncertain === "confirm" || this.output.uncertain === "download" || !this.output.uncertain && !this.output.prepared && !this.legacyAvailable(outputFormat, this.output.draft.legacyBackend, false); this.prepareButton.textContent = this.output.uncertain === "prepare" ? "检查原输出结果" : this.output.prepared ? "查看已生成文件" : "生成并查看预览";
    this.acceptOutput.disabled = this.output.busy || this.output.uncertain === "confirm" || this.output.confirmed || !this.output.prepared?.losses.length; this.acceptOutput.parentElement!.hidden = !this.output.prepared?.losses.length;
    this.confirmButton.disabled = !this.output.prepared || this.output.busy || this.output.uncertain === "prepare" || this.output.uncertain === "download" || this.output.confirmed || !!this.output.prepared.losses.length && !this.acceptOutput.checked; this.confirmButton.textContent = this.output.uncertain === "confirm" ? "检查原文件确认结果" : "确认本次文件";
    this.downloadButton.disabled = !this.output.confirmed || this.output.busy || this.output.uncertain === "confirm"; this.downloadButton.textContent = this.output.received ? "再次下载同一文件" : "下载已确认文件";
    const importLocked = this.exchange.locked || this.readingImport || this.readingMapping; this.importFile.disabled = this.exchange.locked; this.importFormat.disabled = this.exchange.locked; this.parent.disabled = importLocked; this.name.disabled = importLocked;
    this.importFile.accept = ({ native: ".zip,.weftext", docx: ".docx", doc: ".doc", wps: ".wps", bibtex: ".bib,.bibtex", "csl-json": ".json", ris: ".ris" } as Record<string, string>)[this.importFormat.value];
    this.importMapping.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | HTMLButtonElement>("input,select,textarea,button").forEach(input => { input.disabled = importLocked; });
    const unmapped = isBibliography(this.importFormat.value) && (!this.mapping || bibliographyConflictCount(this.mapping) > 0);
    this.importPrepare.disabled = !this.uploadedImport || this.importReadError || this.mappingError || this.captionError || unmapped || this.exchange.busy || this.readingImport || this.readingMapping || this.exchange.uncertain === "commit" || !!this.exchange.receipt || !this.exchange.uncertain && !this.exchange.plan && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true); this.importPrepare.textContent = this.exchange.uncertain === "prepare" ? "检查原导入预览" : "检查原件并预览导入";
    this.acceptImport.disabled = this.exchange.busy || this.exchange.uncertain === "commit" || !!this.exchange.receipt || !this.exchange.plan?.losses.length; this.acceptImport.parentElement!.hidden = !this.exchange.plan?.losses.length;
    this.importCommit.disabled = !this.exchange.plan || this.exchange.busy || this.readingImport || this.readingMapping || this.importReadError || this.mappingError || this.captionError || unmapped || this.exchange.uncertain === "prepare" || !this.exchange.receipt && !!this.exchange.plan.losses.length && !this.acceptImport.checked; this.importCommit.textContent = this.exchange.receipt ? "刷新已导入的副本" : this.exchange.uncertain === "commit" ? "检查原导入结果" : "创建已预览的新副本";
    this.cancelOutput.hidden = !isLegacy(outputFormat) || !this.output.requestId || !this.output.busy && this.output.uncertain !== "prepare"; this.cancelOutput.disabled = this.cancellingOutput; this.cancelOutput.textContent = this.cancellingOutput ? "正在请求取消…" : "取消外部转换";
    this.cancelImport.hidden = !isLegacy(this.importFormat.value) || !this.exchange.requestId || !this.exchange.busy && this.exchange.uncertain !== "prepare"; this.cancelImport.disabled = this.cancellingImport; this.cancelImport.textContent = this.cancellingImport ? "正在请求取消…" : "取消外部转换";
  }
}

/** A closed dialog retains frozen inputs, received receipts and uncertain IDs. */
export class OutputPanel {
  private readonly dialogs = new Map<string, OutputDialog>();
  constructor(private readonly host: OutputPanelHost) {}
  async open(query?: FrozenQuery): Promise<void> {
    await this.host.prepareAction?.(); const basis = this.host.getDocument();
    if (!basis) { this.host.notify("请先打开一个笔记，再选择输出或导入。", true); return; }
    const embedded = this.host.embeddedQueries?.() ?? [];
    const key = `${basis.sessionId}/${basis.serial}/${basis.baseId}/${query?.queryId ?? "note"}/${JSON.stringify(embedded)}`;
    let dialog = this.dialogs.get(key); if (!dialog) { dialog = new OutputDialog(this.host, basis, query, embedded); this.dialogs.set(key, dialog); } dialog.open();
  }
}
