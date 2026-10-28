import { replaceUiChildren, disposeUiTree } from "./ui-locale";
import { workActivity } from "./work-activity";
import type { WorkTaskStatus } from "./work-panel";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { officeLossUiText } from "./office-wire";
import { joinUiText } from "./ui-locale";
import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./recognition-panel.css";
import "./data-view.css";
import { action, ui } from "./data-panel";
import { RecognitionController, decodeRecognitionCapabilities, decodeRecognitionLocator, recognitionTerminal, recognitionActionable, type RecognitionAuthorReview, type RecognitionCapabilities, type RecognitionLocator, type RecognitionOptions, type RecognitionPurpose, type RecognitionReceipt, type RecognitionTarget } from "./recognition-state";
import { RecognitionReviewDraft, confidenceLabel, parseRecognitionPages, parseRecognitionRect, recognitionBlocks, recognitionReviewReady, type RecognitionPage, type RecognitionRect } from "./recognition-review";
export interface RecognitionPanelHost {
    request<T>(path: string, body?: unknown): Promise<T>;
    identity(): string | null;
    target(): RecognitionTarget | null;
    created?(receipt: RecognitionReceipt, target: RecognitionTarget): void | Promise<void>;
    notify?(message: UiText, error?: boolean): void;
    committed?(identity: string): void;
}
interface Field {
    id: string;
    key: string;
    title: string;
    shape: {
        kind: "scalar";
        semantic: string;
    };
    typeId: string;
}
const mappingIdentity = (field: Field): string => JSON.stringify([field.id, field.typeId]);
interface RegionRow {
    id: string;
    page: string;
    rect: string;
    mode: string;
    rotation: string;
    segmentation: string;
    scale: string;
}
const purposes: [
    string,
    UiText
][] = [["resource", uiMessage("recognition.purposeKeepOriginal")], ["text", uiMessage("recognition.purposeExtractText")], ["note", uiMessage("recognition.purposeEditableNote")]] as const;
const statuses: Record<string, UiText> = { pending: uiMessage("recognition.statusQueued"), running: uiMessage("recognition.statusProcessing"), complete: uiMessage("recognition.statusSelectedScopeComplete"), partial: uiMessage("recognition.statusPartialAcceptanceRequired"), blank: uiMessage("recognition.statusBlank"), failed: uiMessage("recognition.statusFailed"), cancelled: uiMessage("recognition.statusCancelled"), interrupted: uiMessage("recognition.statusInterrupted"), prepared: uiMessage("recognition.statusPrepared"), committed: uiMessage("recognition.statusOriginalCommitted"), notRegistered: uiMessage("recognition.statusOriginalNotRegistered"), recoveryPending: uiMessage("recognition.statusOriginalRecovering"), resolved: uiMessage("recognition.statusOriginalEnded") };
const fieldNames: Record<string, UiText> = { invoiceNumber: uiMessage("recognition.fieldInvoiceNumber"), issueDate: uiMessage("recognition.fieldDate"), buyerName: uiMessage("recognition.fieldBuyerName"), buyerTaxId: uiMessage("recognition.fieldBuyerTaxId"), sellerName: uiMessage("recognition.fieldSellerName"), sellerTaxId: uiMessage("recognition.fieldSellerTaxId"), amount: uiMessage("recognition.fieldAmount"), tax: uiMessage("recognition.fieldTax"), total: uiMessage("recognition.fieldTotalIncludingTax"), currency: uiMessage("recognition.fieldCurrency"), issuingAgency: uiMessage("recognition.fieldIssuingAuthority"), documentNumber: uiMessage("recognition.fieldDocumentNumber"), mainRecipient: uiMessage("recognition.fieldPrimaryAddressees") };
const schemeStatus: Record<string, UiText> = { supported: uiMessage("recognition.schemeApplicable"), unknown: uiMessage("recognition.schemeApplicabilityUnknown"), "not-applicable": uiMessage("recognition.schemeNotApplicable"), unsupported: uiMessage("recognition.schemeUnsupportedLayout") };
const evidenceIssueNames: Record<string, UiText> = { "docx-content-requires-original-body-importer": uiMessage("recognition.wordUnconvertedStructures"), "docx-active-content-not-executed": uiMessage("recognition.wordActiveContentNotRun"), "unresolved-docx-style": uiMessage("recognition.wordStylesUncertain"), "unresolved-docx-numbering": uiMessage("recognition.wordNumberingUncertain") };
const candidateStatus: Record<string, UiText> = { present: uiMessage("recognition.fieldCandidateUnconfirmed"), missing: uiMessage("recognition.fieldMissing"), duplicate: uiMessage("recognition.fieldDuplicateCandidates"), ambiguous: uiMessage("recognition.fieldAmbiguous"), "parse-failure": uiMessage("recognition.fieldParseFailed") };
const recognitionIssueNames: Record<string, UiText> = { "host-execution-interrupted": uiMessage("recognition.issueInterruptedCheckpoints"), "unprocessed-after-interruption": uiMessage("recognition.issuePageUnprocessed"), "low-confidence-words-require-review": uiMessage("recognition.issueLowConfidence"), "nonblank-region-without-recognized-text": uiMessage("recognition.issueNoTextInNonblankRegion"), "unmapped-pdf-character": uiMessage("recognition.issueUnmappedPdfCharacters"), "text-crosses-selection-boundary": uiMessage("recognition.issueCrossBoundaryText"), "image-object-budget": uiMessage("recognition.issueImageBudget"), "nested-image-geometry-requires-explicit-regions": uiMessage("recognition.issueNestedImageGeometry"), "summary-columns-ambiguous": uiMessage("recognition.issueInvoiceTotalsAmbiguous"), "competing-invoice-numbers": uiMessage("recognition.issueMultipleInvoiceNumbers"), "currency-not-stated": uiMessage("recognition.issueCurrencyMissing"), "amount-plus-tax-does-not-equal-total": uiMessage("recognition.issueInvoiceTotalMismatch"), "fields-and-text-pattern-structures-require-confirmation": uiMessage("recognition.issueFieldsAndStructureNeedReview"), "source-coverage-not-complete": uiMessage("recognition.issuePartialSourceScope") };
const issueLabel = (code: string): UiText => recognitionIssueNames[code] ?? code;
let sequence = 0;
function label(text: UiText, control: HTMLElement): HTMLElement { control.id ||= `recognition-${++sequence}`; const wrapper = ui("label", "recognition-control"); wrapper.htmlFor = control.id; wrapper.append(ui("span", "data-label", text), control); return wrapper; }
function select(items: [
    string,
    UiText
][], aria: UiText): HTMLSelectElement {
    const control = ui("select", "data-input");
    setUiAttr(control, "aria-label", aria);
    for (const [value, title] of items) {
        const option = ui("option", "", title);
        option.value = value;
        control.append(option);
    }
    return control;
}
function input(aria: UiText, value = ""): HTMLInputElement { const control = ui("input", "data-input"); setUiAttr(control, "aria-label", aria); control.value = value; return control; }
function check(aria: UiText, checked: boolean, change: (value: boolean) => void): HTMLElement {
    const control = input(aria);
    control.className = "recognition-checkbox";
    control.type = "checkbox";
    control.checked = checked;
    control.onchange = () => change(control.checked);
    const wrapper = ui("label", "recognition-check");
    wrapper.append(control, ui("span", "", aria));
    return wrapper;
}
function base64(bytes: Uint8Array): string {
    let binary = "";
    for (let i = 0; i < bytes.length; i += 32768)
        binary += String.fromCharCode(...bytes.subarray(i, i + 32768));
    return btoa(binary);
}
function bytes(value: string): Uint8Array<ArrayBuffer> { const raw = atob(value); return Uint8Array.from(raw, c => c.charCodeAt(0)); }
const sha = async (value: Uint8Array<ArrayBuffer>): Promise<string> => [...new Uint8Array(await crypto.subtle.digest("SHA-256", value))].map(b => b.toString(16).padStart(2, "0")).join("");
/** One shared consumer for desktop and self-hosted WebUI. No private text is persisted. */
export class RecognitionPanel {
    private readonly closeGuard = new ProposalCloseGuard();
    private inputEdited = false;
    private readonly composingTargets = new Set<EventTarget>();
    // Revocation scrubs private display state; opaque identities keep unresolved work
    // from becoming close proof merely because its old UI was retired.
    private readonly retiredOperations = new Map<string, { operationId?: string; jobId?: string; slot?: string; unreadable: boolean }>();
    private readonly retiredProposals = new Set<string>();
    private get unresolved(): boolean {
        const terminal = !this.controller.uncertain && !!this.controller.receipt && ["committed", "resolved", "cancelled"].includes(this.controller.receipt.status);
        return this.retiredOperations.size !== 0 || this.retiredProposals.size !== 0 || this.unreadable || this.reading || this.controller.busy || this.controller.uncertain ||
            !terminal && (!!this.controller.locator || !!this.controller.pendingJobId || !!this.file || !!this.review || this.inputEdited);
    }
    assertCloseReady(): void {
        try { this.closeGuard.assertReady(this.unresolved); }
        catch (error) {
            if (this.identity && this.identity === this.host.identity() && !this.element.open) this.element.showModal();
            throw error;
        }
    }
    freezeForClose(): boolean {
        if (!this.closeGuard.freeze(this.unresolved)) return false;
        this.stopPoll();
        return true;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); this.schedule(); }
    private discardProposal(): void {
        try {
            this.closeGuard.assertMutable();
            const terminal = !this.controller.uncertain && !!this.controller.receipt && ["committed", "resolved", "cancelled"].includes(this.controller.receipt.status);
            this.reconcileRetiredOperation();
            this.closeGuard.assertReady(this.retiredOperations.has(this.identity) || this.unreadable || this.controller.busy || this.controller.uncertain || !!this.controller.locator && !terminal || !!this.controller.pendingJobId && !recognitionTerminal(this.controller.job));
            if (this.controller.locator) this.controller.finish();
            this.retiredProposals.delete(this.identity);
            this.retirePrivate(false, true);
        } catch (error) { this.host.notify?.(errorUiText(error), true); }
    }
    readonly element = ui("dialog", "modal recognition-panel");
    private readonly controller: RecognitionController;
    private identity = "";
    private showActivityOriginal(id: string, kind: "job" | "import", identity: string): void {
        this.closeGuard.assertMutable();
        if (identity !== this.identity || identity !== this.host.identity() ||
            (kind === "job" ? this.controller.pendingJobId !== id && this.controller.job?.jobId !== id : this.controller.locator?.operationId !== id))
            throw new UiError(uiMessage("workActivity.originalUnavailable"));
        if (!this.element.open) this.element.showModal();
    }
    private publishActivity(): void {
        if (!this.identity || this.identity !== this.host.identity()) return;
        const identity = this.identity, job = this.controller.job, jobId = job?.jobId ?? this.controller.pendingJobId;
        if (jobId) {
            const status: WorkTaskStatus = job?.status === "pending" || job?.status === "running" || !job && this.controller.busy ? "running"
                : job?.status === "complete" ? "succeeded" : job?.status === "cancelled" ? "cancelled"
                : job?.status === "failed" || job?.status === "interrupted" ? "failed" : "waiting";
            const detail = job ? statuses[job.status] : uiMessage("recognition.notInspectedYet"),
                actions = [{ id: "details", label: uiMessage("workActivity.details"), run: () => this.showActivityOriginal(jobId, "job", identity) }];
            workActivity.task({ id: jobId, title: uiMessage("recognition.title"), detail, status, actions });
            if (job?.result && ["complete", "partial"].includes(job.status))
                workActivity.result({ id: jobId, title: this.file?.name ?? uiMessage("recognition.title"), detail, actions });
        }
        const locator = this.controller.locator, receipt = this.controller.receipt;
        if (locator) {
            const id = locator.operationId;
            const actions = [{ id: "details", label: uiMessage("workActivity.details"), run: () => this.showActivityOriginal(id, "import", identity) }];
            const status: WorkTaskStatus = this.controller.busy ? "running" : this.controller.uncertain ? "waiting"
                : receipt?.status === "committed" || receipt?.status === "resolved" ? "succeeded"
                : receipt?.status === "cancelled" ? "cancelled" : "waiting";
            const detail = this.controller.uncertain ? uiMessage("recognition.unknownInspectOriginalFirst") : statuses[receipt?.status ?? ""] ?? uiMessage("recognition.notInspectedYet");
            workActivity.task({ id, title: uiMessage("recognition.originalImportTitle"), detail, status, actions });
            if (receipt?.status === "committed")
                workActivity.result({ id, title: uiMessage("recognition.importCommittedHelp"), detail: receipt.preview.summary, actions });
        }
    }
    private cacheKey = "";
    private epoch = 0;
    private poll?: ReturnType<typeof setTimeout>;
    private capabilities?: RecognitionCapabilities;
    private file?: File;
    private upload?: string;
    private uploadDigest?: string;
    private reading = false;
    private unreadable = false;
    private review?: RecognitionReviewDraft;
    private reviewDigest?: string;
    private fields: Field[] = [];
    private fieldGeneration = 0;
    private mappings = new Map<string, string>();
    private regionRows: RegionRow[] = [];
    private readonly regionEditor = ui("section", "recognition-region-editor");
    private readonly regionList = ui("div");
    private previewEpoch = 0;
    private previewUrl?: string;
    private downloadUrls = new Set<string>();
    private acceptedPlan = false;
    private optionsSnapshot?: RecognitionOptions;
    private readonly purpose = select(purposes, uiMessage("recognition.processingPurposeAria"));
    private readonly format = select([["pdf", "PDF"], ["png", uiMessage("recognition.formatPng")], ["jpeg", uiMessage("recognition.formatJpeg")], ["docx", uiMessage("recognition.formatOfficialDocx")]] as const, uiMessage("recognition.formatAria"));
    private readonly fileInput = input(uiMessage("recognition.originalAria"));
    private readonly model = select([], uiMessage("recognition.modelSetAria"));
    private readonly scheme = select([["none", uiMessage("recognition.noSpecializedScheme")]] as const, uiMessage("recognition.schemeAria"));
    private readonly language = select([["", uiMessage("recognition.notDeclared")], ["zh", uiMessage("recognition.languageChinese")], ["en", uiMessage("recognition.languageEnglish")]] as const, uiMessage("recognition.sourceLanguage"));
    private readonly script = select([["", uiMessage("common.unknown")], ["Hans", uiMessage("recognition.scriptSimplified")], ["Hant", uiMessage("recognition.scriptTraditional")], ["Latn", uiMessage("recognition.scriptLatin")]] as const, uiMessage("recognition.sourceScript"));
    private readonly region = select([["", uiMessage("recognition.regionUnknown")], ["CN", uiMessage("recognition.regionMainlandChina")], ["TW", uiMessage("recognition.regionTaiwanChina")], ["HK", uiMessage("recognition.regionHongKongChina")], ["US", uiMessage("recognition.regionUnitedStates")]] as const, uiMessage("recognition.sourceRegion"));
    private readonly pages = input(uiMessage("recognition.pagesAria"), "1");
    private readonly rect = input(uiMessage("recognition.rectangleAria"));
    private readonly mode = select([["auto", uiMessage("recognition.methodAutoFull")], ["text", uiMessage("recognition.methodTextLayerOnly")], ["ocr", uiMessage("recognition.methodExplicitOcr")]] as const, uiMessage("recognition.pageRegionMethodAria"));
    private readonly rotation = select([["0", uiMessage("recognition.rotationNone")], ["90", uiMessage("recognition.rotationClockwiseNinety")], ["180", "180°"], ["270", uiMessage("recognition.rotationClockwiseTwoSeventy")]] as const, uiMessage("recognition.rotationAria"));
    private readonly segmentation = select([["auto", uiMessage("recognition.layoutAutomatic")], ["sparse", uiMessage("recognition.layoutSparseText")], ["block", uiMessage("recognition.layoutTextBlock")], ["line", uiMessage("recognition.layoutSingleLine")]] as const, uiMessage("recognition.layoutAria"));
    private readonly dpi = input(uiMessage("recognition.dpiAria"), "300");
    private readonly parent = input(uiMessage("recognition.newNoteParent"));
    private readonly name = input(uiMessage("recognition.newNoteName"), "资料导入");
    private readonly slot = select([["", uiMessage("recognition.chooseAuthorizedEmptySlot")]] as const, uiMessage("recognition.fixedImportSlotAria"));
    private readonly status = ui("p", "recognition-status");
    private readonly runtime = ui("p", "data-help");
    private readonly selection = ui("p", "data-help");
    private readonly results = ui("section", "recognition-results");
    private readonly original = ui("section", "recognition-original");
    private readonly proposals = ui("section", "recognition-proposals");
    private readonly recovery = ui("section", "recognition-recovery");
    private readonly controls = ui("div", "recognition-grid");
    private readonly startButton = action(uiMessage("recognition.startJob"), () => { void this.run(() => this.start()); });
    private readonly inspectButton = action(uiMessage("recognition.inspectOriginalOnly"), () => { void this.run(() => this.start(true)); });
    private readonly statusButton = action(uiMessage("recognition.inspectJob"), () => { void this.run(async () => { await this.controller.status(); this.adopt(); }); });
    private readonly cancelButton = action(uiMessage("recognition.cancelJob"), () => { void this.run(async () => { this.stopPoll(); await this.controller.cancel(); this.clearPreview(); this.review = undefined; this.adopt(); }); });
    private readonly prepareButton = action(uiMessage("recognition.prepareReviewedImport"), () => { void this.run(() => this.prepare()); });
    private readonly downloadButton = action(uiMessage("recognition.downloadExtractedText"), () => { void this.run(() => this.download()); });
    private readonly downloadEvidenceButton = action(uiMessage("recognition.downloadProvenanceAndLosses"), () => { void this.run(() => this.download(true)); });
    constructor(private readonly host: RecognitionPanelHost, mount: HTMLElement = document.body) {
        this.element.addEventListener("compositionstart", event => { if (event.target) this.composingTargets.add(event.target); }, true);
        this.element.addEventListener("compositionend", event => { if (event.target) this.composingTargets.delete(event.target); }, true);
        this.controller = new RecognitionController({ request: (path, body) => host.request(path, body), identity: () => host.identity(), target: () => host.target(), committed: identity => host.committed?.(identity), id: () => crypto.randomUUID(), persist: locator => this.persist(locator), remove: () => {
                if (this.cacheKey)
                    sessionStorage.removeItem(this.cacheKey);
            } });
        setUiAttr(this.element, "aria-label", uiMessage("recognition.title"));
        this.fileInput.type = "file";
        this.fileInput.accept = ".pdf,.png,.jpg,.jpeg,.docx";
        this.dpi.type = "number";
        this.dpi.min = "72";
        this.dpi.max = "300";
        this.controls.append(label(uiMessage("recognition.purposeLabel"), this.purpose), label(uiMessage("recognition.formatLabel"), this.format), label(uiMessage("recognition.chooseOriginalLabel"), this.fileInput), label(uiMessage("recognition.modelsLabel"), this.model), label(uiMessage("recognition.pagesLabel"), this.pages), label(uiMessage("recognition.rectangleLabel"), this.rect), label(uiMessage("recognition.methodLabel"), this.mode), label(uiMessage("recognition.rotationLabel"), this.rotation), label(uiMessage("recognition.layoutLabel"), this.segmentation), label("DPI（72–300）", this.dpi), label(uiMessage("recognition.schemeLabel"), this.scheme), label(uiMessage("recognition.sourceLanguage"), this.language), label(uiMessage("recognition.sourceScript"), this.script), label(uiMessage("recognition.sourceRegion"), this.region), label(uiMessage("recognition.newNoteParent"), this.parent), label(uiMessage("recognition.newNoteName"), this.name), label(uiMessage("recognition.hostedEmptySlotLabel"), this.slot));
        const actions = ui("div", "recognition-actions");
        actions.append(this.inspectButton, this.startButton, this.statusButton, this.cancelButton, this.downloadButton, this.downloadEvidenceButton);
        this.regionEditor.append(ui("h3", "", uiMessage("recognition.regionDetailsTitle")), ui("p", "data-help", uiMessage("recognition.regionDetailsHelp")), this.regionList, action(uiMessage("recognition.addRegion"), () => { this.regionRows.push({ id: `region-${this.regionRows.length + 1}`, page: this.pages.value.split(/[,，-]/)[0] || "1", rect: this.rect.value, mode: this.mode.value, rotation: this.rotation.value, segmentation: this.segmentation.value, scale: "1" }); this.changed(); this.drawRegions(); }));
        const compare = ui("div", "recognition-compare");
        compare.append(this.original, this.proposals);
        this.results.append(compare);
        this.element.append(ui("h2", "", uiMessage("recognition.title")), ui("p", "data-help", uiMessage("recognition.reviewOriginalHelp")), this.runtime, this.controls, this.regionEditor, this.selection, actions, ui("p", "data-help", uiMessage("recognition.downloadVersusReviewHelp")), this.status, this.results, this.prepareButton, this.recovery, action(uiMessage("recognition.closeRetainingReview"), () => this.element.close()));
        mount.append(this.element);
        this.closeGuard.bind(this.element);
        this.element.append(action(uiMessage("common.cancel"), () => this.discardProposal()));
        this.status.setAttribute("role", "status");
        this.status.setAttribute("aria-live", "polite");
        for (const control of [this.purpose, this.format, this.model, this.scheme, this.language, this.script, this.region, this.mode, this.rotation, this.segmentation, this.slot])
            control.onchange = () => {
                this.changed();
                if (control === this.slot)
                    void this.loadFields();
            };
        for (const control of [this.pages, this.rect, this.dpi, this.parent, this.name])
            control.oninput = () => this.changed();
        this.fileInput.onchange = () => {
            const file = this.fileInput.files?.[0];
            if (file)
                void this.choose(file);
        };
        this.element.addEventListener("cancel", () => this.stopPoll());
        this.element.addEventListener("close", () => this.stopPoll());
    }
    private persist(locator: RecognitionLocator): void {
        if (!this.cacheKey || this.unreadable)
            throw new UiError(uiMessage("recognition.originalIdNotDurable"));
        try {
            sessionStorage.setItem(this.cacheKey, JSON.stringify(locator));
        }
        catch {
            throw new UiError(uiMessage("recognition.originalIdStorageRejected"));
        }
    }
    private stopPoll(): void {
        if (this.poll)
            clearTimeout(this.poll);
        this.poll = undefined;
    }
    private clearPreview(): void {
        this.previewEpoch++;
        if (this.previewUrl)
            URL.revokeObjectURL(this.previewUrl);
        this.previewUrl = undefined;
        replaceUiChildren(this.original, );
    }
    private retirePrivate(preserveLocator: boolean, discardProposal = false): void {
        if (!discardProposal) {
            const terminal = !this.controller.uncertain && !!this.controller.receipt && ["committed", "resolved", "cancelled"].includes(this.controller.receipt.status);
            if (!terminal && (this.file || this.review || this.inputEdited || this.composingTargets.size !== 0)) this.retiredProposals.add(this.identity);
            if (!terminal && (this.controller.locator || this.controller.pendingJobId && !recognitionTerminal(this.controller.job) || this.unreadable)) {
                this.retiredOperations.set(this.identity, {
                    operationId: this.controller.locator?.operationId,
                    jobId: this.controller.pendingJobId,
                    slot: this.controller.locator && "slot" in this.controller.locator.target ? this.controller.locator.target.slot : undefined,
                    unreadable: this.unreadable,
                });
            }
        }
        // Opaque proposal/operation markers above survive destruction of these inputs.
        // Retire only nodes actually observed by this owner, never infer completion from DOM loss.
        for (const target of this.composingTargets) this.closeGuard.retireComposition(target);
        this.composingTargets.clear();
        const keptIdentity = this.identity, keptKey = this.cacheKey;
        this.epoch++;
        this.fieldGeneration++;
        this.stopPoll();
        this.clearPreview();
        for (const url of this.downloadUrls)
            URL.revokeObjectURL(url);
        this.downloadUrls.clear();
        let warning: UiText = "";
        try {
            if (preserveLocator)
                this.controller.hide();
            else
                this.controller.revoke();
        }
        catch {
            warning = uiMessage("recognition.recoveryIdRemovalRejected");
        }
        this.file = undefined;
        this.inputEdited = false;
        this.upload = this.uploadDigest = undefined;
        this.fileInput.value = "";
        this.purpose.value = "resource";
        this.format.value = "pdf";
        this.pages.value = "1";
        this.rect.value = "";
        this.mode.value = "auto";
        this.rotation.value = "0";
        this.segmentation.value = "auto";
        this.dpi.value = "300";
        this.scheme.value = "none";
        this.language.value = this.script.value = this.region.value = "";
        this.parent.value = "";
        this.name.value = "资料导入";
        replaceUiChildren(this.slot, );
        this.regionRows = [];
        replaceUiChildren(this.regionList, );
        setUiText(this.selection, "");
        this.review = undefined;
        this.reviewDigest = undefined;
        this.fields = [];
        this.mappings.clear();
        this.acceptedPlan = false;
        this.optionsSnapshot = undefined;
        this.capabilities = undefined;
        this.identity = preserveLocator ? keptIdentity : "";
        this.cacheKey = preserveLocator ? keptKey : "";
        this.unreadable = this.reading = false;
        replaceUiChildren(this.proposals, );
        replaceUiChildren(this.recovery, );
        setUiText(this.status, "");
        if (this.element.open)
            this.element.close();
        if (warning)
            this.host.notify?.(warning, true);
    }
    clear(): void { this.retirePrivate(false); }
    revoke(): void { this.clear(); }
    retireDisplay(preserveLocator = true): void { this.retirePrivate(preserveLocator); }
    async open(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const identity = this.host.identity();
            if (!identity) {
                this.host.notify?.(uiMessage("recognition.authorizedContextRequired"), true);
                return;
            }
            if (identity !== this.identity) {
                this.retireDisplay(true);
                this.controller.detach();
                this.identity = identity;
                this.cacheKey = `weftext-recognition-operation:${encodeURIComponent(identity)}`;
                try {
                    const raw = sessionStorage.getItem(this.cacheKey);
                    if (raw)
                        this.controller.restore(decodeRecognitionLocator(raw, identity));
                }
                catch (error) {
                    this.unreadable = true;
                    setUiText(this.status, errorUiText(error));
                }
            }
            const retired = this.retiredOperations.get(identity);
            if (retired && !this.controller.locator && !this.unreadable) {
                if (retired.operationId) {
                    const currentTarget = this.host.target();
                    // Operation status/settle is scoped by the authorized library or
                    // slot. Restoring only that scope cannot recreate a commit binding.
                    const target = retired.slot ? { slot: retired.slot } : currentTarget && "sessionId" in currentTarget ? { sessionId: currentTarget.sessionId } : null;
                    if (target) this.controller.restore({ version: 1, identity, operationId: retired.operationId, target });
                } else if (retired.jobId) this.controller.pendingJobId = retired.jobId;
            }
            if (!this.element.open)
                this.element.showModal();
            const epoch = this.epoch;
            this.draw();
            await this.run(async () => {
                const raw = await this.host.request<unknown>("/capabilities");
                if (epoch !== this.epoch || identity !== this.host.identity())
                    return;
                const value = decodeRecognitionCapabilities(raw);
                this.capabilities = value;
                const selectedModel = this.model.value, selectedScheme = this.scheme.value;
                replaceUiChildren(this.model, ...value.models.map(model => { const option = ui("option", "", `${model.id} · ${model.languages.join(" + ")}`); option.value = model.id; return option; }));
                if (value.models.some(m => m.id === selectedModel))
                    this.model.value = selectedModel;
                if (!value.models.length) {
                    const option = ui("option", "", uiMessage("recognition.noInstalledModels"));
                    option.value = "";
                    this.model.append(option);
                }
                replaceUiChildren(this.scheme, ...([["none", uiMessage("recognition.noSpecializedScheme")], ...value.schemes.map(s => [s.id, s.title] as [
                        string,
                        UiText
                    ])] as [
                    string,
                    UiText
                ][]).map(([id, title]) => { const option = ui("option", "", title); option.value = id; return option; }));
                this.scheme.value = value.schemes.some(s => s.id === selectedScheme) ? selectedScheme : "none";
                const target = this.host.target();
                const hosted = !!target && "slot" in target || value.executor === "selfHosted";
                this.parent.parentElement!.hidden = hosted;
                this.slot.parentElement!.hidden = !hosted;
                const slotChoices: [
                    string,
                    UiText
                ][] = [["", uiMessage("recognition.chooseAuthorizedEmptySlot")], ...(value.targets ?? []).map(t => [t.id, () => t.title + uiTextValue(t.available ? "" : uiMessage("recognition.slotCannotCreateSuffix"))] as [
                        string,
                        UiText
                    ])];
                replaceUiChildren(this.slot, ...slotChoices.map(([id, title]) => { const option = ui("option", "", title); option.value = id; const found = value.targets?.find(t => t.id === id); option.disabled = !!found && !found.available; return option; }));
                if (target && "slot" in target)
                    this.slot.value = target.slot;
                else if (target) {
                    this.parent.value ||= target.parent ?? "";
                    if (target.name)
                        this.name.value = target.name;
                }
                await this.loadFields();
                if (this.controller.locator && !this.unreadable)
                    await this.controller.lookup();
                else if (this.controller.pendingJobId)
                    await this.controller.status();
                this.adopt();
            });
            this.schedule();
        } finally { endCloseWork(); }
    }
    private target(): RecognitionTarget {
        const current = this.host.target();
        if (this.capabilities?.executor === "selfHosted" || current && "slot" in current) {
            if (!this.slot.value)
                throw new UiError(uiMessage("recognition.authorizedFixedEmptySlotRequired"));
            return { slot: this.slot.value };
        }
        if (!current || !("sessionId" in current))
            throw new UiError(uiMessage("recognition.authorizedLocalNoteRequired"));
        return { sessionId: current.sessionId, parent: this.parent.value, name: this.name.value };
    }
    private async loadFields(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const generation = ++this.fieldGeneration, identity = this.identity;
            this.fields = [];
            try {
                const target = this.target();
                const value = await this.host.request<{
                    fields: Field[];
                }>("/fields", { target });
                if (generation !== this.fieldGeneration || identity !== this.host.identity())
                    return;
                this.fields = value.fields.filter(f => f.id && f.typeId && f.shape?.kind === "scalar");
                if (this.review)
                    this.drawReview();
            }
            catch { /* Missing scope offers no mappings; body-only remains explicit. */ }
        } finally { endCloseWork(); }
    }
    private drawRegions(): void {
        replaceUiChildren(this.regionList, );
        this.regionRows.forEach((row, index) => {
            const card = ui("fieldset", "recognition-region-row"), grid = ui("div", "recognition-grid");
            card.append(ui("legend", "", displayMessage("recognition.regionLegend", { p0: index + 1 })));
            const controls: [
                keyof RegionRow,
                UiText,
                HTMLInputElement | HTMLSelectElement
            ][] = [["id", uiMessage("recognition.regionNamePlaceholder"), input(displayMessage("recognition.regionNameAria", { p0: index + 1 }), row.id)], ["page", uiMessage("recognition.regionPagePlaceholder"), input(displayMessage("recognition.regionPageAria", { p0: index + 1 }), row.page)], ["rect", uiMessage("recognition.regionCoordinatesPlaceholder"), input(displayMessage("recognition.regionCoordinatesAria", { p0: index + 1 }), row.rect)], ["mode", uiMessage("recognition.methodLabel"), select([["auto", uiMessage("recognition.methodAutoShort")], ["text", uiMessage("recognition.methodTextLayerOnly")], ["ocr", "OCR"]] as const, displayMessage("recognition.regionMethodAria", { p0: index + 1 }))], ["rotation", uiMessage("recognition.regionRotationLabel"), select([["0", "0°"], ["90", "90°"], ["180", "180°"], ["270", "270°"]], displayMessage("recognition.regionRotationAria", { p0: index + 1 }))], ["segmentation", uiMessage("recognition.regionLayoutLabel"), select([["auto", uiMessage("recognition.automatic")], ["sparse", uiMessage("recognition.layoutSparseText")], ["block", uiMessage("recognition.regionLayoutBlock")], ["line", uiMessage("recognition.regionLayoutLine")]] as const, displayMessage("recognition.regionLayoutAria", { p0: index + 1 }))], ["scale", uiMessage("recognition.regionScaleLabel"), select([["1", uiMessage("recognition.regionScaleOriginal")], ["0.75", uiMessage("recognition.regionScaleThreeQuarters")], ["0.5", uiMessage("recognition.regionScaleHalf")]] as const, displayMessage("recognition.regionScaleAria", { p0: index + 1 }))]];
            for (const [key, title, control] of controls) {
                control.value = row[key];
                const update = (): void => { row[key] = control.value; this.changed(); };
                if (control.tagName === "SELECT")
                    control.onchange = update;
                else
                    control.oninput = update;
                grid.append(label(title, control));
            }
            card.append(grid, action(displayMessage("recognition.removeRegion", { p0: index + 1 }), () => { this.regionRows.splice(index, 1); this.changed(); this.drawRegions(); }));
            this.regionList.append(card);
        });
        this.draw();
    }
    private selectedPages(pages: number[], uniform?: RecognitionRect): RecognitionOptions["pages"] {
        const ids = new Set<string>();
        const selections: RecognitionOptions["pages"] = pages.map(page => ({ page, ...(uniform ? { regions: [{ id: `page-${page}-selected`, rect: uniform, mode: this.mode.value as "auto", rotation: Number(this.rotation.value) as 0, segmentation: this.segmentation.value as "auto", ocrScale: 1 }] } : {}) }));
        for (const page of pages) {
            const rows = this.regionRows.filter(r => Number(r.page) === page);
            if (!rows.length) {
                if (!uniform && (this.mode.value !== "auto" || this.rotation.value !== "0" || this.segmentation.value !== "auto"))
                    throw new UiError(displayMessage("recognition.explicitPageRectangleRequired", { p0: page }));
                continue;
            }
            if (rows.length > 64)
                throw new UiError(uiMessage("recognition.regionPageLimit"));
            const regions = rows.map(row => {
                if (!/^[A-Za-z0-9_-]{1,64}$/.test(row.id) || ids.has(row.id))
                    throw new UiError(uiMessage("recognition.regionNameInvalid"));
                ids.add(row.id);
                const rect = parseRecognitionRect(row.rect);
                if (!rect)
                    throw new UiError(displayMessage("recognition.regionRectangleRequired", { p0: uiDisplayParameter(row.id) }));
                return { id: row.id, rect, mode: row.mode as "auto", rotation: Number(row.rotation) as 0, segmentation: row.segmentation as "auto", ocrScale: Number(row.scale) };
            });
            if (regions.some((a, i) => regions.slice(i + 1).some(b => a.rect[0] < b.rect[2] && a.rect[2] > b.rect[0] && a.rect[1] < b.rect[3] && a.rect[3] > b.rect[1])))
                throw new UiError(displayMessage("recognition.overlappingRegions", { p0: page }));
            selections.find(p => p.page === page)!.regions = regions;
        }
        if (this.regionRows.some(r => !Number.isInteger(Number(r.page)) || !pages.includes(Number(r.page))))
            throw new UiError(uiMessage("recognition.regionPageNotSelected"));
        return selections;
    }
    private changed(): void {
        this.closeGuard.assertMutable();
        this.inputEdited = true;
        if (this.controller.locator)
            return;
        this.epoch++;
        this.stopPoll();
        this.controller.retire();
        this.review = undefined;
        this.reviewDigest = undefined;
        this.optionsSnapshot = undefined;
        this.mappings.clear();
        this.acceptedPlan = false;
        this.clearPreview();
        replaceUiChildren(this.proposals, );
        setUiText(this.status, uiMessage("recognition.inputsChangedNewJobRequired"));
        this.draw();
    }
    private async choose(file: File): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            this.regionRows = [];
            replaceUiChildren(this.regionList, );
            this.rect.value = "";
            this.pages.value = "1";
            this.changed();
            this.file = file;
            this.upload = this.uploadDigest = undefined;
            this.reading = true;
            const epoch = this.epoch;
            const extension = file.name.split(".").at(-1)?.toLowerCase();
            this.format.value = extension === "jpg" ? "jpeg" : extension ?? "";
            this.draw();
            try {
                if (!this.format.value || file.size > 16 * 1024 * 1024 || !file.size)
                    throw new UiError(uiMessage("recognition.originalFileInvalid"));
                const raw = new Uint8Array(await file.arrayBuffer()), digest = await sha(raw);
                if (epoch !== this.epoch || this.file !== file || this.identity !== this.host.identity())
                    return;
                this.upload = base64(raw);
                this.uploadDigest = digest;
                setUiText(this.status, displayMessage("recognition.originalReadSummary", { p0: uiDisplayParameter(file.name), p1: file.size, p2: uiDisplayParameter(digest) }));
            }
            catch (error) {
                if (epoch === this.epoch)
                    setUiText(this.status, errorUiText(error));
            }
            finally {
                if (epoch === this.epoch) {
                    this.reading = false;
                    this.draw();
                }
            }
        } finally { endCloseWork(); }
    }
    private options(inspect: boolean): RecognitionOptions {
        const pages = parseRecognitionPages(this.pages.value), rect = parseRecognitionRect(this.rect.value), dpi = Number(this.dpi.value), model = this.capabilities?.models.find(m => m.id === this.model.value);
        if (!Number.isInteger(dpi) || dpi < 72 || dpi > 300)
            throw new UiError(uiMessage("recognition.dpiInvalid"));
        if ((this.mode.value !== "auto" || this.rotation.value !== "0" || this.segmentation.value !== "auto") && !rect && !this.regionRows.length)
            throw new UiError(uiMessage("recognition.regionRequiredForProcessingOptions"));
        if (this.format.value === "docx" && !this.scheme.value.startsWith("official-document"))
            throw new UiError(uiMessage("recognition.docxSchemeRequired"));
        if (!inspect && this.purpose.value !== "resource" && this.format.value !== "docx" && (!model || !this.capabilities?.available))
            throw new UiError(uiMessage("recognition.modelsUnavailable"));
        return { purpose: this.purpose.value as RecognitionPurpose, ...(inspect ? { operation: "inspect" as const } : {}), languages: model?.languages ?? ["eng"], pages: this.selectedPages(pages, rect), dpi, scheme: this.scheme.value, language: this.language.value || undefined, script: this.script.value || undefined, region: this.region.value || undefined, numbering: "keep" };
    }
    private async run(work: () => Promise<void>): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const epoch = this.epoch, identity = this.identity;
            try {
                const promise = work();
                this.draw();
                await promise;
            }
            catch (error) {
                if (epoch === this.epoch && identity === this.host.identity()) {
                    setUiText(this.status, errorUiText(error));
                    this.status.classList.add("data-warning");
                }
            }
            finally {
                this.reconcileRetiredOperation();
                if (epoch === this.epoch && identity === this.host.identity())
                    this.draw();
            }
        } finally { endCloseWork(); }
    }
    private reconcileRetiredOperation(): void {
        const retired = this.retiredOperations.get(this.identity);
        if (!retired || this.identity !== this.host.identity() || retired.unreadable) return;
        if (retired.operationId) {
            if (this.controller.receipt?.operationId !== retired.operationId || this.controller.uncertain || !["committed", "resolved", "cancelled"].includes(this.controller.receipt.status)) return;
        } else if (retired.jobId && (this.controller.job?.jobId !== retired.jobId || !recognitionTerminal(this.controller.job))) return;
        this.retiredOperations.delete(this.identity);
    }
    private async start(inspect = false): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (!this.upload || this.reading)
                throw new UiError(uiMessage("recognition.originalReadRequired"));
            const options = this.options(inspect), target = this.target();
            this.review = undefined;
            this.reviewDigest = undefined;
            this.clearPreview();
            replaceUiChildren(this.proposals, );
            this.optionsSnapshot = structuredClone(options);
            setUiText(this.status, inspect ? uiMessage("recognition.inspectingOriginal") : uiMessage("recognition.backgroundRequested"));
            await this.controller.start(target, this.format.value, this.upload, options);
            this.adopt();
            this.schedule();
        } finally { endCloseWork(); }
    }
    private schedule(): void {
        this.stopPoll();
        if (this.closeGuard.frozen || !this.element.open || this.controller.busy || !this.controller.pendingJobId || recognitionTerminal(this.controller.job))
            return;
        const epoch = this.epoch;
        this.poll = setTimeout(() => {
            if (epoch !== this.epoch || this.identity !== this.host.identity())
                return;
            void this.run(async () => { await this.controller.status(); this.adopt(); this.schedule(); });
        }, 900);
    }
    private adopt(): void {
        const job = this.controller.job;
        if (!job)
            return;
        if (this.uploadDigest && job.inputSha256 !== this.uploadDigest)
            throw new UiError(uiMessage("recognition.originalDigestMismatch"));
        this.status.classList.remove("data-warning");
        setUiText(this.status, displayMessage("recognition.jobSummary", { p0: uiDisplayParameter(statuses[job.status] ?? job.status), p1: uiDisplayParameter(job.confirmationExpired ? uiMessage("recognition.confirmationExpiredSuffix") : ""), p2: uiDisplayParameter(""), p3: uiDisplayParameter(job.jobId) }));
        if (job.result && (job.status === "interrupted" || job.result.workerFinalValidation === false)) {
            this.review = undefined;
            this.reviewDigest = undefined;
            this.clearPreview();
            this.drawCheckpoint();
            this.draw();
            return;
        }
        if (!job.result || !["complete", "partial"].includes(job.status)) {
            this.review = undefined;
            this.reviewDigest = undefined;
            replaceUiChildren(this.proposals, );
            this.clearPreview();
        }
        if (job.result && recognitionTerminal(job) && ["complete", "partial"].includes(job.status) && this.reviewDigest !== (job.resultDigest ?? job.jobId)) {
            this.review = undefined;
            this.review = new RecognitionReviewDraft(job.result, job.scheme ?? undefined);
            this.reviewDigest = job.resultDigest ?? job.jobId;
            this.mappings.clear();
            this.drawReview();
        }
        if (job.confirmationExpired) {
            this.clearPreview();
            this.drawReview();
        }
        this.draw();
    }
    private drawCheckpoint(): void {
        const evidence = this.controller.job?.result;
        replaceUiChildren(this.proposals, );
        if (!evidence)
            return;
        const selected = this.optionsSnapshot?.pages.map(p => p.page) ?? [], returned = evidence.pages.map(p => p.page);
        const missing = [...new Set([...selected.filter(page => !returned.includes(page)), ...evidence.pages.filter(p => p.losses.includes("unprocessed-after-interruption")).map(p => p.page)])];
        this.proposals.append(ui("h3", "", uiMessage("recognition.interruptedCheckpointTitle")), ui("p", "data-warning", uiMessage("recognition.interruptedCheckpointHelp")), ui("p", "data-help", displayMessage("recognition.checkpointPageSummary", { p0: uiDisplayParameter(evidence.pageCount ?? uiMessage("common.unknown")), p1: uiDisplayParameter(selected.join("、") || uiMessage("recognition.frozenSelectionFallback")), p2: uiDisplayParameter(returned.join("、") || uiMessage("recognition.none")) })), ui("p", "data-warning", displayMessage("recognition.unprocessedPages", { p0: uiDisplayParameter(missing.join("、") || uiMessage("recognition.noAdditionalMissingPages")) })));
        for (const page of evidence.pages) {
            const unprocessed = missing.includes(page.page);
            this.proposals.append(ui("h4", "", displayMessage("recognition.pageStatus", { p0: page.page, p1: uiDisplayParameter(unprocessed ? uiMessage("recognition.pageUnprocessedInterrupted") : statuses[page.status] ?? page.status) })));
            for (const region of page.regions) {
                this.proposals.append(ui("p", "data-help", displayMessage("recognition.originalRegionStatus", { p0: uiDisplayParameter(region.id), p1: uiDisplayParameter(statuses[region.status] ?? region.status) })));
                for (const block of region.blocks) {
                    const raw = ui("pre", "recognition-raw", block.text);
                    setUiAttr(raw, "aria-label", displayMessage("recognition.checkpointTextAria", { p0: uiDisplayParameter(block.id) }));
                    this.proposals.append(raw);
                }
                if (!region.blocks.length && region.rawText)
                    this.proposals.append(ui("pre", "recognition-raw", region.rawText));
                for (const loss of region.losses)
                    this.proposals.append(ui("p", "data-warning", issueLabel(loss)));
            }
            for (const loss of page.losses)
                this.proposals.append(ui("p", "data-warning", issueLabel(loss)));
        }
        for (const block of evidence.docxBlocks ?? [])
            this.proposals.append(ui("pre", "recognition-raw", block.text));
        for (const loss of evidence.losses ?? [])
            this.proposals.append(ui("p", "data-warning", issueLabel(loss)));
    }
    private async showOriginal(page: RecognitionPage, rect?: RecognitionRect): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const job = this.controller.job;
            if (!job || !recognitionActionable(job) || this.identity !== this.host.identity())
                throw new UiError(uiMessage("recognition.readOnlyJobNoPreview"));
            this.clearPreview();
            const stamp = this.previewEpoch, epoch = this.epoch;
            setUiText(this.original, displayMessage("recognition.loadingOriginalPage", { p0: page.page }));
            const value = await this.host.request<{
                jobId: string;
                page: number;
                mediaType: string;
                dataBase64: string;
                width: number;
                height: number;
            }>("/preview", { jobId: job.jobId, page: page.page });
            if (stamp !== this.previewEpoch || epoch !== this.epoch || this.identity !== this.host.identity())
                return;
            if (value.jobId !== job.jobId || value.page !== page.page || value.mediaType !== "image/png" || value.width * value.height > 40000000 || value.width <= 0 || value.height <= 0)
                throw new UiError(uiMessage("recognition.originalPreviewIdentityUnconfirmed"));
            const image = new Image(), url = URL.createObjectURL(new Blob([bytes(value.dataBase64)], { type: "image/png" }));
            this.previewUrl = url;
            image.src = url;
            await image.decode();
            if (stamp !== this.previewEpoch || epoch !== this.epoch)
                return;
            if (image.naturalWidth !== value.width || image.naturalHeight !== value.height)
                throw new UiError(uiMessage("recognition.originalPreviewDimensionsMismatch"));
            const canvas = ui("canvas");
            canvas.width = value.width;
            canvas.height = value.height;
            setUiAttr(canvas, "aria-label", displayMessage("recognition.originalPageAlt", { p0: page.page }));
            const context = canvas.getContext("2d");
            if (!context)
                throw new UiError(uiMessage("recognition.canvasUnavailable"));
            context.drawImage(image, 0, 0);
            if (rect) {
                if (!page.width || !page.height || page.coordinateSpace !== "visible-page-top-left" || rect[0] < 0 || rect[1] < 0 || rect[2] > page.width + .01 || rect[3] > page.height + .01)
                    throw new UiError(uiMessage("recognition.originalRegionMappingUnproven"));
                const sx = value.width / page.width, sy = value.height / page.height;
                context.fillStyle = "rgba(255,180,40,.22)";
                context.strokeStyle = "#bd6100";
                context.lineWidth = 3;
                context.fillRect(rect[0] * sx, rect[1] * sy, (rect[2] - rect[0]) * sx, (rect[3] - rect[1]) * sy);
                context.strokeRect(rect[0] * sx, rect[1] * sy, (rect[2] - rect[0]) * sx, (rect[3] - rect[1]) * sy);
            }
            replaceUiChildren(this.original, ui("h3", "", displayMessage("recognition.originalPageTitle", { p0: page.page })), canvas, ui("p", "data-help", displayMessage("recognition.originalDigestAndAnnotations", { p0: uiDisplayParameter(job.inputSha256) })));
        } finally { endCloseWork(); }
    }
    private drawReview(): void {
        replaceUiChildren(this.proposals, );
        const draft = this.review, job = this.controller.job;
        if (!draft || !job?.result)
            return;
        const evidence = job.result, value = draft.value, blocks = recognitionBlocks(evidence), frozen = !!this.controller.locator || !recognitionActionable(job);
        this.proposals.append(ui("h3", "", uiMessage("recognition.noteProposalTitle")), ui("p", "data-help", displayMessage("recognition.evidenceScopeSummary", { p0: uiDisplayParameter(evidence.pageCount ?? uiMessage("common.unknown")), p1: uiDisplayParameter(evidence.pages.map(p => p.page).join("、") || uiMessage("recognition.docxStructuralBlocks")), p2: uiDisplayParameter(statuses[evidence.status]) })));
        const toolbar = ui("div", "recognition-actions"), undo = action(uiMessage("recognition.undoReview"), () => { draft.undo(); this.drawReview(); this.draw(); }), redo = action(uiMessage("recognition.redoReview"), () => { draft.redo(); this.drawReview(); this.draw(); });
        undo.dataset.recognitionUndo = "true";
        redo.dataset.recognitionRedo = "true";
        undo.disabled = frozen || !draft.canUndo;
        redo.disabled = frozen || !draft.canRedo;
        toolbar.append(undo, redo);
        this.proposals.append(toolbar);
        for (const page of evidence.pages) {
            const row = ui("div", "recognition-page-state");
            const viewOriginal = action(uiMessage("recognition.compareOriginalPage"), () => { void this.run(() => this.showOriginal(page)); });
            viewOriginal.disabled = !recognitionActionable(job);
            row.append(ui("strong", "", displayMessage("recognition.pageDimensionsStatus", { p0: page.page, p1: uiDisplayParameter(statuses[page.status] ?? page.status), p2: uiDisplayParameter(page.width ?? "?"), p3: uiDisplayParameter(page.height ?? "?"), p4: uiDisplayParameter(page.units ?? "") })), viewOriginal);
            if (page.readingOrder?.suggested?.length) {
                const order = action(uiMessage("recognition.applyPageReadingOrder"), () => { draft.adoptOrder(page); this.drawReview(); this.draw(); });
                order.disabled = frozen;
                row.append(order);
            }
            for (const region of page.regions)
                row.append(ui("p", "data-help", displayMessage("recognition.regionEvidenceSummary", { p0: uiDisplayParameter(region.id), p1: uiDisplayParameter(region.method === "ocr" ? "OCR" : uiMessage("recognition.textLayer")), p2: uiDisplayParameter(statuses[region.status] ?? region.status), p3: uiDisplayParameter(region.rect.join(", ")) })));
            for (const loss of [...page.losses, ...page.regions.flatMap(r => r.losses)])
                row.append(ui("p", "data-warning", issueLabel(loss)));
            this.proposals.append(row);
        }
        value.blocks.forEach((block, index) => {
            const raw = blocks.find(b => b.id === block.id)!;
            const row = ui("fieldset", "recognition-block");
            row.append(ui("legend", "", displayMessage("recognition.readingOrderBlockSummary", { p0: index + 1, p1: uiDisplayParameter(raw.page ? displayMessage("recognition.pageLabel", { p0: raw.page }) : displayMessage("recognition.wordBlockLabel", { p0: (raw.blockIndex ?? index) + 1 })), p2: uiDisplayParameter(raw.sourceMethod) })), ui("p", "data-help", confidenceLabel(raw.confidence)));
            const original = ui("details");
            original.append(ui("summary", "", uiMessage("recognition.viewOriginalText")), ui("pre", "recognition-raw", raw.text));
            const text = ui("textarea", "data-input");
            setUiAttr(text, "aria-label", displayMessage("recognition.reviewTextAria", { p0: uiDisplayParameter(block.id) }));
            text.rows = 3;
            text.value = block.text;
            text.disabled = frozen;
            text.oninput = () => { draft.change(v => { v.blocks.find(b => b.id === block.id)!.text = text.value; }); this.draw(); };
            row.append(original, label(uiMessage("recognition.editableReviewTextHelp"), text), check(displayMessage("recognition.retainTextBlock", { p0: uiDisplayParameter(block.id) }), block.included, included => { draft.change(v => { v.blocks.find(b => b.id === block.id)!.included = included; }); this.drawReview(); this.draw(); }));
            const actions = ui("div", "recognition-actions");
            for (const [delta, title] of [[-1, uiMessage("recognition.moveBlockUp")], [1, uiMessage("recognition.moveBlockDown")]] as const) {
                const button = action(displayMessage("recognition.moveTextBlockAria", { p0: uiDisplayParameter(title), p1: uiDisplayParameter(block.id) }), () => { draft.move(block.id, delta); this.drawReview(); this.draw(); });
                button.disabled = frozen || index + delta < 0 || index + delta >= value.blocks.length;
                actions.append(button);
            }
            if (raw.page && raw.bbox) {
                const page = evidence.pages.find(p => p.page === raw.page);
                if (page) {
                    const locate = action(displayMessage("recognition.locateOriginalRegion", { p0: uiDisplayParameter(block.id) }), () => { void this.run(() => this.showOriginal(page, raw.bbox)); });
                    locate.disabled = !recognitionActionable(job);
                    actions.append(locate);
                }
            }
            if (evidence.pages.some(p => p.readingOrder?.headerFooterCandidates.includes(block.id)))
                row.append(ui("p", "data-warning", uiMessage("recognition.headerFooterCandidateHelp")));
            row.append(actions);
            this.proposals.append(row);
        });
        value.tables.forEach((table, index) => { const section = ui("fieldset", "recognition-table"); section.append(ui("legend", "", displayMessage("recognition.tableCandidateSummary", { p0: table.page, p1: uiDisplayParameter(table.regionId) })), ui("p", "data-help", uiMessage("recognition.tableCandidateLimitations"))); const grid = ui("table"); table.cells.forEach((cells, row) => { const tr = ui("tr"); cells.forEach((cell, column) => { const td = ui("td"), area = ui("textarea", "data-input"); setUiAttr(area, "aria-label", displayMessage("recognition.tableCellAria", { p0: index + 1, p1: row + 1, p2: column + 1 })); area.value = cell; area.rows = 2; area.disabled = frozen; area.oninput = () => { draft.change(v => { v.tables[index].cells[row][column] = area.value; }); this.draw(); }; td.append(area); tr.append(td); }); grid.append(tr); }); section.append(grid, check(displayMessage("recognition.acceptTable", { p0: index + 1 }), table.accepted, accepted => { void this.run(async () => { draft.acceptTable(index, accepted); this.drawReview(); }); })); this.proposals.append(section); });
        if (job.scheme) {
            const scheme = job.scheme;
            this.proposals.append(ui("h3", "", scheme.scheme.name), ui("p", "data-help", displayMessage("recognition.schemeFieldReviewHelp", { p0: uiDisplayParameter(schemeStatus[scheme.applicability] ?? scheme.applicability), p1: uiDisplayParameter(scheme.reason ? ` · ${scheme.reason}` : "") })));
            if (!this.fields.length)
                this.proposals.append(ui("p", "data-warning", uiMessage("recognition.noCompatibleTargetField")));
            for (const field of value.fields) {
                const original = scheme.fields[field.key], row = ui("fieldset", "recognition-field");
                row.append(ui("legend", "", () => `${uiTextValue(fieldNames[field.key] ?? field.key)} · ${uiTextValue(candidateStatus[original.status] ?? original.status)}`));
                const candidates = select([["", uiMessage("recognition.chooseOriginalCandidate")], ...original.candidates.map((c, i) => [String(i), () => `${uiTextValue(c.raw)} → ${uiTextValue(c.parsed ?? uiMessage("recognition.fieldParseFailed"))}`] as [
                        string,
                        UiText
                    ])], displayMessage("recognition.fieldCandidateAria", { p0: uiDisplayParameter(field.key) }));
                candidates.value = field.candidate >= 0 ? String(field.candidate) : "";
                candidates.disabled = frozen || scheme.applicability !== "supported";
                candidates.onchange = () => {
                    void this.run(async () => {
                        if (candidates.value === "") {
                            draft.change(v => Object.assign(v.fields.find(f => f.key === field.key)!, { candidate: -1, confirmed: false, value: "" }));
                        }
                        else
                            draft.selectField(field.key, Number(candidates.value));
                        this.drawReview();
                    });
                };
                const corrected = input(displayMessage("recognition.fieldReviewValueAria", { p0: uiDisplayParameter(field.key) }), field.value);
                corrected.disabled = frozen || field.candidate < 0 && !field.manualCorrection;
                corrected.oninput = () => { draft.change(v => Object.assign(v.fields.find(f => f.key === field.key)!, { value: corrected.value, confirmed: false })); this.drawReviewConsent(row, field.key); this.draw(); };
                const mapping = select([["", uiMessage("recognition.unmappedReviewOnly")], ...this.fields.map(f => [mappingIdentity(f), displayMessage("recognition.targetFieldChoice", { p0: uiDisplayParameter(f.title || f.key), p1: uiDisplayParameter(f.shape.semantic), p2: uiDisplayParameter(f.typeId) })] as [
                        string,
                        UiText
                    ])], displayMessage("recognition.fieldMappingAria", { p0: uiDisplayParameter(field.key) }));
                mapping.value = this.mappings.get(field.key) ?? "";
                mapping.disabled = frozen;
                mapping.onchange = () => { this.mappings.set(field.key, mapping.value); draft.change(v => { v.fields.find(f => f.key === field.key)!.confirmed = false; }); this.drawReview(); this.draw(); };
                row.append(check(displayMessage("recognition.manualFieldCorrection", { p0: uiDisplayParameter(field.key) }), !!field.manualCorrection, accepted => { draft.change(v => { const next = v.fields.find(f => f.key === field.key)!; next.manualCorrection = accepted; next.confirmed = false; }); this.drawReview(); this.draw(); }), label(uiMessage("recognition.originalCandidatesLabel"), candidates), label(uiMessage("recognition.reviewedValueLabel"), corrected), label(uiMessage("recognition.formalFieldMappingLabel"), mapping));
                if (field.candidate >= 0) {
                    const candidate = original.candidates[field.candidate];
                    row.append(ui("pre", "recognition-raw", joinUiText(candidate.sources.map(s => displayMessage("recognition.candidateSourceSummary", { p0: uiDisplayParameter(s.page ?? "Word"), p1: uiDisplayParameter(s.blockId ?? ""), p2: uiDisplayParameter(s.quote ?? candidate.raw) })), "\n")));
                }
                this.drawReviewConsent(row, field.key);
                this.proposals.append(row);
            }
            if (scheme.structures?.length) {
                const numbering = select([["keep", uiMessage("recognition.numberingKeep")], ["remove", uiMessage("recognition.numberingRemoveConfirmed")], ["rebuild", uiMessage("recognition.numberingRenumberConfirmed")]] as const, uiMessage("recognition.numberingAria"));
                numbering.value = value.numbering;
                numbering.disabled = frozen;
                numbering.onchange = () => { draft.change(v => { v.numbering = numbering.value as "keep"; }); this.draw(); };
                this.proposals.append(label(uiMessage("recognition.numberingLabel"), numbering));
                for (const structure of scheme.structures)
                    this.proposals.append(check(displayMessage("recognition.confirmOfficialStructure", { p0: uiDisplayParameter(structure.id), p1: uiDisplayParameter(structure.raw) }), value.structures.includes(structure.id), accepted => { draft.change(v => { v.structures = accepted ? [...v.structures, structure.id] : v.structures.filter(id => id !== structure.id); }); this.draw(); }));
            }
            for (const issue of scheme.issues)
                this.proposals.append(ui("p", "data-warning", issueLabel(issue.code)));
        }
        for (const loss of evidence.losses ?? [])
            this.proposals.append(ui("p", "data-warning", issueLabel(loss)));
        for (const issue of evidence.issues ?? [])
            this.proposals.append(ui("p", "data-warning", () => `${uiTextValue(evidenceIssueNames[issue.code] ?? issue.code)}${uiTextValue(issue.blockId ? displayMessage("recognition.issueOriginalBlockSuffix", { p0: uiDisplayParameter(issue.blockId) }) : "")}${uiTextValue(issue.elements ? displayMessage("recognition.issueContentKindsSuffix", { p0: uiDisplayParameter(Array.isArray(issue.elements) ? issue.elements.join("、") : String(issue.elements)) }) : "")}`));
        if (evidence.docxBlocks)
            this.proposals.append(ui("p", "data-help", uiMessage("recognition.wordGeometryUnverified")));
        if (evidence.status !== "complete")
            this.proposals.append(check(uiMessage("recognition.acceptPartialScope"), value.acceptPartial, accepted => { draft.change(v => { v.acceptPartial = accepted; }); this.draw(); }));
        this.proposals.append(check(uiMessage("recognition.confirmOriginalAndLosses"), value.acceptLosses, accepted => { draft.change(v => { v.acceptLosses = accepted; }); this.draw(); }));
        if (frozen)
            this.proposals.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("input,textarea,select").forEach(c => { c.disabled = true; });
    }
    private drawReviewConsent(row: HTMLElement, key: string): void {
        const oldConsent = row.querySelector("[data-field-consent]");
        if (oldConsent) { disposeUiTree(oldConsent); oldConsent.remove(); }
        const field = this.review?.value.fields.find(f => f.key === key);
        if (!field)
            return;
        const consent = check(displayMessage("recognition.confirmField", { p0: uiDisplayParameter(key) }), field.confirmed, accepted => { void this.run(async () => { this.review!.confirmField(key, accepted); }); });
        consent.dataset.fieldConsent = key;
        const control = consent.querySelector("input")!;
        control.disabled = !!this.controller.locator || field.candidate < 0 && !field.manualCorrection || !field.value.trim();
        row.append(consent);
    }
    private authorReview(): RecognitionAuthorReview {
        const job = this.controller.job, draft = this.review, purpose = this.optionsSnapshot?.purpose ?? this.purpose.value as RecognitionPurpose;
        if (!job || !recognitionActionable(job))
            throw new UiError(uiMessage("recognition.jobNotFinished"));
        if (purpose === "resource")
            return { purpose, title: this.name.value, blocks: [], fields: [], acceptPartial: true };
        if (!draft || !recognitionReviewReady(draft.evidence, draft.value) || this.optionsSnapshot?.operation === "inspect")
            throw new UiError(uiMessage("recognition.reviewRequired"));
        const review = draft.value;
        const blocks: RecognitionAuthorReview["blocks"] = [];
        const acceptedTables = review.tables.filter(t => t.accepted), emitted = new Set<string>();
        for (const block of review.blocks) {
            for (const table of acceptedTables.filter(t => t.replacedBlocks?.includes(block.id))) {
                const id = `table:${table.page}:${table.regionId}:${table.index}`;
                if (!emitted.has(id)) {
                    blocks.push({ id, text: table.cells.map(r => r.join("\t")).join("\n"), kind: "table", table: table.cells });
                    emitted.add(id);
                }
            }
            if (!block.included)
                continue;
            if (acceptedTables.some(t => t.replacedBlocks?.includes(block.id)))
                throw new UiError(uiMessage("recognition.tableTextSelectedAgain"));
            const structure = job.scheme?.structures?.find(s => review.structures.includes(s.id) && s.sources.some(source => source.blockId === block.id));
            blocks.push({ id: block.id, text: block.text, ...(structure ? { kind: "heading" as const, headingLevel: structure.level ?? 1 } : {}) });
        }
        return { purpose, title: this.name.value, blocks, fields: review.fields.filter(f => f.confirmed).map(f => { const mapping = this.fields.find(field => mappingIdentity(field) === this.mappings.get(f.key)); return { key: f.key, value: f.value, confirmed: true, ...(f.candidate >= 0 ? { candidateIndex: f.candidate } : {}), manualCorrection: !!f.manualCorrection, ...(mapping ? { targetField: mapping.id, targetType: mapping.typeId } : {}) }; }), acceptPartial: review.acceptPartial, numbering: review.numbering, selectedStructureIds: review.structures };
    }
    private async prepare(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const review = this.authorReview(); await this.controller.prepare(this.target(), review); this.acceptedPlan = false; this.drawReview(); this.drawRecovery();
        } finally { endCloseWork(); }
    }
    private drawRecovery(): void {
        replaceUiChildren(this.recovery, );
        const locator = this.controller.locator, receipt = this.controller.receipt;
        if (this.unreadable) {
            this.recovery.append(ui("p", "data-warning", uiMessage("recognition.recoveryUnreadable")));
            return;
        }
        if (!locator)
            return;
        this.recovery.append(ui("h3", "", uiMessage("recognition.originalImportTitle")), ui("code", "", locator.operationId), ui("p", "", this.controller.uncertain ? uiMessage("recognition.unknownInspectOriginalFirst") : statuses[receipt?.status ?? ""] ?? uiMessage("recognition.notInspectedYet")));
        this.recovery.append(ui("p", "data-help", "slot" in locator.target ? displayMessage("recognition.frozenSlotTarget", { p0: uiDisplayParameter(locator.target.slot) }) : displayMessage("recognition.frozenLocalTarget", { p0: uiDisplayParameter(locator.target.parent ?? uiMessage("recognition.currentLibrary")), p1: uiDisplayParameter(locator.target.name ?? uiMessage("recognition.newResource")) })));
        if (receipt?.preview.fields?.length) {
            const section = ui("section");
            section.append(ui("h4", "", uiMessage("recognition.frozenConfirmedFieldsTitle")));
            for (const raw of receipt.preview.fields) {
                if (!raw || typeof raw !== "object")
                    continue;
                const field = raw as {
                    key?: string;
                    value?: string;
                    targetField?: string;
                    targetType?: string;
                    manualCorrection?: boolean;
                };
                section.append(ui("p", "", () => `${uiTextValue(fieldNames[field.key ?? ""] ?? field.key ?? uiMessage("recognition.fieldColumn"))}：${uiTextValue(field.value ?? "")} · ${uiTextValue(field.targetField ? displayMessage("recognition.formalFieldTarget", { p0: uiDisplayParameter(field.targetField), p1: uiDisplayParameter(field.targetType ?? uiMessage("recognition.notDeclared")) }) : uiMessage("recognition.unmappedContentOnly"))}${uiTextValue(field.manualCorrection ? uiMessage("recognition.manualCorrectionRecordedSuffix") : uiMessage("recognition.originalCandidateMatchedSuffix"))}`));
            }
            this.recovery.append(section);
        }
        if (receipt?.preview.coverage !== undefined) {
            const details = ui("details");
            details.append(ui("summary", "", uiMessage("recognition.viewFrozenScope")), ui("pre", "recognition-raw", JSON.stringify(receipt.preview.coverage, null, 2)));
            this.recovery.append(details);
        }
        if (receipt?.preview.summary)
            this.recovery.append(ui("p", "", receipt.preview.summary));
        for (const loss of receipt?.preview.losses ?? [])
            this.recovery.append(ui("p", "data-warning", officeLossUiText(loss)));
        if (receipt?.preview.source) {
            const details = ui("details");
            details.append(ui("summary", "", uiMessage("recognition.viewFrozenNativeSource")), ui("pre", "recognition-raw", receipt.preview.source));
            this.recovery.append(details);
        }
        const lookup = action(uiMessage("recognition.inspectOriginalImport"), () => { void this.run(async () => { await this.controller.lookup(); this.acceptedPlan = false; this.drawRecovery(); }); });
        lookup.disabled = this.controller.busy;
        this.recovery.append(lookup);
        if (receipt?.status === "prepared" && !this.controller.uncertain) {
            this.recovery.append(check(uiMessage("recognition.acceptFrozenPlanLosses"), this.acceptedPlan, accepted => { this.acceptedPlan = accepted; this.drawRecovery(); }));
            const commit = action(uiMessage("recognition.commitSameImportPlan"), () => { void this.run(async () => { await this.controller.commit(this.acceptedPlan); this.drawRecovery(); }); });
            commit.disabled = this.controller.busy || !receipt.commitAllowed || !this.acceptedPlan;
            this.recovery.append(commit);
        }
        if (receipt?.status === "committed") {
            this.recovery.append(ui("p", "data-help", uiMessage("recognition.importCommittedHelp")));
            if (this.host.created)
                this.recovery.append(action(uiMessage("recognition.openAuthorizedNewNote"), () => { void this.run(async () => { await this.host.created?.(receipt, locator.target); this.element.close(); }); }));
        }
        if (receipt && ["committed", "resolved", "cancelled"].includes(receipt.status) && !this.controller.uncertain)
            this.recovery.append(action(uiMessage("recognition.finishAndStartAnother"), () => { void this.run(async () => { this.controller.finish(); this.changed(); }); }));
        else {
            const cancel = action(uiMessage("recognition.cancelUncommittedPlan"), () => { void this.run(async () => { await this.controller.settle(true); this.drawRecovery(); }); });
            cancel.disabled = this.controller.busy;
            this.recovery.append(cancel);
        }
    }
    private async download(provenance = false): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const job = this.controller.job;
            if (!job || !recognitionActionable(job) || this.optionsSnapshot?.purpose !== "text" || this.optionsSnapshot.operation === "inspect")
                throw new UiError(uiMessage("recognition.textDeliveryUnavailable"));
            const epoch = this.epoch, value = await this.host.request<{
                jobId: string;
                fileName: string;
                mediaType: string;
                dataBase64: string;
                sha256: string;
                provenance?: {
                    fileName: string;
                    mediaType: string;
                    dataBase64: string;
                    sha256: string;
                };
            }>("/download", { jobId: job.jobId });
            if (epoch !== this.epoch || this.identity !== this.host.identity())
                return;
            if (value.jobId !== job.jobId)
                throw new UiError(uiMessage("recognition.downloadJobMismatch"));
            const artifact = provenance ? value.provenance : value;
            if (!artifact || artifact.mediaType !== (provenance ? "application/json" : "text/plain") || typeof artifact.fileName !== "string")
                throw new UiError(provenance ? uiMessage("recognition.provenanceUnverifiable") : uiMessage("recognition.extractedTextFormatUnconfirmed"));
            const raw = bytes(artifact.dataBase64);
            if (raw.length > 32 * 1024 * 1024 || await sha(raw) !== artifact.sha256)
                throw new UiError(uiMessage("recognition.deliveryDigestMismatch"));
            if (provenance) {
                const description = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(raw)) as {
                    version?: number;
                    inputSha256?: string;
                    textSha256?: string;
                };
                if (description.version !== 1 || description.inputSha256 !== job.inputSha256 || description.textSha256 !== value.sha256)
                    throw new UiError(uiMessage("recognition.provenanceOriginalMismatch"));
            }
            if (epoch !== this.epoch || this.identity !== this.host.identity())
                return;
            const url = URL.createObjectURL(new Blob([raw], { type: artifact.mediaType }));
            this.downloadUrls.add(url);
            const anchor = ui("a");
            anchor.href = url;
            anchor.download = artifact.fileName.replace(/[/\\]/g, "_");
            anchor.click();
            setTimeout(() => { URL.revokeObjectURL(url); this.downloadUrls.delete(url); }, 1000);
            const identity = this.identity;
            workActivity.result({ id: `${job.jobId}:${artifact.sha256}`, title: artifact.fileName,
                detail: `SHA-256 ${artifact.sha256}`,
                actions: [{ id: "download", label: provenance ? uiMessage("recognition.downloadProvenanceAndLosses") : uiMessage("recognition.downloadExtractedText"), run: async () => {
                    this.showActivityOriginal(job.jobId, "job", identity);
                    await this.download(provenance);
                } }] });
        } finally { endCloseWork(); }
    }
    private draw(): void {
        this.publishActivity();
        const job = this.controller.job, locked = !!this.controller.locator || this.controller.busy || this.reading || this.unreadable;
        for (const section of [this.controls, this.regionEditor])
            for (const control of section.querySelectorAll<HTMLInputElement | HTMLSelectElement | HTMLButtonElement>("input,select,button"))
                control.disabled = locked;
        setUiText(this.runtime, this.capabilities ? displayMessage("recognition.runtimeSummary", { p0: uiDisplayParameter(this.capabilities.executor === "selfHosted" ? uiMessage("recognition.executorHosted") : uiMessage("recognition.executorLocal")), p1: uiDisplayParameter(this.capabilities.available ? uiMessage("recognition.dependenciesAvailable") : uiMessage("recognition.dependenciesUnavailable")) }) : uiMessage("recognition.checkingDependencies"));
        setUiText(this.selection, this.file ? displayMessage("recognition.selectedOriginalSummary", { p0: uiDisplayParameter(this.file.name), p1: this.file.size, p2: uiDisplayParameter(this.pages.value), p3: uiDisplayParameter(this.rect.value ? displayMessage("recognition.selectedRegionSuffix", { p0: uiDisplayParameter(this.rect.value) }) : uiMessage("recognition.wholePagesSuffix")) }) : uiMessage("recognition.pageMemoryScopeHelp"));
        this.startButton.disabled = locked || !this.upload || !this.capabilities;
        this.inspectButton.disabled = this.startButton.disabled || this.format.value === "docx";
        setUiText(this.startButton, this.purpose.value === "resource" ? uiMessage("recognition.keepOriginalSkipRecognition") : job ? uiMessage("recognition.recognizeWithNewJob") : uiMessage("recognition.startJob"));
        this.statusButton.disabled = this.controller.busy || !this.controller.pendingJobId;
        this.cancelButton.disabled = !this.controller.pendingJobId || recognitionTerminal(job);
        this.downloadButton.disabled = !recognitionActionable(job) || this.optionsSnapshot?.purpose !== "text" || this.optionsSnapshot?.operation === "inspect";
        this.downloadEvidenceButton.disabled = this.downloadButton.disabled;
        this.prepareButton.disabled = locked || !recognitionActionable(job) || this.optionsSnapshot?.operation === "inspect" || this.optionsSnapshot?.purpose !== "resource" && (!this.review || !recognitionReviewReady(this.review.evidence, this.review.value));
        const undo = this.proposals.querySelector<HTMLButtonElement>("[data-recognition-undo]"), redo = this.proposals.querySelector<HTMLButtonElement>("[data-recognition-redo]");
        if (undo)
            undo.disabled = !!this.controller.locator || !recognitionActionable(job) || !this.review?.canUndo;
        if (redo)
            redo.disabled = !!this.controller.locator || !recognitionActionable(job) || !this.review?.canRedo;
        this.drawRecovery();
    }
}
