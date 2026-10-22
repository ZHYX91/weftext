import { captureAuthorFingerprint, captureAuthorInput, captureInputFingerprint, captureInputsEqual, captureString, captureText, captureTextSlots, emptyCaptureAuthorTexts, type CaptureAuthorInput, type CaptureAuthorRecovery, type CaptureDraftRecord, type CaptureMutationPayload } from "./capture-draft-state";
import { PersonalInputRecovery, type CaptureTransportInput, type CaptureTransportRecord } from "./personal-input-recovery";

export interface CaptureAuthorCommand { draftId: string; expectedRevision: number; mutationId: string; input: CaptureAuthorInput }
export interface CaptureAuthorPort<R> {
  write(command: CaptureAuthorCommand): Promise<R>;
  lookup(command: CaptureAuthorCommand): Promise<{ status: string; draft: R }>;
  lookupInput(command: {draftId:string;expectedRevision:number;mutationId:string;fingerprint:string}): Promise<{ status:string;draft:R }>;
  recovery(record: R): Pick<CaptureAuthorRecovery, "draftId" | "revision" | "inputCut" | "lastMutation">;
  fullInput(record: R): CaptureDraftRecord["currentInput"] | undefined;
}
function transport(input: CaptureAuthorInput): CaptureTransportInput {
  const raw = (entry: CaptureAuthorInput["rawInputs"][number]) => ({ ...structuredClone(entry), text: captureString(entry.text) });
  const texts = Object.fromEntries(captureTextSlots.map(slot=>[slot,input.texts[slot].map(own=>({text:captureString(own.text),provenance:structuredClone(own.provenance)}))])) as CaptureTransportInput["texts"];
  return { texts, bodyMode: input.bodyMode,
    rawInputs: input.rawInputs.map(raw), detachedRawInputs: input.detachedRawInputs.map(entry => ({ ...entry, input: raw(entry.input) })),
  };
}
function author(input: CaptureTransportInput): CaptureAuthorInput {
  const raw = (entry: CaptureTransportInput["rawInputs"][number]) => ({ fieldId: entry.fieldId,
    valuePath: entry.valuePath.map(part => part.kind === "member" ? { kind: "member" as const, key: part.key } : { kind: "index" as const, index: part.index }),
    controlRole: entry.controlRole, text: captureText(entry.text), provenance: { producerId: entry.provenance.producerId, inputId: entry.provenance.inputId, revision: entry.provenance.revision } });
  const texts = emptyCaptureAuthorTexts();
  for(const slot of captureTextSlots) texts[slot]=input.texts[slot].map(own=>({text:captureText(own.text),provenance:structuredClone(own.provenance)}));
  return { texts, bodyMode: input.bodyMode,
    rawInputs: input.rawInputs.map(raw), detachedRawInputs: input.detachedRawInputs.map(entry => ({ retirementId: entry.retirementId, reason: entry.reason, input: raw(entry.input) })),
  };
}
/** One transport row in the shared pure-input store, removed only by the exact
 * real Core mutation proof. No author effect or duplicate receipt is stored. */
export class CaptureAuthorOutbox<R> {
  constructor(private readonly storage: PersonalInputRecovery, private readonly audience: string, private readonly producerId: string, private readonly port: CaptureAuthorPort<R>, private readonly transportFailure: (error: unknown) => void = () => {}) {}
  stageCurrent(draftId: string, baseRevision: number, producerRevision: number, input: CaptureAuthorInput): Promise<unknown> {
    return this.storage.stageCaptureCurrent(this.audience, { kind: "captureTransport", id: `capture:${draftId}:${this.producerId}`, owner: { kind: "capture", draftId }, producerId: this.producerId, producerRevision, baseRevision, input: transport(structuredClone(input)) });
  }
  async persist(command: CaptureAuthorCommand, producerRevision: number): Promise<R> {
    const cut=structuredClone(command), fingerprint=await captureAuthorFingerprint(cut.expectedRevision,cut.input);
    return this.persistCut(cut,producerRevision,"authorInput",fingerprint,()=>this.port.write(cut),()=>this.port.lookup(cut));
  }
  /** The complete candidate stays in this live attempt/Core only. The one
   * browser transport row retains pure proof plus its original digest. */
  async persistFull(command: CaptureMutationPayload["input"], producerRevision:number, write:(command:CaptureMutationPayload["input"])=>Promise<R>):Promise<R> {
    const cut=structuredClone(command), fingerprint=await captureInputFingerprint(cut.expectedRevision,cut.input);
    const identity={draftId:cut.draftId,expectedRevision:cut.expectedRevision,mutationId:cut.mutationId,fingerprint};
    return this.persistCut({...identity,input:captureAuthorInput(cut.input)},producerRevision,"fullInput",fingerprint,()=>write(cut),()=>this.port.lookupInput(identity),result=>{
      const actual=this.port.fullInput(result);
      if(!actual || !captureInputsEqual(actual,cut.input))throw new Error("完整捕获回执没有同一原表单，未把纯本人投影当作完整 ACK。");
    });
  }
  private async persistCut(command:CaptureAuthorCommand,producerRevision:number,kind:"fullInput"|"authorInput",fingerprint:string,write:()=>Promise<R>,lookup:()=>Promise<{status:string;draft:R}>,validate:(result:R)=>void=()=>{}):Promise<R> {
    const id=`capture:${command.draftId}:${this.producerId}`;
    if(!this.storage.captureTransports(this.audience,command.draftId).some(entry=>entry.id===id)) {
      try {await this.stageCurrent(command.draftId,command.expectedRevision,producerRevision,command.input);}catch(error){this.transportFailure(error);}
    }
    const previous=this.storage.captureTransports(this.audience,command.draftId).find(entry=>entry.id===id)?.pending;
    const pending={producerRevision,command:{kind,expectedRevision:command.expectedRevision,mutationId:command.mutationId,fingerprint},input:transport(command.input)};
    // This synchronous freeze refuses a different outstanding original. An IO
    // failure leaves the same RAM cut; Core remains the persistence authority.
    const staged=this.storage.freezeCaptureMutation(this.audience,id,pending);
    try {await staged;}catch(error){this.transportFailure(error);}
    const proposal=this.storage.captureTransports(this.audience,command.draftId).find(entry=>entry.id===id)!;
    let result:R;
    if(previous) {
      const known=await lookup();
      if(known.status==="applied") {result=known.draft;validate(result);this.check(proposal,result);try{await this.acknowledge(proposal,result);}catch(error){this.transportFailure(error);}return result;}
      if(known.status==="differentCommand")throw new Error("原捕获编号对应另一命令，未重放或换号。");
    }
    try {result=await write();}
    catch(original) {const known=await lookup();if(known.status!=="applied")throw original;result=known.draft;}
    validate(result);this.check(proposal,result);
    try {await this.acknowledge(proposal,result);}catch(error){this.transportFailure(error);}
    return result;
  }
  /** Read-only original lookup first. Superseded/unknown rows remain pure,
   * copy-only proposals; they are never replayed onto a new Schema or writer. */
  async recover(draftId: string): Promise<CaptureTransportRecord[]> {
    const retained: CaptureTransportRecord[] = [];
    for (const proposal of this.storage.captureTransports(this.audience, draftId)) {
      if (!proposal.pending) { retained.push(proposal); continue; }
      const pending = proposal.pending;
      if(pending.command.kind==="authorInput" && await captureAuthorFingerprint(pending.command.expectedRevision,author(pending.input))!==pending.command.fingerprint)throw new Error("恢复 transport 指纹与本人原输入不一致，原件保留。");
      try {
        const lookup = await this.queryOriginal(proposal)!;
        if (lookup.status === "applied") { await this.acknowledge(proposal, lookup.draft); const current = this.storage.captureTransports(this.audience, draftId).find(entry => entry.id === proposal.id); if (current) retained.push(current); continue; }
      } catch { /* Authentication, resource or read failures grant no recovery permission. */ }
      retained.push(proposal);
    }
    return retained;
  }
  queryOriginal(proposal: CaptureTransportRecord): Promise<{ status: string; draft: R }> | undefined {
    if (!proposal.pending) return undefined;
    const pending = proposal.pending;
    const identity={draftId:proposal.owner.draftId,expectedRevision:pending.command.expectedRevision,mutationId:pending.command.mutationId};
    return pending.command.kind==="fullInput" ? this.port.lookupInput({...identity,fingerprint:pending.command.fingerprint}) : this.port.lookup({...identity,input:author(pending.input)});
  }
  private acknowledge(proposal: CaptureTransportRecord, result: R): Promise<unknown> {
    this.check(proposal, result);
    const proof = this.port.recovery(result);
    return this.storage.acknowledgeCapture(this.audience, proposal, { draftId: proof.draftId, mutationId: proof.lastMutation.mutationId, fingerprint: proof.lastMutation.fingerprint, recordRevision: proof.revision });
  }
  private check(proposal: CaptureTransportRecord, result: R): void {
    const proof = this.port.recovery(result);
    const command = proposal.pending?.command;
    if (!command || proof.lastMutation.kind!==(command.kind==="fullInput"?"input":"authorInput") || proof.inputCut!==(command.kind==="fullInput"?"full":"authorOnly") || proof.draftId !== proposal.owner.draftId || proof.revision !== command.expectedRevision + 1 || proof.lastMutation.mutationId !== command.mutationId || proof.lastMutation.fingerprint !== command.fingerprint || proof.lastMutation.resultingRevision !== proof.revision) throw new Error("Core 回执不是同一原 mutation，未移除本人 proposal。");
  }
}
export function localCaptureAuthorPort(request: <T>(path: string, body: unknown) => Promise<T>): CaptureAuthorPort<CaptureDraftRecord> {
  return { write: command => request("/api/capture/draft/author-input", command), lookup: command => request("/api/capture/draft/author-mutation", command), lookupInput: command=>request("/api/capture/draft/input-mutation",command), recovery: record => record, fullInput:record=>record.currentInput };
}
