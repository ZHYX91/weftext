import type { NativeStyleControl, NativeStyleValue } from "./style-panel";

export type LibraryRole = "schema" | "settings" | "rules" | "style";
export interface LibraryProfile { role: LibraryRole; basis: string; source: string; installed: boolean; enabled?: boolean; controls?: NativeStyleControl[]; warnings?: string[] }
export interface LibraryInput { source: string; changes: Record<string, string | boolean>; enabled?: boolean }
export interface LibraryPrepare { role: LibraryRole; basis: string; operationId: string; source?: string; changes?: Record<string, string>; enabled?: boolean }
export interface LibraryPreview { beforeSource?: string; afterSource?: string; summary?: string | string[]; enabled?: boolean }
export interface LibraryReceipt { role: LibraryRole; operationId: string; status: string; planId?: string | null; preview?: LibraryPreview | null; commitAllowed?: boolean }
export interface LibraryPending { request: LibraryPrepare; input: LibraryInput; revision: number; receipt?: LibraryReceipt; commit?: { role: LibraryRole; operationId: string; planId: string } }
export interface LibraryDraft extends LibraryInput { baseSource: string; baseEnabled?: boolean; revision: number; pending?: LibraryPending; lastReceipt?: Pick<LibraryReceipt, "role" | "operationId" | "status"> }
export const libraryRoles: readonly LibraryRole[] = ["schema", "settings", "rules", "style"];
export const libraryTerminal = (status: string) => ["committed", "cancelled", "resolved"].includes(status);
const failure = (message: string) => Object.assign(new Error(message), { code: "resultUnknown" });
export function libraryReceipt(request: LibraryPrepare, incoming: LibraryReceipt, previous?: LibraryReceipt, beforeSource?: string): LibraryReceipt {
  if (!incoming || incoming.role !== request.role || incoming.operationId !== request.operationId || !["prepared", "committed", "cancelled", "resolved", "recoveryPending", "notRegistered", "notCommitted", "pending", "failed", "expired"].includes(incoming.status)) throw failure("配置回执与原角色或原操作不一致，请查询原操作。");
  if (previous?.planId && incoming.planId && previous.planId !== incoming.planId || previous?.status === "committed" && incoming.status !== "committed") throw failure("原配置计划或提交事实发生变化，未采用替换回执。");
  if (incoming.status === "prepared" && incoming.commitAllowed === true && (!incoming.planId || typeof incoming.preview?.beforeSource !== "string" || typeof incoming.preview.afterSource !== "string" || !Array.isArray(incoming.preview.summary) && typeof incoming.preview.summary !== "string")) throw failure("配置预览缺少准确源码或计划，请查询原操作。");
  if (incoming.status === "prepared" && incoming.commitAllowed === true && (beforeSource !== undefined && incoming.preview?.beforeSource !== beforeSource || request.source !== undefined && incoming.preview?.afterSource !== request.source)) throw failure("配置预览不对应原准确源码，未采用替换计划。");
  // Never revive a previously readable preview after permission/owner changes.
  return structuredClone(incoming);
}
export function libraryInput(draft: LibraryInput): LibraryInput { return { source: draft.source, changes: { ...draft.changes }, ...(draft.enabled === undefined ? {} : { enabled: draft.enabled }) }; }
export function libraryCanCommit(draft: LibraryDraft): boolean {
  const pending = draft.pending, receipt = pending?.receipt;
  return !!pending && receipt?.status === "prepared" && receipt.commitAllowed === true && !!receipt.planId && pending.revision === draft.revision && JSON.stringify(pending.input) === JSON.stringify(libraryInput(draft));
}
export function libraryChanges(controls: readonly NativeStyleControl[], raw: LibraryInput["changes"]): Record<string, string> {
  const result: Record<string, string> = {};
  for (const [key, input] of Object.entries(raw)) {
    const control = controls.find(item => item.key === key);
    if (!control?.editable) throw new Error(`原控件输入 ${key} 当前没有准确可写参数，请保留原文并明确处理。`);
    let value: NativeStyleValue;
    if (control.kind === "boolean") { if (typeof input !== "boolean") throw new Error(`控件 ${key} 的恢复输入无效。`); value = input; }
    else {
      if (typeof input !== "string") throw new Error(`控件 ${key} 的恢复输入无效。`);
      if (control.kind === "select") { const index = Number(input); if (!/^\d+$/.test(input) || !control.options?.[index]) throw new Error(`请核对 ${key} 的原选项。`); value = control.options[index].value; }
      else if (control.kind === "number") { value = Number(input); if (!input.trim() || !Number.isFinite(value) || control.min !== undefined && value < control.min || control.max !== undefined && value > control.max) throw new Error(`控件 ${key} 的原数值无效，尚未准备操作。`); }
      else value = input;
    }
    result[key] = String(value);
  }
  return result;
}
export function freezeLibraryPrepare(role: LibraryRole, profile: LibraryProfile, draft: LibraryDraft, operationId: string): LibraryPrepare {
  if (profile.role !== role || !profile.basis || draft.pending || draft.baseSource !== profile.source || draft.baseEnabled !== profile.enabled) throw new Error("配置依据或原操作尚未核对，请保留输入并重新读取。");
  const advanced = draft.source !== draft.baseSource;
  if (advanced && Object.keys(draft.changes).length) throw new Error("完整源码与控件输入同时有改动，请明确选择要准备的原文，尚未发送操作。");
  const changes = libraryChanges(profile.controls ?? [], draft.changes);
  if (profile.installed && !advanced && !Object.keys(changes).length && draft.enabled === profile.enabled) throw new Error("没有待提交的配置改动。");
  if (role !== "style") { try { JSON.parse(draft.source); } catch { throw new Error("配置 JSON 尚未完成，原输入保留，尚未准备操作。"); } }
  return { role, basis: profile.basis, operationId, ...(role !== "style" || advanced || !profile.installed ? { source: draft.source } : Object.keys(changes).length ? { changes } : {}), ...(role === "style" && draft.enabled !== profile.enabled ? { enabled: draft.enabled } : {}) };
}
export function decodeLibraryDraft(raw: string, role: LibraryRole): LibraryDraft {
  const value = JSON.parse(raw) as LibraryDraft;
  if (!value || typeof value.source !== "string" || typeof value.baseSource !== "string" || !Number.isSafeInteger(value.revision) || value.revision < 0 || !value.changes || typeof value.changes !== "object" || Array.isArray(value.changes) || Object.values(value.changes).some(input => typeof input !== "string" && typeof input !== "boolean") || value.enabled !== undefined && typeof value.enabled !== "boolean" || value.baseEnabled !== undefined && typeof value.baseEnabled !== "boolean") throw new Error("配置恢复记录无法解析，原文保留供复制与核对。");
  if (value.pending && (value.pending.request?.role !== role || !value.pending.request.operationId || !value.pending.request.basis || !value.pending.input || !Number.isSafeInteger(value.pending.revision))) throw new Error("配置原操作恢复记录不对应此角色，原记录保留。");
  return value;
}
