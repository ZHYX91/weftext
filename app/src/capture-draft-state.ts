import type { NativeValue } from "./data-panel";
import type { ValuePath } from "./author-input";

/** Raw JS code units, deliberately not Unicode scalars or a UTF-8 byte buffer. */
export interface CaptureText { utf16: number[] }
export type CaptureValuePath = ValuePath;
export interface CaptureInput {
  name: CaptureText; title: CaptureText; body: CaptureText; bodyMode: "text" | "markup";
  parentInput: CaptureText; parametersText: CaptureText; templateId: string | null;
  typeIds: string[]; fields: { fieldId: string; value: NativeValue | null }[];
  rawInputs: { fieldId: string; valuePath: CaptureValuePath[]; controlRole: string; text: CaptureText }[];
  formBasis: { schemaRef: string; semanticDigest: string } | null;
}
export interface FrozenCaptureRequest {
  operationId: string; requestKind: "capture" | "period" | "hostedStructure";
  wire: string; wireDigest: string; inputRevision: number;
}
export interface CaptureDraftRecord {
  version: 1; draftId: string; revision: number; inputRevision: number;
  lastMutation: { mutationId: string; fingerprint: string; resultingRevision: number };
  currentInput: CaptureInput; pending: FrozenCaptureRequest | null;
  target: { rootHint: string; parent: { kind: "libraryRoot" } | { kind: "managed"; reference: string; pathHint: string; identityDigest: string }; hostedSlot: string | null } | null;
}
export interface CaptureFlushReceipt {
  draftId: string; recordRevision: number; inputRevision: number; lastMutationId: string;
  pendingOperationId: string | null; recordDigest: string;
}
export type CaptureTransport = <T>(path: string, body: unknown) => Promise<T>;
export interface CaptureTargetSelection { root: string; parent: string }
type MutationIdentity = { draftId: string; mutationId: string };
interface MutationPayload {
  create: MutationIdentity & { input: CaptureInput; target?: CaptureTargetSelection | null };
  input: MutationIdentity & { expectedRevision: number; input: CaptureInput };
  target: MutationIdentity & { expectedRevision: number; target: CaptureTargetSelection };
  freeze: MutationIdentity & { expectedRevision: number; request: FrozenCaptureRequest };
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
  return { name: captureText(""), title: captureText(""), body: captureText(""), bodyMode: "markup", parentInput: captureText(""), parametersText: captureText(""), templateId: null, typeIds: [], fields: [], rawInputs: [], formBasis: null };
}
function record(value: CaptureDraftRecord, id: string): CaptureDraftRecord {
  if (!value || value.version !== 1 || value.draftId !== id || !uuid.test(id) || !integer(value.revision) || value.revision === 0 || !integer(value.inputRevision) || value.inputRevision > value.revision || !value.lastMutation || !uuid.test(value.lastMutation.mutationId) || value.lastMutation.resultingRevision !== value.revision || !digest.test(value.lastMutation.fingerprint)) throw new Error("捕获回执不对应原草稿和修订，输入保留。");
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
  async mutate<R extends keyof MutationPayload>(route: R, payload: MutationPayload[R]): Promise<CaptureDraftRecord> {
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
    if (surfaceEditRevision !== currentSurfaceEditRevision()) throw new CapturePersistenceConflict();
    const receipt = await this.request<CaptureFlushReceipt>("/api/capture/draft/flush", { draftId: frozen.draftId, recordRevision: frozen.revision, inputRevision: frozen.inputRevision, lastMutationId: frozen.lastMutation.mutationId });
    if (surfaceEditRevision !== currentSurfaceEditRevision() || !receipt || receipt.draftId !== frozen.draftId || receipt.recordRevision !== frozen.revision || receipt.inputRevision !== frozen.inputRevision || receipt.lastMutationId !== frozen.lastMutation.mutationId || receipt.pendingOperationId !== (frozen.pending?.operationId ?? null) || !digest.test(receipt.recordDigest)) throw new CapturePersistenceConflict();
    return structuredClone(receipt);
  }
}
