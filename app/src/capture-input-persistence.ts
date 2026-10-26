import { CaptureAuthorOutbox, type CaptureAuthorPort } from "./capture-author-outbox";
import { captureAuthorInput, type CaptureInput, type CaptureMutationPayload } from "./capture-draft-state";
import type { PersonalInputRecovery, CaptureTransportRecord } from "./personal-input-recovery";

/** A normal authorized edit is one atomic full-input Core mutation containing
 * its own author proof. Browser recovery retains pure input and original
 * command identity only; it never performs an author-only fallback on failure. */
export class CaptureInputPersistence<R> {
  private readonly staged = new Set<Promise<void>>();
  private readonly author: CaptureAuthorOutbox<R>;
  constructor(storage: PersonalInputRecovery, audience: string, producerId: string, authorPort: CaptureAuthorPort<R>, private readonly writeFull: (command: CaptureMutationPayload["input"]) => Promise<R>, private readonly transportFailure: (error: unknown) => void = () => {}) {
    this.author = new CaptureAuthorOutbox(storage, audience, producerId, authorPort, transportFailure);
  }
  stageCurrent(draftId: string, input: CaptureInput, producerRevision: number, baseRevision: number): void {
    const staged = this.author.stageCurrent(draftId, baseRevision, producerRevision, captureAuthorInput(input)).then(() => {}, this.transportFailure);
    this.staged.add(staged);
    void staged.then(() => this.staged.delete(staged), () => this.staged.delete(staged));
  }
  write(original: CaptureMutationPayload["input"], producerRevision?: number): Promise<R> {
    if (producerRevision === undefined) throw new Error("捕获命令没有实际 producer revision。");
    return this.author.persistFull(original,producerRevision,this.writeFull);
  }
  async settleStaging(): Promise<void> {
    while (this.staged.size) await Promise.all([...this.staged]);
  }
  recover(draftId: string) { return this.author.recover(draftId); }
  queryOriginal(proposal: CaptureTransportRecord) { return this.author.queryOriginal(proposal); }
}
