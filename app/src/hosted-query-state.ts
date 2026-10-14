import type { QueryResult, QueryRow } from "./query-panel";

export type HostedQueryRow = QueryRow & { resource?: string; canOpen?: boolean };
export type HostedQueryResult = QueryResult & { rows: HostedQueryRow[]; columns: string[]; queryId: string };
export interface HostedQueryResponse { queryId: string; columns: string[]; result: QueryResult & { rows: HostedQueryRow[] } }

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
