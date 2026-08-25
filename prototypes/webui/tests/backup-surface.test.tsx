import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import BackupSurface, { type BackupDirectoryKind } from "../app/backup-surface";

afterEach(() => cleanup());

const digest = "a".repeat(64);
const sourceNodeId = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1";
const destinationParentId = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb2";

function capabilities(safeMode = false) {
  return {
    backup: {
      schema: "weftext.desktop-backup-capabilities.v1",
      documentProfile: "ascii_doc_v1",
      managedShape: "X/X.adoc",
      annotations: "node_local_weftext.annotations.json",
      fullWorkspace: true,
      verify: true,
      protect: true,
      retention: true,
      alternateRestore: true,
      singleNodeRestore: true,
      subtreeRestore: true,
      restoreDrill: true,
      targetAuthority: "native_directory_capability",
      safeMode,
      workspaceMutationAllowed: !safeMode,
      savedSourceSetReady: true,
      draftCount: 0,
      recoveryIssueCount: 0,
    },
  };
}

function plan(schema: string) {
  return { schema, planDigest: digest, entryCount: 12, totalBytes: 4096, commitState: "ready" };
}

function grant(kind: BackupDirectoryKind) {
  return { capability: `grant-${kind}`, kind, displayPath: `C:\\Safety\\${kind}` };
}

const baseProps = {
  enabled: true,
  safeMode: false,
  blockedReason: "",
  sourceNodeId,
  destinationParentId,
  onWorkspaceChanged: vi.fn(),
  onClose: vi.fn(),
};

describe("Desktop backup product surface", () => {
  it("uses one typed directory grant and commits only the reviewed digest, even in Safe Mode", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/backup/capabilities") return capabilities(true);
      if (path === "/api/backup/preview") return { backup: { stage: "preview", plan: plan("weftext.full-workspace-backup-plan.v1") } };
      if (path === "/api/backup/commit") return { backup: { stage: "committed", receipt: { verified: true } } };
      throw new Error(`unexpected ${path}`);
    });
    const chooseDirectory = vi.fn(async (kind: BackupDirectoryKind) => grant(kind));
    render(<BackupSurface {...baseProps} safeMode request={request} chooseDirectory={chooseDirectory} />);

    expect(await screen.findByText(/X\/X\.adoc/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "选择目标并预览完整备份" }));
    expect(await screen.findByText("weftext.full-workspace-backup-plan.v1")).toBeTruthy();
    expect(chooseDirectory).toHaveBeenCalledWith("backup_parent");
    expect(calls.find((call) => call.path === "/api/backup/preview")?.body).toEqual({
      backupParentCapability: "grant-backup_parent",
    });

    fireEvent.click(screen.getByRole("checkbox"));
    const commit = screen.getByRole("button", { name: "提交精确备份计划" });
    expect((commit as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(commit);
    await waitFor(() => expect(request).toHaveBeenCalledWith("/api/backup/commit", { planDigest: digest }));
    expect(await screen.findByText(/已完成并返回回执/)).toBeTruthy();
  });

  it("blocks authoritative previews for drafts and keeps scoped commit closed in Safe Mode", async () => {
    let reportedSafeMode = false;
    const request = vi.fn(async (path: string) => {
      if (path === "/api/backup/capabilities") return capabilities(reportedSafeMode);
      if (path === "/api/backup/scoped-restore/preview") {
        return { backup: { stage: "scoped_restore_preview", plan: plan("weftext.scoped-workspace-restore-plan.v1") } };
      }
      throw new Error(`unexpected ${path}`);
    });
    const chooseDirectory = vi.fn(async (kind: BackupDirectoryKind) => grant(kind));
    const mounted = render(<BackupSurface {...baseProps} blockedReason="存在未保存的设备草稿" request={request} chooseDirectory={chooseDirectory} />);
    await screen.findByText(/X\/X\.adoc/);
    expect((screen.getByRole("button", { name: "选择目标并预览完整备份" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "选择快照并预览范围恢复" }) as HTMLButtonElement).disabled).toBe(true);

    reportedSafeMode = true;
    mounted.rerender(<BackupSurface {...baseProps} safeMode request={request} chooseDirectory={chooseDirectory} />);
    await waitFor(() => expect(request.mock.calls.filter(([path]) => path === "/api/backup/capabilities")).toHaveLength(2));
    const scopedPreview = screen.getByRole("button", { name: "选择快照并预览范围恢复" });
    await waitFor(() => expect((scopedPreview as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(scopedPreview);
    expect(await screen.findByText("weftext.scoped-workspace-restore-plan.v1")).toBeTruthy();
    expect(request).toHaveBeenCalledWith("/api/backup/scoped-restore/preview", {
      snapshotCapability: "grant-snapshot",
      sourceNodeId,
      destinationParentId,
      destinationName: "Recovered",
      scope: "single_node",
    });
    fireEvent.click(screen.getByRole("checkbox"));
    const commit = screen.getByRole("button", { name: "Safe Mode：当前工作区恢复已暂停" });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    expect(request).not.toHaveBeenCalledWith("/api/backup/scoped-restore/commit", expect.anything());
  });

  it("fails closed when the native capability state disagrees with Safe Mode", async () => {
    const request = vi.fn(async (path: string) => {
      if (path === "/api/backup/capabilities") return capabilities(false);
      throw new Error(`unexpected ${path}`);
    });
    const chooseDirectory = vi.fn(async (kind: BackupDirectoryKind) => grant(kind));
    render(<BackupSurface {...baseProps} safeMode request={request} chooseDirectory={chooseDirectory} />);

    expect(await screen.findByText(/备份能力与当前 Safe Mode 状态不一致/)).toBeTruthy();
    const preview = screen.getByRole("button", { name: "选择目标并预览完整备份" });
    expect((preview as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(preview);
    expect(chooseDirectory).not.toHaveBeenCalled();
  });

  it("revokes commit controls while a reviewed plan no longer has a valid capability contract", async () => {
    const request = vi.fn(async (path: string) => {
      if (path === "/api/backup/capabilities") return capabilities(false);
      if (path === "/api/backup/preview") {
        return { backup: { stage: "preview", plan: plan("weftext.full-workspace-backup-plan.v1") } };
      }
      if (path === "/api/backup/commit") {
        return { backup: { stage: "committed", receipt: { verified: true } } };
      }
      throw new Error(`unexpected ${path}`);
    });
    const chooseDirectory = vi.fn(async (kind: BackupDirectoryKind) => grant(kind));
    const mounted = render(<BackupSurface {...baseProps} request={request} chooseDirectory={chooseDirectory} />);
    await screen.findByText(/X\/X\.adoc/);
    fireEvent.click(screen.getByRole("button", { name: "选择目标并预览完整备份" }));
    expect(await screen.findByText("weftext.full-workspace-backup-plan.v1")).toBeTruthy();
    fireEvent.click(screen.getByRole("checkbox"));
    expect((screen.getByRole("button", { name: "提交精确备份计划" }) as HTMLButtonElement).disabled).toBe(false);

    mounted.rerender(<BackupSurface {...baseProps} safeMode request={request} chooseDirectory={chooseDirectory} />);
    expect(await screen.findByText(/备份能力与当前 Safe Mode 状态不一致/)).toBeTruthy();
    const commit = screen.getByRole("button", { name: "提交精确备份计划" });
    expect((commit as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(commit);
    expect(request).not.toHaveBeenCalledWith("/api/backup/commit", expect.anything());
  });

  it("exposes typed verify, protect, retention, alternate restore, and drill controls", async () => {
    const calls: Array<{ path: string; body?: unknown }> = [];
    const request = vi.fn(async (path: string, body?: unknown) => {
      calls.push({ path, body });
      if (path === "/api/backup/capabilities") return capabilities();
      if (path === "/api/backup/verify") return { backup: { stage: "verified", verification: { complete: true } } };
      if (path === "/api/backup/protect") return { backup: { stage: "protected", protection: { label: "重要恢复点" } } };
      if (path === "/api/backup/retention/recover") return { backup: { stage: "retention_recovered", recovery: {} } };
      if (path === "/api/backup/retention/preview") return { backup: { stage: "retention_preview", plan: plan("weftext.snapshot-retention-plan.v1") } };
      if (path === "/api/backup/restore/preview") return { backup: { stage: "restore_preview", plan: plan("weftext.full-workspace-restore-plan.v1") } };
      if (path === "/api/backup/drill/preview") return { backup: { stage: "drill_preview", plan: plan("weftext.full-workspace-restore-drill-plan.v1") } };
      throw new Error(`unexpected ${path}`);
    });
    const chooseDirectory = vi.fn(async (kind: BackupDirectoryKind) => grant(kind));
    render(<BackupSurface {...baseProps} request={request} chooseDirectory={chooseDirectory} />);
    await screen.findByText(/X\/X\.adoc/);

    fireEvent.click(screen.getByRole("button", { name: "选择快照并逐字节验证" }));
    await waitFor(() => expect(request).toHaveBeenCalledWith("/api/backup/verify", { snapshotCapability: "grant-snapshot" }));
    expect(await screen.findByText(/已验证完整快照/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "选择快照并保护" }));
    await waitFor(() => expect(request).toHaveBeenCalledWith("/api/backup/protect", { snapshotCapability: "grant-snapshot", label: "重要恢复点" }));
    expect(await screen.findByText(/已保护恢复点/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "恢复中断的保留操作" }));
    await waitFor(() => expect(request).toHaveBeenCalledWith("/api/backup/retention/recover", { backupParentCapability: "grant-backup_parent" }));
    expect(await screen.findByText(/保留策略中断证据已恢复或确认完成/)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "选择备份目录并预览保留策略" }));
    expect(await screen.findByText("weftext.snapshot-retention-plan.v1")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "预览到新的 alternate 工作区" }));
    expect(await screen.findByText("weftext.full-workspace-restore-plan.v1")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/backup/restore/preview")?.body).toEqual({
      snapshotCapability: "grant-snapshot",
      destinationParentCapability: "grant-restore_parent",
    });
    fireEvent.click(screen.getByRole("button", { name: "选择目录并预览恢复演练" }));
    expect(await screen.findByText("weftext.full-workspace-restore-drill-plan.v1")).toBeTruthy();
    expect(calls.find((call) => call.path === "/api/backup/drill/preview")?.body).toEqual({
      snapshotCapability: "grant-snapshot",
      drillParentCapability: "grant-drill_parent",
      resultsParentCapability: "grant-drill_results_parent",
    });
  });
});
