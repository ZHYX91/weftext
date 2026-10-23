import { disposeUiTree, joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./history-panel.css";
import { coreDescriptionUiText } from "./ui-diagnostics";
import { HistoryController, historyOwnerKey, historyRecoveryIssueUiText, historySourceDiff, historyTimeLabel, type HistoryComparison, type HistoryContext, type HistoryRestoreBasis, type HistoryRestoreResult, type HistorySource, } from "./history-state";
import { apiErrorUiText } from "./api-error";
import { ProposalCloseGuard } from "./proposal-close-guard";
import { registerNativeProposalOwner } from "./native-proposal-owners";
export interface HistoryPanelHost {
    getDocument(): HistoryContext | null;
    request<T>(path: string, body?: unknown): Promise<T>;
    /** Settle/persist actual author input before preparing a restore, never during browsing. */
    prepareRestore?(): Promise<void>;
    /** The source owner applies the saved base and durably preserves newer input and Undo. */
    restored(result: HistoryRestoreResult, basis: Readonly<HistoryRestoreBasis>): Promise<void>;
    /** Explicit comparison can retire this page's delivery, never the Core fact. */
    finishDelivery?(operationId: string, basis: Readonly<HistoryRestoreBasis>): Promise<boolean>;
    /** Keep ordinary Save from racing a pending or uncertain restore. */
    restorePending?(basis: Readonly<HistoryRestoreBasis>, pending: boolean): void;
    notify(message: UiText, error?: boolean): void;
}
let nextHistoryId = 0;
function element<K extends keyof HTMLElementTagNameMap>(tag: K, className = "", text?: UiText): HTMLElementTagNameMap[K] {
    const value = document.createElement(tag);
    value.className = className;
    if (text !== undefined)
        setUiText(value, text);
    return value;
}
function button(label: UiText, run: () => void | Promise<void>, primary = false): HTMLButtonElement {
    const value = element("button", `history-button${primary ? " primary" : ""}`, label);
    value.type = "button";
    value.onclick = () => { void Promise.resolve(run()).catch(() => { }); };
    return value;
}
function sourcePane(label: UiText, source: string): HTMLElement {
    const section = element("section", "history-source-pane");
    const heading = element("h3", "", label);
    heading.id = `history-source-heading-${++nextHistoryId}`;
    const text = element("pre", "history-source", source);
    text.tabIndex = 0;
    text.setAttribute("role", "region");
    text.setAttribute("aria-labelledby", heading.id);
    section.append(heading, text);
    return section;
}
const availability: Record<string, UiText> = { available: uiMessage("history.availableSource"), missing: uiMessage("history.missingSource"), unreadable: uiMessage("history.unreadableSource"), invalid: uiMessage("history.invalidRecord") };
class HistoryDialog {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly unregisterClose = registerNativeProposalOwner(this);
    assertCloseReady(): void {
        try { this.closeGuard.assertReady(this.pending || this.controller.review !== null || this.controller.uncertain || this.controller.busy); }
        catch (error) { if (!this.dialog.open) this.dialog.showModal(); throw error; }
    }
    freezeForClose(): boolean { return this.closeGuard.freeze(this.pending || this.controller.review !== null || this.controller.uncertain || this.controller.busy); }
    thawAfterClose(): void { this.closeGuard.thaw(); }
    readonly dialog = element("dialog", "history-panel");
    private readonly controller: HistoryController;
    private readonly listHost = element("ol", "history-versions");
    private readonly pathLabel = element("p", "history-path");
    private readonly range = element("p", "history-help");
    private readonly warnings = element("div", "history-warnings");
    private readonly sourceHost = element("div", "history-content");
    private readonly protectedHost = element("section", "history-protected");
    private readonly status = element("p", "history-status");
    private readonly operationHost = element("div", "history-operation");
    private readonly compareField = element("fieldset", "history-compare-choice");
    private readonly compareButton: HTMLButtonElement;
    private readonly restoreButton: HTMLButtonElement;
    private readonly refreshButton: HTMLButtonElement;
    private readonly savedChoice = element("input");
    private readonly draftChoice = element("input");
    private readBusy = false;
    private readGeneration = 0;
    private chosenVersionId: string | null = null;
    private restoreDialog: HTMLDialogElement | null = null;
    private acknowledging = false;
    private unacknowledged: {
        result: HistoryRestoreResult;
        basis: Readonly<HistoryRestoreBasis>;
    } | null = null;
    get pending(): boolean { return this.controller.operationId !== null || this.unacknowledged !== null; }
    constructor(private readonly host: HistoryPanelHost, context: HistoryContext) {
        this.controller = new HistoryController({
            id: () => crypto.randomUUID(), current: () => host.getDocument(),
            list: request => host.request("/api/history/list", request),
            read: request => host.request("/api/history/read", request),
            compare: request => host.request("/api/history/compare", request),
            restore: request => {
                const basis = this.controller.pendingBasis;
                if (basis)
                    host.restorePending?.(basis, true);
                return host.request("/api/history/restore", request);
            },
            status: request => host.request("/api/history/status", request),
            settle: request => host.request("/api/operations/settle", request),
        }, Object.freeze(structuredClone(context)));
        setUiAttr(this.dialog, "aria-label", uiMessage("history.localTitle"));
        const header = element("header", "history-header");
        header.append(element("h2", "", uiMessage("history.localTitle")), button(uiMessage("history.close"), () => this.dialog.close()));
        setUiText(this.pathLabel, context.path);
        const intro = element("p", "history-help", uiMessage("history.localScopeHelp"));
        this.refreshButton = button(uiMessage("history.refresh"), () => this.refresh());
        const side = element("aside", "history-sidebar");
        setUiAttr(side, "aria-label", uiMessage("history.savedVersionsHeading"));
        side.append(this.refreshButton, this.range, this.listHost);
        this.compareField.append(element("legend", "", uiMessage("history.comparisonBasisLegend")));
        const group = `history-compare-${++nextHistoryId}`;
        for (const [control, value, label] of [[this.savedChoice, "currentSaved", uiMessage("history.currentSavedChoice")], [this.draftChoice, "draftSnapshot", uiMessage("history.currentDraftChoice")]] as const) {
            control.type = "radio";
            control.name = group;
            control.value = value;
            setUiAttr(control, "aria-label", label);
            const item = element("label", "history-radio");
            item.append(control, uiTextNode(label));
            this.compareField.append(item);
        }
        this.savedChoice.checked = !context.dirty;
        this.compareButton = button(uiMessage("history.compareSelected"), () => this.compare());
        this.restoreButton = button(uiMessage("history.restoreSelected"), () => this.prepareRestore(), true);
        const actions = element("div", "history-actions");
        actions.append(this.compareField, this.compareButton, this.restoreButton);
        const main = element("section", "history-main");
        main.append(this.warnings, this.sourceHost, actions, this.status, this.operationHost, this.protectedHost);
        const body = element("div", "history-layout");
        body.append(side, main);
        this.status.setAttribute("role", "status");
        this.status.setAttribute("aria-live", "polite");
        this.dialog.append(header, this.pathLabel, intro, body);
        document.body.append(this.dialog);
        this.closeGuard.bind(this.dialog);
        this.draw();
    }
    async open(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (!this.dialog.open)
                this.dialog.showModal();
            if (!this.controller.list || this.host.getDocument()?.sessionId !== this.controller.note.sessionId)
                await this.refresh();
            else
                this.draw();
        } finally { endCloseWork(); }
    }
    private report(error: unknown, prefix: UiText = ""): void {
        const message = joinUiText([prefix, errorUiText(error)]);
        setUiText(this.status, message);
        this.status.classList.add("history-error");
        this.host.notify(message, true);
    }
    private async read(action: () => Promise<unknown>): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const generation = ++this.readGeneration;
            this.readBusy = true;
            this.status.classList.remove("history-error");
            setUiText(this.status, uiMessage("history.reading"));
            this.drawControls();
            try {
                await action();
                if (generation === this.readGeneration)
                    setUiText(this.status, "");
            }
            catch (error) {
                if (generation === this.readGeneration)
                    this.report(error);
            }
            finally {
                if (generation === this.readGeneration) {
                    this.readBusy = false;
                    this.draw();
                }
            }
        } finally { endCloseWork(); }
    }
    private async refresh(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            await this.read(async () => { await this.controller.refresh(); });
        } finally { endCloseWork(); }
    }
    private async select(versionId: string): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            this.chosenVersionId = versionId;
            await this.read(async () => { await this.controller.select(versionId); });
        } finally { endCloseWork(); }
    }
    private async compare(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            if (!this.savedChoice.checked && !this.draftChoice.checked) {
                this.report(new UiError(uiMessage("history.comparisonChoiceRequired")));
                return;
            }
            if (this.draftChoice.checked) {
                try {
                    this.controller.compareDraft();
                    setUiText(this.status, uiMessage("history.draftSnapshotFixed"));
                    this.status.classList.remove("history-error");
                    this.draw();
                }
                catch (error) {
                    this.report(error);
                }
            }
            else
                await this.read(async () => { await this.controller.compareSaved(); });
        } finally { endCloseWork(); }
    }
    private draw(): void {
        const list = this.controller.list;
        setUiText(this.pathLabel, list?.path ?? this.controller.note.path);
        if (list) {
            const { oldestSavedAtUnixMs, newestSavedAtUnixMs } = list.retention;
            const times = oldestSavedAtUnixMs !== null && newestSavedAtUnixMs !== null ? displayMessage("history.knownTimeRange", { p0: uiDisplayParameter(historyTimeLabel({ savedAtUnixMs: oldestSavedAtUnixMs, timeBasis: "commitClock" })), p1: uiDisplayParameter(historyTimeLabel({ savedAtUnixMs: newestSavedAtUnixMs, timeBasis: "commitClock" })) }) : "";
            setUiText(this.range, displayMessage("history.retentionSummary", { p0: list.retention.knownVersions, p1: list.retention.availableVersions, p2: uiDisplayParameter(times) }));
        }
        else
            setUiText(this.range, uiMessage("history.retentionUnknownHelp"));
        this.warnings.replaceChildren();
        if (list?.currentness === "external")
            this.warnings.append(element("p", "history-warning", uiMessage("history.externalChangesHelp")));
        for (const warning of list?.warnings ?? [])
            this.warnings.append(element("p", "history-warning", coreDescriptionUiText(warning)));
        this.listHost.replaceChildren();
        for (const version of list?.versions ?? []) {
            const row = element("li", "history-version");
            const choice = button(historyTimeLabel(version), () => this.select(version.versionId));
            choice.dataset.historyVersion = version.versionId;
            choice.setAttribute("aria-pressed", String(this.chosenVersionId === version.versionId));
            const sequence = version.commitSequence === null ? uiMessage("history.commitOrderUnknown") : displayMessage("history.commitSequence", { p0: version.commitSequence });
            setUiAttr(choice, "aria-label", () => `${uiTextValue(historyTimeLabel(version))}，${uiTextValue(sequence)}，${uiTextValue(availability[version.status] ?? version.status)}${uiTextValue(version.versionId === list!.currentSavedVersionId ? uiMessage("history.currentSavedAriaSuffix") : "")}`);
            choice.append(element("span", "history-version-meta", () => `${uiTextValue(sequence)} · ${uiTextValue(availability[version.status] ?? version.status)}${uiTextValue(version.versionId === list!.currentSavedVersionId ? uiMessage("history.currentSavedSuffix") : "")}`));
            choice.disabled = this.readBusy;
            row.append(choice);
            if (version.status !== "available")
                row.append(element("p", "history-help", coreDescriptionUiText(version.description)));
            this.listHost.append(row);
        }
        if (list && !list.versions.length)
            this.listHost.append(element("li", "history-help", uiMessage("history.noRetainedRecords")));
        this.sourceHost.replaceChildren();
        if (this.controller.comparison)
            this.drawComparison(this.controller.comparison);
        else if (this.controller.selected)
            this.drawSource(this.controller.selected);
        else if (this.chosenVersionId)
            this.sourceHost.append(element("p", "history-warning", uiMessage("history.selectedUnavailable")));
        else
            this.sourceHost.append(element("p", "history-help", uiMessage("history.selectRecordHelp")));
        this.drawProtected();
        this.drawControls();
        this.drawOperation();
    }
    private drawSource(source: HistorySource): void {
        this.sourceHost.append(element("p", "history-help", displayMessage("history.sourceMetadata", { p0: uiDisplayParameter(historyTimeLabel(source)), p1: uiDisplayParameter(source.byteLength ?? uiMessage("common.unknown")), p2: uiDisplayParameter(source.versionId) })));
        this.sourceHost.append(sourcePane(uiMessage("history.selectedSavedSourceHeading"), source.source));
        this.sourceHost.append(element("p", "history-help", uiMessage("history.sourceOnlyHelp")));
    }
    private drawComparison(comparison: HistoryComparison): void {
        const label = comparison.comparison === "currentSaved" ? uiMessage("history.fixedSavedHeading") : displayMessage("history.fixedDraftHeading", { p0: comparison.current.serial });
        this.sourceHost.append(element("p", "history-help", comparison.equal ? uiMessage("history.equalSources") : uiMessage("history.differentSources")));
        const columns = element("div", "history-comparison");
        const left = sourcePane(uiMessage("history.selectedHistoryHeading"), ""), right = sourcePane(label, "");
        const leftText = left.querySelector("pre")!, rightText = right.querySelector("pre")!;
        for (const part of historySourceDiff(comparison.selected.source, comparison.current.source)) {
            if (part.kind !== "added")
                leftText.append(element("span", part.kind === "removed" ? "history-removed" : "", part.text));
            if (part.kind !== "removed")
                rightText.append(element("span", part.kind === "added" ? "history-added" : "", part.text));
        }
        columns.append(left, right);
        this.sourceHost.append(columns);
        this.sourceHost.append(element("p", "history-help", displayMessage("history.comparisonMetadata", { p0: uiDisplayParameter(comparison.selected.versionId), p1: uiDisplayParameter(comparison.current.baseId), p2: uiDisplayParameter(comparison.comparison === "draftSnapshot" ? uiMessage("history.unsavedSnapshotSuffix") : uiMessage("history.fixedComparisonSuffix")) })));
    }
    private drawProtected(): void {
        this.protectedHost.replaceChildren();
        const drafts = this.controller.list?.protectedDrafts ?? [];
        if (!drafts.length)
            return;
        this.protectedHost.append(element("h3", "", displayMessage("history.protectedDraftCount", { p0: drafts.length })), element("p", "history-help", uiMessage("history.protectedDraftHelp")));
        drafts.forEach((draft, index) => {
            const details = element("details", "history-draft");
            details.append(element("summary", "", displayMessage("history.protectedDraftSummary", { p0: index + 1, p1: uiDisplayParameter(draft.baseKnown ? "" : uiMessage("history.unknownDraftBaseSuffix")) })), element("p", "history-help", draft.path), sourcePane(uiMessage("history.protectedDraftHeading"), draft.source));
            if (draft.baseKnown) {
                const base = element("details");
                base.append(element("summary", "", uiMessage("history.viewOriginalDraftBase")), sourcePane(uiMessage("history.originalSavedBaseHeading"), draft.baseSource));
                details.append(base);
            }
            this.protectedHost.append(details);
        });
    }
    private drawControls(): void {
        const disabled = this.readBusy || this.controller.locked || this.unacknowledged !== null || this.acknowledging;
        this.refreshButton.disabled = this.readBusy || this.controller.busy;
        this.compareButton.disabled = !this.controller.selected || this.readBusy || this.controller.busy;
        this.restoreButton.disabled = !this.controller.selected || disabled || this.host.getDocument()?.readOnly === true;
        this.savedChoice.disabled = this.readBusy || this.controller.busy;
        this.draftChoice.disabled = this.savedChoice.disabled;
    }
    private drawOperation(): void {
        disposeUiTree(this.operationHost);
        this.operationHost.replaceChildren();
        const pending = this.controller.pendingBasis;
        if (this.controller.review && !this.controller.operationId) {
            this.operationHost.append(button(uiMessage("history.cancelRestoreDialog"), () => {
                this.closeGuard.assertMutable();
                this.controller.cancelRestore();
                this.restoreDialog?.close();
                this.drawControls();
                this.drawOperation();
            }));
        }
        if (pending) {
            const state = this.controller.operationState;
            const message = state === "committed" ? uiMessage("history.committedResultUnavailable") : state === "missing" ? uiMessage("history.originalNotRegistered") : state === "recoveryPending" ? uiMessage("history.originalRecoveryPending") : uiMessage("history.originalResultUnknown");
            this.operationHost.append(element("p", "history-warning", displayMessage("history.originalOperationSummary", { p0: uiDisplayParameter(message), p1: uiDisplayParameter(this.controller.operationId) })));
            if (this.controller.originalResultError)
                this.operationHost.append(element("p", "history-warning", apiErrorUiText(this.controller.originalResultError)));
            const inspect = button(uiMessage("history.inspectOriginal"), () => this.inspect());
            const retry = button(uiMessage("history.retryOriginal"), () => this.retry());
            inspect.disabled = this.controller.busy;
            retry.disabled = this.controller.busy || state === "recoveryPending" || state === "committed";
            this.operationHost.append(inspect, retry);
        }
        if (this.unacknowledged) {
            const retry = button(uiMessage("history.retryReceiptDelivery"), () => this.acknowledge());
            retry.disabled = this.acknowledging;
            this.operationHost.append(element("p", "history-warning", uiMessage("history.savedReceiptDeliveryPending")), retry);
        }
        const basis = this.unacknowledged?.basis ?? pending;
        const operationId = this.unacknowledged?.result.receipt.operationId ?? this.controller.operationId;
        if (basis && operationId && this.controller.operationState === "committed" && this.host.finishDelivery) {
            const finish = button(uiMessage("history.finishDelivery"), async () => {
                if (this.acknowledging)
                    return;
                const endCloseWork = this.closeGuard.begin();
                this.acknowledging = true;
                this.dialog.close();
                try {
                    if (await this.host.finishDelivery!(operationId, basis)) {
                        this.controller.finishCommittedDelivery();
                        this.unacknowledged = null;
                        this.host.restorePending?.(basis, false);
                        this.host.notify(uiMessage("history.deliveryFinished"));
                    }
                }
                catch (error) { this.report(error); }
                finally { try { this.acknowledging = false; this.drawControls(); this.drawOperation(); } finally { endCloseWork(); } }
            });
            finish.disabled = this.controller.busy || this.acknowledging;
            this.operationHost.append(finish);
        }
    }
    private async prepareRestore(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            await this.read(async () => {
                await this.host.prepareRestore?.();
                const review = await this.controller.prepareRestore();
                const dialog = element("dialog", "history-restore");
                this.closeGuard.bind(dialog);
                setUiAttr(dialog, "aria-label", uiMessage("history.restoreDialogTitle"));
                const heading = element("h2", "", uiMessage("history.restoreDialogTitle"));
                const description = element("p", "history-help", uiMessage("history.restoreScopeHelp"));
                const metadata = element("p", "history-help", displayMessage("history.restoreBasisSummary", { p0: uiDisplayParameter(review.basis.selectedVersionId), p1: uiDisplayParameter(review.basis.baseId) }));
                const sourceDetails = element("details", "history-draft");
                sourceDetails.append(element("summary", "", uiMessage("history.restoreComparisonDisclosure")), sourcePane(uiMessage("history.restoreSourceHeading"), review.basis.selectedSource), sourcePane(uiMessage("history.currentSavedSourceHeading"), review.comparison.current.source));
                const protect = element("input");
                protect.type = "checkbox";
                setUiAttr(protect, "aria-label", uiMessage("history.protectDraftLabel"));
                const choice = element("label", "history-protection");
                choice.append(protect, uiTextNode(uiMessage("history.protectDraftChoice")));
                const draftInfo = element("p", "history-help", review.basis.dirty ? uiMessage("history.dirtyDraftProtectionRequired") : uiMessage("history.cleanDraftProtectionHelp"));
                const note = element("p", "history-status");
                note.setAttribute("role", "status");
                const confirm = button(uiMessage("history.confirmRestore"), async () => {
                    if (!protect.checked)
                        return;
                    const endCloseWork = this.closeGuard.begin();
                    const basis = review.basis;
                    confirm.disabled = true;
                    cancel.disabled = true;
                    protect.disabled = true;
                    try {
                        const result = await this.controller.restore("retain");
                        this.unacknowledged = { result, basis };
                        await this.acknowledge();
                        dialog.close();
                    }
                    catch (error) {
                        this.report(error);
                        setUiText(note, errorUiText(error));
                        dialog.close();
                    }
                    finally {
                        try {
                            this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null);
                            this.drawControls();
                            this.drawOperation();
                        } finally { endCloseWork(); }
                    }
                }, true);
                confirm.disabled = true;
                protect.onchange = () => { confirm.disabled = !protect.checked; };
                const cancel = button(uiMessage("history.cancelRestoreDialog"), () => {
                    this.closeGuard.assertMutable();
                    this.controller.cancelRestore();
                    dialog.close();
                });
                cancel.autofocus = true;
                const actions = element("div", "history-actions");
                actions.append(cancel, confirm);
                dialog.append(heading, description, metadata, sourceDetails, choice, draftInfo, note, actions);
                dialog.oncancel = event => {
                    if (this.closeGuard.frozen || this.controller.busy)
                        event.preventDefault();
                    else if (!this.controller.operationId)
                        this.controller.cancelRestore();
                };
                dialog.onclose = () => {
                    disposeUiTree(dialog);
                    dialog.remove();
                    this.restoreDialog = null;
                };
                document.body.append(dialog);
                this.restoreDialog = dialog;
                dialog.showModal();
            });
        } finally { endCloseWork(); }
    }
    private async acknowledge(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const pending = this.unacknowledged;
            if (!pending || this.acknowledging)
                return;
            this.acknowledging = true;
            this.drawControls();
            this.drawOperation();
            try {
                await this.host.restored(pending.result, pending.basis);
                this.unacknowledged = null;
                this.host.restorePending?.(pending.basis, false);
                const savedMessage = joinUiText([uiMessage("history.restoreSaved"), ...(pending.result.recoveryIssue ? [uiMessage("history.recoveryIssuePrefix"), historyRecoveryIssueUiText(pending.result.recoveryIssue)] : [])]);
                setUiText(this.status, savedMessage);
                this.status.classList.remove("history-error");
                this.host.notify(savedMessage);
                try {
                    await this.controller.refresh();
                }
                catch (error) {
                    this.report(error, uiMessage("history.savedRefreshFailedPrefix"));
                }
                this.draw();
            }
            catch (error) {
                this.report(error, uiMessage("history.savedDeliveryFailedPrefix"));
                throw error;
            }
            finally {
                this.acknowledging = false;
                this.drawControls();
                this.drawOperation();
            }
        } finally { endCloseWork(); }
    }
    private async inspect(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const basis = this.controller.pendingBasis;
            if (!basis)
                return;
            try {
                const attempt = this.controller.inspectOriginal();
                this.drawControls();
                this.drawOperation();
                const result = await attempt;
                if (result) {
                    this.unacknowledged = { result, basis };
                    await this.acknowledge();
                }
                else
                    setUiText(this.status, this.controller.operationState === "committed" ? uiMessage("history.committedResultPending") : this.controller.operationState === "resolved" ? uiMessage("history.originalResolved") : this.controller.operationId === null ? uiMessage("history.rejectedOriginalEnded") : uiMessage("history.originalReceiptPending"));
            }
            catch (error) {
                this.report(error);
            }
            finally {
                this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null);
                this.drawControls();
                this.drawOperation();
            }
        } finally { endCloseWork(); }
    }
    private async retry(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const basis = this.controller.pendingBasis;
            if (!basis)
                return;
            try {
                const attempt = this.controller.retryOriginal();
                this.drawControls();
                this.drawOperation();
                const result = await attempt;
                this.unacknowledged = { result, basis };
                await this.acknowledge();
            }
            catch (error) {
                this.report(error);
            }
            finally {
                this.host.restorePending?.(basis, this.controller.operationId !== null || this.unacknowledged !== null);
                this.drawControls();
                this.drawOperation();
            }
        } finally { endCloseWork(); }
    }
    dispose(): void { this.assertCloseReady(); this.closeGuard.assertMutable(); this.closeGuard.freeze(false); this.unregisterClose(); this.readGeneration++; if (this.restoreDialog) disposeUiTree(this.restoreDialog); disposeUiTree(this.dialog); this.restoreDialog?.remove(); this.dialog.remove(); }
}
/** Closing/reopening preserves an uncertain operation and its immutable request. */
export class HistoryPanel {
    private readonly closeGuard = new ProposalCloseGuard();
    private readonly dialogs = new Map<string, HistoryDialog>();
    constructor(private readonly host: HistoryPanelHost) { }
    assertCloseReady(): void { this.closeGuard.assertReady(false); for (const dialog of this.dialogs.values()) dialog.assertCloseReady(); }
    freezeForClose(): boolean {
        try { this.assertCloseReady(); } catch { return false; }
        if (!this.closeGuard.freeze(false)) return false;
        for (const dialog of this.dialogs.values()) if (!dialog.freezeForClose()) { this.thawAfterClose(); return false; }
        return true;
    }
    thawAfterClose(): void { this.closeGuard.thaw(); for (const dialog of this.dialogs.values()) dialog.thawAfterClose(); }
    hasPending(context: HistoryContext | null): boolean { return !!context && this.dialogs.get(historyOwnerKey(context))?.pending === true; }
    async open(): Promise<void> {
        const endCloseWork = this.closeGuard.begin();
        try {
            const context = this.host.getDocument();
            if (!context?.nodeId || !context.workspaceId) {
                this.host.notify(uiMessage("history.admittedLocalNoteRequired"), true);
                return;
            }
            const key = historyOwnerKey(context);
            let dialog = this.dialogs.get(key);
            if (!dialog) {
                dialog = new HistoryDialog(this.host, context);
                this.dialogs.set(key, dialog);
            }
            await dialog.open();
        } finally { endCloseWork(); }
    }
    dispose(): void {
        this.assertCloseReady();
        this.closeGuard.assertMutable();
        for (const dialog of this.dialogs.values())
            dialog.dispose();
        this.dialogs.clear();
    }
}
