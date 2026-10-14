export const contentFormats = ["native", "bibtex", "csl-json", "ris"] as const;
export type ContentFormat = typeof contentFormats[number];
export interface ContentTarget { id: string; title: string; formats: ContentFormat[]; available: boolean }
export interface ContentLoss { code?: string; message: string; location?: string }
export interface ContentInspection { uploadDigest: string; recordCount?: number; records?: { conflicts?: unknown[]; [key: string]: unknown }[]; losses: ContentLoss[]; canPrepare: boolean; reason?: string | null }
export interface ContentRequest { slot: string; operationId: string; format: ContentFormat; bytesBase64: string }
export interface ContentReceipt { slot: string; operationId: string; status: string; planId?: string | null; preview: { summary?: string; losses?: ContentLoss[]; recordCount?: number }; nodeRefs: { workspaceId: string; nodeId: string }[]; commitAllowed: boolean }
export interface ContentDraft { slot: string; format: ContentFormat; revision: number; bytesBase64?: string; fileName?: string; bytes?: number; job?: { request: ContentRequest; fileName: string; revision: number; receipt?: ContentReceipt; phase?: "prepare" | "commit" | "settle"; commit?: { slot: string; operationId: string; planId: string; acceptLosses: boolean } } }
export interface ContentMembers { scope: "configuredImportSlotMembers"; coverage: "complete" | "partial"; members: { resource: string; nodeRef: { workspaceId: string; nodeId: string }; title?: string; status: "ready" | "unavailable"; canOpen: boolean }[] }
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export const contentEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export function contentBytes(value: string): Uint8Array {
  if (typeof value !== "string" || value.length > 22369624 || value.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value)) throw new Error("原件字节无效或超过 16 MiB；原输入保留，未发送操作。");
  const binary = atob(value); if (binary.length > 16 * 1024 * 1024 || btoa(binary) !== value) throw new Error("原件字节不完整，未发送操作。");
  return Uint8Array.from(binary, c => c.charCodeAt(0));
}
export function contentInspection(value: ContentInspection, digest: string): ContentInspection {
  if (!value || value.uploadDigest !== digest || !/^[a-f0-9]{64}$/.test(digest) || typeof value.canPrepare !== "boolean" || !Array.isArray(value.losses) || value.losses.some(x => !x || typeof x.message !== "string") || value.records !== undefined && !Array.isArray(value.records) || value.recordCount !== undefined && (!Number.isSafeInteger(value.recordCount) || value.recordCount < 0)) throw new Error("导入检查不对应准确原件，未允许准备。");
  if (value.canPrepare && value.records?.some(record => (record.conflicts?.length ?? 0) > 0)) throw new Error("检查仍含字段冲突，不能声明可直接新增。");
  return structuredClone(value);
}
export function freezeContentImport(draft: ContentDraft, target: ContentTarget | undefined, inspected: { revision: number; value: ContentInspection } | undefined, operationId: string): ContentRequest {
  if (draft.job) throw new Error("请先核对或明确结束原导入，不会换编号重放。");
  if (!target || target.id !== draft.slot || !target.available || !target.formats.includes(draft.format)) throw new Error("原固定槽位或格式当前不可新增；原件保留。");
  if (typeof draft.bytesBase64 !== "string" || contentBytes(draft.bytesBase64).length !== draft.bytes || !inspected?.value.canPrepare || inspected.revision !== draft.revision) throw new Error("请先检查本次准确原件；后来输入或同值往返不能沿用旧检查。");
  return { slot: draft.slot, format: draft.format, bytesBase64: draft.bytesBase64, operationId };
}
export function contentReceipt(request: ContentRequest, value: ContentReceipt, previous?: ContentReceipt): ContentReceipt {
  if (!value || value.slot !== request.slot || value.operationId !== request.operationId || !statuses.has(value.status) || typeof value.commitAllowed !== "boolean" || !value.preview || typeof value.preview !== "object" || !Array.isArray(value.nodeRefs) || value.nodeRefs.some(x => !x || typeof x.workspaceId !== "string" || typeof x.nodeId !== "string") || value.preview.losses?.some(x => !x || typeof x.message !== "string") || value.preview.summary !== undefined && typeof value.preview.summary !== "string") throw new Error("导入回执不对应原编号或预览无法确认，未采用替代计划。");
  if (value.commitAllowed && (value.status !== "prepared" || !value.planId)) throw new Error("原导入计划没有准确提交资格。");
  if (previous?.status === "committed" && value.status !== "committed" || previous?.planId && value.planId && previous.planId !== value.planId) throw new Error("原导入事实或计划改变，请继续核对原操作。");
  return structuredClone(value); // Filtered status replaces old private preview.
}
export function canCommitContent(draft: ContentDraft, verified: boolean, reviewedRevision: number | undefined): boolean {
  return verified && reviewedRevision === draft.revision && !!draft.job && draft.job.receipt?.status === "prepared" && draft.job.receipt.commitAllowed && !!draft.job.receipt.planId;
}
export function decodeContentDraft(raw: string): ContentDraft {
  const value = JSON.parse(raw) as ContentDraft;
  if (!value || typeof value.slot !== "string" || !contentFormats.includes(value.format) || !Number.isSafeInteger(value.revision) || value.revision < 0 || value.bytesBase64 !== undefined && (contentBytes(value.bytesBase64).length !== value.bytes || typeof value.fileName !== "string")) throw new Error("导入恢复记录无法解析，原记录保留供核对。");
  if (value.job) { const job = value.job; if (!job.request?.slot || !job.request.operationId || !contentFormats.includes(job.request.format) || typeof job.fileName !== "string" || !Number.isSafeInteger(job.revision) || job.revision < 0) throw new Error("原导入请求无效。"); contentBytes(job.request.bytesBase64); if (job.receipt) job.receipt = contentReceipt(job.request, job.receipt); if (job.commit && (job.commit.slot !== job.request.slot || job.commit.operationId !== job.request.operationId || !job.commit.planId || typeof job.commit.acceptLosses !== "boolean")) throw new Error("原确认请求无效。"); }
  return value;
}
