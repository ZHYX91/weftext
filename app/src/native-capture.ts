import { NativeSurface, type CloseParticipant, type CloseRequest } from "./native-host";
import type { MountedLocalCapture } from "./capture-mount";
import type { PreparedCaptureClose } from "./capture-close";
import type { CaptureResultIdentity } from "./capture-local";
import { beginNativeRequest, freezeNativeApi, nativeApiIdle, thawNativeApi } from "./native-api-gate";
import { message, UiError } from "./ui-locale";

/** One mounted Capture owner and the original host delivery. A queued request
 * cannot consume the Core pending operation: only the Library's adoption ACK
 * lets LocalCaptureSession advance that original. */
export class NativeCapture {
  private closing = false;
  private retired = false;
  private closeGeneration = 0;
  private prepared?: { request: string; proof: PreparedCaptureClose };
  private delivery?: { requestId: string; resolve(): void; reject(error: Error): void };
  private readonly stop: () => void;
  readonly participant: CloseParticipant;

  constructor(private readonly surface: NativeSurface, private readonly mounted: MountedLocalCapture) {
    this.stop = surface.listen(message => {
      if (message.type !== "capture-open-result" || !this.delivery || message.requestId !== this.delivery.requestId) return;
      const delivery = this.delivery;
      if (message.accepted !== true && message.accepted !== false) return;
      this.delivery = undefined;
      if (message.accepted) delivery.resolve();
      else delivery.reject(new UiError(messageForUnconfirmedDelivery()));
    });
    this.participant = {
      prepare: async request => {
        this.prepared = undefined;
        this.assertReady();
        if(this.closing)throw new UiError(message("desktop.unsavedClose"));
        this.closing = true;
        const generation = ++this.closeGeneration;
        let proof: PreparedCaptureClose;
        try {
          proof = await mounted.guard.prepare(); this.assertReady();
          if(generation !== this.closeGeneration)throw new UiError(message("desktop.unsavedClose"));
        }
        catch(error) { if(generation === this.closeGeneration)this.closing = false; throw error; }
        this.prepared = { request: key(request), proof };
        return [{ kind: "capture", ownerId: JSON.stringify(["capture", proof.draftId]), ownerGeneration: proof.ownerGeneration,
          draftId: proof.draftId, recordRevision: proof.recordRevision, inputRevision: proof.inputRevision,
          lastMutationId: proof.lastMutationId, recordDigest: proof.recordDigest,
          ...(proof.pendingOperationId ? { pendingOperationId: proof.pendingOperationId } : {}) }];
      },
      freeze: request => {
        try { this.assertReady(); } catch { return false; }
        if (!this.prepared || this.prepared.request !== key(request) || !freezeNativeApi()) return false;
        if (!mounted.guard.freeze(this.prepared.proof)) { thawNativeApi(); return false; }
        return true;
      },
      thaw: () => { this.closeGeneration++; this.closing = false; this.prepared = undefined; mounted.guard.thaw(); thawNativeApi(); },
    };
  }
  private assertReady(): void {
    if (this.retired || this.delivery || !nativeApiIdle()) throw new UiError(message("desktop.unsavedClose"));
    this.mounted.assertCloseReady();
  }
  assertHandoffReady(): void {
    this.assertReady();
    if(this.closing || this.prepared)throw new UiError(message("desktop.unsavedClose"));
  }
  open(original: CaptureResultIdentity): Promise<void> {
    if(this.retired || this.closing)return Promise.reject(new UiError(message("desktop.unsavedClose")));
    if (this.delivery) return Promise.reject(new UiError(messageForUnconfirmedDelivery()));
    const requestId = crypto.randomUUID();
    return new Promise<void>((resolve, reject) => {
      // This native route does not pass through api(). It must participate in
      // the same admission fence through the Library's actual adoption reply.
      const finish = beginNativeRequest();
      this.delivery = { requestId, resolve: () => { finish(); resolve(); }, reject: error => { finish(); reject(error); } };
      try { this.surface.post({ type: "capture-open", requestId, draftId: original.draftId, operationId: original.operationId }); }
      catch (error) { this.delivery = undefined; finish(); reject(error); }
    });
  }
  requestClose(): Promise<void> {
    // Let the actual initiating form action finish its finally/delivery before
    // the host starts the independent close barrier. No close permit is sent.
    setTimeout(() => { if(this.retired)return; try { this.surface.post({ type: "request-close" }); } catch (error) { this.mounted.form.showError(error); } }, 0);
    return Promise.resolve();
  }
  dispose(): void {
    if(this.delivery)throw new UiError(message("capture.pending"));
    this.retired = true; this.closeGeneration++; this.prepared = undefined; this.stop();
  }
}
const key = (request: CloseRequest): string => JSON.stringify(request);
const messageForUnconfirmedDelivery = () => message("capture.pending");
