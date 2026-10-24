import { UiError, message } from "./ui-locale";
export interface OrderedDraftInput {
  sessionId: string; source: string; serial: number; inputId: string;
  baseId?: string; retainedBaseSource?: string;
}
/** Per-live-writer transport ordering. Durable authority remains in Core. */
export class OrderedDraft {
  private revision = 0;
  private chain: Promise<unknown> = Promise.resolve();
  private ack?: OrderedDraftInput & { inputRevision: number };
  acknowledged(input: Pick<OrderedDraftInput, "sessionId" | "source" | "serial" | "inputId">): number | undefined {
    const ack = this.ack;
    return ack && ack.sessionId === input.sessionId && ack.source === input.source && ack.serial === input.serial && ack.inputId === input.inputId ? ack.inputRevision : undefined;
  }
  persist(input: OrderedDraftInput, send: (wire: Omit<OrderedDraftInput, "serial"> & { inputRevision: number }) => Promise<{ inputRevision: number }>): Promise<number> {
    const snapshot = { ...input }, inputRevision = ++this.revision;
    const { serial: _serial, ...wire } = snapshot;
    const next = this.chain.catch(() => undefined).then(async () => {
      const response = await send({ ...wire, inputRevision });
      if (response.inputRevision !== inputRevision) throw new UiError(message("hostedEditor.draftOrSessionChanged"));
      this.ack = { ...snapshot, inputRevision };
      return inputRevision;
    });
    this.chain = next;
    return next;
  }
}
