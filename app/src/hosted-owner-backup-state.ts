export interface BackupCatalog { enabled: boolean; rights: { read: boolean; export: boolean; download: boolean } | null; scope: "completeSelectedLibraryRecoveryMaterials"; automaticRestore: false }
export interface BackupRequest { requestId: string }
export interface BackupArtifact { digest: string; length: number; filename: string; contentType: string }
export interface BackupReceipt { requestId: string; status: string; planId?: string | null; commitAllowed: boolean; summary?: string | null; artifact?: BackupArtifact | null; deliveryUnavailable?: boolean; automaticRestore: false }
export interface BackupJob { request: BackupRequest; receipt?: BackupReceipt; phase?: "prepare" | "commit" | "settle" | "close"; commit?: { requestId: string; planId: string } }
export interface BackupChunk { requestId: string; digest: string; offset: number; nextOffset: number; length: number; done: boolean; base64: string }
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export const backupEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export function backupReceipt(request: BackupRequest, value: BackupReceipt, previous?: BackupReceipt): BackupReceipt {
  if (!value || value.requestId !== request.requestId || !statuses.has(value.status) || typeof value.commitAllowed !== "boolean" || value.automaticRestore !== false || value.summary != null && typeof value.summary !== "string" || value.commitAllowed && (value.status !== "prepared" || !value.planId)) throw new Error("备份回执不对应原请求或准确计划，未允许生成。");
  if (previous?.status === "committed" && value.status !== "committed" || previous?.planId && value.planId && previous.planId !== value.planId) throw new Error("原备份事实或计划改变，请核对原编号。");
  const a = value.artifact; if (a && (value.status !== "committed" || !/^[a-f0-9]{64}$/.test(a.digest) || !Number.isSafeInteger(a.length) || a.length < 0 || a.length > 80 * 1024 * 1024 || typeof a.filename !== "string" || !a.filename || /[\\/\u0000-\u001f]/.test(a.filename) || typeof a.contentType !== "string" || value.deliveryUnavailable === true)) throw new Error("备份工件无法确认，未授予下载。");
  return structuredClone(value); // Expired/current filtered delivery never inherits cache.
}
export function decodeBackupJob(raw: string): BackupJob {
  const job = JSON.parse(raw) as BackupJob; if (!job?.request || typeof job.request.requestId !== "string" || !job.request.requestId) throw new Error("备份恢复记录无法解析，原记录保留。");
  if (job.receipt) job.receipt = backupReceipt(job.request, job.receipt);
  if (job.commit && (job.commit.requestId !== job.request.requestId || typeof job.commit.planId !== "string" || !job.commit.planId)) throw new Error("原备份确认请求无法解析。");
  return job;
}
export function canCommitBackup(job: BackupJob | undefined, verified: boolean, catalog: BackupCatalog | undefined): boolean { return !!catalog?.enabled && !!catalog.rights?.export && verified && job?.receipt?.status === "prepared" && job.receipt.commitAllowed && !!job.receipt.planId; }
export function canDownloadBackup(job: BackupJob | undefined, verified: boolean, catalog: BackupCatalog | undefined): boolean { return !!catalog?.enabled && !!catalog.rights?.download && verified && job?.receipt?.status === "committed" && !!job.receipt.artifact && !job.receipt.deliveryUnavailable; }
export function backupChunk(request: BackupRequest, artifact: BackupArtifact, offset: number, value: BackupChunk): Uint8Array {
  if (!value || value.requestId !== request.requestId || value.digest !== artifact.digest || value.offset !== offset || value.length !== artifact.length || !Number.isSafeInteger(value.nextOffset) || value.nextOffset < offset || value.nextOffset > artifact.length || typeof value.done !== "boolean" || value.done !== (value.nextOffset === artifact.length) || typeof value.base64 !== "string" || value.base64.length > 87384 || value.base64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(value.base64)) throw new Error("备份分块不对应原工件，未交付下载。");
  const binary = atob(value.base64); if (binary.length > 65536 || binary.length !== value.nextOffset - offset || btoa(binary) !== value.base64 || !binary.length && !(artifact.length === 0 && offset === 0 && value.done)) throw new Error("备份分块缺失或没有进展，未交付下载。");
  return Uint8Array.from(binary, c => c.charCodeAt(0));
}
