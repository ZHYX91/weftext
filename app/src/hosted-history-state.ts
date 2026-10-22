import { message as uiMessage, UiError, type UiText } from "./ui-locale";
import type { HistoryContext, HistoryRestoreBasis, HistoryRestoreResult } from "./history-state";
import { decodeApiErrorBody, type ApiErrorBody } from "./api-error";
import { decodeCoreError, exactKeys } from "./ui-diagnostics";
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
type HostedRestoreReceipt = HistoryRestoreResult["receipt"] & { revisionId: string };
export interface HostedHistoryStatus {
    operationId: string;
    resource: string;
    status: "notRegistered" | "notCommitted" | "committed" | "recoveryPending" | "resolved" | "cancelled";
    resultStatus: "available" | "unavailable";
    resultBasis?: "current" | "superseded" | "external";
    currentRevision?: string | null;
    receipt?: HostedRestoreReceipt;
    resultError?: ApiErrorBody;
    /** Client guard only; never a delivered server fact or part of an outgoing request. */
    guardReason?: UiText;
    result?: HistoryRestoreResult & { receipt: HostedRestoreReceipt };
}
export interface HostedHistoryRecord {
    request: HostedHistoryRequest;
    basis: HistoryRestoreBasis & {
        resource: string;
    };
    status?: HostedHistoryStatus;
    acknowledged?: boolean;
}
function committedFact(record: HostedHistoryRecord, explanation?: UiText): HostedHistoryStatus {
    return {
        operationId: record.request.operationId, resource: record.request.resource, status: "committed", resultStatus: "unavailable",
        ...(validReceipt(record.status?.receipt, record) ? { receipt: structuredClone(record.status!.receipt) } : {}),
        ...(explanation ? { guardReason: explanation } : {}),
    };
}
function validReceipt(receipt: unknown, record: HostedHistoryRecord): receipt is HostedRestoreReceipt {
    if (!receipt || typeof receipt !== "object" || Array.isArray(receipt))
        return false;
    const value = receipt as HostedRestoreReceipt;
    return exactKeys(receipt as Record<string, unknown>, ["operationId", "status", "baseId", "path", "revisionId"], ["recoveryIssue"])
        && value.operationId === record.request.operationId && ["saved", "committed"].includes(value.status)
        && typeof value.baseId === "string" && !!value.baseId && value.path === `resource:${record.request.resource}`
        && typeof value.revisionId === "string" && !!value.revisionId
        && (value.recoveryIssue === undefined || !!decodeCoreError(value.recoveryIssue));
}
export function hostedHistoryStatus(record: HostedHistoryRecord, incoming: HostedHistoryStatus): HostedHistoryStatus {
    if (!incoming || incoming.operationId !== record.request.operationId || incoming.resource !== record.request.resource || !["notRegistered", "notCommitted", "committed", "recoveryPending", "resolved", "cancelled"].includes(incoming.status)) {
        if (record.status?.status === "committed")
            return committedFact(record, uiMessage("hostedHistoryState.committedReceiptMismatch"));
        throw new UiError(uiMessage("hostedHistoryState.restoreReceiptMismatch"));
    }
    if (record.status?.status === "committed" && incoming.status !== "committed")
        return committedFact(record, uiMessage("hostedHistoryState.committedStatusCannotRegress"));
    const status: HostedHistoryStatus = { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, resultStatus: "unavailable" };
    if (incoming.resultError !== undefined) {
        const error = decodeApiErrorBody(incoming.resultError);
        if (!error) {
            if (incoming.status === "committed")
                return committedFact(record, uiMessage("hostedHistoryState.committedResultIncomplete"));
            throw new UiError(uiMessage("error.invalidResponse"));
        }
        status.resultError = error;
    }
    if (incoming.status !== "committed") {
        if (incoming.result)
            throw new UiError(uiMessage("hostedHistoryState.uncommittedResultRejected"));
        return status;
    }
    // A saved fact can survive qualification loss; delivery is renewed only by this response.
    if (record.status?.status === "committed" && validReceipt(record.status.receipt, record))
        status.receipt = structuredClone(record.status.receipt);
    const result = incoming.result;
    if (!result && incoming.resultStatus === "unavailable")
        return status;
    const receipt = result?.receipt;
    if (incoming.resultStatus !== "available" || !result || !validReceipt(receipt, record)
        || result.source !== record.basis.selectedSource || result.versionId !== record.request.versionId
        || !Array.isArray(result.protectedDrafts) || result.protectedDrafts.some(draft => !draft || draft.path !== `resource:${record.request.resource}` || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.baseKnown !== "boolean")
        || result.recoveryIssue !== undefined && !decodeCoreError(result.recoveryIssue)
        || !["current", "superseded", "external"].includes(incoming.resultBasis ?? "")
        || incoming.currentRevision !== null && typeof incoming.currentRevision !== "string"
        || status.receipt && (receipt.baseId !== status.receipt.baseId || receipt.path !== status.receipt.path || receipt.revisionId !== status.receipt.revisionId)) {
        status.guardReason = uiMessage("hostedHistoryState.committedResultIncomplete");
        return status;
    }
    status.receipt = structuredClone(receipt);
    status.resultStatus = "available";
    status.result = structuredClone(result);
    if (status.result.recoveryIssue)
        status.result.recoveryIssue = decodeCoreError(status.result.recoveryIssue)!;
    status.resultBasis = incoming.resultBasis;
    status.currentRevision = incoming.currentRevision;
    return status;
}
/** Reconnect retains a conservative commit gate, never cached delivery rights. */
export function resumeHostedHistoryRecord(record: HostedHistoryRecord): HostedHistoryRecord {
    const resumed = structuredClone(record);
    resumed.status = record.status?.status === "committed" ? hostedHistoryStatus(record, { operationId: record.request.operationId, resource: record.request.resource, status: "committed", resultStatus: "unavailable" }) : undefined;
    return resumed;
}
export function hostedHistoryPending(record: HostedHistoryRecord): boolean { return !record.acknowledged && !["cancelled", "resolved"].includes(record.status?.status ?? ""); }
export function unavailableCommittedHistory(record: HostedHistoryRecord): boolean { return record.status?.status === "committed" && record.status.resultStatus === "unavailable" && !record.status.result && !record.acknowledged; }
export function currentHostedHistoryResult(record: HostedHistoryRecord, source: string, confirmed: HostedHistoryStatus): boolean {
    const status = hostedHistoryStatus(record, confirmed), result = status.result;
    return !!result && status.status === "committed" && status.resultBasis === "current" && typeof result.receipt.revisionId === "string" && status.currentRevision === result.receipt.revisionId && result.source === source;
}
