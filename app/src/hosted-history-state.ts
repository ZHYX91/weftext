import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
import type { HistoryContext, HistoryRestoreBasis, HistoryRestoreResult } from "./history-state";
export interface HostedHistoryContext extends HistoryContext {
    resource: string;
}
export interface HostedHistoryRequest {
    resource: string;
    sessionId: string;
    baseId: string;
    versionId: string;
    operationId: string;
    draftAction: "retain" | "none";
    source: string;
    serial: number;
    sourceHash: string;
}
export interface HostedHistoryStatus {
    operationId: string;
    resource: string;
    status: "notRegistered" | "notCommitted" | "committed" | "recoveryPending" | "resolved" | "cancelled";
    resultStatus: "available" | "unavailable";
    resultBasis?: "current" | "superseded" | "external";
    currentRevision?: string | null;
    result?: HistoryRestoreResult & {
        receipt: HistoryRestoreResult["receipt"] & {
            revisionId?: string;
        };
    };
}
export interface HostedHistoryRecord {
    request: HostedHistoryRequest;
    basis: HistoryRestoreBasis & {
        resource: string;
    };
    status?: HostedHistoryStatus;
    acknowledged?: boolean;
}
export function hostedHistoryStatus(record: HostedHistoryRecord, incoming: HostedHistoryStatus): HostedHistoryStatus {
    if (incoming.operationId !== record.request.operationId || incoming.resource !== record.request.resource || !["notRegistered", "notCommitted", "committed", "recoveryPending", "resolved", "cancelled"].includes(incoming.status))
        throw new UiError(uiMessage("hostedHistoryState.restoreReceiptMismatch"));
    if (incoming.result && (incoming.status !== "committed" || incoming.result.receipt?.operationId !== record.request.operationId || !["saved", "committed"].includes(incoming.result.receipt.status) || !incoming.result.receipt.baseId || incoming.result.receipt.path !== `resource:${record.request.resource}` || incoming.result.source !== record.basis.selectedSource || incoming.result.versionId !== record.request.versionId || !Array.isArray(incoming.result.protectedDrafts) || incoming.result.protectedDrafts.some(draft => draft.path !== `resource:${record.request.resource}` || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.baseKnown !== "boolean")))
        throw new UiError(uiMessage("hostedHistoryState.restoreBasisMissing"));
    // Do not merge an earlier deliverable source into a newly unavailable receipt.
    return structuredClone(incoming);
}
export function hostedHistoryPending(record: HostedHistoryRecord): boolean { return !record.acknowledged && !["cancelled", "resolved"].includes(record.status?.status ?? ""); }
export function unavailableCommittedHistory(record: HostedHistoryRecord): boolean { return record.status?.status === "committed" && record.status.resultStatus === "unavailable" && !record.status.result && !record.acknowledged; }
export function currentHostedHistoryResult(record: HostedHistoryRecord, source: string, confirmed: HostedHistoryStatus): boolean {
    const status = hostedHistoryStatus(record, confirmed), result = status.result;
    return !!result && status.status === "committed" && status.resultBasis === "current" && typeof result.receipt.revisionId === "string" && status.currentRevision === result.receipt.revisionId && result.source === source;
}
