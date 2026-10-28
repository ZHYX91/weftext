import { UiError, message } from "./ui-locale";
export type OrderedDraftInput = {
  sessionId: string; source: string; serial: number; inputId: string;
} & ({baseId: string; retainedBaseSource?: never} | {baseId?: never; retainedBaseSource: string});
export type OrderedDraftTransmission = Readonly<OrderedDraftInput & {inputRevision: number}>;
type Send = (wire: Omit<OrderedDraftInput, "serial"> & {inputRevision: number}) => Promise<{inputRevision: number}>;
function sameInput(left: OrderedDraftInput, right: OrderedDraftInput): boolean {
  return left.sessionId === right.sessionId && left.source === right.source && left.serial === right.serial
    && left.inputId === right.inputId && left.baseId === right.baseId && left.retainedBaseSource === right.retainedBaseSource;
}
/** Per-live-writer ordering. Only a real producer issues a transmission;
 * retirement retries its original identity. Durable authority remains in Core. */
export class OrderedDraft {
  private revision = 0;
  private chain: Promise<unknown> = Promise.resolve();
  private ack?: OrderedDraftTransmission;
  private latest?: OrderedDraftTransmission;
  private readonly owned = new WeakSet<OrderedDraftTransmission>();
  private readonly retries = new WeakMap<OrderedDraftTransmission, Promise<number>>();
  acknowledged(input: OrderedDraftInput): number | undefined {
    return this.ack && sameInput(this.ack,input) ? this.ack.inputRevision : undefined;
  }
  issued(input: OrderedDraftInput): OrderedDraftTransmission | undefined {
    return this.latest && sameInput(this.latest,input) ? this.latest : undefined;
  }
  persist(input: OrderedDraftInput, send: Send, onIssued?: (cut: OrderedDraftTransmission) => void): Promise<number> {
    const cut = Object.freeze({...input,inputRevision:++this.revision});
    this.owned.add(cut); this.latest = cut;
    onIssued?.(cut);
    return this.transmit(cut,send);
  }
  retry(cut: OrderedDraftTransmission, send: Send): Promise<number> {
    if (!this.owned.has(cut)) return Promise.reject(new UiError(message("hostedEditor.draftOrSessionChanged")));
    const pending = this.retries.get(cut);
    if (pending) return pending;
    // Queue after the original producer, including its UI delivery failure.
    // Requalify through the original owner's actual response boundary.
    const next = this.transmit(cut,send);
    this.retries.set(cut,next);
    void next.then(()=>this.retries.delete(cut),()=>this.retries.delete(cut));
    return next;
  }
  async settled(): Promise<void> {
    for (;;) { const selected = this.chain; await selected.catch(()=>undefined); if (selected === this.chain) return; }
  }
  private transmit(cut: OrderedDraftTransmission, send: Send): Promise<number> {
    const {serial: _serial,...wire} = cut;
    const next = this.chain.catch(()=>undefined).then(async()=>{
      const response = await send(wire);
      if (response.inputRevision !== cut.inputRevision) throw new UiError(message("hostedEditor.draftOrSessionChanged"));
      if (!this.ack || this.ack.inputRevision <= cut.inputRevision) this.ack = cut;
      return cut.inputRevision;
    });
    this.chain = next;
    return next;
  }
}
