"use client";

import { useState } from "react";

type ExportRequest = (path: string, body?: unknown) => Promise<Record<string, unknown>>;
type MetadataPolicy = "preserve_weftext" | "remove_weftext";
type ExportDestinationGrant = { capability: string; displayPath: string };

type ExportDiagnostic = {
  code: string;
  severity: "warning" | "omission";
  message: string;
  sourceStart: number | null;
  sourceEnd: number | null;
};

type ExportPlan = {
  contractVersion: string;
  planId: string;
  bundleDigest: string;
  baseWorkspaceRevision: string;
  sourceNodeId: string;
  sourceDocumentRevision: string;
  sourceProfile: string;
  sourceByteLength: number;
  semanticModelVersion: number;
  destination: string;
  metadataPolicy: MetadataPolicy;
  resourcePolicy: "external_references_only";
  mediaType: string;
  artifactDigest: string;
  artifact: string;
  diagnostics: ExportDiagnostic[];
  report: {
    exactBlocks: number;
    loweredBlocks: number;
    preservedLiteralBlocks: number;
    omittedBlocks: number;
  };
  components: Array<{ componentId: string; version: string }>;
};

type ExportReceipt = {
  contractVersion: string;
  createdAt: string;
  planId: string;
  planDigest: string;
  sourceNodeId: string;
  sourceDocumentRevision: string;
  baseWorkspaceRevision: string;
  destination: string;
  artifactDigest: string;
  artifactByteLength: number;
  status: string;
};

type ExportSurfaceProps = {
  enabled: boolean;
  safeMode: boolean;
  blockedReason: string;
  nodeId: string;
  nodeName: string;
  request: ExportRequest;
  chooseDestination(suggestedName: string): Promise<ExportDestinationGrant | null>;
  onCommitted(receipt: ExportReceipt): void;
  onClose(): void;
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isDestinationGrant(value: unknown): value is ExportDestinationGrant {
  return isRecord(value)
    && typeof value.capability === "string"
    && value.capability.length > 0
    && typeof value.displayPath === "string"
    && value.displayPath.length > 0;
}

function isFiniteNonNegative(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isDiagnostic(value: unknown): value is ExportDiagnostic {
  return isRecord(value)
    && typeof value.code === "string"
    && ["warning", "omission"].includes(String(value.severity))
    && typeof value.message === "string"
    && (value.sourceStart === null || isFiniteNonNegative(value.sourceStart))
    && (value.sourceEnd === null || isFiniteNonNegative(value.sourceEnd));
}

function isExportPlan(value: unknown): value is ExportPlan {
  if (!isRecord(value) || !isRecord(value.report)) return false;
  const report = value.report;
  return value.contractVersion === "weftext.export.markdown.v1"
    && typeof value.planId === "string"
    && typeof value.bundleDigest === "string"
    && typeof value.baseWorkspaceRevision === "string"
    && typeof value.sourceNodeId === "string"
    && typeof value.sourceDocumentRevision === "string"
    && typeof value.sourceProfile === "string"
    && isFiniteNonNegative(value.sourceByteLength)
    && isFiniteNonNegative(value.semanticModelVersion)
    && typeof value.destination === "string"
    && ["preserve_weftext", "remove_weftext"].includes(String(value.metadataPolicy))
    && value.resourcePolicy === "external_references_only"
    && value.mediaType === "text/markdown; charset=utf-8"
    && typeof value.artifactDigest === "string"
    && typeof value.artifact === "string"
    && Array.isArray(value.diagnostics)
    && value.diagnostics.every(isDiagnostic)
    && isFiniteNonNegative(report.exactBlocks)
    && isFiniteNonNegative(report.loweredBlocks)
    && isFiniteNonNegative(report.preservedLiteralBlocks)
    && isFiniteNonNegative(report.omittedBlocks)
    && Array.isArray(value.components)
    && value.components.every((component) => isRecord(component)
      && typeof component.componentId === "string"
      && typeof component.version === "string");
}

function isExportReceipt(value: unknown): value is ExportReceipt {
  return isRecord(value)
    && value.contractVersion === "weftext.export.receipt.v1"
    && typeof value.createdAt === "string"
    && typeof value.planId === "string"
    && typeof value.planDigest === "string"
    && typeof value.sourceNodeId === "string"
    && typeof value.sourceDocumentRevision === "string"
    && typeof value.baseWorkspaceRevision === "string"
    && typeof value.destination === "string"
    && typeof value.artifactDigest === "string"
    && isFiniteNonNegative(value.artifactByteLength)
    && value.status === "committed";
}

function safeMarkdownName(nodeName: string) {
  const portable = Array.from(nodeName.trim() || "Weftext export", (character) => {
    const scalar = character.codePointAt(0) ?? 0;
    return scalar < 32 || '<>:"/\\|?*'.includes(character) ? "-" : character;
  }).join("").replace(/[. ]+$/u, "");
  return `${portable || "Weftext export"}.md`;
}

function shortDigest(value: string) {
  return value.length > 24 ? `${value.slice(0, 14)}…${value.slice(-8)}` : value;
}

export default function ExportSurface({
  enabled,
  safeMode,
  blockedReason,
  nodeId,
  nodeName,
  request,
  chooseDestination,
  onCommitted,
  onClose,
}: ExportSurfaceProps) {
  const [metadataPolicy, setMetadataPolicy] = useState<MetadataPolicy>("preserve_weftext");
  const [destination, setDestination] = useState<ExportDestinationGrant | null>(null);
  const [plan, setPlan] = useState<ExportPlan | null>(null);
  const [receipt, setReceipt] = useState<ExportReceipt | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const previewBlocked = !enabled || Boolean(blockedReason) || !destination;
  const commitBlocked = safeMode || Boolean(blockedReason);

  function resetAuthority() {
    setPlan(null);
    setReceipt(null);
    setError("");
  }

  async function selectDestination() {
    if (!enabled || loading) return;
    try {
      const selected = await chooseDestination(safeMarkdownName(nodeName));
      if (!selected) return;
      if (!isDestinationGrant(selected)) throw new Error("Desktop 返回了无效的导出目标 capability");
      setDestination(selected);
      resetAuthority();
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "无法打开系统保存选择器");
    }
  }

  async function createPreview() {
    if (previewBlocked || loading || !destination) return;
    setLoading(true);
    setPlan(null);
    setReceipt(null);
    try {
      const payload = await request("/api/export/markdown/preview", {
        nodeId,
        destinationCapability: destination.capability,
        metadataPolicy,
      });
      const value = isRecord(payload.export) && payload.export.stage === "preview"
        ? payload.export.plan
        : null;
      if (!isExportPlan(value) || value.sourceNodeId !== nodeId || value.metadataPolicy !== metadataPolicy) {
        throw new Error("Core 返回了无效或错配的 Markdown Export Plan");
      }
      setPlan(value);
      setDestination(null);
      setError("");
    } catch (reason) {
      setDestination(null);
      setError(reason instanceof Error ? reason.message : "Core 无法生成 Markdown 导出预览");
    } finally {
      setLoading(false);
    }
  }

  async function commitPreview() {
    if (!plan || commitBlocked || loading || receipt) return;
    setLoading(true);
    try {
      const payload = await request("/api/export/commit", { planId: plan.planId });
      const value = isRecord(payload.export) && payload.export.stage === "committed"
        ? payload.export.receipt
        : null;
      if (!isExportReceipt(value)
        || value.planId !== plan.planId
        || value.planDigest !== plan.bundleDigest
        || value.artifactDigest !== plan.artifactDigest
        || value.destination !== plan.destination) {
        throw new Error("Core 返回了无效或错配的 Markdown 导出回执");
      }
      setReceipt(value);
      setError("");
      onCommitted(value);
    } catch (reason) {
      setPlan(null);
      setError(reason instanceof Error ? reason.message : "Core 拒绝导出提交；请重新预览");
    } finally {
      setLoading(false);
    }
  }

  return <section className="intake-surface export-surface" role="dialog" aria-modal="true" aria-labelledby="export-title">
    <header className="intake-heading">
      <div><span className="eyebrow">EXPLICIT COMPATIBILITY EXPORT</span><h2 id="export-title">导出 Markdown</h2></div>
      <button type="button" aria-label="关闭 Markdown 导出" onClick={onClose}>×</button>
    </header>
    <p>导出只生成工作区之外的 unmanaged Markdown；托管文档仍只有 X/X.adoc。预览固定精确字节与兼容性报告，确认时不会重新渲染，也不会覆盖已有文件。</p>
    <div className="intake-grid">
      <aside className="intake-controls" aria-label="Markdown 导出设置">
        <div className="export-source"><span>当前托管节点</span><strong>{nodeName}</strong><code>{nodeId}</code></div>
        <fieldset disabled={!enabled || loading || Boolean(receipt)}>
          <legend>Weftext 元数据</legend>
          <label><input type="radio" name="metadata-policy" checked={metadataPolicy === "preserve_weftext"} onChange={() => { setMetadataPolicy("preserve_weftext"); resetAuthority(); }} />保留兼容元数据</label>
          <label><input type="radio" name="metadata-policy" checked={metadataPolicy === "remove_weftext"} onChange={() => { setMetadataPolicy("remove_weftext"); resetAuthority(); }} />移除 Weftext 元数据</label>
        </fieldset>
        <label>外部目标文件<input aria-label="Markdown 导出目标" readOnly value={plan?.destination ?? receipt?.destination ?? destination?.displayPath ?? ""} placeholder="尚未选择；必须位于工作区之外" /></label>
        <button type="button" disabled={!enabled || loading || Boolean(receipt)} onClick={() => void selectDestination()}>使用系统选择器选择新文件</button>
        <div className="intake-preview-controls"><button className="primary" type="button" disabled={previewBlocked || loading || Boolean(receipt)} onClick={() => void createPreview()}>{loading && !plan ? "正在生成固定产物…" : "生成精确导出预览"}</button></div>
        <div className="export-guard"><strong>发布约束</strong><span>工作区外 · 仅 .md/.markdown · 仅新建 · 不覆盖 · 精确字节写后校验</span></div>
        {blockedReason && <p className="intake-warning" role="alert">{blockedReason}</p>}
        {safeMode && <p className="intake-warning" role="alert">安全模式允许预览，但不允许发布外部文件。</p>}
        {error && <p className="intake-error" role="alert">{error}</p>}
      </aside>

      <div className="intake-preview" aria-label="Markdown 导出预览">
        {!plan && !receipt && <div className="intake-empty"><strong>尚未生成预览</strong><span>选择工作区外的新文件后，Core 会显示精确 Markdown、降级与遗漏，以及全部 authority 摘要。</span></div>}
        {plan && <>
          <div className="intake-summary">
            <div><span>精确保留</span><strong>{plan.report.exactBlocks} 个块</strong><small>{plan.sourceByteLength.toLocaleString()} 源字节</small></div>
            <div><span>兼容降级</span><strong>{plan.report.loweredBlocks} 个块</strong><small>{plan.report.preservedLiteralBlocks} 个 literal 保留</small></div>
            <div><span>遗漏</span><strong>{plan.report.omittedBlocks} 个块</strong><small>{plan.diagnostics.length} 条诊断</small></div>
          </div>
          <details className="intake-authority" open><summary>固定 authority 与摘要</summary><dl><dt>Plan</dt><dd>{plan.planId}</dd><dt>Bundle</dt><dd>{plan.bundleDigest}</dd><dt>Artifact SHA-256</dt><dd>{plan.artifactDigest}</dd><dt>Workspace revision</dt><dd>{plan.baseWorkspaceRevision}</dd><dt>Document revision</dt><dd>{plan.sourceDocumentRevision}</dd><dt>Profile</dt><dd>{plan.sourceProfile}</dd><dt>Destination</dt><dd>{plan.destination}</dd></dl></details>
          {plan.diagnostics.length > 0 && <section className="intake-diagnostics" aria-label="Markdown 兼容性诊断"><h3>兼容性诊断</h3>{plan.diagnostics.map((diagnostic, index) => <div className={diagnostic.severity === "omission" ? "blocking" : "warning"} key={`${diagnostic.code}-${diagnostic.sourceStart ?? index}`}><strong>{diagnostic.severity === "omission" ? "遗漏" : "降级"} · {diagnostic.code}</strong><span>{diagnostic.message}</span><small>{diagnostic.sourceStart === null ? "文档级" : `源字节 ${diagnostic.sourceStart}–${diagnostic.sourceEnd}`}</small></div>)}</section>}
          <section className="export-artifact" aria-label="精确 Markdown 产物"><h3>精确 Markdown 产物</h3><label>只读固定字节<textarea aria-label="精确 Markdown 产物源码" readOnly value={plan.artifact} /></label><div><span>{new TextEncoder().encode(plan.artifact).length.toLocaleString()} 字节</span><code>{shortDigest(plan.artifactDigest)}</code></div></section>
          <section className="export-components" aria-label="导出组件证据"><h3>组件证据</h3>{plan.components.map((component) => <div key={component.componentId}><strong>{component.componentId}</strong><code>{component.version}</code></div>)}</section>
          {receipt && <section className="export-receipt" role="status" aria-label="Markdown 导出回执"><strong>已发布并完成精确字节校验</strong><span>{receipt.destination}</span><dl><dt>时间</dt><dd>{receipt.createdAt}</dd><dt>Plan digest</dt><dd>{receipt.planDigest}</dd><dt>Artifact SHA-256</dt><dd>{receipt.artifactDigest}</dd><dt>字节</dt><dd>{receipt.artifactByteLength.toLocaleString()}</dd></dl></section>}
          <footer className="intake-actions">{receipt ? <button className="primary" type="button" onClick={onClose}>完成</button> : <><button type="button" disabled={loading} onClick={() => setPlan(null)}>返回修改</button><button className="primary" type="button" disabled={loading || commitBlocked} onClick={() => void commitPreview()}>{loading ? "正在发布固定字节…" : commitBlocked ? "当前预览不能发布" : "确认并发布固定 Markdown"}</button></>}</footer>
        </>}
      </div>
    </div>
  </section>;
}
