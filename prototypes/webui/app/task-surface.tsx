"use client";

import { useCallback, useEffect, useState } from "react";

type SourceRange = { start: number; end: number };
type TaskDateTime = { kind: "date" | "instant"; value: string };

type TaskMetadata = {
  id: string;
  phase: "todo" | "in-progress" | "on-hold" | null;
  resolution: "completed" | "cancelled" | null;
  priority: "lowest" | "low" | "normal" | "medium" | "high" | "highest";
  created: TaskDateTime | null;
  start: TaskDateTime | null;
  scheduled: TaskDateTime | null;
  due: TaskDateTime | null;
  closed: TaskDateTime | null;
  recurrence: { source: string } | null;
  repeatFrom: "due" | "scheduled" | "completion" | null;
  dependencies: string[];
};

export type TaskOccurrence = {
  nodeId: string;
  revision: string;
  task: {
    state: "open" | "closed";
    description: string;
    listDepth: number;
    range: SourceRange;
    metadata: TaskMetadata | null;
    valid: boolean;
  };
};

type TaskDiagnostic = {
  code: string;
  message: string;
  range: SourceRange;
  taskId: string | null;
  dependencyId: string | null;
};

type TaskInspection = {
  nodeId: string;
  occurrences: TaskOccurrence[];
  diagnostics: TaskDiagnostic[];
};

type TaskPlan = {
  planId: string;
  kind: "edit" | "recurrence" | "dependencies";
  baseWorkspaceRevision: string;
  nodeId: string;
  authoring?: {
    proposedSource: string;
    assignedId: string | null;
    target: TaskOccurrence["task"];
  };
  completion?: {
    proposedSource: string;
    completedTask: TaskOccurrence["task"];
    nextTask: TaskOccurrence["task"] | null;
    nextTaskId: string | null;
    stopped: boolean;
  };
  dependencies?: string[];
  documentChanges: unknown[];
};

type TaskEditIntent =
  | { kind: "toggle" }
  | { kind: "set_priority"; priority: TaskMetadata["priority"] | null }
  | { kind: "set_phase"; phase: TaskMetadata["phase"] }
  | { kind: "set_resolution"; resolution: TaskMetadata["resolution"] }
  | { kind: "set_date"; field: "created" | "start" | "scheduled" | "due" | "closed"; value: TaskDateTime | null }
  | { kind: "set_recurrence"; rrule: string | null; repeat_from: TaskMetadata["repeatFrom"] };

type TaskRequest = (path: string, body?: unknown) => Promise<Record<string, unknown>>;

type TaskSurfaceProps = {
  enabled: boolean;
  nodeId: string;
  workspaceRevision: string;
  documentRevision: string;
  blockedReason: string;
  safeMode: boolean;
  request: TaskRequest;
  onCommitted(payload: Record<string, unknown>): Promise<void>;
};

const priorities: TaskMetadata["priority"][] = ["highest", "high", "medium", "normal", "low", "lowest"];
const dateFields = ["created", "start", "scheduled", "due", "closed"] as const;

function taskKey(occurrence: TaskOccurrence) {
  return occurrence.task.metadata?.id ?? `${occurrence.task.range.start}-${occurrence.task.range.end}`;
}

function taskTarget(occurrence: TaskOccurrence) {
  return occurrence.task.metadata
    ? { kind: "id", id: occurrence.task.metadata.id }
    : { kind: "occurrence", range: occurrence.task.range };
}

function localDateValue(date = new Date()) {
  return `${date.getFullYear().toString().padStart(4, "0")}-${(date.getMonth() + 1).toString().padStart(2, "0")}-${date.getDate().toString().padStart(2, "0")}`;
}

function planSource(plan: TaskPlan) {
  return plan.authoring?.proposedSource ?? plan.completion?.proposedSource ?? "";
}

export default function TaskSurface({ enabled, nodeId, workspaceRevision, documentRevision, blockedReason, safeMode, request, onCommitted }: TaskSurfaceProps) {
  const [inspection, setInspection] = useState<TaskInspection | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [plan, setPlan] = useState<TaskPlan | null>(null);
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [priority, setPriority] = useState<TaskMetadata["priority"]>("normal");
  const [phase, setPhase] = useState<"todo" | "in-progress" | "on-hold">("todo");
  const [resolution, setResolution] = useState<"completed" | "cancelled">("completed");
  const [dateField, setDateField] = useState<(typeof dateFields)[number]>("due");
  const [dateKind, setDateKind] = useState<"date" | "instant">("date");
  const [dateValue, setDateValue] = useState("");
  const [rrule, setRrule] = useState("");
  const [repeatFrom, setRepeatFrom] = useState<"due" | "scheduled" | "completion">("due");
  const [dependencies, setDependencies] = useState("");

  const loadInspection = useCallback(async () => {
    if (!enabled) {
      setInspection(null);
      return;
    }
    setLoading(true);
    try {
      const payload = await request(`/api/task/inspect?nodeId=${encodeURIComponent(nodeId)}`);
      setInspection(payload as unknown as TaskInspection);
      setError("");
    } catch (reason) {
      setInspection(null);
      setError(reason instanceof Error ? reason.message : "Core 无法读取任务投影");
    } finally {
      setLoading(false);
    }
  }, [enabled, nodeId, request]);

  useEffect(() => {
    let active = true;
    void Promise.resolve().then(async () => {
      if (!enabled) {
        if (active) setInspection(null);
        return;
      }
      try {
        const payload = await request(`/api/task/inspect?nodeId=${encodeURIComponent(nodeId)}`);
        if (active) {
          setInspection(payload as unknown as TaskInspection);
          setError("");
        }
      } catch (reason) {
        if (active) {
          setInspection(null);
          setError(reason instanceof Error ? reason.message : "Core 无法读取任务投影");
        }
      }
    });
    return () => {
      active = false;
    };
  }, [documentRevision, enabled, nodeId, request]);

  function selectTask(occurrence: TaskOccurrence) {
    const metadata = occurrence.task.metadata;
    setSelectedKey(taskKey(occurrence));
    setPriority(metadata?.priority ?? "normal");
    setPhase(metadata?.phase ?? "todo");
    setResolution(metadata?.resolution ?? "completed");
    setDateField("due");
    setDateKind(metadata?.due?.kind ?? "date");
    setDateValue(metadata?.due?.value ?? "");
    setRrule(metadata?.recurrence?.source ?? "");
    setRepeatFrom(metadata?.repeatFrom ?? "due");
    setDependencies(metadata?.dependencies.join(" ") ?? "");
    setError("");
  }

  async function requestPlan(path: string, body: Record<string, unknown>) {
    if (blockedReason) {
      setError(blockedReason);
      return;
    }
    setLoading(true);
    try {
      const payload = await request(path, body);
      setPlan(payload.plan as TaskPlan);
      setError("");
    } catch (reason) {
      setPlan(null);
      setError(reason instanceof Error ? reason.message : "Core 拒绝了任务预览");
    } finally {
      setLoading(false);
    }
  }

  async function previewEdit(occurrence: TaskOccurrence, intent: TaskEditIntent) {
    await requestPlan("/api/task/edit-preview", {
      nodeId,
      baseWorkspaceRevision: workspaceRevision,
      baseRevision: documentRevision,
      target: taskTarget(occurrence),
      intent,
    });
  }

  async function previewToggle(occurrence: TaskOccurrence) {
    if (occurrence.task.metadata?.recurrence) {
      await requestPlan("/api/task/recurrence-preview", {
        nodeId,
        baseWorkspaceRevision: workspaceRevision,
        baseRevision: documentRevision,
        target: taskTarget(occurrence),
        context: {
          completedAt: { kind: "date", value: localDateValue() },
          utcOffsetMinutes: -new Date().getTimezoneOffset(),
        },
      });
      return;
    }
    await previewEdit(occurrence, { kind: "toggle" });
  }

  async function previewDependencies(occurrence: TaskOccurrence) {
    const ids = dependencies.split(/[\s,]+/u).map((value) => value.trim()).filter(Boolean);
    await requestPlan("/api/task/dependencies-preview", {
      nodeId,
      baseWorkspaceRevision: workspaceRevision,
      baseRevision: documentRevision,
      target: taskTarget(occurrence),
      dependencies: ids,
    });
  }

  async function commitPlan() {
    if (!plan || safeMode || blockedReason) return;
    setLoading(true);
    try {
      const payload = await request("/api/task/transaction/commit", { planId: plan.planId });
      setPlan(null);
      setSelectedKey(null);
      await onCommitted(payload);
      setError("");
    } catch (reason) {
      setPlan(null);
      setError(reason instanceof Error ? reason.message : "Core 拒绝了任务事务提交");
    } finally {
      setLoading(false);
    }
  }

  if (!enabled) return <p className="empty-properties">任务检查器只对已连接的 Weftext AsciiDoc 工作区开放。</p>;

  return <div className="task-surface">
    <div className="task-surface-status" role="status">
      <span>Core 任务投影</span>
      <strong>{loading ? "正在同步…" : `${inspection?.occurrences.length ?? 0} 项 · ${inspection?.diagnostics.length ?? 0} 个诊断`}</strong>
      <button type="button" disabled={loading} onClick={() => void loadInspection()}>刷新</button>
    </div>
    {blockedReason && <p className="task-blocked" role="alert">{blockedReason}</p>}
    {inspection?.diagnostics.length ? <div className="task-diagnostics" aria-label="任务诊断">{inspection.diagnostics.map((diagnostic, index) => <div key={`${diagnostic.code}-${diagnostic.range.start}-${index}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>字节 {diagnostic.range.start}–{diagnostic.range.end}</small></div>)}</div> : null}
    <div className="task-list" aria-label="当前节点任务">
      {inspection?.occurrences.map((occurrence) => {
        const metadata = occurrence.task.metadata;
        const key = taskKey(occurrence);
        const selected = selectedKey === key;
        return <article className={`task-card ${occurrence.task.state}`} key={key}>
          <div className="task-card-main">
            <button
              type="button"
              className="task-toggle"
              aria-label={`${occurrence.task.state === "open" ? "完成" : "重新打开"}任务：${occurrence.task.description}`}
              aria-pressed={occurrence.task.state === "closed"}
              disabled={loading || Boolean(blockedReason) || !occurrence.task.valid}
              onClick={() => void previewToggle(occurrence)}
            >{occurrence.task.state === "closed" ? "✓" : ""}</button>
            <span><strong>{occurrence.task.description}</strong><small>{metadata ? `${metadata.priority} · ${metadata.id}` : "简单任务 · 修订范围身份"}</small></span>
            <button type="button" aria-expanded={selected} onClick={() => selected ? setSelectedKey(null) : selectTask(occurrence)}>编辑</button>
          </div>
          {metadata && <div className="task-facts">
            {metadata.phase && <span>阶段 {metadata.phase}</span>}
            {metadata.resolution && <span>结果 {metadata.resolution}</span>}
            {metadata.due && <span>截止 {metadata.due.value}</span>}
            {metadata.recurrence && <span>重复 {metadata.recurrence.source}</span>}
            {metadata.dependencies.length > 0 && <span>依赖 {metadata.dependencies.length} 项</span>}
          </div>}
          {selected && <div className="task-editor" aria-label={`编辑任务 ${occurrence.task.description}`}>
            <label>优先级<select value={priority} onChange={(event) => setPriority(event.target.value as TaskMetadata["priority"])}>{priorities.map((value) => <option value={value} key={value}>{value}</option>)}</select></label>
            <button type="button" disabled={loading || Boolean(blockedReason)} onClick={() => void previewEdit(occurrence, { kind: "set_priority", priority: priority === "normal" ? null : priority })}>预览优先级</button>
            {occurrence.task.state === "open" ? <><label>阶段<select value={phase} onChange={(event) => setPhase(event.target.value as typeof phase)}><option value="todo">todo</option><option value="in-progress">in-progress</option><option value="on-hold">on-hold</option></select></label><button type="button" disabled={loading || Boolean(blockedReason)} onClick={() => void previewEdit(occurrence, { kind: "set_phase", phase: phase === "todo" ? null : phase })}>预览阶段</button></> : <><label>结果<select value={resolution} onChange={(event) => setResolution(event.target.value as typeof resolution)}><option value="completed">completed</option><option value="cancelled">cancelled</option></select></label><button type="button" disabled={loading || Boolean(blockedReason)} onClick={() => void previewEdit(occurrence, { kind: "set_resolution", resolution: resolution === "completed" ? null : resolution })}>预览结果</button></>}
            <label>日期字段<select value={dateField} onChange={(event) => setDateField(event.target.value as typeof dateField)}>{dateFields.map((field) => <option value={field} key={field}>{field}</option>)}</select></label>
            <label>日期类型<select value={dateKind} onChange={(event) => setDateKind(event.target.value as typeof dateKind)}><option value="date">date</option><option value="instant">instant</option></select></label>
            <label className="task-wide">日期值<input value={dateValue} placeholder={dateKind === "date" ? "2026-09-05" : "2026-09-05T09:00:00+08:00"} onChange={(event) => setDateValue(event.target.value)} /></label>
            <button type="button" disabled={loading || Boolean(blockedReason)} onClick={() => void previewEdit(occurrence, { kind: "set_date", field: dateField, value: dateValue.trim() ? { kind: dateKind, value: dateValue.trim() } : null })}>预览日期</button>
            <label className="task-wide">RRULE<input value={rrule} placeholder="FREQ=WEEKLY;BYDAY=MO;COUNT=10" onChange={(event) => setRrule(event.target.value)} /></label>
            <label>重复基准<select value={repeatFrom} onChange={(event) => setRepeatFrom(event.target.value as typeof repeatFrom)}><option value="due">due</option><option value="scheduled">scheduled</option><option value="completion">completion</option></select></label>
            <button type="button" disabled={loading || Boolean(blockedReason)} onClick={() => void previewEdit(occurrence, { kind: "set_recurrence", rrule: rrule.trim() || null, repeat_from: rrule.trim() ? repeatFrom : null })}>{rrule.trim() ? "预览重复规则" : "预览移除重复"}</button>
            <label className="task-wide">依赖任务 UUID<textarea value={dependencies} placeholder="以空格、逗号或换行分隔" onChange={(event) => setDependencies(event.target.value)} /></label>
            <button type="button" disabled={loading || Boolean(blockedReason) || (!metadata && !dependencies.trim())} onClick={() => void previewDependencies(occurrence)}>预览完整依赖集</button>
          </div>}
        </article>;
      })}
      {!loading && inspection?.occurrences.length === 0 && <p className="empty-properties">当前节点没有任务。</p>}
    </div>
    {error && <p className="task-error" role="alert">{error}</p>}
    {plan && <div className="modal-backdrop task-plan-backdrop" role="presentation"><section className="task-plan-dialog" role="dialog" aria-modal="true" aria-label="确认任务事务">
      <span className="eyebrow">CORE TASK PLAN</span>
      <h3>{plan.kind === "recurrence" ? "确认重复任务完成" : plan.kind === "dependencies" ? "确认完整依赖集" : "确认任务编辑"}</h3>
      <div className="task-plan-summary"><span>计划 {plan.planId}</span><span>{plan.documentChanges.length} 个文档变化</span>{plan.authoring?.assignedId && <span>将分配身份 {plan.authoring.assignedId}</span>}{plan.completion?.nextTaskId && <span>后继身份 {plan.completion.nextTaskId}</span>}{plan.completion?.stopped && <span>此序列将在当前任务后停止</span>}</div>
      <label>精确拟议源<textarea readOnly value={planSource(plan)} /></label>
      {(safeMode || blockedReason) && <p className="task-blocked" role="alert">{safeMode ? "安全模式已启用；确认不会提交。" : blockedReason}</p>}
      <div className="dialog-actions"><button type="button" onClick={() => setPlan(null)}>取消</button><button className="primary" type="button" disabled={loading || safeMode || Boolean(blockedReason)} onClick={() => void commitPlan()}>确认并提交</button></div>
    </section></div>}
  </div>;
}
