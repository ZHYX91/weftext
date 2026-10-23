import { replaceUiChildren } from "./ui-locale";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { decodeProofreadResult, proofreadFindingUiText, processingMatchUiText, type ProofreadResult } from "./proofreading-wire";
export type { ProofreadResult } from "./proofreading-wire";
import { contentLanguageControl, DEFAULT_CONTENT_LOCALE } from "./content-language";
import type { UiLocale } from "./ui-locale";
import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { KnowledgeContext, KnowledgeIntent } from "./knowledge-panel";
import { sameKnowledgeBasis } from "./knowledge-state";
import type { ProcessingDocument } from "./processing-settings-panel";
import { decodeProcessingSettings, defaultProcessingSettings, parseProcessingLanguages, processingLanguageTag, scopeLabel, validateProcessingScope, type ProcessingSettings, type ProcessingRuleScope, type SharedProcessingOrigin } from "./processing-settings";
export interface ProofreadRule {
    id: string;
    version: string;
    category: string;
    text: string;
    message: string;
    replacement: string | null;
    scope?: ProcessingRuleScope | null;
}
export interface ProofreadOptions {
    characters: boolean;
    typos: boolean;
    pairs: boolean;
    sensitive: boolean;
    rules: ProofreadRule[];
    allowlist: string[];
    scopedAllowlist?: {
        text: string;
        scope: ProcessingRuleScope;
    }[];
    processing?: ProcessingSettings;
    processingOrigin?: SharedProcessingOrigin | null;
    selection: [
        number,
        number
    ] | null;
}
export interface ProcessingDefaultsSuggestion {
    processing: ProcessingDocument | null;
    source: UiText;
    baseId: string;
    revision?: string;
}
interface Host {
    resultScope: "local" | "hosted";
    context(): KnowledgeContext | null;
    scan(context: KnowledgeContext, options: ProofreadOptions): Promise<ProofreadResult>;
    perform(label: UiText, intent: KnowledgeIntent): Promise<boolean>;
    processingDefaults?(): Promise<ProcessingDefaultsSuggestion>;
    selectSource?(from: number, to: number): void;
    beforeAction?(): Promise<void>;
    notify(message: UiText): void;
}
const categories: Record<string, UiText> = { character: uiMessage("proofreading.categoryCharacter"), typo: uiMessage("proofreading.categoryTypo"), pair: uiMessage("proofreading.categoryPair"), sensitive: uiMessage("proofreading.categorySensitive") };
export function decodeProofreadingOptions(input: unknown): ProofreadOptions {
    const value = input as ProofreadOptions;
    if (!value || ![value.characters, value.typos, value.pairs, value.sensitive].every(v => typeof v === "boolean") || !Array.isArray(value.rules) || !Array.isArray(value.allowlist) || value.rules.length > 2000 || value.allowlist.length > 2000)
        throw new UiError(uiMessage("proofreading.optionsShapeInvalid"));
    const bytes = (s: string) => new TextEncoder().encode(s).length;
    const ids = new Set<string>();
    for (const r of value.rules) {
        if (!r || typeof r.id !== "string" || !r.id || r.id.startsWith("builtin.") || bytes(r.id) > 100 || ids.has(r.id) || typeof r.version !== "string" || !r.version || bytes(r.version) > 100 || !["typo", "sensitive", "character"].includes(r.category) || typeof r.text !== "string" || !r.text || bytes(r.text) > 512 || typeof r.message !== "string" || bytes(r.message) > 2048 || !(r.replacement === null || typeof r.replacement === "string" && bytes(r.replacement) <= 2048) || r.category === "sensitive" && r.replacement !== null)
            throw new UiError(uiMessage("proofreading.ruleFormatInvalid"));
        ids.add(r.id);
        if (r.scope)
            validateProcessingScope(r.scope);
    }
    if (value.allowlist.some(s => typeof s !== "string" || !s || bytes(s) > 512))
        throw new UiError(uiMessage("proofreading.allowlistFormatInvalid"));
    const processing = decodeProcessingSettings(value.processing);
    if (value.processingOrigin && (value.processingOrigin.source !== "shared-library" || typeof value.processingOrigin.baseId !== "string" || !value.processingOrigin.baseId || value.processingOrigin.baseId.length > 512 || value.processingOrigin.revision !== undefined && typeof value.processingOrigin.revision !== "string"))
        throw new UiError(uiMessage("proofreading.sharedDictionaryOriginMissing"));
    if (value.scopedAllowlist !== undefined && (!Array.isArray(value.scopedAllowlist) || value.scopedAllowlist.length > 2000))
        throw new UiError(uiMessage("proofreading.allowlistBudgetExceeded"));
    for (const entry of value.scopedAllowlist ?? []) {
        if (!entry || typeof entry.text !== "string" || !entry.text || bytes(entry.text) > 512)
            throw new UiError(uiMessage("proofreading.allowlistTermInvalid"));
        validateProcessingScope(entry.scope);
    }
    return structuredClone({ characters: value.characters, typos: value.typos, pairs: value.pairs, sensitive: value.sensitive, rules: value.rules, allowlist: value.allowlist, scopedAllowlist: value.scopedAllowlist ?? [], processing, ...(value.processingOrigin ? { processingOrigin: value.processingOrigin } : {}), selection: null });
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, text: UiText = ""): HTMLElementTagNameMap[K] {
    const e = document.createElement(tag);
    if (text)
        setUiText(e, text);
    return e;
}
function button(label: UiText, run: () => void | Promise<void>): HTMLButtonElement { const b = el("button", label); b.type = "button"; b.className = "knowledge-button"; b.onclick = () => { void run(); }; return b; }
function select(values: Record<string, UiText>): HTMLSelectElement {
    const s = el("select");
    s.className = "knowledge-input";
    for (const [value, label] of Object.entries(values)) {
        const o = el("option", label);
        o.value = value;
        s.append(o);
    }
    return s;
}
function field(label: UiText, input: HTMLElement): HTMLLabelElement { const e = el("label"); e.className = "knowledge-field"; e.append(el("span", label), input); return e; }
const emptyRuleDraft = () => ({ category: "typo", text: "", replacement: "", message: "", replace: true });
/** Settings are local preferences; findings are ephemeral and annotations use Core. */
export class ProofreadingPanel {
    private readonly closeGuard = new ProposalCloseGuard();
    private options: ProofreadOptions = { characters: true, typos: true, pairs: true, sensitive: true, rules: [], allowlist: [], scopedAllowlist: [], processing: defaultProcessingSettings(), selection: null };
    private result: ProofreadResult | null = null;
    private basis: KnowledgeContext | null = null;
    private checkedOptions: ProofreadOptions | null = null;
    private selected = new Set<string>();
    private category = "all";
    private annotationLocale: UiLocale = DEFAULT_CONTENT_LOCALE;
    private scope = "document";
    private busy = false;
    private generation = 0;
    private container: HTMLElement | null = null;
    private message: UiText = "";
    private suggestion: ProcessingDefaultsSuggestion | null = null;
    private ruleDraft = emptyRuleDraft();
    private ruleDraftId: string | null = null;
    private allowDraft: string | null = null;
    private languagesDraft: string | null = null;
    private savedOptions = structuredClone(this.options);
    private lifetime = 0;
    private get unresolved(): boolean {
        return this.busy || JSON.stringify(this.options) !== JSON.stringify(this.savedOptions)
            || JSON.stringify(this.ruleDraft) !== JSON.stringify(emptyRuleDraft())
            || this.allowDraft !== null || this.languagesDraft !== null;
    }
    assertCloseReady(): void { this.closeGuard.assertReady(this.unresolved); }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.unresolved); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    private action(label: UiText, run: () => void | Promise<void>): HTMLButtonElement {
        return button(label, () => { this.closeGuard.assertMutable(); return run(); });
    }
    constructor(private host: Host, private preferences: {
        storageKey: string | null;
        title: UiText;
    } = { storageKey: "weftext-proofread-options-v1", title: uiMessage("proofreading.localTitle") }) {
        try {
            const saved = preferences.storageKey ? localStorage.getItem(preferences.storageKey) : null;
            if (saved)
                this.options = decodeProofreadingOptions(JSON.parse(saved));
        }
        catch { /* Keep independent defaults. */ }
        this.savedOptions = structuredClone(this.options);
    }
    private save(): boolean {
        this.closeGuard.assertMutable();
        this.result = null;
        this.generation++;
        try {
            // Session-only settings remain this owner's volatile author input.
            if (!this.preferences.storageKey) return false;
            localStorage.setItem(this.preferences.storageKey, JSON.stringify({ ...this.options, selection: null }));
            this.savedOptions = structuredClone(this.options);
            return true;
        }
        catch {
            this.message = uiMessage("proofreading.preferencesSaveFailed");
            return false;
        }
    }
    private discardInput(): void {
        this.closeGuard.assertMutable();
        if (this.busy) return;
        this.options = structuredClone(this.savedOptions);
        this.ruleDraft = emptyRuleDraft();
        this.ruleDraftId = null;
        this.allowDraft = null;
        this.languagesDraft = null;
        this.result = null;
        this.generation++;
        this.message = "";
        this.draw();
    }
    dispose(): void { this.generation++; this.lifetime++; if (this.container) replaceUiChildren(this.container); this.container = null; this.result = null; this.basis = null; this.checkedOptions = null; this.selected.clear(); this.message = ""; this.busy = false; }
    render(container: HTMLElement): void { this.closeGuard.assertMutable(); this.closeGuard.bind(container); this.container = container; this.draw(); }
    private async run(): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.busy)
                return;
            const lifetime = this.lifetime;
            this.busy = true;
            try {
                const beforeActionGeneration = this.generation;
                try {
                    await this.host.beforeAction?.();
                }
                catch (error) {
                    if (beforeActionGeneration !== this.generation) return;
                    this.message = error instanceof Error ? errorUiText(error) : String(error);
                    this.draw();
                    return;
                }
                if (beforeActionGeneration !== this.generation) return;
                const context = this.host.context();
                if (!context)
                    return;
                const options = structuredClone(this.options);
                if (this.scope === "selection") {
                    const s = context.selection;
                    if (!s || s.from >= s.to) {
                        this.message = uiMessage("proofreading.selectionRequired");
                        this.draw();
                        return;
                    }
                    const encoder = new TextEncoder();
                    options.selection = [encoder.encode(context.source.slice(0, s.from)).length, encoder.encode(context.source.slice(0, s.to)).length];
                }
                this.result = null;
                this.basis = null;
                this.checkedOptions = null;
                this.selected.clear();
                const generation = ++this.generation;
                this.message = uiMessage("proofreading.scanning");
                this.draw();
                try {
                    const raw = await this.host.scan(context, options);
                    if (generation !== this.generation) return;
                    if (!sameKnowledgeBasis(context, this.host.context())) {
                        this.message = uiMessage("proofreading.scanBasisChanged");
                        return;
                    }
                    const result = decodeProofreadResult(raw, this.host.resultScope);
                    this.result = result;
                    this.basis = context;
                    this.checkedOptions = options;
                    this.selected = new Set(result.findings.filter(f => f.sourceRange && !result.existing.includes(f.id)).map(f => f.id));
                    this.message = displayMessage("proofreading.scanSummary", { p0: result.checkedCharacters, p1: result.findings.length, p2: uiDisplayParameter(result.coverage === "complete" ? "" : uiMessage("proofreading.partialScopeSuffix")) });
                }
                catch (error) {
                    if (generation === this.generation && sameKnowledgeBasis(context, this.host.context())) this.message = errorUiText(error);
                }
                }
            finally {
                if (lifetime === this.lifetime) { this.busy = false; this.draw(); }
            }
            });
    }
    private async add(ignore: boolean): Promise<void> {
        return this.closeGuard.run(async () => {
            if (this.busy)
                return;
            const context = this.host.context();
            if (!this.result || !this.checkedOptions || !sameKnowledgeBasis(this.basis, context)) {
                this.message = uiMessage("proofreading.annotationBasisChanged");
                this.draw();
                return;
            }
            if (context?.dirty) {
                this.message = uiMessage("proofreading.saveBeforeAnnotations");
                this.draw();
                return;
            }
            const findings = this.result.findings.filter(f => this.selected.has(f.id) && (this.category === "all" || f.category === this.category)).map(f => f.id);
            if (!findings.length || findings.length > 200) {
                this.message = uiMessage("proofreading.annotationBatchRange");
                this.draw();
                return;
            }
            const result = this.result;
            const generation = this.generation;
            const lifetime = this.lifetime;
            this.busy = true;
            try {
                const ok = await this.host.perform(ignore ? uiMessage("proofreading.ignoreOperation") : uiMessage("proofreading.annotateOperation"), { kind: "annotationProofread", annotationLocale: this.annotationLocale, options: this.checkedOptions, findings, ignore, ...(result.scanId ? { scanId: result.scanId } : {}) });
                if (ok && generation === this.generation && this.result === result) {
                    result.existing.push(...findings);
                    for (const id of findings)
                        this.selected.delete(id);
                    this.message = ignore ? uiMessage("proofreading.ignoreSaved") : uiMessage("proofreading.annotationsSaved");
                }
            }
            catch (error) {
                if (generation === this.generation && this.result === result) this.message = errorUiText(error);
            }
            finally {
                if (lifetime === this.lifetime) this.busy = false;
            }
            if (lifetime === this.lifetime) this.draw();
            });
    }
    private draw(): void {
        const root = this.container;
        if (!root)
            return;
        replaceUiChildren(root);
        root.append(el("h3", this.preferences.title), el("p", uiMessage("proofreading.help")));
        const processing = this.options.processing ?? defaultProcessingSettings();
        const languages = el("input");
        const savedLanguages = processing.languages.map(processingLanguageTag).join(", ");
        languages.value = this.languagesDraft ?? savedLanguages;
        setUiAttr(languages, "placeholder", uiMessage("proofreading.languagesPlaceholder"));
        languages.disabled = this.busy;
        languages.oninput = () => { this.closeGuard.assertMutable(); this.languagesDraft = languages.value === savedLanguages ? null : languages.value; };
        languages.onchange = () => {
            this.closeGuard.assertMutable();
            this.languagesDraft = languages.value;
            try {
                this.options.processing = { ...processing, languages: parseProcessingLanguages(languages.value) };
                if (this.save()) {
                    this.languagesDraft = null;
                    this.message = uiMessage("proofreading.languagesUpdated");
                }
            }
            catch (error) {
                this.message = error instanceof Error ? errorUiText(error) : String(error);
            }
            this.draw();
        };
        root.append(field(uiMessage("proofreading.languagesLabel"), languages), el("p", uiMessage("proofreading.languageIndependenceHelp")));
        if (this.host.processingDefaults) {
            const load = this.action(uiMessage("proofreading.loadSharedSuggestions"), () => this.closeGuard.run(async () => {
                if (this.busy)
                    return;
                const lifetime = this.lifetime;
                this.busy = true;
                this.draw();
                try {
                    const suggestion = await this.host.processingDefaults!();
                    if (lifetime !== this.lifetime) return;
                    this.suggestion = suggestion;
                    this.message = this.suggestion.processing ? uiMessage("proofreading.sharedSuggestionsLoaded") : uiMessage("proofreading.noSharedSuggestions");
                }
                catch (error) {
                    if (lifetime === this.lifetime) this.message = error instanceof Error ? errorUiText(error) : String(error);
                }
                finally {
                    if (lifetime === this.lifetime) { this.busy = false; this.draw(); }
                }
            }));
            load.disabled = this.busy;
            root.append(load);
        }
        if (this.suggestion?.processing) {
            const suggestion = this.suggestion;
            const details = el("details");
            details.append(el("summary", displayMessage("proofreading.sharedSuggestionsSummary", { p0: uiDisplayParameter(suggestion.source), p1: uiDisplayParameter(suggestion.revision ?? suggestion.processing!.defaults.version) })), el("pre", JSON.stringify(suggestion.processing, null, 2)));
            const adopt = this.action(uiMessage("proofreading.adoptSharedSuggestions"), () => {
                const document = suggestion.processing!;
                this.options = decodeProofreadingOptions({ ...this.options, processing: document.defaults, processingOrigin: { source: "shared-library", baseId: suggestion.baseId, ...(suggestion.revision ? { revision: suggestion.revision } : {}) }, rules: document.rules, allowlist: [], scopedAllowlist: document.scopedAllowlist });
                if (this.save()) {
                    this.message = uiMessage("proofreading.sharedSuggestionsAdopted");
                    this.suggestion = null;
                }
                this.draw();
            });
            adopt.disabled = this.busy;
            details.append(adopt);
            root.append(details);
        }
        for (const [key, label] of [["characters", categories.character], ["typos", categories.typo], ["pairs", categories.pair], ["sensitive", categories.sensitive]] as const) {
            const check = el("input");
            check.type = "checkbox";
            check.checked = this.options[key];
            check.disabled = this.busy;
            check.onchange = () => { this.closeGuard.assertMutable(); this.options[key] = check.checked; this.save(); };
            root.append(field(label, check));
        }
        const settings = el("details");
        settings.append(el("summary", displayMessage("proofreading.dictionarySummary", { p0: this.options.rules.length })));
        settings.append(el("p", this.preferences.storageKey ? uiMessage("proofreading.localDictionaryHelp") : uiMessage("proofreading.sessionDictionaryHelp")));
        const file = el("input");
        file.type = "file";
        file.accept = ".json,application/json";
        file.disabled = this.busy;
        file.onchange = () => {
            this.closeGuard.assertMutable();
            if (this.busy) return;
            void this.closeGuard.run(async () => {
                const lifetime = this.lifetime;
                const generation = this.generation;
                this.busy = true;
                try {
                    const input = file.files?.[0];
                    if (!input)
                        return;
                    if (input.size > 8 * 1024 * 1024)
                        throw new UiError(uiMessage("proofreading.ruleFileTooLarge"));
                    const options = decodeProofreadingOptions(JSON.parse(await input.text()));
                    if (lifetime !== this.lifetime || generation !== this.generation) return;
                    this.options = options;
                    if (this.save()) this.message = displayMessage("proofreading.rulesImported", { p0: options.rules.length, p1: options.allowlist.length });
                }
                catch (error) {
                    if (lifetime === this.lifetime) this.message = error instanceof Error ? errorUiText(error) : String(error);
                }
                finally {
                    if (lifetime === this.lifetime) { this.busy = false; this.draw(); }
                }
            });
        };
        settings.append(field(uiMessage("proofreading.importRulesLabel"), file), this.action(uiMessage("proofreading.exportRules"), () => { const url = URL.createObjectURL(new Blob([JSON.stringify({ ...this.options, selection: null }, null, 2)], { type: "application/json" })); const link = el("a"); link.href = url; link.download = "weftext-proofreading.json"; link.click(); setTimeout(() => URL.revokeObjectURL(url), 1000); }));
        const cat = select({ typo: categories.typo, sensitive: categories.sensitive, character: categories.character });
        cat.value = this.ruleDraft.category;
        cat.onchange = () => { this.closeGuard.assertMutable(); this.ruleDraft.category = cat.value; this.ruleDraft.replace = cat.value !== "sensitive"; replace.checked = this.ruleDraft.replace; replace.disabled = cat.value === "sensitive"; replacement.disabled = cat.value === "sensitive"; };
        const word = el("input");
        word.value = this.ruleDraft.text;
        word.oninput = () => { this.closeGuard.assertMutable(); this.ruleDraft.text = word.value; };
        const replacement = el("input");
        replacement.value = this.ruleDraft.replacement;
        replacement.oninput = () => { this.closeGuard.assertMutable(); this.ruleDraft.replacement = replacement.value; };
        const replace = el("input");
        replace.type = "checkbox";
        replace.checked = this.ruleDraft.replace;
        replace.onchange = () => { this.closeGuard.assertMutable(); this.ruleDraft.replace = replace.checked; };
        const explanation = el("input");
        explanation.value = this.ruleDraft.message;
        explanation.oninput = () => { this.closeGuard.assertMutable(); this.ruleDraft.message = explanation.value; };
        settings.append(field(uiMessage("proofreading.ruleCategory"), cat), field(uiMessage("proofreading.ruleTerm"), word), field(uiMessage("proofreading.ruleHasReplacement"), replace), field(uiMessage("proofreading.ruleReplacement"), replacement), field(uiMessage("proofreading.ruleMessage"), explanation), this.action(uiMessage("proofreading.addRule"), () => {
            const d = this.ruleDraft;
            if (!d.text) {
                this.message = uiMessage("proofreading.ruleTermRequired");
                this.draw();
                return;
            }
            // A failed preference write keeps the same authored rule for retry.
            const id = this.ruleDraftId ??= crypto.randomUUID();
            this.options.rules = this.options.rules.filter(rule => rule.id !== id);
            this.options.rules.push({ id, version: "1", category: d.category, text: d.text, message: d.message || "疑似问题，请核对上下文", replacement: d.category !== "sensitive" && d.replace ? d.replacement : null, scope: { universal: true } });
            if (this.save()) { this.ruleDraft = emptyRuleDraft(); this.ruleDraftId = null; }
            this.draw();
        }));
        settings.append(el("p", uiMessage("proofreading.addRuleScopeHelp")));
        for (const rule of this.options.rules) {
            const row = el("p", () => `${uiTextValue(categories[rule.category])}：${uiTextValue(rule.text)}${uiTextValue(rule.replacement !== null ? ` → ${rule.replacement}` : "")}（${uiTextValue(scopeLabel(rule.scope ?? undefined))}）`);
            if (!rule.scope)
                row.append(this.action(uiMessage("proofreading.confirmUniversalRule"), () => { rule.scope = { universal: true }; this.save(); this.draw(); }));
            row.append(this.action(uiMessage("proofreading.deleteRule"), () => { this.options.rules = this.options.rules.filter(r => r.id !== rule.id); this.save(); this.draw(); }));
            settings.append(row);
        }
        if (this.options.allowlist.length) {
            settings.append(el("p", displayMessage("proofreading.unconfirmedAllowlist", { p0: uiDisplayParameter(this.options.allowlist.join("、")) })), this.action(uiMessage("proofreading.confirmUniversalAllowlist"), () => { this.options.scopedAllowlist = [...this.options.scopedAllowlist ?? [], ...this.options.allowlist.map(text => ({ text, scope: { universal: true } }))]; this.options.allowlist = []; this.save(); this.draw(); }));
        }
        const generalAllowlist = (this.options.scopedAllowlist ?? []).filter(e => e.scope.universal);
        const allow = el("textarea");
        allow.rows = 3;
        const savedAllowlist = generalAllowlist.map(e => e.text).join("\n");
        allow.value = this.allowDraft ?? savedAllowlist;
        allow.oninput = () => { this.closeGuard.assertMutable(); this.allowDraft = allow.value === savedAllowlist ? null : allow.value; };
        allow.onchange = () => {
            this.closeGuard.assertMutable();
            this.allowDraft = allow.value;
            this.options.scopedAllowlist = [...(this.options.scopedAllowlist ?? []).filter(e => !e.scope.universal), ...allow.value.split(/\r?\n/).filter(Boolean).map(text => ({ text, scope: { universal: true } }))];
            if (this.save()) this.allowDraft = null;
        };
        settings.append(field(uiMessage("proofreading.universalAllowlistLabel"), allow));
        for (const entry of (this.options.scopedAllowlist ?? []).filter(e => !e.scope.universal))
            settings.append(el("p", displayMessage("proofreading.scopedAllowlistEntry", { p0: uiDisplayParameter(entry.text), p1: uiDisplayParameter(scopeLabel(entry.scope)) })));
        const discard = this.action(uiMessage("processingSettings.discardInput"), () => this.discardInput());
        discard.disabled = this.busy;
        settings.append(discard);
        root.append(settings);
        const scope = select({ document: uiMessage("proofreading.scopeDocument"), selection: uiMessage("proofreading.scopeSelection") });
        scope.value = this.scope;
        scope.onchange = () => { this.closeGuard.assertMutable(); this.scope = scope.value; };
        const start = this.action(this.busy ? uiMessage("proofreading.scanBusy") : uiMessage("proofreading.startScan"), () => this.run());
        start.disabled = this.busy;
        root.append(field(uiMessage("queryPanel.scopeLabel"), scope), start);
        const message = el("p", this.message);
        message.setAttribute("role", "status");
        root.append(message);
        if (!this.result)
            return;
        for (const warning of this.result.warnings)
            root.append(el("p", warning));
        if (this.result.fragments?.length) {
            const details = el("details");
            details.append(el("summary", uiMessage("proofreading.languageCoverageTitle")));
            for (const fragment of this.result.fragments)
                details.append(el("p", displayMessage("proofreading.languageCoverageEntry", { p0: uiDisplayParameter(fragment.language ? processingLanguageTag(fragment.language) : uiMessage("proofreading.languageRegionUnknown")), p1: uiDisplayParameter(fragment.nativeLanguage ? processingLanguageTag(fragment.nativeLanguage) : uiMessage("common.unknown")), p2: uiDisplayParameter(({ original: uiMessage("proofreading.languageOriginSource"), explicit: uiMessage("proofreading.languageOriginExplicit"), explicitMixed: uiMessage("proofreading.languageOriginExplicitMixed"), unknown: uiMessage("common.unknown") } as Record<string, UiText>)[fragment.source] ?? fragment.source), p3: uiDisplayParameter(fragment.checkedRules.join("、") || uiMessage("proofreading.universalBuiltinRules")), p4: uiDisplayParameter(fragment.skippedRules.length ? joinUiText(fragment.skippedRules.map(([id, reason]) => () => `${id}（${uiTextValue(processingMatchUiText(reason))}）`), "、") : uiMessage("proofreading.noDictionaryScopeExclusions")) })));
            root.append(details);
        }
        if (!sameKnowledgeBasis(this.basis, this.host.context()))
            root.append(el("p", uiMessage("proofreading.oldResultsHelp")));
        const filter = select({ all: uiMessage("proofreading.allCategories"), ...categories });
        filter.value = this.category;
        filter.onchange = () => { this.closeGuard.assertMutable(); this.category = filter.value; this.draw(); };
        root.append(filter);
        root.append(contentLanguageControl("annotation", this.annotationLocale, value => { this.closeGuard.assertMutable(); this.annotationLocale = value; }));
        const shown = this.result.findings.filter(f => this.category === "all" || f.category === this.category);
        root.append(this.action(uiMessage("proofreading.selectLocatedCategoryResults"), () => {
            for (const f of shown)
                if (f.sourceRange && !this.result!.existing.includes(f.id))
                    this.selected.add(f.id);
            this.draw();
        }), this.action(uiMessage("proofreading.clearSelection"), () => { this.selected.clear(); this.draw(); }), this.action(uiMessage("proofreading.annotateSelected"), () => this.add(false)), this.action(uiMessage("proofreading.ignoreSelected"), () => this.add(true)));
        for (const f of shown) {
            const row = el("article");
            row.className = "knowledge-card";
            const check = el("input");
            check.type = "checkbox";
            check.checked = this.selected.has(f.id);
            check.disabled = !f.sourceRange || this.result.existing.includes(f.id);
            check.onchange = () => {
                this.closeGuard.assertMutable();
                if (check.checked)
                    this.selected.add(f.id);
                else
                    this.selected.delete(f.id);
            };
            row.append(field(() => `${uiTextValue(categories[f.category])}：${uiTextValue(f.excerpt)}`, check), el("p", proofreadFindingUiText(f)));
            if (f.replacement !== null)
                row.append(el("p", displayMessage("proofreading.replacementSuggestion", { p0: uiDisplayParameter(f.excerpt), p1: uiDisplayParameter(f.replacement), p2: uiDisplayParameter(f.executable ? "" : uiMessage("proofreading.manualFormattingReviewSuffix")) })));
            if (this.result.existing.includes(f.id))
                row.append(el("p", uiMessage("proofreading.findingRecorded")));
            if (!f.sourceRange)
                row.append(el("p", uiMessage("proofreading.findingLocationUnavailable")));
            else
                row.append(this.action(uiMessage("proofreading.locateFinding"), () => {
                    if (sameKnowledgeBasis(this.basis, this.host.context()))
                        this.host.selectSource?.(f.sourceRange!.utf16From, f.sourceRange!.utf16To);
                    else
                        this.host.notify(uiMessage("proofreading.findingExpired"));
                }));
            root.append(row);
        }
    }
}
