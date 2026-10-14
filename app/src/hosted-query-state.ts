import type { QueryResult, QueryRow } from "./query-panel";
import { parseQueryJson, parseQueryPlan, type QueryPlan } from "./query-panel";

export type HostedQueryRow = QueryRow & { resource?: string; canOpen?: boolean };
export type HostedQueryResult = QueryResult & { rows: HostedQueryRow[]; columns: string[]; queryId: string };
export interface HostedQueryResponse { queryId: string; columns: string[]; result: QueryResult & { rows: HostedQueryRow[] } }
export interface HostedProviderDescriptor { id: string; fingerprint: string; implementation: { id: string; abi: number }; data: { kind: string; key?: string; version?: string }; limits: { maxSteps: number; maxBytes: number; maxExpansion: number }; queryNames: string[] }
export interface HostedSeriesPolicyDescriptor { id: string; fingerprint: string; key: string; kinds: string[]; rule: string; calendar: string | null; zone: string | null; status: "available" | "unavailable" }
export interface HostedQueryDependencies { providers: Record<string, string>; seriesPolicies: Record<string, string> }
export interface HostedDirectoryDescriptor { id: string; title: string; reference: string; kind: "node" | "directory"; lifecycle?: "live" | "trash"; status: "available" | "unavailable" }
export interface HostedScopeCatalog { directories?: HostedDirectoryDescriptor[]; trash?: boolean; scopeCoverage?: "complete" | "partial" }
export function hostedScopeDraft(value: unknown): QueryPlan["scope"] | undefined {
  if (value === undefined) return undefined;
  if (!value || typeof value !== "object" || Array.isArray(value) || Object.keys(value).some(key => !["directory", "descendants", "lifecycle"].includes(key))) throw new Error("范围恢复输入无效，原记录保留。");
  const scope = value as QueryPlan["scope"]; if (typeof scope.descendants !== "boolean" || !["live", "trash"].includes(scope.lifecycle) || scope.directory !== undefined && typeof scope.directory !== "string") throw new Error("范围恢复输入无效，原记录保留。");
  return { ...scope };
}
export function applyHostedScope(source: string, scope: QueryPlan["scope"]): string {
  const plan = parseQueryPlan(source); if ((plan.advanced?.call || plan.advanced?.unionAll) && (scope.directory || !scope.descendants || scope.lifecycle !== "live")) throw new Error("此计划由保存定义调用或 union 生成，外层范围须保持默认；请在实际扫描分支或保存定义内设置范围。原输入与嵌套计划保留。");
  const raw = parseQueryJson(source) as Record<string, unknown>, changed = { ...(raw.scope as object ?? {}), descendants: scope.descendants, lifecycle: scope.lifecycle } as QueryPlan["scope"];
  if (scope.directory) changed.directory = scope.directory; else delete changed.directory;
  return JSON.stringify({ ...raw, scope: changed }, null, 2);
}
export function exactHostedScope(scope: QueryPlan["scope"], catalog: HostedScopeCatalog): void {
  if (scope.lifecycle === "trash" && catalog.trash !== true) throw new Error("原回收站范围缺少独立读取资格；输入保留，未改用在库范围。");
  if (!scope.directory) return;
  const directory = catalog.directories?.find(item => item.id === scope.directory || item.reference === scope.directory);
  if (!directory || directory.status !== "available") throw new Error(`原目录范围 ${scope.directory} 当前不可用或未获权；选择与计划保留，未采用同名目标。`);
  if ((directory.lifecycle ?? "live") !== scope.lifecycle) throw new Error("所选目录与查询生命周期不同，请明确选择匹配范围；未自动改用另一生命周期。");
}
export function hostedDependencyMap(value: unknown): Record<string, string> {
  if (value === undefined) return {};
  if (!value || typeof value !== "object" || Array.isArray(value) || Object.entries(value).some(([id, fingerprint]) => !id || typeof fingerprint !== "string" || !/^[0-9a-f]{64}$/.test(fingerprint))) throw new Error("Provider 恢复选择缺少准确指纹，原记录保留，未改用当前版本。");
  return Object.fromEntries(Object.entries(value));
}
export function exactHostedDependencies(selection: HostedQueryDependencies, catalog: { providers?: Pick<HostedProviderDescriptor, "id" | "fingerprint">[]; seriesPolicies?: Pick<HostedSeriesPolicyDescriptor, "id" | "fingerprint">[] }): HostedQueryDependencies {
  const result = { providers: hostedDependencyMap(selection.providers), seriesPolicies: hostedDependencyMap(selection.seriesPolicies) };
  for (const kind of ["providers", "seriesPolicies"] as const) for (const [id, fingerprint] of Object.entries(result[kind])) {
    const current = catalog[kind]?.find(item => item.id === id);
    if (!current || current.fingerprint !== fingerprint) throw new Error(`原${kind === "providers" ? " Provider" : "系列政策"}选择 ${id} 当前不可用或已改版，请明确重新选择；未采用最新版本。`);
  }
  return result;
}

/** A frozen result has one exact column order, including complete empty results. */
export function decodeHostedQuery(response: HostedQueryResponse): HostedQueryResult {
  const result = response?.result, columns = response?.columns;
  if (!response?.queryId || !Array.isArray(columns) || columns.some(column => typeof column !== "string") || new Set(columns).size !== columns.length || !result || !Array.isArray(result.rows) || !Array.isArray(result.diagnostics) || !["complete", "partial"].includes(result.coverage) || !["current", "stale"].includes(result.freshness) || !Number.isSafeInteger(result.totalKnown) || result.totalKnown < result.rows.length || typeof result.limited !== "boolean") throw new Error("托管查询结果缺少准确列序或范围状态，未安装新结果。");
  if (result.queryId !== undefined && result.queryId !== response.queryId || result.columns !== undefined && JSON.stringify(result.columns) !== JSON.stringify(columns)) throw new Error("托管查询冻结编号或列序不一致，未安装替换结果。");
  for (const row of result.rows) if (!row || !row.cells || typeof row.cells !== "object" || Object.entries(row.cells).some(([column, cell]) => !columns.includes(column) || !cell || !["value", "absent", "null", "invalid", "unavailable"].includes(cell.state)) || row.canOpen && !row.resource) throw new Error("托管查询行缺少准确投影或导航依据，未安装结果。");
  return { ...result, columns: [...columns], queryId: response.queryId };
}

/** Input edits, ABA plan changes and actor reset independently invalidate delivery. */
export class HostedQueryGate {
  private epoch = 0;
  private generation = 0;
  private revision = 0;
  changed(): void { this.revision++; }
  clear(): void { this.epoch++; this.generation++; this.revision++; }
  begin(input: string) { return { epoch: this.epoch, generation: ++this.generation, revision: this.revision, input }; }
  current(token: ReturnType<HostedQueryGate["begin"]>, input: string): boolean { return token.epoch === this.epoch && token.generation === this.generation && token.revision === this.revision && token.input === input; }
}
