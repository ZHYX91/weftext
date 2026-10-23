import type { AuthorProducerSnapshot } from "./author-input";
import type { CaptureController, CaptureMutableRecord } from "./capture-controller";
import { CapturePersistenceConflict, type CaptureFlushReceipt } from "./capture-draft-state";
import type { PersonalControlInput } from "./personal-control-input";

export interface CaptureCloseView {
  /** Actual session recovery state, including another producer's unconfirmed
   * transport proposal. Durable Core pending effects remain recoverable. */
  assertResolved(): void;
  collectOwnedRaw(): void;
  freeze(): void;
  thaw(): void;
}
export type PreparedCaptureClose = CaptureFlushReceipt & { ownerGeneration: string; editRevision: number };

/** This guard prepares a real Core cut and freezes its actual input authority.
 * The native surface must additionally bind its own attempt/epoch/edit serial.
 * Merely constructing a guard never registers a ready native participant. */
export class CaptureCloseGuard<R extends CaptureMutableRecord> {
  private prepared?: { proof: PreparedCaptureClose; producers: AuthorProducerSnapshot };
  private frozen = false;
  constructor(private controller: CaptureController<R>, private inputs: PersonalControlInput, private view: CaptureCloseView) {}

  async prepare(): Promise<PreparedCaptureClose> {
    this.prepared = undefined;
    if (this.frozen) throw new CapturePersistenceConflict();
    this.view.assertResolved();
    if (this.inputs.inspect().producers.some(producer => producer.nativePending || producer.pendingDeliveries)) throw new CapturePersistenceConflict();
    const proof = await this.controller.prepareClose(() => this.view.collectOwnedRaw());
    this.view.assertResolved();
    if (proof.editRevision !== this.controller.editRevision) throw new CapturePersistenceConflict();
    const before = this.inputs.inspect();
    if (before.producers.some(producer => producer.nativePending || producer.pendingDeliveries)) throw new CapturePersistenceConflict();
    // Only producer revisions observed by this controller can be covered by its
    // complete private-input persistence and actual Core readback. IDB ACKs are
    // deliberately absent from this path.
    const cuts = new Map(this.controller.authorCheckpoints.map(cut => [cut.producerId, cut.revision]));
    for (const producer of before.producers) {
      const revision = cuts.get(producer.producerId);
      if (revision === undefined) continue;
      if (producer.owner.kind !== "capture" || producer.owner.draftId !== proof.draftId
        || revision > producer.revision || revision < producer.ackRevision
        || !this.inputs.acknowledge({producerId:producer.producerId,revision}, "applied")) throw new CapturePersistenceConflict();
    }
    const producers = this.inputs.inspect();
    if (producers.producers.some(producer => producer.unapplied || producer.nativePending || producer.pendingDeliveries)) throw new CapturePersistenceConflict();
    if (proof.editRevision !== this.controller.editRevision) throw new CapturePersistenceConflict();
    this.prepared = {proof,producers};
    return structuredClone(proof);
  }
  freeze(proof: PreparedCaptureClose): boolean {
    const prepared = this.prepared;
    try { this.view.assertResolved(); } catch { return false; }
    if (!prepared || this.frozen || JSON.stringify(prepared.proof) !== JSON.stringify(proof)
      || !this.controller.freezeClose(proof)) return false;
    if (!this.inputs.freeze(prepared.producers)) { this.controller.thawClose(); return false; }
    try { this.view.freeze(); }
    catch { this.inputs.thaw(); this.controller.thawClose(); return false; }
    this.frozen = true;
    return true;
  }
  thaw(): void {
    this.prepared = undefined;
    this.frozen = false;
    this.inputs.thaw();
    this.controller.thawClose();
    this.view.thaw();
  }
}
