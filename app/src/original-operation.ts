import { message as uiMessage } from "./ui-locale";
import { decodeApiErrorBody, apiErrorUiText, type ApiErrorBody } from "./api-error";
import { UiError, errorUiText, type UiText } from "./ui-locale";
/** Presentation keeps the frozen request; only the original Core journal decides effects. */
export interface OriginalFact { operationId?: string; status: string; deliveryError?: ApiErrorBody; documentError?: ApiErrorBody }
// A callback failure belongs to this UI delivery attempt, never the server fact.
// A non-enumerable symbol cannot become a JSON protocol or a persisted effect.
const clientDeliveryFailure: unique symbol = Symbol("clientDeliveryFailure");
type DeliveryPresentation = { readonly [clientDeliveryFailure]?: UiText };
/** Carry only this process's branded presentation through a validated UI projection. */
export function copyOriginalDeliveryPresentation<T extends object>(from: object, into: T): T {
  const local = (from as DeliveryPresentation)[clientDeliveryFailure];
  if (local !== undefined) Object.defineProperty(into, clientDeliveryFailure, { value: local });
  return into;
}
export function originalDeliveryErrors(fact: Pick<OriginalFact, "deliveryError" | "documentError">): readonly UiText[] {
  const local = (fact as DeliveryPresentation)[clientDeliveryFailure];
  return [...(fact.deliveryError ? [apiErrorUiText(fact.deliveryError)] : []), ...(fact.documentError ? [apiErrorUiText(fact.documentError)] : []), ...(local === undefined ? [] : [local])];
}
export type OriginalRequest = <T>(path: string, body: unknown) => Promise<T>;
export interface FrozenOriginalAttempt { readonly operationId: string; readonly requestKind: string; readonly wire: string }
export interface OriginalFence { operationId: string; requestKind: string; wireDigest: string }
/** Capture actual JSON bytes before any network/hash await or later author input. */
export function freezeOriginalAttempt(payload: object, requestKind: string): FrozenOriginalAttempt {
  const operationId = (payload as { operationId?: unknown }).operationId;
  if (typeof operationId !== "string") throw new UiError(uiMessage("originalOperation.requestIdMissing"));
  return Object.freeze({ operationId, requestKind, wire: JSON.stringify(payload) });
}
/** The fence never reparses author payload: even rejected Unicode has an exact wire digest. */
export async function originalFence(attempt: FrozenOriginalAttempt): Promise<OriginalFence> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(attempt.wire));
  return { operationId: attempt.operationId, requestKind: attempt.requestKind, wireDigest: [...new Uint8Array(digest)].map(value => value.toString(16).padStart(2, "0")).join("") };
}
export class TerminatedAttempt extends UiError {
  constructor(public originalError: unknown) { super(originalError instanceof Error ? errorUiText(originalError) : uiMessage("originalOperation.endedInputRetained")); }
}
function exact<T extends OriginalFact>(fact: T, id: unknown, statusRead = false): T {
  if (!fact || typeof fact !== "object" || fact.operationId !== id || !["saved", "committed", "unknown", "prepared", "recoveryPending", "resolved"].includes(fact.status) || statusRead && fact.status === "saved") throw new UiError(uiMessage("originalOperation.receiptUnconfirmed"));
  const errors: {deliveryError?: ApiErrorBody; documentError?: ApiErrorBody} = {};
  for (const slot of ["deliveryError", "documentError"] as const) {
    if (Object.hasOwn(fact, slot)) {
      const decoded = decodeApiErrorBody(fact[slot]);
      if (!decoded) throw new UiError(uiMessage("originalOperation.deliveryErrorInvalid"));
      errors[slot] = decoded;
    }
  }
  return { ...fact, ...errors };
}
export const originalCommitted = (fact: OriginalFact): boolean => fact.status === "saved" || fact.status === "committed";
export function committedDeliveryComplete(fact: OriginalFact): boolean { return originalCommitted(fact) && originalDeliveryErrors(fact).length === 0; }
/** No error code proves absence. Query, then durably fence a missing attempt. */
async function reconcileOriginal<T extends OriginalFact>(request: OriginalRequest, attempt: FrozenOriginalAttempt, error: unknown): Promise<T> {
  const operationId = attempt.operationId;
  // A same-editor Save retry explicitly identifies its still-live writer.
  // Generic lookup and SaveAs never acquire that producer from a receipt.
  const sessionId = attempt.requestKind === "save" ? (JSON.parse(attempt.wire) as {sessionId?: unknown}).sessionId : undefined;
  let fact = exact(await request<T>("/api/operations/status", { operationId, ...(typeof sessionId === "string" ? {sessionId} : {}) }), operationId, true);
  if (originalCommitted(fact)) return fact;
  if (fact.status === "unknown") fact = exact(await request<T>("/api/operations/settle", await originalFence(attempt)), operationId, true);
  if (originalCommitted(fact)) return fact;
  if (fact.status === "resolved") throw new TerminatedAttempt(error);
  throw error;
}
export async function submitOriginal<T extends OriginalFact>(request: OriginalRequest, route: string, payload: Record<string, unknown>, kind: string, recoverFirst = false): Promise<T> {
  const attempt = freezeOriginalAttempt(payload, kind);
  if (recoverFirst) return reconcileOriginal(request, attempt, new UiError(uiMessage("originalOperation.awaitingConfirmation")));
  try {
    const fact = exact(await request<T>(route, JSON.parse(attempt.wire)), attempt.operationId);
    if (!originalCommitted(fact) || fact.status === "saved" && !["save", "saveAs"].includes(kind)) throw new UiError(uiMessage("originalOperation.applicableCommittedReceiptRequired"));
    return fact;
  }
  catch (error) { return reconcileOriginal(request, attempt, error); }
}
/** A callback/refresh is delivery, not another author transaction. */
export async function deliverCommitted<T extends OriginalFact>(receipt: T, deliver: (receipt: T) => Promise<void>): Promise<T & OriginalFact> {
  if (!originalCommitted(receipt) || receipt.documentError || receipt.deliveryError) return receipt;
  const delivered = { ...receipt };
  try { await deliver(delivered); }
  catch (error) { Object.defineProperty(delivered, clientDeliveryFailure, { value: errorUiText(error) }); }
  return delivered;
}
