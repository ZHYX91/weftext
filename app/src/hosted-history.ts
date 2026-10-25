import { disposeUiTree, joinUiText, uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import "./history-panel.css";
import { action, ui } from "./data-panel";
import { HistoryController, historyRecoveryIssueUiText, historySourceDiff, historyTimeLabel, type HistoryContext, type HistoryList, type HistoryComparison, type HistoryRestoreBasis, type HistorySource, type SavedHistoryComparison, type DraftHistoryComparison } from "./history-state";
import { hostedCachePrefix } from "./hosted-state";
import { hostedHistoryPending, hostedHistoryStatus, resumeHostedHistoryRecord, unavailableCommittedHistory, type HostedHistoryContext, type HostedHistoryRecord, type HostedHistoryStatus } from "./hosted-history-state";
import { queryBasisWire, type EmbeddedQueryBasis } from "./embedded-query";
import { apiErrorUiText, decodeApiError } from "./api-error";
type Request = <T>(path: string, body: unknown) => Promise<T>;
export interface HostedHistoryHost {
    context(): HostedHistoryContext | null;
    basis(settle: boolean): Promise<EmbeddedQueryBasis>;
    pendingChanged(): void;
    restored(record: HostedHistoryRecord): Promise<"applied" | "superseded">;
    finishUnavailable(record: HostedHistoryRecord): Promise<void>;
}
const hash = async (value: string): Promise<string> => [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(value)))].map(byte => byte.toString(16).padStart(2, "0")).join("");
const labels: Record<HostedHistoryStatus["status"], UiText> = { notRegistered: uiMessage("hostedHistory.notRegisteredStatus"), notCommitted: uiMessage("hostedHistory.notCommittedStatus"), committed: uiMessage("hostedHistory.committedStatus"), recoveryPending: uiMessage("hostedHistory.recoveryPendingStatus"), resolved: uiMessage("hostedHistory.resolvedStatus"), cancelled: uiMessage("hostedHistory.cancelledStatus") };
function pane(label: UiText, source: string) { const section = ui("section", "history-source-pane"), content = ui("pre", "history-source", source); content.tabIndex = 0; setUiAttr(content, "aria-label", label); section.append(ui("h3", "", label), content); return section; }
/** Original restores survive closing a view; native history remains Core-owned. */
export class HostedHistory {
    private epoch = 0;
    private generation = 0;
    private key = "";
    private records: HostedHistoryRecord[] = [];
    private selectedOperation = "";
    private controller?: HistoryController;
    private note?: {
        resource: string;
        sessionId: string;
        nodeId: string;
        workspaceId: string;
    };
    private busy = false;
    private shown = false;
    private draftComparison = false;
    private unreadable?: string;
    private dialog?: HTMLDialogElement;
    private message = ui("p", "history-status");
    private notice: UiText = "";
    private content = ui("section");
    private operations = ui("section");
    constructor(private host: HTMLElement, private request: Request, private source: HostedHistoryHost) { this.message.setAttribute("role", "status"); this.clear(false); }
    hasPending(resource?: string) { return this.records.some(record => (!resource || record.request.resource === resource) && hostedHistoryPending(record)); }
    clear(removeCache = true) {
        this.epoch++;
        this.generation++;
        this.key = "";
        this.records = [];
        this.selectedOperation = "";
        this.controller = undefined;
        this.note = undefined;
        this.busy = false;
        this.shown = false;
        this.unreadable = undefined;
        if (this.dialog) disposeUiTree(this.dialog);
        this.dialog?.remove();
        this.dialog = undefined;
        disposeUiTree(this.content);
        disposeUiTree(this.operations);
        disposeUiTree(this.host);
        this.content.replaceChildren();
        this.operations.replaceChildren();
        this.setNotice("");
        this.host.replaceChildren();
        this.host.hidden = true;
        if (removeCache)
            for (const key of Object.keys(sessionStorage))
                if (key.startsWith("weftext-hosted-history:"))
                    sessionStorage.removeItem(key);
    }
    async connect(deployment: string, actor: string, generation: number) {
        this.clear(false);
        this.key = hostedCachePrefix("history", deployment, actor, generation) + "restores";
        let raw: string | null = null;
        try {
            raw = sessionStorage.getItem(this.key);
            if (raw) {
                const records = JSON.parse(raw) as HostedHistoryRecord[];
                if (!Array.isArray(records) || records.some(record => !record.request?.operationId || !record.request.resource || !record.basis || record.basis.resource !== record.request.resource || typeof record.request.source !== "string" || typeof record.basis.selectedSource !== "string"))
                    throw new Error();
                this.records = records.map(resumeHostedHistoryRecord);
                this.selectedOperation = this.records[0]?.request.operationId ?? "";
            }
        }
        catch {
            this.unreadable = raw ?? "";
        }
        this.mount();
        this.render();
        const epoch = this.epoch;
        for (const record of [...this.records]) {
            if (record.acknowledged)
                continue;
            try {
                await this.lookup(record);
            }
            catch (error) {
                if (epoch === this.epoch)
                    this.report(error);
            }
            if (epoch !== this.epoch)
                return;
        }
        this.source.pendingChanged();
    }
    private mount() { this.host.replaceChildren(ui("h2", "", uiMessage("hostedHistory.title")), this.message, this.content, this.operations); }
    documentChanged() {
        const current = this.source.context();
        if (this.note && (!current || this.note.sessionId !== current.sessionId || this.note.resource !== current.resource)) {
            this.generation++;
            this.controller = undefined;
            this.note = undefined;
            this.shown = false;
            if (this.dialog) disposeUiTree(this.dialog);
            this.dialog?.remove();
            this.dialog = undefined;
        }
        this.render();
    }
    private context(note = this.note): HistoryContext | null { const current = this.source.context(); return current && note && current.resource === note.resource && current.sessionId === note.sessionId ? { ...current, path: `resource:${current.resource}`, nodeId: note.nodeId, workspaceId: note.workspaceId } : null; }
    async open() {
        await this.perform(async () => {
            const current = this.source.context();
            if (!current)
                throw new UiError(uiMessage("hostedHistory.readPermissionRequired"));
            const epoch = this.epoch, list = await this.request<HistoryList>("history/list", { sessionId: current.sessionId });
            if (epoch !== this.epoch || this.source.context()?.sessionId !== current.sessionId)
                return;
            if (!list.nodeId || !list.workspaceId || list.path !== `resource:${current.resource}`)
                throw new UiError(uiMessage("hostedHistory.identityUnavailable"));
            const note = { resource: current.resource, sessionId: current.sessionId, nodeId: list.nodeId, workspaceId: list.workspaceId };
            this.note = note;
            this.controller = new HistoryController({ current: () => this.context(note), id: () => { throw new UiError(uiMessage("hostedHistory.localIdAllocationRefused")); }, list: body => this.request("history/list", body), read: body => this.request("history/read", body), compare: body => this.request("history/compare", body), restore: async () => { throw new UiError(uiMessage("hostedHistory.localRestoreRefused")); }, status: async () => { throw new UiError(uiMessage("hostedHistory.localStatusRefused")); }, settle: async () => { throw new UiError(uiMessage("hostedHistory.localSettleRefused")); } }, Object.freeze(this.context(note)!));
            this.controller.list = structuredClone(list);
            this.shown = true;
            this.draftComparison = current.dirty;
            this.setNotice(uiMessage("hostedHistory.readOnlyScopeHelp"));
        });
    }
    private persist(records = this.records, beforeEffect = false) {
        if (!this.key || this.unreadable !== undefined)
            throw new UiError(uiMessage("hostedHistory.cacheUnavailable"));
        try {
            sessionStorage.setItem(this.key, JSON.stringify(records));
        }
        catch {
            throw new UiError(beforeEffect ? uiMessage("hostedHistory.persistBeforeSendFailed") : uiMessage("hostedHistory.persistenceUnknown"));
        }
    }
    private setNotice(notice: UiText) { this.notice = notice; setUiText(this.message, notice); }
    private report(error: unknown) { this.setNotice(errorUiText(error)); }
    private async perform(run: () => Promise<void>) {
        if (this.busy)
            return;
        const epoch = this.epoch;
        this.busy = true;
        this.render();
        try {
            await run();
        }
        catch (error) {
            if (epoch === this.epoch)
                this.report(error);
        }
        finally {
            if (epoch === this.epoch) {
                this.busy = false;
                this.source.pendingChanged();
                this.render();
            }
        }
    }
    private render() {
        this.host.hidden = !this.shown && !this.records.length && this.unreadable === undefined;
        disposeUiTree(this.content);
        disposeUiTree(this.operations);
        this.content.replaceChildren();
        this.operations.replaceChildren();
        if (this.unreadable !== undefined) {
            this.content.append(ui("p", "history-warning", uiMessage("hostedHistory.unreadableCache")), pane(uiMessage("hostedHistory.originalCacheHeading"), this.unreadable));
            return;
        }
        const controller = this.controller, list = controller?.list;
        if (this.shown && list) {
            this.content.append(ui("p", "history-help", () => `${list.path} · ${uiTextValue(coreDescriptionUiText(list.retention.description))}`));
            const refresh = action(uiMessage("history.refresh"), () => this.perform(async () => { await controller!.refresh(); }));
            refresh.disabled = this.busy;
            this.content.append(refresh);
            const versions = ui("ol", "history-versions");
            for (const version of list.versions) {
                const item = ui("li", "history-version"), choose = action(() => `${uiTextValue(historyTimeLabel(version))} · ${uiTextValue(version.versionId)}`, () => this.perform(async () => { await controller!.select(version.versionId); }));
                choose.disabled = this.busy || version.status !== "available";
                choose.dataset.versionId = version.versionId;
                choose.setAttribute("aria-pressed", String(controller!.selected?.versionId === version.versionId));
                item.append(choose, ui("span", "history-version-meta", () => `${uiTextValue(version.status)} · ${uiTextValue(version.kind)}${uiTextValue(version.commitSequence !== null ? displayMessage("hostedHistory.commitSequenceSuffix", { p0: version.commitSequence }) : "")}`));
                versions.append(item);
            }
            this.content.append(versions);
            for (const warning of list.warnings)
                this.content.append(ui("p", "history-warning", coreDescriptionUiText(warning)));
            if (controller!.selected) {
                this.drawSource(controller!.selected, controller!.comparison);
                const select = ui("select");
                setUiAttr(select, "aria-label", uiMessage("hostedHistory.comparisonLabel"));
                for (const [value, label] of [["saved", uiMessage("history.currentSavedChoice")], ["draft", uiMessage("history.currentDraftChoice")]] as const)
                    select.append(Object.assign(ui("option", "", label), { value }));
                select.value = this.draftComparison ? "draft" : "saved";
                select.onchange = () => this.draftComparison = select.value === "draft";
                const compare = action(uiMessage("hostedHistory.compareSelected"), () => this.compare()), restore = action(uiMessage("hostedHistory.restoreSelected"), () => this.prepare());
                compare.disabled = this.busy;
                const context = this.source.context();
                restore.disabled = this.busy || !context || context.readOnly || context.savePending || context.unresolvedSave || this.hasPending(context.resource);
                this.content.append(select, compare, restore);
            }
            for (const [index, draft] of list.protectedDrafts.entries()) {
                const details = ui("details", "history-draft");
                details.append(ui("summary", "", displayMessage("hostedHistory.protectedDraftSummary", { p0: index + 1, p1: uiDisplayParameter(draft.baseKnown ? "" : uiMessage("history.unknownDraftBaseSuffix")) })), pane(uiMessage("hostedHistory.protectedDraftHeading"), draft.source), pane(uiMessage("hostedHistory.originalDraftBaseHeading"), draft.baseSource));
                this.content.append(details);
            }
        }
        if (this.records.length)
            this.drawOperations();
    }
    private drawSource(selected: HistorySource, comparison: HistoryComparison | null) {
        if (!comparison) {
            this.content.append(pane(uiMessage("hostedHistory.selectedSourceHeading"), selected.source), ui("p", "history-help", coreDescriptionUiText(selected.previewDescription)));
            return;
        }
        const columns = ui("div", "history-comparison"), left = pane(uiMessage("hostedHistory.selectedHistoryHeading"), ""), right = pane(comparison.comparison === "draftSnapshot" ? displayMessage("hostedHistory.fixedDraftHeading", { p0: comparison.current.serial }) : uiMessage("hostedHistory.fixedSavedHeading"), "");
        for (const part of historySourceDiff(comparison.selected.source, comparison.current.source)) {
            if (part.kind !== "added")
                left.querySelector("pre")!.append(ui("span", part.kind === "removed" ? "history-removed" : "", part.text));
            if (part.kind !== "removed")
                right.querySelector("pre")!.append(ui("span", part.kind === "added" ? "history-added" : "", part.text));
        }
        columns.append(left, right);
        this.content.append(columns, ui("p", "history-help", displayMessage("hostedHistory.comparisonMetadata", { p0: uiDisplayParameter(comparison.selected.versionId), p1: uiDisplayParameter(comparison.current.baseId) })));
    }
    private async compare() {
        await this.perform(async () => {
            if (!this.controller?.selected)
                return;
            if (!this.draftComparison) {
                await this.controller.compareSaved();
                return;
            }
            const epoch = this.epoch, controller = this.controller, selected = controller.selected!, basis = await this.source.basis(false), sourceHash = await hash(basis.source);
            const result = await this.request<DraftHistoryComparison & {
                current: DraftHistoryComparison["current"] & {
                    sourceHash: string;
                };
            }>("history/compare-draft", { ...queryBasisWire(basis), sourceHash, versionId: selected.versionId });
            if (epoch !== this.epoch || controller !== this.controller || controller.selected?.versionId !== selected.versionId)
                return;
            if (result.comparison !== "draftSnapshot" || result.selected.versionId !== selected.versionId || result.selected.source !== selected.source || result.current.source !== basis.source || result.current.serial !== basis.serial || result.current.sourceHash !== sourceHash || result.current.path !== `resource:${this.note!.resource}`)
                throw new UiError(uiMessage("hostedHistory.comparisonMismatch"));
            controller.comparison = structuredClone(result);
        });
    }
    private async prepare() {
        await this.perform(async () => {
            if (!this.controller || !this.note || this.hasPending(this.note.resource))
                throw new UiError(uiMessage("hostedHistory.originalPending"));
            await this.source.basis(true);
            const epoch = this.epoch, controller = this.controller, review = await controller.prepareRestore();
            if (epoch !== this.epoch || controller !== this.controller)
                return;
            const dialog = ui("dialog", "history-restore");
            setUiAttr(dialog, "aria-label", uiMessage("hostedHistory.restoreDialogLabel"));
            this.dialog = dialog;
            const protect = ui("input");
            protect.type = "checkbox";
            setUiAttr(protect, "aria-label", uiMessage("hostedHistory.protectDraftLabel"));
            const label = ui("label", "history-protection", uiMessage("hostedHistory.protectDraftChoice"));
            label.prepend(protect);
            const confirm = action(uiMessage("hostedHistory.confirmRestore"), () => {
                void this.perform(async () => {
                    if (!protect.checked)
                        return;
                    const current = this.context();
                    if (!current || current.sessionId !== review.basis.sessionId || current.baseId !== review.basis.baseId || current.source !== review.basis.source || current.serial !== review.basis.serial || current.inputRevision !== review.basis.inputRevision || current.readOnly || current.savePending || current.unresolvedSave)
                        throw new UiError(uiMessage("hostedHistory.restoreBasisChanged"));
                    const sourceHash = await hash(review.basis.source);
                    if (epoch !== this.epoch || this.context()?.serial !== review.basis.serial || this.context()?.source !== review.basis.source)
                        return;
                    const resource = this.note!.resource, operationId = crypto.randomUUID(), record: HostedHistoryRecord = { basis: { ...structuredClone(review.basis), resource }, request: { resource, sessionId: review.basis.sessionId, baseId: review.basis.baseId, versionId: review.basis.selectedVersionId, operationId, draftAction: "retain", inputRevision: review.basis.inputRevision, source: review.basis.source, serial: review.basis.serial, sourceHash } };
                    const records = [...this.records, record];
                    this.persist(records, true);
                    this.records = records;
                    this.selectedOperation = operationId;
                    this.source.pendingChanged();
                    dialog.close();
                    this.dialog = undefined;
                    await this.send(record);
                });
            });
            confirm.disabled = true;
            protect.onchange = () => confirm.disabled = !protect.checked;
            const cancel = action(uiMessage("hostedHistory.cancelRestoreDialog"), () => { controller.cancelRestore(); dialog.close(); });
            dialog.append(ui("h2", "", uiMessage("history.restoreDialogTitle")), ui("p", "history-help", uiMessage("hostedHistory.restoreScopeHelp")), pane(uiMessage("history.restoreSourceHeading"), review.basis.selectedSource), pane(uiMessage("hostedHistory.currentSavedBaseHeading"), review.basis.baseSource), label, cancel, confirm);
            dialog.onclose = () => {
                disposeUiTree(dialog);
                dialog.remove();
                if (this.dialog === dialog)
                    this.dialog = undefined;
            };
            document.body.append(dialog);
            dialog.showModal();
        });
    }
    private drawOperations() {
        this.operations.append(ui("h3", "", uiMessage("hostedHistory.originalOperationHeading")));
        const select = ui("select");
        setUiAttr(select, "aria-label", uiMessage("hostedHistory.originalOperationHeading"));
        for (const record of this.records)
            select.append(Object.assign(ui("option", "", `${record.request.resource} · ${record.request.operationId}`), { value: record.request.operationId }));
        select.value = this.selectedOperation;
        select.onchange = () => { this.selectedOperation = select.value; this.render(); };
        this.operations.append(select);
        const clear = action(uiMessage("hostedHistory.clearCheckedRecords"), () => this.perform(async () => {
            const retained = this.records.filter(record => !record.acknowledged && !["cancelled", "resolved"].includes(record.status?.status ?? ""));
            this.persist(retained);
            this.records = retained;
            if (!retained.some(record => record.request.operationId === this.selectedOperation))
                this.selectedOperation = retained[0]?.request.operationId ?? "";
            this.setNotice(uiMessage("hostedHistory.terminalCacheCleared"));
        }));
        clear.disabled = this.busy || !this.records.some(record => record.acknowledged || ["cancelled", "resolved"].includes(record.status?.status ?? ""));
        this.operations.append(clear);
        const record = this.records.find(item => item.request.operationId === this.selectedOperation);
        if (!record)
            return;
        const originalStatus = record.status;
        this.operations.append(ui("p", "history-help", originalStatus ? () => `${uiTextValue(labels[originalStatus.status])}${uiTextValue(originalStatus.resultBasis && originalStatus.resultBasis !== "current" ? uiMessage("hostedHistory.supersededSaveSuffix") : "")}` : uiMessage("hostedHistory.resultUnchecked")));
        const check = action(uiMessage("hostedHistory.inspectOriginal"), () => this.perform(() => this.lookup(record))), retry = action(uiMessage("hostedHistory.retryOriginal"), () => this.perform(() => this.send(record))), settle = action(uiMessage("hostedHistory.settleOriginal"), () => this.perform(async () => {
            const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/settle", { resource: record.request.resource, operationId: record.request.operationId });
            if (epoch !== this.epoch)
                return;
            this.installStatus(record, incoming);
        })), apply = action(uiMessage("hostedHistory.applyOriginalReceipt"), () => this.perform(() => this.acknowledge(record)));
        for (const button of [check, retry, settle, apply])
            button.disabled = this.busy;
        retry.disabled ||= !!record.status && !["notRegistered", "notCommitted"].includes(record.status.status);
        settle.disabled ||= !!record.status && !["notRegistered", "notCommitted"].includes(record.status.status);
        apply.disabled ||= record.acknowledged === true || record.status?.status !== "committed" || !record.status.result || this.source.context()?.resource !== record.request.resource;
        this.operations.append(check, retry, settle, apply);
        if (unavailableCommittedHistory(record)) {
            const finish = action(uiMessage("hostedHistory.finishUnavailableDelivery"), () => this.perform(async () => {
                const epoch = this.epoch;
                if (!unavailableCommittedHistory(record))
                    throw new UiError(uiMessage("hostedHistory.finishDeliveryUnavailable"));
                await this.source.finishUnavailable(record);
                if (epoch !== this.epoch)
                    return;
                record.acknowledged = true;
                this.persist();
                this.source.pendingChanged();
                this.setNotice(uiMessage("hostedHistory.unavailableDeliveryFinished"));
            }));
            finish.disabled = this.busy || this.source.context()?.resource !== record.request.resource;
            this.operations.append(finish);
        }
        if (record.status?.status === "committed" && !record.status.result)
            this.operations.append(ui("p", "history-warning", uiMessage("hostedHistory.committedResultUnavailable")));
        if (record.status?.result) {
            const details = ui("details");
            details.append(ui("summary", "", uiMessage("hostedHistory.originalResultDisclosure")), pane(uiMessage("hostedHistory.originalSavedSourceHeading"), record.status.result.source));
            for (const draft of record.status.result.protectedDrafts)
                details.append(pane(uiMessage("hostedHistory.originalProtectedDraftHeading"), draft.source), pane(uiMessage("hostedHistory.protectedDraftBaseHeading"), draft.baseSource));
            this.operations.append(details);
        }
    }
    installStatus(record: HostedHistoryRecord, incoming: HostedHistoryStatus): HostedHistoryStatus {
        record.status = hostedHistoryStatus(record, incoming);
        this.persist();
        this.source.pendingChanged();
        this.render();
        if (record.status.guardReason)
            throw new UiError(joinUiText([record.status.guardReason, ...(record.status.resultError ? [" ", apiErrorUiText(record.status.resultError)] : [])]));
        if (record.status.resultError)
            throw decodeApiError(record.status.resultError) ?? new UiError(uiMessage("error.invalidResponse"));
        return record.status;
    }
    private async lookup(record: HostedHistoryRecord) {
        const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/status", { resource: record.request.resource, operationId: record.request.operationId });
        if (epoch !== this.epoch)
            return;
        this.installStatus(record, incoming);
    }
    private async send(record: HostedHistoryRecord) {
        this.persist(undefined, true);
        const epoch = this.epoch, incoming = await this.request<HostedHistoryStatus>("history/restore", record.request);
        if (epoch !== this.epoch)
            return;
        const status = this.installStatus(record, incoming), warning = status.result?.recoveryIssue;
        try {
            if (status.status === "committed" && status.result && this.source.context()?.resource === record.request.resource)
                await this.acknowledge(record);
        }
        finally {
            if (epoch === this.epoch && warning)
                this.setNotice(joinUiText([this.notice, uiMessage("hostedHistory.recoveryIssuePrefix"), historyRecoveryIssueUiText(warning)]));
        }
    }
    private async acknowledge(record: HostedHistoryRecord) {
        const epoch = this.epoch;
        if (record.acknowledged || record.status?.status !== "committed" || !record.status.result)
            return;
        const outcome = await this.source.restored(record);
        if (epoch !== this.epoch)
            return;
        record.acknowledged = true;
        this.persist();
        this.source.pendingChanged();
        this.setNotice(outcome === "applied" ? uiMessage("hostedHistory.restoredApplied") : uiMessage("hostedHistory.restoredSuperseded"));
    }
}
import { coreDescriptionUiText } from "./ui-diagnostics";
