import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import QuerySurface, { canonicalQuerySource } from "../app/query-surface";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

function execution() {
  const name = { outputName: "name", path: "name", field: "name", propertyKey: null, valueType: "string", nullable: false };
  const path = { outputName: "path", path: "path", field: "path", propertyKey: null, valueType: "string", nullable: false };
  return {
    blockIndex: 0,
    analysis: { blocks: [{ source: "nodes", view: "table", body: "", range: { start: 0, end: 80 }, valid: true }], diagnostics: [] },
    result: {
      source: "nodes",
      columns: [name, path],
      rows: [{ identity: { kind: "node", nodeId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1", revision: "a".repeat(64) }, cells: [{ column: name, value: { kind: "text", value: "Workspace" } }, { column: path, value: { kind: "text", value: "/" } }] }],
      groups: [], totalBeforeLimit: 1, truncated: false,
    },
    csv: "name,path\r\nWorkspace,/\r\n",
  };
}

describe("shared query surface", () => {
  it("submits exact canonical source and renders the authorized Core table", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      return { execution: execution() };
    });
    const onOpenNode = vi.fn(async () => undefined);
    render(<QuerySurface enabled nodeId="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1" documentSource="= Document\n" request={request} onOpenNode={onOpenNode} />);

    fireEvent.click(screen.getByRole("button", { name: "运行 Core 查询" }));
    expect(await screen.findByRole("region", { name: "只读查询表格" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: /名称/ })).toBeTruthy();
    expect(screen.getByText("Workspace")).toBeTruthy();
    const body = calls[0].body as { source: string; blockIndex: number; context: Record<string, unknown> };
    expect(calls[0].path).toBe("/api/query/execute");
    expect(body.source).toBe(canonicalQuerySource("nodes", "from nodes as node\nscope workspace\nwhere true\nselect node.name, node.path\norder by node.path asc\nlimit 100\n"));
    expect(body.blockIndex).toBe(0);
    expect(body.context).toMatchObject({ now: expect.any(String), timezone: expect.any(String), locale: expect.any(String), binding: { nodeId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1", heading: null }, today: { year: expect.any(Number), month: expect.any(Number), day: expect.any(Number) } });

    fireEvent.click(screen.getByRole("button", { name: "打开节点" }));
    await waitFor(() => expect(onOpenNode).toHaveBeenCalledWith("aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1"));
  });

  it("uses Core CSV bytes and exposes embedded-block diagnostics", async () => {
    const createObjectUrl = vi.fn(() => "blob:query-csv");
    const revokeObjectUrl = vi.fn();
    Object.defineProperty(URL, "createObjectURL", { configurable: true, value: createObjectUrl });
    Object.defineProperty(URL, "revokeObjectURL", { configurable: true, value: revokeObjectUrl });
    const clicked: string[] = [];
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function () { clicked.push(this.download); });
    const request = vi.fn(async (_path: string, body?: unknown) => {
      const embedded = (body as { source: string }).source.startsWith("= Embedded");
      return embedded ? { execution: { blockIndex: 0, analysis: { blocks: [{ source: "nodes", view: "table", body: "where unknown = true", range: { start: 12, end: 90 }, valid: false }], diagnostics: [{ code: "unknown_field", message: "unknown nodes query field", range: { start: 50, end: 57 } }] }, result: null, csv: null } } : { execution: execution() };
    });
    render(<QuerySurface enabled nodeId="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1" documentSource="= Embedded\n\n[.weftext-query,version=1]\n....\nfrom nodes as node\nscope workspace\nwhere node.unknown = true\nselect node.id\norder by node.id asc\nlimit 10\n....\n" request={request} onOpenNode={async () => undefined} />);

    fireEvent.click(screen.getByRole("button", { name: "运行 Core 查询" }));
    await screen.findByText("Workspace");
    fireEvent.click(screen.getByRole("button", { name: "导出 Core CSV" }));
    expect(createObjectUrl).toHaveBeenCalledOnce();
    expect((createObjectUrl.mock.calls[0][0] as Blob).type).toBe("text/csv;charset=utf-8");
    expect(clicked[0]).toMatch(/^weftext-nodes-\d{4}-\d{2}-\d{2}\.csv$/u);
    expect(revokeObjectUrl).toHaveBeenCalledWith("blob:query-csv");

    fireEvent.click(screen.getByRole("button", { name: "当前文档内嵌块" }));
    fireEvent.click(screen.getByRole("button", { name: "解析并运行所选块" }));
    const diagnostics = await screen.findByLabelText("查询诊断");
    expect(within(diagnostics).getByText("unknown_field")).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "查询块" })).toBeTruthy();
  });

  it("keeps Core block analysis visible while selecting and running another embedded block", async () => {
    const request = vi.fn(async (_path: string, body?: unknown) => {
      const selected = (body as { blockIndex: number }).blockIndex;
      const base = execution();
      const analysis = {
        blocks: [
          { source: "nodes", view: "table", body: "select node.name", range: { start: 12, end: 70 }, valid: true },
          { source: "tasks", view: "table", body: "select task.title", range: { start: 72, end: 140 }, valid: true },
        ],
        diagnostics: [],
      };
      return selected === 1
        ? { execution: { ...base, blockIndex: 1, analysis, result: { ...base.result, source: "tasks" }, csv: "title\r\nWorkspace\r\n" } }
        : { execution: { ...base, analysis } };
    });
    render(<QuerySurface enabled nodeId="aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1" documentSource="= Embedded\n" request={request} onOpenNode={async () => undefined} />);

    fireEvent.click(screen.getByRole("button", { name: "当前文档内嵌块" }));
    fireEvent.click(screen.getByRole("button", { name: "解析并运行所选块" }));
    const selector = await screen.findByRole("combobox", { name: "查询块" });
    fireEvent.change(selector, { target: { value: "1" } });
    expect(screen.getByRole("combobox", { name: "查询块" })).toBeTruthy();
    expect(screen.queryByText("Workspace")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "解析并运行所选块" }));
    await waitFor(() => expect(request).toHaveBeenLastCalledWith("/api/query/execute", expect.objectContaining({ blockIndex: 1 })));
  });

  it("explains why the surface is unavailable without a connected AsciiDoc workspace", () => {
    render(<QuerySurface enabled={false} nodeId="demo" documentSource="" request={async () => ({})} onOpenNode={async () => undefined} />);
    expect(screen.getByText(/只对已连接的 Weftext AsciiDoc 工作区开放/)).toBeTruthy();
  });
});
