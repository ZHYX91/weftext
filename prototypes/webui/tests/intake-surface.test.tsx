import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import IntakeSurface from "../app/intake-surface";

afterEach(() => cleanup());

const pdfUnavailable = {
  available: false,
  code: "docling_lite_installation_not_verified",
  message: "docling.rs Lite is disabled until every installed asset matches one reviewed lock",
  missingPinnedEvidence: ["pinned worker binary"],
  missingIsolationEvidence: ["deny-by-default network sandbox"],
  ambientNetworkAllowed: false,
};

const pdfAvailable = {
  available: true,
  code: "docling_lite_verified",
  message: "verified packaged worker",
  missingPinnedEvidence: [],
  missingIsolationEvidence: [],
  ambientNetworkAllowed: false,
};

function preview() {
  return {
    bundleDigest: "b".repeat(64),
    baseWorkspaceRevision: "w".repeat(64),
    proposalDigest: "p".repeat(64),
    source: {
      displayName: "Notes.md",
      byteLength: 18,
      sha256: "s".repeat(64),
      detectedFormat: "markdown",
      mismatchEvidence: [],
    },
    probe: {
      adapter: { id: "markdown", version: "1" },
      detectedFormat: "markdown",
      encryption: "not_encrypted",
      safeToPlan: true,
      pageCount: null,
      diagnostics: [],
    },
    plan: {
      planId: "plan-markdown",
      destination: "Imported/Notes",
      route: "markdown_explicit",
      resourcePolicy: "retain_selected",
      localOcrPolicy: "disabled",
      agentEnhancement: { mode: "disabled" },
      egress: { mode: "none" },
    },
    document: {
      title: "笔记",
      revision: "i".repeat(64),
      nodes: [{ id: "paragraph-1", kind: { type: "paragraph", text: "你好" }, confidence: 10000, sourceLocations: [] }],
      resources: [{ id: "original", locator: "resources/Notes.md", mediaType: "text/markdown", byteLength: 18, sha256: "r".repeat(64) }],
      diagnostics: [{ code: "markdown_extension_lowered", severity: "warning", message: "扩展语法已显式降级", irNodeId: "paragraph-1" }],
    },
    proposal: {
      proposalId: "proposal-markdown",
      destination: "Imported/Notes",
      nodes: [{
        locator: "Imported/Notes",
        nodeId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
        documentFile: "Notes.adoc",
        exactAsciidoc: "---\nweftext:\n  id: \"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1\"\n---\n= 笔记\n\n你好\n",
        documentSha256: "d".repeat(64),
        resourceReferences: ["resources/Notes.md"],
        resources: [{ locator: "resources/Notes.md", mediaType: "text/markdown", byteLength: 18, sha256: "r".repeat(64), embedded: false }],
      }],
      conflicts: [],
      warnings: ["请复核转换结果"],
      omissions: [],
    },
    receipt: {
      receiptId: "receipt-markdown",
      localProvenance: [{}],
      agentProvenance: [],
      warnings: [],
    },
  };
}

function markdownFile() {
  const bytes = new TextEncoder().encode("# 笔记\n\n你好\n");
  const file = new File([bytes], "Notes.md", { type: "text/markdown" });
  Object.defineProperty(file, "arrayBuffer", { configurable: true, value: async () => bytes.buffer });
  return { file, bytes };
}

function pdfPreview(agentEnhanced = false) {
  const value = preview();
  return {
    ...value,
    bundleDigest: (agentEnhanced ? "e" : "b").repeat(64),
    source: {
      ...value.source,
      displayName: "Review.pdf",
      byteLength: 32,
      detectedFormat: "pdf",
    },
    probe: {
      ...value.probe,
      adapter: { id: "weftext.docling-lite-pdf", version: "0.52.2" },
      detectedFormat: "pdf",
      pageCount: 1,
    },
    plan: {
      ...value.plan,
      planId: agentEnhanced ? "plan-pdf-enhanced" : "plan-pdf",
      route: "docling_lite_pdf",
      localOcrPolicy: "auto_local",
      agentEnhancement: agentEnhanced ? { mode: "selected_regions_only", provider: "reviewed-provider" } : { mode: "disabled" },
      egress: agentEnhanced ? { mode: "agent_selected_evidence", provider: "reviewed-provider" } : { mode: "none" },
    },
    receipt: {
      ...value.receipt,
      receiptId: agentEnhanced ? "receipt-pdf-enhanced" : "receipt-pdf",
      agentProvenance: agentEnhanced ? [{}] : [],
    },
  };
}

function pdfFile() {
  const bytes = new TextEncoder().encode("%PDF-1.7\n%%EOF\n");
  const file = new File([bytes], "Review.pdf", { type: "application/pdf" });
  Object.defineProperty(file, "arrayBuffer", { configurable: true, value: async () => bytes.buffer });
  return { file, bytes };
}

const markdownTaskSettings = {
  dialect: "markdown_checklist_v1",
  pluginVersion: null,
  globalFilter: null,
  indentationWidth: 4,
  statuses: [
    { symbol: " ", name: "Open", statusType: "TODO" },
    { symbol: "x", name: "Closed", statusType: "DONE" },
    { symbol: "X", name: "Closed", statusType: "DONE" },
  ],
};

const obsidianTaskSettings = {
  dialect: "obsidian_tasks_emoji_v1",
  pluginVersion: "8.2.0",
  globalFilter: "#task",
  indentationWidth: 2,
  statuses: [
    { symbol: " ", name: "Todo", statusType: "TODO" },
    { symbol: "x", name: "Done", statusType: "DONE" },
    { symbol: "/", name: "In Progress", statusType: "IN_PROGRESS" },
    { symbol: ">", name: "Deferred", statusType: "ON_HOLD" },
    { symbol: "-", name: "Cancelled", statusType: "CANCELLED" },
    { symbol: "?", name: "Question", statusType: "NON_TASK" },
  ],
};

function taskPreview(settings = markdownTaskSettings, diagnostics: Array<Record<string, unknown>> = []) {
  const review = {
    proposalId: "task-proposal-exact",
    proposalDigest: "p".repeat(64),
    bundleDigest: "b".repeat(64),
  };
  const nodes = [{
    sourceLocator: "Vault/项目.md",
    destinationLocator: "Imported tasks/Vault/项目",
    nodeId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2",
    documentFile: "项目.adoc",
    exactAsciidoc: "---\nweftext:\n  id: \"aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2\"\n---\n= 项目\n\n* [ ] 编写 {task-id=bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb}\n",
    documentDigest: "d".repeat(64),
  }];
  return {
    stage: "preview",
    adapter: "task_source_set",
    committable: diagnostics.length === 0,
    review,
    bundle: {
      contractVersion: "weftext.task-import-bundle.v1",
      bundleDigest: review.bundleDigest,
      baseWorkspaceRevision: "w".repeat(64),
      destinationParentId: "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
      destinationParentLocator: "",
      destinationName: "Imported tasks",
      destinationRootLocator: "Imported tasks",
      sourceSetDigest: "s".repeat(64),
      sourceDocuments: [
        { locator: "Vault/项目.md", source: "- [ ] #task 编写 🆔 write\n" },
        { locator: "Done.md", source: "- [x] 完成\n" },
      ],
      taskPlan: {
        profile: "weftext.task-import.v1",
        settings,
        documents: [{
          locator: "Vault/项目.md",
          sourceDigest: "1".repeat(64),
          proposedSource: "* [ ] 编写 {task-id=bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb}\n",
          edits: [{ kind: "checklist", sourceRange: { start: 0, end: 32 }, replacement: "* [ ] 编写 {task-id=bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb}\n" }],
        }, {
          locator: "Done.md",
          sourceDigest: "2".repeat(64),
          proposedSource: "* [x] 完成\n",
          edits: [{ kind: "checklist", sourceRange: { start: 0, end: 15 }, replacement: "* [x] 完成\n" }],
        }],
        identities: [{
          locator: "Vault/项目.md",
          occurrenceRange: { start: 0, end: 32 },
          legacyId: "write",
          taskId: "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb",
        }],
        diagnostics,
      },
      nodes,
      proposalId: review.proposalId,
      proposalDigest: review.proposalDigest,
      previewCreatedAt: "2026-08-24T00:00:00Z",
    },
  };
}

function taskFiles() {
  const firstBytes = new TextEncoder().encode("- [ ] #task 编写 🆔 write\n");
  const secondBytes = new TextEncoder().encode("- [x] 完成\n");
  const first = new File([firstBytes], "项目.md", { type: "text/markdown" });
  const second = new File([secondBytes], "Done.md", { type: "text/markdown" });
  Object.defineProperty(first, "webkitRelativePath", { configurable: true, value: "Vault/项目.md" });
  Object.defineProperty(first, "arrayBuffer", { configurable: true, value: async () => firstBytes.buffer });
  Object.defineProperty(second, "arrayBuffer", { configurable: true, value: async () => secondBytes.buffer });
  return { files: [first, second], firstBytes, secondBytes };
}

function taskReceipt(task = taskPreview()) {
  return {
    contractVersion: "weftext.task-import-receipt.v1",
    receiptId: "task-receipt-exact",
    createdAt: "2026-08-24T00:00:00Z",
    sourceSetDigest: task.bundle.sourceSetDigest,
    reviewedBundleDigest: task.review.bundleDigest,
    proposalId: task.review.proposalId,
    proposalDigest: task.review.proposalDigest,
    identities: task.bundle.taskPlan.identities,
    nodes: task.bundle.nodes,
    commonReceipts: [{ receiptId: "common-receipt-1" }, { receiptId: "common-receipt-2" }],
    transaction: { planId: "cccccccc-cccc-4ccc-8ccc-cccccccccccc", revision: "n".repeat(64) },
  };
}

describe("Desktop intake surface", () => {
  it("shows the exact common-IR preview and commits only its stored digest", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const onCommitted = vi.fn(async () => undefined);
    const onClose = vi.fn();
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/markdown/preview") return { import: preview() };
      if (path === "/api/import/commit") return { workspace: { revision: "next" }, import: { receipt: {} } };
      throw new Error(`unexpected ${path}`);
    });
    render(<IntakeSurface enabled safeMode={false} blockedReason="" request={request} onCommitted={onCommitted} onClose={onClose} />);

    const { file, bytes } = markdownFile();
    fireEvent.change(screen.getByLabelText("选择导入文件"), { target: { files: [file] } });
    expect(screen.getByLabelText("目标节点路径")).toHaveProperty("value", "Notes");
    fireEvent.change(screen.getByLabelText("目标节点路径"), { target: { value: "Imported/Notes" } });
    fireEvent.click(screen.getByRole("checkbox", { name: "把原始 Markdown 作为可见节点资源保留" }));
    fireEvent.click(screen.getByRole("button", { name: "生成完整 Core 预览" }));

    const tree = await screen.findByLabelText("拟议节点树");
    expect(within(tree).getByRole("textbox", { name: "精确拟议源" })).toHaveProperty("value", preview().proposal.nodes[0].exactAsciidoc);
    expect(within(tree).getByText("resources/Notes.md")).toBeTruthy();
    expect(screen.getByText("扩展语法已显式降级")).toBeTruthy();
    expect(screen.getByText("请复核转换结果")).toBeTruthy();
    const previewCall = calls.find((call) => call.path === "/api/import/markdown/preview");
    expect(previewCall?.body).toEqual({
      displayName: "Notes.md",
      bytes: Array.from(bytes),
      destination: "Imported/Notes",
      retainOriginal: true,
    });

    fireEvent.click(screen.getByRole("button", { name: "确认并提交固定导入" }));
    await waitFor(() => expect(onCommitted).toHaveBeenCalledOnce());
    expect(calls.find((call) => call.path === "/api/import/commit")?.body).toEqual({ bundleDigest: "b".repeat(64) });
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("freezes explicit PDF egress and accepts only a typed agent patch before final commit", async () => {
    const local = pdfPreview();
    const enhanced = pdfPreview(true);
    const evidence = {
      contractVersion: "weftext.import-agent-evidence.v1",
      evidenceDigest: "v".repeat(64),
      baseIrRevision: local.document.revision,
      sourceDigest: local.source.sha256,
      provider: "reviewed-provider",
      selectedNodeIds: ["paragraph-1"],
      nodes: local.document.nodes,
      resources: [],
    };
    const agentReview = {
      previewDigest: "q".repeat(64),
      baseBundleDigest: local.bundleDigest,
      selection: {
        provider: "reviewed-provider",
        selectedNodeIds: ["paragraph-1"],
        retention: "delete-after-call",
        redaction: "selected-ir-only",
      },
      evidenceDigest: evidence.evidenceDigest,
      evidenceByteLength: 481,
      evidence,
      authorizedPlan: { egress: { mode: "agent_selected_evidence", disclosedBytes: 481 } },
      networkExecuted: false,
      requiresExplicitEgressApproval: true,
    };
    const patch = {
      contractVersion: "weftext.import-agent-patch.v1",
      patchId: "agent-patch-exact",
      baseIrRevision: local.document.revision,
      selectedNodeIds: ["paragraph-1"],
      operations: [{ type: "correct_text", node_id: "paragraph-1", expected_text_digest: "x".repeat(64), replacement: "复核修正" }],
      provider: "reviewed-provider",
      model: "reviewed-model",
      egress: agentReview.authorizedPlan.egress,
    };
    const calls: Array<{ path: string; body?: unknown }> = [];
    const onCommitted = vi.fn(async () => undefined);
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfAvailable } };
      if (path === "/api/import/pdf-preview") return { import: local };
      if (path === "/api/import/agent/prepare") return { agentEnhancement: agentReview };
      if (path === "/api/import/agent/apply-approved-patch") return { import: enhanced };
      if (path === "/api/import/commit") return { workspace: { revision: "next" }, import: { receipt: {} } };
      throw new Error(`unexpected ${path}`);
    });
    render(<IntakeSurface enabled safeMode={false} blockedReason="" request={request} onCommitted={onCommitted} onClose={() => undefined} />);

    fireEvent.click(screen.getByRole("radio", { name: "PDF（docling.rs Lite）" }));
    await screen.findByText("本地 PDF worker 已验证");
    const selected = pdfFile();
    fireEvent.change(screen.getByLabelText("选择导入文件"), { target: { files: [selected.file] } });
    fireEvent.click(screen.getByRole("button", { name: "生成完整 Core 预览" }));
    expect(await screen.findByLabelText("Agent typed IR patch review")).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox", { name: /paragraph.*paragraph-1/ }));
    fireEvent.change(screen.getByLabelText("Agent provider 标识"), { target: { value: "reviewed-provider" } });
    fireEvent.click(screen.getByRole("button", { name: "冻结 Agent evidence 与 egress review" }));

    expect(await screen.findByText(agentReview.previewDigest)).toBeTruthy();
    expect(screen.getByLabelText("精确 Agent evidence JSON")).toHaveProperty("value", JSON.stringify(evidence, null, 2));
    expect(screen.getByText("否")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/import/agent/prepare")?.body).toEqual({
      bundleDigest: local.bundleDigest,
      provider: "reviewed-provider",
      selectedNodeIds: ["paragraph-1"],
      retention: "delete-after-call",
      redaction: "selected-ir-only",
    });
    expect((screen.getByRole("button", { name: "当前预览不能提交" }) as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(screen.getByRole("checkbox", { name: /我批准仅将上述 digest 绑定/ }));
    fireEvent.change(screen.getByLabelText("Agent typed patch JSON"), { target: { value: JSON.stringify(patch) } });
    fireEvent.click(screen.getByRole("button", { name: "验证并应用 typed IR patch" }));

    expect(await screen.findByText("Agent typed patch 已应用并重新生成 Core 预览")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/import/agent/apply-approved-patch")?.body).toEqual({
      previewDigest: agentReview.previewDigest,
      egressApproved: true,
      patch,
    });

    fireEvent.click(screen.getByRole("button", { name: "确认并提交固定导入" }));
    await waitFor(() => expect(onCommitted).toHaveBeenCalledOnce());
    expect(calls.find((call) => call.path === "/api/import/commit")?.body).toEqual({ bundleDigest: enhanced.bundleDigest });
  });

  it("fails closed when the pinned PDF worker is unavailable and when commit policy blocks", async () => {
    const request = vi.fn(async (path: string) => {
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/markdown/preview") return { import: preview() };
      throw new Error(`unexpected ${path}`);
    });
    const { rerender } = render(<IntakeSurface enabled safeMode={false} blockedReason="" request={request} onCommitted={async () => undefined} onClose={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: "PDF（docling.rs Lite）" }));
    expect(await screen.findByText("本机尚不能执行 PDF 导入")).toBeTruthy();
    expect((screen.getByLabelText("选择导入文件") as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText("pinned worker binary")).toBeTruthy();
    expect(screen.getByText(/环境网络：禁止/)).toBeTruthy();

    rerender(<IntakeSurface enabled safeMode blockedReason="" request={request} onCommitted={async () => undefined} onClose={() => undefined} />);
    fireEvent.click(screen.getByRole("radio", { name: "Markdown（显式兼容导入）" }));
    const { file } = markdownFile();
    fireEvent.change(screen.getByLabelText("选择导入文件"), { target: { files: [file] } });
    fireEvent.click(screen.getByRole("button", { name: "生成完整 Core 预览" }));
    expect(await screen.findByText("安全模式已启用；可以预览，但不能提交。")).toBeTruthy();
    expect((screen.getByRole("button", { name: "当前预览不能提交" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("requests cooperative worker cancellation without authorizing a commit", async () => {
    let rejectPreview: (reason: Error) => void = () => undefined;
    const pendingPreview = new Promise<Record<string, unknown>>((_resolve, reject) => {
      rejectPreview = reject;
    });
    const request = vi.fn(async (path: string) => {
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/markdown/preview") return pendingPreview;
      if (path === "/api/import/cancel") {
        rejectPreview(new Error("import worker was cancelled and reaped"));
        return { ok: true, cancelRequested: true };
      }
      throw new Error(`unexpected ${path}`);
    });
    render(<IntakeSurface enabled safeMode={false} blockedReason="" request={request} onCommitted={async () => undefined} onClose={() => undefined} />);
    const { file } = markdownFile();
    fireEvent.change(screen.getByLabelText("选择导入文件"), { target: { files: [file] } });
    fireEvent.click(screen.getByRole("button", { name: "生成完整 Core 预览" }));
    fireEvent.click(await screen.findByRole("button", { name: "取消转换" }));
    expect(await screen.findByText("import worker was cancelled and reaped")).toBeTruthy();
    expect(request).toHaveBeenCalledWith("/api/import/cancel");
    expect(screen.queryByRole("button", { name: "确认并提交固定导入" })).toBeNull();
  });

  it("previews a complete Obsidian task source set and commits only the twice-confirmed exact review", async () => {
    const expectedPreview = taskPreview(obsidianTaskSettings);
    const expectedReceipt = taskReceipt(expectedPreview);
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/task/preview") return { import: expectedPreview };
      if (path === "/api/import/task/commit") return { import: { stage: "committed", receipt: expectedReceipt }, workspace: { nodes: [], rootNodeId: expectedPreview.bundle.destinationParentId } };
      throw new Error(`unexpected ${path}`);
    });
    const chooseTaskReceiptDestination = vi.fn(async () => ({
      capability: "opaque-task-receipt-capability",
      displayPath: "D:\\chosen\\task-import.receipt.json",
    }));
    const onCommitted = vi.fn(async () => undefined);
    render(<IntakeSurface
      enabled
      safeMode={false}
      blockedReason=""
      destinationParentId={expectedPreview.bundle.destinationParentId}
      destinationParentName="Workspace"
      workspaceRevision={expectedPreview.bundle.baseWorkspaceRevision}
      request={request}
      chooseTaskReceiptDestination={chooseTaskReceiptDestination}
      onCommitted={onCommitted}
      onClose={() => undefined}
    />);

    fireEvent.click(screen.getByRole("radio", { name: "任务源集合（Markdown / Obsidian Tasks）" }));
    fireEvent.click(screen.getByRole("radio", { name: "obsidian_tasks_emoji_v1" }));
    fireEvent.change(screen.getByLabelText("Obsidian Tasks 插件版本"), { target: { value: "8.2.0" } });
    fireEvent.change(screen.getByLabelText("任务全局过滤器"), { target: { value: "#task" } });
    fireEvent.change(screen.getByLabelText("任务缩进宽度"), { target: { value: "2" } });
    const selected = taskFiles();
    fireEvent.change(screen.getByLabelText("选择任务源文件"), { target: { files: selected.files } });
    expect(screen.getByText("Vault/项目.md")).toBeTruthy();
    expect(screen.getByText("Done.md")).toBeTruthy();
    expect(screen.getByLabelText("导入集合节点名称")).toHaveProperty("value", "Imported tasks");
    fireEvent.click(screen.getByRole("button", { name: "生成 exact source-set review" }));

    expect(await screen.findByText(expectedPreview.bundle.sourceSetDigest)).toBeTruthy();
    expect(screen.getByText("write")).toBeTruthy();
    expect(screen.getByText("bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb")).toBeTruthy();
    expect(screen.getByLabelText("Imported tasks/Vault/项目 exact proposed AsciiDoc")).toHaveProperty("value", expectedPreview.bundle.nodes[0].exactAsciidoc);
    const previewCall = calls.find((call) => call.path === "/api/import/task/preview");
    expect(previewCall?.body).toEqual({
      profile: "weftext.task-import.v1",
      destinationParentId: expectedPreview.bundle.destinationParentId,
      destinationName: "Imported tasks",
      settings: obsidianTaskSettings,
      documents: [
        { locator: "Vault/项目.md", bytes: Array.from(selected.firstBytes) },
        { locator: "Done.md", bytes: Array.from(selected.secondBytes) },
      ],
    });

    fireEvent.click(screen.getByRole("button", { name: "使用系统选择器授权新 JSON receipt" }));
    await waitFor(() => expect(chooseTaskReceiptDestination).toHaveBeenCalledWith(`task-import-${"b".repeat(12)}.receipt.json`));
    expect(screen.getByLabelText("Task import receipt 目标")).toHaveProperty("value", "D:\\chosen\\task-import.receipt.json");
    fireEvent.click(screen.getByRole("checkbox", { name: /我已逐项核对当前 source-set digest/ }));
    const commit = screen.getByRole("button", { name: "二次确认并提交 exact review" });
    fireEvent.click(commit);
    fireEvent.click(commit);

    expect(await screen.findByText("任务源集合已提交，receipt 已精确发布")).toBeTruthy();
    expect(screen.getByText("task-receipt-exact")).toBeTruthy();
    expect(onCommitted).toHaveBeenCalledOnce();
    const commitCalls = calls.filter((call) => call.path === "/api/import/task/commit");
    expect(commitCalls).toHaveLength(1);
    expect(commitCalls[0].body).toEqual({
      review: expectedPreview.review,
      receiptDestinationCapability: "opaque-task-receipt-capability",
    });
    expect(JSON.stringify(commitCalls[0].body)).not.toContain("D:\\\\chosen");
  });

  it("keeps Enter and Escape inert during IME composition and invalidates a stale task review", async () => {
    const expectedPreview = taskPreview();
    const onClose = vi.fn();
    const request = vi.fn(async (path: string) => {
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/task/preview") return { import: expectedPreview };
      throw new Error(`unexpected ${path}`);
    });
    const props = {
      enabled: true,
      safeMode: false,
      blockedReason: "",
      destinationParentId: expectedPreview.bundle.destinationParentId,
      destinationParentName: "Workspace",
      workspaceRevision: expectedPreview.bundle.baseWorkspaceRevision,
      request,
      onCommitted: async () => undefined,
      onClose,
    };
    const { rerender } = render(<IntakeSurface {...props} />);
    fireEvent.click(screen.getByRole("radio", { name: "任务源集合（Markdown / Obsidian Tasks）" }));
    fireEvent.change(screen.getByLabelText("选择任务源文件"), { target: { files: taskFiles().files } });
    const destination = screen.getByLabelText("导入集合节点名称");

    fireEvent.keyDown(destination, { key: "Escape", isComposing: true });
    expect(onClose).not.toHaveBeenCalled();
    fireEvent.keyDown(destination, { key: "Enter", isComposing: true });
    expect(request).not.toHaveBeenCalledWith("/api/import/task/preview", expect.anything());
    fireEvent.keyDown(destination, { key: "Enter", isComposing: false });
    expect(await screen.findByText(expectedPreview.bundle.sourceSetDigest)).toBeTruthy();

    rerender(<IntakeSurface {...props} blockedReason="存在未保存的设备草稿" />);
    expect(screen.getByRole("alert", { name: "" }).textContent).toContain("存在未保存的设备草稿");
    rerender(<IntakeSurface {...props} safeMode />);
    expect(screen.getByText("安全模式已启用；可以预览，但不能提交或恢复。")).toBeTruthy();
    rerender(<IntakeSurface {...props} workspaceRevision={"z".repeat(64)} />);
    expect(await screen.findByText("工作区修订已变化；旧导入预览已作废，请重新生成精确 review。")).toBeTruthy();
    expect(screen.queryByText(expectedPreview.bundle.sourceSetDigest)).toBeNull();
  });

  it("recovers a retained exact task review without replaying the importer or sending a native path", async () => {
    const expectedPreview = taskPreview();
    const expectedReceipt = taskReceipt(expectedPreview);
    const request = vi.fn(async (path: string) => {
      if (path === "/api/import/pdf-capability") return { import: { capability: pdfUnavailable } };
      if (path === "/api/import/task/preview") return { import: expectedPreview };
      if (path === "/api/import/task/commit") throw new Error("receipt handoff interrupted；精确任务源集合 bundle/review 已保留，可尝试恢复");
      if (path === "/api/import/task/recover") return {
        import: {
          stage: "task_recovered",
          recovery: {
            status: "receipt_recovered",
            committed: { receipt: expectedReceipt },
            recovery: {},
          },
        },
        workspace: { nodes: [], rootNodeId: expectedPreview.bundle.destinationParentId },
      };
      throw new Error(`unexpected ${path}`);
    });
    const onCommitted = vi.fn(async () => undefined);
    render(<IntakeSurface
      enabled
      safeMode={false}
      blockedReason=""
      destinationParentId={expectedPreview.bundle.destinationParentId}
      workspaceRevision={expectedPreview.bundle.baseWorkspaceRevision}
      request={request}
      chooseTaskReceiptDestination={async () => ({ capability: "receipt-capability", displayPath: "selected receipt.json" })}
      onCommitted={onCommitted}
      onClose={() => undefined}
    />);
    fireEvent.click(screen.getByRole("radio", { name: "任务源集合（Markdown / Obsidian Tasks）" }));
    fireEvent.change(screen.getByLabelText("选择任务源文件"), { target: { files: taskFiles().files } });
    fireEvent.click(screen.getByRole("button", { name: "生成 exact source-set review" }));
    await screen.findByText(expectedPreview.bundle.sourceSetDigest);
    fireEvent.click(screen.getByRole("button", { name: "使用系统选择器授权新 JSON receipt" }));
    await waitFor(() => expect(screen.getByLabelText("Task import receipt 目标")).toHaveProperty("value", "selected receipt.json"));
    fireEvent.click(screen.getByRole("checkbox", { name: /我已逐项核对当前 source-set digest/ }));
    fireEvent.click(screen.getByRole("button", { name: "二次确认并提交 exact review" }));
    expect(await screen.findByRole("button", { name: "恢复已开始的 exact task import" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "恢复已开始的 exact task import" }));

    expect(await screen.findByText("已恢复 receipt 并完成事务")).toBeTruthy();
    expect(screen.getByText("task-receipt-exact")).toBeTruthy();
    expect(request.mock.calls.filter(([path]) => path === "/api/import/task/preview")).toHaveLength(1);
    expect(request).toHaveBeenCalledWith("/api/import/task/recover", { review: expectedPreview.review });
    expect(onCommitted).toHaveBeenCalledOnce();
  });
});
