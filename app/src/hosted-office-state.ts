import type { OutputFile, OutputLoss, TemplateInspection } from "./export-state";

export type OfficeFormat = "docx" | "xlsx" | "pdf" | "bibtex" | "csl-json" | "ris" | "native";
export const officeFormats: OfficeFormat[] = ["docx", "xlsx", "pdf", "bibtex", "csl-json", "ris", "native"];
export type OfficeBinding = { kind: "body" | "title" | "text" | "query" | "field"; text?: string; field?: string };
export type OfficeDraft = { resource: string; format: OfficeFormat; reviewMode: "none" | "comments" | "changes"; includeResolved: boolean; queryId: string; stylePolicy: "template" | "source"; bindings: Record<string, OfficeBinding>; template?: OutputFile; inspection?: TemplateInspection; bibliographyScope?: "citationWorks" | "selectedWorks"; works?: string[]; bibliographySource?: "draft" | "saved"; includeEmbedded?: boolean };
export type OfficeOutputRequest = Pick<OfficeDraft, "format" | "reviewMode" | "includeResolved" | "stylePolicy" | "bindings" | "template"> & { requestId: string; resource?: string; source?: string; queryId?: string; sessionId?: string; baseId?: string; serial?: number; sourceHash?: string; bibliographyScope?: "citationWorks" | "selectedWorks"; works?: string[]; embeddedQueries?: { occurrenceId: string; queryId: string }[] };
export type OfficeOutputReceipt = { requestId: string; status: "pending" | "prepared" | "confirmed" | "failed" | "expired" | "notRegistered"; filename?: string; contentType?: string; length?: number; digest?: string; losses?: OutputLoss[]; summary?: string; preview?: unknown; message?: string };
export type OfficeOutputVerification = { requestId: string; epoch: number };
export type OfficeImportRequest = { resource: string; operationId: string; bytesBase64: string };
export type OfficeImportReceipt = { operationId: string; resource: string; status: string; planId?: string | null; preview?: { summary?: string | string[]; losses?: OutputLoss[]; source?: string }; commitAllowed?: boolean };
const uncertain = (message: string) => Object.assign(new Error(message), { code: "resultUnknown" });
const statuses = new Set(["pending", "prepared", "confirmed", "failed", "expired", "notRegistered"]);
const importStatuses = new Set(["pending", "prepared", "committed", "cancelled", "resolved", "notRegistered", "notCommitted", "recoveryPending", "failed", "expired"]);
const lossesValid = (losses: unknown): losses is OutputLoss[] => Array.isArray(losses) && losses.every(loss => !!loss && typeof loss.code === "string" && typeof loss.message === "string");

/** A receipt may acknowledge state, but cannot replace the frozen artifact. */
export function outputReceipt(requestId: string, incoming: OfficeOutputReceipt, previous?: OfficeOutputReceipt): OfficeOutputReceipt {
  if (!incoming || incoming.requestId !== requestId || !statuses.has(incoming.status)) throw uncertain("输出回执与原请求不一致，请核对原输出。");
  // A terminal lookup acknowledges the original ID without redelivering old
  // target/source-derived preview data after expiration or lost authority.
  if (["expired", "failed", "notRegistered"].includes(incoming.status)) return { requestId, status: incoming.status, ...(typeof incoming.message === "string" ? { message: incoming.message } : {}) };
  if (previous?.digest) for (const key of ["digest", "length", "filename", "contentType"] as const) {
    if (incoming[key] !== undefined && incoming[key] !== previous[key]) throw uncertain("原输出的准确文件信息发生变化，未交付替换文件。");
  }
  const result = { ...previous, ...incoming };
  if (["prepared", "confirmed"].includes(result.status) && (!result.filename || !result.contentType || !Number.isSafeInteger(result.length) || result.length! < 0 || result.length! > 64 * 1024 * 1024 || !/^[a-f0-9]{64}$/.test(result.digest ?? "") || !lossesValid(result.losses))) throw uncertain("输出回执缺少准确工件或损失信息，请核对原输出。");
  if (previous?.status === "confirmed" && result.status === "prepared") throw uncertain("已确认输出不能退回另一份准备状态，请核对原输出。");
  return result;
}
/** Cache is a recovery locator, never current delivery authority. */
export function officeOutputPresentation(requestId: string, receipt: OfficeOutputReceipt | undefined, verification: OfficeOutputVerification | undefined, epoch: number): OfficeOutputReceipt | undefined {
  return receipt && verification?.requestId === requestId && verification.epoch === epoch ? outputReceipt(requestId, receipt) : undefined;
}
export function importReceipt(request: OfficeImportRequest, incoming: OfficeImportReceipt, previous?: OfficeImportReceipt): OfficeImportReceipt {
  if (!incoming || incoming.operationId !== request.operationId || incoming.resource !== request.resource || !importStatuses.has(incoming.status)) throw uncertain("导入回执与原操作或目标不一致，请核对原导入。");
  if (previous?.planId && incoming.planId && previous.planId !== incoming.planId) throw uncertain("原导入计划发生变化，未提交替换计划。");
  if (incoming.status === "prepared" && incoming.commitAllowed !== false && (!incoming.planId || !lossesValid(incoming.preview?.losses))) throw uncertain("导入预览缺少准确计划或损失信息，请核对原操作。");
  if (previous?.status === "committed" && incoming.status !== "committed") throw uncertain("原导入已提交，不能复活或再次执行。");
  return incoming;
}
export type OfficeChunk = { bytesBase64: string; offset: number; length: number; total: number; sha256: string; done: boolean };
export function officeChunk(chunk: OfficeChunk, offset: number, total: number, sha256: string): Uint8Array {
  if (!chunk || chunk.offset !== offset || chunk.total !== total || chunk.sha256 !== sha256 || typeof chunk.bytesBase64 !== "string" || chunk.bytesBase64.length > 65536 * 4 / 3 + 4 || !Number.isSafeInteger(chunk.length) || chunk.length <= 0 || chunk.length > 65536 || offset + chunk.length > total || chunk.done !== (offset + chunk.length === total)) throw uncertain("下载块与已确认工件不一致，未交付文件；可重新下载原输出。");
  let bytes: Uint8Array;
  try { bytes = Uint8Array.from(atob(chunk.bytesBase64), c => c.charCodeAt(0)); } catch { throw uncertain("下载块编码无效，未交付文件。"); }
  if (bytes.length !== chunk.length) throw uncertain("下载块长度不完整，未交付文件。");
  return bytes;
}
