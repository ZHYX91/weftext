"use client";

import { useEffect, useMemo, useRef, useState } from "react";

type IntakeRequest = (path: string, body?: unknown) => Promise<Record<string, unknown>>;

type ImportDiagnostic = {
  code: string;
  severity: "info" | "warning" | "blocking";
  message: string;
  irNodeId?: string | null;
};

type ImportNode = {
  id: string;
  kind: Record<string, unknown>;
  confidence: string | number;
  sourceLocations: Array<{ page?: number | null }>;
};

type ProposedResource = {
  locator: string;
  mediaType: string;
  byteLength: number;
  sha256: string;
  embedded: boolean;
};

type ProposedNode = {
  locator: string;
  nodeId: string;
  documentFile: string;
  exactAsciidoc: string;
  documentSha256: string;
  resourceReferences: string[];
  resources: ProposedResource[];
};

type ImportPreview = {
  bundleDigest: string;
  baseWorkspaceRevision: string;
  proposalDigest: string;
  source: {
    displayName: string;
    byteLength: number;
    sha256: string;
    detectedFormat: string;
    mismatchEvidence: string[];
  };
  probe: {
    adapter: { id?: string; name?: string; version?: string } | string;
    detectedFormat: string;
    encryption: string;
    safeToPlan: boolean;
    pageCount?: number | null;
    diagnostics: ImportDiagnostic[];
  };
  plan: {
    planId: string;
    destination: string;
    route: unknown;
    resourcePolicy: string;
    localOcrPolicy: string;
    agentEnhancement: unknown;
    egress: unknown;
  };
  document: {
    title: string;
    revision: string;
    nodes: ImportNode[];
    resources: Array<{ id: string; locator: string; mediaType: string; byteLength: number; sha256: string }>;
    diagnostics: ImportDiagnostic[];
  };
  proposal: {
    proposalId: string;
    destination: string;
    nodes: ProposedNode[];
    conflicts: string[];
    warnings: string[];
    omissions: string[];
  };
  receipt: {
    receiptId: string;
    localProvenance: unknown[];
    agentProvenance: unknown[];
    warnings: string[];
  };
};

type PdfCapability = {
  available: boolean;
  code: string;
  message: string;
  missingPinnedEvidence: string[];
  missingIsolationEvidence: string[];
  ambientNetworkAllowed: boolean;
};

type AgentEnhancementReview = {
  previewDigest: string;
  baseBundleDigest: string;
  selection: {
    provider: string;
    selectedNodeIds: string[];
    retention: string;
    redaction: string;
  };
  evidenceDigest: string;
  evidenceByteLength: number;
  evidence: Record<string, unknown>;
  authorizedPlan: { egress: unknown };
  networkExecuted: false;
  requiresExplicitEgressApproval: true;
};

type TaskImportDialect = "markdown_checklist_v1" | "obsidian_tasks_emoji_v1";
type TaskImportStatusType = "TODO" | "DONE" | "IN_PROGRESS" | "ON_HOLD" | "CANCELLED" | "NON_TASK";
type TaskImportStatusMapping = { symbol: string; name: string; statusType: TaskImportStatusType };
type TaskImportSettings = {
  dialect: TaskImportDialect;
  pluginVersion: string | null;
  globalFilter: string | null;
  indentationWidth: number;
  statuses: TaskImportStatusMapping[];
};
type TaskImportReview = { proposalId: string; proposalDigest: string; bundleDigest: string };
type TaskImportDiagnostic = {
  code: string;
  locator: string;
  message: string;
  range: { start: number; end: number };
};
type TaskImportIdentity = {
  locator: string;
  occurrenceRange: { start: number; end: number };
  legacyId: string | null;
  taskId: string;
};
type TaskImportDocumentPlan = {
  locator: string;
  sourceDigest: string;
  proposedSource: string;
  edits: Array<{ kind: string; sourceRange: { start: number; end: number }; replacement: string }>;
};
type TaskImportProposedNode = {
  sourceLocator: string | null;
  destinationLocator: string;
  nodeId: string;
  documentFile: string;
  exactAsciidoc: string;
  documentDigest: string;
};
type TaskImportPreview = {
  stage: "preview";
  adapter: "task_source_set";
  committable: boolean;
  review: TaskImportReview;
  bundle: {
    contractVersion: string;
    bundleDigest: string;
    baseWorkspaceRevision: string;
    destinationParentId: string;
    destinationParentLocator: string;
    destinationName: string;
    destinationRootLocator: string;
    sourceSetDigest: string;
    sourceDocuments: Array<{ locator: string; source: string }>;
    taskPlan: {
      profile: string;
      settings: TaskImportSettings;
      documents: TaskImportDocumentPlan[];
      identities: TaskImportIdentity[];
      diagnostics: TaskImportDiagnostic[];
    };
    nodes: TaskImportProposedNode[];
    proposalId: string;
    proposalDigest: string;
    previewCreatedAt: string;
  };
};
type TaskImportReceipt = {
  contractVersion: string;
  receiptId: string;
  createdAt: string;
  sourceSetDigest: string;
  reviewedBundleDigest: string;
  proposalId: string;
  proposalDigest: string;
  identities: TaskImportIdentity[];
  nodes: TaskImportProposedNode[];
  commonReceipts: Array<{ receiptId: string }>;
  transaction: { planId: string; revision: string };
};
type ReceiptDestinationGrant = { capability: string; displayPath: string };
type SelectedTaskFile = { file: File; locator: string };

type IntakeSurfaceProps = {
  enabled: boolean;
  safeMode: boolean;
  blockedReason: string;
  destinationParentId?: string;
  destinationParentName?: string;
  workspaceRevision?: string;
  request: IntakeRequest;
  chooseTaskReceiptDestination?(suggestedName: string): Promise<ReceiptDestinationGrant | null>;
  onCommitted(payload: Record<string, unknown>): Promise<void>;
  onClose(): void;
};

type ImportKind = "markdown" | "pdf" | "task";

const markdownStatuses: TaskImportStatusMapping[] = [
  { symbol: " ", name: "Open", statusType: "TODO" },
  { symbol: "x", name: "Closed", statusType: "DONE" },
  { symbol: "X", name: "Closed", statusType: "DONE" },
];

const obsidianStatuses: TaskImportStatusMapping[] = [
  { symbol: " ", name: "Todo", statusType: "TODO" },
  { symbol: "x", name: "Done", statusType: "DONE" },
  { symbol: "/", name: "In Progress", statusType: "IN_PROGRESS" },
  { symbol: ">", name: "Deferred", statusType: "ON_HOLD" },
  { symbol: "-", name: "Cancelled", statusType: "CANCELLED" },
  { symbol: "?", name: "Question", statusType: "NON_TASK" },
];

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === "string");
}

function isDiagnostic(value: unknown): value is ImportDiagnostic {
  return isRecord(value)
    && typeof value.code === "string"
    && typeof value.message === "string"
    && ["info", "warning", "blocking"].includes(String(value.severity));
}

function isProposedResource(value: unknown): value is ProposedResource {
  return isRecord(value)
    && typeof value.locator === "string"
    && typeof value.mediaType === "string"
    && typeof value.byteLength === "number"
    && typeof value.sha256 === "string"
    && typeof value.embedded === "boolean";
}

function isProposedNode(value: unknown): value is ProposedNode {
  return isRecord(value)
    && typeof value.locator === "string"
    && typeof value.nodeId === "string"
    && typeof value.documentFile === "string"
    && typeof value.exactAsciidoc === "string"
    && typeof value.documentSha256 === "string"
    && isStringArray(value.resourceReferences)
    && Array.isArray(value.resources)
    && value.resources.every(isProposedResource);
}

function isImportNode(value: unknown): value is ImportNode {
  return isRecord(value)
    && typeof value.id === "string"
    && isRecord(value.kind)
    && Array.isArray(value.sourceLocations);
}

function isImportPreview(value: unknown): value is ImportPreview {
  if (!isRecord(value)
    || typeof value.bundleDigest !== "string"
    || typeof value.baseWorkspaceRevision !== "string"
    || typeof value.proposalDigest !== "string"
    || !isRecord(value.source)
    || !isRecord(value.probe)
    || !isRecord(value.plan)
    || !isRecord(value.document)
    || !isRecord(value.proposal)
    || !isRecord(value.receipt)) return false;
  const source = value.source;
  const probe = value.probe;
  const plan = value.plan;
  const document = value.document;
  const proposal = value.proposal;
  const receipt = value.receipt;
  return typeof source.displayName === "string"
    && typeof source.byteLength === "number"
    && typeof source.sha256 === "string"
    && typeof source.detectedFormat === "string"
    && isStringArray(source.mismatchEvidence)
    && typeof probe.detectedFormat === "string"
    && typeof probe.encryption === "string"
    && typeof probe.safeToPlan === "boolean"
    && Array.isArray(probe.diagnostics)
    && probe.diagnostics.every(isDiagnostic)
    && typeof plan.planId === "string"
    && typeof plan.destination === "string"
    && typeof document.title === "string"
    && typeof document.revision === "string"
    && Array.isArray(document.nodes)
    && document.nodes.every(isImportNode)
    && Array.isArray(document.resources)
    && Array.isArray(document.diagnostics)
    && document.diagnostics.every(isDiagnostic)
    && typeof proposal.proposalId === "string"
    && typeof proposal.destination === "string"
    && Array.isArray(proposal.nodes)
    && proposal.nodes.every(isProposedNode)
    && isStringArray(proposal.conflicts)
    && isStringArray(proposal.warnings)
    && isStringArray(proposal.omissions)
    && typeof receipt.receiptId === "string"
    && Array.isArray(receipt.localProvenance)
    && Array.isArray(receipt.agentProvenance)
    && isStringArray(receipt.warnings);
}

function isPdfCapability(value: unknown): value is PdfCapability {
  return isRecord(value)
    && typeof value.available === "boolean"
    && typeof value.code === "string"
    && typeof value.message === "string"
    && isStringArray(value.missingPinnedEvidence)
    && isStringArray(value.missingIsolationEvidence)
    && typeof value.ambientNetworkAllowed === "boolean";
}

function isAgentEnhancementReview(value: unknown): value is AgentEnhancementReview {
  if (!isRecord(value)
    || typeof value.previewDigest !== "string"
    || typeof value.baseBundleDigest !== "string"
    || !isRecord(value.selection)
    || typeof value.evidenceDigest !== "string"
    || !Number.isSafeInteger(value.evidenceByteLength)
    || Number(value.evidenceByteLength) < 0
    || !isRecord(value.evidence)
    || !isRecord(value.authorizedPlan)
    || value.networkExecuted !== false
    || value.requiresExplicitEgressApproval !== true) return false;
  const selection = value.selection;
  return typeof selection.provider === "string"
    && isStringArray(selection.selectedNodeIds)
    && typeof selection.retention === "string"
    && typeof selection.redaction === "string"
    && "egress" in value.authorizedPlan;
}

function isFiniteOffset(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isSourceRange(value: unknown): value is { start: number; end: number } {
  return isRecord(value)
    && isFiniteOffset(value.start)
    && isFiniteOffset(value.end)
    && value.start <= value.end;
}

function isTaskStatusMapping(value: unknown): value is TaskImportStatusMapping {
  return isRecord(value)
    && typeof value.symbol === "string"
    && Array.from(value.symbol).length === 1
    && typeof value.name === "string"
    && value.name.length > 0
    && ["TODO", "DONE", "IN_PROGRESS", "ON_HOLD", "CANCELLED", "NON_TASK"].includes(String(value.statusType));
}

function isTaskSettings(value: unknown): value is TaskImportSettings {
  return isRecord(value)
    && ["markdown_checklist_v1", "obsidian_tasks_emoji_v1"].includes(String(value.dialect))
    && (value.pluginVersion === null || typeof value.pluginVersion === "string")
    && (value.globalFilter === null || typeof value.globalFilter === "string")
    && isFiniteOffset(value.indentationWidth)
    && value.indentationWidth >= 1
    && value.indentationWidth <= 8
    && Array.isArray(value.statuses)
    && value.statuses.every(isTaskStatusMapping);
}

function isTaskReview(value: unknown): value is TaskImportReview {
  return isRecord(value)
    && typeof value.proposalId === "string"
    && typeof value.proposalDigest === "string"
    && typeof value.bundleDigest === "string";
}

function isTaskIdentity(value: unknown): value is TaskImportIdentity {
  return isRecord(value)
    && typeof value.locator === "string"
    && isSourceRange(value.occurrenceRange)
    && (value.legacyId === null || typeof value.legacyId === "string")
    && typeof value.taskId === "string";
}

function isTaskDiagnostic(value: unknown): value is TaskImportDiagnostic {
  return isRecord(value)
    && typeof value.code === "string"
    && typeof value.locator === "string"
    && typeof value.message === "string"
    && isSourceRange(value.range);
}

function isTaskDocumentPlan(value: unknown): value is TaskImportDocumentPlan {
  return isRecord(value)
    && typeof value.locator === "string"
    && typeof value.sourceDigest === "string"
    && typeof value.proposedSource === "string"
    && Array.isArray(value.edits)
    && value.edits.every((edit) => isRecord(edit)
      && typeof edit.kind === "string"
      && isSourceRange(edit.sourceRange)
      && typeof edit.replacement === "string");
}

function isTaskProposedNode(value: unknown): value is TaskImportProposedNode {
  return isRecord(value)
    && (value.sourceLocator === null || typeof value.sourceLocator === "string")
    && typeof value.destinationLocator === "string"
    && typeof value.nodeId === "string"
    && typeof value.documentFile === "string"
    && typeof value.exactAsciidoc === "string"
    && typeof value.documentDigest === "string";
}

function isTaskImportPreview(value: unknown): value is TaskImportPreview {
  if (!isRecord(value)
    || value.stage !== "preview"
    || value.adapter !== "task_source_set"
    || typeof value.committable !== "boolean"
    || !isTaskReview(value.review)
    || !isRecord(value.bundle)) return false;
  const bundle = value.bundle;
  const taskPlan = bundle.taskPlan;
  if (!isRecord(taskPlan)) return false;
  return bundle.contractVersion === "weftext.task-import-bundle.v1"
    && typeof bundle.bundleDigest === "string"
    && typeof bundle.baseWorkspaceRevision === "string"
    && typeof bundle.destinationParentId === "string"
    && typeof bundle.destinationParentLocator === "string"
    && typeof bundle.destinationName === "string"
    && typeof bundle.destinationRootLocator === "string"
    && typeof bundle.sourceSetDigest === "string"
    && Array.isArray(bundle.sourceDocuments)
    && bundle.sourceDocuments.every((document) => isRecord(document)
      && typeof document.locator === "string"
      && typeof document.source === "string")
    && taskPlan.profile === "weftext.task-import.v1"
    && isTaskSettings(taskPlan.settings)
    && Array.isArray(taskPlan.documents)
    && taskPlan.documents.every(isTaskDocumentPlan)
    && Array.isArray(taskPlan.identities)
    && taskPlan.identities.every(isTaskIdentity)
    && Array.isArray(taskPlan.diagnostics)
    && taskPlan.diagnostics.every(isTaskDiagnostic)
    && Array.isArray(bundle.nodes)
    && bundle.nodes.every(isTaskProposedNode)
    && typeof bundle.proposalId === "string"
    && typeof bundle.proposalDigest === "string"
    && typeof bundle.previewCreatedAt === "string"
    && value.review.bundleDigest === bundle.bundleDigest
    && value.review.proposalId === bundle.proposalId
    && value.review.proposalDigest === bundle.proposalDigest
    && value.committable === (taskPlan.diagnostics.length === 0);
}

function isReceiptDestinationGrant(value: unknown): value is ReceiptDestinationGrant {
  return isRecord(value)
    && typeof value.capability === "string"
    && value.capability.length > 0
    && typeof value.displayPath === "string"
    && value.displayPath.length > 0;
}

function isTaskImportReceipt(value: unknown, review: TaskImportReview): value is TaskImportReceipt {
  return isRecord(value)
    && value.contractVersion === "weftext.task-import-receipt.v1"
    && typeof value.receiptId === "string"
    && typeof value.createdAt === "string"
    && typeof value.sourceSetDigest === "string"
    && value.reviewedBundleDigest === review.bundleDigest
    && value.proposalId === review.proposalId
    && value.proposalDigest === review.proposalDigest
    && Array.isArray(value.identities)
    && value.identities.every(isTaskIdentity)
    && Array.isArray(value.nodes)
    && value.nodes.every(isTaskProposedNode)
    && Array.isArray(value.commonReceipts)
    && value.commonReceipts.every((receipt) => isRecord(receipt) && typeof receipt.receiptId === "string")
    && isRecord(value.transaction)
    && typeof value.transaction.planId === "string"
    && typeof value.transaction.revision === "string";
}

function taskReceiptFromPayload(payload: Record<string, unknown>, review: TaskImportReview) {
  const imported = isRecord(payload.import) ? payload.import : null;
  if (!imported) return null;
  if (isTaskImportReceipt(imported.receipt, review)) return imported.receipt;
  const recovery = isRecord(imported.recovery) ? imported.recovery : null;
  const committed = recovery && isRecord(recovery.committed) ? recovery.committed : null;
  return committed && isTaskImportReceipt(committed.receipt, review) ? committed.receipt : null;
}

function portableTaskLocator(file: File) {
  const relative = typeof file.webkitRelativePath === "string" && file.webkitRelativePath
    ? file.webkitRelativePath
    : file.name;
  return relative.replaceAll("\\", "/");
}

function suggestedDestination(fileName: string) {
  const stem = fileName.replace(/\.[^.]+$/u, "").trim();
  const portable = Array.from(stem, (character) => {
    const scalar = character.codePointAt(0) ?? 0;
    return scalar < 32 || "<>:\"/\\|?*".includes(character) ? "-" : character;
  }).join("");
  return portable.replace(/[. ]+$/u, "") || "Imported";
}

function shortDigest(value: string) {
  return value.length > 18 ? `${value.slice(0, 12)}…${value.slice(-6)}` : value;
}

function enumLabel(value: unknown) {
  if (typeof value === "string") return value;
  if (!isRecord(value)) return "未声明";
  const tag = ["type", "mode", "kind", "adapterId", "id"].find((key) => typeof value[key] === "string");
  return tag ? String(value[tag]) : Object.keys(value).sort().join(", ") || "空";
}

function nodeKind(value: ImportNode["kind"]) {
  return typeof value.type === "string" ? value.type : Object.keys(value)[0] ?? "unknown";
}

function allDiagnostics(preview: ImportPreview) {
  return [...preview.probe.diagnostics, ...preview.document.diagnostics];
}

function flattenImportNodes(nodes: ImportNode[]): ImportNode[] {
  return nodes.flatMap((node) => {
    const children = Array.isArray(node.kind.children)
      ? node.kind.children.filter(isImportNode)
      : [];
    return [node, ...flattenImportNodes(children)];
  });
}

export default function IntakeSurface({
  enabled,
  safeMode,
  blockedReason,
  destinationParentId = "",
  destinationParentName = "当前节点",
  workspaceRevision = "",
  request,
  chooseTaskReceiptDestination,
  onCommitted,
  onClose,
}: IntakeSurfaceProps) {
  const [kind, setKind] = useState<ImportKind>("markdown");
  const [file, setFile] = useState<File | null>(null);
  const [taskFiles, setTaskFiles] = useState<SelectedTaskFile[]>([]);
  const [destination, setDestination] = useState("");
  const [retainOriginal, setRetainOriginal] = useState(false);
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [taskPreview, setTaskPreview] = useState<TaskImportPreview | null>(null);
  const [taskDialect, setTaskDialect] = useState<TaskImportDialect>("markdown_checklist_v1");
  const [pluginVersion, setPluginVersion] = useState("");
  const [globalFilter, setGlobalFilter] = useState("");
  const [indentationWidth, setIndentationWidth] = useState(4);
  const [statuses, setStatuses] = useState<TaskImportStatusMapping[]>(markdownStatuses);
  const [reviewConfirmed, setReviewConfirmed] = useState(false);
  const [receiptDestination, setReceiptDestination] = useState<ReceiptDestinationGrant | null>(null);
  const [taskReceipt, setTaskReceipt] = useState<TaskImportReceipt | null>(null);
  const [recoverable, setRecoverable] = useState(false);
  const [recoveryStatus, setRecoveryStatus] = useState("");
  const [pdfCapability, setPdfCapability] = useState<PdfCapability | null>(null);
  const [agentProvider, setAgentProvider] = useState("");
  const [agentRetention, setAgentRetention] = useState("delete-after-call");
  const [agentRedaction, setAgentRedaction] = useState("selected-ir-only");
  const [agentSelectedNodeIds, setAgentSelectedNodeIds] = useState<string[]>([]);
  const [agentReview, setAgentReview] = useState<AgentEnhancementReview | null>(null);
  const [agentEgressApproved, setAgentEgressApproved] = useState(false);
  const [agentPatchSource, setAgentPatchSource] = useState("");
  const [loading, setLoading] = useState(false);
  const [cancelRequested, setCancelRequested] = useState(false);
  const [error, setError] = useState("");
  const commitInFlight = useRef(false);

  useEffect(() => {
    let active = true;
    if (!enabled) {
      return () => { active = false; };
    }
    void request("/api/import/pdf-capability")
      .then((payload) => {
        const value = isRecord(payload.import) ? payload.import.capability : null;
        if (!isPdfCapability(value)) throw new Error("Core 返回了无效的 PDF capability 契约");
        if (active) setPdfCapability(value);
      })
      .catch((reason: unknown) => {
        if (active) setError(reason instanceof Error ? reason.message : "无法读取 PDF 导入能力");
      });
    return () => { active = false; };
  }, [enabled, request]);

  useEffect(() => {
    if (!workspaceRevision || taskReceipt) return;
    const stale = preview?.baseWorkspaceRevision && preview.baseWorkspaceRevision !== workspaceRevision;
    const staleTask = taskPreview?.bundle.baseWorkspaceRevision
      && taskPreview.bundle.baseWorkspaceRevision !== workspaceRevision;
    if (!stale && !staleTask) return;
    const invalidation = window.setTimeout(() => {
      setPreview(null);
      setTaskPreview(null);
      setAgentSelectedNodeIds([]);
      setAgentReview(null);
      setAgentEgressApproved(false);
      setAgentPatchSource("");
      setReviewConfirmed(false);
      setReceiptDestination(null);
      setRecoverable(false);
      setError("工作区修订已变化；旧导入预览已作废，请重新生成精确 review。");
    }, 0);
    return () => window.clearTimeout(invalidation);
  }, [preview?.baseWorkspaceRevision, taskPreview?.bundle.baseWorkspaceRevision, taskReceipt, workspaceRevision]);

  const diagnostics = useMemo(() => preview ? allDiagnostics(preview) : [], [preview]);
  const selectableAgentNodes = useMemo(
    () => preview ? flattenImportNodes(preview.document.nodes) : [],
    [preview],
  );
  const hasBlockingEvidence = Boolean(preview?.proposal.conflicts.length)
    || diagnostics.some((diagnostic) => diagnostic.severity === "blocking");
  const pdfUnavailable = kind === "pdf" && pdfCapability?.available !== true;
  const taskSettingsReady = taskDialect === "markdown_checklist_v1" || Boolean(pluginVersion.trim());
  const taskPreviewBlocked = kind === "task"
    && (!taskFiles.length || !destinationParentId || !destination.trim() || !taskSettingsReady || statuses.length === 0);
  const commitBlocked = safeMode || Boolean(blockedReason) || hasBlockingEvidence || Boolean(agentReview)
    || Boolean(taskPreview && !taskPreview.committable);

  function clearAgentAuthority() {
    setAgentSelectedNodeIds([]);
    setAgentReview(null);
    setAgentEgressApproved(false);
    setAgentPatchSource("");
  }

  function clearTaskAuthority() {
    setTaskPreview(null);
    setTaskReceipt(null);
    setReviewConfirmed(false);
    setReceiptDestination(null);
    setRecoverable(false);
    setRecoveryStatus("");
  }

  function chooseKind(next: ImportKind) {
    setKind(next);
    setFile(null);
    setTaskFiles([]);
    setDestination("");
    setPreview(null);
    clearAgentAuthority();
    clearTaskAuthority();
    setCancelRequested(false);
    setError("");
  }

  function chooseFile(selected: File | null) {
    setFile(selected);
    setPreview(null);
    clearAgentAuthority();
    setCancelRequested(false);
    setError("");
    if (selected) setDestination(suggestedDestination(selected.name));
  }

  function chooseTaskFiles(files: FileList | null) {
    const selected = Array.from(files ?? [], (selectedFile) => ({
      file: selectedFile,
      locator: portableTaskLocator(selectedFile),
    }));
    const folded = new Set<string>();
    const invalid = selected.find(({ locator }) => !locator.toLowerCase().endsWith(".md")
      || locator.startsWith("/")
      || locator.split("/").some((part) => !part || part === "." || part === "..")
      || !folded.add(locator.toLowerCase()));
    setTaskFiles(invalid ? [] : selected);
    clearTaskAuthority();
    setCancelRequested(false);
    setError(invalid ? `任务源 locator 不是唯一、可移植的 .md 路径：${invalid.locator}` : "");
    if (selected.length && !destination.trim()) setDestination("Imported tasks");
  }

  function updateTaskDialect(next: TaskImportDialect) {
    setTaskDialect(next);
    setStatuses(next === "markdown_checklist_v1" ? markdownStatuses : obsidianStatuses);
    setPluginVersion("");
    setGlobalFilter("");
    clearTaskAuthority();
    setError("");
  }

  function updateStatus(index: number, patch: Partial<TaskImportStatusMapping>) {
    setStatuses((current) => current.map((status, statusIndex) => statusIndex === index ? { ...status, ...patch } : status));
    clearTaskAuthority();
  }

  async function createPreview() {
    if (!enabled || !destination.trim() || pdfUnavailable || (kind === "task" ? taskPreviewBlocked : !file)) return;
    setLoading(true);
    setCancelRequested(false);
    setPreview(null);
    clearAgentAuthority();
    clearTaskAuthority();
    try {
      if (kind === "task") {
        const documents = await Promise.all(taskFiles.map(async ({ file: selectedFile, locator }) => ({
          locator,
          bytes: Array.from(new Uint8Array(await selectedFile.arrayBuffer())),
        })));
        const settings: TaskImportSettings = {
          dialect: taskDialect,
          pluginVersion: taskDialect === "obsidian_tasks_emoji_v1" ? pluginVersion.trim() : null,
          globalFilter: taskDialect === "obsidian_tasks_emoji_v1" ? globalFilter.trim() || null : null,
          indentationWidth,
          statuses,
        };
        const payload = await request("/api/import/task/preview", {
          profile: "weftext.task-import.v1",
          destinationParentId,
          destinationName: destination.trim(),
          settings,
          documents,
        });
        if (!isTaskImportPreview(payload.import)
          || JSON.stringify(payload.import.bundle.taskPlan.settings) !== JSON.stringify(settings)) {
          throw new Error("Core 返回了无效或错配的 Task Import Preview 契约");
        }
        setTaskPreview(payload.import);
        setError("");
        return;
      }
      if (!file) return;
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const path = kind === "markdown" ? "/api/import/markdown/preview" : "/api/import/pdf-preview";
      const payload = await request(path, {
        displayName: file.name,
        bytes,
        destination: destination.trim(),
        ...(kind === "markdown" ? { retainOriginal } : {}),
      });
      if (!isImportPreview(payload.import)) throw new Error("Core 返回了无效的 Import Preview 契约");
      setPreview(payload.import);
      clearAgentAuthority();
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Core 无法生成导入预览");
    } finally {
      setLoading(false);
    }
  }

  function toggleAgentNode(nodeId: string, selected: boolean) {
    setAgentSelectedNodeIds((current) => selected
      ? current.includes(nodeId) ? current : [...current, nodeId]
      : current.filter((id) => id !== nodeId));
    setAgentReview(null);
    setAgentEgressApproved(false);
    setAgentPatchSource("");
  }

  async function prepareAgentReview() {
    if (!preview
      || kind !== "pdf"
      || loading
      || !agentProvider.trim()
      || agentSelectedNodeIds.length === 0) return;
    setLoading(true);
    setAgentReview(null);
    setAgentEgressApproved(false);
    setAgentPatchSource("");
    try {
      const payload = await request("/api/import/agent/prepare", {
        bundleDigest: preview.bundleDigest,
        provider: agentProvider.trim(),
        selectedNodeIds: agentSelectedNodeIds,
        retention: agentRetention,
        redaction: agentRedaction,
      });
      const review = payload.agentEnhancement;
      if (!isAgentEnhancementReview(review)
        || review.baseBundleDigest !== preview.bundleDigest
        || review.selection.provider !== agentProvider.trim()
        || JSON.stringify(review.selection.selectedNodeIds) !== JSON.stringify(agentSelectedNodeIds)
        || review.selection.retention !== agentRetention
        || review.selection.redaction !== agentRedaction) {
        throw new Error("Core 返回了无效或错配的 Agent evidence review 契约");
      }
      setAgentReview(review);
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Core 无法冻结 Agent evidence review");
    } finally {
      setLoading(false);
    }
  }

  async function applyAgentPatch() {
    if (!agentReview || !agentEgressApproved || !agentPatchSource.trim() || loading) return;
    setLoading(true);
    try {
      const patch: unknown = JSON.parse(agentPatchSource);
      if (!isRecord(patch)) throw new Error("Agent 必须返回一个 typed patch JSON object");
      const payload = await request("/api/import/agent/apply-approved-patch", {
        previewDigest: agentReview.previewDigest,
        egressApproved: true,
        patch,
      });
      const enhanced = payload.import;
      if (!isImportPreview(enhanced)
        || enhanced.bundleDigest === agentReview.baseBundleDigest
        || enhanced.receipt.agentProvenance.length === 0
        || enhanced.plan.egress === undefined) {
        throw new Error("Core 返回了无效或未归因的 Agent-enhanced preview");
      }
      setPreview(enhanced);
      clearAgentAuthority();
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "Core 拒绝了 Agent typed IR patch");
    } finally {
      setLoading(false);
    }
  }

  async function selectTaskReceiptDestination() {
    if (!taskPreview || loading || taskReceipt || !chooseTaskReceiptDestination) return;
    try {
      const selected = await chooseTaskReceiptDestination(
        `task-import-${taskPreview.review.bundleDigest.slice(0, 12)}.receipt.json`,
      );
      if (!selected) return;
      if (!isReceiptDestinationGrant(selected)) {
        throw new Error("Desktop 返回了无效的 task import receipt capability");
      }
      setReceiptDestination(selected);
      setReviewConfirmed(false);
      setError("");
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "无法打开 task import receipt 系统保存选择器");
    }
  }

  async function cancelPreview() {
    if (!loading || cancelRequested) return;
    setCancelRequested(true);
    try {
      const payload = await request("/api/import/cancel");
      if (payload.cancelRequested !== true) {
        setError("本地导入任务已经结束；没有需要取消的 worker");
      }
    } catch (reason) {
      setCancelRequested(false);
      setError(reason instanceof Error ? reason.message : "无法请求取消本地导入");
    }
  }

  async function commitPreview() {
    if (commitInFlight.current || commitBlocked) return;
    if (taskPreview) {
      if (!reviewConfirmed || !receiptDestination || taskReceipt) return;
      commitInFlight.current = true;
      setLoading(true);
      setRecoverable(false);
      try {
        const payload = await request("/api/import/task/commit", {
          review: taskPreview.review,
          receiptDestinationCapability: receiptDestination.capability,
        });
        const receipt = taskReceiptFromPayload(payload, taskPreview.review);
        if (!receipt || receipt.sourceSetDigest !== taskPreview.bundle.sourceSetDigest) {
          throw new Error("Core 返回了无效或错配的 task import receipt");
        }
        await onCommitted(payload);
        setTaskReceipt(receipt);
        setReceiptDestination(null);
        setReviewConfirmed(false);
        setRecoveryStatus("");
        setError("");
      } catch (reason) {
        const message = reason instanceof Error ? reason.message : "Core 拒绝了任务源集合提交";
        const canRecover = message.includes("可尝试恢复");
        setRecoverable(canRecover);
        if (canRecover) setReceiptDestination(null);
        setReviewConfirmed(false);
        setError(message);
      } finally {
        commitInFlight.current = false;
        setLoading(false);
      }
      return;
    }
    if (!preview) return;
    commitInFlight.current = true;
    setLoading(true);
    try {
      const payload = await request("/api/import/commit", { bundleDigest: preview.bundleDigest });
      setPreview(null);
      setFile(null);
      await onCommitted(payload);
      setError("");
      onClose();
    } catch (reason) {
      setPreview(null);
      setError(reason instanceof Error ? reason.message : "Core 拒绝了导入提交；请重新预览");
    } finally {
      commitInFlight.current = false;
      setLoading(false);
    }
  }

  async function recoverTaskImport() {
    if (!taskPreview || !recoverable || commitInFlight.current || safeMode || blockedReason) return;
    commitInFlight.current = true;
    setLoading(true);
    try {
      const payload = await request("/api/import/task/recover", { review: taskPreview.review });
      const imported = isRecord(payload.import) ? payload.import : null;
      const recovery = imported && isRecord(imported.recovery) ? imported.recovery : null;
      const status = recovery && typeof recovery.status === "string" ? recovery.status : "";
      if (!["receipt_recovered", "already_finalized", "rolled_back"].includes(status)) {
        throw new Error("Core 返回了无效的 task import recovery 契约");
      }
      await onCommitted(payload);
      const receipt = taskReceiptFromPayload(payload, taskPreview.review);
      setRecoverable(false);
      setError("");
      if (receipt) {
        setTaskReceipt(receipt);
        setRecoveryStatus(status === "receipt_recovered" ? "已恢复 receipt 并完成事务" : "事务与 receipt 已完成");
      } else {
        setTaskPreview(null);
        setRecoveryStatus("未完成的任务源集合事务已精确回滚；请重新预览。");
      }
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : "无法恢复任务源集合事务");
    } finally {
      commitInFlight.current = false;
      setLoading(false);
    }
  }

  const anyPreview = Boolean(preview || taskPreview || taskReceipt);
  const previewButtonDisabled = !enabled || !destination.trim() || loading || pdfUnavailable
    || (kind === "task" ? taskPreviewBlocked : !file);

  return <section
    className={`intake-surface ${kind === "task" ? "task-intake-surface" : ""}`}
    role="dialog"
    aria-modal="true"
    aria-labelledby="intake-title"
  >
    <header className="intake-heading">
      <div><span className="eyebrow">{kind === "task" ? "FROZEN TASK SOURCE SET" : "COMMON IMPORT IR"}</span><h2 id="intake-title">导入内容</h2></div>
      <button type="button" aria-label="关闭导入中心" onClick={onClose}>×</button>
    </header>
    <p>{kind === "task"
      ? "任务源集合先冻结所有选中文档、dialect 设置、identity mapping 与逐文档 AsciiDoc；二次确认只消费当前 exact review，不会重新运行 importer 或重建身份。"
      : "源文件先在受限适配器中转换为 Weftext Import IR，再由 Core 生成完整、可复核的 AsciiDoc 与资源计划。确认只提交当前显示的固定字节，不会重新运行转换。"}</p>
    {!enabled && <p className="intake-error" role="alert">导入中心只对 Desktop 中已打开的 Weftext AsciiDoc 工作区开放。</p>}
    <div className="intake-grid">
      <aside className="intake-controls" aria-label="导入设置">
        <fieldset disabled={!enabled || loading || Boolean(taskReceipt)}>
          <legend>来源格式</legend>
          <label><input type="radio" name="import-kind" checked={kind === "markdown"} onChange={() => chooseKind("markdown")} />Markdown（显式兼容导入）</label>
          <label><input type="radio" name="import-kind" checked={kind === "pdf"} onChange={() => chooseKind("pdf")} />PDF（docling.rs Lite）</label>
          <label><input type="radio" name="import-kind" checked={kind === "task"} onChange={() => chooseKind("task")} />任务源集合（Markdown / Obsidian Tasks）</label>
        </fieldset>
        {kind === "pdf" && pdfCapability && <div className={`intake-capability ${pdfCapability.available ? "ready" : "blocked"}`} role="status">
          <strong>{pdfCapability.available ? "本地 PDF worker 已验证" : "本机尚不能执行 PDF 导入"}</strong>
          <span>{pdfCapability.message}</span>
          <small>环境网络：{pdfCapability.ambientNetworkAllowed ? "允许" : "禁止"} · {pdfCapability.code}</small>
          {!pdfCapability.available && <details><summary>缺少的固定证据</summary><ul>{[...pdfCapability.missingPinnedEvidence, ...pdfCapability.missingIsolationEvidence].map((item) => <li key={item}>{item}</li>)}</ul></details>}
        </div>}

        {kind === "task" ? <>
          <fieldset disabled={!enabled || loading || Boolean(taskReceipt)}>
            <legend>TaskImportSettings.dialect</legend>
            <label><input type="radio" name="task-dialect" checked={taskDialect === "markdown_checklist_v1"} onChange={() => updateTaskDialect("markdown_checklist_v1")} />markdown_checklist_v1</label>
            <label><input type="radio" name="task-dialect" checked={taskDialect === "obsidian_tasks_emoji_v1"} onChange={() => updateTaskDialect("obsidian_tasks_emoji_v1")} />obsidian_tasks_emoji_v1</label>
          </fieldset>
          {taskDialect === "obsidian_tasks_emoji_v1" && <label>Obsidian Tasks 插件版本<input aria-label="Obsidian Tasks 插件版本" value={pluginVersion} disabled={loading || Boolean(taskReceipt)} onChange={(event) => { setPluginVersion(event.target.value); clearTaskAuthority(); }} placeholder="例如 7.21.0（精确版本）" /></label>}
          <label>Global filter（Obsidian 可空；Markdown 固定 null）<input aria-label="任务全局过滤器" value={globalFilter} disabled={taskDialect === "markdown_checklist_v1" || loading || Boolean(taskReceipt)} onChange={(event) => { setGlobalFilter(event.target.value); clearTaskAuthority(); }} placeholder="例如 #task" /></label>
          <label>缩进宽度<input aria-label="任务缩进宽度" type="number" min={1} max={8} value={indentationWidth} disabled={loading || Boolean(taskReceipt)} onChange={(event) => { setIndentationWidth(Number(event.target.value)); clearTaskAuthority(); }} /></label>
          <div className="task-status-mapping" aria-label="任务状态映射">
            <strong>Status mapping</strong>
            {statuses.map((status, index) => <div key={`${index}-${status.symbol}`}>
              <input aria-label={`状态 ${index + 1} 符号`} maxLength={1} readOnly={taskDialect === "markdown_checklist_v1"} value={status.symbol} onChange={(event) => updateStatus(index, { symbol: Array.from(event.target.value)[0] ?? "" })} />
              <input aria-label={`状态 ${index + 1} 名称`} readOnly={taskDialect === "markdown_checklist_v1"} value={status.name} onChange={(event) => updateStatus(index, { name: event.target.value })} />
              <select aria-label={`状态 ${index + 1} 含义`} disabled={taskDialect === "markdown_checklist_v1" || loading || Boolean(taskReceipt)} value={status.statusType} onChange={(event) => updateStatus(index, { statusType: event.target.value as TaskImportStatusType })}>
                {(["TODO", "DONE", "IN_PROGRESS", "ON_HOLD", "CANCELLED", "NON_TASK"] as TaskImportStatusType[]).map((value) => <option key={value}>{value}</option>)}
              </select>
              {taskDialect === "obsidian_tasks_emoji_v1" && <button type="button" aria-label={`移除状态 ${index + 1}`} disabled={loading || statuses.length === 1 || Boolean(taskReceipt)} onClick={() => { setStatuses((current) => current.filter((_, itemIndex) => itemIndex !== index)); clearTaskAuthority(); }}>×</button>}
            </div>)}
            {taskDialect === "obsidian_tasks_emoji_v1" && <button type="button" disabled={loading || Boolean(taskReceipt)} onClick={() => { setStatuses((current) => [...current, { symbol: "!", name: "Custom", statusType: "TODO" }]); clearTaskAuthority(); }}>添加 status mapping</button>}
          </div>
          <label>任务源 Markdown 文件<input aria-label="选择任务源文件" type="file" multiple accept=".md,text/markdown,text/plain" disabled={!enabled || loading || Boolean(taskReceipt)} onChange={(event) => chooseTaskFiles(event.currentTarget.files)} /></label>
          <div className="task-source-files" aria-label="任务源 locator 列表">{taskFiles.map(({ file: selectedFile, locator }) => <div key={locator}><strong>{locator}</strong><span>{selectedFile.size.toLocaleString()} 字节</span></div>)}</div>
          <div className="export-source"><span>目标父节点</span><strong>{destinationParentName}</strong><code>{destinationParentId || "未选择"}</code></div>
        </> : <label>源文件<input aria-label="选择导入文件" type="file" accept={kind === "markdown" ? ".md,.markdown,text/markdown,text/plain" : ".pdf,application/pdf"} disabled={!enabled || loading || pdfUnavailable} onChange={(event) => chooseFile(event.currentTarget.files?.[0] ?? null)} /></label>}

        <label>{kind === "task" ? "导入集合节点名称" : "目标节点路径"}<input
          aria-label={kind === "task" ? "导入集合节点名称" : "目标节点路径"}
          value={destination}
          disabled={!enabled || loading || Boolean(taskReceipt)}
          onChange={(event) => { setDestination(event.target.value); setPreview(null); clearAgentAuthority(); clearTaskAuthority(); }}
          onKeyDown={(event) => {
            if (event.key === "Escape" && !event.nativeEvent.isComposing && !loading) {
              onClose();
              return;
            }
            if (event.key === "Enter" && !event.nativeEvent.isComposing && !previewButtonDisabled) {
              event.preventDefault();
              void createPreview();
            }
          }}
          placeholder={kind === "task" ? "Imported tasks" : "Imported/Document"}
        /></label>
        {kind === "markdown" && <label className="intake-checkbox"><input type="checkbox" checked={retainOriginal} disabled={!enabled || loading} onChange={(event) => { setRetainOriginal(event.target.checked); setPreview(null); }} />把原始 Markdown 作为可见节点资源保留</label>}
        <div className="intake-preview-controls"><button className="primary" type="button" disabled={previewButtonDisabled || Boolean(taskReceipt)} onClick={() => void createPreview()}>{loading && !anyPreview ? "正在本地转换…" : kind === "task" ? "生成 exact source-set review" : "生成完整 Core 预览"}</button>{loading && !anyPreview && <button type="button" disabled={cancelRequested} onClick={() => void cancelPreview()}>{cancelRequested ? "正在停止并清理…" : "取消转换"}</button>}</div>
        {file && kind !== "task" && <div className="intake-source-file"><strong>{file.name}</strong><span>{file.size.toLocaleString()} 字节 · 文件路径不会进入 Import IR</span></div>}
        {(safeMode || blockedReason) && <p className="intake-warning" role="alert">{safeMode ? kind === "task" ? "安全模式已启用；可以预览，但不能提交或恢复。" : "安全模式已启用；可以预览，但不能提交。" : blockedReason}</p>}
        {recoverable && <button type="button" className="task-recover-button" disabled={loading || safeMode || Boolean(blockedReason)} onClick={() => void recoverTaskImport()}>恢复已开始的 exact task import</button>}
        {recoveryStatus && <p className="intake-warning" role="status">{recoveryStatus}</p>}
        {error && <p className="intake-error" role="alert">{error}</p>}
      </aside>

      <div className="intake-preview" aria-label="导入预览">
        {!preview && !taskPreview && !taskReceipt && <div className="intake-empty"><strong>尚未生成预览</strong><span>选择文件与目标后，Core 会展示精确节点源码、identity mapping、阻塞诊断和 receipt authority。</span></div>}

        {taskPreview && <>
          <div className="intake-summary">
            <div><span>Source set</span><strong>{taskPreview.bundle.sourceDocuments.length} 个文档</strong><small>{taskPreview.bundle.sourceDocuments.reduce((total, document) => total + new TextEncoder().encode(document.source).length, 0).toLocaleString()} UTF-8 字节</small></div>
            <div><span>目标</span><strong>{taskPreview.bundle.destinationRootLocator}</strong><small>{taskPreview.bundle.nodes.length} 个 canonical 节点</small></div>
            <div><span>状态</span><strong>{taskPreview.committable ? "可提交" : "被诊断阻塞"}</strong><small>{taskPreview.bundle.taskPlan.identities.length} 个 identity mapping</small></div>
          </div>
          <details className="intake-authority" open><summary>Exact review authority</summary><dl><dt>Contract</dt><dd>{taskPreview.bundle.contractVersion}</dd><dt>Source-set SHA-256</dt><dd>{taskPreview.bundle.sourceSetDigest}</dd><dt>Bundle SHA-256</dt><dd>{taskPreview.review.bundleDigest}</dd><dt>Proposal ID</dt><dd>{taskPreview.review.proposalId}</dd><dt>Proposal SHA-256</dt><dd>{taskPreview.review.proposalDigest}</dd><dt>Workspace revision</dt><dd>{taskPreview.bundle.baseWorkspaceRevision}</dd><dt>Preview timestamp</dt><dd>{taskPreview.bundle.previewCreatedAt}</dd></dl></details>
          <section className="task-settings-review" aria-label="冻结的 TaskImportSettings"><h3>冻结的 TaskImportSettings</h3><dl><dt>dialect</dt><dd>{taskPreview.bundle.taskPlan.settings.dialect}</dd><dt>pluginVersion</dt><dd>{taskPreview.bundle.taskPlan.settings.pluginVersion ?? "null"}</dd><dt>globalFilter</dt><dd>{taskPreview.bundle.taskPlan.settings.globalFilter ?? "null"}</dd><dt>indentationWidth</dt><dd>{taskPreview.bundle.taskPlan.settings.indentationWidth}</dd></dl><div>{taskPreview.bundle.taskPlan.settings.statuses.map((status) => <code key={`${status.symbol}-${status.name}`}>{JSON.stringify(status.symbol)} → {status.name} / {status.statusType}</code>)}</div></section>
          {taskPreview.bundle.taskPlan.diagnostics.length > 0 && <section className="intake-diagnostics" aria-label="任务源集合阻塞诊断"><h3>阻塞诊断</h3>{taskPreview.bundle.taskPlan.diagnostics.map((diagnostic, index) => <div className="blocking" key={`${diagnostic.locator}-${diagnostic.range.start}-${index}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>{diagnostic.locator} · UTF-8 bytes {diagnostic.range.start}–{diagnostic.range.end}</small></div>)}</section>}
          <section className="task-identities" aria-label="任务 identity mappings"><h3>Identity mappings</h3>{taskPreview.bundle.taskPlan.identities.length ? taskPreview.bundle.taskPlan.identities.map((identity) => <div key={`${identity.locator}-${identity.occurrenceRange.start}`}><strong>{identity.legacyId ?? "generated"}</strong><code>{identity.taskId}</code><small>{identity.locator} · {identity.occurrenceRange.start}–{identity.occurrenceRange.end}</small></div>) : <p>此 source set 不需要生成结构化 task identity。</p>}</section>
          <section className="intake-proposed-nodes" aria-label="任务源集合逐文档拟议 AsciiDoc"><h3>逐文档 exact proposed AsciiDoc</h3>{taskPreview.bundle.nodes.map((node) => <article key={node.nodeId}>
            <header><div><strong>{node.sourceLocator ?? "合成父节点"} → {node.destinationLocator}</strong><code>{node.documentFile} · {node.nodeId}</code></div><small>{shortDigest(node.documentDigest)}</small></header>
            <label>Exact proposed AsciiDoc<textarea aria-label={`${node.destinationLocator} exact proposed AsciiDoc`} readOnly value={node.exactAsciidoc} /></label>
          </article>)}</section>
          {!taskReceipt && taskPreview.committable && <section className="task-review-confirmation" aria-label="任务导入二次确认">
            <h3>Receipt 与二次确认</h3>
            <label>系统授权的新 receipt 文件<input aria-label="Task import receipt 目标" readOnly value={receiptDestination?.displayPath ?? ""} placeholder="路径不会进入浏览器请求；只发送一次性 capability" /></label>
            <button type="button" disabled={loading || !chooseTaskReceiptDestination} onClick={() => void selectTaskReceiptDestination()}>使用系统选择器授权新 JSON receipt</button>
            <label className="intake-checkbox"><input type="checkbox" checked={reviewConfirmed} disabled={!receiptDestination || loading || commitBlocked} onChange={(event) => setReviewConfirmed(event.target.checked)} />我已逐项核对当前 source-set digest、identity mappings、诊断与 exact AsciiDoc，并确认提交这份 exact review。</label>
          </section>}
          {taskReceipt && <section className="export-receipt" role="status" aria-label="Task import receipt"><strong>任务源集合已提交，receipt 已精确发布</strong><span>{taskReceipt.receiptId}</span><dl><dt>时间</dt><dd>{taskReceipt.createdAt}</dd><dt>Source set</dt><dd>{taskReceipt.sourceSetDigest}</dd><dt>Reviewed bundle</dt><dd>{taskReceipt.reviewedBundleDigest}</dd><dt>Proposal</dt><dd>{taskReceipt.proposalDigest}</dd><dt>Transaction</dt><dd>{taskReceipt.transaction.planId}</dd><dt>Workspace revision</dt><dd>{taskReceipt.transaction.revision}</dd><dt>Common receipts</dt><dd>{taskReceipt.commonReceipts.map((receipt) => receipt.receiptId).join(", ")}</dd></dl></section>}
          <footer className="intake-actions">{taskReceipt ? <button className="primary" type="button" onClick={onClose}>完成</button> : <><button type="button" disabled={loading} onClick={() => clearTaskAuthority()}>返回修改</button><button className="primary" type="button" disabled={loading || commitBlocked || !reviewConfirmed || !receiptDestination} onClick={() => void commitPreview()}>{loading ? "正在提交 exact review…" : commitBlocked ? "当前 review 不能提交" : "二次确认并提交 exact review"}</button></>}</footer>
        </>}

        {preview && <>
          <div className="intake-summary">
            <div><span>来源</span><strong>{preview.source.displayName}</strong><small>{preview.source.detectedFormat} · {preview.source.byteLength.toLocaleString()} 字节</small></div>
            <div><span>目标</span><strong>{preview.proposal.destination}</strong><small>{preview.proposal.nodes.length} 个节点 · {preview.document.resources.length} 个 IR 资源</small></div>
            <div><span>本地处理</span><strong>{enumLabel(preview.plan.route)}</strong><small>{preview.receipt.localProvenance.length} 条 provenance · agent {preview.receipt.agentProvenance.length}</small></div>
          </div>
          <details className="intake-authority"><summary>固定 authority 与摘要</summary><dl><dt>Bundle</dt><dd>{preview.bundleDigest}</dd><dt>Proposal</dt><dd>{preview.proposalDigest}</dd><dt>IR revision</dt><dd>{preview.document.revision}</dd><dt>Source SHA-256</dt><dd>{preview.source.sha256}</dd><dt>Workspace revision</dt><dd>{preview.baseWorkspaceRevision}</dd><dt>Receipt</dt><dd>{preview.receipt.receiptId}</dd></dl></details>
          {diagnostics.length > 0 && <section className="intake-diagnostics" aria-label="导入诊断"><h3>诊断</h3>{diagnostics.map((diagnostic, index) => <div className={diagnostic.severity} key={`${diagnostic.code}-${diagnostic.irNodeId ?? index}`}><strong>{diagnostic.code}</strong><span>{diagnostic.message}</span><small>{diagnostic.severity}{diagnostic.irNodeId ? ` · IR ${diagnostic.irNodeId}` : ""}</small></div>)}</section>}
          {[...preview.source.mismatchEvidence, ...preview.proposal.conflicts, ...preview.proposal.warnings, ...preview.proposal.omissions, ...preview.receipt.warnings].length > 0 && <section className="intake-findings" aria-label="冲突、警告与遗漏">
            {preview.proposal.conflicts.map((item) => <div className="conflict" key={`conflict-${item}`}><strong>阻塞冲突</strong><span>{item}</span></div>)}
            {preview.source.mismatchEvidence.map((item) => <div key={`mismatch-${item}`}><strong>格式不匹配</strong><span>{item}</span></div>)}
            {preview.proposal.warnings.map((item) => <div key={`warning-${item}`}><strong>警告</strong><span>{item}</span></div>)}
            {preview.proposal.omissions.map((item) => <div key={`omission-${item}`}><strong>遗漏</strong><span>{item}</span></div>)}
            {preview.receipt.warnings.map((item) => <div key={`receipt-${item}`}><strong>Receipt</strong><span>{item}</span></div>)}
          </section>}
          <section className="intake-ir" aria-label="Import IR 节点"><h3>Import IR</h3><div>{preview.document.nodes.map((node) => <span key={node.id}><strong>{nodeKind(node.kind)}</strong><code>{node.id}</code><small>confidence {String(node.confidence)}{node.sourceLocations.some((location) => location.page) ? ` · 页 ${node.sourceLocations.map((location) => location.page).filter(Boolean).join(", ")}` : ""}</small></span>)}</div></section>
          {kind === "pdf" && preview.receipt.agentProvenance.length === 0 && <section className="agent-enhancement-review" aria-label="Agent typed IR patch review">
            <h3>可选 Agent 增强（仅选中的 IR 证据）</h3>
            <p>Weftext 不发送整份 PDF，也不接受 AsciiDoc 重写。先冻结选中的 IR/资源、provider、保留和脱敏策略；当前界面不会自行联网，Agent 只能返回与此 review 绑定的 typed patch。</p>
            <div className="agent-node-selection" role="group" aria-label="选择可离机的 IR 节点">{selectableAgentNodes.map((node) => <label key={node.id}><input
              type="checkbox"
              checked={agentSelectedNodeIds.includes(node.id)}
              disabled={loading || Boolean(agentReview)}
              onChange={(event) => toggleAgentNode(node.id, event.target.checked)}
            /><span>{nodeKind(node.kind)}</span><code>{node.id}</code></label>)}</div>
            <div className="agent-egress-settings">
              <label>Provider 标识<input aria-label="Agent provider 标识" value={agentProvider} disabled={loading || Boolean(agentReview)} onChange={(event) => { setAgentProvider(event.target.value); setAgentReview(null); setAgentEgressApproved(false); }} placeholder="精确 provider / deployment ID" /></label>
              <label>保留策略<select aria-label="Agent 保留策略" value={agentRetention} disabled={loading || Boolean(agentReview)} onChange={(event) => { setAgentRetention(event.target.value); setAgentReview(null); setAgentEgressApproved(false); }}><option value="delete-after-call">delete-after-call</option><option value="provider-policy">provider-policy</option></select></label>
              <label>脱敏策略<select aria-label="Agent 脱敏策略" value={agentRedaction} disabled={loading || Boolean(agentReview)} onChange={(event) => { setAgentRedaction(event.target.value); setAgentReview(null); setAgentEgressApproved(false); }}><option value="selected-ir-only">selected-ir-only</option><option value="manual-redaction-reviewed">manual-redaction-reviewed</option></select></label>
            </div>
            {!agentReview && <button type="button" disabled={loading || !agentProvider.trim() || agentSelectedNodeIds.length === 0} onClick={() => void prepareAgentReview()}>冻结 Agent evidence 与 egress review</button>}
            {agentReview && <div className="agent-egress-review">
              <dl><dt>Review digest</dt><dd>{agentReview.previewDigest}</dd><dt>Evidence digest</dt><dd>{agentReview.evidenceDigest}</dd><dt>离机字节</dt><dd>{agentReview.evidenceByteLength.toLocaleString()}</dd><dt>Provider</dt><dd>{agentReview.selection.provider}</dd><dt>Retention</dt><dd>{agentReview.selection.retention}</dd><dt>Redaction</dt><dd>{agentReview.selection.redaction}</dd><dt>网络已执行</dt><dd>否</dd><dt>费用</dt><dd>未提供；请在 provider 侧确认后再批准</dd></dl>
              <label>精确 Agent evidence<textarea aria-label="精确 Agent evidence JSON" readOnly value={JSON.stringify(agentReview.evidence, null, 2)} /></label>
              <label className="intake-checkbox"><input type="checkbox" checked={agentEgressApproved} disabled={loading} onChange={(event) => setAgentEgressApproved(event.target.checked)} />我批准仅将上述 digest 绑定的 {agentReview.evidenceByteLength.toLocaleString()} 字节发送给 {agentReview.selection.provider}，并已核对 retention、redaction 与 provider 费用/政策。</label>
              <label>Agent 返回的 typed patch JSON<textarea aria-label="Agent typed patch JSON" value={agentPatchSource} disabled={loading || !agentEgressApproved} onChange={(event) => setAgentPatchSource(event.target.value)} placeholder="weftext.import-agent-patch.v1；不接受 whole-document/AsciiDoc 字段" /></label>
              <div><button type="button" disabled={loading} onClick={() => clearAgentAuthority()}>取消 Agent 增强，保留本地结果</button><button className="primary" type="button" disabled={loading || !agentEgressApproved || !agentPatchSource.trim()} onClick={() => void applyAgentPatch()}>验证并应用 typed IR patch</button></div>
            </div>}
          </section>}
          {kind === "pdf" && preview.receipt.agentProvenance.length > 0 && <section className="agent-enhancement-attribution" role="status"><strong>Agent typed patch 已应用并重新生成 Core 预览</strong><span>{preview.receipt.agentProvenance.length} 条 agent provenance；仍需下方最终提交确认。</span></section>}
          <section className="intake-proposed-nodes" aria-label="拟议节点树"><h3>拟议节点与精确 AsciiDoc</h3>{preview.proposal.nodes.map((node) => <article key={node.nodeId}>
            <header><div><strong>{node.locator}</strong><code>{node.documentFile} · {node.nodeId}</code></div><small>{shortDigest(node.documentSha256)}</small></header>
            <label>精确拟议源<textarea readOnly value={node.exactAsciidoc} /></label>
            {node.resources.length > 0 && <div className="intake-resources" aria-label={`${node.locator} 的拟议资源`}>{node.resources.map((resource) => <div key={resource.locator}><strong>{resource.locator}</strong><span>{resource.mediaType} · {resource.byteLength.toLocaleString()} 字节</span><code>{shortDigest(resource.sha256)}</code></div>)}</div>}
          </article>)}</section>
          <footer className="intake-actions"><button type="button" disabled={loading} onClick={() => { setPreview(null); clearAgentAuthority(); }}>返回修改</button><button className="primary" type="button" disabled={loading || commitBlocked} onClick={() => void commitPreview()}>{loading ? "正在提交固定计划…" : commitBlocked ? "当前预览不能提交" : "确认并提交固定导入"}</button></footer>
        </>}
      </div>
    </div>
  </section>;
}
