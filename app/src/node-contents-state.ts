export type ContentSort = "name" | "path" | "authored";
export interface ContentsPreference { sort: ContentSort; thumbnails: boolean; summaries: boolean; density: "compact" | "comfortable" }
export const defaultContentsPreference = (): ContentsPreference => ({ sort: "name", thumbnails: false, summaries: false, density: "compact" });
export function decodeContentsPreference(value: unknown): ContentsPreference {
  const item = value as Partial<ContentsPreference> | null, defaults = defaultContentsPreference();
  return { sort: item && ["name", "path", "authored"].includes(item.sort ?? "") ? item.sort! : defaults.sort,
    thumbnails: typeof item?.thumbnails === "boolean" ? item.thumbnails : defaults.thumbnails,
    summaries: typeof item?.summaries === "boolean" ? item.summaries : defaults.summaries,
    density: item?.density === "comfortable" ? "comfortable" : "compact" };
}
export interface AuthoredMember { targetRef: string; path: string; name: string }
export interface AuthoredOrder { parentRef: string; baseId: string; beforeSource: string | null; members: AuthoredMember[]; orderedRefs: string[]; complete: boolean }
export interface OrderPlan { operationId: string; planId: string; status: string; changes?: { path: string; action: string; bytes: number }[] }
export interface PendingOrder { request: { root: string; parentRef: string; children: string[]; baseId: string; operationId: string }; beforeRefs: string[]; plan?: OrderPlan; attempted?: boolean; status: string }
export function movedSibling(refs: readonly string[], target: string, offset: -1 | 1): string[] {
  const index = refs.indexOf(target), next = index + offset;
  if (index < 0 || next < 0 || next >= refs.length) return [...refs];
  const result = [...refs]; [result[index], result[next]] = [result[next], result[index]]; return result;
}
export function insertedSibling(refs: readonly string[], source: string, target: string, after: boolean): string[] {
  if (source === target || !refs.includes(source) || !refs.includes(target)) return [...refs];
  const result = refs.filter(ref => ref !== source); result.splice(result.indexOf(target) + Number(after), 0, source); return result;
}
export function decodePendingOrder(raw: string, root: string, parentRef: string): PendingOrder {
  if (raw.length > 1024 * 1024) throw new Error("原作者顺序操作记录超出预算");
  const value = JSON.parse(raw) as PendingOrder, request = value?.request;
  if (!request || request.root !== root || request.parentRef !== parentRef || !request.baseId || !request.operationId || !Array.isArray(request.children)
    || request.children.length > 5000 || request.children.some(ref => typeof ref !== "string" || !/^weftext-(node|directory):\/\//.test(ref)) || new Set(request.children).size !== request.children.length
    || !Array.isArray(value.beforeRefs) || value.beforeRefs.length !== request.children.length || new Set(value.beforeRefs).size !== value.beforeRefs.length || value.beforeRefs.some(ref => !request.children.includes(ref))
    || typeof value.status !== "string" || value.plan && (value.plan.operationId !== request.operationId || !value.plan.planId)) throw new Error("原作者顺序操作不对应当前父级或完整输入，未套用到同名对象");
  return value;
}
