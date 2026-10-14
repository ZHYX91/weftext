import type { HistoryContext, HistoryRestoreBasis, HistoryRestoreResult } from "./history-state";

export interface HostedHistoryContext extends HistoryContext { resource: string }
export interface HostedHistoryRequest { resource: string; sessionId: string; baseId: string; versionId: string; operationId: string; draftAction: "retain" | "none"; source: string; serial: number; sourceHash: string }
export interface HostedHistoryStatus {
  operationId: string; resource: string;
  status: "notRegistered" | "notCommitted" | "committed" | "recoveryPending" | "resolved" | "cancelled";
  resultStatus: "available" | "unavailable";
  resultBasis?: "current" | "superseded" | "external";
  currentRevision?: string | null;
  result?: HistoryRestoreResult & { receipt: HistoryRestoreResult["receipt"] & { revisionId?: string } };
}
export interface HostedHistoryRecord { request: HostedHistoryRequest; basis: HistoryRestoreBasis & { resource: string }; status?: HostedHistoryStatus; acknowledged?: boolean }
export function hostedHistoryStatus(record: HostedHistoryRecord, incoming: HostedHistoryStatus): HostedHistoryStatus {
  if (incoming.operationId !== record.request.operationId || incoming.resource !== record.request.resource || !["notRegistered", "notCommitted", "committed", "recoveryPending", "resolved", "cancelled"].includes(incoming.status)) throw new Error("历史恢复回执不对应原操作或原对象，未替换待核对记录。");
  if (incoming.result && (incoming.status !== "committed" || incoming.result.receipt?.operationId !== record.request.operationId || !["saved", "committed"].includes(incoming.result.receipt.status) || !incoming.result.receipt.baseId || incoming.result.receipt.path !== `resource:${record.request.resource}` || incoming.result.source !== record.basis.selectedSource || incoming.result.versionId !== record.request.versionId || !Array.isArray(incoming.result.protectedDrafts) || incoming.result.protectedDrafts.some(draft => draft.path !== `resource:${record.request.resource}` || typeof draft.source !== "string" || typeof draft.baseSource !== "string" || typeof draft.baseKnown !== "boolean"))) throw new Error("原恢复结果缺少准确版本、保稿或过滤后的对象依据，尚未应用回执。");
  // Do not merge an earlier deliverable source into a newly unavailable receipt.
  return structuredClone(incoming);
}
export function hostedHistoryPending(record: HostedHistoryRecord): boolean { return !record.acknowledged && !["cancelled", "resolved"].includes(record.status?.status ?? ""); }
export function unavailableCommittedHistory(record: HostedHistoryRecord): boolean { return record.status?.status === "committed" && record.status.resultStatus === "unavailable" && !record.status.result && !record.acknowledged; }
export function currentHostedHistoryResult(record: HostedHistoryRecord, source: string, confirmed: HostedHistoryStatus): boolean {
  const status = hostedHistoryStatus(record, confirmed), result = status.result;
  return !!result && status.status === "committed" && status.resultBasis === "current" && typeof result.receipt.revisionId === "string" && status.currentRevision === result.receipt.revisionId && result.source === source;
}
