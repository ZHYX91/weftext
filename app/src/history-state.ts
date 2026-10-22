import { locale as interfaceLocale, type UiLocale } from "./ui-locale";
import { message as uiMessage, displayMessage, joinUiText, UiError, type UiText } from "./ui-locale";
/** Transient history presentation only. Core owns versions, retention and Draft persistence. */
import type { DocumentBasis } from "./data-panel";
import type { Receipt } from "./types";
import { coreDescriptionUiText, decodeCoreError, type CoreDescription, type CoreError } from "./ui-diagnostics";
import { freezeOriginalAttempt, originalFence, type FrozenOriginalAttempt, type OriginalFence } from "./original-operation";
import { apiErrorUiText, decodeApiErrorBody, type ApiErrorBody } from "./api-error";
export interface HistoryContext extends DocumentBasis {
    /** One live editor buffer survives Core Session rebinding. */
    ownerKey?: string;
    baseSource: string;
    dirty: boolean;
    readOnly?: boolean;
    savePending?: boolean;
    unresolvedSave?: boolean;
}
export interface ProtectedHistoryDraft {
    path: string;
    baseSource: string;
    source: string;
    baseKnown: boolean;
}
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
    description: CoreDescription;
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
        description: CoreDescription;
    };
    protectedDrafts: ProtectedHistoryDraft[];
    warnings: CoreDescription[];
}
export interface HistorySource extends HistoryVersion {
    source: string;
    readOnly: true;
    previewStatus: "sourceOnly";
    previewDescription: CoreDescription;
}
export interface SavedHistoryComparison {
    selected: HistorySource;
    current: {
        versionId: string | null;
        revisionId: string | null;
        source: string;
        baseId: string;
        path: string;
    };
    equal: boolean;
    comparison: "currentSaved";
}
export interface DraftHistoryComparison {
    selected: HistorySource;
    current: {
        source: string;
        serial: number;
        baseId: string;
        path: string;
    };
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
    recoveryIssue?: CoreError;
}
export interface HistoryOperationStatus {
    operationId: string;
    status: "missing" | "committed" | "recoveryPending" | "resolved";
    receipt?: Receipt;
    result?: HistoryRestoreResult;
    resultError?: ApiErrorBody;
}
/** The generic settlement route returns a flat original fact, never a history result. */
export interface HistorySettlement extends Partial<Receipt> {
    operationId: string;
    status: string;
}
export interface HistoryRestoreBasis extends HistoryContext {
    selectedVersionId: string;
    selectedSource: string;
}
export interface HistoryRestoreReview {
    basis: Readonly<HistoryRestoreBasis>;
    comparison: SavedHistoryComparison;
}
export interface HistoryTransport {
    id(): string;
    current(): HistoryContext | null;
    list(request: {
        sessionId: string;
    }): Promise<HistoryList>;
    read(request: {
        sessionId: string;
        versionId: string;
    }): Promise<HistorySource>;
    compare(request: {
        sessionId: string;
        versionId: string;
    }): Promise<SavedHistoryComparison>;
    restore(request: HistoryRestoreRequest): Promise<HistoryOperationStatus>;
    status(request: {
        operationId: string;
    }): Promise<HistoryOperationStatus>;
    settle(request: OriginalFence): Promise<HistorySettlement>;
}
const clone = <T>(value: T): T => structuredClone(value);
const fail = (code: string, message: UiText): Error => Object.assign(new UiError(message), { code });
export function historyRecoveryIssueUiText(value: CoreError): UiText {
    const issue = decodeCoreError(value);
    if (!issue)
        throw new UiError(uiMessage("error.invalidResponse"));
    const { code, currentSource, ...description } = issue;
    return joinUiText([coreDescriptionUiText(description), ...(currentSource === undefined ? [] : ["\n[currentSource]\n", currentSource])]);
}
const codeOf = (error: unknown): string => typeof error === "object" && error !== null && "code" in error && typeof error.code === "string" ? error.code : "";
function validRestoreReceipt(value: unknown, operationId: string): value is Receipt {
    if (!value || typeof value !== "object")
        return false;
    const receipt = value as Partial<Receipt>;
    return receipt.operationId === operationId && ["saved", "committed"].includes(receipt.status ?? "") && typeof receipt.baseId === "string" && receipt.baseId.length > 0 && typeof receipt.path === "string" && receipt.path.length > 0;
}
export function uncertainHistoryOperation(error: unknown): boolean {
    const code = codeOf(error);
    return !code || ["unknown", "resultUnknown", "recoveryPending", "pending", "in-progress", "pending_reconciliation"].includes(code);
}
export function sameHistoryNote(a: HistoryContext, b: HistoryContext | null): boolean {
    return !!b && a.nodeId === b.nodeId && a.workspaceId === b.workspaceId && (a.ownerKey !== undefined || b.ownerKey !== undefined ? !!a.ownerKey && a.ownerKey === b.ownerKey : a.sessionId === b.sessionId);
}
export function historyOwnerKey(context: HistoryContext): string { return JSON.stringify([context.ownerKey ?? context.sessionId, context.workspaceId, context.nodeId]); }
function sameHistorySession(a: HistoryContext, b: HistoryContext | null): boolean { return sameHistoryNote(a, b) && a.sessionId === b!.sessionId; }
export function sameHistoryDraft(a: HistoryContext, b: HistoryContext | null): boolean {
    return sameHistorySession(a, b) && a.path === b!.path && a.baseId === b!.baseId && a.serial === b!.serial && a.source === b!.source;
}
/** The source owner uses this before applying a restore receipt; newer input always survives. */
export function restoreBufferDisposition(basis: HistoryRestoreBasis, current: HistoryContext | null): "replaceClean" | "retainDraft" | "unrelated" {
    if (!sameHistoryNote(basis, current))
        return "unrelated";
    return basis.dirty || basis.source !== basis.baseSource || !sameHistoryDraft(basis, current) ? "retainDraft" : "replaceClean";
}
export function historyTimeLabel(version: Pick<HistoryVersion, "savedAtUnixMs" | "timeBasis">, language?: UiLocale): UiText {
    if (version.timeBasis !== "commitClock" || version.savedAtUnixMs === null || !Number.isFinite(version.savedAtUnixMs))
        return uiMessage("historyState.timeUnknown");
    const value = new Date(version.savedAtUnixMs);
    return Number.isNaN(value.getTime()) ? uiMessage("historyState.timeUnknown") : () => value.toLocaleString(language ?? interfaceLocale.current, { hour12: false });
}
/** A bounded exact-text diff. The two lanes always reconstruct their original input. */
export interface HistoryDiffPart {
    kind: "same" | "removed" | "added";
    text: string;
}
export function historySourceDiff(before: string, after: string): HistoryDiffPart[] {
    if (before === after)
        return [{ kind: "same", text: before }];
    const lines = (source: string): string[] => source.match(/[^\r\n]*(?:\r\n|\r|\n)|[^\r\n]+$/g) ?? [];
    const a = lines(before), b = lines(after);
    let first = 0, lastA = a.length, lastB = b.length;
    while (first < lastA && first < lastB && a[first] === b[first])
        first++;
    while (lastA > first && lastB > first && a[lastA - 1] === b[lastB - 1]) {
        lastA--;
        lastB--;
    }
    const result: HistoryDiffPart[] = [];
    const append = (kind: HistoryDiffPart["kind"], text: string): void => {
        if (!text)
            return;
        if (result.at(-1)?.kind === kind)
            result[result.length - 1].text += text;
        else
            result.push({ kind, text });
    };
    append("same", a.slice(0, first).join(""));
    const left = a.slice(first, lastA), right = b.slice(first, lastB);
    if ((left.length + 1) * (right.length + 1) > 250000) {
        append("removed", left.join(""));
        append("added", right.join(""));
    }
    else {
        const width = right.length + 1;
        const table = new Uint32Array((left.length + 1) * width);
        for (let i = left.length - 1; i >= 0; i--)
            for (let j = right.length - 1; j >= 0; j--) {
                table[i * width + j] = left[i] === right[j] ? table[(i + 1) * width + j + 1] + 1 : Math.max(table[(i + 1) * width + j], table[i * width + j + 1]);
            }
        let i = 0, j = 0;
        while (i < left.length || j < right.length) {
            if (i < left.length && j < right.length && left[i] === right[j]) {
                append("same", left[i++]);
                j++;
            }
            else if (i < left.length && (j === right.length || table[(i + 1) * width + j] >= table[i * width + j + 1]))
                append("removed", left[i++]);
            else
                append("added", right[j++]);
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
    private operation: HistoryOperationStatus | null = null;
    private generation = 0;
    private pending: {
        request: Readonly<HistoryRestoreRequest>;
        basis: Readonly<HistoryRestoreBasis>;
        attempt: FrozenOriginalAttempt;
    } | null = null;
    constructor(readonly host: HistoryTransport, readonly note: Readonly<HistoryContext>) { }
    get operationId(): string | null { return this.pending?.request.operationId ?? null; }
    get pendingBasis(): Readonly<HistoryRestoreBasis> | null { return this.pending?.basis ?? null; }
    get operationState(): HistoryOperationStatus["status"] | null { return this.operation?.status ?? null; }
    get originalReceipt(): Readonly<Receipt> | null { return this.operation?.receipt ?? null; }
    get originalResultError(): ApiErrorBody | undefined { return this.operation?.resultError; }
    get locked(): boolean { return this.busy || this.pending !== null; }
    finishCommittedDelivery(): void {
        if (this.operation?.status !== "committed")
            throw fail("operationPending", "只有原提交已确定后才可结束本页交付。");
        this.pending = null;
        this.review = null;
        this.uncertain = false;
    }
    private context(): HistoryContext {
        const current = this.host.current();
        if (!sameHistoryNote(this.note, current))
            throw fail("noteChanged", uiMessage("historyState.returnOriginalNote"));
        return clone(current!);
    }
    private writable(context: HistoryContext): void {
        if (context.readOnly)
            throw fail("readOnly", uiMessage("historyState.restoreWriteNotPermitted"));
        if (context.savePending || context.unresolvedSave)
            throw fail("savePending", uiMessage("historyState.resolveOriginalSaveFirst"));
    }
    async refresh(): Promise<boolean> {
        const current = this.context(), generation = ++this.generation;
        const result = await this.host.list({ sessionId: current.sessionId });
        if (generation !== this.generation || !sameHistorySession(current, this.host.current()))
            return false;
        if (result.nodeId !== current.nodeId || result.workspaceId !== current.workspaceId)
            throw fail("identityPending", uiMessage("historyState.noteIdentityMismatch"));
        this.list = clone(result);
        return true;
    }
    async select(versionId: string): Promise<boolean> {
        const current = this.context(), generation = ++this.generation;
        this.selected = null;
        this.comparison = null;
        const row = this.list?.versions.find(version => version.versionId === versionId);
        if (row && row.status !== "available")
            throw fail("historyUnavailable", coreDescriptionUiText(row.description));
        const result = await this.host.read({ sessionId: current.sessionId, versionId });
        if (generation !== this.generation || !sameHistorySession(current, this.host.current()))
            return false;
        this.validateRead(result, versionId);
        this.selected = clone(result);
        return true;
    }
    private validateRead(value: HistorySource, versionId: string): void {
        if (value.versionId !== versionId || value.status !== "available" || value.readOnly !== true || typeof value.source !== "string")
            throw fail("historyUnavailable", uiMessage("historyState.exactReadOnlySourceMissing"));
    }
    async compareSaved(): Promise<boolean> {
        const current = this.context(), selected = this.selected;
        if (!selected)
            throw fail("selectVersion", uiMessage("historyState.selectSavedVersion"));
        const generation = ++this.generation;
        const result = await this.host.compare({ sessionId: current.sessionId, versionId: selected.versionId });
        if (generation !== this.generation || !sameHistorySession(current, this.host.current()))
            return false;
        this.validateRead(result.selected, selected.versionId);
        if (result.comparison !== "currentSaved" || typeof result.current.source !== "string" || !result.current.baseId)
            throw fail("historyUnavailable", uiMessage("historyState.savedComparisonBasisUnavailable"));
        this.selected = clone(result.selected);
        this.comparison = clone(result);
        return true;
    }
    compareDraft(): void {
        const current = this.context();
        if (!this.selected)
            throw fail("selectVersion", uiMessage("historyState.selectSavedVersion"));
        this.generation++;
        this.comparison = {
            selected: clone(this.selected), current: { source: current.source, serial: current.serial, baseId: current.baseId, path: current.path },
            equal: this.selected.source === current.source, comparison: "draftSnapshot",
        };
    }
    async prepareRestore(): Promise<HistoryRestoreReview> {
        if (this.locked)
            throw fail("operationPending", uiMessage("historyState.resolveOriginalRestoreFirst"));
        const context = this.context();
        this.writable(context);
        if (!this.selected)
            throw fail("selectVersion", uiMessage("historyState.selectSavedVersion"));
        const selectedId = this.selected.versionId, generation = ++this.generation;
        const comparison = await this.host.compare({ sessionId: context.sessionId, versionId: selectedId });
        this.validateRead(comparison.selected, selectedId);
        if (generation !== this.generation || this.selected?.versionId !== selectedId || !sameHistoryDraft(context, this.host.current()))
            throw fail("draftChanged", uiMessage("historyState.preparationBasisChanged"));
        if (comparison.comparison !== "currentSaved" || comparison.current.baseId !== context.baseId || comparison.current.source !== context.baseSource || comparison.current.path !== context.path)
            throw fail("staleBase", uiMessage("historyState.diskAndEditingBasisDiffer"));
        this.review = { basis: Object.freeze({ ...context, selectedVersionId: selectedId, selectedSource: comparison.selected.source }), comparison: clone(comparison) };
        return this.review;
    }
    cancelRestore(): void {
        if (this.pending)
            throw fail("operationPending", uiMessage("historyState.restoreAlreadySent"));
        this.review = null;
    }
    async restore(draftAction: HistoryRestoreRequest["draftAction"]): Promise<HistoryRestoreResult> {
        if (this.pending)
            throw fail("operationPending", uiMessage("historyState.continueSameRestore"));
        if (!this.review)
            throw fail("reviewRequired", uiMessage("historyState.viewRestorePreview"));
        const current = this.context(), basis = this.review.basis;
        this.writable(current);
        if (!sameHistoryDraft(basis, current))
            throw fail("draftChanged", uiMessage("historyState.confirmationBasisChanged"));
        if ((basis.dirty || basis.source !== basis.baseSource) && draftAction !== "retain")
            throw fail("draftProtectionRequired", uiMessage("historyState.explicitlyKeepUnsavedDraft"));
        const request = Object.freeze({ sessionId: basis.sessionId, baseId: basis.baseId, versionId: basis.selectedVersionId, operationId: this.host.id(), draftAction, ...(draftAction === "retain" ? { draftSource: basis.source } : {}) });
        this.pending = { request, basis, attempt: freezeOriginalAttempt(request, "save") };
        this.operation = null;
        return this.execute();
    }
    async retryOriginal(): Promise<HistoryRestoreResult> {
        if (!this.pending || this.busy)
            throw fail("operationPending", uiMessage("historyState.noRetryableRestore"));
        if (this.operation?.status === "committed") {
            const result = await this.inspectOriginal();
            if (result)
                return result;
            throw fail("resultUnavailable", uiMessage("historyState.committedResultUndeliverable"));
        }
        return this.execute();
    }
    private validateRestore(result: HistoryRestoreResult): void {
        const pending = this.pending!;
        if (!result || !validRestoreReceipt(result.receipt, pending.request.operationId) || result.source !== pending.basis.selectedSource || result.versionId !== pending.basis.selectedVersionId || !Array.isArray(result.protectedDrafts) || result.protectedDrafts.some(draft => !draft || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.path !== "string" || typeof draft.baseKnown !== "boolean"))
            throw fail("resultUnknown", uiMessage("historyState.restoreReceiptIncomplete"));
        if (result.recoveryIssue !== undefined && !decodeCoreError(result.recoveryIssue))
            throw fail("resultUnknown", uiMessage("error.invalidResponse"));
    }
    private accept(result: HistoryRestoreResult): HistoryRestoreResult {
        this.validateRestore(result);
        this.pending = null;
        this.review = null;
        this.uncertain = false;
        const accepted = clone(result);
        if (accepted.recoveryIssue)
            accepted.recoveryIssue = decodeCoreError(accepted.recoveryIssue)!;
        return accepted;
    }
    private remember(status: HistoryOperationStatus): void {
        if (status.operationId !== this.pending!.request.operationId || !["missing", "committed", "recoveryPending", "resolved"].includes(status.status))
            throw fail("resultUnknown", uiMessage("historyState.queryReceiptMismatch"));
        if (this.operation?.status === "committed" && status.status !== "committed")
            throw fail("resultUnknown", uiMessage("historyState.committedLaterStatusMismatch"));
        if (status.status === "committed") {
            const priorReceipt = this.operation?.status === "committed" && validRestoreReceipt(this.operation.receipt, status.operationId) ? this.operation.receipt : undefined;
            const valid = validRestoreReceipt(status.receipt, status.operationId);
            if (priorReceipt && valid && (status.receipt!.baseId !== priorReceipt.baseId || status.receipt!.path !== priorReceipt.path))
                throw fail("resultUnknown", uiMessage("historyState.laterReceiptSaveMismatch"));
            this.operation = clone({ operationId: status.operationId, status: "committed", receipt: valid ? status.receipt : priorReceipt });
            this.uncertain = false;
            if (status.resultError !== undefined) {
                const error = decodeApiErrorBody(status.resultError);
                if (!error)
                    throw fail("resultUnknown", uiMessage("error.invalidResponse"));
                this.operation.resultError = error;
            }
            if (status.receipt !== undefined && !valid)
                throw fail("resultUnknown", uiMessage("historyState.committedSaveReceiptInvalid"));
            return;
        }
        this.operation = { operationId: status.operationId, status: status.status };
        this.uncertain = status.status !== "resolved";
    }
    private consume(status: HistoryOperationStatus): HistoryRestoreResult | null {
        this.remember(status);
        if (status.status === "committed") {
            const receipt = this.operation?.receipt;
            if (!validRestoreReceipt(receipt, status.operationId))
                throw fail("resultUnknown", uiMessage("historyState.committedReceiptIncomplete"));
            if (!status.result)
                return null;
            if (status.result.receipt?.baseId !== receipt.baseId || status.result.receipt?.path !== receipt.path)
                throw fail("resultUnknown", uiMessage("historyState.resultCommitReceiptMismatch"));
            return this.accept(status.result);
        }
        if (status.status === "resolved") {
            this.pending = null;
            this.review = null;
        }
        return null;
    }
    private async execute(): Promise<HistoryRestoreResult> {
        this.busy = true;
        try {
            const status = await this.host.restore(clone(this.pending!.request)), result = this.consume(status);
            if (result)
                return result;
            throw fail("resultUnavailable", status.status === "committed" ? displayMessage("historyState.committedDeliveryUnavailable", { p0: status.resultError ? apiErrorUiText(status.resultError) : uiMessage("historyState.queryOriginalRestoreResult") }) : uiMessage("historyState.completionNotDelivered"));
        }
        catch (error) {
            // Even an ordinary API error can arise while reading evidence after a
            // commit. Only the original operation status can prove no registered save.
            this.uncertain = this.operation?.status !== "committed";
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
    async inspectOriginal(): Promise<HistoryRestoreResult | null> {
        if (!this.pending || this.busy)
            throw fail("operationPending", uiMessage("historyState.noQueryableRestore"));
        this.busy = true;
        try {
            let result = await this.host.status({ operationId: this.pending.request.operationId });
            this.remember(result);
            if (result.status === "missing") {
                const settled = await this.host.settle(await originalFence(this.pending.attempt));
                if (settled.operationId !== this.pending.request.operationId)
                    throw fail("resultUnknown", uiMessage("historyState.terminationReceiptMismatch"));
                if (["committed", "saved"].includes(settled.status)) {
                    const receipt = validRestoreReceipt(settled, settled.operationId) ? { operationId: settled.operationId, status: settled.status, baseId: settled.baseId, path: settled.path } : undefined;
                    this.remember({ operationId: settled.operationId, status: "committed", receipt });
                    // A restore can commit between missing and settle. The generic flat
                    // receipt proves that fact; only history/status delivers its result.
                    result = await this.host.status({ operationId: settled.operationId });
                }
                else if (settled.status === "resolved" || settled.status === "recoveryPending")
                    result = { operationId: settled.operationId, status: settled.status };
                else
                    throw fail("resultUnknown", uiMessage("historyState.terminationUnconfirmed"));
            }
            return this.consume(result);
        }
        catch (error) {
            this.uncertain = this.operation?.status !== "committed";
            throw error;
        }
        finally {
            this.busy = false;
        }
    }
}
