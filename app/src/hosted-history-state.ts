import type { HistoryContext, HistoryRestoreBasis, HistoryRestoreResult } from "./history-state";

export interface HostedHistoryContext extends HistoryContext { resource: string }
type HostedRestoreReceipt = HistoryRestoreResult["receipt"] & { revisionId: string };
export interface HostedHistoryRequest { resource: string; sessionId: string; baseId: string; versionId: string; operationId: string; draftAction: "retain" | "none"; source: string; serial: number; sourceHash: string }
export interface HostedHistoryStatus {
  operationId: string; resource: string;
  status: "notRegistered" | "notCommitted" | "committed" | "recoveryPending" | "resolved" | "cancelled";
  resultStatus: "available" | "unavailable";
  resultBasis?: "current" | "superseded" | "external";
  currentRevision?: string | null;
  receipt?: HostedRestoreReceipt;
  resultError?: string;
  result?: HistoryRestoreResult & { receipt: HostedRestoreReceipt };
}
export interface HostedHistoryRecord { request: HostedHistoryRequest; basis: HistoryRestoreBasis & { resource: string }; status?: HostedHistoryStatus; acknowledged?: boolean }
function committedFact(record: HostedHistoryRecord, message?: string): HostedHistoryStatus {
  return { operationId: record.request.operationId, resource: record.request.resource, status: "committed", resultStatus: "unavailable",
    ...(validReceipt(record.status?.receipt, record) ? { receipt: structuredClone(record.status!.receipt) } : {}), ...(message ? { resultError: message } : {}) };
}
function validReceipt(receipt: unknown, record: HostedHistoryRecord): receipt is HostedRestoreReceipt {
  if (!receipt || typeof receipt !== "object") return false;
  const value = receipt as HostedRestoreReceipt;
  return value.operationId === record.request.operationId && ["saved", "committed"].includes(value.status)
    && typeof value.baseId === "string" && !!value.baseId && typeof value.message === "string"
    && value.path === `resource:${record.request.resource}` && typeof value.revisionId === "string" && !!value.revisionId;
}
export function hostedHistoryStatus(record: HostedHistoryRecord, incoming: HostedHistoryStatus): HostedHistoryStatus {
  if (!incoming || incoming.operationId !== record.request.operationId || incoming.resource !== record.request.resource || !["notRegistered", "notCommitted", "committed", "recoveryPending", "resolved", "cancelled"].includes(incoming.status)) {
    if (record.status?.status === "committed") return committedFact(record, "后续回执不对应原恢复；原提交事实与编号保留，交付资格等待重新核对。");
    throw new Error("历史恢复回执不对应原操作或原对象，未替换待核对记录。");
  }
  if (record.status?.status === "committed" && incoming.status !== "committed") return committedFact(record, "原恢复已确认提交，后续状态不能解除原号或改成未提交。");
  const status: HostedHistoryStatus = { operationId: incoming.operationId, resource: incoming.resource, status: incoming.status, resultStatus: "unavailable" };
  if (incoming.status !== "committed") {
    if (incoming.result) throw new Error("未提交状态不能携带恢复结果，原记录保留。");
    return status;
  }
  // Keep only a previously validated receipt. Delivery qualification and body
  // are renewed by this response; a bad/unavailable response revives neither.
  if (record.status?.status === "committed" && validReceipt(record.status.receipt, record)) status.receipt = structuredClone(record.status.receipt);
  const result = incoming.result;
  if (!result && incoming.resultStatus === "unavailable") return status;
  const receipt = result?.receipt;
  if (incoming.resultStatus !== "available" || !result || !validReceipt(receipt, record)
    || result.source !== record.basis.selectedSource || result.versionId !== record.request.versionId
    || !Array.isArray(result.protectedDrafts) || result.protectedDrafts.some(draft => !draft || draft.path !== `resource:${record.request.resource}` || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.baseKnown !== "boolean")
    || !["current", "superseded", "external"].includes(incoming.resultBasis ?? "")
    || incoming.currentRevision !== null && typeof incoming.currentRevision !== "string"
    || status.receipt && (receipt.baseId !== status.receipt.baseId || receipt.path !== status.receipt.path || receipt.revisionId !== status.receipt.revisionId)) {
    status.resultError = "原恢复已提交，但准确结果或回执不完整；原事实和编号保留，请继续查询。"; return status;
  }
  status.receipt = structuredClone(receipt); status.resultStatus = "available"; status.result = structuredClone(result);
  status.resultBasis = incoming.resultBasis; status.currentRevision = incoming.currentRevision;
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
