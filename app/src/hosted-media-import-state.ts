export interface HostedResourceCollection { collection: string; owner: string; title: string; lifecycle: "live" | "trash"; read: boolean; download: boolean; import: boolean; status: "ready" | "unavailable" }
export interface ResourceImportRequest { collection: string; operationId: string; name: string; bytesBase64: string }
export interface ResourceImportReceipt { collection: string; owner: string; operationId: string; status: string; planId?: string | null; preview?: { summary?: string | string[]; name?: string; bytes?: number; contentType?: string; metadata?: unknown }; resourceRefs?: string[]; assets?: string[]; commitAllowed: boolean }
export interface ResourceImportDraft { collection: string; name: string; bytesBase64?: string; fileName?: string; bytes?: number; revision: number; pending?: { request: ResourceImportRequest; owner: string; input: string; revision: number; receipt?: ResourceImportReceipt } }
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export const resourceImportEnded = (status?: string) => ["committed", "cancelled", "resolved"].includes(status ?? "");
export const resourceImportInput = (draft: ResourceImportDraft) => JSON.stringify([draft.collection, draft.name, draft.bytesBase64 ?? null]);
export function resourceCollection(value: HostedResourceCollection): HostedResourceCollection {
  if (!value || ![value.collection, value.owner, value.title].every(item => typeof item === "string" && !!item) || !["live", "trash"].includes(value.lifecycle) || !["ready", "unavailable"].includes(value.status) || ![value.read, value.download, value.import].every(item => typeof item === "boolean") || value.download && !value.read || value.lifecycle === "trash" && value.import) throw new Error("资源集合缺少独立权限或准确生命周期；未采用目录替代项。");
  return structuredClone(value);
}
function importBytes(bytesBase64: string): number { if (typeof bytesBase64 !== "string" || bytesBase64.length > 22369624 || bytesBase64.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(bytesBase64)) throw new Error("未取得完整、有效的原件字节，未发起资源写入。"); const length = atob(bytesBase64).length; if (length > 16 * 1024 * 1024) throw new Error("资源导入超过 16 MiB 上限，原件仍保留。"); return length; }
export function freezeResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined, operationId: string): ResourceImportRequest {
  if (!collection || collection.collection !== draft.collection || collection.status !== "ready" || !collection.import || collection.lifecycle !== "live") throw new Error("此集合当前没有独立导入资格；原输入保留。");
  if (draft.pending) throw new Error("请先核对或明确结束原资源导入，不会换编号重放。");
  if (!draft.name.trim() || /[\\/\u0000-\u001f\u007f]/.test(draft.name) || typeof draft.bytesBase64 !== "string" || importBytes(draft.bytesBase64) !== draft.bytes) throw new Error("请保留原件并输入单个资源名称，字节或名称尚未完整。");
  return { collection: draft.collection, operationId, name: draft.name, bytesBase64: draft.bytesBase64 };
}
export function resourceImportReceipt(request: ResourceImportRequest, incoming: ResourceImportReceipt, previous?: ResourceImportReceipt): ResourceImportReceipt {
  if (!incoming || incoming.collection !== request.collection || incoming.operationId !== request.operationId || typeof incoming.owner !== "string" || !incoming.owner || !statuses.has(incoming.status) || typeof incoming.commitAllowed !== "boolean") throw new Error("资源导入回执不对应原编号或集合，请查询原操作。");
  if (previous?.planId && incoming.planId && previous.planId !== incoming.planId || previous?.status === "committed" && incoming.status !== "committed") throw new Error("原资源导入事实或计划改变，未执行替代操作。");
  if (incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId || incoming.preview?.name !== request.name || incoming.preview?.bytes !== importBytes(request.bytesBase64))) throw new Error("导入计划缺少准确原件预览，未允许提交。");
  for (const key of ["assets", "resourceRefs"] as const) if (incoming[key] !== undefined && (!Array.isArray(incoming[key]) || incoming[key]!.some(item => typeof item !== "string" || !item))) throw new Error("导入定位回执无效，未授予资源读取权限。");
  // Incoming permission/config filtering replaces, rather than revives, any
  // old preview or locator. A committed fact survives without its content.
  return structuredClone(incoming);
}
export function canCommitResourceImport(draft: ResourceImportDraft, collection: HostedResourceCollection | undefined): boolean {
  const pending = draft.pending;
  return !!pending && !!collection && collection.collection === pending.request.collection && collection.owner === pending.owner && collection.import && collection.status === "ready" && collection.lifecycle === "live" && pending.receipt?.status === "prepared" && pending.receipt.commitAllowed && !!pending.receipt.planId && pending.revision === draft.revision && pending.input === resourceImportInput(draft);
}
export function decodeResourceImportDraft(raw: string): ResourceImportDraft {
  const draft = JSON.parse(raw) as ResourceImportDraft;
  if (!draft || typeof draft.collection !== "string" || typeof draft.name !== "string" || !Number.isSafeInteger(draft.revision) || draft.revision < 0 || draft.bytesBase64 !== undefined && (importBytes(draft.bytesBase64) !== draft.bytes || typeof draft.fileName !== "string")) throw new Error("资源导入恢复记录无法解析，原记录保留供核对。");
  if (draft.pending) { const p = draft.pending; if (!p.request?.collection || !p.request.operationId || typeof p.request.name !== "string" || typeof p.owner !== "string" || !Number.isSafeInteger(p.revision) || typeof p.input !== "string") throw new Error("原资源导入请求无法解析。"); importBytes(p.request.bytesBase64); if (p.receipt) resourceImportReceipt(p.request, p.receipt); }
  return draft;
}
