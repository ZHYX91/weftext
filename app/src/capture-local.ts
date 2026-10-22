import { CaptureController, type CaptureControllerState } from "./capture-controller";
import { CapturePersistence, captureFieldCandidate, captureInputForPersistence, captureUtf8Ready, emptyCaptureInput, type CaptureDraftRecord, type CaptureInput, type CaptureTargetSelection, type CaptureTransport } from "./capture-draft-state";
import { LocalCapturePersistence } from "./capture-local-persistence";
import { PersonalInputRecovery, type CaptureTransportRecord } from "./personal-input-recovery";
import { fenceCaptureProposal } from "./capture-private-disposition";
import type { OriginalFact } from "./original-operation";

export interface CaptureCreationFact extends OriginalFact {}
export interface CaptureResultIdentity { readonly draftId: string; readonly operationId: string }
interface PointerStorage { getItem(key: string): string | null; setItem(key: string, value: string): void }
export interface LocalCaptureOptions {
  draftId?: string;
  target?: CaptureTargetSelection;
  pointerStorage?: PointerStorage;
  recovery?: PersonalInputRecovery;
  transportFailure?(error: unknown): void;
  changed?(state: CaptureControllerState): void;
}
const pointerKey = "weftext:capture:active-draft";
const committed = (fact: CaptureCreationFact) => fact.status === "committed";

/** All author effects are dispatched from Core's durable frozen request. The
 * browser preference retains only a draft locator, never author input/wire. */
export class LocalCaptureSession {
  readonly controller: CaptureController;
  private retained: CaptureTransportRecord[] = [];
  private constructor(private readonly request: CaptureTransport, record: CaptureDraftRecord, private readonly recovery: PersonalInputRecovery, private readonly audience: string, private readonly persistence: LocalCapturePersistence, producerId: string, changed?: LocalCaptureOptions["changed"]) {
    this.controller = new CaptureController(persistence, record, changed, producerId);
  }
  static async open(request: CaptureTransport, options: LocalCaptureOptions = {}): Promise<LocalCaptureSession> {
    const persistence = new CapturePersistence(request);
    let record: CaptureDraftRecord;
    if (options.draftId) record = await persistence.read(options.draftId);
    else {
      // A failed enumeration must not be mistaken for no previous draft.
      const records = await request<CaptureDraftRecord[]>("/api/capture/draft/list", {});
      let preferred: string | null = null;
      try { preferred = options.pointerStorage?.getItem(pointerKey) ?? null; } catch { /* Locator preference is not author durability. */ }
      const selected = records.find(record => record.draftId === preferred && (!options.target || record.target?.rootHint === options.target.root))
        ?? (records.length === 1 && (!options.target || records[0].target?.rootHint === options.target.root) ? records[0] : undefined);
      if (selected) record = await persistence.read(selected.draftId);
      else {
        const input = emptyCaptureInput(); input.bodyMode = "text";
        record = await persistence.mutate("create", { draftId: crypto.randomUUID(), mutationId: crypto.randomUUID(), input, target: options.target });
      }
    }
    try { options.pointerStorage?.setItem(pointerKey, record.draftId); } catch { /* Core owns every actual input cut. */ }
    return this.install(request, record, options);
  }
  private static async install(request: CaptureTransport, record: CaptureDraftRecord, options: LocalCaptureOptions): Promise<LocalCaptureSession> {
    const device = await request<{ id: string; isCurrent: boolean }>("/api/capture/identity", {});
    if (!device?.isCurrent || !/^[a-f0-9-]{36}$/.test(device.id)) throw new Error("Core 设备身份尚未可靠限定，未借用另一份本人恢复域。");
    const audience = JSON.stringify(["local-device", device.id]), recovery = options.recovery ?? new PersonalInputRecovery();
    try { await recovery.open(audience); } catch (error) { options.transportFailure?.(error); }
    const producerId = crypto.randomUUID(), persistence = new LocalCapturePersistence(request, recovery, audience, producerId, options.transportFailure);
    const retained=await persistence.recover(record.draftId);
    record=await persistence.read(record.draftId);
    // No full durable cut exists after an author-only checkpoint. Keep exact
    // own fragments copy-only, rather than prefill the former private form.
    if(record.inputCut==="authorOnly") {
      const privateInput=record.currentInput;
      const own=(raw:typeof privateInput.rawInputs[number])=>raw.author?[{...structuredClone(raw),text:structuredClone(raw.author.text),binding:"copyOnly" as const}]:[];
      record.currentInput={...emptyCaptureInput(),authorTexts:structuredClone(privateInput.authorTexts),authorBodyMode:privateInput.authorBodyMode,rawInputs:privateInput.rawInputs.flatMap(own),detachedRawInputs:privateInput.detachedRawInputs.flatMap(entry=>own(entry.input).map(input=>({...entry,input})))};
    }
    const session = new LocalCaptureSession(request, record, recovery, audience, persistence, producerId, options.changed);
    session.retained = retained;
    return session;
  }
  get unconfirmed(): CaptureTransportRecord[] { return structuredClone(this.retained); }
  async discardUnconfirmed(proposal: CaptureTransportRecord): Promise<void> {
    // Read-only original lookup precedes any private CAS fence. Never infer
    // non-application from a request/transport error or a superseded marker.
    await fenceCaptureProposal(proposal, { query: async () => this.persistence.queryOriginal(proposal), read: () => this.persistence.read(proposal.owner.draftId), currentRevision: () => this.controller.state.draft.revision, checkpoint: () => this.controller.checkpoint() });
    await this.recovery.discardCapture(this.audience, proposal);
    this.retained = this.retained.filter(entry => entry.id !== proposal.id || entry.sequence !== proposal.sequence);
  }
  assertRecovered(): void { if (this.retained.length) throw new Error("另一个捕获 producer 的本人原输入尚未核定；先取回或明确处置，未覆盖它。"); }
  async selectTarget(target: CaptureTargetSelection): Promise<void> {
    const current = this.controller.state.draft;
    if (current.pending) throw new Error("原操作仍保留；先按原编号确认后再更换目标。");
    if (current.target && current.target.rootHint !== target.root) throw new Error("另一资料库需要独立捕获稿，当前恢复稿保留。");
    await this.controller.selectTarget(target);
  }
  /** Returns the original fact. No open/render failure releases its operationId. */
  async confirmOriginal(): Promise<CaptureCreationFact | null> {
    const pending = this.controller.state.draft.pending;
    if (!pending) return null;
    let fact = await this.request<CaptureCreationFact>("/api/operations/status", { operationId: pending.operationId });
    this.exact(fact, pending.operationId);
    if (fact.status === "unknown") {
      fact = await this.request<CaptureCreationFact>("/api/operations/settle", { operationId: pending.operationId, requestKind: pending.requestKind, wireDigest: pending.wireDigest });
      this.exact(fact, pending.operationId);
    }
    return fact;
  }
  private exact(fact: CaptureCreationFact, operationId: string): void {
    if (!fact || fact.operationId !== operationId || typeof fact.status !== "string") throw new Error("回执不对应原捕获操作，原文与原编号保留。");
  }
  /** Freeze author input and JSON synchronously before the first await. */
  submit(): Promise<CaptureCreationFact> {
    this.assertRecovered();
    const state = this.controller.state;
    if (state.draft.pending) return this.resumeOriginal();
    if(state.draft.inputCut!=="full")throw new Error("当前只有本人恢复检查点，尚未形成完整获权表单。 ");
    const target = state.draft.target;
    if (!target) throw new Error("先选择现有资料库；捕获输入保留。 ");
    const cut = state.input;
    if (cut.rawInputs.some(raw => raw.binding !== "exact")) throw new Error("取回的字段文字尚未核对原基线；未把本人片段当作完整字段自动提交。");
    const ready = captureInputForPersistence(cut);
    for (const raw of ready.rawInputs) captureUtf8Ready(raw.text);
    if (ready.fields.some(field => field.valueState.kind !== "valid")) throw new Error("字段仍有未完成原文，恢复稿保留，请完成后再创建。");
    const payload = {
      operationId: crypto.randomUUID(), root: target.rootHint,
      parent: target.parent.kind === "libraryRoot" ? target.rootHint : target.parent.pathHint,
      name: captureUtf8Ready(cut.name), title: captureUtf8Ready(cut.title) || null,
      types: [...cut.typeIds], fields: Object.fromEntries(ready.fields.map(field => [field.fieldId, captureFieldCandidate(field)])),
      body: captureUtf8Ready(cut.body), bodyMode: cut.bodyMode, formBasis: cut.formBasis,
    };
    const pending = this.controller.freezeSubmission(payload, "capture");
    return pending.then(async original => {
      try {
        const fact = await this.request<CaptureCreationFact>("/api/capture/draft/submit", { draftId: state.draft.draftId });
        this.exact(fact, original.operationId);
        return fact;
      } catch (error) {
        const fact = await this.confirmOriginal();
        if (fact && committed(fact)) return fact;
        if (fact?.status === "resolved") await this.controller.advance(original.operationId, false);
        throw error;
      }
    });
  }
  private async resumeOriginal(): Promise<CaptureCreationFact> {
    const pending = this.controller.state.draft.pending!;
    const fact = await this.confirmOriginal();
    if (!fact) throw new Error("原操作无法确认，捕获输入保留。");
    if (fact.status === "resolved") {
      await this.controller.advance(pending.operationId, false);
      throw new Error("原操作已可靠终结，未产生新的记录；当前输入保留，可再次保存。");
    }
    if (fact.status === "prepared") {
      const retried = await this.request<CaptureCreationFact>("/api/capture/draft/submit", { draftId: this.controller.state.draft.draftId });
      this.exact(retried, pending.operationId);
      return retried;
    }
    return fact;
  }
  async save(mode: "next" | "close" | "open", deliver: (original: CaptureResultIdentity) => Promise<void>, close: () => Promise<void>): Promise<CaptureCreationFact> {
    const fact = await this.submit();
    if (!committed(fact)) throw new Error(`原捕获操作尚未提交：${fact.status}`);
    if (mode === "open") {
      // Delivery reads only an already-committed original. It cannot prepare or
      // execute a creation, and the draft retains pending until UI acceptance.
      const original = this.controller.state.draft.pending;
      if (!original || original.operationId !== fact.operationId) throw new Error("原捕获结果与待交付编号不一致，未创建替代操作。");
      await deliver(Object.freeze({draftId:this.controller.state.draft.draftId,operationId:original.operationId}));
      await this.controller.advance(fact.operationId!, true);
    } else if (mode === "next") await this.controller.advance(fact.operationId!, true);
    else await close();
    return fact;
  }
  /** Create another library's owner without copying an old capability/candidate.
   * The old Core draft remains recoverable under its original identity. */
  async anotherTarget(target: CaptureTargetSelection, changed?: LocalCaptureOptions["changed"]): Promise<LocalCaptureSession> {
    if (this.controller.state.draft.pending) throw new Error("原操作尚未核定，未把其输入复制到另一资料库提交。");
    await this.controller.drain();
    const current = this.controller.state.input;
    const pureRaw=current.rawInputs.flatMap(raw=>raw.author ? [{...raw,text:structuredClone(raw.author.text),binding:"copyOnly" as const}] : []);
    const retired=current.detachedRawInputs.flatMap(entry=>entry.input.author ? [{...entry,input:{...entry.input,text:structuredClone(entry.input.author.text),binding:"copyOnly" as const}}] : []);
    const input: CaptureInput = { ...emptyCaptureInput(), authorTexts:structuredClone(current.authorTexts), authorBodyMode:current.authorBodyMode, detachedRawInputs: [...retired, ...pureRaw.map(raw => ({ retirementId: crypto.randomUUID(), reason: "schema" as const, input: raw }))] };
    const persistence = new CapturePersistence(this.request);
    const record = await persistence.mutate("create", { draftId: crypto.randomUUID(), mutationId: crypto.randomUUID(), input, target });
    return LocalCaptureSession.install(this.request, record, { changed, recovery: this.recovery });
  }
}
