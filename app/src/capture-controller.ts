import type { AuthorInputSession, AuthorInputSnapshot, ValuePath } from "./author-input";
import { CapturePersistenceConflict, captureInputForPersistence, captureInputsEqual, captureText, type CaptureDraftRecord, type CaptureFlushReceipt, type CaptureInput, type CaptureMutationPayload, type CaptureRawInput, type CaptureTargetSelection, type FrozenCaptureRequest } from "./capture-draft-state";
import { freezeOriginalAttempt, originalFence } from "./original-operation";

type Job = { kind: "input" | "barrier"; started: boolean; input?: CaptureInput; producerRevision?: number; run(): Promise<void> };
export interface CaptureMutableRecord {
  version: 2; draftId: string; revision: number; inputRevision: number; inputCut: CaptureDraftRecord["inputCut"];
  lastMutation: CaptureDraftRecord["lastMutation"];
  currentInput: CaptureInput;
  pending: Pick<FrozenCaptureRequest, "operationId" | "requestKind" | "inputRevision"> | null;
}
export interface CapturePersistencePort<R extends CaptureMutableRecord> {
  mutate<K extends keyof CaptureMutationPayload>(route: K, payload: CaptureMutationPayload[K], producerRevision?: number): Promise<R>;
  stageCurrent?(draftId: string, input: CaptureInput, producerRevision: number, baseRevision: number): void;
  flush(draft: R, surfaceEditRevision: number, currentSurfaceEditRevision: () => number): Promise<CaptureFlushReceipt>;
}
export interface CaptureControllerState<R extends CaptureMutableRecord = CaptureDraftRecord> {
  draft: R;
  input: CaptureInput;
  editRevision: number;
  persisting: boolean;
  formReady: boolean;
  blocked: boolean;
  error?: unknown;
}
const equalPath = (left: readonly ValuePath[], right: readonly ValuePath[]) => JSON.stringify(left) === JSON.stringify(right);
const withinPath = (child: readonly ValuePath[], parent: readonly ValuePath[]) => child.length >= parent.length && equalPath(child.slice(0, parent.length), parent);

/** One ordered writer per live producer. Current author input and the original
 * submission cut are separate; an unsuccessful mutation blocks the queue. */
export class CaptureController<R extends CaptureMutableRecord = CaptureDraftRecord> {
  readonly ownerGeneration: string;
  private draft: R;
  private input: CaptureInput;
  private serial = 0;
  private jobs: Job[] = [];
  private running = false;
  private failure?: { error: unknown };
  private waiters: { resolve(): void; reject(error: unknown): void }[] = [];
  private frozen = false;
  private retired = false;
  private composition = 0;
  private authorCuts = new Map<string, number>();
  constructor(private readonly persistence: CapturePersistencePort<R>, draft: R, private readonly changed: (state: CaptureControllerState<R>) => void = () => {}, ownerGeneration: string = crypto.randomUUID(), private readonly completeView: (draft:R)=>boolean=()=>true) {
    this.ownerGeneration = ownerGeneration;
    this.draft = structuredClone(draft);
    this.input = structuredClone(draft.currentInput);
  }
  get state(): CaptureControllerState<R> {
    return { draft: structuredClone(this.draft), input: structuredClone(this.input), editRevision: this.serial, persisting: this.running || this.jobs.length > 0, formReady: this.draft.inputCut==="full" && this.completeView(this.draft) && !this.running && this.jobs.length===0 && !this.failure, blocked: !!this.failure, error: this.failure?.error };
  }
  get editRevision(): number { return this.serial; }
  get composing(): boolean { return this.composition > 0; }
  get authorCheckpoints(): { producerId: string; revision: number }[] { return [...this.authorCuts].map(([producerId, revision]) => ({ producerId, revision })); }
  observeAuthorCut(snapshot: AuthorInputSnapshot): void {
    if (snapshot.context.owner.kind !== "capture" || snapshot.context.owner.draftId !== this.draft.draftId) throw new Error("捕获作者输入所有者不符。");
    this.authorCuts.set(snapshot.producerId, Math.max(snapshot.producerRevision, this.authorCuts.get(snapshot.producerId) ?? 0));
  }
  beginComposition(): void { this.assertMutable(); this.composition++; this.serial++; this.emit(); }
  endComposition(): void { if (this.composition > 0) this.composition--; this.emit(); }
  private assertMutable(): void { if (this.retired || this.frozen) throw new Error("捕获输入已退休或处于关闭冻结状态。"); }
  private emit(): void { this.changed(this.state); }
  /** A caller must supply its owned controls, never the current document. */
  edit(next: CaptureInput): void {
    this.assertMutable();
    this.input = structuredClone(next);
    this.serial++;
    this.queueInput(this.input);
    this.emit();
  }
  private queueInput(input: CaptureInput): void {
    this.persistence.stageCurrent?.(this.draft.draftId, input, this.serial, this.draft.revision);
    const last = this.jobs.at(-1);
    if (last?.kind === "input" && !last.started) { last.input = structuredClone(input); last.producerRevision = this.serial; return; }
    const job: Job = { kind: "input", started: false, input: structuredClone(input), producerRevision: this.serial, run: async () => {} };
    let payload: { draftId: string; mutationId: string; expectedRevision: number; input: CaptureInput } | undefined;
    job.run = async () => {
      // Allocate the command once, including its original expected revision.
      // Retry must not turn an unconfirmed command into a new mutation.
      payload ??= { draftId: this.draft.draftId, mutationId: crypto.randomUUID(), expectedRevision: this.draft.revision, input: captureInputForPersistence(job.input!) };
      this.draft = await this.persistence.mutate("input", payload, job.producerRevision);
    };
    this.jobs.push(job);
    this.start();
  }
  private barrier<T>(run: () => Promise<T>): Promise<T> {
    let result: T;
    let resolve!: (value: T) => void;
    const ready = new Promise<T>(done => { resolve = done; });
    this.jobs.push({ kind: "barrier", started: false, run: async () => { result = await run(); resolve(result); } });
    this.start();
    // A blocked queue rejects this observation, retaining the original job for
    // an explicit retry. The stored promise itself is never orphan-rejected.
    return Promise.race([ready, this.drain().then(() => result)]);
  }
  private start(): void {
    if (this.running || this.failure || !this.jobs.length) return;
    this.running = true;
    queueMicrotask(() => { void this.run(); });
  }
  private async run(): Promise<void> {
    while (this.jobs.length && !this.failure) {
      const job = this.jobs[0];
      job.started = true;
      try { await job.run(); this.jobs.shift(); }
      catch (error) { this.failure = { error }; }
      this.emit();
    }
    this.running = false;
    const waiters = this.waiters.splice(0);
    for (const waiter of waiters) this.failure ? waiter.reject(this.failure.error) : waiter.resolve();
    this.emit();
  }
  async drain(): Promise<void> {
    if (this.failure) throw this.failure.error;
    if (!this.running && !this.jobs.length) return;
    await new Promise<void>((resolve, reject) => { this.waiters.push({ resolve, reject }); });
  }
  /** Retry the identical original mutation; never discard the latest raw cut. */
  retryPersistence(): Promise<void> { this.failure = undefined; this.start(); this.emit(); return this.drain(); }
  selectTarget(target: CaptureTargetSelection): Promise<R> {
    this.assertMutable();
    const selected = structuredClone(target);
    let payload: { draftId: string; mutationId: string; expectedRevision: number; target: CaptureTargetSelection } | undefined;
    return this.barrier(async () => {
      payload ??= { draftId: this.draft.draftId, mutationId: crypto.randomUUID(), expectedRevision: this.draft.revision, target: selected };
      return this.draft = await this.persistence.mutate("target", payload);
    });
  }
  /** This method is deliberately not async: the wire and current cut are
   * captured synchronously before hashing, persistence or any other await. */
  freezeSubmission(payload: Record<string, unknown>, requestKind: FrozenCaptureRequest["requestKind"]): Promise<FrozenCaptureRequest> {
    this.assertMutable();
    if (this.composing) throw new Error("输入法组合尚未结束，原文保留。");
    if(this.draft.inputCut!=="full" || !this.completeView(this.draft))throw new Error("当前只有本人恢复检查点，未获完整表单 ACK，不能提交。");
    if (this.draft.pending || this.jobs.some(job => job.kind === "barrier")) throw new Error("先确认原操作和目标选择，不能另起提交。");
    const attempt = freezeOriginalAttempt(payload, requestKind);
    const cut = structuredClone(this.input);
    this.queueInput(cut);
    let frozen: FrozenCaptureRequest | undefined;
    let command: { draftId: string; mutationId: string; expectedRevision: number; request: FrozenCaptureRequest } | undefined;
    return this.barrier(async () => {
      if (!frozen) {
        const fence = await originalFence(attempt);
        frozen = { ...attempt, requestKind, wireDigest: fence.wireDigest, inputRevision: this.draft.inputRevision };
      }
      command ??= { draftId: this.draft.draftId, mutationId: crypto.randomUUID(), expectedRevision: this.draft.revision, request: frozen };
      this.draft = await this.persistence.mutate("freeze", command);
      return structuredClone(frozen);
    });
  }
  advance(operationId: string, resetInput: boolean): Promise<R> {
    this.assertMutable();
    const observedSerial = this.serial;
    let payload: { draftId: string; mutationId: string; expectedRevision: number; operationId: string; resetInput: boolean } | undefined;
    return this.barrier(async () => {
      payload ??= { draftId: this.draft.draftId, mutationId: crypto.randomUUID(), expectedRevision: this.draft.revision, operationId, resetInput };
      this.draft = await this.persistence.mutate("advance", payload);
      if (this.serial === observedSerial) this.input = structuredClone(this.draft.currentInput);
      return structuredClone(this.draft);
    });
  }
  checkpoint(): Promise<R> {
    this.assertMutable();
    let command: CaptureMutationPayload["checkpoint"] | undefined;
    return this.barrier(async () => {
      command ??= { draftId: this.draft.draftId, expectedRevision: this.draft.revision, mutationId: crypto.randomUUID() };
      return this.draft = await this.persistence.mutate("checkpoint", command);
    });
  }
  /** Bind all text producers to this exact draft and explicit Core sink. */
  authorSnapshot(snapshot: AuthorInputSnapshot, fieldId: string, controlRole: string): void {
    this.observeAuthorCut(snapshot);
    const next = structuredClone(this.input);
    const raw: CaptureRawInput = { fieldId, valuePath: structuredClone([...snapshot.context.path]), controlRole, text: captureText(snapshot.rawValue), binding: "exact", author: { text: captureText(snapshot.text), provenance: { producerId: snapshot.producerId, inputId: snapshot.id, revision: snapshot.revision } } };
    const index = next.rawInputs.findIndex(entry => entry.fieldId === fieldId && entry.controlRole === controlRole && equalPath(entry.valuePath, raw.valuePath));
    if (index < 0) next.rawInputs.push(raw); else {
      const old=next.rawInputs[index];
      if(old.author && (old.author.provenance.producerId!==snapshot.producerId || old.author.provenance.inputId!==snapshot.id)) next.detachedRawInputs.push({retirementId:crypto.randomUUID(),reason:"producer",input:old});
      next.rawInputs[index] = raw;
    }
    this.edit(next);
  }
  /** The shared form invalidates before replacing a branch/list subtree. Keep
   * the old raw unbound; never infer that an identical index is its old owner. */
  retireRaw(fieldId: string, path: readonly ValuePath[], reason: "structure" | "schema"): void {
    const next = structuredClone(this.input);
    const retired = next.rawInputs.filter(entry => entry.fieldId === fieldId && withinPath(entry.valuePath, path));
    if (!retired.length) return;
    next.rawInputs = next.rawInputs.filter(entry => !retired.includes(entry));
    next.detachedRawInputs.push(...retired.map(input => ({ retirementId: crypto.randomUUID(), reason, input })));
    this.edit(next);
  }
  wrapSession(session: AuthorInputSession, fieldId: string): AuthorInputSession {
    return { bind: (...args) => session.bind(...args), bindChoice: (...args) => session.bindChoice(...args), recordAction: (...args) => session.recordAction(...args), capture: control => session.capture(control), inspect: () => session.inspect(), acknowledge: (...args) => session.acknowledge(...args), freeze: checkpoint => session.freeze(checkpoint), thaw: () => session.thaw(), retire: () => session.retire(), invalidate: (slot, path = []) => { this.retireRaw(fieldId, path, "structure"); session.invalidate(slot, path); } };
  }
  /** The participant must first synchronously collect every live owned control.
   * The native outer barrier binds this receipt to its surface/epoch/attempt. */
  async prepareClose(collectOwnedRaw: () => void): Promise<CaptureFlushReceipt & { ownerGeneration: string; editRevision: number }> {
    this.assertMutable();
    if (this.composing) throw new CapturePersistenceConflict();
    collectOwnedRaw();
    const serial = this.serial;
    await this.drain();
    if (this.draft.inputCut!=="full" || !this.completeView(this.draft) || this.composing || this.serial !== serial || !captureInputsEqual(captureInputForPersistence(this.input), this.draft.currentInput)) throw new CapturePersistenceConflict();
    const receipt = await this.persistence.flush(this.draft, serial, () => this.serial);
    return { ...receipt, ownerGeneration: this.ownerGeneration, editRevision: serial };
  }
  freezeClose(proof: { ownerGeneration: string; editRevision: number }): boolean {
    if (this.draft.inputCut!=="full" || !this.completeView(this.draft) || this.retired || this.frozen || this.composing || this.running || this.jobs.length || this.failure || proof.ownerGeneration !== this.ownerGeneration || proof.editRevision !== this.serial) return false;
    this.frozen = true;
    return true;
  }
  thawClose(): void { this.frozen = false; }
  retire(): void { this.retired = true; this.frozen = true; }
}
