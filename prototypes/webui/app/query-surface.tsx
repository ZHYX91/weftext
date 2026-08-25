"use client";

import { useMemo, useState } from "react";

type QuerySource = "nodes" | "tasks" | "headings" | "templates";
type BuilderQuerySource = Extract<QuerySource, "nodes" | "tasks">;
type QueryCellValue = { kind: string; value?: unknown };
type QueryColumn = { outputName: string; path: string; field: string; propertyKey: string | null; valueType: string; nullable: boolean };
type QueryTaskEvidence =
  | { kind: "checklist"; ownerNodeId: string; revision: string; itemRange: { start: number; end: number } }
  | { kind: "node"; nodeId: string; revision: string };
type QueryIdentity =
  | { kind: "node"; nodeId: string; revision: string }
  | { kind: "task"; evidence: QueryTaskEvidence }
  | { kind: "heading"; nodeId: string; revision: string; range: { start: number; end: number } };
type QueryResult = {
  source: QuerySource;
  columns: QueryColumn[];
  rows: Array<{ identity: QueryIdentity; cells: Array<{ column: QueryColumn; value: QueryCellValue }> }>;
  groups: Array<{ column: QueryColumn; value: QueryCellValue; rowCount: number }>;
  totalBeforeLimit: number;
  truncated: boolean;
};
type QueryBlock = {
  source: QuerySource | null;
  view: string | null;
  body: string;
  range: { start: number; end: number };
  valid: boolean;
};
type QueryDiagnostic = { code: string; message: string; range: { start: number; end: number } };
type QueryExecution = {
  blockIndex: number;
  analysis: { blocks: QueryBlock[]; diagnostics: QueryDiagnostic[] };
  result: QueryResult | null;
  csv: string | null;
};

type QuerySurfaceProps = {
  enabled: boolean;
  nodeId: string;
  documentSource: string;
  request(path: string, body?: unknown): Promise<Record<string, unknown>>;
  onOpenNode(nodeId: string): Promise<void>;
};

const nodeBody = "from nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n";
const taskBody = "from tasks as task\nscope workspace\nwhere task.closed = false\nselect task.title, task.owner_node.name, task.closed, task.state, task.priority, task.due\norder by task.due asc nulls last, task.priority desc\nlimit 100\n";

function localDate(date = new Date()) {
  return {
    year: date.getFullYear(),
    month: date.getMonth() + 1,
    day: date.getDate(),
  };
}

function localInstant(date = new Date()) {
  const pad = (value: number, width = 2) => String(value).padStart(width, "0");
  const offsetMinutes = -date.getTimezoneOffset();
  const sign = offsetMinutes < 0 ? "-" : "+";
  const absoluteOffset = Math.abs(offsetMinutes);
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`
    + `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`
    + `${sign}${pad(Math.floor(absoluteOffset / 60))}:${pad(absoluteOffset % 60)}`;
}

export function canonicalQuerySource(source: BuilderQuerySource, body: string) {
  const normalized = body.endsWith("\n") ? body : `${body}\n`;
  return `[.weftext-query,version=1,view=${source === "tasks" ? "task-list" : "table"}]\n....\n${normalized}....\n`;
}

function cellText(cell: QueryCellValue) {
  if (cell.kind === "null" || cell.value === undefined || cell.value === null) return "";
  if (typeof cell.value === "object") {
    const nested = cell.value as { value?: unknown };
    return nested.value === undefined || nested.value === null ? "" : String(nested.value);
  }
  return String(cell.value);
}

const fieldLabels: Record<string, string> = {
  id: "ID",
  name: "名称",
  path: "路径",
  parent_id: "父节点 ID",
  depth: "深度",
  "owner_node.id": "节点 ID",
  "owner_node.name": "节点",
  "owner_node.path": "节点路径",
  title: "任务",
  closed: "已完成",
  state: "状态",
  priority: "优先级",
  created: "创建",
  start: "开始",
  scheduled: "计划",
  due: "截止",
  closed_at: "完成时间",
};

export function downloadQueryCsv(csv: string, source: QuerySource, date = new Date()) {
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8" });
  const href = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = `weftext-${source}-${date.toISOString().slice(0, 10)}.csv`;
  anchor.click();
  URL.revokeObjectURL(href);
}

function rowKey(row: QueryResult["rows"][number], index: number) {
  if (row.identity.kind === "task") {
    const evidence = row.identity.evidence;
    return evidence.kind === "checklist"
      ? `${evidence.ownerNodeId}-${evidence.itemRange.start}-${index}`
      : `${evidence.nodeId}-task-node`;
  }
  return row.identity.kind === "heading"
    ? `${row.identity.nodeId}-heading-${row.identity.range.start}`
    : row.identity.nodeId;
}

function identityNode(identity: QueryIdentity) {
  if (identity.kind !== "task") return { nodeId: identity.nodeId, revision: identity.revision };
  return identity.evidence.kind === "checklist"
    ? { nodeId: identity.evidence.ownerNodeId, revision: identity.evidence.revision }
    : { nodeId: identity.evidence.nodeId, revision: identity.evidence.revision };
}

export default function QuerySurface({ enabled, nodeId, documentSource, request, onOpenNode }: QuerySurfaceProps) {
  const [mode, setMode] = useState<"builder" | "embedded">("builder");
  const [source, setSource] = useState<BuilderQuerySource>("nodes");
  const [body, setBody] = useState(nodeBody);
  const [blockIndex, setBlockIndex] = useState(0);
  const [execution, setExecution] = useState<QueryExecution | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const exactSource = useMemo(
    () => mode === "builder" ? canonicalQuerySource(source, body) : documentSource,
    [body, documentSource, mode, source],
  );

  function chooseMode(next: "builder" | "embedded") {
    setMode(next);
    setBlockIndex(0);
    setExecution(null);
    setError("");
  }

  function chooseSource(next: BuilderQuerySource) {
    setSource(next);
    setBody(next === "nodes" ? nodeBody : taskBody);
    setExecution(null);
  }

  async function execute() {
    if (!enabled) return;
    setLoading(true);
    try {
      const payload = await request("/api/query/execute", {
        source: exactSource,
        blockIndex: mode === "builder" ? 0 : blockIndex,
        context: {
          today: localDate(),
          now: localInstant(),
          timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
          locale: navigator.language,
          binding: { nodeId, heading: null },
        },
      });
      setExecution(payload.execution as QueryExecution);
      setError("");
    } catch (reason) {
      setExecution(null);
      setError(reason instanceof Error ? reason.message : "Core 无法执行查询");
    } finally {
      setLoading(false);
    }
  }

  if (!enabled) return <p className="query-empty">查询与视图只对已连接的 Weftext AsciiDoc 工作区开放。</p>;

  const result = execution?.result ?? null;
  return <div className="query-surface">
    <aside className="query-composer" aria-label="查询编辑器">
      <nav className="query-mode-tabs" aria-label="查询来源">
        <button type="button" aria-current={mode === "builder" ? "page" : undefined} onClick={() => chooseMode("builder")}>新查询</button>
        <button type="button" aria-current={mode === "embedded" ? "page" : undefined} onClick={() => chooseMode("embedded")}>当前文档内嵌块</button>
      </nav>
      {mode === "builder" ? <>
        <label>数据源<select value={source} onChange={(event) => chooseSource(event.target.value as BuilderQuerySource)}><option value="nodes">nodes · 节点</option><option value="tasks">tasks · 任务</option></select></label>
        <label className="query-body-label">规范查询体<textarea aria-label="规范查询体" spellCheck={false} value={body} onChange={(event) => { setBody(event.target.value); setExecution(null); }} /></label>
        <details><summary>精确 portable query 源码</summary><pre>{exactSource}</pre></details>
      </> : <>
        <p>由 Core 从当前精确文档源码识别 `[.weftext-query,version=1,...]` 块；客户端不解析块或字段。</p>
        {execution?.analysis.blocks.length ? <label>查询块<select aria-label="查询块" value={blockIndex} onChange={(event) => {
          setBlockIndex(Number(event.target.value));
          setExecution((current) => current ? { ...current, result: null, csv: null } : current);
        }}>{execution.analysis.blocks.map((block, index) => <option value={index} key={`${block.range.start}-${index}`}>#{index + 1} · {block.source ?? "invalid"} · {block.view ?? "default"}</option>)}</select></label> : null}
      </>}
      <button className="query-run" type="button" disabled={loading} onClick={() => void execute()}>{loading ? "Core 正在执行…" : mode === "embedded" ? "解析并运行所选块" : "运行 Core 查询"}</button>
      {error && <p className="query-error" role="alert">{error}</p>}
      {execution && !result && !execution.analysis.diagnostics.length && <p className="query-error" role="alert">所选编号没有可执行的规范查询块。</p>}
      {execution?.analysis.diagnostics.length ? <div className="query-diagnostics" aria-label="查询诊断">{execution.analysis.diagnostics.map((diagnostic, index) => <div key={`${diagnostic.code}-${diagnostic.range.start}-${index}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>字节 {diagnostic.range.start}–{diagnostic.range.end}</small></div>)}</div> : null}
    </aside>
    <section className="query-results" aria-label="查询结果">
      <header>
        <div><span className="eyebrow">AUTHORIZED CORE RESULT</span><h3>{result ? `${result.source === "nodes" ? "节点" : "任务"}表格` : "查询结果"}</h3></div>
        <div className="query-result-actions"><span role="status">{result ? `${result.rows.length} / ${result.totalBeforeLimit} 行${result.truncated ? " · 已按 limit 截断" : ""}` : "尚未运行"}</span><button type="button" disabled={!execution?.csv || !result} onClick={() => execution?.csv && result && downloadQueryCsv(execution.csv, result.source)}>导出 Core CSV</button></div>
      </header>
      <div className="query-result-body">
        {result?.groups.length ? <div className="query-groups" aria-label="分组统计">{result.groups.map((group, index) => <span key={`${group.column.outputName}-${cellText(group.value)}-${index}`}><strong>{group.column.outputName}: {cellText(group.value) || "空值"}</strong>{group.rowCount} 行</span>)}</div> : null}
        {result ? <div className="query-table-scroll" role="region" aria-label="只读查询表格"><table>
          <thead><tr>{result.columns.map((column) => <th scope="col" key={column.outputName}>{fieldLabels[column.outputName] ?? column.outputName}<small>{column.path}</small></th>)}<th scope="col">来源</th></tr></thead>
          <tbody>{result.rows.map((row, index) => <tr key={rowKey(row, index)}>{result.columns.map((column) => {
            const value = row.cells.find((cell) => cell.column.path === column.path)?.value ?? { kind: "null" };
            return <td key={column.outputName} data-null={value.kind === "null" || undefined}>{cellText(value) || <span aria-label="空值">—</span>}</td>;
          })}<td>{(() => { const owner = identityNode(row.identity); return <><button type="button" onClick={() => void onOpenNode(owner.nodeId)}>打开节点</button><small>{owner.revision.slice(0, 10)}…</small></>; })()}</td></tr>)}</tbody>
        </table>{!result.rows.length && <p className="query-empty">查询有效，但没有授权范围内的匹配行。</p>}</div> : <div className="query-welcome"><strong>一个查询语言，多种派生视图</strong><p>先运行内置节点或任务模板，也可以执行当前文档中的规范查询块。所有筛选、排序、分组、计数与 limit 都由 Core 在权限过滤之后完成。</p></div>}
      </div>
    </section>
  </div>;
}
