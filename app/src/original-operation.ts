/** Presentation keeps the frozen request; only the original Core journal decides effects. */
export interface OriginalFact { operationId?: string; status: string; deliveryError?: { message: string }; documentError?: { message?: string } }
export type OriginalRequest = <T>(path: string, body: unknown) => Promise<T>;
export interface FrozenOriginalAttempt { readonly operationId: string; readonly requestKind: string; readonly wire: string }
export interface OriginalFence { operationId: string; requestKind: string; wireDigest: string }
/** Capture actual JSON bytes before any network/hash await or later author input. */
export function freezeOriginalAttempt(payload: object, requestKind: string): FrozenOriginalAttempt {
  const operationId = (payload as { operationId?: unknown }).operationId;
  if (typeof operationId !== "string") throw new Error("原请求没有操作编号，输入保留。");
  return Object.freeze({ operationId, requestKind, wire: JSON.stringify(payload) });
}
/** The fence never reparses author payload: even rejected Unicode has an exact wire digest. */
export async function originalFence(attempt: FrozenOriginalAttempt): Promise<OriginalFence> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(attempt.wire));
  return { operationId: attempt.operationId, requestKind: attempt.requestKind, wireDigest: [...new Uint8Array(digest)].map(value => value.toString(16).padStart(2, "0")).join("") };
}
export class TerminatedAttempt extends Error {
  constructor(public originalError: unknown) { super(originalError instanceof Error ? originalError.message : "原操作已安全终结，输入保留，可修改后重试。"); }
}
function exact<T extends OriginalFact>(fact: T, id: unknown, statusRead = false): T {
  if (!fact || typeof fact !== "object" || fact.operationId !== id || !["saved", "committed", "unknown", "prepared", "recoveryPending", "resolved"].includes(fact.status) || statusRead && fact.status === "saved") throw new Error("回执不能确认为原操作，保留原请求与输入。");
  return fact;
}
export const originalCommitted = (fact: OriginalFact): boolean => fact.status === "saved" || fact.status === "committed";
export function committedDeliveryComplete(fact: OriginalFact): boolean { return originalCommitted(fact) && !fact.deliveryError && !fact.documentError; }
/** No error code proves absence. Query, then durably fence a missing attempt. */
async function reconcileOriginal<T extends OriginalFact>(request: OriginalRequest, attempt: FrozenOriginalAttempt, error: unknown): Promise<T> {
  const operationId = attempt.operationId;
  let fact = exact(await request<T>("/api/operations/status", { operationId }), operationId, true);
  if (originalCommitted(fact)) return fact;
  if (fact.status === "unknown") fact = exact(await request<T>("/api/operations/settle", await originalFence(attempt)), operationId, true);
  if (originalCommitted(fact)) return fact;
  if (fact.status === "resolved") throw new TerminatedAttempt(error);
  throw error;
}
export async function submitOriginal<T extends OriginalFact>(request: OriginalRequest, route: string, payload: Record<string, unknown>, kind: string, recoverFirst = false): Promise<T> {
  const attempt = freezeOriginalAttempt(payload, kind);
  if (recoverFirst) return reconcileOriginal(request, attempt, new Error("原操作仍待确认，输入保留。"));
  try {
    const fact = exact(await request<T>(route, JSON.parse(attempt.wire)), attempt.operationId);
    if (!originalCommitted(fact) || fact.status === "saved" && !["save", "saveAs"].includes(kind)) throw new Error("原请求尚无适用的已提交回执，保留原编号查询。");
    return fact;
  }
  catch (error) { return reconcileOriginal(request, attempt, error); }
}
/** A callback/refresh is delivery, not another author transaction. */
export async function deliverCommitted<T extends OriginalFact>(receipt: T, deliver: (receipt: T) => Promise<void>): Promise<T & OriginalFact> {
  if (!originalCommitted(receipt) || receipt.documentError) return receipt;
  const delivered = { ...receipt }; delete delivered.deliveryError;
  try { await deliver(delivered); }
  catch (error) { delivered.deliveryError = { message: error instanceof Error ? error.message : String(error) }; }
  return delivered;
}
