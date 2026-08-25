"use client";

import { useEffect, useState } from "react";

export type BackupDirectoryKind = "backup_parent" | "snapshot" | "restore_parent" | "drill_parent" | "drill_results_parent";

type BackupRequest = (path: string, body?: unknown) => Promise<Record<string, unknown>>;
type BackupDirectoryGrant = { capability: string; kind: BackupDirectoryKind; displayPath: string };
type PlanPurpose = "backup" | "retention" | "restore" | "scoped" | "drill";
type ReviewedPlan = {
  purpose: PlanPurpose;
  digest: string;
  schema: string;
  value: Record<string, unknown>;
};

type BackupCapabilities = {
  schema: "weftext.desktop-backup-capabilities.v1";
  documentProfile: "ascii_doc_v1";
  managedShape: "X/X.adoc";
  annotations: "node_local_weftext.annotations.json";
  fullWorkspace: true;
  verify: true;
  protect: true;
  retention: true;
  alternateRestore: true;
  singleNodeRestore: true;
  subtreeRestore: true;
  restoreDrill: true;
  targetAuthority: "native_directory_capability";
  safeMode: boolean;
  workspaceMutationAllowed: boolean;
  savedSourceSetReady: boolean;
  draftCount: number;
  recoveryIssueCount: number;
};

type BackupSurfaceProps = {
  enabled: boolean;
  safeMode: boolean;
  blockedReason: string;
  sourceNodeId: string;
  destinationParentId: string;
  request: BackupRequest;
  chooseDirectory(kind: BackupDirectoryKind): Promise<BackupDirectoryGrant | null>;
  onWorkspaceChanged(): Promise<void> | void;
  onClose(): void;
};

const planSchemas: Record<PlanPurpose, string> = {
  backup: "weftext.full-workspace-backup-plan.v1",
  retention: "weftext.snapshot-retention-plan.v1",
  restore: "weftext.full-workspace-restore-plan.v1",
  scoped: "weftext.scoped-workspace-restore-plan.v1",
  drill: "weftext.full-workspace-restore-drill-plan.v1",
};

const commitRoutes: Record<PlanPurpose, { path: string; stage: string; label: string }> = {
  backup: { path: "/api/backup/commit", stage: "committed", label: "提交精确备份计划" },
  retention: { path: "/api/backup/retention/commit", stage: "retention_committed", label: "执行精确保留计划" },
  restore: { path: "/api/backup/restore/commit", stage: "restored", label: "恢复到干净 alternate 目标" },
  scoped: { path: "/api/backup/scoped-restore/commit", stage: "scoped_restored", label: "提交范围恢复到当前工作区" },
  drill: { path: "/api/backup/drill/commit", stage: "drill_completed", label: "执行精确恢复演练" },
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0;
}

function isDigest(value: unknown): value is string {
  return typeof value === "string" && /^[0-9a-f]{64}$/u.test(value);
}

function requireCapabilities(payload: Record<string, unknown>): BackupCapabilities {
  const value = payload.backup;
  if (!isRecord(value)
    || value.schema !== "weftext.desktop-backup-capabilities.v1"
    || value.documentProfile !== "ascii_doc_v1"
    || value.managedShape !== "X/X.adoc"
    || value.annotations !== "node_local_weftext.annotations.json"
    || value.fullWorkspace !== true
    || value.verify !== true
    || value.protect !== true
    || value.retention !== true
    || value.alternateRestore !== true
    || value.singleNodeRestore !== true
    || value.subtreeRestore !== true
    || value.restoreDrill !== true
    || value.targetAuthority !== "native_directory_capability"
    || typeof value.safeMode !== "boolean"
    || typeof value.workspaceMutationAllowed !== "boolean"
    || typeof value.savedSourceSetReady !== "boolean"
    || !isCount(value.draftCount)
    || !isCount(value.recoveryIssueCount)) {
    throw new Error("Desktop 返回了无效的备份能力契约");
  }
  return value as BackupCapabilities;
}

function requireGrant(value: unknown, kind: BackupDirectoryKind): BackupDirectoryGrant {
  if (!isRecord(value)
    || typeof value.capability !== "string"
    || !value.capability
    || value.kind !== kind
    || typeof value.displayPath !== "string"
    || !value.displayPath) {
    throw new Error("系统选择器返回了错配的备份目录授权");
  }
  return value as BackupDirectoryGrant;
}

function requireBackupEnvelope(payload: Record<string, unknown>, stage: string) {
  const backup = payload.backup;
  if (!isRecord(backup) || backup.stage !== stage) {
    throw new Error("Desktop 返回了错配的备份操作阶段");
  }
  return backup;
}

function requirePlan(payload: Record<string, unknown>, purpose: PlanPurpose, stage: string): ReviewedPlan {
  const envelope = requireBackupEnvelope(payload, stage);
  const value = envelope.plan;
  if (!isRecord(value)
    || value.schema !== planSchemas[purpose]
    || !isDigest(value.planDigest)) {
    throw new Error("Desktop 返回了无效或错配的精确备份计划");
  }
  return { purpose, digest: value.planDigest, schema: value.schema, value };
}

function shortDigest(value: string) {
  return `${value.slice(0, 14)}…${value.slice(-8)}`;
}

function planSummary(plan: ReviewedPlan) {
  const count = plan.value.entryCount;
  const bytes = plan.value.totalBytes;
  const state = plan.value.commitState;
  return [
    typeof count === "number" ? `${count.toLocaleString()} 项` : null,
    typeof bytes === "number" ? `${bytes.toLocaleString()} 字节` : null,
    typeof state === "string" ? `状态 ${state}` : null,
  ].filter(Boolean).join(" · ") || "计划结构与目标绑定已通过校验";
}

export default function BackupSurface({
  enabled,
  safeMode,
  blockedReason,
  sourceNodeId,
  destinationParentId,
  request,
  chooseDirectory,
  onWorkspaceChanged,
  onClose,
}: BackupSurfaceProps) {
  const [capabilitySnapshot, setCapabilitySnapshot] = useState<{
    safeMode: boolean;
    value: BackupCapabilities;
  } | null>(null);
  const [reviewed, setReviewed] = useState<ReviewedPlan | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [result, setResult] = useState("");
  const [protectionLabel, setProtectionLabel] = useState("重要恢复点");
  const [keepLatest, setKeepLatest] = useState(3);
  const [scope, setScope] = useState<"single_node" | "subtree">("single_node");
  const [scopeSource, setScopeSource] = useState(sourceNodeId);
  const [scopeParent, setScopeParent] = useState(destinationParentId);
  const [scopeName, setScopeName] = useState("Recovered");
  const capabilities = enabled && capabilitySnapshot?.safeMode === safeMode
    ? capabilitySnapshot.value
    : null;

  useEffect(() => {
    let current = true;
    if (!enabled) return () => { current = false; };
    void request("/api/backup/capabilities")
      .then((payload) => {
        const next = requireCapabilities(payload);
        if (next.safeMode !== safeMode || next.workspaceMutationAllowed !== !safeMode) {
          throw new Error("Desktop 备份能力与当前 Safe Mode 状态不一致");
        }
        if (current) {
          setError("");
          setCapabilitySnapshot({ safeMode, value: next });
        }
      })
      .catch((reason: unknown) => {
        if (current) setError(reason instanceof Error ? reason.message : "无法读取 Desktop 备份能力");
      });
    return () => { current = false; };
  }, [enabled, request, safeMode]);

  const authoritativeBlockedReason = blockedReason
    || (capabilities && !capabilities.savedSourceSetReady
      ? `Desktop 报告 ${capabilities.draftCount} 份设备草稿与 ${capabilities.recoveryIssueCount} 个恢复问题；完整备份与范围恢复保持关闭。`
      : "");

  function resetReview() {
    setReviewed(null);
    setConfirmed(false);
    setResult("");
  }

  async function select(kind: BackupDirectoryKind) {
    const selected = await chooseDirectory(kind);
    return selected ? requireGrant(selected, kind) : null;
  }

  async function run(action: () => Promise<void>) {
    if (!enabled || !capabilities || busy) return;
    setBusy(true);
    setError("");
    try {
      await action();
    } catch (reason) {
      resetReview();
      setError(reason instanceof Error ? reason.message : "Desktop 拒绝了备份操作");
    } finally {
      setBusy(false);
    }
  }

  async function previewBackup() {
    if (authoritativeBlockedReason) return;
    await run(async () => {
      const target = await select("backup_parent");
      if (!target) return;
      const payload = await request("/api/backup/preview", { backupParentCapability: target.capability });
      setReviewed(requirePlan(payload, "backup", "preview"));
      setConfirmed(false);
    });
  }

  async function verifySnapshot() {
    await run(async () => {
      const snapshot = await select("snapshot");
      if (!snapshot) return;
      const payload = await request("/api/backup/verify", { snapshotCapability: snapshot.capability });
      requireBackupEnvelope(payload, "verified");
      resetReview();
      setResult(`已验证完整快照：${snapshot.displayPath}`);
    });
  }

  async function protectSnapshot() {
    if (!protectionLabel.trim()) return;
    await run(async () => {
      const snapshot = await select("snapshot");
      if (!snapshot) return;
      const payload = await request("/api/backup/protect", {
        snapshotCapability: snapshot.capability,
        label: protectionLabel.trim(),
      });
      requireBackupEnvelope(payload, "protected");
      resetReview();
      setResult(`已保护恢复点“${protectionLabel.trim()}”`);
    });
  }

  async function previewRetention() {
    await run(async () => {
      const target = await select("backup_parent");
      if (!target) return;
      const payload = await request("/api/backup/retention/preview", {
        backupParentCapability: target.capability,
        keepLatestUnprotected: keepLatest,
      });
      setReviewed(requirePlan(payload, "retention", "retention_preview"));
      setConfirmed(false);
    });
  }

  async function recoverRetention() {
    await run(async () => {
      const target = await select("backup_parent");
      if (!target) return;
      const payload = await request("/api/backup/retention/recover", { backupParentCapability: target.capability });
      requireBackupEnvelope(payload, "retention_recovered");
      resetReview();
      setResult("保留策略中断证据已恢复或确认完成");
    });
  }

  async function previewAlternateRestore() {
    await run(async () => {
      const snapshot = await select("snapshot");
      if (!snapshot) return;
      const destination = await select("restore_parent");
      if (!destination) return;
      const payload = await request("/api/backup/restore/preview", {
        snapshotCapability: snapshot.capability,
        destinationParentCapability: destination.capability,
      });
      setReviewed(requirePlan(payload, "restore", "restore_preview"));
      setConfirmed(false);
    });
  }

  async function previewScopedRestore() {
    if (authoritativeBlockedReason || !scopeSource.trim() || !scopeParent.trim() || !scopeName.trim()) return;
    await run(async () => {
      const snapshot = await select("snapshot");
      if (!snapshot) return;
      const payload = await request("/api/backup/scoped-restore/preview", {
        snapshotCapability: snapshot.capability,
        sourceNodeId: scopeSource.trim(),
        destinationParentId: scopeParent.trim(),
        destinationName: scopeName.trim(),
        scope,
      });
      setReviewed(requirePlan(payload, "scoped", "scoped_restore_preview"));
      setConfirmed(false);
    });
  }

  async function previewDrill() {
    await run(async () => {
      const snapshot = await select("snapshot");
      if (!snapshot) return;
      const drillParent = await select("drill_parent");
      if (!drillParent) return;
      const resultsParent = await select("drill_results_parent");
      if (!resultsParent) return;
      const payload = await request("/api/backup/drill/preview", {
        snapshotCapability: snapshot.capability,
        drillParentCapability: drillParent.capability,
        resultsParentCapability: resultsParent.capability,
      });
      setReviewed(requirePlan(payload, "drill", "drill_preview"));
      setConfirmed(false);
    });
  }

  async function commitReviewedPlan() {
    if (!reviewed || !confirmed || (reviewed.purpose === "scoped" && safeMode)) return;
    await run(async () => {
      const route = commitRoutes[reviewed.purpose];
      const payload = await request(route.path, { planDigest: reviewed.digest });
      const envelope = requireBackupEnvelope(payload, route.stage);
      if (!isRecord(envelope.receipt)) throw new Error("Desktop 没有返回可验证的备份操作回执");
      const purpose = reviewed.purpose;
      resetReview();
      setResult(`${route.label}已完成并返回回执`);
      if (purpose === "scoped") await onWorkspaceChanged();
    });
  }

  const commitRoute = reviewed ? commitRoutes[reviewed.purpose] : null;
  const controlsUnavailable = !enabled || !capabilities || busy;
  const planBlockers = reviewed?.value.blockers;
  const planHasBlockers = Array.isArray(planBlockers) && planBlockers.length > 0;
  const coreBlocked = reviewed?.value.commitState === "blocked" || planHasBlockers;
  const sourceSetBlocked = Boolean(
    reviewed
      && (reviewed.purpose === "backup" || reviewed.purpose === "scoped")
      && authoritativeBlockedReason,
  );
  const commitBlocked = !enabled
    || !capabilities
    || !reviewed
    || !confirmed
    || coreBlocked
    || sourceSetBlocked
    || (reviewed.purpose === "scoped" && safeMode)
    || busy;

  return <section className="intake-surface backup-surface" role="dialog" aria-modal="true" aria-labelledby="backup-title">
    <header className="intake-heading">
      <div><span className="eyebrow">FULL PHYSICAL SAFETY</span><h2 id="backup-title">备份与恢复</h2></div>
      <button type="button" aria-label="关闭备份与恢复" onClick={onClose}>×</button>
    </header>
    <div className="backup-overview">
      <p>完整备份逐字节覆盖工作区，包括 ignored、unmanaged 与 Core 保留的 Trash item store。Trash 不是备份；恢复默认只写入新的干净位置。</p>
      {capabilities && <div className="core-connection-note"><span>持久契约</span><strong>{capabilities.managedShape} · node-local annotations · 系统目录授权</strong></div>}
      {authoritativeBlockedReason && <p className="safety-note" role="alert">{authoritativeBlockedReason}</p>}
      {safeMode && <p className="safety-note">Safe Mode 仍允许外部备份、验证和 alternate restore；写回当前工作区的范围恢复保持关闭。</p>}
      {error && <p className="safety-note" role="alert">{error}</p>}
      {result && <div className="core-connection-note" role="status"><span>完成</span><strong>{result}</strong></div>}
    </div>
    <div className="intake-grid">
      <aside className="intake-controls" aria-label="备份与恢复操作">
        <fieldset disabled={controlsUnavailable}>
          <legend>完整物理备份</legend>
          <button type="button" disabled={controlsUnavailable || Boolean(authoritativeBlockedReason)} onClick={() => void previewBackup()}>选择目标并预览完整备份</button>
          <button type="button" onClick={() => void verifySnapshot()}>选择快照并逐字节验证</button>
          <label>保护点标签<input aria-label="备份保护点标签" value={protectionLabel} onChange={(event) => setProtectionLabel(event.target.value)} /></label>
          <button type="button" disabled={!protectionLabel.trim()} onClick={() => void protectSnapshot()}>选择快照并保护</button>
        </fieldset>
        <fieldset disabled={controlsUnavailable}>
          <legend>保留策略</legend>
          <label>保留最新未保护快照<input aria-label="保留最新未保护快照" type="number" min={0} step={1} value={keepLatest} onChange={(event) => setKeepLatest(Math.max(0, Number.parseInt(event.target.value || "0", 10)))} /></label>
          <button type="button" onClick={() => void previewRetention()}>选择备份目录并预览保留策略</button>
          <button type="button" onClick={() => void recoverRetention()}>恢复中断的保留操作</button>
        </fieldset>
        <fieldset disabled={controlsUnavailable}>
          <legend>干净恢复与演练</legend>
          <button type="button" onClick={() => void previewAlternateRestore()}>预览到新的 alternate 工作区</button>
          <button type="button" onClick={() => void previewDrill()}>选择目录并预览恢复演练</button>
        </fieldset>
        <fieldset disabled={controlsUnavailable}>
          <legend>范围恢复到当前工作区</legend>
          <label>范围<select aria-label="范围恢复类型" value={scope} onChange={(event) => setScope(event.target.value as typeof scope)}><option value="single_node">单节点</option><option value="subtree">子树</option></select></label>
          <label>快照源节点 UUID<input aria-label="范围恢复源节点" value={scopeSource} onChange={(event) => setScopeSource(event.target.value)} /></label>
          <label>当前目标父节点 UUID<input aria-label="范围恢复目标父节点" value={scopeParent} onChange={(event) => setScopeParent(event.target.value)} /></label>
          <label>新节点名称<input aria-label="范围恢复目标名称" value={scopeName} onChange={(event) => setScopeName(event.target.value)} /></label>
          <button type="button" disabled={controlsUnavailable || Boolean(authoritativeBlockedReason) || !scopeSource.trim() || !scopeParent.trim() || !scopeName.trim()} onClick={() => void previewScopedRestore()}>选择快照并预览范围恢复</button>
        </fieldset>
      </aside>
      <div className="intake-preview backup-review" aria-live="polite">
        {!reviewed ? <div className="intake-empty"><strong>尚无待确认计划</strong><span>目录 capability 只使用一次；提交只发送当前精确 plan digest。</span></div> : <>
          <span className="eyebrow">EXACT REVIEW</span>
          <h3>{commitRoutes[reviewed.purpose].label}</h3>
          <div className="core-connection-note"><span>计划摘要</span><strong>{planSummary(reviewed)}</strong></div>
          <code>{reviewed.schema}</code>
          <code>{shortDigest(reviewed.digest)}</code>
          {Array.isArray(reviewed.value.blockers) && reviewed.value.blockers.length > 0 && <p className="safety-note" role="alert">Core 已标记此范围恢复计划不可提交；请查看 blocker 后改用干净 alternate restore。</p>}
          <label><input type="checkbox" checked={confirmed} onChange={(event) => setConfirmed(event.target.checked)} />我确认当前精确计划、目标与摘要；提交不会重新选目录或重建计划。</label>
          <button className="primary" type="button" disabled={commitBlocked} onClick={() => void commitReviewedPlan()}>{reviewed.purpose === "scoped" && safeMode ? "Safe Mode：当前工作区恢复已暂停" : coreBlocked ? "Core 安全边界阻止提交" : commitRoute?.label}</button>
        </>}
      </div>
    </div>
  </section>;
}
