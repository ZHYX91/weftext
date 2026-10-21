import { TagTree, type TreeTag } from "./tag-tree";
export type RegistryKind = "admitDirectory" | "createTag" | "renameTag" | "moveTag";
export type RegistryIntent = { kind: "admitDirectory"; slot: string } | { kind: "createTag"; basis: string; displayName: string; parentRef: string | null; assignTo?: { resource: string; fieldId: string; basis: string } } | { kind: "renameTag"; basis: string; reference: string; displayName: string } | { kind: "moveTag"; basis: string; reference: string; parentRef: string | null };
export type RegistryRequest = { operationId: string; intent: RegistryIntent };
export type RegistryResult = { kind: "directory" | "tag"; reference: string; status: string; canResolve?: boolean; title?: string; readOnly?: boolean };
export type RegistryReceipt = { operationId: string; status: string; planId?: string | null; preview: { kind?: string; title?: string; displayName?: string; beforeName?: string; parentRef?: string | null; beforeParentRef?: string | null }; commitAllowed: boolean; navigationRef?: string | null; result?: RegistryResult | null };
export type RegistryDraft = { kind: RegistryKind; slot: string; reference: string; displayName: string; parentRef: string | null };
export type RegistryCatalog = { directoryScope: "configuredAuthorizedDirectories"; directoryCoverage: "complete" | "partial"; directories: { directory: string; title: string; reference: string; status: "available" | "unavailable"; canResolve: boolean }[]; admissionSlots: { slot: string; title: string }[]; tags: { read: boolean; create: boolean; rename: boolean; move: boolean; coverage: "complete" | "partial" | "unavailable"; items: TreeTag[]; basis: string | null } };
export function registryIntent(draft: RegistryDraft, catalog: RegistryCatalog): RegistryIntent {
  if (draft.kind === "admitDirectory") { if (!catalog.admissionSlots.some(slot => slot.slot === draft.slot)) throw new Error("请明确选择获权目录纳入槽位。"); return { kind: draft.kind, slot: draft.slot }; }
  if (!catalog.tags.basis || !catalog.tags.read || catalog.tags.coverage !== "complete") throw new Error("完整标签登记依据不可用，原输入保留。");
  const tree = new TagTree(catalog.tags.items, true), tag = draft.kind === "createTag" ? undefined : tree.byRef.get(draft.reference);
  if (draft.kind !== "createTag" && !tag) throw new Error("原标签当前不可用，请明确重新选择。");
  if (draft.kind === "renameTag") { if (!catalog.tags.rename || !draft.displayName.trim()) throw new Error("标签改名未获权或名称不可用。"); return { kind: draft.kind, basis: catalog.tags.basis, reference: draft.reference, displayName: draft.displayName }; }
  if (draft.parentRef !== null) { const parent = tree.byRef.get(draft.parentRef); if (!parent || tag && tree.contains(tag.tagId, parent.tagId)) throw new Error("父级当前不可用或属于标签自身／后代，原输入保留。"); }
  if (draft.kind === "createTag") { if (!catalog.tags.create || !draft.displayName.trim()) throw new Error("新建标签未获权或名称不可用。"); return { kind: draft.kind, basis: catalog.tags.basis, displayName: draft.displayName, parentRef: draft.parentRef }; }
  if (!catalog.tags.move) throw new Error("标签换父没有独立 move 权限。");
  return { kind: draft.kind, basis: catalog.tags.basis, reference: draft.reference, parentRef: draft.parentRef };
}
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export function registryReceipt(request: RegistryRequest, incoming: RegistryReceipt, previous?: RegistryReceipt): RegistryReceipt {
  if (!incoming || incoming.operationId !== request.operationId || !statuses.has(incoming.status) || typeof incoming.commitAllowed !== "boolean" || !incoming.preview || typeof incoming.preview !== "object" || incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId)) throw new Error("导航回执与原操作不符，未采用替换计划。");
  if (previous?.planId && incoming.planId && incoming.planId !== previous.planId) throw new Error("原导航计划编号发生变化，请继续核对原操作。");
  if (previous?.status === "committed" && incoming.status !== "committed") throw new Error("已提交导航操作不能退回重放状态。");
  for (const key of ["kind", "title", "displayName", "beforeName"] as const) if (incoming.preview[key] !== undefined && typeof incoming.preview[key] !== "string") throw new Error("导航预览格式无法确认，未采用结果。");
  for (const key of ["parentRef", "beforeParentRef"] as const) if (incoming.preview[key] !== undefined && incoming.preview[key] !== null && typeof incoming.preview[key] !== "string") throw new Error("导航父级预览格式无法确认，未采用结果。");
  if (incoming.result && (incoming.status !== "committed" || !["tag", "directory"].includes(incoming.result.kind) || typeof incoming.result.reference !== "string")) throw new Error("导航目标没有准确已提交资格，未采用目标。");
  // A filtered receipt never inherits an old private target/name from cache.
  return incoming;
}
