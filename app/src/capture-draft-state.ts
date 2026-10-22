import type { NativeValue } from "./data-panel";
import type { ValuePath } from "./author-input";
import { parseStrictJson } from "./strict-json";

/** Raw JS code units, deliberately not Unicode scalars or a UTF-8 byte buffer. */
export interface CaptureText { utf16: number[] }
export type CaptureValuePath = ValuePath;
export interface CaptureRawProvenance { producerId: string; inputId: string; revision: number }
export const captureTextSlots = ["name", "title", "body", "rootInput", "parentInput", "parametersText"] as const;
export type CaptureTextSlot = typeof captureTextSlots[number];
export interface CaptureRawAuthor { text: CaptureText; provenance: CaptureRawProvenance }
export type CaptureAuthorTexts = Record<CaptureTextSlot, CaptureRawAuthor[]>;
export const emptyCaptureAuthorTexts = (): CaptureAuthorTexts => ({name:[],title:[],body:[],rootInput:[],parentInput:[],parametersText:[]});
export interface CaptureRawInput {
  fieldId: string; valuePath: CaptureValuePath[]; controlRole: string; text: CaptureText;
  binding: "exact" | "copyOnly";
  author: CaptureRawAuthor | null;
}
export interface CaptureAuthorRawInput { fieldId: string; valuePath: CaptureValuePath[]; controlRole: string; text: CaptureText; provenance: CaptureRawProvenance }
export interface CaptureDetachedRawInput { retirementId: string; reason: "structure" | "schema" | "producer"; input: CaptureRawInput }
export type CaptureFieldValueState = { kind: "valid"; value: NativeValue } | { kind: "incomplete"; wire: CaptureText };
export interface CaptureFieldInput { fieldId: string; valueState: CaptureFieldValueState }
export interface CaptureInput {
  name: CaptureText; title: CaptureText; body: CaptureText; bodyMode: "text" | "markup";
  rootInput: CaptureText; parentInput: CaptureText; parametersText: CaptureText; templateId: string | null;
  authorTexts: CaptureAuthorTexts;
  authorBodyMode: "text" | "markup" | null;
  typeIds: string[]; fields: CaptureFieldInput[];
  rawInputs: CaptureRawInput[];
  detachedRawInputs: CaptureDetachedRawInput[];
  formBasis: { schemaRef: string; semanticDigest: string } | null;
}
export interface CaptureAuthorInput {
  texts: CaptureAuthorTexts; bodyMode: "text" | "markup" | null;
  rawInputs: CaptureAuthorRawInput[];
  detachedRawInputs: { retirementId: string; reason: "structure" | "schema" | "producer"; input: CaptureAuthorRawInput }[];
}
export interface CaptureAuthorRecovery extends CaptureAuthorInput {
  draftId: string; revision: number; inputRevision: number; inputCut: CaptureInputCut;
  lastMutation: CaptureDraftRecord["lastMutation"];
  pending: Pick<FrozenCaptureRequest, "operationId" | "requestKind" | "inputRevision"> | null;
}
/** The projection is copy-only evidence of typed author text. Full rawValue,
 * protected candidate/default/target/template and wire never enter this DTO. */
export function captureAuthorInput(input: CaptureInput): CaptureAuthorInput {
  const own = (raw: CaptureRawInput): CaptureAuthorRawInput[] => raw.author ? [{
    fieldId: raw.fieldId,
    valuePath: raw.valuePath.map(part => part.kind === "member" ? { kind: "member", key: part.key } : { kind: "index", index: part.index }),
    controlRole: raw.controlRole,
    text: structuredClone(raw.author.text),
    provenance: { producerId: raw.author.provenance.producerId, inputId: raw.author.provenance.inputId, revision: raw.author.provenance.revision },
  }] : [];
  return {
    texts: structuredClone(input.authorTexts), bodyMode: input.authorBodyMode,
    rawInputs: input.rawInputs.flatMap(own),
    detachedRawInputs: input.detachedRawInputs.flatMap(entry => own(entry.input).map(raw => ({ retirementId: entry.retirementId, reason: entry.reason, input: raw }))),
  };
}
/** This exact ordered DTO matches Core's authorInput fingerprint, containing
 * only code units and native stable addresses, not a reparsed frozen request. */
export async function captureAuthorFingerprint(expectedRevision: number, input: CaptureAuthorInput): Promise<string> {
  const wire = JSON.stringify([expectedRevision, "authorInput", input]);
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(wire)))].map(byte => byte.toString(16).padStart(2, "0")).join("");
}
export type CaptureInputCut = "full" | "authorOnly";
/** Matches Core canonical JSON for this closed DTO. Native record names are
 * values and author code units are numeric arrays, never object-key aliases. */
export async function captureInputFingerprint(expectedRevision: number, input: CaptureInput): Promise<string> {
  const ordered = (value: unknown): unknown => Array.isArray(value) ? value.map(ordered) : value && typeof value === "object" ? Object.fromEntries(Object.entries(value).sort(([a],[b])=>a<b?-1:a>b?1:0).map(([key,child])=>[key,ordered(child)])) : value;
  const wire=JSON.stringify(ordered([expectedRevision,"input",input]));
  return [...new Uint8Array(await crypto.subtle.digest("SHA-256",new TextEncoder().encode(wire)))].map(byte=>byte.toString(16).padStart(2,"0")).join("");
}
export interface FrozenCaptureRequest {
  operationId: string; requestKind: "capture" | "period" | "hostedStructure";
  wire: string; wireDigest: string; inputRevision: number;
}
export interface CaptureDraftRecord {
  version: 2; draftId: string; revision: number; inputRevision: number; inputCut: CaptureInputCut;
  lastMutation: { kind:"create"|"input"|"authorInput"|"checkpoint"|"target"|"freeze"|"advance"; mutationId: string; fingerprint: string; resultingRevision: number };
  currentInput: CaptureInput; pending: FrozenCaptureRequest | null;
  target: { rootHint: string; parent: { kind: "libraryRoot" } | { kind: "managed"; reference: string; pathHint: string; identityDigest: string }; hostedSlot: string | null } | null;
}
export interface CaptureFlushReceipt {
  draftId: string; recordRevision: number; inputRevision: number; inputCut: CaptureInputCut; lastMutationId: string;
  pendingOperationId: string | null; recordDigest: string;
}
export type CaptureTransport = <T>(path: string, body: unknown) => Promise<T>;
export interface CaptureTargetSelection { root: string; parent: string }
type MutationIdentity = { draftId: string; mutationId: string };
export interface CaptureMutationPayload {
  checkpoint: MutationIdentity & { expectedRevision: number };
  create: MutationIdentity & { input: CaptureInput; target?: CaptureTargetSelection | null };
  input: MutationIdentity & { expectedRevision: number; input: CaptureInput };
  target: MutationIdentity & { expectedRevision: number; target: CaptureTargetSelection };
  freeze: MutationIdentity & { expectedRevision: number; request: FrozenCaptureRequest };
  advance: MutationIdentity & { expectedRevision: number; operationId: string; resetInput: boolean };
}
const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/;
const digest = /^[a-f0-9]{64}$/;
const integer = (value: number) => Number.isSafeInteger(value) && value >= 0;
export function captureText(text: string): CaptureText {
  const utf16 = new Array<number>(text.length);
  for (let index = 0; index < text.length; index++) utf16[index] = text.charCodeAt(index);
  return { utf16 };
}
export function captureString(text: CaptureText): string {
  if (!text || !Array.isArray(text.utf16) || text.utf16.length > 2 * 1024 * 1024 || text.utf16.some(unit => !Number.isInteger(unit) || unit < 0 || unit > 65535)) throw new Error("捕获原文编码无效，原件保留。");
  const pieces: string[] = [];
  for (let index = 0; index < text.utf16.length; index += 4096) pieces.push(String.fromCharCode(...text.utf16.slice(index, index + 4096)));
  return pieces.join("");
}
/** Every control's raw must be retained, even DOM-valid text with a lone surrogate. */
export function captureUtf8Ready(text: CaptureText): string {
  const value = captureString(text);
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index);
      if (!(next >= 0xdc00 && next <= 0xdfff)) throw new Error("尚未完成的 UTF-16 输入已保留，请补全后再创建正文。");
    } else if (unit >= 0xdc00 && unit <= 0xdfff) throw new Error("尚未完成的 UTF-16 输入已保留，请补全后再创建正文。");
  }
  return value;
}
export function emptyCaptureInput(): CaptureInput {
  return { name: captureText(""), title: captureText(""), body: captureText(""), authorTexts: emptyCaptureAuthorTexts(), authorBodyMode:null, bodyMode: "markup", rootInput: captureText(""), parentInput: captureText(""), parametersText: captureText(""), templateId: null, typeIds: [], fields: [], rawInputs: [], detachedRawInputs: [], formBasis: null };
}
/** The snapshot is emitted only by the positively registered author collector.
 * Sampling/restoring a DOM value has no route to this proof installer. */
export function observeCaptureText(input: CaptureInput, slot: CaptureTextSlot, snapshot: {rawValue:string;text:string;producerId:string;id:string;revision:number}): void {
  input[slot] = captureText(snapshot.rawValue);
  const proof={text:captureText(snapshot.text),provenance:{producerId:snapshot.producerId,inputId:snapshot.id,revision:snapshot.revision}};
  const old=input.authorTexts[slot].findIndex(item=>item.provenance.producerId===snapshot.producerId && item.provenance.inputId===snapshot.id);
  if(old<0)input.authorTexts[slot].push(proof);else input.authorTexts[slot][old]=proof;
}
/** The wire is exactly one private native field tree. It cannot include an
 * entire form, protected Schema/Base or a second encoding of a complete value. */
function nativeField(value: unknown, budget = {nodes:0}, depth = 0): NativeValue {
  if (++budget.nodes > 100000 || depth > 32 || !value || typeof value !== "object" || Array.isArray(value)) throw new Error("捕获字段超预算或不是原生值。");
  const object = value as Record<string, unknown>, keys = Object.keys(object).sort().join();
  if (typeof object.kind !== "string" || keys !== (object.kind === "none" ? "kind" : "kind,value")) throw new Error("捕获字段包含未知或缺失的原生成员。");
  if (object.kind === "none") return {kind:"none"};
  if (object.kind === "bool" && typeof object.value === "boolean") return {kind:"bool",value:object.value};
  if (["text","int","float","decimal","date","time","localDatetime","duration"].includes(object.kind) && typeof object.value === "string") return {kind:object.kind as "text",value:object.value};
  if (object.kind === "array" && Array.isArray(object.value)) return {kind:"array",value:object.value.map(item=>nativeField(item,budget,depth+1))};
  if (object.kind === "record" && Array.isArray(object.value)) {
    const names = new Set<string>();
    return {kind:"record",value:object.value.map(item=>{
      if (!item || typeof item !== "object" || Array.isArray(item) || Object.keys(item).sort().join() !== "key,value" || typeof item.key !== "string" || names.has(item.key)) throw new Error("捕获原生字段名称重复或无效。");
      captureUtf8Ready(captureText(item.key));names.add(item.key);
      return {key:item.key,value:nativeField(item.value,budget,depth+1)};
    })};
  }
  throw new Error("捕获字段含未知原生类型或载荷。");
}
function fieldTree(field: CaptureFieldInput): NativeValue {
  const state = field.valueState;
  if (!state || Object.keys(field).sort().join() !== "fieldId,valueState") throw new Error("捕获字段格式不支持；原件保留。");
  if (state.kind === "valid" && Object.keys(state).sort().join() === "kind,value") return nativeField(state.value);
  if (state.kind === "incomplete" && Object.keys(state).sort().join() === "kind,wire") return nativeField(parseStrictJson(captureString(state.wire)));
  throw new Error("捕获字段没有唯一且完整的私有状态。");
}
function fieldComplete(value: NativeValue, fieldId: string, rawInputs: readonly CaptureRawInput[], path: ValuePath[] = []): boolean {
  if (value.kind === "array") return value.value.map((item,index)=>fieldComplete(item,fieldId,rawInputs,[...path,{kind:"index",index}])).every(Boolean);
  if (value.kind === "record") return value.value.map(item=>fieldComplete(item.value,fieldId,rawInputs,[...path,{kind:"member",key:item.key}])).every(Boolean);
  if (value.kind === "none" || value.kind === "bool") return true;
  try { captureUtf8Ready(captureText(value.value));return true; }
  catch {
    if (!rawInputs.some(raw=>raw.fieldId===fieldId && raw.binding==="exact" && JSON.stringify(raw.valuePath)===JSON.stringify(path) && captureString(raw.text)===value.value)) throw new Error("未完成字段没有对应的原文控件地址，未声称已可靠保存。");
    return false;
  }
}
export function captureInputForPersistence(input: CaptureInput): CaptureInput {
  const next = structuredClone(input), fields = new Set<string>();
  for (const field of next.fields) {
    if (!field.fieldId || fields.has(field.fieldId)) throw new Error("捕获字段身份缺失或重复。");
    fields.add(field.fieldId);
    const value = fieldTree(field), complete = fieldComplete(value,field.fieldId,next.rawInputs);
    if (field.valueState.kind === "incomplete" && complete) throw new Error("完整原生字段必须使用 Valid 状态。");
    field.valueState = complete ? {kind:"valid",value} : {kind:"incomplete",wire:captureText(JSON.stringify(value))};
  }
  return next;
}
/** Call only after the host freshly qualifies the original field and Schema.
 * This returns private state to its real owner; no synthetic input event/proof. */
export function restoreCaptureField(field: CaptureFieldInput, input: CaptureInput, authority: {fieldId:string;schemaRef:string;semanticDigest:string}): NativeValue {
  if (field.fieldId!==authority.fieldId || !input.formBasis || input.formBasis.schemaRef!==authority.schemaRef || input.formBasis.semanticDigest!==authority.semanticDigest) throw new Error("字段或 Schema 身份已改变，原文保留为未绑定恢复稿。");
  const value=fieldTree(field),complete=fieldComplete(value,field.fieldId,input.rawInputs);
  if (complete !== (field.valueState.kind === "valid")) throw new Error("原字段状态和编码不一致，未自动恢复另一份候选。");
  return value;
}
export function captureFieldCandidate(field: CaptureFieldInput): NativeValue {
  if (field.valueState?.kind!=="valid") throw new Error("字段仍有未完成原文，请完成后再创建。");
  const value=fieldTree(field);
  if (!fieldComplete(value,field.fieldId,[])) throw new Error("字段尚未形成完整 UTF-8 原生值。");
  return value;
}
export function captureInputsEqual(left: CaptureInput, right: CaptureInput): boolean {
  const ordered = (value: unknown): unknown => Array.isArray(value) ? value.map(ordered) : value && typeof value === "object" ? Object.fromEntries(Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([key, child]) => [key, ordered(child)])) : value;
  return JSON.stringify(ordered(left)) === JSON.stringify(ordered(right));
}
function record(value: CaptureDraftRecord, id: string): CaptureDraftRecord {
  if (!value || value.version !== 2 || !["full", "authorOnly"].includes(value.inputCut) || value.draftId !== id || !uuid.test(id) || !integer(value.revision) || value.revision === 0 || !integer(value.inputRevision) || value.inputRevision > value.revision || !value.lastMutation || !["create","input","authorInput","checkpoint","target","freeze","advance"].includes(value.lastMutation.kind) || !uuid.test(value.lastMutation.mutationId) || value.lastMutation.resultingRevision !== value.revision || !digest.test(value.lastMutation.fingerprint)) throw new Error("捕获回执不对应原草稿和修订，输入保留。");
  if (value.pending && (!uuid.test(value.pending.operationId) || !["capture", "period", "hostedStructure"].includes(value.pending.requestKind) || typeof value.pending.wire !== "string" || !digest.test(value.pending.wireDigest) || !integer(value.pending.inputRevision) || value.pending.inputRevision > value.inputRevision)) throw new Error("捕获原请求无法准确恢复，输入保留。");
  return structuredClone(value);
}
export class CapturePersistenceConflict extends Error {
  constructor(public latest?: CaptureDraftRecord, public original?: unknown) { super("捕获持久化尚未确认或另一窗口已更新；当前新输入保留，未覆盖原件。"); }
}
/** One Core authority. A lost response queries the same mutation, never replays newer input. */
export class CapturePersistence {
  constructor(private readonly request: CaptureTransport) {}
  async read(draftId: string): Promise<CaptureDraftRecord> {
    return record(await this.request<CaptureDraftRecord>("/api/capture/draft/read", { draftId }), draftId);
  }
  async mutate<R extends keyof CaptureMutationPayload>(route: R, payload: CaptureMutationPayload[R]): Promise<CaptureDraftRecord> {
    // Snapshot before the first await. The caller may continue editing its object.
    const frozen = structuredClone(payload);
    const { draftId: _draftId, mutationId: _mutationId, ...originalCommand } = frozen;
    const command = { ...originalCommand, kind: route };
    try {
      const actual = record(await this.request<CaptureDraftRecord>(`/api/capture/draft/${route}`, frozen), frozen.draftId);
      if (actual.lastMutation.mutationId !== frozen.mutationId) throw new CapturePersistenceConflict(actual);
      return actual;
    } catch (original) {
      let lookup: { status: string; draft: CaptureDraftRecord };
      try { lookup = await this.request("/api/capture/draft/mutation", { draftId: frozen.draftId, mutationId: frozen.mutationId, command }); }
      catch { throw new CapturePersistenceConflict(undefined, original); }
      const latest = record(lookup.draft, frozen.draftId);
      if (lookup.status === "applied" && latest.lastMutation.mutationId === frozen.mutationId) return latest;
      throw new CapturePersistenceConflict(latest, original);
    }
  }
  /** ACK only. Native must separately freeze input and consume a surface-bound permit. */
  async flush(draft: CaptureDraftRecord, surfaceEditRevision: number, currentSurfaceEditRevision: () => number): Promise<CaptureFlushReceipt> {
    const frozen = structuredClone(draft);
    if (surfaceEditRevision !== currentSurfaceEditRevision() || frozen.inputCut !== "full") throw new CapturePersistenceConflict();
    const receipt = await this.request<CaptureFlushReceipt>("/api/capture/draft/flush", { draftId: frozen.draftId, recordRevision: frozen.revision, inputRevision: frozen.inputRevision, lastMutationId: frozen.lastMutation.mutationId });
    if (surfaceEditRevision !== currentSurfaceEditRevision() || !receipt || receipt.inputCut !== "full" || receipt.draftId !== frozen.draftId || receipt.recordRevision !== frozen.revision || receipt.inputRevision !== frozen.inputRevision || receipt.lastMutationId !== frozen.lastMutation.mutationId || receipt.pendingOperationId !== (frozen.pending?.operationId ?? null) || !digest.test(receipt.recordDigest)) throw new CapturePersistenceConflict();
    return structuredClone(receipt);
  }
}
