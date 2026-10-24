/** The native surface close protocol. Participants provide actual Core evidence;
 * this module never turns a browser/sessionStorage acknowledgement into a permit. */
export type SurfaceKind = "library" | "capture";
export type CloseRequest = Readonly<{ surfaceId: string; surfaceEpoch: string; closeAttemptId: string }>;
export type DocumentCloseProof = Readonly<{
  kind: "document"; ownerId: string; ownerGeneration: string; sessionId: string;
  baseId: string; sourceDigest: string; recordDigest: string; pendingOperationId?: string;
}>;
export type CaptureCloseProof = Readonly<{
  kind: "capture"; ownerId: string; ownerGeneration: string; draftId: string;
  recordRevision: number; inputRevision: number; lastMutationId: string;
  recordDigest: string; pendingOperationId?: string;
}>;
export type CloseProof = DocumentCloseProof | CaptureCloseProof;
export type ClosePrepared = Readonly<{ surfaceEditRevision: number; proofs: readonly CloseProof[] }>;
export type CloseParticipant = Readonly<{
  /** The owner must flush its live producer/queue before returning proof. */
  prepare(request: CloseRequest): Promise<readonly CloseProof[]>;
  /** Called synchronously before commit acknowledgement. No pending producer may
   * change the prepared input after this returns; failure keeps the surface. */
  freeze(request: CloseRequest): boolean;
  thaw(): void;
}>;

type WebView = {
  postMessage(value: unknown): void;
  addEventListener(type: "message", listener: (event: MessageEvent<unknown>) => void): void;
};
declare global { interface Window { chrome?: { webview?: WebView } } }

function bridge(): WebView | undefined { return window.chrome?.webview; }
function isRequest(value: unknown): value is CloseRequest {
  if (!value || typeof value !== "object") return false;
  const request = value as Partial<CloseRequest>;
  return [request.surfaceId, request.surfaceEpoch, request.closeAttemptId].every(item => typeof item === "string" && item.length > 0 && item.length <= 128);
}

/** One instance for the real library or Capture controller, installed before
 * the host exposes the WebView. Register every producer; never register an empty
 * success adapter for a participant whose persistence is not implemented. */
export class NativeSurface {
  private editRevision = 0;
  private composing = false;
  private binding?: { surfaceId: string; surfaceEpoch: string };
  private pending?: CloseRequest & ClosePrepared;
  private preparing?: CloseRequest;
  private activeAttempt?: CloseRequest;
  private frozen = false;
  private readonly participants = new Map<string, CloseParticipant>();
  private ready = false;
  private readonly listeners = new Set<(message: Record<string, unknown>) => void>();

  constructor(readonly kind: SurfaceKind) {
    const changed = (): void => {
      ++this.editRevision;
      this.pending = undefined;
      this.preparing = undefined;
      this.send({ type: "surface-edited", surfaceEditRevision: this.editRevision });
    };
    document.addEventListener("beforeinput", changed, true);
    document.addEventListener("input", changed, true);
    document.addEventListener("compositionstart", () => { this.composing = true; changed(); }, true);
    document.addEventListener("compositionend", () => { this.composing = false; changed(); }, true);
    bridge()?.addEventListener("message", event => { void this.receive(event.data); });
  }

  get isNative(): boolean { return bridge() !== undefined; }
  reportFailed(): void { bridge()?.postMessage({ type: "surface-failed", kind: this.kind }); }
  /** Fixed product messages still pass the host's typed protocol and original
   * surface checks. Callers cannot supply or replace an incarnation binding. */
  post(value: Record<string, unknown>): void {
    if (!this.binding || !bridge()) throw new Error("The original native surface is not bound");
    this.send(value);
  }
  listen(listener: (message: Record<string, unknown>) => void): () => void {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  }

  register(owner: string, participant: CloseParticipant): () => void {
    if (this.ready || this.participants.has(owner)) throw new Error("Native close participant ownership is already frozen");
    this.participants.set(owner, participant);
    return () => {
      this.participants.delete(owner); this.pending = undefined; this.preparing = undefined;
      this.ready = false;
      this.send({ type: "surface-unready" });
    };
  }

  /** Call only after every actual author producer has registered its barrier. */
  markReady(): void {
    if (!this.participants.size) throw new Error("A native author surface needs its actual persistence participant");
    this.ready = true;
    bridge()?.postMessage({ type: "surface-ready", kind: this.kind });
  }

  private send(value: Record<string, unknown>): void {
    if (this.binding) bridge()?.postMessage({ ...value, ...this.binding });
  }

  private thaw(): void {
    this.frozen = false;
    document.documentElement.inert = false;
    for (const participant of this.participants.values()) participant.thaw();
  }

  private async receive(value: unknown): Promise<void> {
    if (!value || typeof value !== "object") return;
    const message = value as Record<string, unknown>;
    if (message.type === "surface-bound") {
      if (![message.surfaceId, message.surfaceEpoch].every(item => typeof item === "string" && item.length > 0 && item.length <= 128)) return;
      const binding = { surfaceId: message.surfaceId as string, surfaceEpoch: message.surfaceEpoch as string };
      if (this.binding) {
        if (this.binding.surfaceId !== binding.surfaceId || this.binding.surfaceEpoch !== binding.surfaceEpoch) this.reportFailed();
        return;
      }
      try { bindNativeDraftSurface(binding); }
      catch { this.reportFailed(); return; }
      this.binding = binding;
      this.send({ type: "surface-edited", surfaceEditRevision: this.editRevision });
      return;
    }
    if (message.type === "close-cancelled") {
      if (!isRequest(value) || !this.binding || value.surfaceId !== this.binding.surfaceId || value.surfaceEpoch !== this.binding.surfaceEpoch
        || this.activeAttempt?.closeAttemptId !== value.closeAttemptId) return;
      this.pending = undefined; this.preparing = undefined; this.activeAttempt = undefined; this.thaw(); return;
    }
    if (message.type !== "prepare-close" && message.type !== "commit-close") {
      if (this.binding && message.surfaceId === this.binding.surfaceId && message.surfaceEpoch === this.binding.surfaceEpoch)
        for (const listener of this.listeners) listener(message);
      return;
    }
    if (!isRequest(value) || !this.binding || value.surfaceId !== this.binding.surfaceId || value.surfaceEpoch !== this.binding.surfaceEpoch) return;
    const request = value;
    if (message.type === "prepare-close") {
      // Repeated control messages do not restart the original asynchronous
      // owner or replace the identity needed to thaw an already frozen cut.
      if (this.activeAttempt?.closeAttemptId === request.closeAttemptId) return;
      if (this.frozen) {
        this.send({ type: "close-blocked", closeAttemptId: request.closeAttemptId, reason: "The original close is still frozen" });
        return;
      }
      this.pending = undefined;
      this.preparing = request;
      this.activeAttempt = request;
      const serial = this.editRevision;
      try {
        if (!this.ready || this.composing || this.frozen) throw new Error("Author input has not settled");
        const proofs: CloseProof[] = [];
        for (const participant of this.participants.values()) {
          proofs.push(...await participant.prepare(request));
          // Cancellation or supersession may arrive at any await. Do not
          // start another producer on behalf of a retired close attempt.
          if (!this.ready || this.composing || serial !== this.editRevision || this.preparing !== request)
            throw new Error("Author input or close ownership changed while preparing close");
        }
        this.pending = { ...request, surfaceEditRevision: serial, proofs };
        this.send({ type: "close-prepared", closeAttemptId: request.closeAttemptId, surfaceEditRevision: serial, proofs });
      } catch (error) {
        this.send({ type: "close-blocked", closeAttemptId: request.closeAttemptId, reason: error instanceof Error ? error.message : "Persistence is unconfirmed" });
      } finally { if (this.preparing === request) this.preparing = undefined; }
    } else if (message.type === "commit-close") {
      const pending = this.pending;
      if (!pending || pending.closeAttemptId !== request.closeAttemptId || pending.surfaceEditRevision !== this.editRevision || this.composing) {
        this.send({ type: "close-blocked", closeAttemptId: request.closeAttemptId, reason: "Prepared author input is no longer current" });
        return;
      }
      if (this.frozen) return;
      this.frozen = true;
      document.documentElement.inert = true;
      try {
        for (const participant of this.participants.values()) if (!participant.freeze(request)) throw new Error("A live producer could not freeze");
        this.send({ type: "close-frozen", closeAttemptId: request.closeAttemptId, surfaceEditRevision: this.editRevision });
      } catch (error) {
        this.pending = undefined;
        this.thaw();
        this.send({ type: "close-blocked", closeAttemptId: request.closeAttemptId, reason: error instanceof Error ? error.message : "Author input could not freeze" });
      }
    }
  }
}
import { bindNativeDraftSurface } from "./native-draft";
