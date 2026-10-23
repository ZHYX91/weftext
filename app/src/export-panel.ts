import { contentLanguageControl } from "./content-language";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { registerNativeProposalOwner } from "./native-proposal-owners";
import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import { action, decodeProjection, diagnosticText, fieldNames, formatNativeValue, ui, type NoteProjection, type PanelHost } from "./data-panel";
import { bibliographyConflictCount, decodeBibliographyInspection, ExchangeController, OutputController, OutputReadGate, type BibliographyInspection, type DocxImportAnalysis, type ExchangeFormat, type ExchangeReceipt, type ExportFormat, type EmbeddedQuerySelection, type ExportStylePolicy, type FrozenQuery, type LegacyCapabilities, type OutputBinding, type OutputFile, type TemplateInspection } from "./export-state";
import { descriptionUiText } from "./ui-diagnostics";
import { decodeLegacyCapabilities, decodeTemplateInspection, exportErrorUiText, legacyMetadataUiText, officeLossUiText, officePreviewFacts, officePreviewNotices, type LegacyMetadata, type OfficeLoss, type OfficePreview } from "./office-wire";
import { backendLabel, captionKinds, kindLabel, legacyBackendChoices, selectedCaptionOptions, templateInspectionFormat, type CaptionChoice } from "./office-import-options";
const formats: [
    ExportFormat,
    UiText
][] = [["docx", uiMessage("office.formatDocx")], ["doc", uiMessage("export.formatDoc")], ["wps", uiMessage("export.formatWps")], ["pdf", uiMessage("export.formatPdfWithQueries")], ["xlsx", uiMessage("office.formatXlsx")], ["native", uiMessage("export.formatNative")], ["bibtex", uiMessage("export.formatBibtex")], ["csl-json", uiMessage("office.formatCslJson")], ["ris", uiMessage("export.formatRis")], ["source", uiMessage("export.formatExactTypst")]] as const;
const isLegacy = (format: string): boolean => ["doc", "wps"].includes(format);
const isBibliography = (format: string): boolean => ["bibtex", "csl-json", "ris"].includes(format);
const executionNote = uiMessage("export.externalConversionHostDescription");
let nextId = 0;
function labeled(label: UiText, control: HTMLElement): HTMLElement {
    const result = ui("div", "output-control");
    control.id ||= `output-control-${++nextId}`;
    const heading = ui("label", "data-label", label);
    heading.htmlFor = control.id;
    result.append(heading, control);
    return result;
}
function select(options: [
    string,
    UiText
][], current: string): HTMLSelectElement {
    const result = ui("select", "data-input");
    for (const [value, label] of options) {
        const item = ui("option", "", label);
        item.value = value;
        result.append(item);
    }
    result.value = current;
    return result;
}
function lossList(losses: OfficeLoss[]): HTMLElement {
    const section = ui("section", "output-losses");
    section.append(ui("h3", "", losses.length ? displayMessage("export.conversionLossCount", { p0: losses.length }) : uiMessage("office.noReportedLosses")));
    if (losses.length) {
        const list = ui("ul");
        losses.forEach(loss => list.append(ui("li", "", officeLossUiText(loss))));
        section.append(list);
    }
    return section;
}
function fileBytes(file: File): Promise<OutputFile> {
    if (file.size > 64 * 1024 * 1024)
        return Promise.reject(new UiError(uiMessage("export.fileSizeLimit")));
    return new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onerror = () => reject(new UiError(uiMessage("export.fileReadFailedInputRetained")));
        reader.onload = () => {
            const value = reader.result;
            if (typeof value !== "string" || !value.includes(","))
                reject(new UiError(uiMessage("office.fileBytesIncomplete")));
            else
                resolve({ name: file.name, bytesBase64: value.slice(value.indexOf(",") + 1) });
        };
        reader.readAsDataURL(file);
    });
}
async function digest(blob: Blob): Promise<string> { const value = await crypto.subtle.digest("SHA-256", await blob.arrayBuffer()); return [...new Uint8Array(value)].map(byte => byte.toString(16).padStart(2, "0")).join(""); }
/** Safe text-only preview: server-provided markup is never executed. */
function previewContent(value: unknown, label: UiText = uiMessage("export.viewFrozenInputFullPreview")): HTMLElement {
    const details = ui("details", "output-detail");
    details.append(ui("summary", "", label));
    const names: Record<string, UiText> = { format: uiMessage("export.detailFormat"), target: uiMessage("export.detailLayoutTarget"), paragraphCount: uiMessage("export.detailParagraphCount"), rowCount: uiMessage("export.detailResultRowCount"), columns: uiMessage("export.detailColumnOrder"), rows: uiMessage("export.detailResultRows"), title: uiMessage("queryColumn.title"), text: uiMessage("field.text"), body: uiMessage("capture.body"), sheets: uiMessage("export.detailWorksheet"), name: uiMessage("export.detailName"), path: uiMessage("export.detailPath"), scope: uiMessage("graph.scope"), source: uiMessage("office.previewNativeSource"), sourceBytes: uiMessage("export.detailSourceByteLength"), files: uiMessage("export.detailFile"), fileCount: uiMessage("export.detailFileCount"), coverage: uiMessage("export.detailCompleteness"), freshness: uiMessage("export.detailFreshness"), limited: uiMessage("export.detailDisplayLimited"), summary: uiMessage("office.previewDescription"), stylePolicy: uiMessage("export.detailStylePolicy"), publication: uiMessage("export.detailPublicationProperties"), suggestedKey: uiMessage("export.detailOriginalRecordKey"), originalRecord: uiMessage("export.detailCompleteOriginalRecord"), unknownFields: uiMessage("export.detailUnknownFieldsRetained"), sources: uiMessage("export.detailFieldProvenance"), originalField: uiMessage("export.detailOriginalField"), conflicts: uiMessage("export.detailFieldConflicts"), current: uiMessage("export.detailEmbeddedExactOriginal"), imported: uiMessage("export.detailExternalStandardValue"), key: uiMessage("queryFilter.fieldLabel") };
    const draw = (current: unknown, depth: number): HTMLElement => {
        if (current === null || typeof current !== "object")
            return ui("span", "output-preview-value", current === null ? uiMessage("queryValue.explicitNull") : String(current));
        if (depth > 12)
            return ui("span", "output-preview-value", diagnosticText(current));
        if (Array.isArray(current)) {
            const list = ui("ol", "output-preview-list");
            current.forEach(item => { const row = ui("li"); row.append(draw(item, depth + 1)); list.append(row); });
            return list;
        }
        const list = ui("dl", "output-preview-record");
        Object.entries(current).forEach(([key, item]) => { list.append(ui("dt", "", depth === 0 ? names[key] ?? key : key)); const body = ui("dd"); body.append(draw(item, depth + 1)); list.append(body); });
        return list;
    };
    details.append(draw(value, 0));
    return details;
}
function productPreview(value: OfficePreview | null): HTMLElement {
    const section = ui("section", "output-preview");
    officePreviewNotices(value).forEach(notice => section.append(ui("p", "data-help", notice)));
    section.append(previewContent(officePreviewFacts(value)));
    if (value?.legacy) section.append(conversionPreview(value.legacy));
    if (value?.legacyConversion) section.append(conversionPreview(value.legacyConversion));
    return section;
}
function conversionPreview(value: LegacyMetadata): HTMLElement {
    const section = ui("section", "output-preview");
    legacyMetadataUiText(value).forEach(notice => section.append(ui("p", "data-help", notice)));
    const { explanation, diagnostics, causes, details, features, losses, ...facts } = value;
    section.append(previewContent(facts, uiMessage("export.viewExternalConversionOriginalIntermediate")));
    return section;
}
export interface OutputPanelHost extends PanelHost {
    embeddedQueries?(): EmbeddedQuerySelection[];
    imported?(receipt: ExchangeReceipt & { root: string }): Promise<void>;
}
class OutputDialog {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly unregisterClose = registerNativeProposalOwner(this);
    private outputEdited = false;
    private importEdited = false;
    private deliveredImport = false;
    private discarded = false;
    private get unresolved(): boolean {
        return !this.discarded && (this.output.busy || this.exchange.busy || this.output.uncertain !== null || this.exchange.uncertain !== null ||
            (this.outputEdited || this.output.prepared !== null) && !this.output.received ||
            (this.importEdited || this.uploadedImport !== null || this.exchange.input !== null) && !this.deliveredImport);
    }
    assertCloseReady(): void {
        try { this.closeGuard.assertReady(this.unresolved); }
        catch (error) { if (this.unresolved && !this.dialog.open) this.dialog.showModal(); throw error; }
    }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.unresolved); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    private discard(): void {
        try {
            this.closeGuard.assertMutable();
            this.closeGuard.assertReady(this.output.busy || this.exchange.busy || this.output.uncertain !== null || this.exchange.uncertain !== null || !!this.exchange.receipt && !this.deliveredImport);
            this.discarded = true;
            this.dialog.close();
            this.dialog.remove();
            this.closeGuard.freeze(false);
            this.unregisterClose();
            this.onDiscard();
        } catch (error) { this.report(error, this.outputStatus); }
    }
    readonly dialog = ui("dialog", "modal output-panel");
    private readonly output: OutputController;
    private readonly exchange: ExchangeController;
    private readonly gate = new OutputReadGate();
    private readonly importGate = new OutputReadGate();
    private readonly importReadGate = new OutputReadGate();
    private readonly capabilityGate = new OutputReadGate();
    private readonly format: HTMLSelectElement;
    private readonly style: HTMLSelectElement;
    private readonly outputLanguage: HTMLLabelElement;
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
    private readonly importFormat = select([["native", uiMessage("export.formatNative")], ["docx", uiMessage("office.formatDocxImport")], ["doc", uiMessage("export.formatDoc")], ["wps", uiMessage("export.formatWps")], ["bibtex", uiMessage("export.formatBibtex")], ["csl-json", uiMessage("office.formatCslJson")], ["ris", uiMessage("export.formatRis")]] as const, "native");
    private readonly parent = ui("input", "data-input");
    private readonly name = ui("input", "data-input");
    private readonly importFileNote = ui("p", "data-help", uiMessage("export.selectNativePackage"));
    private readonly importPreview = ui("div", "output-preview");
    private readonly importMapping = ui("div", "output-preview");
    private readonly importStatus = ui("p", "output-status");
    private readonly acceptImport = ui("input");
    private readonly importPrepare: HTMLButtonElement;
    private readonly importCommit: HTMLButtonElement;
    private uploadedImport: OutputFile | null = null;
    private inspection: TemplateInspection | null = null;
    private fields: {
        key: string;
        label: UiText;
    }[] = [];
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
    private capabilityError: UiText = "";
    private cancellingOutput = false;
    private cancellingImport = false;
    constructor(private readonly host: OutputPanelHost, basis: NonNullable<ReturnType<PanelHost["getDocument"]>>, query?: FrozenQuery, embedded: EmbeddedQuerySelection[] = [], private readonly onDiscard: () => void = () => {}) {
        this.output = new OutputController({ id: () => crypto.randomUUID(), prepare: request => host.request("/api/export/prepare", request), confirm: request => host.request("/api/export/confirm", request), cancel: request => host.request("/api/export/legacy-cancel", request), download: request => {
                if (!host.requestBlob)
                    throw new UiError(uiMessage("export.binaryDownloadUnavailable"));
                return host.requestBlob("/api/export/download", request);
            }, digest }, basis, query, embedded);
        this.exchange = new ExchangeController({ id: () => crypto.randomUUID(), prepare: request => host.request("/api/exchange/prepare", request), commit: request => host.request("/api/exchange/commit", request), cancel: request => host.request("/api/export/legacy-cancel", request) });
        const heading = ui("h2", "", uiMessage("export.title"));
        heading.id = `output-heading-${++nextId}`;
        this.dialog.setAttribute("aria-labelledby", heading.id);
        const header = ui("header", "output-header");
        header.append(heading, action(uiMessage("common.close"), () => this.dialog.close(), "data-button compact"));
        header.append(action(uiMessage("common.cancel"), () => this.discard(), "data-button compact"));
        const basisInfo = ui("p", "data-help", displayMessage("export.noteInputFrozen", { p0: uiDisplayParameter(basis.path), p1: basis.serial }));
        const sourceDetails = ui("details", "output-detail");
        sourceDetails.append(ui("summary", "", uiMessage("export.viewCurrentNativeSource")), ui("pre", "output-source", basis.source));
        const outputSection = ui("section", "output-section");
        outputSection.append(ui("h3", "", uiMessage("export.generateFile")));
        outputSection.append(ui("p", "data-help", displayMessage("export.embeddedQueryResultsFrozen", { p0: embedded.length })));
        if (query)
            outputSection.append(ui("p", query.coverage === "partial" || query.freshness === "stale" || query.limited ? "data-warning" : "data-help", displayMessage("export.queryInputFrozen", { p0: uiDisplayParameter(query.label), p1: query.rowCount, p2: uiDisplayParameter(query.coverage === "complete" ? uiMessage("export.completeScope") : uiMessage("export.partialScope")), p3: uiDisplayParameter(query.limited ? uiMessage("export.resultLimitSuffix") : ""), p4: uiDisplayParameter(query.freshness === "stale" ? uiMessage("export.freshnessUnconfirmedSuffix") : "") })));
        this.format = select(formats, this.output.draft.format);
        this.format.onchange = () => this.change(() => { const format = this.format.value as ExportFormat; this.output.setDraft({ format, legacyBackend: legacyBackendChoices(this.capabilities, format, false)[0]?.[0] }); this.inspection = null; this.templateError = false; this.readingTemplate = false; this.template.value = ""; this.templateNote.classList.remove("data-warning"); this.gate.invalidate(); this.drawBindings(); });
        this.style = select([["template", uiMessage("export.useOutputTemplateStyle")], ["source", uiMessage("export.retainNativeNoteStyle")]] as const, "template");
        this.style.onchange = () => this.change(() => this.output.setDraft({ stylePolicy: this.style.value as ExportStylePolicy }));
        this.template.type = "file";
        this.template.onchange = () => { void this.inspectTemplate(); };
        this.acceptOutput.type = "checkbox";
        this.acceptOutput.onchange = () => this.drawState();
        const grid = ui("div", "output-grid");
        this.outputLanguage = contentLanguageControl("output", this.output.draft.outputLocale, outputLocale => this.change(() => this.output.setDraft({ outputLocale })));
        grid.append(labeled(uiMessage("export.outputFormat"), this.format), labeled(uiMessage("office.stylePolicyAria"), this.style), this.outputLanguage);
        this.review = select([["none", uiMessage("office.reviewBodyOnly")], ["comments", uiMessage("office.reviewWithComments")], ["changes", uiMessage("export.reviewCommentsPendingChanges")]] as const, this.output.draft.reviewMode ?? "none");
        this.review.onchange = () => this.change(() => this.output.setDraft({ reviewMode: this.review.value as "none" | "comments" | "changes" }));
        this.resolved.type = "checkbox";
        this.resolved.checked = this.output.draft.includeResolved ?? false;
        this.resolved.onchange = () => this.change(() => this.output.setDraft({ includeResolved: this.resolved.checked }));
        grid.append(labeled(uiMessage("export.wordReviewSavedBodyRequired"), this.review), labeled(uiMessage("export.includeResolvedComments"), this.resolved));
        this.outputBackend.onchange = () => this.change(() => this.output.setDraft({ legacyBackend: this.outputBackend.value }));
        this.importBackend.onchange = () => this.refreshImportInput();
        this.outputProbe = action(uiMessage("export.recheckExecutionSoftware"), () => this.loadLegacyCapabilities(), "data-button compact");
        this.importProbe = action(uiMessage("export.recheckExecutionSoftware"), () => this.loadLegacyCapabilities(), "data-button compact");
        this.outputLegacy.append(labeled(uiMessage("export.externalConversionSoftware"), this.outputBackend), ui("p", "data-help", executionNote), ui("p", "data-help", uiMessage("export.legacyOutputConversionDescription")), this.outputLegacyNote, this.outputProbe);
        this.importLegacy.append(labeled(uiMessage("export.externalConversionSoftware"), this.importBackend), ui("p", "data-help", executionNote), ui("p", "data-help", uiMessage("export.legacyImportConversionDescription")), this.importLegacyNote, this.importProbe);
        this.prepareButton = action(uiMessage("export.generateViewPreview"), () => this.prepare(), "data-button primary");
        this.confirmButton = action(uiMessage("export.confirmFile"), () => this.confirm(), "data-button");
        this.downloadButton = action(uiMessage("export.downloadConfirmedFile"), () => this.download(), "data-button");
        this.cancelOutput = action(uiMessage("export.cancelExternalConversion"), () => this.cancelLegacy(false), "data-button");
        const buttons = ui("div", "output-actions");
        buttons.append(this.prepareButton, this.confirmButton, this.downloadButton, this.cancelOutput);
        outputSection.append(grid, this.outputLegacy, labeled(uiMessage("export.optionalOfficeTemplate"), this.template), this.templateNote, this.bindingHost, this.outputPreview, labeled(uiMessage("export.acceptListedOutputLosses"), this.acceptOutput), this.outputStatus, buttons);
        this.outputStatus.setAttribute("role", "status");
        this.outputStatus.setAttribute("aria-live", "polite");
        const importSection = ui("details", "output-import");
        importSection.append(ui("summary", "", uiMessage("export.importWordNativeBibliography")));
        this.importFile.type = "file";
        this.importFile.accept = ".zip,.weftext";
        this.importFile.onchange = () => { void this.readImport(); };
        this.parent.value = basis.root;
        setUiAttr(this.name, "placeholder", uiMessage("export.importCopyNamePlaceholder"));
        this.name.value = "导入副本";
        for (const input of [this.parent, this.name])
            input.oninput = () => this.refreshImportInput();
        this.importFormat.onchange = () => {
            this.closeGuard.assertMutable();
            this.importEdited = true;
            if (this.exchange.locked) {
                this.importFormat.value = this.exchange.input?.format ?? "native";
                return;
            }
            this.importGate.invalidate();
            this.readingMapping = false;
            this.resetImportAnalysis();
            this.fillBackends(this.importBackend, this.importFormat.value, true);
            if (!this.readingImport) {
                this.refreshImportInput();
                void this.inspectImport();
            }
            this.drawState();
        };
        this.acceptImport.type = "checkbox";
        this.acceptImport.onchange = () => this.drawState();
        this.importPrepare = action(uiMessage("export.inspectOriginalPreviewImport"), () => this.prepareImport(), "data-button");
        this.importCommit = action(uiMessage("export.createPreviewedCopy"), () => this.commitImport(), "data-button primary");
        this.cancelImport = action(uiMessage("export.cancelExternalConversion"), () => this.cancelLegacy(true), "data-button");
        const importActions = ui("div", "output-actions");
        importActions.append(this.importPrepare, this.importCommit, this.cancelImport);
        importSection.append(labeled(uiMessage("export.importFormat"), this.importFormat), this.importLegacy, labeled(uiMessage("export.originalFile"), this.importFile), this.importFileNote, labeled(uiMessage("export.importLocation"), this.parent), labeled(uiMessage("export.newCopyDirectoryName"), this.name), ui("p", "data-help", uiMessage("export.importOriginalAndIdentityDescription")), this.importMapping, this.importPreview, labeled(uiMessage("export.acceptImportPlanLosses"), this.acceptImport), this.importStatus, importActions);
        this.importStatus.setAttribute("role", "status");
        this.importStatus.setAttribute("aria-live", "polite");
        this.dialog.append(header, basisInfo, sourceDetails, outputSection, importSection);
        this.dialog.oncancel = event => { event.preventDefault(); this.dialog.close(); };
        document.body.append(this.dialog);
        this.closeGuard.bind(this.dialog);
        this.drawState();
        void this.loadFields();
        void this.loadLegacyCapabilities();
    }
    open(): void {
        this.closeGuard.assertMutable();
        if (!this.dialog.open)
            this.dialog.showModal();
    }
    private change(run: () => void): void {
        this.closeGuard.assertMutable();
        this.outputEdited = true;
        try {
            run();
            setUiText(this.outputStatus, uiMessage("export.selectionChangedGenerateNewFrozenFile"));
            this.outputPreview.replaceChildren();
            this.acceptOutput.checked = false;
        }
        catch (error) {
            this.report(error, this.outputStatus);
        }
        this.drawState();
    }
    private report(error: unknown, target: HTMLElement): void {
        target.classList.add("data-warning");
        setUiText(target, displayMessage("export.failureInputOriginalRetained", { p0: uiDisplayParameter(error instanceof Error ? errorUiText(error) : String(error)) }));
    }
    private fillBackends(control: HTMLSelectElement, format: string, importing: boolean, retained = control.value): void {
        const choices = legacyBackendChoices(this.capabilities, format, importing);
        control.replaceChildren();
        if (!choices.length)
            choices.push(["", this.checkingCapabilities ? uiMessage("export.checkingExecutionHost") : uiMessage("export.noBackendForDirection")]);
        else if (retained && !choices.some(choice => choice[0] === retained))
            choices.unshift(["", uiMessage("export.originalBackendDirectionUnavailable")]);
        for (const [value, label] of choices) {
            const option = ui("option", "", label);
            option.value = value;
            control.append(option);
        }
        control.value = choices.some(choice => choice[0] === retained) ? retained : choices[0][0];
    }
    private legacyAvailable(format: string, backend: string | undefined, importing: boolean): boolean {
        return !isLegacy(format) || legacyBackendChoices(this.capabilities, format, importing).some(choice => choice[0] === backend);
    }
    private legacyNote(format: string, importing: boolean): UiText {
        if (this.checkingCapabilities)
            return uiMessage("export.checkingExecutionSoftwareDirection");
        if (this.capabilityError)
            return displayMessage("export.directionUnavailable", { p0: uiDisplayParameter(this.capabilityError) });
        const direction = format === "doc" ? importing ? "docToDocx" : "docxToDoc" : importing ? "wpsToDocx" : "docxToWps";
        const lines = this.capabilities?.backends.map(backend => { const item = backend.directions.find(value => value.direction === direction); return joinUiText([backendLabel(backend.id), backend.version ? " " + backend.version : "", ": ", item?.available ? displayMessage("office.backendCallable", { p0: uiDisplayParameter(item.contentRoundtripVerified ? "" : uiMessage("office.backendRoundtripUnverifiedSuffix")) }) : uiMessage("queryValue.unavailable"), ...(item ? [" (", descriptionUiText(item), ")"] : [])]); }) ?? [];
        return lines.length ? joinUiText(lines, "；") : uiMessage("export.legacyBackendNotReported");
    }
    private async loadLegacyCapabilities(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (this.output.locked || this.exchange.locked)
                return;
            const generation = this.capabilityGate.begin();
            this.checkingCapabilities = true;
            this.capabilityError = "";
            this.drawState();
            try {
                const incoming = await this.host.request<unknown>("/api/export/legacy-capabilities", {});
                if (!this.capabilityGate.accepts(generation))
                    return;
                const result = decodeLegacyCapabilities(incoming, "local");
                this.capabilities = result;
                if (!this.output.locked) {
                    this.fillBackends(this.outputBackend, this.output.draft.format, false, this.output.draft.legacyBackend);
                    if (isLegacy(this.output.draft.format) && !this.output.draft.legacyBackend && this.outputBackend.value)
                        this.output.setDraft({ legacyBackend: this.outputBackend.value });
                }
                if (!this.exchange.locked) {
                    this.fillBackends(this.importBackend, this.importFormat.value, true);
                    if (isLegacy(this.importFormat.value))
                        this.refreshImportInput();
                }
            }
            catch (error) {
                if (this.capabilityGate.accepts(generation)) {
                    this.capabilities = null;
                    this.capabilityError = error instanceof Error ? errorUiText(error) : String(error);
                }
            }
            finally {
                if (this.capabilityGate.accepts(generation)) {
                    this.checkingCapabilities = false;
                    this.drawState();
                }
            }
        } finally { endCloseWork(); }
    }
    private async cancelLegacy(importing: boolean): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const controller = importing ? this.exchange : this.output;
            const status = importing ? this.importStatus : this.outputStatus;
            if (importing ? this.cancellingImport : this.cancellingOutput)
                return;
            if (importing)
                this.cancellingImport = true;
            else
                this.cancellingOutput = true;
            this.drawState();
            try {
                await controller.cancel();
                if (controller.busy || controller.uncertain === "prepare") {
                    status.classList.remove("data-warning");
                    setUiText(status, uiMessage("export.externalCancellationRequested"));
                }
            }
            catch (error) {
                this.report(error, status);
            }
            finally {
                if (importing)
                    this.cancellingImport = false;
                else
                    this.cancellingOutput = false;
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async loadFields(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            try {
                const basis = this.output.basis;
                const result = await this.host.request<NoteProjection | {
                    projection: NoteProjection;
                }>("/api/data/project", { sessionId: basis.sessionId, source: basis.source, serial: basis.serial });
                this.fields = decodeProjection(result).fields.map(field => ({ key: field.key, label: () => `${uiTextValue(fieldNames[field.key] ?? field.key)}（${uiTextValue(field.key)}）` }));
                this.drawBindings();
            }
            catch {
                setUiText(this.templateNote, uiMessage("export.notePropertiesUnreadable"));
            }
        } finally { endCloseWork(); }
    }
    private async inspectTemplate(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            this.outputEdited = true;
            const file = this.template.files?.[0];
            const retained = this.output.draft.template;
            if (!file && !retained)
                return;
            if (this.output.locked)
                return;
            const generation = this.gate.begin();
            const selectedFormat = this.output.draft.format;
            if (!["docx", "doc", "wps", "xlsx"].includes(selectedFormat))
                return;
            const format = templateInspectionFormat(selectedFormat);
            this.output.setDraft({ bindings: {} });
            this.outputPreview.replaceChildren();
            this.acceptOutput.checked = false;
            this.outputStatus.classList.remove("data-warning");
            setUiText(this.outputStatus, uiMessage("export.checkingTemplateNoNewOutput"));
            this.readingTemplate = true;
            this.templateNote.classList.remove("data-warning");
            setUiText(this.templateNote, uiMessage("export.readingTemplateSlots"));
            this.drawState();
            try {
                const uploaded = file ? await fileBytes(file) : retained!;
                if (!this.gate.accepts(generation))
                    return;
                this.output.setDraft({ template: uploaded, bindings: {} });
                this.inspection = null;
                this.outputPreview.replaceChildren();
                this.acceptOutput.checked = false;
                const incoming = await this.host.request<unknown>("/api/export/template", { format, bytesBase64: uploaded.bytesBase64 });
                if (!this.gate.accepts(generation))
                    return;
                const result = decodeTemplateInspection(incoming, format);
                this.inspection = result;
                this.templateError = false;
                setUiText(this.templateNote, uiMessage("export.templateStandardSlotsChecked"));
                setUiText(this.outputStatus, uiMessage("export.templateCheckedChooseContent"));
            }
            catch (error) {
                if (this.gate.accepts(generation)) {
                    this.templateError = true;
                    this.report(error, this.templateNote);
                    this.outputStatus.classList.add("data-warning");
                    setUiText(this.outputStatus, uiMessage("export.templateCheckFailedNoOutput"));
                }
            }
            finally {
                if (this.gate.accepts(generation)) {
                    this.readingTemplate = false;
                    this.drawBindings();
                    this.drawState();
                }
            }
        } finally { endCloseWork(); }
    }
    private drawBindings(): void {
        this.bindingHost.replaceChildren();
        if (!this.output.draft.template && !this.templateError)
            return;
        const title = ui("div", "output-template-heading");
        title.append(ui("strong", "", this.output.draft.template?.name ?? this.template.files?.[0]?.name ?? uiMessage("export.selectedTemplate")), action(uiMessage("export.removeTemplate"), () => this.change(() => { this.output.setDraft({ template: undefined, bindings: {} }); this.inspection = null; this.templateError = false; this.templateNote.classList.remove("data-warning"); this.template.value = ""; this.drawBindings(); }), "data-button compact"));
        this.bindingHost.append(title);
        if (this.templateError)
            this.bindingHost.append(action(uiMessage("export.recheckTemplate"), () => this.inspectTemplate(), "data-button compact"));
        if (!this.inspection) {
            this.bindingHost.append(ui("p", "data-warning", uiMessage("export.templateNotAccepted")));
            this.drawState();
            return;
        }
        if (!this.inspection.slots.length)
            this.bindingHost.append(ui("p", "data-help", uiMessage("export.templateNoFillableSlots")));
        for (const slot of this.inspection.slots) {
            if (slot.supported === false) {
                this.bindingHost.append(ui("p", "data-warning", joinUiText([slot.label, ": ", slot.reason ? exportErrorUiText(slot.reason) : uiMessage("export.templateSlotUnsupported")])));
                continue;
            }
            const options: [
                string,
                UiText
            ][] = [["", uiMessage("export.retainTemplateContent")]] as const;
            if (slot.kind === "content")
                options.push(["body", uiMessage("export.noteBody")], ["title", uiMessage("knowledge.noteTitle")]);
            if (slot.kind === "cell")
                options.push(["title", uiMessage("knowledge.noteTitle")]);
            if (["content", "cell"].includes(slot.kind)) {
                options.push(...this.fields.map(field => [`field:${field.key}`, displayMessage("export.propertyChoice", { p0: uiDisplayParameter(field.label) })] as [
                    string,
                    UiText
                ]));
                options.push(["text", uiMessage("export.fixedTextThisOutput")]);
            }
            if (["content", "range", "table"].includes(slot.kind) && this.output.draft.query)
                options.push(["query", uiMessage("export.frozenQueryResultThisOutput")]);
            const existing = this.output.draft.bindings[slot.name];
            const value = existing?.kind === "field" ? `field:${existing.field}` : existing?.kind ?? "";
            if (value && !options.some(option => option[0] === value))
                options.push([value, displayMessage("export.retainExistingMapping", { p0: uiDisplayParameter(value) })]);
            const input = select(options, value);
            const row = labeled(() => `${uiTextValue(slot.label ?? slot.name)} · ${uiTextValue(slot.kind === "content" ? uiMessage("export.slotContentControl") : slot.kind === "cell" ? uiMessage("export.slotCellName") : slot.kind === "range" ? uiMessage("export.slotNamedRange") : uiMessage("queryView.table"))}`, input);
            const text = ui("textarea", "data-input");
            text.rows = 2;
            text.value = existing?.text ?? "";
            text.hidden = value !== "text";
            setUiAttr(text, "aria-label", displayMessage("export.slotFixedText", { p0: uiDisplayParameter(slot.label ?? slot.name) }));
            row.append(text);
            const update = (): void => this.change(() => {
                const bindings = { ...this.output.draft.bindings };
                const choice = input.value;
                if (!choice)
                    delete bindings[slot.name];
                else
                    bindings[slot.name] = choice.startsWith("field:") ? { kind: "field", field: choice.slice(6) } : choice === "text" ? { kind: "text", text: text.value } : { kind: choice as OutputBinding["kind"] };
                this.output.setDraft({ bindings });
                text.hidden = choice !== "text";
            });
            input.onchange = update;
            text.oninput = update;
            this.bindingHost.append(row);
            if (slot.description) this.bindingHost.append(ui("p", "data-help", slot.description));
        }
        this.drawState();
    }
    private async prepare(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            this.outputEdited = true;
            if (!this.output.uncertain && !this.output.prepared && !this.legacyAvailable(this.output.draft.format, this.output.draft.legacyBackend, false)) {
                this.report(new UiError(uiMessage("export.externalBackendUnavailable")), this.outputStatus);
                return;
            }
            this.outputStatus.classList.remove("data-warning");
            setUiText(this.outputStatus, this.output.uncertain ? uiMessage("export.checkingOriginalOutput") : uiMessage("export.generatingVerifyingFrozenFile"));
            const promise = this.output.prepare();
            this.drawState();
            try {
                const result = await promise;
                this.outputPreview.replaceChildren(ui("p", "output-summary", result.summary), ui("p", "data-help", displayMessage("export.generatedFileSize", { p0: uiDisplayParameter(result.filename), p1: uiDisplayParameter(result.length.toLocaleString("zh-CN")) })), lossList(result.losses));
                if (result.preview !== undefined)
                    this.outputPreview.append(productPreview(result.preview));
                const identity = ui("details", "output-detail");
                identity.append(ui("summary", "", uiMessage("export.fileInformation")), ui("p", "data-help", displayMessage("export.fileId", { p0: uiDisplayParameter(result.id) })), ui("p", "data-help", `SHA-256 ${result.digest}`));
                this.outputPreview.append(identity);
                this.acceptOutput.checked = false;
                setUiText(this.outputStatus, this.output.confirmed ? uiMessage("export.fileConfirmedSameBytes") : uiMessage("export.exactFileGeneratedAwaitingConfirmation"));
            }
            catch (error) {
                this.report(error, this.outputStatus);
            }
            finally {
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async confirm(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const promise = this.output.confirm(this.acceptOutput.checked);
            setUiText(this.outputStatus, uiMessage("export.confirmingSamePreviewedFile"));
            this.drawState();
            try {
                await promise;
                this.outputStatus.classList.remove("data-warning");
                setUiText(this.outputStatus, uiMessage("export.fileConfirmedDownloadable"));
            }
            catch (error) {
                this.report(error, this.outputStatus);
            }
            finally {
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async download(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const promise = this.output.download();
            setUiText(this.outputStatus, uiMessage("export.receivingVerifyingConfirmedFile"));
            this.drawState();
            try {
                const blob = await promise;
                const url = URL.createObjectURL(blob);
                const link = ui("a");
                link.href = url;
                link.download = this.output.prepared!.filename;
                document.body.append(link);
                link.click();
                link.remove();
                setTimeout(() => URL.revokeObjectURL(url), 60000);
                this.outputStatus.classList.remove("data-warning");
                setUiText(this.outputStatus, uiMessage("export.confirmedBytesHandedToBrowser"));
            }
            catch (error) {
                this.report(error, this.outputStatus);
            }
            finally {
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async readImport(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            this.importEdited = true;
            this.deliveredImport = false;
            const file = this.importFile.files?.[0];
            if (!file || this.exchange.locked)
                return;
            const generation = this.importReadGate.begin();
            this.importGate.invalidate();
            this.readingMapping = false;
            this.resetImportAnalysis();
            this.readingImport = true;
            this.importPreview.replaceChildren();
            this.drawState();
            try {
                const uploaded = await fileBytes(file);
                if (!this.importReadGate.accepts(generation))
                    return;
                this.uploadedImport = uploaded;
                this.importReadError = false;
                setUiText(this.importFileNote, displayMessage("export.uploadedOriginalRetained", { p0: uiDisplayParameter(uploaded.name), p1: uiDisplayParameter(file.size.toLocaleString("zh-CN")) }));
                this.refreshImportInput();
            }
            catch (error) {
                if (this.importReadGate.accepts(generation)) {
                    this.importReadError = true;
                    this.report(error, this.importStatus);
                }
            }
            finally {
                if (this.importReadGate.accepts(generation)) {
                    this.readingImport = false;
                    this.drawState();
                }
            }
            if (!this.importReadError && this.importReadGate.accepts(generation))
                await this.inspectImport();
        } finally { endCloseWork(); }
    }
    private resetImportAnalysis(): void { this.mapping = null; this.mappingError = false; this.docxAnalysis = null; this.captionChoices = []; this.captionError = false; this.importMapping.replaceChildren(); }
    private async inspectImport(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (this.exchange.locked || this.readingImport)
                return;
            const generation = this.importGate.begin();
            this.resetImportAnalysis();
            this.readingMapping = false;
            const format = this.importFormat.value;
            const uploaded = this.uploadedImport;
            if (uploaded && !this.importReadError)
                this.refreshImportInput();
            if (format === "native" || isLegacy(format) || !uploaded || this.importReadError) {
                this.drawState();
                return;
            }
            this.readingMapping = true;
            setUiText(this.importStatus, format === "docx" ? uiMessage("export.checkingWordCaptionsObjects") : uiMessage("export.checkingBibliographyRecordsMapping"));
            this.drawState();
            try {
                if (format === "docx") {
                    const analysis = await this.host.request<DocxImportAnalysis>("/api/exchange/inspect", { format, bytesBase64: uploaded.bytesBase64 });
                    if (!this.importGate.accepts(generation))
                        return;
                    if (!/^[a-f0-9]{64}$/i.test(analysis.contentHash) || !Array.isArray(analysis.captions) || !Array.isArray(analysis.objects) || analysis.captions.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || typeof item.text !== "string" || typeof item.suggestedText !== "string") || analysis.objects.some(item => !Number.isSafeInteger(item.blockIndex) || item.blockIndex < 0 || !captionKinds.some(kind => kind[0] === item.kind)))
                        throw new UiError(uiMessage("export.wordCaptionAnalysisIncomplete"));
                    this.docxAnalysis = analysis;
                    this.captionChoices = analysis.captions.map(caption => ({ enabled: false, captionBlock: caption.blockIndex, objectBlock: "", kind: "", captionText: caption.suggestedText, position: "" }));
                    this.drawCaptionPairings();
                    this.refreshImportInput();
                    this.importStatus.classList.remove("data-warning");
                    setUiText(this.importStatus, uiMessage("export.wordOriginalCheckedCaptionDescription"));
                    return;
                }
                const incoming = await this.host.request<unknown>("/api/exchange/inspect", { format, bytesBase64: uploaded.bytesBase64 });
                if (!this.importGate.accepts(generation))
                    return;
                const result = decodeBibliographyInspection(incoming);
                const conflicts = bibliographyConflictCount(result);
                this.mapping = result;
                this.importMapping.replaceChildren(ui("p", "output-summary", displayMessage("export.bibliographyRecordCount", { p0: result.records.length })), lossList(result.losses), previewContent(result.records, uiMessage("export.viewBibliographyRecordsProvenanceConflicts")));
                if (conflicts) {
                    this.importMapping.prepend(ui("p", "data-warning", displayMessage("export.bibliographyConflictCount", { p0: conflicts })));
                    setUiText(this.importStatus, uiMessage("export.bibliographyConflictsRetained"));
                }
                else {
                    this.importStatus.classList.remove("data-warning");
                    setUiText(this.importStatus, uiMessage("export.bibliographyMappingChecked"));
                }
            }
            catch (error) {
                if (this.importGate.accepts(generation)) {
                    this.mappingError = format !== "docx";
                    this.report(error, this.importStatus);
                    this.importMapping.replaceChildren(ui("p", "data-warning", format === "docx" ? uiMessage("export.captionProposalUnavailable") : uiMessage("export.completeConflictFreeMappingRequired")), action(format === "docx" ? uiMessage("export.recheckWordCaptions") : uiMessage("export.recheckBibliographyOriginal"), () => this.inspectImport(), "data-button compact"));
                }
            }
            finally {
                if (this.importGate.accepts(generation)) {
                    this.readingMapping = false;
                    this.drawState();
                }
            }
        } finally { endCloseWork(); }
    }
    private drawCaptionPairings(): void {
        const analysis = this.docxAnalysis;
        this.importMapping.replaceChildren();
        if (!analysis)
            return;
        this.importMapping.append(ui("h4", "", displayMessage("export.wordCaptionCandidateCount", { p0: analysis.captions.length })), ui("p", "data-help", uiMessage("export.captionPairingDescription")));
        if (!analysis.captions.length) {
            this.importMapping.append(ui("p", "data-help", uiMessage("export.noPairableSeqCaptions")));
            return;
        }
        analysis.captions.forEach((caption, index) => {
            const choice = this.captionChoices[index];
            const row = ui("fieldset", "output-caption-pairing");
            row.append(ui("legend", "", displayMessage("office.captionBlockSummary", { p0: caption.blockIndex + 1, p1: uiDisplayParameter(caption.sequence) })), ui("p", "output-preview-value", caption.text));
            const object = select([["", uiMessage("export.chooseCaptionOwner")], ...analysis.objects.filter(item => item.blockIndex !== caption.blockIndex).map(item => [String(item.blockIndex), displayMessage("office.captionObjectChoice", { p0: item.blockIndex + 1, p1: uiDisplayParameter(kindLabel(item.kind)) })] as [
                    string,
                    UiText
                ])], choice.objectBlock);
            const kind = select([["", uiMessage("export.chooseCaptionKind")], ...captionKinds], choice.kind);
            const position = select([["", uiMessage("export.chooseCaptionPosition")], ["top", uiMessage("office.captionAboveObject")], ["bottom", uiMessage("office.captionBelowObject")]] as const, choice.position);
            const text = ui("textarea", "data-input");
            text.rows = 2;
            text.value = choice.captionText;
            const enabled = ui("input");
            enabled.type = "checkbox";
            enabled.checked = choice.enabled;
            const update = (): void => {
                if (this.exchange.locked)
                    return;
                Object.assign(choice, { enabled: enabled.checked, objectBlock: object.value, kind: kind.value, captionText: text.value, position: position.value });
                this.refreshImportInput();
            };
            object.onchange = update;
            kind.onchange = update;
            position.onchange = update;
            text.oninput = update;
            enabled.onchange = update;
            const grid = ui("div", "output-grid");
            grid.append(labeled(uiMessage("documentNotes.owningObject"), object), labeled(uiMessage("export.captionKind"), kind), labeled(uiMessage("export.captionPosition"), position));
            row.append(grid, labeled(uiMessage("export.captionBodyEditable"), text), labeled(uiMessage("export.confirmCaptionPairing"), enabled));
            this.importMapping.append(row);
        });
    }
    private refreshImportInput(): void {
        this.closeGuard.assertMutable();
        this.importEdited = true;
        this.deliveredImport = false;
        if (!this.uploadedImport || this.exchange.locked)
            return;
        try {
            const basis = this.output.basis;
            const format = this.importFormat.value as ExchangeFormat;
            const docxOptions = format === "docx" ? selectedCaptionOptions(this.docxAnalysis, this.captionChoices) : undefined;
            this.exchange.setInput({ sessionId: basis.sessionId, baseId: basis.baseId, root: basis.root, parent: this.parent.value, name: this.name.value, format, bytesBase64: this.uploadedImport.bytesBase64, ...(isLegacy(format) ? { legacyBackend: this.importBackend.value } : {}), ...(docxOptions ? { docxOptions } : {}) });
            this.captionError = false;
            this.importPreview.replaceChildren();
            this.acceptImport.checked = false;
            this.importStatus.classList.remove("data-warning");
            setUiText(this.importStatus, uiMessage("export.originalLocationRetainedAwaitingPreview"));
        }
        catch (error) {
            this.captionError = true;
            this.importPreview.replaceChildren();
            this.acceptImport.checked = false;
            this.report(error, this.importStatus);
        }
        this.drawState();
    }
    private async prepareImport(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (!this.exchange.uncertain && (this.importReadError || this.mappingError || this.captionError || this.readingImport || this.readingMapping || isBibliography(this.importFormat.value) && (!this.mapping || bibliographyConflictCount(this.mapping)))) {
                this.report(new UiError(uiMessage("export.finishOriginalInspectionBeforePrepare")), this.importStatus);
                return;
            }
            if (!this.exchange.uncertain && !this.exchange.plan && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true)) {
                this.report(new UiError(uiMessage("export.externalBackendUnavailable")), this.importStatus);
                return;
            }
            if (!this.parent.value || !this.name.value) {
                this.report(new UiError(uiMessage("export.importLocationAndNameRequired")), this.importStatus);
                return;
            }
            if (!this.exchange.locked && !this.exchange.plan) {
                this.refreshImportInput();
                if (this.captionError)
                    return;
            }
            const promise = this.exchange.prepare();
            setUiText(this.importStatus, uiMessage("export.checkingOriginalPreparingImport"));
            this.drawState();
            try {
                const plan = await promise;
                this.importPreview.replaceChildren(lossList(plan.losses));
                if (plan.contentPreview) {
                    const p = plan.contentPreview;
                    this.importPreview.append(ui("h4", "", displayMessage("export.wordImportReviewCounts", { p0: p.comments, p1: p.revisions })), ui("p", "data-help", uiMessage("export.wordRevisionsBaselineDescription")), ui("pre", "output-source-preview", p.source.replace(/^#metadata[^\n]*\n/, "")));
                    if (p.truncated)
                        this.importPreview.append(ui("p", "data-help", uiMessage("export.bodyPreviewTruncated")));
                    for (const a of p.annotations)
                        this.importPreview.append(ui("p", "", () => `${uiTextValue(a.author)}：${uiTextValue(a.body)}（${uiTextValue(a.unlocated ? uiMessage("export.unlocatedRetainedOnOriginal") : a.excerpt || uiMessage("export.insertionPointOrReply"))}）`));
                }
                if (plan.changes)
                    this.importPreview.append(previewContent(plan.changes));
                if (plan.legacyConversion !== undefined)
                    this.importPreview.append(conversionPreview(plan.legacyConversion));
                this.acceptImport.checked = false;
                this.importStatus.classList.remove("data-warning");
                setUiText(this.importStatus, uiMessage("export.importPlanPrepared"));
            }
            catch (error) {
                this.report(error, this.importStatus);
            }
            finally {
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async commitImport(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const promise = this.exchange.commit(this.acceptImport.checked);
            setUiText(this.importStatus, uiMessage("export.committingSameImportPlan"));
            this.drawState();
            try {
                const receipt = await promise;
                const opened = await this.deliverReceipt(receipt);
                this.importStatus.classList.remove("data-warning");
                setUiText(this.importStatus, opened ? uiMessage("export.importCommittedOpenedActualNote") : displayMessage("export.importCommittedActualCopyDirectory", { p0: uiDisplayParameter(receipt.path) }));
            }
            catch (error) {
                this.report(error, this.importStatus);
            }
            finally {
                this.drawState();
            }
        } finally { endCloseWork(); }
    }
    private async deliverReceipt(receipt: ExchangeReceipt): Promise<boolean> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const root = this.exchange.input?.root ?? this.output.basis.root;
            this.host.environmentChanged?.(root);
            if (!receipt.path)
                throw Object.assign(new UiError(uiMessage("export.importCommittedReceiptLocationMissing")), { code: "receiptOpenUnavailable" });
            await this.host.imported?.({ ...receipt, root });
            this.deliveredImport = true;
            return false;
        } finally { endCloseWork(); }
    }
    private drawState(): void {
        const outputFormat = this.output.draft.format;
        const office = ["docx", "doc", "wps", "xlsx"].includes(outputFormat);
        const word = ["docx", "doc", "wps"].includes(outputFormat);
        const outputLocked = this.output.locked || this.readingTemplate;
        this.format.value = outputFormat;
        this.style.value = this.output.draft.stylePolicy;
        this.review.value = this.output.draft.reviewMode ?? "none";
        this.resolved.checked = this.output.draft.includeResolved ?? false;
        this.format.disabled = outputLocked;
        this.style.disabled = outputLocked || !office;
        this.outputLanguage.querySelector("select")!.disabled = outputLocked;
        this.template.disabled = outputLocked || !office;
        this.template.accept = outputFormat === "xlsx" ? ".xlsx" : ".docx";
        this.review.disabled = outputLocked || !word;
        this.resolved.disabled = outputLocked || !word;
        this.outputLegacy.hidden = !isLegacy(outputFormat);
        this.importLegacy.hidden = !isLegacy(this.importFormat.value);
        if (!this.output.locked)
            this.fillBackends(this.outputBackend, outputFormat, false, this.output.draft.legacyBackend);
        if (!this.exchange.locked)
            this.fillBackends(this.importBackend, this.importFormat.value, true);
        this.outputBackend.disabled = outputLocked || this.checkingCapabilities || !legacyBackendChoices(this.capabilities, outputFormat, false).length;
        this.importBackend.disabled = this.exchange.locked || this.readingImport || this.readingMapping || this.checkingCapabilities || !legacyBackendChoices(this.capabilities, this.importFormat.value, true).length;
        setUiText(this.outputLegacyNote, this.legacyNote(outputFormat, false));
        setUiText(this.importLegacyNote, this.legacyNote(this.importFormat.value, true));
        this.outputLegacyNote.classList.toggle("data-warning", isLegacy(outputFormat) && !this.legacyAvailable(outputFormat, this.output.draft.legacyBackend, false));
        this.importLegacyNote.classList.toggle("data-warning", isLegacy(this.importFormat.value) && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true));
        this.outputProbe.disabled = this.output.locked || this.exchange.locked || this.checkingCapabilities;
        this.importProbe.disabled = this.outputProbe.disabled;
        if (!this.readingTemplate && !this.output.draft.template && !this.templateNote.classList.contains("data-warning"))
            setUiText(this.templateNote, office ? this.output.draft.format === "xlsx" && !this.output.draft.query ? uiMessage("export.xlsxTemplateDescription") : uiMessage("export.officeTemplateDescription") : this.output.draft.format === "native" ? uiMessage("export.nativePackageDescription") : uiMessage("export.formatNoOfficeTemplate"));
        this.bindingHost.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement | HTMLButtonElement>("input,textarea,select,button").forEach(input => { input.disabled = outputLocked; });
        this.prepareButton.disabled = this.output.busy || this.readingTemplate || this.templateError || !!this.output.draft.template && !this.inspection || this.output.uncertain === "confirm" || this.output.uncertain === "download" || !this.output.uncertain && !this.output.prepared && !this.legacyAvailable(outputFormat, this.output.draft.legacyBackend, false);
        setUiText(this.prepareButton, this.output.uncertain === "prepare" ? uiMessage("export.checkOriginalOutput") : this.output.prepared ? uiMessage("export.viewGeneratedFile") : uiMessage("export.generateViewPreview"));
        this.acceptOutput.disabled = this.output.busy || this.output.uncertain === "confirm" || this.output.confirmed || !this.output.prepared?.losses.length;
        this.acceptOutput.parentElement!.hidden = !this.output.prepared?.losses.length;
        this.confirmButton.disabled = !this.output.prepared || this.output.busy || this.output.uncertain === "prepare" || this.output.uncertain === "download" || this.output.confirmed || !!this.output.prepared.losses.length && !this.acceptOutput.checked;
        setUiText(this.confirmButton, this.output.uncertain === "confirm" ? uiMessage("export.checkOriginalConfirmation") : uiMessage("export.confirmFile"));
        this.downloadButton.disabled = !this.output.confirmed || this.output.busy || this.output.uncertain === "confirm";
        setUiText(this.downloadButton, this.output.received ? uiMessage("export.downloadSameFileAgain") : uiMessage("export.downloadConfirmedFile"));
        const importLocked = this.exchange.locked || this.readingImport || this.readingMapping;
        this.importFile.disabled = this.exchange.locked;
        this.importFormat.disabled = this.exchange.locked;
        this.parent.disabled = importLocked;
        this.name.disabled = importLocked;
        this.importFile.accept = ({ native: ".zip,.weftext", docx: ".docx", doc: ".doc", wps: ".wps", bibtex: ".bib,.bibtex", "csl-json": ".json", ris: ".ris" } as Record<string, string>)[this.importFormat.value];
        this.importMapping.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | HTMLButtonElement>("input,select,textarea,button").forEach(input => { input.disabled = importLocked; });
        const unmapped = isBibliography(this.importFormat.value) && (!this.mapping || bibliographyConflictCount(this.mapping) > 0);
        this.importPrepare.disabled = !this.uploadedImport || this.importReadError || this.mappingError || this.captionError || unmapped || this.exchange.busy || this.readingImport || this.readingMapping || this.exchange.uncertain === "commit" || !!this.exchange.receipt || !this.exchange.uncertain && !this.exchange.plan && !this.legacyAvailable(this.importFormat.value, this.importBackend.value, true);
        setUiText(this.importPrepare, this.exchange.uncertain === "prepare" ? uiMessage("export.checkOriginalImportPreview") : uiMessage("export.inspectOriginalPreviewImport"));
        this.acceptImport.disabled = this.exchange.busy || this.exchange.uncertain === "commit" || !!this.exchange.receipt || !this.exchange.plan?.losses.length;
        this.acceptImport.parentElement!.hidden = !this.exchange.plan?.losses.length;
        this.importCommit.disabled = !this.exchange.plan || this.exchange.busy || this.readingImport || this.readingMapping || this.importReadError || this.mappingError || this.captionError || unmapped || this.exchange.uncertain === "prepare" || !this.exchange.receipt && !!this.exchange.plan.losses.length && !this.acceptImport.checked;
        setUiText(this.importCommit, this.exchange.receipt ? uiMessage("export.refreshImportedCopy") : this.exchange.uncertain === "commit" ? uiMessage("export.checkOriginalImportResult") : uiMessage("export.createPreviewedCopy"));
        this.cancelOutput.hidden = !isLegacy(outputFormat) || !this.output.requestId || !this.output.busy && this.output.uncertain !== "prepare";
        this.cancelOutput.disabled = this.cancellingOutput;
        setUiText(this.cancelOutput, this.cancellingOutput ? uiMessage("export.requestingCancellation") : uiMessage("export.cancelExternalConversion"));
        this.cancelImport.hidden = !isLegacy(this.importFormat.value) || !this.exchange.requestId || !this.exchange.busy && this.exchange.uncertain !== "prepare";
        this.cancelImport.disabled = this.cancellingImport;
        setUiText(this.cancelImport, this.cancellingImport ? uiMessage("export.requestingCancellation") : uiMessage("export.cancelExternalConversion"));
    }
}
/** A closed dialog retains frozen inputs, received receipts and uncertain IDs. */
export class OutputPanel {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly dialogs = new Map<string, OutputDialog>();
    constructor(private readonly host: OutputPanelHost) { }
    assertCloseReady(): void { this.closeGuard.assertReady(false); for (const dialog of this.dialogs.values()) dialog.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        if (!this.closeGuard.freeze(false)) return false;
        for (const dialog of this.dialogs.values()) if (!dialog.freezeForClose()) { this.thawAfterClose(); return false; }
        return true;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); for (const dialog of this.dialogs.values()) dialog.thawAfterClose(); }
    async open(query?: FrozenQuery): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            await this.host.prepareAction?.();
            const basis = this.host.getDocument();
            if (!basis) {
                this.host.notify(uiMessage("export.openNoteBeforeOutputImport"), true);
                return;
            }
            const embedded = this.host.embeddedQueries?.() ?? [];
            const key = `${basis.sessionId}/${basis.serial}/${basis.baseId}/${query?.queryId ?? "note"}/${JSON.stringify(embedded)}`;
            let dialog = this.dialogs.get(key);
            if (!dialog) {
                dialog = new OutputDialog(this.host, basis, query, embedded, () => this.dialogs.delete(key));
                this.dialogs.set(key, dialog);
            }
            dialog.open();
        } finally { endCloseWork(); }
    }
}
