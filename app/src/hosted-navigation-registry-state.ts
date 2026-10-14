export type RegistryKind = "admitDirectory" | "createTag" | "renameTag";
export type RegistryIntent = { kind: "admitDirectory"; slot: string } | { kind: "createTag"; basis: string; displayName: string } | { kind: "renameTag"; basis: string; reference: string; displayName: string };
export type RegistryRequest = { operationId: string; intent: RegistryIntent };
export type RegistryResult = { kind: "directory" | "tag"; reference: string; status: string; canResolve?: boolean; title?: string; readOnly?: boolean };
export type RegistryReceipt = { operationId: string; status: string; planId?: string | null; preview: { kind?: string; title?: string; displayName?: string; beforeName?: string }; commitAllowed: boolean; navigationRef?: string | null; result?: RegistryResult | null };
export type RegistryDraft = { kind: RegistryKind; slot: string; reference: string; displayName: string };
export type RegistryCatalog = { directoryScope: "configuredAuthorizedDirectories"; directoryCoverage: "complete" | "partial"; directories: { directory: string; title: string; reference: string; status: "available" | "unavailable"; canResolve: boolean }[]; admissionSlots: { slot: string; title: string }[]; tags: { read: boolean; create: boolean; rename: boolean; coverage: "complete" | "partial" | "unavailable"; items: { reference: string; displayName: string }[]; basis: string | null } };
const statuses = new Set(["notRegistered", "notCommitted", "prepared", "committed", "recoveryPending", "resolved", "cancelled"]);
export function registryReceipt(request: RegistryRequest, incoming: RegistryReceipt, previous?: RegistryReceipt): RegistryReceipt {
  if (!incoming || incoming.operationId !== request.operationId || !statuses.has(incoming.status) || typeof incoming.commitAllowed !== "boolean" || !incoming.preview || typeof incoming.preview !== "object" || incoming.commitAllowed && (incoming.status !== "prepared" || !incoming.planId)) throw new Error("导航回执与原操作不符，未采用替换计划。");
  if (previous?.planId && incoming.planId && incoming.planId !== previous.planId) throw new Error("原导航计划编号发生变化，请继续核对原操作。");
  if (previous?.status === "committed" && incoming.status !== "committed") throw new Error("已提交导航操作不能退回重放状态。");
  for (const key of ["kind", "title", "displayName", "beforeName"] as const) if (incoming.preview[key] !== undefined && typeof incoming.preview[key] !== "string") throw new Error("导航预览格式无法确认，未采用结果。");
  if (incoming.result && (incoming.status !== "committed" || !["tag", "directory"].includes(incoming.result.kind) || typeof incoming.result.reference !== "string")) throw new Error("导航目标没有准确已提交资格，未采用目标。");
  // A filtered receipt never inherits an old private target/name from cache.
  return incoming;
}
