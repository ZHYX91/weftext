import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import TaskSurface, { type TaskOccurrence } from "../app/task-surface";

afterEach(() => cleanup());

const simpleTask: TaskOccurrence = {
  nodeId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
  revision: "document-revision",
  task: {
    state: "open",
    description: "Simple task",
    listDepth: 0,
    range: { start: 80, end: 97 },
    metadata: null,
    valid: true,
  },
};

const recurringTask: TaskOccurrence = {
  nodeId: simpleTask.nodeId,
  revision: simpleTask.revision,
  task: {
    state: "open",
    description: "Recurring task",
    listDepth: 0,
    range: { start: 98, end: 220 },
    metadata: {
      id: "33333333-3333-4333-8333-333333333333",
      phase: null,
      resolution: null,
      priority: "high",
      created: null,
      start: null,
      scheduled: null,
      due: { kind: "date", value: "2026-08-24" },
      closed: null,
      recurrence: { source: "FREQ=DAILY;COUNT=2" },
      repeatFrom: "due",
      dependencies: [],
    },
    valid: true,
  },
};

function inspection() {
  return { nodeId: simpleTask.nodeId, occurrences: [simpleTask, recurringTask], diagnostics: [] };
}

describe("shared task surface", () => {
  it("previews and single-confirms an ordinary Core task plan", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const onCommitted = vi.fn(async () => undefined);
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path.startsWith("/api/task/inspect")) return inspection();
      if (path === "/api/task/edit-preview") return {
        plan: {
          planId: "plan-edit",
          kind: "edit",
          baseWorkspaceRevision: "workspace-revision",
          nodeId: simpleTask.nodeId,
          authoring: { proposedSource: "* [x] Simple task\n", assignedId: null, target: { ...simpleTask.task, state: "closed" } },
          documentChanges: [{}],
        },
      };
      if (path === "/api/task/transaction/commit") return { workspace: { revision: "next-workspace" }, result: {} };
      throw new Error(`unexpected ${path}`);
    });

    render(<TaskSurface enabled nodeId={simpleTask.nodeId} workspaceRevision="workspace-revision" documentRevision="document-revision" blockedReason="" safeMode={false} request={request} onCommitted={onCommitted} />);
    const toggle = await screen.findByRole("button", { name: "完成任务：Simple task" });
    fireEvent.click(toggle);
    const dialog = await screen.findByRole("dialog", { name: "确认任务事务" });
    expect(screen.getByRole("textbox", { name: "精确拟议源" })).toHaveProperty("value", "* [x] Simple task\n");
    expect(calls.find((call) => call.path === "/api/task/edit-preview")?.body).toEqual({
      nodeId: simpleTask.nodeId,
      baseWorkspaceRevision: "workspace-revision",
      baseRevision: "document-revision",
      target: { kind: "occurrence", range: simpleTask.task.range },
      intent: { kind: "toggle" },
    });
    fireEvent.click(screen.getByRole("button", { name: "确认并提交" }));
    await waitFor(() => expect(onCommitted).toHaveBeenCalledOnce());
    expect(calls.find((call) => call.path === "/api/task/transaction/commit")?.body).toEqual({ planId: "plan-edit" });
    expect(dialog.isConnected).toBe(false);
  });

  it("routes recurring completion and complete dependency replacement through typed plans", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path.startsWith("/api/task/inspect")) return inspection();
      if (path === "/api/task/recurrence-preview") return {
        plan: {
          planId: "plan-recurrence",
          kind: "recurrence",
          baseWorkspaceRevision: "workspace-revision",
          nodeId: simpleTask.nodeId,
          completion: { proposedSource: "next source", completedTask: recurringTask.task, nextTask: recurringTask.task, nextTaskId: "44444444-4444-4444-8444-444444444444", stopped: false },
          documentChanges: [{}],
        },
      };
      if (path === "/api/task/dependencies-preview") return {
        plan: {
          planId: "plan-dependencies",
          kind: "dependencies",
          baseWorkspaceRevision: "workspace-revision",
          nodeId: simpleTask.nodeId,
          dependencies: ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"],
          authoring: { proposedSource: "dependency source", assignedId: null, target: recurringTask.task },
          documentChanges: [{}],
        },
      };
      throw new Error(`unexpected ${path}`);
    });

    render(<TaskSurface enabled nodeId={simpleTask.nodeId} workspaceRevision="workspace-revision" documentRevision="document-revision" blockedReason="" safeMode={false} request={request} onCommitted={async () => undefined} />);
    fireEvent.click(await screen.findByRole("button", { name: "完成任务：Recurring task" }));
    await screen.findByText("后继身份 44444444-4444-4444-8444-444444444444");
    const recurrence = calls.find((call) => call.path === "/api/task/recurrence-preview")?.body as Record<string, unknown>;
    expect(recurrence.target).toEqual({ kind: "id", id: recurringTask.task.metadata?.id });
    expect(recurrence).toMatchObject({
      nodeId: simpleTask.nodeId,
      baseWorkspaceRevision: "workspace-revision",
      baseRevision: "document-revision",
      context: { completedAt: { kind: "date" }, utcOffsetMinutes: expect.any(Number) },
    });
    fireEvent.click(screen.getByRole("button", { name: "取消" }));

    fireEvent.click(screen.getAllByRole("button", { name: "编辑" })[1]);
    fireEvent.change(screen.getByRole("textbox", { name: "依赖任务 UUID" }), {
      target: { value: "11111111-1111-4111-8111-111111111111,\n22222222-2222-4222-8222-222222222222" },
    });
    fireEvent.click(screen.getByRole("button", { name: "预览完整依赖集" }));
    await screen.findByText("确认完整依赖集");
    expect(calls.find((call) => call.path === "/api/task/dependencies-preview")?.body).toEqual({
      nodeId: simpleTask.nodeId,
      baseWorkspaceRevision: "workspace-revision",
      baseRevision: "document-revision",
      target: { kind: "id", id: recurringTask.task.metadata?.id },
      dependencies: ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"],
    });
  });

  it("disables task mutations for dirty source and Safe Mode", async () => {
    const request = vi.fn(async (path: string) => {
      if (path.startsWith("/api/task/inspect")) return inspection();
      if (path === "/api/task/edit-preview") return {
        plan: {
          planId: "safe-plan",
          kind: "edit",
          baseWorkspaceRevision: "workspace-revision",
          nodeId: simpleTask.nodeId,
          authoring: { proposedSource: "safe source", assignedId: null, target: simpleTask.task },
          documentChanges: [{}],
        },
      };
      throw new Error(`unexpected ${path}`);
    });
    const { rerender } = render(<TaskSurface enabled nodeId={simpleTask.nodeId} workspaceRevision="workspace-revision" documentRevision="document-revision" blockedReason="请先保存当前草稿" safeMode={false} request={request} onCommitted={async () => undefined} />);
    expect((await screen.findByRole("button", { name: "完成任务：Simple task" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByRole("alert").textContent).toContain("请先保存当前草稿");

    rerender(<TaskSurface enabled nodeId={simpleTask.nodeId} workspaceRevision="workspace-revision" documentRevision="document-revision" blockedReason="" safeMode request={request} onCommitted={async () => undefined} />);
    fireEvent.click(await screen.findByRole("button", { name: "完成任务：Simple task" }));
    await screen.findByRole("dialog", { name: "确认任务事务" });
    expect((screen.getByRole("button", { name: "确认并提交" }) as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByText("安全模式已启用；确认不会提交。")).toBeTruthy();
  });
});
