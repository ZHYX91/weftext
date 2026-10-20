import { Annotation, EditorState, StateField, Transaction, type Extension } from "@codemirror/state";
import { isolateHistory } from "@codemirror/commands";

export type SourceHistoryCommand = "undo" | "redo";
type CancelTask = () => void;
export interface SourceHistoryHost {
  /** Raw host composition state, excluding this queue's settlement window. */
  composing(): boolean;
  execute(command: SourceHistoryCommand): boolean;
  /** Release outside mode/save actions only after DOM settlement and history. */
  settled(): void;
  schedule?(run: () => void, delayMs: number): CancelTask;
}

/** Owns pending commands, never a second document or a second Undo stack. */
export class SourceHistoryRequests {
  private pending: { command: SourceHistoryCommand; epoch: number }[] = [];
  private active = false;
  private settling = false;
  private epoch = 0;
  private disposed = false;
  private cancelTask?: CancelTask;
  private retryDelay = 16;

  constructor(private host: SourceHistoryHost) {}

  get composing(): boolean { return this.active || this.settling; }

  request(command: SourceHistoryCommand, composingHint = false): boolean {
    if (this.disposed) return false;
    if (composingHint) this.compositionStart();
    if (this.composing || this.host.composing() || this.pending.length) {
      this.pending.push({ command, epoch: this.epoch });
      return true;
    }
    return this.host.execute(command);
  }

  compositionStart(): void {
    if (this.disposed) return;
    this.active = true;
    this.cancelTask?.();
    this.cancelTask = undefined;
    this.retryDelay = 16;
  }

  compositionEnd(): void {
    if (this.disposed) return;
    this.active = false;
    this.settling = true;
    this.cancelTask?.();
    this.retryDelay = 16;
    this.scheduleFlush(0);
  }

  private scheduleFlush(delayMs: number): void {
    const run = () => { this.cancelTask = undefined; this.flush(); };
    this.cancelTask = this.host.schedule ? this.host.schedule(run, delayMs) : (() => { const timer = setTimeout(run, delayMs); return () => clearTimeout(timer); })();
  }

  private waitingForHost(): boolean {
    if (!this.host.composing()) return false;
    // Some hosts clear their raw composition flag after the first task. Keep
    // observing that real end without committing on a timeout or busy looping.
    this.scheduleFlush(this.retryDelay);
    this.retryDelay = Math.min(this.retryDelay * 2, 250);
    return true;
  }

  /** A new source/session/authority retires commands without clearing history. */
  retire(): void { this.epoch++; this.pending = []; }

  dispose(): void {
    this.disposed = true;
    this.retire();
    this.cancelTask?.();
    this.cancelTask = undefined;
  }

  private flush(): void {
    if (this.disposed || this.active || this.waitingForHost()) return;
    try {
      while (this.pending.length) {
        if (this.active || this.waitingForHost()) return;
        const next = this.pending.shift()!;
        if (next.epoch === this.epoch) this.host.execute(next.command);
      }
    } catch (error) {
      this.retire();
      this.settling = false;
      this.host.settled();
      throw error;
    }
    this.settling = false;
    this.host.settled();
  }
}

/** Preserve one native composition unit even when CM reads its first DOM
 * mutation after compositionend and labels it compose without compose.start. */
export function sourceCompositionHistory(epoch: () => number | null, nativeEdit: (transaction: Transaction) => boolean): Extension {
  const owner = Annotation.define<number>();
  const seen = StateField.define<number | null>({ create: () => null, update: (value, transaction) => transaction.annotation(owner) ?? value });
  return [seen, EditorState.transactionFilter.of(transaction => {
    const current = epoch(), event = transaction.annotation(Transaction.userEvent);
    if (current === null || !transaction.docChanged || nativeEdit(transaction) || transaction.annotation(isolateHistory) || !(event === "input" || event === "input.type" || event?.startsWith("input.type.compose"))) return transaction;
    const first = transaction.startState.field(seen) !== current;
    const userEvent = first ? "input.type.compose.start" : "input.type.compose";
    if (!first && event === userEvent) return transaction;
    // Earlier annotations take precedence. Prefix the public transaction
    // instead of reading/replacing any private annotation or history fields.
    return [{ annotations: [owner.of(current), Transaction.userEvent.of(userEvent), ...(first ? [isolateHistory.of("before")] : [])] }, transaction];
  })];
}
