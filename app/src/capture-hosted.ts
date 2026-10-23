import type { DataSchema, NativeValue } from "./data-panel";
import { CaptureController, type CaptureMutableRecord, type CapturePersistencePort } from "./capture-controller";
import { CaptureInputPersistence } from "./capture-input-persistence";
import { captureFieldCandidate, captureInputForPersistence, captureUtf8Ready, emptyCaptureInput, type CaptureTextSlot, type CaptureText, type CaptureAuthorRawInput, type CaptureAuthorRecovery, type CaptureDraftRecord, type CaptureFlushReceipt, type CaptureInput, type CaptureMutationPayload, type CaptureRawInput } from "./capture-draft-state";
import type { PersonalInputRecovery, CaptureTransportRecord } from "./personal-input-recovery";
import type { StructureCatalog, StructureRequest, StructureReceipt } from "./hosted-structure-state";
import { parseStrictJson } from "./strict-json";
import { fenceCaptureProposal } from "./capture-private-disposition";

export interface HostedCaptureView {
  recovery: CaptureAuthorRecovery;
  active: { slot: string; textInputs:Record<CaptureTextSlot,CaptureText>; bodyMode:"text"|"markup"; typeIds: string[]; fields: CaptureInput["fields"]; templateId: string | null; formBasis: CaptureInput["formBasis"]; rawInputs: CaptureInput["rawInputs"]; detachedRawInputs: CaptureInput["detachedRawInputs"] } | null;
}
export interface HostedCaptureRecord extends CaptureMutableRecord { recovery: CaptureAuthorRecovery; active: HostedCaptureView["active"] }
export type HostedCaptureRequest = <T>(path: string, body?: unknown) => Promise<T>;
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const copyOnly = (raw: CaptureAuthorRawInput): CaptureRawInput => ({ fieldId: raw.fieldId, valuePath: structuredClone(raw.valuePath), controlRole: raw.controlRole, text: structuredClone(raw.text), binding: "copyOnly", author: { text: structuredClone(raw.text), provenance: structuredClone(raw.provenance) } });
function decode(view: HostedCaptureView, draftId: string): HostedCaptureRecord {
  const recovery = view?.recovery;
  if (!recovery || !["full","authorOnly"].includes(recovery.inputCut) || recovery.draftId !== draftId || !uuid.test(draftId) || !Number.isSafeInteger(recovery.revision) || recovery.revision < 1 || recovery.lastMutation.resultingRevision !== recovery.revision || !uuid.test(recovery.lastMutation.mutationId) || !/^[a-f0-9]{64}$/.test(recovery.lastMutation.fingerprint)) throw new Error("获权捕获回执不对应原草稿，未采用另一份记录。");
  const active = recovery.inputCut==="full" ? view.active : null;
  const input: CaptureInput = { ...emptyCaptureInput(), ...(active?.textInputs??{}), authorTexts:recovery.texts, authorBodyMode:recovery.bodyMode, bodyMode:active?.bodyMode??"text",
    templateId: active?.templateId ?? null, typeIds: active?.typeIds ?? [], fields: active?.fields ?? [], formBasis: active?.formBasis ?? null,
    rawInputs: active?.rawInputs ?? recovery.rawInputs.map(copyOnly),
    detachedRawInputs: active?.detachedRawInputs ?? recovery.detachedRawInputs.map(entry => ({ ...entry, input: copyOnly(entry.input) })),
  };
  return structuredClone({ version: 2, draftId, revision: recovery.revision, inputRevision: recovery.inputRevision, inputCut: recovery.inputCut, lastMutation: recovery.lastMutation, pending: recovery.pending, currentInput: input, recovery, active });
}
export class HostedCapturePersistence implements CapturePersistencePort<HostedCaptureRecord> {
  private input: CaptureInputPersistence<HostedCaptureRecord>;
  constructor(private readonly request: HostedCaptureRequest, storage: PersonalInputRecovery, audience: string, producerId: string, transportFailure?: (error: unknown) => void) {
    this.input = new CaptureInputPersistence(storage, audience, producerId, {
      write: async command => decode(await request("capture/draft/author-input", command), command.draftId),
      lookup: async command => { const lookup = await request<{ status: string; draft: HostedCaptureView }>("capture/draft/author-mutation", command); return { status: lookup.status, draft: decode(lookup.draft, command.draftId) }; },
      lookupInput:async command=>{const lookup=await request<{status:string;draft:HostedCaptureView}>("capture/draft/input-mutation",command);return{status:lookup.status,draft:decode(lookup.draft,command.draftId)};},
      recovery: record => record.recovery,
      fullInput:record=>record.active && record.inputCut==="full"?record.currentInput:undefined,
    }, command => this.command("input", command), transportFailure);
  }
  stageCurrent(draftId: string, input: CaptureInput, producerRevision: number, baseRevision: number): void { this.input.stageCurrent(draftId, input, producerRevision, baseRevision); }
  mutate<K extends keyof CaptureMutationPayload>(route: K, payload: CaptureMutationPayload[K], producerRevision?: number): Promise<HostedCaptureRecord> {
    if (route === "input") return this.input.write(payload as CaptureMutationPayload["input"], producerRevision);
    if (route === "create" || route === "target") return Promise.reject(new Error("获权捕获目标由独立批准槽位建立，未发送本地私有路径。"));
    return this.command(route, payload);
  }
  private async command(route: string, payload: CaptureMutationPayload[keyof CaptureMutationPayload]): Promise<HostedCaptureRecord> {
    const cut = structuredClone(payload), { draftId, mutationId, ...command } = cut;
    try {
      const record = decode(await this.request<HostedCaptureView>(`capture/draft/${route}`, cut), draftId);
      if (record.lastMutation.mutationId !== mutationId) throw new Error("捕获 mutation 回执编号不符。");
      return record;
    } catch (original) {
      const lookup = await this.request<{ status: string; draft: HostedCaptureView }>("capture/draft/mutation", { draftId, mutationId, command: { kind: route, ...command } });
      const record = decode(lookup.draft, draftId);
      if (lookup.status === "applied" && record.lastMutation.mutationId === mutationId) return record;
      throw original;
    }
  }
  async flush(draft: HostedCaptureRecord, serial: number, currentSerial: () => number): Promise<CaptureFlushReceipt> {
    const cut = structuredClone(draft);
    if (serial !== currentSerial() || cut.inputCut!=="full" || !cut.active) throw new Error("捕获输入在 flush 前已变化。");
    const receipt = await this.request<CaptureFlushReceipt>("capture/draft/flush", { draftId: cut.draftId, recordRevision: cut.revision, inputRevision: cut.inputRevision, lastMutationId: cut.lastMutation.mutationId });
    if (serial !== currentSerial() || receipt.inputCut!=="full" || receipt.draftId !== cut.draftId || receipt.recordRevision !== cut.revision || receipt.inputRevision !== cut.inputRevision || receipt.lastMutationId !== cut.lastMutation.mutationId || receipt.pendingOperationId !== (cut.pending?.operationId ?? null) || !/^[a-f0-9]{64}$/.test(receipt.recordDigest)) throw new Error("捕获 flush 不是当前原 cut 的真实 Core 回执。");
    return receipt;
  }
  recover(draftId: string) { return this.input.recover(draftId); }
  queryOriginal(proposal: CaptureTransportRecord) { return this.input.queryOriginal(proposal); }
  async read(draftId: string) { return decode(await this.request<HostedCaptureView>("capture/draft/read", { draftId }), draftId); }
}
export interface HostedCaptureOptions {
  deployment: string; actor: string; draftId?: string; slot?: string;
  recovery: PersonalInputRecovery; current(): boolean;
  changed?(state: ReturnType<HostedCaptureSession["snapshot"]>): void;
  transportFailure?(error: unknown): void;
}
export class HostedCaptureSession {
  readonly controller: CaptureController<HostedCaptureRecord>;
  private retained: CaptureTransportRecord[];
  private constructor(private request: HostedCaptureRequest, record: HostedCaptureRecord, private persistence: HostedCapturePersistence, producerId: string, retained: CaptureTransportRecord[], private options: HostedCaptureOptions) {
    this.controller = new CaptureController(persistence, record, () => options.changed?.(this.snapshot()), producerId, draft=>draft.active!==null); this.retained = retained;
  }
  get unconfirmed(): CaptureTransportRecord[] { return structuredClone(this.retained); }
  async discardUnconfirmed(proposal: CaptureTransportRecord): Promise<void> {
    await fenceCaptureProposal(proposal, { query: async () => this.persistence.queryOriginal(proposal), read: () => this.persistence.read(proposal.owner.draftId), currentRevision: () => this.controller.state.draft.revision, checkpoint: () => this.controller.checkpoint() });
    if (!this.options.current()) throw new Error("账号或授权在处置期间改变，未处置新的 producer。");
    await this.options.recovery.discardCapture(JSON.stringify([this.options.deployment, this.options.actor]), proposal);
    this.retained = this.retained.filter(entry => entry.id !== proposal.id || entry.sequence !== proposal.sequence);
  }
  snapshot() { return this.controller.state; }
  static async open(rawRequest: HostedCaptureRequest, options: HostedCaptureOptions): Promise<HostedCaptureSession> {
    let retired = false;
    const current = () => { if (!options.current()) retired = true; return !retired; };
    const request: HostedCaptureRequest = async <T>(path: string, body?: unknown): Promise<T> => { if (!current()) throw new Error("原获权捕获 producer 已退休。"); const value = await rawRequest<T>(path, body); if (!current()) throw new Error("账号或授权在捕获交付期间变化，未采用旧受保护值。"); return value; };
    const audience = JSON.stringify([options.deployment, options.actor]); await options.recovery.open(audience);
    let record: HostedCaptureRecord;
    if (options.draftId) record = decode(await request("capture/draft/read", { draftId: options.draftId }), options.draftId);
    else {
      if (!options.slot) throw new Error("先选择明确获权的现有创建槽位。");
      const draftId = crypto.randomUUID(), mutationId = crypto.randomUUID(), input = emptyCaptureInput(); input.bodyMode = "text";
      try { record = decode(await request("capture/draft/create", { draftId, mutationId, slot: options.slot, input }), draftId); }
      catch (original) {
        const lookup = await request<{ status: string; draft: HostedCaptureView }>("capture/draft/mutation", { draftId, mutationId, command: { kind: "create", slot: options.slot, input } });
        if (lookup.status !== "applied") throw original;
        record = decode(lookup.draft, draftId);
      }
    }
    const producerId = crypto.randomUUID(), persistence = new HostedCapturePersistence(request, options.recovery, audience, producerId, options.transportFailure);
    const retained = await persistence.recover(record.draftId);
    record=await persistence.read(record.draftId);
    return new HostedCaptureSession(request, record, persistence, producerId, retained, options);
  }
  private exact(fact: StructureReceipt, id: string): StructureReceipt {
    if (fact.operationId !== id || typeof fact.status !== "string") throw new Error("捕获结果不是原操作的事实，原号保留。"); return fact;
  }
  async confirmOriginal(): Promise<StructureReceipt | null> {
    const pending = this.controller.state.draft.pending; if (!pending) return null;
    let fact = this.exact(await this.request("capture/draft/operation", { draftId: this.controller.state.draft.draftId }), pending.operationId);
    if (fact.status === "unknown") fact = this.exact(await this.request("capture/draft/settle", { draftId: this.controller.state.draft.draftId }), pending.operationId);
    return fact;
  }
  /** Exact author request is captured before every await; Core owns its wire. */
  prepare(catalog: StructureCatalog): Promise<StructureReceipt> {
    if (this.unconfirmed.length) throw new Error("原 producer 的本人输入尚未处置，未建立替代捕获操作。");
    const state = this.controller.state, id = state.draft.draftId;
    if (state.draft.pending) return this.resume();
    if (!state.draft.active) throw new Error("当前只有本人恢复投影，原受保护表单未重新获权。");
    const slot = catalog.slots.find(item => item.resource === state.draft.active!.slot);
    if (!slot?.create || slot.status !== "ready") throw new Error("原批准创建槽位当前不可用。");
    const input = captureInputForPersistence(state.input);
    if (input.rawInputs.some(raw => raw.binding !== "exact") || input.fields.some(field => field.valueState.kind !== "valid")) throw new Error("取回字段尚未核对完整原值，原文字保留，未自动提交片段。");
    for (const raw of input.rawInputs) captureUtf8Ready(raw.text);
    const fields = Object.fromEntries(input.fields.map(field => [field.fieldId, captureFieldCandidate(field)]));
    const payload: StructureRequest = { resource: slot.resource, destination: slot.resource, operationId: crypto.randomUUID(), kind: input.templateId ? "template" : "create" };
    if (input.templateId) {
      const template = catalog.templates.find(item => item.id === input.templateId && item.status === "ready");
      if (!template) throw new Error("原模板未获当前独立资格，原输入保留。");
      payload.template = template.id; if (template.source) payload.source = template.source;
      const parameters: unknown = parseStrictJson(captureUtf8Ready(input.parametersText));
      if (!parameters || typeof parameters !== "object" || Array.isArray(parameters)) throw new Error("本人模板参数尚未形成完整对象。"); payload.parameters = parameters as Record<string, unknown>;
    } else payload.capture = { title: captureUtf8Ready(input.title) || undefined, body: captureUtf8Ready(input.body), bodyMode: input.bodyMode, types: input.typeIds, fields, ...(input.formBasis ? { formBasis: input.formBasis } : {}) };
    const pending = this.controller.freezeSubmission(payload as unknown as Record<string, unknown>, "hostedStructure");
    return pending.then(async original => {
      try { return this.exact(await this.request("capture/draft/submit", { draftId: id }), original.operationId); }
      catch (error) { const fact = await this.confirmOriginal(); if (fact?.status === "committed") return fact; if (fact && ["resolved", "cancelled"].includes(fact.status)) await this.controller.advance(original.operationId, false); throw error; }
    });
  }
  private async resume(): Promise<StructureReceipt> {
    const fact = await this.confirmOriginal(); if (!fact) throw new Error("原捕获操作尚未找到。");
    if (["resolved", "cancelled"].includes(fact.status)) { await this.controller.advance(fact.operationId, false); throw new Error("原操作已可靠终结；当前本人输入保留，可明确重试。"); }
    return fact;
  }
  async commitOriginal(fact: StructureReceipt, acceptLosses: boolean): Promise<StructureReceipt> {
    const pending = this.controller.state.draft.pending;
    if (!pending || fact.operationId !== pending.operationId || fact.status !== "prepared" || !fact.planId || !fact.commitAllowed) throw new Error("原捕获计划没有当前提交资格。");
    try { return this.exact(await this.request("capture/draft/commit", { draftId: this.controller.state.draft.draftId, planId: fact.planId, acceptLosses }), pending.operationId); }
    catch (error) { const current = await this.confirmOriginal(); if (current?.status === "committed") return current; throw error; }
  }
  async save(mode: "next" | "close" | "open", catalog: StructureCatalog, deliver: (resource: string) => Promise<void>, close: () => Promise<void>): Promise<StructureReceipt> {
    let fact = await this.prepare(catalog);
    if (fact.status === "prepared") {
      if (fact.preview?.losses?.length) throw new Error("原捕获计划有明确损失，先核对同一计划，未自动接受损失或重建请求。");
      fact = await this.commitOriginal(fact, false);
    }
    if (fact.status !== "committed") throw new Error(`原捕获结果尚未提交：${fact.status}`);
    if (mode === "open") {
      if (!fact.result?.canOpen || fact.result.status !== "available") throw new Error("捕获已提交；结果当前没有独立打开资格，原操作继续保留。");
      await deliver(fact.result.resource);
      await this.controller.advance(fact.operationId, true);
    } else if (mode === "next") await this.controller.advance(fact.operationId, true);
    else await close();
    return fact;
  }
}
