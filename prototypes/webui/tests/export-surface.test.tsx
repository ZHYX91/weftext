import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import ExportSurface from "../app/export-surface";

afterEach(() => cleanup());

const nodeId = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const destination = "C:\\Exports\\笔记.md";
const destinationGrant = { capability: "markdown-export-destination-opaque", displayPath: destination };

function exportPlan() {
  return {
    contractVersion: "weftext.export.markdown.v1",
    planId: "export-fixed-plan",
    bundleDigest: "b".repeat(64),
    baseWorkspaceRevision: "w".repeat(64),
    sourceNodeId: nodeId,
    sourceDocumentRevision: "d".repeat(64),
    sourceProfile: "ascii_doc_v1",
    sourceByteLength: 81,
    semanticModelVersion: 2,
    destination,
    metadataPolicy: "preserve_weftext",
    resourcePolicy: "external_references_only",
    mediaType: "text/markdown; charset=utf-8",
    artifactDigest: "a".repeat(64),
    artifact: "---\nweftext:\n  id: aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\n---\n# 笔记\n\n正文\n",
    diagnostics: [{ code: "markdown_literal_preserved", severity: "warning", message: "一个块按 literal 保留", sourceStart: 60, sourceEnd: 81 }],
    report: { exactBlocks: 2, loweredBlocks: 1, preservedLiteralBlocks: 1, omittedBlocks: 0 },
    components: [
      { componentId: "weftext-core-document-model", version: "semantic-model-v2" },
      { componentId: "weftext-markdown-exporter", version: "weftext.markdown-exporter.v1" },
    ],
  };
}

function exportReceipt() {
  const plan = exportPlan();
  return {
    contractVersion: "weftext.export.receipt.v1",
    createdAt: "2026-08-24T12:00:00Z",
    planId: plan.planId,
    planDigest: plan.bundleDigest,
    sourceNodeId: nodeId,
    sourceDocumentRevision: plan.sourceDocumentRevision,
    baseWorkspaceRevision: plan.baseWorkspaceRevision,
    destination,
    artifactDigest: plan.artifactDigest,
    artifactByteLength: new TextEncoder().encode(plan.artifact).length,
    status: "committed",
  };
}

describe("Desktop Markdown export surface", () => {
  it("uses the system picker, reviews exact bytes, and commits only the stored plan ID", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/export/markdown/preview") return { export: { stage: "preview", format: "markdown_compatibility", plan: exportPlan() } };
      if (path === "/api/export/commit") return { export: { stage: "committed", format: "markdown_compatibility", receipt: exportReceipt() } };
      throw new Error(`unexpected ${path}`);
    });
    const chooseDestination = vi.fn(async () => destinationGrant);
    const onCommitted = vi.fn();
    render(<ExportSurface enabled safeMode={false} blockedReason="" nodeId={nodeId} nodeName="笔记" request={request} chooseDestination={chooseDestination} onCommitted={onCommitted} onClose={() => undefined} />);

    fireEvent.click(screen.getByRole("button", { name: "使用系统选择器选择新文件" }));
    await waitFor(() => expect(chooseDestination).toHaveBeenCalledWith("笔记.md"));
    expect(screen.getByLabelText("Markdown 导出目标")).toHaveProperty("value", destination);
    fireEvent.click(screen.getByRole("button", { name: "生成精确导出预览" }));

    const artifact = await screen.findByRole("textbox", { name: "精确 Markdown 产物源码" });
    expect(artifact).toHaveProperty("value", exportPlan().artifact);
    expect(screen.getByText("一个块按 literal 保留")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/export/markdown/preview")?.body).toEqual({
      nodeId,
      destinationCapability: destinationGrant.capability,
      metadataPolicy: "preserve_weftext",
    });

    fireEvent.click(screen.getByRole("button", { name: "确认并发布固定 Markdown" }));
    expect(await screen.findByText("已发布并完成精确字节校验")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/export/commit")?.body).toEqual({ planId: exportPlan().planId });
    expect(onCommitted).toHaveBeenCalledWith(exportReceipt());
  });

  it("allows preview in safe mode but keeps the external publish closed", async () => {
    const request = vi.fn(async () => ({ export: { stage: "preview", plan: exportPlan() } }));
    render(<ExportSurface enabled safeMode blockedReason="" nodeId={nodeId} nodeName="笔记" request={request} chooseDestination={async () => destinationGrant} onCommitted={() => undefined} onClose={() => undefined} />);

    fireEvent.click(screen.getByRole("button", { name: "使用系统选择器选择新文件" }));
    await waitFor(() => expect(screen.getByLabelText("Markdown 导出目标")).toHaveProperty("value", destination));
    fireEvent.click(screen.getByRole("button", { name: "生成精确导出预览" }));

    expect(await screen.findByText("安全模式允许预览，但不允许发布外部文件。")).toBeTruthy();
    expect((screen.getByRole("button", { name: "当前预览不能发布" }) as HTMLButtonElement).disabled).toBe(true);
    expect(request).toHaveBeenCalledTimes(1);
  });

  it("fails closed on a mismatched Core plan", async () => {
    const request = vi.fn(async () => ({ export: { stage: "preview", plan: { ...exportPlan(), sourceNodeId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2" } } }));
    render(<ExportSurface enabled safeMode={false} blockedReason="" nodeId={nodeId} nodeName="笔记" request={request} chooseDestination={async () => destinationGrant} onCommitted={() => undefined} onClose={() => undefined} />);

    fireEvent.click(screen.getByRole("button", { name: "使用系统选择器选择新文件" }));
    await waitFor(() => expect(screen.getByLabelText("Markdown 导出目标")).toHaveProperty("value", destination));
    fireEvent.click(screen.getByRole("button", { name: "生成精确导出预览" }));

    expect(await screen.findByText("Core 返回了无效或错配的 Markdown Export Plan")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "确认并发布固定 Markdown" })).toBeNull();
  });
});
