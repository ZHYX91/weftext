import { localCaptureAuthorPort } from "./capture-author-outbox";
import { CaptureInputPersistence } from "./capture-input-persistence";
import type { CapturePersistencePort } from "./capture-controller";
import { CapturePersistence, type CaptureDraftRecord, type CaptureMutationPayload, type CaptureTransport } from "./capture-draft-state";
import type { PersonalInputRecovery, CaptureTransportRecord } from "./personal-input-recovery";

/** One complete authorized input mutation includes its own author proof.
 * The shared pure outbox never stores its Schema/candidate or protected raw. */
export class LocalCapturePersistence implements CapturePersistencePort<CaptureDraftRecord> {
  private readonly core: CapturePersistence;
  private readonly input: CaptureInputPersistence<CaptureDraftRecord>;
  constructor(request: CaptureTransport, storage: PersonalInputRecovery, audience: string, producerId: string, transportFailure?: (error: unknown) => void) {
    this.core = new CapturePersistence(request);
    this.input = new CaptureInputPersistence(storage, audience, producerId, localCaptureAuthorPort(request), command => this.core.mutate("input", command), transportFailure);
  }
  stageCurrent(draftId: string, input: CaptureMutationPayload["input"]["input"], producerRevision: number, baseRevision: number): void {
    this.input.stageCurrent(draftId, input, producerRevision, baseRevision);
  }
  async mutate<K extends keyof CaptureMutationPayload>(route: K, payload: CaptureMutationPayload[K], producerRevision?: number): Promise<CaptureDraftRecord> {
    if (route !== "input") return this.core.mutate(route, payload);
    return this.input.write(payload as CaptureMutationPayload["input"], producerRevision);
  }
  flush: CapturePersistencePort<CaptureDraftRecord>["flush"] = (...args) => this.core.flush(...args);
  recover(draftId: string) { return this.input.recover(draftId); }
  queryOriginal(proposal: CaptureTransportRecord) { return this.input.queryOriginal(proposal); }
  read(draftId: string) { return this.core.read(draftId); }
}
