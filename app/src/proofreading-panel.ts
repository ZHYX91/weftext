import type { KnowledgeContext, KnowledgeIntent } from "./knowledge-panel";
import { sameKnowledgeBasis } from "./knowledge-state";
import type { ProcessingDocument } from "./processing-settings-panel";
import { decodeProcessingSettings, defaultProcessingSettings, parseProcessingLanguages, processingLanguageTag, scopeLabel, validateProcessingScope, type ProcessingSettings, type ProcessingRuleScope, type SharedProcessingOrigin } from "./processing-settings";

export interface ProofreadRule { id: string; version: string; category: string; text: string; message: string; replacement: string | null; scope?: ProcessingRuleScope | null }
export interface ProofreadOptions { characters: boolean; typos: boolean; pairs: boolean; sensitive: boolean; rules: ProofreadRule[]; allowlist: string[]; scopedAllowlist?: { text: string; scope: ProcessingRuleScope }[]; processing?: ProcessingSettings; processingOrigin?: SharedProcessingOrigin | null; selection: [number, number] | null }
interface Finding { id: string; category: string; ruleId: string; excerpt: string; message: string; replacement: string | null; executable: boolean; sourceRange: { from: number; to: number; utf16From: number; utf16To: number } | null }
export interface ProofreadResult { coverage: string; findings: Finding[]; warnings: string[]; checkedCharacters: number; existing: string[]; scanId?: string; fragments?: { language: { language: string; script: string | null; region: string | null } | null; nativeLanguage?: { language: string; script: string | null; region: string | null } | null; source: string; checkedRules: string[]; skippedRules: [string, string][] }[] }
export interface ProcessingDefaultsSuggestion { processing: ProcessingDocument | null; source: string; baseId: string; revision?: string }
interface Host { context(): KnowledgeContext | null; scan(context: KnowledgeContext, options: ProofreadOptions): Promise<ProofreadResult>; perform(label: string, intent: KnowledgeIntent): Promise<boolean>; processingDefaults?(): Promise<ProcessingDefaultsSuggestion>; selectSource?(from: number, to: number): void; beforeAction?(): Promise<void>; notify(message: string): void }
const categories: Record<string, string> = { character: "错误字符／符号", typo: "错别字", pair: "成对符号", sensitive: "敏感词" };
export function decodeProofreadingOptions(input: unknown): ProofreadOptions {
  const value = input as ProofreadOptions;
  if (!value || ![value.characters, value.typos, value.pairs, value.sensitive].every(v => typeof v === "boolean") || !Array.isArray(value.rules) || !Array.isArray(value.allowlist) || value.rules.length > 2000 || value.allowlist.length > 2000) throw new Error("规则文件需要四类开关、rules 与 allowlist，每类最多 2000 项。");
  const bytes = (s: string) => new TextEncoder().encode(s).length;
  const ids = new Set<string>();
  for (const r of value.rules) {
    if (!r || typeof r.id !== "string" || !r.id || r.id.startsWith("builtin.") || bytes(r.id) > 100 || ids.has(r.id) || typeof r.version !== "string" || !r.version || bytes(r.version) > 100 || !["typo", "sensitive", "character"].includes(r.category) || typeof r.text !== "string" || !r.text || bytes(r.text) > 512 || typeof r.message !== "string" || bytes(r.message) > 2048 || !(r.replacement === null || typeof r.replacement === "string" && bytes(r.replacement) <= 2048) || r.category === "sensitive" && r.replacement !== null) throw new Error("规则标识、版本、词条或替换格式无效；敏感词只能提示。");
    ids.add(r.id);
    if (r.scope) validateProcessingScope(r.scope);
  }
  if (value.allowlist.some(s => typeof s !== "string" || !s || bytes(s) > 512)) throw new Error("白名单须为非空词条数组。");
  const processing = decodeProcessingSettings(value.processing);
  if (value.processingOrigin && (value.processingOrigin.source !== "shared-library" || typeof value.processingOrigin.baseId !== "string" || !value.processingOrigin.baseId || value.processingOrigin.baseId.length > 512 || value.processingOrigin.revision !== undefined && typeof value.processingOrigin.revision !== "string")) throw new Error("共享词表缺少准确来源依据。");
  if (value.scopedAllowlist !== undefined && (!Array.isArray(value.scopedAllowlist) || value.scopedAllowlist.length > 2000)) throw new Error("白名单超过预算。");
  for (const entry of value.scopedAllowlist ?? []) { if (!entry || typeof entry.text !== "string" || !entry.text || bytes(entry.text) > 512) throw new Error("白名单词条无效。"); validateProcessingScope(entry.scope); }
  return structuredClone({ characters: value.characters, typos: value.typos, pairs: value.pairs, sensitive: value.sensitive, rules: value.rules, allowlist: value.allowlist, scopedAllowlist: value.scopedAllowlist ?? [], processing, ...(value.processingOrigin ? { processingOrigin: value.processingOrigin } : {}), selection: null });
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text = ""): HTMLElementTagNameMap[K] { const e = document.createElement(tag); if (text) e.textContent = text; return e; }
function button(label: string, run: () => void | Promise<void>): HTMLButtonElement { const b = el("button", label); b.type = "button"; b.className = "knowledge-button"; b.onclick = () => { void run(); }; return b; }
function select(values: Record<string, string>): HTMLSelectElement { const s = el("select"); s.className = "knowledge-input"; for (const [value, label] of Object.entries(values)) { const o = el("option", label); o.value = value; s.append(o); } return s; }
function field(label: string, input: HTMLElement): HTMLLabelElement { const e = el("label"); e.className = "knowledge-field"; e.append(el("span", label), input); return e; }

/** Settings are local preferences; findings are ephemeral and annotations use Core. */
export class ProofreadingPanel {
  private options: ProofreadOptions = { characters: true, typos: true, pairs: true, sensitive: true, rules: [], allowlist: [], scopedAllowlist: [], processing: defaultProcessingSettings(), selection: null };
  private result: ProofreadResult | null = null;
  private basis: KnowledgeContext | null = null;
  private checkedOptions: ProofreadOptions | null = null;
  private selected = new Set<string>();
  private category = "all";
  private scope = "document";
  private busy = false;
  private generation = 0;
  private container: HTMLElement | null = null;
  private message = "";
  private suggestion: ProcessingDefaultsSuggestion | null = null;
  private ruleDraft = { category: "typo", text: "", replacement: "", message: "", replace: true };
  constructor(private host: Host, private preferences: { storageKey: string | null; title: string } = { storageKey: "weftext-proofread-options-v1", title: "本地校对" }) {
    try { const saved = preferences.storageKey ? localStorage.getItem(preferences.storageKey) : null; if (saved) this.options = decodeProofreadingOptions(JSON.parse(saved)); } catch { /* Keep independent defaults. */ }
  }
  private save(): void { this.result = null; this.generation++; try { if (this.preferences.storageKey) localStorage.setItem(this.preferences.storageKey, JSON.stringify({ ...this.options, selection: null })); } catch { this.message = "规则暂存在本次会话，无法保存本机设置。"; } }
  dispose(): void { this.generation++; this.container = null; this.result = null; this.basis = null; }
  render(container: HTMLElement): void { this.container = container; this.draw(); }
  private async run(): Promise<void> {
    if (this.busy) return;
    try { await this.host.beforeAction?.(); } catch (error) { this.message = error instanceof Error ? error.message : String(error); this.draw(); return; }
    const context = this.host.context(); if (!context) return;
    const options = structuredClone(this.options);
    if (this.scope === "selection") {
      const s = context.selection; if (!s || s.from >= s.to) { this.message = "请先选择正文或源码中的准确范围。"; this.draw(); return; }
      const encoder = new TextEncoder(); options.selection = [encoder.encode(context.source.slice(0, s.from)).length, encoder.encode(context.source.slice(0, s.to)).length];
    }
    this.result = null; this.basis = null; this.checkedOptions = null; this.selected.clear();
    this.busy = true; const generation = ++this.generation; this.message = "正在校对当前正文…"; this.draw();
    try {
      const result = await this.host.scan(context, options);
      if (generation !== this.generation || !sameKnowledgeBasis(context, this.host.context())) { this.message = "正文或规则已经变化，本次旧结果未作为当前结果。请重新校对。"; return; }
      this.result = result; this.basis = context; this.checkedOptions = options;
      this.selected = new Set(result.findings.filter(f => f.sourceRange && !result.existing.includes(f.id)).map(f => f.id));
      this.message = `检查 ${result.checkedCharacters} 个字符，发现 ${result.findings.length} 项${result.coverage === "complete" ? "" : "（部分范围）"}。`;
    } catch (error) { this.message = error instanceof Error ? error.message : String(error); }
    finally { this.busy = false; this.draw(); }
  }
  private async add(ignore: boolean): Promise<void> {
    if (this.busy) return;
    const context = this.host.context();
    if (!this.result || !this.checkedOptions || !sameKnowledgeBasis(this.basis, context)) { this.message = "正文已变化，请重新校对；已有批注保留。"; this.draw(); return; }
    if (context?.dirty) { this.message = "请先保存正文并重新校对，再保存范围批注；当前结果仍保留。"; this.draw(); return; }
    const findings = this.result.findings.filter(f => this.selected.has(f.id) && (this.category === "all" || f.category === this.category)).map(f => f.id);
    if (!findings.length || findings.length > 200) { this.message = "每批请选择 1 至 200 条可定位结果。"; this.draw(); return; }
    const result = this.result; this.busy = true;
    try {
      const ok = await this.host.perform(ignore ? "忽略所选校对结果并保存记录" : "批量加入校对批注", { kind: "annotationProofread", options: this.checkedOptions, findings, ignore, ...(result.scanId ? { scanId: result.scanId } : {}) });
      if (ok && this.result === result) { result.existing.push(...findings); for (const id of findings) this.selected.delete(id); this.message = ignore ? "忽略记录已保存，可在已解决批注中查看。" : "所选结果已加入批注，可在批注页审阅和回复。"; }
    } catch (error) { this.message = error instanceof Error ? error.message : String(error); }
    finally { this.busy = false; }
    this.draw();
  }
  private draw(): void {
    const root = this.container; if (!root) return; root.replaceChildren();
    root.append(el("h3", this.preferences.title), el("p", "校对可见正文；不会检查代码、公式和链接内容，也不会自动修改文字。"));
    const processing = this.options.processing ?? defaultProcessingSettings();
    const languages = el("input"); languages.value = processing.languages.map(processingLanguageTag).join(", "); languages.placeholder = "留空按原文标记；如 zh-Hans-CN, en-GB"; languages.disabled = this.busy;
    languages.onchange = () => { try { this.options.processing = { ...processing, languages: parseProcessingLanguages(languages.value) }; this.save(); this.message = "本次明确语言选择已更新，旧结果已失效。"; } catch (error) { this.message = error instanceof Error ? error.message : String(error); } this.draw(); };
    root.append(field("本次处理语言／文字体系／地区（多语言可用逗号分隔）", languages), el("p", "明确选择优先于原文标记；留空只用可证明的原文标记。地区和文字体系未知时保留未知。库默认仅供采用为本次选择，不由界面语言推断。方案：通用校对；不翻译、不简繁转换。"));
    if (this.host.processingDefaults) {
      const load = button("读取共享处理建议", async () => {
        if (this.busy) return;
        this.busy = true; this.draw();
        try { this.suggestion = await this.host.processingDefaults!(); this.message = this.suggestion.processing ? "共享建议已读取；检查来源、语言范围与词表后明确采用。" : "当前库没有共享处理建议。"; }
        catch (error) { this.message = error instanceof Error ? error.message : String(error); }
        finally { this.busy = false; this.draw(); }
      }); load.disabled = this.busy; root.append(load);
    }
    if (this.suggestion?.processing) {
      const suggestion = this.suggestion;
      const details = el("details"); details.append(el("summary", `共享建议（来源：${suggestion.source}；版本：${suggestion.revision ?? suggestion.processing!.defaults.version}）`), el("pre", JSON.stringify(suggestion.processing, null, 2)));
      const adopt = button("采用为本次处理选择与词表", () => {
        const document = suggestion.processing!;
        this.options = decodeProofreadingOptions({ ...this.options, processing: document.defaults, processingOrigin: { source: "shared-library", baseId: suggestion.baseId, ...(suggestion.revision ? { revision: suggestion.revision } : {}) }, rules: document.rules, allowlist: [], scopedAllowlist: document.scopedAllowlist }); this.save(); this.message = "已明确采用共享建议；本次处理将持续核对原共享来源和词表版本。未修改文档或共享默认。"; this.suggestion = null; this.draw();
      }); adopt.disabled = this.busy; details.append(adopt); root.append(details);
    }
    for (const [key, label] of [["characters", categories.character], ["typos", categories.typo], ["pairs", categories.pair], ["sensitive", categories.sensitive]] as const) {
      const check = el("input"); check.type = "checkbox"; check.checked = this.options[key]; check.disabled = this.busy;
      check.onchange = () => { this.options[key] = check.checked; this.save(); }; root.append(field(label, check));
    }
    const settings = el("details"); settings.append(el("summary", `词典与白名单（${this.options.rules.length} 条规则）`));
    settings.append(el("p", this.preferences.storageKey ? "词典按准确词条匹配；未配置时会提示。设置暂存当前浏览器地址，重启或换地址前请导出规则文件，以便重新导入。" : "词典按准确词条匹配；规则仅在本次连接保留，可导出规则文件后重新导入。"));
    const file = el("input"); file.type = "file"; file.accept = ".json,application/json";
    file.onchange = () => { void (async () => { try { const input = file.files?.[0]; if (!input) return; if (input.size > 8 * 1024 * 1024) throw new Error("规则文件超过 8 MiB。"); const options = decodeProofreadingOptions(JSON.parse(await input.text())); this.options = options; this.save(); this.message = `已载入 ${options.rules.length} 条规则和 ${options.allowlist.length} 条白名单。`; } catch (error) { this.message = error instanceof Error ? error.message : String(error); } this.draw(); })(); };
    settings.append(field("导入规则 JSON（替换当前规则与白名单）", file), button("导出规则 JSON", () => { const url = URL.createObjectURL(new Blob([JSON.stringify({ ...this.options, selection: null }, null, 2)], { type: "application/json" })); const link = el("a"); link.href = url; link.download = "weftext-proofreading.json"; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); }));
    const cat = select({ typo: categories.typo, sensitive: categories.sensitive, character: categories.character }); cat.value = this.ruleDraft.category;
    cat.onchange = () => { this.ruleDraft.category = cat.value; this.ruleDraft.replace = cat.value !== "sensitive"; replace.checked = this.ruleDraft.replace; replace.disabled = cat.value === "sensitive"; replacement.disabled = cat.value === "sensitive"; };
    const word = el("input"); word.value = this.ruleDraft.text; word.oninput = () => { this.ruleDraft.text = word.value; };
    const replacement = el("input"); replacement.value = this.ruleDraft.replacement; replacement.oninput = () => { this.ruleDraft.replacement = replacement.value; };
    const replace = el("input"); replace.type = "checkbox"; replace.checked = this.ruleDraft.replace; replace.onchange = () => { this.ruleDraft.replace = replace.checked; };
    const explanation = el("input"); explanation.value = this.ruleDraft.message; explanation.oninput = () => { this.ruleDraft.message = explanation.value; };
    settings.append(field("类别", cat), field("查找词语／符号", word), field("提供明确替换（空替换表示删除；敏感词只提示）", replace), field("建议替换为", replacement), field("提示说明", explanation), button("添加规则", () => {
      const d = this.ruleDraft; if (!d.text) { this.message = "词条不能为空。"; this.draw(); return; }
      this.options.rules.push({ id: crypto.randomUUID(), version: "1", category: d.category, text: d.text, message: d.message || "疑似问题，请核对上下文", replacement: d.category !== "sensitive" && d.replace ? d.replacement : null, scope: { universal: true } }); this.save(); this.ruleDraft.text = ""; this.draw();
    }));
    settings.append(el("p", "“添加规则”明确新增通用词条；需要限定语言／地区或词典版本时导出 JSON 编辑 scope 后再导入。敏感词仅表示所选词表匹配，不代表地区法律判定。"));
    for (const rule of this.options.rules) {
      const row = el("p", `${categories[rule.category]}：${rule.text}${rule.replacement !== null ? ` → ${rule.replacement}` : ""}（${scopeLabel(rule.scope ?? undefined)}）`);
      if (!rule.scope) row.append(button("确认此词条适用于所有语言", () => { rule.scope = { universal: true }; this.save(); this.draw(); }));
      row.append(button("删除规则", () => { this.options.rules = this.options.rules.filter(r => r.id !== rule.id); this.save(); this.draw(); })); settings.append(row);
    }
    if (this.options.allowlist.length) {
      settings.append(el("p", `旧白名单待确认：${this.options.allowlist.join("、")}。保留原词条，确认前不用于排除。`), button("确认旧白名单适用于所有语言", () => { this.options.scopedAllowlist = [...this.options.scopedAllowlist ?? [], ...this.options.allowlist.map(text => ({ text, scope: { universal: true } }))]; this.options.allowlist = []; this.save(); this.draw(); }));
    }
    const generalAllowlist = (this.options.scopedAllowlist ?? []).filter(e => e.scope.universal);
    const allow = el("textarea"); allow.rows = 3; allow.value = generalAllowlist.map(e => e.text).join("\n"); allow.onchange = () => { this.options.scopedAllowlist = [...(this.options.scopedAllowlist ?? []).filter(e => !e.scope.universal), ...allow.value.split(/\r?\n/).filter(Boolean).map(text => ({ text, scope: { universal: true } }))]; this.save(); }; settings.append(field("通用白名单：明确适用于所有语言，每行一个准确词条", allow));
    for (const entry of (this.options.scopedAllowlist ?? []).filter(e => !e.scope.universal)) settings.append(el("p", `限定白名单：${entry.text}（${scopeLabel(entry.scope)}）`));
    root.append(settings);
    const scope = select({ document: "整篇正文", selection: "当前选区" }); scope.value = this.scope; scope.onchange = () => { this.scope = scope.value; };
    const start = button(this.busy ? "校对中…" : "开始校对", () => this.run()); start.disabled = this.busy; root.append(field("范围", scope), start);
    const message = el("p", this.message); message.setAttribute("role", "status"); root.append(message);
    if (!this.result) return;
    for (const warning of this.result.warnings) root.append(el("p", warning));
    if (this.result.fragments?.length) {
      const details = el("details"); details.append(el("summary", "本次实际语言来源与规则覆盖"));
      for (const fragment of this.result.fragments) details.append(el("p", `${fragment.language ? processingLanguageTag(fragment.language) : "语言／地区未知"}；原文标记：${fragment.nativeLanguage ? processingLanguageTag(fragment.nativeLanguage) : "未知"}；来源：${({ original: "原文标记", explicit: "本次明确选择", explicitMixed: "多语言明确选择", unknown: "未知" } as Record<string, string>)[fragment.source] ?? fragment.source}；已检查：${fragment.checkedRules.join("、") || "通用内建规则"}；未检查：${fragment.skippedRules.map(([id, reason]) => `${id}（${reason}）`).join("、") || "无词典范围排除"}`));
      root.append(details);
    }
    if (!sameKnowledgeBasis(this.basis, this.host.context())) root.append(el("p", "这些结果对应旧正文，请重新校对后添加批注。"));
    const filter = select({ all: "全部类别", ...categories }); filter.value = this.category; filter.onchange = () => { this.category = filter.value; this.draw(); }; root.append(filter);
    const shown = this.result.findings.filter(f => this.category === "all" || f.category === this.category);
    root.append(button("选择本类可定位结果", () => { for (const f of shown) if (f.sourceRange && !this.result!.existing.includes(f.id)) this.selected.add(f.id); this.draw(); }), button("取消选择", () => { this.selected.clear(); this.draw(); }), button("将所选结果加入批注", () => this.add(false)), button("忽略所选并保存记录", () => this.add(true)));
    for (const f of shown) {
      const row = el("article"); row.className = "knowledge-card";
      const check = el("input"); check.type = "checkbox"; check.checked = this.selected.has(f.id); check.disabled = !f.sourceRange || this.result.existing.includes(f.id); check.onchange = () => { if (check.checked) this.selected.add(f.id); else this.selected.delete(f.id); };
      row.append(field(`${categories[f.category]}：${f.excerpt}`, check), el("p", f.message));
      if (f.replacement !== null) row.append(el("p", `建议：${f.excerpt} → ${f.replacement}${f.executable ? "" : "（请人工核对格式后修改）"}`));
      if (this.result.existing.includes(f.id)) row.append(el("p", "已加入批注或已忽略；保留既有讨论与状态。"));
      if (!f.sourceRange) row.append(el("p", "无准确作者源位置，保留只读提示。"));
      else row.append(button("定位", () => { if (sameKnowledgeBasis(this.basis, this.host.context())) this.host.selectSource?.(f.sourceRange!.utf16From, f.sourceRange!.utf16To); else this.host.notify("结果已过期，请重新校对。"); }));
      root.append(row);
    }
  }
}
