// Disconnected drafts carry their actual base. Reopening never silently adopts
// a new base for an older proposal, including after a lost save response.
export interface HostedSaveRequest { sessionId: string; baseId: string; operationId: string; source: string; definitionReservations?: string[] }
export interface HostedDraft { source: string; baseSource: string; reference?: string; definitionReservations?: string[]; pending?: { operationId: string; source: string; request?: HostedSaveRequest } }
export function bindHostedDraftReference(draft: HostedDraft, reference: string): HostedDraft {
  if (draft.reference && draft.reference !== reference) throw new Error("当前获权笔记身份与原草稿不同；原引用与输入保留，请明确核对，未重绑原草稿。");
  return { ...draft, reference };
}
export interface HostedQueryAudience { epoch: number; credential: string; actor?: string; deployment?: string; generation?: number; sessionId?: string; queryVersion: number }
export function sameHostedQueryIdentity(left: HostedQueryAudience, right: HostedQueryAudience): boolean { return left.epoch === right.epoch && left.credential === right.credential && left.actor === right.actor && left.deployment === right.deployment && left.generation === right.generation; }
export function sameHostedQueryAudience(left: HostedQueryAudience, right: HostedQueryAudience): boolean { return sameHostedQueryIdentity(left, right) && left.sessionId === right.sessionId && left.queryVersion === right.queryVersion; }
/** Server actor epochs partition every recoverable protected consumer cache. */
export function hostedCachePrefix(kind: "draft" | "field" | "review" | "office" | "query" | "history" | "library" | "resource-import" | "structure", deployment: string, actor: string, generation: number): string {
  if (!deployment || !actor || !Number.isSafeInteger(generation) || generation < 0) throw new Error("服务端授权版本无效，未恢复旧缓存。");
  return `weftext-hosted-${kind}:${encodeURIComponent(deployment)}:${encodeURIComponent(actor)}:g${generation}:`;
}
export function draftDisposition(saved: string, cached?: HostedDraft): "current" | "restore" | "compare" | "unknown" {
  if (cached?.pending) return "unknown";
  if (!cached || cached.source === saved || cached.source === cached.baseSource) return "current";
  return cached.baseSource === saved ? "restore" : "compare";
}
export function applySaveResult(draft: HostedDraft, saved: string): HostedDraft {
  // Typing after dispatch must not be replaced by the older committed proposal.
  const { pending: _pending, ...retained } = draft;
  return { ...retained, baseSource: saved };
}
export function freezeHostedSave(draft: HostedDraft, sessionId: string, baseId: string, operationId: string): HostedSaveRequest {
  if ((draft.definitionReservations?.length ?? 0) > 32) throw new Error("本次草稿关联超过 32 个定义预约，请先按原编号核对并明确处理，尚未发送保存。");
  return { sessionId, baseId, operationId, source: draft.source, ...(draft.definitionReservations?.length ? { definitionReservations: [...draft.definitionReservations] } : {}) };
}
export function mergeDefinitionReservations(ids: readonly string[], receipts: readonly { operationId: string; status: string }[], includeReserved = false): string[] {
  const next = new Set(ids);
  for (const receipt of receipts) {
    if (["admitted", "cancelled"].includes(receipt.status)) next.delete(receipt.operationId);
    else if (includeReserved && ["reserved", "recoveryPending"].includes(receipt.status)) next.add(receipt.operationId);
  }
  return [...next];
}
